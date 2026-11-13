//! The client path with no focus (pacing review M3), driven natively against a real server.
//!
//! A window with no focus updates once per `1 / background_frame_limit` s (`pacing.rs`). The
//! lowest value a player can store is 10, so one update each 100 ms. The windowed winit loop
//! cannot run inside `cargo test`: winit needs the main thread and a display, the test harness
//! runs each test on a worker thread, and a headed run under Xvfb emulates the focus for every
//! window (Playwright), so it never takes the path with no focus. The nearest honest test is
//! this one: it builds the real client app (`build_app` with `--headless`: the same client,
//! network and replica plugins, no window and no renderer) and calls `app.update()` itself once
//! each 100 ms, the period of the update mode with no focus. The server is the real
//! `turts_server` process. Over 20 s after the join, the test asserts:
//! - no disconnect, and the phase stays `Joined`;
//! - no resync request and no buffer overflow (no resync storm);
//! - right after each update, the replica tick is at most `MAX_TICK_GAP` ticks behind the
//!   server tick at that wall time (from the server tick log);
//! - the replica tick advances at least `MIN_TICKS_PER_SEC` ticks per second (server: 20).
//!
//! It needs the release `turts_server` next to the `turts_client` binary, so it carries
//! `#[ignore]`: `cargo build --release -p turts_server` then
//! `cargo test --release -p turts_client --test background_pace -- --ignored`.
//! `TURTS_PACE_PERIOD_MS` changes the period for an experiment (default 100).

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bevy::app::PluginsState;
use turts_client::build_app;
use turts_client::ingest::{IngestCounters, ReplicaState};
use turts_client::join::{JoinState, Phase};
use turts_client::options::parse_cli;

/// The update period of a window with no focus at the lowest stored limit (10).
const DEFAULT_PERIOD_MS: u64 = 100;
/// How long the test measures after the join.
const MEASURE: Duration = Duration::from_secs(20);
/// The most ticks that the replica may be behind the server right after an update.
const MAX_TICK_GAP: i64 = 4;
/// The least replica ticks per second over the measured time (the server runs at 20 Hz).
const MIN_TICKS_PER_SEC: f64 = 19.0;

