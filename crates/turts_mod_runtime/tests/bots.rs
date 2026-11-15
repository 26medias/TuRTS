use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use turts_mod_runtime::{
    archive::PackageArchive,
    pack::{pack_dir, pack_files},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn files(package: &str) -> BTreeMap<String, Vec<u8>> {
    fn visit(base: &Path, here: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(here).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                visit(base, &p, out)
            } else {
                out.insert(
                    p.strip_prefix(base).unwrap().to_str().unwrap().to_owned(),
                    std::fs::read(p).unwrap(),
                );
            }
        }
    }
    let base = root().join("mods").join(package);
    let mut result = BTreeMap::new();
    visit(&base, &base, &mut result);
    result
}
fn read(files: &BTreeMap<String, Vec<u8>>) -> PackageArchive {
    PackageArchive::parse(&pack_files(files).unwrap()).unwrap()
}
fn edit(files: &mut BTreeMap<String, Vec<u8>>, path: &str, from: &str, to: &str) {
    let s = String::from_utf8(files[path].clone()).unwrap();
    assert!(s.contains(from), "missing fixture needle {from}");
    files.insert(path.into(), s.replace(from, to).into_bytes());
}
#[test]
fn public_balance_inventory_and_fully_renamed_packages_load() {
    for package in ["skirmish", "stockyard"] {
        let archive =
            PackageArchive::parse(&pack_dir(&root().join("mods").join(package)).unwrap()).unwrap();
        let catalog = archive.content().unwrap();
        assert_eq!(catalog.bots.len(), 1);
        archive.visuals().unwrap();
        let mut renamed = files(package);
        for (path, bytes) in &mut renamed {
            if path.ends_with(".ron") {
                *bytes = String::from_utf8(bytes.clone())
                    .unwrap()
                    .replace(package, "different_game")
                    .into_bytes();
            }
        }
        let renamed_catalog = read(&renamed).content().unwrap();
        assert_eq!(renamed_catalog.bots.len(), 1);
        assert!(
            renamed_catalog
                .bots
                .keys()
                .all(|id| id.package.0 == "different_game")
        );
    }
}
#[test]
fn unavailable_custom_host_factory_is_portable() {
    let mut f = files("skirmish");
    edit(
        &mut f,
        "bots/opponent.ron",
        "engine:opening",
        "third_party:custom_controller",
    );
    assert!(read(&f).content().is_ok());
}
#[test]
fn invalid_references_capabilities_and_authored_keys_have_paths() {
    for (from, to, path) in [
        ("decision_ms: 1000", "decision_ms: 0", "decision_ms"),
        ("key: \"economy\"", "key: \"opening\"", "controllers[1].key"),
        (
            "key: \"expand_workers\"",
            "key: \"build_trainer\"",
            "knowledge.opening[1].key",
        ),
        (
            "requires: [Gatherer]",
            "requires: [Weapon]",
            "knowledge.roles[1].units[0]",
        ),
        (
            "recipe: \"skirmish:train_worker\"",
            "recipe: \"skirmish:missing\"",
            "knowledge.recipes[0].recipe",
        ),
        (
            "role: \"workers\", count: 6",
            "role: \"missing\", count: 6",
            "knowledge.opening[1].role",
        ),
        (
            "count: 6, recipe: \"worker\"",
            "count: 6, recipe: \"ranged\"",
            "knowledge.opening[1].recipe",
        ),
    ] {
        let mut f = files("skirmish");
        edit(&mut f, "bots/opponent.ron", from, to);
        let error = read(&f).content().unwrap_err();
        assert_eq!(error.field_path, path, "{error}");
        assert_eq!(error.file.as_deref(), Some("bots/opponent.ron"));
    }
}
#[test]
fn written_empty_bot_exports_still_require_api15() {
    let mut f = files("skirmish");
    edit(&mut f, "manifest.ron", "^0.2.15", "^0.2.14");
    edit(
        &mut f,
        "manifest.ron",
        "bots: [\"bots/opponent.ron\"]",
        "bots: []",
    );
    let error = read(&f).content().unwrap_err();
    assert!(error.to_string().contains("exports.bots"));
    assert!(error.to_string().contains("0.2.15"));
}
#[test]
fn bounded_configuration_and_duplicate_map_keys_fail_portably() {
    let mut f = files("skirmish");
    let nested = format!("{}Null{}", "List([".repeat(18), "])".repeat(18));
    edit(
        &mut f,
        "bots/opponent.ron",
        "config: Null",
        &format!("config: {nested}"),
    );
    assert!(
        read(&f)
            .content()
            .unwrap_err()
            .field_path
            .starts_with("controllers[0].config")
    );
    let mut f = files("skirmish");
    edit(
        &mut f,
        "bots/opponent.ron",
        "config: Null",
        "config: Map({\"duplicate\": Null, \"duplicate\": Bool(true)})",
    );
    assert!(read(&f).content().is_err());
}
#[test]
fn inventory_fixture_has_separate_constructor_and_producer_and_physical_costs() {
    let catalog = read(&files("stockyard")).content().unwrap();
    let bot = catalog.bots.values().next().unwrap();
    let builders = &bot
        .knowledge
        .roles
        .iter()
        .find(|r| r.key == "builders")
        .unwrap()
        .units;
    let trainers = &bot
        .knowledge
        .roles
        .iter()
        .find(|r| r.key == "worker_trainers")
        .unwrap()
        .units;
    assert!(builders.iter().all(|b| !trainers.contains(b)));
    assert!(
        catalog
            .production
            .values()
            .flat_map(|p| &p.costs)
            .all(|c| c.source == turts_mod_api::CostSource::ProducerInventory)
    );
}
