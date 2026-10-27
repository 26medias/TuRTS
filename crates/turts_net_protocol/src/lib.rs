//! Wire messages and NET_PROTOCOL_VERSION. Transport-free.
//!
//! This crate holds only serde types. It has no `bevy*` dependency and no Bevy entity id.

pub mod advertisement;
pub mod messages;
pub mod staged_v2;

pub use advertisement::Advertisement;
pub use messages::*;

/// The version of the wire protocol. The client sends it in `Hello` and the server checks it.
pub const NET_PROTOCOL_VERSION: u32 = 1;
