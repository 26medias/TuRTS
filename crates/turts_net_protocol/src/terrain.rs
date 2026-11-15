//! Protocol10 permitted terrain facts. Private strata and seeds have no wire type.
use serde::{Deserialize, Serialize};
use turts_mod_api::{TerrainCell, TerrainDesignation};

use crate::{ContentRef, UnitId};

/// Index into the snapshot's public, canonical material dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainMaterialId(pub u32);

/// A current or remembered observation. Unobserved cells have no record.
/// Their public initial geometry remains available from the verified package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainSurfaceRecord {
    pub cell: TerrainCell,
    pub elevation: i32,
    /// None denotes observed bedrock, not a buried material query.
    pub material: Option<TerrainMaterialId>,
    /// Origin of this exposed surface only. Bedrock has no material/provenance.
    #[serde(default)]
    pub provenance: Option<turts_mod_api::TerrainProvenance>,
    pub observed_tick: u32,
    pub current: bool,
}

/// Physical contact authorized for an owned body, or a permitted foreign sighting.
/// It grants no neighboring cell, buried material or private surface revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainGroundAnchor {
    pub unit: UnitId,
    pub elevation: i32,
    pub observed_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainWorkWait {
    Approaching,
    Unknown,
    Contested,
    Occupied,
    UnsupportedMaterial,
    TooHard,
    Bedrock,
    Ceiling,
    FullCargo,
    InsufficientInput,
    Blocked,
    Paused,
    Unloading,
}

/// Own-worker status. Unknown cells cannot publish material-dependent progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainWorkRecord {
    pub unit: UnitId,
    pub designation: TerrainDesignation,
    pub current_cell: Option<TerrainCell>,
    pub completed_work: u64,
    pub required_work: Option<u64>,
    pub wait: Option<TerrainWorkWait>,
}

/// Installed atomically with the corresponding unit, knowledge and account basis.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainSnapshot {
    /// All public materials in canonical catalog order. It contains no occurrence data.
    pub materials: Vec<ContentRef>,
    pub surfaces: Vec<TerrainSurfaceRecord>,
    pub anchors: Vec<TerrainGroundAnchor>,
    pub work: Vec<TerrainWorkRecord>,
}

/// Cleared records retire only prior permitted facts. Private edit counters never
/// cross this boundary; clients derive chunk revisions from activated changes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDelta {
    pub surfaces: Vec<TerrainSurfaceRecord>,
    pub surfaces_cleared: Vec<TerrainCell>,
    pub anchors: Vec<TerrainGroundAnchor>,
    pub anchors_cleared: Vec<UnitId>,
    pub work: Vec<TerrainWorkRecord>,
    pub work_cleared: Vec<UnitId>,
}
