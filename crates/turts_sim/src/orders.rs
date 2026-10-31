//! Per-unit orders and the fixed-tick order phase.

use std::collections::{BTreeMap, VecDeque};

use bevy_ecs::prelude::{Component, Resource, World};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use turts_mod_api::{Behaviour, ContentId};

use crate::world::MoveTarget;
use crate::{JobId, NodeId, SlotId};

/// A complete move envelope waiting for its atomic spread turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingMovementOrder {
    pub slot: SlotId,
    pub accepted_tick: u32,
    pub seq: u32,
    pub group: u32,
    pub pool_id: u64,
    pub point: Vec2,
    pub unit_ids: Vec<u32>,
    pub order: Option<Order>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub member_orders: BTreeMap<u32, Order>,
    pub plain: bool,
}

impl PendingMovementOrder {
    pub fn order_for_unit(&self, unit_id: u32) -> Option<&Order> {
        if self.member_orders.is_empty() {
            self.order.as_ref()
        } else {
            self.member_orders.get(&unit_id)
        }
    }
}

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

/// Authored identity for a legacy-style point move, kept out of movement inputs.
/// Audit replay reconstructs this transient cache from validated command boundaries.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct PlainMoveOrderIdentities(pub BTreeMap<u32, Order>);

#[derive(Resource, Default)]
struct SpreadIndexCache {
    tick: Option<u32>,
    context: Option<crate::movement::SpreadContext>,
    #[cfg(test)]
    context_builds: u32,
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
        let mut canceled = 0;
        let mut canceled_members = BTreeMap::<u32, usize>::new();
        for request in queue.iter_mut() {
            let removed_from_request: Vec<_> = request
                .unit_ids
                .iter()
                .copied()
                .filter(|id| removed.contains(id))
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

    fn can_queue_after_replacing(
        &self,
        slot: u8,
        _group: u32,
        _tick: u32,
        unit_ids: &[u32],
    ) -> bool {
        let replacement: std::collections::HashSet<_> = unit_ids.iter().copied().collect();
        let mut remaining = self.pending_units.get(&slot).copied().unwrap_or_default();
        if let Some(queue) = self.queues.get(&slot) {
            let removed = queue
                .iter()
                .map(|request| {
                    request
                        .unit_ids
                        .iter()
                        .filter(|id| replacement.contains(id))
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

/// The target carried by one simulation order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderTargetSim {
    Invalid,
    None,
    Point(#[serde(with = "crate::command::vec2_bits")] Vec2),
    Unit(u32),
    Node(NodeId),
    Job(JobId),
    Content(ContentId),
}

/// Whether an order replaces or appends to a unit's queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueMode {
    Replace,
    Append,
}

/// One resolved order stored on a unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub order: ContentId,
    pub behaviour: Behaviour,
    pub target: OrderTargetSim,
    pub group: u32,
}

/// Orders currently assigned to one unit.
#[derive(Component, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UnitOrders {
    pub current: Option<Order>,
    pub queue: VecDeque<Order>,
    pub last_failure: Option<OrderFailure>,
}

/// The last accepted order failure, retained for the owner-facing projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderFailure {
    pub order: ContentId,
    pub reason: crate::command::RejectReason,
    pub tick: u32,
}

/// The order request after server-side content resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderCommand {
    pub order: ContentId,
    pub behaviour: Behaviour,
    pub target_kind: turts_mod_api::TargetKind,
    /// The server-captured definition used to resolve this envelope.
    pub policy: Option<OrderPolicyAudit>,
    pub units: Vec<u32>,
    pub target: OrderTargetSim,
    pub queue: QueueMode,
    pub group: u32,
    pub allowed_units: Vec<String>,
    pub queue_depth: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderPolicyAudit {
    pub id: ContentId,
    pub behaviour: Behaviour,
    pub target: turts_mod_api::TargetKind,
    pub units: Vec<String>,
    pub queue_depth: u8,
    pub unit_capabilities: Vec<OrderUnitCapabilitiesAudit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderUnitCapabilitiesAudit {
    pub unit: String,
    pub movement: bool,
    pub weapon: bool,
    pub gatherer: bool,
    pub producer: bool,
    pub research: bool,
    pub removable: bool,
    pub toggleable: bool,
}

/// Per-unit outcome of an order envelope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitOutcomes {
    pub accepted: Vec<u32>,
    pub refused: Vec<(u32, crate::command::RejectReason)>,
}

pub(crate) fn unit_order_capable(world: &World, kind: turts_mod_api::UnitTypeId) -> bool {
    let Some(entry) = world.resource::<crate::registry::UnitTypes>().get(kind) else {
        return false;
    };
    entry.movement.is_some()
        || world
            .get_resource::<crate::bootstrap::InstalledCatalog>()
            .is_some_and(|catalog| {
                catalog
                    .0
                    .units
                    .values()
                    .any(|unit| unit.id.to_string() == entry.content_id && unit.weapon.is_some())
            })
}

fn unit_toggleable(unit: &turts_mod_api::ResolvedUnit) -> bool {
    unit.extractor.is_some()
        || unit.inserter.is_some()
        || unit.machine.is_some()
        || unit.research.is_some()
        || unit.producer.is_some()
        || unit.weapon.is_some()
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

pub(crate) struct MovementEnvelopeRequest {
    pub(crate) slot: SlotId,
    pub(crate) seq: u32,
    pub(crate) group: u32,
    pub(crate) point: Vec2,
    pub(crate) unit_ids: Vec<u32>,
    pub(crate) order: Option<Order>,
    pub(crate) member_orders: BTreeMap<u32, Order>,
    pub(crate) plain: bool,
}

/// Run one fixed-tick order phase.
pub(crate) fn run_orders(world: &mut World) {
    dispatch_pending_movement(world);
    let unit_entities: BTreeMap<_, _> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<crate::world::UnitId>()
                .map(|id| (id.0, entity.id()))
        })
        .collect();
    for (&unit_id, &entity) in &unit_entities {
        if unit_has_pending_move(world, unit_id) {
            continue;
        }
        let no_plain_move_state = world.entity(entity).get::<MoveTarget>().is_none()
            && world
                .entity(entity)
                .get::<crate::movement::PlainMoveProgress>()
                .is_none();
        let has_current_order = world
            .entity(entity)
            .get::<UnitOrders>()
            .is_some_and(|orders| orders.current.is_some());
        if no_plain_move_state && !has_current_order {
            forget_plain_move_identity(world, unit_id);
        }
        let plain_progress = world
            .entity(entity)
            .get::<crate::movement::PlainMoveProgress>()
            .cloned();
        let gather_stage = world
            .get_resource::<crate::economy::EconomyOrders>()
            .and_then(|orders| orders.0.get(&unit_id))
            .map(|order| order.stage);
        let gather_goal = match gather_stage {
            Some(crate::economy::gather::GatherStage::MovingToNode) => world
                .get_resource::<crate::economy::EconomyOrders>()
                .and_then(|orders| orders.0.get(&unit_id))
                .and_then(|order| order.deposit)
                .map(|node| crate::movement::PlainGatherGoal::Node(node.0)),
            Some(crate::economy::gather::GatherStage::MovingToReceiver) => world
                .get_resource::<crate::economy::EconomyOrders>()
                .and_then(|orders| orders.0.get(&unit_id))
                .and_then(|order| order.receiver)
                .map(crate::movement::PlainGatherGoal::Receiver),
            _ => None,
        };
        let gather_reached = matches!(
            gather_stage,
            Some(
                crate::economy::gather::GatherStage::Gathering
                    | crate::economy::gather::GatherStage::Unloading
            )
        );
        let gather_endpoint_changed = plain_progress
            .as_ref()
            .filter(|progress| progress.activity != crate::movement::MovementActivity::Blocked)
            .and_then(|progress| progress.gather_goal)
            .is_some_and(|previous| Some(previous) != gather_goal);
        if plain_progress.is_some() && (gather_reached || gather_endpoint_changed) {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
                forget_plain_move_identity(world, unit_id);
            });
            continue;
        }
        let completed_plain_fallback = plain_progress.as_ref().is_some_and(|progress| {
            progress.activity == crate::movement::MovementActivity::Resting
                && (progress.claim_count > 0 || progress.slot != progress.original_point)
        });
        let gather_still_approaching = matches!(
            gather_stage,
            Some(
                crate::economy::gather::GatherStage::MovingToNode
                    | crate::economy::gather::GatherStage::MovingToReceiver
            )
        );
        if completed_plain_fallback && !gather_still_approaching {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
                forget_plain_move_identity(world, unit_id);
            });
        }
        let plain_blocked = world
            .entity(entity)
            .get::<crate::movement::PlainMoveProgress>()
            .is_some_and(|progress| {
                (progress.activity == crate::movement::MovementActivity::Blocked
                    && !progress.failure_recorded)
                    || (completed_plain_fallback && gather_still_approaching)
            });
        if plain_blocked {
            let slot = world
                .entity(entity)
                .get::<crate::world::Owner>()
                .map_or(SlotId(0), |owner| owner.0);
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                let current = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .and_then(|orders| orders.current.clone());
                let identity = world
                    .get_resource::<PlainMoveOrderIdentities>()
                    .and_then(|identities| identities.0.get(&unit_id))
                    .cloned();
                let has_queued = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .is_some_and(|orders| !orders.queue.is_empty());
                let had_current = current.is_some();
                let failure_order = current
                    .as_ref()
                    .map(|current| current.order.clone())
                    .or_else(|| identity.as_ref().map(|order| order.order.clone()));
                if let Some(order) = failure_order {
                    let tick = world.resource::<crate::world::Tick>().0;
                    let mut entity_mut = world.entity_mut(entity);
                    if entity_mut.get::<UnitOrders>().is_none() {
                        entity_mut.insert(UnitOrders::default());
                    }
                    if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                        orders.last_failure = Some(OrderFailure {
                            order,
                            reason: crate::command::RejectReason::Blocked,
                            tick,
                        });
                        orders.current = None;
                    }
                }
                if let Some(mut progress) = world
                    .entity_mut(entity)
                    .get_mut::<crate::movement::PlainMoveProgress>()
                {
                    progress.activity = crate::movement::MovementActivity::Blocked;
                    progress.failure_recorded = true;
                }
                clear_gather(world, unit_id);
                forget_plain_move_identity(world, unit_id);
                if has_queued || had_current {
                    start_next_queued_order(world, slot, entity, unit_id);
                }
            });
            continue;
        }
        let current = world
            .entity(entity)
            .get::<UnitOrders>()
            .and_then(|orders| orders.current.clone());
        let blocked = world
            .entity(entity)
            .get::<crate::movement::MoveProgress>()
            .is_some_and(|progress| {
                progress.activity == crate::movement::MovementActivity::Blocked
                    && world.entity(entity).get::<MoveTarget>().is_none()
            });
        if let Some(current) = current.as_ref().filter(|_| blocked) {
            let slot = world
                .entity(entity)
                .get::<crate::world::Owner>()
                .map_or(SlotId(0), |owner| owner.0);
            let tick = world.resource::<crate::world::Tick>().0;
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                {
                    let mut entity_mut = world.entity_mut(entity);
                    if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                        orders.last_failure = Some(OrderFailure {
                            order: current.order.clone(),
                            reason: crate::command::RejectReason::Blocked,
                            tick,
                        });
                        orders.current = None;
                    }
                }
                start_next_queued_order(world, slot, entity, unit_id);
            });
            continue;
        }
        if current.is_none() {
            let waiting = world
                .entity(entity)
                .get::<UnitOrders>()
                .is_some_and(|orders| !orders.queue.is_empty());
            let plain_move_landed = world
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .is_none();
            if waiting && plain_move_landed {
                let slot = world
                    .entity(entity)
                    .get::<crate::world::Owner>()
                    .map_or(SlotId(0), |owner| owner.0);
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    unit_id,
                    |world| {
                        start_next_queued_order(world, slot, entity, unit_id);
                    },
                );
            }
            continue;
        }
        let current = current.unwrap();
        if current.behaviour == Behaviour::Move
            && let OrderTargetSim::Unit(target_id) = current.target
        {
            run_unit_follow_move(world, entity, unit_id, current, target_id, &unit_entities);
            continue;
        }
        let completed_move = current.behaviour == Behaviour::Move
            && world
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .is_none();
        let depleted_gather = current.behaviour == Behaviour::Gather
            && match current.target {
                OrderTargetSim::Node(node) => world
                    .get_resource::<crate::economy::SurfaceNodes>()
                    .is_some_and(|nodes| {
                        nodes
                            .0
                            .iter()
                            .any(|record| record.id == node && record.remaining == 0)
                    }),
                _ => false,
            };
        if !completed_move && !depleted_gather {
            continue;
        }
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            let next = {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                if depleted_gather {
                    orders.last_failure = Some(OrderFailure {
                        order: current.order.clone(),
                        reason: crate::command::RejectReason::NodeDepleted,
                        tick,
                    });
                }
                orders.current = None;
                orders.queue.pop_front()
            };
            if depleted_gather {
                let _ = crate::economy::gather::interrupt_order(world, slot, unit_id);
            }
            if next.is_some() {
                if let Some(next) = next {
                    world
                        .entity_mut(entity)
                        .get_mut::<UnitOrders>()
                        .unwrap()
                        .current = Some(next);
                }
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
    }
    queue_unallocated_move_orders(world);
    dispatch_pending_movement(world);
    finish_movement_dispatch_tick(world);
}

