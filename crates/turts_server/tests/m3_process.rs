mod support;

use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const STATE_DEADLINE: Duration = Duration::from_secs(230);

struct ClientProcess(Child);
impl Drop for ClientProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn spawn_client(args: &[String], log_prefix: &Path) -> ClientProcess {
    let stdout = fs::File::create(log_prefix.with_extension("stdout.log"))
        .expect("client stdout log is created");
    let stderr = fs::File::create(log_prefix.with_extension("stderr.log"))
        .expect("client stderr log is created");
    let child = Command::new(support::client_exe())
        .args(args)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .expect("headless client starts");
    ClientProcess(child)
}

fn latest_json(path: &Path) -> Option<Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn wait_report(path: &Path, deadline: Instant, predicate: impl Fn(&Value) -> bool) -> Value {
    loop {
        if let Some(report) = latest_json(path)
            && predicate(&report)
        {
            return report;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for live client report {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn initial_health(report: &Value, unit: u32) -> bool {
    report["health_trace"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|record| record["unit_id"].as_u64() == Some(u64::from(unit)))
        .and_then(|record| record["entries"].as_array())
        .and_then(|entries| entries.first())
        .is_some_and(|entry| entry["current"].as_u64() == Some(100))
}

fn slot_has_research(report: &Value, slot: u8, technology: &str) -> bool {
    report["replicated_world"]["research"]
        .as_array()
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                row["slot"].as_u64() == Some(u64::from(slot))
                    && row["completed"].as_array().is_some_and(|completed| {
                        completed
                            .iter()
                            .any(|entry| entry.as_str() == Some(technology))
                    })
            })
        })
}

fn report_has_research_for_any_slot(report: &Value, technology: &str) -> bool {
    report["replicated_world"]["research"]
        .as_array()
        .is_some_and(|rows| {
            rows.iter().any(|row| {
                row["completed"].as_array().is_some_and(|completed| {
                    completed
                        .iter()
                        .any(|entry| entry.as_str() == Some(technology))
                })
            })
        })
}

#[test]
fn f11_opponent_privacy_check_rejects_owner_research_row_at_any_slot() {
    let report = serde_json::json!({
        "replicated_world": {
            "research": [{
                "slot": 0,
                "completed": ["forge:hardened_shells"]
            }]
        }
    });
    let current_opponent_check =
        !report_has_research_for_any_slot(&report, "forge:hardened_shells");
    assert!(
        !current_opponent_check,
        "an owner research row in the opponent report must fail privacy regardless of row slot"
    );
}

fn complete_research(report: &Value) -> Option<u32> {
    let tick = u32::try_from(report["research_completed_tick"].as_u64()?).ok()?;
    slot_has_research(report, 0, "forge:hardened_shells").then_some(tick)
}

fn release_atomically(path: &Path) {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, b"release\n").expect("coordinator signal temp file is written");
    fs::rename(&temporary, path).expect("coordinator signal is atomically released");
}

fn health_steps(report: &Value, unit: u32, expected_delta: i64, count: usize) {
    let record = report["health_trace"]
        .as_array()
        .expect("report health_trace is an array")
        .iter()
        .find(|record| record["unit_id"].as_u64() == Some(u64::from(unit)))
        .unwrap_or_else(|| panic!("health trace for drone {unit} is absent"));
    let entries = record["entries"]
        .as_array()
        .expect("health entries are an array");
    assert_eq!(
        entries.len(),
        count + 1,
        "drone {unit} retains the initial health and every hit"
    );
    assert_eq!(
        entries[0]["current"].as_i64(),
        Some(100),
        "drone {unit} starts at 100 health"
    );
    for (index, pair) in entries.windows(2).enumerate() {
        assert_eq!(
            pair[1]["current"].as_i64().unwrap() - pair[0]["current"].as_i64().unwrap(),
            expected_delta,
            "drone {unit} hit {} has the expected damage",
            index + 1,
        );
    }
    assert_eq!(
        entries.last().unwrap()["current"].as_i64(),
        Some(0),
        "the killing hit is retained"
    );
}

#[derive(Debug)]
struct FiredAuditEvent {
    target: u32,
    damage: u64,
    applied: u64,
}

