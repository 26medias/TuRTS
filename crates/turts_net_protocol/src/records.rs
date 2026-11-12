//! The records of protocol 5 (design 6.7.13 and 6.8): lobby, match start, match state, match
//! result, rate records, supply, held construction orders and effects.
//!
//! Lane 0 freezes these shapes. Each feature lane builds its own writer. This module has no
//! `bevy*` type and no engine-internal id. Every unit is a `UnitId`; a content name is a
//! `ContentRef` (a string); a client never needs a recompile for a new mod unit.

use crate::economy::{ContentRef, NodeId};
use crate::messages::{Pos, SlotId, UnitId};
use serde::{Deserialize, Serialize};
use turts_mod_api::Cost;

/// The job state of one site or one production job (design 1.1, "Job state"). The words on the
/// screen are the `states` texts of the mod (6.10 row H3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordState {
    Running,
    Slowed,
    WaitingForFunds(ContentRef),
    OutputBlocked,
    Held,
}

/// A value at a tick and a rate (design 6.8, "Records with a start tick and a rate"):
/// `done` of `total` at `at_tick`, and `per_tick` more in each following tick.
///
/// Units (ruling of the L0-WIRE follow-up): `done`, `total` and `per_tick` count RAW UNITS of 1/Q
/// tick, where Q is the least common multiple of every `Rate` denominator of the catalog (design
/// 6.4, "Progress in integers"). Today Q is 1, so the units are ticks. The client derives the ticks
/// with `turts_rules::progress::base_ticks_done(units, duration_ticks, total)`, where
/// `duration_ticks` comes from the `JobState` (`Replica::job_elapsed_ticks`). Example: Q = 2, a job of
/// 100 ticks at rate 1/2: `total` 200, `per_tick` 1 (a tick adds `rate_step(Q, rate)` units), and 50
/// ticks later `done` is 50 units, which is 25 ticks of base time. The record is
/// sent only when the rate, the job state or `held` changes, never each tick. The client
/// evaluates it at the tick of its last applied delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateRecord {
    pub done: u64,
    pub total: u64,
    pub per_tick: u64,
    pub at_tick: u32,
    pub state: RecordState,
    pub charged: Vec<Cost>,
    /// The record is held (design 6.4, erratum E21): its value does not advance.
    pub held: bool,
    /// The tick at which `held` became true. 0 when `held` is false.
    pub held_since_tick: u32,
}

/// The progress of one site (a building under construction). Owner only.
///
/// Store rule (ruling for `L0-STORE`, MP1 review item 6): the store drops a `SiteProgress` when the
/// unit is in `DeltaRecords::sites_completed` or in `WorldDelta::removed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteProgress {
    /// The site unit. A site that leaves the world in the message that carries its record has no
    /// record in that message (`removed` is the only mention).
    pub unit: UnitId,
    pub progress: RateRecord,
}

/// The progress of one production job. Owner only. The same form as `SiteProgress`.
///
/// Store rule (ruling for `L0-STORE`): the store drops a `JobProgress` whose `(producer, job)` is no
/// longer in the `QueueState` of the producer (the job completed or was cancelled), and one whose
/// producer is removed. The replica does this today in `merge_job_progress`
/// (`crates/turts_client/src/replica.rs`). The elapsed ticks of a job are
/// `turts_rules::progress::value_at` of this record at the last applied tick: `JobState` has no
/// `elapsed_ticks` since protocol 5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobProgress {
    /// The producer unit.
    pub producer: UnitId,
    pub job: crate::economy::JobId,
    pub progress: RateRecord,
}

/// One held `Construct` order of a constructor: a content and a point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeldOrder {
    pub content: ContentRef,
    pub point: Pos,
}

/// The held orders of one constructor (design 6.1). Owner only. The record is complete: an
/// empty `orders` list replaces the held orders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructionQueue {
    pub constructor: UnitId,
    pub orders: Vec<HeldOrder>,
}

/// The supply of one slot and one resource (design 6.2). Owner only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyRecord {
    pub slot: SlotId,
    pub resource: ContentRef,
    pub provided: u64,
    pub consumed: u64,
}

/// The rally of a producer (design 6.5). It is a field of `UnitOrderRecord`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RallyTarget {
    Point(Pos),
    Unit(UnitId),
    Node(NodeId),
}

/// An effect of one tick (design 6.8, "Effects are never replayed"). Effects ride the
/// unreliable state channel. An effect is an event, not state: the client does not check it
/// against its unit table, because `Fired` and `Destroyed` name a unit that the same delta
/// removes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// `weapon` is the index of the weapon on the unit, not a content id (6 B, not 20 B).
    Fired {
        unit: UnitId,
        target: UnitId,
        weapon: u8,
    },
    Destroyed {
        unit: UnitId,
    },
    /// A delivery or a refund at a unit, for the "+N" text. Owner only.
    Credited {
        unit: UnitId,
        resource: ContentRef,
        amount: u64,
    },
}

/// The seat of a lobby (design 6.7.13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seat {
    pub index: u8,
    pub occupant: SeatOccupant,
    pub ready: bool,
    pub colour: u8,
}

/// A seat never carries a `player_id` (design 6.8): the id is a bearer value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeatOccupant {
    Open,
    Human { label: String },
    Bot { content: ContentRef },
}

