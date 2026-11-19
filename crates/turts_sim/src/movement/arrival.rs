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
#[cfg(test)]
pub(super) fn prepare_with_guide(
    units: &mut [MovementFrameUnit],
    original: &[MovementFrameUnit],
    dt: f32,
    bounds: Vec2,
    guide: &dyn super::motion::MotionGuide,
    visibility: Option<&dyn super::MovementVisibility>,
) -> Vec<(u32, Vec2)> {
    prepare_with_kept(
        units,
        original,
        dt,
        bounds,
        guide,
        visibility,
        &mut Vec::new(),
    )
}

/// `kept` receives the ids, in increasing order, of the bodies that keep a temporary stop
/// from `settle_foreign` through this tick.
pub(super) fn prepare_with_kept(
    units: &mut [MovementFrameUnit],
    original: &[MovementFrameUnit],
    dt: f32,
    bounds: Vec2,
    guide: &dyn super::motion::MotionGuide,
    visibility: Option<&dyn super::MovementVisibility>,
    kept: &mut Vec<u32>,
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
    // Under fog: (unit, goal, stop point) of each temporary stop that this tick resets.
    let mut carried = Vec::new();
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
                if visibility.is_some()
                    && let Some(point) = unit.target
                {
                    carried.push((unit.unit_id, progress.original_point, point));
                }
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
    // group. `settle_foreign` handles the bodies of other owners from physical state.
    let fog_owner = |unit: &MovementFrameUnit| {
        if visibility.is_some() {
            u16::from(unit.owner_slot)
        } else {
            u16::MAX
        }
    };
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
            }
        }
    }
    if let Some(basis) = visibility
        && !carried.is_empty()
    {
        keep_foreign_yields(units, &carried, dt, max_radius, basis, kept);
        kept.sort_unstable();
    }
    // Clear the later approach before moving the earlier body through that space.
    units.sort_by(&order);
    resolved_ties
}

/// The most bodies of one kind near one goal that one mover examines in one tick.
const FOREIGN_CANDIDATES: usize = 16;

/// The physical facts of one body in one tick: where it was and where it is.
/// It holds no goal. A decision about a body of another owner reads only this.
struct Body {
    id: u32,
    owner: u8,
    radius: f32,
    /// Zero for a body that cannot move.
    speed: f32,
    before: Vec2,
    after: Vec2,
    /// True for a body that has no order or cannot move. Read only for the owner of the body.
    resting: bool,
}

/// One mover near its goal, with the short lists of the bodies near that goal.
struct GoalWatch {
    row: usize,
    goal: Vec2,
    /// (distance to the goal, row) of the known bodies of other owners, nearest first.
    foreign: Vec<(f32, usize)>,
    /// (distance to the goal, row) of the resting bodies of the same owner, nearest first.
    own_resting: Vec<(f32, usize)>,
}

/// True when the body can come into the goal in about one tick: the arrival window that
/// the shared rule uses for a group, measured on the body.
fn in_window(body: &Body, distance: f32, max_radius: f32, dt: f32) -> bool {
    body.speed > 0.0 && distance <= body.radius + max_radius + body.speed * dt + 0.05
}

