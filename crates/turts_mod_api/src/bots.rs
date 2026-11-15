//! Portable bot authoring data. Controller execution belongs to the host framework.

use crate::{Capability, ContentRef, UniqueMap};
use serde::{Deserialize, Serialize};

/// One explicitly selected opponent profile. Importing it does not claim a slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotDef {
    pub id: String,
    pub name: String,
    pub decision_ms: u32,
    pub start_when: BotStartWhen,
    pub knowledge: BotKnowledge,
    pub controllers: Vec<BotControllerDef>,
}

/// A per-tick guard. Pausing retains pending work and controller state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotStartWhen {
    MatchRunning,
    HumanPresent,
}

/// An authored instance key determines its independent seed and quota.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotControllerDef {
    pub key: String,
    pub factory: ContentRef,
    #[serde(default)]
    pub config: BotValue,
    pub quota: BotQuota,
}

/// Each controller owns these limits. Registration order does not distribute them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotQuota {
    pub queries: u32,
    pub candidates: u32,
    pub rows: u32,
    pub proposals: u32,
}

/// Configuration data, never executable package code. Maps reject duplicate keys.
/// The loader also limits encoded bytes, nesting, and aggregate item count.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotValue {
    #[default]
    Null,
    Bool(bool),
    Integer(i64),
    String(String),
    List(Vec<BotValue>),
    Map(UniqueMap<String, BotValue>),
}

/// Explicit game knowledge shared by composed controllers.
/// Keys identify roles and bindings; list order ranks authored goals only.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotKnowledge {
    #[serde(default)]
    pub roles: Vec<BotRole>,
    #[serde(default)]
    pub orders: Vec<BotOrderBinding>,
    #[serde(default)]
    pub recipes: Vec<BotRecipeBinding>,
    #[serde(default)]
    pub opening: Vec<BotGoal>,
    #[serde(default)]
    pub recovery: Vec<BotGoal>,
    #[serde(default)]
    pub production: Vec<BotProductionTarget>,
    #[serde(default)]
    pub squads: Vec<BotSquadRule>,
}

/// A role explicitly lists content. It does not infer a faction's game rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotRole {
    pub key: String,
    pub units: Vec<ContentRef>,
    #[serde(default)]
    pub requires: Vec<Capability>,
}

/// A named ordinary order and the role that can issue it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotOrderBinding {
    pub key: String,
    pub order: ContentRef,
    pub actor_role: String,
}

/// Construction and unit production remain distinct authored actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotRecipeKind {
    Produce,
    Construct,
}

/// A recipe binding names its source role and ordinary order binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotRecipeBinding {
    pub key: String,
    pub recipe: ContentRef,
    pub actor_role: String,
    pub order: String,
    pub kind: BotRecipeKind,
}

/// Stable keys identify goals across pending results and profile list order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotGoal {
    pub key: String,
    pub priority: u32,
    pub action: BotGoalAction,
}

/// Reusable economic goals. Every string binding resolves within this profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotGoalAction {
    EnsureUnits {
        role: String,
        count: u32,
        recipe: String,
    },
    Gather {
        role: String,
        resource: ContentRef,
        order: String,
    },
    EnsureSupply {
        resource: ContentRef,
        amount: u64,
        provider_role: String,
        recipe: String,
    },
}

/// Desired completed and pending production, rather than repeated queue requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotProductionTarget {
    pub key: String,
    pub role: String,
    pub count: u32,
    pub recipe: String,
    pub priority: u32,
}

/// Simple assembly, defense, and attack policy supplied by a game package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotSquadRule {
    pub key: String,
    pub role: String,
    pub min_units: u32,
    pub anchor_role: String,
    pub attack_order: String,
    pub move_order: String,
    pub defend_radius: f32,
    pub assemble_radius: f32,
    /// Empty accepts any permitted hostile type. Otherwise use these role sets.
    #[serde(default)]
    pub target_roles: Vec<String>,
    pub priority: u32,
}
