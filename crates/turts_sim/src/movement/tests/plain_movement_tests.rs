use super::{
    MovementActivity, MovementFrame, MovementFrameUnit, MovementInput, PlainMoveProgress, step,
    step_frame,
};
use bevy_math::Vec2;
use std::collections::BTreeMap;

fn stalled_plain_frame(pool_slots: Vec<Vec2>) -> MovementFrame {
    let point = Vec2::new(20.0, 10.0);
    MovementFrame {
        bounds: Vec2::splat(64.0),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(10.0, 10.0),
                radius: 0.5,
                speed: Some(0.05),
                target: Some(point),
                progress: None,
                plain_progress: Some(PlainMoveProgress {
                    original_point: point,
                    slot: point,
                    pool_slots,
                    gather_goal: None,
                    activity: MovementActivity::Moving,
                    stalled_ticks: 19,
                    best_distance: 10.0,
                    claim_count: 0,
                    failure_recorded: false,
                }),

                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 2,
                position: Vec2::new(11.0001, 10.0),
                radius: 0.5,
                speed: None,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::new(),
    }
}

#[test]
fn unobstructed_step_keeps_the_legacy_target_until_within_one_step() {
    let position = Vec2::new(22.25925, 9.274589);
    let target = Vec2::new(22.386324, 9.181545);
    let speed = 3.0;
    let dt = 0.05;
    let frame = MovementFrame {
        bounds: Vec2::splat(64.0),
        tick_hz: 20,
        units: vec![MovementFrameUnit {
            unit_id: 3,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(speed),
            target: Some(target),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }],
        pools: BTreeMap::new(),
    };

    let expected = step(
        &MovementInput {
            position,
            target,
            speed,
        },
        dt,
    )
    .position;
    let output = step_frame(&frame, dt);

    assert_eq!(output[0].position, expected);
    assert_eq!(output[0].target, Some(target));
}

#[test]
fn retained_plain_progress_does_not_add_a_post_step_arrival_snap() {
    let target = Vec2::new(2.0, 1.0);
    let position = Vec2::new(1.83, 1.0);
    let frame = MovementFrame {
        bounds: Vec2::splat(16.0),
        tick_hz: 20,
        units: vec![MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(3.0),
            target: Some(target),
            progress: None,
            plain_progress: Some(PlainMoveProgress {
                original_point: target,
                slot: target,
                pool_slots: vec![],
                gather_goal: None,
                activity: MovementActivity::Moving,
                stalled_ticks: 0,
                best_distance: position.distance(target),
                claim_count: 1,
                failure_recorded: false,
            }),

            physical_escape: None,
        }],
        pools: BTreeMap::new(),
    };

    let output = step_frame(&frame, 0.05).remove(0);
    let expected = step(
        &MovementInput {
            position,
            target,
            speed: 3.0,
        },
        0.05,
    )
    .position;

    assert_eq!(output.position, expected);
    assert_eq!(output.target, Some(target));
    assert_eq!(
        output.plain_progress.unwrap().activity,
        MovementActivity::Moving
    );
}

#[test]
fn plain_move_respects_reserved_spacing_before_using_a_zero_primary_point() {
    let target = Vec2::new(0.5, 1.2);
    let frame = MovementFrame {
        bounds: Vec2::splat(2.0),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(0.5, 0.5),
                radius: 0.5,
                speed: Some(3.0),
                target: Some(target),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 1,
                position: Vec2::new(1.5, 1.5),
                radius: 0.5,
                speed: Some(3.0),
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::new(),
    };

    let output = step_frame(&frame, 0.05);

    assert_eq!(output[0].position, Vec2::new(0.5, 0.5));
    assert_eq!(output[0].target, None);
    let progress = output[0]
        .plain_progress
        .as_ref()
        .expect("reserved spacing activates the plain fallback");
    assert_eq!(progress.activity, MovementActivity::Blocked);
    assert!(progress.pool_slots.is_empty());
}

#[test]
fn blocked_plain_unit_does_not_escape_for_reservation_spacing_alone() {
    let position = Vec2::new(10.0, 10.0);
    let frame = MovementFrame {
        bounds: Vec2::splat(64.0),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position,
                radius: 0.5,
                speed: Some(3.0),
                target: None,
                progress: None,
                plain_progress: Some(PlainMoveProgress {
                    original_point: Vec2::new(20.0, 10.0),
                    slot: position,
                    pool_slots: vec![],
                    gather_goal: None,
                    activity: MovementActivity::Blocked,
                    stalled_ticks: 20,
                    best_distance: 10.0,
                    claim_count: 3,
                    failure_recorded: true,
                }),

                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 1,
                position: Vec2::new(11.05, 10.0),
                radius: 0.5,
                speed: Some(3.0),
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::new(),
    };

    let output = step_frame(&frame, 0.05);

    assert_eq!(output[0].position, position);
    assert_eq!(output[0].target, None);
    assert_eq!(
        output[0].plain_progress.as_ref().unwrap().activity,
        MovementActivity::Blocked
    );
}

