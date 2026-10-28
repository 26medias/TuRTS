use std::collections::BTreeMap;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

const MANIFEST: &str = r#"(id: "forge", version: "0.1.0", kind: Game, engine_api: "^0.2.1", dependencies: {}, exports: (units: ["units/turret.ron"], maps: ["maps/map.ron"], visuals: [], resources: ["resources/shell.ron"], nodes: [], production: [], recipes: ["recipes/ammo.ron"], technologies: ["technologies/guns.ron"]), context: Shared, capabilities: [], license: "MIT")"#;
const UNIT: &str = r#"(id: "turret", name: "Turret", footprint: Circle(radius: 1.0), movement: None, inventory: Some((capacity: 20, accepts: ["forge:shell"])), gatherer: None, drop_off: None, producer: None, output_inventory: Some((capacity: 10, accepts: ["forge:shell"])), extractor: None, conveyor: None, inserter: None, machine: Some((recipe: "forge:ammo")), research: None, health: None, weapon: None, removal: None)"#;
const RECIPE: &str = r#"(id: "ammo", name: "Ammo", inputs: [(item: "forge:shell", amount: 1)], outputs: [(item: "forge:shell", amount: 1)], duration_ms: 1000, cancel_refunds: (running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))"#;
const TECH: &str = r#"(id: "guns", name: "Guns", prerequisites: [], duration_ms: 1000, costs: [], payment: ChargeOnQueue, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)), effects: [])"#;
const ITEM: &str = r#"(id: "shell", name: "Shell", unit_name: "shell", kind: Item)"#;
const LEGACY_MANIFEST: &str = r#"(id: "forge", version: "0.1.0", kind: Game, engine_api: "^0.2", dependencies: {}, exports: (units: ["units/turret.ron"], maps: ["maps/map.ron"], visuals: [], resources: ["resources/shell.ron"], nodes: [], production: []), context: Shared, capabilities: [], license: "weapon: None")"#;
const LEGACY_UNIT: &str = r#"(id: "turret", name: "Turret", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None)"#;

