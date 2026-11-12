//! Movement integrator.

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod ecs;
mod frame;
mod plain;
#[cfg(test)]
#[path = "tests/plain_movement_tests.rs"]
mod plain_movement_tests;
mod pool_index;
#[cfg(feature = "test-probes")]
pub mod probe;
mod search;
mod spread;
#[cfg(test)]
#[path = "tests/spread_tests.rs"]
mod spread_tests;

pub(crate) use ecs::integrate_movement;
pub use frame::{
    MovementFrame, MovementFrameOutput, MovementFrameUnit, step_frame,
    step_frame_preserving_resting,
};
pub use plain::PlainMoveProgress;
pub use spread::{SpreadContext, spread_destinations, spread_destinations_with_context};

#[cfg(test)]
use pool_index::{ClearanceRank, MovementPoolIndex, SlotClearance};

#[cfg(test)]
thread_local! {
    static POOL_SLOT_CANDIDATE_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn count_pool_slot_candidates(count: usize) {
    POOL_SLOT_CANDIDATE_CHECKS.with(|checks| checks.set(checks.get() + count));
}

/// Current lifecycle of one accepted group-movement envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementActivity {
    Pending,
    Moving,
    Resting,
    Blocked,
}

/// Per-unit deterministic movement state needed by arrival and replay.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveProgress {
    pub pool_id: u64,
    pub slot: Vec2,
    pub activity: MovementActivity,
    pub pass_mode: bool,
    pub stalling_on_group: bool,
    pub stalled_ticks: u16,
    pub best_distance: f32,
    pub claim_count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_origin: Option<FollowMoveOrigin>,
}

/// The live Unit target and per-actor legal goal used for initial spread.
/// This keeps a shared max-radius pool anchor from looking like target motion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FollowMoveOrigin {
    pub target_unit_id: u32,
    pub goal: Vec2,
}

/// Physical recovery that continues after an order has already failed.
/// Unlike movement progress, this state belongs to the unit body, not its order.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalEscape {
    pub target: Option<Vec2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlainGatherGoal {
    Node(u32),
    Receiver(u32),
}

/// One group's primary destinations and spare destinations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovementPool {
    pub pool_id: u64,
    pub point: Vec2,
    pub spacing: f32,
    pub primary_count: u32,
    pub slots: Vec<Vec2>,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MovementPools(pub BTreeMap<u64, MovementPool>);

#[derive(Debug, Clone, Copy)]
pub struct SpreadMember {
    pub id: u32,
    pub position: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct StaticFootprint {
    pub position: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ReservedFootprint {
    pub position: Vec2,
    pub radius: f32,
}

fn angle(point: Vec2, origin: Vec2) -> f32 {
    (point.y - origin.y).atan2(point.x - origin.x)
}

/// The complete input to one unit's deterministic flat-ground movement step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementInput {
    pub position: bevy_math::Vec2,
    pub target: bevy_math::Vec2,
    pub speed: f32,
}

/// The result of one unit's deterministic movement step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementOutput {
    pub position: bevy_math::Vec2,
    pub reached: bool,
}

/// Move one unit toward its target using the shared simulation and validator step.
pub fn step(input: &MovementInput, dt: f32) -> MovementOutput {
    let step = input.speed * dt;
    let offset = input.target - input.position;
    let distance = offset.length();
    if distance <= step {
        MovementOutput {
            position: input.target,
            reached: true,
        }
    } else {
        MovementOutput {
            position: input.position + offset * (step / distance),
            reached: false,
        }
    }
}
