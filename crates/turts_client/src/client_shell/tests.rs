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
    ron::from_str(include_str!("../../../../mods/forge/input/bindings.ron"))
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
        ron::from_str(include_str!("../../../../mods/muster/interface/hud.ron"))
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

/// A shell app with a focused canvas and a fixed frame step of 250 ms. The sound switch is on,
/// so these tests keep their meaning: they test the browser audio state, not the switch.
fn start_gate_app(audio: crate::debug_feed::AudioFeed) -> App {
    start_gate_app_with(
        audio,
        VolumeSettings {
            sound_on: true,
            ..default()
        },
    )
}

fn start_gate_app_with(audio: crate::debug_feed::AudioFeed, volumes: VolumeSettings) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(CoreEffects::default())
        .insert_resource(volumes)
        .insert_resource(Phase::Joined { slot: 0 })
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

    let result = edit_binding_override(&mut overrides, &defaults, "key_move", key("KeyS"), None);

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
    use turts_interaction::{Core, Effect, InputEvent, NodeView, UnitView, Viewport, WorldView};
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
        ron::from_str::<OrderDef>(include_str!("../../../../mods/forge/orders/order_stop.ron"))
            .unwrap(),
        ron::from_str::<OrderDef>(include_str!("../../../../mods/forge/orders/order_move.ron"))
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
        rules: ron::from_str(include_str!("../../../../mods/forge/orders/rules.ron")).unwrap(),
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
            .expect("both double-click time and distance must be player-editable in the real menu");
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
    let controls =
        crate::client_preferences::ControlSettings::capture(None, None, Some(preferences), None);
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

fn switches(app: &mut App) -> Vec<(Entity, String)> {
    let world = app.world_mut();
    let found = world
        .query_filtered::<(Entity, &Children), With<crate::volume_settings::SoundSwitch>>()
        .iter(world)
        .map(|(entity, children)| (entity, children.to_vec()))
        .collect::<Vec<_>>();
    found
        .into_iter()
        .map(|(entity, children)| {
            let text = children
                .iter()
                .find_map(|child| world.get::<Text>(*child).map(|text| text.0.clone()))
                .expect("each sound switch holds its state text");
            (entity, text)
        })
        .collect()
}

fn click_switch(app: &mut App, entity: Entity) {
    app.world_mut().trigger(Activate { entity });
    app.world_mut().flush();
    app.update();
}

fn running_audio() -> crate::debug_feed::AudioFeed {
    crate::debug_feed::AudioFeed {
        context_states: vec!["running".into()],
        gesture_seen: true,
        ..default()
    }
}

/// CEO decision: the game makes no sound by default, and the player turns it on with one visible
/// control that says its state in words.
#[test]
fn the_play_screen_has_one_sound_switch_that_says_off_by_default_and_one_click_turns_it_on() {
    let mut app = start_gate_app_with(running_audio(), VolumeSettings::default());
    app.update();
    let found = switches(&mut app);
    assert_eq!(found.len(), 1, "one switch on the play screen");
    assert_eq!(found[0].1, "Sound: off", "the default says off, in words");
    let switch = found[0].0;
    assert!(
        app.world().entity(switch).contains::<UiCaptureRoot>(),
        "a click on the switch does not reach the world"
    );

    click_switch(&mut app, switch);
    assert!(app.world().resource::<VolumeSettings>().sound_on);
    assert_eq!(switches(&mut app)[0].1, "Sound: on");

    click_switch(&mut app, switch);
    assert!(!app.world().resource::<VolumeSettings>().sound_on);
    assert_eq!(switches(&mut app)[0].1, "Sound: off");
}

#[test]
fn the_switch_keeps_the_four_levels_across_off_and_on() {
    let levels = VolumeSettings {
        sound_on: false,
        master: 0.35,
        voice: 0.2,
        alerts: 0.6,
        effects: 0.8,
    };
    let mut app = start_gate_app_with(running_audio(), levels);
    app.update();
    let switch = switches(&mut app)[0].0;
    click_switch(&mut app, switch);
    click_switch(&mut app, switch);
    click_switch(&mut app, switch);
    let now = *app.world().resource::<VolumeSettings>();
    assert!(now.sound_on);
    assert_eq!(
        VolumeSettings {
            sound_on: false,
            ..now
        },
        VolumeSettings {
            sound_on: false,
            ..levels
        },
        "the sound switch never writes a level"
    );
}

