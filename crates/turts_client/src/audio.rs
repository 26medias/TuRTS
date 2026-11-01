//! Future Task15: audio output owner.

/// Empty registration point for the future owning task.
pub struct AudioPlugin;
impl bevy::app::Plugin for AudioPlugin {
    fn build(&self, _app: &mut bevy::app::App) {}
}
