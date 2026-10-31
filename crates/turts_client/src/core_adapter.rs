//! Production adapter between Bevy's ordered window stream and the pure interaction core.

use bevy::camera::{PerspectiveProjection, Projection};
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
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
    Binding, Capability, Footprint, InputMapDef, OrderDef, OrderRulesDef, SlotRelation,
};
use turts_net_protocol::{SlotId, UnitId};

use crate::{
    commands::{PendingCommands, submit_order},
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
    last_ui_selection: Option<u32>,
    last_core_primary: Option<u32>,
    pending_sequences: Vec<u32>,
    last_effects: Vec<ValueEffect>,
    markers: Vec<serde_json::Value>,
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
    pub applied_seq: Vec<u32>,
    pub selection: Vec<u32>,
    pub positions: Vec<(u32, f32, f32)>,
    pub results: Vec<serde_json::Value>,
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
    panels: &Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            &ComputedUiTargetCamera,
        ),
        With<UiCaptureRoot>,
    >,
    cameras: &Query<(Entity, &Camera), With<Camera3d>>,
) -> bool {
    panels.iter().any(|(node, transform, visible, target)| {
        if !visible.get() {
            return false;
        }
        let Some(camera_id) = target.get() else {
            return false;
        };
        let Some((_, camera)) = cameras.iter().find(|(entity, _)| *entity == camera_id) else {
            return false;
        };
        let Some(point) = cursor_in_ui_viewport(cursor, camera) else {
            return false;
        };
        node.size().min_element() <= 0.0 || node.contains_point(*transform, point)
    })
}

