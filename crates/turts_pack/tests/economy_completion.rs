mod common;
use common::{
    AuditLookup, advance, archive_from_files, cid, economy_archive, economy_files, scenario,
};
use turts_pack::load_package;
use turts_sim::{
    Command, CommandKind, CommandQueue, EconomyEventKind, JobId, SlotId, apply_command,
    audit_state, committed_tick,
};

fn command(seq: u32, producer: u32, recipe: &str) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Produce {
            producer_id: producer,
            recipe_id: cid(recipe),
        },
    }
}

#[test]
fn completion_spawns_once_after_exact_duration() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let job = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    advance(&mut app, 120);
    assert_eq!(audit_state(app.world_mut()).units.len(), 5);
    app.update();
    let after = audit_state(app.world_mut());
    assert_eq!(after.units.len(), 6);
    assert_eq!(
        committed_tick(app.world()).spawned[0].pos,
        bevy_math::Vec2::new(8.0, 12.0)
    );
    assert!(
        after
            .units
            .iter()
            .any(|u| u.content_id == "probe:worker" && u.owner_slot == 0)
    );
    assert_eq!(committed_tick(app.world()).spawned.len(), 1);
    let spawned_id = committed_tick(app.world()).spawned[0].unit_id;
    let spawned_inventory = after
        .economy
        .inventories
        .iter()
        .find(|inventory| inventory.unit == spawned_id)
        .unwrap();
    assert_eq!(
        spawned_inventory
            .items
            .iter()
            .map(|item| item.on_hand)
            .sum::<u64>(),
        0
    );
    let spawned_order = after
        .economy
        .orders
        .iter()
        .find(|order| order.unit == spawned_id)
        .unwrap();
    assert_eq!(spawned_order.stage, turts_sim::GatherStage::Idle);
    assert!(committed_tick(app.world()).events.iter().any(|e| matches!(e.kind, EconomyEventKind::ProductionCompleted { producer: 1, job: id, .. } if id == job.0)));
    app.update();
    assert_eq!(audit_state(app.world_mut()).units.len(), 6);
    assert_eq!(committed_tick(app.world()).spawned.len(), 0);
    let after_completion = audit_state(app.world_mut());
    let rejected = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        },
    );
    assert_eq!(rejected.outcome, Err(turts_sim::RejectReason::UnknownJob));
    assert_eq!(audit_state(app.world_mut()), after_completion);
}

#[test]
fn queued_cancellation_releases_reservation_without_refund() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let job = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        },
    );
    assert!(result.outcome.is_ok());
    let state = audit_state(app.world_mut());
    let repeated = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        },
    );
    assert_eq!(repeated.outcome, Err(turts_sim::RejectReason::UnknownJob));
    assert_eq!(audit_state(app.world_mut()), state);
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
    assert_eq!((timber.on_hand, timber.reserved), (100, 0));
    assert!(
        state
            .economy
            .queues
            .iter()
            .find(|q| q.producer == 1)
            .unwrap()
            .jobs
            .is_empty()
    );
    app.update();
    assert!(committed_tick(app.world()).events.iter().any(
        |e| matches!(e.kind, EconomyEventKind::ProductionCancelled { job: id, .. } if id == job.0)
    ));
}

