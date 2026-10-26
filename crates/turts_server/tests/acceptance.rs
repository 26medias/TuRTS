//! Acceptance runs T2a, T2d, T3, T4, T5, T6, T6b and T8 (design section 7) with real
//! `turts_server` and `turts_client` processes. The process tests carry `#[ignore]` and run
//! only through `scripts/acceptance.sh`, in release and one at a time. Each test packs its
//! package into its own directory and starts its own server.
//!
//! The tests at the end of the file feed made-up logs to the segment oracle. They start no
//! process, so `cargo test --workspace` runs them: they show that the oracle can go red.

mod support;

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use support::logs::{
    FrameRecord, FrameSummary, TickRecord, arrival_tick, read_frame_log, read_stats, read_tick_log,
    read_tick_log_past, wait_for_arrival,
};
use support::oracle::{OracleReport, check_segments, final_position};
use support::{ServerProc, pack, pack_with, run_client, spawn_server, test_dir, wait_json};

/// The unit that the map of `base` places, for slot 0.
const UNIT: u32 = 1;
/// The number of placements in the map of `base`.
const BASE_PLACEMENTS: u64 = 1;
/// Where the map of `base` places the unit.
const HOME: (f32, f32) = (8.0, 8.0);
/// The `move` target: 12 units at 3.0 per second is 80 ticks at 20 Hz.
const TARGET: (f32, f32) = (20.0, 8.0);
/// The tolerance of a position compare.
const POS_TOLERANCE: f32 = 1e-3;
/// The time a client may take after its `--run-secs` to write its files and exit.
const EXIT_MARGIN: Duration = Duration::from_secs(15);
/// Lag p99 limit of T5, in ticks. Design section 7 says 2; the studio decision "Server TCP
/// no-delay and tighter T5 bound" sets 1.5, so that a Nagle delay on the server goes red.
const LAG_P99_LIMIT: f64 = 1.5;

fn path_text(path: &Path) -> &str {
    path.to_str().expect("the test paths are UTF-8")
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() <= POS_TOLERANCE && (a.1 - b.1).abs() <= POS_TOLERANCE
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_millis() as u64
}

/// A headless client that runs for `run_secs` and writes `<name>.json` (`--out`) and
/// `<name>.frames` (`--frame-log`) into the test directory.
struct ClientRun {
    child: Child,
    out: PathBuf,
    frames: PathBuf,
    run_secs: u64,
}

/// What a client left behind.
struct Finished {
    report: Value,
    frames: PathBuf,
}

impl Finished {
    fn frame_log(&self) -> (Vec<FrameRecord>, FrameSummary) {
        read_frame_log(&self.frames)
    }
}

fn start_client(
    server: &ServerProc,
    dir: &Path,
    name: &str,
    run_secs: u64,
    extra: &[&str],
) -> ClientRun {
    let ws = format!("ws://{}", server.ws_addr);
    let out = dir.join(format!("{name}.json"));
    let frames = dir.join(format!("{name}.frames"));
    let run_secs_text = run_secs.to_string();
    let mut args = vec![
        "--headless",
        "--server",
        &server.http_url,
        "--ws",
        &ws,
        "--run-secs",
        &run_secs_text,
        "--out",
        path_text(&out),
        "--frame-log",
        path_text(&frames),
    ];
    args.extend_from_slice(extra);
    ClientRun {
        child: run_client(&args),
        out,
        frames,
        run_secs,
    }
}

impl ClientRun {
    fn finish(self) -> Finished {
        let timeout = Duration::from_secs(self.run_secs) + EXIT_MARGIN;
        Finished {
            report: wait_json(self.child, &self.out, timeout),
            frames: self.frames,
        }
    }
}

/// The records of one unit, in file order.
fn frames_of(frames: &[FrameRecord], unit_id: u32) -> Vec<&FrameRecord> {
    frames
        .iter()
        .filter(|frame| frame.unit_id == unit_id)
        .collect()
}

