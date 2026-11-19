//! Claim exchange between two moving members of one pool.
//!
//! A slot claim is a plan, not an arrival. With continuous body certification a mover can no
//! longer cross a peer, so a mover whose slot lies behind resting peers stops in front of them,
//! often on the slot of a peer that is still behind it. That starts a chain jam. A stalled mover
//! may therefore take the nearest free pool slot that it can reach by a clear straight chord and
//! that it is strictly nearer to than the current moving claimant. The claimant receives the
//! slot of the stalled mover in the same step, so the set of claimed slots does not change.
//!
//! Rules:
//! - Both units are in the same pool and have the same owner, type and radius. Both move.
//!   A resting unit never loses its point (physical first arrival).
//! - Every choice is geometric: distance, then angle, then slot coordinates. No unit id.
//! - Only stationary bodies veto a slot or its chord. Final certification still checks all bodies.
//! - The search reads the slots near the mover only, so the work per stalled mover is bounded.

use bevy_math::Vec2;
use std::collections::HashMap;

use super::MovementActivity;
use super::frame::MovementFrameUnit;
use super::pool_index::MovementPoolIndex;
use super::spread::nearest_angle_order;

/// `MoveProgress::claim_count` holds two counters while the frame solver owns pool recovery.
/// The low two bits count the spare slots that the mover claimed after a stall (at most 3, as
/// before). The other bits count the exchanges that restarted its stall clock.
const SPARE_CLAIM_MASK: u8 = 0b11;
const RESTART_UNIT: u8 = 0b100;
/// An exchange restarts the stall clock of a mover at most this many times in one order. After
/// that an exchange still changes its slot, but only certified motion restarts the clock, so a
/// mover that exchanges for ever still reaches the ordinary recovery and then `Blocked`.
const RESTART_BUDGET: u8 = 12;

/// Count one find of an unknown resting body of another owner against the restart budget.
/// The stall clock keeps its value. Returns false when the budget is spent.
pub(super) fn spend_discovery(progress: &mut super::MoveProgress) -> bool {
    if progress.claim_count / RESTART_UNIT >= RESTART_BUDGET {
        return false;
    }
    progress.claim_count += RESTART_UNIT;
    true
}

/// True when the mover has no restart budget left for this order.
pub(super) fn budget_spent(progress: &super::MoveProgress) -> bool {
    progress.claim_count / RESTART_UNIT >= RESTART_BUDGET
}

pub(super) fn spare_claims(progress: &super::MoveProgress) -> u8 {
    progress.claim_count & SPARE_CLAIM_MASK
}

/// Restart the stall clock after an exchange if the mover has budget left. Returns false when
/// the budget is spent; then the clock keeps its value.
pub(super) fn restart_stall_clock(progress: &mut super::MoveProgress) -> bool {
    if progress.claim_count / RESTART_UNIT >= RESTART_BUDGET {
        return false;
    }
    progress.claim_count += RESTART_UNIT;
    progress.pass_mode = false;
    progress.stalling_on_group = false;
    progress.stalled_ticks = 0;
    true
}

/// The slots a mover examines lie within this many pool spacings of its body, not of its slot:
/// a mover that stalls in a crowd far from its own slot stands on the slots of its peers.
const REACH_SPACINGS: f32 = 2.5;

pub(super) type SlotKey = (u64, u32, u32);

pub(super) fn slot_key(pool_id: u64, slot: Vec2) -> SlotKey {
    (pool_id, slot.x.to_bits(), slot.y.to_bits())
}

fn moving_member(unit: &MovementFrameUnit) -> Option<(u64, Vec2)> {
    let progress = unit.progress.as_ref()?;
    (progress.activity == MovementActivity::Moving
        && unit.target == Some(progress.slot)
        && unit.plain_progress.is_none()
        && unit.physical_escape.is_none()
        && unit.speed.is_some_and(|speed| speed > 0.0))
    .then_some((progress.pool_id, progress.slot))
}

