mod common;

use common::{
    accounting::{AccountingOracle, AutomaticPhase, AutomaticSubject, ExpectedOperation},
    economy_archive, economy_files, scenario, scenario_with,
};
use turts_pack::load_package;
use turts_sim::{
    AuditState, AuditTrail, Command, CommandKind, EconomyEvent, EconomyEventKind, JobId, SlotId,
    apply_command, audit_state,
};

fn produce(seq: u32, recipe: &str) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Produce {
            producer_id: 1,
            recipe_id: common::cid(recipe),
        },
    }
}
fn trail(app: &mut bevy_app::App) -> &mut AuditTrail {
    app.world_mut().resource_mut::<AuditTrail>().into_inner()
}

fn canonical_operations_jsonl(operations: &[turts_sim::OperationAudit]) -> String {
    let mut sorted = operations.to_vec();
    sorted.sort_by_key(|record| record.operation_id);
    let mut out = String::new();
    for record in sorted {
        out.push_str(&serde_json::to_string(&record).unwrap());
        out.push('\n');
    }
    out
}

fn advance_accounted_tick(app: &mut bevy_app::App, oracle: &mut AccountingOracle, tick: u32) {
    app.update();
    let operations = trail(app).drain_operations();
    let mut cursor = 0usize;
    let starting = oracle.modeled_state().clone();
    for order in &starting.raw_orders {
        let op = operations
            .get(cursor)
            .unwrap_or_else(|| panic!("missing gather operation for {} on tick {tick}", order.unit))
            .clone();
        cursor += 1;
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(order.unit),
                },
                &op.before,
                &op.after,
                &op.events,
                None,
            )
            .unwrap_or_else(|e| panic!("gatherer {} tick {tick}: {e}", order.unit));
    }
    for queue in &starting.economy.queues {
        let Some(job) = queue.jobs.first() else {
            continue;
        };
        let mut current = oracle.modeled_state().clone();
        let mut head = current
            .economy
            .queues
            .iter()
            .find(|q| q.producer == queue.producer)
            .and_then(|q| q.jobs.first())
            .cloned();
        if job.stage != turts_sim::JobStage::OutputBlocked {
            let op = operations
                .get(cursor)
                .unwrap_or_else(|| {
                    panic!(
                        "missing production advance for {} tick {tick}",
                        queue.producer
                    )
                })
                .clone();
            cursor += 1;
            oracle
                .observe(
                    &ExpectedOperation::Automatic {
                        tick,
                        phase: AutomaticPhase::ProductionAdvance,
                        subject: AutomaticSubject::HeadJob {
                            producer: queue.producer,
                            job: job.id,
                        },
                    },
                    &op.before,
                    &op.after,
                    &op.events,
                    None,
                )
                .unwrap_or_else(|e| panic!("production {} tick {tick}: {e}", queue.producer));
            current = oracle.modeled_state().clone();
            head = current
                .economy
                .queues
                .iter()
                .find(|q| q.producer == queue.producer)
                .and_then(|q| q.jobs.first())
                .cloned();
        }
        if head
            .as_ref()
            .is_some_and(|j| j.stage == turts_sim::JobStage::OutputBlocked)
        {
            let id = head.unwrap().id;
            let op = operations
                .get(cursor)
                .unwrap_or_else(|| {
                    panic!("missing output attempt for {} tick {tick}", queue.producer)
                })
                .clone();
            cursor += 1;
            oracle
                .observe(
                    &ExpectedOperation::Automatic {
                        tick,
                        phase: AutomaticPhase::OutputAttempt,
                        subject: AutomaticSubject::HeadJob {
                            producer: queue.producer,
                            job: id,
                        },
                    },
                    &op.before,
                    &op.after,
                    &op.events,
                    None,
                )
                .unwrap_or_else(|e| panic!("output {} tick {tick}: {e}", queue.producer));
        }
    }
    assert_eq!(
        cursor,
        operations.len(),
        "unmodeled operation records at tick {tick}"
    );
    let operation_ids = operations
        .iter()
        .map(|op| op.operation_id)
        .collect::<Vec<_>>();
    assert!(
        operation_ids.windows(2).all(|w| w[0] < w[1]),
        "logical operation IDs are unique and ordered"
    );
    let jsonl = canonical_operations_jsonl(&operations);
    let mut decoded = Vec::new();
    for line in jsonl.lines() {
        decoded.push(serde_json::from_str::<turts_sim::OperationAudit>(line).unwrap());
    }
    let mut sorted = operations.clone();
    sorted.sort_by_key(|op| op.operation_id);
    assert_eq!(
        decoded, sorted,
        "canonical sorted JSONL roundtrips complete owned operations"
    );
    let tick_record = trail(app)
        .drain_ticks()
        .pop()
        .expect("one TickAudit per completed tick");
    let tick_json = serde_json::to_vec(&tick_record).unwrap();
    let tick_roundtrip: turts_sim::TickAudit = serde_json::from_slice(&tick_json).unwrap();
    assert_eq!(
        tick_roundtrip, tick_record,
        "completed tick records retain full before/after snapshots and events"
    );
    let mut published = tick_record.before.pending_events.clone();
    for op in &operations {
        published.extend(op.events.clone());
    }
    published.sort_by_key(|e| e.ordinal);
    assert_eq!(
        published, tick_record.events,
        "published TickAudit events equal command and operation records"
    );
    oracle
        .observe(
            &ExpectedOperation::TickBoundary { tick },
            &tick_record.before,
            &tick_record.after,
            &tick_record.events,
            None,
        )
        .unwrap_or_else(|e| panic!("tick {tick}: {e}"));
}

#[test]
fn e6_every_committed_operation_and_tick_conserves() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    trail(&mut app).enable();
    let command = produce(41, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let records = trail(&mut app).drain_operations();
    let events = records.last().unwrap().events.clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    oracle
        .observe(
            &ExpectedOperation::Command(command.clone()),
            &initial,
            &after,
            &events,
            Some(&result),
        )
        .unwrap();
    assert_eq!(trail(&mut app).commands.len(), 1);
    assert_eq!(
        trail(&mut app).commands[0].operation_id,
        records[0].operation_id,
        "command and logical operation share one correlation ID"
    );
    let capture = app.world().resource::<AuditTrail>();
    assert!(capture.valid);
    let json = serde_json::to_vec(&records[0]).unwrap();
    let restored: turts_sim::OperationAudit = serde_json::from_slice(&json).unwrap();
    assert_eq!(restored, records[0]);
    for tick in 1..=101 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
    }
    assert!(oracle.outputs > 0);
    assert!(oracle.assert_positive_progress().is_ok());
    oracle
        .assert_command_records(&trail(&mut app).commands)
        .unwrap();
}

#[test]
fn e6_noop_log_spoof_and_hidden_partial_write_are_detected() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let before = audit_state(app.world_mut());
    let fake = EconomyEvent {
        tick: 0,
        ordinal: 0,
        owner_slot: 0,
        kind: EconomyEventKind::Extracted {
            node: 1,
            cargo: 3,
            item: common::cid("probe:timber"),
            amount: 1,
        },
    };
    let mut oracle = AccountingOracle::new(
        &before,
        &app.world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0,
    );
    let command = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::MoveTo {
            unit_ids: vec![3],
            target: turts_sim::Vec2::new(3.0, 3.0),
        },
    };
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(command.clone()),
                &before,
                &before,
                std::slice::from_ref(&fake),
                None
            )
            .is_err()
    );
    let mut hidden = before.clone();
    hidden.nodes[0].remaining -= 1;
    let auto = ExpectedOperation::Automatic {
        tick: 1,
        phase: common::accounting::AutomaticPhase::Gather,
        subject: common::accounting::AutomaticSubject::Gatherer(3),
    };
    assert!(
        oracle
            .observe(&auto, &before, &hidden, &[fake], None)
            .is_err()
    );
    let empty = AccountingOracle::new(
        &before,
        &app.world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0,
    );
    assert!(empty.assert_positive_progress().is_err());
}

