//! Ordinary fixed-tick commands, results and immutable accounting use the real package path.
use super::*;
use turts_bot::{BotOrder, TaskKey};
use turts_mod_api::{PaymentPolicy, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

fn id(local: &str) -> turts_mod_api::ContentId {
    turts_mod_api::ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn actor() -> BotActor {
    BotActor {
        slot: 0,
        profile: id("policy"),
    }
}
fn fixture(duration_ms: u64) -> App {
    fixture_with(duration_ms, |_| {})
}
fn fixture_with(duration_ms: u64, prepare: impl FnOnce(&mut turts_mod_api::ContentCatalog)) -> App {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let mut catalog = archive.content().unwrap();
    let recipe = catalog.production.get_mut(&id("hire")).unwrap();
    recipe.payment = PaymentPolicy::ChargeOverTime;
    recipe.duration_ms = duration_ms;
    recipe.costs[0].amount = 12;
    prepare(&mut catalog);
    let map = archive.maps().unwrap().remove(0).1;
    let mut app = turts_sim::headless_app(20);
    let staged = turts_sim::stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: "actors/fixture.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    turts_sim::install_package(app.world_mut(), staged);
    app.insert_resource(crate::orders::ServerCatalog(catalog));
    app.insert_resource(crate::session::Advertised {
        lockfile_hash: "0".repeat(64),
        tick_hz: 20,
    });
    app.init_resource::<crate::session::RateLimitBuckets>();
    turts_sim::initialize_completed_world(app.world_mut());
    let prepared =
        prepare_actor_limits(app.world(), &[actor()], CoordinatorLimits::default(), 65536).unwrap();
    install_bot_actors(app.world_mut(), &[actor()]).unwrap();
    configure_actor_limits(app.world_mut(), prepared);
    install(&mut app);
    app
}
fn command(sequence: u32, units: Vec<u32>, order: &str, target: BotTarget) -> EmittedCommand {
    EmittedCommand {
        actor: actor(),
        sequence,
        task: TaskKey::new("economy", "opening", format!("request-{sequence}")),
        units,
        order: BotOrder {
            content: id(order),
            target,
            queue: BotQueueMode::Replace,
            group: sequence,
        },
    }
}
fn queued(app: &App) -> ReconcileBatch {
    actor_reconciliation(app.world(), &actor()).unwrap()
}

#[test]
fn ordinary_group_refusal_and_actual_job_ids_reach_separate_member_receipts() {
    let mut app = fixture(600);
    stage_bot_command(
        app.world_mut(),
        command(
            1,
            vec![1, 4],
            "order_produce",
            BotTarget::Content(id("hire")),
        ),
    )
    .unwrap();
    app.update();
    let batch = queued(&app);
    assert_eq!(batch.watermark.completed_tick, 1);
    assert_eq!(batch.included.len(), 2);
    let own = batch
        .included
        .iter()
        .find(|effect| effect.request.unit == 1)
        .unwrap();
    let foreign = batch
        .included
        .iter()
        .find(|effect| effect.request.unit == 4)
        .unwrap();
    assert!(!own.refused);
    assert!(foreign.refused && foreign.terminal);
    let work = own.work.as_ref().unwrap();
    let actual = &turts_sim::completed_world(app.world())
        .queues
        .iter()
        .find(|queue| queue.producer == 1)
        .unwrap()
        .jobs[0];
    assert_eq!(work.identity.id, actual.id);
    assert_eq!(work.identity.unit, 1);
    assert_eq!(work.payment, Some(PaymentPolicy::ChargeOverTime));
    assert!(foreign.work.is_none() && foreign.paid.is_empty());
    assert!(
        actor_results(app.world(), &actor())
            .iter()
            .find(|result| result.request.unit == 4)
            .unwrap()
            .reason
            .as_ref()
            .unwrap()
            .contains("NotOwner")
    );
    assert!(app.world().resource::<BotActors>().error.is_none());
}
#[test]
fn same_tick_production_completion_has_exact_terminal_payment_then_ack_releases_tracking() {
    let mut app = fixture(50);
    stage_bot_command(
        app.world_mut(),
        command(1, vec![1], "order_produce", BotTarget::Content(id("hire"))),
    )
    .unwrap();
    app.update(); // Queued becomes Running.
    assert!(!queued(&app).included[0].terminal);
    app.update(); // The one work step completes and removes the queue head.
    let batch = queued(&app);
    let effect = &batch.included[0];
    assert!(effect.terminal);
    assert_eq!(
        effect.work.as_ref().unwrap().state,
        turts_bot::WorkState::Finished
    );
    assert_eq!(effect.paid.iter().map(|row| row.amount).sum::<u64>(), 12);
    assert_eq!(
        effect.effect_watermark,
        Watermark {
            completed_tick: 1,
            revision: 1
        }
    );
    assert!(
        turts_sim::completed_world(app.world())
            .queues
            .iter()
            .filter(|queue| queue.producer == 1)
            .all(|queue| queue.jobs.is_empty())
    );
    let balance = batch
        .accounts
        .iter()
        .find(|row| {
            row.account
                == turts_bot::SpendKey {
                    endpoint: turts_bot::SourceEndpoint::Inventory {
                        unit: 1,
                        port: turts_bot::InventoryPort::Main,
                    },
                    resource: id("wood"),
                }
        })
        .unwrap();
    assert_eq!(balance.on_hand, 88);
    acknowledge(app.world_mut(), &actor(), batch.watermark, &[]).unwrap();
    assert!(
        app.world().resource::<BotActors>().entries[&0]
            .requests
            .is_empty()
    );
    app.update();
    assert!(queued(&app).included.is_empty());
}
#[test]
fn unknown_order_and_shared_rate_refusals_do_not_mutate_the_simulation() {
    let mut app = fixture(600);
    stage_bot_command(
        app.world_mut(),
        command(1, vec![2], "missing_order", BotTarget::None),
    )
    .unwrap();
    app.update();
    let batch = queued(&app);
    assert!(batch.included[0].refused);
    assert!(
        app.world()
            .resource::<turts_sim::CommandResults>()
            .0
            .is_empty()
    );
    acknowledge(app.world_mut(), &actor(), batch.watermark, &[]).unwrap();
    assert_eq!(
        stage_bot_command(
            app.world_mut(),
            command(1, vec![2], "order_stop", BotTarget::None)
        ),
        Err(CommandRejectReason::BadSequence)
    );
    // Spend the ordinary slot bucket for the upcoming tick. No gameplay state is changed.
    for _ in 0..64 {
        assert!(
            app.world_mut()
                .resource_mut::<crate::session::RateLimitBuckets>()
                .admit(0, 2, 20, 1)
        );
    }
    stage_bot_command(
        app.world_mut(),
        command(2, vec![2], "order_stop", BotTarget::None),
    )
    .unwrap();
    app.update();
    let result = actor_results(app.world(), &actor())
        .into_iter()
        .find(|result| result.request.sequence == 2)
        .unwrap();
    assert!(!result.accepted && result.reason.unwrap().contains("RateLimited"));
    assert!(
        app.world()
            .resource::<turts_sim::CommandResults>()
            .0
            .is_empty()
    );
    assert!(app.world().resource::<BotActors>().error.is_none());
}
#[test]
fn a_legal_large_group_is_not_confused_with_the_pending_envelope_limit() {
    let mut app = fixture(600);
    // These are intentionally unknown subjects. Ordinary validation must refuse
    // each one; the internal bridge must retain the complete 128-member envelope.
    stage_bot_command(
        app.world_mut(),
        command(1, (100..228).collect(), "order_stop", BotTarget::None),
    )
    .unwrap();
    app.update();
    let batch = queued(&app);
    assert_eq!(batch.included.len(), 128);
    assert!(batch.included.iter().all(|effect| effect.refused));
    assert!(app.world().resource::<BotActors>().error.is_none());
}
#[test]
fn initial_account_rows_and_invalid_setup_are_prepared_before_permanent_claims() {
    let app = fixture(600);
    assert_eq!(queued(&app).watermark.completed_tick, 0);
    assert!(queued(&app).accounts.iter().any(|row| row.on_hand == 100));
    let before = app.world().resource::<Slots>().owners();
    assert!(
        prepare_actor_limits(app.world(), &[actor()], CoordinatorLimits::default(), 1).is_err()
    );
    assert_eq!(app.world().resource::<Slots>().owners(), before);
    assert_eq!(before[0], Some(u64::MAX));
}

#[test]
fn a_real_weapon_kill_after_the_first_cot_step_keeps_its_exact_terminal_debit() {
    use turts_mod_api::{
        Behaviour, EngagePolicy, FirePolicy, HealthDef, ResolvedWeapon, SlotRelation, TargetFilter,
        TargetKind, WeaponRangeMode,
    };
    let mut app = fixture_with(600, |catalog| {
        catalog.units.get_mut(&id("depot")).unwrap().health = Some(HealthDef { max: 1 });
        catalog.units.get_mut(&id("worker")).unwrap().weapon = Some(ResolvedWeapon {
            range: 120.0,
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
        let mut attack = catalog.orders[&id("order_stop")].clone();
        attack.id = "order_attack".into();
        attack.behaviour = Behaviour::Attack;
        attack.target = TargetKind::Unit;
        attack.fire = Some(FirePolicy::InRange);
        attack.engage = Some(EngagePolicy::None);
        attack.units = vec![];
        catalog.orders.insert(id("order_attack"), attack);
    });
    stage_bot_command(
        app.world_mut(),
        command(1, vec![1], "order_produce", BotTarget::Content(id("hire"))),
    )
    .unwrap();
    app.update(); // The actual production queue enters Running with no paid work yet.
    assert_eq!(queued(&app).included[0].work.as_ref().unwrap().done, 0);
    let attack = PlayerCommand {
        seq: 99,
        order: OrderEnvelope {
            order: id("order_attack").to_string(),
            units: vec![turts_net_protocol::UnitId(5)],
            target: OrderTarget::Unit(turts_net_protocol::UnitId(1)),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        },
    };
    let commands = crate::orders::to_sim(
        SlotId(1),
        &attack,
        &app.world().resource::<crate::orders::ServerCatalog>().0,
    )
    .unwrap();
    app.world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .extend(commands);
    app.update(); // Production pays one twelfth; Combat then kills the producer.
    let batch = queued(&app);
    let effect = &batch.included[0];
    let work = effect.work.as_ref().unwrap();
    assert!(effect.terminal && !effect.refused);
    assert_eq!((work.done, work.total), (1, 12));
    assert_eq!(work.state, turts_bot::WorkState::Failed);
    assert_eq!(effect.paid.iter().map(|row| row.amount).sum::<u64>(), 1);
    assert_eq!(
        effect.paid[0].account.endpoint,
        turts_bot::SourceEndpoint::Inventory {
            unit: 1,
            port: turts_bot::InventoryPort::Main
        }
    );
    assert!(
        !turts_sim::completed_world(app.world())
            .units
            .iter()
            .any(|unit| unit.unit_id == 1)
    );
    assert!(!batch.accounts.iter().any(|row| matches!(
        row.account.endpoint,
        turts_bot::SourceEndpoint::Inventory { unit: 1, .. }
    )));
    let terminal = &turts_sim::committed_tick(app.world()).terminal_work[0];
    assert_eq!(
        (terminal.unit, terminal.job, terminal.done, terminal.total),
        (1, work.identity.id, 1, 12)
    );
    assert_eq!(
        terminal.state,
        turts_sim::receipts::TerminalWorkState::Destroyed
    );
    assert!(
        turts_sim::committed_tick(app.world())
            .events
            .iter()
            .any(|event| matches!(
                event.kind,
                turts_sim::EconomyEventKind::Destroyed { unit: 1, .. }
            ))
    );
    assert!(app.world().resource::<BotActors>().error.is_none());
}

#[test]
fn an_automatic_held_site_can_place_and_complete_before_its_first_publication() {
    use turts_mod_api::{
        Behaviour, ConstructProgress, HealthDef, Ratio, ResolvedConstructor, SitePlacementDef,
        TargetKind,
    };
    let mut app = fixture_with(600, |catalog| {
        let mut building = catalog.units[&id("worker")].clone();
        building.id = id("building");
        building.movement = None;
        building.inventory = None;
        building.gatherer = None;
        building.health = Some(HealthDef { max: 100 });
        building.placement = Some(SitePlacementDef {
            gap: 0.0,
            node_clearance: 0.0,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 4,
            },
        });
        catalog.units.insert(id("building"), building);
        for (name, duration) in [("build_slow", 100), ("build_fast", 50)] {
            let mut recipe = catalog.production[&id("hire")].clone();
            recipe.id = id(name);
            recipe.output_unit = id("building");
            recipe.duration_ms = duration;
            catalog.production.insert(id(name), recipe);
        }
        let builder = catalog.units.get_mut(&id("depot")).unwrap();
        builder.constructor = Some(ResolvedConstructor {
            buildings: vec![id("build_slow"), id("build_fast")],
            concurrent_sites: 1,
            queue_capacity: 8,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
        let mut order = catalog.orders[&id("order_produce")].clone();
        order.id = "order_build".into();
        order.behaviour = Behaviour::Construct;
        order.target = TargetKind::ContentAtPoint;
        catalog.orders.insert(id("order_build"), order);
    });
    app.insert_resource(turts_sim::economy::construction::ConstructionLimit(8));
    stage_bot_command(
        app.world_mut(),
        command(
            1,
            vec![1],
            "order_build",
            BotTarget::ContentAtPoint {
                content: id("build_slow"),
                point: turts_mod_api::WorldPos { x: 20.0, y: 20.0 },
            },
        ),
    )
    .unwrap();
    app.update();
    let first = queued(&app);
    assert_eq!(first.included[0].work.as_ref().unwrap().done, 1);
    stage_bot_command(
        app.world_mut(),
        command(
            2,
            vec![1],
            "order_build",
            BotTarget::ContentAtPoint {
                content: id("build_fast"),
                point: turts_mod_api::WorldPos { x: 28.0, y: 20.0 },
            },
        ),
    )
    .unwrap();
    app.update(); // The second order is held; the first site completes later this tick.
    let held = queued(&app);
    let effect = held
        .included
        .iter()
        .find(|effect| effect.request.sequence == 2)
        .unwrap();
    assert!(effect.pending_placement && !effect.terminal && effect.work.is_none());
    assert!(effect.paid.is_empty() && effect.reserved.is_empty());
    assert_eq!(
        turts_sim::completed_world(app.world()).construction.held[&1].len(),
        1
    );
    app.update(); // Auto placement precedes Commands; its single work step then completes.
    let completed = queued(&app);
    let effect = completed
        .included
        .iter()
        .find(|effect| effect.request.sequence == 2)
        .unwrap();
    let work = effect.work.as_ref().unwrap();
    assert!(effect.terminal && !effect.pending_placement);
    assert_eq!(work.identity.kind, WorkKind::Construction);
    assert_eq!(work.actor_unit, 1);
    assert_eq!((work.done, work.total), (1, 1));
    assert_eq!(work.state, turts_bot::WorkState::Finished);
    assert_eq!(
        effect.effect_watermark,
        Watermark {
            completed_tick: 2,
            revision: 2
        }
    );
    assert_eq!(effect.paid.iter().map(|row| row.amount).sum::<u64>(), 12);
    assert!(
        turts_sim::completed_world(app.world())
            .construction
            .sites
            .is_empty()
    );
    assert!(
        turts_sim::completed_world(app.world())
            .construction
            .held
            .is_empty()
    );
    let terminal = &turts_sim::committed_tick(app.world()).terminal_work[0];
    assert!(terminal.construction);
    assert_eq!(
        (terminal.unit, terminal.job),
        (work.identity.unit, work.identity.id)
    );
    let amount = completed
        .accounts
        .iter()
        .find(|row| {
            row.account
                == turts_bot::SpendKey {
                    endpoint: turts_bot::SourceEndpoint::Inventory {
                        unit: 1,
                        port: turts_bot::InventoryPort::Main,
                    },
                    resource: id("wood"),
                }
        })
        .unwrap();
    assert_eq!(amount.on_hand, 76);
    assert!(app.world().resource::<BotActors>().error.is_none());
}