/// Each slot that a moving pool member claims, with the row of that member.
pub(super) fn claimants(units: &[MovementFrameUnit]) -> HashMap<SlotKey, usize> {
    let mut rows = HashMap::new();
    let mut shared = Vec::new();
    for (index, unit) in units.iter().enumerate() {
        if let Some((pool_id, slot)) = moving_member(unit) {
            let key = slot_key(pool_id, slot);
            if rows.insert(key, index).is_some() {
                shared.push(key);
            }
        }
    }
    // A slot with two claimants has no single owner to exchange with.
    for key in shared {
        rows.remove(&key);
    }
    rows
}

/// A new slot for a mover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Choice {
    /// The row of the moving claimant that gets the slot of the mover. `None`: no unit claims
    /// the slot.
    pub(super) peer: Option<usize>,
    pub(super) slot: Vec2,
}

/// The state of the mover that decides which exchange rules apply (`select`).
#[derive(Debug, Clone, Copy)]
pub(super) struct Request {
    /// False when a true stationary body of the owner cuts the mover off from its own slot.
    pub(super) own_reachable: bool,
    /// True for a stalled mover that can reach its own slot.
    pub(super) shorter_plan: bool,
    /// True for a mover that is about to arrive.
    pub(super) arriving: bool,
    /// The pool spacing and the pool point.
    pub(super) spacing: f32,
    pub(super) centre: Vec2,
}

