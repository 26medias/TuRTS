//! Lockfile parse and write.

use turts_mod_api::{Lockfile, ModError};

/// The lockfile as JSON bytes: pretty printed, with a trailing newline.
pub fn write_lockfile(lock: &Lockfile) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(lock).expect("a lockfile serializes to JSON");
    bytes.push(b'\n');
    bytes
}

/// Read lockfile bytes. An error has an empty package and the field path `lockfile`.
pub fn read_lockfile(bytes: &[u8]) -> Result<Lockfile, ModError> {
    serde_json::from_slice(bytes).map_err(|error| ModError::new("", "lockfile", error.to_string()))
}
