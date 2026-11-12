//! The records of the economy log. The log holds the ledger view of each snapshot (`LedgerAudit`,
//! `turts_sim/src/economy/ledger_view.rs`), not the full `AuditState`: a new sim structure enters
//! the log only through an edit of `LedgerAudit`.

use serde::{Deserialize, Serialize};
use turts_net_protocol::CommandRejectReason;
use turts_sim::economy::transaction::AuditPhase;
use turts_sim::{
    AuditState, CommandAudit, EconomyEvent, LedgerAudit, OperationAudit, TickAudit, ledger_audit,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "record", content = "data", rename_all = "snake_case")]
pub enum EconomyLogRecord {
    Initial(LedgerAudit),
    CommandBoundary(CommandLog),
    Operation(OperationLog),
    Tick {
        tick: u32,
        before: LedgerAudit,
        state: LedgerAudit,
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
        before: LedgerAudit,
        after: LedgerAudit,
    },
}

/// A `CommandAudit` with the ledger view of its two snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandLog {
    pub operation_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_end_id: Option<u64>,
    pub input: turts_sim::Command,
    pub result: turts_sim::CommandResult,
    pub before: LedgerAudit,
    pub after: LedgerAudit,
}

/// An `OperationAudit` with the ledger view of its two snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationLog {
    pub operation_id: u64,
    #[serde(default)]
    pub phase: AuditPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<u32>,
    pub before: LedgerAudit,
    pub after: LedgerAudit,
    pub events: Vec<EconomyEvent>,
}

impl From<CommandAudit> for CommandLog {
    fn from(audit: CommandAudit) -> Self {
        Self {
            operation_id: audit.operation_id,
            operation_end_id: audit.operation_end_id,
            input: audit.input,
            result: audit.result,
            before: ledger_audit(&audit.before),
            after: ledger_audit(&audit.after),
        }
    }
}

impl From<OperationAudit> for OperationLog {
    fn from(audit: OperationAudit) -> Self {
        Self {
            operation_id: audit.operation_id,
            phase: audit.phase,
            unit_id: audit.unit_id,
            before: ledger_audit(&audit.before),
            after: ledger_audit(&audit.after),
            events: audit.events,
        }
    }
}

impl EconomyLogRecord {
    pub fn initial(state: &AuditState) -> Self {
        Self::Initial(ledger_audit(state))
    }

    pub fn command(audit: CommandAudit) -> Self {
        Self::CommandBoundary(audit.into())
    }

    pub fn operation(audit: OperationAudit) -> Self {
        Self::Operation(audit.into())
    }

    pub fn from_tick(audit: TickAudit) -> Self {
        Self::Tick {
            tick: audit.tick,
            before: ledger_audit(&audit.before),
            state: ledger_audit(&audit.after),
            events: audit.events,
        }
    }
}
