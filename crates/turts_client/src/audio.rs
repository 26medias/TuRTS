//! Audio assets, context readiness, category gains and actual sink evidence.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

#[cfg(not(target_family = "wasm"))]
use crate::volume_settings::SettingsStore;
use crate::{
    debug_feed::{AudioFeed, AudioSinkObservation, NoticesFeed, StartedSound},
    feedback::{FeedbackSet, FrameFeedback},
    ingest::ReplicaState,
    join::AcquiredPackages,
    volume_settings::{LoadedSettings, VolumeCategory, VolumeSettings},
};
use bevy::{
    audio::{
        AudioPlayer as BevyAudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, GlobalVolume,
        PlaybackSettings, SpatialAudioSink, Volume,
    },
    prelude::*,
};

#[derive(Resource, Default)]
struct SoundCatalog(BTreeMap<String, Handle<AudioSource>>);

#[derive(Component, Clone, Copy, Debug)]
pub struct AudioCategoryPlayer(pub VolumeCategory, pub i16);

#[derive(Component, Clone, Debug)]
struct AudioIdentity(String);

#[derive(Component, Clone, Copy)]
struct AudioRequestFrame(u64);

#[derive(Resource, Default)]
struct AudioObservationState {
    frame: u64,
    play_id: u64,
    joined: bool,
    observed_players: HashMap<u64, u64>,
}

#[derive(Resource, Default)]
struct SettingsPersistence {
    loaded: bool,
    snapshot: Option<VolumeSettings>,
    controls: Option<crate::client_preferences::ControlSettings>,
    pending_notice: Option<String>,
}

type RealAudioPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static AudioCategoryPlayer,
        &'static AudioIdentity,
        &'static AudioRequestFrame,
        Option<&'static AudioSink>,
        Option<&'static SpatialAudioSink>,
    ),
>;

/// Audio owner. Browser context readiness comes only from the pre-WASM observation hook.
pub struct AudioPlugin;
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::ui_widgets::SliderPlugin>() {
            app.add_plugins(bevy::ui_widgets::SliderPlugin);
        }
        app.add_observer(bevy::ui_widgets::slider_self_update)
            .init_resource::<VolumeSettings>()
            .init_resource::<Assets<AudioSource>>()
            .init_resource::<SoundCatalog>()
            .init_resource::<AudioObservationState>()
            .init_resource::<SettingsPersistence>()
            .insert_resource(GlobalVolume::new(Volume::Linear(1.0)));
        register_settings_systems(app);
        app.add_systems(
            Update,
            (load_sound_assets, spawn_requested_sounds, update_live_gains)
                .chain()
                .after(crate::volume_settings::sync_volume_thumb_positions)
                .after(FeedbackSet),
        );
        app.add_systems(
            Last,
            observe_real_sinks.before(crate::core_adapter::FrameCloseSet),
        );
        #[cfg(target_family = "wasm")]
        app.add_systems(
            Update,
            refresh_browser_audio_state.before(spawn_requested_sounds),
        );
    }
}
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SettingsLoadSet;

fn register_settings_systems(app: &mut App) {
    app.add_systems(
        Update,
        load_client_settings_once
            .in_set(SettingsLoadSet)
            .before(crate::core_adapter::CoreAdapterSet),
    );
    app.add_systems(
        Update,
        (
            bootstrap_volume_settings,
            crate::volume_settings::apply_volume_control_changes,
            persist_volume_settings.before(crate::ui_shared::UiPresentationSet),
            crate::volume_settings::sync_volume_controls,
            crate::volume_settings::sync_volume_thumb_positions,
        )
            .chain()
            .after(FeedbackSet),
    );
}

fn initialize_settings(
    settings: &mut VolumeSettings,
    state: &mut SettingsPersistence,
    controls: Option<&mut crate::client_shell::ClientSettings>,
    health: Option<&mut crate::visuals::HealthDisplay>,
    preferences: Option<&mut crate::core_adapter::CorePreferences>,
    root: Option<&crate::persist::PersistenceConfigRoot>,
) {
    let loaded = load_settings(root);
    *settings = loaded.volumes;
    loaded.controls.apply(controls, health, preferences);
    state.controls = Some(loaded.controls);
    state.pending_notice = loaded.notice;
    state.snapshot = Some(*settings);
    state.loaded = true;
}

