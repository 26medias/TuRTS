//! Server statistics.
//!
//! `Stats` holds the counters that the session and outbound systems write. With `--stats PATH`
//! the server also times every tick and keeps PATH up to date: one JSON object on one line,
//! written every second and when the app exits. The file is replaced as a whole (a temporary
//! file, then a rename), so a reader never sees half a line. A kill without warning leaves the
//! file as it was after the last write. The object also holds `gameplay_entity_count`, the
//! number of entities with a `UnitId` (T3 reads it, M1 verify).
//!
//! A tick is timed from `FixedFirst` to the end of `FixedLast`: the simulation, the change
//! collection and the sending of the deltas. The time between two ticks does not count. The
//! tick log is ordered after the timer, so `--tick-log` adds nothing to a tick time.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bevy::app::{AppExit, FixedFirst, FixedLast, Last};
use bevy::prelude::*;
use turts_sim::{Tick, UnitId};

use crate::config::Cli;
use crate::outbound::{send_command_results, send_deltas};
use crate::synthetic::retarget;

/// How often the file is rewritten.
const WRITE_EVERY: Duration = Duration::from_secs(1);

/// The percentiles cover this many of the newest ticks (about 55 minutes at 20 Hz), so a
/// server that runs for days does not grow without bound.
const SAMPLE_WINDOW: usize = 1 << 16;

/// Bytes in a page for `/proc/self/statm`. Linux uses 4096 on x86-64 and on most arm64 kernels.
const PAGE_BYTES: u64 = 4096;

/// Counters since the start of the process.
#[derive(Resource, Default, Debug)]
pub struct Stats {
    /// `Hello` messages that the server decided (accepted or refused).
    pub hellos: u64,
    /// `ResyncRequest` messages from sessions.
    pub resync_requests: u64,
    /// The postcard size of every `WorldDelta` sent, summed over the sessions.
    pub delta_payload_bytes_total: u64,
}

/// The system that ends the timing of a tick. A system that must run after the timing
/// (the tick log) goes `.after(TickEnd)`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TickEnd;

/// The tick times and the state of the output file.
#[derive(Resource)]
struct TickTimer {
    path: PathBuf,
    /// When the tick that runs now began.
    started: Instant,
    /// The times of the newest ticks, in microseconds, oldest first.
    samples: VecDeque<u32>,
    last_write: Instant,
    /// The time between two writes.
    every: Duration,
}

/// Add the statistics to the app. The counters always exist. The timing and the output exist
/// only with `--stats`.
pub fn install(app: &mut App, cli: &Cli) {
    app.init_resource::<Stats>();
    let Some(path) = &cli.stats else {
        return;
    };
    app.insert_resource(TickTimer {
        path: path.clone(),
        started: Instant::now(),
        samples: VecDeque::new(),
        last_write: Instant::now(),
        every: WRITE_EVERY,
    });
    app.add_systems(FixedFirst, tick_start.before(retarget));
    app.add_systems(
        FixedLast,
        tick_end
            .in_set(TickEnd)
            .after(send_command_results)
            .after(send_deltas),
    );
    app.add_systems(Last, write_when_due);
}

/// FixedFirst: note the time at which the tick began.
fn tick_start(mut timer: ResMut<TickTimer>) {
    timer.started = Instant::now();
}

/// FixedLast, after the sending systems: store the time of the tick.
fn tick_end(mut timer: ResMut<TickTimer>) {
    let micros = timer.started.elapsed().as_micros();
    if timer.samples.len() == SAMPLE_WINDOW {
        timer.samples.pop_front();
    }
    timer
        .samples
        .push_back(u32::try_from(micros).unwrap_or(u32::MAX));
}

