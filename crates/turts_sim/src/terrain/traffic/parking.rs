//! Positive own-body enclosure certificates for private group destinations.
//! A certificate changes private arrival intent only. Holders and public orders stay fixed.
use super::*;
use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};
// These inline buffers and one complete chord result reuse the controller's
// existing 4096-byte helper allowance. No second index survives this phase.
const _: () = assert!(
    NEIGHBORS * std::mem::size_of::<detour::Blocker>()
        + 2 * NEIGHBORS * std::mem::size_of::<Vec2>()
        + 20 * std::mem::size_of::<Option<detour::Blocker>>()
        <= 4096
);
const _: () = assert!(
    NEIGHBORS
        * (std::mem::size_of::<detour::Blocker>()
            + std::mem::size_of::<Vec2>()
            + std::mem::size_of::<usize>())
        <= 4096
);

/// Destination authority from the real accepted own cohort. Numeric bounds use
/// only its public point, count and authored radii, never obstacle-shifted slots.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DestinationRegion {
    pub(crate) goal: Vec2,
    pub(crate) min: Vec2,
    pub(crate) max: Vec2,
    pub(crate) cohort_radius: f32,
}
impl DestinationRegion {
    pub(crate) fn from_dispatch(
        pool: &crate::movement::MovementPool,
        members: &[crate::movement::SpreadMember],
    ) -> Option<Self> {
        if members.is_empty()
            || members.len() > MEMBERS
            || !pool.point.is_finite()
            || pool.primary_count as usize != members.len()
        {
            return None;
        }
        let mut radius = 0.0f32;
        for member in members {
            if !member.radius.is_finite() || member.radius <= 0. {
                return None;
            }
            radius = radius.max(member.radius);
        }
        let spacing = 2. * radius + 0.1;
        if !spacing.is_finite() || pool.spacing != spacing {
            return None;
        }
        let mut ring = 0usize;
        while 1 + 3 * ring * (ring + 1) < members.len() {
            ring += 1;
        }
        // Same coordinate arithmetic as spread placement, rounded outward.
        let extent = Vec2::new(
            spacing * ring as f32,
            spacing * (3.0_f32.sqrt() * 0.5 * ring as f32),
        );
        let low = pool.point - extent;
        let high = pool.point + extent;
        let min = Vec2::new(low.x.next_down(), low.y.next_down());
        let max = Vec2::new(high.x.next_up(), high.y.next_up());
        (min.is_finite() && max.is_finite()).then_some(Self {
            goal: pool.point,
            min,
            max,
            cohort_radius: radius,
        })
    }
    pub(crate) fn contains(self, point: Vec2, radius: f32) -> bool {
        let layer = f64::from(self.cohort_radius) + f64::from(radius);
        point.is_finite()
            && radius.is_finite()
            && radius > 0.
            && f64::from(point.x) >= f64::from(self.min.x) - layer
            && f64::from(point.y) >= f64::from(self.min.y) - layer
            && f64::from(point.x) <= f64::from(self.max.x) + layer
            && f64::from(point.y) <= f64::from(self.max.y) + layer
    }
}
type Blocker = detour::Blocker;
fn point(p: Vec2) -> Point {
    Point {
        x: f64::from(p.x),
        y: f64::from(p.y),
    }
}
fn disk_clear(from: Vec2, to: Vec2, radius: f64, p: Vec2, other: f64) -> Option<bool> {
    swept_disks_clear(
        point(from),
        point(to),
        radius,
        point(p),
        point(p),
        other,
        &mut Budget::new(1),
    )
    .ok()
}
// Exact dyadic conversion. Unsupported scales or arithmetic refuse the certificate.
// The supported range is deliberately smaller than i128's multiplication range.
fn exact(v: f32) -> Option<i128> {
    let value = f64::from(v) * 4_294_967_296.;
    (value.is_finite() && value.abs() < (1u64 << 55) as f64 && value.fract() == 0.)
        .then_some(value as i128)
}
fn orient(a: Vec2, b: Vec2, p: Vec2) -> Option<i128> {
    let dx = exact(b.x)?.checked_sub(exact(a.x)?)?;
    let dy = exact(b.y)?.checked_sub(exact(a.y)?)?;
    dx.checked_mul(exact(p.y)?.checked_sub(exact(a.y)?)?)?
        .checked_sub(dy.checked_mul(exact(p.x)?.checked_sub(exact(a.x)?)?)?)
}
fn crossing(a: Vec2, b: Vec2, p: Vec2) -> Option<bool> {
    let cross = orient(a, b, p)?;
    if cross == 0
        && p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.y >= a.y.min(b.y)
        && p.y <= a.y.max(b.y)
    {
        return None;
    }
    Some((a.y > p.y) != (b.y > p.y) && (cross > 0) == (b.y > a.y))
}
fn root(parent: &[u8; NEIGHBORS], parity: &[bool; NEIGHBORS], mut i: usize) -> (usize, bool) {
    let mut value = false;
    while usize::from(parent[i]) != i {
        value ^= parity[i];
        i = usize::from(parent[i]);
    }
    (i, value)
}
/// Each overlap edge lies wholly inside the open union of inflated holder discs.
/// An inconsistent XOR cycle has different winding parity at the two endpoints.
/// Thus every continuous route between those endpoints crosses forbidden body space.
fn enclosed_with_settled(
    from: Vec2,
    target: Vec2,
    radius: f32,
    blockers: &[Blocker],
    settled: u128,
    charge: &mut impl FnMut() -> bool,
) -> Option<bool> {
    if blockers.len() > NEIGHBORS {
        return None;
    }
    let mut parent = std::array::from_fn(|i| i as u8);
    let mut parity = [false; NEIGHBORS];
    for (i, a) in blockers.iter().enumerate() {
        if settled & (1u128 << i) == 0 {
            continue;
        }
        for (j, b) in blockers[..i].iter().enumerate() {
            if settled & (1u128 << j) == 0 {
                continue;
            }
            if !charge() {
                return None;
            }
            // Sum in f64. Rounding an expanded radius up in f32 could invent a wall.
            if disk_clear(
                a.position,
                a.position,
                f64::from(radius) + f64::from(a.radius),
                b.position,
                f64::from(radius) + f64::from(b.radius),
            )? {
                continue;
            }
            let edge =
                crossing(a.position, b.position, from)? ^ crossing(a.position, b.position, target)?;
            let (ar, ap) = root(&parent, &parity, i);
            let (br, bp) = root(&parent, &parity, j);
            if ar == br {
                if ap ^ bp != edge {
                    return Some(true);
                }
            } else {
                parent[ar] = br as u8;
                parity[ar] = ap ^ bp ^ edge;
            }
        }
    }
    Some(false)
}
#[cfg(test)]
fn enclosed(
    from: Vec2,
    target: Vec2,
    radius: f32,
    blockers: &[Blocker],
    charge: &mut impl FnMut() -> bool,
) -> Option<bool> {
    enclosed_with_settled(from, target, radius, blockers, u128::MAX, charge)
}