/// Fill the short lists of each watch. Work: one pass on all bodies, with a constant number
/// of cell probes for each body. Memory: at most `2 * FOREIGN_CANDIDATES` rows for each watch.
/// A body of another owner goes into a list only when the owner of the mover can know it:
/// by sight (the basis) or by contact range now. So a hidden body cannot push a known body
/// out of a list.
fn watch_goals(
    bodies: &[Body],
    watches: &mut [GoalWatch],
    max_radius: f32,
    max_speed: f32,
    dt: f32,
    basis: &dyn super::MovementVisibility,
) {
    // The widest distance from a goal that a decision reads: a stop ring with a blocker on
    // it, or the arrival window of the fastest body.
    let reach = (4.0 * max_radius + 0.01).max(2.0 * max_radius + max_speed * dt + 0.05);
    let cell = (2.0 * reach).max(f32::EPSILON);
    let key = |value: f32| (value / cell).floor() as i64;
    let mut goals = std::collections::HashMap::<(i64, i64), Vec<usize>>::new();
    for (slot, watch) in watches.iter().enumerate() {
        goals
            .entry((key(watch.goal.x), key(watch.goal.y)))
            .or_default()
            .push(slot);
    }
    let insert = |list: &mut Vec<(f32, usize)>, distance: f32, row: usize| {
        let place = list.partition_point(|&(other, other_row)| {
            other
                .total_cmp(&distance)
                .then_with(|| bodies[other_row].after.x.total_cmp(&bodies[row].after.x))
                .then_with(|| bodies[other_row].after.y.total_cmp(&bodies[row].after.y))
                .is_le()
        });
        if place < FOREIGN_CANDIDATES {
            list.insert(place, (distance, row));
            list.truncate(FOREIGN_CANDIDATES);
        }
    };
    for (row, body) in bodies.iter().enumerate() {
        if !body.after.is_finite() {
            continue;
        }
        for x in key(body.after.x - reach)..=key(body.after.x + reach) {
            for y in key(body.after.y - reach)..=key(body.after.y + reach) {
                let Some(slots) = goals.get(&(x, y)) else {
                    continue;
                };
                for &slot in slots {
                    let watch = &mut watches[slot];
                    let actor = &bodies[watch.row];
                    let distance = body.after.distance(watch.goal);
                    if row == watch.row || distance > reach {
                        continue;
                    }
                    if body.owner == actor.owner {
                        if body.resting {
                            insert(&mut watch.own_resting, distance, row);
                        }
                    } else if basis.unit_permitted(actor.owner, body.id)
                        || actor.after.distance(body.after)
                            <= actor.radius + body.radius + super::route::CONTACT_GAP
                    {
                        insert(&mut watch.foreign, distance, row);
                    }
                }
            }
        }
    }
}

/// Keep a stop that `settle_foreign` gave in the tick before. The stop stays while a known
/// body of another owner that can move is in the arrival window of the goal. The check
/// reads positions only. `settle_foreign` decides at the end of this tick if the stop ends.
fn keep_foreign_yields(
    units: &mut [MovementFrameUnit],
    carried: &[(u32, Vec2, Vec2)],
    dt: f32,
    max_radius: f32,
    basis: &dyn super::MovementVisibility,
    kept: &mut Vec<u32>,
) {
    let rows = units
        .iter()
        .enumerate()
        .map(|(row, unit)| (unit.unit_id, row))
        .collect::<std::collections::HashMap<_, _>>();
    let mut watches = Vec::new();
    let mut points = Vec::new();
    for &(id, goal, point) in carried {
        let Some(&row) = rows.get(&id) else {
            continue;
        };
        // The shared rule of the owner gave this body a new stop in this tick. It has precedence.
        if units[row].target != Some(goal) || units[row].plain_progress.is_some() {
            continue;
        }
        watches.push(GoalWatch {
            row,
            goal,
            foreign: Vec::new(),
            own_resting: Vec::new(),
        });
        points.push(point);
    }
    if watches.is_empty() {
        return;
    }
    let bodies = units
        .iter()
        .map(|unit| Body {
            id: unit.unit_id,
            owner: unit.owner_slot,
            radius: unit.radius,
            speed: unit.speed.filter(|speed| *speed > 0.0).unwrap_or(0.0),
            before: unit.position,
            after: unit.position,
            resting: false,
        })
        .collect::<Vec<_>>();
    let max_speed = bodies.iter().map(|body| body.speed).fold(0.0_f32, f32::max);
    watch_goals(&bodies, &mut watches, max_radius, max_speed, dt, basis);
    for (watch, point) in watches.iter().zip(points) {
        if watch
            .foreign
            .iter()
            .any(|&(distance, row)| in_window(&bodies[row], distance, max_radius, dt))
        {
            stop_goal(&mut units[watch.row], watch.goal, point, true);
            kept.push(units[watch.row].unit_id);
        }
    }
}

