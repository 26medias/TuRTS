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
            .within_unsorted(
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
    // The axial lattice site of each entry of `slots`, and true when a footprint closed a site.
    let mut sites: Vec<(i32, i32)> = Vec::with_capacity(wanted);
    let mut obstructed = false;
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
        sites.push((0, 0));
        nearest_indices.insert((0_i64, 0_i64, slots.len() - 1));
        // The origin is slot zero in the canonical nearest-first hex pool.
        pool_points.push(crate::grid::GridPoint {
            id: 0,
            position: point,
        });
        pool_grid = crate::grid::UniformGrid::new(&pool_points, spacing);
    } else {
        obstructed |= inside_bounds(point, max_radius, context);
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
                    sites.push((i, j));
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
                } else {
                    obstructed |= inside_bounds(candidate, max_radius, context);
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
    if members.len() == 1 && slots.first() != Some(&point) {
        let nearest_hex = slots
            .iter()
            .copied()
            .min_by(|a, b| nearest_angle_order(*a, *b, point));
        if let Some(nearest) =
            nearest_singleton(&members[0], point, context, excluded_units, nearest_hex)
        {
            // A singleton has no formation to preserve. Keep its primary destination
            // on the nearest physical clearance boundary rather than a coarse hex ring.
            // Spare destinations still keep the existing group spacing.
            slots.sort_by(|a, b| {
                a.distance_squared(point)
                    .total_cmp(&b.distance_squared(point))
                    .then_with(|| angle(*a, point).total_cmp(&angle(*b, point)))
            });
            let mut refined = Vec::with_capacity(wanted);
            refined.push(nearest);
            for candidate in slots {
                if refined
                    .iter()
                    .all(|other: &Vec2| other.distance(candidate) >= spacing - 1.0e-5)
                {
                    refined.push(candidate);
                    if refined.len() == wanted {
                        break;
                    }
                }
            }
            return (
                MovementPool {
                    pool_id: 0,
                    point,
                    spacing,
                    primary_count: 1,
                    slots: refined,
                },
                vec![(members[0].id, nearest)],
            );
        }
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
    let reachable = if obstructed && members.len() > 1 {
        reachable_pool(
            ReachInput {
                point,
                radius: max_radius,
                spacing,
                context,
                excluded_units,
                first_ring: ring.min(ring_limit),
                ring_limit,
            },
            &sites,
            members.len(),
            wanted,
        )
    } else {
        None
    };
    slots = reachable.unwrap_or_else(|| {
        nearest_indices
            .iter()
            .map(|(_, _, index)| slots[*index])
            .collect()
    });

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

#[derive(Clone, Copy)]
struct ArrivalCircle {
    x: f64,
    y: f64,
    radius: f64,
}
fn singleton_free(
    point: Vec2,
    radius: f32,
    context: &SpreadContext,
    excluded: &std::collections::HashSet<u32>,
) -> bool {
    if !point.is_finite()
        || point.x < radius
        || point.y < radius
        || point.x > context.bounds.x - radius
        || point.y > context.bounds.y - radius
    {
        return false;
    }
    let clear = |centre: Vec2, other: f32| {
        let dx = f64::from(point.x) - f64::from(centre.x);
        let dy = f64::from(point.y) - f64::from(centre.y);
        let sum = f64::from(radius) + f64::from(other);
        dx * dx + dy * dy >= sum * sum && point.distance(centre) >= radius + other
    };
    let reach = |other: f32| (radius + other).next_up().next_up();
    context
        .static_grid
        .within_unsorted(
            &context.static_points,
            point,
            reach(context.max_static_radius),
        )
        .into_iter()
        .all(|i| {
            clear(
                context.static_footprints[i].position,
                context.static_footprints[i].radius,
            )
        })
        && context
            .reserved_grid
            .within_unsorted(
                &context.reserved_points,
                point,
                reach(context.max_reserved_radius),
            )
            .into_iter()
            .all(|i| {
                context.reserved_units[i].is_some_and(|id| excluded.contains(&id))
                    || clear(context.reserved[i].position, context.reserved[i].radius)
            })
}
fn consider_singleton(
    best: &mut Option<Vec2>,
    candidate: [f64; 2],
    goal: Vec2,
    radius: f32,
    context: &SpreadContext,
    excluded: &std::collections::HashSet<u32>,
) {
    let x = candidate[0] as f32;
    let y = candidate[1] as f32;
    // The analytic point may round inward. Test its adjacent representable poses;
    // every accepted pose is checked against all actual nearby bodies, without
    // introducing a new collision epsilon or reserving somebody else's destination.
    for x in [x.next_down(), x, x.next_up()] {
        for y in [y.next_down(), y, y.next_up()] {
            let point = Vec2::new(x, y);
            let order = |a: Vec2, b: Vec2| {
                let distance = |p: Vec2| {
                    let dx = f64::from(p.x) - f64::from(goal.x);
                    let dy = f64::from(p.y) - f64::from(goal.y);
                    dx * dx + dy * dy
                };
                distance(a).total_cmp(&distance(b)).then_with(|| {
                    (f64::from(a.y) - f64::from(goal.y))
                        .atan2(f64::from(a.x) - f64::from(goal.x))
                        .total_cmp(
                            &(f64::from(b.y) - f64::from(goal.y))
                                .atan2(f64::from(b.x) - f64::from(goal.x)),
                        )
                })
            };
            if best.is_some_and(|best| order(point, best).is_ge()) {
                continue;
            }
            if singleton_free(point, radius, context, excluded) {
                *best = Some(point);
            }
        }
    }
}
/// The closest point outside a union of clearance disks is either a radial
/// projection, an intersection of two disk boundaries, or a map-edge boundary.
/// The existing free hex destination supplies a local search upper bound.
fn nearest_singleton(
    member: &SpreadMember,
    goal: Vec2,
    context: &SpreadContext,
    excluded: &std::collections::HashSet<u32>,
    hex: Option<Vec2>,
) -> Option<Vec2> {
    if singleton_free(goal, member.radius, context, excluded) {
        return Some(goal);
    }
    let mut best = None;
    if let Some(point) = hex {
        consider_singleton(
            &mut best,
            [f64::from(point.x), f64::from(point.y)],
            goal,
            member.radius,
            context,
            excluded,
        );
    }
    let low = f64::from(member.radius);
    let high = [
        f64::from(context.bounds.x - member.radius),
        f64::from(context.bounds.y - member.radius),
    ];
    if high[0] < low || high[1] < low {
        return None;
    }
    for x in [low, high[0]] {
        for y in [low, high[1]] {
            consider_singleton(&mut best, [x, y], goal, member.radius, context, excluded);
        }
    }
    let distance =
        |p: Vec2| (f64::from(p.x) - f64::from(goal.x)).hypot(f64::from(p.y) - f64::from(goal.y));
    let limit = best.map_or_else(
        || {
            [
                Vec2::new(low as f32, low as f32),
                Vec2::new(high[0] as f32, high[1] as f32),
            ]
            .into_iter()
            .map(distance)
            .fold(0.0, f64::max)
        },
        distance,
    );
    let reach = |other: f32| {
        ((limit + f64::from(member.radius) + f64::from(other)) as f32)
            .next_up()
            .next_up()
    };
    let mut circles = Vec::new();
    for i in context.static_grid.within_unsorted(
        &context.static_points,
        goal,
        reach(context.max_static_radius),
    ) {
        let body = context.static_footprints[i];
        circles.push(ArrivalCircle {
            x: f64::from(body.position.x),
            y: f64::from(body.position.y),
            radius: f64::from(member.radius) + f64::from(body.radius),
        });
    }
    for i in context.reserved_grid.within_unsorted(
        &context.reserved_points,
        goal,
        reach(context.max_reserved_radius),
    ) {
        if context.reserved_units[i].is_some_and(|id| excluded.contains(&id)) {
            continue;
        }
        let body = context.reserved[i];
        circles.push(ArrivalCircle {
            x: f64::from(body.position.x),
            y: f64::from(body.position.y),
            radius: f64::from(member.radius) + f64::from(body.radius),
        });
    }
    circles.sort_by(|a, b| {
        a.x.total_cmp(&b.x)
            .then(a.y.total_cmp(&b.y))
            .then(a.radius.total_cmp(&b.radius))
    });
    circles.dedup_by(|a, b| a.x == b.x && a.y == b.y && a.radius == b.radius);
    for (index, circle) in circles.iter().enumerate() {
        let mut dx = f64::from(goal.x) - circle.x;
        let mut dy = f64::from(goal.y) - circle.y;
        if dx == 0.0 && dy == 0.0 {
            dx = f64::from(member.position.x) - circle.x;
            dy = f64::from(member.position.y) - circle.y;
        }
        if dx == 0.0 && dy == 0.0 {
            dx = 1.0;
        }
        let length = dx.hypot(dy);
        consider_singleton(
            &mut best,
            [
                circle.x + dx / length * circle.radius,
                circle.y + dy / length * circle.radius,
            ],
            goal,
            member.radius,
            context,
            excluded,
        );
        for x in [low, high[0]] {
            let square = circle.radius * circle.radius - (x - circle.x) * (x - circle.x);
            if square >= 0.0 {
                let h = square.sqrt();
                for y in [circle.y - h, circle.y + h] {
                    consider_singleton(&mut best, [x, y], goal, member.radius, context, excluded);
                }
            }
        }
        for y in [low, high[1]] {
            let square = circle.radius * circle.radius - (y - circle.y) * (y - circle.y);
            if square >= 0.0 {
                let h = square.sqrt();
                for x in [circle.x - h, circle.x + h] {
                    consider_singleton(&mut best, [x, y], goal, member.radius, context, excluded);
                }
            }
        }
        for other in &circles[..index] {
            let dx = other.x - circle.x;
            let dy = other.y - circle.y;
            let d = dx.hypot(dy);
            if d == 0.0
                || d > circle.radius + other.radius
                || d < (circle.radius - other.radius).abs()
            {
                continue;
            }
            let along =
                (circle.radius * circle.radius - other.radius * other.radius + d * d) / (2.0 * d);
            let height = (circle.radius * circle.radius - along * along)
                .max(0.0)
                .sqrt();
            let mid = [circle.x + dx / d * along, circle.y + dy / d * along];
            for sign in [-1.0, 1.0] {
                consider_singleton(
                    &mut best,
                    [
                        mid[0] - dy / d * height * sign,
                        mid[1] + dx / d * height * sign,
                    ],
                    goal,
                    member.radius,
                    context,
                    excluded,
                );
            }
        }
    }
    best
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
            .within_unsorted(
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
            .within_unsorted(
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
            .within_unsorted(pool_points, candidate, radius * 2.0 + 0.1)
            .into_iter()
            .all(|i| candidate.distance(already_in_pool[i]) >= radius * 2.0 + 0.1 - 1.0e-5)
}

fn inside_bounds(candidate: Vec2, radius: f32, context: &SpreadContext) -> bool {
    candidate.is_finite()
        && candidate.x >= radius
        && candidate.y >= radius
        && candidate.x <= context.bounds.x - radius
        && candidate.y <= context.bounds.y - radius
}

struct ReachInput<'a> {
    point: Vec2,
    radius: f32,
    spacing: f32,
    context: &'a SpreadContext,
    excluded_units: &'a std::collections::HashSet<u32>,
    /// The last ring that the free-site scan examined.
    first_ring: i32,
    ring_limit: i32,
}

/// The rings that the reachability flood adds outside the last ring that gives a slot.
const REACH_MARGIN_RINGS: i32 = 2;

/// The nearest free lattice sites that a body can reach from outside the pool.
///
/// A footprint closes each site that it denies. The gap between two resting neighbours is
/// narrower than a body, so a body steps between two adjacent open sites only if one of the
/// two sites beside that step is open as well (the same gate rule as the claim exchange). The
/// flood starts at the open sites of the outer ring and goes inward. A free site that the
/// flood does not reach is in a pocket that footprints seal: no body can come to rest there.
///
/// The result has the canonical nearest-first order. It is `None` when the reachable sites are
/// fewer than the members; the caller then keeps the unfiltered pool.
fn reachable_pool(
    input: ReachInput<'_>,
    sites: &[(i32, i32)],
    members: usize,
    wanted: usize,
) -> Option<Vec<Vec2>> {
    const SIDES: [(i32, i32); 6] = [(1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)];
    let position = |(i, j): (i32, i32)| {
        Vec2::new(
            input.point.x + input.spacing * (i as f32 + 0.5 * j as f32),
            input.point.y + input.spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
        )
    };
    let hex_ring = |(i, j): (i32, i32)| i.abs().max(j.abs()).max((i + j).abs());
    let mut open: std::collections::HashMap<(i32, i32), bool> =
        sites.iter().map(|site| (*site, true)).collect();
    let scanned = input.first_ring;
    let no_pool = crate::grid::UniformGrid::new(&[], input.spacing);
    let mut is_open = |site: (i32, i32)| {
        if hex_ring(site) <= scanned {
            // The scan examined each site of these rings: a site not in `sites` is closed.
            return open.get(&site).copied().unwrap_or(false);
        }
        *open.entry(site).or_insert_with(|| {
            free_slot_grid(
                position(site),
                input.radius,
                input.context,
                input.excluded_units,
                &[],
                &[],
                &no_pool,
            )
        })
    };
    let key = |site: (i32, i32)| {
        let (i, j) = (i64::from(site.0), i64::from(site.1));
        let angle_key = if site == (0, 0) {
            0
        } else {
            (angle(position(site), input.point) * 1_000_000.0).round() as i64
        };
        (i * i + i * j + j * j, angle_key)
    };
    let mut outer = (scanned + REACH_MARGIN_RINGS).min(input.ring_limit + REACH_MARGIN_RINGS);
    loop {
        // Flood from the open sites of the outer ring over the rings inside it.
        let mut seen = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        for side in 0..6 {
            for step in 0..outer {
                let site = match side {
                    0 => (outer - step, step),
                    1 => (-step, outer),
                    2 => (-outer, outer - step),
                    3 => (-outer + step, -step),
                    4 => (step, -outer),
                    _ => (outer, -outer + step),
                };
                if is_open(site) && seen.insert(site) {
                    queue.push_back(site);
                }
            }
        }
        while let Some(site) = queue.pop_front() {
            for side in 0..6 {
                let step = |side: usize| (site.0 + SIDES[side].0, site.1 + SIDES[side].1);
                let next = step(side);
                if hex_ring(next) > outer
                    || seen.contains(&next)
                    || !is_open(next)
                    || !(is_open(step((side + 5) % 6)) || is_open(step((side + 1) % 6)))
                {
                    continue;
                }
                seen.insert(next);
                queue.push_back(next);
            }
        }
        let mut reached: Vec<_> = seen
            .iter()
            .filter(|site| hex_ring(**site) <= outer - REACH_MARGIN_RINGS)
            .map(|site| (key(*site), *site))
            .collect();
        reached.sort_unstable();
        reached.truncate(wanted);
        let inner = i64::from(outer - REACH_MARGIN_RINGS + 1);
        let minimum_unseen_ring_key = (3 * inner * inner + 3) / 4;
        let complete = reached.len() == wanted
            && reached
                .last()
                .is_some_and(|((ring_key, _), _)| *ring_key < minimum_unseen_ring_key);
        if complete || outer - REACH_MARGIN_RINGS >= input.ring_limit {
            return (reached.len() >= members).then(|| {
                reached
                    .into_iter()
                    .map(|(_, site)| position(site))
                    .collect()
            });
        }
        outer += 1;
    }
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
