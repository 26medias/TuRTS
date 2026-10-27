//! Server configuration (server.ron) and the command line.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The last line of every command line error.
const USAGE: &str = "usage: turts_server <server.ron> [options]. Run `turts_server --help` for \
                     the options.";

/// The text of `turts_server --help`. A test reads this file and checks that every option and
/// every `HostConfig` field appears here.
pub const HELP: &str = "\
usage: turts_server <server.ron> [options]
       turts_server --help

Run the headless authoritative server. It loads the game package that server.ron names,
serves the package to clients over HTTP, and runs the match over WebSocket.

server.ron is one RON record. Every field is required, except static_dir:
  game_package  Path of the package archive that turts-pack wrote.
  http_listen   Address of the HTTP listener, for example \"127.0.0.1:7000\".
  ws_listen     Address of the WebSocket listener, for example \"127.0.0.1:7001\".
  public_url    The HTTP address that clients use, for example \"http://127.0.0.1:7000\".
                The lockfile and the advertisement build their URLs from it.
  tick_hz       Simulation ticks per second. It must be above 0.
  static_dir    A folder that the HTTP listener also serves, for the browser page.

HTTP endpoints (GET):
  /advertisement.json            The package name and version, the engine and protocol
                                 versions, and the URL and hash of the lockfile.
  /lockfile.json                 The packages that the match uses, with their hashes and URLs.
  /packages/<id>-<version>.tar   The package archive. The hash in the lockfile is its hash.
  Any other path comes from static_dir, when it is set.

Options:
  --tick-log PATH
                  Write one line for each unit and tick to PATH: tick,wall_ns,unit_id,x,y.
                  This option is a test hook. It is off when --synthetic-units is above 0.
  --stats PATH    Rewrite PATH with one line of JSON statistics every second and at exit.
                  This option is a test hook.
  --economy-log PATH
                  Write trusted economy audit JSONL to PATH. The file contains private data
                  for all players and is intended only for local test evidence.
  --synthetic-units N
                  Add N units of the first unit type of the package. A system gives each
                  one a new move target every 40 ticks. Use it for load runs.
  --run-ticks N   Exit with code 0 after N simulation ticks. The server may run a few ticks
                  past N.
  --help, -h      Print this text and exit with 0.

Exit codes:
  0  The server ended well (--run-ticks reached).
  1  The server could not start, or it stopped on an error. The reason is on stderr. An
     error in the package names the package, the file, and the field.";

/// Whether the command line asks for the help text.
pub fn wants_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

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
/// [--synthetic-units N] [--run-ticks N] [--economy-log PATH]`.
#[allow(dead_code)] // Tasks 7b and 7c read the options that Task 7a does not.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    pub config: PathBuf,
    pub tick_log: Option<PathBuf>,
    pub stats: Option<PathBuf>,
    pub synthetic_units: u32,
    pub run_ticks: Option<u32>,
    pub economy_log: Option<PathBuf>,
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
        economy_log: None,
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
            "--economy-log" => cli.economy_log = Some(PathBuf::from(value(arg)?)),
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
