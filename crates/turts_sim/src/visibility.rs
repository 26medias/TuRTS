//! Authoritative square sight, permitted phase bases, and remembered public facts.
//! Absence of this resource is the explicit legacy AllVisible mode.
use bevy_ecs::{prelude::Resource, world::World};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use turts_mod_api::{
    ContentId, Facing, KnowledgeBasis, KnowledgeSnapshot, ObservationPhase, RememberedNode,
    RememberedUnit, SLOT_COUNT, VisibilityDef, VisibilityGrid, VisionSharing, WorldPos,
};
use turts_rules::visibility::{Grid, Point};

/// Current public unit facts. This type contains no health, order, or economy state.
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleUnit {
    pub unit: u32,
    pub content: Arc<ContentId>,
    pub owner: u8,
    pub position: WorldPos,
    pub facing: Option<Facing>,
    pub site: bool,
    pub ground_elevation: Option<i32>,
}
impl VisibleUnit {
    fn remembered(&self, tick: u32) -> RememberedUnit {
        RememberedUnit {
            unit: self.unit,
            content: self.content.as_ref().clone(),
            owner: self.owner,
            position: self.position,
            facing: self.facing,
            site: self.site,
            last_seen_tick: tick,
            ground_elevation: self.ground_elevation,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleNode {
    pub node: u32,
    pub content: Arc<ContentId>,
    pub position: WorldPos,
    pub remaining: u64,
}
impl VisibleNode {
    fn remembered(&self, tick: u32) -> RememberedNode {
        RememberedNode {
            node: self.node,
            content: self.content.as_ref().clone(),
            position: self.position,
            remaining: self.remaining,
            last_seen_tick: tick,
        }
    }
}
#[derive(Debug, Clone)]
struct SlotBasis {
    observed: Arc<[u8]>,
    explored: Arc<[u8]>,
    allowed_units: Arc<[u8]>,
    units: Arc<[RememberedUnit]>,
    nodes: Arc<[RememberedNode]>,
    truncated: bool,
}
/// Immutable permission basis. Its unprojected internal rows are never public.
#[derive(Debug, Clone)]
pub struct VisibilityBasis {
    stamp: KnowledgeBasis,
    grid: VisibilityGrid,
    slots: Vec<SlotBasis>,
    units: Arc<[VisibleUnit]>,
    // Row plus one; zero denotes an absent ID. Sparse identities use binary search.
    unit_rows: Option<Arc<[u32]>>,
    nodes: Arc<[VisibleNode]>,
}
// At most 4 MiB per distinct retained identity set, shared across phase bases.
const MAX_DENSE_UNIT_ROWS: usize = 1_000_000;
fn dense_unit_rows(units: &[VisibleUnit]) -> Option<Arc<[u32]>> {
    let entries = usize::try_from(units.last()?.unit).ok()?.checked_add(1)?;
    if entries > MAX_DENSE_UNIT_ROWS || entries > units.len().saturating_mul(4) {
        return None;
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(entries).ok()?;
    rows.resize(entries, 0);
    for (index, unit) in units.iter().enumerate() {
        rows[unit.unit as usize] = u32::try_from(index).ok()?.checked_add(1)?;
    }
    Some(rows.into())
}
fn point(p: WorldPos) -> Point {
    Point { x: p.x, y: p.y }
}
fn bit(mask: &[u8], cell: u32) -> bool {
    mask.get(cell as usize / 8)
        .is_some_and(|byte| byte & (1 << (cell % 8)) != 0)
}
fn rule_grid(grid: VisibilityGrid) -> Grid {
    Grid {
        width: grid.size.x,
        height: grid.size.y,
        cell_size: grid.cell_size,
        columns: grid.columns,
        rows: grid.rows,
    }
}
impl VisibilityBasis {
    pub fn stamp(&self) -> &KnowledgeBasis {
        &self.stamp
    }
    pub fn grid(&self) -> VisibilityGrid {
        self.grid
    }
    pub(crate) fn fog_cell_observed(&self, viewer: u8, cell: u32) -> bool {
        self.slots
            .get(usize::from(viewer))
            .is_some_and(|slot| bit(&slot.observed, cell))
    }
    pub(crate) fn same_observation_mask(&self, viewer: u8, other: &Self) -> bool {
        self.grid == other.grid
            && self
                .slots
                .get(usize::from(viewer))
                .zip(other.slots.get(usize::from(viewer)))
                .is_some_and(|(left, right)| left.observed == right.observed)
    }
    pub fn point_observed(&self, viewer: u8, position: WorldPos) -> bool {
        self.slots.get(usize::from(viewer)).is_some_and(|slot| {
            rule_grid(self.grid)
                .cell(point(position))
                .is_some_and(|cell| bit(&slot.observed, cell))
        })
    }
    /// Exact positive-area terrain/fog overlap. Border tangency alone grants no extra sight.
    pub fn terrain_cell_observed(
        &self,
        viewer: u8,
        terrain: &turts_rules::terrain::geometry::Grid,
        cell: turts_rules::terrain::geometry::Cell,
        budget: &mut turts_rules::terrain::geometry::Budget,
    ) -> Result<bool, turts_rules::terrain::geometry::Error> {
        let Some(slot) = self.slots.get(usize::from(viewer)) else {
            return Ok(false);
        };
        terrain.cell_observed(cell, rule_grid(self.grid), budget, |cell| {
            bit(&slot.observed, cell)
        })
    }
    pub fn area_observed(&self, viewer: u8, min: WorldPos, max: WorldPos) -> bool {
        self.slots.get(usize::from(viewer)).is_some_and(|slot| {
            rule_grid(self.grid).rectangle(point(min), point(max), |cell| bit(&slot.observed, cell))
        })
    }
    pub fn permitted_unit(&self, viewer: u8, id: u32) -> Option<&VisibleUnit> {
        let slot = self.slots.get(usize::from(viewer))?;
        let index = if let Some(rows) = &self.unit_rows {
            rows.get(id as usize)?.checked_sub(1)? as usize
        } else {
            self.units
                .binary_search_by_key(&id, |unit| unit.unit)
                .ok()?
        };
        bit(&slot.allowed_units, index as u32).then(|| &self.units[index])
    }
    pub fn unit_permitted(&self, viewer: u8, id: u32) -> bool {
        self.permitted_unit(viewer, id).is_some()
    }
    /// Enumerate only units permitted by this immutable observation basis.
    pub(crate) fn permitted_units(&self, viewer: u8) -> impl Iterator<Item = &VisibleUnit> {
        let slot = self.slots.get(usize::from(viewer));
        self.units
            .iter()
            .enumerate()
            .filter_map(move |(index, unit)| {
                slot.filter(|slot| bit(&slot.allowed_units, index as u32))
                    .map(|_| unit)
            })
    }
    pub fn permitted_node(&self, viewer: u8, id: u32) -> Option<&VisibleNode> {
        let row = &self.nodes[self
            .nodes
            .binary_search_by_key(&id, |node| node.node)
            .ok()?];
        self.point_observed(viewer, row.position).then_some(row)
    }
    pub fn node_permitted(&self, viewer: u8, id: u32) -> bool {
        self.permitted_node(viewer, id).is_some()
    }
    pub fn knowledge(&self, viewer: u8) -> Option<KnowledgeSnapshot> {
        let slot = self.slots.get(usize::from(viewer))?;
        Some(KnowledgeSnapshot {
            basis: self.stamp.clone(),
            viewer,
            grid: self.grid,
            observed: slot.observed.to_vec(),
            explored: slot.explored.to_vec(),
            remembered_units: slot.units.to_vec(),
            remembered_nodes: slot.nodes.to_vec(),
            retention_truncated: slot.truncated,
        })
    }
}
#[derive(Debug, Clone, PartialEq)]
struct Source {
    owner: u8,
    position: WorldPos,
    radius: f32,
    cells: Option<Vec<u32>>,
}
/// Private optimization storage. Exhaustion changes work, never sight or gameplay rules.
pub const MAX_CACHED_SOURCE_CELL_BYTES: usize = 8 * 1024 * 1024;
fn cached_source_cells(
    grid: Grid,
    position: WorldPos,
    radius: f32,
    remaining_bytes: usize,
) -> Option<Vec<u32>> {
    let limit = remaining_bytes / std::mem::size_of::<u32>();
    if limit == 0 {
        return None;
    }
    let mut cells = Some(Vec::<u32>::new());
    grid.circle(point(position), radius, |cell| {
        let Some(buffer) = &mut cells else {
            return;
        };
        if buffer.len() == limit {
            cells = None;
            return;
        }
        if buffer.len() == buffer.capacity() {
            let capacity = buffer.capacity().saturating_mul(2).max(4).min(limit);
            if buffer.try_reserve_exact(capacity - buffer.len()).is_err()
                || buffer.capacity() > limit
            {
                cells = None;
                return;
            }
        }
        buffer.push(cell);
    });
    cells
}
impl Source {
    fn cached_bytes(&self) -> usize {
        self.cells
            .as_ref()
            .map_or(0, |cells| cells.capacity() * std::mem::size_of::<u32>())
    }
}
#[derive(Debug, Clone, Default)]
struct SlotMemory {
    // Current facts live in the immutable basis. Store only hidden sightings here.
    units: BTreeMap<u32, (VisibleUnit, u32)>,
    nodes: BTreeMap<u32, (VisibleNode, u32)>,
    truncated: bool,
}
#[derive(Debug, Clone)]
struct Coverage {
    counts: Vec<u32>,
    observed: Arc<[u8]>,
    explored: Arc<[u8]>,
}
/// Same-tick event facts are separate from the completed live entity list.
#[derive(Debug, Clone)]
pub struct EventUnitFact {
    pub basis: KnowledgeBasis,
    pub unit: VisibleUnit,
}
#[derive(Debug, Clone)]
pub struct PreparedEventUnit {
    fact: EventUnitFact,
    viewers: Vec<u8>,
    all_visible_terrain: bool,
}
// One tick of commit witnesses, not a terrain or entity history. At overflow the
// entire set becomes incomplete; unit IDs never select retained winners.
const ALL_VISIBLE_TERRAIN_EVENT_LIMIT: usize = turts_mod_api::MAX_REMEMBERED_SUBJECTS as usize;
#[derive(Resource)]
pub(crate) struct AllVisibleTerrainEvents {
    match_id: u64,
    map: ContentId,
    tick: u32,
    phase: ObservationPhase,
    complete: bool,
    events: Vec<PreparedEventUnit>,
}
impl AllVisibleTerrainEvents {
    fn advance(&mut self, tick: u32, phase: ObservationPhase) {
        if self.tick != tick {
            self.tick = tick;
            self.events.clear();
            self.complete = true;
        }
        self.phase = phase;
    }
    fn commit(&mut self, prepared: PreparedEventUnit, limit: usize) {
        if !self.complete {
            return;
        }
        if let Some(existing) = self.events.iter_mut().find(|event| {
            event.fact.basis == prepared.fact.basis
                && event.fact.unit.unit == prepared.fact.unit.unit
        }) {
            *existing = prepared;
            return;
        }
        if self.events.len() >= limit {
            self.events.clear();
            self.complete = false;
            return;
        }
        if self.events.len() == self.events.capacity() {
            let capacity = self.events.capacity().saturating_mul(2).max(4).min(limit);
            if self
                .events
                .try_reserve_exact(capacity - self.events.len())
                .is_err()
                || self.events.capacity() > limit
            {
                self.events = Vec::new();
                self.complete = false;
                return;
            }
        }
        self.events.push(prepared);
    }
}
fn event_tick(world: &World) -> u32 {
    world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0)
}
fn terrain_context_matches(world: &World, context: &AllVisibleTerrainEvents) -> bool {
    world.contains_resource::<crate::terrain::TerrainState>()
        && world
            .get_resource::<crate::bootstrap::InstalledMapIdentity>()
            .is_none_or(|map| map.0 == context.map)
}
/// Bind actual host identity for terrain-only AllVisible commit witnesses.
/// Fresh package installation must remove the old context before another match.
pub fn initialize_all_visible_terrain_event_context(
    world: &mut World,
    match_id: u64,
    map: ContentId,
) -> Result<(), String> {
    if event_tick(world) != 0 {
        return Err("terrain event context can only be finalized at tick zero".into());
    }
    if world.contains_resource::<VisibilityState>()
        || !world.contains_resource::<crate::terrain::TerrainState>()
        || world
            .get_resource::<crate::bootstrap::InstalledMapIdentity>()
            .is_some_and(|installed| installed.0 != map)
    {
        return Err("terrain event context requires the installed AllVisible terrain map".into());
    }
    if let Some(existing) = world.get_resource::<AllVisibleTerrainEvents>() {
        return if existing.match_id == match_id && existing.map == map {
            Ok(())
        } else {
            Err("terrain event match context is already finalized".into())
        };
    }
    world.insert_resource(AllVisibleTerrainEvents {
        match_id,
        map,
        tick: 0,
        phase: ObservationPhase::Start,
        complete: true,
        events: Vec::new(),
    });
    Ok(())
}
/// AllVisible terrain senders must refuse incomplete event witness publication.
/// Ordinary flat and restricted-fog behavior is unchanged.
pub fn event_units_complete(world: &World) -> bool {
    world.contains_resource::<VisibilityState>()
        || !world.contains_resource::<crate::terrain::TerrainState>()
        || world
            .get_resource::<AllVisibleTerrainEvents>()
            .is_some_and(|context| {
                terrain_context_matches(world, context)
                    && (context.tick < event_tick(world)
                        || (context.tick == event_tick(world) && context.complete))
            })
}

#[derive(Resource, Debug, Clone)]
pub struct VisibilityState {
    definition: VisibilityDef,
    context_finalized: bool,
    grid: VisibilityGrid,
    match_id: u64,
    map: ContentId,
    teams: Vec<u8>,
    coverage: Vec<Coverage>,
    sources: BTreeMap<u32, Source>,
    source_cache_bytes: usize,
    memories: Vec<SlotMemory>,
    start: Arc<VisibilityBasis>,
    tactical: Arc<VisibilityBasis>,
    completed: Arc<VisibilityBasis>,
    active: ObservationPhase,
    event_tick: u32,
    events: Vec<PreparedEventUnit>,
    observed_removals: Vec<BTreeSet<u32>>,
}

/// Stage an initialized, fail-closed restricted policy before world mutation.
/// AllVisible callers remove the resource explicitly instead of calling this.
pub fn prepare(
    match_id: u64,
    map: ContentId,
    definition: &VisibilityDef,
    grid: VisibilityGrid,
    teams: &[u8],
) -> Result<VisibilityState, String> {
    let checked =
        Grid::checked(grid.size.x, grid.size.y, grid.cell_size).ok_or("invalid sight grid")?;
    if checked.columns != grid.columns
        || checked.rows != grid.rows
        || definition.cell_size != grid.cell_size
        || !(1..=turts_mod_api::MAX_REMEMBERED_SUBJECTS)
            .contains(&definition.effective_memory_limit())
    {
        return Err("sight grid differs from its validated definition".into());
    }
    let cells = u64::from(grid.columns) * u64::from(grid.rows);
    if cells > turts_mod_api::MAX_VISIBILITY_CELLS {
        return Err("sight grid cell limit exceeded".into());
    }
    let cells = usize::try_from(cells).map_err(|_| "sight grid does not fit this target")?;
    let bytes = cells.div_ceil(8);
    let mut coverage = Vec::new();
    let mut slots = Vec::new();
    for _ in 0..SLOT_COUNT {
        let mut counts = Vec::new();
        counts
            .try_reserve_exact(cells)
            .map_err(|_| "cannot allocate sight coverage")?;
        counts.resize(cells, 0);
        let observed: Arc<[u8]> = vec![0; bytes].into();
        let explored = Arc::clone(&observed);
        slots.push(SlotBasis {
            observed: Arc::clone(&observed),
            explored: Arc::clone(&explored),
            allowed_units: Arc::from([]),
            units: Arc::from([]),
            nodes: Arc::from([]),
            truncated: false,
        });
        coverage.push(Coverage {
            counts,
            observed,
            explored,
        });
    }
    let empty = |phase| {
        Arc::new(VisibilityBasis {
            stamp: KnowledgeBasis {
                match_id,
                map: map.clone(),
                tick: 0,
                phase,
            },
            grid,
            slots: slots.clone(),
            units: Arc::from([]),
            unit_rows: None,
            nodes: Arc::from([]),
        })
    };
    Ok(VisibilityState {
        definition: definition.clone(),
        context_finalized: false,
        grid,
        match_id,
        map: map.clone(),
        teams: teams.to_vec(),
        coverage,
        sources: BTreeMap::new(),
        source_cache_bytes: 0,
        memories: vec![SlotMemory::default(); usize::from(SLOT_COUNT)],
        start: empty(ObservationPhase::Start),
        tactical: empty(ObservationPhase::Tactical),
        completed: empty(ObservationPhase::Completed),
        active: ObservationPhase::Start,
        event_tick: 0,
        events: Vec::new(),
        observed_removals: vec![BTreeSet::new(); usize::from(SLOT_COUNT)],
    })
}
impl VisibilityState {
    /// Retained cell-index optimization storage. Geometry fallback remains exact when full.
    pub fn cached_source_cell_bytes(&self) -> usize {
        self.source_cache_bytes
    }
    pub fn phase(&self, phase: ObservationPhase) -> Arc<VisibilityBasis> {
        Arc::clone(match phase {
            ObservationPhase::Start => &self.start,
            ObservationPhase::Tactical => &self.tactical,
            ObservationPhase::Completed => &self.completed,
        })
    }
    fn shared(&self, viewer: u8, owner: u8) -> bool {
        viewer == owner
            || self.definition.sharing == VisionSharing::Team
                && self
                    .teams
                    .get(usize::from(viewer))
                    .zip(self.teams.get(usize::from(owner)))
                    .is_some_and(|(a, b)| a == b)
    }
    fn adjust_source(&mut self, source: &Source, add: bool) {
        for viewer in 0..SLOT_COUNT {
            if !self.shared(viewer, source.owner) {
                continue;
            }
            let coverage = &mut self.coverage[usize::from(viewer)];
            // Detach each phase mask once, rather than checking Arc ownership for
            // every cell in every source circle. Removal cannot explore new cells.
            let observed = Arc::make_mut(&mut coverage.observed);
            let mut explored = add.then(|| Arc::make_mut(&mut coverage.explored));
            let mut adjust = |cell: u32| {
                let count = &mut coverage.counts[cell as usize];
                // A live source has one allocator-unique unit identity. Coverage
                // counts cannot exceed the number of representable unit IDs.
                if add {
                    *count += 1;
                } else {
                    *count -= 1;
                }
                let flag = 1 << (cell % 8);
                let byte = cell as usize / 8;
                if *count > 0 {
                    observed[byte] |= flag;
                    if let Some(explored) = &mut explored {
                        explored[byte] |= flag;
                    }
                } else {
                    observed[byte] &= !flag;
                }
            };
            if let Some(cells) = &source.cells {
                for cell in cells {
                    adjust(*cell);
                }
            } else {
                rule_grid(self.grid).circle(point(source.position), source.radius, adjust);
            }
        }
    }
    fn adjust_cached_source_delta(&mut self, owner: u8, old: &[u32], new: &[u32]) {
        for viewer in 0..SLOT_COUNT {
            if !self.shared(viewer, owner) {
                continue;
            }
            let coverage = &mut self.coverage[usize::from(viewer)];
            let observed = Arc::make_mut(&mut coverage.observed);
            let explored = Arc::make_mut(&mut coverage.explored);
            let (mut before, mut after) = (0, 0);
            // Circle geometry enumerates cells in increasing row-major order.
            // Shared cells keep their count. Only the symmetric difference writes.
            while before < old.len() || after < new.len() {
                if before < old.len() && after < new.len() && old[before] == new[after] {
                    before += 1;
                    after += 1;
                    continue;
                }
                let add = before == old.len() || (after < new.len() && new[after] < old[before]);
                let cell = if add {
                    let cell = new[after];
                    after += 1;
                    cell
                } else {
                    let cell = old[before];
                    before += 1;
                    cell
                };
                let count = &mut coverage.counts[cell as usize];
                let flag = 1 << (cell % 8);
                let byte = cell as usize / 8;
                if add {
                    *count += 1;
                    observed[byte] |= flag;
                    explored[byte] |= flag;
                } else {
                    *count -= 1;
                    if *count == 0 {
                        observed[byte] &= !flag;
                    }
                }
            }
        }
    }
    fn update(
        &mut self,
        tick: u32,
        phase: ObservationPhase,
        mut units: Vec<VisibleUnit>,
        mut nodes: Vec<VisibleNode>,
        sources: Vec<(u32, u8, WorldPos, f32)>,
    ) {
        // Public lookups use binary search. Canonicalize once at the basis
        // boundary, including callers that enumerate ECS entities in another order.
        if !units.is_sorted_by_key(|row| row.unit) {
            units.sort_unstable_by_key(|row| row.unit);
        }
        if !nodes.is_sorted_by_key(|row| row.node) {
            nodes.sort_unstable_by_key(|row| row.node);
        }
        if self.event_tick != tick {
            self.event_tick = tick;
            self.events.clear();
            for removed in &mut self.observed_removals {
                removed.clear();
            }
        }
        let current: BTreeSet<_> = sources.iter().map(|source| source.0).collect();
        let removed = self
            .sources
            .keys()
            .filter(|id| !current.contains(id))
            .copied()
            .collect::<Vec<_>>();
        for id in removed {
            if let Some(source) = self.sources.remove(&id) {
                self.source_cache_bytes -= source.cached_bytes();
                self.adjust_source(&source, false);
            }
        }
        for (id, owner, position, radius) in sources {
            if self.sources.get(&id).is_some_and(|old| {
                old.owner == owner && old.position == position && old.radius == radius
            }) {
                continue;
            }
            if let Some(old) = self.sources.remove(&id) {
                // Both geometries coexist only within the aggregate cache budget.
                // If there is no room, release the old cache before rebuilding.
                if old.owner == owner
                    && let Some(old_cells) = &old.cells
                    && let Some(cells) = cached_source_cells(
                        rule_grid(self.grid),
                        position,
                        radius,
                        MAX_CACHED_SOURCE_CELL_BYTES - self.source_cache_bytes,
                    )
                {
                    self.adjust_cached_source_delta(owner, old_cells, &cells);
                    self.source_cache_bytes -= old.cached_bytes();
                    let source = Source {
                        owner,
                        position,
                        radius,
                        cells: Some(cells),
                    };
                    self.source_cache_bytes += source.cached_bytes();
                    self.sources.insert(id, source);
                    continue;
                }
                self.source_cache_bytes -= old.cached_bytes();
                self.adjust_source(&old, false);
            }
            let cells = cached_source_cells(
                rule_grid(self.grid),
                position,
                radius,
                MAX_CACHED_SOURCE_CELL_BYTES - self.source_cache_bytes,
            );
            let source = Source {
                owner,
                position,
                radius,
                cells,
            };
            self.source_cache_bytes += source.cached_bytes();
            self.adjust_source(&source, true);
            self.sources.insert(id, source);
        }
        let previous = self.phase(self.active);
        let unit_rows = if units.len() == previous.units.len()
            && units
                .iter()
                .zip(previous.units.iter())
                .all(|(a, b)| a.unit == b.unit)
        {
            previous.unit_rows.clone()
        } else {
            dense_unit_rows(&units)
        };
        let mut next = VisibilityBasis {
            stamp: KnowledgeBasis {
                match_id: self.match_id,
                map: self.map.clone(),
                tick,
                phase,
            },
            grid: self.grid,
            slots: Vec::new(),
            units: units.into(),
            unit_rows,
            nodes: nodes.into(),
        };
        // Coverage is ready before any memory decision. A hidden/absent subject's
        // current location never participates in clearing its old observed cell.
        for viewer in 0..SLOT_COUNT {
            let coverage = &self.coverage[usize::from(viewer)];
            let mut allowed_units = vec![0u8; next.units.len().div_ceil(8)];
            for (index, unit) in next.units.iter().enumerate() {
                if unit.owner == viewer
                    || rule_grid(self.grid)
                        .cell(point(unit.position))
                        .is_some_and(|cell| bit(&coverage.observed, cell))
                {
                    allowed_units[index / 8] |= 1 << (index % 8);
                }
            }
            next.slots.push(SlotBasis {
                allowed_units: allowed_units.into(),
                observed: Arc::clone(&coverage.observed),
                explored: Arc::clone(&coverage.explored),
                units: Arc::from([]),
                nodes: Arc::from([]),
                truncated: false,
            });
        }
        for viewer in 0..SLOT_COUNT {
            let memory = &mut self.memories[usize::from(viewer)];
            // Capture the last permitted phase only when observation ends. The
            // previous basis keeps same-tick movement/facing and its exact tick.
            // Merge the sorted rows to avoid a binary lookup per current subject.
            let mut current_units = next.units.iter().enumerate().peekable();
            for (old_index, unit) in previous.units.iter().enumerate() {
                while current_units
                    .peek()
                    .is_some_and(|(_, row)| row.unit < unit.unit)
                {
                    current_units.next();
                }
                let currently_permitted = current_units.peek().is_some_and(|(index, row)| {
                    row.unit == unit.unit
                        && bit(
                            &next.slots[usize::from(viewer)].allowed_units,
                            *index as u32,
                        )
                });
                if !currently_permitted
                    && bit(
                        &previous.slots[usize::from(viewer)].allowed_units,
                        old_index as u32,
                    )
                {
                    memory
                        .units
                        .insert(unit.unit, (unit.clone(), previous.stamp.tick));
                }
            }
            let mut current_nodes = next.nodes.iter().peekable();
            for node in previous.nodes.iter() {
                while current_nodes.peek().is_some_and(|row| row.node < node.node) {
                    current_nodes.next();
                }
                let currently_permitted = current_nodes.peek().is_some_and(|row| {
                    row.node == node.node && next.point_observed(viewer, row.position)
                });
                if !currently_permitted && previous.point_observed(viewer, node.position) {
                    memory
                        .nodes
                        .insert(node.node, (node.clone(), previous.stamp.tick));
                }
            }
            memory.units.retain(|id, (old, _)| {
                !self.observed_removals[usize::from(viewer)].contains(id)
                    && !next.unit_permitted(viewer, *id)
                    && !next.point_observed(viewer, old.position)
            });
            memory.nodes.retain(|id, (old, _)| {
                !next.node_permitted(viewer, *id) && !next.point_observed(viewer, old.position)
            });
            // Select the oldest timestamp group in one sort, never by subject ID.
            // Avoid building retention work when all stale records fit the limit.
            let limit = self.definition.effective_memory_limit() as usize;
            if memory.units.len() + memory.nodes.len() > limit {
                let mut stale = memory
                    .units
                    .iter()
                    .map(|(id, (_, tick))| (*tick, true, *id))
                    .chain(
                        memory
                            .nodes
                            .iter()
                            .map(|(id, (_, tick))| (*tick, false, *id)),
                    )
                    .collect::<Vec<_>>();
                stale.sort_unstable_by_key(|row| row.0);
                let cutoff = stale[stale.len() - limit - 1].0;
                memory.truncated = true;
                for (_, unit, id) in stale.iter().copied().take_while(|row| row.0 <= cutoff) {
                    if unit {
                        memory.units.remove(&id);
                    } else {
                        memory.nodes.remove(&id);
                    }
                }
            }
            let previous = &previous.slots[usize::from(viewer)];
            let same_units =
                memory.units.len() == previous.units.len()
                    && memory.units.values().zip(previous.units.iter()).all(
                        |((unit, tick), old)| {
                            unit.unit == old.unit
                                && unit.content.as_ref() == &old.content
                                && unit.owner == old.owner
                                && unit.position == old.position
                                && unit.facing == old.facing
                                && unit.site == old.site
                                && *tick == old.last_seen_tick
                        },
                    );
            let stale_units = if same_units {
                Arc::clone(&previous.units)
            } else {
                memory
                    .units
                    .values()
                    .map(|(unit, tick)| unit.remembered(*tick))
                    .collect::<Vec<_>>()
                    .into()
            };
            let same_nodes =
                memory.nodes.len() == previous.nodes.len()
                    && memory.nodes.values().zip(previous.nodes.iter()).all(
                        |((node, tick), old)| {
                            node.node == old.node
                                && node.content.as_ref() == &old.content
                                && node.position == old.position
                                && node.remaining == old.remaining
                                && *tick == old.last_seen_tick
                        },
                    );
            let stale_nodes = if same_nodes {
                Arc::clone(&previous.nodes)
            } else {
                memory
                    .nodes
                    .values()
                    .map(|(node, tick)| node.remembered(*tick))
                    .collect::<Vec<_>>()
                    .into()
            };
            next.slots[usize::from(viewer)].units = stale_units;
            next.slots[usize::from(viewer)].nodes = stale_nodes;
            next.slots[usize::from(viewer)].truncated = memory.truncated;
        }
        self.active = phase;
        let next = Arc::new(next);
        match phase {
            ObservationPhase::Start => self.start = next,
            ObservationPhase::Tactical => self.tactical = next,
            ObservationPhase::Completed => self.completed = next,
        }
    }
}

/// The sight rule of a match: the part that a host can state from the locked package and
/// the room (grid, sharing, teams). An auditor compares it with its own trusted copy.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SightPolicy {
    pub grid: VisibilityGrid,
    pub sharing: VisionSharing,
    pub teams: Vec<u8>,
}
/// One unit type that the sight basis lists. `radius` is `None` for a type with no vision.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SightTypeAudit {
    pub type_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
    pub has_health: bool,
}
/// Privileged audit copy of the constant inputs of sight. It holds no result of a sight
/// computation: no observed cell and no permitted or hidden unit. An auditor computes those.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SightAudit {
    pub policy: SightPolicy,
    /// False for the fail-closed placeholder: no slot may see a unit of another slot.
    pub finalized: bool,
    pub types: Vec<SightTypeAudit>,
}
/// `(content id, vision, has health)` for each unit type; `None` for a type that the basis
/// cannot list. The one source for `capture_units` and for the audit copy.
type SightTypeRow = Option<(Arc<ContentId>, Option<turts_mod_api::VisionDef>, bool)>;
fn sight_type_rows(world: &World) -> Vec<SightTypeRow> {
    let Some(types) = world.get_resource::<crate::UnitTypes>() else {
        return Vec::new();
    };
    let catalog = world.get_resource::<crate::bootstrap::InstalledCatalog>();
    types
        .entries()
        .iter()
        .map(|entry| {
            let id = ContentId::parse(&entry.content_id).ok()?;
            let definition = catalog.and_then(|catalog| catalog.0.units.get(&id));
            let vision = definition.and_then(|unit| unit.vision);
            let has_health = definition.is_some_and(|unit| unit.health.is_some());
            Some((Arc::new(id), vision, has_health))
        })
        .collect()
}
/// None means explicit AllVisible, as everywhere in this module.
pub fn audit(world: &World) -> Option<SightAudit> {
    let state = world.get_resource::<VisibilityState>()?;
    Some(SightAudit {
        policy: SightPolicy {
            grid: state.grid,
            sharing: state.definition.sharing,
            teams: state.teams.clone(),
        },
        finalized: state.context_finalized,
        types: sight_type_rows(world)
            .into_iter()
            .enumerate()
            .filter_map(|(type_id, row)| {
                let (_, vision, has_health) = row?;
                Some(SightTypeAudit {
                    type_id: type_id as u32,
                    radius: vision.map(|vision| vision.radius),
                    has_health,
                })
            })
            .collect(),
    })
}

