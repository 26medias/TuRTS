//! Deterministic gathering, receiver selection, and cargo delivery.

use std::collections::BTreeMap;

use bevy_ecs::{prelude::*, world::World};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use turts_mod_api::{ContentId, ResolvedDropOffEffect};

use super::{
    EconomyOrders, SurfaceNodes,
    events::EconomyEventKind,
    ledger::{Account, Endpoint},
    transaction::transact,
};
use crate::{
    Owner, RejectReason, SimClock, SlotId, UnitId,
    bootstrap::InstalledCatalog,
    ids::NodeId,
    registry::UnitTypes,
    world::{MapBounds, MoveTarget, Position, UnitType},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaitReason {
    NoDeposit,
    NoReceiver,
    DestinationFull,
    DestinationOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatherStage {
    Idle,
    MovingToNode,
    Gathering,
    MovingToReceiver,
    Unloading,
    Waiting { reason: WaitReason },
}

impl GatherStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::MovingToNode => "MovingToNode",
            Self::Gathering => "Gathering",
            Self::MovingToReceiver => "MovingToReceiver",
            Self::Unloading => "Unloading",
            Self::Waiting {
                reason: WaitReason::NoDeposit,
            } => "Waiting(NoDeposit)",
            Self::Waiting {
                reason: WaitReason::NoReceiver,
            } => "Waiting(NoReceiver)",
            Self::Waiting {
                reason: WaitReason::DestinationFull,
            } => "Waiting(DestinationFull)",
            Self::Waiting {
                reason: WaitReason::DestinationOverflow,
            } => "Waiting(DestinationOverflow)",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RateCarry(u16);

impl RateCarry {
    pub(crate) fn remainder(self) -> u16 {
        self.0
    }

    pub fn take_budget(&mut self, per_second: u64, tick_hz: u16) -> Result<u64, RejectReason> {
        if tick_hz == 0 {
            return Err(RejectReason::InvalidAmount);
        }
        let numerator = u128::from(per_second)
            .checked_add(u128::from(self.0))
            .ok_or(RejectReason::ArithmeticOverflow)?;
        let hz = u128::from(tick_hz);
        let budget = numerator / hz;
        self.0 = u16::try_from(numerator % hz).map_err(|_| RejectReason::ArithmeticOverflow)?;
        u64::try_from(budget).map_err(|_| RejectReason::ArithmeticOverflow)
    }
}

pub fn assign_gather(
    world: &mut World,
    slot: SlotId,
    unit: u32,
    node: NodeId,
) -> Result<(), RejectReason> {
    let entity = find_unit(world, unit).ok_or(RejectReason::UnknownUnit)?;
    if world.get::<Owner>(entity).map(|owner| owner.0) != Some(slot) {
        return Err(RejectReason::NotOwner);
    }
    let unit_type = world
        .get::<UnitType>(entity)
        .ok_or(RejectReason::UnknownUnit)?
        .0;
    let types = world
        .get_resource::<UnitTypes>()
        .ok_or(RejectReason::MissingCapability)?;
    let type_entry = types
        .get(unit_type)
        .ok_or(RejectReason::MissingCapability)?;
    if type_entry.movement.is_none() {
        return Err(RejectReason::NotMobile);
    }
    let catalog = world
        .get_resource::<InstalledCatalog>()
        .ok_or(RejectReason::MissingCapability)?;
    let unit_id =
        ContentId::parse(&type_entry.content_id).map_err(|_| RejectReason::MissingCapability)?;
    let gatherer = catalog
        .0
        .units
        .get(&unit_id)
        .and_then(|definition| definition.gatherer.as_ref())
        .ok_or(RejectReason::MissingCapability)?;
    let node_record = world
        .get_resource::<SurfaceNodes>()
        .and_then(|nodes| nodes.0.iter().find(|record| record.id == node))
        .ok_or(RejectReason::UnknownNode)?;
    if node_record.remaining == 0 {
        return Err(RejectReason::NodeDepleted);
    }
    let node_item = catalog
        .0
        .nodes
        .get(&node_record.content_id)
        .ok_or(RejectReason::UnknownNode)?
        .item
        .clone();
    if !gatherer.items.contains(&node_item) {
        return Err(RejectReason::IncompatibleItem);
    }
    let Some(_existing) = world
        .get_resource::<EconomyOrders>()
        .and_then(|orders| orders.0.get(&unit))
    else {
        return Err(RejectReason::MissingCapability);
    };
    let inventory = world
        .resource::<crate::EconomyLedger>()
        .inventories
        .get(&unit)
        .ok_or(RejectReason::MissingCapability)?;
    let has_other_item = inventory
        .items
        .iter()
        .any(|(item, account)| account.on_hand > account.reserved && item != &node_item);
    let used = inventory
        .items
        .values()
        .try_fold(0u128, |sum, account| {
            sum.checked_add(u128::from(account.on_hand))
        })
        .ok_or(RejectReason::ArithmeticOverflow)?;
    let must_deliver = used >= u128::from(inventory.capacity) || has_other_item;
    world.entity_mut(entity).remove::<MoveTarget>();
    let mut orders = world.resource_mut::<EconomyOrders>();
    let order = orders
        .0
        .get_mut(&unit)
        .ok_or(RejectReason::MissingCapability)?;
    order.node = Some(node);
    order.deposit = Some(node);
    order.receiver = None;
    order.stage = if must_deliver {
        GatherStage::MovingToReceiver
    } else {
        GatherStage::MovingToNode
    };
    order.carry = RateCarry::default();
    Ok(())
}

pub fn interrupt_order(world: &mut World, slot: SlotId, unit: u32) -> Result<(), RejectReason> {
    let entity = find_unit(world, unit).ok_or(RejectReason::UnknownUnit)?;
    if world.get::<Owner>(entity).map(|owner| owner.0) != Some(slot) {
        return Err(RejectReason::NotOwner);
    }
    if !world.resource::<EconomyOrders>().0.contains_key(&unit) {
        return Err(RejectReason::MissingCapability);
    }
    world.entity_mut(entity).remove::<MoveTarget>();
    let mut orders = world.resource_mut::<EconomyOrders>();
    let order = orders
        .0
        .get_mut(&unit)
        .ok_or(RejectReason::MissingCapability)?;
    order.node = None;
    order.deposit = None;
    order.receiver = None;
    order.stage = GatherStage::Idle;
    order.carry = RateCarry::default();
    Ok(())
}

pub(crate) fn run_gather(world: &mut World) {
    if !world.contains_resource::<InstalledCatalog>() || !world.contains_resource::<EconomyOrders>()
    {
        return;
    }
    let mut units: Vec<_> = world
        .resource::<EconomyOrders>()
        .0
        .keys()
        .copied()
        .collect();
    units.sort_unstable();
    let auditing = world
        .get_resource::<super::transaction::AuditTrail>()
        .is_some_and(|t| t.enabled);
    if !auditing {
        world.resource_scope(|world, catalog: Mut<InstalledCatalog>| {
            for unit in units {
                process_gatherer(world, unit, &catalog.0);
            }
        });
        return;
    }
    for unit in units {
        super::transaction::capture_operation(world, |world| {
            let catalog = world.resource::<InstalledCatalog>().0.clone();
            process_gatherer(world, unit, &catalog);
        });
    }
}

fn process_gatherer(world: &mut World, unit: u32, catalog: &turts_mod_api::ContentCatalog) {
    let Some(order) = world.resource::<EconomyOrders>().0.get(&unit).cloned() else {
        return;
    };
    if order.stage == GatherStage::Idle {
        return;
    }
    let Some(entity) = find_unit(world, unit) else {
        return;
    };
    let Some(owner) = world.get::<Owner>(entity).map(|owner| owner.0) else {
        return;
    };
    let Some(position) = world.get::<Position>(entity).map(|position| position.0) else {
        return;
    };
    let Some(unit_type) = world.get::<UnitType>(entity).map(|kind| kind.0) else {
        return;
    };
    let Some(type_entry) = world.resource::<UnitTypes>().get(unit_type).cloned() else {
        return;
    };
    let Ok(content_id) = ContentId::parse(&type_entry.content_id) else {
        return;
    };
    let Some(gatherer) = catalog
        .units
        .get(&content_id)
        .and_then(|definition| definition.gatherer.as_ref())
    else {
        return;
    };
    let Some(node_id) = order.deposit else {
        return;
    };
    match order.stage {
        GatherStage::Idle => {}
        GatherStage::MovingToNode => move_or_arrive_node(
            world,
            unit,
            position,
            type_entry.radius,
            gatherer,
            node_id,
            catalog,
        ),
        GatherStage::Gathering => gather_one_tick(world, unit, owner, gatherer, node_id, catalog),
        GatherStage::MovingToReceiver | GatherStage::Waiting { .. }
            if cargo_amount(world, unit) > 0 =>
        {
            move_or_arrive_receiver(
                world,
                unit,
                owner,
                position,
                type_entry.radius,
                node_id,
                gatherer,
                catalog,
            )
        }
        GatherStage::Waiting {
            reason: WaitReason::NoDeposit,
        } => {}
        GatherStage::Unloading => {
            unload_one_tick(world, unit, owner, position, node_id, gatherer, catalog)
        }
        GatherStage::MovingToReceiver | GatherStage::Waiting { .. } => {
            resume_after_delivery(world, unit, position, node_id, gatherer, catalog);
        }
    }
}

fn resume_after_delivery(
    world: &mut World,
    unit: u32,
    position: Vec2,
    current_deposit: NodeId,
    gatherer: &turts_mod_api::ResolvedGatherer,
    catalog: &turts_mod_api::ContentCatalog,
) {
    let original = world
        .resource::<EconomyOrders>()
        .0
        .get(&unit)
        .and_then(|order| order.node);
    let original_live = original.filter(|id| {
        world
            .resource::<SurfaceNodes>()
            .0
            .iter()
            .any(|node| node.id == *id && node.remaining > 0)
    });
    let item = world
        .resource::<SurfaceNodes>()
        .0
        .iter()
        .find(|node| node.id == current_deposit)
        .and_then(|node| catalog.nodes.get(&node.content_id))
        .map(|node| &node.item);
    let next = original_live.or_else(|| {
        item.and_then(|item| closest_deposit(world, position, item, gatherer, catalog))
            .map(|node| node.id)
    });
    if let Some(next) = next {
        set_deposit(world, unit, next);
        set_stage(world, unit, GatherStage::MovingToNode, None);
    } else {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::NoDeposit,
            },
            None,
        );
    }
}

