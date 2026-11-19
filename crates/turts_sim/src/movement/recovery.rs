//! Bounded, fair blocked-recovery escape search (design `2026-11-18-bounded-recovery`).
//!
//! - One schedule pass at tick start gives each blocked row a work allowance. Jobs with no
//!   scan share a fresh budget equally. The oldest job with a scan (then the physical key
//!   x, y, radius) gets the rest; rows with an equal key share it equally. No unit id and no
//!   hash order selects service.
//! - A slice continues today's ring scan from a stored cursor. Before it continues, it
//!   proves that no permitted fact that decides an examined candidate changed (an
//!   order-free digest). A changed region restarts the scan. After `RELAX_AFTER` restarts,
//!   or when the proof costs more than half the slice, the scan runs relaxed and commits
//!   only a point that is free now.
//! - Slice ends use a public charge: candidates, a fixed cell charge, permitted bodies that
//!   touch the candidate disc, and claim keys of the own pool. Hidden bodies and the frame
//!   maximum radius never change it. The actual visits go to the diagnostics.

use bevy_math::Vec2;
use std::collections::{BTreeMap, HashSet};

use super::frame::MovementFrameUnit;
use super::spread::{finite_hex_ring_limit, minimum_unseen_hex_radius, nearest_angle_order};
use super::{RecoveryKind, RecoveryScan, RecoverySearch};

/// Public recovery charge of one owner in one solver pass, before the overshoot terms.
/// It is half the 65,536 owner envelope: the public charge omits non-touching and hidden
/// bodies, so actual operations run above it (the stress fixture measures the ratio). It
/// is separate from the route allowance `route::OWNER_CAP`.
pub(crate) const OWNER_WORK: usize = 32_768;
/// The share of a job with no scan. Most searches end inside it.
pub(crate) const FRESH_SLICE: usize = 1_024;
/// Failed region proofs before a job runs relaxed.
pub(crate) const RELAX_AFTER: u8 = 2;
/// The fixed cell charge of one grid query in the public charge: the 3 x 3 cells of a
/// query whose reach equals the cell size.
const CELL_CHARGE: usize = 9;
const SLOTS: usize = turts_mod_api::SLOT_COUNT as usize;

pub(crate) fn owner_bucket(owner: u8) -> usize {
    usize::from(owner).min(SLOTS - 1)
}

/// Recovery accounting of one tick (summed over solver passes).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RecoveryCounters {
    /// The public charge of each owner.
    pub(crate) owner_work: [usize; SLOTS],
    /// The actual operations of each owner: candidates, cells and bodies visited (hidden
    /// bodies included), visibility checks and claim entries read.
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
        self.phase_bytes = self.phase_bytes.max(other.phase_bytes);
        self.retained_bytes = other.retained_bytes;
    }

    pub(crate) fn peak_owner_work(&self) -> usize {
        self.owner_work.iter().copied().max().unwrap_or(0)
    }

    pub(crate) fn peak_owner_actual(&self) -> usize {
        self.owner_actual.iter().copied().max().unwrap_or(0)
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
    pub(super) position: Vec2,
    pub(super) radius: f32,
}

fn service_order(left: &ScheduleRow, right: &ScheduleRow) -> std::cmp::Ordering {
    right
        .age
        .cmp(&left.age)
        .then_with(|| left.position.x.total_cmp(&right.position.x))
        .then_with(|| left.position.y.total_cmp(&right.position.y))
        .then_with(|| left.radius.total_cmp(&right.radius))
}

