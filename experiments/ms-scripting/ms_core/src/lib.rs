//! MS scripting selection spike: the shared core.
//!
//! The native driver (`ms_native`) and the wasm32 driver (`ms_web`) call the same functions in
//! the same order. Nothing here knows Bevy, lightyear, or the engine crates. The world is a mock
//! (spec section 6 of `docs/superpowers/specs/2026-11-16-ms-scripting-selection.md`).

pub mod alloc;
pub mod bench;
pub mod cand;
#[cfg(target_arch = "wasm32")]
pub mod entropy;
pub mod host;
pub mod probes;
pub mod rng;
pub mod rules;
pub mod world;

pub use bench::{Bench, Workload};
pub use cand::{Kind, Limits, Runtime, ScriptError, ScriptErrorKind};

/// The candidate that this build holds, or `"baseline"` when it holds none.
pub fn compiled_candidates() -> Vec<&'static str> {
    let mut v = Vec::new();
    if cfg!(feature = "rhai") {
        v.push("rhai");
    }
    if cfg!(feature = "rune") {
        v.push("rune");
    }
    if cfg!(feature = "lua") {
        v.push("lua");
    }
    v
}

/// Minimal JSON string escape, so that no serde enters the size comparison.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
