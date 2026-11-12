//! The room: the lobby seats and the lifecycle phases of one match (design 6.7.2, 6.7.3,
//! 6.7.5, 6.7.6; stage 1 of MP1).
//!
//! This module is plain data and pure functions. It has no Bevy system, no message and no
//! clock. The later task `L-MATCH-SRV1` wires it into the session systems and the sim gate. A
//! connection is an opaque `ConnectionKey` (the later task uses `Entity::to_bits`). Every
//! answer is a return value; the later tasks map the values to wire messages.
//!
//! Stage 1 builds the start policy `AllSeated` with `countdown_secs` 0 and the policy
//! `Immediate` (design 6.7.4). The Countdown phase and its edges exist in the transition table
//! for stage 2, but no stage-1 code drives them.
//!
//! The room owns lobby seats only. A claim is a record in `turts_sim::Slots` (D25). The room
//! plans the claims of the start step (`StartPlan`) and the caller applies them in index order.
#![allow(dead_code)] // `L-MATCH-SRV1` wires this module; until then only the tests use it.

use turts_sim::{ClaimError, SlotId, Slots};

/// The refusal text of a connection that has no open slot (same text as `session.rs`).
pub const REASON_NO_OPEN_SLOT: &str = "no open slot";
/// The refusal text of an id that has a live connection (same text as `session.rs`).
pub const REASON_DUPLICATE_WINDOW: &str = "This game is open in another window";
/// The refusal text after the end of the match (design 6.7.5, state table, row Ended).
pub const REASON_MATCH_OVER: &str = "The match is over";

/// The phases of one process (design 6.7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    Boot,
    Lobby,
    /// Stage 2. No stage-1 code enters it.
    Countdown,
    Running,
    Ended,
}

/// One edge of the phase machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomStep {
    /// The boot ends under a lobby policy.
    BootToLobby,
    /// The boot ends under `Immediate` or `--start-now`.
    BootToRunning,
    /// Stage 2: the start policy holds and `countdown_secs` is above 0.
    LobbyToCountdown,
    /// Stage 2: an accepted seat change.
    CountdownToLobby,
    /// The start step in the frame in which the policy holds (`countdown_secs` 0).
    Start,
    /// Stage 2: the count reaches 0.
    CountdownToRunning,
    /// Every slot has an outcome (design 6.7.8).
    MatchOver,
}

impl RoomState {
    /// The state after `step`, or `None` when the table has no such edge (design 6.7.5, first
    /// table).
    pub fn next(self, step: RoomStep) -> Option<RoomState> {
        match (self, step) {
            (RoomState::Boot, RoomStep::BootToLobby) => Some(RoomState::Lobby),
            (RoomState::Boot, RoomStep::BootToRunning) => Some(RoomState::Running),
            (RoomState::Lobby, RoomStep::LobbyToCountdown) => Some(RoomState::Countdown),
            (RoomState::Lobby, RoomStep::Start) => Some(RoomState::Running),
            (RoomState::Countdown, RoomStep::CountdownToLobby) => Some(RoomState::Lobby),
            (RoomState::Countdown, RoomStep::CountdownToRunning) => Some(RoomState::Running),
            (RoomState::Running, RoomStep::MatchOver) => Some(RoomState::Ended),
            _ => None,
        }
    }
}

/// The start policy as the room sees it (design 6.7.4). `AllSeated` has `countdown_secs` 0 in
/// stage 1 (ruling R1); a value above 0 stays unbuilt until stage 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPolicy {
    /// Boot goes to Running. The claim happens at `Hello` in `Slots`, not in the room.
    Immediate,
    /// The start step runs in the frame in which the last seat fills.
    AllSeated,
}

/// What a connecting id already holds (the columns of the table in design 6.7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// The id holds no seat and no claim.
    New,
    /// The id holds a lobby seat (Lobby, Countdown).
    HoldsSeat,
    /// The id owns a claimed slot (Running, Ended). `left_match` is true after `LeaveMatch`.
    HoldsClaim { left_match: bool },
}

/// The refusals of the table (the `JoinRejected` reasons).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NoOpenSlot,
    OpenInAnotherWindow,
    MatchOver,
}

impl Refusal {
    /// The `JoinRejected` text.
    pub fn reason(self) -> &'static str {
        match self {
            Refusal::NoOpenSlot => REASON_NO_OPEN_SLOT,
            Refusal::OpenInAnotherWindow => REASON_DUPLICATE_WINDOW,
            Refusal::MatchOver => REASON_MATCH_OVER,
        }
    }
}

/// The answer of one cell of the state table (design 6.7.5, second table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinVerdict {
    /// Lobby, new id: the lowest open seat, then `LobbyState`.
    TakeSeat,
    /// Running under `Immediate`, new id: the first open slot is claimed (late admission).
    ClaimFirstOpen,
    /// Running, id with a claim and no live connection: `Welcome`, `Snapshot`, then deltas.
    Reconnect,
    /// Ended, owner that did not leave: `Welcome`, `Snapshot`, `MatchResult`.
    ReconnectEnded,
    Refuse(Refusal),
    /// The table says "not possible" (a seat is released when its connection ends), or the
    /// state has no row (Boot). The caller logs it and refuses the connection.
    Impossible,
}

