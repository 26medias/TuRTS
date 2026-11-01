//! Movement integrator.

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_math::Vec2;
use bevy_time::{Fixed, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::registry::UnitTypes;
use crate::world::{MoveTarget, Position, UnitType};

#[cfg(test)]
thread_local! {
    static POOL_SLOT_CANDIDATE_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn count_pool_slot_candidates(count: usize) {
    POOL_SLOT_CANDIDATE_CHECKS.with(|checks| checks.set(checks.get() + count));
}

/// Current lifecycle of one accepted group-movement envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementActivity {
    Pending,
    Moving,
    Resting,
    Blocked,
}

/// Per-unit deterministic movement state needed by arrival and replay.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveProgress {
    pub pool_id: u64,
    pub slot: Vec2,
    pub activity: MovementActivity,
    pub pass_mode: bool,
    pub stalling_on_group: bool,
    pub stalled_ticks: u16,
    pub best_distance: f32,
    pub claim_count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_origin: Option<FollowMoveOrigin>,
}

/// The live Unit target and per-actor legal goal used for initial spread.
/// This keeps a shared max-radius pool anchor from looking like target motion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FollowMoveOrigin {
    pub target_unit_id: u32,
    pub goal: Vec2,
}

/// Default-omitted progress for a single legacy move that has encountered an obstacle.
/// The candidate pool belongs to this unit; it deliberately has no shared pool ID.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainMoveProgress {
    pub original_point: Vec2,
    pub slot: Vec2,
    pub pool_slots: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_goal: Option<PlainGatherGoal>,
    pub activity: MovementActivity,
    pub stalled_ticks: u16,
    pub best_distance: f32,
    pub claim_count: u8,
    pub failure_recorded: bool,
}

/// Physical recovery that continues after an order has already failed.
/// Unlike movement progress, this state belongs to the unit body, not its order.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalEscape {
    pub target: Option<Vec2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlainGatherGoal {
    Node(u32),
    Receiver(u32),
}

/// One group's primary destinations and spare destinations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovementPool {
    pub pool_id: u64,
    pub point: Vec2,
    pub spacing: f32,
    pub primary_count: u32,
    pub slots: Vec<Vec2>,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MovementPools(pub BTreeMap<u64, MovementPool>);

