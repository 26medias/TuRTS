//! The movement frame step.

use bevy_math::Vec2;
use std::collections::BTreeMap;

use super::arrival::SpatialNeighbors;
use super::plain::{PlainMoveProgress, make_plain_progress, step_plain_progress};
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
    /// Explicit physical tie resolution from this solver step. Not serialized.
    pub(crate) arrival_slot: Option<Vec2>,
    /// The flags of the last step of this unit. Tests read them through `probe`.
    #[cfg(feature = "test-probes")]
    pub probe: super::probe::FrameFlags,
}

fn retain_waypoint_motion(output: &mut MovementFrameOutput, target: Option<Vec2>) {
    output.target = target;
    output.arrival_slot = None;
    if let Some(progress) = output.plain_progress.as_mut()
        && progress.activity == MovementActivity::Resting
    {
        progress.activity = MovementActivity::Moving;
    }
    if let Some(progress) = output.progress.as_mut()
        && progress.activity == MovementActivity::Resting
    {
        progress.activity = MovementActivity::Moving;
    }
}

/// Deterministic movement kernel shared by the ECS adapter and economy-log replay.
pub fn step_frame(frame: &MovementFrame, dt: f32) -> Vec<MovementFrameOutput> {
    step_frame_preserving_resting(frame, dt, &std::collections::BTreeSet::new())
}

/// Move a frame while retaining resting slots for orders that still need their
/// assigned destination after a temporary combat pause.
pub fn step_frame_preserving_resting(
    frame: &MovementFrame,
    dt: f32,
    preserve_resting_units: &std::collections::BTreeSet<u32>,
) -> Vec<MovementFrameOutput> {
    step_frame_with_sweep_budget(
        frame,
        dt,
        preserve_resting_units,
        &mut super::sweep::SweepBudget::new(0),
    )
}
/// Replay the same bounded physical service phase as a captured fixed tick.
/// The tick selects fair query service only. Unexhausted geometry is unchanged.
pub fn step_frame_at_tick(frame: &MovementFrame, dt: f32, tick: u32) -> Vec<MovementFrameOutput> {
    step_frame_preserving_resting_at_tick(frame, dt, &std::collections::BTreeSet::new(), tick)
}
/// Tick-aware variant for attack orders that retain an already settled slot.
pub fn step_frame_preserving_resting_at_tick(
    frame: &MovementFrame,
    dt: f32,
    preserve: &std::collections::BTreeSet<u32>,
    tick: u32,
) -> Vec<MovementFrameOutput> {
    step_frame_with_sweep_budget(
        frame,
        dt,
        preserve,
        &mut super::sweep::SweepBudget::new(tick),
    )
}
/// Replay a movement tick against independently reconstructed visibility facts.
pub fn step_frame_preserving_resting_at_tick_with_visibility(
    frame: &MovementFrame,
    dt: f32,
    preserve: &std::collections::BTreeSet<u32>,
    tick: u32,
    visibility: &dyn super::MovementVisibility,
) -> Vec<MovementFrameOutput> {
    step_frame_with_motion_guide_and_visibility(
        frame,
        dt,
        preserve,
        &mut super::sweep::SweepBudget::new(tick),
        &mut super::motion::NoMotionGuide,
        Some(visibility),
    )
}
pub(crate) fn step_frame_with_sweep_budget(
    frame: &MovementFrame,
    dt: f32,
    preserve_resting_units: &std::collections::BTreeSet<u32>,
    budget: &mut super::sweep::SweepBudget,
) -> Vec<MovementFrameOutput> {
    step_frame_with_motion_guide(
        frame,
        dt,
        preserve_resting_units,
        budget,
        &mut super::motion::NoMotionGuide,
    )
}

pub(crate) fn step_frame_with_motion_guide(
    frame: &MovementFrame,
    dt: f32,
    preserve_resting_units: &std::collections::BTreeSet<u32>,
    budget: &mut super::sweep::SweepBudget,
    guide: &mut dyn super::motion::MotionGuide,
) -> Vec<MovementFrameOutput> {
    step_frame_with_motion_guide_and_visibility(
        frame,
        dt,
        preserve_resting_units,
        budget,
        guide,
        None,
    )
}

