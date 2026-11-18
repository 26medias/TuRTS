//! Stateless bounded route search round stationary bodies of the mover's owner.
//!
//! The frame solver calls this only for a pool mover whose straight chord is blocked by a
//! stationary body of its own owner. The search runs again each tick from the frame alone, so
//! the economy-log replay gets the same answer with no private route state.
//!
//! - Obstacles are stationary bodies of the same owner. A body of another owner never changes
//!   a route, so a hidden body cannot change one. Final certification still checks all bodies.
//! - The graph is a square lattice with a fixed world origin. A node is free when its centre
//!   keeps `MARGIN` beyond contact from every obstacle, so an edge between two free nodes
//!   cannot cut an obstacle. The first and the last leg get the exact swept check.
//! - Work is bounded: `NODE_CAP` or `FLOOD_CAP` settled nodes for each search, and `OWNER_CAP` for each owner
//!   in one tick. On exhaustion the search gives no route (fail closed).
//! - Every order is geometric: cost, then node coordinates. No unit id.

use bevy_math::Vec2;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use super::frame::MovementFrameUnit;

const PITCH: f32 = 0.25;
/// Sagitta of a `PITCH * sqrt(2)` chord on a contact circle of radius 0.6 or more, rounded up.
const MARGIN: f32 = 0.03;
pub(super) const NODE_CAP: usize = 2048;
/// The cap of a flood search for another slot. It covers a radius of about 12 units, enough to
/// find a free slot on the far side of a formation of some hundred bodies.
pub(super) const FLOOD_CAP: usize = 8192;
pub(super) const OWNER_CAP: usize = 65_536;
/// The straight look-ahead that decides whether a mover needs a route.
pub(super) const LOOK_AHEAD: f32 = 4.0;

type Node = (i32, i32);

/// True for a body that does not move in this tick by its own order.
pub(super) fn stationary(unit: &MovementFrameUnit) -> bool {
    unit.target.is_none() || !unit.speed.is_some_and(|speed| speed > 0.0)
}

#[derive(Clone, Copy)]
pub(super) struct Planner<'a> {
    pub(super) index: usize,
    pub(super) units: &'a [MovementFrameUnit],
    pub(super) points: &'a [crate::grid::GridPoint],
    pub(super) grid: &'a crate::grid::UniformGrid,
    pub(super) max_radius: f32,
    pub(super) bounds: Vec2,
    /// Start-of-tick facts for each row: `stationary`, and the certified stall clock of a
    /// mover. The search reads these, not the rows that the solver has already stepped, so the
    /// processing order inside a tick cannot change an obstacle.
    pub(super) stationary_at_start: &'a [bool],
    pub(super) start_stall: &'a [Option<u16>],
    /// Also treat a mover of the same owner as an obstacle while its certified stall clock is
    /// at `stall_ticks` or more. Two movers that press on each other then route round each other.
    pub(super) soft: bool,
    pub(super) stall_ticks: u16,
}

/// One reached goal: the path cost and the next point to steer at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Reached {
    pub(super) goal: usize,
    pub(super) cost: f32,
    pub(super) waypoint: Vec2,
}

