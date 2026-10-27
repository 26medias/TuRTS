//! Protocol 2 wire types staged for review before protocol 1 is replaced.

use serde::{Deserialize, Serialize};
use turts_mod_api::Cost;

use crate::messages::{Pos, UnitDelta, UnitId, UnitSnapshot};

/// A stable identifier for a resource node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// A stable identifier for a production job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

/// A content identifier carried by the staged protocol.
pub type ContentRef = String;

/// One player command sent on the reliable ordered channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerCommand {
    pub seq: u32,
    pub kind: PlayerCommandKind,
}

/// The command requested by a player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerCommandKind {
    MoveTo {
        unit_ids: Vec<UnitId>,
        target: Pos,
    },
    Gather {
        unit_id: UnitId,
        node_id: NodeId,
    },
    Produce {
        producer_id: UnitId,
        recipe_id: ContentRef,
    },
    CancelProduction {
        producer_id: UnitId,
        job_id: JobId,
    },
}

/// The server applied a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandApplied {
    pub seq: u32,
    pub tick: u32,
    pub job_id: Option<JobId>,
}

/// A stable, closed set of command rejection reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRejectReason {
    EmptyCommand,
    TooManyUnits,
    DuplicateUnit,
    UnknownUnit,
    NotOwner,
    NotMobile,
    TargetNotFinite,
    TargetOutsideMap,
    MissingCapability,
    UnknownNode,
    IncompatibleItem,
    NodeDepleted,
    UnknownRecipe,
    RecipeNotEnabled,
    QueueFull,
    InsufficientBalance,
    InsufficientItems,
    UnknownJob,
    RefundDestinationFull,
    RefundDestinationOverflow,
    ArithmeticOverflow,
    IdExhausted,
    BadSequence,
}

impl CommandRejectReason {
    /// Every wire rejection reason in stable serialization order.
    pub const ALL: &'static [Self] = &[
        Self::EmptyCommand,
        Self::TooManyUnits,
        Self::DuplicateUnit,
        Self::UnknownUnit,
        Self::NotOwner,
        Self::NotMobile,
        Self::TargetNotFinite,
        Self::TargetOutsideMap,
        Self::MissingCapability,
        Self::UnknownNode,
        Self::IncompatibleItem,
        Self::NodeDepleted,
        Self::UnknownRecipe,
        Self::RecipeNotEnabled,
        Self::QueueFull,
        Self::InsufficientBalance,
        Self::InsufficientItems,
        Self::UnknownJob,
        Self::RefundDestinationFull,
        Self::RefundDestinationOverflow,
        Self::ArithmeticOverflow,
        Self::IdExhausted,
        Self::BadSequence,
    ];

    /// The stable snake_case UI and debug key.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyCommand => "empty_command",
            Self::TooManyUnits => "too_many_units",
            Self::DuplicateUnit => "duplicate_unit",
            Self::UnknownUnit => "unknown_unit",
            Self::NotOwner => "not_owner",
            Self::NotMobile => "not_mobile",
            Self::TargetNotFinite => "target_not_finite",
            Self::TargetOutsideMap => "target_outside_map",
            Self::MissingCapability => "missing_capability",
            Self::UnknownNode => "unknown_node",
            Self::IncompatibleItem => "incompatible_item",
            Self::NodeDepleted => "node_depleted",
            Self::UnknownRecipe => "unknown_recipe",
            Self::RecipeNotEnabled => "recipe_not_enabled",
            Self::QueueFull => "queue_full",
            Self::InsufficientBalance => "insufficient_balance",
            Self::InsufficientItems => "insufficient_items",
            Self::UnknownJob => "unknown_job",
            Self::RefundDestinationFull => "refund_destination_full",
            Self::RefundDestinationOverflow => "refund_destination_overflow",
            Self::ArithmeticOverflow => "arithmetic_overflow",
            Self::IdExhausted => "id_exhausted",
            Self::BadSequence => "bad_sequence",
        }
    }
}

/// The server rejected a command during tick validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRejected {
    pub seq: u32,
    pub tick: u32,
    pub reason: CommandRejectReason,
}

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

/// Reliable full world state from one completed tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    pub units: Vec<UnitSnapshot>,
    pub nodes: Vec<NodeState>,
    pub economy: EconomyState,
}

/// One atomic unreliable world update from `base_tick` to `tick`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldDelta {
    pub tick: u32,
    pub base_tick: u32,
    pub spawned: Vec<UnitSnapshot>,
    pub removed: Vec<UnitId>,
    pub positions: Vec<UnitDelta>,
    pub nodes: Vec<NodeState>,
    pub economy: EconomyChanges,
}
