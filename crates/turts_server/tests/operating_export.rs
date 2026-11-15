//! Real boot, publication, and owner projection of operating-resource supply.
#[allow(dead_code)] // This fixture uses only part of the shared server projection.
#[path = "../src/export.rs"]
mod export;
#[allow(dead_code)] // This fixture uses only part of the shared server projection.
#[path = "../src/permissions.rs"]
mod permissions;

use std::path::Path;
use turts_mod_api::{
    ContentId, ResourceDef, ResourceKind, SourceDef, SupplyAmount, SupplyCounts, SupplyScope,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::economy::events::{committed_tick, completed_world};
use turts_sim::{
    Command, CommandKind, CommandQueue, SlotId, Slots, headless_app, initialize_completed_world,
    install_package, stage_package,
};

fn id(name: &str) -> ContentId {
    ContentId::parse(name).unwrap()
}

fn operating_app() -> bevy_app::App {
    let archive = PackageArchive::parse(
        &pack_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge")).unwrap(),
    )
    .unwrap();
    let mut catalog = archive.content().unwrap();
    for name in ["power", "population"] {
        catalog.resources.insert(
            id(&format!("forge:{name}")),
            ResourceDef {
                id: name.into(),
                name: name.into(),
                unit_name: name.into(),
                kind: ResourceKind::Supply {
                    scope: SupplyScope::Slot,
                    counts: SupplyCounts::Completed,
                },
            },
        );
    }
    let base = catalog.units[&id("forge:drone")].clone();
    for (name, provided, consumed, population) in [
        ("provider", 10, 0, 0),
        ("consumer", 0, 4, 0),
        ("foreign_provider", 77, 0, 88),
    ] {
        let mut unit = base.clone();
        unit.id = id(&format!("forge:{name}"));
        unit.name = name.into();
        unit.movement = None;
        unit.removal.as_mut().unwrap().by_command = true;
        if provided > 0 {
            unit.provides.push(SupplyAmount {
                resource: "forge:power".into(),
                amount: provided,
            });
        }
        if consumed > 0 {
            unit.consumes.push(SupplyAmount {
                resource: "forge:power".into(),
                amount: consumed,
            });
        }
        if population > 0 {
            unit.provides.push(SupplyAmount {
                resource: "forge:population".into(),
                amount: population,
            });
        }
        catalog.units.insert(unit.id.clone(), unit);
    }
    let map = ron::from_str(
        r#"(id:"operating",size:(x:32.0,y:16.0),placements:[
        (unit:"forge:provider",owner_slot:0,pos:(x:4.0,y:4.0),contents:[]),
        (unit:"forge:consumer",owner_slot:0,pos:(x:8.0,y:4.0),contents:[]),
        (unit:"forge:foreign_provider",owner_slot:1,pos:(x:16.0,y:4.0),contents:[])
    ],nodes:[],slot_economy:[])"#,
    )
    .unwrap();
    let mut app = headless_app(20);
    app.world_mut().insert_resource(Slots::new(2));
    let staged = stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: "maps/operating.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    install_package(app.world_mut(), staged);
    initialize_completed_world(app.world_mut());
    app
}

fn supply(snapshot: &turts_net_protocol::Snapshot, resource: &str) -> (u64, u64) {
    let row = snapshot
        .records
        .supply
        .iter()
        .find(|row| row.resource == resource)
        .unwrap();
    (row.provided, row.consumed)
}

#[test]
fn operating_initial_snapshots_publish_private_totals_and_declared_zeros() {
    let app = operating_app();
    let view = completed_world(app.world());
    let own = export::snapshot_for(view, SlotId(0));
    let peer = export::snapshot_for(view, SlotId(1));
    assert_eq!(own.records.supply.len(), 2);
    assert_eq!(peer.records.supply.len(), 2);
    assert!(own.records.supply.iter().all(|row| row.slot.0 == 0));
    assert!(peer.records.supply.iter().all(|row| row.slot.0 == 1));
    assert_eq!(supply(&own, "forge:power"), (10, 4));
    assert_eq!(supply(&own, "forge:population"), (0, 0));
    assert_eq!(supply(&peer, "forge:power"), (77, 0));
    assert_eq!(supply(&peer, "forge:population"), (88, 0));
}

#[test]
fn operating_zero_changes_match_resync_and_fresh_match_snapshots() {
    let mut app = operating_app();
    for unit_id in [1, 2] {
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(Command {
                slot: SlotId(0),
                seq: unit_id,
                kind: CommandKind::Remove { unit_id },
            });
    }
    app.update();
    // Supply is immutable during commands. Removal becomes visible at the next Supply phase.
    assert_eq!(
        supply(
            &export::snapshot_for(completed_world(app.world()), SlotId(0)),
            "forge:power"
        ),
        (10, 4)
    );
    app.update();
    let view = completed_world(app.world());
    let batch = committed_tick(app.world());
    let zero = export::delta_for(batch, view, SlotId(0));
    assert_eq!(
        zero.records.supply.len(),
        1,
        "only power changed; the declared population zero stays in the baseline"
    );
    assert_eq!(
        (
            zero.records.supply[0].provided,
            zero.records.supply[0].consumed
        ),
        (0, 0)
    );
    assert!(zero.records.supply.iter().all(|row| row.slot.0 == 0));
    assert!(
        export::delta_for(batch, view, SlotId(1))
            .records
            .supply
            .is_empty()
    );
    let resync = export::snapshot_for(view, SlotId(0));
    assert_eq!(resync.records.supply.len(), 2);
    assert_eq!(supply(&resync, "forge:power"), (0, 0));
    assert_eq!(supply(&resync, "forge:population"), (0, 0));
    let fresh = operating_app();
    assert_eq!(
        supply(
            &export::snapshot_for(completed_world(fresh.world()), SlotId(0)),
            "forge:power"
        ),
        (10, 4)
    );
}

#[test]
fn operating_projection_keeps_legacy_snapshot_supply_empty() {
    let mut app = headless_app(20);
    initialize_completed_world(app.world_mut());
    assert!(
        export::snapshot_for(completed_world(app.world()), SlotId(0))
            .records
            .supply
            .is_empty()
    );
    assert!(
        export::delta_for(
            committed_tick(app.world()),
            completed_world(app.world()),
            SlotId(0)
        )
        .records
        .supply
        .is_empty()
    );
}
