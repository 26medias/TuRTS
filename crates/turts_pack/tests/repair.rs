//! Repair through public orders, ordinary weapon damage, and fixed simulation ticks.
use bevy_app::App;
use std::{collections::BTreeMap, path::Path};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{Command, CommandKind, OrderCommand, OrderTargetSim, QueueMode, SlotId, Vec2};

fn cid(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn placed(local: &str, owner: u8, x: f32, y: f32) -> Placement {
    Placement {
        unit: cid(local).to_string(),
        owner_slot: owner,
        pos: WorldPos { x, y },
        contents: vec![],
        facing: None,
        start_order: None,
    }
}
fn bare(base: &ResolvedUnit, local: &str) -> ResolvedUnit {
    let mut unit = base.clone();
    unit.id = cid(local);
    unit.name = local.into();
    unit.footprint = Footprint::Circle { radius: 0.5 };
    unit.movement = None;
    unit.inventory = None;
    unit.gatherer = None;
    unit.drop_off = None;
    unit.producer = None;
    unit.constructor = None;
    unit.repairer = None;
    unit.placement = None;
    unit.build_area = None;
    unit.output_inventory = None;
    unit.extractor = None;
    unit.conveyor = None;
    unit.inserter = None;
    unit.machine = None;
    unit.research = None;
    unit.health = None;
    unit.weapon = None;
    unit.armour = None;
    unit.acquire = None;
    unit.provides.clear();
    unit.consumes.clear();
    unit.income = None;
    unit.removal = Some(RemovalDef {
        by_command: true,
        contents: ContentsPolicy::Destroy,
    });
    unit
}
struct Fixture {
    catalog: ContentCatalog,
    map: MapDef,
    hz: u16,
}
impl Fixture {
    fn new(hz: u16) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
        let mut catalog = archive.content().unwrap();
        let mut map = archive.maps().unwrap().remove(0).1;
        let base = catalog.units[&cid("worker")].clone();
        let mut repairer = bare(&base, "repairer");
        repairer.movement = Some(MovementDef { speed: 6.0 });
        repairer.repairer = Some(RepairerDef {
            rate_per_second: 6,
            reach: 0.25,
            cost: vec![RepairCost {
                resource: cid("credits").to_string(),
                amount: 3,
                per_health: 4,
            }],
            targets: TargetFilter {
                slots: SlotRelation::Same,
            },
            classes: vec![],
            auto_range: None,
        });
        let mut target = bare(&base, "target");
        target.footprint = Footprint::Circle { radius: 1.0 };
        target.health = Some(HealthDef { max: 100 });
        let mut enemy = bare(&base, "enemy");
        enemy.health = Some(HealthDef { max: 100 });
        enemy.weapon = Some(ResolvedWeapon {
            range: 30.0,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 5000,
            damage: 90,
            ammo: None,
            targets: TargetFilter {
                slots: SlotRelation::Other,
            },
            damage_type: None,
            bonus: vec![],
            when_short: vec![],
        });
        catalog.units = [repairer, target, enemy]
            .into_iter()
            .map(|unit| (unit.id.clone(), unit))
            .collect();
        catalog.resources = ["credits", "gold"]
            .into_iter()
            .map(|local| {
                (
                    cid(local),
                    ResourceDef {
                        id: local.into(),
                        name: local.into(),
                        unit_name: String::new(),
                        kind: ResourceKind::Balance,
                    },
                )
            })
            .collect();
        catalog.nodes.clear();
        catalog.production.clear();
        catalog.recipes.clear();
        catalog.technologies.clear();
        catalog.combat = None;
        catalog.source_files = BTreeMap::new();
        catalog.orders = [
            ("repair", Behaviour::Repair, TargetKind::Unit),
            ("stop", Behaviour::Stop, TargetKind::None),
            ("move", Behaviour::Move, TargetKind::Point),
            ("hold", Behaviour::Hold, TargetKind::None),
        ]
        .into_iter()
        .map(|(local, behaviour, target)| {
            (
                cid(local),
                OrderDef {
                    id: local.into(),
                    name: local.into(),
                    behaviour,
                    target,
                    fire: None,
                    engage: None,
                    icon: String::new(),
                    hotkey: String::new(),
                    cursor: None,
                    card: None,
                    units: vec![],
                    description: String::new(),
                },
            )
        })
        .collect();
        map.size = MapSize { x: 64.0, y: 64.0 };
        map.nodes.clear();
        map.belts.clear();
        map.environment = None;
        map.ground = None;
        map.placements = vec![
            placed("repairer", 0, 7.0, 8.0),
            placed("target", 0, 10.0, 8.0),
            placed("enemy", 1, 16.0, 8.0),
        ];
        map.slot_economy = vec![SlotEconomyInit {
            slot: 0,
            balances: vec![
                BalanceAmount {
                    balance: cid("credits").to_string(),
                    amount: 1000,
                },
                BalanceAmount {
                    balance: cid("gold").to_string(),
                    amount: 0,
                },
            ],
        }];
        Self { catalog, map, hz }
    }
    fn repairer(&mut self) -> &mut RepairerDef {
        self.catalog
            .units
            .get_mut(&cid("repairer"))
            .unwrap()
            .repairer
            .as_mut()
            .unwrap()
    }
    fn app(self) -> App {
        let mut app = turts_sim::headless_app(self.hz);
        let staged = turts_sim::stage_package(
            app.world(),
            self.catalog,
            SourceDef {
                file: "repair/field.ron".into(),
                value: self.map,
            },
            self.hz,
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        turts_sim::initialize_completed_world(app.world_mut());
        app
    }
}
fn id(app: &App, local: &str) -> u32 {
    turts_sim::completed_world(app.world())
        .units
        .iter()
        .find(|unit| unit.content_id == cid(local).to_string())
        .unwrap()
        .unit_id
}
fn health(app: &mut App, target: u32) -> u64 {
    turts_sim::world_view(app.world_mut())
        .m3
        .healths
        .iter()
        .find(|row| row.unit == target)
        .unwrap()
        .current
}
fn balance(app: &mut App, local: &str) -> u64 {
    turts_sim::world_view(app.world_mut())
        .balances
        .iter()
        .find(|row| row.slot == 0 && row.resource == cid(local))
        .unwrap()
        .on_hand
}
fn position(app: &mut App, unit: u32) -> Vec2 {
    turts_sim::world_view(app.world_mut())
        .units
        .iter()
        .find(|row| row.unit_id == unit)
        .unwrap()
        .pos
}
fn issue(
    app: &mut App,
    source: u32,
    local: &str,
    target: OrderTargetSim,
    queue: QueueMode,
) -> turts_sim::CommandResult {
    let (behaviour, target_kind) = match local {
        "repair" => (Behaviour::Repair, TargetKind::Unit),
        "stop" => (Behaviour::Stop, TargetKind::None),
        "hold" => (Behaviour::Hold, TargetKind::None),
        _ => (Behaviour::Move, TargetKind::Point),
    };
    turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 0,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: cid(local),
                behaviour,
                target_kind,
                policy: None,
                units: vec![source],
                target,
                queue,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        },
    )
}
fn damage_once(app: &mut App) -> u32 {
    let target = id(app, "target");
    let enemy = id(app, "enemy");
    app.update();
    assert_eq!(
        health(app, target),
        10,
        "ordinary enemy weapon supplies the repair damage"
    );
    let result = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(1),
            seq: 0,
            kind: CommandKind::SetEnabled {
                unit_id: enemy,
                enabled: false,
            },
        },
    );
    assert!(result.outcome.is_ok());
    target
}
fn accept(result: turts_sim::CommandResult) {
    assert!(result.outcome.is_ok(), "{result:?}");
    assert!(result.units.refused.is_empty(), "{result:?}");
}

