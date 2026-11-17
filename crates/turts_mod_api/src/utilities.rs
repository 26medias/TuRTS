//! API18 (M6) public underground utility authoring and the route order target.
//!
//! A utility type is a public content definition. It names the `Supply` resource with
//! `SupplyScope::Network` that it carries, the surface ports of unit types, the permitted
//! burial depth, the exposure policy and the overlay appearance. Topology is server state;
//! it has no package field. Placed lines keep a fixed elevation (spec 11.2).
use crate::{ContentId, ContentRef, WorldPos};
use serde::{Deserialize, Serialize};

/// Engine limit: junctions of one slot on one map.
pub const MAX_UTILITY_JUNCTIONS_PER_SLOT: usize = 512;
/// Engine limit: segments of one slot on one map.
pub const MAX_UTILITY_SEGMENTS_PER_SLOT: usize = 512;
/// Engine limit: cells that one segment can touch.
pub const MAX_UTILITY_SEGMENT_CELLS: usize = 256;
/// Engine limit: the largest authored `max_segment_length`, in world units.
pub const MAX_UTILITY_SEGMENT_LENGTH: f32 = 64.0;
/// Engine limit: the largest authored depth, in elevation steps.
pub const MAX_UTILITY_DEPTH: u32 = 64;
/// A split point must be within this plan distance of its segment, in world units.
pub const UTILITY_SPLIT_TOLERANCE: f32 = 0.25;
/// A split point and a segment's two endpoints must be farther apart than this.
pub const UTILITY_MIN_SEGMENT_LENGTH: f32 = 0.5;

/// One authored utility type file, listed in `exports.utilities`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityDef {
    pub id: String,
    pub name: String,
    /// A `Supply` resource with `scope: Network`. One utility type for each such resource.
    pub carries: ContentRef,
    /// Surface ports. One port for each unit type in this utility type.
    pub ports: Vec<UtilityPortDef>,
    pub depth: UtilityDepth,
    /// Largest plan distance from an issuing unit to each route endpoint, in world units.
    pub reach: f32,
    /// Largest plan length of one segment, in world units.
    pub max_segment_length: f32,
    /// `Balance` resources charged once for each new segment. Can be empty.
    #[serde(default)]
    pub segment_cost: Vec<UtilityCost>,
    /// What exposure does to a segment. The engine has no default policy.
    pub exposure: UtilityExposure,
    pub appearance: UtilityAppearance,
}

/// A port of one unit type. `offset` is in the unit's plan frame, in world units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityPortDef {
    pub unit: ContentRef,
    pub offset: WorldPos,
}

/// Requested burial depth in terrain elevation steps: `1 <= min <= default <= max <= 64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityDepth {
    pub default: u32,
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityCost {
    pub resource: ContentRef,
    pub amount: u64,
}

/// Mod policy for a segment whose cover at a touched cell is 0 or less (spec 11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum UtilityExposure {
    /// The segment is marked exposed and still conducts.
    Operate,
    /// The segment is marked exposed and does not conduct until its cover returns.
    Disconnect,
    /// The segment is deleted when it is exposed.
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityAppearance {
    /// Route colour in the engine's authored colour syntax.
    pub colour: String,
    pub exposed_colour: String,
    pub junction: UtilitySymbol,
    pub labels: UtilityLabels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum UtilitySymbol {
    Circle,
    Square,
    Diamond,
}

/// Inspection labels for the four public states of a port or segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityLabels {
    pub powered: String,
    pub short: String,
    pub unconnected: String,
    pub exposed: String,
}

/// A validated utility type with package-qualified references.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUtility {
    pub id: ContentId,
    pub name: String,
    pub carries: ContentId,
    /// Keyed by unit type; one port per unit type.
    pub ports: std::collections::BTreeMap<ContentId, WorldPos>,
    pub depth: UtilityDepth,
    pub reach: f32,
    pub max_segment_length: f32,
    pub segment_cost: Vec<(ContentId, u64)>,
    pub exposure: UtilityExposure,
    pub appearance: UtilityAppearance,
}

/// One endpoint of a route command. Ids are the issuing slot's own ids.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum UtilityEndpoint {
    /// The port of an own completed unit (wire unit id).
    Port(u32),
    /// An own junction.
    Node(u32),
    /// A new junction on an own segment. The segment splits at that point.
    Split { segment: u32, point: WorldPos },
    /// A new junction at a free point.
    Free(WorldPos),
}

/// The target of a `UtilityRoute` order: one new segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityRouteTarget {
    pub utility: ContentRef,
    /// Requested burial depth in elevation steps.
    pub depth: u32,
    pub from: UtilityEndpoint,
    pub to: UtilityEndpoint,
}
