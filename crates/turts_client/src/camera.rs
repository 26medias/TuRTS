//! Task 11 camera owner configuration and extension plugin.

use bevy::prelude::{App, Plugin, Resource};
use turts_interaction::{CameraState, camera::CameraOptions};

#[derive(Resource, Debug, Clone, Default)]
pub struct CameraConfiguration {
    /// Optional owner-selected initial pose. Task 9 falls back to its legacy map-centred pose.
    pub initial_pose: Option<CameraState>,
    /// Authored controls and player camera settings consumed by the future camera owner.
    pub options: CameraOptions,
}

pub struct ClientCameraPlugin;
impl Plugin for ClientCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraConfiguration>();
    }
}
