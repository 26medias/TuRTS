//! Portable authored sight rules and permitted-knowledge metadata.
//!
//! This module grants no world access. Hosts produce immutable permitted bases.

use serde::{Deserialize, Serialize};

/// Maximum UTF-8 bytes in an opt-in fog map's qualified identity.
/// Transaction chunks repeat this identity within their 1024-byte frame limit.
pub const MAX_VISIBILITY_MAP_ID_BYTES: usize = 512;

/// Maximum number of square coverage cells in one authored map.
pub const MAX_VISIBILITY_CELLS: u64 = 1_048_576;
/// Maximum storage for retained slot coverage masks, excluding entity memories.
pub const MAX_VISIBILITY_COVERAGE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum and default number of remembered stale subjects per slot.
pub const MAX_REMEMBERED_SUBJECTS: u32 = 65_536;

/// Which installed slots contribute shared public sight and explored cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum VisionSharing {
    OwnSlot,
    Team,
}

/// API16: optional map rule. Absence of this rule selects AllVisible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityDef {
    pub cell_size: f32,
    pub sharing: VisionSharing,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<u32>,
}
impl VisibilityDef {
    pub fn effective_memory_limit(&self) -> u32 {
        self.memory_limit.unwrap_or(MAX_REMEMBERED_SUBJECTS)
    }
}

/// API16: an optional completed unit's sight circle, in world units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisionDef {
    pub radius: f32,
}

/// Validated map-local grid. The final row/column clips to `size`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityGrid {
    pub size: crate::WorldPos,
    pub cell_size: f32,
    pub columns: u32,
    pub rows: u32,
}

/// Explicit permission phase. Public bot decisions use `Completed` bases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservationPhase {
    Start,
    Tactical,
    Completed,
}

/// Identity of an immutable permitted-knowledge basis, independent of transport.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeBasis {
    pub match_id: u64,
    pub map: crate::ContentId,
    pub tick: u32,
    pub phase: ObservationPhase,
}

/// Public facts retained from one permitted unit observation. Never live health.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RememberedUnit {
    pub unit: u32,
    pub content: crate::ContentId,
    pub owner: u8,
    pub position: crate::WorldPos,
    pub facing: Option<crate::Facing>,
    pub site: bool,
    pub last_seen_tick: u32,
    /// Last permitted physical ground contact; it grants no neighboring terrain facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground_elevation: Option<i32>,
}

/// Public surface-node facts. Remaining resources may be stale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RememberedNode {
    pub node: u32,
    pub content: crate::ContentId,
    pub position: crate::WorldPos,
    pub remaining: u64,
    pub last_seen_tick: u32,
}

/// Complete slot knowledge. Each mask has ceil(columns*rows/8) bytes.
/// Cells are row-major; the least-significant bit is the earliest cell.
/// Unused trailing bits are zero. This contains no hidden entity state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeSnapshot {
    pub basis: KnowledgeBasis,
    pub viewer: u8,
    pub grid: VisibilityGrid,
    pub observed: Vec<u8>,
    pub explored: Vec<u8>,
    pub remembered_units: Vec<RememberedUnit>,
    pub remembered_nodes: Vec<RememberedNode>,
    /// Sticky until map reset. It does not assert any subject's death.
    pub retention_truncated: bool,
}

/// A contiguous row-major coverage replacement. Explored cells cannot be cleared
/// by a delta within the same installed map. Runs must be disjoint and in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageRun {
    pub first_cell: u32,
    pub cell_count: u32,
    pub observed: bool,
    pub explored: bool,
}

/// Changed knowledge for one atomic completed-state patch. Hiding an ID is not
/// removal/death. Revealed IDs name complete public rows in that same patch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeDelta {
    pub basis: KnowledgeBasis,
    pub viewer: u8,
    pub coverage: Vec<CoverageRun>,
    pub hidden_units: Vec<u32>,
    pub hidden_nodes: Vec<u32>,
    pub revealed_units: Vec<u32>,
    pub revealed_nodes: Vec<u32>,
    pub remembered_units: Vec<RememberedUnit>,
    pub remembered_nodes: Vec<RememberedNode>,
    /// Forgetting disproves an old location or drops retained knowledge. It is
    /// deliberately separate from an authorized entity removal event.
    pub forgotten_units: Vec<u32>,
    pub forgotten_nodes: Vec<u32>,
    pub retention_truncated: bool,
}