/// The nearest free point to `from` on the ring of radius `distance` round `goal`.
#[allow(clippy::too_many_arguments)]
fn ring_point(
    goal: Vec2,
    distance: f32,
    from: Vec2,
    radius: f32,
    bounds: Vec2,
    blockers: &[(Vec2, f32)],
    taken: &[(Vec2, f32, usize)],
    own_row: usize,
) -> Option<Vec2> {
    let phase = (from - goal).to_angle();
    let mut best: Option<(f32, Vec2)> = None;
    for turn in 0..65 {
        let angle = if turn == 0 {
            phase
        } else {
            std::f32::consts::TAU * (turn - 1) as f32 / 64.0
        };
        let point = goal + Vec2::from_angle(angle) * distance;
        let inside = point.is_finite()
            && point.x >= radius
            && point.y >= radius
            && point.x <= bounds.x - radius
            && point.y <= bounds.y - radius;
        if !inside
            || !blockers
                .iter()
                .all(|(other, other_radius)| point.distance(*other) >= radius + other_radius)
            || !taken.iter().all(|(other, other_radius, row)| {
                *row == own_row || point.distance(*other) >= radius + other_radius
            })
        {
            continue;
        }
        let cost = point.distance_squared(from);
        if best.as_ref().is_none_or(|(previous, _)| cost < *previous) {
            best = Some((cost, point));
        }
    }
    best.map(|(_, point)| point)
}