#[derive(Debug, Clone, Copy)]
pub struct SpreadMember {
    pub id: u32,
    pub position: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct StaticFootprint {
    pub position: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ReservedFootprint {
    pub position: Vec2,
    pub radius: f32,
}

/// Phase-local obstacle indexes reused while dispatching a batch of envelopes.
pub struct SpreadContext {
    bounds: Vec2,
    reserved: Vec<ReservedFootprint>,
    reserved_units: Vec<Option<u32>>,
    reserved_points: Vec<crate::grid::GridPoint>,
    reserved_grid: crate::grid::UniformGrid,
    max_reserved_radius: f32,
    static_footprints: Vec<StaticFootprint>,
    static_points: Vec<crate::grid::GridPoint>,
    static_grid: crate::grid::UniformGrid,
    max_static_radius: f32,
    reserved_index: std::collections::HashMap<u32, usize>,
}

impl SpreadContext {
    pub fn new(
        reserved: Vec<(Option<u32>, ReservedFootprint)>,
        static_footprints: Vec<StaticFootprint>,
        bounds: Vec2,
        max_unit_radius: f32,
    ) -> Self {
        let cell_size = (2.0 * max_unit_radius + 0.1).max(0.1);
        let reserved_units: Vec<_> = reserved.iter().map(|(id, _)| *id).collect();
        let reserved: Vec<_> = reserved
            .into_iter()
            .map(|(_, footprint)| footprint)
            .collect();
        let reserved_points: Vec<_> = reserved
            .iter()
            .enumerate()
            .map(|(index, footprint)| crate::grid::GridPoint {
                id: reserved_units[index].unwrap_or(index as u32),
                position: footprint.position,
            })
            .collect();
        let static_points: Vec<_> = static_footprints
            .iter()
            .enumerate()
            .map(|(id, footprint)| crate::grid::GridPoint {
                id: id as u32,
                position: footprint.position,
            })
            .collect();
        let max_reserved_radius = reserved
            .iter()
            .map(|item| item.radius)
            .fold(0.0_f32, f32::max);
        let max_static_radius = static_footprints
            .iter()
            .map(|item| item.radius)
            .fold(0.0_f32, f32::max);
        let reserved_index = reserved_units
            .iter()
            .enumerate()
            .filter_map(|(index, unit)| unit.map(|unit| (unit, index)))
            .collect();
        Self {
            bounds,
            reserved_grid: crate::grid::UniformGrid::new(&reserved_points, cell_size),
            static_grid: crate::grid::UniformGrid::new(&static_points, cell_size),
            reserved,
            reserved_units,
            reserved_points,
            max_reserved_radius,
            static_footprints,
            static_points,
            max_static_radius,
            reserved_index,
        }
    }

    pub fn reserve_unit(&mut self, unit_id: u32, footprint: ReservedFootprint) {
        if let Some(index) = self.reserved_index.get(&unit_id).copied() {
            let old = self.reserved[index].position;
            self.reserved[index] = footprint;
            self.reserved_points[index].position = footprint.position;
            self.reserved_grid
                .move_point(index, old, footprint.position);
        } else {
            let index = self.reserved.len();
            self.reserved.push(footprint);
            self.reserved_units.push(Some(unit_id));
            self.reserved_points.push(crate::grid::GridPoint {
                id: unit_id,
                position: footprint.position,
            });
            self.reserved_grid.insert(index, footprint.position);
            self.reserved_index.insert(unit_id, index);
            self.max_reserved_radius = self.max_reserved_radius.max(footprint.radius);
        }
    }

    fn conflicts_with_reserved(&self, candidate: Vec2, radius: f32, excluded_unit: u32) -> bool {
        self.reserved_grid
            .within(
                &self.reserved_points,
                candidate,
                radius + self.max_reserved_radius + 0.1,
            )
            .into_iter()
            .any(|index| {
                self.reserved_units[index] != Some(excluded_unit)
                    && candidate.distance(self.reserved[index].position)
                        < radius + self.reserved[index].radius + 0.1 - 1.0e-5
            })
    }
}

/// Build a deterministic, order-preserving hex pool and assignment for an envelope.
pub fn spread_destinations(
    members: &[SpreadMember],
    point: Vec2,
    reserved_cells: &[ReservedFootprint],
    static_footprints: &[StaticFootprint],
    bounds: Vec2,
) -> (MovementPool, Vec<(u32, Vec2)>) {
    let max_radius = members
        .iter()
        .map(|unit| unit.radius)
        .chain(reserved_cells.iter().map(|item| item.radius))
        .chain(static_footprints.iter().map(|item| item.radius))
        .fold(0.0_f32, f32::max);
    let context = SpreadContext::new(
        reserved_cells
            .iter()
            .copied()
            .map(|cell| (None, cell))
            .collect(),
        static_footprints.to_vec(),
        bounds,
        max_radius,
    );
    spread_destinations_with_context(members, point, &context, &std::collections::HashSet::new())
}

pub fn spread_destinations_with_context(
    members: &[SpreadMember],
    point: Vec2,
    context: &SpreadContext,
    excluded_units: &std::collections::HashSet<u32>,
) -> (MovementPool, Vec<(u32, Vec2)>) {
    assert!(
        !members.is_empty(),
        "a movement pool needs at least one unit"
    );
    let max_radius = members
        .iter()
        .map(|unit| unit.radius)
        .fold(0.0_f32, f32::max);
    let point = Vec2::new(
        point
            .x
            .clamp(max_radius, (context.bounds.x - max_radius).max(max_radius)),
        point
            .y
            .clamp(max_radius, (context.bounds.y - max_radius).max(max_radius)),
    );
    let spacing = 2.0 * max_radius + 0.1;
    let spare_count = 6_usize.max(members.len().div_ceil(4));
    let wanted = members.len() + spare_count;

    let mut slots = Vec::with_capacity(wanted);
    let mut nearest_indices = std::collections::BTreeSet::new();
    let mut pool_points: Vec<crate::grid::GridPoint> = Vec::new();
    let mut pool_grid = crate::grid::UniformGrid::new(&pool_points, spacing);
    if free_slot_grid(
        point,
        max_radius,
        context,
        excluded_units,
        &slots,
        &pool_points,
        &pool_grid,
    ) {
        slots.push(point);
        nearest_indices.insert((0_i64, 0_i64, slots.len() - 1));
        // The origin is slot zero in the canonical nearest-first hex pool.
        pool_points.push(crate::grid::GridPoint {
            id: 0,
            position: point,
        });
        pool_grid = crate::grid::UniformGrid::new(&pool_points, spacing);
    }
    let mut ring = 1_i32;
    let max_dx = (point.x - max_radius)
        .max(context.bounds.x - max_radius - point.x)
        .max(0.0);
    let max_dy = (point.y - max_radius)
        .max(context.bounds.y - max_radius - point.y)
        .max(0.0);
    let max_j = (max_dy * 2.0 / (3.0_f32.sqrt() * spacing)).ceil() as i32 + 1;
    let max_i = (max_dx / spacing + max_j as f32 * 0.5).ceil() as i32 + 1;
    let ring_limit = max_i.saturating_add(max_j).max(1);
    while ring <= ring_limit {
        // Axial hex perimeter: six sides, exactly 6r candidates, no interior scan.
        for side in 0..6 {
            for step in 0..ring {
                let (i, j) = match side {
                    0 => (ring - step, step),
                    1 => (-step, ring),
                    2 => (-ring, ring - step),
                    3 => (-ring + step, -step),
                    4 => (step, -ring),
                    _ => (ring, -ring + step),
                };
                let candidate = Vec2::new(
                    point.x + spacing * (i as f32 + 0.5 * j as f32),
                    point.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
                );
                if free_slot_grid(
                    candidate,
                    max_radius,
                    context,
                    excluded_units,
                    &slots,
                    &pool_points,
                    &pool_grid,
                ) {
                    let index = slots.len();
                    slots.push(candidate);
                    let ring_key = i64::from(i) * i64::from(i)
                        + i64::from(i) * i64::from(j)
                        + i64::from(j) * i64::from(j);
                    let angle_key = (angle(candidate, point) * 1_000_000.0).round() as i64;
                    nearest_indices.insert((ring_key, angle_key, index));
                    if nearest_indices.len() > wanted {
                        nearest_indices.pop_last();
                    }
                    pool_points.push(crate::grid::GridPoint {
                        id: index as u32,
                        position: candidate,
                    });
                    pool_grid.insert(index, candidate);
                }
            }
        }
        if nearest_indices.len() == wanted {
            let next_ring = i64::from(ring + 1);
            let minimum_unseen_ring_key = (3 * next_ring * next_ring + 3) / 4;
            if nearest_indices
                .last()
                .is_some_and(|(ring_key, _, _)| *ring_key < minimum_unseen_ring_key)
            {
                break;
            }
        }
        ring += 1;
    }
    if slots.len() < members.len() {
        let pool = MovementPool {
            pool_id: 0,
            point,
            spacing,
            primary_count: 0,
            slots,
        };
        return (pool, Vec::new());
    }
    slots = nearest_indices
        .iter()
        .map(|(_, _, index)| slots[*index])
        .collect();

    let primary = &slots[..members.len()];
    let centroid = members
        .iter()
        .fold(Vec2::ZERO, |sum, unit| sum + unit.position)
        / members.len() as f32;
    let direction = (point - centroid).try_normalize().unwrap_or(Vec2::X);
    let lateral = Vec2::new(-direction.y, direction.x);
    let mut unit_order: Vec<_> = members.iter().collect();
    unit_order.sort_by(|a, b| {
        (b.position.dot(direction))
            .total_cmp(&a.position.dot(direction))
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut slot_order: Vec<_> = primary.iter().copied().enumerate().collect();
    slot_order.sort_by(|(_, a), (_, b)| {
        b.dot(direction)
            .total_cmp(&a.dot(direction))
            .then_with(|| a.x.total_cmp(&b.x))
            .then_with(|| a.y.total_cmp(&b.y))
    });
    let band = (members.len() as f32).sqrt().round().max(1.0) as usize;
    let mut assignments = Vec::with_capacity(members.len());
    for start in (0..primary.len()).step_by(band) {
        let end = (start + band).min(members.len());
        let mut units = unit_order[start..end].to_vec();
        units.sort_by(|a, b| {
            a.position
                .dot(lateral)
                .total_cmp(&b.position.dot(lateral))
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut destinations = slot_order[start..end].to_vec();
        destinations.sort_by(|(_, a), (_, b)| {
            a.dot(lateral)
                .total_cmp(&b.dot(lateral))
                .then_with(|| a.x.total_cmp(&b.x))
                .then_with(|| a.y.total_cmp(&b.y))
        });
        assignments.extend(
            units
                .into_iter()
                .zip(destinations)
                .map(|(unit, (_, destination))| (unit.id, destination)),
        );
    }
    let pool = MovementPool {
        pool_id: 0,
        point,
        spacing,
        primary_count: members.len() as u32,
        slots,
    };
    (pool, assignments)
}

fn free_slot_grid(
    candidate: Vec2,
    radius: f32,
    context: &SpreadContext,
    excluded_units: &std::collections::HashSet<u32>,
    already_in_pool: &[Vec2],
    pool_points: &[crate::grid::GridPoint],
    pool_grid: &crate::grid::UniformGrid,
) -> bool {
    candidate.is_finite()
        && candidate.x >= radius
        && candidate.y >= radius
        && candidate.x <= context.bounds.x - radius
        && candidate.y <= context.bounds.y - radius
        && context
            .static_grid
            .within(
                &context.static_points,
                candidate,
                radius + context.max_static_radius,
            )
            .into_iter()
            .all(|i| {
                candidate.distance(context.static_footprints[i].position)
                    >= radius + context.static_footprints[i].radius
            })
        && context
            .reserved_grid
            .within(
                &context.reserved_points,
                candidate,
                radius + context.max_reserved_radius + 0.1,
            )
            .into_iter()
            .all(|i| {
                context.reserved_units[i].is_some_and(|unit| excluded_units.contains(&unit))
                    || candidate.distance(context.reserved[i].position)
                        >= radius + context.reserved[i].radius + 0.1 - 1.0e-5
            })
        && pool_grid
            .within(pool_points, candidate, radius * 2.0 + 0.1)
            .into_iter()
            .all(|i| candidate.distance(already_in_pool[i]) >= radius * 2.0 + 0.1 - 1.0e-5)
}

fn angle(point: Vec2, origin: Vec2) -> f32 {
    (point.y - origin.y).atan2(point.x - origin.x)
}

/// The complete input to one unit's deterministic flat-ground movement step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementInput {
    pub position: bevy_math::Vec2,
    pub target: bevy_math::Vec2,
    pub speed: f32,
}

/// The result of one unit's deterministic movement step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementOutput {
    pub position: bevy_math::Vec2,
    pub reached: bool,
}

/// Move one unit toward its target using the shared simulation and validator step.
pub fn step(input: &MovementInput, dt: f32) -> MovementOutput {
    let step = input.speed * dt;
    let offset = input.target - input.position;
    let distance = offset.length();
    if distance <= step {
        MovementOutput {
            position: input.target,
            reached: true,
        }
    } else {
        MovementOutput {
            position: input.position + offset * (step / distance),
            reached: false,
        }
    }
}

struct MovementSearch<'a> {
    bounds: Vec2,
    units: &'a [MovementFrameUnit],
    points: &'a [crate::grid::GridPoint],
    grid: &'a crate::grid::UniformGrid,
    max_radius: f32,
}

impl MovementSearch<'_> {
    fn find_free_singleton_destination(
        &self,
        origin: Vec2,
        radius: f32,
        self_index: usize,
    ) -> Option<Vec2> {
        let spacing = 2.0 * radius + 0.1;
        let ring_limit = finite_hex_ring_limit(origin, radius, self.bounds, spacing);
        let mut best = None;
        for ring in 1..=ring_limit {
            for side in 0..6 {
                for step in 0..ring {
                    let (i, j) = match side {
                        0 => (ring - step, step),
                        1 => (-step, ring),
                        2 => (-ring, ring - step),
                        3 => (-ring + step, -step),
                        4 => (step, -ring),
                        _ => (ring, -ring + step),
                    };
                    let candidate = Vec2::new(
                        origin.x + spacing * (i as f32 + 0.5 * j as f32),
                        origin.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
                    );
                    if candidate.x < radius
                        || candidate.y < radius
                        || candidate.x > self.bounds.x - radius
                        || candidate.y > self.bounds.y - radius
                    {
                        continue;
                    }
                    let free = self
                        .grid
                        .within(self.points, candidate, radius + self.max_radius)
                        .into_iter()
                        .all(|index| {
                            index == self_index
                                || candidate.distance(self.units[index].position)
                                    >= radius + self.units[index].radius - 1.0e-5
                        });
                    if free
                        && best.is_none_or(|previous| {
                            nearest_angle_order(candidate, previous, origin).is_lt()
                        })
                    {
                        best = Some(candidate);
                    }
                }
            }
            if best.is_some_and(|candidate| {
                minimum_unseen_hex_radius(ring, spacing) > candidate.distance(origin) + 1.0e-5
            }) {
                return best;
            }
        }
        best
    }

    fn find_free_escape_destination(
        &self,
        origin: Vec2,
        radius: f32,
        self_index: usize,
        pool_id: u64,
        assigned_slots: &std::collections::HashSet<(u64, u32, u32)>,
    ) -> Option<Vec2> {
        let spacing = 2.0 * radius + 0.1;
        let mut best: Option<Vec2> = None;
        let ring_limit = finite_hex_ring_limit(origin, radius, self.bounds, spacing);
        for ring in 1..=ring_limit {
            for side in 0..6 {
                for step in 0..ring {
                    let (i, j) = match side {
                        0 => (ring - step, step),
                        1 => (-step, ring),
                        2 => (-ring, ring - step),
                        3 => (-ring + step, -step),
                        4 => (step, -ring),
                        _ => (ring, -ring + step),
                    };
                    let candidate = Vec2::new(
                        origin.x + spacing * (i as f32 + 0.5 * j as f32),
                        origin.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
                    );
                    if candidate.x < radius
                        || candidate.y < radius
                        || candidate.x > self.bounds.x - radius
                        || candidate.y > self.bounds.y - radius
                    {
                        continue;
                    }
                    let key = (pool_id, candidate.x.to_bits(), candidate.y.to_bits());
                    if assigned_slots.contains(&key) {
                        continue;
                    }
                    let free = self
                        .grid
                        .within(self.points, candidate, radius + self.max_radius)
                        .into_iter()
                        .all(|index| {
                            index == self_index
                                || candidate.distance(self.units[index].position)
                                    >= radius + self.units[index].radius - 1.0e-5
                        });
                    if free {
                        let replace = best.is_none_or(|previous| {
                            nearest_angle_order(candidate, previous, origin).is_lt()
                        });
                        if replace {
                            best = Some(candidate);
                        }
                    }
                }
            }
            if best.is_some_and(|candidate| {
                minimum_unseen_hex_radius(ring, spacing) > candidate.distance(origin) + 1.0e-5
            }) {
                return best;
            }
        }
        None
    }
}

fn body_overlaps_position(
    position: Vec2,
    self_index: usize,
    radius: f32,
    units: &[MovementFrameUnit],
    points: &[crate::grid::GridPoint],
    grid: &crate::grid::UniformGrid,
    max_radius: f32,
) -> bool {
    grid.within(points, position, radius + max_radius)
        .into_iter()
        .any(|index| {
            index != self_index
                && position.distance(units[index].position) < radius + units[index].radius - 1.0e-5
        })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the shared frame kernel passes immutable index inputs without rebuilding them"
)]
fn step_plain_progress(
    bounds: Vec2,
    tick_hz: u16,
    max_radius: f32,
    dt: f32,
    index: usize,
    unit: &MovementFrameUnit,
    mut progress: PlainMoveProgress,
    units: &[MovementFrameUnit],
    points: &[crate::grid::GridPoint],
    grid: &crate::grid::UniformGrid,
    spread_context: &SpreadContext,
) -> MovementFrameOutput {
    let mut output = MovementFrameOutput {
        unit_id: unit.unit_id,
        position: unit.position,
        target: unit.target,
        progress: unit.progress.clone(),
        plain_progress: Some(progress.clone()),
        physical_escape: unit.physical_escape.clone(),
        failed_blocked: false,
    };
    let Some(speed) = unit.speed else {
        output.target = None;
        output.plain_progress = Some(progress);
        return output;
    };
    if progress.activity == MovementActivity::Resting {
        output.target = None;
        output.plain_progress = Some(progress);
        return output;
    }
    let step_limit = speed * dt;
    let body_occupied = |point: Vec2| {
        grid.within(points, point, unit.radius + max_radius)
            .into_iter()
            .any(|other_index| {
                other_index != index
                    && point.distance(units[other_index].position)
                        < unit.radius + units[other_index].radius - 1.0e-5
            })
    };
    let occupied = |point: Vec2| {
        spread_context.conflicts_with_reserved(point, unit.radius, unit.unit_id)
            || body_occupied(point)
    };

    if progress.activity == MovementActivity::Blocked {
        let escape = unit.target.filter(|target| !occupied(*target)).or_else(|| {
            if !body_occupied(unit.position) {
                return None;
            }
            MovementSearch {
                bounds,
                units,
                points,
                grid,
                max_radius,
            }
            .find_free_singleton_destination(unit.position, unit.radius, index)
        });
        let Some(escape) = escape else {
            output.target = None;
            output.plain_progress = Some(progress);
            return output;
        };
        let offset = escape - unit.position;
        let distance = offset.length();
        let position = if distance <= step_limit {
            escape
        } else if distance > 1.0e-8 {
            unit.position + offset * (step_limit / distance)
        } else {
            unit.position
        };
        let reached = position == escape && !occupied(escape);
        output.position = position;
        output.target = (!reached).then_some(escape);
        progress.slot = escape;
        output.plain_progress = Some(progress);
        return output;
    }

    let movement_target = progress.slot;
    let distance = unit.position.distance(movement_target);
    if (distance <= step_limit || distance <= 0.025) && !occupied(movement_target) {
        output.position = movement_target;
        output.target = None;
        if progress.claim_count > 0 || progress.slot != progress.original_point {
            progress.activity = MovementActivity::Resting;
            output.plain_progress = Some(progress);
        } else {
            output.plain_progress = None;
        }
        return output;
    }

    let desired = movement_target - unit.position;
    let mut velocity = desired.try_normalize().unwrap_or(Vec2::ZERO) * step_limit;
    let mut separated = false;
    for other_index in grid.within(
        points,
        unit.position,
        unit.radius + max_radius + 0.05 + step_limit * 1.2,
    ) {
        if other_index == index {
            continue;
        }
        let other = &units[other_index];
        let offset = unit.position - other.position;
        let separation = offset.length();
        let limit = unit.radius + other.radius + 0.05;
        if separation > 1.0e-8 && separation < limit {
            separated = true;
            velocity += offset / separation * (limit - separation);
            if offset.dot(desired) < 0.0 {
                let along = desired.try_normalize().unwrap_or(Vec2::X);
                velocity += Vec2::new(along.y, -along.x) * (limit - separation);
            }
        }
    }
    if !separated && distance > step_limit {
        velocity = step(
            &MovementInput {
                position: unit.position,
                target: movement_target,
                speed,
            },
            dt,
        )
        .position
            - unit.position;
    } else if velocity.length() > step_limit && step_limit > 0.0 {
        velocity = velocity.normalize() * step_limit;
    }
    let mut position = unit.position + velocity;
    for _ in 0..8 {
        let mut projected = false;
        for other_index in grid.within(
            points,
            position,
            unit.radius + max_radius + step_limit * 1.2,
        ) {
            if other_index == index {
                continue;
            }
            let other = &units[other_index];
            let offset = position - other.position;
            let separation = offset.length();
            let limit = unit.radius + other.radius + 1.0e-4;
            if separation < limit {
                let direction = if separation > 1.0e-8 {
                    offset / separation
                } else if unit.unit_id < other.unit_id {
                    Vec2::X
                } else {
                    -Vec2::X
                };
                position += direction * (limit - separation);
                projected = true;
            }
        }
        if !projected {
            break;
        }
    }
    position.x = position
        .x
        .clamp(unit.radius, (bounds.x - unit.radius).max(unit.radius));
    position.y = position
        .y
        .clamp(unit.radius, (bounds.y - unit.radius).max(unit.radius));

    let next_distance = position.distance(movement_target);
    if position == movement_target && !occupied(movement_target) {
        output.position = position;
        output.target = None;
        if progress.claim_count > 0 || progress.slot != progress.original_point {
            progress.activity = MovementActivity::Resting;
            output.plain_progress = Some(progress);
        } else {
            output.plain_progress = None;
        }
        return output;
    }
    let made_progress = next_distance < progress.best_distance - 0.01 * step_limit;
    if made_progress {
        progress.best_distance = next_distance;
        progress.stalled_ticks = 0;
    } else {
        progress.stalled_ticks = progress.stalled_ticks.saturating_add(1);
    }
    if progress.stalled_ticks >= tick_hz.max(1) {
        let spare = if progress.claim_count < 3 {
            progress
                .pool_slots
                .iter()
                .copied()
                .filter(|candidate| *candidate != progress.slot && !occupied(*candidate))
                .min_by(|a, b| nearest_angle_order(*a, *b, unit.position))
        } else {
            None
        };
        if let Some(spare) = spare {
            progress
                .pool_slots
                .retain(|candidate| *candidate != progress.slot && *candidate != spare);
            progress.slot = spare;
            progress.claim_count += 1;
            progress.stalled_ticks = 0;
            progress.best_distance = position.distance(spare);
            output.target = Some(spare);
        } else {
            progress.activity = MovementActivity::Blocked;
            output.target = None;
            output.failed_blocked = true;
            if body_overlaps_position(
                position,
                index,
                unit.radius,
                units,
                points,
                grid,
                max_radius,
            ) {
                output.physical_escape = Some(PhysicalEscape { target: None });
            }
        }
    } else {
        output.target = Some(movement_target);
    }
    output.position = position;
    output.plain_progress = Some(progress);
    output
}

fn minimum_unseen_hex_radius(ring: i32, spacing: f32) -> f32 {
    let next_ring = i128::from(ring) + 1;
    let minimum_squared = (3 * next_ring * next_ring + 3) / 4;
    (minimum_squared as f64).sqrt() as f32 * spacing
}

fn finite_hex_ring_limit(origin: Vec2, radius: f32, bounds: Vec2, spacing: f32) -> i32 {
    let max_dx = (origin.x - radius)
        .max(bounds.x - radius - origin.x)
        .max(0.0);
    let max_dy = (origin.y - radius)
        .max(bounds.y - radius - origin.y)
        .max(0.0);
    let max_j = (max_dy * 2.0 / (3.0_f32.sqrt() * spacing)).ceil() as i32 + 1;
    let max_i = (max_dx / spacing + max_j as f32 * 0.5).ceil() as i32 + 1;
    max_i.saturating_add(max_j).max(1)
}

fn make_plain_progress(
    unit: &MovementFrameUnit,
    original_point: Vec2,
    context: &SpreadContext,
) -> PlainMoveProgress {
    let member = [SpreadMember {
        id: unit.unit_id,
        position: unit.position,
        radius: unit.radius,
    }];
    let excluded_units = std::collections::HashSet::from([unit.unit_id]);
    let (pool, _) =
        spread_destinations_with_context(&member, original_point, context, &excluded_units);
    let slot = pool.slots.first().copied().unwrap_or(original_point);
    let pool_slots = pool.slots.into_iter().skip(1).take(6).collect();
    PlainMoveProgress {
        original_point,
        slot,
        pool_slots,
        gather_goal: None,
        activity: if pool.primary_count == 0 {
            MovementActivity::Blocked
        } else {
            MovementActivity::Moving
        },
        stalled_ticks: 0,
        best_distance: unit.position.distance(slot),
        claim_count: 0,
        failure_recorded: false,
    }
}

fn nearest_angle_order(left: Vec2, right: Vec2, origin: Vec2) -> std::cmp::Ordering {
    let left_distance = left.distance_squared(origin);
    let right_distance = right.distance_squared(origin);
    if (left_distance - right_distance).abs() > 1.0e-5 {
        left_distance.total_cmp(&right_distance)
    } else {
        (left.y - origin.y)
            .atan2(left.x - origin.x)
            .total_cmp(&(right.y - origin.y).atan2(right.x - origin.x))
    }
}

fn target_key(target: Vec2) -> (u32, u32) {
    (target.x.to_bits(), target.y.to_bits())
}

fn update_target_claims(
    claims: &mut std::collections::HashMap<(u32, u32), usize>,
    old: Option<Vec2>,
    new: Option<Vec2>,
) {
    if old == new {
        return;
    }
    if let Some(old) = old {
        let key = target_key(old);
        if let Some(count) = claims.get_mut(&key) {
            *count -= 1;
            if *count == 0 {
                claims.remove(&key);
            }
        }
    }
    if let Some(new) = new {
        *claims.entry(target_key(new)).or_default() += 1;
    }
}

struct MovementPoolIndex {
    points: Vec<crate::grid::GridPoint>,
    grid: crate::grid::UniformGrid,
    slot_indices: std::collections::HashMap<(u32, u32), usize>,
    clearances: Vec<SlotClearance>,
    available_by_clearance: std::collections::BTreeSet<ClearanceRank>,
    available_slot_indices: std::collections::HashSet<usize>,
    spacing: f32,
    max_reach: f32,
}

#[derive(Debug, Clone, Copy)]
struct ClearanceHit {
    clearance: f64,
    unit_id: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
struct SlotClearance {
    first: ClearanceHit,
    second: ClearanceHit,
}

#[derive(Debug, Clone, Copy)]
struct ClearanceRank {
    clearance: f64,
    slot_index: usize,
}

impl PartialEq for ClearanceRank {
    fn eq(&self, other: &Self) -> bool {
        self.clearance.total_cmp(&other.clearance).is_eq() && self.slot_index == other.slot_index
    }
}

impl Eq for ClearanceRank {}

impl PartialOrd for ClearanceRank {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ClearanceRank {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.clearance
            .total_cmp(&other.clearance)
            .then_with(|| self.slot_index.cmp(&other.slot_index))
    }
}

impl SlotClearance {
    fn empty() -> Self {
        let empty = ClearanceHit {
            clearance: f64::INFINITY,
            unit_id: None,
        };
        Self {
            first: empty,
            second: empty,
        }
    }
}

impl MovementPoolIndex {
    fn new(pool: &MovementPool, bounds: Vec2) -> Self {
        let points: Vec<_> = pool
            .slots
            .iter()
            .enumerate()
            .map(|(index, &position)| crate::grid::GridPoint {
                id: index as u32,
                position,
            })
            .collect();
        let grid = crate::grid::UniformGrid::new(&points, pool.spacing);
        let slot_indices = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                (
                    (point.position.x.to_bits(), point.position.y.to_bits()),
                    index,
                )
            })
            .collect();
        Self {
            points,
            grid,
            slot_indices,
            clearances: vec![SlotClearance::empty(); pool.slots.len()],
            available_by_clearance: std::collections::BTreeSet::new(),
            available_slot_indices: std::collections::HashSet::new(),
            spacing: pool.spacing.max(0.1),
            max_reach: bounds.length(),
        }
    }

