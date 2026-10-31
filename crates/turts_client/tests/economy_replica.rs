//! Task 12 semantic tests use a real base package catalog.

use std::path::PathBuf;

use bevy::platform::time::Instant;
use turts_client::content::ClientContent;
use turts_client::replica::{
    DeltaOutcome, Replica, ReplicaConfig, ReplicaContext, SnapshotOutcome,
};
use turts_mod_api::{Cost, CostSource};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    BalanceState, EconomyChanges, EconomyState, GatherStage, InventoryState, JobId, JobStage,
    NodeId, NodeState, OrderState, Pos, QueueState, SlotId, Snapshot, UnitDelta, UnitId,
    UnitSnapshot, WorldDelta,
};

fn content() -> ClientContent {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive = PackageArchive::parse(&pack_dir(&base).expect("pack real base package"))
        .expect("parse base package");
    ClientContent::from_archives(&[archive]).expect("resolve base catalog")
}

fn replica(limit: usize) -> Replica {
    let mut replica = Replica::with_config(
        20,
        ReplicaConfig {
            max_buffered_batches: limit,
        },
    );
    replica.set_context(ReplicaContext::new(content(), 0));
    replica
}

fn snapshot(tick: u32) -> Snapshot {
    Snapshot {
        tick,
        units: vec![UnitSnapshot {
            unit_id: UnitId(1),
            content_id: "base:command_center".to_string(),
            owner_slot: SlotId(0),
            pos: Pos { x: 8.0, y: 8.0 },
            facing: None,
        }],
        nodes: vec![NodeState {
            id: NodeId(7),
            content_id: "base:ore_deposit".to_string(),
            pos: Pos { x: 24.0, y: 8.0 },
            remaining: 0,
        }],
        economy: EconomyState {
            balances: vec![BalanceState {
                resource: "base:credits".to_string(),
                on_hand: 500,
                reserved: 0,
            }],
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
        unit_orders: Vec::new(),
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
        unit_orders: Vec::new(),
    }
}

#[test]
fn non_finite_geometry_rejects_snapshot_and_delta_atomically() {
    let now = Instant::now();
    let mut world = replica(8);
    let mut invalid_snapshot = snapshot(10);
    invalid_snapshot.units[0].pos.x = f32::NAN;
    assert_eq!(
        world.apply_snapshot(&invalid_snapshot, now),
        SnapshotOutcome::Invalid
    );
    assert_eq!(world.last_applied_tick, None);

    world.apply_snapshot(&snapshot(10), now);
    let before = world.clone();
    let mut invalid_delta = delta(11, 10);
    invalid_delta.positions.push(UnitDelta {
        unit_id: UnitId(1),
        pos: Pos {
            x: f32::INFINITY,
            y: 2.0,
        },
    });
    invalid_delta.economy.balances.push(BalanceState {
        resource: "base:credits".into(),
        on_hand: 900,
        reserved: 0,
    });
    assert_eq!(
        world.apply_delta(&invalid_delta, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(world.last_applied_tick, before.last_applied_tick);
    assert_eq!(world.units, before.units);
    assert_eq!(world.economy, before.economy);

    let mut invalid_node = snapshot(12);
    invalid_node.nodes[0].pos.y = f32::NEG_INFINITY;
    assert_eq!(
        world.apply_snapshot(&invalid_node, now),
        SnapshotOutcome::Invalid
    );
    let mut spawn_replica = replica(8);
    spawn_replica.apply_snapshot(&snapshot(10), now);
    let mut invalid_spawn = delta(11, 10);
    invalid_spawn.spawned.push(UnitSnapshot {
        unit_id: UnitId(2),
        content_id: "base:rifleman".into(),
        owner_slot: SlotId(0),
        pos: Pos {
            x: 1.0,
            y: f32::NAN,
        },

        facing: None,
    });
    assert_eq!(
        spawn_replica.apply_delta(&invalid_spawn, now),
        DeltaOutcome::ResyncRequested
    );
}

#[test]
fn failed_retained_suffix_never_publishes_a_replay_prefix() {
    let now = Instant::now();
    let mut replica = replica(8);
    replica.apply_snapshot(&snapshot(10), now);
    let mut prefix = delta(12, 11);
    prefix.economy.balances.push(BalanceState {
        resource: "base:credits".into(),
        on_hand: 700,
        reserved: 0,
    });
    replica.apply_delta(&prefix, now);
    let mut invalid_suffix = delta(13, 12);
    invalid_suffix.economy.inventories.push(InventoryState {
        unit: UnitId(99),
        items: vec![],
        port: turts_net_protocol::InventoryPort::Main,
    });
    replica.apply_delta(&invalid_suffix, now);
    let before_units = replica.units.clone();
    let before_economy = replica.economy.clone();
    let mut covering = snapshot(11);
    covering.economy.balances[0].on_hand = 600;
    assert_eq!(
        replica.apply_snapshot(&covering, now),
        SnapshotOutcome::ResyncRequested { replayed: 0 }
    );
    assert_eq!(replica.last_applied_tick, Some(10));
    assert_eq!(replica.units, before_units);
    assert_eq!(replica.economy, before_economy);
}

#[test]
fn duplicate_job_cost_endpoints_reject_snapshot_and_delta() {
    let now = Instant::now();
    let mut replica = replica(8);
    let mut invalid = rich_snapshot(10);
    let duplicate = invalid.economy.queues[0].jobs[0].charged[0].clone();
    invalid.economy.queues[0].jobs[0].charged.push(duplicate);
    assert_eq!(
        replica.apply_snapshot(&invalid, now),
        SnapshotOutcome::Invalid
    );
    assert_eq!(replica.last_applied_tick, None);

    replica.apply_snapshot(&rich_snapshot(10), now);
    let mut change = delta(11, 10);
    let mut queue = rich_snapshot(10).economy.queues[0].clone();
    queue.jobs[0].reserved = vec![
        Cost {
            resource: "base:credits".into(),
            amount: 100,
            source: CostSource::SlotBalance
        };
        2
    ];
    change.economy.queues.push(queue);
    assert_eq!(
        replica.apply_delta(&change, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(replica.last_applied_tick, Some(10));
}

#[test]
fn valid_snapshot_discharges_invalid_initial_snapshot_gap() {
    let now = Instant::now();
    let mut replica = replica(8);
    let mut invalid = snapshot(10);
    invalid.units[0].content_id = "unknown:unit".into();
    assert_eq!(
        replica.apply_snapshot(&invalid, now),
        SnapshotOutcome::Invalid
    );
    assert_eq!(replica.highest_observed_tick, None);
    assert_eq!(
        replica.apply_snapshot(&snapshot(11), now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert!(!replica.in_resync());
    assert_eq!(replica.last_applied_tick, Some(11));
}

#[test]
fn invalid_node_and_position_changes_freeze_the_whole_batch() {
    let now = Instant::now();
    let mut replica = replica(3);
    assert_eq!(
        replica.apply_snapshot(&snapshot(10), now),
        SnapshotOutcome::Applied { replayed: 0 }
    );

    let mut update = delta(11, 10);
    update.positions.push(UnitDelta {
        unit_id: UnitId(1),
        pos: Pos { x: 9.0, y: 8.0 },
    });
    update.nodes.push(NodeState {
        id: NodeId(7),
        content_id: "unknown:node".to_string(),
        pos: Pos { x: 24.0, y: 8.0 },
        remaining: 5,
    });

    assert_eq!(
        replica.apply_delta(&update, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(replica.last_applied_tick, Some(10));
    assert_eq!(replica.sample(1, 10.0).unwrap().pos, Pos { x: 8.0, y: 8.0 });
    assert_eq!(replica.nodes[&NodeId(7)].remaining, 0);
}

#[test]
fn stale_snapshot_does_not_rollback_newer_coherent_state() {
    let now = Instant::now();
    let mut replica = replica(3);
    replica.apply_snapshot(&snapshot(10), now);
    let mut movement = delta(11, 10);
    movement.positions.push(UnitDelta {
        unit_id: UnitId(1),
        pos: Pos { x: 12.0, y: 8.0 },
    });
    assert_eq!(replica.apply_delta(&movement, now), DeltaOutcome::Applied);

    assert_eq!(
        replica.apply_snapshot(&snapshot(10), now),
        SnapshotOutcome::IgnoredStale
    );
    assert_eq!(replica.last_applied_tick, Some(11));
    assert_eq!(
        replica.sample(1, 11.0).unwrap().pos,
        Pos { x: 12.0, y: 8.0 }
    );
}

#[test]
fn duplicate_stale_delta_cannot_reapply_an_absolute_refund() {
    let now = Instant::now();
    let mut replica = replica(3);
    replica.apply_snapshot(&snapshot(10), now);
    let mut refund = delta(11, 10);
    refund.economy.balances.push(BalanceState {
        resource: "base:credits".to_string(),
        on_hand: 600,
        reserved: 0,
    });
    assert_eq!(replica.apply_delta(&refund, now), DeltaOutcome::Applied);
    let once = replica.economy.clone();

    assert_eq!(replica.apply_delta(&refund, now), DeltaOutcome::Discarded);
    assert_eq!(replica.economy, once);
}

fn rich_snapshot(tick: u32) -> Snapshot {
    let mut state = snapshot(tick);
    state.units.push(UnitSnapshot {
        unit_id: UnitId(2),
        content_id: "base:harvester".to_string(),
        owner_slot: SlotId(0),
        pos: Pos { x: 12.0, y: 8.0 },
        facing: None,
    });
    state.economy.inventories.push(InventoryState {
        unit: UnitId(2),
        items: vec![BalanceState {
            resource: "base:ore".to_string(),
            on_hand: 10,
            reserved: 0,
        }],
        port: turts_net_protocol::InventoryPort::Main,
    });
    state.economy.orders.push(OrderState {
        unit: UnitId(2),
        node: Some(NodeId(7)),
        receiver: Some(UnitId(1)),
        stage: GatherStage::ToNode,
    });
    state.economy.queues.push(QueueState {
        producer: UnitId(1),
        jobs: vec![turts_net_protocol::JobState {
            id: JobId(44),
            recipe: "base:train_rifleman".to_string(),
            stage: JobStage::Running,
            elapsed_ticks: 10,
            duration_ticks: 100,
            charged: vec![Cost {
                resource: "base:credits".to_string(),
                amount: 100,
                source: CostSource::SlotBalance,
            }],
            reserved: vec![],
        }],
        enabled: true,
    });
    state
}

#[test]
fn spawn_position_and_private_records_apply_atomically() {
    let now = Instant::now();
    let mut replica = replica(3);
    replica.apply_snapshot(&snapshot(10), now);

    let mut spawn = delta(11, 10);
    spawn.spawned.push(UnitSnapshot {
        unit_id: UnitId(2),
        content_id: "base:harvester".to_string(),
        owner_slot: SlotId(0),
        pos: Pos { x: 12.0, y: 8.0 },
        facing: None,
    });
    spawn.economy.inventories.push(InventoryState {
        unit: UnitId(2),
        items: vec![BalanceState {
            resource: "base:ore".to_string(),
            on_hand: 20,
            reserved: 0,
        }],
        port: turts_net_protocol::InventoryPort::Main,
    });
    spawn.positions.push(UnitDelta {
        unit_id: UnitId(2),
        pos: Pos { x: 13.0, y: 8.0 },
    });
    spawn.economy.orders.push(OrderState {
        unit: UnitId(2),
        node: Some(NodeId(7)),
        receiver: Some(UnitId(1)),
        stage: GatherStage::ToNode,
    });
    assert_eq!(replica.apply_delta(&spawn, now), DeltaOutcome::Applied);
    assert!(replica.units.contains_key(&2));
    assert_eq!(
        replica.units[&2].samples.back().unwrap().1,
        Pos { x: 13.0, y: 8.0 }
    );
    assert_eq!(replica.economy.inventories[0].unit, UnitId(2));
    assert_eq!(replica.economy.orders[0].receiver, Some(UnitId(1)));

    let mut malformed = delta(12, 11);
    malformed.spawned.push(UnitSnapshot {
        unit_id: UnitId(3),
        content_id: "base:rifleman".to_string(),
        owner_slot: SlotId(0),
        pos: Pos { x: 9.0, y: 8.0 },
        facing: None,
    });
    malformed.positions.push(UnitDelta {
        unit_id: UnitId(1),
        pos: Pos { x: 15.0, y: 8.0 },
    });
    malformed.economy.balances.push(BalanceState {
        resource: "base:credits".to_string(),
        on_hand: 300,
        reserved: 0,
    });
    malformed.economy.inventories.push(InventoryState {
        unit: UnitId(99),
        items: vec![],
        port: turts_net_protocol::InventoryPort::Main,
    });
    assert_eq!(
        replica.apply_delta(&malformed, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(replica.last_applied_tick, Some(11));
    assert!(!replica.units.contains_key(&3));
    assert_eq!(replica.economy.balances[0].on_hand, 500);
    assert_eq!(replica.sample(1, 11.0).unwrap().pos, Pos { x: 8.0, y: 8.0 });
}

#[test]
fn full_snapshot_replaces_omitted_records_and_removal_cleans_attached_private_state() {
    let now = Instant::now();
    let mut replica = replica(3);
    replica.apply_snapshot(&rich_snapshot(10), now);
    assert_eq!(replica.economy.queues[0].jobs[0].id, JobId(44));

    let mut removal = delta(11, 10);
    removal.removed.push(UnitId(2));
    assert_eq!(replica.apply_delta(&removal, now), DeltaOutcome::Applied);
    assert!(!replica.units.contains_key(&2));
    assert!(replica.economy.inventories.is_empty());
    assert!(replica.economy.orders.is_empty());
    assert_eq!(replica.economy.queues.len(), 1);

    let replacement = snapshot(12);
    assert_eq!(
        replica.apply_snapshot(&replacement, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert_eq!(replica.economy, replacement.economy);
    assert_eq!(
        replica.nodes,
        [(NodeId(7), replacement.nodes[0].clone())].into()
    );
    assert!(!replica.units.contains_key(&2));
}

#[test]
fn duplicate_absolute_changes_reject_the_entire_delta() {
    let now = Instant::now();
    let mut replica = replica(3);
    replica.apply_snapshot(&snapshot(10), now);
    let mut duplicate = delta(11, 10);
    duplicate.positions.push(UnitDelta {
        unit_id: UnitId(1),
        pos: Pos { x: 40.0, y: 8.0 },
    });
    duplicate.economy.balances.extend([
        BalanceState {
            resource: "base:credits".to_string(),
            on_hand: 600,
            reserved: 0,
        },
        BalanceState {
            resource: "base:credits".to_string(),
            on_hand: 700,
            reserved: 0,
        },
    ]);
    assert_eq!(
        replica.apply_delta(&duplicate, now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(replica.last_applied_tick, Some(10));
    assert_eq!(replica.economy.balances[0].on_hand, 500);
    assert_eq!(replica.sample(1, 10.0).unwrap().pos, Pos { x: 8.0, y: 8.0 });
}

#[test]
fn foreign_private_records_and_unknown_snapshot_content_freeze_the_last_world() {
    let now = Instant::now();
    let mut replica = replica(3);
    replica.apply_snapshot(&rich_snapshot(10), now);

    let mut invalid = rich_snapshot(12);
    invalid.units[1].owner_slot = SlotId(1);
    assert_eq!(
        replica.apply_snapshot(&invalid, now),
        SnapshotOutcome::Invalid
    );
    assert_eq!(replica.last_applied_tick, Some(10));
    assert_eq!(replica.units[&2].owner_slot, 0);

    let mut unknown = snapshot(13);
    unknown.units[0].content_id = "unknown:unit".to_string();
    assert_eq!(
        replica.apply_snapshot(&unknown, now),
        SnapshotOutcome::Invalid
    );
    assert_eq!(replica.last_applied_tick, Some(10));
    assert!(replica.units.contains_key(&2));
}

#[test]
fn lost_extraction_conversion_charge_refund_and_spawn_converge_from_snapshot() {
    fn job(id: u64, stage: JobStage, charged: u64) -> turts_net_protocol::JobState {
        turts_net_protocol::JobState {
            id: JobId(id),
            recipe: "base:train_rifleman".to_string(),
            stage,
            elapsed_ticks: if stage == JobStage::Queued { 0 } else { 10 },
            duration_ticks: 100,
            charged: vec![Cost {
                resource: "base:credits".to_string(),
                amount: charged,
                source: CostSource::SlotBalance,
            }],
            reserved: vec![],
        }
    }

    let initial = rich_snapshot(10);
    let mut cases: Vec<(&str, Snapshot, WorldDelta)> = Vec::new();

    let mut extraction = initial.clone();
    extraction.nodes[0].remaining = 80;
    extraction.economy.inventories[0].items[0].on_hand = 30;
    let mut extracted = delta(11, 10);
    extracted.nodes.push(extraction.nodes[0].clone());
    extracted
        .economy
        .inventories
        .push(extraction.economy.inventories[0].clone());
    cases.push(("extraction", extraction, extracted));

    let mut conversion = initial.clone();
    conversion.economy.balances[0].on_hand += 1;
    conversion.economy.inventories[0].items[0].on_hand -= 1;
    let mut converted = delta(11, 10);
    converted
        .economy
        .balances
        .push(conversion.economy.balances[0].clone());
    converted
        .economy
        .inventories
        .push(conversion.economy.inventories[0].clone());
    cases.push(("conversion", conversion, converted));

    let mut charge = initial.clone();
    charge.economy.balances[0].on_hand -= 100;
    charge.economy.queues[0]
        .jobs
        .push(job(45, JobStage::Queued, 100));
    let mut charged = delta(11, 10);
    charged
        .economy
        .balances
        .push(charge.economy.balances[0].clone());
    charged
        .economy
        .queues
        .push(charge.economy.queues[0].clone());
    cases.push(("charge", charge, charged));

    let mut refund = initial.clone();
    refund.economy.balances[0].on_hand += 50;
    refund.economy.queues[0].jobs.clear();
    let mut refunded = delta(11, 10);
    refunded
        .economy
        .balances
        .push(refund.economy.balances[0].clone());
    refunded
        .economy
        .queues
        .push(refund.economy.queues[0].clone());
    cases.push(("refund", refund, refunded));

    let mut spawn = initial.clone();
    spawn.units.push(UnitSnapshot {
        unit_id: UnitId(3),
        content_id: "base:rifleman".to_string(),
        owner_slot: SlotId(0),
        pos: Pos { x: 8.0, y: 12.0 },
        facing: None,
    });
    spawn.economy.queues[0].jobs.clear();
    let mut spawned = delta(11, 10);
    spawned.spawned.push(spawn.units[2].clone());
    spawned.economy.queues.push(spawn.economy.queues[0].clone());
    cases.push(("spawn", spawn, spawned));

    for (change_name, mut expected, change) in cases {
        expected.tick = 16;
        let mut control = replica(8);
        control.apply_snapshot(&initial, Instant::now());
        assert_eq!(
            control.apply_delta(&change, Instant::now()),
            DeltaOutcome::Applied,
            "{change_name}"
        );
        for tick in 12..=16 {
            assert_eq!(
                control.apply_delta(&delta(tick, tick - 1), Instant::now()),
                DeltaOutcome::Applied,
                "{change_name} control heartbeat {tick}"
            );
        }

        let mut recovering = replica(8);
        recovering.apply_snapshot(&initial, Instant::now());
        // The final economic change is lost; only empty heartbeats follow it.
        for tick in 12..=16 {
            let outcome = recovering.apply_delta(&delta(tick, tick - 1), Instant::now());
            if tick == 12 {
                assert_eq!(
                    outcome,
                    DeltaOutcome::ResyncRequested,
                    "{change_name} heartbeat must reveal the gap"
                );
            } else {
                assert_eq!(
                    outcome,
                    DeltaOutcome::Buffered,
                    "{change_name} heartbeat {tick}"
                );
            }
        }
        assert_eq!(
            recovering.apply_snapshot(&expected, Instant::now()),
            SnapshotOutcome::Applied { replayed: 0 },
            "{change_name} snapshot must converge"
        );
        assert_eq!(recovering.economy, expected.economy, "{change_name}");
        assert_eq!(
            recovering.nodes,
            expected.nodes.iter().cloned().map(|n| (n.id, n)).collect(),
            "{change_name}"
        );
        assert_eq!(
            recovering.units.len(),
            expected.units.len(),
            "{change_name}"
        );
        for unit in &expected.units {
            let held = &recovering.units[&unit.unit_id.0];
            assert_eq!(held.content_id, unit.content_id, "{change_name}");
            assert_eq!(held.owner_slot, unit.owner_slot.0, "{change_name}");
            assert_eq!(held.samples.back().unwrap().1, unit.pos, "{change_name}");
        }
    }
}

#[test]
fn sorted_out_of_order_tail_replays_only_a_complete_chain() {
    let now = Instant::now();
    let mut replica = replica(8);
    assert_eq!(
        replica.apply_delta(&delta(13, 12), now),
        DeltaOutcome::Buffered
    );
    assert_eq!(
        replica.apply_delta(&delta(12, 11), now),
        DeltaOutcome::Buffered
    );
    assert_eq!(
        replica.apply_delta(&delta(11, 10), now),
        DeltaOutcome::Buffered
    );
    let buffered: Vec<_> = replica
        .resync
        .as_ref()
        .unwrap()
        .iter()
        .map(|d| d.tick)
        .collect();
    assert_eq!(buffered, vec![11, 12, 13]);
    assert_eq!(
        replica.apply_snapshot(&snapshot(10), now),
        SnapshotOutcome::Applied { replayed: 3 }
    );
    assert_eq!(replica.last_applied_tick, Some(13));
    assert!(!replica.in_resync());
}

#[test]
fn overflow_discards_old_tail_and_trigger_but_keeps_gap_for_a_covering_snapshot() {
    let now = Instant::now();
    let mut replica = replica(3);
    replica.apply_snapshot(&snapshot(10), now);
    assert_eq!(
        replica.apply_delta(&delta(12, 11), now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(
        replica.apply_delta(&delta(13, 12), now),
        DeltaOutcome::Buffered
    );
    assert_eq!(
        replica.apply_delta(&delta(14, 13), now),
        DeltaOutcome::Buffered
    );
    assert_eq!(
        replica.apply_delta(&delta(15, 14), now),
        DeltaOutcome::Buffered
    );
    assert_eq!(replica.buffer_overflows, 1);
    assert_eq!(replica.buffered_batches(), 0);
    assert_eq!(replica.highest_observed_tick, Some(15));
    assert!(replica.in_resync());
    assert_eq!(
        replica.apply_delta(&delta(16, 15), now),
        DeltaOutcome::Buffered
    );
    assert!(replica.buffered_batches() <= 3);
    assert_eq!(
        replica.apply_snapshot(&snapshot(15), now),
        SnapshotOutcome::Applied { replayed: 1 }
    );
    assert_eq!(replica.last_applied_tick, Some(16));
    assert!(!replica.in_resync());
}

#[test]
fn lowering_live_buffer_cap_clears_oversized_tail_and_records_a_gap() {
    let mut replica = replica(5);
    replica.last_applied_tick = Some(10);
    replica.highest_observed_tick = Some(14);
    replica.resync = Some(vec![delta(12, 11), delta(13, 12), delta(14, 13)]);

    replica.set_max_buffered_batches(2);

    assert_eq!(replica.max_buffered_batches(), 2);
    assert_eq!(replica.buffered_batches(), 0);
    assert_eq!(replica.buffer_overflows, 1);
    assert_eq!(replica.resync_span_ticks(), Some(4));
}
