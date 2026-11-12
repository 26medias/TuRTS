//! The pending move queues and the dispatch of a move envelope.

use crate::SlotId;
use crate::orders::apply::{
    clamp_target, clear_gather, movement_envelope_capacity_available_after_replacing,
    unit_has_pending_move, unit_radius,
};
use crate::orders::behaviours::queue::start_next_queued_order;
#[cfg(test)]
use crate::orders::count_legacy_waiter_entity_lookup;
#[cfg(test)]
use crate::orders::count_pending_queue_member_scan_step;
#[cfg(test)]
use crate::orders::count_plain_candidate_index_step;
#[cfg(test)]
use crate::orders::count_plain_promotion_header_step;
use crate::orders::plain::{
    PlainMoveReservation, PlainMoveReservations, arbitrate_plain_move_reservations,
    forget_plain_move_identity, record_plain_dispatch_selection, reservation_precedes_request,
    restore_plain_move_prior,
};
use crate::orders::{
    MovementEnvelopeRequest, Order, OrderFailure, OrderTargetSim, PendingMovementOrder, UnitOrders,
};
use crate::world::MoveTarget;
use bevy_ecs::prelude::{Resource, World};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use turts_mod_api::Behaviour;

/// Per-slot atomic spread queues. This is audited command state, not transient scheduler data.
#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PendingMovementOrders {
    pub queues: BTreeMap<u8, VecDeque<PendingMovementOrder>>,
    pub pending_units: BTreeMap<u8, usize>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pending_member_counts: BTreeMap<u32, usize>,
    pub next_slot: u8,
    pub next_pool_id: u64,
}

/// Transient fixed-tick accounting shared by both order dispatch passes.
#[derive(Resource, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovementDispatchState {
    pub tick: Option<u32>,
    pub start_slot: u8,
    pub used: usize,
    pub cursor_advanced: bool,
}

/// One-shot marker for queued Unit-target activation. Active follows refresh
/// directly and do not become new capped envelopes on later ticks.
#[derive(Resource, Default)]
pub(super) struct UnitMoveNeedsAllocation(pub(super) BTreeSet<u32>);

#[derive(Resource, Default)]
pub(super) struct SpreadIndexCache {
    tick: Option<u32>,
    context: Option<crate::movement::SpreadContext>,
    #[cfg(test)]
    pub(super) context_builds: u32,
}

impl MovementDispatchState {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl PendingMovementOrders {
    pub fn is_empty(&self) -> bool {
        self.queues.is_empty()
            && self.pending_units.values().all(|count| *count == 0)
            && self.pending_member_counts.is_empty()
            && self.next_slot == 0
            && self.next_pool_id == 0
    }

    pub fn queue(&mut self, mut request: PendingMovementOrder) -> bool {
        request.unit_ids.sort_unstable();
        if request.member_orders.is_empty()
            && let Some(order) = &request.order
        {
            request.member_orders = request
                .unit_ids
                .iter()
                .map(|unit_id| (*unit_id, order.clone()))
                .collect();
        }
        let slot = request.slot.0;
        let queued_units = self.pending_units.get(&slot).copied().unwrap_or_default();
        if request.group != 0
            && let Some(index) = self.queues.get(&slot).and_then(|queue| {
                queue.iter().position(|existing| {
                    existing.group == request.group
                        && existing.accepted_tick == request.accepted_tick
                })
            })
        {
            let existing_ids: std::collections::HashSet<_> =
                self.queues[&slot][index].unit_ids.iter().copied().collect();
            let added_count = request
                .unit_ids
                .iter()
                .filter(|id| !existing_ids.contains(id))
                .count();
            if queued_units.saturating_add(added_count) > 16_384 {
                return false;
            }
            let incoming_ids = request.unit_ids;
            let incoming_orders = request.member_orders;
            let added: Vec<_> = {
                let queue = self.queues.get_mut(&slot).unwrap();
                let existing = &mut queue[index];
                let mut existing_ids = existing_ids;
                let added: Vec<_> = incoming_ids
                    .into_iter()
                    .filter(|id| existing_ids.insert(*id))
                    .collect();
                existing.unit_ids.extend(added.iter().copied());
                existing.unit_ids.sort_unstable();
                for (unit_id, order) in incoming_orders {
                    existing.member_orders.insert(unit_id, order);
                }
                added
            };
            self.pending_units.insert(slot, queued_units + added.len());
            for id in added {
                *self.pending_member_counts.entry(id).or_default() += 1;
            }
            return true;
        }
        if queued_units.saturating_add(request.unit_ids.len()) > 16_384 {
            return false;
        }
        let queue = self.queues.entry(slot).or_default();
        self.next_pool_id = self.next_pool_id.saturating_add(1);
        request.pool_id = self.next_pool_id;
        self.pending_units
            .insert(slot, queued_units + request.unit_ids.len());
        for id in &request.unit_ids {
            *self.pending_member_counts.entry(*id).or_default() += 1;
        }
        queue.push_back(request);
        true
    }

    fn pop_front(&mut self, slot: u8) -> Option<PendingMovementOrder> {
        let request = self.queues.get_mut(&slot)?.pop_front()?;
        for id in &request.unit_ids {
            decrement_member_count(&mut self.pending_member_counts, *id);
        }
        let remaining = self
            .pending_units
            .get(&slot)
            .copied()
            .unwrap_or_default()
            .saturating_sub(request.unit_ids.len());
        self.pending_units.insert(slot, remaining);
        if self.queues.get(&slot).is_some_and(VecDeque::is_empty) {
            self.queues.remove(&slot);
        }
        Some(request)
    }

