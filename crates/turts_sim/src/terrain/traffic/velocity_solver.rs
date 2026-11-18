//! Bounded deterministic velocity advice; exact guards decide actual motion.
//!
//! Agent velocity-obstacle construction: van den Berg et al., Reciprocal n-body
//! Collision Avoidance (2011), https://gamma.cs.unc.edu/ORCA/publications/ORCA.pdf.
//! The scalar line/circle program follows the RVO2 reference geometry. This
//! implementation uses fixed arrays and full actor responsibility for every
//! neighbor; it does not assume reciprocal participation or use random normals.
//! LP geometry is adapted from RVO2, copyright 2008 University of North Carolina
//! at Chapel Hill, under Apache-2.0. See docs/licenses/RVO2-Apache-2.0.txt.
//! TuRTS adds metering, fixed storage, explicit refusal and hard/soft masks.
use bevy_math::DVec2;

const NEIGHBORS: usize = 8;
const PARALLEL: f64 = 1.0e-12;
#[derive(Clone, Copy, Debug)]
pub(super) struct Agent {
    pub position: DVec2,
    pub velocity: DVec2,
    pub radius: f64,
}
#[derive(Clone, Copy, Debug)]
struct Line {
    point: DVec2,
    direction: DVec2,
}
impl Line {
    const ZERO: Self = Self {
        point: DVec2::ZERO,
        direction: DVec2::ZERO,
    };
}
fn det(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}
fn unit(v: DVec2) -> Option<DVec2> {
    let length = v.length();
    (length.is_finite() && length > 0.).then(|| v / length)
}
fn valid(a: Agent) -> bool {
    a.position.is_finite() && a.velocity.is_finite() && a.radius.is_finite() && a.radius > 0.
}
fn boundary_with_share(a: Agent, b: Agent, dt: f64, horizon: f64, share: f64) -> Option<Line> {
    let position = b.position - a.position;
    let velocity = a.velocity - b.velocity;
    let distance2 = position.length_squared();
    let radius = a.radius + b.radius;
    let radius2 = radius * radius;
    if !distance2.is_finite() || !radius2.is_finite() {
        return None;
    }
    let (direction, correction) = if distance2 > radius2 {
        let w = velocity - position / horizon;
        let w2 = w.length_squared();
        let projection = w.dot(position);
        if !w2.is_finite() || !projection.is_finite() {
            return None;
        }
        if projection < 0. && projection * projection > radius2 * w2 {
            let normal = unit(w)?;
            (
                DVec2::new(normal.y, -normal.x),
                (radius / horizon - w2.sqrt()) * normal,
            )
        } else {
            let leg = (distance2 - radius2).sqrt();
            let direction = if det(position, w) > 0. {
                DVec2::new(
                    position.x * leg - position.y * radius,
                    position.x * radius + position.y * leg,
                ) / distance2
            } else {
                -DVec2::new(
                    position.x * leg + position.y * radius,
                    -position.x * radius + position.y * leg,
                ) / distance2
            };
            (direction, velocity.dot(direction) * direction - velocity)
        }
    } else {
        let w = velocity - position / dt;
        // Coincident/ambiguous states decline. Do not invent an ID or RNG normal.
        let normal = unit(w)?;
        (
            DVec2::new(normal.y, -normal.x),
            (radius / dt - w.length()) * normal,
        )
    };
    let line = Line {
        point: a.velocity + correction * share,
        direction,
    };
    (line.point.is_finite() && line.direction.is_finite()).then_some(line)
}

#[cfg(test)]
fn boundary(a: Agent, b: Agent, dt: f64, horizon: f64) -> Option<Line> {
    boundary_with_share(a, b, dt, horizon, 1.)
}

#[derive(Clone, Copy, Debug)]
enum Failure {
    Work,
    Arithmetic,
}

/// Strict scalar solve; every refusal keeps the existing strict fallback.
#[cfg(test)]
fn on_line(
    lines: &[Line],
    index: usize,
    speed: f64,
    preferred: DVec2,
    charge: &mut dyn FnMut() -> bool,
) -> Result<Option<DVec2>, ()> {
    project_line(lines, index, speed, preferred, false, charge).map_err(|_| ())
}

