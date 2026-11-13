//! Spread destinations, and the helpers that spread and plain progress share.

use bevy_math::Vec2;

use super::{MovementPool, ReservedFootprint, SpreadMember, StaticFootprint, angle};

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

    pub(super) fn conflicts_with_reserved(
        &self,
        candidate: Vec2,
        radius: f32,
        excluded_unit: u32,
    ) -> bool {
        self.reserved_grid
            .within(
                &self.reserved_points,
                candidate,
                radius + self.max_reserved_radius,
            )
            .into_iter()
            .any(|index| {
                self.reserved_units[index] != Some(excluded_unit)
                    && candidate.distance(self.reserved[index].position)
                        < radius + self.reserved[index].radius - 1.0e-5
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
    let mut spatial_members = members.iter().collect::<Vec<_>>();
    spatial_members.sort_by(|a, b| {
        a.position
            .x
            .total_cmp(&b.position.x)
            .then(a.position.y.total_cmp(&b.position.y))
    });
    let centroid = spatial_members
        .iter()
        .fold(Vec2::ZERO, |sum, unit| sum + unit.position)
        / members.len() as f32;
    let direction = (point - centroid).try_normalize().unwrap_or(Vec2::X);
    let lateral = Vec2::new(-direction.y, direction.x);
    let mut unit_order: Vec<_> = members.iter().collect();
    unit_order.sort_by(|a, b| {
        (b.position.dot(direction))
            .total_cmp(&a.position.dot(direction))
            .then_with(|| a.position.x.total_cmp(&b.position.x))
            .then_with(|| a.position.y.total_cmp(&b.position.y))
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
                .then_with(|| a.position.x.total_cmp(&b.position.x))
                .then_with(|| a.position.y.total_cmp(&b.position.y))
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

pub(super) fn minimum_unseen_hex_radius(ring: i32, spacing: f32) -> f32 {
    let next_ring = i128::from(ring) + 1;
    let minimum_squared = (3 * next_ring * next_ring + 3) / 4;
    (minimum_squared as f64).sqrt() as f32 * spacing
}

pub(super) fn finite_hex_ring_limit(origin: Vec2, radius: f32, bounds: Vec2, spacing: f32) -> i32 {
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

pub(super) fn nearest_angle_order(left: Vec2, right: Vec2, origin: Vec2) -> std::cmp::Ordering {
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

pub(super) fn target_key(target: Vec2) -> (u32, u32) {
    (target.x.to_bits(), target.y.to_bits())
}

pub(super) fn update_target_claims(
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
