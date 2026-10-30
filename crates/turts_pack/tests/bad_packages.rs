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

fn hud3_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hud3")
}

fn hud3_edited_dir(test: &str, edits: &[Edit]) -> PathBuf {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = fs::remove_dir_all(&work);
    let dir = work.join("zeta");
    copy_dir(&hud3_fixture(), &dir);
    for (file, from, to) in edits {
        let path = dir.join(file);
        if from.is_empty() {
            fs::write(&path, to).expect("write a new fixture file");
        } else {
            let text = fs::read_to_string(&path).expect("read a fixture file");
            assert!(text.contains(from), "edit target `{from}` is not in {file}");
            fs::write(&path, text.replacen(from, to, 1)).expect("edit a fixture file");
        }
    }
    dir
}

fn assert_hud3_refused(test: &str, edits: &[Edit], message: &str) {
    let dir = hud3_edited_dir(test, edits);
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8");
    assert_eq!(output.status.code(), Some(2), "{test}: {output:?}");
    assert_eq!(stderr.trim_end(), message, "{test}");
    assert!(output.stdout.is_empty(), "{test}: {output:?}");
    assert!(
        !dir.with_extension("tar").exists(),
        "{test}: archive was written"
    );
}

fn pack_hud3(test: &str, edits: &[Edit]) -> Output {
    let dir = hud3_edited_dir(test, edits);
    Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts")
}

fn hud3_png(width: u32, height: u32, file_size: usize) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0]);
    bytes.resize(file_size.max(24), 0);
    bytes
}

fn hud3_ogg(seconds: u64) -> Vec<u8> {
    let mut packet = b"\x01vorbis".to_vec();
    packet.extend_from_slice(&0u32.to_le_bytes());
    packet.push(2);
    packet.extend_from_slice(&48_000u32.to_le_bytes());
    packet.extend_from_slice(&[0; 12]);
    packet.extend_from_slice(&[0xB8, 1]);
    let mut page = b"OggS\0\x04".to_vec();
    page.extend_from_slice(&(48_000 * seconds).to_le_bytes());
    page.extend_from_slice(&1u32.to_le_bytes());
    page.extend_from_slice(&0u32.to_le_bytes());
    page.extend_from_slice(&[0; 4]);
    page.push(1);
    page.push(packet.len() as u8);
    page.extend_from_slice(&packet);
    page
}

fn hud3_with_image_set(
    test: &str,
    count: usize,
    width: u32,
    height: u32,
    file_size: usize,
) -> PathBuf {
    let dir = hud3_edited_dir(test, &[]);
    let mut images = vec![
        "(id: \"rush-icon\", path: \"assets/icon.png\", nine_slice: None)".to_string(),
        "(id: \"cursor\", path: \"assets/cursor.png\", nine_slice: None)".to_string(),
    ];
    images.extend((0..count).map(|index| {
        format!("(id: \"image-{index}\", path: \"assets/image-{index}.png\", nine_slice: None)")
    }));
    let assets = format!(
        "(images: [{}], fonts: [(id: \"default\", path: \"assets/font.ttf\")], sounds: [(id: \"alert\", path: \"assets/alert.ogg\")], cursors: [(id: \"default\", image: \"cursor\", hotspot: (0, 0))])",
        images.join(",")
    );
    fs::write(dir.join("assets/main.ron"), assets).expect("write assets definition");
    for index in 0..count {
        fs::write(
            dir.join(format!("assets/image-{index}.png")),
            hud3_png(width, height, file_size),
        )
        .expect("write PNG header");
    }
    dir
}

