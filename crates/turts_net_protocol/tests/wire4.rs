use turts_net_protocol::{
    CommandRejectReason, JobId, NodeId, OrderEnvelope, OrderResult, OrderTarget, QueueMode, UnitId,
    UnitRefusal,
};

#[test]
fn every_order_target_round_trips() {
    let targets = [
        OrderTarget::None,
        OrderTarget::Point(turts_net_protocol::Pos { x: 1.25, y: -8.5 }),
        OrderTarget::Unit(UnitId(17)),
        OrderTarget::Node(NodeId(23)),
        OrderTarget::Job(JobId(31)),
        OrderTarget::Content("ore:iron".to_owned()),
    ];

    for target in targets {
        let envelope = OrderEnvelope {
            order: "forge:move".to_owned(),
            units: vec![UnitId(2), UnitId(4)],
            target,
            queue: QueueMode::Append,
            group: 9,
        };
        let encoded = postcard::to_vec::<_, 4096>(&envelope).unwrap();
        let decoded: OrderEnvelope = postcard::from_bytes(&encoded).unwrap();
        assert_eq!(decoded, envelope, "target: {:?}", envelope.target);
    }
}

#[test]
fn maximum_varint_unit_envelope_fits_postcard_message() {
    let envelope = OrderEnvelope {
        order: "forge:order_move".to_owned(),
        units: vec![UnitId(u32::MAX); turts_net_protocol::MAX_ORDER_UNITS],
        target: OrderTarget::None,
        queue: QueueMode::Replace,
        group: u32::MAX,
    };

    let encoded = postcard::to_vec::<_, 32_768>(&envelope).expect("4096-unit envelope serializes");
    assert!(
        encoded.len() <= turts_net_protocol::MAX_ORDER_UNITS * 5 + 64,
        "encoded {} bytes",
        encoded.len()
    );
    let decoded: OrderEnvelope = postcard::from_bytes(&encoded).expect("envelope decodes");
    assert_eq!(decoded, envelope);
}

#[test]
fn order_result_round_trips_with_refusals() {
    let result = OrderResult {
        seq: 12,
        tick: 640,
        accepted: vec![UnitId(3)],
        refused: vec![UnitRefusal {
            unit: UnitId(5),
            reason: CommandRejectReason::OutOfRange,
        }],
        envelope: None,
        job_id: Some(JobId(99)),
    };

    let encoded = postcard::to_vec::<_, 4096>(&result).unwrap();
    let decoded: OrderResult = postcard::from_bytes(&encoded).unwrap();
    assert_eq!(decoded, result);
}

#[test]
fn new_reason_names_are_stable() {
    let expected = [
        (CommandRejectReason::UnknownOrder, "unknown_order"),
        (CommandRejectReason::NotApplicable, "not_applicable"),
        (CommandRejectReason::TargetInvalid, "target_invalid"),
        (CommandRejectReason::TargetGone, "target_gone"),
        (CommandRejectReason::OutOfRange, "out_of_range"),
        (CommandRejectReason::RateLimited, "rate_limited"),
        (CommandRejectReason::NotQueueable, "not_queueable"),
        (CommandRejectReason::Blocked, "blocked"),
    ];

    for (reason, name) in expected {
        assert_eq!(reason.as_str(), name);
    }
}
