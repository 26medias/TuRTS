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
//! - Three classes of work (design `2026-11-19-recovery-actual-work` section 7). K: a read
//!   of a known fact, metered by the row allowance; it includes the retained-target checks
//!   and the stall spare-slot search. P: a neighbourhood read of the base grid (the schedule
//!   read, the exit read, the read at a new position); a fixed count for each row. V: the
//!   owner view build. Hidden bodies out of contact change only P and V work, never a
//!   charge, a result or a schedule.

use bevy_math::Vec2;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;
use std::ops::Range;

use super::frame::MovementFrameUnit;
use super::pool_index::MovementPoolIndex;
use super::recovery_view::{ClaimSet, Contacts, KnownSet, KnownView, Meter, Probe, body_key};
use super::route::CONTACT_GAP;
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
    /// The metered work of the stall spare-slot jobs. It is inside `owner_work`.
    pub(crate) spare_slot_work: u64,
    /// Metered point checks with no search: retained targets and own positions.
    pub(crate) checks: u64,
    /// Neighbourhood reads of the base grid (class P) of recovery rows: the schedule
    /// overlap read, the exit read, the read at a new position, and one read for each pool
    /// slot near the row when its turn ends (the slot clearance refresh). Each has a reach
    /// of at most `radius + max radius`, so it scans at most 16 cells (design
    /// `2026-11-20-bounded-contact`, rule G1).
    pub(crate) physical_reads: u64,
    /// The cells and bodies of the neighbourhood reads, of the contact reads, and the
    /// contacts that the row turns read.
    pub(crate) physical_work: u64,
    /// Contact reads (class P, fog only): one read of the hidden index of the owner for
    /// each schedule row. It reads no own and no permitted body (rule G2).
    pub(crate) contact_reads: u64,
    /// The cells and hidden bodies of the contact reads. It is inside `physical_work`.
    pub(crate) contact_work: u64,
    /// The view work (class V): one membership check for each row, for each owner with a
    /// schedule row under fog or a served row; one insert for each member of a built view
    /// and for each hidden row of a built hidden index; then two units for each move of a
    /// member of a built view.
    pub(crate) view_work: u64,
    /// Work charged to no owner: `physical_work + view_work`.
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
        self.checks += other.checks;
        self.physical_reads += other.physical_reads;
        self.physical_work += other.physical_work;
        self.contact_reads += other.contact_reads;
        self.contact_work += other.contact_work;
        self.view_work += other.view_work;
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

    /// Counts one neighbourhood read of `cells` cells and `bodies` bodies.
    pub(crate) fn physical_read(&mut self, cells: usize, bodies: usize) {
        self.physical_reads += 1;
        self.add_physical_work(cells + bodies);
    }

    /// Counts one contact read of `cells` cells and `bodies` hidden bodies.
    pub(crate) fn contact_read(&mut self, cells: usize, bodies: usize) {
        self.contact_reads += 1;
        self.contact_work += (cells + bodies) as u64;
        self.add_physical_work(cells + bodies);
    }

    /// Counts the slot clearance reads after the turn of a recovery row.
    pub(crate) fn slot_reads(&mut self, (reads, work): (usize, usize)) {
        self.physical_reads += reads as u64;
        self.add_physical_work(work);
    }

    pub(crate) fn add_physical_work(&mut self, work: usize) {
        self.physical_work += work as u64;
        self.frame_work += work as u64;
    }

    pub(crate) fn add_view_work(&mut self, work: u64) {
        self.view_work += work;
        self.frame_work += work;
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
    /// The rows whose allowance pays point checks only (the check floor), sorted.
    check_only: Vec<usize>,
}

impl RecoverySchedule {
    pub(super) fn allowance(&self, row: usize) -> usize {
        self.allowances
            .binary_search_by_key(&row, |entry| entry.0)
            .map_or(0, |at| self.allowances[at].1)
    }