fn finish_movement_dispatch_tick(world: &mut World) {
    const PENDING_SLOTS: u8 = 8;
    let tick = world.resource::<crate::world::Tick>().0;
    let (start_slot, advance_cursor, initialized) = {
        let state = world.resource::<MovementDispatchState>();
        let pending = world.resource::<PendingMovementOrders>();
        if state.tick == Some(tick) {
            (state.start_slot, !state.cursor_advanced, true)
        } else {
            (
                dispatch_start_slot(pending, state, tick),
                true,
                state.tick.is_some()
                    || pending.next_pool_id != 0
                    || pending.next_slot != 0
                    || !pending.queues.is_empty(),
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
        state.used = 0;
        state.cursor_advanced = true;
    }
    world.resource_mut::<PendingMovementOrders>().next_slot = (start_slot + 1) % PENDING_SLOTS;
}

fn dispatch_start_slot(
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

fn queue_unallocated_move_orders(world: &mut World) {
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
            let Some(order) = orders
                .current
                .as_ref()
                .filter(|order| order.behaviour == Behaviour::Move)
            else {
                continue;
            };
            let OrderTargetSim::Point(raw_point) = order.target else {
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
            let raw_point = match members[0].2.target {
                OrderTargetSim::Point(point) => point,
                _ => continue,
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

fn dispatch_pending_movement(world: &mut World) {
    use crate::movement::{
        MoveProgress, MovementActivity, MovementPools, ReservedFootprint, SpreadContext,
        SpreadMember, StaticFootprint, spread_destinations_with_context,
    };
    const SPREAD_CAP: usize = 2048;
    const PENDING_SLOTS: u8 = 8;

    let tick = world.resource::<crate::world::Tick>().0;
    let pending = world.resource::<PendingMovementOrders>();
    let (start_slot, mut used) = {
        let state = world.resource::<MovementDispatchState>();
        if state.tick == Some(tick) {
            (state.start_slot, state.used)
        } else {
            (dispatch_start_slot(pending, state, tick), 0)
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
    for offset in 0..PENDING_SLOTS {
        let slot = (start_slot + offset) % PENDING_SLOTS;
        loop {
            let Some(mut request) = world
                .resource::<PendingMovementOrders>()
                .queues
                .get(&slot)
                .and_then(VecDeque::front)
                .cloned()
            else {
                break;
            };
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
                            {
                                let mut entity_mut = world.entity_mut(entity);
                                entity_mut.remove::<MoveTarget>();
                                entity_mut.remove::<MoveProgress>();
                                entity_mut.remove::<crate::movement::PlainMoveProgress>();
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
                            let advance_cursor = {
                                let mut state = world.resource_mut::<MovementDispatchState>();
                                if state.tick != Some(tick) {
                                    state.tick = Some(tick);
                                    state.start_slot = start_slot;
                                    state.used = 0;
                                    state.cursor_advanced = false;
                                }
                                state.used = state.used.saturating_add(next_count);
                                if !state.cursor_advanced {
                                    state.cursor_advanced = true;
                                    true
                                } else {
                                    false
                                }
                            };
                            if advance_cursor {
                                world.resource_mut::<PendingMovementOrders>().next_slot =
                                    (start_slot + 1) % PENDING_SLOTS;
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
                            });
                            if let Some(order) = order {
                                if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                                    orders.last_failure = None;
                                    orders.current = Some(order);
                                }
                            }
                        }
                        if replace_order {
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

fn run_unit_follow_move(
    world: &mut World,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    current: Order,
    target_id: u32,
    entity_by_unit: &BTreeMap<u32, bevy_ecs::entity::Entity>,
) {
    let target_position = entity_by_unit.get(&target_id).and_then(|target| {
        world
            .entity(*target)
            .get::<crate::world::Position>()
            .map(|pos| pos.0)
    });
    let current_position = world
        .entity(entity)
        .get::<crate::world::Position>()
        .map(|pos| pos.0);
    let Some(target_position) = target_position else {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                orders.last_failure = Some(OrderFailure {
                    order: current.order.clone(),
                    reason: crate::command::RejectReason::TargetGone,
                    tick,
                });
                orders.current = None;
            }
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            start_next_queued_order(world, slot, entity, unit_id);
        });
        return;
    };
    let Some(current_position) = current_position else {
        return;
    };
    if current_position == target_position {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                orders.current = None;
            }
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            start_next_queued_order(world, slot, entity, unit_id);
        });
    } else {
        let move_target = world
            .entity(entity)
            .get::<crate::world::MoveTarget>()
            .map(|target| target.0);
        if move_target != Some(target_position) {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .insert(crate::world::MoveTarget(target_position));
            });
        }
    }
}

fn start_next_queued_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit: u32,
) {
    loop {
        let next = {
            let mut entity_mut = world.entity_mut(entity);
            let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
            let Some(next) = orders.current.take().or_else(|| orders.queue.pop_front()) else {
                return;
            };
            orders.current = Some(next.clone());
            next
        };
        if let Err(reason) = start_order(world, slot, entity, unit, &next) {
            let tick = world.resource::<crate::world::Tick>().0;
            let mut entity_mut = world.entity_mut(entity);
            let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
            orders.current = None;
            orders.last_failure = Some(OrderFailure {
                order: next.order,
                reason,
                tick,
            });
            continue;
        }
        break;
    }
}

fn start_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit: u32,
    order: &Order,
) -> Result<(), crate::command::RejectReason> {
    match (order.behaviour, &order.target) {
        (Behaviour::Move, OrderTargetSim::Point(point)) => {
            let target = clamp_target(world, *point, unit_radius(world, entity));
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            entity_mut.insert(crate::world::MoveTarget(target));
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Move, OrderTargetSim::Unit(target_id)) => {
            let target = world
                .iter_entities()
                .find(|candidate| {
                    candidate
                        .get::<crate::world::UnitId>()
                        .is_some_and(|id| id.0 == *target_id)
                })
                .and_then(|candidate| candidate.get::<crate::world::Position>().map(|pos| pos.0))
                .ok_or(crate::command::RejectReason::TargetGone)?;
            let target = clamp_target(world, target, unit_radius(world, entity));
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            entity_mut.insert(crate::world::MoveTarget(target));
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Hold, _) => {
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            world
                .entity_mut(entity)
                .remove::<crate::movement::MoveProgress>();
            world
                .entity_mut(entity)
                .remove::<crate::movement::PlainMoveProgress>();
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Gather, OrderTargetSim::Node(node)) => {
            let state = world
                .get_resource::<crate::economy::SurfaceNodes>()
                .and_then(|nodes| nodes.0.iter().find(|record| record.id == *node));
            if state.is_none() {
                return Err(crate::command::RejectReason::TargetGone);
            }
            if state.is_some_and(|record| record.remaining == 0) {
                return Err(crate::command::RejectReason::NodeDepleted);
            }
            crate::economy::gather::assign_gather(world, slot, unit, *node)
        }
        _ => Err(crate::command::RejectReason::NotApplicable),
    }
}

