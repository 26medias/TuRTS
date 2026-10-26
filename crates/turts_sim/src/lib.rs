//! Authoritative headless world: fixed ticks, flat map, unit spawning, movement, command validation.

pub mod command;
pub mod harness;
pub mod map;
pub mod movement;
pub mod plugin;
pub mod registry;
pub mod slots;
pub mod world;

/// Fixed simulation ticks per second.
pub const DEFAULT_TICK_HZ: u16 = 20; // prototype constant: measure at M1

pub use bevy_math::Vec2;
pub use command::{
    Command, CommandKind, CommandQueue, CommandResult, CommandResults, RejectReason,
};
pub use harness::headless_app;
pub use map::load_map;
pub use plugin::SimPlugin;
pub use registry::{UnitTypeEntry, UnitTypes};
pub use slots::{ClaimError, SlotId, Slots};
pub use world::{
    ChangedThisTick, MapBounds, MoveTarget, Owner, Position, Tick, UnitId, UnitState, UnitType,
    gameplay_entity_count, snapshot,
};
