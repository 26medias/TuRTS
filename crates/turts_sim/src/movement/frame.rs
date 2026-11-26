//! The input and output rows of the movement step.

use bevy_math::Vec2;
use std::collections::BTreeMap;

use super::plain::PlainMoveProgress;
use super::{MoveProgress, MovementPool, PhysicalEscape};

/// One immutable unit row consumed by deterministic group movement.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrameUnit {
    pub unit_id: u32,
    pub owner_slot: u8,
    pub unit_type: u32,
    pub position: Vec2,
    pub radius: f32,
    pub speed: Option<f32>,
    pub target: Option<Vec2>,
    pub progress: Option<MoveProgress>,
    pub plain_progress: Option<PlainMoveProgress>,
    pub physical_escape: Option<PhysicalEscape>,
}

/// All movement-relevant state for one fixed tick. Both the sim and log validator use this input.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrame {
    pub bounds: Vec2,
    pub tick_hz: u16,
    pub units: Vec<MovementFrameUnit>,
    pub pools: BTreeMap<u64, MovementPool>,
    /// Blocked-recovery jobs by unit id. Identity lookup only; never an order of service.
    pub recovery: BTreeMap<u32, super::RecoverySearch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrameOutput {
    pub unit_id: u32,
    pub position: Vec2,
    pub target: Option<Vec2>,
    pub progress: Option<MoveProgress>,
    pub plain_progress: Option<PlainMoveProgress>,
    pub physical_escape: Option<PhysicalEscape>,
    pub failed_blocked: bool,
    /// Explicit physical tie resolution from this solver step. Not serialized.
    pub(crate) arrival_slot: Option<Vec2>,
    /// The blocked-recovery job after this tick.
    pub recovery: Option<Box<super::RecoverySearch>>,
}
