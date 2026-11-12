//! Public mod API: content ids, package schema types, the UnitRegistry trait, ModError.

pub mod assets;
pub mod content;
pub mod error;
pub mod ids;
pub mod input;
pub mod interface;
pub mod keys;
pub mod lock;
pub mod orders;
pub mod registry;
pub mod schema;
pub mod written;

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
    ActionField, AlertDef, AlertKind, AlertTrigger, Anchor, At, BarStyle, ButtonAction, ButtonIcon,
    CameraDef, CameraStart, Dimension, Direction, Fill, FloatTextStyle, GuidePage, HintCondition,
    HintDef, HintKind, HintTarget, IconField, InterfaceDef, ItemField, LabelShow, LabelStyle,
    LightDef, LineStyle, OutcomeKind, OutcomeReasonKind, OwnUnitState,
    Placement as WidgetPlacement, PlacementStyle, Predicate, PrimaryField, Primitive, Size,
    SoundWhen, Source, StatField, SupplySide, TextAlign, TextValue, TooltipStyle, UnitPresentation,
    Value, Widget, WorldPoint, WorldStyle,
};
pub use lock::{LockEntry, Lockfile};
pub use orders::{
    Behaviour, Capability, CardCell, DEFAULT_QUEUE_DEPTH, EngagePolicy, FirePolicy, OrderDef,
    OrderRulesDef, SmartOrder, SmartTarget, TargetKind,
};
pub use registry::UnitRegistry;
pub use schema::{
    AcquireDef, AmmoDef, ArmourDef, BalanceAmount, BeltLinePlacement, CancelRefunds, CombatDef,
    ConstructProgress, ConstructorDef, ContentsPolicy, ConveyorDef, Cost, CostSource, DamageCell,
    DamagedEffect, DefeatClause, DropOffDef, DropOffEffect, DropOffRule, EffectDef, EffectKind,
    ExecutionContext, Exports, ExtractorDef, Facing, Footprint, GathererDef, GroundRally,
    HealthDef, IncomeDef, InserterDef, InventoryDef, ItemStack, MachineDef, Manifest, MapDef,
    MapSize, MatchDef, ModelDef, MovementDef, NodeDef, NodePlacement, PackageKind, PaymentPolicy,
    Placement, ProducerDef, ProductionDef, Ratio, RecipeDef, RefundRatio, RemovalDef, RepairCost,
    RepairerDef, ResearchStationDef, ResourceDef, ResourceKind, RunRefunds, Shape, ShortEffect,
    SitePlacementDef, SlotEconomyInit, SlotRelation, StartOrder, StartPolicy, StartTarget,
    SupplyAmount, SupplyCounts, SupplyScope, TargetFilter, TechnologyDef, UniqueMap, UnitDef,
    VictoryRule, VisualDef, VisualEffectDef, VisualEffectRefs, VisualEffectsDef, VisualTarget,
    WeaponDef, WeaponStat, WhenShort, WorldPos,
};

pub use written::Written;

/// Version of the engine API that this crate defines.
pub const ENGINE_API_VERSION: &str = "0.2.4";

/// Engine API versions released by the project, in ascending order.
pub const RELEASED_ENGINE_APIS: &[&str] = &["0.1.0", "0.2.0", "0.2.1", "0.2.2", "0.2.3", "0.2.4"];

/// The number of player slots of a match, which is the number of seats that the lobby offers
/// (design 6.7.5, "The seat count is the player-slot count"; 2 in MP1). The simulation, the server
/// and the packer read this one constant; a match file or map file does not declare it (design
/// 6.10 row H15: one different colour for each seat).
pub const SLOT_COUNT: u8 = 2;

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

    /// Design D:2280-2286: 0.2.4 is the one API step of MP1, released with its first row.
    #[test]
    fn the_api_step_of_mp1_is_0_2_4() {
        assert_eq!(ENGINE_API_VERSION, "0.2.4");
        assert_eq!(RELEASED_ENGINE_APIS.len(), 6);
        assert_eq!(RELEASED_ENGINE_APIS[4], "0.2.3");
    }

    #[test]
    fn order_queue_default_is_sixteen() {
        assert_eq!(DEFAULT_QUEUE_DEPTH, 16);
    }
}
