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
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StartedSound {
    pub id: String,
    pub category: String,
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
    pub started: Vec<StartedSound>,
    pub categories: BTreeMap<String, f32>,
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
