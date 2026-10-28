mod common;

use bevy_app::App;
use common::accounting::{AccountingOracle, ExpectedOperation, advance_accounted_tick, trail};
use std::path::PathBuf;
use turts_mod_api::{
    ContentCatalog, ContentId, CostSource, Facing, PaymentPolicy, ResolvedDropOffEffect,
};
use turts_mod_runtime::archive::PackageArchive;
use turts_pack::{load_package, pack_checked};
use turts_sim::{
    Applied, AuditState, Command, CommandKind, CommandQueue, CommittedTick, RejectReason, SlotId,
    UnitState, Vec2, apply_command, audit_state, headless_app, snapshot,
};

fn cid(id: &str) -> ContentId {
    ContentId::parse(id).expect("valid package fixture content id")
}

fn load_disk_package(id: &str) -> (PackageArchive, App) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(id);
    let bytes = pack_checked(&dir).unwrap_or_else(|error| panic!("pack_checked {id}: {error}"));
    let archive =
        PackageArchive::parse(&bytes).unwrap_or_else(|error| panic!("parse {id}: {error}"));
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive)
        .unwrap_or_else(|error| panic!("public load_package {id}: {error}"));
    trail(&mut app).enable();
    (archive, app)
}

fn installed_catalog(app: &App) -> ContentCatalog {
    app.world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone()
}

fn unit_id(state: &AuditState, content_id: &str, owner_slot: u8) -> u32 {
    state
        .units
        .iter()
        .find(|unit| unit.content_id == content_id && unit.owner_slot == owner_slot)
        .unwrap_or_else(|| panic!("missing {content_id} for slot {owner_slot}"))
        .unit_id
}

fn unit_by_id(state: &AuditState, id: u32) -> &turts_sim::UnitView {
    state
        .units
        .iter()
        .find(|unit| unit.unit_id == id)
        .expect("unit id is present in public audit state")
}

#[test]
fn public_forge_boot_and_remove_retain_authored_facing_and_m2_omission() {
    let (_, mut app) = load_disk_package("forge");
    let boot = snapshot(app.world_mut());

    let arm_loader = boot
        .iter()
        .find(|unit| unit.content_id == "forge:arm_loader" && unit.pos == Vec2::new(17.0, 10.0))
        .expect("authored PosY arm loader is present");
    assert_eq!(arm_loader.facing, Some(Facing::PosY));
    let negative_arm_loader = boot
        .iter()
        .find(|unit| unit.content_id == "forge:arm_loader" && unit.pos == Vec2::new(17.0, 6.0))
        .expect("authored NegY arm loader is present");
    assert_eq!(negative_arm_loader.facing, Some(Facing::NegY));

    let belt_ids: Vec<_> = boot
        .iter()
        .filter(|unit| unit.content_id == "forge:belt")
        .map(|unit| {
            assert_eq!(unit.facing, Some(Facing::PosX));
            unit.unit_id
        })
        .collect();
    assert_eq!(belt_ids.len(), 6, "all six authored belt pieces boot");
    let survivor = *belt_ids.last().expect("belt has a final piece");
    for (seq, unit_id) in belt_ids[..belt_ids.len() - 1].iter().copied().enumerate() {
        let result = apply_command(
            app.world_mut(),
            &command(seq as u32 + 1, CommandKind::Remove { unit_id }),
        );
        assert!(
            result.outcome.is_ok(),
            "remove belt piece {unit_id}: {result:?}"
        );
    }

    let after_remove = snapshot(app.world_mut());
    assert_eq!(
        after_remove
            .iter()
            .filter(|unit| unit.content_id == "forge:belt")
            .count(),
        1
    );
    let remaining = after_remove
        .iter()
        .find(|unit| unit.unit_id == survivor)
        .expect("last belt piece remains after other pieces are removed");
    assert_eq!(remaining.facing, Some(Facing::PosX));

    let legacy_unit = UnitState {
        unit_id: 999,
        content_id: "forge:drone".into(),
        owner_slot: 0,
        facing: None,
        pos: Vec2::new(0.0, 0.0),
    };
    let legacy_json = serde_json::to_value(legacy_unit).expect("serialize legacy unit");
    assert!(
        legacy_json.get("facing").is_none(),
        "M2 JSON omits absent facing"
    );
}

