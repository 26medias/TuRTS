#[path = "support/repair_catalog.rs"]
mod fixture;

use bevy::platform::time::Instant;
use turts_client::{
    content::ClientContent,
    replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome},
};
use turts_net_protocol::{
    BalanceState, HealthRecord, Pos, RepairRecord, RepairState, SlotId, Snapshot, SnapshotRecords,
    UnitId, UnitSnapshot, WorldDelta,
};

fn content() -> ClientContent {
    ClientContent {
        catalog: fixture::catalog(),
        ..Default::default()
    }
}

fn action(target: u32, state: RepairState) -> RepairRecord {
    RepairRecord {
        repairer: UnitId(1),
        target: UnitId(target),
        state,
    }
}

fn snapshot(tick: u32) -> Snapshot {
    Snapshot {
        tick,
        units: [
            (1, "engineer", 0),
            (2, "barracks", 0),
            (3, "engineer", 1),
            (4, "barracks", 0),
            (5, "barracks", 1),
        ]
        .into_iter()
        .map(|(id, name, slot)| UnitSnapshot {
            unit_id: UnitId(id),
            content_id: format!("skirmish:{name}"),
            owner_slot: SlotId(slot),
            facing: None,
            pos: Pos {
                x: id as f32 * 4.0,
                y: 4.0,
            },
            site: false,
        })
        .collect(),
        healths: [(1, 60), (2, 200), (3, 60), (4, 200), (5, 200)]
            .into_iter()
            .map(|(id, current)| HealthRecord {
                unit: UnitId(id),
                current,
            })
            .collect(),
        records: SnapshotRecords {
            repairs: vec![action(2, RepairState::Approaching)],
            ..Default::default()
        },
        ..Default::default()
    }
}

fn replica() -> Replica {
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content(), 0));
    assert_eq!(
        replica.apply_snapshot(&snapshot(10), Instant::now()),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    replica
}

fn delta(tick: u32) -> WorldDelta {
    WorldDelta {
        tick,
        base_tick: tick - 1,
        ..Default::default()
    }
}

#[test]
fn repair_absolute_replacements_explicit_clears_and_endpoint_removal_stay_coherent() {
    let mut replica = replica();
    assert_eq!(replica.repair(1).unwrap().state, RepairState::Approaching);
    let mut changed = delta(11);
    changed.records.repairs = vec![action(
        2,
        RepairState::WaitingForFunds("skirmish:credits".into()),
    )];
    assert_eq!(
        replica.apply_delta(&changed, Instant::now()),
        DeltaOutcome::Applied
    );
    assert_eq!(replica.repair(1), Some(&changed.records.repairs[0]));
    let mut clear = delta(12);
    clear.records.repairs_cleared = vec![UnitId(1)];
    assert_eq!(
        replica.apply_delta(&clear, Instant::now()),
        DeltaOutcome::Applied
    );
    assert!(replica.repair(1).is_none());
    let mut retarget = delta(13);
    retarget.records.repairs = vec![action(4, RepairState::Working)];
    assert_eq!(
        replica.apply_delta(&retarget, Instant::now()),
        DeltaOutcome::Applied
    );
    assert_eq!(replica.repair(1).unwrap().target, UnitId(4));
    let mut target_loss = delta(14);
    target_loss.removed = vec![UnitId(4)];
    target_loss.records.repairs_cleared = vec![UnitId(1)];
    assert_eq!(
        replica.apply_delta(&target_loss, Instant::now()),
        DeltaOutcome::Applied
    );
    assert!(replica.repairs().next().is_none());
    let mut working = delta(15);
    working.records.repairs = vec![action(2, RepairState::Working)];
    assert_eq!(
        replica.apply_delta(&working, Instant::now()),
        DeltaOutcome::Applied
    );
    let mut removed = delta(16);
    removed.removed = vec![UnitId(1)];
    assert_eq!(
        replica.apply_delta(&removed, Instant::now()),
        DeltaOutcome::Applied
    );
    assert!(replica.repairs().next().is_none());
}