/// None means only explicit AllVisible. Installed restricted phases always exist.
pub fn basis(world: &World, phase: ObservationPhase) -> Option<Arc<VisibilityBasis>> {
    world
        .get_resource::<VisibilityState>()
        .map(|state| state.phase(phase))
}
pub fn unit_permitted(world: &World, viewer: u8, unit: u32, phase: ObservationPhase) -> bool {
    basis(world, phase).is_none_or(|basis| basis.unit_permitted(viewer, unit))
}
pub fn node_permitted(world: &World, viewer: u8, node: u32, phase: ObservationPhase) -> bool {
    basis(world, phase).is_none_or(|basis| basis.node_permitted(viewer, node))
}

/// Current simulation action basis. None is explicit AllVisible only.
pub fn active_basis(world: &World) -> Option<&VisibilityBasis> {
    let state = world.get_resource::<VisibilityState>()?;
    Some(match state.active {
        ObservationPhase::Start => &state.start,
        ObservationPhase::Tactical => &state.tactical,
        ObservationPhase::Completed => &state.completed,
    })
}
/// Share the current immutable action basis once for a complete system pass.
/// None means explicit AllVisible; restricted placeholders remain fail closed.
pub fn active_basis_shared(world: &World) -> Option<Arc<VisibilityBasis>> {
    let state = world.get_resource::<VisibilityState>()?;
    Some(state.phase(state.active))
}
pub(crate) fn validate_target(
    world: &World,
    viewer: u8,
    target: &crate::orders::OrderTargetSim,
) -> Result<(), crate::RejectReason> {
    let Some(basis) = active_basis(world) else {
        return Ok(());
    };
    let permitted = match target {
        crate::orders::OrderTargetSim::Unit(id) => basis.unit_permitted(viewer, *id),
        crate::orders::OrderTargetSim::Node(id) => basis.node_permitted(viewer, id.0),
        _ => true,
    };
    if permitted {
        Ok(())
    } else {
        Err(crate::RejectReason::TargetInvalid)
    }
}

