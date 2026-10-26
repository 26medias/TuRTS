//! Wire messages.
//!
//! The server and the client exchange these types as postcard bytes. No type here holds a
//! Bevy type or a Bevy entity id.

use serde::{Deserialize, Serialize};

/// The identifier of a unit on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitId(pub u32);

/// The identifier of a player slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotId(pub u8);

/// A position on the flat map.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

/// Client to server, reliable. First message of a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub player_id: u64,
    pub protocol_version: u32,
    pub lockfile_hash: String,
}

/// Server to client. The server accepted the `Hello` and assigned a slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Welcome {
    pub slot_id: SlotId,
    pub tick: u32,
    pub tick_hz: u16,
}

/// Server to client. The server refused the `Hello`. A disconnect follows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinRejected {
    pub reason: String,
}

/// The full state of one unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitSnapshot {
    pub unit_id: UnitId,
    pub content_id: String,
    pub owner_slot: SlotId,
    pub pos: Pos,
}

/// Server to client, reliable. The full state at `tick`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    pub units: Vec<UnitSnapshot>,
}

/// The new position of one unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitDelta {
    pub unit_id: UnitId,
    pub pos: Pos,
}

/// Server to client, unordered unreliable. The units that changed at `tick`.
/// `base_tick` is the previous tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitDeltas {
    pub tick: u32,
    pub base_tick: u32,
    pub changes: Vec<UnitDelta>,
}

/// Client to server, reliable. Order the listed units to move to `target`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveTo {
    pub seq: u32,
    pub unit_ids: Vec<UnitId>,
    pub target: Pos,
}

/// Server to client. The server received command `seq`. This does not mean it accepted it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandAck {
    pub seq: u32,
    pub tick: u32,
}

/// Server to client. Validation of command `seq` failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRejected {
    pub seq: u32,
    pub reason: String,
}

/// Client to server. The server answers with a `Snapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResyncRequest;
