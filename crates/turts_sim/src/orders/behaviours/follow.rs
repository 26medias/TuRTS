//! The follow order of one unit.

use crate::SlotId;
use crate::orders::apply::{clamp_target, unit_has_pending_move, unit_radius};
use crate::orders::behaviours::queue::start_next_queued_order;
use crate::orders::{Order, OrderFailure, UnitOrders};
use crate::world::MoveTarget;
use bevy_ecs::prelude::World;
use std::collections::BTreeMap;

pub(in crate::orders) fn run_unit_follow_move(
    world: &mut World,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    current: Order,
    target_id: u32,
    entity_by_unit: &BTreeMap<u32, bevy_ecs::entity::Entity>,
) {
    let target_position = entity_by_unit.get(&target_id).and_then(|target| {
        world
            .entity(*target)
            .get::<crate::world::Position>()
            .map(|pos| pos.0)
    });
    let current_position = world
        .entity(entity)
        .get::<crate::world::Position>()
        .map(|pos| pos.0);
    let Some(target_position) = target_position else {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                orders.last_failure = Some(OrderFailure {
                    order: current.order.clone(),
                    reason: crate::command::RejectReason::TargetGone,
                    tick,
                });
                orders.current = None;
            }
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            world
                .entity_mut(entity)
                .remove::<crate::movement::MoveProgress>();
            world
                .entity_mut(entity)
                .remove::<crate::movement::PlainMoveProgress>();
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
        return;
    };
    let Some(current_position) = current_position else {
        return;
    };
    let target_goal = clamp_target(world, target_position, unit_radius(world, entity));
    let cohort_progress = world
        .entity(entity)
        .get::<crate::movement::MoveProgress>()
        .cloned();
    let cohort_pool_goal = cohort_progress.as_ref().and_then(|progress| {
        world
            .get_resource::<crate::movement::MovementPools>()
            .and_then(|pools| pools.0.get(&progress.pool_id))
            .map(|pool| pool.point)
    });
    let same_goal_cohort = cohort_progress.as_ref().is_some_and(|progress| {
        progress
            .follow_origin
            .is_some_and(|origin| origin.target_unit_id == target_id && origin.goal == target_goal)
            || (progress.follow_origin.is_none() && cohort_pool_goal == Some(target_goal))
    });
    let cohort_fallback_reached = cohort_progress.as_ref().is_some_and(|progress| {
        same_goal_cohort
            && progress.activity == crate::movement::MovementActivity::Resting
            && progress.slot == current_position
            && world.entity(entity).get::<MoveTarget>().is_none()
            && world
                .entity(entity)
                .get::<crate::movement::PhysicalEscape>()
                .is_none()
    });
    let plain_progress = world
        .entity(entity)
        .get::<crate::movement::PlainMoveProgress>()
        .cloned();
    let same_goal_fallback = plain_progress
        .as_ref()
        .is_some_and(|progress| progress.original_point == target_goal);
    let fallback_reached = same_goal_fallback
        && plain_progress.as_ref().is_some_and(|progress| {
            progress.activity == crate::movement::MovementActivity::Resting
                && progress.slot == current_position
                && world.entity(entity).get::<MoveTarget>().is_none()
                && world
                    .entity(entity)
                    .get::<crate::movement::PhysicalEscape>()
                    .is_none()
        });
    let physical_escape_active = world
        .entity(entity)
        .get::<crate::movement::PhysicalEscape>()
        .is_some();
    if (current_position == target_goal && !physical_escape_active)
        || fallback_reached
        || cohort_fallback_reached
    {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                orders.current = None;
            }
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            if cohort_progress.is_some() {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::MoveProgress>();
            }
            if plain_progress.is_some() {
                world
                    .entity_mut(entity)
                    .remove::<crate::movement::PlainMoveProgress>();
            }
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
    } else if same_goal_fallback || same_goal_cohort {
        // A fallback for the unchanged legal goal is still active or resting;
        // refreshing the target actor must not reopen or erase that result.
    } else {
        let move_target = world
            .entity(entity)
            .get::<crate::world::MoveTarget>()
            .map(|target| target.0);
        if move_target != Some(target_goal) || plain_progress.is_some() {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                if cohort_progress.is_some() {
                    world
                        .entity_mut(entity)
                        .remove::<crate::movement::MoveProgress>();
                }
                let mut entity_mut = world.entity_mut(entity);
                entity_mut.remove::<crate::movement::MoveProgress>();
                entity_mut.remove::<crate::movement::PlainMoveProgress>();
                entity_mut.insert(crate::world::MoveTarget(target_goal));
            });
        }
    }
}
