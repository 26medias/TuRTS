//! Bounded, fair blocked-recovery escape search (designs `2026-11-18-bounded-recovery` and
//! `2026-11-19-recovery-actual-work`).
//!
//! - One schedule pass at tick start gives each blocked row a work allowance. Jobs with no
//!   scan share a fresh budget. The oldest job with a scan (then the physical key x, y,
//!   radius) gets the rest; rows with an equal key share it. No allowance is below
//!   `MIN_SLICE`, and the allowances of one owner never sum above `OWNER_WORK`. No unit id
//!   and no hash order selects service.
//! - A slice reads only the known bodies of its actor (`recovery_view::KnownSet`): the owner
//!   view and the actor contacts. Every cell row, cell, body key, candidate, pool slot,
//!   claim cell and contact that it reads costs one unit of its allowance, and is refused
//!   when the allowance is spent (`Meter`). A paused point query keeps a cursor, so one cell
//!   larger than a slice still ends.
//! - Before it continues, a slice proves that the known facts of the examined region are
//!   unchanged (a digest of cell digests). A changed region restarts the scan. After
//!   `RELAX_AFTER` restarts, or when a proof costs more than half the slice, the scan runs
//!   relaxed and commits only a point that is free now.
//! - Frame work that is not charged to an owner: the owner view build, the contact query of
//!   each actor and the tick-start overlap of each row. Hidden bodies change only this
//!   frame work, never a charge, a result or a schedule.

use bevy_math::Vec2;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;

use super::frame::MovementFrameUnit;
use super::pool_index::MovementPoolIndex;
use super::recovery_view::{ClaimSet, Contacts, KnownSet, KnownView, Meter, Probe, body_key};
use super::spread::{finite_hex_ring_limit, minimum_unseen_hex_radius, nearest_angle_order};
use super::{RecoveryKind, RecoveryPoolCursor, RecoveryScan, RecoverySearch, RecoveryStage};

/// The recovery allowance of one owner in one solver pass. It is half the 65,536 owner
/// envelope and it bounds the actual metered work, not an estimate. It is separate from the
/// route allowance `route::OWNER_CAP`.
pub(crate) const OWNER_WORK: usize = 32_768;
/// The share of a job with no scan. Most searches end inside it.
pub(crate) const FRESH_SLICE: usize = 1_024;
/// The smallest allowance of a served row: it pays the proof reserve of a paused query
/// (at most 4 cell rows, 16 cells and the contacts) and still makes progress.
pub(crate) const MIN_SLICE: usize = 64;
/// Failed region proofs before a job runs relaxed.
pub(crate) const RELAX_AFTER: u8 = 2;
const SLOTS: usize = turts_mod_api::SLOT_COUNT as usize;
/// The clearance tolerance of a ring candidate (the base singleton predicate).
const RING_EPSILON: f32 = 1.0e-5;
/// The clearance tolerance of a pool slot (the base pool predicate).
const SLOT_EPSILON: f32 = 0.0;

#[cfg(test)]
thread_local! {
    /// Test hook: the candidates and slots that one slice may start.
    pub(super) static SLICE_CANDIDATES: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

pub(crate) fn owner_bucket(owner: u8) -> usize {
    usize::from(owner).min(SLOTS - 1)
}

/// Recovery accounting of one tick (summed over solver passes).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RecoveryCounters {
    /// The metered work of each owner. Each unit is one read of a known fact.
    pub(crate) owner_work: [usize; SLOTS],
    /// The actual recovery work of each owner: the metered work plus the bounded pool
    /// prechecks (`Meter::force`). Frame work is in `frame_work`.
    pub(crate) owner_actual: [usize; SLOTS],
    pub(crate) candidates: u64,
    pub(crate) cells: u64,
    pub(crate) bodies: u64,
    pub(crate) visibility_checks: u64,
    pub(crate) slot_checks: u64,
    pub(crate) digests: u64,
    pub(crate) invalidations: u64,
    pub(crate) relaxations: u64,
    pub(crate) relaxed_commits: u64,
    pub(crate) slices: u64,
    pub(crate) completions: u64,
    pub(crate) pauses: u64,
    pub(crate) waits: u64,
    pub(crate) max_age: u16,
    pub(crate) jobs: u64,
    pub(crate) pool_work: u64,
    pub(crate) spare_slot_work: u64,
    /// Work charged to no owner: view builds (one membership check for each row and
    /// owner), contact queries and tick-start overlap queries (cells and bodies visited).
    pub(crate) frame_work: u64,
    pub(crate) phase_bytes: usize,
    pub(crate) retained_bytes: usize,
}

impl RecoveryCounters {
    pub(crate) fn charge(&mut self, owner: u8, public: usize, actual: usize) {
        let bucket = owner_bucket(owner);
        self.owner_work[bucket] = self.owner_work[bucket].saturating_add(public);
        self.owner_actual[bucket] = self.owner_actual[bucket].saturating_add(actual);
    }

