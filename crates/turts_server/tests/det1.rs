//! Two-process determinism acceptance for the real fixed-tick Muster command script.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use support::{mc, spawn_server};

fn run_once(
    label: &str,
    script: &Path,
    selected_ids: &BTreeSet<u32>,
) -> (Vec<String>, Vec<u8>, PathBuf) {
    let dir = std::env::temp_dir().join(format!("det1-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("DET1 process evidence directory is created");
    let package = mc::muster_package(&dir);
    let order_log = dir.join("orders.log");
    let script_arg = script.to_str().unwrap();
    let order_arg = order_log.to_str().unwrap();
    let mut server = spawn_server(
        &package,
        &[
            "--command-script",
            script_arg,
            "--run-ticks",
            "420",
            "--order-log",
            order_arg,
        ],
    );
    let started = Instant::now();
    loop {
        if let Some(status) = server
            .child
            .try_wait()
            .expect("DET1 process state is readable")
        {
            assert!(
                status.success(),
                "DET1 {label} server failed: {}",
                server.log()
            );
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(90),
            "DET1 {label} process did not stop at --run-ticks 420: {}",
            server.log()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let ticks = fs::read_to_string(&server.tick_log).expect("DET1 tick log exists");
    let position_rows = mc::read_positions(&server.tick_log);
    mc::assert_actual_tick_coverage(&position_rows, 420, selected_ids);
    let orders = fs::read(&order_log).expect("DET1 order log exists");
    assert!(ticks.ends_with('\n'), "DET1 tick log is incomplete");
    assert!(
        !orders.is_empty(),
        "DET1 order log has no complete transitions"
    );
    assert!(orders.ends_with(b"\n"), "DET1 order log is incomplete");
    for (index, line) in orders.split_inclusive(|byte| *byte == b'\n').enumerate() {
        assert!(
            line.ends_with(b"\n"),
            "DET1 order log line {} is incomplete",
            index + 1
        );
        assert_eq!(
            line.iter().filter(|byte| **byte == b',').count(),
            3,
            "DET1 order log line {} does not have tick,unit_id,state,reason schema",
            index + 1
        );
    }
    let evidence = server.dir.clone();
    drop(server);
    (ticks.lines().map(str::to_owned).collect(), orders, evidence)
}

#[test]
#[ignore = "MC Task22 two-process acceptance; run under the assigned compiler lease"]
fn det1_fixed_tick_muster_script_is_equal_through_tick_400_and_has_neighbour_work() {
    let script = mc::repo_root().join("tests/browser/fixtures/gm40-script.json");
    let entries = mc::read_script(&script);
    let selected_ids = mc::selected_ids_in_script(&entries);
    let first = run_once("a", &script, &selected_ids);
    let second = run_once("b", &script, &selected_ids);
    let a = normalized_ticks(&first.0, 400);
    let b = normalized_ticks(&second.0, 400);
    assert_eq!(
        a, b,
        "DET1 projected tick logs differ through tick 400 after removing only wall_ns"
    );
    assert_eq!(
        through(&first.1, 400),
        through(&second.1, 400),
        "DET1 order transition logs differ byte-for-byte through tick 400"
    );
    assert_full_tick_unit_coverage(&a, 400);
    let active_ticks = neighbour_work_ticks(&a, 1..=40, 1.05);
    assert!(
        active_ticks > 0,
        "DET1 script had no tick where a selected trooper had at least two neighbours inside 1.05 units; HashMap iteration sensitivity was not exercised"
    );
    eprintln!(
        "DET1 semantic work: {active_ticks} ticks with a trooper having >=2 in-reach neighbours"
    );
}

// Preserve the original byte representation of fields 1 and 3-5; this is the plan's exact
// `tick,unit_id,x,y` projection, with only wall_ns removed. Lines after N are intentionally
// ignored because --run-ticks may complete a frame after the Nth fixed tick.
fn normalized_ticks(lines: &[String], max_tick: u32) -> Vec<String> {
    let mut result = Vec::new();
    for line in lines {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(
            fields.len(),
            5,
            "tick log schema is tick,wall_ns,unit_id,x,y: {line}"
        );
        let tick: u32 = fields[0].parse().expect("tick is an integer");
        if tick <= max_tick {
            result.push(format!(
                "{},{},{},{}",
                fields[0], fields[2], fields[3], fields[4]
            ));
        }
    }
    assert!(
        !result.is_empty(),
        "DET1 found no tick-log rows through N={max_tick}"
    );
    result
}

fn through(log: &[u8], max_tick: u32) -> Vec<u8> {
    let mut result = Vec::new();
    for line in log.split_inclusive(|byte| *byte == b'\n') {
        let tick_text = line
            .split(|byte| *byte == b',')
            .next()
            .expect("order tick field");
        let tick: u32 = std::str::from_utf8(tick_text)
            .expect("order tick is UTF-8")
            .parse()
            .expect("order tick is integer");
        if tick <= max_tick {
            result.extend_from_slice(line);
        }
    }
    result
}

fn assert_full_tick_unit_coverage(lines: &[String], max_tick: u32) {
    let mut by_tick = BTreeMap::<u32, BTreeSet<u32>>::new();
    for line in lines {
        let fields: Vec<_> = line.split(',').collect();
        let tick = fields[0].parse().expect("tick integer");
        let unit = fields[1].parse().expect("unit id integer");
        by_tick.entry(tick).or_default().insert(unit);
    }
    let expected = by_tick.get(&1).expect("complete tick 1 exists").clone();
    assert!(
        expected.len() >= 40,
        "actual Muster tick log has fewer than forty units"
    );
    for tick in 1..=max_tick {
        let actual = by_tick
            .get(&tick)
            .unwrap_or_else(|| panic!("DET1 tick {tick} is missing"));
        assert!(
            expected.is_subset(actual),
            "DET1 tick {tick} is missing unit IDs: {:?}",
            expected.difference(actual).collect::<Vec<_>>()
        );
    }
}

fn neighbour_work_ticks(
    lines: &[String],
    troopers: std::ops::RangeInclusive<u32>,
    reach: f32,
) -> usize {
    let ids: BTreeSet<_> = troopers.collect();
    let mut by_tick = BTreeMap::<u32, BTreeMap<u32, (f32, f32)>>::new();
    for line in lines {
        let fields: Vec<_> = line.split(',').collect();
        let tick = fields[0].parse().expect("tick integer");
        let id = fields[1].parse().expect("unit id integer");
        if ids.contains(&id) {
            by_tick.entry(tick).or_default().insert(
                id,
                (
                    fields[2].parse().expect("x is numeric"),
                    fields[3].parse().expect("y is numeric"),
                ),
            );
        }
    }
    by_tick
        .values()
        .filter(|positions| {
            positions.iter().any(|(unit, point)| {
                positions
                    .iter()
                    .filter(|(other, other_point)| {
                        unit != *other
                            && (point.0 - other_point.0).hypot(point.1 - other_point.1) <= reach
                    })
                    .count()
                    >= 2
            })
        })
        .count()
}
