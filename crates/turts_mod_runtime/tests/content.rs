use turts_mod_api::{ContentId, PackageId, schema::*};
use turts_mod_runtime::content::{
    ContentCatalog, SourceDef, checked_content_ref, validate_content,
};

fn source<T>(file: &str, value: T) -> SourceDef<T> {
    SourceDef {
        file: file.into(),
        value,
    }
}

fn resource(id: &str, kind: ResourceKind) -> SourceDef<ResourceDef> {
    source(
        &format!("resources/{id}.ron"),
        ResourceDef {
            id: id.into(),
            name: id.into(),
            unit_name: id.into(),
            kind,
        },
    )
}

fn valid_worker() -> SourceDef<UnitDef> {
    source(
        "units/worker.ron",
        UnitDef {
            id: "worker".into(),
            name: "Worker".into(),
            footprint: Footprint::Circle { radius: 0.5 },
            movement: Some(MovementDef { speed: 1.0 }),
            inventory: Some(InventoryDef {
                capacity: 2,
                accepts: vec!["cedar:wood".into()],
            }),
            gatherer: Some(GathererDef {
                items: vec!["cedar:wood".into()],
                gather_per_second: 1,
                unload_per_second: 1,
                interaction_range: 0.0,
            }),
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        },
    )
}

fn validate(
    units: Vec<SourceDef<UnitDef>>,
    resources: Vec<SourceDef<ResourceDef>>,
    nodes: Vec<SourceDef<NodeDef>>,
    production: Vec<SourceDef<ProductionDef>>,
) -> Result<ContentCatalog, turts_mod_api::ModError> {
    validate_content(
        &PackageId::parse("cedar").unwrap(),
        resources,
        units,
        nodes,
        production,
    )
}

#[test]
fn cedar_companion_records_parse_against_staged_schema() {
    let manifest = r#"(id: "cedar", version: "0.2.0", kind: Game, engine_api: "^0.2", dependencies: {}, exports: (units: ["units/depot.ron", "units/worker.ron"], maps: ["maps/clearing.ron"], visuals: ["visuals/depot.ron", "visuals/worker.ron", "visuals/grove.ron"], resources: ["resources/wood.ron"], nodes: ["nodes/grove.ron"], production: ["production/hire.ron"]), context: Shared, capabilities: [], license: "LicenseRef-TBD")"#;
    let wood = r#"(id: "wood", name: "Timber", unit_name: "log", kind: Item)"#;
    let depot = r#"(id: "depot", name: "Timber Yard", footprint: Circle(radius: 2.0), movement: None, inventory: Some((capacity: 2000, accepts: ["cedar:wood"])), gatherer: None, drop_off: Some((rules: [(item: "cedar:wood", effect: Store)])), producer: Some((recipes: ["cedar:hire"], queue_capacity: 5, spawn_offsets: [(x: 0.0, y: 4.0), (x: 0.0, y: -4.0)])))"#;
    let worker = r#"(id: "worker", name: "Carrier", footprint: Circle(radius: 0.5), movement: Some((speed: 3.0)), inventory: Some((capacity: 10, accepts: ["cedar:wood"])), gatherer: Some((items: ["cedar:wood"], gather_per_second: 5, unload_per_second: 10, interaction_range: 0.5)), drop_off: None, producer: None)"#;
    let grove = r#"(id: "grove", name: "Timber Stand", item: "cedar:wood", radius: 1.0)"#;
    let hire = r#"(id: "hire", name: "Hire Carrier", output_unit: "cedar:worker", duration_ms: 6000, costs: [(resource: "cedar:wood", amount: 50, source: ProducerInventory)], payment: ReserveThenCharge, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))"#;
    let clearing = r#"(id: "clearing", size: (x: 64.0, y: 64.0), placements: [(unit: "cedar:depot", owner_slot: 0, pos: (x: 8.0, y: 8.0), contents: [(item: "cedar:wood", amount: 100)]), (unit: "cedar:worker", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: []), (unit: "cedar:worker", owner_slot: 0, pos: (x: 12.0, y: 12.0), contents: []), (unit: "cedar:depot", owner_slot: 1, pos: (x: 56.0, y: 8.0), contents: [(item: "cedar:wood", amount: 100)]), (unit: "cedar:worker", owner_slot: 1, pos: (x: 52.0, y: 8.0), contents: []), (unit: "cedar:worker", owner_slot: 1, pos: (x: 52.0, y: 12.0), contents: [])], nodes: [(node: "cedar:grove", pos: (x: 24.0, y: 8.0), remaining: 1000), (node: "cedar:grove", pos: (x: 40.0, y: 8.0), remaining: 1000)], slot_economy: [])"#;
    let visual_depot = r#"(target: Unit("cedar:depot"), shape: Cuboid(x: 3.0, y: 3.0, z: 2.0))"#;
    let visual_worker =
        r#"(target: Unit("cedar:worker"), shape: Capsule(radius: 0.3, height: 1.6))"#;
    let visual_grove = r#"(target: Node("cedar:grove"), shape: Cuboid(x: 1.4, y: 1.4, z: 0.7))"#;
    ron::from_str::<Manifest>(manifest).unwrap();
    let resources = vec![source(
        "resources/wood.ron",
        ron::from_str::<ResourceDef>(wood).unwrap(),
    )];
    let units = vec![
        source("units/depot.ron", ron::from_str::<UnitDef>(depot).unwrap()),
        source(
            "units/worker.ron",
            ron::from_str::<UnitDef>(worker).unwrap(),
        ),
    ];
    let nodes = vec![source(
        "nodes/grove.ron",
        ron::from_str::<NodeDef>(grove).unwrap(),
    )];
    let recipes = vec![source(
        "production/hire.ron",
        ron::from_str::<ProductionDef>(hire).unwrap(),
    )];
    ron::from_str::<MapDef>(clearing).unwrap();
    ron::from_str::<VisualDef>(visual_depot).unwrap();
    ron::from_str::<VisualDef>(visual_worker).unwrap();
    ron::from_str::<VisualDef>(visual_grove).unwrap();
    let catalog = validate_content(
        &PackageId::parse("cedar").unwrap(),
        resources,
        units,
        nodes,
        recipes,
    )
    .unwrap();
    assert_eq!(
        catalog.source_files[&ContentId::parse("cedar:wood").unwrap()],
        "resources/wood.ron"
    );
    assert_eq!(
        catalog.source_files[&ContentId::parse("cedar:worker").unwrap()],
        "units/worker.ron"
    );
    assert_eq!(
        catalog.source_files[&ContentId::parse("cedar:grove").unwrap()],
        "nodes/grove.ron"
    );
    assert_eq!(
        catalog.source_files[&ContentId::parse("cedar:hire").unwrap()],
        "production/hire.ron"
    );
}

