use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::{entity::Entity, prelude::Resource, world::World};
use turts_mod_api::ContentId;

#[path = "transaction_m3.rs"]
mod transaction_m3;
#[allow(unused_imports)] // Later M3 tasks use these stable internal entry points.
pub(crate) use transaction_m3::{extract_from_node, remove_unit, split_line};

use crate::{
    Owner, RejectReason, SlotId, UnitId,
    economy::views::AuditState,
    economy::{
        EconomyOrders, NodeRecord,
        combat::WeaponState,
        conveyor::BeltLine,
        events::{
            EconomyEvent, EconomyEventKind, NextEventOrdinal, PendingEconomyEvents, RemovedThisTick,
        },
        extractor::ExtractorState,
        inserter::InserterState,
        ledger::{
            Account, EconomyLedger, Endpoint, Inventory, Reservation, ReservationClaim,
            ReservationLine,
        },
        production::ProductionQueues,
    },
    ids::{IdAllocators, JobId},
    slots::Slots,
};

#[derive(Resource, Debug, Default)]
pub struct AuditTrail {
    pub enabled: bool,
    pub valid: bool,
    pub commands: Vec<CommandAudit>,
    pub operations: Vec<OperationAudit>,
    pub ticks: Vec<TickAudit>,
    next_operation_id: u64,
    capture_depth: u32,
    tick_before: Option<AuditState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandAudit {
    pub operation_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_end_id: Option<u64>,
    pub input: crate::command::Command,
    pub result: crate::command::CommandResult,
    pub before: AuditState,
    pub after: AuditState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationAudit {
    pub operation_id: u64,
    #[serde(default)]
    pub phase: AuditPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<u32>,
    pub before: AuditState,
    pub after: AuditState,
    pub events: Vec<EconomyEvent>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditPhase {
    Command,
    Orders,
    #[default]
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickAudit {
    pub tick: u32,
    pub before: AuditState,
    pub after: AuditState,
    pub events: Vec<EconomyEvent>,
}

impl AuditTrail {
    pub fn enable(&mut self) {
        self.enabled = true;
        self.valid = true;
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    pub fn drain_commands(&mut self) -> Vec<CommandAudit> {
        std::mem::take(&mut self.commands)
    }
    pub fn drain_operations(&mut self) -> Vec<OperationAudit> {
        std::mem::take(&mut self.operations)
    }
    pub fn drain_ticks(&mut self) -> Vec<TickAudit> {
        std::mem::take(&mut self.ticks)
    }
}

pub(crate) fn capture_operation<R>(world: &mut World, run: impl FnOnce(&mut World) -> R) -> R {
    capture_operation_phase(world, AuditPhase::Other, None, run)
}

pub(crate) fn capture_unit_order_operation<R>(
    world: &mut World,
    unit_id: u32,
    run: impl FnOnce(&mut World) -> R,
) -> R {
    capture_operation_phase(world, AuditPhase::Orders, Some(unit_id), run)
}

fn capture_operation_phase<R>(
    world: &mut World,
    phase: AuditPhase,
    unit_id: Option<u32>,
    run: impl FnOnce(&mut World) -> R,
) -> R {
    let enabled = world
        .get_resource::<AuditTrail>()
        .is_some_and(|trail| trail.enabled);
    if !enabled || world.resource::<AuditTrail>().capture_depth != 0 {
        return run(world);
    }
    let Some(id) = world
        .resource::<AuditTrail>()
        .next_operation_id
        .checked_add(1)
    else {
        world.resource_mut::<AuditTrail>().valid = false;
        return run(world);
    };
    let before = crate::economy::views::audit_state(world);
    let event_start = world
        .get_resource::<PendingEconomyEvents>()
        .map_or(0, |events| events.0.len());
    {
        let mut trail = world.resource_mut::<AuditTrail>();
        trail.next_operation_id = id;
        trail.capture_depth = 1;
    }
    let result = run(world);
    let after = crate::economy::views::audit_state(world);
    let events = world
        .get_resource::<PendingEconomyEvents>()
        .map_or_else(Vec::new, |events| {
            events.0.iter().skip(event_start).cloned().collect()
        });
    let mut trail = world.resource_mut::<AuditTrail>();
    trail.capture_depth = 0;
    trail.operations.push(OperationAudit {
        operation_id: id,
        phase,
        unit_id,
        before,
        after,
        events,
    });
    result
}

pub(crate) fn capture_command(
    world: &mut World,
    input: &crate::command::Command,
    run: impl FnOnce(&mut World) -> crate::command::CommandResult,
) -> crate::command::CommandResult {
    let enabled = world
        .get_resource::<AuditTrail>()
        .is_some_and(|trail| trail.enabled);
    if !enabled || world.resource::<AuditTrail>().capture_depth != 0 {
        return run(world);
    }
    let Some(id) = world
        .resource::<AuditTrail>()
        .next_operation_id
        .checked_add(1)
    else {
        world.resource_mut::<AuditTrail>().valid = false;
        return run(world);
    };
    let before = crate::economy::views::audit_state(world);
    let event_start = world
        .get_resource::<PendingEconomyEvents>()
        .map_or(0, |events| events.0.len());
    {
        let mut trail = world.resource_mut::<AuditTrail>();
        trail.next_operation_id = id;
        trail.capture_depth = 1;
    }
    let result = run(world);
    let after = crate::economy::views::audit_state(world);
    let events = world
        .get_resource::<PendingEconomyEvents>()
        .map_or_else(Vec::new, |events| {
            events.0.iter().skip(event_start).cloned().collect()
        });
    let mut trail = world.resource_mut::<AuditTrail>();
    trail.capture_depth = 0;
    trail.commands.push(CommandAudit {
        operation_id: id,
        operation_end_id: None,
        input: input.clone(),
        result: result.clone(),
        before: before.clone(),
        after: after.clone(),
    });
    trail.operations.push(OperationAudit {
        operation_id: id,
        phase: AuditPhase::Command,
        unit_id: None,
        before,
        after,
        events,
    });
    result
}

pub(crate) fn capture_order_command(
    world: &mut World,
    input: &crate::command::Command,
    run: impl FnOnce(&mut World) -> crate::command::CommandResult,
) -> crate::command::CommandResult {
    let enabled = world
        .get_resource::<AuditTrail>()
        .is_some_and(|trail| trail.enabled);
    if !enabled || world.resource::<AuditTrail>().capture_depth != 0 {
        return run(world);
    }
    let before = crate::economy::views::audit_state(world);
    let operation_start = world.resource::<AuditTrail>().operations.len();
    let result = run(world);
    let after = crate::economy::views::audit_state(world);
    let first_operation = world
        .resource::<AuditTrail>()
        .operations
        .get(operation_start)
        .cloned();
    if let Some(operation) = first_operation {
        let last_operation = world
            .resource::<AuditTrail>()
            .operations
            .last()
            .unwrap()
            .operation_id;
        world
            .resource_mut::<AuditTrail>()
            .commands
            .push(CommandAudit {
                operation_id: operation.operation_id,
                operation_end_id: (last_operation != operation.operation_id)
                    .then_some(last_operation),
                input: input.clone(),
                result: result.clone(),
                before,
                after,
            });
    } else {
        let id = world
            .resource::<AuditTrail>()
            .next_operation_id
            .checked_add(1);
        let Some(id) = id else {
            world.resource_mut::<AuditTrail>().valid = false;
            return result;
        };
        let mut trail = world.resource_mut::<AuditTrail>();
        trail.next_operation_id = id;
        trail.commands.push(CommandAudit {
            operation_id: id,
            operation_end_id: None,
            input: input.clone(),
            result: result.clone(),
            before: before.clone(),
            after: after.clone(),
        });
        trail.operations.push(OperationAudit {
            operation_id: id,
            phase: AuditPhase::Command,
            unit_id: None,
            before,
            after,
            events: Vec::new(),
        });
    }
    result
}

pub(crate) fn begin_tick_audit(world: &mut World) {
    if world
        .get_resource::<AuditTrail>()
        .is_some_and(|trail| trail.enabled)
    {
        let before = crate::economy::views::audit_state(world);
        world.resource_mut::<AuditTrail>().tick_before = Some(before);
    }
}

pub(crate) fn finish_tick_audit(world: &mut World) {
    if !world
        .get_resource::<AuditTrail>()
        .is_some_and(|trail| trail.enabled)
    {
        return;
    }
    let Some(before) = world.resource_mut::<AuditTrail>().tick_before.take() else {
        return;
    };
    let after = crate::economy::views::audit_state(world);
    let tick = after.economy.tick;
    let events = world
        .get_resource::<crate::economy::events::CommittedTick>()
        .map_or_else(Vec::new, |batch| batch.events.clone());
    world.resource_mut::<AuditTrail>().ticks.push(TickAudit {
        tick,
        before,
        after,
        events,
    });
}

#[derive(Default)]
struct WriteSet {
    accounts: BTreeMap<(Endpoint, ContentId), Account>,
    claims: BTreeMap<JobId, ReservationClaim>,
    queues: Option<ProductionQueues>,
    nodes: BTreeMap<crate::ids::NodeId, Option<NodeRecord>>,
    conveyor_lines: BTreeMap<u32, Option<BeltLine>>,
    piece_line: BTreeMap<u32, Option<(u32, u16)>>,
    inserters: BTreeMap<u32, Option<InserterState>>,
    extractors: BTreeMap<u32, Option<ExtractorState>>,
    weapons: BTreeMap<u32, Option<WeaponState>>,
    healths: BTreeMap<u32, Option<u64>>,
    orders: BTreeMap<u32, Option<crate::economy::GatherOrder>>,
    research: BTreeMap<(u8, ContentId), Option<()>>,
    disabled: BTreeMap<u32, Option<()>>,
    next_job_id: Option<u64>,
    next_unit_id: Option<u32>,
    spawned: Vec<SpawnWrite>,
    removed: Vec<RemoveWrite>,
    events: Vec<(u8, EconomyEventKind)>,
    next_event_ordinal: Option<u64>,
    strict_m3: bool,
}

struct RemoveWrite {
    unit: u32,
    entity: Entity,
}

pub(crate) struct SpawnWrite {
    pub id: u32,
    pub unit_type: turts_mod_api::UnitTypeId,
    pub owner_slot: u8,
    pub position: bevy_math::Vec2,
    pub state: crate::world::UnitState,
    pub inventory: Option<Inventory>,
    pub gatherer: bool,
    pub producer: bool,
    pub capability_rows: crate::boot_m3::PreparedCapabilityRows,
}

pub(crate) struct Transaction<'w> {
    world: &'w mut World,
    writes: WriteSet,
}

pub(crate) fn transact<R>(
    world: &mut World,
    run: impl FnOnce(&mut Transaction<'_>) -> Result<R, RejectReason>,
) -> Result<R, RejectReason> {
    capture_operation(world, |world| transact_inner(world, run))
}

fn transact_inner<R>(
    world: &mut World,
    run: impl FnOnce(&mut Transaction<'_>) -> Result<R, RejectReason>,
) -> Result<R, RejectReason> {
    let mut tx = Transaction {
        world,
        writes: WriteSet::default(),
    };
    let result = run(&mut tx)?;
    tx.commit()?;
    Ok(result)
}

impl Transaction<'_> {
    pub(crate) fn allocate_unit_id(&mut self) -> Result<u32, RejectReason> {
        let id = self.writes.next_unit_id.unwrap_or_else(|| {
            self.world
                .get_resource::<IdAllocators>()
                .map_or(1, |ids| ids.next_unit)
        });
        self.writes.next_unit_id = Some(id.checked_add(1).ok_or(RejectReason::IdExhausted)?);
        Ok(id)
    }
    pub(crate) fn stage_spawn(&mut self, spawn: SpawnWrite) {
        self.writes.spawned.push(spawn);
    }
    pub(crate) fn catalog(&self) -> Result<&crate::bootstrap::InstalledCatalog, RejectReason> {
        self.world
            .get_resource::<crate::bootstrap::InstalledCatalog>()
            .ok_or(RejectReason::UnknownRecipe)
    }

    pub(crate) fn queues(&self) -> ProductionQueues {
        self.writes.queues.clone().unwrap_or_else(|| {
            self.world
                .get_resource::<ProductionQueues>()
                .cloned()
                .unwrap_or_default()
        })
    }

    pub(crate) fn stage_queues(&mut self, queues: ProductionQueues) {
        self.writes.queues = Some(queues);
    }

    pub(crate) fn allocate_job_id(&mut self) -> Result<JobId, RejectReason> {
        let next = self.writes.next_job_id.unwrap_or_else(|| {
            self.world
                .get_resource::<IdAllocators>()
                .map_or(1, |ids| ids.next_job)
        });
        self.writes.next_job_id = Some(next.checked_add(1).ok_or(RejectReason::IdExhausted)?);
        Ok(JobId(next))
    }

    pub(crate) fn stage_event(
        &mut self,
        owner_slot: u8,
        kind: EconomyEventKind,
    ) -> Result<(), RejectReason> {
        let ordinal = match self.writes.next_event_ordinal {
            Some(value) => value,
            None => self
                .world
                .get_resource::<NextEventOrdinal>()
                .and_then(|next| next.0)
                .ok_or(RejectReason::ArithmeticOverflow)?,
        };
        self.writes.next_event_ordinal = Some(
            ordinal
                .checked_add(1)
                .ok_or(RejectReason::ArithmeticOverflow)?,
        );
        self.writes.events.push((owner_slot, kind));
        Ok(())
    }
    pub(crate) fn account(&self, endpoint: Endpoint, resource: &ContentId) -> Account {
        if let Some(account) = self.writes.accounts.get(&(endpoint, resource.clone())) {
            return *account;
        }
        let Some(ledger) = self.world.get_resource::<EconomyLedger>() else {
            return Account::default();
        };
        match endpoint {
            Endpoint::Slot(slot) => ledger
                .balances
                .get(&(slot, resource.clone()))
                .copied()
                .unwrap_or_default(),
            Endpoint::Inventory(unit) => ledger
                .inventories
                .get(&unit)
                .and_then(|i| i.items.get(resource))
                .copied()
                .unwrap_or_default(),
            Endpoint::Output(unit) => ledger
                .output_inventories
                .get(&unit)
                .and_then(|i| i.items.get(resource))
                .copied()
                .unwrap_or_default(),
        }
    }

    pub(crate) fn inventory(&self, endpoint: Endpoint) -> Result<&Inventory, RejectReason> {
        let ledger = self
            .world
            .get_resource::<EconomyLedger>()
            .ok_or(RejectReason::IntegrityViolation)?;
        match endpoint {
            Endpoint::Slot(_) => Err(RejectReason::UnknownInventory),
            Endpoint::Inventory(unit) => ledger
                .inventories
                .get(&unit)
                .ok_or(RejectReason::IntegrityViolation),
            Endpoint::Output(unit) => ledger
                .output_inventories
                .get(&unit)
                .ok_or(RejectReason::IntegrityViolation),
        }
    }

    pub(crate) fn inventory_legacy(&self, unit: u32) -> Result<&Inventory, RejectReason> {
        self.inventory_endpoint_legacy(Endpoint::Inventory(unit))
    }

    fn inventory_endpoint_legacy(&self, endpoint: Endpoint) -> Result<&Inventory, RejectReason> {
        let ledger = self
            .world
            .get_resource::<EconomyLedger>()
            .ok_or(match endpoint {
                Endpoint::Output(_) => RejectReason::IntegrityViolation,
                _ => RejectReason::UnknownInventory,
            })?;
        match endpoint {
            Endpoint::Slot(_) => Err(RejectReason::UnknownInventory),
            Endpoint::Inventory(unit) => ledger
                .inventories
                .get(&unit)
                .ok_or(RejectReason::UnknownInventory),
            Endpoint::Output(unit) => ledger
                .output_inventories
                .get(&unit)
                .ok_or(RejectReason::IntegrityViolation),
        }
    }

    pub(crate) fn inventory_used(&self, endpoint: Endpoint) -> Result<u128, RejectReason> {
        self.inventory_used_with(endpoint, false)
    }

    pub(crate) fn inventory_used_legacy(&self, unit: u32) -> Result<u128, RejectReason> {
        self.inventory_used_endpoint_legacy(Endpoint::Inventory(unit))
    }

    fn inventory_used_endpoint_legacy(&self, endpoint: Endpoint) -> Result<u128, RejectReason> {
        self.inventory_used_with(endpoint, true)
    }

    fn inventory_used_with(&self, endpoint: Endpoint, legacy: bool) -> Result<u128, RejectReason> {
        let inventory = if legacy {
            self.inventory_endpoint_legacy(endpoint)?
        } else {
            self.inventory(endpoint)?
        };
        let resources = inventory
            .items
            .keys()
            .cloned()
            .chain(
                self.writes
                    .accounts
                    .keys()
                    .filter(|(candidate, _)| *candidate == endpoint)
                    .map(|(_, resource)| resource.clone()),
            )
            .collect::<BTreeSet<_>>();
        resources.into_iter().try_fold(0_u128, |total, resource| {
            total
                .checked_add(u128::from(self.account(endpoint, &resource).on_hand))
                .ok_or(RejectReason::ArithmeticOverflow)
        })
    }

    pub(crate) fn validate_endpoint(
        &self,
        endpoint: Endpoint,
        resource: &ContentId,
    ) -> Result<(), RejectReason> {
        self.validate_endpoint_with(endpoint, resource, false)
    }

    pub(crate) fn validate_endpoint_legacy(
        &self,
        endpoint: Endpoint,
        resource: &ContentId,
    ) -> Result<(), RejectReason> {
        self.validate_endpoint_with(endpoint, resource, true)
    }

    fn validate_endpoint_with(
        &self,
        endpoint: Endpoint,
        resource: &ContentId,
        legacy: bool,
    ) -> Result<(), RejectReason> {
        match endpoint {
            Endpoint::Slot(slot) => {
                if !self
                    .world
                    .get_resource::<Slots>()
                    .is_some_and(|slots| slots.contains(SlotId(slot)))
                {
                    return Err(RejectReason::UnknownBalance);
                }
                let exists = self
                    .world
                    .get_resource::<EconomyLedger>()
                    .is_some_and(|ledger| ledger.balances.contains_key(&(slot, resource.clone())));
                if !exists {
                    return Err(RejectReason::UnknownBalance);
                }
            }
            Endpoint::Inventory(unit) => {
                let inventory = if legacy {
                    self.inventory_endpoint_legacy(Endpoint::Inventory(unit))?
                } else {
                    self.inventory(Endpoint::Inventory(unit))?
                };
                if !inventory.accepted_items.contains(resource) {
                    return Err(RejectReason::IncompatibleItem);
                }
            }
            Endpoint::Output(unit) => {
                let inventory = if legacy {
                    self.inventory_endpoint_legacy(Endpoint::Output(unit))?
                } else {
                    self.inventory(Endpoint::Output(unit))?
                };
                if !inventory.accepted_items.contains(resource) {
                    return Err(RejectReason::IncompatibleItem);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn stage_account(
        &mut self,
        endpoint: Endpoint,
        resource: ContentId,
        account: Account,
    ) {
        self.writes.accounts.insert((endpoint, resource), account);
    }

    fn commit(self) -> Result<(), RejectReason> {
        self.preflight()?;
        let Transaction { world, mut writes } = self;
        if world.get_resource::<EconomyLedger>().is_none() {
            world.insert_resource(EconomyLedger::default());
        }
        transaction_m3::apply_overlays(world, &mut writes);
        {
            let mut ledger = world.resource_mut::<EconomyLedger>();
            for ((endpoint, resource), account) in writes.accounts {
                match endpoint {
                    Endpoint::Slot(slot) => {
                        ledger.balances.insert((slot, resource), account);
                    }
                    Endpoint::Inventory(unit) => {
                        if let Some(inventory) = ledger.inventories.get_mut(&unit) {
                            inventory.items.insert(resource, account);
                        }
                    }
                    Endpoint::Output(unit) => {
                        ledger
                            .output_inventories
                            .get_mut(&unit)
                            .expect("preflighted output inventory")
                            .items
                            .insert(resource, account);
                    }
                }
            }
            ledger.reservation_claims.extend(writes.claims);
            for removal in &writes.removed {
                ledger.inventories.remove(&removal.unit);
                ledger.output_inventories.remove(&removal.unit);
            }
        }
        if let Some(queues) = writes.queues {
            world.insert_resource(queues);
        }
        if let Some(next_job_id) = writes.next_job_id {
            world.resource_mut::<IdAllocators>().next_job = next_job_id;
        }
        if let Some(next_unit_id) = writes.next_unit_id {
            world.resource_mut::<IdAllocators>().next_unit = next_unit_id;
        }
        for spawn in writes.spawned {
            world.spawn((
                UnitId(spawn.id),
                crate::world::UnitType(spawn.unit_type),
                Owner(SlotId(spawn.owner_slot)),
                crate::world::Position(spawn.position),
            ));
            if let Some(inventory) = spawn.inventory {
                world
                    .resource_mut::<EconomyLedger>()
                    .inventories
                    .insert(spawn.id, inventory);
            }
            if spawn.gatherer {
                world.resource_mut::<EconomyOrders>().0.insert(
                    spawn.id,
                    crate::economy::GatherOrder {
                        node: None,
                        deposit: None,
                        receiver: None,
                        stage: crate::economy::gather::GatherStage::Idle,
                        carry: crate::economy::gather::RateCarry::default(),
                    },
                );
            }
            if spawn.producer {
                world
                    .resource_mut::<ProductionQueues>()
                    .0
                    .insert(spawn.id, Vec::new());
            }
            crate::boot_m3::apply_capability_rows(world, spawn.id, spawn.capability_rows);
            world
                .resource_mut::<crate::economy::events::SpawnedThisTick>()
                .0
                .push(spawn.state);
        }
        for removal in writes.removed {
            if let Some(mut queues) = world.get_resource_mut::<ProductionQueues>() {
                queues.0.remove(&removal.unit);
            }
            if let Some(mut orders) = world.get_resource_mut::<EconomyOrders>() {
                orders.0.remove(&removal.unit);
            }
            assert!(
                world.despawn(removal.entity),
                "unit entity was preflighted before commit"
            );
            world.resource_mut::<RemovedThisTick>().0.push(removal.unit);
            world.init_resource::<super::views_m3::RemovedHistory>();
            world
                .resource_mut::<super::views_m3::RemovedHistory>()
                .0
                .insert(removal.unit);
        }
        if let Some(next_event_ordinal) = writes.next_event_ordinal {
            let tick = world.resource::<crate::world::Tick>().0;
            let first = world
                .resource::<NextEventOrdinal>()
                .0
                .expect("event ordinal was preflighted before transaction commit");
            {
                let mut pending = world.resource_mut::<PendingEconomyEvents>();
                for (offset, (owner_slot, kind)) in writes.events.into_iter().enumerate() {
                    pending.0.push(EconomyEvent {
                        tick,
                        ordinal: first + offset as u64,
                        owner_slot,
                        kind,
                    });
                }
            }
            world.resource_mut::<NextEventOrdinal>().0 = Some(next_event_ordinal);
        }
        Ok(())
    }
}

/// Transfer items between inventories owned by the caller after checking filters and capacity.
pub fn inventory_transfer(
    world: &mut World,
    slot: SlotId,
    from: u32,
    to: u32,
    item: ContentId,
    amount: u64,
) -> Result<(), RejectReason> {
    capture_operation(world, |world| {
        inventory_transfer_inner(world, slot, from, to, item, amount)
    })
}

fn inventory_transfer_inner(
    world: &mut World,
    slot: SlotId,
    from: u32,
    to: u32,
    item: ContentId,
    amount: u64,
) -> Result<(), RejectReason> {
    if amount == 0 {
        return Err(RejectReason::InvalidAmount);
    }
    let from_owner = inventory_owner(world, from)?;
    if from_owner != slot {
        return Err(RejectReason::NotOwner);
    }
    let to_owner = inventory_owner(world, to)?;
    if to_owner != slot {
        return Err(RejectReason::NotOwner);
    }
    if from == to {
        return Err(RejectReason::InvalidAmount);
    }
    transact(world, |tx| {
        tx.validate_endpoint_legacy(Endpoint::Inventory(from), &item)?;
        tx.validate_endpoint_legacy(Endpoint::Inventory(to), &item)?;
        let source = tx.account(Endpoint::Inventory(from), &item);
        let available = source
            .on_hand
            .checked_sub(source.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if available < amount {
            return Err(RejectReason::InsufficientItems);
        }
        let destination = tx.account(Endpoint::Inventory(to), &item);
        let used = tx.inventory_used_legacy(to)?;
        let new_used = used
            .checked_add(u128::from(amount))
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if new_used > u128::from(tx.inventory_legacy(to)?.capacity) {
            return Err(RejectReason::RefundDestinationFull);
        }
        let new_source = source
            .on_hand
            .checked_sub(amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        let new_destination = destination
            .on_hand
            .checked_add(amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        tx.stage_account(
            Endpoint::Inventory(from),
            item.clone(),
            Account {
                on_hand: new_source,
                ..source
            },
        );
        tx.stage_account(
            Endpoint::Inventory(to),
            item,
            Account {
                on_hand: new_destination,
                ..destination
            },
        );
        Ok(())
    })
}

fn inventory_owner(world: &World, id: u32) -> Result<SlotId, RejectReason> {
    let owner = world
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|unit| unit.0 == id))
                .then(|| entity.get::<Owner>().map(|owner| owner.0))?
        })
        .ok_or(RejectReason::UnknownInventory)?;
    Ok(owner)
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn reserve_costs(
    tx: &mut Transaction<'_>,
    job_id: JobId,
    lines: &[ReservationLine],
) -> Result<Reservation, RejectReason> {
    let canonical = validate_lines(lines)?;
    if tx
        .world
        .get_resource::<EconomyLedger>()
        .is_some_and(|ledger| ledger.reservation_claims.contains_key(&job_id))
        || tx.writes.claims.contains_key(&job_id)
    {
        return Err(RejectReason::UnknownJob);
    }
    let mut updates = BTreeMap::new();
    for line in &canonical {
        tx.validate_endpoint_legacy(line.endpoint, &line.resource)?;
        let key = (line.endpoint, line.resource.clone());
        let old = tx.account(line.endpoint, &line.resource);
        if old
            .on_hand
            .checked_sub(old.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?
            < line.amount
        {
            return Err(match line.endpoint {
                Endpoint::Slot(_) => RejectReason::InsufficientBalance,
                Endpoint::Inventory(_) => RejectReason::InsufficientItems,
                Endpoint::Output(_) => RejectReason::InsufficientItems,
            });
        }
        let reserved = old
            .reserved
            .checked_add(line.amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if reserved > old.on_hand {
            return Err(RejectReason::InsufficientItems);
        }
        updates.insert(key, Account { reserved, ..old });
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    let reservation = Reservation {
        job_id,
        lines: canonical,
    };
    tx.writes
        .claims
        .insert(job_id, ReservationClaim::Active(reservation.clone()));
    Ok(reservation)
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn charge_costs(
    tx: &mut Transaction<'_>,
    lines: &[ReservationLine],
) -> Result<(), RejectReason> {
    let canonical = validate_lines(lines)?;
    let mut updates = BTreeMap::new();
    for line in canonical {
        tx.validate_endpoint_legacy(line.endpoint, &line.resource)?;
        let old = tx.account(line.endpoint, &line.resource);
        if old
            .on_hand
            .checked_sub(old.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?
            < line.amount
        {
            return Err(match line.endpoint {
                Endpoint::Slot(_) => RejectReason::InsufficientBalance,
                Endpoint::Inventory(_) => RejectReason::InsufficientItems,
                Endpoint::Output(_) => RejectReason::InsufficientItems,
            });
        }
        updates.insert(
            (line.endpoint, line.resource),
            Account {
                on_hand: old
                    .on_hand
                    .checked_sub(line.amount)
                    .ok_or(RejectReason::ArithmeticOverflow)?,
                ..old
            },
        );
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    Ok(())
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn consume_reservation(
    tx: &mut Transaction<'_>,
    reservation: &Reservation,
) -> Result<(), RejectReason> {
    close_reservation(tx, reservation, true)
}
// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn release_reservation(
    tx: &mut Transaction<'_>,
    reservation: &Reservation,
) -> Result<(), RejectReason> {
    close_reservation(tx, reservation, false)
}
fn close_reservation(
    tx: &mut Transaction<'_>,
    reservation: &Reservation,
    consume: bool,
) -> Result<(), RejectReason> {
    let canonical = validate_lines(&reservation.lines)?;
    let claim = tx
        .writes
        .claims
        .get(&reservation.job_id)
        .or_else(|| {
            tx.world
                .get_resource::<EconomyLedger>()
                .and_then(|ledger| ledger.reservation_claims.get(&reservation.job_id))
        })
        .ok_or(RejectReason::UnknownJob)?;
    if claim
        != &ReservationClaim::Active(Reservation {
            job_id: reservation.job_id,
            lines: canonical.clone(),
        })
    {
        return Err(RejectReason::UnknownJob);
    }
    let mut updates = BTreeMap::new();
    for line in canonical {
        let old = tx.account(line.endpoint, &line.resource);
        let reserved = old
            .reserved
            .checked_sub(line.amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        let on_hand = if consume {
            old.on_hand
                .checked_sub(line.amount)
                .ok_or(RejectReason::ArithmeticOverflow)?
        } else {
            old.on_hand
        };
        updates.insert(
            (line.endpoint, line.resource),
            Account { on_hand, reserved },
        );
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    tx.writes
        .claims
        .insert(reservation.job_id, ReservationClaim::Closed);
    Ok(())
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn refund_charged(
    tx: &mut Transaction<'_>,
    lines: &[ReservationLine],
    numerator: u64,
    denominator: u64,
) -> Result<(), RejectReason> {
    if denominator == 0 || numerator > denominator {
        return Err(RejectReason::InvalidAmount);
    }
    let canonical = validate_lines(lines)?;
    let mut updates = BTreeMap::new();
    let mut inventory_credits = BTreeMap::<Endpoint, u128>::new();
    for line in canonical {
        tx.validate_endpoint_legacy(line.endpoint, &line.resource)?;
        let refund = u64::try_from(
            u128::from(line.amount)
                .checked_mul(u128::from(numerator))
                .ok_or(RejectReason::ArithmeticOverflow)?
                / u128::from(denominator),
        )
        .map_err(|_| RejectReason::ArithmeticOverflow)?;
        let old = tx.account(line.endpoint, &line.resource);
        let on_hand = old
            .on_hand
            .checked_add(refund)
            .ok_or(RejectReason::RefundDestinationOverflow)?;
        if matches!(line.endpoint, Endpoint::Inventory(_) | Endpoint::Output(_)) {
            let total = inventory_credits.entry(line.endpoint).or_default();
            *total = total
                .checked_add(u128::from(refund))
                .ok_or(RejectReason::ArithmeticOverflow)?;
        }
        updates.insert((line.endpoint, line.resource), Account { on_hand, ..old });
    }
    for (endpoint, credit) in inventory_credits {
        let used = tx.inventory_used_endpoint_legacy(endpoint)?;
        if used
            .checked_add(credit)
            .ok_or(RejectReason::ArithmeticOverflow)?
            > u128::from(tx.inventory_endpoint_legacy(endpoint)?.capacity)
        {
            return Err(RejectReason::RefundDestinationFull);
        }
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    Ok(())
}

fn validate_lines(lines: &[ReservationLine]) -> Result<Vec<ReservationLine>, RejectReason> {
    let mut sorted = lines.to_vec();
    if sorted.iter().any(|line| line.amount == 0) {
        return Err(RejectReason::InvalidAmount);
    }
    sorted.sort_by(|a, b| (a.endpoint, &a.resource).cmp(&(b.endpoint, &b.resource)));
    if sorted
        .windows(2)
        .any(|pair| pair[0].endpoint == pair[1].endpoint && pair[0].resource == pair[1].resource)
    {
        return Err(RejectReason::InvalidAmount);
    }
    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slots::Slots;

    pub(super) fn seeded_world() -> World {
        let mut world = World::new();
        world.insert_resource(Slots::new(1));
        world.insert_resource(crate::registry::UnitTypes::default());
        let mut ledger = EconomyLedger::default();
        ledger.balances.insert(
            (0, ContentId::parse("probe:ore").unwrap()),
            Account {
                on_hand: 10,
                reserved: 0,
            },
        );
        ledger.balances.insert(
            (0, ContentId::parse("probe:wood").unwrap()),
            Account {
                on_hand: 1,
                reserved: 0,
            },
        );
        world.insert_resource(ledger);
        world
    }

    pub(super) fn line(resource: &str, amount: u64) -> ReservationLine {
        ReservationLine {
            endpoint: Endpoint::Slot(0),
            resource: ContentId::parse(resource).unwrap(),
            amount,
        }
    }

    #[test]
    fn mixed_cost_reservation_rolls_back_all_lines() {
        let mut world = seeded_world();
        let before = world.resource::<EconomyLedger>().clone();
        let result = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(1), &[line("probe:ore", 5), line("probe:wood", 2)])
        });
        assert_eq!(result, Err(RejectReason::InsufficientBalance));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn mixed_resources_on_one_endpoint_are_valid() {
        let mut world = seeded_world();
        let reservation = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(1), &[line("probe:wood", 1), line("probe:ore", 4)])
        })
        .unwrap();
        assert_eq!(reservation.lines.len(), 2);
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:ore").unwrap())]
                .reserved,
            4
        );
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:wood").unwrap())]
                .reserved,
            1
        );
    }

    #[test]
    fn reservation_contention_and_stale_replay_are_rejected() {
        let mut world = seeded_world();
        let first = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(1), &[line("probe:ore", 8)])
        })
        .unwrap();
        let before_contender = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            transact(&mut world, |tx| reserve_costs(
                tx,
                JobId(2),
                &[line("probe:ore", 8)]
            )),
            Err(RejectReason::InsufficientBalance)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_contender);
        transact(&mut world, |tx| release_reservation(tx, &first)).unwrap();
        let before_replay = world.resource::<EconomyLedger>().clone();
        let cross_job = Reservation {
            job_id: JobId(2),
            lines: first.lines.clone(),
        };
        assert_eq!(
            transact(&mut world, |tx| consume_reservation(tx, &cross_job)),
            Err(RejectReason::UnknownJob)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_replay);
        assert_eq!(
            transact(&mut world, |tx| release_reservation(tx, &first)),
            Err(RejectReason::UnknownJob)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_replay);
        assert_eq!(
            transact(&mut world, |tx| reserve_costs(
                tx,
                JobId(1),
                &[line("probe:ore", 1)]
            )),
            Err(RejectReason::UnknownJob)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_replay);
    }

    #[test]
    fn checked_max_math_and_duplicate_lines_never_commit() {
        let mut world = seeded_world();
        let before = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            transact(&mut world, |tx| reserve_costs(
                tx,
                JobId(1),
                &[line("probe:ore", 1), line("probe:ore", 1)]
            )),
            Err(RejectReason::InvalidAmount)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
        assert_eq!(
            transact(&mut world, |tx| charge_costs(
                tx,
                &[line("probe:ore", u64::MAX)]
            )),
            Err(RejectReason::InsufficientBalance)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn charge_costs_preflights_all_resources_before_debiting() {
        let mut world = seeded_world();
        transact(&mut world, |tx| {
            charge_costs(tx, &[line("probe:ore", 3), line("probe:wood", 1)])
        })
        .unwrap();
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:ore").unwrap())]
                .on_hand,
            7
        );
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:wood").unwrap())]
                .on_hand,
            0
        );
    }

    #[test]
    fn reservation_consumption_is_atomic_and_closes_claim() {
        let mut world = seeded_world();
        let token = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(44), &[line("probe:ore", 6)])
        })
        .unwrap();
        transact(&mut world, |tx| consume_reservation(tx, &token)).unwrap();
        let account = world.resource::<EconomyLedger>().balances
            [&(0, ContentId::parse("probe:ore").unwrap())];
        assert_eq!(
            account,
            Account {
                on_hand: 4,
                reserved: 0
            }
        );
        assert_eq!(
            world.resource::<EconomyLedger>().reservation_claims[&JobId(44)],
            ReservationClaim::Closed
        );
        assert_eq!(
            transact(&mut world, |tx| consume_reservation(tx, &token)),
            Err(RejectReason::UnknownJob)
        );
    }

    #[test]
    fn audit_distinguishes_reservation_claim_payloads() {
        let mut world = seeded_world();
        let token = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(7), &[line("probe:ore", 3)])
        })
        .unwrap();
        let before = crate::economy::views::audit_state(&mut world);
        assert_eq!(
            before.ledger_claims,
            vec![crate::economy::views::ReservationClaimView::Active {
                map_job_id: 7,
                token_job_id: 7,
                lines: vec![crate::economy::views::ReservationLineView {
                    endpoint: crate::economy::views::ReservationEndpointView::Slot(0),
                    resource: ContentId::parse("probe:ore").unwrap(),
                    amount: 3,
                }],
            }],
        );
        let mut changed = token;
        changed.lines[0].amount = 2;
        world
            .resource_mut::<EconomyLedger>()
            .reservation_claims
            .insert(JobId(7), ReservationClaim::Active(changed));
        let after = crate::economy::views::audit_state(&mut world);
        assert_ne!(
            after, before,
            "audit must include authoritative reservation lines"
        );
    }

    #[test]
    fn failed_outer_transaction_preserves_complete_claim_audit() {
        let mut world = seeded_world();
        transact(&mut world, |tx| {
            reserve_costs(tx, JobId(7), &[line("probe:ore", 3)])
        })
        .unwrap();
        let before = crate::economy::views::audit_state(&mut world);
        let result: Result<(), RejectReason> = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(8), &[line("probe:wood", 1)])?;
            Err(RejectReason::UnknownJob)
        });
        assert_eq!(result, Err(RejectReason::UnknownJob));
        assert_eq!(crate::economy::views::audit_state(&mut world), before);
    }

    #[test]
    fn refund_uses_checked_u128_and_leaves_overflow_unchanged() {
        let mut world = seeded_world();
        world.resource_mut::<EconomyLedger>().balances.insert(
            (0, ContentId::parse("probe:ore").unwrap()),
            Account {
                on_hand: u64::MAX,
                reserved: 0,
            },
        );
        let before = world.resource::<EconomyLedger>().clone();
        let result = transact(&mut world, |tx| {
            refund_charged(tx, &[line("probe:ore", u64::MAX)], 2, 1)
        });
        assert_eq!(result, Err(RejectReason::InvalidAmount));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
        let result = transact(&mut world, |tx| {
            refund_charged(tx, &[line("probe:ore", u64::MAX)], 1, 1)
        });
        assert_eq!(result, Err(RejectReason::RefundDestinationOverflow));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }
}

