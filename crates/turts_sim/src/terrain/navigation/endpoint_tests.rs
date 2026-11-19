//! Endpoint bridge regressions. The ordinary movement solver owns collision and
//! arrival decisions; the terrain adapter must not turn its slots into orders.
use super::*;
use crate::movement::{MovementActivity, MovementFrameUnit, PlainMoveProgress};
use bevy_math::Vec2;

fn unit(unit_id: u32, position: Vec2, target: Option<Vec2>) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id,
        owner_slot: 0,
        unit_type: 1,
        position,
        radius: 0.5,
        speed: Some(2.0),
        target,
        progress: None,
        plain_progress: None,
        physical_escape: None,
    }
}

fn frame(units: Vec<MovementFrameUnit>) -> MovementFrame {
    MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::splat(32.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::new(),
    }
}

// Local retry/restoration contract only. These are the captured actor's explicit
// row inputs; no Guide, pool history or permitted terrain state is reconstructed.
fn retry_group() -> MovementFrameUnit {
    let mut original = unit(
        25,
        Vec2::new(28.193436, 22.229105),
        Some(Vec2::new(45.4, 24.)),
    );
    original.unit_type = 0;
    original.radius = 1.1;
    original.speed = Some(2.5);
    original.progress = Some(crate::movement::MoveProgress {
        pool_id: 1,
        slot: original.target.unwrap(),
        activity: MovementActivity::Moving,
        pass_mode: false,
        stalling_on_group: false,
        stalled_ticks: 0,
        best_distance: original.position.distance(original.target.unwrap()),
        claim_count: 0,
        follow_origin: None,
    });
    original
}

fn retry_output(original: &MovementFrameUnit, accepted: Vec2) -> MovementFrameOutput {
    MovementFrameOutput {
        recovery: Default::default(),
        unit_id: original.unit_id,
        position: accepted,
        target: None,
        progress: None,
        plain_progress: None,
        physical_escape: None,
        failed_blocked: false,
        arrival_slot: None,
        #[cfg(feature = "test-probes")]
        probe: Default::default(),
    }
}

#[test]
fn terrain_clipped_direct_group_counts_final_certified_stalls() {
    let mut original = retry_group();
    let from = original.position;
    let goal = original.target;
    for expected in 1..=20 {
        let was_direct = direct_request(&original, &original);
        let mut constrained = original.clone();
        constrained.target = None;
        constrained.speed = None;
        constrained.progress = None;
        let mut output = retry_output(&original, from);
        restore_requested_target(
            &frame(vec![original.clone()]),
            &original,
            &constrained,
            &mut output,
            None,
            was_direct.then_some(0.125),
        );
        assert_eq!(output.position, from);
        assert_eq!(
            output.target, goal,
            "a physical refusal never completes the Move"
        );
        let progress = output.progress.unwrap();
        assert_eq!(
            progress.stalled_ticks, expected,
            "only accepted motion earns progress"
        );
        assert_eq!(progress.slot, goal.unwrap());
        assert_eq!(progress.pool_id, 1);
        assert_eq!(progress.claim_count, 0);
        assert_eq!(progress.activity, MovementActivity::Moving);
        original.progress = Some(progress);
    }
}

#[test]
fn direct_retry_uses_existing_progress_threshold_and_excludes_policy_and_waypoint_waits() {
    for (policy_wait, waypoint, advance, expected) in [
        (true, false, 0., 7),
        (false, true, 0., 7),
        (false, false, 0.00001, 8),
        (false, false, 0.05, 0),
    ] {
        let mut original = retry_group();
        original.progress.as_mut().unwrap().stalled_ticks = 7;
        let mut initial = original.clone();
        if policy_wait {
            initial.target = None;
            initial.speed = None;
        }
        if waypoint {
            initial.target = Some(original.position + Vec2::Y);
        }
        let was_direct = direct_request(&original, &initial);
        let mut constrained = initial;
        constrained.target = None;
        constrained.speed = None;
        let accepted = original.position + Vec2::new(advance, 0.);
        let mut output = retry_output(&original, accepted);
        restore_requested_target(
            &frame(vec![original.clone()]),
            &original,
            &constrained,
            &mut output,
            None,
            was_direct.then_some(0.125),
        );
        assert_eq!(output.position, accepted);
        assert_eq!(output.target, original.target);
        assert_eq!(
            output.progress.as_ref().unwrap().stalled_ticks,
            expected,
            "policy_wait={policy_wait} waypoint={waypoint} advance={advance}"
        );
    }
}