#[test]
fn running_cancellation_refunds_each_policy_by_floor() {
    for (recipe, resource, expected) in [
        ("probe:hire", "probe:timber", 75_u64),
        ("probe:train", "probe:token", 150_u64),
    ] {
        let mut files = economy_files();
        if recipe == "probe:hire" {
            let def = String::from_utf8(files["production/hire.ron"].clone()).unwrap();
            files.insert(
                "production/hire.ron".into(),
                def.replace(
                    "running: (numerator: 1, denominator: 1)",
                    "running: (numerator: 1, denominator: 2)",
                )
                .into_bytes(),
            );
        } else {
            let def = String::from_utf8(files["production/train.ron"].clone()).unwrap();
            files.insert(
                "production/train.ron".into(),
                def.replace(
                    "running: (numerator: 1, denominator: 1)",
                    "running: (numerator: 1, denominator: 2)",
                )
                .into_bytes(),
            );
        }
        let mut app = scenario(20);
        load_package(app.world_mut(), &archive_from_files(files)).unwrap();
        let job = apply_command(app.world_mut(), &command(1, 1, recipe))
            .outcome
            .unwrap()
            .job_id
            .unwrap();
        app.update(); // Consume a reservation for hire or retain ChargeOnQueue payment for train.
        let cancelled = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 2,
                kind: CommandKind::CancelProduction {
                    producer_id: 1,
                    job_id: job,
                },
            },
        );
        assert!(cancelled.outcome.is_ok());
        app.update();
        let state = audit_state(app.world_mut());
        if resource == "probe:timber" {
            let item = state
                .economy
                .inventories
                .iter()
                .find(|i| i.unit == 1)
                .unwrap()
                .items
                .iter()
                .find(|i| i.resource == cid(resource))
                .unwrap();
            assert_eq!((item.on_hand, item.reserved), (expected, 0));
        } else {
            assert_eq!(state.balance(0, resource).on_hand, expected);
        }
        assert!(
            state
                .economy
                .queues
                .iter()
                .find(|q| q.producer == 1)
                .unwrap()
                .jobs
                .is_empty()
        );
        assert!(committed_tick(app.world()).events.iter().any(|event| matches!(event.kind, EconomyEventKind::ProductionCancelled { job: id, .. } if id == job.0)));
    }
}

#[test]
fn output_blocked_cancellation_refunds_both_policies() {
    for (recipe, resource, expected, duration_ticks) in [
        ("probe:hire", "probe:timber", 75_u64, 121_u32),
        ("probe:train", "probe:token", 150_u64, 101_u32),
    ] {
        let mut files = economy_files();
        let yard = String::from_utf8(files["units/yard.ron"].clone()).unwrap();
        files.insert(
            "units/yard.ron".into(),
            yard.replace(
                "(x: 0.0, y: 4.0), (x: 0.0, y: -4.0)",
                "(x: 100.0, y: 100.0), (x: -100.0, y: -100.0)",
            )
            .into_bytes(),
        );
        if recipe == "probe:hire" {
            let def = String::from_utf8(files["production/hire.ron"].clone()).unwrap();
            files.insert(
                "production/hire.ron".into(),
                def.replace(
                    "output_blocked: (numerator: 1, denominator: 1)",
                    "output_blocked: (numerator: 1, denominator: 2)",
                )
                .into_bytes(),
            );
        } else {
            let def = String::from_utf8(files["production/train.ron"].clone()).unwrap();
            files.insert(
                "production/train.ron".into(),
                def.replace(
                    "output_blocked: (numerator: 1, denominator: 1)",
                    "output_blocked: (numerator: 1, denominator: 2)",
                )
                .into_bytes(),
            );
        }
        let mut app = scenario(20);
        load_package(app.world_mut(), &archive_from_files(files)).unwrap();
        let job = apply_command(app.world_mut(), &command(1, 1, recipe))
            .outcome
            .unwrap()
            .job_id
            .unwrap();
        advance(&mut app, duration_ticks);
        assert_eq!(
            audit_state(app.world_mut()).economy.queues[0].jobs[0].stage,
            turts_sim::JobStage::OutputBlocked
        );
        let cancelled = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 2,
                kind: CommandKind::CancelProduction {
                    producer_id: 1,
                    job_id: job,
                },
            },
        );
        assert!(cancelled.outcome.is_ok());
        app.update();
        let state = audit_state(app.world_mut());
        if resource == "probe:timber" {
            let item = state
                .economy
                .inventories
                .iter()
                .find(|i| i.unit == 1)
                .unwrap()
                .items
                .iter()
                .find(|i| i.resource == cid(resource))
                .unwrap();
            assert_eq!((item.on_hand, item.reserved), (expected, 0));
        } else {
            assert_eq!(state.balance(0, resource).on_hand, expected);
        }
        assert_eq!(state.units.len(), 5);
    }
}

