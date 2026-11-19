//! Single-unit progress after a legacy move meets an obstacle.

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use super::arrival::SpatialNeighbors;
use super::frame::{MovementFrameOutput, MovementFrameUnit};
use super::search::{MovementSearch, body_overlaps_position};
use super::spread::{SpreadContext, nearest_angle_order, spread_destinations_with_filter};
use super::{MovementActivity, MovementInput, PhysicalEscape, PlainGatherGoal, SpreadMember, step};

/// Default-omitted progress for a single legacy move that has encountered an obstacle.
/// The candidate pool belongs to this unit; it deliberately has no shared pool ID.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainMoveProgress {
    /// Temporary clearance for an approaching earlier arrival. Reconsider until it arrives.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub arrival_yield: bool,
    pub original_point: Vec2,
    pub slot: Vec2,
    pub pool_slots: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_goal: Option<PlainGatherGoal>,
    pub activity: MovementActivity,
    pub stalled_ticks: u16,
    pub best_distance: f32,
    pub claim_count: u8,
    pub failure_recorded: bool,
}

#[expect(
    clippy::too_many_arguments,
    reason = "the shared frame kernel passes immutable index inputs without rebuilding them"
)]
pub(super) fn step_plain_progress(
    bounds: Vec2,
    tick_hz: u16,
    max_radius: f32,
    dt: f32,
    index: usize,
    unit: &MovementFrameUnit,
    mut progress: PlainMoveProgress,
    units: &[MovementFrameUnit],
    points: &[crate::grid::GridPoint],
    grid: &crate::grid::UniformGrid,
    spread_context: &SpreadContext,
    known: &dyn Fn(u32) -> bool,
    guide: &mut dyn super::motion::MotionGuide,
    budget: &mut super::sweep::SweepBudget,
) -> MovementFrameOutput {
    let mut output = MovementFrameOutput {
        unit_id: unit.unit_id,
        position: unit.position,
        target: unit.target,
        progress: unit.progress.clone(),
        plain_progress: Some(progress.clone()),
        physical_escape: unit.physical_escape.clone(),
        failed_blocked: false,
        arrival_slot: None,
        #[cfg(feature = "test-probes")]
        probe: super::probe::FrameFlags::default(),
    };
    let Some(speed) = unit.speed else {
        output.target = None;
        output.plain_progress = Some(progress);
        return output;
    };
    if progress.activity == MovementActivity::Resting {
        output.target = None;
        output.plain_progress = Some(progress);
        return output;
    }
    let step_limit = speed * dt;
    let body_occupied = |point: Vec2| {
        grid.within_unsorted(points, point, unit.radius + max_radius)
            .into_iter()
            .any(|other_index| {
                other_index != index
                    && known(units[other_index].unit_id)
                    && point.distance(units[other_index].position)
                        < unit.radius + units[other_index].radius - 1.0e-5
            })
    };
    let occupied = |point: Vec2| {
        spread_context
            .conflicts_with_reserved_filtered(point, unit.radius, unit.unit_id, &|id| !known(id))
            || body_occupied(point)
    };

    if progress.activity == MovementActivity::Blocked {
        let escape = unit.target.filter(|target| !occupied(*target)).or_else(|| {
            if !body_occupied(unit.position) {
                return None;
            }
            MovementSearch {
                bounds,
                units,
                points,
                grid,
                max_radius,
            }
            .find_free_singleton_destination(unit.position, unit.radius, index)
        });
        let Some(escape) = escape else {
            output.target = None;
            output.plain_progress = Some(progress);
            return output;
        };
        let offset = escape - unit.position;
        let distance = offset.length();
        // Exhausting local claims does not exempt a clear body from current
        // bounded steering. Initial-overlap recovery keeps its original path.
        // Guide output still receives the unchanged final exact certificates.
        let guided = if body_occupied(unit.position) {
            None
        } else {
            super::motion::guided_position(unit, escape, bounds, dt, guide, budget)
        };
        let position = guided.unwrap_or_else(|| {
            if distance <= step_limit {
                escape
            } else if distance > 1.0e-8 {
                unit.position + offset * (step_limit / distance)
            } else {
                unit.position
            }
        });
        let reached = position == escape && !occupied(escape);
        output.position = position;
        output.target = (!reached).then_some(escape);
        progress.slot = escape;
        output.plain_progress = Some(progress);
        return output;
    }

    let movement_target = progress.slot;
    let distance = unit.position.distance(movement_target);
    let guided = super::motion::guided_position(unit, movement_target, bounds, dt, guide, budget);
    if guided.is_none()
        && (distance <= step_limit || distance <= 0.025)
        && !occupied(movement_target)
    {
        output.position = movement_target;
        output.target = progress.arrival_yield.then_some(movement_target);
        if progress.arrival_yield {
            progress.activity = MovementActivity::Moving;
            output.plain_progress = Some(progress);
            return output;
        }
        if progress.claim_count > 0 || progress.slot != progress.original_point {
            progress.activity = MovementActivity::Resting;
            output.plain_progress = Some(progress);
        } else {
            output.plain_progress = None;
        }
        return output;
    }

    let position = if let Some(position) = guided {
        position
    } else {
        let desired = movement_target - unit.position;
        // Occupancy prevents claiming the waypoint, not stopping at its distance.
        // A full speed step past a nearby occupied point creates an unrelated chord
        // that the continuous body guard must reject. Separation/projection follow.
        let mut velocity = desired.try_normalize().unwrap_or(Vec2::ZERO) * step_limit.min(distance);
        let mut separated = false;
        for other_index in grid.nearby(
            points,
            unit.position,
            super::frame::separation_reach(
                unit.radius,
                max_radius,
                unit.radius + max_radius + 0.05 + step_limit * 1.2,
            ),
        ) {
            if other_index == index {
                continue;
            }
            let other = &units[other_index];
            let offset = unit.position - other.position;
            let separation = offset.length();
            let limit = unit.radius + other.radius + 0.05;
            if separation > 1.0e-8 && separation < limit {
                separated = true;
                velocity += offset / separation * (limit - separation);
                if offset.dot(desired) < 0.0 {
                    let along = desired.try_normalize().unwrap_or(Vec2::X);
                    velocity += Vec2::new(along.y, -along.x) * (limit - separation);
                }
            }
        }
        #[cfg(feature = "test-probes")]
        {
            output.probe.separated = separated;
        }
        if !separated && distance > step_limit {
            velocity = step(
                &MovementInput {
                    position: unit.position,
                    target: movement_target,
                    speed,
                },
                dt,
            )
            .position
                - unit.position;
        } else if velocity.length() > step_limit && step_limit > 0.0 {
            velocity = velocity.normalize() * step_limit;
        }
        let mut position = unit.position + velocity;
        #[cfg(feature = "test-probes")]
        let mut was_projected = false;
        for _ in 0..8 {
            let mut projected = false;
            for other_index in grid.nearby(
                points,
                position,
                unit.radius + max_radius + step_limit * 1.2,
            ) {
                if other_index == index {
                    continue;
                }
                let other = &units[other_index];
                let offset = position - other.position;
                let separation = offset.length();
                let limit = unit.radius + other.radius + 1.0e-4;
                if separation < limit {
                    let direction = if separation > 1.0e-8 {
                        offset / separation
                    } else {
                        super::arrival::separation_direction(unit, other)
                    };
                    position += direction * (limit - separation);
                    projected = true;
                }
            }
            if !projected {
                break;
            }
            #[cfg(feature = "test-probes")]
            {
                was_projected = true;
            }
        }
        position.x = position
            .x
            .clamp(unit.radius, (bounds.x - unit.radius).max(unit.radius));
        position.y = position
            .y
            .clamp(unit.radius, (bounds.y - unit.radius).max(unit.radius));
        // Eight projection passes can cycle between neighbours. Never commit an overlap
        // when the body still has its previous legal position.
        if body_overlaps_position(
            position,
            index,
            unit.radius,
            units,
            points,
            grid,
            max_radius,
        ) && !body_overlaps_position(
            unit.position,
            index,
            unit.radius,
            units,
            points,
            grid,
            max_radius,
        ) {
            position = unit.position;
        }
        #[cfg(feature = "test-probes")]
        {
            output.probe.was_projected = Some(was_projected);
        }

        position
    };
    let next_distance = position.distance(movement_target);
    if position == movement_target && !occupied(movement_target) {
        output.position = position;
        if progress.arrival_yield {
            output.target = Some(movement_target);
            progress.activity = MovementActivity::Moving;
            output.plain_progress = Some(progress);
            return output;
        }
        output.target = None;
        if progress.claim_count > 0 || progress.slot != progress.original_point {
            progress.activity = MovementActivity::Resting;
            output.plain_progress = Some(progress);
        } else {
            output.plain_progress = None;
        }
        return output;
    }
    // Preserve an already due certified recovery across an optimistic raw step.
    let recovery_due = progress.stalled_ticks >= tick_hz.max(1);
    let made_progress = next_distance < progress.best_distance - 0.01 * step_limit;
    if made_progress && !recovery_due {
        progress.best_distance = next_distance;
        progress.stalled_ticks = 0;
    } else {
        progress.stalled_ticks = progress.stalled_ticks.saturating_add(1);
    }
    if progress.stalled_ticks >= tick_hz.max(1) {
        let spare = if progress.claim_count < 3 {
            progress
                .pool_slots
                .iter()
                .copied()
                .filter(|candidate| *candidate != progress.slot && !occupied(*candidate))
                .min_by(|a, b| nearest_angle_order(*a, *b, unit.position))
        } else {
            None
        };
        if let Some(spare) = spare {
            progress
                .pool_slots
                .retain(|candidate| *candidate != progress.slot && *candidate != spare);
            progress.slot = spare;
            progress.claim_count += 1;
            progress.stalled_ticks = 0;
            progress.best_distance = position.distance(spare);
            output.target = Some(spare);
        } else {
            progress.activity = MovementActivity::Blocked;
            output.target = None;
            output.failed_blocked = true;
            if body_overlaps_position(
                position,
                index,
                unit.radius,
                units,
                points,
                grid,
                max_radius,
            ) {
                output.physical_escape = Some(PhysicalEscape { target: None });
            }
        }
    } else {
        output.target = Some(movement_target);
    }
    output.position = position;
    output.plain_progress = Some(progress);
    output
}

