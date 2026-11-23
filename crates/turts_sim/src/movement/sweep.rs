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
/// Under fog the rows are in one fixed physical order (`arrival::prepare_with_kept`, by
/// position), and the order has no term from a goal. A body is certified against the start
/// disc of each body that has no turn yet. So a fixed order moves a dense group in one
/// world direction front first, and in the opposite direction rear first: there each body
/// gets only the gap to the body ahead, and the rear of the group does not move.
///
/// With second turns, a body does not search in its first turn when the start disc of a
/// body of its own owner that had no first turn yet stops its straight step. It keeps its
/// start disc and its unused pair allowance. It gets its second turn immediately after
/// that body has its certified chord. The decision reads two bodies of one owner and the
/// turn order only: no unit id, and no fact of another owner. A body that a body of
/// another owner stops keeps the one-turn rule, so the physical order alone decides which
/// of two bodies of two owners moves first.
pub(super) fn second_turn_under_fog(fog: bool) -> bool {
    fog
}

/// The bodies that wait for a second turn. For each row: the newest body that waits for
/// it, the next body that waits for the same row, and the wait flag. `woken` holds the
/// rows whose second turn is due. Each row waits one time at most, so no list holds more
/// than one entry for each row.
struct Waiters {
    newest: Vec<u32>,
    next: Vec<u32>,
    waiting: Vec<bool>,
    woken: Vec<u32>,
}

