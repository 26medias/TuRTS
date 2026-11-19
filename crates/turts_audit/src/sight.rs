//! Independent sight for the movement replay. The log holds only the constant inputs of
//! sight (`AuditState::raw_sight`). This module computes which bodies a slot may not see
//! from the audited state at the start of the tick. It never reads an observed mask or a
//! hidden-unit list from the log, and it does not call the incremental coverage code of
//! the simulation: only the shared grid geometry of `turts_rules`.
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::VisionSharing;
use turts_rules::visibility::{Grid, Point};
use turts_sim::AuditState;
use turts_sim::visibility::{SightAudit, SightPolicy};

use crate::EconomyLogRecord;

fn checked_grid(sight: &SightAudit) -> Result<(Grid, usize), String> {
    let grid = sight.policy.grid;
    let checked = Grid::checked(grid.size.x, grid.size.y, grid.cell_size)
        .ok_or("audited sight grid is invalid")?;
    let cells = u64::from(grid.columns) * u64::from(grid.rows);
    if checked.columns != grid.columns
        || checked.rows != grid.rows
        || cells > turts_mod_api::MAX_VISIBILITY_CELLS
    {
        return Err("audited sight grid differs from its checked geometry".into());
    }
    Ok((checked, cells as usize))
}

fn shared(policy: &SightPolicy, viewer: u8, owner: u8) -> bool {
    viewer == owner
        || policy.sharing == VisionSharing::Team
            && policy
                .teams
                .get(usize::from(viewer))
                .zip(policy.teams.get(usize::from(owner)))
                .is_some_and(|(a, b)| a == b)
}

fn states(record: &EconomyLogRecord) -> Vec<&AuditState> {
    match record {
        EconomyLogRecord::Initial(state) => vec![state],
        EconomyLogRecord::Operation(operation) => vec![&operation.before, &operation.after],
        EconomyLogRecord::Tick { before, state, .. } => vec![before, state],
        EconomyLogRecord::Admission { before, after, .. } => vec![before, after],
        EconomyLogRecord::CommandBoundary(command) => vec![&command.before, &command.after],
        EconomyLogRecord::Actor(_) | EconomyLogRecord::Claim { .. } => Vec::new(),
    }
}

fn has_pending_unit_target(state: &AuditState) -> bool {
    state
        .pending_movement_orders
        .queues
        .values()
        .flat_map(|queue| queue.iter())
        .any(|request| {
            request.unit_ids.iter().any(|unit| {
                matches!(
                    request.order_for_unit(*unit).map(|order| &order.target),
                    Some(turts_sim::OrderTargetSim::Unit(_))
                )
            })
        })
}

fn states_mut(record: &mut EconomyLogRecord) -> Vec<&mut AuditState> {
    match record {
        EconomyLogRecord::Initial(state) => vec![state],
        EconomyLogRecord::Operation(operation) => {
            vec![&mut operation.before, &mut operation.after]
        }
        EconomyLogRecord::Tick { before, state, .. } => vec![before, state],
        EconomyLogRecord::Admission { before, after, .. } => vec![before, after],
        EconomyLogRecord::CommandBoundary(command) => {
            vec![&mut command.before, &mut command.after]
        }
        EconomyLogRecord::Actor(_) | EconomyLogRecord::Claim { .. } => Vec::new(),
    }
}

/// A lobby server writes `Initial` before the room fixes the teams: the log starts with the
/// fail-closed placeholder (`finalized: false`) and each snapshot after the match start has
/// the final inputs. This is the one permitted change. It must occur before the first `Tick`
/// and the first `Operation` record, so no dispatch can read the placeholder. It may change
/// only `finalized` and `teams`. The result is a copy of the log in which the leading
/// placeholder snapshots have the final inputs; `None` when the log has no such change.
pub(crate) fn finalized_at_start(
    records: &[EconomyLogRecord],
    initial: &AuditState,
) -> Result<Option<Vec<EconomyLogRecord>>, String> {
    let Some(placeholder) = initial.raw_sight.as_ref().filter(|sight| !sight.finalized) else {
        return Ok(None);
    };
    let mut started = false;
    for (index, record) in records.iter().enumerate() {
        let Some(sight) = states(record).into_iter().find_map(|state| {
            state
                .raw_sight
                .as_ref()
                .filter(|sight| *sight != placeholder)
        }) else {
            started |= matches!(
                record,
                EconomyLogRecord::Tick { .. } | EconomyLogRecord::Operation(_)
            );
            continue;
        };
        if started
            || !sight.finalized
            || sight.policy.grid != placeholder.policy.grid
            || sight.policy.sharing != placeholder.policy.sharing
            || sight.types != placeholder.types
            || states(record)
                .into_iter()
                .any(|state| state.raw_sight.as_ref() != Some(sight))
        {
            return Err("sight inputs changed after the Initial record".into());
        }
        let sight = sight.clone();
        let mut copy = records.to_vec();
        for state in copy[..index].iter_mut().flat_map(states_mut) {
            state.raw_sight = Some(sight.clone());
        }
        return Ok(Some(copy));
    }
    Ok(None)
}

