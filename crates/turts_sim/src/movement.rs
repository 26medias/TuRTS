//! Movement integrator.

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_math::Vec2;
use bevy_time::{Fixed, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::registry::UnitTypes;
use crate::world::{MoveTarget, Position, UnitType};

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
            let minimum_unseen_ring_key = i64::from(ring + 1) * i64::from(ring + 1);
            if nearest_indices
                .last()
                .is_some_and(|(ring_key, _, _)| *ring_key < minimum_unseen_ring_key)
            {
                break;
            }
        }
        ring += 1;
    }
    if slots.len() < wanted {
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

    let primary = &slots[..members.len().min(slots.len())];
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
        primary_count: members.len().min(slots.len()) as u32,
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
        for ring in 1..=64_i32 {
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
                    if free {
                        return Some(candidate);
                    }
                }
            }
        }
        None
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
        for ring in 1..=128_i32 {
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
                (ring + 1) as f32 * spacing > candidate.distance(origin) + 1.0e-5
            }) {
                return best;
            }
        }
        None
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
    spacing: f32,
}

impl MovementPoolIndex {
    fn new(pool: &MovementPool) -> Self {
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
        Self {
            points,
            grid,
            spacing: pool.spacing.max(0.1),
        }
    }

    fn nearest_free(
        &self,
        origin: Vec2,
        pool_id: u64,
        allowed_slot: (u64, u32, u32),
        assigned_slots: &std::collections::HashSet<(u64, u32, u32)>,
        is_free: impl Fn(Vec2) -> bool,
    ) -> Option<Vec2> {
        let max_radius = 512.0_f32;
        let mut reach = self.spacing;
        while reach <= max_radius {
            let mut best: Option<(usize, Vec2)> = None;
            for index in self.grid.within(&self.points, origin, reach) {
                let point = self.points[index].position;
                let key = (pool_id, point.x.to_bits(), point.y.to_bits());
                if (assigned_slots.contains(&key) && key != allowed_slot) || !is_free(point) {
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
            if let Some((_, point)) = best {
                return Some(point);
            }
            reach *= 2.0;
        }
        None
    }
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
    let pool_indexes: BTreeMap<_, _> = frame
        .pools
        .iter()
        .map(|(&pool_id, pool)| (pool_id, MovementPoolIndex::new(pool)))
        .collect();
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
            failed_blocked: false,
        })
        .collect();
    let mut assigned_slots: std::collections::HashSet<(u64, u32, u32)> = units
        .iter()
        .filter_map(|unit| {
            unit.progress.as_ref().map(|progress| {
                (
                    progress.pool_id,
                    progress.slot.x.to_bits(),
                    progress.slot.y.to_bits(),
                )
            })
        })
        .collect();
    let mut target_claims: std::collections::HashMap<(u32, u32), usize> = units
        .iter()
        .filter_map(|unit| unit.target.map(target_key))
        .fold(std::collections::HashMap::new(), |mut counts, key| {
            *counts.entry(key).or_default() += 1;
            counts
        });

