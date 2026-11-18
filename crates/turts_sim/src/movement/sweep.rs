//! Certify complete body trajectories before a frame leaves the movement kernel.
//! Endpoint projection alone does not certify the chord between legal endpoints.
mod bounded;
mod support;
use super::{
    MovementActivity, MovementFrame, MovementFrameOutput, MovementFrameUnit, PhysicalEscape,
};
use crate::grid::{GridPoint, UniformGrid};
use bevy_math::Vec2;
pub(crate) use bounded::SweepBudget;
use turts_rules::terrain::geometry::{
    Budget, Point, swept_disks_clear, swept_disks_non_deepening_overlap,
};

fn point(value: Vec2) -> Point {
    Point {
        x: f64::from(value.x),
        y: f64::from(value.y),
    }
}
fn length(a: Vec2, b: Vec2) -> f64 {
    (f64::from(a.x) - f64::from(b.x)).hypot(f64::from(a.y) - f64::from(b.y))
}
/// Projection may separate endpoints beyond the intended step. Bound the actual
/// chord before certification, allowing only coordinate-rounding error.
fn travel_endpoint(unit: &MovementFrameUnit, from: Vec2, to: Vec2, dt: f32) -> Vec2 {
    let Some(speed) = unit.speed else {
        return from;
    };
    if !from.is_finite()
        || !to.is_finite()
        || !speed.is_finite()
        || speed <= 0.
        || !dt.is_finite()
        || dt <= 0.
    {
        return from;
    }
    let ulp = |value: f32| {
        let value = value.abs();
        let next = value.next_up();
        if next.is_finite() {
            f64::from(next) - f64::from(value)
        } else {
            f64::from(value) - f64::from(value.next_down())
        }
    };
    let limit = f64::from(speed) * f64::from(dt);
    let rounding = 4. * (ulp(from.x) + ulp(from.y) + ulp(to.x) + ulp(to.y));
    let distance = length(from, to);
    if distance <= limit + rounding {
        return to;
    }
    let fraction = limit / distance;
    Vec2::new(
        (f64::from(from.x) + (f64::from(to.x) - f64::from(from.x)) * fraction) as f32,
        (f64::from(from.y) + (f64::from(to.y) - f64::from(from.y)) * fraction) as f32,
    )
}
pub(crate) fn pair_clear(
    a: Vec2,
    to: Vec2,
    radius: f32,
    b: Vec2,
    b_to: Vec2,
    b_radius: f32,
) -> bool {
    if !a.is_finite()
        || !to.is_finite()
        || !b.is_finite()
        || !b_to.is_finite()
        || !radius.is_finite()
        || !b_radius.is_finite()
        || radius <= 0.0
        || b_radius <= 0.0
    {
        return false;
    }
    // Outward-rounded relative bounds only reject pairs that cannot touch.
    // Every remaining pair receives the exact checked-rational certificate.
    let r = (f64::from(radius) + f64::from(b_radius)).next_up();
    let outside = |a0: f32, a1: f32, b0: f32, b1: f32| {
        let first = f64::from(a0) - f64::from(b0);
        let last = f64::from(a1) - f64::from(b1);
        first.min(last).next_down() >= r || first.max(last).next_up() <= -r
    };
    if outside(a.x, to.x, b.x, b_to.x) || outside(a.y, to.y, b.y, b_to.y) {
        return true;
    }
    let mut budget = Budget::new(2);
    swept_disks_clear(
        point(a),
        point(to),
        f64::from(radius),
        point(b),
        point(b_to),
        f64::from(b_radius),
        &mut budget,
    )
    .and_then(|clear| {
        if clear {
            Ok(true)
        } else {
            // Existing overlap recovery is not a legal-start sweep. It may only
            // increase separation; no new overlap or deeper penetration is allowed.
            swept_disks_non_deepening_overlap(
                point(a),
                point(to),
                f64::from(radius),
                point(b),
                point(b_to),
                f64::from(b_radius),
                &mut budget,
            )
        }
    })
    .unwrap_or(false)
}
/// Conservative extra peak for a copied permitted planning frame, pool slots,
/// grid/hash growth, and the mutually exclusive certificate/support phases.
pub(crate) fn planning_scratch_bound(count: usize, slots: usize) -> Option<usize> {
    if count > crate::terrain::traffic::index_capacity() || slots > 4 * 4096 {
        return None;
    }
    let index = crate::terrain::traffic::EnvelopeIndex::capacity_bound(count);
    let collect = index
        .checked_add(count.checked_mul(90)?)?
        .checked_add(2 * count.div_ceil(64) * std::mem::size_of::<u64>())?
        .checked_add(bounded::PEERS * std::mem::size_of::<usize>())?
        .checked_add(bounded::WORK / 2 * std::mem::size_of::<usize>() + 4096)?;
    let support = index
        .checked_add(count.checked_mul(61)?)?
        .checked_add(4096)?;
    if collect.max(support) > bounded::SCRATCH_BYTES {
        return None;
    }
    count
        .checked_mul(2048)?
        .checked_add(slots.checked_mul(1024)?)?
        .checked_add(index.checked_mul(2)?)?
        .checked_add(bounded::WORK / 2 * std::mem::size_of::<usize>() + 65_536)
}

