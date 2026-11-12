//! L0-RAW: the version table reads the source file, not the parsed value (design 6.10,
//! "Version-table detection"). A manifest that writes a versioned export group, also with an
//! empty list, needs the API of that group. A field that is always written (`dependencies`)
//! and a variant (`kind: Library`) stay on the parsed value.

use std::collections::BTreeMap;

mod common;

use turts_mod_api::{ENGINE_API_VERSION, ModError};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

const MAP: &str = r#"(id:"map",size:(x:10.0,y:10.0),placements:[],nodes:[],slot_economy:[])"#;

/// A package with one map and the given `exports` tail, `kind` and `dependencies` text.
fn package(engine_api: &str, kind: &str, dependencies: &str, exports_tail: &str) -> PackageArchive {
    let manifest = format!(
        r#"(id: "zeta", version: "0.1.0", kind: {kind}, engine_api: "{engine_api}", dependencies: {dependencies}, exports: (units: [], maps: ["maps/map.ron"], visuals: [], resources: [], nodes: [], production: []{exports_tail}), context: Shared, capabilities: [], license: "test")"#
    );
    let files = BTreeMap::from([
        ("manifest.ron".to_string(), manifest.into_bytes()),
        ("maps/map.ron".to_string(), MAP.as_bytes().to_vec()),
    ]);
    PackageArchive::parse(&pack_files(&files).expect("pack in memory")).expect("manifest parses")
}

fn refusal(archive: &PackageArchive) -> ModError {
    archive
        .content()
        .expect_err("the version table must refuse this package")
}

/// An explicit empty list of each 0.2.3 export group gets the version refusal. This is the
/// red of task L0-RAW: today the presence is `!list.is_empty()` of the parsed value.
#[test]
fn explicit_empty_export_group_is_refused_below_its_api() {
    for group in ["orders", "interface", "input", "assets"] {
        let archive = package("^0.2.2", "Game", "{}", &format!(", {group}: []"));
        assert_eq!(
            refusal(&archive).to_string(),
            format!(
                "zeta/manifest.ron: engine_api: `exports.{group}` needs engine api 0.2.3; `^0.2.2` admits 0.2.2"
            ),
            "group {group}"
        );
    }
}

/// The field that is not written is not "used": no refusal. Control of the test above.
#[test]
fn an_absent_export_group_is_accepted_below_its_api() {
    package("^0.2.2", "Game", "{}", "")
        .content()
        .expect("a package with no 0.2.3 field packs on ^0.2.2");
}

/// The same explicit empty list is accepted when the requirement admits only 0.2.3 or later.
#[test]
fn explicit_empty_export_group_is_accepted_from_its_api() {
    for group in ["orders", "interface", "input", "assets"] {
        package("^0.2.3", "Game", "{}", &format!(", {group}: []"))
            .content()
            .unwrap_or_else(|error| panic!("{group}: {error}"));
    }
}

/// The group is detected in the document, not in the text: a line break and a comment
/// between the key and its value do not hide it.
#[test]
fn raw_detection_reads_the_document_not_the_text() {
    let manifest = r#"(id:"zeta",version:"0.1.0",kind:Game,engine_api:"^0.2.2",dependencies:{},exports:(units:[],maps:["maps/map.ron"],visuals:[],resources:[],nodes:[],production:[],
        input /* written, empty */
        :
        []),context:Shared,capabilities:[],license:"test")"#;
    let files = BTreeMap::from([
        ("manifest.ron".to_string(), manifest.as_bytes().to_vec()),
        ("maps/map.ron".to_string(), MAP.as_bytes().to_vec()),
    ]);
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert_eq!(
        refusal(&archive).to_string(),
        "zeta/manifest.ron: engine_api: `exports.input` needs engine api 0.2.3; `^0.2.2` admits 0.2.2"
    );
}

