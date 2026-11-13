//! Process acceptance: a server stall of 0.6 s makes no resync (the reorder window of the replica).
//!
//! When the server loop is held for 0.6 s, it runs up to 5 fixed ticks in the next frame
//! (`Time<Virtual>` clamps one frame to 250 ms). The 5 deltas wait in one lightyear flush, and
//! lightyear sends the fragments first and the singles by ascending byte size
//! (`crates/turts_net_lightyear/tests/spike.rs`, P8). When the deltas shrink from tick to tick,
//! the later (smaller) delta arrives first, and a strict `base_tick` chain asks for a resync
//! though no delta is lost. The scene here makes the deltas shrink: 16 troopers walk
//! different distances, so they stop one after the other, and the next round sends them back.
//!
//! The stall is real: the test stops the server process with SIGSTOP and continues it with SIGCONT
//! (the smallest honest instrument; no test hook in the server holds its loop). Two native
//! headless clients (slot 0 and slot 1) join. After 6 stalls the test stops the server once more
//! and asserts, for each client: 0 resync requests (and 0 for each reason), and the replica
//! tick equal to the last server tick of the tick log.
//!
//! Red before the repair (the reorder window), green after. `ordered` runs need the release
//! `turts_client` next to the server (`scripts/acceptance.sh`).

mod support;

use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use serde_json::Value;
use support::mc;

const STALLS: u32 = 6;
const STALL: Duration = Duration::from_millis(600);
/// The ticks of play between two stalls (3 s at 20 Hz).
const PLAY_TICKS: u64 = 60;
const ROUND_TICKS: u32 = 30;
const ROUNDS: u32 = 40;
const WALKERS: u32 = 16;
const DEADLINE: Duration = Duration::from_secs(120);