pub(crate) fn progress(
    previous: Option<(Vec2, f32, u16)>,
    from: Vec2,
    to: Vec2,
    slot: Vec2,
    step: f32,
) -> (f32, u16) {
    let (best, stalled) = previous
        .filter(|(point, _, _)| *point == slot)
        .map_or((from.distance(slot), 0), |(_, best, stalled)| {
            (best, stalled)
        });
    let distance = to.distance(slot);
    if distance < best - 0.01 * step {
        (distance, 0)
    } else {
        (best, stalled.saturating_add(1))
    }
}
/// The existing solver's physical arrival order is retained. Earlier certified
/// trajectories are fixed. Later bodies remain stationary until their own turn.
/// Rechecking all relative paths therefore cannot roll a committed peer back
/// through a follower. Opaque IDs are used only to find the original state row.
#[cfg(test)]
#[allow(
    clippy::too_many_arguments,
    reason = "Preserve legacy certification fixtures"
)]
pub(super) fn certify(
    frame: &MovementFrame,
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    points: &mut [GridPoint],
    grid: &mut UniformGrid,
    dt: f32,
    max_radius: f32,
    budget: &mut SweepBudget,
) {
    certify_guided(
        frame,
        units,
        origins,
        outputs,
        points,
        grid,
        dt,
        max_radius,
        budget,
        &super::motion::NoMotionGuide,
    );
}

