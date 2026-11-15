//! Independent audit of real paid-work and repair simulation streams.
use std::{collections::BTreeMap, path::Path};

use bevy_app::App;
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, SlotId, Vec2,
    economy::{construction, production},
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

use turts_audit::{EconomyLogRecord, validate_records};
use turts_sim::{AuditTrail, EconomyEventKind};
fn start(app: &mut App) -> Vec<EconomyLogRecord> {
    app.update();
    app.world_mut().resource_mut::<AuditTrail>().enable();
    vec![EconomyLogRecord::Initial(turts_sim::audit_state(
        app.world_mut(),
    ))]
}
fn ticks(app: &mut App, records: &mut Vec<EconomyLogRecord>, count: u32) {
    for _ in 0..count {
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        records.extend(
            trail
                .drain_commands()
                .into_iter()
                .map(EconomyLogRecord::CommandBoundary),
        );
        records.extend(
            trail
                .drain_operations()
                .into_iter()
                .map(EconomyLogRecord::Operation),
        );
        records.extend(
            trail
                .drain_ticks()
                .into_iter()
                .map(EconomyLogRecord::from_tick),
        );
    }
}
fn save(name: &str, records: &[EconomyLogRecord]) {
    if let Ok(path) = std::env::var("TURTS_PAID_AUDIT_OUT") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join(format!("{name}.jsonl")),
            records
                .iter()
                .map(|record| serde_json::to_string(record).unwrap() + "\n")
                .collect::<String>(),
        )
        .unwrap();
    }
}
fn assert_records(name: &str, records: &[EconomyLogRecord]) {
    save(name, records);
    validate_records(records).unwrap_or_else(|error| panic!("{name}: {error}"));
}
#[test]
fn cumulative_paid_producer_and_site_streams_and_coherent_tampering() {
    for hz in [10, 20] {
        for scaled in [false, true] {
            let mut fixture = Fixture::new();
            fixture.ticks("train", 8);
            fixture.ticks("build_relay", 8);
            mixed_costs(&mut fixture, 400, 7, 5, 100);
            if scaled {
                fixture.unit("factory").consumes = vec![amount("power", 10)];
                fixture
                    .unit("factory")
                    .producer
                    .as_mut()
                    .unwrap()
                    .when_short = vec![short("power", 2)];
                fixture
                    .unit("builder")
                    .constructor
                    .as_mut()
                    .unwrap()
                    .when_short = vec![short("capacity", 3)];
            }
            let mut app = fixture.app_at(hz);
            production::queue_production(app.world_mut(), SlotId(0), 1, cid("train")).unwrap();
            construction::construct(
                app.world_mut(),
                SlotId(0),
                2,
                cid("build_relay"),
                Vec2::new(40., 30.),
            )
            .unwrap();
            let mut records = start(&mut app);
            ticks(&mut app, &mut records, 40);
            assert_records(&format!("progress-{hz}-{scaled}"), &records);
            assert!(records.iter().any(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.events.iter().any(|event|matches!(event.kind,EconomyEventKind::ProgressCharged{..})))));
            let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.before.raw_queues.iter().flat_map(|queue|&queue.jobs).zip(operation.after.raw_queues.iter().flat_map(|queue|&queue.jobs)).any(|(a,b)|a.charged!=b.charged && !b.charged.is_empty()))).unwrap();
            let mut bad = records[..=index].to_vec();
            let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
                unreachable!()
            };
            let job = operation
                .after
                .raw_queues
                .iter_mut()
                .flat_map(|queue| &mut queue.jobs)
                .find(|job| !job.charged.is_empty())
                .unwrap();
            job.charged[0].amount += 1;
            let changed = job.id;
            job.funding.as_mut().unwrap().pending_charges[0].amount += 1;
            operation
                .after
                .economy
                .queues
                .iter_mut()
                .flat_map(|queue| &mut queue.jobs)
                .find(|job| job.id == changed)
                .unwrap()
                .charged[0]
                .amount += 1;
            operation
                .after
                .economy
                .operating
                .as_mut()
                .unwrap()
                .jobs
                .iter_mut()
                .find(|job| job.unit == 1 && job.job == changed)
                .unwrap()
                .funding
                .as_mut()
                .unwrap()
                .pending_charges[0]
                .amount += 1;
            if let Some(balance) = operation
                .after
                .economy
                .balances
                .iter_mut()
                .find(|row| row.resource == cid("credits"))
            {
                balance.on_hand -= 1;
            }
            assert!(
                validate_records(&bad)
                    .unwrap_err()
                    .contains("cumulative catalog floors")
            );
        }
    }
}
#[test]
fn cancellation_flushes_actual_paid_without_duplicating_refund() {
    let mut fixture = Fixture::new();
    fixture.ticks("train", 40);
    let mut app = fixture.app();
    let job = production::queue_production(app.world_mut(), SlotId(0), 1, cid("train")).unwrap();
    let mut records = start(&mut app);
    ticks(&mut app, &mut records, 10);
    app.world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        });
    ticks(&mut app, &mut records, 2);
    assert_records("cancel", &records);
    let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.events.iter().any(|event|matches!(&event.kind,EconomyEventKind::ProductionCancelled{refunded,..} if !refunded.is_empty())))).unwrap();
    let mut bad = records[..=index].to_vec();
    let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
        unreachable!()
    };
    let event = operation
        .events
        .iter_mut()
        .find(|event| matches!(event.kind, EconomyEventKind::ProductionCancelled { .. }))
        .unwrap();
    if let EconomyEventKind::ProductionCancelled { refunded, .. } = &mut event.kind {
        refunded[0].amount += 1;
    }
    operation
        .after
        .economy
        .balances
        .iter_mut()
        .find(|row| row.resource == cid("credits"))
        .unwrap()
        .on_hand += 1;
    let ordinal = event.ordinal;
    let event = event.clone();
    *operation
        .after
        .pending_events
        .iter_mut()
        .find(|row| row.ordinal == ordinal)
        .unwrap() = event;
    assert!(validate_records(&bad).unwrap_err().contains("refund"));
}
fn repair_fixture() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.unit("factory").producer = None;
    fixture.unit("factory").repairer = Some(RepairerDef {
        rate_per_second: 6,
        reach: 30.,
        cost: vec![RepairCost {
            resource: cid("credits").to_string(),
            amount: 3,
            per_health: 4,
        }],
        targets: TargetFilter {
            slots: SlotRelation::Same,
        },
        classes: vec![],
        auto_range: Some(30.),
    });
    fixture.unit("builder").constructor = None;
    fixture.map.placements[1].pos = WorldPos { x: 14., y: 8. };
    fixture.map.placements[2].owner_slot = 1;
    fixture.map.placements[2].pos = WorldPos { x: 24., y: 8. };
    fixture.unit("supply").weapon = Some(ResolvedWeapon {
        range: 20.,
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
    fixture.unit("factory").health = None;

    fixture
}

#[test]
fn fractional_repair_stream_preserves_debt_and_rejects_coherent_heal_tampering() {
    for hz in [10, 20] {
        let fixture = repair_fixture();
        let mut app = fixture.app_at(hz);
        let mut records = start(&mut app);
        ticks(&mut app, &mut records, 25);
        assert_records(&format!("repair-{hz}"), &records);
        let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.before.raw_healths.iter().zip(&operation.after.raw_healths).any(|(a,b)|b.current>a.current))).expect("real repair healed");
        let mut bad = records[..=index].to_vec();
        let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
            unreachable!()
        };
        let health = operation
            .after
            .raw_healths
            .iter_mut()
            .find(|row| row.unit == 2)
            .unwrap();
        health.current += 1;
        let mirror = operation
            .after
            .economy
            .m3
            .raw_healths
            .iter_mut()
            .find(|row| row.unit == 2)
            .unwrap();
        mirror.current += 1;
        assert!(validate_records(&bad).unwrap_err().contains("repair"));
        let mut bad = records[..=index].to_vec();
        let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
            unreachable!()
        };
        let row = operation
            .after
            .raw_repairs
            .iter_mut()
            .find(|row| !row.remainders.is_empty())
            .unwrap();
        row.remainders[0].remainder = (row.remainders[0].remainder + 1) % 4;
        assert!(
            validate_records(&bad)
                .unwrap_err()
                .contains("repair remainder")
        );
    }
}