/// Apply one server-resolved order envelope, validating each unit independently.
pub(crate) fn apply_order(
    world: &mut World,
    slot: SlotId,
    seq: u32,
    command: &OrderCommand,
) -> (
    Result<crate::command::Applied, crate::command::RejectReason>,
    UnitOutcomes,
) {
    use crate::command::RejectReason as R;

    if command.units.is_empty() {
        return (Err(R::EmptyCommand), UnitOutcomes::default());
    }
    if command.units.len() > crate::command::MAX_UNITS_PER_COMMAND {
        return (Err(R::TooManyUnits), UnitOutcomes::default());
    }
    let mut seen = std::collections::HashSet::with_capacity(command.units.len());
    if command.units.iter().any(|unit| !seen.insert(*unit)) {
        return (Err(R::DuplicateUnit), UnitOutcomes::default());
    }

    let mut unit_ids = command.units.clone();
    unit_ids.sort_unstable();
    let entity_by_unit: BTreeMap<_, _> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<crate::world::UnitId>()
                .map(|id| (id.0, entity.id()))
        })
        .collect();
    let mut outcomes = UnitOutcomes::default();
    let mut first_job_id = None;
    let mut spread_units = Vec::new();
    let mut spread_order = None;
    let immediate = matches!(
        command.behaviour,
        Behaviour::Produce
            | Behaviour::CancelProduction
            | Behaviour::Remove
            | Behaviour::SetEnabled(_)
    );
    for unit_id in &unit_ids {
        let Some(entity) = entity_by_unit.get(unit_id).copied() else {
            outcomes.refused.push((*unit_id, R::UnknownUnit));
            continue;
        };
        let owner = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map(|owner| owner.0);
        if owner != Some(slot) {
            outcomes.refused.push((*unit_id, R::NotOwner));
            continue;
        }
        let Some(unit_type) = world
            .entity(entity)
            .get::<crate::world::UnitType>()
            .map(|kind| kind.0)
        else {
            outcomes.refused.push((*unit_id, R::NotApplicable));
            continue;
        };
        let unit_id_text = world
            .resource::<crate::registry::UnitTypes>()
            .get(unit_type)
            .map(|entry| entry.content_id.clone());
        let registry_movement = world
            .resource::<crate::registry::UnitTypes>()
            .get(unit_type)
            .is_some_and(|entry| entry.movement.is_some());
        let definition = world
            .get_resource::<crate::bootstrap::InstalledCatalog>()
            .and_then(|catalog| {
                let content_id = unit_id_text.as_deref()?;
                catalog
                    .0
                    .units
                    .values()
                    .find(|unit| unit.id.to_string() == content_id)
            });
        let capable = match command.behaviour {
            Behaviour::Move => registry_movement,
            Behaviour::Stop | Behaviour::Hold => {
                definition.is_some_and(|unit| unit.weapon.is_some()) || registry_movement
            }
            Behaviour::Attack => match command.target {
                OrderTargetSim::Point(_) => registry_movement,
                OrderTargetSim::Unit(_) => definition.is_some_and(|unit| unit.weapon.is_some()),
                _ => false,
            },
            Behaviour::Gather => definition.is_some_and(|unit| unit.gatherer.is_some()),
            Behaviour::Produce => {
                definition.is_some_and(|unit| unit.producer.is_some() || unit.research.is_some())
            }
            Behaviour::CancelProduction => definition.is_some_and(|unit| {
                unit.producer.is_some() || unit.research.is_some() || unit.machine.is_some()
            }),
            Behaviour::Remove => definition
                .and_then(|unit| unit.removal.as_ref())
                .is_some_and(|removal| removal.by_command),
            Behaviour::SetEnabled(_) => definition.is_some_and(unit_toggleable),
        };
        if !capable {
            let reason = match command.behaviour {
                Behaviour::Gather | Behaviour::Produce | Behaviour::CancelProduction => {
                    R::MissingCapability
                }
                Behaviour::Remove => R::NotRemovable,
                Behaviour::SetEnabled(_) => R::NotToggleable,
                _ => R::NotApplicable,
            };
            outcomes.refused.push((*unit_id, reason));
            continue;
        }
        if !command.allowed_units.is_empty()
            && !unit_id_text
                .as_ref()
                .is_some_and(|id| command.allowed_units.iter().any(|allowed| allowed == id))
        {
            outcomes.refused.push((*unit_id, R::NotApplicable));
            continue;
        }
        if let Err(reason) = validate_target(world, command) {
            outcomes.refused.push((*unit_id, reason));
            continue;
        }
        if command.queue == QueueMode::Append && !immediate {
            let orders = world.entity(entity).get::<UnitOrders>();
            let pending_move = unit_has_pending_move(world, *unit_id);
            let effective_behaviour = if pending_move {
                Some(Behaviour::Move)
            } else {
                orders.and_then(|orders| orders.current.as_ref().map(|current| current.behaviour))
            };
            if effective_behaviour
                .is_some_and(|behaviour| matches!(behaviour, Behaviour::Hold | Behaviour::Gather))
                || orders.is_some_and(|orders| {
                    orders.queue.iter().any(|queued| {
                        matches!(queued.behaviour, Behaviour::Hold | Behaviour::Gather)
                    })
                })
            {
                outcomes.refused.push((*unit_id, R::NotQueueable));
                continue;
            }
            let legacy_gather_active = effective_behaviour != Some(Behaviour::Move)
                && world
                    .get_resource::<crate::economy::EconomyOrders>()
                    .and_then(|orders| orders.0.get(unit_id))
                    .is_some_and(|order| order.node.is_some());
            if legacy_gather_active {
                outcomes.refused.push((*unit_id, R::NotQueueable));
                continue;
            }
            if orders.is_some_and(|orders| orders.queue.len() >= command.queue_depth as usize) {
                outcomes.refused.push((*unit_id, R::QueueFull));
                continue;
            }
        }

        let order = Order {
            order: command.order.clone(),
            behaviour: command.behaviour,
            target: command.target.clone(),
            group: command.group,
        };
        let deferred_move = command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && (command.group != 0 || command.units.len() > 1)
            && matches!(&command.target, OrderTargetSim::Point(_));
        if deferred_move {
            spread_units.push(*unit_id);
            spread_order = Some(order);
            outcomes.accepted.push(*unit_id);
            continue;
        }

        let plain_move = command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && command.group == 0
            && command.units.len() == 1
            && matches!(&command.target, OrderTargetSim::Point(_))
            && plain_move_has_no_prior_order_state(world, command);
        let plain_gather = command.behaviour == Behaviour::Gather
            && command.queue == QueueMode::Replace
            && command.group == 0
            && !unit_has_pending_move(world, *unit_id)
            && world.entity(entity).get::<MoveTarget>().is_none()
            && world
                .entity(entity)
                .get::<crate::movement::MoveProgress>()
                .is_none()
            && world
                .entity(entity)
                .get::<crate::movement::PlainMoveProgress>()
                .is_none();
        if plain_move {
            let OrderTargetSim::Point(target) = &command.target else {
                outcomes.refused.push((*unit_id, R::TargetInvalid));
                continue;
            };
            let target = clamp_target(world, *target, unit_radius(world, entity));
            crate::command::apply_legacy_move_to(world, slot, &[*unit_id], target)
                .expect("order validation precedes legacy move application");
            remember_plain_move_identity(world, *unit_id, order);
            outcomes.accepted.push(*unit_id);
            continue;
        }
        if command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && command.group == 0
            && command.units.len() == 1
            && matches!(&command.target, OrderTargetSim::Point(_))
        {
            let OrderTargetSim::Point(target) = &command.target else {
                outcomes.refused.push((*unit_id, R::TargetInvalid));
                continue;
            };
            crate::economy::transaction::capture_unit_order_operation(world, *unit_id, |world| {
                world
                    .resource_mut::<PendingMovementOrders>()
                    .cancel_units(slot.0, &[*unit_id]);
                let mut movement = world.entity_mut(entity);
                movement.remove::<MoveTarget>();
                movement.remove::<crate::movement::MoveProgress>();
                movement.remove::<crate::movement::PlainMoveProgress>();
                crate::command::apply_legacy_move_to(
                    world,
                    slot,
                    &[*unit_id],
                    clamp_target(world, *target, unit_radius(world, entity)),
                )
                .expect("order validation precedes legacy move application");
                if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                    *orders = UnitOrders::default();
                }
                clear_gather(world, *unit_id);
                remember_plain_move_identity(world, *unit_id, order.clone());
            });
            outcomes.accepted.push(*unit_id);
            continue;
        }
        let result = if plain_gather {
            apply_unit_order(world, slot, entity, *unit_id, order, command)
        } else {
            crate::economy::transaction::capture_unit_order_operation(world, *unit_id, |world| {
                apply_unit_order(world, slot, entity, *unit_id, order, command)
            })
        };
        match result {
            Ok(job_id) => {
                if first_job_id.is_none() {
                    first_job_id = job_id;
                }
                outcomes.accepted.push(*unit_id)
            }
            Err(reason) => outcomes.refused.push((*unit_id, reason)),
        }
    }

    if !spread_units.is_empty() {
        let spread_unit_set: std::collections::HashSet<_> = spread_units.iter().copied().collect();
        let max_radius = spread_units
            .iter()
            .filter_map(|unit_id| entity_by_unit.get(unit_id))
            .map(|entity| unit_radius(world, *entity))
            .fold(0.0_f32, f32::max);
        let point = match command.target {
            OrderTargetSim::Point(point) => clamp_target(world, point, max_radius),
            _ => unreachable!("deferred movement has a point target"),
        };
        let tick = world.resource::<crate::world::Tick>().0;
        let can_queue = world
            .resource::<PendingMovementOrders>()
            .can_queue_after_replacing(slot.0, command.group, tick, &spread_units);
        if !can_queue {
            outcomes
                .accepted
                .retain(|unit| !spread_unit_set.contains(unit));
            outcomes.refused.extend(
                spread_units
                    .iter()
                    .copied()
                    .map(|unit| (unit, R::RateLimited)),
            );
        }
        let mut next_pending = world.resource::<PendingMovementOrders>().clone();
        next_pending.cancel_units(slot.0, &spread_units);
        let queued = can_queue
            && next_pending.queue(PendingMovementOrder {
                slot,
                accepted_tick: tick,
                seq,
                group: command.group,
                pool_id: 0,
                point,
                unit_ids: spread_units.clone(),
                order: spread_order,
                member_orders: BTreeMap::new(),
                plain: false,
            });
        if queued {
            spread_units.sort_unstable();
            for (index, unit_id) in spread_units.iter().enumerate() {
                let next_pending = (index == 0).then(|| next_pending.clone());
                let Some(entity) = entity_by_unit.get(unit_id).copied() else {
                    continue;
                };
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    *unit_id,
                    |world| {
                        if let Some(next_pending) = next_pending {
                            world.insert_resource(next_pending);
                        }
                        if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                            orders.queue.clear();
                        }
                    },
                );
            }
        }
        if !queued && can_queue {
            outcomes
                .accepted
                .retain(|unit| !spread_unit_set.contains(unit));
            outcomes
                .refused
                .extend(spread_units.into_iter().map(|unit| (unit, R::RateLimited)));
        }
    }

    let outcome = if outcomes.accepted.is_empty() {
        outcomes
            .refused
            .first()
            .map_or(Err(R::NotApplicable), |(_, reason)| Err(*reason))
    } else {
        Ok(crate::command::Applied {
            job_id: first_job_id,
        })
    };
    (outcome, outcomes)
}