pub(crate) fn step_frame_with_motion_guide_and_visibility(
    frame: &MovementFrame,
    dt: f32,
    preserve_resting_units: &std::collections::BTreeSet<u32>,
    budget: &mut super::sweep::SweepBudget,
    guide: &mut dyn super::motion::MotionGuide,
    visibility: Option<&dyn super::MovementVisibility>,
) -> Vec<MovementFrameOutput> {
    // Live frames are already sorted. For an unsorted public replay frame,
    // retain one row index per unit so each identity lookup remains logarithmic.
    let unsorted_unit_rows = if visibility.is_some()
        && !frame
            .units
            .windows(2)
            .all(|rows| rows[0].unit_id <= rows[1].unit_id)
    {
        let mut rows = (0..frame.units.len()).collect::<Vec<_>>();
        rows.sort_unstable_by_key(|&row| frame.units[row].unit_id);
        Some(rows)
    } else {
        None
    };
    let mut units = frame.units.clone();
    let mut resolved_ties = super::arrival::prepare_with_guide(
        &mut units,
        &frame.units,
        dt,
        frame.bounds,
        guide,
        visibility,
    );
    resolved_ties.sort_unstable_by_key(|(unit, _)| *unit);
    let origins = units.iter().map(|unit| unit.position).collect::<Vec<_>>();
    let units_at_start: Vec<(u8, f32)> = units
        .iter()
        .map(|unit| (unit.owner_slot, unit.radius))
        .collect();
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
    let exchanges_claims = guide.exchanges_claims();
    if exchanges_claims {
        for index in pool_indexes.values_mut() {
            index.restrict_clearance_to_pool_owner();
        }
        for unit in &units {
            if let Some(pool) = unit
                .progress
                .as_ref()
                .and_then(|progress| pool_indexes.get_mut(&progress.pool_id))
            {
                pool.note_owner(unit.owner_slot);
            }
        }
    }
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
        if unit
            .plain_progress
            .as_ref()
            .is_some_and(|progress| progress.arrival_yield)
        {
            continue;
        }
        plain_reserved.push((
            Some(unit.unit_id),
            ReservedFootprint {
                position: unit.position,
                radius: unit.radius,
            },
        ));
    }
    let mut plain_spread_context =
        SpreadContext::new(plain_reserved, plain_static, frame.bounds, plain_max_radius);
    plain_spread_context.set_static_units_iter(
        units
            .iter()
            .filter(|unit| unit.speed.is_none())
            .map(|unit| Some(unit.unit_id)),
    );
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
            arrival_slot: None,
            #[cfg(feature = "test-probes")]
            probe: super::probe::FrameFlags::default(),
        })
        .collect();
    let mut slot_claim_counts: std::collections::HashMap<(u64, u32, u32), usize> = units
        .iter()
        .filter(|unit| guide.kind(unit.unit_id) == super::motion::MotionKind::Destination)
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
    let mut target_claims: std::collections::HashMap<(u32, u32), [u32; 2]> =
        std::collections::HashMap::new();
    for unit in &units {
        if guide.kind(unit.unit_id) == super::motion::MotionKind::Destination {
            update_target_claims(&mut target_claims, unit.owner_slot, None, unit.target);
        }
    }

    // The certified stall clock of each mover at the start of the tick.
    let start_stall: Vec<Option<u16>> = units
        .iter()
        .map(|unit| {
            (unit.target.is_some() && unit.speed.is_some_and(|speed| speed > 0.0))
                .then(|| {
                    unit.progress
                        .as_ref()
                        .map(|progress| progress.stalled_ticks)
                        .or(unit
                            .plain_progress
                            .as_ref()
                            .map(|progress| progress.stalled_ticks))
                })
                .flatten()
        })
        .collect();
    let stationary_at_start: Vec<bool> = units.iter().map(super::route::stationary).collect();
    // For each stationary row: the other owners that have a body in contact range of it at
    // the start of the tick (sorted). Only such an owner may plan round this body
    // (`route::Planner`).
    let mut contact_witness: Vec<Vec<u8>> = vec![Vec::new(); units.len()];
    if exchanges_claims
        && start_stall.iter().any(Option::is_some)
        && units_at_start
            .iter()
            .any(|row| row.0 != units_at_start[0].0)
    {
        for index in 0..units.len() {
            if !stationary_at_start[index] {
                continue;
            }
            let (owner, radius) = units_at_start[index];
            let mut owners: Vec<u8> = grid
                .within_unsorted(
                    &points,
                    origins[index],
                    radius + max_radius + super::route::CONTACT_GAP,
                )
                .into_iter()
                .filter(|&other| {
                    units_at_start[other].0 != owner
                        && origins[index].distance(origins[other])
                            <= radius + units_at_start[other].1 + super::route::CONTACT_GAP
                })
                .map(|other| units_at_start[other].0)
                .collect();
            owners.sort_unstable();
            owners.dedup();
            contact_witness[index] = owners;
        }
    }
    for (&pool_id, index) in &mut pool_indexes {
        for slot_index in 0..index.points.len() {
            let point = index.points[slot_index].position;
            let assigned =
                assigned_slots.contains(&(pool_id, point.x.to_bits(), point.y.to_bits()));
            index.refresh_slot_clearance(
                slot_index,
                &units,
                &points,
                &grid,
                max_radius,
                assigned,
                &contact_witness,
            );
        }
    }
    // Rows that took the claim exchange and route path in this tick.
    let mut planned = vec![false; units.len()];
    // A mover in a queue waits for the mover in front of it. Its stall clock must not run
    // while a touching mover of its owner has a smaller clock: that mover still made progress
    // more recently. In a real deadlock the clocks become equal and then run together.
    let exchange_stall = (frame.tick_hz / 4).max(2);
    let queue_reach = max_radius * 2.0 + 0.05 + max_step;
    let queued =
        |index: usize, grid: &crate::grid::UniformGrid, points: &[crate::grid::GridPoint]| {
            let Some(own) = start_stall[index] else {
                return false;
            };
            // Below the exchange threshold the clock always runs: a jammed mover must become
            // eligible for an exchange and count as an obstacle for the routes of its peers.
            if own < exchange_stall {
                return false;
            }
            grid.within_unsorted(points, origins[index], queue_reach + max_step)
                .into_iter()
                .any(|other| {
                    other != index
                        && units_at_start[other].0 == units_at_start[index].0
                        && start_stall[other].is_some_and(|clock| clock < own)
                        && origins[index].distance(origins[other])
                            < units_at_start[index].1 + units_at_start[other].1 + 0.05
                })
        };
    let mut route_spent = BTreeMap::<u8, usize>::new();
    // Rows whose slot changed in this tick by an exchange that had no restart budget left.
    let mut exchanged = vec![false; units.len()];
    let mut claimants: Option<std::collections::HashMap<super::exchange::SlotKey, usize>> = None;
    for index in 0..units.len() {
        if index > 0 {
            let previous = &units[index - 1];
            if previous.speed.is_some()
                && !previous
                    .plain_progress
                    .as_ref()
                    .is_some_and(|progress| progress.arrival_yield)
            {
                plain_spread_context.reserve_unit(
                    previous.unit_id,
                    ReservedFootprint {
                        position: previous.position,
                        radius: previous.radius,
                    },
                );
            }
        }
        let mut unit = units[index].clone();
        let owns_destination = guide.kind(unit.unit_id) == super::motion::MotionKind::Destination;
        let Some(speed) = unit.speed else { continue };
        // This index resolves identities only; it never chooses an outcome.
        let known = |id: u32| {
            visibility.is_none_or(|basis| {
                let other = if let Some(rows) = &unsorted_unit_rows {
                    rows.binary_search_by_key(&id, |&row| frame.units[row].unit_id)
                        .ok()
                        .map(|index| &frame.units[rows[index]])
                } else {
                    frame
                        .units
                        .binary_search_by_key(&id, |other| other.unit_id)
                        .ok()
                        .map(|row| &frame.units[row])
                };
                other.is_some_and(|other| {
                    other.owner_slot == unit.owner_slot
                        || basis.unit_permitted(unit.owner_slot, id)
                        || unit.position.distance(other.position)
                            <= unit.radius + other.radius + super::route::CONTACT_GAP
                })
            })
        };
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
                    if owns_destination
                        && remove_pool_slot_claim(
                            &mut slot_claim_counts,
                            &mut pool_indexes,
                            progress.pool_id,
                            progress.slot,
                        )
                    {
                        assigned_slots.remove(&key);
                    }
                }
                units[index].physical_escape = None;
                units[index].progress = outputs[index].progress.clone();
                units[index].plain_progress = outputs[index].plain_progress.clone();
                // The remaining branches consume this local row, not the stored
                // Vec entry. Do not resurrect the discharged escape or its old
                // blocked group progress when ordinary motion resumes below.
                unit.physical_escape = None;
                unit.progress = outputs[index].progress.clone();
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
                    if owns_destination
                        && remove_pool_slot_claim(
                            &mut slot_claim_counts,
                            &mut pool_indexes,
                            progress.pool_id,
                            progress.slot,
                        )
                    {
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
                    &contact_witness,
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
                .within_unsorted(&points, unit.position, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    other_index != index
                        && unit.position.distance(units[other_index].position)
                            < unit.radius + units[other_index].radius - 1.0e-5
                });
            if !occupied {
                if owns_destination {
                    update_target_claims(&mut target_claims, unit.owner_slot, unit.target, None);
                }
                let old_key = (
                    progress.pool_id,
                    progress.slot.x.to_bits(),
                    progress.slot.y.to_bits(),
                );
                if owns_destination
                    && remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        progress.slot,
                    )
                {
                    assigned_slots.remove(&old_key);
                }
                outputs[index].target = None;
                outputs[index].progress = None;
                units[index].target = None;
                units[index].progress = None;
                continue;
            }
            let is_free = |slot: Vec2| {
                grid.within_unsorted(&points, slot, unit.radius + max_radius)
                    .into_iter()
                    .all(|other_index| {
                        other_index == index
                            || !known(units[other_index].unit_id)
                            || slot.distance(units[other_index].position)
                                >= unit.radius + units[other_index].radius
                    })
            };
            let old_slot_key = (
                progress.pool_id,
                progress.slot.x.to_bits(),
                progress.slot.y.to_bits(),
            );
            let target_is_claimed_by_other = owns_destination
                && unit.target.is_some_and(|target| {
                    target_claims
                        .get(&target_key(target))
                        .is_some_and(|counts| {
                            if visibility.is_some() {
                                counts[usize::from(unit.owner_slot != 0)] > 1
                            } else {
                                u64::from(counts[0]) + u64::from(counts[1]) > 1
                            }
                        })
                });
            let target_is_blocked_by_stationary = unit.target.is_some_and(|target| {
                grid.within_unsorted(&points, target, unit.radius + max_radius)
                    .into_iter()
                    .any(|other_index| {
                        if other_index == index || !known(units[other_index].unit_id) {
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
            let escape = retained_escape.or_else(|| {
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
                    &known,
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
                if owns_destination
                    && remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        progress.slot,
                    )
                {
                    assigned_slots.remove(&old_slot_key);
                }
                if owns_destination {
                    add_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        progress.pool_id,
                        escape,
                    );
                }
                if owns_destination {
                    assigned_slots.insert(escape_key);
                }
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
            if owns_destination {
                update_target_claims(
                    &mut target_claims,
                    unit.owner_slot,
                    unit.target,
                    (!reached).then_some(escape),
                );
            }
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
                &contact_witness,
            );
            units[index].target = outputs[index].target;
            units[index].progress = outputs[index].progress.clone();
            continue;
        }
        let planning_context = &plain_spread_context;
        if let Some(progress) = unit.plain_progress.clone() {
            let mut output = step_plain_progress(
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
                planning_context,
                &known,
                guide,
                budget,
            );
            if !owns_destination && !output.failed_blocked {
                retain_waypoint_motion(&mut output, unit.target);
            }
            outputs[index] = output.clone();
            if owns_destination {
                update_target_claims(
                    &mut target_claims,
                    unit.owner_slot,
                    unit.target,
                    output.target,
                );
            }
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
                &contact_witness,
            );
            continue;
        }
        let Some(target) = unit.target else { continue };
        let Some(mut progress) = unit.progress.clone() else {
            let target_occupied = planning_context.conflicts_with_reserved_filtered(
                target,
                unit.radius,
                unit.unit_id,
                &|id| !known(id),
            ) || grid
                .within_unsorted(&points, target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    other_index != index
                        && known(units[other_index].unit_id)
                        && !units[other_index]
                            .plain_progress
                            .as_ref()
                            .is_some_and(|progress| progress.arrival_yield)
                        && target.distance(units[other_index].position)
                            < unit.radius + units[other_index].radius - 1.0e-5
                });
            if owns_destination && target_occupied {
                let progress = make_plain_progress(&unit, target, planning_context, &known);
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
                    planning_context,
                    &known,
                    guide,
                    budget,
                );
                outputs[index] = output.clone();
                if owns_destination {
                    update_target_claims(
                        &mut target_claims,
                        unit.owner_slot,
                        unit.target,
                        output.target,
                    );
                }
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
                    &contact_witness,
                );
                continue;
            }
            let destination = target;
            let desired = destination - unit.position;
            let step_limit = speed * dt;
            let destination_free = grid
                .within_unsorted(&points, destination, unit.radius + max_radius)
                .into_iter()
                .all(|other_index| {
                    other_index == index
                        || destination.distance(units[other_index].position)
                            >= unit.radius + units[other_index].radius - 1.0e-5
                });
            let guided =
                super::motion::guided_position(&unit, destination, frame.bounds, dt, guide, budget);
            if guided.is_none()
                && (desired.length() <= step_limit || desired.length() <= 0.025)
                && destination_free
            {
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
                    &contact_witness,
                );
                continue;
            }
            let mut separated = false;
            let mut was_projected = false;
            let mut position;
            if let Some(next) = guided {
                position = next;
            } else {
                let desired_distance = desired.length();
                let mut velocity = if desired_distance > 1.0e-8 {
                    desired * (step_limit / desired_distance)
                } else {
                    Vec2::ZERO
                };
                let reach = separation_reach(
                    unit.radius,
                    max_radius,
                    unit.radius + max_radius + 0.05 + max_step,
                );
                for other_index in grid.nearby(&points, unit.position, reach) {
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
                position = if !separated && desired_distance > step_limit {
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
                for _ in 0..8 {
                    let mut projected = false;
                    for other_index in
                        grid.nearby(&points, position, unit.radius + max_radius + max_step)
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
                            } else {
                                super::arrival::separation_direction(&unit, other)
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
                // Eight projection passes can cycle between neighbours. Never commit an overlap
                // when the body still has its previous legal position.
                if body_overlaps_position(
                    position,
                    index,
                    unit.radius,
                    &units,
                    &points,
                    &grid,
                    max_radius,
                ) && !body_overlaps_position(
                    unit.position,
                    index,
                    unit.radius,
                    &units,
                    &points,
                    &grid,
                    max_radius,
                ) {
                    position = unit.position;
                }
            }
            #[cfg(feature = "test-probes")]
            {
                outputs[index].probe = super::probe::FrameFlags {
                    separated,
                    was_projected: Some(was_projected),
                };
            }
            // Arrival eligibility was checked before movement, so this step
            // may end early only if it reaches the destination exactly.
            let reached = position == destination;
            if reached && destination_free {
                position = destination;
            }
            outputs[index].position = position;
            outputs[index].target =
                (!(owns_destination && reached && destination_free)).then_some(destination);
            let modeled_steering = guided.is_some_and(|next| {
                next != step(
                    &MovementInput {
                        position: unit.position,
                        target: destination,
                        speed,
                    },
                    dt,
                )
                .position
            });
            if (separated || was_projected || modeled_steering) && !(reached && destination_free) {
                let mut plain = if owns_destination {
                    make_plain_progress(&unit, target, planning_context, &known)
                } else {
                    PlainMoveProgress {
                        arrival_yield: false,
                        original_point: target,
                        slot: destination,
                        pool_slots: Vec::new(),
                        gather_goal: None,
                        activity: MovementActivity::Moving,
                        stalled_ticks: 0,
                        best_distance: unit.position.distance(destination),
                        claim_count: 0,
                        failure_recorded: false,
                    }
                };
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
            if owns_destination {
                update_target_claims(
                    &mut target_claims,
                    unit.owner_slot,
                    unit.target,
                    outputs[index].target,
                );
            }
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
                &contact_witness,
            );
            continue;
        };
        // A claim is a plan. A mover that cannot reach its slot, or that stalls, takes the
        // nearest slot that it can reach before the current moving claimant (`exchange`).
        let mut steer: Option<Vec2> = None;
        // True when the bounded search finds no route to the own slot or to any other slot.
        let mut no_route = false;
        if exchanges_claims
            && owns_destination
            && progress.activity == MovementActivity::Moving
            && let Some(pool_index) = pool_indexes.get(&progress.pool_id)
        {
            planned[index] = true;
            let stall_ticks = exchange_stall;
            let planner = super::route::Planner {
                index,
                units: &units,
                points: &points,
                grid: &grid,
                max_radius,
                bounds: frame.bounds,
                stationary_at_start: &stationary_at_start,
                contact_witness: &contact_witness,
                start_stall: &start_stall,
                soft: true,
                stall_ticks,
            };
            let ahead = unit.position
                + (progress.slot - unit.position).clamp_length_max(super::route::LOOK_AHEAD);
            let own_clear = planner.chord_clear(unit.position, ahead);
            // Only a true stationary body changes a claim. A stalled mover can still move.
            let own_hard_clear = own_clear || planner.hard().chord_clear(unit.position, ahead);
            let stalled = progress.stalled_ticks >= stall_ticks;
            let (spacing, centre) = frame
                .pools
                .get(&progress.pool_id)
                .map_or((2.0 * unit.radius + 0.1, progress.slot), |pool| {
                    (pool.spacing, pool.point)
                });
            let arriving = own_clear && unit.position.distance(progress.slot) <= spacing * 1.5;
            let mut choice = None;
            let shorter_plan = stalled && own_hard_clear;
            if !own_hard_clear || shorter_plan || arriving {
                choice = super::exchange::select(
                    &planner,
                    super::exchange::Request {
                        own_reachable: own_hard_clear,
                        shorter_plan,
                        arriving,
                        spacing,
                        centre,
                    },
                    pool_index,
                    claimants.get_or_insert_with(|| super::exchange::claimants(&units)),
                    &assigned_slots,
                );
            }
            if choice.is_none() && !own_clear {
                let spent = route_spent.entry(unit.owner_slot).or_insert(0usize);
                // The cap can end a search before it knows the answer. Only a search that
                // ran to its end may report that no slot is reachable.
                let mut exhausted = false;
                steer = super::exchange::route_own(&planner, spent, &mut exhausted);
                // A slot that only stalled movers cut off is still the slot of this mover.
                if steer.is_none()
                    && !exhausted
                    && super::exchange::route_own(&planner.hard(), spent, &mut exhausted).is_none()
                    && !exhausted
                {
                    let mut waits = false;
                    if let Some(plan) = super::exchange::route_other(
                        &planner,
                        pool_index,
                        claimants.get_or_insert_with(|| super::exchange::claimants(&units)),
                        &assigned_slots,
                        spent,
                        &mut exhausted,
                        &mut waits,
                        false,
                    ) {
                        steer = Some(plan.waypoint);
                        choice = plan.claim;
                    } else {
                        no_route = !waits && !exhausted;
                    }
                }
                if steer.is_none() && exhausted && stalled && *spent < super::route::OWNER_CAP {
                    // The own route is unknown, not unreachable. A separate search may
                    // only take a proven route to an unclaimed slot, or beat a moving
                    // claimant's straight-line lower bound. Resting claims remain fixed.
                    let mut alternate_exhausted = false;
                    let mut waits = false;
                    if let Some(plan) = super::exchange::route_other(
                        &planner,
                        pool_index,
                        claimants.get_or_insert_with(|| super::exchange::claimants(&units)),
                        &assigned_slots,
                        spent,
                        &mut alternate_exhausted,
                        &mut waits,
                        true,
                    ) {
                        steer = Some(plan.waypoint);
                        choice = plan.claim;
                    }
                }
            }
            if let Some(super::exchange::Choice { peer, slot }) = choice {
                let released = progress.slot;
                let pool_id = progress.pool_id;
                let taken_key = super::exchange::slot_key(pool_id, slot);
                let released_key = super::exchange::slot_key(pool_id, released);
                if let Some(peer) = peer {
                    let peer_target = units[peer].target;
                    let peer_distance = units[peer].position.distance(released);
                    if let Some(state) = units[peer].progress.as_mut() {
                        state.slot = released;
                        exchanged[peer] = !super::exchange::restart_stall_clock(state);
                        // With no budget the old best distance stays the measure of progress.
                        state.best_distance = if exchanged[peer] {
                            state.best_distance.min(peer_distance)
                        } else {
                            peer_distance
                        };
                    }
                    units[peer].target = Some(released);
                    outputs[peer].progress = units[peer].progress.clone();
                    outputs[peer].target = Some(released);
                    update_target_claims(
                        &mut target_claims,
                        units[peer].owner_slot,
                        peer_target,
                        Some(released),
                    );
                    if let Some(rows) = claimants.as_mut() {
                        rows.insert(released_key, peer);
                    }
                } else {
                    if remove_pool_slot_claim(
                        &mut slot_claim_counts,
                        &mut pool_indexes,
                        pool_id,
                        released,
                    ) {
                        assigned_slots.remove(&released_key);
                    }
                    add_pool_slot_claim(&mut slot_claim_counts, &mut pool_indexes, pool_id, slot);
                    assigned_slots.insert(taken_key);
                    if let Some(rows) = claimants.as_mut() {
                        rows.remove(&released_key);
                    }
                }
                if let Some(rows) = claimants.as_mut() {
                    rows.insert(taken_key, index);
                }
                update_target_claims(&mut target_claims, unit.owner_slot, unit.target, Some(slot));
                progress.slot = slot;
                exchanged[index] = !super::exchange::restart_stall_clock(&mut progress);
                progress.best_distance = if exchanged[index] {
                    progress.best_distance.min(unit.position.distance(slot))
                } else {
                    unit.position.distance(slot)
                };
                unit.target = Some(slot);
                unit.progress = Some(progress.clone());
                units[index].target = unit.target;
                units[index].progress = unit.progress.clone();
            }
        }
        let target = unit.target.unwrap_or(target);
        let mut movement_target = target;
        let mut failed_blocked = false;
        let mut position = unit.position;
        let distance = position.distance(movement_target);
        let guided =
            super::motion::guided_position(&unit, movement_target, frame.bounds, dt, guide, budget);
        if guided.is_none() && (distance <= speed * dt || distance <= 0.025) {
            let occupied = grid
                .within_unsorted(&points, movement_target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    let other = &units[other_index];
                    other_index != index
                        && movement_target.distance(other.position)
                            < unit.radius + other.radius - 1.0e-5
                });
            if !occupied {
                if owns_destination {
                    update_target_claims(&mut target_claims, unit.owner_slot, unit.target, None);
                }
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
                    &contact_witness,
                );
                continue;
            }
        }

        let mut near_same_group = false;
        let mut separated = false;
        let step_limit = speed * dt;
        if let Some(next) = guided {
            // Modeling replaces pushes/projection, not the recovery policy's
            // own-pool contact classification. Use the same starting-distance
            // predicate; no peer target or body position chooses a new push.
            let reach = separation_reach(
                unit.radius,
                max_radius,
                unit.radius + max_radius + 0.05 + max_step,
            );
            near_same_group =
                grid.nearby(&points, unit.position, reach)
                    .into_iter()
                    .any(|other_index| {
                        if other_index == index {
                            return false;
                        }
                        let other = &units[other_index];
                        other.owner_slot == unit.owner_slot
                            && other.target.is_some()
                            && unit
                                .progress
                                .as_ref()
                                .is_some_and(|p| p.pool_id == progress.pool_id)
                            && other
                                .progress
                                .as_ref()
                                .is_some_and(|p| p.pool_id == progress.pool_id)
                            && unit.position.distance(other.position)
                                < unit.radius + other.radius + 0.05
                    });
            position = next;
        } else if let Some(waypoint) = steer {
            // A route leg has passed the exact check against every stationary own body.
            position = step(
                &MovementInput {
                    position: unit.position,
                    target: waypoint,
                    speed,
                },
                dt,
            )
            .position;
        } else {
            let desired = movement_target - position;
            let direction = desired.try_normalize().unwrap_or(Vec2::ZERO);
            // An occupied slot still limits the intended approach distance. Keep
            // far-step multiplication unchanged; separation/projection follow.
            let mut velocity = if distance <= speed * dt {
                direction * distance
            } else {
                direction * speed * dt
            };
            let reach = separation_reach(
                unit.radius,
                max_radius,
                unit.radius + max_radius + 0.05 + max_step,
            );
            let neighbours = grid.nearby(&points, position, reach);
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
                for other_index in grid.nearby(&points, position, broad_reach) {
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
                        } else {
                            super::arrival::separation_direction(&unit, other)
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
        }
        let next_distance = position.distance(movement_target);
        if owns_destination && position == movement_target {
            let occupied = grid
                .within_unsorted(&points, movement_target, unit.radius + max_radius)
                .into_iter()
                .any(|other_index| {
                    let other = &units[other_index];
                    other_index != index
                        && movement_target.distance(other.position)
                            < unit.radius + other.radius - 1.0e-5
                });
            if !occupied {
                if owns_destination {
                    update_target_claims(&mut target_claims, unit.owner_slot, unit.target, None);
                }
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
                    &contact_witness,
                );
                continue;
            }
        }
        // This clock comes from the previous certified trajectory. A raw chord
        // that the body guard will reject must not erase an already due recovery.
        let hz = frame.tick_hz.max(1);
        let recovery_due = progress.stalled_ticks >= hz;
        let made_progress = next_distance < progress.best_distance - 0.01 * step_limit;
        progress.stalling_on_group = near_same_group;
        if steer.is_some() && progress.stalled_ticks.saturating_add(1) >= hz {
            // A route does not extend a recovery that is due in this tick. The straight
            // distance of a routed step is no evidence of progress either.
            progress.stalled_ticks = progress.stalled_ticks.saturating_add(1);
        } else if steer.is_some()
            && position != unit.position
            && !super::exchange::budget_spent(&progress)
        {
            // A route leads away from the slot at first. The straight distance is no measure
            // of its progress. A clipped step gets its stall clock back in `sweep::finish`.
            // A mover with no restart budget left loses this rule: then only a new best
            // straight distance restarts its clock, so routes and exchanges that go round in
            // a circle end in the ordinary recovery.
            progress.best_distance = progress.best_distance.min(next_distance);
            progress.stalled_ticks = 0;
        } else if made_progress && !recovery_due {
            progress.best_distance = next_distance;
            progress.stalled_ticks = 0;
        } else if !(exchanges_claims && queued(index, &grid, &points)) {
            progress.stalled_ticks = progress.stalled_ticks.saturating_add(1);
        }
        if progress.stalled_ticks >= hz {
            if no_route {
                // Complete searches found no slot that this mover can reach. Another blind
                // claim only delays the failure report.
                progress.activity = MovementActivity::Blocked;
                failed_blocked = true;
            } else if progress.stalling_on_group && !progress.pass_mode {
                progress.pass_mode = true;
                progress.stalled_ticks = 0;
                progress.best_distance = next_distance;
            } else if super::exchange::spare_claims(&progress) < 3 {
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
                                    .within_unsorted(&points, slot, unit.radius + max_radius)
                                    .into_iter()
                                    .all(|other_index| {
                                        other_index == index
                                            // In exchange mode a body of another owner
                                            // counts only at rest and in contact range of
                                            // this owner, as for a route.
                                            || (exchanges_claims
                                                && units_at_start[other_index].0
                                                    != unit.owner_slot
                                                && !(stationary_at_start[other_index]
                                                    && contact_witness[other_index]
                                                        .binary_search(&unit.owner_slot)
                                                        .is_ok()))
                                            || slot.distance(units[other_index].position)
                                                >= unit.radius + units[other_index].radius
                                    })
                        },
                    )
                });
                if let Some(spare) = spare {
                    if owns_destination
                        && remove_pool_slot_claim(
                            &mut slot_claim_counts,
                            &mut pool_indexes,
                            progress.pool_id,
                            progress.slot,
                        )
                    {
                        assigned_slots.remove(&old_slot_key);
                    }
                    if owns_destination {
                        add_pool_slot_claim(
                            &mut slot_claim_counts,
                            &mut pool_indexes,
                            progress.pool_id,
                            spare,
                        );
                    }
                    if owns_destination {
                        assigned_slots.insert((
                            progress.pool_id,
                            spare.x.to_bits(),
                            spare.y.to_bits(),
                        ));
                    }
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
        if owns_destination {
            update_target_claims(
                &mut target_claims,
                unit.owner_slot,
                unit.target,
                outputs[index].target,
            );
        }
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
            &contact_witness,
        );
    }
    #[cfg(test)]
    RAW_APPROACH_PROBE.with(|probe| {
        if let Some((id, _)) = probe.get()
            && let Some(raw) = outputs.iter().find(|out| out.unit_id == id)
        {
            probe.set(Some((id, Some((raw.position, raw.progress.is_some())))));
        }
    });
    let raw_positions: Vec<Vec2> = if exchanges_claims {
        outputs.iter().map(|output| output.position).collect()
    } else {
        Vec::new()
    };
    guide.finish_prediction(budget);
    super::sweep::certify_guided(
        frame,
        &units,
        &origins,
        &mut outputs,
        &mut points,
        &mut grid,
        dt,
        max_radius,
        budget,
        guide,
    );
    if exchanges_claims {
        for index in 0..units.len() {
            let Some(clock) = start_stall[index] else {
                continue;
            };
            let held = queued(index, &grid, &points);
            let clipped = outputs[index].position != raw_positions[index];
            let at = outputs[index].position;
            let Some(progress) = outputs[index].progress.as_mut() else {
                continue;
            };
            // A planning mover ends the tick in contact range of a resting body of another
            // owner that its owner did not know at the start of the tick. Contact knowledge
            // has no memory, so such a find can repeat. Each find spends restart budget; a
            // mover with no budget left becomes due for the ordinary stall recovery.
            let find_without_budget = planned[index]
                && grid
                    .within_unsorted(
                        &points,
                        at,
                        units_at_start[index].1 + max_radius + super::route::CONTACT_GAP,
                    )
                    .into_iter()
                    .any(|other| {
                        units_at_start[other].0 != units_at_start[index].0
                            && stationary_at_start[other]
                            && contact_witness[other]
                                .binary_search(&units_at_start[index].0)
                                .is_err()
                            && at.distance(origins[other])
                                <= units_at_start[index].1
                                    + units_at_start[other].1
                                    + super::route::CONTACT_GAP
                    })
                && !super::exchange::spend_discovery(progress);
            if exchanged[index] && clipped {
                // The certificate refused the step toward the new slot, and this mover has
                // no restart budget left: the slot change alone is no progress.
                progress.stalled_ticks = if held { clock } else { clock.saturating_add(1) };
            } else if progress.stalled_ticks > clock && held {
                progress.stalled_ticks = clock;
            }
            if find_without_budget {
                // After the queue rule: a queue must not hold this clock back.
                progress.stalled_ticks = progress.stalled_ticks.max(frame.tick_hz);
            }
        }
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
    outputs.sort_unstable_by_key(|output| output.unit_id);
    for output in &mut outputs {
        output.arrival_slot = resolved_ties
            .binary_search_by_key(&output.unit_id, |(unit, _)| *unit)
            .ok()
            .map(|index| resolved_ties[index].1);
    }
    outputs
}

