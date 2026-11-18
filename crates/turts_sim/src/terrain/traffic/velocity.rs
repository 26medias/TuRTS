//! Frozen permitted velocity advice and accepted-motion control history.
mod joint;
mod prediction;
use super::velocity_solver::Agent;
use super::*;
use crate::movement::MovementFrameOutput;
use crate::movement::motion::{MotionGuide, MotionKind, MotionRequest};
use crate::movement::sweep::SweepBudget;
use bevy_math::DVec2;
const TICKET: usize = 128;
#[derive(Clone, Copy, Debug)]
pub(crate) struct HistoryRow {
    unit: u32,
    from: Vec2,
    pose: Vec2,
    tick: u32,
    dt: f32,
    attempt: u32,
    predictive_attempt: u32,
    seen: u32,
}
const _: () = assert!(std::mem::size_of::<HistoryRow>() == 40);
fn bits(p: Vec2) -> [u32; 2] {
    [p.x.to_bits(), p.y.to_bits()]
}
// Only a fully bound, last-tick accepted pose supplies velocity. A missing or
// stale row retains ordinary requested-chord prediction. Zero motion remains a
// soft moving witness; it never acquires a settled holder's hard authority.
fn accepted_velocity(
    history: &[HistoryRow],
    unit: &MovementFrameUnit,
    tick: u32,
    dt: f32,
    profile: Option<[u32; 3]>,
) -> Option<DVec2> {
    let i = history
        .binary_search_by_key(&unit.unit_id, |r| r.unit)
        .ok()?;
    let row = history[i];
    (row.tick.checked_add(1) == Some(tick)
        && bits(row.pose) == bits(unit.position)
        && profile
            == unit
                .speed
                .map(|speed| [unit.radius.to_bits(), unit.unit_type, speed.to_bits()])
        && row.dt.to_bits() == dt.to_bits()
        && dt.is_finite()
        && dt > 0.)
        .then(|| (row.pose.as_dvec2() - row.from.as_dvec2()) / f64::from(dt))
        .filter(|v| v.is_finite())
}
#[cfg(any(test, feature = "test-probes"))]
fn measured(history: &[HistoryRow], unit: &MovementFrameUnit, tick: u32, dt: f32) -> DVec2 {
    if !moving(unit)
        || unit
            .progress
            .as_ref()
            .is_some_and(|p| p.activity == crate::movement::MovementActivity::Resting)
        || unit
            .plain_progress
            .as_ref()
            .is_some_and(|p| p.activity == crate::movement::MovementActivity::Resting)
    {
        DVec2::ZERO
    } else {
        accepted_velocity(
            history,
            unit,
            tick,
            dt,
            unit.speed
                .map(|speed| [unit.radius.to_bits(), unit.unit_type, speed.to_bits()]),
        )
        .unwrap_or(DVec2::ZERO)
    }
}
#[derive(Clone, Copy)]
struct Row {
    unit: u32,
    agent: Agent,
    stationary: bool,
}
struct View {
    rows: Vec<Row>,
    statics: Vec<u16>,
    width: f64,
    max_radius: f64,
}
impl View {
    fn cell(&self, p: DVec2) -> (i32, i32) {
        (
            (p.x / self.width).floor() as i32,
            (p.y / self.width).floor() as i32,
        )
    }
    #[allow(
        clippy::type_complexity,
        reason = "The callback streams permitted bodies without allocation."
    )]
    fn capture(
        frame: &MovementFrame,
        owner: usize,
        _tick: u32,
        dt: f32,
        _history: &[HistoryRow],
        profiles: &BTreeMap<u32, Member>,
        foreign: &mut dyn FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
    ) -> Option<Self> {
        if frame
            .units
            .iter()
            .filter(|u| usize::from(u.owner_slot) == owner)
            .count()
            > INDEX_BODIES
        {
            return None;
        }
        #[cfg(feature = "test-probes")]
        let own_forecast = std::env::var("TURTS_TEST_OWN_FORECAST").ok();
        let mut rows = Vec::with_capacity(INDEX_BODIES);
        for u in frame
            .units
            .iter()
            .filter(|u| usize::from(u.owner_slot) == owner)
        {
            if !u.position.is_finite() || !u.radius.is_finite() || u.radius <= 0. {
                return None;
            }
            let stationary = !moving(u)
                || u.progress
                    .as_ref()
                    .is_some_and(|p| p.activity == crate::movement::MovementActivity::Resting)
                || u.plain_progress
                    .as_ref()
                    .is_some_and(|p| p.activity == crate::movement::MovementActivity::Resting);
            let velocity = if !stationary && dt.is_finite() && dt > 0. {
                let request = MotionRequest {
                    unit_id: u.unit_id,
                    position: u.position,
                    goal: u.target.unwrap_or(u.position),
                    speed: u.speed.unwrap_or(0.),
                    radius: u.radius,
                    bounds: frame.bounds,
                    dt,
                    kind: MotionKind::Waypoint,
                };
                accepted_velocity(
                    _history,
                    u,
                    _tick,
                    dt,
                    profiles
                        .get(&u.unit_id)
                        .filter(|m| usize::from(m.owner) == owner)
                        .map(|m| m.motion_profile),
                )
                .unwrap_or_else(|| {
                    (request.preferred_position().as_dvec2() - u.position.as_dvec2())
                        / f64::from(dt)
                })
            } else {
                DVec2::ZERO
            };
            #[cfg(feature = "test-probes")]
            let velocity = match own_forecast.as_deref() {
                Some("accepted") => measured(_history, u, _tick, dt),
                Some("zero") => DVec2::ZERO,
                _ => velocity,
            };
            rows.push(Row {
                unit: u.unit_id,
                agent: Agent {
                    position: u.position.as_dvec2(),
                    velocity,
                    radius: f64::from(u.radius),
                },
                stationary,
            });
        }
        let mut complete = true;
        let supplied = foreign(owner as u8, &mut |unit, body| {
            if rows.len() == INDEX_BODIES
                || !body.position.is_finite()
                || !body.radius.is_finite()
                || body.radius <= 0.
            {
                complete = false;
                return false;
            }
            rows.push(Row {
                unit,
                agent: Agent {
                    position: body.position.as_dvec2(),
                    velocity: DVec2::ZERO,
                    radius: f64::from(body.radius),
                },
                stationary: true,
            });
            true
        });
        if !supplied || !complete {
            return None;
        }
        let width = (f64::from(frame.bounds.max_element()) / 4096.).max(1.);
        if !width.is_finite() {
            return None;
        }
        rows.sort_unstable_by(|a, b| {
            let cell = |p: DVec2| ((p.x / width).floor() as i32, (p.y / width).floor() as i32);
            cell(a.agent.position)
                .cmp(&cell(b.agent.position))
                .then_with(|| agent_cmp(a.agent, b.agent))
                .then(a.stationary.cmp(&b.stationary))
        });
        let max_radius = rows.iter().map(|r| r.agent.radius).fold(0., f64::max);
        let mut statics = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            if row.stationary {
                statics.push(index as u16);
            }
        }
        Some(Self {
            statics,
            rows,
            width,
            max_radius,
        })
    }
    /// Sparse x columns, binary-searched y ranges. Charge discovery, each
    /// visited column and every extracted row; an incomplete query is refused.
    fn query(
        &self,
        from: DVec2,
        to: DVec2,
        r: f64,
        charge: &mut dyn FnMut() -> bool,
        admit: &mut dyn FnMut() -> bool,
        mut visit: impl FnMut(Row),
    ) -> bool {
        if !admit() || !charge() {
            return false;
        }
        let low = self.cell(from.min(to) - DVec2::splat(r));
        let high = self.cell(from.max(to) + DVec2::splat(r));
        let mut start = self
            .rows
            .partition_point(|a| self.cell(a.agent.position).0 < low.0);
        while start < self.rows.len() && self.cell(self.rows[start].agent.position).0 <= high.0 {
            if !charge() {
                return false;
            }
            let x = self.cell(self.rows[start].agent.position).0;
            let end =
                start + self.rows[start..].partition_point(|a| self.cell(a.agent.position).0 == x);
            let lo = start
                + self.rows[start..end].partition_point(|a| self.cell(a.agent.position).1 < low.1);
            let hi = start
                + self.rows[start..end]
                    .partition_point(|a| self.cell(a.agent.position).1 <= high.1);
            for &row in &self.rows[lo..hi] {
                if !admit() || !charge() {
                    return false;
                }
                visit(row);
            }
            start = end;
        }
        true
    }
}
/// Complete static-only proof. Offsets preserve the frozen view's spatial order;
/// moving rows cannot consume proof credit or conceal a stopped foreign body.
#[allow(
    clippy::too_many_arguments,
    reason = "Frozen proof inputs and shared work are explicit"
)]
fn prove(
    view: &View,
    id: u32,
    from: Vec2,
    radius: f32,
    to: Vec2,
    visits: &mut usize,
    charge: &mut dyn FnMut() -> bool,
    clear: &mut dyn FnMut(Vec2, Vec2) -> bool,
) -> bool {
    if !to.is_finite() || *visits >= OWNER_VISITS {
        return false;
    }
    *visits += 1;
    if !charge() {
        return false;
    }
    let reach = f64::from(radius) + view.max_radius;
    let low = view.cell(from.as_dvec2().min(to.as_dvec2()) - DVec2::splat(reach));
    let high = view.cell(from.as_dvec2().max(to.as_dvec2()) + DVec2::splat(reach));
    let mut start = view
        .statics
        .partition_point(|&i| view.cell(view.rows[usize::from(i)].agent.position).0 < low.0);
    while start < view.statics.len()
        && view
            .cell(view.rows[usize::from(view.statics[start])].agent.position)
            .0
            <= high.0
    {
        if !charge() {
            return false;
        }
        let x = view
            .cell(view.rows[usize::from(view.statics[start])].agent.position)
            .0;
        let end = start
            + view.statics[start..]
                .partition_point(|&i| view.cell(view.rows[usize::from(i)].agent.position).0 == x);
        let lo = start
            + view.statics[start..end].partition_point(|&i| {
                view.cell(view.rows[usize::from(i)].agent.position).1 < low.1
            });
        let hi = start
            + view.statics[start..end].partition_point(|&i| {
                view.cell(view.rows[usize::from(i)].agent.position).1 <= high.1
            });
        for &i in &view.statics[lo..hi] {
            if *visits >= OWNER_VISITS {
                return false;
            }
            *visits += 1;
            if !charge() {
                return false;
            }
            let row = view.rows[usize::from(i)];
            if row.unit == id {
                continue;
            }
            if !charge()
                || !turts_rules::terrain::geometry::swept_disks_clear(
                    turts_rules::terrain::geometry::Point {
                        x: f64::from(from.x),
                        y: f64::from(from.y),
                    },
                    turts_rules::terrain::geometry::Point {
                        x: f64::from(to.x),
                        y: f64::from(to.y),
                    },
                    f64::from(radius),
                    turts_rules::terrain::geometry::Point {
                        x: row.agent.position.x,
                        y: row.agent.position.y,
                    },
                    turts_rules::terrain::geometry::Point {
                        x: row.agent.position.x,
                        y: row.agent.position.y,
                    },
                    row.agent.radius,
                    &mut turts_rules::terrain::geometry::Budget::new(1),
                )
                .unwrap_or(false)
            {
                return false;
            }
        }
        start = end;
    }
    charge() && clear(from, to)
}
/// Conservative failure on nonfinite intermediates; a stationary segment is a point.
fn segment_distance_squared(point: DVec2, from: DVec2, to: DVec2) -> Option<f64> {
    if !point.is_finite() || !from.is_finite() || !to.is_finite() {
        return None;
    }
    let delta = to - from;
    let offset = point - from;
    let length2 = delta.length_squared();
    let projection = offset.dot(delta);
    if !delta.is_finite() || !offset.is_finite() || !length2.is_finite() || !projection.is_finite()
    {
        return None;
    }
    let t = if length2 == 0. {
        0.
    } else {
        (projection / length2).clamp(0., 1.)
    };
    let distance2 = point.distance_squared(from + delta * t);
    (t.is_finite() && distance2.is_finite()).then_some(distance2)
}
fn agent_cmp(a: Agent, b: Agent) -> Ordering {
    a.position
        .x
        .total_cmp(&b.position.x)
        .then(a.position.y.total_cmp(&b.position.y))
        .then(a.radius.total_cmp(&b.radius))
        .then(a.velocity.x.total_cmp(&b.velocity.x))
        .then(a.velocity.y.total_cmp(&b.velocity.y))
}
#[derive(Clone, Copy)]
struct Actor {
    id: u32,
    owner: u8,
    index: usize,
    position: Vec2,
    radius: f32,
    speed: f32,
    goal: Vec2,
    destination: Vec2,
    kind: MotionKind,
    age: u32,
    attempted: bool,
    predictive_age: u32,
    predictive_attempted: bool,
    primary: Option<Vec2>,
    baseline: Option<Vec2>,
    primary_started: bool,
    predictive_serviced: bool,
    spent: usize,
    serviced: bool,
    ambiguous: bool,
    member: Option<u16>,
    advice: Option<Vec2>,
}
impl Actor {
    fn short_destination(&self, dt: f32) -> bool {
        let distance = self.position.as_dvec2().distance(self.goal.as_dvec2());
        let travel = f64::from(self.speed) * f64::from(dt);
        self.kind == MotionKind::Destination
            && distance.is_finite()
            && travel.is_finite()
            && distance <= travel
    }
    fn request(&self, bounds: Vec2, dt: f32) -> MotionRequest {
        MotionRequest {
            unit_id: self.id,
            position: self.position,
            goal: self.goal,
            speed: self.speed,
            radius: self.radius,
            bounds,
            dt,
            kind: self.kind,
        }
    }
}
fn physical_actor(a: &Actor, b: &Actor) -> Ordering {
    point_cmp(a.position, b.position)
        .then(a.radius.total_cmp(&b.radius))
        .then(a.speed.total_cmp(&b.speed))
        .then_with(|| point_cmp(a.goal, b.goal))
}
struct Fallback {
    id: u32,
    kind: MotionKind,
    destination: Vec2,
}
#[cfg_attr(test, derive(Clone))]
struct Ticket {
    spent: usize,
    reserved: usize,
    neighbors: [Agent; 8],
    hard_mask: u8,
    velocity: DVec2,
    len: usize,
    discovered: bool,
    refused: bool,
}
const EMPTY: Agent = Agent {
    position: DVec2::ZERO,
    velocity: DVec2::ZERO,
    radius: 0.,
};
/// Public live-phase reservation. It never depends on another viewer's private
/// row count, retained intent size, or the hidden authoritative frame length.
const JOINT_BASE_BYTES: usize = RETAINED_ALLOWANCE
    // The routing guidance clone and its lookup/order arrays are already gone.
    // Engine input frames predate this controller phase; retain caller/waits.
    + MEMBERS * 128 + 4096
    + SLOTS * INDEX_BODIES * (std::mem::size_of::<Row>() + 2)
    + MEMBERS * std::mem::size_of::<Actor>()
    + INDEX_BODIES * (8 + std::mem::size_of::<Fallback>())
    + std::mem::size_of::<Ticket>()
    + SLOTS * std::mem::size_of::<View>()
    + std::mem::size_of::<Guide<'static>>()
    + std::mem::size_of::<Vec<super::coherence::Member>>()
    + 16384;
