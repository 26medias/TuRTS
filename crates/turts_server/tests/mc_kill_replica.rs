//! Process acceptance: the owner's replica follows a kill by an ordered attack with no resync.
//!
//! The real Muster server applies one fixed-tick `muster:order_attack` (trooper 40 at dummy 46,
//! the MM1 pair). A real native headless client holds slot 0. The weapon record of the attacker
//! is owner-only and its `last_target` names the target. If the server keeps that reference
//! after the target is removed, the client rejects every delta and snapshot until the reference
//! clears (one weapon cooldown), and the owner's view stands still for about 1 s.
//!
//! This test guards one half of the repair: `last_target` never names a removed unit. The other
//! half is that `ordered_target_fired` stays for the ordered attacker, so the Attack order
//! completes and does not fail with `TargetGone`. That marker is sim-internal and is not on the
//! wire, so this process test cannot see it. These sim tests own that half (they go red when the
//! marker is cleared for the ordered attacker, and this process test stays green):
//! `economy::combat::tests::attack_target_gone_after_a_shot_completes_normally`,
//! `ordered_attack_kill_replicates_no_reference_to_the_removed_target` and
//! `kill_by_another_weapon_clears_the_ordered_attacker_target_reference` (`last_failure.is_none()`).

mod support;

use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use serde_json::Value;
use support::mc;
use turts_net_protocol::{OrderEnvelope, OrderTarget, QueueMode, UnitId};

const ATTACKER: u32 = 40;
const TARGET: u32 = 46;
const ATTACK_TICK: u32 = 200;
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

fn latest_json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn target_health(report: &Value) -> Vec<(u64, u64)> {
    report["health_trace"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|record| record["unit_id"].as_u64() == Some(u64::from(TARGET)))
        .and_then(|record| record["entries"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| Some((entry["tick"].as_u64()?, entry["current"].as_u64()?)))
        .collect()
}

fn replica_has_unit(report: &Value, unit: u32) -> bool {
    report["replicated_world"]["units"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|row| row["unit_id"].as_u64() == Some(u64::from(unit)))
}

fn applied_tick(report: &Value) -> Option<u64> {
    report["replicated_world"]["tick"].as_u64()
}

#[test]
#[ignore = "real Muster server and one native headless client; run through scripts/acceptance.sh"]
fn ordered_attack_kill_reaches_the_owner_replica_without_a_resync() {
    let dir = support::test_dir("mc-kill-replica");
    let package = mc::muster_package(&dir);
    let script = dir.join("commands.json");
    mc::write_script(
        &script,
        &[(
            ATTACK_TICK,
            0,
            OrderEnvelope {
                order: "muster:order_attack".into(),
                units: vec![UnitId(ATTACKER)],
                target: OrderTarget::Unit(UnitId(TARGET)),
                queue: QueueMode::Replace,
                group: 0,
            },
        )],
    );
    let server = support::spawn_server(&package, &["--command-script", script.to_str().unwrap()]);
    let report_path = dir.join("owner-live.json");
    let stdout = fs::File::create(dir.join("owner-client.stdout.log")).unwrap();
    let stderr = fs::File::create(dir.join("owner-client.stderr.log")).unwrap();
    let _client = ClientProcess(
        Command::new(support::client_exe())
            .args([
                "--headless",
                "--server",
                &server.http_url,
                "--ws",
                &format!("ws://{}", server.ws_addr),
                "--player-id",
                "62001",
                "--run-secs",
                "110",
                "--live-report",
                report_path.to_str().unwrap(),
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .expect("headless client starts"),
    );

    // Sample the live report. `baseline` is the resync count when the owner replica shows a
    // hit on the target that is not the kill; the kill comes at least one cooldown later.
    let deadline = Instant::now() + DEADLINE;
    let mut baseline: Option<u64> = None;
    let mut samples: Vec<(Instant, u64)> = Vec::new();
    let final_report = loop {
        assert!(
            Instant::now() < deadline,
            "setup/data: no kill of unit {TARGET} in the owner report {} before the deadline; server log:\n{}",
            report_path.display(),
            server.log()
        );
        std::thread::sleep(Duration::from_millis(20));
        let Some(report) = latest_json(&report_path) else {
            continue;
        };
        if report["welcomed"] != true {
            continue;
        }
        assert_eq!(report["slot"], 0, "setup: the only client owns slot 0");
        if let Some(tick) = applied_tick(&report) {
            samples.push((Instant::now(), tick));
        }
        let health = target_health(&report);
        if baseline.is_none()
            && health
                .iter()
                .any(|&(_, current)| current > 0 && current < 40)
        {
            baseline = report["resync_requests_sent"].as_u64();
        }
        // The kill is visible in the owner replica (unit gone) and 40 ticks have passed.
        if !replica_has_unit(&report, TARGET)
            && applied_tick(&report).is_some_and(|tick| {
                health.last().is_some_and(|&(kill, _)| tick >= kill + 40)
                    || samples
                        .first()
                        .is_some_and(|&(_, first)| tick >= first + 600)
            })
        {
            break report;
        }
    };
    let baseline = baseline.unwrap_or_else(|| {
        panic!("setup/data: the owner replica never showed a non-lethal hit on unit {TARGET}")
    });

    // Evidence only: the longest wall-clock time with no new applied tick while joined.
    let mut longest = Duration::ZERO;
    let mut last_change = samples.first().map(|sample| sample.0);
    let mut last_tick = samples.first().map(|sample| sample.1);
    for &(at, tick) in &samples {
        if Some(tick) != last_tick {
            if let Some(changed) = last_change {
                longest = longest.max(at - changed);
            }
            last_change = Some(at);
            last_tick = Some(tick);
        }
    }
    let health = target_health(&final_report);
    let resyncs = final_report["resync_requests_sent"].as_u64().unwrap();
    println!(
        "MC kill replica: baseline resyncs {baseline}, final resyncs {resyncs}, target health {health:?}, longest applied-tick stall {longest:?}, evidence {}",
        dir.display()
    );
    assert_eq!(
        resyncs,
        baseline,
        "the owner replica asked for {} resync(s) across the kill of unit {TARGET} (longest applied-tick stall {longest:?}); report {}",
        resyncs - baseline,
        report_path.display()
    );
    assert_eq!(
        health.last().map(|&(_, current)| current),
        Some(0),
        "the owner replica applied the kill delta itself (health 0 in the removal tick), not a later snapshot: {health:?}"
    );
}
