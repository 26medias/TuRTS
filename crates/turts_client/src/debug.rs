//! Debug hooks.
//!
//! `debug_json` describes the client for a test: the phase, the presented units with the shape
//! from the downloaded visuals, and the last frame. The browser page asks for it through
//! `window.turts_debug_request` and reads it through `window.turts_debug` (Task 11, L0-DBG). The
//! client builds it only for a request ([`DebugRequest`]). The `--out` report embeds it.

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

/// The debug JSON of the last build. It is empty until the first request is served.
#[derive(Resource, Default)]
pub struct DebugState(pub String);

/// The least time between two builds of the debug state: 4 builds each second (design D 9,
/// row DBG1).
pub const DEBUG_MIN_INTERVAL_SECS: f64 = 0.25;

/// The gate of the debug state (design D 9, row DBG1). The client builds [`DebugState`] only
/// when a harness has asked for it, at most [`DEBUG_MIN_INTERVAL_SECS`] apart, for the last
/// presented frame. The browser bridge calls [`DebugRequest::request`] (through
/// `window.turts_debug_request`); a native test calls it directly. The counter `builds` is
/// part of the published record (`debug_builds`), so a test can count the builds.
#[derive(Resource, Debug, Default)]
pub struct DebugRequest {
    /// The least interval of the requests that wait for a build, in seconds.
    pending: Option<f64>,
    /// The `Time<Real>` of the last build, when a clock exists.
    last_build_secs: Option<f64>,
    builds: u64,
}

impl DebugRequest {
    /// Ask for one build of the debug state. `min_interval_secs` is the least time since the
    /// last build; `None` gives [`DEBUG_MIN_INTERVAL_SECS`].
    ///
    /// Ruling (orchestrator, 2026-11-12, row DBG1): the 4 Hz cap stays for every default
    /// request. `min_interval_ms` 0 is a legal test-only form for the SH1 per-frame phase
    /// capture only (`tests/browser/harness/sh1-lifecycle.js`); a grep test
    /// (`tests/browser/harness/debug-request-use.test.js`) fails when any other caller passes a
    /// value.
    pub fn request(&mut self, min_interval_secs: Option<f64>) {
        let wanted = min_interval_secs
            .filter(|secs| secs.is_finite())
            .unwrap_or(DEBUG_MIN_INTERVAL_SECS)
            .max(0.0);
        self.pending = Some(self.pending.map_or(wanted, |pending| pending.min(wanted)));
    }

    /// How many times the client has built the debug state.
    pub fn builds(&self) -> u64 {
        self.builds
    }

    /// Whether a request waits for a build.
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Whether a build is due now: a request waits and the interval has passed. With no clock
    /// (a bare test world) a waiting request is always due.
    fn is_due(&self, now_secs: Option<f64>) -> bool {
        let Some(wanted) = self.pending else {
            return false;
        };
        match (self.last_build_secs, now_secs) {
            (Some(last), Some(now)) => now - last >= wanted,
            _ => true,
        }
    }

    fn mark_built(&mut self, now_secs: Option<f64>) {
        self.builds += 1;
        self.pending = None;
        self.last_build_secs = now_secs;
    }
}

/// The run condition of [`rebuild_debug_state`]: a request waits and the 4 Hz cap allows it.
pub fn debug_build_due(request: Res<DebugRequest>, time: Option<Res<Time<Real>>>) -> bool {
    request.is_due(time.map(|time| time.elapsed_secs_f64()))
}

