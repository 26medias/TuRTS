//! T2c: a package the engine must refuse is refused with the right field path and a message
//! that names no Rust type.

mod common;

use common::{acme_archive, acme_archive_with};
use turts_mod_api::{ModError, Shape};
use turts_mod_runtime::{archive::PackageArchive, resolve::resolve};

/// Rust type names that must never reach a modder.
const TYPE_NAMES: [&str; 8] = [
    "Manifest",
    "UnitDef",
    "MapDef",
    "VisualDef",
    "Exports",
    "Footprint",
    "Shape",
    "Placement",
];

fn assert_no_type_names(error: &ModError) {
    for name in TYPE_NAMES {
        assert!(
            !error.message.contains(name),
            "message names `{name}`: {}",
            error.message
        );
    }
}

/// Parse an archive that must be refused, and check the shared properties of the error.
fn refused(bytes: &[u8], field_path: &str) -> ModError {
    let error = PackageArchive::parse(bytes).expect_err("the package must be refused");
    assert_eq!(error.field_path, field_path, "{error}");
    assert_no_type_names(&error);
    error
}

#[test]
fn t2c_control_unmodified_package_is_accepted() {
    let archive = PackageArchive::parse(&acme_archive()).unwrap();
    assert_eq!(archive.units().unwrap().len(), 1);
    assert_eq!(archive.maps().unwrap().len(), 1);
    assert_eq!(archive.visuals().unwrap().len(), 1);
}

#[test]
fn t2c_engine_api_mismatch() {
    let bytes = acme_archive_with(
        "manifest.ron",
        "engine_api: \"^0.2.3\"",
        "engine_api: \"^0.1\"",
    );
    let error = refused(&bytes, "engine_api");
    assert!(error.message.contains("`^0.1`"), "{error}");
    assert!(error.message.contains("0.2.3"), "{error}");
}

#[test]
fn t2c_engine_api_that_is_not_a_requirement() {
    let bytes = acme_archive_with(
        "manifest.ron",
        "engine_api: \"^0.2.3\"",
        "engine_api: \"soon\"",
    );
    refused(&bytes, "engine_api");
}

#[test]
fn t2c_dependencies_parse_then_resolve_refuses() {
    let bytes = acme_archive_with(
        "manifest.ron",
        "dependencies: {}",
        "dependencies: {\"lib\": \"^1\"}",
    );
    let archive = PackageArchive::parse(&bytes).unwrap();
    let error = resolve(&archive.manifest, &[]).unwrap_err();
    assert_eq!(error.field_path, "dependencies");
    assert!(error.message.contains("`lib`"), "{error}");
    assert!(error.message.contains("`^1`"), "{error}");
    assert!(error.message.contains("not provided"), "{error}");
}

#[test]
fn t2c_unknown_manifest_field() {
    let bytes = acme_archive_with(
        "manifest.ron",
        "    id: \"acme\",",
        "    hash: \"sha256:00\",\n    id: \"acme\",",
    );
    let error = refused(&bytes, "manifest.ron");
    assert!(error.message.contains("unknown field `hash`"), "{error}");
    assert_eq!(error.file.as_deref(), Some("manifest.ron"));
}

#[test]
fn t2c_bad_version_and_bad_id() {
    let bytes = acme_archive_with("manifest.ron", "version: \"0.3.0\"", "version: \"one\"");
    refused(&bytes, "version");
    let bytes = acme_archive_with("manifest.ron", "id: \"acme\"", "id: \"Acme\"");
    refused(&bytes, "id");
}

#[test]
fn t2c_manifest_disagrees_with_lock() {
    let archive = PackageArchive::parse(&acme_archive()).unwrap();
    let entry = archive.lock_entry(vec![]);
    // Control: the entry built from the archive agrees with it.
    assert_eq!(archive.check_against_lock(&entry), Ok(()));

    let mut version = entry.clone();
    version.version = "9.9.9".to_string();
    let error = archive.check_against_lock(&version).unwrap_err();
    assert_eq!(error.field_path, "lock.version");
    assert!(error.message.contains("9.9.9"), "{error}");

    let mut hash = entry.clone();
    let last = hash.hash.pop().unwrap();
    hash.hash.push(if last == '0' { '1' } else { '0' });
    let error = archive.check_against_lock(&hash).unwrap_err();
    assert_eq!(error.field_path, "lock.hash");
    assert!(error.message.contains(&entry.hash), "{error}");
    assert!(error.message.contains(&hash.hash), "{error}");

    // The first mismatch wins: version precedes hash.
    let mut both = hash.clone();
    both.version = "9.9.9".to_string();
    assert_eq!(
        archive.check_against_lock(&both).unwrap_err().field_path,
        "lock.version"
    );

    // Every other field the archive can answer for.
    let mut id = entry.clone();
    id.id = "other".to_string();
    assert_eq!(
        archive.check_against_lock(&id).unwrap_err().field_path,
        "lock.id"
    );
    let mut size = entry.clone();
    size.size += 1;
    assert_eq!(
        archive.check_against_lock(&size).unwrap_err().field_path,
        "lock.size"
    );
    let mut context = entry.clone();
    context.context = turts_mod_api::ExecutionContext::Server;
    assert_eq!(
        archive.check_against_lock(&context).unwrap_err().field_path,
        "lock.context"
    );
    let mut engine_api = entry;
    engine_api.engine_api = "^0.1".to_string();
    assert_eq!(
        archive
            .check_against_lock(&engine_api)
            .unwrap_err()
            .field_path,
        "lock.engine_api"
    );
}