fn archive(
    unit: &str,
    recipe: &str,
    tech: &str,
    engine_api: &str,
    extra: &[(&str, &str)],
) -> PackageArchive {
    let mut manifest = MANIFEST.replace("^0.2.1", engine_api);
    if extra
        .iter()
        .any(|(path, _)| *path == "resources/credit.ron")
    {
        manifest = manifest.replace(
            "resources: [\"resources/shell.ron\"]",
            "resources: [\"resources/shell.ron\", \"resources/credit.ron\"]",
        );
    }
    if extra.iter().any(|(path, _)| *path == "visuals/turret.ron") {
        manifest = manifest.replace("visuals: []", "visuals: [\"visuals/turret.ron\"]");
    }
    let mut files = BTreeMap::from([
        ("manifest.ron".into(), manifest.into_bytes()),
        ("units/turret.ron".into(), unit.into()),
        ("resources/shell.ron".into(), ITEM.into()),
        ("recipes/ammo.ron".into(), recipe.into()),
        ("technologies/guns.ron".into(), tech.into()),
        ("maps/map.ron".into(), r#"(id: "base_map", size: (x: 20.0, y: 20.0), placements: [], nodes: [], slot_economy: [])"#.into()),
    ]);
    for (path, text) in extra {
        files.insert((*path).into(), (*text).into());
    }
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}
const CREDIT: &str = r#"(id: "credit", name: "Credit", unit_name: "credit", kind: Balance)"#;
fn rejected(unit: &str, recipe: &str, tech: &str, field: &str) {
    rejected_with_extra(unit, recipe, tech, field, &[]);
}
fn rejected_with_extra(unit: &str, recipe: &str, tech: &str, field: &str, extra: &[(&str, &str)]) {
    let err = archive(unit, recipe, tech, "^0.2.1", extra)
        .content()
        .unwrap_err();
    assert_eq!(err.package, "forge");
    let expected_file = if field == "id" {
        "recipes/ammo.ron"
    } else if field.starts_with("effects[") || field.starts_with("prerequisites") {
        "technologies/guns.ron"
    } else {
        "units/turret.ron"
    };
    assert_eq!(err.file.as_deref(), Some(expected_file));
    assert_eq!(err.field_path, field, "{err:?}");
}
fn weapon_unit() -> String {
    UNIT.replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None").replace("output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))", "output_inventory: None").replace("weapon: None", "weapon: Some((range: 5.0, cooldown_ms: 1000, damage: 3, ammo: Some((item: \"forge:shell\", per_shot: 1)), targets: (slots: Any)))")
}

#[test]
fn forge_style_package_accepted() {
    archive(UNIT, RECIPE, TECH, "^0.2.1", &[])
        .content()
        .unwrap();
}
#[test]
fn machine_needs_matching_inventories() {
    rejected(
        &UNIT.replace(
            "inventory: Some((capacity: 20, accepts: [\"forge:shell\"]))",
            "inventory: None",
        ),
        RECIPE,
        TECH,
        "machine.inventory",
    );
}
#[test]
fn machine_inputs_exceed_input_capacity() {
    rejected(
        &UNIT.replace("capacity: 20", "capacity: 0"),
        &RECIPE.replace(
            "inputs: [(item: \"forge:shell\", amount: 1)]",
            "inputs: [(item: \"forge:shell\", amount: 21)]",
        ),
        TECH,
        "inventory.capacity",
    );
}
#[test]
fn machine_outputs_exceed_output_capacity() {
    rejected(
        UNIT,
        &RECIPE.replace(
            "outputs: [(item: \"forge:shell\", amount: 1)]",
            "outputs: [(item: \"forge:shell\", amount: 11)]",
        ),
        TECH,
        "output_inventory.capacity",
    );
}
#[test]
fn machine_must_be_static() {
    rejected(
        &UNIT.replace("movement: None", "movement: Some((speed: 1.0))"),
        RECIPE,
        TECH,
        "machine",
    );
}
#[test]
fn health_requires_removal() {
    rejected(
        &UNIT.replace("health: None", "health: Some((max: 100))"),
        RECIPE,
        TECH,
        "health",
    );
}
#[test]
fn weapon_must_be_static() {
    rejected(
        &weapon_unit().replace("movement: None", "movement: Some((speed: 1.0))"),
        RECIPE,
        TECH,
        "weapon",
    );
}
#[test]
fn weapon_ammo_item_not_accepted() {
    rejected(
        &weapon_unit().replace("accepts: [\"forge:shell\"]", "accepts: []"),
        RECIPE,
        TECH,
        "weapon.ammo.item",
    );
}
#[test]
fn modifier_unit_without_weapon() {
    rejected(
        &UNIT
            .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
            .replace(
                "output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))",
                "output_inventory: None",
            ),
        RECIPE,
        &TECH.replace(
            "effects: []",
            "effects: [WeaponModifier(unit: \"forge:turret\", stat: Damage, add: 1)]",
        ),
        "effects[0].unit",
    );
}
#[test]
fn prerequisite_cycle() {
    rejected(
        UNIT,
        RECIPE,
        &TECH.replace("prerequisites: []", "prerequisites: [\"forge:guns\"]"),
        "prerequisites",
    );
}
#[test]
fn missing_machine_recipe_rejects_without_panicking() {
    rejected(
        &UNIT.replace("forge:ammo", "forge:missing"),
        RECIPE,
        TECH,
        "machine.recipe",
    );
}
#[test]
fn missing_research_technology_rejects_without_panicking() {
    let unit = UNIT
        .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
        .replace(
            "output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))",
            "output_inventory: None",
        )
        .replace(
            "research: None",
            "research: Some((technologies: [\"forge:missing\"], queue_capacity: 1))",
        );
    rejected(&unit, RECIPE, TECH, "research.technologies[0]");
}
#[test]
fn missing_prerequisite_technology_rejects() {
    rejected(
        UNIT,
        RECIPE,
        &TECH.replace("prerequisites: []", "prerequisites: [\"forge:missing\"]"),
        "prerequisites[0]",
    );
}
#[test]
fn health_maximum_must_be_positive() {
    let unit = UNIT
        .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
        .replace(
            "output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))",
            "output_inventory: None",
        )
        .replace("health: None", "health: Some((max: 0))")
        .replace(
            "removal: None",
            "removal: Some((by_command: true, contents: Destroy))",
        );
    rejected(&unit, RECIPE, TECH, "health.max");
}
#[test]
fn conveyor_piece_slots_are_bounded() {
    for slots in [0, 17] {
        let unit = UNIT
            .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
            .replace("output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))", "output_inventory: None")
            .replace("conveyor: None", &format!("conveyor: Some((slots_per_piece: {slots}, slots_per_second: 1, accepts: [\"forge:shell\"]))"));
        rejected(&unit, RECIPE, TECH, "conveyor.slots_per_piece");
    }
}
#[test]
fn conveyor_slot_boundaries_are_accepted() {
    for slots in [1, 16] {
        let unit = UNIT
            .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
            .replace("output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))", "output_inventory: None")
            .replace("conveyor: None", &format!("conveyor: Some((slots_per_piece: {slots}, slots_per_second: 1, accepts: [\"forge:shell\"]))"));
        archive(&unit, RECIPE, TECH, "^0.2.1", &[])
            .content()
            .unwrap();
    }
}
#[test]
fn output_inventory_accepts_only_items() {
    let unit = UNIT.replace(
        "output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))",
        "output_inventory: Some((capacity: 10, accepts: [\"forge:credit\"]))",
    );
    rejected_with_extra(
        &unit,
        RECIPE,
        TECH,
        "output_inventory.accepts[0]",
        &[("resources/credit.ron", CREDIT)],
    );
}
#[test]
fn conveyor_accepts_only_items() {
    let unit = UNIT
        .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
        .replace("output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))", "output_inventory: None")
        .replace("conveyor: None", "conveyor: Some((slots_per_piece: 1, slots_per_second: 1, accepts: [\"forge:credit\"]))");
    rejected_with_extra(
        &unit,
        RECIPE,
        TECH,
        "conveyor.accepts[0]",
        &[("resources/credit.ron", CREDIT)],
    );
}
#[test]
fn research_queue_capacity_must_be_positive() {
    let unit = UNIT
        .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
        .replace(
            "output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))",
            "output_inventory: None",
        )
        .replace(
            "research: None",
            "research: Some((technologies: [\"forge:guns\"], queue_capacity: 0))",
        );
    rejected(&unit, RECIPE, TECH, "research.queue_capacity");
}
#[test]
fn research_station_requires_accepted_inventory_cost() {
    let unit = UNIT
        .replace("machine: Some((recipe: \"forge:ammo\"))", "machine: None")
        .replace(
            "output_inventory: Some((capacity: 10, accepts: [\"forge:shell\"]))",
            "output_inventory: None",
        )
        .replace(
            "research: None",
            "research: Some((technologies: [\"forge:guns\"], queue_capacity: 1))",
        );
    let tech = TECH.replace(
        "costs: []",
        "costs: [(resource: \"forge:shell\", amount: 1, source: ProducerInventory)]",
    );
    let invalid_unit = unit.replace("accepts: [\"forge:shell\"]", "accepts: []");
    rejected(&invalid_unit, RECIPE, &tech, "research.technologies");
}
#[test]
fn duplicate_id_across_recipe_and_unit() {
    rejected(
        UNIT,
        &RECIPE.replace("id: \"ammo\"", "id: \"turret\""),
        TECH,
        "id",
    );
}
#[test]
fn two_work_capabilities_on_one_unit() {
    let unit = UNIT.replace(
        "extractor: None",
        "extractor: Some((per_second: 1, interaction_range: 0.0))",
    );
    rejected(&unit, RECIPE, TECH, "extractor");
}
#[test]
fn removal_on_unit_with_queue() {
    rejected(
        &UNIT.replace(
            "removal: None",
            "removal: Some((by_command: true, contents: Destroy))",
        ),
        RECIPE,
        TECH,
        "removal",
    );
}
#[test]
fn item_count_over_u16() {
    let mut files = BTreeMap::from([
        (
            "manifest.ron".into(),
            MANIFEST
                .replace(
                    "[\"resources/shell.ron\"]",
                    &format!(
                        "[{}]",
                        (0..65536)
                            .map(|i| format!("\"resources/{i}.ron\""))
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                )
                .into_bytes(),
        ),
        ("units/turret.ron".into(), UNIT.into()),
        ("recipes/ammo.ron".into(), RECIPE.into()),
        ("technologies/guns.ron".into(), TECH.into()),
        (
            "maps/map.ron".into(),
            r#"(id: "map", size: (x: 10.0, y: 10.0), placements: [], nodes: [], slot_economy: [])"#
                .into(),
        ),
    ]);
    for i in 0..65536 {
        let id = if i == 0 {
            "shell".to_owned()
        } else {
            format!("i{i}")
        };
        files.insert(
            format!("resources/{i}.ron"),
            ITEM.replace("shell", &id).into_bytes(),
        );
    }
    let err = PackageArchive::parse(&pack_files(&files).unwrap())
        .unwrap()
        .content()
        .unwrap_err();
    assert_eq!(err.package, "forge");
    assert!(
        err.file
            .as_deref()
            .is_some_and(|file| file.starts_with("resources/")),
        "{err:?}"
    );
    assert_eq!(err.field_path, "id");
}
#[test]
fn m3_field_with_caret_0_2_rejected() {
    let err = archive(UNIT, RECIPE, TECH, "^0.2", &[])
        .content()
        .unwrap_err();
    assert_eq!(err.package, "forge");
    assert_eq!(err.file.as_deref(), Some("manifest.ron"));
    assert_eq!(err.field_path, "engine_api");
}
#[test]
fn map_and_visual_ids_stay_separate() {
    let map =
        r#"(id: "turret", size: (x: 10.0, y: 10.0), placements: [], nodes: [], slot_economy: [])"#;
    let visual = r#"(target: Unit("forge:turret"), shape: Cuboid(x: 1.0, y: 1.0, z: 1.0))"#;
    let package = archive(
        UNIT,
        RECIPE,
        TECH,
        "^0.2.1",
        &[("maps/map.ron", map), ("visuals/turret.ron", visual)],
    );
    package.content().unwrap();
    assert_eq!(package.maps().unwrap()[0].1.id, "turret");
    assert_eq!(package.visuals().unwrap().len(), 1);
}

fn legacy_archive(manifest: &str, unit: &str, map: &str) -> PackageArchive {
    let files = BTreeMap::from([
        ("manifest.ron".into(), manifest.into()),
        ("units/turret.ron".into(), unit.into()),
        ("resources/shell.ron".into(), ITEM.into()),
        ("maps/map.ron".into(), map.into()),
    ]);
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}
const LEGACY_MAP: &str =
    r#"(id: "map", size: (x: 10.0, y: 10.0), placements: [], nodes: [], slot_economy: [])"#;
#[test]
fn explicit_none_m3_field_with_caret_0_2_rejected() {
    let err = legacy_archive(
        LEGACY_MANIFEST,
        &LEGACY_UNIT.replace("producer: None)", "producer: None, weapon: None)"),
        LEGACY_MAP,
    )
    .content()
    .unwrap_err();
    assert_eq!(err.package, "forge");
    assert_eq!(err.file.as_deref(), Some("manifest.ron"));
    assert_eq!(err.field_path, "engine_api");
}
#[test]
fn explicit_facing_none_with_caret_0_2_rejected() {
    let map = LEGACY_MAP.replace("placements: []", "placements: [(unit: \"forge:turret\", owner_slot: 0, pos: (x: 1.0, y: 1.0), contents: [], facing: None)]");
    let err = legacy_archive(LEGACY_MANIFEST, LEGACY_UNIT, &map)
        .content()
        .unwrap_err();
    assert_eq!(err.package, "forge");
    assert_eq!(err.file.as_deref(), Some("manifest.ron"));
    assert_eq!(err.field_path, "engine_api");
}
#[test]
fn explicit_empty_m3_exports_with_caret_0_2_rejected() {
    let manifest = LEGACY_MANIFEST.replace("production: []", "production: [], recipes: []");
    let err = legacy_archive(&manifest, LEGACY_UNIT, LEGACY_MAP)
        .content()
        .unwrap_err();
    assert_eq!(err.package, "forge");
    assert_eq!(err.file.as_deref(), Some("manifest.ron"));
    assert_eq!(err.field_path, "engine_api");
}
#[test]
fn m3_words_in_comments_and_strings_do_not_trigger_version_rejection() {
    let unit = format!("// weapon: None\n{LEGACY_UNIT}");
    legacy_archive(LEGACY_MANIFEST, &unit, LEGACY_MAP)
        .content()
        .unwrap();
}