#[test]
fn e6_gather_conversion_and_interruptions_conserve_through_delivery() {
    let files = economy_files()
        .into_iter()
        .map(|(path, bytes)| (path, String::from_utf8(bytes).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut files = files;
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace(
        "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
        "drop_off: None",
    );
    let mut app = scenario_with(files, 20);
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    trail(&mut app).enable();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let command = Command {
        slot: SlotId(0),
        seq: 10,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(command.clone()),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    for tick in 1..=1400 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
        if tick == 100 {
            let before_interrupt = audit_state(app.world_mut());
            let cargo_before = before_interrupt
                .economy
                .inventories
                .iter()
                .find(|i| i.unit == 3)
                .unwrap()
                .items
                .iter()
                .map(|x| x.on_hand)
                .sum::<u64>();
            assert!(cargo_before > 0, "fixture has cargo to interrupt");
            let interrupt = Command {
                slot: SlotId(0),
                seq: 11,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![3],
                    target: turts_sim::Vec2::new(30.0, 30.0),
                },
            };
            let result = apply_command(app.world_mut(), &interrupt);
            let after = audit_state(app.world_mut());
            let op = trail(&mut app).drain_operations().pop().unwrap();
            let cargo_after = after
                .economy
                .inventories
                .iter()
                .find(|i| i.unit == 3)
                .unwrap()
                .items
                .iter()
                .map(|x| x.on_hand)
                .sum::<u64>();
            assert_eq!(
                cargo_after, cargo_before,
                "MoveTo interruption keeps physical cargo"
            );
            oracle
                .observe(
                    &ExpectedOperation::Command(interrupt.clone()),
                    &before_interrupt,
                    &after,
                    &op.events,
                    Some(&result),
                )
                .unwrap();
            let resume = Command {
                slot: SlotId(0),
                seq: 12,
                kind: CommandKind::Gather {
                    unit_id: 3,
                    node_id: turts_sim::NodeId(1),
                },
            };
            let result = apply_command(app.world_mut(), &resume);
            let after = audit_state(app.world_mut());
            let op = trail(&mut app).drain_operations().pop().unwrap();
            oracle
                .observe(
                    &ExpectedOperation::Command(resume),
                    &oracle.modeled_state().clone(),
                    &after,
                    &op.events,
                    Some(&result),
                )
                .unwrap();
        }
    }
    assert!(
        oracle.extraction > 0,
        "seeded scenario must extract within 2,000 ticks"
    );
    assert!(
        oracle.delivery > 0,
        "seeded scenario must deliver within 2,000 ticks"
    );
    assert!(
        oracle.conversion > 0,
        "seeded scenario must convert within 2,000 ticks"
    );
    assert!(oracle.assert_positive_progress().is_ok());
    oracle
        .assert_command_records(&trail(&mut app).commands)
        .unwrap();
    let state = audit_state(app.world_mut());
    let total_remaining: u64 = state.nodes.iter().map(|n| n.remaining).sum();
    let timber = state
        .economy
        .inventories
        .iter()
        .flat_map(|i| &i.items)
        .filter(|x| x.resource == common::cid("probe:timber"))
        .map(|x| x.on_hand)
        .sum::<u64>();
    assert_eq!(
        total_remaining + timber + u64::try_from(oracle.delivery).unwrap(),
        2100,
        "node plus cargo plus converted sink equals the fixture's 2,000 node stock and 100 seeded items"
    );
}

#[test]
fn wrong_recipe_with_consistent_state_and_events() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let before = audit_state(app.world_mut());
    let expected = produce(5, "probe:hire");
    let actual = produce(5, "probe:train");
    let result = apply_command(app.world_mut(), &actual);
    let after = audit_state(app.world_mut());
    let record = trail(&mut app).drain_operations().pop().unwrap();
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&before, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(expected),
                &before,
                &after,
                &record.events,
                Some(&result)
            )
            .is_err()
    );
}

#[test]
fn wrong_cancel_target_with_consistent_refund() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let first = apply_command(app.world_mut(), &produce(1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let second = apply_command(app.world_mut(), &produce(2, "probe:train"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let before = audit_state(app.world_mut());
    let expected = Command {
        slot: SlotId(0),
        seq: 3,
        kind: CommandKind::CancelProduction {
            producer_id: 1,
            job_id: first,
        },
    };
    let actual = Command {
        slot: SlotId(0),
        seq: 3,
        kind: CommandKind::CancelProduction {
            producer_id: 1,
            job_id: second,
        },
    };
    let result = apply_command(app.world_mut(), &actual);
    let after = audit_state(app.world_mut());
    let event = app
        .world()
        .resource::<turts_sim::economy::events::PendingEconomyEvents>()
        .0
        .last()
        .unwrap()
        .clone();
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&before, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(expected),
                &before,
                &after,
                &[event],
                Some(&result)
            )
            .is_err()
    );
}

#[test]
fn rejected_command_missing_boundary_or_hidden_write() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let before = audit_state(app.world_mut());
    let bad = Command {
        slot: SlotId(0),
        seq: 2,
        kind: CommandKind::Produce {
            producer_id: 999,
            recipe_id: common::cid("probe:hire"),
        },
    };
    let result = apply_command(app.world_mut(), &bad);
    let after = audit_state(app.world_mut());
    let records = trail(&mut app).drain_commands();
    assert_eq!(
        records.iter().filter(|r| r.input == bad).count(),
        1,
        "every submitted rejection needs one command boundary"
    );
    assert_eq!(records[0].result, result);
    assert_eq!(records[0].before, before);
    assert_eq!(records[0].after, after);
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&before, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(bad),
                &before,
                &after,
                &[],
                Some(&result)
            )
            .is_ok()
    );
    oracle.assert_command_records(&records).unwrap();
    let mut forged = after.clone();
    forged.nodes[0].remaining -= 1;
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(records[0].input.clone()),
                &before,
                &forged,
                &[],
                Some(&result)
            )
            .is_err()
    );
}

#[test]
fn e7_claim_of_each_initialized_or_empty_slot_is_accounting_neutral() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let before = audit_state(app.world_mut());
    let mut expected_claims = before.claims.clone();
    for player in [777, 778] {
        let claimed = app
            .world_mut()
            .resource_mut::<turts_sim::slots::Slots>()
            .claim(player)
            .unwrap();
        expected_claims[usize::from(claimed.0)] = Some(player);
        let after = audit_state(app.world_mut());
        let mut normalized = after.clone();
        normalized.claims = before.claims.clone();
        assert_eq!(
            normalized, before,
            "slot claim {claimed:?} changes only claim ownership, never the complete accounting snapshot"
        );
        assert_eq!(after.claims, expected_claims);
    }
    assert!(
        app.world_mut()
            .resource_mut::<turts_sim::slots::Slots>()
            .claim(779)
            .is_err()
    );
}

#[test]
fn e8_rejection_matches_no_command_control() {
    let mut rejected = scenario(20);
    load_package(rejected.world_mut(), &economy_archive()).unwrap();
    let mut control = scenario(20);
    load_package(control.world_mut(), &economy_archive()).unwrap();
    let command = Command {
        slot: SlotId(0),
        seq: 7,
        kind: CommandKind::MoveTo {
            unit_ids: vec![u32::MAX],
            target: turts_sim::Vec2::new(1.0, 1.0),
        },
    };
    let result = apply_command(rejected.world_mut(), &command);
    assert!(result.outcome.is_err());
    rejected.update();
    control.update();
    assert_eq!(
        audit_state(rejected.world_mut()),
        audit_state(control.world_mut())
    );
}

#[test]
fn audit_state_json_roundtrip_preserves_raw_units_filters_and_clock() {
    let mut app = scenario(30);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let state = audit_state(app.world_mut());
    let bytes = serde_json::to_vec(&state).unwrap();
    let decoded: AuditState = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, state);
    assert_eq!(decoded.tick_hz, Some(30));
    assert!(!decoded.raw_units.is_empty());
    assert!(!decoded.raw_inventories.is_empty());
    assert!(decoded.catalog.is_some());
    assert!(decoded.bounds.is_some());
    assert!(decoded.raw_orders.iter().any(|o| o.rate_carry == 0));
    let _ = JobId(0);
}

