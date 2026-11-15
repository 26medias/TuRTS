//! Whole-package operating-resource acceptance on a small Cedar fixture.
mod common;

use std::collections::BTreeMap;
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

struct Fixture {
    files: BTreeMap<String, Vec<u8>>,
    manifest: Manifest,
    depot: UnitDef,
    worker: UnitDef,
    relay: UnitDef,
    power: ResourceDef,
    capacity: ResourceDef,
}

fn amount(resource: &str, amount: u64) -> SupplyAmount {
    SupplyAmount {
        resource: resource.into(),
        amount,
    }
}
fn rate(resource: &str, numerator: u32, denominator: u32) -> WhenShort {
    WhenShort {
        resource: resource.into(),
        effect: ShortEffect::Rate(Ratio {
            numerator,
            denominator,
        }),
    }
}
fn encode<T: serde::Serialize>(value: &T) -> Vec<u8> {
    ron::ser::to_string(value).unwrap().into_bytes()
}
fn id(text: &str) -> ContentId {
    ContentId::parse(text).unwrap()
}

impl Fixture {
    fn new() -> Self {
        let mut files = common::read_files(&common::package_dir("cedar"));
        let mut manifest: Manifest = ron::de::from_bytes(&files["manifest.ron"]).unwrap();
        manifest.engine_api = "^0.2.13".into();
        manifest.exports.resources.extend([
            "resources/power.ron".into(),
            "resources/capacity.ron".into(),
            "resources/credits.ron".into(),
        ]);
        manifest.exports.units.push("units/relay.ron".into());
        manifest.exports.visuals.push("visuals/relay.ron".into());
        manifest
            .exports
            .production
            .push("production/build_relay.ron".into());
        manifest.exports.match_files.push("match.ron".into());
        files.insert(
            "match.ron".into(),
            b"(start:Immediate,victory:None,open_sites_per_slot:Some(4))".to_vec(),
        );
        let mut depot: UnitDef = ron::de::from_bytes(&files["units/depot.ron"]).unwrap();
        depot.health = Some(HealthDef { max: 300 });
        depot.removal = Some(RemovalDef {
            by_command: false,
            contents: ContentsPolicy::Destroy,
        });
        depot.provides = vec![amount("cedar:power", 20), amount("cedar:capacity", 8)];
        depot.income = Some(IncomeDef {
            resource: "cedar:credits".into(),
            amount: 3,
            period_ms: 1050,
        });
        depot.producer.as_mut().unwrap().when_short =
            vec![rate("cedar:power", 1, 2), rate("cedar:capacity", 1, 3)];
        depot.constructor = Some(ConstructorDef {
            buildings: vec!["cedar:build_relay".into()],
            concurrent_sites: 2,
            queue_capacity: 2,
            progress: ConstructProgress::Autonomous,
            when_short: vec![rate("cedar:power", 1, 4)],
        });
        let mut worker: UnitDef = ron::de::from_bytes(&files["units/worker.ron"]).unwrap();
        worker.gatherer = None;
        worker.health = Some(HealthDef { max: 60 });
        worker.removal = Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy,
        });
        worker.consumes = vec![amount("cedar:power", 3), amount("cedar:capacity", 1)];
        worker.weapon = Some(WeaponDef {
            range: 4.,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 1000,
            damage: 10,
            ammo: None,
            targets: TargetFilter {
                slots: SlotRelation::Enemy,
            },
            damage_type: None,
            bonus: Vec::new(),
            when_short: vec![WhenShort {
                resource: "cedar:power".into(),
                effect: ShortEffect::Disable,
            }],
        });
        let mut relay = depot.clone();
        relay.id = "relay".into();
        relay.name = "Relay".into();
        relay.inventory = None;
        relay.drop_off = None;
        relay.producer = None;
        relay.constructor = None;
        relay.income = None;
        relay.provides = vec![amount("cedar:power", 6)];
        relay.placement = Some(SitePlacementDef {
            gap: 1.2,
            node_clearance: 0.,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 4,
            },
        });
        let visual = String::from_utf8(files["visuals/depot.ron"].clone())
            .unwrap()
            .replace("cedar:depot", "cedar:relay");
        files.insert("visuals/relay.ron".into(), visual.into_bytes());
        let mut building: ProductionDef =
            ron::de::from_bytes(&files["production/hire.ron"]).unwrap();
        building.id = "build_relay".into();
        building.name = "Build relay".into();
        building.output_unit = "cedar:relay".into();
        building.duration_ms = 1000;
        files.insert("production/build_relay.ron".into(), encode(&building));
        let credits = ResourceDef {
            id: "credits".into(),
            name: "Credits".into(),
            unit_name: "credit".into(),
            kind: ResourceKind::Balance,
        };
        files.insert("resources/credits.ron".into(), encode(&credits));
        let power = ResourceDef {
            id: "power".into(),
            name: "Power".into(),
            unit_name: "watt".into(),
            kind: ResourceKind::Supply {
                scope: SupplyScope::Slot,
                counts: SupplyCounts::Completed,
            },
        };
        let capacity = ResourceDef {
            id: "capacity".into(),
            name: "Capacity".into(),
            unit_name: "place".into(),
            kind: power.kind,
        };
        Self {
            files,
            manifest,
            depot,
            worker,
            relay,
            power,
            capacity,
        }
    }

    fn files(mut self) -> BTreeMap<String, Vec<u8>> {
        self.files
            .insert("manifest.ron".into(), encode(&self.manifest));
        self.files
            .insert("units/depot.ron".into(), encode(&self.depot));
        self.files
            .insert("units/worker.ron".into(), encode(&self.worker));
        self.files
            .insert("units/relay.ron".into(), encode(&self.relay));
        self.files
            .insert("resources/power.ron".into(), encode(&self.power));
        self.files
            .insert("resources/capacity.ron".into(), encode(&self.capacity));
        self.files
    }

    fn load(self) -> Result<ContentCatalog, ModError> {
        archive(self.files()).content()
    }
}

