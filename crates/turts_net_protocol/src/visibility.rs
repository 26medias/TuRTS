//! Protocol9 atomic permitted-state transactions. No transport implementation.
//!
//! Application chunks respect the current transport's smaller frame allowance.
//! Consumers must validate stamps, sizes, complete payloads, and references before
//! atomically activating a new state. These declarations do not perform assembly.
use serde::{Deserialize, Serialize};
use turts_mod_api::{KnowledgeBasis, KnowledgeDelta, KnowledgeSnapshot};

use crate::{Snapshot, UnitSnapshot, WorldDelta};

/// Maximum complete encoded chunk, including its stamp and index.
pub const MAX_STATE_CHUNK_BYTES: usize = 1_024;
pub const MAX_PENDING_STATE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_POST_BASE_PATCH_BYTES: u64 = 8 * 1024 * 1024;
pub const STATE_TRANSACTION_TIMEOUT_MS: u64 = 5_000;
pub const STATE_RESTART_WINDOW_MS: u64 = 30_000;
pub const MAX_STATE_RESTARTS: u32 = 3;

/// Connection-local ordering does not grant a slot or a gameplay permission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateTransactionStamp {
    pub basis: KnowledgeBasis,
    pub transaction_id: u64,
    /// None installs a complete snapshot; patches require their active parent.
    pub parent_transaction_id: Option<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateTransactionKind {
    Snapshot,
    Patch,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateTransactionBegin {
    pub stamp: StateTransactionStamp,
    pub kind: StateTransactionKind,
    pub encoded_bytes: u64,
    pub chunk_count: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateTransactionChunk {
    pub stamp: StateTransactionStamp,
    pub index: u32,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateTransactionEnd {
    pub stamp: StateTransactionStamp,
}
/// Sent only after complete validation and atomic client activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateTransactionAck {
    pub stamp: StateTransactionStamp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateTransactionCancelReason {
    Superseded,
    BufferLimit,
    TimedOut,
    InvalidTransaction,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateTransactionCancel {
    pub stamp: StateTransactionStamp,
    pub reason: StateTransactionCancelReason,
}

/// An authorized event may refer to a subject that does not survive until the
/// completed snapshot. This row grants only event-local public presentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventUnitObservation {
    pub basis: KnowledgeBasis,
    pub unit: UnitSnapshot,
}

/// Serialized as one bounded transaction, then divided into chunks. Box avoids
/// embedding the largest world record in every pending enum value.
/// AllVisible retains None knowledge; existing Snapshot/WorldDelta shapes stay intact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum StateTransactionPayload {
    Snapshot {
        state: Box<Snapshot>,
        knowledge: Option<KnowledgeSnapshot>,
        event_units: Vec<EventUnitObservation>,
    },
    Patch {
        state: Box<WorldDelta>,
        knowledge: Option<KnowledgeDelta>,
        event_units: Vec<EventUnitObservation>,
    },
}
