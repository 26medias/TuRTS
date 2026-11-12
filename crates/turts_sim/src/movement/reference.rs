//! The reference copy of the movement kernel (cargo feature `test-probes`).
//!
//! Pre-MP1 slice, task A2a (design section 2 row A2a, section 9 items 8 and 9; plan
//! refinements R9 and R11). This is `step_frame_preserving_resting` as it stood at the base
//! of A2a (commit 8722f8e, `movement/frame.rs`), with two separate branches for a unit that
//! has no pool and a unit that has a pool. The test `tests/kernel_reference.rs` runs this
//! copy and the live kernel on the same seeded frames and compares every f32 with `to_bits`.
//! No suite test pins the bits of a pooled unit, so this copy is the proof of equality
//! (design 9.9).
//!
//! The edits against the original text are the name of the function, the
//! `..Default::default()` of the `FrameFlags` literal (the probe gained the field `phases`), and
//! the local copy of `step` below.
//!
//! The copy still calls the live helper files `plain.rs`, `pool_index.rs`, `search.rs`,
//! `spread.rs` and `grid.rs`. The test `reference_helpers_are_unchanged` in
//! `tests/kernel_reference.rs` pins the sha256 of each. A task that must edit one of them first
//! puts a copy of the old text into the reference (for example `reference_plain.rs`), then
//! updates the pin. The helpers `angle` and the types `MovementPool` and `MoveProgress` in
//! `movement/mod.rs` are not pinned (NOT covered).
//! Task A2b2 deletes the plain-progress model that this copy calls; the later tasks of the
//! slice take a new reference copy from their own base (R9).
//!
//! This module has no `cfg(not(...))` and nothing outside the feature calls it.

use bevy_math::Vec2;
use std::collections::BTreeMap;

use super::frame::{MovementFrame, MovementFrameOutput};
use super::plain::{make_plain_progress, step_plain_progress};
use super::pool_index::{
    MovementPoolIndex, add_pool_slot_claim, refresh_nearby_pool_slot_clearances,
    remove_pool_slot_claim,
};
use super::search::{MovementSearch, body_overlaps_position};
use super::spread::{SpreadContext, nearest_angle_order, target_key, update_target_claims};
use super::{
    MovementActivity, MovementInput, MovementOutput, PhysicalEscape, ReservedFootprint,
    StaticFootprint,
};

/// A copy of `movement::step` at the base of A2a (`movement/mod.rs`), so that an edit of the live
/// `step` cannot change the reference.
fn step(input: &MovementInput, dt: f32) -> MovementOutput {
    let step = input.speed * dt;
    let offset = input.target - input.position;
    let distance = offset.length();
    if distance <= step {
        MovementOutput {
            position: input.target,
            reached: true,
        }
    } else {
        MovementOutput {
            position: input.position + offset * (step / distance),
            reached: false,
        }
    }
}