#[test]
fn scheduled_removal_does_not_publish_phantom_enable_and_keeps_real_enable_delta() {
    let (_, mut app) = load_disk_package("forge");
    let boot = snapshot(app.world_mut());
    let disabled_then_removed = boot
        .iter()
        .find(|unit| unit.content_id == "forge:arm_loader" && unit.pos == Vec2::new(17.0, 10.0))
        .expect("removal target exists")
        .unit_id;
    let surviving = boot
        .iter()
        .find(|unit| unit.content_id == "forge:arm_loader" && unit.pos == Vec2::new(17.0, 6.0))
        .expect("live toggle target exists")
        .unit_id;

    {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.0.push_back(command(
            1,
            CommandKind::SetEnabled {
                unit_id: disabled_then_removed,
                enabled: false,
            },
        ));
        queue.0.push_back(command(
            2,
            CommandKind::SetEnabled {
                unit_id: surviving,
                enabled: false,
            },
        ));
    }
    app.update();
    let disabled_tick = app.world().resource::<CommittedTick>();
    assert_eq!(disabled_tick.enabled.len(), 2);
    assert!(disabled_tick.enabled.iter().all(|change| !change.enabled));

    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(command(
            3,
            CommandKind::Remove {
                unit_id: disabled_then_removed,
            },
        ));
    app.update();
    let removed_tick = app.world().resource::<CommittedTick>();
    assert!(removed_tick.removed.contains(&disabled_then_removed));
    assert!(
        !removed_tick
            .enabled
            .iter()
            .any(|change| change.unit == disabled_then_removed),
        "a removed disabled unit must not publish a phantom enable"
    );

    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(command(
            4,
            CommandKind::SetEnabled {
                unit_id: surviving,
                enabled: true,
            },
        ));
    app.update();
    let enabled_tick = app.world().resource::<CommittedTick>();
    assert_eq!(
        enabled_tick.enabled,
        [turts_sim::economy::views_m3::RawEnabledAudit {
            unit: surviving,
            enabled: true,
        }]
    );

    app.update();
    let quiet_tick = app.world().resource::<CommittedTick>();
    assert!(
        quiet_tick.enabled.is_empty(),
        "removed unit emits no later phantom delta"
    );
}

fn balance(state: &AuditState, slot: u8, resource: &ContentId) -> u64 {
    state
        .economy
        .balances
        .iter()
        .find(|entry| entry.slot == slot && &entry.resource == resource)
        .unwrap_or_else(|| panic!("missing slot {slot} balance {resource}"))
        .on_hand
}

fn inventory_item(state: &AuditState, unit: u32, resource: &ContentId) -> (u64, u64) {
    let inventory = state
        .economy
        .inventories
        .iter()
        .find(|entry| entry.unit == unit)
        .unwrap_or_else(|| panic!("missing inventory for unit {unit}"));
    inventory
        .items
        .iter()
        .find(|item| &item.resource == resource)
        .map_or((0, 0), |item| (item.on_hand, item.reserved))
}

fn recipe_for_output(
    catalog: &ContentCatalog,
    producer_id: &ContentId,
    output_id: &ContentId,
) -> ContentId {
    catalog.units[producer_id]
        .producer
        .as_ref()
        .expect("fixture producer capability is catalogued")
        .recipes
        .iter()
        .find(|recipe_id| {
            catalog
                .production
                .get(*recipe_id)
                .is_some_and(|recipe| &recipe.output_unit == output_id)
        })
        .unwrap_or_else(|| panic!("producer {producer_id} has no recipe for {output_id}"))
        .clone()
}

fn command(seq: u32, kind: CommandKind) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind,
    }
}