    /// Adds the counters of another solver pass of the same tick.
    pub(crate) fn merge(&mut self, other: &Self) {
        for bucket in 0..SLOTS {
            self.owner_work[bucket] =
                self.owner_work[bucket].saturating_add(other.owner_work[bucket]);
            self.owner_actual[bucket] =
                self.owner_actual[bucket].saturating_add(other.owner_actual[bucket]);
        }
        self.candidates += other.candidates;
        self.cells += other.cells;
        self.bodies += other.bodies;
        self.visibility_checks += other.visibility_checks;
        self.slot_checks += other.slot_checks;
        self.digests += other.digests;
        self.invalidations += other.invalidations;
        self.relaxations += other.relaxations;
        self.relaxed_commits += other.relaxed_commits;
        self.slices += other.slices;
        self.completions += other.completions;
        self.pauses += other.pauses;
        self.waits += other.waits;
        self.max_age = self.max_age.max(other.max_age);
        self.jobs = other.jobs;
        self.pool_work += other.pool_work;
        self.spare_slot_work += other.spare_slot_work;
        self.frame_work += other.frame_work;
        self.phase_bytes = self.phase_bytes.max(other.phase_bytes);
        self.retained_bytes = other.retained_bytes;
    }

    pub(crate) fn peak_owner_work(&self) -> usize {
        self.owner_work.iter().copied().max().unwrap_or(0)
    }

    pub(crate) fn peak_owner_actual(&self) -> usize {
        self.owner_actual.iter().copied().max().unwrap_or(0)
    }

    fn absorb(&mut self, meter: &Meter) {
        self.cells += meter.cells;
        self.bodies += meter.bodies;
        self.digests += meter.digests;
        self.slot_checks += meter.claims;
    }
}

/// The work allowances of one tick, sorted by frame row.
pub(super) struct RecoverySchedule {
    allowances: Vec<(usize, usize)>,
}

impl RecoverySchedule {
    pub(super) fn allowance(&self, row: usize) -> usize {
        self.allowances
            .binary_search_by_key(&row, |entry| entry.0)
            .map_or(0, |at| self.allowances[at].1)
    }
}

/// One row that may run a recovery search in this tick.
#[derive(Clone, Copy, Debug)]
pub(super) struct ScheduleRow {
    pub(super) row: usize,
    pub(super) owner: u8,
    pub(super) age: u16,
    pub(super) has_scan: bool,
    /// True when the row overlaps a known body at tick start. Such a body is in contact
    /// range, so `known` admits it. Overlapping rows come first in both tiers: a row that
    /// does not search in this tick must not take a slice before one that does.
    pub(super) overlapping: bool,
    pub(super) position: Vec2,
    pub(super) radius: f32,
}

fn service_order(left: &ScheduleRow, right: &ScheduleRow) -> std::cmp::Ordering {
    right
        .overlapping
        .cmp(&left.overlapping)
        .then_with(|| right.age.cmp(&left.age))
        .then_with(|| left.position.x.total_cmp(&right.position.x))
        .then_with(|| left.position.y.total_cmp(&right.position.y))
        .then_with(|| left.radius.total_cmp(&right.radius))
}

/// Shares `credit` among one batch of rows with an equal service key and returns the
/// credit given. Each row gets an equal share when the share is at least `MIN_SLICE`.
/// Otherwise `credit / MIN_SLICE` rows get `MIN_SLICE`, from a start that moves by that
/// count each tick, so every row is served within `ceil(rows / served)` ticks. Rows of one batch are physically identical: the same owner, position, radius,
/// age and overlap.
fn share(batch: &[ScheduleRow], credit: usize, tick: u32, out: &mut Vec<(usize, usize)>) -> usize {
    let count = batch.len();
    if count == 0 {
        return 0;
    }
    let each = credit / count;
    if each >= MIN_SLICE {
        out.extend(batch.iter().map(|row| (row.row, each)));
        return each * count;
    }
    let served = (credit / MIN_SLICE).min(count);
    let start = (tick as usize).wrapping_mul(served) % count;
    out.extend((0..served).map(|k| (batch[(start + k) % count].row, MIN_SLICE)));
    served * MIN_SLICE
}