#[derive(bevy::ecs::system::SystemParam)]
struct RuntimeControlResources<'w> {
    client: Option<ResMut<'w, crate::client_shell::ClientSettings>>,
    health: Option<ResMut<'w, crate::visuals::HealthDisplay>>,
    preferences: Option<ResMut<'w, crate::core_adapter::CorePreferences>>,
}

fn load_client_settings_once(
    mut settings: ResMut<VolumeSettings>,
    mut state: ResMut<SettingsPersistence>,
    mut controls: RuntimeControlResources,
    root: Option<Res<crate::persist::PersistenceConfigRoot>>,
) {
    if state.loaded {
        return;
    }
    initialize_settings(
        &mut settings,
        &mut state,
        controls.client.as_deref_mut(),
        controls.health.as_deref_mut(),
        controls.preferences.as_deref_mut(),
        root.as_deref(),
    );
}

fn bootstrap_volume_settings(
    mut settings: ResMut<VolumeSettings>,
    mut state: ResMut<SettingsPersistence>,
    mut frame: ResMut<FrameFeedback>,
    mut notices: ResMut<NoticesFeed>,
    mut controls: RuntimeControlResources,
    root: Option<Res<crate::persist::PersistenceConfigRoot>>,
) {
    if !state.loaded {
        initialize_settings(
            &mut settings,
            &mut state,
            controls.client.as_deref_mut(),
            controls.health.as_deref_mut(),
            controls.preferences.as_deref_mut(),
            root.as_deref(),
        );
    }
    // Publish after feedback reset, using the one parse decision made before player input.
    if let Some(notice) = state.pending_notice.take() {
        let value = serde_json::json!({"text":notice,"message":notice});
        frame.notices.push(value.clone());
        notices.0.push(value);
    }
}

#[cfg(target_family = "wasm")]
fn load_settings(_root: Option<&crate::persist::PersistenceConfigRoot>) -> LoadedSettings {
    match crate::volume_settings::read_web_settings() {
        Ok(Some(json)) => crate::volume_settings::decode_settings_json(&json),
        Ok(None) | Err(_) => LoadedSettings {
            volumes: VolumeSettings::default(),
            notice: None,
            ..Default::default()
        },
    }
}

#[cfg(not(target_family = "wasm"))]
fn load_settings(root: Option<&crate::persist::PersistenceConfigRoot>) -> LoadedSettings {
    native_settings_root(root)
        .map(|dir| {
            SettingsStore::for_path(crate::volume_settings::native_settings_path(&dir)).load()
        })
        .unwrap_or(LoadedSettings {
            volumes: VolumeSettings::default(),
            notice: None,
            ..Default::default()
        })
}

#[cfg(not(target_family = "wasm"))]
fn native_settings_root(
    root: Option<&crate::persist::PersistenceConfigRoot>,
) -> Option<std::path::PathBuf> {
    match root {
        Some(root) => root.0.clone(),
        None => crate::volume_settings::native_user_config_dir(),
    }
}

fn persist_volume_settings(
    settings: Res<VolumeSettings>,
    controls: Option<Res<crate::client_shell::ClientSettings>>,
    health: Option<Res<crate::visuals::HealthDisplay>>,
    preferences: Option<Res<crate::core_adapter::CorePreferences>>,
    root: Option<Res<crate::persist::PersistenceConfigRoot>>,
    mut state: ResMut<SettingsPersistence>,
) {
    if !state.loaded {
        return;
    }
    let controls = crate::client_preferences::ControlSettings::capture(
        controls.as_deref(),
        health.as_deref(),
        preferences.as_deref(),
        state.controls,
    );
    let audio_changed = state.snapshot != Some(*settings);
    let controls_changed = state.controls != Some(controls);
    if !audio_changed && !controls_changed {
        return;
    }
    if save_settings(
        *settings,
        &controls,
        audio_changed,
        controls_changed,
        root.as_deref(),
    )
    .is_ok()
    {
        state.snapshot = Some(*settings);
        state.controls = Some(controls);
    }
}

