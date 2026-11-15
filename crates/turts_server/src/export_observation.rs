//! Permission projection before wire serialization. The shared simulation provider supplies sight.
//! Atomic fog transitions and remembered records are grouped in export_visibility.

use crate::permissions::{ObservationPermissions, unit_permitted};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use turts_sim::economy::views::{UnitOrderSummaryView, UnitOrderTargetView};
use turts_sim::{CommittedTick, SlotId, WorldView};

/// Last transmitted permitted geometry. It contains no hidden identifiers or owners.
#[derive(bevy::prelude::Component, Clone, Default)]
pub struct ObservationHistory {
    units: BTreeMap<u32, u8>,
    nodes: BTreeSet<u32>,
    restricted: bool,
    baseline: Option<Box<turts_net_protocol::Snapshot>>,
}

/// A membership change needs an absolute reliable baseline. Absence is not a death event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservationChanged;

impl ObservationHistory {
    fn capture(view: &WorldView) -> Self {
        Self {
            units: view
                .units
                .iter()
                .map(|u| (u.unit_id, u.owner_slot))
                .collect(),
            nodes: view.nodes.iter().map(|n| n.id.0).collect(),
            restricted: true,
            baseline: None,
        }
    }
}

fn summary_visible(
    summary: &UnitOrderSummaryView,
    units: &BTreeSet<u32>,
    nodes: &BTreeSet<u32>,
) -> bool {
    match summary.target {
        UnitOrderTargetView::Unit(id) => units.contains(&id),
        UnitOrderTargetView::Node(id) => nodes.contains(&id.0),
        _ => true,
    }
}

pub(super) fn project_view<'a>(
    view: &'a WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
) -> Cow<'a, WorldView> {
    if policy.all_public_geometry(slot.0) {
        return Cow::Borrowed(view);
    }
    let mut projected = view.clone();
    projected
        .units
        .retain(|u| unit_permitted(policy, slot.0, u.unit_id, u.owner_slot));
    projected
        .nodes
        .retain(|n| policy.node_visible(slot.0, n.id.0));
    let units = projected
        .units
        .iter()
        .map(|u| u.unit_id)
        .collect::<BTreeSet<_>>();
    let nodes = projected
        .nodes
        .iter()
        .map(|n| n.id.0)
        .collect::<BTreeSet<_>>();
    projected.m3.raw_healths.retain(|r| units.contains(&r.unit));
    for row in &mut projected.m3.raw_weapons {
        row.last_target = row.last_target.filter(|id| units.contains(id));
        row.ordered_target_fired = row.ordered_target_fired.filter(|id| units.contains(id));
    }
    for row in &mut projected.m3.raw_inserters {
        row.source = row.source.filter(|id| units.contains(id));
        row.destination = row.destination.filter(|id| units.contains(id));
    }
    for row in &mut projected.m3.raw_extractors {
        row.node = row.node.filter(|id| nodes.contains(id));
    }
    for row in &mut projected.economy.orders {
        let censored = row.node.is_some_and(|id| !nodes.contains(&id.0))
            || row.receiver.is_some_and(|id| !units.contains(&id));
        row.node = row.node.filter(|id| nodes.contains(&id.0));
        row.receiver = row.receiver.filter(|id| units.contains(id));
        if censored {
            row.stage = turts_sim::economy::gather::GatherStage::Idle;
        }
    }
    for row in &mut projected.unit_orders {
        if row
            .current
            .as_ref()
            .is_some_and(|s| !summary_visible(s, &units, &nodes))
        {
            row.current = None;
            row.activity = turts_sim::economy::views::UnitOrderActivityView::Idle;
        }
        row.queued.retain(|s| summary_visible(s, &units, &nodes));
    }
    projected
        .economy
        .repairs
        .retain(|r| units.contains(&r.repairer) && units.contains(&r.target));
    Cow::Owned(projected)
}

pub(super) fn snapshot(
    view: &WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
) -> (turts_net_protocol::Snapshot, ObservationHistory) {
    let projected = project_view(view, slot, policy);
    let history = if policy.all_public_geometry(slot.0) {
        ObservationHistory::default()
    } else {
        ObservationHistory::capture(&projected)
    };
    let wire = super::snapshot_for(&projected, slot);
    let mut history = history;
    if history.restricted {
        history.baseline = Some(Box::new(wire.clone()));
    }
    (wire, history)
}