#[test]
fn command_audit_json_preserves_nonfinite_rejected_target_bits() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let command = Command {
        slot: SlotId(0),
        seq: 99,
        kind: CommandKind::MoveTo {
            unit_ids: vec![3],
            target: turts_sim::Vec2::new(f32::NAN, f32::INFINITY),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    assert_eq!(
        result.outcome,
        Err(turts_sim::RejectReason::TargetNotFinite)
    );
    let record = trail(&mut app).drain_commands().pop().unwrap();
    let json = serde_json::to_vec(&record).unwrap();
    let decoded: turts_sim::CommandAudit = serde_json::from_slice(&json).unwrap();
    assert_eq!(decoded.operation_id, record.operation_id);
    assert_eq!(decoded.result, record.result);
    assert_eq!(decoded.before, record.before);
    assert_eq!(decoded.after, record.after);
    let CommandKind::MoveTo {
        target: original, ..
    } = record.input.kind
    else {
        panic!("wrong command kind")
    };
    let CommandKind::MoveTo { target, .. } = decoded.input.kind else {
        panic!("wrong command kind")
    };
    assert_eq!(target.x.to_bits(), original.x.to_bits());
    assert_eq!(target.y.to_bits(), original.y.to_bits());
}

#[test]
fn e6_transfer_command_boundaries_cover_success_and_rejection() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map=map.replace("(unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [])","(unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 5)])");
    let mut app = scenario_with(files, 20);
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    trail(&mut app).enable();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let before = oracle.modeled_state().clone();
    let result = turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        3,
        1,
        common::cid("probe:timber"),
        2,
    );
    assert!(result.is_ok());
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::InventoryTransfer {
                slot: 0,
                from: 3,
                to: 1,
                item: common::cid("probe:timber"),
                amount: 2,
            },
            &before,
            &after,
            &op.events,
            None,
        )
        .unwrap();
    let before = oracle.modeled_state().clone();
    let result = turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        3,
        1,
        common::cid("probe:timber"),
        u64::MAX,
    );
    assert!(result.is_err());
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::InventoryTransfer {
                slot: 0,
                from: 3,
                to: 1,
                item: common::cid("probe:timber"),
                amount: u64::MAX,
            },
            &before,
            &after,
            &op.events,
            None,
        )
        .unwrap();
    assert_eq!(
        op.before, op.after,
        "rejected transfer still has a captured boundary"
    );
}

#[test]
fn e6_production_duration_and_rate_at_10_and_30_hz() {
    for hz in [10, 30] {
        let mut app = scenario(hz);
        load_package(app.world_mut(), &economy_archive()).unwrap();
        let initial = audit_state(app.world_mut());
        let catalog = app
            .world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .clone();
        trail(&mut app).enable();
        let mut oracle = AccountingOracle::new(&initial, &catalog);
        let cmd = produce(55, "probe:train");
        let result = apply_command(app.world_mut(), &cmd);
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(cmd),
                &initial,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
        let duration = after
            .economy
            .queues
            .iter()
            .find(|q| q.producer == 1)
            .unwrap()
            .jobs[0]
            .duration_ticks;
        assert_eq!(
            duration,
            u64::from(hz) * 5,
            "5s recipe independently rounds to tick rate {hz}"
        );
        for tick in 1..=u32::try_from(duration + 1).unwrap() {
            advance_accounted_tick(&mut app, &mut oracle, tick);
        }
        assert_eq!(
            oracle.outputs, 1,
            "recipe completes exactly once at {hz} Hz"
        );
        assert!(oracle.assert_positive_progress().is_ok());
    }
}

#[test]
fn frozen_produce_model_rejects_consistent_poisoned_charge_and_hidden_filter_write() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let command = produce(901, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let operation = trail(&mut app).drain_operations().pop().unwrap();
    let mut poisoned = after.clone();
    let job = &mut poisoned
        .economy
        .queues
        .iter_mut()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0];
    job.charged[0].amount = job.charged[0].amount.saturating_add(900);
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(command.clone()),
                &initial,
                &poisoned,
                &operation.events,
                Some(&result)
            )
            .is_err(),
        "catalog cost and persistent job model reject a self-consistent poisoned charged line"
    );

    let mut move_app = scenario(20);
    load_package(move_app.world_mut(), &economy_archive()).unwrap();
    let initial = audit_state(move_app.world_mut());
    let catalog = move_app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let cmd = Command {
        slot: SlotId(0),
        seq: 902,
        kind: CommandKind::MoveTo {
            unit_ids: vec![3],
            target: turts_sim::Vec2::new(3.0, 3.0),
        },
    };
    let result = apply_command(move_app.world_mut(), &cmd);
    let mut hidden = audit_state(move_app.world_mut());
    hidden
        .raw_inventories
        .iter_mut()
        .find(|x| x.unit == 3)
        .unwrap()
        .accepted_items
        .clear();
    let events = move_app
        .world()
        .resource::<turts_sim::economy::events::PendingEconomyEvents>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(cmd),
                &initial,
                &hidden,
                &events,
                Some(&result)
            )
            .is_err(),
        "accepted MoveTo cannot rewrite hidden inventory filters"
    );
}

#[test]
fn stationary_unit_teleport_at_automatic_boundary_is_rejected() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut forged = initial.clone();
    let unit = forged
        .raw_units
        .iter_mut()
        .find(|u| u.unit_id == 3)
        .unwrap();
    unit.pos.x += 1.0;
    let pos = unit.pos.clone();
    forged
        .units
        .iter_mut()
        .find(|u| u.unit_id == 3)
        .unwrap()
        .pos = pos.clone();
    forged
        .economy
        .units
        .iter_mut()
        .find(|u| u.unit_id == 3)
        .unwrap()
        .pos = bevy_math::Vec2::new(pos.x, pos.y);
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 0,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(3)
                },
                &forged,
                &forged,
                &[],
                None
            )
            .is_err(),
        "stationary units have independently predicted no-motion transitions"
    );
}

#[test]
fn frozen_start_model_rejects_recipe_switch_with_truthful_start_event() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let command = produce(910, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let queued = trail(&mut app).drain_operations().pop().unwrap();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &initial,
            &after,
            &queued.events,
            Some(&result),
        )
        .unwrap();
    app.update();
    let ops = trail(&mut app).drain_operations();
    let before_tick = oracle.modeled_state().clone();
    for (cursor, order) in before_tick.raw_orders.iter().enumerate() {
        let op = &ops[cursor];
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 1,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(order.unit),
                },
                &op.before,
                &op.after,
                &op.events,
                None,
            )
            .unwrap();
    }
    let start = ops
        .iter()
        .find(|op| {
            op.events
                .iter()
                .any(|e| matches!(e.kind, EconomyEventKind::ProductionStarted { .. }))
        })
        .unwrap();
    let job = start
        .after
        .economy
        .queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0]
        .id;
    let mut fraudulent = start.after.clone();
    fraudulent
        .economy
        .queues
        .iter_mut()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0]
        .recipe = common::cid("probe:hire");
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 1,
                    phase: AutomaticPhase::ProductionAdvance,
                    subject: AutomaticSubject::HeadJob { producer: 1, job }
                },
                &start.before,
                &fraudulent,
                &start.events,
                None
            )
            .is_err(),
        "production start preserves the input-bound recipe even with a truthful event"
    );
}

#[test]
fn cancel_of_requested_job_cannot_drop_an_unrelated_queued_job() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let first_cmd = produce(920, "probe:hire");
    let first_result = apply_command(app.world_mut(), &first_cmd);
    let first_after = audit_state(app.world_mut());
    let first_op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(first_cmd),
            &initial,
            &first_after,
            &first_op.events,
            Some(&first_result),
        )
        .unwrap();
    let second_cmd = produce(921, "probe:train");
    let second_result = apply_command(app.world_mut(), &second_cmd);
    let second_after = audit_state(app.world_mut());
    let second_op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(second_cmd),
            &first_after,
            &second_after,
            &second_op.events,
            Some(&second_result),
        )
        .unwrap();
    let first = first_result.outcome.unwrap().job_id.unwrap();
    let cancel = Command {
        slot: SlotId(0),
        seq: 922,
        kind: CommandKind::CancelProduction {
            producer_id: 1,
            job_id: first,
        },
    };
    let result = apply_command(app.world_mut(), &cancel);
    let after = audit_state(app.world_mut());
    let operation = trail(&mut app).drain_operations().pop().unwrap();
    let mut fraudulent = after.clone();
    fraudulent
        .economy
        .queues
        .iter_mut()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs
        .clear();
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(cancel),
                &second_after,
                &fraudulent,
                &operation.events,
                Some(&result)
            )
            .is_err(),
        "cancel removes exactly its submitted job and preserves the other queued job"
    );
}

#[test]
fn tick_boundary_rejects_forged_event_cursor_and_missing_command_boundary() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let mut after = initial.clone();
    after.economy.tick = 1;
    after.next_event_ordinal = Some(1);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::TickBoundary { tick: 1 },
                &initial,
                &after,
                &[],
                None
            )
            .is_err(),
        "tick completion must drain and reset the independently modeled event cursor"
    );

    trail(&mut app).enable();
    let before = audit_state(app.world_mut());
    let command = Command {
        slot: SlotId(0),
        seq: 930,
        kind: CommandKind::Produce {
            producer_id: 999,
            recipe_id: common::cid("probe:hire"),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let mut oracle = AccountingOracle::new(&before, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(command.clone()),
                &before,
                &after,
                &[],
                Some(&result)
            )
            .is_ok()
    );
    assert!(
        oracle.assert_command_records(&[]).is_err(),
        "submitted rejected command with missing boundary is refuted by owned input ledger"
    );
}

