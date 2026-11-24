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

server.ron is one RON record. The listener, package, URL and tick fields are required:
  game_package  Path of the package archive that turts-pack wrote.
  http_listen   Address of the HTTP listener, for example \"127.0.0.1:7000\".
  ws_listen     Address of the WebSocket listener, for example \"127.0.0.1:7001\".
  public_url    The HTTP address that clients use, for example \"http://127.0.0.1:7000\".
                The lockfile and the advertisement build their URLs from it.
  tick_hz       Simulation ticks per second. It must be above 0.
  static_dir    A folder that the HTTP listener also serves, for the browser page.
  bots          Optional list of (slot, profile, seed) public bot selections.
  bot_limits    Optional decision limits and projection_rows host ceiling; engine defaults apply.
  start_policy  Optional override: Immediate, AllSeated, or AllSeatedReady with countdown_secs.
  terrain_seed_file Optional private file with one unsigned decimal 64-bit terrain seed.
                Absent: generate a private seed from operating-system entropy.
  terrain_provenance Optional new private provenance file. Default: .turts-private/terrain-*.json.
                Both paths must stay outside static_dir. Files are never overwritten.

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
                  Also enable private session observations in the sibling PATH.sessions.jsonl (mode 0600).
                  This option is a local test hook; the file is not served over HTTP.
  --send-trace PATH
                  Write one CSV line for each session and state pump to PATH (mode 0600): the
                  frames sent, the frames not acknowledged, the backlog in ticks, the loss
                  declarations and the restarts. The first line names the fields.
                  This option is a local test hook; the file is not served over HTTP.
  --bot SLOT,PROFILE,SEED
                  Select a public bot profile, for example 1,skirmish:opponent,42.
                  Repeat this option for different bot slots.
  --computer-slot SLOT
                  Select the sole exported public bot profile. Ambiguous profiles are refused.
  --economy-log PATH
                  Write trusted economy audit JSONL to PATH. The file contains private data
                  for all players and is intended only for local test evidence.
  --command-script PATH
                  Apply fixed-tick JSON order envelopes without a network client.
  --order-log PATH
                  Write deterministic order state changes: tick,unit_id,state,reason.
  --synthetic-units N
                  Add N units of the first unit type of the package. A system gives each
                  one a new move target every 40 ticks. Use it for load runs.
  --run-ticks N   Exit with code 0 after N simulation ticks. The server may run a few ticks
                  past N.
  --help, -h      Print this text and exit with 0.

Environment:
  TURTS_MOVEMENT  Temporary test setting, read one time at start: `legacy` (default) or
                  `flock`. Every other value stops the server. It selects the movement
                  model of terrain moves. It is not in server.ron and not on the wire.

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
    #[serde(default)]
    pub start_policy: Option<turts_mod_api::StartPolicy>,
    /// Explicit public profiles. Loading a package does not activate its profiles.
    #[serde(default)]
    pub bots: Vec<BotConfig>,
    #[serde(default)]
    pub bot_limits: BotLimitsConfig,
    #[serde(default)]
    pub terrain_seed_file: Option<String>,
    #[serde(default)]
    pub terrain_provenance: Option<String>,
}

/// One explicitly selected bot actor. The seed does not use connection or wall time.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotConfig {
    pub slot: u8,
    pub profile: String,
    pub seed: u64,
}

/// Operator limits. Controller quotas must fit the selected decision limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BotLimitsConfig {
    pub queries: u32,
    pub candidates: u32,
    pub rows: u32,
    pub proposals: u32,
    pub commands: u32,
    pub affected_units: u32,
    pub pending_requests: u32,
    pub pending_accounts: u32,
    pub projection_rows: usize,
}
impl Default for BotLimitsConfig {
    fn default() -> Self {
        Self {
            queries: 32,
            candidates: 4096,
            rows: 1024,
            proposals: 32,
            commands: 8,
            affected_units: 256,
            pending_requests: 64,
            pending_accounts: 512,
            projection_rows: 65536,
        }
    }
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
        if config
            .start_policy
            .as_ref()
            .is_some_and(|policy| match policy {
                turts_mod_api::StartPolicy::Immediate => false,
                turts_mod_api::StartPolicy::AllSeated { countdown_secs }
                | turts_mod_api::StartPolicy::AllSeatedReady { countdown_secs } => {
                    *countdown_secs > 30
                }
            })
        {
            return Err("server.ron: start_policy countdown_secs must be 0 to 30".into());
        }
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
/// [--synthetic-units N] [--run-ticks N] [--economy-log PATH] [--command-script PATH]
/// [--order-log PATH] [--send-trace PATH]`.
#[allow(dead_code)] // Tasks 7b and 7c read the options that Task 7a does not.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    pub computer_slot: Option<u8>,
    pub bots: Vec<BotConfig>,
    pub config: PathBuf,
    pub tick_log: Option<PathBuf>,
    pub stats: Option<PathBuf>,
    pub synthetic_units: u32,
    pub run_ticks: Option<u32>,
    pub economy_log: Option<PathBuf>,
    pub command_script: Option<PathBuf>,
    pub order_log: Option<PathBuf>,
    pub send_trace: Option<PathBuf>,
}

