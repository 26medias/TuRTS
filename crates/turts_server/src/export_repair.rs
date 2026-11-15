//! Owner-only repair actions projected from completed immutable rows.

use turts_net_protocol::{RepairRecord, RepairState, UnitId};
use turts_sim::{
    SlotId,
    economy::repair::{RepairRow, RepairStatus},
    world::UnitState,
};

fn record(row: &RepairRow) -> RepairRecord {
    RepairRecord {
        repairer: UnitId(row.repairer),
        target: UnitId(row.target),
        state: match &row.state {
            RepairStatus::Approaching => RepairState::Approaching,
            RepairStatus::Working => RepairState::Working,
            RepairStatus::WaitingForFunds(resource) => {
                RepairState::WaitingForFunds(resource.to_string())
            }
        },
    }
}

pub(super) fn snapshot(rows: &[RepairRow], slot: SlotId) -> Vec<RepairRecord> {
    rows.iter()
        .filter(|row| row.owner_slot == slot.0)
        .map(record)
        .collect()
}

pub(super) fn changes(
    rows: &[RepairRow],
    cleared: &[u32],
    removed: &[u32],
    units: &[UnitState],
    slot: SlotId,
) -> (Vec<RepairRecord>, Vec<UnitId>) {
    let records = rows
        .iter()
        .filter(|row| {
            row.owner_slot == slot.0
                && !removed.contains(&row.repairer)
                && !removed.contains(&row.target)
        })
        .map(record)
        .collect();
    let cleared = cleared
        .iter()
        .filter(|id| {
            !removed.contains(id)
                && units
                    .iter()
                    .any(|unit| unit.unit_id == **id && unit.owner_slot == slot.0)
        })
        .map(|id| UnitId(*id))
        .collect();
    (records, cleared)
}
