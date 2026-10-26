//! Networking spike P1 to P7 (Task 6a, design section 8).
//!
//! Every test is `#[ignore]`. Run them in release, one at a time:
//! `cargo test --release -p turts_net_lightyear --features spike --test spike -- --ignored --test-threads=1 --nocapture`
//!
//! The tests start `spike_server`, `spike_client` and `tests/net/relay.py` as processes and
//! read the JSON files that the processes write. Results go to `target/spike/`.
#![cfg(feature = "spike")]

use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const SERVER_BIN: &str = env!("CARGO_BIN_EXE_spike_server");
const CLIENT_BIN: &str = env!("CARGO_BIN_EXE_spike_client");

/// P6 and P7 bound: the tick p99 may rise by 10 % of the baseline or by 1 ms, whichever is larger.
const P99_BOUND_FRACTION: f64 = 0.10;
const P99_BOUND_FLOOR_US: f64 = 1000.0;
/// P7: the server's Send-Q at the middle of the pause must reach this many bytes.
const BACK_PRESSURE_BYTES: u64 = 256 * 1024;
/// P7: the server's RSS may rise by this many bytes during the pause.
const RSS_RISE_LIMIT_BYTES: u64 = 16 * 1024 * 1024;

// ---------------------------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn spike_dir() -> PathBuf {
    let dir = workspace_root().join("target/spike");
    fs::create_dir_all(&dir).expect("create target/spike");
    dir
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis() as u64
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind port 0")
        .local_addr()
        .expect("local addr")
        .port()
}

/// One stdout line of a child process.
#[derive(Clone, Debug)]
struct Line {
    text: String,
}

/// A child process. Drop kills it. Its stdout lines are echoed to stderr and kept.
struct Proc {
    name: &'static str,
    child: Child,
    lines: mpsc::Receiver<Line>,
    seen: Vec<Line>,
}

impl Proc {
    fn spawn(name: &'static str, cmd: &mut Command) -> Proc {
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {name}: {e}"));
        let stdout = child.stdout.take().expect("stdout");
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for text in BufReader::new(stdout).lines().map_while(Result::ok) {
                eprintln!("[{name}] {text}");
                if tx.send(Line { text }).is_err() {
                    break;
                }
            }
        });
        Proc {
            name,
            child,
            lines: rx,
            seen: Vec::new(),
        }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Reads stdout until a line starts with `prefix`. Returns false on timeout or EOF.
    fn wait_line(&mut self, prefix: &str, timeout: Duration) -> bool {
        let end = Instant::now() + timeout;
        if self.seen.iter().any(|l| l.text.starts_with(prefix)) {
            return true;
        }
        loop {
            let left = end.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => {
                    let hit = line.text.starts_with(prefix);
                    self.seen.push(line);
                    if hit {
                        return true;
                    }
                }
                Err(_) => {
                    eprintln!(
                        "[harness] {} printed no line starting with {prefix:?}",
                        self.name
                    );
                    return false;
                }
            }
        }
    }

    /// All stdout lines read so far, plus the lines that are waiting.
    fn all_lines(&mut self) -> Vec<Line> {
        while let Ok(line) = self.lines.try_recv() {
            self.seen.push(line);
        }
        self.seen.clone()
    }

    /// Waits for the process to exit. Returns false on timeout.
    fn wait_exit(&mut self, timeout: Duration) -> bool {
        let end = Instant::now() + timeout;
        while Instant::now() < end {
            if let Ok(Some(_)) = self.child.try_wait() {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Sends SIGTERM and waits for the exit.
    fn terminate(&mut self) {
        let _ = Command::new("kill")
            .args(["-TERM", &self.pid().to_string()])
            .status();
        self.wait_exit(Duration::from_secs(5));
    }

    /// Sends SIGKILL and waits for the exit.
    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        self.kill();
    }
}

fn start_server(port: u16, units: u32, stats: Option<&Path>) -> Proc {
    let mut cmd = Command::new(SERVER_BIN);
    cmd.args([
        "--ws",
        &format!("127.0.0.1:{port}"),
        "--units",
        &units.to_string(),
    ]);
    if let Some(path) = stats {
        let _ = fs::remove_file(path);
        cmd.arg("--stats").arg(path);
    }
    let mut server = Proc::spawn("server", &mut cmd);
    // `listening` comes when the transport has bound the port (lightyear `Linked` on `Server`).
    server.wait_line("spike_server listening", Duration::from_secs(10));
    server
}

fn start_relay(listen: u16, target: u16, extra: &[&str], out: &Path) -> Proc {
    let _ = fs::remove_file(out);
    let script = workspace_root().join("tests/net/relay.py");
    let mut cmd = Command::new("python3");
    cmd.arg(script)
        .args([listen.to_string(), target.to_string()])
        .args(extra)
        .arg("--out")
        .arg(out);
    let mut relay = Proc::spawn("relay", &mut cmd);
    relay.wait_line("relay listening", Duration::from_secs(10));
    relay
}

fn start_client(port: u16, args: &[&str], out: &Path) -> Proc {
    let _ = fs::remove_file(out);
    let mut cmd = Command::new(CLIENT_BIN);
    cmd.args(["--ws", &format!("127.0.0.1:{port}")])
        .args(args)
        .arg("--out")
        .arg(out);
    Proc::spawn("client", &mut cmd)
}

/// Reads a JSON file. A missing or bad file gives `Value::Null`, so each field reads as null.
fn read_json(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            eprintln!("[harness] {} is not JSON: {e}", path.display());
            Value::Null
        }),
        Err(e) => {
            eprintln!("[harness] cannot read {}: {e}", path.display());
            Value::Null
        }
    }
}