    pub fn cancel_units(&mut self, slot: u8, unit_ids: &[u32]) {
        let removed: std::collections::HashSet<_> = unit_ids.iter().copied().collect();
        let Some(queue) = self.queues.get_mut(&slot) else {
            return;
        };
        if unit_ids
            .iter()
            .all(|unit_id| !self.pending_member_counts.contains_key(unit_id))
        {
            return;
        }
        let mut canceled = 0;
        let mut canceled_members = BTreeMap::<u32, usize>::new();
        for request in queue.iter_mut() {
            let removed_from_request: Vec<_> = request
                .unit_ids
                .iter()
                .copied()
                .filter(|id| {
                    #[cfg(test)]
                    count_pending_queue_member_scan_step();
                    removed.contains(id)
                })
                .collect();
            canceled += removed_from_request.len();
            for id in removed_from_request {
                *canceled_members.entry(id).or_default() += 1;
            }
            request.unit_ids.retain(|id| !removed.contains(id));
            request
                .member_orders
                .retain(|unit_id, _| !removed.contains(unit_id));
        }
        queue.retain(|request| !request.unit_ids.is_empty());
        let remaining = self
            .pending_units
            .get(&slot)
            .copied()
            .unwrap_or_default()
            .saturating_sub(canceled);
        self.pending_units.insert(slot, remaining);
        if queue.is_empty() {
            self.queues.remove(&slot);
        }
        for (id, count) in canceled_members {
            for _ in 0..count {
                decrement_member_count(&mut self.pending_member_counts, id);
            }
        }
    }

    pub(super) fn can_queue_after_replacing(
        &self,
        slot: u8,
        _group: u32,
        _tick: u32,
        unit_ids: &[u32],
    ) -> bool {
        if unit_ids
            .iter()
            .all(|unit_id| !self.pending_member_counts.contains_key(unit_id))
        {
            return self
                .pending_units
                .get(&slot)
                .copied()
                .unwrap_or_default()
                .saturating_add(unit_ids.len())
                <= 16_384;
        }
        let replacement: std::collections::HashSet<_> = unit_ids.iter().copied().collect();
        let mut remaining = self.pending_units.get(&slot).copied().unwrap_or_default();
        if let Some(queue) = self.queues.get(&slot) {
            let removed = queue
                .iter()
                .map(|request| {
                    request
                        .unit_ids
                        .iter()
                        .filter(|id| {
                            #[cfg(test)]
                            count_pending_queue_member_scan_step();
                            replacement.contains(id)
                        })
                        .count()
                })
                .sum::<usize>();
            remaining = remaining.saturating_sub(removed);
        }
        remaining.saturating_add(unit_ids.len()) <= 16_384
    }
}

fn decrement_member_count(counts: &mut BTreeMap<u32, usize>, unit_id: u32) {
    if let Some(count) = counts.get_mut(&unit_id) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            counts.remove(&unit_id);
        }
    }
}

fn pending_member_target_is_live(
    request: &PendingMovementOrder,
    unit_id: u32,
    live_ids: &std::collections::HashSet<u32>,
) -> bool {
    match request.order_for_unit(unit_id).map(|order| &order.target) {
        Some(OrderTargetSim::Unit(target_id)) => live_ids.contains(target_id),
        Some(OrderTargetSim::Point(_)) | None => true,
        _ => false,
    }
}

pub(super) fn pending_live_member_count(
    request: &PendingMovementOrder,
    live_ids: &std::collections::HashSet<u32>,
) -> usize {
    request
        .unit_ids
        .iter()
        .filter(|unit_id| {
            live_ids.contains(unit_id)
                && pending_member_target_is_live(request, **unit_id, live_ids)
        })
        .count()
}

pub(crate) fn queue_movement_envelope(world: &mut World, request: MovementEnvelopeRequest) -> bool {
    let tick = world.resource::<crate::world::Tick>().0;
    world
        .resource_mut::<PendingMovementOrders>()
        .queue(PendingMovementOrder {
            slot: request.slot,
            accepted_tick: tick,
            seq: request.seq,
            group: request.group,
            pool_id: 0,
            point: request.point,
            unit_ids: request.unit_ids,
            order: request.order,
            member_orders: request.member_orders,
            plain: request.plain,
        })
}

pub(crate) fn queue_legacy_move_to_envelope(
    world: &mut World,
    request: MovementEnvelopeRequest,
) -> bool {
    let tick = world.resource::<crate::world::Tick>().0;
    if !world
        .resource::<PendingMovementOrders>()
        .can_queue_after_replacing(request.slot.0, 0, tick, &request.unit_ids)
        || !movement_envelope_capacity_available_after_replacing(
            world,
            request.slot,
            &request.unit_ids,
        )
    {
        return false;
    }
    let pending_request = PendingMovementOrder {
        slot: request.slot,
        accepted_tick: tick,
        seq: request.seq,
        group: 0,
        pool_id: 0,
        point: request.point,
        unit_ids: request.unit_ids.clone(),
        order: None,
        member_orders: BTreeMap::new(),
        plain: true,
    };
    let mut pending = world.resource_mut::<PendingMovementOrders>();
    pending.cancel_units(request.slot.0, &request.unit_ids);
    if !pending.queue(pending_request) {
        return false;
    }
    true
}