/// The slot that the mover takes, if a rule gives one. The rules apply in this order.
///
/// Rule 1 (arrive first; only if `own_reachable` is false: a true stationary body of the owner
/// cuts the mover off from its own slot): the nearest slot that the mover reaches by a clear
/// chord and that it is strictly nearer to than the moving claimant, or that no unit claims.
/// At fixed positions each such step strictly reduces the sum, over the reachable claimed
/// slots, of the distance of the holder, so the rule cannot cycle.
///
/// Rule 3 (fill from the inside; only for a mover that is about to arrive, `arriving`): a
/// free slot near the mover that no body of the owner covers and that the mover is nearer to
/// than the moving claimant, in one of two cases. 3a: the slot is beside the own slot and the
/// arrival would cut it off from open ground (`seals`). 3b: the slot lies nearer to the pool
/// point `centre` than the own slot, and the exchange reduces the sum of the two squared slot
/// distances by the margin of rule 2. The mover goes in, and the later body gets the outer slot.
/// 3b never takes a slot from which the mover would seal the slot that it leaves: that is the
/// reverse of 3a, and the two would exchange the same pair in every tick.
///
/// Rule 2 (shorter plan; only if `shorter_plan`: the caller sets it for a stalled mover that
/// can reach its own slot): the exchange with a moving peer that most reduces the sum of the
/// two squared slot distances, by a fixed margin. Movers that press on each other across their
/// slots get out of each other's way this way, and a mover that plugs a crowd far from its
/// slot hands that slot from peer to peer toward the body that is nearest to it.
///
/// Each rule alone strictly reduces its own measure at fixed positions. No single measure
/// covers all three, so the acceptance cases check that every group ends.
pub(super) fn select(
    planner: &super::route::Planner,
    request: Request,
    pool_index: &MovementPoolIndex,
    claimants: &HashMap<SlotKey, usize>,
    assigned: &std::collections::HashSet<SlotKey>,
) -> Option<Choice> {
    let Request {
        own_reachable,
        shorter_plan,
        arriving,
        spacing,
        centre,
    } = request;
    let units = planner.units;
    let index = planner.index;
    let unit = &units[index];
    let (pool_id, own_slot) = moving_member(unit)?;
    let own_distance = unit.position.distance(own_slot);
    let geometric = |chosen: Vec2, slot: Vec2| {
        nearest_angle_order(chosen, slot, unit.position)
            .then_with(|| chosen.x.total_cmp(&slot.x))
            .then_with(|| chosen.y.total_cmp(&slot.y))
    };
    let hard = planner.hard();
    let mut first: Option<(Option<usize>, Vec2)> = None;
    let mut shorter: Option<(f32, usize, Vec2)> = None;
    let mut inner: Option<(Option<usize>, Vec2)> = None;
    for slot in pool_index.slots_within(unit.position, REACH_SPACINGS) {
        if slot == own_slot {
            continue;
        }
        let key = slot_key(pool_id, slot);
        let distance = unit.position.distance(slot);
        let peer_index = match claimants.get(&key) {
            Some(&peer_index) => {
                let peer = &units[peer_index];
                if peer_index == index
                    || moving_member(peer) != Some((pool_id, slot))
                    || peer.owner_slot != unit.owner_slot
                    || peer.unit_type != unit.unit_type
                    || peer.radius != unit.radius
                {
                    continue;
                }
                Some(peer_index)
            }
            // A claim with no eligible moving claimant belongs to a resting or other unit.
            None if assigned.contains(&key) => continue,
            None => None,
        };
        let peer_distance = peer_index.map(|peer| units[peer].position.distance(slot));
        if !own_reachable && peer_distance.is_none_or(|peer_distance| distance < peer_distance) {
            if first.is_none_or(|(_, chosen)| geometric(chosen, slot).is_gt())
                && planner.chord_clear(unit.position, slot)
            {
                first = Some((peer_index, slot));
            }
            continue;
        }
        // The peer must be able to start toward the slot that it gets. Otherwise rule 1 would
        // take the exchange back in the next tick.
        let peer_can_start = |peer: usize| {
            let peer_planner = super::route::Planner {
                index: peer,
                ..hard
            };
            let from = units[peer].position;
            let ahead = from + (own_slot - from).clamp_length_max(super::route::LOOK_AHEAD);
            peer_planner.chord_clear(from, ahead)
        };
        // Rules 2 and 3b share one measure: the sum of the two squared slot distances. An
        // unclaimed slot counts as a peer at no distance from either slot. The margin keeps
        // rounding from admitting an exchange with no real gain.
        let gain = match (peer_index, peer_distance) {
            (Some(peer), Some(peer_distance)) => {
                let peer_after = units[peer].position.distance(own_slot);
                own_distance * own_distance + peer_distance * peer_distance
                    - distance * distance
                    - peer_after * peer_after
            }
            _ => own_distance * own_distance - distance * distance,
        };
        let shorter_step = gain > 1.0e-2;
        if arriving && peer_distance.is_none_or(|peer_distance| distance < peer_distance) {
            let beside = (slot.distance(own_slot) - spacing).abs() <= spacing * 0.05;
            let deeper = distance <= spacing * 1.5
                && slot.distance(centre) < own_slot.distance(centre) - spacing * 0.25;
            // Rule 3a decides between a slot and the pocket behind it. Rule 3b must not
            // reverse that decision, or the mover steps between the two slots for ever.
            if ((beside && seals(&hard, slot, own_slot, spacing))
                || (deeper && shorter_step && !seals(&hard, own_slot, slot, spacing)))
                && inner.is_none_or(|(_, chosen)| geometric(slot, chosen).is_lt())
                && planner.uncovered(slot)
                && planner.chord_clear(unit.position, slot)
            {
                inner = Some((peer_index, slot));
                continue;
            }
        }
        let Some(peer) = peer_index else {
            continue;
        };
        if !shorter_step {
            continue;
        }
        if shorter_plan
            && shorter.is_none_or(|(best_gain, _, chosen)| {
                gain.total_cmp(&best_gain)
                    .then_with(|| geometric(chosen, slot))
                    .is_gt()
            })
            && hard.chord_clear(unit.position, slot)
            && peer_can_start(peer)
            // Rule 3 decides between a slot and the pocket behind it. Rule 2 must not
            // reverse that decision, or the two rules exchange the same pair for ever.
            && !seals(&hard, slot, own_slot, spacing)
            && !seals(&hard, own_slot, slot, spacing)
        {
            shorter = Some((gain, peer, slot));
        }
    }
    first
        .or(inner)
        .map(|(peer, slot)| Choice { peer, slot })
        .or(shorter.map(|(_, peer, slot)| Choice {
            peer: Some(peer),
            slot,
        }))
}

/// A pocket of more open sites than this counts as open ground.
const POCKET_SITES: usize = 24;

