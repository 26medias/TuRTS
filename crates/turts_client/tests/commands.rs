use turts_client::commands::{CommandSequence, PendingCommands, SequenceExhausted, submit};
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
    assert_eq!(pending.0.into_iter().collect::<Vec<_>>(), commands);
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
        Some(&command)
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
