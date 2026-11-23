//! Packed stop behind the holder (operator decision 2026-11-22).
//! Spec: `docs/superpowers/specs/2026-11-15-terrain-group-movement.md`, section
//! "Packed stop behind the holder". A late unit that stopped friendly bodies box
//! out ends its move where it stands. Every input is a start-of-tick frame row:
//! positions, radii, owners and stopped state. No unit id decides a result.
use crate::movement::{MovementFrame, MovementFrameUnit, route::CONTACT_GAP, sweep::pair_clear};
use bevy_math::Vec2;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PackedStop {
    /// The earlier rules stay in force for this tick.
    NotApplicable,
    /// The unit ends its move at its start-of-tick pose.
    Stop,
    /// A free step nearer the point exists. Its end becomes the private stop slot.
    Step(Vec2),
}

/// A stopped friendly body: the owner of the actor, no move target, no escape.
fn stopped_friend(actor: &MovementFrameUnit, other: &MovementFrameUnit) -> bool {
    other.unit_id != actor.unit_id
        && other.owner_slot == actor.owner_slot
        && other.target.is_none()
        && other.physical_escape.is_none()
        && other.position.is_finite()
        && other.radius.is_finite()
        && other.radius > 0.
}

fn gap(a: &MovementFrameUnit, b: &MovementFrameUnit) -> f32 {
    a.position.distance(b.position) - a.radius - b.radius
}

/// Start-of-tick rows in a uniform hash. It is built one time for each tick, and
/// only when one or more units have a complete held clock.
pub(super) struct PackedFrame<'a> {
    frame: &'a MovementFrame,
    cell: f32,
    max_radius: f32,
    cells: HashMap<(i32, i32), Vec<u32>>,
    /// One pack for each (owner, point, actor radius): `None` is a refused build.
    packs: Vec<(PackKey, Option<Vec<bool>>)>,
}
type PackKey = (u8, [u32; 2], u32);

impl<'a> PackedFrame<'a> {
    pub(super) fn new(frame: &'a MovementFrame) -> Self {
        let max_radius = frame
            .units
            .iter()
            .map(|unit| unit.radius)
            .filter(|radius| radius.is_finite() && *radius > 0.)
            .fold(0.0f32, f32::max);
        // Two packed bodies of the largest radius, with the largest actor between,
        // are in the same or in adjacent cells.
        let cell = (4. * max_radius).max(1.);
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (index, unit) in frame.units.iter().enumerate() {
            if unit.position.is_finite() {
                cells
                    .entry(Self::key(cell, unit.position))
                    .or_default()
                    .push(index as u32);
            }
        }
        Self {
            frame,
            cell,
            max_radius,
            cells,
            packs: Vec::new(),
        }
    }
    fn key(cell: f32, position: Vec2) -> (i32, i32) {
        (
            (position.x / cell).floor() as i32,
            (position.y / cell).floor() as i32,
        )
    }
    /// Rows whose centre is within `reach` of `position`. `visit` returns false to stop.
    fn near(&self, position: Vec2, reach: f32, mut visit: impl FnMut(usize) -> bool) -> bool {
        let low = Self::key(self.cell, position - Vec2::splat(reach));
        let high = Self::key(self.cell, position + Vec2::splat(reach));
        for x in low.0..=high.0 {
            for y in low.1..=high.1 {
                for &index in self.cells.get(&(x, y)).into_iter().flatten() {
                    if !visit(index as usize) {
                        return false;
                    }
                }
            }
        }
        true
    }
    /// The stopped friendly bodies that connect to a holder through packed pairs.
    fn pack(
        &mut self,
        actor: &MovementFrameUnit,
        point: Vec2,
        charge: &mut dyn FnMut() -> bool,
    ) -> Option<&[bool]> {
        let key = (
            actor.owner_slot,
            point.to_array().map(f32::to_bits),
            actor.radius.to_bits(),
        );
        if let Some(at) = self.packs.iter().position(|row| row.0 == key) {
            return self.packs[at].1.as_deref();
        }
        let built = self.build(actor, point, charge);
        self.packs.push((key, built));
        self.packs.last().unwrap().1.as_deref()
    }
    fn build(
        &self,
        actor: &MovementFrameUnit,
        point: Vec2,
        charge: &mut dyn FnMut() -> bool,
    ) -> Option<Vec<bool>> {
        let units = &self.frame.units;
        let mut member = vec![false; units.len()];
        let mut queue = Vec::new();
        // Holders: stopped friendly bodies that occupy the point.
        let complete = self.near(point, actor.radius + self.max_radius, |index| {
            if !charge() {
                return false;
            }
            let other = &units[index];
            if stopped_friend(actor, other)
                && point.distance(other.position) < actor.radius + other.radius
            {
                member[index] = true;
                queue.push(index);
            }
            true
        });
        if !complete || queue.is_empty() {
            return None;
        }
        let passable = 2. * actor.radius;
        while let Some(at) = queue.pop() {
            let body = &units[at];
            let reach = body.radius + passable + self.max_radius;
            let complete = self.near(body.position, reach, |index| {
                if member[index] {
                    return true;
                }
                if !charge() {
                    return false;
                }
                let other = &units[index];
                if stopped_friend(actor, other) && gap(body, other) < passable {
                    member[index] = true;
                    queue.push(index);
                }
                true
            });
            if !complete {
                // A refused charge leaves the earlier rules in force for this tick.
                return None;
            }
        }
        Some(member)
    }
}

