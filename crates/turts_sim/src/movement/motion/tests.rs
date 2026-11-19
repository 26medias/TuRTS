use super::*;
use crate::movement::{
    MoveProgress, MovementActivity, MovementFrame, MovementFrameOutput, MovementFrameUnit,
    MovementPool, PlainMoveProgress, step_frame_at_tick, step_frame_with_motion_guide,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
enum Branch {
    Plain,
    Retained,
    Group,
}
fn frame(branch: Branch) -> MovementFrame {
    let goal = Vec2::new(10., 5.);
    let mut actor = MovementFrameUnit {
        unit_id: 7,
        owner_slot: 0,
        unit_type: 1,
        position: Vec2::splat(5.),
        radius: 0.3,
        speed: Some(10.),
        target: Some(goal),
        progress: None,
        plain_progress: None,
        physical_escape: None,
    };
    let mut pools = BTreeMap::new();
    match branch {
        Branch::Plain => {}
        Branch::Retained => {
            actor.plain_progress = Some(PlainMoveProgress {
                arrival_yield: false,
                original_point: goal,
                slot: goal,
                pool_slots: vec![],
                gather_goal: None,
                activity: MovementActivity::Moving,
                stalled_ticks: 0,
                best_distance: 5.,
                claim_count: 0,
                failure_recorded: false,
            })
        }
        Branch::Group => {
            actor.progress = Some(MoveProgress {
                pool_id: 1,
                slot: goal,
                activity: MovementActivity::Moving,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 0,
                best_distance: 5.,
                claim_count: 0,
                follow_origin: None,
            });
            pools.insert(
                1,
                MovementPool {
                    pool_id: 1,
                    point: goal,
                    spacing: 0.7,
                    primary_count: 1,
                    slots: vec![goal],
                },
            );
        }
    }
    let mut holder = actor.clone();
    holder.unit_id = 19;
    holder.owner_slot = 1;
    holder.position = Vec2::new(5., 5.61);
    holder.target = None;
    holder.speed = None;
    holder.progress = None;
    holder.plain_progress = None;
    MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::splat(32.),
        tick_hz: 20,
        units: vec![actor, holder],
        pools,
    }
}
struct Guide {
    pose: Option<Vec2>,
    kind: MotionKind,
    requests: Vec<MotionRequest>,
    finished: usize,
    actual_arrivals_legacy: bool,
}
impl Guide {
    fn new(pose: Option<Vec2>) -> Self {
        Self {
            pose,
            kind: MotionKind::Waypoint,
            requests: vec![],
            finished: 0,
            actual_arrivals_legacy: false,
        }
    }
}
impl MotionGuide for Guide {
    fn kind(&self, _: u32) -> MotionKind {
        self.kind
    }
    fn next_position(&mut self, r: MotionRequest, _: &mut SweepBudget) -> Option<Vec2> {
        assert_eq!(self.finished, 0, "finish must follow every raw request");
        self.requests.push(r);
        if self.actual_arrivals_legacy
            && r.kind == MotionKind::Destination
            && r.position.distance(r.goal) <= r.speed * r.dt
        {
            None
        } else {
            self.pose
        }
    }
    fn finish_prediction(&mut self, _: &mut SweepBudget) {
        self.finished += 1;
    }
}
fn run(frame: &MovementFrame, guide: &mut Guide) -> Vec<MovementFrameOutput> {
    step_frame_with_motion_guide(
        frame,
        0.05,
        &BTreeSet::new(),
        &mut SweepBudget::new(1),
        guide,
    )
}
#[test]
fn all_three_branches_bypass_legacy_wallward_steering() {
    for branch in [Branch::Plain, Branch::Retained, Branch::Group] {
        let frame = frame(branch);
        let expected = Vec2::new(5.5, 5.);
        let legacy = step_frame_at_tick(&frame, 0.05, 1);
        assert_ne!(
            legacy.iter().find(|u| u.unit_id == 7).unwrap().position,
            expected,
            "fixture must exercise old separation: {branch:?}"
        );
        let mut guide = Guide::new(Some(expected));
        let out = run(&frame, &mut guide);
        assert_eq!(
            out.iter().find(|u| u.unit_id == 7).unwrap().position,
            expected,
            "{branch:?}"
        );
        assert_eq!(
            out.iter().find(|u| u.unit_id == 19).unwrap().position,
            frame.units[1].position
        );
        assert_eq!(guide.requests.len(), 1);
        assert_eq!(guide.requests[0].goal, frame.units[0].target.unwrap());
        assert_eq!(guide.finished, 1);
    }
}
#[test]
fn no_hook_and_out_of_scope_motion_are_identical() {
    for branch in [Branch::Plain, Branch::Retained, Branch::Group] {
        let mut frame = frame(branch);
        for invert in [false, true] {
            if invert {
                frame.units.reverse();
            }
            let expected = step_frame_at_tick(&frame, 0.05, 1);
            let mut guide = Guide::new(None);
            guide.kind = MotionKind::Destination;
            assert_eq!(run(&frame, &mut guide), expected, "{branch:?}");
            assert_eq!(guide.finished, 1);
        }
    }
}
#[test]
fn private_short_waypoints_are_controlled_before_arrival_shortcuts() {
    for branch in [Branch::Plain, Branch::Retained, Branch::Group] {
        let mut frame = frame(branch);
        frame.units.remove(1);
        let goal = Vec2::new(5.25, 5.);
        frame.units[0].target = Some(goal);
        if let Some(p) = frame.units[0].plain_progress.as_mut() {
            p.slot = goal;
            p.original_point = goal;
        }
        if let Some(p) = frame.units[0].progress.as_mut() {
            p.slot = goal;
        }
        let mut guide = Guide::new(Some(frame.units[0].position));
        let out = run(&frame, &mut guide);
        assert_eq!(guide.requests.len(), 1, "{branch:?}");
        assert_eq!(out[0].position, frame.units[0].position, "{branch:?}");
        assert!(
            out[0].target.is_some(),
            "private corner cannot finish order: {branch:?}"
        );
        guide = Guide::new(Some(frame.units[0].position));
        guide.kind = MotionKind::Destination;
        guide.actual_arrivals_legacy = true;
        assert_eq!(
            run(&frame, &mut guide),
            step_frame_at_tick(&frame, 0.05, 1),
            "{branch:?}"
        );
    }
}
#[test]
fn malformed_modeled_poses_hold_instead_of_reentering_legacy_steering() {
    for branch in [Branch::Plain, Branch::Retained, Branch::Group] {
        let frame = frame(branch);
        for pose in [Vec2::splat(f32::NAN), Vec2::new(6., 5.), Vec2::new(-1., 5.)] {
            let mut guide = Guide::new(Some(pose));
            let out = run(&frame, &mut guide);
            assert_eq!(
                out.iter().find(|u| u.unit_id == 7).unwrap().position,
                frame.units[0].position
            );
            assert_eq!(guide.requests.len(), 1);
        }
    }
}
#[test]
fn occupied_actual_destination_keeps_legacy_closest_arrival_and_holder() {
    let mut frame = frame(Branch::Plain);
    frame.units[0].speed = Some(100.);
    frame.units[0].target = Some(Vec2::new(6., 5.));
    frame.units[1].position = Vec2::new(6., 5.);
    let mut guide = Guide::new(Some(Vec2::new(5.5, 5.)));
    guide.kind = MotionKind::Destination;
    guide.actual_arrivals_legacy = true;
    let expected = step_frame_at_tick(&frame, 0.05, 1);
    let out = run(&frame, &mut guide);
    assert_eq!(out, expected);
    assert_eq!(
        out.iter().find(|u| u.unit_id == 19).unwrap().position,
        frame.units[1].position
    );
    assert!(out[0].position.distance(out[1].position) >= 0.6);
}
#[test]
fn final_exact_guard_still_rejects_a_modeled_collision() {
    let mut frame = frame(Branch::Plain);
    frame.units[1].position = Vec2::new(5.75, 5.);
    let mut guide = Guide::new(Some(Vec2::new(5.5, 5.)));
    let out = run(&frame, &mut guide);
    let a = out.iter().find(|u| u.unit_id == 7).unwrap();
    let b = out.iter().find(|u| u.unit_id == 19).unwrap();
    assert!(a.position.distance(b.position) >= 0.6);
    assert_ne!(a.position, Vec2::new(5.5, 5.));
}

