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
        income::{self, IncomeActivation},
        ledger::{Account, EconomyLedger, Inventory},
        power::{self, OperatingResources, RateQ},
        production::ProductionQueues,
    },
    ids::IdAllocators,
    map::{check_circle_bounds, check_overlap},
    registry::UnitTypes,
    slots::{SlotId, Slots},
    world::{MapBounds, Owner, Position, UnitFacing, UnitId, UnitType},
};

#[derive(bevy_ecs::prelude::Resource, Debug, Clone, PartialEq)]
pub struct InstalledCatalog(pub ContentCatalog);
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, PartialEq, Eq)]
pub struct ProductionDurations(pub BTreeMap<ContentId, u64>);

/// Income periods checked against the selected server tick rate, keyed by unit definition.
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct IncomePeriods(pub(crate) BTreeMap<ContentId, u64>);

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
    operating: Option<OperatingResources>,
    rate_q: u64,
    income_periods: IncomePeriods,
    income_clocks: Vec<(u32, IncomeActivation)>,
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
    let rate_q = power::catalog_rate_q(&catalog)?;
    let mut income_periods = IncomePeriods::default();
    for unit in catalog.units.values() {
        let Some(income) = &unit.income else { continue };
        let file = catalog
            .source_files
            .get(&unit.id)
            .map_or("content", String::as_str);
        let fail = |field: &str, message: &str| {
            ModError::new(&unit.id.package.0, field, message).with_file(file)
        };
        if income.amount == 0 || unit.health.is_none() {
            return Err(fail(
                "income",
                "income needs a positive amount and a unit with health",
            ));
        }
        let resource = ContentId::parse(&income.resource)
            .map_err(|message| fail("income.resource", &message))?;
        if !catalog
            .resources
            .get(&resource)
            .is_some_and(|resource| resource.kind == ResourceKind::Balance)
        {
            return Err(fail(
                "income.resource",
                "income must reference a declared Balance resource",
            ));
        }
        let period = boot_m3::try_ms_to_ticks(u64::from(income.period_ms), u32::from(tick_hz))
            .filter(|period| *period > 0)
            .ok_or_else(|| {
                fail(
                    "income.period_ms",
                    "period must be positive and fit the server tick range",
                )
            })?;
        income_periods.0.insert(unit.id.clone(), period);
    }
    for unit in catalog.units.values() {
        let file = catalog
            .source_files
            .get(&unit.id)
            .map(String::as_str)
            .unwrap_or("content");
        for (field, value) in [
            unit.extractor
                .as_ref()
                .map(|capability| ("extractor.interaction_range", capability.interaction_range)),
            unit.inserter
                .as_ref()
                .map(|capability| ("inserter.reach", capability.reach)),
            unit.weapon
                .as_ref()
                .map(|capability| ("weapon.range", capability.range)),
        ]
        .into_iter()
        .flatten()
        {
            if !value.is_finite() || value < 0.0 {
                return Err(
                    ModError::new(&unit.id.package.0, field, "must be finite and >= 0")
                        .with_file(file),
                );
            }
        }
    }
    for unit in catalog.units.values() {
        if let Some(weapon) = &unit.weapon
            && boot_m3::try_weapon_cooldown_ticks(weapon.cooldown_ms, u32::from(tick_hz)).is_none()
        {
            let file = catalog
                .source_files
                .get(&unit.id)
                .map(String::as_str)
                .unwrap_or("content");
            return Err(ModError::new(
                &unit.id.package.0,
                "weapon.cooldown_ms",
                format!("duration exceeds u32 cooldown ticks at {tick_hz} Hz"),
            )
            .with_file(file));
        }
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
        if ticks.checked_mul(rate_q).is_none() {
            return Err(ModError::new(&id.package.0, "duration_ms", "duration times the installed RateQ scale exceeds u64 progress; use a shorter duration")
                .with_file(catalog.source_files.get(id).map_or("content", String::as_str)));
        }
        durations.insert(id.clone(), ticks);
    }
    let mut operating = OperatingResources::from_catalog(&catalog, slots.count());
    let mut income_clocks = Vec::new();
    let initial_health: BTreeMap<_, _> = m3
        .capability_rows
        .iter()
        .filter_map(|(id, rows)| rows.health.map(|health| (*id, health)))
        .collect();
    if operating.initialized {
        for (unit_id, kind, owner, _) in units.iter().copied().chain(
            m3.extra_units
                .iter()
                .map(|(id, kind, owner, position)| (*id, kind.0, *owner, *position)),
        ) {
            let entry = types
                .get(kind)
                .ok_or_else(|| error("placements", "unit type is not registered".into()))?;
            let content = ContentId::parse(&entry.content_id)
                .map_err(|message| error("placements", message))?;
            let definition = &catalog.units[&content];
            let health = initial_health.get(&unit_id).copied();
            if health == Some(0) {
                continue;
            }
            let activation = operating
                .prepare_activation(definition, owner)
                .map_err(|reason| {
                    error(
                        "placements",
                        format!(
                            "operating resources of `{content}` cannot be admitted: {reason:?}"
                        ),
                    )
                })?;
            operating.apply_activation(unit_id, activation);
            if let Some(income) = &definition.income {
                let clock =
                    income::prepare_activation(owner, income, income_periods.0[&content], 0)
                        .map_err(|reason| {
                            error(
                                "placements",
                                format!("income of `{content}` cannot activate: {reason:?}"),
                            )
                        })?;
                income_clocks.push((unit_id, clock));
            }
        }
    }
    operating.begin_tick();
    let operating = operating.initialized.then_some(operating);
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
        operating,
        rate_q,
        income_periods,
        income_clocks,
    })
}

