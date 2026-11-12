//! Combat domain: shots, ammo, health and unit removal.
//!
//! Rules:
//! - combat: a shot with an ammo item spends a positive ammo amount; a shot with no ammo item
//!   spends none.
//! - combat: in each operation, the ammo item of each weapon falls by the ammo of its shots, when
//!   no other event of the operation names that unit.
//! - combat: in each step, the health of each unit falls by the applied damage of the shots at it,
//!   and changes in no other way.
//! - combat: the applied damage of a shot is the health of the target, or the damage when it is
//!   smaller.
//! - combat: no shot is at a removed unit.
//! - combat: a `Killed` event names a unit whose health reaches zero in the operation, and a unit
//!   whose health reaches zero has one `Killed` event in that operation.
//! - combat: a unit leaves the world only in an operation with a `Destroyed` event for it, and is
//!   then in `removed_units`; a removed unit has no row in any table.

use std::collections::{BTreeMap, BTreeSet};

use turts_sim::{EconomyEventKind, economy::events::RemoveCause};

use crate::{LedgerAudit, Step, add, delta, differences};

fn health(state: &LedgerAudit) -> BTreeMap<u32, i128> {
    state
        .raw_healths
        .iter()
        .map(|row| (row.unit, i128::from(row.current)))
        .collect()
}

/// The units that an event names (as a source, a target or a location).
fn named_units(kind: &EconomyEventKind) -> Vec<u32> {
    match kind {
        EconomyEventKind::Extracted { cargo, .. } => vec![*cargo],
        EconomyEventKind::Stored { from, to, .. }
        | EconomyEventKind::Converted { from, to, .. } => {
            vec![*from, *to]
        }
        EconomyEventKind::ProductionQueued { producer, .. }
        | EconomyEventKind::ProductionStarted { producer, .. }
        | EconomyEventKind::ProductionCompleted { producer, .. }
        | EconomyEventKind::ProductionCancelled { producer, .. } => vec![*producer],
        EconomyEventKind::Destroyed { unit, .. } => vec![*unit],
        EconomyEventKind::ItemsProduced { machine, .. } => vec![*machine],
        EconomyEventKind::ResearchCompleted { station, .. } => vec![*station],
        EconomyEventKind::Fired { .. } => Vec::new(),
        EconomyEventKind::EnabledChanged { unit, .. } => vec![*unit],
    }
}

