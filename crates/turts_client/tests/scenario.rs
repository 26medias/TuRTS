mod ingest {
    pub use turts_client::ingest::CommandResult;
}
mod options {
    pub use turts_client::options::ClientOptions;
}
mod replica {
    pub use turts_client::replica::{Replica, UnitReplica};
}
mod commands {
    pub use turts_client::commands::ClientCommand;
}

#[allow(dead_code)] // The integration test exercises the parser and pure driver in isolation.
#[path = "../src/scenario.rs"]
mod scenario;

use scenario::{
    DropWorldChange, TraceAction, TraceCommand, TraceDriver, TraceStep, parse_drop_world_changes,
    parse_trace,
};
use turts_client::commands::ClientCommand;
use turts_client::ingest::CommandResult;
use turts_net_protocol::{CommandRejectReason, JobId, NodeId, PlayerCommand, Pos, UnitId};

fn move_kind() -> ClientCommand {
    ClientCommand::MoveTo {
        unit_ids: vec![UnitId(3)],
        target: Pos { x: 4.0, y: 5.0 },
    }
}

fn move_envelope(seq: u32) -> PlayerCommand {
    PlayerCommand {
        seq,
        order: turts_net_protocol::OrderEnvelope {
            order: "muster:order_move".into(),
            units: vec![UnitId(3)],
            target: turts_net_protocol::OrderTarget::Point(Pos { x: 4.0, y: 5.0 }),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        },
    }
}

fn produce_envelope(seq: u32) -> PlayerCommand {
    PlayerCommand {
        seq,
        order: turts_net_protocol::OrderEnvelope {
            order: "base:order_produce".into(),
            units: vec![UnitId(8)],
            target: turts_net_protocol::OrderTarget::Content("base:hire".into()),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        },
    }
}

fn trace_step(after_ticks: u32, command: TraceCommand, require_result: bool) -> TraceStep {
    TraceStep {
        after_ticks,
        command,
        require_result,
    }
}

#[test]
fn parser_round_trips_typed_trace_steps() {
    let steps = vec![trace_step(2, TraceCommand::Submit(move_kind()), true)];
    let json = serde_json::to_string(&steps).unwrap();
    assert_eq!(parse_trace(&json).unwrap(), steps);
}

#[test]
fn parser_rejects_malformed_json() {
    assert!(parse_trace("[{bad json]").is_err());
}

#[test]
fn required_result_waits_for_matching_applied_or_rejected_sequence() {
    let steps = vec![
        trace_step(1, TraceCommand::Submit(move_kind()), true),
        trace_step(
            2,
            TraceCommand::Submit(ClientCommand::Gather {
                unit_id: UnitId(3),
                node_id: NodeId(9),
            }),
            true,
        ),
    ];
    let mut driver = TraceDriver::new(steps, 10).unwrap();
    assert!(driver.update(10, &[]).unwrap().is_empty());
    let action = driver.update(11, &[]).unwrap();
    assert!(matches!(
        action.as_slice(),
        [TraceAction::Submit { step_index: 0, .. }]
    ));
    driver
        .record_sent(0, move_envelope(40), Some(move_kind()), 0)
        .unwrap();

    assert!(
        driver
            .update(11, &[CommandResult::Acked { seq: 40, tick: 11 }])
            .unwrap()
            .is_empty()
    );
    assert!(
        driver
            .update(
                11,
                &[CommandResult::Applied {
                    seq: 41,
                    tick: 11,
                    job_id: None,
                }],
            )
            .unwrap()
            .is_empty()
    );
    assert!(
        driver
            .update(
                11,
                &[CommandResult::Rejected {
                    seq: 40,
                    tick: 12,
                    reason: CommandRejectReason::UnknownUnit,
                }],
            )
            .unwrap()
            .is_empty()
    );
    assert!(driver.update(13, &[]).unwrap().is_empty());
    let next = driver
        .update(
            14,
            &[CommandResult::Applied {
                seq: 40,
                tick: 12,
                job_id: None,
            }],
        )
        .unwrap();
    assert!(matches!(
        next.as_slice(),
        [TraceAction::Submit { step_index: 1, .. }]
    ));
}