fn apply_accounted_command(
    app: &mut App,
    oracle: &mut AccountingOracle,
    command: &Command,
) -> turts_sim::CommandResult {
    let before = audit_state(app.world_mut());
    assert_eq!(
        &before,
        oracle.modeled_state(),
        "command begins at modeled state"
    );
    let result = apply_command(app.world_mut(), command);
    let after = audit_state(app.world_mut());
    let records = trail(app).drain_operations();
    assert_eq!(
        records.len(),
        1,
        "each submitted command has one captured operation"
    );
    let operation = &records[0];
    assert_eq!(
        operation.before, before,
        "operation captures command input state"
    );
    assert_eq!(
        operation.after, after,
        "operation captures command result state"
    );
    oracle
        .observe(
            &ExpectedOperation::Command(command.clone()),
            &before,
            &after,
            &operation.events,
            Some(&result),
        )
        .unwrap_or_else(|error| panic!("command {}: {error}", command.seq));
    assert_eq!(oracle.modeled_state(), &after);
    assert!(trail(app).valid, "oracle capture remains valid");
    result
}

fn assert_all_commands_modeled(app: &mut App, oracle: &AccountingOracle) {
    oracle
        .assert_command_records(&trail(app).commands)
        .expect("every submitted input has one matching command audit record");
}

#[test]
fn base_global_balance_loop() {
    let (archive, mut app) = load_disk_package("base");
    assert_eq!(archive.manifest.id, "base");
    let catalog = installed_catalog(&app);
    let command_center = cid("base:command_center");
    let harvester_type = cid("base:harvester");
    let rifleman_type = cid("base:rifleman");
    let harvester_recipe = recipe_for_output(&catalog, &command_center, &harvester_type);
    let rifleman_recipe = recipe_for_output(&catalog, &command_center, &rifleman_type);
    assert_eq!(catalog.production[&harvester_recipe].costs[0].amount, 600);
    assert_eq!(
        catalog.production[&harvester_recipe].costs[0].source,
        CostSource::SlotBalance
    );
    assert_eq!(
        catalog.production[&harvester_recipe].payment,
        PaymentPolicy::ChargeOnQueue
    );
    assert_eq!(catalog.production[&rifleman_recipe].costs[0].amount, 100);

    let initial = audit_state(app.world_mut());
    let initial_units = initial.units.len();
    let initial_harvester_count = initial
        .units
        .iter()
        .filter(|unit| unit.content_id == "base:harvester" && unit.owner_slot == 0)
        .count();
    let producer_id = unit_id(&initial, "base:command_center", 0);
    let harvester_id = unit_id(&initial, "base:harvester", 0);
    let node_id = initial
        .nodes
        .iter()
        .find(|node| node.content_id == cid("base:ore_deposit") && node.pos.x == 24.0)
        .expect("slot-zero node is present in the loaded map")
        .id;
    let credits = cid("base:credits");
    assert_eq!(balance(&initial, 0, &credits), 500);
    assert_eq!(
        initial
            .economy
            .queues
            .iter()
            .map(|q| q.jobs.len())
            .sum::<usize>(),
        0
    );
    assert!(
        initial
            .economy
            .inventories
            .iter()
            .all(|i| i.items.iter().all(|x| x.on_hand == 0))
    );

    let initial_catalog = installed_catalog(&app);
    let mut oracle = AccountingOracle::new(&initial, &initial_catalog);
    let rejected = apply_accounted_command(
        &mut app,
        &mut oracle,
        &command(
            1,
            CommandKind::Produce {
                producer_id,
                recipe_id: harvester_recipe.clone(),
            },
        ),
    );
    assert_eq!(rejected.outcome, Err(RejectReason::InsufficientBalance));
    assert_eq!(balance(oracle.modeled_state(), 0, &credits), 500);

    let gathered = apply_accounted_command(
        &mut app,
        &mut oracle,
        &command(
            2,
            CommandKind::Gather {
                unit_id: harvester_id,
                node_id,
            },
        ),
    );
    assert_eq!(gathered.outcome, Ok(Applied { job_id: None }));
    for _ in 0..1200 {
        if balance(oracle.modeled_state(), 0, &credits) >= 600 {
            break;
        }
        let next_tick = oracle.modeled_state().economy.tick + 1;
        advance_accounted_tick(&mut app, &mut oracle, next_tick);
    }
    assert!(
        oracle.extraction > 0,
        "finite ore node extraction was observed"
    );
    assert!(oracle.delivery > 0, "ore reached a receiver");
    assert!(oracle.conversion > 0, "ore converted to global credits");
    assert!(balance(oracle.modeled_state(), 0, &credits) >= 600);

    let available = balance(oracle.modeled_state(), 0, &credits);
    let queued = apply_accounted_command(
        &mut app,
        &mut oracle,
        &command(
            3,
            CommandKind::Produce {
                producer_id,
                recipe_id: harvester_recipe,
            },
        ),
    );
    assert!(queued.outcome.is_ok());
    assert_eq!(
        balance(oracle.modeled_state(), 0, &credits),
        available - 600,
        "ChargeOnQueue debits exactly the declared balance cost"
    );
    for _ in 0..1200 {
        if oracle.outputs == 1 {
            break;
        }
        let next_tick = oracle.modeled_state().economy.tick + 1;
        advance_accounted_tick(&mut app, &mut oracle, next_tick);
    }
    assert_eq!(
        oracle.outputs, 1,
        "exactly one independent output completed"
    );
    let final_state = audit_state(app.world_mut());
    assert_eq!(final_state.units.len(), initial_units + 1);
    assert_eq!(
        final_state
            .units
            .iter()
            .filter(|unit| unit.content_id == "base:harvester" && unit.owner_slot == 0)
            .count(),
        initial_harvester_count + 1
    );
    assert_eq!(
        balance(&final_state, 1, &credits),
        500,
        "slot one retains its independent seed balance"
    );
    assert!(oracle.assert_positive_progress().is_ok());
    assert_eq!(&final_state, oracle.modeled_state());
    assert_all_commands_modeled(&mut app, &oracle);
}

