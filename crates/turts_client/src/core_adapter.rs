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
                runtime.last_effects.push(ValueEffect {
                    kind: "order_intent".into(),
                    value: serde_json::json!({"units":order.units.iter().map(|unit| unit.0).collect::<Vec<_>>() }),
                });
                let marker_kind = state
                    .content
                    .as_ref()
                    .and_then(|content| {
                        ContentId::parse(&order.order)
                            .ok()
                            .and_then(|id| content.catalog.orders.get(&id))
                    })
                    .and_then(|order_def| match order_def.behaviour {
                        turts_mod_api::Behaviour::Move => Some("move"),
                        turts_mod_api::Behaviour::Attack => Some("attack"),
                        turts_mod_api::Behaviour::Gather => Some("gather"),
                        _ => None,
                    });
                let intent_id = runtime.next_order_intent_id;
                runtime.next_order_intent_id = intent_id.wrapping_add(1);
                runtime.order_intents_this_frame.push(intent_id);
                if let Some(kind) = marker_kind {
                    output
                        .order_marker_feed
                        .intents
                        .push(crate::commands::OrderMarkerIntent {
                            id: intent_id,
                            kind: kind.to_owned(),
                            order: order.clone(),
                        });
                    runtime.markers.push(serde_json::json!({
                        "intent_id": intent_id,
                        "kind": kind,
                        "order": &order.order,
                        "units": order.units.iter().map(|unit| unit.0).collect::<Vec<_>>(),
                        "target": &order.target,
                    }));
                }
                crate::commands::submit_visual_order(&mut output.pending, order, intent_id);
            }
            Effect::SelectionChanged => {}
            Effect::Marker { point, kind } => {
                let value = serde_json::json!({"x":point.x,"y":point.y,"kind":kind});
                runtime.markers.push(value.clone());
                runtime.last_effects.push(ValueEffect {
                    kind: "marker".into(),
                    value,
                });
            }
            Effect::Notice { text_key, units } => {
                let value = serde_json::json!({"text_key":text_key,"units":units.iter().map(|id|id.0).collect::<Vec<_>>()});
                runtime.last_effects.push(ValueEffect {
                    kind: "notice".into(),
                    value,
                });
            }
            Effect::Sound { id, category } => {
                let value = serde_json::json!({"id":id,"category":category});
                runtime.last_effects.push(ValueEffect {
                    kind: "sound".into(),
                    value: value.clone(),
                });
            }
            Effect::Cursor { context } => {
                runtime.last_effects.push(ValueEffect {
                    kind: "cursor".into(),
                    value: serde_json::json!({"context":context}),
                });
            }
            Effect::ShellAction { action } => {
                let value = serde_json::json!({"action":action});
                runtime.last_effects.push(ValueEffect {
                    kind: "shell_action".into(),
                    value: value.clone(),
                });
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
mod adapter_tests {
    use super::*;
    use crate::{
        acquire::Acquired,
        commands::{CommandSendStatus, OrderMarkerFeed, SentOrderFragment},
        feedback::FrameFeedback,
        ingest::WelcomeInfo,
        replica::UnitReplica,
    };
    use bevy::{
        input::{
            keyboard::{Key, KeyCode, KeyboardInput},
            mouse::{MouseButton, MouseButtonInput},
        },
        window::{PrimaryWindow, Window},
    };
    use std::{
        collections::{HashMap, VecDeque},
        path::PathBuf,
        sync::Mutex,
    };
    use turts_mod_api::{CameraDef, CameraStart};
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
    use turts_net_protocol::{OrderEnvelope, OrderTarget, Pos, QueueMode, UnitId};

    #[test]
    fn minimap_pointer_uses_transformed_visible_widget_coordinates_and_clips() {
        let node = ComputedNode {
            size: Vec2::new(200.0, 100.0),
            ..default()
        };
        let transform = UiGlobalTransform::from(bevy::math::Affine2::from_scale_angle_translation(
            Vec2::splat(2.0),
            0.0,
            Vec2::new(500.0, 400.0),
        ));

        assert_eq!(
            minimap_local_point(&node, transform, true, None, Vec2::new(500.0, 400.0)),
            Some([0.5, 0.5])
        );
        assert_eq!(
            minimap_local_point(&node, transform, true, None, Vec2::new(600.0, 350.0)),
            Some([0.75, 0.25]),
            "screen top-down coordinates must pass through to the core's asymmetric world mapping"
        );
        assert_eq!(
            minimap_local_point(&node, transform, false, None, Vec2::new(500.0, 400.0)),
            None,
            "hidden authored widgets must not capture pointer input"
        );
        assert_eq!(
            minimap_local_point(
                &node,
                transform,
                true,
                Some(&CalculatedClip::FullyClipped),
                Vec2::new(500.0, 400.0),
            ),
            None,
            "fully clipped authored widgets must not capture pointer input"
        );
        let partial_clip = CalculatedClip::Rects(
            vec![CalculatedClipRect {
                rect: Rect::from_corners(Vec2::ZERO, Vec2::new(450.0, 800.0)),
                world_to_clip_local: bevy::math::Affine2::IDENTITY,
            }]
            .into(),
        );
        assert_eq!(
            minimap_local_point(
                &node,
                transform,
                true,
                Some(&partial_clip),
                Vec2::new(500.0, 400.0),
            ),
            None,
            "the visible clip chain must constrain minimap pointer input"
        );
    }

    static ADAPTER_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        ADAPTER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn frame_order_evidence_uses_only_sequence_assigned_fragments_for_this_core_intent() {
        let order = OrderEnvelope {
            order: "muster:move".into(),
            units: vec![UnitId(7)],
            target: OrderTarget::Point(Pos { x: 8.0, y: 9.0 }),
            queue: QueueMode::Replace,
            group: 0,
        };
        let mut feed = OrderMarkerFeed::default();
        feed.sent.push_back(SentOrderFragment {
            intent_id: 41,
            seq: 101,
            order: order.clone(),
        });
        feed.sent.push_back(SentOrderFragment {
            intent_id: 40,
            seq: 100,
            order,
        });

        let emitted = emitted_order_values(&feed, &[41]);

        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0]["intent_id"], 41);
        assert_eq!(emitted[0]["seq"], 101);
        assert_eq!(emitted[0]["units"], serde_json::json!([7]));
        assert_eq!(emitted[0]["order"], "muster:move");
    }

    #[test]
    fn world_hover_respects_ui_pointer_capture_and_focus_loss() {
        let cases = [
            (Some(true), Some(true), false, "captured by UI"),
            (Some(false), Some(false), false, "window focus lost"),
            (Some(false), Some(true), true, "uncaptured focused pointer"),
            (
                None,
                None,
                true,
                "initial pointer state before capture is known",
            ),
        ];
        for (pointer_over_widget, focused, expected, context) in cases {
            assert_eq!(
                world_hover_allowed(pointer_over_widget, focused),
                expected,
                "world hover eligibility when {context}"
            );
        }
    }

    fn fixture_app() -> (App, Entity, Vec2) {
        fixture_app_with_plugins(false, false)
    }

    fn fixture_app_with_client_shell() -> (App, Entity, Vec2) {
        fixture_app_with_plugins(false, true)
    }

    fn acme_order_app() -> (App, Entity, Vec2) {
        acme_order_app_with_widgets(false)
    }

    fn acme_order_app_with_widgets(include_widgets: bool) -> (App, Entity, Vec2) {
        let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/acme");
        let archive =
            PackageArchive::parse(&pack_dir(&package).expect("pack Acme")).expect("parse Acme");
        let mut acquired = if include_widgets {
            crate::acquire::build_acquired(std::slice::from_ref(&archive), String::new())
                .expect("acquire Acme presentation")
        } else {
            let content =
                crate::content::ClientContent::from_archives(std::slice::from_ref(&archive))
                    .expect("resolve Acme content");
            Acquired {
                lockfile_hash: String::new(),
                visuals: HashMap::new(),
                map_size: (64.0, 64.0),
                unit_names: HashMap::new(),
                content,
                presentation: Default::default(),
                verified_root_mod: None,
            }
        };
        if include_widgets {
            acquired
                .presentation
                .interface
                .as_mut()
                .expect("Acme interface")
                .root
                .push(turts_mod_api::Widget {
                    id: "test_guide_toggle".into(),
                    at: Some(turts_mod_api::At {
                        anchor: turts_mod_api::Anchor::TopLeft,
                        offset: (200.0, 16.0),
                        pivot: None,
                    }),
                    size: turts_mod_api::Size {
                        w: turts_mod_api::Dimension::Px(120.0),
                        h: turts_mod_api::Dimension::Px(28.0),
                    },
                    layer: 3,
                    visible: vec![turts_mod_api::Predicate::GuideOpen],
                    tooltip: None,
                    widget: turts_mod_api::Primitive::Button {
                        icon: None,
                        label: Some(turts_mod_api::TextValue::Literal("Guide".into())),
                        action: turts_mod_api::ButtonAction::Input("toggle_guide".into()),
                        hotkey_label: true,
                    },
                });
        }
        let content = acquired.content.clone();
        let mut replica = crate::replica::Replica::new(20);
        replica.units.insert(
            1,
            UnitReplica {
                content_id: "acme:scout".into(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::from([(1, Pos { x: 4.0, y: 4.0 })]),
            },
        );
        replica.last_applied_tick = Some(1);
        replica.snapshot_tick = Some(1);
        let state = ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 1,
                tick_hz: 20,
                match_id: 1,
            }),
            content: Some(content.clone()),
            config: Default::default(),
        };
        let camera = content.camera.as_ref().map_or(
            CameraState {
                look_at: Vec2::new(32.0, 32.0),
                height: 40.0,
                pitch: (30.0f32 / 40.0).atan(),
                yaw: 0.0,
                vertical_fov: std::f32::consts::FRAC_PI_4,
            },
            |definition| {
                crate::camera::camera_state_from_definition(definition, Vec2::new(64.0, 64.0), None)
            },
        );
        let px = Core::new(camera).project_world_to_screen(
            Vec2::new(4.0, 4.0),
            &Viewport {
                size: Vec2::new(1280.0, 720.0),
                scale_factor: 1.0,
            },
        );
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(CoreAdapterPlugin)
            .add_plugins(crate::camera::ClientCameraPlugin);
        if include_widgets {
            app.add_plugins(crate::widgets::WidgetsPlugin);
        }
        app.insert_resource(state)
            .insert_resource(AcquiredPackages(acquired))
            .insert_resource(Presented {
                frame_ns: 1,
                t: 1.0,
                latest_tick: 1,
                units: vec![crate::present::PresentedUnit {
                    unit_id: 1,
                    content_id: "acme:scout".into(),
                    pos: Pos { x: 4.0, y: 4.0 },
                    lo_tick: 1,
                    hi_tick: 1,
                    fraction: 0.0,
                }],
            })
            .init_resource::<PendingCommands>()
            .init_resource::<CommandSendStatus>()
            .init_resource::<CommandResults>();
        if include_widgets {
            app.insert_resource(crate::join::Phase::Joined { slot: 0 })
                .insert_resource(bevy::asset::Assets::<bevy::prelude::Image>::default())
                .insert_resource(bevy::asset::Assets::<bevy::prelude::Font>::default());
        }
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.finish();
        app.cleanup();
        (app, window, px)
    }

    fn fixture_app_with_ui(include_ui: bool) -> (App, Entity, Vec2) {
        fixture_app_with_plugins(include_ui, false)
    }

    fn fixture_app_with_plugins(
        include_ui: bool,
        include_client_shell: bool,
    ) -> (App, Entity, Vec2) {
        fixture_app_with_plugins_and_persist(include_ui, include_client_shell, None)
    }

    fn fixture_app_with_persistence(root: std::path::PathBuf) -> (App, Entity, Vec2) {
        fixture_app_with_plugins_and_persist(false, false, Some(root))
    }

    fn fixture_app_with_plugins_and_persist(
        include_ui: bool,
        include_client_shell: bool,
        persist_root: Option<std::path::PathBuf>,
    ) -> (App, Entity, Vec2) {
        let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
        let archive =
            PackageArchive::parse(&pack_dir(&package).expect("pack Forge")).expect("parse Forge");
        let content =
            crate::content::ClientContent::from_archives(&[archive]).expect("resolve Forge");
        let acquired = Acquired {
            lockfile_hash: String::new(),
            visuals: HashMap::new(),
            map_size: (64.0, 64.0),
            unit_names: HashMap::new(),
            content: content.clone(),
            presentation: Default::default(),
            verified_root_mod: None,
        };
        let mut replica = crate::replica::Replica::new(20);
        for (id, x) in [(1, 32.0), (2, 33.0)] {
            replica.units.insert(
                id,
                UnitReplica {
                    content_id: "forge:drone".into(),
                    owner_slot: 0,
                    facing: None,
                    samples: VecDeque::from([(1, Pos { x, y: 32.0 })]),
                },
            );
        }
        replica.last_applied_tick = Some(1);
        replica.snapshot_tick = Some(1);
        let state = ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 1,
                tick_hz: 20,
                match_id: 1,
            }),
            content: Some(content.clone()),
            config: Default::default(),
        };
        let camera = content.camera.as_ref().map_or(
            CameraState {
                look_at: Vec2::new(32.0, 32.0),
                height: 40.0,
                pitch: (30.0f32 / 40.0).atan(),
                yaw: 0.0,
                vertical_fov: std::f32::consts::FRAC_PI_4,
            },
            |definition| {
                crate::camera::camera_state_from_definition(definition, Vec2::new(64.0, 64.0), None)
            },
        );
        let px = Core::new(camera).project_world_to_screen(
            Vec2::new(32.0, 32.0),
            &Viewport {
                size: Vec2::new(1280.0, 720.0),
                scale_factor: 1.0,
            },
        );
        let presented = Presented {
            frame_ns: 1,
            t: 1.0,
            latest_tick: 1,
            units: vec![
                crate::present::PresentedUnit {
                    unit_id: 1,
                    content_id: "forge:drone".into(),
                    pos: Pos { x: 32.0, y: 32.0 },
                    lo_tick: 1,
                    hi_tick: 1,
                    fraction: 0.0,
                },
                crate::present::PresentedUnit {
                    unit_id: 2,
                    content_id: "forge:drone".into(),
                    pos: Pos { x: 33.0, y: 32.0 },
                    lo_tick: 1,
                    hi_tick: 1,
                    fraction: 0.0,
                },
            ],
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(CoreAdapterPlugin)
            .add_plugins(crate::camera::ClientCameraPlugin)
            .insert_resource(state)
            .insert_resource(AcquiredPackages(acquired))
            .insert_resource(presented)
            .init_resource::<PendingCommands>()
            .init_resource::<CommandSendStatus>()
            .init_resource::<CommandResults>();
        if include_ui {
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::input::InputPlugin,
                bevy::window::WindowPlugin {
                    primary_window: None,
                    ..Default::default()
                },
                bevy::transform::TransformPlugin,
                bevy::image::ImagePlugin::default(),
                bevy::image::TextureAtlasPlugin,
                bevy::mesh::MeshPlugin,
                bevy::text::TextPlugin,
                bevy::camera::CameraPlugin,
                bevy::picking::PickingPlugin,
                bevy::picking::InteractionPlugin,
                bevy::input_focus::InputFocusPlugin,
                bevy::input_focus::InputDispatchPlugin,
                bevy::ui::UiPlugin,
            ));
        }
        if include_client_shell {
            app.insert_resource(crate::volume_settings::VolumeSettings::default())
                .add_plugins(crate::client_shell::ClientShellPlugin);
        }
        if let Some(root) = persist_root {
            app.insert_resource(crate::persist::PersistenceConfigRoot(Some(root)))
                .insert_resource(crate::join::Phase::Joined { slot: 0 })
                .add_plugins(crate::persist::PersistPlugin);
        }
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.finish();
        app.cleanup();
        (app, window, px)
    }

    fn key(window: Entity, code: KeyCode, state: ButtonState, logical: Key) -> WindowEvent {
        WindowEvent::KeyboardInput(KeyboardInput {
            key_code: code,
            logical_key: logical,
            state,
            text: None,
            repeat: false,
            window,
        })
    }
    fn mouse(window: Entity, state: ButtonState) -> WindowEvent {
        WindowEvent::MouseButtonInput(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        })
    }

    #[test]
    fn ordered_window_batch_preserves_ctrl_and_shift_click_chords() {
        let _guard = test_lock();
        let (mut app, window, px) = fixture_app();
        let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
        events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
            window,
            position: px,
            delta: None,
        }));
        events.write(key(
            window,
            KeyCode::ControlLeft,
            ButtonState::Pressed,
            Key::Control,
        ));
        events.write(mouse(window, ButtonState::Pressed));
        events.write(mouse(window, ButtonState::Released));
        events.write(key(
            window,
            KeyCode::ControlLeft,
            ButtonState::Released,
            Key::Control,
        ));
        app.update();
        assert_eq!(
            app.world().resource::<SelectionFeed>().0,
            vec![1, 2],
            "Ctrl-down, pointer, Ctrl-up in one batch must preserve type selection"
        );

        let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
        events.write(key(
            window,
            KeyCode::ShiftLeft,
            ButtonState::Pressed,
            Key::Shift,
        ));
        events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
            window,
            position: px,
            delta: None,
        }));
        events.write(mouse(window, ButtonState::Pressed));
        events.write(mouse(window, ButtonState::Released));
        events.write(key(
            window,
            KeyCode::ShiftLeft,
            ButtonState::Released,
            Key::Shift,
        ));
        app.update();
        assert_eq!(
            app.world().resource::<SelectionFeed>().0,
            vec![2],
            "Shift-down, pointer, Shift-up in one batch must preserve toggle semantics"
        );
    }

    #[test]
    fn locked_relative_motion_sums_into_the_engine_pointer_and_clamps_to_the_window() {
        let _guard = test_lock();
        let (mut app, window, start) = fixture_app();
        app.world_mut()
            .entity_mut(window)
            .insert(bevy::window::CursorOptions {
                grab_mode: bevy::window::CursorGrabMode::None,
                ..default()
            });
        app.world_mut()
            .get_mut::<Window>(window)
            .expect("primary window exists")
            .set_cursor_position(Some(start));
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: start,
                delta: None,
            }));
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .engine_cursor_position(),
            Some(start),
            "unlocked position comes from the actual absolute cursor event"
        );

        app.world_mut()
            .get_mut::<Window>(window)
            .expect("primary window exists")
            .set_cursor_position(None);
        app.world_mut()
            .get_mut::<bevy::window::CursorOptions>(window)
            .expect("primary window has actual cursor options")
            .grab_mode = bevy::window::CursorGrabMode::Locked;
        app.world_mut().resource_mut::<CursorFeed>().lock_active = Some(false);
        app.world_mut()
            .get_mut::<Window>(window)
            .expect("primary window exists")
            .set_cursor_position(Some(start));
        app.world_mut()
            .resource_mut::<Messages<MouseMotion>>()
            .write(MouseMotion {
                delta: Vec2::new(10.0, 10.0),
            });
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .relative_motion_this_frame(),
            Vec2::ZERO,
            "a requested lock without actual browser acquisition must ignore relative motion"
        );
        assert!(!app.world().resource::<CoreRuntime>().cursor_lock_active());
        app.world_mut()
            .get_mut::<Window>(window)
            .expect("primary window exists")
            .set_cursor_position(None);
        app.world_mut().resource_mut::<CursorFeed>().lock_active = Some(true);
        {
            let mut motions = app.world_mut().resource_mut::<Messages<MouseMotion>>();
            motions.write(MouseMotion {
                delta: Vec2::new(24.0, -9.0),
            });
            motions.write(MouseMotion {
                delta: Vec2::new(-4.0, 14.0),
            });
        }
        app.update();
        let expected = start + Vec2::new(20.0, 5.0);
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .engine_cursor_position(),
            Some(expected),
            "multiple actual relative-motion messages are summed before Core pointer dispatch"
        );
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .relative_motion_this_frame(),
            Vec2::new(20.0, 5.0)
        );
        assert!(
            app.world()
                .resource::<CoreRuntime>()
                .cursor_lock_requested()
        );
        assert!(app.world().resource::<CoreRuntime>().cursor_lock_active());

        app.world_mut()
            .get_mut::<Window>(window)
            .expect("primary window exists")
            .set_cursor_position(None);
        app.world_mut()
            .resource_mut::<Messages<MouseMotion>>()
            .write(MouseMotion {
                delta: Vec2::new(5000.0, -5000.0),
            });
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .engine_cursor_position(),
            Some(Vec2::new(1279.0, 0.0)),
            "the in-game pointer is clamped to the logical window bounds"
        );
        assert_eq!(
            app.world()
                .get::<Window>(window)
                .expect("primary window exists")
                .cursor_position(),
            None,
            "the app-owned virtual pointer never replaces Bevy's actual unavailable position"
        );

        app.world_mut()
            .get_mut::<Window>(window)
            .expect("primary window exists")
            .set_cursor_position(Some(Vec2::new(300.0, 200.0)));
        app.world_mut().resource_mut::<CursorFeed>().lock_active = Some(false);
        app.world_mut()
            .resource_mut::<Messages<MouseMotion>>()
            .write(MouseMotion {
                delta: Vec2::new(80.0, 90.0),
            });
        app.update();
        assert!(
            app.world()
                .resource::<CoreRuntime>()
                .cursor_lock_requested()
        );
        assert!(!app.world().resource::<CoreRuntime>().cursor_lock_active());
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .relative_motion_this_frame(),
            Vec2::ZERO,
            "relative movement stops immediately when actual browser lock is lost"
        );
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .engine_cursor_position(),
            Some(Vec2::new(300.0, 200.0)),
            "after actual unlock the engine resumes from the real absolute cursor position"
        );
    }

    #[test]
    fn actual_f10_window_events_open_the_existing_shell_menu_tree() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_client_shell();
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(key(
                window,
                KeyCode::F10,
                ButtonState::Pressed,
                Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            ));
            events.write(key(
                window,
                KeyCode::F10,
                ButtonState::Released,
                Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            ));
        }

        app.update();

        assert!(
            app.world()
                .resource::<CoreEffects>()
                .0
                .iter()
                .any(|effect| matches!(effect, Effect::ShellAction { action } if action == "menu"))
        );
        {
            let world = app.world_mut();
            let mut roots = world.query_filtered::<Entity, With<UiCaptureRoot>>();
            let root = roots
                .single(world)
                .expect("the physical F10 action opens the real shell capture root");
            assert_eq!(
                world.entity(root).get::<bevy::ui::FocusPolicy>(),
                Some(&bevy::ui::FocusPolicy::Block)
            );
            let mut texts = world.query::<&Text>();
            let visible = texts
                .iter(world)
                .map(|text| text.0.as_str())
                .collect::<Vec<_>>();
            assert!(visible.contains(&"Settings"));
            assert!(visible.contains(&"Audio levels"));
        }

        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(key(
                window,
                KeyCode::Escape,
                ButtonState::Pressed,
                Key::Escape,
            ));
            events.write(key(
                window,
                KeyCode::Escape,
                ButtonState::Released,
                Key::Escape,
            ));
        }
        app.update();
        let mut roots = app
            .world_mut()
            .query_filtered::<Entity, With<UiCaptureRoot>>();
        assert_eq!(
            roots.iter(app.world()).count(),
            0,
            "the same physical Escape action closes the menu tree"
        );
    }

    #[test]
    fn retained_frame_records_actual_group_membership_with_selection_and_dispatch() {
        let _guard = test_lock();
        let (mut app, window, px) = fixture_app();
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: px,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![1]);

        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(key(
                window,
                KeyCode::ControlLeft,
                ButtonState::Pressed,
                Key::Control,
            ));
            events.write(key(
                window,
                KeyCode::Digit1,
                ButtonState::Pressed,
                Key::Character("1".into()),
            ));
            events.write(key(
                window,
                KeyCode::Digit1,
                ButtonState::Released,
                Key::Character("1".into()),
            ));
            events.write(key(
                window,
                KeyCode::ControlLeft,
                ButtonState::Released,
                Key::Control,
            ));
        }
        app.update();

        let frame = app
            .world()
            .resource::<FrameFeeds>()
            .recent
            .back()
            .expect("the completed joined frame is retained");
        assert_eq!(frame.selection, vec![1]);
        assert_eq!(frame.groups.get(&1), Some(&vec![1]));
        assert!(frame.named_dispatches.iter().any(|dispatch| {
            dispatch["origin"] == "key"
                && dispatch["code"] == "Digit1"
                && dispatch["matched_action"] == "group_assign_1"
        }));
    }

    #[test]
    fn persisted_groups_restore_after_first_snapshot_and_prune_nonowned_ids() {
        let _guard = test_lock();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after the epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "turts-task16-groups-{}-{unique}",
            std::process::id()
        ));
        let path = crate::persist::native_groups_path(&root, 1, 0);
        crate::persist::save_native_groups(&path, &BTreeMap::from([(1, vec![1, 999])]))
            .expect("the isolated saved group fixture writes");
        let (mut app, _, _) = fixture_app_with_persistence(root.clone());

        app.update();
        assert!(
            app.world().resource::<FrameFeeds>().frame_index > 0,
            "the fixture has completed a first actual Core frame from its accepted snapshot"
        );
        assert_eq!(
            app.world()
                .resource::<FrameFeeds>()
                .recent
                .back()
                .expect("first snapshot frame is retained")
                .groups
                .get(&1),
            Some(&vec![1]),
            "restore is queued after the accepted snapshot and consumed by the first Core step"
        );

        assert_eq!(
            app.world().resource::<SelectionFeed>().0,
            Vec::<u32>::new(),
            "restoring memberships does not forge a selection"
        );
        assert_eq!(
            app.world().resource::<GroupsFeed>().0.get(&1),
            Some(&vec![1]),
            "the real Core restore path keeps the live owned unit and prunes the absent ID"
        );
        assert_eq!(
            crate::persist::load_native_groups(&path).groups.get(&1),
            Some(&vec![1]),
            "the pruned actual group state is retained under its match-and-slot path"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn catalog_qualified_order_id_survives_hotkey_and_real_ground_click() {
        let _guard = test_lock();
        let (mut app, window, scout_px) = acme_order_app();
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: scout_px,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![1]);

        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: Vec2::new(800.0, 400.0),
                delta: None,
            }));
            events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Right,
                state: ButtonState::Pressed,
                window,
            }));
            events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Right,
                state: ButtonState::Released,
                window,
            }));
        }
        app.update();
        let order = app
            .world()
            .resource::<PendingCommands>()
            .0
            .iter()
            .find_map(|pending| match pending {
                crate::commands::PendingCommand::Order { order, .. } => Some(order),
                _ => None,
            })
            .expect("catalog Ground smart rule plus actual right-click must submit an order");
        assert_eq!(order.order, "acme:order_move");
        assert_eq!(order.units, vec![UnitId(1)]);
        assert!(matches!(
            order.target,
            turts_net_protocol::OrderTarget::Point(_)
        ));

        let markers = &app.world().resource::<CoreRuntime>().markers;
        assert_eq!(
            markers.len(),
            1,
            "the clicked order marker exists before any server result"
        );
        assert_eq!(markers[0]["kind"], "move");
        assert_eq!(markers[0]["order"], order.order);
        assert_eq!(markers[0]["units"], serde_json::json!([1]));
        assert_eq!(
            markers[0]["target"],
            serde_json::to_value(&order.target).unwrap()
        );
        assert_eq!(markers[0]["intent_id"], 0);

        app.world_mut().resource_mut::<PendingCommands>().0.clear();
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(key(
                window,
                KeyCode::KeyM,
                ButtonState::Pressed,
                Key::Character("m".into()),
            ));
            events.write(key(
                window,
                KeyCode::KeyM,
                ButtonState::Released,
                Key::Character("m".into()),
            ));
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: Vec2::new(820.0, 420.0),
                delta: None,
            }));
            events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window,
            }));
            events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Released,
                window,
            }));
        }
        app.update();
        let hotkey_order = app
            .world()
            .resource::<PendingCommands>()
            .0
            .iter()
            .find_map(|pending| match pending {
                crate::commands::PendingCommand::Order { order, .. } => Some(order),
                _ => None,
            })
            .expect("catalog KeyM hotkey must resolve the package-qualified OrderDef id");
        assert_eq!(hotkey_order.order, "acme:order_move");
    }

    #[test]
    fn generic_input_queue_runs_in_joined_headless_adapter_once() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app();
        app.world_mut().despawn(window);
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![
            InputEvent::UiAction {
                name: "ui_select:1".into(),
            },
            InputEvent::UiAction {
                name: "acme:order_stop".into(),
            },
        ];

        app.update();
        let orders = app
            .world()
            .resource::<PendingCommands>()
            .0
            .iter()
            .filter_map(|pending| match pending {
                crate::commands::PendingCommand::Order { order, .. } => Some(order),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(orders.len(), 1, "queued order input is applied once");
        assert_eq!(orders[0].order, "acme:order_stop");
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![1]);
        assert!(app.world().resource::<CoreEffects>().0.iter().any(
            |effect| matches!(effect, Effect::Send(order) if order.order == "acme:order_stop")
        ));
        assert!(
            app.world()
                .resource::<CoreEffects>()
                .0
                .contains(&Effect::SelectionChanged)
        );
        assert!(app.world().resource::<CoreInputQueue>().0.is_empty());

        app.update();
        let orders = app
            .world()
            .resource::<PendingCommands>()
            .0
            .iter()
            .filter_map(|pending| match pending {
                crate::commands::PendingCommand::Order { order, .. } => Some(order),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            orders.len(),
            1,
            "the next update does not replay queued input"
        );
        assert!(app.world().resource::<CoreEffects>().0.is_empty());
    }

    #[test]
    fn runtime_binding_override_drives_core_and_effective_bindings_debug() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app();
        app.world_mut().despawn(window);
        app.world_mut()
            .resource_mut::<CorePreferences>()
            .runtime_bindings
            .insert(
                "key_move".into(),
                vec![Binding::Key {
                    code: "KeyN".into(),
                    mods: vec![],
                }],
            );
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![
            InputEvent::UiAction {
                name: "ui_select:1".into(),
            },
            InputEvent::KeyDown {
                code: "KeyN".into(),
                mods: vec![],
            },
            InputEvent::PointerDown {
                button: turts_interaction::Button::Left,
                px: Vec2::new(800.0, 400.0),
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_interaction::Button::Left,
                px: Vec2::new(800.0, 400.0),
                over_widget: false,
            },
        ];
        app.update();
        let order = app
            .world()
            .resource::<PendingCommands>()
            .0
            .iter()
            .find_map(|pending| match pending {
                crate::commands::PendingCommand::Order { order, .. } => Some(order),
                _ => None,
            })
            .expect("runtime KeyN binding resolves the authored Move order");
        assert_eq!(order.order, "acme:order_move");

        let debug: serde_json::Value = serde_json::from_str(&crate::debug::debug_json(app.world()))
            .expect("debug snapshot is JSON");
        assert_eq!(
            debug["effective_bindings"]["actions"]["key_move"][0]["Key"]["code"],
            "KeyN"
        );
        assert_eq!(
            debug["effective_bindings"]["orders"]["acme:order_move"],
            debug["effective_bindings"]["actions"]["key_move"]
        );
    }

    #[test]
    fn task16_actual_modifier_release_during_capture_allows_the_next_wheel() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app();
        app.insert_resource(crate::client_shell::BindingEditorState::default());
        let keyboard = |state| {
            WindowEvent::KeyboardInput(bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::ControlLeft,
                logical_key: bevy::input::keyboard::Key::Control,
                state,
                text: None,
                repeat: false,
                window,
            })
        };
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard(ButtonState::Pressed));
        app.update();
        app.world_mut()
            .resource_mut::<crate::client_shell::BindingEditorState>()
            .capturing = Some("key_move".into());
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard(ButtonState::Released));
        app.update();
        app.world_mut()
            .resource_mut::<crate::client_shell::BindingEditorState>()
            .capturing = None;
        let before = app
            .world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .camera_state()
            .height;
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::MouseWheel(bevy::input::mouse::MouseWheel {
                unit: bevy::input::mouse::MouseScrollUnit::Line,
                x: 0.0,
                y: 1.0,
                window,
                phase: bevy::input::touch::TouchPhase::Moved,
            }));
        app.update();
        let after = app
            .world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .camera_state()
            .height;
        assert!(
            after < before,
            "a Ctrl release during capture must allow the next unmodified wheel to zoom: {before} -> {after}"
        );
    }

    #[test]
    fn task16_captured_pointer_move_binding_dispatches_a_real_world_order() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_client_shell();
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![InputEvent::UiAction {
            name: "menu".into(),
        }];
        app.update();
        crate::client_shell::tests::activate_named_change(app.world_mut(), "key_move");
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(WindowEvent::MouseButtonInput(MouseButtonInput {
                    button: MouseButton::Back,
                    state,
                    window,
                }));
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&vec![Binding::Pointer {
                button: turts_mod_api::Button::Back,
                mods: vec![]
            }]),
            "the actual menu producer must first capture Back for the authored Move action"
        );
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![InputEvent::UiAction {
            name: "menu".into(),
        }];
        app.update();
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![InputEvent::UiAction {
            name: "ui_select:1".into(),
        }];
        app.update();
        app.world_mut().resource_mut::<PendingCommands>().0.clear();
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: Vec2::new(800.0, 400.0),
                delta: None,
            }));
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(WindowEvent::MouseButtonInput(MouseButtonInput {
                    button: MouseButton::Back,
                    state,
                    window,
                }));
        }
        app.update();
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(WindowEvent::MouseButtonInput(MouseButtonInput {
                    button: MouseButton::Left,
                    state,
                    window,
                }));
        }
        app.update();
        let order = app.world().resource::<PendingCommands>().0.iter().find_map(|pending|
            if let crate::commands::PendingCommand::Order { order, .. } = pending { Some(order) } else { None })
            .expect("a physically captured Back binding followed by the actual target click must send the real Move order");
        assert_eq!(order.order, "forge:order_move");
        assert_eq!(order.units, vec![UnitId(1)]);
        assert!(matches!(order.target, OrderTarget::Point(_)));
    }

    #[test]
    fn task16_binding_capture_withholds_real_pointer_actions_before_core_dispatch() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_client_shell();
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![InputEvent::UiAction {
            name: "ui_select:1".into(),
        }];
        app.update();
        app.world_mut()
            .resource_mut::<CorePreferences>()
            .runtime_bindings
            .insert(
                "key_move".into(),
                vec![Binding::Pointer {
                    button: turts_mod_api::Button::Back,
                    mods: vec![],
                }],
            );
        app.world_mut()
            .resource_mut::<crate::client_shell::BindingEditorState>()
            .capturing = Some("key_stop".into());
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: Vec2::new(800.0, 400.0),
                delta: None,
            }));
        for button in [MouseButton::Back, MouseButton::Left] {
            for state in [ButtonState::Pressed, ButtonState::Released] {
                app.world_mut()
                    .resource_mut::<Messages<WindowEvent>>()
                    .write(WindowEvent::MouseButtonInput(MouseButtonInput {
                        button,
                        state,
                        window,
                    }));
            }
        }
        app.update();
        assert!(
            !app.world()
                .resource::<PendingCommands>()
                .0
                .iter()
                .any(|pending| matches!(pending, crate::commands::PendingCommand::Order { .. })),
            "binding capture must suspend pointer-bound Move and its world target click before the actual Core dispatcher"
        );
    }

    #[test]
    fn task16_open_menu_suspends_world_keys_and_keeps_the_real_close_action() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_client_shell();
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![InputEvent::UiAction {
            name: "ui_select:1".into(),
        }];
        app.update();
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![InputEvent::UiAction {
            name: "menu".into(),
        }];
        app.update();
        app.world_mut().resource_mut::<PendingCommands>().0.clear();
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(key(
                window,
                KeyCode::KeyS,
                ButtonState::Pressed,
                Key::Character("s".into()),
            ));
        app.update();
        assert!(
            !app.world()
                .resource::<PendingCommands>()
                .0
                .iter()
                .any(|pending| matches!(pending, crate::commands::PendingCommand::Order { .. })),
            "an open settings menu must not send the pointed world's Stop order"
        );
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(key(window, KeyCode::F10, ButtonState::Pressed, Key::F10));
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<crate::client_shell::SettingsMenuOverlay>>()
                .iter(world)
                .count(),
            0,
            "the shell close action must still work while world input is captured"
        );
    }

    #[test]
    fn physical_binding_capture_suppresses_that_key_before_core_dispatch() {
        let _guard = test_lock();
        for (capturing, expected_orders) in [(false, 1), (true, 0)] {
            let (mut app, window, _) = acme_order_app();
            app.world_mut()
                .resource_mut::<CorePreferences>()
                .runtime_bindings
                .insert(
                    "key_move".into(),
                    vec![Binding::Key {
                        code: "KeyQ".into(),
                        mods: vec![],
                    }],
                );
            let mut binding_editor = crate::client_shell::BindingEditorState::default();
            binding_editor.capturing = capturing.then(|| "key_move".to_owned());
            app.insert_resource(binding_editor);
            app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![
                InputEvent::UiAction {
                    name: "ui_select:1".into(),
                },
                InputEvent::PointerDown {
                    button: turts_interaction::Button::Left,
                    px: Vec2::new(800.0, 400.0),
                    over_widget: false,
                },
                InputEvent::PointerUp {
                    button: turts_interaction::Button::Left,
                    px: Vec2::new(800.0, 400.0),
                    over_widget: false,
                },
            ];
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(WindowEvent::KeyboardInput(
                    bevy::input::keyboard::KeyboardInput {
                        key_code: KeyCode::KeyQ,
                        logical_key: bevy::input::keyboard::Key::Character("q".into()),
                        state: ButtonState::Pressed,
                        text: None,
                        repeat: false,
                        window,
                    },
                ));

            app.update();
            let orders = app
                .world()
                .resource::<PendingCommands>()
                .0
                .iter()
                .filter(|pending| matches!(pending, crate::commands::PendingCommand::Order { .. }))
                .count();
            assert_eq!(
                orders, expected_orders,
                "the same actual key dispatches the bound move outside capture and is withheld while the shell captures it"
            );
        }
    }

    #[test]
    fn pending_order_cursor_seam_tracks_real_core_target_and_cancellation() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app();
        app.world_mut().despawn(window);
        app.update();
        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![
            InputEvent::UiAction {
                name: "ui_select:1".into(),
            },
            InputEvent::UiAction {
                name: "acme:order_move".into(),
            },
        ];
        app.update();
        assert_eq!(
            app.world().resource::<CoreRuntime>().pending_order_id(),
            Some("acme:order_move"),
            "the presentation seam reports the actual order awaiting its target"
        );

        app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![
            InputEvent::PointerDown {
                button: turts_interaction::Button::Right,
                px: Vec2::new(600.0, 350.0),
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_interaction::Button::Right,
                px: Vec2::new(600.0, 350.0),
                over_widget: false,
            },
        ];
        app.update();
        assert_eq!(
            app.world().resource::<CoreRuntime>().pending_order_id(),
            None,
            "right-click cancellation clears the core state observed by the cursor"
        );
    }

    #[test]
    fn guide_button_and_f1_both_toggle_once_through_core_shell_action() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app_with_widgets(true);
        app.update();
        assert!(
            app.world()
                .resource::<crate::widgets::guide::GuideOpen>()
                .is_open
        );
        let guide_button = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<crate::widgets::primitives::AuthoredWidgetId>()
                    .is_some_and(|id| id.0 == "test_guide_toggle")
            })
            .expect("the test-authored guide toggle is installed")
            .id();
        let initial_visibility = *app.world().get::<Visibility>(guide_button).unwrap();
        assert_eq!(initial_visibility, Visibility::Inherited);
        assert_eq!(
            app.world().get::<Text>(guide_button).unwrap().0,
            "Guide [F1]",
            "hotkey_label displays the effective player binding"
        );

        app.world_mut().trigger(bevy::ui_widgets::Activate {
            entity: guide_button,
        });
        assert!(
            app.world()
                .resource::<crate::widgets::guide::GuideOpen>()
                .is_open,
            "button activation queues input without toggling locally"
        );
        app.update();
        assert!(
            !app.world()
                .resource::<crate::widgets::guide::GuideOpen>()
                .is_open
        );
        assert_eq!(
            app.world().get::<Visibility>(guide_button),
            Some(&Visibility::Hidden)
        );
        assert!(app.world().resource::<CoreEffects>().0.iter().any(
            |effect| matches!(effect, Effect::ShellAction { action } if action == "toggle_guide")
        ));
        assert!(app.world().get_entity(guide_button).is_ok());

        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(key(
                window,
                KeyCode::F1,
                ButtonState::Pressed,
                Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            ));
            events.write(key(
                window,
                KeyCode::F1,
                ButtonState::Released,
                Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            ));
        }
        app.update();
        assert!(
            app.world()
                .resource::<crate::widgets::guide::GuideOpen>()
                .is_open
        );
        assert_eq!(
            app.world().get::<Visibility>(guide_button),
            Some(&Visibility::Inherited)
        );
        assert!(
            app.world().get_entity(guide_button).is_ok(),
            "GuideOpen changes retain widget and button identity"
        );
        assert!(app.world().resource::<CoreEffects>().0.iter().any(
            |effect| matches!(effect, Effect::ShellAction { action } if action == "toggle_guide")
        ));

        app.world_mut().resource_mut::<ReplicaState>().replica = None;
        app.update();
        assert!(
            app.world()
                .resource::<crate::widgets::guide::GuideOpen>()
                .is_open,
            "an early adapter return does not replay the prior frame's F1 shell action"
        );
        assert_eq!(
            app.world().get::<Visibility>(guide_button),
            Some(&Visibility::Inherited)
        );
    }

    #[test]
    fn adapter_defers_core_until_joined_and_uses_optional_camera_pose() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app();
        app.world_mut().despawn(window);
        let initial_pose = CameraState {
            look_at: Vec2::new(9.0, 13.0),
            height: 55.0,
            pitch: 0.5,
            yaw: 0.25,
            vertical_fov: 0.7,
        };
        app.world_mut()
            .resource_mut::<crate::camera::CameraConfiguration>()
            .initial_pose = Some(initial_pose);
        assert!(app.world().resource::<CoreRuntime>().core().is_none());
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .core()
                .expect("the joined headless world initializes the core")
                .camera_state(),
            initial_pose
        );
        let camera = &app
            .world()
            .resource::<crate::camera::CameraConfiguration>()
            .options;
        assert!(camera.authored.is_some());
        assert_eq!(camera.scroll_speed, 1.0);
        assert!(camera.edge_scroll);
    }

    #[test]
    fn window_focus_received_before_snapshot_is_retained_for_the_first_core_frame() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_client_shell();
        let acquired = app.world().resource::<AcquiredPackages>().0.clone();
        {
            let mut state = app.world_mut().resource_mut::<ReplicaState>();
            state.content = None;
            state.replica.as_mut().unwrap().snapshot_tick = None;
        }
        app.world_mut().remove_resource::<AcquiredPackages>();
        app.world_mut()
            .get_mut::<Window>(window)
            .expect("fixture has a primary window")
            .focused = true;
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::WindowFocused(bevy::window::WindowFocused {
                window,
                focused: true,
            }));

        app.update();
        assert!(
            app.world().resource::<FocusFeed>().0,
            "the real focused primary Window must update the focus producer during loading"
        );

        app.world_mut().resource_mut::<ReplicaState>().content = Some(acquired.content.clone());
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .snapshot_tick = Some(1);
        app.world_mut().insert_resource(AcquiredPackages(acquired));
        app.update();
        assert!(app.world().resource::<FocusFeed>().0);
        let mut overlays = app
            .world_mut()
            .query_filtered::<Entity, With<crate::client_shell::FocusOverlay>>();
        assert_eq!(
            overlays.iter(app.world()).count(),
            0,
            "first Core initialization must not leave a click-to-play overlay over a focused window"
        );
    }

    #[test]
    fn owned_camera_start_waits_for_snapshot_and_survives_later_resync() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app();
        app.world_mut().despawn(window);
        let authored = CameraDef {
            pitch: 0.62,
            yaw: -0.3,
            field_of_view: 48.0,
            minimum: 12.0,
            maximum: 88.0,
            start: CameraStart::OwnedCentre,
            height: 42.0,
            margin: 6.0,
            pan_speed: 1.0,
            edge_band: true,
        };
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .content
            .camera = Some(authored.clone());
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .content
            .as_mut()
            .expect("joined content")
            .camera = Some(authored);
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .expect("joined replica")
            .snapshot_tick = None;

        app.update();
        assert!(
            app.world().resource::<CoreRuntime>().core().is_none(),
            "Welcome and acquired map metadata must not latch the map-centre fallback before a snapshot"
        );
        assert!(
            app.world()
                .resource::<crate::camera::CameraConfiguration>()
                .initial_pose
                .is_none()
        );

        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .expect("joined replica")
            .snapshot_tick = Some(1);
        app.update();
        let expected = Vec2::new(4.0, 4.0);
        let latched_pose = app
            .world()
            .resource::<CoreRuntime>()
            .core()
            .expect("accepted snapshot starts the core")
            .camera_state();
        assert_eq!(latched_pose.look_at, expected);
        assert_eq!(latched_pose.height, 42.0);
        assert!((latched_pose.vertical_fov - 48.0_f32.to_radians()).abs() < 1.0e-6);

        {
            let mut state = app.world_mut().resource_mut::<ReplicaState>();
            let replica = state.replica.as_mut().expect("joined replica");
            replica.snapshot_tick = None;
            replica.units.get_mut(&1).expect("own scout").samples =
                VecDeque::from([(2, Pos { x: 60.0, y: 50.0 })]);
        }
        app.world_mut().resource_mut::<Presented>().units[0].pos = Pos { x: 60.0, y: 50.0 };
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .core()
                .expect("resync keeps the existing core")
                .camera_state(),
            latched_pose,
            "a later recovery gap must not replace the camera pose latched from the initial snapshot"
        );
    }

    #[test]
    fn empty_initial_snapshot_falls_back_to_authored_map_centre() {
        let _guard = test_lock();
        let (mut app, window, _) = acme_order_app();
        app.world_mut().despawn(window);
        let mut authored = app
            .world()
            .resource::<AcquiredPackages>()
            .0
            .content
            .camera
            .clone()
            .expect("Acme camera");
        authored.start = CameraStart::OwnedCentre;
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .content
            .camera = Some(authored.clone());
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .content
            .as_mut()
            .expect("joined content")
            .camera = Some(authored);
        {
            let mut state = app.world_mut().resource_mut::<ReplicaState>();
            let replica = state.replica.as_mut().expect("joined replica");
            replica.units.clear();
            replica.snapshot_tick = None;
        }
        app.world_mut().resource_mut::<Presented>().units.clear();
        app.update();
        assert!(app.world().resource::<CoreRuntime>().core().is_none());

        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .expect("joined replica")
            .snapshot_tick = Some(1);
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .core()
                .expect("accepted empty snapshot starts the core")
                .camera_state()
                .look_at,
            Vec2::new(32.0, 32.0),
            "an accepted snapshot with no owned units falls back to the map centre"
        );
    }

    #[test]
    fn ui_selection_request_enters_core_and_primary_feed() {
        let _guard = test_lock();
        let (mut app, _, _) = fixture_app();
        app.world_mut()
            .resource_mut::<crate::ui_shared::SelectionRequests>()
            .0
            .push(2);
        app.update();
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![2]);
        assert_eq!(
            app.world().resource::<Selection>().0,
            Some(2),
            "the UI primary selection is written from the core result"
        );
    }

    #[test]
    fn joined_headless_adapter_applies_ui_selection_and_prunes_removed_units() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app();
        app.world_mut().despawn(window);
        app.world_mut()
            .resource_mut::<crate::ui_shared::SelectionRequests>()
            .0
            .push(2);
        app.update();
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![2]);
        assert_eq!(app.world().resource::<Selection>().0, Some(2));

        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .units
            .remove(&2);
        app.world_mut()
            .resource_mut::<Presented>()
            .units
            .retain(|unit| unit.unit_id != 2);
        app.update();
        assert!(app.world().resource::<SelectionFeed>().0.is_empty());
        assert_eq!(app.world().resource::<Selection>().0, None);
    }

    #[test]
    fn window_input_over_computed_visible_panel_is_consumed_by_actual_adapter() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_ui(true);
        let camera_entity = app
            .world_mut()
            .spawn((Camera3d::default(), IsDefaultUiCamera))
            .id();
        app.world_mut()
            .get_mut::<Camera>(camera_entity)
            .unwrap()
            .computed
            .target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: UVec2::new(1280, 720),
            scale_factor: 1.0,
        });
        let panel = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(220.0),
                    height: Val::Px(220.0),
                    ..Default::default()
                },
                bevy::ui::UiTargetCamera(camera_entity),
                Visibility::Visible,
                bevy::camera::visibility::InheritedVisibility::VISIBLE,
                UiCaptureRoot,
            ))
            .id();
        app.update();
        let node = app.world().get::<ComputedNode>(panel).unwrap();
        assert!(node.size().x >= 200.0 && node.size().y >= 200.0);
        assert!(
            app.world()
                .get::<ComputedUiTargetCamera>(panel)
                .and_then(ComputedUiTargetCamera::get)
                .is_some()
        );

        app.world_mut()
            .resource_mut::<crate::ui_shared::SelectionRequests>()
            .0
            .push(1);
        app.update();
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![1]);

        let cursor = Vec2::new(100.0, 100.0);
        let ui_camera = app
            .world()
            .get::<ComputedUiTargetCamera>(panel)
            .and_then(ComputedUiTargetCamera::get)
            .and_then(|entity| app.world().get::<Camera>(entity))
            .unwrap();
        let ui_point = cursor_in_ui_viewport(cursor, ui_camera).unwrap();
        let node = app.world().get::<ComputedNode>(panel).unwrap();
        let transform = app.world().get::<UiGlobalTransform>(panel).unwrap();
        assert!(
            node.contains_point(*transform, ui_point),
            "actual computed panel geometry must contain the test cursor: point={ui_point:?} transform={:?} size={:?}",
            transform.to_scale_angle_translation(),
            node.size()
        );
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: cursor,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![1]);
        assert_eq!(app.world().resource::<PendingCommands>().0.len(), 0);

        let outside = app
            .world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .project_world_to_screen(
                Vec2::new(33.0, 32.0),
                &Viewport {
                    size: Vec2::new(1280.0, 720.0),
                    scale_factor: 1.0,
                },
            );

        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: outside,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        assert_eq!(app.world().resource::<SelectionFeed>().0, vec![2]);
    }

    #[test]
    fn authored_minimap_input_routes_to_core_and_cursor_exit_releases_capture() {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_ui(true);
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .map_size = (80.0, 40.0);
        let camera_entity = app
            .world_mut()
            .spawn((Camera3d::default(), IsDefaultUiCamera))
            .id();
        app.world_mut()
            .get_mut::<Camera>(camera_entity)
            .unwrap()
            .computed
            .target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: UVec2::new(1280, 720),
            scale_factor: 1.0,
        });
        let minimap = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(100.0),
                    top: Val::Px(100.0),
                    width: Val::Px(200.0),
                    height: Val::Px(100.0),
                    ..default()
                },
                bevy::ui::UiTargetCamera(camera_entity),
                Visibility::Visible,
                bevy::camera::visibility::InheritedVisibility::VISIBLE,
                UiCaptureRoot,
                crate::widgets::primitives::MinimapViewConfig {
                    unit_size: 4.0,
                    frame_colour: Some(Color::WHITE),
                    ping_colour: Some(Color::srgb_u8(255, 204, 0)),
                },
            ))
            .id();
        app.update();
        app.update();

        let transform = *app.world().get::<UiGlobalTransform>(minimap).unwrap();
        let camera = app.world().get::<Camera>(camera_entity).unwrap();
        let ui_camera_origin = camera.physical_viewport_rect().unwrap().min.as_vec2();
        let minimap_cursor = transform * Vec2::new(50.0, -25.0);
        let minimap_cursor = minimap_cursor + ui_camera_origin;
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: minimap_cursor,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .core()
                .unwrap()
                .camera_state()
                .look_at,
            Vec2::new(60.0, 30.0),
            "actual transformed (.75,.25) widget input must preserve top-down Y mapping"
        );
        assert!(app.world().resource::<SelectionFeed>().0.is_empty());
        assert!(app.world().resource::<PendingCommands>().0.is_empty());

        let overlay = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                bevy::ui::UiTargetCamera(camera_entity),
                Visibility::Visible,
                UiCaptureRoot,
                ZIndex(i32::MAX - 1),
                bevy::ui::FocusPolicy::Block,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            ))
            .id();
        app.update();
        app.update();
        assert!(
            app.world()
                .get::<bevy::ui::ComputedStackIndex>(overlay)
                .unwrap()
                .0
                > app
                    .world()
                    .get::<bevy::ui::ComputedStackIndex>(minimap)
                    .unwrap()
                    .0,
            "the modal overlay fixture must be above the minimap in actual Bevy UI stack order"
        );
        let camera_before_modal_click = app
            .world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .camera_state();
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: minimap_cursor,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<CoreRuntime>()
                .core()
                .unwrap()
                .camera_state(),
            camera_before_modal_click,
            "a topmost blocking capture must prevent the covered minimap from moving the camera"
        );
        assert!(app.world().resource::<PendingCommands>().0.is_empty());
        app.world_mut().despawn(overlay);
        app.update();
        app.update();

        let other_camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                Camera {
                    computed: bevy::camera::ComputedCameraValues {
                        target_info: Some(bevy::camera::RenderTargetInfo {
                            physical_size: UVec2::new(1280, 720),
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    ..default()
                },
                bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window)),
                Transform::default(),
            ))
            .id();
        let other_camera_overlay = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                bevy::ui::UiTargetCamera(other_camera),
                Visibility::Visible,
                UiCaptureRoot,
                ZIndex(i32::MAX - 1),
                bevy::ui::FocusPolicy::Block,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            ))
            .id();
        app.update();
        app.update();
        assert!(
            app.world()
                .get::<bevy::ui::ComputedStackIndex>(other_camera_overlay)
                .unwrap()
                .0
                > app
                    .world()
                    .get::<bevy::ui::ComputedStackIndex>(minimap)
                    .unwrap()
                    .0,
            "the overlapping second-camera fixture must have a globally higher computed stack index"
        );
        let other_camera_point = transform * Vec2::new(-60.0, 30.0) + ui_camera_origin;
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: other_camera_point,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        let target_camera_state = app
            .world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .camera_state()
            .look_at;
        assert!(
            target_camera_state.distance(Vec2::new(16.0, 8.0)) < 1.0e-4,
            "a second camera's independent UI stack must not block the actual minimap target camera: {target_camera_state:?}"
        );
        app.world_mut().despawn(other_camera_overlay);
        app.world_mut().despawn(other_camera);
        app.update();

        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: minimap_cursor,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
        }
        app.update();
        assert_eq!(
            app.world().resource::<CoreRuntime>().minimap_pointer,
            Some(turts_mod_api::Button::Left)
        );
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::CursorLeft(bevy::window::CursorLeft { window }));
        app.update();
        assert_eq!(app.world().resource::<CoreRuntime>().minimap_pointer, None);

        let world_cursor = app
            .world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .project_world_to_screen(
                Vec2::new(32.0, 32.0),
                &Viewport {
                    size: Vec2::new(1280.0, 720.0),
                    scale_factor: 1.0,
                },
            );
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: world_cursor,
                delta: None,
            }));
            events.write(mouse(window, ButtonState::Pressed));
            events.write(mouse(window, ButtonState::Released));
        }
        app.update();
        assert_eq!(
            app.world().resource::<SelectionFeed>().0,
            vec![1],
            "after CursorLeft, a new world click must still deliver its ordinary down/up pair"
        );
    }

    #[test]
    fn stationary_edge_pointer_stops_when_capture_appears_and_never_rearms_after_exit_or_focus_loss()
     {
        let _guard = test_lock();
        let (mut app, window, _) = fixture_app_with_ui(true);
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                Camera {
                    computed: bevy::camera::ComputedCameraValues {
                        target_info: Some(bevy::camera::RenderTargetInfo {
                            physical_size: UVec2::new(1280, 720),
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    viewport: Some(bevy::camera::Viewport {
                        physical_size: UVec2::new(1280, 720),
                        ..default()
                    }),
                    ..default()
                },
                bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window)),
                IsDefaultUiCamera,
                Transform::from_xyz(32.0, 40.0, 62.0)
                    .looking_at(Vec3::new(32.0, 0.0, 32.0), Vec3::Y),
            ))
            .id();
        app.update();

        let edge = Vec2::new(1.0, 180.0);
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: edge,
                delta: None,
            }));
        app.update();
        assert_eq!(
            app.world().resource::<CoreRuntime>().camera_pan_context(),
            Some("pan_left"),
            "a real pointer at the edge activates edge-pan"
        );

        let panel = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(80.0),
                    width: Val::Px(80.0),
                    height: Val::Px(220.0),
                    ..default()
                },
                bevy::ui::UiTargetCamera(camera),
                Visibility::Visible,
                UiCaptureRoot,
            ))
            .id();
        app.update(); // publish computed UI geometry
        app.update(); // refresh live capture with no new pointer event
        assert_eq!(
            app.world().resource::<CoreRuntime>().camera_pan_context(),
            None,
            "newly visible UI beneath a stationary pointer stops edge-pan"
        );

        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::CursorLeft(bevy::window::CursorLeft { window }));
        app.world_mut().despawn(panel);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<CoreRuntime>().camera_pan_context(),
            None,
            "removing UI after CursorLeft cannot rearm from stale window coordinates"
        );

        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: edge,
                delta: None,
            }));
        app.update();
        assert_eq!(
            app.world().resource::<CoreRuntime>().camera_pan_context(),
            Some("pan_left")
        );
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::WindowFocused(bevy::window::WindowFocused {
                window,
                focused: false,
            }));
        app.update();
        app.update();
        assert!(!app.world().resource::<FocusFeed>().0);
        assert_eq!(
            app.world().resource::<CoreRuntime>().camera_pan_context(),
            None,
            "focus loss clears edge-pan and the saved pointer"
        );
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::WindowFocused(bevy::window::WindowFocused {
                window,
                focused: true,
            }));
        app.update();
        assert!(app.world().resource::<FocusFeed>().0);
        assert_eq!(
            app.world().resource::<CoreRuntime>().camera_pan_context(),
            None,
            "focus gain alone never reuses stale coordinates"
        );
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::KeyboardFocusLost(
                bevy::input::keyboard::KeyboardFocusLost,
            ));
        app.update();
        app.update();
        assert!(
            !app.world().resource::<FocusFeed>().0,
            "actual canvas keyboard-focus loss remains recorded while the OS window stays focused"
        );
    }

    #[test]
    fn frame_results_are_deltas_while_top_level_feed_retains_history() {
        let _guard = test_lock();
        let (mut app, _, _) = fixture_app();
        app.world_mut()
            .resource_mut::<CommandResults>()
            .0
            .push(CommandResult::Order {
                seq: 9,
                tick: 1,
                accepted: vec![UnitId(1)],
                refused: vec![],
                envelope: None,
                job_id: None,
            });
        app.update();
        let first = app.world().resource::<FrameFeeds>().recent.back().unwrap();
        assert_eq!(first.results.len(), 1);
        assert_eq!(app.world().resource::<ResultsFeed>().0.len(), 1);

        app.update();
        let second = app.world().resource::<FrameFeeds>().recent.back().unwrap();
        assert!(
            second.results.is_empty(),
            "an old result must not be re-emitted in a later frame"
        );
        assert_eq!(
            app.world().resource::<ResultsFeed>().0.len(),
            1,
            "top-level feed retains results for later browser oracles"
        );
    }

    #[test]
    fn retained_input_keeps_causal_command_and_result_beyond_recent_frames() {
        let _guard = test_lock();
        let (mut app, _, _) = fixture_app();
        app.world_mut()
            .resource_mut::<CoreRuntime>()
            .pending_sequences
            .push(44);
        app.update();
        for _ in 0..100 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<CommandResults>()
            .0
            .push(CommandResult::Order {
                seq: 44,
                tick: 2,
                accepted: vec![UnitId(1)],
                refused: vec![],
                envelope: None,
                job_id: None,
            });
        app.update();

        let frames = &app.world().resource::<FrameFeeds>().retained_input;
        let emitted = frames
            .iter()
            .find(|frame| frame.applied_seq.contains(&44))
            .expect("physical input frame remains in the causal history");
        let result = frames
            .iter()
            .find(|frame| frame.results.iter().any(|result| result["seq"] == 44))
            .expect("later matching result remains after 100 intervening idle frames");
        assert!(result.frame_index.saturating_sub(emitted.frame_index) > 64);
        assert_eq!(result.results[0]["seq"], emitted.applied_seq[0]);
    }

    #[test]
    fn retained_input_keeps_alert_frames_after_the_recent_window_rolls_over() {
        let _guard = test_lock();
        let (mut app, _, _) = fixture_app();
        for _ in 0..100 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<FrameFeedback>()
            .alerts
            .push(serde_json::json!({"id":"muster_unit_damaged","unit":7,"tick":31}));
        app.update();

        let frames = &app.world().resource::<FrameFeeds>().retained_input;
        let alert_frame = frames
            .iter()
            .find(|frame| {
                frame.feedback.alerts.iter().any(|alert| {
                    alert["id"] == "muster_unit_damaged"
                        && alert["unit"] == 7
                        && alert["tick"] == 31
                })
            })
            .expect("actual alert-bearing causal frame remains retained");
        assert!(alert_frame.frame_index > 64);
        assert_eq!(alert_frame.feedback.alerts[0]["unit"], 7);
    }

    fn add_test_sequence(mut runtime: ResMut<CoreRuntime>) {
        runtime.pending_sequences.push(72);
    }

    #[test]
    fn frame_clear_precedes_and_keeps_same_update_input() {
        let _guard = test_lock();
        let (mut app, _, _) = fixture_app();
        app.world_mut()
            .resource_mut::<CoreRuntime>()
            .pending_sequences
            .push(71);
        app.update();
        assert_eq!(app.world().resource::<FrameFeeds>().retained_input.len(), 1);
        app.add_systems(Update, add_test_sequence.before(CoreAdapterSet));
        clear_frames();
        app.update();
        let frames = app.world().resource::<FrameFeeds>();
        assert_eq!(frames.recent.len(), 1);
        assert_eq!(frames.retained_input.len(), 1);
        assert_eq!(frames.retained_input[0].applied_seq, vec![72]);
        assert!(!frames.retained_input[0].applied_seq.contains(&71));
    }

    #[test]
    fn adapter_does_not_clear_or_forge_owner_presentation_feeds() {
        let _guard = test_lock();
        let (mut app, _, _) = fixture_app();
        {
            let world = app.world_mut();
            let mut feed = world.resource_mut::<AudioFeed>();
            feed.context_state = Some("running".into());
            feed.started.push(crate::debug_feed::StartedSound {
                id: "owner:started".into(),
                category: "ui".into(),
                priority: 0,
            });
            feed.categories.insert("ui".into(), 0.5);
            world.resource_mut::<CursorFeed>().applied_icon = Some("owner:cursor".into());
            world.resource_mut::<CursorFeed>().position = Some([321.0, 123.0]);
            world.resource_mut::<ContextMenuFeed>().count = 4;
            world.resource_mut::<ContextMenuFeed>().count_prevented = 2;
            world
                .resource_mut::<MarkersShown>()
                .0
                .push(serde_json::json!({"owner":true}));
            world
                .resource_mut::<NoticesFeed>()
                .0
                .push(serde_json::json!({"owner":true}));
            *world.resource_mut::<crate::feedback::FrameFeedback>() =
                crate::feedback::FrameFeedback {
                    audio_started: vec![crate::debug_feed::StartedSound {
                        id: "owner:frame-started".into(),
                        category: "orders".into(),
                        priority: 0,
                    }],
                    notices: vec![serde_json::json!({"owner":"frame-notice"})],
                    alerts: vec![serde_json::json!({"owner":"frame-alert"})],
                };
        }
        app.update();
        assert_eq!(app.world().resource::<AudioFeed>().started.len(), 1);
        assert_eq!(
            app.world().resource::<AudioFeed>().context_state.as_deref(),
            Some("running")
        );
        assert_eq!(
            app.world().resource::<CursorFeed>().applied_icon.as_deref(),
            Some("owner:cursor")
        );
        assert_eq!(
            app.world().resource::<CursorFeed>().position,
            Some([321.0, 123.0])
        );
        assert_eq!(app.world().resource::<ContextMenuFeed>().count_prevented, 2);
        assert_eq!(app.world().resource::<MarkersShown>().0.len(), 1);
        assert_eq!(app.world().resource::<NoticesFeed>().0.len(), 1);
        let feedback = app
            .world()
            .resource::<crate::feedback::FrameFeedback>()
            .clone();
        assert_eq!(feedback.audio_started[0].id, "owner:frame-started");
        assert_eq!(
            app.world()
                .resource::<FrameFeeds>()
                .recent
                .back()
                .unwrap()
                .feedback,
            feedback,
            "the Last-stage frame captures future-owner feedback without clearing it"
        );
        assert!(app.world().resource::<CoreRuntime>().effects().is_empty());
    }

    #[test]
    fn actual_last_stage_retains_nonempty_input_for_two_hundred_later_frames() {
        let _guard = test_lock();
        let (mut app, _, _) = fixture_app();
        app.world_mut()
            .resource_mut::<CoreRuntime>()
            .pending_sequences
            .push(71);
        app.update();
        for _ in 0..200 {
            app.update();
        }
        let frames = app.world().resource::<FrameFeeds>();
        let retained = frames
            .retained_input
            .iter()
            .find(|record| record.applied_seq.contains(&71))
            .expect("Last must retain the real frame record containing the input sequence");
        assert!(
            frames.frame_index - retained.frame_index >= 200,
            "retention is counted in actual closed frame records"
        );
        assert_eq!(frames.recent.len(), 64);
    }

    #[test]
    fn full_headless_build_app_can_step_without_window_resources() {
        let _guard = test_lock();
        let options = crate::options::ClientOptions {
            headless: true,
            ..Default::default()
        };
        let mut app = crate::build_app(options);
        for _ in 0..4 {
            app.update();
        }
        assert!(app.world().contains_resource::<Messages<WindowEvent>>());
        assert!(app.world().contains_resource::<FrameFeeds>());
        assert!(app.world().contains_resource::<SelectionFeed>());
    }
}