/// True when a body that rests on `own_slot` cuts the neighbour slot `slot` off from open
/// ground, and the body can still step from `own_slot` into `slot`.
///
/// The sites are the points of the hex lattice of the pool, in the pool and round it. A site
/// is closed when a true stationary body of the owner fills it (`Planner::site_closed`). The
/// gap between two resting neighbours is narrower than a body, so a body steps between two
/// adjacent open sites only if one of the two sites beside that step is open as well. The
/// search floods the open sites from `slot` with `own_slot` closed. A flood that ends within
/// `POCKET_SITES` sites is a sealed pocket.
fn seals(hard: &super::route::Planner, slot: Vec2, own_slot: Vec2, spacing: f32) -> bool {
    // Exact lattice offsets in angular order: no trigonometric rounding decides a gate.
    const SIDES: [(i32, i32); 6] = [(1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)];
    let position = |(i, j): (i32, i32)| {
        Vec2::new(
            slot.x + spacing * (i as f32 + 0.5 * j as f32),
            slot.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
        )
    };
    let own = SIDES
        .iter()
        .position(|&side| position(side).distance(own_slot) <= spacing * 0.05);
    let Some(own) = own else {
        return false;
    };
    let mut known: HashMap<(i32, i32), bool> = HashMap::new();
    let mut open = |site: (i32, i32), own_open: bool| {
        if site == SIDES[own] {
            return own_open;
        }
        *known
            .entry(site)
            .or_insert_with(|| !hard.site_closed(position(site), spacing))
    };
    // The body must be able to step from its own slot into the pocket.
    let beside = |side: usize| [SIDES[(side + 5) % 6], SIDES[(side + 1) % 6]];
    if !beside(own).into_iter().any(|site| open(site, true)) {
        return false;
    }
    // True when the flood from `slot` ends inside a pocket.
    let mut pocket = |own_open: bool| {
        let mut seen = std::collections::HashSet::from([(0, 0)]);
        let mut queue = std::collections::VecDeque::from([(0, 0)]);
        while let Some(site) = queue.pop_front() {
            for side in 0..6 {
                let step = |side: usize| (site.0 + SIDES[side].0, site.1 + SIDES[side].1);
                let next = step(side);
                if seen.contains(&next)
                    || !open(next, own_open)
                    || !(open(step((side + 5) % 6), own_open)
                        || open(step((side + 1) % 6), own_open))
                {
                    continue;
                }
                seen.insert(next);
                if seen.len() > POCKET_SITES {
                    return false;
                }
                queue.push_back(next);
            }
        }
        true
    };
    // The arrival must be the cause: the slot reaches open ground now, and not afterwards.
    !pocket(true) && pocket(false)
}

/// The slots that a flood search examines lie within this many pool spacings of the mover.
const FLOOD_SPACINGS: f32 = 12.0;

/// A route for a mover whose straight chord is blocked by a stationary body of its owner.
pub(super) struct Plan {
    /// The next point to steer at.
    pub(super) waypoint: Vec2,
    /// A new slot for the mover when it cannot reach its own slot at all.
    pub(super) claim: Option<Choice>,
}

/// A route to the mover's own slot, if the bounded search finds one.
pub(super) fn route_own(
    planner: &super::route::Planner,
    spent: &mut usize,
    exhausted: &mut bool,
) -> Option<Vec2> {
    let (_, own_slot) = moving_member(&planner.units[planner.index])?;
    let cap = super::route::OWNER_CAP.min(*spent + super::route::NODE_CAP);
    planner
        .search(&[own_slot], true, None, cap, spent, exhausted)
        .first()
        .map(|reached| reached.waypoint)
}

