//! Bounded local proposals around known own settled bodies.
//! The caller retains corners and the original order. These proposals neither move
//! holders nor replace the final continuous terrain/body certificate.
use bevy_math::Vec2;
use std::cmp::Ordering;
use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};

pub(super) const MAX_BLOCKERS: usize = 128;
pub(super) const MAX_SCANNED_ROWS: usize = 2 * MAX_BLOCKERS;
const GENERATORS: usize = 8;
pub(super) const MAX_TERRAIN_CALLS: usize = 2 * GENERATORS * 2;
pub(super) const MAX_BODY_CHECKS: usize = MAX_BLOCKERS * (1 + 2 * GENERATORS * 2);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Blocker {
    pub position: Vec2,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Detour {
    pub corners: [Vec2; 2],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Work {
    pub scanned_rows: usize,
    pub generator_comparisons: usize,
    pub candidates: usize,
    pub body_checks: usize,
    pub terrain_calls: usize,
}
#[derive(Clone, Copy)]
struct Generator {
    blocker: Blocker,
    distance: f64,
}
fn physical(a: Generator, b: Generator) -> Ordering {
    a.distance
        .total_cmp(&b.distance)
        .then_with(|| a.blocker.position.x.total_cmp(&b.blocker.position.x))
        .then_with(|| a.blocker.position.y.total_cmp(&b.blocker.position.y))
        .then_with(|| a.blocker.radius.total_cmp(&b.blocker.radius))
}
fn point(p: Vec2) -> Point {
    Point {
        x: f64::from(p.x),
        y: f64::from(p.y),
    }
}
fn body_clear(from: Vec2, to: Vec2, radius: f32, blocker: Blocker, work: &mut Work) -> bool {
    work.body_checks += 1;
    swept_disks_clear(
        point(from),
        point(to),
        f64::from(radius),
        point(blocker.position),
        point(blocker.position),
        f64::from(blocker.radius),
        &mut Budget::new(1),
    )
    .unwrap_or(false)
}
fn corner_order(a: Detour, b: Detour) -> Ordering {
    a.corners[0]
        .x
        .total_cmp(&b.corners[0].x)
        .then_with(|| a.corners[0].y.total_cmp(&b.corners[0].y))
        .then_with(|| a.corners[1].x.total_cmp(&b.corners[1].x))
        .then_with(|| a.corners[1].y.total_cmp(&b.corners[1].y))
}

/// Propose two certified local legs; the requested endpoint is never replaced.
///
/// Supply the complete bounded set of known own settled neighbors. `clear` must
/// certify the whole actor footprint against permitted terrain (unknown is false).
/// It may reject for its existing work budget. `charge_pair` must debit the
/// existing shared owner pair budget before every exact body check. Refusal
/// discards the entire proposal, including previously clear alternatives. The caller retains the selected
/// route through exact corner arrival and certifies actual movement independently.
/// The second corner must be strictly closer to the original endpoint. No full
/// endpoint route is promised. Oversized/invalid input is refused atomically.
///
/// No heap storage; eight inline generators. At most 256 preselection row inspections, 1024
/// generator comparisons, 16 proposals, 4224 disk checks and 32 terrain calls.
/// Work is returned even on refusal so the caller can account for every attempt.
#[cfg(test)]
pub(super) fn propose(
    from: Vec2,
    radius: f32,
    requested: Vec2,
    blockers: &[Blocker],
    clear: impl FnMut(Vec2, Vec2) -> bool,
    charge_pair: impl FnMut() -> bool,
) -> (Option<Detour>, Work) {
    propose_along(
        from,
        radius,
        requested,
        requested,
        blockers,
        clear,
        charge_pair,
    )
}

/// The axis may follow the accepted public cohort goal while every progress
/// check remains tied to the original private endpoint. No endpoint is replaced.
pub(super) fn propose_along(
    from: Vec2,
    radius: f32,
    requested: Vec2,
    axis_goal: Vec2,
    blockers: &[Blocker],
    mut clear: impl FnMut(Vec2, Vec2) -> bool,
    mut charge_pair: impl FnMut() -> bool,
) -> (Option<Detour>, Work) {
    let mut work = Work::default();
    if blockers.len() > MAX_BLOCKERS
        || !from.is_finite()
        || !requested.is_finite()
        || !axis_goal.is_finite()
        || from == axis_goal
        || !radius.is_finite()
        || radius <= 0.0
        || from == requested
    {
        return (None, work);
    }
    // Validate before using any row for a candidate; never truncate a neighbor set.
    for blocker in blockers {
        work.scanned_rows += 1;
        if !blocker.position.is_finite() || !blocker.radius.is_finite() || blocker.radius <= 0.0 {
            return (None, work);
        }
    }
    let offset = requested.as_dvec2() - from.as_dvec2();
    let distance = offset.length();
    let axis_offset = axis_goal.as_dvec2() - from.as_dvec2();
    let axis = axis_offset / axis_offset.length();
    let normal = bevy_math::DVec2::new(-axis.y, axis.x);
    let mut generators = [None; GENERATORS];
    for &blocker in blockers {
        work.scanned_rows += 1;
        if !charge_pair() {
            return (None, work);
        }
        if body_clear(from, requested, radius, blocker, &mut work) {
            continue;
        }
        let along = (blocker.position.as_dvec2() - from.as_dvec2()).dot(axis);
        let candidate = Generator {
            blocker,
            distance: along,
        };
        for index in 0..GENERATORS {
            work.generator_comparisons += 1;
            if generators[index].is_none_or(|old| physical(candidate, old).is_lt()) {
                for next in ((index + 1)..GENERATORS).rev() {
                    generators[next] = generators[next - 1];
                }
                generators[index] = Some(candidate);
                break;
            }
        }
    }
    let mut best: Option<(f64, Detour)> = None;
    for generator in generators.into_iter().flatten() {
        let centre = generator.blocker.position.as_dvec2();
        let relative = from.as_dvec2() - centre;
        let length = relative.length();
        let reach = f64::from(radius) + f64::from(generator.blocker.radius);
        if length < reach || length == 0.0 {
            continue;
        }
        let radial = relative / length;
        // This is proposal slack, not collision tolerance. Exact predicates below
        // check the final f32 corners and reject any rounding-induced penetration.
        let slack = reach * 0.0001 + 0.00001;
        let outside = reach + slack;
        for side in [-1.0_f64, 1.0] {
            work.candidates += 1;
            let mut tangent = bevy_math::DVec2::new(-radial.y, radial.x);
            if tangent.dot(normal) * side < 0.0 {
                tangent = -tangent;
            }
            let denominator = tangent.dot(normal);
            if denominator.abs() < 1e-12 {
                continue;
            }
            let travel =
                (side * outside - relative.dot(normal) - radial.dot(normal) * slack) / denominator;
            if travel < 0.0 {
                continue;
            }
            // First leg leaves the near face along an outward tangent; the second
            // passes outside the disc on the chosen side to its far face.
            let route = Detour {
                corners: [
                    (from.as_dvec2() + radial * slack + tangent * travel).as_vec2(),
                    (centre + axis * outside + normal * (side * outside)).as_vec2(),
                ],
            };
            let [first, second] = route.corners;
            if !first.is_finite() || !second.is_finite() || first == from || second == first {
                continue;
            }
            let remaining = second.as_dvec2().distance(requested.as_dvec2());
            let first_leg = first.as_dvec2().distance(from.as_dvec2());
            let second_leg = second.as_dvec2().distance(first.as_dvec2());
            if remaining >= distance || first_leg > 8.0 * reach || second_leg > 8.0 * reach {
                continue;
            }
            // Evaluate every supplied body for both legs. Early exit would make
            // the reported charge depend on neighbor enumeration.
            let mut bodies_clear = true;
            for &body in blockers {
                if !charge_pair() {
                    return (None, work);
                }
                bodies_clear &= body_clear(from, first, radius, body, &mut work);
                if !charge_pair() {
                    return (None, work);
                }
                bodies_clear &= body_clear(first, second, radius, body, &mut work);
            }
            if !bodies_clear {
                continue;
            }
            work.terrain_calls += 1;
            if !clear(from, first) {
                continue;
            }
            work.terrain_calls += 1;
            if !clear(first, second) {
                continue;
            }
            let score = first_leg + second_leg + remaining;
            if best.is_none_or(|(old_score, old_route)| {
                score
                    .total_cmp(&old_score)
                    .then_with(|| corner_order(route, old_route))
                    .is_lt()
            }) {
                best = Some((score, route));
            }
        }
    }
    (best.map(|(_, route)| route), work)
}

pub(super) const SIDESTEPS: usize = 4;
const RETREATS: usize = 3;
const ADVANCE: f64 = 8.0;
pub(super) const MAX_SIDESTEP_PROPOSALS: usize = 2 * SIDESTEPS * RETREATS;
pub(super) const MAX_SIDESTEP_TERRAIN_CALLS: usize = 2 * MAX_SIDESTEP_PROPOSALS;
pub(super) const MAX_SIDESTEP_BODY_CHECKS: usize = MAX_BLOCKERS * (1 + 2 * MAX_SIDESTEP_PROPOSALS);

/// Second, bounded attempt for a group of standing bodies. `propose_along`
/// passes one disc; it finds nothing when the far side of that disc is another
/// standing body. The first leg steps sideways by one to four reaches and
/// back by zero to two reaches: a body in the notch between two standing
/// bodies can only leave it backward. The second leg advances along the axis
/// past the first leg's start by eight reaches (so it is at most ten reaches
/// long). A corner outside the map is refused. Both legs get the same
/// exact body and terrain certificates as every other detour, and the second
/// corner must be strictly nearer to the original endpoint.
///
/// No heap storage. At most 128 row validations, 24 proposals, 6272 disk
/// checks and 48 terrain calls. A budget refusal discards the whole proposal.
#[allow(
    clippy::too_many_arguments,
    reason = "Geometry and budget inputs stay explicit."
)]
pub(super) fn propose_sidestep(
    from: Vec2,
    radius: f32,
    requested: Vec2,
    axis_goal: Vec2,
    bounds: Vec2,
    blockers: &[Blocker],
    mut clear: impl FnMut(Vec2, Vec2) -> bool,
    mut charge_pair: impl FnMut() -> bool,
) -> (Option<Detour>, Work) {
    let mut work = Work::default();
    if blockers.is_empty()
        || blockers.len() > MAX_BLOCKERS
        || !from.is_finite()
        || !requested.is_finite()
        || !axis_goal.is_finite()
        || from == axis_goal
        || !radius.is_finite()
        || radius <= 0.0
        || from == requested
    {
        return (None, work);
    }
    let mut widest = 0.0_f32;
    let mut obstructed = false;
    for &blocker in blockers {
        work.scanned_rows += 1;
        if !blocker.position.is_finite() || !blocker.radius.is_finite() || blocker.radius <= 0.0 {
            return (None, work);
        }
        if !charge_pair() {
            return (None, work);
        }
        widest = widest.max(blocker.radius);
        obstructed |= !body_clear(from, requested, radius, blocker, &mut work);
    }
    if !obstructed {
        return (None, work);
    }
    let distance = (requested.as_dvec2() - from.as_dvec2()).length();
    let axis_offset = axis_goal.as_dvec2() - from.as_dvec2();
    let axis = axis_offset / axis_offset.length();
    let normal = bevy_math::DVec2::new(-axis.y, axis.x);
    let reach = f64::from(radius) + f64::from(widest);
    let mut best: Option<(f64, Detour)> = None;
    for step in 1..=SIDESTEPS {
        for side in [-1.0_f64, 1.0] {
            for retreat in 0..RETREATS {
                work.candidates += 1;
                let back = retreat as f64 * reach;
                let first = from.as_dvec2() + normal * (side * step as f64 * reach) - axis * back;
                let route = Detour {
                    corners: [
                        first.as_vec2(),
                        (first + axis * (back + ADVANCE * reach)).as_vec2(),
                    ],
                };
                let [first, second] = route.corners;
                if !first.is_finite() || !second.is_finite() || first == from || second == first {
                    continue;
                }
                let remaining = second.as_dvec2().distance(requested.as_dvec2());
                if remaining >= distance {
                    continue;
                }
                // A corner off the map is refused here, before any certificate.
                if [first, second].into_iter().any(|corner| {
                    corner.x < radius
                        || corner.y < radius
                        || corner.x > bounds.x - radius
                        || corner.y > bounds.y - radius
                }) {
                    continue;
                }
                // Every supplied body for both legs: the charge does not
                // depend on neighbor enumeration.
                let mut bodies_clear = true;
                for &body in blockers {
                    if !charge_pair() {
                        return (None, work);
                    }
                    bodies_clear &= body_clear(from, first, radius, body, &mut work);
                    if !charge_pair() {
                        return (None, work);
                    }
                    bodies_clear &= body_clear(first, second, radius, body, &mut work);
                }
                if !bodies_clear {
                    continue;
                }
                work.terrain_calls += 1;
                if !clear(from, first) {
                    continue;
                }
                work.terrain_calls += 1;
                if !clear(first, second) {
                    continue;
                }
                let score = first.as_dvec2().distance(from.as_dvec2())
                    + second.as_dvec2().distance(first.as_dvec2())
                    + remaining;
                if best.is_none_or(|(old_score, old_route)| {
                    score
                        .total_cmp(&old_score)
                        .then_with(|| corner_order(route, old_route))
                        .is_lt()
                }) {
                    best = Some((score, route));
                }
            }
        }
    }
    (best.map(|(_, route)| route), work)
}

