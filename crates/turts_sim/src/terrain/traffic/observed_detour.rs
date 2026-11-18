//! Local obstacle advice from CURRENT captured foreign bodies. No foreign intent.
use super::*;
use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};
#[derive(Clone, Copy)]
struct Row {
    id: u32,
    body: ObservedBody,
    foreign: bool,
}
// Cooperative index/results are not live in this phase. One viewer table/index
// and two complete <=128-row leg/generator results are simultaneously live.
const _: () = assert!(
    RETAINED_ALLOWANCE
        + INDEX_BODIES * PER_BODY_SCRATCH
        + FIXED_SCRATCH
        + EnvelopeIndex::capacity_bound(INDEX_BODIES)
        + (INDEX_BODIES + 2 * NEIGHBORS) * std::mem::size_of::<Row>()
        <= SCRATCH_BYTES
);
fn point(p: Vec2) -> Point {
    Point {
        x: p.x.into(),
        y: p.y.into(),
    }
}
fn bounds(from: Vec2, to: Vec2, radius: f32) -> [f64; 4] {
    region(from, to, 16. * f64::from(radius))
}
fn region(from: Vec2, to: Vec2, r: f64) -> [f64; 4] {
    // Broad advice range only; exact full-leg body and terrain predicates decide.
    [
        (f64::from(from.x.min(to.x)) - r).next_down(),
        (f64::from(from.x.max(to.x)) + r).next_up(),
        (f64::from(from.y.min(to.y)) - r).next_down(),
        (f64::from(from.y.max(to.y)) + r).next_up(),
    ]
}
#[allow(
    clippy::too_many_arguments,
    reason = "Exact body tests debit the existing explicit owner work port"
)]
fn certify(
    from: Vec2,
    to: Vec2,
    radius: f32,
    rows: &[Row],
    budget: &mut crate::movement::sweep::SweepBudget,
    owner: u8,
    pairs: &mut usize,
    work: &mut detour::Work,
) -> Option<bool> {
    for row in rows {
        if !budget.charge_guidance_pair(owner, pairs) {
            return None;
        }
        work.body_checks += 1;
        let clear = swept_disks_clear(
            point(from),
            point(to),
            radius.into(),
            point(row.body.position),
            point(row.body.position),
            row.body.radius.into(),
            &mut Budget::new(1),
        )
        .unwrap_or(false);
        // One exact veto disproves the leg. Do not lose it to a later service
        // refusal or spend more pairs on rows that cannot make it clear again.
        if !clear {
            return Some(false);
        }
    }
    Some(true)
}
/// Keep a selected CURRENT-witnessed corner as private intent when optional
/// service cannot finish. This neither advances the cursor nor certifies a
/// chord. Current modeling and the final physical/terrain guards still run.
fn defer_entry(
    proposed: &mut MovementFrame,
    index: usize,
    unit: &MovementFrameUnit,
    state: &mut TrafficState,
    entry: Option<own_detour::Retained>,
) {
    if let Some(saved) = entry {
        proposed.units[index].target = Some(saved.route.corners[saved.corner]);
        proposed.units[index].speed = unit.speed;
        state.members.get_mut(&unit.unit_id).unwrap().passing = None;
    }
    state.members.get_mut(&unit.unit_id).unwrap().local_detour = entry;
}