/// On successful deferred legacy Replace admission, cancel only future work.
/// The current execution (including a provisional plain Move) remains active
/// until the legacy envelope actually gets its spread turn.
pub(crate) fn prepare_legacy_move_to_waiters(
    world: &mut World,
    unit_ids: &[u32],
    entity_by_unit: &std::collections::HashMap<u32, bevy_ecs::entity::Entity>,
) {
    for unit_id in unit_ids {
        #[cfg(test)]
        count_legacy_waiter_entity_lookup();
        if let Some(entity) = entity_by_unit.get(unit_id).copied() {
            if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                orders.queue.clear();
            }
            let candidate = world
                .get_resource_mut::<PlainMoveReservations>()
                .and_then(|mut reservations| reservations.remove(unit_id));
            if let Some(candidate) = candidate {
                restore_plain_move_prior(world, entity, *unit_id, &candidate.prior, false);
            }
            if let Some(mut needs) = world.get_resource_mut::<UnitMoveNeedsAllocation>() {
                needs.0.remove(unit_id);
            }
        }
    }
}

/// Reset an actor for an immediately accepted idless legacy MoveTo Replace.
/// PhysicalEscape is an independent collision obligation and deliberately survives.
pub(crate) fn reset_legacy_move_to_unit(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
) {
    world
        .resource_mut::<PendingMovementOrders>()
        .cancel_units(slot.0, &[unit_id]);
    if let Some(mut needs) = world.get_resource_mut::<UnitMoveNeedsAllocation>() {
        needs.0.remove(&unit_id);
    }
    forget_plain_move_identity(world, unit_id);
    let mut actor = world.entity_mut(entity);
    actor.remove::<crate::movement::MoveProgress>();
    actor.remove::<crate::movement::PlainMoveProgress>();
    if actor
        .get::<UnitOrders>()
        .is_some_and(|orders| *orders != UnitOrders::default())
    {
        actor.insert(UnitOrders::default());
    }
    clear_gather(world, unit_id);
}

#[derive(Default)]
pub(super) struct PromotionHeaderIndex {
    prefix_max: Vec<(u32, u64)>,
}

impl PromotionHeaderIndex {
    pub(super) fn new(queue: &VecDeque<PendingMovementOrder>) -> Self {
        let mut prefix_max = Vec::with_capacity(queue.len());
        let mut maximum = None::<(u32, u64)>;
        for request in queue {
            #[cfg(test)]
            count_plain_promotion_header_step();
            let key = (request.accepted_tick, request.pool_id);
            maximum = Some(maximum.map_or(key, |previous| previous.max(key)));
            prefix_max.push(maximum.unwrap());
        }
        Self { prefix_max }
    }

    fn first_qualifying(&self, accepted_tick: u32, preceding_pool_id: u64) -> Option<usize> {
        let key = (accepted_tick, preceding_pool_id);
        let index = self.prefix_max.partition_point(|maximum| *maximum <= key);
        (index < self.prefix_max.len()).then_some(index)
    }
}

pub(super) fn insert_promoted_request(
    queue: &mut VecDeque<PendingMovementOrder>,
    promoted: PendingMovementOrder,
    accepted_tick: u32,
    preceding_pool_id: u64,
    header: &PromotionHeaderIndex,
) {
    let insert_at = header
        .first_qualifying(accepted_tick, preceding_pool_id)
        .unwrap_or(header.prefix_max.len());
    // Losers are visited in reverse accepted-arrival order. Their original
    // insertion boundaries are therefore nonincreasing within a slot: an
    // earlier boundary stays fixed when later promotions are already present,
    // and an equal boundary prepends to restore FIFO.
    queue.insert(insert_at, promoted);
}

pub(super) fn finish_movement_dispatch_tick(world: &mut World) {
    const PENDING_SLOTS: u8 = 8;
    let tick = world.resource::<crate::world::Tick>().0;
    let (start_slot, advance_cursor, initialized, plain_budget) = {
        let state = world.resource::<MovementDispatchState>();
        let pending = world.resource::<PendingMovementOrders>();
        let plain_budget = world
            .get_resource::<PlainMoveReservations>()
            .filter(|reservations| reservations.budget_tick == Some(tick))
            .map_or(0, |reservations| reservations.budget_used);
        if state.tick == Some(tick) {
            (state.start_slot, !state.cursor_advanced, true, plain_budget)
        } else {
            (
                dispatch_start_slot(pending, state, tick),
                true,
                state.tick.is_some()
                    || pending.next_pool_id != 0
                    || pending.next_slot != 0
                    || !pending.queues.is_empty(),
                plain_budget,
            )
        }
    };
    if !initialized {
        return;
    }
    if !advance_cursor {
        return;
    }
    {
        let mut state = world.resource_mut::<MovementDispatchState>();
        state.tick = Some(tick);
        state.start_slot = start_slot;
        state.used = plain_budget;
        state.cursor_advanced = true;
    }
    world.resource_mut::<PendingMovementOrders>().next_slot = (start_slot + 1) % PENDING_SLOTS;
}

pub(super) fn dispatch_start_slot(
    pending: &PendingMovementOrders,
    state: &MovementDispatchState,
    tick: u32,
) -> u8 {
    const PENDING_SLOTS: u32 = 8;
    match state.tick {
        Some(previous_tick) => {
            let idle_ticks = tick.saturating_sub(previous_tick).saturating_sub(1);
            ((u32::from(pending.next_slot) + idle_ticks) % PENDING_SLOTS) as u8
        }
        None => tick.saturating_sub(1).rem_euclid(PENDING_SLOTS) as u8,
    }
}

