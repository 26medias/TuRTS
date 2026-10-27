//! Task 8b: replica rules and sampling.

use bevy::platform::time::Instant;
use turts_client::replica::{DeltaOutcome, Replica, SnapshotOutcome};
use turts_net_protocol::{Pos, SlotId, Snapshot, UnitDelta, UnitId, UnitSnapshot, WorldDelta};

fn pos(x: f32, y: f32) -> Pos {
    Pos { x, y }
}

fn snapshot(tick: u32, units: &[(u32, Pos)]) -> Snapshot {
    Snapshot {
        tick,
        nodes: vec![],
        economy: turts_net_protocol::EconomyState {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        units: units
            .iter()
            .map(|&(id, p)| UnitSnapshot {
                unit_id: UnitId(id),
                content_id: "unit".to_string(),
                owner_slot: SlotId(0),
                pos: p,
            })
            .collect(),
    }
}

fn delta(tick: u32, base_tick: u32, positions: &[(u32, Pos)]) -> WorldDelta {
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
        positions: positions
            .iter()
            .map(|&(id, p)| UnitDelta {
                unit_id: UnitId(id),
                pos: p,
            })
            .collect(),
    }
}

fn ticks(r: &Replica, unit: u32) -> Vec<u32> {
    r.units[&unit].samples.iter().map(|&(t, _)| t).collect()
}

#[test]
fn new_replica_is_in_resync_and_buffers_deltas() {
    let now = Instant::now();
    let mut r = Replica::new(20);
    assert!(r.in_resync());
    assert_eq!(r.presentation_time(now), None);
    assert_eq!(
        r.apply_delta(&delta(11, 10, &[(1, pos(1.0, 0.0))]), now),
        DeltaOutcome::Buffered
    );
    assert_eq!(
        r.apply_snapshot(&snapshot(10, &[(1, pos(0.0, 0.0))]), now),
        SnapshotOutcome::Applied { replayed: 1 }
    );
    assert!(!r.in_resync());
    assert_eq!(r.last_applied_tick, Some(11));
    assert_eq!(r.snapshot_tick, Some(10));
}

#[test]
fn delta_at_or_below_snapshot_tick_is_discarded() {
    let now = Instant::now();
    let mut r = Replica::new(20);
    r.apply_snapshot(&snapshot(10, &[(1, pos(0.0, 0.0))]), now);
    assert_eq!(
        r.apply_delta(&delta(10, 9, &[]), now),
        DeltaOutcome::Discarded
    );
    assert_eq!(
        r.apply_delta(&delta(11, 10, &[]), now),
        DeltaOutcome::Applied
    );
    assert_eq!(r.last_applied_tick, Some(11));
}

#[test]
fn unchanged_unit_gets_a_sample_each_tick() {
    let now = Instant::now();
    let mut r = Replica::new(20);
    r.apply_snapshot(
        &snapshot(10, &[(1, pos(0.0, 0.0)), (2, pos(5.0, 5.0))]),
        now,
    );
    assert_eq!(
        r.apply_delta(&delta(11, 10, &[(1, pos(1.0, 0.0))]), now),
        DeltaOutcome::Applied
    );
    assert_eq!(r.units[&1].samples.back(), Some(&(11, pos(1.0, 0.0))));
    assert_eq!(r.units[&2].samples.back(), Some(&(11, pos(5.0, 5.0))));
    assert_eq!(ticks(&r, 2), vec![10, 11]);
}

#[test]
fn sample_interpolates_between_consecutive_ticks() {
    let now = Instant::now();
    let mut r = Replica::new(20);
    r.apply_snapshot(&snapshot(10, &[(1, pos(0.0, 0.0))]), now);
    r.apply_delta(&delta(11, 10, &[(1, pos(2.0, 0.0))]), now);
    let p = r.sample(1, 10.25).unwrap();
    assert_eq!(p.pos, pos(0.5, 0.0));
    assert_eq!((p.lo_tick, p.hi_tick), (10, 11));
    assert_eq!(p.fraction, 0.25);
}

#[test]
fn sample_at_an_integer_time_below_latest_brackets_the_next_tick() {
    // Design section 5: the bracket is floor(t) and floor(t) + 1 whenever a sample
    // exists at floor(t) + 1. The browser often samples at an integer t (T7 found it).
    let now = Instant::now();
    let mut r = Replica::new(20);
    r.apply_snapshot(&snapshot(10, &[(1, pos(0.0, 0.0))]), now);
    r.apply_delta(&delta(11, 10, &[(1, pos(2.0, 0.0))]), now);
    let p = r.sample(1, 10.0).unwrap();
    assert_eq!(p.pos, pos(0.0, 0.0));
    assert_eq!((p.lo_tick, p.hi_tick), (10, 11));
    assert_eq!(p.fraction, 0.0);
}

#[test]
fn sample_with_one_sample_is_that_sample() {
    let now = Instant::now();
    let mut r = Replica::new(20);
    r.apply_snapshot(&snapshot(10, &[(1, pos(3.0, 4.0))]), now);
    for t in [8.5, 9.0, 9.99, 10.0] {
        let p = r.sample(1, t).unwrap();
        assert_eq!(p.pos, pos(3.0, 4.0), "t = {t}");
        assert_eq!((p.lo_tick, p.hi_tick, p.fraction), (10, 10, 0.0), "t = {t}");
    }
    assert!(r.sample(2, 10.0).is_none());
}

#[test]
fn sample_never_extrapolates_past_latest() {
    let now = Instant::now();
    let mut r = Replica::new(20);
    r.apply_snapshot(&snapshot(10, &[(1, pos(0.0, 0.0))]), now);
    r.apply_delta(&delta(11, 10, &[(1, pos(2.0, 0.0))]), now);
    let p = r.sample(1, 11.0).unwrap();
    assert_eq!(p.pos, pos(2.0, 0.0));
    assert_eq!(p.fraction, 0.0);
    assert_eq!((p.lo_tick, p.hi_tick), (11, 11));
    let p = r.sample(1, 11.5).unwrap();
    assert_eq!(p.pos, pos(2.0, 0.0));
    assert_eq!((p.lo_tick, p.hi_tick, p.fraction), (11, 11, 0.0));
}

#[test]
fn samples_are_pruned() {
    let now = Instant::now();
    let mut r = Replica::new(20);
    r.apply_snapshot(&snapshot(1, &[(1, pos(0.0, 0.0))]), now);
    assert_eq!(
        r.apply_delta(&delta(2, 1, &[(1, pos(1.0, 0.0))]), now),
        DeltaOutcome::Applied
    );
    assert_eq!(
        r.apply_delta(&delta(3, 2, &[(1, pos(2.0, 0.0))]), now),
        DeltaOutcome::Applied
    );
    for k in 4..=41 {
        assert_eq!(
            r.apply_delta(&delta(k, k - 1, &[(1, pos(k as f32, 0.0))]), now),
            DeltaOutcome::Applied
        );
    }
    assert_eq!(r.last_applied_tick, Some(41));
    let held = ticks(&r, 1);
    assert!(held.len() <= 18, "unit holds {} samples", held.len());
    assert_eq!(*held.last().unwrap(), 41);
    assert!(
        held.windows(2).all(|w| w[1] == w[0] + 1),
        "samples not consecutive: {held:?}"
    );
}