/// The state table of design 6.7.5, one cell for each call. The protocol and lockfile checks
/// come first in every state and are not part of this function.
///
/// `has_open_slot` means an open lobby seat (Lobby) or an open slot in `Slots` (Running under
/// `Immediate`). It is ignored in every other cell.
pub fn judge_join(
    state: RoomState,
    policy: StartPolicy,
    standing: Standing,
    has_live_connection: bool,
    has_open_slot: bool,
) -> JoinVerdict {
    match state {
        RoomState::Boot => JoinVerdict::Impossible,
        RoomState::Lobby | RoomState::Countdown => {
            if has_live_connection {
                return JoinVerdict::Refuse(Refusal::OpenInAnotherWindow);
            }
            match standing {
                Standing::New if state == RoomState::Lobby && has_open_slot => {
                    JoinVerdict::TakeSeat
                }
                // Countdown: every seat is full under `AllSeatedReady`.
                Standing::New => JoinVerdict::Refuse(Refusal::NoOpenSlot),
                Standing::HoldsSeat | Standing::HoldsClaim { .. } => JoinVerdict::Impossible,
            }
        }
        RoomState::Running => {
            if has_live_connection {
                return JoinVerdict::Refuse(Refusal::OpenInAnotherWindow);
            }
            match standing {
                Standing::New if policy == StartPolicy::Immediate && has_open_slot => {
                    JoinVerdict::ClaimFirstOpen
                }
                Standing::New => JoinVerdict::Refuse(Refusal::NoOpenSlot),
                Standing::HoldsClaim { .. } => JoinVerdict::Reconnect,
                Standing::HoldsSeat => JoinVerdict::Impossible,
            }
        }
        RoomState::Ended => {
            if has_live_connection {
                return JoinVerdict::Refuse(Refusal::OpenInAnotherWindow);
            }
            match standing {
                Standing::HoldsClaim { left_match: false } => JoinVerdict::ReconnectEnded,
                Standing::New | Standing::HoldsClaim { left_match: true } => {
                    JoinVerdict::Refuse(Refusal::MatchOver)
                }
                Standing::HoldsSeat => JoinVerdict::Impossible,
            }
        }
    }
}

/// One transport connection. The room never reads inside the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionKey(pub u64);

/// One occupied lobby seat. After the start step the seat keeps its player: it is then the
/// record of a claim, and it is never released (D25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    pub player_id: u64,
    /// The live connection, or `None` after a connection ended in Running or Ended.
    pub connection: Option<ConnectionKey>,
    /// An index into the interface palette `slot_colours` (design 6.7.4, 6.7.6).
    pub colour: u8,
    /// True after the start step claimed the slot with this index.
    pub claimed: bool,
    /// True after `LeaveMatch` (design 6.7.5, row Ended).
    pub left_match: bool,
}

/// Why `SeatTable::occupy` refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatError {
    NoOpenSeat,
    /// No palette index is free. The pack-time check (6.7.4) makes this impossible for a
    /// package with a lobby policy.
    NoFreeColour,
    /// The id holds a seat.
    PlayerSeated,
    /// The connection holds a seat.
    ConnectionSeated,
}

/// The seats of the lobby (design 6.7.3, 6.7.6). The seat index is the player-slot index.
#[derive(Debug, Clone)]
pub struct SeatTable {
    seats: Vec<Option<Seat>>,
    palette_len: u8,
    revision: u32,
}

impl SeatTable {
    /// `seat_count` is the player-slot count (`Slots::count`). `palette_len` is the length of
    /// the interface `slot_colours` list.
    pub fn new(seat_count: u8, palette_len: u8) -> Self {
        Self {
            seats: vec![None; usize::from(seat_count)],
            palette_len,
            revision: 0,
        }
    }

    pub fn seat_count(&self) -> u8 {
        self.seats.len() as u8
    }

    /// The number of accepted changes of an occupant (the `LobbyState` revision).
    pub fn revision(&self) -> u32 {
        self.revision
    }

    pub fn seat(&self, index: u8) -> Option<&Seat> {
        self.seats.get(usize::from(index)).and_then(Option::as_ref)
    }

    /// The label of a seat: `Player N`, N = index + 1. It is not an identity (S:410).
    pub fn label(index: u8) -> String {
        format!("Player {}", u16::from(index) + 1)
    }

    pub fn lowest_open_seat(&self) -> Option<u8> {
        self.seats.iter().position(Option::is_none).map(|i| i as u8)
    }

    /// The lowest palette index that no seat holds (design 6.7.6).
    pub fn lowest_free_colour(&self) -> Option<u8> {
        (0..self.palette_len).find(|colour| {
            self.seats
                .iter()
                .flatten()
                .all(|seat| seat.colour != *colour)
        })
    }

    /// Whether `occupy` can succeed for a new id and a new connection.
    pub fn can_seat(&self) -> bool {
        self.lowest_open_seat().is_some() && self.lowest_free_colour().is_some()
    }

    pub fn seat_of_player(&self, player_id: u64) -> Option<u8> {
        self.seats
            .iter()
            .position(|seat| seat.as_ref().is_some_and(|s| s.player_id == player_id))
            .map(|i| i as u8)
    }

