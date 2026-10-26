//! Headless authoritative server. Task 7b replaces this file with the full server.

mod boot;
mod config;
mod http;
mod net;
mod outbound;
mod session;
mod stats;
mod synthetic;
mod ticklog;

use std::sync::Arc;

fn main() {
    if let Err(message) = run() {
        eprintln!("turts_server: {message}");
        std::process::exit(1);
    }
}

/// Boot the package, then serve HTTP until the process ends.
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = config::read_config(&config::parse_cli(&args)?.config)?;
    let bytes = std::fs::read(&cfg.game_package)
        .map_err(|error| format!("cannot read `{}`: {error}", cfg.game_package))?;
    let mut app = turts_sim::headless_app(cfg.tick_hz);
    let booted = boot::boot(app.world_mut(), &cfg, &bytes).map_err(|error| error.to_string())?;
    let package = &booted.archive.manifest;
    eprintln!(
        "turts_server: package {} {}, lockfile {}",
        package.id, package.version, booted.lockfile_hash
    );
    let static_dir = cfg.static_dir.as_ref().map(std::path::PathBuf::from);
    http::spawn_http(cfg.http_addr(), Arc::new(booted), static_dir)
        .join()
        .map_err(|_| "the HTTP thread stopped".to_string())
}
