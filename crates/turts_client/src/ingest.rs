//! Message ingest.
//!
//! The ingest system reads `Welcome`, `Snapshot` and `UnitDeltas` in `PreUpdate`, after
//! `MessageSystems::Receive`, and feeds them to the `Replica` (design section 5). The logic is
//! in pure functions: [`ingest_one`] handles one message, [`ingest_batch`] orders the messages of
//! one frame. The functions return the messages to send, so a test needs no network.
//!
//! The test hooks (plan R3) live here: dropped deltas, a dropped arrival delta, and held
//! `Snapshot` answers. They make the resync paths testable on the shipped binary.

use bevy::platform::time::Instant;
use bevy::prelude::*;
use lightyear::prelude::*;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::{
    CommandAck, CommandRejected, Pos, ResyncRequest, Snapshot, UnitDeltas, Welcome,
};

use crate::replica::{DeltaOutcome, Replica, SnapshotOutcome};

/// The buffered deltas of a replica that waits for a `Snapshot` and span this many ticks or more
/// make the client ask for one. This covers a lost or late `Welcome` `Snapshot`, so the client
/// never hangs in the initial resync state (G2b, studio call N = 10).
pub const IDLE_RESYNC_SPAN_TICKS: u32 = 10;

/// A message that the server sent and that the replica needs.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Welcome(Welcome),
    Snapshot(Snapshot),
    Deltas(UnitDeltas),
}

impl Incoming {
    /// The order in which one frame handles the messages: `Welcome` first, because a `Snapshot`
    /// that is read before `Replica::new` is lost (G2b).
    fn rank(&self) -> u8 {
        match self {
            Incoming::Welcome(_) => 0,
            Incoming::Snapshot(_) => 1,
            Incoming::Deltas(_) => 2,
        }
    }
}

/// A message that the client must send after ingest.
#[derive(Debug, Clone, PartialEq)]
pub enum Outgoing {
    ResyncRequest,
}

/// What the `Welcome` told the client.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WelcomeInfo {
    pub slot: u8,
    pub tick: u32,
    pub tick_hz: u16,
}

/// The client copy of the world. `replica` is `None` until the `Welcome` arrives.
#[derive(Resource, Default)]
pub struct ReplicaState {
    pub replica: Option<Replica>,
    pub welcome: Option<WelcomeInfo>,
}

/// The test hooks and the resync bookkeeping of ingest.
#[derive(Resource, Default)]
pub struct IngestHooks {
    /// Offsets from the `Welcome` tick (from `--drop-delta-ticks`).
    pub drop_offsets: Vec<u32>,
    /// Absolute ticks of the deltas to ignore: `welcome_tick + offset`, filled at `Welcome`.
    pub drop_ticks: Vec<u32>,
    /// Ignore the first delta that puts the arrival unit at `arrival_target`.
    pub drop_arrival_delta: bool,
    /// The target of the automatic move. Used with `drop_arrival_delta`.
    pub arrival_target: Option<Pos>,
    /// The unit of the automatic move. `None` matches any unit.
    pub arrival_unit: Option<u32>,
    /// Hold each `Snapshot` that answers a request back by this many ticks.
    pub hold_resync_snapshots: u32,
    /// Held answers: the `Snapshot` and the delta tick at which it is released.
    pub held: Vec<(Snapshot, u32)>,
    /// True from the moment a `ResyncRequest` is sent until its answer is applied.
    pub outstanding_request: bool,
    /// The highest delta tick received, dropped deltas included. It is the clock of the hold.
    pub highest_delta_tick: Option<u32>,
}

#[derive(Resource, Default, Debug)]
pub struct IngestCounters {
    pub deltas_received: u32,
    /// The ticks of the ignored deltas.
    pub deltas_dropped: Vec<u32>,
    pub resync_requests_sent: u32,
    pub snapshots_applied: u32,
}

/// What the server said about each command, in arrival order.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct CommandResults(pub Vec<CommandResult>);

#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Acked { seq: u32, tick: u32 },
    Rejected { seq: u32, reason: String },
}

