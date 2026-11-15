//! Authoritative terrain. Selected geology is private; callers must request permitted facts.
mod state;
pub(crate) mod step;
pub(crate) use state::{PreparedCellEdit, TerrainState, prepare};
pub use state::{PrivateTerrainSeed, TerrainError};