fn closest_deposit(
    world: &World,
    position: Vec2,
    item: &ContentId,
    gatherer: &turts_mod_api::ResolvedGatherer,
    catalog: &turts_mod_api::ContentCatalog,
) -> Option<super::NodeRecord> {
    world
        .resource::<SurfaceNodes>()
        .0
        .iter()
        .filter(|candidate| {
            candidate.remaining > 0
                && catalog
                    .nodes
                    .get(&candidate.content_id)
                    .is_some_and(|definition| {
                        &definition.item == item && gatherer.items.contains(item)
                    })
        })
        .min_by(|a, b| {
            position
                .distance_squared(a.position.0)
                .total_cmp(&position.distance_squared(b.position.0))
                .then(a.id.cmp(&b.id))
        })
        .cloned()
}

fn move_or_arrive_node(
    world: &mut World,
    unit: u32,
    position: Vec2,
    unit_radius: f32,
    gatherer: &turts_mod_api::ResolvedGatherer,
    node_id: NodeId,
    catalog: &turts_mod_api::ContentCatalog,
) {
    let Some(node) = world
        .resource::<SurfaceNodes>()
        .0
        .iter()
        .find(|node| node.id == node_id)
        .cloned()
    else {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::NoDeposit,
            },
            None,
        );
        return;
    };
    if node.remaining == 0 {
        if cargo_amount(world, unit) > 0 {
            set_stage(world, unit, GatherStage::MovingToReceiver, None);
            return;
        }
        let Some(item) = catalog
            .nodes
            .get(&node.content_id)
            .map(|definition| &definition.item)
        else {
            return;
        };
        let replacement = closest_deposit(world, position, item, gatherer, catalog);
        if let Some(next) = replacement {
            set_deposit(world, unit, next.id);
            set_stage(world, unit, GatherStage::MovingToNode, None);
        } else {
            set_stage(
                world,
                unit,
                GatherStage::Waiting {
                    reason: WaitReason::NoDeposit,
                },
                None,
            );
        }
        return;
    }
    let Some(node_def) = catalog.nodes.get(&node.content_id) else {
        return;
    };
    let reach = unit_radius + node_def.radius + gatherer.interaction_range;
    if position.distance(node.position.0) <= reach + 1.0e-4 {
        if let Some(entity) = find_unit(world, unit) {
            world.entity_mut(entity).remove::<MoveTarget>();
        }
        set_stage(world, unit, GatherStage::Gathering, None);
        return;
    }
    let target = approach_position(
        position,
        node.position.0,
        unit_radius + node_def.radius + gatherer.interaction_range,
        unit_radius,
        world.get_resource::<MapBounds>().map(|bounds| bounds.size),
    );
    set_target(world, unit, target);
}

