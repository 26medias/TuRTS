//! Validate incoming terrain facts against the same candidate entity/knowledge basis.
use super::store::{Authority, Layout};
use crate::replica::{ReplicaContext, UnitTable};
use std::cell::RefCell;
use turts_mod_api::{ContentId, KnowledgeBasis, KnowledgeSnapshot, TerrainCell};
use turts_net_protocol::{TerrainGroundAnchor, TerrainWorkRecord, TerrainWorkWait};
use turts_rules::terrain::geometry::{Budget, Cell};

pub(crate) struct View<'a, Units: UnitTable> {
    pub context: &'a ReplicaContext,
    pub units: &'a Units,
    pub basis: &'a KnowledgeBasis,
    pub knowledge: Option<&'a KnowledgeSnapshot>,
    pub layout: &'a Layout,
    // All surface validations in a transaction share one public bounded work budget.
    pub budget: RefCell<Budget>,
}
impl<Units: UnitTable> View<'_, Units> {
    fn covered(&self, cell: TerrainCell, current: bool) -> bool {
        let Some(knowledge) = self.knowledge else {
            return true;
        };
        let shape = knowledge.grid;
        let Some(fog) =
            turts_rules::visibility::Grid::checked(shape.size.x, shape.size.y, shape.cell_size)
        else {
            return false;
        };
        self.layout
            .grid
            .cell_observed(
                Cell {
                    x: cell.x,
                    y: cell.y,
                },
                fog,
                &mut self.budget.borrow_mut(),
                |index| {
                    let mask = if current {
                        &knowledge.observed
                    } else {
                        &knowledge.explored
                    };
                    mask.get(index as usize / 8)
                        .is_some_and(|byte| byte & (1 << (index % 8)) != 0)
                },
            )
            .unwrap_or(false)
    }
}
impl<Units: UnitTable> Authority for View<'_, Units> {
    fn surface(&self, cell: TerrainCell, observed_tick: u32, current: bool) -> bool {
        observed_tick <= self.basis.tick
            && self.covered(cell, current)
            && (current || !self.covered(cell, true))
    }
    fn anchor(&self, row: &TerrainGroundAnchor) -> bool {
        if row.observed_tick > self.basis.tick {
            return false;
        }
        if self.units.contains_key(&row.unit.0) {
            return row.current;
        }
        !row.current
            && self.knowledge.is_some_and(|knowledge| {
                knowledge.remembered_units.iter().any(|memory| {
                    memory.unit == row.unit.0
                        && row.observed_tick <= memory.last_seen_tick
                        && memory.ground_elevation == Some(row.elevation)
                })
            })
    }
    fn required_anchors(&self) -> impl Iterator<Item = u32> {
        self.units.rows().map(|(id, _)| id).chain(
            self.knowledge
                .into_iter()
                .flat_map(|knowledge| knowledge.remembered_units.iter().map(|memory| memory.unit)),
        )
    }
    fn work(&self, row: &TerrainWorkRecord) -> bool {
        let Some(unit) = self
            .units
            .get(&row.unit.0)
            .filter(|unit| unit.owner_slot == self.context.slot)
        else {
            return false;
        };
        let Ok(tool) = ContentId::parse(&row.designation.tool) else {
            return false;
        };
        let Some(definition) = self.context.unit_definition(unit.content_id) else {
            return false;
        };
        if !definition.terrain_tools.contains(&tool)
            || !self
                .context
                .content
                .catalog
                .terrain_tools
                .contains_key(&tool)
        {
            return false;
        }
        let unknown = row
            .current_cell
            .is_none_or(|cell| !self.covered(cell, true))
            || row.wait == Some(TerrainWorkWait::Unknown);
        !unknown
            || (row.required_work.is_none()
                && row.completed_work == 0
                && matches!(
                    row.wait,
                    None | Some(
                        TerrainWorkWait::Unknown
                            | TerrainWorkWait::Approaching
                            | TerrainWorkWait::Paused
                            | TerrainWorkWait::Unloading
                            | TerrainWorkWait::Disabled
                            | TerrainWorkWait::Invalid
                    )
                ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ClientContent;
    use turts_mod_api::{
        KnowledgeBasis, ObservationPhase, RememberedUnit, VisibilityGrid, WorldPos,
    };
    use turts_net_protocol::{TerrainGroundAnchor, UnitId};
    #[test]
    fn contacts_distinguish_live_from_remembered_without_inventing_fresh_observation_ticks() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry");
        let archive = turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&path).unwrap(),
        )
        .unwrap();
        let content = ClientContent::from_archives(&[archive]).unwrap();
        let layout = content.terrain.clone().unwrap();
        let context = ReplicaContext::new(content, 0);
        let basis = KnowledgeBasis {
            match_id: 1,
            map: ContentId::parse("quarry:field").unwrap(),
            tick: 20,
            phase: ObservationPhase::Completed,
        };
        let knowledge = KnowledgeSnapshot {
            basis: basis.clone(),
            viewer: 0,
            grid: VisibilityGrid {
                size: WorldPos { x: 32., y: 24. },
                cell_size: 2.,
                columns: 16,
                rows: 12,
            },
            observed: vec![0; 24],
            explored: vec![255; 24],
            remembered_units: vec![RememberedUnit {
                unit: 2,
                content: ContentId::parse("quarry:walker").unwrap(),
                owner: 1,
                position: WorldPos { x: 24., y: 8. },
                facing: None,
                site: false,
                last_seen_tick: 15,
                ground_elevation: Some(2),
            }],
            remembered_nodes: vec![],
            retention_truncated: false,
        };
        let units = std::collections::HashMap::from([(
            1,
            crate::replica::UnitReplica {
                content_id: "quarry:walker".into(),
                owner_slot: 0,
                facing: None,
                samples: std::collections::VecDeque::new(),
            },
        )]);
        let view = View {
            context: &context,
            units: &units,
            basis: &basis,
            knowledge: Some(&knowledge),
            layout: &layout,
            budget: RefCell::new(Budget::new(1024)),
        };
        let mut live = TerrainGroundAnchor {
            unit: UnitId(1),
            elevation: 0,
            observed_tick: 10,
            current: true,
        };
        assert!(
            view.anchor(&live),
            "unchanged current contact may retain its old tick"
        );
        live.current = false;
        assert!(
            !view.anchor(&live),
            "stale contact cannot ground a live body"
        );
        let mut stale = TerrainGroundAnchor {
            unit: UnitId(2),
            elevation: 2,
            observed_tick: 10,
            current: false,
        };
        assert!(
            view.anchor(&stale),
            "retained anchor may predate the final entity sighting"
        );
        stale.current = true;
        assert!(
            !view.anchor(&stale),
            "remembered entity cannot acquire live contact permission"
        );
        stale.current = false;
        stale.observed_tick = 16;
        assert!(!view.anchor(&stale), "hidden refresh is forbidden");
        stale.observed_tick = 10;
        stale.elevation = 3;
        assert!(!view.anchor(&stale));
        let mut required = view.required_anchors().collect::<Vec<_>>();
        required.sort();
        assert_eq!(required, vec![1, 2]);
        let mut work = TerrainWorkRecord {
            unit: UnitId(1),
            designation: turts_mod_api::TerrainDesignation {
                tool: "quarry:spade".into(),
                cells: turts_mod_api::TerrainRect {
                    min: TerrainCell { x: 1, y: 1 },
                    max: TerrainCell { x: 2, y: 2 },
                },
                target_elevation: 0,
            },
            current_cell: Some(TerrainCell { x: 1, y: 1 }),
            completed_work: 0,
            required_work: None,
            wait: Some(TerrainWorkWait::Disabled),
        };
        assert!(view.work(&work));
        work.wait = Some(TerrainWorkWait::Invalid);
        assert!(view.work(&work));
        work.required_work = Some(1);
        assert!(!view.work(&work), "Unknown work must not publish a cost");
        work.required_work = None;
        work.completed_work = 1;
        assert!(
            !view.work(&work),
            "Unknown work must not publish hidden progress"
        );
    }
}
