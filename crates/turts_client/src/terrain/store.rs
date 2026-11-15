//! Stage complete permitted terrain changes before the replica's infallible commit boundary.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use turts_mod_api::{ContentId, KnowledgeBasis, TerrainCell, TerrainDef, TerrainRect};
use turts_net_protocol::{
    TerrainDelta, TerrainGroundAnchor, TerrainSnapshot, TerrainSurfaceRecord, TerrainWorkRecord,
};
use turts_rules::terrain::geometry::{self, Grid, Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    Layout,
    Identity,
    Dictionary,
    Surface,
    Anchor,
    Work,
    Duplicate,
    Missing,
    Revision,
}

/// Geometry from the verified public map. This deliberately omits generation rules.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub map: ContentId,
    pub grid: Grid,
    pub chunk_width: u32,
    initial: Arc<[i32]>,
}
impl Layout {
    pub fn new(map: ContentId, definition: &TerrainDef) -> Result<Self, Invalid> {
        let grid = Grid::checked(
            definition.columns,
            definition.rows,
            definition.length_scale,
            definition.cell_width,
            definition.elevation_step,
            definition.floor,
            definition.ceiling,
        )
        .map_err(|_| Invalid::Layout)?;
        if definition.chunk_width == 0
            || !(grid.floor..=grid.ceiling).contains(&definition.initial_elevation)
        {
            return Err(Invalid::Layout);
        }
        let count = (grid.columns as usize)
            .checked_mul(grid.rows as usize)
            .ok_or(Invalid::Layout)?;
        let mut initial = vec![definition.initial_elevation; count];
        let mut written = vec![false; count];
        for patch in &definition.initial_heights {
            validate_rect(grid, patch.cells, count)?;
            if !(grid.floor..=grid.ceiling).contains(&patch.elevation) {
                return Err(Invalid::Layout);
            }
            for y in patch.cells.min.y..patch.cells.max.y {
                for x in patch.cells.min.x..patch.cells.max.x {
                    let index = y as usize * grid.columns as usize + x as usize;
                    if std::mem::replace(&mut written[index], true) {
                        return Err(Invalid::Layout);
                    }
                    initial[index] = patch.elevation;
                }
            }
        }
        Ok(Self {
            map,
            grid,
            chunk_width: definition.chunk_width,
            initial: initial.into(),
        })
    }
    pub fn initial_elevation(&self, cell: TerrainCell) -> Option<i32> {
        self.index(cell).map(|index| self.initial[index])
    }
    pub fn index(&self, cell: TerrainCell) -> Option<usize> {
        (cell.x < self.grid.columns && cell.y < self.grid.rows)
            .then(|| cell.y as usize * self.grid.columns as usize + cell.x as usize)
    }
    pub fn chunk(&self, cell: TerrainCell) -> TerrainCell {
        TerrainCell {
            x: cell.x / self.chunk_width,
            y: cell.y / self.chunk_width,
        }
    }
    pub fn chunk_columns(&self) -> u32 {
        self.grid.columns.div_ceil(self.chunk_width)
    }
    pub fn chunk_rows(&self) -> u32 {
        self.grid.rows.div_ceil(self.chunk_width)
    }
    pub fn cell_width(&self) -> f32 {
        self.grid.cell_width as f32 / self.grid.length_scale as f32
    }
    pub fn elevation_height(&self, elevation: i32) -> f32 {
        elevation as f32 * self.grid.elevation_step as f32 / self.grid.length_scale as f32
    }
}

