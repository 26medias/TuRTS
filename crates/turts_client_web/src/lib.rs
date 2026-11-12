//! Thin wasm32 entry for the browser client.
//!
//! All client logic lives in `turts_client`. This crate reads the page query, builds the app,
//! and installs three test hooks on `window`:
//!
//! - `window.turts_debug_request(min_interval_ms?)` asks for the debug state and returns a
//!   `Promise` of the debug JSON string (`turts_client::debug::debug_json`). The client builds
//!   the state only on a request, at the end of the next frame that the 4 Hz cap allows (design
//!   D 9, row DBG1), and resolves every waiting promise with that one snapshot, so the
//!   snapshot, `window.turts_scene()` and `window.turts_ui()` come from one frame. The promise
//!   rejects after 10 s with no frame (a hidden tab). `min_interval_ms` lowers the cap for one
//!   request; only a test harness passes it.
//! - `window.turts_debug()` returns the last published debug JSON and asks for nothing. It
//!   returns an empty string until the first request has been served. The page posts nothing.
//! - `window.turts_move(x, y)` queues a move order for the units of this player. The order is
//!   sent only when the client is joined.
//! - `window.turts_memory_bytes()` returns the size of the wasm linear memory in bytes. Wasm
//!   memory never shrinks, so the last value that a run reads is its high-water mark
//!   (`tests/browser/measure.js`, design 4.5).
//! - `window.turts_scene()` returns actual mesh entities and camera projection metadata. Unit
//!   and node entries keep their stable ids and world translations, and add a projected ground
//!   center plus the declared pick radius when both are available. The top-level target size,
//!   viewport, scale factor and `clip_from_world` matrix come from the active Bevy camera/window;
//!   they let pointer tests project arbitrary ground targets without duplicating camera settings.
//!   Coordinates use a top-left origin in window-logical pixels. UI snapshot rectangles separately
//!   use physical pixels local to the camera viewport. No scene or private world values are
//!   injected by the bridge.
//!
//! The page query is `?server=<http url>&ws=<ws url>`. When it is wrong, the page gets the
//! message in `window.turts_error` and the app does not start.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::{LazyLock, Mutex},
};

use bevy::camera::RenderTarget;
use bevy::camera::visibility::InheritedVisibility;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::ui::{CalculatedClip, ComputedNode, ComputedUiTargetCamera, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef};
use turts_client::build_app;
use turts_client::commands::{CommandSendStatus, PendingCommands, owned_mobile_ids, submit_order};
use turts_client::debug::{DEBUG_MIN_INTERVAL_SECS, DebugRequest, DebugState, rebuild_debug_state};
use turts_client::debug_feed::NoticesFeed;
use turts_client::ingest::{
    BrowserIngestConfig, IngestCounters, IngestHooks, ReplicaState, apply_browser_ingest_config,
    ingest_messages,
};
use turts_client::join::Phase;
use turts_client::options::options_from_query;
use turts_client::render::{
    PresentedNode as SceneNode, PresentedUnit as SceneUnit, declared_node_pick_radius,
    declared_unit_pick_radius, projected_ground_center,
};
use turts_client::{ClientOpts, PlayerId};
use turts_net_protocol::{OrderEnvelope, Pos};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// The last debug JSON. The `window.turts_debug` closure reads it.
static DEBUG: Mutex<String> = Mutex::new(String::new());

/// The least interval of the `window.turts_debug_request` calls that no frame has taken yet, in
/// seconds. `None` when no call waits.
static DEBUG_ASK: Mutex<Option<f64>> = Mutex::new(None);

/// How long a `window.turts_debug_request` promise waits for a frame, in milliseconds.
const DEBUG_REQUEST_TIMEOUT_MS: i32 = 10_000;

thread_local! {
    /// The `resolve` functions of the promises that wait for the next published debug state.
    /// The wasm module has one thread, so a thread-local is the right place for JS values.
    static DEBUG_WAITERS: RefCell<Vec<js_sys::Function>> = const { RefCell::new(Vec::new()) };
}

/// The last scene JSON. The `window.turts_scene` closure reads it.
static SCENE: Mutex<String> = Mutex::new(String::new());
static UI: Mutex<String> = Mutex::new(String::new());
static LAYOUT_LABELS: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);