impl Waiters {
    const NONE: u32 = u32::MAX;
    /// The bytes of the lists for `rows` rows.
    const fn bytes(rows: usize) -> usize {
        rows * (3 * std::mem::size_of::<u32>() + 1)
    }
    fn new(rows: usize) -> Self {
        Self {
            newest: vec![Self::NONE; rows],
            next: vec![Self::NONE; rows],
            waiting: vec![false; rows],
            woken: Vec::with_capacity(rows),
        }
    }
    /// Let `body` wait for `blocker`. False when second turns are off, or when `blocker`
    /// waits itself: its first turn is done, so a wait for it can make a cycle.
    fn wait(&mut self, body: usize, blocker: usize) -> bool {
        if blocker >= self.waiting.len() || body >= self.waiting.len() || self.waiting[blocker] {
            return false;
        }
        self.waiting[body] = true;
        self.next[body] = self.newest[blocker];
        self.newest[blocker] = body as u32;
        true
    }
    /// `row` has its certified chord. Its waiters get their turns next, in the order in
    /// which they started to wait (the row order).
    fn wake(&mut self, row: usize) {
        let Some(newest) = self.newest.get_mut(row) else {
            return;
        };
        let mut body = std::mem::replace(newest, Self::NONE);
        while body != Self::NONE {
            self.waiting[body as usize] = false;
            self.woken.push(body);
            body = std::mem::replace(&mut self.next[body as usize], Self::NONE);
        }
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
        false,
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
    second_turn: bool,
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
                &mut budget.escape_keep_scans,
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
    // Second turns (see `second_turn_under_fog`). The lists are counted in the scratch
    // peak. A frame with no room for them keeps the one-turn order.
    let count = units.len();
    let second_turn = second_turn
        && budget.peak_bytes.saturating_add(Waiters::bytes(count)) <= bounded::SCRATCH_BYTES;
    if second_turn {
        budget.peak_bytes += Waiters::bytes(count);
    }
    let mut waiters = Waiters::new(if second_turn { count } else { 0 });
    let mut next_row = 0;
    loop {
        // A due second turn comes before the first turn of the next row.
        let (index, first) = if let Some(row) = waiters.woken.pop() {
            (row as usize, false)
        } else if next_row < count {
            next_row += 1;
            (next_row - 1, true)
        } else {
            break;
        };
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
            waiters.wake(index);
            continue;
        }
        // A stationary body is already protected by every earlier certificate.
        if proposed == from {
            outputs[index].position = from;
            if admitted {
                committed[index] = true;
                candidates.borrow_mut().mark_committed(index);
            }
            waiters.wake(index);
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
        // A second turn continues the pair count of the first turn.
        let used = std::cell::Cell::new(spent[index]);
        let exhausted = std::cell::Cell::new(false);
        // The last body of the same owner with no certified chord yet that stopped a
        // checked step.
        let unturned = std::cell::Cell::new(None);
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
            let turned = committed.get(other).copied().unwrap_or(false);
            let peer_to = if turned {
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
            if !clear && !turned && units[other].owner_slot == units[index].owner_slot {
                unturned.set(Some(other));
            }
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
        let straight = clear(proposed);
        // One check ran, and it stops at the first pair that is not clear.
        if !straight
            && first
            && !exhausted.get()
            && let Some(blocker) = unturned.get()
            && waiters.wait(index, blocker)
        {
            // Not committed: each other body is certified against this start disc.
            spent[index] = used.get();
            continue;
        }
        if !straight {
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
        waiters.wake(index);
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
            &mut budget.escape_keep_scans,
        );
    }
}

/// The overlap predicate and tolerance of the escape exit read
/// (`frame::overlap_read`), on the start-of-tick rows. No output of this tick and
/// no unit id enters the result. It runs only for a row that carries an escape
/// and whose proposed pose was clipped.
fn overlaps_at_start_of_tick(
    frame: &MovementFrame,
    actor: &MovementFrameUnit,
    position: Vec2,
) -> bool {
    frame.units.iter().any(|other| {
        other.unit_id != actor.unit_id
            && position.distance(other.position) < actor.radius + other.radius - 1.0e-5
    })
}
fn finish(
    frame: &MovementFrame,
    unit: &MovementFrameUnit,
    from: Vec2,
    proposed: Vec2,
    output: &mut MovementFrameOutput,
    dt: f32,
    escape_keep_scans: &mut u64,
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
        // An escape is owed only while its trigger is true. Every writer creates it
        // from a body overlap, so a clipped pose keeps the original escape only when
        // the certified pose still overlaps a body. Otherwise the discharge of the
        // frame stands, and a body that only stands in contact can be held and stop.
        // The group progress row follows the escape: when the frame discharged
        // the escape it also dropped the old blocked progress (frame.rs, the exit
        // read), and that row is not brought back here.
        if let Some(escape) = &original.physical_escape
            && {
                *escape_keep_scans += 1;
                overlaps_at_start_of_tick(frame, original, accepted)
            }
        {
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
    /// A clipped pose of a row that carries an escape, next to one resting body.
    fn clipped_escape(gap: f32, ids: [u32; 2]) -> Option<PhysicalEscape> {
        clipped_row(gap, ids).physical_escape
    }
    fn clipped_row(gap: f32, ids: [u32; 2]) -> MovementFrameOutput {
        let holder = body(ids[0], Vec2::new(10., 10.), 0.4, None);
        let at = Vec2::new(10.8 + gap, 10.);
        let proposed = at - Vec2::new(0.1, 0.);
        let mut actor = body(ids[1], at, 0.4, Some(Vec2::new(10., 10.)));
        actor.speed = Some(3.5);
        actor.physical_escape = Some(PhysicalEscape { target: None });
        // The old blocked group row of the start-of-tick body.
        actor.progress = Some(crate::movement::MoveProgress {
            pool_id: 7,
            slot: Vec2::new(10., 10.),
            activity: MovementActivity::Blocked,
            pass_mode: false,
            stalling_on_group: false,
            stalled_ticks: 9,
            best_distance: 0.8,
            claim_count: 0,
            follow_origin: None,
        });
        let frame = MovementFrame {
            recovery: Default::default(),
            units: vec![holder, actor.clone()],
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        // The frame discharged the escape and the sweep refused the proposed pose.
        let mut out = output(&actor, at);
        out.physical_escape = None;
        let mut scans = 0;
        finish(&frame, &actor, at, proposed, &mut out, 0.05, &mut scans);
        assert_eq!(
            scans, 1,
            "one counted scan for one clipped row with an escape"
        );
        assert_eq!(out.position, at);
        out
    }
    #[test]
    fn the_old_blocked_group_row_follows_the_escape() {
        // The frame drops the blocked group row when it discharges the escape
        // (frame.rs, the exit read). A clipped pose in contact does not bring the
        // row back; a clipped pose that still overlaps keeps the row with the escape.
        assert!(clipped_row(0., [1, 2]).progress.is_none());
        let kept = clipped_row(-0.1, [1, 2]);
        assert!(kept.physical_escape.is_some());
        assert_eq!(kept.progress.map(|row| row.stalled_ticks), Some(9));
    }
    #[test]
    fn a_clipped_pose_in_contact_does_not_bring_back_a_discharged_escape() {
        for ids in [[1, 2], [2, 1]] {
            // Exact contact, and an overlap inside the 1.0e-5 tolerance of the exit read.
            assert_eq!(clipped_escape(0., ids), None, "ids {ids:?}");
            assert_eq!(clipped_escape(-4.0e-6, ids), None, "ids {ids:?}");
        }
    }
    #[test]
    fn a_clipped_pose_that_still_overlaps_keeps_its_escape() {
        for ids in [[1, 2], [2, 1]] {
            let kept = clipped_escape(-0.1, ids).expect("a real overlap is still owed");
            // The refused proposal becomes the escape target, as before.
            let proposed = Vec2::new(10.8 - 0.1, 10.) - Vec2::new(0.1, 0.);
            assert_eq!(kept.target, Some(proposed), "ids {ids:?}");
        }
    }
    #[test]
    fn projected_raw_arrival_stays_within_speed_and_keeps_unfinished_order() {
        let from = Vec2::new(26.903395, 24.681269);
        let proposed = Vec2::new(27.017876, 24.547909);
        let mut unit = body(1, from, 0.3, Some(proposed));
        unit.speed = Some(3.5);
        let units = [unit.clone()];
        let frame = MovementFrame {
            recovery: Default::default(),
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
                recovery: Default::default(),
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
                recovery: Default::default(),
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
                recovery: Default::default(),
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
            recovery: Default::default(),
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
            recovery: Default::default(),
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
                recovery: Default::default(),
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
            recovery: Default::default(),
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
    /// Each body asks for its straight step, as a mover of the terrain traffic model does.
    struct Straight;
    impl MotionGuide for Straight {
        fn kind(&self, _: u32) -> MotionKind {
            MotionKind::Destination
        }
        fn next_position(&mut self, request: MotionRequest, _: &mut SweepBudget) -> Option<Vec2> {
            Some(request.preferred_position())
        }
    }
    /// Fog with no sight: a body of another owner is known by contact only.
    struct NoSight;
    impl crate::movement::MovementVisibility for NoSight {
        fn unit_permitted(&self, _: u8, _: u32) -> bool {
            false
        }
    }
    /// One tick under fog of a column of one owner: six bodies 0.1 apart, each with a
    /// point 40 along the column. `east` gives the world direction. A body of the same
    /// owner that does not move stands 2.0 ahead of the front, so the front cannot make
    /// its full step and the column is not one complete support group. With `enemy_goal`,
    /// a body of another owner 6.0 to the side of the column moves to that goal. Returns
    /// the outputs of the column from its rear to its front, and the travel of each.
    fn fog_column(east: bool, enemy_goal: Option<Vec2>) -> (Vec<MovementFrameOutput>, Vec<f32>) {
        let side = if east { 1. } else { -1. };
        let place = |rank: u32| Vec2::new(64. + side * (0.7 * rank as f32 - 20.), 20.);
        let mut units = (0..6)
            .map(|rank| {
                let mut body = unit(
                    rank + 1,
                    place(rank),
                    place(rank) + Vec2::new(side * 40., 0.),
                    100.,
                );
                body.radius = 0.3;
                body
            })
            .collect::<Vec<_>>();
        let mut wall = unit(8, place(5) + Vec2::new(side * 2.6, 0.), place(5), 100.);
        wall.radius = 0.3;
        wall.speed = None;
        wall.target = None;
        units.push(wall);
        if let Some(goal) = enemy_goal {
            let mut enemy = unit(9, Vec2::new(64., 26.), goal, 100.);
            enemy.owner_slot = 1;
            enemy.radius = 0.3;
            units.push(enemy);
        }
        units.sort_by_key(|body| body.unit_id);
        let frame = MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        };
        let outputs = crate::movement::step_frame_with_motion_guide_and_visibility(
            &frame,
            0.05,
            &BTreeSet::new(),
            &mut SweepBudget::new(1),
            &mut Straight,
            Some(&NoSight),
        );
        for (index, a) in outputs.iter().enumerate() {
            for b in &outputs[..index] {
                assert!(
                    a.position.distance(b.position) >= 0.6 - 1e-5,
                    "bodies {} and {} overlap",
                    a.unit_id,
                    b.unit_id
                );
            }
        }
        let column = (0..6)
            .map(|rank| {
                outputs
                    .iter()
                    .find(|output| output.unit_id == rank + 1)
                    .unwrap()
                    .clone()
            })
            .collect::<Vec<_>>();
        let travel = column
            .iter()
            .zip(0..)
            .map(|(output, rank)| output.position.distance(place(rank)))
            .collect();
        (column, travel)
    }
    #[test]
    fn a_fog_column_moves_equally_in_both_world_directions() {
        // The sweep order under fog is physical (by position). In a column toward +x the
        // body ahead has its turn after the body behind. The body behind must not get only
        // the gap to the start disc of the body ahead: the mirror column moves a full step.
        let (_, east) = fog_column(true, None);
        let (_, west) = fog_column(false, None);
        for (rank, (east, west)) in east.iter().zip(&west).enumerate() {
            assert!(
                (east - west).abs() < 1e-3,
                "rank {rank} from the rear: travel {east} toward +x, {west} toward -x"
            );
        }
        assert!(
            east[0] > 1.9,
            "the rear of the column toward +x did not follow the front: {east:?}"
        );
    }
    /// One tick under fog of the column of `fog_column` with bodies of another owner:
    /// `(position, goal, radius)`; a body with no goal does not move. `sight` makes them
    /// all visible. Returns the outputs of the column and of its wall.
    fn column_with_enemies(
        east: bool,
        speed: f32,
        enemies: &[(Vec2, Option<Vec2>, f32)],
        sight: bool,
    ) -> Vec<MovementFrameOutput> {
        struct Sight(bool);
        impl crate::movement::MovementVisibility for Sight {
            fn unit_permitted(&self, _: u8, _: u32) -> bool {
                self.0
            }
        }
        let side = if east { 1. } else { -1. };
        let place = |rank: u32| Vec2::new(64. + side * (0.7 * rank as f32 - 20.), 20.);
        let mut units = (0..6)
            .map(|rank| {
                let mut body = unit(
                    rank + 1,
                    place(rank),
                    place(rank) + Vec2::new(side * 40., 0.),
                    speed,
                );
                body.radius = 0.3;
                body
            })
            .collect::<Vec<_>>();
        let mut wall = unit(8, place(5) + Vec2::new(side * 2.6, 0.), place(5), 100.);
        wall.radius = 0.3;
        wall.speed = None;
        wall.target = None;
        units.push(wall);
        for (n, (at, goal, radius)) in enemies.iter().enumerate() {
            let mut enemy = unit(100 + n as u32, *at, goal.unwrap_or(*at), speed);
            enemy.owner_slot = 1;
            enemy.radius = *radius;
            if goal.is_none() {
                enemy.speed = None;
                enemy.target = None;
            }
            units.push(enemy);
        }
        units.sort_by_key(|body| body.unit_id);
        let frame = MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        };
        let outputs = crate::movement::step_frame_with_motion_guide_and_visibility(
            &frame,
            0.05,
            &BTreeSet::new(),
            &mut SweepBudget::new(1),
            &mut Straight,
            Some(&Sight(sight)),
        );
        outputs
            .into_iter()
            .filter(|output| output.unit_id < 100)
            .collect()
    }
    #[test]
    fn hidden_bodies_near_a_fog_column_change_it_only_on_its_chords() {
        // 400 random worlds: one to six hidden bodies of another owner in lanes beside the
        // column, 0.12 or more clear of each chord of the column. Some stand; some move
        // along their lane in one of the two directions. One more hidden body stands at
        // a corner of the raw step of the front body: its box meets the box of that step,
        // so it is a pair candidate of the front body, and the exact pair test finds it
        // clear (0.78 from the end of the step; the contact range is 0.701). No hidden
        // body stops a step: the column is the column with no enemy.
        let mut state = 99u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 40) as f32) / (1u64 << 24) as f32
        };
        for case in 0..400 {
            let east = case % 2 == 0;
            let speed = [100.0f32, 20.0, 5.0][case % 3];
            let side = if east { 1. } else { -1. };
            let count = 1 + (next() * 6.0) as usize;
            let front = Vec2::new(64. + side * (0.7 * 5. - 20.), 20.);
            let corner = front
                + Vec2::new(
                    side * (speed * 0.05 + 0.55),
                    if next() < 0.5 { 0.55 } else { -0.55 },
                );
            let mut enemies: Vec<(Vec2, Option<Vec2>, f32)> = vec![(corner, None, 0.3)];
            while enemies.len() <= count {
                let x = 64. + side * (next() * 14.0 - 24.0);
                let lane = 0.6 + 0.12 + next() * 1.5;
                let y = 20. + if next() < 0.5 { lane } else { -lane };
                let at = Vec2::new(x, y);
                if enemies.iter().any(|(other, _, _)| other.distance(at) < 0.9) {
                    continue;
                }
                let goal = match (next() * 3.0) as u32 {
                    0 => None,
                    1 => Some(at + Vec2::new(side * 40., 0.)),
                    _ => Some(at - Vec2::new(side * 40., 0.)),
                };
                enemies.push((at, goal, 0.3));
            }
            let alone = column_with_enemies(east, speed, &[], false);
            let dark = column_with_enemies(east, speed, &enemies, false);
            assert_eq!(
                alone, dark,
                "case {case}: east {east}, speed {speed}, enemies {enemies:?}"
            );
        }
        // Control: the test can see a hidden body. A hidden body that stands on a chord of
        // the column (0.5 from its line: closer than the two radii) changes the column.
        for case in 0..40 {
            let east = case % 2 == 0;
            let side = if east { 1. } else { -1. };
            let at = Vec2::new(64. + side * (-20. + 0.35 + 0.7 * (case % 5) as f32), 20.5);
            let alone = column_with_enemies(east, 100., &[], false);
            let dark = column_with_enemies(east, 100., &[(at, None, 0.3)], false);
            assert_ne!(
                alone, dark,
                "control {case}: a body on a chord stops the column"
            );
        }
    }
    #[test]
    fn two_bodies_that_stop_each_other_do_not_wait_for_each_other() {
        // Two bodies of one owner 0.1 apart move toward each other under fog. The first
        // in the turn order waits for the start disc of the second. The second must not
        // wait for the first: a body that waits has no certified chord, so two bodies
        // that wait for each other get no turn, and their raw steps pass through each
        // other with no certificate.
        for flip in [false, true] {
            let (left, right) = (Vec2::new(20., 20.), Vec2::new(20.7, 20.));
            let mut units = vec![
                unit(
                    if flip { 2 } else { 1 },
                    left,
                    left + Vec2::new(40., 0.),
                    100.,
                ),
                unit(
                    if flip { 1 } else { 2 },
                    right,
                    right - Vec2::new(40., 0.),
                    100.,
                ),
            ];
            for body in &mut units {
                body.radius = 0.3;
            }
            units.sort_by_key(|body| body.unit_id);
            let frame = MovementFrame {
                recovery: Default::default(),
                bounds: Vec2::splat(128.),
                tick_hz: 20,
                units,
                pools: BTreeMap::new(),
            };
            let outputs = crate::movement::step_frame_with_motion_guide_and_visibility(
                &frame,
                0.05,
                &BTreeSet::new(),
                &mut SweepBudget::new(1),
                &mut Straight,
                Some(&NoSight),
            );
            let x = |start: Vec2| {
                let unit = frame.units.iter().find(|unit| unit.position == start);
                let id = unit.unwrap().unit_id;
                outputs
                    .iter()
                    .find(|output| output.unit_id == id)
                    .unwrap()
                    .position
                    .x
            };
            assert!(
                x(left) + 0.6 - 1e-5 <= x(right),
                "flip {flip}: the bodies passed through each other: {} and {}",
                x(left),
                x(right)
            );
        }
    }
    #[test]
    fn a_second_turn_continues_the_pair_count_of_the_first_turn() {
        // The guide records the pair count of a body at each pair that it charges. A body
        // that waits and gets a second turn must continue its count: with a count from 0
        // it would get its pair allowance two times.
        struct Recording(std::cell::RefCell<Vec<(u32, usize)>>);
        impl MotionGuide for Recording {
            fn kind(&self, _: u32) -> MotionKind {
                MotionKind::Destination
            }
            fn next_position(
                &mut self,
                request: MotionRequest,
                _: &mut SweepBudget,
            ) -> Option<Vec2> {
                Some(request.preferred_position())
            }
            fn charge_certificate_pair(
                &self,
                unit_id: u32,
                owner: u8,
                spent: &mut usize,
                allowance: usize,
                budget: &mut SweepBudget,
            ) -> bool {
                self.0.borrow_mut().push((unit_id, *spent));
                budget.charge_pair(owner, spent, allowance)
            }
        }
        // The column toward +x: each body behind the front waits for the body ahead.
        let place = |rank: u32| Vec2::new(44. + 0.7 * rank as f32, 20.);
        let mut units = (0..6)
            .map(|rank| {
                let mut body = unit(
                    rank + 1,
                    place(rank),
                    place(rank) + Vec2::new(40., 0.),
                    100.,
                );
                body.radius = 0.3;
                body
            })
            .collect::<Vec<_>>();
        let mut wall = unit(8, place(5) + Vec2::new(2.6, 0.), place(5), 100.);
        wall.radius = 0.3;
        wall.speed = None;
        wall.target = None;
        units.push(wall);
        let frame = MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        };
        let mut guide = Recording(Default::default());
        let outputs = crate::movement::step_frame_with_motion_guide_and_visibility(
            &frame,
            0.05,
            &BTreeSet::new(),
            &mut SweepBudget::new(1),
            &mut guide,
            Some(&NoSight),
        );
        // The rear followed the front: it had a second turn.
        assert!(outputs[0].position.x > place(0).x + 1.9, "{:?}", outputs[0]);
        let pairs = guide.0.into_inner();
        for id in 1..=6u32 {
            let counts: Vec<usize> = pairs
                .iter()
                .filter(|(unit, _)| *unit == id)
                .map(|(_, count)| *count)
                .collect();
            assert!(!counts.is_empty(), "unit {id} charged no pair");
            assert!(
                counts.windows(2).all(|pair| pair[0] < pair[1]),
                "unit {id}: the pair count did not continue: {counts:?}"
            );
        }
    }
    #[test]
    fn a_hidden_body_out_of_contact_does_not_change_a_fog_column() {
        // Paired worlds: the hidden body of the other owner has two different goals, and
        // its motion does not come near the column. The column is equal in the two worlds.
        for east in [true, false] {
            let (near, _) = fog_column(east, Some(Vec2::new(64., 60.)));
            let (far, _) = fog_column(east, Some(Vec2::new(100., 26.)));
            assert_eq!(near, far);
        }
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
                recovery: Default::default(),
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
            recovery: Default::default(),
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
                recovery: Default::default(),
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
            recovery: Default::default(),
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
