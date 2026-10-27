//! Client options: the command line (native) and the page query (browser).

use std::path::PathBuf;

/// What a client run needs and the test hooks of the native binary (plan R3).
#[derive(Clone, Debug)]
pub struct ClientOptions {
    /// Base URL of the server's HTTP endpoints.
    pub server_url: String,
    /// WebSocket address of the server.
    pub ws_url: String,
    pub headless: bool,
    /// A fixed player id. `None` means a random id for the session.
    pub player_id: Option<u64>,
    /// Test hook: write one record per frame to this path.
    pub frame_log: Option<PathBuf>,
    /// Test hook: write the final report to this path.
    pub out: Option<PathBuf>,
    /// Test hook: send a `MoveTo` to this target.
    pub auto_move: Option<(f32, f32)>,
    /// Test hook: ticks after `Welcome` before the automatic move.
    pub auto_move_after_ticks: u32,
    /// Test hook: the unit id for the automatic move.
    pub move_unit: Option<u32>,
    /// Test hook: send this lockfile hash in `Hello` instead of the real one.
    pub hello_hash: Option<String>,
    /// Test hook: stop after this many seconds.
    pub run_secs: Option<f32>,
    /// Test hook: ignore the `WorldDelta` at these tick offsets from the `Welcome` tick.
    pub drop_delta_offsets: Vec<u32>,
    /// Test hook: ignore the first `WorldDelta` that arrives.
    pub drop_arrival_delta: bool,
    /// Test hook: hold each `Snapshot` that answers a resync request back by this many ticks
    /// of `WorldDelta`.
    pub hold_resync_snapshots: u32,
    /// Test hook: send this wire protocol version in `Hello` instead of `NET_PROTOCOL_VERSION`.
    pub protocol_version: Option<u32>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            ws_url: String::new(),
            headless: false,
            player_id: None,
            frame_log: None,
            out: None,
            auto_move: None,
            auto_move_after_ticks: 5,
            move_unit: None,
            hello_hash: None,
            run_secs: None,
            drop_delta_offsets: Vec::new(),
            drop_arrival_delta: false,
            hold_resync_snapshots: 0,
            protocol_version: None,
        }
    }
}

/// The text of `turts_client --help`: every option and test hook, and the exit codes. A test
/// reads the `parse_cli` source and checks that each flag appears here.
pub const USAGE: &str = "\
usage: turts_client --server URL --ws URL [options]
       turts_client --help

Join the match of a server and present it. This is the native client. The browser client
reads ?server=URL&ws=URL from the page address instead.

Required:
  --server URL    Base URL of the server HTTP endpoints, for example http://127.0.0.1:7000.
  --ws URL        WebSocket address of the server, for example ws://127.0.0.1:7001.

Options:
  --headless      Run with no window and no rendering.
  --player-id N   Use N as the player id. The default is a random id for each run.
  --help, -h      Print this text and exit with 0.

Test hooks:
  --run-secs S    Stop after S seconds. Without it, a joined client runs until it is killed.
  --out PATH      When the run ends, write the final report to PATH as JSON. The report
                  holds the join result, the command results and the final positions.
  --frame-log PATH
                  Write one record for each presented frame to PATH.
  --move X,Y      Send one MoveTo to the point X,Y. It moves all the units that the slot of
                  this client owns. Add --move-unit to move one unit instead.
  --move-unit ID  Move only the unit with this unit id. The unit does not have to be yours:
                  the server rejects the command for a unit of another slot.
  --move-after-ticks N
                  Wait N ticks after Welcome before the move. The default is 5.
  --hello-hash H  Send H as the lockfile hash in Hello. The server rejects a wrong hash.
  --protocol-version N
                  Send N as the wire protocol version in Hello.
  --drop-delta-ticks a,b,c
                  Ignore the WorldDelta of the ticks a, b and c. The numbers are offsets
                  from the Welcome tick.
  --drop-arrival-delta
                  Ignore the first WorldDelta that puts a unit at the --move target.
  --hold-resync-snapshots N
                  Hold each Snapshot that answers a resync request back by N ticks.

