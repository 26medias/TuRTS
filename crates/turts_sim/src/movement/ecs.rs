//! The ECS system that runs the movement frame.

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_math::Vec2;
use bevy_time::{Fixed, Time};

use super::frame::{MovementFrame, step_frame_preserving_resting};
use super::plain::PlainMoveProgress;
use super::{MoveProgress, MovementPools, PhysicalEscape};
use crate::registry::UnitTypes;
use crate::world::{MoveTarget, Position, UnitType};

/// FixedUpdate: move each unit with a `MoveTarget` by `speed * delta` toward its target.
/// A unit that is within one step of its target lands on it and loses the `MoveTarget`.
pub(crate) fn integrate_movement(world: &mut World) {
    let delta = world.resource::<Time<Fixed>>().delta_secs();
    let types = world.resource::<UnitTypes>();
    let mut units: Vec<_> = world
        .iter_entities()
        .filter_map(|entity| {
            let unit_type = entity.get::<UnitType>()?;
            let position = entity.get::<Position>()?;
            let entry = types.get(unit_type.0)?;
            Some((
                entity
                    .get::<crate::world::UnitId>()
                    .map_or(u32::MAX, |id| id.0),
                entity.id(),
                crate::movement::MovementFrameUnit {
                    unit_id: entity
                        .get::<crate::world::UnitId>()
                        .map_or(u32::MAX, |id| id.0),
                    owner_slot: entity
                        .get::<crate::world::Owner>()
                        .map_or(0, |owner| owner.0.0),
                    unit_type: unit_type.0.0,
                    position: position.0,
                    radius: entry.radius,
                    speed: entry.movement.as_ref().map(|movement| movement.speed),
                    target: entity.get::<MoveTarget>().map(|target| target.0),
                    progress: entity.get::<MoveProgress>().cloned(),
                    plain_progress: entity.get::<PlainMoveProgress>().cloned(),
                    physical_escape: entity.get::<PhysicalEscape>().cloned(),
                },
            ))
        })
        .collect();
    units.sort_unstable_by_key(|(id, ..)| *id);
    let bounds = world
        .get_resource::<crate::world::MapBounds>()
        .map_or(Vec2::ZERO, |bounds| bounds.size);
    let tick_hz = world
        .get_resource::<crate::SimClock>()
        .map_or(20, |clock| clock.tick_hz);
    let frame = MovementFrame {
        bounds,
        tick_hz,
        units: units.iter().map(|(_, _, unit)| unit.clone()).collect(),
        pools: world.resource::<MovementPools>().0.clone(),
    };
    let preserve_resting_units = units
        .iter()
        .filter_map(|(unit_id, entity, _)| {
            world
                .entity(*entity)
                .get::<crate::orders::UnitOrders>()
                .and_then(|orders| orders.current.as_ref())
                .is_some_and(|order| {
                    order.behaviour == turts_mod_api::Behaviour::Attack
                        && matches!(order.target, crate::orders::OrderTargetSim::Point(_))
                })
                .then_some(*unit_id)
        })
        .collect();
    let outputs = step_frame_preserving_resting(&frame, delta, &preserve_resting_units);
    for ((_, entity, _), output) in units.into_iter().zip(outputs) {
        let mut entity_mut = world.entity_mut(entity);
        if let Some(target) = output.target {
            entity_mut.insert(MoveTarget(target));
        } else {
            entity_mut.remove::<MoveTarget>();
        }
        if let Some(mut position) = entity_mut.get_mut::<Position>() {
            position.set_if_neq(Position(output.position));
        }
        if let Some(progress) = output.progress {
            entity_mut.insert(progress);
        } else {
            entity_mut.remove::<MoveProgress>();
        }
        if let Some(progress) = output.plain_progress {
            entity_mut.insert(progress);
        } else {
            entity_mut.remove::<PlainMoveProgress>();
        }
        if let Some(escape) = output.physical_escape {
            entity_mut.insert(escape);
        } else {
            entity_mut.remove::<PhysicalEscape>();
        }
    }
    let active_pools: std::collections::BTreeSet<_> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<MoveProgress>()
                .map(|progress| progress.pool_id)
        })
        .collect();
    world
        .resource_mut::<MovementPools>()
        .0
        .retain(|pool_id, _| active_pools.contains(pool_id));
}