#[derive(SystemParam)]
struct AdapterInputs<'w, 's> {
    windows: Query<'w, 's, (Entity, &'static Window), With<PrimaryWindow>>,
    window_events: MessageReader<'w, 's, WindowEvent>,
    panels: Query<
        'w,
        's,
        (
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
            &'static ComputedUiTargetCamera,
        ),
        With<UiCaptureRoot>,
    >,
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
    status: Res<'w, crate::commands::CommandSendStatus>,
    selection_feed: ResMut<'w, SelectionFeed>,
    groups_feed: ResMut<'w, GroupsFeed>,
    focus_feed: ResMut<'w, FocusFeed>,
    player_feed: ResMut<'w, PlayerIdFeed>,
    player_id: Option<Res<'w, crate::PlayerId>>,
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
    let mut current_cursor = runtime.cursor;
    for event in input.window_events.read() {
        let primary = input.windows.iter().next().map(|(entity, _)| entity);
        match event {
            WindowEvent::CursorMoved(value) if Some(value.window) == primary => {
                current_cursor = Some(value.position);
                events.push(InputEvent::PointerMove {
                    px: value.position,
                    over_widget: over_widget(value.position, &input.panels, &input.cameras),
                });
            }
            WindowEvent::CursorLeft(value) if Some(value.window) == primary => {
                current_cursor = None;
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
                if let (Some(button), Some(px)) = (pointer_button(value.button), current_cursor) {
                    let over = over_widget(px, &input.panels, &input.cameras);
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
                if !runtime
                    .pressed_keys
                    .iter()
                    .any(|key| key.starts_with("Control"))
                {
                    events.push(InputEvent::Wheel { delta: value.y });
                }
            }
            WindowEvent::WindowFocused(value) if Some(value.window) == primary => {
                output.focus_feed.0 = value.focused;
                if !value.focused {
                    runtime.pressed_keys.clear();
                    events.push(InputEvent::FocusLost);
                }
            }
            WindowEvent::KeyboardFocusLost(_) => {
                output.focus_feed.0 = false;
                runtime.pressed_keys.clear();
                events.push(InputEvent::FocusLost);
            }
            _ => {}
        }
    }
    events.append(&mut output.input_events.0);
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
        .map(|(_, window)| Viewport {
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
                submit_order(&mut output.pending, order)
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
    let new_results = results.0.get(feeds.last_result_count..).unwrap_or_default();
    let result_records = result_values(new_results);
    results_feed.0.extend(result_records.iter().cloned());
    feeds.last_result_count = results.0.len();
    let record = FrameRecord {
        frame_index: feeds.frame_index,
        wall_ms: presented.frame_ns / 1_000_000,
        tick: presented.latest_tick,
        applied_seq: std::mem::take(&mut runtime.pending_sequences),
        selection,
        positions: presented
            .units
            .iter()
            .map(|unit| (unit.unit_id, unit.pos.x, unit.pos.y))
            .collect(),
        results: result_records,
        markers: markers.0.clone(),
        rings: rings.0.clone(),
        feedback: feedback.clone(),
    };
    feeds.recent.push_back(record.clone());
    while feeds.recent.len() > 64 {
        feeds.recent.pop_front();
    }
    if !record.applied_seq.is_empty() {
        feeds.retained_input.push_back(record);
        while feeds.retained_input.len() > 4096 {
            feeds.retained_input.pop_front();
        }
    }
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
impl Plugin for CoreAdapterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CoreRuntime>()
            .init_resource::<CorePreferences>()
            .init_resource::<CoreEffects>()
            .init_resource::<crate::camera::CameraConfiguration>()
            .init_resource::<crate::feedback::FrameFeedback>()
            .init_resource::<CoreInputQueue>()
            .init_resource::<FrameFeeds>()
            .add_message::<WindowEvent>()
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
            .init_resource::<PlayerIdFeed>()
            .init_resource::<crate::debug_feed::AlertPingsFeed>()
            .init_resource::<MarkersShown>()
            .init_resource::<RingsShown>()
            .add_systems(PreUpdate, apply_frame_clear)
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

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use crate::{
        acquire::Acquired, commands::CommandSendStatus, ingest::WelcomeInfo, replica::UnitReplica,
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
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
    use turts_net_protocol::Pos;

    static ADAPTER_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        ADAPTER_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn fixture_app() -> (App, Entity, Vec2) {
        fixture_app_with_ui(false)
    }

    fn acme_order_app() -> (App, Entity, Vec2) {
        let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/acme");
        let archive =
            PackageArchive::parse(&pack_dir(&package).expect("pack Acme")).expect("parse Acme");
        let content =
            crate::content::ClientContent::from_archives(&[archive]).expect("resolve Acme content");
        let acquired = Acquired {
            lockfile_hash: String::new(),
            visuals: HashMap::new(),
            map_size: (64.0, 64.0),
            unit_names: HashMap::new(),
            content: content.clone(),
        };
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
        let camera = CameraState {
            look_at: Vec2::new(32.0, 32.0),
            height: 40.0,
            pitch: (30.0f32 / 40.0).atan(),
            yaw: 0.0,
            vertical_fov: std::f32::consts::FRAC_PI_4,
        };
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
            .insert_resource(state)
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
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.finish();
        app.cleanup();
        (app, window, px)
    }

    fn fixture_app_with_ui(include_ui: bool) -> (App, Entity, Vec2) {
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
        let state = ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 1,
                tick_hz: 20,
                match_id: 1,
            }),
            content: Some(content),
            config: Default::default(),
        };
        let camera = CameraState {
            look_at: Vec2::new(32.0, 32.0),
            height: 40.0,
            pitch: (30.0f32 / 40.0).atan(),
            yaw: 0.0,
            vertical_fov: std::f32::consts::FRAC_PI_4,
        };
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
                crate::commands::PendingCommand::Order { order } => Some(order),
                _ => None,
            })
            .expect("catalog Ground smart rule plus actual right-click must submit an order");
        assert_eq!(order.order, "acme:order_move");
        assert_eq!(order.units, vec![UnitId(1)]);
        assert!(matches!(
            order.target,
            turts_net_protocol::OrderTarget::Point(_)
        ));

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
                crate::commands::PendingCommand::Order { order } => Some(order),
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
                crate::commands::PendingCommand::Order { order } => Some(order),
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
                crate::commands::PendingCommand::Order { order } => Some(order),
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
                crate::commands::PendingCommand::Order { order } => Some(order),
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
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), IsDefaultUiCamera))
            .id();
        app.world_mut()
            .get_mut::<Camera>(camera)
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
                bevy::ui::UiTargetCamera(camera),
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
            });
            feed.categories.insert("ui".into(), 0.5);
            world.resource_mut::<CursorFeed>().applied_icon = Some("owner:cursor".into());
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
