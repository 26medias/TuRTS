//! Owner-local plans built from owned state and the complete captured view.
//! No authoritative guard result or foreign command enters this module.
use super::*;
use crate::movement::{MovementFrame, MovementFrameUnit, step_frame_with_motion_guide};
use std::cell::RefCell;
use std::mem::size_of;

#[derive(Clone, Copy)]
struct Body {
    id: u32,
    from: Vec2,
    to: Vec2,
    radius: f32,
    travel: f64,
    limit: f64,
    owned: bool,
    upgradeable: bool,
}
impl Body {
    fn envelope(self) -> Option<[f64; 4]> {
        if !self.from.is_finite()
            || !self.radius.is_finite()
            || self.radius <= 0.
            || !self.travel.is_finite()
            || self.travel < 0.
        {
            return None;
        }
        let reach = (self.travel + f64::from(self.radius)).next_up();
        Some([
            (f64::from(self.from.x) - reach).next_down(),
            (f64::from(self.from.x) + reach).next_up(),
            (f64::from(self.from.y) - reach).next_down(),
            (f64::from(self.from.y) + reach).next_up(),
        ])
    }
    fn chord(self, to: Vec2) -> [f64; 4] {
        let r = f64::from(self.radius);
        [
            (f64::from(self.from.x.min(to.x)) - r).next_down(),
            (f64::from(self.from.x.max(to.x)) + r).next_up(),
            (f64::from(self.from.y.min(to.y)) - r).next_down(),
            (f64::from(self.from.y.max(to.y)) + r).next_up(),
        ]
    }
}
fn ulp(value: f32) -> f64 {
    let value = value.abs();
    if value.next_up().is_finite() {
        f64::from(value.next_up()) - f64::from(value)
    } else {
        f64::from(value) - f64::from(value.next_down())
    }
}
fn rounding(from: Vec2, to: Vec2) -> f64 {
    4. * (ulp(from.x) + ulp(from.y) + ulp(to.x) + ulp(to.y))
}
fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[1] && b[0] <= a[1] && a[2] <= b[3] && b[2] <= a[3]
}
fn physical_body(a: &Body, b: &Body) -> Ordering {
    point_cmp(a.from, b.from)
        .then(a.radius.total_cmp(&b.radius))
        .then_with(|| point_cmp(a.to, b.to))
        .then(a.travel.total_cmp(&b.travel))
}

pub(super) struct JointPlan {
    bodies: Vec<Body>,
    index: EnvelopeIndex,
    bounds: Vec2,
    peak: usize,
}
impl JointPlan {
    pub(super) fn bytes(&self) -> usize {
        size_of::<Self>() + self.bodies.capacity() * size_of::<Body>() + self.index.bytes()
    }
    pub(super) fn peak_bytes(&self) -> usize {
        self.peak
    }
    pub(super) fn endpoint(&self, id: u32) -> Option<Vec2> {
        self.bodies
            .iter()
            .find(|b| b.id == id && b.owned)
            .map(|b| b.to)
    }
    pub(super) fn metadata(&self) -> MetadataWork {
        self.index.work
    }

    /// Admission, every extracted row, and every predicate use the same ticket.
    /// No endpoint changes until the entire current plan has passed.
    pub(super) fn try_upgrade(
        &mut self,
        id: u32,
        to: Vec2,
        visits: &mut usize,
        charge: &mut dyn FnMut() -> bool,
    ) -> bool {
        let Some(actor) = self.bodies.iter().position(|b| b.id == id && b.upgradeable) else {
            return false;
        };
        let body = self.bodies[actor];
        if !to.is_finite()
            || to.x < body.radius
            || to.y < body.radius
            || to.x > (self.bounds.x - body.radius).max(body.radius)
            || to.y > (self.bounds.y - body.radius).max(body.radius)
            || body.from.as_dvec2().distance(to.as_dvec2()) > body.limit + rounding(body.from, to)
            || *visits >= OWNER_VISITS
            || !charge()
        {
            return false;
        }
        *visits += 1;
        let bounds = body.chord(to);
        let Some(rows) = self.index.query(bounds, None) else {
            return false;
        };
        let mut candidates = [0usize; NEIGHBORS];
        let mut count = 0;
        for row in rows {
            if *visits >= OWNER_VISITS || !charge() {
                return false;
            }
            *visits += 1;
            if row == actor || !overlaps(bounds, self.bodies[row].chord(self.bodies[row].to)) {
                continue;
            }
            if count == NEIGHBORS {
                return false;
            }
            candidates[count] = row;
            count += 1;
        }
        candidates[..count]
            .sort_unstable_by(|&a, &b| physical_body(&self.bodies[a], &self.bodies[b]));
        for &other in &candidates[..count] {
            if !charge() {
                return false;
            }
            let peer = self.bodies[other];
            if !crate::movement::sweep::pair_clear(
                body.from,
                to,
                body.radius,
                peer.from,
                peer.to,
                peer.radius,
            ) {
                return false;
            }
        }
        self.bodies[actor].to = to;
        true
    }
}