#[test]
fn repair_snapshot_replaces_the_set_and_new_match_context_cannot_retain_old_actions() {
    let mut replica = replica();
    let encoded = postcard::to_vec::<_, 4096>(&snapshot(20)).unwrap();
    let mut baseline: Snapshot = postcard::from_bytes(&encoded).unwrap();
    baseline.records.repairs.clear();
    assert_eq!(
        replica.apply_snapshot(&baseline, Instant::now()),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert!(replica.repairs().next().is_none());
    let mut peer = Replica::new(20);
    peer.set_context(ReplicaContext::new(content(), 1));
    let mut peer_snapshot = snapshot(1);
    peer_snapshot.records.repairs = vec![RepairRecord {
        repairer: UnitId(3),
        target: UnitId(5),
        state: RepairState::Working,
    }];
    assert_eq!(
        peer.apply_snapshot(&peer_snapshot, Instant::now()),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert!(peer.repair(1).is_none());
    assert!(peer.repair(3).is_some());
}

#[test]
fn malformed_repair_snapshots_refuse_the_entire_candidate() {
    for case in [
        "duplicate",
        "missing_source",
        "missing_target",
        "foreign_source",
        "no_capability",
        "item_resource",
        "unknown_resource",
        "source_site",
        "target_site",
    ] {
        let mut replica = replica();
        let mut invalid = snapshot(20);
        invalid.economy.balances.push(BalanceState {
            resource: "skirmish:credits".into(),
            on_hand: 99,
            reserved: 0,
        });
        match case {
            "duplicate" => invalid
                .records
                .repairs
                .push(action(4, RepairState::Working)),
            "missing_source" => invalid.records.repairs[0].repairer = UnitId(99),
            "missing_target" => invalid.records.repairs[0].target = UnitId(99),
            "foreign_source" => invalid.records.repairs[0].repairer = UnitId(3),
            "no_capability" => invalid.records.repairs[0].repairer = UnitId(2),
            "item_resource" => {
                invalid.records.repairs[0].state =
                    RepairState::WaitingForFunds("skirmish:wood".into())
            }
            "unknown_resource" => {
                invalid.records.repairs[0].state =
                    RepairState::WaitingForFunds("skirmish:missing".into())
            }
            "source_site" => invalid.units[0].site = true,
            "target_site" => invalid.units[1].site = true,
            _ => unreachable!(),
        }
        assert_eq!(
            replica.apply_snapshot(&invalid, Instant::now()),
            SnapshotOutcome::Invalid,
            "{case}"
        );
        assert_eq!(replica.last_applied_tick, Some(10), "{case}");
        assert!(
            replica.economy.balances.is_empty(),
            "{case} leaked a balance update"
        );
        assert_eq!(
            replica.repair(1),
            Some(&action(2, RepairState::Approaching)),
            "{case}"
        );
    }
}

#[test]
fn malformed_repair_deltas_refuse_replacements_clears_and_other_updates_atomically() {
    for case in [
        "duplicate",
        "duplicate_clear",
        "overlap",
        "foreign_clear",
        "missing_clear",
        "source_removed_write",
        "source_removed_clear",
        "target_removed_write",
        "target_removed_without_clear",
    ] {
        let mut replica = replica();
        let mut invalid = delta(11);
        invalid.economy.balances.push(BalanceState {
            resource: "skirmish:credits".into(),
            on_hand: 99,
            reserved: 0,
        });
        match case {
            "duplicate" => {
                invalid.records.repairs = vec![
                    action(2, RepairState::Working),
                    action(4, RepairState::Working),
                ]
            }
            "duplicate_clear" => invalid.records.repairs_cleared = vec![UnitId(1), UnitId(1)],
            "overlap" => {
                invalid.records.repairs = vec![action(2, RepairState::Working)];
                invalid.records.repairs_cleared = vec![UnitId(1)];
            }
            "foreign_clear" => invalid.records.repairs_cleared = vec![UnitId(3)],
            "missing_clear" => invalid.records.repairs_cleared = vec![UnitId(99)],
            "source_removed_write" => {
                invalid.removed = vec![UnitId(1)];
                invalid.records.repairs = vec![action(2, RepairState::Working)];
            }
            "source_removed_clear" => {
                invalid.removed = vec![UnitId(1)];
                invalid.records.repairs_cleared = vec![UnitId(1)];
            }
            "target_removed_write" => {
                invalid.removed = vec![UnitId(2)];
                invalid.records.repairs = vec![action(2, RepairState::Working)];
            }
            "target_removed_without_clear" => invalid.removed = vec![UnitId(2)],
            _ => unreachable!(),
        }
        assert_eq!(
            replica.apply_delta(&invalid, Instant::now()),
            DeltaOutcome::ResyncRequested,
            "{case}"
        );
        assert_eq!(replica.last_applied_tick, Some(10), "{case}");
        assert!(
            replica.economy.balances.is_empty(),
            "{case} leaked a balance update"
        );
        assert!(
            replica.units.contains_key(&1) && replica.units.contains_key(&2),
            "{case} removed an endpoint"
        );
        assert_eq!(
            replica.repair(1),
            Some(&action(2, RepairState::Approaching)),
            "{case}"
        );
    }
}
