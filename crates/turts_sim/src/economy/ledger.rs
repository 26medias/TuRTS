use crate::ids::JobId;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::ContentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Endpoint {
    Slot(u8),
    Inventory(u32),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReservationLine {
    pub endpoint: Endpoint,
    pub resource: ContentId,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Reservation {
    pub job_id: JobId,
    pub lines: Vec<ReservationLine>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub on_hand: u64,
    pub reserved: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Inventory {
    pub(crate) capacity: u64,
    pub(crate) accepted_items: BTreeSet<ContentId>,
    pub(crate) items: BTreeMap<ContentId, Account>,
}
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct EconomyLedger {
    pub(crate) balances: BTreeMap<(u8, ContentId), Account>,
    pub(crate) inventories: BTreeMap<u32, Inventory>,
    // Authoritative claims; queue Reservation values are associations/snapshots, never authority.
    // Closed tombstones prevent stale token replay and JobId reuse.
    pub(crate) reservation_claims: BTreeMap<JobId, ReservationClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Private transaction claims are populated by crate-internal operations.
pub(crate) enum ReservationClaim {
    Active(Reservation),
    Closed,
}
