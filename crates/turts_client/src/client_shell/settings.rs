//! The settings menu: the player settings, the menu overlay, its toggles and sliders, and the
//! pointer lock.

use super::*;

#[derive(Component, Clone, Copy)]
pub(super) enum SettingsToggle {
    EdgeScroll,
    CursorLock,
    HealthBars,
}

#[derive(Component, Clone, Copy)]
struct SettingsToggleText;

#[derive(Component, Clone, Copy)]
pub(super) struct SettingsToggleState(SettingsToggle);

#[derive(Component, Clone, Copy)]
pub(super) enum SettingsSlider {
    ScrollSpeed,
    UiScale,
    DoubleClick,
    DoubleClickPixels,
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

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct SettingsMenuSources<'w> {
    effects: Res<'w, CoreEffects>,
    volumes: Res<'w, VolumeSettings>,
    settings: Res<'w, ClientSettings>,
    health: Res<'w, crate::visuals::HealthDisplay>,
    acquired: Option<Res<'w, AcquiredPackages>>,
    preferences: Res<'w, CorePreferences>,
    labels: Res<'w, BindingLabels>,
    audio: Option<Res<'w, crate::debug_feed::AudioFeed>>,
}

pub(super) fn toggle_settings_menu(
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

pub(super) fn on_settings_toggle_activated(
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

pub(super) fn update_pointer_lock(
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

pub(super) fn sync_settings_toggle_labels(
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

pub(super) fn apply_client_setting_sliders(
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

pub(super) fn sync_interaction_settings(
    settings: Res<ClientSettings>,
    mut preferences: ResMut<CorePreferences>,
) {
    if settings.is_changed() {
        preferences.interaction.double_click_ms = settings.double_click_ms;
    }
}
