//! Synthetic units for load runs (`--synthetic-units N`).
//!
//! The units are of the first unit type that the loaded package registered. The content id comes
//! from `UnitTypes`, so no package name appears here. The units enter the world through
//! `load_map`, the path that the units of a map take. A system then gives each unit a new
//! `MoveTarget` every 40 ticks.

use bevy::app::FixedFirst;
use bevy::prelude::*;
use turts_mod_api::{ContentId, MapDef, MapSize, Placement, UnitTypeId, WorldPos};
use turts_sim::{MapBounds, MoveTarget, Tick, UnitId, UnitTypes, load_map};

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
    let content_id = world
        .resource::<UnitTypes>()
        .get(UnitTypeId(0))
        .ok_or("--synthetic-units needs a unit type, and the package registered none")?
        .content_id
        .clone();
    let origin = ContentId::parse(&content_id)?.package;
    let size = world
        .get_resource::<MapBounds>()
        .ok_or("--synthetic-units needs a loaded map")?
        .size;
    let tick = world.resource::<Tick>().0;
    // `load_map` continues the unit ids, so the new units are the ones above the highest id now.
    let last_old_id = world
        .query::<&UnitId>()
        .iter(world)
        .map(|id| id.0)
        .max()
        .unwrap_or(0);

    // The placement index is the key of the start point. A unit id is above its index, so a
    // unit never starts on its own first target (a target is keyed by the unit id).
    let map = MapDef {
        id: "synthetic".to_string(),
        size: MapSize {
            x: size.x,
            y: size.y,
        },
        placements: (0..count)
            .map(|index| {
                let point = target_point(tick, index, size);
                Placement {
                    unit: content_id.clone(),
                    owner_slot: 0,
                    pos: WorldPos {
                        x: point.x,
                        y: point.y,
                    },
                }
            })
            .collect(),
    };
    load_map(world, &origin, &map).map_err(|error| error.to_string())?;

    let new_units: Vec<(Entity, u32)> = world
        .query::<(Entity, &UnitId)>()
        .iter(world)
        .filter(|(_, id)| id.0 > last_old_id)
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
