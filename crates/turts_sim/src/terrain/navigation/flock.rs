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
    flock_step::{CLEARANCE, Solid, escape_step, fitted_step, guarded_step},
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
/// Passes of simultaneous overlap corrections between own bodies. The push alone holds
/// rule 4 against own units (no step check between them), so a dense crowd needs 16; a pass
/// with no correction ends the relaxation early.
const RELAX_PASSES: usize = 16;
/// Field work credit of one owner for one tick, in cells.
pub const FIELD_CREDIT: usize = 16_384;
/// Fields that one owner keeps.
const MAX_FIELDS: usize = 16;
/// Cost of one cell step; a diagonal step costs `DIAGONAL`.
const ORTHOGONAL: u32 = 1000;
const DIAGONAL: u32 = 1414;
const UNREACHED: u32 = u32::MAX;
/// The deepest overlap with an own body that lets a mover settle, as a share of the smaller
/// radius. The standing units then relax the rest to zero (rule 4: no overlap at rest; FM3
/// limits a moving overlap to 25 %).
const SETTLE_OVERLAP: f32 = 0.10;
/// Ticks with no own mover near before a pushed idle unit goes back to its place (V6).
const RETURN_DELAY: u16 = 20;
/// The distance at which a unit looks at bodies in its way: four radii and one step.
fn look_round(radius: f32, speed: f32, dt: f32) -> f32 {
    4.0 * radius + speed * dt
}
/// The deepest overlap of two own bodies after the push, as a share of the smaller radius.
/// A deeper pair takes a smaller part of its steps. Rule 4 permits 25 %; the margin keeps
/// the float residue of the relaxation under it.
const MOVING_OVERLAP: f32 = 0.20;
/// The gap that the relaxation leaves between two bodies.
const REST_GAP: f32 = 1.0e-3;
/// A gap up to this value between two bodies is contact.
const TOUCH: f32 = 0.05;
/// An overlap up to this value is none (float residue of the relaxation).
const NO_OVERLAP: f32 = 1.0e-3;
/// Ticks with no progress before a unit with no path becomes a waiting unit.
const WAIT_TICKS: u16 = 10;
/// A unit this near a tight node is on it (metres).
const NODE_REACHED: f32 = 0.01;
/// Goal areas from the goal within which a unit that a standing body stops ends its move
/// there. Farther away it waits with its order kept (V2).
const NEAR_GOAL: f32 = 2.0;
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
    /// The tick at which the unit came into the passage where it is now.
    passage_since: Option<u32>,
    /// The pose at which the unit came into that passage.
    passage_entry: Vec2,
    /// The unit waits because bodies stop it, not because it has no path (rule 5, V2).
    blocked: bool,
}

impl Member {
    /// The direction in which the unit goes: its last velocity, else the line to its goal.
    fn course(&self, at: Vec2) -> Vec2 {
        self.velocity
            .try_normalize()
            .unwrap_or((self.goal - at).normalize_or_zero())
    }
}

/// The seeded coin of a tie in a passage (V5): of two entry poses, the one that goes first.
/// Both units draw the same coin, because the poses are put in a fixed order by geometry.
fn tie_coin(key: u64, tick: u32, a: Vec2, b: Vec2) -> Vec2 {
    let bits = |at: Vec2| (u64::from(at.x.to_bits()) << 32) | u64::from(at.y.to_bits());
    let (low, high) = if bits(a) <= bits(b) { (a, b) } else { (b, a) };
    let draw = crate::terrain::mix64(
        key ^ crate::terrain::mix64(u64::from(tick))
            ^ crate::terrain::mix64(bits(low)).rotate_left(17)
            ^ crate::terrain::mix64(bits(high)).rotate_left(41),
    );
    if draw & 1 == 0 { low } else { high }
}