/// The state of the client as JSON:
/// `{ phase, units: [{ unit_id, content_id, pos: [x, y], shape }], last_frame }`.
/// `shape` is the serde form of the visual (`{"Capsule": {..}}`), or `null` when the client
/// has no visual for the unit. `last_frame` is the first unit of the last frame, or `null`.
pub fn debug_json(world: &World) -> String {
    let phase = world
        .get_resource::<Phase>()
        .map_or_else(|| "unknown".to_string(), |phase| format!("{phase:?}"));
    let tick_hz = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.replica.as_ref().map(|replica| replica.tick_hz));
    let match_id_decimal = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.welcome.map(|welcome| welcome.match_id.to_string()));
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
        .get_resource::<crate::ui_shared::Selection>()
        .and_then(|selection| selection.0);
    let selected_units = world
        .get_resource::<crate::debug_feed::SelectionFeed>()
        .map(|feed| feed.0.clone())
        .unwrap_or_default();
    let pending_order_id = world
        .get_resource::<crate::core_adapter::CoreRuntime>()
        .and_then(|runtime| runtime.pending_order_id());
    let groups = world
        .get_resource::<crate::debug_feed::GroupsFeed>()
        .map(|feed| {
            feed.0
                .iter()
                .map(|(group, units)| (group.to_string(), json!(units)))
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_default();
    let frames = world
        .get_resource::<crate::core_adapter::FrameFeeds>()
        .map(|feeds| json!({"recent":feeds.recent,"retained_input":feeds.retained_input}))
        .unwrap_or_else(|| json!({"recent":[],"retained_input":[]}));
    let results = world
        .get_resource::<crate::debug_feed::ResultsFeed>()
        .map(|feed| feed.0.clone())
        .unwrap_or_default();
    let cursor = world
        .get_resource::<crate::debug_feed::CursorFeed>()
        .map_or_else(
            || json!(crate::debug_feed::CursorFeed::default()),
            |feed| json!(feed),
        );
    let ui_snapshot = world
        .get_resource::<crate::debug_feed::UiSnapshotFeed>()
        .map_or(Value::Null, |feed| feed.0.clone());
    let notices = world
        .get_resource::<crate::debug_feed::NoticesFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let alerts = world
        .get_resource::<crate::debug_feed::AlertsFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let audio = world
        .get_resource::<crate::debug_feed::AudioFeed>()
        .map_or_else(
            || json!(crate::debug_feed::AudioFeed::default()),
            |feed| json!(feed),
        );
    let context_menu = world
        .get_resource::<crate::debug_feed::ContextMenuFeed>()
        .map_or_else(
            || json!(crate::debug_feed::ContextMenuFeed::default()),
            |feed| json!(feed),
        );
    let focus = world
        .get_resource::<crate::debug_feed::FocusFeed>()
        .is_some_and(|feed| feed.0);
    let player_id = world
        .get_resource::<crate::debug_feed::PlayerIdFeed>()
        .and_then(|feed| feed.0);
    let player_id_decimal = world
        .get_resource::<crate::PlayerId>()
        .map(|player_id| player_id.0.to_string());
    let alert_pings = world
        .get_resource::<crate::debug_feed::AlertPingsFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let markers_shown = world
        .get_resource::<crate::debug_feed::MarkersShown>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let rings_shown = world
        .get_resource::<crate::debug_feed::RingsShown>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let effective_bindings = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.content.as_ref())
        .map(|content| {
            let runtime = world
                .get_resource::<crate::core_adapter::CorePreferences>()
                .map(|preferences| preferences.runtime_bindings.clone())
                .unwrap_or_default();
            binding_observation(content, &runtime)
        })
        .unwrap_or_else(|| json!({"actions":{},"orders":{}}));
    let verified_root_mod = visuals
        .and_then(|acquired| acquired.0.verified_root_mod.as_ref())
        .map(|root| json!({"id":root.id,"version":root.version}));
    let replicated_world = world
        .get_resource::<ReplicaState>()
        .and_then(|state| {
            state.replica.as_ref().map(|replica| {
                let content = state.content.as_ref();
                let unit_orders = unit_orders_json(&replica.unit_orders);
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
                            "max_health": unit_max_health(content, &unit.content_id),
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
                    "unit_orders": unit_orders,
                    "research": replica.research,
                    "health_trace": health_trace_json(&replica.health_trace),
                })
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
    let rendered_ui_nodes = world
        .get_resource::<crate::debug_feed::UiSnapshotFeed>()
        .and_then(|feed| feed.0.as_array())
        .map_or(0, Vec::len);
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
    let rejoin = world
        .get_resource::<crate::rejoin::RejoinController>()
        .map(|state| {
            json!({
                "stored_identity": state.stored_identity(),
                "elapsed_secs": state.elapsed_secs(),
                "first_transport_attempt_real_secs": state.first_attempt_real_secs(),
                "terminal_at_real_secs": state.terminal_at_real_secs(),
                "terminal_elapsed_secs": state.terminal_elapsed_secs(),
                "last_pending_deadline_check": state.last_pending_deadline_check().map(|check| json!({
                    "check_index": check.check_index,
                    "frame_index": check.frame_index,
                    "source": check.source,
                    "real_now_secs": check.real_now_secs,
                    "elapsed_secs": check.elapsed_secs,
                })),
                "first_terminal_deadline_check": state.first_terminal_deadline_check().map(|check| json!({
                    "check_index": check.check_index,
                    "frame_index": check.frame_index,
                    "source": check.source,
                    "real_now_secs": check.real_now_secs,
                    "elapsed_secs": check.elapsed_secs,
                })),
                "deadline_checks_causal": state.deadline_checks_are_causal(),
                "attempts_started": state.attempts_started(),
                "hellos_sent": state.hellos_sent(),
                "duplicate_rejections": state.duplicate_rejections(),
                "hello_elapsed_secs": state.hello_elapsed_secs(),
                "current_attempt_present": state.current_attempt().is_some(),
                "retry_ready": state.retry_ready(),
                "text": state.visible_text(),
            })
        })
        .unwrap_or(Value::Null);
    json!({
        "debug_builds": world.get_resource::<DebugRequest>().map_or(0, DebugRequest::builds),
        "phase": phase,
        "rejoin": rejoin,
        "tick_hz": tick_hz,
        "match_id": world.get_resource::<ReplicaState>().and_then(|state| state.welcome.map(|welcome| welcome.match_id)),
        "match_id_decimal": match_id_decimal,
        "slot": world.get_resource::<ReplicaState>().and_then(|state| state.welcome.map(|welcome| welcome.slot)),
        "units": units,
        "last_frame": last_frame,
        "trace": trace,
        "dropped_world_changes": drops,
        "reconnect_required": world.get_resource::<CommandSendStatus>().is_some_and(|status| status.reconnect_required),
        "command_results": command_results,
        "research_completed_tick": world.get_resource::<crate::scenario::ProduceHook>().and_then(|hook| hook.research_completed_tick),
        "selection": selected_units,
        "pending_order_id": pending_order_id,
        "ui_selection": selection,
        "groups": groups,
        "frames": frames,
        "results": results,
        "cursor": cursor,
        "ui_snapshot": ui_snapshot,
        "notices": notices,
        "alerts": alerts,
        "audio": audio,
        "contextmenu": context_menu,
        "focus": focus,
        "player_id": player_id,
        "player_id_decimal": player_id_decimal,
        "alert_pings": alert_pings,
        "markers_shown": markers_shown,
        "rings_shown": rings_shown,
        "effective_bindings": effective_bindings,
        "verified_root_mod": verified_root_mod,
        "rendered_ui_nodes": rendered_ui_nodes,
        "ingest": ingest,
        "replicated_world": replicated_world,
    }).to_string()
}

fn binding_observation(
    content: &crate::content::ClientContent,
    runtime: &BTreeMap<String, Vec<turts_mod_api::Binding>>,
) -> Value {
    fn action_map(
        actions: &BTreeMap<String, Vec<turts_mod_api::Binding>>,
    ) -> serde_json::Map<String, Value> {
        actions
            .iter()
            .map(|(name, bindings)| (name.clone(), json!(bindings)))
            .collect()
    }
    fn order_map(
        content: &crate::content::ClientContent,
        actions: &BTreeMap<String, Vec<turts_mod_api::Binding>>,
    ) -> serde_json::Map<String, Value> {
        content
            .catalog
            .orders
            .iter()
            .filter_map(|(id, order)| {
                actions
                    .get(&order.hotkey)
                    .map(|bindings| (id.to_string(), json!(bindings)))
            })
            .collect()
    }
    let defaults = turts_interaction::input_map::effective_actions_with_runtime(
        &content.input_map,
        &BTreeMap::new(),
    );
    let effective =
        turts_interaction::input_map::effective_actions_with_runtime(&content.input_map, runtime);
    json!({
        "defaults": {"actions":action_map(&defaults),"orders":order_map(content, &defaults)},
        "runtime_overrides": runtime,
        "actions": action_map(&effective),
        "orders": order_map(content, &effective),
    })
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

/// `Last`: rebuild [`DebugState`]. The app runs it only when [`debug_build_due`] says so. It
/// counts the build first, so the record that it writes holds its own build number.
pub fn rebuild_debug_state(world: &mut World) {
    let now = world
        .get_resource::<Time<Real>>()
        .map(|time| time.elapsed_secs_f64());
    if let Some(mut request) = world.get_resource_mut::<DebugRequest>() {
        request.mark_built(now);
    }
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
    let player_id_decimal = world
        .get_resource::<crate::PlayerId>()
        .map(|player_id| player_id.0.to_string());
    let match_id_decimal = state.welcome.map(|welcome| welcome.match_id.to_string());
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
        let unit_orders = unit_orders_json(&replica.unit_orders);
        let units = replica.units.iter().map(|(&id, unit)| {
            let pos = unit.samples.back().map(|(_, pos)| [pos.x, pos.y]);
            json!({ "unit_id": id, "content_id": unit.content_id, "owner_slot": unit.owner_slot, "facing": unit.facing, "pos": pos, "max_health": unit_max_health(state.content.as_ref(), &unit.content_id) })
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
            "unit_orders": unit_orders,
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
    let selection = world
        .get_resource::<crate::debug_feed::SelectionFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let groups = world
        .get_resource::<crate::debug_feed::GroupsFeed>()
        .map(|feed| {
            feed.0
                .iter()
                .map(|(group, units)| (group.to_string(), json!(units)))
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_default();
    let subject_results = world
        .get_resource::<crate::debug_feed::ResultsFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let debug = serde_json::from_str(&debug_json(world)).unwrap_or(Value::Null);
    json!({
        "welcomed": state.welcome.is_some(),
        "tick_hz": state.replica.as_ref().map(|replica| replica.tick_hz),
        "player_id_decimal": player_id_decimal,
        "match_id": state.welcome.map(|welcome| welcome.match_id),
        "match_id_decimal": match_id_decimal,
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
        "selection": selection,
        "groups": groups,
        "results": subject_results,
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

fn unit_orders_json(
    records: &std::collections::HashMap<u32, turts_net_protocol::UnitOrderRecord>,
) -> Vec<Value> {
    let mut records: Vec<_> = records.values().cloned().collect();
    records.sort_by_key(|record| record.unit.0);
    records.into_iter().map(|record| json!(record)).collect()
}

fn unit_max_health(
    content: Option<&crate::content::ClientContent>,
    content_id: &str,
) -> Option<u64> {
    let content_id = turts_mod_api::ContentId::parse(content_id).ok()?;
    content?
        .catalog
        .units
        .get(&content_id)?
        .health
        .map(|health| health.max)
}

#[cfg(test)]
mod unit_order_debug_tests {
    use super::{DebugState, debug_json, rebuild_debug_state, unit_orders_json};
    use crate::ingest::{ReplicaState, WelcomeInfo};
    use crate::replica::Replica;
    use std::collections::HashMap;
    use turts_net_protocol::{Activity, UnitId, UnitOrderRecord};

    #[test]
    fn capture_and_report_share_sorted_received_owner_order_records() {
        let records = HashMap::from([
            (
                9,
                UnitOrderRecord {
                    unit: UnitId(9),
                    activity: Activity::Moving,
                    current: None,
                    queued: Vec::new(),
                    last_failure: None,
                },
            ),
            (
                2,
                UnitOrderRecord {
                    unit: UnitId(2),
                    activity: Activity::Idle,
                    current: None,
                    queued: Vec::new(),
                    last_failure: None,
                },
            ),
        ]);

        let capture = unit_orders_json(&records);
        let report = unit_orders_json(&records);
        assert_eq!(capture, report);
        assert_eq!(capture.len(), 2);
        assert_eq!(capture[0]["unit"], 2);
        assert_eq!(capture[1]["unit"], 9);
        assert_eq!(capture[0]["activity"], "Idle");
        assert_eq!(capture[1]["activity"], "Moving");
    }

    #[test]
    fn debug_capture_and_rebuild_preserve_the_same_received_owner_records() {
        let mut replica = Replica::new(20);
        replica.unit_orders.insert(
            8,
            UnitOrderRecord {
                unit: UnitId(8),
                activity: Activity::Moving,
                current: None,
                queued: Vec::new(),
                last_failure: None,
            },
        );
        let state = ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 1,
                tick: 8,
                tick_hz: 20,
                match_id: 99,
            }),
            ..ReplicaState::default()
        };
        let mut world = bevy::prelude::World::new();
        world.insert_resource(state);
        world.insert_resource(DebugState::default());

        let captured: serde_json::Value = serde_json::from_str(&debug_json(&world)).unwrap();
        rebuild_debug_state(&mut world);
        let rebuilt: serde_json::Value =
            serde_json::from_str(&world.resource::<DebugState>().0).unwrap();
        assert_eq!(
            captured["replicated_world"]["unit_orders"],
            rebuilt["replicated_world"]["unit_orders"]
        );
        assert_eq!(
            captured["tick_hz"], 20,
            "debug exposes the replica rate as a top-level observation"
        );
        assert_eq!(
            rebuilt["tick_hz"], 20,
            "rebuild publishes the same top-level rate"
        );
        assert!(
            captured["replicated_world"].get("tick_hz").is_none(),
            "observation-only tick rate does not change the replicated-world DTO"
        );
        assert_eq!(
            rebuilt["replicated_world"]["unit_orders"][0]["unit"], 8,
            "debug captures expose only the actual Replica owner-specific DTO"
        );
        assert_eq!(
            captured["slot"], 1,
            "debug captures expose the actual Welcome slot"
        );
        assert_eq!(
            rebuilt["slot"], 1,
            "debug rebuild preserves the actual Welcome slot"
        );
    }
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
    use super::{command_result_json, debug_json, rebuild_debug_state, report_json};
    use crate::{
        ClientOpts, PlayerId,
        ingest::{CommandResult, CommandResults, IngestCounters, ReplicaState},
        join::{JoinState, Phase},
        options::ClientOptions,
    };
    use turts_net_protocol::{CommandRejectReason, UnitId, UnitRefusal};

    #[test]
    fn cursor_audio_and_contextmenu_are_typed_objects_with_truthful_empty_defaults() {
        let mut world = bevy::prelude::World::new();
        world.insert_resource(crate::debug_feed::CursorFeed::default());
        world.insert_resource(crate::debug_feed::AudioFeed::default());
        world.insert_resource(crate::debug_feed::ContextMenuFeed::default());
        let debug: serde_json::Value = serde_json::from_str(&super::debug_json(&world)).unwrap();
        assert_eq!(debug["player_id"], serde_json::Value::Null);
        assert_eq!(debug["player_id_decimal"], serde_json::Value::Null);
        assert_eq!(
            debug["cursor"],
            serde_json::json!({
                "context":null,"image_id":null,"applied_icon":null,"position":null,
                "engine_position":null,"lock_requested":false,"lock_active":null,
                "relative_motion":[0.0,0.0],"frame_index":0
            })
        );
        assert_eq!(
            debug["audio"],
            serde_json::json!({
                "context_state":null,
                "context_states":[],
                "gesture_seen":false,
                "sound_on":false,
                "started":[],
                "categories":{},
                "player_entities":0,
                "active_by_category":{},
                "peak_active_by_category":{},
                "sink_observations":[],
                "current_sinks":[],
                "gain_samples":[],
            })
        );
        assert_eq!(
            debug["contextmenu"],
            serde_json::json!({"count":0,"count_prevented":0})
        );
    }

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

    #[test]
    fn player_and_match_ids_are_lossless_in_capture_and_native_report() {
        let mut world = bevy::prelude::World::new();
        world.insert_resource(PlayerId(u64::MAX));
        let replica_state = ReplicaState {
            welcome: Some(crate::ingest::WelcomeInfo {
                slot: 1,
                tick: 1,
                tick_hz: 20,
                match_id: u64::MAX,
            }),
            ..Default::default()
        };
        world.insert_resource(replica_state);
        let debug: serde_json::Value = serde_json::from_str(&debug_json(&world)).unwrap();
        assert_eq!(debug["player_id"], serde_json::Value::Null);
        assert_eq!(debug["player_id_decimal"], u64::MAX.to_string());
        assert_eq!(debug["match_id_decimal"], u64::MAX.to_string());
        world.insert_resource(super::DebugState::default());
        rebuild_debug_state(&mut world);
        let rebuilt: serde_json::Value =
            serde_json::from_str(&world.resource::<super::DebugState>().0).unwrap();
        assert_eq!(rebuilt["player_id_decimal"], u64::MAX.to_string());
        assert_eq!(rebuilt["match_id_decimal"], u64::MAX.to_string());

        world.insert_resource(IngestCounters::default());
        world.insert_resource(CommandResults::default());
        world.insert_resource(Phase::Connecting);
        world.insert_resource(JoinState::default());
        world.insert_resource(ClientOpts(ClientOptions::default()));
        let report = report_json(&world);
        assert_eq!(report["player_id_decimal"], u64::MAX.to_string());
        assert_eq!(report["match_id_decimal"], u64::MAX.to_string());
        assert_eq!(report["debug"]["player_id_decimal"], u64::MAX.to_string());
        assert_eq!(report["debug"]["match_id_decimal"], u64::MAX.to_string());
        assert!(
            report["replicated_world"]
                .get("player_id_decimal")
                .is_none()
        );
    }
}
