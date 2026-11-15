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

#[test]
fn surviving_inventory_refund_is_all_or_discard_and_manual_cancel_stays_atomic() {
    let mut f = Fixture::new();
    f.stock("builder", 20, 6, 6);
    f.stock("store", 100, 20, 0);
    let recipe = f.catalog.production.get_mut(&cid("build")).unwrap();
    recipe.duration_ms = 100;
    recipe.costs = vec![
        cost("ore", 6, CostSource::ProducerInventory),
        cost("gold", 6, CostSource::ProducerInventory),
    ];
    let mut app = f.app();
    let builder = id(&app, "builder");
    let store = id(&app, "store");
    let site = build(&mut app, builder, 30.);
    app.update();
    assert_eq!(stock(&mut app, builder, "ore"), 3);
    assert_eq!(stock(&mut app, builder, "gold"), 3);
    enabled(&mut app, builder, 0, false);
    transaction::inventory_transfer(app.world_mut(), SlotId(0), store, builder, cid("ore"), 11)
        .unwrap();
    let before = live(&mut app).construction.sites[&site].clone();
    assert_eq!(
        construction::cancel_site(app.world_mut(), SlotId(0), site),
        Err(RejectReason::RefundDestinationFull)
    );
    assert_eq!(live(&mut app).construction.sites[&site], before);
    kill(&mut app, site);
    assert_eq!(stock(&mut app, builder, "ore"), 14);
    assert_eq!(stock(&mut app, builder, "gold"), 3);
    assert_eq!(balance(&mut app), 100);
    let (desired, credited, discarded) = refunds_for(&app, site);
    assert!(credited.is_empty());
    for name in ["ore", "gold"] {
        assert_eq!(sum(&desired, EventSource::Inventory(builder), name), 3);
        assert_eq!(sum(&discarded, EventSource::Inventory(builder), name), 3);
    }
    once(&mut app);
}

#[test]
fn destroyed_original_inventory_is_discarded_without_crediting_its_dying_owner() {
    let mut f = Fixture::new();
    f.stock("factory", 20, 10, 0);
    let recipe = f.catalog.production.get_mut(&cid("train")).unwrap();
    recipe.duration_ms = 300;
    recipe.costs = vec![
        cost("credits", 12, CostSource::SlotBalance),
        cost("ore", 12, CostSource::ProducerInventory),
    ];
    let mut app = f.app();
    let factory = id(&app, "factory");
    queue(&mut app, factory, "train");
    app.update();
    app.update();
    assert_eq!(balance(&mut app), 98);
    assert_eq!(stock(&mut app, factory, "ore"), 8);
    kill(&mut app, factory);
    assert_eq!(balance(&mut app), 100);
    assert!(
        !live(&mut app)
            .inventories
            .iter()
            .any(|row| row.unit == factory)
    );
    let (desired, credited, discarded) = refunds_for(&app, factory);
    assert_eq!(sum(&desired, EventSource::Slot(0), "credits"), 4);
    assert_eq!(sum(&credited, EventSource::Slot(0), "credits"), 4);
    assert_eq!(sum(&desired, EventSource::Inventory(factory), "ore"), 4);
    assert_eq!(sum(&credited, EventSource::Inventory(factory), "ore"), 0);
    assert_eq!(sum(&discarded, EventSource::Inventory(factory), "ore"), 4);
    once(&mut app);
}

#[test]
fn missing_constructor_inventory_discards_site_refund_but_keeps_balance_refund() {
    let mut f = Fixture::new();
    f.stock("builder", 20, 10, 0);
    let recipe = f.catalog.production.get_mut(&cid("build")).unwrap();
    recipe.duration_ms = 200;
    recipe.costs = vec![
        cost("credits", 8, CostSource::SlotBalance),
        cost("ore", 8, CostSource::ProducerInventory),
    ];
    let mut app = f.app();
    let builder = id(&app, "builder");
    let site = build(&mut app, builder, 30.);
    app.update();
    assert_eq!(balance(&mut app), 98);
    kill(&mut app, builder);
    assert_eq!(balance(&mut app), 96);
    assert_eq!(live(&mut app).construction.sites[&site].done, 2);
    kill(&mut app, site);
    assert_eq!(balance(&mut app), 100);
    let (desired, credited, discarded) = refunds_for(&app, site);
    assert_eq!(sum(&desired, EventSource::Slot(0), "credits"), 4);
    assert_eq!(sum(&credited, EventSource::Slot(0), "credits"), 4);
    assert_eq!(sum(&desired, EventSource::Inventory(builder), "ore"), 4);
    assert_eq!(sum(&discarded, EventSource::Inventory(builder), "ore"), 4);
    once(&mut app);
}