/// One cost-to-goal field, from one owner's permitted terrain. Its nodes are the points of
/// a lattice of half a terrain cell: cell centres, edge midpoints and cell corners. So the
/// centre line of an opening that is exactly as wide as the body is a row of nodes.
struct Field {
    key: FieldKey,
    /// Terrain cells of the map.
    cell_columns: u32,
    cell_rows: u32,
    /// Nodes of the lattice: two for each terrain cell, plus one.
    columns: u32,
    rows: u32,
    /// The distance between two nodes.
    cell: f32,
    radius: f32,
    /// The largest radius of the owner's units that can move; it sets the passage width.
    widest: f32,
    profile: Traversal,
    goal: Vec2,
    /// Chunk revisions of the permitted view that the field read.
    lease: BTreeMap<usize, u64>,
    revision: u64,
    /// Rows of nodes whose clearance is known.
    scanned_rows: u32,
    passable: Vec<bool>,
    passage: Vec<bool>,
    /// The reached passages and their mouths: the nodes within 2.5 radii of a choke.
    zone: Vec<bool>,
    /// A body at this node touches terrain, or nearly: a step to it must end exactly on it.
    tight: Vec<bool>,
    /// Heights of the terrain cells.
    heights: Vec<Option<i32>>,
    /// The level of each node: the lowest of the cells that it touches, when a body can
    /// step between all of them.
    levels: Vec<Option<i32>>,
    cost: Vec<u32>,
    frontier: BinaryHeap<Reverse<(u32, u32)>>,
    seeded: bool,
    ready_cells: usize,
    /// Positions of the passage nodes that the field reaches, known when it is complete.
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
        let (columns, rows) = (2 * grid.columns + 1, 2 * grid.rows + 1);
        let nodes = (columns * rows) as usize;
        Self {
            key,
            cell_columns: grid.columns,
            cell_rows: grid.rows,
            columns,
            rows,
            cell: grid.cell_width as f32 / grid.length_scale as f32 / 2.0,
            radius,
            widest,
            profile,
            goal,
            lease: BTreeMap::new(),
            revision: 0,
            scanned_rows: 0,
            passable: vec![false; nodes],
            passage: vec![false; nodes],
            zone: vec![false; nodes],
            tight: vec![false; nodes],
            heights: vec![None; (grid.columns * grid.rows) as usize],
            levels: vec![None; nodes],
            cost: vec![UNREACHED; nodes],
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
        self.zone.fill(false);
        self.tight.fill(false);
        self.heights.fill(None);
        self.levels.fill(None);
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
    /// The position of a node.
    fn centre(&self, x: u32, y: u32) -> Vec2 {
        Vec2::new(x as f32 * self.cell, y as f32 * self.cell)
    }
    /// The node nearest a point.
    fn cell_of(&self, point: Vec2) -> Option<(u32, u32)> {
        let (x, y) = ((point.x / self.cell).round(), (point.y / self.cell).round());
        (x >= 0.0 && y >= 0.0 && (x as u32) < self.columns && (y as u32) < self.rows)
            .then_some((x as u32, y as u32))
    }
    fn height(&self, x: i64, y: i64) -> Option<i32> {
        (x >= 0 && y >= 0 && x < i64::from(self.cell_columns) && y < i64::from(self.cell_rows))
            .then(|| self.heights[(y * i64::from(self.cell_columns) + x) as usize])
            .flatten()
    }
    /// The terrain cells that a node touches, on one axis: one cell for a node inside a
    /// cell, two for a node on a grid line.
    fn touched(node: u32) -> [i64; 2] {
        let node = i64::from(node);
        if node % 2 == 1 {
            [(node - 1) / 2, (node - 1) / 2]
        } else {
            [node / 2 - 1, node / 2]
        }
    }
    fn steps(&self, from: usize, to: usize) -> bool {
        match (self.levels[from], self.levels[to]) {
            (Some(a), Some(b)) => a.abs_diff(b) <= self.profile.max_step,
            _ => false,
        }
    }
    /// Spend `credit` nodes of work. Returns the work that was used.
    fn build(&mut self, phase: &TerrainPhase, viewer: u8, credit: usize) -> usize {
        let mut used = 0;
        if self.scanned_rows == 0 && self.lease.is_empty() {
            let max = Cell {
                x: self.cell_columns - 1,
                y: self.cell_rows - 1,
            };
            phase.planning_rectangle(viewer, Cell { x: 0, y: 0 }, max, &mut self.lease);
            for y in 0..self.cell_rows {
                for x in 0..self.cell_columns {
                    self.heights[(y * self.cell_columns + x) as usize] =
                        phase.planning_elevation(viewer, Cell { x, y });
                }
            }
            for y in 0..self.rows {
                for x in 0..self.columns {
                    let mut level: Option<i32> = None;
                    let mut whole = true;
                    for cy in Self::touched(y) {
                        for cx in Self::touched(x) {
                            match (self.height(cx, cy), level) {
                                (None, _) => whole = false,
                                (Some(height), None) => level = Some(height),
                                (Some(height), Some(low)) => {
                                    whole &= height.abs_diff(low) <= self.profile.max_step;
                                    level = Some(low.min(height));
                                }
                            }
                        }
                    }
                    let index = self.index(x, y);
                    self.levels[index] = level.filter(|_| whole);
                }
            }
        }
        // Clearance: the distance from the node to the nearest terrain cell that the class
        // cannot step to. The window covers two diameters, for the passage rule.
        let terrain_cell = 2.0 * self.cell;
        let reach = (4.0 * self.widest / terrain_cell).ceil() as i64 + 1;
        while self.scanned_rows < self.rows && used + self.columns as usize <= credit {
            let y = self.scanned_rows;
            for x in 0..self.columns {
                let index = self.index(x, y);
                let here = self.centre(x, y);
                let mut clearance = f32::MAX;
                if let Some(level) = self.levels[index] {
                    let (cx, cy) = (Self::touched(x)[1], Self::touched(y)[1]);
                    for ny in cy - reach..=cy + reach {
                        for nx in cx - reach..=cx + reach {
                            if self.height(nx, ny).is_some_and(|height| {
                                height.abs_diff(level) <= self.profile.max_step
                            }) {
                                continue;
                            }
                            let low = Vec2::new(nx as f32 * terrain_cell, ny as f32 * terrain_cell);
                            let nearest = here.clamp(low, low + Vec2::splat(terrain_cell));
                            clearance = clearance.min(here.distance(nearest));
                        }
                    }
                } else {
                    clearance = 0.0;
                }
                // A body that exactly fits can stand at the node.
                self.passable[index] = clearance >= self.radius - 1.0e-4;
                // Tight: the wider check body of the step guard cannot stand here.
                self.tight[index] = clearance < self.radius + 2.0 * CLEARANCE;
                // A corridor narrower than two diameters is a passage (design record).
                // The corridor width is the shorter of the two free runs through the node,
                // along x and along y, so one wall beside open ground is not a passage.
                let run = |dx: i64, dy: i64| -> i64 {
                    let mut free = 0;
                    for step in 1..=2 * reach {
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
                let width = (run(1, 0) + run(-1, 0)).min(run(0, 1) + run(0, -1));
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
                        let reach = 2.5 * self.radius;
                        let span = (reach / self.cell).ceil() as i64;
                        for ny in i64::from(y) - span..=i64::from(y) + span {
                            for nx in i64::from(x) - span..=i64::from(x) + span {
                                let inside = nx >= 0
                                    && ny >= 0
                                    && nx < i64::from(self.columns)
                                    && ny < i64::from(self.rows);
                                if inside
                                    && self
                                        .centre(nx as u32, ny as u32)
                                        .distance(self.centre(x, y))
                                        <= reach
                                {
                                    let near = self.index(nx as u32, ny as u32);
                                    self.zone[near] = true;
                                }
                            }
                        }
                    }
                }
            }
            self.chokes = Some(chokes);
        }
        used
    }
    /// A body of the class can go between two adjacent passable nodes.
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
    /// Goal nodes: the passable nodes within one radius (at least one terrain cell) of the
    /// goal. With none, the passable node nearest the goal (a tie goes to the lower node
    /// index, which is a position, not an identity).
    fn seed(&mut self) {
        let reach = self.radius.max(2.0 * self.cell);
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
    /// The advice at `point`: the next node to go to, and whether the step to it must end
    /// exactly on it (a body there touches terrain, or nearly).
    fn advice(&self, point: Vec2) -> Advice {
        let Some((x, y)) = self.cell_of(point) else {
            return Advice::NotReady;
        };
        let here = self.index(x, y);
        let goal_reach = self.radius.max(2.0 * self.cell);
        if point.distance(self.goal) <= goal_reach && self.cost[here] != UNREACHED {
            return Advice::Toward(self.goal, false);
        }
        let mut best: Option<(u32, Vec2, bool)> = None;
        for (dx, dy) in NEIGHBOUR_CELLS {
            let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
            if nx < 0 || ny < 0 || nx >= i64::from(self.columns) || ny >= i64::from(self.rows) {
                continue;
            }
            let next = self.index(nx as u32, ny as u32);
            if self.cost[next] == UNREACHED {
                continue;
            }
            // From a node that the class cannot occupy, any ready neighbour is a way back.
            // From a ready node, only a permitted edge downhill.
            if self.cost[here] != UNREACHED
                && (!self.edge(here, next, dx, dy) || self.cost[next] >= self.cost[here])
            {
                continue;
            }
            let step = if dx != 0 && dy != 0 {
                DIAGONAL
            } else {
                ORTHOGONAL
            };
            let total = self.cost[next].saturating_add(step);
            if best.is_none_or(|(cost, _, _)| total < cost) {
                best = Some((total, self.centre(nx as u32, ny as u32), self.tight[next]));
            }
        }
        // At an exact-fit place the unit must be on the line from its node to the next node.
        // A unit that is off that line goes to its own node first.
        if let Some((_, node, _)) = best
            && self.cost[here] != UNREACHED
            && self.tight[here]
        {
            let own = self.centre(x, y);
            let line = (node - own).normalize_or_zero();
            let offset = point - own;
            if offset.perp_dot(line).abs() > 1.0e-5 || offset.dot(line) < 0.0 {
                return Advice::Toward(own, true);
            }
        }
        match best {
            Some((_, node, tight)) => Advice::Toward(node, tight || self.tight[here]),
            None if self.cost[here] != UNREACHED => Advice::Toward(self.goal, false),
            None if self.complete() => Advice::NoPath,
            None => Advice::NotReady,
        }
    }
    fn in_zone(&self, point: Vec2) -> bool {
        self.cell_of(point)
            .is_some_and(|(x, y)| self.zone[self.index(x, y)])
    }
    fn in_passage(&self, point: Vec2) -> bool {
        self.cell_of(point)
            .is_some_and(|(x, y)| self.passage[self.index(x, y)])
    }
    /// A body of this class can stand at `point` (nearest node).
    fn passable_at(&self, point: Vec2) -> bool {
        self.cell_of(point)
            .is_some_and(|(x, y)| self.passable[self.index(x, y)])
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
    /// Go to this point. `true`: the step to it must end exactly on it.
    Toward(Vec2, bool),
    NotReady,
    NoPath,
}

/// Flock state of the match. Private to the simulation; nothing of it is on the wire.
#[derive(Resource, Default)]
struct FlockState {
    members: BTreeMap<u32, Member>,
    /// For each idle unit: the pose at which it last became idle (V6).
    homes: BTreeMap<u32, Home>,
    fields: Vec<Field>,
    audit: FlockAudit,
}

/// The place of an idle unit (Clara, V6, 2026-11-24). An own push can move the unit away;
/// it goes back when no own mover was near for `RETURN_DELAY` ticks. A new order clears the
/// place. A shift of less than one radius, or a shift while no return is due, moves the
/// place with the unit.
#[derive(Clone, Copy, Debug)]
struct Home {
    pose: Vec2,
    /// Ticks in a row with no own mover near.
    quiet: u16,
    /// The unit goes back to `pose`.
    returning: bool,
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

/// The unit moves to a point that it can share with other units: it has a Move order, an
/// attack-move, or a target with no order (a rally walk).
fn goes_to_an_area(world: &World, unit: u32) -> bool {
    crate::unit_entity(world, unit)
        .and_then(|entity| world.get::<crate::UnitOrders>(entity))
        .and_then(|orders| orders.current.as_ref())
        .is_none_or(|order| match order.behaviour {
            turts_mod_api::Behaviour::Move => true,
            turts_mod_api::Behaviour::Attack => {
                matches!(order.target, crate::orders::OrderTargetSim::Point(_))
            }
            _ => false,
        })
}

/// An own unit that other own units do not push and do not step into: a unit on Hold, and a
/// unit with a terrain work order, in its cells and on the short walks between them (Clara,
/// "Busy own unit", 2026-11-24). It still takes its own steps.
fn stands_firm(world: &World, unit: u32) -> bool {
    crate::unit_entity(world, unit)
        .and_then(|entity| world.get::<crate::UnitOrders>(entity))
        .and_then(|orders| orders.current.as_ref())
        .is_some_and(|order| {
            matches!(
                order.behaviour,
                turts_mod_api::Behaviour::Hold | turts_mod_api::Behaviour::TerrainWork
            )
        })
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
            terrain.tie_key(),
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
            0,
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
    tie_key: u64,
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
        if moves {
            state.homes.remove(&unit.unit_id);
        } else {
            state.homes.entry(unit.unit_id).or_insert(Home {
                pose: unit.position,
                quiet: 0,
                returning: false,
            });
        }
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
        // A move to a point goes to an area: the units form a crowd round the point (rules 2
        // and 3). The target of a gather trip, an attack on a unit or a work approach is a
        // point that the unit must reach itself.
        let order_point = super::public_move_goal(world, unit.unit_id, unit.owner_slot);
        let crowd = goes_to_an_area(world, unit.unit_id);
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
                    passage_since: None,
                    passage_entry: Vec2::ZERO,
                    blocked: false,
                },
            );
        }
    }
    state.members.retain(|unit, _| present.contains(unit));
    state.homes.retain(|unit, _| present.contains(unit));

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
        .map(|unit| unit.speed.is_some() && stands_firm(world, unit.unit_id))
        .collect();
    // A firm unit that walks (a busy unit between its cells) is not pushed, but own units are
    // not held at its start pose: they make room for it in the push.
    let walks_firm: Vec<bool> = units
        .iter()
        .enumerate()
        .map(|(row, unit)| holds[row] && unit.target.is_some())
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
    // An own body that can move is not in the step check: it acts only through the
    // separation force and the push (T-56 change A; rule 4 of 2026-11-23).
    let solids_for = |row: usize, from: Vec2, reach: f32| -> Vec<Solid> {
        let mut solids = Vec::new();
        snapshot.near(from, reach + max_radius, |other| {
            let body = &snapshot.bodies[other];
            if other == row
                || ((!body.fixed || walks_firm[other]) && body.owner == units[row].owner_slot)
            {
                return;
            }
            solids.push((
                body.position.x.to_bits(),
                body.position.y.to_bits(),
                Solid {
                    position: body.position,
                    radius: body.radius,
                },
            ));
        });
        solids.sort_unstable_by_key(|entry| (entry.0, entry.1));
        solids.into_iter().map(|entry| entry.2).collect()
    };
    let guard = |row: usize, from: Vec2, to: Vec2| -> Vec2 {
        let Some(profile) = profiles[row] else {
            return from;
        };
        if from == to {
            return from;
        }
        let solids = solids_for(row, from, from.distance(to) + units[row].radius);
        guarded_step(terrain, from, to, units[row].radius, profile, &solids)
    };