fn audited_events(path: &Path) -> (Option<u32>, Vec<FiredAuditEvent>) {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read audit {}: {error}", path.display()));
    let mut research_tick = None;
    let mut shots = Vec::new();
    for line in text.lines() {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if record["record"] != "tick" {
            continue;
        }
        let tick = record["data"]["tick"].as_u64().unwrap_or_default() as u32;
        for event in record["data"]["events"].as_array().into_iter().flatten() {
            let Some(kind) = event["kind"].as_object() else {
                continue;
            };
            if kind.contains_key("ResearchCompleted") {
                research_tick = Some(event["tick"].as_u64().unwrap_or(u64::from(tick)) as u32);
            }
            if let Some(fired) = kind.get("Fired") {
                shots.push(FiredAuditEvent {
                    target: fired["target"].as_u64().unwrap_or_default() as u32,
                    damage: fired["damage"].as_u64().unwrap_or_default(),
                    applied: fired["applied"].as_u64().unwrap_or_default(),
                });
            }
        }
    }
    (research_tick, shots)
}

fn applied_move_tick(report: &Value) -> u32 {
    report["command_results"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|result| result["seq"].as_u64() == Some(1) && result["applied_tick"].is_number())
        .and_then(|result| result["applied_tick"].as_u64())
        .unwrap_or_else(|| {
            panic!(
                "opponent report has no Applied MoveTo result: {}",
                report["command_results"]
            )
        }) as u32
}

fn has_final_health(report: &Value) -> bool {
    [9, 10].into_iter().all(|unit| {
        report["health_trace"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|record| {
                record["unit_id"].as_u64() == Some(unit)
                    && record["entries"].as_array().is_some_and(|entries| {
                        entries
                            .last()
                            .is_some_and(|entry| entry["current"].as_u64() == Some(0))
                    })
            })
    })
}

#[test]
fn f11_server_log_is_retained_when_acceptance_test_unwinds() {
    let dir = support::test_dir("f11-server-log-retention-probe");
    let package = support::pack("forge", &dir);
    let evidence = dir.join("stable-server-evidence");
    let mut original_server_log = Vec::new();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let server = support::spawn_server_with_evidence(&package, &[], Some(evidence.clone()));
        original_server_log = fs::read(server.dir.join("server.log"))
            .expect("the identified server log is readable before unwind");
        panic!("simulated F11 assertion failure after server startup");
    }));
    let panic = outcome.expect_err("the controlled F11 failure is raised after server startup");
    let panic_message = panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str));
    assert_eq!(
        panic_message,
        Some("simulated F11 assertion failure after server startup")
    );
    assert_eq!(
        fs::read(evidence.join("server.log")).expect("the identified server log is retained"),
        original_server_log,
        "failure-path evidence retains the original identified server log bytes"
    );
}

