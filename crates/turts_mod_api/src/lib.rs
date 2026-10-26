//! Public mod API: content ids, package schema types, the UnitRegistry trait, ModError.

pub mod content;
pub mod error;
pub mod ids;
pub mod lock;
pub mod registry;
pub mod schema;
pub mod schema_v02;

pub use error::ModError;
pub use ids::{ContentId, PackageId, UnitTypeId};
pub use lock::{LockEntry, Lockfile};
pub use registry::UnitRegistry;
pub use schema::{
    ExecutionContext, Exports, Footprint, Manifest, MapDef, MapSize, PackageKind, Placement, Shape,
    UnitDef, VisualDef, WorldPos,
};

/// Version of the engine API that this crate defines.
pub const ENGINE_API_VERSION: &str = "0.1.0";