#[test]
fn destruction_refund_wide_discard_stream_and_coherent_tampering() {
    let mut fixture = Fixture::new();
    fixture.ticks("train", 200);
    fixture.map.slot_economy[0].balances[0].amount = u64::MAX;
    let recipe = fixture.catalog.production.get_mut(&cid("train")).unwrap();
    recipe.payment = PaymentPolicy::ChargeOnQueue;
    recipe.costs[0].amount = u64::MAX;
    recipe.cancel_refunds.destroyed = RefundRatio::ONE;
    fixture.unit("supply").income = Some(IncomeDef {
        resource: cid("credits").to_string(),
        amount: u64::MAX,
        period_ms: 50,
    });
    let mut app = fixture.app();
    production::queue_production(app.world_mut(), SlotId(0), 1, cid("train")).unwrap();
    let mut records = start(&mut app);
    app.world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Produce {
                producer_id: 1,
                recipe_id: cid("train"),
            },
        });
    ticks(&mut app, &mut records, 1);
    app.world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::Remove { unit_id: 1 },
        });
    ticks(&mut app, &mut records, 2);
    assert_records("destruction-wide-discard", &records);
    let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.events.iter().any(|event|matches!(event.kind,EconomyEventKind::DestructionRefunded{..})))).expect("mandatory destruction refund event");
    let mut bad = records[..=index].to_vec();
    let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
        unreachable!()
    };
    let event = operation
        .events
        .iter_mut()
        .find(|event| matches!(event.kind, EconomyEventKind::DestructionRefunded { .. }))
        .unwrap();
    if let EconomyEventKind::DestructionRefunded {
        desired,
        credited,
        discarded,
        ..
    } = &mut event.kind
    {
        assert_eq!(
            desired
                .iter()
                .map(|row| u128::from(row.amount))
                .sum::<u128>(),
            u128::from(u64::MAX) * 2
        );
        assert!(credited.is_empty());
        assert_eq!(
            discarded
                .iter()
                .map(|row| u128::from(row.amount))
                .sum::<u128>(),
            u128::from(u64::MAX) * 2
        );
        discarded[0].amount -= 1;
    }
    let ordinal = event.ordinal;
    let event = event.clone();
    *operation
        .after
        .pending_events
        .iter_mut()
        .find(|row| row.ordinal == ordinal)
        .unwrap() = event;
    assert!(validate_records(&bad).unwrap_err().contains("destruction"));
}