#[test]
fn movement_gate_rejects_tick_skip_second_step_and_boundary_teleport() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut move_command = Command {
        slot: SlotId(0),
        seq: 940,
        kind: CommandKind::MoveTo {
            unit_ids: vec![3],
            target: turts_sim::Vec2::new(30.0, 30.0),
        },
    };
    let move_result = apply_command(app.world_mut(), &move_command);
    let move_after = audit_state(app.world_mut());
    let move_op = trail(&mut app).drain_operations().pop().unwrap();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    oracle
        .observe(
            &ExpectedOperation::Command(move_command.clone()),
            &initial,
            &move_after,
            &move_op.events,
            Some(&move_result),
        )
        .unwrap();
    let mut skipped = move_after.clone();
    skipped.economy.tick = 40;
    skipped.economy.tick = 40;
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 40,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(3)
                },
                &skipped,
                &skipped,
                &[],
                None
            )
            .is_err(),
        "automatic snapshots cannot jump multiple ticks"
    );

    app.update();
    let operations = trail(&mut app).drain_operations();
    let first = &operations[0];
    oracle
        .observe(
            &ExpectedOperation::Automatic {
                tick: 1,
                phase: AutomaticPhase::Gather,
                subject: AutomaticSubject::Gatherer(3),
            },
            &first.before,
            &first.after,
            &first.events,
            None,
        )
        .unwrap();
    let mut second_before = oracle.modeled_state().clone();
    if let Some(unit) = second_before.raw_units.iter_mut().find(|u| u.unit_id == 3) {
        unit.pos.x += 0.1;
        let pos = unit.pos.clone();
        second_before
            .units
            .iter_mut()
            .find(|u| u.unit_id == 3)
            .unwrap()
            .pos = pos.clone();
        second_before
            .economy
            .units
            .iter_mut()
            .find(|u| u.unit_id == 3)
            .unwrap()
            .pos = bevy_math::Vec2::new(pos.x, pos.y);
    }
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 1,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(5)
                },
                &second_before,
                &second_before,
                &[],
                None
            )
            .is_err(),
        "a second movement discontinuity cannot occur within the same tick"
    );

    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    app.update();
    let operations = trail(&mut app).drain_operations();
    for (idx, op) in operations.iter().enumerate() {
        let unit = initial.raw_orders[idx].unit;
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 1,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(unit),
                },
                &op.before,
                &op.after,
                &op.events,
                None,
            )
            .unwrap();
    }
    let record = trail(&mut app).drain_ticks().pop().unwrap();
    let mut forged = record.after.clone();
    let raw = forged
        .raw_units
        .iter_mut()
        .find(|u| u.unit_id == 3)
        .unwrap();
    raw.pos.x += 2.0;
    let pos = raw.pos.clone();
    forged
        .units
        .iter_mut()
        .find(|u| u.unit_id == 3)
        .unwrap()
        .pos = pos.clone();
    forged
        .economy
        .units
        .iter_mut()
        .find(|u| u.unit_id == 3)
        .unwrap()
        .pos = bevy_math::Vec2::new(pos.x, pos.y);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::TickBoundary { tick: 1 },
                &record.before,
                &forged,
                &record.events,
                None
            )
            .is_err(),
        "TickBoundary cannot teleport a stationary unit after the movement gate ran"
    );
    move_command.seq = 941;
}

#[test]
fn e6_reserve_then_charge_tracks_claim_start_and_output() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let command = produce(950, "probe:hire");
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    let raw_queue_json = serde_json::to_vec(&after).unwrap();
    let raw_queue_roundtrip: AuditState = serde_json::from_slice(&raw_queue_json).unwrap();
    assert_eq!(
        raw_queue_roundtrip, after,
        "raw reservation endpoints and active queue survive AuditState JSON roundtrip"
    );
    let queued = after
        .raw_queues
        .iter()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs
        .first()
        .unwrap();
    assert_eq!(
        queued.id,
        result.outcome.as_ref().unwrap().job_id.unwrap().0
    );
    assert!(
        queued
            .reservation
            .as_ref()
            .is_some_and(|r| r.job_id == queued.id && !r.lines.is_empty())
    );
    let operation_json = serde_json::to_vec(&op).unwrap();
    let operation_roundtrip: turts_sim::OperationAudit =
        serde_json::from_slice(&operation_json).unwrap();
    assert_eq!(
        operation_roundtrip, op,
        "logical operation JSONL payload retains active raw reservation lines"
    );
    assert!(
        after
            .ledger_claims
            .iter()
            .any(|x| matches!(x, turts_sim::ReservationClaimView::Active { .. })),
        "ReserveThenCharge claim is visible at queue time"
    );
    for tick in 1..=121 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
    }
    assert_eq!(oracle.outputs, 1);
    assert!(oracle.gross_charge >= 50);
    oracle.assert_positive_progress().unwrap();
}

#[test]
fn e6_mixed_cost_rejection_keeps_every_endpoint_unchanged() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let recipe = files.get_mut("production/train.ron").unwrap();
    *recipe = recipe.replace("(resource: \"probe:token\", amount: 100, source: SlotBalance)", "(resource: \"probe:token\", amount: 100, source: SlotBalance), (resource: \"probe:timber\", amount: 500, source: ProducerInventory)");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "contents: [(item: \"probe:timber\", amount: 100)]",
        "contents: [(item: \"probe:timber\", amount: 10)]",
    );
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let before = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let cmd = produce(951, "probe:train");
    let result = apply_command(app.world_mut(), &cmd);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    assert!(result.outcome.is_err());
    let token_balance = |state: &AuditState| {
        state
            .economy
            .balances
            .iter()
            .find(|balance| balance.slot == 0 && balance.resource == common::cid("probe:token"))
            .unwrap()
            .on_hand
    };
    assert_eq!(
        token_balance(&after),
        token_balance(&before),
        "rejected mixed-cost command must roll back its first SlotBalance debit"
    );
    assert_eq!(
        before, after,
        "rejected mixed costs leave the full state unchanged"
    );
    let mut oracle = AccountingOracle::new(&before, &catalog);
    oracle
        .observe(
            &ExpectedOperation::Command(cmd),
            &before,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
}

#[test]
fn active_gather_cannot_rewrite_compiled_duration_metadata() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let command = Command {
        slot: SlotId(0),
        seq: 960,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    app.update();
    let operation = trail(&mut app)
        .drain_operations()
        .into_iter()
        .find(|x| {
            x.after
                .raw_orders
                .iter()
                .find(|o| o.unit == 3)
                .is_some_and(|o| o.stage != turts_sim::GatherStage::Idle)
        })
        .unwrap();
    let mut forged = operation.after.clone();
    forged.compiled_durations[0].ticks += 999;
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 1,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(3)
                },
                &operation.before,
                &forged,
                &operation.events,
                None
            )
            .is_err(),
        "gather cannot mutate frozen compiled production durations"
    );
}

#[test]
fn e6_transfer_rejections_cover_filter_capacity_and_access() {
    let mut cases = Vec::new();
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let exchange = files.get_mut("units/exchange.ron").unwrap();
    *exchange = exchange.replace("accepts: [\"probe:timber\"]", "accepts: []");
    cases.push((files, 0u8, 3u32, 2u32, common::cid("probe:timber"), 1u64));
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let exchange = files.get_mut("units/exchange.ron").unwrap();
    *exchange = exchange.replace("capacity: 2000", "capacity: 1");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "pos: (x: 8.0, y: 20.0), contents: []",
        "pos: (x: 8.0, y: 20.0), contents: [(item: \"probe:timber\", amount: 1)]",
    );
    cases.push((files, 0, 1, 2, common::cid("probe:timber"), 1));
    let files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    cases.push((files, 1, 1, 3, common::cid("probe:timber"), 1));
    for (files, slot, from, to, item, amount) in cases {
        let mut app = scenario_with(files, 20);
        trail(&mut app).enable();
        let before = audit_state(app.world_mut());
        let catalog = app
            .world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .clone();
        let result = turts_sim::economy::transaction::inventory_transfer(
            app.world_mut(),
            SlotId(slot),
            from,
            to,
            item.clone(),
            amount,
        );
        assert!(result.is_err());
        let after = audit_state(app.world_mut());
        let op = trail(&mut app)
            .drain_operations()
            .pop()
            .expect("rejected trusted transfer is still captured");
        let mut oracle = AccountingOracle::new(&before, &catalog);
        oracle
            .observe(
                &ExpectedOperation::InventoryTransfer {
                    slot,
                    from,
                    to,
                    item,
                    amount,
                },
                &before,
                &after,
                &op.events,
                None,
            )
            .unwrap();
        assert_eq!(before, after);
    }
}