#[test]
fn public_validate_content_preserves_m3_unit_capabilities() {
    let turret = source(
        "units/turret.ron",
        UnitDef {
            id: "turret".into(),
            name: "Turret".into(),
            footprint: Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: Some(InventoryDef {
                capacity: 5,
                accepts: vec!["cedar:shell".into()],
            }),
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: Some(InventoryDef {
                capacity: 2,
                accepts: vec!["cedar:shell".into()],
            }),
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: Some(HealthDef { max: 100 }),
            weapon: Some(WeaponDef {
                range: 8.0,
                cooldown_ms: 1000,
                damage: 10,
                ammo: Some(AmmoDef {
                    item: "cedar:shell".into(),
                    per_shot: 1,
                }),
                targets: TargetFilter {
                    slots: SlotRelation::Other,
                },
            }),
            removal: Some(RemovalDef {
                by_command: false,
                contents: ContentsPolicy::Destroy,
            }),
        },
    );
    let catalog = validate_content(
        &PackageId::parse("cedar").unwrap(),
        vec![resource("shell", ResourceKind::Item)],
        vec![turret],
        vec![],
        vec![],
    )
    .unwrap();

    let turret = catalog
        .units
        .get(&ContentId::parse("cedar:turret").unwrap())
        .unwrap();
    assert_eq!(turret.output_inventory.as_ref().unwrap().capacity, 2);
    assert_eq!(
        turret.weapon.as_ref().unwrap().ammo.as_ref().unwrap().item,
        ContentId::parse("cedar:shell").unwrap()
    );
    assert_eq!(turret.health, Some(HealthDef { max: 100 }));
    assert_eq!(
        turret.removal,
        Some(RemovalDef {
            by_command: false,
            contents: ContentsPolicy::Destroy,
        })
    );
}

