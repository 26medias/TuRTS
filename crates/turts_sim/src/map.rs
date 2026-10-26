//! Flat map from MapDef.

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use turts_mod_api::{ContentId, MapDef, ModError, PackageId, UnitTypeId};

use crate::registry::UnitTypes;
use crate::slots::{SLOT_COUNT, SlotId};
use crate::world::{MapBounds, Owner, Position, UnitId, UnitType};

/// The next unit id to hand out. Ids start at 1 and never repeat, so a second `load_map`
/// call (synthetic units) continues after the first.
#[derive(Resource, Debug)]
struct NextUnitId(u32);

impl Default for NextUnitId {
    fn default() -> Self {
        Self(1)
    }
}

/// One placement that passed every check.
struct Spawn {
    unit_type: UnitTypeId,
    owner_slot: u8,
    pos: Vec2,
}

/// Load `map` of package `origin`. The call checks every field first and spawns nothing on
/// error. On success it inserts `MapBounds` and spawns one unit per placement, with unit ids
/// in placement order.
pub fn load_map(world: &mut World, origin: &PackageId, map: &MapDef) -> Result<(), ModError> {
    let spawns = validate(world, origin, map)?;
    world.insert_resource(MapBounds {
        size: Vec2::new(map.size.x, map.size.y),
    });
    let mut next_id = world.get_resource_or_insert_with(NextUnitId::default);
    let first_id = next_id.0;
    next_id.0 += spawns.len() as u32;
    for (offset, spawn) in spawns.into_iter().enumerate() {
        world.spawn((
            UnitId(first_id + offset as u32),
            UnitType(spawn.unit_type),
            Owner(SlotId(spawn.owner_slot)),
            Position(spawn.pos),
        ));
    }
    Ok(())
}

fn validate(world: &World, origin: &PackageId, map: &MapDef) -> Result<Vec<Spawn>, ModError> {
    let error = |field: &str, message: String| ModError::new(&origin.0, field, message);
    let size_ok = |v: f32| v.is_finite() && v > 0.0;
    if !(size_ok(map.size.x) && size_ok(map.size.y)) {
        return Err(error(
            "size",
            format!(
                "size must be finite and above 0, got ({}, {})",
                map.size.x, map.size.y
            ),
        ));
    }
    let types = world.resource::<UnitTypes>();
    let mut spawns = Vec::with_capacity(map.placements.len());
    for (index, placement) in map.placements.iter().enumerate() {
        let field = |name: &str| format!("placements[{index}].{name}");
        let id =
            ContentId::parse(&placement.unit).map_err(|message| error(&field("unit"), message))?;
        if id.package != *origin {
            return Err(error(
                &field("unit"),
                format!("unit `{}` is not in package `{}`", placement.unit, origin.0),
            ));
        }
        let unit_type = types.find(&placement.unit).ok_or_else(|| {
            error(
                &field("unit"),
                format!("unknown unit type `{}`", placement.unit),
            )
        })?;
        if placement.owner_slot >= SLOT_COUNT {
            return Err(error(
                &field("owner_slot"),
                format!(
                    "owner slot must be below {SLOT_COUNT}, got {}",
                    placement.owner_slot
                ),
            ));
        }
        let (x, y) = (placement.pos.x, placement.pos.y);
        let inside = x.is_finite()
            && y.is_finite()
            && (0.0..=map.size.x).contains(&x)
            && (0.0..=map.size.y).contains(&y);
        if !inside {
            return Err(error(
                &field("pos"),
                format!(
                    "position must be inside the map (0 to {} by 0 to {}), got ({x}, {y})",
                    map.size.x, map.size.y
                ),
            ));
        }
        spawns.push(Spawn {
            unit_type,
            owner_slot: placement.owner_slot,
            pos: Vec2::new(x, y),
        });
    }
    Ok(spawns)
}
