//! Fog arrival against bodies of other owners (`arrival::settle_foreign`).
//! A mover uses its own goal and the physical state of a body that it can know.
use crate::movement::{
    MovementActivity, MovementFrame, MovementFrameUnit, MovementVisibility,
    step_frame_preserving_resting_at_tick_with_visibility,
};
use bevy_math::Vec2;

const DT: f32 = 0.05;
const GOAL: Vec2 = Vec2::new(20.0, 20.0);

/// The pairs (viewer slot, unit id) that sight permits. Contact needs no entry.
struct Sight(Vec<(u8, u32)>);
impl MovementVisibility for Sight {
    fn unit_permitted(&self, viewer: u8, unit: u32) -> bool {
        self.0.contains(&(viewer, unit))
    }
}

fn body(id: u32, owner: u8, position: Vec2, speed: f32, target: Option<Vec2>) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id: id,
        owner_slot: owner,
        unit_type: 1,
        position,
        radius: 0.5,
        speed: (speed > 0.0).then_some(speed),
        target,
        progress: None,
        plain_progress: None,
        physical_escape: None,
    }
}

fn frame(units: Vec<MovementFrameUnit>) -> MovementFrame {
    MovementFrame {
        bounds: Vec2::splat(64.0),
        tick_hz: 20,
        pools: Default::default(),
        units,
    }
}

fn unit(frame: &MovementFrame, id: u32) -> &MovementFrameUnit {
    frame.units.iter().find(|unit| unit.unit_id == id).unwrap()
}