/// Conditions 2 to 5 of the rule. Condition 1 (the held clock) is the caller's.
/// `terrain_clear` is the existing swept terrain certificate of one step.
/// `charge` debits the existing per-owner movement work allowance.
pub(super) fn packed_stop(
    packed: &mut PackedFrame<'_>,
    actor: &MovementFrameUnit,
    point: Vec2,
    terrain_clear: &mut dyn FnMut(Vec2, Vec2) -> bool,
    charge: &mut dyn FnMut() -> bool,
) -> PackedStop {
    let position = actor.position;
    if !position.is_finite()
        || !point.is_finite()
        || !actor.radius.is_finite()
        || actor.radius <= 0.
    {
        return PackedStop::NotApplicable;
    }
    let to_point = point - position;
    let distance = to_point.length();
    if distance <= CONTACT_GAP {
        return PackedStop::NotApplicable;
    }
    let ahead = to_point / distance;
    let units = &packed.frame.units;
    // Bodies in contact with the actor: any owner, stopped or not.
    let mut contacts = Vec::new();
    let reach = actor.radius + packed.max_radius + CONTACT_GAP;
    let complete = packed.near(position, reach, |index| {
        if !charge() {
            return false;
        }
        let other = &units[index];
        if other.unit_id != actor.unit_id
            && other.radius.is_finite()
            && other.radius > 0.
            && gap(actor, other) <= CONTACT_GAP
        {
            contacts.push(index);
        }
        true
    });
    if !complete || contacts.is_empty() {
        return PackedStop::NotApplicable;
    }
    let Some(pack) = packed.pack(actor, point, charge) else {
        return PackedStop::NotApplicable;
    };
    // Condition 2: contact with the pack. Condition 3: each body in contact
    // ahead of the actor is a body of the pack.
    if !contacts.iter().any(|&index| pack[index])
        || contacts
            .iter()
            .any(|&index| !pack[index] && (units[index].position - position).dot(ahead) > 0.)
    {
        return PackedStop::NotApplicable;
    }
    // Condition 4: terrain does not hold the unit on the direct step.
    let length = actor.radius;
    let direct = position + ahead * length.min(distance);
    if !terrain_clear(position, direct) {
        return PackedStop::NotApplicable;
    }
    // Condition 5, the guard: a free step that ends nearer the point.
    let mut steps = vec![direct];
    for &index in &contacts {
        let away = units[index].position - position;
        let tangent = Vec2::new(-away.y, away.x).normalize_or_zero();
        if tangent != Vec2::ZERO {
            steps.push(position + tangent * length);
            steps.push(position - tangent * length);
        }
    }
    let mut best: Option<(f32, Vec2)> = None;
    for end in steps {
        let remaining = end.distance(point);
        if !end.is_finite() || remaining >= distance - CONTACT_GAP {
            continue;
        }
        let mut free = terrain_clear(position, end);
        if free {
            let reach = actor.radius + length + packed.max_radius;
            let complete = packed.near(position, reach, |index| {
                if !charge() {
                    return false;
                }
                let other = &units[index];
                if other.unit_id != actor.unit_id
                    && other.radius.is_finite()
                    && other.radius > 0.
                    && !pair_clear(
                        position,
                        end,
                        actor.radius,
                        other.position,
                        other.position,
                        other.radius,
                    )
                {
                    free = false;
                }
                free
            });
            if !complete && free {
                // A refused charge: the guard has no answer, so the unit does not stop.
                return PackedStop::NotApplicable;
            }
        }
        // A tie between two steps is broken by geometry (the pose), never by an id.
        let better = best.is_none_or(|(least, pose)| {
            remaining < least
                || (remaining == least
                    && end.to_array().map(f32::to_bits) < pose.to_array().map(f32::to_bits))
        });
        if free && better {
            best = Some((remaining, end));
        }
    }
    match best {
        Some((_, end)) => PackedStop::Step(end),
        None => PackedStop::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const R: f32 = 0.4;
    const POINT: Vec2 = Vec2::new(10., 10.);

    fn body(id: u32, owner: u8, position: Vec2, target: Option<Vec2>) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: owner,
            unit_type: 0,
            position,
            radius: R,
            speed: target.map(|_| 3.5),
            target,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn frame(units: Vec<MovementFrameUnit>) -> MovementFrame {
        MovementFrame {
            recovery: Default::default(),
            units,
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            pools: BTreeMap::new(),
        }
    }
    /// A holder on the point and a first row of three stopped bodies on its west
    /// side. The row leaves no gap that a body of radius R can pass.
    fn wall(ids: impl Fn(u32) -> u32) -> Vec<MovementFrameUnit> {
        vec![
            body(ids(1), 0, POINT, None),
            body(ids(2), 0, POINT + Vec2::new(-0.8, 0.), None),
            body(ids(3), 0, POINT + Vec2::new(-0.6, 0.75), None),
            body(ids(4), 0, POINT + Vec2::new(-0.6, -0.75), None),
        ]
    }
    /// The late unit, in contact behind the middle body of the first row.
    fn late(id: u32) -> MovementFrameUnit {
        body(id, 0, POINT + Vec2::new(-1.6, 0.), Some(POINT))
    }
    fn decide(units: Vec<MovementFrameUnit>, actor: &MovementFrameUnit) -> PackedStop {
        decide_with(units, actor, &mut |_, _| true, usize::MAX)
    }
    fn decide_with(
        units: Vec<MovementFrameUnit>,
        actor: &MovementFrameUnit,
        terrain: &mut dyn FnMut(Vec2, Vec2) -> bool,
        mut allowance: usize,
    ) -> PackedStop {
        let frame = frame(units);
        let mut packed = PackedFrame::new(&frame);
        packed_stop(&mut packed, actor, POINT, terrain, &mut || {
            allowance = allowance.saturating_sub(1);
            allowance > 0
        })
    }

    #[test]
    fn a_late_unit_boxed_out_in_the_second_row_stops() {
        for reverse in [false, true] {
            let ids = |id: u32| if reverse { 100 - id } else { id };
            let actor = late(ids(9));
            let mut units = wall(ids);
            units.push(actor.clone());
            if reverse {
                units.reverse();
            }
            assert_eq!(decide(units, &actor), PackedStop::Stop, "reverse {reverse}");
        }
    }
    #[test]
    fn a_free_gap_nearer_the_point_must_be_reached_first() {
        // The actor touches one body of the pack from the side. The place beside
        // the holder is free: a step around the body in contact ends nearer the point.
        let actor = body(9, 0, POINT + Vec2::new(-0.8, 0.8), Some(POINT));
        let mut units = vec![
            body(1, 0, POINT, None),
            body(2, 0, POINT + Vec2::new(-0.8, 0.), None),
            actor.clone(),
        ];
        let PackedStop::Step(end) = decide(units.clone(), &actor) else {
            panic!("the unit stopped at first contact with a free gap ahead");
        };
        assert!(end.distance(POINT) < actor.position.distance(POINT) - CONTACT_GAP);
        // Control: a stopped body in that place closes the gap, and the unit stops.
        units.push(body(3, 0, POINT + Vec2::new(0., 0.85), None));
        assert_eq!(decide(units, &actor), PackedStop::Stop);
    }
    #[test]
    fn terrain_that_holds_the_unit_keeps_the_earlier_rules() {
        let actor = late(9);
        let mut units = wall(|id| id);
        units.push(actor.clone());
        // The direct step is refused by terrain: condition 4.
        let direct = actor.position + Vec2::new(R, 0.);
        let result = decide_with(
            units,
            &actor,
            &mut |_, to| to.distance(direct) > 1.0e-4,
            usize::MAX,
        );
        assert_eq!(result, PackedStop::NotApplicable);
    }
    #[test]
    fn a_pack_of_another_owner_is_never_used() {
        let actor = late(9);
        let mut units = wall(|id| id);
        for unit in &mut units {
            unit.owner_slot = 1;
        }
        units.push(actor.clone());
        assert_eq!(decide(units, &actor), PackedStop::NotApplicable);
    }
    #[test]
    fn a_moving_body_in_contact_ahead_keeps_the_earlier_rules() {
        let actor = late(9);
        let mut units = wall(|id| id);
        // The middle body of the first row has a target: it is not stopped.
        units[1].target = Some(POINT + Vec2::new(5., 5.));
        units.push(actor.clone());
        assert_eq!(decide(units, &actor), PackedStop::NotApplicable);
    }
    #[test]
    fn a_stopped_body_ahead_that_does_not_connect_to_the_holder_is_not_a_pack() {
        // One stopped friendly body far from the holder; the actor touches only it.
        let far = body(2, 0, POINT + Vec2::new(-6., 0.), None);
        let actor = body(9, 0, POINT + Vec2::new(-6.8, 0.), Some(POINT));
        let units = vec![body(1, 0, POINT, None), far, actor.clone()];
        assert_eq!(decide(units, &actor), PackedStop::NotApplicable);
    }
    #[test]
    fn no_holder_on_the_point_keeps_the_earlier_rules() {
        let actor = late(9);
        let mut units = wall(|id| id);
        units.remove(0);
        units.push(actor.clone());
        assert_eq!(decide(units, &actor), PackedStop::NotApplicable);
    }
    #[test]
    fn inputs_are_start_of_tick_rows_so_a_row_behind_waits_one_tick() {
        // B is in contact with A only. A still has its target at the start of the tick.
        for reverse in [false, true] {
            let ids = |id: u32| if reverse { 100 - id } else { id };
            let a = late(ids(9));
            let b = body(ids(10), 0, POINT + Vec2::new(-2.4, 0.), Some(POINT));
            let mut units = wall(ids);
            units.push(a.clone());
            units.push(b.clone());
            assert_eq!(decide(units.clone(), &a), PackedStop::Stop);
            assert_eq!(decide(units.clone(), &b), PackedStop::NotApplicable);
            // The next tick: A is a stopped body, and B can stop.
            let at = units.iter().position(|u| u.unit_id == a.unit_id).unwrap();
            units[at].target = None;
            assert_eq!(decide(units, &b), PackedStop::Stop, "reverse {reverse}");
        }
    }
    #[test]
    fn a_large_pack_is_bounded_by_the_work_allowance_only() {
        // 121 stopped bodies in a square block around the point, and one late unit
        // in contact with the west face.
        let mut units = Vec::new();
        for x in -5..=5 {
            for y in -5..=5 {
                let at = POINT + Vec2::new(x as f32 * 0.8, y as f32 * 0.8);
                units.push(body(units.len() as u32 + 1, 0, at, None));
            }
        }
        let actor = body(900, 0, POINT + Vec2::new(-4.8, 0.), Some(POINT));
        units.push(actor.clone());
        assert_eq!(decide(units.clone(), &actor), PackedStop::Stop);
        // A refused charge leaves the earlier rules in force.
        let refused = decide_with(units, &actor, &mut |_, _| true, 40);
        assert_eq!(refused, PackedStop::NotApplicable);
    }
}
