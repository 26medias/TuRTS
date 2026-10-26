//! Archive hash.

use std::fmt::Write;

use sha2::{Digest, Sha256};
use turts_mod_api::ModError;

/// Prefix of a hash string.
// M1 placeholder; format open (spec 15)
const HASH_PREFIX: &str = "sha256:";

/// The hash string of exact bytes: `sha256:` plus 64 lowercase hex characters.
// M1 placeholder; format open (spec 15)
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(HASH_PREFIX.len() + 2 * digest.len());
    out.push_str(HASH_PREFIX);
    for byte in digest.iter() {
        write!(out, "{byte:02x}").expect("writing to a String does not fail");
    }
    out
}

/// Check that `bytes` hash to `expected`. The error names both hashes.
pub fn verify_hash(bytes: &[u8], expected: &str) -> Result<(), ModError> {
    let actual = sha256_hex(bytes);
    if actual == expected {
        Ok(())
    } else {
        Err(ModError::new(
            "",
            "hash",
            format!("package hash {actual} does not match the expected hash {expected}"),
        ))
    }
}
