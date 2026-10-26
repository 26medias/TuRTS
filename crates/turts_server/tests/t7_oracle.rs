//! T7 oracle (design section 7; plan Task 12): the frames that the browser presented lie on the
//! segments of the server tick log.
//!
//! `scripts/t7.sh` starts the server, drives headless Chrome with `tests/browser/t7.js`, and
//! then runs the `#[ignore]` test below with `T7_DIR` set to the folder of the run
//! (`target/t7`). The folder holds `server-tick.log`, `frames.jsonl` and `result.json`.
//!
//! `frames.jsonl` has one line per distinct debug string that `window.turts_debug()` returned:
//! `{"at_ms": <page clock, ms since the epoch>, "last_frame": {..}, "units": [{..}],
//! "scene": {"cameras_3d": 1, "units": [{..}]}}`. `scene` is what `window.turts_scene()`
//! returned in the same poll; t7.js checks it (one active 3D camera, one unit entity at the
//! presented position) and counts the passes in `result.json` (`scene_checked`). The
//! page clock reads `performance.timeOrigin + performance.now()` at the poll. The poll is
//! later than the frame, so the lag that the oracle works out is an upper bound. The lag is
//! recorded and not judged: the browser clock is not the server clock.
//!
//! The tests that read made-up lines start no process, so `cargo test --workspace` runs them.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use support::logs::{FrameRecord, read_tick_log_past};
use support::oracle::{check_segments, final_position};

/// Fewest frames that make a run meaningful. Software rendering gives about 6 frames a second.
const MIN_FRAMES: usize = 10;
/// Fewest frames between the start and the end of the move. Frames at rest satisfy the
/// oracle without a move, so a run without moving frames proves nothing.
const MIN_MOVING_FRAMES: usize = 3;
/// How long the oracle waits for the server to log a tick after the last frame.
const TICK_WAIT: Duration = Duration::from_secs(10);
/// The tolerance of a position compare.
const POS_TOLERANCE: f32 = 1e-3;

/// The folder of the run: `T7_DIR`, or `target/t7`.
fn t7_dir() -> PathBuf {
    std::env::var_os("T7_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/t7"),
        PathBuf::from,
    )
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() <= POS_TOLERANCE && (a.1 - b.1).abs() <= POS_TOLERANCE
}

fn number(value: &Value, name: &str, line: &str) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("frames.jsonl: no number `{name}` in `{line}`"))
}

fn tick(value: &Value, name: &str, line: &str) -> u32 {
    value
        .as_u64()
        .and_then(|tick| u32::try_from(tick).ok())
        .unwrap_or_else(|| panic!("frames.jsonl: no tick `{name}` in `{line}`"))
}

/// Convert one line of `frames.jsonl` to a [`FrameRecord`]. Panics when the line does not
/// parse, holds a number of units other than one, or lacks a field: a test that reads a
/// broken log must fail, not see fewer frames.
fn frame_from_line(line: &str) -> FrameRecord {
    let poll: Value = serde_json::from_str(line)
        .unwrap_or_else(|error| panic!("frames.jsonl: `{line}` is not JSON: {error}"));
    let units = poll["units"]
        .as_array()
        .unwrap_or_else(|| panic!("frames.jsonl: no `units` array in `{line}`"));
    assert_eq!(units.len(), 1, "frames.jsonl: expected 1 unit in `{line}`");
    let (unit, frame) = (&units[0], &poll["last_frame"]);
    let at_ms = number(&poll["at_ms"], "at_ms", line);
    FrameRecord {
        // Microseconds first: `at_ms` is about 1.8e12, and `at_ms * 1e6` would round at 256 ns.
        frame_ns: (at_ms * 1_000.0).round() as u128 * 1_000,
        unit_id: tick(&unit["unit_id"], "unit_id", line),
        t: number(&frame["t"], "t", line),
        lo: tick(&frame["lo_tick"], "lo_tick", line),
        hi: tick(&frame["hi_tick"], "hi_tick", line),
        fraction: number(&frame["fraction"], "fraction", line) as f32,
        pos: (
            number(&unit["pos"][0], "pos x", line) as f32,
            number(&unit["pos"][1], "pos y", line) as f32,
        ),
        latest_tick: tick(&frame["latest_tick"], "latest_tick", line),
    }
}

