use turts_net_protocol::CommandRejectReason;

#[test]
fn wire_roundtrip_new_reasons() {
    let expected = [
        (CommandRejectReason::NotRemovable, "not_removable"),
        (CommandRejectReason::NotToggleable, "not_toggleable"),
        (
            CommandRejectReason::PrerequisitesMissing,
            "prerequisites_missing",
        ),
        (CommandRejectReason::AlreadyResearched, "already_researched"),
        (
            CommandRejectReason::ResearchInProgress,
            "research_in_progress",
        ),
        (CommandRejectReason::Internal, "internal"),
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
        let encoded = postcard::to_vec::<CommandRejectReason, 16>(&reason).unwrap();
        assert_eq!(
            postcard::from_bytes::<CommandRejectReason>(&encoded),
            Ok(reason)
        );
        assert!(CommandRejectReason::ALL.contains(&reason));
    }
}
