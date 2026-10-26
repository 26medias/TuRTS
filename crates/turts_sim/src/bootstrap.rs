use std::collections::BTreeMap;

use bevy_ecs::world::World;
use bevy_math::Vec2;
use turts_mod_api::{
    ContentCatalog, ContentId, Footprint, MapDef, ModError, ResourceKind, SourceDef, UnitTypeId,
};

use crate::{
    SimClock,
    economy::{
        EconomyOrders, GatherOrder, NodeRecord, SurfaceNodes,
        ledger::{Account, EconomyLedger, Inventory},
        production::ProductionQueues,
    },
    ids::IdAllocators,
    registry::UnitTypes,
    slots::SlotId,
    world::{MapBounds, Owner, Position, UnitId, UnitType},
};

#[derive(bevy_ecs::prelude::Resource, Debug, Clone, PartialEq)]
pub struct InstalledCatalog(pub ContentCatalog);
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, PartialEq, Eq)]
pub struct ProductionDurations(pub BTreeMap<ContentId, u64>);

#[derive(Debug)]
pub struct StagedPackage {
    types: UnitTypes,
    bounds: MapBounds,
    units: Vec<(u32, UnitTypeId, u8, Vec2)>,
    allocators: IdAllocators,
    economy: EconomyLedger,
    nodes: SurfaceNodes,
    orders: EconomyOrders,
    queues: ProductionQueues,
    catalog: InstalledCatalog,
    durations: ProductionDurations,
}

