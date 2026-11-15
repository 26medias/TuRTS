use serde::{Deserialize, Serialize};
use turts_mod_api::ContentId;
use turts_net_protocol::{CommandRejectReason, OrderResult, PlayerCommand};
use turts_sim::{AuditState, CommandAudit, EconomyEvent, OperationAudit, TickAudit};

/// Trusted origin. A bot submission is never replayed as a human input.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ActorLogIdentity {
    Human {
        player_id: u64,
        slot: u8,
        connection: u64,
    },
    Bot {
        profile: ContentId,
        slot: u8,
    },
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

/// Reproducible host limits. These are artifact values, not platform-sized counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorOperatorLimits {
    pub queries: u32,
    pub candidates: u32,
    pub rows: u32,
    pub proposals: u32,
    pub commands: u32,
    pub affected_units: u32,
    pub pending_requests: u32,
    pub pending_accounts: u32,
    pub projection_rows: u64,
    pub lease_steps: u32,
    pub page_rows: u32,
    pub cursor_records: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBotInstallation {
    pub slot: u8,
    pub profile: ContentId,
    pub seed: u64,
    /// Lowercase SHA256 of serde_json::to_vec of the resolved BotDef.
    pub profile_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorInstallation {
    /// The existing accepted lockfile hash. Its encoding is unchanged.
    pub locked_package_hash: String,
    pub engine_api: String,
    pub protocol: u32,
    pub bots: Vec<ActorBotInstallation>,
    pub limits: ActorOperatorLimits,
}

/// These records supplement the ordinary command and state evidence.
/// Lifecycle inputs precede decision guards. Each submission/result pair shares
/// its admission tick and deterministic per-tick ordinal, including refusals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ActorLogRecord {
    Installation(ActorInstallation),
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

/// Hash the validated resolved profile with the encoding used by trusted replay.
pub fn actor_profile_hash(profile: &turts_mod_api::BotDef) -> Result<String, serde_json::Error> {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(profile)?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}
