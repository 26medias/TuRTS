//! Server statistics.
//!
//! Task 7b adds the counters that the session and outbound systems write. Task 7c adds the tick
//! timing and the JSON output.

use bevy::prelude::*;

use crate::config::Cli;

/// Counters since the start of the process.
#[derive(Resource, Default, Debug)]
pub struct Stats {
    /// `Hello` messages that the server decided (accepted or refused).
    pub hellos: u64,
    /// `ResyncRequest` messages from sessions.
    pub resync_requests: u64,
    /// The postcard size of every `UnitDeltas` sent, summed over the sessions.
    pub delta_payload_bytes_total: u64,
}

/// Add the statistics to the app. Task 7c fills in the output.
pub fn install(app: &mut App, _cli: &Cli) {
    app.init_resource::<Stats>();
}