#[test]
fn blocked_retry_spawns_once_without_recharge_and_starts_successor_next_tick() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let first = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let second = apply_command(app.world_mut(), &command(2, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    app.update();
    let set_pos = |world: &mut bevy_ecs::world::World, id: u32, point: bevy_math::Vec2| {
        let entity = world
            .iter_entities()
            .find(|e| {
                e.get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == id)
            })
            .unwrap()
            .id();
        world.entity_mut(entity).insert(turts_sim::Position(point));
    };
    // Block both declared offsets with existing units.
    set_pos(app.world_mut(), 3, bevy_math::Vec2::new(8.0, 12.0));
    set_pos(app.world_mut(), 5, bevy_math::Vec2::new(8.0, 4.0));
    advance(&mut app, 120);
    assert_eq!(
        audit_state(app.world_mut()).economy.queues[0].jobs[0].stage,
        turts_sim::JobStage::OutputBlocked
    );
    let blocked = audit_state(app.world_mut());
    let charged = blocked.economy.queues[0].jobs[0].charged.clone();
    let next_unit = app
        .world()
        .resource::<turts_sim::IdAllocators>()
        .next_values()
        .0;
    set_pos(app.world_mut(), 3, bevy_math::Vec2::new(12.0, 8.0));
    set_pos(app.world_mut(), 5, bevy_math::Vec2::new(52.0, 8.0));
    app.update();
    let after = audit_state(app.world_mut());
    assert_eq!(after.units.len(), 6);
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .0,
        next_unit + 1
    );
    assert_eq!(after.economy.queues[0].jobs.len(), 1);
    assert_eq!(after.economy.queues[0].jobs[0].id, second.0);
    assert_eq!(
        after.economy.queues[0].jobs[0].stage,
        turts_sim::JobStage::Queued
    );
    assert_eq!(
        after.balance(0, "probe:token").on_hand,
        blocked.balance(0, "probe:token").on_hand
    );
    assert!(committed_tick(app.world()).events.iter().any(|event| matches!(event.kind, EconomyEventKind::ProductionCompleted { job, .. } if job == first.0)));
    assert!(!committed_tick(app.world()).events.iter().any(|event| matches!(event.kind, EconomyEventKind::ProductionStarted { job, .. } if job == second.0)));
    assert_eq!(charged[0].amount, 50);
    app.update();
    assert_eq!(
        audit_state(app.world_mut()).economy.queues[0].jobs[0].stage,
        turts_sim::JobStage::Running
    );
}

#[test]
fn same_batch_charge_on_queue_produce_then_cancel_refunds_queued_job() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    app.world_mut().resource_mut::<CommandQueue>().0.extend([
        command(1, 1, "probe:train"),
        Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: JobId(1),
            },
        },
    ]);
    app.update();
    let state = audit_state(app.world_mut());
    assert_eq!(state.balance(0, "probe:token").on_hand, 200);
    assert!(
        state
            .economy
            .queues
            .iter()
            .find(|q| q.producer == 1)
            .unwrap()
            .jobs
            .is_empty()
    );
    let events = &committed_tick(app.world()).events;
    assert!(matches!(
        events[0].kind,
        EconomyEventKind::ProductionQueued { job: 1, .. }
    ));
    assert!(matches!(
        events[1].kind,
        EconomyEventKind::ProductionCancelled { job: 1, .. }
    ));
}