    fn refresh_slot_clearance(
        &mut self,
        slot_index: usize,
        units: &[MovementFrameUnit],
        points: &[crate::grid::GridPoint],
        grid: &crate::grid::UniformGrid,
        max_radius: f32,
        assigned: bool,
    ) {
        if !assigned {
            self.available_by_clearance.remove(&ClearanceRank {
                clearance: self.clearances[slot_index].first.clearance,
                slot_index,
            });
        }
        let slot = self.points[slot_index].position;
        let mut nearest = [SlotClearance::empty().first; 2];
        for unit_index in grid.within(points, slot, 2.0 * max_radius) {
            let unit = &units[unit_index];
            let clearance = f64::from(slot.distance(unit.position)) - f64::from(unit.radius);
            let hit = ClearanceHit {
                clearance: if clearance > f64::from(max_radius) {
                    f64::INFINITY
                } else {
                    clearance
                },
                unit_id: (clearance <= f64::from(max_radius)).then_some(unit.unit_id),
            };
            let earlier = |left: ClearanceHit, right: ClearanceHit| {
                left.clearance
                    .total_cmp(&right.clearance)
                    .then_with(|| left.unit_id.cmp(&right.unit_id))
                    .is_lt()
            };
            if earlier(hit, nearest[0]) {
                nearest[1] = nearest[0];
                nearest[0] = hit;
            } else if earlier(hit, nearest[1]) {
                nearest[1] = hit;
            }
        }
        let clearance = SlotClearance {
            first: nearest[0],
            second: nearest[1],
        };
        self.clearances[slot_index] = clearance;
        if !assigned {
            self.available_by_clearance.insert(ClearanceRank {
                clearance: clearance.first.clearance,
                slot_index,
            });
            self.available_slot_indices.insert(slot_index);
        } else {
            self.available_slot_indices.remove(&slot_index);
        }
    }

    fn set_slot_assigned(&mut self, slot_index: usize, assigned: bool) {
        if assigned {
            if self.available_slot_indices.remove(&slot_index) {
                self.available_by_clearance.remove(&ClearanceRank {
                    clearance: self.clearances[slot_index].first.clearance,
                    slot_index,
                });
            }
        } else if self.available_slot_indices.insert(slot_index) {
            self.available_by_clearance.insert(ClearanceRank {
                clearance: self.clearances[slot_index].first.clearance,
                slot_index,
            });
        }
    }