#[test]
fn e6_self_store_is_never_selected_as_its_own_receiver() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker.replace(
        "drop_off: None",
        r#"drop_off: Some((rules: [(item: "probe:timber", effect: Store)]))"#,
    );
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let command = Command {
        slot: SlotId(0),
        seq: 970,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    for tick in 1..=900 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
        let state = audit_state(app.world_mut());
        assert_ne!(
            state
                .raw_orders
                .iter()
                .find(|o| o.unit == 3)
                .and_then(|o| o.receiver),
            Some(3),
            "a Store gatherer cannot select itself"
        );
    }
    assert!(oracle.delivery > 0);
    oracle.assert_positive_progress().unwrap();
}

#[test]
fn e6_running_cancel_refund_rejects_when_inventory_endpoint_is_full() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace("capacity: 2000", "capacity: 50");
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker
        .replace("capacity: 10", "capacity: 50")
        .replace("unload_per_second: 10", "unload_per_second: 1000");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "pos: (x: 8.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 100)]",
        "pos: (x: 8.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 50)]",
    );
    *map = map.replace(
        "pos: (x: 12.0, y: 8.0), contents: []",
        "pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 50)]",
    );
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let hire = produce(980, "probe:hire");
    let result = apply_command(app.world_mut(), &hire);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(hire),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    let job = result.outcome.unwrap().job_id.unwrap();
    let gather = Command {
        slot: SlotId(0),
        seq: 981,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &gather);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(gather),
            &oracle.modeled_state().clone(),
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    for tick in 1..=12 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
    }
    let before = audit_state(app.world_mut());
    assert_eq!(
        before
            .economy
            .inventories
            .iter()
            .find(|i| i.unit == 1)
            .unwrap()
            .items
            .iter()
            .map(|x| x.on_hand)
            .sum::<u64>(),
        50
    );
    let cancel = Command {
        slot: SlotId(0),
        seq: 982,
        kind: CommandKind::CancelProduction {
            producer_id: 1,
            job_id: job,
        },
    };
    let result = apply_command(app.world_mut(), &cancel);
    assert!(matches!(
        result.outcome,
        Err(turts_sim::RejectReason::RefundDestinationFull)
    ));
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(cancel),
            &before,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn e6_running_cancel_refund_rejects_on_balance_u64_overflow() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "(balance: \"probe:token\", amount: 200)",
        "(balance: \"probe:token\", amount: 18446744073709551615)",
    );
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace(
        "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
        "drop_off: None",
    );
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker
        .replace("capacity: 10", "capacity: 100")
        .replace("gather_per_second: 5", "gather_per_second: 100")
        .replace("unload_per_second: 10", "unload_per_second: 1000");
    let train = files.get_mut("production/train.ron").unwrap();
    *train = train.replace("duration_ms: 5000", "duration_ms: 600000");

    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);

    let command = produce(995, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    let job = result.outcome.unwrap().job_id.unwrap();

    let gather = Command {
        slot: SlotId(0),
        seq: 996,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let before = oracle.modeled_state().clone();
    let result = apply_command(app.world_mut(), &gather);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(gather),
            &before,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();

    let mut reached_max = false;
    for tick in 1..=500 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
        let state = audit_state(app.world_mut());
        if state.economy.balances.iter().any(|balance| {
            balance.slot == 0
                && balance.resource == common::cid("probe:token")
                && balance.on_hand == u64::MAX
        }) {
            reached_max = true;
            break;
        }
    }
    assert!(reached_max, "Convert fills the balance to u64::MAX");
    let before = oracle.modeled_state().clone();
    assert!(
        before
            .economy
            .queues
            .iter()
            .flat_map(|queue| &queue.jobs)
            .any(|queued| queued.id == job.0 && queued.stage == turts_sim::JobStage::Running)
    );
    let command = Command {
        slot: SlotId(0),
        seq: 997,
        kind: CommandKind::CancelProduction {
            producer_id: 1,
            job_id: job,
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let balance_of = |state: &AuditState| {
        state
            .economy
            .balances
            .iter()
            .find(|balance| balance.slot == 0 && balance.resource == common::cid("probe:token"))
            .unwrap()
            .on_hand
    };
    assert_eq!(
        result.outcome,
        Err(turts_sim::RejectReason::RefundDestinationOverflow),
        "refund at balance {} must reject without wrapping (observed after {})",
        balance_of(&before),
        balance_of(&after),
    );
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &before,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn e6_cancel_refund_matrix_covers_both_policies_and_all_stages() {
    for (recipe, ticks, blocked) in [
        ("probe:train", 0, false),
        ("probe:train", 1, false),
        ("probe:hire", 0, false),
        ("probe:hire", 1, false),
        ("probe:train", 101, true),
        ("probe:hire", 121, true),
    ] {
        let mut files = economy_files()
            .into_iter()
            .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
            .collect::<std::collections::BTreeMap<_, _>>();
        if blocked {
            let yard = files.get_mut("units/yard.ron").unwrap();
            *yard = yard.replace(
                "[(x: 0.0, y: 4.0), (x: 0.0, y: -4.0)]",
                "[(x: 0.0, y: 0.0)]",
            );
        }
        let mut app = scenario_with(files, 20);
        trail(&mut app).enable();
        let initial = audit_state(app.world_mut());
        let catalog = app
            .world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .clone();
        let mut oracle = AccountingOracle::new(&initial, &catalog);
        let cmd = produce(990, recipe);
        let result = apply_command(app.world_mut(), &cmd);
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(cmd),
                &initial,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
        let job = result.outcome.unwrap().job_id.unwrap();
        for tick in 1..=ticks {
            advance_accounted_tick(&mut app, &mut oracle, tick);
        }
        let before = audit_state(app.world_mut());
        let frozen = before
            .economy
            .queues
            .iter()
            .find(|q| q.producer == 1)
            .unwrap()
            .jobs
            .iter()
            .find(|j| j.id == job.0)
            .unwrap();
        if ticks == 0 {
            assert_eq!(frozen.stage, turts_sim::JobStage::Queued);
        } else if blocked {
            assert_eq!(frozen.stage, turts_sim::JobStage::OutputBlocked);
        } else {
            assert_eq!(frozen.stage, turts_sim::JobStage::Running);
        }
        let cancel = Command {
            slot: SlotId(0),
            seq: 991,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        };
        let result = apply_command(app.world_mut(), &cancel);
        assert!(result.outcome.is_ok());
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(cancel),
                &before,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
        assert_eq!(oracle.cancellations, 1);
    }
}

#[test]
fn e6_gather_waits_distinguish_no_receiver_full_and_conversion_overflow() {
    for wait in [
        turts_sim::WaitReason::NoReceiver,
        turts_sim::WaitReason::DestinationFull,
        turts_sim::WaitReason::DestinationOverflow,
    ] {
        let mut files = economy_files()
            .into_iter()
            .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let yard = files.get_mut("units/yard.ron").unwrap();
        *yard = yard.replace(
            "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
            "drop_off: None",
        );
        let worker = files.get_mut("units/worker.ron").unwrap();
        *worker = worker.replace("capacity: 10", "capacity: 50");
        let map = files.get_mut("maps/test.ron").unwrap();
        *map = map.replace(
            "pos: (x: 12.0, y: 8.0), contents: []",
            "pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 50)]",
        );
        let exchange = files.get_mut("units/exchange.ron").unwrap();
        match wait {
            turts_sim::WaitReason::NoReceiver=>*exchange=exchange.replace("drop_off: Some((rules: [(item: \"probe:timber\", effect: Convert(balance: \"probe:token\", per_item: 2))]))","drop_off: None"),
            turts_sim::WaitReason::DestinationFull=>{let exchange=files.get_mut("units/exchange.ron").unwrap();*exchange=exchange.replace("capacity: 2000","capacity: 1").replace("effect: Convert(balance: \"probe:token\", per_item: 2)","effect: Store");let map=files.get_mut("maps/test.ron").unwrap();*map=map.replace("pos: (x: 8.0, y: 20.0), contents: []","pos: (x: 8.0, y: 20.0), contents: [(item: \"probe:timber\", amount: 1)]");},
            turts_sim::WaitReason::DestinationOverflow=>{let map=files.get_mut("maps/test.ron").unwrap();*map=map.replace("balance: \"probe:token\", amount: 200)]","balance: \"probe:token\", amount: 18446744073709551614)]");},
            _=>unreachable!(),
        }
        let mut app = scenario_with(files, 20);
        trail(&mut app).enable();
        let initial = audit_state(app.world_mut());
        let catalog = app
            .world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .clone();
        let mut oracle = AccountingOracle::new(&initial, &catalog);
        let cmd = Command {
            slot: SlotId(0),
            seq: 995,
            kind: CommandKind::Gather {
                unit_id: 3,
                node_id: turts_sim::NodeId(1),
            },
        };
        let result = apply_command(app.world_mut(), &cmd);
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(cmd),
                &initial,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
        advance_accounted_tick(&mut app, &mut oracle, 1);
        let state = audit_state(app.world_mut());
        assert!(
            matches!(state.raw_orders.iter().find(|x|x.unit==3).unwrap().stage,turts_sim::GatherStage::Waiting{reason}if reason==wait),
            "receiver state yields the catalog-derived wait reason {wait:?}"
        );
    }
}

#[test]
fn e6_partial_store_unloads_and_exhausts_the_seeded_node() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "pos: (x: 24.0, y: 8.0), remaining: 1000",
        "pos: (x: 24.0, y: 8.0), remaining: 25",
    );
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let cmd = Command {
        slot: SlotId(0),
        seq: 996,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &cmd);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(cmd),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    for tick in 1..=1000 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
    }
    let state = audit_state(app.world_mut());
    assert_eq!(
        state
            .nodes
            .iter()
            .find(|n| n.id == turts_sim::NodeId(1))
            .unwrap()
            .remaining,
        0,
        "the seeded node is depleted exactly once"
    );
    assert!(oracle.extraction >= 25);
    assert!(oracle.delivery > 0);
    oracle.assert_positive_progress().unwrap();
}

