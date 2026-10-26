//! M1 verify, B3: the headless client exits with a non-zero code and prints the error on stderr
//! when it rejects the advertisement, the lockfile or a package. The error names the file and
//! the field, as `ModError` prints them. The `--out` report keeps the same text.
//!
//! The test starts the real `turts_client` binary against a small HTTP server of its own. The
//! server serves a package, a lockfile and an advertisement, with one defect for each case. The
//! client stops at acquisition, so it never opens the WebSocket. Test code may name the
//! example packages.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::lock::write_lockfile;
use turts_mod_runtime::pack::pack_files;
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION};

/// How long the test waits for the client to exit by itself.
const EXIT_LIMIT: Duration = Duration::from_secs(20);
/// The `--run-secs` of the cases that set it. A client that rejects a package exits long before.
const RUN_SECS: u64 = 8;

fn acme_files() -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).expect("under root");
                let key = relative.to_str().expect("utf-8 path").replace('\\', "/");
                out.insert(key, fs::read(&path).expect("read file"));
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/acme");
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

/// A server that answers `GET <path>` with the bytes of its route, and 404 for any other path.
/// The thread stops when the value drops.
struct FakeServer {
    base: String,
    stop: Arc<AtomicBool>,
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn respond(mut stream: TcpStream, routes: &BTreeMap<String, Vec<u8>>) {
    let mut request_line = String::new();
    let mut reader = BufReader::new(stream.try_clone().expect("the stream clones"));
    reader.read_line(&mut request_line).expect("a request line");
    // Read the headers up to the empty line.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("a header line") == 0 || line.trim().is_empty() {
            break;
        }
    }
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let (status, body) = match routes.get(path) {
        Some(body) => ("200 OK", body.as_slice()),
        None => ("404 Not Found", &b""[..]),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

/// Bind a port first, so the caller can put the URL into the route bodies. `routes` gets the
/// base URL and returns the routes.
fn serve(routes: impl FnOnce(&str) -> BTreeMap<String, Vec<u8>>) -> FakeServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port binds");
    listener.set_nonblocking(true).expect("non-blocking");
    let base = format!("http://{}", listener.local_addr().unwrap());
    let routes = routes(&base);
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false).expect("blocking stream");
                    respond(stream, &routes);
                }
                Err(_) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    });
    FakeServer { base, stop }
}

/// What the server serves for one case.
struct Case {
    /// The files of the package. The lock entry describes the archive of these files.
    files: BTreeMap<String, Vec<u8>>,
    /// Serve these bytes for the package instead of the real archive (the hash then differs).
    served_package: Option<Vec<u8>>,
    /// Advertise this lockfile hash instead of the real one.
    advertised_lockfile_hash: Option<String>,
}

fn serve_case(case: Case) -> FakeServer {
    let archive_bytes = pack_files(&case.files).expect("pack in memory");
    let archive = PackageArchive::parse(&archive_bytes).expect("the case package parses");
    serve(|base| {
        let entry = archive.lock_entry(vec![format!("{base}/packages/acme.tar")]);
        let lockfile = write_lockfile(&turts_mod_api::Lockfile {
            packages: vec![entry],
        });
        let advertisement = Advertisement {
            mod_id: "acme".to_string(),
            mod_version: "0.1.0".to_string(),
            engine_version: turts_mod_api::ENGINE_API_VERSION.to_string(),
            protocol_version: NET_PROTOCOL_VERSION,
            lockfile_url: format!("{base}/lockfile.json"),
            lockfile_hash: case
                .advertised_lockfile_hash
                .clone()
                .unwrap_or_else(|| sha256_hex(&lockfile)),
        };
        BTreeMap::from([
            (
                "/advertisement.json".to_string(),
                serde_json::to_vec(&advertisement).unwrap(),
            ),
            ("/lockfile.json".to_string(), lockfile),
            (
                "/packages/acme.tar".to_string(),
                case.served_package.clone().unwrap_or(archive_bytes),
            ),
        ])
    })
}

struct Run {
    code: Option<i32>,
    stderr: String,
    elapsed: Duration,
    out: PathBuf,
}