fn validate_target(
    world: &World,
    command: &OrderCommand,
) -> Result<(), crate::command::RejectReason> {
    use crate::command::RejectReason as R;
    let target_kind_accepts = match (&command.target_kind, &command.target) {
        (turts_mod_api::TargetKind::Point, OrderTargetSim::Point(_))
        | (turts_mod_api::TargetKind::Unit, OrderTargetSim::Unit(_))
        | (turts_mod_api::TargetKind::PointOrUnit, OrderTargetSim::Point(_))
        | (turts_mod_api::TargetKind::PointOrUnit, OrderTargetSim::Unit(_))
        | (turts_mod_api::TargetKind::Node, OrderTargetSim::Node(_))
        | (turts_mod_api::TargetKind::Job, OrderTargetSim::Job(_))
        | (turts_mod_api::TargetKind::Content, OrderTargetSim::Content(_))
        | (turts_mod_api::TargetKind::None, OrderTargetSim::None) => true,
        (turts_mod_api::TargetKind::Content, OrderTargetSim::Invalid)
            if command.behaviour == Behaviour::Produce =>
        {
            true
        }
        _ => false,
    };
    if !target_kind_accepts {
        return Err(R::TargetInvalid);
    }
    match (&command.behaviour, &command.target) {
        (Behaviour::Move, OrderTargetSim::Point(point)) if point.is_finite() => Ok(()),
        (Behaviour::Move, OrderTargetSim::Point(_)) => Err(R::TargetNotFinite),
        (Behaviour::Move, OrderTargetSim::Unit(unit)) => {
            if world.iter_entities().any(|entity| {
                entity
                    .get::<crate::world::UnitId>()
                    .is_some_and(|id| id.0 == *unit)
            }) {
                Ok(())
            } else {
                Err(R::TargetGone)
            }
        }
        (Behaviour::Move, _) => Err(R::TargetInvalid),
        (Behaviour::Stop | Behaviour::Hold, OrderTargetSim::None) => Ok(()),
        (Behaviour::Stop | Behaviour::Hold, _) => Err(R::TargetInvalid),
        (Behaviour::Gather, OrderTargetSim::Node(node)) => {
            if world
                .get_resource::<crate::economy::SurfaceNodes>()
                .is_some_and(|nodes| nodes.0.iter().any(|record| record.id == *node))
            {
                Ok(())
            } else {
                Err(R::UnknownNode)
            }
        }
        (Behaviour::Produce, OrderTargetSim::Content(_))
        | (Behaviour::CancelProduction, OrderTargetSim::Job(_))
        | (Behaviour::Remove, OrderTargetSim::None)
        | (Behaviour::SetEnabled(_), OrderTargetSim::None) => Ok(()),
        (Behaviour::Produce, OrderTargetSim::Invalid) => Err(R::UnknownRecipe),
        (Behaviour::Attack, OrderTargetSim::Unit(unit)) => {
            if world.iter_entities().any(|entity| {
                entity
                    .get::<crate::world::UnitId>()
                    .is_some_and(|id| id.0 == *unit)
            }) {
                Ok(())
            } else {
                Err(R::TargetGone)
            }
        }
        (Behaviour::Attack, OrderTargetSim::Point(point)) if point.is_finite() => Ok(()),
        (Behaviour::Attack, OrderTargetSim::Point(_)) => Err(R::TargetNotFinite),
        _ => Err(R::TargetInvalid),
    }
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

fn unit_has_pending_move(world: &World, unit_id: u32) -> bool {
    world
        .resource::<PendingMovementOrders>()
        .pending_member_counts
        .contains_key(&unit_id)
}

fn unit_orders_empty(world: &World, unit_id: u32) -> bool {
    let entity = world.iter_entities().find(|entity| {
        entity
            .get::<crate::world::UnitId>()
            .is_some_and(|id| id.0 == unit_id)
    });
    let stored_empty = entity
        .and_then(|entity| entity.get::<UnitOrders>())
        .is_none_or(|orders| {
            orders.current.is_none() && orders.queue.is_empty() && orders.last_failure.is_none()
        });
    let gather_empty = world
        .get_resource::<crate::economy::EconomyOrders>()
        .and_then(|orders| orders.0.get(&unit_id))
        .is_none_or(|order| order.node.is_none());
    stored_empty && gather_empty
}

fn unit_radius(world: &World, entity: bevy_ecs::entity::Entity) -> f32 {
    world
        .entity(entity)
        .get::<crate::world::UnitType>()
        .and_then(|kind| world.resource::<crate::registry::UnitTypes>().get(kind.0))
        .map_or(0.5, |unit| unit.radius)
}

fn clamp_target(world: &World, target: Vec2, radius: f32) -> Vec2 {
    world
        .get_resource::<crate::world::MapBounds>()
        .map_or(target, |bounds| {
            Vec2::new(
                target.x.clamp(radius, (bounds.size.x - radius).max(radius)),
                target.y.clamp(radius, (bounds.size.y - radius).max(radius)),
            )
        })
}

fn apply_unit_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    order: Order,
    command: &OrderCommand,
) -> Result<Option<crate::JobId>, crate::command::RejectReason> {
    use crate::command::{CommandKind, RejectReason as R};
    match order.behaviour {
        Behaviour::Move => {
            let pending_replace =
                command.queue == QueueMode::Append && unit_has_pending_move(world, unit_id);
            let target = match &order.target {
                OrderTargetSim::Point(target) => {
                    clamp_target(world, *target, unit_radius(world, entity))
                }
                OrderTargetSim::Unit(target_id) => world
                    .iter_entities()
                    .find(|candidate| {
                        candidate
                            .get::<crate::world::UnitId>()
                            .is_some_and(|id| id.0 == *target_id)
                    })
                    .and_then(|candidate| {
                        candidate.get::<crate::world::Position>().map(|pos| pos.0)
                    })
                    .map(|target| clamp_target(world, target, unit_radius(world, entity)))
                    .ok_or(R::TargetGone)?,
                _ => return Err(R::TargetInvalid),
            };
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            let starts_now = if command.queue == QueueMode::Append {
                let existing_move = world
                    .entity(entity)
                    .get::<crate::world::MoveTarget>()
                    .is_some();
                let pending_move = unit_has_pending_move(world, unit_id);
                let mut entity_mut = world.entity_mut(entity);
                if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                    if orders.current.is_none() && (existing_move || pending_move) {
                        orders.queue.push_back(order.clone());
                        false
                    } else if orders.current.is_none() {
                        orders.current = Some(order.clone());
                        true
                    } else {
                        orders.queue.push_back(order.clone());
                        false
                    }
                } else {
                    world.entity_mut(entity).insert(UnitOrders {
                        current: Some(order.clone()),
                        ..Default::default()
                    });
                    true
                }
            } else {
                world.entity_mut(entity).insert(UnitOrders {
                    current: Some(order),
                    ..Default::default()
                });
                true
            };
            if starts_now {
                let mut entity_mut = world.entity_mut(entity);
                entity_mut.remove::<crate::movement::MoveProgress>();
                entity_mut.remove::<crate::movement::PlainMoveProgress>();
                entity_mut.insert(crate::world::MoveTarget(target));
                forget_plain_move_identity(world, unit_id);
            }
            if !pending_replace {
                clear_gather(world, unit_id);
            }
            Ok(None)
        }
        Behaviour::Stop => {
            clear_replaced_movement(world, slot, entity, unit_id);
            world.entity_mut(entity).insert(UnitOrders::default());
            clear_gather(world, unit_id);
            Ok(None)
        }
        Behaviour::Hold => {
            if command.queue == QueueMode::Append {
                let mut entity_mut = world.entity_mut(entity);
                let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() else {
                    return Err(R::NotQueueable);
                };
                orders.queue.push_back(order);
            } else {
                clear_replaced_movement(world, slot, entity, unit_id);
                world
                    .entity_mut(entity)
                    .remove::<crate::world::MoveTarget>();
                world.entity_mut(entity).insert(UnitOrders {
                    current: Some(order),
                    ..Default::default()
                });
            }
            clear_gather(world, unit_id);
            Ok(None)
        }
        Behaviour::Gather => {
            let OrderTargetSim::Node(node) = order.target else {
                return Err(R::TargetInvalid);
            };
            let depleted = world
                .get_resource::<crate::economy::SurfaceNodes>()
                .and_then(|nodes| nodes.0.iter().find(|record| record.id == node))
                .is_some_and(|record| record.remaining == 0);
            if command.queue == QueueMode::Append {
                let no_current = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .is_none_or(|orders| orders.current.is_none());
                let moving_from_legacy_order = world
                    .entity(entity)
                    .get::<crate::world::MoveTarget>()
                    .is_some()
                    || unit_has_pending_move(world, unit_id);
                if no_current && !moving_from_legacy_order {
                    if depleted {
                        world
                            .entity_mut(entity)
                            .remove::<crate::world::MoveTarget>();
                        clear_gather(world, unit_id);
                    } else {
                        crate::economy::gather::assign_gather(world, slot, unit_id, node)?;
                    }
                    world.entity_mut(entity).insert(UnitOrders {
                        current: Some(order),
                        ..Default::default()
                    });
                } else if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                    orders.queue.push_back(order);
                }
            } else {
                if depleted {
                    clear_replaced_movement(world, slot, entity, unit_id);
                    world
                        .entity_mut(entity)
                        .remove::<crate::world::MoveTarget>();
                    clear_gather(world, unit_id);
                } else {
                    crate::economy::gather::assign_gather(world, slot, unit_id, node)?;
                    clear_replaced_movement(world, slot, entity, unit_id);
                }
                world.entity_mut(entity).insert(UnitOrders {
                    current: Some(order),
                    ..Default::default()
                });
            }
            Ok(None)
        }
        Behaviour::Produce => {
            let OrderTargetSim::Content(recipe_id) = order.target else {
                return Err(R::TargetInvalid);
            };
            let job_id = crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::Produce {
                    producer_id: unit_id,
                    recipe_id,
                },
            )
            .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            Ok(job_id)
        }
        Behaviour::CancelProduction => {
            let OrderTargetSim::Job(job_id) = order.target else {
                return Err(R::TargetInvalid);
            };
            let job_id = crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::CancelProduction {
                    producer_id: unit_id,
                    job_id,
                },
            )
            .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            Ok(job_id)
        }
        Behaviour::Remove => {
            let job_id =
                crate::command::apply_legacy_unit(world, slot, CommandKind::Remove { unit_id })
                    .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                world
                    .resource_mut::<PendingMovementOrders>()
                    .cancel_units(slot.0, &[unit_id]);
            }
            Ok(job_id)
        }
        Behaviour::SetEnabled(enabled) => {
            let job_id = crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::SetEnabled { unit_id, enabled },
            )
            .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            Ok(job_id)
        }
        Behaviour::Attack => Err(R::NotApplicable),
    }
}

fn clear_replaced_movement(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
) {
    world
        .resource_mut::<PendingMovementOrders>()
        .cancel_units(slot.0, &[unit_id]);
    let mut movement = world.entity_mut(entity);
    movement.remove::<MoveTarget>();
    movement.remove::<crate::movement::MoveProgress>();
    movement.remove::<crate::movement::PlainMoveProgress>();
    if let Some(mut identities) = world.get_resource_mut::<PlainMoveOrderIdentities>() {
        identities.0.remove(&unit_id);
    }
}

