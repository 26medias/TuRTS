use std::path::PathBuf;

use turts_mod_api::{
    CancelRefunds, ContentId, Cost, CostSource, DropOffDef, DropOffEffect, DropOffRule,
    ExecutionContext, Exports, Footprint, GathererDef, InventoryDef, ItemStack, MapDef, MapSize,
    MovementDef, NodeDef, NodePlacement, PackageKind, PaymentPolicy, Placement, ProducerDef,
    ProductionDef, RefundRatio, ResolvedDropOffEffect, ResourceDef, ResourceKind, Shape, UniqueMap,
    UnitDef, VisualDef, VisualTarget, WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir, visuals::visuals_by_content};
use turts_pack::load_package;
use turts_sim::{AuditState, UnitView, audit_state, headless_app};

fn package_dir(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(id)
}

fn boot(id: &str) -> (PackageArchive, AuditState) {
    let bytes = pack_dir(&package_dir(id)).unwrap_or_else(|error| panic!("pack {id}: {error}"));
    let archive = PackageArchive::parse(&bytes).unwrap();
    visuals_by_content(std::slice::from_ref(&archive))
        .unwrap_or_else(|error| panic!("visuals {id}: {error}"));
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).unwrap_or_else(|error| panic!("boot {id}: {error}"));
    let state = audit_state(app.world_mut());
    (archive, state)
}

#[test]
fn all_four_source_packages_pass_public_visual_validation() {
    for id in ["acme", "base", "cedar", "haul"] {
        let bytes = pack_dir(&package_dir(id)).unwrap_or_else(|error| panic!("pack {id}: {error}"));
        let archive =
            PackageArchive::parse(&bytes).unwrap_or_else(|error| panic!("parse {id}: {error}"));
        visuals_by_content(std::slice::from_ref(&archive))
            .unwrap_or_else(|error| panic!("visuals {id}: {error}"));
    }
}

fn cid(id: &str) -> ContentId {
    ContentId::parse(id).unwrap()
}

fn owner_unit_counts(state: &AuditState) -> Vec<(u8, usize)> {
    [0, 1]
        .into_iter()
        .map(|slot| {
            (
                slot,
                state
                    .units
                    .iter()
                    .filter(|unit| unit.owner_slot == slot)
                    .count(),
            )
        })
        .collect()
}

fn files_below(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_below(root, &path, out);
        } else {
            out.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

fn assert_placed_units(state: &AuditState, expected: &[(&str, u8, f32, f32)]) {
    let mut actual = state
        .units
        .iter()
        .map(|unit| {
            (
                unit.content_id.as_str(),
                unit.owner_slot,
                unit.pos.x,
                unit.pos.y,
            )
        })
        .collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(&b.1)).then(a.2.total_cmp(&b.2)));
    expected.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(&b.1)).then(a.2.total_cmp(&b.2)));
    assert_eq!(actual, expected);
}

fn placed_unit<'a>(state: &'a AuditState, id: &str, slot: u8, x: f32, y: f32) -> &'a UnitView {
    state
        .units
        .iter()
        .find(|unit| {
            unit.content_id == id && unit.owner_slot == slot && unit.pos.x == x && unit.pos.y == y
        })
        .unwrap_or_else(|| panic!("missing {id} for slot {slot} at ({x}, {y})"))
}

