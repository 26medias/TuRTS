//! Public mod API: content ids, package schema types, the UnitRegistry trait, ModError.

pub mod content;
pub mod error;
pub mod ids;
pub mod lock;
pub mod registry;
pub mod schema;

pub use content::{
    ContentCatalog, ContentRef, ResolvedAmmo, ResolvedConveyor, ResolvedCost, ResolvedDropOff,
    ResolvedDropOffEffect, ResolvedDropOffRule, ResolvedEffect, ResolvedGatherer,
    ResolvedInventory, ResolvedItemStack, ResolvedMachine, ResolvedNode, ResolvedProducer,
    ResolvedProduction, ResolvedRecipe, ResolvedResearchStation, ResolvedTechnology, ResolvedUnit,
    ResolvedWeapon, SourceDef,
};
pub use error::ModError;
pub use ids::{ContentId, PackageId, UnitTypeId};
pub use lock::{LockEntry, Lockfile};
pub use registry::UnitRegistry;
pub use schema::{
    AmmoDef, BalanceAmount, BeltLinePlacement, CancelRefunds, ContentsPolicy, ConveyorDef, Cost,
    CostSource, DropOffDef, DropOffEffect, DropOffRule, EffectDef, ExecutionContext, Exports,
    ExtractorDef, Facing, Footprint, GathererDef, HealthDef, InserterDef, InventoryDef, ItemStack,
    MachineDef, Manifest, MapDef, MapSize, MovementDef, NodeDef, NodePlacement, PackageKind,
    PaymentPolicy, Placement, ProducerDef, ProductionDef, RecipeDef, RefundRatio, RemovalDef,
    ResearchStationDef, ResourceDef, ResourceKind, RunRefunds, Shape, SlotEconomyInit,
    SlotRelation, TargetFilter, TechnologyDef, UniqueMap, UnitDef, VisualDef, VisualTarget,
    WeaponDef, WeaponStat, WorldPos,
};

/// Version of the engine API that this crate defines.
pub const ENGINE_API_VERSION: &str = "0.2.2";

/// Engine API versions released by the project, in ascending order.
pub const RELEASED_ENGINE_APIS: &[&str] = &["0.1.0", "0.2.0", "0.2.1", "0.2.2"];

/// Capabilities that package manifests may request.
pub const KNOWN_CAPABILITIES: &[&str] = &[];

#[cfg(test)]
mod tests {
    use super::{ENGINE_API_VERSION, RELEASED_ENGINE_APIS};

    #[test]
    fn released_list_ends_with_engine_version() {
        assert_eq!(
            RELEASED_ENGINE_APIS.last().copied(),
            Some(ENGINE_API_VERSION)
        );
    }
}
