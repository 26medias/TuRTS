//! Bounded local guidance. The collision solver and physical terrain sweep still own safety.
//!
//! Handles index retained intent only. Entrance priority uses observed positions and time;
//! no iteration over a handle map selects a direction or a steering candidate.
mod coherence;
mod detour;
mod envelope_index;
mod lazy;
mod observed_detour;
mod own_detour;
mod parking;
pub(crate) mod public_passage;
pub(crate) use parking::DestinationRegion;
use public_passage::Rows as GateRows;
mod spatial;
pub(crate) mod velocity;
mod velocity_solver;
use crate::movement::{MovementFrame, MovementFrameUnit};
use bevy_ecs::prelude::Resource;
use bevy_math::Vec2;
pub(crate) use envelope_index::{EnvelopeIndex, MetadataWork};
use spatial::{NeighborRows, Spatial};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

const NEIGHBORS: usize = 128;
const VISITS: usize = 65_536;
const BATCH: usize = 4;
const GATES: usize = 128;
const MEMBERS: usize = 4096;
const OWNER_GATES: usize = GATES / SLOTS;
pub(crate) const SCRATCH_BYTES: usize = 8 * 1024 * 1024;
const SLOTS: usize = turts_mod_api::SLOT_COUNT as usize;
// Half of the existing owner-local work envelope is reserved for final body certification.
const OWNER_VISITS: usize = VISITS / (2 * SLOTS);
/// Held separately by navigation before routing; released only to the frozen
/// coherence/model pass after the routing indexes have been retired.
pub(crate) const GUIDED_RESERVE: usize = 8192;

const OWNER_MEMBERS: usize = MEMBERS / SLOTS;
// Preserve the measured pre-index physical-row admission cap on every target.
const INDEX_BODIES: usize = 12_870;
const PER_BODY_SCRATCH: usize = std::mem::size_of::<(u32, &MovementFrameUnit)>()
    + 2 * std::mem::size_of::<usize>()
    + std::mem::size_of::<MovementFrameUnit>();
const FIXED_SCRATCH: usize = MEMBERS * 128
    + 4 * MEMBERS * std::mem::size_of::<usize>()
    + MEMBERS * std::mem::size_of::<u32>()
    + 4096
    + INDEX_BODIES * std::mem::size_of::<bool>();
const RETAINED_ALLOWANCE: usize = std::mem::size_of::<TrafficState>()
    + 4096
    + MEMBERS * 512
    + MEMBERS * std::mem::size_of::<velocity::HistoryRow>()
    + SLOTS * 32
    + GATES * std::mem::size_of::<Gate>()
    + GATES * BATCH * 256;
const _: () = assert!(
    RETAINED_ALLOWANCE
        + INDEX_BODIES * PER_BODY_SCRATCH
        + FIXED_SCRATCH
        + Spatial::capacity_bound(INDEX_BODIES)
        + NeighborRows::capacity_bound(INDEX_BODIES)
        <= SCRATCH_BYTES
);
pub(crate) const fn index_capacity() -> usize {
    INDEX_BODIES
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum Attempt {
    Neighbor,
    Detour,
    Lazy,
    Parking,
}
#[derive(Clone, Debug, Default)]
struct Attempts {
    values: [u32; 4],
    present: u8,
}
impl Attempts {
    fn get(&self, which: Attempt) -> Option<u32> {
        let i = which as usize;
        (self.present & (1 << i) != 0).then_some(self.values[i])
    }
    fn set(&mut self, which: Attempt, value: Option<u32>) {
        let i = which as usize;
        if let Some(tick) = value {
            self.values[i] = tick;
            self.present |= 1 << i;
        } else {
            self.present &= !(1 << i);
        }
    }
}
const _: () = assert!(std::mem::size_of::<Attempts>() == 20);

#[test]
fn packed_attempt_clocks_preserve_all_tick_bits_and_independent_presence() {
    let variants = [
        Attempt::Neighbor,
        Attempt::Detour,
        Attempt::Lazy,
        Attempt::Parking,
    ];
    let mut packed = Attempts::default();
    let mut reference = [None; 4];
    let mut seed = 0x475032u32;
    for step in 0..100_000 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let index = (seed as usize >> 16) % 4;
        let value = match step % 5 {
            0 => None,
            1 => Some(0),
            2 => Some(u32::MAX),
            3 => Some(1),
            _ => Some(seed),
        };
        reference[index] = value;
        packed.set(variants[index], value);
        for i in 0..4 {
            assert_eq!(
                packed.get(variants[i]),
                reference[i],
                "step {step}, clock {i}"
            );
        }
    }
}
#[derive(Clone, Debug)]
struct Member {
    owner: u8,
    target: Vec2,
    entrance: Option<EntranceStamp>,
    accepted_tick: u32,
    passing: Option<Passing>,
    local_detour: Option<own_detour::Retained>,
    lazy_plan: lazy::Plan,
    parking: Option<Vec2>,
    parking_retained: bool,
    guidance_pairs: usize,
    motion_profile: [u32; 3],
    attempts: Attempts,
}
const _: () =
    assert!(11 * (std::mem::size_of::<Member>() + std::mem::size_of::<u32>()) + 128 <= 5 * 512);
#[derive(Clone, Copy, Debug)]
struct Passing {
    point: Vec2,
    centre: Vec2,
    axis: Vec2,
    direction: f32,
}
#[derive(Debug)]
struct Gate {
    centre: Vec2,
    axis: Vec2,
    half_length: f32,
    radius: f32,
    direction: f32,
    admitted: GateRows,
    reserved_by: [bool; SLOTS],
    provisional_for: Option<u32>,
    issued: usize,
    blocked_contact: Option<(u32, Vec2, Vec2)>,
}
pub(crate) use turts_rules::terrain::geometry::DualSupportContact as EntranceContact;
#[derive(Clone, Copy, Debug, PartialEq)]
struct EntranceStamp {
    tick: u32,
    lower: u32,
    upper: u32,
    denominator: u32,
    point: Vec2,
    inside_at_start: bool,
}
impl EntranceStamp {
    fn new(tick: u32, point: Vec2, contact: EntranceContact) -> Option<Self> {
        let (lower, upper, denominator) = match contact {
            EntranceContact::Inside => (0, 0, 1),
            EntranceContact::Transition {
                lower,
                upper,
                denominator,
            } if denominator > 0 && lower <= upper && upper <= denominator => {
                (lower, upper, denominator)
            }
            _ => return None,
        };
        Some(Self {
            tick,
            point,
            lower,
            upper,
            denominator,
            inside_at_start: contact == EntranceContact::Inside,
        })
    }
    fn contact(self) -> EntranceContact {
        if self.inside_at_start {
            return EntranceContact::Inside;
        }
        EntranceContact::Transition {
            lower: self.lower,
            upper: self.upper,
            denominator: self.denominator,
        }
    }
}
fn contact_order(a: EntranceContact, b: EntranceContact) -> Option<Ordering> {
    use EntranceContact::*;
    let valid = |value| match value {
        Transition {
            lower,
            upper,
            denominator,
        } => denominator > 0 && lower <= upper && upper <= denominator,
        _ => true,
    };
    if !valid(a) || !valid(b) {
        return None;
    }
    match (a, b) {
        (Inside, Inside) => Some(Ordering::Equal),
        (Inside, Outside | Transition { .. }) => Some(Ordering::Less),
        (Outside | Transition { .. }, Inside) => Some(Ordering::Greater),
        (Transition { .. }, Outside) => Some(Ordering::Less),
        (Outside, Transition { .. }) => Some(Ordering::Greater),
        (
            Transition {
                lower: a0,
                upper: a1,
                denominator: ad,
            },
            Transition {
                lower: b0,
                upper: b1,
                denominator: bd,
            },
        ) if ad > 0 && bd > 0 && a0 <= a1 && a1 <= ad && b0 <= b1 && b1 <= bd => {
            let cross = |n: u32, d: u32| u64::from(n) * u64::from(d);
            if cross(a1, bd) < cross(b0, ad) {
                Some(Ordering::Less)
            } else if cross(b1, ad) < cross(a0, bd) {
                Some(Ordering::Greater)
            } else if a0 == a1 && b0 == b1 && cross(a0, bd) == cross(b0, ad) {
                Some(Ordering::Equal)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Match-local traffic intent. Caller must forget even an identical-target Replace.
#[derive(Resource, Default)]
pub(crate) struct TrafficState {
    pub(crate) motion_history: [Vec<velocity::HistoryRow>; SLOTS],
    members: BTreeMap<u32, Member>,
    gates: Vec<Gate>,
    last_tick: Option<u32>,
    pub(crate) neighbor_visits: u64,
    pub(crate) neighbor_bucket_visits: u64,
    pub(crate) neighbor_overflows: u64,
    pub(crate) body_pair_checks: u64,
    pub(crate) tick_body_pair_checks: usize,
    tick_neighbors: usize,
    tick_buckets: usize,
    tick_owner_neighbors: [usize; SLOTS],
    tick_owner_buckets: [usize; SLOTS],
    scratch_bound: usize,
    pub(crate) capacity_blocked: bool,
    pub(crate) index_work: MetadataWork,
    pub(crate) index_work_total: MetadataWork,
    detour_work: detour::Work,
    pub(crate) detour_filter_rows: usize,
}
impl TrafficState {
    pub(crate) fn detour_counters(&self) -> (usize, usize, usize, usize, usize) {
        let w = self.detour_work;
        (
            w.scanned_rows,
            w.generator_comparisons,
            w.candidates,
            w.body_checks,
            w.terrain_calls,
        )
    }
    pub(crate) fn index_query_bounds() -> (u64, u64, u64) {
        (
            envelope_index::query_rank_bound() as u64,
            252,
            envelope_index::query_word_bound() as u64,
        )
    }
    pub(crate) fn retained_members(&self) -> usize {
        self.members.len()
    }
    pub(crate) fn retained_gates(&self) -> usize {
        self.gates.len()
    }
    /// Conservative locked-toolchain node+allocator allowances, not just payload sizes.
    /// Member nodes reserve 512 bytes each; u32 gate-set rows reserve 256 bytes each.
    pub(crate) fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + 4096
            + self.members.len() * 512
            + self
                .motion_history
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<velocity::HistoryRow>())
                .sum::<usize>()
            + SLOTS * 32
            + self.gates.capacity() * std::mem::size_of::<Gate>()
            + self
                .gates
                .iter()
                .map(|gate| gate.admitted.len())
                .sum::<usize>()
                * 256
    }
    pub(crate) fn scratch_bytes(&self) -> usize {
        self.scratch_bound
    }
    pub(crate) fn tick_neighbor_work(&self) -> (usize, usize) {
        (self.tick_neighbors, self.tick_buckets)
    }
    #[cfg(test)]
    pub(crate) fn tick_owner_neighbor_work(&self, owner: u8) -> Option<(usize, usize)> {
        let owner = usize::from(owner);
        Some((
            *self.tick_owner_neighbors.get(owner)?,
            *self.tick_owner_buckets.get(owner)?,
        ))
    }
    pub(crate) fn record_body_guard(&mut self, budget: &crate::movement::sweep::SweepBudget) {
        for owner in 0..SLOTS {
            self.tick_owner_neighbors[owner] += budget.bodies[owner];
            self.tick_owner_buckets[owner] += budget.buckets[owner];
        }
        let bodies = budget.bodies.iter().sum::<usize>();
        let buckets = budget.buckets.iter().sum::<usize>();
        let pairs = budget.pairs.iter().sum::<usize>();
        self.tick_neighbors += bodies;
        self.tick_buckets += buckets;
        self.neighbor_visits = self.neighbor_visits.saturating_add(bodies as u64);
        self.neighbor_bucket_visits = self.neighbor_bucket_visits.saturating_add(buckets as u64);
        self.neighbor_overflows = self
            .neighbor_overflows
            .saturating_add(budget.overflows as u64);
        self.body_pair_checks = self.body_pair_checks.saturating_add(pairs as u64);
        self.tick_body_pair_checks = pairs;
        // Guidance is gone before this index is created. Retained gate/member
        // state remains; reserve the larger sequential transient allocation.
        self.scratch_bound = self.scratch_bound.max(budget.peak_bytes);
    }
    /// Retire a failed local permitted-geometry lease without restarting fair
    /// query service or unrelated physical gates. Caller owns the local lease.
    pub(crate) fn preserves_public_contact(&self, actor: &MovementFrameUnit) -> bool {
        self.gates.iter().any(|g| {
            g.admitted.public()
                && g.reserved_by[usize::from(actor.owner_slot)]
                && gate_local(g, actor)
        })
    }
    pub(crate) fn invalidate_contact(&mut self, actor: &MovementFrameUnit) {
        for gate in &mut self.gates {
            if gate.admitted.public()
                && gate.reserved_by[usize::from(actor.owner_slot)]
                && gate_local(gate, actor)
            {
                gate.admitted.suspend();
            }
        }
        if let Some(member) = self.members.get_mut(&actor.unit_id) {
            member.entrance = None;
            member.passing = None;
            member.local_detour = None;
        }
        self.gates.retain(|gate| {
            gate.admitted.public()
                || gate.provisional_for != Some(actor.unit_id)
                    && !gate.admitted.contains(&actor.unit_id)
                    && !gate_local(gate, actor)
        });
    }

    pub(crate) fn forget(&mut self, unit: u32) {
        self.members.remove(&unit);
        // Intent/owner/affected-geometry retirement invalidates this local
        // reservation. Other bodies keep their own actual entrance histories.
        self.gates
            .retain(|gate| gate.provisional_for != Some(unit) && !gate.admitted.contains(&unit));
        for gate in &mut self.gates {
            gate.admitted.remove(&unit);
        }
    }
}
fn zero(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}
fn point_cmp(a: Vec2, b: Vec2) -> Ordering {
    zero(a.x)
        .total_cmp(&zero(b.x))
        .then(zero(a.y).total_cmp(&zero(b.y)))
}
fn physical(a: &MovementFrameUnit, b: &MovementFrameUnit) -> Ordering {
    point_cmp(a.position, b.position)
        .then(a.radius.total_cmp(&b.radius))
        .then_with(|| {
            point_cmp(
                a.target.unwrap_or(a.position),
                b.target.unwrap_or(b.position),
            )
        })
}
fn heading(unit: &MovementFrameUnit) -> Vec2 {
    unit.target.map_or(Vec2::ZERO, |target| {
        (target - unit.position).normalize_or_zero()
    })
}
fn moving(unit: &MovementFrameUnit) -> bool {
    unit.speed
        .is_some_and(|speed| speed.is_finite() && speed > 0.0)
        && unit.target.is_some_and(|target| {
            target.is_finite() && target.distance_squared(unit.position) > 1e-8
        })
}
fn canonical(mut axis: Vec2) -> Vec2 {
    if axis.x < 0.0 || (axis.x == 0.0 && axis.y < 0.0) {
        axis = -axis;
    }
    axis
}
fn gate_local(gate: &Gate, unit: &MovementFrameUnit) -> bool {
    let offset = unit.position - gate.centre;
    let along = offset.dot(gate.axis);
    // Physical outgoing-side exclusion replaces retained completed actor IDs.
    let outgoing = along * heading(unit).dot(gate.axis).signum() > gate.half_length;
    gate.reserved_by
        .get(usize::from(unit.owner_slot))
        .copied()
        .unwrap_or(false)
        && moving(unit)
        && !outgoing
        && along.abs() <= gate.half_length + 4.0 * gate.radius
        && offset.perp_dot(gate.axis).abs() < 4.0 * gate.radius
}

fn stopped(proposed: &mut MovementFrame, index: usize) {
    proposed.units[index].target = None;
    proposed.units[index].speed = Some(0.0);
}
fn segment_distance(point: Vec2, from: Vec2, to: Vec2) -> f32 {
    let delta = to - from;
    let t = if delta.length_squared() > 0.0 {
        ((point - from).dot(delta) / delta.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    point.distance(from + delta * t)
}
#[cfg(test)]
fn candidate_clear(
    frame: &MovementFrame,
    index: usize,
    to: Vec2,
    neighbors: &[usize],
    clear: &mut impl FnMut(usize, Vec2, Vec2) -> bool,
) -> bool {
    let unit = &frame.units[index];
    if !to.is_finite()
        || to.x < unit.radius
        || to.y < unit.radius
        || to.x > frame.bounds.x - unit.radius
        || to.y > frame.bounds.y - unit.radius
        || !clear(index, unit.position, to)
    {
        return false;
    }
    neighbors.iter().all(|&other| {
        other == index
            || segment_distance(frame.units[other].position, unit.position, to)
                >= unit.radius + frame.units[other].radius
    })
}
struct Live<'a>(Vec<(u32, &'a MovementFrameUnit)>);
impl<'a> Live<'a> {
    fn new(frame: &'a MovementFrame) -> Self {
        let mut rows = frame
            .units
            .iter()
            .map(|unit| (unit.unit_id, unit))
            .collect::<Vec<_>>();
        rows.sort_unstable_by_key(|row| row.0);
        Self(rows)
    }
    fn get(&self, id: &u32) -> Option<&&'a MovementFrameUnit> {
        self.0
            .binary_search_by_key(id, |row| row.0)
            .ok()
            .map(|index| &self.0[index].1)
    }
}
/// Record actual certified output, never a planned contact. Outputs must be the
/// unique unit-ID-sorted final collision+terrain result, from this frame and tick.
/// The callback must use the SAME pinned permitted geometry as movement planning.
/// Caller must forget affected members when their permitted geometry epoch changes.
pub(crate) fn record_committed_contacts(
    frame: &MovementFrame,
    outputs: &[crate::movement::MovementFrameOutput],
    tick: u32,
    state: &mut TrafficState,
    mut contact: impl FnMut(usize, Vec2, Vec2, Vec2) -> EntranceContact,
) {
    if state.capacity_blocked || frame.units.len() > INDEX_BODIES {
        return;
    }
    if !outputs
        .windows(2)
        .all(|rows| rows[0].unit_id < rows[1].unit_id)
    {
        return;
    }
    let mut actual_contacts = Vec::with_capacity(state.members.len());
    let mut order = Vec::with_capacity(frame.units.len());
    order.extend(
        (0..frame.units.len())
            .filter(|&index| state.members.contains_key(&frame.units[index].unit_id)),
    );
    order.sort_by(|&a, &b| physical(&frame.units[a], &frame.units[b]));
    for (rank, &index) in order.iter().enumerate() {
        let unit = &frame.units[index];
        if rank > 0 && physical(unit, &frame.units[order[rank - 1]]) == Ordering::Equal
            || rank + 1 < order.len()
                && physical(unit, &frame.units[order[rank + 1]]) == Ordering::Equal
        {
            continue;
        }
        let Some(member) = state.members.get_mut(&unit.unit_id) else {
            continue;
        };
        if member.owner != unit.owner_slot || Some(member.target) != unit.target {
            continue;
        }
        let Ok(output_index) = outputs.binary_search_by_key(&unit.unit_id, |output| output.unit_id)
        else {
            continue;
        };
        let to = outputs[output_index].position;
        let axis = canonical(heading(unit));
        if axis == Vec2::ZERO {
            continue;
        }
        let observed = contact(index, unit.position, to, axis);
        if EntranceStamp::new(tick, unit.position, observed).is_some() {
            actual_contacts.push(unit.unit_id);
        }
        let at_end = contact(index, to, to, axis);
        if at_end == EntranceContact::Outside {
            // A completed departure retires the old entrance. A later distinct
            // choke on the same player order must earn a new physical timestamp.
            member.entrance = None;
        } else if let Some(stamp) = EntranceStamp::new(tick, unit.position, observed) {
            member.entrance.get_or_insert(stamp);
        }
    }
    actual_contacts.sort_unstable();
    state.gates.retain_mut(|gate| {
        let Some(unit) = gate.provisional_for else {
            return true;
        };
        if actual_contacts.binary_search(&unit).is_err() {
            return false;
        }
        gate.provisional_for = None;
        true
    });
}

/// Change temporary route waypoints only. The caller restores original order endpoints.
/// `clear` must use this actor's permitted terrain, including its full swept footprint.
/// Unknown/uncertified proposals stay stationary; they never become direct permission.
#[cfg(test)]
pub(crate) fn apply(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    clear: impl FnMut(usize, Vec2, Vec2) -> bool,
) -> BTreeSet<u32> {
    // Compatibility callers can still take certified passing/yield paths. They
    // cannot invent a choke winner without a certified first-contact witness.
    apply_with_contacts(frame, proposed, dt, tick, state, clear, |_, _, _, _| {
        EntranceContact::Unresolved
    })
}
#[cfg(test)]
pub(crate) fn apply_with_contacts(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    clear: impl FnMut(usize, Vec2, Vec2) -> bool,
    contact: impl FnMut(usize, Vec2, Vec2, Vec2) -> EntranceContact,
) -> BTreeSet<u32> {
    apply_with_detours(
        frame,
        proposed,
        dt,
        tick,
        state,
        clear,
        contact,
        |_| false,
        &mut crate::movement::sweep::SweepBudget::new(tick),
    )
}
#[allow(
    clippy::too_many_arguments,
    reason = "One synchronous controller shares permitted geometry and the final physical work budget"
)]
#[cfg(test)]
pub(crate) fn apply_with_detours(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    clear: impl FnMut(usize, Vec2, Vec2) -> bool,
    contact: impl FnMut(usize, Vec2, Vec2, Vec2) -> EntranceContact,
    eligible: impl FnMut(usize) -> bool,
    pair_budget: &mut crate::movement::sweep::SweepBudget,
) -> BTreeSet<u32> {
    apply_with_observed_detours(
        frame,
        proposed,
        dt,
        tick,
        state,
        clear,
        contact,
        eligible,
        pair_budget,
        |_, _| None,
        |_| false,
        |_, _| true,
    )
}