/// Commit a fully validated package. This operation has no fallible steps.
pub fn install_package(world: &mut World, staged: StagedPackage) {
    world.remove_resource::<crate::economy::operating::OperatingHolds>();
    world.remove_resource::<OperatingResources>();
    income::clear(world);
    world.insert_resource(RateQ(staged.rate_q));
    world.insert_resource(staged.income_periods);
    if let Some(operating) = staged.operating {
        world.insert_resource(operating);
    }
    world.insert_resource(crate::economy::construction::ConstructionState::default());
    world.remove_resource::<crate::economy::construction::ConstructionLimit>();
    world.remove_resource::<crate::outcome::MatchRules>();
    world.remove_resource::<crate::outcome::MatchOutcomes>();
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
        let order_capable = crate::orders::unit_order_capable(world, kind);
        let mut entity = world.spawn((
            UnitId(id),
            UnitType(kind),
            Owner(SlotId(owner)),
            Position(position),
        ));
        if order_capable {
            crate::orders::writer::insert_default_unit_orders(&mut entity);
        }
        if let Some(facing) = staged.m3.facings.get(&id).copied() {
            entity.insert(UnitFacing(facing));
        }
    }
    for (id, kind, owner, position) in staged.m3.extra_units {
        let order_capable = crate::orders::unit_order_capable(world, kind.0);
        let mut entity = world.spawn((UnitId(id), kind, Owner(SlotId(owner)), Position(position)));
        if order_capable {
            crate::orders::writer::insert_default_unit_orders(&mut entity);
        }
        if let Some(facing) = staged.m3.facings.get(&id).copied() {
            entity.insert(UnitFacing(facing));
        }
    }
    for (id, rows) in staged.m3.capability_rows {
        boot_m3::apply_capability_rows(world, id, rows);
    }
    for (id, clock) in staged.income_clocks {
        income::on_activate(world, id, Some(clock));
    }
}

#[cfg(test)]
mod operating_tests {
    use super::*;
    use crate::economy::{power, views};
    use turts_mod_api::{
        ConstructProgress, HealthDef, IncomeDef, MapSize, Placement, ResolvedConstructor,
        ResolvedProducer, ResourceDef, ShortEffect, SupplyAmount, SupplyCounts, SupplyScope,
        WhenShort, WorldPos,
    };

