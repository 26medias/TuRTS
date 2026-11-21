//! Shared permitted terrain guidance before the existing authoritative movement solver.
pub(crate) mod geometry;
mod shared;
mod storage;
use super::TerrainState;
use crate::movement::{MovementFrame, MovementFrameOutput, MovementFrameUnit};
use bevy_ecs::{prelude::Component, world::World};
use bevy_math::Vec2;
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
/// Reuse the existing frame solver. Unsafe proposals become stopped physical obstacles before
/// a retry, so a following actor cannot move into a position later rolled back underneath it.
pub(crate) fn step(
    world: &mut World,
    frame: &MovementFrame,
    dt: f32,
    preserve: &BTreeSet<u32>,
) -> (Vec<MovementFrameOutput>, BTreeSet<u32>) {
    if world.get_resource::<TerrainState>().is_none() {
        return (
            crate::movement::step_frame_with_motion_guide_and_visibility(
                frame,
                dt,
                preserve,
                &mut crate::movement::sweep::SweepBudget::new(
                    world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0),
                ),
                &mut crate::movement::motion::NoMotionGuide,
                crate::visibility::active_basis(world)
                    .map(|basis| basis as &dyn crate::movement::MovementVisibility),
            ),
            BTreeSet::new(),
        );
    }
    let mut navigation = world
        .remove_resource::<TerrainNavigation>()
        .unwrap_or_default();
    let mut traffic = world
        .remove_resource::<super::traffic::TrafficState>()
        .unwrap_or_default();
    let (outputs, waiting) = {
        let terrain = world.resource::<TerrainState>();
        let start_phase = super::phase(world, turts_mod_api::ObservationPhase::Start);
        let profiles = frame
            .units
            .iter()
            .map(|unit| profile(world, unit))
            .collect::<Vec<_>>();
        let mut body_budget = crate::movement::sweep::SweepBudget::new(
            world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0),
        );
        // Leave useful frozen-view steering work in the existing optional pool.
        for owner in 0..turts_mod_api::SLOT_COUNT {
            assert!(body_budget.reserve_model_amount(owner, super::traffic::GUIDED_RESERVE));
        }
        let mut candidate = frame.clone();
        let mut waiting = BTreeSet::new();
        let mut certified_passage = super::traffic::public_passage::no_certified_legs();
        if let Some(phase) = start_phase.as_ref() {
            navigation.propose(world, frame, phase, &profiles, &mut candidate, &mut waiting);
        } else {
            for unit in &mut candidate.units {
                if unit.target.is_some() {
                    waiting.insert(unit.unit_id);
                    unit.target = None;
                    unit.speed = None;
                }
            }
        }
        let passage_ready = candidate
            .units
            .iter()
            .take(super::traffic::index_capacity())
            .map(|u| u.target.is_some() && u.speed.is_some())
            .collect::<Vec<_>>();
        if let Some(phase) = start_phase.as_ref() {
            navigation.invalidate_contact_history(frame, &mut traffic);
            // Planning and observation must describe the same immutable phase.
            // The provider below never reads live foreign entity state.
            let observed_basis =
                crate::visibility::basis(world, turts_mod_api::ObservationPhase::Start).filter(
                    |basis| {
                        phase.knowledge_basis() == Some(basis.stamp())
                            && basis.stamp().phase == turts_mod_api::ObservationPhase::Start
                            && basis.stamp().tick == world.resource::<crate::Tick>().0
                    },
                );
            let observed_catalog = world.get_resource::<crate::bootstrap::InstalledCatalog>();
            let all_visible = !world.contains_resource::<crate::visibility::VisibilityState>()
                && phase.tick() == world.resource::<crate::Tick>().0
                && phase.observation_phase() == turts_mod_api::ObservationPhase::Start;
            let apply_traffic =
                |guidance: &MovementFrame,
                 candidate: &mut MovementFrame,
                 traffic: &mut super::traffic::TrafficState,
                 navigation: &mut TerrainNavigation,
                 body_budget: &mut crate::movement::sweep::SweepBudget| {
                    // Both sequential callbacks share one removed match-local resource.
                    // No RefCell survives this synchronous controller call.
                    let navigation = RefCell::new(navigation);
                    let captured_body = |fact: &crate::visibility::VisibleUnit| {
                        let definition = observed_catalog?.0.units.get(fact.content.as_ref())?;
                        let position = bevy_math::Vec2::new(fact.position.x, fact.position.y);
                        let turts_mod_api::Footprint::Circle { radius } = definition.footprint;
                        (position.is_finite() && radius.is_finite() && radius > 0.)
                            .then_some(super::traffic::ObservedBody { position, radius })
                    };
                    // The immutable world borrow pins explicit AllVisible facts
                    // until planning ends. Restricted inputs never use this port.
                    let public_body = |viewer, id| {
                        let entity = crate::unit_entity(world, id)?;
                        if world.get::<crate::Owner>(entity)?.0.0 == viewer {
                            return None;
                        }
                        let position = world.get::<crate::Position>(entity)?.0;
                        let kind = world.get::<crate::UnitType>(entity)?.0;
                        let radius = world.resource::<crate::UnitTypes>().get(kind)?.radius;
                        (position.is_finite() && radius.is_finite() && radius > 0.)
                            .then_some(super::traffic::ObservedBody { position, radius })
                    };
                    super::traffic::apply_guided_with_parking(
                        guidance,
                        candidate,
                        dt,
                        world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0),
                        traffic,
                        |index, from, to| {
                            navigation.borrow().contact_ready(
                                frame.units[index].unit_id,
                                frame.units[index].owner_slot,
                                phase,
                            ) && profiles[index].is_some_and(|profile| {
                                clear(
                                    &geometry::Planning {
                                        phase,
                                        viewer: frame.units[index].owner_slot,
                                    },
                                    from,
                                    to,
                                    frame.units[index].radius,
                                    profile,
                                    &mut BTreeMap::new(),
                                )
                            })
                        },
                        |index, from, to, axis| {
                            profiles[index].map_or(
                                super::traffic::EntranceContact::Unresolved,
                                |profile| {
                                    navigation.borrow_mut().observe_contact(
                                        phase,
                                        &frame.units[index],
                                        from,
                                        to,
                                        axis,
                                        profile,
                                    )
                                },
                            )
                        },
                        |index| {
                            let unit = &frame.units[index];
                            // The group kernel takes a spare slot and restarts
                            // its clock at one second, so a full second is never
                            // visible here. Half a second held asks for a detour.
                            unit.progress.as_ref().is_some_and(|p| {
                                p.activity == crate::movement::MovementActivity::Blocked
                                    || p.stalled_ticks >= (frame.tick_hz / 2).max(1)
                            }) || unit.plain_progress.as_ref().is_some_and(|p| {
                                p.activity == crate::movement::MovementActivity::Blocked
                                    || p.stalled_ticks >= frame.tick_hz.max(1)
                            }) || navigation.borrow().waypoint_needs_detour(
                                unit.unit_id,
                                unit.owner_slot,
                                // The local solver retries a blocked waypoint at one
                                // second. Admit detours before that restart so peers
                                // whose clocks differ can both be recognized as stalled.
                                (frame.tick_hz / 2).max(1),
                            )
                        },
                        body_budget,
                        |viewer, id| {
                            if all_visible {
                                return public_body(viewer, id);
                            }
                            let fact = observed_basis.as_ref()?.permitted_unit(viewer, id)?;
                            (fact.owner != viewer)
                                .then(|| captured_body(fact))
                                .flatten()
                        },
                        |index| frame.units[index].target.is_some(),
                        |viewer, emit| {
                            if all_visible {
                                for unit in frame.units.iter().filter(|u| u.owner_slot != viewer) {
                                    let Some(body) = public_body(viewer, unit.unit_id) else {
                                        return false;
                                    };
                                    if !emit(unit.unit_id, body) {
                                        return false;
                                    }
                                }
                                return true;
                            }
                            let Some(basis) = observed_basis.as_ref() else {
                                return false;
                            };
                            // Enumerate the capture itself. A legal construction
                            // cancellation can remove a live site after Start.
                            for fact in basis.permitted_units(viewer).filter(|f| f.owner != viewer)
                            {
                                let Some(body) = captured_body(fact) else {
                                    return false;
                                };
                                if !emit(fact.unit, body) {
                                    return false;
                                }
                            }
                            true
                        },
                        |index| {
                            let navigation = navigation.borrow();
                            let unit = &frame.units[index];
                            let goal = parking_public_goal(world, unit, &navigation)?;
                            navigation.parking_region(unit, goal, profiles[index]?)
                        },
                        |index| {
                            let unit = &frame.units[index];
                            unit.physical_escape.is_none()
                                && (unit.target.is_none()
                                    || unit.progress.as_ref().is_some_and(|p| {
                                        p.activity == crate::movement::MovementActivity::Resting
                                    })
                                    || unit.plain_progress.as_ref().is_some_and(|p| {
                                        p.activity == crate::movement::MovementActivity::Resting
                                    }))
                        },
                    )
                };
            if frame.units.len() > super::traffic::index_capacity() {
                // The controller refuses this physical size before any index
                // allocation. Borrow it directly rather than clone oversized guidance.
                waiting.extend(apply_traffic(
                    frame,
                    &mut candidate,
                    &mut traffic,
                    &mut navigation,
                    &mut body_budget,
                ));
            } else {
                let guidance = MovementFrame {
                    recovery: Default::default(),
                    bounds: frame.bounds,
                    tick_hz: frame.tick_hz,
                    pools: BTreeMap::new(),
                    units: frame
                        .units
                        .iter()
                        .map(|unit| {
                            let active = navigation.allows_traffic(unit.unit_id, unit.owner_slot);
                            crate::movement::MovementFrameUnit {
                                unit_id: unit.unit_id,
                                owner_slot: unit.owner_slot,
                                unit_type: unit.unit_type,
                                position: unit.position,
                                radius: unit.radius,
                                speed: active.then_some(unit.speed).flatten(),
                                target: active.then(|| requested_target(unit)).flatten(),
                                progress: None,
                                plain_progress: None,
                                physical_escape: None,
                            }
                        })
                        .collect(),
                };
                waiting.extend(apply_traffic(
                    &guidance,
                    &mut candidate,
                    &mut traffic,
                    &mut navigation,
                    &mut body_budget,
                ));
            }
            let passage_active = frame
                .units
                .iter()
                .take(super::traffic::index_capacity())
                .map(|unit| {
                    passage_public_goal(world, unit, &navigation).is_some()
                        && navigation.allows_traffic(unit.unit_id, unit.owner_slot)
                        && !unit.progress.as_ref().is_some_and(|p| {
                            p.activity == crate::movement::MovementActivity::Resting
                        })
                        && !unit.plain_progress.as_ref().is_some_and(|p| {
                            p.activity == crate::movement::MovementActivity::Resting
                        })
                })
                .collect::<Vec<_>>();
            let passage_nav = RefCell::new(&mut navigation);
            super::traffic::public_passage::apply(
                frame,
                &mut candidate,
                &mut certified_passage,
                &mut traffic,
                &mut waiting,
                phase.tick(),
                dt,
                &mut body_budget,
                all_visible,
                |index| passage_active[index],
                |index| passage_ready[index],
                |viewer, emit| {
                    super::traffic::velocity::observed(world, frame, phase, viewer, emit)
                },
                |_viewer, id| {
                    // This port is used only for explicit AllVisible. Kind and
                    // authored movement profile are public physical properties.
                    let entity = crate::unit_entity(world, id)?;
                    let kind = world.get::<crate::UnitType>(entity)?.0;
                    let definition = world.resource::<crate::UnitTypes>().get(kind)?;
                    let speed = definition
                        .movement
                        .as_ref()
                        .map_or(0., |movement| movement.speed);
                    (speed.is_finite() && speed >= 0.).then_some([
                        kind.0,
                        speed.to_bits(),
                        u32::from(world.get::<crate::Owner>(entity)?.0.0),
                    ])
                },
                |index, physical_radius, charge| {
                    let unit = &frame.units[index];
                    let navigation = passage_nav.borrow();
                    let goal = passage_public_goal(world, unit, &navigation)?;
                    let profile = profiles[index]?;
                    let radius = navigation
                        .parking_region(unit, goal, profile)
                        .map_or(unit.radius, |scope| scope.cohort_radius.max(unit.radius))
                        .max(physical_radius);
                    drop(navigation);
                    passage_nav
                        .borrow_mut()
                        .public_throat(phase, unit, goal, radius, profile, charge)
                },
                |observer, id, from, to, radius, probe_radius, axis| {
                    // Borrow the owned observer's contact lease. The observed
                    // actor contributes only CURRENT public shape/profile.
                    let Some(entity) = crate::unit_entity(world, id) else {
                        return super::traffic::EntranceContact::Unresolved;
                    };
                    let Some(kind) = world.get::<crate::UnitType>(entity) else {
                        return super::traffic::EntranceContact::Unresolved;
                    };
                    let caller = &frame.units[observer];
                    let body = MovementFrameUnit {
                        unit_id: caller.unit_id,
                        owner_slot: caller.owner_slot,
                        unit_type: kind.0.0,
                        position: from,
                        radius,
                        speed: None,
                        target: None,
                        progress: None,
                        plain_progress: None,
                        physical_escape: None,
                    };
                    let Some(profile) = profile(world, &body) else {
                        return super::traffic::EntranceContact::Unresolved;
                    };
                    passage_nav.borrow_mut().observe_public_contact(
                        phase,
                        &body,
                        from,
                        to,
                        axis,
                        probe_radius,
                        profile,
                    )
                },
                |index, from, to| {
                    profiles[index].is_some_and(|profile| {
                        clear(
                            &geometry::Planning {
                                phase,
                                viewer: frame.units[index].owner_slot,
                            },
                            from,
                            to,
                            frame.units[index].radius,
                            profile,
                            &mut BTreeMap::new(),
                        )
                    })
                },
            );
            #[allow(
                clippy::drop_non_drop,
                reason = "End the RefCell borrow before reusing navigation."
            )]
            drop(passage_nav);
            for (index, (original, proposed)) in
                frame.units.iter().zip(&mut candidate.units).enumerate()
            {
                if certified_passage.get(index).copied().unwrap_or(false) {
                    navigation.load_certified_waypoint_motion(original, proposed);
                } else {
                    navigation.load_waypoint_motion(
                        original,
                        proposed,
                        waiting.contains(&original.unit_id),
                    );
                }
            }
        }
        let tick = world.get_resource::<crate::Tick>().map_or(0, |t| t.0);
        let permitted_clear = |index: usize, from: bevy_math::Vec2, to: bevy_math::Vec2| {
            start_phase.as_ref().is_some_and(|phase| {
                profiles[index].is_some_and(|profile| {
                    clear(
                        &geometry::Planning {
                            phase,
                            viewer: frame.units[index].owner_slot,
                        },
                        from,
                        to,
                        frame.units[index].radius,
                        profile,
                        &mut BTreeMap::new(),
                    )
                })
            })
        };
        let mut model_foreign =
            |viewer, emit: &mut dyn FnMut(u32, super::traffic::ObservedBody) -> bool| {
                start_phase.as_ref().is_some_and(|phase| {
                    super::traffic::velocity::observed(world, frame, phase, viewer, emit)
                })
            };
        for owner in 0..turts_mod_api::SLOT_COUNT {
            body_budget.release_model(owner, super::traffic::GUIDED_RESERVE);
        }
        let parking_slots: Vec<_> = frame
            .units
            .iter()
            .take(super::traffic::index_capacity())
            .map(|u| traffic.parking_slot(u.unit_id))
            .collect();
        // One exact-width flag per admitted body, charged by the existing
        // guidance+128-byte per-body scratch reserve (compile assertion above).
        // No unbounded allocation is made for a refused oversized frame.
        let direct_rows: Vec<bool> = frame
            .units
            .iter()
            .zip(&candidate.units)
            .take(super::traffic::index_capacity())
            .map(|(original, guide)| direct_request(original, guide))
            .collect();
        let mut model_clear = |index, from, to| permitted_clear(index, from, to);
        let mut model = super::traffic::velocity::Guide::capture(
            &candidate,
            dt,
            tick,
            &mut traffic,
            &mut body_budget,
            &mut model_foreign,
            |i| {
                profiles[i].is_some()
                    && navigation.allows_traffic(frame.units[i].unit_id, frame.units[i].owner_slot)
            },
            |i| {
                if certified_passage.get(i).copied().unwrap_or(false) {
                    crate::movement::motion::MotionKind::CertifiedWaypoint
                } else if candidate.units[i].target != frame.units[i].target
                    && parking_slots.get(i).copied().flatten() != candidate.units[i].target
                {
                    crate::movement::motion::MotionKind::Waypoint
                } else {
                    crate::movement::motion::MotionKind::Destination
                }
            },
            |i| {
                parking_slots.get(i).copied().flatten().unwrap_or_else(|| {
                    requested_target(&frame.units[i]).unwrap_or(frame.units[i].position)
                })
            },
            |i| {
                let original = &frame.units[i];
                if waiting.contains(&original.unit_id)
                    || original.physical_escape.is_some()
                    || original.plain_progress.is_some()
                {
                    return None;
                }
                original
                    .progress
                    .as_ref()
                    .filter(|p| {
                        p.activity != crate::movement::MovementActivity::Resting
                            && !p.pass_mode
                            && p.follow_origin.is_none()
                    })
                    .map(|p| p.pool_id)
            },
            &mut model_clear,
        );
        let mut first_intent = None;
        let mut accepted = None;
        for _ in 0..MAX_SOLVER_RETRIES {
            let proposed = crate::movement::step_frame_with_motion_guide_and_visibility(
                &candidate,
                dt,
                preserve,
                &mut body_budget,
                &mut model,
                crate::visibility::active_basis(world)
                    .map(|basis| basis as &dyn crate::movement::MovementVisibility),
            );
            if first_intent.is_none() && candidate.units.len() <= super::traffic::index_capacity() {
                first_intent = Some(
                    frame
                        .units
                        .iter()
                        .zip(&candidate.units)
                        .zip(&proposed)
                        .enumerate()
                        .filter_map(|(index, ((original, guide), intent))| {
                            navigation
                                .prepare_waypoint_motion(
                                    original,
                                    guide,
                                    intent,
                                    waiting.contains(&original.unit_id),
                                )
                                .map(|prepared| (index, prepared))
                        })
                        .collect::<Vec<_>>(),
                );
            }
            let mut unsafe_indices = Vec::new();
            for (index, (unit, output)) in frame.units.iter().zip(&proposed).enumerate() {
                if output.position == unit.position {
                    continue;
                }
                let mut lease = BTreeMap::new();
                if profiles[index].is_none_or(|profile| {
                    !clear(
                        terrain,
                        unit.position,
                        output.position,
                        unit.radius,
                        profile,
                        &mut lease,
                    )
                }) {
                    unsafe_indices.push((index, output.position));
                }
            }
            if unsafe_indices.is_empty() {
                accepted = Some(proposed);
                break;
            }
            navigation.diagnostics.solver_retries =
                navigation.diagnostics.solver_retries.saturating_add(1);
            for (index, proposed) in unsafe_indices {
                let original = &frame.units[index];
                let prefix = profiles[index].map_or(original.position, |profile| {
                    physical_prefix(
                        terrain,
                        original.position,
                        proposed,
                        original.radius,
                        profile,
                    )
                });
                constrain_terrain_prefix(original, &mut candidate.units[index], prefix);
                model.mark_waypoint(original.unit_id);
                waiting.insert(frame.units[index].unit_id);
            }
        }
        // Explicit retry exhaustion, rather than a navigation or traffic wait.
        // Direct rows held by another row's failed retry also earned no movement.
        let whole_frame_held = accepted.is_none();
        let mut outputs = accepted.unwrap_or_else(|| {
            // Finite safe fallback. No pose has been committed and the original bodies still
            // occupy these positions. Preserve every command for later route revalidation.
            frame
                .units
                .iter()
                .map(|unit| {
                    if unit.target.is_some() {
                        waiting.insert(unit.unit_id);
                    }
                    MovementFrameOutput {
                        // A held frame keeps each recovery job unchanged.
                        recovery: frame.recovery.get(&unit.unit_id).copied().map(Box::new),
                        unit_id: unit.unit_id,
                        position: unit.position,
                        target: unit.target,
                        progress: unit.progress.clone(),
                        plain_progress: unit.plain_progress.clone(),
                        physical_escape: unit.physical_escape.clone(),
                        failed_blocked: false,
                        arrival_slot: None,
                        #[cfg(feature = "test-probes")]
                        probe: Default::default(),
                    }
                })
                .collect()
        });
        if let Some(prepared) = first_intent {
            for (index, motion) in prepared {
                if navigation.commit_waypoint_motion(
                    &frame.units[index],
                    motion,
                    &outputs[index],
                    dt,
                ) {
                    waiting.insert(frame.units[index].unit_id);
                }
            }
        }
        if frame.units.len() <= super::traffic::index_capacity()
            && outputs.len() <= super::traffic::index_capacity()
            && let Some(phase) = start_phase.as_ref()
        {
            // Only pose/identity are consumed. Keep command/search allocations in
            // their original output rows, and never clone nested progress here.
            let mut contacts = outputs
                .iter()
                .map(|output| MovementFrameOutput {
                    recovery: Default::default(),
                    unit_id: output.unit_id,
                    position: output.position,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                    failed_blocked: false,
                    arrival_slot: None,
                    #[cfg(feature = "test-probes")]
                    probe: Default::default(),
                })
                .collect::<Vec<_>>();
            contacts.sort_unstable_by_key(|output| output.unit_id);
            let preserve_public_contact = frame
                .units
                .iter()
                .map(|u| traffic.preserves_public_contact(u))
                .collect::<Vec<_>>();
            super::traffic::record_committed_contacts(
                frame,
                &contacts,
                world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0),
                &mut traffic,
                |index, from, to, axis| {
                    let unit = &frame.units[index];
                    let observed = profiles[index]
                        .map_or(super::traffic::EntranceContact::Unresolved, |profile| {
                            navigation.observe_contact(phase, unit, from, to, axis, profile)
                        });
                    if from == to
                        && observed == super::traffic::EntranceContact::Outside
                        && !preserve_public_contact[index]
                    {
                        navigation.contact_departed(unit.unit_id, unit.owner_slot);
                    }
                    observed
                },
            );
        }
        for (index, ((original, constrained), output)) in frame
            .units
            .iter()
            .zip(&candidate.units)
            .zip(&mut outputs)
            .enumerate()
        {
            // The clock runs only on a direct row: the frame target is the
            // order endpoint and no policy wait or waypoint replaced it. A
            // whole-frame hold says nothing about this body, so it is skipped.
            if !whole_frame_held {
                let endpoint = requested_target(original);
                let held = endpoint
                    .filter(|_| {
                        constrained.target == endpoint
                            && original.speed == constrained.speed
                            && output.target == endpoint
                            && original.physical_escape.is_none()
                            && output.physical_escape.is_none()
                            && output.plain_progress.as_ref().is_some_and(|p| {
                                p.activity == crate::movement::MovementActivity::Blocked
                                    && !p.arrival_yield
                            })
                    })
                    .map(|endpoint| output.position.distance(endpoint));
                if navigation.held_final_leg(
                    original.unit_id,
                    original.owner_slot,
                    held,
                    original.radius,
                    frame.tick_hz.max(1).saturating_mul(HELD_FINAL_LEG_SECONDS),
                ) && (endpoint.is_some_and(|goal| {
                    held_position_is_nearest(frame, original, output.position, goal)
                }) || parking_public_goal(world, original, &navigation)
                    .filter(|&goal| Some(goal) != endpoint)
                    .is_some_and(|goal| {
                        // A private slot does not replace the player's point. A
                        // current holder at that point can prove this closest stop.
                        held_position_is_nearest(frame, original, output.position, goal)
                            && permitted_clear(index, output.position, goal)
                    }))
                {
                    // Explicit physical arrival at the current certified pose.
                    output.target = None;
                    output.arrival_slot = Some(output.position);
                    if let Some(progress) = output.plain_progress.as_mut() {
                        progress.activity = crate::movement::MovementActivity::Resting;
                    }
                }
            }
            if constrained.target == requested_target(original)
                && original.speed == constrained.speed
                && let Some(slot) = output.arrival_slot
            {
                navigation.record_arrival_slot(original.unit_id, original.owner_slot, slot);
            }
            if let Some(slot) = accept_parking_arrival(
                original,
                constrained,
                output,
                traffic.parking_slot(original.unit_id),
            ) {
                navigation.record_arrival_slot(original.unit_id, original.owner_slot, slot);
                // Current positive parking authority owns this physical arrival.
                // Ordinary private waypoints still pass through the endpoint guard.
                continue;
            }
            restore_requested_target(
                frame,
                original,
                constrained,
                output,
                navigation.arrival_slot(original.unit_id, original.owner_slot),
                retry_progress_step(
                    original,
                    constrained,
                    direct_rows.get(index).copied().unwrap_or(false),
                    whole_frame_held,
                    dt,
                ),
            );
        }
        for output in &outputs {
            if output.target.is_none() {
                navigation.completed(output.unit_id);
            }
        }
        navigation.finish_tick();
        model.record(&mut traffic, tick, dt);
        drop(model);
        super::traffic::velocity::publish(&mut traffic, frame, &outputs, tick, dt);
        if !world.contains_resource::<crate::visibility::VisibilityState>() {
            super::traffic::public_passage::publish(&mut traffic, frame, &outputs, tick);
        }
        traffic.record_body_guard(&body_budget);
        navigation.record_traffic(&traffic);
        navigation.record_body_index(&body_budget);
        (outputs, waiting)
    };
    world.insert_resource(navigation);
    world.insert_resource(traffic);
    (outputs, waiting)
}

#[cfg(test)]
pub(crate) use shared::debug_field_status;

#[cfg(test)]
#[path = "navigation/endpoint_tests.rs"]
mod endpoint_tests;

#[cfg(test)]
#[path = "navigation_fallback_tests.rs"]
mod fallback_tests;