struct ClientProcess(Child);
impl Drop for ClientProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn signal(pid: u32, name: &str) {
    let status = Command::new("kill")
        .args([&format!("-{name}"), &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -{name} {pid} failed");
}

fn report(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn applied_tick(path: &Path) -> Option<u64> {
    report(path)?["replicated_world"]["tick"].as_u64()
}

fn spawn_client(
    server: &support::ServerProc,
    dir: &Path,
    id: u32,
) -> (ClientProcess, std::path::PathBuf) {
    let report_path = dir.join(format!("client-{id}.json"));
    let stdout = fs::File::create(dir.join(format!("client-{id}.stdout.log"))).unwrap();
    let stderr = fs::File::create(dir.join(format!("client-{id}.stderr.log"))).unwrap();
    let child = Command::new(support::client_exe())
        .args([
            "--headless",
            "--server",
            &server.http_url,
            "--ws",
            &format!("ws://{}", server.ws_addr),
            "--player-id",
            &id.to_string(),
            "--run-secs",
            "110",
            "--live-report",
            report_path.to_str().unwrap(),
        ])
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .expect("headless client starts");
    (ClientProcess(child), report_path)
}

/// `(tick, wall_ns)` of each tick of the tick log (the first line of the tick).
fn tick_walls(path: &Path) -> Vec<(u64, u128)> {
    let text = fs::read_to_string(path).expect("tick log reads");
    let mut out: Vec<(u64, u128)> = Vec::new();
    for line in text.lines() {
        let mut fields = line.split(',');
        let (Some(tick), Some(wall)) = (fields.next(), fields.next()) else {
            continue;
        };
        let (Ok(tick), Ok(wall)) = (tick.parse::<u64>(), wall.parse::<u128>()) else {
            continue;
        };
        if out.last().is_none_or(|last| last.0 != tick) {
            out.push((tick, wall));
        }
    }
    out
}

fn last_tick(path: &Path) -> u64 {
    let mut file = fs::File::open(path).expect("tick log opens");
    let len = file.metadata().unwrap().len();
    let from = len.saturating_sub(256 * 1024);
    file.seek(SeekFrom::Start(from)).unwrap();
    let mut tail = String::new();
    file.read_to_string(&mut tail).expect("tick log tail reads");
    tail.lines()
        .rev()
        .find_map(|line| line.split(',').next()?.parse::<u64>().ok())
        .expect("a tick line")
}

#[test]
#[ignore = "real Muster server stalled with SIGSTOP and two native headless clients; run through scripts/acceptance.sh"]
fn a_server_stall_of_0_6_s_makes_no_resync() {
    let dir = support::test_dir("mc-reorder-stall");
    let package = mc::muster_package(&dir);
    let script = dir.join("commands.json");
    // Walker k (1 to 16) goes `k` tiles up in even rounds and back in odd rounds.
    let mut entries = Vec::new();
    for round in 0..ROUNDS {
        let tick = 60 + round * ROUND_TICKS;
        for k in 1..=WALKERS {
            let y = if round % 2 == 0 {
                18.0 - k as f32
            } else {
                18.0
            };
            let x = 25.0 + 1.1 * ((k - 1) % 8) as f32;
            entries.push((tick, 0u8, mc::point_order(vec![k], x, y.max(1.0), 0)));
        }
    }
    mc::write_script(&script, &entries);
    let server = support::spawn_server(&package, &["--command-script", script.to_str().unwrap()]);
    let pid = server.child.id();
    let (_a, path_a) = spawn_client(&server, &dir, 62101);
    let (_b, path_b) = spawn_client(&server, &dir, 62102);

    let deadline = Instant::now() + DEADLINE;
    let wait_ticks = |from: u64, count: u64| {
        let target = from + count;
        loop {
            assert!(
                Instant::now() < deadline,
                "setup/data: the clients did not reach tick {target} before the deadline; server log:\n{}",
                server.log()
            );
            std::thread::sleep(Duration::from_millis(20));
            if let (Some(a), Some(b)) = (applied_tick(&path_a), applied_tick(&path_b))
                && a.min(b) >= target
            {
                return a.min(b);
            }
        }
    };
    let mut tick = wait_ticks(0, 40);
    assert_eq!(
        report(&path_a).unwrap()["slot"].as_u64().map(|s| s <= 1),
        Some(true)
    );
    for _ in 0..STALLS {
        tick = wait_ticks(tick, PLAY_TICKS);
        signal(pid, "STOP");
        std::thread::sleep(STALL);
        signal(pid, "CONT");
    }
    wait_ticks(tick, PLAY_TICKS);

    // Stop the server a last time: every delta in flight reaches the clients, and the replica
    // tick must equal the last server tick.
    signal(pid, "STOP");
    std::thread::sleep(Duration::from_millis(1500));
    let server_tick = last_tick(&server.tick_log);
    let walls = tick_walls(&server.tick_log);
    let stalls = walls
        .windows(2)
        .filter(|pair| pair[1].1 - pair[0].1 >= 500_000_000)
        .count();
    let burst = walls
        .windows(2)
        .filter(|pair| pair[1].1 - pair[0].1 < 10_000_000)
        .count();
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    for (name, path) in [("A", &path_a), ("B", &path_b)] {
        let r = report(path).expect("client report");
        let get = |key: &str| r[key].as_u64().unwrap_or(u64::MAX);
        let replica_tick = r["replicated_world"]["tick"].as_u64().unwrap_or(0);
        lines.push(format!(
            "client {name}: resyncs {} (chain break {}, validation {}, idle span {}, successor {}), replica tick {replica_tick}, server tick {server_tick}",
            get("resync_requests_sent"),
            get("resync_by_chain_break"),
            get("resync_by_validation"),
            get("resync_by_idle_span"),
            get("resync_by_successor"),
        ));
        if get("resync_requests_sent") != 0 {
            failures.push(format!(
                "client {name} asked for {} resync(s)",
                get("resync_requests_sent")
            ));
        }
        if replica_tick != server_tick {
            failures.push(format!(
                "client {name} replica tick {replica_tick} != server tick {server_tick}"
            ));
        }
    }
    println!(
        "MC reorder stall: stalls seen in the tick log {stalls}, ticks run in a burst (gap under 10 ms) {burst}, {}; evidence {}",
        lines.join("; "),
        dir.display()
    );
    assert!(
        stalls >= STALLS as usize - 1,
        "setup/data: the tick log shows {stalls} stalls of 0.5 s or more, expected about {STALLS}"
    );
    assert!(
        burst >= 3 * (STALLS as usize - 1),
        "setup/data: the tick log shows {burst} burst ticks after the stalls"
    );
    assert!(
        failures.is_empty(),
        "{}; {}",
        failures.join("; "),
        lines.join("; ")
    );
}