fn gather_one_tick(
    world: &mut World,
    unit: u32,
    owner: SlotId,
    gatherer: &turts_mod_api::ResolvedGatherer,
    node_id: NodeId,
    catalog: &turts_mod_api::ContentCatalog,
) {
    let Some(node) = world
        .resource::<SurfaceNodes>()
        .0
        .iter()
        .find(|node| node.id == node_id)
        .cloned()
    else {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::NoDeposit,
            },
            None,
        );
        return;
    };
    if node.remaining == 0 {
        if cargo_amount(world, unit) > 0 {
            set_stage(world, unit, GatherStage::MovingToReceiver, None);
            return;
        }
        let Some(item) = catalog
            .nodes
            .get(&node.content_id)
            .map(|definition| &definition.item)
        else {
            return;
        };
        let current_position = find_unit(world, unit)
            .and_then(|entity| world.get::<Position>(entity).map(|position| position.0))
            .unwrap_or(Vec2::ZERO);
        let replacement =
            closest_deposit(world, current_position, item, gatherer, catalog).map(|node| node.id);
        if let Some(next) = replacement {
            set_deposit(world, unit, next);
            set_stage(world, unit, GatherStage::MovingToNode, None);
        } else {
            set_stage(
                world,
                unit,
                GatherStage::Waiting {
                    reason: WaitReason::NoDeposit,
                },
                None,
            );
        }
        return;
    }
    let Some(node_def) = catalog.nodes.get(&node.content_id) else {
        return;
    };
    if !gatherer.items.contains(&node_def.item) {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::NoDeposit,
            },
            None,
        );
        return;
    }
    let Some(inventory) = world
        .resource::<crate::EconomyLedger>()
        .inventories
        .get(&unit)
    else {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::DestinationFull,
            },
            None,
        );
        return;
    };
    let used = inventory
        .items
        .values()
        .map(|account| u128::from(account.on_hand))
        .sum::<u128>();
    let headroom =
        u64::try_from(u128::from(inventory.capacity).saturating_sub(used)).unwrap_or(u64::MAX);
    if headroom == 0 {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::DestinationFull,
            },
            None,
        );
        return;
    }
    if world
        .resource::<crate::economy::events::NextEventOrdinal>()
        .0
        .is_none()
    {
        return;
    }
    let budget = match take_budget(world, unit, gatherer.gather_per_second) {
        Ok(value) => value,
        Err(_) => return,
    };
    let amount = budget.min(node.remaining).min(headroom);
    if amount == 0 {
        return;
    }
    if extract(world, unit, &node_def.item, amount).is_err() {
        return;
    }
    if let Some(record) = world
        .resource_mut::<SurfaceNodes>()
        .0
        .iter_mut()
        .find(|record| record.id == node_id)
    {
        record.remaining = record.remaining.saturating_sub(amount);
    }
    let _ = crate::economy::events::record_committed_event(
        world,
        owner.0,
        EconomyEventKind::Extracted {
            node: node_id.0,
            cargo: unit,
            item: node_def.item.clone(),
            amount,
        },
    );
    let depleted = world
        .resource::<SurfaceNodes>()
        .0
        .iter()
        .find(|record| record.id == node_id)
        .is_some_and(|record| record.remaining == 0);
    let capacity = world
        .resource::<crate::EconomyLedger>()
        .inventories
        .get(&unit)
        .map_or(0, |inventory| inventory.capacity);
    if depleted || cargo_amount(world, unit) >= capacity {
        set_stage(world, unit, GatherStage::MovingToReceiver, None);
    }
}