pub(super) fn record_dispatch_selection(
    world: &mut World,
    tick: u32,
    start_slot: u8,
    count: usize,
) {
    let prior_plain_budget = world
        .get_resource::<PlainMoveReservations>()
        .filter(|reservations| reservations.budget_tick == Some(tick))
        .map_or(0, |reservations| reservations.budget_used);
    let advance_cursor = {
        let mut state = world.resource_mut::<MovementDispatchState>();
        if state.tick != Some(tick) {
            state.tick = Some(tick);
            state.start_slot = start_slot;
            state.used = prior_plain_budget;
            state.cursor_advanced = prior_plain_budget != 0;
        }
        state.used = state.used.saturating_add(count);
        if !state.cursor_advanced {
            state.cursor_advanced = true;
            true
        } else {
            false
        }
    };
    if advance_cursor {
        world.resource_mut::<PendingMovementOrders>().next_slot = (start_slot + 1) % 8;
    }
    if prior_plain_budget > 0
        && let Some(mut reservations) = world.get_resource_mut::<PlainMoveReservations>()
    {
        reservations.budget_used = 0;
        reservations.budget_tick = None;
    }
}

pub(super) fn queue_unallocated_move_orders(world: &mut World) {
    type GroupKey = (u8, u32, u32);
    type GroupMembers = Vec<(u32, bevy_ecs::entity::Entity, Order)>;
    let entity_by_unit: BTreeMap<_, _> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<crate::world::UnitId>()
                .map(|unit_id| (unit_id.0, entity.id()))
        })
        .collect();
    let mut candidates: std::collections::BTreeSet<_> = entity_by_unit.keys().copied().collect();
    loop {
        let mut grouped: BTreeMap<GroupKey, GroupMembers> = BTreeMap::new();
        for unit_id in std::mem::take(&mut candidates) {
            let Some(entity_id) = entity_by_unit.get(&unit_id).copied() else {
                continue;
            };
            let Ok(entity) = world.get_entity(entity_id) else {
                continue;
            };
            let (Some(owner), Some(orders), Some(target)) = (
                entity.get::<crate::world::Owner>(),
                entity.get::<UnitOrders>(),
                entity.get::<MoveTarget>(),
            ) else {
                continue;
            };
            if unit_has_pending_move(world, unit_id) {
                continue;
            }
            if entity.get::<crate::movement::MoveProgress>().is_some() {
                continue;
            }
            let Some(order) = orders.current.as_ref().filter(|order| {
                order.behaviour == Behaviour::Move
                    || (order.behaviour == Behaviour::Attack
                        && matches!(order.target, OrderTargetSim::Point(_)))
            }) else {
                continue;
            };
            if matches!(order.target, OrderTargetSim::Unit(_))
                && !world
                    .get_resource::<UnitMoveNeedsAllocation>()
                    .is_some_and(|pending| pending.0.contains(&unit_id))
            {
                continue;
            }
            let Some(raw_point) = resolved_move_point(world, &entity_by_unit, order) else {
                continue;
            };
            if clamp_target(world, raw_point, unit_radius(world, entity.id())) != target.0 {
                continue;
            }
            let unique = if order.group == 0 { unit_id } else { 0 };
            grouped
                .entry((owner.0.0, order.group, unique))
                .or_default()
                .push((unit_id, entity_id, order.clone()));
        }
        if grouped.is_empty() {
            break;
        }

        let mut next_candidates = std::collections::BTreeSet::new();
        for ((slot, group, _), mut members) in grouped {
            members.sort_unstable_by_key(|(unit_id, ..)| *unit_id);
            let unit_ids: Vec<_> = members.iter().map(|(unit_id, ..)| *unit_id).collect();
            let Some(raw_point) = resolved_move_point(world, &entity_by_unit, &members[0].2) else {
                continue;
            };
            let max_radius = members
                .iter()
                .map(|(_, entity, _)| unit_radius(world, *entity))
                .fold(0.0_f32, f32::max);
            let point = clamp_target(world, raw_point, max_radius);
            let order = members[0].2.clone();
            let member_orders = members
                .iter()
                .map(|(unit_id, _, order)| (*unit_id, order.clone()))
                .collect::<BTreeMap<_, _>>();
            let tick = world.resource::<crate::world::Tick>().0;
            let can_queue = world
                .resource::<PendingMovementOrders>()
                .can_queue_after_replacing(slot, group, tick, &unit_ids);
            let mut queued = false;
            if can_queue {
                let first_entity = members[0].1;
                let first_unit_id = members[0].0;
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    first_unit_id,
                    |world| {
                        queued = queue_movement_envelope(
                            world,
                            MovementEnvelopeRequest {
                                slot: SlotId(slot),
                                seq: unit_ids[0],
                                group,
                                point,
                                unit_ids: unit_ids.clone(),
                                order: Some(order.clone()),
                                member_orders: member_orders.clone(),
                                plain: false,
                            },
                        );
                        if queued {
                            world.entity_mut(first_entity).remove::<MoveTarget>();
                        }
                    },
                );
                refresh_spread_reservation(world, first_unit_id, first_entity);
            }
            if queued {
                if let Some(mut needs_allocation) =
                    world.get_resource_mut::<UnitMoveNeedsAllocation>()
                {
                    for unit_id in &unit_ids {
                        needs_allocation.0.remove(unit_id);
                    }
                }
                for (unit_id, entity, _) in members.into_iter().skip(1) {
                    crate::economy::transaction::capture_unit_order_operation(
                        world,
                        unit_id,
                        |world| {
                            world.entity_mut(entity).remove::<MoveTarget>();
                        },
                    );
                    refresh_spread_reservation(world, unit_id, entity);
                }
                continue;
            }

            // A queued envelope is admitted or refused as one group. Refused
            // members cannot fall through to the legacy unallocated move path.
            for (unit_id, entity, _) in members {
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    unit_id,
                    |world| {
                        let failure_order = world
                            .entity(entity)
                            .get::<UnitOrders>()
                            .and_then(|orders| orders.current.clone());
                        world.entity_mut(entity).remove::<MoveTarget>();
                        world
                            .entity_mut(entity)
                            .remove::<crate::movement::MoveProgress>();
                        world
                            .entity_mut(entity)
                            .remove::<crate::movement::PlainMoveProgress>();
                        forget_plain_move_identity(world, unit_id);
                        let mut entity_mut = world.entity_mut(entity);
                        if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                            if let Some(order) = failure_order {
                                orders.last_failure = Some(OrderFailure {
                                    order: order.order,
                                    reason: crate::command::RejectReason::RateLimited,
                                    tick,
                                });
                            }
                            orders.current = None;
                        }
                        clear_gather(world, unit_id);
                        start_next_queued_order(world, SlotId(slot), entity, unit_id);
                    },
                );
                refresh_spread_reservation(world, unit_id, entity);
                next_candidates.insert(unit_id);
            }
        }
        // Revisit only actors whose captured refusal advanced their queue. Other
        // ready groups were already handled in this round and must not trigger
        // repeated whole-world rediscovery under cap pressure.
        candidates = next_candidates;
    }
}

