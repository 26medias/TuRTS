//! Whole-package checks for damage contracts. Fixtures use the existing Cedar package.
use std::collections::BTreeMap;
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};
mod common;

struct Fixture {
    files: BTreeMap<String, Vec<u8>>,
    manifest: Manifest,
    unit: UnitDef,
    combat: CombatDef,
    upgrade: Option<ArmourStat>,
}
impl Fixture {
    fn new() -> Self {
        let files = common::read_files(&common::package_dir("cedar"));
        let mut manifest: Manifest = ron::de::from_bytes(&files["manifest.ron"]).unwrap();
        manifest.engine_api = "^0.2.12".into();
        manifest.exports.combat = vec!["combat.ron".into()];
        manifest.exports.orders.clear();
        manifest.exports.interface.clear();
        manifest.exports.input.clear();
        let mut unit: UnitDef = ron::de::from_bytes(&files["units/worker.ron"]).unwrap();
        unit.gatherer = None;
        unit.health = Some(HealthDef { max: 100 });
        unit.removal = Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy,
        });
        unit.armour = Some(ArmourDef {
            class: "cedar:heavy".into(),
            flat: 2,
            points: 0,
            tags: vec![],
            flat_by_type: vec![],
        });
        unit.weapon = Some(WeaponDef {
            range: 6.,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 700,
            damage: 15,
            ammo: None,
            targets: TargetFilter {
                slots: SlotRelation::Enemy,
            },
            damage_type: Some("cedar:pierce".into()),
            bonus: vec![],
            when_short: vec![],
        });
        Self {
            files,
            manifest,
            unit,
            combat: CombatDef {
                damage_types: vec!["pierce".into(), "blade".into()],
                armour_classes: vec!["light".into(), "heavy".into(), "structure".into()],
                tags: vec!["building".into()],
                curve: ArmourCurve::None,
                cells: vec![],
                minimum: 1,
            },
            upgrade: None,
        }
    }
    fn files(mut self) -> BTreeMap<String, Vec<u8>> {
        if let Some(stat) = self.upgrade {
            self.manifest.exports.technologies = vec!["upgrade.ron".into()];
            let mut lab = self.unit.clone();
            lab.id = "laboratory".into();
            lab.name = "Laboratory".into();
            lab.weapon = None;
            lab.armour = None;
            lab.health = None;
            lab.removal = None;
            lab.movement = None;
            lab.research = Some(ResearchStationDef {
                technologies: vec!["cedar:upgrade".into()],
                queue_capacity: 1,
            });
            self.manifest
                .exports
                .units
                .push("units/laboratory.ron".into());
            self.manifest
                .exports
                .visuals
                .push("visuals/laboratory.ron".into());
            let visual = String::from_utf8(self.files["visuals/worker.ron"].clone())
                .unwrap()
                .replace("cedar:worker", "cedar:laboratory");
            self.files
                .insert("visuals/laboratory.ron".into(), visual.into_bytes());
            self.files.insert(
                "units/laboratory.ron".into(),
                ron::ser::to_string(&lab).unwrap().into_bytes(),
            );
            let upgrade = TechnologyDef {
                id: "upgrade".into(),
                name: "Armour upgrade".into(),
                prerequisites: vec![],
                duration_ms: 100,
                costs: vec![Cost {
                    resource: "cedar:wood".into(),
                    amount: 1,
                    source: CostSource::ProducerInventory,
                }],
                payment: PaymentPolicy::ReserveThenCharge,
                cancel_refunds: CancelRefunds {
                    queued: Ratio::ONE,
                    running: Ratio::ZERO,
                    output_blocked: Ratio::ONE,
                    destroyed: Ratio::ZERO,
                },
                effects: vec![EffectDef::ArmourModifier {
                    unit: "cedar:worker".into(),
                    stat,
                    add: 1,
                }],
            };
            self.files.insert(
                "upgrade.ron".into(),
                ron::ser::to_string(&upgrade).unwrap().into_bytes(),
            );
        }
        self.files.insert(
            "manifest.ron".into(),
            ron::ser::to_string(&self.manifest).unwrap().into_bytes(),
        );
        self.files.insert(
            "units/worker.ron".into(),
            ron::ser::to_string(&self.unit).unwrap().into_bytes(),
        );
        self.files.insert(
            "combat.ron".into(),
            ron::ser::to_string(&self.combat).unwrap().into_bytes(),
        );
        self.files
    }
    fn load(self) -> Result<ContentCatalog, ModError> {
        load(self.files())
    }
}
fn load(files: BTreeMap<String, Vec<u8>>) -> Result<ContentCatalog, ModError> {
    PackageArchive::parse(&pack_files(&files).unwrap())
        .unwrap()
        .content()
}
fn ratio(numerator: u32, denominator: u32) -> Ratio {
    Ratio {
        numerator,
        denominator,
    }
}