/// Reads the server's stats file: one JSON object per line. A cut last line is skipped.
fn read_stats(path: &Path) -> Vec<Value> {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

fn u64_of(v: &Value, key: &str) -> u64 {
    v[key]
        .as_u64()
        .unwrap_or_else(|| panic!("field {key} missing or not an integer in {v}"))
}

/// Tick durations (microseconds) of the stats windows that end in `(from_ms, to_ms]` and
/// start at or after `from_ms`. Each line holds the ticks of the second before `unix_ms`.
fn ticks_between(stats: &[Value], from_ms: u64, to_ms: u64) -> Vec<u64> {
    stats
        .iter()
        .filter(|s| {
            let end = u64_of(s, "unix_ms");
            end >= from_ms + 1000 && end <= to_ms
        })
        .flat_map(|s| {
            s["tick_us"]
                .as_array()
                .expect("tick_us array")
                .iter()
                .map(|v| v.as_u64().expect("tick_us value"))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn p99(samples: &[u64]) -> f64 {
    assert!(!samples.is_empty(), "no tick samples in the window");
    let mut s = samples.to_vec();
    s.sort_unstable();
    let rank = ((s.len() as f64) * 0.99).ceil() as usize;
    s[rank.saturating_sub(1)] as f64
}

fn p99_bound_us(baseline_p99: f64) -> f64 {
    (baseline_p99 * P99_BOUND_FRACTION).max(P99_BOUND_FLOOR_US)
}

/// The largest Send-Q (bytes) of the established TCP sockets with local port `port`.
fn send_q(port: u16) -> u64 {
    let out = Command::new("ss")
        .args(["-tni", &format!("( sport = :{port} )")])
        .output()
        .expect("run ss");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("ESTAB"))
        .filter_map(|l| l.split_whitespace().nth(2)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

/// The unix time of the relay line `<word> <unix_ms>`.
fn relay_mark(lines: &[Line], word: &str) -> u64 {
    lines
        .iter()
        .find_map(|l| {
            let mut it = l.text.split_whitespace();
            (it.next() == Some(word)).then(|| it.next()?.parse::<u64>().ok())?
        })
        .unwrap_or_else(|| panic!("relay printed no {word:?} line"))
}

fn run_cargo(args: &[&str]) -> (bool, String) {
    let out = Command::new(cargo())
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("run cargo");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

// ---------------------------------------------------------------------------------------------
// P1 and P2: dependencies
// ---------------------------------------------------------------------------------------------

/// The version strings of `[workspace.dependencies]` in the root `Cargo.toml`.
fn workspace_dependency_versions() -> Vec<(String, String)> {
    let text = fs::read_to_string(workspace_root().join("Cargo.toml")).expect("read Cargo.toml");
    let mut in_section = false;
    let mut found = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == "[workspace.dependencies]";
            continue;
        }
        if !in_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, rest) = line.split_once('=').expect("name = value");
        let rest = rest.trim();
        let version = if rest.starts_with('"') {
            rest.trim_matches('"').to_string()
        } else {
            let after = rest
                .split_once("version")
                .unwrap_or_else(|| panic!("no version for {name}: {line}"))
                .1;
            let after = after.split_once('"').expect("version quote").1;
            after
                .split_once('"')
                .expect("version end quote")
                .0
                .to_string()
        };
        found.push((name.trim().to_string(), version));
    }
    assert!(!found.is_empty(), "no [workspace.dependencies] found");
    found
}

const PERMISSIVE: [&str; 8] = [
    "MIT",
    "Apache-2.0",
    "BSD",
    "ISC",
    "Zlib",
    "Unicode",
    "MPL-2.0",
    "CC0",
];

#[test]
#[ignore]
fn p1_lockfile_pinned_and_licences_recorded() {
    let (ok, text) = run_cargo(&["metadata", "--locked", "--format-version", "1"]);
    assert!(ok, "cargo metadata --locked failed:\n{text}");

    for (name, version) in workspace_dependency_versions() {
        assert!(
            version.starts_with('='),
            "workspace dependency {name} is not pinned: {version:?}"
        );
    }

    let (ok, text) = run_cargo(&[
        "tree",
        "-p",
        "turts_net_lightyear",
        "--features",
        "client,server",
        "-e",
        "normal",
        "--prefix",
        "none",
        "--format",
        "{p} {l}",
    ]);
    assert!(ok, "cargo tree failed:\n{text}");
    let lines: BTreeSet<String> = text
        .lines()
        .filter(|l| !l.contains(" (/"))
        .map(|l| l.trim_end_matches(" (*)").to_string())
        .collect();
    let path = spike_dir().join("licences.txt");
    let listing: String = lines.iter().map(|l| format!("{l}\n")).collect();
    fs::write(&path, listing).expect("write licences.txt");
    eprintln!("[p1] {} packages, list in {}", lines.len(), path.display());
    for line in &lines {
        assert!(
            PERMISSIVE.iter().any(|p| line.contains(p)),
            "licence not in the permissive list: {line}"
        );
    }
}

#[test]
#[ignore]
fn p2_server_has_no_render_crates_and_client_checks_on_wasm() {
    let (ok, text) = run_cargo(&[
        "tree",
        "-p",
        "turts_net_lightyear",
        "--features",
        "server",
        "--prefix",
        "none",
    ]);
    assert!(ok, "cargo tree --features server failed:\n{text}");
    for line in text.lines() {
        for forbidden in ["bevy_render", "bevy_winit", "bevy_pbr"] {
            assert!(
                !line.starts_with(forbidden),
                "server build depends on {forbidden}: {line}"
            );
        }
    }

    let (ok, text) = run_cargo(&[
        "check",
        "-p",
        "turts_net_lightyear",
        "--features",
        "client",
        "--target",
        "wasm32-unknown-unknown",
    ]);
    assert!(ok, "client check for wasm32 failed:\n{text}");
}

// ---------------------------------------------------------------------------------------------
// P3 and P4: session and commands
// ---------------------------------------------------------------------------------------------

#[test]
#[ignore]
fn p3_connect_hello_welcome_and_bad_hash_rejected() {
    let dir = spike_dir();
    let port = free_port();
    let _server = start_server(port, 1, None);

    let out = dir.join("p3_good.json");
    let mut client = start_client(port, &["--hash", "sha256:spike", "--run-secs", "2"], &out);
    client.wait_exit(Duration::from_secs(10));
    let good = read_json(&out);
    eprintln!("[p3] good hash: {good}");
    assert_eq!(
        good["welcomed"],
        json!(true),
        "welcomed == false with the right hash"
    );

    let out = dir.join("p3_bad.json");
    let mut client = start_client(port, &["--hash", "sha256:wrong", "--run-secs", "2"], &out);
    client.wait_exit(Duration::from_secs(10));
    let bad = read_json(&out);
    eprintln!("[p3] wrong hash: {bad}");
    assert_eq!(
        bad["welcomed"],
        json!(false),
        "welcomed with the wrong hash"
    );
    assert_eq!(bad["rejected_reason"], json!("lockfile hash mismatch"));
    assert_eq!(
        bad["disconnected_after_connect"],
        json!(true),
        "no disconnect within 2 s of the client start"
    );
}

#[test]
#[ignore]
fn p4_thousand_moves_get_one_ack_each() {
    let port = free_port();
    let _server = start_server(port, 1, None);
    let out = spike_dir().join("p4.json");
    let mut client = start_client(port, &["--moves", "1000", "--run-secs", "40"], &out);
    client.wait_exit(Duration::from_secs(60));
    let r = read_json(&out);
    eprintln!("[p4] {r}");
    assert_eq!(r["acks"], json!(1000), "acks != 1000");
    assert_eq!(r["ack_seqs_unique"], json!(1000), "ack_seqs_unique != 1000");
}

// ---------------------------------------------------------------------------------------------
// P5 to P7: measured runs. Run them on an idle machine.
// ---------------------------------------------------------------------------------------------

fn p5_run(units: u32) -> Value {
    let dir = spike_dir();
    let port = free_port();
    let relay_port = free_port();
    let stats_path = dir.join(format!("p5_stats_{units}.jsonl"));
    let relay_out = dir.join(format!("p5_relay_{units}.json"));
    let client_out = dir.join(format!("p5_client_{units}.json"));

    let mut server = start_server(port, units, Some(&stats_path));
    let mut relay = start_relay(relay_port, port, &[], &relay_out);
    let mut client = start_client(relay_port, &["--run-secs", "60"], &client_out);
    assert!(
        client.wait_exit(Duration::from_secs(90)),
        "client did not exit"
    );
    if !relay.wait_exit(Duration::from_secs(5)) {
        relay.terminate();
    }
    // One more stats line after the client left, then stop the server.
    thread::sleep(Duration::from_millis(1500));
    server.kill();

    let c = read_json(&client_out);
    let r = read_json(&relay_out);
    let stats = read_stats(&stats_path);
    let last = stats.last().expect("no stats line").clone();
    eprintln!("[p5 units={units}] client {c}");
    eprintln!("[p5 units={units}] relay {r}");

    let deltas = u64_of(&c, "deltas");
    let first_tick = u64_of(&c, "first_tick");
    let last_tick = u64_of(&c, "last_tick");
    let span = last_tick - first_tick + 1;
    let to_client = u64_of(&r, "to_client_bytes");
    let ticks = u64_of(&last, "ticks");
    let payload_total = u64_of(&last, "delta_payload_bytes_total");
    let bytes_per_tick = to_client as f64 / deltas as f64;
    let payload_per_tick = payload_total as f64 / ticks as f64;
    let result = json!({
        "units": units,
        "deltas": deltas,
        "first_tick": first_tick,
        "last_tick": last_tick,
        "lost": span - deltas,
        "deltas_in_order": c["deltas_in_order"],
        "to_client_bytes": to_client,
        "to_server_bytes": r["to_server_bytes"],
        "bytes_per_tick": bytes_per_tick,
        "payload_bytes_per_tick": payload_per_tick,
        "overhead_bytes_per_tick": bytes_per_tick - payload_per_tick,
        "server_ticks": ticks,
        "delta_payload_bytes_total": payload_total,
    });
    eprintln!("[p5 units={units}] result {result}");

    assert_eq!(c["deltas_in_order"], json!(true), "deltas out of order");
    assert!(deltas >= 1140, "deltas {deltas} < 1140");
    assert_eq!(
        deltas, span,
        "deltas {deltas} != last_tick - first_tick + 1 = {span}"
    );
    result
}

#[test]
#[ignore]
fn p5_sixty_seconds_of_deltas_in_order() {
    let one = p5_run(1);
    let thousand = p5_run(1000);
    let all = json!({ "units_1": one, "units_1000": thousand });
    fs::write(spike_dir().join("p5.json"), all.to_string()).expect("write p5.json");
}

/// One P6 run: server with 1,000 units, one client for 20 s. With `kill_at`, the harness
/// kills the client (SIGKILL) at that second. Returns (stats, client start unix ms).
fn p6_run(name: &str, kill_at: Option<u64>) -> (Vec<Value>, u64) {
    let dir = spike_dir();
    let port = free_port();
    let stats_path = dir.join(format!("p6_stats_{name}.jsonl"));
    let client_out = dir.join(format!("p6_client_{name}.json"));
    let mut server = start_server(port, 1000, Some(&stats_path));
    let start = unix_ms();
    let mut client = start_client(port, &["--run-secs", "30"], &client_out);
    if let Some(at) = kill_at {
        thread::sleep(Duration::from_secs(at));
        client.kill();
        eprintln!("[p6 {name}] client killed at {at} s");
        thread::sleep(Duration::from_secs(20 - at));
    } else {
        thread::sleep(Duration::from_secs(20));
    }
    thread::sleep(Duration::from_millis(1200));
    server.kill();
    client.kill();
    (read_stats(&stats_path), start)
}

#[test]
#[ignore]
fn p6_killed_client_keeps_tick_p99() {
    let (base_stats, base_start) = p6_run("baseline", None);
    let base = ticks_between(&base_stats, base_start + 1000, base_start + 20_000);
    let (kill_stats, kill_start) = p6_run("killed", Some(10));
    let after = ticks_between(&kill_stats, kill_start + 11_000, kill_start + 20_000);

    let base_p99 = p99(&base);
    let after_p99 = p99(&after);
    let bound = p99_bound_us(base_p99);
    let result = json!({
        "baseline_samples": base.len(),
        "baseline_p99_us": base_p99,
        "baseline_max_us": base.iter().max(),
        "after_kill_samples": after.len(),
        "after_kill_p99_us": after_p99,
        "after_kill_max_us": after.iter().max(),
        "bound_us": bound,
    });
    eprintln!("[p6] {result}");
    fs::write(spike_dir().join("p6.json"), result.to_string()).expect("write p6.json");
    assert!(
        after_p99 <= base_p99 + bound,
        "tick p99 after the kill {after_p99} us > baseline {base_p99} us + bound {bound} us"
    );
}

#[test]
#[ignore]
fn p7_stalled_reader_keeps_tick_p99_and_bounded_memory() {
    p7_with_relay_args(&[
        "--rcvbuf",
        "4096",
        "--pause-after",
        "10",
        "--pause-secs",
        "5",
    ]);
}

fn p7_with_relay_args(relay_args: &[&str]) {
    let dir = spike_dir();
    let port = free_port();
    let relay_port = free_port();
    let stats_path = dir.join("p7_stats.jsonl");
    let relay_out = dir.join("p7_relay.json");
    let client_out = dir.join("p7_client.json");

    let mut server = start_server(port, 1000, Some(&stats_path));
    let mut relay = start_relay(relay_port, port, relay_args, &relay_out);
    let client_start = unix_ms();
    let mut client = start_client(relay_port, &["--run-secs", "20"], &client_out);

    // Sample the server's Send-Q every 500 ms while the client runs.
    let mut samples: Vec<(u64, u64)> = Vec::new();
    let end = Instant::now() + Duration::from_secs(25);
    while Instant::now() < end {
        samples.push((unix_ms(), send_q(port)));
        if let Ok(Some(_)) = client.child.try_wait() {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    assert!(
        client.wait_exit(Duration::from_secs(10)),
        "client did not exit"
    );
    if !relay.wait_exit(Duration::from_secs(5)) {
        relay.terminate();
    }
    thread::sleep(Duration::from_millis(1200));
    server.kill();

    let relay_lines = relay.all_lines();
    let pause_start = relay_mark(&relay_lines, "pause");
    let pause_end = relay_mark(&relay_lines, "resume");
    let mid = (pause_start + pause_end) / 2;
    let c = read_json(&client_out);
    let r = read_json(&relay_out);
    let stats = read_stats(&stats_path);
    eprintln!("[p7] client {c}");
    eprintln!("[p7] relay {r}");

    let mid_send_q = samples
        .iter()
        .filter(|(t, _)| t.abs_diff(mid) <= 500)
        .map(|(_, q)| *q)
        .max()
        .unwrap_or(0);
    let peak_send_q = samples.iter().map(|(_, q)| *q).max().unwrap_or(0);

    let base = ticks_between(&stats, client_start + 2000, pause_start);
    let during = ticks_between(&stats, pause_start, pause_end + 1000);
    let base_p99 = p99(&base);
    // A run with no pause has no samples here. NaN fails the p99 assertion, which comes after
    // the back-pressure assertion, so such a run fails on "no back-pressure".
    let during_p99 = if during.is_empty() {
        f64::NAN
    } else {
        p99(&during)
    };
    let bound = p99_bound_us(base_p99);

    let before = stats
        .iter()
        .rev()
        .find(|s| u64_of(s, "unix_ms") <= pause_start)
        .expect("no stats line before the pause");
    let rss_before = u64_of(before, "resident_bytes");
    let rss_peak = stats
        .iter()
        .filter(|s| {
            let t = u64_of(s, "unix_ms");
            t > pause_start && t <= pause_end + 1000
        })
        .map(|s| u64_of(s, "resident_peak_bytes"))
        .max()
        .expect("no stats line in the pause");
    let after = stats.last().expect("no stats line");
    let rss_after = u64_of(after, "resident_bytes");
    let at_resume = stats
        .iter()
        .find(|s| u64_of(s, "unix_ms") >= pause_end)
        .expect("no stats line after the pause");
    let payload_pause = u64_of(at_resume, "delta_payload_bytes_total")
        - u64_of(before, "delta_payload_bytes_total");
    let tick_at_resume = u64_of(at_resume, "ticks");

    let first_tick = c["first_tick"].as_u64().unwrap_or(0);
    let last_tick = c["last_tick"].as_u64().unwrap_or(0);
    let deltas = c["deltas"].as_u64().unwrap_or(0);
    let result = json!({
        "relay_args": relay_args,
        "pause_ms": pause_end - pause_start,
        "send_q_mid_pause_bytes": mid_send_q,
        "send_q_peak_bytes": peak_send_q,
        "send_q_samples": samples.iter().map(|(t, q)| json!([t, q])).collect::<Vec<_>>(),
        "baseline_p99_us": base_p99,
        "pause_p99_us": during_p99,
        "pause_max_us": during.iter().max(),
        "bound_us": bound,
        "rss_before_bytes": rss_before,
        "rss_peak_bytes": rss_peak,
        "rss_after_bytes": rss_after,
        "rss_rise_bytes": rss_peak.saturating_sub(rss_before),
        "delta_payload_bytes_during_pause": payload_pause,
        "client": c,
        "relay": r,
        "lost_deltas": (last_tick + 1).saturating_sub(first_tick).saturating_sub(deltas),
        "server_tick_at_resume": tick_at_resume,
    });
    fs::write(dir.join("p7.json"), result.to_string()).expect("write p7.json");
    eprintln!("[p7] result {result}");

    assert_eq!(
        c["disconnected_after_connect"],
        json!(false),
        "the client disconnected"
    );
    assert!(
        last_tick > tick_at_resume,
        "deltas did not resume: last_tick {last_tick} <= server tick at resume {tick_at_resume}"
    );
    assert!(
        mid_send_q >= BACK_PRESSURE_BYTES,
        "no back-pressure: Send-Q {mid_send_q} bytes at the middle of the pause"
    );
    assert!(
        during_p99 <= base_p99 + bound,
        "tick p99 in the pause {during_p99} us > baseline {base_p99} us + bound {bound} us"
    );
    assert!(
        rss_peak.saturating_sub(rss_before) <= RSS_RISE_LIMIT_BYTES,
        "RSS rise {} bytes > {} bytes",
        rss_peak.saturating_sub(rss_before),
        RSS_RISE_LIMIT_BYTES
    );
}