#[test]
fn exhausted_terrain_retry_frame_counts_stalls_for_unchanged_direct_rows() {
    let original = retry_group();
    // The real exhausted retry loop can hold this direct row even though other
    // rows caused every retry. Its unchanged candidate is not a policy wait.
    let mut output = retry_output(&original, original.position);
    output.target = original.target;
    output.progress = original.progress.clone();
    restore_requested_target(
        &frame(vec![original.clone()]),
        &original,
        &original,
        &mut output,
        None,
        Some(0.125),
    );
    assert_eq!(output.position, original.position);
    assert_eq!(output.target, original.target);
    assert_eq!(
        output.progress.as_ref().unwrap().stalled_ticks,
        1,
        "a globally held direct request still earned no movement"
    );

    // An accepted ordinary direct solve already accounted for its own motion.
    // Without an actual retry/fallback cause, restoration must not count twice.
    let mut accepted = retry_output(&original, original.position);
    accepted.target = original.target;
    accepted.progress = original.progress.clone();
    accepted.progress.as_mut().unwrap().stalled_ticks = 3;
    restore_requested_target(
        &frame(vec![original.clone()]),
        &original,
        &original,
        &mut accepted,
        None,
        None,
    );
    assert_eq!(accepted.progress.as_ref().unwrap().stalled_ticks, 3);
}

#[test]
fn only_actual_terrain_retry_causes_credit_for_previously_direct_requests() {
    let original = retry_group();
    for (policy_wait, waypoint, prefix_retry, whole_frame_held, expected) in [
        (false, false, false, false, None),
        (false, false, false, true, Some(0.125)),
        (false, false, true, false, Some(0.125)),
        (true, false, true, true, None),
        (false, true, true, true, None),
    ] {
        let mut initial = original.clone();
        if policy_wait {
            initial.target = None;
            initial.speed = None;
        }
        if waypoint {
            initial.target = Some(original.position + Vec2::Y);
        }
        let mut constrained = initial.clone();
        if prefix_retry {
            constrained.target = None;
            constrained.speed = None;
        }
        assert_eq!(
            retry_progress_step(
                &original,
                &constrained,
                direct_request(&original, &initial),
                whole_frame_held,
                0.05
            ),
            expected,
            "policy={policy_wait} waypoint={waypoint} prefix={prefix_retry} whole={whole_frame_held}"
        );
    }
}

// A moving-peer kernel seam with explicit inputs, not a replay of missing Guide
// history. The real GM2 trace separately establishes this cached-escape case.
struct RetryGuide;
impl crate::movement::motion::MotionGuide for RetryGuide {
    fn kind(&self, _: u32) -> crate::movement::motion::MotionKind {
        crate::movement::motion::MotionKind::Waypoint
    }
    fn arrival_destination(&self, id: u32) -> Option<Vec2> {
        Some(if id == 25 {
            Vec2::new(20., 4.)
        } else {
            Vec2::new(6., 4.)
        })
    }
    fn next_position(
        &mut self,
        request: crate::movement::motion::MotionRequest,
        _: &mut crate::movement::sweep::SweepBudget,
    ) -> Option<Vec2> {
        Some(request.preferred_position())
    }
}

