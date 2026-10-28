use std::collections::BTreeMap;

use bevy_ecs::world::World;
use bevy_math::Vec2;
use turts_mod_api::{
    ContentCatalog, ContentId, Footprint, MapDef, ModError, ResourceKind, SourceDef, UnitTypeId,
};

use crate::{
    SimClock,
    boot_m3::{self, PreparedMapRows},
    economy::{
        EconomyOrders, GatherOrder, NodeRecord, SurfaceNodes,
        gather::{GatherStage, RateCarry},
        ledger::{Account, EconomyLedger, Inventory},
        production::ProductionQueues,
    },
    ids::IdAllocators,
    map::{check_circle_bounds, check_overlap},
    registry::UnitTypes,
    slots::{SlotId, Slots},
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
    m3: PreparedMapRows,
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
    for unit in catalog.units.values() {
        let Some(inserter) = &unit.inserter else {
            continue;
        };
        let file = catalog
            .source_files
            .get(&unit.id)
            .map(String::as_str)
            .unwrap_or("content");
        for (field, duration_ms) in [
            ("inserter.swing_ms", inserter.swing_ms),
            ("inserter.return_ms", inserter.return_ms),
        ] {
            if boot_m3::try_inserter_duration_ticks(duration_ms, u32::from(tick_hz)).is_none() {
                let message = if duration_ms == 0 {
                    "must be positive".to_string()
                } else {
                    format!("duration exceeds u32 elapsed ticks at {tick_hz} Hz")
                };
                return Err(ModError::new(&unit.id.package.0, field, message).with_file(file));
            }
        }
    }
    if world.get_resource::<SimClock>().is_none()
        || world.get_resource::<UnitTypes>().is_none()
        || world.get_resource::<IdAllocators>().is_none()
        || world.get_resource::<Slots>().is_none()
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
    let slots = world.resource::<Slots>();
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
            for slot in 0..slots.count() {
                economy
                    .balances
                    .insert((slot, id.clone()), Account::default());
            }
        }
    }
    for (slot_index, seed) in map.slot_economy.iter().enumerate() {
        if !slots.contains(SlotId(seed.slot)) {
            return Err(error(
                &format!("slot_economy[{slot_index}].slot"),
                format!("slot must be below {}, got {}", slots.count(), seed.slot),
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
        if def.conveyor.is_some() {
            return Err(error(
                &field("unit"),
                "conveyor pieces must be declared in map.belts".into(),
            ));
        }
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
                accepted_items: inventory_def.accepts.iter().cloned().collect(),
                items: BTreeMap::new(),
            };
            let mut used: u128 = 0;
            let mut item_amounts = BTreeMap::<ContentId, u128>::new();
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
                let amount = item_amounts.entry(item).or_default();
                *amount = amount
                    .checked_add(u128::from(stack.amount))
                    .ok_or_else(|| {
                        error(&field("contents"), "inventory item amount overflow".into())
                    })?;
            }
            for (item, amount) in item_amounts {
                let amount = u64::try_from(amount).map_err(|_| {
                    error(
                        &field("contents"),
                        format!("inventory item `{item}` amount exceeds u64"),
                    )
                })?;
                inventory.items.entry(item).or_default().on_hand = amount;
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
                    deposit: None,
                    receiver: None,
                    stage: GatherStage::Idle,
                    carry: RateCarry::default(),
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
    for (line_index, line_def) in map.belts.iter().enumerate() {
        if !slots.contains(SlotId(line_def.owner_slot)) {
            return Err(error(
                &format!("belts[{line_index}].owner_slot"),
                format!(
                    "owner slot must be below {}, got {}",
                    slots.count(),
                    line_def.owner_slot
                ),
            ));
        }
    }
    let m3 = boot_m3::stage_m3_map(
        &catalog,
        map,
        &source_map.file,
        &units,
        boot_m3::MapStageContext {
            nodes: &nodes,
            types: &types,
            allocators: &mut allocators,
            existing_spatial: &mut spatial,
        },
    )?;
    let mut durations = BTreeMap::new();
    for (id, duration_ms) in catalog
        .production
        .iter()
        .map(|(id, recipe)| (id, recipe.duration_ms))
        .chain(
            catalog
                .recipes
                .iter()
                .map(|(id, recipe)| (id, recipe.duration_ms)),
        )
        .chain(
            catalog
                .technologies
                .iter()
                .map(|(id, technology)| (id, technology.duration_ms)),
        )
    {
        let ticks = boot_m3::try_ms_to_ticks(duration_ms, u32::from(tick_hz)).ok_or_else(|| {
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
        m3,
    })
}

/// Commit a fully validated package. This operation has no fallible steps.
pub fn install_package(world: &mut World, staged: StagedPackage) {
    boot_m3::initialize_m3_resources(world);
    world.insert_resource(staged.types);
    world.insert_resource(staged.bounds);
    world.insert_resource(staged.allocators);
    world.insert_resource(staged.economy);
    world.insert_resource(staged.nodes);
    world.insert_resource(staged.orders);
    world.insert_resource(staged.queues);
    world.insert_resource(staged.catalog);
    world.insert_resource(staged.durations);
    world.insert_resource(staged.m3.conveyors);
    world.insert_resource(staged.m3.item_table);
    for (id, kind, owner, position) in staged.units {
        world.spawn((
            UnitId(id),
            UnitType(kind),
            Owner(SlotId(owner)),
            Position(position),
        ));
    }
    for (id, kind, owner, position) in staged.m3.extra_units {
        world.spawn((UnitId(id), kind, Owner(SlotId(owner)), Position(position)));
    }
    for (id, rows) in staged.m3.capability_rows {
        boot_m3::apply_capability_rows(world, id, rows);
    }
}
