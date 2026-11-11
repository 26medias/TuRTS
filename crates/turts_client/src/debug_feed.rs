//! Typed client debug feeds. Each feed is written by the feature owner before the Last-stage snapshot.
use bevy::prelude::Resource;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};

#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct SelectionFeed(pub Vec<u32>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct GroupsFeed(pub BTreeMap<u8, Vec<u32>>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct ResultsFeed(pub Vec<serde_json::Value>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct FramesFeed {
    pub frame_index: u64,
    pub recent: VecDeque<super::core_adapter::FrameRecord>,
    pub retained_input: VecDeque<super::core_adapter::FrameRecord>,
    pub last_result_count: usize,
}
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct CursorFeed {
    pub context: Option<String>,
    pub image_id: Option<String>,
    pub applied_icon: Option<String>,
    /// Actual window-logical cursor position at the same cursor observation.
    /// `None` is retained when the platform reports no position, including pointer lock.
    pub position: Option<[f32; 2]>,
    /// Engine pointer used for Core hover and the in-game cursor. While locked this is derived
    /// from the retained pre-lock position plus real relative MouseMotion samples.
    pub engine_position: Option<[f32; 2]>,
    /// Requested grab state from the actual primary-window CursorOptions component.
    pub lock_requested: bool,
    /// Actual browser pointer-lock state for the game canvas. None means the platform did not
    /// expose a readable lock state; this is separate from Bevy's requested grab state.
    pub lock_active: Option<bool>,
    /// Sum of actual Bevy relative-motion messages consumed for this Core frame.
    pub relative_motion: [f32; 2],
    /// Core frame that produced the engine position and relative-motion sum.
    pub frame_index: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StartedSound {
    pub id: String,
    pub category: String,
    pub priority: i16,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AudioSinkObservation {
    pub play_id: u64,
    pub player_entity: u64,
    pub sound_id: String,
    pub category: String,
    pub gain: f32,
    /// Core frame that requested this actual player entity.
    pub request_frame_index: u64,
    /// Core frame in which the real Bevy sink was observed.
    pub observed_core_frame_index: u64,
    /// Audio owner's monotonically increasing observation frame.
    pub frame_index: u64,
}
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct UiSnapshotFeed(pub serde_json::Value);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct NoticesFeed(pub Vec<serde_json::Value>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct AlertsFeed(pub Vec<serde_json::Value>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct AudioFeed {
    pub context_state: Option<String>,
    /// Actual states of every browser AudioContext captured by the pre-WASM hook.
    pub context_states: Vec<String>,
    /// True only after a real pointer/key activation reported by the page hook.
    pub gesture_seen: bool,
    pub started: Vec<StartedSound>,
    pub categories: BTreeMap<String, f32>,
    /// Count measured from real Bevy audio-player entities by the audio owner.
    pub player_entities: usize,
    /// Live player count per category, measured from real Bevy audio-player entities.
    pub active_by_category: BTreeMap<String, usize>,
    /// Peak live player count by category, sampled from real entities since the feed was reset.
    pub peak_active_by_category: BTreeMap<String, usize>,
    /// Bounded observations of real Bevy sinks, retained through despawn for browser evidence.
    pub sink_observations: VecDeque<AudioSinkObservation>,
    /// Current real sink/gain samples, refreshed after Bevy audio playback each frame.
    pub current_sinks: Vec<AudioSinkObservation>,
    /// Bounded per-frame real sink/gain history for post-playback volume checks.
    pub gain_samples: VecDeque<AudioSinkObservation>,
}
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct ContextMenuFeed {
    pub count: u64,
    pub count_prevented: u64,
}
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct FocusFeed(pub bool);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct PlayerIdFeed(pub Option<u64>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct AlertPingsFeed(pub Vec<serde_json::Value>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct MarkersShown(pub Vec<serde_json::Value>);
#[derive(Resource, Default, Debug, Clone, Serialize)]
pub struct RingsShown(pub Vec<serde_json::Value>);