/// Handles the messages of one frame. `Welcome` goes first, then `Snapshot`, then `UnitDeltas`;
/// inside one kind the arrival order stays.
pub fn ingest_batch(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    state: &mut ReplicaState,
    mut messages: Vec<Incoming>,
    now: Instant,
) -> Vec<Outgoing> {
    messages.sort_by_key(Incoming::rank); // stable
    messages
        .into_iter()
        .flat_map(|message| ingest_one(hooks, counters, state, message, now))
        .collect()
}

/// Handles one message and returns the messages to send.
pub fn ingest_one(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    state: &mut ReplicaState,
    message: Incoming,
    now: Instant,
) -> Vec<Outgoing> {
    match message {
        Incoming::Welcome(welcome) => {
            on_welcome(hooks, state, &welcome);
            Vec::new()
        }
        Incoming::Snapshot(snapshot) => on_snapshot(hooks, counters, state, snapshot, now),
        Incoming::Deltas(deltas) => on_deltas(hooks, counters, state, deltas, now),
    }
}

/// `Welcome` creates the replica, which starts in resync, and fixes the drop ticks.
/// A second `Welcome` is ignored.
fn on_welcome(hooks: &mut IngestHooks, state: &mut ReplicaState, welcome: &Welcome) {
    if state.welcome.is_some() {
        return;
    }
    state.replica = Some(Replica::new(welcome.tick_hz));
    state.welcome = Some(WelcomeInfo {
        slot: welcome.slot_id.0,
        tick: welcome.tick,
        tick_hz: welcome.tick_hz,
    });
    hooks.drop_ticks = hooks
        .drop_offsets
        .iter()
        .map(|offset| welcome.tick + offset)
        .collect();
}

fn on_snapshot(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    state: &mut ReplicaState,
    snapshot: Snapshot,
    now: Instant,
) -> Vec<Outgoing> {
    let Some(replica) = state.replica.as_mut() else {
        return Vec::new();
    };
    // Only an answer to a request is held. The `Snapshot` after `Welcome` applies at once.
    if hooks.outstanding_request && hooks.hold_resync_snapshots > 0 {
        let release_tick = snapshot.tick + hooks.hold_resync_snapshots;
        hooks.held.push((snapshot, release_tick));
        return release_held(hooks, counters, replica, now);
    }
    apply_snapshot(hooks, counters, replica, &snapshot, now)
}

fn on_deltas(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    state: &mut ReplicaState,
    deltas: UnitDeltas,
    now: Instant,
) -> Vec<Outgoing> {
    counters.deltas_received += 1;
    hooks.highest_delta_tick = Some(
        hooks
            .highest_delta_tick
            .map_or(deltas.tick, |high| high.max(deltas.tick)),
    );
    let Some(replica) = state.replica.as_mut() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if should_drop(hooks, &deltas) {
        counters.deltas_dropped.push(deltas.tick);
    } else if replica.apply_delta(&deltas, now) == DeltaOutcome::ResyncRequested {
        out.extend(request_resync(hooks, counters));
    }
    out.extend(release_held(hooks, counters, replica, now));
    out.extend(idle_resync(hooks, counters, replica));
    out
}

/// True when a test hook says to ignore this delta. The arrival hook fires once.
fn should_drop(hooks: &mut IngestHooks, deltas: &UnitDeltas) -> bool {
    let is_arrival = hooks.drop_arrival_delta
        && hooks.arrival_target.is_some_and(|target| {
            deltas.changes.iter().any(|change| {
                hooks
                    .arrival_unit
                    .is_none_or(|unit| change.unit_id.0 == unit)
                    && change.pos == target
            })
        });
    if is_arrival {
        hooks.drop_arrival_delta = false;
    }
    is_arrival || hooks.drop_ticks.contains(&deltas.tick)
}

/// Applies a `Snapshot` and sends the next request if the replay breaks the chain again.
fn apply_snapshot(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    replica: &mut Replica,
    snapshot: &Snapshot,
    now: Instant,
) -> Vec<Outgoing> {
    let outcome = replica.apply_snapshot(snapshot, now);
    counters.snapshots_applied += 1;
    hooks.outstanding_request = false;
    match outcome {
        SnapshotOutcome::Applied { .. } => Vec::new(),
        SnapshotOutcome::ResyncRequested { .. } => {
            request_resync(hooks, counters).into_iter().collect()
        }
    }
}