/// A route to the cheapest other slot that the mover can take: an unclaimed slot, or the slot
/// of a moving peer whose straight distance to it is longer than the route of this mover.
/// `waits` becomes true when the search reaches a slot that a nearer moving peer still claims:
/// then the mover has no slot now, but the answer can change when that peer arrives.
#[expect(
    clippy::too_many_arguments,
    reason = "Route selection shares the existing planner, occupancy, and budget inputs"
)]
pub(super) fn route_other(
    planner: &super::route::Planner,
    pool_index: &MovementPoolIndex,
    claimants: &HashMap<SlotKey, usize>,
    assigned: &std::collections::HashSet<SlotKey>,
    spent: &mut usize,
    exhausted: &mut bool,
    waits: &mut bool,
    independent: bool,
) -> Option<Plan> {
    // The preceding search did not finish, so no alternate-slot decision is safe.
    if *exhausted {
        return None;
    }
    let units = planner.units;
    let unit = &units[planner.index];
    let (pool_id, own_slot) = moving_member(unit)?;
    let mut goals = Vec::new();
    let mut peers = Vec::new();
    let mut limits = Vec::new();
    for slot in pool_index.slots_within(unit.position, FLOOD_SPACINGS) {
        if slot == own_slot {
            continue;
        }
        let key = slot_key(pool_id, slot);
        let peer_index = match claimants.get(&key) {
            Some(&peer_index) => {
                let peer = &units[peer_index];
                if peer_index == planner.index
                    || moving_member(peer) != Some((pool_id, slot))
                    || peer.owner_slot != unit.owner_slot
                    || peer.unit_type != unit.unit_type
                    || peer.radius != unit.radius
                {
                    continue;
                }
                Some(peer_index)
            }
            None if assigned.contains(&key) => continue,
            None => None,
        };
        let limit = peer_index.map_or(f32::INFINITY, |peer| units[peer].position.distance(slot));
        goals.push(slot);
        peers.push(peer_index);
        limits.push(limit);
    }
    if goals.is_empty() {
        return None;
    }
    let cap = super::route::OWNER_CAP.min(*spent + super::route::FLOOD_CAP);
    let reached = planner.search(
        &goals,
        false,
        independent.then_some(limits.as_slice()),
        cap,
        spent,
        exhausted,
    );
    *waits = !reached.is_empty();
    reached
        .into_iter()
        .filter(|reached| reached.cost < limits[reached.goal])
        .min_by(|a, b| {
            a.cost
                .total_cmp(&b.cost)
                .then_with(|| nearest_angle_order(goals[a.goal], goals[b.goal], unit.position))
                .then_with(|| goals[a.goal].x.total_cmp(&goals[b.goal].x))
                .then_with(|| goals[a.goal].y.total_cmp(&goals[b.goal].y))
        })
        .map(|reached| Plan {
            waypoint: reached.waypoint,
            claim: Some(Choice {
                peer: peers[reached.goal],
                slot: goals[reached.goal],
            }),
        })
}

#[cfg(test)]
mod exhaustion_tests {
    use super::*;
    use crate::movement::{MoveProgress, MovementPool};
    use std::collections::HashSet;

