//! The plain point move: its identities, reservations and prior state.

use crate::SlotId;
use crate::orders::apply::{unit_has_pending_move, unit_orders_empty};
#[cfg(test)]
use crate::orders::count_pending_capacity_lookup;
#[cfg(test)]
use crate::orders::count_plain_candidate_index_step;
use crate::orders::pending::{
    MovementDispatchState, PendingMovementOrders, PromotionHeaderIndex, dispatch_start_slot,
    insert_promoted_request, pending_live_member_count, record_dispatch_selection,
};
use crate::orders::{Order, OrderCommand, PendingMovementOrder, UnitOrders};
use crate::world::MoveTarget;
use bevy_ecs::prelude::{Resource, World};
use bevy_math::Vec2;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Authored identity for a legacy-style point move, kept out of movement inputs.
/// Audit replay reconstructs this transient cache from validated command boundaries.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct PlainMoveOrderIdentities(pub BTreeMap<u32, Order>);

#[derive(Resource, Default)]
pub(super) struct PlainMoveReservations {
    pub(super) entries: BTreeMap<u32, PlainMoveReservation>,
    pub(super) slot_counts: BTreeMap<u8, usize>,
    pub(super) winners: BTreeSet<u32>,
    next_ordinal: u64,
    pub(super) budget_tick: Option<u32>,
    budget_start_slot: u8,
    pub(super) budget_used: usize,
}

impl PlainMoveReservations {
    fn insert(&mut self, unit_id: u32, reservation: PlainMoveReservation) {
        if let Some(previous) = self.entries.remove(&unit_id) {
            decrement_slot_count(&mut self.slot_counts, previous.slot.0);
        }
        *self.slot_counts.entry(reservation.slot.0).or_default() += 1;
        self.entries.insert(unit_id, reservation);
        self.winners.remove(&unit_id);
    }

    pub(super) fn remove(&mut self, unit_id: &u32) -> Option<PlainMoveReservation> {
        self.winners.remove(unit_id);
        let previous = self.entries.remove(unit_id)?;
        decrement_slot_count(&mut self.slot_counts, previous.slot.0);
        Some(previous)
    }

    fn remove_dead(&mut self, live_ids: &std::collections::HashSet<u32>) {
        let dead: Vec<_> = self
            .entries
            .keys()
            .filter(|unit_id| !live_ids.contains(unit_id))
            .copied()
            .collect();
        for unit_id in dead {
            self.remove(&unit_id);
        }
        self.winners.retain(|unit_id| live_ids.contains(unit_id));
    }
}