struct Seed {
    id: u32,
    kind: MotionKind,
    destination: Option<Vec2>,
    preferred: Option<Vec2>,
    request: Option<MotionRequest>,
    spent: usize,
}
fn same_request(a: MotionRequest, b: MotionRequest) -> bool {
    a.unit_id == b.unit_id
        && a.kind == b.kind
        && bits(a.position) == bits(b.position)
        && bits(a.goal) == bits(b.goal)
        && a.speed.to_bits() == b.speed.to_bits()
        && a.radius.to_bits() == b.radius.to_bits()
        && a.dt.to_bits() == b.dt.to_bits()
        && bits(a.bounds) == bits(b.bounds)
}
struct PlanningGuide {
    seeds: RefCell<Vec<Seed>>,
    owner: usize,
}
impl PlanningGuide {
    fn charge(&self, id: u32, owner: u8, budget: &mut SweepBudget) -> bool {
        if usize::from(owner) != self.owner {
            return false;
        }
        let mut seeds = self.seeds.borrow_mut();
        let Ok(i) = seeds.binary_search_by_key(&id, |s| s.id) else {
            return false;
        };
        let seed = &mut seeds[i];
        if seed.spent >= TICKET || !budget.can_charge_pair(owner, seed.spent, TICKET) {
            return false;
        }
        if !budget.charge_pair(owner, &mut seed.spent, TICKET) {
            return false;
        }
        budget.model_operations += 1;
        true
    }
}
impl MotionGuide for PlanningGuide {
    fn kind(&self, id: u32) -> MotionKind {
        self.seeds
            .borrow()
            .binary_search_by_key(&id, |s| s.id)
            .ok()
            .map_or(MotionKind::Destination, |i| self.seeds.borrow()[i].kind)
    }
    fn arrival_destination(&self, id: u32) -> Option<Vec2> {
        let seeds = self.seeds.borrow();
        seeds
            .binary_search_by_key(&id, |s| s.id)
            .ok()
            .and_then(|i| seeds[i].destination)
    }
    fn next_position(&mut self, request: MotionRequest, _: &mut SweepBudget) -> Option<Vec2> {
        let seeds = self.seeds.borrow();
        let i = seeds
            .binary_search_by_key(&request.unit_id, |s| s.id)
            .ok()?;
        // The real kernel retains actual short/closest-holder arrival policy.
        let distance = request
            .position
            .as_dvec2()
            .distance(request.goal.as_dvec2());
        let travel = f64::from(request.speed) * f64::from(request.dt);
        if distance.is_finite()
            && travel.is_finite()
            && seeds[i].kind == MotionKind::Destination
            && request.kind == MotionKind::Destination
            && request
                .position
                .as_dvec2()
                .distance(request.goal.as_dvec2())
                <= f64::from(request.speed) * f64::from(request.dt)
        {
            return None;
        }
        let seed = &seeds[i];
        seed.request?;
        let matched = seed
            .request
            .is_some_and(|captured| same_request(captured, request));
        Some(if matched {
            seed.preferred
                .unwrap_or_else(|| request.preferred_position())
        } else {
            request.preferred_position()
        })
    }
    fn charge_certificate_visit(
        &self,
        id: u32,
        owner: u8,
        _: bool,
        budget: &mut SweepBudget,
    ) -> bool {
        if usize::from(owner) != self.owner
            || budget.bodies[self.owner] >= OWNER_VISITS
            || !self.charge(id, owner, budget)
        {
            return false;
        }
        budget.bodies[self.owner] += 1;
        true
    }
    fn examine_certificate_row(&self, id: u32, owner: u8, budget: &mut SweepBudget) -> bool {
        self.charge_certificate_visit(id, owner, false, budget)
    }
    fn admit_certificate_row(&self, _: u32, _: u8, _: bool, _: &mut SweepBudget) -> bool {
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
        if *local >= allowance || !self.charge(id, owner, budget) {
            return false;
        }
        *local += 1;
        true
    }
    fn deferred_support_enabled(&self) -> bool {
        #[cfg(feature = "test-probes")]
        {
            std::env::var("TURTS_TEST_JOINT_SUPPORT_BASELINE").is_ok_and(|v| v == "1")
        }
        #[cfg(not(feature = "test-probes"))]
        {
            false
        }
    }
    fn certificate_search_enabled(&self) -> bool {
        false
    }
}