fn assert_inventory(
    state: &AuditState,
    unit: &UnitView,
    capacity: u64,
    expected_items: &[(&str, u64)],
) {
    let inventory = state
        .economy
        .inventories
        .iter()
        .find(|inventory| inventory.unit == unit.unit_id)
        .unwrap_or_else(|| panic!("missing inventory for {}", unit.content_id));
    assert_eq!(inventory.owner_slot, unit.owner_slot);
    assert_eq!(inventory.capacity, capacity);
    let actual = inventory
        .items
        .iter()
        .map(|item| (item.resource.to_string(), item.on_hand, item.reserved))
        .collect::<Vec<_>>();
    let expected = expected_items
        .iter()
        .map(|(resource, amount)| ((*resource).to_owned(), *amount, 0))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn base_boots_both_slots_with_converting_credit_economy() {
    let (archive, state) = boot("base");
    assert_eq!(archive.manifest.version, "0.2.0");
    let resources = archive.resources().unwrap();
    assert_eq!(resources.len(), 2);
    assert_eq!(
        resources
            .iter()
            .map(|(_, resource)| (&*resource.id, resource.kind))
            .collect::<Vec<_>>(),
        [
            ("ore", ResourceKind::Item),
            ("credits", ResourceKind::Balance)
        ]
    );
    assert_eq!(resources[0].1.name, "Ore");
    assert_eq!(resources[0].1.unit_name, "ore");
    assert_eq!(resources[1].1.name, "Credits");
    assert_eq!(resources[1].1.unit_name, "credit");
    let catalog = archive.content().unwrap();
    assert_eq!(catalog.units.len(), 3);
    assert_eq!(catalog.nodes.len(), 1);
    assert_eq!(catalog.production.len(), 2);
    let command_center = &catalog.units[&cid("base:command_center")];
    assert!(command_center.movement.is_none());
    assert!(command_center.inventory.is_none());
    assert!(command_center.gatherer.is_none());
    let receiver = &command_center.drop_off.as_ref().unwrap().rules[0];
    assert_eq!(receiver.item, cid("base:ore"));
    assert!(matches!(
        receiver.effect,
        ResolvedDropOffEffect::Convert { ref balance, per_item: 1 }
            if balance == &cid("base:credits")
    ));
    assert_eq!(command_center.producer.as_ref().unwrap().queue_capacity, 5);
    assert_eq!(
        command_center.producer.as_ref().unwrap().recipes,
        [cid("base:train_rifleman"), cid("base:build_harvester")]
    );
    assert_eq!(
        command_center.producer.as_ref().unwrap().spawn_offsets,
        [WorldPos { x: 0.0, y: 4.0 }, WorldPos { x: 0.0, y: -4.0 }]
    );
    let harvester = &catalog.units[&cid("base:harvester")];
    assert_eq!(harvester.movement.as_ref().unwrap().speed, 2.0);
    assert_eq!(harvester.inventory.as_ref().unwrap().capacity, 100);
    assert_eq!(harvester.gatherer.as_ref().unwrap().gather_per_second, 20);
    assert_eq!(harvester.gatherer.as_ref().unwrap().unload_per_second, 50);
    assert_eq!(harvester.gatherer.as_ref().unwrap().interaction_range, 0.5);
    assert_eq!(
        harvester.gatherer.as_ref().unwrap().items,
        [cid("base:ore")]
    );
    assert!(harvester.drop_off.is_none());
    assert!(harvester.producer.is_none());
    let rifleman_unit = &catalog.units[&cid("base:rifleman")];
    assert_eq!(rifleman_unit.footprint, Footprint::Circle { radius: 0.4 });
    assert_eq!(rifleman_unit.movement.as_ref().unwrap().speed, 3.0);
    assert!(rifleman_unit.inventory.is_none());
    assert!(rifleman_unit.gatherer.is_none());
    assert!(rifleman_unit.drop_off.is_none());
    assert!(rifleman_unit.producer.is_none());
    let rifleman = &catalog.production[&cid("base:train_rifleman")];
    assert_eq!(rifleman.duration_ms, 5000);
    assert_eq!(rifleman.costs[0].resource, cid("base:credits"));
    assert_eq!(rifleman.costs[0].amount, 100);
    assert_eq!(rifleman.costs[0].source, CostSource::SlotBalance);
    assert_eq!(rifleman.payment, PaymentPolicy::ChargeOnQueue);
    assert_eq!(rifleman.cancel_refunds.queued.numerator, 1);
    assert_eq!(rifleman.cancel_refunds.queued.denominator, 1);
    assert_eq!(rifleman.cancel_refunds.running.numerator, 1);
    assert_eq!(rifleman.cancel_refunds.running.denominator, 2);
    assert_eq!(rifleman.cancel_refunds.output_blocked.numerator, 1);
    assert_eq!(rifleman.cancel_refunds.output_blocked.denominator, 1);
    let built_harvester = &catalog.production[&cid("base:build_harvester")];
    assert_eq!(built_harvester.duration_ms, 15000);
    assert_eq!(built_harvester.costs[0].amount, 600);
    assert_eq!(built_harvester.costs[0].source, CostSource::SlotBalance);
    assert_eq!(built_harvester.payment, PaymentPolicy::ChargeOnQueue);
    assert_eq!(built_harvester.cancel_refunds.running.denominator, 2);
    assert_eq!(
        archive.maps().unwrap()[0].1.size,
        turts_mod_api::MapSize { x: 64.0, y: 64.0 }
    );
    assert_eq!(state.units.len(), 4);
    assert_eq!(owner_unit_counts(&state), [(0, 2), (1, 2)]);
    assert_placed_units(
        &state,
        &[
            ("base:command_center", 0, 8.0, 8.0),
            ("base:harvester", 0, 12.0, 8.0),
            ("base:command_center", 1, 56.0, 8.0),
            ("base:harvester", 1, 52.0, 8.0),
        ],
    );
    assert_eq!(state.nodes.len(), 2);
    assert_eq!(
        state
            .nodes
            .iter()
            .map(|node| (node.pos.x, node.pos.y, node.remaining))
            .collect::<Vec<_>>(),
        [(24.0, 8.0, 1000), (40.0, 8.0, 1000)]
    );
    assert_eq!(state.economy.balances.len(), 2);
    assert_eq!(
        state
            .economy
            .balances
            .iter()
            .map(|balance| (
                balance.slot,
                balance.resource.clone(),
                balance.on_hand,
                balance.reserved
            ))
            .collect::<Vec<_>>(),
        [
            (0, cid("base:credits"), 500, 0),
            (1, cid("base:credits"), 500, 0)
        ]
    );
    assert_eq!(state.economy.inventories.len(), 2);
    assert_inventory(
        &state,
        placed_unit(&state, "base:harvester", 0, 12.0, 8.0),
        100,
        &[],
    );
    assert_inventory(
        &state,
        placed_unit(&state, "base:harvester", 1, 52.0, 8.0),
        100,
        &[],
    );
}

#[test]
fn haul_boots_local_item_economy_for_both_slots() {
    let (archive, state) = boot("haul");
    assert_eq!(archive.manifest.version, "0.2.0");
    let resources = archive.resources().unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].1.kind, ResourceKind::Item);
    assert_eq!(resources[0].1.id, "ore");
    assert_eq!(resources[0].1.name, "Ore");
    assert_eq!(resources[0].1.unit_name, "ore");
    let catalog = archive.content().unwrap();
    assert_eq!(catalog.units.len(), 2);
    assert_eq!(catalog.nodes.len(), 1);
    assert_eq!(catalog.production.len(), 1);
    let depot = &catalog.units[&cid("haul:depot")];
    assert!(depot.movement.is_none());
    assert!(depot.gatherer.is_none());
    assert!(matches!(
        depot.drop_off.as_ref().unwrap().rules[0].effect,
        ResolvedDropOffEffect::Store
    ));
    assert_eq!(depot.inventory.as_ref().unwrap().capacity, 2000);
    assert_eq!(depot.producer.as_ref().unwrap().queue_capacity, 5);
    assert_eq!(
        depot.producer.as_ref().unwrap().recipes,
        [cid("haul:train_worker")]
    );
    assert_eq!(
        depot.producer.as_ref().unwrap().spawn_offsets,
        [WorldPos { x: 0.0, y: 4.0 }, WorldPos { x: 0.0, y: -4.0 }]
    );
    let worker = &catalog.units[&cid("haul:worker")];
    assert_eq!(worker.movement.as_ref().unwrap().speed, 3.0);
    assert_eq!(worker.inventory.as_ref().unwrap().capacity, 10);
    assert_eq!(worker.gatherer.as_ref().unwrap().gather_per_second, 5);
    assert_eq!(worker.gatherer.as_ref().unwrap().unload_per_second, 10);
    assert_eq!(worker.gatherer.as_ref().unwrap().interaction_range, 0.5);
    assert_eq!(worker.gatherer.as_ref().unwrap().items, [cid("haul:ore")]);
    assert!(worker.drop_off.is_none());
    assert!(worker.producer.is_none());
    let recipe = &catalog.production[&cid("haul:train_worker")];
    assert_eq!(recipe.duration_ms, 6000);
    assert_eq!(recipe.costs[0].resource, cid("haul:ore"));
    assert_eq!(recipe.costs[0].amount, 50);
    assert_eq!(recipe.costs[0].source, CostSource::ProducerInventory);
    assert_eq!(recipe.payment, PaymentPolicy::ReserveThenCharge);
    assert_eq!(recipe.cancel_refunds.queued.numerator, 1);
    assert_eq!(recipe.cancel_refunds.queued.denominator, 1);
    assert_eq!(recipe.cancel_refunds.running.numerator, 1);
    assert_eq!(recipe.cancel_refunds.running.denominator, 1);
    assert_eq!(recipe.cancel_refunds.output_blocked.numerator, 1);
    assert_eq!(recipe.cancel_refunds.output_blocked.denominator, 1);
    assert_eq!(
        archive.maps().unwrap()[0].1.size,
        turts_mod_api::MapSize { x: 48.0, y: 48.0 }
    );
    assert_eq!(state.units.len(), 6);
    assert_eq!(owner_unit_counts(&state), [(0, 3), (1, 3)]);
    assert_placed_units(
        &state,
        &[
            ("haul:depot", 0, 8.0, 8.0),
            ("haul:worker", 0, 12.0, 8.0),
            ("haul:worker", 0, 12.0, 10.0),
            ("haul:depot", 1, 40.0, 8.0),
            ("haul:worker", 1, 36.0, 8.0),
            ("haul:worker", 1, 36.0, 10.0),
        ],
    );
    assert_eq!(state.nodes.len(), 2);
    assert_eq!(
        state
            .nodes
            .iter()
            .map(|node| (node.pos.x, node.pos.y, node.remaining))
            .collect::<Vec<_>>(),
        [(20.0, 8.0, 1000), (28.0, 8.0, 1000)]
    );
    assert!(state.economy.balances.is_empty());
    assert_eq!(state.economy.inventories.len(), 6);
    assert_inventory(
        &state,
        placed_unit(&state, "haul:depot", 0, 8.0, 8.0),
        2000,
        &[("haul:ore", 100)],
    );
    assert_inventory(
        &state,
        placed_unit(&state, "haul:depot", 1, 40.0, 8.0),
        2000,
        &[("haul:ore", 100)],
    );
    for (slot, x, y) in [
        (0, 12.0, 8.0),
        (0, 12.0, 10.0),
        (1, 36.0, 8.0),
        (1, 36.0, 10.0),
    ] {
        assert_inventory(
            &state,
            placed_unit(&state, "haul:worker", slot, x, y),
            10,
            &[],
        );
    }
}

