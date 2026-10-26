//! Channel definitions.
//!
//! These types are marker types. `ProtocolPlugin` registers them.

/// Ordered and reliable, in both directions. Carries `Hello`, `Welcome`, `JoinRejected`,
/// `Snapshot`, `MoveTo`, `CommandAck`, `CommandRejected` and `ResyncRequest`.
pub struct ReliableChannel;

/// Unordered and unreliable, server to client. Carries `UnitDeltas`. Every delta that arrives
/// reaches the application; the client detects a gap with the `base_tick` chain.
pub struct StateChannel;
