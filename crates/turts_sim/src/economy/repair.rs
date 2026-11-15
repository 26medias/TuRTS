//! Repair state and exact paid healing, after work payment and before combat.

use crate::RejectReason;
use bevy_ecs::{prelude::Resource, world::World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use turts_mod_api::ContentId;

/// The owner's current repair action. Detailed refusal causes remain audit-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairStatus {
    Approaching,
    Working,
    WaitingForFunds(ContentId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRow {
    pub repairer: u32,
    pub target: u32,
    pub owner_slot: u8,
    pub state: RepairStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairAction {
    pub target: u32,
    pub automatic: bool,
    pub elapsed: u64,
    pub state: RepairStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairHold {
    pub reason: RejectReason,
    pub since_tick: u32,
}

/// One source row is staged with its balance and target-health writes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RepairUnitState {
    pub(crate) action: Option<RepairAction>,
    pub(crate) remainders: BTreeMap<ContentId, u64>,
    pub(crate) hold: Option<RepairHold>,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RepairRuntime {
    pub(crate) units: BTreeMap<u32, RepairUnitState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRemainder {
    pub resource: ContentId,
    pub remainder: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairAuditRow {
    pub repairer: u32,
    pub action: Option<RepairAction>,
    pub remainders: Vec<RepairRemainder>,
    pub hold: Option<RepairHold>,
}

/// Status from order approach. The common order runner owns completion and queue advancement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepairOrderProgress {
    Active,
    Complete,
    Failed(RejectReason),
}

pub(crate) fn run_repair(_world: &mut World) {}