fn resolved_move_point(
    world: &World,
    entity_by_unit: &BTreeMap<u32, bevy_ecs::entity::Entity>,
    order: &Order,
) -> Option<Vec2> {
    match &order.target {
        OrderTargetSim::Point(point) => Some(*point),
        OrderTargetSim::Unit(target_id) => entity_by_unit.get(target_id).and_then(|entity| {
            world
                .entity(*entity)
                .get::<crate::world::Position>()
                .map(|p| p.0)
        }),
        _ => None,
    }
}

/// Keep the phase-local reservation index in sync with order transitions between
/// the two dispatch passes. In particular, a refused Move can immediately start
/// a queued Hold, changing its effective reservation from the old target to its
/// current body position.
fn refresh_spread_reservation(world: &mut World, unit_id: u32, entity: bevy_ecs::entity::Entity) {
    let Some((tick, position, radius, target)) = (|| {
        let tick = world.resource::<crate::world::Tick>().0;
        let entity_ref = world.get_entity(entity).ok()?;
        let id = entity_ref.get::<crate::world::UnitId>()?;
        if id.0 != unit_id {
            return None;
        }
        let position = entity_ref.get::<crate::world::Position>()?.0;
        let unit_type = entity_ref.get::<crate::world::UnitType>()?.0;
        let entry = world
            .resource::<crate::registry::UnitTypes>()
            .get(unit_type)?;
        entry.movement.as_ref()?;
        let target = entity_ref.get::<MoveTarget>().map(|target| target.0);
        Some((tick, position, entry.radius, target))
    })() else {
        return;
    };
    let Some(mut cache) = world.get_resource_mut::<SpreadIndexCache>() else {
        return;
    };
    if cache.tick != Some(tick) {
        return;
    }
    if let Some(context) = &mut cache.context {
        context.reserve_unit(
            unit_id,
            crate::movement::ReservedFootprint {
                position: target.unwrap_or(position),
                radius,
            },
        );
    }
}

