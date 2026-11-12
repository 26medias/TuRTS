use serde::{Deserialize, Serialize};
use turts_net_protocol::CommandRejectReason;
use turts_sim::{AuditState, CommandAudit, EconomyEvent, OperationAudit, TickAudit};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "record", content = "data", rename_all = "snake_case")]
pub enum EconomyLogRecord {
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