struct Server {
    child: Child,
    tick_log: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_ports() -> (u16, u16) {
    let a = TcpListener::bind("127.0.0.1:0").unwrap();
    let b = TcpListener::bind("127.0.0.1:0").unwrap();
    (
        a.local_addr().unwrap().port(),
        b.local_addr().unwrap().port(),
    )
}

fn now_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn start_server(dir: &Path) -> (Server, String, String) {
    let server_exe = Path::new(env!("CARGO_BIN_EXE_turts_client"))
        .with_file_name(format!("turts_server{}", std::env::consts::EXE_SUFFIX));
    assert!(
        server_exe.exists(),
        "build the server first (cargo build --release -p turts_server): {} is absent",
        server_exe.display()
    );
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let tar = dir.join("base.tar");
    fs::write(
        &tar,
        turts_mod_runtime::pack::pack_dir(&mods).expect("base packs"),
    )
    .unwrap();
    let (http_port, ws_port) = free_ports();
    let http = format!("http://127.0.0.1:{http_port}");
    let ws = format!("ws://127.0.0.1:{ws_port}");
    let config = dir.join("server.ron");
    fs::write(
        &config,
        format!(
            "(\n    game_package: {:?},\n    http_listen: \"127.0.0.1:{http_port}\",\n    \
             ws_listen: \"127.0.0.1:{ws_port}\",\n    public_url: \"{http}\",\n    tick_hz: 20,\n)\n",
            tar.display().to_string()
        ),
    )
    .unwrap();
    let tick_log = dir.join("ticks.log");
    let log = fs::File::create(dir.join("server.log")).unwrap();
    let child = Command::new(&server_exe)
        .arg(&config)
        .arg("--tick-log")
        .arg(&tick_log)
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("turts_server starts");
    let server = Server { child, tick_log };
    let deadline = Instant::now() + Duration::from_secs(10);
    while ureq::get(&format!("{http}/advertisement.json"))
        .call()
        .is_err()
    {
        assert!(
            Instant::now() < deadline,
            "the server did not answer in 10 s"
        );
        sleep(Duration::from_millis(50));
    }
    (server, http, ws)
}

/// `(wall_ns, tick)` of each server tick, in file order.
fn server_ticks(path: &Path) -> Vec<(u128, i64)> {
    let mut out: Vec<(u128, i64)> = Vec::new();
    for line in fs::read_to_string(path).unwrap().lines() {
        let mut fields = line.split(',');
        let tick: i64 = fields.next().unwrap().parse().unwrap();
        let wall: u128 = fields.next().unwrap().parse().unwrap();
        if out.last().is_none_or(|(_, last)| *last != tick) {
            out.push((wall, tick));
        }
    }
    out
}

struct Sample {
    wall_ns: u128,
    tick: Option<u32>,
}

#[test]
#[ignore]
fn a_client_updated_each_100_ms_keeps_pace_with_the_server() {
    let period = Duration::from_millis(
        std::env::var("TURTS_PACE_PERIOD_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_PERIOD_MS),
    );
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("background_pace");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let (server, http, ws) = start_server(&dir);

    let args: Vec<String> = ["--headless", "--server", &http, "--ws", &ws]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    let mut app = build_app(parse_cli(&args).expect("client options parse"));
    while app.plugins_state() == PluginsState::Adding {
        sleep(Duration::from_millis(1));
    }
    app.finish();
    app.cleanup();

    let mut next = Instant::now();
    let mut joined_at: Option<Instant> = None;
    let mut samples = Vec::new();
    let join_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        app.update();
        let world = app.world();
        let joined = matches!(world.resource::<Phase>(), Phase::Joined { .. });
        if let Phase::Failed(reason) = world.resource::<Phase>() {
            panic!("the client failed: {reason}");
        }
        match joined_at {
            None if joined => joined_at = Some(Instant::now()),
            None => assert!(Instant::now() < join_deadline, "no join in 15 s"),
            Some(at) => {
                assert!(joined, "the client left Joined after the join");
                samples.push(Sample {
                    wall_ns: now_ns(),
                    tick: world
                        .resource::<ReplicaState>()
                        .replica
                        .as_ref()
                        .and_then(|replica| replica.last_applied_tick),
                });
                if at.elapsed() >= MEASURE {
                    break;
                }
            }
        }
        next += period;
        let now = Instant::now();
        if next > now {
            sleep(next - now);
        }
    }

    let world = app.world();
    let join = world.resource::<JoinState>();
    let counters = world.resource::<IngestCounters>();
    let ticks = server_ticks(&server.tick_log);
    drop(server);
    assert!(join.connected_seen, "the client never connected");
    assert!(!join.disconnected_after_connect, "the client disconnected");
    assert_eq!(counters.resync_requests_sent, 0, "resync requests");
    assert_eq!(counters.buffer_overflows, 0, "buffer overflows");

    let mut gaps = Vec::new();
    for sample in &samples {
        let tick = i64::from(sample.tick.expect("a joined replica has a tick"));
        let server_tick = ticks
            .iter()
            .take_while(|(wall, _)| *wall <= sample.wall_ns)
            .last()
            .map(|(_, tick)| *tick)
            .expect("the server ticked before the sample");
        gaps.push(server_tick - tick);
    }
    gaps.sort_unstable();
    let max_gap = *gaps.last().unwrap();
    let p95_gap = gaps[gaps.len() * 95 / 100];
    let first = samples.first().unwrap();
    let last = samples.last().unwrap();
    let secs = (last.wall_ns - first.wall_ns) as f64 / 1e9;
    let rate = f64::from(last.tick.unwrap() - first.tick.unwrap()) / secs;
    println!(
        "background_pace: period {period:?}, {} samples over {secs:.1} s, tick gap p95 {p95_gap} max {max_gap}, {rate:.2} ticks per second, deltas {}",
        samples.len(),
        counters.deltas_received
    );
    assert!(
        max_gap <= MAX_TICK_GAP,
        "the replica fell behind: tick gap max {max_gap} > {MAX_TICK_GAP} (p95 {p95_gap})"
    );
    assert!(
        rate >= MIN_TICKS_PER_SEC,
        "the replica advanced {rate:.2} ticks per second < {MIN_TICKS_PER_SEC}"
    );
}
