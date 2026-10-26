//! Movement integrator.

use bevy_ecs::prelude::*;
use bevy_time::{Fixed, Time};

use crate::registry::UnitTypes;
use crate::world::{MoveTarget, Position, UnitType};

/// FixedUpdate: move each unit with a `MoveTarget` by `speed * delta` toward its target.
/// A unit that is within one step of its target lands on it and loses the `MoveTarget`.
pub(crate) fn integrate_movement(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    types: Res<UnitTypes>,
    mut units: Query<(Entity, &UnitType, &MoveTarget, &mut Position)>,
) {
    let delta = time.delta_secs();
    for (entity, unit_type, target, mut position) in &mut units {
        let Some(entry) = types.get(unit_type.0) else {
            continue;
        };
        let Some(movement) = &entry.movement else {
            commands.entity(entity).remove::<MoveTarget>();
            continue;
        };
        let step = movement.speed * delta;
        let offset = target.0 - position.0;
        let distance = offset.length();
        let next = if distance <= step {
            commands.entity(entity).remove::<MoveTarget>();
            target.0
        } else {
            position.0 + offset * (step / distance)
        };
        // A move to the current position writes nothing, so it is not a change.
        position.set_if_neq(Position(next));
    }
}
