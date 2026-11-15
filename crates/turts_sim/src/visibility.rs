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
    nodes: Arc<[VisibleNode]>,
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
    pub fn point_observed(&self, viewer: u8, position: WorldPos) -> bool {
        self.slots.get(usize::from(viewer)).is_some_and(|slot| {
            rule_grid(self.grid)
                .cell(point(position))
                .is_some_and(|cell| bit(&slot.observed, cell))
        })
    }
    pub fn area_observed(&self, viewer: u8, min: WorldPos, max: WorldPos) -> bool {
        self.slots.get(usize::from(viewer)).is_some_and(|slot| {
            rule_grid(self.grid).rectangle(point(min), point(max), |cell| bit(&slot.observed, cell))
        })
    }
    pub fn permitted_unit(&self, viewer: u8, id: u32) -> Option<&VisibleUnit> {
        self.slots.get(usize::from(viewer))?;
        let row = &self.units[self
            .units
            .binary_search_by_key(&id, |unit| unit.unit)
            .ok()?];
        (row.owner == viewer || self.point_observed(viewer, row.position)).then_some(row)
    }
    pub fn unit_permitted(&self, viewer: u8, id: u32) -> bool {
        self.permitted_unit(viewer, id).is_some()
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
    cells: Vec<u32>,
}
#[derive(Debug, Clone, Default)]
struct SlotMemory {
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
            for cell in &source.cells {
                let count = &mut coverage.counts[*cell as usize];
                // A live source has one allocator-unique unit identity. Coverage
                // counts cannot exceed the number of representable unit IDs.
                if add {
                    *count += 1;
                } else {
                    *count -= 1;
                }
                let observed = Arc::make_mut(&mut coverage.observed);
                let flag = 1 << (*cell % 8);
                let byte = *cell as usize / 8;
                if *count > 0 {
                    observed[byte] |= flag;
                    Arc::make_mut(&mut coverage.explored)[byte] |= flag;
                } else {
                    observed[byte] &= !flag;
                }
            }
        }
    }
    fn update(
        &mut self,
        tick: u32,
        phase: ObservationPhase,
        units: Vec<VisibleUnit>,
        nodes: Vec<VisibleNode>,
        sources: Vec<(u32, u8, WorldPos, f32)>,
    ) {
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
                self.adjust_source(&old, false);
            }
            let mut cells = Vec::new();
            rule_grid(self.grid).circle(point(position), radius, |cell| cells.push(cell));
            let source = Source {
                owner,
                position,
                radius,
                cells,
            };
            self.adjust_source(&source, true);
            self.sources.insert(id, source);
        }
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
            nodes: nodes.into(),
        };
        // Coverage is ready before any memory decision. A hidden/absent subject's
        // current location never participates in clearing its old observed cell.
        for viewer in 0..SLOT_COUNT {
            let coverage = &self.coverage[usize::from(viewer)];
            next.slots.push(SlotBasis {
                observed: Arc::clone(&coverage.observed),
                explored: Arc::clone(&coverage.explored),
                units: Arc::from([]),
                nodes: Arc::from([]),
                truncated: false,
            });
        }
        let previous = self.phase(self.active);
        for viewer in 0..SLOT_COUNT {
            let memory = &mut self.memories[usize::from(viewer)];
            for unit in next
                .units
                .iter()
                .filter(|unit| next.unit_permitted(viewer, unit.unit))
            {
                memory.units.insert(unit.unit, (unit.clone(), tick));
            }
            for node in next
                .nodes
                .iter()
                .filter(|node| next.node_permitted(viewer, node.node))
            {
                memory.nodes.insert(node.node, (node.clone(), tick));
            }
            memory.units.retain(|id, (old, _)| {
                !self.observed_removals[usize::from(viewer)].contains(id)
                    && (next.unit_permitted(viewer, *id)
                        || !next.point_observed(viewer, old.position))
            });
            memory.nodes.retain(|id, (old, _)| {
                next.node_permitted(viewer, *id) || !next.point_observed(viewer, old.position)
            });
            // Keep current observation storage separate from bounded stale memory.
            // Select the oldest timestamp group in one sort, never by subject ID.
            let mut stale = memory
                .units
                .iter()
                .filter(|(id, _)| !next.unit_permitted(viewer, **id))
                .map(|(id, (_, tick))| (*tick, true, *id))
                .chain(
                    memory
                        .nodes
                        .iter()
                        .filter(|(id, _)| !next.node_permitted(viewer, **id))
                        .map(|(id, (_, tick))| (*tick, false, *id)),
                )
                .collect::<Vec<_>>();
            let limit = self.definition.effective_memory_limit() as usize;
            if stale.len() > limit {
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
            let units = || {
                memory
                    .units
                    .values()
                    .filter(|(unit, _)| !next.unit_permitted(viewer, unit.unit))
            };
            let same_units = units().count() == previous.units.len()
                && units()
                    .zip(previous.units.iter())
                    .all(|((unit, tick), old)| {
                        unit.unit == old.unit
                            && unit.content.as_ref() == &old.content
                            && unit.owner == old.owner
                            && unit.position == old.position
                            && unit.facing == old.facing
                            && unit.site == old.site
                            && *tick == old.last_seen_tick
                    });
            let stale_units = if same_units {
                Arc::clone(&previous.units)
            } else {
                units()
                    .map(|(unit, tick)| unit.remembered(*tick))
                    .collect::<Vec<_>>()
                    .into()
            };
            let nodes = || {
                memory
                    .nodes
                    .values()
                    .filter(|(node, _)| !next.node_permitted(viewer, node.node))
            };
            let same_nodes = nodes().count() == previous.nodes.len()
                && nodes()
                    .zip(previous.nodes.iter())
                    .all(|((node, tick), old)| {
                        node.node == old.node
                            && node.content.as_ref() == &old.content
                            && node.position == old.position
                            && node.remaining == old.remaining
                            && *tick == old.last_seen_tick
                    });
            let stale_nodes = if same_nodes {
                Arc::clone(&previous.nodes)
            } else {
                nodes()
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

fn capture_units(world: &World) -> (Vec<VisibleUnit>, Vec<(u32, u8, WorldPos, f32)>) {
    let Some(types) = world.get_resource::<crate::UnitTypes>() else {
        return (Vec::new(), Vec::new());
    };
    let catalog = world.get_resource::<crate::bootstrap::InstalledCatalog>();
    let definitions = types
        .entries()
        .iter()
        .map(|entry| {
            let id = ContentId::parse(&entry.content_id).ok()?;
            let definition = catalog.and_then(|catalog| catalog.0.units.get(&id));
            let vision = definition.and_then(|unit| unit.vision);
            let has_health = definition.is_some_and(|unit| unit.health.is_some());
            Some((Arc::new(id), vision, has_health))
        })
        .collect::<Vec<_>>();
    let sites = world.get_resource::<crate::economy::construction::ConstructionState>();
    let healths = world.get_resource::<crate::economy::combat::Healths>();
    let mut units = Vec::new();
    let mut sources = Vec::new();
    for entity in world.iter_entities() {
        let (Some(id), Some(kind), Some(owner), Some(position)) = (
            entity.get::<crate::UnitId>(),
            entity.get::<crate::UnitType>(),
            entity.get::<crate::Owner>(),
            entity.get::<crate::Position>(),
        ) else {
            continue;
        };
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
            facing: entity
                .get::<crate::world::UnitFacing>()
                .map(|facing| facing.0),
            site,
        });
        let alive = !has_health
            || healths
                .and_then(|healths| healths.0.get(&id.0))
                .is_some_and(|health| *health > 0);
        if !site
            && alive
            && let Some(vision) = vision
        {
            sources.push((id.0, owner.0.0, position, vision.radius));
        }
    }
    units.sort_unstable_by_key(|unit| unit.unit);
    (units, sources)
}
fn refresh(world: &mut World, phase: ObservationPhase) {
    if !world
        .get_resource::<VisibilityState>()
        .is_some_and(|state| state.context_finalized)
    {
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
pub fn prepare_event_unit(world: &World, unit: u32) -> Option<PreparedEventUnit> {
    let state = world.get_resource::<VisibilityState>()?;
    let basis = state.phase(state.active);
    let row = crate::unit_entity(world, unit)
        .and_then(|entity| {
            let kind = world.get::<crate::UnitType>(entity)?;
            let entry = world.get_resource::<crate::UnitTypes>()?.get(kind.0)?;
            let owner = world.get::<crate::Owner>(entity)?.0.0;
            let position = world.get::<crate::Position>(entity)?.0;
            Some(VisibleUnit {
                unit,
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
    })
}
/// Commit only after the corresponding real event/removal transaction succeeds.
pub fn commit_event_unit(world: &mut World, prepared: PreparedEventUnit, removed: bool) {
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
