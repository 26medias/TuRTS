//! Channel definitions.
//!
//! These types are marker types. `ProtocolPlugin` registers them.

/// Ordered and reliable, in both directions. Carries `Hello`, `Welcome`, `JoinRejected`,
/// `Snapshot`, `MoveTo`, `CommandAck`, `CommandRejected` and `ResyncRequest`.
pub struct ReliableChannel;

/// Sequenced and unreliable, server to client. Carries `UnitDeltas`.
pub struct StateChannel;
