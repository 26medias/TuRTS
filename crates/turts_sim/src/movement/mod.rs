//! Movement integrator.

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Visibility facts used by the deterministic movement replay kernel.
/// Implementations must derive permissions from the trusted observation basis.
pub trait MovementVisibility {
    fn unit_permitted(&self, viewer: u8, unit: u32) -> bool;
}

impl MovementVisibility for crate::visibility::VisibilityBasis {
    fn unit_permitted(&self, viewer: u8, unit: u32) -> bool {
        self.unit_permitted(viewer, unit)
    }
}

mod arrival;
mod ecs;
mod exchange;
#[cfg(test)]
#[path = "tests/exchange_tests.rs"]
mod exchange_tests;
#[cfg(test)]
#[path = "tests/fog_arrival_tests.rs"]
mod fog_arrival_tests;
mod frame;
pub(crate) mod motion;
mod plain;
#[cfg(test)]
#[path = "tests/plain_movement_tests.rs"]
mod plain_movement_tests;
mod pool_index;
#[cfg(feature = "test-probes")]
pub mod probe;
pub(crate) mod recovery;
#[cfg(test)]
#[path = "tests/recovery_actual_tests.rs"]
mod recovery_actual_tests;
#[cfg(test)]
#[path = "tests/recovery_tests.rs"]
mod recovery_tests;
pub(crate) mod recovery_view;
mod route;
mod search;
mod spread;
#[cfg(test)]
#[path = "tests/spread_tests.rs"]
mod spread_tests;
pub(crate) mod sweep;
#[cfg(test)]
pub(crate) use frame::step_frame_with_sweep_budget;
pub(crate) use frame::{step_frame_with_motion_guide, step_frame_with_motion_guide_and_visibility};

pub(crate) use ecs::integrate_movement;
pub use frame::{
    MovementFrame, MovementFrameOutput, MovementFrameUnit, step_frame, step_frame_at_tick,
    step_frame_preserving_resting, step_frame_preserving_resting_at_tick,
    step_frame_preserving_resting_at_tick_with_visibility,
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

/// One blocked-recovery escape job (design 2026-11-18-bounded-recovery). It is a body
/// component in the audited state, so replay rebuilds the same schedule and slices.
/// `pool` names the order it serves; a job of another order is dropped.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RecoverySearch {
    /// The search this job serves; a job of another kind or pool is dropped.
    pub kind: RecoveryKind,
    /// Ticks since the job started. Service is tick-start overlap first, then oldest
    /// first, then by physical key.
    pub age: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan: Option<RecoveryScan>,
}

/// The escape search that a recovery job continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryKind {
    /// The blocked group escape of the pool with this id.
    Group(u64),
    /// The blocked plain escape.
    Plain,
    /// The body escape after a failed order (`PhysicalEscape`).
    Physical,
}

/// The cursor of an unfinished escape search and the proof data of its examined region
/// (designs `2026-11-18-bounded-recovery` and `2026-11-19-recovery-actual-work`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RecoveryScan {
    pub origin: Vec2,
    pub radius: f32,
    pub ring: i32,
    pub side: u8,
    pub step: i32,
    pub best: Option<Vec2>,
    /// The largest distance from `origin` of an examined or started candidate or slot.
    pub examined: f32,
    /// Count and two wrapping sums of the known facts in the examined region.
    pub digest: [u64; 3],
    pub invalidations: u8,
    pub relaxed: bool,
    /// The cell size bits of the owner view that the digest covers.
    #[serde(default)]
    pub cell: u32,
    /// The search stage.
    #[serde(default)]
    pub stage: RecoveryStage,
    /// The pool slot search of a group escape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool: Option<RecoveryPoolCursor>,
    /// The point query that a slice paused inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<RecoveryQuery>,
    /// True after the first pause: a relaxed result must then be checked against the
    /// current state.
    #[serde(default)]
    pub spanned: bool,
}

/// The stage of an escape search.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum RecoveryStage {
    /// The pool slot search of a group escape (skipped by other searches).
    #[default]
    Pool,
    /// The hex ring scan.
    Ring,
    /// A relaxed search checks its chosen point against the current state.
    Verify { point: Vec2, slot: bool },
}

/// The cursor of a pool slot search: the reach level, the position inside the level
/// (a cell index of the level box, or a slot index), and the best free slot so far.
/// Before `checked`, `at` and `item` are the position of the local precheck, which runs
/// when `local` is set (its constant-time part found no slot).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RecoveryPoolCursor {
    pub level: u8,
    pub at: u32,
    pub item: u32,
    pub best: Option<(u32, Vec2)>,
    pub checked: bool,
    #[serde(default)]
    pub local: bool,
    pub done: bool,
}

/// The cursor of a metered point query: the cell row and column of the owner view, the
/// last body key done in that cell, and the next contact. `digest` is the proof of the
/// query region of a relaxed search.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RecoveryQuery {
    pub point: Vec2,
    pub stage: u8,
    pub cy: i32,
    pub cx: i32,
    pub key: Option<[u32; 3]>,
    pub contact: u32,
    pub started: bool,
    pub digest: [u64; 3],
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