#[test]
fn manual_repair_approaches_without_overlap_then_pays_exactly_at_two_tick_rates() {
    for hz in [5, 20] {
        let mut app = Fixture::new(hz).app();
        let source = id(&app, "repairer");
        let target = damage_once(&mut app);
        accept(issue(
            &mut app,
            source,
            "repair",
            OrderTargetSim::Unit(target),
            QueueMode::Replace,
        ));
        let mut previous = 10;
        for _ in 0..u32::from(hz) * 20 {
            app.update();
            let now = health(&mut app, target);
            let gap = position(&mut app, source).distance(position(&mut app, target)) - 1.5;
            assert!(
                gap >= -0.00001,
                "movement keeps gameplay footprints disjoint"
            );
            if now > previous {
                assert!(gap <= 0.25001, "healing requires authored footprint reach");
            }
            previous = now;
            if now == 100 {
                break;
            }
        }
        assert_eq!(health(&mut app, target), 100);
        assert_eq!(
            balance(&mut app, "credits"),
            933,
            "90 applied health costs floor(90 * 3/4)"
        );
        app.update();
        assert!(
            turts_sim::completed_world(app.world())
                .unit_orders
                .iter()
                .find(|row| row.unit_id == source)
                .is_none_or(|row| row.current.is_none())
        );
    }
}

#[test]
fn repeated_stop_and_retarget_cannot_reset_fractional_repair_cost() {
    let mut fixture = Fixture::new(20);
    fixture.repairer().rate_per_second = 20;
    fixture.repairer().cost[0].amount = 1;
    fixture.repairer().cost[0].per_health = 4;
    fixture.map.placements[0].pos.x = 8.25;
    let mut app = fixture.app();
    let source = id(&app, "repairer");
    let target = damage_once(&mut app);
    for _ in 0..8 {
        accept(issue(
            &mut app,
            source,
            "repair",
            OrderTargetSim::Unit(target),
            QueueMode::Replace,
        ));
        app.update();
        accept(issue(
            &mut app,
            source,
            "stop",
            OrderTargetSim::None,
            QueueMode::Replace,
        ));
        app.update();
    }
    assert_eq!(health(&mut app, target), 18);
    assert_eq!(balance(&mut app, "credits"), 998);
}

