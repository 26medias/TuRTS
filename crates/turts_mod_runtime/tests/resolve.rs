use std::collections::BTreeMap;

use turts_mod_api::{ExecutionContext, Exports, Manifest, PackageId, PackageKind, UniqueMap};
use turts_mod_runtime::resolve::resolve;

fn manifest(id: &str, kind: PackageKind, dependencies: &[(&str, &str)]) -> Manifest {
    Manifest {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        kind,
        engine_api: "^0.2.2".to_string(),
        dependencies: UniqueMap(
            dependencies
                .iter()
                .map(|(id, requirement)| (id.to_string(), requirement.to_string()))
                .collect::<BTreeMap<_, _>>(),
        ),
        exports: Exports {
            units: vec![],
            maps: if kind == PackageKind::Game {
                vec!["maps/game.ron".to_string()]
            } else {
                vec![]
            },
            visuals: vec![],
            resources: vec![],
            nodes: vec![],
            production: vec![],
            recipes: vec![],
            technologies: vec![],
        },
        context: ExecutionContext::Shared,
        capabilities: vec![],
        license: "test".to_string(),
    }
}

fn assert_rule(error: turts_mod_api::ModError, word: &str) {
    assert_eq!(error.field_path, "dependencies");
    assert!(error.message.contains(word), "{error}");
    assert_eq!(error.file.as_deref(), Some("manifest.ron"));
}

#[test]
fn lr1_rejects_invalid_dependency_identifier() {
    let root = manifest("root", PackageKind::Game, &[("bad-id", "^1")]);
    assert_rule(resolve(&root, &[]).unwrap_err(), "character");
}

#[test]
fn lr1_rejects_invalid_dependency_requirement() {
    let root = manifest("root", PackageKind::Game, &[("lib", "soon")]);
    assert_rule(
        resolve(&root, &[]).unwrap_err(),
        "invalid version requirement",
    );
}

#[test]
fn lr2_rejects_self_dependency() {
    let root = manifest("root", PackageKind::Game, &[("root", "^1")]);
    assert_rule(resolve(&root, &[]).unwrap_err(), "itself");
}

#[test]
fn lr3_rejects_missing_dependency() {
    let root = manifest("root", PackageKind::Game, &[("lib", "^1")]);
    assert_rule(resolve(&root, &[]).unwrap_err(), "not provided");
}

#[test]
fn lr3_rejects_duplicate_provided_versions() {
    let root = manifest("root", PackageKind::Game, &[("lib", "^1")]);
    let libraries = [
        manifest("lib", PackageKind::Library, &[]),
        manifest("lib", PackageKind::Library, &[]),
    ];
    assert_rule(resolve(&root, &libraries).unwrap_err(), "times");
}

#[test]
fn lr4_rejects_incompatible_dependency_version() {
    let root = manifest("root", PackageKind::Game, &[("lib", "^2")]);
    let library = manifest("lib", PackageKind::Library, &[]);
    assert_rule(resolve(&root, &[library]).unwrap_err(), "does not match");
}

#[test]
fn lr5_rejects_game_as_dependency() {
    let root = manifest("root", PackageKind::Game, &[("lib", "^1")]);
    let game = manifest("lib", PackageKind::Game, &[]);
    assert_rule(resolve(&root, &[game]).unwrap_err(), "only libraries");
}

#[test]
fn lr6_rejects_dependency_cycle() {
    let root = manifest("root", PackageKind::Game, &[("lib", "^1")]);
    let libraries = [
        manifest("lib", PackageKind::Library, &[("other", "^1")]),
        manifest("other", PackageKind::Library, &[("lib", "^1")]),
    ];
    assert_rule(resolve(&root, &libraries).unwrap_err(), "cycle");
}

#[test]
fn lr7_rejects_library_root() {
    let root = manifest("root", PackageKind::Library, &[]);
    let error = resolve(&root, &[]).unwrap_err();
    assert_eq!(error.field_path, "kind");
    assert_eq!(error.message, "a library cannot be the root of a match");
    assert_eq!(error.file.as_deref(), Some("manifest.ron"));
}

#[test]
fn lr8_rejects_unreachable_provided_package() {
    let root = manifest("root", PackageKind::Game, &[]);
    let library = manifest("unused", PackageKind::Library, &[]);
    assert_rule(resolve(&root, &[library]).unwrap_err(), "not reachable");
}

#[test]
fn order_is_dependencies_first_then_id() {
    let root = manifest("a", PackageKind::Game, &[("b", "^1"), ("c", "^1")]);
    let libraries = [
        manifest("b", PackageKind::Library, &[("c", "^1")]),
        manifest("c", PackageKind::Library, &[]),
    ];
    assert_eq!(
        resolve(&root, &libraries).unwrap(),
        vec![
            PackageId("c".to_string()),
            PackageId("b".to_string()),
            PackageId("a".to_string())
        ]
    );
}

#[test]
fn a_root_without_dependencies_resolves_to_itself() {
    let root = manifest("root", PackageKind::Game, &[]);
    assert_eq!(
        resolve(&root, &[]).unwrap(),
        vec![PackageId("root".to_string())]
    );
}
