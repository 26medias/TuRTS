//! Future Task15: local audio and feedback presentation.

use bevy::prelude::Resource;
use serde::Serialize;

use crate::debug_feed::StartedSound;

/// Frame-local presentation evidence written by Task 15 before the Last snapshot.
#[derive(Resource, Debug, Default, Clone, PartialEq, Serialize)]
pub struct FrameFeedback {
    pub audio_started: Vec<StartedSound>,
    pub notices: Vec<serde_json::Value>,
    pub alerts: Vec<serde_json::Value>,
}

/// Empty registration point for the future owning task.
pub struct FeedbackPlugin;
impl bevy::app::Plugin for FeedbackPlugin {
    fn build(&self, _app: &mut bevy::app::App) {}
}