#[cfg(test)]
mod inventory_tests {
    use super::tests::{line, seeded_world};
    use super::*;
    use crate::{Owner, UnitId, economy::ledger::Inventory, slots::Slots};
    use std::collections::BTreeMap;

    pub(super) fn inventory_world() -> World {
        let timber = ContentId::parse("probe:timber").unwrap();
        let mut world = World::new();
        world.insert_resource(Slots::new(1));
        world.spawn((UnitId(1), Owner(SlotId(0))));
        world.spawn((UnitId(2), Owner(SlotId(0))));
        let mut ledger = EconomyLedger::default();
        let mut source = Inventory {
            capacity: 20,
            accepted_items: BTreeSet::from([timber.clone()]),
            items: BTreeMap::new(),
        };
        source.items.insert(
            timber.clone(),
            Account {
                on_hand: 10,
                reserved: 4,
            },
        );
        let destination = Inventory {
            capacity: 20,
            accepted_items: BTreeSet::from([timber.clone()]),
            items: BTreeMap::new(),
        };
        ledger.inventories.insert(1, source);
        ledger.inventories.insert(2, destination);
        world.insert_resource(ledger);
        world
    }

    #[test]
    fn transfer_cannot_spend_reserved_inventory_stock() {
        let mut world = inventory_world();
        let same_endpoint = inventory_transfer(
            &mut world,
            SlotId(0),
            1,
            1,
            ContentId::parse("probe:timber").unwrap(),
            1,
        );
        assert_eq!(same_endpoint, Err(RejectReason::InvalidAmount));
        let before = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            inventory_transfer(
                &mut world,
                SlotId(0),
                1,
                2,
                ContentId::parse("probe:timber").unwrap(),
                7
            ),
            Err(RejectReason::InsufficientItems)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
        inventory_transfer(
            &mut world,
            SlotId(0),
            1,
            2,
            ContentId::parse("probe:timber").unwrap(),
            6,
        )
        .unwrap();
        assert_eq!(
            world.resource::<EconomyLedger>().inventories[&1].items
                [&ContentId::parse("probe:timber").unwrap()],
            Account {
                on_hand: 4,
                reserved: 4
            }
        );
    }

