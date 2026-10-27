mod common;

use common::{AuditLookup, archive_from_files, cid, economy_files, scenario};
use turts_pack::load_package;
use turts_sim::{
    Command, CommandKind, EconomyEventKind, RejectReason, SlotId, apply_command, audit_state,
    committed_tick,
};

fn fixture_with_amounts(timber: u64, token: u64, tick_hz: u16) -> bevy_app::App {
    let mut files = economy_files();
    let mut map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    map = map.replace("amount: 100)", &format!("amount: {timber})"));
    map = map.replace("amount: 200)", &format!("amount: {token})"));
    files.insert("maps/test.ron".into(), map.into_bytes());
    let mut app = scenario(tick_hz);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    app
}

fn produce(slot: u8, seq: u32, producer: u32, recipe: &str) -> Command {
    Command {
        slot: SlotId(slot),
        seq,
        kind: CommandKind::Produce {
            producer_id: producer,
            recipe_id: cid(recipe),
        },
    }
}

#[test]
fn reserve_last_stock_accepts_exactly_one_before_start() {
    let mut app = fixture_with_amounts(50, 200, 20);
    let first = apply_command(app.world_mut(), &produce(0, 1, 1, "probe:hire"));
    let second = apply_command(app.world_mut(), &produce(0, 2, 1, "probe:hire"));
    assert!(first.outcome.is_ok());
    assert_eq!(second.outcome, Err(RejectReason::InsufficientItems));
    let state = audit_state(app.world_mut());
    let timber = state
        .economy
        .inventories
        .iter()
        .find(|i| i.unit == 1)
        .unwrap()
        .items
        .iter()
        .find(|i| i.resource == cid("probe:timber"))
        .unwrap();
    assert_eq!((timber.on_hand, timber.reserved), (50, 50));
    let queued = &state
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0];
    assert_eq!(
        queued.reserved[0].source,
        turts_mod_api::CostSource::ProducerInventory
    );
    assert_eq!(queued.charged.len(), 0);
}

#[test]
fn charge_last_stock_accepts_exactly_one_without_reservations() {
    let mut app = fixture_with_amounts(100, 100, 20);
    let first = apply_command(app.world_mut(), &produce(0, 1, 1, "probe:train"));
    let second = apply_command(app.world_mut(), &produce(0, 2, 1, "probe:train"));
    assert!(first.outcome.is_ok());
    assert_eq!(second.outcome, Err(RejectReason::InsufficientBalance));
    let state = audit_state(app.world_mut());
    let token = state.balance(0, "probe:token");
    assert_eq!((token.on_hand, token.reserved), (0, 0));
    let queued = &state
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0];
    assert_eq!(
        queued.charged[0].source,
        turts_mod_api::CostSource::SlotBalance
    );
    assert_eq!(queued.reserved.len(), 0);
}

#[test]
fn mixed_cost_failure_is_atomic_and_local() {
    // Both cost sources use the same slot endpoint for different resources. The balance cost
    // succeeds in isolation, while the item cost fails; neither source may be partially changed.
    let mut files = economy_files();
    files.insert(
        "production/hire.ron".into(),
        b"(id: \"hire\", name: \"Mixed\", output_unit: \"probe:worker\", duration_ms: 1000, costs: [(resource: \"probe:token\", amount: 1, source: SlotBalance), (resource: \"probe:timber\", amount: 200, source: ProducerInventory)], payment: ChargeOnQueue, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))".to_vec(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let before = audit_state(app.world_mut());
    let result = apply_command(app.world_mut(), &produce(0, 1, 1, "probe:hire"));
    assert_eq!(result.outcome, Err(RejectReason::InsufficientItems));
    assert_eq!(audit_state(app.world_mut()), before);
    assert!(committed_tick(app.world()).events.is_empty());
}

#[test]
fn head_starts_at_zero_then_advances_and_blocks_fifo() {
    let mut app = fixture_with_amounts(100, 200, 10);
    let result = apply_command(app.world_mut(), &produce(0, 1, 1, "probe:hire"));
    assert!(result.outcome.is_ok());
    let job_id = result.outcome.unwrap().job_id.unwrap();
    let second = apply_command(app.world_mut(), &produce(0, 2, 1, "probe:hire"));
    assert!(second.outcome.is_ok());
    app.update();
    let state = audit_state(app.world_mut());
    let first = &state
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0];
    assert_eq!(first.id, job_id.0);
    assert_eq!(first.stage, turts_sim::JobStage::Running);
    assert_eq!(first.elapsed_ticks, 0);
    assert_eq!(
        first.charged[0].source,
        turts_mod_api::CostSource::ProducerInventory
    );
    assert!(
        committed_tick(app.world())
            .events
            .iter()
            .any(|event| matches!(
                event.kind,
                EconomyEventKind::ProductionStarted { producer: 1, job, .. } if job == job_id.0
            ))
    );
    app.update();
    let after = audit_state(app.world_mut());
    let first = &after
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0];
    assert_eq!(first.elapsed_ticks, 1);
    for _ in 0..59 {
        app.update();
    }
    let blocked = audit_state(app.world_mut());
    let jobs = &blocked
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs;
    assert_eq!(jobs[0].stage, turts_sim::JobStage::OutputBlocked);
    assert_eq!(jobs[0].elapsed_ticks, 60);
    assert_eq!(jobs[1].stage, turts_sim::JobStage::Queued);
    app.update();
    let still_blocked = audit_state(app.world_mut());
    let jobs = &still_blocked
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs;
    assert_eq!(jobs[0].stage, turts_sim::JobStage::OutputBlocked);
    assert_eq!(jobs[1].stage, turts_sim::JobStage::Queued);
}