fn archive(files: BTreeMap<String, Vec<u8>>) -> PackageArchive {
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}

#[test]
fn operating_complete_two_slot_package_resolves_supply_income_and_capability_rules() {
    let files = Fixture::new().files();
    let package = archive(files.clone());
    let content = package.content().unwrap();
    package.visuals().unwrap();
    let depot = &content.units[&id("cedar:depot")];
    assert_eq!(
        depot.provides,
        [amount("cedar:power", 20), amount("cedar:capacity", 8)]
    );
    assert_eq!(depot.income.as_ref().unwrap().period_ms, 1050);
    assert_eq!(
        depot.producer.as_ref().unwrap().when_short,
        [rate("cedar:power", 1, 2), rate("cedar:capacity", 1, 3)]
    );
    assert_eq!(
        depot.constructor.as_ref().unwrap().when_short,
        [rate("cedar:power", 1, 4)]
    );
    assert_eq!(
        content.units[&id("cedar:worker")]
            .weapon
            .as_ref()
            .unwrap()
            .when_short[0]
            .effect,
        ShortEffect::Disable
    );
    let map = &package.maps().unwrap()[0].1;
    assert!(
        map.placements
            .iter()
            .any(|placement| placement.owner_slot == 0)
    );
    assert!(
        map.placements
            .iter()
            .any(|placement| placement.owner_slot == 1)
    );
    if let Ok(root) = std::env::var("TURTS_OPERATING_FIXTURE_DIR") {
        for (name, bytes) in files {
            let path = std::path::Path::new(&root).join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
    }
}

#[test]
fn operating_rows_refuse_unknown_resources_wrong_kinds_duplicates_and_zero_amounts() {
    for case in [
        "unknown_provider",
        "wrong_provider_kind",
        "duplicate_provider",
        "zero_provider",
        "unknown_consumer",
        "wrong_consumer_kind",
        "duplicate_consumer",
        "zero_consumer",
        "unknown_income",
        "wrong_income_kind",
        "unknown_constraint",
        "wrong_constraint_kind",
        "duplicate_constraint",
    ] {
        let mut f = Fixture::new();
        let field = match case {
            "unknown_provider" => {
                f.depot.provides[0].resource = "cedar:missing".into();
                "provides[0].resource"
            }
            "wrong_provider_kind" => {
                f.depot.provides[0].resource = "cedar:credits".into();
                "provides[0].resource"
            }
            "duplicate_provider" => {
                f.depot.provides.push(f.depot.provides[0].clone());
                "provides[2].resource"
            }
            "zero_provider" => {
                f.depot.provides[0].amount = 0;
                "provides[0].amount"
            }
            "unknown_consumer" => {
                f.worker.consumes[0].resource = "cedar:missing".into();
                "consumes[0].resource"
            }
            "wrong_consumer_kind" => {
                f.worker.consumes[0].resource = "cedar:wood".into();
                "consumes[0].resource"
            }
            "duplicate_consumer" => {
                f.worker.consumes.push(f.worker.consumes[0].clone());
                "consumes[2].resource"
            }
            "zero_consumer" => {
                f.worker.consumes[0].amount = 0;
                "consumes[0].amount"
            }
            "unknown_income" => {
                f.depot.income.as_mut().unwrap().resource = "cedar:missing".into();
                "income.resource"
            }
            "wrong_income_kind" => {
                f.depot.income.as_mut().unwrap().resource = "cedar:power".into();
                "income.resource"
            }
            "unknown_constraint" => {
                f.depot.producer.as_mut().unwrap().when_short[0].resource = "cedar:missing".into();
                "producer.when_short[0].resource"
            }
            "wrong_constraint_kind" => {
                f.depot.producer.as_mut().unwrap().when_short[0].resource = "cedar:credits".into();
                "producer.when_short[0].resource"
            }
            "duplicate_constraint" => {
                let rules = &mut f.depot.producer.as_mut().unwrap().when_short;
                rules.push(rules[0].clone());
                "producer.when_short[2].resource"
            }
            _ => unreachable!(),
        };
        let error = f.load().unwrap_err().to_string();
        assert!(error.contains(field), "{case}: {error}");
        assert!(error.contains("units/"), "{case}: {error}");
        assert!(
            !error.contains("not built"),
            "validation must run for completed fields: {error}"
        );
    }
}

#[test]
fn operating_capability_effects_refuse_invalid_rates_and_unsupported_combinations() {
    for (owner, effect) in [
        ("producer", ShortEffect::Disable),
        ("constructor", ShortEffect::Disable),
        (
            "weapon",
            ShortEffect::Rate(Ratio {
                numerator: 1,
                denominator: 2,
            }),
        ),
        ("producer", ShortEffect::BlockStart),
        ("weapon", ShortEffect::BlockStart),
        (
            "producer",
            ShortEffect::Rate(Ratio {
                numerator: 0,
                denominator: 1,
            }),
        ),
        (
            "constructor",
            ShortEffect::Rate(Ratio {
                numerator: 1,
                denominator: 0,
            }),
        ),
        (
            "producer",
            ShortEffect::Rate(Ratio {
                numerator: 2,
                denominator: 1,
            }),
        ),
    ] {
        let mut f = Fixture::new();
        match owner {
            "producer" => f.depot.producer.as_mut().unwrap().when_short[0].effect = effect,
            "constructor" => f.depot.constructor.as_mut().unwrap().when_short[0].effect = effect,
            "weapon" => f.worker.weapon.as_mut().unwrap().when_short[0].effect = effect,
            _ => unreachable!(),
        }
        let error = f.load().unwrap_err().to_string();
        assert!(
            error.contains(&format!("{owner}.when_short[0].effect")),
            "{owner} {effect:?}: {error}"
        );
        assert!(!error.contains("not built"), "{error}");
    }
}

#[test]
fn operating_income_requires_health_and_positive_amount_and_period() {
    for case in ["health", "amount", "period"] {
        let mut f = Fixture::new();
        match case {
            "health" => {
                f.depot.health = None;
                f.depot.removal = None;
            }
            "amount" => f.depot.income.as_mut().unwrap().amount = 0,
            "period" => f.depot.income.as_mut().unwrap().period_ms = 0,
            _ => unreachable!(),
        }
        let error = f.load().unwrap_err().to_string();
        assert!(
            error.contains("units/depot.ron") && error.contains("income"),
            "{case}: {error}"
        );
        assert!(
            error.contains(if case == "health" {
                "health"
            } else {
                "positive"
            }),
            "{case}: {error}"
        );
    }
}

#[test]
fn operating_reserved_network_and_queued_supply_remain_refused() {
    let mut f = Fixture::new();
    f.power.kind = ResourceKind::Supply {
        scope: SupplyScope::Slot,
        counts: SupplyCounts::Queued,
    };
    let error = f.load().unwrap_err().to_string();
    assert!(
        error.contains("resources/power.ron")
            && error.contains("kind.counts")
            && error.contains("reserved"),
        "{error}"
    );
    let mut files = Fixture::new().files();
    let power = String::from_utf8(files["resources/power.ron"].clone())
        .unwrap()
        .replace("scope:Slot", "scope:Network");
    assert!(power.contains("scope:Network"));
    files.insert("resources/power.ron".into(), power.into_bytes());
    let error = archive(files).content().unwrap_err().to_string();
    assert!(
        error.contains("resources/power.ron") && error.contains("Network"),
        "{error}"
    );
}

#[test]
fn operating_rate_scale_combines_all_units_and_capabilities_before_the_limit() {
    for (a, b, passes) in [(300, 300, true), (300, 301, false), (256, 256, true)] {
        let mut f = Fixture::new();
        f.depot.producer.as_mut().unwrap().when_short = vec![rate("cedar:power", 1, a)];
        f.depot.constructor.as_mut().unwrap().when_short.clear();
        let mut second = f.depot.clone();
        second.id = "second".into();
        second.income = None;
        second.producer.as_mut().unwrap().when_short = vec![rate("cedar:power", 1, b)];
        let mut files = f.files();
        let mut manifest: Manifest = ron::de::from_bytes(&files["manifest.ron"]).unwrap();
        manifest.exports.units.push("units/second.ron".into());
        manifest.exports.visuals.push("visuals/second.ron".into());
        files.insert("manifest.ron".into(), encode(&manifest));
        files.insert("units/second.ron".into(), encode(&second));
        let visual = String::from_utf8(files["visuals/depot.ron"].clone())
            .unwrap()
            .replace("cedar:depot", "cedar:second");
        files.insert("visuals/second.ron".into(), visual.into_bytes());
        let result = archive(files).content();
        if passes {
            result.unwrap();
        } else {
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("producer.when_short[0].effect")
                    && error.contains("90300")
                    && error.contains("65536"),
                "{error}"
            );
        }
    }
}