fn distance(p: Vec2, goal: Vec2) -> f64 {
    (p.as_dvec2() - goal.as_dvec2()).length_squared()
}
fn candidate_order(a: Vec2, b: Vec2, goal: Vec2) -> Ordering {
    distance(a, goal)
        .total_cmp(&distance(b, goal))
        .then(a.x.total_cmp(&b.x))
        .then(a.y.total_cmp(&b.y))
}
/// Finite geometry candidates use four public-goal and four local boundaries.
/// Local intersections include adjacent representable points. Both rounded and outward points
/// receive exact guards. No actor-relative incremental target can drift forever.
fn candidates(
    radius: f32,
    goal: Vec2,
    from: Vec2,
    blockers: &[Blocker],
    charge: &mut impl FnMut() -> bool,
) -> Option<([Vec2; NEIGHBORS], usize)> {
    let mut generators = [None; 8];
    let mut nearby = [None; 4];
    let order = |a: Blocker, b: Blocker| {
        let gap = |a: Blocker| {
            (a.position.as_dvec2() - goal.as_dvec2()).length()
                - f64::from(a.radius)
                - f64::from(radius)
        };
        gap(a)
            .abs()
            .total_cmp(&gap(b).abs())
            .then(a.position.x.total_cmp(&b.position.x))
            .then(a.position.y.total_cmp(&b.position.y))
            .then(a.radius.total_cmp(&b.radius))
    };
    for &body in blockers {
        if !charge() {
            return None;
        }
        for i in 0..generators.len() {
            if generators[i].is_none_or(|g| order(body, g).is_lt()) {
                for j in (i + 1..generators.len()).rev() {
                    generators[j] = generators[j - 1];
                }
                generators[i] = Some(body);
                break;
            }
        }
    }
    // Keep the same eight generators. Local contact boundaries can supply the
    // reachable exterior component even when public-goal boundaries cannot.
    let local_order = |a: Blocker, b: Blocker| {
        let gap = |a: Blocker| {
            ((a.position.as_dvec2() - from.as_dvec2()).length()
                - f64::from(a.radius)
                - f64::from(radius))
            .abs()
        };
        gap(a)
            .total_cmp(&gap(b))
            .then(a.position.x.total_cmp(&b.position.x))
            .then(a.position.y.total_cmp(&b.position.y))
            .then(a.radius.total_cmp(&b.radius))
    };
    for &body in blockers {
        if !charge() {
            return None;
        }
        for i in 0..nearby.len() {
            if nearby[i].is_none_or(|g| local_order(body, g).is_lt()) {
                for j in (i + 1..nearby.len()).rev() {
                    nearby[j] = nearby[j - 1];
                }
                nearby[i] = Some(body);
                break;
            }
        }
    }
    let public = generators;
    generators = [None; 8];
    let mut used = 0;
    for body in public[..4]
        .iter()
        .chain(nearby.iter())
        .chain(public[4..].iter())
        .flatten()
        .copied()
    {
        if generators[..used].contains(&Some(body)) {
            continue;
        }
        if used == generators.len() {
            break;
        }
        generators[used] = Some(body);
        used += 1;
    }
    let mut points = [Vec2::ZERO; NEIGHBORS];
    let mut count = 0;
    let mut add = |value: Vec2| -> Option<()> {
        if !charge() || !value.is_finite() || count == NEIGHBORS {
            return None;
        }
        points[count] = value;
        count += 1;
        Some(())
    };
    // Eight stable radial samples plus twelve local intersections with ten
    // representable samples each fit the unchanged128-candidate allowance.
    for body in generators.iter().flatten() {
        let centre = body.position.as_dvec2();
        let r = f64::from(radius) + f64::from(body.radius);
        let direction = (goal.as_dvec2() - centre).normalize_or_zero();
        if direction == bevy_math::DVec2::ZERO {
            continue;
        }
        let p = centre + direction * r;
        let scale = (f64::from(radius) / 64.)
            .max(8. * f64::from(f32::EPSILON) * p.abs().max_element().max(1.));
        add((p + direction * scale).as_vec2())?;
    }
    for (i, body) in nearby.iter().flatten().enumerate() {
        let centre = body.position.as_dvec2();
        let r = f64::from(radius) + f64::from(body.radius);
        for other in nearby[..i].iter().flatten() {
            let delta = other.position.as_dvec2() - centre;
            let d = delta.length();
            let other_r = f64::from(radius) + f64::from(other.radius);
            if d == 0. || d > r + other_r || d < (r - other_r).abs() {
                continue;
            }
            let along = (r * r - other_r * other_r + d * d) / (2. * d);
            let height = (r * r - along * along).max(0.).sqrt();
            let axis = delta / d;
            let middle = centre + axis * along;
            let perpendicular = bevy_math::DVec2::new(-axis.y, axis.x) * height;
            for p in [middle + perpendicular, middle - perpendicular] {
                let rounded = p.as_vec2();
                for x in [rounded.x.next_down(), rounded.x, rounded.x.next_up()] {
                    for y in [rounded.y.next_down(), rounded.y, rounded.y.next_up()] {
                        add(Vec2::new(x, y))?;
                    }
                }
                let outward = (p - centre) / r + (p - other.position.as_dvec2()) / other_r;
                let scale = (f64::from(radius) / 64.)
                    .max(8. * f64::from(f32::EPSILON) * p.abs().max_element().max(1.));
                add((p + outward.normalize_or_zero() * scale).as_vec2())?;
            }
        }
    }
    points[..count].sort_by(|a, b| candidate_order(*a, *b, goal));
    Some((points, count))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn apply(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    state: &mut TrafficState,
    service: &mut [usize],
    tick: u32,
    waiting: &BTreeSet<u32>,
    spatial: &mut Spatial,
    visits: &mut [usize; SLOTS],
    quota: usize,
    public_goal: &mut impl FnMut(usize) -> Option<DestinationRegion>,
    settled: &mut impl FnMut(usize) -> bool,
    eligible: &mut impl FnMut(usize) -> bool,
    clear: &mut impl FnMut(usize, Vec2, Vec2) -> bool,
    budget: &mut crate::movement::sweep::SweepBudget,
) {
    service.sort_by(|&a, &b| {
        let age = |i: usize| {
            let member = &state.members[&frame.units[i].unit_id];
            (
                member
                    .attempts
                    .get(Attempt::Parking)
                    .unwrap_or(member.accepted_tick),
                member.attempts.get(Attempt::Parking).is_some(),
            )
        };
        age(a)
            .cmp(&age(b))
            .then_with(|| physical(&frame.units[a], &frame.units[b]))
    });
    for &index in service.iter() {
        let actor = &frame.units[index];
        let previous = state
            .members
            .get_mut(&actor.unit_id)
            .unwrap()
            .parking
            .take();
        if !state.members[&actor.unit_id].parking_retained && !eligible(index) {
            continue;
        }
        if waiting.contains(&actor.unit_id) {
            continue;
        }
        let Some(region) = public_goal(index) else {
            continue;
        };
        let goal = region.goal;
        if !goal.is_finite() {
            continue;
        }
        let Some(target) = actor.target.filter(|t| *t != goal) else {
            continue;
        };
        let owner = usize::from(actor.owner_slot);
        if visits[owner] >= quota {
            continue;
        }
        let member = state.members.get_mut(&actor.unit_id).unwrap();
        let mut charge = || {
            let allowed =
                budget.charge_guidance_pair_compact(actor.owner_slot, &mut member.guidance_pairs);
            if allowed {
                member.attempts.set(Attempt::Parking, Some(tick));
            }
            allowed
        };
        if !charge() {
            continue;
        }
        let mut terrain_calls = 1;
        // This body-only escape cannot authorize arrival across an impassable
        // terrain passage. The entire original private approach must be permitted.
        if !clear(index, actor.position, target) {
            continue;
        }
        let Some(mut rows) =
            spatial.destination_region(frame, index, target, region, &mut visits[owner], quota)
        else {
            continue;
        };
        let mut blockers = [Blocker {
            position: Vec2::ZERO,
            radius: 1.,
        }; NEIGHBORS];
        rows.sort_by(|&a, &b| physical(&frame.units[a], &frame.units[b]));
        let mut count = 0;
        let mut settled_mask = 0u128;
        let mut complete = true;
        for other in rows {
            if !charge() {
                complete = false;
                break;
            }
            if other != index {
                if settled(other) {
                    settled_mask |= 1u128 << count;
                }
                let peer = &frame.units[other];
                blockers[count] = Blocker {
                    position: peer.position,
                    radius: peer.radius,
                };
                count += 1;
            }
        }
        if !complete {
            continue;
        }
        let blockers = &mut blockers[..count];
        let mut occupied = false;
        for (i, body) in blockers.iter().enumerate() {
            if !charge() {
                complete = false;
                break;
            }
            if settled_mask & (1u128 << i) == 0 {
                continue;
            }
            match disk_clear(
                target,
                target,
                f64::from(actor.radius),
                body.position,
                f64::from(body.radius),
            ) {
                Some(false) => {
                    occupied = true;
                    break;
                }
                Some(true) => {}
                None => {
                    complete = false;
                    break;
                }
            }
        }
        if !complete
            || (!occupied
                && enclosed_with_settled(
                    actor.position,
                    target,
                    actor.radius,
                    blockers,
                    settled_mask,
                    &mut charge,
                ) != Some(true))
        {
            continue;
        }
        let Some((mut candidates, mut count)) =
            candidates(actor.radius, goal, actor.position, blockers, &mut charge)
        else {
            continue;
        };
        if let Some(previous) = previous {
            if !charge() {
                continue;
            }
            if !candidates[..count].contains(&previous) {
                let slot = count.min(NEIGHBORS - 1);
                candidates[slot] = previous;
                count = (count + 1).min(NEIGHBORS);
                candidates[..count].sort_by(|a, b| candidate_order(*a, *b, goal));
            }
        }
        let mut selected = None;
        for &candidate in &candidates[..count] {
            if !region.contains(candidate, actor.radius)
                || candidate.x < actor.radius
                || candidate.y < actor.radius
                || candidate.x > frame.bounds.x - actor.radius
                || candidate.y > frame.bounds.y - actor.radius
            {
                continue;
            }
            let Some(mut rows) = spatial.chord(frame, index, candidate, &mut visits[owner], quota)
            else {
                break;
            };
            rows.sort_by(|&a, &b| physical(&frame.units[a], &frame.units[b]));
            let mut safe = true;
            for other in rows {
                if !charge() {
                    complete = false;
                    break;
                }
                if other == index {
                    continue;
                }
                let peer = &frame.units[other];
                if disk_clear(
                    actor.position,
                    candidate,
                    f64::from(actor.radius),
                    peer.position,
                    f64::from(peer.radius),
                ) != Some(true)
                {
                    safe = false;
                    break;
                }
            }
            if !complete {
                break;
            }
            if safe {
                if terrain_calls == detour::MAX_TERRAIN_CALLS {
                    break;
                }
                terrain_calls += 1;
                if clear(index, actor.position, candidate) {
                    selected = Some(candidate);
                    break;
                }
            }
        }
        if complete && let Some(slot) = selected {
            proposed.units[index].target = Some(slot);
            proposed.units[index].speed = actor.speed;
            member.parking = Some(slot);
            member.parking_retained = false;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
mod tests {
    use super::*;
    const BODIES: [(u32, f32, f32, f32, bool); 36] = [
        (1, 46.54999923706055, 18.024425506591797, 0.4, true), // small.0.0
        (2, 47.70000076293945, 20.01628303527832, 0.4, true),  // small.0.1
        (3, 48.849998474121094, 18.024425506591797, 0.4, true), // small.0.2
        (4, 51.150001525878906, 18.024425506591797, 0.4, true), // small.0.3
        (5, 53.45000076293945, 18.024425506591797, 0.4, true), // small.0.4
        (6, 54.599998474121094, 20.01628303527832, 0.4, true), // small.0.5
        (7, 45.400001525878906, 20.01628303527832, 0.4, true), // small.1.0
        (8, 46.54999923706055, 22.008142471313477, 0.4, true), // small.1.1
        (9, 50.0, 20.01628303527832, 0.4, true),               // small.1.2
        (10, 52.29999923706055, 20.01628303527832, 0.4, true), // small.1.3
        (11, 53.45000076293945, 22.008142471313477, 0.4, true), // small.1.4
        (12, 55.75, 22.008142471313477, 0.4, true),            // small.1.5
        (13, 44.25, 22.008142471313477, 0.4, true),            // small.2.0
        (14, 50.0, 24.0, 0.4, true),                           // small.2.1
        (15, 48.849998474121094, 22.008142471313477, 0.4, true), // small.2.2
        (16, 51.150001525878906, 22.008142471313477, 0.4, true), // small.2.3
        (17, 54.599998474121094, 24.0, 0.4, true),             // small.2.4
        (18, 56.900001525878906, 24.0, 0.4, true),             // small.2.5
        (19, 47.70000076293945, 24.0, 0.4, true),              // small.3.0
        (20, 48.849998474121094, 29.975574493408203, 0.4, true), // small.3.1
        (21, 48.849998474121094, 25.991857528686523, 0.4, true), // small.3.2
        (22, 52.29999923706055, 24.0, 0.4, true),              // small.3.3
        (23, 54.599998474121094, 27.98371696472168, 0.4, true), // small.3.4
        (24, 55.75, 25.991857528686523, 0.4, true),            // small.3.5
        (25, 45.400001525878906, 24.0, 1.1, true),             // large.0.0
        (26, 46.06489562988281, 27.235315322875977, 1.1, false), // large.0.1
        (27, 47.86237335205078, 28.503864288330078, 1.1, false), // large.0.2
        (28, 47.36805725097656, 25.462810516357422, 1.1, false), // large.0.3
        (29, 44.25, 25.991857528686523, 1.1, true),            // large.1.0
        (30, 45.74543380737305, 29.894134521484375, 1.1, false), // large.1.1
        (31, 50.0, 27.98371696472168, 1.1, true),              // large.1.2
        (32, 52.29999923706055, 27.98371696472168, 1.1, true), // large.1.3
        (33, 44.12495803833008, 28.272933959960938, 1.1, false), // large.2.0
        (34, 47.73109817504883, 30.974742889404297, 1.1, false), // large.2.1
        (35, 51.150001525878906, 29.975574493408203, 1.1, true), // large.2.2
        (36, 53.45000076293945, 29.975574493408203, 1.1, true), // large.2.3
    ];

    const FROM: Vec2 = Vec2::new(47.36805725097656, 25.462810516357422);
    const TARGET: Vec2 = Vec2::new(53.45000076293945, 25.991857528686523);
    const GOAL: Vec2 = Vec2::new(50., 24.);
    fn captured(reverse: bool) -> (MovementFrame, Vec<bool>, usize) {
        let mut rows = BODIES.to_vec();
        if reverse {
            rows.reverse();
            for row in &mut rows {
                row.0 = 1000 - row.0;
            }
        }
        let actor = rows
            .iter()
            .position(|r| Vec2::new(r.1, r.2) == FROM)
            .unwrap();
        let settled = rows.iter().map(|r| r.4).collect();
        let units = rows
            .into_iter()
            .map(|r| MovementFrameUnit {
                unit_id: r.0,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(r.1, r.2),
                radius: r.3,
                speed: Some(3.5),
                target: (!r.4).then_some(if Vec2::new(r.1, r.2) == FROM {
                    TARGET
                } else {
                    Vec2::new(r.1, r.2)
                }),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect();
        (
            MovementFrame {
                recovery: Default::default(),
                bounds: Vec2::new(64., 48.),
                tick_hz: 20,
                units,
                pools: BTreeMap::new(),
            },
            settled,
            actor,
        )
    }
    fn test_region() -> DestinationRegion {
        DestinationRegion {
            goal: GOAL,
            min: Vec2::new(43.1, 18.024425),
            max: Vec2::new(56.9, 29.975574),
            cohort_radius: 1.1,
        }
    }
    #[test]
    fn captured_production_parking_keeps_public_goal_and_every_physical_holder() {
        let mut expected = None;
        for reverse in [false, true] {
            let (frame, settled, actor) = captured(reverse);
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(2400);
            let waiting = apply_composed(
                &frame,
                &mut proposed,
                0.05,
                2400,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |_| true,
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| (i == actor).then_some(test_region()),
                |i| settled[i],
            );
            let slot = state
                .parking_slot(frame.units[actor].unit_id)
                .expect("positive current cycle must select reachable parking");
            assert!(!waiting.contains(&frame.units[actor].unit_id));

            assert_eq!(proposed.units[actor].target, Some(slot));
            assert_eq!(frame.units[actor].target, Some(TARGET));
            assert!(distance(slot, GOAL) < distance(FROM, GOAL));
            for (i, body) in frame.units.iter().enumerate() {
                if i == actor {
                    continue;
                }
                assert_eq!(proposed.units[i].position, body.position);
                assert_eq!(
                    disk_clear(
                        FROM,
                        slot,
                        1.1f32 as f64,
                        body.position,
                        f64::from(body.radius)
                    ),
                    Some(true)
                );
            }
            assert!(budget.pairs[0] <= 4096);
            if let Some(p) = expected {
                assert_eq!(slot, p);
            } else {
                expected = Some(slot);
            }
        }
    }
    #[test]
    fn captured_gm2_far_witnesses_authorize_only_current_valid_parking() {
        // Actual e5a2a22a tick5042 poses and dispatch-region audit. This checks
        // the bounded service; no missing pool, Guide, or controller is replayed.
        let bodies = [
            (
                1,
                Vec2::new(32.07806396484375, 25.465227127075195),
                0.4,
                None,
            ), // small.0.0
            (
                2,
                Vec2::new(47.70000076293945, 20.01628303527832),
                0.4,
                None,
            ), // small.0.1
            (
                3,
                Vec2::new(48.849998474121094, 18.024425506591797),
                0.4,
                None,
            ), // small.0.2
            (
                4,
                Vec2::new(51.150001525878906, 18.024425506591797),
                0.4,
                None,
            ), // small.0.3
            (
                5,
                Vec2::new(53.45000076293945, 18.024425506591797),
                0.4,
                None,
            ), // small.0.4
            (
                6,
                Vec2::new(54.599998474121094, 20.01628303527832),
                0.4,
                None,
            ), // small.0.5
            (
                7,
                Vec2::new(45.400001525878906, 20.01628303527832),
                0.4,
                None,
            ), // small.1.0
            (
                8,
                Vec2::new(46.54999923706055, 22.008142471313477),
                0.4,
                None,
            ), // small.1.1
            (9, Vec2::new(50.0, 20.01628303527832), 0.4, None), // small.1.2
            (
                10,
                Vec2::new(52.29999923706055, 20.01628303527832),
                0.4,
                None,
            ), // small.1.3
            (
                11,
                Vec2::new(53.45000076293945, 22.008142471313477),
                0.4,
                None,
            ), // small.1.4
            (12, Vec2::new(55.75, 22.008142471313477), 0.4, None), // small.1.5
            (13, Vec2::new(44.25, 22.008142471313477), 0.4, None), // small.2.0
            (14, Vec2::new(50.0, 24.0), 0.4, None),             // small.2.1
            (
                15,
                Vec2::new(48.849998474121094, 22.008142471313477),
                0.4,
                None,
            ), // small.2.2
            (
                16,
                Vec2::new(51.150001525878906, 22.008142471313477),
                0.4,
                None,
            ), // small.2.3
            (17, Vec2::new(54.599998474121094, 24.0), 0.4, None), // small.2.4
            (18, Vec2::new(56.900001525878906, 24.0), 0.4, None), // small.2.5
            (19, Vec2::new(47.70000076293945, 24.0), 0.4, None), // small.3.0
            (
                20,
                Vec2::new(48.849998474121094, 29.975574493408203),
                0.4,
                None,
            ), // small.3.1
            (
                21,
                Vec2::new(48.849998474121094, 25.991857528686523),
                0.4,
                None,
            ), // small.3.2
            (22, Vec2::new(52.29999923706055, 24.0), 0.4, None), // small.3.3
            (
                23,
                Vec2::new(54.599998474121094, 27.98371696472168),
                0.4,
                None,
            ), // small.3.4
            (24, Vec2::new(55.75, 25.991857528686523), 0.4, None), // small.3.5
            (
                25,
                Vec2::new(45.098323822021484, 25.385353088378906),
                1.1,
                Some(Vec2::new(45.400001525878906, 24.0)),
            ), // large.0.0
            (
                26,
                Vec2::new(47.82553482055664, 27.64948844909668),
                1.1,
                None,
            ), // large.0.1
            (
                27,
                Vec2::new(45.77350997924805, 23.291522979736328),
                1.1,
                Some(Vec2::new(51.150001525878906, 25.991857528686523)),
            ), // large.0.2
            (
                28,
                Vec2::new(47.42607116699219, 25.48605728149414),
                1.1,
                None,
            ), // large.0.3
            (
                29,
                Vec2::new(43.51490020751953, 26.912694931030273),
                1.1,
                None,
            ), // large.1.0
            (
                30,
                Vec2::new(50.338932037353516, 30.157453536987305),
                1.1,
                None,
            ), // large.1.1
            (31, Vec2::new(50.0, 27.98371696472168), 1.1, None), // large.1.2
            (
                32,
                Vec2::new(31.64053726196289, 26.899999618530273),
                1.1,
                Some(Vec2::new(52.29999923706055, 27.98371696472168)),
            ), // large.1.3
            (
                33,
                Vec2::new(40.80000305175781, 31.96743392944336),
                1.1,
                None,
            ), // large.2.0
            (
                34,
                Vec2::new(46.54999923706055, 29.975574493408203),
                1.1,
                None,
            ), // large.2.1
            (
                35,
                Vec2::new(45.629329681396484, 27.520309448242188),
                1.1,
                None,
            ), // large.2.2
            (
                36,
                Vec2::new(53.45000076293945, 29.975574493408203),
                1.1,
                None,
            ), // large.2.3
        ];
        let from = Vec2::new(45.77350997924805, 23.291522979736328);
        let target = Vec2::new(51.150001525878906, 25.991857528686523);
        for reverse in [false, true] {
            let mut frame = MovementFrame {
                recovery: Default::default(),
                bounds: Vec2::new(64., 48.),
                tick_hz: 20,
                pools: BTreeMap::new(),
                units: bodies
                    .iter()
                    .map(
                        |&(id, position, radius, target)| crate::movement::MovementFrameUnit {
                            unit_id: if reverse { 1000 - id } else { id },
                            owner_slot: 0,
                            unit_type: 0,
                            position,
                            radius,
                            target,
                            speed: target.map(|_| 2.5),
                            progress: None,
                            plain_progress: None,
                            physical_escape: None,
                        },
                    )
                    .collect(),
            };
            if reverse {
                frame.units.reverse();
            }
            let actor = frame
                .units
                .iter()
                .position(|u| u.position == from && u.target == Some(target))
                .unwrap();
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(5042);
            let region = DestinationRegion {
                goal: GOAL,
                min: Vec2::new(43.099995, 18.024424),
                max: Vec2::new(56.900005, 29.975576),
                cohort_radius: 1.1,
            };
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                5042,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |_| true,
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| (i == actor).then_some(region),
                |i| frame.units[i].target.is_none(),
            );
            let slot = state
                .parking_slot(frame.units[actor].unit_id)
                .expect("complete original region must retain far cycle witnesses");
            assert_eq!(
                slot, from,
                "the existing generator computes this exact current-valid candidate"
            );
            assert_eq!(proposed.units[actor].target, Some(slot));
            assert_eq!(frame.units[actor].target, Some(target));
            assert!(state.members[&frame.units[actor].unit_id].guidance_pairs <= 4096);
            for (i, peer) in frame.units.iter().enumerate() {
                if i == actor {
                    continue;
                }
                assert_eq!(
                    disk_clear(
                        from,
                        slot,
                        f64::from(1.1_f32),
                        peer.position,
                        f64::from(peer.radius)
                    ),
                    Some(true)
                );
                assert_eq!(proposed.units[i].position, peer.position);
            }
        }
    }
    #[test]
    fn terrain_refusal_and_empty_credit_cannot_authorize_parking() {
        for terrain_clear in [false, true] {
            let (frame, settled, actor) = captured(false);
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(2400);
            if terrain_clear {
                budget.pairs[0] = VISITS / (2 * SLOTS);
            }
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                2400,
                &mut state,
                |_, _, _| terrain_clear,
                |_, _, _, _| EntranceContact::Outside,
                |_| true,
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| (i == actor).then_some(test_region()),
                |i| settled[i],
            );
            assert_eq!(state.parking_slot(frame.units[actor].unit_id), None);
            assert_eq!(frame.units[actor].target, Some(TARGET));
        }
    }
    #[test]
    fn exact_cycle_refuses_tangent_edges_missing_holder_and_arithmetic() {
        let ring = [
            Vec2::new(-1., -1.),
            Vec2::new(1., -1.),
            Vec2::new(1., 1.),
            Vec2::new(-1., 1.),
        ];
        let blockers = ring.map(|position| Blocker {
            position,
            radius: 0.5,
        });
        let from = Vec2::new(4., 0.);
        let goal = Vec2::ZERO;
        assert_eq!(
            enclosed(from, goal, 0.5, &blockers, &mut || true),
            Some(false)
        );
        assert_eq!(
            enclosed(from, goal, 0.6, &blockers, &mut || true),
            Some(true)
        );
        assert_eq!(
            enclosed(from, goal, 0.6, &blockers[..3], &mut || true),
            Some(false)
        );
        assert_eq!(enclosed(from, goal, 0.6, &blockers, &mut || false), None);
        assert_eq!(
            enclosed(Vec2::new(f32::MAX, 0.), goal, 0.6, &blockers, &mut || true),
            None
        );
    }
    #[test]
    fn second_canonical_capture_has_current_enclosure_and_reachable_parking() {
        let capture: serde_json::Value =
            serde_json::from_str(include_str!("captured_parking_second.json")).unwrap();
        let rows = capture["bodies"].as_array().unwrap();
        let vec2 = |v: &serde_json::Value| {
            Vec2::new(v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32)
        };
        let units: Vec<_> = rows
            .iter()
            .map(|r| MovementFrameUnit {
                unit_id: r["id"].as_u64().unwrap() as u32,
                owner_slot: 0,
                unit_type: 1,
                position: vec2(&r["position"]),
                radius: r["radius"].as_f64().unwrap() as f32,
                target: (!r["target"].is_null()).then(|| vec2(&r["target"])),
                speed: Some(3.5),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect();
        let settled: Vec<_> = rows
            .iter()
            .map(|r| r["settled"].as_bool().unwrap())
            .collect();
        let frame = MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::new(64., 48.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        };
        let full: Vec<_> = frame
            .units
            .iter()
            .enumerate()
            .filter(|(i, _)| settled[*i])
            .map(|(_, u)| Blocker {
                position: u.position,
                radius: u.radius,
            })
            .collect();
        let mut missing = Vec::new();
        for actor in 0..frame.units.len() {
            let u = &frame.units[actor];
            let Some(target) = u.target else {
                continue;
            };
            let initial: Vec<_> = frame
                .units
                .iter()
                .enumerate()
                .filter(|(i, peer)| {
                    *i != actor
                        && disk_clear(
                            u.position,
                            u.position,
                            f64::from(u.radius),
                            peer.position,
                            f64::from(peer.radius),
                        ) != Some(true)
                })
                .map(|(_, p)| p.unit_id)
                .collect();
            let mut state = TrafficState::default();
            let mut proposed = frame.clone();
            let mut budget = crate::movement::sweep::SweepBudget::new(2400);
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                2400,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |i| i == actor,
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| (i == actor).then_some(test_region()),
                |i| settled[i],
            );
            let selected = state.parking_slot(u.unit_id);
            eprintln!(
                "second actor{} initial_overlap={initial:?} full_cycle={:?} slot={selected:?} pairs={:?}",
                u.unit_id,
                enclosed(u.position, target, u.radius, &full, &mut || true),
                budget.pairs
            );
            if [27, 30].contains(&u.unit_id) && selected.is_none() {
                missing.push(u.unit_id);
            }
            if [25, 29].contains(&u.unit_id) {
                assert_eq!(
                    selected, None,
                    "no current settled enclosure authorizes this actor yet"
                );
            }
        }
        assert!(
            missing.is_empty(),
            "current production parking remains missing for {missing:?}"
        );
    }
    #[test]
    fn unstalled_private_moves_spend_no_parking_work() {
        let (frame, settled, actor) = captured(false);
        let mut expected = None;
        for public in [false, true] {
            let mut state = TrafficState::default();
            let mut proposed = frame.clone();
            let mut budget = crate::movement::sweep::SweepBudget::new(1);
            let mut terrain_calls = 0;
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                1,
                &mut state,
                |_, _, _| {
                    terrain_calls += 1;
                    true
                },
                |_, _, _, _| EntranceContact::Outside,
                |_| false,
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| (public && i == actor).then_some(test_region()),
                |i| settled[i],
            );
            assert_eq!(state.parking_slot(frame.units[actor].unit_id), None);
            let work = (
                budget.pairs,
                state.tick_owner_neighbors,
                state.index_work.queries,
                terrain_calls,
            );
            if let Some(expected) = expected {
                assert_eq!(work, expected);
            } else {
                expected = Some(work);
            }
        }
    }
    #[test]
    fn origin_enclosure_cannot_authorize_distant_parking_with_or_without_goal_holder() {
        for held_goal in [false, true] {
            let goal = Vec2::new(40., 10.);
            let private = Vec2::new(40., 11.);
            let mut units = vec![MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(10., 10.),
                radius: 1.1,
                speed: Some(3.5),
                target: Some(private),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            }];
            for (i, position) in [
                Vec2::new(8., 10.),
                Vec2::new(10., 8.),
                Vec2::new(12., 10.),
                Vec2::new(10., 12.),
            ]
            .into_iter()
            .enumerate()
            {
                let mut body = units[0].clone();
                body.unit_id = i as u32 + 2;
                body.position = position;
                body.radius = 0.4;
                body.target = None;
                units.push(body);
            }
            if held_goal {
                let mut holder = units[1].clone();
                holder.unit_id = 6;
                holder.position = goal;
                units.push(holder);
            }
            let ring: Vec<_> = units[1..5]
                .iter()
                .map(|u| Blocker {
                    position: u.position,
                    radius: u.radius,
                })
                .collect();
            assert_eq!(
                enclosed(units[0].position, private, 1.1, &ring, &mut || true),
                Some(true)
            );
            let region = DestinationRegion {
                goal,
                min: Vec2::new(37.7, 8.008142),
                max: Vec2::new(42.3, 11.991858),
                cohort_radius: 1.1,
            };
            let frame = MovementFrame {
                recovery: Default::default(),
                bounds: Vec2::splat(64.),
                tick_hz: 20,
                units,
                pools: BTreeMap::new(),
            };
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let mut budget = crate::movement::sweep::SweepBudget::new(1);
            apply_composed(
                &frame,
                &mut proposed,
                0.05,
                1,
                &mut state,
                |_, _, _| true,
                |_, _, _, _| EntranceContact::Outside,
                |_| true,
                &mut budget,
                |_, _| None,
                |_| false,
                |_, _| true,
                true,
                |i| (i == 0).then_some(region),
                |i| i != 0,
            );
            assert_eq!(
                state.parking_slot(1),
                None,
                "origin cage, goal_holder={held_goal}"
            );
            assert_eq!(frame.units[0].target, Some(private));
        }
    }
    #[test]
    fn destination_region_uses_public_cohort_geometry_not_hidden_displaced_slots() {
        let members: Vec<_> = (0..36)
            .map(|id| crate::movement::SpreadMember {
                id,
                position: Vec2::ZERO,
                radius: if id < 24 { 0.4 } else { 1.1 },
            })
            .collect();
        let mut pool = crate::movement::MovementPool {
            pool_id: 1,
            point: GOAL,
            spacing: 2. * 1.1 + 0.1,
            primary_count: 36,
            slots: vec![GOAL; 36],
        };
        let region = DestinationRegion::from_dispatch(&pool, &members).unwrap();
        pool.slots.fill(Vec2::new(-100000., 100000.));
        assert_eq!(
            DestinationRegion::from_dispatch(&pool, &members),
            Some(region)
        );
        assert!(!region.contains(Vec2::new(10., 10.), 1.1));
        assert!(region.contains(Vec2::new(43.1, 18.024425), 1.1));
        assert!(region.contains(Vec2::new(43.1 - 2.2, 18.024425), 1.1));
    }
}
