//! Native client binary.
//!
//! `turts_client --server URL --ws URL [--headless] [test hooks]`. `--help` lists the options
//! (`turts_client::options::USAGE`). Exit codes: 0 ended well, 1 the client rejected a package
//! or the join (the reason is on stderr), 2 the command line is wrong.

use std::process::ExitCode;

use turts_client::options::{USAGE, parse_cli, wants_help};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if wants_help(&args) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let options = match parse_cli(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("turts_client: {message}\nRun `turts_client --help` for the options.");
            return ExitCode::from(2);
        }
    };
    match turts_client::build_app(options).run() {
        bevy::app::AppExit::Success => ExitCode::SUCCESS,
        bevy::app::AppExit::Error(code) => ExitCode::from(code.get()),
    }
}