#[test]
fn a_switch_that_is_on_still_says_off_while_the_browser_keeps_audio_locked() {
    let locked = crate::debug_feed::AudioFeed {
        context_states: vec!["suspended".into()],
        gesture_seen: true,
        ..default()
    };
    let mut app = start_gate_app(locked);
    app.update();
    assert_eq!(switches(&mut app)[0].1, "Sound: off");
    app.world_mut()
        .resource_mut::<crate::debug_feed::AudioFeed>()
        .context_states = vec!["running".into()];
    app.update();
    assert_eq!(switches(&mut app)[0].1, "Sound: on");
}

#[test]
fn no_sound_notice_shows_while_the_player_keeps_the_sound_off() {
    let locked = crate::debug_feed::AudioFeed {
        context_states: vec!["suspended".into()],
        gesture_seen: true,
        ..default()
    };
    let mut app = start_gate_app_with(locked, VolumeSettings::default());
    for _ in 0..48 {
        app.update();
        assert!(
            !shows_text(&mut app, SOUND_OFF_NOTICE),
            "the sound is off on purpose; the notice \"click to turn it on\" would be wrong"
        );
    }
}

#[test]
fn the_settings_menu_holds_the_same_switch_beside_the_volume_sliders() {
    let mut app = start_gate_app_with(running_audio(), VolumeSettings::default());
    app.update();
    *app.world_mut().resource_mut::<CoreEffects>() = menu_action();
    app.update();
    app.world_mut().resource_mut::<CoreEffects>().0.clear();
    let found = switches(&mut app);
    assert_eq!(found.len(), 2, "one on the play screen and one in the menu");
    assert!(found.iter().all(|(_, text)| text == "Sound: off"));
    let in_menu = {
        let world = app.world_mut();
        let overlay = world
            .query_filtered::<Entity, With<SettingsMenuOverlay>>()
            .single(world)
            .unwrap();
        found
            .iter()
            .map(|(entity, _)| *entity)
            .find(|entity| {
                let mut at = Some(*entity);
                while let Some(current) = at {
                    if current == overlay {
                        return true;
                    }
                    at = world.get::<ChildOf>(current).map(|parent| parent.parent());
                }
                false
            })
            .expect("one switch sits inside the settings overlay")
    };
    click_switch(&mut app, in_menu);
    assert!(app.world().resource::<VolumeSettings>().sound_on);
    assert!(
        switches(&mut app)
            .iter()
            .all(|(_, text)| text == "Sound: on"),
        "both switches show the one state"
    );
}

#[test]
fn the_play_screen_switch_waits_for_the_join() {
    let mut app = start_gate_app_with(running_audio(), VolumeSettings::default());
    *app.world_mut().resource_mut::<Phase>() = Phase::Joining;
    app.update();
    assert!(switches(&mut app).is_empty(), "no switch before the join");
    *app.world_mut().resource_mut::<Phase>() = Phase::Joined { slot: 0 };
    app.update();
    assert_eq!(switches(&mut app).len(), 1);
}

#[test]
fn the_debug_ui_snapshot_lists_the_sound_switch_with_its_words() {
    let mut app = start_gate_app_with(running_audio(), VolumeSettings::default());
    app.update();
    let rows = crate::widgets::snapshot::snapshot(app.world());
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "sound_switch")
        .expect("the ui snapshot has one sound_switch row");
    assert_eq!(row["text"], "Sound: off");
    assert_eq!(row["capture_root"], true);
    let switch = switches(&mut app)[0].0;
    click_switch(&mut app, switch);
    let rows = crate::widgets::snapshot::snapshot(app.world());
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "sound_switch")
        .unwrap();
    assert_eq!(row["text"], "Sound: on");
}

