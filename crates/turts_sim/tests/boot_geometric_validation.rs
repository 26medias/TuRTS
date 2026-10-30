use std::collections::BTreeMap;

use turts_mod_api::{
    ContentCatalog, ContentId, ExtractorDef, Footprint, InserterDef, MapDef, MapSize,
    NodePlacement, Placement, ResolvedInventory, ResolvedNode, ResolvedUnit, ResolvedWeapon,
    ResourceDef, ResourceKind, SlotRelation, SourceDef, TargetFilter, WorldPos,
};
use turts_sim::{DEFAULT_TICK_HZ, gameplay_entity_count, headless_app, stage_package};

#[derive(Clone, Copy)]
enum GeometricField {
    Extractor,
    Inserter,
    Weapon,
}

impl GeometricField {
    fn name(self) -> &'static str {
        match self {
            Self::Extractor => "extractor.interaction_range",
            Self::Inserter => "inserter.reach",
            Self::Weapon => "weapon.range",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Extractor => "probe:drill",
            Self::Inserter => "probe:arm",
            Self::Weapon => "probe:turret",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::Extractor => "units/drill.ron",
            Self::Inserter => "units/arm.ron",
            Self::Weapon => "units/turret.ron",
        }
    }
}

fn plain_unit(id: &str) -> ResolvedUnit {
    ResolvedUnit {
        id: ContentId::parse(id).unwrap(),
        name: "Support".into(),
        footprint: Footprint::Circle { radius: 0.25 },
        movement: None,
        inventory: Some(ResolvedInventory {
            capacity: 4,
            accepts: vec![],
        }),
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
    }
}

fn catalog(field: GeometricField, value: f32) -> ContentCatalog {
    let id = ContentId::parse(field.id()).unwrap();
    let ore = ContentId::parse("probe:ore").unwrap();
    let unit = ResolvedUnit {
        id: id.clone(),
        name: "Probe unit".into(),
        footprint: Footprint::Circle { radius: 0.25 },
        movement: None,
        inventory: Some(ResolvedInventory {
            capacity: 4,
            accepts: vec![ore.clone()],
        }),
        gatherer: None,
        drop_off: None,
        producer: None,
        output_inventory: None,
        extractor: matches!(field, GeometricField::Extractor).then_some(ExtractorDef {
            per_second: 1,
            interaction_range: value,
        }),
        conveyor: None,
        inserter: matches!(field, GeometricField::Inserter).then_some(InserterDef {
            reach: value,
            swing_ms: 250,
            return_ms: 250,
        }),
        machine: None,
        research: None,
        health: None,
        weapon: matches!(field, GeometricField::Weapon).then_some(ResolvedWeapon {
            range: value,
            cooldown_ms: 250,
            damage: 1,
            ammo: None,
            targets: TargetFilter {
                slots: SlotRelation::Other,
            },
        }),
        removal: None,
    };
    let mut units = BTreeMap::from([(id.clone(), unit)]);
    if matches!(field, GeometricField::Inserter) {
        units.insert(
            ContentId::parse("probe:source").unwrap(),
            plain_unit("probe:source"),
        );
        units.insert(
            ContentId::parse("probe:destination").unwrap(),
            plain_unit("probe:destination"),
        );
    }
    ContentCatalog {
        resources: BTreeMap::from([(
            ore.clone(),
            ResourceDef {
                id: "ore".into(),
                name: "Ore".into(),
                unit_name: "ore".into(),
                kind: ResourceKind::Item,
            },
        )]),
        units,
        nodes: BTreeMap::from([(
            ContentId::parse("probe:deposit").unwrap(),
            ResolvedNode {
                id: ContentId::parse("probe:deposit").unwrap(),
                name: "Deposit".into(),
                item: ore,
                radius: 1.0,
            },
        )]),
        production: BTreeMap::new(),
        recipes: BTreeMap::new(),
        technologies: BTreeMap::new(),
        orders: BTreeMap::new(),
        source_files: BTreeMap::from([(id, field.source().into())]),
    }
}

