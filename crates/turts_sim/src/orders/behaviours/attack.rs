//! The attack order of one unit.

use crate::SlotId;
use crate::orders::apply::{clamp_target, unit_has_pending_move};
use crate::orders::behaviours::queue::start_next_queued_order;
use crate::orders::writer::edit_unit_orders;
use crate::orders::{Order, OrderFailure};
use crate::world::MoveTarget;
use bevy_ecs::prelude::World;
use std::collections::BTreeMap;

pub(super) fn attack_goal(
    world: &World,
    entity: bevy_ecs::entity::Entity,
    target_position: bevy_math::Vec2,
) -> Option<bevy_math::Vec2> {
    let actor = world.entity(entity).get::<crate::world::Position>()?.0;
    let unit_type = world.entity(entity).get::<crate::world::UnitType>()?.0;
    let unit = world
        .resource::<crate::registry::UnitTypes>()
        .get(unit_type)?;
    let id = turts_mod_api::ContentId::parse(&unit.content_id).ok()?;
    let weapon = world
        .resource::<crate::bootstrap::InstalledCatalog>()
        .0
        .units
        .get(&id)?
        .weapon
        .as_ref()?;
    let delta = target_position - actor;
    (delta.length_squared() > weapon.range * weapon.range).then(|| {
        clamp_target(
            world,
            target_position - delta.normalize_or_zero() * weapon.range,
            unit.radius,
        )
    })
}

pub(in crate::orders) fn run_unit_attack(
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
            .map(|position| position.0)
    });
    let Some(target_position) = target_position else {
        let fired = world
            .get_resource::<crate::economy::combat::Weapons>()
            .and_then(|weapons| weapons.0.get(&unit_id))
            .is_some_and(|state| state.ordered_target_fired == Some(target_id));
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            edit_unit_orders(world, entity, |orders| {
                orders.current = None;
                if !fired {
                    orders.last_failure = Some(OrderFailure {
                        order: current.order.clone(),
                        reason: crate::command::RejectReason::TargetGone,
                        tick,
                    });
                }
            });
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<MoveTarget>();
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
        return;
    };
    let Some(goal) = attack_goal(world, entity, target_position) else {
        if world.entity(entity).get::<MoveTarget>().is_some() {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                let mut entity_mut = world.entity_mut(entity);
                entity_mut.remove::<MoveTarget>();
                entity_mut.remove::<crate::movement::MoveProgress>();
                entity_mut.remove::<crate::movement::PlainMoveProgress>();
            });
        }
        return;
    };
    let can_move = world
        .entity(entity)
        .get::<crate::world::UnitType>()
        .and_then(|kind| world.resource::<crate::registry::UnitTypes>().get(kind.0))
        .is_some_and(|unit| unit.movement.is_some());
    if !can_move {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            edit_unit_orders(world, entity, |orders| {
                orders.current = None;
                orders.last_failure = Some(OrderFailure {
                    order: current.order.clone(),
                    reason: crate::command::RejectReason::OutOfRange,
                    tick,
                });
            });
            if !unit_has_pending_move(world, unit_id) {
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
        return;
    }
    if world
        .entity(entity)
        .get::<MoveTarget>()
        .is_none_or(|target| target.0 != goal)
    {
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            let mut entity_mut = world.entity_mut(entity);
            entity_mut.remove::<crate::movement::MoveProgress>();
            entity_mut.remove::<crate::movement::PlainMoveProgress>();
            entity_mut.insert(MoveTarget(goal));
        });
    }
}
