//! Retained local advice for known own settled obstacles. Original orders remain intact.
use super::*;
use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};
#[derive(Clone, Copy, Debug)]
pub(super) struct Retained {
    pub(super) route: detour::Detour,
    pub(super) corner: usize,
    pub(super) requested: Vec2,
    pub(super) radius: u32,
    pub(super) kind: u32,
    pub(super) witness: Option<(u32, ObservedBody)>,
}
fn add(total: &mut detour::Work, work: detour::Work) {
    total.scanned_rows += work.scanned_rows;
    total.generator_comparisons += work.generator_comparisons;
    total.candidates += work.candidates;
    total.body_checks += work.body_checks;
    total.terrain_calls += work.terrain_calls;
}
fn point(p: Vec2) -> Point {
    Point {
        x: f64::from(p.x),
        y: f64::from(p.y),
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "Retain the controller's single bounded query and shared owner work ports"
)]
pub(super) fn apply(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    _dt: f32,
    tick: u32,
    state: &mut TrafficState,
    service: &mut [usize],
    neighbors: &NeighborRows,
    waiting: &mut BTreeSet<u32>,
    eligible: &mut impl FnMut(usize) -> bool,
    public_goal: &mut impl FnMut(usize) -> Option<DestinationRegion>,
    clear: &mut impl FnMut(usize, Vec2, Vec2) -> bool,
    budget: &mut crate::movement::sweep::SweepBudget,
) -> Vec<bool> {
    let mut guided = vec![false; frame.units.len()];
    service.sort_by(|&a, &b| {
        let age = |i: usize| {
            let m = &state.members[&frame.units[i].unit_id];
            (
                m.attempts.get(Attempt::Detour).unwrap_or(m.accepted_tick),
                m.attempts.get(Attempt::Detour).is_some(),
                m.accepted_tick,
            )
        };
        age(a)
            .cmp(&age(b))
            .then_with(|| physical(&frame.units[a], &frame.units[b]))
    });
    for &index in service.iter() {
        let unit = &frame.units[index];
        if waiting.contains(&unit.unit_id) {
            continue;
        }
        if state.members[&unit.unit_id]
            .local_detour
            .is_some_and(|r| r.witness.is_some())
        {
            continue;
        }
        let mut retained = state
            .members
            .get_mut(&unit.unit_id)
            .unwrap()
            .local_detour
            .take()
            .filter(|r| {
                unit.target == Some(r.requested)
                    && unit.radius.to_bits() == r.radius
                    && unit.unit_type == r.kind
            });
        let had_route = retained.is_some();
        let actor_eligible = eligible(index);
        if !had_route && !actor_eligible {
            continue;
        }
        let Some(requested) = unit.target else {
            continue;
        };
        // Complete neighbor query only. Foreign bodies remain in final physics,
        // but their undisclosed intents/geometry never select this own-body route.
        let mut blockers = [detour::Blocker {
            position: Vec2::ZERO,
            radius: 1.,
        }; detour::MAX_BLOCKERS];
        let mut count = 0;
        let mut release_peer: Option<&MovementFrameUnit> = None;
        let mut being_released = false;
        let mut settled_contacts = 0usize;
        let mut widest_contact = 0.0f32;
        for &other in &neighbors[&index] {
            state.detour_filter_rows += 1;
            let peer = &frame.units[other];
            let own_peer = other != index && peer.owner_slot == unit.owner_slot;
            let separation = f64::from(unit.radius) + f64::from(peer.radius);
            if own_peer
                && state
                    .members
                    .get(&peer.unit_id)
                    .and_then(|member| member.local_detour)
                    .is_some_and(|route| {
                        route.witness.is_none()
                            && route.route.corners[0] == route.route.corners[1]
                            && route
                                .requested
                                .as_dvec2()
                                .distance(unit.position.as_dvec2())
                                < separation
                    })
            {
                being_released = true;
            }
            let in_contact = own_peer
                && actor_eligible
                && moving(peer)
                && unit.position.as_dvec2().distance(peer.position.as_dvec2()) <= separation + 0.1;
            // The guidance frame intentionally omits progress. The caller's
            // owner-scoped callback supplies the current stalled state.
            let stalled_peer = in_contact && eligible(other);
            if own_peer
                && actor_eligible
                && !moving(peer)
                && unit.position.as_dvec2().distance(peer.position.as_dvec2()) <= separation + 0.01
            {
                settled_contacts += 1;
                widest_contact = widest_contact.max(peer.radius);
            }
            if stalled_peer {
                let requested_covered =
                    requested.as_dvec2().distance(peer.position.as_dvec2()) < separation;
                let peer_target_free_of_actor = peer.target.is_some_and(|target| {
                    target.as_dvec2().distance(unit.position.as_dvec2()) >= separation
                });
                if requested_covered
                    && peer_target_free_of_actor
                    && release_peer.is_none_or(|previous| physical(peer, previous).is_lt())
                {
                    release_peer = Some(peer);
                }
            }
            // Own stationary or confirmed-stalled bodies are current obstacles.
            if own_peer && (!moving(peer) || !moving(&proposed.units[other]) || stalled_peer) {
                blockers[count] = detour::Blocker {
                    position: peer.position,
                    radius: peer.radius,
                };
                count += 1;
            }
        }
        let blockers = &blockers[..count];
        let mut pairs = state.members[&unit.unit_id].guidance_pairs;
        let exhausted = std::cell::Cell::new(false);
        let mut work = detour::Work::default();
        let mut charge = || {
            let allowed = budget.charge_guidance_pair(unit.owner_slot, &mut pairs);
            exhausted.set(exhausted.get() || !allowed);
            allowed
        };
        if let Some(route) = retained.as_mut() {
            while route.corner < 2 && unit.position == route.route.corners[route.corner] {
                route.corner += 1;
            }
            if route.corner == 2 {
                retained = None;
            } else {
                let next = route.route.corners[route.corner];
                let mut bodies_clear = true;
                for blocker in blockers {
                    work.scanned_rows += 1;
                    if !charge() {
                        bodies_clear = false;
                        break;
                    }
                    work.body_checks += 1;
                    bodies_clear &= swept_disks_clear(
                        point(unit.position),
                        point(next),
                        f64::from(unit.radius),
                        point(blocker.position),
                        point(blocker.position),
                        f64::from(blocker.radius),
                        &mut Budget::new(1),
                    )
                    .unwrap_or(false);
                }
                if bodies_clear {
                    work.terrain_calls += 1;
                    if !clear(index, unit.position, next) {
                        retained = None;
                    }
                } else if !exhausted.get() {
                    retained = None;
                }
            }
        }
        // A blocked peer can occupy this actor's private endpoint while itself
        // pinned. Retreat is temporary; the accepted destination stays intact.
        // Reserve the other scan and terrain credits for the existing detours.
        if retained.is_none()
            && !exhausted.get()
            && actor_eligible
            && let Some(peer) = release_peer
        {
            let away = (unit.position.as_dvec2() - peer.position.as_dvec2()).normalize_or_zero();
            let reach = f64::from(unit.radius) + f64::from(peer.radius) + 0.1;
            let destination = (unit.position.as_dvec2() + away * reach).as_vec2();
            let inside = away != bevy_math::DVec2::ZERO
                && destination.is_finite()
                && destination.x >= unit.radius
                && destination.y >= unit.radius
                && destination.x <= frame.bounds.x - unit.radius
                && destination.y <= frame.bounds.y - unit.radius;
            let mut safe = inside;
            if inside {
                for blocker in blockers {
                    if work.scanned_rows >= detour::MAX_BLOCKERS {
                        safe = false;
                        break;
                    }
                    work.scanned_rows += 1;
                    if !charge() {
                        safe = false;
                        break;
                    }
                    work.body_checks += 1;
                    if swept_disks_clear(
                        point(unit.position),
                        point(destination),
                        f64::from(unit.radius),
                        point(blocker.position),
                        point(blocker.position),
                        f64::from(blocker.radius),
                        &mut Budget::new(1),
                    ) != Ok(true)
                    {
                        safe = false;
                    }
                }
            }
            if safe && work.terrain_calls == 0 {
                work.terrain_calls += 1;
                if clear(index, unit.position, destination) {
                    retained = Some(Retained {
                        route: detour::Detour {
                            corners: [destination; 2],
                        },
                        corner: 0,
                        requested,
                        radius: unit.radius.to_bits(),
                        kind: unit.unit_type,
                        witness: None,
                    });
                }
            }
        }
        if retained.is_none()
            && !exhausted.get()
            && actor_eligible
            && being_released
            && settled_contacts >= 2
        {
            let goal = public_goal(index).map_or(requested, |region| region.goal);
            let away = (unit.position.as_dvec2() - goal.as_dvec2()).normalize_or_zero();
            let reach = f64::from(unit.radius) + f64::from(widest_contact) + 0.1;
            // Bounded escape directions. Once clear, the original route is retried.
            const DIRECTIONS: [(f64, f64); 8] = [
                (1.0, 0.0),
                (0.9238795325112867, 0.3826834323650898),
                (0.9238795325112867, -0.3826834323650898),
                (
                    std::f64::consts::FRAC_1_SQRT_2,
                    std::f64::consts::FRAC_1_SQRT_2,
                ),
                (
                    std::f64::consts::FRAC_1_SQRT_2,
                    -std::f64::consts::FRAC_1_SQRT_2,
                ),
                (0.3826834323650898, 0.9238795325112867),
                (0.3826834323650898, -0.9238795325112867),
                (-1.0, 0.0),
            ];
            if away != bevy_math::DVec2::ZERO {
                for (cos, sin) in DIRECTIONS {
                    let heading = bevy_math::DVec2::new(
                        away.x * cos - away.y * sin,
                        away.x * sin + away.y * cos,
                    );
                    let destination = (unit.position.as_dvec2() + heading * reach).as_vec2();
                    if !destination.is_finite()
                        || destination.x < unit.radius
                        || destination.y < unit.radius
                        || destination.x > frame.bounds.x - unit.radius
                        || destination.y > frame.bounds.y - unit.radius
                    {
                        continue;
                    }
                    let mut safe = true;
                    for blocker in blockers {
                        if work.scanned_rows >= detour::MAX_BLOCKERS {
                            safe = false;
                            break;
                        }
                        work.scanned_rows += 1;
                        if !charge() {
                            safe = false;
                            break;
                        }
                        work.body_checks += 1;
                        if swept_disks_clear(
                            point(unit.position),
                            point(destination),
                            f64::from(unit.radius),
                            point(blocker.position),
                            point(blocker.position),
                            f64::from(blocker.radius),
                            &mut Budget::new(1),
                        ) != Ok(true)
                        {
                            safe = false;
                            break;
                        }
                    }
                    if exhausted.get() {
                        break;
                    }
                    if safe && work.terrain_calls == 0 {
                        work.terrain_calls += 1;
                        if clear(index, unit.position, destination) {
                            retained = Some(Retained {
                                route: detour::Detour {
                                    corners: [destination; 2],
                                },
                                corner: 0,
                                requested,
                                radius: unit.radius.to_bits(),
                                kind: unit.unit_type,
                                witness: None,
                            });
                            break;
                        }
                    }
                }
            }
        }
        if retained.is_none() && !exhausted.get() && (had_route || actor_eligible) {
            let axis_goal = public_goal(index)
                .map(|region| region.goal)
                .filter(|goal| goal.is_finite() && *goal != unit.position)
                .unwrap_or(requested);
            let (route, attempt) = detour::propose_along(
                unit.position,
                unit.radius,
                requested,
                axis_goal,
                blockers,
                |from, to| clear(index, from, to),
                &mut charge,
            );
            add(&mut work, attempt);
            let route = if route.is_none() && !exhausted.get() {
                // One disc cannot be passed when a standing group lies behind it.
                let (route, attempt) = detour::propose_sidestep(
                    unit.position,
                    unit.radius,
                    requested,
                    axis_goal,
                    frame.bounds,
                    blockers,
                    |from, to| clear(index, from, to),
                    &mut charge,
                );
                add(&mut work, attempt);
                route
            } else {
                route
            };
            retained = route.map(|route| Retained {
                route,
                corner: 0,
                requested,
                radius: unit.radius.to_bits(),
                kind: unit.unit_type,
                witness: None,
            });
        }
        // End the closure borrow before observing its shared-budget refusal.

        state.members.get_mut(&unit.unit_id).unwrap().guidance_pairs = pairs;
        if pairs > 0 {
            state
                .members
                .get_mut(&unit.unit_id)
                .unwrap()
                .attempts
                .set(Attempt::Detour, Some(tick));
        }
        debug_assert!(
            work.body_checks
                <= detour::MAX_BODY_CHECKS
                    + detour::MAX_SIDESTEP_BODY_CHECKS
                    + detour::MAX_BLOCKERS
        );
        debug_assert!(work.scanned_rows <= detour::MAX_SCANNED_ROWS + 2 * detour::MAX_BLOCKERS);
        debug_assert!(
            work.terrain_calls
                <= detour::MAX_TERRAIN_CALLS + detour::MAX_SIDESTEP_TERRAIN_CALLS + 1
        );
        add(&mut state.detour_work, work);
        if exhausted.get() {
            budget.overflows += 1;
            waiting.insert(unit.unit_id);
            stopped(proposed, index);
        } else if let Some(route) = retained {
            proposed.units[index].target = Some(route.route.corners[route.corner]);
            proposed.units[index].speed = unit.speed;
            guided[index] = true;
            state.members.get_mut(&unit.unit_id).unwrap().passing = None;
        }
        state.members.get_mut(&unit.unit_id).unwrap().local_detour = retained;
    }
    guided
}

