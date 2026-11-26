//! Test probes of the movement step (cargo feature `test-probes`).
//!
//! Pre-MP1 slice, task Q1b (plan refinement R11, design section 9 items 6, 7, 14
//! and 20). This is the only API that the trace emitter and the reference harness
//! use to read the movement state. A test never names a movement type that a later
//! task deletes, so the same emitter file runs on every build of the slice.
//!
//! Rules of this module (design 9.20):
//! - A probe reads. It never writes simulation state.
//! - There is no `cfg(not(feature = "test-probes"))` anywhere. The feature only adds
//!   this module.
//! - No probe call is inside a condition or an expression whose value the simulation
//!   uses.

use bevy_ecs::world::{EntityRef, World};

use super::plain::PlainMoveProgress;
use super::{MoveProgress, MovementActivity, MovementPools};
use crate::orders::PendingMovementOrders;
use crate::world::UnitId;

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