/// The sight inputs are constant for a match: every snapshot must repeat those of `Initial`.
/// Under restricted sight, this replay does not model a pending movement order with a Unit
/// target (the live server prunes a hidden target). It refuses such a log.
pub(crate) fn validate_records(
    records: &[EconomyLogRecord],
    initial: &AuditState,
) -> Result<(), String> {
    if let Some(sight) = &initial.raw_sight {
        checked_grid(sight)?;
        if sight
            .types
            .windows(2)
            .any(|pair| pair[0].type_id >= pair[1].type_id)
        {
            return Err("audited sight unit types are not in strict order".into());
        }
    }
    for state in records.iter().flat_map(states) {
        if state.raw_sight != initial.raw_sight {
            return Err("sight inputs changed after the Initial record".into());
        }
        if state.raw_sight.is_some() && has_pending_unit_target(state) {
            return Err(
                "restricted sight replay does not support a pending Unit-target movement".into(),
            );
        }
    }
    Ok(())
}

/// The bodies of another slot that `slot` may not see in a group spread of this tick.
/// `start` is the audited state at the start of the tick: the live dispatch reads the Start
/// basis, which the server builds before any command or system of the tick runs. A unit
/// that did not exist at the start is not in that basis, so it is hidden.
/// With no `raw_sight` (AllVisible) the set is empty.
pub(crate) fn hidden_bodies(
    start: &AuditState,
    before: &AuditState,
    slot: u8,
) -> Result<BTreeSet<u32>, String> {
    let Some(sight) = &before.raw_sight else {
        return Ok(BTreeSet::new());
    };
    if start.raw_sight.as_ref() != Some(sight) {
        return Err("sight inputs differ between the tick start and the dispatch".into());
    }
    let (grid, cells) = checked_grid(sight)?;
    let kinds: BTreeMap<_, _> = sight
        .types
        .iter()
        .map(|kind| (kind.type_id, kind))
        .collect();
    let mut observed = vec![false; cells];
    // Health rows are not guaranteed to be sorted. Index positive health once
    // so reconstructing many sight sources does not scan all health rows per unit.
    let alive_units: BTreeSet<_> = start
        .raw_healths
        .iter()
        .filter(|row| row.current > 0)
        .map(|row| row.unit)
        .collect();
    let mut start_units = BTreeMap::new();
    // The fail-closed placeholder has an empty basis: it permits no unit of another slot.
    if sight.finalized {
        for unit in &start.raw_units {
            let Some(kind) = kinds.get(&unit.unit_type) else {
                continue;
            };
            start_units.insert(unit.unit_id, unit);
            let Some(radius) = kind.radius else {
                continue;
            };
            let site = start.economy.construction.sites.contains_key(&unit.unit_id);
            let alive = !kind.has_health || alive_units.contains(&unit.unit_id);
            if site || !alive || !shared(&sight.policy, slot, unit.owner_slot) {
                continue;
            }
            grid.circle(
                Point {
                    x: unit.pos.x,
                    y: unit.pos.y,
                },
                radius,
                |cell| observed[cell as usize] = true,
            );
        }
    }
    Ok(before
        .raw_units
        .iter()
        .filter(|unit| {
            unit.owner_slot != slot
                && !start_units.get(&unit.unit_id).is_some_and(|seen| {
                    seen.owner_slot == slot
                        || grid
                            .cell(Point {
                                x: seen.pos.x,
                                y: seen.pos.y,
                            })
                            .is_some_and(|cell| observed[cell as usize])
                })
        })
        .map(|unit| unit.unit_id)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{VisibilityGrid, WorldPos};
    use turts_sim::economy::{views::RawUnitAudit, views_m3::RawHealthAudit};
    use turts_sim::visibility::SightTypeAudit;

    #[test]
    fn sight_sources_require_positive_health_in_any_row() {
        let mut app = turts_sim::headless_app(20);
        let mut state = turts_sim::audit_state(app.world_mut());
        state.raw_sight = Some(SightAudit {
            policy: SightPolicy {
                grid: VisibilityGrid {
                    size: WorldPos { x: 64.0, y: 64.0 },
                    cell_size: 2.0,
                    columns: 32,
                    rows: 32,
                },
                sharing: VisionSharing::OwnSlot,
                teams: vec![0, 1],
            },
            finalized: true,
            types: vec![SightTypeAudit {
                type_id: 1,
                radius: Some(6.0),
                has_health: true,
            }],
        });
        state.raw_units = vec![
            RawUnitAudit {
                unit_id: 1,
                unit_type: 1,
                owner_slot: 0,
                pos: WorldPos { x: 5.0, y: 5.0 },
                move_target: None,
            },
            RawUnitAudit {
                unit_id: 2,
                unit_type: 1,
                owner_slot: 1,
                pos: WorldPos { x: 6.0, y: 5.0 },
                move_target: None,
            },
        ];
        state.raw_healths = vec![
            RawHealthAudit {
                unit: 1,
                current: 0,
            },
            RawHealthAudit {
                unit: 2,
                current: 100,
            },
            RawHealthAudit {
                unit: 1,
                current: 100,
            },
        ];
        assert!(!hidden_bodies(&state, &state, 0).unwrap().contains(&2));
        state.raw_healths.pop();
        assert!(hidden_bodies(&state, &state, 0).unwrap().contains(&2));
    }
}
