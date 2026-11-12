//! The binding editor of the settings menu: key labels, chord capture, conflict and reset,
//! and the binding rows.

use super::*;

#[derive(Component, Clone, Copy)]
pub(super) enum BindingButtonKind {
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
pub(super) struct BindingInputTimeline {
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
pub(super) struct CaptureModifiers {
    ctrl_left: bool,
    ctrl_right: bool,
    shift_left: bool,
    shift_right: bool,
    alt_left: bool,
    alt_right: bool,
}

impl CaptureModifiers {
    #[cfg(test)]
    pub(super) fn from_keys(keys: Option<&ButtonInput<KeyCode>>) -> Self {
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

pub(super) fn record_binding_input_timeline(
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
pub(super) fn captured_binding(
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

#[derive(Component)]
pub(super) struct BindingFeedbackText;

#[derive(Component)]
pub(super) struct BindingConflictPrompt;

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

pub(super) fn spawn_binding_button(
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

pub(super) fn on_binding_pointer_press(
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

pub(super) fn on_binding_pointer_click(
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

pub(super) fn on_binding_keyboard_activation(
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

pub(super) fn on_binding_button_activated(
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

pub(super) fn capture_physical_binding(
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

pub(super) fn menu_binding_list_label(bindings: &[Binding], labels: &BindingLabels) -> String {
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

pub(super) fn sync_binding_value_labels(
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

pub(super) fn sync_binding_feedback(
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
