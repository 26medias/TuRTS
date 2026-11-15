use bevy_ecs::prelude::Resource;
use std::{collections::BTreeMap, fmt, sync::Arc};
use turts_mod_api::{ContentCatalog, ContentId, ResolvedTerrainDef, TerrainCell};
use turts_rules::terrain::{
    generation::{self, DatumBand, Geology, WeightedMaterial},
    geometry::{self, Cell, Grid, Surface},
    material::{self, Column, Prism},
};

/// Privileged boot input. It has no Debug or serialization implementation.
#[derive(Resource, Clone, Copy)]
pub struct PrivateTerrainSeed(u64);
impl PrivateTerrainSeed {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainError {
    InvalidGrid,
    InvalidHeight,
    OverlappingHeights,
    UnknownMaterial,
    MissingSeed,
    Allocation,
    Geometry(geometry::Error),
    Material(material::Error),
    Generation(generation::Error),
}
/// Private retained surface cache. A shared chunk never includes buried intervals or seed.
#[derive(Clone)]
struct SurfaceChunk {
    revision: u64,
    elevations: Vec<i32>,
}
#[derive(Resource)]
pub(crate) struct TerrainState {
    pub(crate) grid: Grid,
    pub(crate) prism: Prism,
    chunk_width: u32,
    chunk_columns: u32,
    pub(crate) columns: Vec<Column>,
    pub(crate) materials: Vec<ContentId>,
    chunks: Vec<Arc<SurfaceChunk>>,
    dirty_chunks: Vec<bool>,
    pub(crate) step_observations: BTreeMap<(u8, Cell), super::TerrainSurfaceWitness>,
}
// StagedPackage needs Debug. Never print realized columns or private seed.
impl fmt::Debug for TerrainState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TerrainState")
            .field("grid", &self.grid)
            .field("chunk_count", &self.chunks.len())
            .finish_non_exhaustive()
    }
}
pub(crate) struct PreparedCellEdit {
    pub(crate) index: usize,
    pub(crate) cell: Cell,
    pub(crate) expected_revision: u64,
    pub(crate) column: Column,
    chunk_index: usize,
    expected_chunk_revision: u64,
    chunk: Arc<SurfaceChunk>,
}
impl Surface for TerrainState {
    fn elevation(&self, cell: Cell) -> Option<i32> {
        self.index(cell)
            .and_then(|index| self.columns.get(index))
            .map(Column::surface)
    }
}
impl TerrainState {
    pub(crate) fn index(&self, cell: Cell) -> Option<usize> {
        if cell.x >= self.grid.columns || cell.y >= self.grid.rows {
            return None;
        }
        usize::try_from(u64::from(cell.y) * u64::from(self.grid.columns) + u64::from(cell.x)).ok()
    }
    fn chunk_location(&self, cell: Cell) -> Option<(usize, usize)> {
        self.index(cell)?;
        let cx = cell.x / self.chunk_width;
        let cy = cell.y / self.chunk_width;
        let width = self
            .chunk_width
            .min(self.grid.columns - cx * self.chunk_width);
        let chunk =
            usize::try_from(u64::from(cy) * u64::from(self.chunk_columns) + u64::from(cx)).ok()?;
        let offset = usize::try_from(
            u64::from(cell.y % self.chunk_width) * u64::from(width)
                + u64::from(cell.x % self.chunk_width),
        )
        .ok()?;
        Some((chunk, offset))
    }
    /// Prepare a single changed column and a copied local chunk before transaction mutation.
    pub(crate) fn prepare_cell(
        &self,
        cell: Cell,
        column: Column,
    ) -> Result<PreparedCellEdit, TerrainError> {
        let index = self.index(cell).ok_or(TerrainError::InvalidGrid)?;
        let (chunk_index, offset) = self.chunk_location(cell).ok_or(TerrainError::InvalidGrid)?;
        let previous = &self.chunks[chunk_index];
        let revision = previous
            .revision
            .checked_add(1)
            .ok_or(TerrainError::Material(material::Error::ArithmeticOverflow))?;
        let mut elevations = Vec::new();
        elevations
            .try_reserve_exact(previous.elevations.len())
            .map_err(|_| TerrainError::Allocation)?;
        elevations.extend_from_slice(&previous.elevations);
        elevations[offset] = column.surface();
        Ok(PreparedCellEdit {
            index,
            cell,
            expected_revision: self.columns[index].revision(),
            column,
            chunk_index,
            expected_chunk_revision: previous.revision,
            chunk: Arc::new(SurfaceChunk {
                revision,
                elevations,
            }),
        })
    }
    pub(crate) fn validates(&self, edit: &PreparedCellEdit) -> bool {
        self.columns
            .get(edit.index)
            .is_some_and(|column| column.revision() == edit.expected_revision)
            && self
                .chunks
                .get(edit.chunk_index)
                .is_some_and(|chunk| chunk.revision == edit.expected_chunk_revision)
    }
    pub(crate) fn apply_prevalidated(&mut self, edit: PreparedCellEdit) {
        debug_assert!(self.validates(&edit));
        self.columns[edit.index] = edit.column;
        self.chunks[edit.chunk_index] = edit.chunk;
        self.dirty_chunks[edit.chunk_index] = true;
    }
    pub(crate) fn material(&self, key: u32) -> Option<&ContentId> {
        self.materials.get(usize::try_from(key).ok()?)
    }
    pub(crate) fn key(&self, id: &ContentId) -> Option<u32> {
        self.materials
            .binary_search(id)
            .ok()
            .and_then(|index| u32::try_from(index).ok())
    }
    pub(crate) fn changed_chunks(&self) -> usize {
        self.dirty_chunks.iter().filter(|dirty| **dirty).count()
    }
    pub(crate) fn clear_changed_chunks(&mut self) {
        self.dirty_chunks.fill(false);
    }
}

