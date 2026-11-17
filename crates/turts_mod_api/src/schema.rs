//! Package schema types for API 0.2 (0.2.4 adds the types of design 6.10, revision 6.2).

use crate::orders::CardCell;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use std::{collections::BTreeMap, fmt};

pub type ContentRef = String;

/// What a package is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageKind {
    Game,
    Library,
}

/// Where a package runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionContext {
    Server,
    Client,
    Shared,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Footprint {
    Circle { radius: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSize {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPos {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Shape {
    Capsule { radius: f32, height: f32 },
    Cuboid { x: f32, y: f32, z: f32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exports {
    pub units: Vec<String>,
    pub maps: Vec<String>,
    pub visuals: Vec<String>,
    pub resources: Vec<String>,
    pub nodes: Vec<String>,
    pub production: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub technologies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interface: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<String>,
    /// API 0.2.4, row A1: files of type `CombatDef`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combat: Vec<String>,
    /// API 0.2.4, row A2: files of type `MatchDef`. The key is `match` in RON.
    #[serde(default, rename = "match", skip_serializing_if = "Vec::is_empty")]
    pub match_files: Vec<String>,
    /// API 0.2.4, row A3: files of type `VisualEffectsDef`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_effects: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_materials: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_movement_classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_tools: Vec<String>,
    /// API 0.2.15: portable bot profiles. Loading a profile does not install it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bots: Vec<String>,
    /// API 0.2.18 (M6): underground utility types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub utilities: Vec<String>,
}

/// A map that rejects duplicate keys during deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct UniqueMap<K, V>(pub BTreeMap<K, V>);

impl<'de, K, V> Deserialize<'de> for UniqueMap<K, V>
where
    K: Deserialize<'de> + Ord + fmt::Display,
    V: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MapVisitor<K, V>(std::marker::PhantomData<(K, V)>);
        impl<'de, K, V> Visitor<'de> for MapVisitor<K, V>
        where
            K: Deserialize<'de> + Ord + fmt::Display,
            V: Deserialize<'de>,
        {
            type Value = UniqueMap<K, V>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map with unique keys")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut map = BTreeMap::new();
                while let Some((key, value)) = access.next_entry()? {
                    if map.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate map key"));
                    }
                }
                Ok(UniqueMap(map))
            }
        }
        deserializer.deserialize_map(MapVisitor(std::marker::PhantomData))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub version: String,
    pub kind: PackageKind,
    pub engine_api: String,
    pub dependencies: UniqueMap<String, String>,
    pub exports: Exports,
    pub context: ExecutionContext,
    pub capabilities: Vec<String>,
    pub license: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResourceKind {
    Balance,
    Item,
    /// API 0.2.4, row S1: a count of units that exist. A `Supply` resource has no balance.
    Supply {
        scope: SupplyScope,
        counts: SupplyCounts,
    },
}

/// Who shares a `Supply` resource. `Network` is a later API (design 6.10, "additive later").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SupplyScope {
    Slot,
    /// API 0.2.18 (M6): counted per connected network of the one utility type that carries it.
    Network,
}

/// What a `Supply` resource counts. `Queued` is reserved: the packer refuses it (row S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SupplyCounts {
    Completed,
    Queued,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDef {
    pub id: String,
    pub name: String,
    pub unit_name: String,
    pub kind: ResourceKind,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementDef {
    pub speed: f32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryDef {
    pub capacity: u64,
    pub accepts: Vec<ContentRef>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GathererDef {
    pub items: Vec<ContentRef>,
    pub gather_per_second: u64,
    pub unload_per_second: u64,
    pub interaction_range: f32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum DropOffEffect {
    Store,
    Convert { balance: ContentRef, per_item: u64 },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropOffRule {
    pub item: ContentRef,
    pub effect: DropOffEffect,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropOffDef {
    pub rules: Vec<DropOffRule>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerDef {
    pub recipes: Vec<ContentRef>,
    pub queue_capacity: u16,
    pub spawn_offsets: Vec<WorldPos>,
    /// API 0.2.4, row U6.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when_short: Vec<WhenShort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Facing {
    PosX,
    NegX,
    PosY,
    NegY,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractorDef {
    pub per_second: u64,
    pub interaction_range: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConveyorDef {
    pub slots_per_piece: u8,
    pub slots_per_second: u64,
    pub accepts: Vec<ContentRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InserterDef {
    pub reach: f32,
    pub swing_ms: u64,
    pub return_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRefunds {
    pub running: RefundRatio,
    pub output_blocked: RefundRatio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDef {
    pub id: String,
    pub name: String,
    pub inputs: Vec<ItemStack>,
    pub outputs: Vec<ItemStack>,
    pub duration_ms: u64,
    pub cancel_refunds: RunRefunds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineDef {
    pub recipe: ContentRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthDef {
    pub max: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SlotRelation {
    Other,
    Same,
    Any,
    /// API 0.2.4, row L1: a slot of the own team (the own slot included).
    Ally,
    /// API 0.2.4, row L1: a slot of another team.
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetFilter {
    pub slots: SlotRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmmoDef {
    pub item: ContentRef,
    pub per_shot: u64,
}

/// How a weapon measures its authored range (API 0.2.8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponRangeMode {
    #[default]
    Centre,
    Surface,
    /// Range to the target footprint, without adding the source radius (API 0.2.12).
    TargetSurface,
}

impl WeaponRangeMode {
    pub fn is_centre(&self) -> bool {
        *self == Self::Centre
    }

    /// Convert authored reach to center distance. Centre preserves the old arithmetic.
    pub fn centre_reach(self, range: f32, source_radius: f32, target_radius: f32) -> f32 {
        match self {
            Self::Centre => range,
            Self::TargetSurface => (range + target_radius).min(f32::MAX),
            Self::Surface => (range + source_radius + target_radius).min(f32::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDef {
    pub range: f32,
    #[serde(default, skip_serializing_if = "WeaponRangeMode::is_centre")]
    pub range_mode: WeaponRangeMode,
    pub cooldown_ms: u64,
    pub damage: u64,
    pub ammo: Option<AmmoDef>,
    pub targets: TargetFilter,
    /// API 0.2.4, row U2: a damage type of the combat table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<ContentRef>,
    /// API 0.2.12: additive rows that match a target class or tag.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bonus: Vec<DamageBonus>,
    /// API 0.2.4, row U6.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when_short: Vec<WhenShort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ContentsPolicy {
    Destroy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemovalDef {
    pub by_command: bool,
    pub contents: ContentsPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum WeaponStat {
    Damage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum EffectDef {
    ArmourModifier {
        unit: ContentRef,
        stat: ArmourStat,
        add: u64,
    },
    WeaponModifier {
        unit: ContentRef,
        stat: WeaponStat,
        add: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TechnologyDef {
    pub id: String,
    pub name: String,
    pub prerequisites: Vec<ContentRef>,
    pub duration_ms: u64,
    pub costs: Vec<Cost>,
    pub payment: PaymentPolicy,
    pub cancel_refunds: CancelRefunds,
    pub effects: Vec<EffectDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchStationDef {
    pub technologies: Vec<ContentRef>,
    pub queue_capacity: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeltLinePlacement {
    pub conveyor: ContentRef,
    pub owner_slot: u8,
    pub start: WorldPos,
    pub facing: Facing,
    pub pieces: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_class: Option<ContentRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_cargo: Option<crate::TerrainCargoDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_foundation: Option<crate::TerrainFoundation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_tools: Vec<ContentRef>,
    /// API16: completed-unit sight; absence contributes no sight in fog mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vision: Option<crate::visibility::VisionDef>,
    pub id: String,
    pub name: String,
    pub footprint: Footprint,
    #[serde(deserialize_with = "required_option")]
    pub movement: Option<MovementDef>,
    #[serde(deserialize_with = "required_option")]
    pub inventory: Option<InventoryDef>,
    #[serde(deserialize_with = "required_option")]
    pub gatherer: Option<GathererDef>,
    #[serde(deserialize_with = "required_option")]
    pub drop_off: Option<DropOffDef>,
    #[serde(deserialize_with = "required_option")]
    pub producer: Option<ProducerDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_inventory: Option<InventoryDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extractor: Option<ExtractorDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conveyor: Option<ConveyorDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inserter: Option<InserterDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<MachineDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub research: Option<ResearchStationDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon: Option<WeaponDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removal: Option<RemovalDef>,
    /// API 0.2.4, row U1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armour: Option<ArmourDef>,
    /// API 0.2.4, row U4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquire: Option<AcquireDef>,
    /// API 0.2.4, row U5.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<SupplyAmount>,
    /// API 0.2.4, row U5.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<SupplyAmount>,
    /// API 0.2.4, row U7.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub income: Option<IncomeDef>,
    /// API 0.2.4, row U8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_order: Option<StartOrder>,
    /// API 0.2.4, row K1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constructor: Option<ConstructorDef>,
    /// API 0.2.4, row K2: the unit can be built on a site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<SitePlacementDef>,
    /// API 0.2.4, row K3: the radius around the unit in which sites may stand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_area: Option<f32>,
    /// API 0.2.4, row R1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repairer: Option<RepairerDef>,
}

/// API 0.2.4, row U1: an armour class and a flat reduction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmourDef {
    pub class: ContentRef,
    #[serde(default)]
    pub flat: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<ContentRef>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub points: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flat_by_type: Vec<(ContentRef, u64)>,
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

/// API 0.2.12: one additive weapon bonus against a target class or tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageBonus {
    pub against: ContentRef,
    pub add: u64,
}

/// API 0.2.12: the optional points-based armour curve.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ArmourCurve {
    #[default]
    None,
    Ratio {
        k: Ratio,
    },
    Percent {
        per_point: Ratio,
        #[serde(default = "RefundRatio::one")]
        cap: Ratio,
    },
}

impl ArmourCurve {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// API 0.2.12: a slot-owned armour upgrade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ArmourStat {
    Flat,
    Points,
    FlatByType(ContentRef),
}

/// API 0.2.4, row U4: how far an idle unit looks for a target, and how far it chases one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquireDef {
    pub range: f32,
    pub leash: f32,
}

/// API 0.2.4, row U5: an amount of a `Supply` resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyAmount {
    pub resource: ContentRef,
    pub amount: u64,
}

/// API 0.2.4, row U7: `amount` of a balance resource in each period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomeDef {
    pub resource: ContentRef,
    pub amount: u64,
    pub period_ms: u32,
}

/// API 0.2.4, rows U8 and M3: an order that a unit gets when it spawns or is placed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartOrder {
    pub order: ContentRef,
    pub target: StartTarget,
    #[serde(default)]
    pub ground_rally: GroundRally,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum StartTarget {
    Node(u32),
    Point(WorldPos),
    NearestNode,
}

/// Does a ground rally apply to the spawn order? (design 6.10 row U8)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum GroundRally {
    /// The spawn order ignores a ground rally.
    #[default]
    Ignore,
    /// The ground rally follow order comes first, and the spawn order is queued after it.
    ApplyFirst,
}

/// API 0.2.4, row U6: what a short supply does to one capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhenShort {
    pub resource: ContentRef,
    pub effect: ShortEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ShortEffect {
    /// Progress runs at the ratio (producer and constructor only).
    Rate(Ratio),
    /// The weapon neither fires nor acquires (weapon only).
    Disable,
    /// Reserved: the packer refuses it.
    BlockStart,
}

/// API 0.2.4, row K1: a unit that places sites for buildings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructorDef {
    pub buildings: Vec<ContentRef>,
    pub concurrent_sites: u16,
    pub queue_capacity: u16,
    #[serde(default)]
    pub progress: ConstructProgress,
    #[serde(default)]
    pub when_short: Vec<WhenShort>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ConstructProgress {
    #[default]
    Autonomous,
    /// Reserved: the packer refuses it.
    NeedsBuilder { reach: f32 },
}

/// API 0.2.4, row K2: where a unit can stand as a building.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SitePlacementDef {
    pub gap: f32,
    #[serde(default)]
    pub node_clearance: f32,
    #[serde(default)]
    pub in_build_area: bool,
    #[serde(default = "RefundRatio::one")]
    pub site_health: Ratio,
}

/// API 0.2.4, row R1: a unit that heals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairerDef {
    pub rate_per_second: u64,
    pub reach: f32,
    pub cost: Vec<RepairCost>,
    pub targets: TargetFilter,
    #[serde(default)]
    pub classes: Vec<ContentRef>,
    #[serde(default)]
    pub auto_range: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairCost {
    pub resource: ContentRef,
    pub amount: u64,
    pub per_health: u64,
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeDef {
    pub id: String,
    pub name: String,
    pub item: ContentRef,
    pub radius: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CostSource {
    SlotBalance,
    ProducerInventory,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    pub resource: ContentRef,
    pub amount: u64,
    pub source: CostSource,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentPolicy {
    ChargeOnQueue,
    ReserveThenCharge,
    /// API 0.2.4, row P1: the cost is charged while the job runs (design 6.4).
    ChargeOverTime,
}
/// A ratio `numerator / denominator`. The packer refuses a denominator of 0 (design 6.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundRatio {
    pub numerator: u32,
    pub denominator: u32,
}

/// The `Ratio` of design 6.10: the shape of `RefundRatio` (one type, one definition).
pub type Ratio = RefundRatio;

impl RefundRatio {
    /// 0/1: nothing.
    pub const ZERO: RefundRatio = RefundRatio {
        numerator: 0,
        denominator: 1,
    };
    /// 1/1: everything.
    pub const ONE: RefundRatio = RefundRatio {
        numerator: 1,
        denominator: 1,
    };

    /// 1/1, as a function for `#[serde(default = ...)]`.
    pub fn one() -> RefundRatio {
        Self::ONE
    }

    /// True for 0/1 only: the form that `skip_serializing_if` leaves out.
    pub fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }
}

/// 0/1: no refund (the `destroyed` default of design 6.10 row K5).
impl Default for RefundRatio {
    fn default() -> Self {
        Self::ZERO
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRefunds {
    pub queued: RefundRatio,
    pub running: RefundRatio,
    pub output_blocked: RefundRatio,
    /// API 0.2.4, row K5: the refund when the producer or the site is destroyed.
    #[serde(default, skip_serializing_if = "RefundRatio::is_zero")]
    pub destroyed: RefundRatio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionDef {
    pub id: String,
    pub name: String,
    pub output_unit: ContentRef,
    pub duration_ms: u64,
    pub costs: Vec<Cost>,
    pub payment: PaymentPolicy,
    pub cancel_refunds: CancelRefunds,
    /// API 0.2.4, row O8: an input action name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
    /// API 0.2.4, row O10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<CardCell>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemStack {
    pub item: ContentRef,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BalanceAmount {
    pub balance: ContentRef,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub unit: ContentRef,
    pub owner_slot: u8,
    pub pos: WorldPos,
    pub contents: Vec<ItemStack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facing: Option<Facing>,
    /// API 0.2.4, row M3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_order: Option<StartOrder>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodePlacement {
    pub node: ContentRef,
    pub pos: WorldPos,
    pub remaining: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotEconomyInit {
    pub slot: u8,
    pub balances: Vec<BalanceAmount>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<crate::TerrainDef>,
    /// API16: map knowledge policy; absence preserves AllVisible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<crate::visibility::VisibilityDef>,
    /// Optional client presentation. It never changes the authoritative map dimensions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentDef>,
    /// Optional ground presentation. It never participates in simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<GroundDef>,
    pub id: String,
    pub size: MapSize,
    pub placements: Vec<Placement>,
    pub nodes: Vec<NodePlacement>,
    #[serde(deserialize_with = "unique_slot_economy")]
    pub slot_economy: Vec<SlotEconomyInit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub belts: Vec<BeltLinePlacement>,
}

fn unique_slot_economy<'de, D>(deserializer: D) -> Result<Vec<SlotEconomyInit>, D::Error>
where
    D: Deserializer<'de>,
{
    let entries = Vec::<SlotEconomyInit>::deserialize(deserializer)?;
    let mut slots = std::collections::BTreeSet::new();
    let mut balances = std::collections::BTreeSet::new();
    for entry in &entries {
        if !slots.insert(entry.slot) {
            return Err(serde::de::Error::custom("duplicate slot economy entry"));
        }
        for balance in &entry.balances {
            if !balances.insert((entry.slot, balance.balance.as_str())) {
                return Err(serde::de::Error::custom("duplicate balance seed for slot"));
            }
        }
    }
    Ok(entries)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualTarget {
    Unit(ContentRef),
    Node(ContentRef),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualDef {
    pub target: VisualTarget,
    pub shape: Shape,
    /// API 0.2.4, row V2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelDef>,
    /// API 0.2.4, row V4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<VisualEffectRefs>,
}

/// API 0.2.4, row V2: a 3D model for a visual. The three names default to `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDef {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lods: Vec<ModelLod>,
    #[serde(default = "model_scale")]
    pub scale: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animations: Option<ModelAnimations>,
    pub asset: ContentRef,
    #[serde(default)]
    pub team_material: Option<String>,
    #[serde(default)]
    pub turret_node: Option<String>,
    #[serde(default)]
    pub muzzle_node: Option<String>,
}

fn model_scale() -> f32 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelLod {
    pub asset: ContentRef,
    pub below_screen_height: f32,
}

/// A named clip, with optional timing and movement metadata. Legacy strings still load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnimationClipDef {
    Name(String),
    Settings(AnimationClipSettings),
}
impl AnimationClipDef {
    pub fn clip(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Settings(settings) => &settings.clip,
        }
    }
    pub fn settings(&self) -> Option<&AnimationClipSettings> {
        match self {
            Self::Name(_) => None,
            Self::Settings(settings) => Some(settings),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationClipSettings {
    pub clip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stride: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_at: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<String>,
    #[serde(default)]
    pub yaw_offset_deg: f32,
}
/// Client animation selection. Missing actions fall back to idle, then the bind pose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ModelAnimations {
    pub idle: Option<AnimationClipDef>,
    pub r#move: Option<AnimationClipDef>,
    pub attack: Option<AnimationClipDef>,
    pub death: Option<AnimationClipDef>,
    pub work: Option<AnimationClipDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit: Option<AnimationClipDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundDef {
    /// API 0.2.11: a bounded, client-only colour and texture layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<GroundOverlayDef>,
    pub base_colour: ContentRef,
    #[serde(default)]
    pub normal: Option<ContentRef>,
    pub tile_size: f32,
}

/// A second ground layer. Masks use world coordinates, independent of ground mesh topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundOverlayDef {
    pub colour: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_colour: Option<ContentRef>,
    pub tile_size: f32,
    #[serde(default = "ground_layer_one")]
    pub roughness: f32,
    pub patches: Vec<GroundPatchDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundPatchDef {
    pub centre: WorldPos,
    pub size: MapSize,
    #[serde(default)]
    pub rotation_deg: f32,
    pub feather: f32,
    #[serde(default = "ground_layer_one")]
    pub opacity: f32,
}
fn ground_layer_one() -> f32 {
    1.0
}

/// API 0.2.10: map-owned environment settings. These values never affect simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentDef {
    pub background: String,
    #[serde(default)]
    pub ground_margin: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_colour: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fog: Option<DistanceFogDef>,
    #[serde(default)]
    pub antialiasing: Antialiasing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistanceFogDef {
    pub colour: String,
    pub start: f32,
    pub end: f32,
    #[serde(default = "default_fog_opacity")]
    pub opacity: f32,
}
fn default_fog_opacity() -> f32 {
    0.2
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Antialiasing {
    Off,
    #[default]
    Fxaa,
}

/// API 0.2.4, row V4: the effects of a unit visual, each a reference to an effect of the
/// visual effects file (`VisualEffectsDef`). `hit_reduced` absent uses `hit`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualEffectRefs {
    #[serde(default)]
    pub fire: Option<ContentRef>,
    #[serde(default)]
    pub hit: Option<ContentRef>,
    #[serde(default)]
    pub hit_reduced: Option<ContentRef>,
    #[serde(default)]
    pub death: Option<ContentRef>,
    #[serde(default)]
    pub damaged: Option<crate::effects::DamagedEffects>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile: Option<ContentRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<ContentRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<ContentRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamagedEffect {
    pub effect: ContentRef,
    pub below: Ratio,
}

/// API 0.2.4, row V3: the file of `exports.visual_effects`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualEffectsDef {
    pub effects: Vec<VisualEffectDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualEffectDef {
    pub id: String,
    #[serde(default)]
    pub kind: EffectKind,
    #[serde(default = "effect_white")]
    pub colour: String,
    #[serde(default = "effect_size")]
    pub size: f32,
    #[serde(default = "effect_ms")]
    pub ms: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<crate::effects::EffectLayer>,
    #[serde(default, skip_serializing_if = "crate::effects::EffectCost::is_small")]
    pub cost: crate::effects::EffectCost,
    #[serde(default)]
    pub sound: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub enum EffectKind {
    #[default]
    Flash,
    Spark,
    Tracer,
    Burst,
    Smoke,
}

fn effect_white() -> String {
    "#ffffff".into()
}
fn effect_size() -> f32 {
    1.
}
fn effect_ms() -> u32 {
    100
}

/// API 0.2.4, rows C1 to C4: the file of `exports.combat`, the damage table of a match. It has
/// no `id` and no `name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatDef {
    pub damage_types: Vec<String>,
    pub armour_classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "ArmourCurve::is_none")]
    pub curve: ArmourCurve,
    pub cells: Vec<DamageCell>,
    pub minimum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageCell {
    pub damage: ContentRef,
    pub armour: ContentRef,
    pub ratio: Ratio,
}

/// API 0.2.4, rows A2, K6, M1, M2 (design 6.7.4): the file of `exports.match`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchDef {
    /// Absent: `Immediate`.
    #[serde(default)]
    pub start: StartPolicy,
    /// Absent: the engine never ends the match (D28).
    #[serde(default)]
    pub victory: Option<VictoryRule>,
    /// The site cap of design 6.1.
    #[serde(default)]
    pub open_sites_per_slot: Option<u16>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum StartPolicy {
    /// Boot goes to Running; claims happen at `Hello`.
    #[default]
    Immediate,
    AllSeatedReady {
        countdown_secs: u32,
    },
    AllSeated {
        countdown_secs: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum VictoryRule {
    LastTeamStanding { defeat: Vec<DefeatClause> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum DefeatClause {
    KeyUnitsLost { units: Vec<ContentRef> },
    DisconnectedFor { secs: u32 },
}