pub(super) fn make_plain_progress(
    unit: &MovementFrameUnit,
    original_point: Vec2,
    context: &SpreadContext,
    known: &dyn Fn(u32) -> bool,
) -> PlainMoveProgress {
    let member = [SpreadMember {
        id: unit.unit_id,
        position: unit.position,
        radius: unit.radius,
    }];
    let (pool, _) = spread_destinations_with_filter(&member, original_point, context, &|id| {
        id == unit.unit_id || !known(id)
    });
    let slot = pool.slots.first().copied().unwrap_or(original_point);
    let pool_slots = pool.slots.into_iter().skip(1).take(6).collect();
    PlainMoveProgress {
        arrival_yield: false,
        original_point,
        slot,
        pool_slots,
        gather_goal: None,
        activity: if pool.primary_count == 0 {
            MovementActivity::Blocked
        } else {
            MovementActivity::Moving
        },
        stalled_ticks: 0,
        best_distance: unit.position.distance(slot),
        claim_count: 0,
        failure_recorded: false,
    }
}

#[cfg(test)]
mod approach_tests {
    use super::*;
    #[test]
    fn occupied_near_waypoint_does_not_turn_a_short_approach_into_a_full_step_past_it() {
        use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};
        let mut positions = Vec::new();
        for invert in [false, true] {
            let target = Vec2::new(3., 5.);
            let actor = MovementFrameUnit {
                unit_id: if invert { 99 } else { 1 },
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(2., 5.),
                radius: 0.3,
                speed: Some(100.),
                target: Some(target),
                progress: None,
                plain_progress: None,
                physical_escape: None,
            };
            let mut holder = actor.clone();
            holder.unit_id = if invert { 1 } else { 99 };
            holder.position = target;
            holder.target = Some(Vec2::new(10., 5.));
            let units = if invert {
                vec![holder.clone(), actor.clone()]
            } else {
                vec![actor.clone(), holder.clone()]
            };
            let index = usize::from(invert);
            let points = units
                .iter()
                .map(|u| crate::grid::GridPoint {
                    id: u.unit_id,
                    position: u.position,
                })
                .collect::<Vec<_>>();
            let grid = crate::grid::UniformGrid::new(&points, 0.6);
            let context = SpreadContext::new(vec![], vec![], Vec2::splat(16.), 0.3);
            let progress = PlainMoveProgress {
                arrival_yield: false,
                original_point: target,
                slot: target,
                pool_slots: vec![],
                gather_goal: None,
                activity: MovementActivity::Moving,
                stalled_ticks: 0,
                best_distance: 1.,
                claim_count: 0,
                failure_recorded: false,
            };
            let out = step_plain_progress(
                Vec2::splat(16.),
                20,
                0.3,
                0.05,
                index,
                &units[index],
                progress,
                &units,
                &points,
                &grid,
                &context,
                &|_| true,
                &mut super::super::motion::NoMotionGuide,
                &mut super::super::sweep::SweepBudget::new(1),
            );
            assert!(
                out.position.x <= target.x,
                "near occupied waypoint must not produce a five-metre overshoot: {:?}",
                out.position
            );
            let p = |v: Vec2| Point {
                x: f64::from(v.x),
                y: f64::from(v.y),
            };
            assert!(
                swept_disks_clear(
                    p(actor.position),
                    p(out.position),
                    actor.radius.into(),
                    p(holder.position),
                    p(holder.position),
                    holder.radius.into(),
                    &mut Budget::new(1)
                )
                .unwrap()
            );
            assert_eq!(out.target, Some(target));
            assert_eq!(out.plain_progress.as_ref().unwrap().original_point, target);
            positions.push(out.position);
        }
        assert_eq!(positions[0], positions[1]);
    }
}
