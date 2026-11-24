//! Flock physical step guard (T-7, design note T-6 stage C). One body, one short step.
//!
//! The guard is the only authority against solid things: terrain (walls, cliffs, the step
//! and grade rules) and solid bodies (buildings, bodies of another owner, own units on Hold).
//! It reuses the exact predicates: `geometry::physical_prefix` for terrain and
//! `sweep::pair_clear` for one stationary solid body. It uses no swept-pair frame
//! certificate, no budget pool and no other mover.
use super::geometry::{self, TerrainRead};
use crate::movement::sweep::pair_clear;
use bevy_math::Vec2;
use std::collections::BTreeMap;
use turts_rules::terrain::geometry::Traversal;

/// A body that no step may enter. It does not move in this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Solid {
    pub(super) position: Vec2,
    pub(super) radius: f32,
}

/// The farthest point of the segment `from` to `to` that the body can take: the terrain
/// prefix, shortened to the first contact with each solid body. The result is `from` when
/// no part of the step is permitted. The resolution is 2^-20 of the step.
pub(super) fn guarded_step(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    solids: &[Solid],
) -> Vec2 {
    // A whole clear step needs one certificate; only a refused step is bisected.
    let mut end = if geometry::clear(terrain, from, to, radius, profile, &mut BTreeMap::new()) {
        to
    } else {
        geometry::physical_prefix(terrain, from, to, radius, profile)
    };
    for solid in solids {
        end = body_prefix(from, end, radius, *solid);
    }
    if end == to {
        return to;
    }
    // A shortened step stops `MARGIN` before the contact. A pose at the contact itself
    // makes the exact kernel answer "uncertain" for every later step, also away from it.
    let length = from.distance(end);
    if length <= MARGIN {
        from
    } else {
        from + (end - from) * ((length - MARGIN) / length)
    }
}
/// Distance that a shortened step keeps from its contact.
const MARGIN: f32 = 1.0 / 1024.0;

/// The farthest point of `from` to `to` that keeps the body out of one solid body.
fn body_prefix(from: Vec2, to: Vec2, radius: f32, solid: Solid) -> Vec2 {
    let clear = |end: Vec2| {
        pair_clear(
            from,
            end,
            radius,
            solid.position,
            solid.position,
            solid.radius,
        )
    };
    if clear(to) {
        return to;
    }
    let candidate = |part: u32| {
        let fraction = f64::from(part) / f64::from(1u32 << 20);
        Vec2::new(
            (f64::from(from.x) + (f64::from(to.x) - f64::from(from.x)) * fraction) as f32,
            (f64::from(from.y) + (f64::from(to.y) - f64::from(from.y)) * fraction) as f32,
        )
    };
    let (mut low, mut high) = (0u32, 1u32 << 20);
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if clear(candidate(middle)) {
            low = middle;
        } else {
            high = middle;
        }
    }
    let point = candidate(low);
    if clear(point) { point } else { from }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_rules::terrain::{
        Ratio,
        geometry::{Cell, Grid, Surface},
    };

    /// A 16 x 16 map of 1 m cells. Columns with x >= 8 have the height `east`.
    struct Map {
        grid: Grid,
        east: i32,
    }
    impl Map {
        fn new(east: i32) -> Self {
            Self {
                grid: Grid {
                    columns: 16,
                    rows: 16,
                    length_scale: 1000,
                    cell_width: 1000,
                    elevation_step: 250,
                    floor: -8,
                    ceiling: 8,
                },
                east,
            }
        }
    }
    impl Surface for Map {
        fn elevation(&self, cell: Cell) -> Option<i32> {
            (cell.x < 16 && cell.y < 16).then_some(if cell.x >= 8 { self.east } else { 0 })
        }
    }
    impl TerrainRead for Map {
        fn grid(&self) -> Grid {
            self.grid
        }
        fn rectangle(&self, min: Cell, max: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
            let mut height = None;
            for y in min.y..=max.y {
                for x in min.x..=max.x {
                    let Some(value) = self.elevation(Cell { x, y }) else {
                        return false;
                    };
                    lease.insert((y * 16 + x) as usize, 1);
                    if height.is_some_and(|old| old != value) {
                        return false;
                    }
                    height = Some(value);
                }
            }
            true
        }
        fn valid_lease(&self, _: &BTreeMap<usize, u64>) -> bool {
            true
        }
    }
    const RADIUS: f32 = 0.4;
    const FROM: Vec2 = Vec2::new(7.0, 8.5);
    const TO: Vec2 = Vec2::new(7.9, 8.5);
    fn walker() -> Traversal {
        Traversal {
            max_step: 0,
            max_grade: Ratio {
                numerator: 0,
                denominator: 1,
            },
        }
    }
    /// The step went forward, and the body edge did not pass `limit` on the x axis.
    fn stops_before(end: Vec2, limit: f32) {
        assert!(end.x > FROM.x, "no forward motion: {end:?}");
        assert!(
            end.x + RADIUS <= limit,
            "the body passed x = {limit}: {end:?}"
        );
        assert!(
            end.x + RADIUS > limit - 0.002,
            "the stop is not at the obstacle: {end:?}"
        );
        assert_eq!(end.y, FROM.y);
    }

    #[test]
    fn flock_step_on_free_ground_is_taken_whole() {
        assert_eq!(
            guarded_step(&Map::new(0), FROM, TO, RADIUS, walker(), &[]),
            TO
        );
    }
    #[test]
    fn flock_step_stops_at_a_wall() {
        stops_before(
            guarded_step(&Map::new(4), FROM, TO, RADIUS, walker(), &[]),
            8.0,
        );
    }
    #[test]
    fn flock_step_stops_at_a_cliff_edge() {
        stops_before(
            guarded_step(&Map::new(-4), FROM, TO, RADIUS, walker(), &[]),
            8.0,
        );
    }
    #[test]
    fn flock_step_stops_at_a_foreign_body() {
        let body = Solid {
            position: Vec2::new(8.5, 8.5),
            radius: 0.5,
        };
        let end = guarded_step(&Map::new(0), FROM, TO, RADIUS, walker(), &[body]);
        // Contact is at centre distance 0.9, so the body edge stops at x = 8.0.
        stops_before(end, 8.0);
        assert!(end.distance(body.position) >= RADIUS + body.radius);
        assert!(end.distance(body.position) < RADIUS + body.radius + 0.002);
    }
}
