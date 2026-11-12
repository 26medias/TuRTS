//! Wire messages.
//!
//! This crate holds only serde types. It has no Bevy types or Bevy entity ids.

use serde::{Deserialize, Serialize};
pub use turts_mod_api::Facing;

pub use crate::economy::*;
pub use crate::records::*;

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
    /// The start of the match (design 6.7.13). `Welcome` carries it, so there is no message of
    /// its own.
    pub start: MatchStart,
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
    /// The unit is a site: a building under construction (design 6.8). Public. A site completes
    /// when its id is in `DeltaRecords::sites_completed`.
    pub site: bool,
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
    /// A placement (design 6.8, client to server): a building content at a point.
    ContentAtPoint {
        content: ContentRef,
        point: Pos,
    },
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
    /// The rally of a producer (design 6.5, 6.8). `None` until `L-RLY` writes it.
    pub rally: Option<RallyTarget>,
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
    SiteOutsideMap,
    SiteOutOfArea,
    SiteBlocked,
    SiteNearBuilding,
    SiteNearResource,
    ConstructionQueueFull,
    MatchNotStarted,
    SlotDefeated,
    MatchOver,
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
        Self::SiteOutsideMap,
        Self::SiteOutOfArea,
        Self::SiteBlocked,
        Self::SiteNearBuilding,
        Self::SiteNearResource,
        Self::ConstructionQueueFull,
        Self::MatchNotStarted,
        Self::SlotDefeated,
        Self::MatchOver,
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
            Self::SiteOutsideMap => "site_outside_map",
            Self::SiteOutOfArea => "site_out_of_area",
            Self::SiteBlocked => "site_blocked",
            Self::SiteNearBuilding => "site_near_building",
            Self::SiteNearResource => "site_near_resource",
            Self::ConstructionQueueFull => "construction_queue_full",
            Self::MatchNotStarted => "match_not_started",
            Self::SlotDefeated => "slot_defeated",
            Self::MatchOver => "match_over",
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
    /// The tick at which the weapon is ready to fire (design 6.8). It replaces `cooldown`, which
    /// changed in every tick. The ticks left are `turts_rules::progress::cooldown_left`. The
    /// server sends the record only when a field changes. It is 0 for a weapon that is ready
    /// (cooldown 0), so a Snapshot and a chain of deltas give the same record. The server sends
    /// `Cooling` (the status of a weapon that waits for `ready_tick`) and the other statuses as
    /// the sim has them; the client may derive cooling from `ready_tick` and the tick of the
    /// last applied delta.
    pub ready_tick: u32,
    pub damage: Result<u64, CommandRejectReason>,
    pub status: WeaponStatus,
    /// The last target. A record never names a unit that the same message removes, and the
    /// server resends a record that names a unit when that unit leaves (`export::weapon_state`).
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
    pub records: SnapshotRecords,
}
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
    /// The complete record of each unit whose record changed in this tick (design 6.8,
    /// `unit_orders` as changes). The `Snapshot` keeps the full table. The client merges deltas
    /// in chain order only.
    pub unit_orders: Vec<UnitOrderRecord>,
    pub records: DeltaRecords,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResyncRequest;
