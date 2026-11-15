//! Portable bot controllers, bounded observations, and coordinated ordinary orders.
//!
//! The host owns authority and supplies permitted immutable facts. This crate never receives
//! a mutable world or an unrestricted host callback.
pub mod seed;

pub use seed::derive_seed;
