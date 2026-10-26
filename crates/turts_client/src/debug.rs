//! Debug hooks.
//!
//! `debug_json` describes the client for a test: the phase, the presented units with the shape
//! from the downloaded visuals, and the last frame. The browser page reads it through
//! `window.turts_debug` (Task 11). The `--out` report embeds it.

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::ClientOpts;
use crate::framelog::{FrameLog, summary_line};
use crate::ingest::{CommandResult, CommandResults, IngestCounters, ReplicaState};
use crate::join::{AcquiredPackages, JoinState, Phase};
use crate::present::Presented;

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
    json!({ "phase": phase, "units": units, "last_frame": last_frame }).to_string()
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
    let command_results: Vec<Value> = results
        .0
        .iter()
        .map(|result| match result {
            CommandResult::Acked { seq, tick } => json!({ "seq": seq, "ack_tick": tick }),
            CommandResult::Rejected { seq, reason } => json!({ "seq": seq, "rejected": reason }),
        })
        .collect();
    let debug = serde_json::from_str(&debug_json(world)).unwrap_or(Value::Null);
    json!({
        "welcomed": state.welcome.is_some(),
        "rejected_reason": rejected_reason,
        "slot": state.welcome.map(|welcome| welcome.slot),
        "disconnected_after_connect": world.resource::<JoinState>().disconnected_after_connect,
        "resync_requests_sent": counters.resync_requests_sent,
        "deltas_dropped": counters.deltas_dropped,
        "command_results": command_results,
        "final_positions": final_positions,
        "debug": debug,
    })
}

/// `Last`: after `--run-secs`, write the `--out` report and the frame log summary, then exit.
/// `App::run` hands the world to the runner, so the files are written before the exit message.
pub fn exit_when_done(world: &mut World) {
    let Some(limit) = world.resource::<ClientOpts>().0.run_secs else {
        return;
    };
    if world.resource::<Time<Real>>().elapsed_secs() < limit {
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
    world.write_message(AppExit::Success);
}
