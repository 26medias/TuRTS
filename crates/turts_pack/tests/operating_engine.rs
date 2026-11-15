//! Operating work through public creation APIs and actual fixed simulation ticks.
use std::{collections::BTreeMap, path::Path};

use bevy_app::App;
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, IdAllocators, RejectReason, SlotId, Vec2,
    economy::{
        construction, production,
        views::{AuditState, WorldView},
    },
};

fn cid(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn short(local: &str, denominator: u32) -> WhenShort {
    WhenShort {
        resource: cid(local).to_string(),
        effect: ShortEffect::Rate(Ratio {
            numerator: 1,
            denominator,
        }),
    }
}
fn amount(local: &str, amount: u64) -> SupplyAmount {
    SupplyAmount {
        resource: cid(local).to_string(),
        amount,
    }
}
fn placement(local: &str, x: f32, y: f32) -> Placement {
    Placement {
        unit: cid(local).to_string(),
        owner_slot: 0,
        pos: WorldPos { x, y },
        contents: vec![],
        facing: None,
        start_order: None,
    }
}
fn bare(base: &ResolvedUnit, local: &str, mobile: bool) -> ResolvedUnit {
    let mut unit = base.clone();
    unit.id = cid(local);
    unit.name = local.into();
    unit.footprint = Footprint::Circle {
        radius: if mobile { 0.5 } else { 1.0 },
    };
    unit.movement = mobile.then_some(MovementDef { speed: 3.0 });
    unit.inventory = None;
    unit.gatherer = None;
    unit.drop_off = None;
    unit.producer = None;
    unit.constructor = None;
    unit.placement = None;
    unit.build_area = None;
    unit.output_inventory = None;
    unit.extractor = None;
    unit.conveyor = None;
    unit.inserter = None;
    unit.machine = None;
    unit.research = None;
    unit.health = Some(HealthDef { max: 100 });
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
fn recipe(local: &str, output: &str, ticks: u64, cost: u64) -> ResolvedProduction {
    ResolvedProduction {
        id: cid(local),
        name: local.into(),
        output_unit: cid(output),
        duration_ms: ticks * 50,
        costs: vec![ResolvedCost {
            resource: cid("credits"),
            amount: cost,
            source: CostSource::SlotBalance,
        }],
        payment: PaymentPolicy::ChargeOnQueue,
        cancel_refunds: CancelRefunds {
            queued: RefundRatio::ONE,
            running: RefundRatio::ONE,
            output_blocked: RefundRatio::ONE,
            destroyed: RefundRatio::ZERO,
        },
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
        let mut factory = bare(&base, "factory", false);
        factory.producer = Some(ResolvedProducer {
            recipes: vec![cid("train"), cid("train_b")],
            queue_capacity: 8,
            spawn_offsets: vec![WorldPos { x: 0.0, y: 3.0 }, WorldPos { x: 0.0, y: -3.0 }],
            when_short: vec![],
        });
        factory.placement = Some(SitePlacementDef {
            gap: 1.5,
            node_clearance: 0.0,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 4,
            },
        });
        let mut builder = bare(&base, "builder", false);
        builder.constructor = Some(ResolvedConstructor {
            buildings: vec![cid("build_relay"), cid("build_factory")],
            concurrent_sites: 8,
            queue_capacity: 8,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
        builder.build_area = Some(60.0);
        let mut relay = bare(&base, "relay", false);
        relay.placement = factory.placement;
        let supply = bare(&base, "supply", false);
        let product = bare(&base, "product", true);
        let product_b = bare(&base, "product_b", true);
        catalog.units = [factory, builder, relay, supply, product, product_b]
            .into_iter()
            .map(|unit| (unit.id.clone(), unit))
            .collect();
        catalog.resources = [
            (
                "power",
                ResourceKind::Supply {
                    scope: SupplyScope::Slot,
                    counts: SupplyCounts::Completed,
                },
            ),
            (
                "capacity",
                ResourceKind::Supply {
                    scope: SupplyScope::Slot,
                    counts: SupplyCounts::Completed,
                },
            ),
            ("credits", ResourceKind::Balance),
        ]
        .into_iter()
        .map(|(local, kind)| {
            (
                cid(local),
                ResourceDef {
                    id: local.into(),
                    name: local.into(),
                    unit_name: String::new(),
                    kind,
                },
            )
        })
        .collect();
        catalog.production = [
            recipe("train", "product", 1, 7),
            recipe("train_b", "product_b", 1, 7),
            recipe("build_relay", "relay", 2, 11),
            recipe("build_factory", "factory", 1, 13),
        ]
        .into_iter()
        .map(|recipe| (recipe.id.clone(), recipe))
        .collect();
        catalog.nodes.clear();
        catalog.recipes.clear();
        catalog.technologies.clear();
        catalog.orders.clear();
        catalog.combat = None;
        catalog.source_files = BTreeMap::new();
        map.size = MapSize { x: 64.0, y: 64.0 };
        map.nodes.clear();
        map.belts.clear();
        map.environment = None;
        map.ground = None;
        map.placements = vec![
            placement("factory", 8.0, 8.0),
            placement("builder", 54.0, 8.0),
            placement("supply", 54.0, 24.0),
        ];
        map.slot_economy = vec![SlotEconomyInit {
            slot: 0,
            balances: vec![BalanceAmount {
                balance: cid("credits").to_string(),
                amount: 1000,
            }],
        }];
        Self { catalog, map }
    }
    fn unit(&mut self, name: &str) -> &mut ResolvedUnit {
        self.catalog.units.get_mut(&cid(name)).unwrap()
    }
    fn ticks(&mut self, name: &str, ticks: u64) {
        self.catalog
            .production
            .get_mut(&cid(name))
            .unwrap()
            .duration_ms = ticks * 50;
    }
    fn app(self) -> App {
        let mut app = turts_sim::headless_app(20);
        let staged = turts_sim::stage_package(
            app.world(),
            self.catalog,
            SourceDef {
                file: "operating-engine/field.ron".into(),
                value: self.map,
            },
            20,
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        app.world_mut()
            .insert_resource(construction::ConstructionLimit(8));
        turts_sim::initialize_completed_world(app.world_mut());
        app
    }
}
fn audit(app: &mut App) -> AuditState {
    turts_sim::audit_state(app.world_mut())
}
fn balance(state: &AuditState) -> u64 {
    state
        .economy
        .balances
        .iter()
        .find(|row| row.slot == 0 && row.resource == cid("credits"))
        .unwrap()
        .on_hand
}
fn unit_id(app: &App, name: &str) -> u32 {
    view(app)
        .units
        .iter()
        .find(|unit| {
            unit.content_id == cid(name).to_string()
                && !view(app).construction.sites.contains_key(&unit.unit_id)
        })
        .unwrap()
        .unit_id
}
fn view(app: &App) -> &WorldView {
    turts_sim::completed_world(app.world())
}
fn build(app: &mut App, builder: u32, recipe: &str, x: f32, y: f32) -> u32 {
    construction::construct(
        app.world_mut(),
        SlotId(0),
        builder,
        cid(recipe),
        Vec2::new(x, y),
    )
    .unwrap();
    let state = turts_sim::world_view(app.world_mut());
    *state
        .construction
        .sites
        .iter()
        .find(|(_, site)| site.recipe == cid(recipe))
        .unwrap()
        .0
}
fn queue(app: &mut App, factory: u32, recipe: &str) {
    production::queue_production(app.world_mut(), SlotId(0), factory, cid(recipe)).unwrap();
}
fn remove(app: &mut App, unit: u32) {
    let result = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 100 + unit,
            kind: CommandKind::Remove { unit_id: unit },
        },
    );
    assert!(
        result.outcome.is_ok(),
        "remove failed: {:?}",
        result.outcome
    );
}
fn health(app: &App, unit: u32) -> u64 {
    view(app)
        .m3
        .raw_healths
        .iter()
        .find(|row| row.unit == unit)
        .unwrap()
        .current
}
fn raw_job(state: &AuditState, factory: u32) -> &turts_sim::economy::views::RawJobAudit {
    &state
        .raw_queues
        .iter()
        .find(|queue| queue.producer == factory)
        .unwrap()
        .jobs[0]
}
fn job_done(app: &App, factory: u32) -> u64 {
    view(app)
        .operating
        .as_ref()
        .unwrap()
        .jobs
        .iter()
        .find(|row| row.unit == factory)
        .unwrap()
        .done
}

#[derive(Clone, Copy)]
enum Bound {
    Supply,
    Income,
}
fn contribution(unit: &mut ResolvedUnit, bound: Bound, amount: u64) {
    match bound {
        Bound::Supply => unit.provides = vec![self::amount("power", amount)],
        Bound::Income => {
            unit.income = Some(IncomeDef {
                resource: cid("credits").to_string(),
                amount,
                period_ms: 1000,
            })
        }
    }
}
fn mixed_completion_case(factory_built: bool, bound: Bound) {
    let mut fixture = Fixture::new();
    contribution(fixture.unit("product"), bound, 1);
    contribution(fixture.unit("relay"), bound, 1);
    if factory_built {
        contribution(fixture.unit("builder"), bound, u64::MAX - 1);
        fixture.map.placements = vec![placement("builder", 54.0, 8.0)];
        fixture.ticks("build_relay", 3);
    } else {
        contribution(fixture.unit("supply"), bound, u64::MAX - 1)
    }
    let mut app = fixture.app();
    let builder = unit_id(&app, "builder");
    let site = build(&mut app, builder, "build_relay", 30.0, 8.0);
    let factory = if factory_built {
        let factory = build(&mut app, builder, "build_factory", 8.0, 8.0);
        app.update();
        assert!(!view(&app).construction.sites.contains_key(&factory));
        factory
    } else {
        unit_id(&app, "factory")
    };
    assert_eq!(
        factory > site,
        factory_built,
        "fixture must invert runtime identity order through normal creation"
    );
    queue(&mut app, factory, "train");
    app.update();
    let before = audit(&mut app);
    let before_site = serde_json::to_value(&before.economy.construction.sites[&site]).unwrap();
    assert_eq!(
        before.economy.construction.sites[&site].done + 1,
        before.economy.construction.sites[&site].total
    );
    let before_health = health(&app, site);
    app.update();
    let held = audit(&mut app);
    assert_eq!(
        view(&app)
            .units
            .iter()
            .filter(|unit| unit.content_id == cid("product").to_string())
            .count(),
        1
    );
    assert_eq!(
        serde_json::to_value(&held.economy.construction.sites[&site]).unwrap(),
        before_site,
        "overflow must not commit final site progress or charged resources"
    );
    assert_eq!(health(&app, site), before_health);
    assert_eq!(balance(&held), balance(&before));
    assert!(
        view(&app)
            .economy
            .queues
            .iter()
            .find(|queue| queue.producer == factory)
            .unwrap()
            .jobs
            .is_empty()
    );
    let hold = view(&app)
        .operating
        .as_ref()
        .unwrap()
        .holds
        .iter()
        .find(|hold| hold.unit == site)
        .unwrap();
    assert_eq!(hold.reason, RejectReason::ArithmeticOverflow);
    assert_eq!(hold.owner_slot, 0);
    let record = construction::progress_rows(view(&app))
        .into_iter()
        .find(|row| row.unit == site)
        .unwrap();
    assert!(record.held);
    assert_eq!(record.per_tick, 0);
    let released = if factory_built {
        builder
    } else {
        unit_id(&app, "supply")
    };
    remove(&mut app, released);
    app.update();
    assert!(!view(&app).construction.sites.contains_key(&site));
    assert!(
        !view(&app)
            .operating
            .as_ref()
            .unwrap()
            .holds
            .iter()
            .any(|hold| hold.unit == site)
    );
    assert_eq!(balance(&audit(&mut app)), balance(&before));
    let raw = audit(&mut app).raw_operating.unwrap();
    match bound {
        Bound::Supply => assert_eq!(
            raw.live_supply
                .iter()
                .find(|row| row.slot == 0 && row.resource == cid("power"))
                .unwrap()
                .provided,
            2
        ),
        Bound::Income => assert_eq!(
            raw.live_income
                .iter()
                .find(|row| row.0 == 0 && row.1 == cid("credits"))
                .unwrap()
                .2,
            2
        ),
    }
}
#[test]
fn factory_before_site_ids_wins_shared_supply_admission_and_site_recovers() {
    mixed_completion_case(false, Bound::Supply)
}
#[test]
fn factory_after_site_ids_still_wins_shared_supply_admission_and_site_recovers() {
    mixed_completion_case(true, Bound::Supply)
}
#[test]
fn shared_income_bound_uses_same_physical_creation_order_and_recovers() {
    mixed_completion_case(false, Bound::Income)
}

fn rate_fixture() -> Fixture {
    let mut fixture = Fixture::new();
    let constructor = fixture.catalog.units[&cid("builder")].constructor.clone();
    fixture.unit("factory").constructor = constructor;
    fixture
        .unit("factory")
        .constructor
        .as_mut()
        .unwrap()
        .when_short = vec![short("power", 2)];
    fixture
        .unit("factory")
        .producer
        .as_mut()
        .unwrap()
        .when_short = vec![short("power", 2), short("capacity", 3)];
    fixture.unit("supply").consumes = vec![amount("power", 20), amount("capacity", 1)];
    fixture.ticks("build_relay", 6);
    fixture.ticks("train", 6);
    fixture
}
#[test]
fn global_q_keeps_minimum_rates_fractional_progress_and_site_health_through_recovery() {
    let mut app = rate_fixture().app();
    let factory = unit_id(&app, "factory");
    let site = build(&mut app, factory, "build_relay", 30.0, 8.0);
    queue(&mut app, factory, "train");
    let expected = [(3, 0, 31), (6, 2, 37), (9, 4, 43), (12, 6, 50)];
    for (site_done, producer_done, hp) in expected {
        app.update();
        assert_eq!(view(&app).operating.as_ref().unwrap().rate_q, 6);
        assert_eq!(view(&app).construction.sites[&site].done, site_done);
        assert_eq!(view(&app).construction.sites[&site].total, 36);
        assert_eq!(job_done(&app, factory), producer_done);
        assert_eq!(health(&app, site), hp);
        let record = construction::progress_rows(view(&app))
            .into_iter()
            .find(|row| row.unit == site)
            .unwrap();
        assert_eq!(record.per_tick, 3);
    }
    let demand = unit_id(&app, "supply");
    remove(&mut app, demand);
    app.update();
    assert_eq!(view(&app).construction.sites[&site].done, 18);
    assert_eq!(job_done(&app, factory), 12);
    assert_eq!(health(&app, site), 62);
    let record = construction::progress_rows(view(&app))
        .into_iter()
        .find(|row| row.unit == site)
        .unwrap();
    assert_eq!(record.per_tick, 6);
}
#[test]
fn unfinished_site_retains_constructor_rate_after_ordinary_source_removal() {
    let mut app = rate_fixture().app();
    let factory = unit_id(&app, "factory");
    let site = build(&mut app, factory, "build_relay", 30.0, 8.0);
    app.update();
    app.update();
    assert_eq!(view(&app).construction.sites[&site].done, 6);
    remove(&mut app, factory);
    app.update();
    assert_eq!(view(&app).construction.sites[&site].done, 9);
    assert_eq!(health(&app, site), 43);
    let record = construction::progress_rows(view(&app))
        .into_iter()
        .find(|row| row.unit == site)
        .unwrap();
    assert_eq!(record.per_tick, 3);
}
#[test]
fn producer_half_rate_recovery_preserves_the_accumulated_fraction() {
    let mut fixture = Fixture::new();
    fixture
        .unit("factory")
        .producer
        .as_mut()
        .unwrap()
        .when_short = vec![short("power", 2)];
    fixture.unit("supply").consumes = vec![amount("power", 1)];
    fixture.ticks("train", 4);
    let mut app = fixture.app();
    let factory = unit_id(&app, "factory");
    queue(&mut app, factory, "train");
    app.update();
    app.update();
    assert_eq!(job_done(&app, factory), 1);
    let demand = unit_id(&app, "supply");
    remove(&mut app, demand);
    app.update();
    assert_eq!(job_done(&app, factory), 3);
    assert_eq!(view(&app).operating.as_ref().unwrap().jobs[0].total, 8);
    app.update();
    app.update();
    app.update();
    assert!(
        view(&app)
            .economy
            .queues
            .iter()
            .find(|queue| queue.producer == factory)
            .unwrap()
            .jobs
            .is_empty()
    );
}
#[test]
fn producer_overflow_keeps_final_progress_resources_and_ids_then_resumes() {
    let mut fixture = Fixture::new();
    fixture.unit("supply").provides = vec![amount("power", u64::MAX)];
    fixture.unit("product").provides = vec![amount("power", 1)];
    let mut app = fixture.app();
    let factory = unit_id(&app, "factory");
    queue(&mut app, factory, "train");
    app.update();
    let before = audit(&mut app);
    let ids = app.world().resource::<IdAllocators>().next_values();
    app.update();
    let after = audit(&mut app);
    assert_eq!(raw_job(&after, factory), raw_job(&before, factory));
    assert_eq!(balance(&after), balance(&before));
    assert_eq!(app.world().resource::<IdAllocators>().next_values(), ids);
    let hold = view(&app)
        .operating
        .as_ref()
        .unwrap()
        .holds
        .iter()
        .find(|hold| hold.unit == factory)
        .unwrap();
    assert_eq!(hold.reason, RejectReason::ArithmeticOverflow);
    let record = turts_sim::committed_tick(app.world())
        .job_progress
        .iter()
        .find(|row| row.unit == factory)
        .unwrap();
    assert!(record.held);
    assert_eq!(record.per_tick, 0);
    let provider = unit_id(&app, "supply");
    remove(&mut app, provider);
    app.update();
    assert!(
        view(&app)
            .economy
            .queues
            .iter()
            .find(|queue| queue.producer == factory)
            .unwrap()
            .jobs
            .is_empty()
    );
    assert!(
        view(&app)
            .operating
            .as_ref()
            .unwrap()
            .holds
            .iter()
            .all(|hold| hold.unit != factory)
    );
    assert_eq!(balance(&audit(&mut app)), balance(&before));
}
#[test]
fn ordinary_operating_outputs_keep_their_queue_fifo_order() {
    let mut app = Fixture::new().app();
    let factory = unit_id(&app, "factory");
    queue(&mut app, factory, "train_b");
    queue(&mut app, factory, "train");
    app.update();
    app.update();
    assert_eq!(
        view(&app)
            .units
            .iter()
            .filter(|unit| unit.content_id == cid("product_b").to_string())
            .count(),
        1
    );
    assert!(
        !view(&app)
            .units
            .iter()
            .any(|unit| unit.content_id == cid("product").to_string())
    );
    app.update();
    app.update();
    assert_eq!(
        view(&app)
            .units
            .iter()
            .filter(|unit| unit.content_id == cid("product").to_string())
            .count(),
        1
    );
    assert_eq!(balance(&audit(&mut app)), 986);
}

#[test]
fn spawned_and_completed_income_wait_a_full_period_from_their_actual_activation() {
    let mut fixture = Fixture::new();
    fixture.ticks("build_relay", 3);
    for (name, amount) in [("product", 5), ("relay", 7)] {
        fixture.unit(name).income = Some(IncomeDef {
            resource: cid("credits").to_string(),
            amount,
            period_ms: 1_050,
        });
    }
    let mut app = fixture.app();
    let factory = unit_id(&app, "factory");
    let builder = unit_id(&app, "builder");
    let site = build(&mut app, builder, "build_relay", 40.0, 20.0);
    queue(&mut app, factory, "train");
    let initial = balance(&audit(&mut app));
    assert!(audit(&mut app).raw_income_clocks.is_empty());
    let mut product_activation = None;
    let mut site_activation = None;
    while product_activation.is_none() || site_activation.is_none() {
        app.update();
        let state = audit(&mut app);
        let tick = state.economy.tick;
        assert!(tick < 8, "both ordinary creations must complete");
        if let Some(product) = state
            .economy
            .units
            .iter()
            .find(|unit| unit.content_id == cid("product").to_string())
        {
            product_activation.get_or_insert((product.unit_id, tick));
        }
        if !state.economy.construction.sites.contains_key(&site) {
            site_activation.get_or_insert(tick);
        }
        assert_eq!(balance(&state), initial, "no payment at activation");
        assert_eq!(
            state.raw_income_clocks.len(),
            usize::from(product_activation.is_some()) + usize::from(site_activation.is_some())
        );
    }
    let (product, product_tick) = product_activation.unwrap();
    let site_tick = site_activation.unwrap();
    assert!(
        site_tick > product_tick,
        "the two paths activate on different ticks"
    );
    let sources = [(product, product_tick, 5), (site, site_tick, 7)];
    loop {
        let state = audit(&mut app);
        let tick = state.economy.tick;
        let expected = initial
            + sources
                .iter()
                .map(|(_, activation, amount)| u64::from((tick - activation) / 21) * amount)
                .sum::<u64>();
        assert_eq!(balance(&state), expected, "income at tick {tick}");
        for &(unit, activation, _) in &sources {
            let clock = state
                .raw_income_clocks
                .iter()
                .find(|row| row.unit == unit)
                .unwrap();
            assert_eq!(clock.activation_tick, activation);
            assert_eq!(
                clock.next_due,
                u64::from(activation) + u64::from(((tick - activation) / 21 + 1) * 21)
            );
        }
        if tick >= site_tick + 42 {
            break;
        }
        app.update();
    }
}
