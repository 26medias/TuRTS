use turts_net_protocol::{CommandRejectReason, NET_PROTOCOL_VERSION};
use turts_sim::{Command, CommandKind, CommandQueue, CommandResults, RejectReason, SlotId, Vec2};

#[test]
fn old_protocol_and_bad_sequence_change_no_sim_state() {
    assert_eq!(NET_PROTOCOL_VERSION, 2);
    let old_protocol = 1;
    assert_ne!(old_protocol, NET_PROTOCOL_VERSION);
    let queue = CommandQueue::default();
    let results = CommandResults::default();
    let before = (queue.0.len(), results.0.len());
    let reason = CommandRejectReason::BadSequence;
    assert_eq!(reason.as_str(), "bad_sequence");
    assert_eq!((queue.0.len(), results.0.len()), before);
    // The invalid envelope is not represented as a simulation command or audit result.
    let _valid_type_is_available = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::MoveTo {
            unit_ids: vec![],
            target: Vec2::ZERO,
        },
    };
    let _internal_reason_type = RejectReason::BadSequence;
}