#[cfg(test)]
#[allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
mod tests {
    use super::*;
    use turts_rules::terrain::{
        Ratio,
        geometry::{Cell, Grid, Traversal},
    };

    fn terrain(from: Vec2, to: Vec2, radius: f32, surface: impl Fn(Cell) -> Option<i32>) -> bool {
        Grid {
            columns: 64,
            rows: 64,
            length_scale: 1,
            cell_width: 1,
            elevation_step: 1,
            floor: 0,
            ceiling: 4,
        }
        .sweep(
            &surface,
            point(from),
            point(to),
            f64::from(radius),
            Traversal {
                max_step: 0,
                max_grade: Ratio {
                    numerator: 0,
                    denominator: 1,
                },
            },
            &mut Budget::new(16384),
        )
        .result
        .is_ok()
    }
    fn fixture() -> (Vec2, Vec2, [Blocker; 1]) {
        (
            Vec2::new(2.0, 8.0),
            Vec2::new(14.0, 8.0),
            [Blocker {
                position: Vec2::new(8.0, 8.0),
                radius: 1.0,
            }],
        )
    }
    fn certify(from: Vec2, radius: f32, route: Detour, blockers: &[Blocker]) {
        let mut start = from;
        for corner in route.corners {
            for &body in blockers {
                assert!(body_clear(
                    start,
                    corner,
                    radius,
                    body,
                    &mut Work::default()
                ));
            }
            start = corner;
        }
    }
    #[test]
    fn local_detour_routes_around_holder_without_replacing_endpoint() {
        let (from, requested, blockers) = fixture();
        assert!(
            !body_clear(from, requested, 0.5, blockers[0], &mut Work::default()),
            "the straight original continuation is blocked"
        );
        let (route, work) = propose(
            from,
            0.5,
            requested,
            &blockers,
            |a, b| terrain(a, b, 0.5, |_| Some(0)),
            || true,
        );
        let route = route.expect("two certified local corners around the holder");
        certify(from, 0.5, route, &blockers);
        assert!(route.corners[1].distance(requested) < from.distance(requested));
        assert!(body_clear(
            route.corners[1],
            requested,
            0.5,
            blockers[0],
            &mut Work::default()
        ));
        assert_eq!(blockers[0].position, Vec2::new(8.0, 8.0));
        assert!(work.body_checks <= MAX_BODY_CHECKS && work.terrain_calls <= MAX_TERRAIN_CALLS);
    }
    #[test]
    fn blocked_or_unknown_corner_uses_only_permitted_alternative() {
        let (from, requested, blockers) = fixture();
        for upper in [None, Some(4)] {
            let (route, _) = propose(
                from,
                0.5,
                requested,
                &blockers,
                |a, b| terrain(a, b, 0.5, |cell| if cell.y >= 9 { upper } else { Some(0) }),
                || true,
            );
            let route = route.expect("lower bay is known and flat");
            assert!(route.corners.iter().all(|p| p.y < 8.0));
            certify(from, 0.5, route, &blockers);
        }
        let (none, work) = propose(
            from,
            0.5,
            requested,
            &blockers,
            |a, b| terrain(a, b, 0.5, |_| None),
            || true,
        );
        assert!(none.is_none(), "unknown terrain never authorizes a route");
        assert!(work.terrain_calls > 0);
    }
    #[test]
    fn oversized_invalid_or_unobstructed_input_never_yields_partial_route() {
        let (from, requested, blockers) = fixture();
        let oversized = [blockers[0]; MAX_BLOCKERS + 1];
        assert_eq!(
            propose(
                from,
                0.5,
                requested,
                &oversized,
                |_, _| panic!("overflow"),
                || true
            ),
            (None, Work::default())
        );
        assert!(
            propose(
                from,
                0.5,
                requested,
                &[Blocker {
                    radius: f32::NAN,
                    ..blockers[0]
                }],
                |_, _| panic!("invalid"),
                || true
            )
            .0
            .is_none()
        );
        assert!(
            propose(
                from,
                0.5,
                requested,
                &[],
                |_, _| panic!("no own blocker"),
                || true
            )
            .0
            .is_none()
        );
        assert!(
            propose(
                from,
                0.5,
                requested,
                &[Blocker {
                    position: from,
                    ..blockers[0]
                }],
                |_, _| panic!("illegal initial overlap"),
                || true
            )
            .0
            .is_none()
        );
    }
    #[test]
    fn whole_second_leg_is_certified_before_route_is_returned() {
        let (from, requested, blockers) = fixture();
        let (none, work) = propose(from, 0.5, requested, &blockers, |a, _| a == from, || true);
        assert!(
            none.is_none(),
            "a clear first leg cannot authorize an unavailable second leg"
        );
        assert_eq!(work.terrain_calls, 4);
    }