fn read_browser_frames(path: &Path) -> Vec<FrameRecord> {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .lines()
        .filter(|line| !line.is_empty())
        .map(frame_from_line)
        .collect()
}

/// T7: the segment oracle of T5 holds for the positions that the browser presented, and the
/// browser ends where the server ends.
#[test]
#[ignore]
fn t7_browser_frames_satisfy_the_segment_oracle() {
    let dir = t7_dir();
    let result: Value = serde_json::from_str(
        &fs::read_to_string(dir.join("result.json")).expect("t7.js wrote result.json"),
    )
    .expect("result.json is JSON");
    assert_eq!(
        result["welcomed"], true,
        "the browser did not join: {result}"
    );

    let frames = read_browser_frames(&dir.join("frames.jsonl"));
    assert!(
        frames.len() >= MIN_FRAMES,
        "only {} frames in frames.jsonl",
        frames.len()
    );
    // M1 verify: t7.js checked the rendered scene (`window.turts_scene`) on every recorded
    // poll. A run that skipped that check proves only the debug record, not the scene.
    assert_eq!(
        result["scene_checked"].as_u64(),
        Some(frames.len() as u64),
        "t7.js checked the scene on fewer polls than it recorded: {result}"
    );
    let unit = frames[0].unit_id;
    assert!(
        frames.iter().all(|frame| frame.unit_id == unit),
        "the browser presented more than one unit"
    );
    let (first, last) = (frames[0].pos, frames[frames.len() - 1].pos);
    let moving = frames
        .iter()
        .filter(|frame| !near(frame.pos, first) && !near(frame.pos, last))
        .count();
    assert!(
        moving >= MIN_MOVING_FRAMES,
        "only {moving} frames between {first:?} and {last:?}: the unit did not move in view"
    );

    let last_frame_ns = frames[frames.len() - 1].frame_ns;
    let ticks = read_tick_log_past(&dir.join("server-tick.log"), last_frame_ns, TICK_WAIT);
    let report = check_segments(&ticks, &frames, unit)
        .unwrap_or_else(|error| panic!("T7 oracle on the browser frames: {error}"));
    println!(
        "t7_oracle: {} frames ({moving} moving), lag_p99 {:.3} ticks, lag_max {:.3} ticks \
         (recorded, not judged: the poll is later than the frame)",
        report.frames, report.lag_p99, report.lag_max
    );

    let server_final = final_position(&ticks, unit);
    assert!(
        near(last, server_final),
        "the browser ends at {last:?}, the server at {server_final:?}"
    );
    let reported = (
        number(&result["final_pos"][0], "final_pos x", "result.json") as f32,
        number(&result["final_pos"][1], "final_pos y", "result.json") as f32,
    );
    assert!(
        near(reported, server_final),
        "result.json final_pos {reported:?}, the server at {server_final:?}"
    );
}

#[test]
fn a_poll_line_converts_to_a_frame_record() {
    let line = r#"{"at_ms":1793005624879.0999,"last_frame":{"fraction":0.25,"hi_tick":19,"latest_tick":19,"lo_tick":18,"t":18.25},"units":[{"content_id":"base:rifleman","pos":[8,8.5],"shape":null,"unit_id":1}]}"#;
    assert_eq!(
        frame_from_line(line),
        FrameRecord {
            frame_ns: 1_793_005_624_879_100_000,
            unit_id: 1,
            t: 18.25,
            lo: 18,
            hi: 19,
            fraction: 0.25,
            pos: (8.0, 8.5),
            latest_tick: 19,
        }
    );
}

#[test]
#[should_panic(expected = "expected 1 unit")]
fn a_poll_line_with_two_units_is_refused() {
    frame_from_line(
        r#"{"at_ms":1.0,"last_frame":{"fraction":0,"hi_tick":1,"latest_tick":1,"lo_tick":1,"t":1},"units":[{"pos":[0,0],"unit_id":1},{"pos":[1,1],"unit_id":2}]}"#,
    );
}

#[test]
#[should_panic(expected = "no tick `lo_tick`")]
fn a_poll_line_without_a_bracket_is_refused() {
    frame_from_line(
        r#"{"at_ms":1.0,"last_frame":{"fraction":0,"hi_tick":1,"latest_tick":1,"t":1},"units":[{"pos":[0,0],"unit_id":1}]}"#,
    );
}