/// Sort and admit indistinguishable proof batches before any callback.
/// A failed whole plan must not distribute remaining tickets by handle order.
#[allow(
    clippy::too_many_arguments,
    reason = "Keep physical proof admission and its existing accounting ports explicit"
)]
fn terrain_proofs(
    units: &mut [MovementFrameUnit],
    outputs: &[MovementFrameOutput],
    original: &MovementFrame,
    guide: &PlanningGuide,
    budget: &mut SweepBudget,
    work: &mut MetadataWork,
    clear: &mut dyn FnMut(usize, Vec2, Vec2) -> bool,
) -> bool {
    let endpoint = |id| {
        outputs
            .binary_search_by_key(&id, |o| o.unit_id)
            .ok()
            .map(|i| outputs[i].position)
    };
    let physical = |a: &MovementFrameUnit, b: &MovementFrameUnit| {
        point_cmp(a.position, b.position)
            .then(a.radius.total_cmp(&b.radius))
            .then(a.speed.unwrap_or(0.).total_cmp(&b.speed.unwrap_or(0.)))
            .then_with(|| {
                point_cmp(
                    endpoint(a.unit_id).unwrap_or(a.position),
                    endpoint(b.unit_id).unwrap_or(b.position),
                )
            })
            .then_with(|| {
                point_cmp(
                    a.target.unwrap_or(a.position),
                    b.target.unwrap_or(b.position),
                )
            })
            .then(
                matches!(guide.kind(a.unit_id), MotionKind::Waypoint)
                    .cmp(&matches!(guide.kind(b.unit_id), MotionKind::Waypoint)),
            )
            .then(a.unit_type.cmp(&b.unit_type))
    };
    units.sort_unstable_by(|a, b| {
        work.build_comparisons = work.build_comparisons.saturating_add(1);
        physical(a, b)
    });
    let mut cursor = 0;
    while cursor < units.len() {
        let Some(to) = endpoint(units[cursor].unit_id) else {
            return false;
        };
        if to == units[cursor].position {
            cursor += 1;
            continue;
        }
        let mut end = cursor + 1;
        while end < units.len() {
            work.build_comparisons = work.build_comparisons.saturating_add(1);
            if physical(&units[cursor], &units[end]) != Ordering::Equal {
                break;
            }
            end += 1;
        }
        let batch = &units[cursor..end];
        // Every row gets one proof admission, or no row in this physical tie does.
        if batch.len() > (VISITS / SLOTS).saturating_sub(budget.pairs[guide.owner])
            || !batch.iter().all(|u| {
                let seeds = guide.seeds.borrow();
                seeds
                    .binary_search_by_key(&u.unit_id, |s| s.id)
                    .ok()
                    .is_some_and(|i| {
                        budget.can_charge_pair(guide.owner as u8, seeds[i].spent, TICKET)
                    })
            })
        {
            return false;
        }
        for unit in batch {
            if !guide.charge(unit.unit_id, guide.owner as u8, budget) {
                return false;
            }
        }
        for unit in batch {
            let Some(to) = endpoint(unit.unit_id) else {
                return false;
            };
            let Some(index) = original.units.iter().position(|u| {
                u.unit_id == unit.unit_id && usize::from(u.owner_slot) == guide.owner
            }) else {
                return false;
            };
            if !clear(index, unit.position, to) {
                return false;
            }
        }
        cursor = end;
    }
    true
}