#[test]
fn acme_keeps_its_m1_scout_without_economy_content() {
    let (archive, state) = boot("acme");
    assert_eq!(archive.manifest.version, "0.2.0");
    assert!(archive.resources().unwrap().is_empty());
    assert!(archive.nodes().unwrap().is_empty());
    assert!(archive.production().unwrap().is_empty());
    let catalog = archive.content().unwrap();
    assert_eq!(catalog.units.len(), 1);
    assert_eq!(
        catalog.units[&cid("acme:scout")]
            .movement
            .as_ref()
            .unwrap()
            .speed,
        6.0
    );
    assert_eq!(state.units.len(), 1);
    assert!(state.nodes.is_empty());
    assert!(state.economy.balances.is_empty());
    assert!(state.economy.inventories.is_empty());
}

#[test]
fn cedar_matches_the_public_ten_file_authoring_example() {
    let (archive, state) = boot("cedar");
    assert_eq!(archive.manifest.version, "0.2.0");
    let expected = [
        "manifest.ron",
        "resources/wood.ron",
        "units/depot.ron",
        "units/worker.ron",
        "nodes/grove.ron",
        "production/hire.ron",
        "maps/clearing.ron",
        "visuals/depot.ron",
        "visuals/worker.ron",
        "visuals/grove.ron",
    ];
    let mut authored = Vec::new();
    files_below(&package_dir("cedar"), &package_dir("cedar"), &mut authored);
    authored.sort();
    let mut expected_sorted = expected
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    expected_sorted.sort();
    assert_eq!(authored, expected_sorted);
    let exports = &archive.manifest.exports;
    assert_eq!(exports.units, ["units/depot.ron", "units/worker.ron"]);
    assert_eq!(exports.maps, ["maps/clearing.ron"]);
    assert_eq!(
        exports.visuals,
        [
            "visuals/depot.ron",
            "visuals/worker.ron",
            "visuals/grove.ron"
        ]
    );
    assert_eq!(exports.resources, ["resources/wood.ron"]);
    assert_eq!(exports.nodes, ["nodes/grove.ron"]);
    assert_eq!(exports.production, ["production/hire.ron"]);
    for path in expected {
        assert!(archive.file(path).is_some(), "missing {path}");
    }
    assert_eq!(archive.manifest.id, "cedar");
    assert_eq!(archive.manifest.kind, PackageKind::Game);
    assert_eq!(archive.manifest.engine_api, "^0.2");
    assert_eq!(archive.manifest.dependencies, UniqueMap(Default::default()));
    assert_eq!(archive.manifest.context, ExecutionContext::Shared);
    assert!(archive.manifest.capabilities.is_empty());
    assert_eq!(archive.manifest.license, "LicenseRef-TBD");
    let expected_exports = Exports {
        units: vec!["units/depot.ron".into(), "units/worker.ron".into()],
        maps: vec!["maps/clearing.ron".into()],
        visuals: vec![
            "visuals/depot.ron".into(),
            "visuals/worker.ron".into(),
            "visuals/grove.ron".into(),
        ],
        resources: vec!["resources/wood.ron".into()],
        nodes: vec!["nodes/grove.ron".into()],
        production: vec!["production/hire.ron".into()],

        recipes: Vec::new(),
        technologies: Vec::new(),
    };
    assert_eq!(archive.manifest.exports, expected_exports);
    assert_eq!(
        archive
            .resources()
            .unwrap()
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
        [ResourceDef {
            id: "wood".into(),
            name: "Timber".into(),
            unit_name: "log".into(),
            kind: ResourceKind::Item,
        }]
    );
    assert_eq!(
        archive
            .units()
            .unwrap()
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
        [
            UnitDef {
                id: "depot".into(),
                name: "Timber Yard".into(),
                footprint: Footprint::Circle { radius: 2.0 },
                movement: None,
                inventory: Some(InventoryDef {
                    capacity: 2000,
                    accepts: vec!["cedar:wood".into()],
                }),
                gatherer: None,
                drop_off: Some(DropOffDef {
                    rules: vec![DropOffRule {
                        item: "cedar:wood".into(),
                        effect: DropOffEffect::Store,
                    }],
                }),
                producer: Some(ProducerDef {
                    recipes: vec!["cedar:hire".into()],
                    queue_capacity: 5,
                    spawn_offsets: vec![WorldPos { x: 0.0, y: 4.0 }, WorldPos { x: 0.0, y: -4.0 },],
                }),
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
            UnitDef {
                id: "worker".into(),
                name: "Carrier".into(),
                footprint: Footprint::Circle { radius: 0.5 },
                movement: Some(MovementDef { speed: 3.0 }),
                inventory: Some(InventoryDef {
                    capacity: 10,
                    accepts: vec!["cedar:wood".into()],
                }),
                gatherer: Some(GathererDef {
                    items: vec!["cedar:wood".into()],
                    gather_per_second: 5,
                    unload_per_second: 10,
                    interaction_range: 0.5,
                }),
                drop_off: None,
                producer: None,
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        ]
    );
    assert_eq!(
        archive
            .nodes()
            .unwrap()
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
        [NodeDef {
            id: "grove".into(),
            name: "Timber Stand".into(),
            item: "cedar:wood".into(),
            radius: 1.0,
        }]
    );
    assert_eq!(
        archive
            .production()
            .unwrap()
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
        [ProductionDef {
            id: "hire".into(),
            name: "Hire Carrier".into(),
            output_unit: "cedar:worker".into(),
            duration_ms: 6000,
            costs: vec![Cost {
                resource: "cedar:wood".into(),
                amount: 50,
                source: CostSource::ProducerInventory,
            }],
            payment: PaymentPolicy::ReserveThenCharge,
            cancel_refunds: CancelRefunds {
                queued: RefundRatio {
                    numerator: 1,
                    denominator: 1
                },
                running: RefundRatio {
                    numerator: 1,
                    denominator: 1
                },
                output_blocked: RefundRatio {
                    numerator: 1,
                    denominator: 1
                },
            },
        }]
    );
    assert_eq!(
        archive
            .visuals()
            .unwrap()
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
        [
            VisualDef {
                target: VisualTarget::Unit("cedar:depot".into()),
                shape: Shape::Cuboid {
                    x: 3.0,
                    y: 3.0,
                    z: 2.0
                },
            },
            VisualDef {
                target: VisualTarget::Unit("cedar:worker".into()),
                shape: Shape::Capsule {
                    radius: 0.3,
                    height: 1.6
                },
            },
            VisualDef {
                target: VisualTarget::Node("cedar:grove".into()),
                shape: Shape::Cuboid {
                    x: 1.4,
                    y: 1.4,
                    z: 0.7
                },
            },
        ]
    );
    assert_eq!(
        archive.maps().unwrap()[0].1,
        MapDef {
            id: "clearing".into(),
            size: MapSize { x: 64.0, y: 64.0 },
            placements: vec![
                Placement {
                    unit: "cedar:depot".into(),
                    owner_slot: 0,
                    pos: WorldPos { x: 8.0, y: 8.0 },
                    contents: vec![ItemStack {
                        item: "cedar:wood".into(),
                        amount: 100
                    }],

                    facing: None,
                },
                Placement {
                    unit: "cedar:worker".into(),
                    owner_slot: 0,
                    pos: WorldPos { x: 12.0, y: 8.0 },
                    contents: vec![],

                    facing: None,
                },
                Placement {
                    unit: "cedar:worker".into(),
                    owner_slot: 0,
                    pos: WorldPos { x: 12.0, y: 12.0 },
                    contents: vec![],

                    facing: None,
                },
                Placement {
                    unit: "cedar:depot".into(),
                    owner_slot: 1,
                    pos: WorldPos { x: 56.0, y: 8.0 },
                    contents: vec![ItemStack {
                        item: "cedar:wood".into(),
                        amount: 100
                    }],

                    facing: None,
                },
                Placement {
                    unit: "cedar:worker".into(),
                    owner_slot: 1,
                    pos: WorldPos { x: 52.0, y: 8.0 },
                    contents: vec![],

                    facing: None,
                },
                Placement {
                    unit: "cedar:worker".into(),
                    owner_slot: 1,
                    pos: WorldPos { x: 52.0, y: 12.0 },
                    contents: vec![],

                    facing: None,
                },
            ],
            nodes: vec![
                NodePlacement {
                    node: "cedar:grove".into(),
                    pos: WorldPos { x: 24.0, y: 8.0 },
                    remaining: 1000
                },
                NodePlacement {
                    node: "cedar:grove".into(),
                    pos: WorldPos { x: 40.0, y: 8.0 },
                    remaining: 1000
                },
            ],
            slot_economy: vec![],

            belts: Vec::new(),
        }
    );
    let production = archive.production().unwrap();
    assert_eq!(production.len(), 1);
    assert_eq!(production[0].1.duration_ms, 6000);
    assert_eq!(
        production[0].1.costs[0].source,
        CostSource::ProducerInventory
    );
    assert_eq!(production[0].1.payment, PaymentPolicy::ReserveThenCharge);
    assert_eq!(production[0].1.costs[0].amount, 50);
    let resources = archive.resources().unwrap();
    assert_eq!(resources[0].1.name, "Timber");
    assert_eq!(resources[0].1.unit_name, "log");
    assert_eq!(
        archive.maps().unwrap()[0].1.size,
        turts_mod_api::MapSize { x: 64.0, y: 64.0 }
    );
    assert_eq!(state.units.len(), 6);
    assert_eq!(state.nodes.len(), 2);
    assert_placed_units(
        &state,
        &[
            ("cedar:depot", 0, 8.0, 8.0),
            ("cedar:worker", 0, 12.0, 8.0),
            ("cedar:worker", 0, 12.0, 12.0),
            ("cedar:depot", 1, 56.0, 8.0),
            ("cedar:worker", 1, 52.0, 8.0),
            ("cedar:worker", 1, 52.0, 12.0),
        ],
    );
    assert_eq!(state.economy.inventories.len(), 6);
    assert_inventory(
        &state,
        placed_unit(&state, "cedar:depot", 0, 8.0, 8.0),
        2000,
        &[("cedar:wood", 100)],
    );
    assert_inventory(
        &state,
        placed_unit(&state, "cedar:depot", 1, 56.0, 8.0),
        2000,
        &[("cedar:wood", 100)],
    );
    for (slot, x, y) in [
        (0, 12.0, 8.0),
        (0, 12.0, 12.0),
        (1, 52.0, 8.0),
        (1, 52.0, 12.0),
    ] {
        assert_inventory(
            &state,
            placed_unit(&state, "cedar:worker", slot, x, y),
            10,
            &[],
        );
    }
    assert_eq!(
        state
            .nodes
            .iter()
            .map(|node| (node.pos.x, node.pos.y, node.remaining))
            .collect::<Vec<_>>(),
        [(24.0, 8.0, 1000), (40.0, 8.0, 1000)]
    );
    assert!(state.economy.balances.is_empty());
    assert_eq!(
        state
            .economy
            .inventories
            .iter()
            .filter(|inventory| inventory.items.iter().any(|item| item.on_hand == 100))
            .count(),
        2
    );
}