#[test]
fn queue_validation_preserves_foreign_privacy() {
    let mut app = fixture_with_amounts(100, 200, 20);
    let before = audit_state(app.world_mut());
    for (slot, producer, recipe, reason) in [
        (0, 4, "probe:missing", RejectReason::NotOwner),
        (0, 99, "probe:hire", RejectReason::UnknownUnit),
        (0, 1, "probe:missing", RejectReason::UnknownRecipe),
        (0, 2, "probe:hire", RejectReason::MissingCapability),
    ] {
        let result = apply_command(app.world_mut(), &produce(slot, 3, producer, recipe));
        assert_eq!(result.outcome, Err(reason));
    }
    assert_eq!(audit_state(app.world_mut()), before);
}

#[test]
fn two_producers_cannot_spend_the_same_balance() {
    let mut files = economy_files();
    let mut map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    map = map.replace(
        ")], nodes:",
        "), (unit: \"probe:yard\", owner_slot: 0, pos: (x: 24.0, y: 24.0), contents: [])], nodes:",
    );
    map = map.replace("amount: 200)", "amount: 100)");
    files.insert("maps/test.ron".into(), map.into_bytes());
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let first = apply_command(app.world_mut(), &produce(0, 1, 1, "probe:train"));
    let second = apply_command(app.world_mut(), &produce(0, 2, 6, "probe:train"));
    assert!(first.outcome.is_ok());
    assert_eq!(second.outcome, Err(RejectReason::InsufficientBalance));
    let state = audit_state(app.world_mut());
    assert_eq!(
        (
            state.balance(0, "probe:token").on_hand,
            state.balance(0, "probe:token").reserved
        ),
        (0, 0)
    );
    assert_eq!(
        state
            .economy
            .queues
            .iter()
            .map(|q| q.jobs.len())
            .sum::<usize>(),
        1
    );
}

#[test]
fn failed_enqueue_does_not_allocate_job_or_consume_event_ordinal() {
    let mut app = fixture_with_amounts(50, 200, 20);
    app.world_mut()
        .resource_mut::<turts_sim::IdAllocators>()
        .set_next_values(6, 3, u64::MAX);
    let before = audit_state(app.world_mut());
    let event_ordinal = app.world().resource::<turts_sim::NextEventOrdinal>().0;
    let result = apply_command(app.world_mut(), &produce(0, 1, 1, "probe:hire"));
    assert_eq!(result.outcome, Err(RejectReason::IdExhausted));
    assert_eq!(audit_state(app.world_mut()), before);
    assert_eq!(
        app.world().resource::<turts_sim::NextEventOrdinal>().0,
        event_ordinal
    );

    app.world_mut()
        .resource_mut::<turts_sim::IdAllocators>()
        .set_next_values(6, 3, 1);
    app.world_mut()
        .resource_mut::<turts_sim::NextEventOrdinal>()
        .0 = None;
    let before = audit_state(app.world_mut());
    let result = apply_command(app.world_mut(), &produce(0, 2, 1, "probe:hire"));
    assert_eq!(result.outcome, Err(RejectReason::ArithmeticOverflow));
    assert_eq!(audit_state(app.world_mut()), before);
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .2,
        1
    );
}

#[test]
fn queue_capacity_counts_all_jobs() {
    let mut app = fixture_with_amounts(250, 200, 20);
    for seq in 1..=5 {
        assert!(
            apply_command(app.world_mut(), &produce(0, seq, 1, "probe:hire"))
                .outcome
                .is_ok()
        );
    }
    let before = audit_state(app.world_mut());
    let result = apply_command(app.world_mut(), &produce(0, 6, 1, "probe:hire"));
    assert_eq!(result.outcome, Err(RejectReason::QueueFull));
    assert_eq!(audit_state(app.world_mut()), before);
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .2,
        6
    );
}

#[test]
fn production_duration_uses_resolved_ticks_at_10_20_and_30_hz() {
    for tick_hz in [10, 20, 30] {
        let mut app = fixture_with_amounts(100, 200, tick_hz);
        let result = apply_command(app.world_mut(), &produce(0, 1, 1, "probe:hire"));
        let job_id = result.outcome.unwrap().job_id.unwrap();
        app.update(); // The head starts at elapsed zero.
        let duration = (6000_u64 * u64::from(tick_hz)).div_ceil(1000) as u32;
        for _ in 0..duration {
            app.update();
        }
        let job = &audit_state(app.world_mut()).economy.queues[0].jobs[0];
        assert_eq!(job.id, job_id.0);
        assert_eq!(job.duration_ticks, u64::from(duration));
        assert_eq!(job.elapsed_ticks, u64::from(duration));
        assert_eq!(job.stage, turts_sim::JobStage::OutputBlocked);
    }
}

