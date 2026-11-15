//! Owner-only supply baselines and changes from completed immutable state.
//!
//! A supply record names a slot and resource, never a unit. Explicit zero totals remain present.

use turts_net_protocol::messages as wire;
use turts_sim::{CommittedTick, SlotId, economy::views::OperatingView};

pub(super) fn snapshot(operating: Option<&OperatingView>, slot: SlotId) -> Vec<wire::SupplyRecord> {
    operating
        .into_iter()
        .flat_map(|view| &view.supply)
        .filter(|row| row.slot == slot.0)
        .map(|row| wire::SupplyRecord {
            slot: wire::SlotId(row.slot),
            resource: row.resource.to_string(),
            provided: row.provided,
            consumed: row.consumed,
        })
        .collect()
}

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