#[test]
fn constructor_kill_drops_held_requests_but_its_paid_sites_continue() {
    let mut f = Fixture::new();
    let recipe = f.catalog.production.get_mut(&cid("build")).unwrap();
    recipe.duration_ms = 5000;
    recipe.costs[0].amount = 100;
    let mut app = f.app();
    let builder = id(&app, "builder");
    let site = build(&mut app, builder, 30.);
    assert_eq!(
        construction::construct(
            app.world_mut(),
            SlotId(0),
            builder,
            cid("build"),
            Vec2::new(34., 24.)
        )
        .unwrap(),
        None
    );
    app.update();
    assert_eq!(balance(&mut app), 99);
    assert_eq!(live(&mut app).construction.held[&builder].len(), 1);
    kill(&mut app, builder);
    assert!(!live(&mut app).construction.held.contains_key(&builder));
    assert_eq!(live(&mut app).construction.sites[&site].done, 2);
    assert_eq!(balance(&mut app), 98);
    app.update();
    assert_eq!(live(&mut app).construction.sites[&site].done, 3);
    assert_eq!(balance(&mut app), 97);
    assert_eq!(
        live(&mut app).construction.sites.len(),
        1,
        "the removed constructor cannot place its old held request"
    );
}

fn cancel_group(app: &mut App, units: Vec<u32>) -> turts_sim::CommandResult {
    turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 99,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: cid("cancel"),
                behaviour: Behaviour::CancelConstruction,
                target_kind: TargetKind::None,
                policy: None,
                units,
                target: OrderTargetSim::None,
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        },
    )
}
#[test]
fn grouped_manual_refund_competition_uses_site_position_after_creation_id_inversion() {
    for early_created_first in [false, true] {
        let mut f = Fixture::new();
        f.definition("builder")
            .constructor
            .as_mut()
            .unwrap()
            .concurrent_sites = 2;
        f.map.slot_economy[0].balances[0].amount = u64::MAX;
        f.catalog.production.get_mut(&cid("build")).unwrap().payment = PaymentPolicy::ChargeOnQueue;
        f.definition("income").income = Some(IncomeDef {
            resource: cid("credits").to_string(),
            amount: 10,
            period_ms: 100,
        });
        let mut app = f.app();
        let builder = id(&app, "builder");
        let (early, late) = if early_created_first {
            let early = build(&mut app, builder, 8.);
            (early, build(&mut app, builder, 30.))
        } else {
            let late = build(&mut app, builder, 30.);
            (build(&mut app, builder, 8.), late)
        };
        assert_eq!(early < late, early_created_first);
        app.update();
        app.update();
        assert_eq!(balance(&mut app), u64::MAX - 10);
        let result = cancel_group(&mut app, vec![late, early]);
        assert_eq!(result.units.accepted, vec![early]);
        assert_eq!(
            result.units.refused,
            vec![(late, RejectReason::RefundDestinationOverflow)]
        );
        assert_eq!(balance(&mut app), u64::MAX);
        assert!(!live(&mut app).construction.sites.contains_key(&early));
        assert!(live(&mut app).construction.sites.contains_key(&late));
    }
}
#[test]
fn grouped_manual_cancel_refuses_exact_site_position_ties_symmetrically() {
    let mut f = Fixture::new();
    f.definition("builder")
        .constructor
        .as_mut()
        .unwrap()
        .concurrent_sites = 2;
    f.catalog.production.get_mut(&cid("build")).unwrap().payment = PaymentPolicy::ChargeOnQueue;
    let mut app = f.app();
    let builder = id(&app, "builder");
    let a = build(&mut app, builder, 8.);
    let b = build(&mut app, builder, 30.);
    // Legal placement cannot overlap static sites. Supply the exact rank-tie boundary directly.
    // Accounts, health, paid work, runtime identities, and site state remain normally authored.
    for id in [a, b] {
        let entity = app
            .world()
            .iter_entities()
            .find(|e| {
                e.get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == id)
            })
            .unwrap()
            .id();
        app.world_mut()
            .get_mut::<turts_sim::Position>(entity)
            .unwrap()
            .0 = Vec2::new(20., 24.);
    }
    let before = balance(&mut app);
    let result = cancel_group(&mut app, vec![b, a]);
    assert!(result.units.accepted.is_empty());
    assert_eq!(
        result.units.refused,
        vec![(a, RejectReason::Blocked), (b, RejectReason::Blocked)]
    );
    assert_eq!(balance(&mut app), before);
    assert_eq!(live(&mut app).construction.sites.len(), 2);
}
#[test]
fn appended_stop_clears_only_held_requests_on_a_hybrid_constructor() {
    let mut f = Fixture::new();
    let producer = f.definition("factory").producer.clone();
    f.definition("builder").producer = producer;
    let mut app = f.app();
    let builder = id(&app, "builder");
    let site = build(&mut app, builder, 30.);
    construction::construct(
        app.world_mut(),
        SlotId(0),
        builder,
        cid("build"),
        Vec2::new(34., 24.),
    )
    .unwrap();
    queue(&mut app, builder, "train");
    let result = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 99,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: cid("stop"),
                behaviour: Behaviour::Stop,
                target_kind: TargetKind::None,
                policy: None,
                units: vec![builder],
                target: OrderTargetSim::None,
                queue: QueueMode::Append,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        },
    );
    assert!(
        result.outcome.is_ok() && result.units.refused.is_empty(),
        "{result:?}"
    );
    let view = live(&mut app);
    assert!(!view.construction.held.contains_key(&builder));
    assert!(view.construction.sites.contains_key(&site));
    assert_eq!(
        view.queues
            .iter()
            .find(|queue| queue.producer == builder)
            .unwrap()
            .jobs
            .len(),
        1
    );
    assert_eq!(balance(&mut app), 100);
    app.update();
    assert_eq!(live(&mut app).construction.sites[&site].done, 1);
}