/// The targets from `window.turts_move` that no frame has taken yet.
static MOVES: Mutex<Vec<(f32, f32)>> = Mutex::new(Vec::new());
static INGEST_CONFIG: Mutex<Option<BrowserIngestConfig>> = Mutex::new(None);
static INGEST_CONFIG_STATUS: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new("{\"state\":\"idle\"}".to_string()));

#[wasm_bindgen(start)]
pub fn start() {
    let Some(window) = web_sys::window() else {
        return;
    };
    request_keyboard_layout_map();
    let query = window.location().search().unwrap_or_default();
    let mut options = match options_from_query(&query) {
        Ok(options) => options,
        Err(message) => {
            set_global(&window, "turts_error", &JsValue::from_str(&message));
            return;
        }
    };
    let (stored_player_id, corrupt_player_id) = read_session_player_id(&window);
    let stored_identity_rejoin = stored_player_id.is_some();
    options.player_id = stored_player_id;
    let debug = Closure::<dyn Fn() -> String>::new(|| DEBUG.lock().unwrap().clone());
    let clear_frames = Closure::<dyn Fn()>::new(|| {
        turts_client::core_adapter::clear_frames();
        if let Some(window) = web_sys::window() {
            let _ =
                js_sys::Reflect::set(&window, &"__turts_seq_queue".into(), &js_sys::Array::new());
        }
    });
    let debug_function: &js_sys::Function = debug.as_ref().unchecked_ref();
    let _ = js_sys::Reflect::set(
        debug_function,
        &"frames_clear".into(),
        clear_frames.as_ref(),
    );
    clear_frames.forget();
    set_global(&window, "turts_debug", debug.as_ref());
    debug.forget();
    let debug_request = Closure::<dyn Fn(JsValue) -> js_sys::Promise>::new(request_debug);
    set_global(&window, "turts_debug_request", debug_request.as_ref());
    debug_request.forget();
    let move_to = Closure::<dyn Fn(f32, f32)>::new(|x, y| MOVES.lock().unwrap().push((x, y)));
    set_global(&window, "turts_move", move_to.as_ref());
    move_to.forget();
    let scene = Closure::<dyn Fn() -> String>::new(|| SCENE.lock().unwrap().clone());
    set_global(&window, "turts_scene", scene.as_ref());
    scene.forget();
    let ui = Closure::<dyn Fn() -> String>::new(|| UI.lock().unwrap().clone());
    set_global(&window, "turts_ui", ui.as_ref());
    ui.forget();
    let configure_ingest = Closure::<dyn Fn(JsValue) -> Result<(), JsValue>>::new(|value| {
        let json = js_sys::JSON::stringify(&value)?;
        let json = json
            .as_string()
            .ok_or_else(|| JsValue::from_str("configuration must be an object"))?;
        let config =
            BrowserIngestConfig::parse_json(&json).map_err(|error| JsValue::from_str(&error))?;
        let mut pending = INGEST_CONFIG.lock().unwrap();
        if pending.is_some() {
            return Err(JsValue::from_str(
                "an ingest configuration is already queued",
            ));
        }
        *pending = Some(config);
        *INGEST_CONFIG_STATUS.lock().unwrap() = "{\"state\":\"queued\"}".to_string();
        Ok(())
    });
    set_global(&window, "turts_configure_ingest", configure_ingest.as_ref());
    configure_ingest.forget();
    let ingest_status =
        Closure::<dyn Fn() -> String>::new(|| INGEST_CONFIG_STATUS.lock().unwrap().clone());
    set_global(
        &window,
        "turts_ingest_config_status",
        ingest_status.as_ref(),
    );
    ingest_status.forget();
    let memory = Closure::<dyn Fn() -> f64>::new(memory_bytes);
    set_global(&window, "turts_memory_bytes", memory.as_ref());
    memory.forget();

    let mut app = build_app(options);
    if stored_identity_rejoin {
        *app.world_mut()
            .resource_mut::<turts_client::rejoin::RejoinController>() =
            turts_client::rejoin::RejoinController::for_stored_identity();
    }
    if corrupt_player_id {
        app.world_mut()
            .resource_mut::<NoticesFeed>()
            .0
            .push(serde_json::json!({
                "text":"Saved player identity was invalid; a new session identity will be used",
                "message":"Saved player identity was invalid; a new session identity will be used"
            }));
    }
    app.add_systems(
        Update,
        (
            drain_moves,
            apply_keyboard_layout_labels,
            take_debug_request,
        ),
    )
    .add_systems(Last, persist_session_player_id)
    .add_systems(
        PreUpdate,
        apply_queued_ingest_config.before(ingest_messages),
    )
    .add_systems(
        Last,
        (
            publish_debug.after(rebuild_debug_state),
            publish_scene,
            publish_ui.after(turts_client::core_adapter::FrameCloseSet),
        ),
    );
    app.run();
}

