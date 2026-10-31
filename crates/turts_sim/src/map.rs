//! Legacy map staging for direct simulation callers.

use crate::{
    bootstrap::InstalledCatalog,
    economy::SurfaceNodes,
    ids::IdAllocators,
    registry::UnitTypes,
    slots::{SlotId, Slots},
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

    let slots = world
        .get_resource::<Slots>()
        .ok_or_else(|| error("world", "simulation slots are not initialized".into()))?;
    let types = world.resource::<UnitTypes>();
    let mut allocator = *world.resource::<IdAllocators>();
    let mut staged = Vec::with_capacity(map.placements.len());
    let mut circles: Vec<(Vec2, f32, String)> = Vec::new();
    for entity in world.iter_entities() {
        let Some(id) = entity.get::<UnitId>() else {
            continue;
        };
        let unit_type = entity.get::<UnitType>().ok_or_else(|| {
            error(
                "world",
                format!("unit {} has no registered unit type component", id.0),
            )
        })?;
        let position = entity
            .get::<Position>()
            .ok_or_else(|| error("world", format!("unit {} has no position component", id.0)))?;
        let entry = types
            .get(unit_type.0)
            .ok_or_else(|| error("world", format!("unit {} references an unknown type", id.0)))?;
        circles.push((
            position.0,
            entry.radius,
            format!("unit {} ({})", id.0, entry.content_id),
        ));
    }
    if let Some(nodes) = world.get_resource::<SurfaceNodes>()
        && nodes.0.iter().any(|node| node.remaining > 0)
    {
        let catalog = world
            .get_resource::<InstalledCatalog>()
            .ok_or_else(|| error("world", "live nodes have no installed catalog".into()))?;
        for node in nodes.0.iter().filter(|node| node.remaining > 0) {
            let definition = catalog.0.nodes.get(&node.content_id).ok_or_else(|| {
                error(
                    "world",
                    format!(
                        "live node `{}` has no installed definition",
                        node.content_id
                    ),
                )
            })?;
            circles.push((
                node.position.0,
                definition.radius,
                format!("node {} ({})", node.id.0, node.content_id),
            ));
        }
    }
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
        if !slots.contains(SlotId(placement.owner_slot)) {
            return Err(error(
                &field("owner_slot"),
                format!(
                    "owner slot must be below {}, got {}",
                    slots.count(),
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
        let radius = types.get(unit_type).expect("unit type was resolved").radius;
        check_circle_bounds(pos, radius, map.size.x, map.size.y)
            .map_err(|m| error(&field("pos"), m))?;
        check_overlap(pos, radius, &circles).map_err(|m| error(&field("pos"), m))?;
        circles.push((pos, radius, placement.unit.clone()));
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
    if !map.belts.is_empty() {
        return Err(error(
            "belts",
            "direct map loading requires package catalog staging".into(),
        ));
    }
    world.insert_resource(allocator);
    world.insert_resource(MapBounds {
        size: Vec2::new(map.size.x, map.size.y),
    });
    for spawn in staged {
        let order_capable = crate::orders::unit_order_capable(world, spawn.unit_type);
        let mut entity = world.spawn((
            UnitId(spawn.id),
            UnitType(spawn.unit_type),
            Owner(SlotId(spawn.owner_slot)),
            Position(spawn.pos),
        ));
        if order_capable {
            entity.insert(crate::orders::UnitOrders::default());
        }
    }
    Ok(())
}

pub(crate) fn check_circle_bounds(
    pos: Vec2,
    radius: f32,
    width: f32,
    height: f32,
) -> Result<(), String> {
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

pub(crate) fn check_overlap(
    pos: Vec2,
    radius: f32,
    other: &[(Vec2, f32, String)],
) -> Result<(), String> {
    for (other_pos, other_radius, id) in other {
        let required = radius + other_radius;
        if pos.distance_squared(*other_pos) < required * required {
            return Err(format!("placement circle overlaps `{id}`"));
        }
    }
    Ok(())
}
