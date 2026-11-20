//! Test oracles of the metered known-body queries (design `2026-11-19-recovery-actual-work`
//! section 7): the unmetered physical queries of the base `fa3df722`, filtered by `known`.
//! The frame calls them in test builds only, at each check and each spare-slot search, so
//! every movement test compares the metered result with the old one.

use bevy_math::Vec2;
use std::cell::Cell;

use super::frame::MovementFrameUnit;
use super::pool_index::MovementPoolIndex;
use super::recovery::Check;
use super::recovery_view::ClaimSet;

thread_local! {
    /// Comparisons made on this thread: point checks, reached queries, spare searches.
    static COMPARED: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

/// The comparisons made on this thread since the last call: `[checks, reached, spare]`.
pub(super) fn take_compared() -> [usize; 3] {
    COMPARED.with(|count| count.replace([0; 3]))
}

fn count(kind: usize) {
    COMPARED.with(|compared| {
        let mut now = compared.get();
        now[kind] += 1;
        compared.set(now);
    });
}

thread_local! {
    /// Schedule reads compared with the full scan on this thread.
    static SCHEDULE_COMPARED: Cell<usize> = const { Cell::new(0) };
}

/// The schedule reads compared on this thread since the last call.
pub(super) fn take_schedule_compared() -> usize {
    SCHEDULE_COMPARED.with(|count| count.replace(0))
}

/// The largest frame that `check_schedule_read` scans in full (it costs one test for each
/// row of the frame).
const SCHEDULE_ORACLE_ROWS: usize = 4_096;

/// Compares the schedule reads of row `row` (design `2026-11-20-bounded-contact`) with a
/// test of every row of the frame: the overlap flag is true when a row overlaps the actor,
/// and the contacts are the rows in contact range that `member` rejects (the old single
/// read of reach `radius + max radius + CONTACT_GAP`, with no cell and no reach limit).
pub(super) fn check_schedule_read(
    row: usize,
    units: &[MovementFrameUnit],
    reads: &super::recovery::ScheduleReads,
    overlapping: bool,
    member: impl Fn(usize) -> bool,
) {
    if units.len() > SCHEDULE_ORACLE_ROWS {
        return;
    }
    let unit = &units[row];
    let mut expected_overlap = false;
    let mut expected: Vec<u32> = Vec::new();
    for (other, body) in units.iter().enumerate() {
        if other == row {
            continue;
        }
        let (overlap, contact) = super::recovery::touch(unit.position, unit.radius, body);
        expected_overlap |= overlap;
        if contact && !member(other) {
            expected.push(other as u32);
        }
    }
    let mut found = reads.last_contacts().to_vec();
    found.sort_unstable();
    assert_eq!(
        overlapping, expected_overlap,
        "the overlap read of row {row}"
    );
    assert_eq!(found, expected, "the contact read of row {row}");
    SCHEDULE_COMPARED.with(|count| count.set(count.get() + 1));
}

/// The physical state of one row turn and the `known` rule of its actor.
pub(super) struct Oracle<'a> {
    pub(super) units: &'a [MovementFrameUnit],
    pub(super) points: &'a [crate::grid::GridPoint],
    pub(super) grid: &'a crate::grid::UniformGrid,
    pub(super) max_radius: f32,
    pub(super) index: usize,
    pub(super) known: &'a dyn Fn(u32) -> bool,
}

impl Oracle<'_> {
    fn radius(&self) -> f32 {
        self.units[self.index].radius
    }

    fn near(&self, point: Vec2) -> Vec<usize> {
        self.grid
            .within_unsorted(self.points, point, self.radius() + self.max_radius)
    }

    /// The base predicates `target_occupied` and `body_occupied` (tolerance 1e-5) and
    /// `!is_free` (tolerance 0): a known body of another row overlaps a disc at `point`.
    pub(super) fn occupied(&self, point: Vec2, tolerance: f32) -> bool {
        self.near(point).into_iter().any(|other| {
            other != self.index
                && (self.known)(self.units[other].unit_id)
                && point.distance(self.units[other].position)
                    < self.radius() + self.units[other].radius - tolerance
        })
    }

    /// The base predicate `target_is_blocked_by_stationary`, which the retained check of
    /// a group row replaces: a known body at rest overlaps the target.
    pub(super) fn blocked_by_stationary(&self, target: Vec2) -> bool {
        self.near(target).into_iter().any(|other| {
            if other == self.index || !(self.known)(self.units[other].unit_id) {
                return false;
            }
            let other = &self.units[other];
            let stationary = other.target.is_none()
                || other
                    .progress
                    .as_ref()
                    .is_some_and(|state| state.activity == super::MovementActivity::Resting);
            stationary && target.distance(other.position) < self.radius() + other.radius - 1.0e-5
        })
    }

    /// A metered check that ended gives the base result. `claimed` is the base claim test
    /// of the point.
    pub(super) fn compare_check(&self, result: Check, point: Vec2, tolerance: f32, claimed: bool) {
        let clear = !claimed && !self.occupied(point, tolerance);
        match result {
            Check::Clear => {
                assert!(clear, "a metered check reported an occupied point as clear");
                assert!(
                    !self.blocked_by_stationary(point),
                    "a clear point has no resting body on it"
                );
            }
            Check::Blocked => assert!(!clear, "a metered check blocked a clear point"),
            Check::Pending | Check::Unpaid => return,
        }
        count(0);
    }

    /// The reached test of a blocked row: never clear when the base is not, and equal to
    /// the base when the allowance paid the query.
    pub(super) fn compare_reached(&self, free: bool, point: Vec2, tolerance: f32, left: usize) {
        let clear = !self.occupied(point, tolerance);
        assert!(!free || clear, "a reached test took an occupied point");
        assert!(
            free == clear || left == 0,
            "a paid reached test differs from the base"
        );
        count(1);
    }

    /// A spare-slot search that ran in one slice gives the slot of the base
    /// `nearest_free` with the base predicate. `exchange` holds the tick-start rest flags
    /// and contact witnesses of the claim exchange mode.
    pub(super) fn compare_spare(
        &self,
        result: Option<Vec2>,
        pool: &MovementPoolIndex,
        origin: Vec2,
        old_slot_key: (u64, u32, u32),
        claims: &ClaimSet,
        exchange: Option<(&[bool], &[Vec<u8>])>,
    ) {
        let unit = &self.units[self.index];
        let expected = pool.nearest_free(
            unit.unit_id,
            origin,
            old_slot_key.0,
            None,
            claims,
            unit.radius,
            |slot| {
                let key = (old_slot_key.0, slot.x.to_bits(), slot.y.to_bits());
                key != old_slot_key
                    && self.near(slot).into_iter().all(|other| {
                        other == self.index
                            || !(self.known)(self.units[other].unit_id)
                            || exchange.is_some_and(|(stationary, witness)| {
                                self.units[other].owner_slot != unit.owner_slot
                                    && !(stationary[other]
                                        && witness[other].binary_search(&unit.owner_slot).is_ok())
                            })
                            || slot.distance(self.units[other].position)
                                >= unit.radius + self.units[other].radius
                    })
            },
            &Cell::new(0),
        );
        assert_eq!(
            result.map(|point| point.to_array().map(f32::to_bits)),
            expected.map(|point| point.to_array().map(f32::to_bits)),
            "the metered spare-slot search differs from nearest_free"
        );
        count(2);
    }
}
