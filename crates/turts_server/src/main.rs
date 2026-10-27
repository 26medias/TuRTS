//! Headless authoritative server: loads the game package, serves HTTP, and runs the fixed-tick
//! simulation with lightyear sessions.

mod boot;
mod config;
mod export;
mod http;
mod net;
mod outbound;
mod session;
mod stats;
mod synthetic;
mod ticklog;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bevy::app::{App, AppExit, FixedLast, PluginGroup, ScheduleRunnerPlugin};
use bevy::prelude::{MessageWriter, MinimalPlugins, Res, Resource};
use lightyear::prelude::server::ServerPlugins;
use turts_net_lightyear::{ProtocolPlugin, tick_duration};
use turts_sim::{SimPlugin, Tick as SimTick};

use crate::session::Advertised;
use turts_sim::initialize_completed_world;

fn main() {
    if let Err(message) = run() {
        eprintln!("turts_server: {message}");
        std::process::exit(1);
    }
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

    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1))));
    app.add_plugins(ServerPlugins {
        tick_duration: tick_duration(cfg.tick_hz),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(SimPlugin {
        tick_hz: cfg.tick_hz,
    });

    let booted = boot::boot(app.world_mut(), &cfg, &bytes).map_err(|error| error.to_string())?;
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
    outbound::install(&mut app);
    net::install(&mut app, cfg.http_addr(), ws_addr);
    ticklog::install(&mut app, &cli);
    stats::install(&mut app, &cli);
    synthetic::install(&mut app, &cli);
    initialize_completed_world(app.world_mut());
    if let Some(ticks) = cli.run_ticks {
        app.insert_resource(RunTicks(ticks));
        app.add_systems(FixedLast, exit_after_ticks);
    }

    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => Err(format!("the app stopped with exit code {code}")),
    }
}

/// `--run-ticks N`: ask the app to exit once the simulation has run N ticks. The app finishes
/// the frame first, so a frame that ran several catch-up ticks can end a few ticks past N.
fn exit_after_ticks(tick: Res<SimTick>, limit: Res<RunTicks>, mut exit: MessageWriter<AppExit>) {
    if tick.0 >= limit.0 {
        exit.write(AppExit::Success);
    }
}