/// These loops read fixed initial body positions. All effects require distance
/// strictly below `(radius + other.radius) + .05`. Positive finite f32 addition
/// is monotone, so replacing other.radius by max_radius cannot lower that limit.
/// The successor keeps a conservative squared-distance broad phase at rounding
/// boundaries. Taking min with the old reach never introduces a new candidate.
/// Projection and sweep queries must retain their own wider moving-point bounds.
pub(super) fn separation_reach(radius: f32, max_radius: f32, old_reach: f32) -> f32 {
    #[cfg(test)]
    if LEGACY_SEPARATION_QUERY.with(std::cell::Cell::get) {
        return old_reach;
    }
    if old_reach.is_nan() {
        return old_reach;
    }
    old_reach.min((radius + max_radius + 0.05).next_up())
}
#[cfg(test)]
thread_local! {
    static LEGACY_SEPARATION_QUERY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(test)]
#[path = "separation_query_tests.rs"]
mod separation_query_tests;

#[cfg(test)]
type RawApproach = Option<(u32, Option<(Vec2, bool)>)>;
#[cfg(test)]
thread_local! {
    // Opt-in raw observation leaves the production continuous guard active.
    static RAW_APPROACH_PROBE: std::cell::Cell<RawApproach> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
mod group_approach_tests {
    use super::*;
    #[test]
    fn occupied_near_group_target_does_not_create_a_full_step_past_its_slot() {
        let mut results = Vec::new();
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
                progress: Some(super::super::MoveProgress {
                    pool_id: 7,
                    slot: target,
                    activity: MovementActivity::Moving,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: 1.,
                    claim_count: 0,
                    follow_origin: None,
                }),
                plain_progress: None,
                physical_escape: None,
            };
            let mut holder = actor.clone();
            holder.unit_id = if invert { 1 } else { 99 };
            holder.position = target;
            // A physically stopped peer keeps its own pending intent. It must
            // remain an obstacle in either enumeration order.
            holder.speed = Some(0.);
            holder.target = Some(Vec2::new(10., 5.));
            holder.progress = None;
            let frame = MovementFrame {
                bounds: Vec2::splat(16.),
                tick_hz: 20,
                units: if invert {
                    vec![holder.clone(), actor.clone()]
                } else {
                    vec![actor.clone(), holder.clone()]
                },
                pools: BTreeMap::from([(
                    7,
                    super::super::MovementPool {
                        pool_id: 7,
                        point: target,
                        spacing: 0.7,
                        primary_count: 1,
                        slots: vec![target],
                    },
                )]),
            };
            RAW_APPROACH_PROBE.with(|p| p.set(Some((actor.unit_id, None))));
            let outputs = step_frame_at_tick(&frame, 0.05, 1);
            let (raw, group) = RAW_APPROACH_PROBE
                .with(|p| p.replace(None))
                .unwrap()
                .1
                .unwrap();
            assert!(group, "regression must exercise the grouped branch");
            assert!(
                raw.x <= target.x,
                "near occupied group slot must not propose a five-metre overshoot: {raw:?}"
            );
            let out = outputs.iter().find(|u| u.unit_id == actor.unit_id).unwrap();
            let kept = outputs
                .iter()
                .find(|u| u.unit_id == holder.unit_id)
                .unwrap();
            assert_eq!(kept.position, holder.position);
            assert!(out.position.distance(holder.position) >= actor.radius + holder.radius);
            results.push((raw, out.position));
        }
        assert_eq!(results[0], results[1]);
    }
}
