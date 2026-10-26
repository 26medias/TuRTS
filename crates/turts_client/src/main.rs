//! Native client binary.
//!
//! `turts_client --server URL --ws URL [--headless] [test hooks]`. The hooks are listed in
//! `turts_client::options::parse_cli`.

use std::process::ExitCode;

use turts_client::options::parse_cli;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse_cli(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("turts_client: {message}");
            return ExitCode::from(2);
        }
    };
    match turts_client::build_app(options).run() {
        bevy::app::AppExit::Success => ExitCode::SUCCESS,
        bevy::app::AppExit::Error(code) => ExitCode::from(code.get()),
    }
}
