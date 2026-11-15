//! Paid work through public commands, public authoring, and real fixed ticks.
use std::{collections::BTreeMap, path::Path};

use bevy_app::App;
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, RejectReason, SlotId, Vec2,
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
    unit.repairer = None;
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
        payment: PaymentPolicy::ChargeOverTime,
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
        self.app_at(20)
    }
    fn app_at(self, hz: u16) -> App {
        let mut app = turts_sim::headless_app(hz);
        let staged = turts_sim::stage_package(
            app.world(),
            self.catalog,
            SourceDef {
                file: "operating-engine/field.ron".into(),
                value: self.map,
            },
            hz,
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

fn live(app: &mut App) -> WorldView {
    turts_sim::world_view(app.world_mut())
}
fn credits(app: &mut App) -> u64 {
    balance(&audit(app))
}
fn stock(app: &mut App, unit: u32) -> u64 {
    live(app)
        .economy
        .inventories
        .iter()
        .find(|row| row.unit == unit)
        .unwrap()
        .items
        .iter()
        .find(|row| row.resource == cid("ore"))
        .map_or(0, |row| row.on_hand)
}
fn set_enabled(app: &mut App, unit: u32, enabled: bool) {
    let outcome = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2000 + unit,
            kind: CommandKind::SetEnabled {
                unit_id: unit,
                enabled,
            },
        },
    );
    assert!(outcome.outcome.is_ok(), "toggle: {:?}", outcome.outcome);
}
fn funding(app: &mut App, unit: u32) -> production::FundingState {
    live(app)
        .operating
        .unwrap()
        .jobs
        .into_iter()
        .find(|row| row.unit == unit)
        .unwrap()
        .funding
        .unwrap()
}
fn mixed_costs(f: &mut Fixture, duration: u64, balance_cost: u64, item_cost: u64, stock: u64) {
    f.catalog.resources.insert(
        cid("ore"),
        ResourceDef {
            id: "ore".into(),
            name: "ore".into(),
            unit_name: String::new(),
            kind: ResourceKind::Item,
        },
    );
    for name in ["factory", "builder"] {
        f.unit(name).inventory = Some(ResolvedInventory {
            capacity: 100,
            accepts: vec![cid("ore")],
        });
        f.map
            .placements
            .iter_mut()
            .find(|row| row.unit == cid(name).to_string())
            .unwrap()
            .contents = vec![ItemStack {
            item: cid("ore").to_string(),
            amount: stock,
        }];
    }
    for name in ["train", "build_relay"] {
        let recipe = f.catalog.production.get_mut(&cid(name)).unwrap();
        recipe.duration_ms = duration;
        recipe.costs = vec![
            ResolvedCost {
                resource: cid("credits"),
                amount: balance_cost,
                source: CostSource::SlotBalance,
            },
            ResolvedCost {
                resource: cid("ore"),
                amount: item_cost,
                source: CostSource::ProducerInventory,
            },
        ];
    }
}

#[test]
fn cumulative_payment_matches_cost_at_two_tick_rates_and_global_q() {
    for hz in [10, 20] {
        for q in [1, 6] {
            let mut f = Fixture::new();
            mixed_costs(&mut f, 300, 7, 5, 20);
            if q == 6 {
                f.unit("factory").producer.as_mut().unwrap().when_short = vec![short("power", 2)];
                f.unit("builder").constructor.as_mut().unwrap().when_short =
                    vec![short("power", 3)];
                f.unit("supply").provides = vec![amount("power", 1)];
            }
            let mut app = f.app_at(hz);
            let factory = unit_id(&app, "factory");
            let builder = unit_id(&app, "builder");
            let site = build(&mut app, builder, "build_relay", 30., 8.);
            let initial = credits(&mut app);
            assert_eq!(initial, 1000, "admission debited COT");
            queue(&mut app, factory, "train");
            assert_eq!(stock(&mut app, factory), 20);
            app.update(); // queue starts; site advances immediately
            let n = u64::from(hz) * 3 / 10;
            assert_eq!(
                credits(&mut app),
                1000 - 7 / n,
                "site first cumulative floor: Hz={hz} Q={q}"
            );
            assert_eq!(job_done(&app, factory), 0);
            for tick in 1..=n {
                app.update();
                let expected_site = (tick + 1).min(n) * 7 / n;
                assert_eq!(
                    credits(&mut app),
                    1000 - expected_site - tick * 7 / n,
                    "Hz={hz} Q={q} tick={tick}"
                );
                if tick < n {
                    assert_eq!(job_done(&app, factory), tick * q);
                }
            }
            assert!(!view(&app).construction.sites.contains_key(&site));
            assert_eq!(credits(&mut app), 986);
            assert_eq!(stock(&mut app, factory), 15);
            assert_eq!(stock(&mut app, builder), 15);
            assert_eq!(
                view(&app)
                    .units
                    .iter()
                    .filter(|row| row.content_id == cid("product").to_string())
                    .count(),
                1
            );
        }
    }
}