    #[test]
    fn destination_capacity_counts_all_resources_and_rejects_overflowing_u64_sum() {
        let mut world = inventory_world();
        let timber = ContentId::parse("probe:timber").unwrap();
        let wood = ContentId::parse("probe:wood").unwrap();
        {
            let mut ledger = world.resource_mut::<EconomyLedger>();
            let destination = ledger.inventories.get_mut(&2).unwrap();
            destination.capacity = u64::MAX;
            destination.accepted_items.insert(wood.clone());
            destination.items.insert(
                wood,
                Account {
                    on_hand: u64::MAX,
                    reserved: 0,
                },
            );
        }
        let before = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            inventory_transfer(&mut world, SlotId(0), 1, 2, timber, 1),
            Err(RejectReason::RefundDestinationFull)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn refund_can_credit_u64_max_after_checked_u128_multiplication() {
        let mut world = seeded_world();
        world.resource_mut::<EconomyLedger>().balances.insert(
            (0, ContentId::parse("probe:ore").unwrap()),
            Account::default(),
        );
        transact(&mut world, |tx| {
            refund_charged(tx, &[line("probe:ore", u64::MAX)], u64::MAX, u64::MAX)
        })
        .unwrap();
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:ore").unwrap())]
                .on_hand,
            u64::MAX
        );
    }
}

