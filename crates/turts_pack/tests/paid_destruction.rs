//! Paid work destruction uses ordinary orders, real weapons, and authored accounts.
use bevy_app::App;
use std::{collections::BTreeMap, path::Path};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, OrderCommand, OrderTargetSim, QueueMode, RejectReason, SlotId, Vec2,
    economy::{
        construction,
        events::{self, CostLine, EconomyEventKind, EventSource},
        production, transaction,
    },
};
fn cid(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn unit(base: &ResolvedUnit, local: &str) -> ResolvedUnit {
    let mut u = base.clone();
    u.id = cid(local);
    u.name = local.into();
    u.footprint = Footprint::Circle { radius: 0.5 };
    u.movement = None;
    u.inventory = None;
    u.gatherer = None;
    u.drop_off = None;
    u.producer = None;
    u.constructor = None;
    u.repairer = None;
    u.placement = None;
    u.build_area = None;
    u.output_inventory = None;
    u.extractor = None;
    u.conveyor = None;
    u.inserter = None;
    u.machine = None;
    u.research = None;
    u.health = Some(HealthDef { max: 100 });
    u.weapon = None;
    u.armour = None;
    u.acquire = None;
    u.provides.clear();
    u.consumes.clear();
    u.income = None;
    u.removal = None;
    u
}
fn placed(local: &str, slot: u8, x: f32, y: f32) -> Placement {
    Placement {
        unit: cid(local).to_string(),
        owner_slot: slot,
        pos: WorldPos { x, y },
        contents: vec![],
        facing: None,
        start_order: None,
    }
}
fn refunds() -> CancelRefunds {
    CancelRefunds {
        queued: RefundRatio::ONE,
        running: RefundRatio::ONE,
        output_blocked: RefundRatio::ONE,
        destroyed: RefundRatio::ONE,
    }
}
fn cost(local: &str, amount: u64, source: CostSource) -> ResolvedCost {
    ResolvedCost {
        resource: cid(local),
        amount,
        source,
    }
}
fn recipe(local: &str, output: &str) -> ResolvedProduction {
    ResolvedProduction {
        id: cid(local),
        name: local.into(),
        output_unit: cid(output),
        duration_ms: 50_000,
        costs: vec![cost("credits", 10, CostSource::SlotBalance)],
        payment: PaymentPolicy::ChargeOverTime,
        cancel_refunds: refunds(),
    }
}
struct Fixture {
    catalog: ContentCatalog,
    map: MapDef,
}
impl Fixture {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
        let mut catalog = archive.content().unwrap();
        let mut map = archive.maps().unwrap().remove(0).1;
        let base = catalog.units[&cid("worker")].clone();
        let mut factory = unit(&base, "factory");
        factory.producer = Some(ResolvedProducer {
            recipes: vec![cid("train")],
            queue_capacity: 8,
            spawn_offsets: vec![WorldPos { x: 0., y: 3. }],
            when_short: vec![],
        });
        let mut builder = unit(&base, "builder");
        builder.constructor = Some(ResolvedConstructor {
            buildings: vec![cid("build")],
            concurrent_sites: 1,
            queue_capacity: 8,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
        builder.build_area = Some(100.);
        let mut site = unit(&base, "site");
        site.placement = Some(SitePlacementDef {
            gap: 2.,
            node_clearance: 0.,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 4,
            },
        });
        let mut product = unit(&base, "product");
        product.movement = Some(MovementDef { speed: 3. });
        let mut lab = unit(&base, "lab");
        lab.research = Some(ResolvedResearchStation {
            technologies: vec![cid("upgrade"), cid("upgrade_b")],
            queue_capacity: 8,
        });
        let mut enemy = unit(&base, "enemy");
        enemy.weapon = Some(ResolvedWeapon {
            range: 120.,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 50,
            damage: 1000,
            ammo: None,
            targets: TargetFilter {
                slots: SlotRelation::Other,
            },
            damage_type: None,
            bonus: vec![],
            when_short: vec![],
        });
        let mut store = unit(&base, "store");
        store.inventory = Some(ResolvedInventory {
            capacity: 100,
            accepts: vec![cid("ore"), cid("gold")],
        });
        let income = unit(&base, "income");
        catalog.units = [factory, builder, site, product, lab, enemy, store, income]
            .into_iter()
            .map(|u| (u.id.clone(), u))
            .collect();
        catalog.resources = [
            ("credits", ResourceKind::Balance),
            ("ore", ResourceKind::Item),
            ("gold", ResourceKind::Item),
        ]
        .into_iter()
        .map(|(name, kind)| {
            (
                cid(name),
                ResourceDef {
                    id: name.into(),
                    name: name.into(),
                    unit_name: String::new(),
                    kind,
                },
            )
        })
        .collect();
        catalog.production = [recipe("train", "product"), recipe("build", "site")]
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();
        catalog.technologies = [("upgrade", 7), ("upgrade_b", 11)]
            .into_iter()
            .map(|(name, amount)| {
                let tech = ResolvedTechnology {
                    id: cid(name),
                    name: name.into(),
                    prerequisites: vec![],
                    duration_ms: 50_000,
                    costs: vec![cost("credits", amount, CostSource::SlotBalance)],
                    payment: PaymentPolicy::ReserveThenCharge,
                    cancel_refunds: refunds(),
                    effects: vec![],
                };
                (tech.id.clone(), tech)
            })
            .collect();
        catalog.orders = [
            ("attack", Behaviour::Attack, TargetKind::Unit),
            ("stop", Behaviour::Stop, TargetKind::None),
            ("cancel", Behaviour::CancelConstruction, TargetKind::None),
        ]
        .into_iter()
        .map(|(name, behaviour, target)| {
            (
                cid(name),
                OrderDef {
                    id: name.into(),
                    name: name.into(),
                    behaviour,
                    target,
                    fire: Some(FirePolicy::InRange),
                    engage: Some(EngagePolicy::None),
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
        catalog.nodes.clear();
        catalog.recipes.clear();
        catalog.combat = None;
        catalog.source_files = BTreeMap::new();
        map.size = MapSize { x: 128., y: 64. };
        map.nodes.clear();
        map.belts.clear();
        map.environment = None;
        map.ground = None;
        map.placements = vec![
            placed("factory", 0, 8., 8.),
            placed("builder", 0, 40., 8.),
            placed("lab", 0, 72., 8.),
            placed("store", 0, 104., 8.),
            placed("income", 0, 104., 32.),
            placed("enemy", 1, 64., 52.),
        ];
        map.slot_economy = vec![SlotEconomyInit {
            slot: 0,
            balances: vec![BalanceAmount {
                balance: cid("credits").to_string(),
                amount: 100,
            }],
        }];
        Self { catalog, map }
    }
    fn definition(&mut self, name: &str) -> &mut ResolvedUnit {
        self.catalog.units.get_mut(&cid(name)).unwrap()
    }
    fn stock(&mut self, name: &str, capacity: u64, ore: u64, gold: u64) {
        self.definition(name).inventory = Some(ResolvedInventory {
            capacity,
            accepts: vec![cid("ore"), cid("gold")],
        });
        self.map
            .placements
            .iter_mut()
            .find(|p| p.unit == cid(name).to_string())
            .unwrap()
            .contents = [("ore", ore), ("gold", gold)]
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(name, amount)| ItemStack {
                item: cid(name).to_string(),
                amount,
            })
            .collect();
    }
    fn app(self) -> App {
        let mut app = turts_sim::headless_app(20);
        let staged = turts_sim::stage_package(
            app.world(),
            self.catalog,
            SourceDef {
                file: "paid-destruction/field.ron".into(),
                value: self.map,
            },
            20,
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        app.world_mut()
            .insert_resource(construction::ConstructionLimit(8));
        turts_sim::initialize_completed_world(app.world_mut());
        let enemy = id(&app, "enemy");
        enabled(&mut app, enemy, 1, false);
        app
    }
}
fn id(app: &App, name: &str) -> u32 {
    turts_sim::completed_world(app.world())
        .units
        .iter()
        .find(|u| u.content_id == cid(name).to_string())
        .unwrap()
        .unit_id
}
fn live(app: &mut App) -> turts_sim::economy::views::WorldView {
    turts_sim::world_view(app.world_mut())
}
fn balance(app: &mut App) -> u64 {
    live(app)
        .balances
        .iter()
        .find(|b| b.slot == 0 && b.resource == cid("credits"))
        .unwrap()
        .on_hand
}
fn reserved(app: &mut App) -> u64 {
    live(app)
        .balances
        .iter()
        .find(|b| b.slot == 0 && b.resource == cid("credits"))
        .unwrap()
        .reserved
}
fn stock(app: &mut App, unit: u32, name: &str) -> u64 {
    live(app)
        .inventories
        .iter()
        .find(|i| i.unit == unit)
        .unwrap()
        .items
        .iter()
        .find(|r| r.resource == cid(name))
        .map_or(0, |r| r.on_hand)
}
fn enabled(app: &mut App, unit: u32, slot: u8, value: bool) {
    let result = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(slot),
            seq: 0,
            kind: CommandKind::SetEnabled {
                unit_id: unit,
                enabled: value,
            },
        },
    );
    assert!(result.outcome.is_ok(), "toggle: {:?}", result.outcome);
}
fn order(
    app: &mut App,
    slot: u8,
    source: u32,
    name: &str,
    behaviour: Behaviour,
    target: OrderTargetSim,
) {
    let result = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(slot),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: cid(name),
                behaviour,
                target_kind: if name == "attack" {
                    TargetKind::Unit
                } else {
                    TargetKind::None
                },
                policy: None,
                units: vec![source],
                target,
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        },
    );
    assert!(
        result.outcome.is_ok() && result.units.refused.is_empty(),
        "order {name}: {result:?}"
    );
}
fn queue(app: &mut App, source: u32, name: &str) {
    production::queue_production(app.world_mut(), SlotId(0), source, cid(name)).unwrap();
}
fn build(app: &mut App, builder: u32, x: f32) -> u32 {
    construction::construct(
        app.world_mut(),
        SlotId(0),
        builder,
        cid("build"),
        Vec2::new(x, 24.),
    )
    .unwrap();
    *live(app).construction.sites.keys().last().unwrap()
}
fn kill(app: &mut App, victim: u32) {
    let enemy = id(app, "enemy");
    order(
        app,
        1,
        enemy,
        "attack",
        Behaviour::Attack,
        OrderTargetSim::Unit(victim),
    );
    enabled(app, enemy, 1, true);
    app.update();
    assert!(
        !live(app).units.iter().any(|u| u.unit_id == victim),
        "ordinary weapon must remove victim"
    );
    assert!(
        events::committed_tick(app.world())
            .removed
            .contains(&victim)
    );
    enabled(app, enemy, 1, false);
}
fn refunds_for(app: &App, victim: u32) -> (Vec<CostLine>, Vec<CostLine>, Vec<CostLine>) {
    let rows = events::committed_tick(app.world())
        .events
        .iter()
        .filter_map(|e| {
            if let EconomyEventKind::DestructionRefunded {
                unit,
                desired,
                credited,
                discarded,
            } = &e.kind
            {
                assert_eq!(e.owner_slot, 0);
                (*unit == victim).then_some((desired.clone(), credited.clone(), discarded.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    rows[0].clone()
}
fn sum(rows: &[CostLine], source: EventSource, name: &str) -> u64 {
    rows.iter()
        .filter(|r| r.source == source && r.resource == cid(name))
        .map(|r| r.amount)
        .sum()
}
fn once(app: &mut App) {
    let prior = balance(app);
    app.update();
    assert_eq!(balance(app), prior);
    assert!(
        !events::committed_tick(app.world())
            .events
            .iter()
            .any(|e| matches!(e.kind, EconomyEventKind::DestructionRefunded { .. }))
    );
}

#[test]
fn ordinary_kill_refunds_only_paid_cot_progress_once_without_removal_permission() {
    for no_policy in [false, true] {
        let mut f = Fixture::new();
        if !no_policy {
            f.definition("factory").removal = Some(RemovalDef {
                by_command: false,
                contents: ContentsPolicy::Destroy,
            });
        }
        let recipe = f.catalog.production.get_mut(&cid("train")).unwrap();
        recipe.duration_ms = 300;
        recipe.costs[0].amount = 12;
        recipe.cancel_refunds.destroyed = RefundRatio {
            numerator: 1,
            denominator: 2,
        };
        let mut app = f.app();
        let factory = id(&app, "factory");
        queue(&mut app, factory, "train");
        queue(&mut app, factory, "train");
        app.update();
        app.update();
        assert_eq!(balance(&mut app), 98);
        kill(&mut app, factory);
        assert_eq!(balance(&mut app), 98);
        assert_eq!(reserved(&mut app), 0);
        let (desired, credited, discarded) = refunds_for(&app, factory);
        assert_eq!(sum(&desired, EventSource::Slot(0), "credits"), 2);
        assert_eq!(desired, credited);
        assert!(discarded.is_empty());
        once(&mut app);
    }
}
#[test]
fn producer_and_research_reservations_release_without_inventing_credits() {
    for research in [false, true] {
        let mut f = Fixture::new();
        let name = if research { "lab" } else { "factory" };
        f.catalog.production.get_mut(&cid("train")).unwrap().payment =
            PaymentPolicy::ReserveThenCharge;
        let mut app = f.app();
        let victim = id(&app, name);
        queue(&mut app, victim, if research { "upgrade" } else { "train" });
        queue(
            &mut app,
            victim,
            if research { "upgrade_b" } else { "train" },
        );
        app.update();
        assert_eq!(balance(&mut app), if research { 93 } else { 90 });
        assert_eq!(reserved(&mut app), if research { 11 } else { 10 });
        kill(&mut app, victim);
        assert_eq!(balance(&mut app), 100);
        assert_eq!(reserved(&mut app), 0);
        let (desired, credited, discarded) = refunds_for(&app, victim);
        assert_eq!(
            sum(&desired, EventSource::Slot(0), "credits"),
            if research { 7 } else { 10 }
        );
        assert_eq!(desired, credited);
        assert!(discarded.is_empty());
        once(&mut app);
    }
}
#[test]
fn mandatory_balance_refund_credits_only_real_headroom_and_still_kills() {
    let mut f = Fixture::new();
    f.map.slot_economy[0].balances[0].amount = u64::MAX;
    f.catalog.production.get_mut(&cid("train")).unwrap().payment = PaymentPolicy::ChargeOnQueue;
    f.definition("income").income = Some(IncomeDef {
        resource: cid("credits").to_string(),
        amount: 8,
        period_ms: 100,
    });
    let mut app = f.app();
    let factory = id(&app, "factory");
    queue(&mut app, factory, "train");
    app.update();
    assert_eq!(balance(&mut app), u64::MAX - 10);
    kill(&mut app, factory);
    assert_eq!(balance(&mut app), u64::MAX);
    let (desired, credited, discarded) = refunds_for(&app, factory);
    assert_eq!(sum(&desired, EventSource::Slot(0), "credits"), 10);
    assert_eq!(sum(&credited, EventSource::Slot(0), "credits"), 2);
    assert_eq!(sum(&discarded, EventSource::Slot(0), "credits"), 8);
}