#[test]
fn haul_local_item_loop() {
    let (archive, mut app) = load_disk_package("haul");
    assert_eq!(archive.manifest.id, "haul");
    let catalog = installed_catalog(&app);
    let depot_type = cid("haul:depot");
    let worker_type = cid("haul:worker");
    let worker_recipe = recipe_for_output(&catalog, &depot_type, &worker_type);
    let recipe = &catalog.production[&worker_recipe];
    let depot = &catalog.units[&depot_type];
    assert!(matches!(
        depot.drop_off.as_ref().unwrap().rules[0].effect,
        ResolvedDropOffEffect::Store
    ));
    assert_eq!(recipe.duration_ms, 6000);
    assert_eq!(recipe.costs[0].source, CostSource::ProducerInventory);
    assert_eq!(recipe.costs[0].resource, cid("haul:ore"));
    assert_eq!(recipe.costs[0].amount, 50);
    assert_eq!(recipe.payment, PaymentPolicy::ReserveThenCharge);

    let initial = audit_state(app.world_mut());
    let depot_id = unit_id(&initial, "haul:depot", 0);
    let worker_id = unit_id(&initial, "haul:worker", 0);
    let node_id = initial
        .nodes
        .iter()
        .find(|node| node.content_id == cid("haul:ore_deposit") && node.pos.x == 20.0)
        .expect("slot-zero haul node is present")
        .id;
    let ore = cid("haul:ore");
    assert!(
        initial.economy.balances.is_empty(),
        "haul defines no global Balance accounts"
    );
    let initial_depot_stock = inventory_item(&initial, depot_id, &ore).0;
    assert_eq!(initial_depot_stock, 100);
    let initial_worker_count = initial
        .units
        .iter()
        .filter(|unit| unit.content_id == "haul:worker" && unit.owner_slot == 0)
        .count();
    let mut oracle = AccountingOracle::new(&initial, &catalog);

    let gather = apply_accounted_command(
        &mut app,
        &mut oracle,
        &command(
            1,
            CommandKind::Gather {
                unit_id: worker_id,
                node_id,
            },
        ),
    );
    assert_eq!(gather.outcome, Ok(Applied { job_id: None }));
    for _ in 0..1200 {
        if oracle.delivery >= 50 {
            break;
        }
        let next_tick = oracle.modeled_state().economy.tick + 1;
        advance_accounted_tick(&mut app, &mut oracle, next_tick);
    }
    assert!(oracle.extraction > 0, "worker extracted finite node stock");
    assert!(
        oracle.delivery >= 50,
        "positive Store delivery reached local inventory"
    );
    assert!(inventory_item(oracle.modeled_state(), depot_id, &ore).0 > initial_depot_stock);

    let before_payment = inventory_item(oracle.modeled_state(), depot_id, &ore).0;
    let queued = apply_accounted_command(
        &mut app,
        &mut oracle,
        &command(
            2,
            CommandKind::Produce {
                producer_id: depot_id,
                recipe_id: worker_recipe,
            },
        ),
    );
    assert!(queued.outcome.is_ok());
    assert_eq!(
        inventory_item(oracle.modeled_state(), depot_id, &ore),
        (before_payment, 50),
        "ReserveThenCharge claims exactly the declared 50 ore when queued"
    );
    let next_tick = oracle.modeled_state().economy.tick + 1;
    advance_accounted_tick(&mut app, &mut oracle, next_tick);
    assert_eq!(
        inventory_item(oracle.modeled_state(), depot_id, &ore),
        (before_payment - 50, 0),
        "starting the worker job charges the 50 reserved local ore"
    );
    for _ in 0..600 {
        if oracle.outputs == 1 {
            break;
        }
        let next_tick = oracle.modeled_state().economy.tick + 1;
        advance_accounted_tick(&mut app, &mut oracle, next_tick);
    }
    assert_eq!(oracle.outputs, 1);
    let final_state = audit_state(app.world_mut());
    assert_eq!(
        final_state
            .units
            .iter()
            .filter(|unit| unit.content_id == "haul:worker" && unit.owner_slot == 0)
            .count(),
        initial_worker_count + 1
    );
    assert!(oracle.assert_positive_progress().is_ok());
    assert_eq!(&final_state, oracle.modeled_state());
    assert_all_commands_modeled(&mut app, &oracle);
}