const _: () = assert!(
    RETAINED_ALLOWANCE
        + INDEX_BODIES * PER_BODY_SCRATCH
        + FIXED_SCRATCH
        + SLOTS * INDEX_BODIES * (std::mem::size_of::<Row>() + 2)
        + MEMBERS * std::mem::size_of::<Actor>()
        + INDEX_BODIES * (8 + std::mem::size_of::<Fallback>())
        + MEMBERS * std::mem::size_of::<super::coherence::Member>()
        + std::mem::size_of::<Ticket>()
        + SLOTS * std::mem::size_of::<View>()
        + std::mem::size_of::<Guide<'static>>()
        + std::mem::size_of::<Vec<super::coherence::Member>>()
        + 16384
        <= SCRATCH_BYTES
);
pub(crate) struct Guide<'a> {
    actors: Vec<Actor>,
    fallback_ids: Vec<Fallback>,
    views: [Option<View>; SLOTS],
    dt: f32,
    bounds: Vec2,
    #[cfg(test)]
    primary_probe: Option<(u32, Ticket)>,
    joint_peak: usize,
    initial_visits: [usize; SLOTS],
    visits: [usize; SLOTS],
    clear: &'a mut dyn FnMut(usize, Vec2, Vec2) -> bool,
}
impl<'a> Guide<'a> {
    #[allow(clippy::too_many_arguments)]
    #[allow(
        clippy::type_complexity,
        reason = "The callback streams permitted bodies without allocation."
    )]
    pub(crate) fn capture(
        frame: &MovementFrame,
        dt: f32,
        tick: u32,
        state: &mut TrafficState,
        budget: &mut SweepBudget,
        foreign: &mut dyn FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
        scope: impl FnMut(usize) -> bool,
        kind: impl FnMut(usize) -> MotionKind,
        destination: impl FnMut(usize) -> Vec2,
        cohort: impl FnMut(usize) -> Option<u64>,
        clear: &'a mut dyn FnMut(usize, Vec2, Vec2) -> bool,
    ) -> Self {
        #[cfg(feature = "test-probes")]
        let joint_enabled = std::env::var("TURTS_TEST_JOINT_PLAN").is_ok_and(|value| value == "1");
        #[cfg(not(feature = "test-probes"))]
        let joint_enabled = false;
        Self::capture_inner(
            frame,
            dt,
            tick,
            state,
            budget,
            foreign,
            scope,
            kind,
            destination,
            cohort,
            clear,
            joint_enabled,
        )
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(
        clippy::type_complexity,
        clippy::needless_range_loop,
        reason = "Owner-local captures and callback stay explicit."
    )]
    fn capture_inner(
        frame: &MovementFrame,
        dt: f32,
        tick: u32,
        state: &mut TrafficState,
        budget: &mut SweepBudget,
        foreign: &mut dyn FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
        mut scope: impl FnMut(usize) -> bool,
        mut kind: impl FnMut(usize) -> MotionKind,
        mut destination: impl FnMut(usize) -> Vec2,
        mut cohort: impl FnMut(usize) -> Option<u64>,
        clear: &'a mut dyn FnMut(usize, Vec2, Vec2) -> bool,
        joint_enabled: bool,
    ) -> Self {
        if frame.units.len() > INDEX_BODIES {
            return Self {
                actors: Vec::new(),
                fallback_ids: Vec::new(),
                views: std::array::from_fn(|_| None),
                dt,
                bounds: frame.bounds,
                #[cfg(test)]
                primary_probe: None,
                joint_peak: 0,
                initial_visits: state.tick_owner_neighbors,
                visits: state.tick_owner_neighbors,
                clear,
            };
        }
        let mut service = Vec::with_capacity(INDEX_BODIES);
        service.extend(0..frame.units.len());
        service.sort_unstable_by_key(|&i| frame.units[i].unit_id);
        let oversized: [bool; SLOTS] = std::array::from_fn(|owner| {
            frame
                .units
                .iter()
                .filter(|u| usize::from(u.owner_slot) == owner)
                .count()
                > OWNER_MEMBERS
        });
        for owner in 0..SLOTS {
            if oversized[owner] {
                state.motion_history[owner].clear();
            }
        }
        for owner in 0..SLOTS {
            state.motion_history[owner].retain(|r| {
                r.tick.checked_add(1) == Some(tick)
                    && r.dt.to_bits() == dt.to_bits()
                    && service
                        .binary_search_by_key(&r.unit, |&i| frame.units[i].unit_id)
                        .ok()
                        .is_some_and(|i| {
                            let u = &frame.units[service[i]];
                            usize::from(u.owner_slot) == owner && bits(u.position) == bits(r.pose)
                        })
            });
        }
        let mut actors = Vec::with_capacity(MEMBERS);
        let mut fallback_ids = Vec::with_capacity(INDEX_BODIES);
        for (index, u) in frame.units.iter().enumerate() {
            if !scope(index) || !moving(u) {
                continue;
            }
            let owner = usize::from(u.owner_slot);
            if owner >= SLOTS {
                continue;
            }
            let motion_kind = kind(index);
            // A current complete public leg already has the advice it needs.
            // Do not spend optional model work that would bend it off that leg.
            if oversized[owner] || motion_kind == MotionKind::CertifiedWaypoint {
                fallback_ids.push(Fallback {
                    id: u.unit_id,
                    kind: motion_kind,
                    destination: destination(index),
                });
                continue;
            }
            assert!(actors.len() < MEMBERS);
            let history = state.motion_history[owner]
                .binary_search_by_key(&u.unit_id, |r| r.unit)
                .ok()
                .map(|i| state.motion_history[owner][i]);
            let prior = history.map(|r| r.attempt).filter(|&t| t != u32::MAX);
            let age = prior
                .or_else(|| history.map(|r| r.seen))
                .unwrap_or_else(|| {
                    state
                        .members
                        .get(&u.unit_id)
                        .map_or(tick, |m| m.accepted_tick)
                });
            let predictive_prior = history
                .map(|r| r.predictive_attempt)
                .filter(|&t| t != u32::MAX);
            let predictive_age = predictive_prior
                .or_else(|| history.map(|r| r.seen))
                .unwrap_or_else(|| {
                    state
                        .members
                        .get(&u.unit_id)
                        .map_or(tick, |m| m.accepted_tick)
                });
            actors.push(Actor {
                id: u.unit_id,
                owner: u.owner_slot,
                index,
                position: u.position,
                radius: u.radius,
                speed: u.speed.unwrap_or(0.),
                goal: u.target.unwrap_or(u.position),
                destination: destination(index),
                kind: motion_kind,
                age,
                attempted: prior.is_some(),
                predictive_age,
                predictive_attempted: predictive_prior.is_some(),
                primary: None,
                baseline: None,
                primary_started: false,
                predictive_serviced: false,
                spent: 0,
                serviced: false,
                ambiguous: false,
                member: None,
                advice: None,
            });
        }
        let views = std::array::from_fn(|owner| {
            if oversized[owner] {
                budget.model_view_refusals += 1;
                return None;
            }
            let view = View::capture(
                frame,
                owner,
                tick,
                dt,
                &state.motion_history[owner],
                &state.members,
                foreign,
            );
            if view.is_none() {
                budget.model_view_refusals += 1;
            }
            view
        });
        service.clear();
        service.extend(0..actors.len());
        service.sort_unstable_by(|&a, &b| {
            actors[a]
                .age
                .cmp(&actors[b].age)
                .then(actors[a].attempted.cmp(&actors[b].attempted))
                .then_with(|| physical_actor(&actors[a], &actors[b]))
                .then(actors[a].owner.cmp(&actors[b].owner))
        });
        for pair in service.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if actors[a].owner == actors[b].owner
                && actors[a].age == actors[b].age
                && actors[a].attempted == actors[b].attempted
                && physical_actor(&actors[a], &actors[b]) == Ordering::Equal
            {
                actors[a].ambiguous = true;
                actors[b].ambiguous = true;
            }
        }
        let mut visits = state.tick_owner_neighbors;
        // Complete owner batches only. Cohort membership comes from the original
        // order metadata, never temporary steering progress or actor handles.
        let mut members = Vec::with_capacity(MEMBERS);
        for (i, actor) in actors.iter().enumerate() {
            if oversized[actor.owner as usize] || views[actor.owner as usize].is_none() {
                continue;
            }
            let Some(group) = cohort(actor.index) else {
                continue;
            };
            let request = actor.request(frame.bounds, dt);
            let preferred = request.preferred_position().as_dvec2() - actor.position.as_dvec2();
            if actor.kind == MotionKind::Destination
                && actor.position.as_dvec2().distance(actor.goal.as_dvec2())
                    <= f64::from(actor.speed) * f64::from(dt)
            {
                continue;
            }
            if members.len() == MEMBERS {
                break;
            }
            let row = super::coherence::Member::new(
                i,
                actor.owner,
                group,
                actor.position,
                actor.radius,
                actor.speed,
                actor.goal,
                preferred,
            );
            if super::coherence::contribution(&row).is_some() {
                members.push(row);
            }
        }
        members.sort_unstable_by(|a, b| {
            a.owner
                .cmp(&b.owner)
                .then_with(|| super::coherence::physical(a, b))
        });
        let mut eligible = [false; SLOTS];
        for owner in 0..SLOTS {
            let count = members
                .iter()
                .filter(|r| usize::from(r.owner) == owner)
                .count();
            eligible[owner] = budget.model_available(owner as u8) >= count
                && OWNER_VISITS.saturating_sub(visits[owner]) >= count;
        }
        members.retain(|r| eligible[r.owner as usize]);
        for row in &members {
            let actor = &mut actors[row.actor];
            assert!(budget.charge_guidance_pair(actor.owner, &mut actor.spent));
            visits[actor.owner as usize] += 1;
            budget.coherence_members += 1;
            budget.coherence_operations += 1;
            budget.model_operations += 1;
        }
        super::coherence::prepare_cells(&mut members);
        assert!(members.len() <= usize::from(u16::MAX));
        for (index, row) in members.iter().enumerate() {
            actors[row.actor].member = Some(index as u16);
        }
        let member_bytes = members.capacity() * std::mem::size_of::<super::coherence::Member>();
        // Useful proof is work-conserving. The next physical actor can use all
        // unspent owner credit; no retry receives a fresh per-actor allowance.
        for &i in &service {
            if actors[i].ambiguous {
                continue;
            }
            let actor = &mut actors[i];
            let Some(member) = actor.member else {
                continue;
            };
            if actor.spent > TICKET - 32
                || budget.model_available(actor.owner) < 32
                || visits[actor.owner as usize] >= OWNER_VISITS
            {
                continue;
            }
            let Some(view) = &views[actor.owner as usize] else {
                continue;
            };
            let owner = actor.owner;
            let before = actor.spent;
            let mean = super::coherence::local_mean(&members, usize::from(member), || {
                if visits[owner as usize] >= OWNER_VISITS
                    || actor.spent == TICKET
                    || !budget.charge_guidance_pair(owner, &mut actor.spent)
                {
                    return false;
                }
                visits[owner as usize] += 1;
                budget.model_operations += 1;
                budget.coherence_operations += 1;
                true
            });
            // Even an empty/refused neighborhood is real discovery service.
            if actor.spent > before {
                actor.serviced = true;
                budget.coherence_attempts += 1;
            }
            let Some(blended) = mean.and_then(|mean| {
                super::coherence::blend(
                    actor
                        .request(frame.bounds, dt)
                        .preferred_position()
                        .as_dvec2()
                        - actor.position.as_dvec2(),
                    mean,
                )
            }) else {
                continue;
            };
            let next = (actor.position.as_dvec2() + blended).as_vec2();
            let mut charge = || {
                if actor.spent == TICKET || !budget.charge_guidance_pair(owner, &mut actor.spent) {
                    return false;
                }
                budget.model_operations += 1;
                budget.coherence_operations += 1;
                true
            };
            if prove(
                view,
                actor.id,
                actor.position,
                actor.radius,
                next,
                &mut visits[owner as usize],
                &mut charge,
                &mut |from, to| clear(actor.index, from, to),
            ) {
                actor.advice = Some(next);
                budget.coherence_advice += 1;
            }
        }
        drop(members);
        // Diagnostic comparison only: freeze the completed own coherence plan
        // before any predictive solve; never consume a peer's predictive output.
        #[cfg(feature = "test-probes")]
        let mut views = views;
        #[cfg(feature = "test-probes")]
        if std::env::var("TURTS_TEST_COHERENT_FORECAST").is_ok_and(|value| value == "1")
            && dt.is_finite()
            && dt > 0.
        {
            service.sort_unstable_by_key(|&i| actors[i].id);
            for (owner, view) in views.iter_mut().enumerate() {
                let Some(view) = view else { continue };
                for row in &mut view.rows {
                    if row.stationary {
                        continue;
                    }
                    let Ok(at) = service.binary_search_by_key(&row.unit, |&i| actors[i].id) else {
                        continue;
                    };
                    let actor = &actors[service[at]];
                    if usize::from(actor.owner) != owner {
                        continue;
                    }
                    let next = actor
                        .advice
                        .unwrap_or_else(|| actor.request(frame.bounds, dt).preferred_position());
                    row.agent.velocity =
                        (next.as_dvec2() - actor.position.as_dvec2()) / f64::from(dt);
                }
            }
        }
        actors.sort_unstable_by_key(|a| a.id);
        fallback_ids.sort_unstable_by_key(|r| r.id);
        service.clear();
        service.extend(0..actors.len());
        // Arrival-only rows retain metadata but cannot reserve a ticket they
        // always bypass. Private short waypoints still need steering service.
        service.retain(|&i| !actors[i].short_destination(dt));
        // Coherence can serve every actor cheaply. Predictive admission therefore
        // rotates on its own actual-spend clock, not the coherence clock.
        service.sort_unstable_by(|&a, &b| {
            actors[a]
                .predictive_age
                .cmp(&actors[b].predictive_age)
                .then(
                    actors[a]
                        .predictive_attempted
                        .cmp(&actors[b].predictive_attempted),
                )
                .then_with(|| physical_actor(&actors[a], &actors[b]))
                .then(actors[a].owner.cmp(&actors[b].owner))
        });
        #[cfg(feature = "test-probes")]
        let predictive_off =
            std::env::var("TURTS_TEST_PREDICTIVE_OFF").is_ok_and(|value| value == "1");
        #[cfg(not(feature = "test-probes"))]
        let predictive_off = false;
        let mut guide = Self {
            actors,
            fallback_ids,
            views,
            dt,
            bounds: frame.bounds,
            #[cfg(test)]
            primary_probe: None,
            joint_peak: 0,
            initial_visits: state.tick_owner_neighbors,
            visits,
            clear,
        };
        let mut joint_peak = 0;
        for owner_index in 0..SLOTS {
            // The previous owner's retained index is dropped at this loop edge.
            let mut plan = if joint_enabled {
                guide.prepare_joint(owner_index, frame, budget, JOINT_BASE_BYTES)
            } else {
                None
            };
            let owner_ready = !joint_enabled || plan.is_some();
            let mut cursor = 0;
            while !predictive_off && owner_ready && cursor < service.len() {
                if usize::from(guide.actors[service[cursor]].owner) != owner_index {
                    cursor += 1;
                    continue;
                }
                let mut end = cursor + 1;
                while end < service.len()
                    && guide.actors[service[cursor]].owner == guide.actors[service[end]].owner
                    && guide.actors[service[cursor]].predictive_age
                        == guide.actors[service[end]].predictive_age
                    && guide.actors[service[cursor]].predictive_attempted
                        == guide.actors[service[end]].predictive_attempted
                    && physical_actor(&guide.actors[service[cursor]], &guide.actors[service[end]])
                        == Ordering::Equal
                {
                    end += 1;
                }
                if plan.is_some() && end - cursor > 1 {
                    cursor = end;
                    continue;
                }
                let owner = guide.actors[service[cursor]].owner;
                let needed = service[cursor..end]
                    .iter()
                    .map(|&i| TICKET - guide.actors[i].spent)
                    .sum::<usize>();
                let sufficient = service[cursor..end]
                    .iter()
                    .all(|&i| predictive_allowance(guide.actors[i].spent, usize::MAX).is_some());
                let visit_bound = needed + (end - cursor);
                let protected_visits = end - cursor == 1
                    || visit_bound <= OWNER_VISITS.saturating_sub(guide.visits[usize::from(owner)]);
                if guide.views[usize::from(owner)].is_some()
                    && sufficient
                    && protected_visits
                    && budget.reserve_model_amount(owner, needed)
                {
                    let mut unused = 0;
                    for &i in &service[cursor..end] {
                        let before = guide.actors[i].spent;
                        let mut ticket = Ticket {
                            spent: before,
                            reserved: TICKET - before,
                            neighbors: [EMPTY; 8],
                            hard_mask: 0,
                            velocity: DVec2::ZERO,
                            len: 0,
                            discovered: false,
                            refused: false,
                        };
                        let request = guide.actors[i].request(frame.bounds, dt);
                        let result =
                            guide.predict_primary(i, request, &mut ticket, budget, plan.as_mut());
                        guide.actors[i].primary = result;
                        guide.actors[i].primary_started = true;
                        guide.actors[i].predictive_serviced = ticket.spent > before;
                        guide.actors[i].spent = ticket.spent;
                        unused += ticket.reserved;
                        #[cfg(test)]
                        {
                            guide.primary_probe = Some((guide.actors[i].id, ticket));
                        }
                    }
                    // Keep the whole physical batch protected; only the next batch
                    // may consume unused credit. No actor receives another ticket.
                    budget.release_model(owner, unused);
                }
                cursor = end;
            }
            if let Some(plan) = plan.as_ref() {
                joint_peak = joint_peak.max(plan.peak_bytes());
                budget.index_work.add(plan.metadata());
            }
        }
        let bytes = frame.units.len().min(INDEX_BODIES) * PER_BODY_SCRATCH
            + FIXED_SCRATCH
            + guide.actors.capacity() * std::mem::size_of::<Actor>()
            + service.capacity() * 8
            + guide.fallback_ids.capacity() * std::mem::size_of::<Fallback>()
            + std::mem::size_of::<Ticket>().max(member_bytes)
            + guide
                .views
                .iter()
                .flatten()
                .map(|v| v.rows.capacity() * std::mem::size_of::<Row>() + v.statics.capacity() * 2)
                .sum::<usize>()
            + SLOTS * std::mem::size_of::<View>()
            + std::mem::size_of::<Self>()
            + std::mem::size_of::<Vec<super::coherence::Member>>()
            + 16384;
        let joint_live_bytes = bytes
            .saturating_sub(frame.units.len().min(INDEX_BODIES) * PER_BODY_SCRATCH)
            .saturating_sub(FIXED_SCRATCH)
            .saturating_sub(std::mem::size_of::<Ticket>().max(member_bytes))
            + MEMBERS * 128
            + 4096
            + std::mem::size_of::<Ticket>()
            + joint_peak.max(guide.joint_peak);
        // Keep every ordinary routing/coherence peak, but do not add a planning
        // phase to allocations which ended before it began.
        let bytes = bytes.max(joint_live_bytes);
        state.scratch_bound = state.scratch_bound.max(bytes);
        assert!(state.retained_bytes() + bytes <= SCRATCH_BYTES);
        guide
    }
}

