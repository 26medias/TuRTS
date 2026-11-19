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
        recovery: Default::default(),
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

/// A closed wall of resting bodies of another owner between the group and its pool. The gap
/// between two wall bodies is 0.05: no body can pass.
fn foreign_wall() -> Vec<Vec2> {
    (0..12)
        .map(|step| Vec2::new(23.2 + 1.05 * step as f32, 25.0))
        .collect()
}

/// The group knows a wall body only while one of its bodies is in contact range of it, and it
/// keeps no memory of the wall. Two contracts:
/// - Most bodies go round the wall and rest in the pool. Without the contact rule each body
///   presses on the wall and fails.
/// - The run ends. A body that finds unknown wall bodies again and again spends its restart
///   budget and then fails with `Blocked`; it must not move for ever.
#[test]
fn group_goes_round_a_resting_wall_of_another_owner_that_it_touches() {
    let walled = run(|index| index as u32 + 1, false, &foreign_wall());
    assert_eq!(
        walled.moving_at_end, 0,
        "a body still moves at the end ({} Blocked)",
        walled.blocked
    );
    let beyond = walled
        .positions
        .last()
        .unwrap()
        .iter()
        .filter(|point| point.y > 26.0)
        .count();
    assert!(
        beyond >= 20,
        "only {beyond} of {COUNT} bodies went round the wall ({} Blocked)",
        walled.blocked
    );
    assert_eq!(
        beyond + walled.blocked,
        COUNT,
        "each body rests in the pool or fails with Blocked"
    );
}

/// Hidden-world invariance: bodies of another owner that no body of the group comes into
/// contact range of must not change one position, even while the group plans round a wall
/// that it touches.
#[test]
fn untouched_bodies_of_another_owner_do_not_change_a_route_round_a_touched_wall() {
    let baseline = run(|index| index as u32 + 1, false, &foreign_wall());
    let mut foreign = foreign_wall();
    // Inside the search windows of the routes round both wall ends, but never touched.
    for step in 0..4 {
        foreign.push(Vec2::new(14.0, 20.0 + 1.05 * step as f32));
        foreign.push(Vec2::new(44.0, 20.0 + 1.05 * step as f32));
        foreign.push(Vec2::new(27.0 + 1.05 * step as f32, 38.5));
    }
    let hidden = run(|index| index as u32 + 1, false, &foreign);
    let nearest = hidden
        .positions
        .iter()
        .flatten()
        .flat_map(|point| foreign[12..].iter().map(|body| body.distance(*point)))
        .fold(f32::INFINITY, f32::min);
    assert!(
        nearest > 2.0 * RADIUS + 0.2,
        "the scene is wrong: a group body came within {nearest} of an added body"
    );
    assert!(
        baseline.positions == hidden.positions,
        "untouched bodies of another owner changed the group motion"
    );
}

/// A small scene that the old recovery cannot solve. Two own bodies rest side by side on the
/// pool lattice. The slot `far` lies behind the gap between them (the gap is narrower than a
/// body), and the slot `near` lies in front of it. Mover A comes first and claims `far`; the
/// second body claims `near`.
fn wall_scene(second_rests_on_near: bool) -> (MovementFrame, Vec2, Vec2) {
    let spacing = 1.1_f32;
    let rise = spacing * 3.0_f32.sqrt() * 0.5;
    let far = Vec2::new(10.0, 10.0);
    let near = Vec2::new(10.0, 10.0 - 2.0 * rise);
    let wall = [
        Vec2::new(10.0 - 0.5 * spacing, 10.0 - rise),
        Vec2::new(10.0 + 0.5 * spacing, 10.0 - rise),
    ];
    let member = |unit_id: u32, position: Vec2, slot: Vec2, resting: bool| MovementFrameUnit {
        unit_id,
        owner_slot: 0,
        unit_type: 1,
        position,
        radius: RADIUS,
        speed: Some(3.0),
        target: (!resting).then_some(slot),
        progress: Some(MoveProgress {
            pool_id: 7,
            slot,
            activity: if resting {
                MovementActivity::Resting
            } else {
                MovementActivity::Moving
            },
            pass_mode: false,
            stalling_on_group: false,
            stalled_ticks: 0,
            best_distance: position.distance(slot),
            claim_count: 0,
            follow_origin: None,
        }),
        plain_progress: None,
        physical_escape: None,
    };
    let second_start = if second_rests_on_near {
        near
    } else {
        Vec2::new(10.0, 3.5)
    };
    let frame = MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::new(32.0, 32.0),
        tick_hz: 20,
        units: vec![
            member(1, Vec2::new(10.0, 5.0), far, false),
            member(2, second_start, near, second_rests_on_near),
            member(3, wall[0], wall[0], true),
            member(4, wall[1], wall[1], true),
        ],
        pools: BTreeMap::from([(
            7,
            super::MovementPool {
                pool_id: 7,
                point: far,
                spacing,
                primary_count: 4,
                slots: vec![far, near, wall[0], wall[1]],
            },
        )]),
    };
    (frame, far, near)
}