#[test]
#[ignore = "real Forge server and two native headless clients; run through scripts/acceptance.sh"]
fn f11_private_research_coordinator_releases_ordered_opponent_move_and_health_trace() {
    let dir = support::test_dir("f11-m3-private-research");
    let package = support::pack("forge", &dir);
    let audit = dir.join("economy.jsonl");
    let server = support::spawn_server(&package, &["--economy-log", audit.to_str().unwrap()]);
    let owner_report = dir.join("owner-live.json");
    let opponent_report = dir.join("opponent-live.json");
    let release = dir.join("opponent-move.release");
    let ws = format!("ws://{}", server.ws_addr);
    let owner_args = vec![
        "--headless".into(),
        "--server".into(),
        server.http_url.clone(),
        "--ws".into(),
        ws.clone(),
        "--player-id".into(),
        "61001".into(),
        "--run-secs".into(),
        "240".into(),
        "--live-report".into(),
        owner_report.display().to_string(),
        "--produce".into(),
        "8,forge:hardened_shells".into(),
        "--produce-when-inventory".into(),
        "8,forge:shell,10".into(),
    ];
    let opponent_args = vec![
        "--headless".into(),
        "--server".into(),
        server.http_url.clone(),
        "--ws".into(),
        ws,
        "--player-id".into(),
        "61002".into(),
        "--run-secs".into(),
        "240".into(),
        "--live-report".into(),
        opponent_report.display().to_string(),
        "--move".into(),
        "17,4".into(),
        "--move-unit".into(),
        "10".into(),
        "--move-release-file".into(),
        release.display().to_string(),
    ];
    assert!(
        !release.exists(),
        "the unique coordinator release path starts absent"
    );
    let deadline = Instant::now() + STATE_DEADLINE;
    let owner = spawn_client(&owner_args, &dir.join("owner-client"));
    let initial_owner = wait_report(&owner_report, deadline, |report| {
        report["welcomed"] == true && initial_health(report, 9) && initial_health(report, 10)
    });
    assert_eq!(
        initial_owner["slot"], 0,
        "the first welcomed client owns slot 0"
    );

    // Start the opponent only after the owner's replica confirms slot 0. This state barrier
    // makes the slot assignment deterministic without delaying or triggering gameplay.
    let opponent = spawn_client(&opponent_args, &dir.join("opponent-client"));
    let initial_opponent = wait_report(&opponent_report, deadline, |report| {
        report["welcomed"] == true && initial_health(report, 9) && initial_health(report, 10)
    });
    assert_eq!(initial_opponent["slot"], 1);
    // Both native clients started before the first audited shot and report the initial health.
    let (_, early_shots) = audited_events(&audit);
    assert!(
        early_shots.is_empty(),
        "both clients report health 100 before the first audited shot"
    );

    let _completed_owner = wait_report(&owner_report, deadline, |report| {
        complete_research(report).is_some()
    });
    release_atomically(&release);
    let _moved_opponent = wait_report(&opponent_report, deadline, |report| {
        report["command_results"].as_array().is_some_and(|rows| {
            rows.iter()
                .any(|row| row["seq"] == 1 && row["applied_tick"].is_number())
        })
    });
    let final_owner = wait_report(&owner_report, deadline, has_final_health);
    let final_opponent = wait_report(&opponent_report, deadline, has_final_health);

    assert_eq!(final_owner["slot"], 0);
    assert_eq!(final_opponent["slot"], 1);
    assert!(
        !report_has_research_for_any_slot(&final_opponent, "forge:hardened_shells"),
        "opponent report contains no owner research regardless of the leaked row slot"
    );
    let (server_research_tick, shots) = audited_events(&audit);
    let server_research_tick =
        server_research_tick.expect("server audit contains ResearchCompleted");
    let move_tick = applied_move_tick(&final_opponent);
    assert!(
        move_tick > server_research_tick,
        "actual applied MoveTo tick {move_tick} follows server ResearchCompleted tick {server_research_tick}"
    );
    assert_eq!(
        shots.iter().filter(|event| event.target == 9).count(),
        10,
        "server audit has ten shots at drone A"
    );
    assert_eq!(
        shots.iter().filter(|event| event.target == 10).count(),
        5,
        "server audit has five shots at drone B"
    );
    assert!(
        shots
            .iter()
            .filter(|event| event.target == 9)
            .all(|event| event.damage == 10 && event.applied == 10)
    );
    assert!(
        shots
            .iter()
            .filter(|event| event.target == 10)
            .all(|event| event.damage == 20 && event.applied == 20)
    );
    for report in [&final_owner, &final_opponent] {
        health_steps(report, 9, -10, 10);
        health_steps(report, 10, -20, 5);
    }
    let fired: Vec<_> = shots
        .iter()
        .map(|event| {
            serde_json::json!({
                "target": event.target,
                "damage": event.damage,
                "applied": event.applied,
            })
        })
        .collect();
    let evidence = serde_json::json!({
        "owner_slot": final_owner["slot"],
        "opponent_slot": final_opponent["slot"],
        "owner_has_hardened_shells": slot_has_research(&final_owner, 0, "forge:hardened_shells"),
        "opponent_has_hardened_shells": report_has_research_for_any_slot(&final_opponent, "forge:hardened_shells"),
        "server_research_completed_tick": server_research_tick,
        "opponent_move_applied_tick": move_tick,
        "initial_health": { "drone_a": 100, "drone_b": 100 },
        "server_fired_events": fired,
        "owner_health_trace": final_owner["health_trace"],
        "opponent_health_trace": final_opponent["health_trace"],
        "owner_report": owner_report,
        "opponent_report": opponent_report,
        "server_audit": audit,
        "server_log": server.dir.join("server.log"),
        "client_logs": {
            "owner_stdout": dir.join("owner-client.stdout.log"),
            "owner_stderr": dir.join("owner-client.stderr.log"),
            "opponent_stdout": dir.join("opponent-client.stdout.log"),
            "opponent_stderr": dir.join("opponent-client.stderr.log"),
        },
    });
    fs::write(
        dir.join("f11-summary.json"),
        serde_json::to_vec_pretty(&evidence).expect("F11 summary serializes"),
    )
    .expect("F11 summary is retained beside raw reports");
    println!(
        "F11 evidence: {} (server ResearchCompleted tick {}, opponent MoveTo Applied tick {}, 10 x 10 damage at drone A, 5 x 20 damage at drone B)",
        dir.display(),
        server_research_tick,
        move_tick
    );
    drop(owner);
    drop(opponent);
}
