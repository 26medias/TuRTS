//! The movement frame step.

use bevy_math::Vec2;
use std::collections::BTreeMap;

use super::fallback::{FallbackRequest, fallback_pool};
use super::plain::{PlainMoveProgress, step_plain_progress};
use super::pool_index::{
    MovementPoolIndex, add_pool_slot_claim, refresh_nearby_pool_slot_clearances,
    remove_pool_slot_claim,
};
use super::search::{MovementSearch, body_overlaps_position};
use super::spread::{SpreadContext, nearest_angle_order, target_key, update_target_claims};
use super::{
    MoveProgress, MovementActivity, MovementInput, MovementPool, PhysicalEscape, ReservedFootprint,
    StaticFootprint, step,
};

/// One immutable unit row consumed by deterministic group movement.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrameUnit {
    pub unit_id: u32,
    pub owner_slot: u8,
    pub unit_type: u32,
    pub position: Vec2,
    pub radius: f32,
    pub speed: Option<f32>,
    pub target: Option<Vec2>,
    pub progress: Option<MoveProgress>,
    pub plain_progress: Option<PlainMoveProgress>,
    pub physical_escape: Option<PhysicalEscape>,
}

/// All movement-relevant state for one fixed tick. Both the sim and log validator use this input.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrame {
    pub bounds: Vec2,
    pub tick_hz: u16,
    pub units: Vec<MovementFrameUnit>,
    pub pools: BTreeMap<u64, MovementPool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MovementFrameOutput {
    pub unit_id: u32,
    pub position: Vec2,
    pub target: Option<Vec2>,
    pub progress: Option<MoveProgress>,
    pub plain_progress: Option<PlainMoveProgress>,
    pub physical_escape: Option<PhysicalEscape>,
    pub failed_blocked: bool,
    /// The flags of the last step of this unit. Tests read them through `probe`.
    #[cfg(feature = "test-probes")]
    pub probe: super::probe::FrameFlags,
}

/// The result of one movement frame: one output for each unit, in unit-id order, and one
/// fallback request for each unit that the step gave a pool of one (task A2b1; `fallback.rs`).
/// The progress of such a unit names the pool id 0 until `assign_fallback_pools` gives the ids.
#[derive(Debug, Clone, PartialEq)]
pub struct MovementStep {
    pub outputs: Vec<MovementFrameOutput>,
    pub fallbacks: Vec<FallbackRequest>,
}

/// Deterministic movement kernel shared by the ECS adapter and the tests. It returns the outputs
/// only: a fallback pool of one in them keeps the pool id 0 (see `step_frame_with_fallbacks`).
pub fn step_frame(frame: &MovementFrame, dt: f32) -> Vec<MovementFrameOutput> {
    step_frame_preserving_resting(frame, dt, &std::collections::BTreeSet::new())
}

/// Move a frame while retaining resting slots for orders that still need their
/// assigned destination after a temporary combat pause. It returns the outputs only (see
/// `step_frame_with_fallbacks`).
pub fn step_frame_preserving_resting(
    frame: &MovementFrame,
    dt: f32,
    preserve_resting_units: &std::collections::BTreeSet<u32>,
) -> Vec<MovementFrameOutput> {
    step_frame_with_fallbacks(frame, dt, preserve_resting_units).outputs
}

