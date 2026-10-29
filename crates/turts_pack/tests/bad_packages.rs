//! M1 verify, B1 and B2: `turts-pack` refuses a package that the server or the client would
//! refuse. It names the file and the field, exits with 2, and writes no archive.
//!
//! The fixture `fixtures/zeta` is the package that a third-party modder built from the
//! documents alone. Each bad package is the fixture with one edit, the same edit the modder
//! made. Test code may name the example packages.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

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

/// Copy the fixture to `<work>/zeta` and apply edits.
fn edited_dir(test: &str, edits: &[Edit]) -> PathBuf {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = fs::remove_dir_all(&work);
    let dir = work.join("zeta");
    copy_dir(&fixture(), &dir);
    let manifest = dir.join("manifest.ron");
    let manifest_text = fs::read_to_string(&manifest).expect("read fixture manifest");
    assert!(manifest_text.contains("version: \"0.1.0\""));
    fs::write(
        &manifest,
        manifest_text.replace("version: \"0.1.0\"", "version: \"0.2.0\""),
    )
    .expect("migrate fixture package version");
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
    dir
}

/// Copy and edit the fixture, then run `turts-pack` on the copy.
fn pack_edited(test: &str, edits: &[Edit]) -> (PathBuf, Output) {
    let dir = edited_dir(test, edits);
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    (dir.parent().unwrap().to_path_buf(), output)
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
        &[(
            "units/runner.ron",
            "movement: Some((speed: 8.0))",
            "movenment: Some((speed: 8.0))",
        )],
        "zeta/units/runner.ron: unknown field `movenment` at line 1 column 64; did you mean `movement`? \
         (allowed: id, name, footprint, movement, inventory, gatherer, drop_off, producer, output_inventory, extractor, conveyor, inserter, machine, research, health, weapon, removal)",
    );
}

#[test]
fn an_unknown_capability_is_refused() {
    assert_refused(
        "bad_capability",
        &[(
            "manifest.ron",
            "capabilities: []",
            "capabilities: [\"scripts.server\"]",
        )],
        "zeta/manifest.ron: capabilities: unknown capability `scripts.server`; known: none",
    );
}

#[test]
fn a_duplicate_capability_is_refused() {
    assert_refused(
        "duplicate_capability",
        &[(
            "manifest.ron",
            "capabilities: []",
            "capabilities: [\"scripts.server\", \"scripts.server\"]",
        )],
        "zeta/manifest.ron: capabilities: capability `scripts.server` is listed twice",
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
        "zeta/units/crawler.ron: unknown variant `Square` at line 1 column 45 (allowed: Circle)",
    );
}

#[test]
fn a_speed_of_zero_is_refused() {
    assert_refused(
        "bad_speed",
        &[("units/crawler.ron", "speed: 1.5", "speed: 0.0")],
        "zeta/units/crawler.ron: movement.speed: must be finite and > 0",
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
                "(target: Unit(\"zeta:ghost\"), shape: Capsule(radius: 0.3, height: 1.0))",
            ),
        ],
        "zeta/visuals/ghost.ron: target: visual target `zeta:ghost` is not defined as a unit by the archives (allowed: zeta:crawler, zeta:runner)",
    );
}

#[test]
fn a_typo_in_the_unit_of_a_visual_is_refused() {
    assert_refused(
        "bad_orphan_typo",
        &[("visuals/runner.ron", "zeta:runner", "zeta:runer")],
        "zeta/visuals/runner.ron: target: visual target `zeta:runer` is not defined as a unit by the archives (allowed: zeta:crawler, zeta:runner)",
    );
}

fn library_edits(
    engine_api: &'static str,
    dependencies: &'static str,
    maps: &'static str,
) -> [Edit; 4] {
    [
        ("manifest.ron", "kind: Game", "kind: Library"),
        ("manifest.ron", "engine_api: \"^0.2\"", engine_api),
        ("manifest.ron", "dependencies: {}", dependencies),
        ("manifest.ron", "maps: [\"maps/arena.ron\"]", maps),
    ]
}

#[test]
fn a10_library_with_a_map_is_refused() {
    assert_refused(
        "library_with_map",
        &library_edits(
            "engine_api: \"^0.2.2\"",
            "dependencies: {}",
            "maps: [\"maps/arena.ron\"]",
        ),
        "zeta/manifest.ron: exports.maps: a library exports no map, found 1",
    );
}

#[test]
fn a11_library_without_a_map_is_packed() {
    let (work, output) = pack_edited(
        "library_without_map",
        &library_edits("engine_api: \"^0.2.2\"", "dependencies: {}", "maps: []"),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(work.join("zeta.tar").exists());
}

#[test]
fn a11_library_with_dependencies_is_packed() {
    let (work, output) = pack_edited(
        "library_with_dependencies",
        &library_edits(
            "engine_api: \"^0.2.2\"",
            "dependencies: {\"ai_core\": \"^1.0\"}",
            "maps: []",
        ),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(work.join("zeta.tar").exists());
}

#[test]
fn a15_game_with_a_dependency_is_refused() {
    assert_refused(
        "game_with_dependency",
        &[
            (
                "manifest.ron",
                "engine_api: \"^0.2\"",
                "engine_api: \"^0.2.2\"",
            ),
            (
                "manifest.ron",
                "dependencies: {}",
                "dependencies: {\"ai_core\": \"^1.0\"}",
            ),
        ],
        "zeta/manifest.ron: dependencies: dependency `ai_core` `^1.0` is not provided",
    );
}

#[test]
fn sc4_library_needs_api_0_2_2() {
    assert_refused(
        "library_old_api",
        &library_edits("engine_api: \"^0.2\"", "dependencies: {}", "maps: []"),
        "zeta/manifest.ron: engine_api: `kind: Library` needs engine api 0.2.2; `^0.2` admits 0.2.1",
    );
}

#[test]
fn sc4_dependencies_need_api_0_2_2() {
    assert_refused(
        "library_dependencies_old_api",
        &library_edits(
            "engine_api: \"^0.2\"",
            "dependencies: {\"ai_core\": \"^1.0\"}",
            "maps: []",
        ),
        "zeta/manifest.ron: engine_api: `dependencies` needs engine api 0.2.2; `^0.2` admits 0.2.1",
    );
}

#[test]
fn a12_library_root_is_refused_by_load_package() {
    let dir = edited_dir(
        "library_root",
        &library_edits("engine_api: \"^0.2.2\"", "dependencies: {}", "maps: []"),
    );
    let bytes = pack_dir(&dir).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = turts_sim::headless_app(20);
    let error = turts_pack::load_package(app.world_mut(), &archive).unwrap_err();
    assert_eq!(error.message, "a library cannot be the root of a match");
}
