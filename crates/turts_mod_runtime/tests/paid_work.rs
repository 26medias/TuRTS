//! Public package acceptance for API14 paid work and repair.
mod common;
use std::collections::BTreeMap;
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};
fn encode<T: serde::Serialize>(value: &T) -> Vec<u8> {
    ron::ser::to_string(value).unwrap().into_bytes()
}
struct Fixture {
    files: BTreeMap<String, Vec<u8>>,
    manifest: Manifest,
    worker: UnitDef,
    hire: ProductionDef,
}
impl Fixture {
    fn new() -> Self {
        let mut files = common::read_files(&common::package_dir("cedar"));
        let mut manifest: Manifest = ron::de::from_bytes(&files["manifest.ron"]).unwrap();
        manifest.engine_api = "^0.2.14".into();
        manifest.exports.interface.clear();
        manifest.exports.input.clear();
        manifest.exports.orders.clear();
        manifest
            .exports
            .resources
            .push("resources/credits.ron".into());
        files.insert(
            "resources/credits.ron".into(),
            encode(&ResourceDef {
                id: "credits".into(),
                name: "Credits".into(),
                unit_name: String::new(),
                kind: ResourceKind::Balance,
            }),
        );
        manifest.exports.combat.push("combat.ron".into());
        files.insert(
            "combat.ron".into(),
            encode(&CombatDef {
                damage_types: vec!["normal".into()],
                armour_classes: vec!["heavy".into()],
                tags: vec![],
                curve: ArmourCurve::None,
                cells: vec![],
                minimum: 0,
            }),
        );
        let mut worker: UnitDef = ron::de::from_bytes(&files["units/worker.ron"]).unwrap();
        worker.gatherer = None;
        worker.repairer = Some(RepairerDef {
            rate_per_second: 1,
            reach: 0.5,
            cost: vec![RepairCost {
                resource: "cedar:credits".into(),
                amount: 1,
                per_health: 3,
            }],
            targets: TargetFilter {
                slots: SlotRelation::Same,
            },
            classes: vec!["cedar:heavy".into()],
            auto_range: Some(3.),
        });
        let mut hire: ProductionDef = ron::de::from_bytes(&files["production/hire.ron"]).unwrap();
        hire.payment = PaymentPolicy::ChargeOverTime;
        hire.cancel_refunds.destroyed = RefundRatio::ONE;
        Self {
            files,
            manifest,
            worker,
            hire,
        }
    }
    fn archive(mut self) -> PackageArchive {
        self.files
            .insert("manifest.ron".into(), encode(&self.manifest));
        self.files
            .insert("units/worker.ron".into(), encode(&self.worker));
        self.files
            .insert("production/hire.ron".into(), encode(&self.hire));
        PackageArchive::parse(&pack_files(&self.files).unwrap()).unwrap()
    }
    fn fail(self, path: &str) {
        let error = self.archive().content().unwrap_err();
        assert!(error.to_string().contains(path), "expected {path}: {error}");
    }
}
#[test]
fn public_repair_and_incremental_production_reach_the_resolved_catalog() {
    for payment in [PaymentPolicy::ChargeOnQueue, PaymentPolicy::ChargeOverTime] {
        let mut f = Fixture::new();
        let mut depot: UnitDef = ron::de::from_bytes(&f.files["units/depot.ron"]).unwrap();
        let mut relay = depot.clone();
        relay.id = "relay".into();
        relay.name = "Relay".into();
        relay.producer = None;
        relay.drop_off = None;
        relay.health = Some(HealthDef { max: 100 });
        relay.removal = None;
        relay.placement = Some(SitePlacementDef {
            gap: 2.,
            node_clearance: 0.,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 4,
            },
        });
        depot.constructor = Some(ConstructorDef {
            buildings: vec!["cedar:build_relay".into()],
            concurrent_sites: 2,
            queue_capacity: 2,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
        let mut build = f.hire.clone();
        build.id = "build_relay".into();
        build.output_unit = "cedar:relay".into();
        build.payment = payment;
        f.manifest.exports.units.push("units/relay.ron".into());
        f.manifest
            .exports
            .production
            .push("production/build_relay.ron".into());
        f.files.insert("units/depot.ron".into(), encode(&depot));
        f.files.insert("units/relay.ron".into(), encode(&relay));
        f.files
            .insert("production/build_relay.ron".into(), encode(&build));
        f.manifest.exports.match_files.push("match.ron".into());
        f.files.insert(
            "match.ron".into(),
            b"(start:Immediate,victory:None,open_sites_per_slot:Some(4))".to_vec(),
        );
        let catalog = f.archive().content().unwrap();
        let repair = catalog.units[&ContentId::parse("cedar:worker").unwrap()]
            .repairer
            .as_ref()
            .unwrap();
        assert_eq!(repair.cost[0].resource, "cedar:credits");
        assert_eq!(repair.classes, ["cedar:heavy"]);
        assert_eq!(
            catalog.production[&ContentId::parse("cedar:hire").unwrap()].payment,
            PaymentPolicy::ChargeOverTime
        );
        assert_eq!(
            catalog.production[&ContentId::parse("cedar:build_relay").unwrap()].payment,
            payment
        );
    }
}

#[test]
fn repair_costs_reject_wrong_kinds_duplicates_and_unqualified_refs_with_paths() {
    let mut f = Fixture::new();
    f.worker.repairer.as_mut().unwrap().cost[0].resource = "cedar:wood".into();
    f.fail("repairer.cost[0].resource");
    let mut f = Fixture::new();
    let repair = f.worker.repairer.as_mut().unwrap();
    repair.cost.push(repair.cost[0].clone());
    f.fail("repairer.cost[1].resource");
    let mut f = Fixture::new();
    f.worker.repairer.as_mut().unwrap().cost[0].resource = "credits".into();
    f.fail("repairer.cost[0].resource");
    let mut f = Fixture::new();
    f.worker.repairer.as_mut().unwrap().classes[0] = "missing:heavy".into();
    f.fail("repairer.classes[0]");
}
#[test]
fn repair_rejects_invalid_reach_ratios_range_and_unknown_classes() {
    for reach in [0., -1., f32::NAN, f32::INFINITY] {
        let mut f = Fixture::new();
        f.worker.repairer.as_mut().unwrap().reach = reach;
        f.fail("repairer.reach");
    }
    let mut f = Fixture::new();
    f.worker.repairer.as_mut().unwrap().cost[0].per_health = 0;
    f.fail("repairer.cost[0].per_health");
    let mut f = Fixture::new();
    f.worker.repairer.as_mut().unwrap().auto_range = Some(0.1);
    f.fail("repairer.auto_range");
    let mut f = Fixture::new();
    f.worker.repairer.as_mut().unwrap().classes[0] = "cedar:missing".into();
    f.fail("repairer.classes[0]");
}
#[test]
fn repair_free_and_zero_cost_rows_remain_valid() {
    for empty in [false, true] {
        let mut f = Fixture::new();
        if empty {
            f.worker.repairer.as_mut().unwrap().cost.clear();
        } else {
            f.worker.repairer.as_mut().unwrap().cost[0].amount = 0;
        }
        f.archive().content().unwrap();
    }
}
#[test]
fn repair_and_incremental_payment_require_api14() {
    for floor in ["^0.2.3", "^0.2.4", "^0.2.13"] {
        let mut f = Fixture::new();
        f.manifest.engine_api = floor.into();
        f.manifest.exports.combat.clear();
        f.worker.repairer.as_mut().unwrap().classes.clear();
        let error = f.archive().content().unwrap_err().to_string();
        assert!(
            error.contains("needs engine api 0.2.14"),
            "{floor}: {error}"
        );
    }
}
#[test]
fn research_charge_over_time_stays_refused() {
    let mut f = Fixture::new();
    f.manifest.exports.technologies.push("tech.ron".into());
    f.files.insert(
        "tech.ron".into(),
        encode(&TechnologyDef {
            id: "upgrade".into(),
            name: "Upgrade".into(),
            prerequisites: vec![],
            duration_ms: 100,
            costs: vec![],
            payment: PaymentPolicy::ChargeOverTime,
            cancel_refunds: f.hire.cancel_refunds,
            effects: vec![],
        }),
    );
    let error = f.archive().content().unwrap_err().to_string();
    assert!(
        error.contains("`ChargeOverTime` is not supported for technologies"),
        "{error}"
    );
}

#[test]
fn repair_ui_accessors_and_predicates_require_api14_and_load_at_the_floor() {
    for (value, predicate, feature) in [
        (
            "SelectionPrimary(RepairState)",
            "SelectionSingle",
            "PrimaryField::RepairState",
        ),
        (
            "SelectionPrimary(RepairTargetName)",
            "SelectionSingle",
            "PrimaryField::RepairTargetName",
        ),
        (
            "SelectionPrimary(RepairResourceName)",
            "SelectionSingle",
            "PrimaryField::RepairResourceName",
        ),
        (
            "Literal(\"repair\")",
            "RepairStateIs(Working)",
            "Predicate::RepairStateIs",
        ),
        (
            "Literal(\"repair\")",
            "PrimaryHas(Repairer)",
            "Predicate::PrimaryHas(Repairer)",
        ),
        (
            "Literal(\"repair\")",
            "PrimaryHasValue(RepairState)",
            "PrimaryField::RepairState",
        ),
    ] {
        for floor in ["^0.2.13", "^0.2.14"] {
            let mut f = Fixture::new();
            f.manifest.engine_api = floor.into();
            f.manifest.exports.combat.clear();
            f.worker.repairer = None;
            f.hire.payment = PaymentPolicy::ReserveThenCharge;
            f.hire.cancel_refunds.destroyed = RefundRatio::ZERO;
            let mut hud: InterfaceDef = ron::de::from_bytes(&f.files["interface/hud.ron"]).unwrap();
            let text = if value.starts_with("Literal") {
                value.to_string()
            } else {
                format!("Bind({value})")
            };
            hud.root=vec![ron::from_str(&format!("(id:\"repair_state\",size:(w:Px(180.0),h:Px(24.0)),visible:[{predicate}],widget:Text(text:{text},font:None,size:12,colour:\"#ffffff\",align:Left,wrap:false))")).unwrap()];
            f.manifest.exports.interface = vec!["interface/hud.ron".into()];
            f.files.insert("interface/hud.ron".into(), encode(&hud));
            let result = f.archive().content();
            if floor == "^0.2.14" {
                result.unwrap_or_else(|error| panic!("{feature}: {error}"));
            } else {
                let error = result.unwrap_err().to_string();
                assert!(
                    error.contains(&format!("`{feature}` needs engine api 0.2.14")),
                    "{error}"
                );
            }
        }
    }
}