/// Applies the held answers whose release tick the deltas have reached.
fn release_held(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    replica: &mut Replica,
    now: Instant,
) -> Vec<Outgoing> {
    let Some(high) = hooks.highest_delta_tick else {
        return Vec::new();
    };
    let (ready, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut hooks.held)
        .into_iter()
        .partition(|(_, release_tick)| *release_tick <= high);
    hooks.held = waiting;
    ready
        .into_iter()
        .flat_map(|(snapshot, _)| apply_snapshot(hooks, counters, replica, &snapshot, now))
        .collect()
}

/// Idle resync: the replica waits for a `Snapshot`, no request is outstanding, and the buffered
/// deltas span [`IDLE_RESYNC_SPAN_TICKS`] ticks or more.
fn idle_resync(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    replica: &Replica,
) -> Vec<Outgoing> {
    if hooks.outstanding_request {
        return Vec::new();
    }
    let Some(buffer) = replica.resync.as_ref() else {
        return Vec::new();
    };
    let lowest = buffer.iter().map(|delta| delta.tick).min();
    let highest = buffer.iter().map(|delta| delta.tick).max();
    match (lowest, highest) {
        (Some(low), Some(high)) if high - low >= IDLE_RESYNC_SPAN_TICKS => {
            request_resync(hooks, counters).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

/// One `ResyncRequest`, unless one is outstanding already.
fn request_resync(hooks: &mut IngestHooks, counters: &mut IngestCounters) -> Option<Outgoing> {
    if hooks.outstanding_request {
        return None;
    }
    hooks.outstanding_request = true;
    counters.resync_requests_sent += 1;
    Some(Outgoing::ResyncRequest)
}

/// Reads the messages of this frame and feeds the replica. `Welcome` is read first, in the same
/// run as the `Snapshot` (G2b): they arrive in one frame.
#[allow(clippy::too_many_arguments)]
pub fn ingest_messages(
    mut welcomes: Query<&mut MessageReceiver<Welcome>>,
    mut snapshots: Query<&mut MessageReceiver<Snapshot>>,
    mut deltas: Query<&mut MessageReceiver<UnitDeltas>>,
    mut acks: Query<&mut MessageReceiver<CommandAck>>,
    mut rejections: Query<&mut MessageReceiver<CommandRejected>>,
    mut requests: Query<&mut MessageSender<ResyncRequest>>,
    mut hooks: ResMut<IngestHooks>,
    mut counters: ResMut<IngestCounters>,
    mut state: ResMut<ReplicaState>,
    mut results: ResMut<CommandResults>,
) {
    let mut messages = Vec::new();
    for mut receiver in welcomes.iter_mut() {
        messages.extend(receiver.receive().map(Incoming::Welcome));
    }
    for mut receiver in snapshots.iter_mut() {
        messages.extend(receiver.receive().map(Incoming::Snapshot));
    }
    for mut receiver in deltas.iter_mut() {
        messages.extend(receiver.receive().map(Incoming::Deltas));
    }
    // Acks before rejections: the server sends the ack at once.
    for mut receiver in acks.iter_mut() {
        results
            .0
            .extend(receiver.receive().map(|ack| CommandResult::Acked {
                seq: ack.seq,
                tick: ack.tick,
            }));
    }
    for mut receiver in rejections.iter_mut() {
        results
            .0
            .extend(receiver.receive().map(|rejected| CommandResult::Rejected {
                seq: rejected.seq,
                reason: rejected.reason,
            }));
    }
    if messages.is_empty() {
        return;
    }
    let outgoing = ingest_batch(
        &mut hooks,
        &mut counters,
        &mut state,
        messages,
        Instant::now(),
    );
    for message in outgoing {
        match message {
            Outgoing::ResyncRequest => {
                for mut sender in requests.iter_mut() {
                    sender.send::<ReliableChannel>(ResyncRequest);
                }
            }
        }
    }
}