impl Guide<'_> {
    /// A terrain retry prefix is a private waypoint, not a new public goal.
    /// Preserve the captured view, original destination and already spent work.
    pub(crate) fn mark_waypoint(&mut self, unit: u32) {
        if let Ok(i) = self.actors.binary_search_by_key(&unit, |a| a.id) {
            self.actors[i].kind = MotionKind::Waypoint;
            self.actors[i].advice = None;
            self.actors[i].primary = None;
            self.actors[i].baseline = None;
        } else if let Ok(i) = self.fallback_ids.binary_search_by_key(&unit, |r| r.id) {
            self.fallback_ids[i].kind = MotionKind::Waypoint;
        }
    }
    pub(crate) fn record(&self, state: &mut TrafficState, tick: u32, dt: f32) {
        for actor in &self.actors {
            let predictive = actor.predictive_serviced;
            if actor.serviced || predictive {
                mark_attempt(
                    state,
                    actor.id,
                    actor.owner,
                    actor.position,
                    tick,
                    dt,
                    actor.serviced,
                    predictive,
                );
            }
        }
        let delta = self
            .visits
            .iter()
            .zip(self.initial_visits)
            .map(|(a, b)| a - b)
            .sum::<usize>();
        state.tick_owner_neighbors = self.visits;
        state.tick_neighbors += delta;
        state.neighbor_visits += delta as u64;
    }
}

fn predictive_allowance(spent: usize, available: usize) -> Option<usize> {
    let remaining = TICKET.checked_sub(spent)?;
    (remaining >= 64 && available >= remaining).then_some(remaining)
}

fn spend(ticket: &mut Ticket, owner: u8, budget: &mut SweepBudget) -> bool {
    if ticket.spent == TICKET {
        return false;
    }
    if ticket.reserved > 0 {
        ticket.reserved -= 1;
        budget.release_model(owner, 1);
    }
    if !budget.charge_guidance_pair(owner, &mut ticket.spent) {
        return false;
    }
    budget.model_operations += 1;
    true
}
impl MotionGuide for Guide<'_> {
    fn arrival_destination(&self, id: u32) -> Option<Vec2> {
        self.actors
            .binary_search_by_key(&id, |a| a.id)
            .ok()
            .map(|i| self.actors[i].destination)
            .or_else(|| {
                self.fallback_ids
                    .binary_search_by_key(&id, |r| r.id)
                    .ok()
                    .map(|i| self.fallback_ids[i].destination)
            })
    }
    fn kind(&self, id: u32) -> MotionKind {
        self.actors
            .binary_search_by_key(&id, |a| a.id)
            .ok()
            .map(|i| self.actors[i].kind)
            .or_else(|| {
                self.fallback_ids
                    .binary_search_by_key(&id, |r| r.id)
                    .ok()
                    .map(|i| self.fallback_ids[i].kind)
            })
            .unwrap_or(MotionKind::Destination)
    }
    fn next_position(&mut self, request: MotionRequest, budget: &mut SweepBudget) -> Option<Vec2> {
        let fallback = request.preferred_position();
        let distance = request
            .position
            .as_dvec2()
            .distance(request.goal.as_dvec2());
        let travel = f64::from(request.speed) * f64::from(request.dt);
        let Some(i) = self
            .actors
            .binary_search_by_key(&request.unit_id, |a| a.id)
            .ok()
        else {
            if let Ok(i) = self
                .fallback_ids
                .binary_search_by_key(&request.unit_id, |r| r.id)
            {
                if self.fallback_ids[i].kind == MotionKind::Destination
                    && request.kind == MotionKind::Destination
                    && distance.is_finite()
                    && travel.is_finite()
                    && distance <= travel
                {
                    return None;
                }
                if self.fallback_ids[i].kind != MotionKind::CertifiedWaypoint {
                    budget.model_fallbacks += 1;
                }
                return Some(fallback);
            }
            return None;
        };
        let actor = &self.actors[i];
        if actor.kind == MotionKind::Destination
            && request.kind == MotionKind::Destination
            && distance.is_finite()
            && travel.is_finite()
            && distance <= travel
        {
            return None;
        }
        let matched = actor.kind == request.kind
            && bits(actor.position) == bits(request.position)
            && bits(actor.goal) == bits(request.goal)
            && actor.speed.to_bits() == request.speed.to_bits()
            && actor.radius.to_bits() == request.radius.to_bits()
            && self.dt.to_bits() == request.dt.to_bits()
            && bits(self.bounds) == bits(request.bounds);
        if matched && let Some(primary) = actor.primary {
            return Some(primary);
        }
        if !actor.primary_started || (actor.primary.is_some() && !matched) {
            budget.model_fallbacks += 1;
        }
        Some(if matched {
            actor.baseline.or(actor.advice).unwrap_or(fallback)
        } else {
            fallback
        })
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Attempt identity and physical state stay explicit."
)]
fn mark_attempt(
    state: &mut TrafficState,
    unit: u32,
    owner: u8,
    position: Vec2,
    tick: u32,
    dt: f32,
    coherence: bool,
    predictive: bool,
) {
    let owner = owner as usize;
    let rows = &mut state.motion_history[owner];
    if rows.capacity() == 0 {
        *rows = Vec::with_capacity(OWNER_MEMBERS);
    }
    match rows.binary_search_by_key(&unit, |r| r.unit) {
        Ok(i) => {
            if coherence {
                rows[i].attempt = tick;
            }
            if predictive {
                rows[i].predictive_attempt = tick;
            }
        }
        Err(i) if rows.len() < OWNER_MEMBERS => rows.insert(
            i,
            HistoryRow {
                unit,
                from: position,
                pose: position,
                tick: u32::MAX,
                dt,
                attempt: if coherence { tick } else { u32::MAX },
                predictive_attempt: if predictive { tick } else { u32::MAX },
                seen: tick,
            },
        ),
        _ => {}
    }
}
pub(crate) fn publish(
    state: &mut TrafficState,
    frame: &MovementFrame,
    outputs: &[MovementFrameOutput],
    tick: u32,
    dt: f32,
) {
    if frame.units.len() > INDEX_BODIES || outputs.len() > INDEX_BODIES {
        for rows in &mut state.motion_history {
            rows.clear();
        }
        return;
    }
    let mut lookup = Vec::with_capacity(frame.units.len());
    lookup.extend(frame.units.iter().enumerate().map(|(i, u)| (u.unit_id, i)));
    lookup.sort_unstable_by_key(|r| r.0);
    let mut output_lookup = Vec::with_capacity(outputs.len());
    output_lookup.extend(outputs.iter().enumerate().map(|(i, u)| (u.unit_id, i)));
    output_lookup.sort_unstable_by_key(|r| r.0);
    for owner in 0..SLOTS {
        let rows = &mut state.motion_history[owner];
        if frame
            .units
            .iter()
            .filter(|u| usize::from(u.owner_slot) == owner)
            .count()
            > OWNER_MEMBERS
        {
            rows.clear();
            continue;
        }
        rows.retain(|r| {
            lookup
                .binary_search_by_key(&r.unit, |r| r.0)
                .ok()
                .is_some_and(|i| {
                    let u = &frame.units[lookup[i].1];
                    usize::from(u.owner_slot) == owner && u.speed.is_some()
                })
        });
        for unit in frame
            .units
            .iter()
            .filter(|u| usize::from(u.owner_slot) == owner && u.speed.is_some())
        {
            let Ok(output_index) = output_lookup.binary_search_by_key(&unit.unit_id, |r| r.0)
            else {
                continue;
            };
            let output = &outputs[output_lookup[output_index].1];
            if let Some(member) = state
                .members
                .get_mut(&unit.unit_id)
                .filter(|m| usize::from(m.owner) == owner)
            {
                member.motion_profile = [
                    unit.radius.to_bits(),
                    unit.unit_type,
                    unit.speed.unwrap().to_bits(),
                ];
            }
            if rows.capacity() == 0 {
                *rows = Vec::with_capacity(OWNER_MEMBERS);
            }
            match rows.binary_search_by_key(&unit.unit_id, |r| r.unit) {
                Ok(i) => {
                    let attempt = rows[i].attempt;
                    let predictive_attempt = rows[i].predictive_attempt;
                    let seen = rows[i].seen;
                    rows[i] = HistoryRow {
                        unit: unit.unit_id,
                        from: unit.position,
                        pose: output.position,
                        tick,
                        dt,
                        attempt,
                        predictive_attempt,
                        seen,
                    };
                }
                Err(i) if rows.len() < OWNER_MEMBERS => rows.insert(
                    i,
                    HistoryRow {
                        unit: unit.unit_id,
                        from: unit.position,
                        pose: output.position,
                        tick,
                        dt,
                        attempt: u32::MAX,
                        predictive_attempt: u32::MAX,
                        seen: tick,
                    },
                ),
                _ => {}
            }
        }
    }
}

