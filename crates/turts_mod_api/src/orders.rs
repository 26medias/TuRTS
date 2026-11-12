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
    /// API 0.2.4, row O1: place a site for a building (target `ContentAtPoint`).
    Construct,
    /// API 0.2.4, row O2: cancel an own site.
    CancelConstruction,
    /// API 0.2.4, row O3: set the rally of a producer.
    SetRally,
    /// API 0.2.4, row O4: repair an own unit.
    Repair,
    /// API 0.2.4, row O9: give up the match.
    Surrender,
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
    /// API 0.2.4, row O5: a content id and a point (only with `Construct`).
    ContentAtPoint,
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
    /// API 0.2.4, row O6.
    Constructor,
    /// API 0.2.4, row O6.
    Repairer,
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
    /// API 0.2.4, row O6: an own unit that is damaged (with a `Repair` order).
    OwnDamaged,
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
    /// API 0.2.4, row O7: the follow orders of a rally. `None`: the `smart` list serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rally: Option<Vec<SmartOrder>>,
}
