//! Public mod API: content ids, package schema types, the UnitRegistry trait, ModError.

pub mod content;
pub mod error;
pub mod ids;
pub mod lock;
pub mod registry;
pub mod schema;

pub use content::{
    ContentCatalog, ContentRef, ResolvedCost, ResolvedDropOff, ResolvedDropOffEffect,
    ResolvedDropOffRule, ResolvedGatherer, ResolvedInventory, ResolvedNode, ResolvedProducer,
    ResolvedProduction, ResolvedUnit, SourceDef,
};
pub use error::ModError;
pub use ids::{ContentId, PackageId, UnitTypeId};
pub use lock::{LockEntry, Lockfile};
pub use registry::UnitRegistry;
pub use schema::{
    BalanceAmount, CancelRefunds, Cost, CostSource, DropOffDef, DropOffEffect, DropOffRule,
    ExecutionContext, Exports, Footprint, GathererDef, InventoryDef, ItemStack, Manifest, MapDef,
    MapSize, MovementDef, NodeDef, NodePlacement, PackageKind, PaymentPolicy, Placement,
    ProducerDef, ProductionDef, RefundRatio, ResourceDef, ResourceKind, Shape, SlotEconomyInit,
    UniqueMap, UnitDef, VisualDef, VisualTarget, WorldPos,
};

/// Version of the engine API that this crate defines.
pub const ENGINE_API_VERSION: &str = "0.2.0";
