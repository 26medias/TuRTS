//! T1: forbidden-edge checks. Every test runs `scripts/boundary.sh` from the repository root.
//! Every red test asserts the exact stderr line the script prints, so a crash or a lock message
//! cannot pass as "red".

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    Command::new("bash")
        .arg("scripts/boundary.sh")
        .args(args)
        .envs(envs.iter().copied())
        .current_dir(repo_root())
        .output()
        .expect("bash starts")
}

fn run(args: &[&str]) -> Output {
    run_with_env(args, &[])
}

/// The host target triple, as `rustc -vV` prints it.
fn host_triple() -> String {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("rustc starts");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .expect("rustc -vV prints the host")
        .to_string()
}

fn out_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn err_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_red(output: &Output, stderr_is: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {:?} stderr: {:?}",
        out_of(output),
        err_of(output)
    );
    assert_eq!(err_of(output), stderr_is, "stdout: {:?}", out_of(output));
}

fn assert_red_prefix(output: &Output, stderr_starts_with: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {:?} stderr: {:?}",
        out_of(output),
        err_of(output)
    );
    assert!(
        err_of(output).starts_with(stderr_starts_with),
        "stderr: {:?}",
        err_of(output)
    );
}

#[test]
fn t1_boundary_script_passes() {
    let output = run(&[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {:?} stderr: {:?}",
        out_of(&output),
        err_of(&output)
    );
    assert!(
        out_of(&output).trim_end().ends_with("boundary ok"),
        "stdout: {:?}",
        out_of(&output)
    );
    // M1 verify: the browser entry is checked for `turts_sim` on both targets.
    let host = host_triple();
    for line in [
        format!("checked: turts_client_web {host}: no turts_sim"),
        "checked: turts_client_web wasm32-unknown-unknown: no turts_sim".to_string(),
        format!("checked: turts_interaction {host}: no bevy*"),
        "checked: turts_interaction wasm32-unknown-unknown: no bevy*".to_string(),
        format!("checked: turts_interaction {host}: no turts_sim wgpu winit"),
        "checked: turts_interaction wasm32-unknown-unknown: no turts_sim wgpu winit".to_string(),
        format!(
            "checked: turts_audit {host}: no {}",
            AUDIT_FORBIDDEN.join(" ")
        ),
        format!(
            "checked: turts_audit wasm32-unknown-unknown: no {}",
            AUDIT_FORBIDDEN.join(" ")
        ),
        format!("checked: turts_sim {host}: no turts_audit"),
        format!("checked: turts_client {host}: no turts_audit"),
        "checked: turts_client wasm32-unknown-unknown: no turts_audit".to_string(),
        format!("checked: turts_client_web {host}: no turts_audit"),
        "checked: turts_client_web wasm32-unknown-unknown: no turts_audit".to_string(),
        format!("checked: turts_interaction {host}: no turts_audit"),
        "checked: turts_interaction wasm32-unknown-unknown: no turts_audit".to_string(),
        format!("checked: turts_rules {host}: no bevy*"),
        "checked: turts_rules wasm32-unknown-unknown: no bevy*".to_string(),
        format!(
            "checked: turts_rules {host}: no {}",
            RULES_FORBIDDEN.join(" ")
        ),
        format!(
            "checked: turts_rules wasm32-unknown-unknown: no {}",
            RULES_FORBIDDEN.join(" ")
        ),
    ] {
        assert!(
            out_of(&output).lines().any(|got| got == line),
            "missing {line:?} in stdout: {:?}",
            out_of(&output)
        );
    }
}

#[test]
fn t1x_full_run_prints_four_lines() {
    let output = run(&[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {:?}",
        err_of(&output)
    );
    let host = host_triple();
    let expected = [
        format!("checked: turts_interaction {host}: no bevy*"),
        "checked: turts_interaction wasm32-unknown-unknown: no bevy*".to_string(),
        format!("checked: turts_interaction {host}: no turts_sim wgpu winit"),
        "checked: turts_interaction wasm32-unknown-unknown: no turts_sim wgpu winit".to_string(),
    ];
    let stdout = out_of(&output);
    for line in expected {
        assert!(
            stdout.lines().any(|got| got == line),
            "missing {line:?} in {stdout:?}"
        );
    }
}

#[test]
fn t1x_real_crate_wasm32_is_red_for_mod_api() {
    let output = run(&[
        "--rule",
        "forbid",
        "--crate",
        "turts_interaction",
        "--target",
        "wasm32-unknown-unknown",
        "--names",
        "turts_mod_api",
    ]);
    assert_red(
        &output,
        "forbidden edge: turts_interaction -> turts_mod_api\n",
    );
}

/// Copy the sources of the workspace crate `name` and the repository lockfile into a new
/// directory under the system temporary directory. The caller writes a standalone manifest at
/// the returned manifest path. Returns the temporary directory and the manifest path.
fn standalone_copy(name: &str) -> (PathBuf, PathBuf) {
    let root = repo_root();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let short = name.trim_start_matches("turts_");
    let temp =
        std::env::temp_dir().join(format!("turts-{short}-t1x-{}-{stamp}", std::process::id()));
    let copied_crate = temp.join("crates").join(name);
    std::fs::create_dir_all(copied_crate.join("src")).expect("create copied crate");
    for entry in
        std::fs::read_dir(root.join("crates").join(name).join("src")).expect("read sources")
    {
        let entry = entry.expect("source entry");
        std::fs::copy(
            entry.path(),
            copied_crate.join("src").join(entry.file_name()),
        )
        .expect("copy source");
    }
    std::fs::copy(
        root.join("crates").join(name).join("Cargo.toml"),
        copied_crate.join("Cargo.toml"),
    )
    .expect("copy package manifest");
    std::fs::copy(root.join("Cargo.lock"), copied_crate.join("Cargo.lock"))
        .expect("copy lockfile beside standalone workspace manifest");
    assert!(
        copied_crate.join("Cargo.lock").is_file(),
        "seeded Cargo.lock must be beside the standalone workspace manifest"
    );
    assert!(
        std::fs::read(copied_crate.join("Cargo.lock")).expect("read copied lockfile")
            == std::fs::read(root.join("Cargo.lock")).expect("read source lockfile"),
        "standalone workspace must start with the repository lockfile"
    );
    (temp, copied_crate.join("Cargo.toml"))
}

#[test]
fn t1x_copy_with_bevy_render_is_red() {
    let root = repo_root();
    let (temp, manifest) = standalone_copy("turts_interaction");
    let text = format!(
        "[workspace]\nresolver = \"3\"\n\n[package]\nname = \"turts_interaction\"\nversion = \"0.2.0\"\nedition = \"2024\"\nlicense = \"LicenseRef-TBD\"\npublish = false\n\n[dependencies]\nglam = \"=0.33.12\"\nturts_mod_api = {{ path = \"{}\" }}\nturts_net_protocol = {{ path = \"{}\" }}\nbevy_render = \"=0.20.0\"\n",
        root.join("crates/turts_mod_api").display(),
        root.join("crates/turts_net_protocol").display(),
    );
    std::fs::write(&manifest, text).expect("write standalone manifest");

    let output = Command::new("bash")
        .arg("scripts/boundary.sh")
        .args([
            "--rule",
            "no-bevy-prefix",
            "--crate",
            "turts_interaction",
            "--manifest",
        ])
        .arg(&manifest)
        .env("CARGO_NET_OFFLINE", "true")
        .current_dir(&root)
        .output()
        .expect("boundary script starts");
    let _ = std::fs::remove_dir_all(temp);
    assert_red(&output, "forbidden edge: turts_interaction -> bevy_app\n");
}

/// The names that the `turts_audit` rows of `scripts/boundary.sh` forbid, in the script order.
const AUDIT_FORBIDDEN: [&str; 10] = [
    "bevy_render",
    "bevy_winit",
    "bevy_pbr",
    "turts_client",
    "turts_client_web",
    "turts_interaction",
    "turts_server",
    "turts_pack",
    "turts_mod_runtime",
    "turts_net_lightyear",
];

#[test]
fn t1x_audit_copy_with_mod_runtime_is_red() {
    // The fixture crate cannot prove this row: `tree` selects the package by name
    // (`cargo tree -p turts_audit`), and the fixture package is `turts_boundary_fixture`.
    // A standalone copy of `turts_audit` with one planted edge proves the row on both targets.
    let root = repo_root();
    let (temp, manifest) = standalone_copy("turts_audit");
    let text = format!(
        "[workspace]\nresolver = \"3\"\n\n[package]\nname = \"turts_audit\"\nversion = \"0.2.0\"\nedition = \"2024\"\nlicense = \"LicenseRef-TBD\"\npublish = false\n\n[dependencies]\nturts_sim = {{ path = \"{}\" }}\nturts_net_protocol = {{ path = \"{}\" }}\nturts_mod_api = {{ path = \"{}\" }}\nturts_mod_runtime = {{ path = \"{}\" }}\nbevy = {{ version = \"=0.20.0\", default-features = false }}\nserde = {{ version = \"=1.0.229\", features = [\"derive\"] }}\nserde_json = \"=1.0.154\"\n",
        root.join("crates/turts_sim").display(),
        root.join("crates/turts_net_protocol").display(),
        root.join("crates/turts_mod_api").display(),
        root.join("crates/turts_mod_runtime").display(),
    );
    std::fs::write(&manifest, text).expect("write standalone manifest");

    let names = AUDIT_FORBIDDEN.join(",");
    let outputs: Vec<Output> = [host_triple().as_str(), "wasm32-unknown-unknown"]
        .into_iter()
        .map(|target| {
            Command::new("bash")
                .arg("scripts/boundary.sh")
                .args([
                    "--rule",
                    "forbid",
                    "--crate",
                    "turts_audit",
                    "--target",
                    target,
                ])
                .arg("--manifest")
                .arg(&manifest)
                .args(["--names", &names])
                .env("CARGO_NET_OFFLINE", "true")
                .current_dir(&root)
                .output()
                .expect("boundary script starts")
        })
        .collect();
    let _ = std::fs::remove_dir_all(temp);
    for output in &outputs {
        assert_red(output, "forbidden edge: turts_audit -> turts_mod_runtime\n");
    }
}

/// The names that the `turts_rules` rows of `scripts/boundary.sh` forbid, in the script order:
/// every other workspace crate, `wgpu` and `winit`. The `bevy*` crates have their own row.
const RULES_FORBIDDEN: [&str; 14] = [
    "turts_audit",
    "turts_checks",
    "turts_client",
    "turts_client_web",
    "turts_interaction",
    "turts_mod_api",
    "turts_mod_runtime",
    "turts_net_lightyear",
    "turts_net_protocol",
    "turts_pack",
    "turts_server",
    "turts_sim",
    "wgpu",
    "winit",
];

/// Write a standalone manifest for a copy of `turts_rules` with the extra `[dependencies]`
/// lines, and run the boundary rule on both targets. Returns the two outputs (host, wasm32).
fn rules_copy_outputs(extra_dependencies: &str, rule: &[&str]) -> Vec<Output> {
    // The fixture crate cannot prove these rows: `tree` selects the package by name
    // (`cargo tree -p turts_rules`). A standalone copy of `turts_rules` with one planted edge
    // proves each row on both targets.
    let root = repo_root();
    let (temp, manifest) = standalone_copy("turts_rules");
    let text = format!(
        "[workspace]\nresolver = \"3\"\n\n[package]\nname = \"turts_rules\"\nversion = \"0.2.0\"\nedition = \"2024\"\nlicense = \"LicenseRef-TBD\"\npublish = false\n\n[dependencies]\n{extra_dependencies}"
    );
    std::fs::write(&manifest, text).expect("write standalone manifest");
    let outputs: Vec<Output> = [host_triple().as_str(), "wasm32-unknown-unknown"]
        .into_iter()
        .map(|target| {
            Command::new("bash")
                .arg("scripts/boundary.sh")
                .args(rule)
                .args(["--target", target])
                .arg("--manifest")
                .arg(&manifest)
                .env("CARGO_NET_OFFLINE", "true")
                .current_dir(&root)
                .output()
                .expect("boundary script starts")
        })
        .collect();
    let _ = std::fs::remove_dir_all(temp);
    outputs
}

#[test]
fn t1x_rules_copy_with_a_workspace_crate_is_red() {
    let root = repo_root();
    let dependencies = format!(
        "turts_mod_api = {{ path = \"{}\" }}\n",
        root.join("crates/turts_mod_api").display()
    );
    let names = RULES_FORBIDDEN.join(",");
    let outputs = rules_copy_outputs(
        &dependencies,
        &[
            "--rule",
            "forbid",
            "--crate",
            "turts_rules",
            "--names",
            &names,
        ],
    );
    for output in &outputs {
        assert_red(output, "forbidden edge: turts_rules -> turts_mod_api\n");
    }
}

#[test]
fn t1x_rules_copy_with_a_bevy_crate_is_red() {
    let outputs = rules_copy_outputs(
        "bevy_math = \"=0.20.0\"\n",
        &["--rule", "no-bevy-prefix", "--crate", "turts_rules"],
    );
    for output in &outputs {
        assert_red_prefix(output, "forbidden edge: turts_rules -> bevy");
    }
}

#[test]
fn t1x_rules_copy_with_no_planted_edge_is_green() {
    // The copy with no extra edge passes both rows, so the two red tests above are red
    // because of the planted edge and not because of the copy.
    let names = RULES_FORBIDDEN.join(",");
    for rule in [
        vec![
            "--rule",
            "forbid",
            "--crate",
            "turts_rules",
            "--names",
            &names,
        ],
        vec!["--rule", "no-bevy-prefix", "--crate", "turts_rules"],
    ] {
        for output in &rules_copy_outputs("", &rule) {
            assert_eq!(
                output.status.code(),
                Some(0),
                "stderr: {:?}",
                err_of(output)
            );
            assert_eq!(out_of(output), "rule ok\n");
        }
    }
}

#[test]
fn t1x_rules_forbidden_list_is_every_workspace_crate() {
    // The row names every other workspace crate. When a crate is added and the row is not
    // changed, this test is red. The names come from the `crates` directory, and the list
    // that the script runs comes from its own "checked:" line.
    let mut crates: Vec<String> = std::fs::read_dir(repo_root().join("crates"))
        .expect("read crates")
        .map(|entry| {
            entry
                .expect("crate entry")
                .file_name()
                .into_string()
                .expect("crate name is text")
        })
        .filter(|name| name != "turts_rules")
        .collect();
    crates.extend(["wgpu".to_string(), "winit".to_string()]);
    crates.sort();
    let output = run(&[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {:?}",
        err_of(&output)
    );
    let host = host_triple();
    let prefix = format!("checked: turts_rules {host}: no ");
    let stdout = out_of(&output);
    let line = stdout
        .lines()
        .filter_map(|line| line.strip_prefix(prefix.as_str()))
        .find(|rest| *rest != "bevy*")
        .expect("the full run prints the turts_rules row");
    let mut in_script: Vec<String> = line.split(' ').map(str::to_string).collect();
    in_script.sort();
    assert_eq!(in_script, crates);
    let mut in_test: Vec<String> = RULES_FORBIDDEN.iter().map(|n| n.to_string()).collect();
    in_test.sort();
    assert_eq!(in_test, crates);
}

#[test]
fn t1_rule_no_audit_is_red_on_server() {
    // turts_server depends on turts_audit, so the rule "no turts_audit" that the full run
    // applies to turts_sim and the client crates must be red on it.
    let output = run(&[
        "--rule",
        "forbid",
        "--crate",
        "turts_server",
        "--names",
        "turts_audit",
    ]);
    assert_red(&output, "forbidden edge: turts_server -> turts_audit\n");
}

#[test]
fn t1_rule_client_web_is_red_on_a_present_crate() {
    // turts_client_web reaches turts_mod_api through turts_client on wasm32, so the rule that
    // the full run applies to turts_client_web resolves the wasm32 tree and can go red.
    let output = run(&[
        "--rule",
        "forbid",
        "--crate",
        "turts_client_web",
        "--target",
        "wasm32-unknown-unknown",
        "--names",
        "turts_mod_api",
    ]);
    assert_red(
        &output,
        "forbidden edge: turts_client_web -> turts_mod_api\n",
    );
}

#[test]
fn t1_script_fails_when_fixture_is_green() {
    // A fixture that does not reach bevy_render must make the script fail.
    let output = run_with_env(
        &[],
        &[
            (
                "BOUNDARY_FIXTURE_MANIFEST",
                "crates/turts_checks/Cargo.toml",
            ),
            ("BOUNDARY_FIXTURE_PACKAGE", "turts_checks"),
        ],
    );
    assert_red(&output, "fixture not red\n");
}

#[test]
fn t1_rule_forbid_is_red_on_fixture() {
    let output = run(&[
        "--rule",
        "forbid",
        "--crate",
        "turts_boundary_fixture",
        "--manifest",
        "tests/boundary-fixture/Cargo.toml",
        "--names",
        "bevy_render",
    ]);
    assert_red(
        &output,
        "forbidden edge: turts_boundary_fixture -> bevy_render\n",
    );
}

#[test]
fn t1_rule_no_bevy_prefix_is_red_on_fixture() {
    // The first sorted match is bevy_app, not bevy_render.
    let output = run(&[
        "--rule",
        "no-bevy-prefix",
        "--crate",
        "turts_boundary_fixture",
        "--manifest",
        "tests/boundary-fixture/Cargo.toml",
    ]);
    assert_red_prefix(&output, "forbidden edge: turts_boundary_fixture -> bevy");
}

#[test]
fn t1_rule_client_sim_is_red_on_server() {
    // turts_server depends on turts_sim, so the rule "no turts_sim" must be red on it.
    let output = run(&[
        "--rule",
        "forbid",
        "--crate",
        "turts_server",
        "--names",
        "turts_sim",
    ]);
    assert_red(&output, "forbidden edge: turts_server -> turts_sim\n");
}

#[test]
fn t1_rule_no_probes_is_green_on_the_shipped_server_and_client() {
    // The feature `test-probes` of `turts_sim` is off in the server build (normal and build
    // edges). The client crates have no edge to `turts_sim` at all: that is also green.
    for krate in ["turts_server", "turts_client", "turts_client_web"] {
        let output = run(&["--rule", "no-probes", "--crate", krate]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{krate}: stdout: {:?} stderr: {:?}",
            out_of(&output),
            err_of(&output)
        );
        assert_eq!(out_of(&output).trim_end(), "rule ok", "{krate}");
    }
}

#[test]
fn t1_rule_no_probes_is_red_where_the_feature_is_on() {
    // `turts_pack` turns the feature on in its dev-dependencies (the trace emitter). With the
    // dev edge in the check, the rule that the full run applies to the server must go red.
    let output = run(&[
        "--rule",
        "no-probes",
        "--crate",
        "turts_pack",
        "--edges",
        "normal,build,dev",
    ]);
    assert_red(
        &output,
        "forbidden feature: test-probes in turts_pack (edges normal,build,dev)\n",
    );
}

#[test]
fn t1_rule_workspace_is_red_on_present_crate() {
    let output = run(&["--rule", "workspace", "--names", "bevy_app"]);
    assert_red(&output, "forbidden edge: workspace -> bevy_app\n");
}

#[test]
fn t1_script_fails_on_unknown_crate() {
    let output = run(&["--rule", "forbid", "--crate", "turts_nope", "--names", "x"]);
    assert_red_prefix(&output, "cargo tree failed for turts_nope:");
}

#[test]
fn t1_script_fails_on_wrong_target() {
    let output = run(&[
        "--rule",
        "forbid",
        "--crate",
        "turts_sim",
        "--target",
        "nope-unknown-none",
        "--names",
        "x",
    ]);
    assert_red_prefix(&output, "cargo tree failed for turts_sim:");
}

#[test]
fn t1_script_is_green_under_parallel_cargo() {
    // Four script runs in parallel, while a fifth thread keeps taking the cargo lock.
    // Cargo prints "Blocking waiting for file lock" on stderr; the script must not read it as a tree.
    let stop = AtomicBool::new(false);
    thread::scope(|scope| {
        let holder = scope.spawn(|| {
            while !stop.load(Ordering::Relaxed) {
                let status = Command::new("cargo")
                    .args(["tree", "-p", "turts_checks"])
                    .current_dir(repo_root())
                    .output()
                    .expect("cargo starts")
                    .status;
                assert!(status.success());
            }
        });
        let runners: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    run(&[
                        "--rule",
                        "forbid",
                        "--crate",
                        "turts_sim",
                        "--names",
                        "bevy_render",
                    ])
                })
            })
            .collect();
        let outputs: Vec<Output> = runners
            .into_iter()
            .map(|runner| runner.join().expect("runner thread"))
            .collect();
        stop.store(true, Ordering::Relaxed);
        holder.join().expect("lock holder thread");
        for output in &outputs {
            assert_eq!(
                output.status.code(),
                Some(0),
                "stdout: {:?} stderr: {:?}",
                out_of(output),
                err_of(output)
            );
            assert_eq!(out_of(output).trim_end(), "rule ok");
        }
    });
}