    pub fn seat_of_connection(&self, connection: ConnectionKey) -> Option<u8> {
        self.seats
            .iter()
            .position(|seat| {
                seat.as_ref()
                    .is_some_and(|s| s.connection == Some(connection))
            })
            .map(|i| i as u8)
    }

    /// Give the lowest open seat and the lowest free colour to a new id and connection. One
    /// caller judges the `Hello` messages one after the other, so two ids never get one seat.
    pub fn occupy(&mut self, connection: ConnectionKey, player_id: u64) -> Result<u8, SeatError> {
        if self.seat_of_player(player_id).is_some() {
            return Err(SeatError::PlayerSeated);
        }
        if self.seat_of_connection(connection).is_some() {
            return Err(SeatError::ConnectionSeated);
        }
        let index = self.lowest_open_seat().ok_or(SeatError::NoOpenSeat)?;
        let colour = self.lowest_free_colour().ok_or(SeatError::NoFreeColour)?;
        self.seats[usize::from(index)] = Some(Seat {
            player_id,
            connection: Some(connection),
            colour,
            claimed: false,
            left_match: false,
        });
        self.revision += 1;
        Ok(index)
    }

    /// Open a lobby seat again. A claimed seat is never released (D25): the call returns
    /// `false` and changes nothing.
    pub fn release(&mut self, index: u8) -> bool {
        let slot = self.seats.get_mut(usize::from(index));
        match slot {
            Some(entry) if entry.as_ref().is_some_and(|seat| !seat.claimed) => {
                *entry = None;
                self.revision += 1;
                true
            }
            _ => false,
        }
    }

    /// Every seat holds a live connection. A table with no seat is never full.
    pub fn all_seated(&self) -> bool {
        !self.seats.is_empty()
            && self
                .seats
                .iter()
                .all(|seat| seat.as_ref().is_some_and(|s| s.connection.is_some()))
    }

    #[cfg(test)]
    fn set_colour_for_test(&mut self, index: u8, colour: u8) {
        self.seats[usize::from(index)]
            .as_mut()
            .expect("the seat is occupied")
            .colour = colour;
    }
}

/// What `Room::join` decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinOutcome {
    /// A lobby seat was taken: send `LobbyState { you: seat }` and the next revision.
    Seated {
        seat: u8,
        colour: u8,
        label: String,
        revision: u32,
    },
    /// A claimed slot has its connection again (Running or Ended).
    Reconnected {
        slot: SlotId,
        ended: bool,
    },
    /// A second `Hello` on a connection that holds a seat: one log line, no answer
    /// (design 6.7.5, event list).
    Ignored,
    Refused(Refusal),
    /// The policy is `Immediate`: the claim is a `Slots::claim` at `Hello`, as today.
    NotRoomPolicy,
    /// The table has no live cell here (see `JoinVerdict::Impossible`).
    Impossible,
}

/// What `Room::connection_ended` decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEnd {
    /// The connection held no seat: nothing changes.
    Unknown,
    /// Lobby: the seat is open again, the revision went up. Tell the other seats.
    SeatReleased { seat: u8, revision: u32 },
    /// After the start: the slot stays claimed. The later task writes
    /// `SlotConnection { slot, connected: false }` (design 6.7.8).
    SlotDisconnected { slot: SlotId },
}

/// Why `Room::start` refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartError {
    NotInLobby,
    NotAllSeated,
}

/// One claim of the start step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedClaim {
    pub slot: SlotId,
    pub player_id: u64,
    pub connection: ConnectionKey,
    pub colour: u8,
}

/// The claims of the start step in index order: seat i gets slot i (design 6.7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartPlan {
    pub claims: Vec<PlannedClaim>,
}

/// `Slots::claim` gave another slot than the seat index, or refused. The start step exits with
/// code 1 (design 6.7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimMismatch {
    pub seat: u8,
    pub got: Result<SlotId, ClaimError>,
}

impl StartPlan {
    /// Claim each seat in `Slots`, in index order, and check that seat i gets slot i. All
    /// slots are open in a lobby process, so a difference cannot happen.
    pub fn claim_in(&self, slots: &mut Slots) -> Result<(), ClaimMismatch> {
        for claim in &self.claims {
            let got = slots.claim(claim.player_id);
            if got != Ok(claim.slot) {
                return Err(ClaimMismatch {
                    seat: claim.slot.0,
                    got,
                });
            }
        }
        Ok(())
    }
}

/// The room: the phase, the policy and the seats.
#[derive(Debug, Clone)]
pub struct Room {
    state: RoomState,
    policy: StartPolicy,
    seats: SeatTable,
}

impl Room {
    /// Run the boot step (design 6.7.5, row Boot): a lobby policy goes to Lobby; `Immediate`
    /// or `start_now` goes to Running. `seat_count` is `Slots::count()`.
    pub fn boot(policy: StartPolicy, start_now: bool, seat_count: u8, palette_len: u8) -> Self {
        let policy = if start_now {
            StartPolicy::Immediate
        } else {
            policy
        };
        let step = match policy {
            StartPolicy::Immediate => RoomStep::BootToRunning,
            StartPolicy::AllSeated => RoomStep::BootToLobby,
        };
        Self {
            state: RoomState::Boot
                .next(step)
                .expect("the table has both boot edges"),
            policy,
            seats: SeatTable::new(seat_count, palette_len),
        }
    }