#[test]
fn damage_four_game_styles_and_percent_curve_pack() {
    for style in [
        "command_and_conquer",
        "starcraft",
        "age_of_empires",
        "warcraft",
        "percent",
    ] {
        let mut f = Fixture::new();
        f.combat.cells = vec![DamageCell {
            damage: "cedar:pierce".into(),
            armour: "cedar:heavy".into(),
            ratio: ratio(1, 2),
        }];
        match style {
            "command_and_conquer" => f.unit.armour.as_mut().unwrap().flat = 0,
            "starcraft" => f.upgrade = Some(ArmourStat::Flat),
            "age_of_empires" => {
                f.unit.armour.as_mut().unwrap().flat_by_type =
                    vec![("cedar:pierce".into(), 1), ("cedar:blade".into(), 3)];
                f.unit.armour.as_mut().unwrap().tags = vec!["cedar:building".into()];
                f.unit.weapon.as_mut().unwrap().bonus = vec![DamageBonus {
                    against: "cedar:building".into(),
                    add: 4,
                }];
                f.upgrade = Some(ArmourStat::FlatByType("cedar:pierce".into()));
            }
            "warcraft" => {
                f.combat.curve = ArmourCurve::Ratio { k: ratio(50, 3) };
                f.unit.armour.as_mut().unwrap().points = 5;
                f.upgrade = Some(ArmourStat::Points);
            }
            "percent" => {
                f.combat.curve = ArmourCurve::Percent {
                    per_point: ratio(1, 20),
                    cap: ratio(3, 4),
                };
                f.unit.armour.as_mut().unwrap().points = 5;
            }
            _ => unreachable!(),
        }
        let files = f.files();
        if let Ok(root) = std::env::var("TURTS_DAMAGE_FIXTURE_DIR") {
            for (name, bytes) in &files {
                let path = std::path::Path::new(&root).join(style).join(name);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, bytes).unwrap();
            }
        }
        let catalog = load(files).unwrap_or_else(|error| panic!("{style}: {error}"));
        assert!(catalog.combat.is_some());
        assert!(
            catalog.units[&ContentId::parse("cedar:worker").unwrap()]
                .armour
                .is_some()
        );
    }
}

#[test]
fn damage_sparse_table_unarmoured_targets_and_untyped_weapons_pack() {
    let mut f = Fixture::new();
    f.unit.armour = None;
    f.unit.weapon.as_mut().unwrap().damage_type = None;
    f.combat.damage_types.clear();
    f.combat.armour_classes.clear();
    f.combat.tags.clear();
    let catalog = f.load().unwrap();
    assert!(catalog.combat.unwrap().cells.is_empty());
}

