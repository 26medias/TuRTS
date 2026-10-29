//! The `turts-pack` binary: output, exit codes, usage. Test code may name the example
//! packages.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use turts_mod_runtime::hash::sha256_hex;

fn package_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(name)
}

fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create dir");
    for entry in fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("dir entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

fn turts_pack(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn pack_cli_prints_hash() {
    // All public packages that the repository ships pass the checks.
    for name in ["acme", "base", "haul"] {
        let work = fresh_dir(&format!("pack_cli_prints_hash_{name}"));
        let dir = work.join(name);
        copy_dir(&package_dir(name), &dir);
        let output = turts_pack(&[&dir]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        let tokens: Vec<&str> = stdout.split_whitespace().collect();
        assert_eq!(tokens.len(), 2, "{stdout:?}");
        let hash = tokens[1];
        assert!(hash.starts_with("sha256:"), "{hash}");
        assert_eq!(hash.len(), 71, "{hash}");
        // The printed file is `<dir name>.tar` next to the directory, and the hash is its hash.
        let tar = work.join(format!("{name}.tar"));
        assert_eq!(Path::new(tokens[0]), tar.as_path());
        assert_eq!(sha256_hex(&fs::read(&tar).unwrap()), hash);
    }
}

#[test]
fn pack_cli_exits_2_with_the_error_on_stderr() {
    let missing = fresh_dir("pack_cli_error").join("nope");
    let output = turts_pack(&[&missing]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("pack"), "{stderr:?}");
}

#[test]
fn pack_cli_refuses_a_directory_without_a_manifest() {
    let work = fresh_dir("pack_cli_no_manifest");
    let dir = work.join("empty");
    fs::create_dir_all(&dir).unwrap();
    let output = turts_pack(&[&dir]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("no manifest.ron"), "{stderr:?}");
    assert!(!work.join("empty.tar").exists());
}

/// B5: `--help` prints the usage on stdout and exits with 0.
#[test]
fn pack_cli_help_lists_the_usage_and_the_checks() {
    for flag in ["--help", "-h"] {
        let output = turts_pack(&[Path::new(flag)]);
        assert_eq!(output.status.code(), Some(0), "{flag}: {output:?}");
        assert!(output.stderr.is_empty(), "{flag}: {output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.starts_with("usage: turts-pack <dir>"), "{stdout}");
        assert!(stdout.contains("<dir>.tar"), "{stdout}");
        assert!(stdout.contains("sha256"), "{stdout}");
        for check in [
            "unknown field",
            "server rules",
            "exactly one visual",
            "capabilit",
        ] {
            assert!(stdout.contains(check), "usage lacks `{check}`:\n{stdout}");
        }
        assert!(stdout.contains("exits with 2"), "{stdout}");
    }
}

#[test]
fn pack_cli_without_a_directory_or_with_an_unknown_option_prints_the_usage() {
    let none = turts_pack(&[]);
    assert_eq!(none.status.code(), Some(2), "{none:?}");
    assert!(none.stdout.is_empty());
    assert!(
        String::from_utf8(none.stderr)
            .unwrap()
            .contains("usage: turts-pack <dir>")
    );

    let unknown = turts_pack(&[Path::new("--bogus")]);
    assert_eq!(unknown.status.code(), Some(2), "{unknown:?}");
    let stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(
        stderr.starts_with("turts-pack: unknown option `--bogus`"),
        "{stderr}"
    );
    assert!(stderr.contains("usage: turts-pack <dir>"), "{stderr}");
}
