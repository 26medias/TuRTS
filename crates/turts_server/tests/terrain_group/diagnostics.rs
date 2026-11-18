//! Privileged operator checks. These rows never enter a player package or projection.
use super::fixture::Fixture;
use turts_sim::terrain::{NavigationMemberAudit, privileged_navigation_audit};

pub fn bounded(fixture: &Fixture) {
    let audit = privileged_navigation_audit(fixture.app.world());
    let d = &audit.diagnostics;
    let context = format!("{} tick{}: {d:?}", fixture.scene_name, fixture.tick);
    assert!(d.resident_fields <= 32, "field cap: {context}");
    assert!(
        d.resident_bytes.checked_add(d.retired_bytes).unwrap() <= 64 * 1024 * 1024,
        "guidance capacity including retirement: {context}"
    );
    assert!(d.expansions_this_tick <= 128, "expansion bound: {context}");
    assert!(d.edge_checks_this_tick <= 1024, "edge bound: {context}");
    assert!(
        d.admission_entries_this_tick <= 4096,
        "admission bound: {context}"
    );
    assert!(
        d.invalidation_entries_this_tick <= 4096,
        "invalidation bound: {context}"
    );
    assert!(
        d.retired_pages_this_tick <= 16,
        "retirement bound: {context}"
    );
    assert_eq!(
        d.stale_activations, 0,
        "stale guidance activated: {context}"
    );
    assert!(
        d.neighbor_visits_this_tick <= 65_536,
        "neighbor work bound: {context}"
    );
    assert!(
        d.neighbor_bucket_visits_this_tick <= 65_536,
        "neighbor bookkeeping bound: {context}"
    );
    assert!(
        d.body_pair_checks_this_tick <= 65_536,
        "complete body predicate bound: {context}"
    );
    assert!(
        d.pending <= d.memberships,
        "pending actors exceed memberships: {context}"
    );
    for member in &audit.members {
        assert!(
            fixture
                .actors
                .iter()
                .any(|a| a.unit == member.unit && a.slot == member.viewer),
            "foreign match/viewer membership: {member:?}"
        );
    }
}
pub fn member(fixture: &Fixture, label: &str) -> NavigationMemberAudit {
    privileged_navigation_audit(fixture.app.world())
        .members
        .into_iter()
        .find(|member| member.unit == fixture.actor(label).unit)
        .unwrap_or_else(|| panic!("missing active route for {label} tick{}", fixture.tick))
}
pub fn same_identity(before: &NavigationMemberAudit, after: &NavigationMemberAudit) {
    assert_eq!(before.unit, after.unit);
    assert_eq!(before.viewer, after.viewer);
    assert_eq!(
        before.order_epoch, after.order_epoch,
        "distant route order epoch changed"
    );
    assert_eq!(
        before.field, after.field,
        "distant route membership changed"
    );
    assert_eq!(
        before.field_revision, after.field_revision,
        "distant route revision changed"
    );
    assert_eq!(
        before.endpoint.map(f32::to_bits),
        after.endpoint.map(f32::to_bits)
    );
}
pub fn same_lease(before: &NavigationMemberAudit, after: &NavigationMemberAudit) {
    same_identity(before, after);
    assert_eq!(
        before.dependencies, after.dependencies,
        "distant route dependency leases changed"
    );
}
pub fn same_retained_revisions(before: &NavigationMemberAudit, after: &NavigationMemberAudit) {
    same_identity(before, after);
    // Normal reverse expansion can add dependencies; a traversed prefix can
    // cease to be required. Shared retained regions must keep their revisions.
    for &(chunk, revision) in &before.dependencies {
        if let Some((_, current)) = after.dependencies.iter().find(|(key, _)| *key == chunk) {
            assert_eq!(*current, revision, "unrelated cached geometry changed");
        }
    }
}
pub fn no_members(fixture: &Fixture) {
    let audit = privileged_navigation_audit(fixture.app.world());
    assert!(
        audit.members.is_empty(),
        "retained cancelled actors: {:?}",
        audit.members
    );
    assert_eq!(audit.diagnostics.memberships, 0);
    assert_eq!(audit.diagnostics.pending, 0);
}
