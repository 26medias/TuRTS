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
            .insert_resource(GlobalVolume::new(Volume::Linear(1.0)))
            .add_systems(
                Update,
                (
                    bootstrap_volume_settings,
                    crate::volume_settings::apply_volume_control_changes,
                    persist_volume_settings,
                    crate::volume_settings::sync_volume_controls,
                    crate::volume_settings::sync_volume_thumb_positions,
                    load_sound_assets,
                    spawn_requested_sounds,
                    update_live_gains,
                )
                    .chain()
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
fn bootstrap_volume_settings(
    mut settings: ResMut<VolumeSettings>,
    mut state: ResMut<SettingsPersistence>,
    mut frame: ResMut<FrameFeedback>,
    mut notices: ResMut<NoticesFeed>,
) {
    if state.loaded {
        return;
    }
    let loaded = load_settings();
    *settings = loaded.volumes;
    if let Some(notice) = loaded.notice {
        let value = serde_json::json!({"text":notice,"message":notice});
        frame.notices.push(value.clone());
        notices.0.push(value);
    }
    state.snapshot = Some(*settings);
    state.loaded = true;
}

#[cfg(target_family = "wasm")]
fn load_settings() -> LoadedSettings {
    match crate::volume_settings::read_web_settings() {
        Ok(Some(json)) => crate::volume_settings::decode_settings_json(&json),
        Ok(None) | Err(_) => LoadedSettings {
            volumes: VolumeSettings::default(),
            notice: None,
        },
    }
}

#[cfg(not(target_family = "wasm"))]
fn load_settings() -> LoadedSettings {
    crate::volume_settings::native_user_config_dir()
        .map(|dir| {
            SettingsStore::for_path(crate::volume_settings::native_settings_path(&dir)).load()
        })
        .unwrap_or(LoadedSettings {
            volumes: VolumeSettings::default(),
            notice: None,
        })
}

fn persist_volume_settings(settings: Res<VolumeSettings>, mut state: ResMut<SettingsPersistence>) {
    if !state.loaded || state.snapshot == Some(*settings) {
        return;
    }
    if save_settings(*settings).is_ok() {
        state.snapshot = Some(*settings);
    }
}

#[cfg(target_family = "wasm")]
fn save_settings(settings: VolumeSettings) -> Result<(), String> {
    let mut json = crate::volume_settings::read_web_settings()?;
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
    crate::volume_settings::write_web_settings(
        json.as_deref().expect("settings document is generated"),
    )
}

#[cfg(not(target_family = "wasm"))]
fn save_settings(settings: VolumeSettings) -> Result<(), String> {
    let dir = crate::volume_settings::native_user_config_dir()
        .ok_or_else(|| "user settings directory is unavailable".to_string())?;
    let store = SettingsStore::for_path(crate::volume_settings::native_settings_path(&dir));
    for category in [
        VolumeCategory::Master,
        VolumeCategory::Voice,
        VolumeCategory::Alerts,
        VolumeCategory::Effects,
    ] {
        store.set_volume(category, settings.value(category))?;
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
