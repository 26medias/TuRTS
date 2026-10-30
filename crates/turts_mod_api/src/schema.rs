//! Package schema types for API 0.2.

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

/// A map that rejects duplicate keys during deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
pub enum ResourceKind {
    Balance,
    Item,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDef {
    pub range: f32,
    pub cooldown_ms: u64,
    pub damage: u64,
    pub ammo: Option<AmmoDef>,
    pub targets: TargetFilter,
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
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundRatio {
    pub numerator: u32,
    pub denominator: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRefunds {
    pub queued: RefundRatio,
    pub running: RefundRatio,
    pub output_blocked: RefundRatio,
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
}
