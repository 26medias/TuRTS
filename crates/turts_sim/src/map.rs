//! Legacy map staging for direct simulation callers.

use crate::{
    ids::IdAllocators,
    registry::UnitTypes,
    slots::{SLOT_COUNT, SlotId},
    world::{MapBounds, Owner, Position, UnitId, UnitType},
};
use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use turts_mod_api::{ContentId, MapDef, ModError, PackageId, UnitTypeId};

struct Spawn {
    id: u32,
    unit_type: UnitTypeId,
    owner_slot: u8,
    pos: Vec2,
}

/// Load a map for direct simulation use. Package boot should use `turts_pack::load_package`.
pub fn load_map(world: &mut World, origin: &PackageId, map: &MapDef) -> Result<(), ModError> {
    let error = |field: &str, message: String| ModError::new(&origin.0, field, message);
    if !map.size.x.is_finite() || map.size.x <= 0.0 || !map.size.y.is_finite() || map.size.y <= 0.0
    {
        return Err(error(
            "size",
            format!(
                "size must be finite and above 0, got ({}, {})",
                map.size.x, map.size.y
            ),
        ));
    }

    let types = world.resource::<UnitTypes>();
    let mut allocator = *world.resource::<IdAllocators>();
    let mut staged = Vec::with_capacity(map.placements.len());
    let mut circles: Vec<(Vec2, f32)> = Vec::new();
    for (index, placement) in map.placements.iter().enumerate() {
        let field = |name: &str| format!("placements[{index}].{name}");
        let id = ContentId::parse(&placement.unit).map_err(|m| error(&field("unit"), m))?;
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
        if !placement.contents.is_empty() {
            return Err(error(
                &field("contents"),
                "direct map loading cannot initialize inventory contents".into(),
            ));
        }
        let pos = Vec2::new(placement.pos.x, placement.pos.y);
        let radius = types.get(unit_type).map_or(0.0, |entry| entry.radius);
        check_circle(pos, radius, map.size.x, map.size.y).map_err(|m| error(&field("pos"), m))?;
        if circles.iter().any(|(other, other_radius)| {
            pos.distance_squared(*other) < (radius + other_radius).powi(2)
        }) {
            return Err(error(
                &field("pos"),
                "placement circle overlaps another placement".into(),
            ));
        }
        circles.push((pos, radius));
        let unit_id = allocator
            .allocate_unit()
            .map_err(|message| error(&field("unit"), message))?;
        staged.push(Spawn {
            id: unit_id,
            unit_type,
            owner_slot: placement.owner_slot,
            pos,
        });
    }
    if !map.nodes.is_empty() {
        return Err(error(
            "nodes",
            "direct map loading requires package catalog staging".into(),
        ));
    }
    if !map.slot_economy.is_empty() {
        return Err(error(
            "slot_economy",
            "direct map loading requires package catalog staging".into(),
        ));
    }
    world.insert_resource(allocator);
    world.insert_resource(MapBounds {
        size: Vec2::new(map.size.x, map.size.y),
    });
    for spawn in staged {
        world.spawn((
            UnitId(spawn.id),
            UnitType(spawn.unit_type),
            Owner(SlotId(spawn.owner_slot)),
            Position(spawn.pos),
        ));
    }
    Ok(())
}

fn check_circle(pos: Vec2, radius: f32, width: f32, height: f32) -> Result<(), String> {
    if !pos.x.is_finite()
        || !pos.y.is_finite()
        || pos.x - radius < 0.0
        || pos.y - radius < 0.0
        || pos.x + radius > width
        || pos.y + radius > height
    {
        return Err(format!("circle of radius {radius} must fit inside the map"));
    }
    Ok(())
}