#[test]
fn mixed_source_failure_is_atomic_and_cancel_returns_only_actual_paid() {
    let mut f = Fixture::new();
    mixed_costs(&mut f, 200, 8, 4, 1);
    let mut app = f.app();
    let factory = unit_id(&app, "factory");
    let job =
        production::queue_production(app.world_mut(), SlotId(0), factory, cid("train")).unwrap();
    app.update();
    app.update(); // first cost 2 credits +1 item
    assert_eq!(credits(&mut app), 998);
    assert_eq!(stock(&mut app, factory), 0);
    let progress = job_done(&app, factory);
    app.update();
    assert_eq!(credits(&mut app), 998);
    assert_eq!(job_done(&app, factory), progress);
    let wait = funding(&mut app, factory);
    assert_eq!(wait.waiting_for_funds, Some(cid("ore")));
    assert_eq!(
        wait.pending_charges.len(),
        0,
        "wait entry must flush previous debits"
    );
    production::cancel_production(app.world_mut(), SlotId(0), factory, job).unwrap();
    assert_eq!(credits(&mut app), 1000);
    assert_eq!(stock(&mut app, factory), 1);
    assert_eq!(
        production::cancel_production(app.world_mut(), SlotId(0), factory, job),
        Err(RejectReason::UnknownJob)
    );
}

#[test]
fn income_resumes_work_and_display_hold_does_not_veto_prediction() {
    let mut f = Fixture::new();
    f.ticks("train", 100);
    f.catalog.production.get_mut(&cid("train")).unwrap().costs[0].amount = 100;
    f.map.slot_economy[0].balances[0].amount = 0;
    f.unit("supply").income = Some(IncomeDef {
        resource: cid("credits").to_string(),
        amount: 100,
        period_ms: 500,
    });
    let mut app = f.app();
    let factory = unit_id(&app, "factory");
    queue(&mut app, factory, "train");
    for _ in 0..9 {
        app.update();
    }
    let wait = funding(&mut app, factory);
    assert_eq!(job_done(&app, factory), 0);
    assert_eq!(wait.waiting_for_funds, Some(cid("credits")));
    assert_eq!(wait.held_since_tick, Some(2));
    app.update();
    assert_eq!(job_done(&app, factory), 1);
    let active = funding(&mut app, factory);
    assert_eq!(active.waiting_for_funds, None);
    assert_eq!(active.held_since_tick, Some(2));
    assert_eq!(active.recovery_ticks, 1);
    let rows =
        production::queue_progress_rows(view(&app), &view(&app).economy.queues, view(&app).tick);
    assert!(rows[0].held);
    assert_eq!(rows[0].per_tick, 1);
    assert_eq!(
        rows[0].state,
        turts_sim::economy::events::RecordStateRow::Running
    );
    set_enabled(&mut app, factory, false);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(job_done(&app, factory), 1);
    assert_eq!(funding(&mut app, factory).recovery_ticks, 1);
    set_enabled(&mut app, factory, true);
    for _ in 0..18 {
        app.update();
    }
    assert_eq!(funding(&mut app, factory).held_since_tick, Some(2));
    app.update();
    assert_eq!(job_done(&app, factory), 20);
    assert_eq!(funding(&mut app, factory).held_since_tick, None);
}