fn check_removed(state: &LedgerAudit, label: &str) -> Result<(), String> {
    for id in &state.removed_units {
        let id = *id;
        let remains = state.raw_units.iter().any(|row| row.unit_id == id)
            || state.economy.units.iter().any(|row| row.unit_id == id)
            || state.economy.inventories.iter().any(|row| row.unit == id)
            || state
                .raw_output_inventories
                .iter()
                .any(|row| row.unit == id)
            || state
                .raw_inserters
                .iter()
                .any(|row| row.unit == id || row.source == Some(id) || row.destination == Some(id))
            || state.raw_extractors.iter().any(|row| row.unit == id)
            || state.raw_weapons.iter().any(|row| row.unit == id)
            || state.raw_healths.iter().any(|row| row.unit == id)
            || state.disabled.contains(&id)
            || state.raw_belts.iter().any(|row| {
                row.first_piece == id || row.pieces.iter().any(|piece| piece.unit == id)
            })
            || state
                .raw_orders
                .iter()
                .any(|row| row.unit == id || row.receiver == Some(id))
            || state.raw_queues.iter().any(|row| row.producer == id);
        if remains {
            return Err(format!(
                "combat: removed unit {id} remains in the authoritative state after {label}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn check(step: &Step<'_>) -> Result<(), String> {
    let label = step.label();
    let before = health(step.before);
    let after_rows = health(step.after);
    let removed_after: BTreeSet<u32> = step.after.removed_units.iter().copied().collect();
    let removed_before: BTreeSet<u32> = step.before.removed_units.iter().copied().collect();
    // A unit that is removed in the step ends with zero health.
    let mut after = after_rows.clone();
    for unit in before.keys() {
        if !after_rows.contains_key(unit) && removed_after.contains(unit) {
            after.insert(*unit, 0);
        }
    }
    let mut applied = BTreeMap::new();
    let mut ammo = BTreeMap::new();
    let mut killed = BTreeSet::new();
    let mut destroyed = BTreeSet::new();
    let mut other_units = BTreeSet::new();
    let mut running = before.clone();
    for event in step.events {
        match &event.kind {
            EconomyEventKind::Fired {
                weapon,
                target,
                item,
                ammo: spent,
                damage,
                applied: done,
            } => {
                if item.is_some() != (*spent > 0) {
                    return Err(format!(
                        "combat: weapon {weapon} fired with ammo item {item:?} and ammo {spent} in {label}"
                    ));
                }
                if removed_before.contains(target) {
                    return Err(format!(
                        "combat: weapon {weapon} fired at the removed unit {target} in {label}"
                    ));
                }
                let current = running.get(target).copied().unwrap_or(0);
                let expected = current.min(i128::from(*damage));
                if i128::from(*done) != expected {
                    return Err(format!(
                        "combat: the shot of weapon {weapon} at unit {target} applied {done}, its health {current} and damage {damage} give {expected} in {label}"
                    ));
                }
                running.insert(*target, current - i128::from(*done));
                add(&mut applied, *target, -i128::from(*done));
                if let Some(item) = item {
                    add(&mut ammo, (*weapon, item.clone()), -i128::from(*spent));
                }
            }
            EconomyEventKind::Destroyed { unit, cause, .. } => {
                destroyed.insert(*unit);
                if matches!(cause, RemoveCause::Killed { .. }) && !killed.insert(*unit) {
                    return Err(format!(
                        "combat: unit {unit} has more than one Killed event in {label}"
                    ));
                }
                other_units.insert(*unit);
            }
            kind => other_units.extend(named_units(kind)),
        }
    }
    applied.retain(|_, value| *value != 0);
    let actual = delta(&before, &after);
    if let Some((unit, actual, expected)) = differences(&actual, &applied).into_iter().next() {
        return Err(format!(
            "combat: the health of unit {unit} changed by {actual} in {label}; its shots apply {expected}"
        ));
    }
    for (unit, value) in &running {
        let reached_zero = *value == 0 && before.get(unit).is_some_and(|old| *old > 0);
        if reached_zero != killed.contains(unit) {
            return Err(format!(
                "combat: unit {unit} health is {value} and its Killed event count is {} in {label}",
                usize::from(killed.contains(unit))
            ));
        }
    }
    for unit in &killed {
        if !removed_after.contains(unit) {
            return Err(format!(
                "combat: the killed unit {unit} is not removed after {label}"
            ));
        }
    }
    // Ammo: the weapon inventory falls by its shots.
    let items = |state: &LedgerAudit| -> BTreeMap<(u32, turts_mod_api::ContentId), i128> {
        let mut result = BTreeMap::new();
        for inventory in &state.economy.inventories {
            for item in &inventory.items {
                add(
                    &mut result,
                    (inventory.unit, item.resource.clone()),
                    i128::from(item.on_hand),
                );
            }
        }
        result
    };
    if !ammo.is_empty() {
        let changed = delta(&items(step.before), &items(step.after));
        for ((weapon, item), spent) in &ammo {
            if other_units.contains(weapon) {
                continue;
            }
            let lost = changed.get(&(*weapon, item.clone())).copied().unwrap_or(0);
            if lost != *spent {
                return Err(format!(
                    "combat: weapon {weapon} {item} changed by {lost} in {label}; its shots spend {spent}"
                ));
            }
        }
    }
    // Removal.
    let units_after: BTreeSet<u32> = step.after.raw_units.iter().map(|row| row.unit_id).collect();
    for unit in &step.before.raw_units {
        if units_after.contains(&unit.unit_id) {
            continue;
        }
        if !destroyed.contains(&unit.unit_id) || !removed_after.contains(&unit.unit_id) {
            return Err(format!(
                "combat: unit {} left the world with no Destroyed event in {label}",
                unit.unit_id
            ));
        }
    }
    if removed_before
        .iter()
        .any(|unit| !removed_after.contains(unit))
    {
        return Err(format!(
            "combat: a removed unit left removed_units in {label}"
        ));
    }
    check_removed(step.after, &label)
}