/// Step the scene. Returns the last position of each unit id and the number of Blocked reports.
fn settle(mut frame: MovementFrame, fixed: &[u32]) -> (BTreeMap<u32, Vec2>, usize) {
    let start: BTreeMap<u32, Vec2> = frame
        .units
        .iter()
        .map(|unit| (unit.unit_id, unit.position))
        .collect();
    let mut blocked = 0;
    for tick in 1..=400 {
        for output in step_frame_at_tick(&frame, 0.05, tick) {
            blocked += usize::from(output.failed_blocked);
            if fixed.contains(&output.unit_id) {
                assert_eq!(
                    output.position, start[&output.unit_id],
                    "tick {tick}: resting unit {} moved",
                    output.unit_id
                );
                assert_eq!(
                    output.target, None,
                    "tick {tick}: a resting unit got a target"
                );
            }
            let unit = frame
                .units
                .iter_mut()
                .find(|unit| unit.unit_id == output.unit_id)
                .unwrap();
            unit.position = output.position;
            unit.target = output.target;
            unit.progress = output.progress;
            unit.plain_progress = output.plain_progress;
            unit.physical_escape = output.physical_escape;
        }
    }
    assert!(
        frame.units.iter().all(|unit| unit.target.is_none()),
        "every body rests after 400 ticks"
    );
    (
        frame
            .units
            .iter()
            .map(|unit| (unit.unit_id, unit.position))
            .collect(),
        blocked,
    )
}

/// Rule 1. The first mover cannot reach its slot behind the resting pair, so it takes the slot
/// in front, which it reaches before the second mover. The second mover gets the slot behind
/// and walks round the pair. Without the exchange the first mover ends Blocked.
#[test]
fn mover_cut_off_by_resting_peers_takes_the_slot_it_reaches_first() {
    let (frame, far, near) = wall_scene(false);
    let (rest, blocked) = settle(frame, &[3, 4]);
    assert_eq!(blocked, 0, "no body fails with Blocked");
    assert_eq!(rest[&1], near, "the first mover rests on the slot in front");
    assert_eq!(rest[&2], far, "the second mover rests on the slot behind");
}

/// Physical first arrival. The slot in front already holds a resting body. The mover must not
/// take that slot; it walks round the pair to its own slot.
#[test]
fn resting_body_keeps_its_slot_when_a_cut_off_mover_arrives() {
    let (frame, far, _) = wall_scene(true);
    let (rest, blocked) = settle(frame, &[2, 3, 4]);
    assert_eq!(blocked, 0, "no body fails with Blocked");
    assert_eq!(rest[&1], far, "the mover reaches its own slot by a route");
}

#[test]
fn a_unit_arriving_earlier_in_the_tick_keeps_its_slot() {
    for (first_id, later_id) in [(2, 1), (1, 2)] {
        for reverse in [false, true] {
            let (mut frame, far, near) = wall_scene(false);
            frame.units[0].unit_id = later_id;
            frame.units[0].position = Vec2::new(10.0, near.y - 1.3);
            frame.units[1].unit_id = first_id;
            frame.units[1].position = Vec2::new(10.0, near.y - 0.05);
            if reverse {
                frame.units.reverse();
            }
            let outputs = step_frame_at_tick(&frame, 0.05, 1);
            let first = outputs.iter().find(|row| row.unit_id == first_id).unwrap();
            let later = outputs.iter().find(|row| row.unit_id == later_id).unwrap();
            assert_eq!(first.position, near);
            assert_eq!(first.target, None, "the first arrival must keep its point");
            assert_eq!(later.progress.as_ref().unwrap().slot, far);
        }
    }
}
/// One mover of owner 0 at `start` with the slot (10, 12.5), and one body of owner 1 at
/// (10, 10) on the straight line to the slot. `pool_id` 7 is in the frame.
fn lone_mover_scene(start: Vec2, pool_id: u64, foreign_moves: bool) -> MovementFrame {
    let slot = Vec2::new(10.0, 12.5);
    let body = Vec2::new(10.0, 10.0);
    MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::new(32.0, 32.0),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: start,
                radius: RADIUS,
                speed: Some(3.0),
                target: Some(slot),
                progress: Some(MoveProgress {
                    pool_id,
                    slot,
                    activity: MovementActivity::Moving,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: start.distance(slot),
                    claim_count: 0,
                    follow_origin: None,
                }),
                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 2,
                position: body,
                radius: RADIUS,
                speed: Some(3.0),
                // A body with a target that it cannot leave for: it "moves" but stays.
                target: foreign_moves.then_some(body),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::from([(
            7,
            super::MovementPool {
                pool_id: 7,
                point: slot,
                spacing: 1.1,
                primary_count: 1,
                slots: vec![slot],
            },
        )]),
    }
}