    // A body that can stop a unit of `goal` on its way (rule 5, V2): a solid body, a body of
    // another owner, an own unit that waits, and an own idle unit that is not in the crowd
    // of `goal`. An own mover and the own crowd of the goal do not stop it.
    let stops = |row: usize, goal: Vec2, other: usize| -> bool {
        let body = &snapshot.bodies[other];
        if other == row {
            return false;
        }
        if body.fixed || body.owner != units[row].owner_slot {
            return true;
        }
        state
            .members
            .get(&units[other].unit_id)
            .is_none_or(|other| match other.mode {
                Mode::Moving => false,
                Mode::Waiting => true,
                Mode::Settled => other.goal != goal,
            })
    };

    // Stage B: advice velocities, all from the snapshot.
    let basis = crate::visibility::active_basis(world);
    let mut advice = vec![Vec2::ZERO; units.len()];
    let mut no_path = vec![false; units.len()];
    let mut kept_out = vec![false; units.len()];
    let mut yielding = vec![false; units.len()];
    let mut at_mouth = vec![false; units.len()];
    let mut way_free = vec![false; units.len()];
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
        let mut exact_node = false;
        // A unit that stands within `NODE_REACHED` of a tight node is on it: the exact step
        // can be refused there (a node at a terrain contact), and the unit then turns round
        // the node with no approach. It takes the advice of the node and goes on.
        let advice_here = |field: &Field| match field.advice(unit.position) {
            Advice::Toward(node, true) if unit.position.distance(node) < NODE_REACHED => {
                field.advice(node)
            }
            advice => advice,
        };
        let toward = match field.map_or(Advice::NotReady, advice_here) {
            Advice::Toward(point, exact) => {
                exact_node = exact;
                point
            }
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
        // Right of way in a passage, between owners and between own units that go opposite
        // ways: the body that came into the passage first keeps it ("arrive first", CEO
        // 2026-11-12; V3). A unit outside waits at the mouth. A unit inside goes back out
        // only when it meets the holder head-on and the corridor is too narrow for both
        // bodies. Arrivals in one tick (V5): the slower body goes first; at equal speed a
        // seeded coin of the arrival tick and the two entry poses decides. The unit id and
        // the slot never decide. Only permitted bodies count.
        let first_of = |theirs: Option<u32>, entry: Vec2, their_speed: f32| -> bool {
            let mine = member.passage_since.unwrap_or(u32::MAX);
            let theirs = theirs.unwrap_or(u32::MAX);
            if theirs != mine {
                return theirs < mine;
            }
            if their_speed != speed {
                return their_speed < speed;
            }
            tie_coin(tie_key, mine, entry, member.passage_entry) == entry
        };
        // Inside: in the passage itself. A unit at a mouth has not come in yet (V3).
        let inside = field.is_some_and(|field| field.in_passage(unit.position));
        let holder = field.filter(|_| !no_path[row]).and_then(|field| {
            neighbours_in_sum_order(&snapshot, &start, row, 10.0 * unit.radius, |body| {
                if body.owner != unit.owner_slot {
                    return basis.is_none_or(|basis| {
                        basis.unit_permitted(unit.owner_slot, units[body.row].unit_id)
                    });
                }
                // A busy unit does not give the passage to own units: they go round it or
                // wait for it ("Busy own unit", Clara 2026-11-24).
                if holds[row] {
                    return false;
                }
                // An own body that does not move (a building, a unit on Hold, a busy unit)
                // in the passage holds it.
                if body.fixed {
                    return true;
                }
                // An own unit that goes the other way to another goal is a holder also.
                state
                    .members
                    .get(&units[body.row].unit_id)
                    .is_some_and(|other| {
                        other.mode == Mode::Moving
                            && other.goal != member.goal
                            && heading.dot((other.goal - body.position).normalize_or_zero()) < 0.0
                    })
            })
            .into_iter()
            .take(NEIGHBOURS)
            .map(|other| &snapshot.bodies[other])
            .find(|body| {
                // A body that goes somewhere holds the passage and its mouths from its
                // arrival. A body that stands holds only the passage itself, from the start.
                let active = state
                    .members
                    .get(&units[body.row].unit_id)
                    .filter(|other| other.mode != Mode::Settled);
                let theirs = active.map_or(Some(0), |other| other.passage_since);
                // A part of the body is in the passage or its mouth.
                let part_in_zone = [Vec2::ZERO, Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                    .into_iter()
                    .any(|side| field.in_zone(body.position + side * body.radius));
                let holds = match active {
                    Some(other) => other.passage_since.is_some() && part_in_zone,
                    None => field.in_passage(body.position),
                };
                let entry = active.map_or(body.position, |other| other.passage_entry);
                let course = state
                    .members
                    .get(&units[body.row].unit_id)
                    .map_or(Vec2::ZERO, |other| other.course(body.position));
                // Inside the passage, the unit goes back only from a head-on meeting in a
                // corridor where the two bodies can not pass; else it stays in. They can pass
                // when this unit can stand beside the other body, on one side.
                let beside = heading.perp() * (unit.radius + body.radius + 2.0 * CLEARANCE);
                let head_on = course.dot(unit.position - body.position) > 0.0
                    && heading.dot(body.position - unit.position) > 0.0
                    && !field.passable_at(body.position + beside)
                    && !field.passable_at(body.position - beside);
                holds
                    && (if inside {
                        head_on
                    } else {
                        heading.dot(body.position - unit.position) > 0.0
                    })
                    && first_of(theirs, entry, units[body.row].speed.unwrap_or(0.0))
            })
            .map(|body| {
                let course = state
                    .members
                    .get(&units[body.row].unit_id)
                    .map_or(Vec2::ZERO, |other| other.course(body.position));
                (body.position, course)
            })
        });
        let give_passage = holder.map(|(holder, course)| {
            let offset = unit.position - holder;
            if inside {
                // Inside the passage the unit goes back out, to the mouth behind it.
                return (0.25 * offset.normalize_or_zero() - heading).normalize_or_zero() * speed;
            }
            // Outside it keeps the mouth of the passage free and steps out of the course
            // of the holder.
            let mut aside = field
                .and_then(|field| field.chokes.as_ref())
                .into_iter()
                .flatten()
                .map(|choke| (unit.position.distance(*choke), *choke))
                .filter(|(distance, _)| *distance < 6.0 * unit.radius)
                .min_by(|a, b| {
                    (a.0.to_bits(), a.1.x.to_bits(), a.1.y.to_bits()).cmp(&(
                        b.0.to_bits(),
                        b.1.x.to_bits(),
                        b.1.y.to_bits(),
                    ))
                })
                .map_or(Vec2::ZERO, |(_, choke)| {
                    (unit.position - choke).normalize_or_zero()
                });
            // A holder that goes somewhere has a course; the unit steps out of it. A holder
            // that stands has none, and the unit only keeps the mouth free.
            if let Some(course) = course.try_normalize() {
                let side = course.perp_dot(offset);
                if course.dot(offset) > 0.0 && side.abs() < 2.5 * unit.radius {
                    aside += course.perp() * if side < 0.0 { -1.0 } else { 1.0 };
                }
            }
            aside.normalize_or_zero() * speed
        });
        yielding[row] = give_passage.is_some();
        at_mouth[row] = give_passage.is_some() && !inside;
        // A unit that bodies stop waits until one step on its way is free of them (V2).
        if member.mode == Mode::Waiting
            && member.blocked
            && let Some(profile) = profiles[row]
        {
            let to = unit.position + heading * speed * dt;
            let mut solids = Vec::new();
            snapshot.near(
                unit.position,
                speed * dt + unit.radius + max_radius,
                |other| {
                    if stops(row, member.goal, other) {
                        let body = &snapshot.bodies[other];
                        solids.push((
                            body.position.x.to_bits(),
                            body.position.y.to_bits(),
                            Solid {
                                position: body.position,
                                radius: body.radius,
                            },
                        ));
                    }
                },
            );
            solids.sort_unstable_by_key(|entry| (entry.0, entry.1));
            let solids: Vec<Solid> = solids.into_iter().map(|entry| entry.2).collect();
            way_free[row] = give_passage.is_none()
                && guarded_step(terrain, unit.position, to, unit.radius, profile, &solids) == to;
        }
        // A unit that gives the passage aims where that advice goes, so a turned step
        // keeps that way and does not turn back toward the goal.
        aims[row] = give_passage.map_or(toward, |advice| unit.position + advice * dt);
        let reach = unit.radius + max_radius + 1.0;
        let neighbours = neighbours_in_sum_order(&snapshot, &start, row, reach, |body| {
            !body.fixed && body.owner == unit.owner_slot
        });
        let (mut separation, mut alignment, mut centroid, mut flockmates) =
            (Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, 0.0f32);
        let mut give_way = Vec2::ZERO;
        let in_own_zone = field.is_some_and(|field| field.in_zone(unit.position));
        let mut queue_hold = false;
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
                // Two own units that meet head-on pass on the right: each steers to its own
                // right side, so the rule has no winner and needs no identity.
                let other_course = state
                    .members
                    .get(&units[body.row].unit_id)
                    .map_or(Vec2::ZERO, |other| other.course(body.position));
                if heading.dot(other_course) < -0.5 && heading.dot(-offset) > 0.0 {
                    separation -= heading.perp();
                }
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
            // In a passage or its mouth, own units of one field go one after the other: a
            // unit that touches a mate that is nearer the goal (by field cost; equal cost:
            // the lower position) lets the mate go first. The unit id never decides.
            if mate.is_some()
                && in_own_zone
                && gap < 0.25 * unit.radius
                && body.radius == unit.radius
                && let Some(field) = field
            {
                let rank = |at: Vec2| {
                    let cost = field
                        .cell_of(at)
                        .map_or(UNREACHED, |(x, y)| field.cost[field.index(x, y)]);
                    (cost, at.x.to_bits(), at.y.to_bits())
                };
                // A unit on a node that its class cannot occupy has no cost: it does not wait.
                queue_hold |=
                    rank(unit.position).0 != UNREACHED && rank(body.position) < rank(unit.position);
            }
            if let Some(mate) = mate {
                alignment += mate.velocity;
                centroid += body.position;
                flockmates += 1.0;
            }
        }
        // Bodies in the way: buildings, own units on Hold, the bodies of another owner that
        // this owner is permitted to see now, and own units that stand and are not in the
        // crowd of this goal. Each is a stationary obstacle: the unit steers to the side on
        // which it already is. A body dead ahead gives no side (no tie winner); a hidden body
        // gives no force. Contact with a solid body stops the step; an own unit is pushed
        // aside (rule 1). A unit goes up to its own crowd and touches it (rule 3).
        let look = look_round(unit.radius, speed, dt);
        let obstacles = neighbours_in_sum_order(&snapshot, &start, row, look, |body| {
            if body.owner == unit.owner_slot {
                body.fixed
                    || state
                        .members
                        .get(&units[body.row].unit_id)
                        .is_none_or(|other| other.mode != Mode::Moving && other.goal != member.goal)
            } else {
                basis.is_none_or(|basis| {
                    basis.unit_permitted(unit.owner_slot, units[body.row].unit_id)
                })
            }
        });
        let mut avoid = Vec2::ZERO;
        // The part of `avoid` from bodies that can move: a waiting unit side-steps only
        // these (a body that never moves keeps the way shut on each side).
        let mut avoid_movable = Vec2::ZERO;
        for &other in obstacles.iter().take(NEIGHBOURS) {
            let body = &snapshot.bodies[other];
            let offset = body.position - unit.position;
            let ahead = heading.dot(offset);
            let side = heading.perp_dot(offset);
            let lane = unit.radius + body.radius + 0.5 * unit.radius;
            let gap = offset.length() - unit.radius - body.radius;
            if ahead > 0.0 && side != 0.0 && side.abs() < lane && gap < look {
                let force = heading.perp()
                    * side.signum()
                    * (1.0 - side.abs() / lane)
                    * (1.0 - (gap / look).clamp(0.0, 1.0));
                avoid -= force;
                if !body.fixed {
                    avoid_movable -= force;
                }
            }
        }
        let mut wish = heading * speed + separation * speed * 0.6 + avoid * speed * 1.5;
        if flockmates > 0.0 {
            wish += (alignment / flockmates - member.velocity) * 0.15;
            wish += (centroid / flockmates - unit.position).clamp_length_max(1.0) * speed * 0.05;
        }
        // A waiting unit holds its place, and gives way to the flow.
        yielding[row] |= queue_hold;
        advice[row] = if let Some(give_passage) = give_passage {
            give_passage
        } else if queue_hold {
            Vec2::ZERO
        } else if member.mode == Mode::Waiting {
            // A unit that bodies stop steps out of the line of the bodies ahead (V2).
            let aside = if member.blocked {
                avoid_movable
            } else {
                Vec2::ZERO
            };
            ((give_way + aside + if kept_out[row] { heading } else { Vec2::ZERO }) * speed)
                .clamp_length_max(speed)
        } else if unit.position.distance(member.goal) <= speed * dt {
            // The last step goes exactly to the point.
            last_step[row] = Some(member.goal);
            (member.goal - unit.position) / dt
        } else if exact_node && keep_out.is_none() {
            // Beside terrain the unit goes from node to node, exactly: a body that just
            // fits an opening must be on its centre line.
            if unit.position.distance(toward) <= speed * dt {
                last_step[row] = Some(toward);
                (toward - unit.position) / dt
            } else {
                last_step[row] = Some(unit.position + heading * speed * dt);
                heading * speed
            }
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
        let mut end = guard(row, from, last_step[row].unwrap_or(from + wish));
        // A waiting unit takes its step to the side whole, or not at all, and does not turn:
        // so it stands still when the side is not free.
        if last_step[row].is_none()
            && state
                .members
                .get(&unit.unit_id)
                .is_some_and(|member| member.mode == Mode::Waiting)
        {
            positions[row] = if end == from + wish { end } else { from };
            continue;
        }
        // An exact step to a node or to the point, where the body has no clearance.
        if let (Some(node), Some(profile)) = (last_step[row], profiles[row])
            && end != node
        {
            let solids = solids_for(row, from, from.distance(node) + unit.radius);
            end = fitted_step(terrain, from, node, unit.radius, profile, &solids);
        }
        // A unit with no path goes straight to the obstacle and stops there (rule 5); it
        // does not slide along it.
        if end.distance(from) < 0.5 * wish.length() && !no_path[row] {
            let aim = aims[row];
            let mut best = (end.distance(aim), end);
            for degrees in [30.0f32, -30.0, 60.0, -60.0, 90.0, -90.0] {
                let turned = Vec2::from_angle(degrees.to_radians()).rotate(wish);
                let candidate = guard(row, from, from + turned);
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
                let solids = solids_for(row, from, wish.length() + unit.radius);
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

    // Stage C, part 1b: an idle unit that an own push moved goes back to its place (V6), when
    // no own mover was near for `RETURN_DELAY` ticks and no own body stands on the place.
    // A refused or shortened step ends the return: the unit stays where it is. Only own
    // units decide a return, so a return shows nothing of another owner. All returns read
    // the poses before the returns, so the order of the units decides nothing.
    let stepped = positions.clone();
    for (row, unit) in units.iter().enumerate() {
        let (Some(speed), Some(home)) = (unit.speed, state.homes.get(&unit.unit_id).copied())
        else {
            continue;
        };
        if holds[row] {
            continue;
        }
        let reach = look_round(unit.radius, speed, dt);
        let mut mover_near = false;
        snapshot.near(unit.position, reach + max_radius, |other| {
            let body = &snapshot.bodies[other];
            mover_near |= other != row
                && body.owner == unit.owner_slot
                && unit.position.distance(body.position) - unit.radius - body.radius < reach
                && state
                    .members
                    .get(&units[body.row].unit_id)
                    .is_some_and(|other| other.mode == Mode::Moving);
        });
        // An own body that the unit would overlap at `at`.
        let taken = |at: Vec2| {
            let mut taken = false;
            snapshot.near(at, unit.radius + max_radius, |other| {
                let body = &snapshot.bodies[other];
                taken |= other != row
                    && body.owner == unit.owner_slot
                    && at.distance(stepped[other]) < unit.radius + body.radius;
            });
            taken
        };
        let mut place = if mover_near {
            Home {
                quiet: 0,
                returning: false,
                ..home
            }
        } else {
            Home {
                quiet: home.quiet.saturating_add(1),
                ..home
            }
        };
        // When the area becomes quiet, a shift of less than one radius (a crowd that settles)
        // is not a push away: the new pose is the place. A larger shift starts the return.
        if !place.returning && place.quiet == RETURN_DELAY {
            if unit.position.distance(home.pose) < unit.radius {
                place.pose = unit.position;
            } else {
                place.returning = true;
            }
        } else if !place.returning && place.quiet > RETURN_DELAY {
            place.pose = unit.position;
        }
        if place.returning {
            let to = unit.position + (home.pose - unit.position).clamp_length_max(speed * dt);
            let end = if taken(home.pose) || taken(to) {
                unit.position
            } else {
                guard(row, unit.position, to)
            };
            if end != to || end.distance(home.pose) <= f32::EPSILON * home.pose.length() {
                place = Home {
                    pose: end,
                    returning: false,
                    ..place
                };
            }
            positions[row] = end;
        }
        state.homes.insert(unit.unit_id, place);
    }
    // Two returns that end in overlap both stay: no unit wins (as for two owners, E1).
    let returned: Vec<usize> = (0..units.len())
        .filter(|&row| positions[row] != stepped[row])
        .collect();
    let clash: Vec<usize> = returned
        .iter()
        .copied()
        .filter(|&row| {
            returned.iter().any(|&other| {
                other != row
                    && units[other].owner_slot == units[row].owner_slot
                    && positions[row].distance(positions[other])
                        < units[row].radius + units[other].radius
            })
        })
        .collect();
    for row in clash {
        positions[row] = stepped[row];
    }

    // Stage C, part 2: own bodies relax overlaps. Each pass reads the positions of the
    // pass start and applies all corrections together. A moving unit pushes an idle one
    // aside (rule 1).
    let soft = |row: usize| units[row].speed.is_some() && !holds[row] && profiles[row].is_some();
    let moves = |row: usize| {
        state
            .members
            .get(&units[row].unit_id)
            .is_some_and(|member| member.mode == Mode::Moving)
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
            // Own units, and own units that stand firm (Hold, busy): a firm unit is not
            // pushed, but it can step into others, and they make room for it.
            let neighbours = neighbours_in_sum_order(&relaxed, &before, row, REST_GAP, |body| {
                body.owner == units[row].owner_slot && (!body.fixed || holds[body.row])
            });
            let mut push = Vec2::ZERO;
            for other in neighbours {
                // Two units at one pose (both took their last step to one point in one tick)
                // come apart along the line between their poses at the start of the tick.
                let offset = if before[row] == before[other] {
                    (start[row] - start[other]).normalize_or_zero() * 1.0e-3
                } else {
                    before[row] - before[other]
                };
                let distance = offset.length();
                if distance == 0.0 {
                    continue;
                }
                // The relaxation aims at a small gap, so standing bodies end apart and the
                // float residue of the corrections is not an overlap.
                // Two units that the flock never moved keep an exact contact.
                let known = |row: usize| state.members.contains_key(&units[row].unit_id);
                let gap = if known(row) || known(other) {
                    REST_GAP
                } else {
                    0.0
                };
                let overlap = units[row].radius + units[other].radius + gap - distance;
                if overlap <= 0.0 {
                    continue;
                }
                // Two standing units each take the whole overlap, so a packed crowd comes
                // apart in a few ticks. A mover shares an overlap equally with each own unit,
                // so it pushes its way through, and a crowd keeps its place (rule 2).
                let share = if holds[other] || !(moves(row) || moves(other)) {
                    1.0
                } else {
                    0.5
                };
                push += offset / distance * overlap * share;
            }
            if push != Vec2::ZERO {
                let push = push.clamp_length_max(units[row].radius);
                // A push that a wall or a solid body shortens turns to the side, so a unit
                // is not pinned between the pusher and the obstacle. The push that goes
                // farthest wins; equal lengths keep the earlier, smaller turn.
                let mut best = guard(row, before[row], before[row] + push);
                for degrees in [60.0f32, -60.0, 90.0, -90.0] {
                    if best == before[row] + push {
                        break;
                    }
                    let turned = Vec2::from_angle(degrees.to_radians()).rotate(push);
                    let end = guard(row, before[row], before[row] + turned);
                    if end.distance(before[row]) > best.distance(before[row]) {
                        best = end;
                    }
                }
                positions[row] = best;
            }
        }
        // A pass with no correction ends the relaxation.
        if positions == before {
            break;
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
            positions[row] = guard(row, start[row], end);
            // A start pose at a terrain contact refuses every guarded chord. The chord is
            // then checked as an escape step, so a unit that left the contact keeps its move.
            if positions[row] != end
                && last_step[row] == Some(end)
                && let Some(profile) = profiles[row]
            {
                // The exact step of a body with no clearance, checked as a whole.
                let solids = solids_for(row, start[row], chord.length() + units[row].radius);
                positions[row] = fitted_step(
                    terrain,
                    start[row],
                    end,
                    units[row].radius,
                    profile,
                    &solids,
                );
            } else if positions[row] == start[row]
                && let Some(profile) = profiles[row]
            {
                let solids = solids_for(row, start[row], chord.length() + units[row].radius);
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
    // the start pose of the other. When two end poses overlap, both units take the same
    // smaller part of their step (a half, a quarter, an eighth), and with no clear part
    // both keep their start pose: contact stops the step, and no unit wins. A part of a
    // step is on the checked line of the step, so it needs no new terrain check.
    // Two own units that the push could not part (terrain holds one of them) do the same
    // when their overlap is deeper than `MOVING_OVERLAP` (rule 4).
    let ends = positions.clone();
    // The units whose step comes nearer to a body that it may not overlap (so deep).
    let conflicts = |positions: &[Vec2]| -> Vec<bool> {
        let moved = Snapshot::new(
            units
                .iter()
                .enumerate()
                .map(|(row, unit)| Body {
                    row,
                    owner: unit.owner_slot,
                    position: positions[row],
                    radius: unit.radius,
                    // A firm unit that walks moves in this tick: own pairs with it keep the
                    // overlap limit of two moving own units.
                    fixed: unit.speed.is_none() || (holds[row] && !walks_firm[row]),
                })
                .collect(),
        );
        let mut conflict = vec![false; units.len()];
        for (row, unit) in units.iter().enumerate() {
            if positions[row] == start[row] {
                continue;
            }
            moved.near(positions[row], unit.radius + max_radius, |other| {
                let body = &moved.bodies[other];
                let limit = if body.fixed || body.owner != unit.owner_slot {
                    0.0
                } else {
                    MOVING_OVERLAP * unit.radius.min(body.radius)
                };
                // The nearest distance of the two bodies over the tick: both move on straight
                // lines in the same time (f64, as the swept check of a committed step).
                let from = (start[row] - start[other]).as_dvec2();
                let course =
                    ((positions[row] - start[row]) - (positions[other] - start[other])).as_dvec2();
                let part = if course.length_squared() > 0.0 {
                    (-from.dot(course) / course.length_squared()).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let distance = (from + course * part).length();
                if other != row
                    && distance < f64::from(unit.radius) + f64::from(body.radius) - f64::from(limit)
                    && distance < from.length()
                {
                    conflict[row] = true;
                }
            });
        }
        conflict
    };
    let mut stay = vec![false; units.len()];
    for part in [1.0f32, 0.5, 0.25, 0.125] {
        for row in 0..units.len() {
            if stay[row] {
                positions[row] = start[row] + (ends[row] - start[row]) * part;
            }
        }
        let conflict = conflicts(&positions);
        if !conflict.contains(&true) {
            stay.fill(false);
            break;
        }
        // A unit with no conflict keeps the part that it has now.
        stay = conflict;
    }
    // A unit that keeps its start pose can make a new conflict for a unit that kept a part:
    // that unit keeps its start pose also, until no conflict is left.
    while stay.contains(&true) {
        for row in 0..units.len() {
            if stay[row] {
                positions[row] = start[row];
            }
        }
        stay = conflicts(&positions);
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
    let mut blocked = vec![false; units.len()];
    let mut settle_reason = vec![""; units.len()];
    for (row, unit) in units.iter().enumerate() {
        let (Some(member), Some(speed)) = (state.members.get(&unit.unit_id), unit.speed) else {
            continue;
        };
        if member.mode == Mode::Settled || (member.mode == Mode::Waiting && !member.blocked) {
            continue;
        }
        let here = positions[row];
        let near = neighbours_in_sum_order(&settled_now, &positions, row, TOUCH, |body| {
            !body.fixed && body.owner == unit.owner_slot
        });
        let gap =
            |other: usize| here.distance(positions[other]) - unit.radius - units[other].radius;
        // A unit settles when no overlap is deeper than `SETTLE_OVERLAP`. The standing
        // units then relax the rest to zero (no overlap at rest).
        let overlaps = near.iter().any(|&other| {
            gap(other) < -(SETTLE_OVERLAP * unit.radius.min(units[other].radius) + NO_OVERLAP)
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
        // The same holds at a solid body that stands (a building, an own unit on Hold, a body
        // of another owner that does not move). A moving body of another owner is traffic.
        let mut touches_solid = false;
        // A body that stops the unit on its way (V2). The unit waits, with its order kept, at
        // a moving body of another owner (traffic), at an own unit that waits (a queue), and
        // in a passage or its mouth (V3) at each body that stops it, when it is far from the
        // goal (more than `NEAR_GOAL` goal areas). No timer ends the order. Elsewhere a
        // standing body ends the move where the unit stands, as before.
        let mut touches_stop = false;
        let mut touches_traffic = false;
        let mut touches_waiting = false;
        // A unit that a solid body stops can stand up to two steps from it.
        let reach = TOUCH + 2.0 * speed * dt;
        snapshot.near(here, unit.radius + max_radius + reach, |other| {
            let body = &snapshot.bodies[other];
            let stands = body.fixed
                || (body.owner != unit.owner_slot
                    && state
                        .members
                        .get(&units[body.row].unit_id)
                        .is_none_or(|other| other.mode != Mode::Moving));
            let touches = other != row
                && here.distance(positions[other]) - unit.radius - body.radius <= reach;
            touches_stop |= touches && stops(row, member.goal, other);
            touches_traffic |= touches && !stands && body.owner != unit.owner_slot;
            touches_waiting |= touches
                && !body.fixed
                && body.owner == unit.owner_slot
                && state
                    .members
                    .get(&units[body.row].unit_id)
                    .is_some_and(|other| other.mode == Mode::Waiting);
            if touches && stands {
                touches_solid = true;
            }
        });
        // Only a unit of a Move order stops at its crowd or in its goal area. A unit with a
        // point target goes on to the point and pushes a standing own unit off it (rule 1).
        let far = here.distance(member.goal) > NEAR_GOAL * area
            && field_of(row, member).is_some_and(|field| field.in_zone(here));
        let at_crowd = member.crowd
            && ((touches_crowd && inside)
                || (touches_standing
                    && member.stalled >= 2 * SETTLE_TICKS
                    && !no_path[row]
                    && !far));
        let at_solid = !no_path[row] && touches_solid && member.stalled >= 2 * SETTLE_TICKS && !far;
        let reason = touches_traffic || touches_waiting || (touches_stop && far);
        if member.mode == Mode::Waiting {
            // A wait ends when its reason ends.
            blocked[row] = reason;
            continue;
        }
        blocked[row] = !inside && !no_path[row] && member.stalled >= WAIT_TICKS && reason;
        let gave_up = gave_up && member.crowd;
        // A unit does not park in a passage, but the exact point of its order ends its move.
        settle[row] = (at_point || ((at_crowd || at_solid || gave_up) && !in_passage)) && !overlaps;
        settle_reason[row] = if at_point {
            "settled at the point"
        } else if touches_crowd {
            "settled at the crowd"
        } else {
            "settled in the goal area"
        };
    }

    let in_passage_now: Vec<bool> = units
        .iter()
        .enumerate()
        .map(|(row, unit)| {
            state.members.get(&unit.unit_id).is_some_and(|member| {
                field_of(row, member).is_some_and(|field| field.in_zone(positions[row]))
            })
        })
        .collect();
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
        if in_passage_now[row] && member.passage_since.is_none() {
            member.passage_entry = positions[row];
        }
        member.passage_since = in_passage_now[row].then(|| member.passage_since.unwrap_or(tick));
        moving += usize::from(positions[row] != start[row]);
        let distance = positions[row].distance(member.goal);
        // A unit that gives the passage to another owner is not stalled.
        if yielding[row] || distance < member.best_distance - 0.1 * unit.radius {
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
            Mode::Moving if no_path[row] && member.stalled >= WAIT_TICKS => {
                member.blocked = false;
                Mode::Waiting
            }
            Mode::Moving if blocked[row] => {
                member.blocked = true;
                Mode::Waiting
            }
            Mode::Waiting
                if (member.blocked && (way_free[row] || !blocked[row]))
                    || (!member.blocked && !no_path[row]) =>
            {
                member.blocked = false;
                member.stalled = 0;
                member.best_distance = distance;
                Mode::Moving
            }
            mode => mode,
        };
        // The wait marker carries no position or type of what stops the unit (GM6).
        if member.mode == Mode::Waiting || at_mouth[row] {
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