#[test]
fn plain_stall_claims_nearest_spare_and_never_requeues_a_used_slot() {
    let frame = stalled_plain_frame(vec![Vec2::new(13.0, 10.0), Vec2::new(15.0, 10.0)]);
    let first = step_frame(&frame, 0.05).remove(0);
    let first_progress = first.plain_progress.unwrap();
    assert_eq!(first.target, Some(Vec2::new(13.0, 10.0)));
    assert_eq!(first_progress.slot, Vec2::new(13.0, 10.0));
    assert_eq!(first_progress.claim_count, 1);
    assert_eq!(first_progress.pool_slots, vec![Vec2::new(15.0, 10.0)]);

    let mut second_frame = frame;
    second_frame.units[0].position = first.position;
    second_frame.units[0].target = first.target;
    second_frame.units[0].plain_progress = Some(PlainMoveProgress {
        stalled_ticks: 19,
        best_distance: first.position.distance(Vec2::new(13.0, 10.0)),
        ..first_progress
    });
    let second = step_frame(&second_frame, 0.05).remove(0);
    let second_progress = second.plain_progress.unwrap();
    assert_eq!(second.target, Some(Vec2::new(15.0, 10.0)));
    assert_eq!(second_progress.slot, Vec2::new(15.0, 10.0));
    assert_eq!(second_progress.claim_count, 2);
    assert!(!second_progress.pool_slots.contains(&Vec2::new(20.0, 10.0)));
}

#[test]
fn plain_move_without_a_spare_blocks_on_the_first_one_second_deadline() {
    let frame = stalled_plain_frame(Vec::new());
    let output = step_frame(&frame, 0.05).remove(0);
    let progress = output.plain_progress.unwrap();
    assert_eq!(progress.activity, MovementActivity::Blocked);
    assert_eq!(progress.stalled_ticks, 20);
    assert_eq!(output.target, None);
    assert!(output.failed_blocked);
}

#[test]
fn independent_plain_pools_cannot_claim_the_same_reserved_spare() {
    let shared_spare = Vec2::new(13.0, 15.0);
    let make_mover = |unit_id, position: Vec2, target: Vec2, blocker: Vec2| {
        vec![
            MovementFrameUnit {
                unit_id,
                owner_slot: unit_id as u8,
                unit_type: unit_id,
                position,
                radius: 0.5,
                speed: Some(0.05),
                target: Some(target),
                progress: None,
                plain_progress: Some(PlainMoveProgress {
                    original_point: target,
                    slot: target,
                    pool_slots: vec![shared_spare],
                    gather_goal: None,
                    activity: MovementActivity::Moving,
                    stalled_ticks: 19,
                    best_distance: position.distance(target),
                    claim_count: 0,
                    failure_recorded: false,
                }),

                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: unit_id + 10,
                owner_slot: 2,
                unit_type: 20,
                position: blocker,
                radius: 0.5,
                speed: None,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
        ]
    };
    let frame = MovementFrame {
        bounds: Vec2::splat(64.0),
        tick_hz: 20,
        units: [
            make_mover(
                1,
                Vec2::new(10.0, 10.0),
                Vec2::new(20.0, 10.0),
                Vec2::new(11.0001, 10.0),
            ),
            make_mover(
                2,
                Vec2::new(10.0, 20.0),
                Vec2::new(20.0, 20.0),
                Vec2::new(11.0001, 20.0),
            ),
        ]
        .concat(),
        pools: BTreeMap::new(),
    };

    let output = step_frame(&frame, 0.05);
    assert_eq!(output[0].target, Some(shared_spare));
    assert_eq!(output[0].plain_progress.as_ref().unwrap().claim_count, 1);
    assert_eq!(output[1].target, None);
    assert_eq!(
        output[1].plain_progress.as_ref().unwrap().activity,
        MovementActivity::Blocked
    );
}

#[test]
fn plain_move_with_zero_legal_primary_slots_blocks_before_moving() {
    let start = Vec2::new(0.5, 0.5);
    let frame = MovementFrame {
        bounds: Vec2::splat(1.5),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: start,
                radius: 0.5,
                speed: Some(3.0),
                target: Some(Vec2::new(1.0, 1.0)),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 2,
                position: Vec2::new(1.2, 1.2),
                radius: 0.2,
                speed: None,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::new(),
    };

    let output = step_frame(&frame, 0.05).remove(0);
    let progress = output.plain_progress.unwrap();
    assert_eq!(progress.activity, MovementActivity::Blocked);
    assert_eq!(progress.slot, Vec2::new(1.0, 1.0));
    assert!(progress.pool_slots.is_empty());
    assert_eq!(output.position, start);
    assert_eq!(output.target, None);
}