/// Last: write the file when the interval is over, and in the frame in which the app exits.
/// `--run-ticks` writes `AppExit` in `FixedLast`, so this system sees it in the same frame.
fn write_when_due(
    mut timer: ResMut<TickTimer>,
    mut exit: MessageReader<AppExit>,
    stats: Res<Stats>,
    tick: Res<Tick>,
    units: Query<(), With<UnitId>>,
) {
    let exiting = exit.read().count() > 0;
    if !exiting && timer.last_write.elapsed() < timer.every {
        return;
    }
    timer.last_write = Instant::now();
    let text = format!("{}\n", render(&timer, &stats, tick.0, units.iter().count()));
    if let Err(error) = replace_file(&timer.path, &text) {
        eprintln!(
            "turts_server: cannot write --stats `{}`: {error}",
            timer.path.display()
        );
    }
}

/// The JSON object of one write. The percentiles cover the newest `SAMPLE_WINDOW` ticks;
/// `ticks` and the counters count from the start of the process. `gameplay_entity_count` is
/// the number of entities with a `UnitId` at the time of the write (design section 7, T3).
fn render(
    timer: &TickTimer,
    stats: &Stats,
    ticks: u32,
    gameplay_entities: usize,
) -> serde_json::Value {
    let mut sorted: Vec<u32> = timer.samples.iter().copied().collect();
    sorted.sort_unstable();
    let resident = resident_bytes();
    serde_json::json!({
        "unix_ms": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64),
        "ticks": ticks,
        "tick_p50_us": percentile(&sorted, 50),
        "tick_p99_us": percentile(&sorted, 99),
        "tick_max_us": sorted.last().copied().unwrap_or(0),
        "delta_payload_bytes_total": stats.delta_payload_bytes_total,
        "resync_requests": stats.resync_requests,
        "hellos": stats.hellos,
        "gameplay_entity_count": gameplay_entities,
        "resident_bytes": resident,
        "resident_peak_bytes": resident_peak_bytes().max(resident),
    })
}

/// The nearest-rank percentile of `sorted` (smallest first): the sample at rank
/// `ceil(len * per_cent / 100)`. An empty list gives 0.
fn percentile(sorted: &[u32], per_cent: usize) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (sorted.len() * per_cent).div_ceil(100);
    sorted[rank.max(1) - 1]
}

/// Put `text` in `path` in one step: write `<path>.tmp`, then rename it over `path`.
fn replace_file(path: &Path, text: &str) -> std::io::Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

/// Resident set size: the resident pages in `/proc/self/statm` times the page size. It is 0
/// where that file does not exist.
fn resident_bytes() -> u64 {
    fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|text| text.split_whitespace().nth(1)?.parse::<u64>().ok())
        .map_or(0, |pages| pages * PAGE_BYTES)
}