fn project_line(
    lines: &[Line],
    index: usize,
    speed: f64,
    preferred: DVec2,
    directional: bool,
    charge: &mut dyn FnMut() -> bool,
) -> Result<Option<DVec2>, Failure> {
    let line = lines[index];
    let projection = line.point.dot(line.direction);
    let discriminant = projection * projection + speed * speed - line.point.length_squared();
    if !discriminant.is_finite() {
        return Err(Failure::Arithmetic);
    }
    if discriminant < 0. {
        return Ok(None);
    }
    let root = discriminant.sqrt();
    let (mut low, mut high) = (-projection - root, -projection + root);
    for previous in &lines[..index] {
        if !charge() {
            return Err(Failure::Work);
        }
        let denominator = det(line.direction, previous.direction);
        let numerator = det(previous.direction, line.point - previous.point);
        if !denominator.is_finite() || !numerator.is_finite() {
            return Err(Failure::Arithmetic);
        }
        if denominator.abs() <= PARALLEL {
            if numerator < 0. {
                return Ok(None);
            }
            continue;
        }
        let at = numerator / denominator;
        if !at.is_finite() {
            return Err(Failure::Arithmetic);
        }
        if denominator >= 0. {
            high = high.min(at);
        } else {
            low = low.max(at);
        }
        if low > high {
            return Ok(None);
        }
    }
    let at = if directional {
        if preferred.dot(line.direction) > 0. {
            high
        } else {
            low
        }
    } else {
        line.direction.dot(preferred - line.point).clamp(low, high)
    };
    let result = line.point + line.direction * at;
    if result.is_finite() {
        Ok(Some(result))
    } else {
        Err(Failure::Arithmetic)
    }
}

/// Full actor responsibility. Infeasible/numerical/work refusal returns None.
/// The caller owns canonical neighbor ordering and the shared attempt ticket.
#[allow(
    clippy::too_many_arguments,
    reason = "Keep the shared motion-model boundary explicit"
)]
#[cfg(test)]
pub(super) fn solve(
    actor: Agent,
    neighbors: &[Agent],
    preferred: DVec2,
    max_speed: f64,
    dt: f64,
    horizon: f64,
    charge: &mut dyn FnMut() -> bool,
) -> Option<DVec2> {
    if !valid(actor)
        || neighbors.len() > NEIGHBORS
        || !preferred.is_finite()
        || !max_speed.is_finite()
        || max_speed <= 0.
        || !(max_speed * max_speed).is_finite()
        || !dt.is_finite()
        || dt <= 0.
        || !horizon.is_finite()
        || horizon < dt
    {
        return None;
    }
    let mut lines = [Line::ZERO; NEIGHBORS];
    for (line, neighbor) in lines.iter_mut().zip(neighbors) {
        if !charge() || !valid(*neighbor) {
            return None;
        }
        *line = boundary(actor, *neighbor, dt, horizon)?;
    }
    let length = preferred.length();
    if !length.is_finite() {
        return None;
    }
    let mut result = if length > max_speed {
        preferred * (max_speed / length)
    } else {
        preferred
    };
    let lines = &lines[..neighbors.len()];
    for (index, line) in lines.iter().enumerate() {
        if !charge() {
            return None;
        }
        if det(line.direction, line.point - result) > 0. {
            result = on_line(lines, index, max_speed, preferred, charge).ok()??;
        }
    }
    // This validates the approximate model only, never the physical footprint.
    // The independent exact swept-body and terrain guards remain mandatory.
    for line in lines {
        if !charge() {
            return None;
        }
        let residual = det(line.direction, line.point - result);
        if !residual.is_finite() || residual > 1.0e-9 * (1. + max_speed + line.point.length()) {
            return None;
        }
    }
    (result.is_finite() && result.length() <= max_speed * (1. + 1.0e-12)).then_some(result)
}

