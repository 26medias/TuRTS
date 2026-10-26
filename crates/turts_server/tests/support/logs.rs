//! Readers for the logs of a process-level run.
//!
//! - The server tick log (`--tick-log`): `tick,wall_ns,unit_id,x,y`, one line per unit per tick.
//! - The client frame log (`--frame-log`): `frame_ns,unit_id,t,lo,hi,fraction,x,y,latest_tick`,
//!   one line per presented unit per frame, then one summary line that starts with `#`.
//! - The server stats file (`--stats`): one JSON object on one line.
//!
//! A reader panics with the path and the line when a line does not parse: a test that reads a
//! broken log must fail, not see fewer records.

use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};

/// One line of the server tick log.
#[derive(Debug, Clone, PartialEq)]
pub struct TickRecord {
    pub tick: u32,
    /// Wall clock in nanoseconds since the UNIX epoch, at the end of the tick.
    pub wall_ns: u128,
    pub unit_id: u32,
    pub pos: (f32, f32),
}

/// One line of the client frame log.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameRecord {
    /// Wall clock in nanoseconds since the UNIX epoch: the clock of the tick log.
    pub frame_ns: u128,
    pub unit_id: u32,
    /// Presentation time in ticks.
    pub t: f64,
    pub lo: u32,
    pub hi: u32,
    pub fraction: f32,
    pub pos: (f32, f32),
    pub latest_tick: u32,
}

/// The summary line at the end of the frame log.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSummary {
    pub welcome_tick: u32,
    /// `lo` of the first record.
    pub first_present_tick: u32,
    pub resync_requests_sent: u32,
    /// Absolute ticks of the deltas that the client ignored, in the order it ignored them.
    pub deltas_dropped: Vec<u32>,
}

fn field<T: FromStr>(path: &Path, line: &str, text: Option<&str>, name: &str) -> T {
    text.and_then(|text| text.trim().parse().ok())
        .unwrap_or_else(|| panic!("{}: bad `{name}` in line `{line}`", path.display()))
}

/// The complete lines of a file. A last line without a newline is still being written by the
/// process, so it is left out.
fn complete_lines(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let complete = match text.rfind('\n') {
        Some(end) => &text[..=end],
        None => "",
    };
    complete
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Read the server tick log. Panics when the file is absent or a line does not parse.
pub fn read_tick_log(p: &Path) -> Vec<TickRecord> {
    complete_lines(p)
        .iter()
        .map(|line| {
            let mut parts = line.split(',');
            let record = TickRecord {
                tick: field(p, line, parts.next(), "tick"),
                wall_ns: field(p, line, parts.next(), "wall_ns"),
                unit_id: field(p, line, parts.next(), "unit_id"),
                pos: (
                    field(p, line, parts.next(), "x"),
                    field(p, line, parts.next(), "y"),
                ),
            };
            assert!(
                parts.next().is_none(),
                "{}: extra field in `{line}`",
                p.display()
            );
            record
        })
        .collect()
}

/// Read the client frame log and its summary line. Panics when the file is absent, a line does
/// not parse, the summary line is missing, or a summary value is `none`.
pub fn read_frame_log(p: &Path) -> (Vec<FrameRecord>, FrameSummary) {
    let mut frames = Vec::new();
    let mut summary = None;
    for line in complete_lines(p) {
        if let Some(rest) = line.strip_prefix('#') {
            summary = Some(parse_summary(p, &line, rest));
            continue;
        }
        let mut parts = line.split(',');
        let record = FrameRecord {
            frame_ns: field(p, &line, parts.next(), "frame_ns"),
            unit_id: field(p, &line, parts.next(), "unit_id"),
            t: field(p, &line, parts.next(), "t"),
            lo: field(p, &line, parts.next(), "lo_tick"),
            hi: field(p, &line, parts.next(), "hi_tick"),
            fraction: field(p, &line, parts.next(), "fraction"),
            pos: (
                field(p, &line, parts.next(), "x"),
                field(p, &line, parts.next(), "y"),
            ),
            latest_tick: field(p, &line, parts.next(), "latest_tick"),
        };
        assert!(
            parts.next().is_none(),
            "{}: extra field in `{line}`",
            p.display()
        );
        frames.push(record);
    }
    let summary = summary.unwrap_or_else(|| panic!("{}: no summary line (`# ...`)", p.display()));
    (frames, summary)
}

/// `welcome_tick=<w> first_present_tick=<f> resync_requests_sent=<n> deltas_dropped=<t1;t2>`.
fn parse_summary(p: &Path, line: &str, rest: &str) -> FrameSummary {
    let value = |key: &str| -> &str {
        rest.split_whitespace()
            .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))
            .unwrap_or_else(|| panic!("{}: no `{key}` in `{line}`", p.display()))
    };
    let tick = |key: &str| -> u32 {
        let text = value(key);
        assert_ne!(text, "none", "{}: `{key}` is none in `{line}`", p.display());
        field(p, line, Some(text), key)
    };
    let dropped = value("deltas_dropped");
    FrameSummary {
        welcome_tick: tick("welcome_tick"),
        first_present_tick: tick("first_present_tick"),
        resync_requests_sent: field(
            p,
            line,
            Some(value("resync_requests_sent")),
            "resync_requests_sent",
        ),
        deltas_dropped: dropped
            .split(';')
            .filter(|part| !part.is_empty())
            .map(|part| field(p, line, Some(part), "deltas_dropped"))
            .collect(),
    }
}

/// The first tick at which `unit_id` is exactly at `target`, or `None`.
///
/// The compare is exact on purpose. The server lands a unit exactly on its target when the
/// target is within one step, and the log writes each `f32` so that it reads back the same. A
/// tolerance would pick the tick before the stop: in a `base` run, tick 89 logs `19.99997` and
/// the stop is at tick 90 (`20`). The client's `--drop-arrival-delta` hook also compares
/// exactly.
pub fn arrival_tick(ticks: &[TickRecord], unit_id: u32, target: (f32, f32)) -> Option<u32> {
    ticks
        .iter()
        .find(|record| record.unit_id == unit_id && record.pos == target)
        .map(|record| record.tick)
}

/// Poll the growing tick log every 20 ms until it holds a tick with `wall_ns` after `at_ns`,
/// then return all its records. The oracle needs a tick on each side of every frame. Panics
/// after `timeout`.
pub fn read_tick_log_past(p: &Path, at_ns: u128, timeout: Duration) -> Vec<TickRecord> {
    let started = Instant::now();
    loop {
        let ticks = read_tick_log(p);
        if ticks.iter().any(|record| record.wall_ns > at_ns) {
            return ticks;
        }
        assert!(
            started.elapsed() < timeout,
            "{} has no tick after {at_ns} ns in {timeout:?}",
            p.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Poll the growing tick log every 50 ms until `unit_id` is at `target`, and return that tick.
/// Panics after `timeout`.
pub fn wait_for_arrival(p: &Path, unit_id: u32, target: (f32, f32), timeout: Duration) -> u32 {
    let started = Instant::now();
    loop {
        if p.exists()
            && let Some(tick) = arrival_tick(&read_tick_log(p), unit_id, target)
        {
            return tick;
        }
        assert!(
            started.elapsed() < timeout,
            "unit {unit_id} did not reach {target:?} in {timeout:?} ({})",
            p.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Read the stats file. The server replaces it as a whole, so a read never sees half a line.
/// Panics when the file is absent or not JSON.
pub fn read_stats(p: &Path) -> serde_json::Value {
    let text = fs::read_to_string(p)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", p.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not JSON: {error}: {text}", p.display()))
}
