//! World setup and snapshot.

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use turts_mod_api::UnitTypeId;

use crate::registry::UnitTypes;
use crate::slots::SlotId;

/// Marks a gameplay entity. "Gameplay entity" means an entity with this component.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(pub u32);

/// The slot that owns a unit.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owner(pub SlotId);

/// Ground position of a unit.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Position(pub Vec2);

/// The registered unit type of a unit.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitType(pub UnitTypeId);

/// Where a unit is going. Present while the unit moves.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct MoveTarget(pub Vec2);

/// The fixed tick counter. It is 0 before the first tick.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tick(pub u32);

/// Extent of the flat map: `0 <= x <= size.x`, `0 <= y <= size.y`. `load_map` inserts it.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct MapBounds {
    pub size: Vec2,
}

/// The units whose position changed this tick, as `(unit_id, position)`, sorted by unit id.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct ChangedThisTick(pub Vec<(u32, Vec2)>);

/// One unit as the server presents it.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitState {
    pub unit_id: u32,
    pub content_id: String,
    pub owner_slot: u8,
    pub pos: Vec2,
}

/// All units, sorted by unit id.
pub fn snapshot(world: &mut World) -> Vec<UnitState> {
    let mut query = world.query::<(&UnitId, &UnitType, &Owner, &Position)>();
    let types = world.resource::<UnitTypes>();
    let mut states: Vec<UnitState> = query
        .iter(world)
        .map(|(id, unit_type, owner, position)| UnitState {
            unit_id: id.0,
            content_id: types
                .get(unit_type.0)
                .expect("a unit type is registered before a unit of it spawns")
                .content_id
                .clone(),
            owner_slot: owner.0.0,
            pos: position.0,
        })
        .collect();
    states.sort_unstable_by_key(|state| state.unit_id);
    states
}

/// The number of gameplay entities: entities with a `UnitId`.
pub fn gameplay_entity_count(world: &mut World) -> usize {
    world.query::<&UnitId>().iter(world).count()
}

/// FixedPostUpdate: list the units whose position changed in this tick.
///
/// This is `Changed<Position>` with one refinement. The first run of a system sees every
/// component as changed, and `load_map` spawns units outside a tick. A position that nothing
/// wrote after its spawn (`last_changed == added`) is not a change, so a stationary unit never
/// appears in the list.
pub(crate) fn collect_changes(
    mut changed: ResMut<ChangedThisTick>,
    units: Query<(&UnitId, Ref<Position>)>,
) {
    changed.0.clear();
    changed.0.extend(
        units
            .iter()
            .filter(|(_, position)| {
                position.is_changed() && position.last_changed() != position.added()
            })
            .map(|(id, position)| (id.0, position.0)),
    );
    changed.0.sort_unstable_by_key(|(id, _)| *id);
}