pub(crate) fn observed(
    world: &bevy_ecs::world::World,
    frame: &MovementFrame,
    phase: &crate::terrain::TerrainPhase,
    viewer: u8,
    emit: &mut dyn FnMut(u32, ObservedBody) -> bool,
) -> bool {
    use turts_mod_api::ObservationPhase;
    let tick = world.get_resource::<crate::Tick>().map_or(0, |t| t.0);
    if phase.tick() != tick || phase.observation_phase() != ObservationPhase::Start {
        return false;
    }
    if world.contains_resource::<crate::visibility::VisibilityState>() {
        let Some(basis) = crate::visibility::basis(world, ObservationPhase::Start)
            .filter(|b| phase.knowledge_basis() == Some(b.stamp()) && b.stamp().tick == tick)
        else {
            return false;
        };
        let Some(catalog) = world.get_resource::<crate::bootstrap::InstalledCatalog>() else {
            return false;
        };
        for fact in basis.permitted_units(viewer).filter(|f| f.owner != viewer) {
            let Some(def) = catalog.0.units.get(fact.content.as_ref()) else {
                return false;
            };
            let turts_mod_api::Footprint::Circle { radius } = def.footprint;
            if !emit(
                fact.unit,
                ObservedBody {
                    position: Vec2::new(fact.position.x, fact.position.y),
                    radius,
                },
            ) {
                return false;
            }
        }
    } else {
        for unit in frame.units.iter().filter(|u| u.owner_slot != viewer) {
            let Some(entity) = crate::unit_entity(world, unit.unit_id) else {
                return false;
            };
            let (Some(position), Some(kind)) = (
                world.get::<crate::Position>(entity),
                world.get::<crate::UnitType>(entity),
            ) else {
                return false;
            };
            let Some(def) = world.resource::<crate::UnitTypes>().get(kind.0) else {
                return false;
            };
            if !emit(
                unit.unit_id,
                ObservedBody {
                    position: position.0,
                    radius: def.radius,
                },
            ) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
#[allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
mod tests {
    use super::*;
    fn frame(n: usize) -> MovementFrame {
        MovementFrame {
            bounds: Vec2::splat(1000.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units: (0..n)
                .map(|i| MovementFrameUnit {
                    unit_id: i as u32 + 1,
                    owner_slot: 0,
                    unit_type: 0,
                    position: Vec2::new(2. + (i % 40) as f32 * 2., 2. + (i / 40) as f32 * 2.),
                    radius: 0.3,
                    speed: Some(3.5),
                    target: Some(Vec2::new(900., 100.)),
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                })
                .collect(),
        }
    }
    fn outputs(f: &MovementFrame, delta: Vec2) -> Vec<MovementFrameOutput> {
        f.units
            .iter()
            .map(|u| MovementFrameOutput {
                unit_id: u.unit_id,
                position: u.position + delta,
                target: u.target,
                progress: None,
                plain_progress: None,
                physical_escape: None,
                failed_blocked: false,
                arrival_slot: None,
                #[cfg(feature = "test-probes")]
                probe: Default::default(),
            })
            .collect()
    }
    fn none(_: u8, _: &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool {
        true
    }
    fn request(u: &MovementFrameUnit) -> MotionRequest {
        MotionRequest {
            unit_id: u.unit_id,
            position: u.position,
            goal: u.target.unwrap(),
            speed: u.speed.unwrap(),
            radius: u.radius,
            bounds: Vec2::splat(1000.),
            dt: 0.05,
            kind: MotionKind::Waypoint,
        }
    }
    #[test]
    fn certified_public_waypoint_keeps_its_segment_without_optional_model_work() {
        let mut f = frame(1);
        f.units[0].target = Some(Vec2::new(20., 2.));
        let mut state = TrafficState::default();
        let mut budget = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let destination = Vec2::new(20., 9.);
        let mut guide = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::CertifiedWaypoint,
            |_| destination,
            |_| Some(1),
            &mut clear,
        );
        let mut motion = request(&f.units[0]);
        motion.kind = guide.kind(motion.unit_id);
        assert_eq!(
            guide.next_position(motion, &mut budget),
            Some(Vec2::new(2.175, 2.)),
            "a certified horizontal leg cannot be bent toward another destination"
        );
        assert_eq!(
            guide.arrival_destination(motion.unit_id),
            Some(destination),
            "direct private leg must retain its original destination provenance"
        );
        assert_eq!(
            budget.model_operations, 0,
            "ignored optional work is not admitted"
        );
        assert_eq!(budget.model_reserved, [0; SLOTS]);
        // Final terrain retry changes this ephemeral authority to its own prefix.
        guide.mark_waypoint(motion.unit_id);
        motion.kind = guide.kind(motion.unit_id);
        motion.goal = motion.position + Vec2::new(0.01, 0.);
        assert_eq!(guide.next_position(motion, &mut budget), Some(motion.goal));
    }

    #[test]
    fn accepted_stationary_peers_do_not_forecast_unearned_waypoint_motion() {
        // Exact permitted own-body inputs from GM2 tick 4800. This checks the
        // velocity-advice seam, not a replay of the missing controller history.
        let mut f = frame(8);
        let inputs = [
            (
                26,
                Vec2::new(47.88087844848633, 28.223209381103516),
                1.1,
                Some(Vec2::new(48.18081, 27.381725)),
            ),
            (
                27,
                Vec2::new(46.421756744384766, 26.410892486572266),
                1.1,
                Some(Vec2::new(46.516666, 27.161936)),
            ),
            (
                30,
                Vec2::new(50.20627975463867, 28.675945281982422),
                1.1,
                Some(Vec2::new(47.7, 27.983717)),
            ),
            (
                22,
                Vec2::new(48.849998474121094, 29.975574493408203),
                0.4,
                None,
            ),
            (
                34,
                Vec2::new(46.54999923706055, 29.975574493408203),
                1.1,
                None,
            ),
            (29, Vec2::new(44.25, 25.991857528686523), 1.1, None),
            (
                21,
                Vec2::new(48.849998474121094, 25.991857528686523),
                0.4,
                None,
            ),
            (32, Vec2::new(52.3, 27.983717), 1.1, None),
        ];
        for (u, (id, position, radius, target)) in f.units.iter_mut().zip(inputs) {
            u.unit_id = id;
            u.position = position;
            u.radius = radius;
            u.target = target;
            u.speed = target.map(|_| 2.5);
        }
        let mut state = TrafficState::default();
        super::super::apply(&f, &mut f.clone(), 0.05, 4799, &mut state, |_, _, _| true);
        publish(&mut state, &f, &outputs(&f, Vec2::ZERO), 4799, 0.05);
        let mut b = SweepBudget::new(4800);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture_inner(
            &f,
            0.05,
            4800,
            &mut state,
            &mut b,
            &mut none,
            |i| f.units[i].target.is_some(),
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
            false,
        );
        let u = &f.units[1];
        let next = g.next_position(request(u), &mut b).unwrap();
        assert!(
            (next - u.position).dot(u.target.unwrap() - u.position) > 0.,
            "a proven held peer must not create a fictitious approaching velocity: {next:?}"
        );
        assert!(
            g.views[0]
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .filter(|r| [26, 27, 30].contains(&r.unit))
                .all(|r| !r.stationary)
        );
    }
    #[test]
    fn tiny_accepted_motion_and_profile_fences_control_peer_forecast() {
        let mut f = frame(1);
        let mut state = TrafficState::default();
        super::super::apply(&f, &mut f.clone(), 0.05, 1, &mut state, |_, _, _| true);
        let out = outputs(&f, Vec2::new(0.0001, 0.00002));
        publish(&mut state, &f, &out, 1, 0.05);
        f.units[0].position = out[0].position;
        let actual =
            (out[0].position.as_dvec2() - Vec2::new(2., 2.).as_dvec2()) / f64::from(0.05f32);
        let view = View::capture(
            &f,
            0,
            2,
            0.05,
            &state.motion_history[0],
            &state.members,
            &mut none,
        )
        .unwrap();
        assert_eq!(view.rows[0].agent.velocity, actual);
        assert!(actual.length() < 0.01);
        assert!(!view.rows[0].stationary);
        for fence in 0..7 {
            let mut changed = f.clone();
            let (mut tick, mut dt) = (2, 0.05);
            match fence {
                0 => tick += 1,
                1 => dt = 0.1,
                2 => changed.units[0].position.x += 0.01,
                3 => changed.units[0].radius += 0.1,
                4 => changed.units[0].unit_type += 1,
                5 => changed.units[0].speed = Some(2.),
                6 => changed.units[0].owner_slot = 1,
                _ => unreachable!(),
            }
            let owner = usize::from(changed.units[0].owner_slot);
            let view = View::capture(
                &changed,
                owner,
                tick,
                dt,
                &state.motion_history[owner],
                &state.members,
                &mut none,
            )
            .unwrap();
            let mut req = request(&changed.units[0]);
            req.dt = dt;
            let fallback =
                (req.preferred_position().as_dvec2() - req.position.as_dvec2()) / f64::from(dt);
            assert_eq!(
                view.rows[0].agent.velocity, fallback,
                "invalid binding {fence}"
            );
            assert!(!view.rows[0].stationary);
        }
        let unknown = View::capture(&f, 0, 2, 0.05, &[], &state.members, &mut none).unwrap();
        let req = request(&f.units[0]);
        assert_eq!(
            unknown.rows[0].agent.velocity,
            (req.preferred_position().as_dvec2() - req.position.as_dvec2()) / f64::from(req.dt)
        );
    }
    #[test]
    fn accepted_history_resets_owner_tick_pose_and_dt_and_stopped_peers_are_zero() {
        let mut f = frame(1);
        let mut state = TrafficState::default();
        let out = outputs(&f, Vec2::X * 0.1);
        publish(&mut state, &f, &out, 1, 0.05);
        f.units[0].position = out[0].position;
        let actual = (out[0].position - Vec2::new(2., 2.)).as_dvec2() / f64::from(0.05f32);
        assert_eq!(
            measured(&state.motion_history[0], &f.units[0], 2, 0.05),
            actual
        );
        assert_eq!(
            measured(&state.motion_history[1], &f.units[0], 2, 0.05),
            DVec2::ZERO
        );
        assert_eq!(
            measured(&state.motion_history[0], &f.units[0], 3, 0.05),
            DVec2::ZERO
        );
        assert_eq!(
            measured(&state.motion_history[0], &f.units[0], 2, 0.1),
            DVec2::ZERO
        );
        f.units[0].position.x += 0.01;
        assert_eq!(
            measured(&state.motion_history[0], &f.units[0], 2, 0.05),
            DVec2::ZERO
        );
        f.units[0].position = out[0].position;
        f.units[0].target = None;
        assert_eq!(
            measured(&state.motion_history[0], &f.units[0], 2, 0.05),
            DVec2::ZERO
        );
    }
    #[test]
    fn foreign_live_changes_never_replace_current_captured_facts() {
        let base = frame(1);
        let mut results = Vec::new();
        for position in [Vec2::new(20., 20.), Vec2::splat(900.)] {
            let mut f = base.clone();
            let mut foreign = f.units[0].clone();
            foreign.unit_id = 500;
            foreign.owner_slot = 1;
            foreign.position = position;
            foreign.radius = 50.;
            foreign.target = Some(Vec2::ZERO);
            f.units.push(foreign);
            let mut state = TrafficState::default();
            let mut budget = SweepBudget::new(1);
            let mut clear = |_, _, _| true;
            let mut captured = |owner, emit: &mut dyn FnMut(u32, ObservedBody) -> bool| {
                if owner == 0 {
                    emit(
                        500,
                        ObservedBody {
                            position: Vec2::new(20., 20.),
                            radius: 0.3,
                        },
                    )
                } else {
                    true
                }
            };
            let mut guide = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut budget,
                &mut captured,
                |i| i == 0,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap_or(f.units[i].position),
                |_| None,
                &mut clear,
            );
            let proposed = guide.next_position(request(&f.units[0]), &mut budget);
            guide.finish_prediction(&mut budget);
            results.push((proposed, budget.pairs[0], guide.visits[0]));
        }
        assert_eq!(results[0], results[1]);
    }
    #[test]
    fn whole_tickets_share_detour_credit_and_never_refill_on_retry() {
        let f = frame(1);
        let mut state = TrafficState::default();
        let mut budget = SweepBudget::new(1);
        budget.pairs[0] = OWNER_VISITS - 127;
        let mut clear = |_, _, _| true;
        let mut guide = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
        );
        assert!(guide.actors.iter().all(|a| !a.primary_started));
        assert_eq!(
            guide.next_position(request(&f.units[0]), &mut budget),
            Some(request(&f.units[0]).preferred_position())
        );
        drop(guide);
        let mut budget = SweepBudget::new(1);
        let mut guide = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
        );
        assert_eq!(budget.model_reserved[0], 0);
        for _ in 0..50 {
            guide.next_position(request(&f.units[0]), &mut budget);
            guide.finish_prediction(&mut budget);
        }
        assert!(guide.actors[0].spent <= 128);
        assert!(budget.model_operations <= 128);
        assert_eq!(budget.model_reserved[0], 0);
    }
    #[test]
    fn actual_attempt_age_rotates_useful_service_across_a_large_cohort() {
        let mut f = frame(500);
        let mut state = TrafficState::default();
        let mut served = BTreeSet::new();
        for tick in 1..=4 {
            let mut budget = SweepBudget::new(tick);
            let mut clear = |_, _, _| true;
            let mut guide = Guide::capture(
                &f,
                0.05,
                tick,
                &mut state,
                &mut budget,
                &mut none,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap_or(f.units[i].position),
                |_| None,
                &mut clear,
            );
            for u in &f.units {
                guide.next_position(request(u), &mut budget);
            }
            for a in &guide.actors {
                if a.predictive_serviced {
                    served.insert(a.id);
                }
            }
            guide.finish_prediction(&mut budget);
            guide.record(&mut state, tick, 0.05);
            drop(guide);
            let out = outputs(&f, Vec2::X * 0.01);
            publish(&mut state, &f, &out, tick, 0.05);
            for (u, o) in f.units.iter_mut().zip(out) {
                u.position = o.position;
            }
        }
        assert_eq!(served.len(), 500);
    }
    #[test]
    fn fixed_history_capacity_and_oversized_owner_reset_do_not_select_ids() {
        let mut state = TrafficState::default();
        let mut f = frame(OWNER_MEMBERS);
        publish(&mut state, &f, &outputs(&f, Vec2::ZERO), 1, 0.05);
        assert_eq!(state.motion_history[0].capacity(), OWNER_MEMBERS);
        for u in &mut f.units {
            u.owner_slot = 1;
        }
        publish(&mut state, &f, &outputs(&f, Vec2::ZERO), 2, 0.05);
        assert!(state.motion_history[0].is_empty());
        assert_eq!(
            state
                .motion_history
                .iter()
                .map(Vec::capacity)
                .sum::<usize>(),
            MEMBERS
        );
        let oversized = frame(OWNER_MEMBERS + 1);
        publish(
            &mut state,
            &oversized,
            &outputs(&oversized, Vec2::ZERO),
            3,
            0.05,
        );
        assert!(state.motion_history[0].is_empty());
        let mut budget = SweepBudget::new(4);
        let mut clear = |_, _, _| true;
        let guide = Guide::capture(
            &oversized,
            0.05,
            4,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| {
                oversized.units[i]
                    .target
                    .unwrap_or(oversized.units[i].position)
            },
            |_| None,
            &mut clear,
        );
        assert!(guide.views[0].is_none());
        assert!(
            guide
                .actors
                .iter()
                .filter(|a| a.owner == 0)
                .all(|a| !a.primary_started)
        );
        assert!(state.retained_bytes() + state.scratch_bytes() <= SCRATCH_BYTES);
    }
    #[test]
    fn hidden_owner_view_failure_and_rows_cannot_change_protected_admission() {
        let f = frame(2);
        let mut result = Vec::new();
        for hidden in [false, true] {
            let mut state = TrafficState::default();
            let mut budget = SweepBudget::new(1);
            let mut clear = |_, _, _| true;
            let mut supplier = |owner, emit: &mut dyn FnMut(u32, ObservedBody) -> bool| {
                if owner == 1 && hidden {
                    for i in 0..INDEX_BODIES {
                        if !emit(
                            10000 + i as u32,
                            ObservedBody {
                                position: Vec2::splat(500.),
                                radius: 100.,
                            },
                        ) {
                            return false;
                        }
                    }
                    false
                } else {
                    true
                }
            };
            let mut guide = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut budget,
                &mut supplier,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap_or(f.units[i].position),
                |_| None,
                &mut clear,
            );
            let a = guide.next_position(request(&f.units[0]), &mut budget);
            guide.finish_prediction(&mut budget);
            result.push((a, budget.pairs[0], guide.visits[0]));
        }
        assert_eq!(result[0], result[1]);
    }
    #[test]
    fn maximum_model_views_tickets_and_history_fit_the_existing_traffic_limit() {
        let mut f = frame(MEMBERS);
        for (i, u) in f.units.iter_mut().enumerate() {
            u.owner_slot = (i / OWNER_MEMBERS) as u8;
        }
        let mut state = TrafficState::default();
        publish(&mut state, &f, &outputs(&f, Vec2::ZERO), 1, 0.05);
        let mut budget = SweepBudget::new(2);
        let mut clear = |_, _, _| true;
        let mut foreign = |_: u8, emit: &mut dyn FnMut(u32, ObservedBody) -> bool| {
            for i in 0..INDEX_BODIES - OWNER_MEMBERS {
                if !emit(
                    100_000 + i as u32,
                    ObservedBody {
                        position: Vec2::new(500. + (i % 100) as f32, 500. + (i / 100) as f32),
                        radius: 0.3,
                    },
                ) {
                    return false;
                }
            }
            true
        };
        let mut guide = Guide::capture(
            &f,
            0.05,
            2,
            &mut state,
            &mut budget,
            &mut foreign,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
        );
        assert_eq!(
            guide
                .views
                .iter()
                .flatten()
                .map(|v| v.rows.capacity())
                .sum::<usize>(),
            SLOTS * INDEX_BODIES
        );
        assert_eq!(
            guide.actors.iter().filter(|a| a.primary_started).count(),
            MEMBERS
        );
        assert_eq!(
            state
                .motion_history
                .iter()
                .map(Vec::capacity)
                .sum::<usize>(),
            MEMBERS
        );
        assert!(state.retained_bytes() + state.scratch_bytes() <= SCRATCH_BYTES);
        eprintln!(
            "model maximum: history_capacity={} view_capacity={} tickets={} retained={} scratch={} total={} limit={}",
            MEMBERS,
            SLOTS * INDEX_BODIES,
            guide.actors.iter().filter(|a| a.primary_started).count(),
            state.retained_bytes(),
            state.scratch_bytes(),
            state.retained_bytes() + state.scratch_bytes(),
            SCRATCH_BYTES
        );
        guide.finish_prediction(&mut budget);
        assert_eq!(budget.model_reserved, [0; SLOTS]);
    }
    #[test]
    fn production_guide_preserves_short_destination_and_caps_short_waypoint() {
        for kind in [MotionKind::Destination, MotionKind::Waypoint] {
            let mut f = frame(1);
            f.units[0].target = Some(Vec2::new(2.1, 2.));
            let mut state = TrafficState::default();
            let mut budget = SweepBudget::new(1);
            let mut clear = |_, _, _| true;
            let mut guide = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut budget,
                &mut none,
                |_| true,
                |_| kind,
                |i| f.units[i].target.unwrap_or(f.units[i].position),
                |_| None,
                &mut clear,
            );
            let mut r = request(&f.units[0]);
            r.kind = kind;
            let next = guide.next_position(r, &mut budget);
            if kind == MotionKind::Destination {
                assert_eq!(
                    next, None,
                    "raw arrival and occupied-holder policy must run"
                );
                assert_eq!(budget.model_operations, 0);
            } else {
                assert_eq!(
                    next,
                    Some(r.goal),
                    "private waypoint must not overshoot or claim public arrival"
                );
                assert!(budget.model_operations > 0);
            }
            guide.finish_prediction(&mut budget);
            assert_eq!(budget.model_reserved, [0; SLOTS]);
        }
    }
    #[test]
    fn original_arrival_destination_survives_private_waypoint_changes() {
        let f = frame(1);
        let original = Vec2::new(20., 12.);
        let mut state = TrafficState::default();
        let mut budget = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut guide = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::Destination,
            |_| original,
            |_| None,
            &mut clear,
        );
        let unit = f.units[0].unit_id;
        assert_ne!(f.units[0].target, Some(original));
        assert_eq!(guide.arrival_destination(unit), Some(original));
        guide.mark_waypoint(unit);
        assert_eq!(guide.kind(unit), MotionKind::Waypoint);
        assert_eq!(guide.arrival_destination(unit), Some(original));
        guide.mark_waypoint(unit);
        assert_eq!(guide.arrival_destination(unit), Some(original));
        assert_eq!(guide.arrival_destination(u32::MAX), None);
    }

    #[test]
    fn changed_short_retry_goal_stays_guided_without_refilling_the_same_ticket() {
        let f = frame(1);
        let mut state = TrafficState::default();
        let mut budget = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut guide = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::Destination,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
        );
        let mut initial = request(&f.units[0]);
        initial.kind = MotionKind::Destination;
        assert!(guide.next_position(initial, &mut budget).is_some());
        guide.finish_prediction(&mut budget);
        let spent = guide.actors[0].spent;
        let operations = budget.model_operations;
        let mut retry = initial;
        retry.goal = Vec2::new(2.1, 2.);
        guide.mark_waypoint(retry.unit_id);
        retry.kind = guide.kind(retry.unit_id);
        assert_eq!(retry.kind, MotionKind::Waypoint);
        assert_eq!(
            guide.next_position(retry, &mut budget),
            Some(retry.goal),
            "a private retry prefix cannot re-enter legacy short-arrival steering"
        );
        assert_eq!(guide.actors[0].spent, spent);
        assert_eq!(budget.model_operations, operations);
        assert_eq!(budget.model_reserved, [0; SLOTS]);
        assert!(guide.actors[0].spent <= TICKET);
        guide.finish_prediction(&mut budget);
        assert_eq!(budget.model_reserved, [0; SLOTS]);
    }
    #[test]
    fn raw_claims_short_closest_holder_slot_keeps_destination_handling() {
        let f = frame(1);
        let mut state = TrafficState::default();
        let mut budget = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut guide = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::Destination,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
        );
        let mut selected = request(&f.units[0]);
        selected.kind = MotionKind::Destination;
        // Raw claim selection can choose this terminal closest-feasible slot
        // beside a holder while the captured player goal remains farther away.
        selected.goal = Vec2::new(2.1, 2.);
        let before = budget.model_operations;
        assert_eq!(guide.next_position(selected, &mut budget), None);
        assert_eq!(budget.model_operations, before);
        guide.finish_prediction(&mut budget);
        assert_eq!(budget.model_reserved, [0; SLOTS]);
    }
    fn coherent_frame(n: usize) -> MovementFrame {
        let mut f = frame(n);
        for (i, u) in f.units.iter_mut().enumerate() {
            // Adjacent4r cells, while outside the predictive discovery footprint.
            u.position = Vec2::new(2. + (i % 40) as f32 * 1.1, 2. + (i / 40) as f32 * 1.1);
            u.target = Some(u.position + Vec2::new(20., if i % 2 == 0 { 4. } else { -2. }));
        }
        f
    }
    #[test]
    fn complete_coherence_batch_and_cached_target_bound_advice() {
        let f = coherent_frame(2);
        let mut state = TrafficState::default();
        let mut budget = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut guide = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut budget,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| Some(7),
            &mut clear,
        );
        assert_eq!(budget.coherence_members, 2);
        assert_eq!(budget.coherence_attempts, 2);
        assert_eq!(budget.coherence_advice, 2);
        let a = guide.actors[0];
        let cached = a.advice.unwrap();
        let direct = request(&f.units[0]).preferred_position();
        assert_ne!(cached, direct);
        // Remove predictive admission to prove the cached completed fallback.
        guide.actors[0].primary = None;
        assert_eq!(
            guide.next_position(request(&f.units[0]), &mut budget),
            Some(cached)
        );
        let mut changed = request(&f.units[0]);
        changed.goal += Vec2::Y;
        assert_eq!(
            guide.next_position(changed, &mut budget),
            Some(changed.preferred_position())
        );
        guide.mark_waypoint(a.id);
        assert!(guide.actors[0].advice.is_none());
    }
    #[test]
    fn aggregate_requires_complete_owner_credit_and_does_not_reset_service_age() {
        let f = coherent_frame(2);
        for available in [1, 2] {
            let mut state = TrafficState::default();
            let mut b = SweepBudget::new(1);
            b.pairs[0] = OWNER_VISITS - available;
            let mut clear = |_, _, _| true;
            let g = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut b,
                &mut none,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap(),
                |_| Some(7),
                &mut clear,
            );
            assert_eq!(b.coherence_members, if available == 2 { 2 } else { 0 });
            assert_eq!(b.coherence_attempts, 0);
            g.record(&mut state, 1, 0.05);
            assert!(
                state.motion_history[0]
                    .iter()
                    .all(|r| r.attempt == u32::MAX)
            );
        }
    }
    #[test]
    fn coherence_start_32_and_prediction_remaining_64_boundaries() {
        let f = coherent_frame(2);
        for credit in [31, 32] {
            let mut state = TrafficState::default();
            let mut b = SweepBudget::new(1);
            b.pairs[0] = OWNER_VISITS - credit - 2; // full aggregate debit first
            let mut clear = |_, _, _| true;
            let mut g = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut b,
                &mut none,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap(),
                |_| Some(7),
                &mut clear,
            );
            assert_eq!(b.coherence_attempts, if credit == 32 { 1 } else { 0 });
            g.finish_prediction(&mut b);
            assert!(b.model_reserved.iter().all(|&n| n == 0));
        }
        for credit in [63, 64, 127, 128] {
            let f = frame(1);
            let mut state = TrafficState::default();
            let mut b = SweepBudget::new(1);
            b.pairs[0] = OWNER_VISITS - credit;
            let mut clear = |_, _, _| true;
            let mut g = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut b,
                &mut none,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap(),
                |_| None,
                &mut clear,
            );
            // A fresh actor reserves its entire remaining128, not a tiny share.
            assert_eq!(
                g.actors.iter().filter(|a| a.primary_started).count(),
                usize::from(credit == 128)
            );
            g.finish_prediction(&mut b);
        }
    }
    #[test]
    fn complete_static_proof_refuses_blocker_and_terrain_and_preserves_bound() {
        let f = coherent_frame(2);
        for terrain_clear in [false, true] {
            let mut state = TrafficState::default();
            let mut b = SweepBudget::new(1);
            let mut clear = |_, _, _| terrain_clear;
            let mut foreign = |_: u8, emit: &mut dyn FnMut(u32, ObservedBody) -> bool| {
                emit(
                    100,
                    ObservedBody {
                        position: Vec2::new(2.12, 2.02),
                        radius: 0.3,
                    },
                )
            };
            let g = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut b,
                &mut foreign,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap(),
                |_| Some(7),
                &mut clear,
            );
            assert!(g.actors[0].advice.is_none());
            assert!(g.actors.iter().all(|a| a.spent <= 128));
            assert!(b.pairs[0] <= OWNER_VISITS);
        }
    }
    #[test]
    fn all_contributors_pay_but_real_attempt_age_rotates_dense_cohort() {
        let mut f = coherent_frame(500);
        let mut state = TrafficState::default();
        let mut served = BTreeSet::new();
        for tick in 1..=6 {
            let mut b = SweepBudget::new(tick);
            // Enough for complete aggregation + a bounded physical prefix only.
            b.pairs[0] = OWNER_VISITS - 1500;
            let mut clear = |_, _, _| true;
            let mut g = Guide::capture(
                &f,
                0.05,
                tick,
                &mut state,
                &mut b,
                &mut none,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap(),
                |_| Some(7),
                &mut clear,
            );
            assert_eq!(b.coherence_members, 500);
            for a in &g.actors {
                if a.serviced {
                    served.insert(a.id);
                }
            }
            g.finish_prediction(&mut b);
            g.record(&mut state, tick, 0.05);
            drop(g);
            let out = outputs(&f, Vec2::X * 0.001);
            publish(&mut state, &f, &out, tick, 0.05);
            for (u, o) in f.units.iter_mut().zip(out) {
                u.position = o.position;
            }
        }
        assert_eq!(served.len(), 500);
    }
    #[test]
    fn predictive_full_remaining_actor_boundary_and_owner_credit_are_exact() {
        assert_eq!(predictive_allowance(65, 128), None); // only63 actor operations remain
        assert_eq!(predictive_allowance(64, 64), Some(64));
        assert_eq!(predictive_allowance(64, 63), None);
        assert_eq!(predictive_allowance(0, 64), None); // no tiny fresh share
        assert_eq!(predictive_allowance(0, 128), Some(128));
        let mut b = SweepBudget::new(1);
        b.pairs[0] = OWNER_VISITS - 64;
        assert!(b.reserve_model_amount(0, 64));
        assert_eq!(b.model_available(0), 0);
        b.release_model(0, 64);
        assert_eq!(b.model_available(0), 64);
    }
    #[test]
    fn no_discovery_credit_or_missing_view_never_refreshes_attempt_age() {
        let f = coherent_frame(2);
        for missing in [false, true] {
            let mut state = TrafficState::default();
            state.tick_owner_neighbors[0] = OWNER_VISITS - 2;
            let mut b = SweepBudget::new(1);
            let mut clear = |_, _, _| true;
            let mut foreign = |_: u8, _: &mut dyn FnMut(u32, ObservedBody) -> bool| !missing;
            let mut g = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut b,
                &mut foreign,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap(),
                |_| Some(7),
                &mut clear,
            );
            for u in &f.units {
                g.next_position(request(u), &mut b);
            }
            assert_eq!(b.coherence_attempts, 0);
            assert!(g.actors.iter().all(|a| !a.serviced));
            g.record(&mut state, 1, 0.05);
            assert!(
                state.motion_history[0]
                    .iter()
                    .all(|r| r.attempt == u32::MAX)
            );
        }
    }
    #[test]
    fn oversized_owner_keeps_private_and_actual_arrival_metadata_without_modeling() {
        let mut f = frame(OWNER_MEMBERS + 1);
        f.units[0].target = Some(f.units[0].position + Vec2::X * 0.1);
        f.units[1].target = Some(f.units[1].position + Vec2::X * 0.1);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |i| {
                if i == 0 {
                    MotionKind::Destination
                } else {
                    MotionKind::Waypoint
                }
            },
            |_| Vec2::new(800., 800.),
            |_| Some(7),
            &mut clear,
        );
        assert!(g.actors.is_empty());
        assert_eq!(g.fallback_ids.len(), f.units.len());
        let mut actual = request(&f.units[0]);
        actual.kind = g.kind(actual.unit_id);
        assert_eq!(g.next_position(actual, &mut b), None);
        let mut private = request(&f.units[1]);
        private.kind = g.kind(private.unit_id);
        assert_eq!(
            g.next_position(private, &mut b),
            Some(private.preferred_position())
        );
        for u in &f.units {
            assert_eq!(
                g.arrival_destination(u.unit_id),
                Some(Vec2::new(800., 800.))
            );
        }
        g.mark_waypoint(actual.unit_id);
        actual.kind = g.kind(actual.unit_id);
        assert_eq!(actual.kind, MotionKind::Waypoint);
        assert_eq!(
            g.next_position(actual, &mut b),
            Some(actual.preferred_position())
        );
        assert_eq!(
            g.arrival_destination(actual.unit_id),
            Some(Vec2::new(800., 800.))
        );
        assert_eq!(b.model_attempts, 0);
        assert_eq!(b.coherence_attempts, 0);
    }
    #[test]
    fn frozen_group_advice_is_physical_under_actor_and_group_renaming() {
        let base = coherent_frame(8);
        let mut results = Vec::new();
        for reverse in [false, true] {
            let mut f = base.clone();
            if reverse {
                for u in &mut f.units {
                    u.unit_id = 100 - u.unit_id;
                }
                f.units.reverse();
            }
            let mut state = TrafficState::default();
            let mut b = SweepBudget::new(1);
            let mut clear = |_, _, _| true;
            let mut g = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut b,
                &mut none,
                |_| true,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap(),
                |_| Some(if reverse { 901 } else { 7 }),
                &mut clear,
            );
            let mut rows = f
                .units
                .iter()
                .map(|u| {
                    let next = g.next_position(request(u), &mut b).unwrap();
                    (bits(u.position), bits(next))
                })
                .collect::<Vec<_>>();
            rows.sort_unstable();
            g.finish_prediction(&mut b);
            results.push((rows, b.pairs, b.coherence_advice));
        }
        assert_eq!(results[0], results[1]);
    }
    #[test]
    fn admitted_prediction_preserves_validated_non_straight_coherence_preference() {
        let mut f = coherent_frame(2);
        // Stay in the same local coherence neighborhood but beyond the full
        // preferred-step predictive tube, preserving this no-neighbor boundary.
        f.units[1].position.x += 0.1;
        f.units[1].target = f.units[1].target.map(|p| p + Vec2::X * 0.1);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| Some(7),
            &mut clear,
        );
        let cached = g.actors[0].advice.unwrap();
        assert_ne!(cached, request(&f.units[0]).preferred_position());
        assert!(g.actors[0].primary_started);
        assert_eq!(g.next_position(request(&f.units[0]), &mut b), Some(cached));
        assert_eq!(g.primary_probe.as_ref().unwrap().1.len, 0);
        assert_eq!(
            b.model_attempts,
            g.actors
                .iter()
                .filter(|a| a.primary_started && a.predictive_serviced)
                .count() as u64
        );
    }
    #[test]
    fn predictive_service_rotates_while_every_actor_receives_coherence() {
        let base = coherent_frame(8);
        let mut tapes = Vec::new();
        for inverted in [false, true] {
            let mut f = base.clone();
            if inverted {
                for u in &mut f.units {
                    u.unit_id = 100 - u.unit_id;
                }
                f.units.reverse();
            }
            let mut state = TrafficState::default();
            let mut served = BTreeSet::new();
            let mut tape = Vec::new();
            for tick in 1..=8 {
                let mut b = SweepBudget::new(tick);
                // All eight complete group proofs, then a small amount of LP work.
                b.pairs[0] = OWNER_VISITS - (12 * f.units.len() + 128);
                let mut clear = |_, _, _| true;
                let mut g = Guide::capture(
                    &f,
                    0.05,
                    tick,
                    &mut state,
                    &mut b,
                    &mut none,
                    |_| true,
                    |_| MotionKind::Waypoint,
                    |i| f.units[i].target.unwrap(),
                    |_| Some(if inverted { 901 } else { 7 }),
                    &mut clear,
                );
                assert!(g.actors.iter().all(|a| a.serviced && a.advice.is_some()));
                assert_eq!(b.coherence_attempts, 8);
                assert!((1..=3).contains(&g.actors.iter().filter(|a| a.primary_started).count()));
                for u in &f.units {
                    g.next_position(request(u), &mut b);
                }
                let mut recipients = g
                    .actors
                    .iter()
                    .filter(|a| a.predictive_serviced)
                    .map(|a| bits(a.position))
                    .collect::<Vec<_>>();
                assert!((1..=3).contains(&recipients.len()));
                recipients.sort_unstable(); // normalize physical set, not ID-sorted actor storage
                served.extend(recipients.iter().copied());
                tape.push(recipients);
                g.finish_prediction(&mut b);
                g.record(&mut state, tick, 0.05);
                drop(g);
                assert!(state.motion_history[0].iter().all(|r| r.attempt == tick));
                publish(&mut state, &f, &outputs(&f, Vec2::ZERO), tick, 0.05);
            }
            assert_eq!(served.len(), 8);
            tapes.push(tape);
        }
        assert_eq!(tapes[0], tapes[1]);
    }
    #[test]
    fn predictive_zero_spend_preserves_age_and_predictive_only_does_not_age_coherence() {
        let f = coherent_frame(2);
        let mut state = TrafficState::default();
        state.tick_owner_neighbors[0] = OWNER_VISITS;
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        assert!(g.actors.iter().all(|a| a.primary_started));
        for u in &f.units {
            g.next_position(request(u), &mut b);
        }
        assert!(
            g.actors
                .iter()
                .all(|a| a.spent == 0 && !a.predictive_serviced)
        );
        g.record(&mut state, 1, 0.05);
        g.finish_prediction(&mut b);
        drop(g);
        publish(&mut state, &f, &outputs(&f, Vec2::ZERO), 1, 0.05);
        assert!(
            state.motion_history[0]
                .iter()
                .all(|r| r.attempt == u32::MAX && r.predictive_attempt == u32::MAX)
        );
        state.tick_owner_neighbors = [0; SLOTS];
        let mut b = SweepBudget::new(2);
        let mut g = Guide::capture(
            &f,
            0.05,
            2,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        for u in &f.units {
            g.next_position(request(u), &mut b);
        }
        g.record(&mut state, 2, 0.05);
        g.finish_prediction(&mut b);
        drop(g);
        assert!(
            state.motion_history[0]
                .iter()
                .all(|r| r.attempt == u32::MAX && r.predictive_attempt == 2 && r.seen == 1)
        );
    }
    #[test]
    fn short_actual_arrivals_cannot_reserve_prediction_ahead_of_far_transit() {
        let mut f = frame(301);
        for u in &mut f.units[..300] {
            u.target = Some(u.position + Vec2::X * 0.1);
        }
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        b.pairs[0] = OWNER_VISITS - 128;
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Destination,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        assert_eq!(g.actors.len(), 301);
        assert_eq!(g.actors.iter().filter(|a| a.primary_started).count(), 1);
        assert!(
            g.actors
                .iter()
                .filter(|a| a.id != 301)
                .all(|a| !a.primary_started)
        );
        assert!(
            g.actors
                .iter()
                .find(|a| a.id == 301)
                .unwrap()
                .primary_started
        );
        let mut actual = request(&f.units[0]);
        actual.kind = MotionKind::Destination;
        assert_eq!(g.next_position(actual, &mut b), None);
        assert_eq!(g.arrival_destination(actual.unit_id), Some(actual.goal));
        let mut far = request(&f.units[300]);
        far.kind = MotionKind::Destination;
        assert!(g.next_position(far, &mut b).is_some());
        assert_eq!(b.model_attempts, 1);
        g.finish_prediction(&mut b);
        drop(g);
        // The same short row, explicitly private, still gets its whole ticket.
        let mut b = SweepBudget::new(2);
        let mut g = Guide::capture(
            &f,
            0.05,
            2,
            &mut state,
            &mut b,
            &mut none,
            |i| i == 0,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        assert_eq!(g.actors.iter().filter(|a| a.primary_started).count(), 1);
        assert_eq!(g.actors[0].id, 1);
        g.finish_prediction(&mut b);
    }
    #[test]
    fn empty_local_cells_are_charged_service_without_partial_advice_or_prediction_age() {
        let mut f = coherent_frame(2);
        f.units[1].position += Vec2::X * 100.;
        f.units[1].target = f.units[1].target.map(|p| p + Vec2::X * 100.);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        b.pairs[0] = OWNER_VISITS - 52; // discovery fits, but no complete primary ticket remains
        let mut clear = |_, _, _| true;
        let g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| Some(7),
            &mut clear,
        );
        assert!(
            g.actors
                .iter()
                .all(|a| a.spent == 10 && a.serviced && a.advice.is_none())
        );
        assert_eq!(b.coherence_members, 2);
        assert_eq!(b.coherence_attempts, 2);
        assert_eq!(b.coherence_operations, 20);
        assert_eq!(g.visits[0], 20);
        g.record(&mut state, 1, 0.05);
        assert!(
            state.motion_history[0]
                .iter()
                .all(|r| r.attempt == 1 && r.predictive_attempt == u32::MAX)
        );
    }
    #[test]
    fn refused_local_discovery_keeps_all_charges_and_never_caches_partial_mean() {
        let f = coherent_frame(2);
        let mut state = TrafficState::default();
        state.tick_owner_neighbors[0] = OWNER_VISITS - 5;
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| Some(7),
            &mut clear,
        );
        assert_eq!(b.coherence_members, 2);
        assert_eq!(b.coherence_operations, 5);
        assert_eq!(g.visits[0], OWNER_VISITS);
        assert!(g.actors.iter().all(|a| a.advice.is_none()));
        assert_eq!(g.actors.iter().filter(|a| a.serviced).count(), 1);
        let spent = g.actors.iter().map(|a| a.spent).collect::<Vec<_>>();
        g.finish_prediction(&mut b);
        g.mark_waypoint(f.units[0].unit_id);
        assert_eq!(g.actors.iter().map(|a| a.spent).collect::<Vec<_>>(), spent);
    }
    #[test]
    fn segment_filter_handles_zero_short_and_nonfinite_geometry() {
        assert_eq!(
            segment_distance_squared(DVec2::new(2., 3.), DVec2::ZERO, DVec2::ZERO),
            Some(13.)
        );
        assert_eq!(
            segment_distance_squared(DVec2::new(0.05, 0.1), DVec2::ZERO, DVec2::new(0.1, 0.)),
            Some(0.1f64 * 0.1)
        );
        assert_eq!(
            segment_distance_squared(DVec2::new(2., 0.), DVec2::ZERO, DVec2::X),
            Some(1.)
        );
        assert!(segment_distance_squared(DVec2::splat(f64::MAX), DVec2::ZERO, DVec2::X).is_none());
        assert!(
            segment_distance_squared(DVec2::ZERO, DVec2::ZERO, DVec2::splat(f64::NAN)).is_none()
        );
    }
    #[test]
    fn predictive_discovery_reaches_the_real_five_meter_endpoint_peer_and_keeps_guard() {
        let mut f = frame(2);
        f.units[0].position = Vec2::new(69.39347, 27.407396);
        f.units[0].target = Some(Vec2::new(59.98435, 28.068462));
        f.units[0].speed = Some(100.);
        f.units[1].position = Vec2::new(64.41373, 27.74317);
        f.units[1].target = Some(Vec2::new(64.01457, 26.166761));
        f.units[1].speed = Some(100.);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(17);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            17,
            &mut state,
            &mut b,
            &mut none,
            |i| i == 0,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        let mut request = request(&f.units[0]);
        request.speed = 100.;
        assert!(g.next_position(request, &mut b).is_some());
        let ticket = &g.primary_probe.as_ref().unwrap().1;
        assert_eq!(ticket.len, 1);
        assert_eq!(ticket.neighbors[0].position, f.units[1].position.as_dvec2());
        assert_ne!(ticket.neighbors[0].velocity, DVec2::ZERO);
        let out = crate::movement::step_frame_with_motion_guide(
            &f,
            0.05,
            &BTreeSet::new(),
            &mut b,
            &mut g,
        );
        assert!(
            turts_rules::terrain::geometry::swept_disks_clear(
                turts_rules::terrain::geometry::Point {
                    x: f64::from(f.units[0].position.x),
                    y: f64::from(f.units[0].position.y)
                },
                turts_rules::terrain::geometry::Point {
                    x: f64::from(out[0].position.x),
                    y: f64::from(out[0].position.y)
                },
                f64::from(f.units[0].radius),
                turts_rules::terrain::geometry::Point {
                    x: f64::from(f.units[1].position.x),
                    y: f64::from(f.units[1].position.y)
                },
                turts_rules::terrain::geometry::Point {
                    x: f64::from(out[1].position.x),
                    y: f64::from(out[1].position.y)
                },
                f64::from(f.units[1].radius),
                &mut turts_rules::terrain::geometry::Budget::new(1),
            )
            .unwrap()
        );
        assert!(g.actors.iter().all(|a| a.spent <= 128));
    }
    #[test]
    fn swept_discovery_excludes_off_segment_rows_inside_its_rectangle() {
        let mut f = frame(3);
        f.units[0].position = Vec2::splat(10.);
        f.units[0].target = Some(Vec2::splat(15.));
        f.units[0].speed = Some(200.);
        f.units[1].position = Vec2::splat(14.9);
        f.units[1].target = Some(Vec2::splat(30.));
        f.units[2].position = Vec2::new(10., 15.);
        f.units[2].target = None;
        f.units[2].speed = None;
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |i| i == 0,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
        );
        g.next_position(request(&f.units[0]), &mut b);
        let ticket = &g.primary_probe.as_ref().unwrap().1;
        assert!(ticket.discovered && !ticket.refused);
        assert_eq!(ticket.len, 1);
        assert_eq!(ticket.neighbors[0].position, f.units[1].position.as_dvec2());
    }
    #[test]
    fn incomplete_swept_discovery_never_solves_a_partial_subset_or_refills_retry() {
        let mut f = frame(40);
        for (i, u) in f.units.iter_mut().enumerate() {
            u.position = Vec2::new(10. + i as f32 * 0.1, 10.);
            u.target = Some(Vec2::new(30., 10.));
            u.speed = Some(100.);
        }
        let mut state = TrafficState::default();
        state.tick_owner_neighbors[0] = OWNER_VISITS - 4;
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |i| i == 0,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        let request = request(&f.units[0]);
        let next = g.next_position(request, &mut b);
        assert_eq!(next, Some(request.preferred_position()));
        assert_eq!(b.model_attempts, 0);
        assert!(g.primary_probe.as_ref().unwrap().1.refused);
        let spent = g.actors[0].spent;
        let visits = g.visits;
        g.finish_prediction(&mut b);
        g.next_position(request, &mut b);
        assert_eq!(g.actors[0].spent, spent);
        assert_eq!(g.visits, visits);
    }
    #[test]
    fn hard_mask_uses_stationary_witness_not_zero_velocity() {
        let mut f = frame(3);
        f.bounds = Vec2::splat(10.);
        f.units[0].position = Vec2::new(5., 8.);
        f.units[0].target = Some(Vec2::new(5., 9.7));
        f.units[0].speed = Some(40.);
        f.units[1].position = Vec2::new(5., 9.7);
        f.units[1].target = Some(Vec2::new(5., 12.));
        f.units[1].speed = Some(40.);
        f.units[2].position = Vec2::new(6., 9.);
        f.units[2].target = None;
        f.units[2].speed = None;
        let foreign_position = Vec2::new(4.2, 9.4);
        let mut foreign = |_: u8, emit: &mut dyn FnMut(u32, ObservedBody) -> bool| {
            emit(
                100,
                ObservedBody {
                    position: foreign_position,
                    radius: 0.3,
                },
            )
        };
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut foreign,
            |i| i == 0,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap_or(f.units[i].position),
            |_| None,
            &mut clear,
        );
        let moving_zero = g.views[0]
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .find(|r| r.unit == 2)
            .unwrap();
        assert_eq!(moving_zero.agent.velocity, DVec2::ZERO);
        assert!(!moving_zero.stationary);
        let mut req = request(&f.units[0]);
        req.bounds = f.bounds;
        g.next_position(req, &mut b);
        let t = &g.primary_probe.as_ref().unwrap().1;
        assert_eq!(t.len, 3);
        for (i, n) in t.neighbors[..t.len].iter().enumerate() {
            assert_eq!(
                t.hard_mask & (1 << i) != 0,
                n.position != f.units[1].position.as_dvec2()
            );
        }
        let frozen = (
            t.neighbors[..t.len]
                .iter()
                .map(|a| (a.position, a.velocity))
                .collect::<Vec<_>>(),
            t.hard_mask,
        );
        let spent = t.spent;
        g.finish_prediction(&mut b);
        g.mark_waypoint(req.unit_id);
        req.goal = Vec2::new(5., 8.5);
        g.next_position(req, &mut b);
        let t = &g.primary_probe.as_ref().unwrap().1;
        assert_eq!(
            (
                t.neighbors[..t.len]
                    .iter()
                    .map(|a| (a.position, a.velocity))
                    .collect::<Vec<_>>(),
                t.hard_mask
            ),
            frozen
        );
        assert!(t.spent >= spent && t.spent <= 128);
    }
    #[test]
    fn top_eight_hardness_alignment_and_boundary_ignore_handles_and_enumeration() {
        let mut base = frame(10);
        base.bounds = Vec2::splat(10.);
        base.units[0].position = Vec2::new(8., 5.);
        base.units[0].target = Some(Vec2::new(9.7, 5.));
        base.units[0].speed = Some(40.);
        for (i, u) in base.units.iter_mut().enumerate().skip(1) {
            u.position = Vec2::new(9.7, 5.);
            u.target = (i <= 4).then_some(Vec2::new(12., 5.));
            u.speed = (i <= 4).then_some(40.);
        }
        let mut results = Vec::new();
        for reverse in [false, true] {
            let mut f = base.clone();
            if reverse {
                for u in &mut f.units {
                    u.unit_id = 100 - u.unit_id;
                }
                f.units.reverse();
            }
            let actor = f.units.iter().find(|u| u.position.x == 8.).unwrap();
            let mut state = TrafficState::default();
            let mut b = SweepBudget::new(1);
            let mut clear = |_, _, _| true;
            let mut g = Guide::capture(
                &f,
                0.05,
                1,
                &mut state,
                &mut b,
                &mut none,
                |i| f.units[i].position.x == 8.,
                |_| MotionKind::Waypoint,
                |i| f.units[i].target.unwrap_or(f.units[i].position),
                |_| None,
                &mut clear,
            );
            let mut req = request(actor);
            req.bounds = f.bounds;
            g.next_position(req, &mut b);
            let t = &g.primary_probe.as_ref().unwrap().1;
            assert_eq!(t.len, 8);
            assert_eq!(t.hard_mask, 0b11110000);
            assert!(t.neighbors[..8].iter().all(|a| a.velocity == DVec2::ZERO));
            results.push((t.hard_mask, t.spent, b.model_operations, g.visits));
        }
        assert_eq!(results[0], results[1]);
    }
    #[test]
    fn eager_primary_recycles_real_unused_work_before_admitting_next_actor() {
        let f = frame(100);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        b.pairs[0] = OWNER_VISITS - 256;
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        let served = g.actors.iter().filter(|a| a.predictive_serviced).count();
        assert!(
            served >= 16,
            "whole tickets held concurrently previously admit only2; got {served}"
        );
        assert!(g.actors.iter().all(|a| a.spent <= 128));
        assert_eq!(b.model_reserved, [0; SLOTS]);
        let before = (b.model_operations, b.pairs, g.visits);
        for u in &f.units {
            g.next_position(request(u), &mut b);
        }
        assert_eq!((b.model_operations, b.pairs, g.visits), before);
        assert!(b.pairs[0] <= OWNER_VISITS);
    }
    #[test]
    fn complete_physical_tie_requires_whole_operation_and_visit_protection() {
        for (operations, visits, admit) in [(255, 1000, false), (256, 257, false), (256, 258, true)]
        {
            let mut results = Vec::new();
            for reverse in [false, true] {
                let mut f = frame(2);
                f.units[1].position = f.units[0].position;
                f.units[1].target = f.units[0].target;
                if reverse {
                    for u in &mut f.units {
                        u.unit_id = 100 - u.unit_id;
                    }
                    f.units.reverse();
                }
                let mut state = TrafficState::default();
                state.tick_owner_neighbors[0] = OWNER_VISITS - visits;
                let mut b = SweepBudget::new(1);
                b.pairs[0] = OWNER_VISITS - operations;
                let mut clear = |_, _, _| true;
                let g = Guide::capture(
                    &f,
                    0.05,
                    1,
                    &mut state,
                    &mut b,
                    &mut none,
                    |_| true,
                    |_| MotionKind::Waypoint,
                    |i| f.units[i].target.unwrap(),
                    |_| None,
                    &mut clear,
                );
                assert!(g.actors.iter().all(|a| a.primary_started == admit));
                assert!(g.actors.iter().all(|a| a.predictive_serviced == admit));
                assert_eq!(g.actors[0].spent, g.actors[1].spent);
                assert_eq!(b.model_reserved, [0; SLOTS]);
                if !admit {
                    assert_eq!(b.model_operations, 0);
                    assert_eq!(g.visits[0], OWNER_VISITS - visits);
                } else {
                    assert!(g.visits[0] < OWNER_VISITS);
                }
                results.push((g.actors[0].spent, b.model_operations, g.visits));
            }
            assert_eq!(results[0], results[1]);
        }
    }
    #[test]
    fn cached_primary_binds_every_motion_input_and_retries_never_spend() {
        let mut f = coherent_frame(2);
        f.units[1].position.x += 0.1;
        f.units[1].target = f.units[1].target.map(|p| p + Vec2::X * 0.1);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| Some(7),
            &mut clear,
        );
        let original = request(&f.units[0]);
        let cached = g.actors[0].primary.unwrap();
        assert_ne!(cached, original.preferred_position());
        assert_eq!(g.next_position(original, &mut b), Some(cached));
        let before = (b.model_operations, b.pairs, g.visits, g.actors[0].spent);
        for change in 0..7 {
            let mut altered = original;
            match change {
                0 => altered.goal += Vec2::Y,
                1 => altered.position += Vec2::X * 0.01,
                2 => altered.speed *= 0.5,
                3 => altered.radius += 0.01,
                4 => altered.dt *= 0.5,
                5 => altered.bounds += Vec2::X,
                _ => altered.kind = MotionKind::Destination,
            }
            assert_eq!(
                g.next_position(altered, &mut b),
                Some(altered.preferred_position())
            );
            assert_eq!(
                (b.model_operations, b.pairs, g.visits, g.actors[0].spent),
                before
            );
        }
        g.mark_waypoint(original.unit_id);
        assert!(g.actors[0].primary.is_none() && g.actors[0].advice.is_none());
        assert_eq!(
            g.next_position(original, &mut b),
            Some(original.preferred_position())
        );
        assert_eq!(
            (b.model_operations, b.pairs, g.visits, g.actors[0].spent),
            before
        );
    }
    #[test]
    fn valid_zero_displacement_is_a_completed_cached_primary() {
        let mut f = frame(1);
        f.bounds = Vec2::splat(10.);
        f.units[0].position = Vec2::new(9.7, 5.);
        f.units[0].target = Some(Vec2::new(12., 5.));
        f.units[0].speed = Some(40.);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        assert_eq!(g.actors[0].primary, Some(f.units[0].position));
        assert!(g.actors[0].predictive_serviced);
        let mut req = request(&f.units[0]);
        req.bounds = f.bounds;
        assert_eq!(g.next_position(req, &mut b), Some(f.units[0].position));
    }
    #[test]
    fn refined_negative_goal_primary_reaches_proof_without_duplicate_bias_solves() {
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

        let f = frame(1);
        let mut state = TrafficState::default();
        let mut setup = SweepBudget::new(1);
        let proof_calls = std::cell::Cell::new(0usize);
        let mut clear = |_, _, _| {
            proof_calls.set(proof_calls.get() + 1);
            true
        };
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut setup,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
        );
        let dt = f64::from(0.05f32);
        g.actors[0].position = actor.position.as_vec2();
        g.actors[0].radius = actor.radius as f32;
        g.actors[0].speed = 100.;
        g.actors[0].goal = (actor.position + actor.velocity * dt).as_vec2();
        g.actors[0].advice = Some((actor.position + preferred * dt).as_vec2());
        let mut rows = Vec::new();
        rows.push(Row {
            unit: g.actors[0].id,
            agent: actor,
            stationary: false,
        });
        for (i, agent) in neighbors.into_iter().enumerate() {
            rows.push(Row {
                unit: 100 + i as u32,
                agent,
                stationary: false,
            });
        }
        rows.sort_unstable_by(|a, b| {
            let cell = |p: DVec2| (p.x.floor() as i32, p.y.floor() as i32);
            cell(a.agent.position)
                .cmp(&cell(b.agent.position))
                .then_with(|| agent_cmp(a.agent, b.agent))
        });
        g.views[0] = Some(View {
            rows,
            statics: Vec::new(),
            width: 1.,
            max_radius: actor.radius,
        });
        g.visits = [0; SLOTS];
        let mut b = SweepBudget::new(1);
        b.pairs[0] = 24;
        b.model_operations = 24;
        assert!(b.reserve_model_amount(0, 104));
        let mut ticket = Ticket {
            spent: 24,
            reserved: 104,
            neighbors: [EMPTY; 8],
            hard_mask: 0,
            velocity: DVec2::ZERO,
            len: 0,
            discovered: false,
            refused: false,
        };
        let req = g.actors[0].request(f.bounds, 0.05);
        proof_calls.set(0);
        let next = g
            .predict_primary(0, req, &mut ticket, &mut b, None)
            .expect("refinement must reach current proof within128");
        assert!((next.as_dvec2() - actor.position).dot(preferred) < 0.);
        assert_eq!(proof_calls.get(), 1);
        assert!(ticket.spent < 128);
        b.release_model(0, ticket.reserved);
        assert_eq!(b.model_reserved, [0; SLOTS]);
    }
    #[test]
    fn joint_baseline_fallback_precedes_coherence_and_binds_every_motion_input() {
        let f = coherent_frame(2);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| Some(7),
            &mut clear,
        );
        let original = request(&f.units[0]);
        g.actors[0].primary = None;
        g.actors[0].baseline = Some(original.position);
        assert!(g.actors[0].advice.is_some());
        assert_eq!(g.next_position(original, &mut b), Some(original.position));
        let before = (g.actors[0].spent, b.model_operations, b.pairs, g.visits);
        for change in 0..7 {
            let mut req = original;
            match change {
                0 => req.goal += Vec2::Y,
                1 => req.position += Vec2::X * 0.01,
                2 => req.speed *= 0.5,
                3 => req.radius += 0.01,
                4 => req.dt *= 0.5,
                5 => req.bounds += Vec2::X,
                _ => req.kind = MotionKind::Destination,
            }
            assert_eq!(g.next_position(req, &mut b), Some(req.preferred_position()));
            assert_eq!(
                (g.actors[0].spent, b.model_operations, b.pairs, g.visits),
                before
            );
        }
        g.mark_waypoint(original.unit_id);
        assert!(g.actors[0].baseline.is_none());
        assert_eq!(
            g.next_position(original, &mut b),
            Some(original.preferred_position())
        );
    }
    #[test]
    fn public_joint_admission_base_ignores_other_owner_private_counts() {
        eprintln!(
            "joint public base={JOINT_BASE_BYTES} remaining={}",
            SCRATCH_BYTES.saturating_sub(JOINT_BASE_BYTES)
        );
        const { assert!(JOINT_BASE_BYTES < SCRATCH_BYTES) };
        let a = frame(1);
        let mut b = frame(100);
        for u in &mut b.units[1..] {
            u.owner_slot = 1;
            u.position += Vec2::splat(400.);
        }
        let mut values = Vec::new();
        for frame in [&a, &b] {
            let mut state = TrafficState::default();
            let mut budget = SweepBudget::new(1);
            let mut clear = |_, _, _| true;
            let g = Guide::capture_inner(
                frame,
                0.05,
                1,
                &mut state,
                &mut budget,
                &mut none,
                |i| frame.units[i].owner_slot == 0,
                |_| MotionKind::Waypoint,
                |i| frame.units[i].target.unwrap(),
                |_| None,
                &mut clear,
                true,
            );
            let actor = &g.actors[0];
            assert!(
                actor.baseline.is_some(),
                "privacy comparison must exercise admitted planning"
            );
            values.push((
                actor.baseline,
                actor.primary,
                actor.spent,
                budget.pairs[0],
                g.visits[0],
            ));
        }
        assert_eq!(values[0], values[1]);
    }
    #[test]
    fn thousand_captured_rows_fit_the_public_joint_phase_reservation() {
        let f = frame(1000);
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        eprintln!(
            "joint public base={JOINT_BASE_BYTES} remaining={} core1000={:?}",
            SCRATCH_BYTES.saturating_sub(JOINT_BASE_BYTES),
            crate::movement::sweep::planning_scratch_bound(1000, 0)
        );
        let g = Guide::capture_inner(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut none,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
            true,
        );
        assert_eq!(
            g.actors.iter().filter(|a| a.baseline.is_some()).count(),
            1000,
            "public-bound refusal must not silently disable the intended1000-row plan"
        );
        assert!(state.retained_bytes() + state.scratch_bytes() <= SCRATCH_BYTES);
        assert!(g.actors.iter().all(|a| a.spent <= 128));
        assert!(b.pairs[0] <= OWNER_VISITS);
    }
    #[test]
    fn known_held_peer_baseline_replaces_command_forecast_before_primary_upgrade() {
        let mut f = frame(2);
        f.units[0].position = Vec2::new(10., 10.);
        f.units[0].target = Some(Vec2::new(30., 10.));
        f.units[0].speed = Some(100.);
        f.units[1].position = Vec2::new(10.7, 10.);
        f.units[1].target = Some(Vec2::new(100., 10.));
        f.units[1].speed = Some(100.);
        let mut foreign = |owner: u8, emit: &mut dyn FnMut(u32, ObservedBody) -> bool| {
            owner != 0
                || emit(
                    100,
                    ObservedBody {
                        position: Vec2::new(11.4, 10.),
                        radius: 0.3,
                    },
                )
        };
        let mut state = TrafficState::default();
        let mut b = SweepBudget::new(1);
        let mut clear = |_, _, _| true;
        let mut g = Guide::capture_inner(
            &f,
            0.05,
            1,
            &mut state,
            &mut b,
            &mut foreign,
            |_| true,
            |_| MotionKind::Waypoint,
            |i| f.units[i].target.unwrap(),
            |_| None,
            &mut clear,
            true,
        );
        assert!(g.actors.iter().all(|a| a.baseline == Some(a.position)));
        let own = g.views[0]
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .filter(|r| r.unit != 100)
            .collect::<Vec<_>>();
        assert_eq!(own.len(), 2);
        assert!(own.iter().all(|r| r.agent.velocity == DVec2::ZERO));
        let before = (b.model_operations, b.pairs, g.visits);
        for u in &f.units {
            let next = g.next_position(request(u), &mut b).unwrap();
            assert!(next.x >= u.position.x && next.x <= u.position.x + 0.10001);
        }
        assert_eq!((b.model_operations, b.pairs, g.visits), before);
        assert!(g.actors.iter().all(|a| a.spent <= 128));
        assert_eq!(b.model_reserved, [0; SLOTS]);
    }
}