    fn has_potential_free_slot(
        &self,
        unit_id: u32,
        position: Vec2,
        radius: f32,
        allowed_slot: Option<(u32, u32)>,
    ) -> bool {
        let threshold = f64::from(radius) - 1.0e-5;
        if self
            .available_by_clearance
            .last()
            .is_some_and(|rank| rank.clearance >= threshold)
        {
            return true;
        }
        if let Some(slot_key) = allowed_slot
            && let Some(&slot_index) = self.slot_indices.get(&slot_key)
        {
            let clearance = self.clearances[slot_index];
            let nearest_without_self = if clearance.first.unit_id == Some(unit_id) {
                clearance.second
            } else {
                clearance.first
            };
            if nearest_without_self.clearance >= threshold {
                return true;
            }
        }
        let local = self.grid.within(&self.points, position, 2.0 * radius);
        #[cfg(test)]
        count_pool_slot_candidates(local.len());
        local.into_iter().any(|slot_index| {
            if !self.available_slot_indices.contains(&slot_index) {
                return false;
            }
            let clearance = self.clearances[slot_index];
            clearance.first.unit_id == Some(unit_id) && clearance.second.clearance >= threshold
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "pool candidate lookup needs exact caller, reservation and collision context"
    )]
    fn nearest_free(
        &self,
        unit_id: u32,
        origin: Vec2,
        pool_id: u64,
        allowed_slot: Option<(u64, u32, u32)>,
        assigned_slots: &std::collections::HashSet<(u64, u32, u32)>,
        radius: f32,
        is_free: impl Fn(Vec2) -> bool,
    ) -> Option<Vec2> {
        if !self.has_potential_free_slot(
            unit_id,
            origin,
            radius,
            allowed_slot.map(|(_, x, y)| (x, y)),
        ) {
            return None;
        }
        let mut reach = self.spacing;
        loop {
            let query_reach = reach.min(self.max_reach);
            let mut best: Option<(usize, Vec2)> = None;
            let candidates = self.grid.within(&self.points, origin, query_reach);
            #[cfg(test)]
            count_pool_slot_candidates(candidates.len());
            for index in candidates {
                let point = self.points[index].position;
                let key = (pool_id, point.x.to_bits(), point.y.to_bits());
                if (assigned_slots.contains(&key) && Some(key) != allowed_slot) || !is_free(point) {
                    continue;
                }
                if best.is_none_or(|(best_index, best_point)| {
                    nearest_angle_order(point, best_point, origin)
                        .then_with(|| index.cmp(&best_index))
                        .is_lt()
                }) {
                    best = Some((index, point));
                }
            }
            if let Some((_, point)) = best
                && point.distance(origin) <= query_reach + 1.0e-5
            {
                return Some(point);
            }
            if query_reach >= self.max_reach {
                return None;
            }
            reach = (reach * 2.0).min(self.max_reach);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "incremental clearance refresh receives the shared frame indexes and claim state"
)]
fn refresh_nearby_pool_slot_clearances(
    old_position: Vec2,
    new_position: Vec2,
    body_radius: f32,
    slot_points: &[crate::grid::GridPoint],
    slot_refs: &[(u64, usize)],
    slot_grid: &crate::grid::UniformGrid,
    pool_indexes: &mut BTreeMap<u64, MovementPoolIndex>,
    units: &[MovementFrameUnit],
    unit_points: &[crate::grid::GridPoint],
    unit_grid: &crate::grid::UniformGrid,
    assigned_slots: &std::collections::HashSet<(u64, u32, u32)>,
    max_radius: f32,
) {
    let reach = body_radius + max_radius;
    let mut affected = slot_grid
        .within(slot_points, old_position, reach)
        .into_iter()
        .chain(slot_grid.within(slot_points, new_position, reach))
        .collect::<Vec<_>>();
    affected.sort_unstable();
    affected.dedup();
    for global_index in affected {
        let (pool_id, slot_index) = slot_refs[global_index];
        let Some(index) = pool_indexes.get_mut(&pool_id) else {
            continue;
        };
        let point = index.points[slot_index].position;
        let assigned = assigned_slots.contains(&(pool_id, point.x.to_bits(), point.y.to_bits()));
        index.refresh_slot_clearance(
            slot_index,
            units,
            unit_points,
            unit_grid,
            max_radius,
            assigned,
        );
    }
}

fn add_pool_slot_claim(
    claim_counts: &mut std::collections::HashMap<(u64, u32, u32), usize>,
    pool_indexes: &mut BTreeMap<u64, MovementPoolIndex>,
    pool_id: u64,
    point: Vec2,
) {
    let key = (pool_id, point.x.to_bits(), point.y.to_bits());
    let count = claim_counts.entry(key).or_default();
    if *count == 0
        && let Some(index) = pool_indexes.get_mut(&pool_id)
        && let Some(&slot_index) = index.slot_indices.get(&(key.1, key.2))
    {
        index.set_slot_assigned(slot_index, true);
    }
    *count += 1;
}

fn remove_pool_slot_claim(
    claim_counts: &mut std::collections::HashMap<(u64, u32, u32), usize>,
    pool_indexes: &mut BTreeMap<u64, MovementPoolIndex>,
    pool_id: u64,
    point: Vec2,
) -> bool {
    let key = (pool_id, point.x.to_bits(), point.y.to_bits());
    let Some(count) = claim_counts.get_mut(&key) else {
        return false;
    };
    if *count > 1 {
        *count -= 1;
        return false;
    }
    claim_counts.remove(&key);
    if let Some(index) = pool_indexes.get_mut(&pool_id)
        && let Some(&slot_index) = index.slot_indices.get(&(key.1, key.2))
    {
        index.set_slot_assigned(slot_index, false);
    }
    true
}

/// One immutable unit row consumed by deterministic group movement.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrameUnit {
    pub unit_id: u32,
    pub owner_slot: u8,
    pub unit_type: u32,
    pub position: Vec2,
    pub radius: f32,
    pub speed: Option<f32>,
    pub target: Option<Vec2>,
    pub progress: Option<MoveProgress>,
    pub plain_progress: Option<PlainMoveProgress>,
    pub physical_escape: Option<PhysicalEscape>,
}

/// All movement-relevant state for one fixed tick. Both the sim and log validator use this input.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrame {
    pub bounds: Vec2,
    pub tick_hz: u16,
    pub units: Vec<MovementFrameUnit>,
    pub pools: BTreeMap<u64, MovementPool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrameOutput {
    pub unit_id: u32,
    pub position: Vec2,
    pub target: Option<Vec2>,
    pub progress: Option<MoveProgress>,
    pub plain_progress: Option<PlainMoveProgress>,
    pub physical_escape: Option<PhysicalEscape>,
    pub failed_blocked: bool,
}

/// Deterministic movement kernel shared by the ECS adapter and economy-log replay.
pub fn step_frame(frame: &MovementFrame, dt: f32) -> Vec<MovementFrameOutput> {
    let mut units = frame.units.clone();
    units.sort_unstable_by_key(|unit| unit.unit_id);
    let mut points: Vec<_> = units
        .iter()
        .map(|unit| crate::grid::GridPoint {
            id: unit.unit_id,
            position: unit.position,
        })
        .collect();
    let max_radius = units.iter().map(|unit| unit.radius).fold(0.0_f32, f32::max);
    let mut grid = crate::grid::UniformGrid::new(&points, 2.0 * max_radius);
    let mut pool_indexes: BTreeMap<_, _> = frame
        .pools
        .iter()
        .map(|(&pool_id, pool)| (pool_id, MovementPoolIndex::new(pool, frame.bounds)))
        .collect();
    let mut plain_reserved = Vec::new();
    let mut plain_static = Vec::new();
    let mut plain_max_radius = 0.0_f32;
    for unit in &units {
        plain_max_radius = plain_max_radius.max(unit.radius);
        if unit.speed.is_none() {
            plain_static.push(StaticFootprint {
                position: unit.position,
                radius: unit.radius,
            });
            continue;
        }
        plain_reserved.push((
            Some(unit.unit_id),
            ReservedFootprint {
                position: unit.target.unwrap_or(unit.position),
                radius: unit.radius,
            },
        ));
    }
    let mut plain_spread_context =
        SpreadContext::new(plain_reserved, plain_static, frame.bounds, plain_max_radius);
    let max_step = units
        .iter()
        .filter_map(|unit| unit.speed)
        .fold(0.0_f32, f32::max)
        * dt
        * 1.2;
    let mut outputs: Vec<_> = units
        .iter()
        .map(|unit| MovementFrameOutput {
            unit_id: unit.unit_id,
            position: unit.position,
            target: unit.target,
            progress: unit.progress.clone(),
            plain_progress: unit.plain_progress.clone(),
            physical_escape: unit.physical_escape.clone(),
            failed_blocked: false,
        })
        .collect();
    let mut slot_claim_counts: std::collections::HashMap<(u64, u32, u32), usize> = units
        .iter()
        .filter_map(|unit| {
            unit.progress.as_ref().map(|progress| {
                (
                    (
                        progress.pool_id,
                        progress.slot.x.to_bits(),
                        progress.slot.y.to_bits(),
                    ),
                    1usize,
                )
            })
        })
        .fold(
            std::collections::HashMap::new(),
            |mut counts, (key, count)| {
                *counts.entry(key).or_default() += count;
                counts
            },
        );
    let mut assigned_slots: std::collections::HashSet<(u64, u32, u32)> =
        slot_claim_counts.keys().copied().collect();
    let mut slot_refs = Vec::new();
    let mut slot_points = Vec::new();
    for (&pool_id, index) in &pool_indexes {
        for (slot_index, point) in index.points.iter().enumerate() {
            slot_refs.push((pool_id, slot_index));
            slot_points.push(crate::grid::GridPoint {
                id: slot_points.len() as u32,
                position: point.position,
            });
        }
    }
    let slot_grid = crate::grid::UniformGrid::new(&slot_points, 2.0 * max_radius);
    for (&pool_id, index) in &mut pool_indexes {
        for slot_index in 0..index.points.len() {
            let point = index.points[slot_index].position;
            let assigned =
                assigned_slots.contains(&(pool_id, point.x.to_bits(), point.y.to_bits()));
            index.refresh_slot_clearance(slot_index, &units, &points, &grid, max_radius, assigned);
        }
    }
    let mut target_claims: std::collections::HashMap<(u32, u32), usize> = units
        .iter()
        .filter_map(|unit| unit.target.map(target_key))
        .fold(std::collections::HashMap::new(), |mut counts, key| {
            *counts.entry(key).or_default() += 1;
            counts
        });

    for index in 0..units.len() {
        if index > 0 {
            let previous = &units[index - 1];
            if previous.speed.is_some() {
                plain_spread_context.reserve_unit(
                    previous.unit_id,
                    ReservedFootprint {
                        position: previous.target.unwrap_or(previous.position),
                        radius: previous.radius,
                    },
                );
            }
        }
        let unit = units[index].clone();
        let Some(speed) = unit.speed else { continue };
        if unit.physical_escape.is_some() {
            let overlapping = body_overlaps_position(
                unit.position,
                index,
                unit.radius,
                &units,
                &points,
                &grid,
                max_radius,
            );
            if !overlapping {
                outputs[index].physical_escape = None;
                if outputs[index]
                    .progress
                    .as_ref()
                    .is_some_and(|progress| progress.activity == MovementActivity::Blocked)
                {
                    let progress = outputs[index].progress.take().unwrap();
                    let key = (
                        progress.pool_id,
                        progress.slot.x.to_bits(),
                        progress.slot.y.to_bits(),
                    );
                    if remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        progress.slot,
                    ) {
                        assigned_slots.remove(&key);
                    }
                }
                units[index].physical_escape = None;
                units[index].progress = outputs[index].progress.clone();
                units[index].plain_progress = outputs[index].plain_progress.clone();
                // Clearance ends the obligation. A newly accepted target can
                // resume through the ordinary movement path on this same tick.
            } else {
                let escape = unit
                    .physical_escape
                    .as_ref()
                    .and_then(|state| state.target)
                    .filter(|target| {
                        !body_overlaps_position(
                            *target,
                            index,
                            unit.radius,
                            &units,
                            &points,
                            &grid,
                            max_radius,
                        )
                    })
                    .or_else(|| {
                        MovementSearch {
                            bounds: frame.bounds,
                            units: &units,
                            points: &points,
                            grid: &grid,
                            max_radius,
                        }
                        .find_free_singleton_destination(
                            unit.position,
                            unit.radius,
                            index,
                        )
                    });
                let Some(escape) = escape else {
                    outputs[index].physical_escape = Some(PhysicalEscape { target: None });
                    units[index].physical_escape = outputs[index].physical_escape.clone();
                    continue;
                };
                let offset = escape - unit.position;
                let distance = offset.length();
                let mut position = if distance <= speed * dt {
                    escape
                } else if distance > 1.0e-8 {
                    unit.position + offset * ((speed * dt) / distance)
                } else {
                    unit.position
                };
                position.x = position
                    .x
                    .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
                position.y = position
                    .y
                    .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));
                let still_overlapping = body_overlaps_position(
                    position,
                    index,
                    unit.radius,
                    &units,
                    &points,
                    &grid,
                    max_radius,
                );
                outputs[index].position = position;
                outputs[index].physical_escape = still_overlapping.then_some(PhysicalEscape {
                    target: Some(escape),
                });
                if !still_overlapping
                    && outputs[index]
                        .progress
                        .as_ref()
                        .is_some_and(|progress| progress.activity == MovementActivity::Blocked)
                {
                    let progress = outputs[index].progress.take().unwrap();
                    let key = (
                        progress.pool_id,
                        progress.slot.x.to_bits(),
                        progress.slot.y.to_bits(),
                    );
                    if remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        progress.slot,
                    ) {
                        assigned_slots.remove(&key);
                    }
                }
                units[index].position = position;
                units[index].physical_escape = outputs[index].physical_escape.clone();
                units[index].progress = outputs[index].progress.clone();
                units[index].plain_progress = outputs[index].plain_progress.clone();
                points[index].position = position;
                grid.move_point(index, unit.position, position);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    position,
                    unit.radius,
                    &slot_points,
                    &slot_refs,
                    &slot_grid,
                    &mut pool_indexes,
                    &units,
                    &points,
                    &grid,
                    &assigned_slots,
                    max_radius,
                );
                continue;
            }
        }
        if unit
            .progress
            .as_ref()
            .is_some_and(|progress| progress.activity == MovementActivity::Blocked)
        {
            let Some(mut progress) = unit.progress.clone() else {
                continue;
            };
            let Some(_pool) = frame.pools.get(&progress.pool_id) else {
                continue;
            };
            let occupied = grid
                .within(&points, unit.position, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    other_index != index
                        && unit.position.distance(units[other_index].position)
                            < unit.radius + units[other_index].radius - 1.0e-5
                });
            if !occupied {
                update_target_claims(&mut target_claims, unit.target, None);
                let old_key = (
                    progress.pool_id,
                    progress.slot.x.to_bits(),
                    progress.slot.y.to_bits(),
                );
                if remove_pool_slot_claim(
                    &mut slot_claim_counts,
                    &mut pool_indexes,
                    progress.pool_id,
                    progress.slot,
                ) {
                    assigned_slots.remove(&old_key);
                }
                outputs[index].target = None;
                outputs[index].progress = None;
                units[index].target = None;
                units[index].progress = None;
                continue;
            }
            let is_free = |slot: Vec2| {
                grid.within(&points, slot, unit.radius + max_radius)
                    .into_iter()
                    .all(|other_index| {
                        other_index == index
                            || slot.distance(units[other_index].position)
                                >= unit.radius + units[other_index].radius
                    })
            };
            let old_slot_key = (
                progress.pool_id,
                progress.slot.x.to_bits(),
                progress.slot.y.to_bits(),
            );
            let target_is_claimed_by_other = unit.target.is_some_and(|target| {
                target_claims.get(&target_key(target)).copied().unwrap_or(0) > 1
            });
            let target_is_blocked_by_stationary = unit.target.is_some_and(|target| {
                grid.within(&points, target, unit.radius + max_radius)
                    .into_iter()
                    .any(|other_index| {
                        if other_index == index {
                            return false;
                        }
                        let other = &units[other_index];
                        let stationary = other.target.is_none()
                            || other
                                .progress
                                .as_ref()
                                .is_some_and(|state| state.activity == MovementActivity::Resting);
                        stationary
                            && target.distance(other.position) < unit.radius + other.radius - 1.0e-5
                    })
            });
            let retained_escape = unit.target.filter(|slot| {
                let key = (progress.pool_id, slot.x.to_bits(), slot.y.to_bits());
                !target_is_claimed_by_other
                    && !target_is_blocked_by_stationary
                    && is_free(*slot)
                    && (!assigned_slots.contains(&key) || key == old_slot_key)
            });
            let pool_escape = pool_indexes.get(&progress.pool_id).and_then(|index| {
                index.nearest_free(
                    unit.unit_id,
                    unit.position,
                    progress.pool_id,
                    Some(old_slot_key),
                    &assigned_slots,
                    unit.radius,
                    is_free,
                )
            });
            let outside_escape = MovementSearch {
                bounds: frame.bounds,
                units: &units,
                points: &points,
                grid: &grid,
                max_radius,
            }
            .find_free_escape_destination(
                unit.position,
                unit.radius,
                index,
                progress.pool_id,
                &assigned_slots,
            );
            let nearest_new = match (pool_escape, outside_escape) {
                (Some(pool_point), Some(outside_point)) => Some(
                    if nearest_angle_order(pool_point, outside_point, unit.position).is_le() {
                        pool_point
                    } else {
                        outside_point
                    },
                ),
                (Some(point), None) | (None, Some(point)) => Some(point),
                (None, None) => None,
            };
            let escape = retained_escape.or(nearest_new);
            let Some(escape) = escape else { continue };
            let escape_key = (progress.pool_id, escape.x.to_bits(), escape.y.to_bits());
            if escape_key != old_slot_key {
                if remove_pool_slot_claim(
                    &mut slot_claim_counts,
                    &mut pool_indexes,
                    progress.pool_id,
                    progress.slot,
                ) {
                    assigned_slots.remove(&old_slot_key);
                }
                add_pool_slot_claim(
                    &mut slot_claim_counts,
                    &mut pool_indexes,
                    progress.pool_id,
                    escape,
                );
                assigned_slots.insert(escape_key);
                progress.slot = escape;
            }
            let step_limit = speed * dt;
            let delta = escape - unit.position;
            let distance = delta.length();
            let mut position = if distance <= step_limit {
                escape
            } else if distance > 1.0e-8 {
                unit.position + delta / distance * step_limit
            } else {
                unit.position
            };
            position.x = position
                .x
                .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
            position.y = position
                .y
                .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));
            let reached = position == escape && is_free(escape);
            update_target_claims(
                &mut target_claims,
                unit.target,
                (!reached).then_some(escape),
            );
            outputs[index].position = position;
            outputs[index].target = (!reached).then_some(escape);
            outputs[index].progress = Some(progress.clone());
            units[index].position = position;
            points[index].position = position;
            grid.move_point(index, unit.position, position);
            refresh_nearby_pool_slot_clearances(
                unit.position,
                position,
                unit.radius,
                &slot_points,
                &slot_refs,
                &slot_grid,
                &mut pool_indexes,
                &units,
                &points,
                &grid,
                &assigned_slots,
                max_radius,
            );
            units[index].target = outputs[index].target;
            units[index].progress = outputs[index].progress.clone();
            continue;
        }
        if let Some(progress) = unit.plain_progress.clone() {
            let output = step_plain_progress(
                frame.bounds,
                frame.tick_hz,
                max_radius,
                dt,
                index,
                &unit,
                progress,
                &units,
                &points,
                &grid,
                &plain_spread_context,
            );
            outputs[index] = output.clone();
            update_target_claims(&mut target_claims, unit.target, output.target);
            units[index].position = output.position;
            units[index].target = output.target;
            units[index].plain_progress = output.plain_progress.clone();
            points[index].position = output.position;
            grid.move_point(index, unit.position, output.position);
            refresh_nearby_pool_slot_clearances(
                unit.position,
                output.position,
                unit.radius,
                &slot_points,
                &slot_refs,
                &slot_grid,
                &mut pool_indexes,
                &units,
                &points,
                &grid,
                &assigned_slots,
                max_radius,
            );
            continue;
        }
        let Some(target) = unit.target else { continue };
        let Some(mut progress) = unit.progress.clone() else {
            let target_occupied =
                plain_spread_context.conflicts_with_reserved(target, unit.radius, unit.unit_id)
                    || grid
                        .within(&points, target, unit.radius + max_radius)
                        .into_iter()
                        .any(|other_index| {
                            other_index != index
                                && target.distance(units[other_index].position)
                                    < unit.radius + units[other_index].radius - 1.0e-5
                        });
            if target_occupied {
                let progress = make_plain_progress(&unit, target, &plain_spread_context);
                let mut plain_unit = unit.clone();
                plain_unit.target = Some(progress.slot);
                let output = step_plain_progress(
                    frame.bounds,
                    frame.tick_hz,
                    max_radius,
                    dt,
                    index,
                    &plain_unit,
                    progress,
                    &units,
                    &points,
                    &grid,
                    &plain_spread_context,
                );
                outputs[index] = output.clone();
                update_target_claims(&mut target_claims, unit.target, output.target);
                units[index].position = output.position;
                units[index].target = output.target;
                units[index].plain_progress = output.plain_progress.clone();
                points[index].position = output.position;
                grid.move_point(index, unit.position, output.position);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    output.position,
                    unit.radius,
                    &slot_points,
                    &slot_refs,
                    &slot_grid,
                    &mut pool_indexes,
                    &units,
                    &points,
                    &grid,
                    &assigned_slots,
                    max_radius,
                );
                continue;
            }
            let destination = target;
            let desired = destination - unit.position;
            let step_limit = speed * dt;
            let destination_free = grid
                .within(&points, destination, unit.radius + max_radius)
                .into_iter()
                .all(|other_index| {
                    other_index == index
                        || destination.distance(units[other_index].position)
                            >= unit.radius + units[other_index].radius - 1.0e-5
                });
            if (desired.length() <= step_limit || desired.length() <= 0.025) && destination_free {
                outputs[index].position = destination;
                outputs[index].target = None;
                units[index].position = destination;
                units[index].target = None;
                units[index].progress = outputs[index].progress.clone();
                points[index].position = destination;
                grid.move_point(index, unit.position, destination);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    destination,
                    unit.radius,
                    &slot_points,
                    &slot_refs,
                    &slot_grid,
                    &mut pool_indexes,
                    &units,
                    &points,
                    &grid,
                    &assigned_slots,
                    max_radius,
                );
                continue;
            }
            let desired_distance = desired.length();
            let mut velocity = if desired_distance > 1.0e-8 {
                desired * (step_limit / desired_distance)
            } else {
                Vec2::ZERO
            };
            let mut separated = false;
            let reach = unit.radius + max_radius + 0.05 + max_step;
            for other_index in grid.within(&points, unit.position, reach) {
                if other_index == index {
                    continue;
                }
                let other = &units[other_index];
                let offset = unit.position - other.position;
                let separation = offset.length();
                let limit = unit.radius + other.radius + 0.05;
                if separation > 1.0e-8 && separation < limit {
                    separated = true;
                    velocity += offset / separation * (limit - separation);
                    if offset.dot(desired) < 0.0 {
                        let along = desired.try_normalize().unwrap_or(Vec2::X);
                        velocity += Vec2::new(along.y, -along.x) * (limit - separation);
                    }
                }
            }
            let mut position = if !separated && desired_distance > step_limit {
                step(
                    &MovementInput {
                        position: unit.position,
                        target: destination,
                        speed,
                    },
                    dt,
                )
                .position
            } else {
                if velocity.length() > step_limit {
                    velocity = velocity.normalize() * step_limit;
                }
                unit.position + velocity
            };
            let mut was_projected = false;
            for _ in 0..8 {
                let mut projected = false;
                for other_index in
                    grid.within(&points, position, unit.radius + max_radius + max_step)
                {
                    if other_index == index {
                        continue;
                    }
                    let other = &units[other_index];
                    let offset = position - other.position;
                    let separation = offset.length();
                    let limit = unit.radius + other.radius + 1.0e-4;
                    if separation < limit {
                        let direction = if separation > 1.0e-8 {
                            offset / separation
                        } else if unit.unit_id < other.unit_id {
                            Vec2::X
                        } else {
                            -Vec2::X
                        };
                        position += direction * (limit - separation);
                        projected = true;
                    }
                }
                if !projected {
                    break;
                }
                was_projected = true;
            }
            position.x = position
                .x
                .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
            position.y = position
                .y
                .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));
            // Arrival eligibility was checked before movement, so this step
            // may end early only if it reaches the destination exactly.
            let reached = position == destination;
            if reached && destination_free {
                position = destination;
            }
            outputs[index].position = position;
            outputs[index].target = (!(reached && destination_free)).then_some(destination);
            if (separated || was_projected) && !(reached && destination_free) {
                let mut plain = make_plain_progress(&unit, target, &plain_spread_context);
                plain.slot = destination;
                plain.best_distance = unit.position.distance(destination);
                let next_distance = position.distance(destination);
                if next_distance < plain.best_distance - 0.01 * step_limit {
                    plain.best_distance = next_distance;
                } else {
                    plain.stalled_ticks = 1;
                }
                outputs[index].plain_progress = Some(plain);
            }
            update_target_claims(&mut target_claims, unit.target, outputs[index].target);
            units[index].position = position;
            units[index].target = outputs[index].target;
            units[index].progress = outputs[index].progress.clone();
            points[index].position = position;
            grid.move_point(index, unit.position, position);
            refresh_nearby_pool_slot_clearances(
                unit.position,
                position,
                unit.radius,
                &slot_points,
                &slot_refs,
                &slot_grid,
                &mut pool_indexes,
                &units,
                &points,
                &grid,
                &assigned_slots,
                max_radius,
            );
            continue;
        };
        let mut movement_target = target;
        let mut failed_blocked = false;
        let mut position = unit.position;
        let distance = position.distance(movement_target);
        if distance <= speed * dt || distance <= 0.025 {
            let occupied = grid
                .within(&points, movement_target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    let other = &units[other_index];
                    other_index != index
                        && movement_target.distance(other.position)
                            < unit.radius + other.radius - 1.0e-5
                });
            if !occupied {
                update_target_claims(&mut target_claims, unit.target, None);
                outputs[index].position = movement_target;
                outputs[index].target = None;
                progress.activity = MovementActivity::Resting;
                outputs[index].progress = Some(progress);
                units[index].position = movement_target;
                units[index].target = None;
                units[index].progress = outputs[index].progress.clone();
                points[index].position = movement_target;
                grid.move_point(index, unit.position, movement_target);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    movement_target,
                    unit.radius,
                    &slot_points,
                    &slot_refs,
                    &slot_grid,
                    &mut pool_indexes,
                    &units,
                    &points,
                    &grid,
                    &assigned_slots,
                    max_radius,
                );
                continue;
            }
        }

        let desired = movement_target - position;
        let mut velocity = desired.try_normalize().unwrap_or(Vec2::ZERO) * speed * dt;
        let reach = unit.radius + max_radius + 0.05 + max_step;
        let neighbours = grid.within(&points, position, reach);
        let mut near_same_group = false;
        let mut separated = false;
        for other_index in neighbours {
            if other_index == index {
                continue;
            }
            let other = &units[other_index];
            let same_group = unit.owner_slot == other.owner_slot
                && unit
                    .progress
                    .as_ref()
                    .is_some_and(|state| state.pool_id == progress.pool_id)
                && other
                    .progress
                    .as_ref()
                    .is_some_and(|state| state.pool_id == progress.pool_id);
            let other_moving_in_group = same_group && other.target.is_some();
            if other_moving_in_group
                && position.distance(other.position) < unit.radius + other.radius + 0.05
            {
                near_same_group = true;
            }
            if same_group && (!other_moving_in_group || progress.pass_mode) {
                continue;
            }
            let offset = position - other.position;
            let separation = offset.length();
            let limit = unit.radius + other.radius + 0.05;
            if separation > 1.0e-8 && separation < limit {
                separated = true;
                let push = (limit - separation) * offset / separation;
                velocity += push;
                if !same_group && offset.dot(desired) < 0.0 {
                    let along = desired.try_normalize().unwrap_or(Vec2::X);
                    velocity += Vec2::new(along.y, -along.x) * (limit - separation);
                }
            }
        }
        let velocity_len = velocity.length();
        let step_limit = speed * dt;
        if !separated && distance > step_limit {
            position = step(
                &MovementInput {
                    position: unit.position,
                    target: movement_target,
                    speed,
                },
                dt,
            )
            .position;
        } else {
            if velocity_len > step_limit && velocity_len > 0.0 {
                velocity *= step_limit / velocity_len;
            }
            position += velocity;
        }

        // Hard projection prevents footprint overlap. Units resolve in id order.
        let broad_reach = unit.radius + max_radius + max_step;
        for _ in 0..8 {
            let mut projected = false;
            for other_index in grid.within(&points, position, broad_reach) {
                if other_index == index {
                    continue;
                }
                let other = &units[other_index];
                let same_group = unit.owner_slot == other.owner_slot
                    && other
                        .progress
                        .as_ref()
                        .is_some_and(|state| state.pool_id == progress.pool_id);
                if same_group {
                    continue;
                }
                let offset = position - other.position;
                let separation = offset.length();
                let limit = unit.radius + other.radius + 1.0e-4;
                if separation < limit {
                    let direction = if separation > 1.0e-8 {
                        offset / separation
                    } else if unit.unit_id < other.unit_id {
                        Vec2::X
                    } else {
                        -Vec2::X
                    };
                    position += direction * (limit - separation);
                    projected = true;
                }
            }
            if !projected {
                break;
            }
        }
        position.x = position
            .x
            .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
        position.y = position
            .y
            .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));

        let next_distance = position.distance(movement_target);
        if position == movement_target {
            let occupied = grid
                .within(&points, movement_target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    let other = &units[other_index];
                    other_index != index
                        && movement_target.distance(other.position)
                            < unit.radius + other.radius - 1.0e-5
                });
            if !occupied {
                update_target_claims(&mut target_claims, unit.target, None);
                outputs[index].position = movement_target;
                outputs[index].target = None;
                progress.activity = MovementActivity::Resting;
                outputs[index].progress = Some(progress);
                units[index].position = movement_target;
                units[index].target = None;
                units[index].progress = outputs[index].progress.clone();
                points[index].position = movement_target;
                grid.move_point(index, unit.position, movement_target);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    movement_target,
                    unit.radius,
                    &slot_points,
                    &slot_refs,
                    &slot_grid,
                    &mut pool_indexes,
                    &units,
                    &points,
                    &grid,
                    &assigned_slots,
                    max_radius,
                );
                continue;
            }
        }
        let made_progress = next_distance < progress.best_distance - 0.01 * step_limit;
        progress.stalling_on_group = near_same_group;
        if made_progress {
            progress.best_distance = next_distance;
            progress.stalled_ticks = 0;
        } else {
            progress.stalled_ticks = progress.stalled_ticks.saturating_add(1);
        }
        let hz = frame.tick_hz.max(1);
        if progress.stalled_ticks >= hz {
            if progress.stalling_on_group && !progress.pass_mode {
                progress.pass_mode = true;
                progress.stalled_ticks = 0;
                progress.best_distance = next_distance;
            } else if progress.claim_count < 3 {
                let old_slot_key = (
                    progress.pool_id,
                    progress.slot.x.to_bits(),
                    progress.slot.y.to_bits(),
                );
                let spare = pool_indexes.get(&progress.pool_id).and_then(|pool_index| {
                    pool_index.nearest_free(
                        unit.unit_id,
                        position,
                        progress.pool_id,
                        None,
                        &assigned_slots,
                        unit.radius,
                        |slot| {
                            let key = (progress.pool_id, slot.x.to_bits(), slot.y.to_bits());
                            key != old_slot_key
                                && grid
                                    .within(&points, slot, unit.radius + max_radius)
                                    .into_iter()
                                    .all(|other_index| {
                                        other_index == index
                                            || slot.distance(units[other_index].position)
                                                >= unit.radius + units[other_index].radius
                                    })
                        },
                    )
                });
                if let Some(spare) = spare {
                    if remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        progress.slot,
                    ) {
                        assigned_slots.remove(&old_slot_key);
                    }
                    add_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        spare,
                    );
                    assigned_slots.insert((progress.pool_id, spare.x.to_bits(), spare.y.to_bits()));
                    movement_target = spare;
                    progress.slot = spare;
                    progress.claim_count += 1;
                    progress.pass_mode = false;
                    progress.stalled_ticks = 0;
                    progress.best_distance = position.distance(movement_target);
                } else {
                    progress.activity = MovementActivity::Blocked;
                    failed_blocked = true;
                }
            } else {
                progress.activity = MovementActivity::Blocked;
                failed_blocked = true;
            }
        }
        outputs[index].position = position;
        outputs[index].target = (!failed_blocked).then_some(movement_target);
        update_target_claims(&mut target_claims, unit.target, outputs[index].target);
        outputs[index].failed_blocked = failed_blocked;
        if failed_blocked
            && body_overlaps_position(
                position,
                index,
                unit.radius,
                &units,
                &points,
                &grid,
                max_radius,
            )
        {
            outputs[index].physical_escape = Some(PhysicalEscape { target: None });
        }
        outputs[index].progress = Some(progress);
        units[index].position = position;
        units[index].target = outputs[index].target;
        units[index].progress = outputs[index].progress.clone();
        points[index].position = position;
        grid.move_point(index, unit.position, position);
        refresh_nearby_pool_slot_clearances(
            unit.position,
            position,
            unit.radius,
            &slot_points,
            &slot_refs,
            &slot_grid,
            &mut pool_indexes,
            &units,
            &points,
            &grid,
            &assigned_slots,
            max_radius,
        );
    }
    let moving_pools: std::collections::BTreeSet<_> = outputs
        .iter()
        .filter_map(|output| {
            (output.target.is_some()
                || output
                    .progress
                    .as_ref()
                    .is_some_and(|state| state.activity == MovementActivity::Blocked))
            .then(|| output.progress.as_ref().map(|progress| progress.pool_id))
            .flatten()
        })
        .collect();
    for output in &mut outputs {
        if output.progress.as_ref().is_some_and(|progress| {
            progress.activity == MovementActivity::Resting
                && !moving_pools.contains(&progress.pool_id)
        }) {
            output.progress = None;
        }
    }
    outputs
}