#[cfg(test)]
mod refund_tests {
    use super::inventory_tests::inventory_world;
    use super::*;

    #[test]
    fn refund_preflights_combined_inventory_credit_across_resources() {
        let mut world = inventory_world();
        let timber = ContentId::parse("probe:timber").unwrap();
        let wood = ContentId::parse("probe:wood").unwrap();
        {
            let mut ledger = world.resource_mut::<EconomyLedger>();
            let destination = ledger.inventories.get_mut(&2).unwrap();
            destination.capacity = 10;
            destination.accepted_items.insert(wood.clone());
            destination.items.insert(
                wood.clone(),
                Account {
                    on_hand: 4,
                    reserved: 0,
                },
            );
        }
        let before = world.resource::<EconomyLedger>().clone();
        let lines = [
            ReservationLine {
                endpoint: Endpoint::Inventory(2),
                resource: timber,
                amount: 4,
            },
            ReservationLine {
                endpoint: Endpoint::Inventory(2),
                resource: wood,
                amount: 4,
            },
        ];
        assert_eq!(
            transact(&mut world, |tx| refund_charged(tx, &lines, 1, 1)),
            Err(RejectReason::RefundDestinationFull)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }
}

#[cfg(test)]
mod staged_refund_tests {
    use super::inventory_tests::inventory_world;
    use super::*;

