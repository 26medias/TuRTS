//! Public controller ports independent of simulation and rendering.
use crate::{query::BotQueries, types::*};
use turts_mod_api::{BotDef, BotQuota, ContentId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchPhase {
    Waiting,
    Running,
    Ended,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleCount {
    pub role: String,
    pub completed: u32,
    pub pending: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyObservation {
    pub resource: ContentId,
    pub provided: u64,
    pub consumed: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinatorLimits {
    pub commands: u32,
    pub affected_units: u32,
    pub pending_requests: u32,
    pub pending_accounts: u32,
}
impl Default for CoordinatorLimits {
    fn default() -> Self {
        Self {
            commands: 8,
            affected_units: 256,
            pending_requests: 64,
            pending_accounts: 512,
        }
    }
}
pub struct BotFrame<'a> {
    /// Candidate basis shared by every query in a retained decision.
    pub basis: BasisId,
    /// Fresh complete accounting basis, independent of a retained candidate lease.
    pub accounting_basis: BasisId,
    pub completed_tick: u32,
    pub scheduler_tick: u32,
    pub active_ticks: u64,
    pub actor: &'a BotActor,
    pub tick_hz: u16,
    pub phase: MatchPhase,
    pub profile: &'a BotDef,
    pub quota: BotQuota,
    pub limits: CoordinatorLimits,
    pub reconciliation: &'a ReconcileBatch,
    pub role_counts: &'a [RoleCount],
    pub supply: &'a [SupplyObservation],
}
/// Implementations own match-local state. No native preemption/sandbox is promised.
pub trait BotController: Send + Sync {
    fn step(
        &mut self,
        frame: &BotFrame<'_>,
        queries: &mut dyn BotQueries,
        out: &mut dyn ProposalSink,
    );
}
pub trait ProposalSink {
    fn propose(&mut self, proposal: Proposal) -> Result<(), ProposalRejection>;
}