/// Move a frame, and return the fallback pools of one that the step made (task A2b1). The
/// movement system gives them their ids (`assign_fallback_pools`).
pub fn step_frame_with_fallbacks(
    frame: &MovementFrame,
    dt: f32,
    preserve_resting_units: &std::collections::BTreeSet<u32>,
) -> MovementStep {
    let mut units = frame.units.clone();
    units.sort_unstable_by_key(|unit| unit.unit_id);
    let mut points: Vec<_> = units
        .iter()
        .map(|unit| crate::grid::GridPoint {
            id: unit.unit_id,
            position: unit.position,
        })
        .collect();
    let max_radius = units.iter().map(|unit| unit.radius).fold(0.0_f32, f32::max);
    let mut grid = crate::grid::UniformGrid::new(&points, 2.0 * max_radius);
    // A pool of one (one primary slot or none: the fallback pools, a one-unit envelope) has no
    // index for the frame. Its index is built only when its member looks for a free slot
    // (`lazy_pool_index`), from the state of that moment. The clearance of a slot depends only on
    // the units near it at that moment, and the frame refreshes a slot each time a unit moves
    // near it, so the index built late equals the index that the frame would have kept (the A2b1
    // review, finding H1: an index for each pool of one in each tick was the cost of the step).
    let mut pool_indexes: BTreeMap<_, _> = frame
        .pools
        .iter()
        .filter(|(_, pool)| !is_pool_of_one(pool))
        .map(|(&pool_id, pool)| {
            #[cfg(test)]
            super::count_pool_index_build(pool.primary_count);
            (pool_id, MovementPoolIndex::new(pool, frame.bounds))
        })
        .collect();
    let mut plain_reserved = Vec::new();
    let mut plain_static = Vec::new();
    let mut plain_max_radius = 0.0_f32;
    for unit in &units {
        plain_max_radius = plain_max_radius.max(unit.radius);
        if unit.speed.is_none() {
            plain_static.push(StaticFootprint {
                position: unit.position,
                radius: unit.radius,
            });
            continue;
        }
        plain_reserved.push((
            Some(unit.unit_id),
            ReservedFootprint {
                position: unit.target.unwrap_or(unit.position),
                radius: unit.radius,
            },
        ));
    }
    let mut plain_spread_context =
        SpreadContext::new(plain_reserved, plain_static, frame.bounds, plain_max_radius);
    let max_step = units
        .iter()
        .filter_map(|unit| unit.speed)
        .fold(0.0_f32, f32::max)
        * dt
        * 1.2;
    let mut outputs: Vec<_> = units
        .iter()
        .map(|unit| MovementFrameOutput {
            unit_id: unit.unit_id,
            position: unit.position,
            target: unit.target,
            progress: unit.progress.clone(),
            plain_progress: unit.plain_progress.clone(),
            physical_escape: unit.physical_escape.clone(),
            failed_blocked: false,
            #[cfg(feature = "test-probes")]
            probe: super::probe::FrameFlags::default(),
        })
        .collect();
    let mut slot_claim_counts: std::collections::HashMap<(u64, u32, u32), usize> = units
        .iter()
        .filter_map(|unit| {
            unit.progress.as_ref().map(|progress| {
                (
                    (
                        progress.pool_id,
                        progress.slot.x.to_bits(),
                        progress.slot.y.to_bits(),
                    ),
                    1usize,
                )
            })
        })
        .fold(
            std::collections::HashMap::new(),
            |mut counts, (key, count)| {
                *counts.entry(key).or_default() += count;
                counts
            },
        );
    let mut assigned_slots: std::collections::HashSet<(u64, u32, u32)> =
        slot_claim_counts.keys().copied().collect();
    let mut slot_refs = Vec::new();
    let mut slot_points = Vec::new();
    for (&pool_id, index) in &pool_indexes {
        for (slot_index, point) in index.points.iter().enumerate() {
            slot_refs.push((pool_id, slot_index));
            slot_points.push(crate::grid::GridPoint {
                id: slot_points.len() as u32,
                position: point.position,
            });
        }
    }
    let slot_grid = crate::grid::UniformGrid::new(&slot_points, 2.0 * max_radius);
    for (&pool_id, index) in &mut pool_indexes {
        for slot_index in 0..index.points.len() {
            let point = index.points[slot_index].position;
            let assigned =
                assigned_slots.contains(&(pool_id, point.x.to_bits(), point.y.to_bits()));
            index.refresh_slot_clearance(slot_index, &units, &points, &grid, max_radius, assigned);
        }
    }
    let mut target_claims: std::collections::HashMap<(u32, u32), usize> = units
        .iter()
        .filter_map(|unit| unit.target.map(target_key))
        .fold(std::collections::HashMap::new(), |mut counts, key| {
            *counts.entry(key).or_default() += 1;
            counts
        });
    let mut fallbacks = Vec::new();

    for index in 0..units.len() {
        if index > 0 {
            let previous = &units[index - 1];
            if previous.speed.is_some() {
                plain_spread_context.reserve_unit(
                    previous.unit_id,
                    ReservedFootprint {
                        position: previous.target.unwrap_or(previous.position),
                        radius: previous.radius,
                    },
                );
            }
        }
        let unit = units[index].clone();
        let Some(speed) = unit.speed else { continue };
        if unit.physical_escape.is_some() {
            let overlapping = body_overlaps_position(
                unit.position,
                index,
                unit.radius,
                &units,
                &points,
                &grid,
                max_radius,
            );
            if !overlapping {
                outputs[index].physical_escape = None;
                // A `Blocked` fallback pool keeps `Blocked` until a new order or a new trip
                // (design 9.17), as the plain progress did.
                if outputs[index].progress.as_ref().is_some_and(|progress| {
                    progress.activity == MovementActivity::Blocked && !progress.is_fallback()
                }) {
                    let progress = outputs[index].progress.take().unwrap();
                    let key = (
                        progress.pool_id,
                        progress.slot.x.to_bits(),
                        progress.slot.y.to_bits(),
                    );
                    if remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        progress.slot,
                    ) {
                        assigned_slots.remove(&key);
                    }
                }
                units[index].physical_escape = None;
                units[index].progress = outputs[index].progress.clone();
                units[index].plain_progress = outputs[index].plain_progress.clone();
                // Clearance ends the obligation. A newly accepted target can
                // resume through the ordinary movement path on this same tick.
            } else {
                let escape = unit
                    .physical_escape
                    .as_ref()
                    .and_then(|state| state.target)
                    .filter(|target| {
                        !body_overlaps_position(
                            *target,
                            index,
                            unit.radius,
                            &units,
                            &points,
                            &grid,
                            max_radius,
                        )
                    })
                    .or_else(|| {
                        MovementSearch {
                            bounds: frame.bounds,
                            units: &units,
                            points: &points,
                            grid: &grid,
                            max_radius,
                        }
                        .find_free_singleton_destination(
                            unit.position,
                            unit.radius,
                            index,
                        )
                    });
                let Some(escape) = escape else {
                    outputs[index].physical_escape = Some(PhysicalEscape { target: None });
                    units[index].physical_escape = outputs[index].physical_escape.clone();
                    continue;
                };
                let offset = escape - unit.position;
                let distance = offset.length();
                let mut position = if distance <= speed * dt {
                    escape
                } else if distance > 1.0e-8 {
                    unit.position + offset * ((speed * dt) / distance)
                } else {
                    unit.position
                };
                position.x = position
                    .x
                    .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
                position.y = position
                    .y
                    .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));
                let still_overlapping = body_overlaps_position(
                    position,
                    index,
                    unit.radius,
                    &units,
                    &points,
                    &grid,
                    max_radius,
                );
                outputs[index].position = position;
                outputs[index].physical_escape = still_overlapping.then_some(PhysicalEscape {
                    target: Some(escape),
                });
                if !still_overlapping
                    && outputs[index].progress.as_ref().is_some_and(|progress| {
                        progress.activity == MovementActivity::Blocked && !progress.is_fallback()
                    })
                {
                    let progress = outputs[index].progress.take().unwrap();
                    let key = (
                        progress.pool_id,
                        progress.slot.x.to_bits(),
                        progress.slot.y.to_bits(),
                    );
                    if remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        progress.slot,
                    ) {
                        assigned_slots.remove(&key);
                    }
                }
                units[index].position = position;
                units[index].physical_escape = outputs[index].physical_escape.clone();
                units[index].progress = outputs[index].progress.clone();
                units[index].plain_progress = outputs[index].plain_progress.clone();
                points[index].position = position;
                grid.move_point(index, unit.position, position);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    position,
                    unit.radius,
                    &slot_points,
                    &slot_refs,
                    &slot_grid,
                    &mut pool_indexes,
                    &units,
                    &points,
                    &grid,
                    &assigned_slots,
                    max_radius,
                );
                continue;
            }
        }
        if unit
            .progress
            .as_ref()
            .is_some_and(|progress| progress.activity == MovementActivity::Blocked)
        {
            let Some(mut progress) = unit.progress.clone() else {
                continue;
            };
            let Some(_pool) = frame.pools.get(&progress.pool_id) else {
                continue;
            };
            let occupied = grid
                .within(&points, unit.position, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    other_index != index
                        && unit.position.distance(units[other_index].position)
                            < unit.radius + units[other_index].radius - 1.0e-5
                });
            if !occupied {
                update_target_claims(&mut target_claims, unit.target, None);
                if progress.is_fallback() {
                    // A `Blocked` fallback pool keeps `Blocked` and its slot until a new order or
                    // a new trip (design 9.17): it stops, as a `Blocked` plain progress with no
                    // target did. The pool of an order clears its progress here.
                    outputs[index].target = None;
                    units[index].target = None;
                    continue;
                }
                let old_key = (
                    progress.pool_id,
                    progress.slot.x.to_bits(),
                    progress.slot.y.to_bits(),
                );
                if remove_pool_slot_claim(
                    &mut slot_claim_counts,
                    &mut pool_indexes,
                    progress.pool_id,
                    progress.slot,
                ) {
                    assigned_slots.remove(&old_key);
                }
                outputs[index].target = None;
                outputs[index].progress = None;
                units[index].target = None;
                units[index].progress = None;
                continue;
            }
            let is_free = |slot: Vec2| {
                grid.within(&points, slot, unit.radius + max_radius)
                    .into_iter()
                    .all(|other_index| {
                        other_index == index
                            || slot.distance(units[other_index].position)
                                >= unit.radius + units[other_index].radius
                    })
            };
            let old_slot_key = (
                progress.pool_id,
                progress.slot.x.to_bits(),
                progress.slot.y.to_bits(),
            );
            let target_is_claimed_by_other = unit.target.is_some_and(|target| {
                target_claims.get(&target_key(target)).copied().unwrap_or(0) > 1
            });
            let target_is_blocked_by_stationary = unit.target.is_some_and(|target| {
                grid.within(&points, target, unit.radius + max_radius)
                    .into_iter()
                    .any(|other_index| {
                        if other_index == index {
                            return false;
                        }
                        let other = &units[other_index];
                        let stationary = other.target.is_none()
                            || other
                                .progress
                                .as_ref()
                                .is_some_and(|state| state.activity == MovementActivity::Resting);
                        stationary
                            && target.distance(other.position) < unit.radius + other.radius - 1.0e-5
                    })
            });
            let retained_escape = unit.target.filter(|slot| {
                let key = (progress.pool_id, slot.x.to_bits(), slot.y.to_bits());
                !target_is_claimed_by_other
                    && !target_is_blocked_by_stationary
                    && is_free(*slot)
                    && (!assigned_slots.contains(&key) || key == old_slot_key)
            });
            // A new escape is looked for only when the target cannot be kept, so the late index of a
            // pool of one is built only then (the A2b1 re-check). The search is pure: the result is
            // the same as when both candidates were always computed.
            let escape = retained_escape.or_else(|| {
                let lazy = lazy_pool_index(
                    frame,
                    &pool_indexes,
                    progress.pool_id,
                    &assigned_slots,
                    (&units, &points, &grid, max_radius),
                );
                let pool_escape = pool_indexes
                    .get(&progress.pool_id)
                    .or(lazy.as_ref())
                    .and_then(|index| {
                        index.nearest_free(
                            unit.unit_id,
                            unit.position,
                            progress.pool_id,
                            Some(old_slot_key),
                            &assigned_slots,
                            unit.radius,
                            is_free,
                        )
                    });
                let outside_escape = MovementSearch {
                    bounds: frame.bounds,
                    units: &units,
                    points: &points,
                    grid: &grid,
                    max_radius,
                }
                .find_free_escape_destination(
                    unit.position,
                    unit.radius,
                    index,
                    progress.pool_id,
                    &assigned_slots,
                );
                match (pool_escape, outside_escape) {
                    (Some(pool_point), Some(outside_point)) => Some(
                        if nearest_angle_order(pool_point, outside_point, unit.position).is_le() {
                            pool_point
                        } else {
                            outside_point
                        },
                    ),
                    (Some(point), None) | (None, Some(point)) => Some(point),
                    (None, None) => None,
                }
            });
            let Some(escape) = escape else { continue };
            let escape_key = (progress.pool_id, escape.x.to_bits(), escape.y.to_bits());
            if escape_key != old_slot_key {
                if remove_pool_slot_claim(
                    &mut slot_claim_counts,
                    &mut pool_indexes,
                    progress.pool_id,
                    progress.slot,
                ) {
                    assigned_slots.remove(&old_slot_key);
                }
                add_pool_slot_claim(
                    &mut slot_claim_counts,
                    &mut pool_indexes,
                    progress.pool_id,
                    escape,
                );
                assigned_slots.insert(escape_key);
                progress.slot = escape;
            }
            let step_limit = speed * dt;
            let delta = escape - unit.position;
            let distance = delta.length();
            let mut position = if distance <= step_limit {
                escape
            } else if distance > 1.0e-8 {
                unit.position + delta / distance * step_limit
            } else {
                unit.position
            };
            position.x = position
                .x
                .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
            position.y = position
                .y
                .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));
            let reached = position == escape && is_free(escape);
            update_target_claims(
                &mut target_claims,
                unit.target,
                (!reached).then_some(escape),
            );
            outputs[index].position = position;
            outputs[index].target = (!reached).then_some(escape);
            outputs[index].progress = Some(progress.clone());
            units[index].position = position;
            points[index].position = position;
            grid.move_point(index, unit.position, position);
            refresh_nearby_pool_slot_clearances(
                unit.position,
                position,
                unit.radius,
                &slot_points,
                &slot_refs,
                &slot_grid,
                &mut pool_indexes,
                &units,
                &points,
                &grid,
                &assigned_slots,
                max_radius,
            );
            units[index].target = outputs[index].target;
            units[index].progress = outputs[index].progress.clone();
            continue;
        }
        if let Some(progress) = unit.plain_progress.clone() {
            let output = step_plain_progress(
                frame.bounds,
                frame.tick_hz,
                max_radius,
                dt,
                index,
                &unit,
                progress,
                &units,
                &points,
                &grid,
                &plain_spread_context,
            );
            outputs[index] = output.clone();
            update_target_claims(&mut target_claims, unit.target, output.target);
            units[index].position = output.position;
            units[index].target = output.target;
            units[index].plain_progress = output.plain_progress.clone();
            points[index].position = output.position;
            grid.move_point(index, unit.position, output.position);
            refresh_nearby_pool_slot_clearances(
                unit.position,
                output.position,
                unit.radius,
                &slot_points,
                &slot_refs,
                &slot_grid,
                &mut pool_indexes,
                &units,
                &points,
                &grid,
                &assigned_slots,
                max_radius,
            );
            continue;
        }
        let Some(mut target) = unit.target else {
            continue;
        };
        // One step for a unit that moves to its target. A unit with a pool and a unit with no
        // pool pass the same phases in the same order: arrival, steering, projection, then the
        // end of the step. Three values have one float expression for a unit with a pool and
        // another for a unit with no pool; the presence of the pool selects the expression, so
        // that the output is bit-equal to the two branches that this step replaces (design 9.8).
        let mut pooled = unit.progress.is_some();
        let mut progress = unit.progress.clone();
        #[cfg(feature = "test-probes")]
        {
            outputs[index].probe.phases.arrival += 1;
        }
        if !pooled {
            let target_occupied =
                plain_spread_context.conflicts_with_reserved(target, unit.radius, unit.unit_id)
                    || grid
                        .within(&points, target, unit.radius + max_radius)
                        .into_iter()
                        .any(|other_index| {
                            other_index != index
                                && target.distance(units[other_index].position)
                                    < unit.radius + units[other_index].radius - 1.0e-5
                        });
            if target_occupied {
                // Fallback: the goal is occupied, so the trip takes a pool of one with the spread
                // code (design 9.11). A pool with no free slot is `Blocked`, and the unit stops.
                // Else the unit steps to its slot in this frame as a unit with a pool, as the plain
                // progress stepped in its first frame. Its pool has the id 0 until the movement
                // system gives the ids (`fallback.rs`), so it is in no group in this frame.
                let (pool, fallback) = fallback_pool(&unit, target, &plain_spread_context);
                #[cfg(feature = "test-probes")]
                {
                    outputs[index].probe.phases.fallback += 1;
                }
                fallbacks.push(FallbackRequest {
                    unit_id: unit.unit_id,
                    pool,
                });
                if fallback.activity == MovementActivity::Blocked {
                    update_target_claims(&mut target_claims, unit.target, None);
                    outputs[index].target = None;
                    outputs[index].progress = Some(fallback);
                    units[index].target = None;
                    units[index].progress = outputs[index].progress.clone();
                    continue;
                }
                target = fallback.slot;
                progress = Some(fallback);
                pooled = true;
            }
        }
        // A pool id of 0 is a fallback pool of this frame with no id yet: it has no group.
        let pool_id = progress
            .as_ref()
            .map(|state| state.pool_id)
            .filter(|pool_id| *pool_id != 0);
        let pass_mode = progress.as_ref().is_some_and(|state| state.pass_mode);
        let mut movement_target = target;
        let mut failed_blocked = false;
        let mut position = unit.position;
        let distance = position.distance(movement_target);
        let step_limit = speed * dt;
        // Another unit stands on `point`: a footprint overlap of more than 1.0e-5.
        let occupied_at = |point: Vec2| {
            grid.within(&points, point, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    let other = &units[other_index];
                    other_index != index
                        && point.distance(other.position) < unit.radius + other.radius - 1.0e-5
                })
        };
        // `new_target` is the target of the unit after the step; `Some` until the unit arrives
        // or fails. The labelled block ends at the arrival, early or late, or at the end of the
        // step.
        let mut new_target = None;
        'step: {
            // Arrival: the target is within one step, or within 0.025, and free.
            if (distance <= step_limit || distance <= 0.025) && !occupied_at(movement_target) {
                position = movement_target;
                if let Some(state) = progress.as_mut() {
                    state.activity = MovementActivity::Resting;
                }
                break 'step;
            }

            // Steering: the velocity, and the separation from the neighbours.
            #[cfg(feature = "test-probes")]
            {
                outputs[index].probe.phases.steering += 1;
            }
            let desired = movement_target - position;
            let mut velocity = if pooled {
                desired.try_normalize().unwrap_or(Vec2::ZERO) * speed * dt
            } else if distance > 1.0e-8 {
                desired * (step_limit / distance)
            } else {
                Vec2::ZERO
            };
            let reach = unit.radius + max_radius + 0.05 + max_step;
            let neighbours = grid.within(&points, position, reach);
            let mut near_same_group = false;
            let mut separated = false;
            for other_index in neighbours {
                if other_index == index {
                    continue;
                }
                let other = &units[other_index];
                let same_group = pool_id.is_some_and(|pool_id| {
                    unit.owner_slot == other.owner_slot
                        && other
                            .progress
                            .as_ref()
                            .is_some_and(|state| state.pool_id == pool_id)
                });
                let other_moving_in_group = same_group && other.target.is_some();
                if other_moving_in_group
                    && position.distance(other.position) < unit.radius + other.radius + 0.05
                {
                    near_same_group = true;
                }
                if same_group && (!other_moving_in_group || pass_mode) {
                    continue;
                }
                let offset = position - other.position;
                let separation = offset.length();
                let limit = unit.radius + other.radius + 0.05;
                if separation > 1.0e-8 && separation < limit {
                    separated = true;
                    let push = if pooled {
                        (limit - separation) * offset / separation
                    } else {
                        offset / separation * (limit - separation)
                    };
                    velocity += push;
                    if !same_group && offset.dot(desired) < 0.0 {
                        let along = desired.try_normalize().unwrap_or(Vec2::X);
                        velocity += Vec2::new(along.y, -along.x) * (limit - separation);
                    }
                }
            }
            if !separated && distance > step_limit {
                position = step(
                    &MovementInput {
                        position: unit.position,
                        target: movement_target,
                        speed,
                    },
                    dt,
                )
                .position;
            } else {
                if pooled {
                    let velocity_len = velocity.length();
                    if velocity_len > step_limit && velocity_len > 0.0 {
                        velocity *= step_limit / velocity_len;
                    }
                } else if velocity.length() > step_limit {
                    velocity = velocity.normalize() * step_limit;
                }
                position += velocity;
            }

            // Projection: the hard projection prevents footprint overlap. Units resolve in id
            // order. A unit does not push a unit of its own group.
            #[cfg(feature = "test-probes")]
            {
                outputs[index].probe.phases.projection += 1;
            }
            let broad_reach = unit.radius + max_radius + max_step;
            let mut was_projected = false;
            for _ in 0..8 {
                let mut projected = false;
                for other_index in grid.within(&points, position, broad_reach) {
                    if other_index == index {
                        continue;
                    }
                    let other = &units[other_index];
                    let same_group = pool_id.is_some_and(|pool_id| {
                        unit.owner_slot == other.owner_slot
                            && other
                                .progress
                                .as_ref()
                                .is_some_and(|state| state.pool_id == pool_id)
                    });
                    if same_group {
                        continue;
                    }
                    let offset = position - other.position;
                    let separation = offset.length();
                    let limit = unit.radius + other.radius + 1.0e-4;
                    if separation < limit {
                        let direction = if separation > 1.0e-8 {
                            offset / separation
                        } else if unit.unit_id < other.unit_id {
                            Vec2::X
                        } else {
                            -Vec2::X
                        };
                        position += direction * (limit - separation);
                        projected = true;
                    }
                }
                if !projected {
                    break;
                }
                was_projected = true;
            }
            position.x = position
                .x
                .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
            position.y = position
                .y
                .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));
            // A unit with a pool stores `separated` at the end of the step, after the arrival
            // check below; a unit with no pool stores it here. This keeps the flags of the
            // two branches that this step replaces.
            #[cfg(feature = "test-probes")]
            {
                outputs[index].probe.was_projected = Some(was_projected);
                outputs[index].probe.separated = separated && !pooled;
            }

            // Late arrival: the step ended on the target, and the target is free. Arrival
            // eligibility was checked before movement, so this step may end early only if it
            // reaches the destination exactly.
            if position == movement_target && !occupied_at(movement_target) {
                position = movement_target;
                if let Some(state) = progress.as_mut() {
                    state.activity = MovementActivity::Resting;
                }
                break 'step;
            }
            #[cfg(feature = "test-probes")]
            {
                outputs[index].probe.separated = separated;
            }
            new_target = Some(movement_target);

            if let Some(progress) = progress.as_mut() {
                // Stall: the progress bookkeeping of a unit with a pool.
                #[cfg(feature = "test-probes")]
                {
                    outputs[index].probe.phases.stall += 1;
                }
                let next_distance = position.distance(movement_target);
                let made_progress = next_distance < progress.best_distance - 0.01 * step_limit;
                progress.stalling_on_group = near_same_group;
                if made_progress {
                    progress.best_distance = next_distance;
                    progress.stalled_ticks = 0;
                } else {
                    progress.stalled_ticks = progress.stalled_ticks.saturating_add(1);
                }
                let hz = frame.tick_hz.max(1);
                if progress.stalled_ticks >= hz {
                    // Fallback of a unit with a pool: the pass mode, a spare slot, or `Blocked`.
                    #[cfg(feature = "test-probes")]
                    {
                        outputs[index].probe.phases.fallback += 1;
                    }
                    if progress.stalling_on_group && !progress.pass_mode {
                        progress.pass_mode = true;
                        progress.stalled_ticks = 0;
                        progress.best_distance = next_distance;
                    } else if progress.claim_count < 3 {
                        let old_slot_key = (
                            progress.pool_id,
                            progress.slot.x.to_bits(),
                            progress.slot.y.to_bits(),
                        );
                        let lazy = lazy_pool_index(
                            frame,
                            &pool_indexes,
                            progress.pool_id,
                            &assigned_slots,
                            (&units, &points, &grid, max_radius),
                        );
                        let spare = pool_indexes
                            .get(&progress.pool_id)
                            .or(lazy.as_ref())
                            .and_then(|pool_index| {
                                pool_index.nearest_free(
                                    unit.unit_id,
                                    position,
                                    progress.pool_id,
                                    None,
                                    &assigned_slots,
                                    unit.radius,
                                    |slot| {
                                        let key =
                                            (progress.pool_id, slot.x.to_bits(), slot.y.to_bits());
                                        key != old_slot_key
                                            && grid
                                                .within(&points, slot, unit.radius + max_radius)
                                                .into_iter()
                                                .all(|other_index| {
                                                    other_index == index
                                                        || slot
                                                            .distance(units[other_index].position)
                                                            >= unit.radius
                                                                + units[other_index].radius
                                                })
                                    },
                                )
                            });
                        if let Some(spare) = spare {
                            if remove_pool_slot_claim(
                                &mut slot_claim_counts,
                                &mut pool_indexes,
                                progress.pool_id,
                                progress.slot,
                            ) {
                                assigned_slots.remove(&old_slot_key);
                            }
                            add_pool_slot_claim(
                                &mut slot_claim_counts,
                                &mut pool_indexes,
                                progress.pool_id,
                                spare,
                            );
                            assigned_slots.insert((
                                progress.pool_id,
                                spare.x.to_bits(),
                                spare.y.to_bits(),
                            ));
                            movement_target = spare;
                            progress.slot = spare;
                            progress.claim_count += 1;
                            progress.pass_mode = false;
                            progress.stalled_ticks = 0;
                            progress.best_distance = position.distance(movement_target);
                        } else {
                            progress.activity = MovementActivity::Blocked;
                            failed_blocked = true;
                        }
                    } else {
                        progress.activity = MovementActivity::Blocked;
                        failed_blocked = true;
                    }
                }
                new_target = (!failed_blocked).then_some(movement_target);
            } else if separated || was_projected {
                // Fallback of a unit with no pool: it was pushed or projected and did not arrive,
                // so the trip takes a pool of one with the spread code (design 9.11). The unit
                // has moved in this frame; it moves to its slot from the next frame.
                #[cfg(feature = "test-probes")]
                {
                    outputs[index].probe.phases.fallback += 1;
                }
                let (pool, mut fallback) = fallback_pool(&unit, target, &plain_spread_context);
                fallback.best_distance = position.distance(fallback.slot);
                new_target =
                    (fallback.activity != MovementActivity::Blocked).then_some(fallback.slot);
                progress = Some(fallback);
                fallbacks.push(FallbackRequest {
                    unit_id: unit.unit_id,
                    pool,
                });
            }
        }
        // An early arrival of a unit with no pool now updates `target_claims`; the old branch did
        // not. The map is read only in the `Blocked` branch (`target_is_claimed_by_other`), and
        // only together with `target_is_blocked_by_stationary` and `is_free(target)`. The arrived
        // unit stands on its target with a radius above 0, so both of those are decided by it
        // and the output of a later unit does not change. A pool unit already did this call.
        update_target_claims(&mut target_claims, unit.target, new_target);
        outputs[index].position = position;
        outputs[index].target = new_target;
        outputs[index].failed_blocked = failed_blocked;
        if failed_blocked
            && body_overlaps_position(
                position,
                index,
                unit.radius,
                &units,
                &points,
                &grid,
                max_radius,
            )
        {
            outputs[index].physical_escape = Some(PhysicalEscape { target: None });
        }
        outputs[index].progress = progress;
        units[index].position = position;
        units[index].target = new_target;
        units[index].progress = outputs[index].progress.clone();
        points[index].position = position;
        grid.move_point(index, unit.position, position);
        refresh_nearby_pool_slot_clearances(
            unit.position,
            position,
            unit.radius,
            &slot_points,
            &slot_refs,
            &slot_grid,
            &mut pool_indexes,
            &units,
            &points,
            &grid,
            &assigned_slots,
            max_radius,
        );
    }
    let moving_pools: std::collections::BTreeSet<_> = outputs
        .iter()
        .filter_map(|output| {
            (output.target.is_some()
                || output
                    .progress
                    .as_ref()
                    .is_some_and(|state| state.activity == MovementActivity::Blocked))
            .then(|| output.progress.as_ref().map(|progress| progress.pool_id))
            .flatten()
        })
        .collect();
    for output in &mut outputs {
        let collect = output.progress.as_ref().is_some_and(|progress| {
            if progress.is_fallback() {
                // A fallback pool that rests: the step collects it only when it served no gather
                // leg (design 9.19), took no spare and rests on the point of its trip, as the
                // plain progress was dropped at its original point. Else it stays `Resting`, and
                // the order phase ends it (`orders/run.rs`), or the follow order completes on it.
                progress.activity == MovementActivity::Resting
                    && progress.gather_goal.is_none()
                    && progress.claim_count == 0
                    && frame
                        .pools
                        .get(&progress.pool_id)
                        .is_some_and(|pool| pool.point == progress.slot)
            } else {
                progress.activity == MovementActivity::Resting
                    && !moving_pools.contains(&progress.pool_id)
                    && progress.follow_origin.is_none()
                    && !preserve_resting_units.contains(&output.unit_id)
            }
        });
        if collect {
            output.progress = None;
        }
    }
    MovementStep { outputs, fallbacks }
}