    for index in 0..units.len() {
        let unit = units[index].clone();
        let Some(speed) = unit.speed else { continue };
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
                    && (!assigned_slots.contains(&key) || key == old_slot_key)
            });
            let pool_escape = pool_indexes.get(&progress.pool_id).and_then(|index| {
                index.nearest_free(
                    unit.position,
                    progress.pool_id,
                    old_slot_key,
                    &assigned_slots,
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
                assigned_slots.remove(&old_slot_key);
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
            units[index].target = outputs[index].target;
            units[index].progress = outputs[index].progress.clone();
            continue;
        }
        let Some(target) = unit.target else { continue };
        let Some(mut progress) = unit.progress.clone() else {
            let target_occupied = grid
                .within(&points, target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    other_index != index
                        && target.distance(units[other_index].position)
                            < unit.radius + units[other_index].radius - 1.0e-5
                });
            let destination = if target_occupied {
                MovementSearch {
                    bounds: frame.bounds,
                    units: &units,
                    points: &points,
                    grid: &grid,
                    max_radius,
                }
                .find_free_singleton_destination(target, unit.radius, index)
                .unwrap_or(target)
            } else {
                target
            };
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
            if desired.length() <= step_limit && destination_free {
                outputs[index].position = destination;
                outputs[index].target = None;
                units[index].position = destination;
                units[index].target = None;
                units[index].progress = outputs[index].progress.clone();
                points[index].position = destination;
                grid.move_point(index, unit.position, destination);
                continue;
            }
            let mut velocity = desired.try_normalize().unwrap_or(Vec2::ZERO) * speed * dt;
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
                    velocity += offset / separation * (limit - separation);
                    if offset.dot(desired) < 0.0 {
                        let along = desired.try_normalize().unwrap_or(Vec2::X);
                        velocity += Vec2::new(along.y, -along.x) * (limit - separation);
                    }
                }
            }
            if velocity.length() > step_limit {
                velocity = velocity.normalize() * step_limit;
            }
            let mut position = unit.position + velocity;
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
            }
            position.x = position.x.clamp(0.0, frame.bounds.x);
            position.y = position.y.clamp(0.0, frame.bounds.y);
            let reached = position.distance(destination) <= 0.025;
            if reached && destination_free {
                position = destination;
            }
            outputs[index].position = position;
            outputs[index].target = (!(reached && destination_free)).then_some(destination);
            update_target_claims(&mut target_claims, unit.target, outputs[index].target);
            units[index].position = position;
            units[index].target = outputs[index].target;
            units[index].progress = outputs[index].progress.clone();
            points[index].position = position;
            grid.move_point(index, unit.position, position);
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
                continue;
            }
        }

        let desired = movement_target - position;
        let mut velocity = desired.try_normalize().unwrap_or(Vec2::ZERO) * speed * dt;
        let reach = unit.radius + max_radius + 0.05 + max_step;
        let neighbours = grid.within(&points, position, reach);
        let mut near_same_group = false;
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
        if velocity_len > step_limit && velocity_len > 0.0 {
            velocity *= step_limit / velocity_len;
        }
        position += velocity;

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
        if next_distance <= 0.025 {
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
                if let Some(pool) = frame.pools.get(&progress.pool_id) {
                    if let Some(spare) = pool
                        .slots
                        .iter()
                        .filter(|slot| {
                            !assigned_slots.contains(&(
                                progress.pool_id,
                                slot.x.to_bits(),
                                slot.y.to_bits(),
                            ))
                        })
                        .filter(|slot| {
                            grid.within(&points, **slot, unit.radius + max_radius)
                                .into_iter()
                                .all(|other_index| {
                                    other_index == index
                                        || slot.distance(units[other_index].position)
                                            >= unit.radius + units[other_index].radius
                                })
                        })
                        .min_by(|a, b| {
                            position
                                .distance_squared(**a)
                                .total_cmp(&position.distance_squared(**b))
                        })
                    {
                        movement_target = *spare;
                        assigned_slots.insert((
                            progress.pool_id,
                            spare.x.to_bits(),
                            spare.y.to_bits(),
                        ));
                        progress.slot = *spare;
                        progress.claim_count += 1;
                        progress.pass_mode = false;
                        progress.stalled_ticks = 0;
                        progress.best_distance = position.distance(movement_target);
                    } else {
                        progress.activity = MovementActivity::Blocked;
                        failed_blocked = true;
                    }
                }
            } else if progress.stalled_ticks >= hz.saturating_mul(3) {
                progress.activity = MovementActivity::Blocked;
                failed_blocked = true;
            }
        }
        outputs[index].position = position;
        outputs[index].target = (!failed_blocked).then_some(movement_target);
        update_target_claims(&mut target_claims, unit.target, outputs[index].target);
        outputs[index].failed_blocked = failed_blocked;
        outputs[index].progress = Some(progress);
        units[index].position = position;
        units[index].target = outputs[index].target;
        units[index].progress = outputs[index].progress.clone();
        points[index].position = position;
        grid.move_point(index, unit.position, position);
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
        MoveProgress, MovementActivity, MovementFrame, MovementFrameUnit, MovementPool,
        SpreadMember, StaticFootprint, angle, spread_destinations, step_frame,
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
    fn progressless_singleton_move_allows_only_the_small_landing_snap() {
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
            }],
            pools: BTreeMap::new(),
        };
        let output = step_frame(&frame, 0.05).remove(0);
        assert_eq!(output.position, target);
        assert!(output.target.is_none());
        assert!(output.position.distance(frame.units[0].position) <= 0.425);
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
                    }),
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
    fn exhausted_claims_become_blocked_at_the_three_second_stall_deadline() {
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
                        pass_mode: false,
                        stalling_on_group: false,
                        stalled_ticks: 59,
                        best_distance: 0.0,
                        claim_count: 3,
                    }),
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
                    }),
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
}