    /// True when the allowance of `row` is the check floor: it starts no search.
    pub(super) fn check_only(&self, row: usize) -> bool {
        self.check_only.binary_search(&row).is_ok()
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
    /// True for a stalled group mover that may run the spare-slot search in this tick.
    /// Blocked rows are served before spare rows in both tiers.
    pub(super) spare: bool,
    /// True when the row has a retained escape target: its check needs `MIN_SLICE` at
    /// most in the common case, and the row must not stop on the way to its target.
    pub(super) retained: bool,
}

fn service_order(left: &ScheduleRow, right: &ScheduleRow) -> std::cmp::Ordering {
    left.spare
        .cmp(&right.spare)
        .then_with(|| right.overlapping.cmp(&left.overlapping))
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
///
/// The floor: a row that gets no slice above gets `MIN_SLICE` from at most half the rest,
/// in service order, when it has a retained target or is a spare row. For a row with a
/// retained target the floor pays point checks only, so a row on the way to its escape
/// target does not stop while other rows search, and a failed check waits for a search
/// slice as before. For a spare row the floor is a search slice: most spare searches end
/// in it. A batch that does not fit takes turns by tick, as `share`.
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
    let mut check_only = Vec::new();
    let mut served = Vec::new();
    let mut waiting: Vec<ScheduleRow> = Vec::new();
    let mut floor = Vec::new();
    for owner_rows in rows.chunk_by(|left, right| left.owner == right.owner) {
        let split = owner_rows.partition_point(|row| !row.has_scan);
        let (fresh, continuing) = owner_rows.split_at(split);
        served.clear();
        let mut fresh_spent = 0usize;
        // The fresh rows that the loop below gave a slice or a turn.
        let mut covered = 0usize;
        for batch in fresh.chunk_by(|left, right| service_order(left, right).is_eq()) {
            let cost = FRESH_SLICE * batch.len();
            if fresh_spent + cost <= OWNER_WORK / 2 {
                fresh_spent += cost;
                covered += batch.len();
                served.extend(batch.iter().map(|row| (row.row, FRESH_SLICE)));
            } else {
                if served.is_empty() {
                    fresh_spent += share(batch, OWNER_WORK / 2, tick, &mut served);
                    covered += batch.len();
                }
                break;
            }
        }
        let mut rest = OWNER_WORK.saturating_sub(fresh_spent);
        let oldest = continuing
            .chunk_by(|left, right| service_order(left, right).is_eq())
            .next()
            .unwrap_or(&[]);
        waiting.clear();
        waiting.extend(
            fresh[covered..]
                .iter()
                .chain(&continuing[oldest.len()..])
                .filter(|row| row.retained || row.spare),
        );
        floor.clear();
        let mut credit = rest / 2;
        for batch in waiting.chunk_by(|left, right| service_order(left, right).is_eq()) {
            if credit < MIN_SLICE {
                break;
            }
            let from = floor.len();
            credit -= share(batch, credit.min(batch.len() * MIN_SLICE), tick, &mut floor);
            // The rows of a batch are all spare rows or none is (the service key).
            if !batch[0].spare {
                check_only.extend(floor[from..].iter().map(|entry| entry.0));
            }
        }
        rest -= floor.len() * MIN_SLICE;
        allowances.append(&mut floor);
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
    check_only.sort_unstable();
    counters.phase_bytes = counters.phase_bytes.max(
        (rows.capacity() + waiting.capacity()) * std::mem::size_of::<ScheduleRow>()
            + (allowances.capacity() + served.capacity() + floor.capacity())
                * std::mem::size_of::<(usize, usize)>()
            + check_only.capacity() * std::mem::size_of::<usize>(),
    );
    RecoverySchedule {
        allowances,
        check_only,
    }
}

pub(super) enum SliceOutcome {
    /// The search ended: the nearest free point, or none.
    Done(Option<Vec2>),
    /// The slice is spent; keep this scan.
    Paused(RecoveryScan),
}

/// The result of a metered point check with no search.
pub(super) enum PointCheck {
    Clear,
    Blocked,
    /// The slice is spent; keep this check.
    Paused(RecoveryScan),
    /// The slice is spent and the check keeps no cursor.
    Unpaid,
}

/// The pool slot search of a group escape.
#[derive(Clone, Copy)]
pub(super) struct PoolSearch<'a> {
    pub(super) index: &'a MovementPoolIndex,
    pub(super) unit_id: u32,
    /// The claim key that this mover may keep.
    pub(super) allowed: Option<(u64, u32, u32)>,
    /// A slot that the search never takes (the slot that a stalled mover leaves).
    pub(super) skip: Option<(u64, u32, u32)>,
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
    /// True for the stall spare-slot search: the pool stage only, and soft bodies are no
    /// obstacle (`KnownSet::ignore_soft`).
    pub(super) pool_only: bool,
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

    /// True when `scan` is a slice of this search (the cell size of its view may differ).
    fn same_search(&self, scan: &RecoveryScan) -> bool {
        scan.origin.x.to_bits() == self.origin.x.to_bits()
            && scan.origin.y.to_bits() == self.origin.y.to_bits()
            && scan.radius.to_bits() == self.radius.to_bits()
            && scan.pool.is_some() == self.pool.is_some()
            && !scan.is_check()
    }

    /// True when `scan` is the stored check of `point` for this actor.
    fn same_check(&self, scan: &RecoveryScan, point: Vec2, slot: bool) -> bool {
        scan.is_check()
            && scan.origin.x.to_bits() == self.origin.x.to_bits()
            && scan.origin.y.to_bits() == self.origin.y.to_bits()
            && scan.radius.to_bits() == self.radius.to_bits()
            && matches!(
                scan.stage,
                RecoveryStage::Verify { point: stored, slot: stored_slot }
                    if stored.x.to_bits() == point.x.to_bits()
                        && stored.y.to_bits() == point.y.to_bits()
                        && stored_slot == slot
            )
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
                local: false,
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

    /// The own-pool claims of the claim cell that holds `point`.
    fn point_claims(&self) -> Option<(u64, &ClaimSet, f32)> {
        self.slots.map(|(pool, set)| (pool, set, 0.0))
    }

    /// The cost of the local proof of the check of `point`: the bodies that can touch it,
    /// and the claims of its claim cell.
    fn local_cost(&self, known: &KnownSet, point: Vec2) -> usize {
        known.region_cost(point, self.radius + known.view.half(), self.point_claims())
    }

    fn local_digest(&self, known: &KnownSet, point: Vec2, meter: &mut Meter) -> Option<[u64; 3]> {
        known.region_digest(
            point,
            self.radius + known.view.half(),
            self.point_claims(),
            meter,
        )
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

    /// Restores the reserve of a slice that resumes: for its paused probe, or for the
    /// region that it examined. The pool stage reads cells with no slot in them before it
    /// starts the next slot, so it needs the reserve from its first unit.
    fn resume_query(&self, known: &KnownSet, state: &mut RecoveryScan, slice: &mut Slice) -> bool {
        let point = state.query.map_or(self.origin, |query| query.point);
        self.reserve(known, state, point, slice)
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
            match self.precheck(known, state, cursor, pool, slice)? {
                true => {
                    cursor.checked = true;
                    cursor.at = 0;
                    cursor.item = 0;
                }
                false => {
                    cursor.checked = true;
                    cursor.done = true;
                    return Some(());
                }
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
                            let skipped = pool.skip.is_some_and(|key| {
                                self.slots.is_some_and(|(id, _)| {
                                    key == (id, point.x.to_bits(), point.y.to_bits())
                                })
                            });
                            if !skipped && !self.claimed(point, pool.allowed, &mut slice.meter) {
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

    /// The metered `has_potential_free_slot`: two units for its constant-time part, then one
    /// unit for each cell and each slot of the local part. `None` when the slice paused.
    fn precheck(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        cursor: &mut RecoveryPoolCursor,
        pool: PoolSearch,
        slice: &mut Slice,
    ) -> Option<bool> {
        if !self.reserve(known, state, self.origin, slice) {
            return None;
        }
        if !cursor.local {
            if slice.meter.left < slice.meter.floor + 2 {
                return None;
            }
            slice.meter.step();
            slice.meter.step();
            slice.counters.pool_work += 2;
            if pool.index.potential_free_without_local(
                pool.unit_id,
                self.radius,
                pool.allowed.map(|(_, x, y)| (x, y)),
            ) {
                return Some(true);
            }
            cursor.local = true;
            cursor.at = 0;
            cursor.item = 0;
        }
        let reach = 2.0 * self.radius;
        let grid = pool.index.slot_grid();
        let points = &pool.index.points;
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
            if by_cells && cursor.item == 0 {
                if !slice.meter.step() {
                    return None;
                }
                slice.counters.pool_work += 1;
            }
            while (cursor.item as usize) < count {
                if !slice.meter.step() {
                    return None;
                }
                slice.counters.pool_work += 1;
                let slot = if by_cells {
                    members[cursor.item as usize]
                } else {
                    cursor.at as usize
                };
                if points[slot].position.distance_squared(self.origin) <= reach * reach
                    && pool
                        .index
                        .local_slot_supports(slot, pool.unit_id, self.radius)
                {
                    return Some(true);
                }
                cursor.item += 1;
            }
            cursor.item = 0;
            cursor.at += 1;
        }
        Some(false)
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

    /// Checks a point against the current state: its claim (a slot keeps the allowed
    /// claim) and the known bodies, with the tolerance of a slot or of a ring candidate.
    /// `None` when the slice paused. `reserve` keeps the local proof of a pause.
    fn verify_point(
        &self,
        known: &KnownSet,
        state: &mut RecoveryScan,
        point: Vec2,
        slot: bool,
        reserve: bool,
        slice: &mut Slice,
    ) -> Option<bool> {
        if state.query.is_none() {
            if reserve {
                if !self.reserve(known, state, point, slice) {
                    return None;
                }
            } else {
                slice.meter.floor = 0;
            }
            if slice.meter.left < slice.meter.floor + 2 {
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
                    let ring = if self.pool_only {
                        None
                    } else {
                        let Some(ring) = self.ring_stage(known, state, slice) else {
                            return Step::Pause;
                        };
                        ring
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
                    return match self.verify_point(known, state, point, slot, true, slice) {
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

    /// Runs one slice of at most `allowance` metered work.
    #[cfg(test)]
    pub(super) fn run(
        &self,
        known: &KnownSet,
        previous: Option<RecoveryScan>,
        allowance: usize,
        owner: u8,
        counters: &mut RecoveryCounters,
    ) -> SliceOutcome {
        self.run_metered(known, previous, allowance, owner, counters)
            .0
    }

    /// The check state of `point`: no ring, no proof of a region, one point to verify.
    fn check_state(&self, known: &KnownSet, point: Vec2, slot: bool) -> RecoveryScan {
        RecoveryScan {
            origin: self.origin,
            radius: self.radius,
            ring: 0,
            side: 0,
            step: 0,
            best: None,
            examined: 0.0,
            digest: [0; 3],
            invalidations: 0,
            relaxed: true,
            cell: known.cell_bits(),
            stage: RecoveryStage::Verify { point, slot },
            pool: None,
            query: None,
            spanned: true,
        }
    }

    /// Runs the metered check of one point with no search (a retained target, or the own
    /// position): the predicate of `verify_point`. `previous` is the stored check of this
    /// point; it resumes only with its local proof. A `resumable` check keeps the reserve
    /// of that proof and returns `Paused` when the slice is spent; another check returns
    /// `Unpaid`. Returns the result and the work used. A partial query is never `Clear`.
    #[expect(
        clippy::too_many_arguments,
        reason = "one point check needs the actor, the point, the slice and the accounting"
    )]
    pub(super) fn check_point(
        &self,
        known: &KnownSet,
        previous: Option<RecoveryScan>,
        point: Vec2,
        slot: bool,
        allowance: usize,
        resumable: bool,
        owner: u8,
        counters: &mut RecoveryCounters,
    ) -> (PointCheck, usize) {
        let mut slice = Slice {
            meter: Meter::new(allowance),
            allowance,
            started: 0,
            counters,
        };
        slice.counters.checks += 1;
        let mut state = previous
            .filter(|scan| resumable && self.same_check(scan, point, slot))
            .unwrap_or_else(|| self.check_state(known, point, slot));
        if state.cell != known.cell_bits() {
            // The view cell size changed: the cursor is void.
            state.cell = known.cell_bits();
            state.query = None;
        }
        let mut result = None;
        if let Some(query) = state.query {
            match self.local_digest(known, query.point, &mut slice.meter) {
                // The proof holds: restore the reserve of the next pause. A slice that
                // cannot do more keeps the cursor as it is.
                Some(digest) if digest == query.digest => {
                    if !self.reserve(known, &mut state, query.point, &mut slice) {
                        result = Some(PointCheck::Paused(state));
                    }
                }
                Some(_) => state.query = None,
                // The slice cannot pay the proof: keep the cursor as it is.
                None => result = Some(PointCheck::Paused(state)),
            }
        }
        let result = result.unwrap_or_else(|| {
            match self.verify_point(known, &mut state, point, slot, resumable, &mut slice) {
                Some(true) => PointCheck::Clear,
                Some(false) => PointCheck::Blocked,
                None if !resumable => PointCheck::Unpaid,
                None => {
                    slice.meter.floor = 0;
                    if let Some(mut query) = state.query {
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
                    PointCheck::Paused(state)
                }
            }
        });
        let Slice {
            meter, counters, ..
        } = slice;
        counters.absorb(&meter);
        counters.charge(owner, meter.used, meter.used);
        (result, meter.used)
    }

    /// Runs one slice of at most `allowance` metered work. Returns the work used too.
    pub(super) fn run_metered(
        &self,
        known: &KnownSet,
        previous: Option<RecoveryScan>,
        allowance: usize,
        owner: u8,
        counters: &mut RecoveryCounters,
    ) -> (SliceOutcome, usize) {
        let mut slice = Slice {
            meter: Meter::new(allowance),
            allowance,
            started: 0,
            counters,
        };
        slice.counters.slices += 1;
        let previous = previous.filter(|scan| self.same_search(scan));
        let mut state = match previous {
            None => self.fresh(known, 0, false),
            Some(scan) if scan.cell != known.cell_bits() => {
                // The view cell size changed: a paused query cursor is void. It counts as
                // an invalidation, so a view whose largest radius changes again and again
                // makes the scan relaxed. A relaxed scan keeps its place and checks its
                // point at the end; only the paused query restarts.
                slice.counters.invalidations += 1;
                let invalidations = scan.invalidations.saturating_add(1);
                if self.proving(&scan) {
                    self.fresh(known, invalidations, false)
                } else {
                    RecoveryScan {
                        cell: known.cell_bits(),
                        query: None,
                        invalidations,
                        ..scan
                    }
                }
            }
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

    fn finish(&self, owner: u8, slice: Slice, outcome: SliceOutcome) -> (SliceOutcome, usize) {
        let Slice {
            meter, counters, ..
        } = slice;
        match outcome {
            SliceOutcome::Done(_) => counters.completions += 1,
            SliceOutcome::Paused(_) => counters.pauses += 1,
        }
        counters.absorb(&meter);
        counters.charge(owner, meter.used, meter.used);
        (outcome, meter.used)
    }
}

/// The claim exchange rule of a soft body (design section 7, B2): the tick-start facts
/// that `route::Planner` reads.
#[derive(Clone, Copy)]
pub(super) struct SoftRule<'f> {
    /// True for each solver row that was at rest at tick start.
    pub(super) stationary: &'f [bool],
    /// For each resting row: the other owners with a body in contact range of it at tick
    /// start (sorted).
    pub(super) witness: &'f [Vec<u8>],
}

/// The schedule read of one row: its contacts and its tick-start overlap.
struct RowRead {
    row: usize,
    contacts: Range<u32>,
    overlapping: bool,
}

/// The reads of the schedule rows, in row order.
#[derive(Default)]
pub(super) struct ScheduleReads {
    rows: Vec<RowRead>,
    contact_rows: Vec<u32>,
}

impl ScheduleReads {
    /// Starts the read of solver row `row`. Rows come in increasing order.
    pub(super) fn begin(&mut self, row: usize) {
        debug_assert!(self.rows.last().is_none_or(|last| last.row < row));
        let at = self.contact_rows.len() as u32;
        self.rows.push(RowRead {
            row,
            contacts: at..at,
            overlapping: false,
        });
    }

    /// Adds a contact of the row of the last `begin`: a solver row in contact range that
    /// the owner view does not hold.
    pub(super) fn contact(&mut self, other: usize) {
        self.contact_rows.push(other as u32);
        if let Some(read) = self.rows.last_mut() {
            read.contacts.end = self.contact_rows.len() as u32;
        }
    }

    pub(super) fn overlap(&mut self, overlapping: bool) {
        if let Some(read) = self.rows.last_mut() {
            read.overlapping = overlapping;
        }
    }

    /// The contacts of the row of the last `begin` (test oracle).
    #[cfg(test)]
    pub(super) fn last_contacts(&self) -> &[u32] {
        self.rows.last().map_or(&[], |read| {
            &self.contact_rows[read.contacts.start as usize..read.contacts.end as usize]
        })
    }

    fn find(&self, row: usize) -> Option<&RowRead> {
        self.rows
            .binary_search_by_key(&row, |read| read.row)
            .ok()
            .map(|at| &self.rows[at])
    }

    fn bytes(&self) -> usize {
        self.rows.capacity() * std::mem::size_of::<RowRead>()
            + self.contact_rows.capacity() * std::mem::size_of::<u32>()
    }
}

/// The known-body indexes of one solver pass, shared by all rows. Built on first use.
pub(super) struct RecoveryWorld<'f> {
    visibility: Option<&'f dyn super::MovementVisibility>,
    soft: Option<SoftRule<'f>>,
    /// One view for each owner; one shared view (key `None`) with no visibility basis and
    /// no claim exchange.
    views: RefCell<BTreeMap<Option<u8>, KnownView>>,
    /// Under fog: the view membership of each frame row, for each owner that has a
    /// schedule row or a built view. One check for each row and owner in a pass.
    members: RefCell<BTreeMap<u8, Vec<bool>>>,
    /// Under fog: the bodies that each owner does not hold in its view, for the contact
    /// reads of its schedule rows (design `2026-11-20-bounded-contact`, rule G2).
    hidden: RefCell<BTreeMap<u8, HiddenIndex>>,
    /// The largest radius of the frame.
    max_radius: f32,
    reads: OnceCell<ScheduleReads>,
    view_work: Cell<u64>,
    contact_work: Cell<u64>,
    peak_bytes: Cell<usize>,
}

impl<'f> RecoveryWorld<'f> {
    pub(super) fn new(
        visibility: Option<&'f dyn super::MovementVisibility>,
        soft: Option<SoftRule<'f>>,
        max_radius: f32,
    ) -> Self {
        Self {
            visibility,
            soft,
            views: RefCell::new(BTreeMap::new()),
            members: RefCell::new(BTreeMap::new()),
            hidden: RefCell::new(BTreeMap::new()),
            max_radius,
            reads: OnceCell::new(),
            view_work: Cell::new(0),
            contact_work: Cell::new(0),
            peak_bytes: Cell::new(0),
        }
    }

    /// Stores the schedule reads of this pass (one time).
    pub(super) fn set_reads(&self, reads: ScheduleReads) {
        let stored = self.reads.set(reads).is_ok();
        debug_assert!(stored, "one schedule for each pass");
        self.note_bytes(0);
    }

    fn view_key(&self, owner: u8) -> Option<u8> {
        (self.visibility.is_some() || self.soft.is_some()).then_some(owner)
    }

    /// True when `owner` may know solver row `row` without contact: its own body, or a
    /// body that its basis permits. The same rule as `known` in the frame.
    pub(super) fn member(&self, owner: u8, row: usize, units: &[MovementFrameUnit]) -> bool {
        let other = &units[row];
        self.visibility.is_none_or(|basis| {
            other.owner_slot == owner || basis.unit_permitted(owner, other.unit_id)
        })
    }

    /// True when solver row `row` is a soft body for `owner`: in claim exchange mode, a
    /// body of another owner that was not at rest at tick start, or that no body of
    /// `owner` had in contact range then. The stall spare-slot search ignores it.
    pub(super) fn soft(&self, owner: u8, row: usize, units: &[MovementFrameUnit]) -> bool {
        self.soft.is_some_and(|rule| {
            units[row].owner_slot != owner
                && !(rule.stationary[row] && rule.witness[row].binary_search(&owner).is_ok())
        })
    }

    /// True when `row` is a schedule row of this pass (it has a schedule read).
    fn scheduled(&self, row: usize) -> bool {
        self.reads
            .get()
            .is_some_and(|reads| reads.find(row).is_some())
    }

    /// True when the schedule read of `row` found an overlap at tick start.
    fn start_overlap(&self, row: usize) -> bool {
        self.reads
            .get()
            .and_then(|reads| reads.find(row))
            .is_some_and(|read| read.overlapping)
    }

    /// Follows the move of frame row `row` in every view that holds it. Each view that
    /// changes costs two units of view work (one remove and one insert).
    pub(super) fn relocate(&self, row: usize, from: Vec2, to: Vec2, radius: f32) {
        let mut changed = 0;
        for view in self.views.borrow_mut().values_mut() {
            changed += u64::from(view.relocate(row, from, to, radius));
        }
        self.view_work.set(self.view_work.get() + 2 * changed);
    }

    /// Under fog: the view membership of each row for `owner`, computed one time in a
    /// pass (one check for each row, counted as view work).
    fn ensure_members(&self, owner: u8, units: &[MovementFrameUnit]) {
        if self.members.borrow().contains_key(&owner) {
            return;
        }
        let members: Vec<bool> = (0..units.len())
            .map(|row| self.member(owner, row, units))
            .collect();
        self.view_work
            .set(self.view_work.get() + units.len() as u64);
        self.members.borrow_mut().insert(owner, members);
        self.note_bytes(0);
    }

    fn ensure_view(&self, owner: u8, units: &[MovementFrameUnit]) {
        let key = self.view_key(owner);
        if self.views.borrow().contains_key(&key) {
            return;
        }
        let rows: Vec<usize> = if self.visibility.is_some() {
            // The membership pass is shared with the hidden index (rule G2).
            self.ensure_members(owner, units);
            let members = self.members.borrow();
            let members = &members[&owner];
            (0..units.len()).filter(|&row| members[row]).collect()
        } else {
            // One membership check for each row: with no basis every row is a member.
            self.view_work
                .set(self.view_work.get() + units.len() as u64);
            (0..units.len()).collect()
        };
        let members: Vec<_> = rows
            .into_iter()
            .map(|row| {
                let unit = &units[row];
                (
                    row,
                    unit.position,
                    unit.radius,
                    key.is_some() && self.soft(owner, row, units),
                )
            })
            .collect();
        // One insert for each member.
        self.view_work
            .set(self.view_work.get() + members.len() as u64);
        let view = KnownView::build_soft(&members, units.len());
        self.views.borrow_mut().insert(key, view);
        self.note_bytes(members.capacity() * std::mem::size_of::<(usize, Vec2, f32, bool)>());
    }

    fn note_bytes(&self, scratch: usize) {
        let views: usize = self.views.borrow().values().map(KnownView::bytes).sum();
        let members: usize = self.members.borrow().values().map(Vec::capacity).sum();
        let hidden: usize = self.hidden.borrow().values().map(HiddenIndex::bytes).sum();
        let reads = self.reads.get().map_or(0, ScheduleReads::bytes);
        self.peak_bytes.set(
            self.peak_bytes
                .get()
                .max(views + members + hidden + reads + scratch),
        );
    }

    /// The contact read of schedule row `row` (fog only; design
    /// `2026-11-20-bounded-contact`, rule G2): calls `contact` for each body in contact
    /// range of the row (the contact rule of `known`) that its owner view does not hold.
    /// It reads the hidden index of the owner only, so own and permitted bodies cost
    /// nothing. Returns `(cells, hidden bodies tested)`. Rows must be at their tick-start
    /// positions.
    pub(super) fn contact_read(
        &self,
        row: usize,
        units: &[MovementFrameUnit],
        mut contact: impl FnMut(usize),
    ) -> (usize, usize) {
        debug_assert!(
            self.visibility.is_some(),
            "contact reads exist under fog only"
        );
        let unit = &units[row];
        let owner = unit.owner_slot;
        if !self.hidden.borrow().contains_key(&owner) {
            self.ensure_members(owner, units);
            let members = self.members.borrow();
            let index = HiddenIndex::new(&members[&owner], units, self.max_radius);
            drop(members);
            // One insert for each hidden row.
            self.view_work
                .set(self.view_work.get() + index.rows.len() as u64);
            self.hidden.borrow_mut().insert(owner, index);
            self.note_bytes(0);
        }
        let hidden = self.hidden.borrow();
        let index = &hidden[&owner];
        if index.rows.is_empty() {
            return (0, 0);
        }
        index.grid.visit_within(
            &index.points,
            unit.position,
            index.reach(unit.radius),
            |at| {
                let other = index.rows[at] as usize;
                if touch(unit.position, unit.radius, &units[other]).1 {
                    contact(other);
                }
            },
        )
    }

    /// Test hook: builds the owner view of `owner` as a row query does.
    #[cfg(test)]
    pub(super) fn build_view(&self, owner: u8, units: &[MovementFrameUnit]) {
        self.ensure_view(owner, units);
    }

    /// Drops the hidden indexes: the schedule reads are done.
    pub(super) fn end_contact_reads(&self) {
        self.note_bytes(0);
        self.hidden.borrow_mut().clear();
    }

    /// The contacts of the actor of row `row`, from its schedule read: the bodies in
    /// contact range at tick start that its owner view does not hold (the contact rule of
    /// `known`), at their positions now. `hard_only` leaves out the soft ones.
    fn contacts(
        &self,
        owner: u8,
        row: usize,
        units: &[MovementFrameUnit],
        hard_only: bool,
    ) -> Contacts {
        if self.visibility.is_none() {
            return Contacts::default();
        }
        let Some((reads, read)) = self
            .reads
            .get()
            .and_then(|reads| Some((reads, reads.find(row)?)))
        else {
            debug_assert!(
                false,
                "a known-body query needs the schedule read of its row"
            );
            return Contacts::default();
        };
        let rows = &reads.contact_rows[read.contacts.start as usize..read.contacts.end as usize];
        if rows.is_empty() {
            return Contacts::default();
        }
        self.contact_work
            .set(self.contact_work.get() + rows.len() as u64);
        let contacts = Contacts::new(
            rows.iter()
                .map(|&other| other as usize)
                .filter(|&other| !(hard_only && self.soft(owner, other, units)))
                .map(|other| (units[other].position, units[other].radius)),
        );
        self.note_bytes(contacts.bytes());
        contacts
    }

    pub(super) fn view_work(&self) -> u64 {
        self.view_work.get()
    }

    /// The contacts that the rows read from their schedule reads (class P).
    pub(super) fn contact_work(&self) -> u64 {
        self.contact_work.get()
    }

    pub(super) fn peak_bytes(&self) -> usize {
        self.peak_bytes.get()
    }
}

/// The bodies that one owner does not hold in its view, at tick start, for the contact
/// reads (design `2026-11-20-bounded-contact`, rule G2).
///
/// The cell is at least the largest contact reach `max radius + hidden max radius +
/// CONTACT_GAP` (with margin), so a contact read scans at most 4 cells on each axis
/// (`grid` rule G1). It holds no own and no permitted body, so their density costs a
/// contact read nothing.
struct HiddenIndex {
    /// The frame row of each hidden body.
    rows: Vec<u32>,
    points: Vec<crate::grid::GridPoint>,
    grid: crate::grid::UniformGrid,
    /// The largest radius of a hidden body.
    max_radius: f32,
}

impl HiddenIndex {
    fn new(members: &[bool], units: &[MovementFrameUnit], frame_max_radius: f32) -> Self {
        let rows: Vec<u32> = (0..units.len())
            .filter(|&row| !members[row])
            .map(|row| row as u32)
            .collect();
        let points: Vec<_> = rows
            .iter()
            .map(|&row| crate::grid::GridPoint {
                id: row,
                position: units[row as usize].position,
            })
            .collect();
        let max_radius = rows
            .iter()
            .map(|&row| units[row as usize].radius)
            .fold(0.0_f32, f32::max);
        let cell = Self::reach_of(frame_max_radius, max_radius) * 1.001;
        let grid = crate::grid::UniformGrid::new(&points, cell);
        Self {
            rows,
            points,
            grid,
            max_radius,
        }
    }

    /// The reach of a contact read of an actor of `radius`: every hidden body in contact
    /// range is inside it. The relative margin keeps the f32 filter of the read above the
    /// contact rule of `touch` at its edge.
    fn reach_of(radius: f32, hidden_max_radius: f32) -> f32 {
        (radius + hidden_max_radius + CONTACT_GAP) * (1.0 + 1.0e-6)
    }

    fn reach(&self, radius: f32) -> f32 {
        Self::reach_of(radius, self.max_radius)
    }

    fn bytes(&self) -> usize {
        self.rows.capacity() * std::mem::size_of::<u32>()
            + self.points.capacity() * std::mem::size_of::<crate::grid::GridPoint>()
            + self.grid.bytes()
    }
}

/// True when a body of the frame overlaps a disc of `radius` at `position`, and the contact
/// rule of `known`: `(overlap, contact)` of solver row `other` for an actor at `position`.
pub(super) fn touch(position: Vec2, radius: f32, other: &MovementFrameUnit) -> (bool, bool) {
    let distance = position.distance(other.position);
    let reach = radius + other.radius;
    (distance < reach - 1.0e-5, distance <= reach + CONTACT_GAP)
}

/// The result of a metered point check of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Check {
    /// No known body and no claim blocks the point.
    Clear,
    Blocked,
    /// The check did not end and keeps its place (or the row has no allowance): the row
    /// holds its position for this tick.
    Pending,
    /// The slice could not pay a check that keeps no cursor: the stored job continues
    /// without it. Never treat it as `Clear`.
    Unpaid,
}

/// The recovery state of one frame row in one tick.
pub(super) struct RowRecovery<'c, 'f> {
    pub(super) row: usize,
    pub(super) owner: u8,
    /// The metered work that this row may still do in this tick.
    pub(super) allowance: usize,
    /// True when the allowance pays point checks only (the check floor).
    pub(super) check_only: bool,
    pub(super) previous: Option<RecoverySearch>,
    /// True when a check of this tick ended or dropped the stored scan.
    pub(super) consumed: bool,
    pub(super) next_jobs: &'c mut BTreeMap<usize, RecoverySearch>,
    pub(super) counters: &'c mut RecoveryCounters,
    pub(super) world: &'c RecoveryWorld<'f>,
}

impl RowRecovery<'_, '_> {
    fn same_kind(&self, kind: RecoveryKind) -> Option<RecoverySearch> {
        self.previous.filter(|job| job.kind == kind)
    }

    /// The scan that the job of this row still stores.
    fn stored(&self, kind: RecoveryKind) -> Option<RecoveryScan> {
        if self.consumed {
            return None;
        }
        self.same_kind(kind).and_then(|job| job.scan)
    }

    fn keep(&mut self, kind: RecoveryKind, scan: Option<RecoveryScan>) {
        let age = self
            .same_kind(kind)
            .map_or(1, |job| job.age.saturating_add(1));
        self.counters.max_age = self.counters.max_age.max(age);
        self.next_jobs
            .insert(self.row, RecoverySearch { kind, age, scan });
    }

    /// Keeps the job one tick older: the row has no allowance for its next step.
    fn wait(&mut self, kind: RecoveryKind) {
        self.counters.waits += 1;
        let scan = self.stored(kind);
        self.keep(kind, scan);
    }

    /// True when this row may run a search slice now. Otherwise it keeps the job one tick
    /// older.
    pub(super) fn may_search(&mut self, kind: RecoveryKind) -> bool {
        if self.allowance > 0 && !self.check_only {
            return true;
        }
        self.wait(kind);
        false
    }

    /// Runs `run` over the known bodies of this row: its owner view and its contacts.
    fn with_known<R>(
        &mut self,
        units: &[MovementFrameUnit],
        ignore_soft: bool,
        run: impl FnOnce(&KnownSet, &mut RecoveryCounters) -> R,
    ) -> R {
        self.world.ensure_view(self.owner, units);
        let contacts = self
            .world
            .contacts(self.owner, self.row, units, ignore_soft);
        let views = self.world.views.borrow();
        let known = KnownSet {
            view: &views[&self.world.view_key(self.owner)],
            contacts: &contacts,
            own_key: Some(body_key(units[self.row].position, units[self.row].radius)),
            ignore_soft,
        };
        run(&known, self.counters)
    }

    /// The metered check of `point` for the actor of `scan`, with the predicate of
    /// `EscapeScan::verify_point` (design section 7, B1).
    ///
    /// - With no stored scan, the check uses the whole allowance. When the slice is spent
    ///   it stores its cursor as the job of the row (`Pending`) and resumes in a later
    ///   tick.
    /// - With a stored search scan, or the stored check of the own position, the check
    ///   uses at most half the allowance and stores nothing (`Unpaid` when it does not
    ///   end): the stored job continues without it.
    /// - With no allowance the row waits (`Pending`). `idle` is for the check of the own
    ///   position: a row with no job and no overlap at tick start then keeps no job.
    pub(super) fn check(
        &mut self,
        kind: RecoveryKind,
        scan: &EscapeScan,
        point: Vec2,
        slot: bool,
        idle: bool,
        units: &[MovementFrameUnit],
    ) -> Check {
        let stored = self.stored(kind);
        let mine = stored.filter(|stored| scan.same_check(stored, point, slot));
        let other = stored.is_some_and(|stored| {
            mine.is_none() && (!stored.is_check() || scan.same_check(&stored, scan.origin, false))
        });
        if stored.is_some() && mine.is_none() && !other {
            // The check of a point that the row left: drop it.
            self.consumed = true;
        }
        if self.allowance == 0 {
            if !self.world.scheduled(self.row) {
                // The row became blocked in this tick: it has no schedule read and no
                // allowance. Nothing is proved; the search that follows makes it wait.
                return Check::Unpaid;
            }
            if !idle || self.previous.is_some() || self.world.start_overlap(self.row) {
                self.wait(kind);
            }
            return Check::Pending;
        }
        let owner = self.owner;
        if other {
            let allowance = self.allowance / 2;
            let (result, used) = self.with_known(units, false, |known, counters| {
                scan.check_point(known, None, point, slot, allowance, false, owner, counters)
            });
            self.allowance -= used;
            return match result {
                PointCheck::Clear => Check::Clear,
                PointCheck::Blocked => Check::Blocked,
                PointCheck::Paused(_) | PointCheck::Unpaid => Check::Unpaid,
            };
        }
        let allowance = self.allowance;
        let (result, used) = self.with_known(units, false, |known, counters| {
            scan.check_point(known, mine, point, slot, allowance, true, owner, counters)
        });
        self.allowance -= used;
        match result {
            PointCheck::Clear => {
                self.consumed = true;
                Check::Clear
            }
            PointCheck::Blocked => {
                self.consumed = true;
                Check::Blocked
            }
            PointCheck::Paused(state) => {
                self.keep(kind, Some(state));
                Check::Pending
            }
            PointCheck::Unpaid => {
                debug_assert!(false, "a resumable check pauses");
                self.wait(kind);
                Check::Pending
            }
        }
    }

    /// A metered query of the known bodies at `point` with the rest of the allowance and
    /// no cursor: `Some(true)` when none is within `radius + its radius - epsilon`.
    /// `None` when the allowance cannot pay it; never treat that as clear.
    pub(super) fn peek(
        &mut self,
        point: Vec2,
        radius: f32,
        epsilon: f32,
        units: &[MovementFrameUnit],
    ) -> Option<bool> {
        if self.allowance == 0 {
            return None;
        }
        let (allowance, owner) = (self.allowance, self.owner);
        let (result, used) = self.with_known(units, false, |known, counters| {
            let mut meter = Meter::new(allowance);
            let result = known.probe(KnownSet::fresh_query(point), radius, epsilon, &mut meter);
            counters.checks += 1;
            counters.absorb(&meter);
            counters.charge(owner, meter.used, meter.used);
            (result, meter.used)
        });
        self.allowance -= used;
        match result {
            Probe::Clear => Some(true),
            Probe::Blocked => Some(false),
            Probe::Pending(_) => None,
        }
    }

    /// Runs one slice of `scan` for `kind` over the known bodies of this row, with the
    /// rest of its allowance. `None` when the search found no point or did not end in this
    /// slice; an unfinished job stays for the next tick.
    pub(super) fn search(
        &mut self,
        kind: RecoveryKind,
        scan: &EscapeScan,
        units: &[MovementFrameUnit],
    ) -> Option<Vec2> {
        if !self.may_search(kind) {
            return None;
        }
        let previous = self.stored(kind).filter(|stored| !stored.is_check());
        let (allowance, owner) = (self.allowance, self.owner);
        let (outcome, used) = self.with_known(units, scan.pool_only, |known, counters| {
            scan.run_metered(known, previous, allowance, owner, counters)
        });
        self.allowance -= used;
        if matches!(kind, RecoveryKind::Spare(_)) {
            self.counters.spare_slot_work += used as u64;
        }
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

    /// The origin of the spare-slot search of a mover at `position`: the origin of its
    /// stored search while the mover stays within its radius of it, so a stalled mover
    /// that creeps does not start its search again in each tick.
    pub(super) fn spare_origin(&self, kind: RecoveryKind, position: Vec2, radius: f32) -> Vec2 {
        self.stored(kind)
            .filter(|scan| {
                !scan.is_check()
                    && scan.radius.to_bits() == radius.to_bits()
                    && scan.origin.distance(position) <= radius
            })
            .map_or(position, |scan| scan.origin)
    }

    /// True when the last search or check of this row did not end.
    pub(super) fn pending(&self) -> bool {
        self.next_jobs.contains_key(&self.row)
    }

    /// True when the job of the last tick stores no scan of `kind`: a search that ends in
    /// this tick read only the state of this tick.
    #[cfg(test)]
    pub(super) fn fresh(&self, kind: RecoveryKind) -> bool {
        self.same_kind(kind).is_none_or(|job| job.scan.is_none())
    }
}

/// Bytes retained by recovery jobs after a tick.
pub(super) fn retained_bytes(jobs: usize) -> usize {
    jobs * (std::mem::size_of::<RecoverySearch>() + std::mem::size_of::<u32>())
}