/// Gives the allowances of each owner. Fresh jobs get `FRESH_SLICE` each, in service
/// order, from half the budget; a first batch that does not fit shares that half. The
/// oldest batch of jobs with a scan shares the rest. With no such job, the served fresh
/// jobs share the rest too. The allowances of one owner sum to at most `OWNER_WORK`.
pub(super) fn plan(
    mut rows: Vec<ScheduleRow>,
    tick: u32,
    counters: &mut RecoveryCounters,
) -> RecoverySchedule {
    rows.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| left.has_scan.cmp(&right.has_scan))
            .then_with(|| service_order(left, right))
    });
    let mut allowances = Vec::with_capacity(rows.len());
    let mut served = Vec::new();
    for owner_rows in rows.chunk_by(|left, right| left.owner == right.owner) {
        let split = owner_rows.partition_point(|row| !row.has_scan);
        let (fresh, continuing) = owner_rows.split_at(split);
        served.clear();
        let mut fresh_spent = 0usize;
        for batch in fresh.chunk_by(|left, right| service_order(left, right).is_eq()) {
            let cost = FRESH_SLICE * batch.len();
            if fresh_spent + cost <= OWNER_WORK / 2 {
                fresh_spent += cost;
                served.extend(batch.iter().map(|row| (row.row, FRESH_SLICE)));
            } else {
                if served.is_empty() {
                    fresh_spent += share(batch, OWNER_WORK / 2, tick, &mut served);
                }
                break;
            }
        }
        let rest = OWNER_WORK.saturating_sub(fresh_spent);
        let oldest = continuing
            .chunk_by(|left, right| service_order(left, right).is_eq())
            .next()
            .unwrap_or(&[]);
        let fresh_extra = if oldest.is_empty() && !served.is_empty() {
            rest / served.len()
        } else {
            0
        };
        allowances.extend(
            served
                .iter()
                .map(|&(row, given)| (row, given + fresh_extra)),
        );
        share(oldest, rest, tick, &mut allowances);
    }
    allowances.sort_unstable();
    counters.phase_bytes = counters.phase_bytes.max(
        rows.capacity() * std::mem::size_of::<ScheduleRow>()
            + (allowances.capacity() + served.capacity()) * std::mem::size_of::<(usize, usize)>(),
    );
    RecoverySchedule { allowances }
}

pub(super) enum SliceOutcome {
    /// The search ended: the nearest free point, or none.
    Done(Option<Vec2>),
    /// The slice is spent; keep this scan.
    Paused(RecoveryScan),
}

/// The pool slot search of a group escape.
#[derive(Clone, Copy)]
pub(super) struct PoolSearch<'a> {
    pub(super) index: &'a MovementPoolIndex,
    pub(super) unit_id: u32,
    /// The claim key that this mover may keep.
    pub(super) allowed: Option<(u64, u32, u32)>,
}

/// One escape search over the current movement state.
pub(super) struct EscapeScan<'a> {
    pub(super) bounds: Vec2,
    pub(super) origin: Vec2,
    pub(super) radius: f32,
    /// The pool and its claims (group escape only).
    pub(super) slots: Option<(u64, &'a ClaimSet)>,
    /// The pool slot search; its result bounds the ring scan.
    pub(super) pool: Option<PoolSearch<'a>>,
    /// A point that already wins: stop when the nearest unseen ring is farther.
    pub(super) bound: Option<Vec2>,
    /// Test hook. Production is always `true`.
    pub(super) verify: bool,
}

/// One step of a slice.
enum Step {
    Done(Option<Vec2>),
    Pause,
    /// A relaxed point is not free now: start a relaxed search again.
    Restart,
}

/// The state of one slice.
struct Slice<'m> {
    meter: Meter,
    allowance: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    started: usize,
    counters: &'m mut RecoveryCounters,
}

