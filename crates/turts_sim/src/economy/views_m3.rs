//! Public, serializable mirrors of authoritative M3 state for audit logs.
//! These records do not expose Bevy resources or crate-private simulation state.
use super::views::CostView;
use super::{
    combat::{Healths, WeaponStatus, Weapons},
    conveyor::{Conveyors, ItemTable},
    extractor::{ExtractorStatus, Extractors},
    inserter::{ArmStage, ArmWait, Inserters},
    ledger::{Account, EconomyLedger, Inventory},
    research::{Disabled, ResearchState, modifier},
};
use bevy_ecs::world::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use turts_mod_api::{ContentCatalog, ContentId, ResolvedEffect, WeaponStat};
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RemovedHistory(pub(crate) BTreeSet<u32>);

use crate::{
    Owner, RejectReason, UnitId, UnitType, bootstrap::InstalledCatalog, registry::UnitTypes,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawOutputInventoryAudit {
    pub unit: u32,
    pub capacity: u64,
    pub accepted_items: Vec<ContentId>,
    pub items: Vec<(ContentId, Account)>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawBeltSlotAudit {
    pub item_index: u16,
    pub item: Option<ContentId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawBeltPieceAudit {
    pub unit: u32,
    pub slots: Vec<RawBeltSlotAudit>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawBeltAudit {
    pub first_piece: u32,
    pub definition: ContentId,
    pub owner_slot: u8,
    pub slots_per_piece: u8,
    pub slots_per_second: u64,
    pub rate_carry: u16,
    pub raw_slots: Vec<RawBeltSlotAudit>,
    pub pieces: Vec<RawBeltPieceAudit>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawInserterWaitAudit {
    NoSource,
    NoDestination,
    SourceEmpty,
    DestinationFull,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawInserterStageAudit {
    AtSource(Option<RawInserterWaitAudit>),
    ToDestination,
    AtDestination(RawInserterWaitAudit),
    Returning,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawInserterAudit {
    pub unit: u32,
    pub enabled: bool,
    pub stage: RawInserterStageAudit,
    pub elapsed: u32,
    pub source: Option<u32>,
    pub destination: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawExtractorStatusAudit {
    Extracting,
    Full,
    Depleted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExtractorAudit {
    pub unit: u32,
    pub enabled: bool,
    pub node: Option<u32>,
    pub rate_carry: u16,
    pub status: RawExtractorStatusAudit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawWeaponStatusAudit {
    Ready,
    Cooling,
    NoTarget,
    OutOfAmmo,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawWeaponAudit {
    pub unit: u32,
    pub enabled: bool,
    pub cooldown: u32,
    pub effective_damage: Result<u64, RejectReason>,
    pub status: RawWeaponStatusAudit,
    pub last_target: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawHealthAudit {
    pub unit: u32,
    pub current: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogEffectAudit {
    WeaponModifier {
        unit: ContentId,
        stat: WeaponStat,
        add: u64,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogTechnologyAudit {
    pub id: ContentId,
    pub prerequisites: Vec<ContentId>,
    pub duration_ms: u64,
    pub costs: Vec<CostView>,
    pub payment: turts_mod_api::PaymentPolicy,
    pub refunds: turts_mod_api::CancelRefunds,
    pub effects: Vec<CatalogEffectAudit>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogItemStackAudit {
    pub item: ContentId,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogItemRecipeAudit {
    pub id: ContentId,
    pub inputs: Vec<CatalogItemStackAudit>,
    pub outputs: Vec<CatalogItemStackAudit>,
    pub duration_ms: u64,
    pub refunds: turts_mod_api::RunRefunds,
}
pub fn catalog_item_recipe_audits(catalog: &ContentCatalog) -> Vec<CatalogItemRecipeAudit> {
    catalog
        .recipes
        .iter()
        .map(|(id, recipe)| CatalogItemRecipeAudit {
            id: id.clone(),
            inputs: recipe
                .inputs
                .iter()
                .map(|stack| CatalogItemStackAudit {
                    item: stack.item.clone(),
                    amount: stack.amount,
                })
                .collect(),
            outputs: recipe
                .outputs
                .iter()
                .map(|stack| CatalogItemStackAudit {
                    item: stack.item.clone(),
                    amount: stack.amount,
                })
                .collect(),
            duration_ms: recipe.duration_ms,
            refunds: recipe.cancel_refunds,
        })
        .collect()
}

pub fn catalog_technology_audits(catalog: &ContentCatalog) -> Vec<CatalogTechnologyAudit> {
    catalog
        .technologies
        .iter()
        .map(|(id, tech)| CatalogTechnologyAudit {
            id: id.clone(),
            prerequisites: tech.prerequisites.clone(),
            duration_ms: tech.duration_ms,
            costs: tech
                .costs
                .iter()
                .map(|cost| CostView {
                    source: cost.source,
                    resource: cost.resource.clone(),
                    amount: cost.amount,
                })
                .collect(),
            payment: tech.payment,
            refunds: tech.cancel_refunds,
            effects: tech
                .effects
                .iter()
                .map(|effect| match effect {
                    ResolvedEffect::WeaponModifier { unit, stat, add } => {
                        CatalogEffectAudit::WeaponModifier {
                            unit: unit.clone(),
                            stat: *stat,
                            add: *add,
                        }
                    }
                })
                .collect(),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawEnabledAudit {
    pub unit: u32,
    pub enabled: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct M3AuditChanges {
    pub output_inventories: Vec<RawOutputInventoryAudit>,
    pub belts: Vec<RawBeltAudit>,
    pub inserters: Vec<RawInserterAudit>,
    pub extractors: Vec<RawExtractorAudit>,
    pub weapons: Vec<RawWeaponAudit>,
    pub healths: Vec<RawHealthAudit>,
    pub research: Vec<RawResearchAudit>,
    pub enabled: Vec<RawEnabledAudit>,
}
fn changed_by_key<T: Clone + PartialEq, K: Ord>(
    before: &[T],
    after: &[T],
    key: impl Fn(&T) -> K,
) -> Vec<T> {
    let old: std::collections::BTreeMap<_, _> = before.iter().map(|row| (key(row), row)).collect();
    after
        .iter()
        .filter(|row| old.get(&key(row)).is_none_or(|previous| *previous != *row))
        .cloned()
        .collect()
}
pub fn m3_changes(before: &M3AuditState, after: &M3AuditState) -> M3AuditChanges {
    let units: std::collections::BTreeSet<_> = before
        .disabled
        .iter()
        .chain(&after.disabled)
        .copied()
        .collect();
    let enabled = units
        .into_iter()
        .filter_map(|unit| {
            let was_enabled = before.disabled.binary_search(&unit).is_err();
            let is_enabled = after.disabled.binary_search(&unit).is_err();
            (was_enabled != is_enabled).then_some(RawEnabledAudit {
                unit,
                enabled: is_enabled,
            })
        })
        .collect();
    M3AuditChanges {
        output_inventories: changed_by_key(
            &before.raw_output_inventories,
            &after.raw_output_inventories,
            |v| v.unit,
        ),
        belts: changed_by_key(&before.raw_belts, &after.raw_belts, |v| v.first_piece),
        inserters: changed_by_key(&before.raw_inserters, &after.raw_inserters, |v| v.unit),
        extractors: changed_by_key(&before.raw_extractors, &after.raw_extractors, |v| v.unit),
        weapons: changed_by_key(&before.raw_weapons, &after.raw_weapons, |v| v.unit),
        healths: changed_by_key(&before.raw_healths, &after.raw_healths, |v| v.unit),
        research: changed_by_key(&before.research, &after.research, |v| {
            (v.slot, v.technology.clone())
        }),
        enabled,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawResearchAudit {
    pub slot: u8,
    pub technology: ContentId,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct M3AuditState {
    pub raw_output_inventories: Vec<RawOutputInventoryAudit>,
    pub raw_belts: Vec<RawBeltAudit>,
    pub raw_inserters: Vec<RawInserterAudit>,
    pub raw_extractors: Vec<RawExtractorAudit>,
    pub raw_weapons: Vec<RawWeaponAudit>,
    pub raw_healths: Vec<RawHealthAudit>,
    pub research: Vec<RawResearchAudit>,
    pub disabled: Vec<u32>,
    pub removed_units: Vec<u32>,
}
impl M3AuditState {
    pub fn is_empty(&self) -> bool {
        self.raw_output_inventories.is_empty()
            && self.raw_belts.is_empty()
            && self.raw_inserters.is_empty()
            && self.raw_extractors.is_empty()
            && self.raw_weapons.is_empty()
            && self.raw_healths.is_empty()
            && self.research.is_empty()
            && self.disabled.is_empty()
            && self.removed_units.is_empty()
    }
}
fn raw_wait(wait: ArmWait) -> RawInserterWaitAudit {
    match wait {
        ArmWait::NoSource => RawInserterWaitAudit::NoSource,
        ArmWait::NoDestination => RawInserterWaitAudit::NoDestination,
        ArmWait::SourceEmpty => RawInserterWaitAudit::SourceEmpty,
        ArmWait::DestinationFull => RawInserterWaitAudit::DestinationFull,
    }
}
fn raw_stage(stage: ArmStage) -> RawInserterStageAudit {
    match stage {
        ArmStage::AtSource(wait) => RawInserterStageAudit::AtSource(wait.map(raw_wait)),
        ArmStage::ToDestination => RawInserterStageAudit::ToDestination,
        ArmStage::AtDestination(wait) => RawInserterStageAudit::AtDestination(raw_wait(wait)),
        ArmStage::Returning => RawInserterStageAudit::Returning,
    }
}
fn output_inventory(inventory: &Inventory, unit: u32) -> RawOutputInventoryAudit {
    RawOutputInventoryAudit {
        unit,
        capacity: inventory.capacity,
        accepted_items: inventory.accepted_items.iter().cloned().collect(),
        items: inventory
            .items
            .iter()
            .map(|(item, account)| (item.clone(), *account))
            .collect(),
    }
}
/// Build stable audit records in source-map key order. Transient `RemovedThisTick` is excluded:
/// publication drains it outside a transaction; `RemovedHistory` is transaction-owned instead.
pub fn m3_audit_state(world: &World) -> M3AuditState {
    let mut state = M3AuditState::default();
    if let Some(ledger) = world.get_resource::<EconomyLedger>() {
        state.raw_output_inventories = ledger
            .output_inventories
            .iter()
            .map(|(unit, inv)| output_inventory(inv, *unit))
            .collect();
    }
    if let Some(conveyors) = world.get_resource::<Conveyors>() {
        let table = world.get_resource::<ItemTable>();
        state.raw_belts = conveyors
            .lines
            .iter()
            .map(|(id, line)| RawBeltAudit {
                first_piece: *id,
                definition: line.def.clone(),
                owner_slot: line.owner,
                slots_per_piece: line.slots_per_piece,
                slots_per_second: line.slots_per_second,
                rate_carry: line.carry.remainder(),
                raw_slots: line
                    .slots
                    .iter()
                    .map(|ix| RawBeltSlotAudit {
                        item_index: *ix,
                        item: table.and_then(|t| t.content(*ix).cloned()),
                    })
                    .collect(),
                pieces: line
                    .pieces
                    .iter()
                    .enumerate()
                    .map(|(i, unit)| {
                        let slots_per_piece = usize::from(line.slots_per_piece);
                        let start = i.saturating_mul(slots_per_piece);
                        let end = start.saturating_add(slots_per_piece).min(line.slots.len());
                        let piece_slots = line.slots.get(start..end).unwrap_or_default();
                        RawBeltPieceAudit {
                            unit: *unit,
                            slots: piece_slots
                                .iter()
                                .map(|ix| RawBeltSlotAudit {
                                    item_index: *ix,
                                    item: table.and_then(|t| t.content(*ix).cloned()),
                                })
                                .collect(),
                        }
                    })
                    .collect(),
            })
            .collect();
    }
    let disabled = world.get_resource::<Disabled>();
    if let Some(rows) = world.get_resource::<Inserters>() {
        state.raw_inserters = rows
            .0
            .iter()
            .map(|(unit, row)| RawInserterAudit {
                unit: *unit,
                enabled: !disabled.is_some_and(|d| d.0.contains(unit)),
                stage: raw_stage(row.stage),
                elapsed: row.elapsed,
                source: row.source,
                destination: row.destination,
            })
            .collect();
    }
    if let Some(rows) = world.get_resource::<Extractors>() {
        state.raw_extractors = rows
            .0
            .iter()
            .map(|(unit, row)| RawExtractorAudit {
                unit: *unit,
                enabled: !disabled.is_some_and(|d| d.0.contains(unit)),
                node: row.node.map(|n| n.0),
                rate_carry: row.carry.remainder(),
                status: match row.status {
                    ExtractorStatus::Extracting => RawExtractorStatusAudit::Extracting,
                    ExtractorStatus::Full => RawExtractorStatusAudit::Full,
                    ExtractorStatus::Depleted => RawExtractorStatusAudit::Depleted,
                },
            })
            .collect();
    }
    if let Some(rows) = world.get_resource::<Weapons>() {
        let catalog = world.get_resource::<InstalledCatalog>();
        let types = world.get_resource::<UnitTypes>();
        let research = world.get_resource::<ResearchState>();
        let owners: std::collections::BTreeMap<_, _> = world
            .iter_entities()
            .filter_map(|e| Some((e.get::<UnitId>()?.0, e.get::<Owner>()?.0.0)))
            .collect();
        let unit_types: std::collections::BTreeMap<_, _> = world
            .iter_entities()
            .filter_map(|e| Some((e.get::<UnitId>()?.0, e.get::<UnitType>()?.0)))
            .collect();
        state.raw_weapons = rows
            .0
            .iter()
            .map(|(unit, row)| {
                let effective_damage = (|| {
                    let owner = *owners.get(unit).ok_or(RejectReason::MissingCapability)?;
                    let kind = *unit_types
                        .get(unit)
                        .ok_or(RejectReason::MissingCapability)?;
                    let entry = types
                        .and_then(|rows| rows.get(kind))
                        .ok_or(RejectReason::MissingCapability)?;
                    let unit_id = ContentId::parse(&entry.content_id)
                        .map_err(|_| RejectReason::MissingCapability)?;
                    let catalog = catalog.ok_or(RejectReason::MissingCapability)?;
                    let definition = catalog
                        .0
                        .units
                        .get(&unit_id)
                        .ok_or(RejectReason::MissingCapability)?;
                    let weapon = definition
                        .weapon
                        .as_ref()
                        .ok_or(RejectReason::MissingCapability)?;
                    let research = research.ok_or(RejectReason::MissingCapability)?;
                    let add = modifier(research, &catalog.0, owner, &unit_id, WeaponStat::Damage)?;
                    weapon
                        .damage
                        .checked_add(add)
                        .ok_or(RejectReason::ArithmeticOverflow)
                })();
                RawWeaponAudit {
                    unit: *unit,
                    enabled: !disabled.is_some_and(|d| d.0.contains(unit)),
                    cooldown: row.cooldown,
                    effective_damage,
                    status: match row.status {
                        WeaponStatus::Ready => RawWeaponStatusAudit::Ready,
                        WeaponStatus::Cooling => RawWeaponStatusAudit::Cooling,
                        WeaponStatus::NoTarget => RawWeaponStatusAudit::NoTarget,
                        WeaponStatus::OutOfAmmo => RawWeaponStatusAudit::OutOfAmmo,
                    },
                    last_target: row.last_target,
                }
            })
            .collect();
    }
    if let Some(rows) = world.get_resource::<Healths>() {
        state.raw_healths = rows
            .0
            .iter()
            .map(|(unit, hp)| RawHealthAudit {
                unit: *unit,
                current: *hp,
            })
            .collect();
    }
    if let Some(rows) = world.get_resource::<ResearchState>() {
        state.research = rows
            .completed
            .iter()
            .map(|(slot, technology)| RawResearchAudit {
                slot: *slot,
                technology: technology.clone(),
            })
            .collect();
    }
    state.disabled = disabled.map_or_else(Vec::new, |rows| rows.0.iter().copied().collect());
    state.removed_units = world
        .get_resource::<RemovedHistory>()
        .map_or_else(Vec::new, |rows| rows.0.iter().copied().collect());
    state
}

/// Project M3 state for a live WorldView. Audit state keeps every health-table row so the
/// independent oracle can detect dangling records; the live view includes only existing units.
pub fn m3_world_view(world: &World) -> M3AuditState {
    let mut state = m3_audit_state(world);
    let live: std::collections::BTreeSet<_> = world
        .iter_entities()
        .filter_map(|entity| entity.get::<UnitId>().map(|unit| unit.0))
        .collect();
    state.raw_healths.retain(|row| live.contains(&row.unit));
    state.removed_units.clear();
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(capacity: u64) -> Inventory {
        Inventory {
            capacity,
            accepted_items: BTreeSet::new(),
            items: Default::default(),
        }
    }

    #[test]
    fn audit_rejects_state_change_between_operations_for_new_tables() {
        let mut app = crate::headless_app(20);
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        crate::economy::transaction::capture_operation(app.world_mut(), |_| ());
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .output_inventories
            .insert(77, inventory(10));
        crate::economy::transaction::capture_operation(app.world_mut(), |_| ());
        let operations = &app
            .world()
            .resource::<crate::economy::transaction::AuditTrail>()
            .operations;
        assert_eq!(operations.len(), 2);
        assert_ne!(
            operations[0].after.raw_output_inventories, operations[1].before.raw_output_inventories,
            "the real operation snapshots must expose the mutation between operations"
        );
    }

    #[test]
    fn helper_keeps_public_dto_damage_errors_and_sorted_rows() {
        let mut app = crate::headless_app(20);
        let mut ledger = app.world_mut().resource_mut::<EconomyLedger>();
        ledger.output_inventories.insert(77, inventory(10));
        ledger.output_inventories.insert(3, inventory(10));
        let state = m3_audit_state(app.world());
        assert_eq!(
            state
                .raw_output_inventories
                .iter()
                .map(|row| row.unit)
                .collect::<Vec<_>>(),
            [3, 77]
        );
        let _: Result<u64, RejectReason> = RawWeaponAudit {
            unit: 1,
            enabled: true,
            cooldown: 0,
            effective_damage: Err(RejectReason::ArithmeticOverflow),
            status: RawWeaponStatusAudit::Ready,
            last_target: None,
        }
        .effective_damage;
    }

    #[test]
    fn audit_keeps_dangling_health_but_live_view_has_no_corpse_row() {
        let mut app = crate::headless_app(20);
        app.world_mut().resource_mut::<Healths>().0.insert(99, 0);
        assert_eq!(
            m3_audit_state(app.world()).raw_healths,
            vec![RawHealthAudit {
                unit: 99,
                current: 0
            }]
        );
        assert!(m3_world_view(app.world()).raw_healths.is_empty());
    }

    #[test]
    fn raw_belt_audit_keeps_tail_slots_and_safely_projects_short_piece_rows() {
        use super::super::conveyor::{BeltLine, Conveyors};
        use crate::economy::gather::RateCarry;

        let mut app = crate::headless_app(20);
        let mut belts = Conveyors::default();
        belts.lines.insert(
            1,
            BeltLine {
                def: ContentId::parse("fixture:belt").unwrap(),
                owner: 0,
                pieces: vec![1],
                slots: vec![0, 1],
                slots_per_piece: 1,
                slots_per_second: 1,
                carry: RateCarry::default(),
            },
        );
        belts.lines.insert(
            2,
            BeltLine {
                def: ContentId::parse("fixture:belt").unwrap(),
                owner: 0,
                pieces: vec![2, 3],
                slots: vec![0],
                slots_per_piece: 1,
                slots_per_second: 1,
                carry: RateCarry::default(),
            },
        );
        app.world_mut().insert_resource(belts);

        let state = m3_audit_state(app.world());
        assert_eq!(state.raw_belts[0].raw_slots.len(), 2);
        assert_eq!(state.raw_belts[0].pieces[0].slots.len(), 1);
        assert_eq!(state.raw_belts[1].raw_slots.len(), 1);
        assert_eq!(state.raw_belts[1].pieces[1].slots.len(), 0);
    }

    #[test]
    fn weapon_audit_keeps_rows_when_catalog_metadata_is_missing() {
        let mut app = crate::headless_app(20);
        app.world_mut()
            .resource_mut::<Weapons>()
            .0
            .insert(99, super::super::combat::WeaponState::default());

        let state = m3_audit_state(app.world());
        assert_eq!(state.raw_weapons.len(), 1);
        assert_eq!(state.raw_weapons[0].unit, 99);
        assert_eq!(
            state.raw_weapons[0].effective_damage,
            Err(RejectReason::MissingCapability)
        );
    }
}
