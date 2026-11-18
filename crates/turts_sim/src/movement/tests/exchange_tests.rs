//! Frame-level contracts of the claim exchange and the route search (`exchange`, `route`).
//!
//! The scene: thirty bodies in a block walk to a point. The pool is a tight hex lattice, so the
//! group must exchange claims and route round its own resting members to fill it.

use super::{
    MoveProgress, MovementActivity, MovementFrame, MovementFrameUnit, SpreadMember,
    spread_destinations, step_frame_at_tick,
};
use bevy_math::Vec2;
use std::collections::BTreeMap;

const COUNT: usize = 30;
const RADIUS: f32 = 0.5;
const TICKS: u32 = 500;

/// One recorded run: the position of each body, in scene order, after each tick.
struct Run {
    positions: Vec<Vec<Vec2>>,
    /// The number of slot changes of a body that still moved.
    slot_changes: usize,
    blocked: usize,
    moving_at_end: usize,
}

/// `handle(scene index)` gives the opaque unit id. `reverse` reverses the input rows.
/// `foreign` adds stationary bodies of another owner.
fn run(handle: impl Fn(usize) -> u32, reverse: bool, foreign: &[Vec2]) -> Run {
    let bounds = Vec2::new(64.0, 48.0);
    let point = Vec2::new(30.0, 30.0);
    let starts: Vec<Vec2> = (0..COUNT)
        .map(|index| {
            Vec2::new(
                26.0 + 1.1 * (index % 6) as f32,
                18.0 + 1.1 * (index / 6) as f32,
            )
        })
        .collect();
    let members: Vec<SpreadMember> = starts
        .iter()
        .enumerate()
        .map(|(index, &position)| SpreadMember {
            id: index as u32,
            position,
            radius: RADIUS,
        })
        .collect();
    let (mut pool, assignments) = spread_destinations(&members, point, &[], &[], bounds);
    pool.pool_id = 7;
    assert_eq!(assignments.len(), COUNT, "the pool holds the whole group");
    let slot_of: BTreeMap<u32, Vec2> = assignments.into_iter().collect();
    let mut units: Vec<MovementFrameUnit> = (0..COUNT)
        .map(|index| {
            let slot = slot_of[&(index as u32)];
            MovementFrameUnit {
                unit_id: handle(index),
                owner_slot: 0,
                unit_type: 1,
                position: starts[index],
                radius: RADIUS,
                speed: Some(3.0),
                target: Some(slot),
                progress: Some(MoveProgress {
                    pool_id: 7,
                    slot,
                    activity: MovementActivity::Moving,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: starts[index].distance(slot),
                    claim_count: 0,
                    follow_origin: None,
                }),
                plain_progress: None,
                physical_escape: None,
            }
        })
        .collect();
    units.extend(
        foreign
            .iter()
            .enumerate()
            .map(|(index, &position)| MovementFrameUnit {
                unit_id: 100_000 + index as u32,
                owner_slot: 1,
                unit_type: 2,
                position,
                radius: RADIUS,
                speed: Some(3.0),
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            }),
    );
    if reverse {
        units.reverse();
    }
    let mut frame = MovementFrame {
        bounds,
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(7, pool)]),
    };
    let ids: Vec<u32> = (0..COUNT).map(&handle).collect();
    let mut result = Run {
        positions: Vec::new(),
        slot_changes: 0,
        blocked: 0,
        moving_at_end: 0,
    };
    let mut resting: BTreeMap<u32, Vec2> = BTreeMap::new();
    for tick in 1..=TICKS {
        let outputs = step_frame_at_tick(&frame, 0.05, tick);
        for output in outputs {
            let unit = frame
                .units
                .iter_mut()
                .find(|unit| unit.unit_id == output.unit_id)
                .unwrap();
            if output.failed_blocked {
                result.blocked += 1;
            }
            if let (Some(before), Some(after)) = (&unit.progress, &output.progress)
                && before.slot != after.slot
            {
                assert!(
                    unit.target.is_some(),
                    "tick {tick}: a resting body got another slot"
                );
                result.slot_changes += 1;
            }
            // Physical first arrival: a body that rests in its pool never moves again.
            if let Some(point) = resting.get(&output.unit_id) {
                assert_eq!(
                    output.position, *point,
                    "tick {tick}: a resting body lost its point"
                );
            } else if output.target.is_none() && unit.owner_slot == 0 && !output.failed_blocked {
                resting.insert(output.unit_id, output.position);
            }
            unit.position = output.position;
            unit.target = output.target;
            unit.progress = output.progress;
            unit.plain_progress = output.plain_progress;
            unit.physical_escape = output.physical_escape;
        }
        let at = |id: u32| {
            frame
                .units
                .iter()
                .find(|unit| unit.unit_id == id)
                .unwrap()
                .position
        };
        let row: Vec<Vec2> = ids.iter().map(|id| at(*id)).collect();
        for (index, a) in row.iter().enumerate() {
            for b in &row[index + 1..] {
                assert!(
                    a.distance(*b) >= 2.0 * RADIUS - 1.0e-4,
                    "tick {tick}: two bodies overlap"
                );
            }
        }
        result.positions.push(row);
    }
    result.moving_at_end = frame
        .units
        .iter()
        .filter(|unit| unit.owner_slot == 0 && unit.target.is_some())
        .count();
    result
}

#[test]
fn group_fills_its_pool_by_exchange_with_no_blocked_body() {
    let baseline = run(|index| index as u32 + 1, false, &[]);
    assert!(
        baseline.slot_changes > 0,
        "the scene must exercise the exchange"
    );
    assert_eq!(baseline.blocked, 0, "no body fails with Blocked");
    assert_eq!(baseline.moving_at_end, 0, "every body rests");
}

#[test]
fn handles_and_enumeration_do_not_change_group_motion() {
    let baseline = run(|index| index as u32 + 1, false, &[]);
    assert!(baseline.slot_changes > 0);
    // Reversed handles, scrambled handles, and reversed input rows.
    let reversed = run(|index| (COUNT - index) as u32, false, &[]);
    let scrambled = run(|index| ((index * 7 + 3) % COUNT) as u32 + 500, true, &[]);
    let rows = run(|index| index as u32 + 1, true, &[]);
    for (name, other) in [
        ("reversed handles", &reversed),
        ("scrambled handles and reversed rows", &scrambled),
        ("reversed rows", &rows),
    ] {
        assert!(
            baseline.positions == other.positions,
            "{name} changed the physical motion of the group"
        );
    }
}

#[test]
fn bodies_of_another_owner_out_of_contact_do_not_change_group_motion() {
    let baseline = run(|index| index as u32 + 1, false, &[]);
    // A wall of foreign bodies beside the route and round the pool. It lies inside every
    // route search window and inside the exchange reach of the outer slots, but no body of
    // the group touches it. If a route or a claim read these bodies, the motion would change.
    let reach = baseline
        .positions
        .iter()
        .flatten()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), point| {
            (low.min(point.x), high.max(point.x))
        });
    let mut foreign = Vec::new();
    for step in 0..40 {
        let y = 14.0 + 1.05 * step as f32;
        if y > 46.0 {
            break;
        }
        foreign.push(Vec2::new(reach.0 - 2.0 * RADIUS - 0.35, y));
        foreign.push(Vec2::new(reach.1 + 2.0 * RADIUS + 0.35, y));
    }
    let walled = run(|index| index as u32 + 1, false, &foreign);
    assert!(
        baseline.positions == walled.positions,
        "bodies of another owner that touch no mover changed the group motion"
    );
}