#[cfg(target_family = "wasm")]
fn save_settings(
    settings: VolumeSettings,
    controls: &crate::client_preferences::ControlSettings,
    audio_changed: bool,
    controls_changed: bool,
    _root: Option<&crate::persist::PersistenceConfigRoot>,
) -> Result<(), String> {
    if !audio_changed && !controls_changed {
        return Ok(());
    }
    let mut json = crate::volume_settings::read_web_settings()?;
    if audio_changed {
        for category in [
            VolumeCategory::Master,
            VolumeCategory::Voice,
            VolumeCategory::Alerts,
            VolumeCategory::Effects,
        ] {
            json = Some(crate::volume_settings::set_volume_in_json(
                json.as_deref(),
                category,
                settings.value(category),
            )?);
        }
    }
    if controls_changed {
        json = Some(crate::volume_settings::set_controls_in_json(
            json.as_deref(),
            controls,
        )?);
    }
    crate::volume_settings::write_web_settings(
        json.as_deref()
            .ok_or_else(|| "changed settings document is unavailable".to_string())?,
    )
}

#[cfg(not(target_family = "wasm"))]
fn save_settings(
    settings: VolumeSettings,
    controls: &crate::client_preferences::ControlSettings,
    audio_changed: bool,
    controls_changed: bool,
    root: Option<&crate::persist::PersistenceConfigRoot>,
) -> Result<(), String> {
    if !controls.valid() {
        return Err("client settings are outside their supported ranges".into());
    }
    let dir = native_settings_root(root)
        .ok_or_else(|| "user settings directory is unavailable".to_string())?;
    let store = SettingsStore::for_path(crate::volume_settings::native_settings_path(&dir));
    if audio_changed {
        for category in [
            VolumeCategory::Master,
            VolumeCategory::Voice,
            VolumeCategory::Alerts,
            VolumeCategory::Effects,
        ] {
            store.set_volume(category, settings.value(category))?;
        }
    }
    if controls_changed {
        store.set_controls(controls)?;
    }
    Ok(())
}

/// True only after an actual player gesture and a nonempty set of actual running contexts.
pub fn browser_contexts_ready(states: &[String], gesture_seen: bool) -> bool {
    gesture_seen && !states.is_empty() && states.iter().all(|state| state == "running")
}

/// Apply category gain to a sink. Master remains represented in the product; GlobalVolume stays 1.
pub fn set_sink_gain(
    settings: VolumeSettings,
    category: VolumeCategory,
    set_linear_volume: impl FnOnce(f32),
) {
    set_linear_volume(settings.gain(category));
}

fn load_sound_assets(
    packages: Option<Res<AcquiredPackages>>,
    mut catalog: ResMut<SoundCatalog>,
    mut assets: ResMut<Assets<AudioSource>>,
) {
    let Some(packages) = packages else { return };
    for (id, bytes) in &packages.0.presentation.sounds {
        if !catalog.0.contains_key(id) {
            catalog.0.insert(
                id.clone(),
                assets.add(AudioSource {
                    bytes: Arc::from(bytes.clone()),
                }),
            );
        }
    }
}

