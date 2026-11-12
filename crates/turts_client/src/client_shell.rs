//! Shared F10/Escape settings overlay. Task16 composes its additional controls here.

use std::collections::BTreeMap;

use bevy::input_focus::FocusedInput;
use bevy::prelude::*;
use bevy::ui_widgets::{
    Activate, Button as UiButton, ButtonPlugin, Slider, SliderRange, SliderStep, SliderThumb,
    SliderValue,
};
use bevy::window::{PrimaryWindow, WindowEvent};
use turts_mod_api::Binding;

use crate::{
    core_adapter::{CoreAdapterSet, CoreEffects, CorePreferences},
    debug_feed::NoticesFeed,
    feedback::FeedbackSet,
    ingest::ReplicaState,
    join::{AcquiredPackages, Phase},
    rejoin::RejoinController,
    ui_shared::{UiCaptureRoot, UiPresentationSet},
    volume_settings::{VolumeSettings, spawn_volume_controls},
};

#[derive(Component)]
pub(crate) struct SettingsMenuOverlay;

/// The width of the action label column of the menu. An order name of up to 20 characters
/// ("Disable this machine", 192 px in the 16 px default font) stays on one line, so each
/// single-key row keeps the same height.
const MENU_ACTION_LABEL_WIDTH: f32 = 200.0;
/// The start width of the key column. It grows into the free width of the row; the label, the
/// keys and the two buttons fit in the 476 px of the panel without a shrink.
const MENU_BINDING_BASIS_WIDTH: f32 = 120.0;
/// The fixed height of the binding feedback block: one 14 px status line (20 px), the gap
/// (8 px) and the Swap/Cancel row (24 px). The rows do not move when the text shows or clears.
const BINDING_FEEDBACK_HEIGHT: f32 = 52.0;

#[derive(Component)]
pub(crate) struct RejoinOverlayCamera;

#[derive(Component)]
pub(crate) struct FocusOverlay;

#[derive(Component, Clone)]
struct BindingButton {
    action: String,
    kind: BindingButtonKind,
}

#[derive(Component, Clone, Copy)]
enum BindingButtonKind {
    Change,
    Reset,
    Swap,
    Cancel,
}

/// Physical-code labels supplied once by the web entry. An empty map uses US labels.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingLabels(pub BTreeMap<String, String>);

impl BindingLabels {
    /// Build display labels from KeyboardLayoutMap entries. The API returns lowercase letters
    /// for letter positions; the HUD presents those as uppercase key names.
    pub fn from_layout_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self(
            entries
                .into_iter()
                .filter_map(|(code, label)| {
                    if label.is_empty()
                        || label.chars().all(char::is_whitespace)
                        || label.chars().any(char::is_control)
                    {
                        return None;
                    }
                    let label = if label.len() == 1 && label.as_bytes()[0].is_ascii_lowercase() {
                        label.to_ascii_uppercase()
                    } else {
                        label
                    };
                    Some((code, label))
                })
                .collect(),
        )
    }
}

/// Format one binding for display while preserving the physical code used by dispatch.
pub(crate) fn binding_label(binding: &Binding, labels: Option<&BindingLabels>) -> String {
    let modifiers = |mods: &[turts_mod_api::Mod]| {
        [
            turts_mod_api::Mod::Ctrl,
            turts_mod_api::Mod::Shift,
            turts_mod_api::Mod::Alt,
        ]
        .into_iter()
        .filter(|modifier| mods.contains(modifier))
        .map(|modifier| match modifier {
            turts_mod_api::Mod::Ctrl => "Ctrl+",
            turts_mod_api::Mod::Shift => "Shift+",
            turts_mod_api::Mod::Alt => "Alt+",
        })
        .collect::<String>()
    };
    match binding {
        Binding::Key { code, mods } => format!("{}{}", modifiers(mods), key_label(code, labels)),
        Binding::Pointer { button, mods } => format!(
            "{}{} click",
            modifiers(mods),
            match button {
                turts_mod_api::Button::Left => "Left",
                turts_mod_api::Button::Right => "Right",
                turts_mod_api::Button::Middle => "Middle",
                turts_mod_api::Button::Back => "Back",
                turts_mod_api::Button::Forward => "Forward",
            }
        ),
        Binding::Wheel(direction) => format!(
            "Wheel {}",
            match direction {
                turts_mod_api::WheelDirection::Up => "up",
                turts_mod_api::WheelDirection::Down => "down",
            }
        ),
    }
}

fn key_label(code: &str, labels: Option<&BindingLabels>) -> String {
    labels
        .and_then(|map| map.0.get(code))
        .cloned()
        .unwrap_or_else(|| match code {
            code if code.starts_with("Key") && code.len() == 4 => code[3..].to_owned(),
            code if code.starts_with("Digit") && code.len() == 6 => code[5..].to_owned(),
            "Escape" => "Esc".to_owned(),
            "Space" => "Space".to_owned(),
            "ArrowUp" => "Up".to_owned(),
            "ArrowDown" => "Down".to_owned(),
            "ArrowLeft" => "Left".to_owned(),
            "ArrowRight" => "Right".to_owned(),
            other => other.to_owned(),
        })
}

#[derive(Resource, Default)]
pub(crate) struct BindingEditorState {
    pub(crate) capturing: Option<String>,
    conflict: Option<(String, Binding, Vec<String>)>,
    status: Option<String>,
    capture_after_message: Option<usize>,
}

#[derive(Clone, Copy)]
struct PointerReleaseBoundary {
    message_id: usize,
    window: Entity,
    button: bevy::picking::pointer::PointerButton,
}

#[derive(Clone)]
struct BindingInputRecord {
    message_id: usize,
    event: WindowEvent,
    cursor_position: Vec2,
    modifier_snapshot: CaptureModifiers,
    release: Option<PointerReleaseBoundary>,
    keyboard: Option<(Entity, KeyCode, bevy::input::ButtonState)>,
    released_modifier_press: Option<usize>,
}

#[derive(Clone, Copy)]
struct PendingBindingActivation {
    entity: Entity,
    boundary: Option<usize>,
}

#[derive(Resource)]
struct BindingInputTimeline {
    cursor_position: Vec2,
    modifiers: CaptureModifiers,
    records: Vec<BindingInputRecord>,
    claimed_releases: std::collections::HashSet<usize>,
    claimed_presses: std::collections::HashSet<usize>,
    pointer_press_causes: Vec<(
        Entity,
        bevy::picking::pointer::PointerId,
        bevy::picking::pointer::PointerButton,
        usize,
    )>,
    claimed_keys: std::collections::HashSet<usize>,
    pending_activations: Vec<PendingBindingActivation>,
    last_message_id: Option<usize>,
    modifier_press_ids: std::collections::HashMap<KeyCode, usize>,
}

impl Default for BindingInputTimeline {
    fn default() -> Self {
        Self {
            cursor_position: Vec2::ZERO,
            modifiers: CaptureModifiers::default(),
            records: Vec::new(),
            claimed_releases: Default::default(),
            claimed_presses: Default::default(),
            pointer_press_causes: Vec::new(),
            claimed_keys: Default::default(),
            pending_activations: Vec::new(),
            last_message_id: None,
            modifier_press_ids: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CaptureModifiers {
    ctrl_left: bool,
    ctrl_right: bool,
    shift_left: bool,
    shift_right: bool,
    alt_left: bool,
    alt_right: bool,
}

impl CaptureModifiers {
    #[cfg(test)]
    fn from_keys(keys: Option<&ButtonInput<KeyCode>>) -> Self {
        let is_pressed = |code| keys.is_some_and(|keys| keys.pressed(code));
        Self {
            ctrl_left: is_pressed(KeyCode::ControlLeft),
            ctrl_right: is_pressed(KeyCode::ControlRight),
            shift_left: is_pressed(KeyCode::ShiftLeft),
            shift_right: is_pressed(KeyCode::ShiftRight),
            alt_left: is_pressed(KeyCode::AltLeft),
            alt_right: is_pressed(KeyCode::AltRight),
        }
    }

    fn update_from_event(&mut self, event: &WindowEvent, primary: Option<Entity>) -> bool {
        let WindowEvent::KeyboardInput(input) = event else {
            if matches!(event, WindowEvent::KeyboardFocusLost(_))
                || matches!(event, WindowEvent::WindowFocused(focus) if Some(focus.window) == primary && !focus.focused)
            {
                *self = Self::default();
                return true;
            }
            return false;
        };
        // KeyboardInput is window-agnostic once copied to ButtonInput. Keep the capture
        // timeline primary-window-only before changing any modifier state.
        if Some(input.window) != primary {
            return false;
        }
        if input.repeat {
            return false;
        }
        let pressed = input.state == bevy::input::ButtonState::Pressed;
        let modifier = match input.key_code {
            KeyCode::ControlLeft => &mut self.ctrl_left,
            KeyCode::ControlRight => &mut self.ctrl_right,
            KeyCode::ShiftLeft => &mut self.shift_left,
            KeyCode::ShiftRight => &mut self.shift_right,
            KeyCode::AltLeft => &mut self.alt_left,
            KeyCode::AltRight => &mut self.alt_right,
            _ => return false,
        };
        *modifier = pressed;
        true
    }

    fn as_mods(self) -> Vec<turts_mod_api::Mod> {
        let mut mods = Vec::new();
        if self.ctrl_left || self.ctrl_right {
            mods.push(turts_mod_api::Mod::Ctrl);
        }
        if self.shift_left || self.shift_right {
            mods.push(turts_mod_api::Mod::Shift);
        }
        if self.alt_left || self.alt_right {
            mods.push(turts_mod_api::Mod::Alt);
        }
        mods
    }
}

fn record_binding_input_timeline(
    mut messages: MessageReader<WindowEvent>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut timeline: ResMut<BindingInputTimeline>,
) {
    let primary = windows.iter().next();
    timeline.records.clear();
    timeline.claimed_releases.clear();
    timeline.claimed_presses.clear();
    timeline.claimed_keys.clear();
    timeline.pending_activations.clear();

    for (event, message_id) in messages.read_with_id() {
        let message_id = message_id.id;
        timeline.last_message_id = Some(message_id);
        if let WindowEvent::CursorMoved(moved) = event
            && Some(moved.window) == primary
        {
            // Match bevy_picking::input::mouse_pick_events cursor-last behavior.
            timeline.cursor_position = moved.position;
        }
        if matches!(event, WindowEvent::KeyboardFocusLost(_))
            || matches!(event, WindowEvent::WindowFocused(focus) if Some(focus.window) == primary && !focus.focused)
        {
            timeline.pointer_press_causes.clear();
            timeline.modifier_press_ids.clear();
        }
        timeline.modifiers.update_from_event(event, primary);
        let released_modifier_press = match event {
            WindowEvent::KeyboardInput(input)
                if Some(input.window) == primary
                    && !input.repeat
                    && is_capture_modifier(input.key_code) =>
            {
                if input.state == bevy::input::ButtonState::Pressed {
                    timeline
                        .modifier_press_ids
                        .insert(input.key_code, message_id);
                    None
                } else {
                    timeline.modifier_press_ids.remove(&input.key_code)
                }
            }
            _ => None,
        };
        let modifier_snapshot = timeline.modifiers;
        let keyboard = match event {
            WindowEvent::KeyboardInput(input) => Some((input.window, input.key_code, input.state)),
            _ => None,
        };
        let release = match event {
            WindowEvent::MouseButtonInput(input)
                if Some(input.window) == primary
                    && input.state == bevy::input::ButtonState::Released =>
            {
                let button = match input.button {
                    bevy::input::mouse::MouseButton::Left => {
                        Some(bevy::picking::pointer::PointerButton::Primary)
                    }
                    bevy::input::mouse::MouseButton::Right => {
                        Some(bevy::picking::pointer::PointerButton::Secondary)
                    }
                    bevy::input::mouse::MouseButton::Middle => {
                        Some(bevy::picking::pointer::PointerButton::Middle)
                    }
                    _ => None,
                };
                button.map(|button| PointerReleaseBoundary {
                    message_id,
                    window: input.window,
                    button,
                })
            }
            _ => None,
        };
        let cursor_position = timeline.cursor_position;
        timeline.records.push(BindingInputRecord {
            message_id,
            event: event.clone(),
            cursor_position,
            modifier_snapshot,
            released_modifier_press,
            release,
            keyboard,
        });
    }
}

fn is_capture_modifier(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
    )
}

#[cfg(test)]
fn captured_binding(
    event: &WindowEvent,
    primary: Option<Entity>,
    modifiers: &mut CaptureModifiers,
) -> Option<Binding> {
    if modifiers.update_from_event(event, primary) {
        return None;
    }
    captured_binding_for_snapshot(event, primary, *modifiers)
}

fn captured_binding_for_snapshot(
    event: &WindowEvent,
    primary: Option<Entity>,
    modifiers: CaptureModifiers,
) -> Option<Binding> {
    let mods = modifiers.as_mods();
    match event {
        WindowEvent::KeyboardInput(input)
            if Some(input.window) == primary
                && input.state == bevy::input::ButtonState::Pressed
                && !input.repeat
                && !matches!(
                    input.key_code,
                    KeyCode::SuperLeft
                        | KeyCode::SuperRight
                        | KeyCode::ControlLeft
                        | KeyCode::ControlRight
                        | KeyCode::ShiftLeft
                        | KeyCode::ShiftRight
                        | KeyCode::AltLeft
                        | KeyCode::AltRight
                ) =>
        {
            Some(Binding::Key {
                code: format!("{:?}", input.key_code),
                mods,
            })
        }
        WindowEvent::MouseButtonInput(input)
            if Some(input.window) == primary
                && input.state == bevy::input::ButtonState::Pressed =>
        {
            let button = match input.button {
                bevy::input::mouse::MouseButton::Left => turts_mod_api::Button::Left,
                bevy::input::mouse::MouseButton::Right => turts_mod_api::Button::Right,
                bevy::input::mouse::MouseButton::Middle => turts_mod_api::Button::Middle,
                bevy::input::mouse::MouseButton::Back => turts_mod_api::Button::Back,
                bevy::input::mouse::MouseButton::Forward => turts_mod_api::Button::Forward,
                _ => return None,
            };
            Some(Binding::Pointer { button, mods })
        }
        _ => None,
    }
}

#[derive(Component, Clone)]
struct BindingValueText {
    action: String,
}

#[derive(Component)]
struct BindingFeedbackText;

#[derive(Component)]
struct BindingConflictPrompt;

#[derive(Component, Clone, Copy)]
enum SettingsToggle {
    EdgeScroll,
    CursorLock,
    HealthBars,
}

#[derive(Component, Clone, Copy)]
struct SettingsToggleText;

#[derive(Component, Clone, Copy)]
struct SettingsToggleState(SettingsToggle);

#[derive(Component, Clone, Copy)]
enum SettingsSlider {
    ScrollSpeed,
    UiScale,
    DoubleClick,
    DoubleClickPixels,
}

/// Choice presented when a newly captured chord is already bound to another action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingConflictChoice {
    Swap,
    Cancel,
}

/// Outcome used by the Task16 binding editor when changing or resetting one action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BindingEditResult {
    Applied,
    Conflict { actions: Vec<String> },
    Cancelled,
    Reserved,
    UnknownAction,
}

fn same_physical_chord(left: &Binding, right: &Binding) -> bool {
    fn mods_equal(left: &[turts_mod_api::Mod], right: &[turts_mod_api::Mod]) -> bool {
        left.len() == right.len() && left.iter().all(|modifier| right.contains(modifier))
    }
    match (left, right) {
        (
            Binding::Key {
                code: left,
                mods: lm,
            },
            Binding::Key {
                code: right,
                mods: rm,
            },
        ) => left == right && mods_equal(lm, rm),
        (
            Binding::Pointer {
                button: left,
                mods: lm,
            },
            Binding::Pointer {
                button: right,
                mods: rm,
            },
        ) => left == right && mods_equal(lm, rm),
        (Binding::Wheel(left), Binding::Wheel(right)) => left == right,
        _ => false,
    }
}

fn contains_physical_chord(bindings: &[Binding], binding: &Binding) -> bool {
    bindings
        .iter()
        .any(|existing| same_physical_chord(existing, binding))
}

