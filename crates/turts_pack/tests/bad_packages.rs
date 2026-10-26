//! M1 verify, B1 and B2: `turts-pack` refuses a package that the server or the client would
//! refuse. It names the file and the field, exits with 2, and writes no archive.
//!
//! The fixture `fixtures/zeta` is the package that a third-party modder built from the
//! documents alone. Each bad package is the fixture with one edit, the same edit the modder
//! made. Test code may name the example packages.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// One edit of the fixture: `(file, from, to)`. An empty `from` makes a new file with the
/// text `to`. Any other `from` must occur in the file, so an edit cannot miss.
type Edit = (&'static str, &'static str, &'static str);

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zeta")
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

/// Copy the fixture to `<work>/zeta`, apply `edits`, and run `turts-pack` on the copy.
/// Returns the work directory (the archive would be `<work>/zeta.tar`) and the output.
fn pack_edited(test: &str, edits: &[Edit]) -> (PathBuf, Output) {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = fs::remove_dir_all(&work);
    let dir = work.join("zeta");
    copy_dir(&fixture(), &dir);
    for (file, from, to) in edits {
        let path = dir.join(file);
        if from.is_empty() {
            fs::write(&path, to).expect("write a new file");
        } else {
            let text = fs::read_to_string(&path).expect("read a package file");
            assert!(text.contains(from), "edit target `{from}` is not in {file}");
            fs::write(&path, text.replacen(from, to, 1)).expect("write a package file");
        }
    }
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    (work, output)
}

/// The package must be refused with exactly this one line on stderr.
fn assert_refused(test: &str, edits: &[Edit], message: &str) {
    let (work, output) = pack_edited(test, edits);
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8");
    assert_eq!(output.status.code(), Some(2), "{test}: {output:?}");
    assert_eq!(stderr.trim_end(), message, "{test}");
    assert!(output.stdout.is_empty(), "{test}: {output:?}");
    assert!(
        !work.join("zeta.tar").exists(),
        "{test}: the archive was written"
    );
}

#[test]
fn the_control_package_is_packed() {
    let (work, output) = pack_edited("bad_control", &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(work.join("zeta.tar").exists());
}

#[test]
fn a_field_typo_is_refused() {
    assert_refused(
        "bad_typo",
        &[("units/runner.ron", "speed: 8.0", "sped: 8.0")],
        "zeta/units/runner.ron: unknown field `sped` at line 1 column 32 \
         (allowed: id, name, speed, footprint)",
    );
}

#[test]
fn an_unknown_variant_is_refused() {
    assert_refused(
        "bad_variant",
        &[(
            "units/crawler.ron",
            "Circle(radius: 2.5)",
            "Square(side: 2.5)",
        )],
        "zeta/units/crawler.ron: unknown variant `Square` at line 1 column 57 (allowed: Circle)",
    );
}

#[test]
fn a_speed_of_zero_is_refused() {
    assert_refused(
        "bad_speed",
        &[("units/crawler.ron", "speed: 1.5", "speed: 0.0")],
        "zeta/units/crawler.ron: speed: speed must be finite, above 0 and at most 100, got 0",
    );
}

#[test]
fn an_owner_slot_of_two_is_refused() {
    assert_refused(
        "bad_slot",
        &[(
            "maps/arena.ron",
            "owner_slot: 0, pos: (x: 10.0",
            "owner_slot: 2, pos: (x: 10.0",
        )],
        "zeta/maps/arena.ron: placements[1].owner_slot: owner slot must be below 2, got 2",
    );
}

#[test]
fn a_visual_of_a_unit_the_package_does_not_define_is_refused() {
    // A third visual for a unit that does not exist. Every real unit keeps its visual, so the
    // server starts and the old client accepted the package.
    assert_refused(
        "bad_orphan",
        &[
            (
                "manifest.ron",
                "\"visuals/crawler.ron\"]",
                "\"visuals/crawler.ron\", \"visuals/ghost.ron\"]",
            ),
            (
                "visuals/ghost.ron",
                "",
                "(unit: \"zeta:ghost\", shape: Capsule(radius: 0.3, height: 1.0))",
            ),
        ],
        "zeta/visuals/ghost.ron: unit: the visual is for unit `zeta:ghost`, which no unit file \
         of the package defines (allowed: zeta:crawler, zeta:runner)",
    );
}

#[test]
fn a_typo_in_the_unit_of_a_visual_is_refused() {
    assert_refused(
        "bad_orphan_typo",
        &[("visuals/runner.ron", "zeta:runner", "zeta:runer")],
        "zeta/visuals/runner.ron: unit: the visual is for unit `zeta:runer`, which no unit file \
         of the package defines (allowed: zeta:crawler, zeta:runner)",
    );
}