    pub fn state(&self) -> RoomState {
        self.state
    }

    pub fn policy(&self) -> StartPolicy {
        self.policy
    }

    pub fn seats(&self) -> &SeatTable {
        &self.seats
    }

    /// Decide an accepted `Hello` (protocol and lockfile already checked). Call it for the
    /// `Hello` messages of one frame one after the other.
    pub fn join(&mut self, connection: ConnectionKey, player_id: u64) -> JoinOutcome {
        if self.policy == StartPolicy::Immediate {
            return JoinOutcome::NotRoomPolicy;
        }
        if self.seats.seat_of_connection(connection).is_some() {
            return JoinOutcome::Ignored;
        }
        let held = self.seats.seat_of_player(player_id);
        let claimed_state = matches!(self.state, RoomState::Running | RoomState::Ended);
        let standing = match held.and_then(|index| self.seats.seat(index)) {
            None => Standing::New,
            Some(seat) if claimed_state => Standing::HoldsClaim {
                left_match: seat.left_match,
            },
            Some(_) => Standing::HoldsSeat,
        };
        let live = held
            .and_then(|index| self.seats.seat(index))
            .is_some_and(|seat| seat.connection.is_some());
        let verdict = judge_join(
            self.state,
            self.policy,
            standing,
            live,
            self.seats.can_seat(),
        );
        match verdict {
            JoinVerdict::TakeSeat => match self.seats.occupy(connection, player_id) {
                Ok(seat) => {
                    let colour = self
                        .seats
                        .seat(seat)
                        .expect("the seat was just taken")
                        .colour;
                    JoinOutcome::Seated {
                        seat,
                        colour,
                        label: SeatTable::label(seat),
                        revision: self.seats.revision(),
                    }
                }
                Err(_) => JoinOutcome::Refused(Refusal::NoOpenSlot),
            },
            JoinVerdict::Reconnect | JoinVerdict::ReconnectEnded => {
                let index = held.expect("a reconnect needs a seat record");
                self.seats.seats[usize::from(index)]
                    .as_mut()
                    .expect("the seat is occupied")
                    .connection = Some(connection);
                JoinOutcome::Reconnected {
                    slot: SlotId(index),
                    ended: verdict == JoinVerdict::ReconnectEnded,
                }
            }
            JoinVerdict::Refuse(refusal) => JoinOutcome::Refused(refusal),
            JoinVerdict::ClaimFirstOpen => JoinOutcome::NotRoomPolicy,
            JoinVerdict::Impossible => JoinOutcome::Impossible,
        }
    }

    /// The end of one connection (design 6.7.5: a lobby seat is released when its connection
    /// ends; there is no other rule). Call it before `join` and `start_due` in the same frame.
    pub fn connection_ended(&mut self, connection: ConnectionKey) -> ConnectionEnd {
        let Some(index) = self.seats.seat_of_connection(connection) else {
            return ConnectionEnd::Unknown;
        };
        let claimed = self.seats.seat(index).is_some_and(|seat| seat.claimed);
        if claimed {
            self.seats.seats[usize::from(index)]
                .as_mut()
                .expect("the seat is occupied")
                .connection = None;
            return ConnectionEnd::SlotDisconnected {
                slot: SlotId(index),
            };
        }
        self.seats.release(index);
        if let Some(state) = self.state.next(RoomStep::CountdownToLobby) {
            self.state = state;
        }
        ConnectionEnd::SeatReleased {
            seat: index,
            revision: self.seats.revision(),
        }
    }

    /// The start condition of `AllSeated(0)`: every slot has a seated connection. There is no
    /// minimum-player rule, no auto-fill and no deadline (D23).
    pub fn start_due(&self) -> bool {
        self.state == RoomState::Lobby
            && self.policy == StartPolicy::AllSeated
            && self.seats.all_seated()
    }

    /// The start step, room part: mark every seat claimed, go to Running and return the claims
    /// in index order. The caller applies them with `StartPlan::claim_in`.
    pub fn start(&mut self) -> Result<StartPlan, StartError> {
        if self.state != RoomState::Lobby || self.policy != StartPolicy::AllSeated {
            return Err(StartError::NotInLobby);
        }
        if !self.seats.all_seated() {
            return Err(StartError::NotAllSeated);
        }
        let claims = self
            .seats
            .seats
            .iter()
            .enumerate()
            .map(|(index, seat)| {
                let seat = seat.as_ref().expect("all_seated checked the seat");
                PlannedClaim {
                    slot: SlotId(index as u8),
                    player_id: seat.player_id,
                    connection: seat.connection.expect("all_seated checked the connection"),
                    colour: seat.colour,
                }
            })
            .collect();
        for seat in self.seats.seats.iter_mut().flatten() {
            seat.claimed = true;
        }
        self.state = self
            .state
            .next(RoomStep::Start)
            .expect("the table has the start edge");
        Ok(StartPlan { claims })
    }