#[test]
fn e6_charge_and_reserve_policies_compete_for_the_same_last_stock() {
    for (first, second) in [("probe:train", "probe:hire"), ("probe:hire", "probe:train")] {
        let mut files = economy_files()
            .into_iter()
            .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let recipe = files.get_mut("production/hire.ron").unwrap();
        *recipe = recipe.replace(
            "resource: \"probe:timber\", amount: 50, source: ProducerInventory",
            "resource: \"probe:token\", amount: 100, source: SlotBalance",
        );
        let map = files.get_mut("maps/test.ron").unwrap();
        *map = map.replace(
            "balance: \"probe:token\", amount: 200)]",
            "balance: \"probe:token\", amount: 100)]",
        );
        let mut app = scenario_with(files, 20);
        trail(&mut app).enable();
        let initial = audit_state(app.world_mut());
        let catalog = app
            .world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .clone();
        let mut oracle = AccountingOracle::new(&initial, &catalog);
        let first_command = produce(997, first);
        let first_result = apply_command(app.world_mut(), &first_command);
        let first_after = audit_state(app.world_mut());
        let first_op = trail(&mut app).drain_operations().pop().unwrap();
        assert!(first_result.outcome.is_ok());
        oracle
            .observe(
                &ExpectedOperation::Command(first_command),
                &initial,
                &first_after,
                &first_op.events,
                Some(&first_result),
            )
            .unwrap();
        let second_command = produce(998, second);
        let second_result = apply_command(app.world_mut(), &second_command);
        assert!(second_result.outcome.is_err());
        let second_after = audit_state(app.world_mut());
        let second_op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(second_command),
                &first_after,
                &second_after,
                &second_op.events,
                Some(&second_result),
            )
            .unwrap();
        let balance = second_after
            .economy
            .balances
            .iter()
            .find(|x| x.resource == common::cid("probe:token") && x.slot == 0)
            .unwrap();
        assert_eq!(balance.on_hand, if first == "probe:hire" { 100 } else { 0 });
        assert_eq!(
            balance.reserved,
            if first == "probe:hire" { 100 } else { 0 }
        );
    }
}

#[test]
fn review_oracle_rejects_forged_gather_receiver_and_target() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "(unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [])",
        "(unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 10)])",
    );
    let mut app = scenario_with(files, 20);
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    trail(&mut app).enable();
    let mut modeled = AccountingOracle::new(&initial, &catalog);
    let command = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    modeled
        .observe(
            &ExpectedOperation::Command(command.clone()),
            &op.before,
            &op.after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    app.update();
    let op = trail(&mut app).drain_operations().remove(0);
    let expected = ExpectedOperation::Automatic {
        tick: 1,
        phase: AutomaticPhase::Gather,
        subject: AutomaticSubject::Gatherer(3),
    };
    let mut forged = op.after.clone();
    let raw_order = forged.raw_orders.iter_mut().find(|o| o.unit == 3).unwrap();
    raw_order.receiver = Some(2);
    raw_order.stage = turts_sim::GatherStage::Unloading;
    let view = forged
        .economy
        .orders
        .iter_mut()
        .find(|o| o.unit == 3)
        .unwrap();
    view.receiver = Some(2);
    view.stage = turts_sim::GatherStage::Unloading;
    forged
        .raw_units
        .iter_mut()
        .find(|u| u.unit_id == 3)
        .unwrap()
        .move_target = Some(turts_mod_api::WorldPos { x: 60.0, y: 60.0 });
    let mut fraud_oracle = AccountingOracle::new(&initial, &catalog);
    fraud_oracle
        .observe(
            &ExpectedOperation::Command(command),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    assert!(
        fraud_oracle
            .observe(&expected, &op.before, &forged, &op.events, None)
            .is_err(),
        "receiver/stage/target must be derived independently"
    );
}

#[test]
fn review_raw_queue_fields_and_allocator_presence_are_frozen_from_prior_model() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let command = produce(881, "probe:hire");
    let result = apply_command(app.world_mut(), &command);
    let mut after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let raw_job = &mut after
        .raw_queues
        .iter_mut()
        .find(|q| q.producer == 1)
        .unwrap()
        .jobs[0];
    raw_job.duration_ticks += 999;
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(command),
                &op.before,
                &after,
                &op.events,
                Some(&result)
            )
            .is_err(),
        "raw duration is derived from frozen recipe duration"
    );

    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let command = produce(883, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let mut after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    after.resources.allocators = false;
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(command),
                &op.before,
                &after,
                &op.events,
                Some(&result)
            )
            .is_err(),
        "resource-presence bits are immutable evidence"
    );
}

#[test]
fn review_oracle_rejects_hidden_cross_owner_inventory_write_during_gather() {
    let files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let command = Command {
        slot: SlotId(0),
        seq: 2,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let op = trail(&mut app).drain_operations().pop().unwrap();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &op.before,
            &op.after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    let mut found = false;
    for tick in 1..=100 {
        app.update();
        let operations = trail(&mut app).drain_operations();
        for op in operations {
            if !op
                .events
                .iter()
                .any(|e| matches!(e.kind, EconomyEventKind::Extracted { .. }))
            {
                continue;
            }
            let mut forged = op.after.clone();
            let owner_zero = forged
                .economy
                .inventories
                .iter_mut()
                .find(|i| i.unit == 1)
                .unwrap();
            owner_zero
                .items
                .iter_mut()
                .find(|i| i.resource == common::cid("probe:timber"))
                .unwrap()
                .on_hand -= 1;
            forged
                .economy
                .inventories
                .iter_mut()
                .find(|i| i.unit == 5)
                .unwrap()
                .items
                .push(turts_sim::ItemView {
                    resource: common::cid("probe:timber"),
                    on_hand: 1,
                    reserved: 0,
                });
            let mut isolated = AccountingOracle::new(&op.before, &catalog);
            let got = isolated.observe(
                &ExpectedOperation::Automatic {
                    tick,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(3),
                },
                &op.before,
                &forged,
                &op.events,
                None,
            );
            assert!(
                got.is_err(),
                "truthful extraction must reject unrelated cross-owner stock movement"
            );
            found = true;
            break;
        }
        if found {
            break;
        }
    }
    assert!(found, "fixture reaches a gather extraction");
}

#[test]
fn review_oracle_requires_valid_production_output_even_when_event_is_missing() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    trail(&mut app).enable();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let command = produce(78, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let op = trail(&mut app).drain_operations().pop().unwrap();
    let job = result.outcome.as_ref().unwrap().job_id.unwrap().0;
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &op.before,
            &op.after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    for tick in 1..=100 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
    }
    app.update();
    let operations = trail(&mut app).drain_operations();
    assert_eq!(operations.len(), 4);
    for (index, unit) in [3, 5].into_iter().enumerate() {
        let op = &operations[index];
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 101,
                    phase: AutomaticPhase::Gather,
                    subject: AutomaticSubject::Gatherer(unit),
                },
                &op.before,
                &op.after,
                &op.events,
                None,
            )
            .unwrap();
    }
    let advance = &operations[2];
    oracle
        .observe(
            &ExpectedOperation::Automatic {
                tick: 101,
                phase: AutomaticPhase::ProductionAdvance,
                subject: AutomaticSubject::HeadJob { producer: 1, job },
            },
            &advance.before,
            &advance.after,
            &advance.events,
            None,
        )
        .unwrap();
    let op = &operations[3];
    assert!(
        op.events
            .iter()
            .any(|e| matches!(e.kind, EconomyEventKind::ProductionCompleted { .. }))
    );
    let got = oracle.observe(
        &ExpectedOperation::Automatic {
            tick: 101,
            phase: AutomaticPhase::OutputAttempt,
            subject: AutomaticSubject::HeadJob { producer: 1, job },
        },
        &op.before,
        &op.before,
        &[],
        None,
    );
    assert!(
        got.is_err(),
        "valid output must not be reclassified as blocked from event absence"
    );
}