/// Assign a physical chord; an unresolved conflict must leave both actions unchanged.
pub(crate) fn edit_binding_override(
    overrides: &mut BTreeMap<String, Vec<Binding>>,
    defaults: &BTreeMap<String, Vec<Binding>>,
    action: &str,
    binding: Binding,
    choice: Option<BindingConflictChoice>,
) -> BindingEditResult {
    if !defaults.contains_key(action) {
        return BindingEditResult::UnknownAction;
    }
    if is_reserved_browser_chord(&binding) {
        return BindingEditResult::Reserved;
    }
    let mut effective = defaults.clone();
    effective.extend(
        overrides
            .iter()
            .map(|(name, bindings)| (name.clone(), bindings.clone())),
    );
    let conflicts = defaults
        .keys()
        .filter(|other| {
            other.as_str() != action
                && effective
                    .get(*other)
                    .is_some_and(|bindings| contains_physical_chord(bindings, &binding))
        })
        .cloned()
        .collect::<Vec<_>>();
    if conflicts.is_empty() {
        overrides.insert(action.to_owned(), vec![binding]);
        return BindingEditResult::Applied;
    }
    match choice {
        None => BindingEditResult::Conflict { actions: conflicts },
        Some(BindingConflictChoice::Cancel) => BindingEditResult::Cancelled,
        Some(BindingConflictChoice::Swap) => {
            let previous_chord = effective
                .get(action)
                .and_then(|bindings| bindings.first())
                .cloned();
            // Read every affected owner before mutation, then commit the complete swap.
            let changes = conflicts
                .into_iter()
                .map(|other| {
                    let mut changed = Vec::new();
                    for existing in &effective[&other] {
                        let replacement = if same_physical_chord(existing, &binding) {
                            previous_chord.as_ref()
                        } else {
                            Some(existing)
                        };
                        if let Some(replacement) = replacement
                            && !contains_physical_chord(&changed, replacement)
                        {
                            changed.push(replacement.clone());
                        }
                    }
                    (other, changed)
                })
                .collect::<Vec<_>>();
            for (other, changed) in changes {
                overrides.insert(other, changed);
            }
            overrides.insert(action.to_owned(), vec![binding]);
            BindingEditResult::Applied
        }
    }
}

fn is_reserved_browser_chord(binding: &Binding) -> bool {
    let Binding::Key { code, mods } = binding else {
        return false;
    };
    turts_mod_api::RESERVED_BROWSER_KEYS
        .iter()
        .any(|(reserved_code, reserved_mods)| {
            *reserved_code == code
                && reserved_mods.len() == mods.len()
                && reserved_mods.iter().all(|modifier| mods.contains(modifier))
        })
}

/// Restore the authored default and atomically move conflicting player chords.
pub(crate) fn reset_binding_override(
    overrides: &mut BTreeMap<String, Vec<Binding>>,
    defaults: &BTreeMap<String, Vec<Binding>>,
    action: &str,
) -> bool {
    let Some(restored) = defaults.get(action) else {
        return false;
    };
    let previous = overrides
        .get(action)
        .or_else(|| defaults.get(action))
        .into_iter()
        .flatten()
        .filter(|binding| !contains_physical_chord(restored, binding))
        .cloned()
        .collect::<Vec<_>>();
    let mut staged = overrides.clone();
    for (owner, bindings) in overrides
        .iter()
        .filter(|(owner, _)| owner.as_str() != action)
    {
        let Some(authored) = defaults.get(owner) else {
            continue;
        };
        let safe = |candidate: &Binding| {
            defaults.iter().all(|(other, other_defaults)| {
                if other == owner {
                    return true;
                }
                let current = if other == action {
                    restored
                } else {
                    staged.get(other).unwrap_or(other_defaults)
                };
                !contains_physical_chord(current, candidate)
                    || authored
                        .iter()
                        .any(|binding| contains_physical_chord(other_defaults, binding))
            })
        };
        let mut changed = Vec::new();
        let mut displaced = false;
        for binding in bindings {
            if contains_physical_chord(restored, binding)
                && !contains_physical_chord(authored, binding)
            {
                displaced = true;
                let replacements =
                    if let Some(previous) = previous.iter().find(|binding| safe(binding)) {
                        vec![previous.clone()]
                    } else {
                        authored
                            .iter()
                            .filter(|binding| safe(binding))
                            .cloned()
                            .collect()
                    };
                for replacement in replacements {
                    if !contains_physical_chord(&changed, &replacement) {
                        changed.push(replacement);
                    }
                }
            } else if !contains_physical_chord(&changed, binding) {
                changed.push(binding.clone());
            }
        }
        if !displaced {
            continue;
        }
        if changed == *authored {
            staged.remove(owner);
        } else {
            staged.insert(owner.clone(), changed);
        }
    }
    staged.remove(action);
    if staged == *overrides {
        return false;
    }
    *overrides = staged;
    true
}

/// Settings that must reach the existing camera, Bevy UI and window owners.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClientSettings {
    pub scroll_speed: f32,
    pub edge_scroll: bool,
    pub ui_scale: f32,
    pub cursor_lock: bool,
    pub double_click_ms: u64,
    /// The frame limit when the window has the focus (`pacing.rs`; 0 is no limit).
    pub frame_limit: u32,
    /// The frame limit when the window does not have the focus (`pacing.rs`).
    pub background_frame_limit: u32,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            scroll_speed: 1.0,
            edge_scroll: true,
            ui_scale: 1.0,
            cursor_lock: false,
            double_click_ms: 400,
            frame_limit: crate::pacing::DEFAULT_FRAME_LIMIT,
            background_frame_limit: crate::pacing::DEFAULT_BACKGROUND_FRAME_LIMIT,
        }
    }
}

/// Apply persisted player settings to the existing camera, UI and window runtime resources.
/// The window update modes exist only with a window (`WinitSettings`, `pacing.rs`).
pub(crate) fn apply_client_settings(
    settings: Res<ClientSettings>,
    mut camera: Option<ResMut<crate::camera::CameraConfiguration>>,
    mut ui_scale: Option<ResMut<bevy::ui::UiScale>>,
    mut window_modes: Option<ResMut<bevy::winit::WinitSettings>>,
) {
    if !settings.is_changed() {
        return;
    }
    if let Some(camera) = camera.as_deref_mut() {
        camera.options.scroll_speed = settings.scroll_speed;
        camera.options.edge_scroll = settings.edge_scroll;
    }
    if let Some(ui_scale) = ui_scale.as_deref_mut() {
        ui_scale.0 = settings.ui_scale;
    }
    if let Some(window_modes) = window_modes.as_deref_mut() {
        *window_modes =
            crate::pacing::winit_settings(settings.frame_limit, settings.background_frame_limit);
    }
}

/// Engine-owned text row used only when a joined package has no authored interface.
#[derive(Component)]
pub(crate) struct ClientNotice;

/// Own the one settings overlay reached through the interaction core's existing `menu` action.
pub struct ClientShellPlugin;

