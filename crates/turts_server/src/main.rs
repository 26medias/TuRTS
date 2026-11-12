//! Headless authoritative server: loads the game package, serves HTTP, and runs the fixed-tick
//! simulation with lightyear sessions.

mod boot;
mod command_script;
mod config;
mod economylog;
mod export;
mod http;
mod net;
mod orderlog;
mod orders;
mod outbound;
mod session;
mod session_diagnostics;
mod stats;
mod synthetic;
mod ticklog;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bevy::app::{App, AppExit, FixedLast, PluginGroup, ScheduleRunnerPlugin};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::{MessageWriter, MinimalPlugins, Res, Resource};
use lightyear::prelude::server::ServerPlugins;
use turts_net_lightyear::{ProtocolPlugin, tick_duration};
use turts_sim::{AuditTrail, SimPlugin, Tick as SimTick, audit_state};

use crate::session::Advertised;
use turts_sim::initialize_completed_world;

fn main() {
    if let Err(message) = run() {
        eprintln!("turts_server: {message}");
        std::process::exit(1);
    }
}

/// The app with its time, lightyear and simulation plugins: the one the process runs. The frame
/// clamp and the timestep that bound a catch-up burst come from here (see `frame_tests`).
fn base_app(tick_hz: u16) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1))));
    app.add_plugins(ServerPlugins {
        tick_duration: tick_duration(tick_hz),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(SimPlugin { tick_hz });
    app
}

/// Stop the app with exit code 0 after this many ticks (`--run-ticks`).
#[derive(Resource)]
struct RunTicks(u32);

/// Boot the package, serve HTTP and the WebSocket, and run until the process ends.
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if config::wants_help(&args) {
        println!("{}", config::HELP);
        return Ok(());
    }
    let cli = config::parse_cli(&args)?;
    let cfg = config::read_config(&cli.config)?;
    let bytes = std::fs::read(&cfg.game_package)
        .map_err(|error| format!("cannot read `{}`: {error}", cfg.game_package))?;
    let ws_addr: SocketAddr = cfg
        .ws_listen
        .parse()
        .map_err(|error| format!("ws_listen `{}`: {error}", cfg.ws_listen))?;

    let mut app = base_app(cfg.tick_hz);

    let booted = boot::boot(app.world_mut(), &cfg, &bytes).map_err(|error| error.to_string())?;
    let server_catalog = booted
        .archive
        .content()
        .map_err(|error| error.to_string())?;
    app.insert_resource(orders::ServerCatalog(server_catalog));
    app.insert_resource(stats::generate_match_id());
    let package = &booted.archive.manifest;
    eprintln!(
        "turts_server: package {} {}, lockfile {}",
        package.id, package.version, booted.lockfile_hash
    );
    let advertised = Advertised {
        lockfile_hash: booted.lockfile_hash.clone(),
        tick_hz: cfg.tick_hz,
    };
    let static_dir = cfg.static_dir.as_ref().map(PathBuf::from);
    // The HTTP thread runs for the life of the process; the app loop below keeps the process up.
    drop(http::spawn_http(
        cfg.http_addr(),
        Arc::new(booted),
        static_dir,
    ));

    session::install(&mut app, advertised);
    command_script::install(&mut app, cli.command_script.as_deref())?;
    orderlog::install(&mut app, cli.order_log.as_deref())?;
    outbound::install(&mut app);
    net::install(&mut app, cfg.http_addr(), ws_addr);
    ticklog::install(&mut app, &cli);
    stats::install(&mut app, &cli)?;
    synthetic::install(&mut app, &cli);
    initialize_completed_world(app.world_mut());
    if let Some(path) = cli.economy_log.as_ref() {
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let initial = audit_state(app.world_mut());
        let trail = app.world().resource::<AuditTrail>();
        let sink = economylog::EconomyLogSink::create(path, trail, initial)
            .map_err(|error| format!("cannot start economy log {}: {error}", path.display()))?;
        app.insert_resource(sink);
    }
    if let Some(ticks) = cli.run_ticks {
        app.insert_resource(RunTicks(ticks));
        app.add_systems(
            FixedLast,
            (economylog::capture_completed_tick, exit_after_ticks).chain(),
        );
    } else {
        app.add_systems(FixedLast, economylog::capture_completed_tick);
    }

    let result = match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => Err(format!("the app stopped with exit code {code}")),
    };
    if let Some(error) = app
        .world()
        .get_resource::<economylog::EconomyLogSink>()
        .and_then(|sink| sink.error.as_ref())
    {
        return Err(format!("economy log evidence failed: {error}"));
    }
    result
}

/// `--run-ticks N`: ask the app to exit once the simulation has run N ticks. The app finishes
/// the frame first, so a frame that ran several catch-up ticks can end a few ticks past N.
fn exit_after_ticks(tick: Res<SimTick>, limit: Res<RunTicks>, mut exit: MessageWriter<AppExit>) {
    if tick.0 >= limit.0 {
        exit.write(AppExit::Success);
    }
}

/// The client reorder grace (`turts_client::replica::max_catchup_ticks`) rests on two facts about
/// this app. These tests read them from the app that `run` builds, so a change goes red here.
#[cfg(test)]
mod frame_tests {
    use super::*;
    use bevy::prelude::{PostUpdate, Schedules};
    use bevy::time::{Fixed, Time, Virtual};

    /// The app after its plugins finished and one frame ran (the Startup systems have run).
    fn running_app(tick_hz: u16) -> App {
        let mut app = base_app(tick_hz);
        app.finish();
        app.cleanup();
        app.update();
        app
    }

    #[test]
    fn server_frame_clamp_and_timestep_are_pinned() {
        for hz in [20u16, 30, 60] {
            let app = running_app(hz);
            let virtual_time = app.world().resource::<Time<Virtual>>();
            assert_eq!(
                virtual_time.max_delta(),
                Duration::from_millis(250),
                "Time<Virtual> max_delta of the server app: the grace assumes 250 ms"
            );
            assert_eq!(
                app.world().resource::<Time<Fixed>>().timestep(),
                tick_duration(hz),
                "Time<Fixed> timestep of the server app is the tick duration"
            );
        }
    }

    /// One flush for each frame: lightyear sends in `PostUpdate`, once a frame, never in the fixed
    /// schedules (`lightyear_transport-0.31.0/src/plugin.rs:674-679`, `Self::buffer_send`).
    #[test]
    fn lightyear_flushes_once_a_frame_in_post_update() {
        let app = running_app(20);
        let schedules = app.world().resource::<Schedules>();
        let sends = |label| {
            schedules
                .iter()
                .filter(|(l, _)| format!("{l:?}") == label)
                .map(|(_, schedule)| {
                    schedule
                        .systems()
                        .map(|systems| {
                            systems
                                .filter(|(_, system)| {
                                    system.name().to_string().contains("buffer_send")
                                })
                                .count()
                        })
                        .unwrap_or(0)
                })
                .sum::<usize>()
        };
        assert_eq!(
            sends(format!("{:?}", PostUpdate)),
            1,
            "one transport send in PostUpdate"
        );
        for fixed in [
            "FixedFirst",
            "FixedPreUpdate",
            "FixedUpdate",
            "FixedPostUpdate",
            "FixedLast",
        ] {
            assert_eq!(sends(fixed.to_string()), 0, "no transport send in {fixed}");
        }
    }
}