    /// Every slot has an outcome: Running goes to Ended. `false` in any other state.
    pub fn end_match(&mut self) -> bool {
        match self.state.next(RoomStep::MatchOver) {
            Some(state) => {
                self.state = state;
                true
            }
            None => false,
        }
    }

    /// Record `LeaveMatch` of an owner. `false` when the id holds no claim.
    pub fn leave_match(&mut self, player_id: u64) -> bool {
        let Some(index) = self.seats.seat_of_player(player_id) else {
            return false;
        };
        let seat = self.seats.seats[usize::from(index)]
            .as_mut()
            .expect("the seat is occupied");
        if !seat.claimed {
            return false;
        }
        seat.left_match = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: ConnectionKey = ConnectionKey(1);
    const B: ConnectionKey = ConnectionKey(2);
    const C: ConnectionKey = ConnectionKey(3);
    const ID_A: u64 = 100;
    const ID_B: u64 = 200;
    const ID_C: u64 = 300;

    fn lobby() -> Room {
        Room::boot(StartPolicy::AllSeated, false, 2, 2)
    }

    fn seated(outcome: &JoinOutcome) -> (u8, u8) {
        match outcome {
            JoinOutcome::Seated { seat, colour, .. } => (*seat, *colour),
            other => panic!("expected a seat, got {other:?}"),
        }
    }

    // ---- The phase machine (design 6.7.2, first table of 6.7.5) ----

    #[test]
    fn boot_goes_to_lobby_under_a_lobby_policy() {
        let room = Room::boot(StartPolicy::AllSeated, false, 2, 2);
        assert_eq!(room.state(), RoomState::Lobby);
    }

    #[test]
    fn boot_goes_to_running_under_immediate_and_under_start_now() {
        assert_eq!(
            Room::boot(StartPolicy::Immediate, false, 2, 2).state(),
            RoomState::Running
        );
        let forced = Room::boot(StartPolicy::AllSeated, true, 2, 2);
        assert_eq!(forced.state(), RoomState::Running);
        assert_eq!(forced.policy(), StartPolicy::Immediate);
    }

    #[test]
    fn the_transition_table_has_exactly_the_edges_of_the_state_machine() {
        use RoomState::*;
        use RoomStep::*;
        let states = [Boot, Lobby, Countdown, Running, Ended];
        let steps = [
            BootToLobby,
            BootToRunning,
            LobbyToCountdown,
            CountdownToLobby,
            Start,
            CountdownToRunning,
            MatchOver,
        ];
        let mut edges = Vec::new();
        for state in states {
            for step in steps {
                if let Some(next) = state.next(step) {
                    edges.push((state, step, next));
                }
            }
        }
        assert_eq!(
            edges,
            vec![
                (Boot, BootToLobby, Lobby),
                (Boot, BootToRunning, Running),
                (Lobby, LobbyToCountdown, Countdown),
                (Lobby, Start, Running),
                (Countdown, CountdownToLobby, Lobby),
                (Countdown, CountdownToRunning, Running),
                (Running, MatchOver, Ended),
            ]
        );
    }

    // ---- The state table of 6.7.5, one test for each cell that stage 1 needs ----

    #[test]
    fn lobby_new_id_gets_the_lowest_open_seat_then_no_open_slot() {
        let mut room = lobby();
        assert_eq!(seated(&room.join(A, ID_A)), (0, 0));
        assert_eq!(seated(&room.join(B, ID_B)), (1, 1));
        // The cell "no open seat", judged on the pure function.
        assert_eq!(
            judge_join(
                RoomState::Lobby,
                StartPolicy::AllSeated,
                Standing::New,
                false,
                false
            ),
            JoinVerdict::Refuse(Refusal::NoOpenSlot)
        );
        assert_eq!(Refusal::NoOpenSlot.reason(), "no open slot");
    }

    #[test]
    fn lobby_new_id_with_every_seat_taken_is_refused_by_the_room() {
        // Both seats are taken and the caller has not run the start step yet.
        let mut room = lobby();
        room.join(A, ID_A);
        assert_eq!(seated(&room.join(B, ID_B)), (1, 1));
        assert_eq!(
            room.join(C, ID_C),
            JoinOutcome::Refused(Refusal::NoOpenSlot)
        );
    }

    #[test]
    fn lobby_id_with_a_live_connection_gets_open_in_another_window() {
        let mut room = lobby();
        room.join(A, ID_A);
        let outcome = room.join(B, ID_A);
        assert_eq!(outcome, JoinOutcome::Refused(Refusal::OpenInAnotherWindow));
        assert_eq!(
            Refusal::OpenInAnotherWindow.reason(),
            "This game is open in another window"
        );
        // The refused Hello took no seat.
        assert_eq!(room.seats().lowest_open_seat(), Some(1));
    }

    #[test]
    fn lobby_id_with_a_seat_and_no_live_connection_is_not_possible() {
        assert_eq!(
            judge_join(
                RoomState::Lobby,
                StartPolicy::AllSeated,
                Standing::HoldsSeat,
                false,
                true
            ),
            JoinVerdict::Impossible
        );
        // The room keeps the cell empty: the connection end releases the seat.
        let mut room = lobby();
        room.join(A, ID_A);
        room.connection_ended(A);
        assert_eq!(room.seats().seat_of_player(ID_A), None);
    }

    #[test]
    fn countdown_cells_are_no_open_slot_and_another_window() {
        // Stage 2 state: the cells exist so the table is whole.
        assert_eq!(
            judge_join(
                RoomState::Countdown,
                StartPolicy::AllSeated,
                Standing::New,
                false,
                true
            ),
            JoinVerdict::Refuse(Refusal::NoOpenSlot)
        );
        assert_eq!(
            judge_join(
                RoomState::Countdown,
                StartPolicy::AllSeated,
                Standing::HoldsSeat,
                true,
                false
            ),
            JoinVerdict::Refuse(Refusal::OpenInAnotherWindow)
        );
        assert_eq!(
            judge_join(
                RoomState::Countdown,
                StartPolicy::AllSeated,
                Standing::HoldsSeat,
                false,
                false
            ),
            JoinVerdict::Impossible
        );
    }

    fn started() -> Room {
        let mut room = lobby();
        room.join(A, ID_A);
        room.join(B, ID_B);
        room.start().expect("both seats are filled");
        room
    }

    #[test]
    fn running_after_a_lobby_start_new_id_gets_no_open_slot() {
        let mut room = started();
        assert_eq!(room.state(), RoomState::Running);
        assert_eq!(
            room.join(C, ID_C),
            JoinOutcome::Refused(Refusal::NoOpenSlot)
        );
        // Even when a connection of an owner ended: the slot stays claimed (D25).
        room.connection_ended(A);
        assert_eq!(
            room.join(C, ID_C),
            JoinOutcome::Refused(Refusal::NoOpenSlot)
        );
    }

    #[test]
    fn running_owner_without_a_live_connection_reconnects_to_its_slot() {
        let mut room = started();
        assert_eq!(
            room.connection_ended(A),
            ConnectionEnd::SlotDisconnected { slot: SlotId(0) }
        );
        assert_eq!(
            room.join(C, ID_A),
            JoinOutcome::Reconnected {
                slot: SlotId(0),
                ended: false
            }
        );
        assert_eq!(room.seats().seat_of_connection(C), Some(0));
    }

    #[test]
    fn running_owner_with_a_live_connection_gets_another_window() {
        let mut room = started();
        assert_eq!(
            room.join(C, ID_A),
            JoinOutcome::Refused(Refusal::OpenInAnotherWindow)
        );
    }

    #[test]
    fn running_under_immediate_the_first_open_slot_is_claimed() {
        let judge = |standing, live, open| {
            judge_join(
                RoomState::Running,
                StartPolicy::Immediate,
                standing,
                live,
                open,
            )
        };
        assert_eq!(
            judge(Standing::New, false, true),
            JoinVerdict::ClaimFirstOpen
        );
        assert_eq!(
            judge(Standing::New, false, false),
            JoinVerdict::Refuse(Refusal::NoOpenSlot)
        );
        assert_eq!(
            judge(Standing::HoldsClaim { left_match: false }, false, false),
            JoinVerdict::Reconnect
        );
        assert_eq!(
            judge(Standing::HoldsClaim { left_match: false }, true, false),
            JoinVerdict::Refuse(Refusal::OpenInAnotherWindow)
        );
        // The room does not own the claim under Immediate: `Slots::claim` at `Hello`.
        let mut room = Room::boot(StartPolicy::Immediate, false, 2, 2);
        assert_eq!(room.join(A, ID_A), JoinOutcome::NotRoomPolicy);
    }

    #[test]
    fn a_lobby_policy_start_does_not_open_a_slot_for_a_new_id() {
        // The same facts under AllSeated: has_open_slot is ignored in Running.
        assert_eq!(
            judge_join(
                RoomState::Running,
                StartPolicy::AllSeated,
                Standing::New,
                false,
                true
            ),
            JoinVerdict::Refuse(Refusal::NoOpenSlot)
        );
    }

    #[test]
    fn ended_cells_follow_the_table() {
        let mut room = started();
        assert!(room.end_match());
        assert_eq!(room.state(), RoomState::Ended);
        // New id.
        assert_eq!(room.join(C, ID_C), JoinOutcome::Refused(Refusal::MatchOver));
        assert_eq!(Refusal::MatchOver.reason(), "The match is over");
        // Owner that did not leave, with no live connection.
        room.connection_ended(A);
        assert_eq!(
            room.join(C, ID_A),
            JoinOutcome::Reconnected {
                slot: SlotId(0),
                ended: true
            }
        );
        // Owner that left.
        assert!(room.leave_match(ID_B));
        room.connection_ended(B);
        assert_eq!(
            room.join(ConnectionKey(9), ID_B),
            JoinOutcome::Refused(Refusal::MatchOver)
        );
        // Owner with a live connection.
        assert_eq!(
            room.join(ConnectionKey(10), ID_A),
            JoinOutcome::Refused(Refusal::OpenInAnotherWindow)
        );
    }

    #[test]
    fn boot_has_no_live_cell() {
        assert_eq!(
            judge_join(
                RoomState::Boot,
                StartPolicy::AllSeated,
                Standing::New,
                false,
                true
            ),
            JoinVerdict::Impossible
        );
    }

    #[test]
    fn the_refusal_texts_are_the_texts_of_session_rs() {
        let source = include_str!("session.rs");
        for text in [REASON_NO_OPEN_SLOT, REASON_DUPLICATE_WINDOW] {
            assert!(
                source.contains(&format!("\"{text}\"")),
                "session.rs has no `{text}`"
            );
        }
    }

    // ---- The seat table (design 6.7.3, 6.7.6) ----

    #[test]
    fn the_seat_count_is_the_player_slot_count() {
        let slots = Slots::new(2);
        let table = SeatTable::new(slots.count(), 2);
        assert_eq!(table.seat_count(), 2);
    }

    #[test]
    fn the_label_is_player_n_with_n_the_seat_index_plus_one() {
        assert_eq!(SeatTable::label(0), "Player 1");
        assert_eq!(SeatTable::label(1), "Player 2");
        let mut room = lobby();
        match room.join(A, ID_A) {
            JoinOutcome::Seated { label, .. } => assert_eq!(label, "Player 1"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_lowest_open_seat_is_reused_after_a_release() {
        let mut room = lobby();
        room.join(A, ID_A);
        room.join(B, ID_B);
        // Seat 0 leaves. The next id takes seat 0, not a higher seat.
        assert_eq!(
            room.connection_ended(A),
            ConnectionEnd::SeatReleased {
                seat: 0,
                revision: 3
            }
        );
        assert_eq!(seated(&room.join(C, ID_C)), (0, 0));
        // Seat 1 is still the second player.
        assert_eq!(room.seats().seat(1).map(|s| s.player_id), Some(ID_B));
    }

    #[test]
    fn a_new_seat_gets_the_lowest_palette_index_that_no_seat_holds() {
        let mut room = lobby();
        let first = seated(&room.join(A, ID_A));
        let second = seated(&room.join(B, ID_B));
        assert_ne!(
            first.1, second.1,
            "the two seats start with different colours"
        );
        assert_eq!((first.1, second.1), (0, 1));
        // A colour that a leave frees is the lowest free one again.
        room.connection_ended(B);
        assert_eq!(seated(&room.join(C, ID_C)), (1, 1));
    }

    #[test]
    fn the_lowest_free_colour_is_not_the_seat_index() {
        // Stage 1 never makes a seat colour differ from its index (stage 2 `SetColour` does).
        // The test table is built so that seat 0 holds colour 1: the next seat is seat 1 and
        // its colour must be 0, the lowest index that no seat holds.
        let mut table = SeatTable::new(2, 2);
        table.occupy(A, ID_A).unwrap();
        table.set_colour_for_test(0, 1);
        let seat = table.occupy(B, ID_B).unwrap();
        assert_eq!(seat, 1);
        assert_eq!(table.seat(1).map(|s| s.colour), Some(0));
    }

    #[test]
    fn no_free_colour_refuses_the_seat() {
        let mut table = SeatTable::new(2, 1);
        table.occupy(A, ID_A).unwrap();
        assert_eq!(table.occupy(B, ID_B), Err(SeatError::NoFreeColour));
        assert!(!table.can_seat());
    }

    #[test]
    fn two_hello_messages_in_one_frame_get_two_seats_or_one_refusal() {
        let mut room = lobby();
        // Same id on two connections in one frame.
        assert_eq!(seated(&room.join(A, ID_A)), (0, 0));
        assert_eq!(
            room.join(B, ID_A),
            JoinOutcome::Refused(Refusal::OpenInAnotherWindow)
        );
        // Two different ids in one frame get two different seats.
        assert_eq!(seated(&room.join(B, ID_B)), (1, 1));
    }

    #[test]
    fn a_second_hello_on_a_connection_with_a_seat_is_ignored() {
        let mut room = lobby();
        room.join(A, ID_A);
        let revision = room.seats().revision();
        // Same id and another id: both ignored, nothing changes.
        assert_eq!(room.join(A, ID_A), JoinOutcome::Ignored);
        assert_eq!(room.join(A, ID_B), JoinOutcome::Ignored);
        assert_eq!(room.seats().revision(), revision);
        assert_eq!(room.seats().seat_of_player(ID_B), None);
        // After the start the connection still holds its seat record: still ignored.
        room.join(B, ID_B);
        room.start().unwrap();
        assert_eq!(room.join(A, ID_A), JoinOutcome::Ignored);
    }

    // ---- Release, start condition and start step ----

    #[test]
    fn a_lobby_seat_is_released_when_its_connection_ends_and_only_then() {
        let mut room = lobby();
        room.join(A, ID_A);
        assert_eq!(
            room.connection_ended(ConnectionKey(77)),
            ConnectionEnd::Unknown
        );
        assert_eq!(room.seats().seat_of_player(ID_A), Some(0));
        assert_eq!(
            room.connection_ended(A),
            ConnectionEnd::SeatReleased {
                seat: 0,
                revision: 2
            }
        );
        assert_eq!(room.seats().seat_of_player(ID_A), None);
        assert_eq!(room.state(), RoomState::Lobby);
        // The same id joins again at once with a new connection (the reload).
        assert_eq!(seated(&room.join(B, ID_A)), (0, 0));
    }

    #[test]
    fn the_start_waits_for_every_seat_with_no_minimum_and_no_deadline() {
        let mut room = lobby();
        assert!(!room.start_due());
        room.join(A, ID_A);
        assert!(!room.start_due());
        assert_eq!(room.start(), Err(StartError::NotAllSeated));
        assert_eq!(room.state(), RoomState::Lobby);
        room.join(B, ID_B);
        assert!(room.start_due());
    }

    #[test]
    fn an_empty_table_never_starts() {
        let room = Room::boot(StartPolicy::AllSeated, false, 0, 2);
        assert!(!room.start_due());
    }

    #[test]
    fn a_connection_end_in_the_frame_comes_before_the_start() {
        // The receive systems run first: the end of A, then the Hello of B, then the start
        // test. Seat 0 is open again, so the start is not due.
        let mut room = lobby();
        room.join(A, ID_A);
        room.connection_ended(A);
        room.join(B, ID_B);
        assert!(!room.start_due());
        assert_eq!(room.state(), RoomState::Lobby);
    }

    #[test]
    fn the_start_claims_seat_i_to_slot_i_in_index_order() {
        let mut room = lobby();
        // Seat 0 is taken and freed, then taken by another id: the order is by seat, not by
        // time of the Hello.
        room.join(A, ID_A);
        room.join(B, ID_B);
        room.connection_ended(A);
        room.join(C, ID_C);
        assert!(room.start_due());
        let plan = room.start().unwrap();
        assert_eq!(room.state(), RoomState::Running);
        let claims: Vec<_> = plan
            .claims
            .iter()
            .map(|claim| (claim.slot, claim.player_id, claim.connection))
            .collect();
        assert_eq!(claims, vec![(SlotId(0), ID_C, C), (SlotId(1), ID_B, B)]);
        let mut slots = Slots::new(2);
        plan.claim_in(&mut slots).expect("seat i gets slot i");
        assert_eq!(slots.owner_of(SlotId(0)), Some(ID_C));
        assert_eq!(slots.owner_of(SlotId(1)), Some(ID_B));
    }

    #[test]
    fn the_claim_step_reports_a_slot_that_is_not_the_seat_index() {
        let plan = StartPlan {
            claims: vec![PlannedClaim {
                slot: SlotId(0),
                player_id: ID_A,
                connection: A,
                colour: 0,
            }],
        };
        let mut slots = Slots::new(2);
        slots.claim(999).unwrap(); // slot 0 is not open
        assert_eq!(
            plan.claim_in(&mut slots),
            Err(ClaimMismatch {
                seat: 0,
                got: Ok(SlotId(1))
            })
        );
    }

    #[test]
    fn the_start_runs_once_and_only_in_the_lobby() {
        let mut room = started();
        assert_eq!(room.start(), Err(StartError::NotInLobby));
        assert!(!room.start_due());
        let mut immediate = Room::boot(StartPolicy::Immediate, false, 2, 2);
        assert_eq!(immediate.start(), Err(StartError::NotInLobby));
    }

    #[test]
    fn a_connection_end_after_the_start_never_releases_the_claim() {
        let mut room = started();
        assert_eq!(
            room.connection_ended(B),
            ConnectionEnd::SlotDisconnected { slot: SlotId(1) }
        );
        let seat = room.seats().seat(1).expect("the seat record stays");
        assert!(seat.claimed);
        assert_eq!(seat.player_id, ID_B);
        assert_eq!(seat.connection, None);
        // No lobby edge is taken in Running.
        assert_eq!(room.state(), RoomState::Running);
    }

    #[test]
    fn a_reload_that_the_start_overtakes_rejoins_as_a_reconnect() {
        // Stage 1, design 6.7.5: a seated player reloads and the other seat fills before the
        // server notices the end of the old connection.
        let mut room = lobby();
        room.join(A, ID_A);
        // The reload opens connection C with the same id: the old one is still live.
        assert_eq!(
            room.join(C, ID_A),
            JoinOutcome::Refused(Refusal::OpenInAnotherWindow)
        );
        room.join(B, ID_B);
        assert!(room.start_due());
        room.start().unwrap();
        // The server notices the end of the old connection after the start.
        assert_eq!(
            room.connection_ended(A),
            ConnectionEnd::SlotDisconnected { slot: SlotId(0) }
        );
        // The retry of the reload gets its slot back.
        assert_eq!(
            room.join(C, ID_A),
            JoinOutcome::Reconnected {
                slot: SlotId(0),
                ended: false
            }
        );
    }

    #[test]
    fn leave_match_needs_a_claim() {
        let mut room = lobby();
        room.join(A, ID_A);
        assert!(!room.leave_match(ID_A), "a lobby seat is not a claim");
        assert!(!room.leave_match(ID_C), "an unknown id holds no claim");
    }

    #[test]
    fn end_match_runs_only_from_running() {
        let mut room = lobby();
        assert!(!room.end_match());
        assert_eq!(room.state(), RoomState::Lobby);
    }
}