fn spawn_requested_sounds(
    mut commands: Commands,
    feedback: Res<FrameFeedback>,
    catalog: Res<SoundCatalog>,
    settings: Res<VolumeSettings>,
    _feed: Res<AudioFeed>,
    frames: Option<Res<crate::core_adapter::FrameFeeds>>,
    active: Query<(Entity, &AudioCategoryPlayer)>,
) {
    #[cfg(target_family = "wasm")]
    if !browser_contexts_ready(&_feed.context_states, _feed.gesture_seen) {
        return;
    }
    let request_frame_index = frames
        .as_deref()
        .map_or(0, |frames| frames.frame_index.saturating_add(1));
    let mut active_voice: Vec<Entity> = active
        .iter()
        .filter_map(|(entity, category)| (category.0 == VolumeCategory::Voice).then_some(entity))
        .collect();
    let mut active_alerts: Vec<(Entity, i16)> = active
        .iter()
        .filter_map(|(entity, category)| {
            (category.0 == VolumeCategory::Alerts).then_some((entity, category.1))
        })
        .collect();
    for sound in &feedback.audio_started {
        let Some(handle) = catalog.0.get(&sound.id) else {
            continue;
        };
        let Some(category) = parse_category(&sound.category) else {
            continue;
        };
        if category == VolumeCategory::Voice {
            for entity in active_voice.drain(..) {
                commands.entity(entity).despawn();
            }
        } else if category == VolumeCategory::Alerts && active_alerts.len() >= 2 {
            let Some((lowest_index, (_, lowest_priority))) = active_alerts
                .iter()
                .enumerate()
                .min_by_key(|(_, (_, priority))| *priority)
            else {
                continue;
            };
            if sound.priority <= *lowest_priority {
                continue;
            }
            let (entity, _) = active_alerts.remove(lowest_index);
            commands.entity(entity).despawn();
        }
        let entity = commands
            .spawn((
                BevyAudioPlayer(handle.clone()),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(settings.gain(category))),
                AudioCategoryPlayer(category, sound.priority),
                AudioIdentity(sound.id.clone()),
                AudioRequestFrame(request_frame_index),
            ))
            .id();
        if category == VolumeCategory::Voice {
            active_voice.push(entity);
        }
        if category == VolumeCategory::Alerts {
            active_alerts.push((entity, sound.priority));
        }
    }
}

fn parse_category(category: &str) -> Option<VolumeCategory> {
    match category {
        "voice" => Some(VolumeCategory::Voice),
        "alerts" => Some(VolumeCategory::Alerts),
        "effects" => Some(VolumeCategory::Effects),
        _ => None,
    }
}

fn update_live_gains(
    settings: Res<VolumeSettings>,
    mut players: Query<(
        &AudioCategoryPlayer,
        Option<&mut AudioSink>,
        Option<&mut SpatialAudioSink>,
    )>,
) {
    // AudioSink components can arrive after their AudioPlayer was spawned. Keep
    // applying the current value so a sink created after a slider change does
    // not retain the older PlaybackSettings gain.
    for (category, sink, spatial) in &mut players {
        let gain = Volume::Linear(settings.gain(category.0));
        if let Some(mut sink) = sink {
            sink.set_volume(gain);
        }
        if let Some(mut sink) = spatial {
            sink.set_volume(gain);
        }
    }
}

fn observe_real_sinks(
    players: RealAudioPlayers<'_, '_>,
    replica: Option<Res<ReplicaState>>,
    frames: Option<Res<crate::core_adapter::FrameFeeds>>,
    mut state: ResMut<AudioObservationState>,
    mut feed: ResMut<AudioFeed>,
) {
    state.frame = state.frame.saturating_add(1);
    let joined = replica
        .as_deref()
        .is_some_and(|state| state.welcome.is_some() && state.replica.is_some());
    let observed_core_frame_index = frames
        .as_deref()
        .map_or(0, |frames| frames.frame_index.saturating_add(1));
    if joined && !state.joined {
        feed.started.clear();
        feed.active_by_category.clear();
        feed.peak_active_by_category.clear();
        feed.sink_observations.clear();
        feed.current_sinks.clear();
        feed.gain_samples.clear();
        state.observed_players.clear();
    }
    state.joined = joined;
    let mut active = BTreeMap::<String, usize>::new();
    let mut live_players = HashSet::new();
    let mut total = 0usize;
    feed.started.clear();
    feed.current_sinks.clear();
    for (entity, category, identity, request_frame, sink, spatial) in &players {
        total += 1;
        live_players.insert(entity.to_bits());
        let label = category_name(category.0).to_string();
        *active.entry(label.clone()).or_default() += 1;
        let gain = sink
            .map(AudioSinkPlayback::volume)
            .or_else(|| spatial.map(AudioSinkPlayback::volume))
            .map(|volume| volume.to_linear());
        if let Some(gain) = gain.filter(|_| sink.is_some() || spatial.is_some()) {
            feed.started.push(StartedSound {
                id: identity.0.clone(),
                category: label.clone(),
                priority: category.1,
            });
            let player_entity = entity.to_bits();
            let play_id = if let Some(play_id) = state.observed_players.get(&player_entity) {
                *play_id
            } else {
                state.play_id = state.play_id.saturating_add(1);
                let play_id = state.play_id;
                state.observed_players.insert(player_entity, play_id);
                play_id
            };
            let observation = AudioSinkObservation {
                play_id,
                player_entity,
                sound_id: identity.0.clone(),
                category: label.clone(),
                gain,
                request_frame_index: request_frame.0,
                observed_core_frame_index,
                frame_index: state.frame,
            };
            feed.current_sinks.push(observation.clone());
            feed.gain_samples.push_back(observation.clone());
            while feed.gain_samples.len() > 4096 {
                feed.gain_samples.pop_front();
            }
            retain_sink_onset(&mut feed, observation);
        }
    }
    state
        .observed_players
        .retain(|entity, _| live_players.contains(entity));
    for (category, count) in &active {
        let peak = feed
            .peak_active_by_category
            .entry(category.clone())
            .or_default();
        *peak = (*peak).max(*count);
    }
    feed.player_entities = total;
    feed.active_by_category = active;
}