    // Actual afcf GM1-wide tick2400 snapshot: retained-passing-gm1/
    // GM1-wide-quarry-a0-m0-4071672-0/failure-state.json. All 30 settled own
    // holders, unmodified f32 poses; moving peers remain the final guard's job.
    fn gm1_holders() -> [(u32, Blocker); 30] {
        [
            (
                1,
                Blocker {
                    position: Vec2::new(46.54999923706055, 18.024425506591797),
                    radius: 0.4,
                },
            ),
            (
                2,
                Blocker {
                    position: Vec2::new(47.70000076293945, 20.01628303527832),
                    radius: 0.4,
                },
            ),
            (
                3,
                Blocker {
                    position: Vec2::new(48.849998474121094, 18.024425506591797),
                    radius: 0.4,
                },
            ),
            (
                4,
                Blocker {
                    position: Vec2::new(51.150001525878906, 18.024425506591797),
                    radius: 0.4,
                },
            ),
            (
                5,
                Blocker {
                    position: Vec2::new(53.45000076293945, 18.024425506591797),
                    radius: 0.4,
                },
            ),
            (
                6,
                Blocker {
                    position: Vec2::new(54.599998474121094, 20.01628303527832),
                    radius: 0.4,
                },
            ),
            (
                7,
                Blocker {
                    position: Vec2::new(45.400001525878906, 20.01628303527832),
                    radius: 0.4,
                },
            ),
            (
                8,
                Blocker {
                    position: Vec2::new(46.54999923706055, 22.008142471313477),
                    radius: 0.4,
                },
            ),
            (
                9,
                Blocker {
                    position: Vec2::new(50.0, 20.01628303527832),
                    radius: 0.4,
                },
            ),
            (
                10,
                Blocker {
                    position: Vec2::new(52.29999923706055, 20.01628303527832),
                    radius: 0.4,
                },
            ),
            (
                11,
                Blocker {
                    position: Vec2::new(53.45000076293945, 22.008142471313477),
                    radius: 0.4,
                },
            ),
            (
                12,
                Blocker {
                    position: Vec2::new(55.75, 22.008142471313477),
                    radius: 0.4,
                },
            ),
            (
                13,
                Blocker {
                    position: Vec2::new(44.25, 22.008142471313477),
                    radius: 0.4,
                },
            ),
            (
                14,
                Blocker {
                    position: Vec2::new(50.0, 24.0),
                    radius: 0.4,
                },
            ),
            (
                15,
                Blocker {
                    position: Vec2::new(48.849998474121094, 22.008142471313477),
                    radius: 0.4,
                },
            ),
            (
                16,
                Blocker {
                    position: Vec2::new(51.150001525878906, 22.008142471313477),
                    radius: 0.4,
                },
            ),
            (
                17,
                Blocker {
                    position: Vec2::new(54.599998474121094, 24.0),
                    radius: 0.4,
                },
            ),
            (
                18,
                Blocker {
                    position: Vec2::new(56.900001525878906, 24.0),
                    radius: 0.4,
                },
            ),
            (
                19,
                Blocker {
                    position: Vec2::new(47.70000076293945, 24.0),
                    radius: 0.4,
                },
            ),
            (
                20,
                Blocker {
                    position: Vec2::new(48.849998474121094, 29.975574493408203),
                    radius: 0.4,
                },
            ),
            (
                21,
                Blocker {
                    position: Vec2::new(48.849998474121094, 25.991857528686523),
                    radius: 0.4,
                },
            ),
            (
                22,
                Blocker {
                    position: Vec2::new(52.29999923706055, 24.0),
                    radius: 0.4,
                },
            ),
            (
                23,
                Blocker {
                    position: Vec2::new(54.599998474121094, 27.98371696472168),
                    radius: 0.4,
                },
            ),
            (
                24,
                Blocker {
                    position: Vec2::new(55.75, 25.991857528686523),
                    radius: 0.4,
                },
            ),
            (
                25,
                Blocker {
                    position: Vec2::new(45.400001525878906, 24.0),
                    radius: 1.1,
                },
            ),
            (
                28,
                Blocker {
                    position: Vec2::new(53.45000076293945, 25.991857528686523),
                    radius: 1.1,
                },
            ),
            (
                29,
                Blocker {
                    position: Vec2::new(44.25, 25.991857528686523),
                    radius: 1.1,
                },
            ),
            (
                31,
                Blocker {
                    position: Vec2::new(50.0, 27.98371696472168),
                    radius: 1.1,
                },
            ),
            (
                32,
                Blocker {
                    position: Vec2::new(52.29999923706055, 27.98371696472168),
                    radius: 1.1,
                },
            ),
            (
                36,
                Blocker {
                    position: Vec2::new(53.45000076293945, 29.975574493408203),
                    radius: 1.1,
                },
            ),
        ]
    }
    #[test]
    fn actual_gm1_stalled_large_has_safe_local_detour_under_id_and_enumeration_reversal() {
        let from = Vec2::new(41.190101623535156, 23.108901977539062);
        let requested = Vec2::new(47.70000076293945, 27.98371696472168);
        let original = gm1_holders();
        let mut expected = None;
        for reverse in [false, true] {
            let mut labeled = original;
            if reverse {
                labeled.reverse();
                for (id, _) in &mut labeled {
                    *id = 1000 - *id;
                }
            }
            let blockers = labeled.map(|(_, body)| body);
            let (route, work) = propose(
                from,
                1.1,
                requested,
                &blockers,
                |a, b| terrain(a, b, 1.1, |_| Some(0)),
                || true,
            );
            let route = route.expect("captured large.1.1 receives a safe local detour");
            certify(from, 1.1, route, &blockers);
            assert!(route.corners[1].distance(requested) < from.distance(requested));
            assert!(work.body_checks <= MAX_BODY_CHECKS && work.terrain_calls <= MAX_TERRAIN_CALLS);
            if let Some((old_route, old_checks, old_calls)) = expected {
                assert_eq!(route, old_route);
                assert_eq!(work.body_checks, old_checks);
                assert_eq!(work.terrain_calls, old_calls);
            } else {
                expected = Some((route, work.body_checks, work.terrain_calls));
            }
        }
    }
    #[test]
    fn dense_input_work_stays_inside_fixed_bounds() {
        let (from, requested, blockers) = fixture();
        let dense = [blockers[0]; MAX_BLOCKERS];
        let (route, work) = propose(from, 0.5, requested, &dense, |_, _| true, || true);
        assert!(route.is_some());
        assert_eq!(work.scanned_rows, MAX_SCANNED_ROWS);
        assert_eq!(work.candidates, 16);
        assert_eq!(work.body_checks, MAX_BODY_CHECKS);
        assert_eq!(work.terrain_calls, MAX_TERRAIN_CALLS);
        assert!(work.generator_comparisons <= MAX_BLOCKERS * GENERATORS);
    }

