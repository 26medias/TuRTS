//! T1: forbidden-edge checks. Every test runs `scripts/boundary.sh` from the repository root.
//! Every red test asserts the exact stderr line the script prints, so a crash or a lock message
//! cannot pass as "red".

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

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