/// A CURRENT permitted Start body. Foreign orders and live poses are absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ObservedBody {
    pub(crate) position: Vec2,
    pub(crate) radius: f32,
}

#[allow(
    clippy::too_many_arguments,
    reason = "Pin observation and physical budget ports in one synchronous controller"
)]
#[cfg(test)]
pub(crate) fn apply_with_observed_detours(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    clear: impl FnMut(usize, Vec2, Vec2) -> bool,
    contact: impl FnMut(usize, Vec2, Vec2, Vec2) -> EntranceContact,
    eligible: impl FnMut(usize) -> bool,
    pair_budget: &mut crate::movement::sweep::SweepBudget,
    observed: impl FnMut(u8, u32) -> Option<ObservedBody>,
    observed_eligible: impl FnMut(usize) -> bool,
    observed_rows: impl FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
) -> BTreeSet<u32> {
    apply_composed(
        frame,
        proposed,
        dt,
        tick,
        state,
        clear,
        contact,
        eligible,
        pair_budget,
        observed,
        observed_eligible,
        observed_rows,
        false,
        |_| None,
        |_| false,
    )
}

/// Routing for the frozen preferred-velocity composition. Optional incomplete
/// own-neighbor advice cannot erase the incoming permitted candidate. Physical
/// gates, capacity fences and permission waits still hold normally.
#[allow(
    clippy::too_many_arguments,
    reason = "Same pinned synchronous ports as the legacy controller"
)]
#[cfg(test)]
pub(crate) fn apply_guided_with_observed_detours(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    clear: impl FnMut(usize, Vec2, Vec2) -> bool,
    contact: impl FnMut(usize, Vec2, Vec2, Vec2) -> EntranceContact,
    eligible: impl FnMut(usize) -> bool,
    pair_budget: &mut crate::movement::sweep::SweepBudget,
    observed: impl FnMut(u8, u32) -> Option<ObservedBody>,
    observed_eligible: impl FnMut(usize) -> bool,
    observed_rows: impl FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
) -> BTreeSet<u32> {
    apply_composed(
        frame,
        proposed,
        dt,
        tick,
        state,
        clear,
        contact,
        eligible,
        pair_budget,
        observed,
        observed_eligible,
        observed_rows,
        true,
        |_| None,
        |_| false,
    )
}