#[test]
fn damage_refusals_name_the_source_and_fix_or_nearest_name() {
    for case in [
        "points_without_curve",
        "unknown_type",
        "unknown_class",
        "unknown_tag",
        "unknown_bonus",
        "unknown_flat_type",
        "zero_k",
        "zero_k_den",
        "cap_one",
        "cap_above_one",
        "per_point_den_zero",
        "points_modifier_without_curve",
    ] {
        let mut f = Fixture::new();
        let expected = match case {
            "points_without_curve" => {
                f.unit.armour.as_mut().unwrap().points = 5;
                vec![
                    "units/worker.ron",
                    "armour.points",
                    "no curve",
                    "Add curve: Ratio",
                    "armour.flat",
                ]
            }
            "unknown_type" => {
                f.unit.weapon.as_mut().unwrap().damage_type = Some("cedar:pirce".into());
                vec![
                    "units/worker.ron",
                    "weapon.damage_type",
                    "did you mean `cedar:pierce`",
                ]
            }
            "unknown_class" => {
                f.unit.armour.as_mut().unwrap().class = "cedar:hevy".into();
                vec![
                    "units/worker.ron",
                    "armour.class",
                    "did you mean `cedar:heavy`",
                ]
            }
            "unknown_tag" => {
                f.unit.armour.as_mut().unwrap().tags = vec!["cedar:buildng".into()];
                vec![
                    "units/worker.ron",
                    "armour.tags",
                    "did you mean `cedar:building`",
                ]
            }
            "unknown_bonus" => {
                f.unit.weapon.as_mut().unwrap().bonus = vec![DamageBonus {
                    against: "cedar:strcture".into(),
                    add: 3,
                }];
                vec![
                    "units/worker.ron",
                    "weapon.bonus",
                    "did you mean `cedar:structure`",
                ]
            }
            "unknown_flat_type" => {
                f.unit.armour.as_mut().unwrap().flat_by_type = vec![("cedar:pirce".into(), 2)];
                vec![
                    "units/worker.ron",
                    "armour.flat_by_type",
                    "did you mean `cedar:pierce`",
                ]
            }
            "zero_k" => {
                f.combat.curve = ArmourCurve::Ratio { k: ratio(0, 1) };
                vec![
                    "combat.ron",
                    "curve.k",
                    "positive ratio",
                    "use curve: Ratio",
                ]
            }
            "zero_k_den" => {
                f.combat.curve = ArmourCurve::Ratio { k: ratio(1, 0) };
                vec!["combat.ron", "curve.k", "denominator above 0"]
            }
            "cap_one" => {
                f.combat.curve = ArmourCurve::Percent {
                    per_point: ratio(1, 20),
                    cap: ratio(1, 1),
                };
                vec!["combat.ron", "curve", "cap below 1", "use cap:"]
            }
            "cap_above_one" => {
                f.combat.curve = ArmourCurve::Percent {
                    per_point: ratio(1, 20),
                    cap: ratio(2, 1),
                };
                vec!["combat.ron", "curve", "cap below 1"]
            }
            "per_point_den_zero" => {
                f.combat.curve = ArmourCurve::Percent {
                    per_point: ratio(1, 0),
                    cap: ratio(3, 4),
                };
                vec!["combat.ron", "curve", "denominators above 0"]
            }
            "points_modifier_without_curve" => {
                f.upgrade = Some(ArmourStat::Points);
                vec![
                    "upgrade.ron",
                    "stat",
                    "Points needs a combat curve",
                    "Add curve: Ratio",
                ]
            }
            _ => unreachable!(),
        };
        let error = f.load().unwrap_err().to_string();
        for text in expected {
            assert!(error.contains(text), "{case}: missing {text} in {error}");
        }
    }
}

#[test]
fn damage_percent_missing_cap_has_an_actionable_refusal() {
    let mut f = Fixture::new();
    f.combat.curve = ArmourCurve::Percent {
        per_point: ratio(1, 20),
        cap: ratio(3, 4),
    };
    let mut files = f.files();
    let text = String::from_utf8(files["combat.ron"].clone()).unwrap();
    let remove = ",cap:(numerator:3,denominator:4)";
    assert_eq!(text.matches(remove).count(), 1, "{text}");
    files.insert("combat.ron".into(), text.replace(remove, "").into_bytes());
    let error = load(files).unwrap_err().to_string();
    assert!(
        error.contains("cap below 1") && error.contains("use cap:"),
        "{error}"
    );
}
