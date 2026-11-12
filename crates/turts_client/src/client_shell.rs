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

/// Settings that must reach the existing camera and Bevy UI owners.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClientSettings {
    pub scroll_speed: f32,
    pub edge_scroll: bool,
    pub ui_scale: f32,
    pub cursor_lock: bool,
    pub double_click_ms: u64,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            scroll_speed: 1.0,
            edge_scroll: true,
            ui_scale: 1.0,
            cursor_lock: false,
            double_click_ms: 400,
        }
    }
}

/// Apply persisted player settings to the existing camera and UI runtime resources.
pub(crate) fn apply_client_settings(
    settings: Res<ClientSettings>,
    mut camera: Option<ResMut<crate::camera::CameraConfiguration>>,
    mut ui_scale: Option<ResMut<bevy::ui::UiScale>>,
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
pub(crate) mod tests;