    #[test]
    fn exhausted_shared_pair_budget_discards_even_an_earlier_candidate() {
        let (from, requested, blockers) = fixture();
        // Direct check plus both legs of the first candidate are affordable;
        // the next candidate's first pair cannot be charged.
        let mut remaining = 3;
        let (route, work) = propose(
            from,
            0.5,
            requested,
            &blockers,
            |_, _| true,
            || {
                if remaining == 0 {
                    false
                } else {
                    remaining -= 1;
                    true
                }
            },
        );
        assert!(
            route.is_none(),
            "a partial search cannot return the earlier clear candidate"
        );
        assert_eq!(work.body_checks, 3);
        assert_eq!(work.terrain_calls, 2);
        let (route, work) = propose(
            from,
            0.5,
            requested,
            &blockers,
            |_, _| panic!("no budget"),
            || false,
        );
        assert!(route.is_none());
        assert_eq!(work.body_checks, 0);
    }
    fn standing_group() -> (Vec2, Vec2, Vec<Blocker>) {
        // Actual GM4 tick113 poses: eleven waiting large bodies of the main
        // group, and the far actor in the notch between two of them.
        let mut blockers = Vec::new();
        for y in [30.0, 32.5, 35.0] {
            for x in [8.0, 10.5, 13.0, 15.5] {
                if (x, y) != (15.5, 35.0) {
                    blockers.push(Blocker {
                        position: Vec2::new(x, y),
                        radius: 1.1,
                    });
                }
            }
        }
        (Vec2::new(9.25, 36.81039), Vec2::new(22., 2.), blockers)
    }
    #[test]
    fn sidestep_leaves_the_notch_of_a_standing_group_that_one_disc_route_cannot_pass() {
        let (from, requested, blockers) = standing_group();
        let (single, _) = propose(from, 1.1, requested, &blockers, |_, _| true, || true);
        assert_eq!(single, None, "one-disc routes end inside the group");
        let (route, work) = propose_sidestep(
            from,
            1.1,
            requested,
            requested,
            Vec2::new(64., 48.),
            &blockers,
            |_, _| true,
            || true,
        );
        let route = route.expect("a bounded route around the standing group");
        certify(from, 1.1, route, &blockers);
        assert!(route.corners[1].distance(requested) < from.distance(requested));
        assert_eq!(work.candidates, MAX_SIDESTEP_PROPOSALS);
        assert!(work.body_checks <= MAX_SIDESTEP_BODY_CHECKS);
        assert!(work.terrain_calls <= MAX_SIDESTEP_TERRAIN_CALLS);
        // The same input in reverse row order selects the same route.
        let mut reversed = blockers.clone();
        reversed.reverse();
        let (again, _) = propose_sidestep(
            from,
            1.1,
            requested,
            requested,
            Vec2::new(64., 48.),
            &reversed,
            |_, _| true,
            || true,
        );
        assert_eq!(again, Some(route));
    }
    #[test]
    fn sidestep_refuses_a_clear_leg_terrain_refusal_and_budget_refusal() {
        let (from, requested, blockers) = standing_group();
        // No standing body obstructs this endpoint: nothing to go around.
        let (clear_leg, _) = propose_sidestep(
            from,
            1.1,
            Vec2::new(9.25, 46.),
            Vec2::new(9.25, 46.),
            Vec2::new(64., 48.),
            &blockers,
            |_, _| true,
            || true,
        );
        assert_eq!(clear_leg, None);
        // One far body that does not obstruct the leg, and open ground: every
        // side step would certify, so only the obstruction guard refuses.
        let aside = [Blocker {
            position: Vec2::new(40., 40.),
            radius: 1.1,
        }];
        let (unobstructed, work) = propose_sidestep(
            from,
            1.1,
            requested,
            requested,
            Vec2::new(64., 48.),
            &aside,
            |_, _| true,
            || true,
        );
        assert_eq!(unobstructed, None);
        assert_eq!(work.candidates, 0);
        // A map so small that every corner is outside it: no terrain call.
        let (off_map, work) = propose_sidestep(
            from,
            1.1,
            requested,
            requested,
            Vec2::new(12., 40.),
            &blockers,
            |_, _| true,
            || true,
        );
        assert_eq!(off_map, None);
        assert_eq!(work.terrain_calls, 0);
        let (walled, work) = propose_sidestep(
            from,
            1.1,
            requested,
            requested,
            Vec2::new(64., 48.),
            &blockers,
            |_, _| false,
            || true,
        );
        assert_eq!(walled, None);
        assert!(work.terrain_calls > 0 && work.terrain_calls <= MAX_SIDESTEP_TERRAIN_CALLS);
        let mut credit = 40;
        let (refused, _) = propose_sidestep(
            from,
            1.1,
            requested,
            requested,
            Vec2::new(64., 48.),
            &blockers,
            |_, _| true,
            || {
                credit -= 1;
                credit > 0
            },
        );
        assert_eq!(
            refused, None,
            "a budget refusal discards the whole proposal"
        );
    }
}
