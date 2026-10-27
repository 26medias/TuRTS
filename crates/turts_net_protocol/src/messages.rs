//! Wire messages.
//!
//! This crate holds only serde types. It has no Bevy types or Bevy entity ids.

use serde::{Deserialize, Serialize};

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
    pub kind: PlayerCommandKind,
}
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
        recipe_id: String,
    },
    CancelProduction {
        producer_id: UnitId,
        job_id: JobId,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandAck {
    pub seq: u32,
    pub tick: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandApplied {
    pub seq: u32,
    pub tick: u32,
    pub job_id: Option<JobId>,
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
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRejected {
    pub seq: u32,
    pub tick: u32,
    pub reason: CommandRejectReason,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    pub units: Vec<UnitSnapshot>,
    pub nodes: Vec<NodeState>,
    pub economy: EconomyState,
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
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResyncRequest;