#[test]
fn paid_wait_income_recovery_and_hold_hysteresis_are_independently_replayed() {
    let mut fixture = Fixture::new();
    fixture.ticks("train", 40);
    fixture.map.slot_economy[0].balances[0].amount = 1;
    fixture.unit("supply").income = Some(IncomeDef {
        resource: cid("credits").to_string(),
        amount: 7,
        period_ms: 1000,
    });
    let mut app = fixture.app();
    production::queue_production(app.world_mut(), SlotId(0), 1, cid("train")).unwrap();
    let mut records = start(&mut app);
    ticks(&mut app, &mut records, 60);
    assert_records("wait-income-recovery", &records);
    assert!(records.iter().any(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.after.raw_queues.iter().flat_map(|queue|&queue.jobs).any(|job|job.funding.as_ref().is_some_and(|funding|funding.waiting_for_funds.is_some())))));
    let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.after.raw_queues.iter().flat_map(|queue|&queue.jobs).any(|job|job.funding.as_ref().is_some_and(|funding|funding.waiting_for_funds.is_none() && funding.held_since_tick.is_some() && funding.recovery_ticks==1)))).expect("affordable progress while display hold remains true");
    let mut bad = records[..=index].to_vec();
    let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
        unreachable!()
    };
    let job = operation
        .after
        .raw_queues
        .iter_mut()
        .flat_map(|queue| &mut queue.jobs)
        .find(|job| {
            job.funding
                .as_ref()
                .is_some_and(|funding| funding.held_since_tick.is_some())
        })
        .unwrap();
    *job.funding
        .as_mut()
        .unwrap()
        .held_since_tick
        .as_mut()
        .unwrap() += 1;
    let changed = job.id;
    let mirror = operation
        .after
        .economy
        .operating
        .as_mut()
        .unwrap()
        .jobs
        .iter_mut()
        .find(|job| job.unit == 1 && job.job == changed)
        .unwrap();
    *mirror
        .funding
        .as_mut()
        .unwrap()
        .held_since_tick
        .as_mut()
        .unwrap() += 1;
    mirror.held_since_tick += 1;
    assert!(
        validate_records(&bad)
            .unwrap_err()
            .contains("hold/recovery")
    );
}

