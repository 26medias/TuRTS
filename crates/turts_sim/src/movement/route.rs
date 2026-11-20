//! Stateless bounded route search round stationary bodies of the mover's owner.
//!
//! The frame solver calls this only for a pool mover whose straight chord is blocked by a
//! stationary body of its own owner. The search runs again each tick from the frame alone, so
//! the economy-log replay gets the same answer with no private route state.
//!
//! - Obstacles are stationary bodies of the same owner, and resting bodies of another owner in
//!   contact range (`CONTACT_GAP`). A body of another owner that no body of the mover's owner
//!   touches never changes a route, so a hidden body cannot change one. Final certification
//!   still checks all bodies.
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
/// A flood search tries one direct chord only to a goal within this distance of the mover.
const DIRECT_REACH: f32 = 3.0;
pub(super) const NODE_CAP: usize = 2048;
/// The cap of a flood search for another slot. It covers a radius of about 12 units, enough to
/// find a free slot on the far side of a formation of some hundred bodies.
pub(super) const FLOOD_CAP: usize = 8192;
pub(super) const OWNER_CAP: usize = 65_536;
/// The largest closed region round a goal that `Planner::enclosed` proves. A pocket of a few
/// pool slots inside resting bodies has far fewer free nodes.
const ENCLOSED_CAP: usize = 256;
/// The straight look-ahead that decides whether a mover needs a route. This value is
/// empirical and the MC post-ring row is sensitive to it: 7.0 and 8.0 pass that row; 4.0, 6.0,
/// 10.0 and 12.0 miss its deadline by 10 to 70 ticks; 9.0 ends with Blocked units. A short
/// look-ahead lets a group walk up to an obstacle and jam in front of it. No argument shows
/// that 8.0 is right for another scene (see the review record of 2026-11-18).
pub(super) const LOOK_AHEAD: f32 = 8.0;

/// Contact range: the gap between two bodies at rest on adjacent pool slots, with a float
/// allowance. A route goes round a stationary body of another owner only while a body of the
/// mover's owner is this near to it at the start of the tick. The swept certificate already
/// stops a mover that steps at the same body, so a route gets no fact beyond physical contact.
/// The rule keeps no memory: a body that no body of the owner touches any more is unknown
/// again. `exchange::spend_discovery` bounds the steps that find such a body.
pub(super) const CONTACT_GAP: f32 = 0.1 + 1.0e-3;

type Node = (i32, i32);

