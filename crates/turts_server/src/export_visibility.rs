//! Atomic permitted snapshots and ordinary visibility transitions. No raw truth enters a baseline.
#[path = "export_visibility_memory.rs"]
mod memory;
use crate::permissions::ObservationPermissions;
use std::collections::BTreeMap;
use turts_mod_api::{CoverageRun, KnowledgeDelta, KnowledgeSnapshot};
use turts_net_protocol::{self as wire, StateTransactionPayload};
use turts_sim::{CommittedTick, SlotId, WorldView, visibility::EventUnitFact};

/// Private lookup indexes keep the established wire ordering and duplicate semantics.
struct SortedIds(Vec<u32>);
impl FromIterator<u32> for SortedIds {
    fn from_iter<T: IntoIterator<Item = u32>>(rows: T) -> Self {
        let mut ids = rows.into_iter().collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        Self(ids)
    }
}
impl SortedIds {
    fn contains(&self, id: &u32) -> bool {
        self.0.binary_search(id).is_ok()
    }
    fn iter(&self) -> std::slice::Iter<'_, u32> {
        self.0.iter()
    }
    fn difference<'a>(&'a self, other: &'a Self) -> impl Iterator<Item = &'a u32> + 'a {
        let mut right = 0;
        self.0.iter().filter(move |id| {
            while right < other.0.len() && other.0[right] < **id {
                right += 1;
            }
            other.0.get(right) != Some(*id)
        })
    }
}
struct FlatUnits<'a>(Vec<&'a wire::UnitSnapshot>);
impl<'a> FlatUnits<'a> {
    fn new(rows: &'a [wire::UnitSnapshot]) -> Self {
        let mut index = rows.iter().collect::<Vec<_>>();
        index.sort_by_key(|row| row.unit_id.0);
        Self(index)
    }
    fn get(&self, id: &u32) -> Option<&'a wire::UnitSnapshot> {
        let end = self.0.partition_point(|row| row.unit_id.0 <= *id);
        end.checked_sub(1)
            .and_then(|i| self.0.get(i))
            .copied()
            .filter(|row| row.unit_id.0 == *id)
    }
    fn contains_key(&self, id: &u32) -> bool {
        self.get(id).is_some()
    }
}
struct FlatFacts<'a>(Vec<&'a EventUnitFact>);
impl<'a> FlatFacts<'a> {
    fn new(rows: &'a [EventUnitFact]) -> Self {
        let mut index = rows.iter().collect::<Vec<_>>();
        index.sort_by_key(|row| row.unit.unit);
        Self(index)
    }
    fn rows(&self, id: u32) -> &[&'a EventUnitFact] {
        let first = self.0.partition_point(|row| row.unit.unit < id);
        let end = self.0[first..].partition_point(|row| row.unit.unit == id);
        &self.0[first..first + end]
    }
    fn shared_phase(&self, a: u32, b: u32) -> bool {
        let right = self.rows(b);
        self.rows(a)
            .iter()
            .any(|a| right.iter().any(|b| a.basis == b.basis))
    }
}