fn queue_order(app: &mut App, seq: u32, local: &str, target: turts_sim::OrderTargetSim) {
    let state = turts_sim::audit_state(app.world_mut());
    let definition = state
        .catalog
        .as_ref()
        .unwrap()
        .order_definitions
        .iter()
        .find(|row| row.id == cid(local))
        .unwrap();
    let policy = turts_sim::OrderPolicyAudit {
        id: definition.id.clone(),
        behaviour: definition.behaviour,
        target: definition.target,
        fire: definition.fire,
        engage: definition.engage,
        units: definition.units.clone(),
        queue_depth: definition.queue_depth,
        unit_capabilities: definition.unit_capabilities.clone(),
    };
    app.world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq,
            kind: CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: definition.id.clone(),
                behaviour: definition.behaviour,
                target_kind: definition.target,
                policy: Some(policy),
                allowed_units: definition.units.clone(),
                units: vec![1],
                target,
                queue: turts_sim::QueueMode::Replace,
                group: 0,
                queue_depth: definition.queue_depth,
            })),
        });
}
#[test]
fn explicit_stop_and_move_keep_repair_cost_debt_in_real_order_stream() {
    let mut fixture = repair_fixture();
    fixture.unit("factory").movement = Some(MovementDef { speed: 3. });
    for (local, behaviour, target) in [
        ("stop", Behaviour::Stop, TargetKind::None),
        ("move", Behaviour::Move, TargetKind::Point),
    ] {
        fixture.catalog.orders.insert(
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
        );
    }
    let mut app = fixture.app();
    let mut records = start(&mut app);
    ticks(&mut app, &mut records, 5);
    let before = turts_sim::audit_state(app.world_mut());
    let debt = before
        .raw_repairs
        .iter()
        .find(|row| row.repairer == 1)
        .unwrap()
        .remainders
        .clone();
    assert!(debt.iter().any(|row| row.remainder != 0));
    queue_order(&mut app, 1, "stop", turts_sim::OrderTargetSim::None);
    ticks(&mut app, &mut records, 3);
    let stopped = turts_sim::audit_state(app.world_mut());
    assert!(
        stopped
            .unit_orders
            .iter()
            .find(|row| row.unit_id == 1)
            .unwrap()
            .orders
            .repair_stopped
    );
    let source = stopped
        .raw_repairs
        .iter()
        .find(|row| row.repairer == 1)
        .unwrap();
    assert_eq!(source.remainders, debt);
    assert!(source.action.is_none());
    assert_eq!(before.raw_healths, stopped.raw_healths);
    queue_order(
        &mut app,
        2,
        "move",
        turts_sim::OrderTargetSim::Point(Vec2::new(8., 8.)),
    );
    ticks(&mut app, &mut records, 10);
    assert_records("repair-stop-move", &records);
    let resumed = turts_sim::audit_state(app.world_mut());
    assert!(
        !resumed
            .unit_orders
            .iter()
            .any(|row| row.unit_id == 1 && row.orders.repair_stopped)
    );
    assert!(
        resumed
            .raw_healths
            .iter()
            .find(|row| row.unit == 2)
            .unwrap()
            .current
            > stopped
                .raw_healths
                .iter()
                .find(|row| row.unit == 2)
                .unwrap()
                .current
    );
    let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.after.unit_orders.iter().any(|row|row.unit_id==1 && row.orders.repair_stopped))).unwrap();
    let mut bad = records[..=index].to_vec();
    let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
        unreachable!()
    };
    operation
        .after
        .unit_orders
        .iter_mut()
        .find(|row| row.unit_id == 1)
        .unwrap()
        .orders
        .repair_stopped = false;
    assert!(
        validate_records(&bad).is_err(),
        "coherent Stop latch corruption must fail independent order modeling"
    );
}

