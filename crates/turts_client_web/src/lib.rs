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
use turts_client::commands::{CommandSendStatus, PendingCommands, owned_mobile_ids, submit};
use turts_client::debug::{DebugState, rebuild_debug_state};
use turts_client::economy_ui::rendered_ui_snapshot;
use turts_client::ingest::ReplicaState;
use turts_client::options::options_from_query;
use turts_client::render::{PresentedNode as SceneNode, PresentedUnit as SceneUnit};
use turts_net_protocol::PlayerCommandKind;
use turts_net_protocol::Pos;
use wasm_bindgen::prelude::*;

/// The last debug JSON. The `window.turts_debug` closure reads it.
static DEBUG: Mutex<String> = Mutex::new(String::new());

/// The last scene JSON. The `window.turts_scene` closure reads it.
static SCENE: Mutex<String> = Mutex::new(String::new());
static UI: Mutex<String> = Mutex::new(String::new());

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
    let ui = Closure::<dyn Fn() -> String>::new(|| UI.lock().unwrap().clone());
    set_global(&window, "turts_ui", ui.as_ref());
    ui.forget();
    let memory = Closure::<dyn Fn() -> f64>::new(memory_bytes);
    set_global(&window, "turts_memory_bytes", memory.as_ref());
    memory.forget();

    let mut app = build_app(options);
    app.add_systems(Update, drain_moves).add_systems(
        Last,
        (
            publish_debug.after(rebuild_debug_state),
            publish_scene,
            publish_ui,
        ),
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
fn browser_move_commands(
    state: &ReplicaState,
    targets: &[(f32, f32)],
) -> Option<Vec<PlayerCommandKind>> {
    let ready = state.content.is_some()
        && state.welcome.is_some()
        && state
            .replica
            .as_ref()
            .is_some_and(|replica| replica.snapshot_tick.is_some());
    if !ready {
        return None;
    }
    let unit_ids = owned_mobile_ids(state);
    if unit_ids.is_empty() {
        return Some(Vec::new());
    }
    Some(
        targets
            .iter()
            .map(|&(x, y)| PlayerCommandKind::MoveTo {
                unit_ids: unit_ids.clone(),
                target: Pos { x, y },
            })
            .collect(),
    )
}

fn drain_moves(
    mut pending: ResMut<PendingCommands>,
    status: Res<CommandSendStatus>,
    state: Res<ReplicaState>,
) {
    if status.reconnect_required {
        MOVES.lock().unwrap().clear();
        return;
    }
    let mut moves = MOVES.lock().unwrap();
    if moves.is_empty() {
        return;
    }
    let Some(commands) = browser_move_commands(&state, &moves) else {
        return;
    };
    moves.clear();
    for command in commands {
        submit(&mut pending, command);
    }
}

fn publish_debug(state: Res<DebugState>) {
    DEBUG.lock().unwrap().clone_from(&state.0);
}

/// `Last`: list the rendered units and count the active 3D cameras for the page.
/// `GlobalTransform` is up to date here: the transform propagation runs in `PostUpdate`.
fn publish_scene(
    units: Query<(&SceneUnit, &GlobalTransform), With<Mesh3d>>,
    nodes: Query<(&SceneNode, &GlobalTransform), With<Mesh3d>>,
    cameras: Query<&Camera, With<Camera3d>>,
) {
    let mut entries: Vec<(u32, Vec3)> = units
        .iter()
        .map(|(unit, transform)| (unit.0, transform.translation()))
        .collect();
    entries.sort_unstable_by_key(|(unit_id, _)| *unit_id);
    let mut node_entries: Vec<(u32, Vec3)> = nodes
        .iter()
        .map(|(node, transform)| (node.0, transform.translation()))
        .collect();
    node_entries.sort_unstable_by_key(|(id, _)| *id);
    let active_cameras = cameras.iter().filter(|camera| camera.is_active).count();
    *SCENE.lock().unwrap() = scene_json(active_cameras, &entries, &node_entries);
}

fn publish_ui(world: &mut World) {
    let snapshot = rendered_ui_snapshot(world);
    *UI.lock().unwrap() = serde_json::to_string(&snapshot).unwrap_or_default();
}

/// The JSON of `window.turts_scene`. A non-finite coordinate is written as `null`, so the page
/// can always parse the text and the check fails on the value.
fn scene_json(cameras_3d: usize, entries: &[(u32, Vec3)], nodes: &[(u32, Vec3)]) -> String {
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
    let node_items: Vec<String> = nodes
        .iter()
        .map(|(node_id, at)| {
            format!(
                "{{\"node_id\":{node_id},\"translation\":[{},{},{}]}}",
                number(at.x),
                number(at.y),
                number(at.z)
            )
        })
        .collect();
    format!(
        "{{\"cameras_3d\":{cameras_3d},\"units\":[{}],\"nodes\":[{}]}}",
        items.join(","),
        node_items.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_json_lists_units_with_their_translation() {
        assert_eq!(
            scene_json(0, &[], &[]),
            r#"{"cameras_3d":0,"units":[],"nodes":[]}"#
        );
        assert_eq!(
            scene_json(
                1,
                &[
                    (1, Vec3::new(20.0, 0.9, 8.5)),
                    (2, Vec3::new(f32::NAN, 0.0, 1.0))
                ],
                &[(7, Vec3::new(24.0, 0.4, 8.0))]
            ),
            concat!(
                r#"{"cameras_3d":1,"units":["#,
                r#"{"unit_id":1,"translation":[20,0.9,8.5]},"#,
                r#"{"unit_id":2,"translation":[null,0,1]}],"nodes":["#,
                r#"{"node_id":7,"translation":[24,0.4,8]}]}"#
            )
        );
    }

    #[test]
    fn scene_json_reports_the_camera_count() {
        // M1 verify: T7 asserts exactly one active 3D camera, so the count must reach the page.
        assert!(scene_json(2, &[], &[]).starts_with(r#"{"cameras_3d":2,"#));
    }

    #[test]
    fn browser_move_adapter_waits_for_snapshot_and_filters_static_units() {
        use std::{collections::VecDeque, path::PathBuf};
        use turts_client::{
            content::ClientContent,
            ingest::{ReplicaState, WelcomeInfo},
            replica::{Replica, UnitReplica},
        };
        use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
        use turts_net_protocol::{Pos, UnitId};

        let targets = [(4.0, 5.0)];
        assert!(browser_move_commands(&ReplicaState::default(), &targets).is_none());

        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
        let archive = PackageArchive::parse(&pack_dir(&base).unwrap()).unwrap();
        let content = ClientContent::from_archives(&[archive]).unwrap();
        let mut state = ReplicaState::default();
        let mut replica = Replica::new(20);
        replica.snapshot_tick = Some(1);
        let static_id = content
            .catalog
            .units
            .iter()
            .find(|(_, def)| def.movement.is_none())
            .unwrap()
            .0
            .to_string();
        let mobile_id = content
            .catalog
            .units
            .iter()
            .find(|(_, def)| def.movement.is_some())
            .unwrap()
            .0
            .to_string();
        replica.units.insert(
            1,
            UnitReplica {
                content_id: static_id,
                owner_slot: 0,
                samples: VecDeque::new(),
            },
        );
        replica.units.insert(
            2,
            UnitReplica {
                content_id: mobile_id,
                owner_slot: 0,
                samples: VecDeque::new(),
            },
        );
        replica.units.insert(
            3,
            UnitReplica {
                content_id: replica.units[&2].content_id.clone(),
                owner_slot: 1,
                samples: VecDeque::new(),
            },
        );
        replica.units.insert(
            4,
            UnitReplica {
                content_id: replica.units[&2].content_id.clone(),
                owner_slot: 0,
                samples: VecDeque::new(),
            },
        );
        state.welcome = Some(WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
        });
        state.content = Some(content);
        state.replica = Some(replica);
        let expanded = browser_move_commands(&state, &targets).unwrap();
        assert_eq!(
            expanded,
            vec![PlayerCommandKind::MoveTo {
                unit_ids: vec![UnitId(2), UnitId(4)],
                target: Pos { x: 4.0, y: 5.0 },
            }]
        );

        let mut static_only = state;
        static_only.replica.as_mut().unwrap().units.remove(&2);
        static_only.replica.as_mut().unwrap().units.remove(&3);
        static_only.replica.as_mut().unwrap().units.remove(&4);
        assert!(
            browser_move_commands(&static_only, &targets)
                .unwrap()
                .is_empty()
        );
    }
}