#[allow(
    clippy::too_many_arguments,
    reason = "Reuse the frame kernel's existing spatial storage and original pose vector"
)]
pub(super) fn certify_guided(
    frame: &MovementFrame,
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    points: &mut [GridPoint],
    grid: &mut UniformGrid,
    dt: f32,
    _max_radius: f32,
    budget: &mut SweepBudget,
    guide: &dyn super::motion::MotionGuide,
) {
    // Flat scalar arrays only; never clone nested command or progress buffers.
    let admitted = units.len() <= crate::terrain::traffic::index_capacity();
    if !admitted {
        budget.overflows = budget.overflows.saturating_add(units.len());
        for index in 0..units.len() {
            let proposed = outputs[index].position;
            outputs[index].position = origins[index];
            finish(
                frame,
                &units[index],
                origins[index],
                proposed,
                &mut outputs[index],
                dt,
            );
            let old = points[index].position;
            points[index].position = origins[index];
            grid.move_point(index, old, origins[index]);
        }
        return;
    }
    let deferred = if admitted && guide.deferred_support_enabled() {
        support::deferred_eligibility(frame, units, origins, outputs, dt, guide)
    } else {
        Vec::new()
    };
    let raw = if admitted {
        outputs.iter().map(|row| row.position).collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut spent = if admitted {
        vec![0usize; units.len()]
    } else {
        Vec::new()
    };
    let mut committed = if admitted {
        vec![false; units.len()]
    } else {
        Vec::new()
    };
    let candidates = std::cell::RefCell::new(bounded::collect_guided_deferred(
        units,
        origins,
        outputs,
        frame.bounds,
        budget,
        guide,
        &deferred,
    ));
    #[cfg(feature = "test-probes")]
    let trace_ids = std::env::var("TURTS_BODY_TRACE_IDS").ok();
    for (index, origin) in origins.iter().enumerate() {
        let old = points[index].position;
        points[index].position = *origin;
        grid.move_point(index, old, *origin);
    }
    for index in 0..units.len() {
        let from = origins[index];
        // Keep raw[] unchanged for finish(): a clipped raw arrival must regain
        // its unfinished order. Every geometric alternative uses this limit.
        let proposed = travel_endpoint(&units[index], from, outputs[index].position, dt);
        #[cfg(feature = "test-probes")]
        let tracing = trace_ids.as_ref().is_some_and(|ids| {
            ids.split(',')
                .any(|id| id.parse::<u32>().ok() == Some(units[index].unit_id))
        });
        #[cfg(feature = "test-probes")]
        if tracing {
            eprintln!(
                "BODY_BEGIN tick={} unit={} from={from:?} target={:?} raw={:?} bounded={proposed:?}",
                budget.epoch, units[index].unit_id, units[index].target, raw[index]
            );
        }
        if deferred.get(index).copied().unwrap_or(false) {
            // Hold geometry only. Untouched raw semantics remain for support and finish.
            outputs[index].position = from;
            committed[index] = true;
            candidates.borrow_mut().mark_committed(index);
            continue;
        }
        // A stationary body is already protected by every earlier certificate.
        if proposed == from {
            outputs[index].position = from;
            if admitted {
                committed[index] = true;
                candidates.borrow_mut().mark_committed(index);
            }
            continue;
        }
        // Candidate storage is bounded before certification starts. Query failures
        // retain the original stationary body, which every earlier path certifies.
        let (mut neighbors, allowance) = candidates.borrow().get(index).map_or_else(
            || (Vec::new(), 0),
            |(rows, allowance)| (rows.to_vec(), allowance),
        );
        neighbors.sort_unstable_by(|&a, &b| {
            points[a]
                .position
                .x
                .total_cmp(&points[b].position.x)
                .then(points[a].position.y.total_cmp(&points[b].position.y))
                .then(a.cmp(&b))
        });
        let seen = std::cell::RefCell::new(neighbors.clone());
        let used = std::cell::Cell::new(0usize);
        let exhausted = std::cell::Cell::new(false);
        #[cfg(feature = "test-probes")]
        let traced_pairs = std::cell::Cell::new(0usize);
        let budget = std::cell::RefCell::new(&mut *budget);
        let check = |to: Vec2, other: usize| {
            let mut count = used.get();
            if !guide.charge_certificate_pair(
                units[index].unit_id,
                units[index].owner_slot,
                &mut count,
                allowance,
                &mut budget.borrow_mut(),
            ) {
                exhausted.set(true);
                return false;
            }
            used.set(count);
            let peer_to = if committed.get(other).copied().unwrap_or(false) {
                outputs[other].position
            } else {
                origins[other]
            };
            let clear = pair_clear(
                from,
                to,
                units[index].radius,
                origins[other],
                peer_to,
                units[other].radius,
            );
            #[cfg(feature = "test-probes")]
            if tracing && !clear && traced_pairs.get() < 8 {
                traced_pairs.set(traced_pairs.get() + 1);
                eprintln!(
                    "BODY_BLOCK tick={} unit={} peer={} earlier={} to={to:?} peer_from={:?} peer_to={peer_to:?}",
                    budget.borrow().epoch,
                    units[index].unit_id,
                    units[other].unit_id,
                    committed.get(other).copied().unwrap_or(false),
                    origins[other]
                );
            }
            clear
        };
        let clear = |to: Vec2| {
            if exhausted.get() {
                return false;
            }
            let rows = candidates.borrow_mut().query_guided(
                index,
                to,
                units,
                origins,
                outputs,
                &mut seen.borrow_mut(),
                &committed,
                &mut budget.borrow_mut(),
                guide,
            );
            let Some(rows) = rows else {
                exhausted.set(true);
                return false;
            };
            rows.into_iter().all(|other| check(to, other))
        };
        let mut accepted = proposed;
        if !clear(proposed) {
            accepted = from;
            if guide.certificate_search_enabled() {
                let limit = length(from, proposed);
                let mut detour = proposed;
                // Use the existing short-range separation neighborhood. A distant
                // unseen body can only shorten travel to contact, never steer early.
                for _ in 0..8 {
                    let mut changed = false;
                    for &other in &neighbors {
                        if other == index {
                            continue;
                        }
                        let peer_to = if committed.get(other).copied().unwrap_or(false) {
                            outputs[other].position
                        } else {
                            origins[other]
                        };
                        if check(detour, other) {
                            continue;
                        }
                        let normal = from.as_dvec2() - origins[other].as_dvec2();
                        let square = normal.length_squared();
                        let near = f64::from(units[index].radius + units[other].radius + 0.05);
                        if square == 0.0 || square >= near * near {
                            continue;
                        }
                        let peer_motion = peer_to.as_dvec2() - origins[other].as_dvec2();
                        let motion = detour.as_dvec2() - from.as_dvec2();
                        let inward = normal.dot(motion - peer_motion);
                        if inward >= 0.0 {
                            continue;
                        }
                        // Remove the inward relative component instead of joining two
                        // projected circle endpoints by an unsafe interior chord.
                        let mut slide = motion - normal * (inward / square);
                        let distance = slide.length();
                        if distance > limit {
                            slide *= limit / distance;
                        }
                        let mut candidate = (from.as_dvec2() + slide).as_vec2();
                        if candidate == from || length(from, candidate) > limit {
                            continue;
                        }
                        for _ in 0..2 {
                            if check(candidate, other) {
                                break;
                            }
                            // Round a tangent proposal outward by representable steps.
                            // This is motion construction, never collision tolerance.
                            let x = if normal.x > 0.0 {
                                candidate.x.next_up()
                            } else if normal.x < 0.0 {
                                candidate.x.next_down()
                            } else {
                                candidate.x
                            };
                            let y = if normal.y > 0.0 {
                                candidate.y.next_up()
                            } else if normal.y < 0.0 {
                                candidate.y.next_down()
                            } else {
                                candidate.y
                            };
                            let x_gain =
                                (f64::from(x) - f64::from(candidate.x)).abs() * normal.x.abs();
                            let y_gain =
                                (f64::from(y) - f64::from(candidate.y)).abs() * normal.y.abs();
                            let rounded = if x_gain >= y_gain {
                                Vec2::new(x, candidate.y)
                            } else {
                                Vec2::new(candidate.x, y)
                            };
                            if length(from, rounded) > limit {
                                break;
                            }
                            candidate = rounded;
                        }
                        if candidate != detour {
                            detour = candidate;
                            changed = true;
                        }
                    }
                    if detour != from && clear(detour) {
                        accepted = detour;
                        break;
                    }
                    if !changed {
                        break;
                    }
                }
                if accepted == from && clear(from) {
                    // Each retained f32 pose is independently certified. This does not
                    // assume that safe relative velocities form a monotone interval.
                    let (mut lower, mut upper) = (0u32, 1u32 << 24);
                    for _ in 0..24 {
                        let middle = lower + (upper - lower) / 2;
                        let fraction = f64::from(middle) / f64::from(1u32 << 24);
                        let candidate = Vec2::new(
                            (f64::from(from.x)
                                + (f64::from(proposed.x) - f64::from(from.x)) * fraction)
                                as f32,
                            (f64::from(from.y)
                                + (f64::from(proposed.y) - f64::from(from.y)) * fraction)
                                as f32,
                        );
                        if clear(candidate) {
                            lower = middle;
                            accepted = candidate;
                        } else {
                            upper = middle;
                        }
                    }
                }
            }
        }
        if exhausted.get() {
            budget.borrow_mut().overflows += 1;
        }
        outputs[index].position = accepted;
        if admitted {
            spent[index] = used.get();
            committed[index] = true;
        }
        #[cfg(feature = "test-probes")]
        if tracing {
            eprintln!(
                "BODY_END tick={} unit={} accepted={accepted:?} checks={} allowance={allowance} exhausted={}",
                budget.borrow().epoch,
                units[index].unit_id,
                used.get(),
                exhausted.get()
            );
        }
        candidates.borrow_mut().committed(
            index,
            from,
            accepted,
            units[index].radius,
            &mut budget.borrow_mut(),
        );
        let old = points[index].position;
        points[index].position = accepted;
        grid.move_point(index, old, accepted);
    }
    if let Some(index) = candidates.borrow_mut().index_work() {
        budget.index_work.add(index);
    }
    // Reuse the no-longer-needed commit flags as baseline admission facts.
    for (i, flag) in committed.iter_mut().enumerate() {
        *flag = candidates.borrow().get(i).is_some();
    }
    drop(candidates);
    if admitted {
        support::upgrade_guided_deferred(
            frame, units, origins, &raw, outputs, &mut spent, &committed, &deferred, dt, budget,
            guide,
        );
    }
    for index in 0..units.len() {
        finish(
            frame,
            &units[index],
            origins[index],
            raw[index],
            &mut outputs[index],
            dt,
        );
    }
}

fn finish(
    frame: &MovementFrame,
    unit: &MovementFrameUnit,
    from: Vec2,
    proposed: Vec2,
    output: &mut MovementFrameOutput,
    dt: f32,
) {
    let accepted = output.position;
    if accepted != proposed {
        let Some(original) = frame
            .units
            .iter()
            .find(|original| original.unit_id == unit.unit_id)
        else {
            output.position = from;
            return;
        };
        output.position = accepted;
        // A direct guided request can have no legacy progress row. Its raw
        // improvement is not earned when the final body certificate clips it.
        // Preserve that refusal as ordinary recovery feedback, just as for an
        // existing progress row, without changing the certified pose or goal.
        if !output.failed_blocked
            && output.progress.is_none()
            && output.plain_progress.is_none()
            && original.progress.is_none()
            && unit
                .speed
                .is_some_and(|speed| speed.is_finite() && speed > 0.)
            && let Some(slot) = unit.target.filter(|point| point.is_finite())
        {
            output.plain_progress = original.plain_progress.clone().or_else(|| {
                Some(super::PlainMoveProgress {
                    arrival_yield: false,
                    original_point: original.target.unwrap_or(slot),
                    slot,
                    pool_slots: Vec::new(),
                    gather_goal: None,
                    activity: MovementActivity::Moving,
                    stalled_ticks: 0,
                    best_distance: from.distance(slot),
                    claim_count: 0,
                    failure_recorded: false,
                })
            });
        }

        if !output.failed_blocked {
            output.target = output
                .progress
                .as_ref()
                .map(|p| p.slot)
                .or_else(|| output.plain_progress.as_ref().map(|p| p.slot))
                .or(original.target);
        }
        if let Some(p) = &mut output.progress {
            let previous = original
                .progress
                .as_ref()
                .map(|p| (p.slot, p.best_distance, p.stalled_ticks));
            let recovered = original.progress.as_ref().is_some_and(|old| {
                old.pass_mode != p.pass_mode
                    || old.claim_count != p.claim_count
                    || old.slot != p.slot
            });
            (p.best_distance, p.stalled_ticks) = if recovered {
                (accepted.distance(p.slot), 0)
            } else {
                progress(
                    previous,
                    from,
                    accepted,
                    p.slot,
                    unit.speed.unwrap_or(0.0) * dt,
                )
            };
            if p.activity == MovementActivity::Resting {
                p.activity = MovementActivity::Moving;
            }
        }
        if let Some(p) = &mut output.plain_progress {
            let previous = original
                .plain_progress
                .as_ref()
                .map(|p| (p.slot, p.best_distance, p.stalled_ticks));
            let recovered = original
                .plain_progress
                .as_ref()
                .is_some_and(|old| old.claim_count != p.claim_count || old.slot != p.slot);
            (p.best_distance, p.stalled_ticks) = if recovered {
                (accepted.distance(p.slot), 0)
            } else {
                progress(
                    previous,
                    from,
                    accepted,
                    p.slot,
                    unit.speed.unwrap_or(0.0) * dt,
                )
            };
            if p.activity == MovementActivity::Resting {
                p.activity = MovementActivity::Moving;
            }
        }
        if let Some(escape) = &original.physical_escape {
            output.physical_escape = Some(PhysicalEscape {
                target: escape.target.or(Some(proposed)),
            });
            if output.progress.is_none() {
                output.progress = original.progress.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn projected_raw_arrival_stays_within_speed_and_keeps_unfinished_order() {
        let from = Vec2::new(26.903395, 24.681269);
        let proposed = Vec2::new(27.017876, 24.547909);
        let mut unit = body(1, from, 0.3, Some(proposed));
        unit.speed = Some(3.5);
        let units = [unit.clone()];
        let frame = MovementFrame {
            units: units.to_vec(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        let mut outputs = [output(&unit, proposed)];
        outputs[0].target = None;
        let mut points = [GridPoint {
            id: 1,
            position: from,
        }];
        let mut grid = UniformGrid::new(&points, 0.6);
        certify(
            &frame,
            &units,
            &[from],
            &mut outputs,
            &mut points,
            &mut grid,
            0.05,
            0.3,
            &mut SweepBudget::new(0),
        );
        assert!(
            length(from, outputs[0].position) <= 0.17501,
            "projection exceeded the speed limit: {:?}",
            outputs[0].position
        );
        assert_eq!(
            outputs[0].target,
            Some(proposed),
            "clipped arrival lost its order"
        );
    }
    #[test]
    fn a_travel_cap_that_rounds_to_the_origin_cannot_leave_the_raw_endpoint() {
        for speed in [None, Some(0.), Some(f32::MIN_POSITIVE)] {
            let from = Vec2::new(26., 24.);
            let proposed = from + Vec2::X;
            let mut unit = body(1, from, 0.3, Some(proposed));
            unit.speed = speed;
            let units = [unit.clone()];
            let frame = MovementFrame {
                units: units.to_vec(),
                bounds: Vec2::splat(128.),
                tick_hz: 20,
                pools: BTreeMap::new(),
            };
            let mut outputs = [output(&unit, proposed)];
            let mut points = [GridPoint {
                id: 1,
                position: from,
            }];
            let mut grid = UniformGrid::new(&points, 0.6);
            certify(
                &frame,
                &units,
                &[from],
                &mut outputs,
                &mut points,
                &mut grid,
                0.05,
                0.3,
                &mut SweepBudget::new(0),
            );
            assert_eq!(outputs[0].position, from, "speed={speed:?}");
            assert_eq!(outputs[0].target, Some(proposed));
        }
    }
    #[test]
    fn thousand_fast_parallel_chords_preserve_full_motion_and_handle_inversion() {
        fn run(reverse: bool) -> Vec<Vec2> {
            let units = (0..1000)
                .map(|n| {
                    let from = Vec2::new(2. + (n % 32) as f32 * 6., 2. + (n / 32) as f32);
                    let mut u = body(
                        if reverse { 1000 - n } else { n + 1 },
                        from,
                        0.4,
                        Some(from + Vec2::X * 20.),
                    );
                    u.speed = Some(100.);
                    u
                })
                .collect::<Vec<_>>();
            let frame = MovementFrame {
                bounds: Vec2::new(256., 64.),
                tick_hz: 20,
                units: units.clone(),
                pools: BTreeMap::new(),
            };
            let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
            let mut outputs = units
                .iter()
                .map(|u| output(u, u.position + Vec2::X * 5.))
                .collect::<Vec<_>>();
            let mut points = units
                .iter()
                .map(|u| GridPoint {
                    id: u.unit_id,
                    position: u.position,
                })
                .collect::<Vec<_>>();
            let mut grid = UniformGrid::new(&points, 0.8);
            let mut budget = SweepBudget::new(0);
            let mut frame = frame;
            if reverse {
                frame.units.reverse();
            }
            certify(
                &frame,
                &units,
                &origins,
                &mut outputs,
                &mut points,
                &mut grid,
                0.05,
                0.4,
                &mut budget,
            );
            assert_eq!(budget.overflows, 0);
            assert_eq!(budget.index_rebuilds, 0);
            assert!(budget.peak_bytes <= bounded::SCRATCH_BYTES);
            for (u, o) in units.iter().zip(&outputs) {
                assert_eq!(o.position, u.position + Vec2::X * 5.);
            }
            outputs.iter().map(|o| o.position).collect()
        }
        assert_eq!(run(false), run(true));
    }
    #[test]
    fn parallel_diagonal_chords_get_credit_for_newly_relevant_certified_peers() {
        fn run(reverse: bool) -> Vec<Vec2> {
            let units = (0..1000)
                .map(|n| {
                    let from = Vec2::new(2., 2. + n as f32);
                    let mut u = body(
                        if reverse { 1000 - n } else { n + 1 },
                        from,
                        0.4,
                        Some(from + Vec2::X * 20.),
                    );
                    // The frozen (5,1) chord is longer than a five-unit step.
                    u.speed = Some(110.);
                    u
                })
                .collect::<Vec<_>>();
            let frame = MovementFrame {
                bounds: Vec2::splat(2048.),
                tick_hz: 20,
                units: units.clone(),
                pools: BTreeMap::new(),
            };
            let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
            let mut outputs = units
                .iter()
                .map(|u| output(u, u.position + Vec2::new(5., 1.)))
                .collect::<Vec<_>>();
            let mut points = units
                .iter()
                .map(|u| GridPoint {
                    id: u.unit_id,
                    position: u.position,
                })
                .collect::<Vec<_>>();
            let mut grid = UniformGrid::new(&points, 0.8);
            let mut budget = SweepBudget::new(0);
            let mut frame = frame;
            if reverse {
                frame.units.reverse();
            }
            certify(
                &frame,
                &units,
                &origins,
                &mut outputs,
                &mut points,
                &mut grid,
                0.05,
                0.4,
                &mut budget,
            );
            assert_eq!(budget.overflows, 0);
            assert_eq!(budget.index_rebuilds, 0);
            assert!(budget.peak_bytes <= bounded::SCRATCH_BYTES);
            for (u, o) in units.iter().zip(&outputs) {
                assert_eq!(o.position, u.position + Vec2::new(5., 1.));
            }
            outputs.iter().map(|o| o.position).collect()
        }
        assert_eq!(run(false), run(true));
    }
    fn body(id: u32, position: Vec2, radius: f32, target: Option<Vec2>) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 0,
            position,
            radius,
            // These tests inject complete geometric chords directly. Give each
            // chord sufficient travel; speed-limit regressions override this.
            speed: target.map(|_| 1000.),
            target,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn output(unit: &MovementFrameUnit, position: Vec2) -> MovementFrameOutput {
        MovementFrameOutput {
            unit_id: unit.unit_id,
            position,
            target: None,
            progress: None,
            plain_progress: None,
            physical_escape: None,
            failed_blocked: false,
            arrival_slot: None,
            #[cfg(feature = "test-probes")]
            probe: Default::default(),
        }
    }
    fn verify(
        units: Vec<MovementFrameUnit>,
        ends: Vec<Vec2>,
        reverse_input: bool,
    ) -> Vec<MovementFrameOutput> {
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut outputs = units
            .iter()
            .zip(ends)
            .map(|(unit, end)| output(unit, end))
            .collect::<Vec<_>>();
        let mut frame = MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            units: units.clone(),
            pools: BTreeMap::new(),
        };
        if reverse_input {
            frame.units.reverse();
        }
        let mut points = outputs
            .iter()
            .map(|row| GridPoint {
                id: row.unit_id,
                position: row.position,
            })
            .collect::<Vec<_>>();
        let max_radius = units.iter().map(|u| u.radius).fold(0.0_f32, f32::max);
        let mut grid = UniformGrid::new(&points, 2.0 * max_radius);
        let mut budget = SweepBudget::new(0);
        certify(
            &frame,
            &units,
            &origins,
            &mut outputs,
            &mut points,
            &mut grid,
            0.05,
            max_radius,
            &mut budget,
        );
        assert!(
            budget.pairs.iter().sum::<usize>() > 0,
            "known physical sweep cases must exercise real pair accounting"
        );
        assert!(
            budget.peak_bytes > 0,
            "real index storage must be accounted"
        );
        outputs
    }
    // Independent elementary relative-motion oracle for these exactly representable
    // small scenes. It does not invoke the production rational predicate.
    fn minimum(a: Vec2, to: Vec2, b: Vec2, b_to: Vec2) -> f64 {
        let r = a.as_dvec2() - b.as_dvec2();
        let v = (to.as_dvec2() - a.as_dvec2()) - (b_to.as_dvec2() - b.as_dvec2());
        let t = if v.length_squared() == 0.0 {
            0.0
        } else {
            (-r.dot(v) / v.length_squared()).clamp(0.0, 1.0)
        };
        (r + t * v).length_squared()
    }
    #[test]
    fn clear_endpoints_do_not_certify_the_reviewed_gm6_chord() {
        let fixed = Vec2::new(29.020203, 24.5);
        let from = Vec2::new(29.714956, 25.829517);
        let to = Vec2::new(29.84633, 25.752123);
        let r = f64::from(1.1f32) + f64::from(0.4f32);
        assert!((from.as_dvec2() - fixed.as_dvec2()).length_squared() > r * r);
        assert!((to.as_dvec2() - fixed.as_dvec2()).length_squared() > r * r);
        assert!(
            minimum(from, to, fixed, fixed) < r * r,
            "the original committed chord is independently red"
        );
        let mut traces = Vec::new();
        for invert in [false, true] {
            let units = vec![
                body(if invert { 99 } else { 1 }, fixed, 1.1, None),
                body(if invert { 1 } else { 99 }, from, 0.4, Some(to)),
            ];
            let outputs = verify(units, vec![fixed, to], invert);
            assert_eq!(outputs[0].position, fixed, "the settled holder cannot move");
            assert_ne!(outputs[1].position, to);
            assert_eq!(
                outputs[1].target,
                Some(to),
                "shortening does not complete the order"
            );
            assert!(minimum(from, outputs[1].position, fixed, fixed) >= r * r);
            traces.push(outputs[1].position);
        }
        assert_eq!(
            traces[0], traces[1],
            "handles and input enumeration are not physical priority"
        );
    }
    #[test]
    fn short_free_goal_around_a_settled_body_finishes_without_cutting_its_circle() {
        let fixed = Vec2::new(29.020203, 24.5);
        let from = Vec2::new(29.714956, 25.829517);
        let target = Vec2::new(29.84633, 25.752123);
        let radius = f64::from(1.1f32) + f64::from(0.4f32);
        for invert in [false, true] {
            let moving = if invert { 1 } else { 99 };
            let mut frame = MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: BTreeMap::new(),
                units: vec![
                    body(if invert { 99 } else { 1 }, fixed, 1.1, None),
                    body(moving, from, 0.4, Some(target)),
                ],
            };
            if invert {
                frame.units.reverse();
            }
            for _ in 0..20 {
                let previous = frame
                    .units
                    .iter()
                    .find(|u| u.unit_id == moving)
                    .unwrap()
                    .position;
                let outputs = crate::movement::step_frame(&frame, 0.05);
                for output in outputs {
                    let actor = frame
                        .units
                        .iter_mut()
                        .find(|u| u.unit_id == output.unit_id)
                        .unwrap();
                    if actor.unit_id == moving {
                        assert!(
                            minimum(previous, output.position, fixed, fixed) >= radius * radius
                        );
                    } else {
                        assert_eq!(output.position, fixed, "settled unit is never evicted");
                    }
                    actor.position = output.position;
                    actor.target = output.target;
                    actor.progress = output.progress;
                    actor.plain_progress = output.plain_progress;
                    actor.physical_escape = output.physical_escape;
                }
            }
            let actor = frame.units.iter().find(|u| u.unit_id == moving).unwrap();
            assert_eq!(
                actor.position, target,
                "a verified prefix alone cannot replace feasible completion"
            );
            assert_eq!(actor.target, None);
        }
    }

    #[test]
    fn exact_tangent_chord_remains_live() {
        let from = Vec2::new(8.0, 11.0);
        let to = Vec2::new(12.0, 11.0);
        let fixed = Vec2::splat(10.0);
        let outputs = verify(
            vec![body(1, fixed, 0.5, None), body(2, from, 0.5, Some(to))],
            vec![fixed, to],
            false,
        );
        assert_eq!(outputs[1].position, to);
        assert_eq!(minimum(from, to, fixed, fixed), 1.0);
    }
    #[test]
    fn moving_peers_are_certified_by_relative_paths_not_final_spacing() {
        let a = Vec2::new(8.0, 10.0);
        let a_to = Vec2::new(12.0, 10.0);
        let b = Vec2::new(10.0, 8.0);
        let b_to = Vec2::new(10.0, 12.0);
        assert_eq!(minimum(a, a_to, b, b_to), 0.0);
        let outputs = verify(
            vec![body(1, a, 0.5, Some(a_to)), body(2, b, 0.5, Some(b_to))],
            vec![a_to, b_to],
            false,
        );
        assert_eq!(outputs[0].position, a_to);
        assert!(minimum(a, outputs[0].position, b, outputs[1].position) >= 1.0);
        assert_eq!(outputs[1].target, Some(b_to));
    }
    #[test]
    fn invalid_initial_overlap_can_separate_without_creating_deeper_contact() {
        let from = Vec2::splat(10.0);
        let left = Vec2::new(9.0, 10.0);
        let right = Vec2::new(11.0, 10.0);
        let outputs = verify(
            vec![
                body(1, from, 0.5, Some(left)),
                body(2, from, 0.5, Some(right)),
            ],
            vec![left, right],
            false,
        );
        assert_eq!(outputs[0].position, left);
        assert_eq!(outputs[1].position, right);
    }
}

#[cfg(test)]
mod planning_tests {
    use super::*;
    use crate::movement::motion::{MotionGuide, MotionKind, MotionRequest};
    use std::{
        cell::{Cell, RefCell},
        collections::{BTreeMap, BTreeSet},
    };

    struct PlanGuide {
        spent: RefCell<BTreeMap<u32, usize>>,
        cap: usize,
        visits: Cell<usize>,
        pairs: Cell<usize>,
        deferred: bool,
    }
    impl PlanGuide {
        fn new(initial: usize, cap: usize) -> Self {
            Self {
                spent: RefCell::new([(1, initial), (2, initial)].into()),
                cap,
                visits: Cell::new(0),
                pairs: Cell::new(0),
                deferred: false,
            }
        }
    }
    impl MotionGuide for PlanGuide {
        fn deferred_support_enabled(&self) -> bool {
            self.deferred
        }
        fn kind(&self, _: u32) -> MotionKind {
            MotionKind::Waypoint
        }
        fn arrival_destination(&self, _: u32) -> Option<Vec2> {
            Some(Vec2::new(20., 2.))
        }
        fn next_position(&mut self, request: MotionRequest, _: &mut SweepBudget) -> Option<Vec2> {
            Some(request.preferred_position())
        }
        fn certificate_search_enabled(&self) -> bool {
            false
        }
        fn charge_certificate_visit(
            &self,
            id: u32,
            owner: u8,
            _legacy: bool,
            budget: &mut SweepBudget,
        ) -> bool {
            let mut rows = self.spent.borrow_mut();
            let Some(spent) = rows.get_mut(&id) else {
                return false;
            };
            if !budget.can_charge_pair(owner, *spent, self.cap) || !budget.body(owner) {
                return false;
            }
            if !budget.charge_pair(owner, spent, self.cap) {
                return false;
            }
            self.visits.set(self.visits.get() + 1);
            true
        }
        fn examine_certificate_row(&self, id: u32, owner: u8, budget: &mut SweepBudget) -> bool {
            self.charge_certificate_visit(id, owner, false, budget)
        }
        fn admit_certificate_row(
            &self,
            _id: u32,
            _owner: u8,
            _legacy: bool,
            _budget: &mut SweepBudget,
        ) -> bool {
            true
        }
        fn charge_certificate_pair(
            &self,
            id: u32,
            owner: u8,
            local: &mut usize,
            allowance: usize,
            budget: &mut SweepBudget,
        ) -> bool {
            if *local >= allowance {
                return false;
            }
            let mut rows = self.spent.borrow_mut();
            let Some(spent) = rows.get_mut(&id) else {
                return false;
            };
            if !budget.charge_pair(owner, spent, self.cap) {
                return false;
            }
            *local += 1;
            self.pairs.set(self.pairs.get() + 1);
            true
        }
    }
    fn unit(id: u32, position: Vec2, target: Vec2, speed: f32) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 0,
            position,
            radius: 0.2,
            speed: Some(speed),
            target: Some(target),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn run(
        units: Vec<MovementFrameUnit>,
        initial: usize,
        cap: usize,
    ) -> (
        MovementFrame,
        Vec<MovementFrameOutput>,
        PlanGuide,
        SweepBudget,
    ) {
        let frame = MovementFrame {
            bounds: Vec2::splat(32.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        };
        let mut guide = PlanGuide::new(initial, cap);
        let mut budget = SweepBudget::new(1);
        let out = crate::movement::step_frame_with_motion_guide(
            &frame,
            1.,
            &BTreeSet::new(),
            &mut budget,
            &mut guide,
        );
        (frame, out, guide, budget)
    }
    #[test]
    fn deferred_dense_convoy_uses_one_complete_support_proof_under_same_ticket() {
        for reverse in [false, true] {
            let mut units = (0..8)
                .map(|i| {
                    unit(
                        i + 1,
                        Vec2::new(2. + i as f32 * 0.7, 2.),
                        Vec2::new(20., 2.),
                        5.,
                    )
                })
                .collect::<Vec<_>>();
            for u in &mut units {
                u.radius = 0.3;
            }
            if reverse {
                units.reverse();
                for u in &mut units {
                    u.unit_id = 100 - u.unit_id;
                }
            }
            let frame = MovementFrame {
                bounds: Vec2::splat(32.),
                tick_hz: 20,
                units,
                pools: BTreeMap::new(),
            };
            let mut guide = PlanGuide::new(108, 128);
            guide.deferred = true;
            *guide.spent.borrow_mut() = frame.units.iter().map(|u| (u.unit_id, 108)).collect();
            let mut budget = SweepBudget::new(1);
            let outputs = crate::movement::step_frame_with_motion_guide(
                &frame,
                1.,
                &BTreeSet::new(),
                &mut budget,
                &mut guide,
            );
            for output in &outputs {
                let u = frame
                    .units
                    .iter()
                    .find(|u| u.unit_id == output.unit_id)
                    .unwrap();
                assert_eq!(
                    output.position,
                    u.position + Vec2::X * 5.,
                    "unit {} must complete the safe5m convoy step",
                    u.unit_id
                );
            }
            for (i, a) in outputs.iter().enumerate() {
                for b in &outputs[..i] {
                    let ua = frame.units.iter().find(|u| u.unit_id == a.unit_id).unwrap();
                    let ub = frame.units.iter().find(|u| u.unit_id == b.unit_id).unwrap();
                    assert!(pair_clear(
                        ua.position,
                        a.position,
                        ua.radius,
                        ub.position,
                        b.position,
                        ub.radius
                    ));
                }
            }
            assert!(guide.spent.borrow().values().all(|&spent| spent <= 128));
            assert_eq!(
                guide.spent.borrow().values().sum::<usize>(),
                8 * 108 + budget.pairs[0]
            );
            assert!(budget.support_accepted_members >= 8);
        }
    }
    #[test]
    fn deferred_dependency_refusal_preserves_the_complete_held_baseline() {
        let units = (0..2)
            .map(|i| {
                unit(
                    i + 1,
                    Vec2::new(2. + i as f32 * 0.7, 2.),
                    Vec2::new(20., 2.),
                    5.,
                )
            })
            .collect::<Vec<_>>();
        let f = MovementFrame {
            bounds: Vec2::splat(32.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        };
        let origins = f.units.iter().map(|u| u.position).collect::<Vec<_>>();
        let raw = origins
            .iter()
            .map(|p| *p + Vec2::X * 5.)
            .collect::<Vec<_>>();
        let mut outputs = f
            .units
            .iter()
            .map(|u| MovementFrameOutput {
                unit_id: u.unit_id,
                position: u.position,
                target: u.target,
                progress: None,
                plain_progress: None,
                physical_escape: None,
                failed_blocked: false,
                arrival_slot: None,
                #[cfg(feature = "test-probes")]
                probe: Default::default(),
            })
            .collect::<Vec<_>>();
        let guide = PlanGuide::new(0, 128);
        guide.spent.borrow_mut().insert(2, 127);
        let mut budget = SweepBudget::new(1);
        support::upgrade_guided_deferred(
            &f,
            &f.units,
            &origins,
            &raw,
            &mut outputs,
            &mut [0; 2],
            &[false; 2],
            &[true; 2],
            1.,
            &mut budget,
            &guide,
        );
        assert_eq!(
            outputs.iter().map(|o| o.position).collect::<Vec<_>>(),
            origins
        );
        assert_eq!(guide.spent.borrow()[&2], 128);
        assert!(guide.spent.borrow()[&1] > 0);
        assert_eq!(
            outputs.iter().map(|o| o.target).collect::<Vec<_>>(),
            f.units.iter().map(|u| u.target).collect::<Vec<_>>()
        );
        assert!(budget.support_failures > 0);
    }
    #[test]
    fn cheap_planning_convoy_keeps_exact_support_and_shared_actor_work() {
        let (f, mut out, g, mut b) = run(
            vec![
                unit(1, Vec2::new(2., 2.), Vec2::new(20., 2.), 1.),
                unit(2, Vec2::new(2.6, 2.), Vec2::new(20., 2.), 1.),
            ],
            7,
            128,
        );
        assert_eq!(out[0].position, Vec2::new(3., 2.));
        assert_eq!(out[1].position, Vec2::new(3.6, 2.));
        assert!(pair_clear(
            f.units[0].position,
            out[0].position,
            0.2,
            f.units[1].position,
            out[1].position,
            0.2
        ));
        // Explicitly exercise a held baseline: ordinary physical ordering may
        // already advance the leader first without needing atomic support.
        let origins = f.units.iter().map(|u| u.position).collect::<Vec<_>>();
        let raw = out.iter().map(|u| u.position).collect::<Vec<_>>();
        for (row, &from) in out.iter_mut().zip(&origins) {
            row.position = from;
        }
        support::upgrade_guided(
            &f,
            &f.units,
            &origins,
            &raw,
            &mut out,
            &mut [0, 0],
            &[true, true],
            1.,
            &mut b,
            &g,
        );
        assert_eq!(out[0].position, raw[0]);
        assert_eq!(out[1].position, raw[1]);
        assert!(g.visits.get() > 0 && g.pairs.get() > 0 && b.support_queries > 0);
        assert_eq!(b.pairs[0], g.visits.get() + g.pairs.get());
        assert_eq!(g.spent.borrow().values().sum::<usize>(), 14 + b.pairs[0]);
        assert!(g.spent.borrow().values().all(|&n| n <= 128));
    }
    #[test]
    fn cheap_planning_crossing_keeps_a_complete_safe_baseline() {
        let (f, out, _, _) = run(
            vec![
                unit(1, Vec2::new(4., 5.), Vec2::new(6., 5.), 2.),
                unit(2, Vec2::new(5., 4.), Vec2::new(5., 6.), 2.),
            ],
            0,
            128,
        );
        assert!(pair_clear(
            f.units[0].position,
            out[0].position,
            0.2,
            f.units[1].position,
            out[1].position,
            0.2
        ));
        assert!(
            out.iter()
                .zip(&f.units)
                .any(|(a, b)| a.position == b.position)
        );
    }
    #[test]
    fn discovery_alone_can_exhaust_the_same_planning_actor_cap() {
        let (f, out, g, b) = run(
            vec![
                unit(1, Vec2::new(2., 2.), Vec2::new(20., 2.), 1.),
                unit(2, Vec2::new(2.6, 2.), Vec2::new(20., 2.), 1.),
            ],
            0,
            1,
        );
        assert!(
            out.iter()
                .zip(&f.units)
                .all(|(a, b)| a.position == b.position)
        );
        assert_eq!(g.pairs.get(), 0);
        assert_eq!(g.visits.get(), 2);
        assert_eq!(b.pairs[0], 2);
        assert!(g.spent.borrow().values().all(|&n| n == 1));
    }
    #[test]
    fn empty_planning_queries_are_charged_without_changing_default_guard_charges() {
        let u = unit(1, Vec2::new(2., 2.), Vec2::new(20., 2.), 1.);
        let (_, _, g, b) = run(vec![u.clone()], 0, 128);
        // Initial admission extracts self; the cached empty query pays only its admission.
        assert_eq!(g.visits.get(), 3);
        assert_eq!(b.pairs[0], 3);
        let f = MovementFrame {
            bounds: Vec2::splat(32.),
            tick_hz: 20,
            units: vec![u],
            pools: BTreeMap::new(),
        };
        let mut budget = SweepBudget::new(1);
        let mut legacy = crate::movement::motion::NoMotionGuide;
        crate::movement::step_frame_with_motion_guide(
            &f,
            1.,
            &BTreeSet::new(),
            &mut budget,
            &mut legacy,
        );
        assert_eq!(budget.bodies[0], 0);
        assert_eq!(budget.pairs[0], 0);
    }
    #[test]
    fn planning_capacity_refuses_unsupported_counts_before_allocation() {
        assert!(planning_scratch_bound(crate::terrain::traffic::index_capacity() + 1, 0).is_none());
        assert!(planning_scratch_bound(1, 16_385).is_none());
        assert!(
            planning_scratch_bound(500, 0).unwrap()
                > crate::terrain::traffic::EnvelopeIndex::capacity_bound(500)
        );
    }
}

#[cfg(test)]
#[path = "sweep/feedback_tests.rs"]
mod feedback_tests;
