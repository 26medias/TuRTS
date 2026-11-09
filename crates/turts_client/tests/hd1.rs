//! HD1: the native process accepts action files and reports real order results.

use std::collections::BTreeMap;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use turts_mod_runtime::pack::pack_dir;

const READY_TIMEOUT: Duration = Duration::from_secs(10);

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_muster(dir: &Path) -> (Server, String, String) {
    let package_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/browser/fixtures/hd1/muster");
    let package = dir.join("muster.tar");
    fs::write(&package, pack_dir(&package_dir).expect("pack Muster")).unwrap();

    let http_socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let ws_socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let http = http_socket.local_addr().unwrap();
    let ws = ws_socket.local_addr().unwrap();
    drop((http_socket, ws_socket));
    let base = format!("http://{http}");
    let config = dir.join("server.ron");
    fs::write(
        &config,
        format!(
            "(game_package: {:?}, http_listen: {:?}, ws_listen: {:?}, public_url: {:?}, tick_hz: 20,)",
            package.display().to_string(),
            http.to_string(),
            ws.to_string(),
            base,
        ),
    )
    .unwrap();

    let target = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("target dir"));
    let server_path = target.join("release/turts_server");
    let log = fs::File::create(dir.join("server.log")).unwrap();
    let server_args = vec![config.display().to_string()];
    fs::write(
        dir.join("server-argv.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "executable": server_path.display().to_string(), "args": server_args,
        }))
        .unwrap(),
    )
    .unwrap();
    let child = Command::new(server_path)
        .arg(&config)
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("turts_server starts");
    let mut server = Server(child);
    let started = Instant::now();
    loop {
        if let Some(status) = server.0.try_wait().unwrap() {
            panic!("Muster server exited early: {status}");
        }
        if ureq::get(&format!("{base}/advertisement.json"))
            .call()
            .is_ok()
        {
            return (server, base, format!("ws://{ws}"));
        }
        assert!(
            started.elapsed() < READY_TIMEOUT,
            "Muster server did not start"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn action_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/browser/fixtures/hd1-actions.json")
}

fn camera_action_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/browser/fixtures/hd1-camera-actions.json")
}

