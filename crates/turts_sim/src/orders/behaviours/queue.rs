//! The start of a queued order and of the order of a unit.

use crate::SlotId;
use crate::orders::apply::{clamp_target, unit_radius};
use crate::orders::behaviours::attack::attack_goal;
use crate::orders::pending::UnitMoveNeedsAllocation;
use crate::orders::plain::forget_plain_move_identity;
use crate::orders::writer::edit_unit_orders;
use crate::orders::{Order, OrderFailure, OrderTargetSim};
use bevy_ecs::prelude::World;
use turts_mod_api::Behaviour;

pub(in crate::orders) fn start_next_queued_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit: u32,
) {
    loop {
        let Some(next) = edit_unit_orders(world, entity, |orders| {
            let next = orders.current.take().or_else(|| orders.queue.pop_front())?;
            orders.current = Some(next.clone());
            Some(next)
        })
        .unwrap() else {
            return;
        };
        if let Err(reason) = start_order(world, slot, entity, unit, &next) {
            let tick = world.resource::<crate::world::Tick>().0;
            edit_unit_orders(world, entity, |orders| {
                orders.current = None;
                orders.last_failure = Some(OrderFailure {
                    order: next.order,
                    reason,
                    tick,
                });
            })
            .unwrap();
            continue;
        }
        break;
    }
}

pub(in crate::orders) fn start_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit: u32,
    order: &Order,
) -> Result<(), crate::command::RejectReason> {
    match (order.behaviour, &order.target) {
        (Behaviour::Move, OrderTargetSim::Point(point)) => {
            let target = clamp_target(world, *point, unit_radius(world, entity));
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            entity_mut.insert(crate::world::MoveTarget(target));
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Move, OrderTargetSim::Unit(target_id)) => {
            let target = world
                .iter_entities()
                .find(|candidate| {
                    candidate
                        .get::<crate::world::UnitId>()
                        .is_some_and(|id| id.0 == *target_id)
                })
                .and_then(|candidate| candidate.get::<crate::world::Position>().map(|pos| pos.0))
                .ok_or(crate::command::RejectReason::TargetGone)?;
            let target = clamp_target(world, target, unit_radius(world, entity));
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            entity_mut.insert(crate::world::MoveTarget(target));
            world.init_resource::<UnitMoveNeedsAllocation>();
            world
                .resource_mut::<UnitMoveNeedsAllocation>()
                .0
                .insert(unit);
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Attack, OrderTargetSim::Unit(target_id)) => {
            let target_position = world
                .iter_entities()
                .find(|candidate| {
                    candidate
                        .get::<crate::world::UnitId>()
                        .is_some_and(|id| id.0 == *target_id)
                })
                .and_then(|candidate| candidate.get::<crate::world::Position>().map(|pos| pos.0))
                .ok_or(crate::command::RejectReason::TargetGone)?;
            let target = attack_goal(world, entity, target_position);
            if let Some(mut weapons) = world.get_resource_mut::<crate::economy::combat::Weapons>()
                && let Some(state) = weapons.0.get_mut(&unit)
            {
                state.last_target = None;
                state.ordered_target_fired = None;
            }
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            if let Some(target) = target {
                entity_mut.insert(crate::world::MoveTarget(target));
            } else {
                entity_mut.remove::<crate::world::MoveTarget>();
            }
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Attack, OrderTargetSim::Point(point)) => {
            let target = clamp_target(world, *point, unit_radius(world, entity));
            if let Some(mut weapons) = world.get_resource_mut::<crate::economy::combat::Weapons>()
                && let Some(state) = weapons.0.get_mut(&unit)
            {
                state.last_target = None;
                state.ordered_target_fired = None;
            }
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            entity_mut.insert(crate::world::MoveTarget(target));
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Hold, _) => {
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            world
                .entity_mut(entity)
                .remove::<crate::movement::MoveProgress>();
            world
                .entity_mut(entity)
                .remove::<crate::movement::PlainMoveProgress>();
            forget_plain_move_identity(world, unit);
            Ok(())
        }
        (Behaviour::Gather, OrderTargetSim::Node(node)) => {
            let state = world
                .get_resource::<crate::economy::SurfaceNodes>()
                .and_then(|nodes| nodes.0.iter().find(|record| record.id == *node));
            if state.is_none() {
                return Err(crate::command::RejectReason::TargetGone);
            }
            if state.is_some_and(|record| record.remaining == 0) {
                return Err(crate::command::RejectReason::NodeDepleted);
            }
            crate::economy::gather::assign_gather(world, slot, unit, *node)
        }
        _ => Err(crate::command::RejectReason::NotApplicable),
    }
}