#[test]
fn cached_waypoint_escape_cannot_override_a_certified_retry_prefix() {
    let mut original = unit(25, Vec2::new(4., 4.), Some(Vec2::new(20., 4.)));
    original.radius = 1.;
    original.speed = Some(2.5);
    let prefix = Vec2::new(4.01, 4.);
    let mut candidate = original.clone();
    candidate.target = Some(Vec2::new(5., 3.));
    candidate.physical_escape = Some(crate::movement::PhysicalEscape {
        target: Some(Vec2::new(4., 7.)),
    });
    constrain_terrain_prefix(&original, &mut candidate, prefix);
    let mut peer = unit(26, Vec2::new(1.8, 4.), Some(Vec2::new(6., 4.)));
    peer.radius = 1.;
    peer.speed = Some(5.);
    let trial = frame(vec![candidate.clone(), peer]);
    let mut outputs = crate::movement::step_frame_with_motion_guide(
        &trial,
        0.05,
        &std::collections::BTreeSet::new(),
        &mut crate::movement::sweep::SweepBudget::new(3086),
        &mut RetryGuide,
    );
    let output = outputs
        .iter_mut()
        .find(|row| row.unit_id == original.unit_id)
        .unwrap();
    assert_eq!(
        output.position, prefix,
        "cached local escape must not replace the already certified prefix"
    );
    restore_requested_target(
        &frame(vec![original.clone()]),
        &original,
        &candidate,
        output,
        None,
        None,
    );
    assert_eq!(
        output.target, original.target,
        "a prefix is never terminal arrival"
    );
    assert!(
        output.physical_escape.is_none(),
        "private escape intent cannot become a body obligation"
    );
}

#[test]
fn an_original_body_escape_is_retained_while_an_unsafe_trial_holds() {
    let mut original = unit(25, Vec2::new(4., 4.), Some(Vec2::new(20., 4.)));
    original.physical_escape = Some(crate::movement::PhysicalEscape {
        target: Some(Vec2::new(4., 7.)),
    });
    let mut candidate = original.clone();
    constrain_terrain_prefix(&original, &mut candidate, Vec2::new(4.01, 4.));
    let mut output = crate::movement::step_frame(&frame(vec![candidate.clone()]), 0.05).remove(0);
    assert_eq!(
        output.position, original.position,
        "hold only this unsafe body for the trial"
    );
    restore_requested_target(
        &frame(vec![original.clone()]),
        &original,
        &candidate,
        &mut output,
        None,
        None,
    );
    assert_eq!(output.target, original.target);
    assert_eq!(
        output.physical_escape, original.physical_escape,
        "an original physical recovery obligation must survive the retry"
    );
}

fn bridge_tick(frame: &mut MovementFrame, ties: &mut BTreeMap<u32, Vec2>) {
    let mut outputs = crate::movement::step_frame(frame, 0.05);
    for output in &mut outputs {
        let original = frame
            .units
            .iter()
            .find(|unit| unit.unit_id == output.unit_id)
            .unwrap();
        if let Some(slot) = output.arrival_slot {
            ties.insert(output.unit_id, slot);
        }
        restore_requested_target(
            frame,
            original,
            original,
            output,
            ties.get(&output.unit_id).copied(),
            None,
        );
    }
    for output in outputs {
        let actor = frame
            .units
            .iter_mut()
            .find(|unit| unit.unit_id == output.unit_id)
            .unwrap();
        actor.position = output.position;
        actor.target = output.target;
        actor.progress = output.progress;
        actor.plain_progress = output.plain_progress;
        actor.physical_escape = output.physical_escape;
    }
}