#[test]
fn modeled_plain_hold_records_real_stall_without_changing_the_endpoint() {
    let frame = frame(Branch::Plain);
    let mut guide = Guide::new(Some(frame.units[0].position));
    let out = run(&frame, &mut guide);
    let actor = out.iter().find(|u| u.unit_id == 7).unwrap();
    let progress = actor
        .plain_progress
        .as_ref()
        .expect("modeled wait needs ordinary recovery state");
    assert_eq!(actor.target, frame.units[0].target);
    assert_eq!(progress.original_point, frame.units[0].target.unwrap());
    assert_eq!(progress.stalled_ticks, 1);
    assert!(!actor.failed_blocked);
}

#[test]
fn guided_group_hold_keeps_own_pool_pass_recovery_before_spare_failure() {
    let mut frame = frame(Branch::Group);
    frame.units[0].progress.as_mut().unwrap().stalled_ticks = frame.tick_hz - 1;
    let mut peer = frame.units[0].clone();
    peer.unit_id = 19;
    peer.position = Vec2::new(5., 5.61);
    peer.target = Some(Vec2::new(10., 5.61));
    peer.progress.as_mut().unwrap().slot = peer.target.unwrap();
    peer.progress.as_mut().unwrap().stalled_ticks = 0;
    frame.units[1] = peer;
    frame
        .pools
        .get_mut(&1)
        .unwrap()
        .slots
        .push(Vec2::new(10., 5.61));
    for invert in [false, true] {
        let mut input = frame.clone();
        if invert {
            input.units.reverse();
        }
        let mut guide = Guide::new(Some(Vec2::splat(5.)));
        let out = run(&input, &mut guide);
        let actor = out.iter().find(|u| u.unit_id == 7).unwrap();
        let progress = actor.progress.as_ref().unwrap();
        assert_eq!(actor.position, Vec2::splat(5.));
        assert!(progress.stalling_on_group);
        assert!(
            progress.pass_mode,
            "own cohort wait must try pass mode first"
        );
        assert_eq!(progress.claim_count, 0);
        assert_eq!(progress.stalled_ticks, 0);
        assert_eq!(progress.activity, MovementActivity::Moving);
        assert_eq!(actor.target, frame.units[0].target);
        assert!(!actor.failed_blocked);
    }
}

