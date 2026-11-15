//! Terrain routes constrain proposals before the existing collision solver commits them.
use super::TerrainState;
use crate::movement::{MovementFrame, MovementFrameOutput, MovementFrameUnit};
use bevy_ecs::{
    prelude::{Component, Resource},
    world::World,
};
use bevy_math::Vec2;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
};
use turts_mod_api::ContentId;
use turts_rules::terrain::{
    Ratio,
    geometry::{Budget, Cell, Point, Traversal},
};
const SEARCH_POPS_PER_TICK: usize = 256;
const MAX_ACTIVE_SEARCHES: usize = 16;
const MAX_SOLVER_RETRIES: usize = 8;
const GEOMETRY_BUDGET: usize = 16_384;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TerrainMotionWait;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavigationDiagnostics {
    pub searches: u64,
    pub cache_hits: u64,
    pub invalidations: u64,
    pub solver_retries: u64,
    pub budget_waits: u64,
}
#[derive(Clone, PartialEq, Eq)]
struct Key {
    kind: u32,
    radius: u32,
    goal_x: u32,
    goal_y: u32,
    profile: Traversal,
}
#[derive(Clone)]
struct Node {
    cost: u64,
    parent: Option<Cell>,
    closed: bool,
}
struct Search {
    start: Cell,
    goal: Cell,
    nodes: BTreeMap<Cell, Node>,
    frontier: BinaryHeap<Reverse<(u64, u64, Cell)>>,
    lease: BTreeMap<usize, u64>,
}
struct Route {
    cells: Vec<Cell>,
    cursor: usize,
    lease: BTreeMap<usize, u64>,
}
enum RouteState {
    Searching(Search),
    Ready(Route),
    Blocked(BTreeMap<usize, u64>),
}
struct Entry {
    key: Key,
    state: RouteState,
}
#[derive(Resource, Default)]
pub(crate) struct TerrainNavigation {
    entries: BTreeMap<u32, Entry>,
    diagnostics: NavigationDiagnostics,
}
pub fn privileged_diagnostics(world: &World) -> NavigationDiagnostics {
    world
        .get_resource::<TerrainNavigation>()
        .map_or_else(NavigationDiagnostics::default, |navigation| {
            navigation.diagnostics
        })
}
pub(crate) fn order_changed(world: &mut World, unit: u32) {
    if let Some(mut navigation) = world.get_resource_mut::<TerrainNavigation>() {
        navigation.entries.remove(&unit);
    }
}
fn point(point: Vec2) -> Point {
    Point {
        x: f64::from(point.x),
        y: f64::from(point.y),
    }
}
fn bits(value: f32) -> u32 {
    if value == 0.0 { 0 } else { value.to_bits() }
}
fn centre(terrain: &TerrainState, cell: Cell) -> Vec2 {
    let width = f64::from(terrain.grid.cell_width) / f64::from(terrain.grid.length_scale);
    Vec2::new(
        ((f64::from(cell.x) + 0.5) * width) as f32,
        ((f64::from(cell.y) + 0.5) * width) as f32,
    )
}
fn profile(world: &World, unit: &MovementFrameUnit) -> Option<Traversal> {
    let entry = world
        .get_resource::<crate::UnitTypes>()?
        .get(turts_mod_api::UnitTypeId(unit.unit_type))?;
    let id = ContentId::parse(&entry.content_id).ok()?;
    let catalog = &world
        .get_resource::<crate::bootstrap::InstalledCatalog>()?
        .0;
    let class = catalog
        .units
        .get(&id)?
        .terrain_class
        .as_ref()
        .and_then(|id| catalog.terrain_movement_classes.get(id));
    Some(class.map_or(
        Traversal {
            max_step: 0,
            max_grade: Ratio {
                numerator: 0,
                denominator: 1,
            },
        },
        |class| Traversal {
            max_step: class.max_step,
            max_grade: Ratio {
                numerator: u64::from(class.max_grade.numerator),
                denominator: u64::from(class.max_grade.denominator),
            },
        },
    ))
}
/// Certify the entire segment. A flat chunk rectangle is an exact sufficient certificate;
/// otherwise use the common analytic swept-disk predicate. End support is always checked.
fn clear(
    terrain: &TerrainState,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    lease: &mut BTreeMap<usize, u64>,
) -> bool {
    // Also lease rejected edge neighborhoods. A later local edit can open an old no-route
    // result; failing endpoint support must not make the blocking chunk invisible to caches.
    if let (Ok(first), Ok(last)) = (
        terrain.grid.point_to_cell(point(from)),
        terrain.grid.point_to_cell(point(to)),
    ) {
        let width = f64::from(terrain.grid.cell_width) / f64::from(terrain.grid.length_scale);
        let reach = (f64::from(radius) / width).ceil() as u32;
        let reach = reach.saturating_add(1);
        let minimum = Cell {
            x: first.x.min(last.x).saturating_sub(reach),
            y: first.y.min(last.y).saturating_sub(reach),
        };
        let maximum = Cell {
            x: first
                .x
                .max(last.x)
                .saturating_add(reach)
                .min(terrain.grid.columns - 1),
            y: first
                .y
                .max(last.y)
                .saturating_add(reach)
                .min(terrain.grid.rows - 1),
        };
        terrain.rectangle_certificate(minimum, maximum, lease);
    }
    let mut budget = Budget::new(GEOMETRY_BUDGET);
    let Ok(first) = terrain.grid.disk_support(
        terrain,
        point(from),
        f64::from(radius),
        profile.max_step,
        &mut budget,
    ) else {
        return false;
    };
    let Ok(last) = terrain.grid.disk_support(
        terrain,
        point(to),
        f64::from(radius),
        profile.max_step,
        &mut budget,
    ) else {
        return false;
    };
    let mut minimum = Cell {
        x: u32::MAX,
        y: u32::MAX,
    };
    let mut maximum = Cell { x: 0, y: 0 };
    for cell in first.cells.iter().chain(&last.cells) {
        minimum.x = minimum.x.min(cell.x);
        minimum.y = minimum.y.min(cell.y);
        maximum.x = maximum.x.max(cell.x);
        maximum.y = maximum.y.max(cell.y);
    }
    if terrain.rectangle_certificate(minimum, maximum, lease) {
        return true;
    }
    let sweep = terrain.grid.sweep(
        terrain,
        point(from),
        point(to),
        f64::from(radius),
        profile,
        &mut budget,
    );
    sweep.result.is_ok() && sweep.numerator == sweep.denominator
}
fn heuristic(from: Cell, to: Cell) -> u64 {
    let dx = u64::from(from.x.abs_diff(to.x));
    let dy = u64::from(from.y.abs_diff(to.y));
    dx.min(dy) * 14 + dx.abs_diff(dy) * 10
}
fn start_search(start: Cell, goal: Cell) -> Search {
    Search {
        start,
        goal,
        nodes: BTreeMap::from([(
            start,
            Node {
                cost: 0,
                parent: None,
                closed: false,
            },
        )]),
        frontier: BinaryHeap::from([Reverse((heuristic(start, goal), 0, start))]),
        lease: BTreeMap::new(),
    }
}
fn search_step(
    terrain: &TerrainState,
    search: &mut Search,
    radius: f32,
    profile: Traversal,
) -> Option<Option<Route>> {
    for _ in 0..SEARCH_POPS_PER_TICK {
        let Some(Reverse((_, cost, cell))) = search.frontier.pop() else {
            return Some(None);
        };
        let Some(node) = search.nodes.get_mut(&cell) else {
            continue;
        };
        if node.closed || node.cost != cost {
            continue;
        }
        node.closed = true;
        if cell == search.goal {
            let mut path = VecDeque::new();
            let mut cursor = Some(cell);
            while let Some(cell) = cursor {
                path.push_front(cell);
                cursor = search.nodes.get(&cell).and_then(|node| node.parent);
            }
            let cells = path.into_iter().collect::<Vec<_>>();
            // Lease only the route's actual clearance certificates, not discarded search branches.
            let mut lease = BTreeMap::new();
            for edge in cells.windows(2) {
                if !clear(
                    terrain,
                    centre(terrain, edge[0]),
                    centre(terrain, edge[1]),
                    radius,
                    profile,
                    &mut lease,
                ) {
                    return Some(None);
                }
            }
            return Some(Some(Route {
                cells,
                cursor: 0,
                lease,
            }));
        }
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (Ok(x), Ok(y)) = (
                    u32::try_from(i64::from(cell.x) + dx),
                    u32::try_from(i64::from(cell.y) + dy),
                ) else {
                    continue;
                };
                if x >= terrain.grid.columns || y >= terrain.grid.rows {
                    continue;
                }
                let next = Cell { x, y };
                let mut budget = Budget::new(GEOMETRY_BUDGET);
                // Diagonals cannot bypass either orthogonal intermediate or its grade/step edges.
                let segment_clear = clear(
                    terrain,
                    centre(terrain, cell),
                    centre(terrain, next),
                    radius,
                    profile,
                    &mut search.lease,
                );
                if !segment_clear
                    || terrain
                        .grid
                        .transition(terrain, cell, next, profile, &mut budget)
                        .is_err()
                {
                    continue;
                }
                let Some(next_cost) = cost.checked_add(if dx == 0 || dy == 0 { 10 } else { 14 })
                else {
                    continue;
                };
                if search
                    .nodes
                    .get(&next)
                    .is_some_and(|node| node.cost <= next_cost)
                {
                    continue;
                }
                search.nodes.insert(
                    next,
                    Node {
                        cost: next_cost,
                        parent: Some(cell),
                        closed: false,
                    },
                );
                let Some(estimate) = next_cost.checked_add(heuristic(next, search.goal)) else {
                    continue;
                };
                search.frontier.push(Reverse((estimate, next_cost, next)));
            }
        }
    }
    None
}
fn state_lease(state: &RouteState) -> &BTreeMap<usize, u64> {
    match state {
        RouteState::Searching(search) => &search.lease,
        RouteState::Ready(route) => &route.lease,
        RouteState::Blocked(lease) => lease,
    }
}
fn waypoint(
    terrain: &TerrainState,
    navigation: &mut TerrainNavigation,
    unit: &MovementFrameUnit,
    goal: Vec2,
    profile: Traversal,
) -> Option<Vec2> {
    let mut direct = BTreeMap::new();
    if clear(
        terrain,
        unit.position,
        goal,
        unit.radius,
        profile,
        &mut direct,
    ) {
        return Some(goal);
    }
    let key = Key {
        kind: unit.unit_type,
        radius: bits(unit.radius),
        goal_x: bits(goal.x),
        goal_y: bits(goal.y),
        profile,
    };
    if navigation
        .entries
        .get(&unit.unit_id)
        .is_some_and(|entry| entry.key != key || !terrain.lease_valid(state_lease(&entry.state)))
    {
        navigation.entries.remove(&unit.unit_id);
        navigation.diagnostics.invalidations =
            navigation.diagnostics.invalidations.saturating_add(1);
    }
    if !navigation.entries.contains_key(&unit.unit_id) {
        if navigation
            .entries
            .values()
            .filter(|entry| matches!(entry.state, RouteState::Searching(_)))
            .count()
            >= MAX_ACTIVE_SEARCHES
        {
            navigation.diagnostics.budget_waits =
                navigation.diagnostics.budget_waits.saturating_add(1);
            return None;
        }
        let (Ok(start), Ok(destination)) = (
            terrain.grid.point_to_cell(point(unit.position)),
            terrain.grid.point_to_cell(point(goal)),
        ) else {
            return None;
        };
        navigation.entries.insert(
            unit.unit_id,
            Entry {
                key,
                state: RouteState::Searching(start_search(start, destination)),
            },
        );
        navigation.diagnostics.searches = navigation.diagnostics.searches.saturating_add(1);
    } else {
        navigation.diagnostics.cache_hits = navigation.diagnostics.cache_hits.saturating_add(1);
    }
    let entry = navigation.entries.get_mut(&unit.unit_id)?;
    if let RouteState::Searching(search) = &mut entry.state {
        if terrain.grid.point_to_cell(point(unit.position)).ok() != Some(search.start) {
            navigation.entries.remove(&unit.unit_id);
            return None;
        }
        if let Some(result) = search_step(terrain, search, unit.radius, profile) {
            entry.state = result.map_or_else(
                || RouteState::Blocked(search.lease.clone()),
                RouteState::Ready,
            );
        }
    }
    let RouteState::Ready(route) = &mut entry.state else {
        return None;
    };
    let width = terrain.grid.cell_width as f32 / terrain.grid.length_scale as f32;
    while route.cursor < route.cells.len() {
        let point = centre(terrain, route.cells[route.cursor]);
        if unit.position.distance(point) <= width * 0.25 {
            route.cursor += 1;
            continue;
        }
        if clear(
            terrain,
            unit.position,
            point,
            unit.radius,
            profile,
            &mut route.lease,
        ) {
            return Some(point);
        }
        navigation.entries.remove(&unit.unit_id);
        return None;
    }
    clear(
        terrain,
        unit.position,
        goal,
        unit.radius,
        profile,
        &mut route.lease,
    )
    .then_some(goal)
}
/// Reuse the existing frame solver. Unsafe proposals become stopped physical obstacles before
/// a retry, so a following actor cannot move into a position later rolled back underneath it.
pub(crate) fn step(
    world: &mut World,
    frame: &MovementFrame,
    dt: f32,
    preserve: &BTreeSet<u32>,
) -> (Vec<MovementFrameOutput>, BTreeSet<u32>) {
    if world.get_resource::<TerrainState>().is_none() {
        return (
            crate::movement::step_frame_preserving_resting(frame, dt, preserve),
            BTreeSet::new(),
        );
    }
    let mut navigation = world
        .remove_resource::<TerrainNavigation>()
        .unwrap_or_default();
    let (outputs, waiting) = {
        let terrain = world.resource::<TerrainState>();
        let profiles = frame
            .units
            .iter()
            .map(|unit| profile(world, unit))
            .collect::<Vec<_>>();
        let mut candidate = frame.clone();
        let mut waiting = BTreeSet::new();
        let live = frame
            .units
            .iter()
            .map(|unit| unit.unit_id)
            .collect::<BTreeSet<_>>();
        navigation.entries.retain(|unit, _| live.contains(unit));
        let mut order = (0..frame.units.len()).collect::<Vec<_>>();
        order.sort_by(|&left, &right| {
            let left = &frame.units[left];
            let right = &frame.units[right];
            let content = |unit: &MovementFrameUnit| {
                world
                    .resource::<crate::UnitTypes>()
                    .get(turts_mod_api::UnitTypeId(unit.unit_type))
                    .map(|entry| entry.content_id.as_str())
            };
            f32::from_bits(bits(left.position.x))
                .total_cmp(&f32::from_bits(bits(right.position.x)))
                .then_with(|| {
                    f32::from_bits(bits(left.position.y))
                        .total_cmp(&f32::from_bits(bits(right.position.y)))
                })
                .then_with(|| {
                    left.target
                        .map(|goal| (bits(goal.x), bits(goal.y)))
                        .cmp(&right.target.map(|goal| (bits(goal.x), bits(goal.y))))
                })
                .then_with(|| content(left).cmp(&content(right)))
        });
        for index in order {
            let unit = &frame.units[index];
            let Some(goal) = unit.target else {
                continue;
            };
            if unit.speed.is_none() {
                continue;
            }
            let waypoint = profiles[index]
                .and_then(|profile| waypoint(terrain, &mut navigation, unit, goal, profile));
            if let Some(point) = waypoint {
                if point != goal {
                    candidate.units[index].target = Some(point);
                    candidate.units[index].progress = None;
                    candidate.units[index].plain_progress = None;
                }
            } else {
                candidate.units[index].speed = None;
                candidate.units[index].target = None;
                waiting.insert(unit.unit_id);
            }
        }
        let mut accepted = None;
        for _ in 0..MAX_SOLVER_RETRIES {
            let proposed = crate::movement::step_frame_preserving_resting(&candidate, dt, preserve);
            let mut unsafe_indices = Vec::new();
            for (index, (unit, output)) in frame.units.iter().zip(&proposed).enumerate() {
                if output.position == unit.position {
                    continue;
                }
                let mut lease = BTreeMap::new();
                if profiles[index].is_none_or(|profile| {
                    !clear(
                        terrain,
                        unit.position,
                        output.position,
                        unit.radius,
                        profile,
                        &mut lease,
                    )
                }) {
                    unsafe_indices.push(index);
                }
            }
            if unsafe_indices.is_empty() {
                accepted = Some(proposed);
                break;
            }
            navigation.diagnostics.solver_retries =
                navigation.diagnostics.solver_retries.saturating_add(1);
            for index in unsafe_indices {
                candidate.units[index].speed = None;
                candidate.units[index].target = None;
                candidate.units[index].progress = None;
                candidate.units[index].plain_progress = None;
                waiting.insert(frame.units[index].unit_id);
            }
        }
        let mut outputs = accepted.unwrap_or_else(|| {
            // Finite safe fallback. No pose has been committed and the original bodies still
            // occupy these positions. Preserve every command for later route revalidation.
            frame
                .units
                .iter()
                .map(|unit| {
                    if unit.target.is_some() {
                        waiting.insert(unit.unit_id);
                    }
                    MovementFrameOutput {
                        unit_id: unit.unit_id,
                        position: unit.position,
                        target: unit.target,
                        progress: unit.progress.clone(),
                        plain_progress: unit.plain_progress.clone(),
                        physical_escape: unit.physical_escape.clone(),
                        failed_blocked: false,
                        #[cfg(feature = "test-probes")]
                        probe: Default::default(),
                    }
                })
                .collect()
        });
        for ((original, constrained), output) in
            frame.units.iter().zip(&candidate.units).zip(&mut outputs)
        {
            if original.target != constrained.target || original.speed != constrained.speed {
                output.target = if original.target == Some(output.position) {
                    None
                } else {
                    original.target
                };
                output.progress = original.progress.clone();
                output.plain_progress = original.plain_progress.clone();
            }
        }
        (outputs, waiting)
    };
    world.insert_resource(navigation);
    (outputs, waiting)
}
