//! Engine-owned load, rejection, and connection-loss screens.

use crate::{
    commands::CommandSendStatus,
    join::{AcquiredPackages, Acquisition, Phase},
    rejoin::RejoinController,
    ui_shared::{UiCaptureRoot, UiPresentationSet},
    widgets::primitives::AuthoredWidgetId,
};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button as UiButton};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum ProductScreenKind {
    Load,
    Rejection,
    ConnectionLost,
}

#[derive(Resource, Default)]
struct ProductScreenCamera(Option<Entity>);

#[derive(Component)]
struct ProductScreenCameraMarker;

#[derive(Component)]
pub(crate) struct ReloadButton;

#[derive(Component, Clone, Copy)]
enum ScreenTextRole {
    Heading,
    Detail,
    Package,
}

#[derive(Component)]
struct ProgressFill;

#[derive(Component)]
struct ReloadFailureText;

#[derive(Resource, Default, Debug, Clone)]
pub(crate) struct ReloadStatus(pub(crate) Option<String>);

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReloadRequested;

pub(crate) struct ClientScreensPlugin;

impl Plugin for ClientScreensPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        app.init_resource::<ReloadStatus>()
            .init_resource::<ProductScreenCamera>()
            .add_message::<ReloadRequested>()
            .add_systems(PreStartup, spawn_product_screen_camera)
            .add_systems(
                Update,
                sync_product_screen
                    .after(crate::client_shell::sync_client_notice)
                    .after(crate::client_shell::sync_rejoin_camera)
                    .before(UiPresentationSet),
            )
            .add_observer(request_reload);
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, reload_browser_on_request.after(UiPresentationSet));
    }
}

fn spawn_product_screen_camera(
    mut commands: Commands,
    mut product_camera: ResMut<ProductScreenCamera>,
) {
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 100,
                is_active: false,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ProductScreenCameraMarker,
        ))
        .id();
    product_camera.0 = Some(camera);
}

#[derive(bevy::ecs::system::SystemParam)]
struct ProductScreenSources<'w> {
    phase: Option<Res<'w, Phase>>,
    acquisition: Option<Res<'w, Acquisition>>,
    acquired: Option<Res<'w, AcquiredPackages>>,
    send_status: Option<Res<'w, CommandSendStatus>>,
    rejoin: Option<Res<'w, RejoinController>>,
    reload_status: Res<'w, ReloadStatus>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct ProductScreenViews<'w, 's> {
    product_camera: ResMut<'w, ProductScreenCamera>,
    rejoin_cameras: Query<'w, 's, Entity, With<crate::client_shell::RejoinOverlayCamera>>,
    screens: Query<'w, 's, (Entity, &'static ProductScreenKind, &'static UiTargetCamera)>,
    screen_texts: Query<
        'w,
        's,
        (
            &'static ScreenTextRole,
            &'static mut Text,
            &'static mut Visibility,
        ),
        Without<ReloadFailureText>,
    >,
    progress_fills: Query<'w, 's, &'static mut Node, With<ProgressFill>>,
    failure_texts:
        Query<'w, 's, (&'static mut Text, &'static mut Visibility), With<ReloadFailureText>>,
    cameras: Query<'w, 's, &'static mut Camera>,
}

