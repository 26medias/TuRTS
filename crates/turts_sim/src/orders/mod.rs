//! Per-unit orders and the fixed-tick order phase.

mod apply;
mod behaviours;
mod pending;
mod plain;
mod run;
#[cfg(test)]
mod tests;

pub(crate) use apply::{apply_order, clear_replaced_movement, unit_order_capable};
pub use pending::{MovementDispatchState, PendingMovementOrders};
pub(crate) use pending::{
    prepare_legacy_move_to_waiters, queue_legacy_move_to_envelope, reset_legacy_move_to_unit,
};
pub use plain::PlainMoveOrderIdentities;
pub(crate) use plain::{
    forget_plain_move_identity, plain_move_has_no_prior_order_state, plain_move_reservation_exists,
};
pub(crate) use run::run_orders;

use crate::{JobId, NodeId, SlotId};
use bevy_ecs::prelude::Component;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use turts_mod_api::{Behaviour, ContentId};

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
