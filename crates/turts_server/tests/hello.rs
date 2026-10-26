//! Process-level `Hello` tests: a real `turts_server` and real `turts_client` processes.
//! They carry `#[ignore]` and run through `scripts/acceptance.sh` in release, serially. They
//! need the client binary (`cargo build -p turts_client` first).

mod support;

use std::path::Path;
use std::time::Duration;

use support::{pack, run_client, spawn_server, test_dir, wait_json};

/// How long a client may take to write its `--out` file after its `--run-secs` ends.
const CLIENT_EXIT_TIMEOUT: Duration = Duration::from_secs(15);

fn path_text(path: &Path) -> &str {
    path.to_str().expect("the test paths are UTF-8")
}

/// Review Focus 1 (D25): two clients send `Hello` with the same `player_id`. The second gets
/// `JoinRejected { reason: "player already in match" }` and the first keeps its slot.
#[test]
#[ignore]
fn hello_duplicate_player_is_rejected() {
    let dir = test_dir("hello_duplicate_player");
    let server = spawn_server(&pack("base", &dir), &[]);
    let ws = format!("ws://{}", server.ws_addr);
    let out_first = dir.join("first.json");
    let out_second = dir.join("second.json");
    let start = |out: &Path| {
        run_client(&[
            "--headless",
            "--server",
            &server.http_url,
            "--ws",
            &ws,
            "--player-id",
            "7",
            "--run-secs",
            "3",
            "--out",
            path_text(out),
        ])
    };
    let first = start(&out_first);
    // The second client starts after the first one joined, so the first owns the slot.
    std::thread::sleep(Duration::from_millis(500));
    let second = start(&out_second);
    let first = wait_json(first, &out_first, CLIENT_EXIT_TIMEOUT);
    let second = wait_json(second, &out_second, CLIENT_EXIT_TIMEOUT);

    assert_eq!(
        first["welcomed"], true,
        "the first client keeps its slot: {first}"
    );
    assert_eq!(first["rejected_reason"], serde_json::Value::Null);
    assert_eq!(second["welcomed"], false, "the second client: {second}");
    assert_eq!(second["rejected_reason"], "player already in match");
}

/// A `Hello` with another protocol version gets `JoinRejected`, then the server disconnects the
/// client. The client writes its report after 2 s, so a true `disconnected_after_connect` means
/// the disconnect came within 2 s.
#[test]
#[ignore]
fn hello_wrong_protocol_is_rejected() {
    let dir = test_dir("hello_wrong_protocol");
    let server = spawn_server(&pack("base", &dir), &[]);
    let ws = format!("ws://{}", server.ws_addr);
    let out = dir.join("client.json");
    let client = run_client(&[
        "--headless",
        "--server",
        &server.http_url,
        "--ws",
        &ws,
        "--protocol-version",
        "99",
        "--run-secs",
        "2",
        "--out",
        path_text(&out),
    ]);
    let report = wait_json(client, &out, CLIENT_EXIT_TIMEOUT);

    assert_eq!(report["welcomed"], false, "{report}");
    assert_eq!(report["rejected_reason"], "protocol version mismatch");
    assert_eq!(
        report["disconnected_after_connect"], true,
        "the server disconnects a rejected client within 2 s: {report}"
    );
}
