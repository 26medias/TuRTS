use turts_client::commands::{CommandSequence, PendingCommands, SequenceExhausted, submit};
use turts_net_protocol::staged_v2::{JobId, NodeId, PlayerCommandKind};
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