#[allow(clippy::too_many_arguments)]
fn move_or_arrive_receiver(
    world: &mut World,
    unit: u32,
    owner: SlotId,
    position: Vec2,
    unit_radius: f32,
    node_id: NodeId,
    gatherer: &turts_mod_api::ResolvedGatherer,
    catalog: &turts_mod_api::ContentCatalog,
) {
    let Some(item) = lowest_cargo(world, unit) else {
        resume_after_delivery(world, unit, position, node_id, gatherer, catalog);
        return;
    };
    let selected = choose_receiver(world, unit, owner, position, &item, catalog);
    let Some((receiver, receiver_pos, receiver_radius)) = selected else {
        let reason = receiver_wait_reason(world, unit, owner, &item, catalog);
        set_stage(world, unit, GatherStage::Waiting { reason }, None);
        return;
    };
    let Some(entity) = find_unit(world, unit) else {
        return;
    };
    let Some(gatherer_pos) = world.get::<Position>(entity).map(|pos| pos.0) else {
        return;
    };
    if receiver
        != world.resource::<EconomyOrders>().0[&unit]
            .receiver
            .unwrap_or(u32::MAX)
    {
        set_stage(world, unit, GatherStage::MovingToReceiver, Some(receiver));
        return;
    }
    let receiver_range = unit_radius + receiver_radius + gatherer.interaction_range;
    if gatherer_pos.distance(receiver_pos) <= receiver_range + 1.0e-4 {
        if let Some(entity) = find_unit(world, unit) {
            world.entity_mut(entity).remove::<MoveTarget>();
        }
        set_stage(world, unit, GatherStage::Unloading, Some(receiver));
        return;
    }
    let target = approach_position(
        position,
        receiver_pos,
        receiver_range,
        unit_radius,
        world.get_resource::<MapBounds>().map(|bounds| bounds.size),
    );
    set_target(world, unit, target);
    let _ = gatherer;
    let _ = node_id;
}