fn decrement_slot_count(counts: &mut BTreeMap<u8, usize>, slot: u8) {
    if let Some(count) = counts.get_mut(&slot) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            counts.remove(&slot);
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct PlainMoveReservation {
    pub(super) slot: SlotId,
    pub(super) accepted_tick: u32,
    seq: u32,
    pub(super) arrival_ordinal: u64,
    preceding_pool_id: u64,
    point: Vec2,
    order: Order,
    pub(super) prior: PlainMovePriorState,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PlainMovePriorState {
    unit_orders: Option<UnitOrders>,
    move_target: Option<MoveTarget>,
    move_progress: Option<crate::movement::MoveProgress>,
    plain_progress: Option<crate::movement::PlainMoveProgress>,
    physical_escape: Option<crate::movement::PhysicalEscape>,
    gather: Option<crate::economy::GatherOrder>,
    identity: Option<Order>,
}

pub(super) fn arbitrate_plain_move_reservations(world: &mut World) {
    const SPREAD_CAP: usize = 2048;
    const PENDING_SLOTS: u8 = 8;
    let tick = world.resource::<crate::world::Tick>().0;
    let live_ids: std::collections::HashSet<_> = world
        .iter_entities()
        .filter_map(|entity| entity.get::<crate::world::UnitId>().map(|id| id.0))
        .collect();
    if let Some(mut reservations) = world.get_resource_mut::<PlainMoveReservations>() {
        reservations.remove_dead(&live_ids);
    }
    let Some(reservations) = world.get_resource::<PlainMoveReservations>() else {
        return;
    };
    if reservations.entries.is_empty() {
        return;
    }
    let pending = world.resource::<PendingMovementOrders>();
    let state = world.resource::<MovementDispatchState>();
    let start_slot = if state.tick == Some(tick) {
        state.start_slot
    } else {
        dispatch_start_slot(pending, state, tick)
    };
    let mut used = if state.tick == Some(tick) {
        state.used
    } else {
        reservations
            .budget_tick
            .filter(|budget_tick| *budget_tick == tick)
            .map_or(0, |_| reservations.budget_used)
    };
    let mut winners = BTreeSet::new();
    let mut reservation_candidates = BTreeMap::<u8, VecDeque<(u32, PlainMoveReservation)>>::new();
    for (&unit_id, reservation) in &reservations.entries {
        #[cfg(test)]
        count_plain_candidate_index_step();
        reservation_candidates
            .entry(reservation.slot.0)
            .or_default()
            .push_back((unit_id, reservation.clone()));
    }
    for candidates in reservation_candidates.values_mut() {
        let mut ordered: Vec<_> = candidates.drain(..).collect();
        ordered
            .sort_by_key(|(unit_id, entry)| (entry.accepted_tick, entry.arrival_ordinal, *unit_id));
        candidates.extend(ordered);
    }
    for offset in 0..PENDING_SLOTS {
        let slot = (start_slot + offset) % PENDING_SLOTS;
        let mut queue_index = 0;
        let candidates = reservation_candidates.entry(slot).or_default();
        loop {
            let queued = pending
                .queues
                .get(&slot)
                .and_then(|queue| queue.get(queue_index));
            let candidate = candidates.front();
            let choose_candidate = match (candidate, queued) {
                (Some((unit_id, entry)), Some(request)) => {
                    reservation_precedes_request(*unit_id, entry, request)
                }
                (Some(_), None) => true,
                _ => false,
            };
            let count = if choose_candidate {
                1
            } else if let Some(request) = queued {
                pending_live_member_count(request, &live_ids)
            } else {
                break;
            };
            if count == 0 {
                queue_index += 1;
                continue;
            }
            if used > 0 && used.saturating_add(count) > SPREAD_CAP {
                break;
            }
            if choose_candidate {
                let unit_id = candidates.front().unwrap().0;
                winners.insert(unit_id);
                candidates.pop_front();
            } else {
                queue_index += 1;
            }
            used = used.saturating_add(count);
        }
    }
    let mut losers: Vec<_> = reservations
        .entries
        .iter()
        .filter(|(unit_id, _)| !winners.contains(unit_id))
        .map(|(&unit_id, reservation)| (unit_id, reservation.clone()))
        .collect();
    losers.sort_by_key(|(unit_id, entry)| {
        std::cmp::Reverse((entry.accepted_tick, entry.arrival_ordinal, *unit_id))
    });
    world.resource_mut::<PlainMoveReservations>().winners = winners;

    let entity_by_unit: BTreeMap<_, _> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<crate::world::UnitId>()
                .map(|id| (id.0, entity.id()))
        })
        .collect();
    let promotion_headers = {
        let pending = world.resource::<PendingMovementOrders>();
        (0..PENDING_SLOTS)
            .map(|slot| {
                let header = pending
                    .queues
                    .get(&slot)
                    .map(PromotionHeaderIndex::new)
                    .unwrap_or_default();
                (slot, header)
            })
            .collect::<BTreeMap<_, _>>()
    };
    for (unit_id, reservation) in losers {
        let Some(entity) = entity_by_unit.get(&unit_id).copied() else {
            world
                .resource_mut::<PlainMoveReservations>()
                .remove(&unit_id);
            continue;
        };
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            let request = PendingMovementOrder {
                slot: reservation.slot,
                accepted_tick: reservation.accepted_tick,
                seq: reservation.seq,
                group: 0,
                pool_id: 0,
                point: reservation.point,
                unit_ids: vec![unit_id],
                order: Some(reservation.order.clone()),
                member_orders: BTreeMap::from([(unit_id, reservation.order.clone())]),
                plain: false,
            };
            let queued = world.resource_mut::<PendingMovementOrders>().queue(request);
            debug_assert!(queued, "reserved plain move capacity must remain available");
            if let Some(queue) = world
                .resource_mut::<PendingMovementOrders>()
                .queues
                .get_mut(&reservation.slot.0)
            {
                let promoted = queue
                    .pop_back()
                    .expect("the newly queued singleton remains at the queue tail");
                insert_promoted_request(
                    queue,
                    promoted,
                    reservation.accepted_tick,
                    reservation.preceding_pool_id,
                    promotion_headers
                        .get(&reservation.slot.0)
                        .expect("every scheduler slot has an immutable header index"),
                );
            }
            restore_plain_move_prior(world, entity, unit_id, &reservation.prior, true);
            world
                .resource_mut::<PlainMoveReservations>()
                .remove(&unit_id);
        });
    }
}