/// Includes the nested allocations retained by an owned plain-move row.
fn row_bytes(unit: &MovementFrameUnit) -> usize {
    size_of::<MovementFrameUnit>()
        + unit
            .plain_progress
            .as_ref()
            .map_or(0, |p| p.pool_slots.len() * size_of::<Vec2>())
}
impl Guide<'_> {
    pub(super) fn prepare_joint(
        &mut self,
        owner: usize,
        frame: &MovementFrame,
        budget: &mut SweepBudget,
        base_bytes: usize,
    ) -> Option<JointPlan> {
        if owner >= SLOTS || self.dt <= 0. || !self.dt.is_finite() {
            return None;
        }
        let view = self.views[owner].as_ref()?;
        let owned = frame
            .units
            .iter()
            .filter(|u| usize::from(u.owner_slot) == owner)
            .count();
        let n = view.rows.len();
        if n > INDEX_BODIES || owned > OWNER_MEMBERS || owned == 0 {
            return None;
        }
        // Only referenced owned pools are retained, never foreign pool sizes.
        let uses_pool = |id| {
            frame.units.iter().any(|u| {
                usize::from(u.owner_slot) == owner
                    && u.progress.as_ref().is_some_and(|p| p.pool_id == id)
            })
        };
        let slots = frame
            .pools
            .iter()
            .filter(|(id, _)| uses_pool(**id))
            .try_fold(0usize, |n, (_, p)| n.checked_add(p.slots.len()))?
            .checked_add(
                frame
                    .units
                    .iter()
                    .filter(|u| usize::from(u.owner_slot) == owner)
                    .try_fold(0usize, |n, u| {
                        n.checked_add(u.plain_progress.as_ref().map_or(0, |p| p.pool_slots.len()))
                    })?,
            )?;
        let pool_count = frame.pools.keys().filter(|&&id| uses_pool(id)).count();
        let frame_bytes = size_of::<MovementFrame>() + n * size_of::<MovementFrameUnit>()
            + frame.units.iter().filter(|u| usize::from(u.owner_slot) == owner)
                .map(|u| row_bytes(u) - size_of::<MovementFrameUnit>()).sum::<usize>()
            + slots * size_of::<Vec2>()
            // Conservative BTree node/header allowance for every selected pool.
            + pool_count * (size_of::<crate::movement::MovementPool>() + 128);
        let scratch = crate::movement::sweep::planning_scratch_bound(n, slots)?;
        let seeds_bytes = size_of::<PlanningGuide>() + owned * size_of::<Seed>();
        let plan_bytes =
            size_of::<JointPlan>() + n * size_of::<Body>() + EnvelopeIndex::capacity_bound(n);
        // Root's conservative core bound already includes the copied proxy
        // Unit Vec. Do not reserve that same live allocation twice. Selected
        // pool containers/nested slots remain separately conservative here.
        let frame_extra = frame_bytes - n * size_of::<MovementFrameUnit>();
        let peak = (frame_extra + seeds_bytes + scratch)
            .max(
                frame_bytes
                    + seeds_bytes
                    + n * (size_of::<MovementFrameOutput>() + size_of::<Body>()),
            )
            .max(plan_bytes);
        if base_bytes.checked_add(peak)? > SCRATCH_BYTES {
            return None;
        }
        self.joint_peak = self.joint_peak.max(peak);
        let mut local = MovementFrame {
            recovery: Default::default(),
            bounds: frame.bounds,
            tick_hz: frame.tick_hz,
            units: Vec::with_capacity(n),
            pools: Default::default(),
        };
        let mut seeds = Vec::with_capacity(owned);
        for unit in frame
            .units
            .iter()
            .filter(|u| usize::from(u.owner_slot) == owner)
        {
            let row = view.rows.iter().find(|r| r.unit == unit.unit_id)?;
            if row.agent.position != unit.position.as_dvec2()
                || row.agent.radius != f64::from(unit.radius)
            {
                return None;
            }
            let actor = self
                .actors
                .iter()
                .find(|a| a.id == unit.unit_id && usize::from(a.owner) == owner);
            seeds.push(Seed {
                id: unit.unit_id,
                kind: actor.map_or(MotionKind::Destination, |a| a.kind),
                destination: actor.map(|a| a.destination),
                preferred: actor.map(|a| {
                    a.advice
                        .unwrap_or_else(|| a.request(self.bounds, self.dt).preferred_position())
                }),
                request: actor.map(|a| a.request(self.bounds, self.dt)),
                spent: actor.map_or(0, |a| a.spent),
            });
            let mut copy = unit.clone();
            if let Some(p) = &mut copy.plain_progress {
                // clone() may carry spare capacity; retain precisely the counted rows.
                let mut exact = Vec::with_capacity(p.pool_slots.len());
                exact.extend_from_slice(&p.pool_slots);
                p.pool_slots = exact;
            }
            local.units.push(copy);
        }
        for row in &view.rows {
            if local.units.iter().any(|u| u.unit_id == row.unit) {
                continue;
            }
            let position = row.agent.position.as_vec2();
            let radius = row.agent.radius as f32;
            if position.as_dvec2() != row.agent.position
                || f64::from(radius) != row.agent.radius
                || !row.stationary
            {
                return None;
            }
            local.units.push(MovementFrameUnit {
                unit_id: row.unit,
                owner_slot: ((owner + 1) % SLOTS) as u8,
                unit_type: 0,
                position,
                radius,
                speed: None,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            });
        }
        if local.units.len() != n {
            return None;
        }
        for (&id, pool) in &frame.pools {
            if !uses_pool(id) {
                continue;
            }
            let mut copy = pool.clone();
            let mut exact = Vec::with_capacity(pool.slots.len());
            exact.extend_from_slice(&pool.slots);
            copy.slots = exact;
            local.pools.insert(id, copy);
        }
        seeds.sort_unstable_by_key(|s| s.id);
        if seeds.windows(2).any(|w| w[0].id == w[1].id) {
            return None;
        }
        let mut planning = PlanningGuide {
            seeds: RefCell::new(seeds),
            owner,
        };
        let mut temp = SweepBudget::new(budget.epoch);
        let half = VISITS / (2 * SLOTS);
        let original_real_pairs = budget.pairs[owner];
        // Diagnostic only: determine whether baseline retries starve prediction.
        // Reservations are not work. Remove them when merging actual charges.
        #[cfg(feature = "test-probes")]
        let requested = std::env::var("TURTS_TEST_JOINT_RESERVE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        #[cfg(not(feature = "test-probes"))]
        let requested = 0;
        let available = budget.model_available(owner as u8);
        let terrain_reserve = if requested > 0 {
            owned.min(available)
        } else {
            0
        };
        let primary_reserve = requested
            .min(available.saturating_sub(terrain_reserve))
            .min(OWNER_VISITS.saturating_sub(self.visits[owner]));
        let initial_pairs = half
            .checked_add(budget.pairs[owner])?
            .checked_add(budget.model_reserved[owner])?
            .checked_add(primary_reserve)?;
        temp.pairs.fill(VISITS / SLOTS);
        temp.pairs[owner] = initial_pairs.checked_add(terrain_reserve)?;
        temp.bodies.fill(OWNER_VISITS);
        temp.bodies[owner] = self.visits[owner].checked_add(primary_reserve)?;
        let outputs = step_frame_with_motion_guide(
            &local,
            self.dt,
            &Default::default(),
            &mut temp,
            &mut planning,
        );
        self.visits[owner] = temp.bodies[owner].saturating_sub(primary_reserve);
        budget.index_work.add(temp.index_work);
        budget.envelope_filters = budget
            .envelope_filters
            .saturating_add(temp.envelope_filters);
        budget.index_rebuilds = budget.index_rebuilds.saturating_add(temp.index_rebuilds);
        budget.index_updates = budget.index_updates.saturating_add(temp.index_updates);
        budget.index_update_entries = budget
            .index_update_entries
            .saturating_add(temp.index_update_entries);
        budget.overflows = budget.overflows.saturating_add(temp.overflows);
        budget.support_queries = budget.support_queries.saturating_add(temp.support_queries);
        budget.support_pairs = budget.support_pairs.saturating_add(temp.support_pairs);
        budget.support_failures = budget
            .support_failures
            .saturating_add(temp.support_failures);
        budget.support_accepted_members = budget
            .support_accepted_members
            .saturating_add(temp.support_accepted_members);
        temp.pairs[owner] -= terrain_reserve;
        let terrain_valid = terrain_proofs(
            &mut local.units[..owned],
            &outputs,
            frame,
            &planning,
            &mut temp,
            &mut budget.index_work,
            self.clear,
        );
        // The validation oracle consumes the same cumulative optional ticket.
        budget.pairs[owner] = original_real_pairs + temp.pairs[owner].saturating_sub(initial_pairs);
        budget.model_operations = budget
            .model_operations
            .saturating_add(temp.model_operations);
        for actor in self
            .actors
            .iter_mut()
            .filter(|a| usize::from(a.owner) == owner)
        {
            if let Ok(i) = planning
                .seeds
                .borrow()
                .binary_search_by_key(&actor.id, |s| s.id)
            {
                actor.spent = planning.seeds.borrow()[i].spent;
            }
        }
        if !terrain_valid {
            return None;
        }
        // Compact endpoints are built only after the core's scratch has dropped.
        let mut bodies = Vec::with_capacity(n);
        for unit in &local.units {
            let output = outputs.iter().find(|o| o.unit_id == unit.unit_id)?;
            let owned = usize::from(unit.owner_slot) == owner;
            let limit = if owned {
                f64::from(unit.speed.unwrap_or(0.).max(0.)) * f64::from(self.dt)
            } else {
                0.
            };
            bodies.push(Body {
                id: unit.unit_id,
                from: unit.position,
                to: output.position,
                radius: unit.radius,
                travel: if owned {
                    limit + 16. * ulp(self.bounds.max_element())
                } else {
                    0.
                },
                limit,
                owned,
                upgradeable: owned
                    && self.actors.iter().any(|a| a.id == unit.unit_id)
                    && unit.target.is_some()
                    && unit.physical_escape.is_none()
                    && !output.failed_blocked
                    && unit
                        .progress
                        .as_ref()
                        .is_none_or(|p| p.activity != crate::movement::MovementActivity::Resting)
                    && unit
                        .plain_progress
                        .as_ref()
                        .is_none_or(|p| p.activity != crate::movement::MovementActivity::Resting)
                    && !(planning.kind(unit.unit_id) == MotionKind::Destination
                        && (output.target.is_none()
                            || output.progress.as_ref().is_some_and(|p| {
                                p.activity == crate::movement::MovementActivity::Resting
                            })
                            || output.plain_progress.as_ref().is_some_and(|p| {
                                p.activity == crate::movement::MovementActivity::Resting
                            }))),
            });
        }
        drop(outputs);
        drop(local);
        drop(planning);
        let index = EnvelopeIndex::new(n, |i| bodies[i].envelope())?;
        let plan = JointPlan {
            bodies,
            index,
            bounds: self.bounds,
            peak,
        };
        if base_bytes.checked_add(plan.bytes())? > SCRATCH_BYTES {
            budget.index_work.add(plan.metadata());
            return None;
        }
        for actor in self
            .actors
            .iter_mut()
            .filter(|a| usize::from(a.owner) == owner)
        {
            actor.baseline = plan.endpoint(actor.id);
        }
        for row in &mut self.views[owner].as_mut()?.rows {
            if let Some(to) = plan.endpoint(row.unit) {
                row.agent.velocity = (to.as_dvec2() - row.agent.position) / f64::from(self.dt);
            }
        }
        Some(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unit(id: u32, x: f32, goal: f32) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(x, 10.),
            radius: 0.3,
            speed: Some(10.),
            target: Some(Vec2::new(goal, 10.)),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn frame(units: Vec<MovementFrameUnit>) -> MovementFrame {
        MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            units,
            pools: Default::default(),
        }
    }
    fn guide<'a>(
        frame: &MovementFrame,
        clear: &'a mut dyn FnMut(usize, Vec2, Vec2) -> bool,
    ) -> Guide<'a> {
        let actors = frame
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.owner_slot == 0 && moving(u))
            .map(|(index, u)| Actor {
                id: u.unit_id,
                owner: 0,
                index,
                position: u.position,
                radius: u.radius,
                speed: u.speed.unwrap(),
                goal: u.target.unwrap(),
                destination: u.target.unwrap(),
                kind: MotionKind::Destination,
                age: 0,
                attempted: false,
                predictive_age: 0,
                predictive_attempted: false,
                primary: None,
                baseline: None,
                primary_started: false,
                predictive_serviced: false,
                spent: 0,
                serviced: false,
                ambiguous: false,
                member: None,
                advice: None,
            })
            .collect();
        Guide {
            actors,
            fallback_ids: vec![],
            views: [
                View::capture(frame, 0, 1, 0.05, &[], &BTreeMap::new(), &mut |_, _| true),
                None,
            ],
            dt: 0.05,
            bounds: frame.bounds,
            primary_probe: None,
            joint_peak: 0,
            initial_visits: [0; SLOTS],
            visits: [0; SLOTS],
            clear,
        }
    }
    fn body(id: u32, from: Vec2, to: Vec2, owned: bool) -> Body {
        Body {
            id,
            from,
            to,
            radius: 0.3,
            travel: if owned { 8. } else { 0. },
            limit: if owned { 8. } else { 0. },
            owned,
            upgradeable: owned,
        }
    }
    fn plan(bodies: Vec<Body>) -> JointPlan {
        let index = EnvelopeIndex::new(bodies.len(), |i| bodies[i].envelope()).unwrap();
        JointPlan {
            bodies,
            index,
            bounds: Vec2::splat(128.),
            peak: 0,
        }
    }
    #[test]
    fn joint_baseline_convoy_uses_support_and_keeps_cumulative_tickets() {
        for reverse in [false, true] {
            let mut units = vec![unit(7, 10., 30.), unit(90, 10.7, 100.)];
            if reverse {
                units.reverse();
                units[0].unit_id = 2;
                units[1].unit_id = 100;
            }
            let f = frame(units);
            let mut clear = |_, _, _| true;
            let mut g = guide(&f, &mut clear);
            for a in &mut g.actors {
                a.spent = 3;
            }
            let mut budget = SweepBudget::new(1);
            budget.pairs[0] = 6;
            let p = g.prepare_joint(0, &f, &mut budget, 0).unwrap();
            for u in &f.units {
                assert!(p.endpoint(u.unit_id).unwrap().x > u.position.x);
            }
            assert!(g.actors.iter().all(|a| a.spent > 3 && a.spent <= TICKET));
            assert!(budget.pairs[0] > 6);
            assert!(budget.support_queries > 0);
            assert!(g.visits[0] > 0 && g.visits[0] <= OWNER_VISITS);
            assert!(p.peak_bytes() >= p.bytes());
            assert!(
                g.actors
                    .iter()
                    .all(|a| !a.attempted && !a.predictive_attempted && !a.serviced)
            );
        }
    }
    #[test]
    fn joint_baseline_uses_captured_foreigners_not_truth_frame_or_commands() {
        let own = unit(7, 10., 100.);
        let f = frame(vec![own.clone()]);
        let mut clear = |_, _, _| true;
        let mut g = guide(&f, &mut clear);
        let mut foreign = |_: u8, sink: &mut dyn FnMut(u32, ObservedBody) -> bool| {
            sink(
                99,
                ObservedBody {
                    position: Vec2::new(10.8, 10.),
                    radius: 0.3,
                },
            )
        };
        g.views[0] = View::capture(&f, 0, 1, 0.05, &[], &BTreeMap::new(), &mut foreign);
        let p = g.prepare_joint(0, &f, &mut SweepBudget::new(1), 0).unwrap();
        let stopped = p.endpoint(7).unwrap();
        assert!(stopped.x < 10.5);
        assert_eq!(
            p.bodies.iter().find(|b| b.id == 99).unwrap().to,
            Vec2::new(10.8, 10.)
        );
        drop(g);
        let mut truth = f.clone();
        let mut hidden = own;
        hidden.unit_id = 100;
        hidden.owner_slot = 1;
        hidden.position = Vec2::new(10.6, 10.);
        hidden.target = Some(Vec2::splat(1.));
        truth.units.push(hidden);
        let mut g = guide(&truth, &mut clear);
        g.views[0] = View::capture(&truth, 0, 1, 0.05, &[], &BTreeMap::new(), &mut foreign);
        assert_eq!(
            g.prepare_joint(0, &truth, &mut SweepBudget::new(1), 0)
                .unwrap()
                .endpoint(7),
            Some(stopped)
        );
    }
    #[test]
    fn joint_refusal_has_no_baseline_and_keeps_paid_work() {
        let f = frame(vec![unit(7, 10., 100.)]);
        let mut clear = |_, _, _| false;
        let mut g = guide(&f, &mut clear);
        let mut budget = SweepBudget::new(1);
        assert!(g.prepare_joint(0, &f, &mut budget, 0).is_none());
        assert!(g.actors[0].baseline.is_none());
        assert!(g.actors[0].spent > 0 && budget.pairs[0] > 0);
        let spent = g.actors[0].spent;
        assert!(g.prepare_joint(0, &f, &mut budget, SCRATCH_BYTES).is_none());
        assert_eq!(spent, g.actors[0].spent);
        g.visits[0] = OWNER_VISITS;
        let p = g.prepare_joint(0, &f, &mut budget, 0).unwrap();
        assert_eq!(p.endpoint(7), Some(f.units[0].position));
        assert!(g.actors[0].spent <= TICKET);
    }
    #[test]
    fn joint_actual_short_destination_and_waiting_holder_keep_real_policy() {
        let mut mover = unit(7, 9.8, 10.5);
        mover.speed = Some(20.);
        let mut holder = unit(90, 10.5, 10.5);
        holder.speed = None;
        holder.target = None;
        let f = frame(vec![mover.clone(), holder]);
        let mut clear = |_, _, _| true;
        let mut g = guide(&f, &mut clear);
        let p = g.prepare_joint(0, &f, &mut SweepBudget::new(1), 0).unwrap();
        assert_eq!(p.endpoint(90), Some(Vec2::new(10.5, 10.)));
        assert!(p.endpoint(7).unwrap().distance(Vec2::new(10.5, 10.)) >= 0.6 - 0.00001);
        mover.position.x = 20.;
        mover.target = Some(Vec2::new(20.25, 10.));
        let f = frame(vec![mover]);
        let mut g = guide(&f, &mut clear);
        let mut p = g.prepare_joint(0, &f, &mut SweepBudget::new(1), 0).unwrap();
        assert_eq!(p.endpoint(7), Some(Vec2::new(20.25, 10.)));
        assert!(!p.try_upgrade(7, Vec2::new(20.4, 10.), &mut 0, &mut || true));
    }
    #[test]
    fn joint_upgrades_check_current_plan_and_rollback_on_crossing_or_credit() {
        for reverse in [false, true] {
            let mut rows = vec![
                body(7, Vec2::new(10., 10.), Vec2::new(10., 10.), true),
                body(99, Vec2::new(11., 9.), Vec2::new(11., 11.), true),
            ];
            if reverse {
                rows.reverse();
                rows[0].id = 2;
                rows[1].id = 100;
            }
            let id = rows.iter().find(|b| b.from.y == 10.).unwrap().id;
            let mut p = plan(rows);
            let old = p.endpoint(id);
            let mut visits = 0;
            assert!(!p.try_upgrade(id, Vec2::new(12., 10.), &mut visits, &mut || true));
            assert_eq!(p.endpoint(id), old);
            let mut credit = 1;
            assert!(
                !p.try_upgrade(id, Vec2::new(10., 11.), &mut visits, &mut || {
                    if credit == 0 {
                        false
                    } else {
                        credit -= 1;
                        true
                    }
                })
            );
            assert_eq!(p.endpoint(id), old);
            assert!(p.try_upgrade(id, Vec2::new(10., 11.), &mut visits, &mut || true));
            assert_eq!(p.endpoint(id), Some(Vec2::new(10., 11.)));
            assert!(!p.try_upgrade(id, Vec2::new(30., 10.), &mut visits, &mut || true));
        }
    }
    #[test]
    fn joint_negative_terrain_proof_spends_in_physical_order_after_renaming() {
        let mut outcomes = Vec::new();
        for reverse in [false, true] {
            let mut units = vec![unit(7, 10., 100.), unit(90, 30., 100.)];
            if reverse {
                units.reverse();
                units[0].unit_id = 2;
                units[1].unit_id = 100;
            }
            let f = frame(units);
            let mut proofs = Vec::new();
            let mut clear = |index: usize, _: Vec2, _: Vec2| {
                proofs.push(f.units[index].position.x.to_bits());
                f.units[index].position.x < 20.
            };
            let mut g = guide(&f, &mut clear);
            let mut budget = SweepBudget::new(1);
            assert!(g.prepare_joint(0, &f, &mut budget, 0).is_none());
            let mut spends = g
                .actors
                .iter()
                .map(|a| (a.position.x.to_bits(), a.spent))
                .collect::<Vec<_>>();
            spends.sort_unstable();
            let visits = g.visits;
            assert!(g.actors.iter().all(|a| a.baseline.is_none()));
            drop(g);
            assert_eq!(proofs, vec![10f32.to_bits(), 30f32.to_bits()]);
            outcomes.push((budget.pairs, visits, spends, proofs));
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
    #[test]
    fn joint_indistinguishable_terrain_batch_cannot_partially_take_credit() {
        for remaining in [1usize, 2] {
            for reverse in [false, true] {
                let mut units = vec![unit(7, 10., 100.), unit(90, 10., 100.)];
                if reverse {
                    units.reverse();
                    units[0].unit_id = 2;
                    units[1].unit_id = 100;
                }
                let original = frame(units.clone());
                let mut outputs = units
                    .iter()
                    .map(|u| MovementFrameOutput {
                        recovery: Default::default(),
                        unit_id: u.unit_id,
                        position: u.position + Vec2::X * 0.1,
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
                outputs.sort_unstable_by_key(|o| o.unit_id);
                let mut seeds = units
                    .iter()
                    .map(|u| Seed {
                        id: u.unit_id,
                        kind: MotionKind::Destination,
                        destination: u.target,
                        preferred: None,
                        request: None,
                        spent: 0,
                    })
                    .collect::<Vec<_>>();
                seeds.sort_unstable_by_key(|s| s.id);
                let planning = PlanningGuide {
                    owner: 0,
                    seeds: RefCell::new(seeds),
                };
                let mut budget = SweepBudget::new(1);
                budget.pairs[0] = VISITS / SLOTS - remaining;
                let before = budget.pairs[0];
                let mut calls = 0;
                assert!(!terrain_proofs(
                    &mut units,
                    &outputs,
                    &original,
                    &planning,
                    &mut budget,
                    &mut MetadataWork::default(),
                    &mut |_, _, _| {
                        calls += 1;
                        false
                    }
                ));
                assert_eq!(calls, usize::from(remaining == 2));
                assert_eq!(budget.pairs[0] - before, if remaining == 2 { 2 } else { 0 });
                assert!(
                    planning
                        .seeds
                        .borrow()
                        .iter()
                        .all(|s| s.spent == usize::from(remaining == 2))
                );
            }
        }
    }
    #[test]
    fn joint_changed_far_selected_slot_matches_real_guide_fallback() {
        let f = frame(vec![unit(7, 10., 100.)]);
        let mut clear = |_, _, _| true;
        let mut real = guide(&f, &mut clear);
        let captured = real.actors[0].request(real.bounds, real.dt);
        let preferred = Vec2::new(10.5, 10.);
        real.actors[0].advice = Some(preferred);
        let mut planning = PlanningGuide {
            owner: 0,
            seeds: RefCell::new(vec![Seed {
                id: 7,
                kind: captured.kind,
                destination: Some(captured.goal),
                preferred: Some(preferred),
                request: Some(captured),
                spent: 0,
            }]),
        };
        let mut budget = SweepBudget::new(1);
        assert_eq!(
            planning.next_position(captured, &mut budget),
            Some(preferred)
        );
        let mut changed = captured;
        changed.goal = Vec2::new(10., 20.);
        assert_eq!(
            planning.next_position(changed, &mut budget),
            real.next_position(changed, &mut budget)
        );
        assert_eq!(
            planning.next_position(changed, &mut budget),
            Some(changed.preferred_position())
        );
        for which in 0..6 {
            let mut changed = captured;
            match which {
                0 => changed.position.y += 1.,
                1 => changed.speed += 1.,
                2 => changed.radius += 0.1,
                3 => changed.dt *= 2.,
                4 => changed.bounds.x += 1.,
                _ => changed.kind = MotionKind::Waypoint,
            }
            assert_eq!(
                planning.next_position(changed, &mut budget),
                real.next_position(changed, &mut budget)
            );
        }
        changed.goal = Vec2::new(10.1, 10.);
        assert_eq!(planning.next_position(changed, &mut budget), None);
        assert_eq!(planning.seeds.borrow()[0].spent, 0);
    }
    #[test]
    fn joint_query_and_pair_share_one_seed_counter_without_refunds() {
        let g = PlanningGuide {
            owner: 0,
            seeds: RefCell::new(vec![Seed {
                id: 7,
                kind: MotionKind::Destination,
                destination: None,
                preferred: None,
                request: None,
                spent: TICKET - 2,
            }]),
        };
        let mut b = SweepBudget::new(1);
        b.pairs[0] = VISITS / (2 * SLOTS);
        assert!(g.charge_certificate_visit(7, 0, false, &mut b));
        let mut local = 0;
        assert!(g.charge_certificate_pair(7, 0, &mut local, 10, &mut b));
        let visits = b.bodies[0];
        assert!(!g.charge_certificate_visit(7, 0, false, &mut b));
        assert_eq!(b.bodies[0], visits);
        assert!(!g.charge_certificate_pair(7, 0, &mut local, 10, &mut b));
        assert_eq!(g.seeds.borrow()[0].spent, TICKET);
        assert_eq!(local, 1);
        assert!(!g.certificate_search_enabled());
    }
}
