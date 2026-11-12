//! The one order writer (MP1 ruling R19, design 6.8 "`unit_orders` as changes").
//!
//! Every production write of `UnitOrders` goes through a function of this file. Each function
//! notes the unit in `OrderChanges`. At the end of the tick, `publish_order_changes` turns the
//! noted units into the change list of `CommittedTick`: the complete record of each unit whose
//! record changed, and the units whose record left the view.
//!
//! A write of `UnitOrders` is not the only input of the record. The activity and the plain-move
//! identity also read `MoveTarget`, the movement progress components, the pending move table
//! and the gather order. `collect_order_inputs` notes the units whose movement components
//! changed (Bevy change detection). The unit is only a candidate: the publish step compares the
//! new record with the record of the previous tick, so an extra candidate costs one comparison
//! and sends nothing.
//!
//! The scan `crates/turts_checks/tests/order_writers.rs` fails on a write of `UnitOrders`
//! outside this file. A write that the scan cannot see (an inferred type) is caught by the
//! `test-probes` check `check_order_changes`, which compares the list with the difference of the
//! two views.

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;

use crate::economy::views::UnitOrderView;
use crate::movement::{MoveProgress, PhysicalEscape, PlainMoveProgress};
use crate::orders::UnitOrders;
use crate::world::{MoveTarget, UnitId};

/// The units whose order record may have changed in the current tick.
#[derive(Resource, Debug, Default, Clone)]
pub struct OrderChanges {
    touched: BTreeSet<u32>,
    /// The units that had a pending move at the end of the previous tick.
    pending_before: BTreeSet<u32>,
}

impl OrderChanges {
    /// Note one unit. A unit that does not change after all costs one comparison.
    pub(crate) fn note(&mut self, unit: u32) {
        self.touched.insert(unit);
    }
}

fn note_entity(world: &mut World, entity: Entity) {
    if let Some(unit) = world.get::<UnitId>(entity).map(|id| id.0)
        && let Some(mut changes) = world.get_resource_mut::<OrderChanges>()
    {
        changes.note(unit);
    }
}

/// Replace the orders of a unit (insert them when the unit has none).
pub(crate) fn set_unit_orders(world: &mut World, entity: Entity, new: UnitOrders) {
    world.entity_mut(entity).insert(new);
    note_entity(world, entity);
}

/// Edit the orders of a unit in place. `None` when the unit has no `UnitOrders`.
pub(crate) fn edit_unit_orders<R>(
    world: &mut World,
    entity: Entity,
    edit: impl FnOnce(&mut UnitOrders) -> R,
) -> Option<R> {
    let result = world
        .entity_mut(entity)
        .get_mut::<UnitOrders>()
        .map(|mut orders| edit(&mut orders));
    if result.is_some() {
        note_entity(world, entity);
    }
    result
}

/// Remove the orders of a unit.
pub(crate) fn clear_unit_orders(world: &mut World, entity: Entity) {
    world.entity_mut(entity).remove::<UnitOrders>();
    note_entity(world, entity);
}

/// Give a new unit empty orders. The unit is spawned in this tick, so the publish step sends its
/// record with the unit.
pub(crate) fn insert_default_unit_orders(entity: &mut EntityWorldMut) {
    entity.insert(UnitOrders::default());
}

/// `FixedPostUpdate`, before the publish step: note the units whose movement components
/// changed or left in this tick.
#[allow(clippy::type_complexity)]
pub(crate) fn collect_order_inputs(
    mut changes: ResMut<OrderChanges>,
    changed: Query<
        &UnitId,
        Or<(
            Changed<MoveTarget>,
            Changed<MoveProgress>,
            Changed<PlainMoveProgress>,
            Changed<PhysicalEscape>,
        )>,
    >,
    mut moved: RemovedComponents<MoveTarget>,
    mut progress: RemovedComponents<MoveProgress>,
    mut plain: RemovedComponents<PlainMoveProgress>,
    mut escape: RemovedComponents<PhysicalEscape>,
    ids: Query<&UnitId>,
) {
    for unit in &changed {
        changes.note(unit.0);
    }
    for entity in moved
        .read()
        .chain(progress.read())
        .chain(plain.read())
        .chain(escape.read())
    {
        if let Ok(unit) = ids.get(entity) {
            changes.note(unit.0);
        }
    }
}

