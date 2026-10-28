//! M3 transactional resource overlays, extraction, and removal.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::{entity::Entity, world::World};
use turts_mod_api::ContentId;

use crate::{
    Owner, RejectReason, UnitId,
    economy::{
        EconomyOrders, NodeRecord, SurfaceNodes,
        combat::{Healths, Weapons},
        conveyor::{BeltLine, Conveyors, ItemTable},
        events::{
            DestroyedLine, DestroyedLocation, EconomyEventKind, NextEventOrdinal,
            PendingEconomyEvents, RemoveCause, RemovedThisTick, SpawnedThisTick,
        },
        extractor::Extractors,
        inserter::Inserters,
        ledger::{Account, EconomyLedger, Endpoint, ReservationClaim},
        production::ProductionQueues,
        research::{Disabled, ResearchState},
    },
};

use super::{RemoveWrite, Transaction, WriteSet};

impl Transaction<'_> {
    pub(crate) fn mark_m3(&mut self) {
        self.writes.strict_m3 = true;
    }

    fn node(&self, id: crate::ids::NodeId) -> Result<NodeRecord, RejectReason> {
        if let Some(staged) = self.writes.nodes.get(&id) {
            return staged.clone().ok_or(RejectReason::UnknownNode);
        }
        self.world
            .get_resource::<SurfaceNodes>()
            .and_then(|nodes| nodes.0.iter().find(|node| node.id == id))
            .cloned()
            .ok_or(RejectReason::UnknownNode)
    }

    fn stage_node(&mut self, node: NodeRecord) {
        self.writes.nodes.insert(node.id, Some(node));
    }

    pub(super) fn preflight(&self) -> Result<(), RejectReason> {
        for (endpoint, resource) in self.writes.accounts.keys() {
            let missing_main_inventory = match endpoint {
                Endpoint::Inventory(unit) => !self
                    .world
                    .get_resource::<EconomyLedger>()
                    .is_some_and(|ledger| ledger.inventories.contains_key(unit)),
                Endpoint::Slot(_) | Endpoint::Output(_) => false,
            };
            if missing_main_inventory && !self.writes.strict_m3 {
                continue;
            }
            if missing_main_inventory {
                return Err(RejectReason::IntegrityViolation);
            }
            self.validate_endpoint(*endpoint, resource)?;
        }

        let inventory_endpoints = self
            .writes
            .accounts
            .keys()
            .filter_map(|(endpoint, _)| match endpoint {
                Endpoint::Inventory(unit)
                    if self
                        .world
                        .get_resource::<EconomyLedger>()
                        .is_some_and(|ledger| ledger.inventories.contains_key(unit)) =>
                {
                    Some(*endpoint)
                }
                Endpoint::Output(_) => Some(*endpoint),
                Endpoint::Inventory(_) | Endpoint::Slot(_) => None,
            })
            .collect::<BTreeSet<_>>();
        for endpoint in inventory_endpoints {
            let inventory = self.inventory(endpoint)?;
            if self.inventory_used(endpoint)? > u128::from(inventory.capacity) {
                return Err(RejectReason::RefundDestinationFull);
            }
        }

        if !self.writes.nodes.is_empty() {
            let nodes = self
                .world
                .get_resource::<SurfaceNodes>()
                .ok_or(RejectReason::IntegrityViolation)?;
            if self
                .writes
                .nodes
                .keys()
                .any(|id| !nodes.0.iter().any(|node| node.id == *id))
            {
                return Err(RejectReason::IntegrityViolation);
            }
        }
        if (!self.writes.conveyor_lines.is_empty() || !self.writes.piece_line.is_empty())
            && !self.world.contains_resource::<Conveyors>()
        {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.inserters.is_empty() && !self.world.contains_resource::<Inserters>() {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.extractors.is_empty() && !self.world.contains_resource::<Extractors>() {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.weapons.is_empty() && !self.world.contains_resource::<Weapons>() {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.healths.is_empty() && !self.world.contains_resource::<Healths>() {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.orders.is_empty() && !self.world.contains_resource::<EconomyOrders>() {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.research.is_empty() && !self.world.contains_resource::<ResearchState>() {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.disabled.is_empty() && !self.world.contains_resource::<Disabled>() {
            return Err(RejectReason::IntegrityViolation);
        }
        if !self.writes.removed.is_empty() {
            if !self.world.contains_resource::<RemovedThisTick>() {
                return Err(RejectReason::IntegrityViolation);
            }
            let mut seen = BTreeSet::new();
            for removal in &self.writes.removed {
                if !seen.insert(removal.unit)
                    || self.world.get::<UnitId>(removal.entity) != Some(&UnitId(removal.unit))
                    || self
                        .writes
                        .spawned
                        .iter()
                        .any(|spawn| spawn.id == removal.unit)
                    || self
                        .world
                        .get_resource::<SpawnedThisTick>()
                        .is_some_and(|spawned| {
                            spawned.0.iter().any(|state| state.unit_id == removal.unit)
                        })
                {
                    return Err(RejectReason::IntegrityViolation);
                }
                if self
                    .world
                    .get_resource::<ProductionQueues>()
                    .and_then(|queues| queues.0.get(&removal.unit))
                    .is_some_and(|queue| !queue.is_empty())
                {
                    return Err(RejectReason::NotRemovable);
                }
            }
        }
        if !self.writes.events.is_empty()
            && (!self.world.contains_resource::<PendingEconomyEvents>()
                || !self.world.contains_resource::<NextEventOrdinal>())
        {
            return Err(RejectReason::IntegrityViolation);
        }
        Ok(())
    }
}

pub(super) fn apply_overlays(world: &mut World, writes: &mut WriteSet) {
    if !writes.nodes.is_empty() {
        let mut nodes = world.resource_mut::<SurfaceNodes>();
        for (id, node) in std::mem::take(&mut writes.nodes) {
            if let Some(node) = node {
                let index = nodes
                    .0
                    .iter()
                    .position(|current| current.id == id)
                    .expect("node was preflighted before commit");
                nodes.0[index] = node;
            } else {
                nodes.0.retain(|current| current.id != id);
            }
        }
    }
    if !writes.conveyor_lines.is_empty() || !writes.piece_line.is_empty() {
        let mut conveyors = world.resource_mut::<Conveyors>();
        for (id, line) in std::mem::take(&mut writes.conveyor_lines) {
            if let Some(line) = line {
                conveyors.lines.insert(id, line);
            } else {
                conveyors.lines.remove(&id);
            }
        }
        for (piece, line) in std::mem::take(&mut writes.piece_line) {
            if let Some(line) = line {
                conveyors.piece_line.insert(piece, line);
            } else {
                conveyors.piece_line.remove(&piece);
            }
        }
    }
    if !writes.inserters.is_empty() {
        let mut inserters = world.resource_mut::<Inserters>();
        apply_key_overlay(&mut inserters.0, std::mem::take(&mut writes.inserters));
    }
    if !writes.extractors.is_empty() {
        let mut extractors = world.resource_mut::<Extractors>();
        apply_key_overlay(&mut extractors.0, std::mem::take(&mut writes.extractors));
    }
    if !writes.weapons.is_empty() {
        let mut weapons = world.resource_mut::<Weapons>();
        apply_key_overlay(&mut weapons.0, std::mem::take(&mut writes.weapons));
    }
    if !writes.healths.is_empty() {
        let mut healths = world.resource_mut::<Healths>();
        apply_key_overlay(&mut healths.0, std::mem::take(&mut writes.healths));
    }
    if !writes.orders.is_empty() {
        let mut orders = world.resource_mut::<EconomyOrders>();
        apply_key_overlay(&mut orders.0, std::mem::take(&mut writes.orders));
    }
    if !writes.research.is_empty() {
        let mut research = world.resource_mut::<ResearchState>();
        for ((slot, technology), completed) in std::mem::take(&mut writes.research) {
            if completed.is_some() {
                research.completed.insert((slot, technology));
            } else {
                research.completed.remove(&(slot, technology));
            }
        }
    }
    if !writes.disabled.is_empty() {
        let mut disabled = world.resource_mut::<Disabled>();
        for (unit, value) in std::mem::take(&mut writes.disabled) {
            if value.is_some() {
                disabled.0.insert(unit);
            } else {
                disabled.0.remove(&unit);
            }
        }
    }
}

fn apply_key_overlay<K: Ord, V>(table: &mut BTreeMap<K, V>, overlay: BTreeMap<K, Option<V>>) {
    for (key, value) in overlay {
        if let Some(value) = value {
            table.insert(key, value);
        } else {
            table.remove(&key);
        }
    }
}

#[allow(dead_code)] // used from Task 7
pub(crate) fn split_line(
    line: &BeltLine,
    piece_index: usize,
) -> (Option<BeltLine>, Option<BeltLine>) {
    assert!(
        piece_index < line.pieces.len(),
        "piece index must refer to a belt piece"
    );
    let slots_per_piece = usize::from(line.slots_per_piece);
    assert_eq!(line.slots.len(), line.pieces.len() * slots_per_piece);
    let before_slots = piece_index * slots_per_piece;
    let after_slots = (piece_index + 1) * slots_per_piece;
    let left = (piece_index > 0).then(|| BeltLine {
        pieces: line.pieces[..piece_index].to_vec(),
        slots: line.slots[..before_slots].to_vec(),
        ..line.clone()
    });
    let right = (piece_index + 1 < line.pieces.len()).then(|| BeltLine {
        pieces: line.pieces[piece_index + 1..].to_vec(),
        slots: line.slots[after_slots..].to_vec(),
        ..line.clone()
    });
    (left, right)
}

#[allow(dead_code)] // used from Task 6
pub(crate) fn extract_from_node(
    tx: &mut Transaction<'_>,
    node_id: crate::ids::NodeId,
    to: Endpoint,
    item: ContentId,
    amount: u64,
) -> Result<(), RejectReason> {
    if amount == 0 {
        return Err(RejectReason::InvalidAmount);
    }
    let mut node = tx.node(node_id)?;
    let catalog_item = tx
        .catalog()?
        .0
        .nodes
        .get(&node.content_id)
        .map(|definition| &definition.item)
        .ok_or(RejectReason::IntegrityViolation)?;
    if catalog_item != &item {
        return Err(RejectReason::IncompatibleItem);
    }
    if node.remaining < amount {
        return Err(RejectReason::NodeDepleted);
    }
    let unit = match to {
        Endpoint::Inventory(unit) | Endpoint::Output(unit) => unit,
        Endpoint::Slot(_) => return Err(RejectReason::IncompatibleItem),
    };
    tx.validate_endpoint(to, &item)?;
    let destination = tx.inventory(to)?;
    let used = tx.inventory_used(to)?;
    if used
        .checked_add(u128::from(amount))
        .ok_or(RejectReason::ArithmeticOverflow)?
        > u128::from(destination.capacity)
    {
        return Err(RejectReason::RefundDestinationFull);
    }
    let current = tx.account(to, &item);
    let on_hand = current
        .on_hand
        .checked_add(amount)
        .ok_or(RejectReason::RefundDestinationOverflow)?;
    let entity = find_unit_entity(tx.world, unit).ok_or(RejectReason::UnknownUnit)?;
    let owner = tx
        .world
        .get::<Owner>(entity)
        .map(|owner| owner.0.0)
        .ok_or(RejectReason::IntegrityViolation)?;

    tx.mark_m3();
    node.remaining = node
        .remaining
        .checked_sub(amount)
        .ok_or(RejectReason::NodeDepleted)?;
    tx.stage_node(node);
    tx.stage_account(to, item.clone(), Account { on_hand, ..current });
    tx.stage_event(
        owner,
        EconomyEventKind::Extracted {
            node: node_id.0,
            cargo: unit,
            item,
            amount,
        },
    )?;
    Ok(())
}

#[allow(dead_code)] // used from Task 11
pub(crate) fn remove_unit(
    tx: &mut Transaction<'_>,
    unit: u32,
    cause: RemoveCause,
) -> Result<(), RejectReason> {
    if tx.writes.removed.iter().any(|removal| removal.unit == unit)
        || tx
            .world
            .get_resource::<SpawnedThisTick>()
            .is_some_and(|spawned| spawned.0.iter().any(|state| state.unit_id == unit))
    {
        return Err(RejectReason::IntegrityViolation);
    }
    let entity = find_unit_entity(tx.world, unit).ok_or(RejectReason::UnknownUnit)?;
    tx.mark_m3();
    let owner = tx
        .world
        .get::<Owner>(entity)
        .map(|owner| owner.0.0)
        .ok_or(RejectReason::IntegrityViolation)?;
    if tx
        .world
        .get_resource::<ProductionQueues>()
        .and_then(|queues| queues.0.get(&unit))
        .is_some_and(|queue| !queue.is_empty())
    {
        return Err(RejectReason::NotRemovable);
    }

    let ledger = tx
        .world
        .get_resource::<EconomyLedger>()
        .ok_or(RejectReason::IntegrityViolation)?;
    let has_reservation = ledger
        .reservation_claims
        .values()
        .chain(tx.writes.claims.values())
        .any(|claim| match claim {
            ReservationClaim::Active(reservation) => reservation.lines.iter().any(|line| {
                matches!(line.endpoint, Endpoint::Inventory(id) | Endpoint::Output(id) if id == unit)
            }),
            ReservationClaim::Closed => false,
        });
    if has_reservation {
        return Err(RejectReason::IntegrityViolation);
    }
    let mut items = Vec::new();
    append_inventory_contents(
        tx,
        &mut items,
        Endpoint::Inventory(unit),
        DestroyedLocation::Inventory(unit),
    )?;
    append_inventory_contents(
        tx,
        &mut items,
        Endpoint::Output(unit),
        DestroyedLocation::Output(unit),
    )?;

    if let Some((line_key, piece_index)) = tx
        .world
        .get_resource::<Conveyors>()
        .and_then(|conveyors| conveyors.piece_line.get(&unit).copied())
    {
        let line = tx
            .world
            .get_resource::<Conveyors>()
            .and_then(|conveyors| conveyors.lines.get(&line_key))
            .cloned()
            .ok_or(RejectReason::IntegrityViolation)?;
        let index = usize::from(piece_index);
        if line.pieces.get(index) != Some(&unit)
            || usize::from(line.slots_per_piece) == 0
            || line.slots.len() != line.pieces.len() * usize::from(line.slots_per_piece)
        {
            return Err(RejectReason::IntegrityViolation);
        }
        let start = index * usize::from(line.slots_per_piece);
        let end = start + usize::from(line.slots_per_piece);
        let table = tx
            .world
            .get_resource::<ItemTable>()
            .ok_or(RejectReason::IntegrityViolation)?;
        let mut destroyed = BTreeMap::<ContentId, u64>::new();
        for item_ix in &line.slots[start..end] {
            if *item_ix == 0 {
                continue;
            }
            let content = table
                .content(*item_ix)
                .ok_or(RejectReason::IntegrityViolation)?;
            let amount = destroyed.entry(content.clone()).or_default();
            *amount = amount
                .checked_add(1)
                .ok_or(RejectReason::ArithmeticOverflow)?;
        }
        items.extend(destroyed.into_iter().map(|(item, amount)| DestroyedLine {
            location: DestroyedLocation::BeltPiece(unit),
            item,
            amount,
        }));
        let (left, right) = split_line(&line, index);
        for piece in &line.pieces {
            tx.writes.piece_line.insert(*piece, None);
        }
        if let Some(left) = left {
            for (piece_index, piece) in left.pieces.iter().enumerate() {
                tx.writes.piece_line.insert(
                    *piece,
                    Some((
                        line_key,
                        u16::try_from(piece_index).map_err(|_| RejectReason::IntegrityViolation)?,
                    )),
                );
            }
            tx.writes.conveyor_lines.insert(line_key, Some(left));
        } else {
            tx.writes.conveyor_lines.insert(line_key, None);
        }
        if let Some(right) = right {
            let right_key = *right
                .pieces
                .first()
                .ok_or(RejectReason::IntegrityViolation)?;
            for (piece_index, piece) in right.pieces.iter().enumerate() {
                tx.writes.piece_line.insert(
                    *piece,
                    Some((
                        right_key,
                        u16::try_from(piece_index).map_err(|_| RejectReason::IntegrityViolation)?,
                    )),
                );
            }
            tx.writes.conveyor_lines.insert(right_key, Some(right));
        }
    }
    items.sort_by(|a, b| (a.location, &a.item).cmp(&(b.location, &b.item)));

    let inserters = tx
        .world
        .get_resource::<Inserters>()
        .ok_or(RejectReason::IntegrityViolation)?;
    for (id, state) in &inserters.0 {
        if *id == unit {
            tx.writes.inserters.insert(*id, None);
            continue;
        }
        if state.source == Some(unit) || state.destination == Some(unit) {
            let mut updated = state.clone();
            if updated.source == Some(unit) {
                updated.source = None;
            }
            if updated.destination == Some(unit) {
                updated.destination = None;
            }
            tx.writes.inserters.insert(*id, Some(updated));
        }
    }
    tx.writes.extractors.insert(unit, None);
    tx.writes.weapons.insert(unit, None);
    tx.writes.healths.insert(unit, None);
    tx.writes.disabled.insert(unit, None);

    let orders = tx
        .world
        .get_resource::<EconomyOrders>()
        .ok_or(RejectReason::IntegrityViolation)?;
    for (id, order) in &orders.0 {
        if *id == unit {
            tx.writes.orders.insert(*id, None);
        } else if order.receiver == Some(unit) {
            let mut updated = order.clone();
            updated.receiver = None;
            tx.writes.orders.insert(*id, Some(updated));
        }
    }
    tx.mark_m3();
    tx.writes.removed.push(RemoveWrite { unit, entity });
    tx.stage_event(owner, EconomyEventKind::Destroyed { unit, cause, items })?;
    Ok(())
}

fn append_inventory_contents(
    tx: &Transaction<'_>,
    items: &mut Vec<DestroyedLine>,
    endpoint: Endpoint,
    location: DestroyedLocation,
) -> Result<(), RejectReason> {
    let Some(inventory) =
        tx.world
            .get_resource::<EconomyLedger>()
            .and_then(|ledger| match endpoint {
                Endpoint::Inventory(unit) => ledger.inventories.get(&unit),
                Endpoint::Output(unit) => ledger.output_inventories.get(&unit),
                Endpoint::Slot(_) => None,
            })
    else {
        return Ok(());
    };
    let resources = inventory
        .items
        .keys()
        .cloned()
        .chain(
            tx.writes
                .accounts
                .keys()
                .filter(|(candidate, _)| *candidate == endpoint)
                .map(|(_, resource)| resource.clone()),
        )
        .collect::<BTreeSet<_>>();
    for item in resources {
        let account = tx.account(endpoint, &item);
        if account.reserved != 0 {
            return Err(RejectReason::IntegrityViolation);
        }
        if account.on_hand != 0 {
            items.push(DestroyedLine {
                location,
                item,
                amount: account.on_hand,
            });
        }
    }
    Ok(())
}

fn find_unit_entity(world: &World, unit: u32) -> Option<Entity> {
    world.iter_entities().find_map(|entity| {
        entity
            .get::<UnitId>()
            .is_some_and(|id| id.0 == unit)
            .then_some(entity.id())
    })
}

// Transport phases stage only the affected line, never the whole conveyor resource.
impl Transaction<'_> {
    pub(crate) fn conveyor_line(&self, id: u32) -> Result<BeltLine, RejectReason> {
        if let Some(staged) = self.writes.conveyor_lines.get(&id) {
            return staged.clone().ok_or(RejectReason::IntegrityViolation);
        }
        self.world
            .get_resource::<Conveyors>()
            .and_then(|conveyors| conveyors.lines.get(&id))
            .cloned()
            .ok_or(RejectReason::IntegrityViolation)
    }

    pub(crate) fn stage_conveyor_line(&mut self, id: u32, line: BeltLine) {
        self.mark_m3();
        self.writes.conveyor_lines.insert(id, Some(line));
    }
}