#[test]
fn cancel_resolves_only_an_earlier_applied_production_job() {
    let steps = vec![
        trace_step(
            0,
            TraceCommand::Submit(ClientCommand::Produce {
                producer_id: UnitId(8),
                recipe_id: "base:hire".to_owned(),
            }),
            true,
        ),
        trace_step(
            0,
            TraceCommand::CancelAppliedJob {
                producer_id: UnitId(8),
                produced_by_step: 0,
            },
            true,
        ),
    ];
    let mut driver = TraceDriver::new(steps, 7).unwrap();
    let first = driver.update(7, &[]).unwrap();
    assert!(matches!(
        first.as_slice(),
        [TraceAction::Submit { step_index: 0, .. }]
    ));
    driver
        .record_sent(
            0,
            produce_envelope(9),
            Some(ClientCommand::Produce {
                producer_id: UnitId(8),
                recipe_id: "base:hire".to_owned(),
            }),
            0,
        )
        .unwrap();
    let cancel = driver
        .update(
            8,
            &[CommandResult::Applied {
                seq: 9,
                tick: 8,
                job_id: Some(JobId(55)),
            }],
        )
        .unwrap();
    assert!(matches!(
        cancel.as_slice(),
        [TraceAction::Submit {
            step_index: 1,
            kind: ClientCommand::CancelProduction {
                producer_id: UnitId(8),
                job_id: JobId(55),
            },
        }]
    ));
}

#[test]
fn cancel_fails_after_rejected_production_reference() {
    let steps = vec![
        trace_step(
            0,
            TraceCommand::Submit(ClientCommand::Produce {
                producer_id: UnitId(8),
                recipe_id: "base:hire".to_owned(),
            }),
            true,
        ),
        trace_step(
            0,
            TraceCommand::CancelAppliedJob {
                producer_id: UnitId(8),
                produced_by_step: 0,
            },
            true,
        ),
    ];
    let mut driver = TraceDriver::new(steps, 7).unwrap();
    driver.update(7, &[]).unwrap();
    driver
        .record_sent(
            0,
            produce_envelope(9),
            Some(ClientCommand::Produce {
                producer_id: UnitId(8),
                recipe_id: "base:hire".to_owned(),
            }),
            0,
        )
        .unwrap();
    assert!(
        driver
            .update(
                8,
                &[CommandResult::Rejected {
                    seq: 9,
                    tick: 8,
                    reason: CommandRejectReason::UnknownRecipe,
                }],
            )
            .is_err()
    );
}

#[test]
fn cancel_rejects_a_reference_that_is_not_an_earlier_required_produce() {
    let steps = vec![
        trace_step(0, TraceCommand::Submit(move_kind()), true),
        trace_step(
            0,
            TraceCommand::CancelAppliedJob {
                producer_id: UnitId(8),
                produced_by_step: 0,
            },
            true,
        ),
    ];
    assert!(TraceDriver::new(steps, 0).is_err());
}

#[test]
fn repeat_waits_for_the_original_terminal_then_requires_a_new_terminal() {
    let steps = vec![
        trace_step(0, TraceCommand::Submit(move_kind()), false),
        trace_step(0, TraceCommand::RepeatLastEnvelope, true),
    ];
    let mut driver = TraceDriver::new(steps, 3).unwrap();
    let first = driver.update(3, &[]).unwrap();
    assert!(matches!(
        first.as_slice(),
        [TraceAction::Submit { step_index: 0, .. }]
    ));
    let actual = move_envelope(77);
    driver
        .record_sent(0, actual.clone(), Some(move_kind()), 0)
        .unwrap();
    assert!(driver.update(3, &[]).unwrap().is_empty());
    let original_applied = vec![CommandResult::Applied {
        seq: 77,
        tick: 3,
        job_id: None,
    }];
    let repeated = driver.update(3, &original_applied).unwrap();
    assert!(matches!(
        repeated.as_slice(),
        [TraceAction::RepeatLastEnvelope {
            step_index: 1,
            envelope,
        }] if envelope == &actual
    ));
    let original_applied = vec![CommandResult::Applied {
        seq: 77,
        tick: 3,
        job_id: None,
    }];
    driver
        .record_sent(1, actual.clone(), None, original_applied.len())
        .unwrap();
    assert!(driver.update(3, &original_applied).unwrap().is_empty());
    assert!(
        !driver.is_complete(),
        "the original Applied cannot fulfill the repeat"
    );
    let mut new_results = original_applied;
    new_results.push(CommandResult::Rejected {
        seq: 77,
        tick: 4,
        reason: CommandRejectReason::BadSequence,
    });
    assert!(driver.update(4, &new_results).unwrap().is_empty());
    assert!(driver.is_complete());
}