/// Call the browser API once during entry startup. A missing API or rejected promise publishes
/// an empty map, which keeps the engine's US physical-code labels as the fallback.
fn request_keyboard_layout_map() {
    let result = (|| {
        let window = web_sys::window()?;
        let navigator =
            js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("navigator")).ok()?;
        let keyboard = js_sys::Reflect::get(&navigator, &JsValue::from_str("keyboard")).ok()?;
        if keyboard.is_null() || keyboard.is_undefined() {
            return None;
        }
        let method = js_sys::Reflect::get(&keyboard, &JsValue::from_str("getLayoutMap")).ok()?;
        let method = method.dyn_into::<js_sys::Function>().ok()?;
        method
            .call0(&keyboard)
            .ok()?
            .dyn_into::<js_sys::Promise>()
            .ok()
    })();
    let Some(promise) = result else {
        *LAYOUT_LABELS.lock().unwrap() = Some(BTreeMap::new());
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let labels = JsFuture::from(promise)
            .await
            .ok()
            .and_then(layout_map_entries)
            .unwrap_or_default();
        *LAYOUT_LABELS.lock().unwrap() = Some(labels);
    });
}

fn layout_map_entries(map: JsValue) -> Option<BTreeMap<String, String>> {
    let entries = js_sys::Reflect::get(&map, &JsValue::from_str("entries")).ok()?;
    let entries = entries.dyn_into::<js_sys::Function>().ok()?;
    let iterator = entries.call0(&map).ok()?;
    let rows = js_sys::Array::from(&iterator);
    let mut labels = BTreeMap::new();
    for row in rows.iter() {
        let pair = js_sys::Array::from(&row);
        let (Some(code), Some(label)) = (pair.get(0).as_string(), pair.get(1).as_string()) else {
            continue;
        };
        labels.insert(code, label);
    }
    Some(labels)
}

fn apply_keyboard_layout_labels(
    mut labels: ResMut<turts_client::client_shell::BindingLabels>,
    mut applied: Local<bool>,
) {
    if *applied {
        return;
    }
    let Some(entries) = LAYOUT_LABELS.lock().unwrap().take() else {
        return;
    };
    *labels = turts_client::client_shell::BindingLabels::from_layout_entries(entries);
    *applied = true;
}

const SESSION_PLAYER_ID_KEY: &str = "turts.v1.player_id";

fn read_session_player_id(window: &web_sys::Window) -> (Option<u64>, bool) {
    let Ok(Some(storage)) = window.session_storage() else {
        return (None, false);
    };
    let Ok(Some(value)) = storage.get_item(SESSION_PLAYER_ID_KEY) else {
        return (None, false);
    };
    match turts_client::options::parse_session_player_id(&value) {
        Some(player_id) => (Some(player_id), false),
        None => (None, true),
    }
}

fn persist_session_player_id(
    phase: Res<Phase>,
    player_id: Res<PlayerId>,
    mut persisted: Local<bool>,
) {
    if *persisted || !matches!(*phase, Phase::Joined { .. }) {
        return;
    }
    *persisted = true;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.session_storage() else {
        return;
    };
    let _ = storage.set_item(SESSION_PLAYER_ID_KEY, &player_id.0.to_string());
}