impl EscapeScan<'_> {
    fn spacing(&self) -> f32 {
        2.0 * self.radius + 0.1
    }

    fn same_search(&self, scan: &RecoveryScan, known: &KnownSet) -> bool {
        scan.origin.x.to_bits() == self.origin.x.to_bits()
            && scan.origin.y.to_bits() == self.origin.y.to_bits()
            && scan.radius.to_bits() == self.radius.to_bits()
            && scan.cell == known.cell_bits()
            && scan.pool.is_some() == self.pool.is_some()
    }

    fn fresh(&self, known: &KnownSet, invalidations: u8, relaxed: bool) -> RecoveryScan {
        RecoveryScan {
            origin: self.origin,
            radius: self.radius,
            ring: 1,
            side: 0,
            step: 0,
            best: None,
            examined: 0.0,
            digest: [0; 3],
            invalidations,
            relaxed: relaxed || invalidations >= RELAX_AFTER,
            cell: known.cell_bits(),
            stage: if self.pool.is_some() {
                RecoveryStage::Pool
            } else {
                RecoveryStage::Ring
            },
            pool: self.pool.map(|_| RecoveryPoolCursor {
                level: 0,
                at: 0,
                item: 0,
                best: None,
                checked: false,
                done: false,
            }),
            query: None,
            spanned: false,
        }
    }

    fn proving(&self, state: &RecoveryScan) -> bool {
        self.verify && !state.relaxed
    }

    fn claims(&self, examined: f32) -> Option<(u64, &ClaimSet, f32)> {
        self.slots.map(|(pool, set)| (pool, set, examined))
    }

    /// The region whose known facts decide every candidate within `examined`.
    fn region_reach(&self, known: &KnownSet, examined: f32) -> f32 {
        examined + self.radius + known.view.half()
    }

    fn region_cost(&self, known: &KnownSet, examined: f32) -> usize {
        known.region_cost(
            self.origin,
            self.region_reach(known, examined),
            self.claims(examined),
        )
    }

    fn region_digest(
        &self,
        known: &KnownSet,
        examined: f32,
        meter: &mut Meter,
    ) -> Option<[u64; 3]> {
        known.region_digest(
            self.origin,
            self.region_reach(known, examined),
            self.claims(examined),
            meter,
        )
    }

    fn local_cost(&self, known: &KnownSet, point: Vec2) -> usize {
        known.region_cost(point, self.radius + known.view.half(), None)
    }

    fn local_digest(&self, known: &KnownSet, point: Vec2, meter: &mut Meter) -> Option<[u64; 3]> {
        known.region_digest(point, self.radius + known.view.half(), None, meter)
    }

    /// Sets the reserve that a pause at `point` needs. False when the slice cannot do one
    /// more unit of work above it.
    fn reserve(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        point: Vec2,
        slice: &mut Slice,
    ) -> bool {
        slice.meter.floor = 0;
        let examined = state.examined.max(point.distance(self.origin));
        if self.proving(state) {
            let cost = self.region_cost(known, examined);
            if cost > slice.allowance / 2 {
                // The proof would cost more than half a slice: run relaxed.
                slice.counters.relaxations += 1;
                state.relaxed = true;
            } else {
                slice.meter.floor = cost;
            }
        }
        if !self.proving(state) && matches!(state.stage, RecoveryStage::Verify { .. }) {
            // Only the check of a relaxed point resumes with a proof: a relaxed scan
            // trusts none of its verdicts.
            slice.meter.floor = self.local_cost(known, point);
        }
        slice.meter.left > slice.meter.floor
    }

    /// Starts a candidate or slot at `point`: the reserve, the start limit and the
    /// candidate unit. False when the slice must pause first.
    fn begin(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        point: Vec2,
        slice: &mut Slice,
    ) -> bool {
        #[cfg(test)]
        if slice.started >= SLICE_CANDIDATES.with(|cap| cap.get()) {
            return false;
        }
        // One unit for the candidate and one for its claim check.
        if !self.reserve(known, state, point, slice) || slice.meter.left < slice.meter.floor + 3 {
            return false;
        }
        slice.started += 1;
        state.examined = state.examined.max(point.distance(self.origin));
        slice.meter.step()
    }

    /// The claim check of a candidate or slot (one unit, reserved by `begin`).
    fn claimed(&self, point: Vec2, allowed: Option<(u64, u32, u32)>, meter: &mut Meter) -> bool {
        let Some((pool, set)) = self.slots else {
            return false;
        };
        meter.step();
        meter.claims += 1;
        let key = (pool, point.x.to_bits(), point.y.to_bits());
        set.contains(&key) && Some(key) != allowed
    }

    fn in_bounds(&self, point: Vec2) -> bool {
        let radius = self.radius;
        point.x >= radius
            && point.y >= radius
            && point.x <= self.bounds.x - radius
            && point.y <= self.bounds.y - radius
    }

    /// Runs the probe of `state.query`. `None` when it paused.
    fn probe(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        epsilon: f32,
        slice: &mut Slice,
    ) -> Option<bool> {
        let query = state.query.take()?;
        match known.probe(query, self.radius, epsilon, &mut slice.meter) {
            Probe::Blocked => Some(false),
            Probe::Clear => Some(true),
            Probe::Pending(next) => {
                state.query = Some(next);
                None
            }
        }
    }

    /// Restores the reserve of a probe that a slice resumes.
    fn resume_query(&self, known: &KnownSet, state: &mut RecoveryScan, slice: &mut Slice) -> bool {
        let Some(query) = state.query else {
            return true;
        };
        self.reserve(known, state, query.point, slice)
    }

    /// The pool slot search. `None` when the slice paused.
    fn pool_stage(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        slice: &mut Slice,
    ) -> Option<()> {
        let Some(pool) = self.pool else {
            return Some(());
        };
        let mut cursor = state.pool.expect("a group search has a pool cursor");
        let result = self.pool_levels(known, state, &mut cursor, pool, slice);
        state.pool = Some(cursor);
        result
    }

    fn pool_levels(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        cursor: &mut RecoveryPoolCursor,
        pool: PoolSearch,
        slice: &mut Slice,
    ) -> Option<()> {
        if cursor.done {
            return Some(());
        }
        if !cursor.checked {
            // A local precheck: its work is bounded by the slots near the mover.
            let (possible, work) = pool.index.has_potential_free_slot_counted(
                pool.unit_id,
                self.origin,
                self.radius,
                pool.allowed.map(|(_, x, y)| (x, y)),
            );
            slice.meter.force(work);
            slice.counters.pool_work += work as u64;
            cursor.checked = true;
            if !possible {
                cursor.done = true;
                return Some(());
            }
        }
        let grid = pool.index.slot_grid();
        let points = &pool.index.points;
        let max_reach = pool.index.max_reach();
        loop {
            let reach =
                (pool.index.spacing() * 2.0_f32.powi(i32::from(cursor.level))).min(max_reach);
            let inner = (cursor.level > 0).then(|| {
                (pool.index.spacing() * 2.0_f32.powi(i32::from(cursor.level) - 1)).min(max_reach)
            });
            let ((low_x, low_y), (high_x, high_y)) = grid.cell_box(self.origin, reach);
            let width = (high_x - low_x + 1) as u128;
            let area = width * (high_y - low_y + 1) as u128;
            let by_cells = area <= points.len() as u128;
            let positions = if by_cells { area } else { points.len() as u128 };
            while u128::from(cursor.at) < positions {
                let members: &[usize] = if by_cells {
                    let at = u128::from(cursor.at);
                    grid.cell_members(low_x + (at % width) as i64, low_y + (at / width) as i64)
                } else {
                    &[]
                };
                let count = if by_cells { members.len() } else { 1 };
                if by_cells && cursor.item == 0 && state.query.is_none() && !slice.meter.step() {
                    return None;
                }
                while (cursor.item as usize) < count {
                    let slot = if by_cells {
                        members[cursor.item as usize]
                    } else {
                        cursor.at as usize
                    };
                    let point = points[slot].position;
                    if state.query.is_none() {
                        let distance = point.distance_squared(self.origin);
                        let in_level = distance <= reach * reach
                            && inner.is_none_or(|inner| distance > inner * inner);
                        if in_level {
                            if !self.begin(known, state, point, slice) {
                                return None;
                            }
                            slice.counters.candidates += 1;
                            if !self.claimed(point, pool.allowed, &mut slice.meter) {
                                state.query = Some(KnownSet::fresh_query(point));
                            }
                        } else if !slice.meter.step() {
                            return None;
                        }
                    }
                    if state.query.is_some() {
                        let free = self.probe(known, state, SLOT_EPSILON, slice)?;
                        let slot_index = slot as u32;
                        if free
                            && cursor.best.is_none_or(|(best_index, best_point)| {
                                nearest_angle_order(point, best_point, self.origin)
                                    .then_with(|| slot_index.cmp(&best_index))
                                    .is_lt()
                            })
                        {
                            cursor.best = Some((slot_index, point));
                        }
                    }
                    cursor.item += 1;
                }
                cursor.item = 0;
                cursor.at += 1;
            }
            if cursor.best.is_some() || reach >= max_reach {
                cursor.done = true;
                return Some(());
            }
            cursor.level += 1;
            cursor.at = 0;
            cursor.item = 0;
        }
    }

    /// Continues the hex ring order. `Some(best)` when the scan ends.
    fn ring_stage(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        slice: &mut Slice,
    ) -> Option<Option<Vec2>> {
        let spacing = self.spacing();
        let origin = self.origin;
        let bound = state
            .pool
            .and_then(|cursor| cursor.best.map(|best| best.1))
            .or(self.bound);
        let ring_limit = finite_hex_ring_limit(origin, self.radius, self.bounds, spacing);
        while state.ring <= ring_limit {
            let ring = state.ring;
            while state.side < 6 {
                while state.step < ring {
                    let step = state.step;
                    let (i, j) = match state.side {
                        0 => (ring - step, step),
                        1 => (-step, ring),
                        2 => (-ring, ring - step),
                        3 => (-ring + step, -step),
                        4 => (step, -ring),
                        _ => (ring, -ring + step),
                    };
                    let candidate = Vec2::new(
                        origin.x + spacing * (i as f32 + 0.5 * j as f32),
                        origin.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
                    );
                    if state.query.is_none() {
                        if !self.begin(known, state, candidate, slice) {
                            return None;
                        }
                        slice.counters.candidates += 1;
                        if self.in_bounds(candidate)
                            && !self.claimed(candidate, None, &mut slice.meter)
                        {
                            state.query = Some(KnownSet::fresh_query(candidate));
                        }
                    }
                    if state.query.is_some() {
                        let free = self.probe(known, state, RING_EPSILON, slice)?;
                        if free
                            && state.best.is_none_or(|previous| {
                                nearest_angle_order(candidate, previous, origin).is_lt()
                            })
                        {
                            state.best = Some(candidate);
                        }
                    }
                    state.step += 1;
                }
                state.step = 0;
                state.side += 1;
            }
            let unseen = minimum_unseen_hex_radius(ring, spacing);
            if state
                .best
                .is_some_and(|candidate| unseen > candidate.distance(origin) + 1.0e-5)
                || bound.is_some_and(|bound| unseen > bound.distance(origin) + 1.0e-5)
            {
                return Some(state.best);
            }
            state.side = 0;
            state.ring += 1;
        }
        Some(state.best)
    }

    /// The pool point and the ring point, chosen as the base (the nearer by angle order).
    fn choose(&self, state: &RecoveryScan, ring: Option<Vec2>) -> Option<(Vec2, bool)> {
        let pool = state.pool.and_then(|cursor| cursor.best.map(|best| best.1));
        match (pool, ring) {
            (Some(slot), Some(point)) => {
                Some(if nearest_angle_order(slot, point, self.origin).is_le() {
                    (slot, true)
                } else {
                    (point, false)
                })
            }
            (Some(slot), None) => Some((slot, true)),
            (None, Some(point)) => Some((point, false)),
            (None, None) => None,
        }
    }

    /// Checks a relaxed point against the current state. `None` when the slice paused.
    fn verify_point(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        point: Vec2,
        slot: bool,
        slice: &mut Slice,
    ) -> Option<bool> {
        if state.query.is_none() {
            if !self.reserve(known, state, point, slice) || slice.meter.left < slice.meter.floor + 2
            {
                return None;
            }
            let allowed = if slot {
                self.pool.and_then(|pool| pool.allowed)
            } else {
                None
            };
            if self.claimed(point, allowed, &mut slice.meter) {
                return Some(false);
            }
            state.query = Some(KnownSet::fresh_query(point));
        }
        let epsilon = if slot { SLOT_EPSILON } else { RING_EPSILON };
        self.probe(known, state, epsilon, slice)
    }

    fn advance(&self, known: &KnownSet, state: &mut RecoveryScan, slice: &mut Slice) -> Step {
        if !self.resume_query(known, state, slice) {
            return Step::Pause;
        }
        loop {
            match state.stage {
                RecoveryStage::Pool => {
                    if self.pool_stage(known, state, slice).is_none() {
                        return Step::Pause;
                    }
                    state.stage = RecoveryStage::Ring;
                }
                RecoveryStage::Ring => {
                    let Some(ring) = self.ring_stage(known, state, slice) else {
                        return Step::Pause;
                    };
                    let chosen = self.choose(state, ring);
                    match chosen {
                        Some((point, slot)) if state.spanned && !self.proving(state) => {
                            state.stage = RecoveryStage::Verify { point, slot };
                        }
                        _ => return Step::Done(chosen.map(|(point, _)| point)),
                    }
                }
                RecoveryStage::Verify { point, slot } => {
                    return match self.verify_point(known, state, point, slot, slice) {
                        None => Step::Pause,
                        Some(true) => {
                            slice.counters.relaxed_commits += 1;
                            Step::Done(Some(point))
                        }
                        Some(false) => Step::Restart,
                    };
                }
            }
        }
    }

    /// Runs one slice of at most `allowance` metered work (plus the bounded pool
    /// precheck, at most once for each pool search).
    pub(super) fn run(
        &self,
        known: &KnownSet,
        previous: Option<RecoveryScan>,
        allowance: usize,
        owner: u8,
        counters: &mut RecoveryCounters,
    ) -> SliceOutcome {
        let mut slice = Slice {
            meter: Meter::new(allowance),
            allowance,
            started: 0,
            counters,
        };
        slice.counters.slices += 1;
        let previous = previous.filter(|scan| self.same_search(scan, known));
        let mut state = match previous {
            None => self.fresh(known, 0, false),
            Some(mut scan) if !self.proving(&scan) => {
                // The paused check of a relaxed point resumes only with its local proof.
                // A paused scan probe resumes as it is: the chosen point is checked.
                if let Some(query) = scan.query
                    && matches!(scan.stage, RecoveryStage::Verify { .. })
                {
                    match self.local_digest(known, query.point, &mut slice.meter) {
                        Some(digest) if digest == query.digest => {}
                        Some(_) => scan.query = None,
                        None => {
                            return self.finish(owner, slice, SliceOutcome::Paused(scan));
                        }
                    }
                }
                scan
            }
            Some(mut scan) => {
                slice.meter.floor = allowance - allowance / 2;
                let digest = self.region_digest(known, scan.examined, &mut slice.meter);
                slice.meter.floor = 0;
                match digest {
                    Some(digest) if digest == scan.digest => scan,
                    Some(_) => {
                        slice.counters.invalidations += 1;
                        self.fresh(known, scan.invalidations.saturating_add(1), false)
                    }
                    None => {
                        // The proof costs more than half the slice: run relaxed. Nothing
                        // examined before is trusted; the chosen point is checked now.
                        slice.counters.relaxations += 1;
                        scan.relaxed = true;
                        scan.query = None;
                        scan
                    }
                }
            }
        };
        let outcome = match self.advance(known, &mut state, &mut slice) {
            Step::Done(point) => SliceOutcome::Done(point),
            Step::Restart => SliceOutcome::Paused(self.fresh(known, state.invalidations, true)),
            Step::Pause => {
                slice.meter.floor = 0;
                state.spanned = true;
                if self.proving(&state) {
                    match self.region_digest(known, state.examined, &mut slice.meter) {
                        Some(digest) => state.digest = digest,
                        None => {
                            debug_assert!(false, "the proof reserve covers the pause digest");
                            state.relaxed = true;
                            state.query = None;
                        }
                    }
                } else if let Some(mut query) = state.query
                    && matches!(state.stage, RecoveryStage::Verify { .. })
                {
                    match self.local_digest(known, query.point, &mut slice.meter) {
                        Some(digest) => {
                            query.digest = digest;
                            state.query = Some(query);
                        }
                        None => {
                            debug_assert!(false, "the local reserve covers the pause digest");
                            state.query = None;
                        }
                    }
                }
                SliceOutcome::Paused(state)
            }
        };
        self.finish(owner, slice, outcome)
    }

    fn finish(&self, owner: u8, slice: Slice, outcome: SliceOutcome) -> SliceOutcome {
        let Slice {
            meter, counters, ..
        } = slice;
        match outcome {
            SliceOutcome::Done(_) => counters.completions += 1,
            SliceOutcome::Paused(_) => counters.pauses += 1,
        }
        counters.absorb(&meter);
        counters.charge(owner, meter.used - meter.forced, meter.used);
        outcome
    }
}

