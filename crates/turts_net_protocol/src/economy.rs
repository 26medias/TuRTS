//! Protocol 2 economy and stable identifier records.

use crate::messages::{Pos, UnitId};
use serde::{Deserialize, Serialize};
use turts_mod_api::Cost;

/// A stable identifier for a resource node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// A stable identifier for a production job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

/// A content identifier carried by the staged protocol.
pub type ContentRef = String;

/// A surface resource node visible in a replicated world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeState {
    pub id: NodeId,
    pub content_id: ContentRef,
    pub pos: Pos,
    pub remaining: u64,
}

/// A resource account, including its reserved amount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceState {
    pub resource: ContentRef,
    pub on_hand: u64,
    pub reserved: u64,
}

/// The contents of one unit inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryState {
    pub unit: UnitId,
    pub items: Vec<BalanceState>,
}

/// A gather order and its current stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderState {
    pub unit: UnitId,
    pub node: Option<NodeId>,
    pub receiver: Option<UnitId>,
    pub stage: GatherStage,
}

/// A producer's complete queued jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueState {
    pub producer: UnitId,
    pub jobs: Vec<JobState>,
}

/// Production progress and its committed and reserved costs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobState {
    pub id: JobId,
    pub recipe: ContentRef,
    pub stage: JobStage,
    pub elapsed_ticks: u64,
    pub duration_ticks: u64,
    pub charged: Vec<Cost>,
    pub reserved: Vec<Cost>,
}

/// The current stage of a production job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStage {
    Queued,
    Running,
    OutputBlocked,
}

/// The current stage of a gather order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatherStage {
    ToNode,
    Gathering,
    ToDropOff,
    Unloading,
    Idle,
    Waiting { reason: GatherWaitReason },
}

/// A stable status explaining why a gather order waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatherWaitReason {
    NoDropOff,
    DestinationFull,
    DestinationOverflow,
    NoDeposit,
    Depleted,
}

impl GatherWaitReason {
    /// Every stable gather waiting status.
    pub const ALL: &'static [Self] = &[
        Self::NoDropOff,
        Self::DestinationFull,
        Self::DestinationOverflow,
        Self::NoDeposit,
        Self::Depleted,
    ];

    /// The stable snake_case UI and debug key.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoDropOff => "no_drop_off",
            Self::DestinationFull => "destination_full",
            Self::DestinationOverflow => "destination_overflow",
            Self::NoDeposit => "no_deposit",
            Self::Depleted => "depleted",
        }
    }
}

/// The complete player-visible economy state in a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyState {
    pub balances: Vec<BalanceState>,
    pub inventories: Vec<InventoryState>,
    pub orders: Vec<OrderState>,
    pub queues: Vec<QueueState>,
}

/// Absolute replacements for economy records changed in a world delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyChanges {
    pub balances: Vec<BalanceState>,
    pub inventories: Vec<InventoryState>,
    pub orders: Vec<OrderState>,
    pub queues: Vec<QueueState>,
}