#[test]
fn site_missing_retained_inventory_freezes_health_even_zero_difference() {
    let mut f = Fixture::new();
    mixed_costs(&mut f, 1000, 1, 1, 10);
    let mut app = f.app();
    let builder = unit_id(&app, "builder");
    let site = build(&mut app, builder, "build_relay", 30., 8.);
    app.update();
    assert_eq!(view(&app).construction.sites[&site].done, 1);
    let before = health(&app, site);
    remove(&mut app, builder);
    app.update();
    assert_eq!(view(&app).construction.sites[&site].done, 1);
    assert_eq!(health(&app, site), before);
    assert_eq!(credits(&mut app), 1000);
    assert!(
        view(&app)
            .operating
            .as_ref()
            .unwrap()
            .holds
            .iter()
            .any(|hold| hold.unit == site)
    );
    // Nothing was charged yet, so even the missing refund destination permits zero refund.
    construction::cancel_site(app.world_mut(), SlotId(0), site).unwrap();
}

#[test]
fn site_cancel_refunds_paid_rows_and_manual_disable_preserves_work() {
    let mut f = Fixture::new();
    mixed_costs(&mut f, 200, 8, 4, 10);
    let mut app = f.app();
    let builder = unit_id(&app, "builder");
    let site = build(&mut app, builder, "build_relay", 30., 8.);
    app.update();
    assert_eq!(credits(&mut app), 998);
    assert_eq!(stock(&mut app, builder), 9);
    set_enabled(&mut app, builder, false);
    app.update();
    assert_eq!(view(&app).construction.sites[&site].done, 1);
    assert_eq!(credits(&mut app), 998);
    construction::cancel_site(app.world_mut(), SlotId(0), site).unwrap();
    assert_eq!(credits(&mut app), 1000);
    assert_eq!(stock(&mut app, builder), 10);
    assert_eq!(
        construction::cancel_site(app.world_mut(), SlotId(0), site),
        Err(RejectReason::UnknownUnit)
    );
}

#[test]
fn stop_clears_only_held_requests_and_keeps_placed_site() {
    let mut f = Fixture::new();
    f.unit("builder")
        .constructor
        .as_mut()
        .unwrap()
        .concurrent_sites = 1;
    f.ticks("build_relay", 100);
    let mut app = f.app();
    let builder = unit_id(&app, "builder");
    let site = build(&mut app, builder, "build_relay", 30., 8.);
    assert_eq!(
        construction::construct(
            app.world_mut(),
            SlotId(0),
            builder,
            cid("build_relay"),
            Vec2::new(34., 8.)
        )
        .unwrap(),
        None
    );
    assert_eq!(live(&mut app).construction.held[&builder].len(), 1);
    construction::clear_held(app.world_mut(), SlotId(0), builder).unwrap();
    assert!(!live(&mut app).construction.held.contains_key(&builder));
    app.update();
    assert_eq!(view(&app).construction.sites[&site].done, 1);
    assert_eq!(credits(&mut app), 1000);
}

#[test]
fn final_admission_failure_does_not_take_last_site_payment_or_health() {
    let mut f = Fixture::new();
    f.unit("supply").provides = vec![amount("capacity", u64::MAX - 1)];
    f.unit("product").provides = vec![amount("capacity", 1)];
    f.unit("relay").provides = vec![amount("capacity", 1)];
    let mut app = f.app();
    let factory = unit_id(&app, "factory");
    let builder = unit_id(&app, "builder");
    let site = build(&mut app, builder, "build_relay", 30., 8.);
    queue(&mut app, factory, "train");
    app.update();
    assert_eq!(credits(&mut app), 995);
    let initial_health = health(&app, site);
    app.update();
    assert_eq!(credits(&mut app), 988);
    assert_eq!(view(&app).construction.sites[&site].done, 1);
    assert_eq!(health(&app, site), initial_health);
    assert_eq!(
        view(&app)
            .operating
            .as_ref()
            .unwrap()
            .holds
            .iter()
            .find(|hold| hold.unit == site)
            .unwrap()
            .reason,
        RejectReason::ArithmeticOverflow
    );
    let output = unit_id(&app, "product");
    remove(&mut app, output);
    app.update();
    assert!(!view(&app).construction.sites.contains_key(&site));
    assert_eq!(credits(&mut app), 982);
    app.update();
    assert_eq!(credits(&mut app), 982, "completed site cannot debit twice");
}

