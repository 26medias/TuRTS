use std::{
    fs,
    path::{Path, PathBuf},
};

use turts_mod_api::schema::{
    AmmoDef, BeltLinePlacement, ContentsPolicy, ConveyorDef, Exports, ExtractorDef, Facing,
    Footprint, HealthDef, InserterDef, InventoryDef, MachineDef, MapDef, MapSize, Placement,
    RemovalDef, ResearchStationDef, SlotRelation, TargetFilter, UnitDef, WeaponDef, WorldPos,
};

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

#[test]
fn old_packages_omit_m3_defaults_when_serialized_and_nonempty_fields_remain() {
    for package in ["base", "haul", "acme", "cedar"] {
        visit_package_ron_serialization(&workspace_root().join("mods").join(package));
    }

    let exports = serde_json::to_value(Exports {
        units: vec![],
        maps: vec![],
        visuals: vec![],
        resources: vec![],
        nodes: vec![],
        production: vec![],
        recipes: vec!["recipes/shells.ron".into()],
        technologies: vec!["technologies/hardened.ron".into()],
        orders: vec![],
        interface: vec![],
        input: vec![],
        assets: vec![],
    })
    .unwrap();
    assert!(exports["recipes"].is_array());
    assert!(exports["technologies"].is_array());

    let unit = serde_json::to_value(m3_unit()).unwrap();
    for field in [
        "output_inventory",
        "extractor",
        "conveyor",
        "inserter",
        "machine",
        "research",
        "health",
        "weapon",
        "removal",
    ] {
        assert!(
            unit.get(field).is_some(),
            "nonempty M3 field {field} must serialize"
        );
    }
    let map = serde_json::to_value(m3_map()).unwrap();
    assert!(map["belts"].is_array());
    assert!(map["placements"][0]["facing"].is_string());
}

fn visit_package_ron_serialization(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit_package_ron_serialization(&path);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "ron") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let relative = path.strip_prefix(workspace_root()).unwrap();
        let kind = if relative
            .file_name()
            .is_some_and(|name| name == "manifest.ron")
        {
            "manifest".to_owned()
        } else {
            relative
                .components()
                .nth(2)
                .unwrap()
                .as_os_str()
                .to_string_lossy()
                .into_owned()
        };
        match kind.as_str() {
            "manifest" => {
                let value: turts_mod_api::schema::Manifest = ron::from_str(&text).unwrap();
                let json = serde_json::to_value(value).unwrap();
                for field in ["recipes", "technologies"] {
                    assert!(
                        json["exports"].get(field).is_none(),
                        "{} serialized default export {field}",
                        relative.display()
                    );
                }
            }
            "units" => {
                let value: UnitDef = ron::from_str(&text).unwrap();
                let json = serde_json::to_value(value).unwrap();
                for field in [
                    "output_inventory",
                    "extractor",
                    "conveyor",
                    "inserter",
                    "machine",
                    "research",
                    "health",
                    "weapon",
                    "removal",
                ] {
                    assert!(
                        json.get(field).is_none(),
                        "{} serialized default unit field {field}",
                        relative.display()
                    );
                }
            }
            "maps" => {
                let value: MapDef = ron::from_str(&text).unwrap();
                let json = serde_json::to_value(value).unwrap();
                assert!(
                    json.get("belts").is_none(),
                    "{} serialized default map belts",
                    relative.display()
                );
                for placement in json["placements"].as_array().unwrap() {
                    assert!(
                        placement.get("facing").is_none(),
                        "{} serialized default placement facing",
                        relative.display()
                    );
                }
            }
            _ => {}
        }
    }
}

fn m3_unit() -> UnitDef {
    UnitDef {
        id: "turret".into(),
        name: "Turret".into(),
        footprint: Footprint::Circle { radius: 1.0 },
        movement: None,
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
        output_inventory: Some(InventoryDef {
            capacity: 1,
            accepts: vec!["forge:shell".into()],
        }),
        extractor: Some(ExtractorDef {
            per_second: 1,
            interaction_range: 0.5,
        }),
        conveyor: Some(ConveyorDef {
            slots_per_piece: 1,
            slots_per_second: 1,
            accepts: vec!["forge:shell".into()],
        }),
        inserter: Some(InserterDef {
            reach: 1.0,
            swing_ms: 250,
            return_ms: 250,
        }),
        machine: Some(MachineDef {
            recipe: "forge:shells".into(),
        }),
        research: Some(ResearchStationDef {
            technologies: vec!["forge:hardened".into()],
            queue_capacity: 1,
        }),
        health: Some(HealthDef { max: 100 }),
        weapon: Some(WeaponDef {
            range: 8.0,
            cooldown_ms: 1000,
            damage: 10,
            ammo: Some(AmmoDef {
                item: "forge:shell".into(),
                per_shot: 1,
            }),
            targets: TargetFilter {
                slots: SlotRelation::Other,
            },
        }),
        removal: Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy,
        }),
    }
}

fn m3_map() -> MapDef {
    MapDef {
        id: "proving_ground".into(),
        size: MapSize { x: 32.0, y: 16.0 },
        placements: vec![Placement {
            unit: "forge:turret".into(),
            owner_slot: 0,
            pos: WorldPos { x: 8.0, y: 8.0 },
            contents: vec![],
            facing: Some(Facing::PosX),
        }],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![BeltLinePlacement {
            conveyor: "forge:turret".into(),
            owner_slot: 0,
            start: WorldPos { x: 4.0, y: 8.0 },
            facing: Facing::PosX,
            pieces: 1,
        }],
    }
}
