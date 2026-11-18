//! Atomic same-owner upgrades over a complete certified baseline.
use super::*;
use crate::movement::motion::{MotionGuide, MotionKind};
use crate::terrain::traffic::EnvelopeIndex;
const _: () = assert!(
    EnvelopeIndex::capacity_bound(crate::terrain::traffic::index_capacity())
        + crate::terrain::traffic::index_capacity() * 61
        + 4096
        <= bounded::SCRATCH_BYTES
);

fn semantic_destination(unit: &MovementFrameUnit, guide: &dyn MotionGuide) -> Option<Vec2> {
    unit.target?;
    match guide.kind(unit.unit_id) {
        MotionKind::Destination => unit.target,
        MotionKind::Waypoint | MotionKind::CertifiedWaypoint => {
            guide.arrival_destination(unit.unit_id)
        }
    }
    .filter(|target| target.is_finite())
}

fn support_eligible(
    original: &MovementFrameUnit,
    raw: &MovementFrameOutput,
    from: Vec2,
    dt: f32,
    destination: Option<Vec2>,
) -> bool {
    let Some(target) = destination else {
        return false;
    };
    let Some(speed) = original.speed else {
        return false;
    };
    speed.is_finite()
        && speed > 0.
        && dt.is_finite()
        && dt > 0.
        && target.is_finite()
        && from.is_finite()
        && length(from, target) > f64::from(speed) * f64::from(dt)
        && raw.position != target
        && original
            .progress
            .as_ref()
            .is_none_or(|p| p.activity != MovementActivity::Resting)
        && original
            .plain_progress
            .as_ref()
            .is_none_or(|p| p.activity != MovementActivity::Resting)
        && raw.target.is_some()
        && !raw.failed_blocked
        && raw
            .progress
            .as_ref()
            .is_none_or(|p| p.activity != MovementActivity::Resting)
        && raw
            .plain_progress
            .as_ref()
            .is_none_or(|p| p.activity != MovementActivity::Resting)
}

/// Capture only from original rows and untouched raw semantic outputs.
/// Deferred permission is not a paid ordinary certificate.
pub(super) fn deferred_eligibility(
    frame: &MovementFrame,
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    raw: &[MovementFrameOutput],
    dt: f32,
    guide: &dyn MotionGuide,
) -> Vec<bool> {
    let ordered = frame.units.windows(2).all(|w| w[0].unit_id < w[1].unit_id);
    let mut flags = Vec::with_capacity(units.len());
    for (i, unit) in units.iter().enumerate() {
        let original = if ordered {
            frame
                .units
                .binary_search_by_key(&unit.unit_id, |u| u.unit_id)
                .ok()
                .map(|j| &frame.units[j])
        } else {
            frame.units.iter().find(|u| u.unit_id == unit.unit_id)
        };
        flags.push(original.is_some_and(|old| {
            old.physical_escape.is_none()
                && raw[i].physical_escape.is_none()
                && old.radius.is_finite()
                && old.radius > 0.
                && raw[i].position.is_finite()
                && raw[i].position.cmpge(Vec2::ZERO).all()
                && raw[i].position.cmple(frame.bounds).all()
                && travel_endpoint(old, origins[i], raw[i].position, dt) == raw[i].position
                && support_eligible(
                    old,
                    &raw[i],
                    origins[i],
                    dt,
                    semantic_destination(old, guide),
                )
        }));
    }
    flags
}

/// A clipped raw arrival may retry at its own turn, but never lend support.
/// Raw output Resting/target=None describes the proposed arrival; only actual
/// certified completion or an already resting original freezes this root.
fn root_eligible(
    original: &MovementFrameUnit,
    baseline: &MovementFrameOutput,
    from: Vec2,
    raw: Vec2,
    dt: f32,
    destination: Option<Vec2>,
) -> bool {
    let (Some(target), Some(speed)) = (destination, original.speed) else {
        return false;
    };
    speed.is_finite()
        && speed > 0.
        && dt.is_finite()
        && dt > 0.
        && target.is_finite()
        && from.is_finite()
        && raw.is_finite()
        && travel_endpoint(original, from, raw, dt) == raw
        && baseline.position != target
        && baseline.position != raw
        && !baseline.failed_blocked
        && original
            .progress
            .as_ref()
            .is_none_or(|p| p.activity != MovementActivity::Resting)
        && original
            .plain_progress
            .as_ref()
            .is_none_or(|p| p.activity != MovementActivity::Resting)
}