    fn prepare_empty_tight_inventory(world: &mut World) {
        let wood = ContentId::parse("probe:wood").unwrap();
        let mut ledger = world.resource_mut::<EconomyLedger>();
        let destination = ledger.inventories.get_mut(&2).unwrap();
        destination.capacity = 5;
        destination.accepted_items.insert(wood);
    }

    #[test]
    fn second_refund_to_new_same_resource_rejects_and_outer_transaction_rolls_back() {
        let mut world = inventory_world();
        prepare_empty_tight_inventory(&mut world);
        let before = world.resource::<EconomyLedger>().clone();
        let line = ReservationLine {
            endpoint: Endpoint::Inventory(2),
            resource: ContentId::parse("probe:timber").unwrap(),
            amount: 4,
        };
        let result = transact(&mut world, |tx| {
            refund_charged(tx, std::slice::from_ref(&line), 1, 1)?;
            refund_charged(tx, std::slice::from_ref(&line), 1, 1)?;
            Ok(())
        });
        assert_eq!(result, Err(RejectReason::RefundDestinationFull));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn second_refund_to_different_new_resource_rejects_but_first_can_commit() {
        let mut world = inventory_world();
        prepare_empty_tight_inventory(&mut world);
        let timber = ReservationLine {
            endpoint: Endpoint::Inventory(2),
            resource: ContentId::parse("probe:timber").unwrap(),
            amount: 4,
        };
        let wood = ReservationLine {
            endpoint: Endpoint::Inventory(2),
            resource: ContentId::parse("probe:wood").unwrap(),
            amount: 4,
        };
        transact(&mut world, |tx| {
            refund_charged(tx, std::slice::from_ref(&timber), 1, 1)?;
            assert_eq!(
                refund_charged(tx, std::slice::from_ref(&wood), 1, 1),
                Err(RejectReason::RefundDestinationFull)
            );
            Ok(())
        })
        .unwrap();
        let inventory = &world.resource::<EconomyLedger>().inventories[&2];
        assert_eq!(inventory.items[&timber.resource].on_hand, 4);
        assert!(!inventory.items.contains_key(&wood.resource));
    }
}

#[cfg(test)]
mod audit_capture_tests {
    use super::{AuditTrail, capture_operation};
    use bevy_ecs::prelude::{Resource, World};

    #[derive(Resource)]
    struct Marker(u32);

    #[test]
    fn disabled_capture_runs_directly_without_creating_audit_records() {
        let mut world = World::new();
        world.insert_resource(AuditTrail::default());
        let result = capture_operation(&mut world, |world| {
            world.insert_resource(Marker(7));
            11
        });
        assert_eq!(result, 11);
        assert_eq!(world.resource::<Marker>().0, 7);
        let trail = world.resource::<AuditTrail>();
        assert!(!trail.enabled);
        assert!(trail.operations.is_empty());
        assert!(trail.commands.is_empty());
    }

    #[test]
    fn diagnostic_id_exhaustion_invalidates_evidence_without_changing_gameplay() {
        let mut world = World::new();
        world.insert_resource(AuditTrail::default());
        {
            let mut trail = world.resource_mut::<AuditTrail>();
            trail.enable();
            trail.next_operation_id = u64::MAX;
        }
        let result = capture_operation(&mut world, |world| {
            world.insert_resource(Marker(19));
            23
        });
        assert_eq!(result, 23);
        assert_eq!(world.resource::<Marker>().0, 19);
        let trail = world.resource::<AuditTrail>();
        assert!(!trail.valid);
        assert!(trail.operations.is_empty());
    }
}

#[cfg(test)]
mod raw_audit_capture_tests {
    use super::*;
    use crate::{
        economy::{
            events::{NextEventOrdinal, PendingEconomyEvents},
            ledger::{Endpoint, Reservation, ReservationLine},
            production::{JobStage, ProductionJob, ProductionQueues},
            views::{ReservationEndpointView, ReservationLineView, audit_state},
        },
        world::Tick,
    };
    use bevy_ecs::world::World;