/// Ordinary group Move parking. The ports expose original public intent and
/// actual own settled state; temporary terrain waits are never settled holders.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_guided_with_parking(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    clear: impl FnMut(usize, Vec2, Vec2) -> bool,
    contact: impl FnMut(usize, Vec2, Vec2, Vec2) -> EntranceContact,
    eligible: impl FnMut(usize) -> bool,
    pair_budget: &mut crate::movement::sweep::SweepBudget,
    observed: impl FnMut(u8, u32) -> Option<ObservedBody>,
    observed_eligible: impl FnMut(usize) -> bool,
    observed_rows: impl FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
    public_goal: impl FnMut(usize) -> Option<DestinationRegion>,
    settled: impl FnMut(usize) -> bool,
) -> BTreeSet<u32> {
    apply_composed(
        frame,
        proposed,
        dt,
        tick,
        state,
        clear,
        contact,
        eligible,
        pair_budget,
        observed,
        observed_eligible,
        observed_rows,
        true,
        public_goal,
        settled,
    )
}
impl TrafficState {
    pub(crate) fn parking_slot(&self, unit: u32) -> Option<Vec2> {
        self.members
            .get(&unit)
            .filter(|member| !member.parking_retained)
            .and_then(|member| member.parking)
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "One explicit policy selects bounded controller composition"
)]
#[allow(
    clippy::needless_range_loop,
    reason = "Owner indices select independent service budgets."
)]
fn apply_composed(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    mut clear: impl FnMut(usize, Vec2, Vec2) -> bool,
    mut contact: impl FnMut(usize, Vec2, Vec2, Vec2) -> EntranceContact,
    mut eligible: impl FnMut(usize) -> bool,
    pair_budget: &mut crate::movement::sweep::SweepBudget,
    mut observed: impl FnMut(u8, u32) -> Option<ObservedBody>,
    mut observed_eligible: impl FnMut(usize) -> bool,
    mut observed_rows: impl FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
    guided: bool,
    mut public_goal: impl FnMut(usize) -> Option<DestinationRegion>,
    mut settled: impl FnMut(usize) -> bool,
) -> BTreeSet<u32> {
    let route_quota = if guided {
        OWNER_VISITS.saturating_sub(GUIDED_RESERVE)
    } else {
        OWNER_VISITS
    };
    for member in state.members.values_mut() {
        member.parking_retained = member.parking.is_some();
        member.guidance_pairs = 0;
    }
    state.tick_body_pair_checks = 0;
    state.tick_neighbors = 0;
    state.tick_buckets = 0;
    state.tick_owner_neighbors = [0; SLOTS];
    state.tick_owner_buckets = [0; SLOTS];
    state.capacity_blocked = false;
    state.index_work = MetadataWork::default();
    state.detour_work = detour::Work::default();
    state.detour_filter_rows = 0;
    // Exact-width spatial/live vectors, two order vectors, worst-case waiting-set
    // nodes, bounded successful-query Vec growth and per-query map nodes. Scratch
    // includes transient gate scans/retirement and allocator allowance.
    // Guidance rows contain no nested progress/escape allocations and pools are empty.
    // Reserve their full unit-vector clone alongside the controller's own storage.
    state.scratch_bound = frame
        .units
        .len()
        .saturating_mul(PER_BODY_SCRATCH)
        .saturating_add(FIXED_SCRATCH);
    let mut waiting = BTreeSet::new();
    if frame.units.len() > INDEX_BODIES
        || state.retained_bytes().saturating_add(state.scratch_bound) > SCRATCH_BYTES
    {
        state.capacity_blocked = true;
        // An oversized physical frame is borrowed directly by the caller. No
        // guidance clone or spatial index is allocated on this refusal path.
        let guidance_bytes = if frame.units.len() <= INDEX_BODIES {
            frame.units.len() * std::mem::size_of::<MovementFrameUnit>()
        } else {
            0
        };
        state.scratch_bound = guidance_bytes
            .saturating_add(frame.units.len().min(INDEX_BODIES) * 128)
            .saturating_add(256);
        for (index, unit) in frame.units.iter().enumerate().take(proposed.units.len()) {
            if moving(unit) {
                stopped(proposed, index);
                if waiting.len() < INDEX_BODIES {
                    waiting.insert(unit.unit_id);
                }
            }
        }
        return waiting;
    }
    if frame.units.len() != proposed.units.len() || !dt.is_finite() || dt <= 0.0 {
        for index in 0..proposed.units.len() {
            stopped(proposed, index);
        }
        return frame
            .units
            .iter()
            .filter(|unit| moving(unit))
            .map(|unit| unit.unit_id)
            .collect();
    }
    if state.last_tick.is_some_and(|previous| tick < previous) {
        *state = TrafficState::default();
    }
    state.last_tick = Some(tick);
    let live = Live::new(frame);
    let retired = state
        .members
        .iter()
        .filter_map(|(&id, member)| {
            let retained = live.get(&id).is_some_and(|unit| {
                moving(unit)
                    && unit.owner_slot == member.owner
                    && unit.target == Some(member.target)
            });
            (!retained).then_some(id)
        })
        .collect::<Vec<_>>();
    for id in retired {
        state.forget(id);
    }
    let mut order = Vec::with_capacity(frame.units.len());
    order.extend((0..frame.units.len()).filter(|&index| moving(&frame.units[index])));
    order.sort_by(|&a, &b| physical(&frame.units[a], &frame.units[b]));
    let mut visits = [0; SLOTS];
    let mut bookkeeping = [0; SLOTS];
    let mut owner_members = [0; SLOTS];
    for member in state.members.values() {
        if let Some(count) = owner_members.get_mut(usize::from(member.owner)) {
            *count += 1;
        }
    }
    order.retain(|&index| {
        let unit = &frame.units[index];
        let Some(count) = owner_members.get_mut(usize::from(unit.owner_slot)) else {
            waiting.insert(unit.unit_id);
            stopped(proposed, index);
            return false;
        };
        if !state.members.contains_key(&unit.unit_id) {
            if *count == OWNER_MEMBERS {
                waiting.insert(unit.unit_id);
                stopped(proposed, index);
                return false;
            }
            *count += 1;
        }
        state.members.entry(unit.unit_id).or_insert(Member {
            owner: unit.owner_slot,
            target: unit.target.unwrap_or(unit.position),
            entrance: None,
            accepted_tick: tick,
            passing: None,
            local_detour: None,
            lazy_plan: lazy::Plan::None,
            parking: None,
            parking_retained: false,
            guidance_pairs: 0,
            motion_profile: [0; 3],
            attempts: Attempts::default(),
        });
        true
    });
    // Work scheduling is independent of the physical order used by steering.
    // A new request joins at this tick, behind older unserved or rotated work.
    let mut service = order.clone();
    if guided {
        // Exact indistinguishable physical starts receive the same refusal, independent of IDs.
        for pair in order.windows(2) {
            if physical(&frame.units[pair[0]], &frame.units[pair[1]]) == Ordering::Equal {
                for &index in pair {
                    waiting.insert(frame.units[index].unit_id);
                    stopped(proposed, index);
                }
            }
        }
        observed_detour::apply(
            frame,
            proposed,
            dt,
            tick,
            state,
            &mut service,
            &waiting,
            &mut visits,
            &mut clear,
            pair_budget,
            &mut observed,
            &mut observed_eligible,
            &mut observed_rows,
            route_quota,
        );
    }
    // Foreign capture/index have dropped before cooperative index allocation.
    let early_index_work = state.index_work;
    let mut spatial = Spatial::new(frame);
    let mut neighbors = NeighborRows::new(frame.units.len());
    service.sort_by(|&a, &b| {
        let left = &frame.units[a];
        let right = &frame.units[b];
        let age = |unit: &MovementFrameUnit| {
            let member = &state.members[&unit.unit_id];
            (
                member
                    .attempts
                    .get(Attempt::Neighbor)
                    .unwrap_or(member.accepted_tick),
                member.attempts.get(Attempt::Neighbor).is_some(),
                member.accepted_tick,
            )
        };
        age(left)
            .cmp(&age(right))
            .then_with(|| physical(left, right))
    });
    for &index in &service {
        let unit = &frame.units[index];
        let owner = usize::from(unit.owner_slot);
        let before = (visits[owner], bookkeeping[owner]);
        let result = spatial.neighbors(
            frame,
            index,
            dt,
            &mut visits[owner],
            &mut bookkeeping[owner],
            route_quota,
        );
        if visits[owner] > before.0 || bookkeeping[owner] > before.1 {
            // A dense partial query consumed service and rotates. A query after
            // exhaustion consumed nothing and must keep its pending age.
            if let Some(member) = state.members.get_mut(&unit.unit_id) {
                member.attempts.set(Attempt::Neighbor, Some(tick));
            }
        }
        let Some(mut near) = result else {
            state.neighbor_overflows += 1;
            if !guided {
                waiting.insert(unit.unit_id);
                stopped(proposed, index);
            }
            continue;
        };
        near.sort_by(|&a, &b| physical(&frame.units[a], &frame.units[b]));
        if !neighbors.insert(index, near) {
            state.neighbor_overflows += 1;
            if !guided {
                waiting.insert(unit.unit_id);
                stopped(proposed, index);
            }
        }
    }
    if guided {
        // Only stages that consume NeighborRows use this complete-row service
        // list. Keep physical order intact for every real gate participant.
        service.retain(|&index| neighbors.has(index));
    }
    state.index_work = early_index_work;
    state.index_work.add(spatial.work);
    state.scratch_bound = state.scratch_bound.max(
        frame.units.len() * PER_BODY_SCRATCH + FIXED_SCRATCH + spatial.bytes() + neighbors.bytes(),
    );
    state.tick_neighbors = visits.iter().sum();
    state.tick_buckets = bookkeeping.iter().sum();
    state.tick_owner_neighbors = visits;
    state.tick_owner_buckets = bookkeeping;
    state.neighbor_visits += state.tick_neighbors as u64;
    state.neighbor_bucket_visits += state.tick_buckets as u64;
    if !guided {
        // Exact indistinguishable physical starts receive the same refusal, independent of IDs.
        for pair in order.windows(2) {
            if physical(&frame.units[pair[0]], &frame.units[pair[1]]) == Ordering::Equal {
                for &index in pair {
                    waiting.insert(frame.units[index].unit_id);
                    stopped(proposed, index);
                }
            }
        }
    }
    let mut locally_guided = own_detour::apply(
        frame,
        proposed,
        dt,
        tick,
        state,
        &mut service,
        &neighbors,
        &mut waiting,
        &mut eligible,
        &mut public_goal,
        &mut clear,
        pair_budget,
    );
    if guided {
        // Preserve already selected/retained foreign intent through optional own
        // steering. Real gate waits still run over the full physical order below.
        for &index in &order {
            if state.members[&frame.units[index].unit_id]
                .local_detour
                .is_some_and(|route| route.witness.is_some())
            {
                locally_guided[index] = true;
            }
        }
    }
    let mut reservations = [0; SLOTS];
    for gate in &mut state.gates {
        if gate.admitted.public() {
            for owner in 0..SLOTS {
                reservations[owner] += usize::from(gate.reserved_by[owner]);
            }
            continue;
        }
        let mut present = [false; SLOTS];
        for &index in &order {
            let unit = &frame.units[index];
            if gate_local(gate, unit) {
                present[usize::from(unit.owner_slot)] = true;
            }
        }
        for owner in 0..SLOTS {
            gate.reserved_by[owner] &= present[owner];
            if gate.reserved_by[owner] {
                reservations[owner] += 1;
            }
        }
    }
    state
        .gates
        .retain(|gate| gate.reserved_by.iter().any(|reserved| *reserved));
    lazy::prepare(
        frame,
        proposed,
        dt,
        tick,
        state,
        &mut service,
        &neighbors,
        &locally_guided,
        &mut waiting,
        &mut spatial,
        &mut visits,
        &mut clear,
        route_quota,
        !guided,
    );
    for &index in &order {
        let unit = &frame.units[index];
        if waiting.contains(&unit.unit_id) {
            stopped(proposed, index);
            continue;
        }
        if locally_guided[index] || !neighbors.has(index) {
            continue;
        }
        let plan = state.members[&unit.unit_id].lazy_plan;
        let gate_key = state
            .gates
            .iter()
            .find(|g| g.admitted.contains(&unit.unit_id) && gate_local(g, unit))
            .map(|g| (g.centre, g.axis, g.direction));
        if let lazy::Plan::Keep(point) = plan {
            if let Some(point) = point {
                proposed.units[index].target = Some(point);
                proposed.units[index].speed = unit.speed;
            }
            continue;
        }
        let direction = heading(unit);
        for &other in &neighbors[&index] {
            let peer = &frame.units[other];
            if other == index
                || peer.owner_slot != unit.owner_slot
                || !moving(peer)
                || direction.dot(heading(peer)) > -0.75
            {
                continue;
            }
            let delta = peer.position - unit.position;
            let reach = 4.0 * (unit.radius + peer.radius);
            if delta.length_squared() > reach * reach || delta.dot(direction) <= 0.0 {
                continue;
            }
            let axis = canonical(delta.normalize_or_zero());
            if axis == Vec2::ZERO {
                continue;
            }
            let centre = (unit.position + peer.position) * 0.5;
            let lateral = unit.radius + peer.radius;
            if let lazy::Plan::Fresh {
                peer: chosen,
                point,
            } = plan
                && chosen == peer.unit_id
            {
                state.members.get_mut(&unit.unit_id).unwrap().passing =
                    gate_key.map(|(centre, axis, direction)| Passing {
                        point,
                        centre,
                        axis,
                        direction,
                    });
                proposed.units[index].target = Some(point);
                proposed.units[index].speed = unit.speed;
                break;
            }
            if state.gates.iter().any(|gate| {
                if gate.admitted.public() {
                    return false;
                }
                gate.axis.dot(axis) > 0.8 && gate.centre.distance(centre) < gate.half_length
            }) {
                continue;
            }
            let segment = |index: usize| {
                let actor = &frame.units[index];
                let guidance = &proposed.units[index];
                let endpoint = guidance.target.map_or(actor.position, |target| {
                    actor.position
                        + (target - actor.position)
                            .clamp_length_max(guidance.speed.unwrap_or(0.0) * dt)
                });
                (actor.position, endpoint)
            };
            let (from, to) = segment(index);
            let left = contact(index, from, to, axis);
            let (from, to) = segment(other);
            let right = contact(other, from, to, axis);
            for (actor, observed) in [(unit, left), (peer, right)] {
                if observed == EntranceContact::Inside
                    && let Some(member) = state.members.get_mut(&actor.unit_id)
                    && let Some(stamp) = EntranceStamp::new(tick, actor.position, observed)
                {
                    member.entrance.get_or_insert(stamp);
                }
            }
            let historical = |actor: &MovementFrameUnit| {
                state
                    .members
                    .get(&actor.unit_id)
                    .and_then(|member| member.entrance)
            };
            let priority = if left == EntranceContact::Unresolved
                || right == EntranceContact::Unresolved
            {
                None
            } else {
                match (historical(unit), historical(peer)) {
                    (Some(a), Some(b)) if a.tick != b.tick => Some(a.tick.cmp(&b.tick)),
                    (Some(a), Some(b)) => contact_order(a.contact(), b.contact()).map(|ordering| {
                        if ordering == Ordering::Equal {
                            point_cmp(a.point, b.point)
                        } else {
                            ordering
                        }
                    }),
                    (Some(a), None) if a.tick < tick && right != EntranceContact::Unresolved => {
                        Some(Ordering::Less)
                    }
                    (None, Some(b)) if b.tick < tick && left != EntranceContact::Unresolved => {
                        Some(Ordering::Greater)
                    }
                    _ => contact_order(left, right).map(|ordering| {
                        if ordering == Ordering::Equal {
                            physical(unit, peer)
                        } else {
                            ordering
                        }
                    }),
                }
            };
            let Some(priority) = priority else {
                // No future reservation is taken for Outside/unknown contact,
                // and overlapping bounds never turn into an ID or ETA winner.
                continue;
            };
            if priority == Ordering::Equal {
                continue;
            }
            let winner = if priority == Ordering::Less {
                unit
            } else {
                peer
            };
            let mut reserved_by = [false; SLOTS];
            for owner in [usize::from(unit.owner_slot), usize::from(peer.owner_slot)] {
                if !reserved_by[owner] && reservations[owner] < OWNER_GATES {
                    reserved_by[owner] = true;
                    reservations[owner] += 1;
                }
            }
            if !reserved_by.iter().any(|reserved| *reserved) {
                waiting.insert(unit.unit_id);
                stopped(proposed, index);
                break;
            }
            state.gates.push(Gate {
                centre,
                axis,
                half_length: lateral * 2.0 + 2.0,
                radius: unit.radius.max(peer.radius),
                direction: heading(winner).dot(axis).signum(),
                admitted: GateRows::new(),
                reserved_by,
                provisional_for: state
                    .members
                    .get(&winner.unit_id)
                    .and_then(|member| member.entrance)
                    .is_none()
                    .then_some(winner.unit_id),
                issued: 0,
                blocked_contact: None,
            });
        }
    }
    state
        .gates
        .sort_by(|a, b| point_cmp(a.centre, b.centre).then_with(|| point_cmp(a.axis, b.axis)));
    let mut row_credit =
        (GATES * BATCH).saturating_sub(state.gates.iter().map(|g| g.admitted.len()).sum());
    let mut owner_row_credit = std::array::from_fn::<_, SLOTS, _>(|owner| {
        (OWNER_GATES * BATCH).saturating_sub(
            state
                .gates
                .iter()
                .filter(|g| g.reserved_by[owner])
                .map(|g| g.admitted.len())
                .sum(),
        )
    });
    for gate in &mut state.gates {
        if gate.admitted.public() {
            continue;
        }
        let mut admitted = std::mem::take(&mut gate.admitted);
        admitted.retain(|id| {
            live.get(id).is_some_and(|unit| {
                let passed = (unit.position - gate.centre).dot(gate.axis) * gate.direction
                    > gate.half_length;
                // A lateral retreat has left this physical resource as surely as
                // an axial exit. Retaining that reservation can couple two gates
                // forever while each actor waits outside the gate it owns.
                let lateral = (unit.position - gate.centre).perp_dot(gate.axis).abs();
                let body_reach = (4.0 * gate.radius + unit.radius * gate.axis.length()).next_up();
                !passed
                    && moving(unit)
                    && lateral <= body_reach
                    && gate
                        .reserved_by
                        .get(usize::from(unit.owner_slot))
                        .copied()
                        .unwrap_or(false)
            })
        });
        gate.admitted = admitted;
        let near = order
            .iter()
            .copied()
            .filter(|&index| gate_local(gate, &frame.units[index]))
            .collect::<Vec<_>>();
        gate.blocked_contact = None;
        let unresolved = near.iter().any(|&index| {
            let unit = &frame.units[index];
            // Existing authority concerns actual occupancy, not a speculative
            // waypoint chord. A bad proposed path must not veto safe admitted
            // peers; the final terrain/body guards still certify that path.
            let unresolved = contact(index, unit.position, unit.position, gate.axis)
                == EntranceContact::Unresolved;
            if unresolved {
                gate.blocked_contact = Some((unit.unit_id, unit.position, unit.position));
            }
            unresolved
        });
        if unresolved {
            // Pending lease checks hold the local gate without erasing its
            // committed timestamps or using them to grant new authority.
            for &index in &near {
                waiting.insert(frame.units[index].unit_id);
                stopped(proposed, index);
            }
            continue;
        }
        let opposite = near
            .iter()
            .any(|&index| heading(&frame.units[index]).dot(gate.axis) * gate.direction < -0.5);
        let same = near
            .iter()
            .any(|&index| heading(&frame.units[index]).dot(gate.axis) * gate.direction > 0.5);
        if gate.admitted.is_empty() && opposite && (gate.issued >= BATCH || !same) {
            gate.direction = -gate.direction;
            gate.issued = 0;
        } else if gate.admitted.is_empty() && !opposite {
            // Four is a fairness turn, not a lifetime limit. With no opposition,
            // the following physical batch must retain service.
            gate.issued = 0;
        }
        let mut candidates = near
            .iter()
            .copied()
            .filter(|&index| {
                !waiting.contains(&frame.units[index].unit_id)
                    && heading(&frame.units[index]).dot(gate.axis) * gate.direction > 0.5
                    && !gate.admitted.contains(&frame.units[index].unit_id)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|&a, &b| {
            let left = &frame.units[a];
            let right = &frame.units[b];
            let age = |unit: &MovementFrameUnit| {
                state
                    .members
                    .get(&unit.unit_id)
                    .and_then(|member| member.entrance)
                    .map_or(tick, |entry| entry.tick)
            };
            age(left)
                .cmp(&age(right))
                .then_with(|| {
                    let distance = |unit: &MovementFrameUnit| {
                        (gate.half_length
                            - (unit.position - gate.centre).dot(gate.axis) * gate.direction)
                            .max(0.0)
                    };
                    distance(left).total_cmp(&distance(right))
                })
                .then_with(|| physical(left, right))
        });
        for index in candidates {
            if gate.issued == BATCH {
                break;
            }
            let unit = &frame.units[index];
            let guidance = &proposed.units[index];
            let to = guidance.target.map_or(unit.position, |target| {
                unit.position
                    + (target - unit.position).clamp_length_max(guidance.speed.unwrap_or(0.0) * dt)
            });
            if contact(index, unit.position, to, gate.axis) == EntranceContact::Unresolved {
                continue;
            }
            let owner = usize::from(unit.owner_slot);
            if row_credit == 0 || owner_row_credit[owner] == 0 {
                continue;
            }
            if gate.admitted.insert(unit.unit_id) {
                row_credit -= 1;
                owner_row_credit[owner] -= 1;
            }
            gate.issued += 1;
        }
        for index in near {
            let unit = &frame.units[index];
            if gate.admitted.contains(&unit.unit_id) || waiting.contains(&unit.unit_id) {
                continue;
            }
            waiting.insert(unit.unit_id);
            stopped(proposed, index);
            let side = -(heading(unit).dot(gate.axis)).signum();
            state.members.get_mut(&unit.unit_id).unwrap().lazy_plan = lazy::Plan::Bay(lazy::Bay {
                mouth: gate.centre + gate.axis * side * (gate.half_length + unit.radius),
                normal: Vec2::new(-gate.axis.y, gate.axis.x),
                radius: gate.radius,
            });
        }
    }
    state.gates.retain(|gate| {
        gate.admitted.public()
            || order
                .iter()
                .any(|&index| gate_local(gate, &frame.units[index]))
    });
    lazy::park(
        frame,
        proposed,
        dt,
        tick,
        state,
        &mut service,
        &mut spatial,
        &mut visits,
        &mut clear,
        route_quota,
    );
    parking::apply(
        frame,
        proposed,
        state,
        &mut service,
        tick,
        &waiting,
        &mut spatial,
        &mut visits,
        route_quota,
        &mut public_goal,
        &mut settled,
        &mut eligible,
        &mut clear,
        pair_budget,
    );
    state.index_work = early_index_work;
    state.index_work.add(spatial.work);
    drop(spatial);
    drop(neighbors);
    if !guided {
        observed_detour::apply(
            frame,
            proposed,
            dt,
            tick,
            state,
            &mut service,
            &waiting,
            &mut visits,
            &mut clear,
            pair_budget,
            &mut observed,
            &mut observed_eligible,
            &mut observed_rows,
            route_quota,
        );
    }
    let final_visits = visits.iter().sum::<usize>();
    state.neighbor_visits = state
        .neighbor_visits
        .saturating_add((final_visits - state.tick_neighbors) as u64);
    state.tick_neighbors = final_visits;
    state.tick_owner_neighbors = visits;
    state.index_work_total.add(state.index_work);
    waiting
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unit(id: u32, position: Vec2, target: Option<Vec2>) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(2.0),
            target,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn exact_contact(
        grid: turts_rules::terrain::geometry::Grid,
        surface: &impl turts_rules::terrain::geometry::Surface,
        from: Vec2,
        to: Vec2,
        axis: Vec2,
        radius: f32,
    ) -> EntranceContact {
        use turts_rules::terrain::geometry::{Budget, Point};
        let point = |value: Vec2| Point {
            x: f64::from(value.x),
            y: f64::from(value.y),
        };
        let reach = 2.0 * f64::from(radius);
        let offset = Point {
            x: -f64::from(axis.y) * reach,
            y: f64::from(axis.x) * reach,
        };
        grid.first_dual_support_contact(
            surface,
            point(from),
            point(to),
            [
                offset,
                Point {
                    x: -offset.x,
                    y: -offset.y,
                },
            ],
            f64::from(radius),
            0,
            &mut Budget::new(65_536),
        )
    }
    fn frame(units: Vec<MovementFrameUnit>) -> MovementFrame {
        MovementFrame {
            bounds: Vec2::splat(30.0),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        }
    }
    #[test]
    fn open_opposing_traffic_steers_to_opposite_sides_under_allocation_and_enumeration_reversal() {
        fn proposals(reversed: bool) -> Vec<(Vec2, Option<Vec2>)> {
            let mut units = vec![
                unit(
                    if reversed { 99 } else { 1 },
                    Vec2::new(8.0, 10.0),
                    Some(Vec2::new(20.0, 10.0)),
                ),
                unit(
                    if reversed { 1 } else { 99 },
                    Vec2::new(12.0, 10.0),
                    Some(Vec2::new(2.0, 10.0)),
                ),
            ];
            if reversed {
                units.reverse();
            }
            let original = frame(units);
            let mut proposed = original.clone();
            let waiting = apply(
                &original,
                &mut proposed,
                0.05,
                1,
                &mut TrafficState::default(),
                |_, _, _| true,
            );
            assert!(waiting.is_empty());
            let mut result = proposed
                .units
                .iter()
                .map(|unit| (unit.position, unit.target))
                .collect::<Vec<_>>();
            result.sort_by(|a, b| point_cmp(a.0, b.0));
            assert!(result[0].1.unwrap().y > 10.0);
            assert!(result[1].1.unwrap().y < 10.0);
            result
        }
        assert_eq!(proposals(false), proposals(true));
    }
    #[test]
    fn dense_query_overflow_stops_all_affected_bodies_without_removing_obstacles() {
        let original = frame(
            (0..130)
                .map(|id| unit(id, Vec2::new(8.0, 8.0), Some(Vec2::new(20.0, 8.0))))
                .collect(),
        );
        let mut proposed = original.clone();
        let mut state = TrafficState::default();
        let waiting = apply(&original, &mut proposed, 0.05, 1, &mut state, |_, _, _| {
            true
        });
        assert_eq!(waiting.len(), 130);
        assert!(state.neighbor_visits <= VISITS as u64);
        assert_eq!(proposed.units.len(), original.units.len());
        for (before, after) in original.units.iter().zip(&proposed.units) {
            assert_eq!(before.position, after.position);
            assert_eq!(before.radius, after.radius);
            assert_eq!(after.speed, Some(0.0));
            assert_eq!(after.target, None);
        }
    }
    #[test]
    fn global_neighbor_budget_rotates_service_under_id_and_enumeration_reversal() {
        fn run(invert: bool) -> Vec<Vec<(u32, u32)>> {
            let mut units = Vec::new();
            for cluster in 0..10_u32 {
                for cell in 0..100_u32 {
                    let key = cluster * 100 + cell;
                    let point = Vec2::new(
                        20.01 + cluster as f32 * 20.0 + (cell % 10) as f32 * 0.0401,
                        20.01 + (cell / 10) as f32 * 0.0401,
                    );
                    let mut actor = unit(
                        if invert { 999 - key } else { key },
                        point,
                        Some(point + Vec2::X),
                    );
                    actor.radius = 0.02;
                    actor.speed = Some(0.01);
                    units.push(actor);
                }
            }
            if invert {
                units.reverse();
            }
            let mut original = frame(units);
            original.bounds = Vec2::new(240.0, 60.0);
            let mut state = TrafficState::default();
            let mut completed = BTreeSet::new();
            let mut trace = Vec::new();
            // Genuine relevant neighbors exhaust service: centres remain farther
            // apart than diameter .04, while the local query reach is .16.
            // The former .09/.01 fixture exhausted only coarse grid aliases.
            // Two bounded fair rotations cover incomplete attempts at a quota
            // boundary after the public per-slot split. The old four-tick
            // assumption used the unsplit global budget.
            let ticks = 2 * (1000_usize * 100).div_ceil(OWNER_VISITS) as u32;
            for tick in 1..=ticks {
                let before = state.neighbor_visits;
                let waiting = apply(
                    &original,
                    &mut original.clone(),
                    0.05,
                    tick,
                    &mut state,
                    |_, _, _| true,
                );
                assert!(state.neighbor_visits - before <= VISITS as u64);
                assert!(
                    !waiting.is_empty(),
                    "fixture must exhaust the global budget"
                );
                let mut served = original
                    .units
                    .iter()
                    .filter(|actor| !waiting.contains(&actor.unit_id))
                    .map(|actor| (actor.position.x.to_bits(), actor.position.y.to_bits()))
                    .collect::<Vec<_>>();
                served.sort();
                completed.extend(served.iter().copied());
                trace.push(served);
            }
            assert_eq!(
                completed.len(),
                1000,
                "repeated leftmost traffic must not starve the other actors"
            );
            trace
        }
        assert_eq!(run(false), run(true));
    }

    #[test]
    fn sparse_fast_outlier_does_not_turn_remote_bodies_into_local_query_overflow() {
        let mut units = Vec::new();
        for id in 0..16 {
            let point = Vec2::new(4.0 + (id % 4) as f32 * 1.2, 4.0 + (id / 4) as f32 * 1.2);
            let mut actor = unit(id, point, Some(point + Vec2::X * 10.0));
            actor.radius = 0.2;
            units.push(actor);
        }
        for id in 0..200 {
            let mut body = unit(
                100 + id,
                Vec2::new(16.0 + (id % 20) as f32 * 0.3, 16.0 + (id / 20) as f32 * 0.3),
                None,
            );
            body.radius = 0.1;
            units.push(body);
        }
        let mut fast = unit(999, Vec2::new(100.0, 100.0), Some(Vec2::new(150.0, 100.0)));
        fast.radius = 0.2;
        fast.speed = Some(100.0);
        units.push(fast);
        let mut original = frame(units);
        original.bounds = Vec2::splat(200.0);
        let mut state = TrafficState::default();
        let waiting = apply(
            &original,
            &mut original.clone(),
            0.05,
            1,
            &mut state,
            |_, _, _| true,
        );
        assert!(
            waiting.is_empty(),
            "remote static bodies and a fast outlier are not local congestion: {waiting:?}"
        );
        assert_eq!(state.neighbor_overflows, 0);
    }

    #[test]
    fn spatial_query_includes_blockers_at_the_end_of_a_fast_actor_sweep() {
        let mut actor = unit(1, Vec2::new(10.0, 10.0), Some(Vec2::new(24.0, 10.0)));
        actor.speed = Some(100.0);
        let original = frame(vec![actor, unit(2, Vec2::new(15.5, 10.0), None)]);
        let mut spatial = Spatial::new(&original);
        let mut visits = 0;
        let neighbors = spatial
            .chord(&original, 0, Vec2::new(15., 10.), &mut visits, VISITS)
            .unwrap();
        assert!(neighbors.contains(&1));
        assert!(
            !candidate_clear(
                &original,
                0,
                Vec2::new(15.0, 10.0),
                &neighbors,
                &mut |_, _, _| true
            ),
            "the distant end of an actual steering sweep must retain its blocker"
        );
    }

    #[test]
    fn settled_holder_is_never_given_a_yield_target() {
        let original = frame(vec![
            unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(20.0, 10.0))),
            unit(2, Vec2::new(12.0, 10.0), None),
        ]);
        let mut proposed = original.clone();
        apply(
            &original,
            &mut proposed,
            0.05,
            1,
            &mut TrafficState::default(),
            |_, _, _| true,
        );
        assert_eq!(proposed.units[1], original.units[1]);
    }
    #[test]
    fn cancellation_owner_change_and_match_reset_release_retained_memberships() {
        let mut original = frame(vec![unit(
            1,
            Vec2::new(8.0, 10.0),
            Some(Vec2::new(20.0, 10.0)),
        )]);
        let mut state = TrafficState::default();
        apply(
            &original,
            &mut original.clone(),
            0.05,
            7,
            &mut state,
            |_, _, _| true,
        );
        state.members.get_mut(&1).unwrap().entrance =
            EntranceStamp::new(4, Vec2::new(7.0, 10.0), EntranceContact::Inside);
        state.forget(1);
        assert!(state.members.is_empty());
        apply(
            &original,
            &mut original.clone(),
            0.05,
            8,
            &mut state,
            |_, _, _| true,
        );
        original.units[0].owner_slot = 1;
        apply(
            &original,
            &mut original.clone(),
            0.05,
            9,
            &mut state,
            |_, _, _| true,
        );
        assert_eq!(state.members[&1].owner, 1);
        assert_eq!(state.members[&1].entrance, None);
        original.units[0].target = None;
        apply(
            &original,
            &mut original.clone(),
            0.05,
            10,
            &mut state,
            |_, _, _| true,
        );
        assert!(state.members.is_empty());
        state.neighbor_overflows = 5;
        apply(
            &original,
            &mut original.clone(),
            0.05,
            0,
            &mut state,
            |_, _, _| true,
        );
        assert_eq!(state.neighbor_overflows, 0);
    }
    #[test]
    fn exact_indistinguishable_starts_are_contested_even_when_ids_differ() {
        let original = frame(vec![
            unit(99, Vec2::new(8.0, 10.0), Some(Vec2::new(20.0, 10.0))),
            unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(20.0, 10.0))),
        ]);
        let mut proposed = original.clone();
        let waiting = apply(
            &original,
            &mut proposed,
            0.05,
            1,
            &mut TrafficState::default(),
            |_, _, _| true,
        );
        assert_eq!(waiting, BTreeSet::from([1, 99]));
        assert!(
            proposed
                .units
                .iter()
                .all(|unit| unit.target.is_none() && unit.speed == Some(0.0))
        );
    }
    #[test]
    fn drained_batch_keeps_serving_when_no_opposing_actor_remains() {
        let original = frame(vec![unit(
            1,
            Vec2::new(8.0, 10.0),
            Some(Vec2::new(24.0, 10.0)),
        )]);
        let mut proposed = original.clone();
        let mut state = TrafficState::default();
        state.gates.push(Gate {
            centre: Vec2::new(10.0, 10.0),
            axis: Vec2::X,
            half_length: 4.0,
            radius: 0.5,
            direction: 1.0,
            admitted: GateRows::new(),
            reserved_by: std::array::from_fn(|owner| owner == 0),
            provisional_for: None,
            issued: BATCH,
            blocked_contact: None,
        });
        let waiting = apply_with_contacts(
            &original,
            &mut proposed,
            0.05,
            7,
            &mut state,
            |_, _, _| true,
            |_, _, _, _| EntranceContact::Inside,
        );
        assert!(
            waiting.is_empty(),
            "a completed fairness turn cannot stop the only remaining side"
        );
        assert!(state.gates[0].admitted.contains(&1));
    }

    #[test]
    fn single_file_direction_and_first_batch_depend_on_physical_entrance_not_ids() {
        fn crossing(reverse: bool) -> Vec<(Vec2, bool)> {
            let mut units = Vec::new();
            for n in 0..6 {
                units.push(unit(
                    if reverse { 100 - n } else { n },
                    Vec2::new(8.0 - n as f32 * 1.1, 10.0),
                    Some(Vec2::new(24.0, 10.0)),
                ));
            }
            units.push(unit(
                if reverse { 0 } else { 100 },
                Vec2::new(12.0, 10.0),
                Some(Vec2::new(2.0, 10.0)),
            ));
            if reverse {
                units.reverse();
            }
            let original = frame(units);
            let mut proposed = original.clone();
            let mut state = TrafficState::default();
            // Both lateral directions are unavailable inside this synthetic single-file
            // corridor. The solver remains responsible for actual movement and collisions.
            apply_with_contacts(
                &original,
                &mut proposed,
                0.05,
                1,
                &mut state,
                |_, from, to| from.y == to.y,
                |_, _, _, _| EntranceContact::Inside,
            );
            assert!(!state.gates.is_empty());
            assert!(state.gates.iter().all(|gate| gate.admitted.len() <= BATCH));
            let mut result = original
                .units
                .iter()
                .map(|unit| {
                    (
                        unit.position,
                        state
                            .gates
                            .iter()
                            .any(|gate| gate.admitted.contains(&unit.unit_id)),
                    )
                })
                .collect::<Vec<_>>();
            result.sort_by(|a, b| point_cmp(a.0, b.0));
            result
        }
        assert_eq!(crossing(false), crossing(true));
    }
    #[test]
    fn opposing_pair_uses_passing_bay_and_both_finish_without_overlap() {
        fn supported(from: Vec2, to: Vec2) -> bool {
            use turts_rules::terrain::{
                Ratio,
                geometry::{Budget, Cell, Grid, Point, Traversal},
            };
            let grid = Grid {
                columns: 30,
                rows: 30,
                length_scale: 1000,
                cell_width: 1000,
                elevation_step: 250,
                floor: 0,
                ceiling: 8,
            };
            let surface = |cell: Cell| {
                Some(if cell.x == 10 && !(9..12).contains(&cell.y) {
                    4
                } else {
                    0
                })
            };
            let point = |value: Vec2| Point {
                x: f64::from(value.x),
                y: f64::from(value.y),
            };
            let sweep = grid.sweep(
                &surface,
                point(from),
                point(to),
                1.125,
                Traversal {
                    max_step: 0,
                    max_grade: Ratio {
                        numerator: 0,
                        denominator: 1,
                    },
                },
                &mut Budget::new(65_536),
            );
            sweep.result.is_ok() && sweep.numerator == sweep.denominator
        }
        fn actual_world() -> bevy_ecs::world::World {
            use turts_mod_api::{
                ResolvedTerrainDatumBand, ResolvedTerrainDef, ResolvedTerrainMaterial,
                ResolvedTerrainWeightedMaterial, TerrainCell, TerrainHeightRect,
                TerrainPlacedRemoval, TerrainRect,
            };
            use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
            let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
            let mut catalog = archive.content().unwrap();
            let id = turts_mod_api::ContentId::parse("base:traffic_fixture_stone").unwrap();
            catalog.terrain_materials.insert(
                id.clone(),
                ResolvedTerrainMaterial {
                    id: id.clone(),
                    name: "Fixture stone".into(),
                    surface_color: "#888888".into(),
                    hardness: 1,
                    work_per_world_cube: turts_mod_api::Ratio {
                        numerator: 4,
                        denominator: 1,
                    },
                    natural_outputs: vec![],
                    fill_inputs: vec![],
                    placed_removal: TerrainPlacedRemoval::NoYield,
                },
            );
            let wall = |min, max| TerrainHeightRect {
                cells: TerrainRect {
                    min: TerrainCell { x: 10, y: min },
                    max: TerrainCell { x: 11, y: max },
                },
                elevation: 4,
            };
            let state = super::super::prepare(
                &catalog,
                &ResolvedTerrainDef {
                    columns: 30,
                    rows: 30,
                    length_scale: 1000,
                    cell_width: 1000,
                    elevation_step: 250,
                    chunk_width: 4,
                    initial_elevation: 0,
                    initial_heights: vec![wall(0, 9), wall(12, 30)],
                    floor: 0,
                    ceiling: 8,
                    generation: vec![ResolvedTerrainDatumBand {
                        bottom: 0,
                        top: 4,
                        choices: vec![ResolvedTerrainWeightedMaterial {
                            material: id,
                            weight: 1,
                        }],
                    }],
                },
                super::super::PrivateTerrainSeed::new(77),
            )
            .unwrap();
            let actor = catalog
                .units
                .get_mut(&turts_mod_api::ContentId::parse("base:harvester").unwrap())
                .unwrap();
            actor.footprint = turts_mod_api::Footprint::Circle { radius: 1.125 };
            actor.movement = Some(turts_mod_api::MovementDef { speed: 2.0 });
            let types =
                crate::UnitTypes::from_catalog(&catalog, &crate::UnitTypes::default()).unwrap();
            let mut world = bevy_ecs::world::World::new();
            world.insert_resource(types);
            world.insert_resource(crate::bootstrap::InstalledCatalog(catalog));
            world.insert_resource(state);
            world.insert_resource(crate::Tick(0));
            super::super::phase::initialize_phases(&mut world);
            world
        }
        fn run(invert: bool, layout: u8) -> Vec<Vec<Vec2>> {
            let mut units = vec![
                unit(
                    if invert { 99 } else { 1 },
                    Vec2::new(
                        match layout {
                            1 => 8.875,
                            2 => 9.25,
                            _ => 8.0,
                        },
                        10.5,
                    ),
                    Some(Vec2::new(24.0, 10.5)),
                ),
                unit(
                    if invert { 1 } else { 99 },
                    Vec2::new(
                        match layout {
                            1 => 12.125,
                            2 => 11.5,
                            _ => 12.0,
                        },
                        10.5,
                    ),
                    Some(Vec2::new(2.0, 10.5)),
                ),
            ];
            // One1.125-radius body fits the3-wide lane; two cannot pass
            // abreast. Binary-exact mouth tangencies isolate exact t=0+.
            for unit in &mut units {
                unit.radius = 1.125;
            }
            assert!(2.0 * units[0].radius <= 3.0 && 4.0 * units[0].radius > 3.0);
            if invert {
                units.reverse();
            }
            let destinations = units
                .iter()
                .map(|unit| (unit.unit_id, unit.target.unwrap()))
                .collect::<BTreeMap<_, _>>();
            let mut current = frame(units);
            let mut world = actual_world();
            let kind = world
                .resource::<crate::UnitTypes>()
                .find("base:harvester")
                .unwrap();
            for unit in &mut current.units {
                unit.unit_type = kind.0;
            }
            let mut trace = Vec::new();
            let mut reported_endpoint_changes = BTreeSet::new();
            for tick in 0..600 {
                world.resource_mut::<crate::Tick>().0 = tick;
                super::super::phase::capture_phase(
                    &mut world,
                    turts_mod_api::ObservationPhase::Start,
                );
                // This is the production route/controller/collision/terrain retry
                // path, not a raw collision proposal or a weakened pose check.
                let (outputs, _) =
                    super::super::navigation::step(&mut world, &current, 0.05, &BTreeSet::new());
                for output in outputs {
                    let actor = current
                        .units
                        .iter_mut()
                        .find(|actor| actor.unit_id == output.unit_id)
                        .unwrap();
                    assert!(
                        supported(actor.position, output.position),
                        "uncertified traffic segment at tick {tick}"
                    );
                    let destination = destinations[&actor.unit_id];
                    if output.target != Some(destination)
                        && output.position != destination
                        && reported_endpoint_changes.insert(actor.unit_id)
                    {
                        eprintln!(
                            "first endpoint change layout={layout} tick={tick} unit={} from={:?} destination={destination:?} output={output:?} nav={:#?}",
                            actor.unit_id,
                            actor.position,
                            super::super::navigation::privileged_navigation_audit(&world)
                        );
                    }
                    actor.position = output.position;
                    actor.target = output.target;
                    actor.progress = output.progress;
                    actor.plain_progress = output.plain_progress;
                    actor.physical_escape = output.physical_escape;
                    if actor
                        .target
                        .is_some_and(|goal| goal.distance_squared(actor.position) < 1e-6)
                    {
                        actor.target = None;
                    }
                }
                assert!(
                    current.units[0]
                        .position
                        .distance(current.units[1].position)
                        >= 2.25 - 1e-5,
                    "overlap at tick {tick}"
                );
                let mut points = current
                    .units
                    .iter()
                    .map(|actor| actor.position)
                    .collect::<Vec<_>>();
                points.sort_by(|a, b| point_cmp(*a, *b));
                trace.push(points);
            }
            if current.units.iter().any(|actor| actor.target.is_some()) {
                let traffic = world.resource::<TrafficState>();
                eprintln!(
                    "traffic diagnostic layout={layout}: gates={:?}, entrances={:?},nav={:#?}",
                    traffic
                        .gates
                        .iter()
                        .map(|gate| (
                            gate.centre,
                            gate.direction,
                            gate.issued,
                            &gate.admitted,
                            gate.provisional_for
                        ))
                        .collect::<Vec<_>>(),
                    traffic
                        .members
                        .iter()
                        .map(|(id, member)| (*id, member.entrance))
                        .collect::<Vec<_>>(),
                    super::super::navigation::privileged_navigation_audit(&world)
                );
                eprintln!(
                    "field status {:?}",
                    super::super::navigation::debug_field_status(&world)
                );
            }
            assert!(
                current.units.iter().all(|actor| actor.target.is_none()
                    && actor.position == destinations[&actor.unit_id]),
                "feasible opposing passage did not finish, layout={layout}, units={:?}",
                current.units
            );
            trace
        }
        for layout in 0..3 {
            // Layout1 starts at the two exact binary-representable mouth
            // tangencies: both enter at t=0+. Layout2 starts both Inside.
            // Both have passing bays and must finish; stable contested traffic
            // cannot substitute for progress in this feasible geometry.
            assert_eq!(
                run(false, layout),
                run(true, layout),
                "opaque allocation must not alter any physical step in layout {layout}"
            );
        }
    }
    #[test]
    fn hard_fenced_intent_cannot_regain_a_target_or_speed_from_an_opposing_peer() {
        let mut fenced = unit(1, Vec2::new(8.0, 10.0), None);
        fenced.speed = None;
        let original = frame(vec![
            fenced,
            unit(2, Vec2::new(12.0, 10.0), Some(Vec2::new(2.0, 10.0))),
        ]);
        let mut proposed = original.clone();
        let mut state = TrafficState::default();
        apply(&original, &mut proposed, 0.05, 1, &mut state, |_, _, _| {
            true
        });
        assert_eq!(proposed.units[0], original.units[0]);
        assert!(!state.members.contains_key(&1));
    }

    #[test]
    fn soft_pending_guidance_can_take_only_a_certified_local_yield() {
        let original = frame(vec![
            unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(20.0, 10.0))),
            unit(2, Vec2::new(12.0, 10.0), Some(Vec2::new(2.0, 10.0))),
        ]);
        let mut proposed = original.clone();
        stopped(&mut proposed, 0);
        apply(
            &original,
            &mut proposed,
            0.05,
            1,
            &mut TrafficState::default(),
            |_, _, _| true,
        );
        assert!(proposed.units[0].target.is_some());
        assert_eq!(proposed.units[0].speed, original.units[0].speed);
        assert_eq!(original.units[0].target, Some(Vec2::new(20.0, 10.0)));
    }

    #[test]
    fn retained_member_pressure_stops_newcomers_and_keeps_their_full_bodies() {
        let original = frame(
            (0..=MEMBERS as u32)
                .map(|id| unit(id, Vec2::new(8.0, 10.0), Some(Vec2::new(20.0, 10.0))))
                .collect(),
        );
        let mut proposed = original.clone();
        let mut state = TrafficState::default();
        let waiting = apply(&original, &mut proposed, 0.05, 1, &mut state, |_, _, _| {
            true
        });
        assert_eq!(state.retained_members(), OWNER_MEMBERS);
        assert_eq!(proposed.units.len(), original.units.len());
        assert_eq!(
            proposed.units[MEMBERS].position,
            original.units[MEMBERS].position
        );
        assert_eq!(
            proposed.units[MEMBERS].radius,
            original.units[MEMBERS].radius
        );
        assert!(waiting.contains(&(MEMBERS as u32)));
        assert_eq!(proposed.units[MEMBERS].speed, Some(0.0));
        assert!(state.retained_bytes() + state.scratch_bytes() <= SCRATCH_BYTES);
        let (bodies, buckets) = state.tick_neighbor_work();
        assert!(bodies <= VISITS && buckets <= VISITS);
        assert!(state.retained_gates() <= GATES);
    }

    #[test]
    fn foreign_member_pressure_leaves_the_other_slots_reservation_available() {
        let mut units = (0..MEMBERS as u32 - 1)
            .map(|id| {
                let mut actor = unit(100 + id, Vec2::new(8.0, 10.0), Some(Vec2::new(20.0, 10.0)));
                actor.owner_slot = 1;
                actor
            })
            .collect::<Vec<_>>();
        units.push(unit(
            1,
            Vec2::new(1000.0, 10.0),
            Some(Vec2::new(1100.0, 10.0)),
        ));
        let mut original = frame(units);
        original.bounds = Vec2::new(1200.0, 60.0);
        let mut state = TrafficState::default();
        let waiting = apply(
            &original,
            &mut original.clone(),
            0.05,
            1,
            &mut state,
            |_, _, _| true,
        );
        assert!(state.members.contains_key(&1));
        assert!(!waiting.contains(&1));
        assert_eq!(
            state
                .members
                .values()
                .filter(|member| member.owner == 1)
                .count(),
            OWNER_MEMBERS
        );
        assert_eq!(
            state
                .members
                .values()
                .filter(|member| member.owner == 0)
                .count(),
            1
        );
        assert!(state.retained_bytes() + state.scratch_bytes() <= SCRATCH_BYTES);
    }

    #[test]
    fn distant_foreign_service_load_cannot_consume_a_protected_slots_quota() {
        type TraceRow = (Vec<(Vec2, Option<Vec2>, Option<f32>, bool)>, (usize, usize));
        fn protected_trace(foreign_active: bool) -> Vec<TraceRow> {
            let mut units = Vec::new();
            for cluster in 0..8_u32 {
                for cell in 0..100_u32 {
                    let point = Vec2::new(
                        20.01 + cluster as f32 * 20.0 + (cell % 10) as f32 * 0.09,
                        20.01 + (cell / 10) as f32 * 0.09,
                    );
                    let mut actor = unit(
                        100 + cluster * 100 + cell,
                        point,
                        foreign_active.then_some(point + Vec2::X),
                    );
                    actor.owner_slot = 1;
                    actor.radius = 0.01;
                    actor.speed = Some(0.01);
                    units.push(actor);
                }
            }
            for id in 0..16 {
                let point = Vec2::new(
                    1000.01 + (id % 4) as f32 * 0.09,
                    20.01 + (id / 4) as f32 * 0.09,
                );
                let mut actor = unit(id, point, Some(point + Vec2::X));
                actor.radius = 0.01;
                actor.speed = Some(0.01);
                units.push(actor);
            }
            let mut original = frame(units);
            original.bounds = Vec2::new(1200.0, 60.0);
            let mut state = TrafficState::default();
            let mut trace = Vec::new();
            for tick in 1..=4 {
                let mut proposed = original.clone();
                let waiting = apply(
                    &original,
                    &mut proposed,
                    0.05,
                    tick,
                    &mut state,
                    |_, _, _| true,
                );
                let facts = proposed
                    .units
                    .iter()
                    .filter(|actor| actor.owner_slot == 0)
                    .map(|actor| {
                        (
                            actor.position,
                            actor.target,
                            actor.speed,
                            waiting.contains(&actor.unit_id),
                        )
                    })
                    .collect::<Vec<_>>();
                assert!(facts.iter().all(|row| !row.3));
                let protected_work = state.tick_owner_neighbor_work(0).unwrap();
                assert!(protected_work.0 <= OWNER_VISITS && protected_work.1 <= OWNER_VISITS);
                let (bodies, buckets) = state.tick_neighbor_work();
                assert!(bodies <= VISITS && buckets <= VISITS);
                trace.push((facts, protected_work));
            }
            trace
        }
        assert_eq!(protected_trace(false), protected_trace(true));
    }

    #[test]
    fn distant_foreign_gate_saturation_does_not_change_a_protected_local_entrance() {
        fn local(saturated: bool) -> (f32, Vec<(Vec2, bool, bool)>) {
            let mut protected = unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0)));
            protected.owner_slot = 0;
            let mut peer = unit(2, Vec2::new(12.0, 10.0), Some(Vec2::new(2.0, 10.0)));
            peer.owner_slot = 0;
            let mut units = vec![protected, peer];
            let mut state = TrafficState::default();
            for i in 0..OWNER_GATES {
                let centre = Vec2::new(100.0 + i as f32 * 20.0, 10.0);
                let mut foreign = unit(
                    100 + i as u32,
                    centre - Vec2::X * 2.0,
                    Some(centre + Vec2::X * 20.0),
                );
                foreign.owner_slot = 1;
                units.push(foreign);
                if saturated {
                    state.gates.push(Gate {
                        centre,
                        axis: Vec2::X,
                        half_length: 4.0,
                        radius: 0.5,
                        direction: 1.0,
                        admitted: GateRows::new(),
                        reserved_by: std::array::from_fn(|slot| slot == 1),
                        provisional_for: None,
                        issued: 0,
                        blocked_contact: None,
                    });
                }
            }
            let mut original = frame(units);
            original.bounds = Vec2::new(1600.0, 30.0);
            let mut proposed = original.clone();
            let waiting = apply_with_contacts(
                &original,
                &mut proposed,
                0.05,
                7,
                &mut state,
                |_, from, to| from.y == to.y,
                |_, _, _, _| EntranceContact::Inside,
            );
            let gate = state
                .gates
                .iter()
                .find(|gate| gate.centre.x < 50.0)
                .expect("protected quota must admit its local physical gate");
            let facts = original
                .units
                .iter()
                .take(2)
                .map(|unit| {
                    (
                        unit.position,
                        waiting.contains(&unit.unit_id),
                        gate.admitted.contains(&unit.unit_id),
                    )
                })
                .collect();
            assert!(
                state
                    .gates
                    .iter()
                    .filter(|gate| gate.reserved_by[1])
                    .count()
                    <= OWNER_GATES
            );
            assert!(state.retained_bytes() + state.scratch_bytes() <= SCRATCH_BYTES);
            (gate.direction, facts)
        }
        assert_eq!(local(false), local(true));
    }

    #[test]
    fn outgoing_bodies_retire_gate_records_and_foreign_presence_cannot_pin_a_slots_credit() {
        let mut own = unit(1, Vec2::new(15.0, 10.0), Some(Vec2::new(24.0, 10.0)));
        own.owner_slot = 0;
        let mut foreign = unit(2, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0)));
        foreign.owner_slot = 1;
        let original = frame(vec![own, foreign]);
        let mut state = TrafficState::default();
        state.gates.push(Gate {
            centre: Vec2::new(10.0, 10.0),
            axis: Vec2::X,
            half_length: 4.0,
            radius: 0.5,
            direction: 1.0,
            admitted: GateRows::from([1]),
            reserved_by: [true; SLOTS],
            provisional_for: None,
            issued: 1,
            blocked_contact: None,
        });
        apply(
            &original,
            &mut original.clone(),
            0.05,
            7,
            &mut state,
            |_, _, _| true,
        );
        assert_eq!(state.gates.len(), 1);
        assert!(
            !state.gates[0].reserved_by[0],
            "distant foreign activity cannot retain an absent slot's quota"
        );
        assert!(state.gates[0].reserved_by[1]);
        assert!(!state.gates[0].admitted.contains(&1));
        assert!(state.gates[0].admitted.len() <= BATCH);
    }

    #[test]
    fn stored_inside_precedes_exact_zero_time_entry_without_losing_contact_kind() {
        let inside = EntranceStamp::new(4, Vec2::new(11.5, 10.5), EntranceContact::Inside).unwrap();
        let entering = EntranceStamp::new(
            4,
            Vec2::new(8.875, 10.5),
            EntranceContact::Transition {
                lower: 0,
                upper: 0,
                denominator: 1,
            },
        )
        .unwrap();
        // Entry at t=0+ is later than a body already positively inside at t=0.
        // Physical x ordering must not reverse that known temporal relationship.
        assert_eq!(inside.contact(), EntranceContact::Inside);
        assert_eq!(
            contact_order(inside.contact(), entering.contact()),
            Some(Ordering::Less)
        );
        assert_eq!(
            contact_order(entering.contact(), inside.contact()),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn contact_interval_order_never_uses_eta_or_guesses_overlapping_bounds() {
        let early = EntranceContact::Transition {
            lower: 1,
            upper: 2,
            denominator: 100,
        };
        let late = EntranceContact::Transition {
            lower: 7,
            upper: 9,
            denominator: 100,
        };
        assert_eq!(contact_order(early, late), Some(Ordering::Less));
        assert_eq!(contact_order(late, early), Some(Ordering::Greater));
        assert_eq!(
            contact_order(
                early,
                EntranceContact::Transition {
                    lower: 1,
                    upper: 3,
                    denominator: 100
                }
            ),
            None
        );
        assert_eq!(
            contact_order(EntranceContact::Outside, EntranceContact::Outside),
            None
        );
        assert_eq!(contact_order(EntranceContact::Unresolved, early), None);
        assert_eq!(
            contact_order(
                EntranceContact::Transition {
                    lower: u32::MAX - 1,
                    upper: u32::MAX,
                    denominator: u32::MAX
                },
                EntranceContact::Transition {
                    lower: 1,
                    upper: 1,
                    denominator: 2
                }
            ),
            Some(Ordering::Greater)
        );
        assert_eq!(
            contact_order(
                EntranceContact::Transition {
                    lower: 1,
                    upper: 2,
                    denominator: 0
                },
                early
            ),
            None
        );
    }

    #[test]
    fn only_actual_committed_contact_is_retained_for_cross_tick_precedence() {
        let original = frame(vec![
            unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0))),
            unit(2, Vec2::new(12.0, 10.0), Some(Vec2::new(2.0, 10.0))),
        ]);
        let mut state = TrafficState::default();
        let mut proposed = original.clone();
        apply_with_contacts(
            &original,
            &mut proposed,
            0.05,
            7,
            &mut state,
            |_, from, to| from.y == to.y,
            |_, _, _, _| EntranceContact::Transition {
                lower: 1,
                upper: 2,
                denominator: 10,
            },
        );
        assert!(
            state
                .members
                .values()
                .all(|member| member.entrance.is_none()),
            "a prediction is not actual entrance history"
        );
        let outputs = crate::movement::step_frame(&original, 0.05);
        // This unit test injects certified callback witnesses to isolate history
        // lifetime. The exact public corridor regression below uses real geometry.
        record_committed_contacts(&original, &outputs, 7, &mut state, |index, from, to, _| {
            if from == to {
                EntranceContact::Inside
            } else if index == 0 {
                EntranceContact::Transition {
                    lower: 8,
                    upper: 9,
                    denominator: 10,
                }
            } else {
                EntranceContact::Transition {
                    lower: 1,
                    upper: 2,
                    denominator: 10,
                }
            }
        });
        assert_eq!(state.members[&1].entrance.unwrap().tick, 7);
        assert_eq!(state.members[&2].entrance.unwrap().upper, 2);
        let mut next = original.clone();
        for output in outputs {
            next.units
                .iter_mut()
                .find(|unit| unit.unit_id == output.unit_id)
                .unwrap()
                .position = output.position;
        }
        apply_with_contacts(
            &next,
            &mut next.clone(),
            0.05,
            8,
            &mut state,
            |_, from, to| from.y == to.y,
            |_, _, _, _| EntranceContact::Inside,
        );
        assert!(!state.gates.is_empty());
        assert!(
            state.gates.iter().all(|gate| gate.direction < 0.0),
            "earlier committed right entrance must outrank left-side geometry"
        );
        state.forget(2);
        assert!(!state.members.contains_key(&2));
    }

    #[test]
    fn a_forecast_winner_that_did_not_enter_cannot_keep_its_gate() {
        let original = frame(vec![
            unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0))),
            unit(2, Vec2::new(12.0, 10.0), Some(Vec2::new(2.0, 10.0))),
        ]);
        let mut state = TrafficState::default();
        let mut proposed = original.clone();
        apply_with_contacts(
            &original,
            &mut proposed,
            0.05,
            7,
            &mut state,
            |_, from, to| from.y == to.y,
            |index, _, _, _| {
                if index == 0 {
                    EntranceContact::Transition {
                        lower: 1,
                        upper: 2,
                        denominator: 10,
                    }
                } else {
                    EntranceContact::Transition {
                        lower: 7,
                        upper: 8,
                        denominator: 10,
                    }
                }
            },
        );
        assert_eq!(state.gates.len(), 1);
        assert_eq!(state.gates[0].provisional_for, Some(1));
        let outputs = crate::movement::step_frame(&original, 0.05);
        // A final solver result without that certified entrance revokes the
        // provisional direction, even if its forecast was earlier.
        record_committed_contacts(&original, &outputs, 7, &mut state, |_, _, _, _| {
            EntranceContact::Outside
        });
        assert!(state.gates.is_empty());
        assert!(
            state
                .members
                .values()
                .all(|member| member.entrance.is_none())
        );
    }

    #[test]
    fn local_contact_lease_failure_keeps_service_age_and_distant_gate_history() {
        let actor = unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0)));
        let distant = unit(2, Vec2::new(100.0, 10.0), Some(Vec2::new(120.0, 10.0)));
        let mut original = frame(vec![actor.clone(), distant]);
        original.bounds = Vec2::splat(128.0);
        let mut state = TrafficState::default();
        apply(
            &original,
            &mut original.clone(),
            0.05,
            7,
            &mut state,
            |_, _, _| true,
        );
        for member in state.members.values_mut() {
            member.entrance = EntranceStamp::new(6, Vec2::new(8.0, 10.0), EntranceContact::Inside);
        }
        for (centre, id) in [(Vec2::new(10.0, 10.0), 1), (Vec2::new(100.0, 10.0), 2)] {
            state.gates.push(Gate {
                centre,
                axis: Vec2::X,
                half_length: 4.0,
                radius: 0.5,
                direction: 1.0,
                admitted: GateRows::from([id]),
                reserved_by: std::array::from_fn(|owner| owner == 0),
                provisional_for: None,
                issued: 1,
                blocked_contact: None,
            });
        }
        let age = (
            state.members[&1].accepted_tick,
            state.members[&1].attempts.get(Attempt::Neighbor),
        );
        state.invalidate_contact(&actor);
        assert!(state.members[&1].entrance.is_none());
        assert_eq!(
            (
                state.members[&1].accepted_tick,
                state.members[&1].attempts.get(Attempt::Neighbor)
            ),
            age
        );
        assert!(state.members[&2].entrance.is_some());
        assert_eq!(state.gates.len(), 1);
        assert_eq!(state.gates[0].centre, Vec2::new(100.0, 10.0));
    }

    #[test]
    fn uncertified_proposed_chord_does_not_freeze_existing_certified_entrants() {
        for reversed in [false, true] {
            let left = if reversed { 99 } else { 1 };
            let right = if reversed { 1 } else { 99 };
            let mut original = frame(vec![
                unit(left, Vec2::new(8., 10.), Some(Vec2::new(24., 10.))),
                unit(right, Vec2::new(12., 10.), Some(Vec2::new(2., 10.))),
                unit(50, Vec2::new(6., 10.), Some(Vec2::new(24., 10.))),
            ]);
            if reversed {
                original.units.reverse();
            }
            let mut state = TrafficState::default();
            state.gates.push(Gate {
                centre: Vec2::new(10., 10.),
                axis: Vec2::X,
                half_length: 4.,
                radius: 0.5,
                direction: 1.,
                admitted: GateRows::from([left]),
                reserved_by: [true; SLOTS],
                provisional_for: None,
                issued: 1,
                blocked_contact: None,
            });
            let mut proposed = original.clone();
            let waiting = apply_with_contacts(
                &original,
                &mut proposed,
                0.05,
                8,
                &mut state,
                |_, from, to| from.y == to.y,
                |_, from, to, _| {
                    if from == to {
                        EntranceContact::Inside
                    } else {
                        EntranceContact::Unresolved
                    }
                },
            );
            let winner = proposed.units.iter().find(|u| u.unit_id == left).unwrap();
            assert!(winner.speed.is_some());
            assert!(!waiting.contains(&left));
            assert!(state.gates[0].admitted.contains(&left));
            assert!(
                !state.gates[0].admitted.contains(&right),
                "uncertified new entry gains no authority"
            );
            assert!(
                !state.gates[0].admitted.contains(&50),
                "an unresolved same-direction chord cannot earn a new admission"
            );
            assert!(state.gates[0].blocked_contact.is_none());
        }
    }
    #[test]
    fn lateral_departures_release_each_gate_without_erasing_real_contact_history() {
        for reversed in [false, true] {
            let ids = if reversed { [99, 1] } else { [1, 99] };
            let mut original = frame(vec![
                unit(
                    ids[0],
                    Vec2::new(28.9834, 30.6732),
                    Some(Vec2::new(48., 23.5)),
                ),
                unit(
                    ids[1],
                    Vec2::new(25.0189, 24.6860),
                    Some(Vec2::new(48., 23.5)),
                ),
            ]);
            original.bounds = Vec2::splat(64.);
            if reversed {
                original.units.reverse();
            }
            let mut state = TrafficState::default();
            apply(
                &original,
                &mut original.clone(),
                0.05,
                7,
                &mut state,
                |_, _, _| true,
            );
            let stamp = EntranceStamp::new(6, Vec2::new(30., 23.), EntranceContact::Inside);
            for member in state.members.values_mut() {
                member.entrance = stamp;
            }
            for (centre, axis, half_length, id) in [
                (
                    Vec2::new(31.099745, 22.667612),
                    Vec2::new(0.99997056, -0.0076717027),
                    6.4,
                    ids[0],
                ),
                (
                    Vec2::new(31.56247, 25.54685),
                    Vec2::new(0.7251996, -0.68853873),
                    5.,
                    ids[1],
                ),
            ] {
                state.gates.push(Gate {
                    centre,
                    axis,
                    half_length,
                    radius: 1.1,
                    direction: 1.,
                    admitted: GateRows::from([id]),
                    reserved_by: [true; SLOTS],
                    provisional_for: None,
                    issued: BATCH,
                    blocked_contact: None,
                });
            }
            assert!(!gate_local(
                &state.gates[0],
                original.units.iter().find(|u| u.unit_id == ids[0]).unwrap()
            ));
            assert!(!gate_local(
                &state.gates[1],
                original.units.iter().find(|u| u.unit_id == ids[1]).unwrap()
            ));
            apply_with_contacts(
                &original,
                &mut original.clone(),
                0.05,
                8,
                &mut state,
                |_, from, to| from == to,
                |_, _, _, _| EntranceContact::Inside,
            );
            assert!(!state.gates[0].admitted.contains(&ids[0]));
            assert!(!state.gates[1].admitted.contains(&ids[1]));
            assert!(state.gates[0].admitted.contains(&ids[1]));
            assert!(state.gates[1].admitted.contains(&ids[0]));
            assert!(state.members.values().all(|m| m.entrance == stamp));
        }
    }
    #[test]
    fn admitted_passing_waypoint_does_not_chase_a_waiting_peers_changed_position() {
        println!(
            "TRAFFIC_ALLOCATION M={} U={} STATE={} GATE={} MEMBER={} PER_BODY={} FIXED={} RETAINED={}",
            INDEX_BODIES,
            std::mem::size_of::<MovementFrameUnit>(),
            std::mem::size_of::<TrafficState>(),
            std::mem::size_of::<Gate>(),
            std::mem::size_of::<Member>(),
            PER_BODY_SCRATCH,
            FIXED_SCRATCH,
            RETAINED_ALLOWANCE
        );
        for reversed in [false, true] {
            let ids = if reversed { [99, 1] } else { [1, 99] };
            let mut original = frame(vec![
                unit(ids[0], Vec2::new(8., 10.), Some(Vec2::new(24., 10.))),
                unit(ids[1], Vec2::new(12., 10.), Some(Vec2::new(2., 10.))),
            ]);
            if reversed {
                original.units.reverse();
            }
            let mut state = TrafficState::default();
            state.gates.push(Gate {
                centre: Vec2::new(10., 10.),
                axis: Vec2::X,
                half_length: 4.,
                radius: 0.5,
                direction: 1.,
                admitted: GateRows::from([ids[0]]),
                reserved_by: [true; SLOTS],
                provisional_for: None,
                issued: 1,
                blocked_contact: None,
            });
            let mut proposed = original.clone();
            apply_with_contacts(
                &original,
                &mut proposed,
                0.05,
                8,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Inside,
            );
            let first = proposed
                .units
                .iter()
                .find(|u| u.unit_id == ids[0])
                .unwrap()
                .target;
            assert_ne!(
                first,
                Some(Vec2::new(24., 10.)),
                "a certified passing path remains available"
            );
            original
                .units
                .iter_mut()
                .find(|u| u.unit_id == ids[1])
                .unwrap()
                .position
                .y += 0.2;
            proposed = original.clone();
            apply_with_contacts(
                &original,
                &mut proposed,
                0.05,
                9,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Inside,
            );
            assert_eq!(
                proposed
                    .units
                    .iter()
                    .find(|u| u.unit_id == ids[0])
                    .unwrap()
                    .target,
                first,
                "a waiting peer cannot change the admitted actor's continuation each tick"
            );
        }
    }
    #[test]
    fn foreign_private_targets_do_not_change_cooperative_guidance_before_contact() {
        let run = |foreign_target: Option<Vec2>, reverse: bool| {
            let mut foreign = unit(
                if reverse { 1 } else { 99 },
                Vec2::new(12., 10.),
                foreign_target,
            );
            foreign.owner_slot = 1;
            let mut original = frame(vec![
                unit(
                    if reverse { 99 } else { 1 },
                    Vec2::new(8., 10.),
                    Some(Vec2::new(24., 10.)),
                ),
                foreign,
            ]);
            if reverse {
                original.units.reverse();
            }
            let mut proposed = original.clone();
            let mut state = TrafficState::default();
            let waiting = apply_with_contacts(
                &original,
                &mut proposed,
                0.05,
                1,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Inside,
            );
            assert!(
                state.gates.is_empty(),
                "foreign private requests cannot create cooperative gates"
            );
            let own = proposed.units.iter().find(|u| u.owner_slot == 0).unwrap();
            let physics = crate::movement::step_frame(&original, 0.05);
            let own_id = own.unit_id;
            (
                own.target,
                own.speed,
                waiting.contains(&own_id),
                physics
                    .iter()
                    .find(|u| u.unit_id == own_id)
                    .unwrap()
                    .position,
            )
        };
        let reference = run(Some(Vec2::new(2., 10.)), false);
        for target in [
            Some(Vec2::new(2., 10.)),
            Some(Vec2::new(24., 10.)),
            Some(Vec2::new(12., 24.)),
            None,
        ] {
            for reverse in [false, true] {
                assert_eq!(run(target, reverse), reference);
            }
        }
    }
    #[test]
    fn different_owners_still_receive_complete_continuous_body_collision_checks() {
        let mut other = unit(99, Vec2::new(12., 10.), Some(Vec2::new(8., 10.)));
        other.owner_slot = 1;
        let mut original = frame(vec![
            unit(1, Vec2::new(8., 10.), Some(Vec2::new(12., 10.))),
            other,
        ]);
        for u in &mut original.units {
            u.speed = Some(4.);
        }
        let output = crate::movement::step_frame(&original, 1.);
        let a = output
            .iter()
            .find(|u| u.unit_id == 1)
            .unwrap()
            .position
            .as_dvec2();
        let b = output
            .iter()
            .find(|u| u.unit_id == 99)
            .unwrap()
            .position
            .as_dvec2();
        let r = original.units[0].position.as_dvec2() - original.units[1].position.as_dvec2();
        let v = (a - original.units[0].position.as_dvec2())
            - (b - original.units[1].position.as_dvec2());
        let t = if v.length_squared() == 0. {
            0.
        } else {
            (-r.dot(v) / v.length_squared()).clamp(0., 1.)
        };
        let radius = f64::from(original.units[0].radius) + f64::from(original.units[1].radius);
        assert!(
            (r + t * v).length_squared() >= radius * radius,
            "owner privacy must not omit a physical collision body"
        );
    }
    #[test]
    fn unresolved_current_authority_holds_old_priority_without_destroying_history() {
        let original = frame(vec![
            unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0))),
            unit(2, Vec2::new(12.0, 10.0), Some(Vec2::new(2.0, 10.0))),
        ]);
        let mut state = TrafficState::default();
        apply(
            &original,
            &mut original.clone(),
            0.05,
            7,
            &mut state,
            |_, _, _| true,
        );
        state.members.get_mut(&1).unwrap().entrance =
            EntranceStamp::new(5, original.units[0].position, EntranceContact::Inside);
        state.members.get_mut(&2).unwrap().entrance =
            EntranceStamp::new(6, original.units[1].position, EntranceContact::Inside);
        let original_stamp = state.members[&1].entrance;
        apply_with_contacts(
            &original,
            &mut original.clone(),
            0.05,
            8,
            &mut state,
            |_, from, to| from.y == to.y,
            |_, _, _, _| EntranceContact::Unresolved,
        );
        assert!(
            state.gates.is_empty(),
            "old history cannot override unresolved current leases"
        );
        state.gates.push(Gate {
            centre: Vec2::new(10.0, 10.0),
            axis: Vec2::X,
            half_length: 4.0,
            radius: 0.5,
            direction: 1.0,
            admitted: GateRows::from([1]),
            reserved_by: std::array::from_fn(|owner| owner == 0),
            provisional_for: None,
            issued: 1,
            blocked_contact: None,
        });
        let mut proposed = original.clone();
        let waiting = apply_with_contacts(
            &original,
            &mut proposed,
            0.05,
            9,
            &mut state,
            |_, _, _| false,
            |_, _, _, _| EntranceContact::Unresolved,
        );
        assert_eq!(waiting, BTreeSet::from([1, 2]));
        assert!(proposed.units.iter().all(|unit| unit.speed == Some(0.0)));
        assert_eq!(state.members[&1].entrance, original_stamp);
        assert_eq!(state.gates[0].direction, 1.0);
    }

    #[test]
    fn committed_contact_callback_admission_uses_physical_order_and_contests_exact_ties() {
        fn run(invert: bool) -> Vec<Vec2> {
            let mut units = vec![
                unit(
                    if invert { 99 } else { 1 },
                    Vec2::new(8.0, 10.0),
                    Some(Vec2::new(24.0, 10.0)),
                ),
                unit(
                    if invert { 1 } else { 99 },
                    Vec2::new(2.0, 10.0),
                    Some(Vec2::new(24.0, 10.0)),
                ),
            ];
            if invert {
                units.reverse();
            }
            let original = frame(units);
            let mut state = TrafficState::default();
            apply(
                &original,
                &mut original.clone(),
                0.05,
                7,
                &mut state,
                |_, _, _| true,
            );
            let outputs = crate::movement::step_frame(&original, 0.05);
            let mut seen = Vec::new();
            record_committed_contacts(&original, &outputs, 7, &mut state, |index, _, _, _| {
                seen.push(original.units[index].position);
                EntranceContact::Outside
            });
            seen
        }
        let expected = vec![
            Vec2::new(2.0, 10.0),
            Vec2::new(2.0, 10.0),
            Vec2::new(8.0, 10.0),
            Vec2::new(8.0, 10.0),
        ];
        assert_eq!(run(false), expected);
        assert_eq!(run(true), expected);
        let original = frame(vec![
            unit(1, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0))),
            unit(2, Vec2::new(8.0, 10.0), Some(Vec2::new(24.0, 10.0))),
        ]);
        let mut state = TrafficState::default();
        apply(
            &original,
            &mut original.clone(),
            0.05,
            7,
            &mut state,
            |_, _, _| true,
        );
        let outputs = crate::movement::step_frame(&original, 0.05);
        let mut calls = 0;
        record_committed_contacts(&original, &outputs, 7, &mut state, |_, _, _, _| {
            calls += 1;
            EntranceContact::Inside
        });
        assert_eq!(calls, 0);
        assert!(
            state
                .members
                .values()
                .all(|member| member.entrance.is_none())
        );
    }

    #[test]
    fn first_positive_area_entrance_precedes_faster_later_arrival_in_the_same_tick() {
        use turts_rules::terrain::{
            Ratio,
            geometry::{Budget, Cell, Grid, Point, Traversal},
        };
        fn direction(invert: bool) -> f32 {
            let mut left = unit(
                if invert { 99 } else { 1 },
                Vec2::new(4.895, 3.5),
                Some(Vec2::new(14.0, 3.5)),
            );
            left.radius = 1.1;
            left.speed = Some(0.5);
            let mut right = unit(
                if invert { 1 } else { 99 },
                Vec2::new(11.18, 3.5),
                Some(Vec2::new(2.0, 3.5)),
            );
            right.radius = 1.1;
            right.speed = Some(2.0);
            // Entrance is the infimum time at which the footprint has positive
            // area inside x[6,10], not centre arrival or ETA to a meeting point.
            // Tangency itself grants no positive area. The left footprint first
            // enters after approximately .01s; the right enters after .04s.
            let left_entry = (6.0 - f64::from(left.radius) - f64::from(left.position.x))
                / f64::from(left.speed.unwrap());
            let right_entry = (f64::from(right.position.x) - (10.0 + f64::from(right.radius)))
                / f64::from(right.speed.unwrap());
            assert!(
                0.0 < left_entry && left_entry < right_entry && right_entry < 0.05,
                "fixture must have distinct same-tick physical entrance times: {left_entry},{right_entry}"
            );
            let mut units = vec![left, right];
            if invert {
                units.reverse();
            }
            let mut original = frame(units);
            original.bounds = Vec2::new(16.0, 8.0);
            let mut proposed = original.clone();
            let mut state = TrafficState::default();
            let grid = Grid {
                columns: 16,
                rows: 8,
                length_scale: 1000,
                cell_width: 1000,
                elevation_step: 250,
                floor: 0,
                ceiling: 8,
            };
            let surface = |cell: Cell| {
                Some(if (6..10).contains(&cell.x) && !(2..5).contains(&cell.y) {
                    4
                } else {
                    0
                })
            };
            let profile = Traversal {
                max_step: 0,
                max_grade: Ratio {
                    numerator: 0,
                    denominator: 1,
                },
            };
            apply_with_contacts(
                &original,
                &mut proposed,
                0.05,
                7,
                &mut state,
                |index, from, to| {
                    let point = |value: Vec2| Point {
                        x: f64::from(value.x),
                        y: f64::from(value.y),
                    };
                    let result = grid.sweep(
                        &surface,
                        point(from),
                        point(to),
                        f64::from(original.units[index].radius),
                        profile,
                        &mut Budget::new(65_536),
                    );
                    result.result.is_ok() && result.numerator == result.denominator
                },
                |index, from, to, axis| {
                    exact_contact(grid, &surface, from, to, axis, original.units[index].radius)
                },
            );
            assert_eq!(
                state.gates.len(),
                1,
                "exact legal corridor must generate one physical entrance gate"
            );
            state.gates[0].direction
        }
        let normal = direction(false);
        let inverted = direction(true);
        assert_eq!(
            normal, inverted,
            "allocation and enumeration cannot select an entrance winner"
        );
        assert!(
            normal > 0.0,
            "the slow left actor entered first; ETA to encounter centre must not favor the later fast right actor"
        );
    }
}

