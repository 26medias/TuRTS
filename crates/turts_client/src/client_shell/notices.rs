//! Shell notices and overlays: the focus overlay, the sound-off notice, the client notice
//! row, and the rejoin camera.

use super::*;

/// Engine-owned text row used only when a joined package has no authored interface.
#[derive(Component)]
pub(crate) struct ClientNotice;

pub(super) fn sync_focus_overlay(
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
pub(super) struct SoundLockTimer(Option<std::time::Duration>);

/// Show `SOUND_OFF_NOTICE` for 8 s when audio stays locked after the first gesture. Each later
/// gesture asks the browser to resume audio through the page hook (index.html).
pub(super) fn sync_sound_off_notice(
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
pub(super) fn sync_sound_status_label(
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

pub(super) fn sound_status_label(audio: Option<&crate::debug_feed::AudioFeed>) -> &'static str {
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