/// `dependencies` is a required manifest field: every manifest writes it, also `{}`. It stays
/// on the parsed value, so an old package that writes `dependencies: {}` is not refused.
/// A raw key walk for it would refuse every package below 0.2.2.
#[test]
fn the_required_dependencies_field_is_detected_on_the_parsed_value() {
    package("^0.2.1", "Game", "{}", "")
        .content()
        .expect("`dependencies: {}` is not a use of 0.2.2");
    let archive = package("^0.2.1", "Game", r#"{"ai_core": "^1.0"}"#, "");
    assert_eq!(
        refusal(&archive).to_string(),
        "zeta/manifest.ron: engine_api: `dependencies` needs engine api 0.2.2; `^0.2.1` admits 0.2.1"
    );
}

/// `kind: Library` is a variant: detected on the parsed value.
#[test]
fn a_library_below_its_api_is_refused() {
    let manifest = r#"(id: "zeta", version: "0.1.0", kind: Library, engine_api: "^0.2.1", dependencies: {}, exports: (units: [], maps: [], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#;
    let files = BTreeMap::from([("manifest.ron".to_string(), manifest.as_bytes().to_vec())]);
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert_eq!(
        refusal(&archive).to_string(),
        "zeta/manifest.ron: engine_api: `kind: Library` needs engine api 0.2.2; `^0.2.1` admits 0.2.1"
    );
}

/// The released API of this tree stays 0.2.3 in this task (the bump is in L0-API1).
#[test]
fn the_engine_api_version_is_unchanged() {
    assert_eq!(ENGINE_API_VERSION, "0.2.3");
}

// ---- Review repairs (M2, L4, L7): one presence source, the file named, RAW rows first ----

const UNIT: &str = r#"(id: "turret", name: "Turret", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None"#;

/// A package with one unit and one map at `engine_api`; `unit_tail` and `map_tail` are written
/// into the unit and the map, `exports_tail` into `exports`.
fn unit_package(
    engine_api: &str,
    exports_tail: &str,
    unit_tail: &str,
    map_tail: &str,
) -> PackageArchive {
    let manifest = format!(
        r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "{engine_api}", dependencies: {{}}, exports: (units: ["units/turret.ron"], maps: ["maps/map.ron"], visuals: [], resources: [], nodes: [], production: []{exports_tail}), context: Shared, capabilities: [], license: "test")"#
    );
    let map = format!(
        r#"(id:"map",size:(x:10.0,y:10.0),placements:[]{map_tail},nodes:[],slot_economy:[])"#
    );
    let files = BTreeMap::from([
        ("manifest.ron".to_string(), manifest.into_bytes()),
        (
            "units/turret.ron".to_string(),
            format!("{UNIT}{unit_tail})").into_bytes(),
        ),
        ("maps/map.ron".to_string(), map.into_bytes()),
    ]);
    PackageArchive::parse(&pack_files(&files).expect("pack in memory")).expect("package parses")
}

/// M2: the manifest in its `Manifest(..)` and `Exports(..)` forms, with no new group, is not
/// a use of a new API. The second presence parser refused it with an internal type name.
#[test]
fn named_struct_forms_without_a_new_group_are_accepted() {
    let manifest = r#"Manifest(id: "zeta", version: "0.1.0", kind: Game, engine_api: "^0.2.2", dependencies: {}, exports: Exports(units: [], maps: ["maps/map.ron"], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#;
    let files = BTreeMap::from([
        ("manifest.ron".to_string(), manifest.as_bytes().to_vec()),
        ("maps/map.ron".to_string(), MAP.as_bytes().to_vec()),
    ]);
    PackageArchive::parse(&pack_files(&files).unwrap())
        .unwrap()
        .content()
        .expect("named forms of the manifest are accepted");
}

/// The named forms with a new group are refused for the group, with the table text.
#[test]
fn named_struct_forms_with_a_new_group_get_the_table_text() {
    let manifest = r#"Manifest(id: "zeta", version: "0.1.0", kind: Game, engine_api: "^0.2.2", dependencies: {}, exports: Exports(units: [], maps: ["maps/map.ron"], visuals: [], resources: [], nodes: [], production: [], orders: []), context: Shared, capabilities: [], license: "test")"#;
    let files = BTreeMap::from([
        ("manifest.ron".to_string(), manifest.as_bytes().to_vec()),
        ("maps/map.ron".to_string(), MAP.as_bytes().to_vec()),
    ]);
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert_eq!(
        refusal(&archive).to_string(),
        "zeta/manifest.ron: engine_api: `exports.orders` needs engine api 0.2.3; `^0.2.2` admits 0.2.2"
    );
}