/// One match setting. The list is empty in MP1 (the mod settings schema is M9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LobbyStart {
    Waiting,
    Countdown { remaining_ms: u32 },
}

/// The lobby of a room. Server to client, reliable, to the room members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyState {
    pub revision: u32,
    pub you: Option<u8>,
    pub seats: Vec<Seat>,
    pub settings: Vec<SettingEntry>,
    pub start: LobbyStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LobbyActionKind {
    SetReady(bool),
    SetColour(u8),
}

/// A lobby action. Client to server, reliable.
///
/// The receiver takes the player from the connection that sent the message, never from the
/// message: it has no slot and no player id. It has no receiver in lane 0 and costs no bucket
/// token; owner `L-MATCH-SRV1` (limit the message there).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyAction {
    pub revision: u32,
    pub action: LobbyActionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LobbyRefusal {
    ColourTaken,
    ColourUnknown,
    LobbyChanged,
    NotInLobby,
}

impl LobbyRefusal {
    pub const ALL: &'static [Self] = &[
        Self::ColourTaken,
        Self::ColourUnknown,
        Self::LobbyChanged,
        Self::NotInLobby,
    ];
    /// The shell text key. These are not mod `reasons` keys.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ColourTaken => "colour_taken",
            Self::ColourUnknown => "colour_unknown",
            Self::LobbyChanged => "lobby_changed",
            Self::NotInLobby => "not_in_lobby",
        }
    }
}

/// The refusal of a lobby action. Server to client, reliable, to the sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyRefused {
    pub revision: u32,
    pub reason: LobbyRefusal,
}

/// The start of a match: a part of `Welcome` (design 6.7.13). `teams` and `colours` are
/// indexed by slot.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MatchStart {
    pub start_tick: u32,
    pub teams: Vec<u8>,
    pub colours: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotConnection {
    pub slot: SlotId,
    pub connected: bool,
    pub since_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Victory,
    Defeat,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeReason {
    KeyUnitsLost,
    Surrendered,
    Disconnected,
    OpponentsDefeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotOutcome {
    pub outcome: Outcome,
    pub reason: OutcomeReason,
    pub tick: u32,
}

/// The public match state: the connection of each slot and the outcome of each slot. It rides
/// the delta on change, and the `Snapshot` holds it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MatchState {
    pub connections: Vec<SlotConnection>,
    pub outcomes: Vec<Option<SlotOutcome>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultSlot {
    pub slot: SlotId,
    pub label: String,
    pub outcome: Outcome,
    pub reason: OutcomeReason,
    pub tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatheredStat {
    pub resource: ContentRef,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotStats {
    pub slot: SlotId,
    pub units_trained: u32,
    pub units_lost: u32,
    pub units_destroyed: u32,
    pub structures_completed: u32,
    pub structures_lost: u32,
    pub gathered: Vec<GatheredStat>,
}

/// The result of the match: reliable, once, public, and only after the end tick. The
/// statistics are in no other message (row RES1). `stats` is empty in stage 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchResult {
    pub end_tick: u32,
    pub slots: Vec<ResultSlot>,
    pub stats: Vec<SlotStats>,
    pub duration_ticks: u32,
}

/// A client leaves the match. Client to server, reliable.
///
/// The receiver takes the player from the connection that sent the message, never from the
/// message (it has no field). It has no receiver in lane 0 and costs no bucket token; owner
/// `L-MATCH-SRV1` (limit the message there).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveMatch;

/// The text of `JoinRejected` for a `Hello` after the end of the match (design 6.8).
pub const REASON_MATCH_OVER: &str = "The match is over";

/// The records of protocol 5 that ride the delta next to `unit_orders`. The writers of every
/// field except `orders_cleared` are empty at the lane-0 exit; a feature lane fills its own.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DeltaRecords {
    /// Sites that completed in this tick (design 6.8, "Site existence and footprint").
    pub sites_completed: Vec<UnitId>,
    pub sites: Vec<SiteProgress>,
    pub jobs: Vec<JobProgress>,
    pub construction_queues: Vec<ConstructionQueue>,
    pub supply: Vec<SupplyRecord>,
    /// The units whose `UnitOrderRecord` left the view in this tick. A removed unit is not
    /// listed: `removed` is its only mention.
    pub orders_cleared: Vec<UnitId>,
    pub effects: Vec<Effect>,
    /// `None` means no change. The `Snapshot` holds the whole state.
    pub match_state: Option<MatchState>,
}

/// The records of protocol 5 that the `Snapshot` holds next to `unit_orders`.
///
/// Rule (ruling for `L0-STORE`, MP1 review item 6): every `Snapshot` carries `Some(match_state)`.
/// The lane-0 value for "no match state yet" is the empty state (no connection row, no outcome),
/// written by `export::snapshot_for`. A reader that finds `None` reads it as that empty state. In
/// a delta, `match_state: None` means "no change".
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SnapshotRecords {
    pub sites: Vec<SiteProgress>,
    pub jobs: Vec<JobProgress>,
    pub construction_queues: Vec<ConstructionQueue>,
    pub supply: Vec<SupplyRecord>,
    pub match_state: Option<MatchState>,
}