/// Step unit 1 for `ticks`. Returns its last row and the largest `claim_count` that it had.
fn follow_mover(mut frame: MovementFrame, ticks: u32) -> (MovementFrameUnit, u8) {
    let mut most = 0;
    for tick in 1..=ticks {
        for output in step_frame_at_tick(&frame, 0.05, tick) {
            let unit = frame
                .units
                .iter_mut()
                .find(|unit| unit.unit_id == output.unit_id)
                .unwrap();
            unit.position = output.position;
            unit.target = output.target;
            unit.progress = output.progress;
            unit.plain_progress = output.plain_progress;
            unit.physical_escape = output.physical_escape;
        }
        if let Some(progress) = &frame.units[0].progress {
            most = most.max(progress.claim_count);
        }
    }
    (frame.units[0].clone(), most)
}

/// The planner contract for a body of another owner, on one chord through that body.
#[test]
fn a_route_reads_a_body_of_another_owner_only_at_rest_and_in_contact() {
    let frame = lone_mover_scene(Vec2::new(10.0, 7.0), 7, false);
    let points: Vec<_> = frame
        .units
        .iter()
        .map(|unit| crate::grid::GridPoint {
            id: unit.unit_id,
            position: unit.position,
        })
        .collect();
    let grid = crate::grid::UniformGrid::new(&points, 1.0);
    let clear = |stationary: bool, witness: &[u8]| {
        let witness = [Vec::new(), witness.to_vec()];
        super::route::Planner {
            index: 0,
            units: &frame.units,
            points: &points,
            grid: &grid,
            max_radius: RADIUS,
            bounds: frame.bounds,
            stationary_at_start: &[false, stationary],
            contact_witness: &witness,
            start_stall: &[Some(0), None],
            soft: true,
            stall_ticks: 5,
        }
        .chord_clear(Vec2::new(10.0, 7.0), Vec2::new(10.0, 12.5))
    };
    assert!(
        !clear(true, &[0]),
        "a resting body in contact is an obstacle"
    );
    assert!(clear(true, &[]), "a resting body out of contact is unknown");
    assert!(
        clear(true, &[1]),
        "contact of another owner tells this owner nothing"
    );
    assert!(
        clear(false, &[0]),
        "a moving body is never a route obstacle"
    );
}

/// The find of an unknown resting body costs one restart unit, one time. Contact with a body
/// that the owner knows already costs nothing, so a mover that slides along a known body
/// keeps its budget. The mover still goes round the body and rests on its slot.
#[test]
fn only_the_find_of_an_unknown_body_spends_restart_budget() {
    // The mover starts in contact range: the body is known at the first tick.
    let (unit, most) = follow_mover(lone_mover_scene(Vec2::new(10.0, 8.95), 7, false), 200);
    assert!(unit.target.is_none(), "the mover rests on its slot");
    assert_eq!(unit.position, Vec2::new(10.0, 12.5));
    assert_eq!(most, 0, "contact with a known body spent budget");
    // The mover walks into the body: one find.
    let (unit, most) = follow_mover(lone_mover_scene(Vec2::new(10.0, 6.0), 7, false), 200);
    assert!(unit.target.is_none(), "the mover rests on its slot");
    assert_eq!(
        most, 4,
        "one find costs one restart unit (4 in claim_count)"
    );
}

/// A mover that the exchange path does not plan for (its pool is not in the frame) keeps its
/// `claim_count`: the legacy recovery reads that field as a plain spare-claim count.
#[test]
fn a_mover_outside_the_exchange_path_spends_no_restart_budget() {
    let (_, most) = follow_mover(lone_mover_scene(Vec2::new(10.0, 6.0), 99, false), 60);
    assert_eq!(most & !0b11, 0, "an unplanned mover got restart units");
}
