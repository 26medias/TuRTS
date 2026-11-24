//! Flock movement on editable terrain (T-7; design note T-6, stages A to D).
//!
//! Each tick reads one frozen snapshot of the frame. All bodies are updated together from
//! it, and every sum runs in a geometric order (distance, then position), so the order of
//! the rows and the unit ids cannot change a result.
//!
//! A. Guidance: one cost-to-goal field for each (owner, body class, goal), built from the
//!    owner's permitted terrain only, with a fixed work credit in cells for each tick.
//!    Guidance is advice. A unit with no ready field cell steers on the straight line.
//! B. Steering: goal direction, separation, alignment and weak cohesion over at most
//!    `NEIGHBOURS` neighbours.
//! C. Physical resolution: `flock_step::guarded_step` is the only authority against
//!    terrain and solid bodies. Own bodies then relax overlaps in `RELAX_PASSES` passes.
//! D. Arrival and waiting.
use super::{
    flock_step::{CLEARANCE, Solid, escape_step, guarded_step},
    geometry::TerrainRead,
};
use crate::{
    movement::{MovementActivity, MovementFrame, MovementFrameOutput, MovementFrameUnit},
    terrain::{TerrainPhase, TerrainState},
};
use bevy_ecs::{prelude::Resource, world::World};
use bevy_math::Vec2;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
};
use turts_rules::terrain::{
    Ratio,
    geometry::{Cell, Grid, Surface, Traversal},
};

/// The traversal of a unit with no terrain class: level ground only.
const LEVEL_GROUND: Traversal = Traversal {
    max_step: 0,
    max_grade: Ratio {
        numerator: 0,
        denominator: 1,
    },
};
/// Flat ground of a map with no editable terrain: one level inside the bounds.
struct FlatGround {
    grid: Grid,
}
impl FlatGround {
    fn new(bounds: Vec2) -> Self {
        Self {
            grid: Grid {
                columns: bounds.x.max(1.0).ceil() as u32,
                rows: bounds.y.max(1.0).ceil() as u32,
                length_scale: 1000,
                cell_width: 1000,
                elevation_step: 250,
                floor: -8,
                ceiling: 8,
            },
        }
    }
}
impl Surface for FlatGround {
    fn elevation(&self, cell: Cell) -> Option<i32> {
        (cell.x < self.grid.columns && cell.y < self.grid.rows).then_some(0)
    }
}
impl TerrainRead for FlatGround {
    fn grid(&self) -> Grid {
        self.grid
    }
    fn rectangle(&self, _: Cell, max: Cell, _: &mut BTreeMap<usize, u64>) -> bool {
        max.x < self.grid.columns && max.y < self.grid.rows
    }
}
/// Neighbours that one unit reads for steering.
const NEIGHBOURS: usize = 8;
/// Passes of simultaneous overlap corrections between own bodies.
const RELAX_PASSES: usize = 8;
/// Field work credit of one owner for one tick, in cells.
pub const FIELD_CREDIT: usize = 16_384;
/// Fields that one owner keeps.
const MAX_FIELDS: usize = 16;
/// Cost of one cell step; a diagonal step costs `DIAGONAL`.
const ORTHOGONAL: u32 = 1000;
const DIAGONAL: u32 = 1414;
const UNREACHED: u32 = u32::MAX;
/// The deepest overlap that a step may make between two own bodies that can move, as a
/// share of the smaller radius (rule 4; FM3 limits it to 25 %). Two units that step at each
/// other in one tick each use the cap against the start pose of the other, so the overlap
/// of a pair stays below two caps.
const OVERLAP_CAP: f32 = 0.10;
/// The gap that the relaxation leaves between two bodies.
const REST_GAP: f32 = 1.0e-3;
/// A gap up to this value between two bodies is contact.
const TOUCH: f32 = 0.05;
/// An overlap up to this value is none (float residue of the relaxation).
const NO_OVERLAP: f32 = 1.0e-3;
/// Ticks with no progress before a unit with no path becomes a waiting unit.
const WAIT_TICKS: u16 = 10;
/// Ticks with no approach before a unit inside its goal area settles where it stands.
const SETTLE_TICKS: u16 = 20;

type ClassKey = (u32, u32, u64, u64);
type GoalKey = (u32, u32);
type FieldKey = (u8, ClassKey, GoalKey);

fn class_key(radius: f32, profile: Traversal) -> ClassKey {
    (
        radius.to_bits(),
        profile.max_step,
        profile.max_grade.numerator,
        profile.max_grade.denominator,
    )
}
fn goal_key(goal: Vec2) -> GoalKey {
    (goal.x.to_bits(), goal.y.to_bits())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Moving,
    /// No path for this body class. The unit stands at the obstacle and can be pushed.
    Waiting,
    /// The order ended here. The unit is idle; the entry keeps its crowd membership.
    Settled,
}
#[derive(Clone, Debug)]
struct Member {
    owner: u8,
    goal: Vec2,
    mode: Mode,
    velocity: Vec2,
    best_distance: f32,
    stalled: u16,
    /// The pose at which the unit settled.
    rest: Vec2,
    /// The unit is in the crowd of a Move order (rules 2 and 3).
    crowd: bool,
}

