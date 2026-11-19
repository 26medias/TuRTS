//! Physical arrivals take precedence over future destination plans.
use super::{MovementActivity, MovementFrameUnit, PlainMoveProgress};
use bevy_math::Vec2;

fn eta(unit: &MovementFrameUnit) -> f32 {
    match (unit.target, unit.speed) {
        (Some(target), Some(speed)) if speed > 0.0 => unit.position.distance(target) / speed,
        _ => 0.0,
    }
}

pub(super) fn geometry(a: &MovementFrameUnit, b: &MovementFrameUnit) -> std::cmp::Ordering {
    geometry_goals(a, b, a.target, b.target)
}
fn geometry_goals(
    a: &MovementFrameUnit,
    b: &MovementFrameUnit,
    a_goal: Option<Vec2>,
    b_goal: Option<Vec2>,
) -> std::cmp::Ordering {
    a.position
        .x
        .total_cmp(&b.position.x)
        .then(a.position.y.total_cmp(&b.position.y))
        .then(a.radius.total_cmp(&b.radius))
        .then(a.speed.unwrap_or(0.0).total_cmp(&b.speed.unwrap_or(0.0)))
        .then(
            a_goal
                .map_or(0.0, |p| p.x)
                .total_cmp(&b_goal.map_or(0.0, |p| p.x)),
        )
        .then(
            a_goal
                .map_or(0.0, |p| p.y)
                .total_cmp(&b_goal.map_or(0.0, |p| p.y)),
        )
}