pub(super) fn record_plain_dispatch_selection(
    world: &mut World,
    tick: u32,
    start_slot: u8,
    count: usize,
) {
    if world.resource::<MovementDispatchState>().tick == Some(tick) {
        let start_slot = world.resource::<MovementDispatchState>().start_slot;
        record_dispatch_selection(world, tick, start_slot, count);
        return;
    }
    world.init_resource::<PlainMoveReservations>();
    let mut reservations = world.resource_mut::<PlainMoveReservations>();
    if reservations.budget_tick != Some(tick) {
        reservations.budget_tick = Some(tick);
        reservations.budget_start_slot = start_slot;
        reservations.budget_used = 0;
    }
    reservations.budget_used = reservations.budget_used.saturating_add(count);
}

pub(super) fn reservation_precedes_request(
    _unit_id: u32,
    reservation: &PlainMoveReservation,
    request: &PendingMovementOrder,
) -> bool {
    reservation.accepted_tick < request.accepted_tick
        || (reservation.accepted_tick == request.accepted_tick
            && request.pool_id > reservation.preceding_pool_id)
}

pub(crate) fn plain_move_has_no_prior_order_state(world: &World, command: &OrderCommand) -> bool {
    command.units.len() == 1
        && unit_orders_empty(world, command.units[0])
        && !unit_has_pending_move(world, command.units[0])
        && !world.iter_entities().any(|entity| {
            entity
                .get::<crate::world::UnitId>()
                .is_some_and(|id| id.0 == command.units[0])
                && (entity.get::<MoveTarget>().is_some()
                    || entity.get::<crate::movement::MoveProgress>().is_some()
                    || entity.get::<crate::movement::PlainMoveProgress>().is_some())
        })
}

pub(crate) fn plain_move_reservation_exists(world: &World, unit_id: u32) -> bool {
    world
        .get_resource::<PlainMoveReservations>()
        .is_some_and(|reservations| reservations.entries.contains_key(&unit_id))
}

pub(super) fn plain_move_reservation_capacity_available(
    world: &World,
    slot: SlotId,
    unit_id: u32,
) -> bool {
    #[cfg(test)]
    count_pending_capacity_lookup();
    let pending = world.resource::<PendingMovementOrders>();
    let pending_count = pending
        .pending_units
        .get(&slot.0)
        .copied()
        .unwrap_or_default();
    let pending_replaced = pending
        .pending_member_counts
        .get(&unit_id)
        .copied()
        .unwrap_or_default();
    let reserved_count = world
        .get_resource::<PlainMoveReservations>()
        .and_then(|reservations| reservations.slot_counts.get(&slot.0).copied())
        .unwrap_or_default();
    let replaces_reservation = world
        .get_resource::<PlainMoveReservations>()
        .and_then(|reservations| reservations.entries.get(&unit_id))
        .is_some_and(|reservation| reservation.slot == slot);
    pending_count
        .saturating_sub(pending_replaced)
        .saturating_add(reserved_count)
        .saturating_sub(usize::from(replaces_reservation))
        .saturating_add(1)
        <= 16_384
}

pub(super) fn capture_plain_move_prior(
    world: &World,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
) -> PlainMovePriorState {
    let entity = world.entity(entity);
    PlainMovePriorState {
        unit_orders: entity.get::<UnitOrders>().cloned(),
        move_target: entity.get::<MoveTarget>().copied(),
        move_progress: entity.get::<crate::movement::MoveProgress>().cloned(),
        plain_progress: entity.get::<crate::movement::PlainMoveProgress>().cloned(),
        physical_escape: entity.get::<crate::movement::PhysicalEscape>().cloned(),
        gather: world
            .get_resource::<crate::economy::EconomyOrders>()
            .and_then(|orders| orders.0.get(&unit_id))
            .cloned(),
        identity: world
            .get_resource::<PlainMoveOrderIdentities>()
            .and_then(|identities| identities.0.get(&unit_id))
            .cloned(),
    }
}

