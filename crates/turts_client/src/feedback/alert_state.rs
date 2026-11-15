//! Stub alert triggers of task `UI-ALERT` (plan 5.3; design 7.9, row H4). L0-CORE made this file;
//! `UI-ALERT` fills it. Each function returns the units that raise the alert of `rule` in this
//! evaluation, as the arms of `collect_alert_events` do. Until `UI-ALERT` builds a trigger, it
//! raises no alert.

use super::*;
use turts_mod_api::AlertDef;

pub(super) fn construction_complete(
    _current: &Replica,
    _previous: Option<&Replica>,
    _slot: u8,
    _rule: &AlertDef,
    _runtime: &mut FeedbackRuntime,
) -> Vec<u32> {
    Vec::new()
}

pub(super) fn supply_short(
    current: &Replica,
    previous: Option<&Replica>,
    _slot: u8,
    rule: &AlertDef,
    _runtime: &mut FeedbackRuntime,
) -> Vec<u32> {
    let AlertTrigger::SupplyShort(resource) = &rule.trigger else {
        return Vec::new();
    };
    supply_transition(current, previous, resource, true)
}

pub(super) fn supply_restored(
    current: &Replica,
    previous: Option<&Replica>,
    _slot: u8,
    rule: &AlertDef,
    _runtime: &mut FeedbackRuntime,
) -> Vec<u32> {
    let AlertTrigger::SupplyRestored(resource) = &rule.trigger else {
        return Vec::new();
    };
    supply_transition(current, previous, resource, false)
}

fn supply_transition(
    current: &Replica,
    previous: Option<&Replica>,
    resource: &str,
    short: bool,
) -> Vec<u32> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    // Snapshots establish a baseline. A supply alert represents a later accepted transition.
    if current
        .supply(resource, turts_mod_api::SupplySide::Provided)
        .is_none()
        || previous
            .supply(resource, turts_mod_api::SupplySide::Provided)
            .is_none()
        || current.supply_short(resource) != short
        || previous.supply_short(resource) == short
    {
        return Vec::new();
    }
    // Zero identifies an alert for the whole slot, without an arbitrary unit or map position.
    vec![0]
}

pub(super) fn insufficient_funds(
    _current: &Replica,
    _previous: Option<&Replica>,
    _slot: u8,
    _rule: &AlertDef,
    _runtime: &mut FeedbackRuntime,
) -> Vec<u32> {
    Vec::new()
}

pub(super) fn gatherer_idle(
    _current: &Replica,
    _previous: Option<&Replica>,
    _slot: u8,
    _rule: &AlertDef,
    _runtime: &mut FeedbackRuntime,
) -> Vec<u32> {
    Vec::new()
}
