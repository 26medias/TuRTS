use std::{
    fs,
    path::{Path, PathBuf},
};

use turts_mod_api::schema::{MapDef, UnitDef};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn assert_package_ron_loads(path: &Path) {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
    let relative = path.strip_prefix(workspace_root()).unwrap();
    let kind = if relative
        .file_name()
        .is_some_and(|name| name == "manifest.ron")
    {
        "manifest".into()
    } else {
        relative
            .components()
            .nth(2)
            .unwrap()
            .as_os_str()
            .to_string_lossy()
    };
    let parsed = match kind.as_ref() {
        "manifest" => ron::from_str::<turts_mod_api::schema::Manifest>(&text).map(|_| ()),
        "units" => ron::from_str::<UnitDef>(&text).map(|_| ()),
        "maps" => ron::from_str::<MapDef>(&text).map(|_| ()),
        "visuals" => ron::from_str::<turts_mod_api::schema::VisualDef>(&text).map(|_| ()),
        "resources" => ron::from_str::<turts_mod_api::schema::ResourceDef>(&text).map(|_| ()),
        "nodes" => ron::from_str::<turts_mod_api::schema::NodeDef>(&text).map(|_| ()),
        "production" => ron::from_str::<turts_mod_api::schema::ProductionDef>(&text).map(|_| ()),
        _ => panic!("unexpected package path {}", relative.display()),
    };
    parsed.unwrap_or_else(|error| panic!("{}: {error}", relative.display()));
}

fn visit_package_ron(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit_package_ron(&path);
        } else if path.extension().is_some_and(|extension| extension == "ron") {
            assert_package_ron_loads(&path);
        }
    }
}

#[test]
fn forge_shaped_ron_parses() {
    let unit = r#"(
        id: "shell_turret",
        name: "Shell Turret",
        footprint: Circle(radius: 1.2),
        movement: None,
        inventory: Some((capacity: 5, accepts: ["forge:shell"])),
        gatherer: None,
        drop_off: None,
        producer: None,
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: Some((max: 100)),
        weapon: Some((range: 8.0, cooldown_ms: 1000, damage: 10,
            ammo: Some((item: "forge:shell", per_shot: 1)),
            targets: (slots: Other))),
        removal: Some((by_command: false, contents: Destroy)),
    )"#;
    let recipe = r#"(
        id: "make_shells", name: "Make Shells",
        inputs: [(item: "forge:ore", amount: 2)],
        outputs: [(item: "forge:shell", amount: 1)], duration_ms: 2000,
        cancel_refunds: (running: (numerator: 1, denominator: 1),
            output_blocked: (numerator: 1, denominator: 1)),
    )"#;
    let technology = r#"(
        id: "hardened_shells", name: "Hardened Shells", prerequisites: [],
        duration_ms: 10000,
        costs: [(resource: "forge:shell", amount: 10, source: ProducerInventory)],
        payment: ReserveThenCharge,
        cancel_refunds: (queued: (numerator: 1, denominator: 1),
            running: (numerator: 1, denominator: 2),
            output_blocked: (numerator: 1, denominator: 1)),
        effects: [WeaponModifier(unit: "forge:shell_turret", stat: Damage, add: 10)],
    )"#;
    let map = r#"(
        id: "proving_ground", size: (x: 32.0, y: 16.0),
        placements: [(unit: "forge:shell_turret", owner_slot: 0,
            pos: (x: 10.0, y: 8.0), contents: [], facing: Some(PosX))],
        nodes: [], slot_economy: [],
        belts: [(conveyor: "forge:chain_belt", owner_slot: 0,
            start: (x: 4.0, y: 8.0), facing: PosX, pieces: 6)],
    )"#;

    assert!(ron::from_str::<UnitDef>(unit).is_ok());
    assert!(ron::from_str::<turts_mod_api::schema::RecipeDef>(recipe).is_ok());
    assert!(ron::from_str::<turts_mod_api::schema::TechnologyDef>(technology).is_ok());
    assert!(ron::from_str::<MapDef>(map).is_ok());
}

#[test]
fn old_ron_still_loads() {
    let root = workspace_root();
    for package in ["base", "haul", "acme", "cedar"] {
        visit_package_ron(&root.join("mods").join(package));
    }
}

#[test]
fn unknown_field_typo_rejected() {
    let unit = r#"(
        id: "worker", name: "Carrier", footprint: Circle(radius: 0.5),
        movement: None, inventory: None, gatherer: None, drop_off: None, producer: None,
        output_inventroy: None,
    )"#;
    assert!(ron::from_str::<UnitDef>(unit).is_err());
}