fn retain_sink_onset(feed: &mut AudioFeed, observation: AudioSinkObservation) {
    if feed
        .sink_observations
        .iter()
        .any(|prior| prior.player_entity == observation.player_entity)
    {
        return;
    }
    feed.sink_observations.push_back(observation);
    while feed.sink_observations.len() > 512 {
        feed.sink_observations.pop_front();
    }
}

fn category_name(category: VolumeCategory) -> &'static str {
    match category {
        VolumeCategory::Master => "master",
        VolumeCategory::Voice => "voice",
        VolumeCategory::Alerts => "alerts",
        VolumeCategory::Effects => "effects",
    }
}

#[cfg(test)]
mod audio_policy_tests {
    use super::*;

    #[test]
    fn a_queued_player_without_a_real_sink_is_not_reported_as_started_audio() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AudioPlugin)
            .insert_resource(AudioFeed::default())
            .insert_resource(NoticesFeed::default())
            .insert_resource(FrameFeedback::default());
        app.world_mut().spawn((
            AudioCategoryPlayer(VolumeCategory::Alerts, 10),
            AudioIdentity("muster:muster_alert".into()),
            AudioRequestFrame(7),
        ));

        app.update();

        let audio = app.world().resource::<AudioFeed>();
        assert_eq!(
            audio.player_entities, 1,
            "the queued player count remains separately observable"
        );
        assert!(
            audio.started.is_empty(),
            "started requires an actual Bevy AudioSink or SpatialAudioSink"
        );
        assert!(audio.current_sinks.is_empty());
        assert!(audio.gain_samples.is_empty());
        assert!(audio.sink_observations.is_empty());
    }

    #[test]
    fn onset_history_keeps_the_first_sample_when_live_gain_is_sampled_for_over_96_frames() {
        let mut feed = AudioFeed::default();
        let first = AudioSinkObservation {
            play_id: 1,
            player_entity: 12,
            sound_id: "muster:muster_alert".into(),
            category: "alerts".into(),
            gain: 0.4,
            request_frame_index: 20,
            observed_core_frame_index: 20,
            frame_index: 20,
        };
        retain_sink_onset(&mut feed, first);
        for frame_index in 21..=140 {
            feed.current_sinks = vec![AudioSinkObservation {
                play_id: 1,
                player_entity: 12,
                sound_id: "muster:muster_alert".into(),
                category: "alerts".into(),
                gain: 0.8,
                request_frame_index: 20,
                observed_core_frame_index: 20,
                frame_index,
            }];
            feed.gain_samples.push_back(feed.current_sinks[0].clone());
            let current = feed.current_sinks[0].clone();
            retain_sink_onset(&mut feed, current);
        }
        assert_eq!(
            feed.sink_observations.len(),
            1,
            "one actual entity has one stable onset record"
        );
        assert_eq!(
            feed.sink_observations[0].frame_index, 20,
            "repeated current samples cannot replace its onset frame"
        );
        assert_eq!(
            feed.current_sinks[0].frame_index, 140,
            "current actual sink data remains independently fresh"
        );
        assert_eq!(
            feed.current_sinks[0].gain, 0.8,
            "current volume checks see the live gain"
        );
        assert_eq!(
            feed.gain_samples.back().unwrap().frame_index,
            140,
            "recent actual gain history remains distinct from the stable onset"
        );
    }

    #[test]
    fn a_new_voice_response_replaces_the_existing_voice_entity() {
        let mut app = App::new();
        app.add_plugins(AudioPlugin)
            .insert_resource(AudioFeed::default())
            .insert_resource(NoticesFeed::default())
            .insert_resource(FrameFeedback {
                audio_started: vec![StartedSound {
                    id: "muster:new_voice".into(),
                    category: "voice".into(),
                    priority: 0,
                }],
                ..Default::default()
            });
        let old = app
            .world_mut()
            .spawn((
                AudioCategoryPlayer(VolumeCategory::Voice, 0),
                AudioIdentity("muster:old_voice".into()),
            ))
            .id();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: Arc::from(Vec::<u8>::new()),
            });
        app.world_mut()
            .resource_mut::<SoundCatalog>()
            .0
            .insert("muster:new_voice".into(), handle);

        app.update();

        let voices = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &AudioCategoryPlayer, &AudioIdentity)>();
            query
                .iter(world)
                .filter(|(_, category, _)| category.0 == VolumeCategory::Voice)
                .map(|(entity, _, identity)| (entity, identity.0.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            voices.len(),
            1,
            "only one voice entity remains after a response starts"
        );
        assert_ne!(
            voices[0].0, old,
            "the prior voice is replaced by the new response"
        );
        assert_eq!(voices[0].1, "muster:new_voice");
    }

    #[test]
    fn higher_priority_alert_replaces_the_lowest_of_two_live_alerts() {
        let mut app = App::new();
        app.add_plugins(AudioPlugin)
            .insert_resource(AudioFeed::default())
            .insert_resource(NoticesFeed::default())
            .insert_resource(FrameFeedback {
                audio_started: vec![StartedSound {
                    id: "muster:priority_alert".into(),
                    category: "alerts".into(),
                    priority: 20,
                }],
                ..Default::default()
            });
        let low = app
            .world_mut()
            .spawn((
                AudioCategoryPlayer(VolumeCategory::Alerts, 1),
                AudioIdentity("muster:low".into()),
            ))
            .id();
        app.world_mut().spawn((
            AudioCategoryPlayer(VolumeCategory::Alerts, 10),
            AudioIdentity("muster:high".into()),
        ));
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: Arc::from(Vec::<u8>::new()),
            });
        app.world_mut()
            .resource_mut::<SoundCatalog>()
            .0
            .insert("muster:priority_alert".into(), handle);

        app.update();

        let alerts = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &AudioCategoryPlayer, &AudioIdentity)>();
            query
                .iter(world)
                .filter(|(_, category, _)| category.0 == VolumeCategory::Alerts)
                .map(|(entity, _, identity)| (entity, identity.0.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(alerts.len(), 2, "the simultaneous alert cap remains two");
        assert!(
            !alerts.iter().any(|(entity, _)| *entity == low),
            "the lower priority live alert is retired"
        );
        assert!(alerts.iter().any(|(_, id)| id == "muster:priority_alert"));
    }
}

