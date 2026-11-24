//! Privileged operator checks. These rows never enter a player package or projection.
use super::fixture::Fixture;
use turts_sim::terrain::{FLOCK_FIELD_CREDIT, FlockMemberAudit, privileged_flock_audit};

/// The fixed work limits of the flock: the field credit of each owner and 8 neighbours.
pub fn bounded(fixture: &Fixture) {
    let audit = privileged_flock_audit(fixture.app.world());
    let context = format!("{} tick{}", fixture.scene_name, fixture.tick);
    for (owner, work, credit) in &audit.owners {
        assert_eq!(*credit, FLOCK_FIELD_CREDIT, "owner {owner}: {context}");
        assert!(
            work <= credit,
            "field credit of owner {owner}: {work}: {context}"
        );
    }
    for member in &audit.members {
        assert!(
            member.neighbours.len() <= 8,
            "neighbour bound: {member:?}: {context}"
        );
        assert!(
            fixture
                .actors
                .iter()
                .any(|a| a.unit == member.unit && a.slot == member.owner),
            "foreign match/owner membership: {member:?}"
        );
    }
}
/// The flock row of a unit with a move that has not ended.
pub fn member(fixture: &Fixture, label: &str) -> Option<FlockMemberAudit> {
    privileged_flock_audit(fixture.app.world())
        .members
        .into_iter()
        .find(|member| member.unit == fixture.actor(label).unit)
}
/// The guidance field of a unit is the same field (body class and goal).
pub fn same_field_key(before: &FlockMemberAudit, after: &FlockMemberAudit) {
    assert_eq!(before.unit, after.unit);
    assert_eq!(before.owner, after.owner);
    assert_eq!(
        (before.field_class, before.field_goal),
        (after.field_class, after.field_goal),
        "distant field membership changed"
    );
}
/// The guidance field of a distant unit is the same field at the same revision.
pub fn same_field(before: &FlockMemberAudit, after: &FlockMemberAudit) {
    same_field_key(before, after);
    assert_eq!(
        before.field_revision, after.field_revision,
        "distant field revision changed"
    );
}
pub fn no_members(fixture: &Fixture) {
    let audit = privileged_flock_audit(fixture.app.world());
    assert!(
        audit.members.is_empty(),
        "retained cancelled actors: {:?}",
        audit.members
    );
    assert_eq!(audit.moving, 0);
}