    fn line(endpoint: Endpoint, resource: &str, amount: u64) -> ReservationLine {
        ReservationLine {
            endpoint,
            resource: ContentId::parse(resource).unwrap(),
            amount,
        }
    }

    #[test]
    fn raw_queue_capture_preserves_keys_fifo_job_fields_and_tokens() {
        let mut queues = ProductionQueues::default();
        queues.0.insert(
            7,
            vec![
                ProductionJob {
                    id: JobId(11),
                    recipe: ContentId::parse("probe:unit").unwrap(),
                    stage: JobStage::Running,
                    elapsed_ticks: 4,
                    duration_ticks: 19,
                    charged: vec![line(Endpoint::Slot(3), "probe:ore", 7)],
                    reservation: Some(Reservation {
                        job_id: JobId(99),
                        lines: vec![line(Endpoint::Inventory(42), "probe:wood", 5)],
                    }),
                },
                ProductionJob {
                    id: JobId(12),
                    recipe: ContentId::parse("probe:other").unwrap(),
                    stage: JobStage::OutputBlocked,
                    elapsed_ticks: 19,
                    duration_ticks: 19,
                    charged: vec![],
                    reservation: Some(Reservation {
                        job_id: JobId(12),
                        lines: vec![],
                    }),
                },
            ],
        );
        queues.0.insert(8, vec![]);
        queues.0.insert(
            900,
            vec![ProductionJob {
                id: JobId(13),
                recipe: ContentId::parse("probe:orphan").unwrap(),
                stage: JobStage::Queued,
                elapsed_ticks: 0,
                duration_ticks: 3,
                charged: vec![],
                reservation: None,
            }],
        );
        let mut world = World::new();
        world.insert_resource(crate::registry::UnitTypes::default());
        world.insert_resource(queues);
        let captured = audit_state(&mut world);
        assert_eq!(
            captured
                .raw_queues
                .iter()
                .map(|q| q.producer)
                .collect::<Vec<_>>(),
            [7, 8, 900]
        );
        assert!(captured.raw_queues[1].jobs.is_empty());
        let jobs = &captured.raw_queues[0].jobs;
        assert_eq!(jobs.iter().map(|j| j.id).collect::<Vec<_>>(), [11, 12]);
        assert_eq!(jobs[0].recipe, ContentId::parse("probe:unit").unwrap());
        assert_eq!(jobs[0].stage, JobStage::Running);
        assert_eq!((jobs[0].elapsed_ticks, jobs[0].duration_ticks), (4, 19));
        assert_eq!(
            jobs[0].charged,
            vec![ReservationLineView {
                endpoint: ReservationEndpointView::Slot(3),
                resource: ContentId::parse("probe:ore").unwrap(),
                amount: 7
            }]
        );
        assert_eq!(jobs[0].reservation.as_ref().unwrap().job_id, 99);
        assert_eq!(
            jobs[0].reservation.as_ref().unwrap().lines,
            vec![ReservationLineView {
                endpoint: ReservationEndpointView::Inventory(42),
                resource: ContentId::parse("probe:wood").unwrap(),
                amount: 5
            }]
        );
        assert_eq!(jobs[1].reservation.as_ref().unwrap().job_id, 12);
        assert!(jobs[1].reservation.as_ref().unwrap().lines.is_empty());
    }

    #[test]
    fn rejected_capture_detects_hidden_queue_endpoint_and_token_mutation() {
        let mut queues = ProductionQueues::default();
        queues.0.insert(
            7,
            vec![ProductionJob {
                id: JobId(1),
                recipe: ContentId::parse("probe:unit").unwrap(),
                stage: JobStage::Queued,
                elapsed_ticks: 0,
                duration_ticks: 5,
                charged: vec![line(Endpoint::Slot(0), "probe:ore", 2)],
                reservation: Some(Reservation {
                    job_id: JobId(1),
                    lines: vec![line(Endpoint::Slot(0), "probe:ore", 3)],
                }),
            }],
        );
        let mut world = World::new();
        world.insert_resource(crate::registry::UnitTypes::default());
        world.insert_resource(queues);
        world.insert_resource(AuditTrail::default());
        world.resource_mut::<AuditTrail>().enable();
        let input = crate::command::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::command::CommandKind::Produce {
                producer_id: 7,
                recipe_id: ContentId::parse("probe:unit").unwrap(),
            },
        };
        let result = capture_command(&mut world, &input, |world| {
            let mut queues = world.resource_mut::<ProductionQueues>();
            let job = &mut queues.0.get_mut(&7).unwrap()[0];
            job.charged[0].endpoint = Endpoint::Slot(1);
            let reservation = job.reservation.as_mut().unwrap();
            reservation.job_id = JobId(99);
            reservation.lines[0].endpoint = Endpoint::Slot(1);
            crate::command::CommandResult {
                slot: input.slot,
                seq: input.seq,
                outcome: Err(RejectReason::UnknownRecipe),
                units: Default::default(),
            }
        });
        assert_eq!(result.outcome, Err(RejectReason::UnknownRecipe));
        let boundary = &world.resource::<AuditTrail>().commands[0];
        assert_eq!(boundary.result, result);
        assert_ne!(boundary.before, boundary.after);
    }

    #[test]
    fn resource_presence_distinguishes_absent_defaults_and_exhausted_cursor() {
        let mut world = World::new();
        world.insert_resource(crate::registry::UnitTypes::default());
        world.insert_resource(IdAllocators::default());
        world.insert_resource(Tick(0));
        world.insert_resource(PendingEconomyEvents::default());
        world.insert_resource(NextEventOrdinal(None));
        let present = audit_state(&mut world);
        assert!(present.resources.allocators && present.resources.tick);
        assert!(present.resources.pending_events && present.resources.event_ordinal);
        assert_eq!(present.next_event_ordinal, None);
        world.remove_resource::<IdAllocators>();
        let missing = audit_state(&mut world);
        assert_ne!(present, missing);
        assert!(!missing.resources.allocators);
        world.insert_resource(IdAllocators::default());
        world.remove_resource::<Tick>();
        let missing = audit_state(&mut world);
        assert_ne!(present, missing);
        assert!(!missing.resources.tick);
        world.insert_resource(Tick(0));
        world.remove_resource::<PendingEconomyEvents>();
        let missing = audit_state(&mut world);
        assert_ne!(present, missing);
        assert!(!missing.resources.pending_events);
        world.insert_resource(PendingEconomyEvents::default());
        world.remove_resource::<NextEventOrdinal>();
        let missing = audit_state(&mut world);
        assert_ne!(present, missing);
        assert!(!missing.resources.event_ordinal);
        assert_eq!(missing.next_event_ordinal, None);
    }
}

#[cfg(test)]
mod m3_transaction_red_tests {
    use super::*;

    fn belt_line(pieces: Vec<u32>) -> crate::economy::conveyor::BeltLine {
        let mut carry = crate::economy::gather::RateCarry::default();
        carry.take_budget(1, 20).unwrap();
        crate::economy::conveyor::BeltLine {
            def: ContentId::parse("probe:belt").unwrap(),
            owner: 0,
            slots: (1..=(pieces.len() * 2) as u16).collect(),
            slots_per_piece: 2,
            pieces,
            slots_per_second: 1,
            carry,
        }
    }

    #[test]
    fn split_line_first_middle_last_and_only_piece() {
        let line = belt_line(vec![10, 11, 12, 13]);

        let (first_left, first_right) = split_line(&line, 0);
        assert!(first_left.is_none());
        let first_right = first_right.unwrap();
        assert_eq!(first_right.pieces, vec![11, 12, 13]);
        assert_eq!(first_right.slots, vec![3, 4, 5, 6, 7, 8]);
        assert_eq!(first_right.carry, line.carry);

        let (middle_left, middle_right) = split_line(&line, 1);
        let middle_left = middle_left.unwrap();
        let middle_right = middle_right.unwrap();
        assert_eq!(middle_left.pieces, vec![10]);
        assert_eq!(middle_left.slots, vec![1, 2]);
        assert_eq!(middle_right.pieces, vec![12, 13]);
        assert_eq!(middle_right.slots, vec![5, 6, 7, 8]);
        assert_eq!(middle_left.carry, line.carry);
        assert_eq!(middle_right.carry, line.carry);

        let (last_left, last_right) = split_line(&line, 3);
        let last_left = last_left.unwrap();
        assert_eq!(last_left.pieces, vec![10, 11, 12]);
        assert_eq!(last_left.slots, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(last_left.carry, line.carry);
        assert!(last_right.is_none());

        let (only_left, only_right) = split_line(&belt_line(vec![99]), 0);
        assert!(only_left.is_none());
        assert!(only_right.is_none());
    }

    #[test]
    fn write_to_missing_output_endpoint_is_integrity_violation() {
        let mut world = World::new();
        world.insert_resource(EconomyLedger::default());
        let item = ContentId::parse("probe:item").unwrap();
        assert_eq!(
            transact(&mut world, |tx| tx
                .validate_endpoint(Endpoint::Output(404), &item)),
            Err(RejectReason::IntegrityViolation),
        );
    }
}

#[cfg(test)]
mod m3_transaction_contract_tests {
    use super::*;
    use crate::{
        economy::{
            EconomyOrders, SurfaceNodes,
            combat::{Healths, Weapons},
            conveyor::{BeltLine, Conveyors, ItemTable},
            events::{
                CommittedTick, DestroyedLine, DestroyedLocation, RemoveCause, RemovedThisTick,
            },
            extractor::Extractors,
            gather::{GatherStage, RateCarry},
            inserter::{InserterState, Inserters},
            ledger::Inventory,
            production::ProductionQueues,
            research::{Disabled, ResearchState},
        },
        ids::NodeId,
        world::{Position, Tick},
    };
    use bevy_ecs::prelude::World;
    use bevy_math::Vec2;
    use std::collections::BTreeMap;