#[test]
fn full_refund_destination_rejects_atomically_then_retry_succeeds() {
    let mut files = economy_files();
    let yard = String::from_utf8(files["units/yard.ron"].clone()).unwrap();
    files.insert(
        "units/yard.ron".into(),
        yard.replace("capacity: 2000", "capacity: 100").into_bytes(),
    );
    let worker = String::from_utf8(files["units/worker.ron"].clone()).unwrap();
    files.insert(
        "units/worker.ron".into(),
        worker.replace("capacity: 10", "capacity: 100").into_bytes(),
    );
    let map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    files.insert(
        "maps/test.ron".into(),
        map.replace(
            "pos: (x: 12.0, y: 8.0), contents: []",
            "pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 50)]",
        )
        .into_bytes(),
    );
    let recipe = String::from_utf8(files["production/hire.ron"].clone()).unwrap();
    files.insert(
        "production/hire.ron".into(),
        recipe
            .replace(
                "running: (numerator: 1, denominator: 1)",
                "running: (numerator: 1, denominator: 2)",
            )
            .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let job = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    app.update();
    turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        3,
        1,
        cid("probe:timber"),
        50,
    )
    .unwrap();
    let before_reject = audit_state(app.world_mut());
    let pending_before = app
        .world()
        .resource::<turts_sim::economy::events::PendingEconomyEvents>()
        .0
        .clone();
    let ordinal_before = app
        .world()
        .resource::<turts_sim::economy::events::NextEventOrdinal>()
        .0;
    let rejected = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        },
    );
    assert_eq!(
        rejected.outcome,
        Err(turts_sim::RejectReason::RefundDestinationFull)
    );
    assert_eq!(audit_state(app.world_mut()), before_reject);
    assert_eq!(
        app.world()
            .resource::<turts_sim::economy::events::PendingEconomyEvents>()
            .0,
        pending_before
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::economy::events::NextEventOrdinal>()
            .0,
        ordinal_before
    );
    app.update();
    let progressed = audit_state(app.world_mut());
    let progressed_job = &progressed.economy.queues[0].jobs[0];
    assert_eq!(progressed_job.stage, turts_sim::JobStage::Running);
    assert_eq!(
        progressed_job.elapsed_ticks,
        before_reject.economy.queues[0].jobs[0].elapsed_ticks + 1
    );
    assert_eq!(
        progressed_job.charged,
        before_reject.economy.queues[0].jobs[0].charged
    );
    turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        1,
        3,
        cid("probe:timber"),
        25,
    )
    .unwrap();
    let accepted = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        },
    );
    assert!(accepted.outcome.is_ok());
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
    assert_eq!((timber.on_hand, timber.reserved), (100, 0));
}

#[test]
fn checked_shared_unit_id_exhaustion_keeps_blocked_job_and_charge() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let job = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    app.update();
    let set_pos = |world: &mut bevy_ecs::world::World, id: u32, point: bevy_math::Vec2| {
        let entity = world
            .iter_entities()
            .find(|e| {
                e.get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == id)
            })
            .unwrap()
            .id();
        world.entity_mut(entity).insert(turts_sim::Position(point));
    };
    set_pos(app.world_mut(), 3, bevy_math::Vec2::new(8.0, 12.0));
    set_pos(app.world_mut(), 5, bevy_math::Vec2::new(8.0, 4.0));
    advance(&mut app, 120);
    let before = audit_state(app.world_mut());
    let (next_node, next_job) = {
        let values = app
            .world()
            .resource::<turts_sim::IdAllocators>()
            .next_values();
        (values.1, values.2)
    };
    app.world_mut()
        .resource_mut::<turts_sim::IdAllocators>()
        .set_next_values(u32::MAX, next_node, next_job);
    set_pos(app.world_mut(), 3, bevy_math::Vec2::new(12.0, 8.0));
    set_pos(app.world_mut(), 5, bevy_math::Vec2::new(52.0, 8.0));
    app.update();
    assert_eq!(audit_state(app.world_mut()).units.len(), 5);
    assert_eq!(
        audit_state(app.world_mut()).economy.queues[0].jobs[0].stage,
        turts_sim::JobStage::OutputBlocked
    );
    assert_eq!(
        audit_state(app.world_mut()).economy.queues[0].jobs[0].charged,
        before.economy.queues[0].jobs[0].charged
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .0,
        u32::MAX
    );
    assert!(!committed_tick(app.world()).events.iter().any(|event| matches!(event.kind, EconomyEventKind::ProductionCompleted { job: id, .. } if id == job.0)));
}

