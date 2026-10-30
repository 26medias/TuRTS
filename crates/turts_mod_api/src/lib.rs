//! Public mod API: content ids, package schema types, the UnitRegistry trait, ModError.

pub mod assets;
pub mod content;
pub mod error;
pub mod ids;
pub mod input;
pub mod interface;
pub mod lock;
pub mod orders;
pub mod registry;
pub mod schema;

pub use assets::{AssetsDef, CursorAsset, FileAsset, ImageAsset};
pub use content::{
    ContentCatalog, ContentRef, ResolvedAmmo, ResolvedConveyor, ResolvedCost, ResolvedDropOff,
    ResolvedDropOffEffect, ResolvedDropOffRule, ResolvedEffect, ResolvedGatherer,
    ResolvedInventory, ResolvedItemStack, ResolvedMachine, ResolvedNode, ResolvedProducer,
    ResolvedProduction, ResolvedRecipe, ResolvedResearchStation, ResolvedTechnology, ResolvedUnit,
    ResolvedWeapon, SourceDef,
};
pub use error::ModError;
pub use ids::{ContentId, PackageId, UnitTypeId};
pub use input::{
    ActionDef, Binding, Button, EngineOverride, InputMapDef, KNOWN_KEY_CODES, Mod,
    RESERVED_BROWSER_KEYS, WheelDirection,
};
pub use interface::{
    AlertDef, AlertTrigger, Anchor, At, ButtonAction, ButtonIcon, CameraDef, CameraStart,
    Dimension, Direction, Fill, GuidePage, InterfaceDef, ItemField, LightDef,
    Placement as WidgetPlacement, Predicate, PrimaryField, Primitive, Size, Source, TextAlign,
    TextValue, TooltipStyle, UnitPresentation, Value, Widget, WorldPoint, WorldStyle,
};
pub use lock::{LockEntry, Lockfile};
pub use orders::{
    Behaviour, Capability, CardCell, DEFAULT_QUEUE_DEPTH, EngagePolicy, FirePolicy, OrderDef,
    OrderRulesDef, SmartOrder, SmartTarget, TargetKind,
};
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
pub const ENGINE_API_VERSION: &str = "0.2.3";

/// Engine API versions released by the project, in ascending order.
pub const RELEASED_ENGINE_APIS: &[&str] = &["0.1.0", "0.2.0", "0.2.1", "0.2.2", "0.2.3"];

/// Capabilities that package manifests may request.
pub const KNOWN_CAPABILITIES: &[&str] = &[];

#[cfg(test)]
mod tests {
    use super::{DEFAULT_QUEUE_DEPTH, ENGINE_API_VERSION, RELEASED_ENGINE_APIS};

    #[test]
    fn released_list_ends_with_engine_version() {
        assert_eq!(
            RELEASED_ENGINE_APIS.last().copied(),
            Some(ENGINE_API_VERSION)
        );
    }

    #[test]
    fn order_queue_default_is_sixteen() {
        assert_eq!(DEFAULT_QUEUE_DEPTH, 16);
    }
}
