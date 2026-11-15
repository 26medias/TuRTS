//! Immutable, permission-bound surface/contact snapshots. No geology or private revision API.
use super::{TerrainObservedCell, TerrainState, state::ChunkTable};
use bevy_ecs::{prelude::Resource, world::World};
use std::{collections::BTreeMap, fmt, sync::Arc};
use turts_mod_api::{ContentId, ObservationPhase, TerrainCell};
use turts_rules::terrain::geometry::{Cell, Grid};
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainSurfaceObservation {
    pub cell: TerrainCell,
    pub elevation: i32,
    pub material: Option<ContentId>,
    pub provenance: Option<turts_mod_api::TerrainProvenance>,
    pub last_seen_tick: u32,
    pub current: bool,
}
/// Retain this Arc alongside its WorldView/visibility basis. It never reads live World data.
pub struct TerrainPhase {
    grid: Grid,
    chunk_width: u32,
    chunk_columns: u32,
    surface: ChunkTable,
    memory: BTreeMap<u8, super::history::MemoryTable>,
    coverage: Option<Arc<super::history::Coverage>>,
    initial: Arc<[i32]>,
    initial_ranges: Arc<[(i32, i32)]>,
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
            && self.memory == other.memory
            && self.initial == other.initial
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
    fn location(&self, cell: TerrainCell) -> Option<(usize, usize)> {
        if cell.x >= self.grid.columns || cell.y >= self.grid.rows {
            return None;
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
        Some((index, offset))
    }
    pub fn initial_elevation(&self, cell: TerrainCell) -> Option<i32> {
        self.location(cell)?;
        let index =
            usize::try_from(u64::from(cell.y) * u64::from(self.grid.columns) + u64::from(cell.x))
                .ok()?;
        self.initial.get(index).copied()
    }
    pub fn surface(&self, viewer: u8, cell: TerrainCell) -> Option<TerrainSurfaceObservation> {
        if viewer >= turts_mod_api::SLOT_COUNT {
            return None;
        }
        let (chunk, offset) = self.location(cell)?;
        let row = self
            .memory
            .get(&viewer)?
            .get(chunk)?
            .rows
            .as_ref()?
            .get(offset)?
            .as_ref()?;
        let material = row
            .material
            .and_then(|key| self.materials.get(key as usize))
            .cloned();
        let current = super::history::seen(
            self.permissions.as_deref(),
            self.coverage.as_deref(),
            viewer,
            self.grid,
            Cell {
                x: cell.x,
                y: cell.y,
            },
        );
        Some(TerrainSurfaceObservation {
            cell,
            elevation: row.elevation,
            material,
            provenance: row.provenance,
            last_seen_tick: row.last_seen_tick,
            current,
        })
    }
    pub fn observe_cell(&self, viewer: u8, cell: TerrainCell) -> Option<TerrainObservedCell> {
        let row = self.surface(viewer, cell)?;
        row.current.then_some(TerrainObservedCell {
            cell,
            elevation: row.elevation,
            material: row.material,
            provenance: row.provenance,
        })
    }
    /// Optimistic movement uses authorized current/stale facts, then public initial geometry.
    /// This is not a clearance answer for placement/work queries, where Unknown stays Unknown.
    pub(crate) fn planning_elevation(&self, viewer: u8, cell: Cell) -> Option<i32> {
        let cell = TerrainCell {
            x: cell.x,
            y: cell.y,
        };
        let (chunk, offset) = self.location(cell)?;
        self.memory
            .get(&viewer)
            .and_then(|memory| memory.get(chunk))
            .and_then(|chunk| chunk.rows.as_ref())
            .and_then(|rows| rows[offset])
            .map(|row| row.elevation)
            .or_else(|| self.initial_elevation(cell))
    }
    pub(crate) fn planning_rectangle(
        &self,
        viewer: u8,
        min: Cell,
        max: Cell,
        lease: &mut BTreeMap<usize, u64>,
    ) -> bool {
        let mut height = None;
        let mut flat = true;
        for cy in min.y / self.chunk_width..=max.y / self.chunk_width {
            for cx in min.x / self.chunk_width..=max.x / self.chunk_width {
                let index =
                    (u64::from(cy) * u64::from(self.chunk_columns) + u64::from(cx)) as usize;
                let memory = self
                    .memory
                    .get(&viewer)
                    .and_then(|memory| memory.get(index));
                lease.insert(index, memory.map_or(0, |chunk| chunk.geometry_revision));
                let Some((low, high)) = memory
                    .and_then(|chunk| chunk.planning_range)
                    .or_else(|| self.initial_ranges.get(index).copied())
                else {
                    return false;
                };
                flat &= low == high && height.is_none_or(|height| height == low);
                height = Some(low);
            }
        }
        flat
    }
    pub(crate) fn planning_lease_valid(&self, viewer: u8, lease: &BTreeMap<usize, u64>) -> bool {
        lease
            .iter()
            .all(|(index, revision)| self.knowledge_revision(viewer, *index) == *revision)
    }
    pub(crate) fn knowledge_revision(&self, viewer: u8, chunk: usize) -> u64 {
        self.memory
            .get(&viewer)
            .and_then(|memory| memory.get(chunk))
            .map_or(0, |chunk| chunk.geometry_revision)
    }
    #[cfg(test)]
    pub(crate) fn chunk_for(&self, cell: Cell) -> Option<usize> {
        self.location(TerrainCell {
            x: cell.x,
            y: cell.y,
        })
        .map(|(index, _)| index)
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
    if world.get_resource::<TerrainState>().is_none() {
        return;
    }
    let Some(history) = super::history::capture(world, phase) else {
        return;
    };
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
    let permissions = crate::visibility::basis(world, phase);
    let work = (0..turts_mod_api::SLOT_COUNT)
        .flat_map(|viewer| super::work_for(world, viewer))
        .map(|mut row| {
            if row.current_cell.is_some_and(|cell| {
                !super::history::seen(
                    permissions.as_deref(),
                    history.coverage.as_deref(),
                    row.owner_slot,
                    terrain.grid,
                    Cell {
                        x: cell.x,
                        y: cell.y,
                    },
                )
            }) {
                row.required_work = None;
                row.completed_work = 0;
                row.wait = super::TerrainWorkWait::Unknown;
            }
            row
        })
        .collect();
    let snapshot = Arc::new(TerrainPhase {
        grid: terrain.grid,
        chunk_width: terrain.chunk_width,
        chunk_columns: terrain.chunk_columns,
        surface: terrain.chunks.clone(),
        memory: history.slots,
        coverage: history.coverage,
        initial: Arc::clone(&terrain.initial),
        initial_ranges: Arc::clone(&terrain.initial_ranges),
        materials: Arc::clone(&terrain.materials),
        ground,
        work,
        tick: world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0),
        phase,
        permissions,
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
