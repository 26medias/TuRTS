//! Movement integrator.

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_time::{Fixed, Time};

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
            let target = entity.get::<MoveTarget>()?;
            let position = entity.get::<Position>()?;
            let speed = types
                .get(unit_type.0)
                .and_then(|entry| entry.movement.as_ref())
                .map(|movement| movement.speed);
            Some((
                entity
                    .get::<crate::world::UnitId>()
                    .map_or(u32::MAX, |id| id.0),
                entity.id(),
                speed,
                target.0,
                position.0,
            ))
        })
        .collect();
    units.sort_unstable_by_key(|(id, ..)| *id);
    for (_, entity, speed, target, current) in units {
        let Some(speed) = speed else {
            world.entity_mut(entity).remove::<MoveTarget>();
            continue;
        };
        let step = speed * delta;
        let offset = target - current;
        let distance = offset.length();
        let next = if distance <= step {
            target
        } else {
            current + offset * (step / distance)
        };
        let mut entity_mut = world.entity_mut(entity);
        if distance <= step {
            entity_mut.remove::<MoveTarget>();
        }
        if let Some(mut position) = entity_mut.get_mut::<Position>() {
            position.set_if_neq(Position(next));
        }
    }
}
