//! Immutable, permission-bound surface/contact snapshots. No geology or private revision API.
use super::{TerrainObservedCell, TerrainState, state::ChunkTable};
use bevy_ecs::{prelude::Resource, world::World};
use std::{collections::BTreeMap, fmt, sync::Arc};
use turts_mod_api::{ContentId, ObservationPhase, TerrainCell};
use turts_rules::terrain::geometry::{Budget, Cell, Grid};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ground {
    owner: u8,
    elevation: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainGroundObservation {
    pub unit: u32,
    pub elevation: i32,
    pub observed_tick: u32,
}
/// Retain this Arc alongside its WorldView/visibility basis. It never reads live World data.
pub struct TerrainPhase {
    grid: Grid,
    chunk_width: u32,
    chunk_columns: u32,
    surface: ChunkTable,
    materials: Arc<[ContentId]>,
    ground: BTreeMap<u32, Ground>,
    work: Vec<super::TerrainWorkView>,
    tick: u32,
    phase: ObservationPhase,
    permissions: Option<Arc<crate::visibility::VisibilityBasis>>,
}
impl fmt::Debug for TerrainPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerrainPhase")
            .field("grid", &self.grid)
            .field("tick", &self.tick)
            .field("phase", &self.phase)
            .finish_non_exhaustive()
    }
}
impl PartialEq for TerrainPhase {
    fn eq(&self, other: &Self) -> bool {
        self.grid == other.grid
            && self.chunk_width == other.chunk_width
            && self.chunk_columns == other.chunk_columns
            && self.surface == other.surface
            && self.materials == other.materials
            && self.ground == other.ground
            && self.work == other.work
            && self.tick == other.tick
            && self.phase == other.phase
            && match (&self.permissions, &other.permissions) {
                (None, None) => true,
                (Some(left), Some(right)) => {
                    Arc::ptr_eq(left, right)
                        || (0..turts_mod_api::SLOT_COUNT)
                            .all(|viewer| left.knowledge(viewer) == right.knowledge(viewer))
                }
                _ => false,
            }
    }
}
impl TerrainPhase {
    pub fn grid(&self) -> Grid {
        self.grid
    }
    pub fn tick(&self) -> u32 {
        self.tick
    }
    pub fn observation_phase(&self) -> ObservationPhase {
        self.phase
    }
    pub fn materials(&self) -> &[ContentId] {
        &self.materials
    }
    pub fn knowledge_basis(&self) -> Option<&turts_mod_api::KnowledgeBasis> {
        self.permissions.as_ref().map(|basis| basis.stamp())
    }
    pub fn observe_cell(&self, viewer: u8, cell: TerrainCell) -> Option<TerrainObservedCell> {
        if viewer >= turts_mod_api::SLOT_COUNT
            || cell.x >= self.grid.columns
            || cell.y >= self.grid.rows
        {
            return None;
        }
        if let Some(permissions) = &self.permissions {
            let fog = permissions.grid();
            let visits = usize::try_from(u64::from(fog.columns) * u64::from(fog.rows))
                .ok()?
                .checked_mul(2)?
                .checked_add(16)?;
            let mut budget = Budget::new(visits);
            if !permissions
                .terrain_cell_observed(
                    viewer,
                    &self.grid,
                    Cell {
                        x: cell.x,
                        y: cell.y,
                    },
                    &mut budget,
                )
                .ok()?
            {
                return None;
            }
        }
        let cx = cell.x / self.chunk_width;
        let cy = cell.y / self.chunk_width;
        let width = self
            .chunk_width
            .min(self.grid.columns - cx * self.chunk_width);
        let index =
            usize::try_from(u64::from(cy) * u64::from(self.chunk_columns) + u64::from(cx)).ok()?;
        let offset = usize::try_from(
            u64::from(cell.y % self.chunk_width) * u64::from(width)
                + u64::from(cell.x % self.chunk_width),
        )
        .ok()?;
        let chunk = self.surface.get(index)?;
        let material = chunk
            .materials
            .get(offset)?
            .and_then(|key| self.materials.get(key as usize))
            .cloned();
        let provenance = *chunk.provenance.get(offset)?;
        Some(TerrainObservedCell {
            cell,
            elevation: *chunk.elevations.get(offset)?,
            material,
            provenance,
        })
    }
    pub fn work_for(&self, viewer: u8) -> impl Iterator<Item = &super::TerrainWorkView> {
        self.work.iter().filter(move |row| row.owner_slot == viewer)
    }
    pub fn ground_anchor(&self, viewer: u8, unit: u32) -> Option<TerrainGroundObservation> {
        if viewer >= turts_mod_api::SLOT_COUNT {
            return None;
        }
        let ground = self.ground.get(&unit)?;
        if ground.owner != viewer
            && self
                .permissions
                .as_ref()
                .is_some_and(|basis| !basis.unit_permitted(viewer, unit))
        {
            return None;
        }
        Some(TerrainGroundObservation {
            unit,
            elevation: ground.elevation,
            observed_tick: self.tick,
        })
    }
}
#[derive(Resource, Default)]
pub(crate) struct TerrainPhases {
    start: Option<Arc<TerrainPhase>>,
    tactical: Option<Arc<TerrainPhase>>,
    completed: Option<Arc<TerrainPhase>>,
}
pub fn phase(world: &World, requested: ObservationPhase) -> Option<Arc<TerrainPhase>> {
    let phases = world.get_resource::<TerrainPhases>()?;
    match requested {
        ObservationPhase::Start => &phases.start,
        ObservationPhase::Tactical => &phases.tactical,
        ObservationPhase::Completed => &phases.completed,
    }
    .clone()
}
pub(crate) fn capture_phase(world: &mut World, phase: ObservationPhase) {
    let Some(terrain) = world.get_resource::<TerrainState>() else {
        return;
    };
    let ground = world
        .iter_entities()
        .filter_map(|entity| {
            let unit = entity.get::<crate::UnitId>()?.0;
            let owner = entity.get::<crate::Owner>()?.0.0;
            Some((
                unit,
                Ground {
                    owner,
                    elevation: super::ground_elevation(world, unit)?,
                },
            ))
        })
        .collect();
    let snapshot = Arc::new(TerrainPhase {
        grid: terrain.grid,
        chunk_width: terrain.chunk_width,
        chunk_columns: terrain.chunk_columns,
        surface: terrain.chunks.clone(),
        materials: Arc::clone(&terrain.materials),
        ground,
        work: (0..turts_mod_api::SLOT_COUNT)
            .flat_map(|viewer| super::work_for(world, viewer))
            .collect(),
        tick: world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0),
        phase,
        permissions: crate::visibility::basis(world, phase),
    });
    world.init_resource::<TerrainPhases>();
    let mut phases = world.resource_mut::<TerrainPhases>();
    *match phase {
        ObservationPhase::Start => &mut phases.start,
        ObservationPhase::Tactical => &mut phases.tactical,
        ObservationPhase::Completed => &mut phases.completed,
    } = Some(snapshot);
}
pub(crate) fn initialize_phases(world: &mut World) {
    for phase in [
        ObservationPhase::Start,
        ObservationPhase::Tactical,
        ObservationPhase::Completed,
    ] {
        capture_phase(world, phase);
    }
}
