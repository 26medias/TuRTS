//! Inventories domain: the items in unit inventories, output inventories, belts, nodes and job
//! charges.
//!
//! Rules:
//! - inventories: in each step, the total count of each item (unit inventories, output inventories,
//!   belt slots, node remaining amounts and the non-slot charge lines of jobs) changes by the sum
//!   of its logged effects: minus `Converted` amounts, minus shot ammo, minus `Destroyed` items,
//!   plus `ItemsProduced` outputs, plus the non-slot refund lines, minus the non-slot charge
//!   lines of the jobs that end in the step. A step with no events changes no count.
//! - inventories: for a unit that is not an inserter and that no inserter serves, in each step,
//!   the items of the unit (its inventory and its output inventory) change by the sum of the
//!   events that name the unit.
//! - inventories: no inventory holds more items than its capacity.

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::ContentId;
use turts_sim::economy::events::DestroyedLocation as DestroyedLocationView;
use turts_sim::{EconomyEventKind, EventSource, ReservationEndpointView};

use crate::{LedgerAudit, Step, add, delta, differences, ended_jobs, job_charges, new_charges};

fn node_items(state: &LedgerAudit, initial: &LedgerAudit) -> BTreeMap<ContentId, ContentId> {
    initial
        .catalog
        .as_ref()
        .map(|catalog| {
            catalog
                .nodes
                .iter()
                .map(|node| (node.id.clone(), node.item.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|(node, _)| state.nodes.iter().any(|row| &row.content_id == node))
        .collect()
}

fn total_items(state: &LedgerAudit, initial: &LedgerAudit) -> BTreeMap<ContentId, i128> {
    let mut total = BTreeMap::new();
    for inventory in &state.economy.inventories {
        for item in &inventory.items {
            add(&mut total, item.resource.clone(), i128::from(item.on_hand));
        }
    }
    for output in &state.raw_output_inventories {
        for (item, account) in &output.items {
            add(&mut total, item.clone(), i128::from(account.on_hand));
        }
    }
    for conveyor in &state.raw_belts {
        for slot in &conveyor.raw_slots {
            if let Some(item) = &slot.item {
                add(&mut total, item.clone(), 1);
            }
        }
    }
    let items = node_items(state, initial);
    for node in &state.nodes {
        if let Some(item) = items.get(&node.content_id) {
            add(&mut total, item.clone(), i128::from(node.remaining));
        }
    }
    for lines in job_charges(state).values() {
        for line in *lines {
            if !matches!(line.endpoint, ReservationEndpointView::Slot(_)) {
                add(&mut total, line.resource.clone(), i128::from(line.amount));
            }
        }
    }
    total
}

fn unit_items(state: &LedgerAudit) -> BTreeMap<(u32, ContentId), i128> {
    let mut items = BTreeMap::new();
    for inventory in &state.economy.inventories {
        for item in &inventory.items {
            add(
                &mut items,
                (inventory.unit, item.resource.clone()),
                i128::from(item.on_hand),
            );
        }
    }
    for output in &state.raw_output_inventories {
        for (item, account) in &output.items {
            add(
                &mut items,
                (output.unit, item.clone()),
                i128::from(account.on_hand),
            );
        }
    }
    items
}

fn source_unit(source: EventSource) -> Option<u32> {
    match source {
        EventSource::Slot(_) => None,
        EventSource::Inventory(unit) | EventSource::Output(unit) => Some(unit),
    }
}

pub(crate) fn check(step: &Step<'_>, initial: &LedgerAudit) -> Result<(), String> {
    let label = step.label();
    let ended = ended_jobs(step.before, step.after);
    let charges_before = job_charges(step.before);

    // The total count of each item.
    let actual = delta(
        &total_items(step.before, initial),
        &total_items(step.after, initial),
    );
    let mut expected = BTreeMap::new();
    // The items of each unit that the events name.
    let actual_units = delta(&unit_items(step.before), &unit_items(step.after));
    let mut expected_units = BTreeMap::new();
    for event in step.events {
        match &event.kind {
            EconomyEventKind::Extracted {
                cargo,
                item,
                amount,
                ..
            } => add(
                &mut expected_units,
                (*cargo, item.clone()),
                i128::from(*amount),
            ),
            EconomyEventKind::Stored {
                from,
                to,
                item,
                amount,
            } => {
                add(
                    &mut expected_units,
                    (*from, item.clone()),
                    -i128::from(*amount),
                );
                add(
                    &mut expected_units,
                    (*to, item.clone()),
                    i128::from(*amount),
                );
            }
            EconomyEventKind::Converted {
                from, item, amount, ..
            } => {
                add(&mut expected, item.clone(), -i128::from(*amount));
                add(
                    &mut expected_units,
                    (*from, item.clone()),
                    -i128::from(*amount),
                );
            }
            EconomyEventKind::Fired {
                weapon,
                item: Some(item),
                ammo,
                ..
            } => {
                add(&mut expected, item.clone(), -i128::from(*ammo));
                add(
                    &mut expected_units,
                    (*weapon, item.clone()),
                    -i128::from(*ammo),
                );
            }
            EconomyEventKind::Destroyed { items, .. } => {
                for line in items {
                    add(&mut expected, line.item.clone(), -i128::from(line.amount));
                    match line.location {
                        DestroyedLocationView::Inventory(unit)
                        | DestroyedLocationView::Output(unit) => add(
                            &mut expected_units,
                            (unit, line.item.clone()),
                            -i128::from(line.amount),
                        ),
                        DestroyedLocationView::BeltPiece(_) => {}
                    }
                }
            }
            EconomyEventKind::ItemsProduced {
                machine, outputs, ..
            } => {
                for line in outputs {
                    add(
                        &mut expected,
                        line.resource.clone(),
                        i128::from(line.amount),
                    );
                    add(
                        &mut expected_units,
                        (*machine, line.resource.clone()),
                        i128::from(line.amount),
                    );
                }
            }
            EconomyEventKind::ProductionCancelled { refunded, .. } => {
                for line in refunded {
                    if let Some(unit) = source_unit(line.source) {
                        add(
                            &mut expected,
                            line.resource.clone(),
                            i128::from(line.amount),
                        );
                        add(
                            &mut expected_units,
                            (unit, line.resource.clone()),
                            i128::from(line.amount),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    for job in &ended {
        for line in charges_before.get(job).copied().unwrap_or_default() {
            if !matches!(line.endpoint, ReservationEndpointView::Slot(_)) {
                add(
                    &mut expected,
                    line.resource.clone(),
                    -i128::from(line.amount),
                );
            }
        }
    }
    for (endpoint, resource, amount) in new_charges(step.before, step.after) {
        match endpoint {
            ReservationEndpointView::Slot(_) => {}
            ReservationEndpointView::Inventory(unit) | ReservationEndpointView::Output(unit) => {
                add(&mut expected_units, (unit, resource), -amount);
            }
        }
    }
    expected.retain(|_, value| *value != 0);
    if let Some((item, actual, expected)) = differences(&actual, &expected).into_iter().next() {
        return Err(format!(
            "inventories: the count of {item} changed by {actual} in {label}; its logged events explain {expected}"
        ));
    }

    // Inserters, and the units that they serve, move items with no event: they are not checked
    // one by one.
    let mut served = BTreeSet::new();
    for state in [step.before, step.after] {
        for inserter in &state.raw_inserters {
            served.insert(inserter.unit);
            served.extend(inserter.source);
            served.extend(inserter.destination);
        }
    }
    let removed: BTreeSet<u32> = step.after.removed_units.iter().copied().collect();
    expected_units.retain(|(unit, _), value| *value != 0 && !served.contains(unit));
    let actual_units: BTreeMap<_, _> = actual_units
        .into_iter()
        .filter(|((unit, _), _)| !served.contains(unit) && !removed.contains(unit))
        .collect();
    expected_units.retain(|(unit, _), _| !removed.contains(unit));
    if let Some(((unit, item), actual, expected)) = differences(&actual_units, &expected_units)
        .into_iter()
        .next()
    {
        return Err(format!(
            "inventories: unit {unit} {item} changed by {actual} in {label}; the events that name the unit explain {expected}"
        ));
    }

    for inventory in &step.after.economy.inventories {
        let used: u128 = inventory
            .items
            .iter()
            .map(|item| u128::from(item.on_hand))
            .sum();
        if used > u128::from(inventory.capacity) {
            return Err(format!(
                "inventories: unit {} holds {used} items above its capacity {} after {label}",
                inventory.unit, inventory.capacity
            ));
        }
    }
    for output in &step.after.raw_output_inventories {
        let used: u128 = output
            .items
            .iter()
            .map(|(_, account)| u128::from(account.on_hand))
            .sum();
        if used > u128::from(output.capacity) {
            return Err(format!(
                "inventories: the output of unit {} holds {used} items above its capacity {} after {label}",
                output.unit, output.capacity
            ));
        }
    }
    Ok(())
}
