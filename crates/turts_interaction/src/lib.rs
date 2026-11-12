//! Renderer-independent input and interaction contracts for a player client.

mod bindings;
pub mod camera;
pub mod compose;
pub mod groups;
pub mod input_map;
pub mod minimap;
pub mod projection;
pub mod selection;
pub mod smart;
mod world_view;

use bindings::{logical_modifier, physical_modifiers};
use compose::{intended_units, send_explicit_units};
use glam::Vec2;
use groups::engine_group_action;
use std::collections::{BTreeMap, HashMap};
use turts_mod_api::{
    Behaviour, Capability, InputMapDef, Mod, OrderDef, OrderRulesDef, SmartTarget, TargetKind,
};
use turts_net_protocol::{
    JobId, NodeId, OrderEnvelope, OrderTarget, Pos, QueueMode, SlotId, UnitId,
};

pub use turts_mod_api::Button;
pub use world_view::{NodeView, UnitView, WorldView};

/// The plain state machine that turns frame input into client effects.
#[derive(Debug)]
pub struct Core {
    camera: CameraState,
    selected: Vec<UnitId>,
    groups: Vec<Vec<UnitId>>,
    keys: Vec<String>,
    physical_mods: Vec<Mod>,
    held_modifier_actions: HashMap<String, Vec<String>>,
    pointer_down: Option<(Button, Vec2, bool, u64)>,
    pointer: Option<(Vec2, bool)>,
    held_buttons: Vec<Button>,
    key_box_gesture: Option<(String, Vec2, u64)>,
    ui_capture: Option<bool>,
    pending_order: Option<String>,
    last_click: Option<(u64, Vec2, String)>,
    next_group: u32,
    camera_controller: camera::CameraController,
    /// Bounded evidence of named actions that actually reached the Core dispatch branch.
    dispatch_records: Vec<NamedDispatch>,
    dispatch_records_truncated: usize,
}

/// One named action observed at the interaction dispatch branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedDispatch {
    pub input_index: usize,
    pub origin: &'static str,
    pub code: Option<String>,
    pub matched_action: String,
    pub matched_order: Option<String>,
}

#[derive(Clone, Copy)]
struct DispatchSource<'a> {
    input_index: usize,
    origin: &'static str,
    code: Option<&'a str>,
}

impl Core {
    /// Creates the interaction core with the caller's initial camera pose.
    pub fn new(initial_camera: CameraState) -> Self {
        Self {
            camera: initial_camera,
            selected: Vec::new(),
            groups: vec![Vec::new(); 10],
            keys: Vec::new(),
            physical_mods: Vec::new(),
            held_modifier_actions: HashMap::new(),
            pointer_down: None,
            pointer: None,
            held_buttons: Vec::new(),
            key_box_gesture: None,
            ui_capture: None,
            pending_order: None,
            last_click: None,
            next_group: 1,
            camera_controller: camera::CameraController::default(),
            dispatch_records: Vec::new(),
            dispatch_records_truncated: 0,
        }
    }

    /// Returns the camera pose without allowing presentation to mutate core state.
    pub fn camera_state(&self) -> CameraState {
        self.camera
    }

    /// Read-only cursor context for the camera controller's active pan direction.
    pub fn camera_pan_context(&self) -> Option<&str> {
        self.camera_controller.pan_context()
    }

    /// Read-only presentation seam for the order currently awaiting a target.
    pub fn pending_order_id(&self) -> Option<&str> {
        self.pending_order.as_deref()
    }

    /// Actual named dispatches from the most recent `step`, capped to keep evidence bounded.
    pub fn dispatch_records(&self) -> &[NamedDispatch] {
        &self.dispatch_records
    }

    /// Number of actual dispatches omitted from the bounded most-recent-step evidence.
    pub fn dispatch_records_truncated(&self) -> usize {
        self.dispatch_records_truncated
    }

    /// Read-only observation seam for the active unit selection.
    ///
    /// Task 9 preparation stub: behavior is implemented after semantic red evidence is recorded.
    pub fn selected_units(&self) -> &[UnitId] {
        &self.selected
    }