fn apply_queued_ingest_config(
    mut hooks: ResMut<IngestHooks>,
    mut counters: ResMut<IngestCounters>,
    mut state: ResMut<ReplicaState>,
    mut options: ResMut<ClientOpts>,
) {
    let Some(config) = INGEST_CONFIG.lock().unwrap().take() else {
        return;
    };
    if state
        .replica
        .as_ref()
        .is_none_or(|replica| replica.last_applied_tick.is_none())
    {
        *INGEST_CONFIG.lock().unwrap() = Some(config);
        *INGEST_CONFIG_STATUS.lock().unwrap() =
            "{\"state\":\"waiting_for_coherent_snapshot\"}".to_string();
        return;
    }
    match apply_browser_ingest_config(config, &mut hooks, &mut counters, &mut state, &mut options) {
        Ok(tick) => {
            *INGEST_CONFIG_STATUS.lock().unwrap() =
                format!("{{\"state\":\"applied\",\"arm_tick\":{tick}}}");
        }
        Err(error) => {
            let error =
                serde_json::to_string(&error).unwrap_or_else(|_| "\"configuration failed\"".into());
            *INGEST_CONFIG_STATUS.lock().unwrap() =
                format!("{{\"state\":\"rejected\",\"error\":{error}}}");
        }
    }
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
) -> Option<Vec<OrderEnvelope>> {
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
    let content = state.content.as_ref()?;
    targets
        .iter()
        .map(|&(x, y)| {
            turts_client::commands::move_order_envelope(content, unit_ids.clone(), Pos { x, y })
        })
        .collect::<Option<Vec<_>>>()
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
        submit_order(&mut pending, command);
    }
}

/// `window.turts_debug_request`: note the request for the next frame, and return a promise that
/// `publish_debug` resolves with the snapshot that serves it.
fn request_debug(min_interval_ms: JsValue) -> js_sys::Promise {
    let wanted = min_interval_ms
        .as_f64()
        .filter(|ms| ms.is_finite() && *ms >= 0.0)
        .map_or(DEBUG_MIN_INTERVAL_SECS, |ms| ms / 1000.0);
    {
        let mut ask = DEBUG_ASK.lock().unwrap();
        *ask = Some(ask.map_or(wanted, |pending| pending.min(wanted)));
    }
    js_sys::Promise::new(&mut |resolve, reject| {
        DEBUG_WAITERS.with(|waiters| waiters.borrow_mut().push(resolve));
        let timeout = Closure::once_into_js(move || {
            let _ = reject.call1(
                &JsValue::NULL,
                &JsValue::from_str("turts_debug_request: no frame served the request"),
            );
        });
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                timeout.unchecked_ref(),
                DEBUG_REQUEST_TIMEOUT_MS,
            );
        }
    })
}

/// `Update`: hand the calls of `window.turts_debug_request` to the client, which builds the
/// state at the end of this frame (`turts_client::debug::debug_build_due`).
fn take_debug_request(mut request: ResMut<DebugRequest>) {
    if let Some(min_interval_secs) = DEBUG_ASK.lock().unwrap().take() {
        request.request(Some(min_interval_secs));
    }
}

/// `Last`: copy the debug state to the bridge only when the client built a new one, and resolve
/// the waiting promises with it. Every waiter asked before this frame began, so the snapshot
/// is newer than its request.
fn publish_debug(
    state: Res<DebugState>,
    request: Res<DebugRequest>,
    mut published_builds: Local<u64>,
) {
    if request.builds() == *published_builds {
        return;
    }
    *published_builds = request.builds();
    DEBUG.lock().unwrap().clone_from(&state.0);
    let waiters = DEBUG_WAITERS.with(|waiters| std::mem::take(&mut *waiters.borrow_mut()));
    if waiters.is_empty() {
        return;
    }
    let text = JsValue::from_str(&state.0);
    for resolve in waiters {
        let _ = resolve.call1(&JsValue::NULL, &text);
    }
}

/// `Last`: list the rendered units and count the active 3D cameras for the page.
/// `GlobalTransform` is up to date here: the transform propagation runs in `PostUpdate`.
type SceneWidgetQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static turts_client::widgets::primitives::AuthoredWidgetId,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static ComputedUiTargetCamera,
        &'static InheritedVisibility,
        Option<&'static CalculatedClip>,
    ),
>;