/// The order part of one committed tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderChangeList {
    /// The complete record of each unit whose record changed, in unit id order.
    pub records: Vec<UnitOrderView>,
    /// The units whose record left the view, with their owner slot, in unit id order. A removed
    /// unit is not listed: `removed` is its only mention.
    pub cleared: Vec<(u32, u8)>,
}

/// The candidates of this tick: the noted units, the units spawned in this tick, the units that
/// the gather order or the pending move table changed, and every unit that appears in the new
/// records but not in the old ones.
pub(crate) fn take_candidates(
    world: &mut World,
    spawned: impl Iterator<Item = u32>,
    gather_changed: impl Iterator<Item = u32>,
) -> BTreeSet<u32> {
    let pending_now: BTreeSet<u32> = world
        .get_resource::<crate::orders::PendingMovementOrders>()
        .map(|pending| pending.pending_member_counts.keys().copied().collect())
        .unwrap_or_default();
    let mut changes = world.resource_mut::<OrderChanges>();
    let mut candidates = std::mem::take(&mut changes.touched);
    candidates.extend(
        changes
            .pending_before
            .symmetric_difference(&pending_now)
            .copied(),
    );
    changes.pending_before = pending_now;
    candidates.extend(spawned);
    candidates.extend(gather_changed);
    candidates
}

fn find(rows: &[UnitOrderView], unit: u32) -> Option<&UnitOrderView> {
    rows.binary_search_by_key(&unit, |row| row.unit_id)
        .ok()
        .map(|index| &rows[index])
}

/// Compare each candidate with the previous record and build the change list.
///
/// `before` and `after` are sorted by unit id. A unit that is in `removed` is skipped: the
/// message that removes a unit never carries a record for it (the removal rule of the order
/// records).
pub(crate) fn order_change_list(
    candidates: &BTreeSet<u32>,
    before: &[UnitOrderView],
    after: &[UnitOrderView],
    removed: &[u32],
) -> OrderChangeList {
    let mut list = OrderChangeList::default();
    if before.is_empty() {
        // Nothing was published before this tick (the first tick of a world that no one
        // initialised): every record is new, whoever noted it.
        list.records = after
            .iter()
            .filter(|row| removed.binary_search(&row.unit_id).is_err())
            .cloned()
            .collect();
        return list;
    }
    for &unit in candidates {
        if removed.binary_search(&unit).is_ok() {
            continue;
        }
        match (find(before, unit), find(after, unit)) {
            (_, Some(new)) if find(before, unit) != Some(new) => list.records.push(new.clone()),
            (Some(old), None) => list.cleared.push((unit, old.owner_slot)),
            _ => {}
        }
    }
    list
}

/// The set of units whose record differs between two sorted views. The `test-probes` check
/// compares it with the change list.
#[cfg(feature = "test-probes")]
pub(crate) fn view_difference(before: &[UnitOrderView], after: &[UnitOrderView]) -> BTreeSet<u32> {
    use std::collections::BTreeMap;
    let old: BTreeMap<u32, &UnitOrderView> = before.iter().map(|row| (row.unit_id, row)).collect();
    let new: BTreeMap<u32, &UnitOrderView> = after.iter().map(|row| (row.unit_id, row)).collect();
    old.keys()
        .chain(new.keys())
        .copied()
        .filter(|unit| old.get(unit) != new.get(unit))
        .collect()
}

/// `test-probes`: the change list must hold every unit whose record differs from the record of
/// the previous tick. A removed unit is not in the list, so it is left out of the comparison.
/// `units` is the sorted unit list of this tick.
#[cfg(feature = "test-probes")]
pub(crate) fn check_order_changes(
    list: &OrderChangeList,
    before: &[UnitOrderView],
    after: &[UnitOrderView],
    removed: &[u32],
    units: &[crate::world::UnitState],
) {
    let listed: BTreeSet<u32> = list
        .records
        .iter()
        .map(|row| row.unit_id)
        .chain(list.cleared.iter().map(|(unit, _)| *unit))
        .collect();
    let missing: Vec<u32> = view_difference(before, after)
        .into_iter()
        .filter(|unit| {
            removed.binary_search(unit).is_err()
                && !listed.contains(unit)
                // A unit that is gone without a removal (a fixture that despawns an entity)
                // has no wire record to clear.
                && units
                    .binary_search_by_key(unit, |state| state.unit_id)
                    .is_ok()
        })
        .collect();
    assert!(
        missing.is_empty(),
        "the order change list misses the units {missing:?}: their record changed in this tick \
         and no writer noted them (write through `orders::writer`, or note the input)"
    );
}
