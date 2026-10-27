//! The shared lightyear protocol plugin: one registration order for both ends.
//!
//! lightyear carries the `turts_net_protocol` messages only. This crate does not use
//! replication, prediction, interpolation or deterministic mode.

pub mod channels;
pub mod protocol_plugin;

use core::time::Duration;

pub use channels::{ReliableChannel, StateChannel};
pub use protocol_plugin::ProtocolPlugin;

/// The length of one simulation tick for a tick rate in hertz.
pub fn tick_duration(tick_hz: u16) -> Duration {
    Duration::from_secs_f64(1.0 / tick_hz as f64)
}