#[test]
fn references_are_qualified_strings_and_kind_checked() {
    assert!(
        checked_content_ref(
            &PackageId::parse("cedar").unwrap(),
            "units/worker.ron",
            "gatherer.items[0]",
            "cedar:wood"
        )
        .is_ok()
    );
    assert!(
        checked_content_ref(
            &PackageId::parse("cedar").unwrap(),
            "units/worker.ron",
            "gatherer.items[0]",
            "other:wood"
        )
        .is_ok()
    );
    assert!(
        checked_content_ref(
            &PackageId::parse("cedar").unwrap(),
            "units/worker.ron",
            "gatherer.items[0]",
            "bad-id:wood"
        )
        .is_err()
    );
    assert!(
        checked_content_ref(
            &PackageId::parse("cedar").unwrap(),
            "units/worker.ron",
            "gatherer.items[0]",
            "cedar:bad:wood"
        )
        .is_err()
    );
    let mut worker = valid_worker();
    worker.value.gatherer.as_mut().unwrap().items[0] = "cedar:missing".into();
    let bad = validate(
        vec![worker],
        vec![resource("wood", ResourceKind::Item)],
        vec![],
        vec![],
    )
    .unwrap_err();
    assert_eq!(bad.field_path, "gatherer.items[0]");
    assert!(bad.file.as_deref().unwrap().ends_with("worker.ron"));
    let wrong_kind = validate(
        vec![valid_worker()],
        vec![resource("wood", ResourceKind::Balance)],
        vec![],
        vec![],
    )
    .unwrap_err();
    assert_eq!(wrong_kind.field_path, "inventory.accepts[0]");
}