/// The known-body indexes of one solver pass, shared by all rows. Built on first use.
pub(super) struct RecoveryWorld<'f> {
    /// The rows at tick start: the contact rule of `known` reads them, by unit id.
    start_units: &'f [MovementFrameUnit],
    /// The tick-start position of each solver row (the solver sorts its rows).
    start_positions: OnceCell<Vec<Vec2>>,
    visibility: Option<&'f dyn super::MovementVisibility>,
    /// One view for each owner; one shared view (key `None`) without a visibility basis.
    views: RefCell<BTreeMap<Option<u8>, KnownView>>,
    start_grid: OnceCell<(Vec<crate::grid::GridPoint>, crate::grid::UniformGrid, f32)>,
    frame_work: Cell<u64>,
    peak_bytes: Cell<usize>,
}

impl<'f> RecoveryWorld<'f> {
    pub(super) fn new(
        start_units: &'f [MovementFrameUnit],
        visibility: Option<&'f dyn super::MovementVisibility>,
    ) -> Self {
        Self {
            start_units,
            start_positions: OnceCell::new(),
            visibility,
            views: RefCell::new(BTreeMap::new()),
            start_grid: OnceCell::new(),
            frame_work: Cell::new(0),
            peak_bytes: Cell::new(0),
        }
    }