/// The program keeps its last feasible prefix only for geometric infeasibility.
/// Arithmetic and work refusal cannot authorize a relaxed model.
enum Program {
    Complete(DVec2),
    Infeasible { at: usize, seed: DVec2 },
}
fn program(
    lines: &[Line],
    speed: f64,
    preferred: DVec2,
    directional: bool,
    charge: &mut dyn FnMut() -> bool,
) -> Result<Program, Failure> {
    let length = preferred.length();
    if !length.is_finite() {
        return Err(Failure::Arithmetic);
    }
    let mut result = if directional {
        unit(preferred).ok_or(Failure::Arithmetic)? * speed
    } else if length > speed {
        preferred * (speed / length)
    } else {
        preferred
    };
    for (i, line) in lines.iter().enumerate() {
        if !charge() {
            return Err(Failure::Work);
        }
        let violation = det(line.direction, line.point - result);
        if !violation.is_finite() {
            return Err(Failure::Arithmetic);
        }
        if violation > 0. {
            let Some(next) = project_line(lines, i, speed, preferred, directional, charge)? else {
                return Ok(Program::Infeasible {
                    at: i,
                    seed: result,
                });
            };
            result = next;
        }
    }
    Ok(Program::Complete(result))
}

/// Fixed-storage projected refinement follows RVO2's LP3 geometry.
/// Unlike the reference, an unresolved projected program declines the proposal.
/// https://github.com/snape/RVO2/blob/main/src/Agent.cc (Apache-2.0).
fn refine(
    lines: &[Line],
    hard: usize,
    first: usize,
    speed: f64,
    mut result: DVec2,
    charge: &mut dyn FnMut() -> bool,
) -> Result<DVec2, Failure> {
    let mut distance = 0.;
    for i in first..lines.len() {
        if !charge() {
            return Err(Failure::Work);
        }
        let violation = det(lines[i].direction, lines[i].point - result);
        if !violation.is_finite() {
            return Err(Failure::Arithmetic);
        }
        if violation <= distance {
            continue;
        }
        let mut projected = [Line::ZERO; NEIGHBORS];
        projected[..hard].copy_from_slice(&lines[..hard]);
        let mut count = hard;
        for j in hard..i {
            if !charge() {
                return Err(Failure::Work);
            }
            let denominator = det(lines[i].direction, lines[j].direction);
            if !denominator.is_finite() {
                return Err(Failure::Arithmetic);
            }
            let point = if denominator.abs() <= PARALLEL {
                if lines[i].direction.dot(lines[j].direction) > 0. {
                    continue;
                }
                (lines[i].point + lines[j].point) * 0.5
            } else {
                lines[i].point
                    + lines[i].direction
                        * (det(lines[j].direction, lines[i].point - lines[j].point) / denominator)
            };
            let direction =
                unit(lines[j].direction - lines[i].direction).ok_or(Failure::Arithmetic)?;
            if !point.is_finite() {
                return Err(Failure::Arithmetic);
            }
            projected[count] = Line { point, direction };
            count += 1;
        }
        let objective = DVec2::new(-lines[i].direction.y, lines[i].direction.x);
        result = match program(&projected[..count], speed, objective, true, charge)? {
            Program::Complete(value) => value,
            Program::Infeasible { .. } => return Err(Failure::Arithmetic),
        };
        distance = det(lines[i].direction, lines[i].point - result);
        if !distance.is_finite() {
            return Err(Failure::Arithmetic);
        }
    }
    Ok(result)
}