/// Gives the allowances of each owner. Fresh jobs get `FRESH_SLICE` each, in service
/// order, from half the budget; a first batch that does not fit shares that half equally.
/// The oldest batch of jobs with a scan shares the rest equally. With no such job, the
/// served fresh jobs share the rest too.
pub(super) fn plan(
    mut rows: Vec<ScheduleRow>,
    counters: &mut RecoveryCounters,
) -> RecoverySchedule {
    rows.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| left.has_scan.cmp(&right.has_scan))
            .then_with(|| service_order(left, right))
    });
    let mut allowances = Vec::with_capacity(rows.len());
    counters.phase_bytes = counters.phase_bytes.max(
        rows.capacity() * std::mem::size_of::<ScheduleRow>()
            + allowances.capacity() * std::mem::size_of::<(usize, usize)>(),
    );
    for owner_rows in rows.chunk_by(|left, right| left.owner == right.owner) {
        let split = owner_rows.partition_point(|row| !row.has_scan);
        let (fresh, continuing) = owner_rows.split_at(split);
        let mut served = Vec::new();
        let mut fresh_spent = 0usize;
        for batch in fresh.chunk_by(|left, right| service_order(left, right).is_eq()) {
            let cost = FRESH_SLICE * batch.len();
            if fresh_spent + cost <= OWNER_WORK / 2 {
                fresh_spent += cost;
                served.extend(batch.iter().map(|row| (row.row, FRESH_SLICE)));
            } else {
                if served.is_empty() {
                    let share = (OWNER_WORK / 2 / batch.len()).max(1);
                    fresh_spent += share * batch.len();
                    served.extend(batch.iter().map(|row| (row.row, share)));
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
        for (row, share) in served {
            allowances.push((row, share + fresh_extra));
        }
        for row in oldest {
            allowances.push((row.row, (rest / oldest.len()).max(1)));
        }
    }
    allowances.sort_unstable();
    RecoverySchedule { allowances }
}

pub(super) enum SliceOutcome {
    /// The search ended: the nearest free point, or none.
    Done(Option<Vec2>),
    /// The slice is spent; keep this scan.
    Paused(RecoveryScan),
}

/// The claim keys of all pools: `(pool id, x bits, y bits)`.
pub(super) type AssignedSlots = HashSet<(u64, u32, u32)>;

/// One escape search over the current movement state.
pub(super) struct EscapeScan<'a> {
    pub(super) bounds: Vec2,
    pub(super) units: &'a [MovementFrameUnit],
    pub(super) points: &'a [crate::grid::GridPoint],
    pub(super) grid: &'a crate::grid::UniformGrid,
    pub(super) max_radius: f32,
    pub(super) origin: Vec2,
    pub(super) radius: f32,
    pub(super) self_index: usize,
    /// The pool and its assigned slot keys (group escape only).
    pub(super) slots: Option<(u64, &'a AssignedSlots)>,
    pub(super) known: &'a dyn Fn(u32) -> bool,
    /// A point that already wins: stop when the nearest unseen ring is farther.
    pub(super) bound: Option<Vec2>,
    /// Test hook. Production is always `true`.
    pub(super) verify: bool,
}

struct Spend<'c> {
    public: usize,
    actual: usize,
    counters: &'c mut RecoveryCounters,
}

const DIGEST_SEEDS: [u64; 2] = [0x243f_6a88_85a3_08d3, 0x1319_8a2e_0370_7344];
const BODY_TAG: u64 = 1 << 63;

fn mix(a: u64, b: u64, seed: u64) -> u64 {
    let mut z = a
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .rotate_left(31)
        .wrapping_add(b ^ seed);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn add_fact(digest: &mut [u64; 3], a: u64, b: u64) {
    digest[0] = digest[0].wrapping_add(1);
    digest[1] = digest[1].wrapping_add(mix(a, b, DIGEST_SEEDS[0]));
    digest[2] = digest[2].wrapping_add(mix(a, b, DIGEST_SEEDS[1]));
}

fn bits(point: Vec2) -> u64 {
    (u64::from(point.x.to_bits()) << 32) | u64::from(point.y.to_bits())
}

impl EscapeScan<'_> {
    fn spacing(&self) -> f32 {
        2.0 * self.radius + 0.1
    }

    fn same_search(&self, scan: &RecoveryScan) -> bool {
        scan.origin.x.to_bits() == self.origin.x.to_bits()
            && scan.origin.y.to_bits() == self.origin.y.to_bits()
            && scan.radius.to_bits() == self.radius.to_bits()
    }

    fn fresh(&self, invalidations: u8, relaxed: bool) -> RecoveryScan {
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
        }
    }

    /// Today's free predicate for one candidate. It visits every body in reach, so the
    /// result and the charges do not depend on the grid visit order.
    fn free(&self, candidate: Vec2, spend: &mut Spend) -> bool {
        let radius = self.radius;
        if candidate.x < radius
            || candidate.y < radius
            || candidate.x > self.bounds.x - radius
            || candidate.y > self.bounds.y - radius
        {
            return false;
        }
        if let Some((pool, assigned)) = self.slots {
            spend.public += 1;
            spend.actual += 1;
            spend.counters.slot_checks += 1;
            if assigned.contains(&(pool, candidate.x.to_bits(), candidate.y.to_bits())) {
                return false;
            }
        }
        let mut free = true;
        let mut checks = 0usize;
        let mut touching = 0usize;
        let (cells, tested) =
            self.grid
                .visit_within(self.points, candidate, radius + self.max_radius, |index| {
                    if index == self.self_index {
                        return;
                    }
                    checks += 1;
                    let other = &self.units[index];
                    if (self.known)(other.unit_id) {
                        let distance = candidate.distance(other.position);
                        if distance <= radius + other.radius + 1.0e-3 {
                            touching += 1;
                        }
                        if distance < radius + other.radius - 1.0e-5 {
                            free = false;
                        }
                    }
                });
        spend.public += CELL_CHARGE + touching;
        spend.actual += cells + tested + checks;
        spend.counters.cells += cells as u64;
        spend.counters.bodies += tested as u64;
        spend.counters.visibility_checks += checks as u64;
        free
    }

    /// The order-free digest of the permitted facts that decide the candidates within
    /// `examined` of the origin: each permitted body whose disc can reach one of them, and
    /// the claim keys of the own pool within `examined`. Membership uses each body's own
    /// radius, never the frame maximum, so hidden bodies cannot change it.
    fn digest(&self, examined: f32, spend: &mut Spend) -> [u64; 3] {
        let mut digest = [0u64; 3];
        let mut checks = 0usize;
        let mut members = 0usize;
        let reach = examined + self.radius + self.max_radius + 1.0e-3;
        let (cells, tested) = self
            .grid
            .visit_within(self.points, self.origin, reach, |index| {
                if index == self.self_index {
                    return;
                }
                checks += 1;
                let other = &self.units[index];
                if (self.known)(other.unit_id)
                    && other.position.distance(self.origin)
                        <= examined + self.radius + other.radius + 1.0e-3
                {
                    members += 1;
                    add_fact(
                        &mut digest,
                        BODY_TAG | u64::from(other.radius.to_bits()),
                        bits(other.position),
                    );
                }
            });
        spend.public += CELL_CHARGE + members;
        spend.actual += cells + tested + checks;
        spend.counters.cells += cells as u64;
        spend.counters.bodies += tested as u64;
        spend.counters.visibility_checks += checks as u64;
        if let Some((pool, assigned)) = self.slots {
            spend.actual += assigned.len();
            spend.counters.slot_checks += assigned.len() as u64;
            for &(slot_pool, x, y) in assigned {
                let point = Vec2::new(f32::from_bits(x), f32::from_bits(y));
                if slot_pool == pool {
                    // Claims of the own pool are public to its owner.
                    spend.public += 1;
                    if point.distance(self.origin) <= examined + 1.0e-3 {
                        add_fact(&mut digest, slot_pool & !BODY_TAG, bits(point));
                    }
                }
            }
        }
        spend.counters.digests += 1;
        digest
    }

    /// Runs one slice of at most `allowance` public charge (plus one candidate and one
    /// digest of overshoot).
    pub(super) fn run(
        &self,
        previous: Option<RecoveryScan>,
        allowance: usize,
        owner: u8,
        counters: &mut RecoveryCounters,
    ) -> SliceOutcome {
        let mut spend = Spend {
            public: 0,
            actual: 0,
            counters,
        };
        spend.counters.slices += 1;
        let previous = previous.filter(|scan| self.same_search(scan));
        let mut resumed = previous.is_some();
        let mut state = match previous {
            Some(scan) if scan.relaxed || !self.verify => scan,
            Some(mut scan) => {
                let digest = self.digest(scan.examined, &mut spend);
                let costly = spend.public > allowance / 2;
                if costly {
                    spend.counters.relaxations += 1;
                }
                if digest == scan.digest {
                    scan.relaxed = costly;
                    scan
                } else {
                    spend.counters.invalidations += 1;
                    resumed = false;
                    self.fresh(scan.invalidations.saturating_add(1), costly)
                }
            }
            None => self.fresh(0, false),
        };
        let outcome = match self.scan(&mut state, allowance, &mut spend) {
            Some(best) if resumed && (state.relaxed || !self.verify) => {
                // A relaxed scan read a changing region: commit only a point free now.
                if best.is_none_or(|point| self.free(point, &mut spend)) {
                    if best.is_some() {
                        spend.counters.relaxed_commits += 1;
                    }
                    SliceOutcome::Done(best)
                } else {
                    SliceOutcome::Paused(self.fresh(state.invalidations, true))
                }
            }
            Some(best) => SliceOutcome::Done(best),
            None => {
                if !state.relaxed && self.verify {
                    state.digest = self.digest(state.examined, &mut spend);
                }
                SliceOutcome::Paused(state)
            }
        };
        match outcome {
            SliceOutcome::Done(_) => spend.counters.completions += 1,
            SliceOutcome::Paused(_) => spend.counters.pauses += 1,
        }
        let (public, actual) = (spend.public, spend.actual);
        spend.counters.charge(owner, public, actual);
        outcome
    }

    /// Continues today's ring order. `Some(result)` when the search ends, `None` when the
    /// allowance is spent first.
    fn scan(
        &self,
        state: &mut RecoveryScan,
        allowance: usize,
        spend: &mut Spend,
    ) -> Option<Option<Vec2>> {
        let spacing = self.spacing();
        let origin = self.origin;
        let ring_limit = finite_hex_ring_limit(origin, self.radius, self.bounds, spacing);
        while state.ring <= ring_limit {
            let ring = state.ring;
            while state.side < 6 {
                while state.step < ring {
                    if spend.public >= allowance {
                        return None;
                    }
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
                    spend.public += 1;
                    spend.actual += 1;
                    spend.counters.candidates += 1;
                    state.examined = state.examined.max(candidate.distance(origin));
                    if self.free(candidate, spend)
                        && state.best.is_none_or(|previous| {
                            nearest_angle_order(candidate, previous, origin).is_lt()
                        })
                    {
                        state.best = Some(candidate);
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
                || self
                    .bound
                    .is_some_and(|bound| unseen > bound.distance(origin) + 1.0e-5)
            {
                return Some(state.best);
            }
            state.side = 0;
            state.ring += 1;
        }
        Some(state.best)
    }
}

/// The recovery state of one frame row in one tick.
pub(super) struct RowRecovery<'c> {
    pub(super) row: usize,
    pub(super) owner: u8,
    pub(super) allowance: usize,
    pub(super) previous: Option<RecoverySearch>,
    pub(super) next_jobs: &'c mut BTreeMap<usize, RecoverySearch>,
    pub(super) counters: &'c mut RecoveryCounters,
}

impl RowRecovery<'_> {
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

    /// Runs one slice of `scan` for `kind`. `None` when the search found no point or did
    /// not end in this slice; an unfinished job stays for the next tick.
    pub(super) fn search(&mut self, kind: RecoveryKind, scan: &EscapeScan) -> Option<Vec2> {
        if !self.may_search(kind) {
            return None;
        }
        let previous = self.same_kind(kind).and_then(|job| job.scan);
        let allowance = std::mem::take(&mut self.allowance);
        match scan.run(previous, allowance, self.owner, self.counters) {
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

    /// Charges a search that this row runs outside the ring scan (pool slots). It comes
    /// out of the slice allowance.
    pub(super) fn charge_pool(&mut self, public: usize, actual: usize) {
        self.counters.pool_work += public as u64;
        self.counters.charge(self.owner, public, actual);
        self.allowance = self.allowance.saturating_sub(public).max(1);
    }
}

/// Bytes retained by recovery jobs after a tick.
pub(super) fn retained_bytes(jobs: usize) -> usize {
    jobs * (std::mem::size_of::<RecoverySearch>() + std::mem::size_of::<u32>())
}
