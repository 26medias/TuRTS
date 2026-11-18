//! Fair query service followed by the controller's unchanged physical application order.
use super::*;
#[derive(Clone, Copy, Debug, Default)]
pub(super) enum Plan {
    #[default]
    None,
    Keep(Option<Vec2>),
    Fresh {
        peer: u32,
        point: Vec2,
    },
    Bay(Bay),
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Bay {
    pub(super) mouth: Vec2,
    pub(super) normal: Vec2,
    pub(super) radius: f32,
}
fn order(frame: &MovementFrame, state: &TrafficState, service: &mut [usize]) {
    service.sort_by(|&a, &b| {
        let age = |i: usize| {
            let m = &state.members[&frame.units[i].unit_id];
            (
                m.attempts.get(Attempt::Lazy).unwrap_or(m.accepted_tick),
                m.attempts.get(Attempt::Lazy).is_some(),
                m.accepted_tick,
            )
        };
        age(a)
            .cmp(&age(b))
            .then_with(|| physical(&frame.units[a], &frame.units[b]))
    });
}
#[allow(
    clippy::too_many_arguments,
    reason = "One query consumes only the existing explicit owner work credit"
)]
fn check(
    frame: &MovementFrame,
    index: usize,
    to: Vec2,
    inspect_terrain: bool,
    spatial: &mut Spatial,
    visits: &mut [usize; SLOTS],
    clear: &mut impl FnMut(usize, Vec2, Vec2) -> bool,
    quota: usize,
) -> Result<(bool, bool), ()> {
    let unit = &frame.units[index];
    if !to.is_finite()
        || to.x < unit.radius
        || to.y < unit.radius
        || to.x > frame.bounds.x - unit.radius
        || to.y > frame.bounds.y - unit.radius
    {
        return Ok((false, false));
    }
    let credit = visits.get_mut(usize::from(unit.owner_slot)).ok_or(())?;
    let neighbors = spatial.chord(frame, index, to, credit, quota).ok_or(())?;
    let bodies = neighbors.iter().all(|&other| {
        other == index
            || segment_distance(frame.units[other].position, unit.position, to)
                >= unit.radius + frame.units[other].radius
    });
    let terrain = (bodies || inspect_terrain) && clear(index, unit.position, to);
    Ok((bodies, terrain))
}
#[allow(
    clippy::too_many_arguments,
    reason = "Keep fair metadata service separate from physical gate order"
)]
pub(super) fn prepare(
    frame: &MovementFrame,
    proposed: &MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    service: &mut [usize],
    neighbors: &NeighborRows,
    guided: &[bool],
    waiting: &mut BTreeSet<u32>,
    spatial: &mut Spatial,
    visits: &mut [usize; SLOTS],
    clear: &mut impl FnMut(usize, Vec2, Vec2) -> bool,
    quota: usize,
    hold_optional_refusal: bool,
) {
    for member in state.members.values_mut() {
        member.lazy_plan = Plan::None;
    }
    order(frame, state, service);
    for &index in service.iter() {
        let unit = &frame.units[index];
        if waiting.contains(&unit.unit_id) || guided[index] {
            continue;
        }
        let owner = usize::from(unit.owner_slot);
        let before = visits[owner];
        let gate_key = state
            .gates
            .iter()
            .find(|g| g.admitted.contains(&unit.unit_id) && gate_local(g, unit))
            .map(|g| (g.centre, g.axis, g.direction));
        let saved = state.members[&unit.unit_id]
            .passing
            .filter(|p| gate_key == Some((p.centre, p.axis, p.direction)));
        let mut unavailable = false;
        let mut result = Plan::None;
        if let Some(saved) = saved {
            let at_connector = unit.position.distance(saved.point) <= unit.speed.unwrap_or(0.) * dt;
            if at_connector
                && proposed.units[index]
                    .target
                    .is_some_and(|p| clear(index, unit.position, p))
            {
                result = Plan::Keep(None);
                state.members.get_mut(&unit.unit_id).unwrap().passing = None;
            } else {
                match check(
                    frame,
                    index,
                    saved.point,
                    true,
                    spatial,
                    visits,
                    clear,
                    quota,
                ) {
                    Ok((_, true)) => result = Plan::Keep(Some(saved.point)),
                    Ok(_) => {}
                    Err(()) => unavailable = true,
                }
            }
        }
        if matches!(result, Plan::None) && !unavailable {
            state.members.get_mut(&unit.unit_id).unwrap().passing = None;
            let direction = heading(unit);
            'peers: for &other in &neighbors[&index] {
                let peer = &frame.units[other];
                if other == index
                    || peer.owner_slot != unit.owner_slot
                    || !moving(peer)
                    || direction.dot(heading(peer)) > -0.75
                {
                    continue;
                }
                let delta = peer.position - unit.position;
                let reach = 4. * (unit.radius + peer.radius);
                if delta.length_squared() > reach * reach || delta.dot(direction) <= 0. {
                    continue;
                }
                let axis = canonical(delta.normalize_or_zero());
                if axis == Vec2::ZERO {
                    continue;
                }
                let centre = (unit.position + peer.position) * 0.5;
                let normal = Vec2::new(-axis.y, axis.x);
                let lateral = unit.radius + peer.radius;
                for offset in [normal * lateral, -normal * lateral] {
                    let side = if direction.dot(axis) >= 0. {
                        offset
                    } else {
                        -offset
                    };
                    let point = centre + side;
                    match check(frame, index, point, false, spatial, visits, clear, quota) {
                        Ok((true, true)) => {
                            result = Plan::Fresh {
                                peer: peer.unit_id,
                                point,
                            };
                            break 'peers;
                        }
                        Ok(_) => {}
                        Err(()) => {
                            unavailable = true;
                            break 'peers;
                        }
                    }
                }
            }
        }
        let member = state.members.get_mut(&unit.unit_id).unwrap();
        member.lazy_plan = result;
        if visits[owner] > before {
            member.attempts.set(Attempt::Lazy, Some(tick));
        }
        if unavailable {
            if hold_optional_refusal {
                waiting.insert(unit.unit_id);
            }
            state.neighbor_overflows += 1;
        }
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "Service bounded parking proposals fairly after physical admissions"
)]
pub(super) fn park(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    service: &mut [usize],
    spatial: &mut Spatial,
    visits: &mut [usize; SLOTS],
    clear: &mut impl FnMut(usize, Vec2, Vec2) -> bool,
    quota: usize,
) {
    order(frame, state, service);
    for &index in service.iter() {
        let unit = &frame.units[index];
        let request =
            match std::mem::take(&mut state.members.get_mut(&unit.unit_id).unwrap().lazy_plan) {
                Plan::Bay(request) => request,
                _ => continue,
            };
        let owner = usize::from(unit.owner_slot);
        let before = visits[owner];
        for offset in [2.5f32, -2.5, 4., -4.] {
            let bay = request.mouth + request.normal * (offset * request.radius);
            if bay.distance_squared(unit.position) <= 1e-8 {
                break;
            }
            let step = (bay - unit.position).clamp_length_max(unit.speed.unwrap_or(0.) * dt);
            if step.length_squared() <= 1e-8 {
                continue;
            }
            match check(
                frame,
                index,
                unit.position + step,
                false,
                spatial,
                visits,
                clear,
                quota,
            ) {
                Ok((true, true)) => {
                    proposed.units[index].target = Some(bay);
                    proposed.units[index].speed = unit.speed;
                    break;
                }
                Ok(_) => {}
                Err(()) => {
                    state.neighbor_overflows += 1;
                    break;
                }
            }
        }
        if visits[owner] > before {
            state
                .members
                .get_mut(&unit.unit_id)
                .unwrap()
                .attempts
                .set(Attempt::Lazy, Some(tick));
        }
    }
}