type SightSource = (u32, u8, WorldPos, f32);

fn capture_units(world: &mut World) -> (Vec<VisibleUnit>, Vec<SightSource>) {
    // A fresh typed query includes structural changes made between phase anchors.
    // Column access avoids repeated dynamic component lookup for every entity.
    let mut query = world.query::<(
        &crate::UnitId,
        &crate::UnitType,
        &crate::Owner,
        &crate::Position,
        Option<&crate::world::UnitFacing>,
    )>();
    if !world.contains_resource::<crate::UnitTypes>() {
        return (Vec::new(), Vec::new());
    }
    let definitions = sight_type_rows(world);
    let sites = world.get_resource::<crate::economy::construction::ConstructionState>();
    let healths = world.get_resource::<crate::economy::combat::Healths>();
    let mut units = Vec::new();
    let mut sources = Vec::new();
    for (id, kind, owner, position, facing) in query.iter(world) {
        let Some(Some((content, vision, has_health))) = definitions.get(kind.0.0 as usize) else {
            continue;
        };
        let site = sites.is_some_and(|sites| sites.sites.contains_key(&id.0));
        let position = WorldPos {
            x: position.0.x,
            y: position.0.y,
        };
        units.push(VisibleUnit {
            unit: id.0,
            content: Arc::clone(content),
            owner: owner.0.0,
            position,
            facing: facing.map(|facing| facing.0),
            site,
            ground_elevation: crate::terrain::ground_elevation(world, id.0),
        });
        if !site && let Some(vision) = vision {
            let alive = !has_health
                || healths
                    .and_then(|healths| healths.0.get(&id.0))
                    .is_some_and(|health| *health > 0);
            if alive {
                sources.push((id.0, owner.0.0, position, vision.radius));
            }
        }
    }
    units.sort_unstable_by_key(|unit| unit.unit);
    (units, sources)
}
fn refresh(world: &mut World, phase: ObservationPhase) {
    let tick = event_tick(world);
    if !world.contains_resource::<VisibilityState>()
        && let Some(mut context) = world.get_resource_mut::<AllVisibleTerrainEvents>()
    {
        context.advance(tick, phase);
    }
    if !world
        .get_resource::<VisibilityState>()
        .is_some_and(|state| state.context_finalized)
    {
        crate::terrain::phase::capture_phase(world, phase);
        return;
    }
    let (units, sources) = capture_units(world);
    let mut nodes = world
        .get_resource::<crate::SurfaceNodes>()
        .map(|nodes| {
            nodes
                .0
                .iter()
                .map(|node| VisibleNode {
                    node: node.id.0,
                    content: Arc::new(node.content_id.clone()),
                    position: WorldPos {
                        x: node.position.0.x,
                        y: node.position.0.y,
                    },
                    remaining: node.remaining,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    nodes.sort_unstable_by_key(|node| node.node);
    let tick = world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0);
    world
        .resource_mut::<VisibilityState>()
        .update(tick, phase, units, nodes, sources);
    crate::terrain::phase::capture_phase(world, phase);
}
/// Finalize the real match identity and installed teams before initial publication.
/// Match ID zero is reserved for explicitly initialized headless diagnostics.
/// A server must use its real room match ID. Later context changes are refused.
pub fn initialize_context(world: &mut World, match_id: u64, teams: &[u8]) -> Result<(), String> {
    if world
        .get_resource::<crate::Tick>()
        .is_some_and(|tick| tick.0 != 0)
    {
        return Err("sight context can only be finalized at tick zero".into());
    }
    if teams.len() > usize::from(SLOT_COUNT) {
        return Err("sight context contains too many slot teams".into());
    }
    let Some(state) = world.get_resource::<VisibilityState>() else {
        if world.contains_resource::<crate::terrain::TerrainState>() {
            let map = world
                .get_resource::<crate::bootstrap::InstalledMapIdentity>()
                .ok_or("terrain event context requires the installed map identity")?
                .0
                .clone();
            initialize_all_visible_terrain_event_context(world, match_id, map)?;
        }
        return Ok(());
    };
    if state.context_finalized {
        return if state.match_id == match_id && state.teams == teams {
            Ok(())
        } else {
            Err("sight match context is already finalized".into())
        };
    }
    let mut prepared = prepare(
        match_id,
        state.map.clone(),
        &state.definition,
        state.grid,
        teams,
    )?;
    prepared.context_finalized = true;
    world.insert_resource(prepared);
    initialize(world);
    Ok(())
}
/// AllVisible is ready without a restricted context. Restricted placeholders are
/// fail closed; the host must finalize them before publication or bot setup.
pub fn context_ready(world: &World) -> bool {
    world
        .get_resource::<VisibilityState>()
        .is_none_or(|state| state.context_finalized)
}

pub fn initialize(world: &mut World) {
    refresh_start(world);
    refresh_tactical(world);
    refresh_completed(world);
}
pub fn refresh_start(world: &mut World) {
    refresh(world, ObservationPhase::Start);
}
pub fn refresh_tactical(world: &mut World) {
    refresh(world, ObservationPhase::Tactical);
}
pub fn refresh_completed(world: &mut World) {
    refresh(world, ObservationPhase::Completed);
}

/// Prepare before a transaction removes/mutates an event subject. This has no side effects.
fn capture_event_unit(world: &World, unit: u32) -> Option<VisibleUnit> {
    crate::unit_entity(world, unit).and_then(|entity| {
        let kind = world.get::<crate::UnitType>(entity)?;
        let entry = world.get_resource::<crate::UnitTypes>()?.get(kind.0)?;
        let owner = world.get::<crate::Owner>(entity)?.0.0;
        let position = world.get::<crate::Position>(entity)?.0;
        Some(VisibleUnit {
            unit,
            ground_elevation: crate::terrain::ground_elevation(world, unit),
            content: Arc::new(ContentId::parse(&entry.content_id).ok()?),
            owner,
            position: WorldPos {
                x: position.x,
                y: position.y,
            },
            facing: world
                .get::<crate::world::UnitFacing>(entity)
                .map(|facing| facing.0),
            site: world
                .get_resource::<crate::economy::construction::ConstructionState>()
                .is_some_and(|state| state.sites.contains_key(&unit)),
        })
    })
}
pub fn prepare_event_unit(world: &World, unit: u32) -> Option<PreparedEventUnit> {
    let Some(state) = world.get_resource::<VisibilityState>() else {
        let context = world.get_resource::<AllVisibleTerrainEvents>()?;
        if !terrain_context_matches(world, context) {
            return None;
        }
        let row = capture_event_unit(world, unit)?;
        row.ground_elevation?;
        let tick = event_tick(world);
        return Some(PreparedEventUnit {
            fact: EventUnitFact {
                basis: KnowledgeBasis {
                    match_id: context.match_id,
                    map: context.map.clone(),
                    tick,
                    phase: if context.tick == tick {
                        context.phase
                    } else {
                        ObservationPhase::Start
                    },
                },
                unit: row,
            },
            viewers: Vec::new(),
            all_visible_terrain: true,
        });
    };
    let basis = state.phase(state.active);
    let row = capture_event_unit(world, unit)
        .or_else(|| basis.units.iter().find(|row| row.unit == unit).cloned())?;
    let viewers = (0..SLOT_COUNT)
        .filter(|viewer| *viewer == row.owner || basis.point_observed(*viewer, row.position))
        .collect();
    Some(PreparedEventUnit {
        fact: EventUnitFact {
            basis: basis.stamp.clone(),
            unit: row,
        },
        viewers,
        all_visible_terrain: false,
    })
}
/// Commit only after the corresponding real event/removal transaction succeeds.
pub fn commit_event_unit(world: &mut World, prepared: PreparedEventUnit, removed: bool) {
    if prepared.all_visible_terrain {
        if world.contains_resource::<VisibilityState>()
            || prepared.fact.basis.tick != event_tick(world)
            || !world
                .get_resource::<AllVisibleTerrainEvents>()
                .is_some_and(|context| {
                    terrain_context_matches(world, context)
                        && context.match_id == prepared.fact.basis.match_id
                        && context.map == prepared.fact.basis.map
                })
        {
            return;
        }
        if let Some(mut context) = world.get_resource_mut::<AllVisibleTerrainEvents>() {
            let phase = if context.tick == prepared.fact.basis.tick {
                context.phase
            } else {
                prepared.fact.basis.phase
            };
            context.advance(prepared.fact.basis.tick, phase);
            context.commit(prepared, ALL_VISIBLE_TERRAIN_EVENT_LIMIT);
        }
        return;
    }
    let Some(mut state) = world.get_resource_mut::<VisibilityState>() else {
        return;
    };
    if removed {
        for viewer in &prepared.viewers {
            state.observed_removals[usize::from(*viewer)].insert(prepared.fact.unit.unit);
            state.memories[usize::from(*viewer)]
                .units
                .remove(&prepared.fact.unit.unit);
        }
    }
    if let Some(existing) = state.events.iter_mut().find(|event| {
        event.fact.basis == prepared.fact.basis && event.fact.unit.unit == prepared.fact.unit.unit
    }) {
        *existing = prepared;
    } else {
        state.events.push(prepared);
    }
}
pub fn event_units(world: &World, viewer: u8) -> Vec<EventUnitFact> {
    if !world.contains_resource::<VisibilityState>() {
        return world
            .get_resource::<AllVisibleTerrainEvents>()
            .filter(|context| {
                viewer < SLOT_COUNT
                    && terrain_context_matches(world, context)
                    && context.complete
                    && context.tick == event_tick(world)
            })
            .map(|context| {
                context
                    .events
                    .iter()
                    .map(|event| event.fact.clone())
                    .collect()
            })
            .unwrap_or_default();
    }
    world
        .get_resource::<VisibilityState>()
        .map(|state| {
            state
                .events
                .iter()
                .filter(|event| event.viewers.contains(&viewer))
                .map(|event| event.fact.clone())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "visibility_tests.rs"]
mod tests;

#[cfg(test)]
mod source_cache_tests {
    use super::*;
    #[test]
    fn aggregate_large_circle_cache_is_bounded_and_streaming_preserves_exact_counts() {
        let definition = VisibilityDef {
            cell_size: 1.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        };
        let grid = VisibilityGrid {
            size: WorldPos {
                x: 1024.0,
                y: 1024.0,
            },
            cell_size: 1.0,
            columns: 1024,
            rows: 1024,
        };
        let mut state = prepare(
            1,
            ContentId::parse("fixture:large").unwrap(),
            &definition,
            grid,
            &[0, 1],
        )
        .unwrap();
        let source = |id| (id, 0, WorldPos { x: 512.0, y: 512.0 }, f32::MAX);
        state.update(
            1,
            ObservationPhase::Completed,
            vec![],
            vec![],
            vec![source(1), source(2), source(3)],
        );
        assert!(state.cached_source_cell_bytes() <= MAX_CACHED_SOURCE_CELL_BYTES);
        assert_eq!(
            state
                .sources
                .values()
                .filter(|source| source.cells.is_some())
                .count(),
            2
        );
        assert!(
            state.sources[&3].cells.is_none(),
            "the third million-cell circle must stream instead of allocating another4MiB"
        );
        assert!(state.coverage[0].counts.iter().all(|count| *count == 3));
        assert!(
            state
                .completed
                .knowledge(0)
                .unwrap()
                .observed
                .iter()
                .all(|byte| *byte == 255)
        );
        state.update(
            2,
            ObservationPhase::Completed,
            vec![],
            vec![],
            vec![source(1), source(2)],
        );
        assert!(
            state.coverage[0].counts.iter().all(|count| *count == 2),
            "removing an uncached source uses the same exact geometry"
        );
        state.update(
            3,
            ObservationPhase::Completed,
            vec![],
            vec![],
            vec![source(2), source(4)],
        );
        assert!(
            state.sources[&4].cells.is_some(),
            "released cache capacity can serve a later source"
        );
        assert!(state.cached_source_cell_bytes() <= MAX_CACHED_SOURCE_CELL_BYTES);
        assert!(state.coverage[0].counts.iter().all(|count| *count == 2));
        state.update(4, ObservationPhase::Completed, vec![], vec![], vec![]);
        assert_eq!(state.cached_source_cell_bytes(), 0);
        assert!(state.coverage[0].counts.iter().all(|count| *count == 0));
        assert!(
            state
                .completed
                .knowledge(0)
                .unwrap()
                .explored
                .iter()
                .all(|byte| *byte == 255)
        );
    }
    #[test]
    fn insufficient_cache_capacity_uses_no_partial_circle_and_keeps_one_shared_geometry() {
        let grid = Grid::checked(16.0, 16.0, 1.0).unwrap();
        let position = WorldPos { x: 8.0, y: 8.0 };
        assert!(cached_source_cells(grid, position, 6.0, 16).is_none());
        let cached = cached_source_cells(grid, position, 6.0, 4096).unwrap();
        let mut streamed = vec![];
        grid.circle(point(position), 6.0, |cell| streamed.push(cell));
        assert_eq!(cached, streamed);
    }
}

#[cfg(test)]
mod all_visible_terrain_event_tests {
    use super::*;
    use crate::{Owner, Position, SlotId, Tick, UnitId, UnitType, UnitTypes};
    use turts_mod_api::ResolvedTerrainDef;
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

    fn scene() -> (World, turts_mod_api::UnitTypeId) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
        let catalog = PackageArchive::parse(&pack_dir(&root).unwrap())
            .unwrap()
            .content()
            .unwrap();
        let types = UnitTypes::from_catalog(&catalog, &UnitTypes::default()).unwrap();
        let kind = types.find("base:command_center").unwrap();
        let terrain = crate::terrain::prepare(
            &catalog,
            &ResolvedTerrainDef {
                length_scale: 1,
                cell_width: 1,
                elevation_step: 1,
                columns: 8,
                rows: 8,
                chunk_width: 4,
                initial_elevation: -3,
                initial_heights: Vec::new(),
                floor: -3,
                ceiling: 3,
                generation: Vec::new(),
            },
            crate::terrain::PrivateTerrainSeed::new(123),
        )
        .unwrap();
        let mut world = World::new();
        world.insert_resource(Tick(0));
        world.insert_resource(types);
        world.insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        world.insert_resource(crate::bootstrap::InstalledMapIdentity(
            ContentId::parse("base:flat").unwrap(),
        ));
        world.insert_resource(terrain);
        (world, kind)
    }
    fn born(world: &mut World, kind: turts_mod_api::UnitTypeId) -> bevy_ecs::entity::Entity {
        let entity = world
            .spawn((
                UnitId(u32::MAX),
                Owner(SlotId(1)),
                UnitType(kind),
                Position(bevy_math::Vec2::new(4.0, 4.0)),
            ))
            .id();
        world.flush();
        entity
    }

    #[test]
    fn terrain_birth_death_witness_requires_commit_and_retains_real_contact() {
        let (mut world, kind) = scene();
        assert!(!event_units_complete(&world));
        initialize_context(&mut world, 47, &[0, 1]).unwrap();
        assert!(event_units_complete(&world));
        refresh_tactical(&mut world);
        let entity = born(&mut world, kind);
        let prepared = prepare_event_unit(&world, u32::MAX).unwrap();
        assert!(event_units(&world, 0).is_empty());
        assert_eq!(prepared.fact.unit.ground_elevation, Some(-3));
        world.despawn(entity);
        commit_event_unit(&mut world, prepared, true);
        for viewer in [0, 1] {
            let facts = event_units(&world, viewer);
            assert_eq!(facts.len(), 1);
            assert_eq!(facts[0].basis.match_id, 47);
            assert_eq!(facts[0].basis.phase, ObservationPhase::Tactical);
            assert_eq!(facts[0].unit.unit, u32::MAX);
            assert_eq!(facts[0].unit.ground_elevation, Some(-3));
        }
        assert!(event_units(&world, SLOT_COUNT).is_empty());
        initialize_context(&mut world, 47, &[0, 1]).unwrap();
        assert_eq!(event_units(&world, 0).len(), 1);
        assert!(initialize_context(&mut world, 48, &[0, 1]).is_err());
        world.resource_mut::<Tick>().0 = 1;
        assert!(event_units(&world, 0).is_empty());
        refresh_start(&mut world);
        assert!(event_units_complete(&world));
        assert!(
            world
                .resource::<AllVisibleTerrainEvents>()
                .events
                .is_empty()
        );
    }

    #[test]
    fn terrain_witness_deduplicates_only_same_phase_and_checks_context_reset() {
        let (mut world, kind) = scene();
        initialize_context(&mut world, 47, &[0, 1]).unwrap();
        let entity = born(&mut world, kind);
        let start = prepare_event_unit(&world, u32::MAX).unwrap();
        commit_event_unit(&mut world, start.clone(), false);
        commit_event_unit(&mut world, start, false);
        refresh_tactical(&mut world);
        world.get_mut::<Position>(entity).unwrap().0.x = 4.25;
        let tactical = prepare_event_unit(&world, u32::MAX).unwrap();
        commit_event_unit(&mut world, tactical.clone(), false);
        let facts = event_units(&world, 0);
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].unit.position.x, 4.0);
        assert_eq!(facts[1].unit.position.x, 4.25);
        world.remove_resource::<AllVisibleTerrainEvents>();
        assert!(!event_units_complete(&world));
        initialize_context(&mut world, 48, &[0, 1]).unwrap();
        commit_event_unit(&mut world, tactical, false);
        assert!(event_units(&world, 0).is_empty());
        world.resource_mut::<Tick>().0 = 1;
        assert!(initialize_context(&mut world, 48, &[0, 1]).is_err());
    }

    #[test]
    fn terrain_witness_cap_fails_the_whole_set_and_resets_next_tick() {
        let (mut world, kind) = scene();
        initialize_context(&mut world, 47, &[0, 1]).unwrap();
        born(&mut world, kind);
        let prepared = prepare_event_unit(&world, u32::MAX).unwrap();
        {
            let mut context = world.resource_mut::<AllVisibleTerrainEvents>();
            context.commit(prepared.clone(), 1);
            context.commit(prepared.clone(), 1);
            assert!(context.complete);
            let mut different = prepared;
            different.fact.unit.unit = 1;
            context.commit(different, 1);
        }
        assert!(!event_units_complete(&world));
        assert!(event_units(&world, 0).is_empty());
        world.resource_mut::<Tick>().0 = 1;
        refresh_start(&mut world);
        assert!(event_units_complete(&world));
        let new = prepare_event_unit(&world, u32::MAX).unwrap();
        commit_event_unit(&mut world, new, false);
        assert_eq!(event_units(&world, 0).len(), 1);
        world.remove_resource::<crate::terrain::TerrainState>();
        assert!(event_units_complete(&world));
        assert!(prepare_event_unit(&world, u32::MAX).is_none());
        assert!(event_units(&world, 0).is_empty());
    }
}