#[cfg(target_family = "wasm")]
fn refresh_browser_audio_state(mut feed: ResMut<AudioFeed>) {
    use wasm_bindgen::JsCast;
    let snapshot = js_sys::Reflect::get(&js_sys::global(), &"__turts_audio_snapshot".into())
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
        .and_then(|function| function.call0(&js_sys::global()).ok());
    let Some(snapshot) = snapshot else {
        feed.context_state = None;
        feed.context_states.clear();
        feed.gesture_seen = false;
        return;
    };
    let states = js_sys::Reflect::get(&snapshot, &"states".into())
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Array>().ok())
        .map(|array| {
            array
                .iter()
                .filter_map(|value| value.as_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    feed.gesture_seen = js_sys::Reflect::get(&snapshot, &"gesture_seen".into())
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    feed.context_state = if states.is_empty() {
        None
    } else if states.iter().all(|state| state == "running") {
        Some("running".into())
    } else if states.iter().any(|state| state == "suspended") {
        Some("suspended".into())
    } else {
        Some("closed".into())
    };
    feed.context_states = states;
}

#[cfg(all(test, not(target_family = "wasm")))]
mod control_settings_persistence_contract {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new(json: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "turts-controls-{}-{nonce}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
            let path = crate::volume_settings::native_settings_path(&root);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, json).unwrap();
            Self(root)
        }
        fn json(&self) -> serde_json::Value {
            serde_json::from_slice(
                &fs::read(crate::volume_settings::native_settings_path(&self.0)).unwrap(),
            )
            .unwrap()
        }
        fn app(&self) -> App {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .insert_resource(crate::persist::PersistenceConfigRoot(Some(self.0.clone())))
                .init_resource::<VolumeSettings>()
                .init_resource::<SettingsPersistence>()
                .init_resource::<FrameFeedback>()
                .init_resource::<NoticesFeed>()
                .init_resource::<crate::client_shell::ClientSettings>()
                .init_resource::<crate::visuals::HealthDisplay>()
                .init_resource::<crate::core_adapter::CorePreferences>();
            register_settings_systems(&mut app);
            app
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn valid_document() -> &'static str {
        r#"{"audio":{"master":0.35,"voice":0.2,"alerts":0.6,"effects":0.8},"controls":{"scroll_speed":2.0,"edge_scroll":false,"ui_scale":1.25,"cursor_lock":true,"double_click_ms":650,"double_click_pixels":6.0,"health_mode":"always","future":"keep"},"unknown":{"keep":true}}"#
    }

    #[derive(Resource, Default)]
    struct FirstCoreSettings(Option<(f32, f32, f32)>);

    fn observe_first_core_settings(
        controls: Res<crate::client_shell::ClientSettings>,
        preferences: Res<crate::core_adapter::CorePreferences>,
        volume: Res<VolumeSettings>,
        mut seen: ResMut<FirstCoreSettings>,
    ) {
        if seen.0.is_none() {
            seen.0 = Some((
                controls.scroll_speed,
                preferences.interaction.double_click_pixels,
                volume.master,
            ));
        }
    }

    #[test]
    fn stored_preferences_reach_the_first_core_input_before_feedback_and_save() {
        let fixture = Fixture::new(valid_document());
        let mut app = fixture.app();
        app.init_resource::<FirstCoreSettings>()
            .configure_sets(
                Update,
                crate::core_adapter::CoreAdapterSet.before(FeedbackSet),
            )
            .add_systems(
                Update,
                observe_first_core_settings.in_set(crate::core_adapter::CoreAdapterSet),
            );
        app.update();
        assert_eq!(
            app.world().resource::<VolumeSettings>().master,
            0.35,
            "isolated file was actually loaded"
        );
        assert_eq!(
            app.world().resource::<FirstCoreSettings>().0,
            Some((2.0, 6.0, 0.35)),
            "saved preferences must reach the first Core input before feedback or save"
        );
    }

    #[test]
    fn existing_settings_bootstrap_restores_control_resources_from_the_isolated_native_store() {
        let fixture = Fixture::new(valid_document());
        let mut app = fixture.app();
        app.update();
        assert_eq!(
            app.world().resource::<VolumeSettings>().master,
            0.35,
            "actual injected store must load before judging controls"
        );
        let controls = app
            .world()
            .resource::<crate::client_shell::ClientSettings>();
        assert_eq!(
            controls.scroll_speed, 2.0,
            "stored scroll speed must survive client startup"
        );
        assert!(!controls.edge_scroll);
        assert_eq!(controls.ui_scale, 1.25);
        assert!(controls.cursor_lock);
        assert_eq!(controls.double_click_ms, 650);
        assert!(matches!(
            app.world().resource::<crate::visuals::HealthDisplay>().0,
            crate::visuals::HealthMode::Always
        ));
        let preferences = app
            .world()
            .resource::<crate::core_adapter::CorePreferences>();
        assert_eq!(preferences.interaction.double_click_pixels, 6.0);
        assert_eq!(preferences.interaction.double_click_ms, 650);
        assert!(app.world().resource::<NoticesFeed>().0.is_empty());
        assert_eq!(
            fixture.json(),
            serde_json::from_str::<serde_json::Value>(valid_document()).unwrap(),
            "bootstrap does not rewrite a valid document"
        );
    }

    #[test]
    fn control_edits_persist_through_the_existing_settings_owner_and_preserve_other_fields() {
        let fixture = Fixture::new(valid_document());
        let mut app = fixture.app();
        app.update();
        {
            let mut settings = app
                .world_mut()
                .resource_mut::<crate::client_shell::ClientSettings>();
            settings.scroll_speed = 1.75;
            settings.edge_scroll = true;
            settings.ui_scale = 1.0;
            settings.cursor_lock = false;
            settings.double_click_ms = 450;
        }
        app.world_mut()
            .resource_mut::<crate::visuals::HealthDisplay>()
            .0 = crate::visuals::HealthMode::Selected;
        app.world_mut()
            .resource_mut::<crate::core_adapter::CorePreferences>()
            .interaction
            .double_click_pixels = 5.0;
        app.update();
        let saved = fixture.json();
        assert_eq!(
            saved["controls"]["scroll_speed"], 1.75,
            "actual client control edits must write the existing settings document"
        );
        assert_eq!(saved["controls"]["health_mode"], "selected");
        assert_eq!(saved["controls"]["double_click_pixels"], 5.0);
        assert_eq!(saved["controls"]["future"], "keep");
        assert_eq!(saved["unknown"]["keep"], true);
        let original = serde_json::from_str::<serde_json::Value>(valid_document()).unwrap();
        assert_eq!(
            saved["audio"], original["audio"],
            "a control edit preserves every audio field"
        );
        let mut reloaded = fixture.app();
        reloaded.update();
        assert_eq!(
            reloaded
                .world()
                .resource::<crate::client_shell::ClientSettings>()
                .scroll_speed,
            1.75
        );
        assert!(
            !reloaded
                .world()
                .resource::<crate::client_shell::ClientSettings>()
                .cursor_lock
        );
        assert_eq!(
            reloaded
                .world()
                .resource::<crate::core_adapter::CorePreferences>()
                .interaction
                .double_click_pixels,
            5.0
        );
    }

    #[test]
    fn legacy_audio_defaults_controls_and_each_corrupt_value_emits_one_notice() {
        for (json, corrupt) in [
            (r#"{"audio":{"master":0.35}}"#, false),
            (
                r#"{"audio":{"master":0.35},"controls":{"ui_scale":"bad"}}"#,
                true,
            ),
            (
                r#"{"audio":{"master":0.35},"controls":{"health_mode":"unknown"}}"#,
                true,
            ),
            ("{not-json", true),
        ] {
            let fixture = Fixture::new(json);
            let mut app = fixture.app();
            for _ in 0..3 {
                app.update();
            }
            assert_eq!(
                app.world()
                    .resource::<crate::client_shell::ClientSettings>()
                    .scroll_speed,
                1.0,
                "defaults for {json}"
            );
            assert_eq!(
                app.world().resource::<NoticesFeed>().0.len(),
                usize::from(corrupt),
                "exactly one notice per corrupt settings value for {json}"
            );
            assert_eq!(
                app.world().resource::<VolumeSettings>().master,
                if corrupt { 1.0 } else { 0.35 },
                "audio shares the same corrupt-value decision for {json}"
            );
        }
    }
}
