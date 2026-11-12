//! Test probes of the movement step (cargo feature `test-probes`).
//!
//! Pre-MP1 slice, task Q1b (plan refinement R11, design section 9 items 6, 7, 14
//! and 20). This is the only API that the trace emitter and the reference harness
//! use to read the movement state. A test never names a movement type that a later
//! task deletes, so the same emitter file runs on every build of the slice.
//!
//! Rules of this module (design 9.20):
//! - A probe reads. It never writes simulation state. The one write is the resource
//!   `LastFrameFlags`, which no simulation code reads.
//! - There is no `cfg(not(feature = "test-probes"))` anywhere. The feature only adds
//!   a field, a resource and a call that stores into them.
//! - No probe call is inside a condition or an expression whose value the simulation
//!   uses.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use bevy_ecs::world::{EntityRef, World};

use super::frame::MovementFrameOutput;
use super::plain::PlainMoveProgress;
use super::{MoveProgress, MovementActivity, MovementPools};
use crate::orders::PendingMovementOrders;
use crate::world::UnitId;

/// The flags of the last step of one unit.
///
/// Each flag is a value of one branch of the movement step. A unit that does not step
/// leaves `separated` `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameFlags {
    /// `separated`: a neighbour was closer than the separation limit.
    pub separated: bool,
    /// `was_projected`: the hard projection moved the unit in at least one of its
    /// passes. Each of the three branches (no pool, pool, plain) stores its real
    /// value after its projection loop. `None` when the unit did not reach a
    /// projection loop in the frame (no movement, a snap to the target before the
    /// loop, an escape step).
    pub was_projected: Option<bool>,
}

/// The flags of the last movement frame, by unit id. One entry for each unit of
/// the frame. The movement system replaces the whole map at each tick.
#[derive(Resource, Debug, Default)]
pub struct LastFrameFlags(pub BTreeMap<u32, FrameFlags>);

/// Store the flags of one frame. Called by the movement system after the frame.
pub(super) fn store_frame_flags(world: &mut World, outputs: &[MovementFrameOutput]) {
    if !world.contains_resource::<LastFrameFlags>() {
        world.insert_resource(LastFrameFlags::default());
    }
    let mut flags = world.resource_mut::<LastFrameFlags>();
    flags.0.clear();
    for output in outputs {
        flags.0.insert(output.unit_id, output.probe);
    }
}

fn unit_entity(world: &World, unit: u32) -> Option<EntityRef<'_>> {
    world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit))
}

/// The movement state of a unit: `"plain"` (a `PlainMoveProgress`), `"pool"` (a
/// `MoveProgress`) or `"none"`. A unit that has both reads `"plain"`, so that a
/// count of plain unit-ticks cannot hide one. A unit that does not exist reads
/// `"none"`.
pub fn unit_state(world: &World, unit: u32) -> &'static str {
    let Some(entity) = unit_entity(world, unit) else {
        return "none";
    };
    if entity.contains::<PlainMoveProgress>() {
        "plain"
    } else if entity.contains::<MoveProgress>() {
        "pool"
    } else {
        "none"
    }
}

/// The `MovementActivity` of the `MoveProgress` of a unit: `"Pending"`, `"Moving"`,
/// `"Resting"` or `"Blocked"`; `"none"` when the unit has no `MoveProgress`. It is
/// not the order or the gather stage (design 9.7).
pub fn activity(world: &World, unit: u32) -> &'static str {
    let Some(progress) = unit_entity(world, unit)
        .and_then(|entity| entity.get::<MoveProgress>().map(|p| p.activity))
    else {
        return "none";
    };
    match progress {
        MovementActivity::Pending => "Pending",
        MovementActivity::Moving => "Moving",
        MovementActivity::Resting => "Resting",
        MovementActivity::Blocked => "Blocked",
    }
}

/// `separated` of the last frame of a unit. `false` when the unit did not step.
pub fn separated(world: &World, unit: u32) -> bool {
    world
        .get_resource::<LastFrameFlags>()
        .and_then(|flags| flags.0.get(&unit))
        .is_some_and(|flags| flags.separated)
}

/// `was_projected` of the last frame of a unit: `Some(value)` for a unit that reached
/// a projection loop in the frame (any of the three branches stores its real value);
/// `None` for a unit that did not (no movement in the frame, a snap before the loop,
/// an escape step) and for a unit that is not in the last frame.
pub fn was_projected(world: &World, unit: u32) -> Option<bool> {
    world
        .get_resource::<LastFrameFlags>()
        .and_then(|flags| flags.0.get(&unit))
        .and_then(|flags| flags.was_projected)
}

/// The number of pools in the `MovementPools` resource.
pub fn pool_count(world: &World) -> usize {
    world
        .get_resource::<MovementPools>()
        .map_or(0, |pools| pools.0.len())
}

/// The number of units that wait in a pending movement envelope. The source is
/// `PendingMovementOrders.pending_units` (`orders.rs`): the sum over the slots.
pub fn pending_movement_orders(world: &World) -> usize {
    world
        .get_resource::<PendingMovementOrders>()
        .map_or(0, |pending| pending.pending_units.values().sum())
}
