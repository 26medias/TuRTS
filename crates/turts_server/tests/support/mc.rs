//! Process-level helpers for the MC movement acceptance instruments.
//!
//! These helpers deliberately treat missing or partial process logs as setup/data errors. They
//! never interpret an absent record as a gameplay failure.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use turts_net_protocol::{OrderEnvelope, OrderTarget, Pos, QueueMode, UnitId};

use super::{ServerProc, spawn_server};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ScriptEntry {
    pub tick: u32,
    pub slot: u8,
    pub envelope: OrderEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionRow {
    pub tick: u32,
    pub unit_id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderTransition {
    pub tick: u32,
    pub unit_id: u32,
    pub state: String,
    pub reason: String,
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

pub fn muster_package(dir: &Path) -> PathBuf {
    super::pack("muster", dir)
}

pub fn point_order(ids: Vec<u32>, x: f32, y: f32, group: u32) -> OrderEnvelope {
    OrderEnvelope {
        order: "muster:order_move".into(),
        units: ids.into_iter().map(UnitId).collect(),
        target: OrderTarget::Point(Pos { x, y }),
        queue: QueueMode::Replace,
        group,
    }
}

pub fn gather_order(unit_id: u32) -> OrderEnvelope {
    OrderEnvelope {
        order: "muster:order_gather".into(),
        units: vec![UnitId(unit_id)],
        target: OrderTarget::Node(turts_net_protocol::NodeId(1)),
        queue: QueueMode::Replace,
        group: 0,
    }
}

pub fn write_script(path: &Path, entries: &[(u32, u8, OrderEnvelope)]) {
    let serialized: Vec<_> = entries
        .iter()
        .map(|(tick, slot, envelope)| ScriptEntry {
            tick: *tick,
            slot: *slot,
            envelope: envelope.clone(),
        })
        .collect();
    fs::write(
        path,
        serde_json::to_vec_pretty(&serialized).expect("typed script serializes"),
    )
    .expect("fixed-tick script is written");
}

pub fn read_script(path: &Path) -> Vec<ScriptEntry> {
    let bytes = fs::read(path).unwrap_or_else(|error| {
        panic!(
            "cannot read typed command script {}: {error}",
            path.display()
        )
    });
    serde_json::from_slice(&bytes).unwrap_or_else(|error| {
        panic!("command script does not match typed protocol schema: {error}")
    })
}

pub fn start_muster(dir: &Path, script: &Path, run_ticks: u32, extra: &[&str]) -> ServerProc {
    let package = muster_package(dir);
    let run_ticks = run_ticks.to_string();
    let mut args = vec![
        "--command-script",
        script.to_str().unwrap(),
        "--run-ticks",
        &run_ticks,
    ];
    args.extend_from_slice(extra);
    spawn_server(&package, &args)
}

pub fn finish(
    mut server: ServerProc,
    order_log: &Path,
    run_ticks: u32,
) -> (PathBuf, PathBuf, PathBuf) {
    let started = Instant::now();
    loop {
        if let Some(status) = server.child.try_wait().expect("server process state reads") {
            assert!(
                status.success(),
                "Muster server exited unsuccessfully: {}",
                server.log()
            );
            let output = (
                server.tick_log.clone(),
                order_log.to_path_buf(),
                server.dir.clone(),
            );
            drop(server);
            return output;
        }
        assert!(
            started.elapsed() < Duration::from_secs(90),
            "Muster server did not reach --run-ticks {run_ticks}: {}",
            server.log()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn read_positions(path: &Path) -> Vec<PositionRow> {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("cannot read complete tick log {}: {error}", path.display())
    });
    assert!(
        text.ends_with('\n'),
        "tick log is truncated: {}",
        path.display()
    );
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(
            fields.len(),
            5,
            "tick log line {} has wrong schema: {line}",
            index + 1
        );
        let tick = fields[0].parse().expect("tick is an integer");
        let _wall_ns: u128 = fields[1].parse().expect("wall_ns is an integer");
        let unit_id = fields[2].parse().expect("unit_id is an integer");
        let x: f32 = fields[3].parse().expect("x is numeric");
        let y: f32 = fields[4].parse().expect("y is numeric");
        assert!(
            x.is_finite() && y.is_finite(),
            "tick log has non-finite position at line {}",
            index + 1
        );
        rows.push(PositionRow {
            tick,
            unit_id,
            x,
            y,
        });
    }
    assert!(
        !rows.is_empty(),
        "tick log has no complete rows: {}",
        path.display()
    );
    rows
}

pub fn read_transitions(path: &Path) -> Vec<OrderTransition> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read order log {}: {error}", path.display()));
    assert!(
        text.ends_with('\n'),
        "order log is truncated: {}",
        path.display()
    );
    let rows: Vec<_> = text
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let fields: Vec<_> = line.split(',').collect();
            assert_eq!(
                fields.len(),
                4,
                "order log line {} has wrong schema: {line}",
                index + 1
            );
            OrderTransition {
                tick: fields[0].parse().expect("order tick is an integer"),
                unit_id: fields[1].parse().expect("order unit id is an integer"),
                state: fields[2].to_owned(),
                reason: fields[3].to_owned(),
            }
        })
        .collect();
    rows
}

