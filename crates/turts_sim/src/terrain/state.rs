use bevy_ecs::prelude::Resource;
use std::{collections::BTreeMap, fmt, sync::Arc};
use turts_mod_api::{ContentCatalog, ContentId, ResolvedTerrainDef};
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
#[derive(Clone, PartialEq, Eq)]
pub(super) struct SurfaceChunk {
    pub(super) revision: u64,
    minimum: i32,
    maximum: i32,
    pub(super) elevations: Vec<i32>,
    pub(super) materials: Vec<Option<u32>>,
    pub(super) provenance: Vec<Option<turts_mod_api::TerrainProvenance>>,
}
const CHUNKS_PER_PAGE: usize = 1024;
/// Two-level COW table: an edit copies at most two public pointer pages plus its one chunk.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct PagedChunks<T> {
    pages: Arc<Vec<Arc<Vec<Arc<T>>>>>,
    len: usize,
}
pub(super) type ChunkTable = PagedChunks<SurfaceChunk>;
impl<T> PagedChunks<T> {
    pub(super) fn new(chunks: Vec<Arc<T>>) -> Result<Self, TerrainError> {
        let len = chunks.len();
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(len.div_ceil(CHUNKS_PER_PAGE))
            .map_err(|_| TerrainError::Allocation)?;
        let mut source = chunks.into_iter();
        while source.len() > 0 {
            pages.push(Arc::new(source.by_ref().take(CHUNKS_PER_PAGE).collect()));
        }
        Ok(Self {
            pages: Arc::new(pages),
            len,
        })
    }
    pub(super) fn get(&self, index: usize) -> Option<&Arc<T>> {
        self.pages
            .get(index / CHUNKS_PER_PAGE)?
            .get(index % CHUNKS_PER_PAGE)
    }
    pub(super) fn replaced_many(
        &self,
        replacements: &[(usize, Arc<T>)],
    ) -> Result<Self, TerrainError> {
        if replacements.is_empty() {
            return Ok(Self {
                pages: Arc::clone(&self.pages),
                len: self.len,
            });
        }
        if replacements.windows(2).any(|rows| rows[0].0 >= rows[1].0) {
            return Err(TerrainError::InvalidGrid);
        }
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(self.pages.len())
            .map_err(|_| TerrainError::Allocation)?;
        pages.extend(self.pages.iter().cloned());
        let mut cursor = 0;
        while cursor < replacements.len() {
            let page_index = replacements[cursor].0 / CHUNKS_PER_PAGE;
            let source = pages.get(page_index).ok_or(TerrainError::InvalidGrid)?;
            let mut page = Vec::new();
            page.try_reserve_exact(source.len())
                .map_err(|_| TerrainError::Allocation)?;
            page.extend(source.iter().cloned());
            while cursor < replacements.len()
                && replacements[cursor].0 / CHUNKS_PER_PAGE == page_index
            {
                let (index, value) = &replacements[cursor];
                *page
                    .get_mut(index % CHUNKS_PER_PAGE)
                    .ok_or(TerrainError::InvalidGrid)? = Arc::clone(value);
                cursor += 1;
            }
            pages[page_index] = Arc::new(page);
        }
        Ok(Self {
            pages: Arc::new(pages),
            len: self.len,
        })
    }
    pub(super) fn len(&self) -> usize {
        self.len
    }
    pub(super) fn replaced(&self, index: usize, value: Arc<T>) -> Result<Self, TerrainError> {
        let page_index = index / CHUNKS_PER_PAGE;
        let source = self
            .pages
            .get(page_index)
            .ok_or(TerrainError::InvalidGrid)?;
        let mut page = Vec::new();
        page.try_reserve_exact(source.len())
            .map_err(|_| TerrainError::Allocation)?;
        page.extend(source.iter().cloned());
        *page
            .get_mut(index % CHUNKS_PER_PAGE)
            .ok_or(TerrainError::InvalidGrid)? = value;
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(self.pages.len())
            .map_err(|_| TerrainError::Allocation)?;
        pages.extend(self.pages.iter().cloned());
        pages[page_index] = Arc::new(page);
        Ok(Self {
            pages: Arc::new(pages),
            len: self.len,
        })
    }
}
impl<T> std::ops::Index<usize> for PagedChunks<T> {
    type Output = Arc<T>;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("validated public chunk index")
    }
}
#[derive(Resource)]
pub(crate) struct TerrainState {
    pub(crate) grid: Grid,
    pub(crate) prism: Prism,
    pub(super) chunk_width: u32,
    pub(super) chunk_columns: u32,
    pub(crate) columns: Vec<Column>,
    pub(crate) materials: Arc<[ContentId]>,
    pub(super) initial: Arc<[i32]>,
    pub(super) initial_ranges: Arc<[(i32, i32)]>,
    pub(super) chunks: ChunkTable,
    table_revision: u64,
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
    table: ChunkTable,
    expected_table_revision: u64,
    next_table_revision: u64,
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
    pub(super) fn chunk_location(&self, cell: Cell) -> Option<(usize, usize)> {
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
        let mut materials = Vec::new();
        materials
            .try_reserve_exact(previous.materials.len())
            .map_err(|_| TerrainError::Allocation)?;
        materials.extend_from_slice(&previous.materials);
        materials[offset] = column.exposed().map(|exposed| exposed.material);
        let mut provenance = Vec::new();
        provenance
            .try_reserve_exact(previous.provenance.len())
            .map_err(|_| TerrainError::Allocation)?;
        provenance.extend_from_slice(&previous.provenance);
        provenance[offset] = column.exposed().map(|exposed| match exposed.provenance {
            material::Provenance::Natural => turts_mod_api::TerrainProvenance::Natural,
            material::Provenance::NoYield => turts_mod_api::TerrainProvenance::Placed,
        });

        let next_table_revision = self
            .table_revision
            .checked_add(1)
            .ok_or(TerrainError::Material(material::Error::ArithmeticOverflow))?;

        Ok(PreparedCellEdit {
            index,
            cell,
            expected_revision: self.columns[index].revision(),
            column,
            chunk_index,
            expected_chunk_revision: previous.revision,
            expected_table_revision: self.table_revision,
            next_table_revision,
            table: self.chunks.replaced(
                chunk_index,
                Arc::new(SurfaceChunk {
                    revision,
                    minimum: elevations
                        .iter()
                        .copied()
                        .min()
                        .ok_or(TerrainError::InvalidGrid)?,
                    maximum: elevations
                        .iter()
                        .copied()
                        .max()
                        .ok_or(TerrainError::InvalidGrid)?,
                    elevations,
                    materials,
                    provenance,
                }),
            )?,
        })
    }
    pub(crate) fn validates(&self, edit: &PreparedCellEdit) -> bool {
        self.table_revision == edit.expected_table_revision
            && self
                .columns
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
        self.chunks = edit.table;
        self.table_revision = edit.next_table_revision;
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
    pub(crate) fn lease_valid(&self, lease: &BTreeMap<usize, u64>) -> bool {
        lease.iter().all(|(index, revision)| {
            self.chunks
                .get(*index)
                .is_some_and(|chunk| chunk.revision == *revision)
        })
    }
    /// Every returned chunk is read by this rectangle certificate, so edits elsewhere do not
    /// invalidate it. The bounds come from exact endpoint disk support, not rounded poses.
    pub(crate) fn rectangle_certificate(
        &self,
        minimum: Cell,
        maximum: Cell,
        lease: &mut BTreeMap<usize, u64>,
    ) -> bool {
        let mut height = None;
        let mut flat = true;
        for cy in minimum.y / self.chunk_width..=maximum.y / self.chunk_width {
            for cx in minimum.x / self.chunk_width..=maximum.x / self.chunk_width {
                let index =
                    (u64::from(cy) * u64::from(self.chunk_columns) + u64::from(cx)) as usize;
                let Some(chunk) = self.chunks.get(index) else {
                    return false;
                };
                lease.insert(index, chunk.revision);
                flat &= chunk.minimum == chunk.maximum
                    && height.is_none_or(|height| height == chunk.minimum);
                height = Some(chunk.minimum);
            }
        }
        flat
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
            let mut top_materials = Vec::new();
            top_materials
                .try_reserve_exact(elevations.capacity())
                .map_err(|_| TerrainError::Allocation)?;
            for y in y0..y0 + height {
                for x in x0..x0 + width {
                    let index =
                        usize::try_from(u64::from(y) * u64::from(grid.columns) + u64::from(x))
                            .map_err(|_| TerrainError::InvalidGrid)?;
                    elevations.push(initial[index]);
                    top_materials.push(columns[index].exposed().map(|exposed| exposed.material));
                }
            }
            chunks.push(Arc::new(SurfaceChunk {
                revision: 0,
                minimum: elevations
                    .iter()
                    .copied()
                    .min()
                    .ok_or(TerrainError::InvalidGrid)?,
                maximum: elevations
                    .iter()
                    .copied()
                    .max()
                    .ok_or(TerrainError::InvalidGrid)?,
                elevations,
                provenance: top_materials
                    .iter()
                    .map(|key| key.map(|_| turts_mod_api::TerrainProvenance::Natural))
                    .collect(),
                materials: top_materials,
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
        materials: materials.into(),
        initial: initial.into(),
        dirty_chunks: vec![false; chunk_count],
        initial_ranges: chunks
            .iter()
            .map(|chunk| (chunk.minimum, chunk.maximum))
            .collect::<Vec<_>>()
            .into(),
        chunks: ChunkTable::new(chunks)?,
        table_revision: 0,
        step_observations: BTreeMap::new(),
    })
}