/// Validate and own a complete package install without changing `world`.
pub fn stage_package(
    world: &World,
    catalog: ContentCatalog,
    source_map: SourceDef<MapDef>,
    tick_hz: u16,
) -> Result<StagedPackage, ModError> {
    let package = catalog
        .units
        .keys()
        .next()
        .or_else(|| catalog.nodes.keys().next())
        .or_else(|| catalog.resources.keys().next())
        .or_else(|| catalog.production.keys().next())
        .map_or_else(String::new, |id| id.package.0.clone());
    let error =
        |field: &str, msg: String| ModError::new(&package, field, msg).with_file(&source_map.file);
    if tick_hz == 0 {
        return Err(error("tick_hz", "must be above zero".into()));
    }
    if world.get_resource::<SimClock>().is_none()
        || world.get_resource::<UnitTypes>().is_none()
        || world.get_resource::<IdAllocators>().is_none()
    {
        return Err(error(
            "world",
            "simulation resources are not initialized".into(),
        ));
    }
    if world.contains_resource::<MapBounds>()
        || world
            .iter_entities()
            .any(|entity| entity.contains::<UnitId>())
    {
        return Err(error(
            "map",
            "a package is already installed in this world".into(),
        ));
    }
    let map = &source_map.value;
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
    let existing = world.resource::<UnitTypes>();
    let types = UnitTypes::from_catalog(&catalog, existing).map_err(|mut e| {
        e.file.get_or_insert(source_map.file.clone());
        e
    })?;
    let mut allocators = *world.resource::<IdAllocators>();
    let mut units = Vec::with_capacity(map.placements.len());
    let mut spatial: Vec<(Vec2, f32, String)> = Vec::new();
    let mut economy = EconomyLedger::default();
    let mut orders = EconomyOrders::default();
    let mut queues = ProductionQueues::default();
    let unit_by_id = &catalog.units;

    // Every balance resource exists for each player slot, even when its seed is zero.
    for (id, resource) in &catalog.resources {
        if resource.kind == ResourceKind::Balance {
            for slot in 0..2 {
                economy
                    .balances
                    .insert((slot, id.clone()), Account::default());
            }
        }
    }
    for (slot_index, seed) in map.slot_economy.iter().enumerate() {
        if seed.slot >= 2 {
            return Err(error(
                &format!("slot_economy[{slot_index}].slot"),
                format!("slot must be below 2, got {}", seed.slot),
            ));
        }
        for (balance_index, amount) in seed.balances.iter().enumerate() {
            let id = ContentId::parse(&amount.balance).map_err(|m| {
                error(
                    &format!("slot_economy[{slot_index}].balances[{balance_index}].balance"),
                    m,
                )
            })?;
            let def = catalog.resources.get(&id).ok_or_else(|| {
                error(
                    &format!("slot_economy[{slot_index}].balances[{balance_index}].balance"),
                    format!("unknown resource `{id}`"),
                )
            })?;
            if def.kind != ResourceKind::Balance {
                return Err(error(
                    &format!("slot_economy[{slot_index}].balances[{balance_index}].balance"),
                    format!("`{id}` is not a balance resource"),
                ));
            }
            let account = economy
                .balances
                .get_mut(&(seed.slot, id))
                .expect("all balance accounts are seeded");
            account.on_hand = amount.amount;
        }
    }

    for (index, placement) in map.placements.iter().enumerate() {
        let field = |suffix: &str| format!("placements[{index}].{suffix}");
        let id = ContentId::parse(&placement.unit).map_err(|m| error(&field("unit"), m))?;
        let def = unit_by_id
            .get(&id)
            .ok_or_else(|| error(&field("unit"), format!("unknown unit `{id}`")))?;
        if placement.owner_slot >= 2 {
            return Err(error(
                &field("owner_slot"),
                format!("owner slot must be below 2, got {}", placement.owner_slot),
            ));
        }
        let pos = Vec2::new(placement.pos.x, placement.pos.y);
        let radius = match def.footprint {
            Footprint::Circle { radius } => radius,
        };
        check_circle_bounds(pos, radius, map.size.x, map.size.y)
            .map_err(|m| error(&field("pos"), m))?;
        check_overlap(pos, radius, &spatial).map_err(|m| error(&field("pos"), m))?;
        spatial.push((pos, radius, id.to_string()));
        if def.inventory.is_none() && !placement.contents.is_empty() {
            return Err(error(&field("contents"), "unit has no inventory".into()));
        }
        if let Some(inventory_def) = &def.inventory {
            let mut inventory = Inventory {
                capacity: inventory_def.capacity,
                items: BTreeMap::new(),
            };
            let mut used: u128 = 0;
            for (item_index, stack) in placement.contents.iter().enumerate() {
                let item = ContentId::parse(&stack.item).map_err(|m| {
                    error(
                        &format!("placements[{index}].contents[{item_index}].item"),
                        m,
                    )
                })?;
                if !inventory_def.accepts.contains(&item) {
                    return Err(error(
                        &format!("placements[{index}].contents[{item_index}].item"),
                        format!("inventory does not accept `{item}`"),
                    ));
                }
                let resource = catalog
                    .resources
                    .get(&item)
                    .expect("accepted inventory item was validated");
                if resource.kind != ResourceKind::Item {
                    return Err(error(
                        &format!("placements[{index}].contents[{item_index}].item"),
                        format!("`{item}` is not an item resource"),
                    ));
                }
                if stack.amount == 0 {
                    return Err(error(
                        &format!("placements[{index}].contents[{item_index}].amount"),
                        "must be positive".into(),
                    ));
                }
                used = used
                    .checked_add(u128::from(stack.amount))
                    .ok_or_else(|| error(&field("contents"), "inventory amount overflow".into()))?;
                let account = inventory.items.entry(item).or_default();
                account.on_hand = account
                    .on_hand
                    .checked_add(stack.amount)
                    .ok_or_else(|| error(&field("contents"), "inventory stack overflow".into()))?;
            }
            if used > u128::from(inventory.capacity) {
                return Err(error(
                    &field("contents"),
                    format!("contents exceed inventory capacity {}", inventory.capacity),
                ));
            }
            economy.inventories.insert(allocators.next_unit, inventory);
        }
        if def.gatherer.is_some() {
            orders.0.insert(
                allocators.next_unit,
                GatherOrder {
                    node: None,
                    receiver: None,
                    stage: "Idle".into(),
                },
            );
        }
        if def.producer.is_some() {
            queues.0.insert(allocators.next_unit, Vec::new());
        }
        let type_id = types
            .find(&id.to_string())
            .expect("candidate contains catalog unit");
        let unit_id = allocators
            .allocate_unit()
            .map_err(|m| error(&field("unit"), m))?;
        units.push((unit_id, type_id, placement.owner_slot, pos));
    }
    let mut nodes = Vec::with_capacity(map.nodes.len());
    for (index, placement) in map.nodes.iter().enumerate() {
        let field = |suffix: &str| format!("nodes[{index}].{suffix}");
        let content = ContentId::parse(&placement.node).map_err(|m| error(&field("node"), m))?;
        let def = catalog
            .nodes
            .get(&content)
            .ok_or_else(|| error(&field("node"), format!("unknown node `{content}`")))?;
        let pos = Vec2::new(placement.pos.x, placement.pos.y);
        check_circle_bounds(pos, def.radius, map.size.x, map.size.y)
            .map_err(|m| error(&field("pos"), m))?;
        check_overlap(pos, def.radius, &spatial).map_err(|m| error(&field("pos"), m))?;
        spatial.push((pos, def.radius, content.to_string()));
        if placement.remaining == 0 {
            return Err(error(&field("remaining"), "must be positive".into()));
        }
        let id = allocators
            .allocate_node()
            .map_err(|m| error(&field("node"), m))?;
        nodes.push(NodeRecord {
            id,
            content_id: content,
            position: Position(pos),
            remaining: placement.remaining,
        });
    }
    let mut durations = BTreeMap::new();
    for (id, recipe) in &catalog.production {
        let ticks = u128::from(recipe.duration_ms)
            .checked_mul(u128::from(tick_hz))
            .and_then(|value| value.checked_add(999))
            .ok_or_else(|| {
                error(
                    "production.duration_ms",
                    format!("duration for `{id}` overflows"),
                )
            })?
            / 1000;
        let ticks = u64::try_from(ticks).map_err(|_| {
            error(
                "production.duration_ms",
                format!("duration for `{id}` exceeds tick range"),
            )
        })?;
        if ticks == 0 {
            return Err(error(
                "production.duration_ms",
                format!("duration for `{id}` resolves to fewer than one tick"),
            ));
        }
        durations.insert(id.clone(), ticks);
    }
    Ok(StagedPackage {
        types,
        bounds: MapBounds {
            size: Vec2::new(map.size.x, map.size.y),
        },
        units,
        allocators,
        economy,
        nodes: SurfaceNodes(nodes),
        orders,
        queues,
        catalog: InstalledCatalog(catalog),
        durations: ProductionDurations(durations),
    })
}

/// Commit a fully validated package. This operation has no fallible steps.
pub fn install_package(world: &mut World, staged: StagedPackage) {
    world.insert_resource(staged.types);
    world.insert_resource(staged.bounds);
    world.insert_resource(staged.allocators);
    world.insert_resource(staged.economy);
    world.insert_resource(staged.nodes);
    world.insert_resource(staged.orders);
    world.insert_resource(staged.queues);
    world.insert_resource(staged.catalog);
    world.insert_resource(staged.durations);
    for (id, kind, owner, position) in staged.units {
        world.spawn((
            UnitId(id),
            UnitType(kind),
            Owner(SlotId(owner)),
            Position(position),
        ));
    }
}

fn check_circle_bounds(pos: Vec2, radius: f32, width: f32, height: f32) -> Result<(), String> {
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
fn check_overlap(pos: Vec2, radius: f32, other: &[(Vec2, f32, String)]) -> Result<(), String> {
    for (other_pos, other_radius, id) in other {
        let required = radius + other_radius;
        if pos.distance_squared(*other_pos) < required * required {
            return Err(format!("circle overlaps `{id}`"));
        }
    }
    Ok(())
}