#[derive(Clone)]
pub struct PermittedBaseline {
    pub state: wire::Snapshot,
    pub knowledge: Option<KnowledgeSnapshot>,
}
impl PermittedBaseline {
    pub fn retained_bytes(&self) -> usize {
        memory::baseline(self)
    }
    pub fn capture(view: &WorldView, slot: SlotId, policy: &dyn ObservationPermissions) -> Self {
        Self {
            state: super::snapshot_permitted(view, slot, policy),
            knowledge: policy.knowledge(slot.0),
        }
    }
    /// Build the next basis by transferring unchanged live unit names. The old
    /// basis is consumed for diffing only; it must not be serialized afterward.
    /// Names of hidden/removed units remain intact for truthful old event handles.
    pub(crate) fn capture_reusing_metadata(
        &mut self,
        view: &WorldView,
        slot: SlotId,
        policy: &dyn ObservationPermissions,
    ) -> Self {
        Self {
            state: super::export_permitted::snapshot_reusing(
                view,
                slot,
                policy,
                Some(&mut self.state.units),
            ),
            knowledge: policy.knowledge(slot.0),
        }
    }
    /// The legacy AllVisible path can retain an earlier truthful public observation for cosmetics.
    /// It never labels that old position as a current phase observation.
    pub fn all_visible_patch(
        &self,
        next: &Self,
        batch: &CommittedTick,
        slot: SlotId,
        context: &turts_mod_api::KnowledgeBasis,
    ) -> StateTransactionPayload {
        let StateTransactionPayload::Patch {
            mut state,
            knowledge,
            ..
        } = self.patch(next, batch, slot, &[])
        else {
            unreachable!()
        };
        let current = next
            .state
            .units
            .iter()
            .map(|unit| unit.unit_id.0)
            .collect::<SortedIds>();
        let old = FlatUnits::new(&self.state.units);
        state.removed = batch
            .removed
            .iter()
            .filter(|id| old.contains_key(id))
            .copied()
            .map(wire::UnitId)
            .collect();
        let effects = super::export_effects::effects(batch, slot);
        let mut reference_ids = Vec::with_capacity(effects.len().saturating_mul(2));
        for effect in &effects {
            match effect {
                wire::Effect::Fired { unit, target, .. } => {
                    reference_ids.push(unit.0);
                    reference_ids.push(target.0);
                }
                wire::Effect::Destroyed { unit } | wire::Effect::Credited { unit, .. } => {
                    reference_ids.push(unit.0);
                }
            }
        }
        let references = reference_ids.into_iter().collect::<SortedIds>();
        let mut observations = BTreeMap::new();
        for unit in &self.state.units {
            if references.contains(&unit.unit_id.0) && !current.contains(&unit.unit_id.0) {
                let mut basis = context.clone();
                basis.tick = self.state.tick;
                basis.phase = turts_mod_api::ObservationPhase::Completed;
                observations.insert(
                    unit.unit_id.0,
                    wire::EventUnitObservation {
                        basis,
                        unit: unit.clone(),
                    },
                );
            }
        }
        for unit in &batch.spawned {
            if references.contains(&unit.unit_id) && !current.contains(&unit.unit_id) {
                let mut basis = context.clone();
                basis.tick = batch.tick;
                basis.phase = turts_mod_api::ObservationPhase::Completed;
                observations.insert(
                    unit.unit_id,
                    wire::EventUnitObservation {
                        basis,
                        unit: super::unit_snapshot(unit),
                    },
                );
            }
        }
        let known =
            |unit: wire::UnitId| current.contains(&unit.0) || observations.contains_key(&unit.0);
        state.records.effects = effects
            .into_iter()
            .filter(|effect| match effect {
                wire::Effect::Fired { unit, target, .. } => known(*unit) && known(*target),
                wire::Effect::Destroyed { unit } | wire::Effect::Credited { unit, .. } => {
                    known(*unit)
                }
            })
            .collect();
        StateTransactionPayload::Patch {
            state,
            knowledge,
            event_units: observations.into_values().collect(),
        }
    }
    /// Diffs only two already-permitted states. Routine hide/reveal needs no full resync.
    pub fn patch(
        &self,
        next: &Self,
        batch: &CommittedTick,
        slot: SlotId,
        event_facts: &[EventUnitFact],
    ) -> StateTransactionPayload {
        let old = &self.state;
        let new = &next.state;
        let old_units = FlatUnits::new(&old.units);
        let new_units = FlatUnits::new(&new.units);
        let witnessed = event_facts
            .iter()
            .map(|fact| fact.unit.unit)
            .collect::<SortedIds>();
        let removed = batch
            .removed
            .iter()
            .copied()
            .filter(|id| old_units.contains_key(id))
            .filter(|id| {
                witnessed.contains(id)
                    || old_units
                        .get(id)
                        .is_some_and(|unit| unit.owner_slot.0 == slot.0)
            })
            .collect::<SortedIds>();
        let mut state = wire::WorldDelta {
            tick: new.tick,
            base_tick: old.tick,
            spawned: new
                .units
                .iter()
                .filter(|unit| !old_units.contains_key(&unit.unit_id.0))
                .cloned()
                .collect(),
            removed: removed.iter().copied().map(wire::UnitId).collect(),
            positions: new
                .units
                .iter()
                .filter(|unit| {
                    old_units
                        .get(&unit.unit_id.0)
                        .is_some_and(|old| old.pos != unit.pos)
                })
                .map(|unit| wire::UnitDelta {
                    unit_id: unit.unit_id,
                    pos: unit.pos,
                })
                .collect(),
            nodes: changed(&old.nodes, &new.nodes, |row| row.id.0),
            economy: wire::EconomyChanges {
                balances: changed(&old.economy.balances, &new.economy.balances, |row| {
                    row.resource.clone()
                }),
                inventories: changed(&old.economy.inventories, &new.economy.inventories, |row| {
                    (row.unit.0, row.port == wire::InventoryPort::Output)
                }),
                orders: changed(&old.economy.orders, &new.economy.orders, |row| row.unit.0),
                queues: changed(&old.economy.queues, &new.economy.queues, |row| {
                    row.producer.0
                }),
            },
            belts: changed(&old.belts, &new.belts, |row| row.piece.0),
            inserters: changed(&old.inserters, &new.inserters, |row| row.unit.0),
            extractors: changed(&old.extractors, &new.extractors, |row| row.unit.0),
            weapons: changed(&old.weapons, &new.weapons, |row| row.unit.0),
            healths: changed(&old.healths, &new.healths, |row| row.unit.0),
            research: changed(&old.research, &new.research, |row| row.slot.0),
            unit_orders: changed(&old.unit_orders, &new.unit_orders, |row| row.unit.0),
            records: wire::DeltaRecords {
                repairs: changed(&old.records.repairs, &new.records.repairs, |row| {
                    row.repairer.0
                }),
                repairs_cleared: old
                    .records
                    .repairs
                    .iter()
                    .filter(|row| {
                        new_units.contains_key(&row.repairer.0)
                            && !new
                                .records
                                .repairs
                                .iter()
                                .any(|new| new.repairer == row.repairer)
                    })
                    .map(|row| row.repairer)
                    .collect(),
                sites_completed: new
                    .units
                    .iter()
                    .filter(|unit| {
                        !unit.site && old_units.get(&unit.unit_id.0).is_some_and(|old| old.site)
                    })
                    .map(|unit| unit.unit_id)
                    .collect(),
                sites: changed(&old.records.sites, &new.records.sites, |row| row.unit.0),
                jobs: changed(&old.records.jobs, &new.records.jobs, |row| {
                    (row.producer.0, row.job.0)
                }),
                construction_queues: changed(
                    &old.records.construction_queues,
                    &new.records.construction_queues,
                    |row| row.constructor.0,
                ),
                supply: changed(&old.records.supply, &new.records.supply, |row| {
                    (row.slot.0, row.resource.clone())
                }),
                orders_cleared: old
                    .unit_orders
                    .iter()
                    .filter(|row| {
                        new_units.contains_key(&row.unit.0)
                            && !new.unit_orders.iter().any(|new| new.unit == row.unit)
                    })
                    .map(|row| row.unit)
                    .collect(),
                effects: super::export_effects::effects(batch, slot),
                match_state: if old.records.match_state != new.records.match_state {
                    new.records.match_state.clone()
                } else {
                    None
                },
            },
        };
        // Authorize events with their phase witness, never with the later hidden world position.
        let facts_by_unit = FlatFacts::new(event_facts);
        state.records.effects.retain(|effect| match effect {
            wire::Effect::Fired { unit, target, .. } => {
                facts_by_unit.shared_phase(unit.0, target.0)
            }
            wire::Effect::Destroyed { unit } => witnessed.contains(&unit.0),
            wire::Effect::Credited { unit, .. } => {
                new_units.contains_key(&unit.0) || witnessed.contains(&unit.0)
            }
        });
        let event_units = event_facts
            .iter()
            .filter(|fact| !new_units.contains_key(&fact.unit.unit))
            .map(|fact| wire::EventUnitObservation {
                basis: fact.basis.clone(),
                unit: wire::UnitSnapshot {
                    unit_id: wire::UnitId(fact.unit.unit),
                    content_id: fact.unit.content.to_string(),
                    owner_slot: wire::SlotId(fact.unit.owner),
                    facing: fact.unit.facing,
                    pos: wire::Pos {
                        x: fact.unit.position.x,
                        y: fact.unit.position.y,
                    },
                    site: fact.unit.site,
                },
            })
            .collect();
        let knowledge = match (&self.knowledge, &next.knowledge) {
            (Some(old), Some(new)) => Some(knowledge_delta(
                old,
                new,
                &self.state,
                &next.state,
                &removed,
            )),
            _ => None,
        };
        StateTransactionPayload::Patch {
            state: Box::new(state),
            knowledge,
            event_units,
        }
    }
}
fn changed<T: Clone + PartialEq, K: Ord>(old: &[T], new: &[T], key: impl Fn(&T) -> K) -> Vec<T> {
    if new.is_empty() {
        return Vec::new();
    }
    if old.is_empty() {
        return new.to_vec();
    }
    // One contiguous index replaces one tree allocation per old row. Stable
    // sorting keeps the last old duplicate as the comparison row, as before.
    let mut index = old.iter().map(|row| (key(row), row)).collect::<Vec<_>>();
    index.sort_by(|a, b| a.0.cmp(&b.0));
    let mut end = 0;
    new.iter()
        .filter(|row| {
            let key = key(row);
            if end > 0 && index[end - 1].0 > key {
                end = index.partition_point(|(old, _)| *old <= key);
            } else {
                // Canonical ID-ordered families advance together in linear time.
                // Bound the scan before binary fallback so arbitrary new ordering
                // cannot turn this private index into a quadratic search.
                let stop = (end + 8).min(index.len());
                while end < stop && index[end].0 <= key {
                    end += 1;
                }
                if end < index.len() && index[end].0 <= key {
                    end += index[end..].partition_point(|(old, _)| *old <= key);
                }
            }
            end.checked_sub(1)
                .and_then(|position| index.get(position))
                .filter(|(old, _)| *old == key)
                .map(|(_, old)| *old)
                != Some(*row)
        })
        .cloned()
        .collect()
}

