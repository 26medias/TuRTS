//! Stub alert triggers of task `UI2-CONN` (plan 5.3; design 7.9, row H4). L0-CORE made this file;
//! `UI2-CONN` fills it. Each function returns the units that raise the alert of `rule` in this
//! evaluation, as the arms of `collect_alert_events` do. Until `UI2-CONN` builds a trigger, it
//! raises no alert.

use super::*;
use turts_mod_api::AlertDef;

pub(super) fn slot_disconnected(
    _current: &Replica,
    _previous: Option<&Replica>,
    _slot: u8,
    _rule: &AlertDef,
    _runtime: &mut FeedbackRuntime,
) -> Vec<u32> {
    Vec::new()
}

pub(super) fn slot_reconnected(
    _current: &Replica,
    _previous: Option<&Replica>,
    _slot: u8,
    _rule: &AlertDef,
    _runtime: &mut FeedbackRuntime,
) -> Vec<u32> {
    Vec::new()
}