#[test]
fn production_costs_do_not_pool_other_depots() {
    let mut files = economy_files();
    let mut map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    map = map.replace("amount: 100)", "amount: 1)");
    map = map.replace(
        "pos: (x: 8.0, y: 20.0), contents: []",
        "pos: (x: 8.0, y: 20.0), contents: [(item: \"probe:timber\", amount: 50)]",
    );
    files.insert("maps/test.ron".into(), map.into_bytes());
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    assert_eq!(
        apply_command(app.world_mut(), &produce(0, 1, 1, "probe:hire")).outcome,
        Err(RejectReason::InsufficientItems)
    );
    turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        2,
        1,
        cid("probe:timber"),
        50,
    )
    .unwrap();
    assert!(
        apply_command(app.world_mut(), &produce(0, 2, 1, "probe:hire"))
            .outcome
            .is_ok()
    );
    let state = audit_state(app.world_mut());
    let yard = state
        .economy
        .inventories
        .iter()
        .find(|i| i.unit == 1)
        .unwrap();
    let exchange = state
        .economy
        .inventories
        .iter()
        .find(|i| i.unit == 2)
        .unwrap();
    assert_eq!(yard.items[0].on_hand, 51);
    assert_eq!(yard.items[0].reserved, 50);
    assert_eq!(exchange.items[0].on_hand, 0);
}

#[test]
fn distinct_balance_resources_can_share_one_payment_endpoint() {
    let mut files = economy_files();
    files.insert(
        "resources/seal.ron".into(),
        b"(id: \"seal\", name: \"Seal\", unit_name: \"seals\", kind: Balance)".to_vec(),
    );
    let manifest = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    files.insert(
        "manifest.ron".into(),
        manifest
            .replace(
                "resources/token.ron\"]",
                "resources/token.ron\", \"resources/seal.ron\"]",
            )
            .into_bytes(),
    );
    files.insert(
        "production/train.ron".into(),
        b"(id: \"train\", name: \"Train Worker\", output_unit: \"probe:worker\", duration_ms: 5000, costs: [(resource: \"probe:token\", amount: 100, source: SlotBalance), (resource: \"probe:seal\", amount: 10, source: SlotBalance)], payment: ChargeOnQueue, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))".to_vec(),
    );
    let mut map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    map = map.replace(
        "(balance: \"probe:token\", amount: 200)",
        "(balance: \"probe:token\", amount: 200), (balance: \"probe:seal\", amount: 30)",
    );
    files.insert("maps/test.ron".into(), map.into_bytes());
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    assert!(
        apply_command(app.world_mut(), &produce(0, 1, 1, "probe:train"))
            .outcome
            .is_ok()
    );
    let state = audit_state(app.world_mut());
    assert_eq!(state.balance(0, "probe:token").on_hand, 100);
    assert_eq!(state.balance(0, "probe:seal").on_hand, 20);
}

#[test]
fn disabled_recipe_is_rejected_without_allocating_a_job() {
    let mut files = economy_files();
    let yard = String::from_utf8(files["units/yard.ron"].clone()).unwrap();
    files.insert(
        "units/yard.ron".into(),
        yard.replace("[\"probe:hire\", \"probe:train\"]", "[\"probe:hire\"]")
            .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let before = audit_state(app.world_mut());
    assert_eq!(
        apply_command(app.world_mut(), &produce(0, 1, 1, "probe:train")).outcome,
        Err(RejectReason::RecipeNotEnabled)
    );
    assert_eq!(audit_state(app.world_mut()), before);
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .2,
        1
    );
}

#[test]
fn queue_capacity_includes_output_blocked_head() {
    let mut files = economy_files();
    let yard = String::from_utf8(files["units/yard.ron"].clone()).unwrap();
    files.insert(
        "units/yard.ron".into(),
        yard.replace("queue_capacity: 5", "queue_capacity: 1")
            .into_bytes(),
    );
    let mut app = scenario(10);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    assert!(
        apply_command(app.world_mut(), &produce(0, 1, 1, "probe:hire"))
            .outcome
            .is_ok()
    );
    for _ in 0..61 {
        app.update();
    }
    let before = audit_state(app.world_mut());
    assert_eq!(
        before.economy.queues[0].jobs[0].stage,
        turts_sim::JobStage::OutputBlocked
    );
    assert_eq!(
        apply_command(app.world_mut(), &produce(0, 2, 1, "probe:hire")).outcome,
        Err(RejectReason::QueueFull)
    );
    assert_eq!(audit_state(app.world_mut()), before);
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .2,
        2
    );
}