fn publish_scene(
    units: Query<(&SceneUnit, &GlobalTransform), With<Mesh3d>>,
    nodes: Query<(&SceneNode, &GlobalTransform), With<Mesh3d>>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, &RenderTarget), With<Camera3d>>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    widgets: SceneWidgetQuery<'_, '_>,
    state: Res<ReplicaState>,
    adapter_info: Option<Res<RenderAdapterInfo>>,
) {
    let active: Vec<_> = cameras
        .iter()
        .filter(|(_, camera, _, _)| camera.is_active)
        .collect();
    let primary_window = windows.iter().next();
    let primary_entity = primary_window.map(|(entity, _)| entity);
    let primary_active: Vec<_> = active
        .iter()
        .filter(|(_, _, _, target)| match target {
            RenderTarget::Window(WindowRef::Primary) => primary_entity.is_some(),
            RenderTarget::Window(WindowRef::Entity(entity)) => {
                primary_entity.as_ref() == Some(entity)
            }
            RenderTarget::Image(_) | RenderTarget::TextureView(_) | RenderTarget::None { .. } => {
                false
            }
        })
        .collect();
    let camera_metadata = if primary_active.len() == 1 {
        primary_window.and_then(|(_, window)| {
            let (_, camera, transform, _) = *primary_active[0];
            Some(SceneCamera {
                window_logical_size: Vec2::new(window.width(), window.height()),
                viewport_logical_rect: camera.logical_viewport_rect()?,
                viewport_physical_rect: camera.physical_viewport_rect()?,
                scale_factor: window.scale_factor(),
                clip_from_world: camera.clip_from_view() * transform.to_matrix().inverse(),
            })
        })
    } else {
        None
    };
    let projection_camera = primary_active
        .first()
        .filter(|_| primary_active.len() == 1)
        .map(|(_, camera, transform, _)| (*camera, *transform));

    let mut entries: Vec<SceneEntry> = units
        .iter()
        .map(|(unit, transform)| {
            let translation = transform.translation();
            let pick_radius = state
                .replica
                .as_ref()
                .and_then(|replica| replica.units.get(&unit.0))
                .and_then(|unit| {
                    declared_unit_pick_radius(state.content.as_ref()?, &unit.content_id)
                });
            let screen_center = projection_camera.and_then(|(camera, camera_transform)| {
                projected_ground_center(
                    camera,
                    camera_transform,
                    Pos {
                        x: translation.x,
                        y: translation.z,
                    },
                )
            });
            SceneEntry {
                id: unit.0,
                translation,
                screen_center,
                pick_radius,
            }
        })
        .collect();
    entries.sort_unstable_by_key(|entry| entry.id);
    let mut node_entries: Vec<SceneEntry> = nodes
        .iter()
        .map(|(node, transform)| {
            let translation = transform.translation();
            let pick_radius = state
                .replica
                .as_ref()
                .and_then(|replica| replica.nodes.values().find(|value| value.id.0 == node.0))
                .and_then(|value| {
                    declared_node_pick_radius(state.content.as_ref()?, &value.content_id)
                });
            let screen_center = projection_camera.and_then(|(camera, camera_transform)| {
                projected_ground_center(
                    camera,
                    camera_transform,
                    Pos {
                        x: translation.x,
                        y: translation.z,
                    },
                )
            });
            SceneEntry {
                id: node.0,
                translation,
                screen_center,
                pick_radius,
            }
        })
        .collect();
    node_entries.sort_unstable_by_key(|entry| entry.id);
    let adapter = adapter_info.map(|info| SceneAdapter {
        backend: format!("{:?}", info.backend),
        name: info.name.clone(),
        device_type: format!("{:?}", info.device_type),
    });
    let mut scene: serde_json::Value = serde_json::from_str(&scene_json(
        active.len(),
        camera_metadata,
        adapter,
        &entries,
        &node_entries,
    ))
    .unwrap_or_else(|_| serde_json::json!({}));
    let mut widget_rects = widgets
        .iter()
        .filter_map(|(id, node, transform, target, visible, clip)| {
            if !visible.get()
                || node.size().min_element() <= 0.0
                || !intersects_calculated_clip(node, transform, clip)
            {
                return None;
            }
            let camera_id = target.get()?;
            let (_, camera, _, _) = cameras
                .iter()
                .find(|(entity, _, _, _)| *entity == camera_id)?;
            let viewport = camera.physical_viewport_rect()?;
            let scale = camera.target_scaling_factor()?;
            if !camera.is_active || !scale.is_finite() || scale <= 0.0 {
                return None;
            }
            let centre = transform.to_scale_angle_translation().2;
            let size = node.size();
            let origin = viewport.min.as_vec2();
            let rect = [
                (origin.x + centre.x - size.x / 2.0) / scale,
                (origin.y + centre.y - size.y / 2.0) / scale,
                size.x / scale,
                size.y / scale,
            ];
            rect.iter()
                .all(|value| value.is_finite())
                .then(|| serde_json::json!({"id":id.0,"rect":rect}))
        })
        .collect::<Vec<_>>();
    widget_rects.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    scene["widgets"] = serde_json::Value::Array(widget_rects);
    *SCENE.lock().unwrap() = scene.to_string();
}

