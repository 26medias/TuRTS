//! Test probes of the movement step (cargo feature `test-probes`).
//!
//! Pre-MP1 slice, task Q1b (plan refinement R11, design section 9 items 6, 7, 14
//! and 20). This is the only API that the trace emitter and the reference harness
//! use to read the movement state. A test never names a movement type that a later
//! task deletes, so the same emitter file runs on every build of the slice.
//!
//! Rules of this module (design 9.20):
//! - A probe reads. It never writes simulation state. The one write is the resource
//!   `LastFrameFlags`, which no simulation code reads.
//! - There is no `cfg(not(feature = "test-probes"))` anywhere. The feature only adds
//!   a field, a resource and a call that stores into them.
//! - No probe call is inside a condition or an expression whose value the simulation
//!   uses.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use bevy_ecs::world::{EntityRef, World};

use super::frame::MovementFrameOutput;
use super::plain::PlainMoveProgress;
use super::{MoveProgress, MovementActivity, MovementPools};
use crate::orders::PendingMovementOrders;
use crate::world::UnitId;

/// The flags of the last step of one unit.
///
/// Each flag is a value of one branch of the movement step. A unit that does not step
/// leaves `separated` `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameFlags {
    /// `separated`: a neighbour was closer than the separation limit.
    pub separated: bool,
    /// `was_projected`: the hard projection moved the unit in at least one of its
    /// passes. Each of the three branches (no pool, pool, plain) stores its real
    /// value after its projection loop. `None` when the unit did not reach a
    /// projection loop in the frame (no movement, a snap to the target before the
    /// loop, an escape step).
    pub was_projected: Option<bool>,
    /// How many times each shared phase ran for this unit in this frame.
    pub phases: PhaseCalls,
}

/// The calls of each shared phase of the free-movement step, for one unit in one frame
/// (pre-MP1 task A2a, design 9.5). A unit that enters the step passes `arrival` once; a
/// unit that is not snapped to its target passes `steering` and `projection` once; the
/// end of the step passes `stall` (a unit with a pool) or `fallback` (a unit with no pool,
/// or a pool unit that stalled to its deadline) at most once. A second branch that
/// repeats a phase gives a count of 2 here. A unit that does not enter the step, such as
/// an escape step or a plain-progress step, has all counts at 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhaseCalls {
    /// The unit entered the free-movement step.
    pub arrival: u8,
    /// The velocity, the separation from neighbours and the step or the cap.
    pub steering: u8,
    /// The hard projection loop.
    pub projection: u8,
    /// The progress bookkeeping of a unit with a pool (`best_distance`, `stalled_ticks`).
    pub stall: u8,
    /// The obstacle fallback decision: the pool deadline, or the plain progress.
    pub fallback: u8,
}

/// The flags of the last movement frame, by unit id. One entry for each unit of
/// the frame. The movement system replaces the whole map at each tick.
#[derive(Resource, Debug, Default)]
pub struct LastFrameFlags(pub BTreeMap<u32, FrameFlags>);

/// Store the flags of one frame. Called by the movement system after the frame.
pub(super) fn store_frame_flags(world: &mut World, outputs: &[MovementFrameOutput]) {
    if !world.contains_resource::<LastFrameFlags>() {
        world.insert_resource(LastFrameFlags::default());
    }
    let mut flags = world.resource_mut::<LastFrameFlags>();
    flags.0.clear();
    for output in outputs {
        flags.0.insert(output.unit_id, output.probe);
    }
}

fn unit_entity(world: &World, unit: u32) -> Option<EntityRef<'_>> {
    world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit))
}

/// The movement state of a unit: `"plain"` (a `PlainMoveProgress`), `"pool"` (a
/// `MoveProgress`) or `"none"`. A unit that has both reads `"plain"`, so that a
/// count of plain unit-ticks cannot hide one. A unit that does not exist reads
/// `"none"`.
pub fn unit_state(world: &World, unit: u32) -> &'static str {
    let Some(entity) = unit_entity(world, unit) else {
        return "none";
    };
    if entity.contains::<PlainMoveProgress>() {
        "plain"
    } else if entity.contains::<MoveProgress>() {
        "pool"
    } else {
        "none"
    }
}

/// The `MovementActivity` of the `MoveProgress` of a unit: `"Pending"`, `"Moving"`,
/// `"Resting"` or `"Blocked"`; `"none"` when the unit has no `MoveProgress`. It is
/// not the order or the gather stage (design 9.7).
pub fn activity(world: &World, unit: u32) -> &'static str {
    let Some(progress) = unit_entity(world, unit)
        .and_then(|entity| entity.get::<MoveProgress>().map(|p| p.activity))
    else {
        return "none";
    };
    match progress {
        MovementActivity::Pending => "Pending",
        MovementActivity::Moving => "Moving",
        MovementActivity::Resting => "Resting",
        MovementActivity::Blocked => "Blocked",
    }
}

/// `separated` of the last frame of a unit. `false` when the unit did not step.
pub fn separated(world: &World, unit: u32) -> bool {
    world
        .get_resource::<LastFrameFlags>()
        .and_then(|flags| flags.0.get(&unit))
        .is_some_and(|flags| flags.separated)
}