pub(super) fn dispatch_pending_movement(world: &mut World) {
    use crate::movement::{
        MoveProgress, MovementActivity, MovementPools, ReservedFootprint, SpreadContext,
        SpreadMember, StaticFootprint, spread_destinations_with_context,
    };
    const SPREAD_CAP: usize = 2048;
    const PENDING_SLOTS: u8 = 8;

    arbitrate_plain_move_reservations(world);
    let tick = world.resource::<crate::world::Tick>().0;
    let pending = world.resource::<PendingMovementOrders>();
    let (start_slot, mut used) = {
        let state = world.resource::<MovementDispatchState>();
        if state.tick == Some(tick) {
            (state.start_slot, state.used)
        } else {
            let budget = world
                .get_resource::<PlainMoveReservations>()
                .filter(|reservations| reservations.budget_tick == Some(tick))
                .map_or(0, |reservations| reservations.budget_used);
            (dispatch_start_slot(pending, state, tick), budget)
        }
    };
    let entity_by_unit: BTreeMap<_, _> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<crate::world::UnitId>()
                .map(|id| (id.0, entity.id()))
        })
        .collect();
    let mut index_cache = world
        .remove_resource::<SpreadIndexCache>()
        .unwrap_or_default();
    if index_cache.tick != Some(tick) || index_cache.context.is_none() {
        let types = world.resource::<crate::registry::UnitTypes>();
        let mut reserved = Vec::new();
        let mut static_footprints = Vec::new();
        let mut max_unit_radius = 0.0_f32;
        for entity in world.iter_entities() {
            let (Some(id), Some(position), Some(unit_type)) = (
                entity.get::<crate::world::UnitId>(),
                entity.get::<crate::world::Position>(),
                entity.get::<crate::world::UnitType>(),
            ) else {
                continue;
            };
            let Some(unit) = types.get(unit_type.0) else {
                continue;
            };
            max_unit_radius = max_unit_radius.max(unit.radius);
            if unit.movement.is_none() {
                static_footprints.push(StaticFootprint {
                    position: position.0,
                    radius: unit.radius,
                });
            } else {
                reserved.push((
                    Some(id.0),
                    ReservedFootprint {
                        position: entity
                            .get::<MoveTarget>()
                            .map_or(position.0, |target| target.0),
                        radius: unit.radius,
                    },
                ));
            }
        }
        let bounds = world
            .get_resource::<crate::world::MapBounds>()
            .map_or(Vec2::new(64.0, 64.0), |bounds| bounds.size);
        index_cache.context = Some(SpreadContext::new(
            reserved,
            static_footprints,
            bounds,
            max_unit_radius,
        ));
        index_cache.tick = Some(tick);
        #[cfg(test)]
        {
            index_cache.context_builds += 1;
        }
    }
    let mut spread_context = index_cache.context.take().unwrap();
    let mut reservation_candidates = BTreeMap::<u8, VecDeque<(u32, PlainMoveReservation)>>::new();
    if let Some(reservations) = world.get_resource::<PlainMoveReservations>() {
        for &unit_id in &reservations.winners {
            let Some(reservation) = reservations.entries.get(&unit_id) else {
                continue;
            };
            #[cfg(test)]
            count_plain_candidate_index_step();
            reservation_candidates
                .entry(reservation.slot.0)
                .or_default()
                .push_back((unit_id, reservation.clone()));
        }
    }
    for candidates in reservation_candidates.values_mut() {
        let mut ordered: Vec<_> = candidates.drain(..).collect();
        ordered
            .sort_by_key(|(unit_id, entry)| (entry.accepted_tick, entry.arrival_ordinal, *unit_id));
        candidates.extend(ordered);
    }
    for offset in 0..PENDING_SLOTS {
        let slot = (start_slot + offset) % PENDING_SLOTS;
        let candidates = reservation_candidates.entry(slot).or_default();
        loop {
            let candidate = candidates.front().cloned();
            #[cfg(test)]
            if candidate.is_some() {
                count_plain_candidate_index_step();
            }
            let queued_head = world
                .resource::<PendingMovementOrders>()
                .queues
                .get(&slot)
                .and_then(VecDeque::front)
                .cloned();
            let choose_candidate = match (&candidate, &queued_head) {
                (Some((unit_id, entry)), Some(request)) => {
                    reservation_precedes_request(*unit_id, entry, request)
                }
                (Some(_), None) => true,
                _ => false,
            };
            if choose_candidate {
                if used > 0 && used.saturating_add(1) > SPREAD_CAP {
                    break;
                }
                let (unit_id, _) = candidates.pop_front().unwrap();
                if let Some(mut reservations) = world.get_resource_mut::<PlainMoveReservations>() {
                    reservations.remove(&unit_id);
                }
                record_plain_dispatch_selection(world, tick, start_slot, 1);
                used = used.saturating_add(1);
                continue;
            }
            let Some(mut request) = queued_head else {
                break;
            };
            let target_gone_ids: Vec<_> = request
                .unit_ids
                .iter()
                .copied()
                .filter(|unit_id| {
                    request
                        .order_for_unit(*unit_id)
                        .and_then(|order| match order.target {
                            OrderTargetSim::Unit(target_id) => Some(target_id),
                            _ => None,
                        })
                        .is_some_and(|target_id| !entity_by_unit.contains_key(&target_id))
                })
                .collect();
            for unit_id in target_gone_ids {
                let Some(entity) = entity_by_unit.get(&unit_id).copied() else {
                    continue;
                };
                let failure_order = request.order_for_unit(unit_id).cloned();
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    unit_id,
                    |world| {
                        let mut pending = world.resource_mut::<PendingMovementOrders>();
                        let queue = pending
                            .queues
                            .get_mut(&slot)
                            .expect("the target-gone movement envelope remains queued");
                        let head = queue
                            .front_mut()
                            .expect("the target-gone movement envelope remains at its head");
                        debug_assert_eq!(head.pool_id, request.pool_id);
                        head.unit_ids.retain(|member| *member != unit_id);
                        head.member_orders.remove(&unit_id);
                        let remaining = pending
                            .pending_units
                            .get(&slot)
                            .copied()
                            .unwrap_or_default()
                            .saturating_sub(1);
                        pending.pending_units.insert(slot, remaining);
                        decrement_member_count(&mut pending.pending_member_counts, unit_id);
                        if pending
                            .queues
                            .get(&slot)
                            .and_then(VecDeque::front)
                            .is_some_and(|head| head.unit_ids.is_empty())
                        {
                            pending.pop_front(slot);
                        }
                        let tick = world.resource::<crate::world::Tick>().0;
                        let mut actor = world.entity_mut(entity);
                        actor.remove::<MoveTarget>();
                        actor.remove::<MoveProgress>();
                        actor.remove::<crate::movement::PlainMoveProgress>();
                        if failure_order.is_some() && actor.get::<UnitOrders>().is_none() {
                            actor.insert(UnitOrders::default());
                        }
                        if let (Some(order), Some(mut orders)) =
                            (failure_order.clone(), actor.get_mut::<UnitOrders>())
                        {
                            orders.current = None;
                            orders.last_failure = Some(OrderFailure {
                                order: order.order,
                                reason: crate::command::RejectReason::TargetGone,
                                tick,
                            });
                        }
                        forget_plain_move_identity(world, unit_id);
                        clear_gather(world, unit_id);
                        start_next_queued_order(world, SlotId(slot), entity, unit_id);
                    },
                );
                let actor = world.entity(entity);
                if let (Some(position), Some(unit_type)) = (
                    actor.get::<crate::world::Position>(),
                    actor.get::<crate::world::UnitType>(),
                ) {
                    let reserved_position = actor
                        .get::<MoveTarget>()
                        .map_or(position.0, |target| target.0);
                    let radius = world
                        .resource::<crate::registry::UnitTypes>()
                        .get(unit_type.0)
                        .map_or(0.5, |entry| entry.radius);
                    spread_context.reserve_unit(
                        unit_id,
                        ReservedFootprint {
                            position: reserved_position,
                            radius,
                        },
                    );
                }
                request.unit_ids.retain(|member| *member != unit_id);
                request.member_orders.remove(&unit_id);
            }
            if request.unit_ids.is_empty() {
                continue;
            }
            let live_ids: Vec<_> = request
                .unit_ids
                .iter()
                .copied()
                .filter(|unit_id| entity_by_unit.contains_key(unit_id))
                .collect();
            if live_ids.len() != request.unit_ids.len() {
                let live_set: std::collections::HashSet<_> = live_ids.iter().copied().collect();
                let removed_ids: Vec<_> = request
                    .unit_ids
                    .iter()
                    .copied()
                    .filter(|unit_id| !live_set.contains(unit_id))
                    .collect();
                crate::economy::transaction::capture_orders_operation(world, |world| {
                    let mut pending = world.resource_mut::<PendingMovementOrders>();
                    if live_ids.is_empty() {
                        pending
                            .pop_front(slot)
                            .expect("the all-dead pending envelope remains at the queue head");
                    } else {
                        let queued_units = pending
                            .pending_units
                            .get(&slot)
                            .copied()
                            .unwrap_or_default();
                        let queue = pending.queues.get_mut(&slot).unwrap();
                        let head = queue.front_mut().unwrap();
                        debug_assert_eq!(head.pool_id, request.pool_id);
                        head.unit_ids = live_ids.clone();
                        head.member_orders
                            .retain(|unit_id, _| live_set.contains(unit_id));
                        pending
                            .pending_units
                            .insert(slot, queued_units.saturating_sub(removed_ids.len()));
                        for id in &removed_ids {
                            decrement_member_count(&mut pending.pending_member_counts, *id);
                        }
                    }
                });
                if live_ids.is_empty() {
                    continue;
                }
                request.unit_ids = live_ids;
            }
            if let Some(anchor_id) = request.unit_ids.first().copied()
                && let Some(Order {
                    target: OrderTargetSim::Unit(target_id),
                    ..
                }) = request.order_for_unit(anchor_id)
                && let Some(target_entity) = entity_by_unit.get(target_id)
                && let Some(target_position) =
                    world.entity(*target_entity).get::<crate::world::Position>()
            {
                let max_radius = request
                    .unit_ids
                    .iter()
                    .filter_map(|unit_id| entity_by_unit.get(unit_id))
                    .map(|entity| unit_radius(world, *entity))
                    .fold(0.0_f32, f32::max);
                request.point = clamp_target(world, target_position.0, max_radius);
            }
            // Dead members are pruned before the shared tick cap is evaluated, so
            // the surviving envelope keeps its original arrival order and uses its
            // actual live-member count for admission.
            let next_count = request.unit_ids.len();
            if next_count == 0 || (used > 0 && used.saturating_add(next_count) > SPREAD_CAP) {
                break;
            }
            let members: Vec<_> = request
                .unit_ids
                .iter()
                .filter_map(|unit_id| {
                    let entity = *entity_by_unit.get(unit_id)?;
                    let entity_ref = world.entity(entity);
                    let position = entity_ref.get::<crate::world::Position>()?.0;
                    let unit_type = entity_ref.get::<crate::world::UnitType>()?.0;
                    let radius = world
                        .resource::<crate::registry::UnitTypes>()
                        .get(unit_type)
                        .map_or(0.5, |entry| entry.radius);
                    Some((
                        entity,
                        SpreadMember {
                            id: *unit_id,
                            position,
                            radius,
                        },
                    ))
                })
                .collect();
            let moving_ids: std::collections::HashSet<_> =
                request.unit_ids.iter().copied().collect();
            let input: Vec<_> = members.iter().map(|(_, member)| *member).collect();
            let (mut pool, assignments) = spread_destinations_with_context(
                &input,
                request.point,
                &spread_context,
                &moving_ids,
            );
            if members.len() != request.unit_ids.len()
                || assignments.len() != request.unit_ids.len()
            {
                // Fail an impossible envelope atomically once its spread turn arrives.
                if members.is_empty() {
                    world
                        .resource_mut::<PendingMovementOrders>()
                        .pop_front(slot);
                }
                let member_count = members.len();
                for (index, (entity, member)) in members.into_iter().enumerate() {
                    let last = index + 1 == member_count;
                    let failure_order = request.order_for_unit(member.id).cloned();
                    crate::economy::transaction::capture_unit_order_operation(
                        world,
                        member.id,
                        |world| {
                            if last {
                                let popped = world
                                    .resource_mut::<PendingMovementOrders>()
                                    .pop_front(slot)
                                    .expect("the failed spread envelope remains at its queue head");
                                debug_assert_eq!(popped.pool_id, request.pool_id);
                            }
                            let tick = world.resource::<crate::world::Tick>().0;
                            let idless_legacy = request.plain
                                && request.order.is_none()
                                && request.member_orders.is_empty();
                            {
                                let mut entity_mut = world.entity_mut(entity);
                                entity_mut.remove::<MoveTarget>();
                                entity_mut.remove::<MoveProgress>();
                                entity_mut.remove::<crate::movement::PlainMoveProgress>();
                                if idless_legacy
                                    && let Some(mut orders) = entity_mut.get_mut::<UnitOrders>()
                                {
                                    orders.current = None;
                                    orders.last_failure = None;
                                }
                                if let (Some(order), Some(mut orders)) =
                                    (failure_order.clone(), entity_mut.get_mut::<UnitOrders>())
                                {
                                    orders.current = None;
                                    orders.last_failure = Some(OrderFailure {
                                        order: order.order,
                                        reason: crate::command::RejectReason::Blocked,
                                        tick,
                                    });
                                }
                            }
                            if failure_order.is_some() {
                                clear_gather(world, member.id);
                            } else if request.plain {
                                clear_gather(world, member.id);
                                world.entity_mut(entity).insert(
                                    crate::movement::PlainMoveProgress {
                                        original_point: request.point,
                                        slot: member.position,
                                        pool_slots: Vec::new(),
                                        gather_goal: None,
                                        activity: MovementActivity::Blocked,
                                        stalled_ticks: 0,
                                        best_distance: member.position.distance(request.point),
                                        claim_count: 0,
                                        failure_recorded: true,
                                    },
                                );
                            }
                            forget_plain_move_identity(world, member.id);
                        },
                    );
                    spread_context.reserve_unit(
                        member.id,
                        ReservedFootprint {
                            position: member.position,
                            radius: member.radius,
                        },
                    );
                }
                break;
            }
            pool.pool_id = request.pool_id;
            let order_by_unit = request.member_orders.clone();
            let assignment_by_unit: BTreeMap<_, _> = assignments.into_iter().collect();
            for (_, member) in &members {
                if let Some(target) = assignment_by_unit.get(&member.id) {
                    spread_context.reserve_unit(
                        member.id,
                        ReservedFootprint {
                            position: *target,
                            radius: member.radius,
                        },
                    );
                }
            }
            let mut first_operation = true;
            for (entity, member) in members {
                let Some(target) = assignment_by_unit.get(&member.id).copied() else {
                    continue;
                };
                let order = order_by_unit
                    .get(&member.id)
                    .cloned()
                    .or_else(|| request.order.clone());
                let replace_order = order.is_some();
                let follow_origin = order.as_ref().and_then(|order| {
                    let OrderTargetSim::Unit(target_id) = &order.target else {
                        return None;
                    };
                    let target_position = entity_by_unit
                        .get(target_id)
                        .and_then(|entity| world.entity(*entity).get::<crate::world::Position>())?
                        .0;
                    Some(crate::movement::FollowMoveOrigin {
                        target_unit_id: *target_id,
                        goal: clamp_target(world, target_position, member.radius),
                    })
                });
                let first = first_operation;
                first_operation = false;
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    member.id,
                    |world| {
                        if first {
                            let popped = world
                                .resource_mut::<PendingMovementOrders>()
                                .pop_front(slot)
                                .expect("the selected spread envelope remains at its queue head");
                            debug_assert_eq!(popped.pool_id, request.pool_id);
                            if request.plain {
                                if let Some(mut reservations) =
                                    world.get_resource_mut::<PlainMoveReservations>()
                                {
                                    for unit_id in &request.unit_ids {
                                        reservations.remove(unit_id);
                                    }
                                }
                                record_plain_dispatch_selection(
                                    world, tick, start_slot, next_count,
                                );
                            } else {
                                record_dispatch_selection(world, tick, start_slot, next_count);
                            }
                            world
                                .resource_mut::<MovementPools>()
                                .0
                                .insert(request.pool_id, pool.clone());
                        }
                        {
                            let mut entity_mut = world.entity_mut(entity);
                            entity_mut.remove::<crate::movement::PlainMoveProgress>();
                            entity_mut.insert(MoveTarget(target));
                            entity_mut.insert(MoveProgress {
                                pool_id: request.pool_id,
                                slot: target,
                                activity: MovementActivity::Moving,
                                pass_mode: false,
                                stalling_on_group: false,
                                stalled_ticks: 0,
                                best_distance: member.position.distance(target),
                                claim_count: 0,
                                follow_origin,
                            });
                            if let Some(order) = order
                                && let Some(mut orders) = entity_mut.get_mut::<UnitOrders>()
                            {
                                orders.last_failure = None;
                                orders.current = Some(order);
                            } else if request.plain
                                && request.member_orders.is_empty()
                                && let Some(mut orders) = entity_mut.get_mut::<UnitOrders>()
                            {
                                // A legacy MoveTo Replace keeps only Appends accepted while
                                // this envelope waited, not the replaced current order.
                                orders.last_failure = None;
                                orders.current = None;
                            }
                        }
                        if replace_order || (request.plain && request.member_orders.is_empty()) {
                            forget_plain_move_identity(world, member.id);
                            clear_gather(world, member.id);
                        }
                    },
                );
            }
            used = used.saturating_add(next_count);
        }
    }
    index_cache.context = Some(spread_context);
    world.insert_resource(index_cache);
}
