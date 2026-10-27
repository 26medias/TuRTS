//! Test harness: pack a package, start the server binary, run the client binary.
//!
//! `logs` reads the server tick log, the client frame log and the stats file. `oracle` checks
//! a frame log against the tick log (design section 7, T5). Not every test file uses every
//! function.
#![allow(dead_code)]

pub mod logs;
pub mod oracle;

use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use std::collections::BTreeMap;

use turts_mod_runtime::pack::{pack_dir, pack_files};

/// How long `spawn_server` waits for the HTTP endpoint.
const READY_TIMEOUT: Duration = Duration::from_secs(10);

/// A running `turts_server` process. Dropping it kills the process.
pub struct ServerProc {
    pub child: Child,
    /// For example `http://127.0.0.1:41000`.
    pub http_url: String,
    /// The WebSocket listen address without a scheme, for example `127.0.0.1:41001`. A client
    /// connects to `ws://<ws_addr>`.
    pub ws_addr: String,
    pub dir: PathBuf,
    pub tick_log: PathBuf,
    pub stats: PathBuf,
    pub economy_log: Option<PathBuf>,
}

impl Drop for ServerProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A free local TCP port: bind port 0, read the port, drop the socket.
pub fn free_port() -> u16 {
    free_ports::<1>()[0]
}

/// `N` different free ports. The sockets stay open until all are chosen.
fn free_ports<const N: usize>() -> [u16; N] {
    let sockets: Vec<TcpListener> = (0..N)
        .map(|_| TcpListener::bind("127.0.0.1:0").expect("a local port binds"))
        .collect();
    std::array::from_fn(|index| sockets[index].local_addr().unwrap().port())
}

/// The `mods` directory of the repository.
pub fn mods_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

/// A fresh empty directory for one test, under the target directory of the test run.
pub fn test_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the test directory is created");
    dir
}

/// Pack `mods/<mods_dir_name>` into `<into>/<mods_dir_name>.tar` and return that path.
pub fn pack(mods_dir_name: &str, into: &Path) -> PathBuf {
    let bytes = pack_dir(&mods_dir().join(mods_dir_name)).expect("the package packs");
    fs::create_dir_all(into).expect("the target directory is created");
    let tar = into.join(format!("{mods_dir_name}.tar"));
    fs::write(&tar, bytes).expect("the archive is written");
    tar
}

/// Pack `mods/<mods_dir_name>` with the file at `relative` replaced by `text`, into
/// `<into>/<mods_dir_name>.tar`, and return that path. `mods/` is not changed.
pub fn pack_with(mods_dir_name: &str, relative: &str, text: &str, into: &Path) -> PathBuf {
    let root = mods_dir().join(mods_dir_name);
    let mut files = BTreeMap::new();
    collect_files(&root, &root, &mut files);
    assert!(
        files.contains_key(relative),
        "mods/{mods_dir_name} has no {relative}"
    );
    files.insert(relative.to_string(), text.as_bytes().to_vec());
    let bytes = pack_files(&files).expect("the package packs");
    fs::create_dir_all(into).expect("the target directory is created");
    let tar = into.join(format!("{mods_dir_name}.tar"));
    fs::write(&tar, bytes).expect("the archive is written");
    tar
}

/// Every file under `dir`, by its path relative to `root` with `/` separators.
fn collect_files(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(dir).expect("the package directory is read") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else {
            let relative = path.strip_prefix(root).expect("under the root");
            let key = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.insert(key, fs::read(&path).expect("a package file is read"));
        }
    }
}

