//! Balances domain: the slot balance accounts (`economy.balances`).
//!
//! Rules:
//! - balances: in each step, the `on_hand` change of each slot balance equals the sum of its logged
//!   effects: plus the `credited` amount of each `Converted` event of that slot, plus the refunded
//!   lines of that slot, minus the new job charge lines of that slot. A step with no events (a
//!   gap, or the end of a tick) changes no balance.

use std::collections::BTreeMap;

use turts_mod_api::ContentId;
use turts_sim::{EconomyEventKind, EventSource, ReservationEndpointView};

use crate::{LedgerAudit, Step, add, delta, differences, new_charges};

fn on_hand(state: &LedgerAudit) -> BTreeMap<(u8, ContentId), i128> {
    let mut result = BTreeMap::new();
    for balance in &state.economy.balances {
        add(
            &mut result,
            (balance.slot, balance.resource.clone()),
            i128::from(balance.on_hand),
        );
    }
    result
}

pub(crate) fn check(step: &Step<'_>) -> Result<(), String> {
    let actual = delta(&on_hand(step.before), &on_hand(step.after));
    let mut expected = BTreeMap::new();
    for event in step.events {
        match &event.kind {
            EconomyEventKind::Converted {
                balance, credited, ..
            } => add(
                &mut expected,
                (event.owner_slot, balance.clone()),
                i128::from(*credited),
            ),
            EconomyEventKind::ProductionCancelled { refunded, .. } => {
                for line in refunded {
                    if let EventSource::Slot(slot) = line.source {
                        add(
                            &mut expected,
                            (slot, line.resource.clone()),
                            i128::from(line.amount),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    for (endpoint, resource, amount) in new_charges(step.before, step.after) {
        if let ReservationEndpointView::Slot(slot) = endpoint {
            add(&mut expected, (slot, resource), -amount);
        }
    }
    expected.retain(|_, value| *value != 0);
    if let Some(((slot, resource), actual, expected)) =
        differences(&actual, &expected).into_iter().next()
    {
        return Err(format!(
            "balances: slot {slot} {resource} on_hand changed by {actual} in {}; its logged events explain {expected}",
            step.label()
        ));
    }
    Ok(())
}