#[test]
fn acme_has_no_economy() {
    let (archive, mut app) = load_disk_package("acme");
    assert_eq!(archive.manifest.id, "acme");
    assert!(archive.resources().unwrap().is_empty());
    assert!(archive.production().unwrap().is_empty());
    assert!(archive.maps().unwrap()[0].1.slot_economy.is_empty());
    let catalog = installed_catalog(&app);
    assert_eq!(catalog.units.len(), 1);
    let scout = catalog
        .units
        .get(&cid("acme:scout"))
        .expect("public scout type");
    assert_eq!(scout.movement.as_ref().unwrap().speed, 6.0);
    assert!(scout.inventory.is_none());
    assert!(scout.gatherer.is_none());
    assert!(scout.drop_off.is_none());
    assert!(scout.producer.is_none());

    let initial = audit_state(app.world_mut());
    assert!(initial.economy.balances.is_empty());
    assert!(initial.economy.inventories.is_empty());
    assert!(initial.economy.queues.is_empty());
    assert!(initial.economy.orders.is_empty());
    let scout_id = unit_id(&initial, "acme:scout", 0);
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let result = apply_accounted_command(
        &mut app,
        &mut oracle,
        &command(
            1,
            CommandKind::MoveTo {
                unit_ids: vec![scout_id],
                target: Vec2::new(40.0, 4.0),
            },
        ),
    );
    assert!(
        result.outcome.is_ok(),
        "the existing mobile scout command still works"
    );
    for _ in 0..160 {
        if unit_by_id(oracle.modeled_state(), scout_id).pos
            == (turts_mod_api::WorldPos { x: 40.0, y: 4.0 })
        {
            break;
        }
        let next_tick = oracle.modeled_state().economy.tick + 1;
        advance_accounted_tick(&mut app, &mut oracle, next_tick);
    }
    let final_state = audit_state(app.world_mut());
    assert_eq!(
        unit_by_id(&final_state, scout_id).pos,
        turts_mod_api::WorldPos { x: 40.0, y: 4.0 }
    );
    assert_eq!(&final_state, oracle.modeled_state());
    assert_all_commands_modeled(&mut app, &oracle);
}

