//! Thin wasm32 entry for the browser client.
//!
//! All client logic lives in `turts_client`. This crate reads the page query, builds the app,
//! and installs three test hooks on `window`:
//!
//! - `window.turts_debug()` returns the debug JSON of `turts_client::debug::debug_json`. It
//!   returns an empty string until the first frame has run.
//! - `window.turts_move(x, y)` queues a move order for the units of this player. The order is
//!   sent only when the client is joined.
//! - `window.turts_memory_bytes()` returns the size of the wasm linear memory in bytes. Wasm
//!   memory never shrinks, so the last value that a run reads is its high-water mark
//!   (`tests/browser/measure.js`, design 4.5).
//! - `window.turts_scene()` returns the rendered scene as JSON:
//!   `{"cameras_3d": 1, "units": [{"unit_id": 1, "translation": [x, y, z]}]}`. `cameras_3d` is
//!   the number of active 3D cameras (M1 verify): a scene with meshes and no camera shows
//!   nothing. `units` has one entry for each entity that has a mesh and a unit id
//!   (`turts_client::render::PresentedUnit`), with its world translation, ascending by unit id.
//!   A map `(x, y)` is the scene point `(x, lift, y)`. It returns an empty string until the
//!   first frame has run. T7 reads it (M1 verify): the browser "presents" a unit only when the
//!   scene holds it and a camera can see it.
//!
//! The page query is `?server=<http url>&ws=<ws url>`. When it is wrong, the page gets the
//! message in `window.turts_error` and the app does not start.

use std::sync::Mutex;

use bevy::prelude::*;
use turts_client::build_app;
use turts_client::debug::{DebugState, rebuild_debug_state};
use turts_client::input::PendingMoves;
use turts_client::options::options_from_query;
use turts_client::render::PresentedUnit as SceneUnit;
use turts_net_protocol::Pos;
use wasm_bindgen::prelude::*;

/// The last debug JSON. The `window.turts_debug` closure reads it.
static DEBUG: Mutex<String> = Mutex::new(String::new());

/// The last scene JSON. The `window.turts_scene` closure reads it.
static SCENE: Mutex<String> = Mutex::new(String::new());

/// The targets from `window.turts_move` that no frame has taken yet.
static MOVES: Mutex<Vec<(f32, f32)>> = Mutex::new(Vec::new());

#[wasm_bindgen(start)]
pub fn start() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let query = window.location().search().unwrap_or_default();
    let options = match options_from_query(&query) {
        Ok(options) => options,
        Err(message) => {
            set_global(&window, "turts_error", &JsValue::from_str(&message));
            return;
        }
    };
    let debug = Closure::<dyn Fn() -> String>::new(|| DEBUG.lock().unwrap().clone());
    set_global(&window, "turts_debug", debug.as_ref());
    debug.forget();
    let move_to = Closure::<dyn Fn(f32, f32)>::new(|x, y| MOVES.lock().unwrap().push((x, y)));
    set_global(&window, "turts_move", move_to.as_ref());
    move_to.forget();
    let scene = Closure::<dyn Fn() -> String>::new(|| SCENE.lock().unwrap().clone());
    set_global(&window, "turts_scene", scene.as_ref());
    scene.forget();
    let memory = Closure::<dyn Fn() -> f64>::new(memory_bytes);
    set_global(&window, "turts_memory_bytes", memory.as_ref());
    memory.forget();

    let mut app = build_app(options);
    app.add_systems(Update, drain_moves).add_systems(
        Last,
        (publish_debug.after(rebuild_debug_state), publish_scene),
    );
    app.run();
}

fn set_global(window: &web_sys::Window, name: &str, value: &JsValue) {
    // `Reflect::set` on a window object does not fail. A failure would only lose a test hook.
    let _ = js_sys::Reflect::set(window, &JsValue::from_str(name), value);
}

/// The size of the wasm linear memory now: `memory_size(0) * 65536` (one page is 64 KiB).
#[cfg(target_family = "wasm")]
fn memory_bytes() -> f64 {
    core::arch::wasm32::memory_size::<0>() as f64 * 65_536.0
}

/// A native build has no wasm memory, and `start` never runs there. The function keeps the
/// native `cargo clippy --workspace --all-targets` build whole.
#[cfg(not(target_family = "wasm"))]
fn memory_bytes() -> f64 {
    0.0
}

/// `Update`: move the targets of `window.turts_move` into the input queue.
fn drain_moves(mut pending: ResMut<PendingMoves>) {
    let mut moves = MOVES.lock().unwrap();
    if moves.is_empty() {
        return;
    }
    pending.0.extend(moves.drain(..).map(|(x, y)| Pos { x, y }));
}

/// `Last`: copy the debug JSON, which `rebuild_debug_state` made this frame, for the page.
fn publish_debug(state: Res<DebugState>) {
    DEBUG.lock().unwrap().clone_from(&state.0);
}

/// `Last`: list the rendered units and count the active 3D cameras for the page.
/// `GlobalTransform` is up to date here: the transform propagation runs in `PostUpdate`.
fn publish_scene(
    units: Query<(&SceneUnit, &GlobalTransform), With<Mesh3d>>,
    cameras: Query<&Camera, With<Camera3d>>,
) {
    let mut entries: Vec<(u32, Vec3)> = units
        .iter()
        .map(|(unit, transform)| (unit.0, transform.translation()))
        .collect();
    entries.sort_unstable_by_key(|(unit_id, _)| *unit_id);
    let active_cameras = cameras.iter().filter(|camera| camera.is_active).count();
    *SCENE.lock().unwrap() = scene_json(active_cameras, &entries);
}

/// The JSON of `window.turts_scene`. A non-finite coordinate is written as `null`, so the page
/// can always parse the text and the check fails on the value.
fn scene_json(cameras_3d: usize, entries: &[(u32, Vec3)]) -> String {
    let number = |value: f32| {
        if value.is_finite() {
            value.to_string()
        } else {
            "null".to_string()
        }
    };
    let items: Vec<String> = entries
        .iter()
        .map(|(unit_id, at)| {
            format!(
                "{{\"unit_id\":{unit_id},\"translation\":[{},{},{}]}}",
                number(at.x),
                number(at.y),
                number(at.z)
            )
        })
        .collect();
    format!(
        "{{\"cameras_3d\":{cameras_3d},\"units\":[{}]}}",
        items.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_json_lists_units_with_their_translation() {
        assert_eq!(scene_json(0, &[]), r#"{"cameras_3d":0,"units":[]}"#);
        assert_eq!(
            scene_json(
                1,
                &[
                    (1, Vec3::new(20.0, 0.9, 8.5)),
                    (2, Vec3::new(f32::NAN, 0.0, 1.0))
                ]
            ),
            concat!(
                r#"{"cameras_3d":1,"units":["#,
                r#"{"unit_id":1,"translation":[20,0.9,8.5]},"#,
                r#"{"unit_id":2,"translation":[null,0,1]}]}"#
            )
        );
    }

    #[test]
    fn scene_json_reports_the_camera_count() {
        // M1 verify: T7 asserts exactly one active 3D camera, so the count must reach the page.
        assert!(scene_json(2, &[]).starts_with(r#"{"cameras_3d":2,"#));
    }
}