Exit codes:
  0  The run ended well: --run-secs passed, or the window closed.
  1  The client rejected the advertisement, a package, or the join. The reason is on
     stderr and in the --out report. A client with a window exits with 1 only when
     --run-secs ends.
  2  The command line is wrong.";

/// Whether the command line asks for the usage text.
pub fn wants_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

/// Read the command line, without the program name. [`USAGE`] lists the options.
///
/// `--server` and `--ws` are required. The `--drop-delta-ticks` values are offsets from the
/// `Welcome` tick. An unknown flag, a missing value, or a value that does not parse is an error
/// that names the flag.
pub fn parse_cli(args: &[String]) -> Result<ClientOptions, String> {
    let mut options = ClientOptions::default();
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let mut value = || {
            rest.next()
                .map(String::as_str)
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--server" => options.server_url = value()?.to_string(),
            "--ws" => options.ws_url = value()?.to_string(),
            "--headless" => options.headless = true,
            "--player-id" => options.player_id = Some(number(flag, value()?)?),
            "--frame-log" => options.frame_log = Some(PathBuf::from(value()?)),
            "--out" => options.out = Some(PathBuf::from(value()?)),
            "--move" => options.auto_move = Some(point(flag, value()?)?),
            "--move-after-ticks" => options.auto_move_after_ticks = number(flag, value()?)?,
            "--move-unit" => options.move_unit = Some(number(flag, value()?)?),
            "--hello-hash" => options.hello_hash = Some(value()?.to_string()),
            "--run-secs" => options.run_secs = Some(number(flag, value()?)?),
            "--drop-delta-ticks" => {
                options.drop_delta_offsets = value()?
                    .split(',')
                    .map(|part| number(flag, part))
                    .collect::<Result<_, _>>()?;
            }
            "--drop-arrival-delta" => options.drop_arrival_delta = true,
            "--hold-resync-snapshots" => options.hold_resync_snapshots = number(flag, value()?)?,
            "--protocol-version" => options.protocol_version = Some(number(flag, value()?)?),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.server_url.is_empty() {
        return Err("--server is required".to_string());
    }
    if options.ws_url.is_empty() {
        return Err("--ws is required".to_string());
    }
    Ok(options)
}

/// Read the page query of the browser client: `?server=<url>&ws=<url>`. Both keys are
/// required, and the error names the first one that is missing. A value may be percent encoded.
/// Other keys are ignored.
pub fn options_from_query(query: &str) -> Result<ClientOptions, String> {
    let mut options = ClientOptions::default();
    for pair in query.trim_start_matches('?').split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "server" => options.server_url = percent_decode(value)?,
            "ws" => options.ws_url = percent_decode(value)?,
            _ => {}
        }
    }
    if options.server_url.is_empty() {
        return Err("the page query has no `server` key".to_string());
    }
    if options.ws_url.is_empty() {
        return Err("the page query has no `ws` key".to_string());
    }
    Ok(options)
}

/// A random player id for one session. It is a placeholder identity (design 5, `Hello`).
/// It needs no `getrandom` dependency: native uses the random keys of the std `RandomState`;
/// the browser uses `Math.random`.
pub fn random_player_id() -> u64 {
    #[cfg(not(target_family = "wasm"))]
    {
        use std::hash::{BuildHasher, Hasher};
        std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish()
    }
    #[cfg(target_family = "wasm")]
    {
        let half = || (js_sys::Math::random() * 4_294_967_296.0) as u64;
        (half() << 32) | half()
    }
}

fn number<T: std::str::FromStr>(flag: &str, text: &str) -> Result<T, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("{flag}: `{text}` is not a valid number"))
}

fn point(flag: &str, text: &str) -> Result<(f32, f32), String> {
    let (x, y) = text
        .split_once(',')
        .ok_or_else(|| format!("{flag}: `{text}` must be X,Y"))?;
    Ok((number(flag, x)?, number(flag, y)?))
}

/// Decode `%XX` escapes. Any other character stays as it is.
fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let digits = bytes
                .get(i + 1..i + 3)
                .and_then(|pair| std::str::from_utf8(pair).ok())
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(|| format!("`{text}` has a bad % escape"))?;
            out.push(digits);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| format!("`{text}` is not valid UTF-8 after decoding"))
}