/// Fog arrival against bodies of other owners. It runs after the sweep of each tick.
///
/// A mover uses its own goal, and three physical facts of each body that its owner can know
/// (by sight, or by contact range now): the radius, the speed, and the positions before and
/// after the sweep of this tick. It never reads the goal or the order of a body of another
/// owner. Contact and motion are thus facts that occurred, not a prediction.
///
/// A contender for the goal of the mover is a known body of another owner that is in the
/// arrival window of that goal and came nearer to it in this tick. A body that the sweep
/// stopped against the mover in the tick of first contact is a contender too.
///
/// - Start. The mover compares its time to the goal with the time of each contender, from
///   the positions before the sweep (the sweep order of the contact tick does not change
///   them). An earlier contender arrived first: the mover gets a temporary stop on the ring
///   round the goal. Equal times: no winner, a permanent stop halfway. A contender that was
///   in contact range before the sweep starts nothing: that contest is already decided.
/// - Keep. A temporary stop stays while there is a contender, or a known body of another
///   owner that moves in the goal. If not, the mover gets its goal back. When the contender
///   stops on the goal, the holder rule makes the stop final.
#[allow(clippy::too_many_arguments)]
pub(super) fn settle_foreign(
    before: &[MovementFrameUnit],
    outputs: &mut [super::MovementFrameOutput],
    dt: f32,
    bounds: Vec2,
    guide: &dyn super::motion::MotionGuide,
    basis: &dyn super::MovementVisibility,
    kept: &[u32],
    ties: &mut Vec<(u32, Vec2)>,
) {
    let Some(first_owner) = before.first().map(|unit| unit.owner_slot) else {
        return;
    };
    if before.iter().all(|unit| unit.owner_slot == first_owner) {
        return;
    }
    let own_ties = ties.len();
    let mut out_rows = Vec::with_capacity(before.len());
    let mut bodies = Vec::with_capacity(before.len());
    for unit in before {
        let Ok(out) = outputs.binary_search_by_key(&unit.unit_id, |output| output.unit_id) else {
            continue;
        };
        out_rows.push(out);
        bodies.push(Body {
            id: unit.unit_id,
            owner: unit.owner_slot,
            radius: unit.radius,
            speed: unit.speed.filter(|speed| *speed > 0.0).unwrap_or(0.0),
            before: unit.position,
            after: outputs[out].position,
            resting: unit.speed.is_none() || outputs[out].target.is_none(),
        });
    }
    let max_radius = bodies
        .iter()
        .map(|body| body.radius)
        .fold(0.0_f32, f32::max);
    let max_speed = bodies.iter().map(|body| body.speed).fold(0.0_f32, f32::max);
    let mut watches = Vec::new();
    // The temporary stops of each owner at each goal: (point, radius, row).
    let mut stops = std::collections::HashMap::<(u32, u32, u8), Vec<(Vec2, f32, usize)>>::new();
    for (row, body) in bodies.iter().enumerate() {
        let output = &outputs[out_rows[row]];
        let Some(target) = output.target else {
            continue;
        };
        if body.speed <= 0.0 || guide.kind(body.id) != super::motion::MotionKind::Destination {
            continue;
        }
        let yielding = output
            .plain_progress
            .as_ref()
            .filter(|progress| progress.arrival_yield);
        if let Some(progress) = yielding {
            let goal = progress.original_point;
            stops
                .entry((goal.x.to_bits(), goal.y.to_bits(), body.owner))
                .or_default()
                .push((target, body.radius, row));
        }
        // A stop from the shared rule of the owner, and a tie that it resolved in this
        // tick, stay as they are.
        if (yielding.is_some() && kept.binary_search(&body.id).is_err())
            || ties[..own_ties]
                .binary_search_by_key(&body.id, |(unit, _)| *unit)
                .is_ok()
        {
            continue;
        }
        // The goal is the point of the order, also when the closest-arrival rule already
        // gave this body another point near it.
        let goal = output
            .plain_progress
            .as_ref()
            .map_or(target, |progress| progress.original_point);
        if goal.is_finite()
            && (yielding.is_some() || in_window(body, body.after.distance(goal), max_radius, dt))
        {
            watches.push(GoalWatch {
                row,
                goal,
                foreign: Vec::new(),
                own_resting: Vec::new(),
            });
        }
    }
    if watches.is_empty() {
        return;
    }
    watch_goals(&bodies, &mut watches, max_radius, max_speed, dt, basis);
    // The order of the decisions is physical. It does not come from a row or a unit id.
    watches.sort_by(|a, b| {
        let (pa, pb) = (bodies[a.row].after, bodies[b.row].after);
        pa.x.total_cmp(&pb.x).then(pa.y.total_cmp(&pb.y))
    });
    for watch in &watches {
        let actor = &bodies[watch.row];
        let goal = watch.goal;
        let key = (goal.x.to_bits(), goal.y.to_bits(), actor.owner);
        let output = &mut outputs[out_rows[watch.row]];
        let yielding = output
            .plain_progress
            .as_ref()
            .is_some_and(|progress| progress.arrival_yield);
        let new_contact = |body: &Body| {
            actor.before.distance(body.before)
                > actor.radius + body.radius + super::route::CONTACT_GAP
                && actor.after.distance(body.after)
                    <= actor.radius + body.radius + super::route::CONTACT_GAP
        };
        // In the tick of first contact the sweep stops the second of the two bodies, or
        // turns it to the side. Its motion in that tick is thus not a measure of where it
        // was going. So each body that comes into contact with the mover in this tick is a
        // contender, if it moved at all. A body that rests is never a contender.
        let contenders = watch
            .foreign
            .iter()
            .map(|&(distance, row)| (distance, &bodies[row]))
            .filter(|(distance, body)| {
                in_window(body, *distance, max_radius, dt)
                    && (body.before.distance(goal) - distance > 1.0e-6
                        || (body.before != body.after && new_contact(body)))
            })
            .collect::<Vec<_>>();
        let blockers = watch
            .own_resting
            .iter()
            .map(|&(_, row)| &bodies[row])
            .chain(
                watch
                    .foreign
                    .iter()
                    .map(|&(_, row)| &bodies[row])
                    .filter(|body| body.before == body.after),
            )
            .map(|body| (body.after, body.radius))
            .collect::<Vec<_>>();
        // The largest contender that the mover must stay clear of.
        let mut first: Option<f32> = None;
        let mut tied: Option<&Body> = None;
        let mut tied_count = 0usize;
        if yielding {
            // A body that goes through the goal keeps the stop until it is out of the goal.
            first = watch
                .foreign
                .iter()
                .map(|&(distance, row)| (distance, &bodies[row]))
                .filter(|(distance, body)| {
                    body.before != body.after
                        && (*distance < actor.radius + body.radius - 1.0e-5
                            || (in_window(body, *distance, max_radius, dt)
                                && body.before.distance(goal) - distance > 1.0e-6))
                })
                .map(|(_, body)| body.radius)
                .max_by(f32::total_cmp);
            if first.is_none() {
                output.target = Some(goal);
                if let Some(cohort) = &mut output.progress {
                    cohort.slot = goal;
                    cohort.activity = MovementActivity::Moving;
                }
                output.plain_progress = None;
                if let Some(list) = stops.get_mut(&key) {
                    list.retain(|(_, _, row)| *row != watch.row);
                }
                continue;
            }
        } else {
            let own_time = actor.before.distance(goal) / actor.speed;
            for (_, body) in &contenders {
                if actor.before.distance(body.before)
                    <= actor.radius + body.radius + super::route::CONTACT_GAP
                {
                    continue;
                }
                let time = body.before.distance(goal) / body.speed;
                if (time - own_time).abs() <= 1e-6 {
                    tied_count += 1;
                    tied.get_or_insert(*body);
                } else if time < own_time {
                    first = Some(first.map_or(body.radius, |radius| radius.max(body.radius)));
                }
            }
        }
        if let Some(first_radius) = first {
            let distance = actor.radius + first_radius + 0.0001;
            if yielding
                && output
                    .target
                    .is_some_and(|point| point.distance(goal) >= distance - 1.0e-6)
            {
                continue;
            }
            let taken = stops.get(&key).map_or(&[][..], Vec::as_slice);
            if let Some(point) = ring_point(
                goal,
                distance,
                actor.after,
                actor.radius,
                bounds,
                &blockers,
                taken,
                watch.row,
            ) {
                let list = stops.entry(key).or_default();
                list.retain(|(_, _, row)| *row != watch.row);
                list.push((point, actor.radius, watch.row));
                stop_parts(
                    actor.after,
                    &mut output.target,
                    &mut output.progress,
                    &mut output.plain_progress,
                    goal,
                    point,
                    true,
                );
            }
        } else if let Some(other) = tied {
            let point = if tied_count == 1 {
                (actor.after - other.after)
                    .try_normalize()
                    .map(|direction| {
                        goal + direction * ((actor.radius + other.radius) * 0.5 + 0.0001)
                    })
            } else {
                // Three or more bodies in one exact tie: each keeps its own side of a ring.
                let radius = actor.radius.max(other.radius) + 0.0001;
                (actor.after - goal).try_normalize().map(|direction| {
                    goal + direction
                        * (radius / (std::f32::consts::PI / (tied_count + 1) as f32).sin())
                })
            };
            let free = |point: &Vec2| {
                point.is_finite()
                    && point.x >= actor.radius
                    && point.y >= actor.radius
                    && point.x <= bounds.x - actor.radius
                    && point.y <= bounds.y - actor.radius
                    && blockers
                        .iter()
                        .all(|(other, radius)| point.distance(*other) >= actor.radius + radius)
            };
            if let Some(point) = point.filter(free) {
                stop_parts(
                    actor.after,
                    &mut output.target,
                    &mut output.progress,
                    &mut output.plain_progress,
                    goal,
                    point,
                    false,
                );
                ties.push((actor.id, point));
            }
        }
    }
}

fn stop_goal(unit: &mut MovementFrameUnit, goal: Vec2, point: Vec2, temporary: bool) {
    stop_parts(
        unit.position,
        &mut unit.target,
        &mut unit.progress,
        &mut unit.plain_progress,
        goal,
        point,
        temporary,
    );
}

fn stop_parts(
    position: Vec2,
    target: &mut Option<Vec2>,
    progress: &mut Option<super::MoveProgress>,
    plain_progress: &mut Option<PlainMoveProgress>,
    goal: Vec2,
    point: Vec2,
    temporary: bool,
) {
    *target = Some(point);
    if let Some(cohort) = progress {
        cohort.slot = point;
        cohort.activity = MovementActivity::Moving;
        cohort.best_distance = position.distance(point);
        if !temporary {
            *plain_progress = None;
            return;
        }
    }
    *plain_progress = Some(PlainMoveProgress {
        arrival_yield: temporary,
        original_point: goal,
        slot: point,
        pool_slots: vec![],
        gather_goal: None,
        activity: MovementActivity::Moving,
        stalled_ticks: 0,
        best_distance: position.distance(point),
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