#[test]
fn geometric_output_wait_is_fully_paid_once_and_cancel_uses_blocked_ratio() {
    let mut f = Fixture::new();
    f.unit("product").footprint = Footprint::Circle { radius: 20. };
    let mut app = f.app();
    let factory = unit_id(&app, "factory");
    let job =
        production::queue_production(app.world_mut(), SlotId(0), factory, cid("train")).unwrap();
    app.update();
    app.update();
    assert_eq!(credits(&mut app), 993);
    assert_eq!(
        view(&app)
            .economy
            .queues
            .iter()
            .find(|row| row.producer == factory)
            .unwrap()
            .jobs[0]
            .stage,
        production::JobStage::OutputBlocked
    );
    for _ in 0..5 {
        app.update();
        assert_eq!(credits(&mut app), 993);
    }
    production::cancel_production(app.world_mut(), SlotId(0), factory, job).unwrap();
    assert_eq!(credits(&mut app), 1000);
}

#[test]
fn shared_physical_payment_order_ignores_creation_ids_and_preserves_fifo() {
    for build_factory in [false, true] {
        let mut f = Fixture::new();
        f.map.slot_economy[0].balances[0].amount = 1;
        for name in ["train", "train_b", "build_relay"] {
            f.catalog.production.get_mut(&cid(name)).unwrap().costs[0].amount = 1;
        }
        if build_factory {
            f.map
                .placements
                .retain(|row| row.unit != cid("factory").to_string());
            f.ticks("build_relay", 3);
            f.catalog
                .production
                .get_mut(&cid("build_factory"))
                .unwrap()
                .costs
                .clear();
        }
        let mut app = f.app();
        let builder = unit_id(&app, "builder");
        let site = build(&mut app, builder, "build_relay", 30., 8.);
        let factory = if build_factory {
            let id = build(&mut app, builder, "build_factory", 8., 8.);
            app.update();
            id
        } else {
            unit_id(&app, "factory")
        };
        assert_eq!(factory > site, build_factory);
        queue(&mut app, factory, "train");
        queue(&mut app, factory, "train_b");
        app.update();
        app.update();
        assert_eq!(credits(&mut app), 0);
        assert_eq!(
            view(&app)
                .units
                .iter()
                .filter(|row| row.content_id == cid("product").to_string())
                .count(),
            1
        );
        assert_eq!(
            view(&app)
                .units
                .iter()
                .filter(|row| row.content_id == cid("product_b").to_string())
                .count(),
            0
        );
        let waiting = &view(&app).construction.sites[&site];
        assert_eq!(waiting.done, waiting.total - 1);
        assert_eq!(
            waiting.funding.as_ref().unwrap().waiting_for_funds,
            Some(cid("credits"))
        );
    }
}

#[test]
fn missing_paid_refund_destination_refuses_manual_cancellation_atomically() {
    let mut f = Fixture::new();
    mixed_costs(&mut f, 200, 8, 4, 10);
    let mut app = f.app();
    let builder = unit_id(&app, "builder");
    let site = build(&mut app, builder, "build_relay", 30., 8.);
    app.update();
    assert_eq!(credits(&mut app), 998);
    let prior_health = health(&app, site);
    remove(&mut app, builder);
    let before = serde_json::to_value(live(&mut app).construction.sites[&site].clone()).unwrap();
    assert!(construction::cancel_site(app.world_mut(), SlotId(0), site).is_err());
    assert_eq!(credits(&mut app), 998);
    assert_eq!(
        serde_json::to_value(live(&mut app).construction.sites[&site].clone()).unwrap(),
        before
    );
    app.update();
    assert_eq!(health(&app, site), prior_health);
}