/// Build complete private columns and immutable surface chunks without changing the World.
/// The caller supplies runtime-resolved public rules and the private server seed explicitly.
pub(crate) fn prepare(
    catalog: &ContentCatalog,
    definition: &ResolvedTerrainDef,
    seed: PrivateTerrainSeed,
) -> Result<TerrainState, TerrainError> {
    let grid = Grid::checked(
        definition.columns,
        definition.rows,
        definition.length_scale,
        definition.cell_width,
        definition.elevation_step,
        definition.floor,
        definition.ceiling,
    )
    .map_err(TerrainError::Geometry)?;
    let count64 = u64::from(grid.columns) * u64::from(grid.rows);
    material::storage_bound(count64, grid.floor, grid.ceiling).map_err(TerrainError::Material)?;
    let count = usize::try_from(count64).map_err(|_| TerrainError::InvalidGrid)?;
    if definition.chunk_width == 0 {
        return Err(TerrainError::InvalidGrid);
    }
    let mut initial = Vec::new();
    initial
        .try_reserve_exact(count)
        .map_err(|_| TerrainError::Allocation)?;
    initial.resize(count, definition.initial_elevation);
    let mut patched = Vec::new();
    patched
        .try_reserve_exact(count)
        .map_err(|_| TerrainError::Allocation)?;
    patched.resize(count, false);
    for patch in &definition.initial_heights {
        if patch.cells.min.x >= patch.cells.max.x
            || patch.cells.min.y >= patch.cells.max.y
            || patch.cells.max.x > grid.columns
            || patch.cells.max.y > grid.rows
        {
            return Err(TerrainError::InvalidGrid);
        }
        for y in patch.cells.min.y..patch.cells.max.y {
            for x in patch.cells.min.x..patch.cells.max.x {
                let index = usize::try_from(u64::from(y) * u64::from(grid.columns) + u64::from(x))
                    .map_err(|_| TerrainError::InvalidGrid)?;
                if patched[index] {
                    return Err(TerrainError::OverlappingHeights);
                }
                patched[index] = true;
                initial[index] = patch.elevation;
            }
        }
    }
    if initial
        .iter()
        .any(|height| *height < grid.floor || *height > grid.ceiling)
    {
        return Err(TerrainError::InvalidHeight);
    }
    let maximum_initial = initial
        .iter()
        .copied()
        .max()
        .ok_or(TerrainError::InvalidGrid)?;
    let materials = catalog
        .terrain_materials
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let keys = materials
        .iter()
        .enumerate()
        .map(|(index, id)| {
            u32::try_from(index)
                .map(|key| (id, key))
                .map_err(|_| TerrainError::InvalidGrid)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let bands = definition
        .generation
        .iter()
        .map(|band| {
            let choices = band
                .choices
                .iter()
                .map(|choice| {
                    let material = keys
                        .get(&choice.material)
                        .copied()
                        .ok_or(TerrainError::UnknownMaterial)?;
                    Ok(WeightedMaterial {
                        material,
                        weight: choice.weight,
                    })
                })
                .collect::<Result<Vec<_>, TerrainError>>()?;
            Ok(DatumBand {
                bottom: band.bottom,
                top: band.top,
                choices,
            })
        })
        .collect::<Result<Vec<_>, TerrainError>>()?;
    let geology = Geology::new(
        generation::GENERATOR_VERSION,
        seed.0,
        grid.columns,
        grid.rows,
        grid.floor,
        grid.ceiling,
        maximum_initial,
        bands,
    )
    .map_err(TerrainError::Generation)?;
    let mut columns = Vec::new();
    columns
        .try_reserve_exact(count)
        .map_err(|_| TerrainError::Allocation)?;
    for y in 0..grid.rows {
        for x in 0..grid.columns {
            let index = usize::try_from(u64::from(y) * u64::from(grid.columns) + u64::from(x))
                .map_err(|_| TerrainError::InvalidGrid)?;
            columns.push(
                geology
                    .column(x, y, initial[index])
                    .map_err(TerrainError::Generation)?,
            );
        }
    }
    let chunk_columns = grid.columns.div_ceil(definition.chunk_width);
    let chunk_rows = grid.rows.div_ceil(definition.chunk_width);
    let chunk_count = usize::try_from(u64::from(chunk_columns) * u64::from(chunk_rows))
        .map_err(|_| TerrainError::InvalidGrid)?;
    let mut chunks = Vec::new();
    chunks
        .try_reserve_exact(chunk_count)
        .map_err(|_| TerrainError::Allocation)?;
    for cy in 0..chunk_rows {
        for cx in 0..chunk_columns {
            let x0 = cx * definition.chunk_width;
            let y0 = cy * definition.chunk_width;
            let width = definition.chunk_width.min(grid.columns - x0);
            let height = definition.chunk_width.min(grid.rows - y0);
            let mut elevations = Vec::new();
            elevations
                .try_reserve_exact(
                    usize::try_from(u64::from(width) * u64::from(height))
                        .map_err(|_| TerrainError::InvalidGrid)?,
                )
                .map_err(|_| TerrainError::Allocation)?;
            for y in y0..y0 + height {
                for x in x0..x0 + width {
                    let index =
                        usize::try_from(u64::from(y) * u64::from(grid.columns) + u64::from(x))
                            .map_err(|_| TerrainError::InvalidGrid)?;
                    elevations.push(initial[index]);
                }
            }
            chunks.push(Arc::new(SurfaceChunk {
                revision: 0,
                elevations,
            }));
        }
    }
    let prism = Prism::new(
        u64::from(grid.length_scale),
        u64::from(grid.cell_width),
        u64::from(grid.elevation_step),
    )
    .map_err(TerrainError::Material)?;
    Ok(TerrainState {
        grid,
        prism,
        chunk_width: definition.chunk_width,
        chunk_columns,
        columns,
        materials,
        dirty_chunks: vec![false; chunk_count],
        chunks,
        step_observations: BTreeMap::new(),
    })
}

pub(crate) fn cell(cell: TerrainCell) -> Cell {
    Cell {
        x: cell.x,
        y: cell.y,
    }
}
