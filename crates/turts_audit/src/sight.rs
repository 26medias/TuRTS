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
            let alive = !kind.has_health
                || start
                    .raw_healths
                    .iter()
                    .any(|row| row.unit == unit.unit_id && row.current > 0);
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