/// One cost-to-goal field on the terrain cells, from one owner's permitted terrain.
struct Field {
    key: FieldKey,
    columns: u32,
    rows: u32,
    cell: f32,
    radius: f32,
    /// The largest radius of the owner's units that can move; it sets the passage width.
    widest: f32,
    profile: Traversal,
    goal: Vec2,
    /// Chunk revisions of the permitted view that the field read.
    lease: BTreeMap<usize, u64>,
    revision: u64,
    /// Rows of cells whose clearance is known.
    scanned_rows: u32,
    passable: Vec<bool>,
    passage: Vec<bool>,
    heights: Vec<Option<i32>>,
    cost: Vec<u32>,
    frontier: BinaryHeap<Reverse<(u32, u32)>>,
    seeded: bool,
    ready_cells: usize,
    /// Centres of the passage cells that the field reaches, known when it is complete.
    chokes: Option<Vec<Vec2>>,
    last_used: u32,
}
impl Field {
    fn new(
        key: FieldKey,
        phase: &TerrainPhase,
        radius: f32,
        widest: f32,
        profile: Traversal,
        goal: Vec2,
    ) -> Self {
        let grid = phase.grid();
        let cells = (grid.columns * grid.rows) as usize;
        Self {
            key,
            columns: grid.columns,
            rows: grid.rows,
            cell: grid.cell_width as f32 / grid.length_scale as f32,
            radius,
            widest,
            profile,
            goal,
            lease: BTreeMap::new(),
            revision: 0,
            scanned_rows: 0,
            passable: vec![false; cells],
            passage: vec![false; cells],
            heights: vec![None; cells],
            cost: vec![UNREACHED; cells],
            frontier: BinaryHeap::new(),
            seeded: false,
            ready_cells: 0,
            chokes: None,
            last_used: 0,
        }
    }
    fn restart(&mut self) {
        self.revision += 1;
        self.scanned_rows = 0;
        self.passable.fill(false);
        self.passage.fill(false);
        self.heights.fill(None);
        self.cost.fill(UNREACHED);
        self.frontier.clear();
        self.seeded = false;
        self.ready_cells = 0;
        self.chokes = None;
        self.lease.clear();
    }
    fn complete(&self) -> bool {
        self.seeded && self.frontier.is_empty()
    }
    fn index(&self, x: u32, y: u32) -> usize {
        (y * self.columns + x) as usize
    }
    fn centre(&self, x: u32, y: u32) -> Vec2 {
        Vec2::new((x as f32 + 0.5) * self.cell, (y as f32 + 0.5) * self.cell)
    }
    fn cell_of(&self, point: Vec2) -> Option<(u32, u32)> {
        let (x, y) = ((point.x / self.cell).floor(), (point.y / self.cell).floor());
        (x >= 0.0 && y >= 0.0 && (x as u32) < self.columns && (y as u32) < self.rows)
            .then_some((x as u32, y as u32))
    }
    fn steps(&self, from: usize, to: usize) -> bool {
        match (self.heights[from], self.heights[to]) {
            (Some(a), Some(b)) => a.abs_diff(b) <= self.profile.max_step,
            _ => false,
        }
    }
    /// Spend `credit` cells of work. Returns the work that was used.
    fn build(&mut self, phase: &TerrainPhase, viewer: u8, credit: usize) -> usize {
        let mut used = 0;
        if self.scanned_rows == 0 && self.lease.is_empty() {
            let max = Cell {
                x: self.columns - 1,
                y: self.rows - 1,
            };
            phase.planning_rectangle(viewer, Cell { x: 0, y: 0 }, max, &mut self.lease);
            for y in 0..self.rows {
                for x in 0..self.columns {
                    let index = self.index(x, y);
                    self.heights[index] = phase.planning_elevation(viewer, Cell { x, y });
                }
            }
        }
        // Clearance: the distance from the cell centre to the nearest cell that the class
        // cannot step to. The window covers two diameters, for the passage rule.
        let reach = (4.0 * self.widest / self.cell).ceil() as i64 + 1;
        while self.scanned_rows < self.rows && used + self.columns as usize <= credit {
            let y = self.scanned_rows;
            for x in 0..self.columns {
                let index = self.index(x, y);
                let centre = self.centre(x, y);
                let mut clearance = f32::MAX;
                if self.heights[index].is_none() {
                    clearance = 0.0;
                } else {
                    for ny in i64::from(y) - reach..=i64::from(y) + reach {
                        for nx in i64::from(x) - reach..=i64::from(x) + reach {
                            let inside = nx >= 0
                                && ny >= 0
                                && nx < i64::from(self.columns)
                                && ny < i64::from(self.rows);
                            if inside && self.steps(index, self.index(nx as u32, ny as u32)) {
                                continue;
                            }
                            let low = Vec2::new(nx as f32 * self.cell, ny as f32 * self.cell);
                            let nearest = centre.clamp(low, low + Vec2::splat(self.cell));
                            clearance = clearance.min(centre.distance(nearest));
                        }
                    }
                }
                self.passable[index] = clearance >= self.radius;
                // A corridor narrower than two diameters is a passage (T-6 history section).
                // The corridor width is the shorter of the two free runs through the cell,
                // along x and along y, so one wall beside open ground is not a passage.
                let run = |dx: i64, dy: i64| -> i64 {
                    let mut free = 0;
                    for step in 1..=reach {
                        let (nx, ny) = (i64::from(x) + dx * step, i64::from(y) + dy * step);
                        let inside = nx >= 0
                            && ny >= 0
                            && nx < i64::from(self.columns)
                            && ny < i64::from(self.rows);
                        if !inside || !self.steps(index, self.index(nx as u32, ny as u32)) {
                            break;
                        }
                        free += 1;
                    }
                    free
                };
                let width = (1 + run(1, 0) + run(-1, 0)).min(1 + run(0, 1) + run(0, -1));
                self.passage[index] = (width as f32) * self.cell < 4.0 * self.widest;
            }
            self.scanned_rows += 1;
            used += self.columns as usize;
        }
        if self.scanned_rows < self.rows {
            return used;
        }
        if !self.seeded {
            self.seed();
            self.seeded = true;
        }
        while used < credit {
            let Some(Reverse((cost, index))) = self.frontier.pop() else {
                break;
            };
            let index = index as usize;
            if cost != self.cost[index] {
                continue;
            }
            used += 1;
            self.ready_cells += 1;
            let (x, y) = ((index as u32) % self.columns, (index as u32) / self.columns);
            for (dx, dy) in NEIGHBOUR_CELLS {
                let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                if nx < 0 || ny < 0 || nx >= i64::from(self.columns) || ny >= i64::from(self.rows) {
                    continue;
                }
                let next = self.index(nx as u32, ny as u32);
                if !self.edge(index, next, dx, dy) {
                    continue;
                }
                let step = if dx != 0 && dy != 0 {
                    DIAGONAL
                } else {
                    ORTHOGONAL
                };
                let total = cost.saturating_add(step);
                if total < self.cost[next] {
                    self.cost[next] = total;
                    self.frontier.push(Reverse((total, next as u32)));
                }
            }
        }
        if self.complete() && self.chokes.is_none() {
            let mut chokes = Vec::new();
            for y in 0..self.rows {
                for x in 0..self.columns {
                    let index = self.index(x, y);
                    if self.passage[index] && self.cost[index] != UNREACHED {
                        chokes.push(self.centre(x, y));
                    }
                }
            }
            self.chokes = Some(chokes);
        }
        used
    }
    /// A body of the class can go between two adjacent passable cells.
    fn edge(&self, from: usize, to: usize, dx: i64, dy: i64) -> bool {
        if !self.passable[from] || !self.passable[to] || !self.steps(from, to) {
            return false;
        }
        if dx != 0 && dy != 0 {
            let (x, y) = ((from as u32) % self.columns, (from as u32) / self.columns);
            let side_a = self.index((i64::from(x) + dx) as u32, y);
            let side_b = self.index(x, (i64::from(y) + dy) as u32);
            return self.passable[side_a] && self.passable[side_b];
        }
        true
    }
    /// Goal cells: the passable cells within one radius (at least one cell) of the goal.
    /// With none, the passable cell nearest the goal (a tie goes to the lower cell index,
    /// which is a position, not an identity).
    fn seed(&mut self) {
        let reach = self.radius.max(self.cell);
        let mut nearest: Option<(f32, usize)> = None;
        for y in 0..self.rows {
            for x in 0..self.columns {
                let index = self.index(x, y);
                if !self.passable[index] {
                    continue;
                }
                let distance = self.centre(x, y).distance(self.goal);
                if distance <= reach {
                    let cost = (distance / self.cell * ORTHOGONAL as f32) as u32;
                    self.cost[index] = cost;
                    self.frontier.push(Reverse((cost, index as u32)));
                }
                if nearest.is_none_or(|(best, _)| distance < best) {
                    nearest = Some((distance, index));
                }
            }
        }
        if self.frontier.is_empty()
            && let Some((_, index)) = nearest
        {
            self.cost[index] = 0;
            self.frontier.push(Reverse((0, index as u32)));
        }
    }
    /// The advice direction at `point`, or `None` when the field has no ready cell there.
    fn advice(&self, point: Vec2) -> Advice {
        let Some((x, y)) = self.cell_of(point) else {
            return Advice::NotReady;
        };
        let here = self.index(x, y);
        let goal_reach = self.radius.max(self.cell);
        if point.distance(self.goal) <= goal_reach && self.cost[here] != UNREACHED {
            return Advice::Toward(self.goal);
        }
        let mut best: Option<(u32, Vec2)> = None;
        for (dx, dy) in NEIGHBOUR_CELLS {
            let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
            if nx < 0 || ny < 0 || nx >= i64::from(self.columns) || ny >= i64::from(self.rows) {
                continue;
            }
            let next = self.index(nx as u32, ny as u32);
            if self.cost[next] == UNREACHED {
                continue;
            }
            // From a cell that the class cannot occupy at its centre, any ready neighbour
            // is a way back. From a ready cell, only a permitted edge downhill.
            if self.cost[here] != UNREACHED
                && (!self.edge(here, next, dx, dy) || self.cost[next] >= self.cost[here])
            {
                continue;
            }
            let centre = self.centre(nx as u32, ny as u32);
            let step = if dx != 0 && dy != 0 {
                DIAGONAL
            } else {
                ORTHOGONAL
            };
            let total = self.cost[next].saturating_add(step);
            if best.is_none_or(|(cost, _)| total < cost) {
                best = Some((total, centre));
            }
        }
        match best {
            Some((_, centre)) => Advice::Toward(centre),
            None if self.cost[here] != UNREACHED => Advice::Toward(self.goal),
            None if self.complete() => Advice::NoPath,
            None => Advice::NotReady,
        }
    }
    fn in_passage(&self, point: Vec2) -> bool {
        self.cell_of(point)
            .is_some_and(|(x, y)| self.passage[self.index(x, y)])
    }
}
const NEIGHBOUR_CELLS: [(i64, i64); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];
#[derive(Clone, Copy, Debug, PartialEq)]
enum Advice {
    Toward(Vec2),
    NotReady,
    NoPath,
}