    fn cid(value: &str) -> ContentId {
        ContentId::parse(value).unwrap()
    }
    fn definition(name: &str) -> turts_mod_api::ResolvedUnit {
        turts_mod_api::ResolvedUnit {
            id: cid(name),
            name: name.into(),
            footprint: Footprint::Circle { radius: 0.5 },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            constructor: None,
            placement: None,
            build_area: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: Some(HealthDef { max: 100 }),
            weapon: None,
            removal: None,
            provides: vec![],
            consumes: vec![],
            income: None,
            armour: None,
            acquire: None,
        }
    }
    fn fixture() -> (ContentCatalog, SourceDef<MapDef>) {
        let mut provider = definition("operating:provider");
        provider.provides = vec![SupplyAmount {
            resource: "operating:power".into(),
            amount: 10,
        }];
        provider.income = Some(IncomeDef {
            resource: "operating:credits".into(),
            amount: 3,
            period_ms: 1050,
        });
        let mut consumer = definition("operating:consumer");
        consumer.consumes = vec![SupplyAmount {
            resource: "operating:power".into(),
            amount: 20,
        }];
        let short = |denominator| WhenShort {
            resource: "operating:power".into(),
            effect: ShortEffect::Rate(turts_mod_api::Ratio {
                numerator: 1,
                denominator,
            }),
        };
        consumer.producer = Some(ResolvedProducer {
            recipes: vec![],
            queue_capacity: 1,
            spawn_offsets: vec![WorldPos { x: 2.0, y: 0.0 }],
            when_short: vec![short(2)],
        });
        consumer.constructor = Some(ResolvedConstructor {
            buildings: vec![],
            concurrent_sites: 1,
            queue_capacity: 1,
            progress: ConstructProgress::Autonomous,
            when_short: vec![short(3)],
        });
        let resources = [
            (
                "operating:power",
                ResourceKind::Supply {
                    scope: SupplyScope::Slot,
                    counts: SupplyCounts::Completed,
                },
            ),
            ("operating:credits", ResourceKind::Balance),
        ]
        .into_iter()
        .map(|(name, kind)| {
            (
                cid(name),
                ResourceDef {
                    id: name.rsplit(':').next().unwrap().into(),
                    name: name.into(),
                    unit_name: String::new(),
                    kind,
                },
            )
        })
        .collect();
        let catalog = ContentCatalog {
            combat: None,
            resources,
            units: [provider, consumer]
                .into_iter()
                .map(|unit| (unit.id.clone(), unit))
                .collect(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: BTreeMap::new(),
            source_files: BTreeMap::from([
                (cid("operating:provider"), "units/provider.ron".into()),
                (cid("operating:consumer"), "units/consumer.ron".into()),
            ]),
        };
        let placement = |unit: &str, x: f32| Placement {
            unit: unit.into(),
            owner_slot: 0,
            pos: WorldPos { x, y: 8.0 },
            contents: vec![],
            facing: None,
            start_order: None,
        };
        let map = SourceDef {
            file: "maps/field.ron".into(),
            value: MapDef {
                id: "field".into(),
                size: MapSize { x: 40.0, y: 30.0 },
                placements: vec![
                    placement("operating:provider", 8.0),
                    placement("operating:consumer", 15.0),
                ],
                nodes: vec![],
                slot_economy: vec![],
                belts: vec![],
                environment: None,
                ground: None,
            },
        };
        (catalog, map)
    }

    #[test]
    fn staging_is_atomic_and_installs_tick_zero_clocks_and_exact_capability_rates() {
        let mut app = crate::headless_app(20);
        let (catalog, map) = fixture();
        let before = views::audit_state(app.world_mut());
        let staged = stage_package(app.world(), catalog, map, 20).unwrap();
        assert_eq!(views::audit_state(app.world_mut()), before);
        install_package(app.world_mut(), staged);
        assert_eq!(app.world().resource::<RateQ>().0, 6);
        assert_eq!(
            app.world().resource::<IncomePeriods>().0[&cid("operating:provider")],
            21
        );
        let clocks = income::clock_audit(app.world());
        assert_eq!(clocks.len(), 1);
        assert_eq!((clocks[0].activation_tick, clocks[0].next_due), (0, 21));
        let view = views::world_view(app.world_mut());
        let operating = view.operating.unwrap();
        assert_eq!(operating.rate_q, 6);
        assert_eq!(
            operating
                .rates
                .iter()
                .find(|row| row.unit == 2)
                .unwrap()
                .producer,
            (1, 2)
        );
        assert_eq!(
            operating
                .rates
                .iter()
                .find(|row| row.unit == 2)
                .unwrap()
                .constructor,
            (1, 3)
        );
        assert!(
            operating
                .supply
                .iter()
                .any(|row| row.slot == 0 && row.provided == 10 && row.consumed == 20)
        );
        assert!(
            operating
                .supply
                .iter()
                .any(|row| row.slot == 1 && row.provided == 0 && row.consumed == 0)
        );
        for tick in 1..=20 {
            app.world_mut().resource_mut::<crate::world::Tick>().0 = tick;
            income::run_income(app.world_mut());
        }
        assert_eq!(
            app.world().resource::<EconomyLedger>().balances[&(0, cid("operating:credits"))]
                .on_hand,
            0
        );
        app.world_mut().resource_mut::<crate::world::Tick>().0 = 21;
        income::run_income(app.world_mut());
        assert_eq!(
            app.world().resource::<EconomyLedger>().balances[&(0, cid("operating:credits"))]
                .on_hand,
            3
        );
        let audit = views::audit_state(app.world_mut());
        assert_eq!(audit.raw_income_clocks[0].next_due, 42);
        assert_eq!(audit.raw_operating.unwrap().active.len(), 2);
    }

    #[test]
    fn boot_supply_overflow_leaves_the_entire_world_unchanged() {
        let mut app = crate::headless_app(20);
        let (mut catalog, mut map) = fixture();
        catalog
            .units
            .get_mut(&cid("operating:provider"))
            .unwrap()
            .provides[0]
            .amount = u64::MAX;
        map.value.placements[1].unit = "operating:provider".into();
        let before = views::audit_state(app.world_mut());
        let error = stage_package(app.world(), catalog, map, 20).unwrap_err();
        assert!(error.message.contains("ArithmeticOverflow"));
        assert_eq!(views::audit_state(app.world_mut()), before);
        assert!(!app.world().contains_resource::<OperatingResources>());
    }

    #[test]
    fn period_zero_is_source_aware_and_maximum_authored_period_is_valid() {
        let app = crate::headless_app(20);
        let (mut catalog, map) = fixture();
        catalog
            .units
            .get_mut(&cid("operating:provider"))
            .unwrap()
            .income
            .as_mut()
            .unwrap()
            .period_ms = 0;
        let error = stage_package(app.world(), catalog.clone(), map.clone(), 20).unwrap_err();
        assert_eq!(error.field_path, "income.period_ms");
        assert_eq!(error.file.as_deref(), Some("units/provider.ron"));
        catalog
            .units
            .get_mut(&cid("operating:provider"))
            .unwrap()
            .income
            .as_mut()
            .unwrap()
            .period_ms = u32::MAX;
        let staged = stage_package(app.world(), catalog, map, u16::MAX).unwrap();
        assert_eq!(
            staged.income_periods.0[&cid("operating:provider")],
            (u64::from(u32::MAX) * u64::from(u16::MAX)).div_ceil(1000)
        );
        assert!(boot_m3::try_ms_to_ticks(u64::MAX, u32::MAX).is_none());
    }

    #[test]
    fn merged_denominator_error_names_the_library_source_before_world_mutation() {
        let mut app = crate::headless_app(20);
        let (mut catalog, map) = fixture();
        let consumer = catalog.units.get_mut(&cid("operating:consumer")).unwrap();
        consumer.producer.as_mut().unwrap().when_short[0].effect =
            ShortEffect::Rate(turts_mod_api::Ratio {
                numerator: 1,
                denominator: 257,
            });
        consumer.constructor.as_mut().unwrap().when_short.clear();
        let mut library = consumer.clone();
        library.id = cid("library:unit");
        library.producer.as_mut().unwrap().when_short[0].effect =
            ShortEffect::Rate(turts_mod_api::Ratio {
                numerator: 1,
                denominator: 263,
            });
        catalog
            .source_files
            .insert(library.id.clone(), "units/library.ron".into());
        catalog.units.insert(library.id.clone(), library);
        let before = views::audit_state(app.world_mut());
        let error = stage_package(app.world(), catalog, map, 20).unwrap_err();
        assert!(error.message.contains("65,536"));
        // The merged catalog's final rate, not a package-local subtotal, exceeds the bound.
        assert!(error.field_path.contains("when_short"));
        assert_eq!(error.package, "operating");
        assert_eq!(error.file.as_deref(), Some("units/consumer.ron"));
        assert_eq!(views::audit_state(app.world_mut()), before);
    }

    #[test]
    fn scaled_work_duration_overflow_is_refused_before_package_commit() {
        let mut app = crate::headless_app(20);
        let (mut catalog, map) = fixture();
        let consumer = catalog.units.get_mut(&cid("operating:consumer")).unwrap();
        consumer.producer.as_mut().unwrap().when_short[0].effect =
            ShortEffect::Rate(turts_mod_api::Ratio {
                numerator: 1,
                denominator: 65_536,
            });
        consumer.constructor.as_mut().unwrap().when_short.clear();
        let recipe = cid("operating:slow");
        catalog
            .source_files
            .insert(recipe.clone(), "production/slow.ron".into());
        catalog.production.insert(
            recipe.clone(),
            turts_mod_api::ResolvedProduction {
                id: recipe,
                name: "Slow".into(),
                output_unit: cid("operating:provider"),
                duration_ms: u64::MAX,
                costs: vec![],
                payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
                cancel_refunds: turts_mod_api::CancelRefunds {
                    queued: turts_mod_api::RefundRatio::ZERO,
                    running: turts_mod_api::RefundRatio::ZERO,
                    output_blocked: turts_mod_api::RefundRatio::ZERO,
                    destroyed: turts_mod_api::RefundRatio::ZERO,
                },
            },
        );
        let before = views::audit_state(app.world_mut());
        let error = stage_package(app.world(), catalog, map, 20).unwrap_err();
        assert_eq!(error.field_path, "duration_ms");
        assert_eq!(error.file.as_deref(), Some("production/slow.ron"));
        assert!(error.message.contains("RateQ"));
        assert_eq!(views::audit_state(app.world_mut()), before);
    }

    #[test]
    fn retained_site_rules_match_world_and_view_after_constructor_removal() {
        let mut app = crate::headless_app(20);
        let (mut catalog, mut map) = fixture();
        let constructor = catalog.units.get_mut(&cid("operating:consumer")).unwrap();
        let rules = constructor.constructor.as_ref().unwrap().when_short.clone();
        constructor.consumes.clear();
        let mut demand = definition("operating:demand");
        demand.consumes = vec![SupplyAmount {
            resource: "operating:power".into(),
            amount: 20,
        }];
        catalog.units.insert(demand.id.clone(), demand);
        map.value.placements.push(Placement {
            unit: "operating:demand".into(),
            owner_slot: 0,
            pos: WorldPos { x: 22.0, y: 8.0 },
            contents: vec![],
            facing: None,
            start_order: None,
        });
        let staged = stage_package(app.world(), catalog, map, 20).unwrap();
        install_package(app.world_mut(), staged);
        let source = crate::unit_entity(app.world(), 2).unwrap();
        power::remove_active(app.world_mut(), 2);
        app.world_mut().despawn(source);
        app.world_mut()
            .resource_mut::<crate::economy::combat::Healths>()
            .0
            .remove(&2);
        power::run_supply(app.world_mut());
        let view = views::world_view(app.world_mut());
        assert_eq!(
            power::rate_for_rules(app.world(), 0, &rules),
            turts_rules::Ratio::new(1, 3).unwrap()
        );
        assert_eq!(
            power::rate_for_rules_in_view(&view, 0, &rules),
            turts_rules::Ratio::new(1, 3).unwrap()
        );
        assert_eq!(
            power::rate_for_rules_in_view(&view, 1, &rules),
            turts_rules::Ratio::ONE
        );
    }

    #[test]
    fn holds_publish_owner_and_first_tick_once_then_clear_once() {
        use crate::economy::{operating, transaction::AuditTrail};
        let mut app = crate::headless_app(20);
        let (catalog, map) = fixture();
        let staged = stage_package(app.world(), catalog, map, 20).unwrap();
        install_package(app.world_mut(), staged);
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let empty = serde_json::to_value(views::audit_state(app.world_mut())).unwrap();
        assert!(empty.get("raw_operating_holds").is_none());
        assert!(empty["economy"]["operating"].get("holds").is_none());
        app.world_mut().resource_mut::<crate::Tick>().0 = 5;
        operating::hold(app.world_mut(), 2, crate::RejectReason::ArithmeticOverflow);
        assert_eq!(app.world().resource::<AuditTrail>().operations.len(), 1);
        let state = views::audit_state(app.world_mut());
        assert_eq!(state.raw_operating_holds.len(), 1);
        assert_eq!(state.raw_operating_holds[0].owner_slot, 0);
        assert_eq!(state.raw_operating_holds[0].since_tick, 5);
        assert_eq!(
            state.economy.operating.as_ref().unwrap().holds,
            state.raw_operating_holds
        );
        app.world_mut().resource_mut::<crate::Tick>().0 = 9;
        operating::hold(app.world_mut(), 2, crate::RejectReason::ArithmeticOverflow);
        assert_eq!(app.world().resource::<AuditTrail>().operations.len(), 1);
        assert_eq!(operating::holds(app.world())[0].since_tick, 5);
        operating::clear_hold(app.world_mut(), 2);
        operating::clear_hold(app.world_mut(), 2);
        assert_eq!(app.world().resource::<AuditTrail>().operations.len(), 2);
        let state = views::audit_state(app.world_mut());
        assert!(state.raw_operating_holds.is_empty());
        assert!(state.economy.operating.as_ref().unwrap().holds.is_empty());
        let last = app
            .world()
            .resource::<AuditTrail>()
            .operations
            .last()
            .unwrap();
        assert_eq!(last.before.raw_operating_holds.len(), 1);
        assert!(last.after.raw_operating_holds.is_empty());
    }

    #[test]
    fn no_operating_install_clears_old_supply_and_income_without_audit_fields() {
        let mut app = crate::headless_app(20);
        let (catalog, map) = fixture();
        let state = OperatingResources::from_catalog(&catalog, 2);
        app.world_mut().insert_resource(state);
        let clock = income::prepare_activation(
            0,
            catalog.units[&cid("operating:provider")]
                .income
                .as_ref()
                .unwrap(),
            21,
            0,
        )
        .unwrap();
        income::on_activate(app.world_mut(), 99, Some(clock));
        app.world_mut()
            .insert_resource(crate::economy::operating::OperatingHolds(BTreeMap::from([
                (
                    99,
                    crate::economy::operating::OperatingHold {
                        unit: 99,
                        owner_slot: 0,
                        reason: crate::RejectReason::ArithmeticOverflow,
                        since_tick: 4,
                    },
                ),
            ])));
        let mut catalog = catalog;
        for unit in catalog.units.values_mut() {
            unit.provides.clear();
            unit.consumes.clear();
            unit.income = None;
            if let Some(producer) = unit.producer.as_mut() {
                producer.when_short.clear()
            }
            if let Some(constructor) = unit.constructor.as_mut() {
                constructor.when_short.clear()
            }
        }
        catalog
            .resources
            .retain(|_, resource| resource.kind == ResourceKind::Balance);
        let staged = stage_package(app.world(), catalog, map, 20).unwrap();
        install_package(app.world_mut(), staged);
        assert!(!app.world().contains_resource::<OperatingResources>());
        assert!(income::clock_audit(app.world()).is_empty());
        assert!(crate::economy::operating::holds(app.world()).is_empty());
        assert_eq!(app.world().resource::<RateQ>().0, 1);
        let json = serde_json::to_value(views::audit_state(app.world_mut())).unwrap();
        assert!(json.get("raw_operating").is_none());
        assert!(json.get("raw_income_clocks").is_none());
        assert!(json.get("raw_operating_holds").is_none());
        assert!(json["economy"].get("operating").is_none());
    }
}
