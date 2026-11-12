//! The obstacle fallback: a pool of one for a trip with no pool (pre-MP1 slice, task A2b1).
//!
//! A trip with no pool (a gather trip, a queued start, an attack approach, a follow, a resume, a
//! legacy single move, a synthetic unit) gets a pool of one when its goal is occupied, or when it
//! is pushed or projected off its line and does not arrive (design section 1, the technical
//! director ruling; design 9.11). The pool comes from the spread code of the move envelopes
//! (`spread_destinations_with_context`), with the unit as the only member. It replaces the plain
//! progress of the old movement model.
//!
//! The step function is pure, and the pool ids live in `PendingMovementOrders::next_pool_id`
//! (`orders/pending.rs`). So the step returns one `FallbackRequest` for each fallback, with the
//! pool id 0 (no pool has the id 0: the counter gives 1 first), and the movement system takes the
//! ids from that counter after the step, in unit-id order (`assign_fallback_pools`). The legacy
//! envelope failure (`orders/pending.rs`) takes its ids from the same counter. One id space, one
//! order.

use bevy_math::Vec2;

use super::frame::{MovementFrameOutput, MovementFrameUnit};
use super::spread::{SpreadContext, spread_destinations_with_context};
use super::{MoveFallback, MoveProgress, MovementActivity, MovementPool, SpreadMember};

/// The pool of one that the step gives to one unit. `pool.pool_id` is 0 until
/// `assign_fallback_pools` gives it an id.
#[derive(Debug, Clone, PartialEq)]
pub struct FallbackRequest {
    pub unit_id: u32,
    /// The spread result for the unit alone at the point of its trip (`pool.point`).
    pub pool: MovementPool,
}

/// The pool of one of `unit` at `point`, and the progress of the unit in it. The slot is the
/// spread assignment of the unit (the free slot nearest to the point, MC design 6.3), or the
/// point when the spread finds no free slot; then the activity is `Blocked`.
pub(super) fn fallback_pool(
    unit: &MovementFrameUnit,
    point: Vec2,
    context: &SpreadContext,
) -> (MovementPool, MoveProgress) {
    let member = [SpreadMember {
        id: unit.unit_id,
        position: unit.position,
        radius: unit.radius,
    }];
    let excluded_units = std::collections::HashSet::from([unit.unit_id]);
    let (pool, assignments) =
        spread_destinations_with_context(&member, point, context, &excluded_units);
    let slot = assignments
        .first()
        .map_or(pool.point, |(_, destination)| *destination);
    let progress = MoveProgress {
        pool_id: 0,
        slot,
        activity: if pool.primary_count == 0 {
            MovementActivity::Blocked
        } else {
            MovementActivity::Moving
        },
        pass_mode: false,
        stalling_on_group: false,
        stalled_ticks: 0,
        best_distance: unit.position.distance(slot),
        claim_count: 0,
        follow_origin: None,
        gather_goal: None,
        fallback: Some(MoveFallback {
            failure_recorded: false,
        }),
    };
    (pool, progress)
}

/// Give each fallback pool of one step its id from `next_pool_id`, in unit-id order, and write
/// the id into the progress of its unit. Returns the pools with their ids. The movement system
/// passes `PendingMovementOrders::next_pool_id`; a test that chains frames passes its own
/// counter.
pub fn assign_fallback_pools(
    outputs: &mut [MovementFrameOutput],
    mut requests: Vec<FallbackRequest>,
    next_pool_id: &mut u64,
) -> Vec<MovementPool> {
    requests.sort_unstable_by_key(|request| request.unit_id);
    // The outputs of a step are in unit-id order, as the requests are: one walk over both
    // (the A2b1 review, finding L1).
    debug_assert!(
        outputs
            .windows(2)
            .all(|pair| pair[0].unit_id < pair[1].unit_id),
        "the outputs of a step are in unit-id order"
    );
    let mut cursor = 0;
    requests
        .into_iter()
        .map(|mut request| {
            *next_pool_id = next_pool_id.saturating_add(1);
            request.pool.pool_id = *next_pool_id;
            while outputs
                .get(cursor)
                .is_some_and(|output| output.unit_id < request.unit_id)
            {
                cursor += 1;
            }
            let progress = outputs
                .get_mut(cursor)
                .filter(|output| output.unit_id == request.unit_id)
                .and_then(|output| output.progress.as_mut())
                .expect("a fallback request names a unit of the frame that holds its progress");
            debug_assert_eq!(progress.pool_id, 0, "a fallback pool has no id yet");
            progress.pool_id = *next_pool_id;
            request.pool
        })
        .collect()
}