/// Start `turts_server` on two free ports with `package_tar` as the game package. The call
/// writes `server.ron` into a new directory, passes `--tick-log` and `--stats`, then adds
/// `extra`. It returns when `GET /advertisement.json` answers; it panics after 10 seconds, or
/// at once when the process exits early. The process stops when the value drops.
pub fn expect_server_boot_failure(package_tar: &Path) -> String {
    let dir = test_dir("expected-server-boot-failure");
    let [http_port, ws_port] = free_ports::<2>();
    let http_listen = SocketAddr::from(([127, 0, 0, 1], http_port));
    let ws_listen = SocketAddr::from(([127, 0, 0, 1], ws_port));
    let public_url = format!("http://{http_listen}");
    let config = dir.join("server.ron");
    fs::write(
        &config,
        format!(
            "(\n    game_package: {:?},\n    http_listen: \"{http_listen}\",\n    ws_listen: \"{ws_listen}\",\n    public_url: \"{public_url}\",\n    tick_hz: 20,\n)\n",
            package_tar.display().to_string()
        ),
    )
    .expect("server.ron is written");
    let output = Command::new(env!("CARGO_BIN_EXE_turts_server"))
        .arg(&config)
        .output()
        .expect("turts_server process starts");
    assert!(
        !output.status.success(),
        "invalid package boot unexpectedly succeeded"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn spawn_server(package_tar: &Path, extra: &[&str]) -> ServerProc {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = test_dir(&format!("server_{}_{id}", std::process::id()));
    let [http_port, ws_port] = free_ports::<2>();
    let http_listen = SocketAddr::from(([127, 0, 0, 1], http_port));
    let ws_listen = SocketAddr::from(([127, 0, 0, 1], ws_port));
    let http_url = format!("http://{http_listen}");
    let config = dir.join("server.ron");
    fs::write(
        &config,
        format!(
            "(\n    game_package: {:?},\n    http_listen: \"{http_listen}\",\n    \
             ws_listen: \"{ws_listen}\",\n    public_url: \"{http_url}\",\n    tick_hz: 20,\n)\n",
            package_tar.display().to_string()
        ),
    )
    .expect("server.ron is written");
    let tick_log = dir.join("ticks.log");
    let stats = dir.join("stats.json");
    let log = fs::File::create(dir.join("server.log")).expect("the server log is created");
    let child = Command::new(env!("CARGO_BIN_EXE_turts_server"))
        .arg(&config)
        .arg("--tick-log")
        .arg(&tick_log)
        .arg("--stats")
        .arg(&stats)
        .args(extra)
        .stdout(Stdio::from(log.try_clone().expect("the log handle clones")))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("turts_server starts");
    let economy_log = extra
        .windows(2)
        .find(|pair| pair[0] == "--economy-log")
        .map(|pair| PathBuf::from(pair[1]));
    let mut server = ServerProc {
        child,
        http_url,
        ws_addr: ws_listen.to_string(),
        dir,
        tick_log,
        stats,
        economy_log,
    };
    server.wait_until_ready();
    server
}

impl ServerProc {
    fn wait_until_ready(&mut self) {
        let url = format!("{}/advertisement.json", self.http_url);
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("the child state is read") {
                panic!("turts_server exited early ({status}):\n{}", self.log());
            }
            if ureq::get(&url).call().is_ok() {
                return;
            }
            if started.elapsed() > READY_TIMEOUT {
                panic!("turts_server did not answer {url} in 10 s:\n{}", self.log());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// The text that the server wrote to stdout and stderr.
    pub fn log(&self) -> String {
        fs::read_to_string(self.dir.join("server.log")).unwrap_or_default()
    }
}

/// The path of the `turts_client` binary, next to `turts_server`. Panics when it is absent.
pub fn client_exe() -> PathBuf {
    let server = Path::new(env!("CARGO_BIN_EXE_turts_server"));
    let name = format!("turts_client{}", std::env::consts::EXE_SUFFIX);
    let exe = server.with_file_name(name);
    assert!(
        exe.exists(),
        "build turts_client first (scripts/acceptance.sh): {} is absent",
        exe.display()
    );
    exe
}

/// Start `turts_client` with `args`. Its stdout is discarded.
pub fn run_client(args: &[&str]) -> Child {
    Command::new(client_exe())
        .args(args)
        .stdout(Stdio::null())
        .spawn()
        .expect("turts_client starts")
}

/// Wait up to `timeout` for the client to exit, then read the JSON file that it wrote to
/// `out`. Panics when the client does not exit in time (it is killed first) or when the file
/// is absent or not JSON.
pub fn wait_json(mut child: Child, out: &Path, timeout: Duration) -> serde_json::Value {
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child state is read") {
            break status;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            panic!("turts_client did not exit in {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let text = fs::read_to_string(out).unwrap_or_else(|error| {
        panic!(
            "turts_client ({status}) wrote no {}: {error}",
            out.display()
        )
    });
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not JSON ({status}): {error}", out.display()))
}