fn unload_one_tick(
    world: &mut World,
    unit: u32,
    owner: SlotId,
    position: Vec2,
    node_id: NodeId,
    gatherer: &turts_mod_api::ResolvedGatherer,
    catalog: &turts_mod_api::ContentCatalog,
) {
    let Some(item) = lowest_cargo(world, unit) else {
        resume_after_delivery(world, unit, position, node_id, gatherer, catalog);
        return;
    };
    let Some((receiver, receiver_pos, receiver_radius)) =
        choose_receiver(world, unit, owner, position, &item, catalog)
    else {
        let reason = receiver_wait_reason(world, unit, owner, &item, catalog);
        set_stage(world, unit, GatherStage::Waiting { reason }, None);
        return;
    };
    if receiver
        != world.resource::<EconomyOrders>().0[&unit]
            .receiver
            .unwrap_or(u32::MAX)
    {
        set_stage(world, unit, GatherStage::MovingToReceiver, Some(receiver));
        return;
    }
    let Some(entity) = find_unit(world, unit) else {
        return;
    };
    let Some(unit_type) = world.get::<UnitType>(entity).map(|kind| kind.0) else {
        return;
    };
    let radius = world
        .resource::<UnitTypes>()
        .get(unit_type)
        .map_or(0.0, |entry| entry.radius);
    if position.distance(receiver_pos)
        > radius + receiver_radius + gatherer.interaction_range + 1.0e-4
    {
        set_stage(world, unit, GatherStage::MovingToReceiver, Some(receiver));
        return;
    }
    if world
        .resource::<crate::economy::events::NextEventOrdinal>()
        .0
        .is_none()
    {
        return;
    }
    let budget = match take_budget(world, unit, gatherer.unload_per_second) {
        Ok(value) => value,
        Err(_) => return,
    };
    if budget == 0 {
        return;
    }
    let Some((effect, available)) = receiver_rule(world, receiver, owner, &item, catalog) else {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::NoReceiver,
            },
            None,
        );
        return;
    };
    let amount = budget
        .min(cargo_available(world, unit, &item))
        .min(available);
    if amount == 0 {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: receiver_wait_reason(world, unit, owner, &item, catalog),
            },
            None,
        );
        return;
    }
    let credited = match effect {
        ResolvedDropOffEffect::Store => amount,
        ResolvedDropOffEffect::Convert { per_item, .. } => match amount.checked_mul(per_item) {
            Some(value) => value,
            None => {
                set_stage(
                    world,
                    unit,
                    GatherStage::Waiting {
                        reason: WaitReason::DestinationOverflow,
                    },
                    None,
                );
                return;
            }
        },
    };
    if !world
        .resource::<crate::economy::events::NextEventOrdinal>()
        .0
        .is_some()
    {
        return;
    }
    let result = match &effect {
        ResolvedDropOffEffect::Store => store(world, owner, unit, receiver, &item, amount),
        ResolvedDropOffEffect::Convert { balance, per_item } => convert(
            world, owner, unit, receiver, &item, balance, *per_item, amount,
        ),
    };
    if result.is_err() {
        set_stage(
            world,
            unit,
            GatherStage::Waiting {
                reason: WaitReason::DestinationOverflow,
            },
            None,
        );
        return;
    }
    let kind = match effect {
        ResolvedDropOffEffect::Store => EconomyEventKind::Stored {
            from: unit,
            to: receiver,
            item: item.clone(),
            amount,
        },
        ResolvedDropOffEffect::Convert { balance, .. } => EconomyEventKind::Converted {
            from: unit,
            to: receiver,
            item: item.clone(),
            amount,
            balance,
            credited,
        },
    };
    let _ = crate::economy::events::record_committed_event(world, owner.0, kind);
    let _ = node_id;
}