/// FixedUpdate: move each unit with a `MoveTarget` by `speed * delta` toward its target.
/// A unit that is within one step of its target lands on it and loses the `MoveTarget`.
pub(crate) fn integrate_movement(world: &mut World) {
    let delta = world.resource::<Time<Fixed>>().delta_secs();
    let types = world.resource::<UnitTypes>();
    let mut units: Vec<_> = world
        .iter_entities()
        .filter_map(|entity| {
            let unit_type = entity.get::<UnitType>()?;
            let position = entity.get::<Position>()?;
            let entry = types.get(unit_type.0)?;
            Some((
                entity
                    .get::<crate::world::UnitId>()
                    .map_or(u32::MAX, |id| id.0),
                entity.id(),
                crate::movement::MovementFrameUnit {
                    unit_id: entity
                        .get::<crate::world::UnitId>()
                        .map_or(u32::MAX, |id| id.0),
                    owner_slot: entity
                        .get::<crate::world::Owner>()
                        .map_or(0, |owner| owner.0.0),
                    unit_type: unit_type.0.0,
                    position: position.0,
                    radius: entry.radius,
                    speed: entry.movement.as_ref().map(|movement| movement.speed),
                    target: entity.get::<MoveTarget>().map(|target| target.0),
                    progress: entity.get::<MoveProgress>().cloned(),
                    plain_progress: entity.get::<PlainMoveProgress>().cloned(),
                    physical_escape: entity.get::<PhysicalEscape>().cloned(),
                },
            ))
        })
        .collect();
    units.sort_unstable_by_key(|(id, ..)| *id);
    let bounds = world
        .get_resource::<crate::world::MapBounds>()
        .map_or(Vec2::ZERO, |bounds| bounds.size);
    let tick_hz = world
        .get_resource::<crate::SimClock>()
        .map_or(20, |clock| clock.tick_hz);
    let frame = MovementFrame {
        bounds,
        tick_hz,
        units: units.iter().map(|(_, _, unit)| unit.clone()).collect(),
        pools: world.resource::<MovementPools>().0.clone(),
    };
    let outputs = step_frame(&frame, delta);
    for ((_, entity, _), output) in units.into_iter().zip(outputs) {
        let mut entity_mut = world.entity_mut(entity);
        if let Some(target) = output.target {
            entity_mut.insert(MoveTarget(target));
        } else {
            entity_mut.remove::<MoveTarget>();
        }
        if let Some(mut position) = entity_mut.get_mut::<Position>() {
            position.set_if_neq(Position(output.position));
        }
        if let Some(progress) = output.progress {
            entity_mut.insert(progress);
        } else {
            entity_mut.remove::<MoveProgress>();
        }
        if let Some(progress) = output.plain_progress {
            entity_mut.insert(progress);
        } else {
            entity_mut.remove::<PlainMoveProgress>();
        }
        if let Some(escape) = output.physical_escape {
            entity_mut.insert(escape);
        } else {
            entity_mut.remove::<PhysicalEscape>();
        }
    }
    let active_pools: std::collections::BTreeSet<_> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<MoveProgress>()
                .map(|progress| progress.pool_id)
        })
        .collect();
    world
        .resource_mut::<MovementPools>()
        .0
        .retain(|pool_id, _| active_pools.contains(pool_id));
}

