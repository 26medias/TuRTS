//! Debug hooks.
//!
//! `debug_json` describes the client for a test: the phase, the presented units with the shape
//! from the downloaded visuals, and the last frame. The browser page reads it through
//! `window.turts_debug` (Task 11). The `--out` report embeds it.

use std::collections::BTreeMap;
use std::num::NonZeroU8;

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::ClientOpts;
use crate::commands::CommandSendStatus;
use crate::framelog::{FrameLog, summary_line};
use crate::ingest::{
    CommandResult, CommandResults, IngestCounters, ReceiptKind, ReceiptWitnesses, ReplicaState,
};
use crate::join::{AcquiredPackages, JoinState, Phase};
use crate::present::Presented;
use crate::scenario::ScenarioState;

/// The debug JSON, rebuilt each frame.
#[derive(Resource, Default)]
pub struct DebugState(pub String);

/// The state of the client as JSON:
/// `{ phase, units: [{ unit_id, content_id, pos: [x, y], shape }], last_frame }`.
/// `shape` is the serde form of the visual (`{"Capsule": {..}}`), or `null` when the client
/// has no visual for the unit. `last_frame` is the first unit of the last frame, or `null`.
pub fn debug_json(world: &World) -> String {
    let phase = world
        .get_resource::<Phase>()
        .map_or_else(|| "unknown".to_string(), |phase| format!("{phase:?}"));
    let presented = world.get_resource::<Presented>();
    let visuals = world.get_resource::<AcquiredPackages>();
    let units: Vec<Value> = presented
        .iter()
        .flat_map(|presented| presented.units.iter())
        .map(|unit| {
            let shape = visuals
                .and_then(|acquired| acquired.0.visuals.get(&unit.content_id))
                .map_or(Value::Null, |shape| json!(shape));
            json!({
                "unit_id": unit.unit_id,
                "content_id": unit.content_id,
                "pos": [unit.pos.x, unit.pos.y],
                "shape": shape,
            })
        })
        .collect();
    let last_frame = presented
        .and_then(|presented| {
            presented.units.first().map(|unit| {
                json!({
                    "t": presented.t,
                    "lo_tick": unit.lo_tick,
                    "hi_tick": unit.hi_tick,
                    "fraction": unit.fraction,
                    "latest_tick": presented.latest_tick,
                })
            })
        })
        .unwrap_or(Value::Null);
    let selection = world
        .get_resource::<crate::economy_ui::Selection>()
        .and_then(|selection| selection.0);
    let replicated_world = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.replica.as_ref())
        .map(|replica| {
            let mut units: Vec<_> = replica
                .units
                .iter()
                .map(|(&unit_id, unit)| {
                    json!({
                        "unit_id": unit_id,
                        "content_id": unit.content_id,
                        "owner_slot": unit.owner_slot,
                        "facing": unit.facing,
                        "pos": unit.samples.back().map(|(_, pos)| [pos.x, pos.y]),
                    })
                })
                .collect();
            units.sort_by_key(|unit| unit["unit_id"].as_u64().unwrap_or_default());
            let mut nodes: Vec<_> = replica.nodes.values().map(|node| json!(node)).collect();
            nodes.sort_by_key(|node| node["id"].as_u64().unwrap_or_default());
            json!({
                "tick": replica.last_applied_tick,
                "units": units,
                "nodes": nodes,
                "economy": replica.economy,
                "belts": replica.belts,
                "inserters": replica.inserters,
                "extractors": replica.extractors,
                "weapons": replica.weapons,
                "healths": replica.healths,
                "research": replica.research,
                "health_trace": health_trace_json(&replica.health_trace),
            })
        })
        .unwrap_or(Value::Null);
    let command_results = world
        .get_resource::<CommandResults>()
        .map(|results| {
            results
                .0
                .iter()
                .map(command_result_json)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let rendered_ui_nodes = crate::economy_ui::rendered_ui_snapshot(world).len();
    let ingest = world.get_resource::<crate::ingest::IngestHooks>().map(|hooks| {
        let replica = world.get_resource::<ReplicaState>().and_then(|state| state.replica.as_ref());
        json!({
            "max_buffered_batches": replica.map(|replica| replica.max_buffered_batches()),
            "buffered_batches": replica.map(|replica| replica.buffered_batches()),
            "resync_active": replica.map(|replica| replica.in_resync()),
            "buffer_overflows": world.get_resource::<IngestCounters>().map(|counters| counters.buffer_overflows),
            "resync_requests_sent": world.get_resource::<IngestCounters>().map(|counters| counters.resync_requests_sent),
            "successor_requests": world.get_resource::<IngestCounters>().map(|counters| counters.successor_requests),
            "max_outstanding_requests": world.get_resource::<IngestCounters>().map(|counters| counters.max_outstanding_requests),
            "hold_resync_snapshots": hooks.hold_resync_snapshots,
            "held_snapshots": hooks.held.len(),
            "outstanding_request": hooks.outstanding_request,
            "highest_delta_tick": hooks.highest_delta_tick,
            "armed_drop_world_changes": hooks.armed_drop_world_changes.as_ref().map(|armed| json!({
                "kinds": armed.requested,
                "arm_tick": armed.arm_tick,
                "baseline_tick": armed.baseline_tick,
            })),
        })
    }).unwrap_or(Value::Null);
    let scenario = world.get_resource::<ScenarioState>();
    let trace = scenario
        .map(|scenario| {
            json!({
                "enabled": scenario.enabled,
                "complete": scenario.is_complete(),
                "error": scenario.error.as_deref(),
                "bindings": scenario.bindings.iter().map(|binding| json!({
                    "step": binding.step_index,
                    "action": &binding.action,
                    "envelope": &binding.envelope,
                    "result": binding.result.as_ref().map(command_result_json),
                    "arm_tick": binding.arm_tick,
                })).collect::<Vec<_>>(),
            })
        })
        .unwrap_or(Value::Null);
    let drops = world
        .get_resource::<crate::ingest::IngestHooks>()
        .map(|hooks| {
            hooks
                .dropped_world_changes
                .iter()
                .map(|record| {
                    json!({
                        "kind": record.kind,
                        "arm_tick": record.arm_tick,
                        "dropped_tick": record.dropped_tick,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "phase": phase,
        "match_id": world.get_resource::<ReplicaState>().and_then(|state| state.welcome.map(|welcome| welcome.match_id)),
        "units": units,
        "last_frame": last_frame,
        "trace": trace,
        "dropped_world_changes": drops,
        "reconnect_required": world.get_resource::<CommandSendStatus>().is_some_and(|status| status.reconnect_required),
        "command_results": command_results,
        "research_completed_tick": world.get_resource::<crate::scenario::ProduceHook>().and_then(|hook| hook.research_completed_tick),
        "selection": selection,
        "rendered_ui_nodes": rendered_ui_nodes,
        "ingest": ingest,
        "replicated_world": replicated_world,
    }).to_string()
}

fn health_trace_json(
    trace: &std::collections::BTreeMap<u32, Vec<crate::replica::HealthTraceEntry>>,
) -> Vec<Value> {
    trace.iter().map(|(&unit_id, entries)| json!({
        "unit_id": unit_id,
        "entries": entries.iter().map(|entry| json!({ "tick": entry.tick, "current": entry.current })).collect::<Vec<_>>(),
    })).collect()
}

fn command_result_json(result: &CommandResult) -> Value {
    match result {
        CommandResult::Acked { seq, tick } => json!({ "seq": seq, "ack_tick": tick }),
        CommandResult::Applied { seq, tick, job_id } => {
            json!({ "seq": seq, "applied_tick": tick, "job_id": job_id.map(|id| id.0) })
        }
        CommandResult::Rejected { seq, tick, reason } => {
            json!({ "seq": seq, "rejected_tick": tick, "rejected": reason.as_str() })
        }
        CommandResult::Order {
            seq,
            tick,
            accepted,
            refused,
            envelope,
            job_id,
        } => {
            let mut value = serde_json::Map::new();
            value.insert("seq".into(), json!(seq));
            if !accepted.is_empty() {
                value.insert("applied_tick".into(), json!(tick));
                value.insert("job_id".into(), json!(job_id.map(|id| id.0)));
            }
            if accepted.is_empty() && (envelope.is_some() || !refused.is_empty()) {
                let reason = envelope.or_else(|| refused.first().map(|refusal| refusal.reason));
                if let Some(reason) = reason {
                    value.insert("rejected_tick".into(), json!(tick));
                    value.insert("rejected".into(), json!(reason.as_str()));
                }
            }
            value.insert(
                "accepted".into(),
                json!(accepted.iter().map(|unit| unit.0).collect::<Vec<_>>()),
            );
            value.insert(
                "refused".into(),
                json!(
                    refused
                        .iter()
                        .map(|refusal| json!({
                            "unit": refusal.unit.0,
                            "reason": refusal.reason.as_str(),
                        }))
                        .collect::<Vec<_>>()
                ),
            );
            Value::Object(value)
        }
    }
}

/// `Last`: rebuild [`DebugState`].
pub fn rebuild_debug_state(world: &mut World) {
    let json = debug_json(world);
    world.resource_mut::<DebugState>().0 = json;
}

/// The `--out` report: what the client saw and did, as JSON.
pub fn report_json(world: &World) -> Value {
    let state = world.resource::<ReplicaState>();
    let counters = world.resource::<IngestCounters>();
    let results = world.resource::<CommandResults>();
    let rejected_reason = match world.resource::<Phase>() {
        Phase::Failed(reason) => Some(reason.clone()),
        _ => None,
    };
    let final_positions: BTreeMap<u32, [f32; 2]> = state
        .replica
        .iter()
        .flat_map(|replica| replica.units.iter())
        .filter_map(|(&id, unit)| {
            let &(_, pos) = unit.samples.back()?;
            Some((id, [pos.x, pos.y]))
        })
        .collect();
    let replicated_world = state.replica.as_ref().map(|replica| {
        let units = replica.units.iter().map(|(&id, unit)| {
            let pos = unit.samples.back().map(|(_, pos)| [pos.x, pos.y]);
            json!({ "unit_id": id, "content_id": unit.content_id, "owner_slot": unit.owner_slot, "facing": unit.facing, "pos": pos })
        }).collect::<Vec<_>>();
        json!({
            "tick": replica.last_applied_tick,
            "units": units,
            "nodes": replica.nodes.values().collect::<Vec<_>>(),
            "economy": replica.economy,
            "belts": replica.belts,
            "inserters": replica.inserters,
            "extractors": replica.extractors,
            "weapons": replica.weapons,
            "healths": replica.healths,
            "research": replica.research,
            "health_trace": health_trace_json(&replica.health_trace),
        })
    }).unwrap_or(Value::Null);
    let receipts = world.get_resource::<ReceiptWitnesses>();
    let receipt_records: Vec<Value> = receipts.into_iter().flat_map(|r| r.records.iter()).map(|w| json!({
        "connection_epoch": w.connection_epoch, "seq": w.seq,
        "kind": match w.kind { ReceiptKind::Ack => "ack", ReceiptKind::Applied => "applied", ReceiptKind::Rejected => "rejected" },
        "reliable": w.reliable, "message_id": w.message_id, "remote_tick": w.remote_tick,
    })).collect();
    let receipt_proofs: Vec<Value> = receipts.into_iter().flat_map(|r| r.records.iter()).filter(|w| w.kind == ReceiptKind::Ack).filter_map(|ack| {
        let ordered = receipts?.ordered_for(ack.seq, ack.connection_epoch)?;
        Some(json!({ "seq": ack.seq, "connection_epoch": ack.connection_epoch, "ack_precedes_rejection": ordered }))
    }).collect();
    let command_results: Vec<Value> = results.0.iter().map(command_result_json).collect();
    let debug = serde_json::from_str(&debug_json(world)).unwrap_or(Value::Null);
    json!({
        "welcomed": state.welcome.is_some(),
        "match_id": state.welcome.map(|welcome| welcome.match_id),
        "rejected_reason": rejected_reason,
        "slot": state.welcome.map(|welcome| welcome.slot),
        "disconnected_after_connect": world.resource::<JoinState>().disconnected_after_connect,
        "resync_requests_sent": counters.resync_requests_sent,
        "resync_buffer_batches": world.resource::<ClientOpts>().0.resync_buffer_batches,
        "max_buffered_batches": counters.max_buffered_batches,
        "max_outstanding_requests": counters.max_outstanding_requests,
        "buffer_overflows": counters.buffer_overflows,
        "successor_requests": counters.successor_requests,
        "deltas_dropped": counters.deltas_dropped,
        "command_results": command_results,
        "trace": world.get_resource::<ScenarioState>().map(|scenario| json!({
            "enabled": scenario.enabled,
            "complete": scenario.is_complete(),
            "error": scenario.error.as_deref(),
            "bindings": scenario.bindings.iter().map(|binding| json!({
                "step": binding.step_index,
                "action": &binding.action,
                "envelope": &binding.envelope,
                "result": binding.result.as_ref().map(command_result_json),
                "arm_tick": binding.arm_tick,
            })).collect::<Vec<_>>(),
        })).unwrap_or(Value::Null),
        "dropped_world_changes": world.get_resource::<crate::ingest::IngestHooks>()
            .map(|hooks| hooks.dropped_world_changes.iter().map(|record| json!({
                "kind": record.kind, "arm_tick": record.arm_tick, "dropped_tick": record.dropped_tick,
            })).collect::<Vec<_>>()).unwrap_or_default(),
        "receipt_witnesses": receipt_records,
        "receipt_order_proofs": receipt_proofs,
        "reconnect_required": world.get_resource::<CommandSendStatus>().is_some_and(|status| status.reconnect_required),
        "final_positions": final_positions,
        "health_trace": state.replica.as_ref().map(|replica| health_trace_json(&replica.health_trace)).unwrap_or_default(),
        "research_completed_tick": world.get_resource::<crate::scenario::ProduceHook>().and_then(|hook| hook.research_completed_tick),
        "replicated_world": replicated_world,
        "snapshots_applied": counters.snapshots_applied,
        "deltas_received": counters.deltas_received,
        "held_snapshots": world.get_resource::<crate::ingest::IngestHooks>().map_or(0, |hooks| hooks.held.len()),
        "debug": debug,
    })
}

/// The reason of a failed join or acquisition, when the phase is `Failed`.
fn failure_reason(world: &World) -> Option<String> {
    match world.resource::<Phase>() {
        Phase::Failed(reason) => Some(reason.clone()),
        _ => None,
    }
}

/// Whether a headless client has nothing more to wait for after a failure. A failure before
/// the connection (the advertisement, the lockfile or a package) is final at once. A failure
/// after the connection (`JoinRejected`) is final when the server has disconnected, so the
/// report holds `disconnected_after_connect` (design section 7, T2d).
fn failure_is_final(world: &World) -> bool {
    let join = world.resource::<JoinState>();
    !join.connected_seen || join.disconnected_after_connect
}

/// `Last`: end a native run, write the `--out` report and the frame log summary, then exit.
/// `App::run` hands the world to the runner, so the files are written before the exit message.
///
/// The run ends when `--run-secs` has passed, or when a headless client has failed (a rejected
/// package or join) and has nothing more to wait for. A failed run prints the reason on stderr
/// and exits with code 1 (M1 verify, B3); any other end is a success. The browser has no
/// `--run-secs`, so a failure there only shows in the debug state.
pub fn exit_when_done(world: &mut World) {
    let (run_secs, headless) = {
        let opts = &world.resource::<ClientOpts>().0;
        (opts.run_secs, opts.headless)
    };
    let failure = failure_reason(world);
    let timed_out =
        run_secs.is_some_and(|limit| world.resource::<Time<Real>>().elapsed_secs() >= limit);
    let failed_for_good = headless && failure.is_some() && failure_is_final(world);
    if !(timed_out || failed_for_good) {
        return;
    }
    if let Some(path) = world.resource::<ClientOpts>().0.out.clone()
        && let Err(error) = std::fs::write(&path, report_json(world).to_string())
    {
        eprintln!("turts_client: cannot write {}: {error}", path.display());
    }
    let summary = {
        let welcome_tick = world
            .resource::<ReplicaState>()
            .welcome
            .map(|welcome| welcome.tick);
        let counters = world.resource::<IngestCounters>();
        summary_line(
            welcome_tick,
            world.resource::<FrameLog>().first_present_tick,
            counters.resync_requests_sent,
            &counters.deltas_dropped,
        )
    };
    world.resource_mut::<FrameLog>().finish(&summary);
    match failure {
        Some(reason) => {
            eprintln!("turts_client: {reason}");
            world.write_message(AppExit::Error(NonZeroU8::MIN));
        }
        None => {
            world.write_message(AppExit::Success);
        }
    }
}

#[cfg(test)]
mod command_result_json_tests {
    use super::command_result_json;
    use crate::ingest::CommandResult;
    use turts_net_protocol::{CommandRejectReason, UnitId, UnitRefusal};

    #[test]
    fn empty_order_rejection_preserves_legacy_debug_keys() {
        let value = command_result_json(&CommandResult::Order {
            seq: 9,
            tick: 41,
            accepted: Vec::new(),
            refused: Vec::new(),
            envelope: Some(CommandRejectReason::EmptyCommand),
            job_id: None,
        });
        assert_eq!(value["seq"], 9);
        assert_eq!(value["rejected_tick"], 41);
        assert_eq!(value["rejected"], "empty_command");
        assert_eq!(value["accepted"], serde_json::json!([]));
        assert_eq!(value["refused"], serde_json::json!([]));
    }

    #[test]
    fn partial_order_acceptance_is_not_reported_as_all_rejected() {
        let value = command_result_json(&CommandResult::Order {
            seq: 10,
            tick: 42,
            accepted: vec![UnitId(1)],
            refused: vec![UnitRefusal {
                unit: UnitId(2),
                reason: CommandRejectReason::NotOwner,
            }],
            envelope: None,
            job_id: None,
        });
        assert_eq!(value["applied_tick"], 42);
        assert!(value.get("rejected_tick").is_none());
        assert!(value.get("rejected").is_none());
        assert_eq!(value["accepted"], serde_json::json!([1]));
        assert_eq!(value["refused"][0]["reason"], "not_owner");
    }
}