fn map(field: GeometricField, placed: bool) -> MapDef {
    MapDef {
        id: "field".into(),
        size: MapSize { x: 24.0, y: 24.0 },
        placements: if placed {
            let mut placements = vec![Placement {
                unit: field.id().into(),
                owner_slot: 0,
                pos: WorldPos { x: 8.0, y: 8.0 },
                contents: vec![],
                facing: matches!(field, GeometricField::Inserter)
                    .then_some(turts_mod_api::Facing::PosX),
            }];
            if matches!(field, GeometricField::Inserter) {
                placements.extend([
                    Placement {
                        unit: "probe:source".into(),
                        owner_slot: 0,
                        pos: WorldPos { x: 7.0, y: 8.0 },
                        contents: vec![],
                        facing: None,
                    },
                    Placement {
                        unit: "probe:destination".into(),
                        owner_slot: 0,
                        pos: WorldPos { x: 9.0, y: 8.0 },
                        contents: vec![],
                        facing: None,
                    },
                ]);
            }
            placements
        } else {
            vec![]
        },
        nodes: if placed && matches!(field, GeometricField::Extractor) {
            vec![NodePlacement {
                node: "probe:deposit".into(),
                pos: WorldPos { x: 9.5, y: 8.0 },
                remaining: 8,
            }]
        } else {
            vec![]
        },
        slot_economy: vec![],
        belts: vec![],
    }
}

fn invalid_values() -> Vec<f32> {
    let mut values = vec![
        -1.0,
        f32::from_bits(0x8000_0001),
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut state = 0x4d33_7a91_u32;
    for _ in 0..16 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        values.push(f32::from_bits(0x8000_0001 | (state & 0x7f7f_fffe)));
    }
    values
}

#[test]
fn direct_resolved_catalog_boot_rejects_invalid_geometric_scalars_before_binding_or_install() {
    let fields = [
        GeometricField::Extractor,
        GeometricField::Inserter,
        GeometricField::Weapon,
    ];
    for field in fields {
        for value in invalid_values() {
            for placed in [false, true] {
                let mut app = headless_app(DEFAULT_TICK_HZ);
                let world = app.world_mut();
                let before_entities = gameplay_entity_count(world);
                let baseline = stage_package(
                    world,
                    catalog(field, 1.0),
                    SourceDef {
                        file: "maps/field.ron".into(),
                        value: map(field, placed),
                    },
                    DEFAULT_TICK_HZ,
                )
                .unwrap_or_else(|err| {
                    panic!(
                        "valid {} baseline at placement={placed} failed: {err:?}",
                        field.name()
                    )
                });
                drop(baseline);
                let err = stage_package(
                    world,
                    catalog(field, value),
                    SourceDef {
                        file: "maps/field.ron".into(),
                        value: map(field, placed),
                    },
                    DEFAULT_TICK_HZ,
                )
                .expect_err("invalid resolved scalar must be rejected before geometric binding");
                assert_eq!(err.package, "probe");
                assert_eq!(err.file.as_deref(), Some(field.source()), "{err:?}");
                assert_eq!(err.field_path, field.name(), "{err:?}");
                assert!(
                    err.message.contains("finite") && err.message.contains(">= 0"),
                    "{err:?}"
                );
                assert_eq!(gameplay_entity_count(world), before_entities);
                assert!(world.get_resource::<turts_sim::MapBounds>().is_none());
                assert!(world.resource::<turts_sim::UnitTypes>().is_empty());
            }
        }
    }
}

#[test]
fn direct_resolved_catalog_boot_accepts_zero_and_finite_boundaries_when_unplaced() {
    for field in [
        GeometricField::Extractor,
        GeometricField::Inserter,
        GeometricField::Weapon,
    ] {
        for value in [0.0, -0.0, f32::from_bits(1), 1.0, f32::MAX] {
            let app = headless_app(DEFAULT_TICK_HZ);
            stage_package(
                app.world(),
                catalog(field, value),
                SourceDef {
                    file: "maps/field.ron".into(),
                    value: map(field, false),
                },
                DEFAULT_TICK_HZ,
            )
            .unwrap_or_else(|err| panic!("valid {}={value:?} rejected: {err:?}", field.name()));
        }
    }
}
