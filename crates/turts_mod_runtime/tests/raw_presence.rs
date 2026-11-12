//! L0-RAW: the version table reads the source file, not the parsed value (design 6.10,
//! "Version-table detection"). A manifest that writes a versioned export group, also with an
//! empty list, needs the API of that group. A field that is always written (`dependencies`)
//! and a variant (`kind: Library`) stay on the parsed value.

use std::collections::BTreeMap;

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