#[cfg(test)]
mod spread_tests {
    use super::{
        MoveProgress, MovementActivity, MovementFrame, MovementFrameUnit, MovementInput,
        MovementPool, MovementPoolIndex, PlainMoveProgress, SpreadMember, StaticFootprint, angle,
        spread_destinations, step, step_frame,
    };
    use bevy_math::Vec2;
    use std::collections::BTreeMap;

    #[test]
    fn progressless_singleton_move_never_spends_two_steps_in_one_tick() {
        let target = Vec2::new(22.0, 10.0);
        let frame = MovementFrame {
            bounds: Vec2::new(64.0, 64.0),
            tick_hz: 20,
            units: vec![MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(21.2, 10.0),
                radius: 0.5,
                speed: Some(8.0),
                target: Some(target),
                progress: None,

                plain_progress: None,
                physical_escape: None,
            }],
            pools: BTreeMap::new(),
        };
        let output = step_frame(&frame, 0.05).remove(0);
        assert!((output.position.x - 21.6).abs() < 1.0e-5);
        assert_eq!(output.position.y, 10.0);
        assert_eq!(output.target, Some(target));
        assert!((output.position.distance(frame.units[0].position) - 0.4).abs() < 1.0e-5);
    }

    #[test]
    fn progressless_singleton_move_keeps_the_exact_free_step_before_arrival() {
        let target = Vec2::new(22.0, 10.0);
        let frame = MovementFrame {
            bounds: Vec2::new(64.0, 64.0),
            tick_hz: 20,
            units: vec![MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(21.577, 10.0),
                radius: 0.5,
                speed: Some(8.0),
                target: Some(target),
                progress: None,

                plain_progress: None,
                physical_escape: None,
            }],
            pools: BTreeMap::new(),
        };
        let output = step_frame(&frame, 0.05).remove(0);
        let expected = step(
            &MovementInput {
                position: frame.units[0].position,
                target,
                speed: 8.0,
            },
            0.05,
        );
        assert_eq!(output.position, expected.position);
        assert_eq!(output.target, Some(target));
        assert!(output.position.distance(frame.units[0].position) <= 0.4);
    }

    #[test]
    fn active_group_progress_does_not_add_a_post_step_arrival_snap() {
        let target = Vec2::new(2.0, 1.0);
        let position = Vec2::new(1.83, 1.0);
        let frame = MovementFrame {
            bounds: Vec2::splat(16.0),
            tick_hz: 20,
            units: vec![MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position,
                radius: 0.5,
                speed: Some(3.0),
                target: Some(target),
                progress: Some(MoveProgress {
                    pool_id: 1,
                    slot: target,
                    activity: MovementActivity::Moving,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: position.distance(target),
                    claim_count: 0,

                    follow_origin: None,
                }),
                plain_progress: None,
                physical_escape: None,
            }],
            pools: BTreeMap::new(),
        };

        let output = step_frame(&frame, 0.05).remove(0);
        let expected = step(
            &MovementInput {
                position,
                target,
                speed: 3.0,
            },
            0.05,
        )
        .position;

        assert_eq!(output.position, expected);
        assert_eq!(output.target, Some(target));
        assert_eq!(output.progress.unwrap().activity, MovementActivity::Moving);
    }

    #[test]
    fn contact_correction_does_not_add_a_post_step_arrival_snap() {
        let target = Vec2::new(5.0, 5.0);
        let position = Vec2::new(4.83, 5.0);
        let modes = [
            (None, None),
            (
                Some(MoveProgress {
                    pool_id: 1,
                    slot: target,
                    activity: MovementActivity::Moving,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: position.distance(target),
                    claim_count: 0,

                    follow_origin: None,
                }),
                None,
            ),
            (
                None,
                Some(PlainMoveProgress {
                    original_point: target,
                    slot: target,
                    pool_slots: vec![],
                    gather_goal: None,
                    activity: MovementActivity::Moving,
                    stalled_ticks: 0,
                    best_distance: position.distance(target),
                    claim_count: 1,
                    failure_recorded: false,
                }),
            ),
        ];

        for (progress, plain_progress) in modes {
            let frame = MovementFrame {
                bounds: Vec2::splat(16.0),
                tick_hz: 20,
                units: vec![
                    MovementFrameUnit {
                        unit_id: 1,
                        owner_slot: 0,
                        unit_type: 1,
                        position,
                        radius: 0.5,
                        speed: Some(3.0),
                        target: Some(target),
                        progress,
                        plain_progress,

                        physical_escape: None,
                    },
                    MovementFrameUnit {
                        unit_id: 2,
                        owner_slot: 1,
                        unit_type: 2,
                        position: Vec2::new(3.81, 5.0),
                        radius: 0.5,
                        speed: None,
                        target: None,
                        progress: None,
                        plain_progress: None,
                        physical_escape: None,
                    },
                ],
                pools: BTreeMap::new(),
            };

            let output = step_frame(&frame, 0.05).remove(0);
            assert!(
                output.position.distance(position) <= 0.150_001,
                "contact movement exceeded one step: {:?}",
                output.position
            );
            assert_eq!(output.target, Some(target));
            assert_ne!(output.position, target);
        }
    }

    #[test]
    fn pool_has_primary_slots_and_ceiling_spares() {
        for count in [1_usize, 7, 8, 28] {
            let members: Vec<_> = (0..count)
                .map(|id| SpreadMember {
                    id: id as u32 + 1,
                    position: Vec2::new(10.0 + id as f32, 10.0),
                    radius: 0.5,
                })
                .collect();
            let (pool, assignments) = spread_destinations(
                &members,
                Vec2::new(32.0, 32.0),
                &[],
                &[],
                Vec2::new(64.0, 64.0),
            );
            assert_eq!(pool.primary_count, count as u32);
            assert_eq!(pool.slots.len(), count + 6_usize.max(count.div_ceil(4)));
            assert_eq!(assignments.len(), count);
        }
    }

    #[test]
    fn pool_spacing_uses_the_largest_unit_radius() {
        let members = [
            SpreadMember {
                id: 1,
                position: Vec2::new(10.0, 10.0),
                radius: 0.2,
            },
            SpreadMember {
                id: 2,
                position: Vec2::new(11.0, 10.0),
                radius: 0.7,
            },
        ];
        let (pool, _) = spread_destinations(
            &members,
            Vec2::new(32.0, 32.0),
            &[],
            &[],
            Vec2::new(64.0, 64.0),
        );
        assert_eq!(pool.spacing, 1.5);
        for (index, slot) in pool.slots.iter().enumerate() {
            for other in &pool.slots[index + 1..] {
                assert!(slot.distance(*other) >= pool.spacing - 1.0e-5);
            }
        }
    }

    #[test]
    fn pool_spacing_uses_small_actual_radii_without_a_half_unit_floor() {
        let members = [SpreadMember {
            id: 1,
            position: Vec2::new(4.0, 4.0),
            radius: 0.05,
        }];
        let (pool, _) = spread_destinations(
            &members,
            Vec2::new(32.0, 32.0),
            &[],
            &[],
            Vec2::new(64.0, 64.0),
        );
        assert!((pool.spacing - 0.2).abs() < 1.0e-6);
        assert!(pool.slots[0].distance(pool.slots[1]) >= 0.2 - 1.0e-6);
    }

    #[test]
    fn pool_starts_at_origin_then_orders_equal_radius_by_angle() {
        let member = [SpreadMember {
            id: 1,
            position: Vec2::new(4.0, 4.0),
            radius: 0.5,
        }];
        let (pool, assignments) = spread_destinations(
            &member,
            Vec2::new(32.0, 32.0),
            &[],
            &[],
            Vec2::new(64.0, 64.0),
        );
        assert_eq!(pool.slots.len(), 7);
        assert_eq!(pool.slots[0], Vec2::new(32.0, 32.0));
        let s = 1.1_f32;
        let expected = [
            Vec2::new(32.0 - 0.5 * s, 32.0 - 3.0_f32.sqrt() * 0.5 * s),
            Vec2::new(32.0 + 0.5 * s, 32.0 - 3.0_f32.sqrt() * 0.5 * s),
            Vec2::new(32.0 + s, 32.0),
            Vec2::new(32.0 + 0.5 * s, 32.0 + 3.0_f32.sqrt() * 0.5 * s),
            Vec2::new(32.0 - 0.5 * s, 32.0 + 3.0_f32.sqrt() * 0.5 * s),
            Vec2::new(32.0 - s, 32.0),
        ];
        for (actual, expected) in pool.slots[1..].iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 1.0e-5 && (actual.y - expected.y).abs() < 1.0e-5,
                "actual {actual:?} expected {expected:?}"
            );
        }
        assert_eq!(assignments, vec![(1, Vec2::new(32.0, 32.0))]);
    }

    #[test]
    fn large_pool_enumeration_is_nearest_first_and_complete() {
        let members: Vec<_> = (0..200)
            .map(|id| SpreadMember {
                id: id + 1,
                position: Vec2::new(10.0 + id as f32 * 0.01, 10.0),
                radius: 0.5,
            })
            .collect();
        let (pool, assignments) = spread_destinations(
            &members,
            Vec2::new(32.0, 32.0),
            &[],
            &[],
            Vec2::new(64.0, 64.0),
        );
        assert_eq!(pool.slots.len(), 250);
        assert_eq!(assignments.len(), 200);
        assert_eq!(pool.slots[0], Vec2::new(32.0, 32.0));
        let keys: Vec<_> = pool
            .slots
            .iter()
            .map(|slot| {
                let delta = *slot - Vec2::new(32.0, 32.0);
                let ring_key =
                    (delta.length_squared() / (pool.spacing * pool.spacing)).round() as i64;
                let angle = angle(*slot, Vec2::new(32.0, 32.0));
                (ring_key, angle)
            })
            .collect();
        assert!(keys.windows(2).all(|pair| {
            pair[0].0 < pair[1].0 || (pair[0].0 == pair[1].0 && pair[0].1 <= pair[1].1 + 1.0e-5)
        }));
    }

    #[test]
    fn pool_order_matches_independent_axial_lattice_reference_at_medium_and_large_sizes() {
        let center = Vec2::new(32.0, 32.0);
        let spacing = 1.1_f32;
        for member_count in [135_u32, 1_000] {
            let members: Vec<_> = (0..member_count)
                .map(|id| SpreadMember {
                    id: id + 1,
                    position: Vec2::new(10.0 + id as f32 * 0.001, 12.0),
                    radius: 0.5,
                })
                .collect();
            let spare_count = 6_usize.max(members.len().div_ceil(4));
            let wanted = members.len() + spare_count;
            let mut reference = vec![(0_i64, 0_i64, center)];
            for i in -64_i32..=64 {
                for j in -64_i32..=64 {
                    let ring = i.abs().max(j.abs()).max((i + j).abs());
                    if ring == 0 {
                        continue;
                    }
                    let position = Vec2::new(
                        center.x + spacing * (i as f32 + 0.5 * j as f32),
                        center.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
                    );
                    if position.x < 0.5
                        || position.y < 0.5
                        || position.x > 63.5
                        || position.y > 63.5
                    {
                        continue;
                    }
                    let ring_key = i64::from(i) * i64::from(i)
                        + i64::from(i) * i64::from(j)
                        + i64::from(j) * i64::from(j);
                    let angle_key = ((position.y - center.y).atan2(position.x - center.x)
                        * 1_000_000.0)
                        .round() as i64;
                    reference.push((ring_key, angle_key, position));
                }
            }
            reference.sort_by_key(|(ring_key, angle_key, _)| (*ring_key, *angle_key));
            let expected: Vec<_> = reference
                .into_iter()
                .take(wanted)
                .map(|(_, _, position)| position)
                .collect();

            let (pool, assignments) =
                spread_destinations(&members, center, &[], &[], Vec2::new(64.0, 64.0));
            assert_eq!(pool.slots.len(), wanted, "member count {member_count}");
            assert_eq!(
                pool.primary_count, member_count,
                "member count {member_count}"
            );
            assert_eq!(assignments.len(), member_count as usize);
            let mismatch = pool
                .slots
                .iter()
                .zip(&expected)
                .position(|(actual, expected)| actual != expected);
            if let Some(index) = mismatch {
                let start = index.saturating_sub(2);
                let end = (index + 3).min(expected.len());
                panic!(
                    "member count {member_count}, first mismatch {index}; actual={:?}; expected={:?}",
                    &pool.slots[start..end],
                    &expected[start..end]
                );
            }
        }
    }

    #[test]
    fn static_footprint_is_skipped_and_assignments_repeat_bit_exactly() {
        let members: Vec<_> = (0..20)
            .map(|id| SpreadMember {
                id: id as u32 + 1,
                position: Vec2::new(10.0 + id as f32 * 0.5, 10.0),
                radius: 0.5,
            })
            .collect();
        let obstacle = [StaticFootprint {
            position: Vec2::new(32.0, 32.0),
            radius: 2.0,
        }];
        let a = spread_destinations(
            &members,
            Vec2::new(32.0, 32.0),
            &[],
            &obstacle,
            Vec2::new(64.0, 64.0),
        );
        let b = spread_destinations(
            &members,
            Vec2::new(32.0, 32.0),
            &[],
            &obstacle,
            Vec2::new(64.0, 64.0),
        );
        assert_eq!(a, b);
        assert!(
            a.0.slots
                .iter()
                .all(|slot| slot.distance(obstacle[0].position) >= 2.5)
        );
    }

    #[test]
    fn blocked_escape_chooses_nearest_free_point_when_old_pool_is_far_and_occupied() {
        let origin = Vec2::new(10.0, 10.0);
        let old_slot = Vec2::new(50.0, 50.0);
        let pool = MovementPool {
            pool_id: 7,
            point: old_slot,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![old_slot],
        };
        let frame = MovementFrame {
            bounds: Vec2::new(64.0, 64.0),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position: origin,
                    radius: 0.5,
                    speed: Some(3.0),
                    target: Some(old_slot),
                    progress: Some(MoveProgress {
                        pool_id: 7,
                        slot: old_slot,
                        activity: MovementActivity::Blocked,
                        pass_mode: true,
                        stalling_on_group: false,
                        stalled_ticks: 60,
                        best_distance: 1.0,
                        claim_count: 3,

                        follow_origin: None,
                    }),

                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 1,
                    unit_type: 1,
                    position: origin,
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,

                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 3,
                    owner_slot: 1,
                    unit_type: 1,
                    position: old_slot,
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,

                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::from([(7, pool)]),
        };
        let output = step_frame(&frame, 0.05);
        let mover = &output[0];
        let escape = mover.target.expect("the blocked escape remains explicit");
        assert_ne!(escape, old_slot, "the blocked old pool point is occupied");
        assert!(
            escape.distance(origin) <= 1.1 + 1.0e-5,
            "nearest legal free point is local: {escape:?}"
        );
        assert!(
            mover.position.distance(origin) <= 0.15 + 1.0e-5,
            "escape advances at most one speed step"
        );
    }

    #[test]
    fn no_spare_pool_blocks_on_the_first_other_group_stall_deadline() {
        let target = Vec2::new(11.0, 10.0);
        let pool = MovementPool {
            pool_id: 8,
            point: target,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![target],
        };
        let frame = MovementFrame {
            bounds: Vec2::new(64.0, 64.0),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position: Vec2::new(10.0, 10.0),
                    radius: 0.5,
                    speed: Some(3.0),
                    target: Some(target),
                    progress: Some(MoveProgress {
                        pool_id: 8,
                        slot: target,
                        activity: MovementActivity::Moving,
                        pass_mode: false,
                        stalling_on_group: false,
                        stalled_ticks: 19,
                        best_distance: 1.0,
                        claim_count: 0,

                        follow_origin: None,
                    }),

                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 1,
                    unit_type: 1,
                    position: Vec2::new(10.2, 10.0),
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,

                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::from([(8, pool)]),
        };

        let mut before_deadline = frame.clone();
        before_deadline.units[0]
            .progress
            .as_mut()
            .unwrap()
            .stalled_ticks = 18;
        let before = step_frame(&before_deadline, 0.05).remove(0);
        assert!(
            !before.failed_blocked,
            "the unit has not stalled for one second yet"
        );
        assert_eq!(before.target, Some(target));

        let mover = step_frame(&frame, 0.05).remove(0);

        assert!(mover.failed_blocked);
        assert_eq!(mover.target, None);
        assert_eq!(
            mover.progress.as_ref().map(|progress| progress.activity),
            Some(MovementActivity::Blocked)
        );
    }

    #[test]
    fn exhausted_claims_become_blocked_at_the_one_second_stall_deadline() {
        let point = Vec2::new(10.0, 10.0);
        let pool = MovementPool {
            pool_id: 9,
            point,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![point],
        };
        let frame = MovementFrame {
            bounds: Vec2::new(64.0, 64.0),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position: point,
                    radius: 0.5,
                    speed: Some(3.0),
                    target: Some(point),
                    progress: Some(MoveProgress {
                        pool_id: 9,
                        slot: point,
                        activity: MovementActivity::Moving,
                        pass_mode: true,
                        stalling_on_group: false,
                        stalled_ticks: 19,
                        best_distance: 0.0,
                        claim_count: 3,

                        follow_origin: None,
                    }),

                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 0,
                    unit_type: 1,
                    position: point,
                    radius: 0.5,
                    speed: Some(3.0),
                    target: None,
                    progress: Some(MoveProgress {
                        pool_id: 9,
                        slot: point,
                        activity: MovementActivity::Resting,
                        pass_mode: false,
                        stalling_on_group: false,
                        stalled_ticks: 0,
                        best_distance: 0.0,
                        claim_count: 0,

                        follow_origin: None,
                    }),

                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::from([(9, pool)]),
        };
        let output = step_frame(&frame, 0.05);
        assert!(output[0].failed_blocked);
        assert_eq!(output[0].target, None);
        assert_eq!(
            output[0]
                .progress
                .as_ref()
                .map(|progress| progress.activity),
            Some(MovementActivity::Blocked)
        );
    }

    #[test]
    fn zero_available_pool_slots_skip_repeated_grid_queries() {
        let pool_id = 9;
        let slots: Vec<_> = (0..5_000_u32)
            .map(|index| Vec2::new(index as f32 + 1.0, 1.0))
            .collect();
        let pool = MovementPool {
            pool_id,
            point: Vec2::ZERO,
            spacing: 1.1,
            primary_count: 5_000,
            slots: slots.clone(),
        };
        let index = MovementPoolIndex::new(&pool, Vec2::new(10_000.0, 10_000.0));
        let assigned: std::collections::HashSet<_> = slots
            .iter()
            .map(|point| (pool_id, point.x.to_bits(), point.y.to_bits()))
            .collect();
        let is_free_calls = std::sync::atomic::AtomicUsize::new(0);
        for _ in 0..5_000 {
            assert_eq!(
                index.nearest_free(1, Vec2::ZERO, pool_id, None, &assigned, 0.5, |_| {
                    is_free_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    true
                },),
                None
            );
        }
        assert_eq!(is_free_calls.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[test]
    fn nearest_free_checks_the_final_reach_interval() {
        let pool = MovementPool {
            pool_id: 4,
            point: Vec2::ZERO,
            spacing: 1.0,
            primary_count: 1,
            slots: vec![Vec2::new(300.0, 0.0)],
        };
        let mut index = MovementPoolIndex::new(&pool, Vec2::new(300.0, 300.0));
        index.clearances[0] = super::SlotClearance::empty();
        index.available_by_clearance.insert(super::ClearanceRank {
            clearance: f64::INFINITY,
            slot_index: 0,
        });
        index.available_slot_indices.insert(0);
        assert_eq!(
            index.nearest_free(
                1,
                Vec2::ZERO,
                pool.pool_id,
                None,
                &std::collections::HashSet::new(),
                0.5,
                |_| true,
            ),
            Some(Vec2::new(300.0, 0.0))
        );
    }

    #[test]
    fn unavailable_spares_are_not_hidden_by_an_assigned_free_primary() {
        let pool = MovementPool {
            pool_id: 9,
            point: Vec2::ZERO,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(11.0, 0.0)],
        };
        let mut index = MovementPoolIndex::new(&pool, Vec2::new(20.0, 20.0));
        let units = vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(5.0, 0.0),
                radius: 0.5,
                speed: Some(3.0),
                target: Some(Vec2::new(12.0, 0.0)),
                progress: None,

                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 99,
                owner_slot: 1,
                unit_type: 2,
                position: Vec2::new(10.5, 0.0),
                radius: 1.0,
                speed: None,
                target: None,
                progress: None,

                plain_progress: None,
                physical_escape: None,
            },
        ];
        let points = units
            .iter()
            .map(|unit| crate::grid::GridPoint {
                id: unit.unit_id,
                position: unit.position,
            })
            .collect::<Vec<_>>();
        let grid = crate::grid::UniformGrid::new(&points, 2.0);
        let assigned = std::collections::HashSet::from([(
            pool.pool_id,
            Vec2::ZERO.x.to_bits(),
            Vec2::ZERO.y.to_bits(),
        )]);
        for slot_index in 0..index.points.len() {
            let point = index.points[slot_index].position;
            index.refresh_slot_clearance(
                slot_index,
                &units,
                &points,
                &grid,
                1.0,
                assigned.contains(&(pool.pool_id, point.x.to_bits(), point.y.to_bits())),
            );
        }
        assert!(!index.has_potential_free_slot(1, units[0].position, 0.5, None));
    }

    #[test]
    fn step_frame_rejects_fully_occupied_spares_without_pool_wide_stalled_scans() {
        let count = 64_usize;
        let bounds = Vec2::splat(512.0);
        let center = Vec2::splat(256.0);
        let starts = (0..count)
            .map(|index| {
                Vec2::new(
                    10.0 + (index % 8) as f32 * 3.0,
                    10.0 + (index / 8) as f32 * 3.0,
                )
            })
            .collect::<Vec<_>>();
        let members = starts
            .iter()
            .enumerate()
            .map(|(index, &position)| SpreadMember {
                id: index as u32 + 1,
                position,
                radius: [0.2, 0.3, 0.5][index % 3],
            })
            .collect::<Vec<_>>();
        let (pool, assignments) = spread_destinations(&members, center, &[], &[], bounds);
        let target_by_id = assignments.iter().copied().collect::<BTreeMap<_, _>>();
        assert_eq!(pool.primary_count, count as u32);
        assert_eq!(pool.slots.len(), count + count.div_ceil(4));
        let assigned_keys = assignments
            .iter()
            .map(|(_, target)| (target.x.to_bits(), target.y.to_bits()))
            .collect::<std::collections::HashSet<_>>();
        let primary_keys = pool.slots[..count]
            .iter()
            .map(|target| (target.x.to_bits(), target.y.to_bits()))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(assigned_keys, primary_keys);

        let mut units = Vec::new();
        for index in 0..count {
            let unit_id = index as u32 + 1;
            let position = starts[index];
            let radius = [0.2, 0.3, 0.5][index % 3];
            let target = target_by_id[&unit_id];
            let direction = (target - position).normalize();
            let blocker_position = position + direction * (radius + 0.5001);
            assert!((radius + 0.5..radius + 0.502).contains(&position.distance(blocker_position)));
            units.push(MovementFrameUnit {
                unit_id,
                owner_slot: 0,
                unit_type: index as u32,
                position,
                radius,
                speed: Some(0.05),
                target: Some(target),
                progress: Some(MoveProgress {
                    pool_id: 9,
                    slot: target,
                    activity: MovementActivity::Moving,
                    pass_mode: true,
                    stalling_on_group: false,
                    stalled_ticks: 19,
                    best_distance: position.distance(target),
                    claim_count: 0,

                    follow_origin: None,
                }),

                plain_progress: None,
                physical_escape: None,
            });
            units.push(MovementFrameUnit {
                unit_id: 10_000 + unit_id,
                owner_slot: 1,
                unit_type: 4,
                position: blocker_position,
                radius: 0.5,
                speed: None,
                target: None,
                progress: None,

                plain_progress: None,
                physical_escape: None,
            });
        }
        for (index, &point) in pool.slots.iter().skip(count).enumerate() {
            units.push(MovementFrameUnit {
                unit_id: 20_000 + index as u32,
                owner_slot: 1,
                unit_type: 4,
                position: point,
                radius: 0.5,
                speed: Some(3.0),
                target: None,
                progress: Some(MoveProgress {
                    pool_id: 77,
                    slot: point,
                    activity: MovementActivity::Resting,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: 0.0,
                    claim_count: 0,

                    follow_origin: None,
                }),

                plain_progress: None,
                physical_escape: None,
            });
        }
        for (left_index, left) in units.iter().enumerate() {
            for right in &units[left_index + 1..] {
                assert!(
                    left.position.distance(right.position) + 1.0e-5 >= left.radius + right.radius,
                    "invalid overlapping fixture units {} and {}",
                    left.unit_id,
                    right.unit_id
                );
            }
        }
        assert!(pool.slots[..count].iter().all(|slot| {
            units
                .iter()
                .all(|unit| slot.distance(unit.position) + 1.0e-5 >= 0.5 + unit.radius)
        }));
        let frame = MovementFrame {
            bounds,
            tick_hz: 20,
            units,
            pools: BTreeMap::from([(9, pool)]),
        };
        super::POOL_SLOT_CANDIDATE_CHECKS.with(|checks| checks.set(0));
        let output = step_frame(&frame, 0.05);
        let inspected = super::POOL_SLOT_CANDIDATE_CHECKS.with(std::cell::Cell::get);
        assert!(
            output[..count].iter().all(|row| row.failed_blocked),
            "stalled outputs: {:?}",
            &output[..3]
        );
        assert!(
            inspected < count,
            "{count} stalled units inspected {inspected} pool slot candidates despite every spare being occupied"
        );
        eprintln!("canonical N={count} occupied-spare candidate inspections={inspected}");
    }
}

