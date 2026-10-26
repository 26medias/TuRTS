//! Server configuration (server.ron) and the command line.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const USAGE: &str = "usage: turts_server <server.ron> [--tick-log PATH] [--stats PATH] \
                     [--synthetic-units N] [--run-ticks N]";

/// The fields of `server.ron` (design 4.4). `static_dir` is the plan's optional extra: the
/// folder that the HTTP server serves for the browser page.
#[derive(Debug, Clone, Deserialize)]
pub struct HostConfig {
    /// Path of the game package archive.
    pub game_package: String,
    pub http_listen: String,
    pub ws_listen: String,
    pub public_url: String,
    pub tick_hz: u16,
    #[serde(default)]
    pub static_dir: Option<String>,
}

impl HostConfig {
    /// Read the text of a `server.ron` and check it. `tick_hz` is above 0, and both listen
    /// addresses are socket addresses.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        // `implicit_some` lets a file write `static_dir: "dir"` without `Some(..)`.
        let config: Self = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|error| format!("server.ron: {error}"))?;
        if config.tick_hz == 0 {
            return Err("server.ron: tick_hz must be above 0".to_string());
        }
        for (field, value) in [
            ("http_listen", &config.http_listen),
            ("ws_listen", &config.ws_listen),
        ] {
            value.parse::<SocketAddr>().map_err(|error| {
                format!("server.ron: {field} `{value}` is not a socket address: {error}")
            })?;
        }
        Ok(config)
    }

    /// The HTTP listen address. `from_ron` checked that it parses.
    pub fn http_addr(&self) -> SocketAddr {
        self.http_listen
            .parse()
            .expect("from_ron checked http_listen")
    }
}

/// Read and check the `server.ron` at `path`.
pub fn read_config(path: &Path) -> Result<HostConfig, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
    HostConfig::from_ron(&text)
}

/// The command line: `turts_server <server.ron> [--tick-log P] [--stats P]
/// [--synthetic-units N] [--run-ticks N]`.
#[allow(dead_code)] // Tasks 7b and 7c read the options that Task 7a does not.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    pub config: PathBuf,
    pub tick_log: Option<PathBuf>,
    pub stats: Option<PathBuf>,
    pub synthetic_units: u32,
    pub run_ticks: Option<u32>,
}

/// Parse the arguments after the program name.
pub fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut config = None;
    let mut cli = Cli {
        config: PathBuf::new(),
        tick_log: None,
        stats: None,
        synthetic_units: 0,
        run_ticks: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--tick-log" => cli.tick_log = Some(PathBuf::from(value(arg)?)),
            "--stats" => cli.stats = Some(PathBuf::from(value(arg)?)),
            "--synthetic-units" => cli.synthetic_units = number(arg, value(arg)?)?,
            "--run-ticks" => cli.run_ticks = Some(number(arg, value(arg)?)?),
            option if option.starts_with("--") => {
                return Err(format!("unknown option `{option}`\n{USAGE}"));
            }
            path => {
                if config.replace(PathBuf::from(path)).is_some() {
                    return Err(format!("more than one server.ron path\n{USAGE}"));
                }
            }
        }
    }
    cli.config = config.ok_or_else(|| format!("no server.ron path\n{USAGE}"))?;
    Ok(cli)
}

fn number(name: &str, text: &str) -> Result<u32, String> {
    text.parse()
        .map_err(|_| format!("{name} needs a whole number, got `{text}`"))
}