/// Flock state of the match. Private to the simulation; nothing of it is on the wire.
#[derive(Resource, Default)]
struct FlockState {
    members: BTreeMap<u32, Member>,
    fields: Vec<Field>,
    audit: FlockAudit,
}

/// Privileged test record of the last tick. Reading it changes nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlockAudit {
    pub tick: u32,
    /// Units with a move that changed position in the tick.
    pub moving: usize,
    pub members: Vec<FlockMemberAudit>,
    /// For each owner slot: field work of the tick in cells, and the credit.
    pub owners: Vec<(u8, usize, usize)>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FlockMemberAudit {
    pub unit: u32,
    pub owner: u8,
    /// `moving`, `waiting` or `settled`.
    pub state: &'static str,
    /// Advice velocity, as f32 bits.
    pub advice: [u32; 2],
    /// Neighbour unit ids in the order that the steering sum used.
    pub neighbours: Vec<u32>,
    /// Field identity: body class (radius bits, max step, grade) and goal bits.
    pub field_class: (u32, u32, u64, u64),
    pub field_goal: (u32, u32),
    pub field_revision: u64,
    pub field_ready_cells: usize,
}
pub fn privileged_flock_audit(world: &World) -> FlockAudit {
    world
        .get_resource::<FlockState>()
        .map(|state| state.audit.clone())
        .unwrap_or_default()
}

/// One body of the frozen snapshot.
struct Body {
    row: usize,
    owner: u8,
    position: Vec2,
    radius: f32,
    /// A body that never moves in this tick and that no step may enter for its own owner:
    /// a building or an own unit on Hold. Bodies of another owner are solid for every unit.
    fixed: bool,
}

