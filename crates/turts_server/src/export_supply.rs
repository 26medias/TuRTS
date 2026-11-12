//! The supply records of one delta (design 6.2, 6.8). Owner only, sent on change. The writer of
//! the sim rows is `L-PWR`; the sim rows are empty in lane 0.
//!
//! This file reads only `CommittedTick` fields (the scan `order_writers.rs`). A supply record
//! names a slot and a resource, never a unit, so the removal rule has nothing to strip.

use turts_net_protocol::messages as wire;
use turts_sim::{CommittedTick, SlotId};

pub(super) fn supply(batch: &CommittedTick, slot: SlotId) -> Vec<wire::SupplyRecord> {
    batch
        .supply
        .iter()
        .filter(|row| row.slot == slot.0)
        .map(|row| wire::SupplyRecord {
            slot: wire::SlotId(row.slot),
            resource: row.resource.to_string(),
            provided: row.provided,
            consumed: row.consumed,
        })
        .collect()
}