pub(super) fn restore_plain_move_prior(
    world: &mut World,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    prior: &PlainMovePriorState,
    preserve_append_suffix: bool,
) {
    let suffix = if preserve_append_suffix {
        world
            .entity(entity)
            .get::<UnitOrders>()
            .map(|orders| orders.queue.clone())
            .unwrap_or_default()
    } else {
        VecDeque::new()
    };
    {
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.remove::<MoveTarget>();
        entity_mut.remove::<crate::movement::MoveProgress>();
        entity_mut.remove::<crate::movement::PlainMoveProgress>();
        entity_mut.remove::<crate::movement::PhysicalEscape>();
        if let Some(target) = prior.move_target {
            entity_mut.insert(target);
        }
        if let Some(progress) = &prior.move_progress {
            entity_mut.insert(progress.clone());
        }
        if let Some(progress) = &prior.plain_progress {
            entity_mut.insert(progress.clone());
        }
        if let Some(escape) = &prior.physical_escape {
            entity_mut.insert(escape.clone());
        }
        let mut orders = prior.unit_orders.clone().unwrap_or_default();
        // Replace cleared the old queue at acceptance; only later Appends survive.
        orders.queue = suffix;
        if prior.unit_orders.is_some() || !orders.queue.is_empty() {
            entity_mut.insert(orders);
        } else {
            entity_mut.remove::<UnitOrders>();
        }
    }
    if let Some(mut economy_orders) = world.get_resource_mut::<crate::economy::EconomyOrders>() {
        economy_orders.0.remove(&unit_id);
        if let Some(gather) = &prior.gather {
            economy_orders.0.insert(unit_id, gather.clone());
        }
    } else if let Some(gather) = &prior.gather {
        let mut economy_orders = crate::economy::EconomyOrders::default();
        economy_orders.0.insert(unit_id, gather.clone());
        world.insert_resource(economy_orders);
    }
    if let Some(mut identities) = world.get_resource_mut::<PlainMoveOrderIdentities>() {
        identities.0.remove(&unit_id);
    }
    if let Some(identity) = &prior.identity {
        world.init_resource::<PlainMoveOrderIdentities>();
        world
            .resource_mut::<PlainMoveOrderIdentities>()
            .0
            .insert(unit_id, identity.clone());
    }
}

pub(super) fn remember_plain_move_identity(world: &mut World, unit_id: u32, order: Order) {
    world.init_resource::<PlainMoveOrderIdentities>();
    world
        .resource_mut::<PlainMoveOrderIdentities>()
        .0
        .insert(unit_id, order);
}

pub(super) fn record_plain_move_reservation(
    world: &mut World,
    unit_id: u32,
    slot: SlotId,
    seq: u32,
    point: Vec2,
    order: Order,
    prior: PlainMovePriorState,
) {
    let accepted_tick = world.resource::<crate::world::Tick>().0;
    let preceding_pool_id = world.resource::<PendingMovementOrders>().next_pool_id;
    world.init_resource::<PlainMoveReservations>();
    let mut reservations = world.resource_mut::<PlainMoveReservations>();
    reservations.next_ordinal = reservations.next_ordinal.saturating_add(1);
    let arrival_ordinal = reservations.next_ordinal;
    reservations.insert(
        unit_id,
        PlainMoveReservation {
            slot,
            accepted_tick,
            seq,
            arrival_ordinal,
            preceding_pool_id,
            point,
            order,
            prior,
        },
    );
}

pub(crate) fn forget_plain_move_identity(world: &mut World, unit_id: u32) {
    if let Some(mut identities) = world.get_resource_mut::<PlainMoveOrderIdentities>() {
        identities.0.remove(&unit_id);
    }
    if let Some(mut reservations) = world.get_resource_mut::<PlainMoveReservations>() {
        reservations.remove(&unit_id);
    }
}
