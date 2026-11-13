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

/// VAL1: the packer refuses a value of the 0.2.4 validation column with exit 2, the text of its row
/// on stderr, and no archive. The control is the same package with the value corrected (a copy of
/// `cedar` that needs `^0.2.4`): it packs with exit 0. Two rows: the key lists (H1, T2) and the
/// unique default keys (KEY1 (b)).
#[test]
fn pack_cli_refuses_a_value_of_the_validation_column_with_exit_2() {
    let cases = [
        (
            "interface/hud.ron",
            "    reasons: {},",
            r#"    reasons: {"queue_ful": "x"},"#,
            "cedar/interface/hud.ron: reasons.queue_ful: unknown reason key `queue_ful`; did you mean `queue_full`?",
        ),
        (
            "input/bindings.ron",
            r#"(name: "key_cancel", default: [Key(code: "KeyC", mods: [])])"#,
            r#"(name: "key_cancel", default: [Key(code: "KeyP", mods: [])])"#,
            "cedar/input/bindings.ron: actions[4].default[0]: `KeyP` is also the default key of `key_produce`; each default key must be unique",
        ),
    ];
    for (index, (file, old, new, expected)) in cases.iter().enumerate() {
        let work = fresh_dir(&format!("pack_cli_val1_{index}"));
        let dir = work.join("cedar");
        copy_dir(&package_dir("cedar"), &dir);
        let manifest = fs::read_to_string(dir.join("manifest.ron")).unwrap();
        fs::write(
            dir.join("manifest.ron"),
            manifest.replace(r#"engine_api: "^0.2.3""#, r#"engine_api: "^0.2.4""#),
        )
        .unwrap();
        // The control: `cedar` at `^0.2.4` packs.
        let control = turts_pack(&[&dir]);
        assert_eq!(
            control.status.code(),
            Some(0),
            "control {index}: {control:?}"
        );
        fs::remove_file(work.join("cedar.tar")).unwrap();
        let text = fs::read_to_string(dir.join(file)).unwrap();
        assert_eq!(
            text.matches(old).count(),
            1,
            "{file}: the edit target occurs once"
        );
        fs::write(dir.join(file), text.replace(old, new)).unwrap();
        let output = turts_pack(&[&dir]);
        assert_eq!(output.status.code(), Some(2), "{index}: {output:?}");
        assert!(output.stdout.is_empty(), "{index}: {output:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with(expected), "{index}: {stderr:?}");
        assert!(!work.join("cedar.tar").exists(), "{index}: no archive");
    }
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
            "librar",
        ] {
            assert!(stdout.contains(check), "usage lacks `{check}`:\n{stdout}");
        }
        assert!(stdout.contains("exits with 2"), "{stdout}");
    }
}

/// L5: the usage tells the modder the version rule: a field that is written, also empty, needs
/// its API.
#[test]
fn pack_cli_help_states_the_engine_api_rule() {
    let output = turts_pack(&[Path::new("--help")]);
    // The usage wraps its lines: compare the text with the line breaks collapsed.
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stdout = stdout.split_whitespace().collect::<Vec<_>>().join(" ");
    for text in [
        "`engine_api` must exclude every engine API that lacks a feature the package writes",
        "a field written empty (`orders: []`) counts",
        "leave it out for an older API",
    ] {
        assert!(stdout.contains(text), "usage lacks `{text}`:\n{stdout}");
    }
}

/// L2 (review of `L0-API1`): the usage lists the `UNBUILT` refusal and the edition rule, so a modder
/// who gets either one finds the text in the help.
#[test]
fn pack_cli_help_lists_the_unbuilt_refusal_and_the_edition_rule() {
    let output = turts_pack(&[Path::new("--help")]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stdout = stdout.split_whitespace().collect::<Vec<_>>().join(" ");
    for text in [
        // The refusal of a feature that no engine build has yet.
        "`<feature>` is not built in this engine build; leave it out until an engine build supports it",
        // The edition rule: the edition is the lowest API that the requirement admits.
        "edition of a package is the lowest engine API that its `engine_api` requirement admits",
        "older edition keeps the behaviour of its API",
    ] {
        assert!(stdout.contains(text), "usage lacks `{text}`:\n{stdout}");
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

/// M2 of the re-review and H1 of the third re-check: the help states the rule of the sound switch where
/// a modder meets the action name: the engine draws its own switch in each frame in which no mod switch
/// is visible, a package may leave the play-screen switch out, and F10 is reserved for the settings menu.
#[test]
fn pack_cli_help_states_the_sound_switch_rule() {
    let output = turts_pack(&[Path::new("--help")]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stdout = stdout.split_whitespace().collect::<Vec<_>>().join(" ");
    for text in [
        "the engine action `toggle_sound` switches the sound",
        "in each frame in which no `Button` with `Input(\"toggle_sound\")` is visible",
        "so a package may leave the play-screen switch out",
        "F10 is reserved for the settings menu",
        "refuses a package of the edition 0.2.4 that binds F10 to another action",
    ] {
        assert!(stdout.contains(text), "usage lacks `{text}`:\n{stdout}");
    }
}