    /// Read-only observation seam for a control group.
    ///
    /// Task 9 preparation stub: behavior is implemented after semantic red evidence is recorded.
    pub fn group_members(&self, group: u8) -> &[UnitId] {
        self.groups
            .get(group as usize)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Projects a ground point into screen space for callers and deterministic fixtures.
    ///
    /// Task 9 preparation stub: behavior is implemented after semantic red evidence is recorded.
    pub fn project_world_to_screen(&self, point: Vec2, view: &Viewport) -> Vec2 {
        projection::world_to_screen(point, view, self.camera).unwrap_or(Vec2::ZERO)
    }

    /// Processes one frame of input. Interaction behavior is added by later tasks.
    pub fn step(
        &mut self,
        events: &[InputEvent],
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.dispatch_records.clear();
        self.dispatch_records_truncated = 0;
        self.prune(world);
        self.ui_capture = events.iter().find_map(|event| match event {
            InputEvent::UiCapture { allow_shell } => Some(*allow_shell),
            _ => None,
        });
        let mut ignored_wheel_inputs = Vec::new();
        let mut wheel_camera_actions = Vec::new();
        for (input_index, event) in events.iter().enumerate() {
            match event {
                InputEvent::UiCapture { allow_shell } => self.begin_ui_capture(*allow_shell),
                InputEvent::KeyDown { code, mods } => {
                    self.on_key_down(input_index, code, mods, world, view, now_ms, &mut effects);
                }
                InputEvent::KeyUp { code } => {
                    self.on_key_up(code, world, view, &mut effects);
                }
                InputEvent::FocusLost => {
                    self.on_focus_lost();
                }
                InputEvent::UiAction { name } => {
                    self.on_ui_action(input_index, name, world, view, now_ms, &mut effects);
                }
                InputEvent::UiCameraCenter { position } => {
                    self.on_ui_camera_center(position, world, &mut effects);
                }
                InputEvent::UiOrder {
                    order,
                    units,
                    target,
                } => {
                    self.on_ui_order(order, units, target, world, &mut effects);
                }
                InputEvent::RestoreGroups { groups } => {
                    self.on_restore_groups(groups, world, &mut effects);
                }
                InputEvent::PointerDown {
                    button,
                    px,
                    over_widget,
                } => {
                    self.on_pointer_down(button, px, over_widget, world, now_ms);
                }
                InputEvent::PointerMove { px, over_widget } => {
                    self.on_pointer_move(px, over_widget);
                }
                InputEvent::PointerUp {
                    button,
                    px,
                    over_widget,
                } => {
                    self.on_pointer_up(
                        input_index,
                        button,
                        px,
                        over_widget,
                        world,
                        view,
                        now_ms,
                        &mut effects,
                    );
                }
                InputEvent::PointerLeft => {
                    self.on_pointer_left();
                }
                InputEvent::Wheel { delta } => {
                    self.on_wheel(
                        input_index,
                        delta,
                        world,
                        view,
                        now_ms,
                        &mut ignored_wheel_inputs,
                        &mut wheel_camera_actions,
                        &mut effects,
                    );
                }
                InputEvent::Minimap { button, local } => {
                    self.on_minimap(button, local, world, view, now_ms, &mut effects);
                }
            }
        }
        self.prune(world);
        let mut held_camera_actions: Vec<_> = self
            .keys
            .iter()
            .filter_map(|code| self.action_for_key(code, world))
            .filter(|action| camera_pan_action(action))
            .collect();
        held_camera_actions.extend(
            self.held_buttons
                .iter()
                .filter_map(|button| self.action_for_pointer(*button, world))
                .filter(|action| camera_pan_action(action)),
        );
        held_camera_actions.extend(wheel_camera_actions);
        if self.ui_capture.is_some() {
            held_camera_actions.clear();
        }
        self.camera_controller.step(
            &mut self.camera,
            events.iter().enumerate().filter_map(|(index, event)| {
                (!ignored_wheel_inputs.contains(&index)
                    && (self.ui_capture.is_none()
                        || matches!(
                            event,
                            InputEvent::UiCapture { .. }
                                | InputEvent::FocusLost
                                | InputEvent::PointerLeft
                        )))
                .then_some(event)
            }),
            world,
            view,
            now_ms,
            &self.selected,
            &held_camera_actions,
            &mut effects,
        );
        self.held_modifier_actions.remove("wheel:pulse");
        effects
    }

    /// The `InputEvent::KeyDown` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    #[allow(clippy::too_many_arguments, clippy::ptr_arg)] // the arm's bindings, as the arm used them
    fn on_key_down(
        &mut self,
        input_index: usize,
        code: &String,
        mods: &Vec<Mod>,
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) {
        if !self.keys.contains(code) {
            self.keys.push(code.clone());
        }
        self.physical_mods = mods.clone();
        let modifier_actions = self.modifier_actions_for_key(code, world);
        if !modifier_actions.is_empty() {
            self.held_modifier_actions
                .insert(code.clone(), modifier_actions);
        }
        let action = self.action_for_key(code, world);
        if let Some(action) = action {
            self.dispatch_named_action(
                &action,
                DispatchSource {
                    input_index,
                    origin: "key",
                    code: Some(code),
                },
                world,
                view,
                now_ms,
                effects,
            );
        }
    }

    /// The `InputEvent::KeyUp` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_key_up(
        &mut self,
        code: &String,
        world: &dyn WorldView,
        view: &Viewport,
        effects: &mut Vec<Effect>,
    ) {
        if self.ui_capture.is_none()
            && self
                .key_box_gesture
                .as_ref()
                .is_some_and(|(key, _, _)| key == code)
        {
            let (_, start, down_ms) = self.key_box_gesture.take().expect("matched key gesture");
            if let Some((px, false)) = self.pointer {
                self.complete_selection_gesture(
                    start,
                    px,
                    start.distance(px) > 4.0,
                    down_ms,
                    world,
                    view,
                    effects,
                );
            }
        }
        self.keys.retain(|key| key != code);
        self.held_modifier_actions.remove(code);
        self.physical_mods = physical_modifiers(&self.keys);
    }

