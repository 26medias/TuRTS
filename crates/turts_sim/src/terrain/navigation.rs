//! The movement step of one tick: the flock on permitted terrain.
mod flock;
mod flock_step;
pub(crate) mod geometry;
use crate::movement::{MovementFrame, MovementFrameOutput};
use bevy_ecs::{prelude::Component, world::World};
pub use flock::{FIELD_CREDIT, FlockAudit, FlockMemberAudit, privileged_flock_audit};
use std::collections::BTreeSet;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TerrainMotionWait;

fn requested_target(unit: &crate::movement::MovementFrameUnit) -> Option<bevy_math::Vec2> {
    unit.target.map(|target| {
        unit.plain_progress
            .as_ref()
            .map_or(target, |progress| progress.original_point)
    })
}

fn public_move_goal(world: &World, id: u32, owner: u8) -> Option<bevy_math::Vec2> {
    let entity = crate::unit_entity(world, id)?;
    if world.get::<crate::Owner>(entity)?.0.0 != owner {
        return None;
    }
    let order = world.get::<crate::UnitOrders>(entity)?.current.as_ref()?;
    let crate::orders::OrderTargetSim::Point(goal) = order.target else {
        return None;
    };
    (order.behaviour == turts_mod_api::Behaviour::Move).then_some(goal)
}

/// The movement step of one tick: the flock (design record 2026-11-23). It is the only
/// movement path, on editable terrain and on a flat map.
pub(crate) fn step(
    world: &mut World,
    frame: &MovementFrame,
    dt: f32,
) -> (Vec<MovementFrameOutput>, BTreeSet<u32>) {
    flock::step(world, frame, dt)
}
