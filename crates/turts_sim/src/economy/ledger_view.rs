//! The ledger view of one audit snapshot (pre-MP1 slice, task A1b).
//!
//! `AuditState` (`views.rs`) is the in-process test snapshot. It keeps all its fields, because the
//! frozen M2 stream compares the full `AuditState` before and after each operation. The economy
//! log audit (`turts_audit`) reads only `LedgerAudit`, and the log will write only `LedgerAudit`
//! (task A1c). So a new sim structure enters the log format only through an edit of this file.
//!
//! Each field has the name, the type and the serde form of the `AuditState` field that it copies,
//! so the JSON of a field is the same in both views. The readers of the log need these fields:
//! - the ledger domains of `turts_audit`: accounts, inventories, outputs, belts, inserters,
//!   extractors, weapons, healths, nodes, ledger claims, queues and jobs, research, the catalog,
//!   units (id, type, owner), removed units, the clock, the event ordinals, the pending events,
//!   the slot claims and the id allocators;
//! - `tests/browser/m2-audit.js`: `economy` (the units with `pos` and `facing`, the nodes with
//!   `pos`, the balances, the inventories, the gather orders with `stage`, the queues);
//! - `crates/turts_server/tests/el1.rs`: `raw_units`, `movement_progress`,
//!   `plain_movement_progress`, `physical_escapes`, `unit_orders` and `movement_pools`.
//!
//! `AuditState` fields that the log does not need, and that `LedgerAudit` does not hold: `bounds`,
//! `resources`, `pending_movement_orders`, `movement_dispatch`, `compiled_durations`,
//! `unit_types` and `units`.

use serde::{Deserialize, Serialize};

use super::views::{
    AuditState, CatalogAudit, RawInventoryAudit, RawMovementProgressAudit, RawOrderAudit,
    RawPhysicalEscapeAudit, RawPlainMovementProgressAudit, RawQueueAudit, RawUnitAudit,
    RawUnitOrdersAudit, ReservationClaimView, WorldView,
};
use super::views_m3::{
    RawBeltAudit, RawExtractorAudit, RawHealthAudit, RawInserterAudit, RawOutputInventoryAudit,
    RawResearchAudit, RawWeaponAudit,
};
use crate::ids::IdAllocators;

/// The `AuditState` fields that `LedgerAudit` does not copy. A test compares the JSON of both
/// views with these keys removed.
pub const LEDGER_DROPPED_FIELDS: [&str; 7] = [
    "bounds",
    "resources",
    "pending_movement_orders",
    "movement_dispatch",
    "compiled_durations",
    "unit_types",
    "units",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerAudit {
    pub tick_hz: Option<u16>,
    pub raw_units: Vec<RawUnitAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub movement_progress: Vec<RawMovementProgressAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plain_movement_progress: Vec<RawPlainMovementProgressAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub physical_escapes: Vec<RawPhysicalEscapeAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub movement_pools: Vec<crate::movement::MovementPool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unit_orders: Vec<RawUnitOrdersAudit>,
    pub raw_inventories: Vec<RawInventoryAudit>,
    pub raw_orders: Vec<RawOrderAudit>,
    pub raw_queues: Vec<RawQueueAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_output_inventories: Vec<RawOutputInventoryAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_belts: Vec<RawBeltAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_inserters: Vec<RawInserterAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_extractors: Vec<RawExtractorAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_weapons: Vec<RawWeaponAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_healths: Vec<RawHealthAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub research: Vec<RawResearchAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_units: Vec<u32>,
    pub next_event_ordinal: Option<u64>,
    pub pending_events: Vec<super::events::EconomyEvent>,
    pub catalog: Option<CatalogAudit>,
    pub ledger_claims: Vec<ReservationClaimView>,
    pub nodes: Vec<super::views::NodeView>,
    pub economy: WorldView,
    pub allocators: IdAllocators,
    pub claims: Vec<Option<u64>>,
}

/// The one function from the audit snapshot to the ledger view.
pub fn ledger_audit(state: &AuditState) -> LedgerAudit {
    LedgerAudit {
        tick_hz: state.tick_hz,
        raw_units: state.raw_units.clone(),
        movement_progress: state.movement_progress.clone(),
        plain_movement_progress: state.plain_movement_progress.clone(),
        physical_escapes: state.physical_escapes.clone(),
        movement_pools: state.movement_pools.clone(),
        unit_orders: state.unit_orders.clone(),
        raw_inventories: state.raw_inventories.clone(),
        raw_orders: state.raw_orders.clone(),
        raw_queues: state.raw_queues.clone(),
        raw_output_inventories: state.raw_output_inventories.clone(),
        raw_belts: state.raw_belts.clone(),
        raw_inserters: state.raw_inserters.clone(),
        raw_extractors: state.raw_extractors.clone(),
        raw_weapons: state.raw_weapons.clone(),
        raw_healths: state.raw_healths.clone(),
        research: state.research.clone(),
        disabled: state.disabled.clone(),
        removed_units: state.removed_units.clone(),
        next_event_ordinal: state.next_event_ordinal,
        pending_events: state.pending_events.clone(),
        catalog: state.catalog.clone(),
        ledger_claims: state.ledger_claims.clone(),
        nodes: state.nodes.clone(),
        economy: state.economy.clone(),
        allocators: state.allocators,
        claims: state.claims.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{LEDGER_DROPPED_FIELDS, ledger_audit};
    use crate::{MoveTarget, Owner, Position, SlotId, UnitId, UnitOrders, UnitType, Vec2};
    use turts_mod_api::{Footprint, PackageId, UnitDef, UnitRegistry};

    /// Every field that `LedgerAudit` keeps has the JSON of the `AuditState` field. The test
    /// fails when a field is dropped, renamed or copied from the wrong source.
    #[test]
    fn ledger_view_json_is_the_audit_json_without_the_dropped_fields() {
        let mut app = crate::headless_app(10);
        let kind = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &PackageId("probe".into()),
                UnitDef {
                    id: "walker".into(),
                    name: "Walker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    footprint: Footprint::Circle { radius: 0.5 },
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            )
            .unwrap();
        app.world_mut().spawn((
            UnitId(4),
            UnitType(kind),
            Owner(SlotId(1)),
            Position(Vec2::new(1.5, -2.25)),
            MoveTarget(Vec2::new(10.0, 0.0)),
            UnitOrders::default(),
        ));
        let state = crate::audit_state(app.world_mut());
        assert_eq!(
            state.raw_units.len(),
            1,
            "the probe unit is in the snapshot"
        );
        let ledger = serde_json::to_value(ledger_audit(&state)).unwrap();
        let mut audit = serde_json::to_value(&state).unwrap();
        let object = audit.as_object_mut().unwrap();
        for field in LEDGER_DROPPED_FIELDS {
            object.remove(field);
            assert!(
                ledger.get(field).is_none(),
                "the ledger view does not hold the dropped field {field}"
            );
        }
        assert_eq!(ledger, audit);
        let units = &ledger["economy"]["units"];
        assert_eq!(
            units[0]["unit_id"], 4,
            "the browser oracle reads economy.units"
        );
        assert!(
            units[0]["pos"].is_array(),
            "the browser oracle reads the unit pos"
        );
        assert_eq!(ledger["raw_units"][0]["unit_id"], 4, "EL1 reads raw_units");
    }
}