#[test]
fn review_oracle_rejects_forged_event_owner() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    trail(&mut app).enable();
    let command = produce(79, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let op = trail(&mut app).drain_operations().pop().unwrap();
    let mut forged = op.after.clone();
    let mut events = op.events.clone();
    events[0].owner_slot = 1;
    forged.pending_events.last_mut().unwrap().owner_slot = 1;
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Command(command),
                &op.before,
                &forged,
                &events,
                Some(&result)
            )
            .is_err(),
        "command event owner must come from submitted slot"
    );
}

#[test]
fn review_empty_receiver_stage_resumes_after_transfer_removes_cargo() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "(unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [])",
        "(unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 10)])",
    );
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let gather = Command {
        slot: SlotId(0),
        seq: 201,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &gather);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(gather),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    let before_transfer = oracle.modeled_state().clone();
    let result = turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        3,
        1,
        common::cid("probe:timber"),
        10,
    );
    assert!(result.is_ok());
    let after_transfer = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::InventoryTransfer {
                slot: 0,
                from: 3,
                to: 1,
                item: common::cid("probe:timber"),
                amount: 10,
            },
            &before_transfer,
            &after_transfer,
            &op.events,
            None,
        )
        .unwrap();

    app.update();
    let operations = trail(&mut app).drain_operations();
    let op = operations
        .iter()
        .find(|op| op.before.raw_orders.iter().any(|order| order.unit == 3))
        .unwrap();
    let expected = ExpectedOperation::Automatic {
        tick: 1,
        phase: AutomaticPhase::Gather,
        subject: AutomaticSubject::Gatherer(3),
    };
    assert_eq!(
        op.after
            .raw_orders
            .iter()
            .find(|order| order.unit == 3)
            .unwrap()
            .stage,
        turts_sim::GatherStage::MovingToNode,
        "empty receiver phase resumes the live deposit"
    );
    let truthful = oracle.observe(&expected, &op.before, &op.after, &op.events, None);
    let mut suppressed = AccountingOracle::new(&op.before, &catalog);
    let no_op = suppressed.observe(&expected, &op.before, &op.before, &[], None);
    assert!(
        truthful.is_ok() && no_op.is_err(),
        "truthful empty-cargo resume must pass and suppressed transition must fail: truthful={truthful:?}, suppressed={no_op:?}"
    );
}

#[test]
fn review_stocked_no_deposit_wait_selects_receiver_after_transfer() {
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace("remaining: 1000", "remaining: 1");
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let gather = Command {
        slot: SlotId(0),
        seq: 202,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &gather);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(gather),
            &initial,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();

    let mut waiting_tick = None;
    for tick in 1..=1200 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
        if oracle
            .modeled_state()
            .raw_orders
            .iter()
            .find(|order| order.unit == 3)
            .is_some_and(|order| {
                order.stage
                    == (turts_sim::GatherStage::Waiting {
                        reason: turts_sim::WaitReason::NoDeposit,
                    })
            })
        {
            waiting_tick = Some(tick);
            break;
        }
    }
    let waiting_tick = waiting_tick.expect("finite nodes reach an empty NoDeposit wait");
    let before_transfer = oracle.modeled_state().clone();
    let result = turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        1,
        3,
        common::cid("probe:timber"),
        1,
    );
    assert!(result.is_ok());
    let after_transfer = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::InventoryTransfer {
                slot: 0,
                from: 1,
                to: 3,
                item: common::cid("probe:timber"),
                amount: 1,
            },
            &before_transfer,
            &after_transfer,
            &op.events,
            None,
        )
        .unwrap();

    app.update();
    let operations = trail(&mut app).drain_operations();
    let op = operations
        .iter()
        .find(|op| op.before.raw_orders.iter().any(|order| order.unit == 3))
        .unwrap();
    let expected = ExpectedOperation::Automatic {
        tick: waiting_tick + 1,
        phase: AutomaticPhase::Gather,
        subject: AutomaticSubject::Gatherer(3),
    };
    assert_eq!(
        op.after
            .raw_orders
            .iter()
            .find(|order| order.unit == 3)
            .unwrap()
            .stage,
        turts_sim::GatherStage::MovingToReceiver,
        "new cargo wakes the NoDeposit order and selects its Store receiver"
    );
    assert_eq!(
        op.after
            .raw_orders
            .iter()
            .find(|order| order.unit == 3)
            .unwrap()
            .receiver,
        Some(1),
    );
    let truthful = oracle.observe(&expected, &op.before, &op.after, &op.events, None);
    let mut suppressed = AccountingOracle::new(&op.before, &catalog);
    let no_op = suppressed.observe(&expected, &op.before, &op.before, &[], None);
    assert!(
        truthful.is_ok() && no_op.is_err(),
        "stocked NoDeposit must wake for receiver selection and reject a no-op: truthful={truthful:?}, suppressed={no_op:?}"
    );
}

#[test]
fn review_oracle_accepts_local_transfer_into_empty_inventory_row() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let result = turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        1,
        3,
        common::cid("probe:timber"),
        1,
    );
    assert!(result.is_ok());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::InventoryTransfer {
                slot: 0,
                from: 1,
                to: 3,
                item: common::cid("probe:timber"),
                amount: 1,
            },
            &op.before,
            &op.after,
            &op.events,
            None,
        )
        .unwrap();
    for amount in [1, 1] {
        let before = oracle.modeled_state().clone();
        let result = turts_sim::economy::transaction::inventory_transfer(
            app.world_mut(),
            SlotId(0),
            3,
            1,
            common::cid("probe:timber"),
            amount,
        );
        if amount == 1
            && before
                .economy
                .inventories
                .iter()
                .find(|i| i.unit == 3)
                .unwrap()
                .items
                .iter()
                .any(|i| i.on_hand > 0)
        {
            assert!(result.is_ok());
        } else {
            assert!(
                result.is_err(),
                "empty source inventory cannot create stock"
            );
        }
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::InventoryTransfer {
                    slot: 0,
                    from: 3,
                    to: 1,
                    item: common::cid("probe:timber"),
                    amount,
                },
                &before,
                &after,
                &op.events,
                None,
            )
            .unwrap();
    }
    advance_accounted_tick(&mut app, &mut oracle, 1);
    let command = produce(882, "probe:train");
    let before = oracle.modeled_state().clone();
    let result = apply_command(app.world_mut(), &command);
    let after = audit_state(app.world_mut());
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &before,
            &after,
            &op.events,
            Some(&result),
        )
        .unwrap();
}

#[test]
fn review_oracle_matches_rejected_nan_command_bits() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let command = Command {
        slot: SlotId(0),
        seq: 80,
        kind: CommandKind::MoveTo {
            unit_ids: vec![3],
            target: turts_sim::Vec2::new(f32::from_bits(0x7fc0_0001), 1.0),
        },
    };
    let result = apply_command(app.world_mut(), &command);
    let op = trail(&mut app).drain_operations().pop().unwrap();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    oracle
        .observe(
            &ExpectedOperation::Command(command),
            &op.before,
            &op.after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    oracle
        .assert_command_records(&trail(&mut app).commands)
        .unwrap();
}