#[cfg(test)]
#[allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
mod tests {
    use super::*;
    #[test]
    fn accepted_public_axis_retreats_around_holder_without_replacing_private_endpoint() {
        use turts_rules::terrain::{
            Ratio,
            geometry::{Cell, Grid, Traversal},
        };
        // Actual GM2 choke pose, settled editor and accepted dispatch goal.
        // This is a current-input service seam, not a controller-history replay.
        let from = Vec2::new(31.64053726196289, 26.899999618530273);
        let endpoint = Vec2::new(52.29999923706055, 27.98371696472168);
        let holder = Vec2::new(32.07806396484375, 25.465227127075195);
        let grid = Grid {
            columns: 64,
            rows: 48,
            length_scale: 1000,
            cell_width: 1000,
            elevation_step: 250,
            floor: -8,
            ceiling: 8,
        };
        let surface = |c: Cell| {
            (c.x < 64 && c.y < 48).then_some(
                if (30..34).contains(&c.x) && !(20..28).contains(&c.y) {
                    4
                } else {
                    0
                },
            )
        };
        let clear = |from: Vec2, to: Vec2| {
            let proof = grid.sweep(
                &surface,
                point(from),
                point(to),
                f64::from(1.1_f32),
                Traversal {
                    max_step: 0,
                    max_grade: Ratio {
                        numerator: 0,
                        denominator: 1,
                    },
                },
                &mut Budget::new(1000000),
            );
            proof.result.is_ok() && proof.numerator == proof.denominator
        };
        assert!(
            detour::propose(
                from,
                1.1,
                endpoint,
                &[detour::Blocker {
                    position: holder,
                    radius: 0.4
                }],
                clear,
                || true
            )
            .0
            .is_none(),
            "the original private axis has no admitted pair of legs here"
        );
        let mut expected = None;
        for reverse in [false, true] {
            let actor_id = if reverse { 700 } else { 32 };
            let mut frame = MovementFrame {
                bounds: Vec2::new(64., 48.),
                tick_hz: 20,
                pools: BTreeMap::new(),
                units: vec![
                    MovementFrameUnit {
                        unit_id: actor_id,
                        owner_slot: 0,
                        unit_type: 0,
                        position: from,
                        radius: 1.1,
                        speed: Some(2.5),
                        target: Some(endpoint),
                        progress: None,
                        plain_progress: None,
                        physical_escape: None,
                    },
                    MovementFrameUnit {
                        unit_id: if reverse { 900 } else { 1 },
                        owner_slot: 0,
                        unit_type: 1,
                        position: holder,
                        radius: 0.4,
                        speed: None,
                        target: None,
                        progress: None,
                        plain_progress: None,
                        physical_escape: None,
                    },
                ],
            };
            if reverse {
                frame.units.reverse();
            }
            let actor = frame
                .units
                .iter()
                .position(|u| u.unit_id == actor_id)
                .unwrap();
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(5042);
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                5042,
                &mut state,
                |_, from, to| clear(from, to),
                |_, _, _, _| EntranceContact::Outside,
                |_| true,
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| {
                    (i == actor).then_some(DestinationRegion {
                        goal: Vec2::new(50., 24.),
                        min: Vec2::new(43.099995, 18.024424),
                        max: Vec2::new(56.900005, 29.975576),
                        cohort_radius: 1.1,
                    })
                },
                |i| frame.units[i].target.is_none(),
            );
            let retained = state.members[&actor_id]
                .local_detour
                .expect("accepted public axis must expose the bounded retreat");
            assert_eq!(retained.requested, endpoint);
            assert_eq!(frame.units[actor].target, Some(endpoint));
            assert_eq!(
                proposed.units[actor].target,
                Some(retained.route.corners[0])
            );
            assert!(retained.route.corners[0].x < from.x);
            assert!(retained.route.corners[1].distance(endpoint) < from.distance(endpoint));
            let bound = 8. * (f64::from(1.1_f32) + f64::from(0.4_f32));
            assert!(
                from.as_dvec2()
                    .distance(retained.route.corners[0].as_dvec2())
                    <= bound
            );
            assert!(
                retained.route.corners[0]
                    .as_dvec2()
                    .distance(retained.route.corners[1].as_dvec2())
                    <= bound
            );
            assert!(clear(from, retained.route.corners[0]));
            assert!(clear(retained.route.corners[0], retained.route.corners[1]));
            for (i, u) in frame.units.iter().enumerate() {
                if i != actor {
                    assert_eq!(proposed.units[i].position, holder);
                    assert_eq!(proposed.units[i].target, u.target);
                }
            }
            assert!(state.members[&actor_id].guidance_pairs <= 4096);
            if let Some(previous) = expected {
                assert_eq!(retained.route, previous);
            } else {
                expected = Some(retained.route);
            }
        }
    }
    fn frame(reverse: bool, foreign: bool) -> MovementFrame {
        let unit = |id, p, target, owner| MovementFrameUnit {
            unit_id: id,
            owner_slot: owner,
            unit_type: 1,
            position: p,
            radius: 0.5,
            speed: Some(3.5),
            target,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        };
        let mut units = vec![
            unit(
                if reverse { 99 } else { 1 },
                Vec2::new(8., 10.),
                Some(Vec2::new(20., 10.)),
                0,
            ),
            unit(
                if reverse { 1 } else { 99 },
                Vec2::new(10., 10.),
                None,
                if foreign { 1 } else { 0 },
            ),
        ];
        if reverse {
            units.reverse();
        }
        MovementFrame {
            bounds: Vec2::splat(30.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        }
    }
    fn drive(
        frame: &MovementFrame,
        tick: u32,
        state: &mut TrafficState,
        budget: &mut crate::movement::sweep::SweepBudget,
    ) -> (MovementFrame, BTreeSet<u32>) {
        let mut proposed = frame.clone();
        let waiting = apply_with_detours(
            frame,
            &mut proposed,
            0.05,
            tick,
            state,
            |_, _, _| true,
            |_, _, _, _| EntranceContact::Outside,
            |_| true,
            budget,
        );
        (proposed, waiting)
    }
    #[test]
    fn retained_corners_keep_endpoint_and_retire_on_changed_intent_and_owner() {
        let mut reference = None;
        for reversed in [false, true] {
            let mut original = frame(reversed, false);
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(1);
            let (proposed, waiting) = drive(&original, 1, &mut state, &mut budget);
            let actor = original
                .units
                .iter()
                .position(|u| u.target.is_some())
                .unwrap();
            let id = original.units[actor].unit_id;
            let route = state.members[&id].local_detour.unwrap();
            assert!(!waiting.contains(&id));
            assert_eq!(proposed.units[actor].target, Some(route.route.corners[0]));
            assert_eq!(original.units[actor].target, Some(Vec2::new(20., 10.)));
            assert_eq!(
                budget.pairs.iter().sum::<usize>(),
                state.detour_work.body_checks
            );
            assert!(budget.pairs[0] > 0);
            let corners = route.route.corners;
            if let Some(expected) = reference {
                assert_eq!(corners, expected);
            } else {
                reference = Some(corners);
            }
            // Synthetic accepted-corner input tests the controller lifecycle only;
            // public simulation acceptance must still execute the actual movement.
            original.units[actor].position = corners[0];
            let (proposed, _) = drive(
                &original,
                2,
                &mut state,
                &mut crate::movement::sweep::SweepBudget::new(2),
            );
            assert_eq!(proposed.units[actor].target, Some(corners[1]));
            original.units[actor].target = Some(Vec2::new(24., 10.));
            let mut proposed = original.clone();
            apply_with_detours(
                &original,
                &mut proposed,
                0.05,
                3,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |_| false,
                &mut crate::movement::sweep::SweepBudget::new(3),
            );
            assert!(state.members[&id].local_detour.is_none());
            assert_eq!(proposed.units[actor].target, Some(Vec2::new(24., 10.)));
            original.units[actor].owner_slot = 1;
            let (proposed, _) = drive(
                &original,
                4,
                &mut state,
                &mut crate::movement::sweep::SweepBudget::new(4),
            );
            assert!(state.members[&id].local_detour.is_none());
            assert_eq!(proposed.units[actor].target, original.units[actor].target);
            original.units[actor].target = None;
            drive(
                &original,
                5,
                &mut state,
                &mut crate::movement::sweep::SweepBudget::new(5),
            );
            assert!(!state.members.contains_key(&id));
        }
    }
    #[test]
    fn exhausted_guidance_credit_preserves_final_physics_credit_and_retries_fairly() {
        let original = frame(false, false);
        let mut state = TrafficState::default();
        let mut budget = crate::movement::sweep::SweepBudget::new(1);
        budget.pairs[0] = VISITS / (2 * SLOTS);
        let (proposed, waiting) = drive(&original, 1, &mut state, &mut budget);
        assert!(waiting.contains(&1));
        assert_eq!(proposed.units[0].target, None);
        assert!(state.members[&1].local_detour.is_none());
        assert_eq!(
            state.members[&1].attempts.get(Attempt::Detour),
            None,
            "no uncharged attempt may jump older pending service"
        );
        assert_eq!(budget.pairs[0], VISITS / (2 * SLOTS));
        let mut actor_pairs = 0;
        assert!(
            budget.charge_pair(0, &mut actor_pairs, 4096),
            "the final guard retains its original unused credit"
        );
        let (_, waiting) = drive(
            &original,
            2,
            &mut state,
            &mut crate::movement::sweep::SweepBudget::new(2),
        );
        assert!(!waiting.contains(&1));
        assert!(state.members[&1].local_detour.is_some());
    }
    #[test]
    fn foreign_settled_bodies_never_select_an_owner_detour() {
        let original = frame(false, true);
        let mut state = TrafficState::default();
        let mut budget = crate::movement::sweep::SweepBudget::new(1);
        let (proposed, _) = drive(&original, 1, &mut state, &mut budget);
        assert!(state.members[&1].local_detour.is_none());
        assert_eq!(proposed.units[0].target, original.units[0].target);
        assert_eq!(budget.pairs.iter().sum::<usize>(), 0);
    }
    #[test]
    fn blocked_own_mover_can_be_detoured_around_without_changing_its_order() {
        let capture: serde_json::Value =
            serde_json::from_str(include_str!("captured_gm2_browser.json")).unwrap();
        let vec2 = |v: &serde_json::Value| {
            Vec2::new(v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32)
        };
        let stalled: std::collections::BTreeSet<u32> = capture["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["plain_progress"]["activity"] == "Blocked")
            .map(|r| r["id"].as_u64().unwrap() as u32)
            .collect();
        let mut units: Vec<_> = capture["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| MovementFrameUnit {
                unit_id: r["id"].as_u64().unwrap() as u32,
                owner_slot: 0,
                unit_type: 1,
                position: vec2(&r["position"]),
                radius: r["radius"].as_f64().unwrap() as f32,
                target: (!r["target"].is_null()).then(|| vec2(&r["target"])),
                speed: Some(2.5),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect();
        let mut answer = None;
        for reverse in [false, true] {
            if reverse {
                units.reverse();
            }
            let frame = MovementFrame {
                bounds: Vec2::new(64., 48.),
                tick_hz: 20,
                units: units.clone(),
                pools: BTreeMap::new(),
            };
            let actor = frame.units.iter().position(|u| u.unit_id == 33).unwrap();
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(5860);
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                5860,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |i| stalled.contains(&frame.units[i].unit_id),
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| {
                    (i == actor).then_some(DestinationRegion {
                        goal: Vec2::new(50.000004, 24.000015),
                        min: Vec2::new(43.100002, 18.024439),
                        max: Vec2::new(56.900005, 29.975592),
                        cohort_radius: 1.1,
                    })
                },
                |i| frame.units[i].target.is_none(),
            );
            let route = state.members[&33]
                .local_detour
                .expect("stalled peer must be treated as a current obstacle");
            assert_eq!(route.requested, frame.units[actor].target.unwrap());
            assert_eq!(proposed.units[actor].target, Some(route.route.corners[0]));
            assert_eq!(route.route.corners[0], route.route.corners[1]);
            assert!(route.route.corners[0].distance(frame.units[actor].position) > 2.2);
            for (other, peer) in frame.units.iter().enumerate() {
                if other != actor {
                    assert_eq!(
                        swept_disks_clear(
                            point(frame.units[actor].position),
                            point(route.route.corners[0]),
                            f64::from(frame.units[actor].radius),
                            point(peer.position),
                            point(peer.position),
                            f64::from(peer.radius),
                            &mut Budget::new(1)
                        ),
                        Ok(true)
                    );
                }
            }
            assert!(state.members[&33].guidance_pairs <= 4096);
            if let Some(expected) = answer {
                assert_eq!(route.route.corners, expected);
            }
            answer = Some(route.route.corners);
        }
    }
    #[test]
    fn pinned_mover_does_not_repeat_a_solo_retreat() {
        let capture: serde_json::Value =
            serde_json::from_str(include_str!("captured_gm2_after_release.json")).unwrap();
        let vec2 = |v: &serde_json::Value| {
            Vec2::new(v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32)
        };
        let stalled: std::collections::BTreeSet<u32> = capture["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["plain_progress"]["activity"] == "Blocked")
            .map(|r| r["id"].as_u64().unwrap() as u32)
            .collect();
        let mut units: Vec<_> = capture["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| MovementFrameUnit {
                unit_id: r["id"].as_u64().unwrap() as u32,
                owner_slot: 0,
                unit_type: 1,
                position: vec2(&r["position"]),
                radius: r["radius"].as_f64().unwrap() as f32,
                target: (!r["target"].is_null()).then(|| vec2(&r["target"])),
                speed: Some(2.5),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect();
        for reverse in [false, true] {
            if reverse {
                units.reverse();
            }
            let frame = MovementFrame {
                bounds: Vec2::new(64., 48.),
                tick_hz: 20,
                units: units.clone(),
                pools: BTreeMap::new(),
            };
            let actor = frame.units.iter().position(|u| u.unit_id == 36).unwrap();
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(5860);
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                5860,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |i| stalled.contains(&frame.units[i].unit_id),
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| {
                    (i == actor).then_some(DestinationRegion {
                        goal: Vec2::new(50.000004, 24.000015),
                        min: Vec2::new(43.100002, 18.024439),
                        max: Vec2::new(56.900005, 29.975592),
                        cohort_radius: 1.1,
                    })
                },
                |i| frame.units[i].target.is_none(),
            );
            assert!(
                state.members[&36].local_detour.is_none(),
                "a solo actor must not oscillate on a retreat"
            );
        }
    }
    #[test]
    fn solo_mover_does_not_oscillate_after_touching_settled_units() {
        let capture: serde_json::Value =
            serde_json::from_str(include_str!("captured_gm3_solo_retreat.json")).unwrap();
        let vec2 = |v: &serde_json::Value| {
            Vec2::new(v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32)
        };
        let stalled: std::collections::BTreeSet<u32> = capture["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["plain_progress"]["activity"] == "Blocked")
            .map(|r| r["id"].as_u64().unwrap() as u32)
            .collect();
        let mut units: Vec<_> = capture["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| MovementFrameUnit {
                unit_id: r["id"].as_u64().unwrap() as u32,
                owner_slot: 0,
                unit_type: 1,
                position: vec2(&r["position"]),
                radius: r["radius"].as_f64().unwrap() as f32,
                target: (!r["target"].is_null()).then(|| vec2(&r["target"])),
                speed: Some(2.5),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect();
        for reverse in [false, true] {
            if reverse {
                units.reverse();
            }
            let frame = MovementFrame {
                bounds: Vec2::new(64., 48.),
                tick_hz: 20,
                units: units.clone(),
                pools: BTreeMap::new(),
            };
            let actor = frame.units.iter().position(|u| u.unit_id == 14).unwrap();
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(5860);
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                5860,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |i| stalled.contains(&frame.units[i].unit_id),
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| {
                    (i == actor).then_some(DestinationRegion {
                        goal: Vec2::new(48.0, 23.5),
                        min: Vec2::new(41.099995, 17.524424),
                        max: Vec2::new(54.900005, 29.475576),
                        cohort_radius: 1.1,
                    })
                },
                |i| frame.units[i].target.is_none(),
            );
            assert!(
                state.members[&14].local_detour.is_none(),
                "a solo actor must not oscillate on a retreat"
            );
        }
    }
}
