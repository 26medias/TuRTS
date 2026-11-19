#![allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
use super::{
    MovementActivity, MovementFrame, MovementFrameUnit, MovementInput, PlainMoveProgress, step,
    step_frame,
};
use bevy_math::Vec2;
use std::collections::BTreeMap;

fn stalled_plain_frame(pool_slots: Vec<Vec2>) -> MovementFrame {
    let point = Vec2::new(20.0, 10.0);
    MovementFrame {
        recovery: Default::default(),
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
                    arrival_yield: false,
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
        recovery: Default::default(),
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
        recovery: Default::default(),
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
                arrival_yield: false,
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
fn plain_move_uses_physical_clearance_instead_of_a_future_reservation_buffer() {
    let target = Vec2::new(0.5, 1.2);
    let frame = MovementFrame {
        recovery: Default::default(),
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

    assert_eq!(output[0].position, Vec2::new(0.5, 0.65));
    assert_eq!(output[0].target, Some(target));
    assert!(output[0].plain_progress.is_none());
    assert!(output[0].position.distance(output[1].position) >= 1.0);
}

#[test]
fn blocked_plain_unit_does_not_escape_for_reservation_spacing_alone() {
    let position = Vec2::new(10.0, 10.0);
    let frame = MovementFrame {
        recovery: Default::default(),
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
                    arrival_yield: false,
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
        arrival_yield: false,
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
fn independent_plain_pools_can_approach_a_spare_before_either_body_arrives() {
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
                    arrival_yield: false,
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
        recovery: Default::default(),
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
    assert_eq!(output[1].target, Some(shared_spare));
    assert_eq!(
        output[1].plain_progress.as_ref().unwrap().activity,
        MovementActivity::Moving
    );
}

#[test]
fn plain_move_with_zero_legal_primary_slots_blocks_before_moving() {
    let start = Vec2::new(0.5, 0.5);
    let frame = MovementFrame {
        recovery: Default::default(),
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
                position: Vec2::new(0.75, 0.75),
                radius: 0.6,
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

#[test]
fn plain_move_finds_nearest_physical_point_when_the_hex_pool_is_empty() {
    let start = Vec2::new(0.5, 0.5);
    let frame = MovementFrame {
        recovery: Default::default(),
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
    assert_eq!(progress.activity, MovementActivity::Moving);
    let centre = f64::from(1.2_f32);
    let r = f64::from(0.5_f32) + f64::from(0.2_f32);
    let minimum = r - (centre - 1.0) * 2.0_f64.sqrt();
    assert!((f64::from(progress.slot.distance(Vec2::ONE)) - minimum).abs() < 0.0001);
    assert!(
        progress.slot.x >= 0.5
            && progress.slot.y >= 0.5
            && progress.slot.x <= 1.0
            && progress.slot.y <= 1.0
    );
    assert!(progress.slot.distance(Vec2::splat(1.2)) >= 0.7);
    assert_ne!(output.position, start);
    assert!(output.position.distance(start) <= 3.0 * 0.05 + 1.0e-6);
    assert!(output.position.distance(Vec2::splat(1.2)) >= 0.7);
    assert!(progress.pool_slots.is_empty());
    assert!(output.target.is_some());
}

#[test]
fn certified_stall_deadline_survives_an_optimistic_raw_step() {
    // The preceding guard has already certified a full second without progress.
    // Raw projection alone cannot retire that recovery debt.
    for grouped in [false, true] {
        let mut frame = stalled_plain_frame(vec![Vec2::new(13., 10.)]);
        frame.units.truncate(1);
        frame.units[0].speed = Some(3.);
        let plain = frame.units[0].plain_progress.as_mut().unwrap();
        plain.stalled_ticks = 60;
        if grouped {
            let p = frame.units[0].plain_progress.take().unwrap();
            frame.units[0].progress = Some(super::MoveProgress {
                pool_id: 7,
                slot: p.slot,
                activity: MovementActivity::Moving,
                pass_mode: true,
                stalling_on_group: false,
                stalled_ticks: 60,
                best_distance: p.best_distance,
                claim_count: 0,
                follow_origin: None,
            });
            frame.pools.insert(
                7,
                super::MovementPool {
                    pool_id: 7,
                    point: p.slot,
                    spacing: 1.1,
                    primary_count: 1,
                    slots: vec![p.slot, Vec2::new(13., 10.)],
                },
            );
        }
        let output = step_frame(&frame, 0.05).remove(0);
        let (slot, claims, stalled) = if grouped {
            let p = output.progress.unwrap();
            (p.slot, p.claim_count, p.stalled_ticks)
        } else {
            let p = output.plain_progress.unwrap();
            (p.slot, p.claim_count, p.stalled_ticks)
        };
        assert_eq!(slot, Vec2::new(13., 10.));
        assert_eq!(claims, 1);
        assert_eq!(
            stalled, 0,
            "a deliberate recovery starts a fresh certified clock"
        );
    }
}

struct RecoveryGuide {
    next: Vec2,
    calls: usize,
}
impl super::motion::MotionGuide for RecoveryGuide {
    fn kind(&self, _: u32) -> super::motion::MotionKind {
        super::motion::MotionKind::Destination
    }
    fn next_position(
        &mut self,
        _: super::motion::MotionRequest,
        _: &mut super::sweep::SweepBudget,
    ) -> Option<Vec2> {
        self.calls += 1;
        Some(self.next)
    }
}
fn captured_clear_recovery_frame(escape: bool) -> MovementFrame {
    // Actual GM3 tick2400 recovery inputs. This tests the raw-motion contract,
    // not a reconstruction of the missing controller/Guide history.
    let position = Vec2::new(35.365936279296875, 23.531789779663086);
    let target = Vec2::new(45.70000076293945, 23.5);
    MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::new(64., 48.),
        tick_hz: 20,
        pools: BTreeMap::new(),
        units: vec![MovementFrameUnit {
            unit_id: 9,
            owner_slot: 0,
            unit_type: 0,
            position,
            radius: 1.1,
            speed: Some(2.5),
            target: Some(target),
            progress: None,
            plain_progress: Some(PlainMoveProgress {
                arrival_yield: false,
                original_point: target,
                slot: target,
                pool_slots: vec![
                    Vec2::new(45.7, 27.483717),
                    Vec2::new(48., 19.516283),
                    Vec2::new(48., 27.483717),
                ],
                gather_goal: None,
                activity: MovementActivity::Blocked,
                stalled_ticks: 681,
                best_distance: 10.239281,
                claim_count: 3,
                failure_recorded: false,
            }),
            physical_escape: escape.then_some(super::PhysicalEscape {
                target: Some(Vec2::new(34.585526, 23.416143)),
            }),
        }],
    }
}
#[test]
fn blocked_plain_clear_body_uses_current_guidance_instead_of_full_retry() {
    let f = captured_clear_recovery_frame(false);
    let next = Vec2::new(35.373294830322266, 23.5324764251709);
    let mut guide = RecoveryGuide { next, calls: 0 };
    let mut budget = super::sweep::SweepBudget::new(2400);
    let out = super::frame::step_frame_with_motion_guide(
        &f,
        0.05,
        &Default::default(),
        &mut budget,
        &mut guide,
    );
    assert_eq!(
        out[0].position, next,
        "the clear-body blocked branch must use the current bounded primary"
    );
    assert_eq!(guide.calls, 1);
    assert_eq!(out[0].target, f.units[0].target);
}
#[test]
fn cleared_physical_escape_is_not_restored_by_the_local_unit_copy() {
    let f = captured_clear_recovery_frame(true);
    let mut budget = super::sweep::SweepBudget::new(2400);
    let out = super::frame::step_frame_with_motion_guide(
        &f,
        0.05,
        &Default::default(),
        &mut budget,
        &mut super::motion::NoMotionGuide,
    );
    assert!(
        out[0].physical_escape.is_none(),
        "the current clear body has discharged its old escape obligation"
    );
    assert_eq!(out[0].target, f.units[0].target);
}
#[test]
fn overlapping_physical_recovery_keeps_its_obligation_and_bypasses_optional_guidance() {
    let mut f = captured_clear_recovery_frame(true);
    f.units[0].position = Vec2::new(10., 10.);
    f.units[0].physical_escape = Some(super::PhysicalEscape {
        target: Some(Vec2::new(8., 10.)),
    });
    let mut holder = f.units[0].clone();
    holder.unit_id = 10;
    holder.owner_slot = 1;
    holder.position = Vec2::new(10.5, 10.);
    holder.speed = None;
    holder.target = None;
    holder.physical_escape = None;
    holder.plain_progress = None;
    f.units.push(holder);
    let mut guide = RecoveryGuide {
        next: f.units[0].position,
        calls: 0,
    };
    let mut budget = super::sweep::SweepBudget::new(2400);
    let out = super::frame::step_frame_with_motion_guide(
        &f,
        0.05,
        &Default::default(),
        &mut budget,
        &mut guide,
    );
    let actor = out.iter().find(|o| o.unit_id == 9).unwrap();
    let holder = out.iter().find(|o| o.unit_id == 10).unwrap();
    assert_eq!(holder.position, f.units[1].position);
    assert!(
        actor.position.distance(holder.position) >= f.units[0].position.distance(holder.position)
    );
    assert!(actor.physical_escape.is_some());
    assert_eq!(guide.calls, 0);
}

#[test]
fn physical_recovery_does_not_plan_around_an_uncontacted_hidden_body() {
    struct Fog;
    impl super::MovementVisibility for Fog {
        fn unit_permitted(&self, _: u8, _: u32) -> bool {
            false
        }
    }
    let mut frame = captured_clear_recovery_frame(true);
    let actor = &mut frame.units[0];
    actor.position = Vec2::new(10.0, 10.0);
    actor.radius = 0.5;
    actor.speed = Some(1.0);
    actor.target = None;
    actor.plain_progress = None;
    actor.physical_escape = Some(super::PhysicalEscape { target: None });
    let mut holder = actor.clone();
    holder.unit_id = 10;
    holder.speed = None;
    holder.physical_escape = None;
    frame.units.push(holder.clone());
    let mut hidden = holder;
    hidden.unit_id = 11;
    hidden.owner_slot = 1;
    hidden.position = Vec2::new(40.0, 40.0);
    frame.units.push(hidden);
    let run = |frame: &MovementFrame| {
        super::step_frame_preserving_resting_at_tick_with_visibility(
            frame,
            0.05,
            &Default::default(),
            1,
            &Fog,
        )[0]
        .clone()
    };
    let baseline = run(&frame);
    let destination = baseline.physical_escape.as_ref().unwrap().target.unwrap();
    let direction = (destination - frame.units[0].position).normalize();
    frame.units[2].position = frame.units[0].position + direction * 1.15;
    let hidden_result = run(&frame);
    assert_eq!(
        baseline.position, hidden_result.position,
        "hidden body changes physical recovery before contact"
    );
    assert_eq!(baseline.physical_escape, hidden_result.physical_escape);

    frame.units[0].physical_escape.as_mut().unwrap().target = Some(destination);
    let retained = run(&frame);
    assert_eq!(
        baseline.position, retained.position,
        "hidden body invalidated a retained destination"
    );
    assert_eq!(baseline.physical_escape, retained.physical_escape);

    struct Visible;
    impl super::MovementVisibility for Visible {
        fn unit_permitted(&self, _: u8, _: u32) -> bool {
            true
        }
    }
    let visible = super::step_frame_preserving_resting_at_tick_with_visibility(
        &frame,
        0.05,
        &Default::default(),
        1,
        &Visible,
    );
    assert_ne!(
        baseline.position, visible[0].position,
        "a visible body must reserve its space"
    );
}