pub(super) fn delta(
    batch: &CommittedTick,
    view: &WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
    previous: &ObservationHistory,
) -> Result<(turts_net_protocol::WorldDelta, ObservationHistory), ObservationChanged> {
    if policy.all_public_geometry(slot.0) {
        if previous.restricted {
            return Err(ObservationChanged);
        }
        return Ok((
            super::delta_for(batch, view, slot),
            ObservationHistory::default(),
        ));
    }
    if !previous.restricted {
        return Err(ObservationChanged);
    }
    let projected = project_view(view, slot, policy);
    let mut current = ObservationHistory::capture(&projected);
    let canonical = super::snapshot_for(&projected, slot);
    if previous
        .units
        .keys()
        .any(|id| !current.units.contains_key(id) && !batch.removed.contains(id))
        || current.units.keys().any(|id| {
            !previous.units.contains_key(id) && !batch.spawned.iter().any(|u| u.unit_id == *id)
        })
        || current.nodes != previous.nodes
    {
        return Err(ObservationChanged);
    }
    let units = current.units.keys().copied().collect::<BTreeSet<_>>();
    let nodes = current.nodes.clone();
    let event_allowed = |id: &u32| {
        units.contains(id)
            || previous.units.get(id).is_some_and(|owner| {
                batch.removed.contains(id) && unit_permitted(policy, slot.0, *id, *owner)
            })
    };
    let mut changes = batch.clone();
    changes.spawned.retain(|r| units.contains(&r.unit_id));
    changes.removed.retain(|id| event_allowed(id));
    changes.positions.retain(|(id, _)| units.contains(id));
    changes.nodes.retain(|r| nodes.contains(&r.id.0));
    changes.healths.retain(|r| units.contains(&r.unit));
    changes.sites_completed.retain(|id| units.contains(id));
    changes.effects.retain(|r| match r {
        turts_sim::economy::events::EffectRow::Fired { unit, target, .. } => {
            event_allowed(unit) && event_allowed(target)
        }
        turts_sim::economy::events::EffectRow::Destroyed { unit } => event_allowed(unit),
        turts_sim::economy::events::EffectRow::Credited { unit, .. } => event_allowed(unit),
    });
    // Reuse the sanitized immutable full rows for changed own records, so a censored
    // reference becomes an explicit replacement rather than leaving stale client state.
    for r in &mut changes.weapons {
        r.last_target = r.last_target.filter(|id| units.contains(id));
        r.ordered_target_fired = r.ordered_target_fired.filter(|id| units.contains(id));
    }
    for r in &mut changes.inserters {
        r.source = r.source.filter(|id| units.contains(id));
        r.destination = r.destination.filter(|id| units.contains(id));
    }
    for r in &mut changes.extractors {
        r.node = r.node.filter(|id| nodes.contains(id));
    }
    for r in &mut changes.economy.orders {
        if let Some(clean) = projected
            .economy
            .orders
            .iter()
            .find(|clean| clean.unit == r.unit)
        {
            *r = clean.clone();
        }
    }
    for r in &mut changes.unit_orders {
        if let Some(clean) = projected
            .unit_orders
            .iter()
            .find(|clean| clean.unit_id == r.unit_id)
        {
            *r = clean.clone();
        }
    }
    for r in &changes.repairs {
        if units.contains(&r.repairer)
            && !units.contains(&r.target)
            && !changes.repairs_cleared.contains(&r.repairer)
        {
            changes.repairs_cleared.push(r.repairer);
        }
    }
    changes
        .repairs
        .retain(|r| units.contains(&r.repairer) && units.contains(&r.target));
    changes.repairs_cleared.sort_unstable();
    changes.repairs_cleared.dedup();
    let mut wire = super::delta_for(&changes, &projected, slot);
    let old = previous.baseline.as_deref().ok_or(ObservationChanged)?;
    // Raw changes involving censored endpoints must not become duplicate wire rows:
    // their frequency or serialized size would otherwise disclose hidden activity.
    retain_changed(&mut wire.economy.orders, &old.economy.orders, |r| r.unit);
    retain_changed(&mut wire.inserters, &old.inserters, |r| r.unit);
    retain_changed(&mut wire.extractors, &old.extractors, |r| r.unit);
    retain_changed(&mut wire.weapons, &old.weapons, |r| r.unit);
    retain_changed(&mut wire.unit_orders, &old.unit_orders, |r| r.unit);
    retain_changed(&mut wire.records.repairs, &old.records.repairs, |r| {
        r.repairer
    });
    wire.records
        .repairs_cleared
        .retain(|id| old.records.repairs.iter().any(|r| r.repairer == *id));
    for repair in &old.records.repairs {
        if units.contains(&repair.repairer.0)
            && !canonical
                .records
                .repairs
                .iter()
                .any(|r| r.repairer == repair.repairer)
            && !wire.records.repairs_cleared.contains(&repair.repairer)
        {
            wire.records.repairs_cleared.push(repair.repairer);
        }
    }
    wire.records.repairs_cleared.sort_by_key(|id| id.0);
    current.baseline = Some(Box::new(canonical));
    Ok((wire, current))
}

fn retain_changed<T: PartialEq, K: PartialEq>(
    rows: &mut Vec<T>,
    previous: &[T],
    key: impl Fn(&T) -> K,
) {
    rows.retain(|row| previous.iter().find(|old| key(old) == key(row)) != Some(row));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_visible_borrows_the_original_view_and_preserves_legacy_wire_bytes() {
        let view = WorldView::default();
        let batch = CommittedTick {
            tick: 1,
            ..Default::default()
        };
        let slot = SlotId(0);
        assert!(matches!(
            project_view(&view, slot, &crate::permissions::AllVisible),
            Cow::Borrowed(_)
        ));
        let (snapshot, history) = snapshot(&view, slot, &crate::permissions::AllVisible);
        assert!(!history.restricted);
        assert!(history.baseline.is_none());
        assert_eq!(
            postcard::to_vec::<_, 16384>(&snapshot).unwrap(),
            postcard::to_vec::<_, 16384>(&super::super::snapshot_projected(&view, slot)).unwrap()
        );
        let (changed, _) = delta(
            &batch,
            &view,
            slot,
            &crate::permissions::AllVisible,
            &history,
        )
        .unwrap();
        assert_eq!(
            postcard::to_vec::<_, 16384>(&changed).unwrap(),
            postcard::to_vec::<_, 16384>(&super::super::delta_projected(&batch, &view, slot))
                .unwrap()
        );
    }
}
