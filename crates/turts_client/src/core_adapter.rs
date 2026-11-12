//! Production adapter between Bevy's ordered window stream and the pure interaction core.

use bevy::camera::{PerspectiveProjection, Projection};
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseMotion;
use bevy::platform::time::Instant;
use bevy::prelude::IntoScheduleConfigs;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowEvent};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::atomic::{AtomicBool, Ordering},
};
use turts_interaction::{
    CameraState, Core, Effect, InputEvent, InteractionSettings, NodeView, UnitView, Viewport,
    WorldView, camera::CameraOptions,
};
use turts_mod_api::{
    Binding, Capability, ContentId, Footprint, InputMapDef, OrderDef, OrderRulesDef, SlotRelation,
};
use turts_net_protocol::{SlotId, UnitId};

use crate::{
    client_shell::BindingEditorState,
    commands::PendingCommands,
    debug_feed::{
        AudioFeed, ContextMenuFeed, CursorFeed, FocusFeed, GroupsFeed, MarkersShown, NoticesFeed,
        PlayerIdFeed, ResultsFeed, RingsShown, SelectionFeed,
    },
    ingest::{CommandResult, CommandResults, ReplicaState},
    join::AcquiredPackages,
    present::Presented,
    render::setup_scene,
    ui_shared::{Selection, UiCaptureRoot, UiPresentationSet, cursor_in_ui_viewport},
};

mod effects;
use effects::{
    apply_cursor, apply_marker, apply_notice, apply_send, apply_shell_action, apply_sound,
};

static CLEAR_FRAMES_REQUESTED: AtomicBool = AtomicBool::new(false);

type CapturePanelQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
        &'static ComputedUiTargetCamera,
        Option<&'static CalculatedClip>,
        &'static bevy::ui::ComputedStackIndex,
    ),
    With<UiCaptureRoot>,
>;

type MinimapInputQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
        &'static ComputedUiTargetCamera,
        Option<&'static CalculatedClip>,
    ),
    With<crate::widgets::primitives::MinimapViewConfig>,
>;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CoreAdapterSet;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameCloseSet;

/// Ordered input from presentation plugins and headless callers into the interaction core.
/// Producers enqueue events before [`CoreAdapterSet`] runs; the adapter drains them once.
#[derive(Resource, Default)]
pub struct CoreInputQueue(pub Vec<InputEvent>);

/// Called only by the browser harness' explicit `frames_clear` hook.
pub fn clear_frames() {
    CLEAR_FRAMES_REQUESTED.store(true, Ordering::Release);
}

/// Record the browser's actual pointer-lock state for the real game canvas. Bevy's
/// CursorOptions only describes a request; it does not prove that the browser granted it.
#[cfg(target_family = "wasm")]
fn refresh_actual_pointer_lock(mut cursor: ResMut<CursorFeed>) {
    cursor.lock_active = actual_canvas_pointer_lock();
}

#[cfg(not(target_family = "wasm"))]
fn refresh_actual_pointer_lock() {}

#[cfg(target_family = "wasm")]
fn actual_canvas_pointer_lock() -> Option<bool> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window()?;
    let document = js_sys::Reflect::get(window.as_ref(), &"document".into()).ok()?;
    let canvas = js_sys::Reflect::get(&document, &"getElementById".into())
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?
        .call1(&document, &"turts".into())
        .ok()?;
    if canvas.is_null() || canvas.is_undefined() {
        return None;
    }
    let locked = js_sys::Reflect::get(&document, &"pointerLockElement".into()).ok()?;
    if locked.is_null() || locked.is_undefined() {
        Some(false)
    } else {
        Some(locked == canvas)
    }
}

/// Copy the renderer-independent camera state into Bevy's camera transform and projection.
pub fn camera_components(camera: CameraState) -> (Transform, Projection) {
    let look_at = Vec3::new(camera.look_at.x, 0.0, camera.look_at.y);
    let back = camera.height * camera.pitch.tan();
    let offset = Vec3::new(
        camera.yaw.sin() * back,
        camera.height,
        camera.yaw.cos() * back,
    );
    let transform = Transform::from_translation(look_at + offset).looking_at(look_at, Vec3::Y);
    let projection = Projection::Perspective(PerspectiveProjection {
        fov: camera.vertical_fov,
        ..Default::default()
    });
    (transform, projection)
}