/// The tick log once it holds a tick after the last frame, so that every frame lies between
/// two logged ticks.
fn ticks_for(server: &ServerProc, frames: &[FrameRecord]) -> Vec<TickRecord> {
    let last_frame_ns = frames.iter().map(|frame| frame.frame_ns).max().unwrap_or(0);
    read_tick_log_past(&server.tick_log, last_frame_ns, Duration::from_secs(2))
}

/// The tick log grouped by tick.
fn by_tick(ticks: &[TickRecord]) -> BTreeMap<u32, Vec<&TickRecord>> {
    let mut grouped: BTreeMap<u32, Vec<&TickRecord>> = BTreeMap::new();
    for record in ticks {
        grouped.entry(record.tick).or_default().push(record);
    }
    grouped
}

/// The position of `unit_id` at `tick` in the tick log.
fn pos_at(ticks: &[TickRecord], unit_id: u32, tick: u32) -> (f32, f32) {
    ticks
        .iter()
        .find(|record| record.unit_id == unit_id && record.tick == tick)
        .unwrap_or_else(|| panic!("the tick log has no unit {unit_id} at tick {tick}"))
        .pos
}

/// The stats file as the server wrote it after `after_ms` (UNIX milliseconds). The server
/// writes the file first 1 s after its start and then rewrites it each second, so this waits
/// for at most 5 s, also for the first write.
fn stats_after(path: &Path, after_ms: u64) -> Value {
    let started = Instant::now();
    loop {
        let stats = if path.exists() {
            read_stats(path)
        } else {
            Value::Null
        };
        if stats["unix_ms"]
            .as_u64()
            .is_some_and(|written| written > after_ms)
        {
            return stats;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the stats file was not rewritten after {after_ms}: {stats}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Print an oracle result and append it to `target/tmp/acceptance-oracle.log`, so the numbers
/// of a passing run stay on record.
fn record(label: &str, report: &OracleReport) {
    let line = format!(
        "{} {label}: frames={} lag_p99={:.3} lag_max={:.3}",
        now_ms(),
        report.frames,
        report.lag_p99,
        report.lag_max
    );
    println!("{line}");
    let log = Path::new(env!("CARGO_TARGET_TMPDIR")).join("acceptance-oracle.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log) {
        let _ = writeln!(file, "{line}");
    }
}

/// T2a package parity: the server spawns exactly the units of the package's map, and the
/// client presents their content ids with the shape from the downloaded visuals.
#[test]
#[ignore]
fn t2a_package_parity() {
    for (package, expected_id, expected_shape, expected_pos) in [
        ("base", "base:rifleman", "Capsule", (8.0, 8.0)),
        ("acme", "acme:scout", "Cuboid", (4.0, 4.0)),
    ] {
        let dir = test_dir(&format!("t2a_{package}"));
        let server = spawn_server(&pack(package, &dir), &[]);
        let run = start_client(&server, &dir, "client", 3, &[]).finish();
        let report = &run.report;

        let finals = report["final_positions"]
            .as_object()
            .unwrap_or_else(|| panic!("{package}: no final_positions: {report}"));
        assert_eq!(finals.len(), 1, "{package}: one unit: {report}");
        let (_, pos) = finals.iter().next().expect("one entry");
        let pos = (
            pos[0].as_f64().expect("x") as f32,
            pos[1].as_f64().expect("y") as f32,
        );
        assert!(
            near(pos, expected_pos),
            "{package}: expected_pos {expected_pos:?}, client has {pos:?}"
        );

        let units = report["debug"]["units"]
            .as_array()
            .unwrap_or_else(|| panic!("{package}: no debug.units: {report}"));
        assert_eq!(units.len(), 1, "{package}: one presented unit: {report}");
        assert_eq!(units[0]["content_id"], expected_id, "{package}: {report}");
        let shape = units[0]["shape"]
            .as_object()
            .and_then(|shape| shape.keys().next().cloned());
        assert_eq!(
            shape.as_deref(),
            Some(expected_shape),
            "{package}: expected_shape: {report}"
        );

        // The server half, from the tick log: every tick holds exactly one unit, at its place.
        let ticks = read_tick_log(&server.tick_log);
        assert!(!ticks.is_empty(), "{package}: empty tick log");
        for (tick, records) in by_tick(&ticks) {
            assert_eq!(records.len(), 1, "{package}: tick {tick} holds one unit");
            assert!(
                near(records[0].pos, expected_pos),
                "{package}: tick {tick} has the unit at {:?}",
                records[0].pos
            );
        }
    }
}

/// T2d (server half): a `Hello` with a wrong lockfile hash gets `JoinRejected` and a disconnect.
#[test]
#[ignore]
fn t2d_bad_hello_hash_is_rejected() {
    let dir = test_dir("t2d_hello_hash");
    let server = spawn_server(&pack("base", &dir), &[]);
    let run = start_client(
        &server,
        &dir,
        "client",
        2,
        &["--hello-hash", "sha256:wrong"],
    )
    .finish();
    let report = &run.report;
    assert_eq!(report["welcomed"], false, "{report}");
    assert_eq!(
        report["rejected_reason"], "lockfile hash mismatch",
        "{report}"
    );
    assert_eq!(report["disconnected_after_connect"], true, "{report}");
}

/// T3 (process): two claims create no gameplay entity. The `--stats` file counts every entity
/// with a `UnitId` (`gameplay_entity_count`), so an entity without a `Position` counts too
/// (M1 verify). The tick log has one line for each `Query<(&UnitId, &Position)>` match.
#[test]
#[ignore]
fn t3_process_claims_create_nothing() {
    let dir = test_dir("t3_claims");
    let server = spawn_server(&pack("base", &dir), &[]);
    let before = stats_after(&server.stats, now_ms());
    assert_eq!(before["hellos"], 0, "no claim yet: {before}");
    assert_eq!(
        before["gameplay_entity_count"], BASE_PLACEMENTS,
        "gameplay entities before the claims: {before}"
    );
    let first = start_client(&server, &dir, "first", 3, &[]);
    std::thread::sleep(Duration::from_millis(300));
    let second = start_client(&server, &dir, "second", 3, &[]);
    let first = first.finish();
    let second = second.finish();
    assert_eq!(first.report["slot"], 0, "{}", first.report);
    assert_eq!(second.report["slot"], 1, "{}", second.report);
    // Both clients have exited, so this line was written after both Welcomes.
    let after_both = stats_after(&server.stats, now_ms());
    assert_eq!(after_both["hellos"], 2, "both claims: {after_both}");
    assert_eq!(
        after_both["gameplay_entity_count"], BASE_PLACEMENTS,
        "gameplay entities after both Welcomes: {after_both}"
    );

    let after = first
        .frame_log()
        .1
        .welcome_tick
        .max(second.frame_log().1.welcome_tick);
    let ticks = read_tick_log(&server.tick_log);
    let grouped = by_tick(&ticks);
    let after_claims = grouped.range(after + 1..).count();
    assert!(
        after_claims >= 20,
        "only {after_claims} ticks after both Welcomes (tick {after})"
    );
    for (tick, records) in grouped {
        let mut ids: Vec<u32> = records.iter().map(|record| record.unit_id).collect();
        ids.dedup();
        assert_eq!(
            ids.len(),
            1,
            "distinct unit_id count at tick {tick} (both Welcomes by {after}): {ids:?}"
        );
    }
}

/// The map of `base` with a second rifleman that slot 1 owns (M1 verify, owner slot).
const TWO_OWNERS_MAP: &str = r#"(
    id: "flat",
    size: (x: 64.0, y: 64.0),
    placements: [
        (unit: "base:rifleman", owner_slot: 0, pos: (x: 8.0, y: 8.0), contents: []),
        (unit: "base:rifleman", owner_slot: 1, pos: (x: 40.0, y: 40.0), contents: []),
    ],
    nodes: [],
    slot_economy: [],
)
"#;

/// The package of one T4 case.
enum T4Package {
    /// `mods/<name>` as it is.
    Mods(&'static str),
    /// `base` with `TWO_OWNERS_MAP`.
    TwoOwners,
}

/// One T4 case: `label` names the test directory, `reason` is the expected rejection, and
/// `homes` lists every unit of the map with the place it must never leave.
struct T4Case {
    label: &'static str,
    reason: &'static str,
    package: T4Package,
    needs_holder: bool,
    args: &'static [&'static str],
    homes: &'static [(u32, (f32, f32))],
}

/// T4: invalid `MoveTo` commands. Each gets `CommandAck`, then `CommandRejected` with its
/// reason, and no unit ever moves.
///
/// M1 verify added two cases. `outside_acme` sends a target with x in (48, 64) on the 48 by 48
/// map of `acme`, so a bound that ignores the map size goes red. `not_owner_by_map` uses a map
/// placement with `owner_slot: 1`: the sender alone takes slot 0 and orders unit 2, which slot
/// 1 owns, so a `load_map` that ignores `owner_slot` goes red.
#[test]
#[ignore]
fn t4_command_validation() {
    const BASE_HOME: &[(u32, (f32, f32))] = &[(UNIT, HOME)];
    let cases = [
        T4Case {
            label: "not_owner",
            reason: "not_owner",
            package: T4Package::Mods("base"),
            needs_holder: true,
            args: &["--move", "10,8", "--move-unit", "1"],
            homes: BASE_HOME,
        },
        T4Case {
            label: "target_not_finite",
            reason: "target_not_finite",
            package: T4Package::Mods("base"),
            needs_holder: false,
            args: &["--move", "NaN,8"],
            homes: BASE_HOME,
        },
        T4Case {
            label: "target_outside_map",
            reason: "target_outside_map",
            package: T4Package::Mods("base"),
            needs_holder: false,
            args: &["--move", "99,8"],
            homes: BASE_HOME,
        },
        T4Case {
            label: "outside_acme",
            reason: "target_outside_map",
            package: T4Package::Mods("acme"),
            needs_holder: false,
            args: &["--move", "56,4"],
            homes: &[(UNIT, (4.0, 4.0))],
        },
        T4Case {
            label: "not_owner_by_map",
            reason: "not_owner",
            package: T4Package::TwoOwners,
            needs_holder: false,
            args: &["--move", "10,8", "--move-unit", "2"],
            homes: &[(UNIT, HOME), (2, (40.0, 40.0))],
        },
    ];
    for T4Case {
        label,
        reason,
        package,
        needs_holder,
        args,
        homes,
    } in cases
    {
        let dir = test_dir(&format!("t4_{label}"));
        let tar = match package {
            T4Package::Mods(name) => pack(name, &dir),
            T4Package::TwoOwners => pack_with("base", "maps/flat.ron", TWO_OWNERS_MAP, &dir),
        };
        let server = spawn_server(&tar, &[]);
        // The holder takes slot 0, so the sender gets slot 1 and owns no unit.
        let holder = needs_holder.then(|| start_client(&server, &dir, "holder", 4, &[]));
        if holder.is_some() {
            std::thread::sleep(Duration::from_millis(500));
        }
        let sender = start_client(&server, &dir, "sender", 3, args).finish();
        let report = &sender.report;
        if let Some(holder) = holder {
            let holder = holder.finish();
            assert_eq!(holder.report["slot"], 0, "{label}: {}", holder.report);
            assert_eq!(report["slot"], 1, "{label}: {report}");
        } else {
            assert_eq!(report["slot"], 0, "{label}: {report}");
        }

        let results = report["command_results"]
            .as_array()
            .unwrap_or_else(|| panic!("{label}: no command_results: {report}"));
        assert_eq!(results.len(), 2, "{label}: {report}");
        assert_eq!(results[0]["seq"], 1, "{label}: {report}");
        assert!(
            results[0]["ack_tick"].is_u64(),
            "{label}: the first result is the ack: {report}"
        );
        assert_eq!(
            results[1],
            json!({ "seq": 1, "rejected": reason }),
            "{label}: {report}"
        );

        let ticks = read_tick_log(&server.tick_log);
        assert!(!ticks.is_empty(), "{label}: empty tick log");
        for (tick, records) in by_tick(&ticks) {
            assert_eq!(records.len(), homes.len(), "{label}: units at tick {tick}");
        }
        for record in &ticks {
            let home = homes
                .iter()
                .find(|(unit, _)| *unit == record.unit_id)
                .map(|(_, home)| *home)
                .unwrap_or_else(|| panic!("{label}: unexpected unit {}", record.unit_id));
            assert!(
                near(record.pos, home),
                "{label}: unit {} at {:?} at tick {}, home {home:?}",
                record.unit_id,
                record.pos,
                record.tick
            );
        }
    }
}

/// T5 convergence: every presented position lies on the server's segment, lag p99 is at most
/// `LAG_P99_LIMIT` ticks, and the client ends where the server ends.
#[test]
#[ignore]
fn t5_convergence() {
    let dir = test_dir("t5_convergence");
    let server = spawn_server(&pack("base", &dir), &[]);
    let run = start_client(&server, &dir, "client", 8, &["--move", "20,8"]).finish();
    let exit_ms = now_ms();
    let (frames, summary) = run.frame_log();
    let ticks = ticks_for(&server, &frames);

    let report =
        check_segments(&ticks, &frames, UNIT).unwrap_or_else(|error| panic!("T5 oracle: {error}"));
    record("t5_convergence", &report);
    // Bytes per tick for the record: the postcard payload of the `UnitDeltas`, not the
    // WebSocket frame size (Task 13 measures that).
    let stats = stats_after(&server.stats, exit_ms);
    let ticks_with_session =
        stats["ticks"].as_u64().expect("ticks") - u64::from(summary.welcome_tick);
    let payload = stats["delta_payload_bytes_total"].as_u64().expect("bytes");
    println!(
        "t5_convergence: delta payload {payload} bytes over {ticks_with_session} ticks = {:.2} bytes per tick",
        payload as f64 / ticks_with_session as f64
    );
    assert!(
        report.lag_p99 <= LAG_P99_LIMIT,
        "lag_p99 {:.3} > {LAG_P99_LIMIT} (lag_max {:.3})",
        report.lag_p99,
        report.lag_max
    );
    let own = frames_of(&frames, UNIT);
    assert!(own.len() >= 300, "only {} frames", own.len());
    let last = own.last().expect("frames").pos;
    let server_final = final_position(&ticks, UNIT);
    assert!(near(server_final, TARGET), "server final {server_final:?}");
    assert!(
        near(last, server_final),
        "client last {last:?}, server {server_final:?}"
    );
}

/// T6 second viewer: a client that sends nothing sees the move of the other client, and its
/// frames satisfy the T5 oracle against the same tick log. The mover's frames are checked the
/// same way: the order of the sessions on the server is not fixed, so a server that sends
/// deltas to one session only can starve either client. (A run with that defect starved the
/// mover after the viewer joined, and the viewer alone stayed green.)
#[test]
#[ignore]
fn t6_second_viewer() {
    let dir = test_dir("t6_second_viewer");
    let server = spawn_server(&pack("base", &dir), &[]);
    let mover = start_client(&server, &dir, "mover", 8, &["--move", "20,8"]);
    std::thread::sleep(Duration::from_millis(300));
    let viewer = start_client(&server, &dir, "viewer", 8, &[]);
    let mover = mover.finish();
    let viewer = viewer.finish();
    assert_eq!(
        viewer.report["command_results"],
        json!([]),
        "the viewer sends nothing"
    );

    for (name, run) in [("viewer", &viewer), ("mover", &mover)] {
        let (frames, _) = run.frame_log();
        let ticks = ticks_for(&server, &frames);
        let report = check_segments(&ticks, &frames, UNIT)
            .unwrap_or_else(|error| panic!("T6 oracle on the {name}: {error}"));
        record(&format!("t6_second_viewer {name}"), &report);
        let own = frames_of(&frames, UNIT);
        assert!(own.len() >= 300, "only {} {name} frames", own.len());
        let last = own.last().expect("frames").pos;
        assert!(near(last, TARGET), "the {name} ends at {last:?}");
    }
}

/// T6b late join: a client that joins after the unit arrived presents the final position
/// within 3 ticks of its `Welcome`.
#[test]
#[ignore]
fn t6b_late_join() {
    let dir = test_dir("t6b_late_join");
    let server = spawn_server(&pack("base", &dir), &[]);
    let mover = start_client(&server, &dir, "mover", 8, &["--move", "20,8"]);
    let arrival = wait_for_arrival(&server.tick_log, UNIT, TARGET, Duration::from_secs(20));
    let late = start_client(&server, &dir, "late", 2, &[]).finish();
    mover.finish();
    let (frames, summary) = late.frame_log();

    assert!(
        summary.welcome_tick > arrival,
        "the late client joined at {} before the arrival at {arrival}",
        summary.welcome_tick
    );
    assert!(
        summary.first_present_tick <= summary.welcome_tick + 3,
        "first_present_tick {} > welcome_tick {} + 3",
        summary.first_present_tick,
        summary.welcome_tick
    );
    let first = frames_of(&frames, UNIT)
        .first()
        .map(|frame| frame.pos)
        .expect("the late client presents the unit");
    assert!(near(first, TARGET), "the first record is at {first:?}");
}

/// One T8 run: the client drops the delta at Welcome + 10 and the delta that carries the
/// arrival. Each loss costs exactly one `ResyncRequest`, and the client ends at the target.
fn t8_case(name: &str, extra: &[&str]) {
    let dir = test_dir(&format!("t8_{name}"));
    let server = spawn_server(&pack("base", &dir), &[]);
    let mut args = vec![
        "--move",
        "20,8",
        "--drop-delta-ticks",
        "10",
        "--drop-arrival-delta",
    ];
    args.extend_from_slice(extra);
    let run = start_client(&server, &dir, name, 8, &args).finish();
    let exit_ms = now_ms();
    let (frames, summary) = run.frame_log();
    let ticks = ticks_for(&server, &frames);

    let arrival = arrival_tick(&ticks, UNIT, TARGET).expect("the unit arrives");
    let first_drop = summary.welcome_tick + 10;
    assert_eq!(
        summary.deltas_dropped,
        vec![first_drop, arrival],
        "{name}: the drops are Welcome + 10 and the arrival delta"
    );
    assert_ne!(
        pos_at(&ticks, UNIT, first_drop),
        pos_at(&ticks, UNIT, first_drop - 1),
        "{name}: the first drop is a delta during the move"
    );
    assert_eq!(
        summary.resync_requests_sent, 2,
        "{name}: resync_requests_sent"
    );
    let stats = stats_after(&server.stats, exit_ms);
    assert_eq!(stats["resync_requests"], 2, "{name}: stats {stats}");

    let report = check_segments(&ticks, &frames, UNIT)
        .unwrap_or_else(|error| panic!("{name}: oracle: {error}"));
    record(&format!("t8_{name}"), &report);
    let last = frames_of(&frames, UNIT).last().expect("frames").pos;
    assert!(near(last, TARGET), "{name}: the client ends at {last:?}");
}

/// T8 lost delta: case 1 drops two deltas; case 2 also holds each `Snapshot` that answers a
/// `ResyncRequest` back by 3 ticks.
#[test]
#[ignore]
fn t8_lost_delta() {
    t8_case("drops", &[]);
    t8_case("held_snapshots", &["--hold-resync-snapshots", "3"]);
}

// The oracle on made-up logs. A unit moves 0.15 per tick from x = 8 from tick 10 on; ticks
// are 50 ms apart.

const TICK_NS: u128 = 50_000_000;

fn made_up_ticks() -> Vec<TickRecord> {
    (0..40)
        .map(|tick| TickRecord {
            tick,
            wall_ns: u128::from(tick) * TICK_NS,
            unit_id: UNIT,
            pos: (8.0 + 0.15 * tick.saturating_sub(10) as f32, 8.0),
        })
        .collect()
}

/// A correct frame at presentation time `t` with `latest` as the latest applied tick: one tick
/// of delay and 5 ms of latency.
fn made_up_frame(ticks: &[TickRecord], t: f64, latest: u32) -> FrameRecord {
    let lo = t.floor() as u32;
    let fraction = (t - t.floor()) as f32;
    let (a, b) = (ticks[lo as usize].pos, ticks[lo as usize + 1].pos);
    FrameRecord {
        frame_ns: ((t + 1.0) * TICK_NS as f64) as u128 + 5_000_000,
        unit_id: UNIT,
        t,
        lo,
        hi: lo + 1,
        fraction,
        pos: (a.0 + (b.0 - a.0) * fraction, a.1 + (b.1 - a.1) * fraction),
        latest_tick: latest,
    }
}

fn made_up_frames(ticks: &[TickRecord]) -> Vec<FrameRecord> {
    (0..60)
        .map(|i| {
            let t = 12.0 + f64::from(i) / 3.0 + 0.01;
            made_up_frame(ticks, t, t.floor() as u32 + 1)
        })
        .collect()
}

#[test]
fn oracle_accepts_frames_on_the_segment() {
    let ticks = made_up_ticks();
    let report = check_segments(&ticks, &made_up_frames(&ticks), UNIT).expect("green");
    assert_eq!(report.frames, 60);
    assert!(
        report.lag_p99 > 1.0 && report.lag_p99 < 1.2,
        "{}",
        report.lag_p99
    );
}

#[test]
fn oracle_rejects_a_position_off_the_segment() {
    let ticks = made_up_ticks();
    let mut frames = made_up_frames(&ticks);
    frames[30].pos.0 += 0.01;
    let error = check_segments(&ticks, &frames, UNIT).expect_err("red");
    assert!(error.contains("off segment"), "{error}");
}

#[test]
fn oracle_rejects_a_stalled_bracket_below_the_latest_tick() {
    let ticks = made_up_ticks();
    let mut frames = made_up_frames(&ticks);
    let frame = &mut frames[30];
    frame.hi = frame.lo;
    frame.fraction = 0.0;
    frame.pos = ticks[frame.lo as usize].pos;
    let error = check_segments(&ticks, &frames, UNIT).expect_err("red");
    assert!(error.contains("bracket"), "{error}");
}

#[test]
fn oracle_rejects_a_bracket_above_the_latest_tick() {
    let ticks = made_up_ticks();
    let mut frames = made_up_frames(&ticks);
    frames[30].latest_tick = frames[30].lo;
    let error = check_segments(&ticks, &frames, UNIT).expect_err("red");
    assert!(error.contains("above latest_tick"), "{error}");
}

#[test]
fn oracle_rejects_a_latest_tick_that_the_server_has_not_logged() {
    let ticks = made_up_ticks();
    let mut frames = made_up_frames(&ticks);
    frames[30].latest_tick += 3;
    frames[30].hi = frames[30].lo + 1;
    let error = check_segments(&ticks, &frames, UNIT).expect_err("red");
    assert!(error.contains("ahead of the tick log"), "{error}");
}

#[test]
fn oracle_rejects_a_fraction_that_is_not_t_minus_lo() {
    let ticks = made_up_ticks();
    let mut frames = made_up_frames(&ticks);
    frames[30].fraction += 0.01;
    let error = check_segments(&ticks, &frames, UNIT).expect_err("red");
    assert!(error.contains("fraction"), "{error}");
}

#[test]
fn oracle_rejects_a_presentation_time_that_goes_back() {
    let ticks = made_up_ticks();
    let mut frames = made_up_frames(&ticks);
    let back = frames[29].t - 0.2;
    let latest = frames[30].latest_tick;
    let frame_ns = frames[30].frame_ns;
    frames[30] = made_up_frame(&ticks, back, latest);
    frames[30].frame_ns = frame_ns;
    let error = check_segments(&ticks, &frames, UNIT).expect_err("red");
    assert!(error.contains("decreased"), "{error}");
}

#[test]
fn oracle_rejects_an_empty_frame_log() {
    let ticks = made_up_ticks();
    let error = check_segments(&ticks, &[], UNIT).expect_err("red");
    assert!(error.contains("no frames"), "{error}");
}