/// A pool with one primary slot or none (`MovementPool::primary_count`): it gets no index for the
/// frame (`lazy_pool_index`).
fn is_pool_of_one(pool: &MovementPool) -> bool {
    pool.primary_count <= 1
}

/// The index of a pool of one, built from the state of this moment of the frame: the units, their
/// grid and the assigned slots. `None` for a pool that has a frame index (`pool_indexes`) or that
/// the frame does not hold.
fn lazy_pool_index(
    frame: &MovementFrame,
    pool_indexes: &BTreeMap<u64, MovementPoolIndex>,
    pool_id: u64,
    assigned_slots: &std::collections::HashSet<(u64, u32, u32)>,
    (units, points, grid, max_radius): (
        &[MovementFrameUnit],
        &[crate::grid::GridPoint],
        &crate::grid::UniformGrid,
        f32,
    ),
) -> Option<MovementPoolIndex> {
    if pool_indexes.contains_key(&pool_id) {
        return None;
    }
    let pool = frame
        .pools
        .get(&pool_id)
        .filter(|pool| is_pool_of_one(pool))?;
    #[cfg(test)]
    super::count_pool_index_build(pool.primary_count);
    let mut index = MovementPoolIndex::new(pool, frame.bounds);
    for slot_index in 0..index.points.len() {
        let point = index.points[slot_index].position;
        let assigned = assigned_slots.contains(&(pool_id, point.x.to_bits(), point.y.to_bits()));
        index.refresh_slot_clearance(slot_index, units, points, grid, max_radius, assigned);
    }
    Some(index)
}