/// The kernel of the base of A2a: move a frame while retaining resting slots for orders that
/// still need their assigned destination after a temporary combat pause.
pub fn reference_step_frame_preserving_resting(
    frame: &MovementFrame,
    dt: f32,
    preserve_resting_units: &std::collections::BTreeSet<u32>,
) -> Vec<MovementFrameOutput> {
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
    let mut pool_indexes: BTreeMap<_, _> = frame
        .pools
        .iter()
        .map(|(&pool_id, pool)| (pool_id, MovementPoolIndex::new(pool, frame.bounds)))
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
                if outputs[index]
                    .progress
                    .as_ref()
                    .is_some_and(|progress| progress.activity == MovementActivity::Blocked)
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
                    && outputs[index]
                        .progress
                        .as_ref()
                        .is_some_and(|progress| progress.activity == MovementActivity::Blocked)
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
            let pool_escape = pool_indexes.get(&progress.pool_id).and_then(|index| {
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
            let nearest_new = match (pool_escape, outside_escape) {
                (Some(pool_point), Some(outside_point)) => Some(
                    if nearest_angle_order(pool_point, outside_point, unit.position).is_le() {
                        pool_point
                    } else {
                        outside_point
                    },
                ),
                (Some(point), None) | (None, Some(point)) => Some(point),
                (None, None) => None,
            };
            let escape = retained_escape.or(nearest_new);
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
        let Some(target) = unit.target else { continue };
        let Some(mut progress) = unit.progress.clone() else {
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
                let progress = make_plain_progress(&unit, target, &plain_spread_context);
                let mut plain_unit = unit.clone();
                plain_unit.target = Some(progress.slot);
                let output = step_plain_progress(
                    frame.bounds,
                    frame.tick_hz,
                    max_radius,
                    dt,
                    index,
                    &plain_unit,
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
            let destination = target;
            let desired = destination - unit.position;
            let step_limit = speed * dt;
            let destination_free = grid
                .within(&points, destination, unit.radius + max_radius)
                .into_iter()
                .all(|other_index| {
                    other_index == index
                        || destination.distance(units[other_index].position)
                            >= unit.radius + units[other_index].radius - 1.0e-5
                });
            if (desired.length() <= step_limit || desired.length() <= 0.025) && destination_free {
                outputs[index].position = destination;
                outputs[index].target = None;
                units[index].position = destination;
                units[index].target = None;
                units[index].progress = outputs[index].progress.clone();
                points[index].position = destination;
                grid.move_point(index, unit.position, destination);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    destination,
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
            let desired_distance = desired.length();
            let mut velocity = if desired_distance > 1.0e-8 {
                desired * (step_limit / desired_distance)
            } else {
                Vec2::ZERO
            };
            let mut separated = false;
            let reach = unit.radius + max_radius + 0.05 + max_step;
            for other_index in grid.within(&points, unit.position, reach) {
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
            let mut position = if !separated && desired_distance > step_limit {
                step(
                    &MovementInput {
                        position: unit.position,
                        target: destination,
                        speed,
                    },
                    dt,
                )
                .position
            } else {
                if velocity.length() > step_limit {
                    velocity = velocity.normalize() * step_limit;
                }
                unit.position + velocity
            };
            let mut was_projected = false;
            for _ in 0..8 {
                let mut projected = false;
                for other_index in
                    grid.within(&points, position, unit.radius + max_radius + max_step)
                {
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
            #[cfg(feature = "test-probes")]
            {
                outputs[index].probe = super::probe::FrameFlags {
                    separated,
                    was_projected: Some(was_projected),
                    ..Default::default()
                };
            }
            // Arrival eligibility was checked before movement, so this step
            // may end early only if it reaches the destination exactly.
            let reached = position == destination;
            if reached && destination_free {
                position = destination;
            }
            outputs[index].position = position;
            outputs[index].target = (!(reached && destination_free)).then_some(destination);
            if (separated || was_projected) && !(reached && destination_free) {
                let mut plain = make_plain_progress(&unit, target, &plain_spread_context);
                plain.slot = destination;
                plain.best_distance = unit.position.distance(destination);
                let next_distance = position.distance(destination);
                if next_distance < plain.best_distance - 0.01 * step_limit {
                    plain.best_distance = next_distance;
                } else {
                    plain.stalled_ticks = 1;
                }
                outputs[index].plain_progress = Some(plain);
            }
            update_target_claims(&mut target_claims, unit.target, outputs[index].target);
            units[index].position = position;
            units[index].target = outputs[index].target;
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
            continue;
        };
        let mut movement_target = target;
        let mut failed_blocked = false;
        let mut position = unit.position;
        let distance = position.distance(movement_target);
        if distance <= speed * dt || distance <= 0.025 {
            let occupied = grid
                .within(&points, movement_target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    let other = &units[other_index];
                    other_index != index
                        && movement_target.distance(other.position)
                            < unit.radius + other.radius - 1.0e-5
                });
            if !occupied {
                update_target_claims(&mut target_claims, unit.target, None);
                outputs[index].position = movement_target;
                outputs[index].target = None;
                progress.activity = MovementActivity::Resting;
                outputs[index].progress = Some(progress);
                units[index].position = movement_target;
                units[index].target = None;
                units[index].progress = outputs[index].progress.clone();
                points[index].position = movement_target;
                grid.move_point(index, unit.position, movement_target);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    movement_target,
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

        let desired = movement_target - position;
        let mut velocity = desired.try_normalize().unwrap_or(Vec2::ZERO) * speed * dt;
        let reach = unit.radius + max_radius + 0.05 + max_step;
        let neighbours = grid.within(&points, position, reach);
        let mut near_same_group = false;
        let mut separated = false;
        for other_index in neighbours {
            if other_index == index {
                continue;
            }
            let other = &units[other_index];
            let same_group = unit.owner_slot == other.owner_slot
                && unit
                    .progress
                    .as_ref()
                    .is_some_and(|state| state.pool_id == progress.pool_id)
                && other
                    .progress
                    .as_ref()
                    .is_some_and(|state| state.pool_id == progress.pool_id);
            let other_moving_in_group = same_group && other.target.is_some();
            if other_moving_in_group
                && position.distance(other.position) < unit.radius + other.radius + 0.05
            {
                near_same_group = true;
            }
            if same_group && (!other_moving_in_group || progress.pass_mode) {
                continue;
            }
            let offset = position - other.position;
            let separation = offset.length();
            let limit = unit.radius + other.radius + 0.05;
            if separation > 1.0e-8 && separation < limit {
                separated = true;
                let push = (limit - separation) * offset / separation;
                velocity += push;
                if !same_group && offset.dot(desired) < 0.0 {
                    let along = desired.try_normalize().unwrap_or(Vec2::X);
                    velocity += Vec2::new(along.y, -along.x) * (limit - separation);
                }
            }
        }
        let velocity_len = velocity.length();
        let step_limit = speed * dt;
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
            if velocity_len > step_limit && velocity_len > 0.0 {
                velocity *= step_limit / velocity_len;
            }
            position += velocity;
        }

        // Hard projection prevents footprint overlap. Units resolve in id order.
        let broad_reach = unit.radius + max_radius + max_step;
        #[cfg(feature = "test-probes")]
        let mut was_projected = false;
        for _ in 0..8 {
            let mut projected = false;
            for other_index in grid.within(&points, position, broad_reach) {
                if other_index == index {
                    continue;
                }
                let other = &units[other_index];
                let same_group = unit.owner_slot == other.owner_slot
                    && other
                        .progress
                        .as_ref()
                        .is_some_and(|state| state.pool_id == progress.pool_id);
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
            #[cfg(feature = "test-probes")]
            {
                was_projected = true;
            }
        }
        position.x = position
            .x
            .clamp(unit.radius, (frame.bounds.x - unit.radius).max(unit.radius));
        position.y = position
            .y
            .clamp(unit.radius, (frame.bounds.y - unit.radius).max(unit.radius));
        #[cfg(feature = "test-probes")]
        {
            outputs[index].probe.was_projected = Some(was_projected);
        }

        let next_distance = position.distance(movement_target);
        if position == movement_target {
            let occupied = grid
                .within(&points, movement_target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    let other = &units[other_index];
                    other_index != index
                        && movement_target.distance(other.position)
                            < unit.radius + other.radius - 1.0e-5
                });
            if !occupied {
                update_target_claims(&mut target_claims, unit.target, None);
                outputs[index].position = movement_target;
                outputs[index].target = None;
                progress.activity = MovementActivity::Resting;
                outputs[index].progress = Some(progress);
                units[index].position = movement_target;
                units[index].target = None;
                units[index].progress = outputs[index].progress.clone();
                points[index].position = movement_target;
                grid.move_point(index, unit.position, movement_target);
                refresh_nearby_pool_slot_clearances(
                    unit.position,
                    movement_target,
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
                let spare = pool_indexes.get(&progress.pool_id).and_then(|pool_index| {
                    pool_index.nearest_free(
                        unit.unit_id,
                        position,
                        progress.pool_id,
                        None,
                        &assigned_slots,
                        unit.radius,
                        |slot| {
                            let key = (progress.pool_id, slot.x.to_bits(), slot.y.to_bits());
                            key != old_slot_key
                                && grid
                                    .within(&points, slot, unit.radius + max_radius)
                                    .into_iter()
                                    .all(|other_index| {
                                        other_index == index
                                            || slot.distance(units[other_index].position)
                                                >= unit.radius + units[other_index].radius
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
                    assigned_slots.insert((progress.pool_id, spare.x.to_bits(), spare.y.to_bits()));
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
        #[cfg(feature = "test-probes")]
        {
            outputs[index].probe.separated = separated;
        }
        outputs[index].position = position;
        outputs[index].target = (!failed_blocked).then_some(movement_target);
        update_target_claims(&mut target_claims, unit.target, outputs[index].target);
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
        outputs[index].progress = Some(progress);
        units[index].position = position;
        units[index].target = outputs[index].target;
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
        if output.progress.as_ref().is_some_and(|progress| {
            progress.activity == MovementActivity::Resting
                && !moving_pools.contains(&progress.pool_id)
                && progress.follow_origin.is_none()
                && !preserve_resting_units.contains(&output.unit_id)
        }) {
            output.progress = None;
        }
    }
    outputs
}
