//! Movement integrator.

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod ecs;
mod fallback;
mod frame;
mod plain;
#[cfg(test)]
#[path = "tests/plain_movement_tests.rs"]
mod plain_movement_tests;
mod pool_index;
#[cfg(feature = "test-probes")]
pub mod probe;
#[cfg(feature = "test-probes")]
pub mod reference;
mod search;
mod spread;
#[cfg(test)]
#[path = "tests/spread_tests.rs"]
mod spread_tests;

pub(crate) use ecs::integrate_movement;
pub use fallback::{FallbackRequest, assign_fallback_pools};
pub use frame::{
    MovementFrame, MovementFrameOutput, MovementFrameUnit, MovementStep, step_frame,
    step_frame_preserving_resting, step_frame_with_fallbacks,
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

#[cfg(test)]
thread_local! {
    /// The pool indexes that the step built for pools of one (one primary slot or none), in the
    /// frame setup or on demand (task A2b1, review finding H1).
    static POOL_OF_ONE_INDEX_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn count_pool_index_build(primary_count: u32) {
    if primary_count <= 1 {
        POOL_OF_ONE_INDEX_BUILDS.with(|builds| builds.set(builds.get() + 1));
    }
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
    /// The gather leg that a fallback pool serves (pre-MP1 slice, task A2b1; design 9.19). The
    /// gather writes it (`economy/gather.rs`, `set_target`), and the order phase clears the pool
    /// when the gather is reached or its endpoint changes (`orders/run.rs`). The movement step
    /// does not collect a fallback pool while it is `Some`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_goal: Option<PlainGatherGoal>,
    /// `Some` for a pool of one that the movement step gives to a trip with no pool when its goal
    /// is occupied or it is pushed off its line (the fallback), and for the `Blocked` pool of one
    /// that a failed legacy envelope writes (task A2b1). It replaces the plain progress of the
    /// old movement model. `None` for the pool of an order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<MoveFallback>,
}

/// The state that only a fallback pool has (task A2b1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveFallback {
    /// The order phase recorded the `Blocked` result of this trip one time
    /// (`orders/run.rs`), so it does not record it again. A `Blocked` fallback pool keeps
    /// `Blocked` until a new order or a new trip (design 9.17).
    pub failure_recorded: bool,
}

impl MoveProgress {
    /// The pool is a fallback pool (see `MoveProgress::fallback`).
    pub fn is_fallback(&self) -> bool {
        self.fallback.is_some()
    }
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