impl Plugin for ClientShellPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ButtonPlugin>() {
            app.add_plugins(ButtonPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ScrollAreaPlugin>() {
            app.add_plugins(bevy::ui_widgets::ScrollAreaPlugin);
        }
        app.add_plugins(crate::client_screens::ClientScreensPlugin);
        app.init_resource::<ClientSettings>()
            .init_resource::<CorePreferences>()
            .init_resource::<BindingEditorState>()
            .init_resource::<BindingInputTimeline>()
            .init_resource::<BindingLabels>()
            .init_resource::<crate::visuals::HealthDisplay>()
            .add_message::<WindowEvent>()
            .add_systems(
                First,
                record_binding_input_timeline
                    .after(bevy::ecs::message::MessageUpdateSystems)
                    .before(bevy::picking::PickingSystems::Input),
            )
            .add_systems(
                Update,
                (apply_client_setting_sliders, apply_client_settings)
                    .chain()
                    .after(crate::audio::SettingsLoadSet)
                    .before(CoreAdapterSet),
            )
            .add_systems(Update, sync_settings_toggle_labels)
            .add_systems(
                Update,
                sync_interaction_settings
                    .after(crate::audio::SettingsLoadSet)
                    .after(apply_client_setting_sliders)
                    .before(CoreAdapterSet),
            )
            .add_systems(
                Update,
                update_pointer_lock
                    .after(crate::audio::SettingsLoadSet)
                    .after(apply_client_setting_sliders)
                    .before(CoreAdapterSet),
            )
            .add_observer(on_binding_pointer_press)
            .add_observer(on_binding_pointer_click)
            .add_observer(on_binding_keyboard_activation)
            .add_observer(on_binding_button_activated);
        app.add_observer(on_settings_toggle_activated);
        app.add_systems(
            Update,
            toggle_settings_menu
                .after(CoreAdapterSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            (capture_physical_binding, sync_binding_feedback)
                .chain()
                .after(toggle_settings_menu)
                .before(UiPresentationSet),
        )
        .add_systems(
            Update,
            sync_binding_value_labels
                .after(toggle_settings_menu)
                .after(capture_physical_binding),
        );
        app.add_systems(
            Update,
            sync_client_notice
                .after(FeedbackSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            sync_focus_overlay
                .after(sync_client_notice)
                .after(CoreAdapterSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            sync_rejoin_camera
                .after(sync_focus_overlay)
                .before(UiPresentationSet),
        );
        app.init_resource::<SoundLockTimer>().add_systems(
            Update,
            (
                sync_sound_off_notice.after(sync_focus_overlay),
                sync_sound_status_label.after(toggle_settings_menu),
            )
                .before(UiPresentationSet),
        );
    }
}

fn sync_focus_overlay(
    focus: Option<Res<crate::debug_feed::FocusFeed>>,
    audio: Option<Res<crate::debug_feed::AudioFeed>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    overlays: Query<Entity, With<FocusOverlay>>,
    mut commands: Commands,
) {
    let Some(focus) = focus else {
        return;
    };
    let Some(window_focused) = windows.iter().next().map(|window| window.focused) else {
        return;
    };
    // Ruling O15: the overlay waits for the first gesture, not for running audio. A browser can
    // keep its context suspended after the gesture; the sound notice covers that case.
    let awaiting_gesture = audio.as_deref().is_some_and(|feed| {
        crate::audio::BrowserAudio::from_feed(feed) == crate::audio::BrowserAudio::AwaitingGesture
    });
    if focus.0 && window_focused && !awaiting_gesture {
        for entity in &overlays {
            commands.entity(entity).despawn();
        }
        return;
    }
    if !overlays.is_empty() {
        return;
    }
    commands
        .spawn((
            FocusOverlay,
            UiCaptureRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ZIndex(i32::MAX),
            bevy::ui::FocusPolicy::Block,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Click to play"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// The shell text that shows when audio stays locked after the first gesture.
pub(crate) const SOUND_OFF_NOTICE: &str = "Sound is off. Click or press a key to turn it on.";
/// The resume request of a gesture needs some frames. The notice waits this long first.
const SOUND_NOTICE_DELAY_SECS: f32 = 0.5;
const SOUND_NOTICE_SECS: f32 = 8.0;

/// The shell row of `SOUND_OFF_NOTICE`. It does not capture input.
#[derive(Component)]
pub(crate) struct SoundOffNotice;

/// The time that the current locked period started, from `Time<Real>`.
#[derive(Resource, Default)]
struct SoundLockTimer(Option<std::time::Duration>);

/// Show `SOUND_OFF_NOTICE` for 8 s when audio stays locked after the first gesture. Each later
/// gesture asks the browser to resume audio through the page hook (index.html).
fn sync_sound_off_notice(
    audio: Option<Res<crate::debug_feed::AudioFeed>>,
    time: Res<Time<Real>>,
    mut timer: ResMut<SoundLockTimer>,
    notices: Query<Entity, With<SoundOffNotice>>,
    mut commands: Commands,
) {
    let locked = audio.as_deref().is_some_and(|feed| {
        crate::audio::BrowserAudio::from_feed(feed) == crate::audio::BrowserAudio::Locked
    });
    let now = time.elapsed();
    let visible = if locked {
        let since = *timer.0.get_or_insert(now);
        let age = now.saturating_sub(since).as_secs_f32();
        (SOUND_NOTICE_DELAY_SECS..SOUND_NOTICE_DELAY_SECS + SOUND_NOTICE_SECS).contains(&age)
    } else {
        timer.0 = None;
        false
    };
    if !visible {
        for entity in &notices {
            commands.entity(entity).despawn();
        }
        return;
    }
    if !notices.is_empty() {
        return;
    }
    commands.spawn((
        SoundOffNotice,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Percent(50.0),
            width: Val::Px(440.0),
            margin: UiRect::left(Val::Px(-220.0)),
            padding: UiRect::all(Val::Px(8.0)),
            justify_content: JustifyContent::Center,
            ..default()
        },
        ZIndex(i32::MAX - 2),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
        bevy::picking::Pickable::IGNORE,
        Text::new(SOUND_OFF_NOTICE),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
    ));
}

/// Keep the menu sound label equal to the browser audio state.
fn sync_sound_status_label(
    audio: Option<Res<crate::debug_feed::AudioFeed>>,
    mut labels: Query<&mut Text, With<crate::volume_settings::SoundStatusText>>,
) {
    let label = sound_status_label(audio.as_deref());
    for mut text in &mut labels {
        if text.0 != label {
            text.0 = label.to_owned();
        }
    }
}

fn sound_status_label(audio: Option<&crate::debug_feed::AudioFeed>) -> &'static str {
    audio
        .map_or(
            crate::audio::BrowserAudio::NotObserved,
            crate::audio::BrowserAudio::from_feed,
        )
        .menu_label()
}

pub(crate) fn sync_client_notice(
    state: Option<Res<ReplicaState>>,
    phase: Option<Res<Phase>>,
    acquired: Option<Res<AcquiredPackages>>,
    notices: Option<Res<NoticesFeed>>,
    rejoin: Option<Res<RejoinController>>,
    mut rows: Query<(Entity, &mut Text), With<ClientNotice>>,
    mut commands: Commands,
) {
    let rejoin_text = rejoin
        .as_deref()
        .and_then(RejoinController::visible_text)
        .filter(|text| {
            // The rejection screen owns this failure and shows its player-word copy.
            !(crate::client_screens::player_rejection_owns_reason(text)
                && phase
                    .as_deref()
                    .is_some_and(|phase| matches!(phase, Phase::Failed(_))))
        });
    let active = rejoin_text.is_some()
        || (phase
            .as_deref()
            .is_some_and(|phase| matches!(phase, Phase::Joined { .. }))
            && state
                .as_deref()
                .is_some_and(|state| state.welcome.is_some())
            && acquired
                .as_deref()
                .is_some_and(|packages| packages.0.presentation.interface.is_none()));
    if !active {
        for (entity, _) in &mut rows {
            commands.entity(entity).despawn();
        }
        return;
    }
    let fallback_text = notices
        .as_deref()
        .and_then(|notices| notices.0.last())
        .and_then(|notice| notice.get("text").or_else(|| notice.get("message")))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let text = rejoin_text.as_deref().unwrap_or(fallback_text);
    if text.is_empty() {
        for (entity, _) in &mut rows {
            commands.entity(entity).despawn();
        }
        return;
    }
    if let Some((_, mut existing)) = rows.iter_mut().next() {
        if existing.0.as_str() != text {
            *existing = Text::new(text);
        }
        return;
    }
    commands.spawn((
        ClientNotice,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            width: Val::Px(420.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

pub(crate) fn sync_rejoin_camera(
    rejoin: Option<Res<RejoinController>>,
    mut cameras: Query<(Entity, &mut Camera), With<RejoinOverlayCamera>>,
    mut commands: Commands,
) {
    let should_show = rejoin
        .as_deref()
        .is_some_and(|rejoin| rejoin.visible_text().is_some());
    if should_show {
        if cameras.is_empty() {
            commands.spawn((Camera2d, RejoinOverlayCamera));
        }
        return;
    }
    for (entity, mut camera) in &mut cameras {
        camera.is_active = false;
        commands.entity(entity).despawn();
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct SettingsMenuSources<'w> {
    effects: Res<'w, CoreEffects>,
    volumes: Res<'w, VolumeSettings>,
    settings: Res<'w, ClientSettings>,
    health: Res<'w, crate::visuals::HealthDisplay>,
    acquired: Option<Res<'w, AcquiredPackages>>,
    preferences: Res<'w, CorePreferences>,
    labels: Res<'w, BindingLabels>,
    audio: Option<Res<'w, crate::debug_feed::AudioFeed>>,
}

fn toggle_settings_menu(
    sources: SettingsMenuSources,
    mut editor: ResMut<BindingEditorState>,
    overlays: Query<Entity, With<SettingsMenuOverlay>>,
    mut cursors: Query<&mut bevy::window::CursorOptions, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let SettingsMenuSources {
        effects,
        volumes,
        settings,
        health,
        acquired,
        preferences,
        labels,
        audio,
    } = sources;
    if !effects.is_changed()
        || !effects.0.iter().any(
            |effect| matches!(effect, turts_interaction::Effect::ShellAction { action } if action == "menu"),
        )
    {
        return;
    }

    release_pointer_lock(&mut cursors);

    if let Some(overlay) = overlays.iter().next() {
        commands.entity(overlay).despawn();
        editor.capturing = None;
        editor.capture_after_message = None;
        editor.conflict = None;
        editor.status = None;
        return;
    }

    let input_map = acquired
        .as_deref()
        .map(|acquired| &acquired.0.content.input_map);
    let effective = input_map.map_or_else(BTreeMap::new, |input| {
        let mut actions = turts_interaction::input_map::effective_actions(input);
        for (action, bindings) in &preferences.runtime_bindings {
            actions.insert(action.clone(), bindings.clone());
        }
        actions
    });
    let action_ids = input_map.map_or_else(Vec::new, |input| {
        crate::widgets::sources::controls_order(input, effective.keys())
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    });
    let action_label = |action: &str| {
        acquired.as_deref().map_or_else(
            || crate::widgets::sources::humanize_action(action),
            |acquired| crate::widgets::sources::action_label(&acquired.0.content, action),
        )
    };

    let overlay = commands
        .spawn((
            SettingsMenuOverlay,
            UiCaptureRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ZIndex(i32::MAX - 1),
            bevy::ui::FocusPolicy::Block,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
        ))
        .id();

    // The frame holds the title and the binding feedback, which do not scroll, and the
    // scrolling list of the settings. So the rebind prompt, the conflict text and Swap/Cancel
    // stay in view for a Change on any row (RUN133 H1).
    let frame = commands
        .spawn((
            Node {
                width: Val::Px(520.0),
                height: Val::Px(680.0),
                max_height: Val::Percent(95.0),
                padding: UiRect::all(Val::Px(22.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                overflow: bevy::ui::Overflow::clip(),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.075, 0.09, 0.12)),
            BorderColor::all(Color::srgb(0.34, 0.43, 0.52)),
        ))
        .id();
    commands.entity(overlay).add_child(frame);
    commands.entity(frame).with_children(|frame| {
        frame.spawn((
            Text::new("Settings"),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        if !action_ids.is_empty() {
            frame
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(BINDING_FEEDBACK_HEIGHT),
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    row_gap: Val::Px(8.0),
                    overflow: bevy::ui::Overflow::clip(),
                    ..default()
                })
                .with_children(|feedback| {
                    feedback.spawn((
                        BindingFeedbackText,
                        Text::new(""),
                        Node {
                            width: Val::Percent(100.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.78, 0.36)),
                    ));
                });
        }
    });
    let panel = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                overflow: bevy::ui::Overflow {
                    x: bevy::ui::OverflowAxis::Clip,
                    y: bevy::ui::OverflowAxis::Scroll,
                },
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    commands.entity(frame).add_child(panel);
    commands.entity(panel).with_children(|panel| {
        if !action_ids.is_empty() {
            panel.spawn((
                Text::new("Key bindings"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            for action in &action_ids {
                let binding = effective
                    .get(action)
                    .map(|bindings| menu_binding_list_label(bindings, &labels))
                    .unwrap_or_else(|| "Unbound".into());
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(30.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(action_label(action)),
                            Node {
                                width: Val::Px(MENU_ACTION_LABEL_WIDTH),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                        ));
                        row.spawn((
                            BindingValueText {
                                action: action.clone(),
                            },
                            Text::new(binding),
                            Node {
                                width: Val::Px(MENU_BINDING_BASIS_WIDTH),
                                flex_grow: 1.0,
                                ..default()
                            },
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                        ));
                        spawn_binding_button(row, "Change", action, BindingButtonKind::Change);
                        spawn_binding_button(row, "Reset", action, BindingButtonKind::Reset);
                    });
            }
        }
        panel.spawn((
            Text::new("Gameplay settings"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        spawn_settings_slider(
            panel,
            "Scroll speed",
            SettingsSlider::ScrollSpeed,
            settings.scroll_speed,
            0.25,
            3.0,
            0.05,
        );
        spawn_settings_toggle(
            panel,
            "Edge scroll",
            on_off(settings.edge_scroll),
            SettingsToggle::EdgeScroll,
        );
        spawn_settings_slider(
            panel,
            "UI scale",
            SettingsSlider::UiScale,
            settings.ui_scale,
            0.75,
            2.0,
            0.05,
        );
        spawn_settings_toggle(
            panel,
            "Health bars",
            match health.0 {
                crate::visuals::HealthMode::Selected => "Selected",
                crate::visuals::HealthMode::DamagedOrSelected => "Damaged or selected",
                crate::visuals::HealthMode::Always => "Always",
            },
            SettingsToggle::HealthBars,
        );
        spawn_settings_toggle(
            panel,
            "Keep the cursor in the game",
            on_off(settings.cursor_lock),
            SettingsToggle::CursorLock,
        );
        spawn_settings_slider(
            panel,
            "Double-click time",
            SettingsSlider::DoubleClick,
            settings.double_click_ms as f32,
            100.0,
            1000.0,
            50.0,
        );
        spawn_settings_slider(
            panel,
            "Double-click range",
            SettingsSlider::DoubleClickPixels,
            preferences.interaction.double_click_pixels,
            0.0,
            32.0,
            1.0,
        );
    });
    spawn_volume_controls(
        &mut commands,
        panel,
        *volumes,
        sound_status_label(audio.as_deref()),
    );
}

fn spawn_settings_toggle(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    initial_state: &str,
    toggle: SettingsToggle,
) {
    parent
        .spawn((
            UiButton,
            toggle,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(30.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.22, 0.28)),
        ))
        .with_children(|button| {
            button.spawn((
                SettingsToggleText,
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            button.spawn((
                SettingsToggleState(toggle),
                Text::new(initial_state),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_settings_slider(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    setting: SettingsSlider,
    value: f32,
    minimum: f32,
    maximum: f32,
    step: f32,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(34.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                Node {
                    width: Val::Px(170.0),
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            row.spawn((
                Node {
                    width: Val::Px(0.0),
                    min_width: Val::Px(0.0),
                    flex_grow: 1.0,
                    height: Val::Px(12.0),
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.20, 0.24, 0.30)),
                Slider {
                    track_click: bevy::ui_widgets::TrackClick::Snap,
                    ..default()
                },
                SliderRange::new(minimum, maximum),
                SliderStep(step),
                SliderValue(value),
                setting,
            ))
            .with_children(|track| {
                track.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(0.0),
                        top: Val::Px(-4.0),
                        width: Val::Px(20.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.40, 0.75, 0.95)),
                    SliderThumb,
                ));
            });
        });
}

fn on_settings_toggle_activated(
    trigger: On<Activate>,
    toggles: Query<&SettingsToggle>,
    mut settings: ResMut<ClientSettings>,
    mut health: ResMut<crate::visuals::HealthDisplay>,
    mut cursors: Query<&mut bevy::window::CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(toggle) = toggles.get(trigger.entity) else {
        return;
    };
    match toggle {
        SettingsToggle::EdgeScroll => settings.edge_scroll = !settings.edge_scroll,
        SettingsToggle::CursorLock => {
            settings.cursor_lock = !settings.cursor_lock;
            if !settings.cursor_lock {
                release_pointer_lock(&mut cursors);
            }
        }
        SettingsToggle::HealthBars => {
            health.0 = match health.0 {
                crate::visuals::HealthMode::Selected => {
                    crate::visuals::HealthMode::DamagedOrSelected
                }
                crate::visuals::HealthMode::DamagedOrSelected => crate::visuals::HealthMode::Always,
                crate::visuals::HealthMode::Always => crate::visuals::HealthMode::Selected,
            };
        }
    }
}

fn release_pointer_lock(
    cursors: &mut Query<&mut bevy::window::CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(mut cursor) = cursors.single_mut() {
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn update_pointer_lock(
    settings: Res<ClientSettings>,
    mut events: MessageReader<WindowEvent>,
    windows: Query<Entity, With<PrimaryWindow>>,
    menus: Query<(), With<SettingsMenuOverlay>>,
    focus_overlays: Query<(), With<FocusOverlay>>,
    mut cursors: Query<&mut bevy::window::CursorOptions, With<PrimaryWindow>>,
) {
    let Some(primary) = windows.iter().next() else {
        events.read().for_each(drop);
        return;
    };
    for event in events.read() {
        match event {
            WindowEvent::WindowFocused(value) if value.window == primary && !value.focused => {
                release_pointer_lock(&mut cursors);
            }
            WindowEvent::KeyboardFocusLost(_) => release_pointer_lock(&mut cursors),
            WindowEvent::MouseButtonInput(value)
                if value.window == primary
                    && value.button == bevy::input::mouse::MouseButton::Left
                    && value.state == bevy::input::ButtonState::Pressed
                    && settings.cursor_lock
                    && menus.is_empty()
                    && focus_overlays.is_empty() =>
            {
                if let Ok(mut cursor) = cursors.single_mut() {
                    cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
                    cursor.visible = false;
                }
            }
            _ => {}
        }
    }
}

fn sync_settings_toggle_labels(
    settings: Res<ClientSettings>,
    health: Option<Res<crate::visuals::HealthDisplay>>,
    mut states: Query<(&SettingsToggleState, &mut Text)>,
) {
    for (state, mut text) in &mut states {
        let value = match state.0 {
            SettingsToggle::EdgeScroll => on_off(settings.edge_scroll).to_string(),
            SettingsToggle::CursorLock => on_off(settings.cursor_lock).to_string(),
            SettingsToggle::HealthBars => health
                .as_deref()
                .map_or("Damaged or selected", |health| match health.0 {
                    crate::visuals::HealthMode::Selected => "Selected",
                    crate::visuals::HealthMode::DamagedOrSelected => "Damaged or selected",
                    crate::visuals::HealthMode::Always => "Always",
                })
                .to_string(),
        };
        if text.0.as_str() != value {
            *text = Text::new(value);
        }
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "On" } else { "Off" }
}

fn apply_client_setting_sliders(
    mut settings: ResMut<ClientSettings>,
    mut preferences: ResMut<CorePreferences>,
    changed: Query<(&SettingsSlider, &SliderValue), Changed<SliderValue>>,
) {
    for (control, value) in &changed {
        match control {
            SettingsSlider::ScrollSpeed => settings.scroll_speed = value.0,
            SettingsSlider::UiScale => settings.ui_scale = value.0,
            SettingsSlider::DoubleClick => settings.double_click_ms = value.0.round() as u64,
            SettingsSlider::DoubleClickPixels => {
                preferences.interaction.double_click_pixels = value.0
            }
        }
    }
}

fn sync_interaction_settings(
    settings: Res<ClientSettings>,
    mut preferences: ResMut<CorePreferences>,
) {
    if settings.is_changed() {
        preferences.interaction.double_click_ms = settings.double_click_ms;
    }
}

fn spawn_binding_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: &str,
    kind: BindingButtonKind,
) {
    parent
        .spawn((
            UiButton,
            BindingButton {
                action: action.to_owned(),
                kind,
            },
            Node {
                min_width: Val::Px(56.0),
                height: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.22, 0.28)),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn on_binding_pointer_press(
    trigger: On<bevy::picking::events::PointerPress>,
    buttons: Query<&BindingButton>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut timeline: ResMut<BindingInputTimeline>,
) {
    if buttons.get(trigger.entity).is_err() {
        return;
    }
    let primary = windows.iter().next();
    let expected_button = match trigger.button {
        bevy::picking::pointer::PointerButton::Primary => bevy::input::mouse::MouseButton::Left,
        bevy::picking::pointer::PointerButton::Secondary => bevy::input::mouse::MouseButton::Right,
        bevy::picking::pointer::PointerButton::Middle => bevy::input::mouse::MouseButton::Middle,
    };
    let boundary = timeline.records.iter().find_map(|record| {
        let WindowEvent::MouseButtonInput(input) = &record.event else {
            return None;
        };
        if timeline.claimed_presses.contains(&record.message_id)
            || Some(input.window) != primary
            || input.button != expected_button
            || input.state != bevy::input::ButtonState::Pressed
            || record.cursor_position != trigger.pointer.position
        {
            return None;
        }
        let target =
            bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(input.window))
                .normalize(primary)?;
        (target == trigger.pointer.target).then_some(record.message_id)
    });
    timeline
        .pointer_press_causes
        .retain(|(entity, pointer, button, _)| {
            *entity != trigger.entity || *pointer != trigger.pointer.id || *button != trigger.button
        });
    if let Some(boundary) = boundary {
        timeline.claimed_presses.insert(boundary);
        timeline.pointer_press_causes.push((
            trigger.entity,
            trigger.pointer.id,
            trigger.button,
            boundary,
        ));
    }
}

fn on_binding_pointer_click(
    trigger: On<bevy::picking::events::PointerClick>,
    buttons: Query<&BindingButton>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut timeline: ResMut<BindingInputTimeline>,
) {
    if buttons.get(trigger.entity).is_err() {
        return;
    }
    let primary = windows.iter().next();
    let activation_boundary = timeline
        .pointer_press_causes
        .iter()
        .find(|(entity, pointer, button, _)| {
            *entity == trigger.entity && *pointer == trigger.pointer.id && *button == trigger.button
        })
        .map(|cause| cause.3);
    let boundary = timeline.records.iter().find_map(|record| {
        let release = record.release?;
        if activation_boundary.is_none_or(|boundary| release.message_id <= boundary)
            || timeline.claimed_releases.contains(&release.message_id)
            || release.button != trigger.button
            || Some(release.window) != primary
            || record.cursor_position != trigger.pointer.position
        {
            return None;
        }
        let target =
            bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(release.window))
                .normalize(primary)?;
        (target == trigger.pointer.target).then_some(release.message_id)
    });
    timeline
        .pointer_press_causes
        .retain(|(entity, pointer, button, _)| {
            *entity != trigger.entity || *pointer != trigger.pointer.id || *button != trigger.button
        });
    if let Some(boundary) = boundary {
        timeline.claimed_releases.insert(boundary);
    }
    timeline.pending_activations.push(PendingBindingActivation {
        entity: trigger.entity,
        boundary,
    });
}

fn on_binding_keyboard_activation(
    trigger: On<FocusedInput<bevy::input::keyboard::KeyboardInput>>,
    buttons: Query<&BindingButton>,
    mut timeline: ResMut<BindingInputTimeline>,
) {
    if buttons.get(trigger.focused_entity).is_err()
        || trigger.input.repeat
        || trigger.input.state != bevy::input::ButtonState::Pressed
        || !matches!(trigger.input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        return;
    }
    let boundary = timeline.records.iter().find_map(|record| {
        let (window, key, state) = record.keyboard?;
        let WindowEvent::KeyboardInput(input) = &record.event else {
            return None;
        };
        if input.repeat {
            return None;
        }
        if timeline.claimed_keys.contains(&record.message_id)
            || window != trigger.input.window
            || key != trigger.input.key_code
            || state != trigger.input.state
        {
            return None;
        }
        Some(record.message_id)
    });
    if let Some(boundary) = boundary {
        timeline.claimed_keys.insert(boundary);
    }
    timeline.pending_activations.push(PendingBindingActivation {
        entity: trigger.focused_entity,
        boundary,
    });
}

fn on_binding_button_activated(
    trigger: On<Activate>,
    buttons: Query<&BindingButton>,
    acquired: Option<Res<AcquiredPackages>>,
    preferences: ResMut<CorePreferences>,
    editor: ResMut<BindingEditorState>,
    mut timeline: ResMut<BindingInputTimeline>,
) {
    let Some(activation_index) = timeline
        .pending_activations
        .iter()
        .position(|activation| activation.entity == trigger.entity)
    else {
        // Programmatic activation has no physical cause. Its safe boundary is the end of
        // the already-buffered timeline; keyboard and pointer activation use their cause id.
        return on_binding_button_activated_at_boundary(
            trigger.entity,
            buttons,
            acquired,
            preferences,
            editor,
            timeline,
            None,
        );
    };
    let activation = timeline.pending_activations.remove(activation_index);
    on_binding_button_activated_at_boundary(
        trigger.entity,
        buttons,
        acquired,
        preferences,
        editor,
        timeline,
        Some(activation),
    );
}

fn on_binding_button_activated_at_boundary(
    entity: Entity,
    buttons: Query<&BindingButton>,
    acquired: Option<Res<AcquiredPackages>>,
    mut preferences: ResMut<CorePreferences>,
    mut editor: ResMut<BindingEditorState>,
    timeline: ResMut<BindingInputTimeline>,
    activation: Option<PendingBindingActivation>,
) {
    let Ok(button) = buttons.get(entity) else {
        return;
    };
    if editor.capturing.is_some()
        && activation.is_some_and(|activation| {
            activation.boundary.is_some_and(|boundary| {
                editor
                    .capture_after_message
                    .is_none_or(|old| boundary > old)
            })
        })
    {
        // The click is eligible capture input. Do not let its deferred generic Activate
        // restart Change or apply Reset/Swap/Cancel before Update consumes the press.
        return;
    }
    let Some(acquired) = acquired else { return };
    let defaults = turts_interaction::input_map::effective_actions(&acquired.0.content.input_map);
    match button.kind {
        BindingButtonKind::Change => {
            editor.capture_after_message = activation
                .and_then(|activation| activation.boundary)
                .or_else(|| timeline.last_message_id);
            editor.capturing = Some(button.action.clone());
            editor.conflict = None;
            editor.status = Some(format!(
                "Press a key or mouse button for {}",
                crate::widgets::sources::action_label(&acquired.0.content, &button.action)
            ));
        }
        BindingButtonKind::Reset => {
            if reset_binding_override(&mut preferences.runtime_bindings, &defaults, &button.action)
            {
                editor.status = Some(format!(
                    "{} restored to the package default",
                    crate::widgets::sources::action_label(&acquired.0.content, &button.action)
                ));
            } else if defaults.contains_key(&button.action) {
                editor.status = Some(format!(
                    "{} already uses the package default",
                    crate::widgets::sources::action_label(&acquired.0.content, &button.action)
                ));
            } else {
                editor.status = Some("Action is unavailable".into());
            }
            editor.capturing = None;
            editor.capture_after_message = None;
            editor.conflict = None;
        }
        BindingButtonKind::Swap => {
            if editor
                .conflict
                .as_ref()
                .is_some_and(|(action, _, _)| *action == button.action)
                && let Some((action, binding, _)) = editor.conflict.take()
            {
                let result = edit_binding_override(
                    &mut preferences.runtime_bindings,
                    &defaults,
                    &action,
                    binding,
                    Some(BindingConflictChoice::Swap),
                );
                editor.status = Some(match result {
                    BindingEditResult::Applied => "Bindings swapped".into(),
                    BindingEditResult::Reserved => "That key is reserved by the browser".into(),
                    _ => "The binding could not be changed".into(),
                });
            }
            editor.capturing = None;
            editor.capture_after_message = None;
        }
        BindingButtonKind::Cancel => {
            if let Some((action, binding, _)) = editor.conflict.take() {
                let result = edit_binding_override(
                    &mut preferences.runtime_bindings,
                    &defaults,
                    &action,
                    binding,
                    Some(BindingConflictChoice::Cancel),
                );
                debug_assert_eq!(result, BindingEditResult::Cancelled);
            }
            editor.conflict = None;
            editor.capturing = None;
            editor.capture_after_message = None;
            editor.status = Some("Binding change cancelled".into());
        }
    }
}

fn capture_physical_binding(
    timeline: Res<BindingInputTimeline>,
    windows: Query<Entity, With<PrimaryWindow>>,
    labels: Res<BindingLabels>,
    acquired: Option<Res<AcquiredPackages>>,
    mut preferences: ResMut<CorePreferences>,
    mut editor: ResMut<BindingEditorState>,
) {
    let Some(action) = editor.capturing.clone() else {
        editor.capture_after_message = None;
        return;
    };
    let Some(acquired) = acquired else {
        editor.capture_after_message = timeline.last_message_id;
        return;
    };
    let primary = windows.iter().next();
    let Some(binding) = timeline.records.iter().find_map(|record| {
        (editor
            .capture_after_message
            .is_none_or(|boundary| record.message_id > boundary))
        .then(|| {
            if let WindowEvent::KeyboardInput(input) = &record.event
                && input.state == bevy::input::ButtonState::Released
                && Some(input.window) == primary
                && is_capture_modifier(input.key_code)
                && record
                    .released_modifier_press
                    .is_some_and(|activation_boundary| {
                        editor
                            .capture_after_message
                            .is_none_or(|boundary| activation_boundary > boundary)
                    })
            {
                Some(Binding::Key {
                    code: format!("{:?}", input.key_code),
                    mods: record.modifier_snapshot.as_mods(),
                })
            } else {
                captured_binding_for_snapshot(&record.event, primary, record.modifier_snapshot)
            }
        })
        .flatten()
    }) else {
        return;
    };
    if matches!(&binding, Binding::Key { code, .. } if code == "Escape") {
        editor.capturing = None;
        editor.capture_after_message = None;
        editor.conflict = None;
        editor.status = Some("Binding change cancelled".into());
        return;
    }
    let defaults = turts_interaction::input_map::effective_actions(&acquired.0.content.input_map);
    match edit_binding_override(
        &mut preferences.runtime_bindings,
        &defaults,
        &action,
        binding.clone(),
        None,
    ) {
        BindingEditResult::Applied => {
            editor.status = Some(format!(
                "{} changed to {}",
                crate::widgets::sources::action_label(&acquired.0.content, &action),
                binding_label(&binding, Some(&labels))
            ));
            editor.capturing = None;
        }
        BindingEditResult::Conflict {
            actions: conflicting,
        } => {
            editor.status = Some(format!(
                "{} is also bound to {}",
                binding_label(&binding, Some(&labels)),
                conflicting
                    .iter()
                    .map(|action| {
                        crate::widgets::sources::action_label(&acquired.0.content, action)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            editor.conflict = Some((action.clone(), binding, conflicting));
            editor.capturing = None;
        }
        BindingEditResult::Reserved => {
            editor.status = Some(format!(
                "{} is reserved by the browser",
                binding_label(&binding, Some(&labels))
            ));
            editor.capturing = None;
        }
        BindingEditResult::UnknownAction => {
            editor.status = Some("Action is unavailable".into());
            editor.capturing = None;
        }
        BindingEditResult::Cancelled => {
            editor.status = Some("Binding change cancelled".into());
            editor.capturing = None;
        }
    }
    editor.capture_after_message = None;
}

fn menu_binding_list_label(bindings: &[Binding], labels: &BindingLabels) -> String {
    if bindings.is_empty() {
        "Unbound".into()
    } else {
        bindings
            .iter()
            .map(|binding| binding_label(binding, Some(labels)))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn sync_binding_value_labels(
    labels: Res<BindingLabels>,
    preferences: Res<CorePreferences>,
    acquired: Option<Res<AcquiredPackages>>,
    mut values: Query<(&BindingValueText, &mut Text)>,
) {
    if !labels.is_changed() && !preferences.is_changed() {
        return;
    }
    let Some(acquired) = acquired else { return };
    let defaults = turts_interaction::input_map::effective_actions(&acquired.0.content.input_map);
    for (value, mut text) in &mut values {
        let bindings = preferences
            .runtime_bindings
            .get(&value.action)
            .or_else(|| defaults.get(&value.action));
        text.0 = bindings
            .map(|bindings| menu_binding_list_label(bindings, &labels))
            .unwrap_or_else(|| "Unbound".into());
        if text.0.is_empty() {
            text.0 = "Unbound".into();
        }
    }
}

fn sync_binding_feedback(
    editor: Res<BindingEditorState>,
    mut feedback: Query<(&mut Text, &ChildOf), With<BindingFeedbackText>>,
    prompts: Query<Entity, With<BindingConflictPrompt>>,
    mut commands: Commands,
) {
    let Ok((mut text, parent)) = feedback.single_mut() else {
        return;
    };
    let value = editor.status.as_deref().unwrap_or("");
    if text.0.as_str() != value {
        *text = Text::new(value);
    }
    if editor.conflict.is_some() && prompts.is_empty() {
        let conflicting_action = editor
            .conflict
            .as_ref()
            .map(|(action, _, _)| action.clone())
            .expect("the swap prompt is created only for an actual conflict");
        commands.entity(parent.parent()).with_children(|parent| {
            parent
                .spawn((
                    BindingConflictPrompt,
                    Node {
                        height: Val::Px(24.0),
                        flex_shrink: 0.0,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    spawn_binding_button(row, "Swap", &conflicting_action, BindingButtonKind::Swap);
                    spawn_binding_button(row, "Cancel", "", BindingButtonKind::Cancel);
                });
        });
    } else if editor.conflict.is_none() {
        for prompt in &prompts {
            commands.entity(prompt).despawn();
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::volume_settings::{VolumeCategory, VolumeControl};
    use crate::{
        acquire::{Acquired, PresentationContent},
        content::ClientContent,
        debug_feed::FocusFeed,
        ingest::WelcomeInfo,
    };

    fn key(code: &str) -> Binding {
        Binding::Key {
            code: code.to_owned(),
            mods: vec![],
        }
    }

    fn forge_input_defaults() -> turts_mod_api::InputMapDef {
        ron::from_str(include_str!("../../../mods/forge/input/bindings.ron"))
            .expect("the actual Forge default binding fixture parses")
    }

    fn forge_defaults() -> BTreeMap<String, Vec<Binding>> {
        turts_interaction::input_map::effective_actions(&forge_input_defaults())
    }

    fn forge_acquired_packages() -> AcquiredPackages {
        let content = ClientContent {
            input_map: forge_input_defaults(),
            ..Default::default()
        };
        AcquiredPackages(Acquired {
            lockfile_hash: "verified-fixture-lock".into(),
            visuals: Default::default(),
            map_size: (128.0, 72.0),
            unit_names: Default::default(),
            content,
            presentation: PresentationContent::default(),
            verified_root_mod: None,
        })
    }

    fn menu_action() -> CoreEffects {
        CoreEffects(vec![turts_interaction::Effect::ShellAction {
            action: "menu".into(),
        }])
    }

    fn notice_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(ReplicaState {
                welcome: Some(WelcomeInfo {
                    slot: 0,
                    tick: 1,
                    tick_hz: 20,
                    match_id: 1,
                }),
                ..default()
            })
            .insert_resource(Phase::Joining)
            .insert_resource(AcquiredPackages(Acquired {
                lockfile_hash: "fixture".into(),
                visuals: Default::default(),
                map_size: (64.0, 48.0),
                unit_names: Default::default(),
                content: ClientContent::default(),
                presentation: PresentationContent::default(),
                verified_root_mod: None,
            }))
            .insert_resource(NoticesFeed(vec![
                serde_json::json!({"text":"This game defines no orders"}),
            ]))
            .add_systems(Update, sync_client_notice);
        app
    }

    #[test]
    fn no_orders_fallback_waits_for_join_and_yields_to_an_authored_interface() {
        let mut app = notice_app();
        app.update();
        let mut rows = app
            .world_mut()
            .query_filtered::<Entity, With<ClientNotice>>();
        assert_eq!(
            rows.iter(app.world()).count(),
            0,
            "Welcome before Phase::Joined does not show the fallback"
        );

        *app.world_mut().resource_mut::<Phase>() = Phase::Joined { slot: 0 };
        app.update();
        let mut rows = app
            .world_mut()
            .query_filtered::<&Text, With<ClientNotice>>();
        let rows = rows.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "This game defines no orders");

        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .interface = Some(
            ron::from_str(include_str!("../../../mods/muster/interface/hud.ron"))
                .expect("real authored interface fixture parses"),
        );
        app.update();
        let mut rows = app
            .world_mut()
            .query_filtered::<Entity, With<ClientNotice>>();
        assert_eq!(
            rows.iter(app.world()).count(),
            0,
            "authored interfaces own their notices without an extra engine panel"
        );
    }

    #[test]
    fn repeated_and_updated_engine_notices_keep_one_visible_client_notice_entity() {
        let mut app = notice_app();
        *app.world_mut().resource_mut::<Phase>() = Phase::Joined { slot: 0 };
        app.update();
        let mut rows = app
            .world_mut()
            .query_filtered::<(Entity, &Text), With<ClientNotice>>();
        let first = rows
            .iter(app.world())
            .next()
            .expect("joined fallback notice");
        let entity = first.0;
        assert_eq!(first.1.0, "This game defines no orders");

        app.world_mut().resource_mut::<NoticesFeed>().0.extend([
            serde_json::json!({"text":"A package setting is invalid"}),
            serde_json::json!({"text":"A package setting is invalid"}),
        ]);
        app.update();

        let mut rows = app
            .world_mut()
            .query_filtered::<(Entity, &Text), With<ClientNotice>>();
        let visible = rows.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(
            visible.len(),
            1,
            "notice updates must not stack duplicate rows"
        );
        assert_eq!(
            visible[0].0, entity,
            "notice updates reuse the existing row"
        );
        assert_eq!(visible[0].1.0, "A package setting is invalid");
    }

    #[test]
    fn bounded_rejoin_uses_a_visible_shell_camera_through_timeout_and_releases_it_on_welcome() {
        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt(44));
        assert!(rejoin.on_connected(44));
        assert!(rejoin.on_hello_sent(44));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Phase::Joining)
            .insert_resource(rejoin)
            .add_systems(Update, (sync_client_notice, sync_rejoin_camera).chain());

        app.update();
        let mut notices = app
            .world_mut()
            .query_filtered::<&Text, With<ClientNotice>>();
        let visible = notices.iter(app.world()).next().expect("rejoin notice");
        assert_eq!(visible.0, "Reconnecting to your game\n15 s");
        let mut cameras = app
            .world_mut()
            .query_filtered::<Entity, With<RejoinOverlayCamera>>();
        assert_eq!(
            cameras.iter(app.world()).count(),
            1,
            "the pre-join notice has its own actual UI camera"
        );

        app.world_mut()
            .resource_mut::<RejoinController>()
            .advance(15.0, false);
        *app.world_mut().resource_mut::<Phase>() =
            Phase::Failed(crate::rejoin::REJOIN_FINAL_TEXT.to_owned());
        app.update();
        let mut notices = app
            .world_mut()
            .query_filtered::<&Text, With<ClientNotice>>();
        assert_eq!(
            notices
                .iter(app.world())
                .next()
                .expect("terminal rejoin notice")
                .0,
            crate::rejoin::REJOIN_FINAL_TEXT,
            "the exact final refusal remains rendered after the 15-second deadline"
        );
        let mut cameras = app
            .world_mut()
            .query_filtered::<Entity, With<RejoinOverlayCamera>>();
        assert_eq!(
            cameras.iter(app.world()).count(),
            1,
            "timeout does not leave the failed page blank"
        );

        app.world_mut()
            .resource_mut::<RejoinController>()
            .advance(0.0, true);
        *app.world_mut().resource_mut::<Phase>() = Phase::Joined { slot: 0 };
        app.update();
        let mut notices = app
            .world_mut()
            .query_filtered::<Entity, With<ClientNotice>>();
        assert_eq!(
            notices.iter(app.world()).count(),
            0,
            "Welcome clears the temporary shell notice"
        );
        let mut cameras = app
            .world_mut()
            .query_filtered::<Entity, With<RejoinOverlayCamera>>();
        assert_eq!(
            cameras.iter(app.world()).count(),
            0,
            "Welcome releases the temporary shell camera"
        );
    }

    #[test]
    fn the_existing_menu_action_opens_one_capturing_overlay_with_four_volume_controls() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .add_plugins(ClientShellPlugin);

        app.update();

        let world = app.world_mut();
        let mut overlays = world.query_filtered::<Entity, With<SettingsMenuOverlay>>();
        let overlay = overlays.single(world).expect("one settings overlay opens");
        assert!(
            world.entity(overlay).contains::<UiCaptureRoot>(),
            "the full-screen overlay consumes world pointer input"
        );
        assert_eq!(
            world.entity(overlay).get::<bevy::ui::FocusPolicy>(),
            Some(&bevy::ui::FocusPolicy::Block),
            "the overlay blocks pointer hit-testing behind it"
        );
        let mut controls = world.query::<&VolumeControl>();
        let categories = controls
            .iter(world)
            .map(|control| control.0)
            .collect::<Vec<_>>();
        assert_eq!(categories.len(), 4);
        assert!(categories.contains(&VolumeCategory::Master));
        assert!(categories.contains(&VolumeCategory::Voice));
        assert!(categories.contains(&VolumeCategory::Alerts));
        assert!(categories.contains(&VolumeCategory::Effects));
    }

    #[test]
    fn late_layout_labels_update_the_open_menu_without_rebuilding_capture_or_conflict_state() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .add_plugins(ClientShellPlugin);
        app.update();
        app.world_mut().resource_mut::<CoreEffects>().0.clear();

        let overlay_before = {
            let world = app.world_mut();
            let mut overlays = world.query_filtered::<Entity, With<SettingsMenuOverlay>>();
            overlays
                .iter(world)
                .next()
                .expect("the settings menu is open")
        };
        let panel = {
            let world = app.world_mut();
            let mut panels = world.query_filtered::<Entity, With<bevy::ui_widgets::ScrollArea>>();
            panels
                .iter(world)
                .next()
                .expect("the open settings panel scrolls")
        };
        app.world_mut()
            .entity_mut(panel)
            .insert(bevy::ui::ScrollPosition(Vec2::new(0.0, 32.0)));
        let value_entity = {
            let world = app.world_mut();
            let mut values = world.query::<(Entity, &BindingValueText)>();
            values
                .iter(world)
                .find(|(_, value)| value.action == "key_move")
                .map(|(entity, _)| entity)
                .expect("the actual key_move value text is marked")
        };
        assert_eq!(app.world().get::<Text>(value_entity).unwrap().0, "M");
        {
            let mut editor = app.world_mut().resource_mut::<BindingEditorState>();
            editor.capturing = Some("key_move".into());
            editor.conflict = Some(("key_move".into(), key("KeyQ"), vec!["key_stop".into()]));
            editor.status = Some("Swap or cancel".into());
        }

        *app.world_mut().resource_mut::<BindingLabels>() =
            BindingLabels::from_layout_entries([("KeyM".into(), "q".into())]);
        app.update();

        assert_eq!(app.world().get::<Text>(value_entity).unwrap().0, "Q");
        let world = app.world_mut();
        let mut overlays = world.query_filtered::<Entity, With<SettingsMenuOverlay>>();
        assert_eq!(overlays.iter(world).next(), Some(overlay_before));
        assert_eq!(
            app.world()
                .get::<bevy::ui::ScrollPosition>(panel)
                .unwrap()
                .0,
            Vec2::new(0.0, 32.0)
        );
        let editor = app.world().resource::<BindingEditorState>();
        assert_eq!(editor.capturing.as_deref(), Some("key_move"));
        assert_eq!(
            editor.conflict.as_ref().map(|(_, _, other)| other.clone()),
            Some(vec!["key_stop".to_owned()])
        );
        assert_eq!(editor.status.as_deref(), Some("Swap or cancel"));
    }

    #[test]
    fn the_same_menu_action_closes_the_overlay_without_spawning_a_second_root() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .add_plugins(ClientShellPlugin);
        app.update();

        app.world_mut().resource_mut::<CoreEffects>().0.clear();
        app.update();
        *app.world_mut().resource_mut::<CoreEffects>() = menu_action();
        app.update();

        let mut overlays = app
            .world_mut()
            .query_filtered::<Entity, With<SettingsMenuOverlay>>();
        assert_eq!(
            overlays.iter(app.world()).count(),
            0,
            "the existing menu action toggles the one overlay closed"
        );
    }

    #[test]
    fn the_existing_menu_exposes_binding_capture_and_reset_controls() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<bevy::input::mouse::MouseButton>::default())
            .add_plugins(ClientShellPlugin);
        app.update();

        let mut texts = app.world_mut().query::<&Text>();
        let visible = texts
            .iter(app.world())
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(
            visible.contains(&"Key bindings"),
            "the menu lists mod actions"
        );
        assert!(
            visible.contains(&"Change"),
            "each action can enter physical capture"
        );
        assert!(
            visible.contains(&"Reset"),
            "each action can restore its package default"
        );
        assert!(
            visible.contains(&"Key move"),
            "mod actions use readable labels"
        );
        assert!(
            visible.contains(&"Pan camera up"),
            "unoverridden engine defaults remain editable"
        );
        assert!(visible.contains(&"Audio levels"));
        assert!(
            visible.contains(&"M"),
            "the row shows the shared display label for Forge's physical KeyM default"
        );

        let (action_label, label_font, label_node) = {
            let world = app.world_mut();
            let mut rows = world.query::<(&Text, &TextFont, &Node)>();
            let (text, font, node) = rows
                .iter(world)
                .find(|(text, _, _)| text.0 == "Pan camera down")
                .expect("actual engine action label is in the menu");
            (text.0.clone(), font.font_size, node.width)
        };
        assert_eq!(action_label, "Pan camera down");
        assert_eq!(label_font, FontSize::Px(16.0));
        assert_eq!(label_node, Val::Px(MENU_ACTION_LABEL_WIDTH));
        let (action_end, audio_heading, first_audio_control) = {
            let world = app.world_mut();
            let mut rows = world.query::<(Entity, &Text)>();
            let rows = rows
                .iter(world)
                .map(|(entity, text)| (entity, text.0.as_str()))
                .collect::<Vec<_>>();
            let find = |label| {
                rows.iter()
                    .find(|(_, text)| *text == label)
                    .map(|(entity, _)| entity.index())
                    .unwrap()
            };
            (find("Zoom out"), find("Audio levels"), find("Master"))
        };
        assert!(
            action_end < audio_heading && audio_heading < first_audio_control,
            "the audio heading stays with the audio controls after the binding list"
        );

        app.world_mut()
            .init_resource::<crate::debug_feed::UiSnapshotFeed>();
        crate::widgets::snapshot::publish_snapshot(app.world_mut());
        let snapshot = app
            .world()
            .resource::<crate::debug_feed::UiSnapshotFeed>()
            .0
            .clone();
        let menu = snapshot
            .as_array()
            .expect("UiSnapshotFeed remains an actual row array")
            .iter()
            .find(|row| row["kind"] == "settings_menu")
            .expect("the actual settings menu root is in the existing UI snapshot");
        assert_eq!(menu["capture_root"], true);
        assert_eq!(menu["focus_policy_block"], true);
        assert_eq!(menu["rect"], serde_json::Value::Null);
        assert_eq!(menu["visible"], false, "no camera is not rendered proof");
        assert!(menu["text_nodes"].as_array().unwrap().iter().any(|node| {
            node["text"] == "Settings"
                && node["rect"] == serde_json::Value::Null
                && node["visible"] == false
        }));
        assert_eq!(menu["descendants_truncated"], false);
        assert_eq!(menu["text_truncated"], false);
        let mut scroll_areas = app
            .world_mut()
            .query_filtered::<Entity, With<bevy::ui_widgets::ScrollArea>>();
        assert_eq!(
            scroll_areas.iter(app.world()).count(),
            1,
            "the complete settings list stays reachable in a bounded scroll area"
        );
    }

    #[test]
    fn focus_feed_loss_shows_a_capturing_click_to_play_overlay_and_recovery_removes_it() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(CoreEffects::default())
            .insert_resource(VolumeSettings::default())
            .insert_resource(FocusFeed(false))
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    focused: false,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.update();

        {
            let world = app.world_mut();
            let mut overlays = world.query_filtered::<Entity, With<UiCaptureRoot>>();
            let overlay = overlays
                .single(world)
                .expect("FocusFeed(false) shows one click-to-play overlay");
            assert!(world.entity(overlay).contains::<UiCaptureRoot>());
            assert_eq!(
                world.entity(overlay).get::<bevy::ui::FocusPolicy>(),
                Some(&bevy::ui::FocusPolicy::Block),
                "the focus overlay prevents clicks from reaching the game world"
            );
            let mut labels = world.query::<&Text>();
            assert!(labels.iter(world).any(|text| text.0 == "Click to play"));
        }

        *app.world_mut().resource_mut::<FocusFeed>() = FocusFeed(true);
        app.world_mut()
            .get_mut::<Window>(window)
            .expect("the test's primary window exists")
            .focused = true;
        app.update();
        let mut overlays = app
            .world_mut()
            .query_filtered::<Entity, With<UiCaptureRoot>>();
        assert_eq!(
            overlays.iter(app.world()).count(),
            0,
            "focus-feed recovery removes the overlay"
        );

        app.world_mut()
            .insert_resource(crate::debug_feed::AudioFeed {
                context_states: vec!["suspended".into()],
                gesture_seen: false,
                ..default()
            });
        app.update();
        let mut overlays = app
            .world_mut()
            .query_filtered::<Entity, With<UiCaptureRoot>>();
        let overlay = overlays
            .single(app.world())
            .expect("a locked browser audio context shows the click-to-play overlay");
        assert!(app.world().entity(overlay).contains::<FocusOverlay>());

        {
            let mut audio = app
                .world_mut()
                .resource_mut::<crate::debug_feed::AudioFeed>();
            audio.context_states = vec!["running".into()];
            audio.gesture_seen = true;
        }
        app.update();
        let mut overlays = app
            .world_mut()
            .query_filtered::<Entity, With<UiCaptureRoot>>();
        assert_eq!(
            overlays.iter(app.world()).count(),
            0,
            "a real running, gesture-unlocked audio context removes the overlay"
        );
    }

    const SOUND_OFF_NOTICE: &str = "Sound is off. Click or press a key to turn it on.";

    /// A shell app with a focused canvas and a fixed frame step of 250 ms.
    fn start_gate_app(audio: crate::debug_feed::AudioFeed) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(CoreEffects::default())
            .insert_resource(VolumeSettings::default())
            .insert_resource(FocusFeed(true))
            .insert_resource(audio)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(250),
            ))
            .add_plugins(ClientShellPlugin);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ));
        app
    }

    fn focus_overlays(app: &mut App) -> usize {
        let mut overlays = app
            .world_mut()
            .query_filtered::<Entity, With<FocusOverlay>>();
        overlays.iter(app.world()).count()
    }

    fn shows_text(app: &mut App, wanted: &str) -> bool {
        let mut texts = app.world_mut().query::<&Text>();
        texts.iter(app.world()).any(|text| text.0 == wanted)
    }

    /// Design 10.1 with ruling O15: the start overlay waits for the first gesture only. When
    /// audio stays locked after it, input works and a notice shows for 8 s.
    #[test]
    fn locked_audio_after_the_first_gesture_removes_the_overlay_and_shows_the_sound_notice() {
        let mut app = start_gate_app(crate::debug_feed::AudioFeed {
            context_states: vec!["suspended".into()],
            gesture_seen: false,
            ..default()
        });
        app.update();
        assert_eq!(focus_overlays(&mut app), 1, "no gesture yet: Click to play");
        assert!(!shows_text(&mut app, SOUND_OFF_NOTICE));

        app.world_mut()
            .resource_mut::<crate::debug_feed::AudioFeed>()
            .gesture_seen = true;
        app.update();
        assert_eq!(
            focus_overlays(&mut app),
            0,
            "the first gesture removes the overlay, also when audio stays locked"
        );
        let mut first_shown = None;
        let mut last_shown = None;
        for frame in 0..48 {
            app.update();
            assert_eq!(
                focus_overlays(&mut app),
                0,
                "input works while audio is locked"
            );
            if shows_text(&mut app, SOUND_OFF_NOTICE) {
                first_shown.get_or_insert(frame);
                last_shown = Some(frame);
                // Input works: the notice takes no pointer input.
                let world = app.world_mut();
                let notice = world
                    .query_filtered::<Entity, With<SoundOffNotice>>()
                    .single(world)
                    .expect("one sound notice");
                assert!(
                    world.get::<UiCaptureRoot>(notice).is_none(),
                    "the sound notice must not capture world input"
                );
                assert_eq!(
                    world.get::<bevy::picking::Pickable>(notice),
                    Some(&bevy::picking::Pickable::IGNORE),
                    "the sound notice lets the pointer through"
                );
            }
        }
        let (first, last) = (first_shown.unwrap(), last_shown.unwrap());
        assert!(first <= 4, "the notice shows within 1 s: frame {first}");
        let frames = last - first + 1;
        assert!(
            (31..=33).contains(&frames),
            "the notice shows for 8 s (32 frames of 250 ms): {frames}"
        );

        *app.world_mut().resource_mut::<CoreEffects>() = menu_action();
        app.update();
        assert!(
            shows_text(&mut app, "Sound: off"),
            "the menu says Sound: off"
        );
        app.world_mut().resource_mut::<CoreEffects>().0.clear();
        app.world_mut()
            .resource_mut::<crate::debug_feed::AudioFeed>()
            .context_states = vec!["running".into()];
        app.update();
        assert!(
            shows_text(&mut app, "Sound: on"),
            "the menu follows the audio"
        );
        assert!(!shows_text(&mut app, "Sound: off"));

        *app.world_mut().resource_mut::<FocusFeed>() = FocusFeed(false);
        app.update();
        assert_eq!(
            focus_overlays(&mut app),
            1,
            "a focus loss shows the overlay again"
        );
    }

    #[test]
    fn running_audio_after_the_first_gesture_shows_no_sound_notice() {
        let mut app = start_gate_app(crate::debug_feed::AudioFeed {
            context_states: vec!["running".into()],
            gesture_seen: true,
            ..default()
        });
        for _ in 0..12 {
            app.update();
            assert_eq!(focus_overlays(&mut app), 0);
            assert!(!shows_text(&mut app, SOUND_OFF_NOTICE));
        }
        *app.world_mut().resource_mut::<CoreEffects>() = menu_action();
        app.update();
        assert!(shows_text(&mut app, "Sound: on"));
    }

    #[test]
    fn a_conflicting_binding_reports_the_other_action_without_mutating_overrides() {
        let defaults = forge_defaults();
        let mut overrides = defaults.clone();
        let before = overrides.clone();

        let result =
            edit_binding_override(&mut overrides, &defaults, "key_move", key("KeyS"), None);

        assert_eq!(
            result,
            BindingEditResult::Conflict {
                actions: vec!["key_stop".into()]
            }
        );
        assert_eq!(overrides, before, "a prompt cannot mutate either binding");
    }

    #[test]
    fn actual_multi_owner_conflict_swaps_every_shared_pointer_owner() {
        let (mut app, window, _) = open_binding_menu_for_capture();
        activate_named_change(app.world_mut(), "key_stop");
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(mouse_event(
                    window,
                    bevy::input::mouse::MouseButton::Left,
                    state,
                ));
        }
        app.update();
        app.update();
        assert!(
            app.world()
                .resource::<BindingEditorState>()
                .conflict
                .is_some()
        );
        let world = app.world_mut();
        let swap = world
            .query::<(Entity, &BindingButton)>()
            .iter(world)
            .find_map(|(entity, button)| {
                matches!(button.kind, BindingButtonKind::Swap).then_some(entity)
            })
            .expect("the actual shared Left conflict must offer Swap");
        world.trigger(Activate { entity: swap });
        world.flush();
        let overrides = &world.resource::<CorePreferences>().runtime_bindings;
        assert_eq!(
            overrides.get("key_stop"),
            Some(&vec![Binding::Pointer {
                button: turts_mod_api::Button::Left,
                mods: vec![]
            }])
        );
        for action in ["box_select", "select"] {
            assert_eq!(
                overrides.get(action),
                Some(&vec![key("KeyS")]),
                "Swap must move every owner of the shared Left chord, preserving the select/box pair"
            );
        }
    }

    #[test]
    fn binding_conflicts_use_the_dispatchers_modifier_set_semantics() {
        use turts_mod_api::Mod;
        let mut results = Vec::new();
        for (authored, captured) in [
            (
                Binding::Key {
                    code: "KeyR".into(),
                    mods: vec![Mod::Ctrl, Mod::Shift],
                },
                Binding::Key {
                    code: "KeyR".into(),
                    mods: vec![Mod::Shift, Mod::Ctrl],
                },
            ),
            (
                Binding::Pointer {
                    button: turts_mod_api::Button::Back,
                    mods: vec![Mod::Ctrl, Mod::Shift],
                },
                Binding::Pointer {
                    button: turts_mod_api::Button::Back,
                    mods: vec![Mod::Shift, Mod::Ctrl],
                },
            ),
        ] {
            let mut defaults = forge_defaults();
            defaults.insert("key_move".into(), vec![authored]);
            let mut overrides = BTreeMap::new();
            results.push(edit_binding_override(
                &mut overrides,
                &defaults,
                "key_stop",
                captured,
                None,
            ));
            assert!(
                overrides.is_empty(),
                "the unresolved equivalent chord cannot change either action"
            );
        }
        assert_eq!(
            results,
            vec![
                BindingEditResult::Conflict {
                    actions: vec!["key_move".into()]
                };
                2
            ]
        );
    }

    #[test]
    fn reset_compares_modifier_sets_and_keeps_equivalent_authored_sharing() {
        use turts_mod_api::Mod;
        let mut defaults = forge_defaults();
        let chord = Binding::Key {
            code: "KeyR".into(),
            mods: vec![Mod::Ctrl, Mod::Shift],
        };
        let reversed = Binding::Key {
            code: "KeyR".into(),
            mods: vec![Mod::Shift, Mod::Ctrl],
        };
        defaults.insert("key_stop".into(), vec![chord]);
        let mut overrides = BTreeMap::from([
            ("key_stop".into(), vec![key("KeyQ")]),
            ("key_move".into(), vec![reversed]),
        ]);
        assert!(reset_binding_override(
            &mut overrides,
            &defaults,
            "key_stop"
        ));
        assert_eq!(overrides.get("key_move"), Some(&vec![key("KeyQ")]));
        let pointer = Binding::Pointer {
            button: turts_mod_api::Button::Left,
            mods: vec![Mod::Ctrl, Mod::Shift],
        };
        let reversed = Binding::Pointer {
            button: turts_mod_api::Button::Left,
            mods: vec![Mod::Shift, Mod::Ctrl],
        };
        defaults.insert("select".into(), vec![pointer.clone()]);
        defaults.insert("box_select".into(), vec![reversed]);
        overrides.insert("select".into(), vec![key("KeyZ")]);
        overrides.insert("box_select".into(), vec![pointer.clone()]);
        assert!(reset_binding_override(&mut overrides, &defaults, "select"));
        assert_eq!(overrides.get("box_select"), Some(&vec![pointer]));
    }

    #[test]
    fn reset_fallback_cannot_steal_an_unrelated_players_binding() {
        let defaults = forge_defaults();
        let mut overrides = BTreeMap::new();
        assert_eq!(
            edit_binding_override(&mut overrides, &defaults, "key_stop", key("KeyZ"), None),
            BindingEditResult::Applied
        );
        assert_eq!(
            edit_binding_override(&mut overrides, &defaults, "key_move", key("KeyS"), None),
            BindingEditResult::Applied
        );
        assert_eq!(
            edit_binding_override(
                &mut overrides,
                &defaults,
                "key_stop",
                key("Escape"),
                Some(BindingConflictChoice::Swap)
            ),
            BindingEditResult::Applied
        );
        let untouched_move = overrides["key_move"].clone();
        let untouched_cancel = overrides["cancel"].clone();
        assert!(reset_binding_override(&mut overrides, &defaults, "menu"));
        assert!(!overrides.contains_key("menu"));
        assert_eq!(overrides["key_move"], untouched_move);
        assert_eq!(overrides["cancel"], untouched_cancel);
        let mut effective = defaults.clone();
        effective.extend(overrides);
        for binding in &effective["key_stop"] {
            assert!(
                !effective["key_move"].contains(binding) && !effective["cancel"].contains(binding),
                "Reset must not displace Stop onto an unrelated active Move or Cancel chord"
            );
        }
        assert!(
            effective["key_stop"].is_empty(),
            "no safe old or authored fallback remains; preserve the other owners and expose Unbound"
        );
    }

    #[test]
    fn reset_preserves_other_chords_and_authored_shared_defaults() {
        let defaults = forge_defaults();
        let mut overrides = BTreeMap::from([
            ("key_stop".into(), vec![key("KeyM")]),
            ("key_move".into(), vec![key("KeyS"), key("KeyP")]),
            ("queue_modifier".into(), vec![key("ShiftLeft")]),
        ]);
        assert!(reset_binding_override(
            &mut overrides,
            &defaults,
            "key_stop"
        ));
        assert_eq!(
            overrides.get("key_move"),
            Some(&vec![key("KeyM"), key("KeyP")])
        );
        assert_eq!(
            overrides.get("queue_modifier"),
            Some(&vec![key("ShiftLeft")])
        );
        assert!(!overrides.contains_key("key_stop"));
        // An already-default action must still repair a conflicting player chord.
        overrides.insert("key_move".into(), vec![key("KeyS")]);
        assert!(reset_binding_override(
            &mut overrides,
            &defaults,
            "key_stop"
        ));
        assert!(!overrides.contains_key("key_move"));
        assert!(!reset_binding_override(
            &mut overrides,
            &defaults,
            "key_stop"
        ));
    }

    #[test]
    fn actual_menu_swap_then_reset_keeps_key_s_executable_as_typed_stop() {
        use turts_interaction::{
            Core, Effect, InputEvent, NodeView, UnitView, Viewport, WorldView,
        };
        use turts_mod_api::{InputMapDef, OrderDef, OrderRulesDef};
        use turts_net_protocol::{SlotId, UnitId};
        let (mut app, window, _) = open_binding_menu_for_capture();
        activate_named_change(app.world_mut(), "key_stop");
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(keyboard_event(window, KeyCode::KeyM, state, false));
        }
        app.update();
        app.update();
        let swap = {
            let world = app.world_mut();
            world
                .query::<(Entity, &BindingButton)>()
                .iter(world)
                .find_map(|(entity, button)| {
                    matches!(button.kind, BindingButtonKind::Swap).then_some(entity)
                })
                .unwrap()
        };
        app.world_mut().trigger(Activate { entity: swap });
        app.world_mut().flush();
        let reset = {
            let world = app.world_mut();
            world
                .query::<(Entity, &BindingButton)>()
                .iter(world)
                .find_map(|(entity, button)| {
                    (button.action == "key_stop" && matches!(button.kind, BindingButtonKind::Reset))
                        .then_some(entity)
                })
                .unwrap()
        };
        app.world_mut().trigger(Activate { entity: reset });
        app.world_mut().flush();
        struct ResetWorld {
            units: Vec<UnitView>,
            orders: Vec<OrderDef>,
            input: InputMapDef,
            rules: OrderRulesDef,
            overrides: BTreeMap<String, Vec<Binding>>,
        }
        impl WorldView for ResetWorld {
            fn units(&self) -> &[UnitView] {
                &self.units
            }
            fn nodes(&self) -> &[NodeView] {
                &[]
            }
            fn own_slot(&self) -> SlotId {
                SlotId(0)
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
                &self.overrides
            }
            fn selection_priority(&self) -> &[String] {
                &[]
            }
            fn can_gather(&self, _: &UnitView, _: &NodeView) -> bool {
                false
            }
            fn can_weapon_target(&self, _: &UnitView, _: &UnitView) -> bool {
                false
            }
        }
        let mut orders = vec![
            ron::from_str::<OrderDef>(include_str!("../../../mods/forge/orders/order_stop.ron"))
                .unwrap(),
            ron::from_str::<OrderDef>(include_str!("../../../mods/forge/orders/order_move.ron"))
                .unwrap(),
        ];
        for order in &mut orders {
            order.id = format!("forge:{}", order.id);
        }
        let world = ResetWorld {
            units: vec![UnitView {
                id: UnitId(7),
                content_id: "forge:drone".into(),
                owner_slot: SlotId(0),
                position: Vec2::ZERO,
                footprint_radius: 1.0,
                capabilities: vec![],
                content_index: 0,
            }],
            orders,
            input: forge_input_defaults(),
            rules: ron::from_str(include_str!("../../../mods/forge/orders/rules.ron")).unwrap(),
            overrides: app
                .world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .clone(),
        };
        let mut core = Core::default();
        let viewport = Viewport {
            size: Vec2::new(1280.0, 720.0),
            scale_factor: 1.0,
        };
        let point = core.project_world_to_screen(Vec2::ZERO, &viewport);
        core.step(
            &[
                InputEvent::PointerDown {
                    button: turts_mod_api::Button::Left,
                    px: point,
                    over_widget: false,
                },
                InputEvent::PointerUp {
                    button: turts_mod_api::Button::Left,
                    px: point,
                    over_widget: false,
                },
            ],
            &world,
            &viewport,
            0,
        );
        assert_eq!(core.selected_units(), &[UnitId(7)]);
        let effects = core.step(
            &[InputEvent::KeyDown {
                code: "KeyS".into(),
                mods: vec![],
            }],
            &world,
            &viewport,
            100,
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(order)
            if order.order == "forge:order_stop" && order.units == [UnitId(7)])),
            "actual menu Swap then Reset must leave KeyS sending typed Stop, not arming Move"
        );
    }

    #[test]
    fn a_declared_action_with_no_default_binding_can_be_bound_in_the_real_menu() {
        let mut app = App::new();
        let mut acquired = forge_acquired_packages();
        acquired
            .0
            .content
            .input_map
            .actions
            .push(turts_mod_api::ActionDef {
                name: "optional_action".into(),
                default: vec![],
            });
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(acquired)
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();
        app.world_mut().resource_mut::<CoreEffects>().0.clear();
        activate_named_change(app.world_mut(), "optional_action");
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::KeyP,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("optional_action"),
            Some(&vec![key("KeyP")]),
            "a valid declared action is editable even when its mod starts it unbound"
        );
    }

    #[test]
    fn cancel_preserves_bindings_and_swap_moves_the_exact_previous_chord() {
        let defaults = forge_defaults();
        let mut overrides = defaults.clone();
        let before = overrides.clone();

        let cancelled = edit_binding_override(
            &mut overrides,
            &defaults,
            "key_move",
            key("KeyS"),
            Some(BindingConflictChoice::Cancel),
        );
        assert_eq!(cancelled, BindingEditResult::Cancelled);
        assert_eq!(overrides, before, "Cancel leaves both actions unchanged");

        let swapped = edit_binding_override(
            &mut overrides,
            &defaults,
            "key_move",
            key("KeyS"),
            Some(BindingConflictChoice::Swap),
        );
        assert_eq!(swapped, BindingEditResult::Applied);
        assert_eq!(overrides["key_move"], vec![key("KeyS")]);
        assert_eq!(overrides["key_stop"], vec![key("KeyM")]);
    }

    #[test]
    fn reset_removes_the_override_and_restores_the_verified_package_default() {
        let input = forge_input_defaults();
        let defaults = turts_interaction::input_map::effective_actions(&input);
        let mut overrides = BTreeMap::from([("key_move".into(), vec![key("KeyQ")])]);

        assert!(reset_binding_override(
            &mut overrides,
            &defaults,
            "key_move"
        ));
        assert_eq!(
            turts_interaction::input_map::effective_actions_with_runtime(&input, &overrides)["key_move"],
            defaults["key_move"],
            "Reset resolves the actual package default, not a caller-supplied key"
        );
    }

    #[test]
    fn reserved_browser_chords_are_rejected_without_changing_any_action() {
        let defaults = forge_defaults();
        let mut overrides = defaults.clone();
        let before = overrides.clone();
        let reserved = Binding::Key {
            code: "F12".into(),
            mods: vec![],
        };

        assert_eq!(
            edit_binding_override(&mut overrides, &defaults, "key_move", reserved, None),
            BindingEditResult::Reserved
        );
        assert_eq!(overrides, before);
    }

    #[derive(Resource)]
    struct TestPointerClicks {
        entity: Entity,
        count: usize,
    }

    fn inject_test_pointer_clicks(world: &mut World) {
        let Some(test_clicks) = world.remove_resource::<TestPointerClicks>() else {
            return;
        };
        let window = {
            let mut windows = world.query_filtered::<Entity, With<PrimaryWindow>>();
            windows.single(world).expect("test has one primary window")
        };
        let location = bevy::picking::pointer::Location {
            target: bevy::camera::NormalizedRenderTarget::Window(
                bevy::window::WindowRef::Entity(window)
                    .normalize(Some(window))
                    .unwrap(),
            ),
            position: Vec2::ZERO,
        };
        let pointer =
            bevy::picking::events::Pointer::new(bevy::picking::pointer::PointerId::Mouse, location);
        for _ in 0..test_clicks.count {
            world.trigger(bevy::picking::events::PointerPress {
                entity: test_clicks.entity,
                pointer: pointer.clone(),
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(window, 0.0, None, None),
                count: 1,
            });
            world.flush();
            world.trigger(bevy::picking::events::PointerClick {
                entity: test_clicks.entity,
                pointer: pointer.clone(),
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(window, 0.0, None, None),
                duration: std::time::Duration::from_millis(10),
                count: 1,
            });
            world.flush();
            assert_eq!(
                world.resource::<BindingEditorState>().capturing.as_deref(),
                Some("key_move"),
                "the real ButtonPlugin click activation starts key_move capture before later input is read"
            );
            world.trigger(bevy::picking::events::PointerRelease {
                entity: test_clicks.entity,
                pointer: pointer.clone(),
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(window, 0.0, None, None),
            });
            world.flush();
        }
    }

    #[derive(Resource)]
    struct TestKeyboardActivation {
        entity: Entity,
        window: Entity,
    }

    fn inject_test_keyboard_activation(world: &mut World) {
        let Some(activation) = world.remove_resource::<TestKeyboardActivation>() else {
            return;
        };
        world.trigger(FocusedInput::new(
            activation.entity,
            bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: bevy::input::keyboard::Key::Enter,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: activation.window,
            },
            activation.window,
        ));
        world.flush();
        assert_eq!(
            world.resource::<BindingEditorState>().capturing.as_deref(),
            Some("key_move"),
            "the focused keyboard activation starts capture before the following key is read"
        );
    }

    fn open_binding_menu_for_capture() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<bevy::input::mouse::MouseButton>::default())
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();
        app.world_mut().resource_mut::<CoreEffects>().0.clear();
        let change = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &BindingButton)>();
            buttons
                .iter(world)
                .find(|(_, button)| {
                    button.action == "key_move" && matches!(button.kind, BindingButtonKind::Change)
                })
                .map(|(entity, _)| entity)
                .expect("Change is a real menu button")
        };
        app.add_systems(
            PreUpdate,
            (inject_test_pointer_clicks, inject_test_keyboard_activation),
        );
        (app, window, change)
    }

    fn queue_pointer_open_then_ctrl_right_batch(world: &mut World, window: Entity) {
        let left = bevy::input::mouse::MouseButton::Left;
        world
            .resource_mut::<Messages<WindowEvent>>()
            .write(mouse_event(window, left, bevy::input::ButtonState::Pressed));
        world
            .resource_mut::<Messages<WindowEvent>>()
            .write(mouse_event(
                window,
                left,
                bevy::input::ButtonState::Released,
            ));
        world
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::ControlLeft,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        world
            .resource_mut::<Messages<WindowEvent>>()
            .write(mouse_event(
                window,
                bevy::input::mouse::MouseButton::Right,
                bevy::input::ButtonState::Pressed,
            ));
        world
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::ControlLeft,
                bevy::input::ButtonState::Released,
                false,
            ));
    }

    #[test]
    fn opening_click_batch_keeps_the_later_ordered_ctrl_right_pointer_input() {
        let (mut app, window, change) = open_binding_menu_for_capture();
        queue_pointer_open_then_ctrl_right_batch(app.world_mut(), window);
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ControlLeft);
            keys.release(KeyCode::ControlLeft);
        }
        app.world_mut().insert_resource(TestPointerClicks {
            entity: change,
            count: 1,
        });
        app.update();

        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::ControlLeft),
            "the final ButtonInput modifier is up even though the intervening pointer press had Ctrl"
        );
        let expected = vec![Binding::Pointer {
            button: turts_mod_api::Button::Right,
            mods: vec![turts_mod_api::Mod::Ctrl],
        }];
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&expected),
            "capture correlates its cutoff to the opening release and keeps later Ctrl+Right in the same WindowEvent batch"
        );
    }

    #[test]
    fn pointer_activation_ignores_an_earlier_unmatched_release_in_the_same_batch() {
        let (mut app, window, change) = open_binding_menu_for_capture();
        let left = bevy::input::mouse::MouseButton::Left;
        for state in [
            bevy::input::ButtonState::Released,
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(mouse_event(window, left, state));
        }
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::KeyQ,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.world_mut().insert_resource(TestPointerClicks {
            entity: change,
            count: 1,
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&vec![Binding::Key {
                code: "KeyQ".into(),
                mods: vec![]
            }]),
            "a release without this button's press must not become the opening-click boundary"
        );
    }

    #[test]
    fn next_same_button_click_is_captured_without_restarting_change() {
        let (mut app, window, change) = open_binding_menu_for_capture();
        let left = bevy::input::mouse::MouseButton::Left;
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(mouse_event(window, left, state));
        }
        app.world_mut().insert_resource(TestPointerClicks {
            entity: change,
            count: 2,
        });
        app.update();
        let editor = app.world().resource::<BindingEditorState>();
        let (action, binding, other) = editor
            .conflict
            .as_ref()
            .expect("the next Left click must reach the actual binding conflict path");
        assert_eq!(action, "key_move");
        assert_eq!(
            binding,
            &Binding::Pointer {
                button: turts_mod_api::Button::Left,
                mods: vec![]
            }
        );
        assert!(
            other == &vec!["box_select".to_owned(), "select".to_owned()],
            "Left belongs to both real engine selection actions"
        );
        assert!(
            editor.capturing.is_none(),
            "duplicate Change activation must not restart capture"
        );
    }

    #[test]
    fn keyboard_activation_keeps_the_next_key_from_the_same_window_event_batch() {
        let (mut app, window, change) = open_binding_menu_for_capture();
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::Enter,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::KeyQ,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.world_mut().insert_resource(TestKeyboardActivation {
            entity: change,
            window,
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&vec![Binding::Key {
                code: "KeyQ".into(),
                mods: vec![],
            }]),
            "focused Enter marks the activation boundary and preserves the following same-batch key"
        );
    }

    #[test]
    fn keyboard_activation_ignores_an_earlier_repeat_in_the_same_batch() {
        let (mut app, window, change) = open_binding_menu_for_capture();
        for (state, repeat) in [
            (bevy::input::ButtonState::Pressed, true),
            (bevy::input::ButtonState::Released, false),
        ] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(keyboard_event(window, KeyCode::Enter, state, repeat));
        }

        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::Enter,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::KeyQ,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.world_mut().insert_resource(TestKeyboardActivation {
            entity: change,
            window,
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&vec![Binding::Key {
                code: "KeyQ".into(),
                mods: vec![],
            }]),
            "focused Enter marks the activation boundary and preserves the following same-batch key"
        );
    }

    #[test]
    fn secondary_window_modifier_does_not_change_primary_window_pointer_capture() {
        let (mut app, primary, change) = open_binding_menu_for_capture();
        let secondary = app.world_mut().spawn((Window::default(),)).id();
        let left = bevy::input::mouse::MouseButton::Left;
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(mouse_event(
                primary,
                left,
                bevy::input::ButtonState::Pressed,
            ));
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(mouse_event(
                primary,
                left,
                bevy::input::ButtonState::Released,
            ));
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                secondary,
                KeyCode::ControlLeft,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(mouse_event(
                primary,
                bevy::input::mouse::MouseButton::Back,
                bevy::input::ButtonState::Pressed,
            ));
        app.world_mut().insert_resource(TestPointerClicks {
            entity: change,
            count: 1,
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&vec![Binding::Pointer {
                button: turts_mod_api::Button::Back,
                mods: vec![],
            }]),
            "secondary-window modifiers are filtered before the primary-window event fold"
        );
    }

    #[test]
    fn layout_labels_normalize_ascii_letters_and_preserve_punctuation_and_unicode() {
        let labels = BindingLabels::from_layout_entries([
            ("KeyA".into(), "q".into()),
            ("Digit1".into(), "&".into()),
            ("KeyE".into(), "é".into()),
            ("Space".into(), " ".into()),
            ("KeyZ".into(), String::new()),
            ("Tab".into(), "\t".into()),
        ]);
        assert_eq!(labels.0["KeyA"], "Q");
        assert_eq!(labels.0["Digit1"], "&");
        assert_eq!(labels.0["KeyE"], "é");
        assert!(!labels.0.contains_key("Space"));
        assert!(!labels.0.contains_key("KeyZ"));
        assert!(!labels.0.contains_key("Tab"));
    }

    fn keyboard_event(
        window: Entity,
        key_code: KeyCode,
        state: bevy::input::ButtonState,
        repeat: bool,
    ) -> WindowEvent {
        let logical_key = match key_code {
            KeyCode::ControlLeft | KeyCode::ControlRight => bevy::input::keyboard::Key::Control,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => bevy::input::keyboard::Key::Shift,
            KeyCode::AltLeft | KeyCode::AltRight => bevy::input::keyboard::Key::Alt,
            KeyCode::Escape => bevy::input::keyboard::Key::Escape,
            _ => bevy::input::keyboard::Key::Character("r".into()),
        };
        WindowEvent::KeyboardInput(bevy::input::keyboard::KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat,
            window,
        })
    }

    fn mouse_event(
        window: Entity,
        button: bevy::input::mouse::MouseButton,
        state: bevy::input::ButtonState,
    ) -> WindowEvent {
        WindowEvent::MouseButtonInput(bevy::input::mouse::MouseButtonInput {
            button,
            state,
            window,
        })
    }

    #[test]
    fn capture_folds_left_and_right_modifier_transitions_in_message_order() {
        let window = Entity::from_bits(7);
        for (modifier_key, modifier) in [
            (KeyCode::ControlLeft, turts_mod_api::Mod::Ctrl),
            (KeyCode::ControlRight, turts_mod_api::Mod::Ctrl),
            (KeyCode::ShiftLeft, turts_mod_api::Mod::Shift),
            (KeyCode::ShiftRight, turts_mod_api::Mod::Shift),
            (KeyCode::AltLeft, turts_mod_api::Mod::Alt),
            (KeyCode::AltRight, turts_mod_api::Mod::Alt),
        ] {
            let events = [
                keyboard_event(
                    window,
                    modifier_key,
                    bevy::input::ButtonState::Pressed,
                    false,
                ),
                mouse_event(
                    window,
                    bevy::input::mouse::MouseButton::Right,
                    bevy::input::ButtonState::Pressed,
                ),
                keyboard_event(
                    window,
                    modifier_key,
                    bevy::input::ButtonState::Released,
                    false,
                ),
            ];
            let mut state = CaptureModifiers::default();
            let captured = events
                .iter()
                .find_map(|event| captured_binding(event, Some(window), &mut state));
            assert_eq!(
                captured,
                Some(Binding::Pointer {
                    button: turts_mod_api::Button::Right,
                    mods: vec![modifier]
                }),
                "each left/right modifier is active at the intervening pointer press"
            );
        }
    }

    #[test]
    fn held_modifier_release_before_pointer_press_is_not_captured() {
        let window = Entity::from_bits(7);
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ControlRight);
        let mut state = CaptureModifiers::from_keys(Some(&keys));
        let events = [
            keyboard_event(
                window,
                KeyCode::ControlRight,
                bevy::input::ButtonState::Released,
                false,
            ),
            mouse_event(
                window,
                bevy::input::mouse::MouseButton::Right,
                bevy::input::ButtonState::Pressed,
            ),
        ];
        let captured = events
            .iter()
            .find_map(|event| captured_binding(event, Some(window), &mut state));
        assert_eq!(
            captured,
            Some(Binding::Pointer {
                button: turts_mod_api::Button::Right,
                mods: vec![]
            }),
            "a modifier held before the frame is cleared by its earlier release message"
        );
    }

    #[test]
    fn secondary_window_modifier_does_not_fold_and_primary_focus_loss_clears_modifiers() {
        let primary = Entity::from_bits(7);
        let secondary = Entity::from_bits(8);
        let mut state = CaptureModifiers::default();
        assert_eq!(
            captured_binding(
                &keyboard_event(
                    secondary,
                    KeyCode::ControlLeft,
                    bevy::input::ButtonState::Pressed,
                    false,
                ),
                Some(primary),
                &mut state,
            ),
            None,
            "secondary-window keyboard messages are rejected before modifier folding"
        );
        assert_eq!(
            captured_binding(
                &mouse_event(
                    primary,
                    bevy::input::mouse::MouseButton::Right,
                    bevy::input::ButtonState::Pressed,
                ),
                Some(primary),
                &mut state,
            ),
            Some(Binding::Pointer {
                button: turts_mod_api::Button::Right,
                mods: vec![],
            })
        );

        let mut held_keys = ButtonInput::<KeyCode>::default();
        held_keys.press(KeyCode::ShiftRight);
        let mut held = CaptureModifiers::from_keys(Some(&held_keys));
        captured_binding(
            &WindowEvent::WindowFocused(bevy::window::WindowFocused {
                window: primary,
                focused: false,
            }),
            Some(primary),
            &mut held,
        );
        assert_eq!(
            captured_binding(
                &mouse_event(
                    primary,
                    bevy::input::mouse::MouseButton::Right,
                    bevy::input::ButtonState::Pressed,
                ),
                Some(primary),
                &mut held,
            ),
            Some(Binding::Pointer {
                button: turts_mod_api::Button::Right,
                mods: vec![],
            }),
            "primary-window focus loss clears a held modifier"
        );
    }

    #[test]
    fn capture_reads_actual_pointer_events_with_physical_button_and_modifiers() {
        let window = Entity::from_bits(7);
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::ShiftRight);
        let event = WindowEvent::MouseButtonInput(bevy::input::mouse::MouseButtonInput {
            button: bevy::input::mouse::MouseButton::Right,
            state: bevy::input::ButtonState::Pressed,
            window,
        });
        let mut modifiers = CaptureModifiers::from_keys(Some(&keys));
        assert_eq!(
            captured_binding(&event, Some(window), &mut modifiers),
            Some(Binding::Pointer {
                button: turts_mod_api::Button::Right,
                mods: vec![turts_mod_api::Mod::Ctrl, turts_mod_api::Mod::Shift],
            }),
            "the producer uses the real pressed WindowEvent and the current modifier state"
        );
    }

    #[test]
    fn capture_escape_is_recognized_as_cancel_instead_of_a_binding() {
        let window = Entity::from_bits(7);
        let event = WindowEvent::KeyboardInput(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        assert_eq!(
            captured_binding(&event, Some(window), &mut CaptureModifiers::default()),
            Some(Binding::Key {
                code: "Escape".into(),
                mods: vec![]
            }),
            "capture receives the physical Escape code; capture_physical_binding cancels it"
        );
    }

    fn baseline_mouse_event(
        window: Entity,
        button: bevy::input::mouse::MouseButton,
        state: bevy::input::ButtonState,
    ) -> WindowEvent {
        WindowEvent::MouseButtonInput(bevy::input::mouse::MouseButtonInput {
            button,
            state,
            window,
        })
    }

    pub(crate) fn activate_named_change(world: &mut World, action: &str) {
        let entity = world
            .query::<(Entity, &BindingButton)>()
            .iter(world)
            .find_map(|(entity, button)| {
                (button.action == action && matches!(button.kind, BindingButtonKind::Change))
                    .then_some(entity)
            })
            .expect("the actual menu must contain the named Change button");
        world.trigger(Activate { entity });
        world.flush();
        assert_eq!(
            world.resource::<BindingEditorState>().capturing.as_deref(),
            Some(action)
        );
    }

    #[test]
    fn standalone_modifier_release_captures_a_real_rebindable_modifier_key() {
        let (mut app, window, _) = open_binding_menu_for_capture();
        activate_named_change(app.world_mut(), "queue_modifier");
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(keyboard_event(window, KeyCode::AltRight, state, false));
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("queue_modifier"),
            Some(&vec![Binding::Key {
                code: "AltRight".into(),
                mods: vec![]
            }]),
            "a standalone modifier key must remain rebindable while modifier chords still wait for their key/button"
        );
    }

    #[test]
    fn standalone_shared_default_modifier_reaches_the_real_conflict_prompt() {
        let (mut app, window, _) = open_binding_menu_for_capture();
        activate_named_change(app.world_mut(), "queue_modifier");
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut()
                .resource_mut::<Messages<WindowEvent>>()
                .write(keyboard_event(window, KeyCode::ShiftRight, state, false));
        }
        app.update();
        let editor = app.world().resource::<BindingEditorState>();
        let (action, binding, other) = editor
            .conflict
            .as_ref()
            .expect("an already shared Shift chord must offer the actual swap-or-cancel prompt");
        assert_eq!(action, "queue_modifier");
        assert_eq!(other, &vec!["add_modifier".to_owned()]);
        assert_eq!(
            binding,
            &Binding::Key {
                code: "ShiftRight".into(),
                mods: vec![]
            }
        );
        assert!(
            !app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .contains_key("queue_modifier")
        );
    }

    #[test]
    fn releasing_a_modifier_held_before_capture_does_not_bind_it() {
        let (mut app, window, _) = open_binding_menu_for_capture();
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::ShiftRight,
                bevy::input::ButtonState::Pressed,
                false,
            ));
        app.update();
        app.update();
        activate_named_change(app.world_mut(), "queue_modifier");
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(keyboard_event(
                window,
                KeyCode::ShiftRight,
                bevy::input::ButtonState::Released,
                false,
            ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<BindingEditorState>()
                .capturing
                .as_deref(),
            Some("queue_modifier"),
            "release of an old held modifier is not a new binding gesture"
        );
        assert!(
            !app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .contains_key("queue_modifier")
        );
    }

    #[test]
    fn baseline_capture_producer_omits_a_real_pointer_button_binding() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<bevy::input::mouse::MouseButton>::default())
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();
        app.world_mut().resource_mut::<CoreEffects>().0.clear();
        let change = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &BindingButton)>();
            buttons
                .iter(world)
                .find(|(_, button)| {
                    button.action == "key_move" && matches!(button.kind, BindingButtonKind::Change)
                })
                .map(|(entity, _)| entity)
                .expect("Change is a real menu button")
        };
        app.world_mut().trigger(Activate { entity: change });
        app.world_mut().flush();
        assert_eq!(
            app.world()
                .resource::<BindingEditorState>()
                .capturing
                .as_deref(),
            Some("key_move"),
            "the named producer test must start capture before the pointer message"
        );

        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(baseline_mouse_event(
                window,
                bevy::input::mouse::MouseButton::Back,
                bevy::input::ButtonState::Pressed,
            ));
        app.update();

        let expected = vec![Binding::Pointer {
            button: turts_mod_api::Button::Back,
            mods: vec![],
        }];
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&expected),
            "the actual pressed mouse WindowEvent must produce the physical unbound Back pointer binding"
        );
    }

    #[test]
    fn actual_menu_change_button_captures_a_physical_key_and_reset_restores_default() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();
        let change = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &BindingButton)>();
            buttons
                .iter(world)
                .find(|(_, button)| {
                    button.action == "key_move" && matches!(button.kind, BindingButtonKind::Change)
                })
                .map(|(entity, _)| entity)
                .expect("the actual key_move Change button is in the menu tree")
        };

        app.world_mut().trigger(Activate { entity: change });
        assert_eq!(
            app.world()
                .resource::<BindingEditorState>()
                .capturing
                .as_deref(),
            Some("key_move")
        );
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::KeyboardInput(
                bevy::input::keyboard::KeyboardInput {
                    key_code: KeyCode::KeyQ,
                    logical_key: bevy::input::keyboard::Key::Character("q".into()),
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            Some(&vec![key("KeyQ")]),
            "the physical input path changes only the selected action"
        );

        let reset = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &BindingButton)>();
            buttons
                .iter(world)
                .find(|(_, button)| {
                    button.action == "key_move" && matches!(button.kind, BindingButtonKind::Reset)
                })
                .map(|(entity, _)| entity)
                .expect("the actual key_move Reset button is in the menu tree")
        };
        app.world_mut().trigger(Activate { entity: reset });
        assert!(
            !app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .contains_key("key_move"),
            "Reset removes only the player override so the real package default applies"
        );
    }

    #[test]
    fn actual_settings_controls_change_camera_cursor_health_and_interaction_resources() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .insert_resource(crate::camera::CameraConfiguration::default())
            .insert_resource(bevy::ui::UiScale(1.0))
            .add_plugins(ClientShellPlugin);
        app.world_mut().spawn((
            Window::default(),
            PrimaryWindow,
            bevy::window::CursorOptions::default(),
        ));
        app.update();

        let text = |world: &mut World, wanted: &str| {
            let mut labels = world.query::<&Text>();
            labels.iter(world).any(|label| label.0.contains(wanted))
        };
        assert!(text(app.world_mut(), "Scroll speed"));
        assert!(text(app.world_mut(), "Keep the cursor in the game"));
        assert!(text(app.world_mut(), "Off"));
        assert!(text(app.world_mut(), "Edge scroll"));
        assert!(text(app.world_mut(), "Audio levels"));

        let button = |world: &mut World, wanted: SettingsToggle| {
            let mut query = world.query::<(Entity, &SettingsToggle)>();
            query
                .iter(world)
                .find_map(|(entity, value)| {
                    std::mem::discriminant(value)
                        .eq(&std::mem::discriminant(&wanted))
                        .then_some(entity)
                })
                .expect("the requested actual settings control is present")
        };
        let cursor_lock = button(app.world_mut(), SettingsToggle::CursorLock);
        app.world_mut().trigger(Activate {
            entity: cursor_lock,
        });
        assert!(app.world().resource::<ClientSettings>().cursor_lock);

        let edge_scroll = button(app.world_mut(), SettingsToggle::EdgeScroll);
        app.world_mut().trigger(Activate {
            entity: edge_scroll,
        });
        assert!(!app.world().resource::<ClientSettings>().edge_scroll);
        app.update();
        assert!(
            !app.world()
                .resource::<crate::camera::CameraConfiguration>()
                .options
                .edge_scroll
        );

        let speed = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &SettingsSlider)>();
            query
                .iter(world)
                .find_map(|(entity, setting)| {
                    matches!(setting, SettingsSlider::ScrollSpeed).then_some(entity)
                })
                .expect("the actual Scroll speed slider is present")
        };
        app.world_mut().entity_mut(speed).insert(SliderValue(1.75));
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::camera::CameraConfiguration>()
                .options
                .scroll_speed,
            1.75
        );

        let health_bars = button(app.world_mut(), SettingsToggle::HealthBars);
        app.world_mut().trigger(Activate {
            entity: health_bars,
        });
        assert_eq!(
            app.world().resource::<crate::visuals::HealthDisplay>().0,
            crate::visuals::HealthMode::Always
        );
        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .interaction
                .double_click_ms,
            app.world().resource::<ClientSettings>().double_click_ms
        );
    }

    #[test]
    fn pointer_lock_requires_the_opt_in_and_real_primary_click_and_releases_on_focus_loss() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(CoreEffects::default())
            .insert_resource(VolumeSettings::default())
            .insert_resource(ClientSettings {
                cursor_lock: true,
                ..ClientSettings::default()
            })
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((
                Window::default(),
                PrimaryWindow,
                bevy::window::CursorOptions::default(),
            ))
            .id();
        app.update();
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::MouseButtonInput(
                bevy::input::mouse::MouseButtonInput {
                    button: bevy::input::mouse::MouseButton::Left,
                    state: bevy::input::ButtonState::Pressed,
                    window,
                },
            ));
        app.update();
        let options = app
            .world()
            .get::<bevy::window::CursorOptions>(window)
            .unwrap();
        assert_eq!(options.grab_mode, bevy::window::CursorGrabMode::Locked);
        assert!(!options.visible);

        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::KeyboardFocusLost(
                bevy::input::keyboard::KeyboardFocusLost,
            ));
        app.update();
        let options = app
            .world()
            .get::<bevy::window::CursorOptions>(window)
            .unwrap();
        assert_eq!(options.grab_mode, bevy::window::CursorGrabMode::None);
        assert!(options.visible);
    }

    #[test]
    fn actual_swap_button_moves_the_conflicting_chord_to_the_previous_action_key() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();
        let change = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &BindingButton)>();
            buttons
                .iter(world)
                .find(|(_, button)| {
                    button.action == "key_move" && matches!(button.kind, BindingButtonKind::Change)
                })
                .map(|(entity, _)| entity)
                .expect("the actual key_move Change button is present")
        };
        app.world_mut().trigger(Activate { entity: change });
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::KeyboardInput(
                bevy::input::keyboard::KeyboardInput {
                    key_code: KeyCode::KeyS,
                    logical_key: bevy::input::keyboard::Key::Character("s".into()),
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            ));
        app.update();
        assert!(
            app.world()
                .resource::<BindingEditorState>()
                .conflict
                .is_some()
        );
        app.update();

        let swap = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &BindingButton)>();
            buttons
                .iter(world)
                .find(|(_, button)| matches!(button.kind, BindingButtonKind::Swap))
                .map(|(entity, _)| entity)
                .expect("the actual conflict prompt supplies a Swap button")
        };
        app.world_mut().trigger(Activate { entity: swap });
        let overrides = &app.world().resource::<CorePreferences>().runtime_bindings;
        assert_eq!(overrides.get("key_move"), Some(&vec![key("KeyS")]));
        assert_eq!(overrides.get("key_stop"), Some(&vec![key("KeyM")]));
        assert_eq!(
            app.world()
                .resource::<BindingEditorState>()
                .status
                .as_deref(),
            Some("Bindings swapped")
        );
    }

    #[test]
    fn a_key_seen_before_capture_does_not_become_the_next_binding() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(forge_acquired_packages())
            .add_plugins(ClientShellPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();

        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::KeyboardInput(
                bevy::input::keyboard::KeyboardInput {
                    key_code: KeyCode::KeyZ,
                    logical_key: bevy::input::keyboard::Key::Character("z".into()),
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            ));
        app.update();
        let change = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &BindingButton)>();
            buttons
                .iter(world)
                .find(|(_, button)| {
                    button.action == "key_move" && matches!(button.kind, BindingButtonKind::Change)
                })
                .map(|(entity, _)| entity)
                .expect("the actual key_move Change button is present")
        };
        app.world_mut().trigger(Activate { entity: change });
        app.update();

        assert_eq!(
            app.world()
                .resource::<CorePreferences>()
                .runtime_bindings
                .get("key_move"),
            None,
            "entering capture cannot reuse a physical key pressed before capture began"
        );
        assert_eq!(
            app.world()
                .resource::<BindingEditorState>()
                .capturing
                .as_deref(),
            Some("key_move")
        );
    }

    #[test]
    fn task16_menu_exposes_the_existing_player_double_click_distance() {
        let (mut app, _, _) = open_binding_menu_for_capture();
        let slider = {
            let world = app.world_mut();
            let label = world
                .query::<(Entity, &Text)>()
                .iter(world)
                .find_map(|(entity, text)| (text.0 == "Double-click range").then_some(entity))
                .expect(
                    "both double-click time and distance must be player-editable in the real menu",
                );
            let row = world.get::<ChildOf>(label).unwrap().parent();
            world
                .get::<Children>(row)
                .unwrap()
                .iter()
                .find(|entity| {
                    world.get::<SettingsSlider>(*entity).is_some()
                        && world.get::<SliderRange>(*entity).is_some()
                })
                .unwrap()
        };
        let range = app.world().get::<SliderRange>(slider).unwrap();
        assert_eq!((range.start(), range.end()), (0.0, 32.0));
        app.world_mut().entity_mut(slider).insert(SliderValue(8.0));
        app.update();
        app.update();
        let preferences = app.world().resource::<CorePreferences>();
        assert_eq!(
            preferences.interaction.double_click_pixels, 8.0,
            "the real distance slider must reach the existing Core preference"
        );
        let controls = crate::client_preferences::ControlSettings::capture(
            None,
            None,
            Some(preferences),
            None,
        );
        let encoded = crate::volume_settings::set_controls_in_json(None, &controls).unwrap();
        assert_eq!(
            crate::volume_settings::decode_settings_json(&encoded)
                .controls
                .double_click_pixels,
            8.0
        );
    }

    #[test]
    fn ui_scale_menu_offers_the_design_maximum() {
        let (mut app, _, _) = open_binding_menu_for_capture();
        app.insert_resource(bevy::ui::UiScale::default());
        let slider = {
            let world = app.world_mut();
            world
                .query::<(Entity, &SettingsSlider)>()
                .iter(world)
                .find_map(|(entity, setting)| {
                    matches!(setting, SettingsSlider::UiScale).then_some(entity)
                })
                .unwrap()
        };
        let range = app.world().get::<SliderRange>(slider).unwrap();
        assert_eq!(range.start(), 0.75);
        assert_eq!(
            range.end(),
            2.0,
            "the real menu must offer the complete design8.3 range"
        );
        app.world_mut().entity_mut(slider).insert(SliderValue(2.0));
        app.update();
        app.update();
        assert_eq!(app.world().resource::<ClientSettings>().ui_scale, 2.0);
        assert_eq!(app.world().resource::<bevy::ui::UiScale>().0, 2.0);
    }

    #[test]
    fn ui_scale_persistence_accepts_the_design_endpoints() {
        for scale in [0.75, 1.0, 1.5, 1.75, 2.0] {
            let controls = crate::client_preferences::ControlSettings {
                ui_scale: scale,
                ..Default::default()
            };
            assert!(controls.valid(), "design8.3 scale {scale} must be valid");
            let encoded = crate::volume_settings::set_controls_in_json(None, &controls).unwrap();
            let loaded = crate::volume_settings::decode_settings_json(&encoded);
            assert!(loaded.notice.is_none());
            assert_eq!(loaded.controls.ui_scale, scale);
        }
        for scale in [0.74, 2.01, f32::NAN, f32::INFINITY] {
            let controls = crate::client_preferences::ControlSettings {
                ui_scale: scale,
                ..Default::default()
            };
            assert!(!controls.valid());
        }
    }

    #[test]
    fn a_stored_frame_limit_reaches_the_debug_state_through_the_applied_window_modes() {
        let loaded =
            crate::volume_settings::decode_settings_json(r#"{"controls":{"frame_limit":20}}"#);
        assert_eq!(loaded.notice, None);
        let mut client = ClientSettings::default();
        loaded.controls.apply(Some(&mut client), None, None);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(client)
            .insert_resource(crate::pacing::default_winit_settings())
            .add_systems(Update, apply_client_settings);
        app.update();
        let debug: serde_json::Value =
            serde_json::from_str(&crate::debug::debug_json(app.world())).unwrap();
        assert_eq!(
            debug["frame_limits"],
            serde_json::json!({"focused": 20, "background": 60}),
            "turts_debug must show the limits that the window loop applies"
        );
    }

    #[test]
    fn the_frame_limit_setting_reaches_the_window_update_modes() {
        use bevy::winit::{UpdateMode, WinitSettings};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(ClientSettings {
                frame_limit: 30,
                ..ClientSettings::default()
            })
            .insert_resource(crate::pacing::default_winit_settings())
            .add_systems(Update, apply_client_settings);
        app.update();
        let modes = app.world().resource::<WinitSettings>();
        assert_eq!(
            modes.update_mode(true),
            UpdateMode::reactive_low_power(std::time::Duration::from_secs_f64(1.0 / 30.0))
        );
        assert_eq!(
            modes.update_mode(false),
            UpdateMode::reactive_low_power(std::time::Duration::from_secs_f64(1.0 / 60.0))
        );
        app.world_mut().resource_mut::<ClientSettings>().frame_limit = 0;
        app.update();
        assert_eq!(
            app.world().resource::<WinitSettings>().update_mode(true),
            UpdateMode::Continuous
        );
    }

    #[test]
    fn player_camera_and_ui_scale_settings_reach_the_existing_runtime_resources() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(ClientSettings {
                scroll_speed: 0.6,
                edge_scroll: false,
                ui_scale: 1.25,
                ..ClientSettings::default()
            })
            .insert_resource(crate::camera::CameraConfiguration::default())
            .insert_resource(bevy::ui::UiScale(1.0))
            .add_systems(Update, apply_client_settings);
        let window = app
            .world_mut()
            .spawn((
                bevy::window::Window {
                    resolution: bevy::window::WindowResolution::new(1280, 720)
                        .with_scale_factor_override(2.0),
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();

        app.update();

        let camera = &app
            .world()
            .resource::<crate::camera::CameraConfiguration>()
            .options;
        assert_eq!(camera.scroll_speed, 0.6);
        assert!(!camera.edge_scroll);
        assert_eq!(
            app.world()
                .get::<bevy::window::Window>(window)
                .expect("the test has an actual primary window")
                .scale_factor(),
            2.0
        );
        assert_eq!(
            app.world().resource::<bevy::ui::UiScale>().0,
            1.25,
            "UiScale uses the player value without a device-pixel multiplier"
        );
    }

    // Baseline contract tests. They use only resources and plugin APIs that already exist.
    // On the pre-screen product source these fail because no product screen nodes are produced.
    mod missing_product_screens_baseline {
        use super::*;
        use crate::{
            commands::CommandSendStatus,
            core_adapter::{CoreEffects, CorePreferences},
            join::Acquisition,
            volume_settings::VolumeSettings,
        };

        fn shell_app(phase: Phase) -> App {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .add_plugins(ClientShellPlugin)
                .insert_resource(phase)
                .init_resource::<Acquisition>()
                .init_resource::<CommandSendStatus>()
                .init_resource::<CoreEffects>()
                .init_resource::<CorePreferences>()
                .insert_resource(VolumeSettings::default());
            app
        }

        fn has_text(app: &mut App, expected: &str) -> bool {
            let world = app.world_mut();
            let mut texts = world.query::<&Text>();
            texts.iter(world).any(|text| text.0 == expected)
        }

        #[test]
        fn load_phase_produces_a_real_load_screen_heading() {
            let mut app = shell_app(Phase::FetchAdvertisement);
            app.update();
            assert!(has_text(&mut app, "Loading game"));
        }

        #[test]
        fn rejected_reason_produces_player_worded_rejection_screen() {
            let mut app = shell_app(Phase::Failed("no open slot".to_owned()));
            app.update();
            assert!(has_text(&mut app, "This game is full"));
        }

        #[test]
        fn reconnect_required_produces_connection_lost_screen() {
            let mut app = shell_app(Phase::Joined { slot: 0 });
            app.world_mut()
                .resource_mut::<CommandSendStatus>()
                .reconnect_required = true;
            app.update();
            assert!(has_text(&mut app, "Connection lost"));
        }
    }

    fn packed_forge_packages() -> AcquiredPackages {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
        let archive = turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&dir).expect("pack forge"),
        )
        .expect("parse forge");
        let mut packages = forge_acquired_packages();
        packages.0.content = ClientContent::from_archives(&[archive]).expect("resolve forge");
        packages
    }

    /// The action labels of the open menu, in the order of the panel rows.
    fn menu_action_labels(app: &mut App) -> Vec<String> {
        let world = app.world_mut();
        let panel = world
            .query_filtered::<Entity, With<bevy::ui_widgets::ScrollArea>>()
            .iter(world)
            .next()
            .expect("the open settings panel scrolls");
        let rows = world.get::<Children>(panel).unwrap().to_vec();
        rows.into_iter()
            .filter_map(|row| {
                let first = *world.get::<Children>(row)?.first()?;
                let node = world.get::<Node>(first)?;
                (node.width == Val::Px(MENU_ACTION_LABEL_WIDTH))
                    .then(|| world.get::<Text>(first).unwrap().0.clone())
            })
            .collect()
    }

    /// Player path step 14 (PT1): a first-time player finds Stop in the menu. A mod action row
    /// takes the name of its order; mod actions come first, then camera and selection, then groups.
    #[test]
    fn menu_rows_name_mod_actions_by_order_and_list_them_before_the_groups() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(packed_forge_packages())
            .add_plugins(ClientShellPlugin);
        app.update();
        let labels = menu_action_labels(&mut app);
        assert_eq!(labels.len(), 51, "{labels:?}");
        assert!(
            labels.contains(&"Stop selected units".to_owned()),
            "{labels:?}"
        );
        assert!(
            !labels.iter().any(|label| label.starts_with("Key ")),
            "{labels:?}"
        );
        let is_group = |label: &String| {
            ["Assign group ", "Select group ", "Add to group "]
                .iter()
                .any(|prefix| label.starts_with(prefix))
        };
        let mut orders = app
            .world()
            .resource::<AcquiredPackages>()
            .0
            .content
            .catalog
            .orders
            .values()
            .map(|order| order.name.clone())
            .collect::<Vec<_>>();
        orders.sort_unstable();
        let mut first = labels[..orders.len()].to_vec();
        first.sort_unstable();
        assert_eq!(first, orders, "mod actions come first: {labels:?}");
        let first_group = labels.iter().position(is_group).unwrap();
        assert!(first_group > orders.len(), "{labels:?}");
        assert!(labels[first_group..].iter().all(is_group), "{labels:?}");
    }

    /// RUN133 H1: the rebind prompt, the conflict text and Swap/Cancel stay in view for a Change
    /// on any row. The feedback block is outside the scrolling rows, above them, with a fixed
    /// height, so the rows do not move when the text shows or clears.
    #[test]
    fn binding_feedback_stays_above_the_scrolling_rows_with_a_fixed_height() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(menu_action())
            .insert_resource(VolumeSettings::default())
            .insert_resource(packed_forge_packages())
            .add_plugins(ClientShellPlugin);
        app.update();
        let world = app.world_mut();
        let scroll = world
            .query_filtered::<Entity, With<bevy::ui_widgets::ScrollArea>>()
            .single(world)
            .expect("one scrolling area");
        let text = world
            .query_filtered::<Entity, With<BindingFeedbackText>>()
            .single(world)
            .expect("one feedback text");
        let block = world.get::<ChildOf>(text).unwrap().parent();
        let mut ancestor = Some(block);
        while let Some(entity) = ancestor {
            assert_ne!(
                entity, scroll,
                "the feedback block must not scroll with the rows"
            );
            ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent);
        }
        let panel = world.get::<ChildOf>(scroll).unwrap().parent();
        let siblings = world.get::<Children>(panel).unwrap().to_vec();
        let block_index = siblings.iter().position(|entity| *entity == block);
        let scroll_index = siblings.iter().position(|entity| *entity == scroll);
        assert!(
            block_index.is_some() && block_index < scroll_index,
            "the feedback block sits above the scrolling rows in the same panel"
        );
        let node = world.get::<Node>(block).unwrap();
        assert_eq!(node.height, Val::Px(BINDING_FEEDBACK_HEIGHT));
        assert_eq!(node.flex_shrink, 0.0);
    }
}
