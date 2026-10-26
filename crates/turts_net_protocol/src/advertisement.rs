//! Server advertisement.
//!
//! The server sends this value as JSON over HTTP (section 5 of the design).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Advertisement {
    pub mod_id: String,
    pub mod_version: String,
    pub engine_version: String,
    pub protocol_version: u32,
    pub lockfile_url: String,
    pub lockfile_hash: String,
}
