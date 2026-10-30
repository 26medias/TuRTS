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

#[test]
fn t1x_copy_with_bevy_render_is_red() {
    let root = repo_root();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let temp = std::env::temp_dir().join(format!(
        "turts-interaction-t1x-{}-{stamp}",
        std::process::id()
    ));
    let copied_crate = temp.join("crates/turts_interaction");
    std::fs::create_dir_all(copied_crate.join("src")).expect("create copied crate");
    for entry in std::fs::read_dir(root.join("crates/turts_interaction/src")).expect("read sources")
    {
        let entry = entry.expect("source entry");
        std::fs::copy(
            entry.path(),
            copied_crate.join("src").join(entry.file_name()),
        )
        .expect("copy source");
    }
    std::fs::copy(
        root.join("crates/turts_interaction/Cargo.toml"),
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

    let manifest = copied_crate.join("Cargo.toml");
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