fn extract(
    world: &mut World,
    unit: u32,
    item: &ContentId,
    amount: u64,
) -> Result<(), RejectReason> {
    transact(world, |tx| {
        let endpoint = Endpoint::Inventory(unit);
        tx.validate_endpoint(endpoint, item)?;
        let inventory = tx.inventory(unit)?;
        let used = tx.inventory_used(unit)?;
        if used
            .checked_add(u128::from(amount))
            .ok_or(RejectReason::ArithmeticOverflow)?
            > u128::from(inventory.capacity)
        {
            return Err(RejectReason::RefundDestinationFull);
        }
        let old = tx.account(endpoint, item);
        let on_hand = old
            .on_hand
            .checked_add(amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        tx.stage_account(endpoint, item.clone(), Account { on_hand, ..old });
        Ok(())
    })
}

fn store(
    world: &mut World,
    owner: SlotId,
    from: u32,
    to: u32,
    item: &ContentId,
    amount: u64,
) -> Result<(), RejectReason> {
    if from == to {
        return Err(RejectReason::InvalidAmount);
    }
    if owner_of(world, from)? != owner || owner_of(world, to)? != owner {
        return Err(RejectReason::NotOwner);
    }
    transact(world, |tx| {
        let src = Endpoint::Inventory(from);
        let dst = Endpoint::Inventory(to);
        tx.validate_endpoint(src, item)?;
        tx.validate_endpoint(dst, item)?;
        let source = tx.account(src, item);
        let destination = tx.account(dst, item);
        let available = source
            .on_hand
            .checked_sub(source.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if available < amount {
            return Err(RejectReason::InsufficientItems);
        }
        let used = tx.inventory_used(to)?;
        if used
            .checked_add(u128::from(amount))
            .ok_or(RejectReason::ArithmeticOverflow)?
            > u128::from(tx.inventory(to)?.capacity)
        {
            return Err(RejectReason::RefundDestinationFull);
        }
        tx.stage_account(
            src,
            item.clone(),
            Account {
                on_hand: source.on_hand - amount,
                ..source
            },
        );
        tx.stage_account(
            dst,
            item.clone(),
            Account {
                on_hand: destination
                    .on_hand
                    .checked_add(amount)
                    .ok_or(RejectReason::ArithmeticOverflow)?,
                ..destination
            },
        );
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
fn convert(
    world: &mut World,
    owner: SlotId,
    from: u32,
    _to: u32,
    item: &ContentId,
    balance: &ContentId,
    per_item: u64,
    amount: u64,
) -> Result<(), RejectReason> {
    if owner_of(world, from)? != owner {
        return Err(RejectReason::NotOwner);
    }
    transact(world, |tx| {
        let inventory_endpoint = Endpoint::Inventory(from);
        let balance_endpoint = Endpoint::Slot(owner.0);
        tx.validate_endpoint(inventory_endpoint, item)?;
        tx.validate_endpoint(balance_endpoint, balance)?;
        let source = tx.account(inventory_endpoint, item);
        let destination = tx.account(balance_endpoint, balance);
        if source
            .on_hand
            .checked_sub(source.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?
            < amount
        {
            return Err(RejectReason::InsufficientItems);
        }
        let credit = amount
            .checked_mul(per_item)
            .ok_or(RejectReason::RefundDestinationOverflow)?;
        let on_hand = destination
            .on_hand
            .checked_add(credit)
            .ok_or(RejectReason::RefundDestinationOverflow)?;
        tx.stage_account(
            inventory_endpoint,
            item.clone(),
            Account {
                on_hand: source.on_hand - amount,
                ..source
            },
        );
        tx.stage_account(
            balance_endpoint,
            balance.clone(),
            Account {
                on_hand,
                ..destination
            },
        );
        Ok(())
    })
}

fn choose_receiver(
    world: &World,
    unit: u32,
    owner: SlotId,
    position: Vec2,
    item: &ContentId,
    catalog: &turts_mod_api::ContentCatalog,
) -> Option<(u32, Vec2, f32)> {
    let types = world.get_resource::<UnitTypes>()?;
    let positions: BTreeMap<_, _> = world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?.0;
            let unit_type = entity.get::<UnitType>()?.0;
            let owner_id = entity.get::<Owner>()?.0;
            let pos = entity.get::<Position>()?.0;
            (owner_id == owner).then_some((id, (pos, unit_type)))
        })
        .collect();
    positions
        .iter()
        .filter_map(|(&id, &(pos, unit_type))| {
            let entry = types.get(unit_type)?;
            let content = ContentId::parse(&entry.content_id).ok()?;
            catalog.units.get(&content)?.drop_off.as_ref()?;
            let (effect, available) = receiver_rule(world, id, owner, item, catalog)?;
            if id == unit && matches!(effect, ResolvedDropOffEffect::Store) {
                return None;
            }
            if available == 0 {
                return None;
            }
            Some((id, position.distance_squared(pos), pos, entry.radius))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(id, _, pos, radius)| (id, pos, radius))
}

fn receiver_rule(
    world: &World,
    receiver: u32,
    owner: SlotId,
    item: &ContentId,
    catalog: &turts_mod_api::ContentCatalog,
) -> Option<(ResolvedDropOffEffect, u64)> {
    let types = world.get_resource::<UnitTypes>()?;
    let entity = find_unit(world, receiver)?;
    let type_id = world.get::<UnitType>(entity)?.0;
    let type_entry = types.get(type_id)?;
    let content = ContentId::parse(&type_entry.content_id).ok()?;
    let rule = catalog
        .units
        .get(&content)?
        .drop_off
        .as_ref()?
        .rules
        .iter()
        .find(|rule| &rule.item == item)?;
    match &rule.effect {
        ResolvedDropOffEffect::Store => {
            let inventory = world
                .resource::<crate::EconomyLedger>()
                .inventories
                .get(&receiver)?;
            if !inventory.accepted_items.contains(item) {
                return None;
            }
            let used = inventory
                .items
                .values()
                .map(|account| u128::from(account.on_hand))
                .sum::<u128>();
            let capacity = u128::from(inventory.capacity);
            let headroom = u64::try_from(capacity.saturating_sub(used)).unwrap_or(u64::MAX);
            Some((ResolvedDropOffEffect::Store, headroom))
        }
        ResolvedDropOffEffect::Convert { balance, per_item } => {
            let ledger = world.get_resource::<crate::EconomyLedger>()?;
            let account = ledger.balances.get(&(owner.0, balance.clone()))?;
            if *per_item == 0 {
                return Some((rule.effect.clone(), u64::MAX));
            }
            let headroom = u64::MAX.saturating_sub(account.on_hand) / *per_item;
            Some((rule.effect.clone(), headroom))
        }
    }
}

fn receiver_wait_reason(
    world: &World,
    unit: u32,
    owner: SlotId,
    item: &ContentId,
    catalog: &turts_mod_api::ContentCatalog,
) -> WaitReason {
    let mut saw_overflow = false;
    let mut saw_full = false;
    let types = world.get_resource::<UnitTypes>();
    for entity in world.iter_entities() {
        if entity.get::<Owner>().is_none_or(|slot| slot.0 != owner) {
            continue;
        }
        let Some(kind) = entity
            .get::<UnitType>()
            .and_then(|kind| types.and_then(|types| types.get(kind.0)))
        else {
            continue;
        };
        let Ok(id) = ContentId::parse(&kind.content_id) else {
            continue;
        };
        let Some(rule) = catalog
            .units
            .get(&id)
            .and_then(|definition| definition.drop_off.as_ref())
            .and_then(|drop_off| drop_off.rules.iter().find(|rule| &rule.item == item))
        else {
            continue;
        };
        match &rule.effect {
            ResolvedDropOffEffect::Store => {
                let receiver = entity.get::<UnitId>().map_or(u32::MAX, |id| id.0);
                if receiver == unit {
                    continue;
                }
                if let Some(inventory) = world
                    .resource::<crate::EconomyLedger>()
                    .inventories
                    .get(&receiver)
                    && inventory.accepted_items.contains(item)
                {
                    let used = inventory
                        .items
                        .values()
                        .map(|account| u128::from(account.on_hand))
                        .sum::<u128>();
                    saw_full |= used >= u128::from(inventory.capacity);
                }
            }
            ResolvedDropOffEffect::Convert { balance, per_item } => {
                if world
                    .resource::<crate::EconomyLedger>()
                    .balances
                    .get(&(owner.0, balance.clone()))
                    .is_some_and(|account| {
                        *per_item > 0 && u64::MAX.saturating_sub(account.on_hand) / *per_item == 0
                    })
                {
                    saw_overflow = true;
                }
            }
        }
    }
    if saw_full {
        WaitReason::DestinationFull
    } else if saw_overflow {
        WaitReason::DestinationOverflow
    } else {
        WaitReason::NoReceiver
    }
}

fn approach_position(
    from: Vec2,
    center: Vec2,
    reach: f32,
    unit_radius: f32,
    bounds: Option<Vec2>,
) -> Vec2 {
    let mut direction = from - center;
    if direction.length_squared() <= f32::EPSILON {
        direction = Vec2::X;
    } else {
        direction = direction.normalize();
    }
    let mut target = center + direction * reach;
    if let Some(bounds) = bounds {
        target.x = target
            .x
            .clamp(unit_radius, (bounds.x - unit_radius).max(unit_radius));
        target.y = target
            .y
            .clamp(unit_radius, (bounds.y - unit_radius).max(unit_radius));
    }
    target
}

fn take_budget(world: &mut World, unit: u32, rate: u64) -> Result<u64, RejectReason> {
    let hz = world.resource::<SimClock>().tick_hz;
    world
        .resource_mut::<EconomyOrders>()
        .0
        .get_mut(&unit)
        .ok_or(RejectReason::MissingCapability)?
        .carry
        .take_budget(rate, hz)
}

fn lowest_cargo(world: &World, unit: u32) -> Option<ContentId> {
    world
        .resource::<crate::EconomyLedger>()
        .inventories
        .get(&unit)?
        .items
        .iter()
        .filter(|(_, account)| account.on_hand > account.reserved)
        .map(|(item, _)| item.clone())
        .next()
}

fn cargo_amount(world: &World, unit: u32) -> u64 {
    world
        .resource::<crate::EconomyLedger>()
        .inventories
        .get(&unit)
        .map_or(0, |inventory| {
            inventory
                .items
                .values()
                .map(|account| account.on_hand.saturating_sub(account.reserved))
                .sum()
        })
}
fn cargo_available(world: &World, unit: u32, item: &ContentId) -> u64 {
    world
        .resource::<crate::EconomyLedger>()
        .inventories
        .get(&unit)
        .and_then(|inventory| inventory.items.get(item))
        .map_or(0, |account| {
            account.on_hand.saturating_sub(account.reserved)
        })
}
fn owner_of(world: &World, unit: u32) -> Result<SlotId, RejectReason> {
    let entity = find_unit(world, unit).ok_or(RejectReason::UnknownUnit)?;
    world
        .get::<Owner>(entity)
        .map(|owner| owner.0)
        .ok_or(RejectReason::UnknownUnit)
}
fn find_unit(world: &World, unit: u32) -> Option<Entity> {
    world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit))
        .map(|entity| entity.id())
}

fn set_stage(world: &mut World, unit: u32, stage: GatherStage, receiver: Option<u32>) {
    let changed = {
        let mut orders = world.resource_mut::<EconomyOrders>();
        let Some(order) = orders.0.get_mut(&unit) else {
            return;
        };
        let changed = order.stage != stage;
        if changed {
            order.stage = stage;
            order.carry = RateCarry::default();
        }
        if let Some(receiver) = receiver {
            order.receiver = Some(receiver);
        } else if !matches!(
            stage,
            GatherStage::MovingToReceiver | GatherStage::Unloading
        ) {
            order.receiver = None;
        }
        changed
    };
    if (changed || matches!(stage, GatherStage::Waiting { .. } | GatherStage::Idle))
        && let Some(entity) = find_unit(world, unit)
    {
        world.entity_mut(entity).remove::<MoveTarget>();
    }
}
fn set_deposit(world: &mut World, unit: u32, deposit: NodeId) {
    if let Some(order) = world.resource_mut::<EconomyOrders>().0.get_mut(&unit) {
        order.deposit = Some(deposit);
    }
    if let Some(entity) = find_unit(world, unit) {
        world.entity_mut(entity).remove::<MoveTarget>();
    }
}
fn set_target(world: &mut World, unit: u32, target: Vec2) {
    if let Some(entity) = find_unit(world, unit) {
        if let Some(mut current) = world.get_mut::<MoveTarget>(entity) {
            current.0 = target;
        } else {
            world.entity_mut(entity).insert(MoveTarget(target));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::GatherOrder;

    #[test]
    fn carry_accumulates_fractional_budget_at_each_rate() {
        for hz in [10, 20, 30] {
            let mut carry = RateCarry::default();
            let total: u64 = (0..hz).map(|_| carry.take_budget(5, hz).unwrap()).sum();
            assert_eq!(total, 5);
            assert_eq!(carry, RateCarry::default());
        }
    }

    #[test]
    fn approach_is_stable_when_centers_coincide_and_clips_to_map_edges() {
        let coincident = approach_position(
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 10.0),
            2.0,
            0.5,
            Some(Vec2::splat(64.0)),
        );
        assert_eq!(coincident, Vec2::new(12.0, 10.0));
        let clipped = approach_position(
            Vec2::new(0.0, 5.0),
            Vec2::new(5.0, 5.0),
            10.0,
            1.0,
            Some(Vec2::splat(10.0)),
        );
        assert_eq!(clipped, Vec2::new(1.0, 5.0));
    }

    #[test]
    fn unused_whole_budget_is_not_carried_forward() {
        let mut carry = RateCarry::default();
        assert_eq!(carry.take_budget(25, 10), Ok(2));
        assert_eq!(carry.take_budget(25, 10), Ok(3));
        assert_eq!(carry.take_budget(25, 10), Ok(2));
    }

    #[test]
    fn phase_transition_resets_fractional_carry() {
        let mut carry = RateCarry::default();
        assert_eq!(carry.take_budget(1, 10), Ok(0));
        let mut world = World::new();
        let mut orders = BTreeMap::new();
        orders.insert(
            1,
            GatherOrder {
                node: Some(NodeId(1)),
                deposit: Some(NodeId(1)),
                receiver: None,
                stage: GatherStage::Gathering,
                carry,
            },
        );
        world.insert_resource(EconomyOrders(orders));
        set_stage(&mut world, 1, GatherStage::MovingToReceiver, Some(2));
        assert_eq!(
            world.resource::<EconomyOrders>().0[&1].carry,
            RateCarry::default()
        );
    }

    #[test]
    fn carry_rejects_zero_rate_clock_and_overflow() {
        let mut carry = RateCarry::default();
        assert_eq!(carry.take_budget(1, 0), Err(RejectReason::InvalidAmount));
        assert_eq!(carry.take_budget(1, 2), Ok(0));
        assert_eq!(carry.take_budget(u64::MAX, 2), Ok(1u64 << 63));
        assert_eq!(carry, RateCarry::default());
        let mut carry = RateCarry::default();
        assert_eq!(carry.take_budget(u64::MAX, 2), Ok(u64::MAX / 2));
        assert_eq!(carry.take_budget(u64::MAX, 2), Ok(1u64 << 63));
    }
}