/// The frozen snapshot: bodies in buckets of `size` metres.
struct Snapshot {
    bodies: Vec<Body>,
    buckets: BTreeMap<(i32, i32), Vec<usize>>,
    size: f32,
}
impl Snapshot {
    fn new(bodies: Vec<Body>) -> Self {
        let size = bodies
            .iter()
            .fold(2.0f32, |size, body| size.max(body.radius * 2.0 + 1.0));
        let mut buckets: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for (index, body) in bodies.iter().enumerate() {
            buckets
                .entry(Self::bucket(body.position, size))
                .or_default()
                .push(index);
        }
        Self {
            bodies,
            buckets,
            size,
        }
    }
    fn bucket(point: Vec2, size: f32) -> (i32, i32) {
        (
            (point.x / size).floor() as i32,
            (point.y / size).floor() as i32,
        )
    }
    /// Body indices near `point`, in no promised order. Callers sort by geometry.
    fn near(&self, point: Vec2, reach: f32, mut visit: impl FnMut(usize)) {
        let span = (reach / self.size).ceil() as i32;
        let (cx, cy) = Self::bucket(point, self.size);
        for y in cy - span..=cy + span {
            for x in cx - span..=cx + span {
                if let Some(bucket) = self.buckets.get(&(x, y)) {
                    bucket.iter().copied().for_each(&mut visit);
                }
            }
        }
    }
}

/// THE neighbour order of the flock (T-7 seam for the reversal test): candidates by
/// distance, then by position. Never by unit id, row or hash order. Bodies on the same
/// centre are left out: they have no separation direction.
fn neighbours_in_sum_order(
    snapshot: &Snapshot,
    positions: &[Vec2],
    me: usize,
    reach: f32,
    accept: impl Fn(&Body) -> bool,
) -> Vec<usize> {
    let here = positions[me];
    let mut found = Vec::new();
    snapshot.near(here, snapshot.size + reach, |other| {
        let body = &snapshot.bodies[other];
        let there = positions[other];
        if other != me && there != here && accept(body) {
            let gap = here.distance(there) - snapshot.bodies[me].radius - body.radius;
            if gap <= reach {
                found.push((
                    here.distance(there).to_bits(),
                    there.x.to_bits(),
                    there.y.to_bits(),
                    other,
                ));
            }
        }
    });
    found.sort_unstable_by_key(|entry| (entry.0, entry.1, entry.2));
    found.into_iter().map(|entry| entry.3).collect()
}

fn is_hold(world: &World, unit: u32) -> bool {
    crate::unit_entity(world, unit)
        .and_then(|entity| world.get::<crate::UnitOrders>(entity))
        .and_then(|orders| orders.current.as_ref())
        .is_some_and(|order| order.behaviour == turts_mod_api::Behaviour::Hold)
}

fn output_row(unit: &MovementFrameUnit) -> MovementFrameOutput {
    MovementFrameOutput {
        recovery: Default::default(),
        unit_id: unit.unit_id,
        position: unit.position,
        target: unit.target,
        progress: unit.progress.clone(),
        plain_progress: unit.plain_progress.clone(),
        physical_escape: None,
        failed_blocked: false,
        arrival_slot: None,
        #[cfg(feature = "test-probes")]
        probe: Default::default(),
    }
}

/// One flock tick for all bodies of the frame.
pub(super) fn step(
    world: &mut World,
    frame: &MovementFrame,
    dt: f32,
) -> (Vec<MovementFrameOutput>, BTreeSet<u32>) {
    let mut state = world.remove_resource::<FlockState>().unwrap_or_default();
    let tick = world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0);
    let result = if let Some(terrain) = world.get_resource::<TerrainState>() {
        let phase = crate::terrain::phase(world, turts_mod_api::ObservationPhase::Start);
        step_inner(
            world,
            terrain,
            phase.as_deref(),
            frame,
            dt,
            tick,
            &mut state,
        )
    } else {
        // A map with no editable terrain: flat ground inside the map bounds. No field is
        // needed; each unit steers on the straight line and the guard keeps it in bounds.
        step_inner(
            world,
            &FlatGround::new(frame.bounds),
            None,
            frame,
            dt,
            tick,
            &mut state,
        )
    };
    world.insert_resource(state);
    result
}