fn sync_product_screen(
    sources: ProductScreenSources,
    views: ProductScreenViews,
    mut commands: Commands,
) {
    let ProductScreenSources {
        phase,
        acquisition,
        acquired,
        send_status,
        rejoin,
        reload_status,
    } = sources;
    let ProductScreenViews {
        mut product_camera,
        rejoin_cameras,
        screens,
        screen_texts,
        progress_fills,
        failure_texts,
        mut cameras,
    } = views;
    let rejoin_visible = rejoin
        .as_deref()
        .is_some_and(|rejoin| rejoin.visible_text().is_some());
    let rejoin_owns_guidance = rejoin
        .as_deref()
        .is_some_and(RejoinController::owns_rejoin_guidance);
    let mut kind = if send_status
        .as_deref()
        .is_some_and(|status| status.reconnect_required)
    {
        Some(ProductScreenKind::ConnectionLost)
    } else {
        match phase.as_deref() {
            Some(Phase::Failed(_)) => Some(ProductScreenKind::Rejection),
            Some(Phase::Joined { .. }) | None => None,
            Some(_) => Some(ProductScreenKind::Load),
        }
    };
    if rejoin_owns_guidance
        && matches!(
            kind,
            Some(ProductScreenKind::Load | ProductScreenKind::Rejection)
        )
    {
        kind = None;
    }

    let rejoin_camera = if rejoin_visible && kind.is_some() {
        rejoin_cameras.iter().next()
    } else {
        None
    };
    if rejoin_visible && kind.is_some() && rejoin_camera.is_none() {
        return;
    }
    let using_rejoin_camera = rejoin_camera.is_some();
    let camera = if let Some(camera) = rejoin_camera {
        if let Some(entity) = product_camera.0.take() {
            commands.entity(entity).despawn();
        }
        camera
    } else if kind.is_some() {
        ensure_product_camera(&mut commands, &mut product_camera)
    } else {
        if let Some(entity) = product_camera.0.take() {
            commands.entity(entity).despawn();
        }
        for (entity, _, _) in &screens {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut has_stale_screen = false;
    for (entity, existing_kind, target_camera) in &screens {
        if Some(*existing_kind) != kind || target_camera.0 != camera {
            has_stale_screen = true;
            commands.entity(entity).despawn();
        }
    }
    if has_stale_screen {
        if !using_rejoin_camera {
            set_screen_camera(camera, &mut cameras, false);
        }
        return;
    }
    let Some(kind) = kind else {
        set_screen_camera(camera, &mut cameras, false);
        return;
    };
    let screen_copy = match kind {
        ProductScreenKind::Load => load_copy(
            phase.as_deref().expect("a load screen has a join phase"),
            acquisition.as_deref(),
            acquired.as_deref(),
        ),
        ProductScreenKind::Rejection => rejection_copy(phase.as_deref()),
        ProductScreenKind::ConnectionLost => ScreenCopy {
            heading: "Connection lost".to_owned(),
            detail: "Reload the client to reconnect to the game.".to_owned(),
            package: None,
            progress: None,
            action: Some("Reload"),
        },
    };
    if screens.iter().any(|(_, existing, _)| *existing == kind) {
        update_screen_content(kind, &screen_copy, screen_texts, progress_fills);
        for (mut text, mut visibility) in failure_texts {
            if let Some(message) = reload_status.0.as_deref() {
                if text.0 != message {
                    *text = Text::new(message);
                }
                *visibility = Visibility::Inherited;
            }
        }
        set_screen_camera(camera, &mut cameras, true);
        return;
    }

    set_screen_camera(camera, &mut cameras, true);
    spawn_screen(
        &mut commands,
        camera,
        kind,
        screen_copy,
        reload_status.0.as_deref(),
    );
}

fn update_screen_content(
    kind: ProductScreenKind,
    copy: &ScreenCopy,
    mut texts: Query<(&ScreenTextRole, &mut Text, &mut Visibility), Without<ReloadFailureText>>,
    mut progress_fills: Query<&mut Node, With<ProgressFill>>,
) {
    for (role, mut text, mut visibility) in &mut texts {
        let value = match role {
            ScreenTextRole::Heading => Some(copy.heading.as_str()),
            ScreenTextRole::Detail => Some(copy.detail.as_str()),
            ScreenTextRole::Package => copy.package.as_deref(),
        };
        if let Some(value) = value {
            if text.0 != value {
                *text = Text::new(value);
            }
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    if kind == ProductScreenKind::Load
        && let Some(progress) = copy.progress
    {
        for mut fill in &mut progress_fills {
            fill.width = Val::Percent(progress * 100.0);
        }
    }
}

fn ensure_product_camera(
    commands: &mut Commands,
    product_camera: &mut ResMut<ProductScreenCamera>,
) -> Entity {
    if let Some(camera) = product_camera.0 {
        return camera;
    }
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 100,
                is_active: false,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ProductScreenCameraMarker,
        ))
        .id();
    product_camera.0 = Some(camera);
    camera
}

fn set_screen_camera(camera: Entity, cameras: &mut Query<&mut Camera>, active: bool) {
    if let Ok(mut camera) = cameras.get_mut(camera) {
        camera.is_active = active;
    }
}

struct ScreenCopy {
    heading: String,
    detail: String,
    package: Option<String>,
    progress: Option<f32>,
    action: Option<&'static str>,
}

fn load_copy(
    phase: &Phase,
    acquisition: Option<&Acquisition>,
    acquired: Option<&AcquiredPackages>,
) -> ScreenCopy {
    let (detail, progress) = match phase {
        Phase::FetchAdvertisement => ("Fetching server details".to_owned(), Some(0.0)),
        Phase::FetchLockfile => (
            "Fetching and checking the package list".to_owned(),
            Some(0.0),
        ),
        Phase::FetchPackages { left } => {
            let (verified, total) = acquisition.map_or((0, *left), Acquisition::package_progress);
            let detail = acquisition
                .and_then(Acquisition::current_package)
                .map(|_| "Downloading package".to_owned())
                .unwrap_or_else(|| "Downloading a server package".to_owned());
            (
                format!("{detail} ({verified} of {total} packages verified)"),
                ratio(verified, total),
            )
        }
        Phase::VerifyPackage { left } => {
            let (verified, total) = acquisition.map_or((0, *left), Acquisition::package_progress);
            let detail = acquisition
                .and_then(Acquisition::current_package)
                .map(|_| "Verifying package".to_owned())
                .unwrap_or_else(|| "Verifying a server package".to_owned());
            (
                format!("{detail} ({verified} of {total} packages verified)"),
                ratio(verified, total),
            )
        }
        Phase::Connecting | Phase::Joining => (
            "Joining the game".to_owned(),
            acquisition.and_then(|value| {
                let (verified, total) = value.package_progress();
                ratio(verified, total)
            }),
        ),
        Phase::Joined { .. } | Phase::Failed(_) => (String::new(), None),
    };
    ScreenCopy {
        heading: "Loading game".to_owned(),
        detail,
        package: acquisition
            .and_then(Acquisition::current_package)
            .map(|entry| format!("{} {}", entry.id, entry.version))
            .or_else(|| {
                acquired
                    .and_then(|packages| packages.0.verified_root_mod.as_ref())
                    .map(|root| format!("{} {}", root.id, root.version))
            }),
        progress,
        action: None,
    }
}

fn ratio(done: usize, total: usize) -> Option<f32> {
    if total == 0 {
        Some(1.0)
    } else {
        Some((done as f32 / total as f32).clamp(0.0, 1.0))
    }
}

pub(crate) fn file_mismatch_reason(reason: &str) -> bool {
    reason == "lockfile hash mismatch" || reason.starts_with("lockfile hash mismatch:")
}

pub(crate) fn player_rejection_owns_reason(reason: &str) -> bool {
    file_mismatch_reason(reason)
        || reason == "protocol version mismatch"
        || reason == crate::rejoin::DUPLICATE_PLAYER_REASON
}

fn rejection_copy(phase: Option<&Phase>) -> ScreenCopy {
    let rejected_reason = match phase {
        Some(Phase::Failed(reason)) => reason.as_str(),
        _ => "The client could not join the game.",
    };
    let (heading, detail) = player_rejection(rejected_reason);
    ScreenCopy {
        heading,
        detail,
        package: (!player_rejection_owns_reason(rejected_reason))
            .then(|| format!("Reason: {rejected_reason}")),
        progress: None,
        action: Some("Reload"),
    }
}

fn player_rejection(reason: &str) -> (String, String) {
    match reason {
        "protocol version mismatch" => (
            "Client and server versions do not match".to_owned(),
            "Reload the client to get the latest version. If this continues, ask the server owner to update the game."
                .to_owned(),
        ),
        reason if file_mismatch_reason(reason) => (
            "Game files do not match the server".to_owned(),
            "Reload to fetch the game files again. If this continues, ask the server owner to restart the game."
                .to_owned(),
        ),
        "no open slot" => (
            "This game is full".to_owned(),
            "Wait for a player to leave, then reload this page.".to_owned(),
        ),
        crate::rejoin::REJOIN_FINAL_TEXT | crate::rejoin::DUPLICATE_PLAYER_REASON => (
            "This game is open in another window".to_owned(),
            "Close the other window, then reload this page.".to_owned(),
        ),
        _ => (
            "The client could not join the game".to_owned(),
            "Check the server address and your connection. If the problem continues, ask the server owner for help, then reload this page."
                .to_owned(),
        )
    }
}

fn spawn_screen(
    commands: &mut Commands,
    camera: Entity,
    kind: ProductScreenKind,
    copy: ScreenCopy,
    reload_failure: Option<&str>,
) {
    commands
        .spawn((
            kind,
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
            ZIndex(i32::MAX - 2),
            bevy::ui::FocusPolicy::Block,
            AuthoredWidgetId(format!("shell.{}.root", screen_id(kind))),
            UiTargetCamera(camera),
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    Node {
                        width: Val::Px(520.0),
                        max_width: Val::Percent(90.0),
                        padding: UiRect::all(Val::Px(28.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(14.0),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.075, 0.09, 0.12)),
                    BorderColor::all(Color::srgb(0.28, 0.38, 0.48)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        ScreenTextRole::Heading,
                        Text::new(copy.heading),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        AuthoredWidgetId(format!("shell.{}.heading", screen_id(kind))),
                    ));
                    panel.spawn((
                        ScreenTextRole::Detail,
                        Text::new(copy.detail),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.82, 0.86, 0.91)),
                        AuthoredWidgetId(format!("shell.{}.detail", screen_id(kind))),
                    ));
                    panel.spawn((
                        ScreenTextRole::Package,
                        Text::new(copy.package.clone().unwrap_or_default()),
                        if copy.package.is_some() {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        },
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.75, 0.82)),
                        AuthoredWidgetId(format!("shell.{}.package", screen_id(kind))),
                    ));
                    if let Some(progress) = copy.progress {
                        panel
                            .spawn((
                                Node {
                                    width: Val::Px(360.0),
                                    max_width: Val::Percent(100.0),
                                    height: Val::Px(14.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.18, 0.22, 0.28)),
                                AuthoredWidgetId("shell.load.progress.track".to_owned()),
                            ))
                            .with_children(|track| {
                                track.spawn((
                                    ProgressFill,
                                    Node {
                                        width: Val::Percent(progress * 100.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.24, 0.68, 0.88)),
                                    AuthoredWidgetId("shell.load.progress.fill".to_owned()),
                                ));
                            });
                    }
                    if let Some(action) = copy.action {
                        panel.spawn((
                            ReloadFailureText,
                            Text::new(reload_failure.unwrap_or("")),
                            if reload_failure.is_some() {
                                Visibility::Visible
                            } else {
                                Visibility::Hidden
                            },
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.72, 0.56)),
                            AuthoredWidgetId("shell.reload.failure".to_owned()),
                        ));
                        panel
                            .spawn((
                                UiButton,
                                ReloadButton,
                                Node {
                                    min_width: Val::Px(150.0),
                                    min_height: Val::Px(42.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.16, 0.42, 0.58)),
                                AuthoredWidgetId(format!("shell.{}.reload", screen_id(kind))),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(action),
                                    TextFont {
                                        font_size: FontSize::Px(16.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                    }
                });
        });
}

