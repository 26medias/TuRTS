//! Package schema types: Manifest, UnitDef, MapDef, VisualDef.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `manifest.ron`: the nine fields of spec 5.5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    // M1 placeholder; format open (spec 15)
    pub version: String,
    pub kind: PackageKind,
    pub engine_api: String,
    pub dependencies: BTreeMap<String, String>,
    pub exports: Exports,
    pub context: ExecutionContext,
    // M1 placeholder; format open (spec 15)
    pub capabilities: Vec<String>,
    pub license: String,
}

/// What a package is for.
// M1 placeholder; format open (spec 15)
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

/// Archive paths of the files a package exports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exports {
    pub units: Vec<String>,
    pub maps: Vec<String>,
    pub visuals: Vec<String>,
}

/// `units/<name>.ron`: a gameplay definition. `id` is local (no `:`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitDef {
    pub id: String,
    pub name: String,
    /// World units per second.
    pub speed: f32,
    pub footprint: Footprint,
}

/// Unit footprint. M1 has one variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Footprint {
    Circle { radius: f32 },
}

/// `maps/<name>.ron`: size and unit placements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapDef {
    pub id: String,
    pub size: MapSize,
    pub placements: Vec<Placement>,
}

/// Map extent on the ground plane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSize {
    pub x: f32,
    pub y: f32,
}

/// One unit placed at match start. `unit` is a full content id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub unit: String,
    pub owner_slot: u8,
    pub pos: WorldPos,
}

/// Position on the ground plane: origin at one map corner, `x` right, `y` forward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPos {
    pub x: f32,
    pub y: f32,
}

/// `visuals/<name>.ron`: presentation only. The server never reads it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualDef {
    /// Full content id of the unit.
    pub unit: String,
    pub shape: Shape,
}

/// Visual shape. Named fields only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Shape {
    Capsule { radius: f32, height: f32 },
    Cuboid { x: f32, y: f32, z: f32 },
}
