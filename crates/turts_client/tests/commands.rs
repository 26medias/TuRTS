use turts_client::commands::{
    CommandSequence, PendingCommand, PendingCommands, SequenceExhausted, submit,
};
use turts_net_protocol::{JobId, NodeId, PlayerCommandKind};
use turts_net_protocol::{Pos, UnitId};

#[test]
fn one_sequence_covers_move_gather_produce_cancel() {
    let mut pending = PendingCommands::default();
    submit(
        &mut pending,
        PlayerCommandKind::MoveTo {
            unit_ids: vec![UnitId(1)],
            target: Pos { x: 2.0, y: 3.0 },
        },
    );
    submit(
        &mut pending,
        PlayerCommandKind::Gather {
            unit_id: UnitId(2),
            node_id: NodeId(3),
        },
    );
    submit(
        &mut pending,
        PlayerCommandKind::Produce {
            producer_id: UnitId(4),
            recipe_id: "base:hire".to_owned(),
        },
    );
    submit(
        &mut pending,
        PlayerCommandKind::CancelProduction {
            producer_id: UnitId(4),
            job_id: JobId(5),
        },
    );
    let mut sequence = CommandSequence::default();
    let allocated: Vec<_> = pending
        .0
        .iter()
        .map(|_| sequence.allocate().expect("sequence available"))
        .collect();
    assert_eq!(allocated, vec![1, 2, 3, 4]);
}

#[test]
fn pending_commands_preserve_fifo_order() {
    let mut pending = PendingCommands::default();
    let commands = [
        PlayerCommandKind::Gather {
            unit_id: UnitId(1),
            node_id: NodeId(10),
        },
        PlayerCommandKind::Produce {
            producer_id: UnitId(2),
            recipe_id: "base:hire".to_owned(),
        },
        PlayerCommandKind::CancelProduction {
            producer_id: UnitId(2),
            job_id: JobId(20),
        },
    ];
    for command in commands.clone() {
        submit(&mut pending, command);
    }
    assert_eq!(
        pending
            .0
            .into_iter()
            .map(|command| match command {
                PendingCommand::Submit {
                    kind,
                    trace_step: None,
                } => kind,
                other => panic!("unexpected pending command {other:?}"),
            })
            .collect::<Vec<_>>(),
        commands
    );
}

#[test]
fn max_sequence_is_allocated_once_then_remains_exhausted() {
    let mut sequence = CommandSequence {
        next: Some(u32::MAX),
    };
    assert_eq!(sequence.allocate(), Ok(u32::MAX));
    assert_eq!(sequence.allocate(), Err(SequenceExhausted));
    assert_eq!(sequence.allocate(), Err(SequenceExhausted));
}

#[test]
fn fresh_sequence_starts_at_one() {
    let mut sequence = CommandSequence::default();
    assert_eq!(sequence.allocate(), Ok(1));
}

#[test]
fn ack_precedes_rejection_and_applied_is_distinct() {
    use turts_client::ingest::{CommandResult, ReceiptKind, ReceiptWitness, receipt_ordered};
    use turts_net_protocol::JobId;
    let ack = ReceiptWitness {
        connection_epoch: 7,
        seq: 12,
        kind: ReceiptKind::Ack,
        reliable: true,
        message_id: Some(8),
        remote_tick: 10,
    };
    let rejected = ReceiptWitness {
        connection_epoch: 7,
        seq: 12,
        kind: ReceiptKind::Rejected,
        reliable: true,
        message_id: Some(9),
        remote_tick: 10,
    };
    assert!(receipt_ordered(ack, rejected, 2));
    let applied = CommandResult::Applied {
        seq: 12,
        tick: 11,
        job_id: Some(JobId(5)),
    };
    assert_eq!(
        applied,
        CommandResult::Applied {
            seq: 12,
            tick: 11,
            job_id: Some(JobId(5))
        }
    );
}

#[test]
fn send_reversal_plant_fails_witness_even_when_typed_drain_is_ack_first() {
    use turts_client::ingest::{CommandResult, ReceiptKind, ReceiptWitness, receipt_ordered};
    let typed_drain = [
        CommandResult::Acked { seq: 4, tick: 1 },
        CommandResult::Rejected {
            seq: 4,
            tick: 1,
            reason: turts_net_protocol::CommandRejectReason::BadSequence,
        },
    ];
    assert!(matches!(typed_drain[0], CommandResult::Acked { .. }));
    let ack_received = ReceiptWitness {
        connection_epoch: 3,
        seq: 4,
        kind: ReceiptKind::Ack,
        reliable: true,
        message_id: Some(42),
        remote_tick: 1,
    };
    let rejected_received = ReceiptWitness {
        connection_epoch: 3,
        seq: 4,
        kind: ReceiptKind::Rejected,
        reliable: true,
        message_id: Some(41),
        remote_tick: 1,
    };
    assert!(!receipt_ordered(ack_received, rejected_received, 2));
    assert!(!receipt_ordered(ack_received, rejected_received, 32768));
}

