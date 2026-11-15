//! Borrow the authoritative view. Materialize only permitted public geometry and owned records.
use super::*;
use crate::permissions::{ObservationPermissions, unit_permitted};
pub(super) struct Geometry {
    units: Vec<u32>,
    nodes: Vec<u32>,
}
impl Geometry {
    pub(super) fn unit(&self, id: u32) -> bool {
        self.units.binary_search(&id).is_ok()
    }
    pub(super) fn node(&self, id: u32) -> bool {
        self.nodes.binary_search(&id).is_ok()
    }
    fn summary(&self, summary: &wire::OrderSummary) -> bool {
        match summary.target {
            wire::OrderTarget::Unit(id) => self.unit(id.0),
            wire::OrderTarget::Node(id) => self.node(id.0),
            _ => true,
        }
    }
}
pub(super) fn snapshot(
    view: &WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
) -> wire::Snapshot {
    snapshot_reusing(view, slot, policy, None)
}
pub(super) fn snapshot_reusing(
    view: &WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
    reuse: Option<&mut [wire::UnitSnapshot]>,
) -> wire::Snapshot {
    if policy.all_public_geometry(slot.0) {
        return super::snapshot_projected_reusing(view, slot, None, reuse);
    }
    let mut geometry = Geometry {
        units: view
            .units
            .iter()
            .filter(|u| unit_permitted(policy, slot.0, u.unit_id, u.owner_slot))
            .map(|u| u.unit_id)
            .collect(),
        nodes: view
            .nodes
            .iter()
            .filter(|n| policy.node_visible(slot.0, n.id.0))
            .map(|n| n.id.0)
            .collect(),
    };
    geometry.units.sort_unstable();
    geometry.nodes.sort_unstable();
    let mut state = super::snapshot_projected_reusing(view, slot, Some(&geometry), reuse);
    for row in &mut state.weapons {
        row.last_target = row.last_target.filter(|id| geometry.unit(id.0));
    }
    for row in &mut state.inserters {
        row.source = row.source.filter(|id| geometry.unit(id.0));
        row.destination = row.destination.filter(|id| geometry.unit(id.0));
    }
    for row in &mut state.extractors {
        row.node = row.node.filter(|id| geometry.node(id.0));
    }
    for row in &mut state.economy.orders {
        let censored = row.node.is_some_and(|id| !geometry.node(id.0))
            || row.receiver.is_some_and(|id| !geometry.unit(id.0));
        row.node = row.node.filter(|id| geometry.node(id.0));
        row.receiver = row.receiver.filter(|id| geometry.unit(id.0));
        if censored {
            row.stage = wire::GatherStage::Idle;
        }
    }
    for row in &mut state.unit_orders {
        if row
            .current
            .as_ref()
            .is_some_and(|summary| !geometry.summary(summary))
        {
            row.current = None;
            row.activity = wire::Activity::Idle;
        }
        row.queued.retain(|summary| geometry.summary(summary));
    }
    state
        .records
        .repairs
        .retain(|row| geometry.unit(row.repairer.0) && geometry.unit(row.target.0));
    state
}