#[test]
fn fixture_geometry_and_two_slots_match_design() {
    let (base_archive, base_app) = load_disk_package("base");
    let base_map = &base_archive.maps().unwrap()[0].1;
    assert_eq!(base_map.size, turts_mod_api::MapSize { x: 64.0, y: 64.0 });
    assert_eq!(base_map.slot_economy.len(), 2);
    assert_eq!(
        base_map
            .slot_economy
            .iter()
            .map(|slot| (slot.slot, slot.balances[0].amount))
            .collect::<Vec<_>>(),
        [(0, 500), (1, 500)]
    );
    assert_eq!(
        base_map
            .nodes
            .iter()
            .map(|node| node.pos.x)
            .collect::<Vec<_>>(),
        [24.0, 40.0]
    );
    for (owner_slot, x) in [(0, 8.0), (1, 56.0)] {
        assert!(base_map.placements.iter().any(|placed| {
            placed.unit == "base:command_center"
                && placed.owner_slot == owner_slot
                && placed.pos.x == x
        }));
    }
    let base = installed_catalog(&base_app);
    let command_center = &base.units[&cid("base:command_center")];
    assert_eq!(
        command_center
            .producer
            .as_ref()
            .unwrap()
            .spawn_offsets
            .len(),
        2
    );
    assert_ne!(
        command_center.producer.as_ref().unwrap().spawn_offsets[0],
        command_center.producer.as_ref().unwrap().spawn_offsets[1]
    );
    let harvester = &base.units[&cid("base:harvester")];
    let gatherer = harvester.gatherer.as_ref().unwrap();
    assert_eq!(gatherer.gather_per_second, 20);
    assert_eq!(gatherer.unload_per_second, 50);
    assert_eq!(gatherer.interaction_range, 0.5);

    let (haul_archive, haul_app) = load_disk_package("haul");
    let haul_map = &haul_archive.maps().unwrap()[0].1;
    assert_eq!(haul_map.size, turts_mod_api::MapSize { x: 48.0, y: 48.0 });
    assert!(haul_map.slot_economy.is_empty());
    assert_eq!(
        haul_map
            .nodes
            .iter()
            .map(|node| node.pos.x)
            .collect::<Vec<_>>(),
        [20.0, 28.0]
    );
    assert_eq!(
        haul_map
            .placements
            .iter()
            .filter(|placed| placed.owner_slot == 0 && placed.unit == "haul:depot")
            .count(),
        1
    );
    assert_eq!(
        haul_map
            .placements
            .iter()
            .filter(|placed| placed.owner_slot == 1 && placed.unit == "haul:depot")
            .count(),
        1
    );
    assert!(haul_map.placements.iter().any(|placed| {
        placed.unit == "haul:depot" && placed.owner_slot == 0 && placed.pos.x == 8.0
    }));
    assert!(haul_map.placements.iter().any(|placed| {
        placed.unit == "haul:depot" && placed.owner_slot == 1 && placed.pos.x == 40.0
    }));
    let haul = installed_catalog(&haul_app);
    let depot = &haul.units[&cid("haul:depot")];
    assert_eq!(depot.producer.as_ref().unwrap().spawn_offsets.len(), 2);
    assert_ne!(
        depot.producer.as_ref().unwrap().spawn_offsets[0],
        depot.producer.as_ref().unwrap().spawn_offsets[1]
    );
    let worker = &haul.units[&cid("haul:worker")];
    let gatherer = worker.gatherer.as_ref().unwrap();
    assert_eq!(gatherer.gather_per_second, 5);
    assert_eq!(gatherer.unload_per_second, 10);
    assert_eq!(gatherer.interaction_range, 0.5);
}
