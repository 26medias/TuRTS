//! Single-unit progress after a legacy move meets an obstacle.

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use super::{MovementActivity, PlainGatherGoal};

/// Default-omitted progress for a single legacy move that has encountered an obstacle.
/// The candidate pool belongs to this unit; it deliberately has no shared pool ID.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainMoveProgress {
    /// Temporary clearance for an approaching earlier arrival. Reconsider until it arrives.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub arrival_yield: bool,
    pub original_point: Vec2,
    pub slot: Vec2,
    pub pool_slots: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_goal: Option<PlainGatherGoal>,
    pub activity: MovementActivity,
    pub stalled_ticks: u16,
    pub best_distance: f32,
    pub claim_count: u8,
    pub failure_recorded: bool,
}