/// Run the headless client against `server`. `run_secs` is `--run-secs`, or none. The client
/// is killed when it does not exit within the limit.
fn run_client(test: &str, server: &FakeServer, run_secs: Option<u64>) -> Run {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the test directory is created");
    let out = dir.join("client.json");
    let stderr_path = dir.join("stderr.txt");
    let mut command = Command::new(env!("CARGO_BIN_EXE_turts_client"));
    command
        .args(["--headless", "--server", &server.base])
        .args(["--ws", "ws://127.0.0.1:1", "--out"])
        .arg(&out);
    if let Some(secs) = run_secs {
        command.args(["--run-secs", &secs.to_string()]);
    }
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::null())
        .stderr(fs::File::create(&stderr_path).expect("stderr file"))
        .spawn()
        .expect("turts_client starts");
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child state is read") {
            break Some(status);
        }
        if started.elapsed() > EXIT_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Run {
        code: status.and_then(|status| status.code()),
        stderr: fs::read_to_string(&stderr_path).unwrap_or_default(),
        elapsed: started.elapsed(),
        out,
    }
}

/// The client exited by itself with code 1, soon, and printed one line that starts with
/// `turts_client: ` and `expect`. The `--out` report holds the same reason.
fn assert_rejected(run: &Run, expect_start: &str, expect_in_text: &[&str]) {
    assert_eq!(run.code, Some(1), "exit code; stderr: {:?}", run.stderr);
    assert!(
        run.stderr
            .starts_with(&format!("turts_client: {expect_start}")),
        "{:?}",
        run.stderr
    );
    for text in expect_in_text {
        assert!(run.stderr.contains(text), "{text} not in {:?}", run.stderr);
    }
    assert_eq!(run.stderr.trim_end().lines().count(), 1, "{:?}", run.stderr);
    assert!(
        run.elapsed < Duration::from_secs(RUN_SECS / 2),
        "the client took {:?} to reject",
        run.elapsed
    );
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&run.out).expect("the --out report exists"))
            .expect("the report is JSON");
    let reason = report["rejected_reason"].as_str().expect("a reason");
    assert_eq!(
        run.stderr.trim_end(),
        format!("turts_client: {reason}"),
        "stderr and the report disagree"
    );
}

#[test]
fn a_package_that_does_not_match_its_hash_ends_the_client_with_the_error() {
    let files = acme_files();
    let mut served = pack_files(&files).unwrap();
    served[520] ^= 1;
    let server = serve_case(Case {
        files,
        served_package: Some(served),
        advertised_lockfile_hash: None,
    });
    let run = run_client("reject_package_hash", &server, Some(RUN_SECS));
    assert_rejected(&run, "acme/-: hash: ", &["sha256:"]);
}

#[test]
fn a_package_with_a_bad_visual_ends_the_client_with_the_file_and_the_field() {
    // The hash matches the bytes, so only the contents are wrong: a visual of a unit that no
    // unit file defines.
    let mut files = acme_files();
    let manifest = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    let edited = manifest.replace(
        "visuals: [\"visuals/scout.ron\"]",
        "visuals: [\"visuals/scout.ron\", \"visuals/ghost.ron\"]",
    );
    assert_ne!(manifest, edited);
    files.insert("manifest.ron".to_string(), edited.into_bytes());
    files.insert(
        "visuals/ghost.ron".to_string(),
        b"(target: Unit(\"acme:ghost\"), shape: Capsule(radius: 0.3, height: 1.0))".to_vec(),
    );
    let server = serve_case(Case {
        files,
        served_package: None,
        advertised_lockfile_hash: None,
    });
    let run = run_client("reject_package_contents", &server, Some(RUN_SECS));
    assert_rejected(
        &run,
        "acme/visuals/ghost.ron: target: ",
        &["`acme:ghost`", "(allowed: acme:scout)"],
    );
}

#[test]
fn a_lockfile_that_does_not_match_the_advertised_hash_ends_the_client() {
    let server = serve_case(Case {
        files: acme_files(),
        served_package: None,
        advertised_lockfile_hash: Some(sha256_hex(b"another lockfile")),
    });
    let run = run_client("reject_lockfile_hash", &server, Some(RUN_SECS));
    assert_rejected(&run, "lockfile hash mismatch", &["sha256:"]);
}

#[test]
fn a_rejecting_client_exits_without_run_secs() {
    // No `--run-secs`: the client must not wait for a limit that does not exist.
    let mut files = acme_files();
    files.insert(
        "visuals/scout.ron".to_string(),
        b"(target: Unit(\"acme:scoot\"), shape: Cuboid(x: 1.0, y: 0.8, z: 1.4))".to_vec(),
    );
    let server = serve_case(Case {
        files,
        served_package: None,
        advertised_lockfile_hash: None,
    });
    let run = run_client("reject_without_run_secs", &server, None);
    assert_eq!(run.code, Some(1), "stderr: {:?}", run.stderr);
    assert!(
        run.stderr.contains("acme/visuals/scout.ron: target: "),
        "{:?}",
        run.stderr
    );
    assert!(run.elapsed < Duration::from_secs(10), "{:?}", run.elapsed);
}
