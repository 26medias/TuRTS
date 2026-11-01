//! Wire messages.
//!
//! This crate holds only serde types. It has no Bevy types or Bevy entity ids.

use serde::{Deserialize, Serialize};
pub use turts_mod_api::Facing;

pub use crate::economy::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotId(pub u8);
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub player_id: u64,
    pub protocol_version: u32,
    pub lockfile_hash: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Welcome {
    pub slot_id: SlotId,
    pub tick: u32,
    pub tick_hz: u16,
    pub match_id: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinRejected {
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitSnapshot {
    pub unit_id: UnitId,
    pub content_id: String,
    pub owner_slot: SlotId,
    pub facing: Option<Facing>,
    pub pos: Pos,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitDelta {
    pub unit_id: UnitId,
    pub pos: Pos,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerCommand {
    pub seq: u32,
    pub order: OrderEnvelope,
}

/// The largest number of subject units in one order envelope (K4).
pub const MAX_ORDER_UNITS: usize = 4096;

/// One order request shared by all of its subject units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderEnvelope {
    pub order: ContentRef,
    pub units: Vec<UnitId>,
    pub target: OrderTarget,
    pub queue: QueueMode,
    pub group: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderTarget {
    None,
    Point(Pos),
    Unit(UnitId),
    Node(NodeId),
    Job(JobId),
    Content(ContentRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueMode {
    Replace,
    Append,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitRefusal {
    pub unit: UnitId,
    pub reason: CommandRejectReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderResult {
    pub seq: u32,
    pub tick: u32,
    pub accepted: Vec<UnitId>,
    pub refused: Vec<UnitRefusal>,
    pub envelope: Option<CommandRejectReason>,
    pub job_id: Option<JobId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    Idle,
    Pending,
    Moving,
    Attacking,
    Holding,
    Gathering,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderSummary {
    pub order: ContentRef,
    pub target: OrderTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderFailure {
    pub order: ContentRef,
    pub reason: CommandRejectReason,
    pub tick: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitOrderRecord {
    pub unit: UnitId,
    pub activity: Activity,
    pub current: Option<OrderSummary>,
    pub queued: Vec<OrderSummary>,
    pub last_failure: Option<OrderFailure>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandAck {
    pub seq: u32,
    pub tick: u32,
}
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
    NotRemovable,
    NotToggleable,
    PrerequisitesMissing,
    AlreadyResearched,
    ResearchInProgress,
    Internal,
    UnknownOrder,
    NotApplicable,
    TargetInvalid,
    TargetGone,
    OutOfRange,
    RateLimited,
    NotQueueable,
    Blocked,
}
impl CommandRejectReason {
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
        Self::NotRemovable,
        Self::NotToggleable,
        Self::PrerequisitesMissing,
        Self::AlreadyResearched,
        Self::ResearchInProgress,
        Self::Internal,
        Self::UnknownOrder,
        Self::NotApplicable,
        Self::TargetInvalid,
        Self::TargetGone,
        Self::OutOfRange,
        Self::RateLimited,
        Self::NotQueueable,
        Self::Blocked,
    ];
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
            Self::NotRemovable => "not_removable",
            Self::NotToggleable => "not_toggleable",
            Self::PrerequisitesMissing => "prerequisites_missing",
            Self::AlreadyResearched => "already_researched",
            Self::ResearchInProgress => "research_in_progress",
            Self::Internal => "internal",
            Self::UnknownOrder => "unknown_order",
            Self::NotApplicable => "not_applicable",
            Self::TargetInvalid => "target_invalid",
            Self::TargetGone => "target_gone",
            Self::OutOfRange => "out_of_range",
            Self::RateLimited => "rate_limited",
            Self::NotQueueable => "not_queueable",
            Self::Blocked => "blocked",
        }
    }
}

/// One item reference for each slot on one belt piece. Empty slots carry no index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeltPieceRecord {
    pub piece: UnitId,
    pub slots: Vec<Option<ContentRef>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InserterWaitReason {
    NoSource,
    NoDestination,
    SourceEmpty,
    DestinationFull,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InserterStage {
    AtSource(Option<InserterWaitReason>),
    ToDestination,
    AtDestination(InserterWaitReason),
    Returning,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InserterRecord {
    pub unit: UnitId,
    pub enabled: bool,
    pub stage: InserterStage,
    pub elapsed: u32,
    pub source: Option<UnitId>,
    pub destination: Option<UnitId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractorStatus {
    Extracting,
    Full,
    Depleted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractorRecord {
    pub unit: UnitId,
    pub enabled: bool,
    pub node: Option<NodeId>,
    pub status: ExtractorStatus,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponStatus {
    Ready,
    Cooling,
    NoTarget,
    OutOfAmmo,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponRecord {
    pub unit: UnitId,
    pub enabled: bool,
    pub cooldown: u32,
    pub damage: Result<u64, CommandRejectReason>,
    pub status: WeaponStatus,
    pub last_target: Option<UnitId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthRecord {
    pub unit: UnitId,
    pub current: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchRecord {
    pub slot: SlotId,
    pub completed: Vec<ContentRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    pub units: Vec<UnitSnapshot>,
    pub nodes: Vec<NodeState>,
    pub economy: EconomyState,
    pub belts: Vec<BeltPieceRecord>,
    pub inserters: Vec<InserterRecord>,
    pub extractors: Vec<ExtractorRecord>,
    pub weapons: Vec<WeaponRecord>,
    pub healths: Vec<HealthRecord>,
    pub research: Vec<ResearchRecord>,
    pub unit_orders: Vec<UnitOrderRecord>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldDelta {
    pub tick: u32,
    pub base_tick: u32,
    pub spawned: Vec<UnitSnapshot>,
    pub removed: Vec<UnitId>,
    pub positions: Vec<UnitDelta>,
    pub nodes: Vec<NodeState>,
    pub economy: EconomyChanges,
    pub belts: Vec<BeltPieceRecord>,
    pub inserters: Vec<InserterRecord>,
    pub extractors: Vec<ExtractorRecord>,
    pub weapons: Vec<WeaponRecord>,
    pub healths: Vec<HealthRecord>,
    pub research: Vec<ResearchRecord>,
    pub unit_orders: Vec<UnitOrderRecord>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResyncRequest;
