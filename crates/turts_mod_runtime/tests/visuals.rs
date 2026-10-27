//! The client path check: visuals and units match one to one. The check is shared by the
//! client and `turts-pack`. Test code may name the example packages.

mod common;

use std::collections::BTreeMap;

use common::{acme_with, package_dir, read_files};
use turts_mod_api::{ModError, Shape};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::pack::pack_files;
use turts_mod_runtime::visuals::visuals_by_content;

const SCOUT_VISUAL: &str =
    "(target: Unit(\"acme:scout\"), shape: Capsule(radius: 0.3, height: 1.0))";

fn archive(files: &BTreeMap<String, Vec<u8>>) -> PackageArchive {
    PackageArchive::parse(&pack_files(files).expect("pack in memory")).expect("parse")
}

/// The `acme` package with a second visual file, `visuals/second.ron`, that holds `text`.
fn acme_with_second_visual(text: &str) -> BTreeMap<String, Vec<u8>> {
    let mut files = acme_with(
        "manifest.ron",
        "visuals: [\"visuals/scout.ron\"]",
        "visuals: [\"visuals/scout.ron\", \"visuals/second.ron\"]",
    );
    files.insert("visuals/second.ron".to_string(), text.as_bytes().to_vec());
    files
}

fn refused(files: &BTreeMap<String, Vec<u8>>) -> ModError {
    let error = visuals_by_content(&[archive(files)]).expect_err("the package must be refused");
    for name in ["UnitDef", "VisualDef", "Shape", "Exports"] {
        assert!(!error.message.contains(name), "{error}");
    }
    error
}

#[test]
fn the_control_package_has_one_visual_per_unit() {
    let visuals = visuals_by_content(&[archive(&read_files(&package_dir("acme")))]).unwrap();
    assert_eq!(visuals.len(), 1);
    assert_eq!(
        visuals["acme:scout"],
        Shape::Cuboid {
            x: 1.0,
            y: 0.8,
            z: 1.4
        }
    );
}

#[test]
fn a_visual_of_an_unknown_unit_names_its_file_and_the_unit_field() {
    let error = refused(&acme_with_second_visual(
        "(target: Unit(\"acme:ghost\"), shape: Capsule(radius: 0.3, height: 1.0))",
    ));
    assert_eq!(error.package, "acme");
    assert_eq!(error.file.as_deref(), Some("visuals/second.ron"));
    assert_eq!(error.field_path, "target");
    assert!(error.message.contains("`acme:ghost`"), "{error}");
    assert!(
        error
            .allowed
            .as_ref()
            .is_some_and(|ids| ids.contains(&"acme:scout".to_string()))
    );
}

#[test]
fn a_second_visual_for_a_unit_names_the_second_file() {
    let error = refused(&acme_with_second_visual(SCOUT_VISUAL));
    assert_eq!(error.file.as_deref(), Some("visuals/second.ron"));
    assert_eq!(error.field_path, "target");
    assert!(error.message.contains("`visuals/scout.ron`"), "{error}");
}

#[test]
fn a_unit_without_a_visual_names_the_manifest_and_exports_visuals() {
    let files = acme_with(
        "manifest.ron",
        "visuals: [\"visuals/scout.ron\"]",
        "visuals: []",
    );
    let error = refused(&files);
    assert_eq!(error.file.as_deref(), Some("manifest.ron"));
    assert_eq!(error.field_path, "exports.visuals");
    assert!(error.message.contains("`acme:scout`"), "{error}");
}

#[test]
fn a_visual_that_does_not_parse_is_an_error_of_its_file() {
    let error = refused(&acme_with_second_visual(
        "(target: Unit(\"acme:scout\"), shape: Sphere)",
    ));
    assert_eq!(error.file.as_deref(), Some("visuals/second.ron"));
    assert!(
        error.message.contains("unknown variant `Sphere`"),
        "{error}"
    );
}
