//! Pure projection of the immutable Completed terrain basis. No live terrain reads.
use std::collections::BTreeMap;
use turts_mod_api::{ObservationPhase, TerrainCell, TerrainDesignation};
use turts_net_protocol::{
    TerrainDelta, TerrainGroundAnchor, TerrainMaterialId, TerrainSnapshot, TerrainSurfaceRecord,
    TerrainWorkRecord, TerrainWorkWait, UnitId,
};
use turts_sim::{SlotId, WorldView};

use crate::permissions::{ObservationPermissions, unit_permitted};

pub(super) fn snapshot(
    view: &WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
) -> Option<TerrainSnapshot> {
    snapshot_checked(view, slot, policy).ok().flatten()
}

/// Distinguish a legacy flat map from a malformed or mismatched terrain basis.
/// The caller must refuse publication on Err, rather than fabricate a contact.
pub(super) fn snapshot_checked(
    view: &WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
) -> Result<Option<TerrainSnapshot>, &'static str> {
    let Some(phase) = view.terrain.as_deref() else {
        return Ok(None);
    };
    if slot.0 >= turts_mod_api::SLOT_COUNT
        || phase.tick() != view.tick
        || phase.observation_phase() != ObservationPhase::Completed
        || phase.knowledge_basis() != policy.knowledge_basis()
        || phase.knowledge_basis().is_some_and(|basis| {
            basis.tick != view.tick || basis.phase != ObservationPhase::Completed
        })
    {
        return Err("terrain basis does not match the completed permitted view");
    }
    if phase.materials().windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("terrain material dictionary is not canonical");
    }
    let materials = phase.materials().iter().map(ToString::to_string).collect();
    let dictionary = phase
        .materials()
        .iter()
        .enumerate()
        .map(|(index, material)| {
            u32::try_from(index)
                .map(|index| (material, TerrainMaterialId(index)))
                .map_err(|_| "terrain material dictionary is too large")
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let grid = phase.grid();
    let valid_height = |height| height >= grid.floor && height <= grid.ceiling;
    let mut surfaces = Vec::new();
    for y in 0..grid.rows {
        for x in 0..grid.columns {
            let cell = TerrainCell { x, y };
            let Some(row) = phase.surface(slot.0, cell) else {
                continue;
            };
            if row.cell != cell
                || row.last_seen_tick > view.tick
                || !valid_height(row.elevation)
                || row.material.is_some() != row.provenance.is_some()
                || (row.material.is_none() && row.elevation != grid.floor)
            {
                return Err("terrain surface observation is malformed");
            }
            let material = row
                .material
                .as_ref()
                .map(|material| {
                    dictionary
                        .get(material)
                        .copied()
                        .ok_or("terrain surface names an unknown public material")
                })
                .transpose()?;
            surfaces.push(TerrainSurfaceRecord {
                cell,
                elevation: row.elevation,
                material,
                provenance: row.provenance,
                observed_tick: row.last_seen_tick,
                current: row.current,
            });
        }
    }
    let mut anchors = BTreeMap::new();
    for unit in &view.units {
        if !unit_permitted(policy, slot.0, unit.unit_id, unit.owner_slot) {
            continue;
        }
        let contact = phase
            .ground_anchor(slot.0, unit.unit_id)
            .ok_or("permitted live terrain unit lacks its ground contact")?;
        if contact.unit != unit.unit_id
            || contact.observed_tick != view.tick
            || !valid_height(contact.elevation)
        {
            return Err("live terrain ground contact is malformed");
        }
        if anchors
            .insert(
                unit.unit_id,
                TerrainGroundAnchor {
                    unit: UnitId(unit.unit_id),
                    elevation: contact.elevation,
                    observed_tick: contact.observed_tick,
                    current: true,
                },
            )
            .is_some()
        {
            return Err("permitted terrain view has duplicate unit identities");
        }
    }
    if let Some(knowledge) = policy.knowledge(slot.0) {
        if knowledge.viewer != slot.0 || Some(&knowledge.basis) != phase.knowledge_basis() {
            return Err("remembered terrain contacts have a mismatched knowledge basis");
        }
        for unit in knowledge.remembered_units {
            if anchors.contains_key(&unit.unit) {
                return Err("terrain unit is both live and remembered");
            }
            let elevation = unit
                .ground_elevation
                .ok_or("remembered terrain unit lacks its permitted ground contact")?;
            if unit.last_seen_tick > view.tick || !valid_height(elevation) {
                return Err("remembered terrain ground contact is malformed");
            }
            anchors.insert(
                unit.unit,
                TerrainGroundAnchor {
                    unit: UnitId(unit.unit),
                    elevation,
                    observed_tick: unit.last_seen_tick,
                    current: false,
                },
            );
        }
    } else if phase.knowledge_basis().is_some() {
        return Err("terrain view lacks its matching permitted knowledge");
    }
    let mut work = BTreeMap::new();
    for row in phase.work_for(slot.0) {
        if !anchors.get(&row.unit).is_some_and(|anchor| anchor.current) || row.owner_slot != slot.0
        {
            return Err("terrain work lacks its permitted live owner contact");
        }
        let record = TerrainWorkRecord {
            unit: UnitId(row.unit),
            designation: TerrainDesignation {
                tool: row.tool.to_string(),
                cells: row.cells,
                target_elevation: row.target_elevation,
            },
            current_cell: row.current_cell,
            completed_work: row.completed_work,
            required_work: row.required_work,
            wait: work_wait(&row.wait),
        };
        if work.insert(row.unit, record).is_some() {
            return Err("terrain work has duplicate unit identities");
        }
    }
    Ok(Some(TerrainSnapshot {
        materials,
        surfaces,
        anchors: anchors.into_values().collect(),
        work: work.into_values().collect(),
    }))
}

fn work_wait(wait: &turts_sim::terrain::TerrainWorkWait) -> Option<TerrainWorkWait> {
    use turts_sim::terrain::TerrainWorkWait as Sim;
    Some(match wait {
        Sim::Working => return None,
        Sim::Unknown => TerrainWorkWait::Unknown,
        Sim::Approaching => TerrainWorkWait::Approaching,
        Sim::Blocked => TerrainWorkWait::Blocked,
        Sim::Disabled => TerrainWorkWait::Disabled,
        Sim::Material => TerrainWorkWait::UnsupportedMaterial,
        Sim::Floor => TerrainWorkWait::Bedrock,
        Sim::Ceiling => TerrainWorkWait::Ceiling,
        Sim::MissingInputs => TerrainWorkWait::InsufficientInput,
        Sim::CargoFull => TerrainWorkWait::FullCargo,
        Sim::Contested => TerrainWorkWait::Contested,
        Sim::Budget => TerrainWorkWait::Paused,
        Sim::Invalid => TerrainWorkWait::Invalid,
        Sim::Unloading => TerrainWorkWait::Unloading,
    })
}

/// Dictionary installation and retirement require a full context baseline.
pub(super) fn delta_compatible(
    old: &Option<TerrainSnapshot>,
    new: &Option<TerrainSnapshot>,
) -> bool {
    match (old, new) {
        (None, None) => true,
        (Some(old), Some(new)) => old.materials == new.materials,
        _ => false,
    }
}

pub(super) fn delta(
    old: &Option<TerrainSnapshot>,
    new: &Option<TerrainSnapshot>,
) -> Option<TerrainDelta> {
    if !delta_compatible(old, new) {
        return None;
    }
    let (Some(old), Some(new)) = (old, new) else {
        return None;
    };
    let (surfaces, surfaces_cleared) = changed(
        &old.surfaces,
        &new.surfaces,
        |row| (row.cell.y, row.cell.x),
        |left, right| {
            left == right
                || (left.current
                    && right.current
                    && left.cell == right.cell
                    && left.elevation == right.elevation
                    && left.material == right.material
                    && left.provenance == right.provenance)
        },
    );
    let (anchors, anchors_cleared) = changed(
        &old.anchors,
        &new.anchors,
        |row| row.unit.0,
        |left, right| {
            left == right
                || (left.current
                    && right.current
                    && left.unit == right.unit
                    && left.elevation == right.elevation)
        },
    );
    let (work, work_cleared) = changed(&old.work, &new.work, |row| row.unit.0, |a, b| a == b);
    let result = TerrainDelta {
        surfaces,
        surfaces_cleared: surfaces_cleared
            .into_iter()
            .map(|(y, x)| TerrainCell { x, y })
            .collect(),
        anchors,
        anchors_cleared: anchors_cleared.into_iter().map(UnitId).collect(),
        work,
        work_cleared: work_cleared.into_iter().map(UnitId).collect(),
    };
    (!result.surfaces.is_empty()
        || !result.surfaces_cleared.is_empty()
        || !result.anchors.is_empty()
        || !result.anchors_cleared.is_empty()
        || !result.work.is_empty()
        || !result.work_cleared.is_empty())
    .then_some(result)
}

fn changed<T: Clone, K: Ord + Copy>(
    old: &[T],
    new: &[T],
    key: impl Fn(&T) -> K,
    same: impl Fn(&T, &T) -> bool,
) -> (Vec<T>, Vec<K>) {
    let before: BTreeMap<_, _> = old.iter().map(|row| (key(row), row)).collect();
    let after: BTreeMap<_, _> = new.iter().map(|row| (key(row), row)).collect();
    let rows = after
        .iter()
        .filter_map(|(key, row)| {
            (!before.get(key).is_some_and(|old| same(old, row))).then(|| (*row).clone())
        })
        .collect();
    let cleared = before
        .keys()
        .filter(|key| !after.contains_key(key))
        .copied()
        .collect();
    (rows, cleared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> TerrainSnapshot {
        TerrainSnapshot {
            materials: vec!["quarry:rock".into()],
            surfaces: vec![TerrainSurfaceRecord {
                cell: TerrainCell { x: 2, y: 3 },
                elevation: 4,
                material: Some(TerrainMaterialId(0)),
                provenance: Some(turts_mod_api::TerrainProvenance::Natural),
                observed_tick: 10,
                current: true,
            }],
            anchors: vec![TerrainGroundAnchor {
                unit: UnitId(u32::MAX),
                elevation: 4,
                observed_tick: 10,
                current: true,
            }],
            work: vec![TerrainWorkRecord {
                unit: UnitId(u32::MAX),
                designation: TerrainDesignation {
                    tool: "quarry:dig".into(),
                    cells: turts_mod_api::TerrainRect {
                        min: TerrainCell { x: 2, y: 3 },
                        max: TerrainCell { x: 3, y: 4 },
                    },
                    target_elevation: 1,
                },
                current_cell: Some(TerrainCell { x: 2, y: 3 }),
                completed_work: 7,
                required_work: Some(11),
                wait: None,
            }],
        }
    }

    #[test]
    fn unchanged_current_observation_ticks_are_quiet_but_stale_transitions_are_exact() {
        let old = state();
        let mut new = old.clone();
        new.surfaces[0].observed_tick = 11;
        new.anchors[0].observed_tick = 11;
        assert_eq!(delta(&Some(old.clone()), &Some(new.clone())), None);
        new.surfaces[0].current = false;
        new.anchors[0].current = false;
        let change = delta(&Some(old), &Some(new.clone())).unwrap();
        assert_eq!(change.surfaces, new.surfaces);
        assert_eq!(change.anchors, new.anchors);
        assert!(change.work.is_empty());
        let mut later = new.clone();
        later.surfaces[0].observed_tick = 12;
        later.anchors[0].observed_tick = 12;
        let change = delta(&Some(new), &Some(later.clone())).unwrap();
        assert_eq!(change.surfaces, later.surfaces);
        assert_eq!(change.anchors, later.anchors);
    }

    #[test]
    fn disappearance_clears_all_tables_and_dictionary_changes_require_baseline() {
        let old = state();
        let empty = TerrainSnapshot {
            materials: old.materials.clone(),
            ..Default::default()
        };
        let change = delta(&Some(old.clone()), &Some(empty)).unwrap();
        assert_eq!(change.surfaces_cleared, vec![TerrainCell { x: 2, y: 3 }]);
        assert_eq!(change.anchors_cleared, vec![UnitId(u32::MAX)]);
        assert_eq!(change.work_cleared, vec![UnitId(u32::MAX)]);
        let mut different = old.clone();
        different.materials.push("quarry:sand".into());
        assert!(!delta_compatible(&Some(old.clone()), &Some(different)));
        assert!(!delta_compatible(&None, &Some(old)));
    }
}