fn record_work(state: &mut TrafficState, work: detour::Work) {
    state.detour_work.scanned_rows += work.scanned_rows;
    state.detour_work.generator_comparisons += work.generator_comparisons;
    state.detour_work.candidates += work.candidates;
    state.detour_work.body_checks += work.body_checks;
    state.detour_work.terrain_calls += work.terrain_calls;
}
fn physical_row(a: &Row, b: &Row) -> Ordering {
    point_cmp(a.body.position, b.body.position)
        .then_with(|| a.body.radius.total_cmp(&b.body.radius))
}
/// Eight witnesses, two sides, up to two first-corner variants: at most 32
/// candidates. Both variants consume the same unchanged query/pair credit.
/// A witness alone determines both corners; other rows only veto unsafe routes.
fn propose(
    unit: &MovementFrameUnit,
    requested: Vec2,
    rows: &[Row],
    clear: &mut impl FnMut(Vec2, Vec2) -> bool,
    body_clear: &mut impl FnMut(Vec2, Vec2, &mut detour::Work) -> Option<bool>,
    work: &mut detour::Work,
) -> Result<Option<(detour::Detour, Row)>, ()> {
    match body_clear(unit.position, requested, work) {
        Some(true) => return Ok(None),
        Some(false) => {}
        None => return Err(()),
    }
    let axis = (requested.as_dvec2() - unit.position.as_dvec2()).normalize_or_zero();
    if axis == bevy_math::DVec2::ZERO {
        return Ok(None);
    }
    let normal = bevy_math::DVec2::new(-axis.y, axis.x);
    // Exact geometric extrema and nearest obstructions. Never use row/handle ties.
    let mut generators = [None; 8];
    for (slot, direction) in [axis, -axis, normal, -normal].into_iter().enumerate() {
        for row in rows {
            work.scanned_rows += 1;
            if !row.foreign {
                continue;
            }
            work.generator_comparisons += 1;
            let score = (row.body.position.as_dvec2() - unit.position.as_dvec2()).dot(direction)
                + f64::from(row.body.radius);
            if generators[slot].is_none_or(|old: Row| {
                let prior = (old.body.position.as_dvec2() - unit.position.as_dvec2())
                    .dot(direction)
                    + f64::from(old.body.radius);
                score
                    .total_cmp(&prior)
                    .then_with(|| physical_row(row, &old).reverse())
                    .is_gt()
            }) {
                generators[slot] = Some(*row);
            }
        }
    }
    for row in rows {
        work.scanned_rows += 1;
        if !row.foreign {
            continue;
        }
        let distance = unit
            .position
            .as_dvec2()
            .distance_squared(row.body.position.as_dvec2());
        let mut candidate = *row;
        let mut score = distance;
        for slot in &mut generators[4..] {
            work.generator_comparisons += 1;
            if slot.is_none_or(|old: Row| {
                score
                    .total_cmp(
                        &unit
                            .position
                            .as_dvec2()
                            .distance_squared(old.body.position.as_dvec2()),
                    )
                    .then_with(|| physical_row(&candidate, &old))
                    .is_lt()
            }) {
                let previous = slot.replace(candidate);
                if let Some(old) = previous {
                    candidate = old;
                    score = unit
                        .position
                        .as_dvec2()
                        .distance_squared(old.body.position.as_dvec2());
                } else {
                    break;
                }
            }
        }
    }
    let mut best: Option<(bool, f64, detour::Detour, Row)> = None;
    let from = unit.position.as_dvec2();
    let original = from.distance(requested.as_dvec2());
    for witness in generators.into_iter().flatten() {
        let mut ambiguous = false;
        for row in rows {
            work.generator_comparisons += 1;
            ambiguous |= row.id != witness.id && physical_row(row, &witness) == Ordering::Equal;
        }
        if ambiguous {
            continue;
        }

        let relative = witness.body.position.as_dvec2() - from;
        let along = relative.dot(axis);
        let cross = relative.dot(normal);
        let reach = f64::from(unit.radius) + f64::from(witness.body.radius);
        // Generating geometry uses only this captured witness and the actor.
        let slack = reach * 0.0001 + 0.00001;
        for side in [-1., 1.] {
            // Preserve the actor's requested lateral formation lane. A shared
            // obstacle face alone must not collapse a moving cohort onto one
            // temporary point. This uses only own intent and this exact witness.
            let lateral = if axis.x.abs() >= axis.y.abs() {
                f64::from((requested.y - witness.body.position.y).abs())
            } else {
                f64::from((requested.x - witness.body.position.x).abs())
            };
            let alternatives = if along > 0. {
                [Some((along * 0.5, lateral)), Some((-2. * reach, 0.))]
            } else {
                [Some((-2. * reach, 0.)), None]
            };
            for (first_along, lane) in alternatives.into_iter().flatten() {
                let across = cross + side * (reach + slack + lane);
                work.candidates += 1;
                let first = (from + axis * first_along + normal * across).as_vec2();
                let second = (from + axis * (along + reach + slack) + normal * across).as_vec2();
                if !first.is_finite()
                    || !second.is_finite()
                    || first == unit.position
                    || first == second
                    || second.as_dvec2().distance(requested.as_dvec2()) >= original
                {
                    continue;
                }
                let first_clear = body_clear(unit.position, first, work).ok_or(())?;
                let second_clear = body_clear(first, second, work).ok_or(())?;
                if !first_clear || !second_clear {
                    continue;
                }
                work.terrain_calls += 1;
                if !clear(unit.position, first) {
                    continue;
                }
                work.terrain_calls += 1;
                if !clear(first, second) {
                    continue;
                }
                // Prefer a complete currently certified continuation over a
                // short local bypass whose remaining chord hits the same wall.
                // Staged local advice is retained when no complete option exists.
                let mut complete = body_clear(second, requested, work).ok_or(())?;
                if complete {
                    work.terrain_calls += 1;
                    complete = clear(second, requested);
                }
                let route = detour::Detour {
                    corners: [first, second],
                };
                let score = from.distance(first.as_dvec2())
                    + first.as_dvec2().distance(second.as_dvec2())
                    + second.as_dvec2().distance(requested.as_dvec2());
                if best.is_none_or(|(old_complete, old, prior, _)| {
                    complete
                        .cmp(&old_complete)
                        .reverse()
                        .then_with(|| score.total_cmp(&old))
                        .then_with(|| point_cmp(first, prior.corners[0]))
                        .then_with(|| point_cmp(second, prior.corners[1]))
                        .is_lt()
                }) {
                    best = Some((complete, score, route, witness));
                }
                // A valid forward route ends this side; backward is a recovery
                // alternative, never extra work after a certified forward proposal.
                break;
            }
        }
    }
    Ok(best.map(|(_, _, route, row)| (route, row)))
}
fn query(
    index: &mut EnvelopeIndex,
    rows: &[Row],
    b: [f64; 4],
    visits: &mut usize,
    quota: usize,
    work: &mut detour::Work,
) -> Option<Vec<Row>> {
    if *visits >= quota {
        return None;
    }
    *visits += 1; // Empty metadata queries also consume existing credit.
    let found = index.query(b, None)?;
    let mut local = Vec::with_capacity(NEIGHBORS);
    for row in found {
        if *visits >= quota || local.len() == NEIGHBORS {
            return None;
        }
        *visits += 1;
        local.push(rows[row]);
    }
    // At most 128 complete rows, in-place with no new allocation. Count this
    // bounded ordering alongside other nonexhaustible geometry comparisons;
    // sorting work never consumes or replenishes owner/pair service credit.
    local.sort_unstable_by(|a, b| {
        work.generator_comparisons += 1;
        physical_row(a, b)
    });
    Some(local)
}
#[allow(
    clippy::too_many_arguments,
    reason = "One bounded phase shares existing owner work and permission ports"
)]
#[allow(
    clippy::needless_range_loop,
    reason = "Owner indices select independent service budgets."
)]
pub(super) fn apply(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    dt: f32,
    tick: u32,
    state: &mut TrafficState,
    service: &mut [usize],
    waiting: &BTreeSet<u32>,
    visits: &mut [usize; SLOTS],
    clear: &mut impl FnMut(usize, Vec2, Vec2) -> bool,
    budget: &mut crate::movement::sweep::SweepBudget,
    observed: &mut impl FnMut(u8, u32) -> Option<ObservedBody>,
    eligible: &mut impl FnMut(usize) -> bool,
    observed_rows: &mut impl FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
    quota: usize,
) {
    #[cfg(feature = "test-probes")]
    let trace_ids = std::env::var("TURTS_BODY_TRACE_IDS").ok();
    service.sort_by(|&a, &b| {
        let age = |i: usize| {
            state.members[&frame.units[i].unit_id]
                .attempts
                .get(Attempt::Detour)
                .unwrap_or(state.members[&frame.units[i].unit_id].accepted_tick)
        };
        age(a)
            .cmp(&age(b))
            .then_with(|| physical(&frame.units[a], &frame.units[b]))
    });
    for owner in 0..SLOTS {
        // Reserve the PUBLIC cap, not surviving frame length: a legal command
        // may remove a foreign body after Start without changing its capture.
        let mut rows = Vec::with_capacity(INDEX_BODIES);
        // The capture allocation is live even when an empty/incomplete supplier
        // prevents index construction. Keep that early phase peak truthful.
        state.scratch_bound = state.scratch_bound.max(
            frame.units.len() * PER_BODY_SCRATCH
                + FIXED_SCRATCH
                + rows.capacity() * std::mem::size_of::<Row>(),
        );
        for unit in &frame.units {
            state.detour_filter_rows += 1;
            if usize::from(unit.owner_slot) == owner && !moving(unit) {
                rows.push(Row {
                    id: unit.unit_id,
                    body: ObservedBody {
                        position: unit.position,
                        radius: unit.radius,
                    },
                    foreign: false,
                });
            }
        }
        let mut foreign = false;
        let mut complete = true;
        let supplied = observed_rows(owner as u8, &mut |id, body| {
            state.detour_filter_rows += 1;
            if !complete {
                return false;
            }
            if rows.len() == INDEX_BODIES {
                complete = false;
                return false;
            }
            rows.push(Row {
                id,
                body,
                foreign: true,
            });
            foreign = true;
            true
        });
        if !foreign || !complete || !supplied {
            for &i in service
                .iter()
                .filter(|&&i| usize::from(frame.units[i].owner_slot) == owner)
            {
                let member = state.members.get_mut(&frame.units[i].unit_id).unwrap();
                if member.local_detour.is_some_and(|r| r.witness.is_some()) {
                    member.local_detour = None;
                }
            }
            continue;
        }
        let Some(mut index) = EnvelopeIndex::new(rows.len(), |i| {
            let r = rows[i];
            (r.body.position.is_finite() && r.body.radius.is_finite() && r.body.radius > 0.)
                .then(|| region(r.body.position, r.body.position, f64::from(r.body.radius)))
        }) else {
            for &i in service
                .iter()
                .filter(|&&i| usize::from(frame.units[i].owner_slot) == owner)
            {
                let member = state.members.get_mut(&frame.units[i].unit_id).unwrap();
                if member.local_detour.is_some_and(|r| r.witness.is_some()) {
                    member.local_detour = None;
                }
            }
            continue;
        };
        state.scratch_bound = state.scratch_bound.max(
            frame.units.len() * PER_BODY_SCRATCH
                + FIXED_SCRATCH
                + rows.capacity() * std::mem::size_of::<Row>()
                + index.bytes(),
        );
        for &i in service.iter() {
            let unit = &frame.units[i];
            if usize::from(unit.owner_slot) != owner || waiting.contains(&unit.unit_id) {
                continue;
            }
            let retained = state.members[&unit.unit_id].local_detour;
            #[cfg(feature = "test-probes")]
            let tracing = (91..=98).contains(&tick)
                && trace_ids.as_ref().is_some_and(|ids| {
                    ids.split(',')
                        .any(|id| id.parse::<u32>().ok() == Some(unit.unit_id))
                });
            #[cfg(feature = "test-probes")]
            if tracing {
                eprintln!(
                    "DETOUR_BEGIN tick={tick} unit={} from={:?} requested={:?} retained={retained:?}",
                    unit.unit_id, unit.position, unit.target
                );
            }
            if retained.is_some_and(|r| r.witness.is_none()) {
                continue;
            }
            let Some(requested) = unit.target else {
                continue;
            };
            let mut retained = retained.filter(|r| {
                state.members[&unit.unit_id].owner == unit.owner_slot
                    && r.corner < 2
                    && r.requested == requested
                    && r.radius == unit.radius.to_bits()
                    && r.kind == unit.unit_type
                    && r.witness
                        .is_some_and(|(id, body)| observed(owner as u8, id) == Some(body))
            });
            #[cfg(feature = "test-probes")]
            if tracing {
                eprintln!(
                    "DETOUR_AUTH tick={tick} unit={} accepted={retained:?}",
                    unit.unit_id
                );
            }
            // A completed local route releases independently of optional work.
            retained = retained.filter(|r| {
                let last = r.route.corners[1].as_dvec2();
                let toward = requested.as_dvec2() - last;
                !(r.corner == 1
                    && (unit.position == r.route.corners[1]
                        || (toward.length_squared() > 0.
                            && (unit.position.as_dvec2() - last).dot(toward) >= 0.)))
            });
            let mut entry = retained;
            state.members.get_mut(&unit.unit_id).unwrap().local_detour = None;
            if retained.is_none() && !eligible(i) {
                continue;
            }
            let mut pairs = 0;
            let mut work = detour::Work::default();
            {
                // A clear requested chord needs no broad generator search. Only
                // a CURRENT foreign obstruction enables early observed advice;
                // own settled obstacles keep their existing helper deadline.
                let before = visits[owner];
                let direct = query(
                    &mut index,
                    &rows,
                    region(unit.position, requested, f64::from(unit.radius)),
                    &mut visits[owner],
                    quota,
                    &mut work,
                );
                if visits[owner] > before {
                    state
                        .members
                        .get_mut(&unit.unit_id)
                        .unwrap()
                        .attempts
                        .set(Attempt::Detour, Some(tick));
                }
                let Some(mut direct) = direct else {
                    state.neighbor_overflows += 1;
                    defer_entry(proposed, i, unit, state, entry);
                    continue;
                };
                state.detour_filter_rows += direct.len();
                if retained.is_none() {
                    direct.retain(|row| row.foreign);
                }
                match certify(
                    unit.position,
                    requested,
                    unit.radius,
                    &direct,
                    budget,
                    unit.owner_slot,
                    &mut pairs,
                    &mut work,
                ) {
                    Some(false) => {}
                    Some(true) => {
                        let released = if retained.is_some() {
                            work.terrain_calls += 1;
                            clear(i, unit.position, requested)
                        } else {
                            true
                        };
                        if released {
                            record_work(state, work);
                            continue;
                        }
                    }
                    None => {
                        state.neighbor_overflows += 1;
                        record_work(state, work);
                        defer_entry(proposed, i, unit, state, entry);
                        continue;
                    }
                }
            }
            let before = visits[owner];
            let result = query(
                &mut index,
                &rows,
                bounds(unit.position, requested, unit.radius),
                &mut visits[owner],
                quota,
                &mut work,
            );
            if visits[owner] > before {
                state
                    .members
                    .get_mut(&unit.unit_id)
                    .unwrap()
                    .attempts
                    .set(Attempt::Detour, Some(tick));
            }
            let Some(local) = result else {
                state.neighbor_overflows += 1;
                record_work(state, work);
                defer_entry(proposed, i, unit, state, entry);
                continue;
            };
            let mut exhausted = false;
            let mut continuation = None;
            let mut bodies_clear = |from: Vec2, to: Vec2, work: &mut detour::Work| {
                let current = query(
                    &mut index,
                    &rows,
                    region(from, to, f64::from(unit.radius)),
                    &mut visits[owner],
                    quota,
                    work,
                )?;
                certify(
                    from,
                    to,
                    unit.radius,
                    &current,
                    budget,
                    unit.owner_slot,
                    &mut pairs,
                    work,
                )
            };
            if let Some(saved) = retained.as_mut() {
                // Intermediate points do not own space. Skip the first corner
                // only after a complete current body + terrain connector proof.
                if saved.corner == 0 {
                    let to = saved.route.corners[1];
                    match bodies_clear(unit.position, to, &mut work) {
                        Some(true) => {
                            work.terrain_calls += 1;
                            if clear(i, unit.position, to) {
                                saved.corner = 1;
                            }
                        }
                        Some(false) => {}
                        None => {
                            exhausted = true;
                        }
                    }
                }
                let last = saved.route.corners[1].as_dvec2();
                let toward = requested.as_dvec2() - last;
                if saved.corner == 1
                    && toward.length_squared() > 0.
                    && (unit.position.as_dvec2() - last).dot(toward) >= 0.
                {
                    // This retires LOCAL guidance only. If the requested endpoint
                    // is still blocked, a new fully certified local route follows.
                    saved.corner = 2;
                }
                while saved.corner < 2 && unit.position == saved.route.corners[saved.corner] {
                    saved.corner += 1;
                }
                if saved.corner == 2 {
                    retained = None;
                    entry = None;
                } else {
                    let mut from = unit.position;
                    let mut valid = true;

                    for &to in &saved.route.corners[saved.corner..] {
                        match bodies_clear(from, to, &mut work) {
                            Some(true) => {
                                work.terrain_calls += 1;
                                valid &= clear(i, from, to);
                            }
                            Some(false) => valid = false,
                            None => {
                                exhausted = true;
                                break;
                            }
                        }
                        from = to;
                    }
                    #[cfg(feature = "test-probes")]
                    if tracing {
                        eprintln!(
                            "DETOUR_VALID tick={tick} unit={} valid={valid} exhausted={exhausted} saved={saved:?}",
                            unit.unit_id
                        );
                    }
                    if !valid {
                        // A complete negative leg proof dominates any later
                        // service exhaustion; never restore that old intent.
                        retained = None;
                        entry = None;
                    }
                    if exhausted {
                        state.neighbor_overflows += 1;
                    }
                }
            }
            if retained.is_none() && !exhausted {
                match propose(
                    unit,
                    requested,
                    &local,
                    &mut |a, b| clear(i, a, b),
                    &mut bodies_clear,
                    &mut work,
                ) {
                    Ok(Some((route, witness))) => {
                        retained = Some(own_detour::Retained {
                            route,
                            corner: 0,
                            requested,
                            radius: unit.radius.to_bits(),
                            kind: unit.unit_type,
                            witness: Some((witness.id, witness.body)),
                        })
                    }
                    Ok(None) => {}
                    Err(()) => {
                        state.neighbor_overflows += 1;
                    }
                }
            }
            if let Some(saved) = retained.as_ref().filter(|saved| saved.corner == 1) {
                // A corner is a route aid, not an owned destination. Look one
                // authored step along its already retained final leg, but use
                // that connector only after complete current body/terrain proof.
                // The retained corner and its CURRENT witness stay unchanged.
                let last = saved.route.corners[1].as_dvec2();
                let final_leg = requested.as_dvec2() - last;
                let length = final_leg.length();
                let step = f64::from(unit.speed.unwrap_or(0.)) * f64::from(dt);
                if length > 0. && step > 0. && step.is_finite() {
                    // Farthest-first bounded connectors. A full-step lookahead
                    // may cut the obstacle at the turn; shorter final-leg points
                    // remain useful without requiring arrival at the corner.
                    for divisor in [1., 2., 4., 8.] {
                        let ahead =
                            (last + final_leg * ((step / divisor).min(length) / length)).as_vec2();
                        if !ahead.is_finite() || ahead == saved.route.corners[1] {
                            continue;
                        }
                        work.candidates += 1;
                        match bodies_clear(unit.position, ahead, &mut work) {
                            Some(true) => {
                                work.terrain_calls += 1;
                                if clear(i, unit.position, ahead) {
                                    continuation = Some(ahead);
                                    break;
                                }
                            }
                            Some(false) => {}
                            None => {
                                exhausted = true;
                                state.neighbor_overflows += 1;
                                break;
                            }
                        }
                    }
                }
            }
            if exhausted {
                // Restore the ENTRY intent only: partially advanced cursors and
                // new lookahead are not published as fresh route certificates.
                retained = entry;
                continuation = None;
            }
            if pairs > 0 {
                state
                    .members
                    .get_mut(&unit.unit_id)
                    .unwrap()
                    .attempts
                    .set(Attempt::Detour, Some(tick));
            }
            record_work(state, work);
            #[cfg(feature = "test-probes")]
            if tracing {
                eprintln!(
                    "DETOUR_END tick={tick} unit={} retained={retained:?}",
                    unit.unit_id
                );
            }
            if let Some(saved) = retained {
                proposed.units[i].target =
                    Some(continuation.unwrap_or(saved.route.corners[saved.corner]));
                proposed.units[i].speed = unit.speed;
                state.members.get_mut(&unit.unit_id).unwrap().passing = None;
            }
            state.members.get_mut(&unit.unit_id).unwrap().local_detour = retained;
        }
        state.index_work.add(index.work);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture(invert: bool) -> MovementFrame {
        let mut units = vec![MovementFrameUnit {
            unit_id: if invert { 1000 } else { 1 },
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(67.90744, 30.401608),
            radius: 0.3,
            speed: Some(100.),
            target: Some(Vec2::new(36.9, 32.)),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }];
        for y in 0..10 {
            for x in 0..10 {
                let mut row = units[0].clone();
                row.unit_id = if invert {
                    999 - (y * 10 + x)
                } else {
                    2 + y * 10 + x
                };
                row.owner_slot = 1;
                row.position = Vec2::new(60.625 + x as f32 * 0.75, 28.625 + y as f32 * 0.75);
                row.speed = None;
                row.target = None;
                units.push(row);
            }
        }
        if invert {
            units.reverse();
        }
        MovementFrame {
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        }
    }
    pub(super) fn run(
        frame: &MovementFrame,
        state: &mut TrafficState,
        tick: u32,
        visible: bool,
        shift: Vec2,
    ) -> MovementFrame {
        run_captured(frame, frame, state, tick, visible, shift, true)
    }
    fn run_captured(
        frame: &MovementFrame,
        captured: &MovementFrame,
        state: &mut TrafficState,
        tick: u32,
        visible: bool,
        shift: Vec2,
        complete: bool,
    ) -> MovementFrame {
        let mut proposed = frame.clone();
        let mut budget = crate::movement::sweep::SweepBudget::new(tick);
        apply_with_observed_detours(
            frame,
            &mut proposed,
            0.05,
            tick,
            state,
            |_, _, _| true,
            |_, _, _, _| EntranceContact::Outside,
            |_| false,
            &mut budget,
            |viewer, id| {
                captured
                    .units
                    .iter()
                    .find(|u| u.unit_id == id && u.owner_slot != viewer && visible)
                    .map(|u| ObservedBody {
                        position: u.position + shift,
                        radius: u.radius,
                    })
            },
            |_| true,
            |viewer, sink| {
                for row in captured
                    .units
                    .iter()
                    .filter(|u| u.owner_slot != viewer && visible)
                {
                    if !sink(
                        row.unit_id,
                        ObservedBody {
                            position: row.position + shift,
                            radius: row.radius,
                        },
                    ) {
                        return false;
                    }
                }
                complete
            },
        );
        proposed
    }
    #[test]
    fn disclosed_wall_generates_safe_corners_under_id_and_enumeration_inversion() {
        let mut results = Vec::new();
        for invert in [false, true] {
            let frame = fixture(invert);
            let mut state = TrafficState::default();
            let proposed = run(&frame, &mut state, 1, true, Vec2::ZERO);
            let actor = frame.units.iter().find(|u| u.owner_slot == 0).unwrap();
            let route = state.members[&actor.unit_id]
                .local_detour
                .expect("captured wall must produce advice");
            assert!(route.witness.is_some());
            assert_ne!(
                proposed
                    .units
                    .iter()
                    .find(|u| u.owner_slot == 0)
                    .unwrap()
                    .target,
                actor.target
            );
            let rows = frame
                .units
                .iter()
                .filter(|u| u.owner_slot == 1)
                .map(|u| Row {
                    id: u.unit_id,
                    foreign: true,
                    body: ObservedBody {
                        position: u.position,
                        radius: u.radius,
                    },
                })
                .collect::<Vec<_>>();
            let mut from = actor.position;
            let mut pairs = 0;
            for to in route.route.corners {
                assert_eq!(
                    certify(
                        from,
                        to,
                        actor.radius,
                        &rows,
                        &mut crate::movement::sweep::SweepBudget::new(1),
                        0,
                        &mut pairs,
                        &mut detour::Work::default()
                    ),
                    Some(true)
                );
                from = to;
            }
            results.push(route.route.corners);
        }
        assert_eq!(results[0], results[1]);
    }
    #[test]
    fn observed_controller_escapes_the_frozen_wall_with_real_continuous_movement() {
        for invert in [false, true] {
            let mut frame = fixture(invert);
            let mut state = TrafficState::default();
            for tick in 1..=20 {
                let advice = run(&frame, &mut state, tick, true, Vec2::ZERO);
                let outputs = crate::movement::step_frame_at_tick(&advice, 0.05, tick);
                for unit in &mut frame.units {
                    let output = outputs
                        .iter()
                        .find(|out| out.unit_id == unit.unit_id)
                        .unwrap();
                    if unit.owner_slot == 0 {
                        for body in advice.units.iter().filter(|other| other.owner_slot == 1) {
                            assert!(
                                swept_disks_clear(
                                    point(unit.position),
                                    point(output.position),
                                    unit.radius.into(),
                                    point(body.position),
                                    point(body.position),
                                    body.radius.into(),
                                    &mut Budget::new(1)
                                )
                                .unwrap()
                            );
                        }
                    }
                    unit.position = output.position;
                }
            }
            let actor = frame.units.iter().find(|u| u.owner_slot == 0).unwrap();
            assert!(
                actor.position.x < 60.,
                "controller must pass the wall, got {:?}",
                actor.position
            );
            assert_eq!(actor.target, Some(Vec2::new(36.9, 32.)));
        }
    }
    #[test]
    fn far_disclosed_obstruction_approach_advances_toward_the_witness() {
        for invert in [false, true] {
            let mut frame = fixture(invert);
            let actor_index = frame.units.iter().position(|u| u.owner_slot == 0).unwrap();
            frame.units[actor_index].position = Vec2::new(84.9, 30.401608);
            let mut state = TrafficState::default();
            let advice = run(&frame, &mut state, 11, true, Vec2::ZERO);
            let actor = &frame.units[actor_index];
            let route = state.members[&actor.unit_id].local_detour.unwrap();
            let axis = (actor.target.unwrap() - actor.position).normalize();
            assert!(
                (route.route.corners[0] - actor.position).dot(axis) > 0.,
                "far approach must advance, not retreat: {:?}",
                route.route.corners
            );
            assert_eq!(
                advice.units[actor_index].target,
                Some(route.route.corners[0])
            );
            for tick in 11..=20 {
                let advice = run(&frame, &mut state, tick, true, Vec2::ZERO);
                let outputs = crate::movement::step_frame_at_tick(&advice, 0.05, tick);
                let actor = &mut frame.units[actor_index];
                let output = outputs
                    .iter()
                    .find(|out| out.unit_id == actor.unit_id)
                    .unwrap();
                for body in advice.units.iter().filter(|u| u.owner_slot == 1) {
                    assert!(
                        swept_disks_clear(
                            point(actor.position),
                            point(output.position),
                            actor.radius.into(),
                            point(body.position),
                            point(body.position),
                            body.radius.into(),
                            &mut Budget::new(1)
                        )
                        .unwrap()
                    );
                }
                actor.position = output.position;
            }
            assert!(
                frame.units[actor_index].position.x < 54.,
                "ten real return ticks must get beyond wall sight neighborhood: {:?}",
                frame.units[actor_index].position
            );
        }
    }
    #[test]
    fn after_start_foreign_removal_does_not_erase_captured_obstacle_or_advice() {
        for invert in [false, true] {
            let captured = fixture(invert);
            let mut removed = captured.clone();
            removed.units.retain(|u| u.owner_slot == 0);
            let mut original_state = TrafficState::default();
            let mut removed_state = TrafficState::default();
            let original = run_captured(
                &captured,
                &captured,
                &mut original_state,
                1,
                true,
                Vec2::ZERO,
                true,
            );
            let after = run_captured(
                &removed,
                &captured,
                &mut removed_state,
                1,
                true,
                Vec2::ZERO,
                true,
            );
            let a = original.units.iter().find(|u| u.owner_slot == 0).unwrap();
            let b = &after.units[0];
            assert_ne!(
                a.target,
                captured
                    .units
                    .iter()
                    .find(|u| u.owner_slot == 0)
                    .unwrap()
                    .target
            );
            assert_eq!(
                a.target, b.target,
                "same pinned Start still contains removed foreign sites"
            );
            assert_eq!(original_state.detour_work, removed_state.detour_work);
            assert_eq!(
                original_state.members[&a.unit_id]
                    .local_detour
                    .unwrap()
                    .route,
                removed_state.members[&b.unit_id]
                    .local_detour
                    .unwrap()
                    .route
            );
        }
    }
    #[test]
    fn incomplete_capture_drops_advice_instead_of_using_a_valid_prefix() {
        let frame = fixture(false);
        let mut state = TrafficState::default();
        run(&frame, &mut state, 1, true, Vec2::ZERO);
        assert!(state.members[&1].local_detour.is_some());
        let advice = run_captured(&frame, &frame, &mut state, 2, true, Vec2::ZERO, false);
        assert_eq!(advice.units[0].target, frame.units[0].target);
        assert!(state.members[&1].local_detour.is_none());
        assert_eq!(state.detour_work.candidates, 0);
    }
    #[test]
    fn retained_final_leg_can_skip_a_crowded_corner_with_complete_clearance() {
        let mut frame = fixture(false);
        frame.units[0].position = Vec2::new(79.45619, 30.937775);
        frame.units[0].target = Some(Vec2::new(37.6, 33.21243));
        let mut state = TrafficState::default();
        run(&frame, &mut state, 92, true, Vec2::ZERO);
        let saved = state.members[&1].local_detour.expect("captured wall route");
        frame.units[0].position = Vec2::new(72.51199, 28.44224);
        let advice = run(&frame, &mut state, 94, true, Vec2::ZERO);
        let target = advice.units[0].target.unwrap();
        assert!(
            target.x < saved.route.corners[1].x,
            "a certified continuation must pass the temporary corner: {target:?}"
        );
        for body in &frame.units[1..] {
            assert!(
                swept_disks_clear(
                    point(frame.units[0].position),
                    point(target),
                    frame.units[0].radius.into(),
                    point(body.position),
                    point(body.position),
                    body.radius.into(),
                    &mut Budget::new(1)
                )
                .unwrap()
            );
        }
        assert_eq!(
            state.members[&1].local_detour.unwrap().requested,
            frame.units[0].target.unwrap()
        );
        // A newly CURRENT obstacle on that connector must veto the shortcut.
        let mut blocked = frame.clone();
        let mut body = blocked.units[1].clone();
        body.unit_id = 9000;
        body.position = frame.units[0].position.lerp(target, 0.5);
        blocked.units.push(body);
        let blocked_advice = run(&blocked, &mut state, 95, true, Vec2::ZERO);
        assert_ne!(blocked_advice.units[0].target, Some(target));
    }
    #[test]
    fn clear_disclosed_chord_pays_no_broad_generator_search() {
        let mut frame = fixture(false);
        frame.units[0].position = Vec2::new(67.90744, 38.);
        frame.units[0].target = Some(Vec2::new(36.9, 38.));
        let mut state = TrafficState::default();
        let advice = run(&frame, &mut state, 1, true, Vec2::ZERO);
        assert_eq!(advice.units[0].target, frame.units[0].target);
        assert_eq!(state.detour_work.candidates, 0);
        assert_eq!(state.detour_work.scanned_rows, 0);
        assert_eq!(state.detour_work.body_checks, 0);
        assert_eq!(
            state.index_work.queries, 2,
            "one own discovery and one paid narrow observed query"
        );
    }
    #[test]
    fn hidden_wall_cannot_select_advice_and_sight_loss_discards_retained_generator() {
        let frame = fixture(false);
        let mut state = TrafficState::default();
        let hidden = run(&frame, &mut state, 1, false, Vec2::ZERO);
        assert_eq!(hidden.units[0].target, frame.units[0].target);
        run(&frame, &mut state, 2, true, Vec2::ZERO);
        assert!(state.members[&1].local_detour.is_some());
        let hidden = run(&frame, &mut state, 3, false, Vec2::ZERO);
        assert!(state.members[&1].local_detour.is_none());
        assert_eq!(hidden.units[0].target, frame.units[0].target);
    }
    #[test]
    fn captured_start_pose_not_live_foreign_pose_selects_advice() {
        let frame = fixture(false);
        let mut changed = fixture(false);
        for row in &mut changed.units[1..] {
            row.position += Vec2::splat(1000.);
            row.target = Some(Vec2::ZERO);
            row.radius = 50.;
        }
        let a = run(&frame, &mut TrafficState::default(), 1, true, Vec2::ZERO);
        let mut proposed = changed.clone();
        apply_with_observed_detours(
            &changed,
            &mut proposed,
            0.05,
            1,
            &mut TrafficState::default(),
            |_, _, _| true,
            |_, _, _, _| EntranceContact::Outside,
            |_| false,
            &mut crate::movement::sweep::SweepBudget::new(1),
            |viewer, id| {
                frame
                    .units
                    .iter()
                    .find(|u| u.unit_id == id && u.owner_slot != viewer)
                    .map(|u| ObservedBody {
                        position: u.position,
                        radius: u.radius,
                    })
            },
            |_| true,
            |viewer, sink| {
                for row in frame.units.iter().filter(|u| u.owner_slot != viewer) {
                    if !sink(
                        row.unit_id,
                        ObservedBody {
                            position: row.position,
                            radius: row.radius,
                        },
                    ) {
                        return false;
                    }
                }
                true
            },
        );
        assert_eq!(a.units[0].target, proposed.units[0].target);
    }
}

#[cfg(test)]
#[path = "observed_detour_wall_tests.rs"]
mod wall_tests;

#[cfg(test)]
mod deferred_intent_tests {
    use super::*;
    fn seeded(invert: bool) -> (MovementFrame, u32, Member) {
        let frame = super::tests::fixture(invert);
        let id = frame
            .units
            .iter()
            .find(|u| u.owner_slot == 0)
            .unwrap()
            .unit_id;
        let mut state = TrafficState::default();
        super::tests::run(&frame, &mut state, 1, true, Vec2::ZERO);
        let member = state.members[&id].clone();
        assert!(
            member.local_detour.is_some(),
            "seed must use a real complete route proof"
        );
        (frame, id, member)
    }
    fn fresh_state(id: u32, member: &Member) -> TrafficState {
        let mut state = TrafficState::default();
        state.members.insert(id, member.clone());
        state
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Exercise exact existing credit and capture ports"
    )]
    fn deferred(
        frame: &MovementFrame,
        captured: &MovementFrame,
        state: &mut TrafficState,
        visits: &mut [usize; SLOTS],
        quota: usize,
        budget: &mut crate::movement::sweep::SweepBudget,
        mut clear: impl FnMut(usize, Vec2, Vec2) -> bool,
    ) -> MovementFrame {
        let i = frame
            .units
            .iter()
            .position(|u| state.members.contains_key(&u.unit_id))
            .unwrap();
        let mut service = vec![i];
        let mut proposed = frame.clone();
        apply(
            frame,
            &mut proposed,
            0.05,
            2,
            state,
            &mut service,
            &BTreeSet::new(),
            visits,
            &mut clear,
            budget,
            &mut |viewer, id| {
                captured
                    .units
                    .iter()
                    .find(|u| u.unit_id == id && u.owner_slot != viewer)
                    .map(|u| ObservedBody {
                        position: u.position,
                        radius: u.radius,
                    })
            },
            &mut |_| true,
            &mut |viewer, sink| {
                captured
                    .units
                    .iter()
                    .filter(|u| u.owner_slot != viewer)
                    .all(|u| {
                        sink(
                            u.unit_id,
                            ObservedBody {
                                position: u.position,
                                radius: u.radius,
                            },
                        )
                    })
            },
            quota,
        );
        proposed
    }
    fn assert_entry(
        state: &TrafficState,
        proposed: &MovementFrame,
        id: u32,
        entry: own_detour::Retained,
    ) {
        let retained = state.members[&id]
            .local_detour
            .expect("defer witnessed intent, not its certificate");
        assert_eq!(retained.route, entry.route);
        assert_eq!(retained.corner, entry.corner);
        assert_eq!(retained.requested, entry.requested);
        assert_eq!(retained.witness, entry.witness);
        assert_eq!(
            proposed
                .units
                .iter()
                .find(|u| u.unit_id == id)
                .unwrap()
                .target,
            Some(entry.route.corners[entry.corner])
        );
    }
    #[test]
    fn zero_neighbor_or_pair_credit_preserves_entry_corner_under_renaming() {
        for invert in [false, true] {
            let (frame, id, member) = seeded(invert);
            let entry = member.local_detour.unwrap();
            for no_pairs in [false, true] {
                let mut state = fresh_state(id, &member);
                let mut visits = [0; SLOTS];
                let mut budget = crate::movement::sweep::SweepBudget::new(2);
                let quota = OWNER_VISITS;
                if no_pairs {
                    budget.pairs[0] = OWNER_VISITS;
                } else {
                    visits[0] = quota;
                }
                let before_pairs = budget.pairs;
                let before_visits = visits;
                let proposed = deferred(
                    &frame,
                    &frame,
                    &mut state,
                    &mut visits,
                    quota,
                    &mut budget,
                    |_, _, _| true,
                );
                assert_entry(&state, &proposed, id, entry);
                assert!(state.neighbor_overflows > 0);
                assert!(budget.pairs[0] >= before_pairs[0]);
                assert!(visits[0] >= before_visits[0]);
            }
        }
    }
    #[test]
    fn partial_saved_leg_service_never_publishes_a_new_cursor() {
        for invert in [false, true] {
            let (mut frame, id, member) = seeded(invert);
            let entry = member.local_detour.unwrap();
            frame
                .units
                .iter_mut()
                .find(|u| u.unit_id == id)
                .unwrap()
                .position = entry.route.corners[0];
            let mut witnessed_partial = false;
            for quota in 1..=1024 {
                let mut state = fresh_state(id, &member);
                let mut visits = [0; SLOTS];
                let mut budget = crate::movement::sweep::SweepBudget::new(2);
                let mut proved = 0;
                let proposed = deferred(
                    &frame,
                    &frame,
                    &mut state,
                    &mut visits,
                    quota,
                    &mut budget,
                    |_, _, _| {
                        proved += 1;
                        true
                    },
                );
                if proved > 0
                    && state.neighbor_overflows > 0
                    && state.members[&id].local_detour.is_some()
                {
                    assert_entry(&state, &proposed, id, entry);
                    assert!(budget.pairs[0] > 0 && visits[0] > 0);
                    witnessed_partial = true;
                    break;
                }
            }
            assert!(
                witnessed_partial,
                "must cross a real successful proof before service exhaustion"
            );
        }
    }
    #[test]
    fn complete_negative_saved_leg_wins_over_later_exhaustion() {
        let (frame, id, member) = seeded(false);
        let entry = member.local_detour.unwrap();
        let from = frame
            .units
            .iter()
            .find(|u| u.unit_id == id)
            .unwrap()
            .position;
        let mut witnessed_negative_then_exhaustion = false;
        for quota in 1..=1024 {
            let mut state = fresh_state(id, &member);
            let mut visits = [0; SLOTS];
            let mut budget = crate::movement::sweep::SweepBudget::new(2);
            let mut negative = false;
            deferred(
                &frame,
                &frame,
                &mut state,
                &mut visits,
                quota,
                &mut budget,
                |_, a, b| {
                    if a == from && b == entry.route.corners[entry.corner] {
                        negative = true;
                        false
                    } else {
                        true
                    }
                },
            );
            if negative && state.neighbor_overflows > 0 {
                assert!(state.members[&id].local_detour.is_none());
                assert!(budget.pairs[0] > 0 && visits[0] > 0);
                witnessed_negative_then_exhaustion = true;
                break;
            }
        }
        assert!(witnessed_negative_then_exhaustion);
    }
    #[test]
    fn missing_changed_witness_and_changed_identity_cannot_defer_old_intent() {
        let (frame, id, member) = seeded(false);
        let entry = member.local_detour.unwrap();
        let witness = entry.witness.unwrap().0;
        for case in 0..6 {
            let mut input = frame.clone();
            let mut captured = frame.clone();
            match case {
                0 => captured.units.retain(|u| u.unit_id != witness),
                1 => {
                    captured
                        .units
                        .iter_mut()
                        .find(|u| u.unit_id == witness)
                        .unwrap()
                        .position += Vec2::X
                }
                2 => {
                    input
                        .units
                        .iter_mut()
                        .find(|u| u.unit_id == id)
                        .unwrap()
                        .target = Some(Vec2::new(20., 20.))
                }
                3 => {
                    input
                        .units
                        .iter_mut()
                        .find(|u| u.unit_id == id)
                        .unwrap()
                        .radius = 0.4
                }
                4 => {
                    input
                        .units
                        .iter_mut()
                        .find(|u| u.unit_id == id)
                        .unwrap()
                        .unit_type += 1
                }
                _ => {
                    input
                        .units
                        .iter_mut()
                        .find(|u| u.unit_id == id)
                        .unwrap()
                        .owner_slot = 1;
                    captured = input.clone();
                }
            }
            let mut state = fresh_state(id, &member);
            let mut visits = [0; SLOTS];
            let mut budget = crate::movement::sweep::SweepBudget::new(2);
            deferred(
                &input,
                &captured,
                &mut state,
                &mut visits,
                0,
                &mut budget,
                |_, _, _| true,
            );
            assert!(state.members[&id].local_detour.is_none(), "case{case}");
        }
    }
    #[test]
    fn complete_direct_clearance_and_completed_route_release_intent() {
        let (mut frame, id, member) = seeded(false);
        frame
            .units
            .iter_mut()
            .find(|u| u.unit_id == id)
            .unwrap()
            .position = Vec2::new(67.90744, 38.);
        let mut state = fresh_state(id, &member);
        let mut visits = [0; SLOTS];
        let mut budget = crate::movement::sweep::SweepBudget::new(2);
        let proposed = deferred(
            &frame,
            &frame,
            &mut state,
            &mut visits,
            OWNER_VISITS,
            &mut budget,
            |_, _, _| true,
        );
        assert!(state.members[&id].local_detour.is_none());
        assert_eq!(
            proposed
                .units
                .iter()
                .find(|u| u.unit_id == id)
                .unwrap()
                .target,
            frame.units.iter().find(|u| u.unit_id == id).unwrap().target
        );
        let mut member = member;
        member.local_detour.as_mut().unwrap().corner = 1;
        frame
            .units
            .iter_mut()
            .find(|u| u.unit_id == id)
            .unwrap()
            .position = member.local_detour.unwrap().route.corners[1];
        let mut state = fresh_state(id, &member);
        let mut visits = [0; SLOTS];
        deferred(
            &frame,
            &frame,
            &mut state,
            &mut visits,
            0,
            &mut budget,
            |_, _, _| true,
        );
        assert!(state.members[&id].local_detour.is_none());
    }
    #[test]
    fn actual_same_coordinate_order_change_forgets_retained_route_before_early_return() {
        let (frame, id, member) = seeded(false);
        let same_target = frame.units.iter().find(|u| u.unit_id == id).unwrap().target;
        let mut world = bevy_ecs::world::World::new();
        world.insert_resource(fresh_state(id, &member));
        crate::terrain::navigation::order_changed(&mut world, id);
        assert!(!world.resource::<TrafficState>().members.contains_key(&id));
        assert_eq!(
            frame.units.iter().find(|u| u.unit_id == id).unwrap().target,
            same_target
        );
    }
}