#[test]
fn two_producers_completing_together_get_ordered_shared_ids() {
    let mut files = economy_files();
    let map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    files.insert(
        "maps/test.ron".into(),
        map.replace(
            "(slot: 1, balances: [])",
            "(slot: 1, balances: [(balance: \"probe:token\", amount: 100)])",
        )
        .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let first = apply_command(app.world_mut(), &command(1, 1, "probe:train"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let second = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(1),
            seq: 1,
            kind: CommandKind::Produce {
                producer_id: 4,
                recipe_id: cid("probe:train"),
            },
        },
    )
    .outcome
    .unwrap()
    .job_id
    .unwrap();
    advance(&mut app, 101);
    let spawned = &committed_tick(app.world()).spawned;
    assert_eq!(spawned.len(), 2);
    assert_eq!((spawned[0].unit_id, spawned[0].owner_slot), (6, 0));
    assert_eq!((spawned[1].unit_id, spawned[1].owner_slot), (7, 1));
    let completed: Vec<_> = committed_tick(app.world())
        .events
        .iter()
        .filter_map(|event| {
            if let EconomyEventKind::ProductionCompleted {
                producer,
                job,
                output_unit_id,
            } = event.kind
            {
                Some((producer, job, output_unit_id))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(completed, vec![(1, first.0, 6), (4, second.0, 7)]);
}

#[test]
fn output_checks_ordered_offsets_nodes_and_allows_tangency() {
    let mut files = economy_files();
    let yard = String::from_utf8(files["units/yard.ron"].clone()).unwrap();
    files.insert(
        "units/yard.ron".into(),
        yard.replace(
            "(x: 0.0, y: 4.0), (x: 0.0, y: -4.0)",
            "(x: 16.0, y: 0.0), (x: 0.0, y: 2.5)",
        )
        .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap();
    advance(&mut app, 121);
    let spawned = &committed_tick(app.world()).spawned;
    assert_eq!(spawned.len(), 1);
    // The first offset overlaps the live node at (24, 8); the second is exactly tangent
    // to the producer's radius and must be accepted.
    assert_eq!(spawned[0].pos, bevy_math::Vec2::new(8.0, 10.5));
}

#[test]
fn non_head_and_same_batch_reserve_then_charge_cancellations_release_only_target_reservation() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let head = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let waiting = apply_command(app.world_mut(), &command(2, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: waiting,
            },
        },
    );
    assert!(result.outcome.is_ok());
    let state = audit_state(app.world_mut());
    let jobs = &state
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, head.0);
    assert_eq!(jobs[0].reserved[0].amount, 50);
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
    assert_eq!((timber.on_hand, timber.reserved), (100, 50));

    let mut same_batch = scenario(20);
    load_package(same_batch.world_mut(), &economy_archive()).unwrap();
    same_batch
        .world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .extend([
            command(1, 1, "probe:hire"),
            Command {
                slot: SlotId(0),
                seq: 2,
                kind: CommandKind::CancelProduction {
                    producer_id: 1,
                    job_id: JobId(1),
                },
            },
        ]);
    same_batch.update();
    let state = audit_state(same_batch.world_mut());
    assert!(
        state
            .economy
            .queues
            .iter()
            .find(|q| q.producer == 1)
            .unwrap()
            .jobs
            .is_empty()
    );
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
    assert_eq!((timber.on_hand, timber.reserved), (100, 0));
}

#[test]
fn odd_floor_and_queued_nonhead_charge_on_queue() {
    let mut files = economy_files();
    let train = String::from_utf8(files["production/train.ron"].clone()).unwrap();
    files.insert(
        "production/train.ron".into(),
        train
            .replace("amount: 100", "amount: 3")
            .replace(
                "queued: (numerator: 1, denominator: 1)",
                "queued: (numerator: 1, denominator: 2)",
            )
            .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let first = apply_command(app.world_mut(), &command(1, 1, "probe:train"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let second = apply_command(app.world_mut(), &command(2, 1, "probe:train"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: second,
            },
        },
    );
    assert!(result.outcome.is_ok());
    let state = audit_state(app.world_mut());
    assert_eq!(state.balance(0, "probe:token").on_hand, 195);
    assert_eq!(state.economy.queues[0].jobs[0].id, first.0);
    let event = app
        .world()
        .resource::<turts_sim::economy::events::PendingEconomyEvents>()
        .0
        .last()
        .unwrap();
    match &event.kind {
        EconomyEventKind::ProductionCancelled {
            released, refunded, ..
        } => {
            assert!(released.is_empty());
            assert_eq!(
                (refunded[0].source, refunded[0].amount),
                (turts_sim::EventSource::Slot(0), 1)
            );
        }
        _ => panic!("expected cancellation event"),
    }
}

#[test]
fn cancel_just_before_completion_prevents_spawn() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let job = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    advance(&mut app, 120);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        });
    app.update();
    assert_eq!(audit_state(app.world_mut()).units.len(), 5);
    assert!(committed_tick(app.world()).spawned.is_empty());
    assert!(committed_tick(app.world()).events.iter().any(|event| matches!(event.kind, EconomyEventKind::ProductionCancelled { job: id, .. } if id == job.0)));
}

#[test]
fn earlier_producer_spawn_blocks_shared_candidate() {
    let mut files = economy_files();
    let map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    files.insert(
        "maps/test.ron".into(),
        map.replace("pos: (x: 56.0, y: 8.0)", "pos: (x: 8.0, y: 16.0)")
            .replace(
                "(slot: 1, balances: [])",
                "(slot: 1, balances: [(balance: \"probe:token\", amount: 100)])",
            )
            .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    apply_command(app.world_mut(), &command(1, 1, "probe:train"))
        .outcome
        .unwrap();
    apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(1),
            seq: 1,
            kind: CommandKind::Produce {
                producer_id: 4,
                recipe_id: cid("probe:train"),
            },
        },
    )
    .outcome
    .unwrap();
    advance(&mut app, 101);
    assert_eq!(committed_tick(app.world()).spawned.len(), 1);
    assert_eq!(
        committed_tick(app.world()).spawned[0].pos,
        bevy_math::Vec2::new(8.0, 12.0)
    );
    let state = audit_state(app.world_mut());
    assert_eq!(
        state
            .economy
            .queues
            .iter()
            .find(|q| q.producer == 4)
            .unwrap()
            .jobs[0]
            .stage,
        turts_sim::JobStage::OutputBlocked
    );
}