struct ArrivalGuide {
    actual: BTreeMap<u32, Vec2>,
    requests: Vec<MotionRequest>,
    arrive: bool,
}
impl MotionGuide for ArrivalGuide {
    fn kind(&self, unit: u32) -> MotionKind {
        if self.actual.contains_key(&unit) {
            MotionKind::Waypoint
        } else {
            MotionKind::Destination
        }
    }
    fn arrival_destination(&self, unit: u32) -> Option<Vec2> {
        self.actual.get(&unit).copied()
    }
    fn next_position(&mut self, request: MotionRequest, _: &mut SweepBudget) -> Option<Vec2> {
        self.requests.push(request);
        (request.kind == MotionKind::Waypoint).then(|| {
            if self.arrive {
                request.preferred_position()
            } else {
                request.position
            }
        })
    }
}
#[test]
fn same_private_corner_different_real_goals_never_creates_an_arrival_slot() {
    for invert in [false, true] {
        let mut frame = frame(Branch::Plain);
        let corner = Vec2::new(6., 5.);
        frame.units[0].target = Some(corner);
        let mut peer = frame.units[0].clone();
        peer.unit_id = 19;
        peer.position = Vec2::new(7., 5.);
        frame.units[1] = peer;
        if invert {
            frame.units[0].unit_id = 19;
            frame.units[1].unit_id = 7;
            frame.units.reverse();
        }
        let mut guide = ArrivalGuide {
            actual: frame
                .units
                .iter()
                .map(|u| {
                    (
                        u.unit_id,
                        if u.position.x < 6. {
                            Vec2::new(12., 5.)
                        } else {
                            Vec2::new(1., 5.)
                        },
                    )
                })
                .collect(),
            requests: vec![],
            arrive: false,
        };
        let output = step_frame_with_motion_guide(
            &frame,
            0.05,
            &BTreeSet::new(),
            &mut SweepBudget::new(1),
            &mut guide,
        );
        assert_eq!(guide.requests.len(), 2);
        assert!(
            guide.requests.iter().all(|r| r.goal == corner),
            "no tie/closest slot may replace a connector"
        );
        for out in output {
            assert!(out.arrival_slot.is_none());
            assert_eq!(out.target, Some(corner));
            if let Some(p) = out.plain_progress {
                assert!(!p.arrival_yield);
                assert_eq!(p.claim_count, 0);
                assert!(p.pool_slots.is_empty());
            }
        }
    }
}
#[test]
fn shortening_a_private_connector_keeps_actual_destination_service_order() {
    for invert in [false, true] {
        let mut frame = frame(Branch::Plain);
        frame.units[0].position = Vec2::new(3., 5.);
        let mut peer = frame.units[0].clone();
        peer.unit_id = 19;
        peer.position = Vec2::new(5., 5.);
        peer.target = Some(Vec2::new(6., 5.));
        frame.units[1] = peer;
        if invert {
            frame.units[0].unit_id = 19;
            frame.units[1].unit_id = 7;
            frame.units.reverse();
        }
        let guide = ArrivalGuide {
            actual: frame
                .units
                .iter()
                .map(|u| {
                    (
                        u.unit_id,
                        if u.position.x < 4. {
                            Vec2::new(13., 5.)
                        } else {
                            Vec2::new(25., 5.)
                        },
                    )
                })
                .collect(),
            requests: vec![],
            arrive: false,
        };
        let mut orders = vec![];
        for connector in [Vec2::new(8., 5.), Vec2::new(3.1, 5.)] {
            let mut units = frame.units.clone();
            units.iter_mut().find(|u| u.position.x < 4.).unwrap().target = Some(connector);
            let original = units.clone();
            assert!(
                crate::movement::arrival::prepare_with_guide(
                    &mut units,
                    &original,
                    0.05,
                    frame.bounds,
                    &guide,
                    None,
                )
                .is_empty()
            );
            orders.push(units.iter().map(|u| u.position).collect::<Vec<_>>());
        }
        assert_eq!(orders[0], orders[1]);
        assert_eq!(orders[0], vec![Vec2::new(3., 5.), Vec2::new(5., 5.)]);
    }
}
#[test]
fn private_waypoints_skip_arrival_yield_recovery_but_remain_physical_bodies() {
    let mut frame = frame(Branch::Retained);
    frame.units[0]
        .plain_progress
        .as_mut()
        .unwrap()
        .arrival_yield = true;
    frame.units[1].position = frame.units[0].target.unwrap();
    let before = frame.units[0].clone();
    let guide = ArrivalGuide {
        actual: BTreeMap::from([(7, Vec2::new(20., 5.))]),
        requests: vec![],
        arrive: false,
    };
    let mut units = frame.units.clone();
    let original = units.clone();
    crate::movement::arrival::prepare_with_guide(
        &mut units,
        &original,
        0.05,
        frame.bounds,
        &guide,
        None,
    );
    assert_eq!(units.iter().find(|u| u.unit_id == 7).unwrap(), &before);
    assert!(
        units
            .iter()
            .any(|u| u.unit_id == 19 && u.position == frame.units[1].position)
    );
}
#[test]
fn occupied_private_corner_does_not_spread_or_claim_a_nearest_destination() {
    let mut frame = frame(Branch::Plain);
    frame.units[0].target = Some(Vec2::new(6., 5.));
    frame.units[1].position = Vec2::new(6., 5.);
    let mut guide = ArrivalGuide {
        actual: BTreeMap::from([(7, Vec2::new(12., 5.))]),
        requests: vec![],
        arrive: false,
    };
    let output = step_frame_with_motion_guide(
        &frame,
        0.05,
        &BTreeSet::new(),
        &mut SweepBudget::new(1),
        &mut guide,
    );
    assert_eq!(guide.requests[0].goal, Vec2::new(6., 5.));
    let actor = output.iter().find(|u| u.unit_id == 7).unwrap();
    assert_eq!(actor.position, Vec2::splat(5.));
    assert!(actor.arrival_slot.is_none());
    let progress = actor.plain_progress.as_ref().unwrap();
    assert_eq!(progress.slot, Vec2::new(6., 5.));
    assert_eq!(progress.claim_count, 0);
    assert!(progress.pool_slots.is_empty());
    assert_eq!(
        output.iter().find(|u| u.unit_id == 19).unwrap().position,
        Vec2::new(6., 5.)
    );
}
#[test]
fn a_reached_private_segment_remains_nonterminal_in_all_raw_branches() {
    for branch in [Branch::Plain, Branch::Retained, Branch::Group] {
        let mut frame = frame(branch);
        frame.units.remove(1);
        let corner = Vec2::new(5.25, 5.);
        frame.units[0].target = Some(corner);
        if let Some(p) = frame.units[0].plain_progress.as_mut() {
            p.slot = corner;
            p.original_point = corner;
        }
        if let Some(p) = frame.units[0].progress.as_mut() {
            p.slot = corner;
        }
        let mut guide = ArrivalGuide {
            actual: BTreeMap::from([(7, Vec2::new(20., 5.))]),
            requests: vec![],
            arrive: true,
        };
        let out = step_frame_with_motion_guide(
            &frame,
            0.05,
            &BTreeSet::new(),
            &mut SweepBudget::new(1),
            &mut guide,
        );
        assert_eq!(out[0].position, corner, "{branch:?}");
        assert_eq!(out[0].target, Some(corner));
        assert!(out[0].arrival_slot.is_none());
        assert!(
            out[0]
                .progress
                .as_ref()
                .is_none_or(|p| p.activity != MovementActivity::Resting)
        );
        assert!(
            out[0]
                .plain_progress
                .as_ref()
                .is_none_or(|p| p.activity != MovementActivity::Resting)
        );
    }
}
#[test]
fn actual_contested_destination_retains_the_legacy_fractional_arrival_winner() {
    for invert in [false, true] {
        let mut frame = frame(Branch::Plain);
        frame.units[0].target = Some(Vec2::new(6., 5.));
        frame.units[0].speed = Some(100.);
        let mut peer = frame.units[0].clone();
        peer.unit_id = 19;
        peer.position = Vec2::new(7.5, 5.);
        frame.units[1] = peer;
        if invert {
            frame.units[0].unit_id = 19;
            frame.units[1].unit_id = 7;
            frame.units.reverse();
        }
        let mut guide = ArrivalGuide {
            actual: BTreeMap::new(),
            requests: vec![],
            arrive: false,
        };
        let expected = step_frame_at_tick(&frame, 0.05, 1);
        let out = step_frame_with_motion_guide(
            &frame,
            0.05,
            &BTreeSet::new(),
            &mut SweepBudget::new(1),
            &mut guide,
        );
        assert_eq!(out, expected);
        let first_id = frame
            .units
            .iter()
            .find(|u| u.position.x == 5.)
            .unwrap()
            .unit_id;
        assert_eq!(
            out.iter().find(|u| u.unit_id == first_id).unwrap().position,
            Vec2::new(6., 5.)
        );
    }
}