impl Planner<'_> {
    fn obstacle(&self, other_index: usize) -> bool {
        let unit = &self.units[self.index];
        let other = &self.units[other_index];
        other_index != self.index
            && other.owner_slot == unit.owner_slot
            && (self.stationary_at_start[other_index]
                || (self.soft
                    && self.start_stall[other_index]
                        .is_some_and(|clock| clock >= self.stall_ticks)))
    }

    /// The same search with true stationary bodies only.
    pub(super) fn hard(&self) -> Self {
        Self {
            soft: false,
            ..*self
        }
    }

    /// Exact: no obstacle touches the body swept from `from` to `to`.
    pub(super) fn chord_clear(&self, from: Vec2, to: Vec2) -> bool {
        let unit = &self.units[self.index];
        let middle = (from + to) * 0.5;
        let reach = from.distance(to) * 0.5 + unit.radius + self.max_radius;
        self.grid
            .within_unsorted(self.points, middle, reach)
            .into_iter()
            .all(|other_index| {
                let other = &self.units[other_index];
                !self.obstacle(other_index)
                    || super::sweep::pair_clear(
                        from,
                        to,
                        unit.radius,
                        other.position,
                        other.position,
                        other.radius,
                    )
            })
    }

    /// True when no other body of the owner, moving or not, overlaps the mover's body at
    /// `point`.
    pub(super) fn uncovered(&self, point: Vec2) -> bool {
        let unit = &self.units[self.index];
        self.grid
            .within_unsorted(self.points, point, unit.radius + self.max_radius)
            .into_iter()
            .all(|other_index| {
                let other = &self.units[other_index];
                other_index == self.index
                    || other.owner_slot != unit.owner_slot
                    || point.distance(other.position) >= unit.radius + other.radius - 1.0e-5
            })
    }

    /// True when a body of the mover's size at `site` would lie outside the map or deep in a
    /// stationary body of the owner: deeper than a neighbour slot of the pool lattice lies.
    pub(super) fn site_closed(&self, site: Vec2, spacing: f32) -> bool {
        let unit = &self.units[self.index];
        if site.x < unit.radius - 1.0e-3
            || site.y < unit.radius - 1.0e-3
            || site.x > self.bounds.x - unit.radius + 1.0e-3
            || site.y > self.bounds.y - unit.radius + 1.0e-3
        {
            return true;
        }
        let slack = (spacing - 2.0 * unit.radius).max(0.0);
        self.grid
            .within_unsorted(self.points, site, unit.radius + self.max_radius)
            .into_iter()
            .any(|other_index| {
                let other = &self.units[other_index];
                self.obstacle(other_index)
                    && site.distance(other.position) < unit.radius + other.radius - slack
            })
    }

    fn point(node: Node) -> Vec2 {
        Vec2::new(node.0 as f32 * PITCH, node.1 as f32 * PITCH)
    }

    fn node_free(&self, node: Node) -> bool {
        let unit = &self.units[self.index];
        let point = Self::point(node);
        if point.x < unit.radius
            || point.y < unit.radius
            || point.x > self.bounds.x - unit.radius
            || point.y > self.bounds.y - unit.radius
        {
            return false;
        }
        self.grid
            .within_unsorted(self.points, point, unit.radius + self.max_radius + MARGIN)
            .into_iter()
            .all(|other_index| {
                let other = &self.units[other_index];
                !self.obstacle(other_index)
                    || point.distance(other.position) >= unit.radius + other.radius + MARGIN
            })
    }

    fn nodes_near(point: Vec2) -> impl Iterator<Item = Node> {
        let x = (point.x / PITCH).floor() as i32;
        let y = (point.y / PITCH).floor() as i32;
        (-1..=2).flat_map(move |dx| (-1..=2).map(move |dy| (x + dx, y + dy)))
    }

    /// Search from the mover to `goals`. `toward` is the single goal of a directed search; then
    /// the search stops when it reaches that goal. Without it the search floods and returns each
    /// goal that it reaches. `spent` counts settled nodes against `cap`.
    pub(super) fn search(
        &self,
        goals: &[Vec2],
        toward: bool,
        cap: usize,
        spent: &mut usize,
    ) -> Vec<Reached> {
        let start = self.units[self.index].position;
        // A goal that one clear chord reaches needs no lattice node. A body inside a ring of
        // resting neighbours has no free node near it, but can still step onto its slot.
        let direct: Vec<Reached> = goals
            .iter()
            .enumerate()
            .filter(|(_, goal)| self.chord_clear(start, **goal))
            .map(|(goal, point)| Reached {
                goal,
                cost: start.distance(*point),
                waypoint: *point,
            })
            .collect();
        if toward && !direct.is_empty() {
            return direct;
        }
        let mut attached: HashMap<Node, Vec<usize>> = HashMap::new();
        for (goal_index, goal) in goals.iter().enumerate() {
            for node in Self::nodes_near(*goal) {
                attached.entry(node).or_default().push(goal_index);
            }
        }
        let heuristic = |point: Vec2| {
            if toward {
                point.distance(goals[0])
            } else {
                0.0
            }
        };
        let mut free: HashMap<Node, bool> = HashMap::new();
        let mut best: HashMap<Node, (f32, Option<Node>)> = HashMap::new();
        let mut settled: HashMap<Node, ()> = HashMap::new();
        // Positive finite f32 values order as their bit patterns.
        let mut open: BinaryHeap<Reverse<(u32, u32, Node)>> = BinaryHeap::new();
        for node in Self::nodes_near(start) {
            let point = Self::point(node);
            if *free.entry(node).or_insert_with(|| self.node_free(node))
                && self.chord_clear(start, point)
            {
                let cost = start.distance(point);
                best.insert(node, (cost, None));
                open.push(Reverse((
                    (cost + heuristic(point)).to_bits(),
                    cost.to_bits(),
                    node,
                )));
            }
        }
        let mut reached: Vec<Option<(f32, Node)>> = vec![None; goals.len()];
        'search: while let Some(Reverse((_, cost_bits, node))) = open.pop() {
            let cost = f32::from_bits(cost_bits);
            if best.get(&node).is_none_or(|(known, _)| *known != cost)
                || settled.insert(node, ()).is_some()
            {
                continue;
            }
            if *spent >= cap {
                break;
            }
            *spent += 1;
            let point = Self::point(node);
            if let Some(goal_indexes) = attached.get(&node) {
                for &goal_index in goal_indexes {
                    if reached[goal_index].is_none() && self.chord_clear(point, goals[goal_index]) {
                        reached[goal_index] =
                            Some((cost + point.distance(goals[goal_index]), node));
                        if toward {
                            break 'search;
                        }
                    }
                }
            }
            for (dx, dy) in [
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, -1),
                (0, 1),
                (1, -1),
                (1, 0),
                (1, 1),
            ] {
                let next = (node.0 + dx, node.1 + dy);
                if settled.contains_key(&next)
                    || !*free.entry(next).or_insert_with(|| self.node_free(next))
                {
                    continue;
                }
                let next_point = Self::point(next);
                let next_cost = cost + point.distance(next_point);
                if best.get(&next).is_none_or(|(known, _)| next_cost < *known) {
                    best.insert(next, (next_cost, Some(node)));
                    open.push(Reverse((
                        (next_cost + heuristic(next_point)).to_bits(),
                        next_cost.to_bits(),
                        next,
                    )));
                }
            }
        }
        reached
            .into_iter()
            .enumerate()
            .filter_map(|(goal, row)| {
                if let Some(direct) = direct.iter().find(|direct| direct.goal == goal) {
                    return Some(*direct);
                }
                let (cost, last) = row?;
                let mut path = vec![goals[goal], Self::point(last)];
                let mut node = last;
                while let Some((_, Some(parent))) = best.get(&node) {
                    node = *parent;
                    path.push(Self::point(node));
                }
                // `path` runs from the goal back to the first node. Steer at the farthest
                // point that the body reaches by one exact clear chord. The first node passed
                // that check when the search began.
                let first = *path.last()?;
                let count = path.len();
                let head = count.min(8);
                let waypoint = (0..head)
                    .chain(count.saturating_sub(24).max(head)..count)
                    .map(|rank| path[rank])
                    .find(|point| self.chord_clear(start, *point))
                    .unwrap_or(first);
                Some(Reached {
                    goal,
                    cost,
                    waypoint,
                })
            })
            .collect()
    }
}