#[test]
#[ignore = "Task 10 HD1 process acceptance"]
fn headless_actions_drive_core_and_report_per_unit_results_on_muster() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("hd1");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let (_server, http, ws) = start_muster(&dir);
    let out = dir.join("out.json");
    let actions = action_fixture();
    assert!(
        actions.is_file(),
        "action fixture is present: {}",
        actions.display()
    );
    let client_executable = env!("CARGO_BIN_EXE_turts_client");
    let client_args = vec![
        "--headless",
        "--server",
        http.as_str(),
        "--ws",
        ws.as_str(),
        "--actions",
        actions.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--run-secs",
        "6",
    ];
    fs::write(
        dir.join("client-argv.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "executable": client_executable, "args": client_args,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("test-executable.txt"),
        std::env::current_exe().unwrap().display().to_string(),
    )
    .unwrap();
    let env_names = [
        "CARGO_TARGET_DIR",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_PROFILE_TEST_DEBUG",
        "CARGO_INCREMENTAL",
        "CARGO_BUILD_JOBS",
        "RUSTFLAGS",
    ];
    let execution_env: BTreeMap<_, _> = env_names
        .into_iter()
        .filter_map(|name| std::env::var(name).ok().map(|value| (name, value)))
        .collect();
    fs::write(
        dir.join("execution-env.json"),
        serde_json::to_vec_pretty(&execution_env).unwrap(),
    )
    .unwrap();
    let output = Command::new(client_executable)
        .args(&client_args)
        .output()
        .expect("turts_client starts");
    assert!(
        output.status.success(),
        "headless actions failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("--out report exists")).unwrap();
    assert_eq!(
        report["welcomed"], true,
        "the client joins Muster: {report}"
    );
    let units = report["replicated_world"]["units"]
        .as_array()
        .expect("the actual replicated unit list is reported");
    let mut by_kind = BTreeMap::<&str, Vec<u32>>::new();
    for unit in units {
        if unit["owner_slot"] == 0
            && let Some(content) = unit["content_id"].as_str()
            && (content == "muster:trooper" || content == "muster:worker")
        {
            by_kind
                .entry(content)
                .or_default()
                .push(unit["unit_id"].as_u64().unwrap() as u32);
        }
    }
    for members in by_kind.values_mut() {
        members.sort_unstable();
    }
    let troopers = by_kind.get("muster:trooper").expect("Muster troopers");
    let workers = by_kind.get("muster:worker").expect("Muster workers");
    assert_eq!(
        troopers.len(),
        39,
        "the Remove behavior removes the clicked trooper from the replicated world"
    );
    assert_eq!(workers.len(), 2);

    let group_ids = |value: &serde_json::Value| {
        let mut ids: Vec<u32> = value
            .as_array()
            .expect("group membership is an array")
            .iter()
            .map(|id| id.as_u64().unwrap() as u32)
            .collect();
        ids.sort_unstable();
        ids
    };
    assert_eq!(
        group_ids(&report["groups"]["1"]),
        *troopers,
        "group 1 assignment; client stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        group_ids(&report["groups"]["2"]),
        *workers,
        "group 2 assignment"
    );
    assert_eq!(
        group_ids(&report["selection"]),
        *troopers,
        "the final group recall selects group 1"
    );

    let results = report["results"].as_array().expect("per-unit results");
    let units_for = |sequence: u64, status: &str| {
        let mut ids: Vec<u32> = results
            .iter()
            .filter(|result| result["seq"] == sequence && result["status"] == status)
            .map(|result| result["unit"].as_u64().unwrap() as u32)
            .collect();
        ids.sort_unstable();
        ids
    };
    let exact_result = |sequence: u64, accepted: &[u32], refused: &[u32]| {
        let mut accepted_expected = accepted.to_vec();
        accepted_expected.sort_unstable();
        let mut refused_expected = refused.to_vec();
        refused_expected.sort_unstable();
        assert_eq!(
            units_for(sequence, "accepted"),
            accepted_expected,
            "accepted set for sequence {sequence}"
        );
        assert_eq!(
            units_for(sequence, "refused"),
            refused_expected,
            "refused set for sequence {sequence}"
        );
        assert!(
            results
                .iter()
                .filter(|result| result["seq"] == sequence)
                .all(|result| matches!(result["status"].as_str(), Some("accepted" | "refused"))),
            "sequence {sequence} contains no extra status results"
        );
    };
    assert_eq!(units_for(2, "accepted"), *troopers, "smart move result");
    assert_eq!(
        units_for(3, "accepted"),
        *troopers,
        "attack on a dummy is accepted"
    );
    assert_eq!(
        units_for(4, "accepted"),
        *workers,
        "workers gather at the node"
    );
    assert_eq!(units_for(5, "accepted"), *workers, "worker move result");
    assert_eq!(units_for(6, "accepted"), *troopers, "stop result");
    assert_eq!(units_for(7, "accepted"), *troopers, "hold result");
    assert_eq!(
        units_for(8, "refused"),
        *troopers,
        "the real server refuses Shift+Move while Hold is active"
    );
    exact_result(1, &[1], &[]);
    for sequence in 2..=5 {
        let expected = match sequence {
            2 | 3 => troopers.as_slice(),
            4 | 5 => workers.as_slice(),
            _ => unreachable!(),
        };
        exact_result(sequence, expected, &[]);
    }
    exact_result(6, troopers, &[]);
    exact_result(7, troopers, &[]);
    exact_result(8, &[], troopers);
    exact_result(9, &[43], &[]);
    exact_result(10, &[43], &[]);
    exact_result(11, &[43], &[]);
    exact_result(12, &[43], &[]);
}

#[test]
#[ignore = "Task 10 focused C195 camera reprojection process acceptance"]
fn same_tick_double_group_recall_reprojects_the_following_world_target() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("hd1-camera");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let (_server, http, ws) = start_muster(&dir);
    let out = dir.join("out.json");
    let actions = camera_action_fixture();
    assert!(
        actions.is_file(),
        "C195 action fixture is present: {}",
        actions.display()
    );
    let client_executable = env!("CARGO_BIN_EXE_turts_client");
    let client_args = [
        "--headless",
        "--server",
        http.as_str(),
        "--ws",
        ws.as_str(),
        "--actions",
        actions.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--run-secs",
        "4",
    ];
    fs::write(
        dir.join("client-argv.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "executable": client_executable, "args": client_args,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("test-executable.txt"),
        std::env::current_exe().unwrap().display().to_string(),
    )
    .unwrap();
    let output = Command::new(client_executable)
        .args(client_args)
        .output()
        .expect("headless client starts");
    assert!(
        output.status.success(),
        "C195 headless actions failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("C195 report exists")).unwrap();
    assert_eq!(report["welcomed"], true);
    assert_eq!(report["groups"]["2"], serde_json::json!([41, 42]));
    let orders = report["replicated_world"]["unit_orders"]
        .as_array()
        .expect("replicated worker order state");
    for unit in [41, 42] {
        let entry = orders
            .iter()
            .find(|entry| entry["unit"] == unit)
            .expect("worker order entry");
        let target = &entry["current"]["target"]["Point"];
        let x = target["x"].as_f64().expect("target x");
        let y = target["y"].as_f64().expect("target y");
        assert!(
            (x - 32.0).abs() < 0.05 && (y - 22.0).abs() < 0.05,
            "same-effective-tick recalls must reproject from the fresh camera; worker {unit} target was ({x}, {y})"
        );
    }
}