#[test]
fn full_circle_crossing_boundary_blocks_first_offset() {
    let mut files = economy_files();
    let yard = String::from_utf8(files["units/yard.ron"].clone()).unwrap();
    files.insert(
        "units/yard.ron".into(),
        yard.replace(
            "(x: 0.0, y: 4.0), (x: 0.0, y: -4.0)",
            "(x: -7.75, y: 0.0), (x: 0.0, y: -4.0)",
        )
        .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    apply_command(app.world_mut(), &command(1, 1, "probe:train"))
        .outcome
        .unwrap();
    advance(&mut app, 101);
    assert_eq!(
        committed_tick(app.world()).spawned[0].pos,
        bevy_math::Vec2::new(8.0, 4.0)
    );
}

#[test]
fn event_staging_failure_restores_queued_cancel_claims_and_stock() {
    for recipe in ["probe:hire", "probe:train"] {
        let mut app = scenario(20);
        load_package(app.world_mut(), &economy_archive()).unwrap();
        let job = apply_command(app.world_mut(), &command(1, 1, recipe))
            .outcome
            .unwrap()
            .job_id
            .unwrap();
        app.world_mut()
            .resource_mut::<turts_sim::economy::events::NextEventOrdinal>()
            .0 = Some(u64::MAX);
        let before = audit_state(app.world_mut());
        let pending = app
            .world()
            .resource::<turts_sim::economy::events::PendingEconomyEvents>()
            .clone();
        assert_eq!(
            apply_command(
                app.world_mut(),
                &Command {
                    slot: SlotId(0),
                    seq: 2,
                    kind: CommandKind::CancelProduction {
                        producer_id: 1,
                        job_id: job
                    }
                }
            )
            .outcome,
            Err(turts_sim::RejectReason::ArithmeticOverflow)
        );
        assert_eq!(audit_state(app.world_mut()), before);
        assert_eq!(
            *app.world()
                .resource::<turts_sim::economy::events::PendingEconomyEvents>(),
            pending
        );
        assert_eq!(
            app.world()
                .resource::<turts_sim::economy::events::NextEventOrdinal>()
                .0,
            Some(u64::MAX)
        );
    }
}

#[test]
fn converted_income_can_overflow_charge_on_queue_refund_without_partial_writes() {
    let mut files = economy_files();
    let worker = String::from_utf8(files["units/worker.ron"].clone()).unwrap();
    files.insert(
        "units/worker.ron".into(),
        worker
            .replace("capacity: 10", "capacity: 1")
            .replace("unload_per_second: 10", "unload_per_second: 20")
            .into_bytes(),
    );
    let exchange = String::from_utf8(files["units/exchange.ron"].clone()).unwrap();
    files.insert(
        "units/exchange.ron".into(),
        exchange.replace("per_item: 2", "per_item: 4").into_bytes(),
    );
    let map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    files.insert(
        "maps/test.ron".into(),
        map.replace(
            "unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: []",
            "unit: \"probe:worker\", owner_slot: 0, pos: (x: 8.0, y: 17.0), contents: [(item: \"probe:timber\", amount: 1)]",
        )
        .replace(
            "balance: \"probe:token\", amount: 200",
            &format!("balance: \"probe:token\", amount: {}", u64::MAX - 2),
        )
        .into_bytes(),
    );
    let recipe = String::from_utf8(files["production/train.ron"].clone()).unwrap();
    files.insert(
        "production/train.ron".into(),
        recipe
            .replace(
                "resource: \"probe:token\", amount: 100, source: SlotBalance",
                "resource: \"probe:token\", amount: 2, source: SlotBalance",
            )
            .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    assert!(
        apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::Gather {
                    unit_id: 3,
                    node_id: turts_sim::NodeId(1),
                },
            },
        )
        .outcome
        .is_ok()
    );
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(command(2, 1, "probe:train"));
    app.update();
    let first_job = app.world().resource::<turts_sim::CommandResults>().0[0]
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    assert!(committed_tick(app.world()).events.iter().any(
        |event| matches!(event.kind, EconomyEventKind::ProductionStarted { job, .. } if job == first_job.0)
    ));
    advance(&mut app, 2);
    assert!(
        committed_tick(app.world())
            .events
            .iter()
            .any(|event| matches!(
                event.kind,
                EconomyEventKind::Converted {
                    amount: 1,
                    credited: 4,
                    ..
                }
            ))
    );
    assert_eq!(
        audit_state(app.world_mut())
            .balance(0, "probe:token")
            .on_hand,
        u64::MAX
    );

    let before = audit_state(app.world_mut());
    let pending_before = app
        .world()
        .resource::<turts_sim::economy::events::PendingEconomyEvents>()
        .0
        .clone();
    let ordinal_before = app
        .world()
        .resource::<turts_sim::economy::events::NextEventOrdinal>()
        .0;
    let job_id_before = app
        .world()
        .resource::<turts_sim::IdAllocators>()
        .next_values()
        .2;
    let rejected = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: first_job,
            },
        },
    );
    assert_eq!(
        rejected.outcome,
        Err(turts_sim::RejectReason::RefundDestinationOverflow)
    );
    assert_eq!(audit_state(app.world_mut()), before);
    assert_eq!(
        app.world()
            .resource::<turts_sim::economy::events::PendingEconomyEvents>()
            .0,
        pending_before
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::economy::events::NextEventOrdinal>()
            .0,
        ordinal_before
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .2,
        job_id_before
    );

    let second = apply_command(app.world_mut(), &command(4, 1, "probe:train"));
    let second_job = second.outcome.unwrap().job_id.unwrap();
    assert_ne!(first_job, second_job);
    assert_eq!(
        audit_state(app.world_mut())
            .balance(0, "probe:token")
            .on_hand,
        u64::MAX - 2
    );
    assert!(
        apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 5,
                kind: CommandKind::CancelProduction {
                    producer_id: 1,
                    job_id: first_job,
                },
            },
        )
        .outcome
        .is_ok()
    );
    let after_retry = audit_state(app.world_mut());
    assert_eq!(after_retry.balance(0, "probe:token").on_hand, u64::MAX);
    let remaining_jobs = &after_retry
        .economy
        .queues
        .iter()
        .find(|queue| queue.producer == 1)
        .unwrap()
        .jobs;
    assert_eq!(remaining_jobs.len(), 1);
    assert_eq!(remaining_jobs[0].id, second_job.0);
}