fn packages_with_interface(ron_text: &str) -> AcquiredPackages {
    let mut packages = forge_acquired_packages();
    packages.0.presentation.interface =
        Some(ron::from_str(ron_text).expect("the interface fixture parses"));
    packages
}

/// A stand-in for a widget of the mod: a button with an action. `visible` is what the widget tree
/// gives a widget that its `visible` list shows or hides (`InheritedVisibility`).
fn mod_button(app: &mut App, action: &str, visible: bool) -> Entity {
    app.world_mut()
        .spawn((
            crate::widgets::primitives::WidgetButtonAction(
                crate::widgets::primitives::WidgetAction::Input {
                    name: action.into(),
                },
            ),
            if visible {
                InheritedVisibility::VISIBLE
            } else {
                InheritedVisibility::HIDDEN
            },
        ))
        .id()
}

/// D67 (Decided): the play screen is the mod's. Re-check of pass 3, M2 (ruling (c)): a mod switch that
/// can never be visible (the `[GuideOpen, Not(GuideOpen)]` of the reviewer) must not turn off the
/// engine switch. The honest rule, in the code: the engine draws its own sound switch in each frame
/// in which no button with the action `toggle_sound` is visible. A static check of the `visible`
/// lists is not complete, so the rule does not use it.
#[test]
fn the_engine_draws_its_switch_in_each_frame_in_which_no_mod_switch_is_visible() {
    // The interface places a switch (muster), but the widget is hidden: the engine switch shows.
    let mut app = start_gate_app_with(running_audio(), VolumeSettings::default());
    app.insert_resource(packages_with_interface(include_str!(
        "../../../../mods/muster/interface/hud.ron"
    )));
    let hidden = mod_button(&mut app, "toggle_sound", false);
    app.update();
    assert_eq!(
        switches(&mut app).len(),
        1,
        "a switch that is never visible leaves the player the engine switch"
    );
    // The mod switch becomes visible: the engine switch goes.
    app.world_mut()
        .entity_mut(hidden)
        .insert(InheritedVisibility::VISIBLE);
    app.update();
    assert!(
        switches(&mut app).is_empty(),
        "a visible mod switch: the engine draws none"
    );
    // And hidden again: the engine switch comes back in the next frame.
    app.world_mut()
        .entity_mut(hidden)
        .insert(InheritedVisibility::HIDDEN);
    app.update();
    assert_eq!(switches(&mut app).len(), 1);
}

/// The two widgets of a package (`SoundOn` and `Not(SoundOn)`): one is visible, one hidden: no engine
/// switch. A visible button with another action does not count. No widget at all: the engine switch.
#[test]
fn a_visible_mod_switch_of_a_pair_hides_the_engine_switch_and_another_action_does_not() {
    let mut pair = start_gate_app_with(running_audio(), VolumeSettings::default());
    mod_button(&mut pair, "toggle_sound", false);
    mod_button(&mut pair, "toggle_sound", true);
    pair.update();
    assert!(switches(&mut pair).is_empty());

    let mut other = start_gate_app_with(running_audio(), VolumeSettings::default());
    mod_button(&mut other, "toggle_guide", true);
    other.update();
    assert_eq!(
        switches(&mut other).len(),
        1,
        "a visible button with another action"
    );

    let mut none = start_gate_app_with(running_audio(), VolumeSettings::default());
    none.update();
    assert_eq!(switches(&mut none).len(), 1, "no widget at all");
}

/// The six shipped packages place the switch in their layouts (the layout is the mod's), and with its
/// widget visible the engine draws none.
#[test]
fn the_six_shipped_layouts_place_the_switch_and_a_visible_one_hides_the_engine_switch() {
    for package in ["muster", "forge", "base", "cedar", "haul", "acme"] {
        let text = std::fs::read_to_string(format!(
            "{}/../../mods/{package}/interface/hud.ron",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert!(
            text.contains(r#"Input("toggle_sound")"#),
            "{package} places the switch"
        );
        let mut app = start_gate_app_with(running_audio(), VolumeSettings::default());
        mod_button(&mut app, "toggle_sound", true);
        app.update();
        assert!(
            switches(&mut app).is_empty(),
            "{package}: its own switch is visible"
        );
    }
}
