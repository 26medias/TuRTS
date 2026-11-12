//! Per-unit orders and the fixed-tick order phase.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy_ecs::prelude::{Component, Resource, World};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use turts_mod_api::{Behaviour, ContentId};

use crate::world::MoveTarget;
use crate::{JobId, NodeId, SlotId};

#[cfg(test)]
thread_local! {
    static PENDING_QUEUE_MEMBER_SCAN_STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PENDING_CAPACITY_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PLAIN_CANDIDATE_INDEX_STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PLAIN_PROMOTION_HEADER_STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static LEGACY_WAITER_ENTITY_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn count_pending_queue_member_scan_step() {
    PENDING_QUEUE_MEMBER_SCAN_STEPS.with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
fn count_pending_capacity_lookup() {
    PENDING_CAPACITY_LOOKUPS.with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
fn count_plain_candidate_index_step() {
    PLAIN_CANDIDATE_INDEX_STEPS.with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
fn count_plain_promotion_header_step() {
    PLAIN_PROMOTION_HEADER_STEPS.with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
fn count_legacy_waiter_entity_lookup() {
    LEGACY_WAITER_ENTITY_LOOKUPS.with(|count| count.set(count.get() + 1));
}

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
struct PlainMoveReservations {
    entries: BTreeMap<u32, PlainMoveReservation>,
    slot_counts: BTreeMap<u8, usize>,
    winners: BTreeSet<u32>,
    next_ordinal: u64,
    budget_tick: Option<u32>,
    budget_start_slot: u8,
    budget_used: usize,
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

    fn remove(&mut self, unit_id: &u32) -> Option<PlainMoveReservation> {
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

/// One-shot marker for queued Unit-target activation. Active follows refresh
/// directly and do not become new capped envelopes on later ticks.
#[derive(Resource, Default)]
struct UnitMoveNeedsAllocation(BTreeSet<u32>);

#[derive(Debug, Clone)]
struct PlainMoveReservation {
    slot: SlotId,
    accepted_tick: u32,
    seq: u32,
    arrival_ordinal: u64,
    preceding_pool_id: u64,
    point: Vec2,
    order: Order,
    prior: PlainMovePriorState,
}

#[derive(Debug, Clone, Default)]
struct PlainMovePriorState {
    unit_orders: Option<UnitOrders>,
    move_target: Option<MoveTarget>,
    move_progress: Option<crate::movement::MoveProgress>,
    plain_progress: Option<crate::movement::PlainMoveProgress>,
    physical_escape: Option<crate::movement::PhysicalEscape>,
    gather: Option<crate::economy::GatherOrder>,
    identity: Option<Order>,
}

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

    fn can_queue_after_replacing(
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

fn pending_live_member_count(
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire: Option<turts_mod_api::FirePolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engage: Option<turts_mod_api::EngagePolicy>,
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

#[derive(Default)]
struct PromotionHeaderIndex {
    prefix_max: Vec<(u32, u64)>,
}

impl PromotionHeaderIndex {
    fn new(queue: &VecDeque<PendingMovementOrder>) -> Self {
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

fn insert_promoted_request(
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

/// Run one fixed-tick order phase.
pub(crate) fn run_orders(world: &mut World) {
    dispatch_pending_movement(world);
    let mut unit_entities = BTreeMap::new();
    let mut engagement_points = Vec::new();
    let mut engagement_positions = BTreeMap::new();
    let mut engagement_owners = BTreeMap::new();
    for entity in world.iter_entities() {
        let Some(id) = entity.get::<crate::world::UnitId>() else {
            continue;
        };
        unit_entities.insert(id.0, entity.id());
        let (Some(position), Some(owner)) = (
            entity.get::<crate::world::Position>(),
            entity.get::<crate::world::Owner>(),
        ) else {
            continue;
        };
        engagement_points.push(crate::grid::GridPoint {
            id: id.0,
            position: position.0,
        });
        engagement_positions.insert(id.0, position.0);
        engagement_owners.insert(id.0, owner.0.0);
    }
    engagement_points.sort_unstable_by_key(|point| point.id);
    let engagement_cell_size = world
        .get_resource::<crate::bootstrap::InstalledCatalog>()
        .into_iter()
        .flat_map(|catalog| catalog.0.units.values())
        .filter_map(|unit| unit.weapon.as_ref().map(|weapon| weapon.range))
        .fold(f32::EPSILON, f32::max);
    let engagement_grid = crate::grid::UniformGrid::new(&engagement_points, engagement_cell_size);
    for (&unit_id, &entity) in &unit_entities {
        let replacement_pending = unit_has_pending_move(world, unit_id);
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
        let current_is_unit_follow = world
            .entity(entity)
            .get::<UnitOrders>()
            .and_then(|orders| orders.current.as_ref())
            .is_some_and(|current| {
                matches!(current.behaviour, Behaviour::Move | Behaviour::Attack)
                    && matches!(current.target, OrderTargetSim::Unit(_))
            });
        if completed_plain_fallback && !gather_still_approaching && !current_is_unit_follow {
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
        if plain_blocked && gather_still_approaching {
            // M2 design 5: a gather order waits visibly if movement cannot reach range, and
            // the cycle never discards cargo. So a blocked trip of a gather cycle does not
            // fail and does not clear the cycle: it waits, then tries the approach again
            // (gather.rs, ruling O16 extension). The order keeps its own end rules.
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
                crate::economy::gather::wait_after_blocked_trip(world, unit_id);
            });
            continue;
        }
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
                if (has_queued || had_current) && !unit_has_pending_move(world, unit_id) {
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
                if !unit_has_pending_move(world, unit_id) {
                    start_next_queued_order(world, slot, entity, unit_id);
                }
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
            if waiting && plain_move_landed && !replacement_pending {
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
        if matches!(current.behaviour, Behaviour::Move | Behaviour::Attack)
            && let OrderTargetSim::Unit(target_id) = current.target
        {
            if world
                .get_resource::<UnitMoveNeedsAllocation>()
                .is_some_and(|pending| pending.0.contains(&unit_id))
            {
                // A queued Unit follow is a new envelope. It must pass through
                // the shared cap before ordinary follow completion can advance
                // the actor's post-move queue.
                continue;
            }
            if current.behaviour == Behaviour::Attack {
                run_unit_attack(world, entity, unit_id, current, target_id, &unit_entities);
            } else {
                run_unit_follow_move(world, entity, unit_id, current, target_id, &unit_entities);
            }
            continue;
        }
        if current.behaviour == Behaviour::Attack
            && let OrderTargetSim::Point(point) = current.target
            && world.entity(entity).get::<MoveTarget>().is_none()
        {
            let engage = world
                .resource::<crate::bootstrap::InstalledCatalog>()
                .0
                .orders
                .get(&current.order)
                .is_some_and(|definition| {
                    definition.engage == Some(turts_mod_api::EngagePolicy::StopAndFire)
                });
            let target_in_range = engage
                && attack_point_has_target_in_range(
                    world,
                    entity,
                    unit_id,
                    &engagement_grid,
                    &engagement_points,
                    &engagement_positions,
                    &engagement_owners,
                );
            if target_in_range {
                continue;
            }
            let resting_at_slot = world
                .entity(entity)
                .get::<crate::movement::MoveProgress>()
                .is_some_and(|progress| {
                    progress.activity == crate::movement::MovementActivity::Resting
                        && world
                            .entity(entity)
                            .get::<crate::world::Position>()
                            .is_some_and(|position| position.0 == progress.slot)
                })
                || (world
                    .entity(entity)
                    .get::<crate::movement::MoveProgress>()
                    .is_none()
                    && world
                        .entity(entity)
                        .get::<crate::world::Position>()
                        .is_some_and(|position| position.0 == point));
            if !resting_at_slot {
                let resume = world
                    .entity(entity)
                    .get::<crate::movement::MoveProgress>()
                    .map_or(point, |progress| progress.slot);
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    unit_id,
                    |world| {
                        world.entity_mut(entity).insert(MoveTarget(resume));
                    },
                );
                continue;
            }
        }
        let completed_move = matches!(current.behaviour, Behaviour::Move | Behaviour::Attack)
            && matches!(current.target, OrderTargetSim::Point(_))
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
            }
            // Design 5.3 with M2 design 5: a depleted node starts the delivery of the cargo
            // that the gatherer holds, and the gather cycle never discards cargo. So the order
            // fails only when the cycle holds no cargo to deliver.
            && !crate::economy::gather::delivers_cargo(world, unit_id);
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
                if replacement_pending {
                    None
                } else {
                    orders.queue.pop_front()
                }
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
                if !unit_has_pending_move(world, unit_id) {
                    start_next_queued_order(world, slot, entity, unit_id);
                }
            }
        });
    }
    queue_unallocated_move_orders(world);
    dispatch_pending_movement(world);
    finish_movement_dispatch_tick(world);
}

fn attack_point_has_target_in_range(
    world: &World,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    grid: &crate::grid::UniformGrid,
    points: &[crate::grid::GridPoint],
    positions: &BTreeMap<u32, Vec2>,
    owners: &BTreeMap<u32, u8>,
) -> bool {
    let Some(origin) = positions.get(&unit_id).copied() else {
        return false;
    };
    let Some(owner) = owners.get(&unit_id).copied() else {
        return false;
    };
    let Some(unit_type) = world.entity(entity).get::<crate::world::UnitType>() else {
        return false;
    };
    let Some(unit_definition) = world
        .resource::<crate::registry::UnitTypes>()
        .get(unit_type.0)
    else {
        return false;
    };
    let Ok(content_id) = turts_mod_api::ContentId::parse(&unit_definition.content_id) else {
        return false;
    };
    let Some(weapon) = world
        .resource::<crate::bootstrap::InstalledCatalog>()
        .0
        .units
        .get(&content_id)
        .and_then(|unit| unit.weapon.as_ref())
    else {
        return false;
    };
    let Some(healths) = world.get_resource::<crate::economy::combat::Healths>() else {
        return false;
    };
    grid.within(points, origin, weapon.range)
        .into_iter()
        .any(|index| {
            let target = points[index].id;
            let Some(target_owner) = owners.get(&target).copied() else {
                return false;
            };
            if !healths.0.get(&target).is_some_and(|health| *health > 0) {
                return false;
            }
            let relation_matches = match weapon.targets.slots {
                turts_mod_api::SlotRelation::Other => target_owner != owner,
                turts_mod_api::SlotRelation::Same => target_owner == owner,
                turts_mod_api::SlotRelation::Any => true,
            };
            relation_matches && positions.contains_key(&target)
        })
}

fn finish_movement_dispatch_tick(world: &mut World) {
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

fn arbitrate_plain_move_reservations(world: &mut World) {
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

fn record_dispatch_selection(world: &mut World, tick: u32, start_slot: u8, count: usize) {
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

fn record_plain_dispatch_selection(world: &mut World, tick: u32, start_slot: u8, count: usize) {
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

fn reservation_precedes_request(
    _unit_id: u32,
    reservation: &PlainMoveReservation,
    request: &PendingMovementOrder,
) -> bool {
    reservation.accepted_tick < request.accepted_tick
        || (reservation.accepted_tick == request.accepted_tick
            && request.pool_id > reservation.preceding_pool_id)
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

fn dispatch_pending_movement(world: &mut World) {
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

fn attack_goal(
    world: &World,
    entity: bevy_ecs::entity::Entity,
    target_position: bevy_math::Vec2,
) -> Option<bevy_math::Vec2> {
    let actor = world.entity(entity).get::<crate::world::Position>()?.0;
    let unit_type = world.entity(entity).get::<crate::world::UnitType>()?.0;
    let unit = world
        .resource::<crate::registry::UnitTypes>()
        .get(unit_type)?;
    let id = turts_mod_api::ContentId::parse(&unit.content_id).ok()?;
    let weapon = world
        .resource::<crate::bootstrap::InstalledCatalog>()
        .0
        .units
        .get(&id)?
        .weapon
        .as_ref()?;
    let delta = target_position - actor;
    (delta.length_squared() > weapon.range * weapon.range).then(|| {
        clamp_target(
            world,
            target_position - delta.normalize_or_zero() * weapon.range,
            unit.radius,
        )
    })
}

fn run_unit_attack(
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
            .map(|position| position.0)
    });
    let Some(target_position) = target_position else {
        let fired = world
            .get_resource::<crate::economy::combat::Weapons>()
            .and_then(|weapons| weapons.0.get(&unit_id))
            .is_some_and(|state| state.ordered_target_fired == Some(target_id));
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            let mut entity_mut = world.entity_mut(entity);
            if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                orders.current = None;
                if !fired {
                    orders.last_failure = Some(OrderFailure {
                        order: current.order.clone(),
                        reason: crate::command::RejectReason::TargetGone,
                        tick,
                    });
                }
            }
            entity_mut.remove::<MoveTarget>();
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
        return;
    };
    let Some(goal) = attack_goal(world, entity, target_position) else {
        if world.entity(entity).get::<MoveTarget>().is_some() {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                let mut entity_mut = world.entity_mut(entity);
                entity_mut.remove::<MoveTarget>();
                entity_mut.remove::<crate::movement::MoveProgress>();
                entity_mut.remove::<crate::movement::PlainMoveProgress>();
            });
        }
        return;
    };
    let can_move = world
        .entity(entity)
        .get::<crate::world::UnitType>()
        .and_then(|kind| world.resource::<crate::registry::UnitTypes>().get(kind.0))
        .is_some_and(|unit| unit.movement.is_some());
    if !can_move {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                orders.current = None;
                orders.last_failure = Some(OrderFailure {
                    order: current.order.clone(),
                    reason: crate::command::RejectReason::OutOfRange,
                    tick,
                });
            }
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
        return;
    }
    if world
        .entity(entity)
        .get::<MoveTarget>()
        .is_none_or(|target| target.0 != goal)
    {
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            entity_mut.insert(MoveTarget(goal));
        });
    }
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
            world
                .entity_mut(entity)
                .remove::<crate::movement::MoveProgress>();
            world
                .entity_mut(entity)
                .remove::<crate::movement::PlainMoveProgress>();
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
        return;
    };
    let Some(current_position) = current_position else {
        return;
    };
    let target_goal = clamp_target(world, target_position, unit_radius(world, entity));
    let cohort_progress = world
        .entity(entity)
        .get::<crate::movement::MoveProgress>()
        .cloned();
    let cohort_pool_goal = cohort_progress.as_ref().and_then(|progress| {
        world
            .get_resource::<crate::movement::MovementPools>()
            .and_then(|pools| pools.0.get(&progress.pool_id))
            .map(|pool| pool.point)
    });
    let same_goal_cohort = cohort_progress.as_ref().is_some_and(|progress| {
        progress
            .follow_origin
            .is_some_and(|origin| origin.target_unit_id == target_id && origin.goal == target_goal)
            || (progress.follow_origin.is_none() && cohort_pool_goal == Some(target_goal))
    });
    let cohort_fallback_reached = cohort_progress.as_ref().is_some_and(|progress| {
        same_goal_cohort
            && progress.activity == crate::movement::MovementActivity::Resting
            && progress.slot == current_position
            && world.entity(entity).get::<MoveTarget>().is_none()
            && world
                .entity(entity)
                .get::<crate::movement::PhysicalEscape>()
                .is_none()
    });
    let plain_progress = world
        .entity(entity)
        .get::<crate::movement::PlainMoveProgress>()
        .cloned();
    let same_goal_fallback = plain_progress
        .as_ref()
        .is_some_and(|progress| progress.original_point == target_goal);
    let fallback_reached = same_goal_fallback
        && plain_progress.as_ref().is_some_and(|progress| {
            progress.activity == crate::movement::MovementActivity::Resting
                && progress.slot == current_position
                && world.entity(entity).get::<MoveTarget>().is_none()
                && world
                    .entity(entity)
                    .get::<crate::movement::PhysicalEscape>()
                    .is_none()
        });
    let physical_escape_active = world
        .entity(entity)
        .get::<crate::movement::PhysicalEscape>()
        .is_some();
    if (current_position == target_goal && !physical_escape_active)
        || fallback_reached
        || cohort_fallback_reached
    {
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
            if cohort_progress.is_some() {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::MoveProgress>();
            }
            if plain_progress.is_some() {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
            }
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
    } else if same_goal_fallback || same_goal_cohort {
        // A fallback for the unchanged legal goal is still active or resting;
        // refreshing the target actor must not reopen or erase that result.
    } else {
        let move_target = world
            .entity(entity)
            .get::<crate::world::MoveTarget>()
            .map(|target| target.0);
        if move_target != Some(target_goal) || plain_progress.is_some() {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                if cohort_progress.is_some() {
                    world
                        .entity_mut(entity)
                        .remove::<crate::movement::MoveProgress>();
                }
                let mut entity_mut = world.entity_mut(entity);
                entity_mut.remove::<crate::movement::MoveProgress>();
                entity_mut.remove::<crate::movement::PlainMoveProgress>();
                entity_mut.insert(crate::world::MoveTarget(target_goal));
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
            world.init_resource::<UnitMoveNeedsAllocation>();
            world
                .resource_mut::<UnitMoveNeedsAllocation>()
                .0
                .insert(unit);
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Attack, OrderTargetSim::Unit(target_id)) => {
            let target_position = world
                .iter_entities()
                .find(|candidate| {
                    candidate
                        .get::<crate::world::UnitId>()
                        .is_some_and(|id| id.0 == *target_id)
                })
                .and_then(|candidate| candidate.get::<crate::world::Position>().map(|pos| pos.0))
                .ok_or(crate::command::RejectReason::TargetGone)?;
            let target = attack_goal(world, entity, target_position);
            if let Some(mut weapons) = world.get_resource_mut::<crate::economy::combat::Weapons>()
                && let Some(state) = weapons.0.get_mut(&unit)
            {
                state.last_target = None;
                state.ordered_target_fired = None;
            }
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            if let Some(target) = target {
                entity_mut.insert(crate::world::MoveTarget(target));
            } else {
                entity_mut.remove::<crate::world::MoveTarget>();
            }
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Attack, OrderTargetSim::Point(point)) => {
            let target = clamp_target(world, *point, unit_radius(world, entity));
            if let Some(mut weapons) = world.get_resource_mut::<crate::economy::combat::Weapons>()
                && let Some(state) = weapons.0.get_mut(&unit)
            {
                state.last_target = None;
                state.ordered_target_fired = None;
            }
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

        let deferred_move = (command.behaviour == Behaviour::Move
            || (command.behaviour == Behaviour::Attack
                && matches!(command.target, OrderTargetSim::Point(_))))
            && command.queue == QueueMode::Replace
            && (((command.group != 0 || command.units.len() > 1)
                && matches!(
                    &command.target,
                    OrderTargetSim::Point(_) | OrderTargetSim::Unit(_)
                ))
                || matches!(&command.target, OrderTargetSim::Unit(_)));
        let singleton_replace_move = command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && command.group == 0
            && command.units.len() == 1
            && matches!(&command.target, OrderTargetSim::Point(_));
        if singleton_replace_move
            && !plain_move_reservation_capacity_available(world, slot, *unit_id)
        {
            outcomes.refused.push((*unit_id, R::RateLimited));
            continue;
        }
        let inherited_prior = if singleton_replace_move {
            let reservation = world
                .get_resource_mut::<PlainMoveReservations>()
                .and_then(|mut reservations| reservations.remove(unit_id));
            reservation.map(|reservation| {
                restore_plain_move_prior(world, entity, *unit_id, &reservation.prior, false);
                reservation.prior
            })
        } else {
            None
        };
        let reservation_prior = singleton_replace_move.then(|| {
            inherited_prior
                .clone()
                .unwrap_or_else(|| capture_plain_move_prior(world, entity, *unit_id))
        });
        let order = Order {
            order: command.order.clone(),
            behaviour: command.behaviour,
            target: command.target.clone(),
            group: command.group,
        };
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
            && (plain_move_has_no_prior_order_state(world, command)
                || plain_move_reservation_exists(world, *unit_id));
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
            remember_plain_move_identity(world, *unit_id, order.clone());
            record_plain_move_reservation(
                world,
                *unit_id,
                slot,
                seq,
                target,
                order.clone(),
                reservation_prior.clone().unwrap_or_default(),
            );
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
            let target = clamp_target(world, *target, unit_radius(world, entity));
            crate::economy::transaction::capture_unit_order_operation(world, *unit_id, |world| {
                world
                    .resource_mut::<PendingMovementOrders>()
                    .cancel_units(slot.0, &[*unit_id]);
                let mut movement = world.entity_mut(entity);
                movement.remove::<MoveTarget>();
                movement.remove::<crate::movement::MoveProgress>();
                movement.remove::<crate::movement::PlainMoveProgress>();
                crate::command::apply_legacy_move_to(world, slot, &[*unit_id], target)
                    .expect("order validation precedes legacy move application");
                if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                    *orders = UnitOrders::default();
                }
                clear_gather(world, *unit_id);
                remember_plain_move_identity(world, *unit_id, order.clone());
            });
            record_plain_move_reservation(
                world,
                *unit_id,
                slot,
                seq,
                target,
                order.clone(),
                reservation_prior.clone().unwrap_or_default(),
            );
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
                if command.queue == QueueMode::Replace
                    && command.behaviour != Behaviour::Move
                    && let Some(mut reservations) =
                        world.get_resource_mut::<PlainMoveReservations>()
                {
                    reservations.remove(unit_id);
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
            OrderTargetSim::Point(point) => point,
            OrderTargetSim::Unit(target_id) => entity_by_unit
                .get(&target_id)
                .and_then(|entity| {
                    world
                        .entity(*entity)
                        .get::<crate::world::Position>()
                        .map(|p| p.0)
                })
                .expect("validated unit target remains present during command application"),
            _ => unreachable!("deferred movement has a point or unit target"),
        };
        let point = clamp_target(world, point, max_radius);
        let tick = world.resource::<crate::world::Tick>().0;
        let can_queue = world
            .resource::<PendingMovementOrders>()
            .can_queue_after_replacing(slot.0, command.group, tick, &spread_units)
            && movement_envelope_capacity_available_after_replacing(world, slot, &spread_units);
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
        } else {
            let request = PendingMovementOrder {
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
            };
            let mut request = Some(request);
            spread_units.sort_unstable();
            for unit_id in &spread_units {
                let Some(entity) = entity_by_unit.get(unit_id).copied() else {
                    continue;
                };
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    *unit_id,
                    |world| {
                        if let Some(request) = request.take() {
                            let mut pending = world.resource_mut::<PendingMovementOrders>();
                            pending.cancel_units(slot.0, &spread_units);
                            assert!(
                                pending.queue(request),
                                "capacity was preflighted before captured envelope admission"
                            );
                        }
                        if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                            orders.queue.clear();
                        }
                        let canceled_plain = world
                            .get_resource_mut::<PlainMoveReservations>()
                            .and_then(|mut reservations| reservations.remove(unit_id));
                        if let Some(reservation) = canceled_plain {
                            restore_plain_move_prior(
                                world,
                                entity,
                                *unit_id,
                                &reservation.prior,
                                false,
                            );
                        }
                    },
                );
            }
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

pub(crate) fn plain_move_reservation_exists(world: &World, unit_id: u32) -> bool {
    world
        .get_resource::<PlainMoveReservations>()
        .is_some_and(|reservations| reservations.entries.contains_key(&unit_id))
}

fn plain_move_reservation_capacity_available(world: &World, slot: SlotId, unit_id: u32) -> bool {
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

fn movement_envelope_capacity_available_after_replacing(
    world: &World,
    slot: SlotId,
    unit_ids: &[u32],
) -> bool {
    let replace_ids: std::collections::HashSet<_> = unit_ids.iter().copied().collect();
    let pending = world.resource::<PendingMovementOrders>();
    let current_pending = pending
        .pending_units
        .get(&slot.0)
        .copied()
        .unwrap_or_default();
    let removed_pending = replace_ids
        .iter()
        .filter_map(|unit_id| {
            #[cfg(test)]
            count_pending_capacity_lookup();
            pending.pending_member_counts.get(unit_id)
        })
        .copied()
        .sum::<usize>();
    let pending_after = current_pending
        .saturating_sub(removed_pending)
        .saturating_add(replace_ids.len());
    let reserved_after = world
        .get_resource::<PlainMoveReservations>()
        .map_or(0, |reservations| {
            reservations
                .slot_counts
                .get(&slot.0)
                .copied()
                .unwrap_or_default()
                .saturating_sub(
                    replace_ids
                        .iter()
                        .filter(|unit_id| {
                            reservations
                                .entries
                                .get(unit_id)
                                .is_some_and(|reservation| reservation.slot == slot)
                        })
                        .count(),
                )
        });
    pending_after.saturating_add(reserved_after) <= 16_384
}

fn capture_plain_move_prior(
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

fn restore_plain_move_prior(
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

fn unit_has_pending_move(world: &World, unit_id: u32) -> bool {
    world
        .resource::<PendingMovementOrders>()
        .pending_member_counts
        .contains_key(&unit_id)
        || plain_move_reservation_exists(world, unit_id)
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
        Behaviour::Attack => match order.target {
            OrderTargetSim::Unit(_) | OrderTargetSim::Point(_) => {
                let pending_replace =
                    command.queue == QueueMode::Append && unit_has_pending_move(world, unit_id);
                let active_order = order.clone();
                if command.queue == QueueMode::Replace {
                    clear_replaced_movement(world, slot, entity, unit_id);
                }
                let mut orders = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .cloned()
                    .unwrap_or_default();
                let starts_now = if command.queue == QueueMode::Append {
                    if orders.current.is_some()
                        || world.entity(entity).get::<MoveTarget>().is_some()
                        || unit_has_pending_move(world, unit_id)
                    {
                        orders.queue.push_back(order.clone());
                        false
                    } else {
                        orders.current = Some(order);
                        true
                    }
                } else {
                    orders.current = Some(order);
                    orders.queue.clear();
                    true
                };
                world.entity_mut(entity).insert(orders);
                if starts_now {
                    if let Some(mut weapons) =
                        world.get_resource_mut::<crate::economy::combat::Weapons>()
                        && let Some(state) = weapons.0.get_mut(&unit_id)
                    {
                        state.last_target = None;
                        state.ordered_target_fired = None;
                    }
                    start_order(world, slot, entity, unit_id, &active_order)?;
                }
                if !pending_replace {
                    clear_gather(world, unit_id);
                }
                Ok(None)
            }
            _ => Err(R::TargetInvalid),
        },
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
    }
}

pub(crate) fn clear_replaced_movement(
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

fn record_plain_move_reservation(
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

    #[test]
    fn legacy_waiter_preparation_uses_one_indexed_lookup_per_member() {
        const MEMBERS: u32 = 512;
        let mut world = World::new();
        let mut entity_by_unit = std::collections::HashMap::new();
        for unit_id in 1..=MEMBERS {
            let entity = world
                .spawn((
                    crate::world::UnitId(unit_id),
                    UnitOrders {
                        current: Some(Order {
                            order: ContentId::parse("test:move").unwrap(),
                            behaviour: Behaviour::Move,
                            target: OrderTargetSim::Point(Vec2::new(12.0, 8.0)),
                            group: 0,
                        }),
                        queue: VecDeque::from([Order {
                            order: ContentId::parse("test:old-future").unwrap(),
                            behaviour: Behaviour::Hold,
                            target: OrderTargetSim::None,
                            group: 0,
                        }]),
                        ..Default::default()
                    },
                ))
                .id();
            entity_by_unit.insert(unit_id, entity);
        }
        LEGACY_WAITER_ENTITY_LOOKUPS.with(|count| count.set(0));
        prepare_legacy_move_to_waiters(
            &mut world,
            &(1..=MEMBERS).collect::<Vec<_>>(),
            &entity_by_unit,
        );
        assert_eq!(
            LEGACY_WAITER_ENTITY_LOOKUPS.with(std::cell::Cell::get),
            MEMBERS as usize,
            "each accepted member uses the shared entity index once"
        );
        assert!((1..=MEMBERS).all(|unit_id| {
            world
                .entity(entity_by_unit[&unit_id])
                .get::<UnitOrders>()
                .is_some_and(|orders| orders.queue.is_empty())
        }));
    }

    #[test]
    fn unrelated_pending_members_do_not_scan_for_singleton_replacement_preflight() {
        PENDING_QUEUE_MEMBER_SCAN_STEPS.with(|count| count.set(0));
        let mut pending = PendingMovementOrders::default();
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1000,
            group: 91,
            pool_id: 0,
            point: Vec2::new(48.0, 48.0),
            unit_ids: (1000..17_000).collect(),
            order: None,
            member_orders: BTreeMap::new(),
            plain: false,
        }));
        assert_eq!(pending.pending_units.get(&0), Some(&16_000));

        for unit_id in 20_000..21_000 {
            assert!(pending.can_queue_after_replacing(0, 0, 1, &[unit_id]));
            pending.cancel_units(0, &[unit_id]);
        }

        PENDING_QUEUE_MEMBER_SCAN_STEPS.with(|count| {
            assert_eq!(
                count.get(),
                0,
                "unrelated pending members must use the maintained membership index"
            );
        });
        assert_eq!(pending.pending_units.get(&0), Some(&16_000));
        assert_eq!(pending.pending_member_counts.len(), 16_000);
    }

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
    fn accepted_legacy_gather_clears_stale_movement_state_and_identity() {
        let (mut app, entity) = gatherer_app();
        let worker = ContentId::parse("test:worker").unwrap();
        let item = ContentId::parse("test:iron").unwrap();
        app.world_mut()
            .resource_mut::<crate::bootstrap::InstalledCatalog>()
            .0
            .units
            .get_mut(&worker)
            .unwrap()
            .gatherer
            .as_mut()
            .unwrap()
            .items = vec![item.clone()];
        app.world_mut()
            .insert_resource(crate::economy::EconomyOrders(BTreeMap::from([(
                1,
                crate::economy::GatherOrder {
                    node: None,
                    deposit: None,
                    receiver: None,
                    stage: crate::economy::gather::GatherStage::Idle,
                    carry: crate::economy::gather::RateCarry::default(),
                },
            )])));
        app.world_mut().insert_resource(crate::EconomyLedger {
            inventories: BTreeMap::from([(
                1,
                crate::economy::ledger::Inventory {
                    capacity: 4,
                    accepted_items: std::collections::BTreeSet::from([item]),
                    items: BTreeMap::new(),
                },
            )]),
            ..Default::default()
        });
        app.world_mut().entity_mut(entity).insert((
            crate::movement::MoveProgress {
                pool_id: 5,
                slot: Vec2::new(30.0, 30.0),
                activity: crate::movement::MovementActivity::Blocked,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 20,
                best_distance: 1.0,
                claim_count: 3,

                follow_origin: None,
            },
            crate::movement::PlainMoveProgress {
                original_point: Vec2::new(30.0, 30.0),
                slot: Vec2::new(30.0, 30.0),
                pool_slots: vec![],
                gather_goal: None,
                activity: crate::movement::MovementActivity::Blocked,
                stalled_ticks: 20,
                best_distance: 1.0,
                claim_count: 0,
                failure_recorded: true,
            },
            MoveTarget(Vec2::new(30.0, 30.0)),
            UnitOrders {
                current: Some(Order {
                    order: ContentId::parse("test:move-a").unwrap(),
                    behaviour: Behaviour::Move,
                    target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
                    group: 4,
                }),
                queue: VecDeque::from([Order {
                    order: ContentId::parse("test:hold-c").unwrap(),
                    behaviour: Behaviour::Hold,
                    target: OrderTargetSim::None,
                    group: 4,
                }]),
                last_failure: Some(OrderFailure {
                    order: ContentId::parse("test:old-failure").unwrap(),
                    reason: crate::RejectReason::Blocked,
                    tick: 7,
                }),
            },
        ));
        app.world_mut()
            .insert_resource(PlainMoveOrderIdentities(BTreeMap::from([(
                1,
                Order {
                    order: ContentId::parse("test:old-move").unwrap(),
                    behaviour: Behaviour::Move,
                    target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
                    group: 0,
                },
            )])));

        let result = crate::command::apply_command(
            app.world_mut(),
            &crate::command::Command {
                slot: SlotId(0),
                seq: 8,
                kind: crate::command::CommandKind::Gather {
                    unit_id: 1,
                    node_id: NodeId(1),
                },
            },
        );

        assert!(
            result.outcome.is_ok(),
            "legacy Gather refusal: {:?}",
            result.outcome
        );
        let actor = app.world().entity(entity);
        assert!(actor.get::<MoveTarget>().is_none());
        assert!(actor.get::<crate::movement::MoveProgress>().is_none());
        assert!(actor.get::<crate::movement::PlainMoveProgress>().is_none());
        assert_eq!(actor.get::<UnitOrders>(), Some(&UnitOrders::default()));
        assert!(
            !app.world()
                .resource::<PlainMoveOrderIdentities>()
                .0
                .contains_key(&1)
        );
        assert_eq!(
            app.world()
                .resource::<crate::economy::EconomyOrders>()
                .0
                .get(&1)
                .unwrap()
                .stage,
            crate::economy::gather::GatherStage::MovingToNode
        );
        let view = crate::economy::views::world_view(app.world_mut());
        assert_eq!(
            view.unit_orders
                .iter()
                .find(|order| order.unit_id == 1)
                .unwrap()
                .activity,
            crate::economy::views::UnitOrderActivityView::Gathering
        );
    }

    #[test]
    fn refused_legacy_gather_preserves_existing_order_and_queue() {
        let (mut app, entity) = gatherer_app();
        let worker = ContentId::parse("test:worker").unwrap();
        let item = ContentId::parse("test:iron").unwrap();
        app.world_mut()
            .resource_mut::<crate::bootstrap::InstalledCatalog>()
            .0
            .units
            .get_mut(&worker)
            .unwrap()
            .gatherer
            .as_mut()
            .unwrap()
            .items = vec![item.clone()];
        app.world_mut()
            .resource_mut::<crate::economy::SurfaceNodes>()
            .0[0]
            .remaining = 0;
        app.world_mut().insert_resource(crate::EconomyLedger {
            inventories: BTreeMap::from([(
                1,
                crate::economy::ledger::Inventory {
                    capacity: 4,
                    accepted_items: std::collections::BTreeSet::from([item]),
                    items: BTreeMap::new(),
                },
            )]),
            ..Default::default()
        });
        let before = UnitOrders {
            current: Some(Order {
                order: ContentId::parse("test:move-a").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
                group: 4,
            }),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:hold-c").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 4,
            }]),
            last_failure: Some(OrderFailure {
                order: ContentId::parse("test:old-failure").unwrap(),
                reason: crate::RejectReason::Blocked,
                tick: 7,
            }),
        };
        app.world_mut().entity_mut(entity).insert(before.clone());

        let result = crate::command::apply_command(
            app.world_mut(),
            &crate::command::Command {
                slot: SlotId(0),
                seq: 9,
                kind: crate::command::CommandKind::Gather {
                    unit_id: 1,
                    node_id: NodeId(1),
                },
            },
        );

        assert_eq!(result.outcome, Err(crate::RejectReason::NodeDepleted));
        assert_eq!(
            app.world().entity(entity).get::<UnitOrders>(),
            Some(&before)
        );
        assert_eq!(
            app.world().entity(entity).get::<MoveTarget>(),
            Some(&MoveTarget(Vec2::new(30.0, 30.0)))
        );
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
    fn candidate_capacity_and_winner_indexes_visit_only_relevant_members() {
        let (mut capacity_app, _) = gatherer_app();
        let mut pending = PendingMovementOrders::default();
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1,
            group: 9,
            pool_id: 0,
            point: Vec2::new(50.0, 50.0),
            unit_ids: (10_000..26_000).collect(),
            order: None,
            member_orders: BTreeMap::new(),
            plain: false,
        }));
        capacity_app.world_mut().insert_resource(pending);
        let accepted_order = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(50.0, 50.0)),
            group: 0,
        };
        for unit_id in 27_000..27_064 {
            record_plain_move_reservation(
                capacity_app.world_mut(),
                unit_id,
                SlotId(0),
                unit_id,
                Vec2::new(50.0, 50.0),
                accepted_order.clone(),
                PlainMovePriorState::default(),
            );
        }
        PENDING_CAPACITY_LOOKUPS.with(|count| count.set(0));
        PENDING_QUEUE_MEMBER_SCAN_STEPS.with(|count| count.set(0));
        let admitted = (30_000..31_000)
            .filter(|unit_id| {
                if !plain_move_reservation_capacity_available(
                    capacity_app.world(),
                    SlotId(0),
                    *unit_id,
                ) {
                    return false;
                }
                record_plain_move_reservation(
                    capacity_app.world_mut(),
                    *unit_id,
                    SlotId(0),
                    *unit_id,
                    Vec2::new(50.0, 50.0),
                    accepted_order.clone(),
                    PlainMovePriorState::default(),
                );
                true
            })
            .count();
        assert_eq!(admitted, 320, "the indexed counts still enforce 16,384");
        assert_eq!(
            PENDING_CAPACITY_LOOKUPS.with(std::cell::Cell::get),
            1000,
            "each admission checks only its actor membership and per-slot counters"
        );
        assert_eq!(
            PENDING_QUEUE_MEMBER_SCAN_STEPS.with(std::cell::Cell::get),
            0,
            "unrelated 16,000-member requests are not rescanned for each singleton"
        );

        let (mut dispatch_app, first) = gatherer_app();
        dispatch_app
            .world_mut()
            .insert_resource(crate::world::MapBounds {
                size: Vec2::new(64.0, 64.0),
            });
        let mobile_type = dispatch_app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        let target = Vec2::new(50.0, 50.0);
        for unit_id in 1..=128_u32 {
            let entity = if unit_id == 1 {
                first
            } else {
                dispatch_app
                    .world_mut()
                    .spawn((
                        crate::UnitId(unit_id),
                        crate::UnitType(mobile_type),
                        crate::Owner(SlotId(0)),
                        crate::Position(Vec2::new(
                            4.0 + (unit_id % 12) as f32 * 1.5,
                            4.0 + (unit_id / 12) as f32 * 1.5,
                        )),
                        UnitOrders::default(),
                    ))
                    .id()
            };
            dispatch_app
                .world_mut()
                .entity_mut(entity)
                .insert(MoveTarget(target));
            record_plain_move_reservation(
                dispatch_app.world_mut(),
                unit_id,
                SlotId(0),
                unit_id,
                target,
                accepted_order.clone(),
                PlainMovePriorState::default(),
            );
        }
        PLAIN_CANDIDATE_INDEX_STEPS.with(|count| count.set(0));
        dispatch_pending_movement(dispatch_app.world_mut());
        dispatch_pending_movement(dispatch_app.world_mut());
        assert!(
            dispatch_app
                .world()
                .resource::<PlainMoveReservations>()
                .entries
                .is_empty()
        );
        assert_eq!(
            dispatch_app
                .world()
                .resource::<PlainMoveReservations>()
                .budget_used,
            128
        );
        assert!(
            dispatch_app
                .world()
                .resource::<MovementDispatchState>()
                .is_default()
        );
        assert_eq!(
            PLAIN_CANDIDATE_INDEX_STEPS.with(std::cell::Cell::get),
            128 * 3,
            "one arbitration index, one dispatch index and one pop per winner bound candidate work linearly"
        );
    }

    #[test]
    fn promotion_header_scan_is_shared_by_losers_with_the_same_predecessor() {
        const OLD: u32 = 16_000;
        const LOSERS: u32 = 64;
        let mut queue = VecDeque::new();
        for index in 0..OLD {
            queue.push_back(PendingMovementOrder {
                slot: SlotId(0),
                accepted_tick: 5,
                seq: index,
                group: 0,
                pool_id: u64::from(index + 1),
                point: Vec2::ZERO,
                unit_ids: vec![100_000 + index],
                order: None,
                member_orders: BTreeMap::new(),
                plain: true,
            });
        }
        PLAIN_PROMOTION_HEADER_STEPS.with(|count| count.set(0));
        let header = PromotionHeaderIndex::new(&queue);
        for index in (0..LOSERS).rev() {
            insert_promoted_request(
                &mut queue,
                PendingMovementOrder {
                    slot: SlotId(0),
                    accepted_tick: 6,
                    seq: index,
                    group: 0,
                    pool_id: u64::from(OLD + index + 1),
                    point: Vec2::new(index as f32, 0.0),
                    unit_ids: vec![index + 1],
                    order: None,
                    member_orders: BTreeMap::new(),
                    plain: false,
                },
                6,
                u64::from(OLD),
                &header,
            );
        }
        let promoted: Vec<_> = queue
            .iter()
            .skip(OLD as usize)
            .map(|request| request.unit_ids[0])
            .collect();
        assert_eq!(promoted, (1..=LOSERS).collect::<Vec<_>>());
        assert_eq!(
            PLAIN_PROMOTION_HEADER_STEPS.with(std::cell::Cell::get),
            OLD as usize,
            "one immutable predecessor boundary scans the old header once, regardless of loser count"
        );
    }

    #[test]
    fn promotion_boundaries_are_isolated_per_slot_and_index_distinct_predecessors_once() {
        let mut long = VecDeque::from([pending_header(0, 5, 1), pending_header(0, 5, 2)]);
        let mut short = VecDeque::from([pending_header(1, 5, 1)]);
        let long_header = PromotionHeaderIndex::new(&long);
        let short_header = PromotionHeaderIndex::new(&short);
        insert_promoted_request(&mut long, pending_header(0, 6, 3), 6, 99, &long_header);
        insert_promoted_request(&mut short, pending_header(1, 6, 3), 6, 99, &short_header);
        assert_eq!(long.len(), 3);
        assert_eq!(short.len(), 2);

        let mut unequal_long = VecDeque::from([pending_header(0, 6, 1), pending_header(0, 6, 2)]);
        let mut unequal_short = VecDeque::from([pending_header(1, 6, 1)]);
        let long_index = PromotionHeaderIndex::new(&unequal_long);
        let short_index = PromotionHeaderIndex::new(&unequal_short);
        for offset in (0..3_u64).rev() {
            insert_promoted_request(
                &mut unequal_long,
                pending_header(0, 6, 101 + offset),
                6,
                99,
                &long_index,
            );
            insert_promoted_request(
                &mut unequal_short,
                pending_header(1, 6, 201 + offset),
                6,
                99,
                &short_index,
            );
        }
        assert_eq!(
            unequal_long
                .iter()
                .map(|request| request.pool_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 101, 102, 103],
            "slot 0 keeps its original length-two boundary and FIFO order"
        );
        assert_eq!(
            unequal_short
                .iter()
                .map(|request| request.pool_id)
                .collect::<Vec<_>>(),
            vec![1, 201, 202, 203],
            "slot 1 uses its own length-one boundary and preserves FIFO"
        );

        const OLD: u32 = 16_000;
        const LOSERS: u32 = 64;
        let mut queue = VecDeque::new();
        for index in 0..OLD {
            queue.push_back(pending_header(0, 5, u64::from(index + 1)));
        }
        PLAIN_PROMOTION_HEADER_STEPS.with(|count| count.set(0));
        let header = PromotionHeaderIndex::new(&queue);
        for index in (0..LOSERS).rev() {
            insert_promoted_request(
                &mut queue,
                pending_header(0, 6, u64::from(OLD + index + 1)),
                6,
                u64::from(index),
                &header,
            );
        }
        assert_eq!(queue.len(), OLD as usize + LOSERS as usize);
        assert_eq!(
            queue
                .iter()
                .skip(OLD as usize)
                .map(|request| request.unit_ids[0])
                .collect::<Vec<_>>(),
            (OLD + 1..=OLD + LOSERS).collect::<Vec<_>>(),
            "nonmonotone raw header IDs do not change reverse-promotion FIFO"
        );
        assert_eq!(
            PLAIN_PROMOTION_HEADER_STEPS.with(std::cell::Cell::get),
            OLD as usize,
            "all predecessor queries share one immutable scan of the existing header"
        );
    }

    #[test]
    fn promotion_header_index_matches_linear_reference_for_unordered_ids_and_reverse_fifo() {
        let original = VecDeque::from([
            pending_header(0, 6, 7),
            pending_header(0, 6, 2),
            pending_header(0, 6, 9),
            pending_header(0, 6, 4),
        ]);
        let mut indexed = original.clone();
        let mut reference = original.clone();
        let header = PromotionHeaderIndex::new(&original);
        let predecessors = [10, 8, 6, 6, 1];
        for (reverse_ordinal, predecessor) in predecessors.into_iter().enumerate() {
            let promoted = pending_header(0, 6, 20 + reverse_ordinal as u64);
            insert_promoted_request(&mut indexed, promoted.clone(), 6, predecessor, &header);

            let linear_boundary = reference
                .iter()
                .position(|request| (request.accepted_tick, request.pool_id) > (6, predecessor))
                .unwrap_or(reference.len());
            reference.insert(linear_boundary, promoted);
            assert_eq!(indexed, reference, "after predecessor query {predecessor}");
        }
    }

    fn pending_header(slot: u8, accepted_tick: u32, pool_id: u64) -> PendingMovementOrder {
        PendingMovementOrder {
            slot: SlotId(slot),
            accepted_tick,
            seq: pool_id as u32,
            group: 0,
            pool_id,
            point: Vec2::ZERO,
            unit_ids: vec![pool_id as u32],
            order: None,
            member_orders: BTreeMap::new(),
            plain: true,
        }
    }

    #[test]
    fn missing_unit_target_does_not_consume_virtual_spread_capacity() {
        let (mut app, _first) = gatherer_app();
        let mut pending = PendingMovementOrders::default();
        let missing_target = Order {
            order: ContentId::parse("test:follow-missing").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Unit(999),
            group: 71,
        };
        let point_order = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(50.0, 50.0)),
            group: 71,
        };
        let member_orders = BTreeMap::from([
            (10, missing_target),
            (11, point_order.clone()),
            (12, point_order),
        ]);
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1,
            group: 71,
            pool_id: 0,
            point: Vec2::new(50.0, 50.0),
            unit_ids: vec![10, 11, 12],
            order: None,
            member_orders,
            plain: false,
        }));
        app.world_mut().insert_resource(pending);
        app.world_mut().insert_resource(MovementDispatchState {
            tick: Some(0),
            start_slot: 0,
            used: 2045,
            cursor_advanced: true,
        });
        for unit_id in [10, 11, 12] {
            app.world_mut().spawn(crate::UnitId(unit_id));
        }
        let candidate = Order {
            order: ContentId::parse("test:move-candidate").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(52.0, 50.0)),
            group: 0,
        };
        record_plain_move_reservation(
            app.world_mut(),
            1,
            SlotId(0),
            2,
            Vec2::new(52.0, 50.0),
            candidate,
            PlainMovePriorState::default(),
        );

        arbitrate_plain_move_reservations(app.world_mut());

        assert!(
            app.world()
                .resource::<PlainMoveReservations>()
                .winners
                .contains(&1),
            "the two valid pending members leave one cap slot for the later singleton"
        );
        assert_eq!(
            pending_live_member_count(
                app.world().resource::<PendingMovementOrders>().queues[&0]
                    .front()
                    .unwrap(),
                &app.world()
                    .iter_entities()
                    .filter_map(|entity| entity.get::<crate::UnitId>().map(|id| id.0))
                    .collect(),
            ),
            2
        );
    }

    #[test]
    fn dead_first_unit_target_member_does_not_anchor_surviving_envelope() {
        let (mut app, first) = gatherer_app();
        app.world_mut().entity_mut(first).insert(crate::UnitId(2));
        app.world_mut()
            .entity_mut(first)
            .insert(crate::Position(Vec2::new(12.0, 8.0)));
        let mobile_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        for (unit_id, position) in [(3, Vec2::new(24.0, 24.0)), (4, Vec2::new(48.0, 48.0))] {
            app.world_mut().spawn((
                crate::UnitId(unit_id),
                crate::UnitType(mobile_type),
                crate::Owner(SlotId(0)),
                crate::Position(position),
                UnitOrders::default(),
            ));
        }
        let dead_member_order = Order {
            order: ContentId::parse("test:follow-three").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Unit(3),
            group: 81,
        };
        let survivor_order = Order {
            order: ContentId::parse("test:follow-four").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Unit(4),
            group: 81,
        };
        let mut pending = PendingMovementOrders::default();
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1,
            group: 81,
            pool_id: 1,
            point: Vec2::new(24.0, 24.0),
            unit_ids: vec![1, 2],
            order: Some(dead_member_order.clone()),
            member_orders: BTreeMap::from([(1, dead_member_order), (2, survivor_order.clone())]),
            plain: false,
        }));
        app.world_mut().insert_resource(pending);

        dispatch_pending_movement(app.world_mut());

        let target = app.world().entity(first).get::<MoveTarget>().unwrap().0;
        assert!(
            target.distance(Vec2::new(48.0, 48.0)) < 2.0,
            "the surviving member's live Unit target, not the removed first member's target, anchors the pool"
        );
        assert!(
            target.distance(Vec2::new(24.0, 24.0)) > 20.0,
            "the dead first member's stale target must not redirect the survivor"
        );
        assert!(
            app.world()
                .resource::<PendingMovementOrders>()
                .pending_member_counts
                .is_empty()
        );
        assert_eq!(
            app.world()
                .entity(first)
                .get::<UnitOrders>()
                .unwrap()
                .current,
            Some(survivor_order)
        );
    }

    #[test]
    fn target_gone_queued_hold_refreshes_reservation_before_survivor_spread() {
        let (mut app, first) = gatherer_app();
        let mobile_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        app.world_mut().entity_mut(first).insert((
            crate::Position(Vec2::new(8.0, 8.0)),
            MoveTarget(Vec2::new(40.0, 40.0)),
            UnitOrders {
                current: Some(Order {
                    order: ContentId::parse("test:follow-gone").unwrap(),
                    behaviour: Behaviour::Move,
                    target: OrderTargetSim::Unit(99),
                    group: 5,
                }),
                queue: VecDeque::from([Order {
                    order: ContentId::parse("test:hold").unwrap(),
                    behaviour: Behaviour::Hold,
                    target: OrderTargetSim::None,
                    group: 0,
                }]),
                last_failure: None,
            },
        ));
        let survivor = app
            .world_mut()
            .spawn((
                crate::UnitId(2),
                crate::UnitType(mobile_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(12.0, 8.0)),
                UnitOrders::default(),
            ))
            .id();
        let failed_order = Order {
            order: ContentId::parse("test:follow-gone").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Unit(99),
            group: 5,
        };
        let survivor_order = Order {
            order: ContentId::parse("test:move-point").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(8.0, 8.0)),
            group: 5,
        };
        let mut pending = PendingMovementOrders::default();
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1,
            group: 5,
            pool_id: 1,
            point: Vec2::new(8.0, 8.0),
            unit_ids: vec![1, 2],
            order: Some(failed_order.clone()),
            member_orders: BTreeMap::from([(1, failed_order), (2, survivor_order)]),
            plain: false,
        }));
        app.world_mut().insert_resource(pending);

        dispatch_pending_movement(app.world_mut());

        assert_eq!(
            app.world().entity(first).get::<MoveTarget>(),
            None,
            "TargetGone starts the queued Hold before the survivor is spread"
        );
        let allocated = app
            .world()
            .entity(survivor)
            .get::<crate::movement::MoveProgress>()
            .expect("the surviving group member receives a movement pool")
            .slot;
        assert!(
            allocated.distance(Vec2::new(8.0, 8.0)) >= 1.1 - 1.0e-4,
            "the survivor's destination respects the failed member's new resting position"
        );
    }

    #[test]
    fn queued_self_follow_waits_for_the_shared_cap_before_advancing_hold() {
        let (mut app, entity) = gatherer_app();
        let unit_id = 1;
        let move_id = ContentId::parse("test:move").unwrap();
        let hold_id = ContentId::parse("test:hold").unwrap();
        let missing_follow = Order {
            order: move_id.clone(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Unit(999),
            group: 0,
        };
        let queued_self_follow = Order {
            order: move_id.clone(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Unit(unit_id),
            group: 0,
        };
        let queued_hold = Order {
            order: hold_id,
            behaviour: Behaviour::Hold,
            target: OrderTargetSim::None,
            group: 0,
        };
        app.world_mut().entity_mut(entity).insert((
            UnitOrders {
                current: Some(missing_follow.clone()),
                queue: VecDeque::from([queued_self_follow.clone(), queued_hold.clone()]),
                last_failure: None,
            },
            MoveTarget(Vec2::new(16.0, 8.0)),
        ));
        let tick = app.world().resource::<crate::world::Tick>().0;
        let request = PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: tick,
            seq: 1,
            group: 1,
            pool_id: 1,
            point: Vec2::new(16.0, 8.0),
            unit_ids: vec![unit_id],
            order: Some(missing_follow.clone()),
            member_orders: BTreeMap::from([(unit_id, missing_follow)]),
            plain: false,
        };
        assert!(
            app.world_mut()
                .resource_mut::<PendingMovementOrders>()
                .queue(request)
        );
        *app.world_mut().resource_mut::<MovementDispatchState>() = MovementDispatchState {
            tick: Some(tick),
            start_slot: 0,
            used: 2048,
            cursor_advanced: true,
        };

        run_orders(app.world_mut());

        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert_eq!(orders.current.as_ref(), Some(&queued_self_follow));
        assert_eq!(orders.queue.front(), Some(&queued_hold));
        assert!(unit_has_pending_move(app.world(), unit_id));
        assert!(
            app.world()
                .resource::<PendingMovementOrders>()
                .queues
                .get(&0)
                .unwrap()
                .iter()
                .any(
                    |request| request.order_for_unit(unit_id).is_some_and(|order| {
                        order.behaviour == Behaviour::Move
                            && order.target == OrderTargetSim::Unit(unit_id)
                    })
                ),
            "the self-follow waits as an initial movement envelope"
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

                follow_origin: None,
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
    fn late_refused_replace_keeps_provisional_plain_move_and_append_suffix() {
        let (mut app, entity) = gatherer_app();
        let candidate = OrderCommand {
            order: ContentId::parse("test:move-candidate").unwrap(),
            behaviour: Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::Point,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        };
        let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 1, &candidate);
        assert_eq!(outcomes.accepted, vec![1]);
        let suffix = OrderCommand {
            order: ContentId::parse("test:move-suffix").unwrap(),
            behaviour: Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::Point,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Point(Vec2::new(45.0, 40.0)),
            queue: QueueMode::Append,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        };
        let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 2, &suffix);
        assert_eq!(outcomes.accepted, vec![1]);
        let before_orders = app
            .world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .clone();
        let before_target = app.world().entity(entity).get::<MoveTarget>().unwrap().0;
        assert_eq!(before_target, Vec2::new(40.0, 40.0));
        assert!(before_orders.current.is_none());
        assert_eq!(before_orders.queue.len(), 1);
        assert_eq!(
            before_orders.queue[0].order, suffix.order,
            "the accepted Append suffix remains queued behind the legacy-style candidate"
        );
        assert!(
            app.world()
                .resource::<PlainMoveReservations>()
                .entries
                .contains_key(&1)
        );
        let before_identity = app.world().resource::<PlainMoveOrderIdentities>().0[&1].clone();

        app.world_mut().resource_mut::<crate::AuditTrail>().enable();
        let rejected = OrderCommand {
            order: ContentId::parse("test:gather-incompatible").unwrap(),
            behaviour: Behaviour::Gather,
            target_kind: turts_mod_api::TargetKind::Node,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Node(NodeId(1)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        };
        let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 3, &rejected);
        assert_eq!(
            outcomes.refused,
            vec![(1, crate::command::RejectReason::IncompatibleItem)]
        );
        assert_eq!(
            app.world().entity(entity).get::<UnitOrders>().unwrap(),
            &before_orders
        );
        assert_eq!(
            app.world().entity(entity).get::<MoveTarget>().unwrap().0,
            before_target
        );
        assert!(
            app.world()
                .resource::<PlainMoveReservations>()
                .entries
                .contains_key(&1)
        );
        assert_eq!(
            app.world().resource::<PlainMoveOrderIdentities>().0[&1],
            before_identity
        );
        let trail = app.world().resource::<crate::AuditTrail>();
        assert_eq!(trail.operations.len(), 1);
        assert_eq!(trail.operations[0].before, trail.operations[0].after);
    }

    #[test]
    fn accepted_legacy_move_to_cancels_a_modern_plain_candidate() {
        let (mut app, entity) = gatherer_app();
        let candidate = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:modern-move").unwrap(),
                behaviour: Behaviour::Move,
                target_kind: turts_mod_api::TargetKind::PointOrUnit,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        assert_eq!(
            crate::apply_command(app.world_mut(), &candidate)
                .units
                .accepted,
            vec![1]
        );
        assert!(
            app.world()
                .resource::<PlainMoveReservations>()
                .entries
                .contains_key(&1)
        );
        let active = Order {
            order: ContentId::parse("test:active-before-legacy").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(32.0, 32.0)),
            group: 5,
        };
        app.world_mut().entity_mut(entity).insert((
            UnitOrders {
                current: Some(active.clone()),
                queue: VecDeque::from([Order {
                    order: ContentId::parse("test:future-before-legacy").unwrap(),
                    behaviour: Behaviour::Hold,
                    target: OrderTargetSim::Point(Vec2::new(8.0, 8.0)),
                    group: 0,
                }]),
                last_failure: None,
            },
            crate::movement::MoveProgress {
                pool_id: 3,
                slot: Vec2::new(32.0, 32.0),
                activity: crate::movement::MovementActivity::Moving,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 0,
                best_distance: 1.0,
                claim_count: 0,
                follow_origin: None,
            },
            crate::movement::PhysicalEscape {
                target: Some(Vec2::new(9.0, 8.0)),
            },
        ));

        let rejected = crate::Command {
            slot: SlotId(1),
            seq: 2,
            kind: crate::CommandKind::MoveTo {
                unit_ids: vec![1],
                target: Vec2::new(44.0, 44.0),
            },
        };
        assert_eq!(
            crate::apply_command(app.world_mut(), &rejected).outcome,
            Err(crate::command::RejectReason::NotOwner)
        );
        assert!(
            app.world()
                .resource::<PlainMoveReservations>()
                .entries
                .contains_key(&1)
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current,
            Some(active)
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .queue
                .len(),
            1
        );
        assert!(
            app.world()
                .entity(entity)
                .get::<crate::movement::MoveProgress>()
                .is_some()
        );

        let accepted = crate::Command {
            slot: SlotId(0),
            seq: 3,
            kind: crate::CommandKind::MoveTo {
                unit_ids: vec![1],
                target: Vec2::new(44.0, 44.0),
            },
        };
        assert_eq!(
            crate::apply_command(app.world_mut(), &accepted).outcome,
            Ok(crate::Applied { job_id: None })
        );
        assert_eq!(
            app.world().entity(entity).get::<MoveTarget>().unwrap().0,
            Vec2::new(44.0, 44.0)
        );
        assert_eq!(
            app.world().entity(entity).get::<UnitOrders>().unwrap(),
            &UnitOrders::default()
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
                .get::<crate::movement::PhysicalEscape>()
                .and_then(|escape| escape.target),
            Some(Vec2::new(9.0, 8.0)),
            "legacy MoveTo replaces voluntary movement without erasing physical escape"
        );
        assert!(
            !app.world()
                .resource::<PlainMoveReservations>()
                .entries
                .contains_key(&1)
        );
        app.update();
        assert_eq!(
            app.world().entity(entity).get::<MoveTarget>().unwrap().0,
            Vec2::new(44.0, 44.0),
            "the replaced cohort progress cannot restore its older target"
        );
        assert!(
            !app.world()
                .resource::<PlainMoveOrderIdentities>()
                .0
                .contains_key(&1)
        );
    }

    #[test]
    fn refused_legacy_multi_move_to_preserves_gather_and_prior_orders() {
        let (mut app, first) = gatherer_app();
        let first_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        let second = app
            .world_mut()
            .spawn((
                crate::UnitId(2),
                crate::UnitType(first_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(10.0, 8.0)),
                UnitOrders::default(),
            ))
            .id();
        let current = Order {
            order: ContentId::parse("test:old-current").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(20.0, 20.0)),
            group: 1,
        };
        let queued = Order {
            order: ContentId::parse("test:old-queued").unwrap(),
            behaviour: Behaviour::Hold,
            target: OrderTargetSim::Point(Vec2::new(10.0, 8.0)),
            group: 0,
        };
        let gather = crate::economy::GatherOrder {
            node: Some(crate::NodeId(7)),
            deposit: Some(crate::NodeId(7)),
            receiver: None,
            stage: crate::economy::gather::GatherStage::Gathering,
            carry: crate::economy::gather::RateCarry::default(),
        };
        for entity in [first, second] {
            app.world_mut().entity_mut(entity).insert(UnitOrders {
                current: Some(current.clone()),
                queue: VecDeque::from([queued.clone()]),
                last_failure: None,
            });
        }
        app.world_mut()
            .resource_mut::<crate::economy::EconomyOrders>()
            .0
            .insert(1, gather.clone());
        app.world_mut()
            .resource_mut::<crate::economy::EconomyOrders>()
            .0
            .insert(2, gather.clone());
        let before_targets = [
            app.world().entity(first).get::<MoveTarget>().copied(),
            app.world().entity(second).get::<MoveTarget>().copied(),
        ];
        let mut full = PendingMovementOrders::default();
        assert!(full.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: 1,
            group: 0,
            pool_id: 0,
            point: Vec2::new(48.0, 48.0),
            unit_ids: (10_000..26_384).collect(),
            order: None,
            member_orders: BTreeMap::new(),
            plain: true,
        }));
        app.world_mut().insert_resource(full);

        let refused = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::MoveTo {
                unit_ids: vec![1, 2],
                target: Vec2::new(40.0, 40.0),
            },
        };
        let result = crate::apply_command(app.world_mut(), &refused);
        assert!(result.units.accepted.is_empty());
        assert_eq!(result.units.refused.len(), 2);
        for (index, (unit_id, entity)) in [(1, first), (2, second)].into_iter().enumerate() {
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .current,
                Some(current.clone())
            );
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .queue,
                VecDeque::from([queued.clone()])
            );
            assert_eq!(
                app.world().resource::<crate::economy::EconomyOrders>().0[&unit_id],
                gather
            );
            assert_eq!(
                app.world().entity(entity).get::<MoveTarget>().copied(),
                before_targets[index]
            );
        }
        assert_eq!(
            app.world()
                .resource::<PendingMovementOrders>()
                .pending_units[&0],
            16_384
        );
    }

    #[test]
    fn authored_plain_blocked_identity_is_acknowledged_while_legacy_multi_waits() {
        let (mut app, first) = gatherer_app();
        let first_type = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        let second = app
            .world_mut()
            .spawn((
                crate::UnitId(2),
                crate::UnitType(first_type),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(10.0, 8.0)),
                UnitOrders::default(),
            ))
            .id();
        let authored = Order {
            order: ContentId::parse("test:authored-old-move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(20.0, 20.0)),
            group: 0,
        };
        app.world_mut().entity_mut(first).insert((
            UnitOrders::default(),
            crate::movement::PlainMoveProgress {
                original_point: Vec2::new(20.0, 20.0),
                slot: Vec2::new(8.0, 8.0),
                pool_slots: Vec::new(),
                gather_goal: None,
                activity: crate::movement::MovementActivity::Blocked,
                stalled_ticks: 20,
                best_distance: Vec2::new(20.0, 20.0).distance(Vec2::new(8.0, 8.0)),
                claim_count: 3,
                failure_recorded: false,
            },
        ));
        app.world_mut().init_resource::<PlainMoveOrderIdentities>();
        app.world_mut()
            .resource_mut::<PlainMoveOrderIdentities>()
            .0
            .insert(1, authored.clone());
        let tick = app.world().resource::<crate::world::Tick>().0;
        *app.world_mut().resource_mut::<MovementDispatchState>() = MovementDispatchState {
            tick: Some(tick),
            start_slot: 0,
            used: 2047,
            cursor_advanced: true,
        };

        let replacement = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::MoveTo {
                unit_ids: vec![1, 2],
                target: Vec2::new(40.0, 40.0),
            },
        };
        let admitted = crate::apply_command(app.world_mut(), &replacement);
        assert_eq!(admitted.units.accepted, vec![1, 2]);
        assert_eq!(
            app.world().resource::<PlainMoveOrderIdentities>().0.get(&1),
            Some(&authored)
        );
        run_orders(app.world_mut());

        let orders = app.world().entity(first).get::<UnitOrders>().unwrap();
        assert_eq!(orders.current, None);
        let failure = orders
            .last_failure
            .as_ref()
            .expect("prior A is acknowledged");
        assert_eq!(failure.order, authored.order);
        assert_eq!(failure.reason, crate::command::RejectReason::Blocked);
        assert_eq!(failure.tick, tick);
        assert!(
            app.world()
                .entity(first)
                .get::<crate::movement::PlainMoveProgress>()
                .is_some_and(|progress| progress.failure_recorded)
        );
        assert!(unit_has_pending_move(app.world(), 1));
        assert!(unit_has_pending_move(app.world(), 2));
        assert!(
            !app.world()
                .resource::<PlainMoveOrderIdentities>()
                .0
                .contains_key(&1)
        );
        assert!(
            app.world()
                .entity(second)
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .is_none()
        );
    }

    #[test]
    fn admitted_legacy_multi_move_retains_current_until_dispatch_and_keeps_new_append() {
        let (mut app, first) = gatherer_app();
        app.world_mut().insert_resource(crate::world::MapBounds {
            size: Vec2::new(64.0, 64.0),
        });
        let kind = app
            .world()
            .entity(first)
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        let second = app
            .world_mut()
            .spawn((
                crate::UnitId(2),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(10.0, 8.0)),
                UnitOrders::default(),
            ))
            .id();
        let head = app
            .world_mut()
            .spawn((
                crate::UnitId(3),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(16.0, 8.0)),
                UnitOrders::default(),
            ))
            .id();
        let current = Order {
            order: ContentId::parse("test:prior-move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
            group: 4,
        };
        let gather = crate::economy::GatherOrder {
            node: Some(crate::NodeId(7)),
            deposit: Some(crate::NodeId(7)),
            receiver: None,
            stage: crate::economy::gather::GatherStage::Gathering,
            carry: crate::economy::gather::RateCarry::default(),
        };
        for (unit_id, entity) in [(1, first), (2, second)] {
            app.world_mut().entity_mut(entity).insert((
                UnitOrders {
                    current: Some(current.clone()),
                    queue: VecDeque::from([Order {
                        order: ContentId::parse("test:old-future").unwrap(),
                        behaviour: Behaviour::Hold,
                        target: OrderTargetSim::None,
                        group: 0,
                    }]),
                    last_failure: None,
                },
                MoveTarget(Vec2::new(30.0, 30.0)),
            ));
            app.world_mut()
                .resource_mut::<crate::economy::EconomyOrders>()
                .0
                .insert(unit_id, gather.clone());
        }
        let tick = app.world().resource::<crate::world::Tick>().0;
        let head_order = Order {
            order: ContentId::parse("test:head-move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(50.0, 50.0)),
            group: 7,
        };
        assert!(
            app.world_mut()
                .resource_mut::<PendingMovementOrders>()
                .queue(PendingMovementOrder {
                    slot: SlotId(0),
                    accepted_tick: tick,
                    seq: 1,
                    group: 7,
                    pool_id: 0,
                    point: Vec2::new(50.0, 50.0),
                    unit_ids: vec![3],
                    order: Some(head_order.clone()),
                    member_orders: BTreeMap::from([(3, head_order)]),
                    plain: false,
                })
        );
        *app.world_mut().resource_mut::<MovementDispatchState>() = MovementDispatchState {
            tick: Some(tick),
            start_slot: 0,
            used: 2047,
            cursor_advanced: true,
        };

        let replace = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::MoveTo {
                unit_ids: vec![1, 2],
                target: Vec2::new(40.0, 40.0),
            },
        };
        let result = crate::apply_command(app.world_mut(), &replace);
        assert_eq!(result.units.accepted, vec![1, 2]);
        for (unit_id, entity) in [(1, first), (2, second)] {
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .current,
                Some(current.clone())
            );
            assert!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .queue
                    .is_empty()
            );
            assert_eq!(
                app.world().resource::<crate::economy::EconomyOrders>().0[&unit_id].stage,
                crate::economy::gather::GatherStage::Gathering
            );
        }
        let append = crate::Command {
            slot: SlotId(0),
            seq: 3,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:append-hold").unwrap(),
                behaviour: Behaviour::Hold,
                target_kind: turts_mod_api::TargetKind::None,
                policy: None,
                units: vec![1, 2],
                target: OrderTargetSim::None,
                queue: QueueMode::Append,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        assert_eq!(
            crate::apply_command(app.world_mut(), &append)
                .units
                .accepted,
            vec![1, 2]
        );

        dispatch_pending_movement(app.world_mut());
        assert_eq!(
            app.world()
                .resource::<PendingMovementOrders>()
                .pending_units[&0],
            2
        );
        for entity in [first, second] {
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .current,
                Some(current.clone())
            );
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .queue
                    .len(),
                1
            );
        }
        app.world_mut().resource_mut::<crate::world::Tick>().0 = tick + 1;
        dispatch_pending_movement(app.world_mut());
        for (unit_id, entity) in [(1, first), (2, second)] {
            let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
            assert_eq!(orders.current, None);
            assert_eq!(orders.queue.len(), 1, "only post-Replace Append survives");
            assert!(app.world().entity(entity).get::<MoveTarget>().is_some());
            assert!(
                !app.world()
                    .resource::<PendingMovementOrders>()
                    .pending_member_counts
                    .contains_key(&unit_id)
            );
            assert_eq!(
                app.world().resource::<crate::economy::EconomyOrders>().0[&unit_id].stage,
                crate::economy::gather::GatherStage::Idle
            );
        }
        assert!(app.world().entity(head).get::<MoveTarget>().is_some());
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