/// An immutable geometry view for the input adapter. Only changed chunks copy
/// when a new atomic state activates; the adapter never clones a whole height map.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceView {
    pub layout: Arc<Layout>,
    chunks: Vec<Option<Arc<Vec<i32>>>>,
}
impl SurfaceView {
    fn new(layout: Arc<Layout>) -> Self {
        let count = layout.chunk_columns() as usize * layout.chunk_rows() as usize;
        Self {
            layout,
            chunks: vec![None; count],
        }
    }
    pub fn elevation(&self, cell: TerrainCell) -> Option<i32> {
        self.layout.index(cell)?;
        let chunk = self.layout.chunk(cell);
        let index = chunk.y as usize * self.layout.chunk_columns() as usize + chunk.x as usize;
        let width = self
            .layout
            .chunk_width
            .min(self.layout.grid.columns - chunk.x * self.layout.chunk_width);
        let local = (cell.y % self.layout.chunk_width) as usize * width as usize
            + (cell.x % self.layout.chunk_width) as usize;
        self.chunks[index]
            .as_ref()
            .map(|heights| heights[local])
            .or_else(|| self.layout.initial_elevation(cell))
    }
    fn set(&mut self, cell: TerrainCell, elevation: i32) {
        if self.elevation(cell) == Some(elevation) {
            return;
        }
        let chunk = self.layout.chunk(cell);
        let index = chunk.y as usize * self.layout.chunk_columns() as usize + chunk.x as usize;
        let x0 = chunk.x * self.layout.chunk_width;
        let y0 = chunk.y * self.layout.chunk_width;
        let width = self.layout.chunk_width.min(self.layout.grid.columns - x0);
        let height = self.layout.chunk_width.min(self.layout.grid.rows - y0);
        let heights = self.chunks[index].get_or_insert_with(|| {
            let mut heights = Vec::with_capacity(width as usize * height as usize);
            for y in y0..y0 + height {
                for x in x0..x0 + width {
                    heights.push(
                        self.layout
                            .initial_elevation(TerrainCell { x, y })
                            .expect("validated cell"),
                    );
                }
            }
            Arc::new(heights)
        });
        Arc::make_mut(heights)[(cell.y - y0) as usize * width as usize + (cell.x - x0) as usize] =
            elevation;
    }
    pub(crate) fn retained_bytes(&self) -> Option<usize> {
        self.chunks.iter().try_fold(
            self.chunks
                .capacity()
                .checked_mul(std::mem::size_of::<Option<Arc<Vec<i32>>>>())?,
            |bytes, chunk| {
                bytes.checked_add(chunk.as_ref().map_or(0, |heights| {
                    heights
                        .capacity()
                        .saturating_mul(std::mem::size_of::<i32>())
                }))
            },
        )
    }
}
impl Surface for SurfaceView {
    fn elevation(&self, cell: geometry::Cell) -> Option<i32> {
        self.elevation(TerrainCell {
            x: cell.x,
            y: cell.y,
        })
    }
}
impl Surface for Layout {
    fn elevation(&self, cell: geometry::Cell) -> Option<i32> {
        self.initial_elevation(TerrainCell {
            x: cell.x,
            y: cell.y,
        })
    }
}