#[test]
fn review_self_convert_and_partial_ratio_headroom_are_accounted() {
    // Self Convert is legal and should win nearest-receiver selection over a farther exchange.
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker.replace("drop_off: None", "drop_off: Some((rules: [(item: \"probe:timber\", effect: Convert(balance: \"probe:token\", per_item: 2))]))");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "pos: (x: 12.0, y: 8.0), contents: []",
        "pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 10)]",
    );
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let cmd = Command {
        slot: SlotId(0),
        seq: 831,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &cmd);
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(cmd),
            &op.before,
            &op.after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    for tick in 1..=300 {
        advance_accounted_tick(&mut app, &mut oracle, tick);
        if oracle.conversion > 0 {
            break;
        }
    }
    assert!(
        oracle.conversion > 0,
        "worker's self Convert rule produces positive accounted conversion"
    );
    assert!(oracle.delivery > 0);

    // A ratio-two receiver with four balance units of headroom can accept exactly two items.
    let mut files = economy_files()
        .into_iter()
        .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace(
        "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
        "drop_off: None",
    );
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker.replace("unload_per_second: 10", "unload_per_second: 1000");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map
        .replace(
            "pos: (x: 12.0, y: 8.0), contents: []",
            "pos: (x: 12.0, y: 20.0), contents: [(item: \"probe:timber\", amount: 10)]",
        )
        .replace(
            "balance: \"probe:token\", amount: 200)]",
            "balance: \"probe:token\", amount: 18446744073709551611)]",
        );
    let mut app = scenario_with(files, 20);
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    let cmd = Command {
        slot: SlotId(0),
        seq: 832,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    };
    let result = apply_command(app.world_mut(), &cmd);
    let op = trail(&mut app).drain_operations().pop().unwrap();
    oracle
        .observe(
            &ExpectedOperation::Command(cmd),
            &op.before,
            &op.after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    let mut converted = false;
    for tick in 1..=40 {
        let before_delivery = oracle.delivery;
        let before_conversion = oracle.conversion;
        advance_accounted_tick(&mut app, &mut oracle, tick);
        if oracle.delivery > before_delivery {
            assert_eq!(
                oracle.delivery - before_delivery,
                2,
                "headroom of four with ratio two permits exactly two items"
            );
            assert_eq!(oracle.conversion - before_conversion, 4);
            converted = true;
            break;
        }
    }
    assert!(converted, "positive partial ratio headroom is usable");
}

#[test]
fn review_automatic_production_event_owner_is_derived_from_producer() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    trail(&mut app).enable();
    let initial = audit_state(app.world_mut());
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let command = produce(833, "probe:train");
    let result = apply_command(app.world_mut(), &command);
    let op = trail(&mut app).drain_operations().pop().unwrap();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    oracle
        .observe(
            &ExpectedOperation::Command(command.clone()),
            &op.before,
            &op.after,
            &op.events,
            Some(&result),
        )
        .unwrap();
    let job = result.outcome.unwrap().job_id.unwrap().0;
    app.update();
    let ops = trail(&mut app).drain_operations();
    oracle
        .observe(
            &ExpectedOperation::Automatic {
                tick: 1,
                phase: AutomaticPhase::Gather,
                subject: AutomaticSubject::Gatherer(3),
            },
            &ops[0].before,
            &ops[0].after,
            &ops[0].events,
            None,
        )
        .unwrap();
    oracle
        .observe(
            &ExpectedOperation::Automatic {
                tick: 1,
                phase: AutomaticPhase::Gather,
                subject: AutomaticSubject::Gatherer(5),
            },
            &ops[1].before,
            &ops[1].after,
            &ops[1].events,
            None,
        )
        .unwrap();
    assert!(matches!(
        ops[2].events[0].kind,
        EconomyEventKind::ProductionStarted { .. }
    ));
    let mut forged = ops[2].clone();
    forged.events[0].owner_slot = 1;
    forged.after.pending_events.last_mut().unwrap().owner_slot = 1;
    assert!(
        oracle
            .observe(
                &ExpectedOperation::Automatic {
                    tick: 1,
                    phase: AutomaticPhase::ProductionAdvance,
                    subject: AutomaticSubject::HeadJob { producer: 1, job }
                },
                &forged.before,
                &forged.after,
                &forged.events,
                None
            )
            .is_err()
    );
}

#[test]
fn e8_authority_and_validation_rejections_match_active_gather_and_production_control() {
    let mut rejected = scenario(20);
    load_package(rejected.world_mut(), &economy_archive()).unwrap();
    let mut control = scenario(20);
    load_package(control.world_mut(), &economy_archive()).unwrap();
    trail(&mut rejected).enable();
    let initial = audit_state(rejected.world_mut());
    let catalog = rejected
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut oracle = AccountingOracle::new(&initial, &catalog);
    for (command, expected_result) in [
        (
            Command {
                slot: SlotId(0),
                seq: 840,
                kind: CommandKind::Gather {
                    unit_id: 3,
                    node_id: turts_sim::NodeId(1),
                },
            },
            true,
        ),
        (produce(841, "probe:train"), true),
        (
            Command {
                slot: SlotId(1),
                seq: 842,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![3],
                    target: turts_sim::Vec2::new(4.0, 4.0),
                },
            },
            false,
        ),
        (
            Command {
                slot: SlotId(0),
                seq: 843,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![u32::MAX],
                    target: turts_sim::Vec2::new(4.0, 4.0),
                },
            },
            false,
        ),
    ] {
        let before = oracle.modeled_state().clone();
        let result = apply_command(rejected.world_mut(), &command);
        if expected_result {
            let control_result = apply_command(control.world_mut(), &command);
            assert_eq!(control_result, result);
        }
        assert_eq!(result.outcome.is_ok(), expected_result);
        let after = audit_state(rejected.world_mut());
        let op = trail(&mut rejected).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(command),
                &before,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
    }
    for tick in 1..=120 {
        advance_accounted_tick(&mut rejected, &mut oracle, tick);
        control.update();
    }
    assert!(
        oracle.extraction > 0,
        "gathering progresses during both rejected-command controls"
    );
    assert!(
        oracle.operations > 100,
        "production and gather operations remain active through the comparison"
    );
    assert_eq!(
        audit_state(rejected.world_mut()),
        audit_state(control.world_mut()),
        "owner and unknown-unit rejections match the active-work no-command control"
    );
}

#[test]
fn e6_gather_carry_and_refund_rounding_are_accounted_at_10_20_30_hz() {
    for hz in [10, 20, 30] {
        let mut files = economy_files()
            .into_iter()
            .map(|(p, b)| (p, String::from_utf8(b).unwrap()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let train = files.get_mut("production/train.ron").unwrap();
        *train = train.replace(
            "queued: (numerator: 1, denominator: 1)",
            "queued: (numerator: 1, denominator: 3)",
        );
        let mut app = scenario_with(files, hz);
        trail(&mut app).enable();
        let initial = audit_state(app.world_mut());
        let catalog = app
            .world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .clone();
        let mut oracle = AccountingOracle::new(&initial, &catalog);
        let gather = Command {
            slot: SlotId(0),
            seq: 850,
            kind: CommandKind::Gather {
                unit_id: 3,
                node_id: turts_sim::NodeId(1),
            },
        };
        let result = apply_command(app.world_mut(), &gather);
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(gather),
                &op.before,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
        for tick in 1..=200u32 {
            advance_accounted_tick(&mut app, &mut oracle, tick);
        }
        assert!(oracle.extraction > 0, "gather rate advances under {hz} Hz");
        let cmd = produce(851, "probe:train");
        let before = oracle.modeled_state().clone();
        let result = apply_command(app.world_mut(), &cmd);
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(cmd),
                &before,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
        let job = result.outcome.unwrap().job_id.unwrap();
        let cancel = Command {
            slot: SlotId(0),
            seq: 852,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        };
        let before = oracle.modeled_state().clone();
        let result = apply_command(app.world_mut(), &cancel);
        let after = audit_state(app.world_mut());
        let op = trail(&mut app).drain_operations().pop().unwrap();
        oracle
            .observe(
                &ExpectedOperation::Command(cancel),
                &before,
                &after,
                &op.events,
                Some(&result),
            )
            .unwrap();
        assert!(op.events.iter().any(|e| matches!(&e.kind, EconomyEventKind::ProductionCancelled { refunded, .. } if refunded.iter().any(|x| x.amount == 33))), "catalog refund ratio rounds 100 tokens to 33 at {hz} Hz");
    }
}