#[test]
fn sub_tick_heal_rate_advances_zero_heal_ticks_and_free_repair_is_valid() {
    for hz in [5, 20] {
        let mut fixture = Fixture::new(hz);
        fixture.repairer().rate_per_second = 1;
        fixture.repairer().cost.clear();
        fixture.map.placements[0].pos.x = 8.25;
        let mut app = fixture.app();
        let source = id(&app, "repairer");
        let target = damage_once(&mut app);
        accept(issue(
            &mut app,
            source,
            "repair",
            OrderTargetSim::Unit(target),
            QueueMode::Replace,
        ));
        for _ in 0..hz - 1 {
            app.update();
        }
        assert_eq!(health(&mut app, target), 10);
        app.update();
        assert_eq!(health(&mut app, target), 11);
        assert_eq!(balance(&mut app, "credits"), 1000);
    }
}

#[test]
fn unaffordable_multi_cost_repair_is_atomic_and_income_resumes_it() {
    let mut fixture = Fixture::new(20);
    fixture.repairer().rate_per_second = 20;
    fixture.repairer().cost = ["credits", "gold"]
        .into_iter()
        .map(|local| RepairCost {
            resource: cid(local).to_string(),
            amount: 1,
            per_health: 1,
        })
        .collect();
    fixture.map.placements[0].pos.x = 8.25;
    let mut payer = fixture.catalog.units[&cid("repairer")].clone();
    payer.id = cid("payer");
    payer.name = "payer".into();
    payer.repairer = None;
    payer.movement = None;
    payer.income = Some(IncomeDef {
        resource: cid("gold").to_string(),
        amount: 2,
        period_ms: 200,
    });
    fixture.catalog.units.insert(payer.id.clone(), payer);
    fixture.map.placements.push(placed("payer", 0, 40.0, 40.0));
    let mut app = fixture.app();
    let source = id(&app, "repairer");
    let target = damage_once(&mut app);
    accept(issue(
        &mut app,
        source,
        "repair",
        OrderTargetSim::Unit(target),
        QueueMode::Replace,
    ));
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(health(&mut app, target), 10);
    assert_eq!(balance(&mut app, "credits"), 1000);
    assert_eq!(balance(&mut app, "gold"), 0);
    app.update();
    assert_eq!(health(&mut app, target), 11);
    assert_eq!(balance(&mut app, "credits"), 999);
    assert_eq!(balance(&mut app, "gold"), 1);
}

#[test]
fn stop_latches_automation_until_an_accepted_move_finishes() {
    let mut fixture = Fixture::new(20);
    fixture.repairer().auto_range = Some(12.0);
    fixture.repairer().rate_per_second = 20;
    fixture.repairer().cost.clear();
    fixture.map.placements[0].pos.x = 8.25;
    let mut app = fixture.app();
    let source = id(&app, "repairer");
    let target = damage_once(&mut app);
    accept(issue(
        &mut app,
        source,
        "stop",
        OrderTargetSim::None,
        QueueMode::Replace,
    ));
    let refused = issue(
        &mut app,
        source,
        "repair",
        OrderTargetSim::Unit(u32::MAX),
        QueueMode::Replace,
    );
    assert!(!refused.units.refused.is_empty());
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(health(&mut app, target), 10);
    accept(issue(
        &mut app,
        source,
        "move",
        OrderTargetSim::Point(Vec2::new(7.0, 10.0)),
        QueueMode::Replace,
    ));
    for _ in 0..100 {
        app.update();
    }
    assert!(health(&mut app, target) > 10);
}

#[test]
fn two_repairers_cannot_overheal_or_overcharge_a_shared_target() {
    let mut fixture = Fixture::new(20);
    fixture.repairer().rate_per_second = 2000;
    fixture.repairer().cost[0].amount = 1;
    fixture.repairer().cost[0].per_health = 1;
    fixture.map.placements[0].pos.x = 8.25;
    let mut second = fixture.catalog.units[&cid("repairer")].clone();
    second.id = cid("second");
    second.name = "second".into();
    fixture.catalog.units.insert(second.id.clone(), second);
    fixture.map.placements.push(placed("second", 0, 10.0, 6.25));
    let mut app = fixture.app();
    let source = id(&app, "repairer");
    let second = id(&app, "second");
    let target = damage_once(&mut app);
    accept(issue(
        &mut app,
        source,
        "repair",
        OrderTargetSim::Unit(target),
        QueueMode::Replace,
    ));
    accept(issue(
        &mut app,
        second,
        "repair",
        OrderTargetSim::Unit(target),
        QueueMode::Replace,
    ));
    app.update();
    assert_eq!(health(&mut app, target), 100);
    assert_eq!(balance(&mut app, "credits"), 910);
}
