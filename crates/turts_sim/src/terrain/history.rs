//! Shared observation history. Only authorized top facts can change remembered rows/epochs.
use super::{
    TerrainState,
    state::{ChunkTable, PagedChunks},
};
use bevy_ecs::{prelude::Resource, world::World};
use std::{collections::BTreeMap, sync::Arc};
use turts_mod_api::{ObservationPhase, TerrainProvenance, VisibilityGrid};
use turts_rules::terrain::geometry::{Budget, Cell};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Remembered {
    pub(super) elevation: i32,
    pub(super) material: Option<u32>,
    pub(super) provenance: Option<TerrainProvenance>,
    pub(super) last_seen_tick: u32,
}
impl Remembered {
    fn facts(&self) -> (i32, Option<u32>, Option<TerrainProvenance>) {
        (self.elevation, self.material, self.provenance)
    }
}
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct MemoryChunk {
    pub(super) geometry_revision: u64,
    pub(super) rows: Option<Vec<Option<Remembered>>>,
    pub(super) planning_range: Option<(i32, i32)>,
}
pub(super) type MemoryTable = PagedChunks<MemoryChunk>;
#[derive(Clone)]
struct SlotHistory {
    rows: MemoryTable,
    surface: ChunkTable,
    permissions: Option<Arc<crate::visibility::VisibilityBasis>>,
    tick: u32,
    initialized: bool,
}
/// Public grid overlap only. Flat ranges avoid one allocation per terrain cell.
pub(super) struct Coverage {
    fog: VisibilityGrid,
    spans: Vec<(u32, u32)>,
    cells: Vec<u32>,
}
impl Coverage {
    fn build(terrain: &TerrainState, fog: VisibilityGrid) -> Option<Self> {
        let rules = turts_rules::visibility::Grid {
            width: fog.size.x,
            height: fog.size.y,
            cell_size: fog.cell_size,
            columns: fog.columns,
            rows: fog.rows,
        };
        let count = terrain.columns.len();
        let limit = usize::try_from(u64::from(fog.columns) * u64::from(fog.rows))
            .ok()?
            .checked_mul(2)?
            .checked_add(16)?;
        let mut spans = Vec::new();
        spans.try_reserve_exact(count).ok()?;
        let mut cells = Vec::new();
        for y in 0..terrain.grid.rows {
            for x in 0..terrain.grid.columns {
                let start = u32::try_from(cells.len()).ok()?;
                let mut failed = false;
                let mut budget = Budget::new(limit);
                let valid = terrain
                    .grid
                    .cell_observed(Cell { x, y }, rules, &mut budget, |cell| {
                        // Optional cache only. Exhaustion falls back to the same exact overlap iterator.
                        if cells.len() >= 16_777_216 || cells.try_reserve(1).is_err() {
                            failed = true;
                            return false;
                        }
                        cells.push(cell);
                        true
                    })
                    .ok()?;
                if failed || !valid {
                    return None;
                }
                spans.push((start, u32::try_from(cells.len()).ok()?));
            }
        }
        Some(Self { fog, spans, cells })
    }
}
pub(super) fn seen(
    permissions: Option<&crate::visibility::VisibilityBasis>,
    coverage: Option<&Coverage>,
    viewer: u8,
    terrain: turts_rules::terrain::geometry::Grid,
    cell: Cell,
) -> bool {
    let Some(basis) = permissions else {
        return true;
    };
    if let Some(coverage) = coverage.filter(|coverage| coverage.fog == basis.grid()) {
        let index = u64::from(cell.y) * u64::from(terrain.columns) + u64::from(cell.x);
        let Some((start, end)) = usize::try_from(index)
            .ok()
            .and_then(|index| coverage.spans.get(index))
            .copied()
        else {
            return false;
        };
        return coverage.cells[start as usize..end as usize]
            .iter()
            .all(|cell| basis.fog_cell_observed(viewer, *cell));
    }
    let fog = basis.grid();
    let Some(visits) = usize::try_from(u64::from(fog.columns) * u64::from(fog.rows))
        .ok()
        .and_then(|count| count.checked_mul(2))
        .and_then(|count| count.checked_add(16))
    else {
        return false;
    };
    basis
        .terrain_cell_observed(viewer, &terrain, cell, &mut Budget::new(visits))
        .unwrap_or(false)
}
#[derive(Resource, Default)]
pub(crate) struct TerrainHistories {
    slots: BTreeMap<u8, SlotHistory>,
    coverage: Option<Arc<Coverage>>,
}
struct ChunkEdit {
    rows: Vec<Option<Remembered>>,
    geometry_changed: bool,
}
fn set_row(
    terrain: &TerrainState,
    history: &SlotHistory,
    edits: &mut BTreeMap<usize, ChunkEdit>,
    cell: Cell,
    row: Remembered,
) {
    let (index, offset) = terrain.chunk_location(cell).expect("validated public cell");
    let current = edits
        .get(&index)
        .and_then(|edit| edit.rows[offset])
        .or_else(|| {
            history
                .rows
                .get(index)
                .and_then(|chunk| chunk.rows.as_ref())
                .and_then(|rows| rows[offset])
        });
    if current == Some(row) {
        return;
    }
    let edit = edits.entry(index).or_insert_with(|| ChunkEdit {
        rows: history.rows[index]
            .rows
            .clone()
            .unwrap_or_else(|| vec![None; terrain.chunks[index].elevations.len()]),
        geometry_changed: false,
    });
    edit.geometry_changed |= current.map(|row| row.facts()) != Some(row.facts());
    edit.rows[offset] = Some(row);
}
fn planning_range(terrain: &TerrainState, chunk: usize, rows: &[Option<Remembered>]) -> (i32, i32) {
    let cx = chunk as u64 % u64::from(terrain.chunk_columns);
    let cy = chunk as u64 / u64::from(terrain.chunk_columns);
    let x0 = cx as u32 * terrain.chunk_width;
    let y0 = cy as u32 * terrain.chunk_width;
    let width = terrain.chunk_width.min(terrain.grid.columns - x0);
    rows.iter()
        .enumerate()
        .map(|(offset, row)| {
            row.map_or_else(
                || {
                    let x = x0 + offset as u32 % width;
                    let y = y0 + offset as u32 / width;
                    terrain.initial
                        [(u64::from(y) * u64::from(terrain.grid.columns) + u64::from(x)) as usize]
                },
                |row| row.elevation,
            )
        })
        .fold((i32::MAX, i32::MIN), |(low, high), height| {
            (low.min(height), high.max(height))
        })
}
fn update_slot(
    terrain: &TerrainState,
    history: &mut SlotHistory,
    viewer: u8,
    permissions: &Option<Arc<crate::visibility::VisibilityBasis>>,
    coverage: Option<&Coverage>,
    tick: u32,
    phase: ObservationPhase,
) {
    let same_mask = match (&history.permissions, permissions) {
        (None, None) => true,
        (Some(left), Some(right)) => left.same_observation_mask(viewer, right),
        _ => false,
    };
    let mut edits = BTreeMap::new();
    for chunk_index in 0..terrain.chunks.len() {
        if same_mask && Arc::ptr_eq(&history.surface[chunk_index], &terrain.chunks[chunk_index]) {
            continue;
        }
        let cx = chunk_index as u64 % u64::from(terrain.chunk_columns);
        let cy = chunk_index as u64 / u64::from(terrain.chunk_columns);
        let x0 = cx as u32 * terrain.chunk_width;
        let y0 = cy as u32 * terrain.chunk_width;
        let width = terrain.chunk_width.min(terrain.grid.columns - x0);
        let height = terrain.chunk_width.min(terrain.grid.rows - y0);
        for y in y0..y0 + height {
            for x in x0..x0 + width {
                let cell = Cell { x, y };
                let offset = ((y - y0) * width + x - x0) as usize;
                let old = history.rows[chunk_index]
                    .rows
                    .as_ref()
                    .and_then(|rows| rows[offset]);
                let observed = seen(permissions.as_deref(), coverage, viewer, terrain.grid, cell);
                let was_observed = seen(
                    history.permissions.as_deref(),
                    coverage,
                    viewer,
                    terrain.grid,
                    cell,
                );
                if observed {
                    let top = &terrain.chunks[chunk_index];
                    let mut row = Remembered {
                        elevation: top.elevations[offset],
                        material: top.materials[offset],
                        provenance: top.provenance[offset],
                        last_seen_tick: tick,
                    };
                    if let Some(old) = old
                        && old.facts() == row.facts()
                        && was_observed
                    {
                        row.last_seen_tick = old.last_seen_tick;
                    }
                    set_row(terrain, history, &mut edits, cell, row);
                } else if was_observed && let Some(mut row) = old {
                    // Continuous unchanged observations need no timestamp traffic. Record their
                    // final authorized tick once, when the cell becomes stale.
                    row.last_seen_tick = row.last_seen_tick.max(history.tick);
                    set_row(terrain, history, &mut edits, cell, row);
                }
            }
        }
    }
    if phase == ObservationPhase::Completed {
        for ((owner, cell), witness) in &terrain.step_observations {
            if *owner != viewer {
                continue;
            }
            let material = witness
                .surface
                .material
                .as_ref()
                .and_then(|id| terrain.key(id));
            set_row(
                terrain,
                history,
                &mut edits,
                *cell,
                Remembered {
                    elevation: witness.surface.elevation,
                    material,
                    provenance: witness.surface.provenance,
                    last_seen_tick: witness.observed_tick,
                },
            );
        }
    }
    let replacements = edits
        .into_iter()
        .map(|(index, edit)| {
            let previous = &history.rows[index];
            let revision = if edit.geometry_changed {
                previous
                    .geometry_revision
                    .checked_add(1)
                    .expect("three capture phases per bounded tick")
            } else {
                previous.geometry_revision
            };
            (
                index,
                Arc::new(MemoryChunk {
                    geometry_revision: revision,
                    planning_range: Some(planning_range(terrain, index, &edit.rows)),
                    rows: Some(edit.rows),
                }),
            )
        })
        .collect::<Vec<_>>();
    history.rows = history
        .rows
        .replaced_many(&replacements)
        .expect("validated bounded observation cache");
    history.surface = terrain.chunks.clone();
    history.permissions = permissions.clone();
    history.tick = tick;
}
pub(super) struct CapturedHistory {
    pub(super) slots: BTreeMap<u8, MemoryTable>,
    pub(super) coverage: Option<Arc<Coverage>>,
}
pub(super) fn capture(world: &mut World, phase: ObservationPhase) -> Option<CapturedHistory> {
    let mut history = world
        .remove_resource::<TerrainHistories>()
        .unwrap_or_default();
    let terrain = world.get_resource::<TerrainState>()?;
    let permissions = crate::visibility::basis(world, phase);
    if let Some(basis) = &permissions
        && history
            .coverage
            .as_ref()
            .is_none_or(|coverage| coverage.fog != basis.grid())
    {
        history.coverage = Coverage::build(terrain, basis.grid()).map(Arc::new);
    }
    let tick = world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0);
    for viewer in 0..turts_mod_api::SLOT_COUNT {
        if permissions.is_none()
            && viewer > 0
            && let Some(first) = history.slots.get(&0).cloned()
        {
            history.slots.insert(viewer, first);
            continue;
        }
        let slot = history.slots.entry(viewer).or_insert_with(|| {
            let empty = Arc::new(MemoryChunk::default());
            let rows = MemoryTable::new(vec![empty; terrain.chunks.len()])
                .expect("bounded public chunk table");
            // Empty rows are not observations, regardless of this initial mask.
            SlotHistory {
                rows,
                surface: terrain.chunks.clone(),
                permissions: permissions.clone(),
                tick,
                initialized: false,
            }
        });
        // An absent prior chunk means first capture; use a separate forced initial fill.
        if !slot.initialized {
            let mut edits = BTreeMap::new();
            for y in 0..terrain.grid.rows {
                for x in 0..terrain.grid.columns {
                    let cell = Cell { x, y };
                    if !seen(
                        permissions.as_deref(),
                        history.coverage.as_deref(),
                        viewer,
                        terrain.grid,
                        cell,
                    ) {
                        continue;
                    }
                    let (index, offset) = terrain.chunk_location(cell).expect("public cell");
                    let top = &terrain.chunks[index];
                    set_row(
                        terrain,
                        slot,
                        &mut edits,
                        cell,
                        Remembered {
                            elevation: top.elevations[offset],
                            material: top.materials[offset],
                            provenance: top.provenance[offset],
                            last_seen_tick: tick,
                        },
                    );
                }
            }
            let replacements = edits
                .into_iter()
                .map(|(index, edit)| {
                    (
                        index,
                        Arc::new(MemoryChunk {
                            geometry_revision: 1,
                            planning_range: Some(planning_range(terrain, index, &edit.rows)),
                            rows: Some(edit.rows),
                        }),
                    )
                })
                .collect::<Vec<_>>();
            slot.rows = slot
                .rows
                .replaced_many(&replacements)
                .expect("initial public history");
            slot.surface = terrain.chunks.clone();
            slot.initialized = true;
        }
        update_slot(
            terrain,
            slot,
            viewer,
            &permissions,
            history.coverage.as_deref(),
            tick,
            phase,
        );
    }
    let captured = CapturedHistory {
        slots: history
            .slots
            .iter()
            .map(|(&viewer, slot)| (viewer, slot.rows.clone()))
            .collect(),
        coverage: history.coverage.clone(),
    };
    world.insert_resource(history);
    Some(captured)
}