#[test]
fn gathering_depletion_unlocks_one_blocked_output_with_empty_typed_capabilities() {
    let mut files = economy_files();
    let yard = String::from_utf8(files["units/yard.ron"].clone()).unwrap();
    files.insert(
        "units/yard.ron".into(),
        yard.replace("(x: 0.0, y: 4.0), (x: 0.0, y: -4.0)", "(x: 16.0, y: 0.0)")
            .into_bytes(),
    );
    let map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    files.insert(
        "maps/test.ron".into(),
        map.replacen("remaining: 1000", "remaining: 2", 1)
            .replace(
                "unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: []",
                "unit: \"probe:worker\", owner_slot: 0, pos: (x: 22.0, y: 8.0), contents: []",
            )
            .into_bytes(),
    );
    let worker = String::from_utf8(files["units/worker.ron"].clone()).unwrap();
    files.insert(
        "units/worker.ron".into(),
        worker
            .replace("gather_per_second: 5", "gather_per_second: 20")
            .into_bytes(),
    );
    let recipe = String::from_utf8(files["production/hire.ron"].clone()).unwrap();
    files.insert(
        "production/hire.ron".into(),
        recipe
            .replace("duration_ms: 6000", "duration_ms: 50")
            .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    assert!(
        apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::Gather {
                    unit_id: 3,
                    node_id: turts_sim::NodeId(1),
                },
            },
        )
        .outcome
        .is_ok()
    );
    let initial_next_unit = app
        .world()
        .resource::<turts_sim::IdAllocators>()
        .next_values()
        .0;
    let job = apply_command(app.world_mut(), &command(2, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();

    app.update();
    assert_eq!(
        audit_state(app.world_mut()).economy.queues[0].jobs[0].duration_ticks,
        1
    );
    app.update();
    let blocked = audit_state(app.world_mut());
    assert_eq!(
        blocked
            .nodes
            .iter()
            .find(|node| node.id.0 == 1)
            .unwrap()
            .remaining,
        1
    );
    assert_eq!(
        blocked.economy.queues[0].jobs[0].stage,
        turts_sim::JobStage::OutputBlocked
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .0,
        initial_next_unit
    );
    assert_eq!(
        blocked
            .economy
            .inventories
            .iter()
            .find(|inventory| inventory.unit == 1)
            .unwrap()
            .items[0]
            .on_hand,
        50
    );
    assert!(!committed_tick(app.world()).events.iter().any(|event| matches!(event.kind, EconomyEventKind::ProductionCompleted { job: id, .. } if id == job.0)));

    app.update();

    let completed = audit_state(app.world_mut());
    assert_eq!(
        completed
            .nodes
            .iter()
            .find(|node| node.id.0 == 1)
            .unwrap()
            .remaining,
        0
    );
    assert_eq!(completed.units.len(), 6);
    assert_eq!(
        app.world()
            .resource::<turts_sim::IdAllocators>()
            .next_values()
            .0,
        initial_next_unit + 1
    );
    assert_eq!(
        completed
            .economy
            .inventories
            .iter()
            .find(|inventory| inventory.unit == initial_next_unit)
            .unwrap()
            .items,
        Vec::<turts_sim::ItemView>::new()
    );
    let order = completed
        .economy
        .orders
        .iter()
        .find(|order| order.unit == initial_next_unit)
        .unwrap();
    assert_eq!(order.stage, turts_sim::GatherStage::Idle);
    assert_eq!((order.node, order.receiver), (None, None));
    assert_eq!(completed.economy.queues[0].jobs.len(), 0);
    assert_eq!(
        completed
            .economy
            .inventories
            .iter()
            .find(|inventory| inventory.unit == 1)
            .unwrap()
            .items[0]
            .on_hand,
        50
    );
    assert_eq!(committed_tick(app.world()).spawned.len(), 1);
    assert_eq!(
        committed_tick(app.world()).spawned[0].unit_id,
        initial_next_unit
    );
    assert_eq!(committed_tick(app.world()).events.iter().filter(|event| matches!(event.kind, EconomyEventKind::ProductionCompleted { job: id, .. } if id == job.0)).count(), 1);
}