#[test]
fn automatic_retarget_keeps_fractional_cost_debt_between_real_damaged_targets() {
    let mut fixture = repair_fixture();
    fixture
        .unit("factory")
        .repairer
        .as_mut()
        .unwrap()
        .rate_per_second = 60;
    let mut enemy = fixture.unit("supply").clone();
    enemy.id = cid("supply_b");
    fixture.catalog.units.insert(enemy.id.clone(), enemy);
    fixture.map.placements.push(placement("product", 14., 16.));
    let mut enemy = placement("supply_b", 24., 16.);
    enemy.owner_slot = 1;
    fixture.map.placements.push(enemy);
    let mut app = fixture.app();
    let mut records = start(&mut app);
    ticks(&mut app, &mut records, 60);
    assert_records("repair-retarget", &records);
    let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.after.raw_repairs.iter().any(|row|row.repairer==1 && row.action.as_ref().is_some_and(|action|action.target==4 && action.elapsed==0)))).expect("automatic repair selects the second genuinely damaged target");
    let EconomyLogRecord::Operation(operation) = &records[index] else {
        unreachable!()
    };
    let before = operation
        .before
        .raw_repairs
        .iter()
        .find(|row| row.repairer == 1)
        .unwrap();
    let after = operation
        .after
        .raw_repairs
        .iter()
        .find(|row| row.repairer == 1)
        .unwrap();
    assert_eq!(before.remainders, after.remainders);
    assert!(after.remainders.iter().any(|row| row.remainder != 0));
    let mut bad = records[..=index].to_vec();
    let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
        unreachable!()
    };
    operation
        .after
        .raw_repairs
        .iter_mut()
        .find(|row| row.repairer == 1)
        .unwrap()
        .remainders
        .clear();
    assert!(
        validate_records(&bad)
            .unwrap_err()
            .contains("fractional cost debt")
    );
}

#[test]
fn paid_site_cancel_replays_original_balance_and_inventory_refunds() {
    use bevy::prelude::{IntoScheduleConfigs, World};
    let mut fixture = Fixture::new();
    mixed_costs(&mut fixture, 2000, 7, 5, 100);
    let mut app = fixture.app();
    construction::construct(
        app.world_mut(),
        SlotId(0),
        2,
        cid("build_relay"),
        Vec2::new(40., 30.),
    )
    .unwrap();
    let initial = turts_sim::audit_state(app.world_mut());
    let site = *initial.economy.construction.sites.keys().next().unwrap();
    app.add_systems(
        bevy_app::FixedUpdate,
        (move |world: &mut World| {
            if world.resource::<turts_sim::Tick>().0 == 22 {
                construction::cancel_site(world, SlotId(0), site).unwrap();
            }
        })
        .in_set(turts_sim::SimSet::Commands),
    );
    let mut records = start(&mut app);
    ticks(&mut app, &mut records, 24);
    assert_records("site-cancel-mixed-refund", &records);
    let final_state = turts_sim::audit_state(app.world_mut());
    assert!(
        !final_state
            .raw_units
            .iter()
            .any(|unit| unit.unit_id == site)
    );
    assert_eq!(
        final_state
            .economy
            .balances
            .iter()
            .find(|row| row.resource == cid("credits"))
            .unwrap()
            .on_hand,
        1000
    );
    assert_eq!(
        final_state
            .economy
            .inventories
            .iter()
            .find(|row| row.unit == 2)
            .unwrap()
            .items
            .iter()
            .find(|row| row.resource == cid("ore"))
            .unwrap()
            .on_hand,
        100
    );
}

