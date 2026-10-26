//! Lockfile entry types. The lockfile is a JSON document (spec 5.5).

use serde::{Deserialize, Serialize};

use crate::schema::ExecutionContext;

/// The exact package set a server uses. Clients download this set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lockfile {
    pub packages: Vec<LockEntry>,
}

/// One locked package: the seven fields of spec 5.5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockEntry {
    pub id: String,
    pub version: String,
    /// `sha256:` plus 64 lowercase hex characters, over the exact archive bytes.
    // M1 placeholder; format open (spec 15)
    pub hash: String,
    pub size: u64,
    pub urls: Vec<String>,
    pub context: ExecutionContext,
    pub engine_api: String,
}
