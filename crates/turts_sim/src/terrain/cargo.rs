//! Explicit timed physical unloading. Terrain cargo is stored, never implicitly converted.
use crate::{
    Owner, Position, RejectReason, UnitType,
    bootstrap::InstalledCatalog,
    economy::{ledger::EconomyLedger, transaction},
    orders::{OrderTargetSim, UnitOrders},
    unit_index::unit_entity,
};
use bevy_ecs::{prelude::Resource, world::World};
use std::collections::BTreeMap;
use turts_mod_api::{Behaviour, ContentId, ResolvedDropOffEffect, TerrainCargoDef};
#[derive(Resource, Default)]
pub(crate) struct CargoRuntime {
    carries: BTreeMap<u32, u64>,
}
pub(super) fn audit_carries(world: &World) -> Vec<(u32, u64)> {
    world
        .get_resource::<CargoRuntime>()
        .into_iter()
        .flat_map(|runtime| runtime.carries.iter().map(|(&unit, &carry)| (unit, carry)))
        .collect()
}
fn definition(world: &World, unit: u32) -> Option<&turts_mod_api::ResolvedUnit> {
    let kind = world.get::<UnitType>(unit_entity(world, unit)?)?;
    let id = ContentId::parse(
        &world
            .get_resource::<crate::UnitTypes>()?
            .get(kind.0)?
            .content_id,
    )
    .ok()?;
    world.get_resource::<InstalledCatalog>()?.0.units.get(&id)
}
pub(crate) fn validate_order(
    world: &World,
    actor: u32,
    target: &OrderTargetSim,
) -> Result<(), RejectReason> {
    let OrderTargetSim::Unit(receiver) = target else {
        return Err(RejectReason::TargetInvalid);
    };
    if actor == *receiver {
        return Err(RejectReason::TargetInvalid);
    }
    let source = unit_entity(world, actor).ok_or(RejectReason::UnknownUnit)?;
    let destination = unit_entity(world, *receiver).ok_or(RejectReason::TargetGone)?;
    if world.get::<Owner>(source).map(|owner| owner.0)
        != world.get::<Owner>(destination).map(|owner| owner.0)
    {
        return Err(RejectReason::NotOwner);
    }
    if let Some(healths) = world.get_resource::<crate::economy::combat::Healths>() {
        if healths.0.get(&actor).is_some_and(|health| *health == 0)
            || healths.0.get(receiver).is_some_and(|health| *health == 0)
        {
            return Err(RejectReason::TargetGone);
        }
    }
    if !definition(world, actor).is_some_and(|unit| unit.terrain_cargo.is_some()) {
        return Err(RejectReason::MissingCapability);
    }
    if !definition(world, *receiver)
        .and_then(|unit| unit.drop_off.as_ref())
        .is_some_and(|drop_off| {
            drop_off
                .rules
                .iter()
                .any(|rule| matches!(rule.effect, ResolvedDropOffEffect::Store))
        })
    {
        return Err(RejectReason::TargetInvalid);
    }
    if world
        .get_resource::<crate::economy::construction::ConstructionState>()
        .is_some_and(|state| state.sites.contains_key(receiver))
    {
        return Err(RejectReason::TargetInvalid);
    }
    Ok(())
}
pub(crate) fn order_changed(world: &mut World, unit: u32) {
    if let Some(mut runtime) = world.get_resource_mut::<CargoRuntime>() {
        runtime.carries.remove(&unit);
    }
}
fn one(world: &mut World, actor: u32, receiver: u32, cargo: TerrainCargoDef) {
    let Some(source) = unit_entity(world, actor) else {
        return;
    };
    let Some(destination) = unit_entity(world, receiver) else {
        return;
    };
    if crate::visibility::validate_target(
        world,
        world.get::<Owner>(source).map_or(0, |owner| owner.0.0),
        &OrderTargetSim::Unit(receiver),
    )
    .is_err()
    {
        return;
    }
    if validate_order(world, actor, &OrderTargetSim::Unit(receiver)).is_err() {
        return;
    }
    if world
        .get_resource::<crate::economy::research::Disabled>()
        .is_some_and(|disabled| disabled.0.contains(&actor))
    {
        return;
    }
    let Some(owner) = world.get::<Owner>(source).map(|owner| owner.0) else {
        return;
    };
    let (Some(position), Some(target)) = (
        world.get::<Position>(source).map(|position| position.0),
        world
            .get::<Position>(destination)
            .map(|position| position.0),
    ) else {
        return;
    };
    let radius = |entity| {
        world
            .get::<UnitType>(entity)
            .and_then(|kind| world.get_resource::<crate::UnitTypes>()?.get(kind.0))
            .map_or(0.0, |unit| unit.radius)
    };
    let bodies = radius(source) + radius(destination);
    let reach = f64::from(bodies) + f64::from(cargo.interaction_range);
    let dx = f64::from(position.x) - f64::from(target.x);
    let dy = f64::from(position.y) - f64::from(target.y);
    if dx * dx + dy * dy > reach * reach {
        let direction = (position - target).normalize_or_zero();
        world
            .entity_mut(source)
            .insert(crate::world::MoveTarget(target + direction * bodies));
        return;
    }
    world
        .entity_mut(source)
        .remove::<crate::world::MoveTarget>();
    let stock = world
        .get_resource::<EconomyLedger>()
        .and_then(|ledger| ledger.inventories.get(&actor))
        .map(|inventory| {
            inventory
                .items
                .iter()
                .filter_map(|(item, account)| {
                    account
                        .on_hand
                        .checked_sub(account.reserved)
                        .filter(|amount| *amount > 0)
                        .map(|amount| (item.clone(), amount))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if stock.is_empty() {
        order_changed(world, actor);
        crate::orders::finish_terrain_order(world, actor);
        return;
    }
    let hz = u64::from(world.resource::<crate::SimClock>().tick_hz);
    world.init_resource::<CargoRuntime>();
    let carry = world
        .resource::<CargoRuntime>()
        .carries
        .get(&actor)
        .copied()
        .unwrap_or(0);
    let earned = u128::from(cargo.unload_per_second) + u128::from(carry);
    let Ok(mut budget) = u64::try_from(earned / u128::from(hz)) else {
        return;
    };
    let next_carry = (earned % u128::from(hz)) as u64;
    if budget == 0 {
        world
            .resource_mut::<CargoRuntime>()
            .carries
            .insert(actor, next_carry);
        return;
    }
    let mut transferred = false;
    for (item, available) in stock {
        if budget == 0 {
            break;
        }
        let rule = world
            .get_resource::<InstalledCatalog>()
            .and_then(|catalog| {
                crate::economy::gather::receiver_rule(world, receiver, owner, &item, &catalog.0)
            });
        let Some((ResolvedDropOffEffect::Store, headroom)) = rule else {
            continue;
        };
        let amount = budget.min(available).min(headroom);
        if amount == 0 {
            continue;
        }
        let result = transaction::transact(world, |tx| {
            crate::economy::gather::stage_store(tx, owner, actor, receiver, &item, amount)?;
            tx.stage_event(
                owner.0,
                crate::EconomyEventKind::Stored {
                    from: actor,
                    to: receiver,
                    item: item.clone(),
                    amount,
                },
            )?;
            Ok(())
        });
        if result.is_err() {
            return;
        }
        transferred = true;
        budget -= amount;
    }
    if transferred {
        world
            .resource_mut::<CargoRuntime>()
            .carries
            .insert(actor, next_carry);
    }
}
pub(crate) fn run(world: &mut World) {
    let mut actors = world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<crate::UnitId>()?.0;
            let order = entity.get::<UnitOrders>()?.current.as_ref()?;
            if order.behaviour != Behaviour::UnloadCargo {
                return None;
            }
            let OrderTargetSim::Unit(receiver) = order.target else {
                return None;
            };
            let cargo = definition(world, id)?.terrain_cargo?;
            let position = entity.get::<Position>()?.0;
            let target = world.get::<Position>(unit_entity(world, receiver)?)?.0;
            let content = definition(world, id)?.id.clone();
            let zero = |value: f32| if value == 0.0 { 0.0 } else { value };
            Some((
                id,
                receiver,
                cargo,
                f64::from(position.distance_squared(target)),
                zero(position.x),
                zero(position.y),
                content,
            ))
        })
        .collect::<Vec<_>>();
    let rank = |left: &(u32, u32, TerrainCargoDef, f64, f32, f32, ContentId),
                right: &(u32, u32, TerrainCargoDef, f64, f32, f32, ContentId)| {
        left.3
            .total_cmp(&right.3)
            .then_with(|| left.4.total_cmp(&right.4))
            .then_with(|| left.5.total_cmp(&right.5))
            .then_with(|| left.6.cmp(&right.6))
    };
    actors.sort_by(rank);
    for index in 0..actors.len() {
        let actor = &actors[index];
        if index
            .checked_sub(1)
            .is_some_and(|prior| rank(actor, &actors[prior]).is_eq())
            || actors
                .get(index + 1)
                .is_some_and(|next| rank(actor, next).is_eq())
        {
            continue;
        }
        transaction::capture_unit_order_operation(world, actor.0, |world| {
            one(world, actor.0, actor.1, actor.2)
        });
    }
}