    fn view_key(&self, owner: u8) -> Option<u8> {
        self.visibility.map(|_| owner)
    }

    /// True when `owner` may know solver row `row` without contact: its own body, or a
    /// body that its basis permits. The same rule as `known` in the frame.
    pub(super) fn member(&self, owner: u8, row: usize, units: &[MovementFrameUnit]) -> bool {
        let other = &units[row];
        self.visibility.is_none_or(|basis| {
            other.owner_slot == owner || basis.unit_permitted(owner, other.unit_id)
        })
    }

    /// The tick-start position of solver row `row`, found by unit id as `known` does.
    pub(super) fn start_position(&self, row: usize, units: &[MovementFrameUnit]) -> Vec2 {
        self.start_positions(units)[row]
    }

    fn start_positions(&self, units: &[MovementFrameUnit]) -> &[Vec2] {
        self.start_positions.get_or_init(|| {
            let mut by_id: Vec<(u32, usize)> = self
                .start_units
                .iter()
                .enumerate()
                .map(|(index, unit)| (unit.unit_id, index))
                .collect();
            by_id.sort_unstable();
            self.add_frame_work(by_id.len());
            units
                .iter()
                .map(|unit| {
                    by_id
                        .binary_search_by_key(&unit.unit_id, |entry| entry.0)
                        .map_or(unit.position, |at| self.start_units[by_id[at].1].position)
                })
                .collect()
        })
    }