pub fn selected_ids_in_script(entries: &[ScriptEntry]) -> BTreeSet<u32> {
    entries
        .iter()
        .flat_map(|entry| entry.envelope.units.iter().map(|id| id.0))
        .collect()
}

pub fn by_tick(rows: &[PositionRow]) -> BTreeMap<u32, BTreeMap<u32, (f32, f32)>> {
    let mut result = BTreeMap::new();
    for row in rows {
        assert!(
            result
                .entry(row.tick)
                .or_insert_with(BTreeMap::new)
                .insert(row.unit_id, (row.x, row.y))
                .is_none(),
            "duplicate tick/unit position ({}, {})",
            row.tick,
            row.unit_id
        );
    }
    result
}

pub fn ids_at(rows: &[PositionRow], tick: u32) -> BTreeSet<u32> {
    rows.iter()
        .filter(|row| row.tick == tick)
        .map(|row| row.unit_id)
        .collect()
}

pub fn assert_full_tick_coverage(rows: &[PositionRow], through: u32, units: &BTreeSet<u32>) {
    let ticks = by_tick(rows);
    for tick in 1..=through {
        let actual: BTreeSet<_> = ticks
            .get(&tick)
            .unwrap_or_else(|| panic!("tick log missing tick {tick}"))
            .keys()
            .copied()
            .collect();
        assert!(
            units.is_subset(&actual),
            "tick {tick} lacks selected unit coverage: missing {:?}",
            units.difference(&actual).collect::<Vec<_>>()
        );
    }
}

/// Verify every selected ID appears at every actual tick, including harmless ticks after the
/// configured server limit. A short or sparse log is a data/setup failure before geometry checks.
pub fn assert_actual_tick_coverage(
    rows: &[PositionRow],
    configured_end: u32,
    units: &BTreeSet<u32>,
) -> u32 {
    let ticks = by_tick(rows);
    let actual_end = *ticks
        .keys()
        .next_back()
        .expect("position log has actual ticks");
    assert!(
        actual_end >= configured_end,
        "tick log ends at {actual_end}, before configured --run-ticks {configured_end}"
    );
    assert!(!units.is_empty(), "command script selects actual unit IDs");
    for tick in 1..=actual_end {
        let actual: BTreeSet<_> = ticks
            .get(&tick)
            .unwrap_or_else(|| panic!("tick log is not contiguous: missing tick {tick}"))
            .keys()
            .copied()
            .collect();
        assert!(
            units.is_subset(&actual),
            "tick {tick} lacks selected script units: missing {:?}",
            units.difference(&actual).collect::<Vec<_>>()
        );
    }
    actual_end
}

pub fn assert_rest_and_geometry(
    rows: &[PositionRow],
    ids: &BTreeSet<u32>,
    target: (f32, f32),
    radius: f32,
    max_target_distance: f32,
    rest_ticks: u32,
) {
    let end = rows.iter().map(|row| row.tick).max().unwrap();
    assert_rest_and_geometry_at(
        rows,
        ids,
        target,
        radius,
        max_target_distance,
        rest_ticks,
        end,
    );
}