#[test]
fn operating_hud_values_predicate_and_transition_alerts_pack_through_the_public_archive() {
    let mut files = Fixture::new().files();
    let mut interface: InterfaceDef = ron::de::from_bytes(&files["interface/hud.ron"]).unwrap();
    let mut widget = interface.root[0].clone();
    widget.id = "operating_supply".into();
    widget.visible = vec![Predicate::SupplyShort("cedar:power".into())];
    widget.widget = Primitive::Text {
        text: TextValue::Format(
            "Power {0}/{1}".into(),
            vec![
                Value::Supply("cedar:power".into(), SupplySide::Provided),
                Value::Supply("cedar:power".into(), SupplySide::Consumed),
            ],
        ),
        font: None,
        size: 12,
        colour: "#ffffff".into(),
        align: TextAlign::Left,
        wrap: false,
    };
    interface.root.push(widget);
    for (name, trigger) in [("short", "SupplyShort"), ("restored", "SupplyRestored")] {
        let alert:AlertDef=ron::from_str(&format!("(id:\"{name}\",trigger:{trigger}(\"cedar:power\"),sound:None,text:\"Power changed\",minimap_ping:false,cooldown:1.0,priority:1,coalesce_radius:0.0)")).unwrap();
        interface.alerts.push(alert);
    }
    files.insert("interface/hud.ron".into(), encode(&interface));
    archive(files).content().unwrap();
}