#[test]
fn repeat_can_be_scheduled_in_the_original_result_update() {
    let steps = vec![
        trace_step(0, TraceCommand::Submit(move_kind()), true),
        trace_step(0, TraceCommand::RepeatLastEnvelope, true),
    ];
    let mut driver = TraceDriver::new(steps, 3).unwrap();
    driver.update(3, &[]).unwrap();
    let actual = move_envelope(78);
    driver
        .record_sent(0, actual.clone(), Some(move_kind()), 0)
        .unwrap();
    let original_applied = vec![CommandResult::Applied {
        seq: 78,
        tick: 3,
        job_id: None,
    }];
    let repeated = driver.update(3, &original_applied).unwrap();
    assert!(matches!(
        repeated.as_slice(),
        [TraceAction::RepeatLastEnvelope {
            step_index: 1,
            envelope,
        }] if envelope == &actual
    ));
    driver
        .record_sent(1, actual.clone(), None, original_applied.len())
        .unwrap();
    assert!(driver.update(3, &original_applied).unwrap().is_empty());
    assert!(!driver.is_complete());
    let mut results = original_applied;
    results.push(CommandResult::Rejected {
        seq: 78,
        tick: 4,
        reason: CommandRejectReason::BadSequence,
    });
    driver.update(4, &results).unwrap();
    assert!(driver.is_complete());
}

#[test]
fn arm_is_local_and_following_zero_offset_submit_is_same_update() {
    let steps = vec![
        trace_step(
            2,
            TraceCommand::ArmDropWorldChanges {
                kinds: vec![DropWorldChange::Extraction],
            },
            false,
        ),
        trace_step(0, TraceCommand::Submit(move_kind()), true),
    ];
    let mut driver = TraceDriver::new(steps, 10).unwrap();
    let actions = driver.update(12, &[]).unwrap();
    assert!(matches!(
        actions.as_slice(),
        [
            TraceAction::ArmDropWorldChanges {
                step_index: 0,
                arm_tick: 12,
                kinds,
            },
            TraceAction::Submit { step_index: 1, .. }
        ] if kinds == &[DropWorldChange::Extraction]
    ));
}

#[test]
fn arm_requires_no_result_and_zero_offset_following_action() {
    let invalid_result = vec![trace_step(
        0,
        TraceCommand::ArmDropWorldChanges {
            kinds: vec![DropWorldChange::Spawn],
        },
        true,
    )];
    assert!(TraceDriver::new(invalid_result, 0).is_err());
    let invalid_followup = vec![
        trace_step(
            0,
            TraceCommand::ArmDropWorldChanges {
                kinds: vec![DropWorldChange::Spawn],
            },
            false,
        ),
        trace_step(1, TraceCommand::Submit(move_kind()), true),
    ];
    assert!(TraceDriver::new(invalid_followup, 0).is_err());
}

#[test]
fn drop_world_change_parser_accepts_only_unique_known_classes() {
    assert_eq!(
        parse_drop_world_changes("extraction,conversion,charge,refund,spawn,final_change").unwrap(),
        vec![
            DropWorldChange::Extraction,
            DropWorldChange::Conversion,
            DropWorldChange::Charge,
            DropWorldChange::Refund,
            DropWorldChange::Spawn,
            DropWorldChange::FinalChange,
        ]
    );
    assert!(parse_drop_world_changes("").is_err());
    assert!(parse_drop_world_changes("charge,charge").is_err());
    assert!(parse_drop_world_changes("teleport").is_err());
}
