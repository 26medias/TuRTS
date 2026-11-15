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
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let mut catalog = archive.content().unwrap();
    let recipe = catalog.production.get_mut(&id("hire")).unwrap();
    recipe.payment = PaymentPolicy::ChargeOverTime;
    recipe.duration_ms = duration_ms;
    recipe.costs[0].amount = 12;
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
