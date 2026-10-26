//! Presentation record.
//!
//! Each frame the client works out where every unit is shown: presentation time `t` from the
//! `Replica`, then the position between two consecutive samples (design section 5). The result
//! is the `Presented` resource. The frame log, the debug JSON and the renderer read it. It never
//! writes to the `Replica`.

use bevy::platform::time::Instant;
use bevy::prelude::*;
use turts_net_protocol::Pos;

use crate::ingest::ReplicaState;

/// One unit as shown in one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentedUnit {
    pub unit_id: u32,
    pub content_id: String,
    pub pos: Pos,
    pub lo_tick: u32,
    pub hi_tick: u32,
    pub fraction: f32,
}

/// What the last frame showed. `units` is empty before the first `Snapshot`.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct Presented {
    /// Wall time of the frame in nanoseconds since the UNIX epoch. This is the clock of the
    /// server tick log.
    pub frame_ns: u128,
    /// Presentation time in ticks.
    pub t: f64,
    pub latest_tick: u32,
    /// Ascending by `unit_id`.
    pub units: Vec<PresentedUnit>,
}

/// The wall clock in nanoseconds since the UNIX epoch.
#[cfg(not(target_family = "wasm"))]
pub fn frame_ns() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos())
}

/// `(performance.timeOrigin + performance.now()) * 1e6`: the wall clock with the resolution of
/// the performance timer. It falls back to `Date.now()`.
#[cfg(target_family = "wasm")]
pub fn frame_ns() -> u128 {
    use js_sys::{Function, Reflect};
    let performance_ms = || -> Option<f64> {
        let performance = Reflect::get(&js_sys::global(), &"performance".into()).ok()?;
        let origin = Reflect::get(&performance, &"timeOrigin".into())
            .ok()?
            .as_f64()?;
        let now = Function::from(Reflect::get(&performance, &"now".into()).ok()?)
            .call0(&performance)
            .ok()?
            .as_f64()?;
        Some(origin + now)
    };
    let ms = performance_ms().unwrap_or_else(js_sys::Date::now);
    (ms * 1e6) as u128
}

/// `Update`: record where each unit is shown in this frame.
pub fn update_presented(state: Res<ReplicaState>, mut presented: ResMut<Presented>) {
    let now = Instant::now();
    presented.frame_ns = frame_ns();
    presented.units.clear();
    let Some(replica) = state.replica.as_ref() else {
        return;
    };
    let (Some(t), Some(latest_tick)) = (replica.presentation_time(now), replica.last_applied_tick)
    else {
        return;
    };
    presented.t = t;
    presented.latest_tick = latest_tick;
    let mut ids: Vec<u32> = replica.units.keys().copied().collect();
    ids.sort_unstable();
    for unit_id in ids {
        let Some(shown) = replica.sample(unit_id, t) else {
            continue;
        };
        presented.units.push(PresentedUnit {
            unit_id,
            content_id: replica.units[&unit_id].content_id.clone(),
            pos: shown.pos,
            lo_tick: shown.lo_tick,
            hi_tick: shown.hi_tick,
            fraction: shown.fraction,
        });
    }
}