    /// The `InputEvent::FocusLost` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_focus_lost(&mut self) {
        self.keys.clear();
        self.physical_mods.clear();
        self.held_modifier_actions.clear();
        self.pointer_down = None;
        self.pointer = None;
        self.held_buttons.clear();
        self.key_box_gesture = None;
    }

    /// The `InputEvent::UiAction` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_ui_action(
        &mut self,
        input_index: usize,
        name: &String,
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) {
        if self
            .ui_capture
            .is_some_and(|allow_shell| !allow_shell || !shell_action(name))
        {
            return;
        }
        if let Some(unit_text) = name.strip_prefix("ui_select:") {
            if let Ok(id) = unit_text.parse::<u32>()
                && world
                    .units()
                    .iter()
                    .any(|unit| unit.id == UnitId(id) && unit.owner_slot == world.own_slot())
            {
                self.selected.clear();
                self.selected.push(UnitId(id));
                effects.push(Effect::SelectionChanged);
            }
        } else if let Some(order) = world.orders().iter().find(|order| order.id == *name) {
            self.record_named_dispatch(input_index, "ui", None, name, Some(&order.id));
            self.dispatch_order(order, world, effects);
        } else {
            self.dispatch_named_action(
                name,
                DispatchSource {
                    input_index,
                    origin: "ui",
                    code: None,
                },
                world,
                view,
                now_ms,
                effects,
            );
        }
    }

    /// The `InputEvent::UiCameraCenter` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_ui_camera_center(
        &mut self,
        position: &Vec2,
        world: &dyn WorldView,
        effects: &mut Vec<Effect>,
    ) {
        if self.ui_capture.is_some() {
            return;
        }
        if self
            .camera_controller
            .centre_world(&mut self.camera, *position, world)
        {
            effects.push(Effect::CameraChanged);
        }
    }

    /// The `InputEvent::UiOrder` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_ui_order(
        &mut self,
        order: &String,
        units: &[UnitId],
        target: &UiOrderTarget,
        world: &dyn WorldView,
        effects: &mut Vec<Effect>,
    ) {
        if self.ui_capture.is_some() {
            return;
        }
        let Some(definition) = world
            .orders()
            .iter()
            .find(|definition| definition.id == *order)
        else {
            return;
        };
        let (target_kind, protocol_target) = match target {
            UiOrderTarget::Content(id) => (TargetKind::Content, OrderTarget::Content(id.clone())),
            UiOrderTarget::Job(id) => (TargetKind::Job, OrderTarget::Job(*id)),
        };
        if definition.target != target_kind {
            return;
        }
        let Some(valid_units) = intended_units(definition, units, world) else {
            return;
        };
        let valid_units: Vec<_> = valid_units
            .into_iter()
            .filter(|id| {
                world
                    .units()
                    .iter()
                    .find(|unit| unit.id == *id)
                    .is_some_and(|unit| match target {
                        UiOrderTarget::Content(content) => world.can_produce(unit, content),
                        UiOrderTarget::Job(job) => world.has_job(unit, *job),
                    })
            })
            .collect();
        if valid_units.is_empty() {
            return;
        }
        let group = if valid_units.len() > 1 {
            self.alloc_group()
        } else {
            0
        };
        send_explicit_units(
            definition,
            &valid_units,
            protocol_target,
            QueueMode::Replace,
            group,
            effects,
        );
    }

    /// The `InputEvent::RestoreGroups` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_restore_groups(
        &mut self,
        groups: &Vec<(u8, Vec<UnitId>)>,
        world: &dyn WorldView,
        effects: &mut Vec<Effect>,
    ) {
        self.groups = vec![Vec::new(); 10];
        for (group, members) in groups {
            if !(1..=9).contains(group) {
                continue;
            }
            let mut seen = std::collections::HashSet::new();
            self.groups[*group as usize] = members
                .iter()
                .copied()
                .filter(|id| {
                    seen.insert(*id)
                        && world
                            .units()
                            .iter()
                            .any(|unit| unit.id == *id && unit.owner_slot == world.own_slot())
                })
                .collect();
        }
        effects.push(Effect::GroupsChanged);
    }

    /// The `InputEvent::PointerDown` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_pointer_down(
        &mut self,
        button: &Button,
        px: &Vec2,
        over_widget: &bool,
        world: &dyn WorldView,
        now_ms: u64,
    ) {
        if let Some(allow_shell) = self.ui_capture {
            if allow_shell && *button != Button::Left {
                self.pointer_down = Some((*button, *px, false, now_ms));
            }
            return;
        }
        self.pointer = Some((*px, *over_widget));
        if !over_widget {
            if !self.held_buttons.contains(button) {
                self.held_buttons.push(*button);
            }
            let modifiers = self.modifier_actions_for_pointer(*button, world);
            if !modifiers.is_empty() {
                self.held_modifier_actions
                    .insert(format!("pointer:{button:?}"), modifiers);
            }
            self.pointer_down = Some((*button, *px, false, now_ms));
        }
    }

    /// The `InputEvent::PointerMove` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_pointer_move(&mut self, px: &Vec2, over_widget: &bool) {
        if self.ui_capture.is_some() {
            return;
        }
        self.pointer = Some((*px, *over_widget));
        if let Some((button, start, dragged, down_ms)) = self.pointer_down {
            self.pointer_down =
                Some((button, start, dragged || start.distance(*px) > 4.0, down_ms));
        }
    }

    /// The `InputEvent::PointerUp` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    #[allow(clippy::too_many_arguments)] // the arm's bindings, as the arm used them
    fn on_pointer_up(
        &mut self,
        input_index: usize,
        button: &Button,
        px: &Vec2,
        over_widget: &bool,
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) {
        if self.ui_capture.is_none() {
            self.pointer = Some((*px, *over_widget));
        }
        self.held_buttons.retain(|held| held != button);
        self.held_modifier_actions
            .remove(&format!("pointer:{button:?}"));
        let Some((down_button, start, dragged, down_ms)) = self.pointer_down.take() else {
            return;
        };
        if let Some(allow_shell) = self.ui_capture {
            if allow_shell
                && down_button == *button
                && *button != Button::Left
                && let Some(action) = self.action_for_pointer(*button, world)
                && shell_action(&action)
            {
                self.dispatch_named_action(
                    &action,
                    DispatchSource {
                        input_index,
                        origin: "pointer",
                        code: None,
                    },
                    world,
                    view,
                    now_ms,
                    effects,
                );
            }
            return;
        }
        if *over_widget || down_button != *button {
            return;
        }
        let dragged = dragged || start.distance(*px) > 4.0;
        let pointer_action = self.action_for_pointer(down_button, world);
        if matches!(pointer_action.as_deref(), Some("select" | "box_select")) {
            self.complete_selection_gesture(start, *px, dragged, down_ms, world, view, effects);
        } else if let Some(action) = pointer_action {
            self.dispatch_named_action(
                &action,
                DispatchSource {
                    input_index,
                    origin: "pointer",
                    code: None,
                },
                world,
                view,
                now_ms,
                effects,
            );
        } else if down_button == Button::Right && self.pending_order.take().is_some() {
            effects.push(Effect::Cursor {
                context: "default".into(),
            });
        }
    }

    /// The `InputEvent::PointerLeft` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_pointer_left(&mut self) {
        self.pointer_down = None;
        self.pointer = None;
        self.key_box_gesture = None;
        self.held_buttons.clear();
        self.held_modifier_actions
            .retain(|key, _| !key.starts_with("pointer:"));
    }

    /// The `InputEvent::Wheel` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    #[allow(clippy::too_many_arguments)] // the arm's bindings, as the arm used them
    fn on_wheel(
        &mut self,
        input_index: usize,
        delta: &f32,
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
        ignored_wheel_inputs: &mut Vec<usize>,
        wheel_camera_actions: &mut Vec<String>,
        effects: &mut Vec<Effect>,
    ) {
        if self.ui_capture.is_some() || self.physical_mods.contains(&Mod::Ctrl) {
            ignored_wheel_inputs.push(input_index);
        } else if delta.is_finite()
            && *delta != 0.0
            && let Some(action) = self.action_for_wheel(*delta, world)
        {
            if matches!(action.as_str(), "zoom_in" | "zoom_out") {
                self.record_named_dispatch(input_index, "wheel", None, &action, None);
            } else {
                if camera_pan_action(&action) {
                    wheel_camera_actions.push(action.clone());
                }
                if logical_modifier(&action).is_some() {
                    self.held_modifier_actions
                        .insert("wheel:pulse".into(), vec![action.clone()]);
                }
                self.dispatch_named_action(
                    &action,
                    DispatchSource {
                        input_index,
                        origin: "wheel",
                        code: None,
                    },
                    world,
                    view,
                    now_ms,
                    effects,
                );
            }
        }
    }

    /// The `InputEvent::Minimap` arm of `step`, moved out of it with its body unchanged (L0-CORE).
    fn on_minimap(
        &mut self,
        button: &Button,
        local: &[f32; 2],
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) {
        if self.ui_capture.is_some() {
            return;
        }
        minimap::handle_input(self, *button, *local, world, view, now_ms, effects);
    }

    fn begin_ui_capture(&mut self, allow_shell: bool) {
        self.ui_capture = Some(allow_shell);
        self.pointer = None;
        self.pointer_down = None;
        self.key_box_gesture = None;
        self.camera_controller.suspend_pointer();
    }

    fn record_named_dispatch(
        &mut self,
        input_index: usize,
        origin: &'static str,
        code: Option<&str>,
        action: &str,
        matched_order: Option<&str>,
    ) {
        const MAX_DISPATCH_RECORDS_PER_STEP: usize = 64;
        if self.dispatch_records.len() >= MAX_DISPATCH_RECORDS_PER_STEP {
            self.dispatch_records_truncated = self.dispatch_records_truncated.saturating_add(1);
            return;
        }
        self.dispatch_records.push(NamedDispatch {
            input_index,
            origin,
            code: code.map(str::to_owned),
            matched_action: action.to_owned(),
            matched_order: matched_order.map(str::to_owned),
        });
    }

    fn dispatch_named_action(
        &mut self,
        action: &str,
        source: DispatchSource<'_>,
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) {
        if self
            .ui_capture
            .is_some_and(|allow_shell| !allow_shell || !shell_action(action))
        {
            return;
        }
        if matches!(action, "select" | "box_select" | "command") {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            let Some((px, false)) = self.pointer else {
                return;
            };
            if action == "command" {
                if self.pending_order.take().is_some() {
                    effects.push(Effect::Cursor {
                        context: "default".into(),
                    });
                } else {
                    self.smart_command(px, view, world, effects);
                }
            } else if action == "box_select" && source.origin == "key" {
                if let Some(code) = source.code {
                    self.key_box_gesture = Some((code.into(), px, now_ms));
                }
            } else {
                self.complete_selection_gesture(px, px, false, now_ms, world, view, effects);
            }
        } else if matches!(action, "zoom_in" | "zoom_out") {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            let steps = if action == "zoom_in" { 1.0 } else { -1.0 };
            if camera::zoom_by_steps(&mut self.camera, world, steps) {
                effects.push(Effect::CameraChanged);
            }
        } else if camera_pan_action(action) || logical_modifier(action).is_some() {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
        } else if let Some(group) = engine_group_action(action, "group_assign_") {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            self.assign_group(group, false, world);
            effects.push(Effect::GroupsChanged);
        } else if let Some(group) = engine_group_action(action, "group_add_") {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            self.assign_group(group, true, world);
            effects.push(Effect::GroupsChanged);
        } else if let Some(group) = engine_group_action(action, "group_recall_") {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            self.recall_group(group, world);
            effects.push(Effect::SelectionChanged);
            if self.camera_controller.group_recall_centres(
                action,
                now_ms,
                world.interaction_settings().double_click_ms,
                &mut self.camera,
                &self.selected,
                world,
            ) {
                effects.push(Effect::CameraChanged);
            }
        } else if let Some(order) = world.orders().iter().find(|order| order.hotkey == action) {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                Some(&order.id),
            );
            self.dispatch_order(order, world, effects);
        } else if action == "centre_selection" {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            if self
                .camera_controller
                .centre_selection(&mut self.camera, &self.selected, world)
            {
                effects.push(Effect::CameraChanged);
            }
        } else if action == "menu" || action == "toggle_guide" {
            if action == "menu" {
                self.begin_ui_capture(true);
            }
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            effects.push(Effect::ShellAction {
                action: action.into(),
            });
        } else if action == "cancel" {
            self.record_named_dispatch(
                source.input_index,
                source.origin,
                source.code,
                action,
                None,
            );
            if self.pending_order.take().is_some() {
                effects.push(Effect::Cursor {
                    context: "default".into(),
                });
            } else {
                self.begin_ui_capture(true);
                effects.push(Effect::ShellAction {
                    action: "menu".into(),
                });
            }
        }
    }

    fn dispatch_order(
        &mut self,
        order: &OrderDef,
        world: &dyn WorldView,
        effects: &mut Vec<Effect>,
    ) {
        if order.target == TargetKind::None {
            self.pending_order = None;
            self.compose_explicit(order, OrderTarget::None, world, effects);
            effects.push(Effect::Cursor {
                context: "default".into(),
            });
        } else {
            self.pending_order = Some(order.id.clone());
            effects.push(Effect::Cursor {
                context: order.cursor.clone().unwrap_or_else(|| "target".into()),
            });
        }
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new(CameraState::default())
    }
}