    /// Follows the move of frame row `row` in every view that holds it.
    pub(super) fn relocate(&self, row: usize, from: Vec2, to: Vec2, radius: f32) {
        for view in self.views.borrow_mut().values_mut() {
            view.relocate(row, from, to, radius);
        }
    }

    fn ensure_view(&self, owner: u8, units: &[MovementFrameUnit]) {
        let key = self.view_key(owner);
        if self.views.borrow().contains_key(&key) {
            return;
        }
        let members: Vec<_> = units
            .iter()
            .enumerate()
            .filter(|(row, _)| self.member(owner, *row, units))
            .map(|(row, unit)| (row, unit.position, unit.radius))
            .collect();
        self.add_frame_work(units.len());
        let view = KnownView::build(&members, units.len());
        self.views.borrow_mut().insert(key, view);
        self.note_bytes(members.capacity() * std::mem::size_of::<(usize, Vec2, f32)>());
    }

    fn add_frame_work(&self, work: usize) {
        self.frame_work.set(self.frame_work.get() + work as u64);
    }

    fn note_bytes(&self, scratch: usize) {
        let views: usize = self.views.borrow().values().map(KnownView::bytes).sum();
        let grid = self.start_grid.get().map_or(0, |(points, _, _)| {
            points.capacity() * (std::mem::size_of::<crate::grid::GridPoint>() + 32)
        });
        self.peak_bytes
            .set(self.peak_bytes.get().max(views + grid + scratch));
    }