/// True for a body that does not move in this tick by its own order.
pub(super) fn stationary(unit: &MovementFrameUnit) -> bool {
    unit.target.is_none() || !unit.speed.is_some_and(|speed| speed > 0.0)
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn exhausted_flood_does_not_return_a_partial_goal_set() {
        let units = [MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(5.0, 5.0),
            radius: 0.5,
            speed: Some(1.0),
            target: None,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }];
        let points = [crate::grid::GridPoint {
            id: 1,
            position: units[0].position,
        }];
        let grid = crate::grid::UniformGrid::new(&points, 1.0);
        let planner = Planner {
            index: 0,
            units: &units,
            points: &points,
            grid: &grid,
            max_radius: 0.5,
            bounds: Vec2::splat(20.0),
            stationary_at_start: &[false],
            contact_witness: &[vec![]],
            start_stall: &[None],
            soft: false,
            stall_ticks: 2,
        };
        let mut spent = 0;
        let mut exhausted = false;
        let reached = planner.search(
            &[Vec2::new(6.0, 5.0), Vec2::new(7.0, 5.0)],
            false,
            None,
            1,
            &mut spent,
            &mut exhausted,
        );
        assert!(exhausted);
        assert!(
            reached.is_empty(),
            "an unfinished flood must not choose from partial goals"
        );
    }

    /// One mover at (5, 10) and stationary bodies of its owner at `bodies`.
    fn ring_scene(bodies: &[Vec2]) -> (Vec<MovementFrameUnit>, Vec<crate::grid::GridPoint>) {
        let body = |unit_id: u32, position: Vec2| MovementFrameUnit {
            unit_id,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(1.0),
            target: None,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        };
        let mut units = vec![body(1, Vec2::new(5.0, 10.0))];
        units.extend(
            bodies
                .iter()
                .enumerate()
                .map(|(index, &at)| body(2 + index as u32, at)),
        );
        let points = units
            .iter()
            .map(|unit| crate::grid::GridPoint {
                id: unit.unit_id,
                position: unit.position,
            })
            .collect();
        (units, points)
    }

    #[test]
    fn route_queries_ignore_foreign_noncontact_bodies_in_output_and_budget() {
        fn run(hidden: &[Vec2]) -> (bool, Vec<Reached>, usize, bool) {
            let (mut units, _) = ring_scene(&[Vec2::new(10.0, 10.0)]);
            units.extend(
                hidden
                    .iter()
                    .enumerate()
                    .map(|(i, &position)| MovementFrameUnit {
                        unit_id: 100 + i as u32,
                        owner_slot: 1,
                        unit_type: 1,
                        position,
                        radius: 0.01,
                        speed: None,
                        target: None,
                        progress: None,
                        plain_progress: None,
                        physical_escape: None,
                    }),
            );
            let points: Vec<_> = units
                .iter()
                .map(|u| crate::grid::GridPoint {
                    id: u.unit_id,
                    position: u.position,
                })
                .collect();
            let grid = crate::grid::UniformGrid::new(&points, 1.0);
            let stationary: Vec<_> = (0..units.len()).map(|i| i != 0).collect();
            let witness = vec![vec![]; units.len()];
            let stall = vec![None; units.len()];
            let planner = Planner {
                index: 0,
                units: &units,
                points: &points,
                grid: &grid,
                max_radius: 0.5,
                bounds: Vec2::splat(40.0),
                stationary_at_start: &stationary,
                contact_witness: &witness,
                start_stall: &stall,
                soft: false,
                stall_ticks: 2,
            };
            let chord = planner.chord_clear(Vec2::new(5.0, 10.0), Vec2::new(8.0, 10.0));
            let mut spent = 0;
            let mut exhausted = false;
            let reached = planner.search(
                &[Vec2::new(15.0, 10.0)],
                true,
                None,
                NODE_CAP,
                &mut spent,
                &mut exhausted,
            );
            (chord, reached, spent, exhausted)
        }
        let hidden = [Vec2::new(7.0, 10.4), Vec2::new(8.0, 10.4)];
        let mut reversed = hidden;
        reversed.reverse();
        assert_eq!(run(&[]), run(&hidden));
        assert_eq!(run(&[]), run(&reversed));
    }

    /// The six hex neighbours of `centre` at the pool spacing 1.1.
    fn hex_ring(centre: Vec2) -> Vec<Vec2> {
        let rise = 1.1 * 3.0_f32.sqrt() * 0.5;
        [
            (1.1, 0.0),
            (0.55, rise),
            (-0.55, rise),
            (-1.1, 0.0),
            (-0.55, -rise),
            (0.55, -rise),
        ]
        .into_iter()
        .map(|(x, y)| centre + Vec2::new(x, y))
        .collect()
    }

    fn toward(
        bodies: &[Vec2],
        goal: Vec2,
        cap: usize,
        spent: &mut usize,
        exhausted: &mut bool,
    ) -> Vec<Reached> {
        let (units, points) = ring_scene(bodies);
        let grid = crate::grid::UniformGrid::new(&points, 1.0);
        let stationary: Vec<bool> = (0..units.len()).map(|index| index != 0).collect();
        let witness = vec![vec![]; units.len()];
        let stall = vec![None; units.len()];
        let planner = Planner {
            index: 0,
            units: &units,
            points: &points,
            grid: &grid,
            max_radius: 0.5,
            bounds: Vec2::splat(40.0),
            stationary_at_start: &stationary,
            contact_witness: &witness,
            start_stall: &stall,
            soft: false,
            stall_ticks: 2,
        };
        planner.search(&[goal], true, None, cap, spent, exhausted)
    }

    /// A goal inside a closed ring of stationary bodies. The forward search floods the open
    /// ground to its cap; the closed region round the goal is the complete answer.
    #[test]
    fn a_goal_in_a_closed_ring_is_proven_unreachable_not_unknown() {
        let goal = Vec2::new(15.0, 10.0);
        let ring = hex_ring(goal);
        let mut spent = 0;
        let mut exhausted = false;
        let reached = toward(&ring, goal, NODE_CAP, &mut spent, &mut exhausted);
        assert!(reached.is_empty());
        assert!(!exhausted, "a closed ring is a complete answer");
        assert!(spent <= NODE_CAP + ENCLOSED_CAP, "bounded work: {spent}");
        // Two adjacent ring bodies away: the ring is open and the search finds the route.
        let mut spent = 0;
        let mut exhausted = false;
        let reached = toward(&ring[..4], goal, NODE_CAP, &mut spent, &mut exhausted);
        assert!(!exhausted);
        assert_eq!(reached.len(), 1, "the open ring has a route");
    }

    /// The proof must not turn an unfinished search on open ground into "no route".
    #[test]
    fn an_unfinished_search_toward_an_open_goal_stays_unknown() {
        // A goal with no body near it.
        let mut spent = 0;
        let mut exhausted = false;
        let wall: Vec<Vec2> = (0..40).map(|row| Vec2::new(10.0, row as f32)).collect();
        let reached = toward(&wall, Vec2::new(15.0, 10.0), 64, &mut spent, &mut exhausted);
        assert!(reached.is_empty());
        assert!(
            exhausted,
            "an open goal behind a long wall is unknown at this cap"
        );
        // A closed ring, but no room under the owner cap for the proof.
        let goal = Vec2::new(15.0, 10.0);
        let mut spent = OWNER_CAP - ENCLOSED_CAP + 1;
        let before = spent;
        let mut exhausted = false;
        let reached = toward(
            &hex_ring(goal),
            goal,
            before + 8,
            &mut spent,
            &mut exhausted,
        );
        assert!(reached.is_empty());
        assert!(exhausted, "no proof without budget: fail closed");
        assert!(spent <= OWNER_CAP);
    }

    /// The mover stands inside the closed region with the goal. A cap that ends the forward
    /// search early must leave the answer unknown: the closed region is no proof here.
    #[test]
    fn a_closed_region_that_holds_the_mover_is_no_proof() {
        let centre = Vec2::new(15.0, 10.0);
        let mut bodies: Vec<Vec2> = (0..19)
            .map(|step| {
                let angle = step as f32 * std::f32::consts::TAU / 19.0;
                centre + 3.0 * Vec2::new(angle.cos(), angle.sin())
            })
            .collect();
        // One body between the mover and the goal: no direct chord.
        bodies.push(centre);
        let (mut units, mut points) = ring_scene(&bodies);
        units[0].position = Vec2::new(13.5, 10.0);
        points[0].position = units[0].position;
        let goal = Vec2::new(16.5, 10.0);
        let grid = crate::grid::UniformGrid::new(&points, 1.0);
        let stationary: Vec<bool> = (0..units.len()).map(|index| index != 0).collect();
        let witness = vec![vec![]; units.len()];
        let stall = vec![None; units.len()];
        let planner = Planner {
            index: 0,
            units: &units,
            points: &points,
            grid: &grid,
            max_radius: 0.5,
            bounds: Vec2::splat(40.0),
            stationary_at_start: &stationary,
            contact_witness: &witness,
            start_stall: &stall,
            soft: false,
            stall_ticks: 2,
        };
        let mut spent = 0;
        let mut exhausted = false;
        let reached = planner.search(&[goal], true, None, 4, &mut spent, &mut exhausted);
        assert!(reached.is_empty());
        assert!(
            exhausted,
            "the mover is inside the region: unknown, not cut off"
        );
        let mut spent = 0;
        let mut exhausted = false;
        let reached = planner.search(&[goal], true, None, NODE_CAP, &mut spent, &mut exhausted);
        assert!(!exhausted);
        assert_eq!(reached.len(), 1, "the full search finds the route");
    }

    #[test]
    fn later_goal_connector_can_beat_an_ineligible_first_connector() {
        let start = Vec2::new(5.03, 5.07);
        let goal = Vec2::new(8.838_02, 6.864_918);
        let units = [MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position: start,
            radius: 0.5,
            speed: Some(1.0),
            target: None,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }];
        let points = [crate::grid::GridPoint {
            id: 1,
            position: start,
        }];
        let grid = crate::grid::UniformGrid::new(&points, 1.0);
        let planner = Planner {
            index: 0,
            units: &units,
            points: &points,
            grid: &grid,
            max_radius: 0.5,
            bounds: Vec2::splat(20.0),
            stationary_at_start: &[false],
            contact_witness: &[vec![]],
            start_stall: &[None],
            soft: false,
            stall_ticks: 2,
        };
        let mut spent = 0;
        let mut exhausted = false;
        // The first settled connector costs about 4.518. A later connector costs 4.482.
        let reached = planner.search(
            &[goal],
            false,
            Some(&[4.5]),
            8192,
            &mut spent,
            &mut exhausted,
        );
        assert!(!exhausted);
        assert!(
            reached.iter().any(|r| r.cost < 4.5),
            "later eligible connector was lost: {reached:?}"
        );
    }
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
    /// For each stationary row, the other owners with a body in contact range of it at the
    /// start of the tick (sorted).
    pub(super) contact_witness: &'a [Vec<u8>],
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
        if other_index == self.index {
            return false;
        }
        if other.owner_slot != unit.owner_slot {
            // A body of another owner: only at rest, and only in contact range of the owner.
            return self.stationary_at_start[other_index]
                && self.contact_witness[other_index]
                    .binary_search(&unit.owner_slot)
                    .is_ok();
        }
        self.stationary_at_start[other_index]
            || (self.soft
                && self.start_stall[other_index].is_some_and(|clock| clock >= self.stall_ticks))
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
        !self
            .grid
            .any_within(self.points, middle, reach, |other_index| {
                let other = &self.units[other_index];
                self.obstacle(other_index)
                    && !super::sweep::pair_clear(
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

    /// True when the lattice nodes that connect to `goal` form a closed region of at most
    /// `ENCLOSED_CAP` nodes and no first node of the mover at `start` is in it. The forward
    /// search uses the same nodes and the same steps, so it can never reach such a goal: an
    /// obstacle ring round a slot makes the forward search flood the open ground to its cap,
    /// and this check gives the answer that the flood cannot give. The work is bounded by
    /// `ENCLOSED_CAP` and counts against `spent`; the check does not run when the owner cap
    /// has no room for it.
    fn enclosed(
        &self,
        goal: Vec2,
        start: Vec2,
        free: &mut HashMap<Node, bool>,
        spent: &mut usize,
    ) -> bool {
        if *spent + ENCLOSED_CAP > OWNER_CAP {
            return false;
        }
        let mut region: HashMap<Node, ()> = HashMap::new();
        let mut queue: Vec<Node> = Vec::new();
        for node in Self::nodes_near(goal) {
            if *free.entry(node).or_insert_with(|| self.node_free(node))
                && self.chord_clear(Self::point(node), goal)
            {
                region.insert(node, ());
                queue.push(node);
            }
        }
        while let Some(node) = queue.pop() {
            *spent += 1;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let next = (node.0 + dx, node.1 + dy);
                    if region.contains_key(&next)
                        || !*free.entry(next).or_insert_with(|| self.node_free(next))
                    {
                        continue;
                    }
                    if region.len() >= ENCLOSED_CAP {
                        return false;
                    }
                    region.insert(next, ());
                    queue.push(next);
                }
            }
        }
        !Self::nodes_near(start).any(|node| region.contains_key(&node))
    }

    /// Search from the mover to `goals`. `toward` is the single goal of a directed search; then
    /// the search stops when it reaches that goal. Without it the search floods and returns each
    /// goal that it reaches. `spent` counts each settled node and each direct chord against
    /// `cap`. `exhausted` becomes true when the cap ends the search: then a goal that the
    /// search did not reach is unknown, not unreachable.
    pub(super) fn search(
        &self,
        goals: &[Vec2],
        toward: bool,
        eligible_limits: Option<&[f32]>,
        cap: usize,
        spent: &mut usize,
        exhausted: &mut bool,
    ) -> Vec<Reached> {
        if *exhausted {
            return Vec::new();
        }
        let start = self.units[self.index].position;
        // A goal that one clear chord reaches needs no lattice node. A body inside a ring of
        // resting neighbours has no free node near it, but can still step onto its slot. A
        // flood tries this only for the goals beside the mover.
        let mut direct: Vec<Reached> = Vec::new();
        for (goal, point) in goals.iter().enumerate() {
            let distance = start.distance(*point);
            if !toward && distance > DIRECT_REACH {
                continue;
            }
            if *spent >= cap {
                *exhausted = true;
                return Vec::new();
            }
            *spent += 1;
            if self.chord_clear(start, *point) {
                direct.push(Reached {
                    goal,
                    cost: distance,
                    waypoint: *point,
                });
            }
        }
        if toward && !direct.is_empty() {
            return direct;
        }
        let eligible =
            |goal: usize, cost: f32| eligible_limits.is_some_and(|limits| cost < limits[goal]);
        let mut best_goal = direct
            .iter()
            .filter(|r| eligible(r.goal, r.cost))
            .map(|r| r.cost)
            .fold(f32::INFINITY, f32::min);
        let maximum_limit = eligible_limits.map_or(f32::INFINITY, |limits| {
            limits.iter().copied().fold(0.0, f32::max)
        });
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
            // No unsettled path can beat an eligible route once its prefix exceeds that
            // route's total cost. Include equal costs for geometric tie breaking.
            if !toward && (cost > best_goal || cost >= maximum_limit) {
                break;
            }
            if *spent >= cap {
                *exhausted = true;
                break;
            }
            *spent += 1;
            let point = Self::point(node);
            if let Some(goal_indexes) = attached.get(&node) {
                for &goal_index in goal_indexes {
                    let total = cost + point.distance(goals[goal_index]);
                    if reached[goal_index].is_none_or(|(previous, _)| total < previous)
                        && self.chord_clear(point, goals[goal_index])
                    {
                        reached[goal_index] = Some((total, node));
                        if eligible(goal_index, total) {
                            best_goal = best_goal.min(total);
                        }
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
        if *exhausted && toward && self.enclosed(goals[0], start, &mut free, spent) {
            // The cap ended the forward search, but the goal side is a closed region that
            // the mover is not in. That is a complete answer: no route exists.
            *exhausted = false;
            return Vec::new();
        }
        if *exhausted {
            return Vec::new();
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