fn camera_pan_action(action: &str) -> bool {
    matches!(
        action,
        "camera_pan_up" | "camera_pan_down" | "camera_pan_left" | "camera_pan_right"
    )
}

fn shell_action(action: &str) -> bool {
    matches!(action, "menu" | "cancel" | "toggle_guide")
}

/// Input observed by the client adapter during one frame.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Per-frame UI scope. Keep physical key bookkeeping; suspend world actions.
    UiCapture {
        allow_shell: bool,
    },
    KeyDown {
        code: String,
        mods: Vec<Mod>,
    },
    KeyUp {
        code: String,
    },
    PointerMove {
        px: Vec2,
        over_widget: bool,
    },
    PointerDown {
        button: Button,
        px: Vec2,
        over_widget: bool,
    },
    PointerUp {
        button: Button,
        px: Vec2,
        over_widget: bool,
    },
    PointerLeft,
    Wheel {
        delta: f32,
    },
    FocusLost,
    UiAction {
        name: String,
    },
    /// Internal typed camera action from a rendered client widget.
    UiCameraCenter {
        position: Vec2,
    },
    /// Typed content/job order from an authored UI control or deterministic fixture.
    /// The supplied unit set is validated against ownership, order eligibility, and target facts.
    UiOrder {
        order: String,
        units: Vec<UnitId>,
        target: UiOrderTarget,
    },
    /// Restore persisted control groups after validating membership against the joined world.
    RestoreGroups {
        groups: Vec<(u8, Vec<UnitId>)>,
    },
    /// Pointer input on the minimap, with widget-local coordinates in `0..1`.
    Minimap {
        button: Button,
        local: [f32; 2],
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiOrderTarget {
    Content(String),
    Job(JobId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractionSettings {
    pub double_click_ms: u64,
    pub double_click_pixels: f32,
}

impl Default for InteractionSettings {
    fn default() -> Self {
        Self {
            double_click_ms: 400,
            double_click_pixels: 4.0,
        }
    }
}

/// Logical viewport dimensions and device scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub size: Vec2,
    pub scale_factor: f32,
}

/// The renderer-independent camera pose held by the interaction core.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    pub look_at: Vec2,
    pub height: f32,
    /// Angle in radians from the vertical axis.
    pub pitch: f32,
    pub yaw: f32,
    pub vertical_fov: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            look_at: Vec2::ZERO,
            height: 40.0,
            pitch: 0.643_501_1,
            yaw: 0.0,
            vertical_fov: 0.9,
        }
    }
}

/// An output for the client adapter to apply or present.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Send(OrderEnvelope),
    SelectionChanged,
    GroupsChanged,
    CameraChanged,
    Marker {
        point: Vec2,
        kind: String,
    },
    Sound {
        id: String,
        category: String,
    },
    Notice {
        text_key: String,
        units: Vec<UnitId>,
    },
    Cursor {
        context: String,
    },
    ShellAction {
        action: String,
    },
}
