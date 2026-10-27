use std::path::PathBuf;

use turts_mod_api::{ContentId, CostSource, PaymentPolicy, ResolvedDropOffEffect, ResourceKind};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{AuditState, audit_state, headless_app};

fn package_dir(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(id)
}

fn boot(id: &str) -> (PackageArchive, AuditState) {
    let bytes = pack_dir(&package_dir(id)).unwrap_or_else(|error| panic!("pack {id}: {error}"));
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).unwrap_or_else(|error| panic!("boot {id}: {error}"));
    let state = audit_state(app.world_mut());
    (archive, state)
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
    let receiver = &command_center.drop_off.as_ref().unwrap().rules[0];
    assert_eq!(receiver.item, cid("base:ore"));
    assert!(matches!(
        receiver.effect,
        ResolvedDropOffEffect::Convert { ref balance, per_item: 1 }
            if balance == &cid("base:credits")
    ));
    assert_eq!(command_center.producer.as_ref().unwrap().queue_capacity, 5);
    let harvester = &catalog.units[&cid("base:harvester")];
    assert_eq!(harvester.movement.as_ref().unwrap().speed, 2.0);
    assert_eq!(harvester.inventory.as_ref().unwrap().capacity, 100);
    assert_eq!(harvester.gatherer.as_ref().unwrap().gather_per_second, 20);
    assert_eq!(harvester.gatherer.as_ref().unwrap().unload_per_second, 50);
    assert_eq!(harvester.gatherer.as_ref().unwrap().interaction_range, 0.5);
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
            .map(|balance| (balance.slot, balance.on_hand))
            .collect::<Vec<_>>(),
        [(0, 500), (1, 500)]
    );
    assert_eq!(state.economy.inventories.len(), 2);
    assert!(
        state
            .economy
            .inventories
            .iter()
            .all(|inventory| inventory.items.iter().all(|item| item.on_hand == 0))
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
    assert!(matches!(
        depot.drop_off.as_ref().unwrap().rules[0].effect,
        ResolvedDropOffEffect::Store
    ));
    assert_eq!(depot.inventory.as_ref().unwrap().capacity, 2000);
    assert_eq!(depot.producer.as_ref().unwrap().queue_capacity, 5);
    let worker = &catalog.units[&cid("haul:worker")];
    assert_eq!(worker.movement.as_ref().unwrap().speed, 3.0);
    assert_eq!(worker.inventory.as_ref().unwrap().capacity, 10);
    assert_eq!(worker.gatherer.as_ref().unwrap().gather_per_second, 5);
    assert_eq!(worker.gatherer.as_ref().unwrap().unload_per_second, 10);
    assert_eq!(worker.gatherer.as_ref().unwrap().interaction_range, 0.5);
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
    assert_eq!(
        state
            .economy
            .inventories
            .iter()
            .filter(|inventory| inventory.items.iter().any(|item| item.on_hand == 100))
            .count(),
        2
    );
    assert!(
        state
            .economy
            .inventories
            .iter()
            .filter(|inventory| inventory.items.iter().any(|item| item.on_hand == 100))
            .all(|inventory| inventory.owner_slot <= 1)
    );
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