fn knowledge_delta(
    old: &KnowledgeSnapshot,
    new: &KnowledgeSnapshot,
    before: &wire::Snapshot,
    after: &wire::Snapshot,
    removed: &SortedIds,
) -> KnowledgeDelta {
    let before_units = before
        .units
        .iter()
        .map(|row| row.unit_id.0)
        .collect::<SortedIds>();
    let after_units = after
        .units
        .iter()
        .map(|row| row.unit_id.0)
        .collect::<SortedIds>();
    let before_nodes = before
        .nodes
        .iter()
        .map(|row| row.id.0)
        .collect::<SortedIds>();
    let after_nodes = after
        .nodes
        .iter()
        .map(|row| row.id.0)
        .collect::<SortedIds>();
    let mut coverage: Vec<CoverageRun> = vec![];
    for byte in 0..new.observed.len() {
        if old.observed.get(byte) == new.observed.get(byte)
            && old.explored.get(byte) == new.explored.get(byte)
        {
            continue;
        }
        for bit in 0..8 {
            let cell = (byte * 8 + bit) as u32;
            if u64::from(cell) >= u64::from(new.grid.columns) * u64::from(new.grid.rows) {
                break;
            }
            let mask = 1 << bit;
            let observed = new.observed[byte] & mask != 0;
            let explored = new.explored[byte] & mask != 0;
            if old.observed.get(byte).is_some_and(|old| old & mask != 0) == observed
                && old.explored.get(byte).is_some_and(|old| old & mask != 0) == explored
            {
                continue;
            }
            if let Some(last) = coverage.last_mut()
                && last.first_cell + last.cell_count == cell
                && last.observed == observed
                && last.explored == explored
            {
                last.cell_count += 1;
            } else {
                coverage.push(CoverageRun {
                    first_cell: cell,
                    cell_count: 1,
                    observed,
                    explored,
                });
            }
        }
    }
    let new_mem_units = new
        .remembered_units
        .iter()
        .map(|row| row.unit)
        .collect::<SortedIds>();
    let new_mem_nodes = new
        .remembered_nodes
        .iter()
        .map(|row| row.node)
        .collect::<SortedIds>();
    KnowledgeDelta {
        basis: new.basis.clone(),
        viewer: new.viewer,
        coverage,
        hidden_units: before_units
            .difference(&after_units)
            .filter(|id| !removed.contains(id))
            .copied()
            .collect(),
        hidden_nodes: before_nodes.difference(&after_nodes).copied().collect(),
        revealed_units: after_units.difference(&before_units).copied().collect(),
        revealed_nodes: after_nodes.difference(&before_nodes).copied().collect(),
        remembered_units: changed(&old.remembered_units, &new.remembered_units, |row| row.unit),
        remembered_nodes: changed(&old.remembered_nodes, &new.remembered_nodes, |row| row.node),
        forgotten_units: old
            .remembered_units
            .iter()
            .filter(|row| !new_mem_units.contains(&row.unit))
            .map(|row| row.unit)
            .collect(),
        forgotten_nodes: old
            .remembered_nodes
            .iter()
            .filter(|row| !new_mem_nodes.contains(&row.node))
            .map(|row| row.node)
            .collect(),
        retention_truncated: new.retention_truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use turts_mod_api::{ContentId, KnowledgeBasis, ObservationPhase, VisibilityGrid, WorldPos};
    #[test]
    fn flat_change_index_keeps_new_order_and_the_last_old_duplicate() {
        let old = vec![(2, 20), (1, 10), (2, 21), (4, 40)];
        let new = vec![(4, 40), (3, 30), (2, 20), (1, 10), (2, 21), (3, 31)];
        assert_eq!(
            changed(&old, &new, |row| row.0),
            vec![(3, 30), (2, 20), (3, 31)]
        );
        assert_eq!(
            changed::<_, u32>(&[], &[(7, 1), (3, 4)], |row| row.0),
            vec![(7, 1), (3, 4)]
        );
        assert!(changed(&old, &[], |row| row.0).is_empty());
        let old = (0..100).map(|key| (key, key * 2)).collect::<Vec<_>>();
        assert!(changed(&old, &old, |row| row.0).is_empty());
        let jumps = vec![(99, 0), (0, 0), (99, 198), (0, 1), (50, 100), (101, 1)];
        assert_eq!(
            changed(&old, &jumps, |row| row.0),
            vec![(99, 0), (0, 1), (101, 1)]
        );
    }
    #[test]
    fn flat_membership_keeps_set_order_duplicates_and_last_unit_row() {
        let left = [u32::MAX, 3, 1, 3].into_iter().collect::<SortedIds>();
        let right = [1, 2].into_iter().collect::<SortedIds>();
        assert_eq!(
            left.difference(&right).copied().collect::<Vec<_>>(),
            vec![3, u32::MAX]
        );
        assert!(left.contains(&3));
        assert!(!left.contains(&2));
        let mut last = unit(2);
        last.pos.x = 9.;
        let rows = [unit(2), unit(1), last];
        assert_eq!(FlatUnits::new(&rows).get(&2).unwrap().pos.x, 9.);
    }
    #[test]
    fn flat_event_lookup_requires_an_exact_shared_phase() {
        let mut different = fact(2, 7);
        different.basis.phase = ObservationPhase::Start;
        let mut facts = vec![different, fact(1, 7), fact(2, 6)];
        assert!(!FlatFacts::new(&facts).shared_phase(1, 2));
        facts.push(fact(2, 7));
        assert!(FlatFacts::new(&facts).shared_phase(1, 2));
        assert!(!FlatFacts::new(&facts).shared_phase(1, 99));
    }
    fn basis(tick: u32) -> KnowledgeBasis {
        KnowledgeBasis {
            match_id: 4,
            map: ContentId::parse("fixture:map").unwrap(),
            tick,
            phase: ObservationPhase::Completed,
        }
    }
    fn unit(id: u32) -> wire::UnitSnapshot {
        wire::UnitSnapshot {
            unit_id: wire::UnitId(id),
            content_id: "fixture:unit".into(),
            owner_slot: wire::SlotId(if id == 1 { 0 } else { 1 }),
            facing: None,
            pos: wire::Pos { x: 5.0, y: 5.0 },
            site: false,
        }
    }
    fn snapshot(tick: u32, enemy: bool) -> PermittedBaseline {
        let mut state = wire::Snapshot {
            tick,
            units: vec![unit(1)],
            ..Default::default()
        };
        if enemy {
            state.units.push(unit(2));
            state.healths.push(wire::HealthRecord {
                unit: wire::UnitId(2),
                current: 10,
            });
        }
        PermittedBaseline {
            state,
            knowledge: Some(KnowledgeSnapshot {
                basis: basis(tick),
                viewer: 0,
                grid: VisibilityGrid {
                    size: WorldPos { x: 8.0, y: 8.0 },
                    cell_size: 2.0,
                    columns: 4,
                    rows: 4,
                },
                observed: vec![0; 2],
                explored: vec![255; 2],
                remembered_units: vec![],
                remembered_nodes: vec![],
                retention_truncated: false,
            }),
        }
    }
    fn fact(id: u32, tick: u32) -> EventUnitFact {
        let mut stamp = basis(tick);
        stamp.phase = ObservationPhase::Tactical;
        EventUnitFact {
            basis: stamp,
            unit: turts_sim::visibility::VisibleUnit {
                unit: id,
                content: Arc::new(ContentId::parse("fixture:unit").unwrap()),
                owner: if id == 1 { 0 } else { 1 },
                position: WorldPos { x: 5.0, y: 5.0 },
                facing: None,
                site: false,
            },
        }
    }
    #[test]
    fn ordinary_hide_and_reveal_are_atomic_differences_without_false_death() {
        let old = snapshot(1, true);
        let hidden = snapshot(2, false);
        let batch = CommittedTick {
            tick: 2,
            removed: vec![2],
            ..Default::default()
        };
        let StateTransactionPayload::Patch {
            state,
            knowledge: Some(knowledge),
            ..
        } = old.patch(&hidden, &batch, SlotId(0), &[])
        else {
            panic!()
        };
        assert!(state.removed.is_empty());
        assert_eq!(knowledge.hidden_units, vec![2]);
        assert!(state.healths.is_empty());
        let revealed = snapshot(3, true);
        let StateTransactionPayload::Patch {
            state,
            knowledge: Some(knowledge),
            ..
        } = hidden.patch(
            &revealed,
            &CommittedTick {
                tick: 3,
                ..Default::default()
            },
            SlotId(0),
            &[],
        )
        else {
            panic!()
        };
        assert_eq!(state.spawned, vec![unit(2)]);
        assert_eq!(
            state.healths,
            vec![wire::HealthRecord {
                unit: wire::UnitId(2),
                current: 10
            }]
        );
        assert_eq!(knowledge.revealed_units, vec![2]);
    }
    #[test]
    fn event_phase_authorizes_removed_endpoints_and_transient_deaths() {
        use turts_sim::economy::events::EffectRow;
        let old = snapshot(1, true);
        let next = snapshot(2, false);
        let batch = CommittedTick {
            tick: 2,
            removed: vec![2, 3],
            effects: vec![
                EffectRow::Fired {
                    unit: 2,
                    target: 1,
                    weapon: 0,
                    reduced: false,
                },
                EffectRow::Destroyed { unit: 2 },
                EffectRow::Destroyed { unit: 3 },
            ],
            ..Default::default()
        };
        let facts = vec![fact(1, 2), fact(2, 2), fact(3, 2)];
        let StateTransactionPayload::Patch {
            state,
            knowledge: Some(knowledge),
            event_units,
        } = old.patch(&next, &batch, SlotId(0), &facts)
        else {
            panic!()
        };
        assert_eq!(state.removed, vec![wire::UnitId(2)]);
        assert!(knowledge.hidden_units.is_empty());
        assert_eq!(state.records.effects.len(), 3);
        assert_eq!(
            event_units
                .iter()
                .map(|row| row.unit.unit_id.0)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        let StateTransactionPayload::Patch { state, .. } = old.patch(&next, &batch, SlotId(0), &[])
        else {
            panic!()
        };
        assert!(state.records.effects.is_empty());
        assert!(state.removed.is_empty());
    }
    #[test]
    fn all_visible_cosmetics_keep_truthful_old_and_birth_stamps_and_omit_unknown_handles() {
        use turts_sim::economy::events::EffectRow;
        let mut old = snapshot(1, true);
        old.knowledge = None;
        let mut next = snapshot(2, false);
        next.knowledge = None;
        let batch = CommittedTick {
            tick: 2,
            removed: vec![2, 3, 99],
            spawned: vec![turts_sim::world::UnitState {
                unit_id: 3,
                content_id: "fixture:unit".into(),
                owner_slot: 1,
                facing: None,
                pos: bevy::math::Vec2::new(7.0, 7.0),
            }],
            effects: vec![
                EffectRow::Fired {
                    unit: 2,
                    target: 1,
                    weapon: 0,
                    reduced: false,
                },
                EffectRow::Destroyed { unit: 2 },
                EffectRow::Destroyed { unit: 3 },
                EffectRow::Destroyed { unit: 99 },
            ],
            ..Default::default()
        };
        let StateTransactionPayload::Patch {
            state,
            knowledge,
            event_units,
        } = old.all_visible_patch(&next, &batch, SlotId(0), &basis(2))
        else {
            panic!()
        };
        assert!(knowledge.is_none());
        assert_eq!(state.removed, vec![wire::UnitId(2)]);
        assert_eq!(
            state.records.effects.len(),
            3,
            "unknown legacy cosmetic handle is omitted"
        );
        let old_observation = event_units
            .iter()
            .find(|row| row.unit.unit_id.0 == 2)
            .unwrap();
        assert_eq!(old_observation.basis.tick, 1);
        assert_eq!(old_observation.basis.phase, ObservationPhase::Completed);
        assert_eq!(old_observation.unit.pos, unit(2).pos);
        let born = event_units
            .iter()
            .find(|row| row.unit.unit_id.0 == 3)
            .unwrap();
        assert_eq!(born.basis.tick, 2);
        assert_eq!(born.unit.pos, wire::Pos { x: 7.0, y: 7.0 });
        assert!(!event_units.iter().any(|row| row.unit.unit_id.0 == 99));
    }
    #[test]
    fn hidden_present_moved_and_destroyed_truth_produces_equal_permitted_patches() {
        use turts_sim::economy::events::EffectRow;
        let old = snapshot(1, false);
        let next = snapshot(2, false);
        let absent = CommittedTick {
            tick: 2,
            ..Default::default()
        };
        let hidden = CommittedTick {
            tick: 2,
            removed: vec![77],
            positions: vec![(77, bevy::math::Vec2::new(7.0, 7.0))],
            effects: vec![EffectRow::Destroyed { unit: 77 }],
            ..Default::default()
        };
        assert_eq!(
            old.patch(&next, &absent, SlotId(0), &[]),
            old.patch(&next, &hidden, SlotId(0), &[])
        );
    }
}
