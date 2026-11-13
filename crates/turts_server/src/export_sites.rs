//! The site records of one delta (design 6.8). Public: site existence and completion. Owner:
//! the progress of a site. All rows come from the committed simulation tick.
//!
//! This file reads only `CommittedTick` fields (the scan `order_writers.rs`).
//!
//! Removal rule: a record never names a unit that the same delta removes. A site that is
//! destroyed or completed-and-destroyed in the tick has `removed` as its only mention.

use turts_net_protocol::messages as wire;
use turts_sim::{CommittedTick, SlotId};

use super::export_jobs::rate_record;

/// The sites that completed in this tick. Public. A site that is also removed in this tick is
/// left out.
pub(super) fn sites_completed(batch: &CommittedTick) -> Vec<wire::UnitId> {
    batch
        .sites_completed
        .iter()
        .filter(|unit| batch.removed.binary_search(unit).is_err())
        .map(|unit| wire::UnitId(*unit))
        .collect()
}

/// The site progress rows of the owner. A removed site has no row.
pub(super) fn site_progress(batch: &CommittedTick, slot: SlotId) -> Vec<wire::SiteProgress> {
    batch
        .site_progress
        .iter()
        .filter(|row| row.owner_slot == slot.0 && batch.removed.binary_search(&row.unit).is_err())
        .map(|row| wire::SiteProgress {
            unit: wire::UnitId(row.unit),
            progress: rate_record(row),
        })
        .collect()
}

/// The construction queues of the owner. A removed constructor has no row.
pub(super) fn construction_queues(
    batch: &CommittedTick,
    slot: SlotId,
) -> Vec<wire::ConstructionQueue> {
    batch
        .construction_queues
        .iter()
        .filter(|row| {
            row.owner_slot == slot.0 && batch.removed.binary_search(&row.constructor).is_err()
        })
        .map(|row| wire::ConstructionQueue {
            constructor: wire::UnitId(row.constructor),
            orders: row
                .orders
                .iter()
                .map(|(content, point)| wire::HeldOrder {
                    content: content.to_string(),
                    point: wire::Pos {
                        x: point.x,
                        y: point.y,
                    },
                })
                .collect(),
        })
        .collect()
}
