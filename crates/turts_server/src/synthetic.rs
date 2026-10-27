//! Synthetic units for load runs (`--synthetic-units N`).
//!
//! The units are of the first unit type that the loaded package registered. The content id comes
//! from `UnitTypes`, so no package name appears here. The units enter the world through
//! `load_map`, the path that the units of a map take. A system then gives each unit a new
//! `MoveTarget` every 40 ticks.

use bevy::app::FixedFirst;
use bevy::math::Vec2;
use bevy::prelude::*;
use turts_mod_api::UnitTypeId;
use turts_sim::{MapBounds, MoveTarget, Tick, UnitId, UnitTypes, spawn_synthetic_units};

use crate::config::Cli;

/// Every synthetic unit gets a new `MoveTarget` after this many ticks.
const RETARGET_TICKS: u64 = 40;

/// Marks a unit that `--synthetic-units` made. The units that the map placed do not have it.
#[derive(Component, Debug, Clone, Copy)]
pub struct Synthetic;

/// Add the synthetic units and the system that moves them. Do nothing when the count is 0.
///
/// # Panics
///
/// When `spawn` fails, for example when the package registered no unit type. The option is for
/// load runs, and a run that cannot make its units must not go on without them.
pub fn install(app: &mut App, cli: &Cli) {
    if cli.synthetic_units == 0 {
        return;
    }
    if let Err(message) = spawn(app.world_mut(), cli.synthetic_units) {
        panic!("turts_server: {message}");
    }
    app.add_systems(FixedFirst, retarget);
}

/// Spawn `count` units of the first registered unit type through `load_map`, at points inside
/// `MapBounds`, owned by slot 0. Each unit starts with a `MoveTarget`. The map keeps its own
/// units: this call adds units and removes none.
pub fn spawn(world: &mut World, count: u32) -> Result<(), String> {
    let unit_type = (0..world.resource::<UnitTypes>().len())
        .filter_map(|i| {
            let id = UnitTypeId(i as u32);
            world
                .resource::<UnitTypes>()
                .get(id)
                .filter(|entry| entry.movement.is_some())
                .map(|_| id)
        })
        .next()
        .ok_or("--synthetic-units needs a mobile unit type")?;
    let size = world
        .get_resource::<MapBounds>()
        .ok_or("--synthetic-units needs a loaded map")?
        .size;
    let tick = world.resource::<Tick>().0;
    let positions: Vec<Vec2> = (0..count)
        .map(|index| target_point(tick, index, size))
        .collect();
    let ids = spawn_synthetic_units(world, unit_type, &positions)?;
    let new_units: Vec<(Entity, u32)> = world
        .query::<(Entity, &UnitId)>()
        .iter(world)
        .filter(|(_, id)| ids.contains(&id.0))
        .map(|(entity, id)| (entity, id.0))
        .collect();
    for (entity, id) in new_units {
        world
            .entity_mut(entity)
            .insert((Synthetic, MoveTarget(target_point(tick, id, size))));
    }
    Ok(())
}

/// FixedFirst: give a unit a new target when its turn comes. The turns of the units are spread
/// over the 40 ticks (unit id plus tick, modulo 40), so each tick does the same small amount of
/// work and the tick time does not jump on every 40th tick. `Tick` still holds the previous
/// tick here. The system runs first in the tick, so the new target is in place before the
/// movement system reads it.
pub(crate) fn retarget(
    tick: Res<Tick>,
    bounds: Res<MapBounds>,
    units: Query<(Entity, &UnitId), With<Synthetic>>,
    mut commands: Commands,
) {
    for (entity, id) in &units {
        if (u64::from(tick.0) + u64::from(id.0)) % RETARGET_TICKS == 0 {
            commands
                .entity(entity)
                .insert(MoveTarget(target_point(tick.0, id.0, bounds.size)));
        }
    }
}

/// A point inside the map (0 to `size` on each axis, ends included), from `tick` and a unit
/// key. The same input always gives the same point. Two splitmix64 draws, seeded from the tick
/// and the key, give the two coordinates.
pub fn target_point(tick: u32, key: u32, size: Vec2) -> Vec2 {
    let mut state = (u64::from(tick) << 32) | u64::from(key);
    let x = unit_interval(splitmix64(&mut state));
    let y = unit_interval(splitmix64(&mut state));
    Vec2::new(x * size.x, y * size.y)
}

/// One step of splitmix64 (Steele, Lea and Flood, 2014).
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A number from 0 up to (not including) 1, from the top 24 bits. An `f32` holds 24 bits.
fn unit_interval(bits: u64) -> f32 {
    (bits >> 40) as f32 / (1u32 << 24) as f32
}
