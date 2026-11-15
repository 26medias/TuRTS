use serde::{Deserialize, Serialize};
use turts_mod_api::ContentId;
use turts_net_protocol::{CommandRejectReason, OrderResult, PlayerCommand};
use turts_sim::{AuditState, CommandAudit, EconomyEvent, OperationAudit, TickAudit};

/// Trusted origin. A bot submission is never replayed as a human input.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ActorLogIdentity {
    Human { player_id: u64, slot: u8 },
    Bot { profile: ContentId, slot: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorPhase {
    Waiting,
    Running,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorTask {
    pub controller: String,
    pub goal: String,
    pub instance: String,
}

/// These records supplement the ordinary command and state evidence.
/// Lifecycle inputs precede decision guards. Each submission/result pair shares
/// its admission tick and deterministic per-tick ordinal, including refusals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ActorLogRecord {
    Lifecycle {
        tick: u32,
        humans_present: bool,
        phase: ActorPhase,
    },
    Submission {
        tick: u32,
        ordinal: u64,
        actor: ActorLogIdentity,
        sequence: u32,
        task: Option<ActorTask>,
        envelope: PlayerCommand,
    },
    Result {
        tick: u32,
        ordinal: u64,
        actor: ActorLogIdentity,
        sequence: u32,
        result: OrderResult,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "record", content = "data", rename_all = "snake_case")]
pub enum EconomyLogRecord {
    Actor(ActorLogRecord),
    Initial(AuditState),
    CommandBoundary(CommandAudit),
    Operation(OperationAudit),
    Tick {
        tick: u32,
        before: AuditState,
        state: AuditState,
        events: Vec<EconomyEvent>,
    },
    /// An accepted Hello claim, independently applied to the prior model's slot vector.
    Claim {
        tick: u32,
        player_id: u64,
        slot: u8,
    },
    /// A wire-only refusal; this is deliberately not represented as a typed command audit.
    Admission {
        ordinal: u64,
        connection: u64,
        player_id: u64,
        slot: u8,
        seq: u32,
        tick: u32,
        reason: CommandRejectReason,
        wire_format: String,
        raw_envelope: Vec<u8>,
        before: AuditState,
        after: AuditState,
    },
}

impl EconomyLogRecord {
    #[allow(dead_code)]
    pub fn from_tick(audit: TickAudit) -> Self {
        Self::Tick {
            tick: audit.tick,
            before: audit.before,
            state: audit.after,
            events: audit.events,
        }
    }
}
