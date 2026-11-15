//! API17 public terrain authoring and ordinary work designations.
//!
//! This module contains generation rules and initial surface geometry only.
//! A private match seed and realized geological columns are server state and
//! have no serializable field in this public contract. Root wiring is separate.
use crate::{ContentRef, Ratio};
use serde::{Deserialize, Serialize};

/// Map-local integer cell coordinate. The grid bounds determine valid values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainCell {
    pub x: u32,
    pub y: u32,
}

/// Half-open cell rectangle: min is included and max is excluded on both axes.
/// Validation rejects empty, reversed, or out-of-grid rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainRect {
    pub min: TerrainCell,
    pub max: TerrainCell,
}

/// Public initial-surface patch in integer elevation steps above the fixed datum.
/// Patches must be in bounds and non-overlapping; they override the base height.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainHeightRect {
    pub cells: TerrainRect,
    pub elevation: i32,
}

/// Optional map terrain. Lengths are positive integer quanta, not actor pose units.
/// Actor positions and radii keep their continuous world-coordinate meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDef {
    /// Number of length quanta in one world unit.
    pub length_scale: u32,
    /// Horizontal side length of one square cell, in length quanta.
    pub cell_width: u32,
    /// Vertical length of one elevation step, in length quanta.
    pub elevation_step: u32,
    pub columns: u32,
    pub rows: u32,
    /// Side length of a chunk, in cells; final chunks may be partial.
    pub chunk_width: u32,
    /// Base initial height of cells without a patch, in integer elevation steps.
    pub initial_elevation: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_heights: Vec<TerrainHeightRect>,
    /// Non-extractable bedrock level, in the same fixed elevation datum.
    pub floor: i32,
    /// Maximum permitted surface level, including placed material.
    pub ceiling: i32,
    pub generation: TerrainGenerationDef,
}

/// Public, versioned geological generation rules. The server supplies the seed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TerrainGenerationDef {
    /// Ordered contiguous datum bands covering floor through the highest initial
    /// surface. Each generated column is clipped to its own initial elevation.
    Version1 { bands: Vec<TerrainDatumBand> },
}

/// A bottom-inclusive, top-exclusive interval in integer elevation steps.
/// Every choice has a positive weight and names a public material definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDatumBand {
    pub bottom: i32,
    pub top: i32,
    pub choices: Vec<TerrainWeightedMaterial>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainWeightedMaterial {
    pub material: ContentRef,
    pub weight: u64,
}

/// An exact resource coefficient per cubic world unit. Runtime validation checks
/// reference kinds, nonzero denominator, and exact bounded quantities per prism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainResourceRate {
    pub resource: ContentRef,
    pub per_world_cube: Ratio,
}

/// Public material behavior. This does not declare where the material occurs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainMaterialDef {
    pub id: String,
    pub name: String,
    /// Public surface colour in the engine's authored colour syntax.
    pub surface_color: String,
    pub hardness: u64,
    /// Required integer work per cubic world unit, independent of tool speed.
    pub work_per_world_cube: Ratio,
    pub natural_outputs: Vec<TerrainResourceRate>,
    pub fill_inputs: Vec<TerrainResourceRate>,
    /// Required explicit provenance policy. No placed prism yields natural ore.
    pub placed_removal: TerrainPlacedRemoval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TerrainPlacedRemoval {
    NoYield,
}

/// Public traversal limits. Both the step limit and exact rise/run grade apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainMovementClassDef {
    pub id: String,
    pub name: String,
    /// Maximum covered elevation spread and cardinal edge rise, in elevation steps.
    pub max_step: u32,
    /// Absolute rise in world units divided by one cell's horizontal world width.
    pub max_grade: Ratio,
}

/// Supported static foundation policy. It never silently removes a structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TerrainFoundation {
    ForbidEdit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TerrainOperation {
    Excavate,
    Fill { material: ContentRef },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TerrainOutputDestination {
    Inventory,
    SlotBalances,
}

/// Public work capability referenced by an eligible unit. Names grant no behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainToolDef {
    pub id: String,
    pub name: String,
    pub operation: TerrainOperation,
    pub work_per_second: u64,
    /// Finite, nonnegative world distance. No integer-grid snapping is implied.
    pub reach: f32,
    /// Materials currently exposed at the work cell, never buried query results.
    pub allowed_materials: Vec<ContentRef>,
    pub max_hardness: u64,
    pub output_destination: TerrainOutputDestination,
}

/// Ordinary typed work target. Validation bounds the rectangle to at most 256
/// cells and checks its tool and target elevation against the installed map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDesignation {
    pub tool: ContentRef,
    pub cells: TerrainRect,
    pub target_elevation: i32,
}

/// Resolved public records. These contain no selected underground material or seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerrainResourceRate {
    pub resource: crate::ContentId,
    pub per_world_cube: Ratio,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerrainMaterial {
    pub id: crate::ContentId,
    pub name: String,
    pub surface_color: String,
    pub hardness: u64,
    pub work_per_world_cube: Ratio,
    pub natural_outputs: Vec<ResolvedTerrainResourceRate>,
    pub fill_inputs: Vec<ResolvedTerrainResourceRate>,
    pub placed_removal: TerrainPlacedRemoval,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerrainMovementClass {
    pub id: crate::ContentId,
    pub name: String,
    pub max_step: u32,
    pub max_grade: Ratio,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTerrainOperation {
    Excavate,
    Fill { material: crate::ContentId },
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTerrainTool {
    pub id: crate::ContentId,
    pub name: String,
    pub operation: ResolvedTerrainOperation,
    pub work_per_second: u64,
    pub reach: f32,
    pub allowed_materials: Vec<crate::ContentId>,
    pub max_hardness: u64,
    pub output_destination: TerrainOutputDestination,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerrainWeightedMaterial {
    pub material: crate::ContentId,
    pub weight: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerrainDatumBand {
    pub bottom: i32,
    pub top: i32,
    pub choices: Vec<ResolvedTerrainWeightedMaterial>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerrainDef {
    pub length_scale: u32,
    pub cell_width: u32,
    pub elevation_step: u32,
    pub columns: u32,
    pub rows: u32,
    pub chunk_width: u32,
    pub initial_elevation: i32,
    pub initial_heights: Vec<TerrainHeightRect>,
    pub floor: i32,
    pub ceiling: i32,
    pub generation: Vec<ResolvedTerrainDatumBand>,
}

/// Explicit timed unloading for physical terrain cargo; no gatherer capability is implied.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainCargoDef {
    pub unload_per_second: u64,
    pub interaction_range: f32,
}