/// The replica supplies these answers from the candidate's validated knowledge/unit view.
/// An owned anchor is contact permission only; it does not make surface() return true.
pub trait Authority {
    fn surface(&self, cell: TerrainCell, observed_tick: u32, current: bool) -> bool;
    fn anchor(&self, row: &TerrainGroundAnchor) -> bool;
    fn required_anchors(&self) -> impl Iterator<Item = u32>;
    fn work(&self, row: &TerrainWorkRecord) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct PermittedTerrain {
    pub layout: Arc<Layout>,
    pub basis: KnowledgeBasis,
    pub geometry: Arc<SurfaceView>,
    /// Canonical public dictionary. Its entries convey no occurrence information.
    pub materials: Arc<[String]>,
    pub surfaces: BTreeMap<TerrainCell, TerrainSurfaceRecord>,
    pub anchors: BTreeMap<u32, TerrainGroundAnchor>,
    pub work: BTreeMap<u32, TerrainWorkRecord>,
    revisions: BTreeMap<TerrainCell, u64>,
}

/// Entirely validated before mutation. No private server revision is represented here.
pub struct Patch {
    basis: KnowledgeBasis,
    delta: TerrainDelta,
    revisions: Vec<(TerrainCell, u64)>,
}

impl PermittedTerrain {
    pub fn stage_snapshot(
        layout: Arc<Layout>,
        basis: &KnowledgeBasis,
        dictionary: &[String],
        snapshot: &TerrainSnapshot,
        authority: &impl Authority,
        previous: Option<&Self>,
    ) -> Result<Self, Invalid> {
        if basis.map != layout.map {
            return Err(Invalid::Identity);
        }
        if snapshot.materials != dictionary {
            return Err(Invalid::Dictionary);
        }
        let geometry = Arc::new(SurfaceView::new(layout.clone()));
        let mut next = Self {
            layout,
            geometry,
            basis: basis.clone(),
            materials: dictionary.to_vec().into(),
            surfaces: BTreeMap::new(),
            anchors: BTreeMap::new(),
            work: BTreeMap::new(),
            revisions: BTreeMap::new(),
        };
        for row in &snapshot.surfaces {
            next.validate_surface(row, basis, authority)?;
            if next.surfaces.insert(row.cell, row.clone()).is_some() {
                return Err(Invalid::Duplicate);
            }
        }
        for row in &snapshot.anchors {
            next.validate_anchor(row, basis, authority)?;
            if next.anchors.insert(row.unit.0, row.clone()).is_some() {
                return Err(Invalid::Duplicate);
            }
        }
        for row in &snapshot.work {
            next.validate_work(row, authority)?;
            if next.work.insert(row.unit.0, row.clone()).is_some() {
                return Err(Invalid::Duplicate);
            }
        }
        if authority
            .required_anchors()
            .any(|id| !next.anchors.contains_key(&id))
        {
            return Err(Invalid::Anchor);
        }
        let previous = previous
            .filter(|old| old.basis.match_id == basis.match_id && old.layout == next.layout);
        if let Some(old) = previous {
            next.revisions.clone_from(&old.revisions);
            next.geometry = old.geometry.clone();
            let cells = old
                .surfaces
                .keys()
                .chain(next.surfaces.keys())
                .copied()
                .collect::<BTreeSet<_>>();
            let mut dirty = BTreeSet::new();
            for cell in cells {
                if visual(old.surfaces.get(&cell)) != visual(next.surfaces.get(&cell)) {
                    next.dirty_neighbors(cell, &mut dirty);
                }
            }
            for chunk in dirty {
                let revision = old
                    .revision(chunk)
                    .checked_add(1)
                    .ok_or(Invalid::Revision)?;
                next.revisions.insert(chunk, revision);
            }
        }
        let cells = next
            .surfaces
            .keys()
            .chain(previous.into_iter().flat_map(|old| old.surfaces.keys()))
            .copied()
            .collect::<BTreeSet<_>>();
        for cell in cells {
            let elevation = next
                .surfaces
                .get(&cell)
                .map(|row| row.elevation)
                .or_else(|| next.layout.initial_elevation(cell))
                .expect("validated cell");
            if next.geometry.elevation(cell) != Some(elevation) {
                Arc::make_mut(&mut next.geometry).set(cell, elevation);
            }
        }
        Ok(next)
    }
    pub fn stage_delta(
        &self,
        basis: &KnowledgeBasis,
        delta: &TerrainDelta,
        authority: &impl Authority,
    ) -> Result<Patch, Invalid> {
        if basis.match_id != self.basis.match_id
            || basis.map != self.basis.map
            || basis.tick <= self.basis.tick
        {
            return Err(Invalid::Identity);
        }
        let mut surfaces = BTreeSet::new();
        let mut anchors = BTreeSet::new();
        let mut work = BTreeSet::new();
        let mut dirty = BTreeSet::new();
        for row in &delta.surfaces {
            self.validate_surface(row, basis, authority)?;
            if !surfaces.insert(row.cell) {
                return Err(Invalid::Duplicate);
            }
            if visual(self.surfaces.get(&row.cell)) != visual(Some(row)) {
                self.dirty_neighbors(row.cell, &mut dirty);
            }
        }
        for &cell in &delta.surfaces_cleared {
            if !surfaces.insert(cell) {
                return Err(Invalid::Duplicate);
            }
            if !self.surfaces.contains_key(&cell) {
                return Err(Invalid::Missing);
            }
            self.dirty_neighbors(cell, &mut dirty);
        }
        for row in &delta.anchors {
            self.validate_anchor(row, basis, authority)?;
            if !anchors.insert(row.unit.0) {
                return Err(Invalid::Duplicate);
            }
        }
        for unit in &delta.anchors_cleared {
            if !anchors.insert(unit.0) {
                return Err(Invalid::Duplicate);
            }
            if !self.anchors.contains_key(&unit.0) {
                return Err(Invalid::Missing);
            }
        }
        for row in &delta.work {
            self.validate_work(row, authority)?;
            if !work.insert(row.unit.0) {
                return Err(Invalid::Duplicate);
            }
        }
        for unit in &delta.work_cleared {
            if !work.insert(unit.0) {
                return Err(Invalid::Duplicate);
            }
            if !self.work.contains_key(&unit.0) {
                return Err(Invalid::Missing);
            }
        }
        for (cell, row) in &self.surfaces {
            if !surfaces.contains(cell) {
                self.validate_surface(row, basis, authority)?;
            }
        }
        for (id, row) in &self.anchors {
            if !anchors.contains(id) {
                self.validate_anchor(row, basis, authority)?;
            }
        }
        for (id, row) in &self.work {
            if !work.contains(id) {
                self.validate_work(row, authority)?;
            }
        }
        let replacements = delta
            .anchors
            .iter()
            .map(|row| (row.unit.0, row))
            .collect::<BTreeMap<_, _>>();
        let cleared = delta
            .anchors_cleared
            .iter()
            .map(|unit| unit.0)
            .collect::<BTreeSet<_>>();
        if authority.required_anchors().any(|id| {
            !replacements.contains_key(&id)
                && (cleared.contains(&id) || !self.anchors.contains_key(&id))
        }) {
            return Err(Invalid::Anchor);
        }
        let revisions = dirty
            .into_iter()
            .map(|chunk| {
                self.revision(chunk)
                    .checked_add(1)
                    .map(|revision| (chunk, revision))
                    .ok_or(Invalid::Revision)
            })
            .collect::<Result<_, _>>()?;
        Ok(Patch {
            basis: basis.clone(),
            delta: delta.clone(),
            revisions,
        })
    }
    /// Call only at the replica's commit boundary, after account/unit/knowledge staging succeeds.
    pub fn commit(&mut self, patch: Patch) {
        for cell in patch.delta.surfaces_cleared {
            if let Some(initial) = self.layout.initial_elevation(cell) {
                if self.geometry.elevation(cell) != Some(initial) {
                    Arc::make_mut(&mut self.geometry).set(cell, initial);
                }
            }
            self.surfaces.remove(&cell);
        }
        for row in patch.delta.surfaces {
            if self.geometry.elevation(row.cell) != Some(row.elevation) {
                Arc::make_mut(&mut self.geometry).set(row.cell, row.elevation);
            }
            self.surfaces.insert(row.cell, row);
        }
        for unit in patch.delta.anchors_cleared {
            self.anchors.remove(&unit.0);
        }
        for row in patch.delta.anchors {
            self.anchors.insert(row.unit.0, row);
        }
        for unit in patch.delta.work_cleared {
            self.work.remove(&unit.0);
        }
        for row in patch.delta.work {
            self.work.insert(row.unit.0, row);
        }
        self.revisions.extend(patch.revisions);
        self.basis = patch.basis;
    }
    pub fn elevation(&self, cell: TerrainCell) -> Option<i32> {
        self.geometry.elevation(cell)
    }
    pub fn revision(&self, chunk: TerrainCell) -> u64 {
        self.revisions.get(&chunk).copied().unwrap_or(0)
    }
    fn validate_surface(
        &self,
        row: &TerrainSurfaceRecord,
        basis: &KnowledgeBasis,
        authority: &impl Authority,
    ) -> Result<(), Invalid> {
        if self.layout.index(row.cell).is_none()
            || !(self.layout.grid.floor..=self.layout.grid.ceiling).contains(&row.elevation)
            || row.observed_tick > basis.tick
            || row
                .material
                .is_some_and(|id| id.0 as usize >= self.materials.len())
            || (row.material.is_none() != (row.elevation == self.layout.grid.floor))
            || row.material.is_some() != row.provenance.is_some()
            || !authority.surface(row.cell, row.observed_tick, row.current)
        {
            return Err(Invalid::Surface);
        }
        Ok(())
    }
    fn validate_anchor(
        &self,
        row: &TerrainGroundAnchor,
        basis: &KnowledgeBasis,
        authority: &impl Authority,
    ) -> Result<(), Invalid> {
        if row.observed_tick > basis.tick
            || !(self.layout.grid.floor..=self.layout.grid.ceiling).contains(&row.elevation)
            || !authority.anchor(row)
        {
            return Err(Invalid::Anchor);
        }
        Ok(())
    }
    fn validate_work(
        &self,
        row: &TerrainWorkRecord,
        authority: &impl Authority,
    ) -> Result<(), Invalid> {
        validate_rect(self.layout.grid, row.designation.cells, 256).map_err(|_| Invalid::Work)?;
        if !(self.layout.grid.floor..=self.layout.grid.ceiling)
            .contains(&row.designation.target_elevation)
            || row
                .current_cell
                .is_some_and(|cell| !in_rect(cell, row.designation.cells))
            || row
                .required_work
                .is_some_and(|required| required == 0 || row.completed_work > required)
            || (row.required_work.is_none() && row.completed_work != 0)
            || !authority.work(row)
        {
            return Err(Invalid::Work);
        }
        Ok(())
    }
    fn dirty_neighbors(&self, cell: TerrainCell, dirty: &mut BTreeSet<TerrainCell>) {
        dirty.insert(self.layout.chunk(cell));
        for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
            let (x, y) = (i64::from(cell.x) + dx, i64::from(cell.y) + dy);
            if x >= 0
                && y >= 0
                && x < i64::from(self.layout.grid.columns)
                && y < i64::from(self.layout.grid.rows)
            {
                dirty.insert(self.layout.chunk(TerrainCell {
                    x: x as u32,
                    y: y as u32,
                }));
            }
        }
    }
}
impl Surface for PermittedTerrain {
    fn elevation(&self, cell: geometry::Cell) -> Option<i32> {
        self.elevation(TerrainCell {
            x: cell.x,
            y: cell.y,
        })
    }
}
fn visual(row: Option<&TerrainSurfaceRecord>) -> Option<(i32, Option<u32>, bool)> {
    row.map(|row| (row.elevation, row.material.map(|id| id.0), row.current))
}
fn in_rect(cell: TerrainCell, rect: TerrainRect) -> bool {
    cell.x >= rect.min.x && cell.x < rect.max.x && cell.y >= rect.min.y && cell.y < rect.max.y
}
fn validate_rect(grid: Grid, rect: TerrainRect, limit: usize) -> Result<(), Invalid> {
    if rect.min.x >= rect.max.x
        || rect.min.y >= rect.max.y
        || rect.max.x > grid.columns
        || rect.max.y > grid.rows
        || u64::from(rect.max.x - rect.min.x) * u64::from(rect.max.y - rect.min.y) > limit as u64
    {
        return Err(Invalid::Layout);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{ObservationPhase, TerrainGenerationDef};
    use turts_net_protocol::TerrainMaterialId;
    struct Public;
    impl Authority for Public {
        fn surface(&self, _: TerrainCell, _: u32, _: bool) -> bool {
            true
        }
        fn anchor(&self, _: &TerrainGroundAnchor) -> bool {
            true
        }
        fn required_anchors(&self) -> impl Iterator<Item = u32> {
            std::iter::empty()
        }
        fn work(&self, _: &TerrainWorkRecord) -> bool {
            true
        }
    }
    pub(super) fn layout() -> Arc<Layout> {
        Arc::new(
            Layout::new(
                ContentId::parse("test:map").unwrap(),
                &TerrainDef {
                    length_scale: 4,
                    cell_width: 4,
                    elevation_step: 1,
                    columns: 8,
                    rows: 4,
                    chunk_width: 2,
                    initial_elevation: 2,
                    initial_heights: vec![],
                    floor: 0,
                    ceiling: 8,
                    generation: TerrainGenerationDef::Version1 { bands: vec![] },
                },
            )
            .unwrap(),
        )
    }
    fn basis(tick: u32) -> KnowledgeBasis {
        KnowledgeBasis {
            match_id: 7,
            map: ContentId::parse("test:map").unwrap(),
            tick,
            phase: ObservationPhase::Completed,
        }
    }
    fn row(x: u32, y: u32, elevation: i32, tick: u32) -> TerrainSurfaceRecord {
        TerrainSurfaceRecord {
            cell: TerrainCell { x, y },
            elevation,
            material: Some(TerrainMaterialId(0)),
            provenance: Some(turts_mod_api::TerrainProvenance::Natural),
            observed_tick: tick,
            current: true,
        }
    }
    fn state() -> PermittedTerrain {
        PermittedTerrain::stage_snapshot(
            layout(),
            &basis(1),
            &["test:soil".into()],
            &TerrainSnapshot {
                materials: vec!["test:soil".into()],
                surfaces: vec![row(1, 1, 2, 1)],
                ..Default::default()
            },
            &Public,
            None,
        )
        .unwrap()
    }
    #[test]
    fn a_rejected_update_changes_neither_surface_anchor_nor_chunk_revision() {
        let current = state();
        let before = current.clone();
        let invalid = TerrainDelta {
            surfaces: vec![row(1, 1, 3, 2), row(8, 0, 3, 2)],
            ..Default::default()
        };
        assert!(current.stage_delta(&basis(2), &invalid, &Public).is_err());
        assert_eq!(current, before);
    }
    #[test]
    fn one_border_edit_updates_only_neighbor_chunks_and_keeps_the_old_view_immutable() {
        let mut current = state();
        let old = current.geometry.clone();
        let patch = current
            .stage_delta(
                &basis(2),
                &TerrainDelta {
                    surfaces: vec![row(1, 1, 3, 2)],
                    ..Default::default()
                },
                &Public,
            )
            .unwrap();
        current.commit(patch);
        assert_eq!(old.elevation(TerrainCell { x: 1, y: 1 }), Some(2));
        assert_eq!(current.elevation(TerrainCell { x: 1, y: 1 }), Some(3));
        assert_eq!(current.revision(TerrainCell { x: 0, y: 0 }), 1);
        assert_eq!(current.revision(TerrainCell { x: 1, y: 0 }), 1);
        assert_eq!(current.revision(TerrainCell { x: 0, y: 1 }), 1);
        assert_eq!(current.revision(TerrainCell { x: 1, y: 1 }), 0);
        assert_eq!(current.revision(TerrainCell { x: 3, y: 1 }), 0);
    }
    #[test]
    fn timestamp_only_refresh_does_not_rebuild_a_chunk_or_clone_geometry() {
        let mut current = state();
        let old = current.geometry.clone();
        let patch = current
            .stage_delta(
                &basis(2),
                &TerrainDelta {
                    surfaces: vec![row(1, 1, 2, 2)],
                    ..Default::default()
                },
                &Public,
            )
            .unwrap();
        current.commit(patch);
        assert!(Arc::ptr_eq(&old, &current.geometry));
        assert_eq!(current.revision(TerrainCell { x: 0, y: 0 }), 0);
    }
    #[test]
    fn stale_map_dictionary_duplicates_and_unanchored_bodies_fail_closed() {
        let mut wrong = basis(2);
        wrong.map = ContentId::parse("test:other").unwrap();
        assert!(
            state()
                .stage_delta(&wrong, &TerrainDelta::default(), &Public)
                .is_err()
        );
        let duplicate = TerrainDelta {
            surfaces: vec![row(1, 1, 3, 2), row(1, 1, 4, 2)],
            ..Default::default()
        };
        assert!(state().stage_delta(&basis(2), &duplicate, &Public).is_err());
        assert!(
            PermittedTerrain::stage_snapshot(
                layout(),
                &basis(1),
                &["test:soil".into()],
                &TerrainSnapshot::default(),
                &Public,
                None
            )
            .is_err()
        );
        struct Required;
        impl Authority for Required {
            fn surface(&self, _: TerrainCell, _: u32, _: bool) -> bool {
                true
            }
            fn anchor(&self, _: &TerrainGroundAnchor) -> bool {
                true
            }
            fn required_anchors(&self) -> impl Iterator<Item = u32> {
                [17].into_iter()
            }
            fn work(&self, _: &TerrainWorkRecord) -> bool {
                true
            }
        }
        assert!(
            state()
                .stage_delta(&basis(2), &TerrainDelta::default(), &Required)
                .is_err()
        );
    }
}