pub fn assert_rest_and_geometry_at(
    rows: &[PositionRow],
    ids: &BTreeSet<u32>,
    target: (f32, f32),
    radius: f32,
    max_target_distance: f32,
    rest_intervals: u32,
    end_tick: u32,
) {
    assert_stationary_and_disjoint_at(rows, ids, radius, rest_intervals, end_tick);
    let ticks = by_tick(rows);
    let first = end_tick
        .checked_sub(rest_intervals)
        .expect("configured end reaches the complete rest interval");
    for tick in first..=end_tick {
        let positions = ticks.get(&tick).expect("selected rest tick is present");
        for id in ids {
            assert!(
                distance(*positions.get(id).unwrap(), target) <= max_target_distance,
                "unit {id} is outside target radius at tick {tick}"
            );
        }
    }
}

pub fn assert_stationary_and_disjoint(
    rows: &[PositionRow],
    ids: &BTreeSet<u32>,
    radius: f32,
    rest_ticks: u32,
) {
    let end = rows
        .iter()
        .map(|row| row.tick)
        .max()
        .expect("positions have ticks");
    assert_stationary_and_disjoint_at(rows, ids, radius, rest_ticks, end);
}

pub fn assert_stationary_and_disjoint_at(
    rows: &[PositionRow],
    ids: &BTreeSet<u32>,
    radius: f32,
    rest_intervals: u32,
    end_tick: u32,
) {
    let ticks = by_tick(rows);
    assert!(
        end_tick >= rest_intervals,
        "configured endpoint is shorter than the at-rest window"
    );
    let first = end_tick - rest_intervals;
    let baseline = ticks
        .get(&first)
        .expect("rest interval starts with an actual tick");
    for tick in first..=end_tick {
        let positions = ticks.get(&tick).expect("selected rest tick is present");
        for id in ids {
            let pos = positions
                .get(id)
                .unwrap_or_else(|| panic!("tick {tick} missing selected unit {id}"));
            assert!(
                distance(
                    *baseline
                        .get(id)
                        .expect("rest baseline contains selected unit"),
                    *pos
                ) <= 0.001,
                "unit {id} moved more than 0.001 over the rest interval at tick {tick}"
            );
        }
        assert_no_overlap(positions, ids, radius, tick);
    }
}

pub fn moving_units_between(
    rows: &[PositionRow],
    ids: &BTreeSet<u32>,
    from_tick: u32,
    through_tick: u32,
) -> BTreeSet<u32> {
    let ticks = by_tick(rows);
    let mut moving = BTreeSet::new();
    for tick in from_tick.saturating_add(1)..=through_tick {
        let prior = ticks
            .get(&(tick - 1))
            .unwrap_or_else(|| panic!("motion witness lacks prior tick {}", tick - 1));
        let current = ticks
            .get(&tick)
            .unwrap_or_else(|| panic!("motion witness lacks tick {tick}"));
        for id in ids {
            let before = prior
                .get(id)
                .unwrap_or_else(|| panic!("motion witness prior tick lacks unit {id}"));
            let after = current
                .get(id)
                .unwrap_or_else(|| panic!("motion witness tick {tick} lacks unit {id}"));
            if distance(*before, *after) > 0.001 {
                moving.insert(*id);
            }
        }
    }
    moving
}

pub fn assert_no_overlap(
    positions: &BTreeMap<u32, (f32, f32)>,
    ids: &BTreeSet<u32>,
    radius: f32,
    tick: u32,
) {
    let ordered: Vec<_> = ids
        .iter()
        .map(|id| {
            (
                *id,
                *positions
                    .get(id)
                    .unwrap_or_else(|| panic!("tick {tick} missing selected unit {id}")),
            )
        })
        .collect();
    for (index, (left_id, left)) in ordered.iter().enumerate() {
        for (right_id, right) in ordered.iter().skip(index + 1) {
            assert!(
                distance(*left, *right) >= 2.0 * radius - 0.001,
                "units {left_id} and {right_id} overlap at tick {tick}"
            );
        }
    }
}

pub fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

pub fn blocked_ids(rows: &[OrderTransition]) -> BTreeSet<u32> {
    rows.iter()
        .filter(|row| row.state == "Blocked")
        .map(|row| row.unit_id)
        .collect()
}