    #[test]
    fn exhausted_route_never_scans_alternate_slots() {
        let position = Vec2::new(5.0, 5.0);
        let slot = Vec2::new(9.0, 5.0);
        let units = [MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(1.0),
            target: Some(slot),
            progress: Some(MoveProgress {
                pool_id: 7,
                slot,
                activity: MovementActivity::Moving,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 0,
                best_distance: 4.0,
                claim_count: 0,
                follow_origin: None,
            }),
            plain_progress: None,
            physical_escape: None,
        }];
        let points = [crate::grid::GridPoint { id: 1, position }];
        let grid = crate::grid::UniformGrid::new(&points, 1.0);
        let planner = super::super::route::Planner {
            index: 0,
            units: &units,
            points: &points,
            grid: &grid,
            max_radius: 0.5,
            bounds: Vec2::splat(20.0),
            stationary_at_start: &[false],
            contact_witness: &[vec![]],
            start_stall: &[None],
            soft: false,
            stall_ticks: 2,
        };
        let pool = MovementPool {
            pool_id: 7,
            point: slot,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![slot, Vec2::new(6.0, 5.0)],
        };
        let index = MovementPoolIndex::new(&pool, Vec2::splat(20.0));
        let mut spent = super::super::route::OWNER_CAP;
        let mut exhausted = true;
        let mut waits = false;
        let before = super::super::pool_index::slot_range_queries();
        let plan = route_other(
            &planner,
            &index,
            &HashMap::new(),
            &HashSet::new(),
            &mut spent,
            &mut exhausted,
            &mut waits,
            false,
        );
        assert!(plan.is_none());
        assert!(exhausted && !waits);
        assert_eq!(
            super::super::pool_index::slot_range_queries(),
            before,
            "an exhausted search must not enumerate alternate pool slots"
        );
    }

    #[test]
    fn exhausted_own_route_can_take_a_proven_free_slot_without_displacing_resting_units() {
        for reverse in [false, true] {
            let start = Vec2::new(5.0, 5.0);
            let own = Vec2::new(9.0, 5.0);
            let free = Vec2::new(6.0, 5.0);
            let occupied = Vec2::new(7.0, 5.0);
            let mut units = vec![
                MovementFrameUnit {
                    unit_id: if reverse { 99 } else { 1 },
                    owner_slot: 0,
                    unit_type: 1,
                    position: start,
                    radius: 0.5,
                    speed: Some(1.0),
                    target: Some(own),
                    progress: Some(MoveProgress {
                        pool_id: 7,
                        slot: own,
                        activity: MovementActivity::Moving,
                        pass_mode: false,
                        stalling_on_group: false,
                        stalled_ticks: 0,
                        best_distance: 4.0,
                        claim_count: 0,
                        follow_origin: None,
                    }),
                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: if reverse { 1 } else { 99 },
                    owner_slot: 0,
                    unit_type: 1,
                    position: own,
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 3,
                    owner_slot: 0,
                    unit_type: 1,
                    position: occupied,
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
            ];
            if reverse {
                units.reverse();
            }
            let mover = units.iter().position(|u| u.target.is_some()).unwrap();
            let points: Vec<_> = units
                .iter()
                .map(|u| crate::grid::GridPoint {
                    id: u.unit_id,
                    position: u.position,
                })
                .collect();
            let grid = crate::grid::UniformGrid::new(&points, 1.0);
            let stationary: Vec<_> = units.iter().map(|u| u.target.is_none()).collect();
            let witnesses = vec![vec![]; units.len()];
            let stalls = vec![None; units.len()];
            let planner = super::super::route::Planner {
                index: mover,
                units: &units,
                points: &points,
                grid: &grid,
                max_radius: 0.5,
                bounds: Vec2::splat(20.0),
                stationary_at_start: &stationary,
                contact_witness: &witnesses,
                start_stall: &stalls,
                soft: false,
                stall_ticks: 2,
            };
            let pool = MovementPool {
                pool_id: 7,
                point: own,
                spacing: 1.1,
                primary_count: 1,
                slots: vec![own, occupied, free],
            };
            let index = MovementPoolIndex::new(&pool, Vec2::splat(20.0));
            let assigned = HashSet::from([slot_key(7, own), slot_key(7, occupied)]);
            let mut spent = 0;
            let mut own_exhausted = false;
            assert!(route_own(&planner, &mut spent, &mut own_exhausted).is_none());
            // A resting body stands on the own slot. The forward search runs to its cap,
            // and the empty region round the slot then proves that no route exists.
            assert!(!own_exhausted, "a covered own slot is a proven dead end");
            assert!(
                spent >= super::super::route::NODE_CAP,
                "the forward search ran"
            );
            assert!(spent < super::super::route::OWNER_CAP);
            let mut alternate_exhausted = false;
            let mut waits = false;
            let plan = route_other(
                &planner,
                &index,
                &HashMap::new(),
                &assigned,
                &mut spent,
                &mut alternate_exhausted,
                &mut waits,
                true,
            )
            .expect("free route");
            assert!(!alternate_exhausted);
            assert_eq!(
                plan.claim,
                Some(Choice {
                    peer: None,
                    slot: free
                })
            );
            assert_eq!(
                units
                    .iter()
                    .find(|u| u.position == occupied)
                    .unwrap()
                    .target,
                None
            );
            let mut spent = super::super::route::OWNER_CAP - 1;
            let mut exhausted = false;
            let mut waits = false;
            let plan = route_other(
                &planner,
                &index,
                &HashMap::new(),
                &assigned,
                &mut spent,
                &mut exhausted,
                &mut waits,
                true,
            );
            assert!(exhausted);
            assert!(
                plan.is_none(),
                "an unfinished alternate search cannot claim a partial result"
            );
        }
    }
}
