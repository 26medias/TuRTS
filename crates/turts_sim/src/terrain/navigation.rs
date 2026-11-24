//! Shared permitted terrain guidance before the existing authoritative movement solver.
mod flock;
mod flock_step;
pub(crate) mod geometry;
mod packed;
mod shared;
mod storage;
use super::TerrainState;
use crate::movement::{MovementFrame, MovementFrameOutput, MovementFrameUnit};
use bevy_ecs::{prelude::Component, world::World};
use bevy_math::Vec2;
pub use flock::{FIELD_CREDIT, FlockAudit, FlockMemberAudit, privileged_flock_audit};
use geometry::{clear, physical_prefix, profile};
pub use shared::{
    NavigationAudit, NavigationDestinationRegionAudit, NavigationDiagnostics,
    NavigationMemberAudit, NavigationMemberState, privileged_contact_debug, privileged_diagnostics,
    privileged_navigation_audit,
};
pub(crate) use shared::{
    TerrainNavigation, dispatch_order_changed, order_changed, take_owner_changes,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
const MAX_SOLVER_RETRIES: usize = 8;
// Contact recording runs after the temporary guidance is dropped. Its output
// copy reuses the controller's per-body guidance+128-byte scratch reservation.
const _: () = assert!(
    // A solver may also retain one explicit tie record per body. Vec growth is
    // bounded by twice that count; it shares the existing per-body reservation.
    std::mem::size_of::<MovementFrameOutput>()
        + 2 * std::mem::size_of::<(u32, bevy_math::Vec2)>()
        + std::mem::size_of::<bevy_math::Vec2>()
        + std::mem::size_of::<usize>()
        + std::mem::size_of::<Option<bevy_math::Vec2>>()
        + 3 * std::mem::size_of::<bool>()
        <= std::mem::size_of::<crate::movement::MovementFrameUnit>() + 128
);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TerrainMotionWait;
/// Collision slots and route waypoints are not new order endpoints. Ordinary
/// Replace/Stop clears movement progress and retires the navigation epoch.
/// Seconds that a clear body can be held on its direct final leg, with no
/// approach of a quarter radius, before it stops where it stands.
const HELD_FINAL_LEG_SECONDS: u16 = 3;

/// A timeout alone cannot establish arrival. A stationary own body must occupy
/// the endpoint, and the current collision-safe pose must meet the geometric
/// lower bound on distance to it. This does not use unit IDs for priority.
fn held_position_is_nearest(
    frame: &MovementFrame,
    actor: &crate::movement::MovementFrameUnit,
    position: bevy_math::Vec2,
    goal: bevy_math::Vec2,
) -> bool {
    if !position.is_finite() || !goal.is_finite() || !actor.radius.is_finite() || actor.radius <= 0.
    {
        return false;
    }
    let mut lower_bound = 0.0f64;
    let mut occupied = false;
    for other in &frame.units {
        if other.unit_id == actor.unit_id
            || other.owner_slot != actor.owner_slot
            || other.target.is_some()
            || other.physical_escape.is_some()
            || !other.position.is_finite()
            || !other.radius.is_finite()
            || other.radius <= 0.
        {
            continue;
        }
        let separation = f64::from(actor.radius) + f64::from(other.radius);
        let goal_distance = (goal.as_dvec2() - other.position.as_dvec2()).length();
        if goal_distance < separation {
            occupied = true;
            lower_bound = lower_bound.max(separation - goal_distance);
        }
    }
    let distance = (position.as_dvec2() - goal.as_dvec2()).length();
    // Inputs and output poses are f32. Permit only rounding error at their scale.
    let scale = position
        .abs()
        .max(goal.abs())
        .max_element()
        .max(actor.radius)
        .max(1.);
    let tolerance = f64::from(scale) * f64::from(f32::EPSILON) * 8.;
    occupied && distance <= lower_bound + tolerance
}

fn requested_target(unit: &crate::movement::MovementFrameUnit) -> Option<bevy_math::Vec2> {
    unit.target.map(|target| {
        unit.plain_progress
            .as_ref()
            .map_or(target, |progress| progress.original_point)
    })
}

// Public order provenance survives group-to-plain local recovery. Neither a
// blocked local controller nor its physical escape changes the accepted Move.
fn parking_public_goal(
    world: &World,
    unit: &crate::movement::MovementFrameUnit,
    navigation: &TerrainNavigation,
) -> Option<bevy_math::Vec2> {
    if !navigation.continues_order(unit.unit_id, unit.owner_slot) {
        return None;
    }
    public_move_goal(world, unit.unit_id, unit.owner_slot)
}
fn public_move_goal(world: &World, id: u32, owner: u8) -> Option<bevy_math::Vec2> {
    let entity = crate::unit_entity(world, id)?;
    if world.get::<crate::Owner>(entity)?.0.0 != owner {
        return None;
    }
    let order = world.get::<crate::UnitOrders>(entity)?.current.as_ref()?;
    let crate::orders::OrderTargetSim::Point(goal) = order.target else {
        return None;
    };
    (order.behaviour == turts_mod_api::Behaviour::Move).then_some(goal)
}

// Plain ordinary Moves retain their accepted identity in this existing store.
// The navigation epoch and owner fence must still agree before using its goal.
fn passage_public_goal(
    world: &World,
    unit: &MovementFrameUnit,
    navigation: &TerrainNavigation,
) -> Option<Vec2> {
    if !navigation.continues_order(unit.unit_id, unit.owner_slot) {
        return None;
    }
    let entity = crate::unit_entity(world, unit.unit_id)?;
    if world.get::<crate::Owner>(entity)?.0.0 != unit.owner_slot {
        return None;
    }
    let current = world.get::<crate::UnitOrders>(entity)?.current.as_ref();
    let order = current.or_else(|| {
        world
            .get_resource::<crate::orders::PlainMoveOrderIdentities>()?
            .0
            .get(&unit.unit_id)
    })?;
    let crate::orders::OrderTargetSim::Point(point) = order.target else {
        return None;
    };
    (order.behaviour == turts_mod_api::Behaviour::Move).then_some(point)
}

/// Called only after a real ordinary Move spread dispatch commits its order.
/// Admission misses have no side map and cannot gain late reconstructed scope.
pub(crate) fn record_dispatch_parking_region(
    world: &mut World,
    members: &[crate::movement::SpreadMember],
    region: super::traffic::DestinationRegion,
) {
    if !world.contains_resource::<super::TerrainState>() {
        return;
    }
    // Dispatch is bounded by the existing cohort cap. This temporary batch and
    // rollback IDs reuse navigation's existing8MiB scratch phase reservation.
    let mut shapes = Vec::with_capacity(members.len());
    let mut cohort_owner = None;
    for member in members {
        let Some(entity) = crate::unit_entity(world, member.id) else {
            return;
        };
        let Some(owner) = world.get::<crate::Owner>(entity).map(|o| o.0.0) else {
            return;
        };
        if cohort_owner.is_some_and(|old| old != owner) {
            return;
        }
        cohort_owner = Some(owner);
        if public_move_goal(world, member.id, owner) != Some(region.goal) {
            return;
        }
        let Some(kind) = world.get::<crate::UnitType>(entity).map(|k| k.0.0) else {
            return;
        };
        let Some(radius) = world
            .get_resource::<crate::UnitTypes>()
            .and_then(|t| t.get(turts_mod_api::UnitTypeId(kind)))
            .map(|t| t.radius)
        else {
            return;
        };
        if radius.to_bits() != member.radius.to_bits() {
            return;
        }
        let unit = crate::movement::MovementFrameUnit {
            unit_id: member.id,
            owner_slot: owner,
            unit_type: kind,
            position: member.position,
            radius,
            speed: None,
            target: None,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        };
        let Some(profile) = profile(world, &unit) else {
            return;
        };
        shapes.push((member.id, kind, radius, profile));
    }
    let Some(owner) = cohort_owner else {
        return;
    };
    world.init_resource::<TerrainNavigation>();
    let tick = world.get_resource::<crate::Tick>().map_or(0, |t| t.0);
    if let Some(mut navigation) = world.get_resource_mut::<TerrainNavigation>() {
        navigation.record_dispatch_scopes(owner, region, &shapes, tick);
    }
}

/// Only a current positive parking certificate can supply this slot. Ordinary
/// private corners still use restore_requested_target. Reaching the authorized
/// point retires obsolete physical recovery before the next fixed tick.
fn accept_parking_arrival(
    original: &crate::movement::MovementFrameUnit,
    constrained: &crate::movement::MovementFrameUnit,
    output: &mut MovementFrameOutput,
    slot: Option<bevy_math::Vec2>,
) -> Option<bevy_math::Vec2> {
    let slot = slot?;
    if constrained.target != Some(slot)
        || original.speed != constrained.speed
        || output.position != slot
        || output.target.is_some()
        || output.failed_blocked
    {
        return None;
    }
    output.physical_escape = None;
    output.arrival_slot = Some(slot);
    Some(slot)
}

fn direct_request(
    original: &crate::movement::MovementFrameUnit,
    guide: &crate::movement::MovementFrameUnit,
) -> bool {
    requested_target(original).is_some_and(|point| point.is_finite())
        && guide.target == requested_target(original)
        && guide.speed == original.speed
        && original
            .speed
            .is_some_and(|speed| speed.is_finite() && speed > 0.)
}

fn retry_progress_step(
    original: &crate::movement::MovementFrameUnit,
    constrained: &crate::movement::MovementFrameUnit,
    initially_direct: bool,
    whole_frame_held: bool,
    dt: f32,
) -> Option<f32> {
    if !initially_direct
        || (!whole_frame_held
            && constrained.target == requested_target(original)
            && constrained.speed == original.speed)
    {
        return None;
    }
    original
        .speed
        .map(|speed| speed * dt)
        .filter(|step| step.is_finite() && *step > 0.)
}

fn account_retry_progress(
    original: &crate::movement::MovementFrameUnit,
    output: &mut MovementFrameOutput,
    terrain_retry_step: Option<f32>,
) {
    // A direct permitted request earned no raw progress after the final
    // terrain retry or its finite-safe whole-frame fallback. Its recovery clock
    // must use the final certified pose, not the
    // raw improvement discarded with the retry. Policy waits and temporary
    // route requests never supply this direct-row authority.
    if let Some(step) = terrain_retry_step.filter(|step| step.is_finite() && *step > 0.) {
        if let Some(progress) = output.progress.as_mut() {
            let previous = original
                .progress
                .as_ref()
                .map(|p| (p.slot, p.best_distance, p.stalled_ticks));
            (progress.best_distance, progress.stalled_ticks) = crate::movement::sweep::progress(
                previous,
                original.position,
                output.position,
                progress.slot,
                step,
            );
        }
        if let Some(progress) = output.plain_progress.as_mut() {
            let previous = original
                .plain_progress
                .as_ref()
                .map(|p| (p.slot, p.best_distance, p.stalled_ticks));
            (progress.best_distance, progress.stalled_ticks) = crate::movement::sweep::progress(
                previous,
                original.position,
                output.position,
                progress.slot,
                step,
            );
        }
    }
}

fn constrain_terrain_prefix(
    original: &crate::movement::MovementFrameUnit,
    candidate: &mut crate::movement::MovementFrameUnit,
    prefix: bevy_math::Vec2,
) {
    // A cached connector escape is uncommitted local intent. It must not run
    // ahead of this certified prefix. A real original body escape remains owed;
    // hold that unsafe body for this trial instead of cancelling its recovery.
    let follows_prefix = prefix != original.position && original.physical_escape.is_none();
    candidate.speed = follows_prefix.then_some(original.speed).flatten();
    candidate.target = follows_prefix.then_some(prefix);
    candidate.progress = None;
    candidate.plain_progress = None;
    candidate.physical_escape = original.physical_escape.clone();
}

fn restore_requested_target(
    frame: &MovementFrame,
    original: &crate::movement::MovementFrameUnit,
    constrained: &crate::movement::MovementFrameUnit,
    output: &mut MovementFrameOutput,
    arrival_slot: Option<bevy_math::Vec2>,
    terrain_retry_step: Option<f32>,
) {
    let Some(endpoint) = requested_target(original) else {
        return;
    };
    if output.position == endpoint {
        output.target = None;
        // The retained terrain order reached its exact endpoint after a local
        // recovery. Do not publish that old, unacknowledged local refusal as a
        // terminal failure now. Previously recorded failure history stays intact.
        if output.plain_progress.as_ref().is_some_and(|progress| {
            progress.activity == crate::movement::MovementActivity::Blocked
                && !progress.failure_recorded
        }) {
            output.plain_progress = None;
        }
        return;
    }
    if constrained.target != Some(endpoint) || original.speed != constrained.speed {
        // A temporary waypoint can only complete that segment, never the order.
        output.target = (output.position != endpoint).then_some(endpoint);
        output.progress = original.progress.clone();
        output.plain_progress = original.plain_progress.clone();
        account_retry_progress(original, output, terrain_retry_step);
        return;
    }
    if output.target.is_some() {
        output.target = Some(endpoint);
        account_retry_progress(original, output, terrain_retry_step);
        return;
    }
    if arrival_slot == Some(output.position) {
        return;
    }
    let resting =
        output.plain_progress.as_ref().is_some_and(|progress| {
            progress.activity == crate::movement::MovementActivity::Resting
        }) || output.progress.as_ref().is_some_and(|progress| {
            progress.activity == crate::movement::MovementActivity::Resting
        });
    let held = resting
        && frame.units.iter().any(|other| {
            other.unit_id != original.unit_id
                && (other.speed.is_none()
                    || other.target.is_none()
                    || other.progress.as_ref().is_some_and(|progress| {
                        progress.activity == crate::movement::MovementActivity::Resting
                    }))
                && endpoint.distance(other.position) < original.radius + other.radius
        });
    if held {
        // The ordinary closest-arrival solver keeps the existing physical holder.
        return;
    }
    // A moving obstruction may have disappeared while a fallback was approached.
    // Retry the same requested endpoint instead of completing at that detour.
    output.target = Some(endpoint);
    if let Some(progress) = output
        .progress
        .as_mut()
        .filter(|p| p.activity == crate::movement::MovementActivity::Resting)
    {
        progress.activity = crate::movement::MovementActivity::Moving;
        progress.slot = endpoint;
        progress.best_distance = output.position.distance(endpoint);
        progress.stalled_ticks = 0;
    }
    if let Some(progress) = output
        .plain_progress
        .as_mut()
        .filter(|p| p.activity == crate::movement::MovementActivity::Resting)
    {
        progress.activity = crate::movement::MovementActivity::Moving;
        progress.slot = endpoint;
        progress.original_point = endpoint;
        progress.best_distance = output.position.distance(endpoint);
        progress.stalled_ticks = 0;
    }
    // Retain the solver's new, accepted-position progress. Restoring stale
    // pre-step state would retry the same exhausted slot forever.
    output.failed_blocked = false;
}
/// The movement step of one tick: the flock (design record 2026-11-23). It is the only
/// movement path, on editable terrain and on a flat map.
pub(crate) fn step(
    world: &mut World,
    frame: &MovementFrame,
    dt: f32,
) -> (Vec<MovementFrameOutput>, BTreeSet<u32>) {
    flock::step(world, frame, dt)
}

#[cfg(test)]
pub(crate) use shared::debug_field_status;

#[cfg(test)]
#[path = "navigation/endpoint_tests.rs"]
mod endpoint_tests;

#[cfg(test)]
#[path = "navigation_fallback_tests.rs"]
mod fallback_tests;
