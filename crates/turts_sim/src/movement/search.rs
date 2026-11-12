//! The slot search of one movement step.

use bevy_math::Vec2;

use super::frame::MovementFrameUnit;
use super::spread::{finite_hex_ring_limit, minimum_unseen_hex_radius, nearest_angle_order};

pub(super) struct MovementSearch<'a> {
    pub(super) bounds: Vec2,
    pub(super) units: &'a [MovementFrameUnit],
    pub(super) points: &'a [crate::grid::GridPoint],
    pub(super) grid: &'a crate::grid::UniformGrid,
    pub(super) max_radius: f32,
}

impl MovementSearch<'_> {
    pub(super) fn find_free_singleton_destination(
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

    pub(super) fn find_free_escape_destination(
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

pub(super) fn body_overlaps_position(
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
