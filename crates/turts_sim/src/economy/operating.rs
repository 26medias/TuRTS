//! Shared activation preparation and diagnostics for automatic operating work.

use super::{income, power, transaction};
use crate::{Owner, RejectReason, Tick, bootstrap::IncomePeriods, unit_index::unit_entity};
use bevy_ecs::{prelude::Resource, world::World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use turts_mod_api::ResolvedUnit;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatingHold {
    pub unit: u32,
    pub owner_slot: u8,
    pub reason: RejectReason,
    pub since_tick: u32,
}

#[derive(Resource, Default)]
pub(crate) struct OperatingHolds(pub(crate) BTreeMap<u32, OperatingHold>);

pub(crate) fn prepare_activation(
    world: &World,
    definition: &ResolvedUnit,
    owner_slot: u8,
) -> Result<(power::Activation, Option<income::IncomeActivation>), RejectReason> {
    let activation = power::can_activate(world, definition, owner_slot)?;
    let clock = definition
        .income
        .as_ref()
        .map(|definition_income| {
            let period = world
                .get_resource::<IncomePeriods>()
                .and_then(|periods| periods.0.get(&definition.id))
                .copied()
                .ok_or(RejectReason::IntegrityViolation)?;
            income::prepare_activation(
                owner_slot,
                definition_income,
                period,
                world.resource::<Tick>().0,
            )
        })
        .transpose()?;
    Ok((activation, clock))
}

pub(crate) fn hold(world: &mut World, unit: u32, reason: RejectReason) {
    if world
        .get_resource::<OperatingHolds>()
        .and_then(|holds| holds.0.get(&unit))
        .is_some_and(|old| old.reason == reason)
    {
        return;
    }
    let Some(owner_slot) = unit_entity(world, unit)
        .and_then(|entity| world.get::<Owner>(entity))
        .map(|owner| owner.0.0)
    else {
        return;
    };
    let since_tick = world.resource::<Tick>().0;
    transaction::capture_operation(world, |world| {
        world.init_resource::<OperatingHolds>();
        world.resource_mut::<OperatingHolds>().0.insert(
            unit,
            OperatingHold {
                unit,
                owner_slot,
                reason,
                since_tick,
            },
        );
    });
}

pub(crate) fn clear_hold(world: &mut World, unit: u32) {
    if !world
        .get_resource::<OperatingHolds>()
        .is_some_and(|holds| holds.0.contains_key(&unit))
    {
        return;
    }
    transaction::capture_operation(world, |world| {
        world.resource_mut::<OperatingHolds>().0.remove(&unit);
    });
}

pub(crate) fn holds(world: &World) -> Vec<OperatingHold> {
    world
        .get_resource::<OperatingHolds>()
        .map_or_else(Vec::new, |holds| holds.0.values().cloned().collect())
}