fn refuse_hud3_dir(test: &str, dir: &Path, message: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(dir)
        .output()
        .expect("turts-pack starts");
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8");
    assert_eq!(output.status.code(), Some(2), "{test}: {output:?}");
    assert_eq!(stderr.trim_end(), message, "{test}");
    assert!(output.stdout.is_empty(), "{test}: {output:?}");
    assert!(
        !dir.with_extension("tar").exists(),
        "{test}: archive was written"
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

#[test]
fn hud3_unknown_widget_kind_is_a_named_schema_error() {
    assert_hud3_refused(
        "hud3_unknown_widget",
        &[(
            "interface/main.ron",
            "widget: Button(",
            "widget: NoSuchWidget(",
        )],
        "zeta/interface/main.ron: unknown variant `NoSuchWidget` at line 4 column 143 (allowed: Box, Image, Text, Bar, Button, List, Grid, Tooltip, Cursor, MinimapView)",
    );
}

#[test]
fn hud3_unknown_widget_field_is_refused_by_strict_schema() {
    assert_hud3_refused(
        "hud3_unknown_widget_field",
        &[("interface/main.ron", "hotkey_label", "hotkey_lable")],
        "zeta/interface/main.ron: unknown field `hotkey_lable` at line 4 column 235; did you mean `hotkey_label`? (allowed: icon, label, action, hotkey_label)",
    );
}

#[test]
fn hud3_unknown_source_is_refused() {
    assert_hud3_refused(
        "hud3_unknown_source",
        &[(
            "interface/main.ron",
            "source: CommandCard",
            "source: SecretSource",
        )],
        "zeta/interface/main.ron: unknown variant `SecretSource` at line 5 column 153 (allowed: Balance, Balances, SelectionUnits, CommandCard, BuildMenu, ProductionQueue, ControlGroups, Alerts, Bindings, Guide)",
    );
}

#[test]
fn hud3_duplicate_widget_id_is_refused() {
    assert_hud3_refused(
        "hud3_duplicate_widget",
        &[(
            "interface/main.ron",
            "id: \"orders\"",
            "id: \"rush-button\"",
        )],
        "zeta/interface/main.ron: root[1].id: duplicate widget id `rush-button`",
    );
}

#[test]
fn hud3_duplicate_card_cell_is_refused() {
    assert_hud3_refused(
        "hud3_duplicate_card",
        &[("orders/hold.ron", "row: 0, column: 1", "row: 0, column: 0")],
        "zeta/orders/hold.ron: card: card cell (0, 0) collides for unit `zeta:trooper` with order `zeta:rush`",
    );
}

#[test]
fn hud3_missing_icon_asset_is_refused() {
    assert_hud3_refused(
        "hud3_missing_asset",
        &[("orders/rush.ron", "icon: \"rush-icon\"", "icon: \"absent\"")],
        "zeta/orders/rush.ron: icon: unknown image asset `absent`",
    );
}

#[test]
fn hud3_default_reserved_browser_binding_is_refused() {
    assert_hud3_refused(
        "hud3_reserved_binding",
        &[
            ("input/main.ron", "KeyR", "KeyW"),
            ("input/main.ron", "mods: []", "mods: [Ctrl]"),
        ],
        "zeta/input/main.ron: actions[0].default[0]: Ctrl+KeyW is reserved by the browser",
    );
}

#[test]
fn hud3_unknown_physical_key_code_is_refused() {
    assert_hud3_refused(
        "hud3_unknown_key_code",
        &[("input/main.ron", "KeyR", "KeyFlorp")],
        "zeta/input/main.ron: actions[0].default[0].code: unknown physical key code `KeyFlorp`",
    );
}

#[test]
fn hud3_supported_modifier_key_code_is_accepted() {
    let dir = hud3_edited_dir(
        "hud3_modifier_key",
        &[("input/main.ron", "KeyR", "ShiftLeft")],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn hud3_supported_ime_key_code_is_accepted() {
    let output = pack_hud3("hud3_convert_key", &[("input/main.ron", "KeyR", "Convert")]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn hud3_supported_bevy_key_codes_are_accepted() {
    for code in [
        "BrowserBack",
        "WakeUp",
        "Meta",
        "Hyper",
        "Turbo",
        "Abort",
        "Resume",
        "Suspend",
        "Again",
        "Copy",
        "Cut",
        "Find",
        "Open",
        "Paste",
        "Props",
        "Select",
        "Undo",
        "Hiragana",
        "Katakana",
        "F35",
    ] {
        let output = pack_hud3(
            &format!("hud3_supported_key_{code}"),
            &[("input/main.ron", "KeyR", code)],
        );
        assert_eq!(output.status.code(), Some(0), "{code}: {output:?}");
    }
}

#[test]
fn hud3_reserved_browser_binding_ignores_modifier_order() {
    assert_hud3_refused(
        "hud3_reserved_binding_permuted",
        &[
            ("input/main.ron", "KeyR", "KeyW"),
            ("input/main.ron", "mods: []", "mods: [Shift, Ctrl]"),
        ],
        "zeta/input/main.ron: actions[0].default[0]: Ctrl+Shift+KeyW is reserved by the browser",
    );
}

#[test]
fn hud3_cursor_widget_obeys_cursor_dimensions() {
    let dir = hud3_edited_dir(
        "hud3_primitive_cursor_dimension",
        &[
            (
                "interface/main.ron",
                "widget: MinimapView(frame: \"#ffffff\")",
                "widget: Cursor(cursors: {\"default\": (\"rush-icon\", (0, 0))})",
            ),
            (
                "assets/main.ron",
                "cursors: [(id: \"default\", image: \"cursor\", hotspot: (0, 0))]",
                "cursors: []",
            ),
            (
                "interface/main.ron",
                "cursors: {\"default\": \"default\"}",
                "cursors: {}",
            ),
            (
                "orders/rush.ron",
                "cursor: Some(\"default\")",
                "cursor: None",
            ),
        ],
    );
    let mut png = fs::read(dir.join("assets/icon.png")).unwrap();
    png[16..20].copy_from_slice(&256u32.to_be_bytes());
    png[20..24].copy_from_slice(&256u32.to_be_bytes());
    fs::write(dir.join("assets/icon.png"), png).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim_end(),
        "zeta/interface/main.ron: root[3].widget.cursors.default: cursor image is 256 x 256 px; maximum is 128 x 128 px",
    );
}

#[test]
fn hud3_cursor_widget_hotspot_must_be_inside_image() {
    assert_hud3_refused(
        "hud3_cursor_hotspot",
        &[(
            "interface/main.ron",
            "widget: MinimapView(frame: \"#ffffff\")",
            "widget: Cursor(cursors: {\"default\": (\"rush-icon\", (999, 999))})",
        )],
        "zeta/interface/main.ron: root[3].widget.cursors.default: hotspot (999, 999) must be inside the 32 x 32 px image",
    );
}

#[test]
fn hud3_button_image_supports_primary_portrait_binding() {
    let dir = hud3_edited_dir(
        "hud3_primary_portrait_binding",
        &[
            (
                "interface/main.ron",
                "widget: MinimapView(frame: \"#ffffff\")",
                "widget: Image(image: Bind(SelectionPrimary(Portrait)), nine_slice: None, tint: None)",
            ),
            (
                "interface/main.ron",
                "reasons: {}, units: {}, selection_priority: []",
                "reasons: {}, units: {\"zeta:trooper\": (icon: Some(\"rush-icon\"), portrait: Some(\"rush-icon\"), responses: {})}, selection_priority: [\"zeta:trooper\"]",
            ),
        ],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn hud3_button_action_rejects_icon_item_binding() {
    let output = pack_hud3(
        "hud3_button_action_icon",
        &[(
            "interface/main.ron",
            "action: Input(\"rush\")",
            "action: Item(Icon)",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unknown variant `Icon`")
    );
}

#[test]
fn hud3_queue_depth_above_255_has_field_range_error() {
    assert_hud3_refused(
        "hud3_queue_depth_overflow",
        &[("orders/rules.ron", "queue_depth: 16", "queue_depth: 256")],
        "zeta/orders/rules.ron: queue_depth: must be between 1 and 255, got 256",
    );
}

#[test]
fn hud3_order_rules_typo_reports_rules_schema() {
    let output = pack_hud3(
        "hud3_order_rules_typo",
        &[("orders/rules.ron", "queue_depth: 16", "queue_dept: 16")],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("zeta/orders/rules.ron: unknown field `queue_dept`"));
    assert!(stderr.contains("(allowed: queue_depth, smart)"));
}

#[test]
fn hud3_order_rules_classification_allows_spacing_before_colon() {
    let output = pack_hud3(
        "hud3_order_rules_spaced_typo",
        &[
            ("orders/rules.ron", "queue_depth: 16", "queue_depth : 16"),
            ("orders/rules.ron", "smart: []", "smart : [], typo: true"),
        ],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("zeta/orders/rules.ron: unknown field `typo`"));
    assert!(stderr.contains("(allowed: queue_depth, smart)"));
}

#[test]
fn hud3_order_units_require_declared_namespace() {
    let output = pack_hud3(
        "hud3_order_unknown_namespace",
        &[("orders/rush.ron", "zeta:trooper", "absent:trooper")],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("units: unknown dependency namespace `absent`")
    );
}

#[test]
fn hud3_widget_order_requires_declared_namespace() {
    let output = pack_hud3(
        "hud3_widget_order_unknown_namespace",
        &[(
            "interface/main.ron",
            "action: Input(\"rush\")",
            "action: Order(\"absent:rush\")",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("root[0].widget.action: unknown dependency namespace `absent`")
    );
}

#[test]
fn hud3_smart_order_requires_declared_namespace() {
    let output = pack_hud3(
        "hud3_smart_order_unknown_namespace",
        &[(
            "orders/rules.ron",
            "smart: []",
            "smart: [(target: Ground, needs: Movement, order: \"absent:rush\")]",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("smart[0].order: unknown dependency namespace `absent`")
    );
}

#[test]
fn hud3_selection_priority_requires_declared_namespace() {
    let output = pack_hud3(
        "hud3_selection_priority_unknown_namespace",
        &[(
            "interface/main.ron",
            "selection_priority: []",
            "selection_priority: [\"absent:trooper\"]",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("selection_priority[0]: unknown dependency namespace `absent`")
    );
}

#[test]
fn hud3_unit_presentation_requires_declared_namespace() {
    let output = pack_hud3(
        "hud3_unit_presentation_unknown_namespace",
        &[(
            "interface/main.ron",
            "units: {}",
            "units: {\"absent:trooper\": (icon: Some(\"rush-icon\"), portrait: None, responses: {})}",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("units: unknown dependency namespace `absent`")
    );
}

#[test]
fn hud3_nested_balance_requires_declared_namespace() {
    let output = pack_hud3(
        "hud3_nested_balance_unknown_namespace",
        &[(
            "interface/main.ron",
            "text: Literal(\"Hint\")",
            "text: Bind(Format(\"{0}\", [Balance(\"absent:ore\")]))",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("root[2].widget.text: unknown dependency namespace `absent`")
    );
}

#[test]
fn hud3_balance_value_rejects_local_non_balance_content() {
    let output = pack_hud3(
        "hud3_balance_wrong_kind",
        &[(
            "interface/main.ron",
            "text: Literal(\"Hint\")",
            "text: Bind(Format(\"{0}\", [Balance(\"zeta:trooper\")]))",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("root[2].widget.text: unknown Balance resource `zeta:trooper`")
    );
}

#[test]
fn hud3_balance_list_source_requires_declared_namespace() {
    let output = pack_hud3(
        "hud3_balance_source_unknown_namespace",
        &[(
            "interface/main.ron",
            "source: CommandCard",
            "source: Balance(\"absent:ore\")",
        )],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("root[1].widget.source: unknown dependency namespace `absent`")
    );
}

#[test]
fn hud3_guide_layout_warnings_are_emitted_without_refusal() {
    let guide = "guide: [(id: \"help\", title: \"Help\", body: \"Body\", widgets: [(id: \"small-text\", size: (w: Px(40.0), h: Px(12.0)), widget: Text(text: Literal(\"x\"), font: None, size: 10, colour: \"#ffffff\", align: Left, wrap: false)), (id: \"small-button\", size: (w: Px(20.0), h: Px(20.0)), widget: Button(icon: None, label: Some(Literal(\"OK\")), action: Input(\"rush\"), hotkey_label: false))])]";
    let dir = hud3_edited_dir(
        "hud3_guide_layout_warnings",
        &[("interface/main.ron", "guide: []", guide)],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(
            "guide[0].widgets[0].widget.size: text is 10 px; recommended minimum is 11 px"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "guide[0].widgets[1].size: button is 20 x 20 px; recommended minimum is 24 x 24 px"
        ),
        "{stderr}"
    );
}

#[test]
fn hud3_cancel_production_requires_job_target() {
    assert_hud3_refused(
        "hud3_cancel_target",
        &[(
            "orders/rush.ron",
            "behaviour: Attack, target: Point",
            "behaviour: CancelProduction, target: Point",
        )],
        "zeta/orders/rush.ron: target: CancelProduction requires Job, got Point",
    );
}

#[test]
fn hud3_queue_depth_zero_is_refused() {
    assert_hud3_refused(
        "hud3_queue_depth_zero",
        &[("orders/rules.ron", "queue_depth: 16", "queue_depth: 0")],
        "zeta/orders/rules.ron: queue_depth: must be between 1 and 255, got 0",
    );
}

#[test]
fn hud3_older_engine_requirement_is_refused_for_interface_export() {
    let dir = hud3_edited_dir(
        "hud3_old_engine_api",
        &[
            (
                "manifest.ron",
                "engine_api: \"^0.2.3\"",
                "engine_api: \"^0.2\"",
            ),
            (
                "manifest.ron",
                "orders: [\"orders/rush.ron\", \"orders/hold.ron\", \"orders/rules.ron\"]",
                "orders: []",
            ),
            ("manifest.ron", "input: [\"input/main.ron\"]", "input: []"),
            (
                "manifest.ron",
                "assets: [\"assets/main.ron\"]",
                "assets: []",
            ),
            (
                "interface/main.ron",
                "font: Some(\"default\")",
                "font: None",
            ),
            (
                "interface/main.ron",
                "cursors: {\"default\": \"default\"}",
                "cursors: {}",
            ),
        ],
    );
    let path = dir.join("interface/main.ron");
    let mut interface = fs::read_to_string(&path).unwrap();
    let root_start = interface.find("    root: [").unwrap();
    let root_end = interface[root_start..].find("    ],").unwrap() + root_start + 6;
    interface.replace_range(root_start..root_end, "    root: [],");
    interface = interface.replace("font: Some(\"default\")", "font: None");
    fs::write(path, interface).unwrap();
    refuse_hud3_dir(
        "hud3_old_engine_api",
        &dir,
        "zeta/manifest.ron: engine_api: `exports.interface` needs engine api 0.2.3; `^0.2` admits 0.2.2",
    );
}

#[test]
fn hud3_png_over_2048_pixels_is_refused_from_ihdr() {
    let dir = hud3_edited_dir("hud3_png_dimension", &[]);
    fs::write(dir.join("assets/icon.png"), hud3_png(2049, 1, 32)).unwrap();
    refuse_hud3_dir(
        "hud3_png_dimension",
        &dir,
        "zeta/assets/main.ron: images[0].path: PNG width 2049 exceeds the 2048 pixel limit",
    );
}

#[test]
fn hud3_cursor_over_128_pixels_is_refused_from_ihdr() {
    let dir = hud3_edited_dir("hud3_cursor_dimension", &[]);
    fs::write(dir.join("assets/cursor.png"), hud3_png(129, 129, 32)).unwrap();
    refuse_hud3_dir(
        "hud3_cursor_dimension",
        &dir,
        "zeta/assets/main.ron: cursors[0].image: cursor image is 129 x 129 px; maximum is 128 x 128 px",
    );
}

#[test]
fn hud3_decoded_image_sum_over_256_mib_is_refused_per_package() {
    let dir = hud3_with_image_set("hud3_image_decoded_cap", 17, 2048, 2048, 32);
    refuse_hud3_dir(
        "hud3_image_decoded_cap",
        &dir,
        "zeta/assets/main.ron: images: decoded image bytes 285220864 exceed the 268435456 byte package limit",
    );
}

#[test]
fn hud3_decoded_cap_uses_png_dimensions_not_compressed_file_bytes() {
    let dir = hud3_with_image_set("hud3_image_compressed_cap", 65, 2048, 2048, 19_771);
    refuse_hud3_dir(
        "hud3_image_compressed_cap",
        &dir,
        "zeta/assets/main.ron: images: decoded image bytes 1090527232 exceed the 268435456 byte package limit",
    );
}

#[test]
fn hud3_missing_font_file_is_refused() {
    let dir = hud3_edited_dir("hud3_missing_font", &[]);
    fs::remove_file(dir.join("assets/font.ttf")).unwrap();
    refuse_hud3_dir(
        "hud3_missing_font",
        &dir,
        "zeta/assets/main.ron: fonts[0].path: asset file `assets/font.ttf` is missing",
    );
}

#[test]
fn hud3_font_above_8_mib_is_refused() {
    let dir = hud3_edited_dir("hud3_large_font", &[]);
    let mut bytes = vec![0; 8 * 1024 * 1024 + 1];
    bytes[..4].copy_from_slice(&[0, 1, 0, 0]);
    fs::write(dir.join("assets/font.ttf"), bytes).unwrap();
    refuse_hud3_dir(
        "hud3_large_font",
        &dir,
        "zeta/assets/main.ron: fonts[0].path: font file is 8388609 bytes; maximum is 8388608 bytes",
    );
}

#[test]
fn hud3_sound_over_30_seconds_is_refused_from_ogg_granule() {
    let dir = hud3_edited_dir("hud3_sound_duration", &[]);
    fs::write(dir.join("assets/alert.ogg"), hud3_ogg(31)).unwrap();
    refuse_hud3_dir(
        "hud3_sound_duration",
        &dir,
        "zeta/assets/main.ron: sounds[0].path: sound is 31 seconds; maximum is 30 seconds",
    );
}

#[test]
fn hud3_sound_file_above_32_mib_is_refused() {
    let dir = hud3_edited_dir("hud3_large_sound", &[]);
    let mut bytes = hud3_ogg(1);
    bytes.resize(32 * 1024 * 1024 + 1, 0);
    fs::write(dir.join("assets/alert.ogg"), bytes).unwrap();
    refuse_hud3_dir(
        "hud3_large_sound",
        &dir,
        "zeta/assets/main.ron: sounds[0].path: sound file is 33554433 bytes; maximum is 33554432 bytes",
    );
}

#[test]
fn hud3_total_sound_duration_above_300_seconds_is_refused() {
    let dir = hud3_edited_dir("hud3_sound_aggregate", &[]);
    let mut sounds = Vec::new();
    for index in 0..10 {
        sounds.push(format!(
            "(id: \"sound-{index}\", path: \"assets/sound-{index}.ogg\")"
        ));
        fs::write(dir.join(format!("assets/sound-{index}.ogg")), hud3_ogg(30)).unwrap();
    }
    sounds.push("(id: \"sound-last\", path: \"assets/sound-last.ogg\")".to_owned());
    fs::write(dir.join("assets/sound-last.ogg"), hud3_ogg(1)).unwrap();
    let asset_path = dir.join("assets/main.ron");
    let assets = fs::read_to_string(&asset_path).unwrap();
    fs::write(
        asset_path,
        assets.replace(
            "sounds: [(id: \"alert\", path: \"assets/alert.ogg\")],",
            &format!("sounds: [{}],", sounds.join(", ")),
        ),
    )
    .unwrap();
    refuse_hud3_dir(
        "hud3_sound_aggregate",
        &dir,
        "zeta/assets/main.ron: sounds: total duration 301 seconds exceeds the 300 second package limit",
    );
}

#[test]
fn hud3_ten_pixel_text_warns_without_refusing_the_package() {
    let dir = hud3_edited_dir(
        "hud3_small_text",
        &[("interface/main.ron", "size: 12, colour", "size: 10, colour")],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim_end(),
        "zeta/interface/main.ron: root[2].widget.size: text is 10 px; recommended minimum is 11 px"
    );
}

#[test]
fn hud3_small_button_warns_without_refusing_the_package() {
    let dir = hud3_edited_dir(
        "hud3_small_button",
        &[(
            "interface/main.ron",
            "w: Px(32.0), h: Px(32.0)",
            "w: Px(23.0), h: Px(23.0)",
        )],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(&dir)
        .output()
        .expect("turts-pack starts");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim_end(),
        "zeta/interface/main.ron: root[0].size: button is 23 x 23 px; recommended minimum is 24 x 24 px"
    );
}
