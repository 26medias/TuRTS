use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

use bevy::platform::time::Instant;
use turts_client::{
    content::ClientContent,
    replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome},
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    BalanceState, BeltPieceRecord, EconomyChanges, EconomyState, ExtractorRecord, ExtractorStatus,
    Facing, HealthRecord, InserterRecord, InserterStage, InventoryPort, InventoryState, NodeId,
    NodeState, Pos, ResearchRecord, SlotId, Snapshot, UnitDelta, UnitId, UnitSnapshot,
    WeaponRecord, WeaponStatus, WorldDelta,
};

fn content() -> &'static ClientContent {
    static CONTENT: OnceLock<ClientContent> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
        let archive =
            PackageArchive::parse(&pack_dir(&base).expect("pack forge")).expect("parse forge");
        ClientContent::from_archives(&[archive]).expect("resolve forge")
    })
}
fn unit(id: u32, content_id: &str, owner: u8) -> UnitSnapshot {
    UnitSnapshot {
        unit_id: UnitId(id),
        content_id: content_id.into(),
        owner_slot: SlotId(owner),
        facing: Some(Facing::PosX),
        pos: Pos {
            x: id as f32,
            y: 0.0,
        },
    }
}
fn snapshot(tick: u32, units: Vec<UnitSnapshot>) -> Snapshot {
    Snapshot {
        tick,
        units,
        nodes: vec![],
        economy: EconomyState {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    }
}
fn delta(tick: u32, base_tick: u32) -> WorldDelta {
    WorldDelta {
        tick,
        base_tick,
        spawned: vec![],
        removed: vec![],
        positions: vec![],
        nodes: vec![],
        economy: EconomyChanges {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    }
}
fn replica(slot: u8) -> Replica {
    let mut result = Replica::new(20);
    result.set_context(ReplicaContext::new(content().clone(), slot));
    result
}
fn inv(unit: u32, port: InventoryPort, resource: &str, amount: u64) -> InventoryState {
    InventoryState {
        unit: UnitId(unit),
        port,
        items: vec![BalanceState {
            resource: resource.into(),
            on_hand: amount,
            reserved: 0,
        }],
    }
}

#[test]
fn replica_merges_new_records_through_one_helper() {
    let now = Instant::now();
    let mut initial = snapshot(
        10,
        vec![
            unit(1, "forge:press", 0),
            unit(2, "forge:belt", 0),
            unit(3, "forge:arm_loader", 0),
            unit(4, "forge:drill", 0),
            unit(5, "forge:turret", 0),
            unit(6, "forge:drone", 1),
            unit(7, "forge:lab", 0),
        ],
    );
    initial.nodes = vec![NodeState {
        id: NodeId(4),
        content_id: "forge:ore_vein".into(),
        pos: Pos { x: 0.0, y: 0.0 },
        remaining: 10,
    }];
    initial.economy.inventories = vec![
        inv(1, InventoryPort::Main, "forge:ore", 2),
        inv(1, InventoryPort::Output, "forge:shell", 1),
    ];
    initial.belts = vec![BeltPieceRecord {
        piece: UnitId(2),
        slots: vec![Some("forge:ore".into()), None, None, None],
    }];
    initial.inserters = vec![InserterRecord {
        unit: UnitId(3),
        enabled: true,
        stage: InserterStage::Returning,
        elapsed: 3,
        source: None,
        destination: None,
    }];
    initial.extractors = vec![ExtractorRecord {
        unit: UnitId(4),
        enabled: true,
        node: Some(NodeId(4)),
        status: ExtractorStatus::Extracting,
    }];
    initial.weapons = vec![WeaponRecord {
        unit: UnitId(5),
        enabled: true,
        cooldown: 2,
        damage: Ok(10),
        status: WeaponStatus::Ready,
        last_target: Some(UnitId(6)),
    }];
    initial.healths = vec![HealthRecord {
        unit: UnitId(6),
        current: 100,
    }];
    initial.research = vec![ResearchRecord {
        slot: SlotId(0),
        completed: vec!["forge:hardened_shells".into()],
    }];
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert_eq!(
        replica.economy.inventories.len(),
        2,
        "Main and Output share a unit id"
    );
    let mut update = delta(11, 10);
    update.economy.inventories = vec![inv(1, InventoryPort::Output, "forge:shell", 2)];
    update.belts = vec![BeltPieceRecord {
        piece: UnitId(2),
        slots: vec![None, Some("forge:shell".into()), None, None],
    }];
    update.weapons = vec![WeaponRecord {
        unit: UnitId(5),
        enabled: false,
        cooldown: 2,
        damage: Err(turts_net_protocol::CommandRejectReason::MissingCapability),
        status: WeaponStatus::NoTarget,
        last_target: None,
    }];
    assert_eq!(replica.apply_delta(&update, now), DeltaOutcome::Applied);
    assert_eq!(replica.economy.inventories.len(), 2);
    assert_eq!(
        replica
            .economy
            .inventories
            .iter()
            .find(|i| i.port == InventoryPort::Main)
            .unwrap()
            .items[0]
            .on_hand,
        2
    );
    assert_eq!(
        replica
            .economy
            .inventories
            .iter()
            .find(|i| i.port == InventoryPort::Output)
            .unwrap()
            .items[0]
            .on_hand,
        2
    );
    assert_eq!(replica.belts[0].slots[1], Some("forge:shell".into()));
    assert_eq!(
        replica.weapons[0].damage,
        Err(turts_net_protocol::CommandRejectReason::MissingCapability)
    );
}

#[test]
fn removed_unit_clears_all_records() {
    let now = Instant::now();
    let mut initial = snapshot(
        10,
        vec![
            unit(1, "forge:belt", 0),
            unit(2, "forge:drone", 1),
            unit(3, "forge:press", 0),
            unit(4, "forge:arm_loader", 0),
            unit(5, "forge:drill", 0),
            unit(6, "forge:turret", 0),
            unit(7, "forge:lab", 0),
        ],
    );
    initial.belts.push(BeltPieceRecord {
        piece: UnitId(1),
        slots: vec![None; 4],
    });
    initial.inserters.push(InserterRecord {
        unit: UnitId(4),
        enabled: true,
        stage: InserterStage::Returning,
        elapsed: 1,
        source: None,
        destination: None,
    });
    initial.extractors.push(ExtractorRecord {
        unit: UnitId(5),
        enabled: true,
        node: None,
        status: ExtractorStatus::Depleted,
    });
    initial.weapons.push(WeaponRecord {
        unit: UnitId(6),
        enabled: true,
        cooldown: 0,
        damage: Ok(10),
        status: WeaponStatus::Ready,
        last_target: None,
    });
    initial.healths.push(HealthRecord {
        unit: UnitId(2),
        current: 100,
    });
    initial.economy.inventories = vec![
        inv(3, InventoryPort::Main, "forge:ore", 1),
        inv(3, InventoryPort::Output, "forge:shell", 1),
    ];
    initial.research.push(ResearchRecord {
        slot: SlotId(0),
        completed: vec!["forge:hardened_shells".into()],
    });
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    let mut removal = delta(11, 10);
    removal.removed = vec![
        UnitId(1),
        UnitId(2),
        UnitId(3),
        UnitId(4),
        UnitId(5),
        UnitId(6),
        UnitId(7),
    ];
    removal.healths = vec![HealthRecord {
        unit: UnitId(2),
        current: 0,
    }];
    assert_eq!(replica.apply_delta(&removal, now), DeltaOutcome::Applied);
    assert!(!replica.units.contains_key(&2));
    assert!(replica.healths.iter().all(|record| record.unit.0 != 2));
    assert_eq!(replica.health_trace[&2].last().unwrap().current, 0);
    assert!(replica.units.is_empty());
    assert!(replica.economy.inventories.is_empty());
    assert!(
        replica.belts.is_empty()
            && replica.inserters.is_empty()
            && replica.extractors.is_empty()
            && replica.weapons.is_empty()
            && replica.healths.is_empty()
    );
    assert_eq!(replica.research.len(), 1);
}

#[test]
fn health_trace_includes_killing_hit() {
    let now = Instant::now();
    let mut initial = snapshot(4, vec![unit(9, "forge:drone", 1)]);
    initial.healths = vec![HealthRecord {
        unit: UnitId(9),
        current: 100,
    }];
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    let mut hit = delta(5, 4);
    hit.healths = vec![HealthRecord {
        unit: UnitId(9),
        current: 80,
    }];
    assert_eq!(replica.apply_delta(&hit, now), DeltaOutcome::Applied);
    let mut killing = delta(6, 5);
    killing.positions = vec![UnitDelta {
        unit_id: UnitId(9),
        pos: Pos { x: 99.0, y: 5.0 },
    }];
    killing.healths = vec![HealthRecord {
        unit: UnitId(9),
        current: 0,
    }];
    killing.removed = vec![UnitId(9)];
    assert_eq!(replica.apply_delta(&killing, now), DeltaOutcome::Applied);
    assert_eq!(
        replica.health_trace[&9]
            .iter()
            .map(|entry| (entry.tick, entry.current))
            .collect::<Vec<_>>(),
        vec![(4, 100), (5, 80), (6, 0)]
    );
    assert!(!replica.units.contains_key(&9));
    assert!(replica.healths.iter().all(|record| record.unit.0 != 9));
}

#[test]
fn late_join_replica_matches_server_view() {
    let now = Instant::now();
    let mut view = snapshot(
        22,
        vec![
            unit(1, "forge:belt", 0),
            unit(2, "forge:arm_loader", 0),
            unit(3, "forge:drill", 0),
            unit(4, "forge:turret", 0),
            unit(5, "forge:drone", 1),
            unit(6, "forge:lab", 0),
        ],
    );
    view.belts = vec![BeltPieceRecord {
        piece: UnitId(1),
        slots: vec![None, Some("forge:ore".into()), None, None],
    }];
    view.inserters = vec![InserterRecord {
        unit: UnitId(2),
        enabled: true,
        stage: InserterStage::AtSource(None),
        elapsed: 0,
        source: Some(UnitId(1)),
        destination: None,
    }];
    view.extractors = vec![ExtractorRecord {
        unit: UnitId(3),
        enabled: false,
        node: None,
        status: ExtractorStatus::Full,
    }];
    view.weapons = vec![WeaponRecord {
        unit: UnitId(4),
        enabled: true,
        cooldown: 3,
        damage: Err(turts_net_protocol::CommandRejectReason::ArithmeticOverflow),
        status: WeaponStatus::Cooling,
        last_target: Some(UnitId(5)),
    }];
    view.healths = vec![HealthRecord {
        unit: UnitId(5),
        current: 0,
    }];
    view.research = vec![ResearchRecord {
        slot: SlotId(0),
        completed: vec!["forge:hardened_shells".into()],
    }];
    view.economy.inventories = vec![inv(3, InventoryPort::Main, "forge:ore", 2)];
    let mut joined = replica(0);
    assert_eq!(
        joined.apply_snapshot(&view, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert_eq!(joined.last_applied_tick, Some(view.tick));
    assert_eq!(joined.units[&1].facing, Some(Facing::PosX));
    assert_eq!(joined.belts, view.belts);
    assert_eq!(joined.inserters, view.inserters);
    assert_eq!(joined.extractors, view.extractors);
    assert_eq!(joined.weapons, view.weapons);
    assert_eq!(joined.healths, view.healths);
    assert_eq!(joined.research, view.research);
    assert_eq!(joined.economy, view.economy);
    assert_eq!(
        joined.health_trace,
        BTreeMap::from([(
            5,
            vec![turts_client::replica::HealthTraceEntry {
                tick: 22,
                current: 0
            }]
        )])
    );
}

#[test]
fn invalid_candidates_do_not_change_records_or_health_history() {
    let now = Instant::now();
    let mut initial = snapshot(1, vec![unit(1, "forge:drone", 1), unit(2, "forge:belt", 0)]);
    initial.healths = vec![HealthRecord {
        unit: UnitId(1),
        current: 100,
    }];
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    let before_trace = replica.health_trace.clone();
    let before_belts = replica.belts.clone();
    let mut invalid = delta(2, 1);
    invalid.healths = vec![HealthRecord {
        unit: UnitId(1),
        current: 50,
    }];
    invalid.belts = vec![
        BeltPieceRecord {
            piece: UnitId(2),
            slots: vec![Some("forge:ore".into()), None, None, None],
        },
        BeltPieceRecord {
            piece: UnitId(2),
            slots: vec![None; 4],
        },
    ];
    assert_eq!(
        replica.apply_delta(&invalid, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(replica.health_trace, before_trace);
    assert_eq!(replica.belts, before_belts);
    assert_eq!(replica.last_applied_tick, Some(1));
}

#[test]
fn stale_replayed_delta_does_not_duplicate_health_trace() {
    let now = Instant::now();
    let mut initial = snapshot(1, vec![unit(1, "forge:drone", 1)]);
    initial.healths = vec![HealthRecord {
        unit: UnitId(1),
        current: 100,
    }];
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    let mut update = delta(2, 1);
    update.healths = vec![HealthRecord {
        unit: UnitId(1),
        current: 90,
    }];
    assert_eq!(replica.apply_delta(&update, now), DeltaOutcome::Applied);
    assert_eq!(replica.apply_delta(&update, now), DeltaOutcome::Discarded);
    assert_eq!(replica.health_trace[&1].len(), 2);
}

#[test]
fn owner_capability_and_content_validation_fail_closed_while_health_is_public() {
    let now = Instant::now();
    let mut initial = snapshot(
        1,
        vec![
            unit(1, "forge:belt", 0),
            unit(2, "forge:belt", 1),
            unit(3, "forge:drone", 1),
        ],
    );
    initial.healths = vec![HealthRecord {
        unit: UnitId(3),
        current: 100,
    }];
    let mut base = replica(0);
    assert_eq!(
        base.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert_eq!(base.healths, initial.healths, "foreign health is public");

    for invalid_record in [
        BeltPieceRecord {
            piece: UnitId(2),
            slots: vec![None; 4],
        },
        BeltPieceRecord {
            piece: UnitId(3),
            slots: vec![None; 4],
        },
        BeltPieceRecord {
            piece: UnitId(1),
            slots: vec![Some("forge:missing".into()), None, None, None],
        },
    ] {
        let mut replica = replica(0);
        assert_eq!(
            replica.apply_snapshot(&initial, now),
            SnapshotOutcome::Applied { replayed: 0 }
        );
        let before = replica.clone();
        let mut invalid = delta(2, 1);
        invalid.belts = vec![invalid_record];
        assert_eq!(
            replica.apply_delta(&invalid, now),
            DeltaOutcome::ResyncRequested
        );
        assert_eq!(replica.last_applied_tick, before.last_applied_tick);
        assert_eq!(replica.belts, before.belts);
        assert_eq!(replica.health_trace, before.health_trace);
    }
}

#[test]
fn equal_health_and_resync_snapshots_do_not_duplicate_or_lose_trace() {
    let now = Instant::now();
    let mut initial = snapshot(10, vec![unit(9, "forge:drone", 1)]);
    initial.healths = vec![HealthRecord {
        unit: UnitId(9),
        current: 100,
    }];
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );

    let mut equal = delta(11, 10);
    equal.healths = vec![HealthRecord {
        unit: UnitId(9),
        current: 100,
    }];
    assert_eq!(replica.apply_delta(&equal, now), DeltaOutcome::Applied);
    assert_eq!(replica.health_trace[&9].len(), 1);

    let mut missing = delta(13, 12);
    missing.healths = vec![HealthRecord {
        unit: UnitId(9),
        current: 80,
    }];
    assert_eq!(
        replica.apply_delta(&missing, now),
        DeltaOutcome::ResyncRequested
    );
    let mut resync = snapshot(12, vec![unit(9, "forge:drone", 1)]);
    resync.healths = vec![HealthRecord {
        unit: UnitId(9),
        current: 90,
    }];
    assert_eq!(
        replica.apply_snapshot(&resync, now),
        SnapshotOutcome::Applied { replayed: 1 }
    );
    assert_eq!(
        replica.health_trace[&9]
            .iter()
            .map(|entry| (entry.tick, entry.current))
            .collect::<Vec<_>>(),
        vec![(10, 100), (12, 90), (13, 80)]
    );
    assert_eq!(replica.healths, missing.healths);
}

#[test]
fn debug_state_reports_actual_records_and_typed_health_trace() {
    use bevy::prelude::World;
    use turts_client::{
        ClientOpts,
        debug::{debug_json, report_json},
        ingest::{CommandResults, IngestCounters, ReplicaState},
        join::{JoinState, Phase},
        options::ClientOptions,
    };

    let now = Instant::now();
    let mut view = snapshot(7, vec![unit(4, "forge:drone", 1)]);
    view.healths = vec![HealthRecord {
        unit: UnitId(4),
        current: 70,
    }];
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&view, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    let mut world = World::new();
    world.insert_resource(ReplicaState {
        replica: Some(replica),
        ..Default::default()
    });
    world.insert_resource(IngestCounters::default());
    world.insert_resource(CommandResults::default());
    world.insert_resource(JoinState::default());
    world.insert_resource(Phase::Joined { slot: 0 });
    world.insert_resource(ClientOpts(ClientOptions::default()));
    let debug: serde_json::Value = serde_json::from_str(&debug_json(&world)).unwrap();
    assert_eq!(debug["replicated_world"]["healths"][0]["current"], 70);
    assert_eq!(debug["replicated_world"]["health_trace"][0]["unit_id"], 4);
    assert_eq!(
        debug["replicated_world"]["health_trace"][0]["entries"][0],
        serde_json::json!({"tick": 7, "current": 70})
    );
    let report = report_json(&world);
    assert_eq!(
        report["health_trace"][0]["entries"][0],
        serde_json::json!({"tick": 7, "current": 70})
    );
    assert_eq!(report["replicated_world"]["healths"][0]["current"], 70);
}

#[test]
fn snapshot_accepts_main_and_output_inventory_for_one_unit() {
    let now = Instant::now();
    let mut view = snapshot(10, vec![unit(1, "forge:press", 0)]);
    view.economy.inventories = vec![
        inv(1, InventoryPort::Main, "forge:ore", 1),
        inv(1, InventoryPort::Output, "forge:shell", 1),
    ];
    let mut replica = replica(0);
    assert_eq!(
        replica.apply_snapshot(&view, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert_eq!(replica.economy.inventories, view.economy.inventories);
}