/// M2: one kind of mistake, one text. `recipes: []` at `^0.2` gets the table text, not a
/// second text from a second parser.
#[test]
fn an_explicit_empty_m3_export_gets_the_table_text() {
    for group in ["recipes", "technologies"] {
        let archive = unit_package("^0.2", &format!(", {group}: []"), "", "");
        assert_eq!(
            refusal(&archive).to_string(),
            format!(
                "zeta/manifest.ron: engine_api: `exports.{group}` needs engine api 0.2.1; `^0.2` admits 0.2.0"
            ),
            "group {group}"
        );
    }
}

/// M2 and L4: every M3 field of a unit and of a map, written with its default value, gets the
/// table text, and the message names the file and the path where the field is written.
#[test]
fn every_m3_field_written_empty_gets_the_table_text_and_the_file() {
    let unit_fields = [
        "output_inventory",
        "extractor",
        "conveyor",
        "inserter",
        "machine",
        "research",
        "health",
        "weapon",
        "removal",
    ];
    for field in unit_fields {
        let archive = unit_package("^0.2", "", &format!(", {field}: None"), "");
        assert_eq!(
            refusal(&archive).to_string(),
            format!(
                "zeta/manifest.ron: engine_api: `unit.{field}` needs engine api 0.2.1; `^0.2` admits 0.2.0 (written in units/turret.ron: {field})"
            ),
            "unit field {field}"
        );
        // Control: the same file at `^0.2.1` is accepted.
        unit_package("^0.2.1", "", &format!(", {field}: None"), "")
            .content()
            .unwrap_or_else(|error| panic!("{field} at ^0.2.1: {error}"));
    }
    let archive = unit_package("^0.2", "", "", ", belts: []");
    assert_eq!(
        refusal(&archive).to_string(),
        "zeta/manifest.ron: engine_api: `map.belts` needs engine api 0.2.1; `^0.2` admits 0.2.0 (written in maps/map.ron: belts)"
    );
    let archive = unit_package("^0.2", "", "", "");
    archive
        .content()
        .expect("a map with no M3 field is accepted at ^0.2");
}

/// M2 and the path through a list: `placements[].facing`, written `None` in one placement.
#[test]
fn a_placement_facing_written_empty_gets_the_table_text() {
    let manifest = r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "^0.2", dependencies: {}, exports: (units: ["units/turret.ron"], maps: ["maps/map.ron"], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#;
    let map = r#"(id:"map",size:(x:10.0,y:10.0),placements:[(unit:"zeta:turret",owner_slot:0,pos:(x:1.0,y:1.0),contents:[]),(unit:"zeta:turret",owner_slot:0,pos:(x:2.0,y:2.0),contents:[],facing:None)],nodes:[],slot_economy:[])"#;
    let files = BTreeMap::from([
        ("manifest.ron".to_string(), manifest.as_bytes().to_vec()),
        (
            "units/turret.ron".to_string(),
            format!("{UNIT})").into_bytes(),
        ),
        ("maps/map.ron".to_string(), map.as_bytes().to_vec()),
    ]);
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert_eq!(
        refusal(&archive).to_string(),
        "zeta/manifest.ron: engine_api: `map.placements.facing` needs engine api 0.2.1; `^0.2` admits 0.2.0 (written in maps/map.ron: placements.facing)"
    );
}

/// L7: a package that uses a new group at an old API is told so first, not about a content
/// cross-check ("unknown image asset") that the missing groups of the same old API cause.
/// This is the package "orders with one entry at ^0.2.2": acme with the `orders` group only.
#[test]
fn the_version_table_speaks_before_the_content_checks() {
    let mut files = common::acme_with(
        "manifest.ron",
        "engine_api: \"^0.2.3\"",
        "engine_api: \"^0.2.2\"",
    );
    let manifest = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    let manifest = manifest
        .replace("input: [\"input/bindings.ron\"],", "")
        .replace("assets: [\"assets/assets.ron\"],", "")
        .replace("interface: [\"interface/hud.ron\"],", "");
    assert!(!manifest.contains("interface:") && manifest.contains("orders:"));
    files.insert("manifest.ron".to_string(), manifest.into_bytes());
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert_eq!(
        refusal(&archive).to_string(),
        "acme/manifest.ron: engine_api: `exports.orders` needs engine api 0.2.3; `^0.2.2` admits 0.2.2"
    );
}