#[cfg(test)]
#[allow(
    clippy::too_many_arguments,
    reason = "Preserve legacy certification fixtures"
)]
pub(super) fn upgrade(
    frame: &MovementFrame,
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    raw: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    spent: &mut [usize],
    admitted: &[bool],
    dt: f32,
    budget: &mut SweepBudget,
) {
    upgrade_guided(
        frame,
        units,
        origins,
        raw,
        outputs,
        spent,
        admitted,
        dt,
        budget,
        &crate::movement::motion::NoMotionGuide,
    );
}

#[allow(
    clippy::too_many_arguments,
    reason = "Share flat geometry and spent work without cloning semantic state"
)]
#[cfg(test)]
pub(super) fn upgrade_guided(
    frame: &MovementFrame,
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    raw: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    spent: &mut [usize],
    admitted: &[bool],
    dt: f32,
    budget: &mut SweepBudget,
    guide: &dyn MotionGuide,
) {
    upgrade_guided_deferred(
        frame,
        units,
        origins,
        raw,
        outputs,
        spent,
        admitted,
        &[],
        dt,
        budget,
        guide,
    );
}
#[allow(
    clippy::too_many_arguments,
    reason = "Deferred permission is separate from paid ordinary ranges"
)]
pub(super) fn upgrade_guided_deferred(
    frame: &MovementFrame,
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    raw: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    spent: &mut [usize],
    admitted: &[bool],
    deferred: &[bool],
    dt: f32,
    budget: &mut SweepBudget,
    guide: &dyn MotionGuide,
) {
    let n = units.len();
    if n == 0 || n > crate::terrain::traffic::index_capacity() {
        return;
    }
    // Simultaneous: caller raw/spent/committed, eligibility/staging, stage queue,
    // envelope storage and one complete index. No second index is live.
    let bytes = EnvelopeIndex::capacity_bound(n) + n * (8 + 8 + 1 + 1 + 1 + 1 + 1 + 8 + 32) + 4096;
    if bytes > bounded::SCRATCH_BYTES {
        return;
    }
    budget.peak_bytes = budget.peak_bytes.max(bytes);
    let mut roots = Vec::with_capacity(n);
    let mut allowed = Vec::with_capacity(n);
    // Authoritative ECS frames are sorted by handle. Use it only for lookup;
    // arbitrary caller order and duplicate-handle behavior retain the old path.
    let ordered_lookup = frame
        .units
        .windows(2)
        .all(|pair| pair[0].unit_id < pair[1].unit_id);
    for (i, unit) in units.iter().enumerate() {
        let original = if ordered_lookup {
            frame
                .units
                .binary_search_by_key(&unit.unit_id, |old| old.unit_id)
                .ok()
                .map(|index| &frame.units[index])
        } else {
            frame.units.iter().find(|old| old.unit_id == unit.unit_id)
        };
        roots.push(
            deferred.get(i).copied().unwrap_or(false)
                || admitted[i]
                    && original.is_some_and(|old| {
                        root_eligible(
                            old,
                            &outputs[i],
                            origins[i],
                            raw[i],
                            dt,
                            semantic_destination(old, guide),
                        )
                    }),
        );
        allowed.push(
            deferred.get(i).copied().unwrap_or(false)
                || admitted[i]
                    && original.is_some_and(|old| {
                        travel_endpoint(old, origins[i], raw[i], dt) == raw[i]
                            && support_eligible(
                                old,
                                &outputs[i],
                                origins[i],
                                dt,
                                semantic_destination(old, guide),
                            )
                    }),
        );
    }
    // Option<Vec<_>>::collect may grow geometrically. Reserve the exact row
    // count explicitly so the accounted envelope capacity cannot grow.
    let mut envelopes = Vec::with_capacity(n);
    for (i, u) in units.iter().enumerate() {
        let Some(mut b) = bounded::envelope(origins[i], outputs[i].position, u.radius) else {
            return;
        };
        // Raw envelopes affect nonexhaustible metadata only. Before extracting
        // any outsider, replace this union with its certified envelope below.
        if roots[i] || allowed[i] {
            let Some(r) = bounded::envelope(origins[i], raw[i], u.radius) else {
                return;
            };
            b = [
                b[0].min(r[0]),
                b[1].max(r[1]),
                b[2].min(r[2]),
                b[3].max(r[3]),
            ];
        }
        envelopes.push(b);
    }
    let Some(mut index) = EnvelopeIndex::new(n, |i| Some(envelopes[i])) else {
        return;
    };
    let mut staged = vec![false; n];
    let mut queue = Vec::with_capacity(n);
    // Preserve the old physical order. There is no owner- or handle-based promotion.
    #[cfg(feature = "test-probes")]
    let trace_ids = std::env::var("TURTS_BODY_TRACE_IDS").ok();
    #[cfg(feature = "test-probes")]
    let trace_ticks = std::env::var("TURTS_BODY_TRACE_TICKS").ok();
    for root in 0..n {
        if !roots[root] || outputs[root].position == raw[root] {
            continue;
        }
        #[cfg(feature = "test-probes")]
        let tracing = trace_ids.as_ref().is_some_and(|ids| {
            ids.split(',')
                .any(|id| id.parse::<u32>().ok() == Some(units[root].unit_id))
        }) && trace_ticks.as_ref().is_none_or(|ticks| {
            ticks
                .split(',')
                .any(|tick| tick.parse::<u32>().ok() == Some(budget.epoch))
        });
        queue.clear();
        queue.push(root);
        staged[root] = true;
        let mut cursor = 0;
        let mut complete = true;
        while cursor < queue.len() && complete {
            let actor = queue[cursor];
            cursor += 1;
            let Some(bounds) = bounded::envelope(origins[actor], raw[actor], units[actor].radius)
            else {
                complete = false;
                break;
            };
            // Empty queries are charged too. Work is never reset after rollback.
            if !guide.charge_certificate_visit(
                units[actor].unit_id,
                units[actor].owner_slot,
                true,
                budget,
            ) {
                complete = false;
                break;
            }
            budget.support_queries = budget.support_queries.saturating_add(1);
            let Some(rows) = index.query(bounds, None) else {
                complete = false;
                break;
            };
            let mut peers = 0;
            for other in rows {
                if !guide.examine_certificate_row(
                    units[actor].unit_id,
                    units[actor].owner_slot,
                    budget,
                ) {
                    complete = false;
                    break;
                }
                if other == actor {
                    continue;
                }
                budget.envelope_filters = budget.envelope_filters.saturating_add(1);
                let to = if staged[other] {
                    raw[other]
                } else {
                    outputs[other].position
                };
                let Some(actual) = bounded::envelope(origins[other], to, units[other].radius)
                else {
                    complete = false;
                    break;
                };
                if !bounded::intersects(bounds, actual) {
                    continue;
                }
                if peers == bounded::PEERS
                    || !guide.admit_certificate_row(
                        units[actor].unit_id,
                        units[actor].owner_slot,
                        true,
                        budget,
                    )
                {
                    complete = false;
                    break;
                }
                peers += 1;
                if !guide.charge_certificate_pair(
                    units[actor].unit_id,
                    units[actor].owner_slot,
                    &mut spent[actor],
                    bounded::PAIRS_PER_ACTOR,
                    budget,
                ) {
                    complete = false;
                    break;
                }
                budget.support_pairs = budget.support_pairs.saturating_add(1);
                if pair_clear(
                    origins[actor],
                    raw[actor],
                    units[actor].radius,
                    origins[other],
                    to,
                    units[other].radius,
                ) {
                    continue;
                }
                #[cfg(feature = "test-probes")]
                if tracing {
                    eprintln!(
                        "SUPPORT_PAIR tick={} root={} actor={} peer={} actor_from={:?} actor_raw={:?} peer_from={:?} peer_to={to:?} peer_raw={:?} staged={} allowed={}",
                        budget.epoch,
                        units[root].unit_id,
                        units[actor].unit_id,
                        units[other].unit_id,
                        origins[actor],
                        raw[actor],
                        origins[other],
                        raw[other],
                        staged[other],
                        allowed[other]
                    );
                }
                // A failed certified pair can request eligible friendly support.
                // The newly staged actor checks its raw pair against every
                // already staged member before this transaction may publish.
                if !staged[other]
                    && units[other].owner_slot == units[actor].owner_slot
                    && allowed[other]
                    && outputs[other].position != raw[other]
                {
                    staged[other] = true;
                    queue.push(other);
                } else {
                    complete = false;
                    break;
                }
            }
        }
        #[cfg(feature = "test-probes")]
        if tracing {
            let ids = queue
                .iter()
                .take(16)
                .map(|&i| units[i].unit_id)
                .collect::<Vec<_>>();
            eprintln!(
                "SUPPORT_END tick={} root={} complete={complete} members={} first={ids:?}",
                budget.epoch,
                units[root].unit_id,
                queue.len()
            );
        }
        if complete {
            for &i in &queue {
                outputs[i].position = raw[i];
            }
            budget.support_accepted_members = budget
                .support_accepted_members
                .saturating_add(queue.len() as u64);
        } else {
            budget.support_failures = budget.support_failures.saturating_add(1);
        }
        for &i in &queue {
            staged[i] = false;
        }
    }
    budget.index_work.add(index.work);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    pub(super) fn unit(id: u32, owner: u8, p: Vec2) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: owner,
            unit_type: 0,
            position: p,
            radius: 0.2,
            speed: Some(100.),
            target: Some(p + Vec2::X * 20.),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    pub(super) fn rows(
        units: &[MovementFrameUnit],
        positions: &[Vec2],
    ) -> Vec<MovementFrameOutput> {
        units
            .iter()
            .zip(positions)
            .map(|(u, &p)| MovementFrameOutput {
                unit_id: u.unit_id,
                position: p,
                target: u.target,
                progress: None,
                plain_progress: None,
                physical_escape: None,
                failed_blocked: false,
                arrival_slot: None,
                #[cfg(feature = "test-probes")]
                probe: Default::default(),
            })
            .collect()
    }
    pub(super) fn run(
        units: Vec<MovementFrameUnit>,
        raw: Vec<Vec2>,
        baseline: Vec<Vec2>,
        spent: &mut [usize],
        budget: &mut SweepBudget,
    ) -> Vec<MovementFrameOutput> {
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let frame = MovementFrame {
            units: units.clone(),
            bounds: Vec2::splat(64.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        let mut outputs = rows(&units, &baseline);
        upgrade(
            &frame,
            &units,
            &origins,
            &raw,
            &mut outputs,
            spent,
            &vec![true; units.len()],
            0.05,
            budget,
        );
        outputs
    }
    #[test]
    fn deferred_eligibility_uses_raw_semantics_and_real_destination() {
        struct Guide {
            kind: MotionKind,
        }
        impl MotionGuide for Guide {
            fn kind(&self, _: u32) -> MotionKind {
                self.kind
            }
            fn arrival_destination(&self, _: u32) -> Option<Vec2> {
                Some(Vec2::new(40., 2.))
            }
            fn next_position(
                &mut self,
                _: crate::movement::motion::MotionRequest,
                _: &mut SweepBudget,
            ) -> Option<Vec2> {
                None
            }
        }
        let mut u = unit(1, 0, Vec2::new(2., 2.));
        u.target = Some(Vec2::new(2.1, 2.));
        let frame = MovementFrame {
            units: vec![u.clone()],
            bounds: Vec2::splat(64.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        let mut raw = rows(&[u.clone()], &[Vec2::new(2.1, 2.)]);
        assert_eq!(
            deferred_eligibility(
                &frame,
                &[u.clone()],
                &[u.position],
                &raw,
                0.05,
                &Guide {
                    kind: MotionKind::Destination
                }
            ),
            [false]
        );
        let g = Guide {
            kind: MotionKind::Waypoint,
        };
        assert_eq!(
            deferred_eligibility(&frame, &[u.clone()], &[u.position], &raw, 0.05, &g),
            [true]
        );
        let resting = crate::movement::MoveProgress {
            pool_id: 1,
            slot: u.position,
            activity: MovementActivity::Resting,
            pass_mode: false,
            stalling_on_group: false,
            stalled_ticks: 0,
            best_distance: 0.,
            claim_count: 0,
            follow_origin: None,
        };
        for case in 0..10 {
            let mut f = frame.clone();
            let mut r = raw.clone();
            match case {
                0 => f.units[0].physical_escape = Some(PhysicalEscape { target: None }),
                1 => r[0].physical_escape = Some(PhysicalEscape { target: None }),
                2 => f.units[0].progress = Some(resting.clone()),
                3 => r[0].progress = Some(resting.clone()),
                4 => r[0].target = None,
                5 => r[0].failed_blocked = true,
                6 => r[0].position.x = f32::NAN,
                7 => r[0].position.x = 10.,
                8 => f.units[0].speed = Some(f32::NAN),
                _ => f.units[0].radius = f32::NAN,
            }
            assert_eq!(
                deferred_eligibility(&f, &f.units, &[u.position], &r, 0.05, &g),
                [false],
                "case{case}"
            );
        }
        raw[0].position = u.position;
        assert_eq!(
            deferred_eligibility(&frame, &[u.clone()], &[u.position], &raw, 0.05, &g),
            [true],
            "Holding position must not manufacture eligibility or erase eligible raw semantics"
        );
    }
    #[test]
    fn clear_friendly_chain_upgrades_atomically_under_handle_reversal() {
        for invert in [false, true] {
            let units = vec![
                unit(if invert { 9 } else { 1 }, 0, Vec2::ZERO),
                unit(if invert { 1 } else { 9 }, 0, Vec2::X),
            ];
            let mut budget = SweepBudget::new(0);
            let mut spent = vec![0; 2];
            let out = run(
                units,
                vec![Vec2::X * 2., Vec2::X * 3.],
                vec![Vec2::ZERO, Vec2::X],
                &mut spent,
                &mut budget,
            );
            assert_eq!(
                (out[0].position, out[1].position),
                (Vec2::X * 2., Vec2::X * 3.)
            );
            assert_eq!(budget.support_accepted_members, 2);
            assert!(spent.iter().all(|&n| n > 0));
        }
    }
    #[test]
    fn failed_stage_keeps_prior_success_and_all_semantic_state() {
        let units = vec![
            unit(1, 0, Vec2::ZERO),
            unit(2, 0, Vec2::new(0., 10.)),
            unit(3, 0, Vec2::new(1., 10.)),
        ];
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut expected = rows(&units, &origins);
        expected[0].position = Vec2::X;
        let mut budget = SweepBudget::new(0);
        let out = run(
            units,
            vec![Vec2::X, Vec2::new(1., 10.), Vec2::new(0., 10.)],
            origins,
            &mut [0; 3],
            &mut budget,
        );
        assert_eq!(out, expected);
        assert_eq!(budget.support_accepted_members, 1);
        assert!(budget.support_failures > 0);
    }
    #[test]
    fn genuine_crossing_rolls_back_and_never_refunds_work() {
        let units = vec![unit(1, 0, Vec2::ZERO), unit(2, 0, Vec2::X)];
        let mut spent = vec![17, 23];
        let mut budget = SweepBudget::new(0);
        let out = run(
            units,
            vec![Vec2::X, Vec2::ZERO],
            vec![Vec2::ZERO, Vec2::X],
            &mut spent,
            &mut budget,
        );
        assert_eq!((out[0].position, out[1].position), (Vec2::ZERO, Vec2::X));
        assert!(spent[0] > 17 && spent[1] > 23);
        assert!(budget.support_failures > 0);
    }
    #[test]
    fn arriving_outsider_keeps_its_certified_trajectory() {
        let a = unit(1, 0, Vec2::ZERO);
        let mut arrival = unit(2, 0, Vec2::new(0.5, 1.));
        arrival.target = Some(Vec2::new(0.5, -1.));
        let mut budget = SweepBudget::new(0);
        let out = run(
            vec![a, arrival],
            vec![Vec2::X, Vec2::new(20., 20.)],
            vec![Vec2::ZERO, Vec2::new(0.5, -1.)],
            &mut [0; 2],
            &mut budget,
        );
        assert_eq!(out[0].position, Vec2::ZERO);
        assert_eq!(out[1].position, Vec2::new(0.5, -1.));
        assert_eq!(budget.support_accepted_members, 0);
    }
    #[test]
    fn foreign_future_never_supplies_support_or_changes_charge() {
        let units = vec![unit(1, 0, Vec2::ZERO), unit(2, 1, Vec2::X)];
        let mut results = Vec::new();
        for foreign in [Vec2::X * 3., Vec2::splat(1000.)] {
            let mut budget = SweepBudget::new(0);
            let out = run(
                units.clone(),
                vec![Vec2::X * 2., foreign],
                vec![Vec2::ZERO, Vec2::X],
                &mut [0; 2],
                &mut budget,
            );
            results.push((out[0].position, budget.bodies[0], budget.pairs[0]));
        }
        assert_eq!(results[0], results[1]);
        assert_eq!(results[0].0, Vec2::ZERO);
    }
    #[test]
    fn exhausted_actor_credit_cannot_be_replenished_by_support() {
        let units = [unit(1, 0, Vec2::ZERO)];
        let mut spent = [bounded::PAIRS_PER_ACTOR, 0];
        let mut budget = SweepBudget::new(0);
        // Relevant settled outsider forces a predicate, even though paths clear.
        let mut wall = unit(2, 1, Vec2::new(0.5, 0.3));
        wall.speed = None;
        wall.target = None;
        let out = run(
            vec![units[0].clone(), wall],
            vec![Vec2::X, Vec2::new(0.5, 0.3)],
            vec![Vec2::ZERO, Vec2::new(0.5, 0.3)],
            &mut spent,
            &mut budget,
        );
        assert_eq!(out[0].position, Vec2::ZERO);
        assert!(budget.support_failures > 0);
        assert_eq!(spent[0], bounded::PAIRS_PER_ACTOR);
        assert_eq!(budget.pairs[0], 0);
    }
}

#[cfg(test)]
mod closure_tests {
    use super::tests::*;
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn safe_three_member_cycle_is_fully_certified_before_publish() {
        let origins = vec![Vec2::new(2., 2.), Vec2::new(4., 2.), Vec2::new(3., 4.)];
        let raw = vec![origins[1], origins[2], origins[0]];
        for i in 0..3 {
            for j in i + 1..3 {
                assert!(pair_clear(origins[i], raw[i], 0.2, origins[j], raw[j], 0.2));
            }
        }
        let units = origins
            .iter()
            .enumerate()
            .map(|(i, &p)| unit(i as u32 + 1, 0, p))
            .collect();
        let mut budget = SweepBudget::new(0);
        let out = run(units, raw.clone(), origins, &mut [0; 3], &mut budget);
        assert_eq!(out.iter().map(|o| o.position).collect::<Vec<_>>(), raw);
        assert_eq!(budget.support_accepted_members, 3);
    }
    #[test]
    fn full_guard_rescues_safe_follower_after_baseline_without_promoting_arrivals() {
        let units = vec![unit(1, 0, Vec2::new(2., 2.)), unit(2, 0, Vec2::new(3., 2.))];
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let raw = vec![Vec2::new(7., 2.), Vec2::new(8., 2.)];
        let mut outputs = rows(&units, &raw);
        let frame = MovementFrame {
            units: units.clone(),
            bounds: Vec2::splat(64.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        let mut points = units
            .iter()
            .map(|u| GridPoint {
                id: u.unit_id,
                position: u.position,
            })
            .collect::<Vec<_>>();
        let mut grid = UniformGrid::new(&points, 0.4);
        let mut budget = SweepBudget::new(0);
        certify(
            &frame,
            &units,
            &origins,
            &mut outputs,
            &mut points,
            &mut grid,
            0.05,
            0.2,
            &mut budget,
        );
        assert_eq!(outputs.iter().map(|o| o.position).collect::<Vec<_>>(), raw);
        assert!(budget.support_accepted_members > 0);
        assert!(budget.pairs.iter().sum::<usize>() <= bounded::WORK);
    }
}

#[cfg(test)]
mod arrival_root_tests {
    use super::tests::*;
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn support_cannot_restore_a_raw_step_beyond_the_authored_speed() {
        let units = [unit(1, 0, Vec2::new(2., 2.))];
        let frame = MovementFrame {
            units: units.to_vec(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        let baseline = Vec2::new(3., 2.);
        let mut outputs = rows(&units, &[baseline]);
        upgrade(
            &frame,
            &units,
            &[units[0].position],
            &[Vec2::new(8., 2.)],
            &mut outputs,
            &mut [0],
            &[true],
            0.05,
            &mut SweepBudget::new(0),
        );
        assert_eq!(outputs[0].position, baseline);
    }
    #[test]
    fn held_short_raw_arrival_retries_after_its_friendly_blocker_advances() {
        for invert in [false, true] {
            let mut units = vec![
                unit(if invert { 2 } else { 1 }, 0, Vec2::new(2., 2.)),
                unit(if invert { 1 } else { 2 }, 0, Vec2::new(2.7, 2.)),
            ];
            units[0].target = Some(Vec2::new(3.5, 2.));
            units[1].target = Some(Vec2::new(100., 2.));
            for u in &mut units {
                u.radius = 0.3;
            }
            let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
            let raw = vec![Vec2::new(3.5, 2.), Vec2::new(7.7, 2.)];
            let mut outputs = rows(&units, &raw);
            outputs[0].target = None;
            let mut frame = MovementFrame {
                units: units.clone(),
                bounds: Vec2::splat(128.),
                tick_hz: 20,
                pools: BTreeMap::new(),
            };
            if invert {
                frame.units.reverse();
            }
            let mut points = units
                .iter()
                .map(|u| GridPoint {
                    id: u.unit_id,
                    position: u.position,
                })
                .collect::<Vec<_>>();
            let mut grid = UniformGrid::new(&points, 0.6);
            let mut budget = SweepBudget::new(0);
            certify(
                &frame,
                &units,
                &origins,
                &mut outputs,
                &mut points,
                &mut grid,
                0.05,
                0.3,
                &mut budget,
            );
            assert_eq!(outputs[0].position, raw[0]);
            assert_eq!(outputs[0].target, None);
            assert_eq!(outputs[1].position, raw[1]);
            assert!(budget.support_accepted_members > 0);
        }
    }
    #[test]
    fn later_root_finds_the_upgraded_arrival_sweep_in_the_union_index() {
        let mut units = vec![
            unit(1, 0, Vec2::new(2., 2.)),
            unit(2, 0, Vec2::new(2.7, 2.)),
            unit(3, 0, Vec2::new(2.75, 0.)),
        ];
        for u in &mut units {
            u.radius = 0.3;
        }
        units[0].target = Some(Vec2::new(3.5, 2.));
        units[1].target = Some(Vec2::new(100., 2.));
        units[2].target = Some(Vec2::new(2.75, 20.));
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let raw = vec![Vec2::new(3.5, 2.), Vec2::new(7.7, 2.), Vec2::new(2.75, 4.)];
        let baseline = vec![Vec2::new(2.1, 2.), raw[1], origins[2]];
        let frame = MovementFrame {
            units: units.clone(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        let mut outputs = rows(&units, &baseline);
        outputs[0].target = None;
        let mut budget = SweepBudget::new(0);
        upgrade(
            &frame,
            &units,
            &origins,
            &raw,
            &mut outputs,
            &mut [0; 3],
            &[true; 3],
            0.05,
            &mut budget,
        );
        assert_eq!(outputs[0].position, raw[0]);
        assert_eq!(
            outputs[2].position, baseline[2],
            "later root must respect the earlier actual arrival"
        );
        assert!(pair_clear(
            origins[0],
            outputs[0].position,
            0.3,
            origins[2],
            outputs[2].position,
            0.3
        ));
        assert!(!pair_clear(
            origins[0], raw[0], 0.3, origins[2], raw[2], 0.3
        ));
    }
}

#[cfg(test)]
mod waypoint_tests {
    use super::tests::{rows, unit};
    use super::*;
    use std::collections::BTreeMap;

    struct Guide {
        waypoint: u32,
        destination: Option<Vec2>,
    }
    impl MotionGuide for Guide {
        fn kind(&self, id: u32) -> MotionKind {
            if id == self.waypoint {
                MotionKind::Waypoint
            } else {
                MotionKind::Destination
            }
        }
        fn arrival_destination(&self, id: u32) -> Option<Vec2> {
            (id == self.waypoint).then_some(self.destination).flatten()
        }
        fn next_position(
            &mut self,
            _: crate::movement::motion::MotionRequest,
            _: &mut SweepBudget,
        ) -> Option<Vec2> {
            None
        }
    }
    fn run(
        units: Vec<MovementFrameUnit>,
        raw: &[Vec2],
        baseline: &[Vec2],
        guide: &Guide,
    ) -> Vec<Vec2> {
        let frame = MovementFrame {
            units: units.clone(),
            bounds: Vec2::splat(64.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        };
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut outputs = rows(&units, baseline);
        upgrade_guided(
            &frame,
            &units,
            &origins,
            raw,
            &mut outputs,
            &mut vec![0; units.len()],
            &vec![true; units.len()],
            0.05,
            &mut SweepBudget::new(0),
            guide,
        );
        outputs.into_iter().map(|o| o.position).collect()
    }
    #[test]
    fn near_private_corner_can_lend_motion_but_actual_arrival_cannot() {
        for invert in [false, true] {
            let root = if invert { 9 } else { 1 };
            let peer = if invert { 1 } else { 9 };
            let mut units = vec![
                unit(root, 0, Vec2::new(2., 2.)),
                unit(peer, 0, Vec2::new(3., 2.)),
            ];
            units[1].target = Some(Vec2::new(4., 2.));
            let raw = [Vec2::new(3., 2.), Vec2::new(4., 2.)];
            let baseline = [units[0].position, units[1].position];
            let private = Guide {
                waypoint: peer,
                destination: Some(Vec2::new(23., 2.)),
            };
            assert_eq!(run(units.clone(), &raw, &baseline, &private), raw);
            let actual = Guide {
                waypoint: u32::MAX,
                destination: None,
            };
            assert_eq!(
                run(units.clone(), &raw, &baseline, &actual),
                [baseline[0], raw[1]]
            );
            for metadata in [None, Some(Vec2::splat(f32::NAN))] {
                let missing = Guide {
                    waypoint: peer,
                    destination: metadata,
                };
                assert_eq!(run(units.clone(), &raw, &baseline, &missing), baseline);
            }
            units[1].progress = Some(crate::movement::MoveProgress {
                pool_id: 1,
                slot: units[1].position,
                activity: MovementActivity::Resting,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 0,
                best_distance: 0.,
                claim_count: 0,
                follow_origin: None,
            });
            assert_eq!(run(units, &raw, &baseline, &private), baseline);
        }
    }
    #[test]
    fn private_corner_does_not_freeze_a_root_but_actual_completion_does() {
        let mut actor = unit(7, 0, Vec2::new(2., 2.));
        actor.target = Some(Vec2::new(4., 2.));
        let baseline = [Vec2::new(4., 2.)];
        let raw = [Vec2::new(5., 2.)];
        let private = Guide {
            waypoint: 7,
            destination: Some(Vec2::new(22., 2.)),
        };
        assert_eq!(run(vec![actor.clone()], &raw, &baseline, &private), raw);
        let actual = Guide {
            waypoint: u32::MAX,
            destination: None,
        };
        assert_eq!(run(vec![actor.clone()], &raw, &baseline, &actual), baseline);
        let missing = Guide {
            waypoint: 7,
            destination: None,
        };
        assert_eq!(run(vec![actor], &raw, &baseline, &missing), baseline);
    }
}
