//! The order phase of one fixed tick.

use crate::SlotId;
use crate::orders::apply::{clear_gather, unit_has_pending_move};
use crate::orders::behaviours::attack::run_unit_attack;
use crate::orders::behaviours::follow::run_unit_follow_move;
use crate::orders::behaviours::queue::start_next_queued_order;
use crate::orders::pending::{
    UnitMoveNeedsAllocation, dispatch_pending_movement, finish_movement_dispatch_tick,
    queue_unallocated_move_orders,
};
use crate::orders::plain::{PlainMoveOrderIdentities, forget_plain_move_identity};
use crate::orders::{OrderFailure, OrderTargetSim, UnitOrders};
use crate::world::MoveTarget;
use bevy_ecs::prelude::World;
use bevy_math::Vec2;
use std::collections::BTreeMap;
use turts_mod_api::Behaviour;

/// Run one fixed-tick order phase.
pub(crate) fn run_orders(world: &mut World) {
    dispatch_pending_movement(world);
    let mut unit_entities = BTreeMap::new();
    let mut engagement_points = Vec::new();
    let mut engagement_positions = BTreeMap::new();
    let mut engagement_owners = BTreeMap::new();
    for entity in world.iter_entities() {
        let Some(id) = entity.get::<crate::world::UnitId>() else {
            continue;
        };
        unit_entities.insert(id.0, entity.id());
        let (Some(position), Some(owner)) = (
            entity.get::<crate::world::Position>(),
            entity.get::<crate::world::Owner>(),
        ) else {
            continue;
        };
        engagement_points.push(crate::grid::GridPoint {
            id: id.0,
            position: position.0,
        });
        engagement_positions.insert(id.0, position.0);
        engagement_owners.insert(id.0, owner.0.0);
    }
    engagement_points.sort_unstable_by_key(|point| point.id);
    let engagement_cell_size = world
        .get_resource::<crate::bootstrap::InstalledCatalog>()
        .into_iter()
        .flat_map(|catalog| catalog.0.units.values())
        .filter_map(|unit| unit.weapon.as_ref().map(|weapon| weapon.range))
        .fold(f32::EPSILON, f32::max);
    let engagement_grid = crate::grid::UniformGrid::new(&engagement_points, engagement_cell_size);
    for (&unit_id, &entity) in &unit_entities {
        let replacement_pending = unit_has_pending_move(world, unit_id);
        let no_plain_move_state = world.entity(entity).get::<MoveTarget>().is_none()
            && world
                .entity(entity)
                .get::<crate::movement::PlainMoveProgress>()
                .is_none();
        let has_current_order = world
            .entity(entity)
            .get::<UnitOrders>()
            .is_some_and(|orders| orders.current.is_some());
        if no_plain_move_state && !has_current_order {
            forget_plain_move_identity(world, unit_id);
        }
        let plain_progress = world
            .entity(entity)
            .get::<crate::movement::PlainMoveProgress>()
            .cloned();
        let gather_stage = world
            .get_resource::<crate::economy::EconomyOrders>()
            .and_then(|orders| orders.0.get(&unit_id))
            .map(|order| order.stage);
        let gather_goal = match gather_stage {
            Some(crate::economy::gather::GatherStage::MovingToNode) => world
                .get_resource::<crate::economy::EconomyOrders>()
                .and_then(|orders| orders.0.get(&unit_id))
                .and_then(|order| order.deposit)
                .map(|node| crate::movement::PlainGatherGoal::Node(node.0)),
            Some(crate::economy::gather::GatherStage::MovingToReceiver) => world
                .get_resource::<crate::economy::EconomyOrders>()
                .and_then(|orders| orders.0.get(&unit_id))
                .and_then(|order| order.receiver)
                .map(crate::movement::PlainGatherGoal::Receiver),
            _ => None,
        };
        let gather_reached = matches!(
            gather_stage,
            Some(
                crate::economy::gather::GatherStage::Gathering
                    | crate::economy::gather::GatherStage::Unloading
            )
        );
        let gather_endpoint_changed = plain_progress
            .as_ref()
            .filter(|progress| progress.activity != crate::movement::MovementActivity::Blocked)
            .and_then(|progress| progress.gather_goal)
            .is_some_and(|previous| Some(previous) != gather_goal);
        if plain_progress.is_some() && (gather_reached || gather_endpoint_changed) {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
                forget_plain_move_identity(world, unit_id);
            });
            continue;
        }
        let completed_plain_fallback = plain_progress.as_ref().is_some_and(|progress| {
            progress.activity == crate::movement::MovementActivity::Resting
                && (progress.claim_count > 0 || progress.slot != progress.original_point)
        });
        let gather_still_approaching = matches!(
            gather_stage,
            Some(
                crate::economy::gather::GatherStage::MovingToNode
                    | crate::economy::gather::GatherStage::MovingToReceiver
            )
        );
        let current_is_unit_follow = world
            .entity(entity)
            .get::<UnitOrders>()
            .and_then(|orders| orders.current.as_ref())
            .is_some_and(|current| {
                matches!(current.behaviour, Behaviour::Move | Behaviour::Attack)
                    && matches!(current.target, OrderTargetSim::Unit(_))
            });
        if completed_plain_fallback && !gather_still_approaching && !current_is_unit_follow {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
                forget_plain_move_identity(world, unit_id);
            });
        }
        let plain_blocked = world
            .entity(entity)
            .get::<crate::movement::PlainMoveProgress>()
            .is_some_and(|progress| {
                (progress.activity == crate::movement::MovementActivity::Blocked
                    && !progress.failure_recorded)
                    || (completed_plain_fallback && gather_still_approaching)
            });
        if plain_blocked && gather_still_approaching {
            // M2 design 5: a gather order waits visibly if movement cannot reach range, and
            // the cycle never discards cargo. So a blocked trip of a gather cycle does not
            // fail and does not clear the cycle: it waits, then tries the approach again
            // (gather.rs, ruling O16 extension). The order keeps its own end rules.
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
                crate::economy::gather::wait_after_blocked_trip(world, unit_id);
            });
            continue;
        }
        if plain_blocked {
            let slot = world
                .entity(entity)
                .get::<crate::world::Owner>()
                .map_or(SlotId(0), |owner| owner.0);
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                let current = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .and_then(|orders| orders.current.clone());
                let identity = world
                    .get_resource::<PlainMoveOrderIdentities>()
                    .and_then(|identities| identities.0.get(&unit_id))
                    .cloned();
                let has_queued = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .is_some_and(|orders| !orders.queue.is_empty());
                let had_current = current.is_some();
                let failure_order = current
                    .as_ref()
                    .map(|current| current.order.clone())
                    .or_else(|| identity.as_ref().map(|order| order.order.clone()));
                if let Some(order) = failure_order {
                    let tick = world.resource::<crate::world::Tick>().0;
                    let mut entity_mut = world.entity_mut(entity);
                    if entity_mut.get::<UnitOrders>().is_none() {
                        entity_mut.insert(UnitOrders::default());
                    }
                    if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                        orders.last_failure = Some(OrderFailure {
                            order,
                            reason: crate::command::RejectReason::Blocked,
                            tick,
                        });
                        orders.current = None;
                    }
                }
                if let Some(mut progress) = world
                    .entity_mut(entity)
                    .get_mut::<crate::movement::PlainMoveProgress>()
                {
                    progress.activity = crate::movement::MovementActivity::Blocked;
                    progress.failure_recorded = true;
                }
                clear_gather(world, unit_id);
                forget_plain_move_identity(world, unit_id);
                if (has_queued || had_current) && !unit_has_pending_move(world, unit_id) {
                    start_next_queued_order(world, slot, entity, unit_id);
                }
            });
            continue;
        }
        let current = world
            .entity(entity)
            .get::<UnitOrders>()
            .and_then(|orders| orders.current.clone());
        let blocked = world
            .entity(entity)
            .get::<crate::movement::MoveProgress>()
            .is_some_and(|progress| {
                progress.activity == crate::movement::MovementActivity::Blocked
                    && world.entity(entity).get::<MoveTarget>().is_none()
            });
        if let Some(current) = current.as_ref().filter(|_| blocked) {
            let slot = world
                .entity(entity)
                .get::<crate::world::Owner>()
                .map_or(SlotId(0), |owner| owner.0);
            let tick = world.resource::<crate::world::Tick>().0;
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                {
                    let mut entity_mut = world.entity_mut(entity);
                    if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                        orders.last_failure = Some(OrderFailure {
                            order: current.order.clone(),
                            reason: crate::command::RejectReason::Blocked,
                            tick,
                        });
                        orders.current = None;
                    }
                }
                if !unit_has_pending_move(world, unit_id) {
                    start_next_queued_order(world, slot, entity, unit_id);
                }
            });
            continue;
        }
        if current.is_none() {
            let waiting = world
                .entity(entity)
                .get::<UnitOrders>()
                .is_some_and(|orders| !orders.queue.is_empty());
            let plain_move_landed = world
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .is_none();
            if waiting && plain_move_landed && !replacement_pending {
                let slot = world
                    .entity(entity)
                    .get::<crate::world::Owner>()
                    .map_or(SlotId(0), |owner| owner.0);
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    unit_id,
                    |world| {
                        start_next_queued_order(world, slot, entity, unit_id);
                    },
                );
            }
            continue;
        }
        let current = current.unwrap();
        if matches!(current.behaviour, Behaviour::Move | Behaviour::Attack)
            && let OrderTargetSim::Unit(target_id) = current.target
        {
            if world
                .get_resource::<UnitMoveNeedsAllocation>()
                .is_some_and(|pending| pending.0.contains(&unit_id))
            {
                // A queued Unit follow is a new envelope. It must pass through
                // the shared cap before ordinary follow completion can advance
                // the actor's post-move queue.
                continue;
            }
            if current.behaviour == Behaviour::Attack {
                run_unit_attack(world, entity, unit_id, current, target_id, &unit_entities);
            } else {
                run_unit_follow_move(world, entity, unit_id, current, target_id, &unit_entities);
            }
            continue;
        }
        if current.behaviour == Behaviour::Attack
            && let OrderTargetSim::Point(point) = current.target
            && world.entity(entity).get::<MoveTarget>().is_none()
        {
            let engage = world
                .resource::<crate::bootstrap::InstalledCatalog>()
                .0
                .orders
                .get(&current.order)
                .is_some_and(|definition| {
                    definition.engage == Some(turts_mod_api::EngagePolicy::StopAndFire)
                });
            let target_in_range = engage
                && attack_point_has_target_in_range(
                    world,
                    entity,
                    unit_id,
                    &engagement_grid,
                    &engagement_points,
                    &engagement_positions,
                    &engagement_owners,
                );
            if target_in_range {
                continue;
            }
            let resting_at_slot = world
                .entity(entity)
                .get::<crate::movement::MoveProgress>()
                .is_some_and(|progress| {
                    progress.activity == crate::movement::MovementActivity::Resting
                        && world
                            .entity(entity)
                            .get::<crate::world::Position>()
                            .is_some_and(|position| position.0 == progress.slot)
                })
                || (world
                    .entity(entity)
                    .get::<crate::movement::MoveProgress>()
                    .is_none()
                    && world
                        .entity(entity)
                        .get::<crate::world::Position>()
                        .is_some_and(|position| position.0 == point));
            if !resting_at_slot {
                let resume = world
                    .entity(entity)
                    .get::<crate::movement::MoveProgress>()
                    .map_or(point, |progress| progress.slot);
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    unit_id,
                    |world| {
                        world.entity_mut(entity).insert(MoveTarget(resume));
                    },
                );
                continue;
            }
        }
        let completed_move = matches!(current.behaviour, Behaviour::Move | Behaviour::Attack)
            && matches!(current.target, OrderTargetSim::Point(_))
            && world
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .is_none();
        let depleted_gather = current.behaviour == Behaviour::Gather
            && match current.target {
                OrderTargetSim::Node(node) => world
                    .get_resource::<crate::economy::SurfaceNodes>()
                    .is_some_and(|nodes| {
                        nodes
                            .0
                            .iter()
                            .any(|record| record.id == node && record.remaining == 0)
                    }),
                _ => false,
            }
            // Design 5.3 with M2 design 5: a depleted node starts the delivery of the cargo
            // that the gatherer holds, and the gather cycle never discards cargo. So the order
            // fails only when the cycle holds no cargo to deliver.
            && !crate::economy::gather::delivers_cargo(world, unit_id);
        if !completed_move && !depleted_gather {
            continue;
        }
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            let next = {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                if depleted_gather {
                    orders.last_failure = Some(OrderFailure {
                        order: current.order.clone(),
                        reason: crate::command::RejectReason::NodeDepleted,
                        tick,
                    });
                }
                orders.current = None;
                if replacement_pending {
                    None
                } else {
                    orders.queue.pop_front()
                }
            };
            if depleted_gather {
                let _ = crate::economy::gather::interrupt_order(world, slot, unit_id);
            }
            if next.is_some() {
                if let Some(next) = next {
                    world
                        .entity_mut(entity)
                        .get_mut::<UnitOrders>()
                        .unwrap()
                        .current = Some(next);
                }
                if !unit_has_pending_move(world, unit_id) {
                    start_next_queued_order(world, slot, entity, unit_id);
                }
            }
        });
    }
    queue_unallocated_move_orders(world);
    dispatch_pending_movement(world);
    finish_movement_dispatch_tick(world);
}

