//! Task 8b: T8 resync logic (design section 5, G2: resync starts at Welcome).

use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

use bevy::platform::time::Instant;
use turts_client::content::ClientContent;
use turts_client::replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{Pos, SlotId, Snapshot, UnitDelta, UnitId, UnitSnapshot, WorldDelta};

/// Unit 1 is at x = tick on every tick, so a position shows the tick it came from.
fn snapshot(tick: u32) -> Snapshot {
    Snapshot {
        tick,
        nodes: vec![],
        economy: turts_net_protocol::EconomyState {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        units: vec![UnitSnapshot {
            unit_id: UnitId(1),
            content_id: "base:command_center".to_string(),
            owner_slot: SlotId(0),
            pos: Pos {
                x: tick as f32,
                y: 0.0,
            },

            facing: None,
        }],
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
        nodes: vec![],
        economy: turts_net_protocol::EconomyChanges {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        positions: vec![UnitDelta {
            unit_id: UnitId(1),
            pos: Pos {
                x: tick as f32,
                y: 0.0,
            },
        }],
        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
        unit_orders: Vec::new(),
    }
}

fn replica() -> Replica {
    static CONTENT: OnceLock<ClientContent> = OnceLock::new();
    let content = CONTENT
        .get_or_init(|| {
            let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
            let archive =
                PackageArchive::parse(&pack_dir(&base).expect("pack base")).expect("parse base");
            ClientContent::from_archives(&[archive]).expect("resolve base")
        })
        .clone();
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content, 0));
    replica
}

fn buffered_ticks(r: &Replica) -> Vec<u32> {
    r.resync
        .as_ref()
        .expect("in resync")
        .iter()
        .map(|d| d.tick)
        .collect()
}