/// The largest resident set size so far: `VmHWM` in `/proc/self/status`, which the kernel keeps
/// for the whole life of the process. It is 0 where that file does not exist.
fn resident_peak_bytes() -> u64 {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|text| {
            let line = text.lines().find(|line| line.starts_with("VmHWM:"))?;
            line.split_whitespace().nth(1)?.parse::<u64>().ok()
        })
        .map_or(0, |kilobytes| kilobytes * 1024)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use turts_sim::headless_app;

    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("turts_stats_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cli(stats: Option<&Path>) -> Cli {
        Cli {
            config: PathBuf::from("server.ron"),
            tick_log: None,
            stats: stats.map(Path::to_path_buf),
            economy_log: None,
            synthetic_units: 0,
            run_ticks: None,
        }
    }

    /// A headless app with the statistics installed. `sleep_in_tick` makes every fixed tick
    /// take at least 2 ms, so the timer has something to measure.
    fn stats_app(path: Option<&Path>, sleep_in_tick: bool) -> App {
        let mut app = headless_app(20);
        install(&mut app, &cli(path));
        if sleep_in_tick {
            app.add_systems(FixedUpdate, || {
                std::thread::sleep(Duration::from_millis(2));
            });
        }
        app
    }

    fn read_json(path: &Path) -> serde_json::Value {
        let text = fs::read_to_string(path).unwrap();
        assert!(
            text.ends_with('\n'),
            "one line, ended by a newline: {text:?}"
        );
        assert_eq!(
            text.trim_end().lines().count(),
            1,
            "one JSON object: {text:?}"
        );
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn percentile_is_the_nearest_rank() {
        let hundred: Vec<u32> = (1..=100).collect();
        assert_eq!(percentile(&hundred, 50), 50);
        assert_eq!(percentile(&hundred, 99), 99);
        assert_eq!(percentile(&hundred, 100), 100);
        // Two samples: the 99th percentile is the larger one, the median the smaller one.
        assert_eq!(percentile(&[10, 20], 50), 10);
        assert_eq!(percentile(&[10, 20], 99), 20);
        assert_eq!(percentile(&[7], 99), 7);
        assert_eq!(percentile(&[], 99), 0);
    }

    #[test]
    fn the_file_holds_the_counters_and_one_timed_tick_per_tick() {
        let dir = test_dir("counters");
        let path = dir.join("stats.json");
        let mut app = stats_app(Some(&path), true);
        {
            let mut stats = app.world_mut().resource_mut::<Stats>();
            stats.hellos = 3;
            stats.resync_requests = 2;
            stats.delta_payload_bytes_total = 12_345;
        }
        // Two gameplay entities, and one entity without a `UnitId` that must not count.
        app.world_mut().spawn(UnitId(1));
        app.world_mut().spawn(UnitId(2));
        app.world_mut().spawn(Name::new("not a unit"));
        for _ in 0..5 {
            app.update();
            // Time between the ticks does not belong to a tick.
            std::thread::sleep(Duration::from_millis(40));
        }
        assert!(
            !path.exists(),
            "nothing is written before the first interval is over"
        );
        app.world_mut().write_message(AppExit::Success);
        app.update();

        let json = read_json(&path);
        let ticks = app.world().resource::<Tick>().0;
        assert_eq!(ticks, 6);
        assert_eq!(json["ticks"], ticks);
        assert_eq!(json["hellos"], 3);
        assert_eq!(json["resync_requests"], 2);
        assert_eq!(json["delta_payload_bytes_total"], 12_345);
        assert_eq!(json["gameplay_entity_count"], 2);
        let (p50, p99, max) = (
            json["tick_p50_us"].as_u64().unwrap(),
            json["tick_p99_us"].as_u64().unwrap(),
            json["tick_max_us"].as_u64().unwrap(),
        );
        assert!(
            2_000 <= p50 && p50 <= p99 && p99 <= max,
            "{p50} {p99} {max}"
        );
        assert!(
            max < 40_000,
            "a tick is not the time between ticks: {max} us"
        );
        let resident = json["resident_bytes"].as_u64().unwrap();
        let peak = json["resident_peak_bytes"].as_u64().unwrap();
        assert!(
            resident > 1_000_000,
            "a process holds more than 1 MB: {resident}"
        );
        assert!(peak >= resident, "{peak} {resident}");
        assert!(json["unix_ms"].as_u64().unwrap() > 1_700_000_000_000);
        let files: Vec<_> = fs::read_dir(&dir).unwrap().collect();
        assert_eq!(files.len(), 1, "the temporary file is gone: {files:?}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_file_is_rewritten_when_the_interval_is_over() {
        let dir = test_dir("interval");
        let path = dir.join("stats.json");
        let mut app = stats_app(Some(&path), false);
        app.world_mut().resource_mut::<TickTimer>().every = Duration::ZERO;
        app.update();
        assert_eq!(read_json(&path)["ticks"], 1);
        app.update();
        app.update();
        assert_eq!(
            read_json(&path)["ticks"],
            3,
            "the file holds the newest line"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn without_a_stats_path_only_the_counters_exist() {
        let mut app = stats_app(None, false);
        app.update();
        assert!(app.world().get_resource::<Stats>().is_some());
        assert!(app.world().get_resource::<TickTimer>().is_none());
    }
}