#[test]
fn multi_cost_repair_wait_and_income_recovery_reject_wrong_missing_resource() {
    let mut fixture = repair_fixture();
    fixture.catalog.resources.insert(
        cid("gold"),
        ResourceDef {
            id: "gold".into(),
            name: "Gold".into(),
            unit_name: String::new(),
            kind: ResourceKind::Balance,
        },
    );
    fixture.map.slot_economy[0].balances.push(BalanceAmount {
        balance: cid("gold").to_string(),
        amount: 0,
    });
    fixture.unit("builder").income = Some(IncomeDef {
        resource: cid("gold").to_string(),
        amount: 1,
        period_ms: 1000,
    });
    fixture
        .unit("factory")
        .repairer
        .as_mut()
        .unwrap()
        .cost
        .push(RepairCost {
            resource: cid("gold").to_string(),
            amount: 1,
            per_health: 1,
        });
    let mut app = fixture.app();
    let mut records = start(&mut app);
    ticks(&mut app, &mut records, 45);
    assert_records("repair-wait-income", &records);
    let index=records.iter().position(|record|matches!(record,EconomyLogRecord::Operation(operation) if operation.before.raw_repairs!=operation.after.raw_repairs && operation.after.raw_repairs.iter().any(|row|row.action.as_ref().is_some_and(|action|matches!(action.state,turts_sim::economy::repair::RepairStatus::WaitingForFunds(_)))))).expect("real unaffordable repair waited");
    let mut bad = records[..=index].to_vec();
    let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
        unreachable!()
    };
    operation
        .after
        .raw_repairs
        .iter_mut()
        .find(|row| row.repairer == 1)
        .unwrap()
        .action
        .as_mut()
        .unwrap()
        .state = turts_sim::economy::repair::RepairStatus::WaitingForFunds(cid("credits"));
    operation
        .after
        .economy
        .repairs
        .iter_mut()
        .find(|row| row.repairer == 1)
        .unwrap()
        .state = turts_sim::economy::repair::RepairStatus::WaitingForFunds(cid("credits"));
    assert!(
        validate_records(&bad)
            .unwrap_err()
            .contains("first unaffordable")
    );
    let mut bad = records[..=index].to_vec();
    let EconomyLogRecord::Operation(operation) = bad.last_mut().unwrap() else {
        unreachable!()
    };
    operation
        .after
        .economy
        .balances
        .iter_mut()
        .find(|row| row.resource == cid("credits"))
        .unwrap()
        .on_hand -= 1;
    operation
        .after
        .raw_healths
        .iter_mut()
        .find(|row| row.unit == 2)
        .unwrap()
        .current += 1;
    operation
        .after
        .economy
        .m3
        .raw_healths
        .iter_mut()
        .find(|row| row.unit == 2)
        .unwrap()
        .current += 1;
    assert!(
        validate_records(&bad)
            .unwrap_err()
            .contains("without an eligible heal tick")
    );
}

#[test]
fn real_lethal_damage_refunds_only_balance_headroom_without_removal_permission() {
    let mut fixture = Fixture::new();
    fixture.ticks("train", 200);
    fixture.unit("factory").removal = None;
    fixture.map.slot_economy[0].balances[0].amount = u64::MAX;
    let recipe = fixture.catalog.production.get_mut(&cid("train")).unwrap();
    recipe.payment = PaymentPolicy::ChargeOnQueue;
    recipe.costs[0].amount = 10;
    recipe.cancel_refunds.destroyed = RefundRatio::ONE;
    fixture.unit("supply").income = Some(IncomeDef {
        resource: cid("credits").to_string(),
        amount: 8,
        period_ms: 50,
    });
    let mut enemy = bare(&fixture.catalog.units[&cid("supply")], "executioner", true);
    enemy.movement = Some(MovementDef { speed: 40. });
    enemy.weapon = Some(ResolvedWeapon {
        range: 4.,
        range_mode: WeaponRangeMode::Centre,
        cooldown_ms: 10000,
        damage: 100,
        ammo: None,
        targets: TargetFilter {
            slots: SlotRelation::Other,
        },
        damage_type: None,
        bonus: vec![],
        when_short: vec![],
    });
    fixture.catalog.units.insert(enemy.id.clone(), enemy);
    let mut enemy = placement("executioner", 24., 8.);
    enemy.owner_slot = 1;
    fixture.map.placements.push(enemy);
    let mut app = fixture.app();
    production::queue_production(app.world_mut(), SlotId(0), 1, cid("train")).unwrap();
    let mut records = start(&mut app);
    app.world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(1),
            seq: 1,
            kind: CommandKind::MoveTo {
                unit_ids: vec![4],
                target: Vec2::new(8., 8.),
            },
        });
    ticks(&mut app, &mut records, 20);
    assert_records("lethal-balance-headroom", &records);
    let refund = records
        .iter()
        .find_map(|record| {
            if let EconomyLogRecord::Operation(operation) = record {
                operation.events.iter().find_map(|event| {
                    if let EconomyEventKind::DestructionRefunded {
                        unit: 1,
                        desired,
                        credited,
                        discarded,
                    } = &event.kind
                    {
                        Some((desired, credited, discarded))
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .expect("real weapon death emits mandatory refund accounting");
    assert_eq!(refund.0.iter().map(|row| row.amount).sum::<u64>(), 10);
    assert_eq!(refund.1.iter().map(|row| row.amount).sum::<u64>(), 2);
    assert_eq!(refund.2.iter().map(|row| row.amount).sum::<u64>(), 8);
    assert!(
        !turts_sim::audit_state(app.world_mut())
            .raw_units
            .iter()
            .any(|unit| unit.unit_id == 1)
    );
}
