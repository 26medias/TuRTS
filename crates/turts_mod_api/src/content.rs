//! Typed API 0.2 references and resolved content catalog entries.

use std::collections::BTreeMap;

use crate::{ContentId, schema::*};

/// A qualified reference as authored in RON. Runtime validation resolves this into `ContentId`.
pub type ContentRef = String;

/// A definition paired with its archive path for source-aware diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceDef<T> {
    pub file: String,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedInventory {
    pub capacity: u64,
    pub accepts: Vec<ContentId>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGatherer {
    pub items: Vec<ContentId>,
    pub gather_per_second: u64,
    pub unload_per_second: u64,
    pub interaction_range: f32,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedDropOffEffect {
    Store,
    Convert { balance: ContentId, per_item: u64 },
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDropOffRule {
    pub item: ContentId,
    pub effect: ResolvedDropOffEffect,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDropOff {
    pub rules: Vec<ResolvedDropOffRule>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedProducer {
    pub recipes: Vec<ContentId>,
    pub queue_capacity: u16,
    pub spawn_offsets: Vec<WorldPos>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConveyor {
    pub slots_per_piece: u8,
    pub slots_per_second: u64,
    pub accepts: Vec<ContentId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMachine {
    pub recipe: ContentId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedResearchStation {
    pub technologies: Vec<ContentId>,
    pub queue_capacity: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAmmo {
    pub item: ContentId,
    pub per_shot: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWeapon {
    pub range: f32,
    pub cooldown_ms: u64,
    pub damage: u64,
    pub ammo: Option<ResolvedAmmo>,
    pub targets: TargetFilter,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUnit {
    pub id: ContentId,
    pub name: String,
    pub footprint: Footprint,
    pub movement: Option<MovementDef>,
    pub inventory: Option<ResolvedInventory>,
    pub gatherer: Option<ResolvedGatherer>,
    pub drop_off: Option<ResolvedDropOff>,
    pub producer: Option<ResolvedProducer>,
    pub output_inventory: Option<ResolvedInventory>,
    pub extractor: Option<ExtractorDef>,
    pub conveyor: Option<ResolvedConveyor>,
    pub inserter: Option<InserterDef>,
    pub machine: Option<ResolvedMachine>,
    pub research: Option<ResolvedResearchStation>,
    pub health: Option<HealthDef>,
    pub weapon: Option<ResolvedWeapon>,
    pub removal: Option<RemovalDef>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedNode {
    pub id: ContentId,
    pub name: String,
    pub item: ContentId,
    pub radius: f32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCost {
    pub resource: ContentId,
    pub amount: u64,
    pub source: CostSource,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProduction {
    pub id: ContentId,
    pub name: String,
    pub output_unit: ContentId,
    pub duration_ms: u64,
    pub costs: Vec<ResolvedCost>,
    pub payment: PaymentPolicy,
    pub cancel_refunds: CancelRefunds,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedItemStack {
    pub item: ContentId,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRecipe {
    pub id: ContentId,
    pub name: String,
    pub inputs: Vec<ResolvedItemStack>,
    pub outputs: Vec<ResolvedItemStack>,
    pub duration_ms: u64,
    pub cancel_refunds: RunRefunds,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedEffect {
    WeaponModifier {
        unit: ContentId,
        stat: WeaponStat,
        add: u64,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTechnology {
    pub id: ContentId,
    pub name: String,
    pub prerequisites: Vec<ContentId>,
    pub duration_ms: u64,
    pub costs: Vec<ResolvedCost>,
    pub payment: PaymentPolicy,
    pub cancel_refunds: CancelRefunds,
    pub effects: Vec<ResolvedEffect>,
}

/// Fully resolved definitions, keyed by their package-qualified ids.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentCatalog {
    pub resources: BTreeMap<ContentId, ResourceDef>,
    pub units: BTreeMap<ContentId, ResolvedUnit>,
    pub nodes: BTreeMap<ContentId, ResolvedNode>,
    pub production: BTreeMap<ContentId, ResolvedProduction>,
    pub recipes: BTreeMap<ContentId, ResolvedRecipe>,
    pub technologies: BTreeMap<ContentId, ResolvedTechnology>,
    pub source_files: BTreeMap<ContentId, String>,
}