#[test]
fn t2c_malformed_visual_loads_on_server_path_and_fails_on_client_path() {
    let bytes = acme_archive_with(
        "visuals/scout.ron",
        "shape: Cuboid(x: 1.0, y: 0.8, z: 1.4)",
        "shape: Sphere(r: 1.0)",
    );
    // The server path: parse, units, maps. The server never reads visuals.
    let archive = PackageArchive::parse(&bytes).expect("parse is Ok");
    archive.units().expect("units are Ok");
    archive.maps().expect("maps are Ok");
    // The client path.
    let error = archive.visuals().expect_err("visuals must fail");
    assert!(
        error.message.starts_with("unknown variant `Sphere`"),
        "{error}"
    );
    assert_eq!(
        error.allowed,
        Some(vec!["Capsule".to_string(), "Cuboid".to_string()])
    );
    assert_eq!(error.file.as_deref(), Some("visuals/scout.ron"));
    assert_eq!(error.field_path, "visuals/scout.ron");
    assert_no_type_names(&error);
}

#[test]
fn t2c_unnamed_cuboid_fields_are_rejected_on_the_client_path() {
    let bytes = acme_archive_with(
        "visuals/scout.ron",
        "Cuboid(x: 1.0, y: 0.8, z: 1.4)",
        "Cuboid(1.0, 0.8, 1.4)",
    );
    let archive = PackageArchive::parse(&bytes).unwrap();
    assert!(archive.visuals().is_err());
}

#[test]
fn t2c_unmodified_visual_has_named_fields() {
    let archive = PackageArchive::parse(&acme_archive()).unwrap();
    let visuals = archive.visuals().unwrap();
    assert_eq!(
        visuals[0].1.shape,
        Shape::Cuboid {
            x: 1.0,
            y: 0.8,
            z: 1.4
        }
    );
}

#[test]
fn t2c_two_maps_rejected() {
    let bytes = acme_archive_with(
        "manifest.ron",
        "maps: [\"maps/field.ron\"]",
        "maps: [\"maps/field.ron\", \"maps/other.ron\"]",
    );
    let error = refused(&bytes, "exports.maps");
    assert!(error.message.contains("found 2"), "{error}");
}

#[test]
fn t2c_export_paths_must_stay_inside_the_archive() {
    for (field, bad) in [
        ("exports.units", "units: [\"../scout.ron\"]"),
        ("exports.units", "units: [\"/units/scout.ron\"]"),
        ("exports.units", "units: [\"units/scout.txt\"]"),
        ("exports.visuals", "visuals: [\"visuals/../scout.ron\"]"),
    ] {
        let from = if field == "exports.units" {
            "units: [\"units/scout.ron\"]"
        } else {
            "visuals: [\"visuals/scout.ron\"]"
        };
        let bytes = acme_archive_with("manifest.ron", from, bad);
        refused(&bytes, field);
    }
}

#[test]
fn t2c_malformed_ron_names_no_type() {
    let cases = [
        "[1]",
        "(",
        "Circle(radius: 0.4)",
        "(id: \"scout\", name: \"Scout\", footprint: 3, movement: None, inventory: None, gatherer: None, drop_off: None, producer: None)",
    ];
    for text in cases {
        let error = turts_mod_runtime::parse::parse_ron::<turts_mod_api::UnitDef>(
            "pkg",
            "units/one.ron",
            text,
        )
        .expect_err("malformed text must be refused");
        assert_no_type_names(&error);
    }
}

/// What a modder reads: an unknown field and an unknown variant both list the names that the
/// file accepts, and the file name appears once (M1 verify, B4).
#[test]
fn t2c_unknown_names_print_the_allowed_names_and_the_file_once() {
    let archive = PackageArchive::parse(&acme_archive_with(
        "units/scout.ron",
        "movement: Some((speed: 6.0))",
        "movenment: Some((speed: 6.0))",
    ))
    .unwrap();
    let text = archive.units().unwrap_err().to_string();
    assert_eq!(
        text,
        "acme/units/scout.ron: unknown field `movenment` at line 1 column 62; did you mean `movement`? \
         (allowed: id, name, footprint, movement, inventory, gatherer, drop_off, producer, output_inventory, extractor, conveyor, inserter, machine, research, health, weapon, removal)"
    );
    assert_eq!(text.matches("units/scout.ron").count(), 1, "{text}");

    let archive = PackageArchive::parse(&acme_archive_with(
        "units/scout.ron",
        "Circle(radius: 0.6)",
        "Square(side: 0.6)",
    ))
    .unwrap();
    let text = archive.units().unwrap_err().to_string();
    assert!(text.contains("unknown variant `Square`"), "{text}");
    assert!(text.ends_with("(allowed: Circle)"), "{text}");

    // The manifest is parsed before the package id is known: no empty package, one file.
    let error = PackageArchive::parse(&acme_archive_with(
        "manifest.ron",
        "    id: \"acme\",",
        "    hash: \"sha256:00\",\n    id: \"acme\",",
    ))
    .unwrap_err()
    .to_string();
    assert!(
        error.starts_with("manifest.ron: unknown field `hash`"),
        "{error}"
    );
    assert!(error.contains("(allowed: id, version, "), "{error}");
}