#[test]
fn disconnected_sender_preserves_fifo_and_sequence_until_connected() {
    use bevy::prelude::*;
    use lightyear::prelude::*;
    use turts_client::commands::{CommandSendStatus, send_commands};
    use turts_client::ingest::{ReceiptWitnesses, ReplicaState};
    use turts_client::join::Phase;
    use turts_net_protocol::PlayerCommand;

    let mut app = App::new();
    app.insert_resource(PendingCommands::default())
        .insert_resource(CommandSequence::default())
        .insert_resource(CommandSendStatus::default())
        .insert_resource(Phase::Joined { slot: 0 })
        .insert_resource(ReplicaState::default())
        .insert_resource(ReceiptWitnesses::default())
        .add_systems(Update, send_commands);
    let disconnected = app
        .world_mut()
        .spawn(MessageSender::<PlayerCommand>::default())
        .id();
    let command = PlayerCommandKind::Gather {
        unit_id: UnitId(1),
        node_id: NodeId(2),
    };
    submit(
        &mut app.world_mut().resource_mut::<PendingCommands>(),
        command.clone(),
    );

    app.update();
    assert_eq!(
        app.world().resource::<PendingCommands>().0.front(),
        Some(&PendingCommand::Submit {
            kind: command.clone(),
            trace_step: None,
        })
    );
    assert_eq!(app.world().resource::<CommandSequence>().next, Some(1));

    app.world_mut()
        .entity_mut(disconnected)
        .insert((RemoteId(PeerId::Netcode(5)), Connected));
    app.update();
    assert!(app.world().resource::<PendingCommands>().0.is_empty());
    assert_eq!(app.world().resource::<CommandSequence>().next, Some(2));
    app.update();
    assert_eq!(app.world().resource::<CommandSequence>().next, Some(2));
}

#[test]
#[ignore = "requires a fresh actual p4b artifact via TURTS_RECEIPT_WITNESS_REPORT and run ID"]
fn captured_transport_reversal_fails_product_receipt_ordering_helper() {
    use std::fs;
    use turts_client::ingest::{ReceiptKind, ReceiptWitness, ReceiptWitnesses, receipt_ordered};

    let path = std::env::var("TURTS_RECEIPT_WITNESS_REPORT")
        .expect("run p4b first and set TURTS_RECEIPT_WITNESS_REPORT to its fresh JSON output");
    let expected_run_id = std::env::var("TURTS_RECEIPT_WITNESS_RUN_ID")
        .expect("set TURTS_RECEIPT_WITNESS_RUN_ID to the producer run id");
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read captured transport report"))
            .expect("captured report is JSON");
    assert_eq!(report["run_id"], expected_run_id);
    assert_eq!(
        report["typed_command_results"],
        serde_json::json!(["ack", "rejected"])
    );

    let epoch = report["connection_epoch"]
        .as_u64()
        .expect("captured connection epoch");
    let reliable_count = report["reliable_message_count"]
        .as_u64()
        .expect("captured reliable count") as u32;
    assert!((2..32768).contains(&reliable_count));
    let records = report["receipt_witnesses"]
        .as_array()
        .expect("captured receipt witness array");
    let mut receipts = ReceiptWitnesses::default();
    for record in records {
        let kind = match record["kind"].as_str().expect("captured kind") {
            "ack" => ReceiptKind::Ack,
            "applied" => ReceiptKind::Applied,
            "rejected" => ReceiptKind::Rejected,
            _ => continue,
        };
        assert_eq!(record["connection_epoch"].as_u64(), Some(epoch));
        assert!(
            record["reliable"]
                .as_bool()
                .expect("captured reliable flag")
        );
        assert!(
            record["message_id"].as_u64().is_some(),
            "captured reliable MessageId required"
        );
        assert!(
            record["remote_tick"].as_u64().is_some(),
            "captured remote tick required"
        );
        receipts.records.push(ReceiptWitness {
            connection_epoch: epoch,
            seq: record["seq"].as_u64().expect("captured sequence") as u32,
            kind,
            reliable: true,
            message_id: record["message_id"].as_u64().map(|id| id as u16),
            remote_tick: record["remote_tick"]
                .as_u64()
                .expect("captured remote tick") as u32,
        });
    }
    receipts.reliable_counts.insert(epoch, reliable_count);
    let ack = receipts
        .records
        .iter()
        .find(|record| record.kind == ReceiptKind::Ack)
        .copied()
        .expect("captured Ack");
    let rejected = receipts
        .records
        .iter()
        .find(|record| record.kind == ReceiptKind::Rejected)
        .copied()
        .expect("captured Rejected");
    assert_eq!(
        ack.seq, rejected.seq,
        "captured results correspond to one command sequence"
    );
    assert!(
        !receipt_ordered(ack, rejected, reliable_count),
        "product receipt helper must reject actual server-send reversal metadata"
    );
    assert_eq!(receipts.ordered_for(ack.seq, epoch), Some(false));
}
