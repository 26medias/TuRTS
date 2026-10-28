//! Protocol 3 export policy against the booted Forge package.

use std::path::Path;

#[path = "../src/export.rs"]
mod export;

use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::pack::pack_dir;
use turts_net_protocol::messages as wire;
use turts_sim::economy::events::{CommittedTick, committed_tick, completed_world};
use turts_sim::economy::views_m3::{
    RawBeltAudit, RawBeltPieceAudit, RawBeltSlotAudit, RawExtractorAudit, RawExtractorStatusAudit,
    RawHealthAudit, RawInserterAudit, RawInserterStageAudit, RawResearchAudit, RawWeaponAudit,
    RawWeaponStatusAudit,
};
use turts_sim::{
    Command, CommandKind, CommandQueue, SlotId, UnitState, Vec2, headless_app,
    initialize_completed_world,
};

fn forge_app() -> bevy_app::App {
    let package_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
    let bytes = pack_dir(&package_dir).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    initialize_completed_world(app.world_mut());
    app
}

fn mixed_owner_view() -> turts_sim::WorldView {
    let app = forge_app();
    let mut view = completed_world(app.world()).clone();
    view.units.push(UnitState {
        unit_id: 90,
        content_id: "forge:arm_loader".into(),
        owner_slot: 1,
        facing: None,
        pos: Vec2::new(25.0, 8.0),
    });
    view.m3.raw_belts.push(RawBeltAudit {
        first_piece: 80,
        definition: turts_mod_api::ContentId::parse("forge:belt").unwrap(),
        owner_slot: 1,
        slots_per_piece: 1,
        slots_per_second: 2,
        rate_carry: 0,
        raw_slots: vec![],
        pieces: vec![RawBeltPieceAudit {
            unit: 80,
            slots: vec![RawBeltSlotAudit {
                item_index: 1,
                item: None,
            }],
        }],
    });
    view.m3.raw_inserters.extend([
        RawInserterAudit {
            unit: 2,
            enabled: true,
            stage: RawInserterStageAudit::ToDestination,
            elapsed: 3,
            source: Some(1),
            destination: Some(3),
        },
        RawInserterAudit {
            unit: 90,
            enabled: true,
            stage: RawInserterStageAudit::Returning,
            elapsed: 2,
            source: Some(90),
            destination: None,
        },
    ]);
    view
}

fn test_batch(tick: u32) -> CommittedTick {
    CommittedTick {
        tick,
        ..Default::default()
    }
}

#[test]
fn snapshot_for_slot1_has_no_owner_only_records() {
    let view = mixed_owner_view();
    let slot1 = export::snapshot_for(&view, SlotId(1));
    assert_eq!(
        slot1
            .inserters
            .iter()
            .map(|row| row.unit)
            .collect::<Vec<_>>(),
        vec![wire::UnitId(90)]
    );
    assert_eq!(
        slot1.belts.iter().map(|row| row.piece).collect::<Vec<_>>(),
        vec![wire::UnitId(80)]
    );
    let slot0 = export::snapshot_for(&view, SlotId(0));
    assert!(
        !slot0
            .inserters
            .iter()
            .any(|row| row.unit == wire::UnitId(90))
    );
    assert!(slot0.belts.iter().all(|row| row.piece != wire::UnitId(80)));
    assert!(slot1.belts.iter().all(|record| {
        view.m3
            .raw_belts
            .iter()
            .find(|belt| belt.pieces.iter().any(|piece| piece.unit == record.piece.0))
            .is_some_and(|belt| belt.owner_slot == 1)
    }));
    assert!(slot1.extractors.iter().all(|row| {
        view.units
            .iter()
            .any(|unit| unit.unit_id == row.unit.0 && unit.owner_slot == 1)
    }));
    assert!(slot1.weapons.iter().all(|row| {
        view.units
            .iter()
            .any(|unit| unit.unit_id == row.unit.0 && unit.owner_slot == 1)
    }));
    assert!(slot1.research.is_empty());
}

#[test]
fn delta_for_slot1_has_healths_and_removed_only() {
    let mut view = mixed_owner_view();
    view.m3.research = vec![
        RawResearchAudit {
            slot: 0,
            technology: turts_mod_api::ContentId::parse("forge:hardened_shells").unwrap(),
        },
        RawResearchAudit {
            slot: 1,
            technology: turts_mod_api::ContentId::parse("forge:hardened_shells").unwrap(),
        },
    ];
    let mut batch = test_batch(9);
    batch.removed = vec![2];
    batch.healths = vec![RawHealthAudit {
        unit: 2,
        current: 0,
    }];
    batch.inserters = vec![RawInserterAudit {
        unit: 2,
        enabled: false,
        stage: RawInserterStageAudit::AtSource(None),
        elapsed: 0,
        source: None,
        destination: None,
    }];
    batch.extractors = vec![RawExtractorAudit {
        unit: 1,
        enabled: false,
        node: Some(1),
        rate_carry: 0,
        status: RawExtractorStatusAudit::Depleted,
    }];
    batch.weapons = vec![RawWeaponAudit {
        unit: 7,
        enabled: false,
        cooldown: 0,
        effective_damage: Ok(10),
        status: RawWeaponStatusAudit::Ready,
        last_target: None,
    }];
    batch.belts = vec![RawBeltAudit {
        first_piece: 20,
        definition: turts_mod_api::ContentId::parse("forge:belt").unwrap(),
        owner_slot: 0,
        slots_per_piece: 2,
        slots_per_second: 4,
        rate_carry: 0,
        raw_slots: vec![],
        pieces: vec![RawBeltPieceAudit {
            unit: 20,
            slots: vec![RawBeltSlotAudit {
                item_index: 4,
                item: Some(turts_mod_api::ContentId::parse("forge:shell").unwrap()),
            }],
        }],
    }];
    batch.research = vec![RawResearchAudit {
        slot: 0,
        technology: turts_mod_api::ContentId::parse("forge:hardened_shells").unwrap(),
    }];
    let delta = export::delta_for(&batch, &view, SlotId(1));
    assert_eq!(
        delta.healths,
        vec![wire::HealthRecord {
            unit: wire::UnitId(2),
            current: 0
        }]
    );
    assert_eq!(delta.removed, vec![wire::UnitId(2)]);
    assert!(delta.belts.is_empty());
    assert!(delta.inserters.is_empty());
    assert!(delta.extractors.is_empty());
    assert!(delta.weapons.is_empty());
    assert!(delta.research.is_empty());
}