/// Privileged diagnostic state. Contains other owners' intents; never export to players.
pub fn privileged_traffic_debug(world: &bevy_ecs::world::World) -> String {
    world.get_resource::<TrafficState>().map_or_else(
        || String::from("no traffic"),
        |state| format!("gates={:?}; members={:?}", state.gates, state.members),
    )
}

#[cfg(test)]
mod guided_composition_tests {
    use super::*;
    fn actor(id: u32, position: Vec2, target: Option<Vec2>) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.3,
            speed: target.map(|_| 3.5),
            target,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    // Dense own traffic spends the public owner quota before the far, otherwise
    // isolated actor. No foreign facts, authored coordinates or test policy env.
    fn pressure() -> MovementFrame {
        let mut units = Vec::new();
        for n in 0..130 {
            let p = Vec2::new(10. + n as f32 * 0.001, 10.);
            units.push(actor(n + 1, p, Some(Vec2::new(30., 10.))));
        }
        for n in 0..128 {
            units.push(actor(
                n + 200,
                Vec2::new(10. + n as f32 * 0.001, 10.2),
                None,
            ));
        }
        units.push(actor(
            1000,
            Vec2::new(100., 10.),
            Some(Vec2::new(120., 10.)),
        ));
        MovementFrame {
            bounds: Vec2::splat(256.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        }
    }
    fn run(
        frame: &MovementFrame,
        proposed: &mut MovementFrame,
        state: &mut TrafficState,
        guided: bool,
    ) -> BTreeSet<u32> {
        apply_composed(
            frame,
            proposed,
            0.05,
            1,
            state,
            |_, _, _| true,
            |_, _, _, _| EntranceContact::Inside,
            |_| false,
            &mut crate::movement::sweep::SweepBudget::new(1),
            |_, _| None,
            |_| false,
            |_, _| true,
            guided,
            |_| None,
            |_| false,
        )
    }
    fn run_current_wall(
        frame: &MovementFrame,
        candidate: &mut MovementFrame,
        state: &mut TrafficState,
        witness: Option<(u32, ObservedBody)>,
    ) -> BTreeSet<u32> {
        let mut budget = crate::movement::sweep::SweepBudget::new(1);
        for owner in 0..SLOTS {
            assert!(budget.reserve_model_amount(owner as u8, GUIDED_RESERVE));
        }
        let result = apply_guided_with_observed_detours(
            frame,
            candidate,
            0.05,
            1,
            state,
            |_, _, _| true,
            |_, _, _, _| EntranceContact::Inside,
            |_| false,
            &mut budget,
            |viewer, id| {
                witness
                    .filter(|(key, _)| viewer == 0 && id == *key)
                    .map(|(_, body)| body)
            },
            |_| true,
            |viewer, emit| viewer != 0 || witness.is_none_or(|(id, body)| emit(id, body)),
        );
        for owner in 0..SLOTS {
            budget.release_model(owner as u8, GUIDED_RESERVE);
        }
        assert_eq!(budget.model_reserved, [0; SLOTS]);
        assert!(
            state
                .tick_owner_neighbors
                .iter()
                .all(|&n| n <= OWNER_VISITS - GUIDED_RESERVE)
        );
        assert!(state.retained_bytes() + state.scratch_bound <= SCRATCH_BYTES);
        result
    }
    #[test]
    fn current_foreign_route_is_serviced_before_dense_optional_own_pressure() {
        let mut normalized = Vec::new();
        for invert in [false, true] {
            let mut frame = pressure();
            frame.units.retain(|u| u.target.is_some());
            if invert {
                for u in &mut frame.units {
                    u.unit_id = 4000 - u.unit_id;
                }
                frame.units.reverse();
            }
            let id = frame
                .units
                .iter()
                .find(|u| u.position.x == 100.)
                .unwrap()
                .unit_id;
            let body = ObservedBody {
                position: Vec2::new(110., 10.),
                radius: 0.3,
            };
            let witness = (if invert { 9001 } else { 9000 }, body);
            let mut state = TrafficState::default();
            let mut candidate = frame.clone();
            let waiting = run_current_wall(&frame, &mut candidate, &mut state, Some(witness));
            assert!(
                state.neighbor_overflows > 0,
                "Dense own-neighbor pressure must still exhaust its same public quota"
            );
            assert!(!waiting.contains(&id));
            let route=state.members[&id].local_detour.expect("CURRENT foreign obstruction must get its existing witnessed route before optional own rows exhaust visits");
            assert_eq!(route.witness, Some(witness));
            let u = frame.units.iter().find(|u| u.unit_id == id).unwrap();
            for (from, to) in [
                (u.position, route.route.corners[0]),
                (route.route.corners[0], route.route.corners[1]),
            ] {
                assert!(crate::movement::sweep::pair_clear(
                    from,
                    to,
                    u.radius,
                    body.position,
                    body.position,
                    body.radius
                ));
            }
            let target = candidate
                .units
                .iter()
                .find(|u| u.unit_id == id)
                .unwrap()
                .target;
            assert_eq!(target, Some(route.route.corners[route.corner]));
            assert!(state.members[&id].attempts.get(Attempt::Detour).is_some());
            normalized.push((route.route.corners, target));
            // Undisclosed geometry cannot select a route even under the same pressure.
            let mut hidden = frame.clone();
            let mut hidden_state = TrafficState::default();
            run_current_wall(&frame, &mut hidden, &mut hidden_state, None);
            assert!(hidden_state.members[&id].local_detour.is_none());
            assert_eq!(
                hidden
                    .units
                    .iter()
                    .find(|u| u.unit_id == id)
                    .unwrap()
                    .target,
                u.target
            );
        }
        assert_eq!(normalized[0], normalized[1]);
    }
    #[test]
    fn real_gate_wait_keeps_priority_and_preserves_early_foreign_route_intent() {
        let mut frame = pressure();
        frame.units.retain(|u| u.target.is_some());
        frame
            .units
            .push(actor(1001, Vec2::new(101., 10.), Some(Vec2::new(90., 10.))));
        let mut state = TrafficState::default();
        state.gates.push(Gate {
            centre: Vec2::new(100.5, 10.),
            axis: Vec2::X,
            half_length: 2.,
            radius: 0.3,
            direction: 1.,
            admitted: GateRows::from([1000]),
            reserved_by: [true, false],
            provisional_for: None,
            issued: 1,
            blocked_contact: None,
        });
        let body = ObservedBody {
            position: Vec2::new(97., 10.),
            radius: 0.3,
        };
        let mut candidate = frame.clone();
        let waiting = run_current_wall(&frame, &mut candidate, &mut state, Some((9000, body)));
        assert!(waiting.contains(&1001));
        assert_eq!(
            candidate
                .units
                .iter()
                .find(|u| u.unit_id == 1001)
                .unwrap()
                .target,
            None,
            "The retained route is private intent, not permission to skip a real wait"
        );
        assert_eq!(
            state.members[&1001].local_detour.unwrap().witness,
            Some((9000, body)),
            "Waiting must not erase a still-valid foreign witness/corner"
        );
        assert!(state.gates.iter().any(|g| g.admitted.contains(&1000)));
    }
    #[test]
    fn optional_neighbor_refusal_preserves_the_isolated_permitted_candidate() {
        for invert in [false, true] {
            let mut frame = pressure();
            if invert {
                for u in &mut frame.units {
                    u.unit_id = 2000 - u.unit_id;
                }
                frame.units.reverse();
            }
            let id = frame
                .units
                .iter()
                .find(|u| u.position.x == 100.)
                .unwrap()
                .unit_id;
            let mut candidate = frame.clone();
            let incoming = Some(Vec2::new(110., 12.));
            candidate
                .units
                .iter_mut()
                .find(|u| u.unit_id == id)
                .unwrap()
                .target = incoming;
            let mut state = TrafficState::default();
            let waiting = run(&frame, &mut candidate, &mut state, true);
            let row = candidate.units.iter().find(|u| u.unit_id == id).unwrap();
            assert!(state.neighbor_overflows > 0);
            assert_eq!(state.tick_owner_neighbors[0], OWNER_VISITS - GUIDED_RESERVE);
            assert!(!waiting.contains(&id));
            assert_eq!(
                row.target, incoming,
                "do not restore a different, unpermitted endpoint"
            );
            assert_eq!(row.speed, Some(3.5));
            assert!(!state.capacity_blocked);
            assert!(state.retained_bytes() + state.scratch_bound <= SCRATCH_BYTES);
            let mut legacy = frame.clone();
            let mut legacy_state = TrafficState::default();
            let legacy_wait = run(&frame, &mut legacy, &mut legacy_state, false);
            assert!(legacy_wait.contains(&id));
            assert_eq!(
                legacy.units.iter().find(|u| u.unit_id == id).unwrap().speed,
                Some(0.0)
            );
            assert_eq!(legacy_state.tick_owner_neighbors[0], OWNER_VISITS);
        }
    }
    #[test]
    fn real_gate_wait_survives_missing_optional_neighbor_rows() {
        let mut frame = pressure();
        let late = actor(1001, Vec2::new(101., 10.), Some(Vec2::new(90., 10.)));
        frame.units.push(late);
        let mut state = TrafficState::default();
        state.gates.push(Gate {
            centre: Vec2::new(100.5, 10.),
            axis: Vec2::X,
            half_length: 2.,
            radius: 0.3,
            direction: 1.,
            admitted: GateRows::from([1000]),
            reserved_by: [true, false],
            provisional_for: None,
            issued: 1,
            blocked_contact: None,
        });
        let mut candidate = frame.clone();
        let waiting = run(&frame, &mut candidate, &mut state, true);
        assert!(state.neighbor_overflows > 0);
        assert!(!waiting.contains(&1000));
        assert!(
            waiting.contains(&1001),
            "optional refusal must not bypass an existing physical gate"
        );
        assert_eq!(
            candidate
                .units
                .iter()
                .find(|u| u.unit_id == 1001)
                .unwrap()
                .speed,
            Some(0.0)
        );
        assert_eq!(
            candidate
                .units
                .iter()
                .find(|u| u.unit_id == 1001)
                .unwrap()
                .target,
            None
        );
        assert!(state.gates.iter().any(|g| g.admitted.contains(&1000)));
    }
    #[test]
    fn legacy_public_wrapper_still_uses_full_quota_and_refusal_holds() {
        let frame = pressure();
        let mut a = frame.clone();
        let mut b = frame.clone();
        let mut sa = TrafficState::default();
        let mut sb = TrafficState::default();
        let expected = run(&frame, &mut a, &mut sa, false);
        let actual = apply_with_observed_detours(
            &frame,
            &mut b,
            0.05,
            1,
            &mut sb,
            |_, _, _| true,
            |_, _, _, _| EntranceContact::Inside,
            |_| false,
            &mut crate::movement::sweep::SweepBudget::new(1),
            |_, _| None,
            |_| false,
            |_, _| true,
        );
        assert_eq!(a, b);
        assert_eq!(expected, actual);
        assert_eq!(sa.tick_owner_neighbors, sb.tick_owner_neighbors);
        assert!(actual.contains(&1000));
    }
}
