//! Client logic: fetch, acquisition, join, replica, interpolation, presentation, input.

pub mod acquire;
pub mod content;
pub mod debug;
pub mod fetch;
pub mod framelog;
pub mod ingest;
pub mod input;
pub mod interp;
pub mod join;
pub mod options;
pub mod present;
pub mod render;
pub mod replica;

use std::time::Duration;

use bevy::app::{App, Plugin, ScheduleRunnerPlugin};
use bevy::prelude::*;
use lightyear::prelude::MessageSystems;
use lightyear::prelude::client::ClientPlugins;
use turts_net_lightyear::{ProtocolPlugin, tick_duration};
use turts_net_protocol::Pos;

use crate::debug::{DebugState, exit_when_done, rebuild_debug_state};
use crate::framelog::{FrameLog, write_frame_log};
use crate::ingest::{CommandResults, IngestCounters, IngestHooks, ReplicaState, ingest_messages};
use crate::input::ClickInputPlugin;
use crate::join::{
    Acquisition, JoinState, Phase, auto_move, drive_acquisition, on_connected, on_disconnected,
    receive_rejection, start_connection, update_phase,
};
use crate::options::{ClientOptions, random_player_id};
use crate::present::{Presented, update_presented};
use crate::render::UnitRenderPlugin;

/// The tick rate of the lightyear link (not of the simulation: `Welcome` carries that).
const LINK_TICK_HZ: u16 = 20; // prototype constant: measure at M1

/// The options of this run.
#[derive(Resource)]
pub struct ClientOpts(pub ClientOptions);

/// The placeholder identity of this session (design 5, `Hello`).
#[derive(Resource)]
pub struct PlayerId(pub u64);

/// All client logic: the join, ingest, the presentation record, the logs and the debug hooks.
pub struct ClientPlugin(pub ClientOptions);

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        let opts = self.0.clone();
        app.insert_resource(PlayerId(opts.player_id.unwrap_or_else(random_player_id)))
            .insert_resource(Phase::FetchAdvertisement)
            .insert_resource(IngestHooks {
                drop_offsets: opts.drop_delta_offsets.clone(),
                drop_arrival_delta: opts.drop_arrival_delta,
                arrival_target: opts.auto_move.map(|(x, y)| Pos { x, y }),
                arrival_unit: opts.move_unit,
                hold_resync_snapshots: opts.hold_resync_snapshots,
                ..Default::default()
            })
            .insert_resource(FrameLog::open(opts.frame_log.as_deref()))
            .insert_resource(ClientOpts(opts))
            .init_resource::<IngestCounters>()
            .init_resource::<ReplicaState>()
            .init_resource::<CommandResults>()
            .init_resource::<JoinState>()
            .init_resource::<Acquisition>()
            .init_resource::<Presented>()
            .init_resource::<DebugState>();
        app.add_observer(on_connected).add_observer(on_disconnected);
        app.add_systems(
            PreUpdate,
            (receive_rejection, ingest_messages).after(MessageSystems::Receive),
        );
        app.add_systems(
            Update,
            (
                drive_acquisition,
                start_connection,
                update_phase,
                update_presented,
                write_frame_log,
            )
                .chain(),
        );
        app.add_systems(PostUpdate, auto_move);
        app.add_systems(Last, (rebuild_debug_state, exit_when_done).chain());
    }
}

/// The window of the presentation path. On wasm32 it binds to the page canvas `#turts` and
/// fits it to the parent; on native, `canvas` has no effect.
fn window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: "TuRTS".to_string(),
            canvas: Some("#turts".to_string()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    }
}

/// Builds the client app. Headless: `MinimalPlugins` at 60 Hz. Otherwise `DefaultPlugins` with
/// the window, the scene and the click input. `--run-secs` ends the run with `AppExit`.
pub fn build_app(opts: ClientOptions) -> App {
    let mut app = App::new();
    let headless = opts.headless;
    if headless {
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0),
        )));
    } else {
        app.add_plugins(DefaultPlugins.set(window_plugin()));
    }
    app.add_plugins(ClientPlugins {
        tick_duration: tick_duration(LINK_TICK_HZ),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(ClientPlugin(opts));
    if !headless {
        app.add_plugins((UnitRenderPlugin, ClickInputPlugin));
    }
    app
}
