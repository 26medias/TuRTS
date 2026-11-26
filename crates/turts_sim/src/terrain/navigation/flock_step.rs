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
    // The terrain check uses a body that is `CLEARANCE` wider than the true body. So every
    // committed pose keeps that distance from terrain, and a later step of the true body
    // never starts at a contact, where the exact kernel answers "uncertain".
    step(
        terrain,
        from,
        to,
        radius,
        radius + CLEARANCE,
        profile,
        solids,
    )
}

/// A step for a body that stands so near terrain that `guarded_step` refuses every step:
/// a glancing approach can end at the contact of the wider check body. The step is checked
/// with the true body, and it is taken only when its end pose again keeps the clearance.
/// So the pose moves away from the contact, and the true body never enters terrain.
pub(super) fn escape_step(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    solids: &[Solid],
) -> Vec2 {
    let end = step(terrain, from, to, radius, radius, profile, solids);
    let wide = radius + CLEARANCE;
    if end != from && geometry::clear_now(terrain, end, end, wide, profile) {
        return end;
    }
    let stands_clear = |body: f32| geometry::clear_now(terrain, from, from, body, profile);
    if stands_clear(radius) {
        return from;
    }
    // The pose itself is not permitted (a body that was made too near the map edge or a
    // cliff). The largest body that is permitted at the pose makes the step: the true body
    // goes no deeper into terrain, and it can go back to permitted ground.
    // The check needs a body larger than zero.
    let (mut low, mut high) = (radius / 64.0, radius);
    if !stands_clear(low) {
        return from;
    }
    for _ in 0..8 {
        let middle = 0.5 * (low + high);
        if stands_clear(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    // The step is taken whole or not at all, so the body never ends at a new contact.
    let whole = geometry::clear_now(terrain, from, to, low, profile)
        && solids.iter().all(|solid| {
            pair_clear(
                from,
                to,
                radius,
                solid.position,
                solid.position,
                solid.radius,
            )
        });
    if whole { to } else { from }
}

/// A whole step to a lattice node for a body that exactly fits an opening: it has no
/// clearance there. The step is taken, whole, only when the exact check of the true body
/// permits all of it; else the body stays.
pub(super) fn fitted_step(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    solids: &[Solid],
) -> Vec2 {
    let clear = geometry::clear_now(terrain, from, to, radius, profile)
        && solids.iter().all(|solid| {
            pair_clear(
                from,
                to,
                radius,
                solid.position,
                solid.position,
                solid.radius,
            )
        });
    if clear { to } else { from }
}

fn step(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    wide: f32,
    profile: Traversal,
    solids: &[Solid],
) -> Vec2 {
    // A whole clear step needs one certificate; only a refused step is bisected.
    let mut end = if geometry::clear_now(terrain, from, to, wide, profile) {
        to
    } else {
        geometry::physical_prefix(terrain, from, to, wide, profile)
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
/// Distance that every committed pose keeps from terrain.
pub(super) const CLEARANCE: f32 = 1.0 / 512.0;
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
    use std::collections::BTreeMap;
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
            end.x + RADIUS > limit - 0.004,
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

    /// A body that stands nearer the map edge than its radius (a pose that no step made)
    /// goes back to permitted ground, and it goes no nearer the edge.
    #[test]
    fn a_body_too_near_the_map_edge_goes_back_inside() {
        let map = Map::new(0);
        let profile = walker();
        let from = Vec2::new(4.06, 0.35);
        let inward = Vec2::new(3.88, 0.59);
        assert_eq!(guarded_step(&map, from, inward, 0.6, profile, &[]), from);
        assert_eq!(escape_step(&map, from, inward, 0.6, profile, &[]), inward);
        let outward = Vec2::new(4.06, 0.2);
        assert_eq!(escape_step(&map, from, outward, 0.6, profile, &[]), from);
    }
}
