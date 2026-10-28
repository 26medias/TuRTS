use turts_sim::RejectReason;

#[test]
fn m3_reject_reasons_have_stable_names() {
    let cases = [
        (RejectReason::NotRemovable, "not_removable"),
        (RejectReason::NotToggleable, "not_toggleable"),
        (RejectReason::PrerequisitesMissing, "prerequisites_missing"),
        (RejectReason::AlreadyResearched, "already_researched"),
        (RejectReason::ResearchInProgress, "research_in_progress"),
        (RejectReason::IntegrityViolation, "integrity_violation"),
    ];

    for (reason, expected) in cases {
        assert_eq!(reason.as_str(), expected);
    }
}