#[cfg(test)]
mod plain_movement_tests {
    use super::{
        MovementActivity, MovementFrame, MovementFrameUnit, MovementInput, PlainMoveProgress, step,
        step_frame,
    };
    use bevy_math::Vec2;
    use std::collections::BTreeMap;

    fn stalled_plain_frame(pool_slots: Vec<Vec2>) -> MovementFrame {
        let point = Vec2::new(20.0, 10.0);
        MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position: Vec2::new(10.0, 10.0),
                    radius: 0.5,
                    speed: Some(0.05),
                    target: Some(point),
                    progress: None,
                    plain_progress: Some(PlainMoveProgress {
                        original_point: point,
                        slot: point,
                        pool_slots,
                        gather_goal: None,
                        activity: MovementActivity::Moving,
                        stalled_ticks: 19,
                        best_distance: 10.0,
                        claim_count: 0,
                        failure_recorded: false,
                    }),

                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 1,
                    unit_type: 2,
                    position: Vec2::new(11.0001, 10.0),
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::new(),
        }
    }

    #[test]
    fn unobstructed_step_keeps_the_legacy_target_until_within_one_step() {
        let position = Vec2::new(22.25925, 9.274589);
        let target = Vec2::new(22.386324, 9.181545);
        let speed = 3.0;
        let dt = 0.05;
        let frame = MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            units: vec![MovementFrameUnit {
                unit_id: 3,
                owner_slot: 0,
                unit_type: 1,
                position,
                radius: 0.5,
                speed: Some(speed),
                target: Some(target),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            }],
            pools: BTreeMap::new(),
        };

        let expected = step(
            &MovementInput {
                position,
                target,
                speed,
            },
            dt,
        )
        .position;
        let output = step_frame(&frame, dt);

        assert_eq!(output[0].position, expected);
        assert_eq!(output[0].target, Some(target));
    }

    #[test]
    fn retained_plain_progress_does_not_add_a_post_step_arrival_snap() {
        let target = Vec2::new(2.0, 1.0);
        let position = Vec2::new(1.83, 1.0);
        let frame = MovementFrame {
            bounds: Vec2::splat(16.0),
            tick_hz: 20,
            units: vec![MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position,
                radius: 0.5,
                speed: Some(3.0),
                target: Some(target),
                progress: None,
                plain_progress: Some(PlainMoveProgress {
                    original_point: target,
                    slot: target,
                    pool_slots: vec![],
                    gather_goal: None,
                    activity: MovementActivity::Moving,
                    stalled_ticks: 0,
                    best_distance: position.distance(target),
                    claim_count: 1,
                    failure_recorded: false,
                }),

                physical_escape: None,
            }],
            pools: BTreeMap::new(),
        };

        let output = step_frame(&frame, 0.05).remove(0);
        let expected = step(
            &MovementInput {
                position,
                target,
                speed: 3.0,
            },
            0.05,
        )
        .position;

        assert_eq!(output.position, expected);
        assert_eq!(output.target, Some(target));
        assert_eq!(
            output.plain_progress.unwrap().activity,
            MovementActivity::Moving
        );
    }

    #[test]
    fn plain_move_respects_reserved_spacing_before_using_a_zero_primary_point() {
        let target = Vec2::new(0.5, 1.2);
        let frame = MovementFrame {
            bounds: Vec2::splat(2.0),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position: Vec2::new(0.5, 0.5),
                    radius: 0.5,
                    speed: Some(3.0),
                    target: Some(target),
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 1,
                    unit_type: 1,
                    position: Vec2::new(1.5, 1.5),
                    radius: 0.5,
                    speed: Some(3.0),
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::new(),
        };

        let output = step_frame(&frame, 0.05);

        assert_eq!(output[0].position, Vec2::new(0.5, 0.5));
        assert_eq!(output[0].target, None);
        let progress = output[0]
            .plain_progress
            .as_ref()
            .expect("reserved spacing activates the plain fallback");
        assert_eq!(progress.activity, MovementActivity::Blocked);
        assert!(progress.pool_slots.is_empty());
    }

    #[test]
    fn blocked_plain_unit_does_not_escape_for_reservation_spacing_alone() {
        let position = Vec2::new(10.0, 10.0);
        let frame = MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position,
                    radius: 0.5,
                    speed: Some(3.0),
                    target: None,
                    progress: None,
                    plain_progress: Some(PlainMoveProgress {
                        original_point: Vec2::new(20.0, 10.0),
                        slot: position,
                        pool_slots: vec![],
                        gather_goal: None,
                        activity: MovementActivity::Blocked,
                        stalled_ticks: 20,
                        best_distance: 10.0,
                        claim_count: 3,
                        failure_recorded: true,
                    }),

                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 1,
                    unit_type: 1,
                    position: Vec2::new(11.05, 10.0),
                    radius: 0.5,
                    speed: Some(3.0),
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::new(),
        };

        let output = step_frame(&frame, 0.05);

        assert_eq!(output[0].position, position);
        assert_eq!(output[0].target, None);
        assert_eq!(
            output[0].plain_progress.as_ref().unwrap().activity,
            MovementActivity::Blocked
        );
    }

    #[test]
    fn plain_stall_claims_nearest_spare_and_never_requeues_a_used_slot() {
        let frame = stalled_plain_frame(vec![Vec2::new(13.0, 10.0), Vec2::new(15.0, 10.0)]);
        let first = step_frame(&frame, 0.05).remove(0);
        let first_progress = first.plain_progress.unwrap();
        assert_eq!(first.target, Some(Vec2::new(13.0, 10.0)));
        assert_eq!(first_progress.slot, Vec2::new(13.0, 10.0));
        assert_eq!(first_progress.claim_count, 1);
        assert_eq!(first_progress.pool_slots, vec![Vec2::new(15.0, 10.0)]);

        let mut second_frame = frame;
        second_frame.units[0].position = first.position;
        second_frame.units[0].target = first.target;
        second_frame.units[0].plain_progress = Some(PlainMoveProgress {
            stalled_ticks: 19,
            best_distance: first.position.distance(Vec2::new(13.0, 10.0)),
            ..first_progress
        });
        let second = step_frame(&second_frame, 0.05).remove(0);
        let second_progress = second.plain_progress.unwrap();
        assert_eq!(second.target, Some(Vec2::new(15.0, 10.0)));
        assert_eq!(second_progress.slot, Vec2::new(15.0, 10.0));
        assert_eq!(second_progress.claim_count, 2);
        assert!(!second_progress.pool_slots.contains(&Vec2::new(20.0, 10.0)));
    }

    #[test]
    fn plain_move_without_a_spare_blocks_on_the_first_one_second_deadline() {
        let frame = stalled_plain_frame(Vec::new());
        let output = step_frame(&frame, 0.05).remove(0);
        let progress = output.plain_progress.unwrap();
        assert_eq!(progress.activity, MovementActivity::Blocked);
        assert_eq!(progress.stalled_ticks, 20);
        assert_eq!(output.target, None);
        assert!(output.failed_blocked);
    }

    #[test]
    fn independent_plain_pools_cannot_claim_the_same_reserved_spare() {
        let shared_spare = Vec2::new(13.0, 15.0);
        let make_mover = |unit_id, position: Vec2, target: Vec2, blocker: Vec2| {
            vec![
                MovementFrameUnit {
                    unit_id,
                    owner_slot: unit_id as u8,
                    unit_type: unit_id,
                    position,
                    radius: 0.5,
                    speed: Some(0.05),
                    target: Some(target),
                    progress: None,
                    plain_progress: Some(PlainMoveProgress {
                        original_point: target,
                        slot: target,
                        pool_slots: vec![shared_spare],
                        gather_goal: None,
                        activity: MovementActivity::Moving,
                        stalled_ticks: 19,
                        best_distance: position.distance(target),
                        claim_count: 0,
                        failure_recorded: false,
                    }),

                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: unit_id + 10,
                    owner_slot: 2,
                    unit_type: 20,
                    position: blocker,
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
            ]
        };
        let frame = MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            units: [
                make_mover(
                    1,
                    Vec2::new(10.0, 10.0),
                    Vec2::new(20.0, 10.0),
                    Vec2::new(11.0001, 10.0),
                ),
                make_mover(
                    2,
                    Vec2::new(10.0, 20.0),
                    Vec2::new(20.0, 20.0),
                    Vec2::new(11.0001, 20.0),
                ),
            ]
            .concat(),
            pools: BTreeMap::new(),
        };

        let output = step_frame(&frame, 0.05);
        assert_eq!(output[0].target, Some(shared_spare));
        assert_eq!(output[0].plain_progress.as_ref().unwrap().claim_count, 1);
        assert_eq!(output[1].target, None);
        assert_eq!(
            output[1].plain_progress.as_ref().unwrap().activity,
            MovementActivity::Blocked
        );
    }

    #[test]
    fn plain_move_with_zero_legal_primary_slots_blocks_before_moving() {
        let start = Vec2::new(0.5, 0.5);
        let frame = MovementFrame {
            bounds: Vec2::splat(1.5),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position: start,
                    radius: 0.5,
                    speed: Some(3.0),
                    target: Some(Vec2::new(1.0, 1.0)),
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 1,
                    unit_type: 2,
                    position: Vec2::new(1.2, 1.2),
                    radius: 0.2,
                    speed: None,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::new(),
        };

        let output = step_frame(&frame, 0.05).remove(0);
        let progress = output.plain_progress.unwrap();
        assert_eq!(progress.activity, MovementActivity::Blocked);
        assert_eq!(progress.slot, Vec2::new(1.0, 1.0));
        assert!(progress.pool_slots.is_empty());
        assert_eq!(output.position, start);
        assert_eq!(output.target, None);
    }
}
