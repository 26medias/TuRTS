//! Task22 EL1 process acceptance. The in-process JSONL reader is the production validator used
//! by the server's `--economy-log` sink; this test validates its captured records, not a replica.

mod support;

use std::fs;
use std::time::{Duration, Instant};

use support::{mc, spawn_server};
use turts_mod_api::Behaviour;
use turts_sim::CommandKind;

#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn el1_muster_movement_and_queued_gather_validate_every_captured_tick() {
    let root = mc::repo_root();
    let dir = std::env::temp_dir().join(format!("el1-muster-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("EL1 evidence directory is created");
    let package = mc::muster_package(&dir);
    let script = root.join("tests/browser/fixtures/gm40-script.json");
    let script_entries = mc::read_script(&script);
    let selected_ids = mc::selected_ids_in_script(&script_entries);
    let economy_log = dir.join("economy.jsonl");
    let order_log = dir.join("orders.log");
    let mut server = spawn_server(
        &package,
        &[
            "--command-script",
            script.to_str().unwrap(),
            "--run-ticks",
            "420",
            "--economy-log",
            economy_log.to_str().unwrap(),
            "--order-log",
            order_log.to_str().unwrap(),
        ],
    );
    let started = Instant::now();
    loop {
        if let Some(status) = server
            .child
            .try_wait()
            .expect("EL1 server process state reads")
        {
            assert!(
                status.success(),
                "EL1 server/validator failed: {}",
                server.log()
            );
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(90),
            "EL1 server did not stop at fixed tick limit: {}",
            server.log()
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    let records = turts_audit::read_jsonl_file(&economy_log).unwrap_or_else(|error| {
        panic!("EL1 actual economy log failed its production validator: {error}")
    });
    let tick_records: Vec<_> = records
        .iter()
        .filter_map(|record| match record {
            turts_audit::EconomyLogRecord::Tick {
                tick,
                before,
                state,
                ..
            } => Some((*tick, before, state)),
            _ => None,
        })
        .collect();
    let actual_end = assert_economy_tick_coverage(&tick_records, 420, &selected_ids);
    for (expected, (actual, before, state)) in (1..=actual_end).zip(&tick_records) {
        assert_eq!(
            *actual, expected,
            "EL1 economy log tick coverage is contiguous"
        );
        assert_eq!(
            state.economy.tick, *actual,
            "EL1 published tick state matches its record tick"
        );
        assert!(
            before.tick_hz.is_some_and(|hz| hz > 0),
            "EL1 actual prior state retains the fixed tick rate"
        );
    }
    assert!(
        records.iter().any(|record| matches!(record,
            turts_audit::EconomyLogRecord::CommandBoundary(command)
                if matches!(&command.input.kind, CommandKind::Order(order)
                    if order.behaviour == Behaviour::Gather
                        && order.queue == turts_sim::QueueMode::Append
                        && command.result.units.accepted == [41]
                        && command.result.units.refused.is_empty())
        )),
        "EL1 fixture must deliver the queued Muster gather through the actual command path"
    );

    // Measure the serialized movement-state contribution from the actual captured AuditState
    // values. This is intentionally a component-size method, not whole-log-by-unit division:
    // the latter attributes duplicated shared catalog/ledger bytes to each unit.
    let (unit_rows, unit_bytes, slot_count, slot_bytes) = movement_state_bytes(&records);
    assert!(
        unit_rows > 0,
        "EL1 fixture emitted no non-default movement state"
    );
    assert!(
        slot_count > 0,
        "EL1 fixture emitted no actual movement-pool slots"
    );
    let metadata = fs::metadata(&economy_log).expect("EL1 economy log file size is available");
    eprintln!(
        "EL1 measured evidence: log_bytes={} movement_unit_state_pairs={} movement_unit_state_json_bytes={} avg_bytes_per_nondefault_unit_state={:.3} pool_slot_count={} pool_slot_json_bytes={} avg_bytes_per_pool_slot={:.3}",
        metadata.len(),
        unit_rows,
        unit_bytes,
        unit_bytes as f64 / unit_rows as f64,
        slot_count,
        slot_bytes,
        slot_bytes as f64 / slot_count as f64
    );
    eprintln!(
        "EL1 measure method: for each captured Tick.before and Tick.state, union unit IDs across nondefault RawMovementProgressAudit, RawPlainMovementProgressAudit, RawPhysicalEscapeAudit, and RawUnitOrdersAudit rows; sum each row's serde_json encoded length and divide by distinct (captured-state, unit ID) pairs. Separately sum encoded actual Vec2 pool slots and divide by actual slots. Count brackets and commas are excluded; shared AuditState/catalog/record bytes are not attributed. The machine and the commit are recorded in docs/measurements/mc.md."
    );

    let tick_path = server.tick_log.clone();
    let order_path = order_log.clone();
    let evidence_dir = server.dir.clone();
    drop(server);
    let positions = mc::read_positions(&tick_path);
    let transitions = mc::read_transitions(&order_path);
    let troop_ids: std::collections::BTreeSet<_> = (1..=40).collect();
    mc::assert_actual_tick_coverage(&positions, 420, &selected_ids);
    mc::assert_full_tick_coverage(&positions, 400, &troop_ids);
    assert!(
        transitions
            .iter()
            .any(|row| troop_ids.contains(&row.unit_id) && row.state == "Moving"),
        "EL1 verifies the GM40/GMA command script actually changed troop order state"
    );
    eprintln!(
        "EL1 retained server process evidence directory: {}",
        evidence_dir.display()
    );
}

fn assert_economy_tick_coverage(
    tick_records: &[(u32, &turts_sim::AuditState, &turts_sim::AuditState)],
    configured_end: u32,
    selected_ids: &std::collections::BTreeSet<u32>,
) -> u32 {
    let actual_end = tick_records.last().map_or(0, |(tick, _, _)| *tick);
    assert!(
        actual_end >= configured_end,
        "EL1 economy audit ends at tick {actual_end}, before configured --run-ticks {configured_end}"
    );
    assert_eq!(
        tick_records.len(),
        actual_end as usize,
        "EL1 economy audit has a contiguous 1..=actual-end tick sequence"
    );
    for (expected, (actual, before, state)) in (1..=actual_end).zip(tick_records) {
        assert_eq!(
            *actual, expected,
            "EL1 economy audit tick sequence is contiguous"
        );
        for (label, audit) in [("before", *before), ("state", *state)] {
            let actual_units: std::collections::BTreeSet<_> =
                audit.raw_units.iter().map(|unit| unit.unit_id).collect();
            assert!(
                selected_ids.is_subset(&actual_units),
                "EL1 tick {actual} {label} audit omits script units: {:?}",
                selected_ids.difference(&actual_units).collect::<Vec<_>>()
            );
        }
    }
    actual_end
}

fn movement_state_bytes(records: &[turts_audit::EconomyLogRecord]) -> (usize, usize, usize, usize) {
    let mut unit_state_count = 0;
    let mut unit_bytes = 0;
    let mut slot_count = 0;
    let mut slot_bytes = 0;
    for record in records {
        let turts_audit::EconomyLogRecord::Tick { before, state, .. } = record else {
            continue;
        };
        for audit in [before, state] {
            let mut bytes_by_unit = std::collections::BTreeMap::<u32, usize>::new();
            for value in &audit.movement_progress {
                *bytes_by_unit.entry(value.unit_id).or_default() += serde_json::to_vec(value)
                    .expect("movement progress JSON encodes")
                    .len();
            }
            for value in &audit.plain_movement_progress {
                *bytes_by_unit.entry(value.unit_id).or_default() += serde_json::to_vec(value)
                    .expect("plain movement progress JSON encodes")
                    .len();
            }
            for value in &audit.physical_escapes {
                *bytes_by_unit.entry(value.unit_id).or_default() += serde_json::to_vec(value)
                    .expect("physical escape JSON encodes")
                    .len();
            }
            for value in &audit.unit_orders {
                *bytes_by_unit.entry(value.unit_id).or_default() += serde_json::to_vec(value)
                    .expect("unit order state JSON encodes")
                    .len();
            }
            unit_state_count += bytes_by_unit.len();
            unit_bytes += bytes_by_unit.values().sum::<usize>();
            for pool in &audit.movement_pools {
                for slot in &pool.slots {
                    slot_count += 1;
                    slot_bytes += serde_json::to_vec(slot)
                        .expect("actual pool slot JSON encodes")
                        .len();
                }
            }
        }
    }
    (unit_state_count, unit_bytes, slot_count, slot_bytes)
}
