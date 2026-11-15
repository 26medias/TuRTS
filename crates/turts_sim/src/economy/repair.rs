//! Repair state and exact paid healing, after work payment and before combat.

use super::ledger::{Endpoint, ReservationLine};
use super::{combat::Healths, construction::ConstructionState, transaction};
use crate::bootstrap::InstalledCatalog;
use crate::orders::{OrderTargetSim, UnitOrders};
use crate::registry::UnitTypes;
use crate::unit_index::unit_entity;
use crate::world::MoveTarget;
use crate::{Owner, Position, RejectReason, SimClock, Tick, UnitId, UnitType};
use bevy_ecs::{prelude::Resource, world::World};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroU64};
use turts_mod_api::ContentId;
use turts_mod_api::{Behaviour, RepairerDef, ResolvedUnit};

/// The owner's current repair action. Detailed refusal causes remain audit-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairStatus {
    Approaching,
    Working,
    WaitingForFunds(ContentId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRow {
    pub repairer: u32,
    pub target: u32,
    pub owner_slot: u8,
    pub state: RepairStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairAction {
    pub target: u32,
    pub automatic: bool,
    pub elapsed: u64,
    pub state: RepairStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairHold {
    pub reason: RejectReason,
    pub since_tick: u32,
}

/// One source row is staged with its balance and target-health writes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RepairUnitState {
    pub(crate) action: Option<RepairAction>,
    pub(crate) remainders: BTreeMap<ContentId, u64>,
    pub(crate) hold: Option<RepairHold>,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RepairRuntime {
    pub(crate) units: BTreeMap<u32, RepairUnitState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRemainder {
    pub resource: ContentId,
    pub remainder: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairAuditRow {
    pub repairer: u32,
    pub action: Option<RepairAction>,
    pub remainders: Vec<RepairRemainder>,
    pub hold: Option<RepairHold>,
}

/// Status from order approach. The common order runner owns completion and queue advancement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepairOrderProgress {
    Active,
    Complete,
    Failed(RejectReason),
}

fn definition(world: &World, unit: u32) -> Option<&ResolvedUnit> {
    let entity = unit_entity(world, unit)?;
    let kind = world.get::<UnitType>(entity)?;
    let id = &world.get_resource::<UnitTypes>()?.get(kind.0)?.content_id;
    world
        .get_resource::<InstalledCatalog>()?
        .0
        .units
        .get(&ContentId::parse(id).ok()?)
}

fn radius(world: &World, unit: u32) -> Option<f32> {
    let entity = unit_entity(world, unit)?;
    Some(
        world
            .get_resource::<UnitTypes>()?
            .get(world.get::<UnitType>(entity)?.0)?
            .radius,
    )
}

fn completed(world: &World, unit: u32) -> bool {
    !world
        .get_resource::<ConstructionState>()
        .is_some_and(|sites| sites.sites.contains_key(&unit))
}

fn alive(world: &World, unit: u32) -> bool {
    unit_entity(world, unit).is_some()
        && !world
            .get_resource::<Healths>()
            .and_then(|healths| healths.0.get(&unit))
            .is_some_and(|health| *health == 0)
}

fn eligible_target(
    world: &World,
    source: u32,
    target: u32,
    repairer: &RepairerDef,
) -> Result<(u64, u64), RejectReason> {
    let source_entity = unit_entity(world, source).ok_or(RejectReason::UnknownUnit)?;
    let target_entity = unit_entity(world, target).ok_or(RejectReason::TargetGone)?;
    if !completed(world, target) {
        return Err(RejectReason::TargetInvalid);
    }
    let own = world
        .get::<Owner>(source_entity)
        .ok_or(RejectReason::IntegrityViolation)?
        .0
        .0;
    let other = world
        .get::<Owner>(target_entity)
        .ok_or(RejectReason::IntegrityViolation)?
        .0
        .0;
    if !crate::rules_adapter::holds_own_team(repairer.targets.slots, own, other) {
        return Err(RejectReason::TargetInvalid);
    }
    let target_definition = definition(world, target).ok_or(RejectReason::TargetInvalid)?;
    if !repairer.classes.is_empty()
        && !target_definition
            .armour
            .as_ref()
            .is_some_and(|armour| repairer.classes.contains(&armour.class))
    {
        return Err(RejectReason::TargetInvalid);
    }
    let max = target_definition
        .health
        .as_ref()
        .ok_or(RejectReason::TargetInvalid)?
        .max;
    let health = world
        .get_resource::<Healths>()
        .and_then(|healths| healths.0.get(&target))
        .copied()
        .ok_or(RejectReason::TargetInvalid)?;
    if health == 0 {
        return Err(RejectReason::TargetInvalid);
    }
    Ok((health, max))
}

pub(crate) fn validate_order(world: &World, source: u32, target: u32) -> Result<(), RejectReason> {
    if !alive(world, source) || !completed(world, source) {
        return Err(RejectReason::UnknownUnit);
    }
    let repairer = definition(world, source)
        .and_then(|definition| definition.repairer.as_ref())
        .ok_or(RejectReason::MissingCapability)?;
    let (health, max) = eligible_target(world, source, target, repairer)?;
    if health >= max {
        return Err(RejectReason::TargetInvalid);
    }
    Ok(())
}

fn source_state(world: &World, source: u32) -> RepairUnitState {
    world
        .get_resource::<RepairRuntime>()
        .and_then(|runtime| runtime.units.get(&source))
        .cloned()
        .unwrap_or_default()
}

fn save_state(world: &mut World, source: u32, state: RepairUnitState) -> Result<(), RejectReason> {
    if source_state(world, source) == state {
        return Ok(());
    }
    transaction::transact(world, |tx| {
        tx.stage_repair_state(source, state);
        Ok(())
    })
}

pub(crate) fn order_changed(world: &mut World, source: u32) {
    let mut state = source_state(world, source);
    state.action = None;
    state.hold = None;
    let _ = save_state(world, source, state);
}

pub(crate) fn remove_unit(world: &mut World, unit: u32) {
    if let Some(mut runtime) = world.get_resource_mut::<RepairRuntime>() {
        runtime.units.remove(&unit);
        for state in runtime.units.values_mut() {
            if state
                .action
                .as_ref()
                .is_some_and(|action| action.target == unit)
            {
                state.action = None;
                state.hold = None;
            }
        }
    }
}

pub(crate) fn active_action(world: &World, source: u32) -> bool {
    world
        .get_resource::<RepairRuntime>()
        .and_then(|runtime| runtime.units.get(&source))
        .is_some_and(|state| state.action.is_some())
}

pub(crate) fn status_rows(world: &World) -> Vec<RepairRow> {
    world
        .get_resource::<RepairRuntime>()
        .into_iter()
        .flat_map(|runtime| runtime.units.iter())
        .filter_map(|(&source, state)| {
            let action = state.action.as_ref()?;
            let entity = unit_entity(world, source)?;
            let repairer = definition(world, source)?.repairer.as_ref()?;
            if !alive(world, source)
                || !eligible_target(world, source, action.target, repairer)
                    .is_ok_and(|(health, max)| health < max)
            {
                return None;
            }
            Some(RepairRow {
                repairer: source,
                target: action.target,
                owner_slot: world.get::<Owner>(entity)?.0.0,
                state: action.state.clone(),
            })
        })
        .collect()
}

pub(crate) fn audit_rows(world: &World) -> Vec<RepairAuditRow> {
    world
        .get_resource::<RepairRuntime>()
        .into_iter()
        .flat_map(|runtime| runtime.units.iter())
        .map(|(&source, state)| RepairAuditRow {
            repairer: source,
            action: state.action.clone(),
            remainders: state
                .remainders
                .iter()
                .map(|(resource, &remainder)| RepairRemainder {
                    resource: resource.clone(),
                    remainder,
                })
                .collect(),
            hold: state.hold.clone(),
        })
        .collect()
}

fn positions(world: &World, source: u32, target: u32) -> Option<(Vec2, Vec2, f32)> {
    let from = world.get::<Position>(unit_entity(world, source)?)?.0;
    let to = world.get::<Position>(unit_entity(world, target)?)?.0;
    Some((from, to, radius(world, source)? + radius(world, target)?))
}

fn in_reach(world: &World, source: u32, target: u32, reach: f32) -> bool {
    positions(world, source, target).is_some_and(|(from, to, radii)| {
        super::combat::within_weapon_range(from, to, reach + radii)
    })
}

fn clear_motion(world: &mut World, source: u32) {
    let Some(entity) = unit_entity(world, source) else {
        return;
    };
    if world.get::<MoveTarget>(entity).is_none() {
        return;
    }
    transaction::capture_unit_order_operation(world, source, |world| {
        let mut entity = world.entity_mut(entity);
        entity.remove::<MoveTarget>();
        entity.remove::<crate::movement::MoveProgress>();
        entity.remove::<crate::movement::PlainMoveProgress>();
    });
}

fn approach(
    world: &mut World,
    source: u32,
    target: u32,
    reach: f32,
) -> Result<RepairStatus, RejectReason> {
    let (from, to, radii) = positions(world, source, target).ok_or(RejectReason::TargetGone)?;
    if super::combat::within_weapon_range(from, to, reach + radii) {
        clear_motion(world, source);
        return Ok(RepairStatus::Working);
    }
    let entity = unit_entity(world, source).ok_or(RejectReason::UnknownUnit)?;
    if definition(world, source).is_none_or(|definition| definition.movement.is_none()) {
        return Err(RejectReason::OutOfRange);
    }
    let mut goal = super::combat::weapon_stand_off(to, from, reach + radii);
    if let (Some(bounds), Some(radius)) = (
        world.get_resource::<crate::world::MapBounds>(),
        radius(world, source),
    ) {
        goal = goal.clamp(
            Vec2::splat(radius),
            (bounds.size - Vec2::splat(radius)).max(Vec2::splat(radius)),
        );
    }
    if world
        .get::<MoveTarget>(entity)
        .is_none_or(|old| old.0 != goal)
    {
        transaction::capture_unit_order_operation(world, source, |world| {
            let mut entity = world.entity_mut(entity);
            entity.remove::<crate::movement::MoveProgress>();
            entity.remove::<crate::movement::PlainMoveProgress>();
            entity.insert(MoveTarget(goal));
        });
    }
    Ok(RepairStatus::Approaching)
}

fn begin_action(
    world: &mut World,
    source: u32,
    target: u32,
    automatic: bool,
    status: RepairStatus,
) -> Result<(), RejectReason> {
    let mut state = source_state(world, source);
    let elapsed = state
        .action
        .as_ref()
        .filter(|action| action.target == target && action.automatic == automatic)
        .map_or(0, |action| action.elapsed);
    state.action = Some(RepairAction {
        target,
        automatic,
        elapsed,
        state: status,
    });
    save_state(world, source, state)
}

pub(crate) fn run_order_approach(world: &mut World, source: u32) -> RepairOrderProgress {
    let Some(entity) = unit_entity(world, source) else {
        return RepairOrderProgress::Failed(RejectReason::UnknownUnit);
    };
    let target = world
        .get::<UnitOrders>(entity)
        .and_then(|orders| orders.current.as_ref())
        .filter(|order| order.behaviour == Behaviour::Repair)
        .and_then(|order| match order.target {
            OrderTargetSim::Unit(target) => Some(target),
            _ => None,
        });
    let Some(target) = target else {
        order_changed(world, source);
        return RepairOrderProgress::Failed(RejectReason::TargetInvalid);
    };
    let Some(repairer) = definition(world, source)
        .and_then(|definition| definition.repairer.as_ref())
        .cloned()
    else {
        return RepairOrderProgress::Failed(RejectReason::MissingCapability);
    };
    let (health, max) = match eligible_target(world, source, target, &repairer) {
        Ok(values) => values,
        Err(reason) => {
            order_changed(world, source);
            clear_motion(world, source);
            return RepairOrderProgress::Failed(reason);
        }
    };
    if health >= max {
        order_changed(world, source);
        clear_motion(world, source);
        return RepairOrderProgress::Complete;
    }
    match approach(world, source, target, repairer.reach)
        .and_then(|status| begin_action(world, source, target, false, status))
    {
        Ok(()) => RepairOrderProgress::Active,
        Err(reason) => {
            order_changed(world, source);
            RepairOrderProgress::Failed(reason)
        }
    }
}

fn hold(world: &mut World, source: u32, reason: RejectReason) {
    let tick = world.get_resource::<Tick>().map_or(0, |tick| tick.0);
    let mut state = source_state(world, source);
    if state.hold.as_ref().is_none_or(|hold| hold.reason != reason) {
        state.hold = Some(RepairHold {
            reason,
            since_tick: tick,
        });
        let _ = save_state(world, source, state);
    }
}

fn heal_source(world: &mut World, source: u32, repairer: &RepairerDef) {
    let Some(action) = source_state(world, source).action else {
        return;
    };
    if !in_reach(world, source, action.target, repairer.reach) {
        return;
    }
    let (health, max) = match eligible_target(world, source, action.target, repairer) {
        Ok(values) => values,
        Err(_) => {
            order_changed(world, source);
            return;
        }
    };
    if health >= max {
        order_changed(world, source);
        return;
    }
    let Some(hz) = world
        .get_resource::<SimClock>()
        .and_then(|clock| NonZeroU32::new(u32::from(clock.tick_hz)))
    else {
        hold(world, source, RejectReason::IntegrityViolation);
        return;
    };
    let Some(next_elapsed) = action.elapsed.checked_add(1) else {
        hold(world, source, RejectReason::ArithmeticOverflow);
        return;
    };
    let applied = turts_rules::heal::applied_heal(
        turts_rules::heal::heal_difference(repairer.rate_per_second, next_elapsed, hz),
        max,
        health,
    );
    let owner = unit_entity(world, source)
        .and_then(|entity| world.get::<Owner>(entity))
        .map(|owner| owner.0.0);
    let Some(owner) = owner else {
        return;
    };
    let mut state = source_state(world, source);
    let mut debits = Vec::with_capacity(repairer.cost.len());
    for cost in &repairer.cost {
        let (Ok(resource), Some(per)) = (
            ContentId::parse(&cost.resource),
            NonZeroU64::new(cost.per_health),
        ) else {
            hold(world, source, RejectReason::IntegrityViolation);
            return;
        };
        let remainder = state.remainders.get(&resource).copied().unwrap_or(0);
        let Some((amount, next)) =
            turts_rules::heal::repair_charge_with_remainder(applied, cost.amount, per, remainder)
        else {
            hold(world, source, RejectReason::ArithmeticOverflow);
            return;
        };
        state.remainders.insert(resource.clone(), next);
        if amount != 0 {
            debits.push(ReservationLine {
                endpoint: Endpoint::Slot(owner),
                resource,
                amount,
            });
        }
    }
    let mut missing = None;
    let result = transaction::transact(world, |tx| {
        tx.mark_m3();
        for debit in &debits {
            tx.validate_endpoint_legacy(debit.endpoint, &debit.resource)?;
            let account = tx.account(debit.endpoint, &debit.resource);
            let available = account
                .on_hand
                .checked_sub(account.reserved)
                .ok_or(RejectReason::ArithmeticOverflow)?;
            if available < debit.amount {
                missing = Some(debit.resource.clone());
                return Err(RejectReason::InsufficientBalance);
            }
        }
        transaction::charge_costs(tx, &debits)?;
        if applied != 0 {
            tx.stage_health(
                action.target,
                health
                    .checked_add(applied)
                    .ok_or(RejectReason::ArithmeticOverflow)?,
            );
        }
        if let Some(action) = state.action.as_mut() {
            action.elapsed = next_elapsed;
            action.state = RepairStatus::Working;
        }
        state.hold = None;
        tx.stage_repair_state(source, state);
        Ok(())
    });
    if let Err(reason) = result {
        if let Some(resource) = missing {
            let mut state = source_state(world, source);
            if let Some(action) = state.action.as_mut() {
                action.state = RepairStatus::WaitingForFunds(resource);
            }
            state.hold = None;
            let _ = save_state(world, source, state);
        } else {
            hold(world, source, reason);
        }
    }
}

fn suppress_acquired(world: &mut World, source: u32) {
    let Some(entity) = unit_entity(world, source) else {
        return;
    };
    if world
        .get::<UnitOrders>(entity)
        .is_some_and(|orders| orders.acquired.is_some())
    {
        transaction::capture_post_movement_order_operation(world, source, |world| {
            crate::orders::writer::edit_unit_orders(world, entity, |orders| orders.acquired = None);
        });
    }
}

pub(crate) fn run_repair(world: &mut World) {
    if !world
        .get_resource::<InstalledCatalog>()
        .is_some_and(|catalog| catalog.0.units.values().any(|unit| unit.repairer.is_some()))
    {
        return;
    }
    let actors = world
        .iter_entities()
        .filter_map(|entity| Some((entity.get::<UnitId>()?.0, entity.get::<Position>()?.0)))
        .collect::<Vec<_>>();
    let points = actors
        .iter()
        .map(|&(id, position)| crate::grid::GridPoint { id, position })
        .collect::<Vec<_>>();
    let max_radius = actors
        .iter()
        .filter_map(|(id, _)| radius(world, *id))
        .fold(0_f32, f32::max);
    let cell_size = world
        .resource::<InstalledCatalog>()
        .0
        .units
        .values()
        .filter_map(|unit| {
            unit.repairer
                .as_ref()
                .and_then(|repairer| repairer.auto_range)
        })
        .fold(1_f32, f32::max);
    let grid = crate::grid::UniformGrid::new(&points, cell_size);
    let mut sources = actors
        .iter()
        .filter_map(|&(id, position)| {
            let repairer = definition(world, id)?.repairer.clone()?;
            (alive(world, id) && completed(world, id)).then_some((id, position, repairer))
        })
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| {
        left.1
            .x
            .total_cmp(&right.1.x)
            .then_with(|| left.1.y.total_cmp(&right.1.y))
    });
    let mut candidates = Vec::new();
    for (source, position, repairer) in sources {
        let Some(entity) = unit_entity(world, source) else {
            continue;
        };
        if world
            .get_resource::<super::research::Disabled>()
            .is_some_and(|disabled| disabled.0.contains(&source))
        {
            continue;
        }
        let orders = world.get::<UnitOrders>(entity);
        if orders
            .and_then(|orders| orders.current.as_ref())
            .is_some_and(|order| order.behaviour == Behaviour::Repair)
        {
            if run_order_approach(world, source) == RepairOrderProgress::Active {
                suppress_acquired(world, source);
                candidates.push((source, position, repairer));
            }
            continue;
        }
        let explicit = orders.is_some_and(|orders| {
            orders.repair_stopped || orders.current.is_some() || !orders.queue.is_empty()
        }) || world
            .get_resource::<crate::orders::PendingMovementOrders>()
            .is_some_and(|pending| pending.pending_member_counts.contains_key(&source))
            || world
                .get_resource::<super::EconomyOrders>()
                .and_then(|orders| orders.0.get(&source))
                .is_some_and(|order| {
                    order.node.is_some() || order.deposit.is_some() || order.receiver.is_some()
                })
            || world
                .get_resource::<crate::orders::PlainMoveOrderIdentities>()
                .is_some_and(|orders| orders.0.contains_key(&source));
        if explicit {
            if active_action(world, source) {
                order_changed(world, source);
            }
            continue;
        }
        let old = source_state(world, source).action;
        let target = if let Some(action) = old.as_ref() {
            if !action.automatic
                || !eligible_target(world, source, action.target, &repairer)
                    .is_ok_and(|(health, max)| health < max)
                || !repairer
                    .auto_range
                    .is_some_and(|range| in_reach(world, source, action.target, range))
            {
                order_changed(world, source);
                clear_motion(world, source);
                continue;
            }
            Some(action.target)
        } else {
            let Some(range) = repairer.auto_range else {
                continue;
            };
            let source_radius = radius(world, source).unwrap_or(0.0);
            let mut best: Option<(u32, (f32, f32, f32))> = None;
            let mut tied = false;
            for index in grid.within(&points, position, range + source_radius + max_radius) {
                let (target, target_position) = actors[index];
                if !eligible_target(world, source, target, &repairer)
                    .is_ok_and(|(health, max)| health < max)
                    || !in_reach(world, source, target, range)
                {
                    continue;
                }
                let gap = (position.distance(target_position)
                    - source_radius
                    - radius(world, target).unwrap_or(0.0))
                .max(0.0);
                let rank = (gap, target_position.x, target_position.y);
                let comparison = best.as_ref().map(|(_, old)| {
                    rank.0
                        .total_cmp(&old.0)
                        .then_with(|| rank.1.total_cmp(&old.1))
                        .then_with(|| rank.2.total_cmp(&old.2))
                });
                match comparison {
                    None | Some(std::cmp::Ordering::Less) => {
                        best = Some((target, rank));
                        tied = false;
                    }
                    Some(std::cmp::Ordering::Equal) => tied = true,
                    _ => {}
                }
            }
            (!tied).then(|| best.map(|(target, _)| target)).flatten()
        };
        if let Some(target) = target {
            suppress_acquired(world, source);
            match approach(world, source, target, repairer.reach)
                .and_then(|status| begin_action(world, source, target, true, status))
            {
                Ok(()) => candidates.push((source, position, repairer)),
                Err(_) => {
                    order_changed(world, source);
                    clear_motion(world, source);
                }
            }
        }
    }
    for (index, (source, position, repairer)) in candidates.iter().enumerate() {
        let tied = index
            .checked_sub(1)
            .is_some_and(|previous| candidates[previous].1 == *position)
            || candidates
                .get(index + 1)
                .is_some_and(|next| next.1 == *position);
        if tied {
            hold(world, *source, RejectReason::Blocked);
            continue;
        }
        heal_source(world, *source, repairer);
    }
}