#[test]
fn moving_obstruction_cannot_finish_order_at_temporary_slot() {
    let destination = Vec2::new(10.0, 10.0);
    for invert in [false, true] {
        let actor = unit(
            if invert { 99 } else { 1 },
            Vec2::new(8.0, 10.0),
            Some(destination),
        );
        let moving = unit(
            if invert { 1 } else { 99 },
            destination,
            Some(Vec2::new(20.0, 10.0)),
        );
        let actor_id = actor.unit_id;
        let mut world = frame(if invert {
            vec![moving, actor]
        } else {
            vec![actor, moving]
        });
        let mut ties = BTreeMap::new();
        for _ in 0..240 {
            bridge_tick(&mut world, &mut ties);
            let actor = world
                .units
                .iter()
                .find(|unit| unit.unit_id == actor_id)
                .unwrap();
            assert!(actor.target.is_some() || actor.position == destination);
            assert!(actor.target.is_none() || requested_target(actor) == Some(destination));
            assert!(world.units[0].position.distance(world.units[1].position) >= 1.0 - 1e-5);
        }
        let actor = world
            .units
            .iter()
            .find(|unit| unit.unit_id == actor_id)
            .unwrap();
        assert_eq!(actor.position, destination);
        assert_eq!(actor.target, None);
    }
}

#[test]
fn settled_holder_keeps_point_and_closest_arrival_can_complete() {
    let destination = Vec2::new(10.0, 10.0);
    let actor = unit(1, Vec2::new(8.0, 10.0), Some(destination));
    let holder = unit(99, destination, None);
    let mut world = frame(vec![actor, holder]);
    let mut ties = BTreeMap::new();
    for _ in 0..200 {
        bridge_tick(&mut world, &mut ties);
    }
    assert_eq!(world.units[1].position, destination);
    assert_eq!(world.units[0].target, None);
    let distance = world.units[0].position.distance(destination);
    assert!(
        (1.0..=1.00001).contains(&distance),
        "closest arrival distance={distance}"
    );
}

#[test]
fn explicit_simultaneous_arrival_ties_remain_valid_completion() {
    let destination = Vec2::new(10.0, 10.0);
    let mut world = frame(vec![
        unit(1, Vec2::new(8.0, 10.0), Some(destination)),
        unit(99, Vec2::new(12.0, 10.0), Some(destination)),
    ]);
    let mut ties = BTreeMap::new();
    for _ in 0..200 {
        bridge_tick(&mut world, &mut ties);
        assert!(world.units[0].position.distance(world.units[1].position) >= 1.0 - 1e-5);
    }
    assert_eq!(
        ties.len(),
        2,
        "only explicit solver decisions authorize these slots"
    );
    assert!(world.units.iter().all(|unit| unit.target.is_none()));
}

#[test]
fn waypoint_completion_retains_requested_endpoint_and_epoch_input() {
    let destination = Vec2::new(10.0, 10.0);
    let original = unit(1, Vec2::new(5.0, 10.0), Some(destination));
    let mut constrained = original.clone();
    let waypoint = Vec2::new(6.0, 10.0);
    constrained.target = Some(waypoint);
    let frame = frame(vec![original.clone()]);
    let mut output = crate::movement::step_frame(&frame, 0.05).remove(0);
    output.position = waypoint;
    output.target = None;
    restore_requested_target(
        &frame,
        &original,
        &constrained,
        &mut output,
        Some(waypoint),
        None,
    );
    assert_eq!(output.target, Some(destination));
    assert_eq!(output.position, waypoint, "the bridge never snaps a pose");

    let mut fallback = original;
    fallback.target = Some(waypoint);
    fallback.plain_progress = Some(PlainMoveProgress {
        arrival_yield: false,
        original_point: destination,
        slot: waypoint,
        pool_slots: Vec::new(),
        gather_goal: None,
        activity: MovementActivity::Moving,
        stalled_ticks: 0,
        best_distance: 1.0,
        claim_count: 0,
        failure_recorded: false,
    });
    assert_eq!(requested_target(&fallback), Some(destination));
    fallback.target = None;
    assert_eq!(
        requested_target(&fallback),
        None,
        "Stop cannot revive retained progress"
    );
}