/// `was_projected` of the last frame of a unit: `Some(value)` for a unit that reached
/// a projection loop in the frame (any of the three branches stores its real value);
/// `None` for a unit that did not (no movement in the frame, a snap before the loop,
/// an escape step) and for a unit that is not in the last frame.
pub fn was_projected(world: &World, unit: u32) -> Option<bool> {
    world
        .get_resource::<LastFrameFlags>()
        .and_then(|flags| flags.0.get(&unit))
        .and_then(|flags| flags.was_projected)
}

/// The calls of each shared phase for a unit in the last frame (`PhaseCalls::default()` when
/// the unit did not step or is not in the last frame).
pub fn phase_calls(world: &World, unit: u32) -> PhaseCalls {
    world
        .get_resource::<LastFrameFlags>()
        .and_then(|flags| flags.0.get(&unit))
        .map_or_else(PhaseCalls::default, |flags| flags.phases)
}

/// The number of pools in the `MovementPools` resource.
pub fn pool_count(world: &World) -> usize {
    world
        .get_resource::<MovementPools>()
        .map_or(0, |pools| pools.0.len())
}

/// The number of units that wait in a pending movement envelope. The source is
/// `PendingMovementOrders.pending_units` (`orders.rs`): the sum over the slots.
pub fn pending_movement_orders(world: &World) -> usize {
    world
        .get_resource::<PendingMovementOrders>()
        .map_or(0, |pending| pending.pending_units.values().sum())
}

/// The fallback pools of one trip of one unit (pre-MP1 slice, task A2b1; gate 2 M1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackTrip {
    /// The unit id.
    pub unit: u32,
    /// The number of the trip of this unit, from 1.
    pub trip: u64,
    /// The fallback pools that the movement step gave to the unit in this trip.
    pub fallback_pools: u32,
}

/// Where each unit is in its trips, and the fallback pools of each trip (task A2b1). The
/// movement system writes it after each frame; no simulation code reads it.
#[derive(Resource, Debug, Default)]
pub struct FallbackTripLog {
    units: BTreeMap<u32, TripCursor>,
    pools: BTreeMap<(u32, u64), u32>,
}

#[derive(Debug, Default)]
struct TripCursor {
    trip: u64,
    /// The `MoveTarget` that the movement step left on the unit.
    target_after_step: Option<bevy_math::Vec2>,
    /// The gather leg of the unit at the last frame: the stage and the goal.
    leg: Option<(bool, super::PlainGatherGoal)>,
}

/// The gather leg of a unit: `(true, node)` on the way to a node, `(false, receiver)` on the way
/// to a receiver, `None` for any other stage or no gather order. The same source as the gather
/// protocol of `orders/run.rs`.
fn gather_leg(world: &World, unit: u32) -> Option<(bool, super::PlainGatherGoal)> {
    use crate::economy::gather::GatherStage;
    let order = world
        .get_resource::<crate::economy::EconomyOrders>()?
        .0
        .get(&unit)?;
    match order.stage {
        GatherStage::MovingToNode => order
            .deposit
            .map(|node| (true, super::PlainGatherGoal::Node(node.0))),
        GatherStage::MovingToReceiver => order
            .receiver
            .map(|receiver| (false, super::PlainGatherGoal::Receiver(receiver))),
        _ => None,
    }
}

/// Count the fallback pools of one frame by trip. `inputs` holds the unit id and the
/// `MoveTarget` of each unit at the start of the frame; `fallback_units` the units that the step
/// gave a pool of one. A unit stays in its trip while its gather leg is the same, or, with no
/// gather leg, while its `MoveTarget` is the one that the movement step left on it. Any other
/// change (a target that a writer other than the movement step sets, replaces or removes; a gather
/// leg that starts, ends or changes) starts a new trip.
pub(super) fn record_fallback_trips(
    world: &mut World,
    inputs: &[(u32, Option<bevy_math::Vec2>)],
    outputs: &[MovementFrameOutput],
    fallback_units: &[u32],
) {
    let legs: Vec<_> = inputs
        .iter()
        .map(|(unit, _)| gather_leg(world, *unit))
        .collect();
    if !world.contains_resource::<FallbackTripLog>() {
        world.insert_resource(FallbackTripLog::default());
    }
    let mut log = world.resource_mut::<FallbackTripLog>();
    for (((unit, target), leg), output) in inputs.iter().zip(legs).zip(outputs) {
        debug_assert_eq!(*unit, output.unit_id);
        let cursor = log.units.entry(*unit).or_default();
        let same_trip = match leg {
            Some(leg) => cursor.trip > 0 && cursor.leg == Some(leg),
            None => {
                cursor.trip > 0
                    && cursor.leg.is_none()
                    && target.is_some()
                    && *target == cursor.target_after_step
            }
        };
        if !same_trip {
            cursor.trip += 1;
        }
        cursor.leg = leg;
        cursor.target_after_step = output.target;
        let trip = cursor.trip;
        if fallback_units.contains(unit) {
            *log.pools.entry((*unit, trip)).or_default() += 1;
        }
    }
}

/// The trips that made at least one fallback pool, in (unit, trip) order. A trip ends when the
/// `MoveTarget` of its unit is removed or replaced by a writer other than the movement step, or
/// when its gather leg ends (`record_fallback_trips`).
pub fn fallback_trips(world: &World) -> Vec<FallbackTrip> {
    world
        .get_resource::<FallbackTripLog>()
        .map(|log| {
            log.pools
                .iter()
                .map(|(&(unit, trip), &fallback_pools)| FallbackTrip {
                    unit,
                    trip,
                    fallback_pools,
                })
                .collect()
        })
        .unwrap_or_default()
}
