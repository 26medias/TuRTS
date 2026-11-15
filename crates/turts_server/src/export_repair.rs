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

#[cfg(test)]
mod tests {
    use super::*;

    fn row(repairer: u32, target: u32, owner_slot: u8, state: RepairStatus) -> RepairRow {
        RepairRow {
            repairer,
            target,
            owner_slot,
            state,
        }
    }

    fn unit(unit_id: u32, owner_slot: u8) -> UnitState {
        UnitState {
            unit_id,
            content_id: "fixture:repairer".into(),
            owner_slot,
            facing: None,
            pos: Default::default(),
        }
    }

    #[test]
    fn snapshots_include_only_the_owners_actions_with_exact_wait_resource() {
        let rows = [
            row(1, 2, 0, RepairStatus::Approaching),
            row(3, 4, 1, RepairStatus::Working),
            row(
                5,
                2,
                0,
                RepairStatus::WaitingForFunds(
                    turts_mod_api::ContentId::parse("fixture:credits").unwrap(),
                ),
            ),
        ];
        assert_eq!(
            snapshot(&rows, SlotId(0)),
            vec![
                RepairRecord {
                    repairer: UnitId(1),
                    target: UnitId(2),
                    state: RepairState::Approaching
                },
                RepairRecord {
                    repairer: UnitId(5),
                    target: UnitId(2),
                    state: RepairState::WaitingForFunds("fixture:credits".into())
                },
            ]
        );
        assert_eq!(
            snapshot(&rows, SlotId(1)),
            vec![RepairRecord {
                repairer: UnitId(3),
                target: UnitId(4),
                state: RepairState::Working,
            }]
        );
        assert!(snapshot(&[], SlotId(0)).is_empty());
        assert!(snapshot(&rows, SlotId(2)).is_empty());
    }

    #[test]
    fn deltas_clear_only_surviving_owned_sources_and_never_write_removed_endpoints() {
        let rows = [
            row(1, 2, 0, RepairStatus::Working),
            row(3, 4, 1, RepairStatus::Working),
            row(5, 6, 0, RepairStatus::Approaching),
            row(7, 8, 0, RepairStatus::Approaching),
        ];
        let units = [unit(1, 0), unit(3, 1), unit(5, 0), unit(9, 0)];
        let (actions, cleared) = changes(&rows, &[3, 7, 9, 20], &[6, 7], &units, SlotId(0));
        assert_eq!(
            actions,
            vec![RepairRecord {
                repairer: UnitId(1),
                target: UnitId(2),
                state: RepairState::Working,
            }]
        );
        assert_eq!(cleared, vec![UnitId(9)]);
        let (actions, cleared) = changes(&rows, &[3, 7, 9, 20], &[6, 7], &units, SlotId(1));
        assert_eq!(
            actions,
            vec![RepairRecord {
                repairer: UnitId(3),
                target: UnitId(4),
                state: RepairState::Working,
            }]
        );
        assert_eq!(cleared, vec![UnitId(3)]);
    }
}