fn attack_point_has_target_in_range(
    world: &World,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    grid: &crate::grid::UniformGrid,
    points: &[crate::grid::GridPoint],
    positions: &BTreeMap<u32, Vec2>,
    owners: &BTreeMap<u32, u8>,
) -> bool {
    let Some(origin) = positions.get(&unit_id).copied() else {
        return false;
    };
    let Some(owner) = owners.get(&unit_id).copied() else {
        return false;
    };
    let Some(unit_type) = world.entity(entity).get::<crate::world::UnitType>() else {
        return false;
    };
    let Some(unit_definition) = world
        .resource::<crate::registry::UnitTypes>()
        .get(unit_type.0)
    else {
        return false;
    };
    let Ok(content_id) = turts_mod_api::ContentId::parse(&unit_definition.content_id) else {
        return false;
    };
    let Some(weapon) = world
        .resource::<crate::bootstrap::InstalledCatalog>()
        .0
        .units
        .get(&content_id)
        .and_then(|unit| unit.weapon.as_ref())
    else {
        return false;
    };
    let Some(healths) = world.get_resource::<crate::economy::combat::Healths>() else {
        return false;
    };
    grid.within(points, origin, weapon.range)
        .into_iter()
        .any(|index| {
            let target = points[index].id;
            let Some(target_owner) = owners.get(&target).copied() else {
                return false;
            };
            if !healths.0.get(&target).is_some_and(|health| *health > 0) {
                return false;
            }
            let relation_matches = match weapon.targets.slots {
                turts_mod_api::SlotRelation::Other => target_owner != owner,
                turts_mod_api::SlotRelation::Same => target_owner == owner,
                turts_mod_api::SlotRelation::Any => true,
            };
            relation_matches && positions.contains_key(&target)
        })
}