#[derive(Resource, Default)]
pub struct CoreRuntime {
    core: Option<Core>,
    content_index: Option<ContentIndex>,
    started: Option<Instant>,
    pressed_keys: HashSet<String>,
    cursor: Option<Vec2>,
    cursor_lock_requested: bool,
    cursor_lock_active: bool,
    relative_motion_this_frame: Vec2,
    pointer_over_widget: Option<bool>,
    minimap_pointer: Option<turts_mod_api::Button>,
    window_focused: Option<bool>,
    last_ui_selection: Option<u32>,
    last_core_primary: Option<u32>,
    pending_sequences: Vec<u32>,
    order_intents_this_frame: Vec<u64>,
    last_effects: Vec<ValueEffect>,
    markers: Vec<serde_json::Value>,
    next_order_intent_id: u64,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct CorePreferences {
    pub runtime_bindings: BTreeMap<String, Vec<Binding>>,
    pub interaction: InteractionSettings,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct CoreEffects(pub Vec<Effect>);
impl CoreRuntime {
    pub fn core(&self) -> Option<&Core> {
        self.core.as_ref()
    }
    pub fn effects(&self) -> &[ValueEffect] {
        &self.last_effects
    }
    /// Read-only interaction state used to present the cursor for a pending target order.
    pub fn pending_order_id(&self) -> Option<&str> {
        self.core.as_ref()?.pending_order_id()
    }
    /// Read-only camera controller pan context for mod-authored cursor presentation.
    pub fn camera_pan_context(&self) -> Option<&str> {
        self.core.as_ref()?.camera_pan_context()
    }
    /// Whether world-space hover is allowed at the pointer's current physical location.
    pub fn pointer_world_hover_allowed(&self) -> bool {
        world_hover_allowed(self.pointer_over_widget, self.window_focused)
    }
    /// Position used by actual Core pointer events; this may be relative-motion-derived while
    /// the real window cursor position is unavailable under pointer lock.
    pub fn engine_cursor_position(&self) -> Option<Vec2> {
        self.cursor
    }
    pub fn cursor_lock_requested(&self) -> bool {
        self.cursor_lock_requested
    }
    pub fn cursor_lock_active(&self) -> bool {
        self.cursor_lock_active
    }
    pub fn relative_motion_this_frame(&self) -> Vec2 {
        self.relative_motion_this_frame
    }
}

fn world_hover_allowed(pointer_over_widget: Option<bool>, window_focused: Option<bool>) -> bool {
    pointer_over_widget != Some(true) && window_focused != Some(false)
}

#[derive(Clone, Debug)]
pub struct ValueEffect {
    pub kind: String,
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct FrameRecord {
    pub frame_index: u64,
    pub wall_ms: u128,
    pub tick: u32,
    pub replica_tick: Option<u32>,
    pub applied_seq: Vec<u32>,
    pub selection: Vec<u32>,
    /// Actual Core group membership closed in the same frame as selection and dispatch.
    pub groups: BTreeMap<u8, Vec<u32>>,
    /// Actual Core target-order state after this frame's input dispatch.
    pub pending_order_id: Option<String>,
    pub positions: Vec<(u32, f32, f32)>,
    pub results: Vec<serde_json::Value>,
    /// Actual sequence-assigned orders emitted by this frame's core intents.
    pub emitted_orders: Vec<serde_json::Value>,
    /// Named actions observed at the renderer-independent Core dispatch branch.
    pub named_dispatches: Vec<serde_json::Value>,
    /// Dispatch observations omitted after the per-step evidence cap.
    pub named_dispatches_truncated: usize,
    pub markers: Vec<serde_json::Value>,
    pub rings: Vec<serde_json::Value>,
    pub feedback: crate::feedback::FrameFeedback,
}

pub type FrameFeeds = crate::debug_feed::FramesFeed;

#[derive(Clone)]
struct DenseUnit {
    index: u32,
    footprint_radius: f32,
    capabilities: Vec<Capability>,
    gather_items: HashSet<u32>,
    target_slots: Option<SlotRelation>,
}
#[derive(Clone)]
struct DenseNode {
    index: u32,
    radius: f32,
    item: u32,
}
#[derive(Clone, Default)]
struct ContentIndex {
    units: HashMap<String, DenseUnit>,
    nodes: HashMap<String, DenseNode>,
    ranks: HashMap<String, usize>,
}

struct FrameWorld {
    units: Vec<UnitView>,
    nodes: Vec<NodeView>,
    own_slot: SlotId,
    orders: Vec<OrderDef>,
    rules: OrderRulesDef,
    input: InputMapDef,
    priority: Vec<String>,
    runtime_bindings: BTreeMap<String, Vec<Binding>>,
    interaction: InteractionSettings,
    camera_options: CameraOptions,
    map_size: Option<Vec2>,
    index: ContentIndex,
    producible: HashSet<(u32, String)>,
    jobs: HashSet<(u32, u64)>,
}
impl WorldView for FrameWorld {
    fn units(&self) -> &[UnitView] {
        &self.units
    }
    fn nodes(&self) -> &[NodeView] {
        &self.nodes
    }
    fn own_slot(&self) -> SlotId {
        self.own_slot
    }
    fn orders(&self) -> &[OrderDef] {
        &self.orders
    }
    fn order_rules(&self) -> &OrderRulesDef {
        &self.rules
    }
    fn input_map(&self) -> &InputMapDef {
        &self.input
    }
    fn runtime_binding_overrides(&self) -> &BTreeMap<String, Vec<Binding>> {
        &self.runtime_bindings
    }
    fn interaction_settings(&self) -> InteractionSettings {
        self.interaction
    }
    fn camera_options(&self) -> CameraOptions {
        self.camera_options.clone()
    }
    fn map_size(&self) -> Option<Vec2> {
        self.map_size
    }
    fn selection_priority(&self) -> &[String] {
        &self.priority
    }
    fn selection_rank(&self, content_id: &str) -> usize {
        self.index
            .ranks
            .get(content_id)
            .copied()
            .unwrap_or(usize::MAX)
    }
    fn can_gather(&self, unit: &UnitView, node: &NodeView) -> bool {
        self.index
            .units
            .get(&unit.content_id)
            .is_some_and(|unit_facts| {
                self.index
                    .nodes
                    .get(&node.content_id)
                    .is_some_and(|node_facts| unit_facts.gather_items.contains(&node_facts.item))
            })
    }
    fn can_produce(&self, unit: &UnitView, content_id: &str) -> bool {
        self.producible
            .contains(&(unit.id.0, content_id.to_owned()))
    }
    fn has_job(&self, unit: &UnitView, job: turts_net_protocol::JobId) -> bool {
        self.jobs.contains(&(unit.id.0, job.0))
    }
    fn can_weapon_target(&self, unit: &UnitView, target: &UnitView) -> bool {
        let relation = self
            .index
            .units
            .get(&unit.content_id)
            .and_then(|facts| facts.target_slots);
        match relation {
            Some(SlotRelation::Any) => true,
            Some(SlotRelation::Same) => unit.owner_slot == target.owner_slot,
            Some(SlotRelation::Other) => unit.owner_slot != target.owner_slot,
            None => false,
        }
    }
}

fn content_index(state: &ReplicaState, acquired: Option<&AcquiredPackages>) -> ContentIndex {
    let Some(content) = state
        .content
        .as_ref()
        .or_else(|| acquired.map(|a| &a.0.content))
    else {
        return ContentIndex::default();
    };
    let items: HashMap<_, _> = content
        .catalog
        .resources
        .keys()
        .enumerate()
        .map(|(index, id)| (id.to_string(), index as u32))
        .collect();
    let units = content
        .catalog
        .units
        .iter()
        .enumerate()
        .map(|(index, (id, unit))| {
            let radius = match unit.footprint {
                Footprint::Circle { radius } => radius,
            };
            let mut capabilities = Vec::new();
            if unit.movement.is_some() {
                capabilities.push(Capability::Movement);
            }
            if unit.weapon.is_some() {
                capabilities.push(Capability::Weapon);
            }
            if unit.gatherer.is_some() {
                capabilities.push(Capability::Gatherer);
            }
            if unit.producer.is_some() {
                capabilities.push(Capability::Producer);
            }
            if unit.research.is_some() {
                capabilities.push(Capability::Research);
            }
            let gather_items = unit
                .gatherer
                .as_ref()
                .map(|g| {
                    g.items
                        .iter()
                        .filter_map(|item| items.get(&item.to_string()).copied())
                        .collect()
                })
                .unwrap_or_default();
            (
                id.to_string(),
                DenseUnit {
                    index: index as u32,
                    footprint_radius: radius,
                    capabilities,
                    gather_items,
                    target_slots: unit.weapon.as_ref().map(|w| w.targets.slots),
                },
            )
        })
        .collect();
    let nodes = content
        .catalog
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, (id, node))| {
            items.get(&node.item.to_string()).map(|item| {
                (
                    id.to_string(),
                    DenseNode {
                        index: index as u32,
                        radius: node.radius,
                        item: *item,
                    },
                )
            })
        })
        .collect();
    let ranks = content
        .selection_priority
        .iter()
        .enumerate()
        .map(|(rank, id)| (id.clone(), rank))
        .collect();
    ContentIndex {
        units,
        nodes,
        ranks,
    }
}

fn frame_world(
    state: &ReplicaState,
    acquired: Option<&AcquiredPackages>,
    presented: &Presented,
    index: ContentIndex,
    preferences: &CorePreferences,
    camera_options: CameraOptions,
) -> Option<FrameWorld> {
    let (welcome, replica, content) = (
        state.welcome?,
        state.replica.as_ref()?,
        state
            .content
            .as_ref()
            .or_else(|| acquired.map(|a| &a.0.content))?,
    );
    let positions: HashMap<_, _> = presented
        .units
        .iter()
        .map(|unit| (unit.unit_id, unit.pos))
        .collect();
    let mut units = Vec::with_capacity(replica.units.len());
    for (&id, unit) in &replica.units {
        let Some(pos) = positions
            .get(&id)
            .or_else(|| unit.samples.back().map(|(_, p)| p))
        else {
            continue;
        };
        let facts = index.units.get(&unit.content_id);
        units.push(UnitView {
            id: UnitId(id),
            content_id: unit.content_id.clone(),
            owner_slot: SlotId(unit.owner_slot),
            position: Vec2::new(pos.x, pos.y),
            footprint_radius: facts.map_or(0.5, |f| f.footprint_radius),
            capabilities: facts.map_or_else(Vec::new, |f| f.capabilities.clone()),
            content_index: facts.map_or(u32::MAX, |f| f.index),
        });
    }
    units.sort_by_key(|unit| unit.id.0);
    let mut nodes = Vec::with_capacity(replica.nodes.len());
    for (&id, node) in &replica.nodes {
        let facts = index.nodes.get(&node.content_id);
        nodes.push(NodeView {
            id,
            content_id: node.content_id.clone(),
            position: Vec2::new(node.pos.x, node.pos.y),
            radius: facts.map_or(0.0, |f| f.radius),
            content_index: facts.map_or(u32::MAX, |f| f.index),
        });
    }
    nodes.sort_by_key(|node| node.id.0);
    let mut producible = HashSet::new();
    for (&producer, unit) in &replica.units {
        let Ok(content_id) = ContentId::parse(&unit.content_id) else {
            continue;
        };
        let Some(definition) = content.catalog.units.get(&content_id) else {
            continue;
        };
        if let Some(producer_def) = &definition.producer {
            producible.extend(
                producer_def
                    .recipes
                    .iter()
                    .map(|id| (producer, id.to_string())),
            );
        }
        if let Some(research_def) = &definition.research {
            producible.extend(
                research_def
                    .technologies
                    .iter()
                    .map(|id| (producer, id.to_string())),
            );
        }
    }
    let jobs = replica
        .economy
        .queues
        .iter()
        .flat_map(|queue| {
            queue
                .jobs
                .iter()
                .map(move |job| (queue.producer.0, job.id.0))
        })
        .collect();
    Some(FrameWorld {
        units,
        nodes,
        own_slot: SlotId(welcome.slot),
        orders: content
            .catalog
            .orders
            .iter()
            .map(|(id, definition)| {
                let mut definition = definition.clone();
                definition.id = id.to_string();
                definition
            })
            .collect(),
        rules: content.order_rules.clone(),
        input: content.input_map.clone(),
        priority: content.selection_priority.clone(),
        runtime_bindings: preferences.runtime_bindings.clone(),
        interaction: preferences.interaction,
        camera_options,
        map_size: acquired.map(|a| Vec2::new(a.0.map_size.0, a.0.map_size.1)),
        index,
        producible,
        jobs,
    })
}

fn mods_from_pressed(keys: &HashSet<String>) -> Vec<turts_mod_api::Mod> {
    let mut mods = Vec::new();
    if keys.iter().any(|key| key.starts_with("Shift")) {
        mods.push(turts_mod_api::Mod::Shift);
    }
    if keys.iter().any(|key| key.starts_with("Control")) {
        mods.push(turts_mod_api::Mod::Ctrl);
    }
    if keys.iter().any(|key| key.starts_with("Alt")) {
        mods.push(turts_mod_api::Mod::Alt);
    }
    mods
}
fn pointer_button(button: bevy::input::mouse::MouseButton) -> Option<turts_mod_api::Button> {
    use bevy::input::mouse::MouseButton as M;
    Some(match button {
        M::Left => turts_mod_api::Button::Left,
        M::Right => turts_mod_api::Button::Right,
        M::Middle => turts_mod_api::Button::Middle,
        M::Back => turts_mod_api::Button::Back,
        M::Forward => turts_mod_api::Button::Forward,
        _ => return None,
    })
}
fn over_widget(
    cursor: Vec2,
    panels: &CapturePanelQuery<'_, '_>,
    cameras: &Query<(Entity, &Camera), With<Camera3d>>,
) -> bool {
    capture_stack_at(cursor, panels, cameras, None).is_some()
}

fn capture_stack_at(
    cursor: Vec2,
    panels: &CapturePanelQuery<'_, '_>,
    cameras: &Query<(Entity, &Camera), With<Camera3d>>,
    target_camera: Option<Entity>,
) -> Option<(Entity, u32)> {
    panels
        .iter()
        .filter_map(|(entity, node, transform, visible, target, clip, stack)| {
            if !visible.get() || node.size().min_element() <= 0.0 {
                return None;
            }
            let camera_id = target.get()?;
            if target_camera.is_some_and(|expected| expected != camera_id) {
                return None;
            }
            let (_, camera) = cameras.iter().find(|(entity, _)| *entity == camera_id)?;
            if !camera.is_active {
                return None;
            }
            let point = cursor_in_ui_viewport(cursor, camera)?;
            if !node.contains_point(*transform, point) {
                return None;
            }
            let inside_clip = match clip {
                Some(CalculatedClip::FullyClipped) => false,
                Some(CalculatedClip::Rects(clips)) => clips.iter().all(|clip| {
                    clip.rect
                        .contains(clip.world_to_clip_local.transform_point2(point))
                }),
                None => true,
            };
            inside_clip.then_some((entity, stack.0))
        })
        .max_by_key(|(_, stack)| *stack)
}

fn minimap_local_point(
    node: &ComputedNode,
    transform: UiGlobalTransform,
    visible: bool,
    clip: Option<&CalculatedClip>,
    point: Vec2,
) -> Option<[f32; 2]> {
    if !visible || node.size().min_element() <= 0.0 || !node.contains_point(transform, point) {
        return None;
    }
    if let Some(clip) = clip {
        match clip {
            CalculatedClip::FullyClipped => return None,
            CalculatedClip::Rects(rects)
                if !rects.iter().all(|rect| {
                    rect.rect
                        .contains(rect.world_to_clip_local.transform_point2(point))
                }) =>
            {
                return None;
            }
            CalculatedClip::Rects(_) => {}
        }
    }
    let local = transform.try_inverse()?.transform_point2(point);
    let half = node.size() * 0.5;
    let normalized = Vec2::new(
        (local.x + half.x) / node.size().x,
        (local.y + half.y) / node.size().y,
    );
    (normalized.is_finite()
        && normalized.x >= 0.0
        && normalized.y >= 0.0
        && normalized.x <= 1.0
        && normalized.y <= 1.0)
        .then_some([normalized.x, normalized.y])
}

fn minimap_at(
    cursor: Vec2,
    minimaps: &MinimapInputQuery<'_, '_>,
    panels: &CapturePanelQuery<'_, '_>,
    cameras: &Query<(Entity, &Camera), With<Camera3d>>,
) -> Option<[f32; 2]> {
    minimaps
        .iter()
        .find_map(|(entity, node, transform, visible, target, clip)| {
            if !visible.get() {
                return None;
            }
            let camera_id = target.get()?;
            let (_, camera) = cameras.iter().find(|(entity, _)| *entity == camera_id)?;
            if !camera.is_active {
                return None;
            }
            let (topmost, _) = capture_stack_at(cursor, panels, cameras, Some(camera_id))?;
            if entity != topmost {
                return None;
            }
            let point = cursor_in_ui_viewport(cursor, camera)?;
            minimap_local_point(node, *transform, visible.get(), clip, point)
        })
}

#[derive(SystemParam)]
struct AdapterInputs<'w, 's> {
    windows: Query<
        'w,
        's,
        (
            Entity,
            &'static Window,
            Option<&'static bevy::window::CursorOptions>,
        ),
        With<PrimaryWindow>,
    >,
    window_events: MessageReader<'w, 's, WindowEvent>,
    mouse_motion: MessageReader<'w, 's, MouseMotion>,
    cursor_feed: Res<'w, CursorFeed>,
    panels: CapturePanelQuery<'w, 's>,
    settings_menus: Query<'w, 's, (), With<crate::client_shell::SettingsMenuOverlay>>,
    minimaps: MinimapInputQuery<'w, 's>,
    cameras: Query<'w, 's, (Entity, &'static Camera), With<Camera3d>>,
    camera_query: Query<'w, 's, (&'static mut Transform, &'static mut Projection), With<Camera3d>>,
}

#[derive(SystemParam)]
struct AdapterOutputs<'w> {
    pending: ResMut<'w, PendingCommands>,
    ui_selection: ResMut<'w, Selection>,
    selection_requests: ResMut<'w, crate::ui_shared::SelectionRequests>,
    input_events: ResMut<'w, CoreInputQueue>,
    core_effects: ResMut<'w, CoreEffects>,
    order_marker_feed: ResMut<'w, crate::commands::OrderMarkerFeed>,
    status: Res<'w, crate::commands::CommandSendStatus>,
    selection_feed: ResMut<'w, SelectionFeed>,
    groups_feed: ResMut<'w, GroupsFeed>,
    focus_feed: ResMut<'w, FocusFeed>,
    player_feed: ResMut<'w, PlayerIdFeed>,
    player_id: Option<Res<'w, crate::PlayerId>>,
    binding_editor: Option<Res<'w, BindingEditorState>>,
}

#[allow(clippy::too_many_arguments)]
fn drive_core_adapter(
    mut runtime: ResMut<CoreRuntime>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    presented: Res<Presented>,
    preferences: Res<CorePreferences>,
    mut camera_configuration: ResMut<crate::camera::CameraConfiguration>,
    mut input: AdapterInputs,
    mut output: AdapterOutputs,
) {
    // A no-world, focus, or session early return must not let last frame's intent
    // masquerade as a click produced by this frame when visuals observe a reset.
    output.order_marker_feed.begin_interaction_frame();
    runtime.order_intents_this_frame.clear();
    if runtime.content_index.is_none() && (state.content.is_some() || acquired.is_some()) {
        runtime.content_index = Some(content_index(&state, acquired.as_deref()));
    }
    if runtime.content_index.is_none() {
        return;
    }
    if camera_configuration.options.authored.is_none()
        && let Some(content) = state
            .content
            .as_ref()
            .or_else(|| acquired.as_ref().map(|value| &value.0.content))
    {
        camera_configuration.options.authored = content.camera.clone();
    }
    let Some(world) = frame_world(
        &state,
        acquired.as_deref(),
        &presented,
        runtime.content_index.clone().unwrap_or_default(),
        &preferences,
        camera_configuration.options.clone(),
    ) else {
        return;
    };
    if runtime.core.is_none() {
        if !state
            .replica
            .as_ref()
            .is_some_and(|replica| replica.snapshot_tick.is_some())
        {
            return;
        }
        let Some(map_size) = world.map_size() else {
            return;
        };
        let initial_camera = camera_configuration.initial_pose.unwrap_or(CameraState {
            look_at: map_size * 0.5,
            height: 40.0,
            pitch: (30.0f32 / 40.0).atan(),
            yaw: 0.0,
            vertical_fov: std::f32::consts::FRAC_PI_4,
        });
        runtime.core = Some(Core::new(initial_camera));
    }
    let mut events = Vec::new();
    let (lock_requested, primary_window_size) = input
        .windows
        .iter()
        .next()
        .map(|(_, window, cursor)| {
            (
                cursor
                    .is_some_and(|cursor| cursor.grab_mode == bevy::window::CursorGrabMode::Locked),
                Vec2::new(window.width(), window.height()),
            )
        })
        .unwrap_or((false, Vec2::ZERO));
    let relative_motion = input
        .mouse_motion
        .read()
        .fold(Vec2::ZERO, |sum, motion| sum + motion.delta);
    runtime.cursor_lock_requested = lock_requested;
    let lock_active = input.cursor_feed.lock_active == Some(true);
    runtime.cursor_lock_active = lock_active;
    runtime.relative_motion_this_frame = if lock_active {
        relative_motion
    } else {
        Vec2::ZERO
    };
    let binding_capture_active = output
        .binding_editor
        .as_deref()
        .is_some_and(|editor| editor.capturing.is_some());
    if binding_capture_active || !input.settings_menus.is_empty() {
        events.push(InputEvent::UiCapture {
            allow_shell: !binding_capture_active,
        });
    }
    let starting_cursor = if input.cursor_feed.lock_active == Some(false) {
        input
            .windows
            .iter()
            .next()
            .and_then(|(_, window, _)| window.cursor_position())
    } else {
        runtime.cursor
    };
    let mut current_cursor = (runtime.window_focused != Some(false))
        .then_some(starting_cursor)
        .flatten();
    for event in input.window_events.read() {
        let primary = input.windows.iter().next().map(|(entity, _, _)| entity);
        match event {
            WindowEvent::CursorMoved(value) if Some(value.window) == primary => {
                if runtime.window_focused != Some(false) && !lock_active {
                    current_cursor = Some(value.position);
                    let over = over_widget(value.position, &input.panels, &input.cameras);
                    runtime.pointer_over_widget = Some(over);
                    events.push(InputEvent::PointerMove {
                        px: value.position,
                        over_widget: over,
                    });
                    if runtime.minimap_pointer == Some(turts_mod_api::Button::Left)
                        && let Some(local) = minimap_at(
                            value.position,
                            &input.minimaps,
                            &input.panels,
                            &input.cameras,
                        )
                    {
                        events.push(InputEvent::Minimap {
                            button: turts_mod_api::Button::Left,
                            local,
                        });
                    }
                }
            }
            WindowEvent::CursorLeft(value) if Some(value.window) == primary => {
                current_cursor = None;
                runtime.pointer_over_widget = None;
                runtime.minimap_pointer = None;
                events.push(InputEvent::PointerLeft);
            }
            WindowEvent::KeyboardInput(value) if Some(value.window) == primary && !value.repeat => {
                let code = format!("{:?}", value.key_code);
                match value.state {
                    ButtonState::Pressed => {
                        runtime.pressed_keys.insert(code.clone());
                        events.push(InputEvent::KeyDown {
                            code,
                            mods: mods_from_pressed(&runtime.pressed_keys),
                        });
                    }
                    ButtonState::Released => {
                        events.push(InputEvent::KeyUp { code: code.clone() });
                        runtime.pressed_keys.remove(&code);
                    }
                }
            }
            WindowEvent::MouseButtonInput(value) if Some(value.window) == primary => {
                if value.state == ButtonState::Released
                    && current_cursor.is_none()
                    && pointer_button(value.button) == runtime.minimap_pointer
                {
                    runtime.minimap_pointer = None;
                }
                if let (Some(button), Some(px)) = (pointer_button(value.button), current_cursor) {
                    let over = over_widget(px, &input.panels, &input.cameras);
                    let minimap = minimap_at(px, &input.minimaps, &input.panels, &input.cameras);
                    if value.state == ButtonState::Pressed {
                        if let Some(local) = minimap {
                            runtime.minimap_pointer = Some(button);
                            if button == turts_mod_api::Button::Left {
                                events.push(InputEvent::Minimap { button, local });
                            }
                            continue;
                        }
                    } else if runtime.minimap_pointer == Some(button) {
                        runtime.minimap_pointer = None;
                        if let Some(local) = minimap {
                            events.push(InputEvent::Minimap { button, local });
                        }
                        continue;
                    }
                    match value.state {
                        ButtonState::Pressed => events.push(InputEvent::PointerDown {
                            button,
                            px,
                            over_widget: over,
                        }),
                        ButtonState::Released => events.push(InputEvent::PointerUp {
                            button,
                            px,
                            over_widget: over,
                        }),
                    }
                }
            }
            WindowEvent::MouseWheel(value) if Some(value.window) == primary => {
                let over_ui = current_cursor
                    .is_some_and(|cursor| over_widget(cursor, &input.panels, &input.cameras));
                if !over_ui
                    && !runtime
                        .pressed_keys
                        .iter()
                        .any(|key| key.starts_with("Control"))
                {
                    events.push(InputEvent::Wheel { delta: value.y });
                }
            }
            WindowEvent::WindowFocused(value) if Some(value.window) == primary => {
                output.focus_feed.0 = value.focused;
                runtime.window_focused = Some(value.focused);
                if !value.focused {
                    runtime.pressed_keys.clear();
                    runtime.minimap_pointer = None;
                    current_cursor = None;
                    runtime.pointer_over_widget = None;
                    events.push(InputEvent::FocusLost);
                }
            }
            WindowEvent::KeyboardFocusLost(_) => {
                output.focus_feed.0 = false;
                runtime.pressed_keys.clear();
                runtime.minimap_pointer = None;
                current_cursor = None;
                runtime.pointer_over_widget = None;
                events.push(InputEvent::FocusLost);
            }
            _ => {}
        }
    }
    if lock_active
        && runtime.window_focused != Some(false)
        && relative_motion.is_finite()
        && relative_motion != Vec2::ZERO
    {
        let start = current_cursor.or_else(|| {
            input
                .windows
                .iter()
                .next()
                .and_then(|(_, window, _)| window.cursor_position())
        });
        if let Some(position) = start
            && primary_window_size.is_finite()
            && primary_window_size.x > 0.0
            && primary_window_size.y > 0.0
        {
            current_cursor = Some(
                (position + relative_motion)
                    .clamp(Vec2::ZERO, primary_window_size - Vec2::splat(1.0)),
            );
            let position = current_cursor.expect("the virtual cursor was just assigned");
            let over = over_widget(position, &input.panels, &input.cameras);
            runtime.pointer_over_widget = Some(over);
            events.push(InputEvent::PointerMove {
                px: position,
                over_widget: over,
            });
        }
    }
    events.append(&mut output.input_events.0);
    if runtime.window_focused != Some(false)
        && let Some(px) = current_cursor
        && !events
            .iter()
            .any(|event| matches!(event, InputEvent::PointerMove { .. }))
    {
        // Re-evaluate capture while the physical pointer is stationary. UI visibility and
        // clipping can change under it without a new OS CursorMoved event.
        let over = over_widget(px, &input.panels, &input.cameras);
        if runtime.pointer_over_widget != Some(over) {
            runtime.pointer_over_widget = Some(over);
            events.push(InputEvent::PointerMove {
                px,
                over_widget: over,
            });
        }
    }
    runtime.cursor = current_cursor;
    for unit in std::mem::take(&mut output.selection_requests.0) {
        events.push(InputEvent::UiAction {
            name: format!("ui_select:{unit}"),
        });
    }
    #[cfg(target_family = "wasm")]
    runtime.pending_sequences.extend(drain_harness_sequences());
    if output.ui_selection.0 != runtime.last_ui_selection
        && output.ui_selection.0 != runtime.last_core_primary
        && let Some(id) = output.ui_selection.0
    {
        events.push(InputEvent::UiAction {
            name: format!("ui_select:{id}"),
        });
    }
    let view = input
        .windows
        .iter()
        .next()
        .map(|(_, window, _)| Viewport {
            size: Vec2::new(window.width(), window.height()),
            scale_factor: window.scale_factor(),
        })
        .unwrap_or(Viewport {
            size: Vec2::new(1280.0, 720.0),
            scale_factor: 1.0,
        });
    let started = runtime.started.get_or_insert_with(Instant::now);
    let now_ms = started.elapsed().as_millis() as u64;
    let (effects, primary, camera_state) = {
        let core = runtime.core.as_mut().expect("initialized above");
        let effects = core.step(&events, &world, &view, now_ms);
        let primary = core.selected_units().first().map(|id| id.0);
        (effects, primary, core.camera_state())
    };
    output.core_effects.0 = effects.clone();
    runtime.last_effects.clear();
    runtime.markers.clear();
    for effect in effects {
        match effect {
            Effect::Send(order) if !output.status.reconnect_required => {
                apply_send(order, &mut runtime, &state, &mut output);
            }
            Effect::SelectionChanged => {}
            Effect::Marker { point, kind } => {
                apply_marker(point, kind, &mut runtime);
            }
            Effect::Notice { text_key, units } => {
                apply_notice(text_key, units, &mut runtime);
            }
            Effect::Sound { id, category } => {
                apply_sound(id, category, &mut runtime);
            }
            Effect::Cursor { context } => {
                apply_cursor(context, &mut runtime);
            }
            Effect::ShellAction { action } => {
                apply_shell_action(action, &mut runtime);
            }
            Effect::GroupsChanged | Effect::CameraChanged => {}
            Effect::Send(_) => {}
        }
    }
    output.ui_selection.0 = primary;
    runtime.last_core_primary = primary;
    runtime.last_ui_selection = output.ui_selection.0;
    output.selection_feed.0 = runtime
        .core
        .as_ref()
        .map(|core| core.selected_units().iter().map(|id| id.0).collect())
        .unwrap_or_default();
    if let Some(core) = runtime.core.as_ref() {
        output.groups_feed.0 = (1..=9)
            .map(|group| {
                (
                    group,
                    core.group_members(group).iter().map(|id| id.0).collect(),
                )
            })
            .collect();
    }
    output.player_feed.0 = output.player_id.as_deref().map(|player| player.0);
    let (transform, projection) = camera_components(camera_state);
    for (mut current_transform, mut current_projection) in &mut input.camera_query {
        *current_transform = transform;
        *current_projection = projection.clone();
    }
}

fn apply_frame_clear(mut feeds: ResMut<FrameFeeds>, mut runtime: ResMut<CoreRuntime>) {
    if CLEAR_FRAMES_REQUESTED.swap(false, Ordering::AcqRel) {
        feeds.recent.clear();
        feeds.retained_input.clear();
        runtime.pending_sequences.clear();
    }
}

pub fn result_values(results: &[CommandResult]) -> Vec<serde_json::Value> {
    let mut output = Vec::new();
    for result in results {
        if let CommandResult::Order {
            seq,
            tick,
            accepted,
            refused,
            envelope,
            ..
        } = result
        {
            output.extend(accepted.iter().map(
                |unit| serde_json::json!({"seq":seq,"tick":tick,"unit":unit.0,"status":"accepted"}),
            ));
            output.extend(refused.iter().map(|refusal|serde_json::json!({"seq":seq,"tick":tick,"unit":refusal.unit.0,"status":"refused","reason":refusal.reason.as_str()})));
            if accepted.is_empty()
                && refused.is_empty()
                && let Some(reason) = envelope
            {
                output.push(serde_json::json!({"seq":seq,"tick":tick,"status":"rejected","reason":reason.as_str()}));
            }
        }
    }
    output
}

// A Bevy system reads the frame, replica, presentation, result, and widget feeds at Last.
#[allow(clippy::too_many_arguments)]
fn close_core_frame(
    mut feeds: ResMut<FrameFeeds>,
    mut runtime: ResMut<CoreRuntime>,
    presented: Res<Presented>,
    results: Res<CommandResults>,
    state: Res<ReplicaState>,
    mut results_feed: ResMut<ResultsFeed>,
    order_marker_feed: Res<crate::commands::OrderMarkerFeed>,
    markers: Res<MarkersShown>,
    rings: Res<RingsShown>,
    feedback: Res<crate::feedback::FrameFeedback>,
) {
    if state.welcome.is_none() {
        return;
    }
    feeds.frame_index = feeds.frame_index.wrapping_add(1);
    let selection = runtime
        .core
        .as_ref()
        .map(|core| core.selected_units().iter().map(|unit| unit.0).collect())
        .unwrap_or_default();
    let groups = runtime
        .core
        .as_ref()
        .map(|core| {
            (1..=9)
                .map(|group| {
                    (
                        group,
                        core.group_members(group)
                            .iter()
                            .map(|unit| unit.0)
                            .collect(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let (named_dispatches, named_dispatches_truncated) = runtime
        .core
        .as_ref()
        .map(|core| {
            (
                core.dispatch_records()
                    .iter()
                    .map(|dispatch| {
                        serde_json::json!({
                            "input_index": dispatch.input_index,
                            "origin": dispatch.origin,
                            "code": dispatch.code,
                            "matched_action": dispatch.matched_action,
                            "matched_order": dispatch.matched_order,
                        })
                    })
                    .collect(),
                core.dispatch_records_truncated(),
            )
        })
        .unwrap_or_default();
    let pending_order_id = runtime
        .core
        .as_ref()
        .and_then(|core| core.pending_order_id().map(str::to_owned));
    let new_results = results.0.get(feeds.last_result_count..).unwrap_or_default();
    let result_records = result_values(new_results);
    results_feed.0.extend(result_records.iter().cloned());
    feeds.last_result_count = results.0.len();
    let current_intents = std::mem::take(&mut runtime.order_intents_this_frame);
    let emitted_orders = emitted_order_values(&order_marker_feed, &current_intents);
    let record = FrameRecord {
        frame_index: feeds.frame_index,
        wall_ms: presented.frame_ns / 1_000_000,
        tick: presented.latest_tick,
        replica_tick: state
            .replica
            .as_ref()
            .and_then(|replica| replica.last_applied_tick),
        applied_seq: std::mem::take(&mut runtime.pending_sequences),
        selection,
        groups,
        pending_order_id,
        positions: presented
            .units
            .iter()
            .map(|unit| (unit.unit_id, unit.pos.x, unit.pos.y))
            .collect(),
        results: result_records,
        emitted_orders,
        named_dispatches,
        named_dispatches_truncated,
        markers: markers.0.clone(),
        rings: rings.0.clone(),
        feedback: feedback.clone(),
    };
    feeds.recent.push_back(record.clone());
    while feeds.recent.len() > 64 {
        feeds.recent.pop_front();
    }
    // Historical name: this bounded causal history retains physical-input
    // frames and later result-bearing frames needed to connect outcomes back
    // to their emitted command sequence.
    if !record.applied_seq.is_empty()
        || !record.results.is_empty()
        || !record.feedback.alerts.is_empty()
        || !record.named_dispatches.is_empty()
    {
        feeds.retained_input.push_back(record);
        while feeds.retained_input.len() > 4096 {
            feeds.retained_input.pop_front();
        }
    }
}

fn emitted_order_values(
    feed: &crate::commands::OrderMarkerFeed,
    current_intents: &[u64],
) -> Vec<serde_json::Value> {
    let current_intents: HashSet<_> = current_intents.iter().copied().collect();
    feed.sent
        .iter()
        .filter(|fragment| current_intents.contains(&fragment.intent_id))
        .map(|fragment| {
            serde_json::json!({
                "intent_id": fragment.intent_id,
                "seq": fragment.seq,
                "order": &fragment.order.order,
                "units": fragment.order.units.iter().map(|unit| unit.0).collect::<Vec<_>>(),
                "target": &fragment.order.target,
            })
        })
        .collect()
}

#[cfg(target_family = "wasm")]
fn drain_harness_sequences() -> Vec<u32> {
    use js_sys::wasm_bindgen::JsValue;
    let global = js_sys::global();
    let Ok(value) = js_sys::Reflect::get(&global, &JsValue::from_str("__turts_seq_queue")) else {
        return Vec::new();
    };
    let array = js_sys::Array::from(&value);
    let values = array
        .iter()
        .filter_map(|value| {
            js_sys::Reflect::get(&value, &JsValue::from_str("seq"))
                .ok()?
                .as_f64()
        })
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= u32::MAX as f64)
        .map(|value| value as u32)
        .collect();
    let _ = js_sys::Reflect::set(
        &global,
        &JsValue::from_str("__turts_seq_queue"),
        &js_sys::Array::new(),
    );
    values
}

pub struct CoreAdapterPlugin;
#[derive(Resource, Default)]
struct FocusFeedWindow(Option<Entity>);

impl Plugin for CoreAdapterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CoreRuntime>()
            .init_resource::<CorePreferences>()
            .init_resource::<CoreEffects>()
            .init_resource::<crate::commands::OrderMarkerFeed>()
            .init_resource::<crate::camera::CameraConfiguration>()
            .init_resource::<crate::feedback::FrameFeedback>()
            .init_resource::<CoreInputQueue>()
            .init_resource::<FrameFeeds>()
            .add_message::<WindowEvent>()
            .add_message::<MouseMotion>()
            .init_resource::<Selection>()
            .init_resource::<crate::ui_shared::SelectionRequests>()
            .init_resource::<SelectionFeed>()
            .init_resource::<GroupsFeed>()
            .init_resource::<ResultsFeed>()
            .init_resource::<CursorFeed>()
            .init_resource::<crate::debug_feed::UiSnapshotFeed>()
            .init_resource::<NoticesFeed>()
            .init_resource::<crate::debug_feed::AlertsFeed>()
            .init_resource::<AudioFeed>()
            .init_resource::<ContextMenuFeed>()
            .init_resource::<FocusFeed>()
            .init_resource::<FocusFeedWindow>()
            .init_resource::<PlayerIdFeed>()
            .init_resource::<crate::debug_feed::AlertPingsFeed>()
            .init_resource::<MarkersShown>()
            .init_resource::<RingsShown>()
            .add_systems(PreUpdate, apply_frame_clear)
            .add_systems(
                Update,
                (refresh_actual_pointer_lock, update_primary_window_focus).before(CoreAdapterSet),
            )
            .add_systems(
                Update,
                drive_core_adapter
                    .in_set(CoreAdapterSet)
                    .before(UiPresentationSet)
                    .after(crate::present::update_presented)
                    .after(setup_scene),
            )
            .add_systems(
                Last,
                close_core_frame
                    .in_set(FrameCloseSet)
                    .before(crate::debug::rebuild_debug_state),
            );
    }
}

fn update_primary_window_focus(
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut events: MessageReader<WindowEvent>,
    mut focus: ResMut<FocusFeed>,
    mut focus_window: ResMut<FocusFeedWindow>,
) {
    let Some((primary, window)) = windows.iter().next() else {
        events.read().for_each(drop);
        focus_window.0 = None;
        return;
    };
    if focus_window.0 != Some(primary) {
        focus.0 = window.focused;
        focus_window.0 = Some(primary);
    }
    for event in events.read() {
        match event {
            WindowEvent::WindowFocused(value) if value.window == primary => {
                focus.0 = value.focused;
            }
            WindowEvent::KeyboardFocusLost(_) => focus.0 = false,
            _ => {}
        }
    }
}

#[cfg(test)]
mod adapter_tests;
