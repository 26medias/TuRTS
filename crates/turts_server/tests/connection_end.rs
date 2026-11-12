//! Process test for the end of a connection (MP1 design open point 21, task L-ROOM0).
//!
//! It starts a real `turts_server` with `--stats` and a real headless `turts_client`, ends the
//! client in two ways (a SIGKILL, and a clean exit), and reads the private session observer
//! file (`<stats>.sessions.jsonl`). The observer `on_session_removed` writes the record
//! `session_authority_removed` when the `Session` component leaves the connection entity. The
//! test measures the time L from the end of the client process to that record, and it checks
//! that the record belongs to the connection that the `session_added` record names.
//!
//! The test carries `#[ignore]` and runs through `scripts/acceptance.sh` in release. It needs
//! the client binary (`cargo build -p turts_client` first).

mod support;

use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use support::{pack, run_client, spawn_server, test_dir};

/// The observer record must come within this time after the end of the client (design 6.7.8
/// allows the 10 s netcode timeout and LOB1 allows 11 s).
const NOTICE_LIMIT: Duration = Duration::from_secs(11);

/// How long the client may take to join before the test gives up.
const JOIN_LIMIT: Duration = Duration::from_secs(10);

fn path_text(path: &Path) -> &str {
    path.to_str().expect("the test paths are UTF-8")
}

fn unix_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_micros() as u64
}

/// The records of the private observer file with `event`, in file order. A missing file gives
/// an empty list.
fn records(sessions: &Path, event: &str) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(sessions).unwrap_or_default();
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["event"] == event)
        .collect()
}

/// Wait until the file has `count` records with `event`, or `limit` passes.
fn wait_for_records(
    sessions: &Path,
    event: &str,
    count: usize,
    limit: Duration,
) -> Vec<serde_json::Value> {
    let started = Instant::now();
    loop {
        let found = records(sessions, event);
        if found.len() >= count || started.elapsed() > limit {
            return found;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Start the server and one client, wait for the `session_added` record, and return the pieces.
struct Run {
    server: support::ServerProc,
    client: Child,
    sessions: std::path::PathBuf,
    added: serde_json::Value,
}

fn start_run(name: &str, player_id: &str, client_extra: &[&str]) -> Run {
    let dir = test_dir(name);
    let server = spawn_server(&pack("base", &dir), &[]);
    let sessions = std::path::PathBuf::from(format!("{}.sessions.jsonl", path_text(&server.stats)));
    let ws = format!("ws://{}", server.ws_addr);
    let mut args = vec![
        "--headless",
        "--server",
        &server.http_url,
        "--ws",
        &ws,
        "--player-id",
        player_id,
    ];
    args.extend_from_slice(client_extra);
    let client = run_client(&args);
    let added = wait_for_records(&sessions, "session_added", 1, JOIN_LIMIT);
    assert_eq!(
        added.len(),
        1,
        "the client joined and the observer wrote one session_added record:\n{}",
        server.log()
    );
    Run {
        added: added[0].clone(),
        server,
        client,
        sessions,
    }
}

/// Check the removal record of `run` and return L in milliseconds from `ended_us`.
fn removal_lag_ms(run: &Run, ended_us: u64) -> f64 {
    let removed = wait_for_records(
        &run.sessions,
        "session_authority_removed",
        1,
        NOTICE_LIMIT + Duration::from_secs(2),
    );
    assert_eq!(
        removed.len(),
        1,
        "no removal record: the server wrote no `session_authority_removed` record after the \
         client ended; session records:\n{}\nserver log:\n{}",
        std::fs::read_to_string(&run.sessions).unwrap_or_default(),
        run.server.log()
    );
    let removed = &removed[0];
    assert_eq!(
        removed["entity_bits_with_generation_decimal"],
        run.added["entity_bits_with_generation_decimal"],
        "the removal record names the connection of the session_added record"
    );
    let at_us = removed["host_unix_us"].as_u64().expect("host_unix_us");
    let lag_ms = at_us.saturating_sub(ended_us) as f64 / 1000.0;
    assert!(
        lag_ms < NOTICE_LIMIT.as_millis() as f64,
        "the removal record came {lag_ms} ms after the client ended; the limit is {} ms",
        NOTICE_LIMIT.as_millis()
    );
    lag_ms
}

/// A killed client: the process gets SIGKILL, so it sends no close frame and no netcode
/// disconnect packet.
#[test]
#[ignore]
fn connection_end_sigkill_gives_one_removal_record_within_the_limit() {
    let mut run = start_run("connection_end_sigkill", "11", &[]);
    let killed_us = unix_us();
    run.client.kill().expect("SIGKILL reaches the client");
    run.client.wait().expect("the client is reaped");
    let lag_ms = removal_lag_ms(&run, killed_us);
    println!("client_ended_unix_us={killed_us}");
    println!("L_sigkill_ms={lag_ms:.1}");
}

/// A client that ends its own process after a short run (the headless client leaves with its
/// transport closed by the process exit).
#[test]
#[ignore]
fn connection_end_clean_exit_gives_one_removal_record_within_the_limit() {
    let mut run = start_run("connection_end_clean", "12", &["--run-secs", "4"]);
    let status = run.client.wait().expect("the client exits by itself");
    let ended_us = unix_us();
    assert!(
        status.code().is_some(),
        "the client ended by exit, not by a signal: {status}"
    );
    let lag_ms = removal_lag_ms(&run, ended_us);
    println!("client_ended_unix_us={ended_us}");
    println!("L_clean_exit_ms={lag_ms:.1}");
}
