//! Portable authored sight rules and permitted-knowledge metadata.
//!
//! This module grants no world access. Hosts produce immutable permitted bases.

use serde::{Deserialize, Serialize};

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