fn intersects_calculated_clip(
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    clip: Option<&CalculatedClip>,
) -> bool {
    match clip {
        Some(CalculatedClip::FullyClipped) => false,
        Some(CalculatedClip::Rects(clips)) => {
            let centre = transform.to_scale_angle_translation().2;
            let half = node.size() / 2.0;
            let corners = [
                Vec2::new(centre.x - half.x, centre.y - half.y),
                Vec2::new(centre.x + half.x, centre.y - half.y),
                Vec2::new(centre.x - half.x, centre.y + half.y),
                Vec2::new(centre.x + half.x, centre.y + half.y),
            ];
            clips.iter().all(|clip| {
                let mut local = corners
                    .into_iter()
                    .map(|point| clip.world_to_clip_local.transform_point2(point));
                let first = local.next().unwrap_or(Vec2::ZERO);
                let bounds = local.fold(Rect::from_corners(first, first), |bounds, point| {
                    Rect::from_corners(bounds.min.min(point), bounds.max.max(point))
                });
                !bounds.intersect(clip.rect).is_empty()
            })
        }
        None => true,
    }
}

fn publish_ui(world: &mut World) {
    let snapshot = world
        .get_resource::<turts_client::debug_feed::UiSnapshotFeed>()
        .map(|feed| feed.0.clone())
        .unwrap_or_else(|| turts_client::widgets::snapshot::snapshot(world));
    *UI.lock().unwrap() = snapshot.to_string();
}

/// Actual camera data needed to project world points without duplicating camera constants.
#[derive(Clone, Copy)]
struct SceneCamera {
    window_logical_size: Vec2,
    viewport_logical_rect: Rect,
    viewport_physical_rect: URect,
    scale_factor: f32,
    clip_from_world: Mat4,
}

#[derive(Clone)]
struct SceneAdapter {
    backend: String,
    name: String,
    device_type: String,
}

#[derive(Clone, Copy)]
struct SceneEntry {
    id: u32,
    translation: Vec3,
    screen_center: Option<Vec2>,
    pick_radius: Option<f32>,
}