/// One tick. Panics when two bodies overlap after it.
fn tick(frame: &mut MovementFrame, sight: &Sight, tick: u32) {
    let outputs = step_frame_preserving_resting_at_tick_with_visibility(
        frame,
        DT,
        &Default::default(),
        tick,
        sight,
    );
    for output in outputs {
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
    for (index, a) in frame.units.iter().enumerate() {
        for b in &frame.units[index + 1..] {
            assert!(
                a.position.distance(b.position) >= a.radius + b.radius - 1e-4,
                "tick {tick}: bodies {} and {} overlap: {:?} {:?}",
                a.unit_id,
                b.unit_id,
                a.position,
                b.position
            );
        }
    }
}

fn run(mut frame: MovementFrame, sight: &Sight, ticks: u32) -> MovementFrame {
    for index in 0..ticks {
        tick(&mut frame, sight, index);
    }
    frame
}

fn yielding(unit: &MovementFrameUnit) -> bool {
    unit.plain_progress
        .as_ref()
        .is_some_and(|progress| progress.arrival_yield)
}

/// The same two physical bodies with each choice of unit ids, row order and owner slots.
fn permutations(
    build: impl Fn(u32, u8, u32, u8) -> (MovementFrameUnit, MovementFrameUnit),
) -> Vec<(MovementFrame, u32, u32)> {
    let mut cases = Vec::new();
    for ids in [(1, 9), (9, 1)] {
        for owners in [(0, 1), (1, 0)] {
            for reverse in [false, true] {
                let (first, second) = build(ids.0, owners.0, ids.1, owners.1);
                let units = if reverse {
                    vec![second, first]
                } else {
                    vec![first, second]
                };
                cases.push((frame(units), ids.0, ids.1));
            }
        }
    }
    cases
}

#[test]
fn a_small_arrival_advantage_keeps_the_point_under_fog() {
    // The two bodies cannot see each other. They touch 0.02 s before the faster one arrives.
    for (start, fast, slow) in permutations(|fast, fast_owner, slow, slow_owner| {
        (
            body(fast, fast_owner, Vec2::new(18.0, 20.0), 3.1, Some(GOAL)),
            body(slow, slow_owner, Vec2::new(22.0, 20.0), 3.0, Some(GOAL)),
        )
    }) {
        let end = run(start, &Sight(vec![]), 400);
        assert_eq!(
            unit(&end, fast).position,
            GOAL,
            "the earlier arrival lost the point: {:?}",
            end.units
        );
        assert!(end.units.iter().all(|unit| unit.target.is_none()));
        assert!(unit(&end, slow).position.distance(GOAL) < 1.0 + 1e-3);
    }
}

#[test]
fn an_exact_tie_under_fog_has_no_winner_for_any_id_row_or_slot() {
    for (west, other) in [
        (Vec2::new(18.0, 20.0), Vec2::new(22.0, 20.0)),
        (Vec2::new(18.0, 20.0), Vec2::new(20.0, 22.0)),
        (Vec2::new(18.63, 20.0), Vec2::new(20.0, 21.37)),
    ] {
        let mut previous: Option<(Vec2, Vec2)> = None;
        for (start, a, b) in permutations(|a, a_owner, b, b_owner| {
            (
                body(a, a_owner, west, 3.0, Some(GOAL)),
                body(b, b_owner, other, 3.0, Some(GOAL)),
            )
        }) {
            let end = run(start, &Sight(vec![]), 400);
            let result = (unit(&end, a).position, unit(&end, b).position);
            assert!(end.units.iter().all(|unit| unit.target.is_none()));
            assert!(
                (result.0.distance(GOAL) - result.1.distance(GOAL)).abs() < 1e-4
                    && result.0.distance(GOAL) > 0.49,
                "a tie had a winner: {result:?}"
            );
            if let Some(previous) = previous {
                assert_eq!(previous, result, "ids, rows or slots changed a tie");
            }
            previous = Some(result);
        }
    }
}

#[test]
fn a_hidden_body_near_our_goal_changes_nothing_without_contact() {
    // The enemy crosses our goal area first and never comes into contact range of our unit.
    fn world(enemy: Option<Vec2>, sight: &Sight) -> Vec<MovementFrameUnit> {
        let mut units = vec![body(1, 0, Vec2::new(18.9, 20.0), 0.5, Some(GOAL))];
        if let Some(start) = enemy {
            units.push(body(2, 1, start, 3.0, Some(start + Vec2::new(0.0, 6.0))));
        }
        let mut frame = frame(units);
        let mut trace = Vec::new();
        for index in 0..120 {
            tick(&mut frame, sight, index);
            if let Some(enemy) = frame.units.iter().find(|unit| unit.unit_id == 2) {
                let ours = unit(&frame, 1);
                assert!(
                    // 0.2 is more than one tick of closing motion.
                    ours.position.distance(enemy.position) > 1.0 + 0.101 + 0.2,
                    "fixture error: the bodies came near contact"
                );
            }
            trace.push(unit(&frame, 1).clone());
        }
        trace
    }
    let alone = world(None, &Sight(vec![]));
    let far = world(Some(Vec2::new(50.0, 50.0)), &Sight(vec![]));
    let hidden = world(Some(Vec2::new(20.9, 17.0)), &Sight(vec![]));
    assert_eq!(alone, far);
    assert_eq!(
        alone, hidden,
        "a hidden body changed our unit without contact"
    );
    // The fixture is live: the same body, when sight permits it, changes our unit.
    let seen = world(Some(Vec2::new(20.9, 17.0)), &Sight(vec![(0, 2)]));
    assert!(
        seen.iter().any(yielding),
        "the visible first arrival was ignored"
    );
}

#[test]
fn a_seen_body_that_passes_does_not_keep_our_point() {
    // The enemy reaches our goal first and goes on. Our unit gives way and then completes
    // its order at the goal area. The closest-arrival rule may move the final point by less
    // than one body radius, because the enemy body is in sight at the goal.
    let sight = Sight(vec![(0, 2), (1, 1)]);
    let mut frame = frame(vec![
        body(1, 0, Vec2::new(18.3, 20.0), 3.0, Some(GOAL)),
        body(
            2,
            1,
            Vec2::new(20.0, 18.8),
            3.0,
            Some(Vec2::new(20.0, 30.0)),
        ),
    ]);
    let mut gave_way = false;
    for index in 0..200 {
        tick(&mut frame, &sight, index);
        gave_way |= unit(&frame, 1).target.is_some_and(|target| target != GOAL);
    }
    assert!(gave_way, "our unit never gave way to the first arrival");
    assert!(unit(&frame, 1).position.distance(GOAL) < 0.5);
    assert_eq!(unit(&frame, 2).position, Vec2::new(20.0, 30.0));
    assert!(frame.units.iter().all(|unit| unit.target.is_none()));
}

#[test]
fn equal_enemy_bodies_with_different_goals_give_equal_results() {
    // Same heading, different private goals. While the enemy bodies are equal, our unit is
    // equal, also in the tick of contact and after it.
    fn world(enemy_goal: Vec2) -> Vec<(Vec2, MovementFrameUnit)> {
        let mut frame = frame(vec![
            body(1, 0, Vec2::new(18.1, 20.0), 3.0, Some(GOAL)),
            body(2, 1, Vec2::new(21.37, 20.0), 3.0, Some(enemy_goal)),
        ]);
        (0..40)
            .map(|index| {
                tick(&mut frame, &Sight(vec![]), index);
                (unit(&frame, 2).position, unit(&frame, 1).clone())
            })
            .collect()
    }
    let contest = world(GOAL);
    for other in [Vec2::new(19.0, 20.0), Vec2::new(10.0, 20.0)] {
        let passing = world(other);
        let mut reacted = false;
        for (index, (a, b)) in contest.iter().zip(&passing).enumerate() {
            assert_eq!(a.1, b.1, "tick {index}: the enemy goal {other:?} leaked");
            reacted |= a.1.target != Some(GOAL);
            if a.0 != b.0 {
                break;
            }
        }
        assert!(
            reacted,
            "the comparison must include our reaction to the enemy body"
        );
    }
}

#[test]
fn mixed_radii_keep_clear_of_the_first_arrival() {
    for (big_first, sight) in [
        (true, Sight(vec![])),
        (false, Sight(vec![])),
        (true, Sight(vec![(0, 2), (1, 1)])),
        (false, Sight(vec![(0, 2), (1, 1)])),
    ] {
        let mut small = body(1, 0, Vec2::new(18.0, 20.0), 3.0, Some(GOAL));
        small.radius = 0.3;
        let mut big = body(2, 1, Vec2::new(22.0, 20.0), 3.0, Some(GOAL));
        big.radius = 1.0;
        if big_first {
            small.position.x = 17.6;
        } else {
            big.position.x = 22.4;
        }
        let end = run(frame(vec![small, big]), &sight, 400);
        let (first, second) = if big_first { (2, 1) } else { (1, 2) };
        assert_eq!(unit(&end, first).position, GOAL, "{:?}", end.units);
        assert!(end.units.iter().all(|unit| unit.target.is_none()));
        assert!(unit(&end, second).position.distance(GOAL) < 1.3 + 2e-3);
    }
}

#[test]
fn a_follower_of_another_owner_stops_behind_the_first_arrival() {
    // With sight, the spread rule of the base build gives the follower a point of the second
    // ring (1.9 from the goal). That rule is not a part of this change.
    for (sight, limit) in [
        (Sight(vec![]), 1.1 + 2e-3),
        (Sight(vec![(0, 2), (1, 1)]), 2.0),
    ] {
        let end = run(
            frame(vec![
                body(1, 0, Vec2::new(18.8, 20.0), 3.0, Some(GOAL)),
                body(2, 1, Vec2::new(17.2, 20.0), 3.0, Some(GOAL)),
            ]),
            &sight,
            400,
        );
        assert_eq!(unit(&end, 1).position, GOAL);
        assert!(end.units.iter().all(|unit| unit.target.is_none()));
        assert!(
            unit(&end, 2).position.distance(GOAL) < limit,
            "{:?}",
            end.units
        );
    }
}

#[test]
fn a_tie_next_to_a_building_never_enters_it() {
    // The building of a third row stands where one tied body would stop.
    for sight in [Sight(vec![]), Sight(vec![(0, 2), (1, 1), (0, 3), (1, 3)])] {
        let mut building = body(3, 1, Vec2::new(19.2, 20.0), 0.0, None);
        building.radius = 0.3;
        let end = run(
            frame(vec![
                body(1, 0, Vec2::new(20.0, 18.0), 3.0, Some(GOAL)),
                body(2, 1, Vec2::new(20.0, 22.0), 3.0, Some(GOAL)),
                building,
            ]),
            &sight,
            600,
        );
        assert_eq!(unit(&end, 3).position, Vec2::new(19.2, 20.0));
        assert!(end.units.iter().all(|unit| {
            unit.plain_progress
                .as_ref()
                .is_none_or(|progress| progress.activity != MovementActivity::Blocked)
        }));
    }
}

#[test]
fn bodies_of_one_owner_keep_the_shared_rule_under_fog() {
    // One owner knows both goals. The earlier body keeps the point, as without fog.
    let end = run(
        frame(vec![
            body(1, 0, Vec2::new(18.0, 20.0), 3.1, Some(GOAL)),
            body(2, 0, Vec2::new(22.0, 20.0), 3.0, Some(GOAL)),
            body(3, 1, Vec2::new(50.0, 50.0), 3.0, None),
        ]),
        &Sight(vec![]),
        400,
    );
    assert_eq!(unit(&end, 1).position, GOAL);
    assert!(end.units.iter().all(|unit| unit.target.is_none()));
}

#[test]
fn the_distance_to_a_hidden_enemy_goal_does_not_change_the_contact_tick() {
    // The enemy does not contest our goal. Its body follows one line in both worlds; only
    // the distance to its private goal differs. The two bodies touch inside tick 6.
    fn world(enemy_goal: Vec2) -> Vec<(Vec2, MovementFrameUnit)> {
        let mut frame = frame(vec![
            body(1, 0, Vec2::new(17.0, 20.0), 3.0, Some(GOAL)),
            body(2, 1, Vec2::new(19.55, 20.0), 3.0, Some(enemy_goal)),
        ]);
        (0..8)
            .map(|index| {
                tick(&mut frame, &Sight(vec![]), index);
                (unit(&frame, 2).position, unit(&frame, 1).clone())
            })
            .collect()
    }
    let near = world(Vec2::new(17.5, 20.0));
    let far = world(Vec2::new(10.0, 20.0));
    for (index, (a, b)) in near.iter().zip(&far).enumerate() {
        if index > 0 && near[index - 1].0 != far[index - 1].0 {
            break;
        }
        assert_eq!(a.1, b.1, "tick {index}: the enemy goal distance leaked");
    }
    assert_ne!(
        near[6].1.position,
        Vec2::new(17.0 + 7.0 * 0.15, 20.0),
        "fixture error: the bodies never touched"
    );
}

#[test]
fn a_hidden_building_at_the_goal_does_not_change_our_own_contest() {
    // Two bodies of one owner contest the goal. A hidden building of the enemy stands next
    // to the goal. Before a body of ours touches it, the building must change nothing.
    fn world(building: Vec2) -> Vec<Vec<MovementFrameUnit>> {
        let mut frame = frame(vec![
            body(1, 0, Vec2::new(18.0, 20.0), 3.0, Some(GOAL)),
            body(2, 0, Vec2::new(22.08, 20.0), 3.0, Some(GOAL)),
            body(3, 1, building, 0.0, None),
        ]);
        let mut trace = Vec::new();
        for index in 0..40 {
            tick(&mut frame, &Sight(vec![]), index);
            let near = [1, 2]
                .iter()
                .any(|id| unit(&frame, *id).position.distance(building) <= 1.0 + 0.101 + 0.15);
            if near {
                break;
            }
            trace.push(vec![unit(&frame, 1).clone(), unit(&frame, 2).clone()]);
        }
        trace
    }
    let far = world(Vec2::new(50.0, 50.0));
    let hidden = world(Vec2::new(20.0, 20.99));
    assert!(hidden.len() >= 7, "fixture error: contact came too early");
    assert_eq!(
        hidden[..],
        far[..hidden.len()],
        "a hidden building changed the contest of our two bodies before contact"
    );
    assert!(
        hidden.iter().flatten().any(yielding),
        "fixture error: the contest was not decided before contact"
    );
}

#[test]
fn exact_ties_on_one_axis_have_no_winner_at_any_distance_radius_or_speed() {
    // Bit-equal distances on one axis. The contact tick falls at a different part of a
    // step in each case, so the sweep stops a different body first. No sight, and sight.
    let mut cases = 0;
    for radius in [0.3_f32, 0.5, 1.0] {
        for speed in [0.7_f32, 3.0, 6.0] {
            for eighths in 0..=24 {
                let offset = 2.0 * radius + 0.5 + eighths as f32 / 8.0;
                for axis in [Vec2::X, Vec2::Y] {
                    for seen in [false, true] {
                        let sight = Sight(if seen { vec![(0, 2), (1, 1)] } else { vec![] });
                        let mut a = body(1, 0, GOAL - axis * offset, speed, Some(GOAL));
                        let mut b = body(2, 1, GOAL + axis * offset, speed, Some(GOAL));
                        a.radius = radius;
                        b.radius = radius;
                        let end = run(frame(vec![a, b]), &sight, 400);
                        let (da, db) = (
                            unit(&end, 1).position.distance(GOAL),
                            unit(&end, 2).position.distance(GOAL),
                        );
                        assert!(
                            (da - db).abs() < 1e-4 && end.units.iter().all(|u| u.target.is_none()),
                            "radius {radius} speed {speed} offset {offset} axis {axis:?} \
                             sight {seen}: a tie had a winner: {da} {db}"
                        );
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 900);
}

#[test]
fn a_hidden_order_that_cannot_move_its_body_changes_nothing() {
    // A hidden enemy stands behind our resting body. In one world it has an order through
    // our mover; in the other world it has no order. While the enemy body is at the same
    // place in the two worlds, and there is no contact, our mover must be the same.
    let world = |enemy_goal: Option<Vec2>| {
        let mut wall = body(4, 0, Vec2::new(20.3, 20.0), 0.0, None);
        wall.radius = 0.1;
        let mut frame = frame(vec![
            body(1, 0, Vec2::new(18.85, 20.0), 3.0, Some(GOAL)),
            body(2, 1, Vec2::new(20.95, 20.0), 20.0, enemy_goal),
            wall,
        ]);
        (0..40)
            .map(|index| {
                tick(&mut frame, &Sight(vec![]), index);
                (unit(&frame, 1).clone(), unit(&frame, 2).position)
            })
            .collect::<Vec<_>>()
    };
    let ordered = world(Some(Vec2::new(10.0, 20.0)));
    let idle = world(None);
    let mut compared = 0;
    for (index, (a, b)) in ordered.iter().zip(&idle).enumerate() {
        if a.1 != b.1 || a.0.position.distance(a.1) <= 1.0 + 0.101 {
            break;
        }
        assert_eq!(a.0, b.0, "tick {index}: a hidden order changed our mover");
        compared += 1;
    }
    // The first build changed our mover in tick 0 of this fixture.
    assert!(compared >= 2, "the fixture compared {compared} ticks only");
}

#[test]
fn general_angle_results_are_independent_of_id_owner_and_row_order() {
    // Positions within 0.0001 world units count as equal.
    const POSITION_TOLERANCE: f32 = 0.0001;
    let mut changed = 0;
    for i in 0..160 {
        let angle = i as f32 * std::f32::consts::PI / 180.0;
        let direction = Vec2::new(angle.cos(), angle.sin());
        for seen in [false, true] {
            let mut reference = None;
            for (start, a, b) in permutations(|a, ao, b, bo| {
                (
                    body(a, ao, GOAL - Vec2::X * 3.0, 3.0, Some(GOAL)),
                    body(b, bo, GOAL + direction * 3.0, 3.0, Some(GOAL)),
                )
            }) {
                let sight = Sight(if seen {
                    vec![
                        (unit(&start, a).owner_slot, b),
                        (unit(&start, b).owner_slot, a),
                    ]
                } else {
                    vec![]
                });
                let end = run(start, &sight, 150);
                let result = (unit(&end, a).position, unit(&end, b).position);
                if let Some(prior) = reference {
                    let prior: (Vec2, Vec2) = prior;
                    if result.0.distance(prior.0) > POSITION_TOLERANCE
                        || result.1.distance(prior.1) > POSITION_TOLERANCE
                    {
                        changed += 1;
                        eprintln!(
                            "ANGLE {i} seen {seen} reference {prior:?} result {result:?} ids {a} {b}"
                        );
                    }
                } else {
                    reference = Some(result);
                }
            }
        }
    }
    assert_eq!(changed, 0);
}

#[test]
fn actual_first_arrival_keeps_point_at_general_angles() {
    // A body within 0.0001 world units of the goal counts as arrived.
    const ARRIVAL_TOLERANCE: f32 = 0.0001;
    let mut violations = 0;
    let mut contested = 0;
    for i in 0..155 {
        let angle = i as f32 * std::f32::consts::PI / 180.0;
        let direction = Vec2::new(angle.cos(), angle.sin());
        for seen in [false, true] {
            for a_early in [false, true] {
                let a_distance = if a_early { 2.99 } else { 3.0 };
                let b_distance = if a_early { 3.0 } else { 2.99 };
                let sight = Sight(if seen { vec![(0, 2), (1, 1)] } else { vec![] });
                let mut f = frame(vec![
                    body(1, 0, GOAL - Vec2::X * a_distance, 3.0, Some(GOAL)),
                    body(2, 1, GOAL + direction * b_distance, 3.0, Some(GOAL)),
                ]);
                let mut first = None;
                for t in 0..150 {
                    tick(&mut f, &sight, t);
                    if first.is_none() {
                        let a = unit(&f, 1).position.distance(GOAL) < ARRIVAL_TOLERANCE;
                        let b = unit(&f, 2).position.distance(GOAL) < ARRIVAL_TOLERANCE;
                        if a != b {
                            first = Some(if a { 1 } else { 2 });
                        }
                    }
                }
                if let Some(id) = first {
                    contested += 1;
                    if unit(&f, id).position.distance(GOAL) >= ARRIVAL_TOLERANCE {
                        violations += 1;
                        eprintln!(
                            "ANGLE {i} sight {seen} a_early {a_early} first {id} end {:?}",
                            unit(&f, id).position
                        );
                    }
                }
            }
        }
    }
    eprintln!("first arrivals {contested} violations {violations}");
    assert!(contested > 500);
    assert_eq!(violations, 0);
}