    /// The bodies that the actor of row `row` touches at tick start and its owner view
    /// does not hold (the contact rule of `known`). One physical query of frame work.
    fn contacts(
        &self,
        owner: u8,
        row: usize,
        position: Vec2,
        radius: f32,
        units: &[MovementFrameUnit],
    ) -> Contacts {
        if self.visibility.is_none() {
            return Contacts::default();
        }
        let starts = self.start_positions(units);
        let (points, grid, max_radius) = self.start_grid.get_or_init(|| {
            // Solver rows at their tick-start positions.
            let points: Vec<_> = units
                .iter()
                .zip(starts)
                .map(|(unit, &position)| crate::grid::GridPoint {
                    id: unit.unit_id,
                    position,
                })
                .collect();
            let max_radius = units.iter().map(|unit| unit.radius).fold(0.0_f32, f32::max);
            let grid = crate::grid::UniformGrid::new(&points, 2.0 * max_radius);
            (points, grid, max_radius)
        });
        let views = self.views.borrow();
        let view = views.get(&self.view_key(owner));
        let mut bodies = Vec::new();
        let (cells, tested) = grid.visit_within(
            points,
            position,
            radius + max_radius + super::route::CONTACT_GAP,
            |other| {
                if other != row
                    && !view.is_some_and(|view| view.contains(other))
                    && position.distance(starts[other])
                        <= radius + units[other].radius + super::route::CONTACT_GAP
                {
                    bodies.push((units[other].position, units[other].radius));
                }
            },
        );
        drop(views);
        self.add_frame_work(cells + tested);
        let contacts = Contacts::new(bodies);
        self.note_bytes(contacts.bytes());
        contacts
    }

    pub(super) fn frame_work(&self) -> u64 {
        self.frame_work.get()
    }

    pub(super) fn peak_bytes(&self) -> usize {
        self.peak_bytes.get()
    }
}

/// The recovery state of one frame row in one tick.
pub(super) struct RowRecovery<'c, 'f> {
    pub(super) row: usize,
    pub(super) owner: u8,
    pub(super) allowance: usize,
    pub(super) previous: Option<RecoverySearch>,
    pub(super) next_jobs: &'c mut BTreeMap<usize, RecoverySearch>,
    pub(super) counters: &'c mut RecoveryCounters,
    pub(super) world: &'c RecoveryWorld<'f>,
}

impl RowRecovery<'_, '_> {
    fn same_kind(&self, kind: RecoveryKind) -> Option<RecoverySearch> {
        self.previous.filter(|job| job.kind == kind)
    }

    fn keep(&mut self, kind: RecoveryKind, scan: Option<RecoveryScan>) {
        let age = self
            .same_kind(kind)
            .map_or(1, |job| job.age.saturating_add(1));
        self.counters.max_age = self.counters.max_age.max(age);
        self.next_jobs
            .insert(self.row, RecoverySearch { kind, age, scan });
    }

    /// True when this row may run a slice now. Otherwise it keeps the job one tick older.
    pub(super) fn may_search(&mut self, kind: RecoveryKind) -> bool {
        if self.allowance > 0 {
            return true;
        }
        self.counters.waits += 1;
        let scan = self.same_kind(kind).and_then(|job| job.scan);
        self.keep(kind, scan);
        false
    }

    /// Runs one slice of `scan` for `kind` over the known bodies of this row. `None` when
    /// the search found no point or did not end in this slice; an unfinished job stays for
    /// the next tick.
    pub(super) fn search(
        &mut self,
        kind: RecoveryKind,
        scan: &EscapeScan,
        units: &[MovementFrameUnit],
    ) -> Option<Vec2> {
        if !self.may_search(kind) {
            return None;
        }
        let previous = self.same_kind(kind).and_then(|job| job.scan);
        let allowance = std::mem::take(&mut self.allowance);
        self.world.ensure_view(self.owner, units);
        let contacts = self
            .world
            .contacts(self.owner, self.row, scan.origin, scan.radius, units);
        let views = self.world.views.borrow();
        let known = KnownSet {
            view: &views[&self.world.view_key(self.owner)],
            contacts: &contacts,
            own_key: Some(body_key(units[self.row].position, units[self.row].radius)),
        };
        let outcome = scan.run(&known, previous, allowance, self.owner, self.counters);
        drop(views);
        match outcome {
            SliceOutcome::Done(point) => {
                self.next_jobs.remove(&self.row);
                point
            }
            SliceOutcome::Paused(state) => {
                self.keep(kind, Some(state));
                None
            }
        }
    }

    /// True when the last search of this row did not end.
    pub(super) fn pending(&self) -> bool {
        self.next_jobs.contains_key(&self.row)
    }
}

/// Bytes retained by recovery jobs after a tick.
pub(super) fn retained_bytes(jobs: usize) -> usize {
    jobs * (std::mem::size_of::<RecoverySearch>() + std::mem::size_of::<u32>())
}
