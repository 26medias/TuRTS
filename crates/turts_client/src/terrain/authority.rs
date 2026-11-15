//! Validate incoming terrain facts against the same candidate entity/knowledge basis.
use super::store::{Authority, Layout};
use crate::replica::{ReplicaContext, UnitTable};
use std::cell::RefCell;
use turts_mod_api::{ContentId, KnowledgeBasis, KnowledgeSnapshot, TerrainCell};
use turts_net_protocol::{
    TerrainGroundAnchor, TerrainWorkRecord, TerrainWorkWait, visibility::EventUnitObservation,
};
use turts_rules::terrain::geometry::{Budget, Cell};

pub(crate) struct View<'a, Units: UnitTable> {
    pub context: &'a ReplicaContext,
    pub units: &'a Units,
    pub basis: &'a KnowledgeBasis,
    pub knowledge: Option<&'a KnowledgeSnapshot>,
    pub event_units: &'a [EventUnitObservation],
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
            return true;
        }
        if self
            .event_units
            .iter()
            .any(|fact| fact.unit.unit_id == row.unit && row.observed_tick <= fact.basis.tick)
        {
            return true;
        }
        self.knowledge.is_some_and(|knowledge| {
            knowledge.remembered_units.iter().any(|memory| {
                memory.unit == row.unit.0
                    && row.observed_tick <= memory.last_seen_tick
                    && memory.ground_elevation == Some(row.elevation)
            })
        })
    }
    fn required_anchors(&self) -> impl Iterator<Item = u32> {
        self.units
            .rows()
            .map(|(id, _)| id)
            .chain(self.event_units.iter().map(|fact| fact.unit.unit_id.0))
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
        !unknown || (row.required_work.is_none() && row.completed_work == 0)
    }
}