#[test]
fn duplicate_keys_and_cross_kind_ids_reject() {
    let duplicate_manifest_dependencies = r#"(id: "cedar", version: "0.2.0", kind: Game, engine_api: "^0.2", dependencies: {"alpha": "1", "alpha": "2"}, exports: (units: [], maps: [], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "x")"#;
    assert!(ron::from_str::<Manifest>(duplicate_manifest_dependencies).is_err());
    let duplicate_costs = r#"[(resource: "cedar:credits", amount: 1, source: SlotBalance), (resource: "cedar:credits", amount: 2, source: SlotBalance)]"#;
    assert!(ron::from_str::<Vec<Cost>>(duplicate_costs).is_ok());
    let bad = validate(
        vec![source(
            "units/wood.ron",
            UnitDef {
                id: "wood".into(),
                name: "Wood".into(),
                footprint: Footprint::Circle { radius: 1.0 },
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        )],
        vec![resource("wood", ResourceKind::Item)],
        vec![],
        vec![],
    );
    assert!(bad.is_err());
}

#[test]
fn all_definitions_are_validated() {
    let bad_unused = resource("unused", ResourceKind::Item);
    let wood = resource("wood", ResourceKind::Item);
    let worker = valid_worker();
    let result = validate(
        vec![worker],
        vec![wood, bad_unused],
        vec![source(
            "nodes/grove.ron",
            NodeDef {
                id: "grove".into(),
                name: "Grove".into(),
                item: "cedar:missing".into(),
                radius: 1.0,
            },
        )],
        vec![],
    );
    let error = result.unwrap_err();
    assert_eq!(error.field_path, "item");
    assert_eq!(error.file.as_deref(), Some("nodes/grove.ron"));
}

#[test]
fn capability_and_recipe_matrix() {
    let mut worker = valid_worker();
    worker.value.gatherer = Some(GathererDef {
        items: vec![],
        gather_per_second: 1,
        unload_per_second: 1,
        interaction_range: f32::NAN,
    });
    assert!(
        validate(
            vec![worker],
            vec![resource("wood", ResourceKind::Item)],
            vec![],
            vec![]
        )
        .is_err()
    );

    assert!(ron::from_str::<ResourceKind>("Unknown").is_err());
    let spaced = source(
        "resources/wood.ron",
        ResourceDef {
            id: "wood chips".into(),
            name: "Wood".into(),
            unit_name: "chip".into(),
            kind: ResourceKind::Item,
        },
    );
    let static_unit = source(
        "units/worker.ron",
        UnitDef {
            id: "worker".into(),
            name: "Worker".into(),
            footprint: Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        },
    );
    assert!(validate(vec![static_unit.clone()], vec![spaced], vec![], vec![]).is_ok());

    let empty_accepts = source(
        "units/worker.ron",
        UnitDef {
            inventory: Some(InventoryDef {
                capacity: 1,
                accepts: vec![],
            }),
            ..static_unit.value.clone()
        },
    );
    assert!(validate(vec![empty_accepts], vec![], vec![], vec![]).is_ok());

    let bad_radius = source(
        "units/worker.ron",
        UnitDef {
            footprint: Footprint::Circle { radius: f32::NAN },
            ..static_unit.value.clone()
        },
    );
    assert!(validate(vec![bad_radius], vec![], vec![], vec![]).is_err());
    let radius_at_bound = source(
        "units/worker.ron",
        UnitDef {
            footprint: Footprint::Circle { radius: 50.0 },
            ..static_unit.value.clone()
        },
    );
    assert!(validate(vec![radius_at_bound], vec![], vec![], vec![]).is_ok());
    let radius_above_bound = source(
        "units/worker.ron",
        UnitDef {
            footprint: Footprint::Circle { radius: 50.01 },
            ..static_unit.value.clone()
        },
    );
    let error = validate(vec![radius_above_bound], vec![], vec![], vec![]).unwrap_err();
    assert_eq!(error.field_path, "footprint.radius");
    assert_eq!(error.file.as_deref(), Some("units/worker.ron"));

    let speed_at_bound = source(
        "units/worker.ron",
        UnitDef {
            movement: Some(MovementDef { speed: 100.0 }),
            ..static_unit.value.clone()
        },
    );
    assert!(validate(vec![speed_at_bound], vec![], vec![], vec![]).is_ok());
    let speed_above_bound = source(
        "units/worker.ron",
        UnitDef {
            movement: Some(MovementDef { speed: 100.01 }),
            ..static_unit.value.clone()
        },
    );
    let error = validate(vec![speed_above_bound], vec![], vec![], vec![]).unwrap_err();
    assert_eq!(error.field_path, "movement.speed");
    assert_eq!(error.file.as_deref(), Some("units/worker.ron"));
    let bad_node = source(
        "nodes/tree.ron",
        NodeDef {
            id: "tree".into(),
            name: "Tree".into(),
            item: "cedar:wood".into(),
            radius: 0.0,
        },
    );
    assert!(
        validate(
            vec![],
            vec![resource("wood", ResourceKind::Item)],
            vec![bad_node],
            vec![]
        )
        .is_err()
    );

    let mut mobile_producer = valid_worker();
    mobile_producer.value.producer = Some(ProducerDef {
        recipes: vec!["cedar:hire".into()],
        queue_capacity: 1,
        spawn_offsets: vec![turts_mod_api::schema::WorldPos { x: 0.0, y: 1.0 }],
    });
    let error = validate(
        vec![mobile_producer],
        vec![resource("wood", ResourceKind::Item)],
        vec![],
        vec![],
    )
    .unwrap_err();
    assert_eq!(error.field_path, "producer");
    assert_eq!(error.file.as_deref(), Some("units/worker.ron"));

    let producer = UnitDef {
        id: "yard".into(),
        name: "Yard".into(),
        footprint: Footprint::Circle { radius: 1.0 },
        movement: None,
        inventory: Some(InventoryDef {
            capacity: 20,
            accepts: vec!["cedar:wood".into(), "cedar:stone".into()],
        }),
        gatherer: None,
        drop_off: None,
        producer: Some(ProducerDef {
            recipes: vec!["cedar:hire".into()],
            queue_capacity: 1,
            spawn_offsets: vec![turts_mod_api::schema::WorldPos { x: 0.0, y: 1.0 }],
        }),
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: None,
        weapon: None,
        removal: None,
    };
    let worker = UnitDef {
        movement: Some(MovementDef { speed: 1.0 }),
        inventory: None,
        ..static_unit.value.clone()
    };
    let make_recipe = |costs| ProductionDef {
        id: "hire".into(),
        name: "Hire".into(),
        output_unit: "cedar:worker".into(),
        duration_ms: 1,
        costs,
        payment: PaymentPolicy::ReserveThenCharge,
        cancel_refunds: CancelRefunds {
            queued: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            running: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            output_blocked: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
        },
    };
    let good_costs = vec![
        Cost {
            resource: "cedar:wood".into(),
            amount: 1,
            source: CostSource::ProducerInventory,
        },
        Cost {
            resource: "cedar:stone".into(),
            amount: 1,
            source: CostSource::ProducerInventory,
        },
    ];
    assert!(
        validate(
            vec![
                source("units/yard.ron", producer.clone()),
                source("units/worker.ron", worker.clone())
            ],
            vec![
                resource("wood", ResourceKind::Item),
                resource("stone", ResourceKind::Item)
            ],
            vec![],
            vec![source("production/hire.ron", make_recipe(good_costs))]
        )
        .is_ok()
    );
    let duplicate_costs = vec![
        Cost {
            resource: "cedar:wood".into(),
            amount: 1,
            source: CostSource::ProducerInventory,
        },
        Cost {
            resource: "cedar:wood".into(),
            amount: 2,
            source: CostSource::ProducerInventory,
        },
    ];
    assert!(
        validate(
            vec![
                source("units/yard.ron", producer.clone()),
                source("units/worker.ron", worker.clone())
            ],
            vec![resource("wood", ResourceKind::Item)],
            vec![],
            vec![source("production/hire.ron", make_recipe(duplicate_costs))]
        )
        .is_err()
    );
    let static_worker = UnitDef {
        id: "worker".into(),
        name: "Worker".into(),
        footprint: Footprint::Circle { radius: 1.0 },
        movement: None,
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: None,
        weapon: None,
        removal: None,
    };
    let error = validate(
        vec![source("units/worker.ron", static_worker)],
        vec![],
        vec![],
        vec![source("production/hire.ron", make_recipe(vec![]))],
    )
    .unwrap_err();
    assert_eq!(error.field_path, "output_unit");
    assert_eq!(error.file.as_deref(), Some("production/hire.ron"));
    let bad_refunds = make_recipe(vec![Cost {
        resource: "cedar:wood".into(),
        amount: 1,
        source: CostSource::ProducerInventory,
    }]);
    let mut bad_refunds = bad_refunds;
    bad_refunds.cancel_refunds.queued.numerator = 0;
    assert!(
        validate(
            vec![
                source("units/yard.ron", producer),
                source("units/worker.ron", worker)
            ],
            vec![resource("wood", ResourceKind::Item)],
            vec![],
            vec![source("production/hire.ron", bad_refunds)]
        )
        .is_err()
    );
}

#[test]
fn late_producer_cost_errors_keep_definition_file_and_both_indexes() {
    let yard = UnitDef {
        id: "yard".into(),
        name: "Yard".into(),
        footprint: Footprint::Circle { radius: 1.0 },
        movement: None,
        inventory: Some(InventoryDef {
            capacity: 10,
            accepts: vec!["cedar:wood".into()],
        }),
        gatherer: None,
        drop_off: None,
        producer: Some(ProducerDef {
            recipes: vec!["cedar:free".into(), "cedar:hire".into()],
            queue_capacity: 2,
            spawn_offsets: vec![turts_mod_api::schema::WorldPos { x: 0.0, y: 1.0 }],
        }),
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: None,
        weapon: None,
        removal: None,
    };
    let worker = UnitDef {
        id: "worker".into(),
        name: "Worker".into(),
        footprint: Footprint::Circle { radius: 1.0 },
        movement: Some(MovementDef { speed: 1.0 }),
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: None,
        weapon: None,
        removal: None,
    };
    let no_costs = |id: &str| ProductionDef {
        id: id.into(),
        name: id.into(),
        output_unit: "cedar:worker".into(),
        duration_ms: 1,
        costs: vec![],
        payment: PaymentPolicy::ChargeOnQueue,
        cancel_refunds: CancelRefunds {
            queued: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            running: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            output_blocked: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
        },
    };
    let mut bad_cost_recipe = no_costs("hire");
    bad_cost_recipe.costs.push(Cost {
        resource: "cedar:stone".into(),
        amount: 1,
        source: CostSource::ProducerInventory,
    });
    let error = validate(
        vec![
            source("units/yard-real.ron", yard.clone()),
            source("units/worker-real.ron", worker.clone()),
        ],
        vec![
            resource("wood", ResourceKind::Item),
            resource("stone", ResourceKind::Item),
        ],
        vec![],
        vec![
            source("production/free-real.ron", no_costs("free")),
            source("production/hire-real.ron", bad_cost_recipe.clone()),
        ],
    )
    .unwrap_err();
    assert_eq!(error.field_path, "producer.recipes[1].costs[0]");
    assert_eq!(error.file.as_deref(), Some("units/yard-real.ron"));

    let mut no_inventory_yard = yard;
    no_inventory_yard.inventory = None;
    let error = validate(
        vec![
            source("units/yard-real.ron", no_inventory_yard),
            source("units/worker-real.ron", worker),
        ],
        vec![
            resource("wood", ResourceKind::Item),
            resource("stone", ResourceKind::Item),
        ],
        vec![],
        vec![
            source("production/free-real.ron", no_costs("free")),
            source("production/hire-real.ron", bad_cost_recipe),
        ],
    )
    .unwrap_err();
    assert_eq!(error.field_path, "producer.recipes[1].costs[0]");
    assert_eq!(error.file.as_deref(), Some("units/yard-real.ron"));
}