    fn content(value: &str) -> ContentId {
        ContentId::parse(value).unwrap()
    }

    fn inventory(capacity: u64, accepted: &[ContentId]) -> Inventory {
        Inventory {
            capacity,
            accepted_items: accepted.iter().cloned().collect(),
            items: BTreeMap::new(),
        }
    }

    fn install_extraction_node(world: &mut World, item: &ContentId, remaining: u64) {
        let node_id = content("probe:deposit");
        let mut catalog = turts_mod_api::ContentCatalog {
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        catalog.nodes.insert(
            node_id.clone(),
            turts_mod_api::ResolvedNode {
                id: node_id.clone(),
                name: "Deposit".to_string(),
                item: item.clone(),
                radius: 1.0,
            },
        );
        world.insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        world.insert_resource(SurfaceNodes(vec![crate::economy::NodeRecord {
            id: NodeId(1),
            content_id: node_id,
            position: Position(Vec2::ZERO),
            remaining,
        }]));
    }

    fn removal_world() -> World {
        let unit = 7;
        let ore = content("probe:ore");
        let parts = content("probe:parts");
        let held = content("probe:held");
        let belt_item = content("probe:belt_item");
        let mut world = World::new();
        world.insert_resource(crate::slots::Slots::new(1));
        world.insert_resource(crate::ids::IdAllocators::default());
        world.insert_resource(Tick::default());
        world.insert_resource(ProductionQueues::default());
        world.insert_resource(EconomyOrders::default());
        let mut nodes = SurfaceNodes::default();
        nodes.0.push(crate::economy::NodeRecord {
            id: NodeId(1),
            content_id: ore.clone(),
            position: Position(Vec2::ZERO),
            remaining: 4,
        });
        world.insert_resource(nodes);
        world.insert_resource(RemovedThisTick::default());
        world.insert_resource(PendingEconomyEvents::default());
        world.insert_resource(NextEventOrdinal::default());

        let mut ledger = EconomyLedger::default();
        let mut main = inventory(10, &[ore.clone(), held.clone()]);
        main.items.insert(
            ore,
            Account {
                on_hand: 2,
                reserved: 0,
            },
        );
        main.items.insert(
            held,
            Account {
                on_hand: 4,
                reserved: 0,
            },
        );
        ledger.inventories.insert(unit, main);
        let mut output = inventory(10, std::slice::from_ref(&parts));
        output.items.insert(
            parts,
            Account {
                on_hand: 3,
                reserved: 0,
            },
        );
        ledger.output_inventories.insert(unit, output);
        world.insert_resource(ledger);

        let mut carry = RateCarry::default();
        carry.take_budget(1, 20).unwrap();
        let mut conveyors = Conveyors::default();
        conveyors.lines.insert(
            70,
            BeltLine {
                def: content("probe:belt"),
                owner: 0,
                pieces: vec![70, 7, 90],
                slots: vec![1, 0, 1, 1, 1, 0],
                slots_per_piece: 2,
                slots_per_second: 2,
                carry,
            },
        );
        conveyors
            .piece_line
            .extend([(70, (70, 0)), (7, (70, 1)), (90, (70, 2))]);
        world.insert_resource(conveyors);
        world.insert_resource(ItemTable(vec![belt_item.clone()]));
        let mut inserters = Inserters::default();
        inserters.0.insert(7, InserterState::default());
        inserters.0.insert(
            10,
            InserterState {
                source: Some(7),
                destination: Some(9),
                ..Default::default()
            },
        );
        world.insert_resource(inserters);
        world.insert_resource(Extractors::default());
        world.insert_resource(Weapons::default());
        world.insert_resource(Healths::default());
        world.insert_resource(ResearchState::default());
        world.insert_resource(Disabled::default());
        world.spawn((UnitId(unit), Owner(SlotId(0)), Position(Vec2::ZERO)));
        world.resource_mut::<EconomyOrders>().0.insert(
            unit,
            crate::economy::GatherOrder {
                node: Some(NodeId(1)),
                deposit: Some(NodeId(1)),
                receiver: Some(8),
                stage: GatherStage::Idle,
                carry: RateCarry::default(),
            },
        );
        world
            .resource_mut::<Extractors>()
            .0
            .insert(unit, crate::economy::extractor::ExtractorState::default());
        world
            .resource_mut::<Weapons>()
            .0
            .insert(unit, crate::economy::combat::WeaponState::default());
        world.resource_mut::<Healths>().0.insert(unit, 5);
        world.resource_mut::<Disabled>().0.insert(unit);
        world
    }

    #[test]
    fn output_inventory_is_its_own_inventory() {
        let mut world = removal_world();
        {
            let mut ledger = world.resource_mut::<EconomyLedger>();
            ledger.inventories.get_mut(&7).unwrap().capacity = 10;
            ledger.output_inventories.get_mut(&7).unwrap().capacity = 5;
        }
        let result = transact(&mut world, |tx| {
            assert_eq!(tx.inventory(Endpoint::Output(7))?.capacity, 5);
            assert_eq!(tx.inventory_used(Endpoint::Output(7))?, 3);
            assert_eq!(tx.inventory_used_legacy(7)?, 6);
            Ok(())
        });
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn remove_unit_destroys_and_lists_inventory_output_hand_and_belt_contents() {
        let mut world = removal_world();
        let belt_item = content("probe:belt_item");
        let mut conveyors = Conveyors::default();
        let belt = BeltLine {
            def: content("probe:belt"),
            owner: 0,
            pieces: vec![70, 7, 90],
            slots: vec![1, 0, 1, 1, 1, 0],
            slots_per_piece: 2,
            slots_per_second: 2,
            carry: RateCarry::default(),
        };
        conveyors
            .piece_line
            .extend([(70, (70, 0)), (7, (70, 1)), (90, (70, 2))]);
        conveyors.lines.insert(70, belt);
        world.insert_resource(conveyors);
        world.resource_mut::<ItemTable>().0 = vec![belt_item.clone()];
        let mut inserters = Inserters::default();
        inserters.0.insert(7, InserterState::default());
        inserters.0.insert(
            10,
            InserterState {
                source: Some(7),
                destination: Some(9),
                ..Default::default()
            },
        );
        world.insert_resource(inserters);

        transact(&mut world, |tx| remove_unit(tx, 7, RemoveCause::Removed)).unwrap();

        assert!(
            world
                .iter_entities()
                .all(|entity| entity.get::<UnitId>().map(|id| id.0) != Some(7))
        );
        assert!(
            !world
                .resource::<EconomyLedger>()
                .inventories
                .contains_key(&7)
        );
        assert!(
            !world
                .resource::<EconomyLedger>()
                .output_inventories
                .contains_key(&7)
        );
        assert!(!world.resource::<EconomyOrders>().0.contains_key(&7));
        assert!(!world.resource::<Extractors>().0.contains_key(&7));
        assert!(!world.resource::<Weapons>().0.contains_key(&7));
        assert!(!world.resource::<Healths>().0.contains_key(&7));
        assert!(!world.resource::<Disabled>().0.contains(&7));
        assert_eq!(world.resource::<Inserters>().0[&10].source, None);
        assert_eq!(world.resource::<Inserters>().0[&10].destination, Some(9));
        let expected = vec![
            DestroyedLine {
                location: DestroyedLocation::Inventory(7),
                item: content("probe:held"),
                amount: 4,
            },
            DestroyedLine {
                location: DestroyedLocation::Inventory(7),
                item: content("probe:ore"),
                amount: 2,
            },
            DestroyedLine {
                location: DestroyedLocation::Output(7),
                item: content("probe:parts"),
                amount: 3,
            },
            DestroyedLine {
                location: DestroyedLocation::BeltPiece(7),
                item: belt_item,
                amount: 2,
            },
        ];
        assert_eq!(
            world
                .resource::<PendingEconomyEvents>()
                .0
                .last()
                .unwrap()
                .kind,
            EconomyEventKind::Destroyed {
                unit: 7,
                cause: RemoveCause::Removed,
                items: expected
            },
        );
        assert_eq!(world.resource::<RemovedThisTick>().0, vec![7]);
        let lines = &world.resource::<Conveyors>().lines;
        assert_eq!(lines[&70].pieces, vec![70]);
        assert_eq!(lines[&70].slots, vec![1, 0]);
        assert_eq!(lines[&90].pieces, vec![90]);
        assert_eq!(lines[&90].slots, vec![1, 0]);
    }

    #[test]
    fn remove_unit_with_reserved_stock_rejects_integrity_violation() {
        let mut world = removal_world();
        world
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&7)
            .unwrap()
            .items
            .get_mut(&content("probe:ore"))
            .unwrap()
            .reserved = 1;
        let before_ledger = world.resource::<EconomyLedger>().clone();
        let before_events = world.resource::<PendingEconomyEvents>().clone();
        let result = transact(&mut world, |tx| remove_unit(tx, 7, RemoveCause::Removed));
        assert_eq!(result, Err(RejectReason::IntegrityViolation));
        assert_eq!(*world.resource::<EconomyLedger>(), before_ledger);
        assert_eq!(*world.resource::<PendingEconomyEvents>(), before_events);
        assert!(
            world
                .iter_entities()
                .any(|entity| entity.get::<UnitId>().map(|id| id.0) == Some(7))
        );
    }

    #[test]
    fn failed_write_leaves_all_internal_state_equal() {
        let mut world = removal_world();
        world.resource_mut::<EconomyLedger>().balances.insert(
            (0, content("probe:credits")),
            Account {
                on_hand: 2,
                reserved: 0,
            },
        );
        let ledger = world.resource::<EconomyLedger>().clone();
        let nodes = world.resource::<SurfaceNodes>().clone();
        let orders = world.resource::<EconomyOrders>().clone();
        let conveyors = world.resource::<Conveyors>().clone();
        let inserters = world.resource::<Inserters>().clone();
        let extractors = world.resource::<Extractors>().clone();
        let weapons = world.resource::<Weapons>().clone();
        let healths = world.resource::<Healths>().clone();
        let research = world.resource::<ResearchState>().clone();
        let disabled = world.resource::<Disabled>().clone();
        let queues = world.resource::<ProductionQueues>().clone();
        let allocators = *world.resource::<crate::ids::IdAllocators>();
        let events = world.resource::<PendingEconomyEvents>().clone();
        let next_event = *world.resource::<NextEventOrdinal>();
        let removed = world.resource::<RemovedThisTick>().clone();
        let unit_count = crate::world::gameplay_entity_count(&mut world);

        let result = transact(&mut world, |tx| {
            reserve_costs(
                tx,
                JobId(50),
                &[ReservationLine {
                    endpoint: Endpoint::Slot(0),
                    resource: content("probe:credits"),
                    amount: 1,
                }],
            )?;
            tx.allocate_unit_id()?;
            tx.stage_event(
                0,
                EconomyEventKind::Destroyed {
                    unit: 999,
                    cause: RemoveCause::Removed,
                    items: Vec::new(),
                },
            )?;
            remove_unit(tx, 7, RemoveCause::Removed)?;
            tx.stage_account(
                Endpoint::Output(404),
                content("probe:missing"),
                Account {
                    on_hand: 1,
                    reserved: 0,
                },
            );
            Ok(())
        });
        assert_eq!(result, Err(RejectReason::IntegrityViolation));
        assert_eq!(*world.resource::<EconomyLedger>(), ledger);
        assert_eq!(*world.resource::<SurfaceNodes>(), nodes);
        assert_eq!(*world.resource::<EconomyOrders>(), orders);
        assert_eq!(*world.resource::<Conveyors>(), conveyors);
        assert_eq!(*world.resource::<Inserters>(), inserters);
        assert_eq!(*world.resource::<Extractors>(), extractors);
        assert_eq!(*world.resource::<Weapons>(), weapons);
        assert_eq!(*world.resource::<Healths>(), healths);
        assert_eq!(*world.resource::<ResearchState>(), research);
        assert_eq!(*world.resource::<Disabled>(), disabled);
        assert_eq!(*world.resource::<ProductionQueues>(), queues);
        assert_eq!(*world.resource::<crate::ids::IdAllocators>(), allocators);
        assert_eq!(*world.resource::<PendingEconomyEvents>(), events);
        assert_eq!(*world.resource::<NextEventOrdinal>(), next_event);
        assert_eq!(*world.resource::<RemovedThisTick>(), removed);
        assert_eq!(crate::world::gameplay_entity_count(&mut world), unit_count);
    }

    #[test]
    fn extract_from_node_is_atomic() {
        let mut world = removal_world();
        let item = content("probe:ore");
        install_extraction_node(&mut world, &item, 2);
        let mut output = inventory(0, std::slice::from_ref(&item));
        output.capacity = 0;
        world
            .resource_mut::<EconomyLedger>()
            .output_inventories
            .insert(20, output);
        let before_nodes = world.resource::<SurfaceNodes>().clone();
        let before_ledger = world.resource::<EconomyLedger>().clone();

        assert_eq!(
            transact(&mut world, |tx| extract_from_node(
                tx,
                NodeId(1),
                Endpoint::Output(20),
                item,
                1
            )),
            Err(RejectReason::RefundDestinationFull),
        );
        assert_eq!(*world.resource::<SurfaceNodes>(), before_nodes);
        assert_eq!(*world.resource::<EconomyLedger>(), before_ledger);
    }

    #[test]
    fn extract_from_node_credits_output_and_records_extracted_event() {
        let mut world = removal_world();
        let item = content("probe:ore");
        install_extraction_node(&mut world, &item, 2);
        world
            .resource_mut::<EconomyLedger>()
            .output_inventories
            .insert(20, inventory(1, std::slice::from_ref(&item)));
        world.spawn((UnitId(20), Owner(SlotId(0)), Position(Vec2::ZERO)));

        transact(&mut world, |tx| {
            extract_from_node(tx, NodeId(1), Endpoint::Output(20), item.clone(), 1)
        })
        .unwrap();

        assert_eq!(world.resource::<SurfaceNodes>().0[0].remaining, 1);
        assert_eq!(
            world.resource::<EconomyLedger>().output_inventories[&20].items[&item].on_hand,
            1,
        );
        assert!(matches!(
            &world.resource::<PendingEconomyEvents>().0[0].kind,
            EconomyEventKind::Extracted { node: 1, cargo: 20, item: actual, amount: 1 } if actual == &item,
        ));
    }

    #[test]
    fn removed_this_tick_reaches_committed_tick() {
        let mut app = crate::headless_app(20);
        app.world_mut().resource_mut::<RemovedThisTick>().0.push(42);
        app.update();
        assert_eq!(app.world().resource::<CommittedTick>().removed, vec![42]);
        assert!(app.world().resource::<RemovedThisTick>().0.is_empty());
    }

    #[test]
    fn missing_belt_item_index_rejects_removal_without_changes() {
        let mut world = removal_world();
        world.resource_mut::<ItemTable>().0.clear();
        let ledger = world.resource::<EconomyLedger>().clone();
        let conveyors = world.resource::<Conveyors>().clone();
        let inserters = world.resource::<Inserters>().clone();
        let removed = world.resource::<RemovedThisTick>().clone();
        let unit_count = crate::world::gameplay_entity_count(&mut world);

        assert_eq!(
            transact(&mut world, |tx| remove_unit(tx, 7, RemoveCause::Removed)),
            Err(RejectReason::IntegrityViolation),
        );
        assert_eq!(*world.resource::<EconomyLedger>(), ledger);
        assert_eq!(*world.resource::<Conveyors>(), conveyors);
        assert_eq!(*world.resource::<Inserters>(), inserters);
        assert_eq!(*world.resource::<RemovedThisTick>(), removed);
        assert_eq!(crate::world::gameplay_entity_count(&mut world), unit_count);
    }

    #[test]
    fn m3_extract_to_missing_main_inventory_is_integrity_violation() {
        let mut world = removal_world();
        let item = content("probe:ore");
        install_extraction_node(&mut world, &item, 2);
        let before_nodes = world.resource::<SurfaceNodes>().clone();
        let before_ledger = world.resource::<EconomyLedger>().clone();

        assert_eq!(
            transact(&mut world, |tx| extract_from_node(
                tx,
                NodeId(1),
                Endpoint::Inventory(404),
                item,
                1,
            )),
            Err(RejectReason::IntegrityViolation),
        );
        assert_eq!(*world.resource::<SurfaceNodes>(), before_nodes);
        assert_eq!(*world.resource::<EconomyLedger>(), before_ledger);
    }

    #[test]
    fn legacy_main_inventory_write_to_missing_row_keeps_skip_behavior() {
        let mut world = World::new();
        world.insert_resource(EconomyLedger::default());
        let item = content("probe:legacy");

        let result = transact(&mut world, |tx| {
            tx.stage_account(
                Endpoint::Inventory(404),
                item.clone(),
                Account {
                    on_hand: 3,
                    reserved: 0,
                },
            );
            Ok(())
        });

        assert_eq!(result, Ok(()));
        assert!(
            !world
                .resource::<EconomyLedger>()
                .inventories
                .contains_key(&404)
        );
    }

    #[test]
    fn removing_a_unit_spawned_earlier_this_tick_is_rejected() {
        let mut world = removal_world();
        world.insert_resource(crate::economy::events::SpawnedThisTick(vec![
            crate::world::UnitState {
                unit_id: 7,
                content_id: "probe:unit".to_string(),
                owner_slot: 0,
                facing: None,
                pos: Vec2::ZERO,
            },
        ]));
        let ledger = world.resource::<EconomyLedger>().clone();
        let conveyors = world.resource::<Conveyors>().clone();
        let inserters = world.resource::<Inserters>().clone();

        assert_eq!(
            transact(&mut world, |tx| remove_unit(tx, 7, RemoveCause::Removed)),
            Err(RejectReason::IntegrityViolation),
        );
        assert_eq!(*world.resource::<EconomyLedger>(), ledger);
        assert_eq!(*world.resource::<Conveyors>(), conveyors);
        assert_eq!(*world.resource::<Inserters>(), inserters);
        assert_eq!(
            world
                .resource::<crate::economy::events::SpawnedThisTick>()
                .0[0]
                .unit_id,
            7
        );
        assert!(
            world
                .iter_entities()
                .any(|entity| entity.get::<UnitId>().map(|id| id.0) == Some(7))
        );
    }
}