fn screen_id(kind: ProductScreenKind) -> &'static str {
    match kind {
        ProductScreenKind::Load => "load",
        ProductScreenKind::Rejection => "rejection",
        ProductScreenKind::ConnectionLost => "connection_lost",
    }
}

fn request_reload(
    trigger: On<Activate>,
    buttons: Query<(), With<ReloadButton>>,
    mut requested: MessageWriter<ReloadRequested>,
) {
    if buttons.get(trigger.entity).is_ok() {
        requested.write(ReloadRequested);
    }
}

#[cfg(target_arch = "wasm32")]
fn reload_browser_on_request(
    mut requests: MessageReader<ReloadRequested>,
    mut status: ResMut<ReloadStatus>,
) {
    if requests.read().next().is_none() {
        return;
    }
    let result = web_sys::window()
        .ok_or_else(|| "browser window is unavailable".to_owned())
        .and_then(|window| {
            window
                .location()
                .reload()
                .map_err(|error| format!("{error:?}"))
        });
    status.0 = result
        .err()
        .map(|error| format!("The page could not reload: {error}. Reload it from your browser."));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_native_reload(app: &mut App) {
    app.add_plugins(NativeReloadPlugin);
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeReloadPlugin;

#[cfg(not(target_arch = "wasm32"))]
impl Plugin for NativeReloadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, relaunch_native_on_request.after(UiPresentationSet));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn relaunch_native_on_request(
    mut requests: MessageReader<ReloadRequested>,
    player_id: Res<crate::PlayerId>,
    mut status: ResMut<ReloadStatus>,
    mut exit: MessageWriter<AppExit>,
) {
    if requests.read().next().is_none() {
        return;
    }
    let result = std::env::current_exe()
        .map_err(|error| format!("cannot locate the client executable: {error}"))
        .and_then(|executable| {
            let arguments = native_relaunch_args(std::env::args_os().skip(1), player_id.0);
            std::process::Command::new(executable)
                .args(arguments)
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("cannot restart the client: {error}"))
        });
    match result {
        Ok(()) => {
            exit.write(AppExit::Success);
        }
        Err(error) => {
            status.0 = Some(format!(
                "The client could not restart ({error}). Keep this window open and try again."
            ));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_relaunch_args(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
    player_id: u64,
) -> Vec<std::ffi::OsString> {
    let mut args = arguments.into_iter().peekable();
    let mut kept = Vec::new();
    while let Some(argument) = args.next() {
        if argument == "--player-id" {
            let _ = args.next();
        } else if argument.to_string_lossy().starts_with("--player-id=") {
            continue;
        } else {
            kept.push(argument);
        }
    }
    kept.push("--player-id".into());
    kept.push(player_id.to_string().into());
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{debug_feed::UiSnapshotFeed, join::Phase};

    fn app_with_phase(phase: Phase) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(crate::client_shell::ClientShellPlugin)
            .insert_resource(phase)
            .init_resource::<Acquisition>()
            .init_resource::<CommandSendStatus>()
            .init_resource::<UiSnapshotFeed>()
            .init_resource::<crate::core_adapter::CoreEffects>()
            .init_resource::<crate::core_adapter::CorePreferences>()
            .insert_resource(crate::volume_settings::VolumeSettings::default());
        app
    }

    #[test]
    fn fetch_phase_creates_actual_load_text_and_progress_nodes() {
        let mut app = app_with_phase(Phase::FetchAdvertisement);
        app.update();
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        let content = texts
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(content.contains(&"Loading game"));
        assert!(content.contains(&"Fetching server details"));
        let mut progress = world.query::<(&AuthoredWidgetId, &Node)>();
        assert!(
            progress
                .iter(world)
                .any(|(id, _)| id.0 == "shell.load.progress.fill")
        );
        *app.world_mut().resource_mut::<Phase>() = Phase::FetchLockfile;
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        let content = texts
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(content.contains(&"Fetching and checking the package list"));
        assert!(!content.contains(&"Fetching server details"));
    }

    #[test]
    fn stored_identity_full_game_refusal_keeps_player_guidance_and_reload() {
        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt_at(9, 0.0));
        assert!(matches!(
            rejoin.on_rejection(9, "no open slot".into()),
            crate::rejoin::RejectionDisposition::Terminal(_)
        ));
        let mut app = app_with_phase(Phase::Failed("no open slot".into()));
        app.insert_resource(rejoin);
        app.update();
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        assert!(
            texts.iter(world).any(|text| text.0 == "This game is full"),
            "a retained identity rejected from a full match must keep the mapped player guidance"
        );
        let mut reload = world.query_filtered::<Entity, With<ReloadButton>>();
        assert_eq!(
            reload.iter(world).count(),
            1,
            "the same refusal must retain its real Reload action"
        );
    }

    #[test]
    fn changed_server_files_explain_the_actual_rejection_in_player_words() {
        let reason = "lockfile hash mismatch: package hash sha256:actual does not match the expected hash sha256:expected";
        let mut app = app_with_phase(Phase::Failed(reason.into()));
        app.update();
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        let content = texts
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(
            content.contains(&"Game files do not match the server"),
            "the actual hash rejection must explain which game data failed in player words"
        );
        assert!(
            content
                .iter()
                .any(|text| text.starts_with("Reload to fetch the game files again.")),
            "the actual rejection must offer the relevant next step"
        );
        assert!(
            !content.iter().any(|text| text.contains("sha256:")),
            "raw checksum diagnostics stay out of the player guidance"
        );
        let mut reload = world.query_filtered::<Entity, With<ReloadButton>>();
        assert_eq!(reload.iter(world).count(), 1);
    }

    #[test]
    fn terminal_hash_rejection_keeps_raw_diagnostics_out_of_both_screen_owners() {
        for reason in [
            "lockfile hash mismatch: package hash sha256:actual does not match the expected hash sha256:expected",
            "lockfile hash mismatch",
        ] {
            let mut rejoin = RejoinController::for_stored_identity();
            assert!(rejoin.start_attempt_at(9, 0.0));
            assert!(rejoin.on_connected(9));
            assert!(matches!(
                rejoin.on_rejection(9, reason.into()),
                crate::rejoin::RejectionDisposition::Terminal(_)
            ));
            let mut app = app_with_phase(Phase::Failed(reason.into()));
            app.insert_resource(rejoin);
            app.update();
            app.update();
            let world = app.world_mut();
            let mut texts = world.query::<&Text>();
            let content = texts
                .iter(world)
                .map(|text| text.0.as_str())
                .collect::<Vec<_>>();
            assert!(content.contains(&"Game files do not match the server"));
            assert!(
                !content.iter().any(|text| text.contains("sha256:") || text.contains("lockfile hash mismatch")),
                "a terminal rejoin notice must not repeat the raw checksum over the player-word screen"
            );
        }
    }

    #[test]
    fn protocol_rejection_explains_version_mismatch_in_both_screen_owners() {
        let reason = "protocol version mismatch";
        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt_at(9, 0.0));
        assert!(rejoin.on_connected(9));
        assert!(matches!(
            rejoin.on_rejection(9, reason.into()),
            crate::rejoin::RejectionDisposition::Terminal(_)
        ));
        let mut app = app_with_phase(Phase::Failed(reason.into()));
        app.insert_resource(rejoin);
        app.update();
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        let content = texts
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(
            content.contains(&"Client and server versions do not match"),
            "the actual server version refusal needs player words"
        );
        assert!(
            content
                .iter()
                .any(|text| text.starts_with("Reload the client to get the latest version."))
        );
        assert!(
            !content
                .iter()
                .any(|text| text.contains("protocol version mismatch")),
            "raw wire terminology must not repeat over the player-word rejection"
        );
    }

    #[test]
    fn task16_menu_maps_immediate_duplicate_refusal_to_close_and_reload() {
        let reason = crate::rejoin::DUPLICATE_PLAYER_REASON;
        let mut rejoin = RejoinController::default();
        assert!(rejoin.start_attempt_at(9, 0.0));
        assert!(rejoin.on_connected(9));
        assert!(matches!(
            rejoin.on_rejection(9, reason.into()),
            crate::rejoin::RejectionDisposition::Terminal(_)
        ));
        let mut app = app_with_phase(Phase::Failed(reason.into()));
        app.insert_resource(rejoin);
        app.update();
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<(&AuthoredWidgetId, &Text)>();
        let heading = texts
            .iter(world)
            .find_map(|(id, text)| (id.0 == "shell.rejection.heading").then(|| text.0.clone()))
            .unwrap();
        let detail = texts
            .iter(world)
            .find_map(|(id, text)| (id.0 == "shell.rejection.detail").then(|| text.0.clone()))
            .unwrap();
        assert_eq!(heading, "This game is open in another window");
        assert!(detail.starts_with("Close the other window, then reload"));
        let mut reload = world.query_filtered::<Entity, With<ReloadButton>>();
        assert_eq!(reload.iter(world).count(), 1);
    }

    #[test]
    fn rejected_reason_has_player_action_copy() {
        let mut app = app_with_phase(Phase::Failed("no open slot".to_owned()));
        app.update();
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        let content = texts
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert!(content.contains(&"This game is full"));
        assert!(
            content
                .iter()
                .any(|text| text.contains("Wait for a player to leave"))
        );
        assert!(content.iter().any(|text| text == &"Reason: no open slot"));
    }

    #[test]
    fn current_joined_transport_disconnect_reaches_the_actual_lost_screen() {
        use lightyear::prelude::Disconnected;
        let mut app = app_with_phase(Phase::Joined { slot: 0 });
        let current = app.world_mut().spawn_empty().id();
        let mut rejoin = RejoinController::default();
        assert!(rejoin.start_attempt(current.to_bits()));
        assert!(rejoin.on_connected(current.to_bits()));
        rejoin.advance(0.0, true);
        let mut join = crate::join::JoinState::default();
        join.on_connected();
        app.insert_resource(rejoin)
            .insert_resource(join)
            .add_observer(crate::join::on_disconnected);
        app.world_mut()
            .entity_mut(current)
            .insert(Disconnected::default());
        app.world_mut().flush();
        app.update();
        app.update();
        assert!(
            app.world()
                .resource::<CommandSendStatus>()
                .reconnect_required,
            "the actual current Joined transport's Disconnected event must request the designed reload path"
        );
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "Connection lost")
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<ReloadButton>>()
                .iter(world)
                .count(),
            1
        );
    }

    #[test]
    fn stale_initial_and_prejoin_disconnects_do_not_claim_connection_loss() {
        use lightyear::prelude::Disconnected;
        for (joined, connected, stale) in [
            (true, true, true),
            (true, false, false),
            (false, true, false),
        ] {
            let mut app = app_with_phase(if joined {
                Phase::Joined { slot: 0 }
            } else {
                Phase::Joining
            });
            let current = app.world_mut().spawn_empty().id();
            let old = app.world_mut().spawn_empty().id();
            let mut rejoin = RejoinController::default();
            assert!(rejoin.start_attempt(current.to_bits()));
            if connected {
                assert!(rejoin.on_connected(current.to_bits()));
            }
            if joined {
                rejoin.advance(0.0, true);
            }
            app.insert_resource(rejoin)
                .init_resource::<crate::join::JoinState>()
                .add_observer(crate::join::on_disconnected);
            app.world_mut()
                .entity_mut(if stale { old } else { current })
                .insert(Disconnected::default());
            app.world_mut().flush();
            app.update();
            app.update();
            assert!(
                !app.world()
                    .resource::<CommandSendStatus>()
                    .reconnect_required,
                "stale, initial and prejoin transport events must not become a post-join loss"
            );
        }
    }

    #[test]
    fn reconnect_required_creates_exact_lost_screen_and_button_action() {
        let mut app = app_with_phase(Phase::Joined { slot: 0 });
        app.world_mut()
            .resource_mut::<CommandSendStatus>()
            .reconnect_required = true;
        app.update();
        app.update();
        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        assert!(texts.iter(world).any(|text| text.0 == "Connection lost"));
        let mut buttons = world.query::<(&ReloadButton, &UiButton, &AuthoredWidgetId)>();
        assert!(
            buttons
                .iter(world)
                .any(|(_, _, id)| id.0 == "shell.connection_lost.reload")
        );
    }

    #[test]
    fn activating_the_rendered_reload_button_emits_one_real_reload_request() {
        let mut app = app_with_phase(Phase::Joined { slot: 0 });
        app.world_mut()
            .resource_mut::<CommandSendStatus>()
            .reconnect_required = true;
        app.update();
        app.update();
        let button = {
            let world = app.world_mut();
            let mut buttons = world.query_filtered::<Entity, With<ReloadButton>>();
            buttons
                .iter(world)
                .next()
                .expect("the lost screen has a reload button")
        };
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(
            app.world().resource::<Messages<ReloadRequested>>().len(),
            1,
            "the actual UI button queues one platform reload action"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_relaunch_arguments_preserve_options_and_replace_the_player_id() {
        let args = native_relaunch_args(
            [
                std::ffi::OsString::from("--server"),
                std::ffi::OsString::from("http://127.0.0.1:9000"),
                std::ffi::OsString::from("--player-id"),
                std::ffi::OsString::from("12"),
                std::ffi::OsString::from("--out"),
                std::ffi::OsString::from("view.json"),
            ],
            84,
        );
        let text = args
            .into_iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            text,
            [
                "--server",
                "http://127.0.0.1:9000",
                "--out",
                "view.json",
                "--player-id",
                "84",
            ]
        );
    }

    #[test]
    fn an_existing_rejoin_countdown_suppresses_duplicate_load_screen_and_camera() {
        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt_at(7, 0.0));
        let mut app = app_with_phase(Phase::Joining);
        app.insert_resource(rejoin);
        app.update();
        app.update();
        let world = app.world_mut();
        let mut screens = world.query::<&ProductScreenKind>();
        assert_eq!(screens.iter(world).count(), 0);
        let mut cameras = world.query_filtered::<Entity, With<ProductScreenCameraMarker>>();
        assert_eq!(cameras.iter(world).count(), 0);
        let mut notices = world.query_filtered::<&Text, With<crate::client_shell::ClientNotice>>();
        let visible = notices
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>();
        assert_eq!(visible, vec!["Reconnecting to your game\n15 s"]);
        let mut rejoin_cameras =
            world.query_filtered::<Entity, With<crate::client_shell::RejoinOverlayCamera>>();
        assert_eq!(rejoin_cameras.iter(world).count(), 1);
    }
}