/// Snapshot 10, deltas 11 and 12 applied, 13 lost, 14 requests resync, 15 is buffered.
fn replica_after_loss_of_13(now: Instant) -> Replica {
    let mut r = replica();
    assert_eq!(
        r.apply_snapshot(&snapshot(10), now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert_eq!(r.apply_delta(&delta(11, 10), now), DeltaOutcome::Applied);
    assert_eq!(r.apply_delta(&delta(12, 11), now), DeltaOutcome::Applied);
    assert_eq!(
        r.apply_delta(&delta(14, 13), now),
        DeltaOutcome::ResyncRequested
    );
    assert_eq!(r.apply_delta(&delta(15, 14), now), DeltaOutcome::Buffered);
    r
}

#[test]
fn t8_missing_delta_requests_resync_once_and_buffers() {
    let now = Instant::now();
    let r = replica_after_loss_of_13(now);
    assert!(r.in_resync());
    assert_eq!(r.last_applied_tick, Some(12));
    assert_eq!(buffered_ticks(&r), vec![14, 15]);
}

#[test]
fn t8_snapshot_replays_buffer_and_drops_old() {
    let now = Instant::now();
    let mut r = replica_after_loss_of_13(now);
    assert_eq!(
        r.apply_snapshot(&snapshot(15), now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert!(!r.in_resync());
    assert_eq!(r.last_applied_tick, Some(15));
    assert_eq!(r.apply_delta(&delta(16, 15), now), DeltaOutcome::Applied);
    assert_eq!(r.last_applied_tick, Some(16));
}

#[test]
fn t8_late_snapshot_replays_newer_deltas() {
    let now = Instant::now();
    let mut r = replica_after_loss_of_13(now);
    for k in 16..=18 {
        assert_eq!(r.apply_delta(&delta(k, k - 1), now), DeltaOutcome::Buffered);
    }
    assert_eq!(
        r.apply_snapshot(&snapshot(15), now),
        SnapshotOutcome::Applied { replayed: 3 }
    );
    assert!(!r.in_resync());
    assert_eq!(r.last_applied_tick, Some(18));
    let held: Vec<u32> = r.units[&1].samples.iter().map(|&(t, _)| t).collect();
    assert_eq!(held, vec![15, 16, 17, 18]);
}

#[test]
fn t8_broken_chain_during_replay_starts_one_new_resync() {
    let now = Instant::now();
    let mut r = replica_after_loss_of_13(now);
    assert_eq!(r.apply_delta(&delta(16, 15), now), DeltaOutcome::Buffered);
    assert_eq!(r.apply_delta(&delta(18, 17), now), DeltaOutcome::Buffered);
    // A disconnected component keeps only its newer tail; the missing coverage remains known.
    assert_eq!(buffered_ticks(&r), vec![18]);
    assert_eq!(
        r.apply_snapshot(&snapshot(15), now),
        SnapshotOutcome::ResyncRequested { replayed: 0 }
    );
    assert!(r.in_resync());
    // An incomplete retained tail keeps the previously coherent display frozen.
    assert_eq!(r.last_applied_tick, Some(12));
    assert_eq!(buffered_ticks(&r), vec![18]);
}

/// A port of `m1-gate2/net/t8model2.py`. Welcome at tick 100; deltas 110 and 190 are lost;
/// each answer snapshot arrives one tick after its request; the Welcome snapshot is never
/// held, but arrives 0 or 2 ticks late; answer snapshots are held 0 or 3 ticks.
/// Ingest order per tick: release held snapshots, then arriving snapshots, then the delta.
fn run_model(hold: u32, welcome_lag: u32) -> (u32, Replica) {
    const W: u32 = 100;
    const END: u32 = 300;
    const RTT: u32 = 1;
    let drops = [110, 190];
    let now = Instant::now();
    let mut r = replica();
    let mut held: Vec<(u32, u32)> = Vec::new();
    let mut requests = 0;
    let mut snaps: BTreeMap<u32, (u32, bool)> = BTreeMap::new();
    snaps.insert(W + welcome_lag, (W, false));
    for tick in W..END {
        let ready: Vec<(u32, u32)> = held.iter().copied().filter(|h| h.1 < tick).collect();
        for h in ready {
            held.retain(|x| *x != h);
            if let SnapshotOutcome::ResyncRequested { .. } = r.apply_snapshot(&snapshot(h.0), now) {
                requests += 1;
                snaps.insert(tick + RTT, (tick + RTT, true));
            }
        }
        if let Some((s, answer)) = snaps.remove(&tick) {
            if answer && hold > 0 {
                held.push((s, s + hold));
            } else if let SnapshotOutcome::ResyncRequested { .. } =
                r.apply_snapshot(&snapshot(s), now)
            {
                requests += 1;
                snaps.insert(tick + RTT, (tick + RTT, true));
            }
        }
        if tick == W || drops.contains(&tick) {
            continue;
        }
        if r.apply_delta(&delta(tick, tick - 1), now) == DeltaOutcome::ResyncRequested {
            requests += 1;
            snaps.insert(tick + RTT, (tick + RTT, true));
        }
    }
    (requests, r)
}

#[test]
fn t8_model_scenarios_match_python_model() {
    for hold in [0, 3] {
        for welcome_lag in [0, 2] {
            let (requests, r) = run_model(hold, welcome_lag);
            let label = format!("hold {hold}, welcome late by {welcome_lag}");
            assert_eq!(requests, 2, "{label}");
            assert_eq!(r.last_applied_tick, Some(299), "{label}");
            assert!(!r.in_resync(), "{label}");
            let p = r.sample(1, 299.0).expect("unit 1");
            assert_eq!(p.pos, Pos { x: 299.0, y: 0.0 }, "{label}");
        }
    }
}

#[test]
fn delta_with_unknown_unit_requests_resync() {
    let now = Instant::now();
    let mut r = replica();
    assert_eq!(
        r.apply_snapshot(&snapshot(10), now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    let unknown = WorldDelta {
        tick: 11,
        base_tick: 10,
        spawned: vec![],
        removed: vec![],
        nodes: vec![],
        economy: turts_net_protocol::EconomyChanges {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        positions: vec![UnitDelta {
            unit_id: UnitId(9),
            pos: Pos { x: 1.0, y: 1.0 },
        }],

        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
        unit_orders: Vec::new(),
    };
    assert_eq!(r.apply_delta(&unknown, now), DeltaOutcome::ResyncRequested);
    assert!(r.in_resync());
    assert!(!r.units.contains_key(&9), "no invented unit");
    assert_eq!(r.last_applied_tick, Some(10));
    assert_eq!(r.units[&1].samples.len(), 1, "no partial apply");
    assert_eq!(
        r.apply_delta(&delta(12, 11), now),
        DeltaOutcome::Buffered,
        "one request only"
    );
}