#[cfg(test)]
mod saved_leg_negative_tests {
    use super::*;
    fn row(id: u32, position: Vec2) -> Row {
        Row {
            id,
            body: ObservedBody {
                position,
                radius: 0.3,
            },
            foreign: true,
        }
    }
    #[test]
    fn first_body_veto_survives_one_credit_without_inspecting_later_rows() {
        let rows = [row(1, Vec2::new(11., 10.)), row(2, Vec2::new(11., 12.))];
        let mut budget = crate::movement::sweep::SweepBudget::new(2);
        budget.pairs[0] = OWNER_VISITS - 1;
        let mut pairs = 0;
        let mut work = detour::Work::default();
        assert_eq!(
            certify(
                Vec2::splat(10.),
                Vec2::new(12., 10.),
                0.3,
                &rows,
                &mut budget,
                0,
                &mut pairs,
                &mut work
            ),
            Some(false)
        );
        assert_eq!(pairs, 1);
        assert_eq!(work.body_checks, 1);
        assert_eq!(budget.pairs[0], OWNER_VISITS);
        // The completed veto is retained; no uncharged positive row can turn
        // this into the Unknown result that restores deferred entry intent.
    }
    #[test]
    fn clear_first_body_with_second_uncharged_remains_unknown() {
        let rows = [row(1, Vec2::new(11., 12.)), row(2, Vec2::new(11., 10.))];
        let mut budget = crate::movement::sweep::SweepBudget::new(2);
        budget.pairs[0] = OWNER_VISITS - 1;
        let mut pairs = 0;
        let mut work = detour::Work::default();
        assert_eq!(
            certify(
                Vec2::splat(10.),
                Vec2::new(12., 10.),
                0.3,
                &rows,
                &mut budget,
                0,
                &mut pairs,
                &mut work
            ),
            None
        );
        assert_eq!(pairs, 1);
        assert_eq!(work.body_checks, 1);
        assert_eq!(budget.pairs[0], OWNER_VISITS);
    }
    #[test]
    fn canonical_extraction_keeps_one_credit_veto_independent_of_handles_and_enumeration() {
        for veto_first in [false, true] {
            let mut outcomes = Vec::new();
            for invert in [false, true] {
                let mut rows = vec![
                    row(1, Vec2::new(11., 10.)),
                    row(
                        2,
                        if veto_first {
                            Vec2::new(11., 12.)
                        } else {
                            Vec2::new(10.5, 12.)
                        },
                    ),
                ];
                if invert {
                    rows.reverse();
                    for row in &mut rows {
                        row.id = 1000 - row.id;
                    }
                }
                let mut index = EnvelopeIndex::new(rows.len(), |i| {
                    Some(region(
                        rows[i].body.position,
                        rows[i].body.position,
                        f64::from(rows[i].body.radius),
                    ))
                })
                .unwrap();
                let mut visits = 0;
                let mut work = detour::Work::default();
                let mut budget = crate::movement::sweep::SweepBudget::new(2);
                budget.pairs[0] = OWNER_VISITS - 1;
                let extracted = query(
                    &mut index,
                    &rows,
                    region(Vec2::splat(10.), Vec2::new(12., 10.), 3.),
                    &mut visits,
                    OWNER_VISITS,
                    &mut work,
                )
                .unwrap();
                assert!(work.generator_comparisons > 0);
                assert_eq!(
                    budget.pairs[0],
                    OWNER_VISITS - 1,
                    "metadata ordering cannot debit/refill pair credit"
                );
                let mut pairs = 0;
                let result = certify(
                    Vec2::splat(10.),
                    Vec2::new(12., 10.),
                    0.3,
                    &extracted,
                    &mut budget,
                    0,
                    &mut pairs,
                    &mut work,
                );
                assert_eq!(result, if veto_first { Some(false) } else { None });
                assert_eq!(pairs, 1);
                assert_eq!(work.body_checks, 1);
                outcomes.push((result, pairs, work.body_checks, visits, budget.pairs[0]));
            }
            assert_eq!(outcomes[0], outcomes[1]);
        }
    }
}