#[test]
fn late_join_snapshot_has_belts_hands_healths() {
    let view = mixed_owner_view();
    let snapshot = export::snapshot_for(&view, SlotId(0));
    assert_eq!(
        snapshot
            .units
            .iter()
            .find(|unit| unit.unit_id == wire::UnitId(2))
            .unwrap()
            .facing,
        Some(wire::Facing::PosX)
    );
    assert!(
        !snapshot.belts.is_empty(),
        "Forge's six belt pieces appear in the late-join snapshot"
    );
    assert!(
        !snapshot.inserters.is_empty(),
        "Forge inserter hands are present"
    );
    assert!(!snapshot.healths.is_empty(), "live Forge health is public");
}

#[test]
fn killed_unit_delta_has_final_health_zero_and_removed() {
    let view = mixed_owner_view();
    let mut batch = test_batch(10);
    batch.removed = vec![13];
    batch.healths = vec![RawHealthAudit {
        unit: 13,
        current: 0,
    }];
    let delta = export::delta_for(&batch, &view, SlotId(0));
    assert_eq!(
        delta.healths,
        vec![wire::HealthRecord {
            unit: wire::UnitId(13),
            current: 0
        }]
    );
    assert_eq!(delta.removed, vec![wire::UnitId(13)]);
}

#[test]
fn live_toggle_publishes_queue_state_when_job_list_is_unchanged() {
    let mut app = forge_app();
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::SetEnabled {
                unit_id: 4,
                enabled: false,
            },
        });
    app.update();
    let batch = committed_tick(app.world());
    assert!(
        batch.economy.queues.is_empty(),
        "the toggle itself does not change queued jobs"
    );
    assert!(
        batch
            .enabled
            .iter()
            .any(|row| row.unit == 4 && !row.enabled)
    );
    let view = completed_world(app.world());
    let delta = export::delta_for(batch, view, SlotId(0));
    assert_eq!(delta.economy.queues.len(), 1);
    assert_eq!(delta.economy.queues[0].producer, wire::UnitId(4));
    assert!(!delta.economy.queues[0].enabled);
    assert!(delta.economy.queues[0].jobs.is_empty());
}

#[test]
fn main_and_output_inventories_keep_distinct_ports() {
    let view = mixed_owner_view();
    let snapshot = export::snapshot_for(&view, SlotId(0));
    assert!(
        snapshot
            .economy
            .inventories
            .iter()
            .any(|row| { row.unit == wire::UnitId(4) && row.port == wire::InventoryPort::Main })
    );
    assert!(
        snapshot
            .economy
            .inventories
            .iter()
            .any(|row| { row.unit == wire::UnitId(4) && row.port == wire::InventoryPort::Output })
    );
}

#[test]
fn weapon_effective_damage_preserves_checked_error() {
    let mut view = mixed_owner_view();
    view.m3.raw_weapons = vec![RawWeaponAudit {
        unit: 7,
        enabled: true,
        cooldown: 0,
        effective_damage: Err(turts_sim::RejectReason::ArithmeticOverflow),
        status: RawWeaponStatusAudit::Ready,
        last_target: None,
    }];
    let snapshot = export::snapshot_for(&view, SlotId(0));
    assert_eq!(
        snapshot.weapons[0].damage,
        Err(wire::CommandRejectReason::ArithmeticOverflow)
    );
}

#[test]
fn empty_state_produces_empty_m3_snapshot_and_delta() {
    let mut view = mixed_owner_view();
    view.m3.raw_belts.clear();
    view.m3.raw_inserters.clear();
    view.m3.raw_extractors.clear();
    view.m3.raw_weapons.clear();
    view.m3.raw_healths.clear();
    view.m3.research.clear();
    let snapshot = export::snapshot_for(&view, SlotId(0));
    assert!(snapshot.belts.is_empty());
    assert!(snapshot.inserters.is_empty());
    assert!(snapshot.extractors.is_empty());
    assert!(snapshot.weapons.is_empty());
    assert!(snapshot.healths.is_empty());
    assert!(snapshot.research.is_empty());
    let batch = test_batch(3);
    let delta = export::delta_for(&batch, &view, SlotId(0));
    assert!(delta.belts.is_empty());
    assert!(delta.inserters.is_empty());
    assert!(delta.extractors.is_empty());
    assert!(delta.weapons.is_empty());
    assert!(delta.healths.is_empty());
    assert!(delta.research.is_empty());
}

#[test]
fn output_inventory_delta_retains_owner_and_output_port() {
    let view = mixed_owner_view();
    let mut batch = test_batch(view.tick);
    batch.output_inventories = view.m3.raw_output_inventories.clone();
    let owner = export::delta_for(&batch, &view, SlotId(0));
    assert!(
        owner
            .economy
            .inventories
            .iter()
            .any(|row| { row.unit == wire::UnitId(4) && row.port == wire::InventoryPort::Output })
    );
    let opponent = export::delta_for(&batch, &view, SlotId(1));
    assert!(opponent.economy.inventories.is_empty());
}