#[test]
fn rejected_resting_detour_reactivates_the_requested_goal_instead_of_freezing_it() {
    let endpoint = Vec2::new(20., 2.);
    let original = unit(1, Vec2::new(16., 4.), Some(endpoint));
    let frame = frame(vec![original.clone()]);
    let mut output = crate::movement::step_frame(&frame, 0.05).remove(0);
    output.target = None;
    output.position = original.position;
    output.plain_progress = Some(PlainMoveProgress {
        arrival_yield: false,
        original_point: endpoint,
        slot: original.position,
        pool_slots: vec![],
        gather_goal: None,
        activity: MovementActivity::Resting,
        stalled_ticks: 0,
        best_distance: 0.,
        claim_count: 1,
        failure_recorded: false,
    });
    restore_requested_target(&frame, &original, &original, &mut output, None, None);
    assert_eq!(output.target, Some(endpoint));
    let progress = output.plain_progress.unwrap();
    assert_eq!(progress.activity, MovementActivity::Moving);
    assert_eq!(progress.slot, endpoint);
    assert_eq!(progress.best_distance, original.position.distance(endpoint));
}

#[test]
fn exact_requested_arrival_retires_only_unrecorded_local_plain_refusal() {
    for recorded in [false, true] {
        let endpoint = Vec2::new(12.0, 8.0);
        let mut original = unit(1, endpoint - Vec2::X, Some(endpoint));
        original.plain_progress = Some(PlainMoveProgress {
            arrival_yield: false,
            original_point: endpoint,
            slot: endpoint,
            pool_slots: vec![],
            gather_goal: None,
            activity: MovementActivity::Blocked,
            stalled_ticks: 20,
            best_distance: 1.0,
            claim_count: 3,
            failure_recorded: recorded,
        });
        let frame = frame(vec![original.clone()]);
        let mut output = crate::movement::step_frame(&frame, 1.0).remove(0);
        assert_eq!(output.position, endpoint);
        assert!(output.target.is_none());
        restore_requested_target(&frame, &original, &original, &mut output, None, None);
        assert_eq!(
            output.plain_progress.is_some(),
            recorded,
            "only an unacknowledged terrain-local refusal becomes successful arrival"
        );
    }
}

#[test]
fn certified_parking_arrival_retires_escape_and_holds_against_later_arrivals() {
    for reversed in [false, true] {
        let public = Vec2::new(20., 10.);
        let parking = Vec2::new(8.125, 10.);
        let mut original = unit(
            if reversed { 99 } else { 1 },
            Vec2::new(8., 10.),
            Some(public),
        );
        original.speed = Some(100.);
        original.physical_escape = Some(crate::movement::PhysicalEscape {
            target: Some(Vec2::new(8.25, 10.)),
        });
        let mut constrained = original.clone();
        constrained.target = Some(parking);
        let mut navigation = TerrainNavigation::default();
        navigation.load_waypoint_motion(&original, &mut constrained, false);
        assert!(constrained.physical_escape.is_none());
        let mut output =
            crate::movement::step_frame(&frame(vec![constrained.clone()]), 0.05).remove(0);
        assert_eq!(output.position, parking);
        assert_eq!(output.target, None);
        assert_eq!(
            accept_parking_arrival(&original, &constrained, &mut output, None),
            None
        );
        assert_eq!(
            accept_parking_arrival(&original, &constrained, &mut output, Some(parking)),
            Some(parking)
        );
        assert_eq!(output.arrival_slot, Some(parking));
        assert!(output.physical_escape.is_none());
        let mut holder = constrained;
        holder.position = output.position;
        holder.target = output.target;
        holder.progress = output.progress;
        holder.plain_progress = output.plain_progress;
        holder.physical_escape = output.physical_escape;
        let later = unit(
            if reversed { 1 } else { 99 },
            Vec2::new(7., 10.),
            Some(parking),
        );
        let mut active = frame(vec![holder, later]);
        let mut ties = BTreeMap::new();
        for _ in 0..20 {
            bridge_tick(&mut active, &mut ties);
            assert_eq!(active.units[0].position, parking);
            assert!(active.units[0].target.is_none());
            assert!(active.units[0].physical_escape.is_none());
            assert!(active.units[0].position.distance(active.units[1].position) >= 1.);
        }
    }
}