/// Static predictions remain hard. Only conflicting moving-own predictions may
/// use a compromise. Every returned velocity still needs current exact guards.
#[allow(clippy::too_many_arguments, reason = "Explicit shared model contract")]
#[cfg(test)]
pub(super) fn solve_relaxed(
    actor: Agent,
    neighbors: &[Agent],
    hard_mask: u8,
    preferred: DVec2,
    max_speed: f64,
    dt: f64,
    horizon: f64,
    charge: &mut dyn FnMut() -> bool,
) -> Option<DVec2> {
    solve_with_status(
        actor, neighbors, hard_mask, preferred, max_speed, dt, horizon, charge,
    )
    .map(|result| result.velocity)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Solution {
    pub velocity: DVec2,
    pub refined: bool,
}

#[allow(clippy::too_many_arguments, reason = "Explicit shared model contract")]
#[cfg(test)]
pub(super) fn solve_with_status(
    actor: Agent,
    neighbors: &[Agent],
    hard_mask: u8,
    preferred: DVec2,
    max_speed: f64,
    dt: f64,
    horizon: f64,
    charge: &mut dyn FnMut() -> bool,
) -> Option<Solution> {
    solve_with_share(
        actor, neighbors, hard_mask, preferred, max_speed, dt, horizon, 1., charge,
    )
}

/// Explicit responsibility is available to bounded diagnostic drivers.
#[allow(clippy::too_many_arguments, reason = "Explicit shared model contract")]
pub(super) fn solve_with_share(
    actor: Agent,
    neighbors: &[Agent],
    hard_mask: u8,
    preferred: DVec2,
    max_speed: f64,
    dt: f64,
    horizon: f64,
    share: f64,
    charge: &mut dyn FnMut() -> bool,
) -> Option<Solution> {
    if !share.is_finite()
        || share <= 0.
        || share > 1.
        || !valid(actor)
        || neighbors.len() > NEIGHBORS
        || !preferred.is_finite()
        || !max_speed.is_finite()
        || max_speed <= 0.
        || !(max_speed * max_speed).is_finite()
        || !dt.is_finite()
        || dt <= 0.
        || !horizon.is_finite()
        || horizon < dt
    {
        return None;
    }
    let mut lines = [Line::ZERO; NEIGHBORS];
    let mut count = 0;
    let mut hard = 0;
    for is_hard in [true, false] {
        for (i, &neighbor) in neighbors.iter().enumerate() {
            if (hard_mask & (1 << i) != 0) != is_hard {
                continue;
            }
            if !charge() || !valid(neighbor) {
                return None;
            }
            lines[count] = boundary_with_share(
                actor,
                neighbor,
                dt,
                horizon,
                if is_hard { 1. } else { share },
            )?;
            count += 1;
        }
        if is_hard {
            hard = count;
        }
    }
    let lines = &lines[..count];
    let (result, checked, refined) =
        match program(lines, max_speed, preferred, false, charge).ok()? {
            Program::Complete(value) => (value, count, false),
            Program::Infeasible { at, seed } if at >= hard => (
                refine(lines, hard, at, max_speed, seed, charge).ok()?,
                hard,
                true,
            ),
            Program::Infeasible { .. } => return None,
        };
    for line in &lines[..checked] {
        if !charge() {
            return None;
        }
        let residual = det(line.direction, line.point - result);
        if !residual.is_finite() || residual > 1e-9 * (1. + max_speed + line.point.length()) {
            return None;
        }
    }
    if !charge() {
        return None;
    }
    (result.is_finite() && result.length() <= max_speed * (1. + 1e-12)).then_some(Solution {
        velocity: result,
        refined,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn agent(x: f64, y: f64, vx: f64, vy: f64) -> Agent {
        Agent {
            position: DVec2::new(x, y),
            velocity: DVec2::new(vx, vy),
            radius: 0.25,
        }
    }
    fn run(a: Agent, others: &[Agent], preferred: DVec2, speed: f64, dt: f64) -> Option<DVec2> {
        let mut work = 0;
        solve(a, others, preferred, speed, dt, dt * 2., &mut || {
            work += 1;
            work <= 128
        })
    }
    #[test]
    fn free_and_coherent_parallel_motion_keep_the_preferred_velocity() {
        let a = agent(0., 0., 3., 0.);
        assert_eq!(run(a, &[], DVec2::X * 3., 3., 0.05), Some(DVec2::X * 3.));
        let neighbors = [agent(-0.8, 0., 3., 0.), agent(0.8, 0., 3., 0.)];
        assert_eq!(
            run(a, &neighbors, DVec2::X * 3., 3., 0.05),
            Some(DVec2::X * 3.)
        );
        assert_eq!(run(a, &[], DVec2::X * 20., 3., 0.05), Some(DVec2::X * 3.));
    }
    #[test]
    fn stationary_holder_receives_full_avoidance_without_reciprocity() {
        let a = agent(-1., 0., 2., 0.);
        let b = agent(0., 0., 0., 0.);
        let v = run(a, &[b], DVec2::X * 2., 2., 0.25).unwrap();
        assert!(v.x < 2. && v.length() <= 2.);
        assert!(certified(a, v, b, DVec2::ZERO, 0.25));
        assert_eq!(b.position, DVec2::ZERO);
    }
    fn certified(a: Agent, av: DVec2, b: Agent, bv: DVec2, dt: f64) -> bool {
        use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};
        let point = |v: DVec2| {
            let v = v.as_vec2();
            Point {
                x: f64::from(v.x),
                y: f64::from(v.y),
            }
        };
        swept_disks_clear(
            point(a.position),
            point(a.position + av * dt),
            a.radius,
            point(b.position),
            point(b.position + bv * dt),
            b.radius,
            &mut Budget::new(1),
        )
        .unwrap()
    }
    #[test]
    fn crossing_proposals_pass_the_independent_exact_sweep_predicate() {
        let a = agent(-1., 0., 4., 0.);
        let b = agent(0., -1., 0., 4.);
        assert!(!certified(a, a.velocity, b, b.velocity, 0.25));
        let av = run(a, &[b], a.velocity, 4., 0.25).unwrap();
        let bv = run(b, &[a], b.velocity, 4., 0.25).unwrap();
        assert!(certified(a, av, b, bv, 0.25), "{av:?} {bv:?}");
    }
    #[test]
    fn degenerate_nonfinite_and_infeasible_models_decline_without_randomness() {
        let a = agent(0., 0., 0., 0.);
        assert!(run(a, &[a], DVec2::X, 1., 0.05).is_none());
        let ring = [
            agent(0.1, 0., 0., 0.),
            agent(-0.1, 0., 0., 0.),
            agent(0., 0.1, 0., 0.),
            agent(0., -0.1, 0., 0.),
        ];
        assert!(run(a, &ring, DVec2::X, 1., 0.05).is_none());
        let mut bad = a;
        bad.position.x = f64::NAN;
        assert!(run(bad, &[], DVec2::X, 1., 0.05).is_none());
        assert!(run(a, &[], DVec2::splat(f64::INFINITY), 1., 0.05).is_none());
        assert!(run(a, &[a; 9], DVec2::X, 1., 0.05).is_none());
    }
    #[test]
    fn every_half_plane_and_projection_respects_work_refusal() {
        let a = agent(-1., 0., 2., 0.);
        let others = [agent(0., 0., 0., 0.), agent(-1., 0.8, 0., 0.)];
        let mut full = 0;
        let expected = solve(a, &others, DVec2::X * 2., 2., 0.25, 0.5, &mut || {
            full += 1;
            true
        });
        assert!(expected.is_some() && full >= 6);
        for limit in 0..full {
            let mut used = 0;
            assert!(
                solve(a, &others, DVec2::X * 2., 2., 0.25, 0.5, &mut || {
                    if used == limit {
                        false
                    } else {
                        used += 1;
                        true
                    }
                })
                .is_none()
            );
            assert_eq!(used, limit);
        }
    }
    #[test]
    fn recorded_infeasible_tick_55_refines_with_hard_obstacles_and_metered_prefixes() {
        let actor = Agent {
            position: DVec2::new(70.51676940917969, 30.677452087402344),
            velocity: DVec2::new(-32.840117918847234, -79.08409000807866),
            radius: 0.30000001192092896,
        };
        let neighbors = [
            Agent {
                position: DVec2::new(71.05615997314453, 30.414663314819336),
                velocity: DVec2::new(-75.1847828152063, -65.93364617327383),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(69.8780517578125, 30.530649185180664),
                velocity: DVec2::new(-21.497039474591023, -80.4358661109825),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(69.84275817871094, 28.301786422729492),
                velocity: DVec2::new(-93.3879075439586, -28.07903247953903),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(69.21282958984375, 28.126665115356445),
                velocity: DVec2::new(-99.80270237024573, 6.27819051924064),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(68.65523529052734, 28.49808120727539),
                velocity: DVec2::new(-99.75211948525728, 7.036781206178945),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(69.46985626220703, 27.584484100341797),
                velocity: DVec2::new(-99.97268527786903, 2.3384475359551975),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(68.739501953125, 27.599231719970703),
                velocity: DVec2::new(0.21301269213836355, 0.6723403830477538),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(68.33978271484375, 26.91623878479004),
                velocity: DVec2::new(-99.96658176233498, 2.5852965923354536),
                radius: 0.30000001192092896,
            },
        ];
        let preferred = DVec2::new(-37.01156561059407, -77.21988562911919);
        let dt = f64::from(0.05f32);
        assert!(
            solve(actor, &neighbors, preferred, 100., dt, 2. * dt, &mut || {
                true
            })
            .is_none()
        );
        let mut operations = 0;
        let result = solve_relaxed(
            actor,
            &neighbors,
            0u8,
            preferred,
            100.,
            dt,
            2. * dt,
            &mut || {
                operations += 1;
                true
            },
        )
        .expect("bounded compromise");
        assert!(result.is_finite() && result.length() <= 100. * (1. + 1e-12));
        let status = solve_with_status(
            actor,
            &neighbors,
            0u8,
            preferred,
            100.,
            dt,
            2. * dt,
            &mut || true,
        )
        .unwrap();
        assert!(status.refined);
        assert_eq!(status.velocity, result);
        eprintln!("recorded 55 refinement operations={operations} result={result:?}");
        // This fixture has no hard constraints: all neighbors are soft.
        for limit in 0..operations {
            let mut spent = 0;
            assert!(
                solve_relaxed(
                    actor,
                    &neighbors,
                    0u8,
                    preferred,
                    100.,
                    dt,
                    2. * dt,
                    &mut || {
                        if spent == limit {
                            return false;
                        }
                        spent += 1;
                        true
                    }
                )
                .is_none()
            );
            assert_eq!(spent, limit);
        }
        assert!(
            solve_relaxed(
                actor,
                &neighbors,
                255,
                preferred,
                100.,
                dt,
                2. * dt,
                &mut || true
            )
            .is_none()
        );
    }

    #[test]
    fn recorded_infeasible_tick_57_refines_with_hard_obstacles_and_metered_prefixes() {
        let actor = Agent {
            position: DVec2::new(69.2637710571289, 29.43859100341797),
            velocity: DVec2::new(-50.90164108720957, -53.37833325001101),
            radius: 0.30000001192092896,
        };
        let neighbors = [
            Agent {
                position: DVec2::new(68.77886199951172, 29.085222244262695),
                velocity: DVec2::new(7.104949845299872, -22.912711755741956),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(69.4876937866211, 28.821516036987305),
                velocity: DVec2::new(-92.82012801140355, -37.20767919409753),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(70.02155303955078, 29.627185821533203),
                velocity: DVec2::new(-74.132536737136, -67.11463828214052),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(69.55877685546875, 28.14924430847168),
                velocity: DVec2::new(-99.79034275128927, -6.471023463144549),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(68.39857482910156, 28.11905288696289),
                velocity: DVec2::new(13.904266150232164, -13.86051157324744),
                radius: 0.30000001192092896,
            },
            Agent {
                position: DVec2::new(67.375, 28.625),
                velocity: DVec2::new(0.0, 0.0),
                radius: 0.30000001192092896,
            },
        ];
        let preferred = DVec2::new(-46.221312787811264, -57.478865720646596);
        let dt = f64::from(0.05f32);
        assert!(
            solve(actor, &neighbors, preferred, 100., dt, 2. * dt, &mut || {
                true
            })
            .is_none()
        );
        let mut operations = 0;
        let result = solve_relaxed(
            actor,
            &neighbors,
            32u8,
            preferred,
            100.,
            dt,
            2. * dt,
            &mut || {
                operations += 1;
                true
            },
        )
        .expect("bounded compromise");
        assert!(result.is_finite() && result.length() <= 100. * (1. + 1e-12));
        let status = solve_with_status(
            actor,
            &neighbors,
            32u8,
            preferred,
            100.,
            dt,
            2. * dt,
            &mut || true,
        )
        .unwrap();
        assert!(status.refined);
        assert_eq!(status.velocity, result);
        eprintln!("recorded 57 refinement operations={operations} result={result:?}");
        for (i, peer) in neighbors.iter().enumerate() {
            if 32u8 & (1 << i) == 0 {
                continue;
            }
            let line = boundary(actor, *peer, dt, 2. * dt).unwrap();
            assert!(det(line.direction, line.point - result) <= 1e-9);
        }
        for limit in 0..operations {
            let mut spent = 0;
            assert!(
                solve_relaxed(
                    actor,
                    &neighbors,
                    32u8,
                    preferred,
                    100.,
                    dt,
                    2. * dt,
                    &mut || {
                        if spent == limit {
                            return false;
                        }
                        spent += 1;
                        true
                    }
                )
                .is_none()
            );
            assert_eq!(spent, limit);
        }
        assert!(
            solve_relaxed(
                actor,
                &neighbors,
                255,
                preferred,
                100.,
                dt,
                2. * dt,
                &mut || true
            )
            .is_none()
        );
    }

    #[test]
    fn relaxed_parallel_opposed_geometry_and_arithmetic_refusal_remain_bounded() {
        let lines = [
            Line {
                point: DVec2::new(2., 0.),
                direction: -DVec2::Y,
            },
            Line {
                point: DVec2::new(-2., 0.),
                direction: DVec2::Y,
            },
        ];
        let Program::Infeasible { at, seed } =
            program(&lines, 1., DVec2::ZERO, false, &mut || true).unwrap()
        else {
            panic!("opposed constraints must be infeasible");
        };
        let value = refine(&lines, 0, at, 1., seed, &mut || true).unwrap();
        assert!(value.is_finite() && value.length() <= 1.);
        let invalid = [Line {
            point: DVec2::splat(f64::MAX),
            direction: DVec2::X,
        }];
        assert!(matches!(
            program(&invalid, 1., DVec2::ZERO, false, &mut || true),
            Err(Failure::Arithmetic)
        ));
        let actor = agent(0., 0., 0., 0.);
        assert!(solve_relaxed(actor, &[actor], 0, DVec2::X, 1., 0.1, 0.2, &mut || true).is_none());
    }
    #[test]
    fn feasible_primary_status_preserves_optional_bias_eligibility() {
        let actor = agent(0., 0., 0., 0.);
        let result =
            solve_with_status(actor, &[], 0, DVec2::X, 1., 0.1, 0.2, &mut || true).unwrap();
        assert!(!result.refined);
        assert_eq!(result.velocity, DVec2::X);
    }
    #[test]
    fn reciprocal_diagnostic_preserves_hard_lines_and_certifies_joint_head_on_step() {
        let a = agent(-1., 0., 0., 0.);
        let b = agent(1., 0., 0., 0.);
        let av = solve_with_share(a, &[b], 0, DVec2::X * 10., 10., 0.05, 0.1, 0.5, &mut || {
            true
        })
        .unwrap()
        .velocity;
        let bv = solve_with_share(
            b,
            &[a],
            0,
            -DVec2::X * 10.,
            10.,
            0.05,
            0.1,
            0.5,
            &mut || true,
        )
        .unwrap()
        .velocity;
        assert!(certified(a, av, b, bv, 0.05));
        let stopped = agent(2., 0., 0., 0.);
        let full = solve_with_status(
            a,
            &[stopped],
            1,
            DVec2::X * 100.,
            100.,
            0.05,
            0.1,
            &mut || true,
        )
        .unwrap();
        let half = solve_with_share(
            a,
            &[stopped],
            1,
            DVec2::X * 100.,
            100.,
            0.05,
            0.1,
            0.5,
            &mut || true,
        )
        .unwrap();
        assert_eq!(full.velocity, half.velocity);
        for share in [0., -0.5, 1.1, f64::NAN] {
            assert!(
                solve_with_share(a, &[], 0, DVec2::X, 100., 0.05, 0.1, share, &mut || true)
                    .is_none()
            );
        }
    }
}