fn remember_plain_move_identity(world: &mut World, unit_id: u32, order: Order) {
    world.init_resource::<PlainMoveOrderIdentities>();
    world
        .resource_mut::<PlainMoveOrderIdentities>()
        .0
        .insert(unit_id, order);
}

fn forget_plain_move_identity(world: &mut World, unit_id: u32) {
    if let Some(mut identities) = world.get_resource_mut::<PlainMoveOrderIdentities>() {
        identities.0.remove(&unit_id);
    }
}

fn clear_gather(world: &mut World, unit_id: u32) {
    if let Some(mut orders) = world.get_resource_mut::<crate::economy::EconomyOrders>()
        && let Some(order) = orders.0.get_mut(&unit_id)
    {
        order.node = None;
        order.deposit = None;
        order.receiver = None;
        order.stage = crate::economy::gather::GatherStage::Idle;
        order.carry = crate::economy::gather::RateCarry::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{Footprint, GathererDef, PackageId, ProducerDef, UnitDef, UnitRegistry};

    fn gatherer_app() -> (bevy_app::App, bevy_ecs::entity::Entity) {
        let mut app = crate::headless_app(20);
        let id = ContentId::parse("test:worker").unwrap();
        let kind = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &PackageId("test".into()),
                UnitDef {
                    id: "worker".into(),
                    name: "Worker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: Some(GathererDef {
                        items: vec![],
                        gather_per_second: 1,
                        unload_per_second: 1,
                        interaction_range: 1.0,
                    }),
                    drop_off: None,
                    producer: Some(ProducerDef {
                        recipes: vec![],
                        queue_capacity: 1,
                        spawn_offsets: vec![],
                    }),
                    footprint: Footprint::Circle { radius: 0.5 },
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            )
            .unwrap();
        let mut catalog = turts_mod_api::ContentCatalog {
            resources: Default::default(),
            units: Default::default(),
            nodes: Default::default(),
            production: Default::default(),
            recipes: Default::default(),
            technologies: Default::default(),
            orders: Default::default(),
            order_queue_depths: Default::default(),
            source_files: Default::default(),
        };
        catalog.units.insert(
            id.clone(),
            turts_mod_api::ResolvedUnit {
                id: id.clone(),
                name: "Worker".into(),
                footprint: Footprint::Circle { radius: 0.5 },
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: Some(turts_mod_api::ResolvedGatherer {
                    items: vec![],
                    gather_per_second: 1,
                    unload_per_second: 1,
                    interaction_range: 1.0,
                }),
                drop_off: None,
                producer: Some(turts_mod_api::ResolvedProducer {
                    recipes: vec![],
                    queue_capacity: 1,
                    spawn_offsets: vec![],
                }),
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        );
        let node_id = ContentId::parse("test:ore").unwrap();
        catalog.nodes.insert(
            node_id.clone(),
            turts_mod_api::ResolvedNode {
                id: node_id,
                name: "Ore".into(),
                item: ContentId::parse("test:iron").unwrap(),
                radius: 1.0,
            },
        );
        app.world_mut()
            .insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        let entity = app
            .world_mut()
            .spawn((
                crate::UnitId(1),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(8.0, 8.0)),
                crate::world::MoveTarget(Vec2::new(30.0, 30.0)),
                UnitOrders::default(),
            ))
            .id();
        app.world_mut()
            .insert_resource(crate::economy::SurfaceNodes(vec![
                crate::economy::NodeRecord {
                    id: NodeId(1),
                    content_id: ContentId::parse("test:ore").unwrap(),
                    position: crate::Position(Vec2::new(10.0, 8.0)),
                    remaining: 1,
                },
            ]));
        (app, entity)
    }

    #[test]
    fn one_pending_slot_refuses_a_two_member_queued_group_atomically_before_movement() {
        let (mut app, first) = gatherer_app();
        app.world_mut().insert_resource(crate::world::MapBounds {
            size: Vec2::new(64.0, 64.0),
        });
        let mobile_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        let second = app
            .world_mut()
            .spawn((
                crate::UnitId(2),
                crate::UnitType(mobile_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(9.0, 8.0)),
                MoveTarget(Vec2::new(30.0, 30.0)),
                UnitOrders::default(),
            ))
            .id();
        let move_order = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
            group: 77,
        };
        app.world_mut().entity_mut(first).insert(UnitOrders {
            current: Some(move_order.clone()),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:hold").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 0,
            }]),
            last_failure: None,
        });
        app.world_mut().entity_mut(second).insert(UnitOrders {
            current: Some(Order {
                order: ContentId::parse("test:move-alt").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(31.0, 30.0)),
                group: 77,
            }),
            ..Default::default()
        });
        app.world_mut()
            .entity_mut(second)
            .insert(MoveTarget(Vec2::new(31.0, 30.0)));

        let pending_ids = (1000..17_383).collect::<Vec<_>>();
        let mut full_pending = PendingMovementOrders::default();
        assert!(full_pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1000,
            group: 3,
            pool_id: 0,
            point: Vec2::new(40.0, 40.0),
            unit_ids: pending_ids,
            order: None,
            member_orders: BTreeMap::new(),
            plain: false,
        }));
        assert_eq!(full_pending.pending_units.get(&0), Some(&16_383));
        app.world_mut().insert_resource(full_pending.clone());
        app.world_mut().resource_mut::<crate::AuditTrail>().enable();

        queue_unallocated_move_orders(app.world_mut());

        assert_eq!(
            app.world().resource::<PendingMovementOrders>(),
            &full_pending,
            "failed group admission leaves every pre-existing pending envelope unchanged"
        );
        for entity in [first, second] {
            assert!(
                app.world().entity(entity).get::<MoveTarget>().is_none(),
                "unit {} must not retain an unallocated target after group refusal",
                app.world().entity(entity).get::<crate::UnitId>().unwrap().0
            );
            assert!(
                app.world()
                    .entity(entity)
                    .get::<crate::movement::MoveProgress>()
                    .is_none()
            );
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .last_failure
                    .as_ref()
                    .map(|failure| failure.reason),
                Some(crate::RejectReason::RateLimited)
            );
        }
        assert_eq!(
            app.world()
                .entity(first)
                .get::<UnitOrders>()
                .unwrap()
                .current
                .as_ref()
                .map(|order| order.behaviour),
            Some(Behaviour::Hold),
            "normal same-phase queued continuation still starts after the atomic refusal"
        );
        assert_eq!(
            app.world()
                .entity(first)
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .as_ref()
                .unwrap()
                .order,
            ContentId::parse("test:move").unwrap()
        );
        assert_eq!(
            app.world()
                .entity(second)
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .as_ref()
                .unwrap()
                .order,
            ContentId::parse("test:move-alt").unwrap()
        );
        let operations = &app.world().resource::<crate::AuditTrail>().operations;
        assert_eq!(operations.len(), 2);
        assert_eq!(operations[0].before.pending_movement_orders, full_pending);
        assert_eq!(operations[0].after.pending_movement_orders, full_pending);
        assert_eq!(operations[1].before, operations[0].after);
    }

    #[test]
    fn merged_group_spread_keeps_each_member_order_identity() {
        let (mut app, first) = gatherer_app();
        let mobile_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        app.world_mut().entity_mut(first).remove::<UnitOrders>();
        let second = app
            .world_mut()
            .spawn((
                crate::UnitId(2),
                crate::UnitType(mobile_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(12.0, 8.0)),
                UnitOrders::default(),
            ))
            .id();
        let first_order = Order {
            order: ContentId::parse("test:move-a").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 9,
        };
        let second_order = Order {
            order: ContentId::parse("test:move-b").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(20.0, 20.0)),
            group: 9,
        };
        for (unit_id, entity, order) in [
            (1, first, first_order.clone()),
            (2, second, second_order.clone()),
        ] {
            app.world_mut().entity_mut(entity).insert(UnitOrders {
                current: Some(order.clone()),
                ..Default::default()
            });
            assert!(queue_movement_envelope(
                app.world_mut(),
                MovementEnvelopeRequest {
                    slot: SlotId(0),
                    seq: unit_id,
                    group: 9,
                    point: match order.target {
                        OrderTargetSim::Point(point) => point,
                        _ => unreachable!(),
                    },
                    unit_ids: vec![unit_id],
                    order: Some(order.clone()),
                    member_orders: BTreeMap::from([(unit_id, order)]),
                    plain: false,
                },
            ));
        }
        let pending = app.world().resource::<PendingMovementOrders>();
        let envelope = pending.queues.get(&0).unwrap().front().unwrap();
        assert_eq!(envelope.unit_ids, vec![1, 2]);
        assert_eq!(envelope.order_for_unit(1), Some(&first_order));
        assert_eq!(envelope.order_for_unit(2), Some(&second_order));
        assert_eq!(pending.pending_units.get(&0), Some(&2));

        let latest_order = Order {
            order: ContentId::parse("test:move-b-latest").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(25.0, 25.0)),
            group: 9,
        };
        assert!(queue_movement_envelope(
            app.world_mut(),
            MovementEnvelopeRequest {
                slot: SlotId(0),
                seq: 2,
                group: 9,
                point: Vec2::new(25.0, 25.0),
                unit_ids: vec![2],
                order: Some(latest_order.clone()),
                member_orders: BTreeMap::from([(2, latest_order.clone())]),
                plain: false,
            },
        ));
        let pending = app.world().resource::<PendingMovementOrders>();
        let envelope = pending.queues.get(&0).unwrap().front().unwrap();
        assert_eq!(
            envelope.point,
            Vec2::new(40.0, 40.0),
            "K4 retains first-P anchor"
        );
        assert_eq!(envelope.order_for_unit(1), Some(&first_order));
        assert_eq!(envelope.order_for_unit(2), Some(&latest_order));
        assert_eq!(pending.pending_units.get(&0), Some(&2));
        let mut canceled_copy = pending.clone();
        canceled_copy.cancel_units(0, &[1]);
        let surviving = canceled_copy.queues.get(&0).unwrap().front().unwrap();
        assert_eq!(surviving.unit_ids, vec![2]);
        assert!(surviving.order_for_unit(1).is_none());
        assert_eq!(surviving.order_for_unit(2), Some(&latest_order));

        dispatch_pending_movement(app.world_mut());
        assert_eq!(
            app.world()
                .entity(first)
                .get::<UnitOrders>()
                .unwrap()
                .current,
            Some(first_order)
        );
        assert_eq!(
            app.world()
                .entity(second)
                .get::<UnitOrders>()
                .unwrap()
                .current,
            Some(latest_order)
        );
        assert_eq!(
            app.world()
                .resource::<crate::movement::MovementPools>()
                .0
                .len(),
            1,
            "distinct accepted order IDs still use the K4 shared movement envelope"
        );
    }

    #[test]
    fn retained_old_move_is_not_reenqueued_behind_pending_replacement() {
        let (mut app, entity) = gatherer_app();
        let old_order = Order {
            order: ContentId::parse("test:move-old").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
            group: 8,
        };
        let replacement = Order {
            order: ContentId::parse("test:move-new").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 9,
        };
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(old_order),
            ..Default::default()
        });
        let mut pending = PendingMovementOrders::default();
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1,
            group: 9,
            pool_id: 0,
            point: Vec2::new(40.0, 40.0),
            unit_ids: vec![1],
            order: Some(replacement.clone()),
            member_orders: BTreeMap::from([(1, replacement.clone())]),
            plain: false,
        }));
        app.world_mut().insert_resource(pending.clone());

        queue_unallocated_move_orders(app.world_mut());

        assert_eq!(app.world().resource::<PendingMovementOrders>(), &pending);
        assert_eq!(
            app.world()
                .resource::<PendingMovementOrders>()
                .queues
                .get(&0)
                .unwrap()
                .len(),
            1,
            "the retained Move A cannot be appended behind replacement B"
        );
        assert_eq!(
            app.world()
                .resource::<PendingMovementOrders>()
                .queues
                .get(&0)
                .unwrap()[0]
                .order_for_unit(1),
            Some(&replacement)
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current
                .as_ref()
                .unwrap()
                .order,
            ContentId::parse("test:move-old").unwrap(),
            "the old current order remains until B dispatches"
        );
    }

    #[test]
    fn production_dispatch_reuses_the_spatial_index_for_many_small_envelopes() {
        let (mut app, first) = gatherer_app();
        app.world_mut().insert_resource(crate::world::MapBounds {
            size: Vec2::new(128.0, 128.0),
        });
        let mobile_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        let post_type = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &turts_mod_api::PackageId("test".into()),
                UnitDef {
                    id: "post".into(),
                    name: "Post".into(),
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    footprint: Footprint::Circle { radius: 0.5 },
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            )
            .unwrap();
        for index in 0..512_u32 {
            let column = index % 32;
            let row = index / 32;
            app.world_mut().spawn((
                crate::UnitId(1_000 + index),
                crate::UnitType(post_type),
                crate::Owner(SlotId(1)),
                crate::Position(Vec2::new(
                    30.0 + column as f32 * 1.5,
                    2.0 + row as f32 * 1.5,
                )),
            ));
        }
        for index in 1..128_u32 {
            let column = index % 16;
            let row = index / 16;
            app.world_mut().spawn((
                crate::UnitId(index + 1),
                crate::UnitType(mobile_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(
                    10.0 + column as f32 * 1.5,
                    35.0 + row as f32 * 1.5,
                )),
                UnitOrders::default(),
            ));
        }
        let order = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(90.0, 90.0)),
            group: 0,
        };
        for unit_id in 1..=128_u32 {
            assert!(queue_movement_envelope(
                app.world_mut(),
                MovementEnvelopeRequest {
                    slot: SlotId(0),
                    seq: unit_id,
                    group: 0,
                    point: Vec2::new(90.0, 90.0),
                    unit_ids: vec![unit_id],
                    order: Some(order.clone()),
                    member_orders: BTreeMap::new(),
                    plain: false,
                },
            ));
        }

        dispatch_pending_movement(app.world_mut());
        assert_eq!(
            app.world()
                .resource::<PendingMovementOrders>()
                .pending_units
                .get(&0),
            Some(&0)
        );
        let assigned: std::collections::BTreeSet<_> = app
            .world()
            .iter_entities()
            .filter_map(|entity| {
                let id = entity.get::<crate::UnitId>()?.0;
                (1..=128)
                    .contains(&id)
                    .then(|| {
                        entity
                            .get::<MoveTarget>()
                            .map(|target| (target.0.x.to_bits(), target.0.y.to_bits()))
                    })
                    .flatten()
            })
            .collect();
        assert_eq!(
            assigned.len(),
            128,
            "every singleton receives a unique slot"
        );

        dispatch_pending_movement(app.world_mut());
        assert_eq!(
            app.world().resource::<SpreadIndexCache>().context_builds,
            1,
            "the production batch and second dispatch pass reuse one world index"
        );
    }

    #[test]
    fn second_dispatch_refreshes_reservations_after_rate_limited_hold_continuation() {
        let (mut app, held) = gatherer_app();
        app.world_mut().insert_resource(crate::world::MapBounds {
            size: Vec2::new(64.0, 64.0),
        });
        let mobile_type = app.world().entity(held).get::<crate::UnitType>().unwrap().0;
        app.world_mut().entity_mut(held).insert((
            crate::Position(Vec2::new(20.0, 20.0)),
            MoveTarget(Vec2::new(40.0, 40.0)),
            UnitOrders {
                current: Some(Order {
                    order: ContentId::parse("test:move-refused").unwrap(),
                    behaviour: Behaviour::Move,
                    target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                    group: 77,
                }),
                queue: VecDeque::from([Order {
                    order: ContentId::parse("test:hold-next").unwrap(),
                    behaviour: Behaviour::Hold,
                    target: OrderTargetSim::None,
                    group: 0,
                }]),
                last_failure: None,
            },
        ));
        let old_target = Vec2::new(40.0, 40.0);
        let hold_position = Vec2::new(20.0, 20.0);
        let second = app
            .world_mut()
            .spawn((
                crate::UnitId(2),
                crate::UnitType(mobile_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(8.0, 8.0)),
            ))
            .id();
        let third = app
            .world_mut()
            .spawn((
                crate::UnitId(3),
                crate::UnitType(mobile_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(10.0, 8.0)),
            ))
            .id();

        // Build the actual production index before the first dispatch pass.
        dispatch_pending_movement(app.world_mut());
        let stale_fill = (1000..17_382).collect::<Vec<_>>();
        let mut pending = PendingMovementOrders::default();
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1000,
            group: 99,
            pool_id: 0,
            point: Vec2::new(50.0, 50.0),
            unit_ids: stale_fill,
            order: None,
            member_orders: BTreeMap::new(),
            plain: false,
        }));
        for (unit_id, point, group) in [(2, old_target, 1), (3, hold_position, 2)] {
            assert!(pending.queue(PendingMovementOrder {
                slot: SlotId(0),
                accepted_tick: 0,
                seq: unit_id,
                group,
                pool_id: 0,
                point,
                unit_ids: vec![unit_id],
                order: None,
                member_orders: BTreeMap::new(),
                plain: false,
            }));
        }
        assert_eq!(pending.pending_units.get(&0), Some(&16_384));
        app.world_mut().insert_resource(pending);

        // The refusal replaces the old target reservation with the current
        // body reservation because the queued Hold starts in this same phase.
        queue_unallocated_move_orders(app.world_mut());
        assert_eq!(
            app.world()
                .entity(held)
                .get::<UnitOrders>()
                .unwrap()
                .current
                .as_ref()
                .map(|order| order.behaviour),
            Some(Behaviour::Hold)
        );
        assert!(app.world().entity(held).get::<MoveTarget>().is_none());

        // The second production dispatch reuses the refreshed index: the old
        // destination is available, while the new Hold position remains taken.
        dispatch_pending_movement(app.world_mut());
        let old_target_assignee = app
            .world()
            .entity(second)
            .get::<MoveTarget>()
            .expect("the stale refused target must no longer reserve this point");
        assert_eq!(old_target_assignee.0, old_target);
        let hold_target_assignee = app
            .world()
            .entity(third)
            .get::<MoveTarget>()
            .expect("the other envelope must still be dispatched");
        assert!(
            hold_target_assignee.0.distance(hold_position) >= 1.1 - 1.0e-5,
            "the new Hold position remains reserved, got {:?}",
            hold_target_assignee.0
        );
        assert_eq!(app.world().resource::<SpreadIndexCache>().context_builds, 1);
    }

    #[test]
    fn pristine_idle_ticks_keep_legacy_audit_state_omitting_movement_scheduler_fields() {
        let mut app = crate::headless_app(20);
        for _ in 0..10 {
            app.update();
        }
        assert!(app.world().resource::<crate::world::Tick>().0 >= 8);
        assert!(app.world().resource::<MovementDispatchState>().is_default());
        assert!(app.world().resource::<PendingMovementOrders>().is_empty());
        let audit = crate::audit_state(app.world_mut());
        let serialized = serde_json::to_value(audit).unwrap();
        assert!(serialized.get("pending_movement_orders").is_none());
        assert!(serialized.get("movement_dispatch").is_none());
    }

    #[test]
    fn first_cohort_turn_uses_global_tick_then_idle_ticks_keep_rotation() {
        let (mut app, first) = gatherer_app();
        for _ in 0..10 {
            app.update();
        }
        assert!(app.world().resource::<MovementDispatchState>().is_default());
        let first_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        for id in 2..=8_u32 {
            app.world_mut().spawn((
                crate::UnitId(id),
                crate::UnitType(first_type),
                crate::Owner(SlotId((id - 1) as u8)),
                crate::Position(Vec2::new(4.0 + id as f32, 4.0)),
                UnitOrders::default(),
            ));
        }
        let tick = app.world().resource::<crate::world::Tick>().0;
        let order = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 0,
        };
        for slot in 0..8_u8 {
            assert!(queue_movement_envelope(
                app.world_mut(),
                MovementEnvelopeRequest {
                    slot: SlotId(slot),
                    seq: u32::from(slot),
                    group: 0,
                    point: Vec2::new(40.0, 40.0),
                    unit_ids: vec![u32::from(slot) + 1],
                    order: Some(order.clone()),
                    member_orders: BTreeMap::new(),
                    plain: false,
                },
            ));
        }
        dispatch_pending_movement(app.world_mut());
        let first_turn = ((tick.saturating_sub(1)) % 8) as u8;
        assert_eq!(
            app.world().resource::<MovementDispatchState>().start_slot,
            first_turn
        );
        assert_eq!(
            app.world().resource::<PendingMovementOrders>().next_slot,
            (first_turn + 1) % 8
        );

        app.update();
        app.update();
        let next_tick = app.world().resource::<crate::world::Tick>().0;
        let expected_turn = ((tick.saturating_sub(1) + 2) % 8) as u8;
        assert_eq!(
            app.world().resource::<MovementDispatchState>().start_slot,
            expected_turn,
            "the stored first turn advances across two idle ticks"
        );
        assert_eq!(
            app.world().resource::<MovementDispatchState>().tick,
            Some(next_tick)
        );
    }

    #[test]
    fn remove_capability_requires_by_command_before_filter_and_target_checks() {
        let (mut app, entity) = gatherer_app();
        let worker_id = ContentId::parse("test:worker").unwrap();
        let catalog = &mut app
            .world_mut()
            .resource_mut::<crate::bootstrap::InstalledCatalog>()
            .0;
        catalog.units.get_mut(&worker_id).unwrap().removal = Some(turts_mod_api::RemovalDef {
            by_command: false,
            contents: turts_mod_api::ContentsPolicy::Destroy,
        });

        let command = OrderCommand {
            order: ContentId::parse("test:remove").unwrap(),
            behaviour: Behaviour::Remove,
            target_kind: turts_mod_api::TargetKind::None,
            policy: None,
            units: vec![app.world().entity(entity).get::<crate::UnitId>().unwrap().0],
            target: OrderTargetSim::Point(Vec2::new(8.0, 8.0)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec!["other:unit".into()],
            queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
        };
        let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 1, &command);
        assert_eq!(
            outcomes.refused,
            vec![(1, crate::command::RejectReason::NotRemovable)],
            "a disabled removal capability wins over custom filter and target failures"
        );
    }

    #[test]
    fn append_gather_waits_behind_a_legacy_plain_move() {
        let (mut app, entity) = gatherer_app();
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:gather").unwrap(),
                behaviour: Behaviour::Gather,
                target_kind: turts_mod_api::TargetKind::Node,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Node(NodeId(1)),
                queue: QueueMode::Append,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1]);
        let entity_ref = app.world().entity(entity);
        assert_eq!(
            entity_ref.get::<crate::world::MoveTarget>().unwrap().0,
            Vec2::new(30.0, 30.0)
        );
        let orders = entity_ref.get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert_eq!(orders.queue.len(), 1);
        assert_eq!(orders.queue[0].behaviour, Behaviour::Gather);
    }

    #[test]
    fn deferred_replace_refusal_preserves_current_and_queued_orders() {
        let (mut app, entity) = gatherer_app();
        let current = Order {
            order: ContentId::parse("test:gather").unwrap(),
            behaviour: Behaviour::Gather,
            target: OrderTargetSim::Node(NodeId(1)),
            group: 4,
        };
        let queued = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(22.0, 22.0)),
            group: 5,
        };
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(current.clone()),
            queue: VecDeque::from([queued.clone()]),
            ..Default::default()
        });
        app.world_mut()
            .resource_mut::<PendingMovementOrders>()
            .pending_units
            .insert(0, 16_384);
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:move").unwrap(),
                behaviour: Behaviour::Move,
                target_kind: turts_mod_api::TargetKind::Point,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                queue: QueueMode::Replace,
                group: 1,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, Vec::<u32>::new());
        assert_eq!(
            result.units.refused,
            vec![(1, crate::RejectReason::RateLimited)]
        );
        let after = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert_eq!(after.current, Some(current));
        assert_eq!(after.queue, VecDeque::from([queued]));
        assert!(
            app.world()
                .resource::<PendingMovementOrders>()
                .queues
                .is_empty()
        );
    }

    #[test]
    fn pending_move_activity_overrides_prior_blocked_failure_without_erasing_it() {
        let (mut app, entity) = gatherer_app();
        let move_order = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 7,
        };
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(move_order.clone()),
            last_failure: Some(crate::OrderFailure {
                order: move_order.order.clone(),
                reason: crate::RejectReason::Blocked,
                tick: 0,
            }),
            ..Default::default()
        });
        assert!(queue_movement_envelope(
            app.world_mut(),
            MovementEnvelopeRequest {
                slot: SlotId(0),
                seq: 2,
                group: 7,
                point: Vec2::new(40.0, 40.0),
                unit_ids: vec![1],
                order: Some(move_order),
                member_orders: BTreeMap::new(),
                plain: false,
            },
        ));
        let view = crate::economy::views::world_view(app.world_mut());
        assert_eq!(
            view.unit_orders[0].activity,
            crate::economy::views::UnitOrderActivityView::Pending
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .as_ref()
                .unwrap()
                .reason,
            crate::RejectReason::Blocked,
            "the prior failure remains stored while the replacement is pending"
        );
    }

    #[test]
    fn appended_move_replaces_blocked_export_activity_but_keeps_failure_history() {
        let (mut app, entity) = gatherer_app();
        app.world_mut().entity_mut(entity).remove::<MoveTarget>();
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            last_failure: Some(crate::OrderFailure {
                order: ContentId::parse("test:move").unwrap(),
                reason: crate::RejectReason::Blocked,
                tick: 0,
            }),
            ..Default::default()
        });
        let command = crate::Command {
            slot: SlotId(0),
            seq: 3,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:move").unwrap(),
                behaviour: Behaviour::Move,
                target_kind: turts_mod_api::TargetKind::Point,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                queue: QueueMode::Append,
                group: 8,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1]);
        let view = crate::economy::views::world_view(app.world_mut());
        assert_eq!(
            view.unit_orders[0].activity,
            crate::economy::views::UnitOrderActivityView::Moving
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .as_ref()
                .unwrap()
                .reason,
            crate::RejectReason::Blocked
        );
    }

    #[test]
    fn fresh_blocked_progress_precedes_active_move_in_export() {
        let (mut app, entity) = gatherer_app();
        let order = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 7,
        };
        app.world_mut().entity_mut(entity).insert((
            MoveTarget(Vec2::new(40.0, 40.0)),
            crate::movement::MoveProgress {
                pool_id: 1,
                slot: Vec2::new(40.0, 40.0),
                activity: crate::movement::MovementActivity::Blocked,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 20,
                best_distance: 1.0,
                claim_count: 3,
            },
        ));
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(order),
            ..Default::default()
        });
        let view = crate::economy::views::world_view(app.world_mut());
        assert_eq!(
            view.unit_orders[0].activity,
            crate::economy::views::UnitOrderActivityView::Blocked
        );
    }

    #[test]
    fn hold_replace_cancels_a_stale_pending_move_before_dispatch() {
        let (mut app, entity) = gatherer_app();
        let stale = Vec2::new(40.0, 40.0);
        assert!(queue_movement_envelope(
            app.world_mut(),
            MovementEnvelopeRequest {
                slot: SlotId(0),
                seq: 1,
                group: 9,
                point: stale,
                unit_ids: vec![1],
                order: Some(Order {
                    order: ContentId::parse("test:move").unwrap(),
                    behaviour: Behaviour::Move,
                    target: OrderTargetSim::Point(stale),
                    group: 9,
                }),
                member_orders: BTreeMap::new(),
                plain: false,
            },
        ));
        let command = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:hold").unwrap(),
                behaviour: Behaviour::Hold,
                target_kind: turts_mod_api::TargetKind::None,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::None,
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1]);
        assert_eq!(
            app.world()
                .resource::<PendingMovementOrders>()
                .pending_units
                .get(&0),
            Some(&0),
            "Hold replacement removes the old request from pending membership"
        );
        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert_eq!(orders.current.as_ref().unwrap().behaviour, Behaviour::Hold);
        assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
        dispatch_pending_movement(app.world_mut());
        assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
    }

    #[test]
    fn gather_replace_cancels_a_stale_pending_move_before_dispatch() {
        let (mut app, entity) = gatherer_app();
        app.world_mut()
            .resource_mut::<crate::economy::SurfaceNodes>()
            .0[0]
            .remaining = 0;
        let stale = Vec2::new(40.0, 40.0);
        assert!(queue_movement_envelope(
            app.world_mut(),
            MovementEnvelopeRequest {
                slot: SlotId(0),
                seq: 1,
                group: 9,
                point: stale,
                unit_ids: vec![1],
                order: Some(Order {
                    order: ContentId::parse("test:move").unwrap(),
                    behaviour: Behaviour::Move,
                    target: OrderTargetSim::Point(stale),
                    group: 9,
                }),
                member_orders: BTreeMap::new(),
                plain: false,
            },
        ));
        let command = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:gather").unwrap(),
                behaviour: Behaviour::Gather,
                target_kind: turts_mod_api::TargetKind::Node,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Node(NodeId(1)),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1]);
        assert_eq!(
            app.world()
                .resource::<PendingMovementOrders>()
                .pending_units
                .get(&0),
            Some(&0),
            "Gather replacement removes the old request from pending membership"
        );
        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert_eq!(
            orders.current.as_ref().unwrap().behaviour,
            Behaviour::Gather
        );
        assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
        dispatch_pending_movement(app.world_mut());
        assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
    }

    #[test]
    fn append_hold_refusal_preserves_active_gather_state() {
        let (mut app, entity) = gatherer_app();
        let active_gather = Order {
            order: ContentId::parse("test:gather").unwrap(),
            behaviour: Behaviour::Gather,
            target: OrderTargetSim::Node(NodeId(1)),
            group: 0,
        };
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(active_gather),
            ..Default::default()
        });
        app.world_mut()
            .insert_resource(crate::economy::EconomyOrders(
                [(
                    1,
                    crate::economy::GatherOrder {
                        node: Some(NodeId(1)),
                        deposit: Some(NodeId(1)),
                        receiver: None,
                        stage: crate::economy::gather::GatherStage::Gathering,
                        carry: crate::economy::gather::RateCarry::default(),
                    },
                )]
                .into(),
            ));
        let before = app
            .world()
            .resource::<crate::economy::EconomyOrders>()
            .clone();
        let command = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:hold").unwrap(),
                behaviour: Behaviour::Hold,
                target_kind: turts_mod_api::TargetKind::None,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::None,
                queue: QueueMode::Append,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(
            result.units.refused,
            vec![(1, crate::command::RejectReason::NotQueueable)]
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current
                .as_ref()
                .unwrap()
                .behaviour,
            Behaviour::Gather
        );
        assert_eq!(
            *app.world().resource::<crate::economy::EconomyOrders>(),
            before
        );
    }

    #[test]
    fn rejected_incompatible_gather_preserves_current_order_and_queue() {
        let (mut app, entity) = gatherer_app();
        let before_order = Order {
            order: ContentId::parse("test:hold").unwrap(),
            behaviour: Behaviour::Hold,
            target: OrderTargetSim::None,
            group: 0,
        };
        let queued = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 0,
        };
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(before_order),
            queue: VecDeque::from([queued]),
            last_failure: None,
        });
        let before = app
            .world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .clone();
        app.world_mut().resource_mut::<crate::AuditTrail>().enable();
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:gather").unwrap(),
                behaviour: Behaviour::Gather,
                target_kind: turts_mod_api::TargetKind::Node,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Node(NodeId(1)),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(
            result.units.refused,
            vec![(1, crate::RejectReason::IncompatibleItem)]
        );
        assert_eq!(
            app.world().entity(entity).get::<UnitOrders>().unwrap(),
            &before
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .unwrap()
                .0,
            Vec2::new(30.0, 30.0)
        );
        let trail = app.world().resource::<crate::AuditTrail>();
        assert_eq!(trail.commands.len(), 1);
        assert_eq!(trail.operations.len(), 1);
        assert_eq!(trail.operations[0].before, trail.operations[0].after);
    }

    #[test]
    fn gather_on_an_existing_empty_node_is_accepted_then_fails_as_depleted() {
        let (mut app, entity) = gatherer_app();
        app.world_mut()
            .resource_mut::<crate::economy::SurfaceNodes>()
            .0[0]
            .remaining = 0;
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:gather").unwrap(),
                behaviour: Behaviour::Gather,
                target_kind: turts_mod_api::TargetKind::Node,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Node(NodeId(1)),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1]);
        assert!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current
                .is_some()
        );
        app.world_mut().resource_mut::<crate::Tick>().0 = 8;
        run_orders(app.world_mut());
        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert_eq!(
            orders.last_failure.as_ref().unwrap().reason,
            crate::RejectReason::NodeDepleted
        );
        assert_eq!(orders.last_failure.as_ref().unwrap().tick, 8);
    }

    #[test]
    fn immediate_cancel_ignores_unit_order_queue_and_preserves_it_on_failure() {
        let (mut app, entity) = gatherer_app();
        let existing = UnitOrders {
            current: Some(Order {
                order: ContentId::parse("test:hold").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 0,
            }),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:move").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                group: 0,
            }]),
            last_failure: None,
        };
        app.world_mut().entity_mut(entity).insert(existing.clone());
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:cancel").unwrap(),
                behaviour: Behaviour::CancelProduction,
                target_kind: turts_mod_api::TargetKind::Job,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Job(crate::JobId(999)),
                queue: QueueMode::Append,
                group: 0,
                allowed_units: vec![],
                queue_depth: 1,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(
            result.units.refused,
            vec![(1, crate::RejectReason::UnknownJob)]
        );
        assert_eq!(
            app.world().entity(entity).get::<UnitOrders>().unwrap(),
            &existing
        );
    }

    #[test]
    fn set_enabled_keeps_all_legacy_toggleable_capability_families() {
        let (app, _) = gatherer_app();
        let id = ContentId::parse("test:worker").unwrap();
        let base = app
            .world()
            .resource::<crate::bootstrap::InstalledCatalog>()
            .0
            .units[&id]
            .clone();
        let mut variants = Vec::new();
        let mut unit = base.clone();
        unit.extractor = Some(turts_mod_api::ExtractorDef {
            per_second: 1,
            interaction_range: 1.0,
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.inserter = Some(turts_mod_api::InserterDef {
            reach: 1.0,
            swing_ms: 1,
            return_ms: 1,
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.machine = Some(turts_mod_api::ResolvedMachine {
            recipe: ContentId::parse("test:recipe").unwrap(),
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.research = Some(turts_mod_api::ResolvedResearchStation {
            technologies: vec![],
            queue_capacity: 1,
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.producer = Some(turts_mod_api::ResolvedProducer {
            recipes: vec![],
            queue_capacity: 1,
            spawn_offsets: vec![],
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.weapon = Some(turts_mod_api::ResolvedWeapon {
            range: 1.0,
            cooldown_ms: 1,
            damage: 1,
            ammo: None,
            targets: turts_mod_api::TargetFilter {
                slots: turts_mod_api::SlotRelation::Any,
            },
        });
        variants.push(unit);
        assert!(variants.iter().all(unit_toggleable));
        let mut conveyor_only = base;
        conveyor_only.producer = None;
        conveyor_only.gatherer = None;
        conveyor_only.conveyor = Some(turts_mod_api::ResolvedConveyor {
            slots_per_piece: 1,
            slots_per_second: 1,
            accepts: vec![],
        });
        assert!(!unit_toggleable(&conveyor_only));
    }

    #[test]
    fn immediate_produce_returns_the_first_job_in_unit_id_order() {
        let (mut app, _) = gatherer_app();
        let producer = ContentId::parse("test:worker").unwrap();
        let recipe = ContentId::parse("test:recipe").unwrap();
        let mut catalog = app
            .world()
            .resource::<crate::bootstrap::InstalledCatalog>()
            .0
            .clone();
        catalog
            .units
            .get_mut(&producer)
            .unwrap()
            .producer
            .as_mut()
            .unwrap()
            .recipes = vec![recipe.clone()];
        catalog.production.insert(
            recipe.clone(),
            turts_mod_api::ResolvedProduction {
                id: recipe.clone(),
                name: "Recipe".into(),
                output_unit: producer.clone(),
                duration_ms: 1000,
                costs: vec![],
                payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
                cancel_refunds: turts_mod_api::CancelRefunds {
                    queued: turts_mod_api::RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    running: turts_mod_api::RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    output_blocked: turts_mod_api::RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                },
            },
        );
        app.world_mut()
            .insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        app.world_mut()
            .insert_resource(crate::bootstrap::ProductionDurations(
                std::collections::BTreeMap::from([(recipe.clone(), 20)]),
            ));
        let kind = app
            .world()
            .entity(
                app.world()
                    .iter_entities()
                    .find(|entity| entity.get::<crate::UnitId>().is_some_and(|id| id.0 == 1))
                    .unwrap()
                    .id(),
            )
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        app.world_mut().spawn((
            crate::UnitId(2),
            crate::UnitType(kind),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(12.0, 8.0)),
            UnitOrders::default(),
        ));
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:produce").unwrap(),
                behaviour: Behaviour::Produce,
                target_kind: turts_mod_api::TargetKind::Content,
                policy: None,
                units: vec![2, 1],
                target: OrderTargetSim::Content(recipe),
                queue: QueueMode::Append,
                group: 1,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1, 2]);
        assert_eq!(
            result.outcome,
            Ok(crate::Applied {
                job_id: Some(crate::JobId(1))
            })
        );
        let queues = app
            .world()
            .resource::<crate::economy::production::ProductionQueues>();
        assert_eq!(queues.0.get(&1).unwrap()[0].id, crate::JobId(1));
        assert_eq!(queues.0.get(&2).unwrap()[0].id, crate::JobId(2));
        let owner_view = crate::economy::views::world_view(app.world_mut());
        assert_eq!(
            owner_view
                .economy
                .queues
                .iter()
                .find(|queue| queue.producer == 1)
                .unwrap()
                .jobs[0]
                .id,
            1
        );
        let cancel = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:cancel").unwrap(),
                behaviour: Behaviour::CancelProduction,
                target_kind: turts_mod_api::TargetKind::Job,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Job(result.outcome.unwrap().job_id.unwrap()),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let cancelled = crate::apply_command(app.world_mut(), &cancel);
        assert_eq!(cancelled.units.accepted, vec![1]);
        let owner_view = crate::economy::views::world_view(app.world_mut());
        assert!(
            owner_view
                .economy
                .queues
                .iter()
                .all(|queue| queue.producer != 1 || queue.jobs.is_empty())
        );
    }

    #[test]
    fn depleted_gather_writes_last_failure() {
        let mut app = crate::headless_app(20);
        let kind = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &PackageId("test".into()),
                UnitDef {
                    id: "worker".into(),
                    name: "Worker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    footprint: Footprint::Circle { radius: 0.5 },
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            )
            .unwrap();
        let order_id = ContentId::parse("test:gather").unwrap();
        let entity = app
            .world_mut()
            .spawn((
                crate::UnitId(1),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(8.0, 8.0)),
                UnitOrders {
                    current: Some(Order {
                        order: order_id.clone(),
                        behaviour: Behaviour::Gather,
                        target: OrderTargetSim::Node(NodeId(1)),
                        group: 0,
                    }),
                    queue: VecDeque::new(),
                    last_failure: None,
                },
            ))
            .id();
        app.world_mut()
            .insert_resource(crate::economy::SurfaceNodes(vec![
                crate::economy::NodeRecord {
                    id: NodeId(1),
                    content_id: ContentId::parse("test:ore").unwrap(),
                    position: crate::Position(Vec2::new(10.0, 8.0)),
                    remaining: 0,
                },
            ]));
        app.world_mut()
            .resource_mut::<crate::EconomyOrders>()
            .0
            .insert(
                1,
                crate::economy::GatherOrder {
                    node: Some(NodeId(1)),
                    deposit: None,
                    receiver: None,
                    stage: crate::GatherStage::Gathering,
                    carry: crate::RateCarry::default(),
                },
            );
        app.world_mut().resource_mut::<crate::Tick>().0 = 4;
        app.world_mut().resource_mut::<crate::AuditTrail>().enable();

        run_orders(app.world_mut());

        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert_eq!(
            orders.last_failure,
            Some(OrderFailure {
                order: order_id,
                reason: crate::RejectReason::NodeDepleted,
                tick: 4,
            })
        );
        assert_eq!(
            app.world().resource::<crate::AuditTrail>().operations[0].phase,
            crate::economy::transaction::AuditPhase::Orders
        );
    }

    #[test]
    fn queued_gather_that_depletes_before_activation_fails_and_idles() {
        let mut app = crate::headless_app(20);
        let kind = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &PackageId("test".into()),
                UnitDef {
                    id: "worker".into(),
                    name: "Worker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    footprint: Footprint::Circle { radius: 0.5 },
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            )
            .unwrap();
        let order_id = ContentId::parse("test:gather").unwrap();
        let entity = app
            .world_mut()
            .spawn((
                crate::UnitId(1),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(8.0, 8.0)),
                UnitOrders {
                    current: None,
                    queue: VecDeque::from([Order {
                        order: order_id.clone(),
                        behaviour: Behaviour::Gather,
                        target: OrderTargetSim::Node(NodeId(1)),
                        group: 0,
                    }]),
                    last_failure: None,
                },
            ))
            .id();
        app.world_mut()
            .insert_resource(crate::economy::SurfaceNodes(vec![
                crate::economy::NodeRecord {
                    id: NodeId(1),
                    content_id: ContentId::parse("test:ore").unwrap(),
                    position: crate::Position(Vec2::new(10.0, 8.0)),
                    remaining: 0,
                },
            ]));
        app.world_mut().resource_mut::<crate::Tick>().0 = 7;
        app.world_mut().resource_mut::<crate::AuditTrail>().enable();

        run_orders(app.world_mut());

        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert!(orders.queue.is_empty());
        assert_eq!(
            orders.last_failure,
            Some(OrderFailure {
                order: order_id,
                reason: crate::RejectReason::NodeDepleted,
                tick: 7,
            })
        );
        assert_eq!(
            app.world().resource::<crate::AuditTrail>().operations.len(),
            1
        );
        assert_eq!(
            app.world().resource::<crate::AuditTrail>().operations[0].phase,
            crate::economy::transaction::AuditPhase::Orders
        );
    }
}