/// Compare the remaining arrival time, including arrivals inside this tick. An exact
/// contested tie has no winner: stop the tied bodies around the point without overlap.
/// A plan never reserves a point ahead of a body that gets there first.
/// Private steering connectors never participate in destination ownership.
/// They retain full physical bodies and transit order uses the real order goal.
pub(super) fn prepare_with_guide(
    units: &mut [MovementFrameUnit],
    original: &[MovementFrameUnit],
    dt: f32,
    bounds: Vec2,
    guide: &dyn super::motion::MotionGuide,
    visibility: Option<&dyn super::MovementVisibility>,
) -> Vec<(u32, Vec2)> {
    let owns = |unit: &MovementFrameUnit| {
        guide.kind(unit.unit_id) == super::motion::MotionKind::Destination
    };
    let order = |a: &MovementFrameUnit, b: &MovementFrameUnit| {
        // Under fog the sweep order is physical. An order by arrival time lets the distance
        // to a private goal decide which of two touching bodies of two owners moves first.
        if visibility.is_some() {
            return geometry_goals(a, b, None, None);
        }
        let goal = |unit: &MovementFrameUnit| {
            if owns(unit) {
                unit.target
            } else {
                guide
                    .arrival_destination(unit.unit_id)
                    .filter(|p| p.is_finite())
            }
        };
        let arrival = |unit: &MovementFrameUnit, goal: Option<Vec2>| match (goal, unit.speed) {
            (Some(goal), Some(speed)) if speed > 0. => unit.position.distance(goal) / speed,
            _ => 0.,
        };
        let a_goal = goal(a);
        let b_goal = goal(b);
        arrival(a, a_goal)
            .total_cmp(&arrival(b, b_goal))
            .then_with(|| geometry_goals(a, b, a_goal, b_goal))
    };
    // Under fog a body of another owner takes part in a decision only when the owner of the
    // actor can know it: by sight or by contact. With no basis, all bodies are known.
    let known = |actor: &MovementFrameUnit, other: &MovementFrameUnit| {
        visibility.is_none_or(|basis| {
            other.owner_slot == actor.owner_slot
                || basis.unit_permitted(actor.owner_slot, other.unit_id)
                || actor.position.distance(other.position)
                    <= actor.radius + other.radius + super::route::CONTACT_GAP
        })
    };
    let mut resolved_ties = Vec::new();
    // Reuse the input rows. A holder index uses no more space than a position.
    let holders = original
        .iter()
        .enumerate()
        .filter_map(|(index, unit)| unit.target.is_none().then_some(index))
        .collect::<Vec<_>>();
    for unit in units.iter_mut() {
        if !owns(unit) {
            continue;
        }
        if let Some(progress) = unit
            .plain_progress
            .as_mut()
            .filter(|progress| progress.arrival_yield)
        {
            if holders.iter().any(|&index| {
                let holder = &original[index];
                holder.position.distance(progress.original_point) < 1e-5
                    && visibility.is_none_or(|basis| {
                        holder.owner_slot == unit.owner_slot
                            || basis.unit_permitted(unit.owner_slot, holder.unit_id)
                            || unit.position.distance(holder.position)
                                <= unit.radius + holder.radius + super::route::CONTACT_GAP
                    })
            }) {
                progress.arrival_yield = false;
                progress.activity = MovementActivity::Resting;
                progress.slot = unit.position;
                unit.target = None;
                if let Some(cohort) = &mut unit.progress {
                    cohort.slot = unit.position;
                    cohort.activity = MovementActivity::Resting;
                }
            } else {
                unit.target = Some(progress.original_point);
                if let Some(cohort) = &mut unit.progress {
                    cohort.slot = progress.original_point;
                    cohort.activity = MovementActivity::Moving;
                }
                unit.plain_progress = None;
            }
        }
    }
    units.sort_by(&order);
    let max_radius = units.iter().map(|u| u.radius).fold(0.0_f32, f32::max);
    // Under fog a destination is private to its owner. Only bodies of one owner share a
    // group. `foreign_arrivals` handles the bodies of other owners from physical state.
    let fog_owner = |unit: &MovementFrameUnit| {
        if visibility.is_some() {
            u16::from(unit.owner_slot)
        } else {
            u16::MAX
        }
    };
    let mut own_tied = Vec::new();
    let mut own_yields: Vec<OwnYield> = Vec::new();
    let mut groups = std::collections::BTreeMap::<(u32, u32, u16), Vec<usize>>::new();
    for (index, unit) in units.iter().enumerate() {
        if !owns(unit) {
            continue;
        }
        let (Some(target), Some(speed)) = (unit.target, unit.speed) else {
            continue;
        };
        if speed > 0.0
            && unit.position.distance(target) <= unit.radius + max_radius + speed * dt + 0.05
        {
            groups
                .entry((target.x.to_bits(), target.y.to_bits(), fog_owner(unit)))
                .or_default()
                .push(index);
        }
    }
    for indices in groups.values().filter(|indices| indices.len() > 1) {
        let goal = units[indices[0]].target.unwrap();
        let clear = indices
            .iter()
            .map(|index| approach_clear(units, *index, indices, goal, &known))
            .collect::<Vec<_>>();
        let horizon = indices
            .iter()
            .zip(&clear)
            .filter(|(_, clear)| **clear)
            .map(|(index, _)| eta(&units[*index]))
            .min_by(f32::total_cmp);
        let Some(horizon) = horizon else {
            continue;
        };
        let mut arrivals = indices
            .iter()
            .zip(clear)
            .map(|(index, clear)| {
                let arrival = if clear {
                    eta(&units[*index])
                } else {
                    forecast_arrival(units, *index, indices, goal, bounds, dt, horizon, &known)
                        .unwrap_or(f32::INFINITY)
                };
                (*index, arrival)
            })
            .collect::<Vec<_>>();
        arrivals.sort_by(|(a, ta), (b, tb)| {
            ta.total_cmp(tb)
                .then_with(|| geometry(&units[*a], &units[*b]))
        });
        let first = arrivals[0].1;
        let tied = arrivals
            .iter()
            .take_while(|(_, time)| (*time - first).abs() <= 1e-6)
            .map(|(index, _)| *index)
            .collect::<Vec<_>>();
        if tied.len() > 1 {
            let mut ordered = tied.clone();
            ordered.sort_by(|a, b| {
                let pa = units[*a].position - goal;
                let pb = units[*b].position - goal;
                pa.y.atan2(pa.x)
                    .total_cmp(&pb.y.atan2(pb.x))
                    .then_with(|| geometry(&units[*a], &units[*b]))
            });
            let phase = if ordered.len() == 2 {
                (units[ordered[0]].position - units[ordered[1]].position).to_angle()
            } else {
                (units[ordered[0]].position - goal).to_angle()
            };
            let radius = ordered
                .iter()
                .map(|index| units[*index].radius)
                .fold(0.0_f32, f32::max);
            let ring = if ordered.len() == 2 {
                (units[ordered[0]].radius + units[ordered[1]].radius) * 0.5 + 0.0001
            } else {
                (radius + 0.0001) / (std::f32::consts::PI / ordered.len() as f32).sin()
            };
            if let Some(destinations) =
                tie_destinations(units, &ordered, goal, phase, ring, bounds, &known)
            {
                for (index, point) in ordered.iter().zip(destinations) {
                    stop_goal(&mut units[*index], goal, point, false);
                    resolved_ties.push((units[*index].unit_id, point));
                    own_tied.push(*index);
                }
            }
        }
        let winner = arrivals[0].0;
        let mut reserved = Vec::new();
        for (index, _) in arrivals.iter().skip(tied.len()) {
            if let Some(point) = yield_goal(
                units, *index, winner, indices, goal, bounds, &reserved, &known,
            ) {
                reserved.push((point, units[*index].radius));
                stop_goal(&mut units[*index], goal, point, true);
                if visibility.is_some() {
                    own_yields.push(OwnYield {
                        goal,
                        owner: units[*index].owner_slot,
                        point,
                        radius: units[*index].radius,
                    });
                }
            }
        }
    }
    if let Some(basis) = visibility {
        foreign_arrivals(
            units,
            dt,
            bounds,
            max_radius,
            &owns,
            basis,
            &own_tied,
            &mut own_yields,
            &mut resolved_ties,
        );
    }
    // Clear the later approach before moving the earlier body through that space.
    units.sort_by(&order);
    resolved_ties
}

/// A temporary stop point that one owner already gave to one of its bodies in this tick.
struct OwnYield {
    goal: Vec2,
    owner: u8,
    point: Vec2,
    radius: f32,
}

/// The most bodies near one goal that one mover examines in one tick.
const FOREIGN_CANDIDATES: usize = 16;

/// The straight displacement that a body tries to make in this tick.
fn tick_motion(unit: &MovementFrameUnit, dt: f32) -> Vec2 {
    match (unit.target, unit.speed) {
        (Some(target), Some(speed)) if speed > 0.0 => {
            let offset = target - unit.position;
            let distance = offset.length();
            if distance <= speed * dt || distance <= 0.0 {
                offset
            } else {
                offset * (speed * dt / distance)
            }
        }
        _ => Vec2::ZERO,
    }
}