/// Parse the arguments after the program name.
pub fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut config = None;
    let mut cli = Cli {
        computer_slot: None,
        bots: Vec::new(),
        config: PathBuf::new(),
        tick_log: None,
        stats: None,
        synthetic_units: 0,
        run_ticks: None,
        economy_log: None,
        command_script: None,
        order_log: None,
        send_trace: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--bot" => cli.bots.push(parse_bot(value(arg)?)?),
            "--computer-slot" => {
                if cli.computer_slot.is_some() {
                    return Err("--computer-slot may appear only once".into());
                }
                cli.computer_slot = Some(
                    u8::try_from(number(arg, value(arg)?)?)
                        .map_err(|_| "--computer-slot must fit in a slot number".to_string())?,
                )
            }
            "--tick-log" => cli.tick_log = Some(PathBuf::from(value(arg)?)),
            "--stats" => cli.stats = Some(PathBuf::from(value(arg)?)),
            "--synthetic-units" => cli.synthetic_units = number(arg, value(arg)?)?,
            "--run-ticks" => cli.run_ticks = Some(number(arg, value(arg)?)?),
            "--economy-log" => cli.economy_log = Some(PathBuf::from(value(arg)?)),
            "--command-script" => cli.command_script = Some(PathBuf::from(value(arg)?)),
            "--order-log" => cli.order_log = Some(PathBuf::from(value(arg)?)),
            "--send-trace" => cli.send_trace = Some(PathBuf::from(value(arg)?)),
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
    if cli.computer_slot.is_some() && !cli.bots.is_empty() {
        return Err("--computer-slot cannot be combined with --bot".into());
    }
    cli.config = config.ok_or_else(|| format!("no server.ron path\n{USAGE}"))?;
    Ok(cli)
}

fn number(name: &str, text: &str) -> Result<u32, String> {
    text.parse()
        .map_err(|_| format!("{name} needs a whole number, got `{text}`"))
}

fn parse_bot(value: &str) -> Result<BotConfig, String> {
    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 3 {
        return Err("--bot needs SLOT,PROFILE,SEED".into());
    }
    let slot = fields[0]
        .parse::<u8>()
        .map_err(|_| "--bot slot must fit in a slot number")?;
    turts_mod_api::ContentId::parse(fields[1]).map_err(|e| format!("--bot profile: {e}"))?;
    let seed = fields[2]
        .parse::<u64>()
        .map_err(|_| "--bot seed must be an unsigned 64-bit integer")?;
    Ok(BotConfig {
        slot,
        profile: fields[1].into(),
        seed,
    })
}

#[cfg(test)]
mod bot_tests {
    use super::*;
    #[test]
    fn explicit_profiles_keep_distinct_slots_names_and_full_width_seeds() {
        let args = [
            "server.ron",
            "--bot",
            "0,moda:custom,18446744073709551615",
            "--bot",
            "1,modb:other,42",
        ]
        .map(str::to_owned);
        let cli = parse_cli(&args).unwrap();
        assert_eq!(
            cli.bots,
            vec![
                BotConfig {
                    slot: 0,
                    profile: "moda:custom".into(),
                    seed: u64::MAX
                },
                BotConfig {
                    slot: 1,
                    profile: "modb:other".into(),
                    seed: 42
                }
            ]
        );
    }
    #[test]
    fn malformed_selection_and_mixed_legacy_alias_are_refused() {
        for argument in [
            "1",
            "1,unqualified,1",
            "-1,mod:profile,1",
            "1,mod:profile,-1",
            "1,mod:profile,18446744073709551616",
        ] {
            assert!(
                parse_cli(&["server.ron".into(), "--bot".into(), argument.into()]).is_err(),
                "{argument}"
            );
        }
        assert!(
            parse_cli(
                &[
                    "server.ron",
                    "--computer-slot",
                    "1",
                    "--bot",
                    "0,mod:profile,1"
                ]
                .map(str::to_owned)
            )
            .is_err()
        );
        assert!(
            parse_cli(
                &["server.ron", "--computer-slot", "1", "--computer-slot", "0"].map(str::to_owned)
            )
            .is_err()
        );
    }
}