fn number(value: f32) -> serde_json::Value {
    serde_json::Number::from_f64(value as f64)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod widget_scene_tests {
    use super::*;

    #[test]
    fn scene_widget_bridge_omits_fully_clipped_nodes_and_keeps_partial_intersections() {
        let node = ComputedNode {
            size: Vec2::splat(20.0),
            ..default()
        };
        let transform = UiGlobalTransform::default();
        assert!(!intersects_calculated_clip(
            &node,
            &transform,
            Some(&CalculatedClip::FullyClipped)
        ));
        let outside = CalculatedClip::default().with_rect(
            Rect::from_corners(Vec2::splat(20.0), Vec2::splat(30.0)),
            &transform,
        );
        assert!(!intersects_calculated_clip(
            &node,
            &transform,
            Some(&outside)
        ));
        let partial = CalculatedClip::default().with_rect(
            Rect::from_corners(Vec2::splat(-5.0), Vec2::splat(5.0)),
            &transform,
        );
        assert!(intersects_calculated_clip(
            &node,
            &transform,
            Some(&partial)
        ));
        assert!(intersects_calculated_clip(&node, &transform, None));
    }
}

fn pair(value: Vec2) -> serde_json::Value {
    serde_json::Value::Array(vec![number(value.x), number(value.y)])
}

fn rect(value: Rect) -> serde_json::Value {
    serde_json::Value::Array(vec![
        number(value.min.x),
        number(value.min.y),
        number(value.width()),
        number(value.height()),
    ])
}

fn scene_entry(entry: SceneEntry, identity_key: &str) -> serde_json::Value {
    let center = entry.screen_center.filter(|value| value.is_finite());
    let mut value = serde_json::json!({
        "translation": [number(entry.translation.x), number(entry.translation.y), number(entry.translation.z)],
        "ground_screen_center": center.map(pair).unwrap_or(serde_json::Value::Null),
        "pick_radius": entry.pick_radius.filter(|value| value.is_finite()).map(number).unwrap_or(serde_json::Value::Null),
    });
    value
        .as_object_mut()
        .unwrap()
        .insert(identity_key.into(), entry.id.into());
    value
}

/// `window.turts_scene` includes only entities with meshes. Camera coordinates and the pick
/// circles are read-only metadata from the actual camera and accepted package catalog.
fn scene_json(
    cameras_3d: usize,
    camera: Option<SceneCamera>,
    adapter: Option<SceneAdapter>,
    entries: &[SceneEntry],
    nodes: &[SceneEntry],
) -> String {
    let (window_size, viewport, physical_viewport, scale_factor, clip_from_world) = camera.map_or(
        (
            serde_json::Value::Null,
            serde_json::Value::Null,
            serde_json::Value::Null,
            serde_json::Value::Null,
            serde_json::Value::Null,
        ),
        |camera| {
            let matrix = serde_json::Value::Array(
                camera
                    .clip_from_world
                    .to_cols_array()
                    .into_iter()
                    .map(number)
                    .collect(),
            );
            (
                pair(camera.window_logical_size),
                rect(camera.viewport_logical_rect),
                serde_json::Value::Array(vec![
                    number(camera.viewport_physical_rect.min.x as f32),
                    number(camera.viewport_physical_rect.min.y as f32),
                    number(
                        (camera.viewport_physical_rect.max.x - camera.viewport_physical_rect.min.x)
                            as f32,
                    ),
                    number(
                        (camera.viewport_physical_rect.max.y - camera.viewport_physical_rect.min.y)
                            as f32,
                    ),
                ]),
                number(camera.scale_factor),
                matrix,
            )
        },
    );
    serde_json::json!({
        "cameras_3d": cameras_3d,
        "coordinate_space": "window-logical-pixels-top-left",
        "window_logical_size": window_size,
        "viewport_logical_rect": viewport,
        "viewport_physical_rect": physical_viewport,
        "ui_rect_coordinate_space": "camera-viewport-physical-pixels-top-left",
        "scale_factor": scale_factor,
        "clip_from_world": clip_from_world,
        "render_adapter": adapter.map(|adapter| serde_json::json!({
            "backend": adapter.backend,
            "name": adapter.name,
            "device_type": adapter.device_type,
        })),
        "units": entries.iter().copied().map(|entry| scene_entry(entry, "unit_id")).collect::<Vec<_>>(),
        "nodes": nodes.iter().copied().map(|entry| scene_entry(entry, "node_id")).collect::<Vec<_>>(),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_json_lists_rendered_entities_with_camera_projection_and_pick_metadata() {
        let camera = SceneCamera {
            window_logical_size: Vec2::new(1280.0, 720.0),
            viewport_logical_rect: Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
            viewport_physical_rect: URect::from_corners(UVec2::ZERO, UVec2::new(1280, 720)),
            scale_factor: 1.0,
            clip_from_world: Mat4::IDENTITY,
        };
        let json: serde_json::Value = serde_json::from_str(&scene_json(
            1,
            Some(camera),
            Some(SceneAdapter {
                backend: "BrowserWebGpu".into(),
                name: "SwiftShader Device (Subzero)".into(),
                device_type: "Cpu".into(),
            }),
            &[
                SceneEntry {
                    id: 1,
                    translation: Vec3::new(20.0, 0.9, 8.5),
                    screen_center: Some(Vec2::new(640.0, 360.0)),
                    pick_radius: Some(0.5),
                },
                SceneEntry {
                    id: 2,
                    translation: Vec3::new(f32::NAN, 0.0, 1.0),
                    screen_center: None,
                    pick_radius: None,
                },
            ],
            &[SceneEntry {
                id: 7,
                translation: Vec3::new(24.0, 0.4, 8.0),
                screen_center: Some(Vec2::new(700.0, 355.0)),
                pick_radius: Some(1.0),
            }],
        ))
        .unwrap();
        assert_eq!(json["cameras_3d"], 1);
        assert_eq!(json["coordinate_space"], "window-logical-pixels-top-left");
        assert_eq!(
            json["window_logical_size"],
            serde_json::json!([1280.0, 720.0])
        );
        assert_eq!(
            json["viewport_logical_rect"],
            serde_json::json!([0.0, 0.0, 1280.0, 720.0])
        );
        assert_eq!(
            json["viewport_physical_rect"],
            serde_json::json!([0.0, 0.0, 1280.0, 720.0])
        );
        assert_eq!(
            json["ui_rect_coordinate_space"],
            "camera-viewport-physical-pixels-top-left"
        );
        assert_eq!(json["scale_factor"], 1.0);
        assert_eq!(json["clip_from_world"].as_array().unwrap().len(), 16);
        assert_eq!(json["render_adapter"]["backend"], "BrowserWebGpu");
        assert_eq!(
            json["render_adapter"]["name"],
            "SwiftShader Device (Subzero)"
        );
        assert_eq!(json["render_adapter"]["device_type"], "Cpu");
        assert_eq!(json["units"][0]["unit_id"], 1);
        assert_eq!(json["units"][0]["translation"][0], 20.0);
        assert!((json["units"][0]["translation"][1].as_f64().unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(json["units"][0]["translation"][2], 8.5);
        assert_eq!(
            json["units"][0]["ground_screen_center"],
            serde_json::json!([640.0, 360.0])
        );
        assert_eq!(json["units"][0]["pick_radius"], 0.5);
        assert_eq!(json["units"][1]["translation"][0], serde_json::Value::Null);
        assert_eq!(
            json["units"][1]["ground_screen_center"],
            serde_json::Value::Null
        );
        assert_eq!(json["nodes"][0]["node_id"], 7);
        assert_eq!(json["nodes"][0]["pick_radius"], 1.0);
    }

    #[test]
    fn scene_json_keeps_camera_count_and_null_projection_when_unavailable() {
        let json: serde_json::Value =
            serde_json::from_str(&scene_json(0, None, None, &[], &[])).unwrap();
        assert_eq!(json["cameras_3d"], 0);
        assert_eq!(json["clip_from_world"], serde_json::Value::Null);
        assert_eq!(json["units"], serde_json::json!([]));
        assert_eq!(json["nodes"], serde_json::json!([]));
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
        use turts_net_protocol::{OrderEnvelope, OrderTarget, Pos, QueueMode, UnitId};

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
                facing: None,
                samples: VecDeque::new(),
            },
        );
        replica.units.insert(
            2,
            UnitReplica {
                content_id: mobile_id,
                owner_slot: 0,
                facing: None,
                samples: VecDeque::new(),
            },
        );
        replica.units.insert(
            3,
            UnitReplica {
                content_id: replica.units[&2].content_id.clone(),
                owner_slot: 1,
                facing: None,
                samples: VecDeque::new(),
            },
        );
        replica.units.insert(
            4,
            UnitReplica {
                content_id: replica.units[&2].content_id.clone(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::new(),
            },
        );
        state.welcome = Some(WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
            match_id: 0,
        });
        let expected_order = turts_client::commands::move_order_envelope(
            &content,
            Vec::new(),
            Pos { x: 4.0, y: 5.0 },
        )
        .unwrap()
        .order;
        state.content = Some(content);
        state.replica = Some(replica);
        let expanded = browser_move_commands(&state, &targets).unwrap();
        assert_eq!(
            expanded,
            vec![OrderEnvelope {
                order: expected_order,
                units: vec![UnitId(2), UnitId(4)],
                target: OrderTarget::Point(Pos { x: 4.0, y: 5.0 }),
                queue: QueueMode::Replace,
                group: 0,
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