#[allow(clippy::too_many_arguments)]
fn step_inner(
    world: &World,
    terrain: &impl TerrainRead,
    phase: Option<&TerrainPhase>,
    frame: &MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut FlockState,
) -> (Vec<MovementFrameOutput>, BTreeSet<u32>) {
    let units = &frame.units;
    let profiles: Vec<Option<Traversal>> = units
        .iter()
        .map(|unit| Some(super::geometry::profile(world, unit).unwrap_or(LEVEL_GROUND)))
        .collect();
    // Membership: a unit with a target is in the flock of its goal.
    let mut present = BTreeSet::new();
    let mut resettled = BTreeSet::new();
    for unit in units {
        present.insert(unit.unit_id);
        let moves = unit.target.is_some() && unit.speed.is_some();
        if !moves {
            // A unit with no target keeps only a settled entry (its crowd membership).
            if state
                .members
                .get(&unit.unit_id)
                .is_some_and(|member| member.mode != Mode::Settled)
            {
                state.members.remove(&unit.unit_id);
            }
            continue;
        }
        // The goal is the public point of the order, inside the map.
        // A Move order goes to an area: its units form a crowd round the point (rules 2
        // and 3). Every other target (a gather trip, an attack approach, a work approach) is
        // a point that the unit must reach itself.
        let order_point = super::public_move_goal(world, unit.unit_id, unit.owner_slot);
        let crowd = order_point.is_some();
        let goal = order_point
            .or_else(|| super::requested_target(unit))
            .unwrap_or(unit.position);
        // The goal is inside the map, at a pose that the body can take.
        let inset = Vec2::splat(unit.radius + 2.0 * CLEARANCE);
        let goal = goal.clamp(inset, (frame.bounds - inset).max(inset));
        // A unit that settled for this goal and still stands at that pose is at its goal:
        // the same target again does not start a new move.
        if state.members.get(&unit.unit_id).is_some_and(|member| {
            member.mode == Mode::Settled
                && member.goal == goal
                && member.owner == unit.owner_slot
                && member.rest == unit.position
        }) {
            resettled.insert(unit.unit_id);
            continue;
        }
        let fresh = state.members.get(&unit.unit_id).is_none_or(|member| {
            member.mode == Mode::Settled || member.goal != goal || member.owner != unit.owner_slot
        });
        if fresh {
            state.members.insert(
                unit.unit_id,
                Member {
                    owner: unit.owner_slot,
                    goal,
                    mode: Mode::Moving,
                    velocity: Vec2::ZERO,
                    best_distance: unit.position.distance(goal),
                    stalled: 0,
                    rest: unit.position,
                    crowd,
                },
            );
        }
    }
    state.members.retain(|unit, _| present.contains(unit));

    // Stage A: fields. Each owner has a fixed credit; fields in use are served by key order,
    // which is geometry (class, goal), not identity.
    let mut wanted: BTreeMap<FieldKey, (f32, Traversal, Vec2)> = BTreeMap::new();
    for (row, unit) in units.iter().enumerate() {
        if let (Some(member), Some(profile)) = (state.members.get(&unit.unit_id), profiles[row])
            && member.mode != Mode::Settled
        {
            wanted.insert(
                (
                    unit.owner_slot,
                    class_key(unit.radius, profile),
                    goal_key(member.goal),
                ),
                (unit.radius, profile, member.goal),
            );
        }
    }
    // Owner slots of the frame can go past the player slots (tests, neutral bodies).
    let slots = units
        .iter()
        .map(|unit| usize::from(unit.owner_slot) + 1)
        .max()
        .unwrap_or(0)
        .max(usize::from(turts_mod_api::SLOT_COUNT));
    let mut widest = vec![0.0f32; slots];
    for unit in units.iter().filter(|unit| unit.speed.is_some()) {
        let slot = &mut widest[usize::from(unit.owner_slot)];
        *slot = slot.max(unit.radius);
    }
    let mut owners: Vec<(u8, usize, usize)> = (0..slots)
        .map(|owner| (owner as u8, 0, FIELD_CREDIT))
        .collect();
    if let Some(phase) = phase {
        for (key, (radius, profile, goal)) in &wanted {
            if !state.fields.iter().any(|field| field.key == *key) {
                let kept = state
                    .fields
                    .iter()
                    .filter(|field| field.key.0 == key.0)
                    .count();
                if kept >= MAX_FIELDS
                    && let Some(oldest) = state
                        .fields
                        .iter()
                        .enumerate()
                        .filter(|(_, field)| {
                            field.key.0 == key.0 && !wanted.contains_key(&field.key)
                        })
                        .min_by_key(|(_, field)| (field.last_used, field.key))
                        .map(|(index, _)| index)
                {
                    state.fields.remove(oldest);
                }
                state.fields.push(Field::new(
                    *key,
                    phase,
                    *radius,
                    widest[usize::from(key.0)].max(*radius),
                    *profile,
                    *goal,
                ));
            }
        }
        state.fields.sort_by_key(|field| field.key);
        for field in &mut state.fields {
            if !wanted.contains_key(&field.key) {
                continue;
            }
            field.last_used = tick;
            let viewer = field.key.0;
            if !field.lease.is_empty() && !phase.planning_lease_valid(viewer, &field.lease) {
                field.restart();
            }
            let owner = &mut owners[usize::from(viewer)];
            let used = field.build(phase, viewer, owner.2 - owner.1);
            owner.1 += used;
        }
    }

    // The frozen snapshot.
    let holds: Vec<bool> = units
        .iter()
        .map(|unit| unit.speed.is_some() && is_hold(world, unit.unit_id))
        .collect();
    let snapshot = Snapshot::new(
        units
            .iter()
            .enumerate()
            .map(|(row, unit)| Body {
                row,
                owner: unit.owner_slot,
                position: unit.position,
                radius: unit.radius,
                fixed: unit.speed.is_none() || holds[row],
            })
            .collect(),
    );
    let start: Vec<Vec2> = units.iter().map(|unit| unit.position).collect();
    let max_radius = units
        .iter()
        .fold(0.0f32, |radius, unit| radius.max(unit.radius));
    let field_of = |row: usize, member: &Member| -> Option<&Field> {
        let key = (
            units[row].owner_slot,
            class_key(units[row].radius, profiles[row]?),
            goal_key(member.goal),
        );
        state.fields.iter().find(|field| field.key == key)
    };
    // Bodies that no step of `row` may enter: buildings, own units on Hold, every body of
    // another owner (true positions; contact is a permitted fact for the toucher only).
    let solids_for = |row: usize, from: Vec2, reach: f32, capped: bool| -> Vec<Solid> {
        let mut solids = Vec::new();
        snapshot.near(from, reach + max_radius, |other| {
            let body = &snapshot.bodies[other];
            if other == row {
                return;
            }
            // An own body that can move is soft: a step may overlap it up to the cap of
            // rule 4, and never deeper. Every other body is solid at its full radius.
            let soft = !body.fixed && body.owner == units[row].owner_slot;
            if soft && !capped {
                return;
            }
            let radius = if soft {
                body.radius - OVERLAP_CAP * body.radius.min(units[row].radius)
            } else {
                body.radius
            };
            solids.push((
                body.position.x.to_bits(),
                body.position.y.to_bits(),
                Solid {
                    position: body.position,
                    radius,
                },
            ));
        });
        solids.sort_unstable_by_key(|entry| (entry.0, entry.1));
        solids.into_iter().map(|entry| entry.2).collect()
    };
    // `capped`: the step is a unit's own move, so the overlap cap with own bodies applies.
    // A push and the chord of the tick are checked against solid things only.
    let guard = |row: usize, from: Vec2, to: Vec2, capped: bool| -> Vec2 {
        let Some(profile) = profiles[row] else {
            return from;
        };
        if from == to {
            return from;
        }
        let solids = solids_for(row, from, from.distance(to) + units[row].radius, capped);
        guarded_step(terrain, from, to, units[row].radius, profile, &solids)
    };

    // Stage B: advice velocities, all from the snapshot.
    let mut advice = vec![Vec2::ZERO; units.len()];
    let mut no_path = vec![false; units.len()];
    let mut kept_out = vec![false; units.len()];
    let mut aims = start.clone();
    let mut last_step: Vec<Option<Vec2>> = vec![None; units.len()];
    let mut audit_members = Vec::new();
    let mut audit_rows = Vec::new();
    for (row, unit) in units.iter().enumerate() {
        let (Some(member), Some(speed)) = (state.members.get(&unit.unit_id), unit.speed) else {
            continue;
        };
        if member.mode == Mode::Settled {
            continue;
        }
        let field = field_of(row, member);
        let toward = match field.map_or(Advice::NotReady, |field| field.advice(unit.position)) {
            Advice::Toward(point) => point,
            Advice::NotReady => member.goal,
            Advice::NoPath => {
                no_path[row] = true;
                member.goal
            }
        };
        // A unit with no path keeps out of the narrow places that other units of the
        // same owner and goal go through (rule 5: it waits at the side of the flow).
        let keep_out = if no_path[row] {
            let reach = 3.0 * unit.radius;
            state
                .fields
                .iter()
                .filter(|other| {
                    other.key.0 == unit.owner_slot && other.key.2 == goal_key(member.goal)
                })
                .filter_map(|other| other.chokes.as_ref())
                .flatten()
                .map(|choke| (unit.position.distance(*choke), *choke))
                .filter(|(distance, _)| *distance < reach)
                .min_by(|a, b| {
                    (a.0.to_bits(), a.1.x.to_bits(), a.1.y.to_bits()).cmp(&(
                        b.0.to_bits(),
                        b.1.x.to_bits(),
                        b.1.y.to_bits(),
                    ))
                })
                .map(|(_, choke)| (unit.position - choke).normalize_or_zero())
        } else {
            None
        };
        let heading = keep_out.unwrap_or_else(|| (toward - unit.position).normalize_or_zero());
        kept_out[row] = keep_out.is_some();
        aims[row] = toward;
        let reach = unit.radius + max_radius + 1.0;
        let neighbours = neighbours_in_sum_order(&snapshot, &start, row, reach, |body| {
            !body.fixed && body.owner == unit.owner_slot
        });
        let (mut separation, mut alignment, mut centroid, mut flockmates) =
            (Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, 0.0f32);
        let mut give_way = Vec2::ZERO;
        for &other in neighbours.iter().take(NEIGHBOURS) {
            let body = &snapshot.bodies[other];
            let offset = unit.position - body.position;
            let distance = offset.length();
            let gap = distance - unit.radius - body.radius;
            let personal = 0.5 * unit.radius;
            // Separation is between units that move. A unit goes up to a standing unit and
            // touches it (rule 3), or pushes it (rule 1).
            let other_moves = state
                .members
                .get(&units[body.row].unit_id)
                .is_some_and(|other| other.mode == Mode::Moving);
            if gap < personal && other_moves {
                separation += offset / distance * ((personal - gap) / personal).min(2.0);
            }
            // A waiting unit steps to the side of each mover that comes at it (rule 5).
            if member.mode == Mode::Waiting
                && let Some(mover) = state
                    .members
                    .get(&units[body.row].unit_id)
                    .filter(|other| other.mode == Mode::Moving)
            {
                let course = if mover.velocity.length_squared() > 1.0e-6 {
                    mover.velocity.normalize()
                } else {
                    (mover.goal - body.position).normalize_or_zero()
                };
                let ahead = course.dot(offset);
                let side = course.perp_dot(offset);
                let lane = unit.radius + body.radius * 1.5;
                if ahead > 0.0 && side != 0.0 && side.abs() < lane {
                    give_way += course.perp() * side.signum() * (1.0 - side.abs() / lane);
                }
            }
            let mate = state
                .members
                .get(&units[body.row].unit_id)
                .filter(|other| other.mode == Mode::Moving && other.goal == member.goal);
            if let Some(mate) = mate {
                alignment += mate.velocity;
                centroid += body.position;
                flockmates += 1.0;
            }
        }
        let mut wish = heading * speed + separation * speed * 0.6;
        if flockmates > 0.0 {
            wish += (alignment / flockmates - member.velocity) * 0.15;
            wish += (centroid / flockmates - unit.position).clamp_length_max(1.0) * speed * 0.05;
        }
        // A waiting unit holds its place, and gives way to the flow.
        advice[row] = if member.mode == Mode::Waiting {
            ((give_way + if kept_out[row] { heading } else { Vec2::ZERO }) * speed)
                .clamp_length_max(speed)
        } else if unit.position.distance(member.goal) <= speed * dt {
            // The last step goes exactly to the point.
            last_step[row] = Some(member.goal);
            (member.goal - unit.position) / dt
        } else {
            wish.clamp_length_max(speed)
        };
        audit_rows.push(row);
        audit_members.push(FlockMemberAudit {
            unit: unit.unit_id,
            owner: unit.owner_slot,
            state: if member.mode == Mode::Waiting {
                "waiting"
            } else {
                "moving"
            },
            advice: [advice[row].x.to_bits(), advice[row].y.to_bits()],
            neighbours: neighbours
                .iter()
                .take(NEIGHBOURS)
                .map(|&other| units[snapshot.bodies[other].row].unit_id)
                .collect(),
            field_class: field.map_or((0, 0, 0, 0), |field| field.key.1),
            field_goal: goal_key(member.goal),
            field_revision: field.map_or(0, |field| field.revision),
            field_ready_cells: field.map_or(0, |field| field.ready_cells),
        });
    }

    // Stage C, part 1: each mover takes its guarded step. A refused step turns: the unit
    // tries the same step length at 30, 60 and 90 degrees to each side, and takes the
    // turned step that ends nearest its advice point. Equal ends take the smaller turn.
    let mut positions = start.clone();
    for (row, unit) in units.iter().enumerate() {
        let wish = advice[row] * dt;
        if wish == Vec2::ZERO {
            continue;
        }
        let from = unit.position;
        let mut end = guard(row, from, last_step[row].unwrap_or(from + wish), true);
        if end.distance(from) < 0.5 * wish.length() {
            let aim = aims[row];
            let mut best = (end.distance(aim), end);
            for degrees in [30.0f32, -30.0, 60.0, -60.0, 90.0, -90.0] {
                let turned = Vec2::from_angle(degrees.to_radians()).rotate(wish);
                let candidate = guard(row, from, from + turned, true);
                if candidate.distance(from) >= 0.2 * wish.length()
                    && candidate.distance(aim) < best.0
                {
                    best = (candidate.distance(aim), candidate);
                }
            }
            end = best.1;
            // No step at all: the pose is at a terrain contact. Try the same steps as
            // escape steps, which must end with the clearance again.
            if end == from
                && let Some(profile) = profiles[row]
            {
                let solids = solids_for(row, from, wish.length() + unit.radius, true);
                let mut best = (f32::MAX, from);
                for degrees in [
                    0.0f32, 30.0, -30.0, 60.0, -60.0, 90.0, -90.0, 135.0, -135.0, 180.0,
                ] {
                    let turned = Vec2::from_angle(degrees.to_radians()).rotate(wish);
                    let candidate =
                        escape_step(terrain, from, from + turned, unit.radius, profile, &solids);
                    if candidate != from && candidate.distance(aim) < best.0 {
                        best = (candidate.distance(aim), candidate);
                    }
                }
                end = best.1;
            }
        }
        positions[row] = end;
    }

    // Stage C, part 2: own bodies relax overlaps. Each pass reads the positions of the
    // pass start and applies all corrections together. A mover gives way less than a
    // standing own unit, so a moving unit pushes an idle one aside (rule 1).
    let soft = |row: usize| units[row].speed.is_some() && !holds[row] && profiles[row].is_some();
    let weight = |row: usize| -> f32 {
        match state
            .members
            .get(&units[row].unit_id)
            .map(|member| member.mode)
        {
            Some(Mode::Moving) => 0.25,
            _ => 0.75,
        }
    };
    for _ in 0..RELAX_PASSES {
        let relaxed = Snapshot::new(
            units
                .iter()
                .enumerate()
                .map(|(row, unit)| Body {
                    row,
                    owner: unit.owner_slot,
                    position: positions[row],
                    radius: unit.radius,
                    fixed: !soft(row),
                })
                .collect(),
        );
        let before = positions.clone();
        for row in 0..units.len() {
            if !soft(row) {
                continue;
            }
            let neighbours = neighbours_in_sum_order(&relaxed, &before, row, REST_GAP, |body| {
                !body.fixed && body.owner == units[row].owner_slot
            });
            let mut push = Vec2::ZERO;
            for other in neighbours {
                let offset = before[row] - before[other];
                let distance = offset.length();
                // The relaxation aims at a small gap, so standing bodies end apart and the
                // float residue of the corrections is not an overlap.
                let overlap = units[row].radius + units[other].radius + REST_GAP - distance;
                if overlap <= 0.0 {
                    continue;
                }
                let (mine, theirs) = (weight(row), weight(other));
                // Two standing units each take the whole overlap, so a packed crowd comes
                // apart in a few ticks. A mover and a standing unit share it by weight.
                let share = if mine == theirs && mine > 0.5 {
                    1.0
                } else {
                    mine / (mine + theirs)
                };
                push += offset / distance * overlap * share;
            }
            if push != Vec2::ZERO {
                let push = push.clamp_length_max(units[row].radius);
                positions[row] = guard(row, before[row], before[row] + push, true);
                // A push that a wall or a solid body refuses turns to the side, so a unit
                // is not pinned between the pusher and the obstacle.
                if positions[row] == before[row] {
                    for degrees in [60.0f32, -60.0, 90.0, -90.0] {
                        let turned = Vec2::from_angle(degrees.to_radians()).rotate(push);
                        let end = guard(row, before[row], before[row] + turned, true);
                        if end != before[row] {
                            positions[row] = end;
                            break;
                        }
                    }
                }
            }
        }
    }

    // The committed step of a tick is the straight segment from the start position to the
    // final position. The sub-steps above were each checked; their chord is checked again,
    // so that no committed step crosses terrain or a solid body.
    // No unit goes farther in one tick than its speed permits, also when it is pushed.
    for row in 0..units.len() {
        if positions[row] != start[row] {
            let limit = units[row].speed.unwrap_or(0.0) * dt;
            let whole = positions[row] - start[row];
            let chord = whole.clamp_length_max(limit);
            // An unclamped chord ends at the exact position, with no rounding of the sum.
            let end = if chord == whole {
                positions[row]
            } else {
                start[row] + chord
            };
            positions[row] = guard(row, start[row], end, true);
            // A start pose at a terrain contact refuses every guarded chord. The chord is
            // then checked as an escape step, so a unit that left the contact keeps its move.
            if positions[row] == start[row]
                && let Some(profile) = profiles[row]
            {
                let solids = solids_for(row, start[row], chord.length() + units[row].radius, true);
                positions[row] = escape_step(
                    terrain,
                    start[row],
                    end,
                    units[row].radius,
                    profile,
                    &solids,
                );
            }
        }
    }

    // Bodies of different owners move at the same time, and each step was checked against
    // the start pose of the other. When two end poses overlap, both units keep their start
    // pose: contact stops the step, and no unit wins. One pass is sufficient: a unit that
    // stays is at the pose that every other step was checked against.
    let moved = Snapshot::new(
        units
            .iter()
            .enumerate()
            .map(|(row, unit)| Body {
                row,
                owner: unit.owner_slot,
                position: positions[row],
                radius: unit.radius,
                fixed: unit.speed.is_none() || holds[row],
            })
            .collect(),
    );
    let mut stay = vec![false; units.len()];
    for (row, unit) in units.iter().enumerate() {
        if positions[row] == start[row] {
            continue;
        }
        moved.near(positions[row], unit.radius + max_radius, |other| {
            let body = &moved.bodies[other];
            if other != row
                && (body.fixed || body.owner != unit.owner_slot)
                && positions[row].distance(positions[other]) < unit.radius + body.radius
                && positions[row].distance(positions[other]) < start[row].distance(start[other])
            {
                stay[row] = true;
            }
        });
    }
    for row in 0..units.len() {
        if stay[row] {
            positions[row] = start[row];
        }
    }

    // Stage D: arrival and waiting, from the relaxed positions.
    let settled_now = Snapshot::new(
        units
            .iter()
            .enumerate()
            .map(|(row, unit)| Body {
                row,
                owner: unit.owner_slot,
                position: positions[row],
                radius: unit.radius,
                fixed: !soft(row),
            })
            .collect(),
    );
    // The goal area of an order: the disc that holds its members at a packing share of 0.35.
    let mut goal_area: BTreeMap<(u8, GoalKey), f32> = BTreeMap::new();
    for unit in units {
        if let Some(member) = state.members.get(&unit.unit_id) {
            *goal_area
                .entry((unit.owner_slot, goal_key(member.goal)))
                .or_default() += unit.radius * unit.radius;
        }
    }
    for area in goal_area.values_mut() {
        *area = (*area / 0.35).sqrt();
    }
    let mut settle = vec![false; units.len()];
    let mut settle_reason = vec![""; units.len()];
    for (row, unit) in units.iter().enumerate() {
        let (Some(member), Some(speed)) = (state.members.get(&unit.unit_id), unit.speed) else {
            continue;
        };
        if member.mode != Mode::Moving {
            continue;
        }
        let here = positions[row];
        let near = neighbours_in_sum_order(&settled_now, &positions, row, TOUCH, |body| {
            !body.fixed && body.owner == unit.owner_slot
        });
        let gap =
            |other: usize| here.distance(positions[other]) - unit.radius - units[other].radius;
        // A unit settles when no overlap is deeper than the cap of a step. The standing
        // units then relax the rest to zero (no overlap at rest).
        let overlaps = near.iter().any(|&other| {
            gap(other) < -(OVERLAP_CAP * unit.radius.min(units[other].radius) + NO_OVERLAP)
        });
        let touches_crowd = near.iter().any(|&other| {
            state
                .members
                .get(&units[other].unit_id)
                .is_some_and(|other| other.mode == Mode::Settled && other.goal == member.goal)
        });
        // A unit one step from its point takes that step exactly, so only the exact pose
        // is at the point.
        let at_point = here == member.goal;
        // A unit that makes no approach inside the goal area of its order stops there.
        let area = goal_area
            .get(&(unit.owner_slot, goal_key(member.goal)))
            .copied()
            .unwrap_or(0.0);
        let gave_up =
            member.stalled >= SETTLE_TICKS && !no_path[row] && here.distance(member.goal) <= area;
        let in_passage = field_of(row, member).is_some_and(|field| field.in_passage(here));
        // Rule 3 applies inside the goal area. Outside it, a unit that touches the crowd goes
        // on, so that the crowd fills as a blob and does not grow as a line toward the flow.
        // A unit that cannot get in settles at the crowd after a longer time with no approach.
        let inside = here.distance(member.goal) <= area;
        // A unit that cannot go on stops at a settled or waiting own unit of any order.
        // An idle unit with no order is not a place to stop: it is pushed aside (rule 1).
        let touches_standing = near.iter().any(|&other| {
            state
                .members
                .get(&units[other].unit_id)
                .is_some_and(|other| other.mode != Mode::Moving)
        });
        // The same holds at a solid body (an enemy, a building, an own unit on Hold).
        let mut touches_solid = false;
        // A unit that a solid body stops can stand up to two steps from it.
        let reach = TOUCH + 2.0 * speed * dt;
        snapshot.near(here, unit.radius + max_radius + reach, |other| {
            let body = &snapshot.bodies[other];
            if other != row
                && (body.fixed || body.owner != unit.owner_slot)
                && here.distance(positions[other]) - unit.radius - body.radius <= reach
            {
                touches_solid = true;
            }
        });
        // Only a unit of a Move order stops at its crowd or in its goal area. A unit with a
        // point target goes on to the point and pushes a standing own unit off it (rule 1).
        let at_crowd = member.crowd
            && ((touches_crowd && inside)
                || (touches_standing && member.stalled >= 2 * SETTLE_TICKS && !no_path[row]));
        let at_solid = touches_solid && member.stalled >= 2 * SETTLE_TICKS && !no_path[row];
        let gave_up = gave_up && member.crowd;
        settle[row] = (at_point || at_crowd || at_solid || gave_up) && !overlaps && !in_passage;
        settle_reason[row] = if at_point {
            "settled at the point"
        } else if touches_crowd {
            "settled at the crowd"
        } else {
            "settled in the goal area"
        };
    }

    let mut outputs: Vec<MovementFrameOutput> = units.iter().map(output_row).collect();
    let mut waiting = BTreeSet::new();
    let mut moving = 0;
    for (row, unit) in units.iter().enumerate() {
        outputs[row].position = positions[row];
        let Some(member) = state.members.get_mut(&unit.unit_id) else {
            continue;
        };
        if member.mode == Mode::Settled {
            if resettled.contains(&unit.unit_id) {
                outputs[row].target = None;
                if let Some(progress) = outputs[row].progress.as_mut() {
                    progress.activity = MovementActivity::Resting;
                }
                if let Some(progress) = outputs[row].plain_progress.as_mut() {
                    progress.activity = MovementActivity::Resting;
                }
            }
            continue;
        }
        member.velocity = (positions[row] - start[row]) / dt;
        moving += usize::from(positions[row] != start[row]);
        let distance = positions[row].distance(member.goal);
        if distance < member.best_distance - 0.1 * unit.radius {
            member.best_distance = distance;
            member.stalled = 0;
        } else {
            member.stalled = member.stalled.saturating_add(1);
        }
        if settle[row] {
            member.mode = Mode::Settled;
            member.velocity = Vec2::ZERO;
            member.rest = positions[row];
            outputs[row].target = None;
            if let Some(progress) = outputs[row].progress.as_mut() {
                progress.activity = MovementActivity::Resting;
            }
            if let Some(progress) = outputs[row].plain_progress.as_mut() {
                progress.activity = MovementActivity::Resting;
            }
            continue;
        }
        member.mode = match member.mode {
            Mode::Moving if no_path[row] && member.stalled >= WAIT_TICKS => Mode::Waiting,
            Mode::Waiting if !no_path[row] => {
                member.stalled = 0;
                member.best_distance = distance;
                Mode::Moving
            }
            mode => mode,
        };
        if member.mode == Mode::Waiting {
            waiting.insert(unit.unit_id);
        }
        for progress in outputs[row].progress.iter_mut() {
            progress.activity = MovementActivity::Moving;
        }
        for progress in outputs[row].plain_progress.iter_mut() {
            progress.activity = MovementActivity::Moving;
        }
    }
    for (member, row) in audit_members.iter_mut().zip(audit_rows) {
        if settle[row] {
            member.state = settle_reason[row];
        }
    }
    audit_members.sort_by_key(|member| member.unit);
    state.audit = FlockAudit {
        tick,
        moving,
        members: audit_members,
        owners,
    };
    (outputs, waiting)
}
