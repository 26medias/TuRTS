//! Industry domain: resource nodes and belts.
//!
//! Rules:
//! - industry: in each step, the `remaining` amount of each node falls by the sum of the
//!   `Extracted` events of that node, and changes in no other way. A node never leaves the log.
//! - industry: the item of an `Extracted` event is the item of its node in the catalog.
//! - industry: the slots of the pieces of a belt are its line slots, in order (the piece view is
//!   a copy of the line, so a belt item is held one time).
//! - industry: a gather order fails with `node_depleted` only when its unit holds no cargo (a
//!   depleted node starts the delivery of the held cargo; the cargo is never left behind).

use std::collections::BTreeMap;

use turts_sim::EconomyEventKind;

use crate::{LedgerAudit, Step, add, delta, differences};

fn remaining(state: &LedgerAudit) -> BTreeMap<u32, i128> {
    state
        .nodes
        .iter()
        .map(|node| (node.id.0, i128::from(node.remaining)))
        .collect()
}

pub(crate) fn check(step: &Step<'_>, initial: &LedgerAudit) -> Result<(), String> {
    let label = step.label();
    let before = remaining(step.before);
    let after = remaining(step.after);
    if let Some(node) = before.keys().find(|node| !after.contains_key(node)) {
        return Err(format!("industry: node {node} left the log in {label}"));
    }
    let actual = delta(&before, &after);
    let mut expected = BTreeMap::new();
    for event in step.events {
        if let EconomyEventKind::Extracted {
            node, item, amount, ..
        } = &event.kind
        {
            add(&mut expected, *node, -i128::from(*amount));
            let node_item = step
                .before
                .nodes
                .iter()
                .find(|row| row.id.0 == *node)
                .and_then(|row| {
                    initial.catalog.as_ref().and_then(|catalog| {
                        catalog
                            .nodes
                            .iter()
                            .find(|entry| entry.id == row.content_id)
                            .map(|entry| entry.item.clone())
                    })
                });
            if node_item.as_ref() != Some(item) {
                return Err(format!(
                    "industry: node {node} gives {item} in {label}, its catalog item is {node_item:?}"
                ));
            }
        }
    }
    expected.retain(|_, value| *value != 0);
    if let Some((node, actual, expected)) = differences(&actual, &expected).into_iter().next() {
        return Err(format!(
            "industry: node {node} remaining changed by {actual} in {label}; its Extracted events explain {expected}"
        ));
    }
    for row in &step.after.unit_orders {
        let Some(failure) = &row.orders.last_failure else {
            continue;
        };
        if failure.reason != turts_sim::RejectReason::NodeDepleted {
            continue;
        }
        let new_failure = step
            .before
            .unit_orders
            .iter()
            .find(|old| old.unit_id == row.unit_id)
            .and_then(|old| old.orders.last_failure.as_ref())
            != Some(failure);
        let cargo: u64 = step
            .after
            .economy
            .inventories
            .iter()
            .filter(|inventory| inventory.unit == row.unit_id)
            .flat_map(|inventory| &inventory.items)
            .map(|item| item.on_hand)
            .sum();
        if new_failure && cargo > 0 {
            return Err(format!(
                "industry: unit {} fails its gather order with node_depleted while it holds {cargo} cargo in {label}",
                row.unit_id
            ));
        }
    }
    for conveyor in &step.after.raw_belts {
        let pieces: Vec<_> = conveyor
            .pieces
            .iter()
            .flat_map(|piece| piece.slots.iter())
            .collect();
        let line: Vec<_> = conveyor.raw_slots.iter().collect();
        if pieces != line {
            return Err(format!(
                "industry: the pieces of conveyor line {} differ from its line slots after {label}",
                conveyor.first_piece
            ));
        }
    }
    Ok(())
}