/// True when the two bodies are in contact range now, or touch during this tick on their
/// straight motions. Contact is physical knowledge for both owners. The second case keeps
/// the result free of the order in which the sweep moves the two bodies in that tick.
fn touches_this_tick(a: &MovementFrameUnit, b: &MovementFrameUnit, dt: f32) -> bool {
    let gap = a.position - b.position;
    let touching = a.radius + b.radius;
    if gap.length() <= touching + super::route::CONTACT_GAP {
        return true;
    }
    let closing = tick_motion(a, dt) - tick_motion(b, dt);
    let length = closing.length_squared();
    let fraction = if length > 0.0 {
        (-gap.dot(closing) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (gap + closing * fraction).length() <= touching + 1.0e-5
}

/// Fog arrival against bodies of other owners.
///
/// A mover uses three things only: its own goal, and the position, radius and speed of each
/// body that its owner can know. It never reads the goal of a body of another owner. A body
/// is known when the visibility basis permits it, or when it is in contact range, or when it
/// comes into contact range during this tick (`touches_this_tick`).
///
/// A known moving body that stands in the mover's goal and is nearer to that goal in time
/// arrived there first. The mover stops next to the goal for as long as that is true. When
/// the two times are equal there is no winner: the mover stops halfway. A body that does
/// not move is not a contender here; the holder rule and the closest-arrival rule own it.
///
/// Work: one pass on all rows, then at most `FOREIGN_CANDIDATES` bodies for each mover that
/// is near its goal. Memory: one small list for each such mover.
#[allow(clippy::too_many_arguments)]
fn foreign_arrivals(
    units: &mut [MovementFrameUnit],
    dt: f32,
    bounds: Vec2,
    max_radius: f32,
    owns: &dyn Fn(&MovementFrameUnit) -> bool,
    basis: &dyn super::MovementVisibility,
    own_tied: &[usize],
    own_yields: &mut Vec<OwnYield>,
    resolved_ties: &mut Vec<(u32, Vec2)>,
) {
    let Some(first_owner) = units.first().map(|unit| unit.owner_slot) else {
        return;
    };
    if units.iter().all(|unit| unit.owner_slot == first_owner) {
        return;
    }
    struct Mover {
        index: usize,
        goal: Vec2,
        reach: f32,
        // (distance to the goal, row), nearest first.
        near: Vec<(f32, usize)>,
    }
    let mut movers = Vec::new();
    for (index, unit) in units.iter().enumerate() {
        let (Some(target), Some(speed)) = (unit.target, unit.speed) else {
            continue;
        };
        if !owns(unit) || speed <= 0.0 || own_tied.contains(&index) {
            continue;
        }
        let goal = unit
            .plain_progress
            .as_ref()
            .filter(|progress| progress.arrival_yield)
            .map_or(target, |progress| progress.original_point);
        if goal.is_finite()
            && unit.position.distance(goal) <= unit.radius + max_radius + speed * dt + 0.05
        {
            movers.push(Mover {
                index,
                goal,
                reach: 2.0 * (unit.radius + max_radius) + 0.01,
                near: Vec::new(),
            });
        }
    }
    if movers.is_empty() {
        return;
    }
    // One cell is wider than two times the largest reach, so a body looks in at most four
    // cells for the goals that it can be near.
    let cell = (2.0 * (4.0 * max_radius + 0.01)).max(f32::EPSILON);
    let key = |value: f32| (value / cell).floor() as i64;
    let mut goals = std::collections::HashMap::<(i64, i64), Vec<usize>>::new();
    for (slot, mover) in movers.iter().enumerate() {
        goals
            .entry((key(mover.goal.x), key(mover.goal.y)))
            .or_default()
            .push(slot);
    }
    let reach = cell * 0.5;
    for (row, body) in units.iter().enumerate() {
        if !body.position.is_finite() {
            continue;
        }
        for x in key(body.position.x - reach)..=key(body.position.x + reach) {
            for y in key(body.position.y - reach)..=key(body.position.y + reach) {
                let Some(slots) = goals.get(&(x, y)) else {
                    continue;
                };
                for &slot in slots {
                    let mover = &movers[slot];
                    let actor = &units[mover.index];
                    let distance = body.position.distance(mover.goal);
                    if row == mover.index || distance > mover.reach {
                        continue;
                    }
                    // A hidden body must not push a known body out of the short list.
                    if body.owner_slot != actor.owner_slot
                        && !basis.unit_permitted(actor.owner_slot, body.unit_id)
                        && !touches_this_tick(actor, body, dt)
                    {
                        continue;
                    }
                    let near = &mut movers[slot].near;
                    let place = near.partition_point(|&(other, other_row)| {
                        other
                            .total_cmp(&distance)
                            .then_with(|| geometry_goals(&units[other_row], body, None, None))
                            .is_le()
                    });
                    if place < FOREIGN_CANDIDATES {
                        near.insert(place, (distance, row));
                        near.truncate(FOREIGN_CANDIDATES);
                    }
                }
            }
        }
    }
    for mover in movers {
        let actor = units[mover.index].clone();
        let goal = mover.goal;
        let speed = actor.speed.unwrap_or(0.0);
        let own_time = actor.position.distance(goal) / speed;
        // The largest body that arrived first, and the nearest body in an exact tie.
        let mut first: Option<f32> = None;
        let mut tied: Option<usize> = None;
        let mut tied_count = 0usize;
        for &(distance, row) in &mover.near {
            let body = &units[row];
            let (Some(body_speed), Some(_)) = (body.speed, body.target) else {
                continue;
            };
            if body.owner_slot == actor.owner_slot
                || body_speed <= 0.0
                || distance >= actor.radius + body.radius - 1.0e-5
            {
                continue;
            }
            let time = distance / body_speed;
            if (time - own_time).abs() <= 1e-6 {
                // A body that moves away from the goal in this tick does not contest it.
                if tick_motion(body, dt).dot(goal - body.position) > 0.0 {
                    tied_count += 1;
                    tied.get_or_insert(row);
                }
            } else if time < own_time {
                first = Some(first.map_or(body.radius, |radius| radius.max(body.radius)));
            }
        }
        let blockers = mover
            .near
            .iter()
            .map(|&(_, row)| &units[row])
            .filter(|body| body.speed.is_none() || body.target.is_none())
            .map(|body| (body.position, body.radius))
            .collect::<Vec<_>>();
        let inside = |point: Vec2| {
            point.is_finite()
                && point.x >= actor.radius
                && point.y >= actor.radius
                && point.x <= bounds.x - actor.radius
                && point.y <= bounds.y - actor.radius
        };
        let clear = |point: Vec2| {
            blockers
                .iter()
                .all(|(other, radius)| point.distance(*other) >= actor.radius + radius)
        };
        if let Some(first_radius) = first {
            let distance = actor.radius + first_radius + 0.0001;
            let yielding = actor
                .plain_progress
                .as_ref()
                .is_some_and(|progress| progress.arrival_yield);
            if yielding
                && actor
                    .target
                    .is_some_and(|point| point.distance(goal) >= distance)
            {
                continue;
            }
            let phase = (actor.position - goal).to_angle();
            let mut best: Option<(f32, Vec2)> = None;
            for turn in 0..65 {
                let angle = if turn == 0 {
                    phase
                } else {
                    std::f32::consts::TAU * (turn - 1) as f32 / 64.0
                };
                let point = goal + Vec2::from_angle(angle) * distance;
                if !inside(point) || !clear(point) {
                    continue;
                }
                if !own_yields.iter().all(|other| {
                    other.owner != actor.owner_slot
                        || other.goal != goal
                        || Some(other.point) == actor.target
                        || point.distance(other.point) >= actor.radius + other.radius
                }) {
                    continue;
                }
                let cost = point.distance_squared(actor.position);
                if best.as_ref().is_none_or(|(previous, _)| cost < *previous) {
                    best = Some((cost, point));
                }
            }
            if let Some((_, point)) = best {
                if yielding {
                    own_yields.retain(|other| {
                        other.owner != actor.owner_slot
                            || other.goal != goal
                            || Some(other.point) != actor.target
                    });
                }
                own_yields.push(OwnYield {
                    goal,
                    owner: actor.owner_slot,
                    point,
                    radius: actor.radius,
                });
                stop_goal(&mut units[mover.index], goal, point, true);
            }
        } else if let Some(row) = tied
            && actor.target == Some(goal)
        {
            let other = &units[row];
            let point = if tied_count == 1 {
                (actor.position - other.position)
                    .try_normalize()
                    .map(|direction| {
                        goal + direction * ((actor.radius + other.radius) * 0.5 + 0.0001)
                    })
            } else {
                // Three or more bodies in one exact tie: each keeps its own side of a ring.
                let radius = actor.radius.max(other.radius) + 0.0001;
                (actor.position - goal).try_normalize().map(|direction| {
                    goal + direction
                        * (radius / (std::f32::consts::PI / (tied_count + 1) as f32).sin())
                })
            };
            if let Some(point) = point.filter(|point| inside(*point) && clear(*point)) {
                stop_goal(&mut units[mover.index], goal, point, false);
                resolved_ties.push((actor.unit_id, point));
            }
        }
    }
}

fn stop_goal(unit: &mut MovementFrameUnit, goal: Vec2, point: Vec2, temporary: bool) {
    unit.target = Some(point);
    if let Some(cohort) = &mut unit.progress {
        cohort.slot = point;
        cohort.activity = MovementActivity::Moving;
        cohort.best_distance = unit.position.distance(point);
        if !temporary {
            unit.plain_progress = None;
            return;
        }
    }
    unit.plain_progress = Some(PlainMoveProgress {
        arrival_yield: temporary,
        original_point: goal,
        slot: point,
        pool_slots: vec![],
        gather_goal: None,
        activity: MovementActivity::Moving,
        stalled_ticks: 0,
        best_distance: unit.position.distance(point),
        claim_count: 1,
        failure_recorded: false,
    });
}

/// `known(actor, other)`: the owner of `actor` can know the body `other`.
type Known<'a> = dyn Fn(&MovementFrameUnit, &MovementFrameUnit) -> bool + 'a;

fn approach_clear(
    units: &[MovementFrameUnit],
    index: usize,
    contenders: &[usize],
    goal: Vec2,
    known: &Known,
) -> bool {
    let actor = &units[index];
    let segment = goal - actor.position;
    units
        .iter()
        .enumerate()
        .filter(|(other, unit)| !contenders.contains(other) && known(actor, unit))
        .all(|(_, other)| {
            let fraction = if segment.length_squared() > 0.0 {
                ((other.position - actor.position).dot(segment) / segment.length_squared())
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            (actor.position + segment * fraction).distance(other.position)
                >= actor.radius + other.radius
        })
}

/// A short obstructed approach uses the same movement kernel with the competing bodies
/// absent. Stop as soon as it cannot beat the clear route. No recursive contested group exists.
#[allow(clippy::too_many_arguments)]
fn forecast_arrival(
    units: &[MovementFrameUnit],
    index: usize,
    contenders: &[usize],
    goal: Vec2,
    bounds: Vec2,
    dt: f32,
    horizon: f32,
    known: &Known,
) -> Option<f32> {
    if dt <= 0.0 || horizon > 2.0 {
        return None;
    }
    let mut frame = super::MovementFrame {
        bounds,
        tick_hz: (1.0 / dt).round().max(1.0) as u16,
        pools: Default::default(),
        units: units
            .iter()
            .enumerate()
            .filter(|(other, unit)| {
                *other == index || (!contenders.contains(other) && known(&units[index], unit))
            })
            .map(|(other, unit)| {
                let mut unit = unit.clone();
                unit.progress = None;
                unit.plain_progress = None;
                unit.physical_escape = None;
                if other != index {
                    unit.speed = None;
                    unit.target = None;
                } else {
                    unit.target = Some(goal);
                }
                unit
            })
            .collect(),
    };
    let id = units[index].unit_id;
    for tick in 0..(horizon / dt).ceil() as usize + 1 {
        let before = frame.units.iter().find(|unit| unit.unit_id == id)?.clone();
        let outputs = super::step_frame(&frame, dt);
        for output in outputs {
            let unit = frame
                .units
                .iter_mut()
                .find(|unit| unit.unit_id == output.unit_id)?;
            unit.position = output.position;
            unit.target = output.target;
            unit.progress = output.progress;
            unit.plain_progress = output.plain_progress;
            unit.physical_escape = output.physical_escape;
        }
        if frame
            .units
            .iter()
            .find(|unit| unit.unit_id == id)?
            .position
            .distance(goal)
            < 1e-5
        {
            return Some(tick as f32 * dt + before.position.distance(goal) / before.speed?);
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn yield_goal(
    units: &[MovementFrameUnit],
    index: usize,
    winner: usize,
    contenders: &[usize],
    goal: Vec2,
    bounds: Vec2,
    reserved: &[(Vec2, f32)],
    known: &Known,
) -> Option<Vec2> {
    let actor = &units[index];
    let distance = actor.radius + units[winner].radius + 0.0001;
    let phase = (actor.position - goal).to_angle();
    let mut best: Option<(f32, Vec2)> = None;
    for turn in 0..65 {
        let angle = if turn == 0 {
            phase
        } else {
            std::f32::consts::TAU * (turn - 1) as f32 / 64.0
        };
        let point = goal + Vec2::from_angle(angle) * distance;
        if point.x < actor.radius
            || point.y < actor.radius
            || point.x > bounds.x - actor.radius
            || point.y > bounds.y - actor.radius
        {
            continue;
        }
        if !reserved
            .iter()
            .all(|(other, radius)| point.distance(*other) >= actor.radius + radius)
        {
            continue;
        }
        if !units
            .iter()
            .enumerate()
            .filter(|(other, unit)| {
                !contenders.contains(other)
                    && (unit.speed.is_none() || unit.target.is_none())
                    && known(actor, unit)
            })
            .all(|(_, other)| point.distance(other.position) >= actor.radius + other.radius)
        {
            continue;
        }
        let cost = point.distance_squared(actor.position);
        if best.as_ref().is_none_or(|(previous, _)| cost < *previous) {
            best = Some((cost, point));
        }
    }
    best.map(|(_, point)| point)
}

/// Keep the symmetric stop clear of the map edge and existing bodies. Choose the nearest
/// legal shared centre first. Only geometry ranks the possible axes and centre shifts.
fn tie_destinations(
    units: &[MovementFrameUnit],
    ordered: &[usize],
    goal: Vec2,
    phase: f32,
    ring: f32,
    bounds: Vec2,
    known: &Known,
) -> Option<Vec<Vec2>> {
    let blockers = units
        .iter()
        .enumerate()
        .filter(|(index, unit)| {
            !ordered.contains(index)
                && (unit.speed.is_none() || unit.target.is_none())
                && ordered.iter().any(|tied| known(&units[*tied], unit))
                && unit.position.distance(goal) <= ring * 5.0 + unit.radius * 2.0
        })
        .map(|(_, unit)| unit)
        .collect::<Vec<_>>();
    let mut best: Option<(f32, f32, Vec<Vec2>)> = None;
    for turn in 0..65 {
        let angle = if turn == 0 {
            phase
        } else {
            std::f32::consts::TAU * (turn - 1) as f32 / 64.0
        };
        let offsets = (0..ordered.len())
            .map(|rank| {
                Vec2::from_angle(angle + std::f32::consts::TAU * rank as f32 / ordered.len() as f32)
                    * ring
            })
            .collect::<Vec<_>>();
        let mut lower = Vec2::splat(f32::NEG_INFINITY);
        let mut upper = Vec2::splat(f32::INFINITY);
        for (index, offset) in ordered.iter().zip(&offsets) {
            let radius = Vec2::splat(units[*index].radius);
            lower = lower.max(radius - *offset);
            upper = upper.min(bounds - radius - *offset);
        }
        if lower.x > upper.x || lower.y > upper.y {
            continue;
        }
        // Most ties need only the goal. Extra centres cover an edge, corner, or nearby wall.
        for shift in 0..17 {
            if shift > 0 && blockers.is_empty() {
                continue;
            }
            let candidate = if shift == 0 {
                goal
            } else {
                let direction =
                    Vec2::from_angle(std::f32::consts::TAU * ((shift - 1) % 8) as f32 / 8.0);
                goal + direction * ring * (1 + (shift - 1) / 8) as f32
            };
            let centre = candidate.clamp(lower, upper);
            let points = offsets
                .iter()
                .map(|offset| centre + *offset)
                .collect::<Vec<_>>();
            let legal = ordered.iter().zip(&points).all(|(index, point)| {
                blockers.iter().all(|blocker| {
                    point.distance(blocker.position)
                        >= units[*index].radius + blocker.radius + 0.00001
                })
            });
            if !legal {
                continue;
            }
            let cost = points
                .iter()
                .map(|point| point.distance_squared(goal))
                .sum::<f32>();
            // Equal-distance slots must not send tied bodies through one another.
            // Prefer the assignment with less travel, using geometry alone.
            let travel = ordered
                .iter()
                .zip(&points)
                .map(|(index, point)| units[*index].position.distance_squared(*point))
                .sum::<f32>();
            if best.as_ref().is_none_or(|(previous, previous_travel, _)| {
                cost < *previous - 1e-5
                    || ((cost - *previous).abs() <= 1e-5 && travel < *previous_travel - 1e-5)
            }) {
                best = Some((cost, travel, points));
            }
        }
    }
    best.map(|(_, _, points)| points)
}

/// Collision sums and projections use spatial order. The grid's general id ordering is
/// useful for stable records, but it cannot give a movement body priority.
pub(super) trait SpatialNeighbors {
    fn nearby(&self, points: &[crate::grid::GridPoint], position: Vec2, reach: f32) -> Vec<usize>;
}
impl SpatialNeighbors for crate::grid::UniformGrid {
    fn nearby(&self, points: &[crate::grid::GridPoint], position: Vec2, reach: f32) -> Vec<usize> {
        // The grid caches exact float total-order keys and refreshes them on
        // every point move. The result keeps the same(x,y,index) order.
        self.within_spatial(points, position, reach)
    }
}

pub(super) fn separation_direction(unit: &MovementFrameUnit, other: &MovementFrameUnit) -> Vec2 {
    (unit.position - other.position)
        .try_normalize()
        .or_else(|| {
            (unit.target.unwrap_or(unit.position) - other.target.unwrap_or(other.position))
                .try_normalize()
        })
        .unwrap_or(Vec2::X)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::{MovementFrame, step_frame};
    fn unit(id: u32, position: Vec2, speed: f32, target: Vec2) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(speed),
            target: Some(target),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn run(frame: MovementFrame) -> MovementFrame {
        run_steps(frame, 600)
    }
    fn run_steps(mut frame: MovementFrame, n: usize) -> MovementFrame {
        for _ in 0..n {
            let outputs = step_frame(&frame, 0.05);
            for output in outputs {
                let unit = frame
                    .units
                    .iter_mut()
                    .find(|u| u.unit_id == output.unit_id)
                    .unwrap();
                unit.position = output.position;
                unit.target = output.target;
                unit.progress = output.progress;
                unit.plain_progress = output.plain_progress;
                unit.physical_escape = output.physical_escape;
            }
        }
        frame
    }
    #[test]
    fn faster_later_id_arrives_first_and_keeps_the_point() {
        let goal = Vec2::new(30.0, 20.0);
        for ids in [[1, 9], [9, 1]] {
            let frame = run(MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![
                    unit(ids[0], Vec2::new(10.0, 20.0), 1.0, goal),
                    unit(ids[1], Vec2::new(30.0, 10.0), 5.0, goal),
                ],
            });
            assert!(
                frame.units[1].position.distance(goal) < 0.001,
                "fast unit lost point: {:?}",
                frame.units
            );
            assert!(frame.units[0].position.distance(frame.units[1].position) >= 1.0 - 1e-4);
        }
    }
    #[test]
    fn simultaneous_arrivals_stop_symmetrically_without_an_id_winner() {
        let goal = Vec2::new(20.0, 20.0);
        let mut previous = None;
        for ids in [[1, 9], [9, 1]] {
            let frame = run(MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![
                    unit(ids[0], Vec2::new(18.0, 20.0), 3.0, goal),
                    unit(ids[1], Vec2::new(22.0, 20.0), 3.0, goal),
                ],
            });
            let positions = frame.units.iter().map(|u| u.position).collect::<Vec<_>>();
            assert!(positions[0].distance(positions[1]) >= 1.0 - 1e-4);
            assert!(positions[0].distance(goal) >= 0.499 && positions[1].distance(goal) >= 0.499);
            assert!(
                (positions[0] + positions[1] - 2.0 * goal).length() < 0.001,
                "asymmetric: {positions:?}"
            );
            if let Some(previous) = previous {
                assert_eq!(positions, previous);
            }
            previous = Some(positions);
        }
    }

    #[test]
    fn arrivals_inside_one_tick_use_the_fractional_travel_time() {
        let goal = Vec2::new(20.0, 20.0);
        for ids in [[1, 9], [9, 1]] {
            let mut fast = unit(ids[0], Vec2::new(19.92, 20.0), 4.0, goal);
            fast.radius = 0.05;
            let mut slow = unit(ids[1], Vec2::new(20.12, 20.0), 4.0, goal);
            slow.radius = 0.05;
            let frame = MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![fast, slow],
            };
            let output = step_frame(&frame, 0.05);
            let fast = output.iter().find(|u| u.unit_id == ids[0]).unwrap();
            let slow = output.iter().find(|u| u.unit_id == ids[1]).unwrap();
            assert_eq!(fast.position, goal);
            assert!(
                slow.position.distance(goal) >= 0.1 - 1e-5,
                "later arrival overlapped first arrival: {output:?}"
            );
        }
    }

    #[test]
    fn an_arrived_unit_keeps_its_point_when_a_later_order_reaches_it() {
        let goal = Vec2::new(20.0, 20.0);
        for ids in [[1, 9], [9, 1]] {
            let mut first = unit(ids[0], goal, 3.0, goal);
            first.target = None;
            let frame = run(MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![first, unit(ids[1], Vec2::new(10.0, 20.0), 4.0, goal)],
            });
            assert_eq!(frame.units[0].position, goal);
            assert!(frame.units[1].position.distance(goal) >= 1.0 - 1e-5);
            assert!(
                frame.units[1].target.is_none(),
                "the later unit never stopped"
            );
        }
    }
    #[test]
    fn boundary_tie_finishes_without_overlap_or_blocked_order() {
        let goal = Vec2::new(0.5, 10.0);
        let frame = run_steps(
            MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![
                    unit(1, Vec2::new(0.5, 12.0), 3.0, goal),
                    unit(2, Vec2::new(2.5, 10.0), 3.0, goal),
                ],
            },
            600,
        );
        println!("{:?}", frame.units);
        assert!(frame.units.iter().all(|u| u.target.is_none()));
        assert!(frame.units.iter().all(|u| {
            u.plain_progress
                .as_ref()
                .is_none_or(|p| p.activity != crate::movement::MovementActivity::Blocked)
        }));
        assert!(frame.units[0].position.distance(frame.units[1].position) >= 1.0 - 1e-4);
    }
    #[test]
    fn tied_late_arrivals_do_not_choose_slots_inside_the_existing_holder() {
        let goal = Vec2::new(20.0, 20.0);
        let mut holder = unit(3, goal, 3.0, goal);
        holder.target = None;
        let frame = run_steps(
            MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![
                    unit(1, Vec2::new(18.0, 20.0), 3.0, goal),
                    unit(2, Vec2::new(22.0, 20.0), 3.0, goal),
                    holder,
                ],
            },
            600,
        );
        println!("holder {:?}", frame.units);
        assert_eq!(frame.units[2].position, goal);
        assert!(frame.units[0].position.distance(frame.units[2].position) >= 1.0 - 1e-4);
        assert!(frame.units[1].position.distance(frame.units[2].position) >= 1.0 - 1e-4);
        assert!(frame.units.iter().all(|u| {
            u.plain_progress
                .as_ref()
                .is_none_or(|p| p.activity != crate::movement::MovementActivity::Blocked)
        }));
    }
    #[test]
    fn boundary_tie_must_not_give_one_unit_the_centre() {
        let goal = Vec2::new(0.5, 10.0);
        let frame = run_steps(
            MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![
                    unit(1, Vec2::new(0.6, 10.0), 3.0, goal),
                    unit(2, Vec2::new(2.6, 10.0), 63.0, goal),
                ],
            },
            600,
        );
        println!("boundary winner {:?}", frame.units);
        assert!(
            frame.units.iter().all(|u| u.position.distance(goal) > 0.49),
            "an exact tie gave one unit the centre"
        );
    }
    #[test]
    fn tie_slots_do_not_enter_a_static_building() {
        let goal = Vec2::new(20.0, 20.0);
        let mut obstacle = unit(3, Vec2::new(20.0, 18.4), 0.0, goal);
        obstacle.speed = None;
        obstacle.target = None;
        obstacle.radius = 1.0;
        let frame = run_steps(
            MovementFrame {
                bounds: Vec2::splat(64.0),
                tick_hz: 20,
                pools: Default::default(),
                units: vec![
                    unit(1, Vec2::new(18.0, 20.0), 3.0, goal),
                    unit(2, Vec2::new(20.0, 22.0), 3.0, goal),
                    obstacle,
                ],
            },
            600,
        );
        println!("static {:?}", frame.units);
        assert!(frame.units.iter().all(|u| {
            u.plain_progress
                .as_ref()
                .is_none_or(|p| p.activity != crate::movement::MovementActivity::Blocked)
        }));
    }
    fn path_frame(subjects: Vec<MovementFrameUnit>) -> MovementFrame {
        let goal = Vec2::new(20.0, 20.0);
        let mut obstacle = unit(3, Vec2::new(19.0, 20.99), 0.0, goal);
        obstacle.speed = None;
        obstacle.target = None;
        MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            pools: Default::default(),
            units: [subjects, vec![obstacle]].concat(),
        }
    }
    fn reached_at(mut frame: MovementFrame, id: u32, goal: Vec2) -> Option<f32> {
        for i in 0..200 {
            let old = frame
                .units
                .iter()
                .find(|u| u.unit_id == id)
                .unwrap()
                .clone();
            let outputs = step_frame(&frame, 0.05);
            for output in outputs {
                let u = frame
                    .units
                    .iter_mut()
                    .find(|u| u.unit_id == output.unit_id)
                    .unwrap();
                u.position = output.position;
                u.target = output.target;
                u.progress = output.progress;
                u.plain_progress = output.plain_progress;
                u.physical_escape = output.physical_escape;
            }
            if frame
                .units
                .iter()
                .find(|u| u.unit_id == id)
                .unwrap()
                .position
                .distance(goal)
                < 1e-5
            {
                return Some(i as f32 * 0.05 + old.position.distance(goal) / old.speed.unwrap());
            }
        }
        None
    }
    #[test]
    fn equal_straight_line_estimates_are_not_equal_actual_arrivals() {
        let goal = Vec2::new(20.0, 20.0);
        let left = unit(1, Vec2::new(18.85, 20.0), 3.0, goal);
        let right = unit(2, Vec2::new(21.15, 20.0), 3.0, goal);
        let left_tick = reached_at(path_frame(vec![left.clone()]), 1, goal);
        let right_tick = reached_at(path_frame(vec![right.clone()]), 2, goal);
        println!("isolated route arrivals left={left_tick:?} right={right_tick:?}");
        let frame = run_steps(path_frame(vec![left, right]), 600);
        println!("unequal paths {:?}", frame.units);
        assert_eq!(
            frame.units[1].position, goal,
            "the unobstructed unit should arrive first and keep the point"
        );
    }
    #[test]
    fn a_small_arrival_advantage_keeps_the_point() {
        let goal = Vec2::new(20.0, 20.0);
        for ids in [[1, 2], [2, 1]] {
            let frame = run_steps(
                MovementFrame {
                    bounds: Vec2::splat(64.0),
                    tick_hz: 20,
                    pools: Default::default(),
                    units: vec![
                        unit(ids[0], Vec2::new(18.0, 20.0), 3.1, goal),
                        unit(ids[1], Vec2::new(22.0, 20.0), 3.0, goal),
                    ],
                },
                600,
            );
            println!("small advantage {:?}", frame.units);
            assert_eq!(
                frame.units[0].position, goal,
                "the earlier arrival lost the point"
            );
        }
    }

    #[test]
    fn a_temporary_yield_does_not_reserve_the_point_after_the_earlier_unit_leaves() {
        let goal = Vec2::new(20.0, 20.0);
        let mut frame = MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            pools: Default::default(),
            units: vec![
                unit(1, Vec2::new(18.0, 20.0), 3.1, goal),
                unit(2, Vec2::new(22.0, 20.0), 3.0, goal),
            ],
        };
        for _ in 0..20 {
            frame = run_steps(frame, 1);
            if frame.units[1]
                .plain_progress
                .as_ref()
                .is_some_and(|progress| progress.arrival_yield)
            {
                break;
            }
        }
        assert!(
            frame.units[1]
                .plain_progress
                .as_ref()
                .is_some_and(|progress| progress.arrival_yield)
        );
        assert_ne!(frame.units[0].position, goal);
        frame.units.remove(0);
        frame = run_steps(frame, 200);
        assert_eq!(
            frame.units[0].position, goal,
            "an order kept a point that no body reached"
        );
    }
    #[test]
    fn later_plain_mover_stops_at_closest_feasible_point_beside_stationary_holder() {
        fn run_case(ids: [u32; 2], reverse: bool, mobile_holder: bool) -> Vec<Vec2> {
            let goal = Vec2::new(31.000_002, 15.999_989);
            let holder = Vec2::new(32.0, 16.0);
            let mover_radius = 0.85_f32;
            let holder_radius = 0.35_f32;
            let mut mover = unit(ids[0], Vec2::new(18.0, 20.0), 3.0, goal);
            mover.radius = mover_radius;
            let mut settled = unit(ids[1], holder, 0.0, holder);
            settled.owner_slot = 1;
            settled.radius = holder_radius;
            settled.speed = mobile_holder.then_some(3.0);
            settled.target = None;
            let mut units = vec![mover, settled];
            if reverse {
                units.reverse();
            }
            let mut frame = MovementFrame {
                bounds: Vec2::new(48.0, 32.0),
                tick_hz: 20,
                pools: Default::default(),
                units,
            };
            let minimum = (f64::from(mover_radius) + f64::from(holder_radius)
                - (f64::from(goal.x) - f64::from(holder.x))
                    .hypot(f64::from(goal.y) - f64::from(holder.y)))
            .max(0.0);
            let mut trace = Vec::new();
            let mut completed = None;
            for tick in 0..600 {
                for output in step_frame(&frame, 0.05) {
                    let actor = frame
                        .units
                        .iter_mut()
                        .find(|actor| actor.unit_id == output.unit_id)
                        .unwrap();
                    actor.position = output.position;
                    actor.target = output.target;
                    actor.progress = output.progress;
                    actor.plain_progress = output.plain_progress;
                    actor.physical_escape = output.physical_escape;
                }
                let moving = frame
                    .units
                    .iter()
                    .find(|actor| actor.unit_id == ids[0])
                    .unwrap();
                let settled = frame
                    .units
                    .iter()
                    .find(|actor| actor.unit_id == ids[1])
                    .unwrap();
                assert_eq!(
                    settled.position, holder,
                    "first holder moved at tick {tick}"
                );
                assert!(
                    moving.position.distance(holder) >= mover_radius + holder_radius - 1.0e-5,
                    "overlap at tick {tick}: {:?}",
                    moving.position
                );
                trace.push(moving.position);
                if moving.target.is_none() {
                    completed = Some(tick);
                    break;
                }
            }
            let moving = frame
                .units
                .iter()
                .find(|actor| actor.unit_id == ids[0])
                .unwrap();
            assert!(
                completed.is_some(),
                "real frame never completed: {moving:?}"
            );
            let actual = f64::from(moving.position.distance(goal));
            eprintln!(
                "closest arrival ids={ids:?} reverse={reverse} tick={completed:?} position={:?} distance={actual} analytic_minimum={minimum}",
                moving.position
            );
            assert!(
                actual <= minimum + 0.002,
                "closest feasible arrival requires {minimum}, got {actual} at {:?}",
                moving.position
            );
            trace
        }
        for mobile_holder in [false, true] {
            let baseline = run_case([1, 99], false, mobile_holder);
            for (ids, reverse) in [([99, 1], false), ([1, 99], true), ([99, 1], true)] {
                assert_eq!(
                    baseline,
                    run_case(ids, reverse, mobile_holder),
                    "opaque handles/enumeration changed physical arrival"
                );
            }
        }
    }
}
