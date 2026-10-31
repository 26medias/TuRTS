use crate::ContentRef;
use serde::{Deserialize, Serialize};

pub const DEFAULT_QUEUE_DEPTH: u8 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderDef {
    pub id: String,
    pub name: String,
    pub behaviour: Behaviour,
    pub target: TargetKind,
    pub fire: Option<FirePolicy>,
    pub engage: Option<EngagePolicy>,
    pub icon: String,
    pub hotkey: String,
    pub cursor: Option<String>,
    pub card: Option<CardCell>,
    pub units: Vec<ContentRef>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardCell {
    pub row: u8,
    pub column: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Behaviour {
    Move,
    Stop,
    Hold,
    Attack,
    Gather,
    Produce,
    CancelProduction,
    Remove,
    SetEnabled(bool),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TargetKind {
    None,
    Point,
    Unit,
    PointOrUnit,
    Node,
    Job,
    Content,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FirePolicy {
    Never,
    InRange,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum EngagePolicy {
    None,
    StopAndFire,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Capability {
    Movement,
    Weapon,
    Gatherer,
    Producer,
    Research,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SmartTarget {
    Ground,
    OwnUnit,
    ForeignUnit,
    ForeignUnitWeaponTarget,
    Node,
    ResourceNodeAccepted,
    Content,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmartOrder {
    pub target: SmartTarget,
    pub needs: Capability,
    pub order: ContentRef,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderRulesDef {
    pub queue_depth: u8,
    pub smart: Vec<SmartOrder>,
}
