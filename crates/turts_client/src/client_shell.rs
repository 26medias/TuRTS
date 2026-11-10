//! Shared F10/Escape settings overlay. Task16 composes its additional controls here.

use bevy::prelude::*;

use crate::{
    core_adapter::{CoreAdapterSet, CoreEffects},
    debug_feed::NoticesFeed,
    feedback::FeedbackSet,
    ingest::ReplicaState,
    join::{AcquiredPackages, Phase},
    rejoin::RejoinController,
    ui_shared::{UiCaptureRoot, UiPresentationSet},
    volume_settings::{VolumeSettings, spawn_volume_controls},
};

#[derive(Component)]
struct SettingsMenuOverlay;

#[derive(Component)]
struct RejoinOverlayCamera;

/// Engine-owned text row used only when a joined package has no authored interface.
#[derive(Component)]
pub(crate) struct ClientNotice;

/// Own the one settings overlay reached through the interaction core's existing `menu` action.
pub struct ClientShellPlugin;

impl Plugin for ClientShellPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle_settings_menu
                .after(CoreAdapterSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            sync_client_notice
                .after(FeedbackSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            sync_rejoin_camera
                .after(sync_client_notice)
                .before(UiPresentationSet),
        );
    }
}

fn sync_client_notice(
    state: Option<Res<ReplicaState>>,
    phase: Option<Res<Phase>>,
    acquired: Option<Res<AcquiredPackages>>,
    notices: Option<Res<NoticesFeed>>,
    rejoin: Option<Res<RejoinController>>,
    mut rows: Query<(Entity, &mut Text), With<ClientNotice>>,
    mut commands: Commands,
) {
    let rejoin_text = rejoin.as_deref().and_then(RejoinController::visible_text);
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

fn sync_rejoin_camera(
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

fn toggle_settings_menu(
    effects: Res<CoreEffects>,
    settings: Res<VolumeSettings>,
    overlays: Query<Entity, With<SettingsMenuOverlay>>,
    mut commands: Commands,
) {
    if !effects.is_changed()
        || !effects.0.iter().any(
            |effect| matches!(effect, turts_interaction::Effect::ShellAction { action } if action == "menu"),
        )
    {
        return;
    }

    if let Some(overlay) = overlays.iter().next() {
        commands.entity(overlay).despawn();
        return;
    }

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

    let panel = commands
        .spawn((
            Node {
                width: Val::Px(520.0),
                min_height: Val::Px(300.0),
                padding: UiRect::all(Val::Px(22.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.075, 0.09, 0.12)),
            BorderColor::all(Color::srgb(0.34, 0.43, 0.52)),
        ))
        .id();

    commands.entity(overlay).add_child(panel);
    commands.entity(panel).with_children(|panel| {
        panel.spawn((
            Text::new("Settings"),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        panel.spawn((
            Text::new("Audio levels"),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::srgb(0.78, 0.84, 0.9)),
        ));
    });
    spawn_volume_controls(&mut commands, panel, *settings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::volume_settings::{VolumeCategory, VolumeControl};
    use crate::{
        acquire::{Acquired, PresentationContent},
        content::ClientContent,
        ingest::WelcomeInfo,
    };

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
}
