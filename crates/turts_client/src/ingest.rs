//! Message ingest.
//!
//! The ingest system reads `Welcome`, `Snapshot` and `WorldDelta` in `PreUpdate`, after
//! `MessageSystems::Receive`, and feeds them to the `Replica` (design section 5). The logic is
//! in pure functions: [`ingest_one`] handles one message, [`ingest_batch`] orders the messages of
//! one frame. The functions return the messages to send, so a test needs no network.
//!
//! The test hooks (plan R3) live here: dropped deltas, a dropped arrival delta, and held
//! `Snapshot` answers. They make the resync paths testable on the shipped binary.

use bevy::platform::time::Instant;
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::transport::channel::ChannelKind;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::{
    CommandAck, CommandRejectReason, OrderResult, Pos, ResyncRequest, Snapshot, Welcome, WorldDelta,
};

use crate::replica::{
    DeltaOutcome, Replica, ReplicaConfig, ReplicaContext, SnapshotOutcome, replace_keyed,
};
use crate::scenario::{DropWorldChange, parse_drop_world_changes};
use crate::{ClientOpts, content::ClientContent, join::AcquiredPackages};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use turts_net_protocol::{EconomyChanges, EconomyState, NodeId, NodeState};

/// The buffered deltas of a replica that waits for a `Snapshot` and span this many ticks or more
/// make the client ask for one. This covers a lost or late `Welcome` `Snapshot`, so the client
/// never hangs in the initial resync state (G2b, studio call N = 10).
pub const IDLE_RESYNC_SPAN_TICKS: u32 = 10;

/// A message that the server sent and that the replica needs.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Welcome(Welcome),
    Snapshot(Snapshot),
    Deltas(WorldDelta),
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
    pub match_id: u64,
}

/// The client copy of the world. `replica` is `None` until the `Welcome` arrives.
#[derive(Resource, Default)]
pub struct ReplicaState {
    pub replica: Option<Replica>,
    pub welcome: Option<WelcomeInfo>,
    /// Cloned once from accepted packages before the first Welcome is processed.
    pub content: Option<ClientContent>,
    pub config: ReplicaConfig,
}

/// A strictly validated browser-only ingest control. It changes the existing replica hooks;
/// it does not create a second recovery path or issue a player command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserIngestConfig {
    drop_world_changes: Vec<DropWorldChange>,
    pub hold_resync_snapshots: u32,
    pub max_buffered_batches: usize,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BrowserIngestConfigJson {
    drop_world_changes: Vec<String>,
    hold_resync_snapshots: u32,
    max_buffered_batches: usize,
}

impl BrowserIngestConfig {
    /// Parse all three required controls atomically, rejecting unknown or malformed fields.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        let raw: BrowserIngestConfigJson = serde_json::from_str(json)
            .map_err(|error| format!("invalid ingest configuration: {error}"))?;
        if raw.max_buffered_batches == 0 {
            return Err("max_buffered_batches must be a positive integer".into());
        }
        let mut drop_world_changes = Vec::with_capacity(raw.drop_world_changes.len());
        for name in raw.drop_world_changes {
            let mut parsed = parse_drop_world_changes(&name)?;
            if parsed.len() != 1 {
                return Err(format!(
                    "drop_world_changes entries must be single names: `{name}`"
                ));
            }
            let kind = parsed.remove(0);
            if drop_world_changes.contains(&kind) {
                return Err(format!("duplicate WorldDelta change kind `{name}`"));
            }
            drop_world_changes.push(kind);
        }
        Ok(Self {
            drop_world_changes,
            hold_resync_snapshots: raw.hold_resync_snapshots,
            max_buffered_batches: raw.max_buffered_batches,
        })
    }
}

/// Apply browser controls to a joined coherent replica, preserving the world, held response and
/// outstanding request. The caller queues this operation onto Bevy's main thread.
pub fn apply_browser_ingest_config(
    config: BrowserIngestConfig,
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    state: &mut ReplicaState,
    options: &mut crate::ClientOpts,
) -> Result<u32, String> {
    let replica = state
        .replica
        .as_mut()
        .ok_or_else(|| "ingest configuration is waiting for a joined replica".to_string())?;
    let arm_tick = replica
        .last_applied_tick
        .ok_or_else(|| "ingest configuration is waiting for a coherent snapshot".to_string())?;

    let previous_overflows = replica.buffer_overflows;
    replica.set_max_buffered_batches(config.max_buffered_batches);
    counters.buffer_overflows = counters
        .buffer_overflows
        .saturating_add(replica.buffer_overflows.saturating_sub(previous_overflows));
    state.config.max_buffered_batches = config.max_buffered_batches;
    options.0.resync_buffer_batches = config.max_buffered_batches;
    hooks.initial_drop_world_changes.clear();
    hooks.hold_resync_snapshots = config.hold_resync_snapshots;
    hooks.hold_resync_snapshots_remaining = None;
    if hooks.hold_resync_snapshots == 0 {
        let eligible_tick = hooks.highest_delta_tick.unwrap_or(0);
        for (_, release_tick) in &mut hooks.held {
            *release_tick = eligible_tick;
        }
    }
    if config.drop_world_changes.is_empty() {
        hooks.armed_drop_world_changes = None;
    } else {
        arm_drop_world_changes(hooks, config.drop_world_changes, replica.clone(), arm_tick);
    }
    Ok(arm_tick)
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
    /// Remaining resync Snapshot answers allowed to receive the delay; None means unlimited.
    pub hold_resync_snapshots_remaining: Option<u32>,
    /// Held answers: the `Snapshot` and the delta tick at which it is released.
    pub held: Vec<(Snapshot, u32)>,
    /// True from the moment a `ResyncRequest` is sent until its answer is applied.
    pub outstanding_request: bool,
    /// The highest delta tick received, dropped deltas included. It is the clock of the hold.
    pub highest_delta_tick: Option<u32>,
    pub(crate) initial_drop_world_changes: Vec<DropWorldChange>,
    pub(crate) armed_drop_world_changes: Option<ArmedDropWorldChanges>,
    pub(crate) dropped_world_changes: Vec<DroppedWorldChange>,
}

#[derive(Debug, Clone)]
pub(crate) struct ArmedDropWorldChanges {
    pub(crate) requested: BTreeSet<DropWorldChange>,
    pub(crate) baseline_nodes: HashMap<NodeId, NodeState>,
    pub(crate) baseline_economy: EconomyState,
    pub(crate) baseline_tick: u32,
    pub(crate) arm_tick: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct DroppedWorldChange {
    pub(crate) kind: DropWorldChange,
    pub(crate) arm_tick: u32,
    pub(crate) dropped_tick: u32,
}

pub(crate) fn arm_drop_world_changes(
    hooks: &mut IngestHooks,
    kinds: Vec<DropWorldChange>,
    replica: Replica,
    arm_tick: u32,
) {
    hooks.armed_drop_world_changes = Some(ArmedDropWorldChanges {
        requested: kinds.into_iter().collect(),
        baseline_nodes: replica.nodes,
        baseline_economy: replica.economy,
        baseline_tick: replica.last_applied_tick.unwrap_or(arm_tick),
        arm_tick,
    });
}

#[derive(Resource, Default, Debug)]
pub struct IngestCounters {
    pub deltas_received: u32,
    /// The ticks of the ignored deltas.
    pub deltas_dropped: Vec<u32>,
    pub resync_requests_sent: u32,
    pub snapshots_applied: u32,
    pub max_buffered_batches: usize,
    pub max_outstanding_requests: usize,
    pub buffer_overflows: u32,
    pub successor_requests: u32,
}

/// What the server said about each command, in arrival order.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct CommandResults(pub Vec<CommandResult>);

#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Acked {
        seq: u32,
        tick: u32,
    },
    Applied {
        seq: u32,
        tick: u32,
        job_id: Option<turts_net_protocol::JobId>,
    },
    Rejected {
        seq: u32,
        tick: u32,
        reason: CommandRejectReason,
    },
    Order {
        seq: u32,
        tick: u32,
        accepted: Vec<turts_net_protocol::UnitId>,
        refused: Vec<turts_net_protocol::UnitRefusal>,
        envelope: Option<CommandRejectReason>,
        job_id: Option<turts_net_protocol::JobId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptKind {
    Ack,
    Applied,
    Rejected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptWitness {
    pub connection_epoch: u64,
    pub seq: u32,
    pub kind: ReceiptKind,
    pub reliable: bool,
    pub message_id: Option<u16>,
    pub remote_tick: u32,
}
#[derive(Resource, Debug, Default, Clone)]
pub struct ReceiptWitnesses {
    pub records: Vec<ReceiptWitness>,
    pub reliable_counts: BTreeMap<u64, u32>,
    pub invalid_epochs: BTreeSet<u64>,
    pub epochs: BTreeMap<u64, u64>,
    pub next_epoch: u64,
}
impl ReceiptWitnesses {
    pub fn start_epoch(&mut self, entity: u64) -> u64 {
        self.next_epoch = self.next_epoch.saturating_add(1);
        self.epochs.insert(entity, self.next_epoch);
        self.next_epoch
    }
    pub fn epoch_for(&self, entity: u64) -> u64 {
        *self.epochs.get(&entity).unwrap_or(&entity)
    }
    pub fn record_message(&mut self, epoch: u64, reliable: bool) {
        if !reliable {
            return;
        }
        let count = self.reliable_counts.entry(epoch).or_default();
        *count = count.saturating_add(1);
        if *count >= 32768 {
            self.invalid_epochs.insert(epoch);
        }
    }
    fn record_receipt(&mut self, witness: ReceiptWitness) {
        self.records.push(witness);
    }
    pub fn ordered_for(&self, seq: u32, epoch: u64) -> Option<bool> {
        let matching: Vec<_> = self
            .records
            .iter()
            .copied()
            .filter(|w| w.connection_epoch == epoch && w.seq == seq)
            .collect();
        let mut acks = matching.iter().filter(|w| w.kind == ReceiptKind::Ack);
        let mut rejections = matching.iter().filter(|w| w.kind == ReceiptKind::Rejected);
        let ack = *acks.next()?;
        let rejected = *rejections.next()?;
        // Repeated envelopes with the same sequence cannot be paired safely without another
        // wire correlation field. Refuse to claim ordering when either receipt is ambiguous.
        if acks.next().is_some() || rejections.next().is_some() {
            return None;
        }
        let count = self
            .reliable_counts
            .get(&epoch)
            .copied()
            .unwrap_or(u32::MAX);
        Some(!self.invalid_epochs.contains(&epoch) && receipt_ordered(ack, rejected, count))
    }
}
pub fn receipt_ordered(ack: ReceiptWitness, rejected: ReceiptWitness, epoch_count: u32) -> bool {
    if epoch_count >= 32768
        || ack.connection_epoch != rejected.connection_epoch
        || ack.seq != rejected.seq
        || !ack.reliable
        || !rejected.reliable
    {
        return false;
    }
    let (Some(a), Some(r)) = (ack.message_id, rejected.message_id) else {
        return false;
    };
    let distance = r.wrapping_sub(a);
    (1..32768).contains(&distance)
}

/// Handles the messages of one frame. `Welcome` goes first, then `Snapshot`, then `WorldDelta`;
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
    let mut replica = Replica::with_config(welcome.tick_hz, state.config);
    if let Some(content) = state.content.clone() {
        replica.set_context(ReplicaContext::new(content, welcome.slot_id.0));
    }
    state.replica = Some(replica);
    state.welcome = Some(WelcomeInfo {
        slot: welcome.slot_id.0,
        tick: welcome.tick,
        tick_hz: welcome.tick_hz,
        match_id: welcome.match_id,
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
    if hooks.outstanding_request
        && hooks.hold_resync_snapshots > 0
        && hooks.hold_resync_snapshots_remaining != Some(0)
    {
        if hooks.held.is_empty() {
            let release_tick = snapshot.tick + hooks.hold_resync_snapshots;
            hooks.held.push((snapshot, release_tick));
            if let Some(remaining) = &mut hooks.hold_resync_snapshots_remaining {
                *remaining = remaining.saturating_sub(1);
            }
        }
        return release_held(hooks, counters, replica, now);
    }
    apply_snapshot(hooks, counters, replica, &snapshot, now)
}

fn on_deltas(
    hooks: &mut IngestHooks,
    counters: &mut IngestCounters,
    state: &mut ReplicaState,
    deltas: WorldDelta,
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
    let ordinary_drop = should_drop(hooks, &deltas);
    let matched_loss = !ordinary_drop && should_drop_world_change(hooks, replica, &deltas);
    if ordinary_drop || matched_loss {
        counters.deltas_dropped.push(deltas.tick);
    } else {
        let before_overflows = replica.buffer_overflows;
        if replica.apply_delta(&deltas, now) == DeltaOutcome::ResyncRequested {
            out.extend(request_resync(hooks, counters));
        }
        counters.max_buffered_batches = counters
            .max_buffered_batches
            .max(replica.buffered_batches());
        counters.buffer_overflows = counters
            .buffer_overflows
            .saturating_add(replica.buffer_overflows.saturating_sub(before_overflows));
    }
    out.extend(release_held(hooks, counters, replica, now));
    out.extend(idle_resync(hooks, counters, replica));
    out
}

fn should_drop_world_change(
    hooks: &mut IngestHooks,
    replica: &Replica,
    deltas: &WorldDelta,
) -> bool {
    let Some(mut armed) = hooks.armed_drop_world_changes.take() else {
        return false;
    };
    if deltas.base_tick != armed.baseline_tick
        || armed.baseline_tick.checked_add(1) != Some(deltas.tick)
        || replica.last_applied_tick != Some(armed.baseline_tick)
    {
        hooks.armed_drop_world_changes = Some(armed);
        return false;
    }
    let matches = matching_world_changes(&armed, deltas);
    let fired: Vec<_> = armed
        .requested
        .iter()
        .filter(|kind| matches.contains(kind))
        .copied()
        .collect();
    if !fired.is_empty() {
        for kind in fired {
            armed.requested.remove(&kind);
            hooks.dropped_world_changes.push(DroppedWorldChange {
                kind,
                arm_tick: armed.arm_tick,
                dropped_tick: deltas.tick,
            });
        }
        hooks.armed_drop_world_changes = (!armed.requested.is_empty()).then_some(armed);
        return true;
    }
    for node in &deltas.nodes {
        armed.baseline_nodes.insert(node.id, node.clone());
    }
    if !merge_economy_baseline(&mut armed.baseline_economy, &deltas.economy) {
        hooks.armed_drop_world_changes = Some(armed);
        return false;
    }
    armed.baseline_tick = deltas.tick;
    hooks.armed_drop_world_changes = Some(armed);
    false
}

fn merge_economy_baseline(state: &mut EconomyState, changes: &EconomyChanges) -> bool {
    let mut staged = state.clone();
    if !replace_keyed(&mut staged.balances, &changes.balances, |record| {
        record.resource.clone()
    }) || !replace_keyed(&mut staged.inventories, &changes.inventories, |record| {
        (
            record.unit.0,
            match record.port {
                turts_net_protocol::InventoryPort::Main => 0u8,
                turts_net_protocol::InventoryPort::Output => 1u8,
            },
        )
    }) || !replace_keyed(&mut staged.orders, &changes.orders, |record| record.unit.0)
        || !replace_keyed(&mut staged.queues, &changes.queues, |record| {
            record.producer.0
        })
    {
        return false;
    }
    *state = staged;
    true
}

fn matching_world_changes(
    armed: &ArmedDropWorldChanges,
    delta: &WorldDelta,
) -> BTreeSet<DropWorldChange> {
    let mut found = BTreeSet::new();
    let changes = &delta.economy;
    if !changes.balances.is_empty()
        || !changes.inventories.is_empty()
        || !changes.orders.is_empty()
        || !changes.queues.is_empty()
    {
        found.insert(DropWorldChange::FinalChange);
    }
    if !delta.spawned.is_empty() {
        found.insert(DropWorldChange::Spawn);
    }

    let increased = economy_increased(&armed.baseline_economy, changes);
    let decreased_inventory = changes.inventories.iter().any(|inventory| {
        let old = armed
            .baseline_economy
            .inventories
            .iter()
            .find(|item| item.unit == inventory.unit && item.port == inventory.port);
        inventory.items.iter().any(|item| {
            old.and_then(|old| {
                old.items
                    .iter()
                    .find(|previous| previous.resource == item.resource)
            })
            .is_some_and(|previous| item.on_hand < previous.on_hand)
        })
    });
    let decreased_balance = changes.balances.iter().any(|balance| {
        armed
            .baseline_economy
            .balances
            .iter()
            .find(|old| old.resource == balance.resource)
            .is_some_and(|old| balance.on_hand < old.on_hand)
    });
    let increased_balance = changes.balances.iter().any(|balance| {
        armed
            .baseline_economy
            .balances
            .iter()
            .find(|old| old.resource == balance.resource)
            .is_some_and(|old| balance.on_hand > old.on_hand)
    });
    if decreased_inventory && increased_balance {
        found.insert(DropWorldChange::Conversion);
    }
    if has_charged_line_transition(&armed.baseline_economy, changes)
        && (decreased_inventory || decreased_balance)
    {
        found.insert(DropWorldChange::Charge);
    }

    let extracted = delta.nodes.iter().any(|node| {
        armed
            .baseline_nodes
            .get(&node.id)
            .is_some_and(|old| node.remaining < old.remaining)
    }) && changes.inventories.iter().any(|inventory| {
        let old = armed
            .baseline_economy
            .inventories
            .iter()
            .find(|item| item.unit == inventory.unit && item.port == inventory.port);
        inventory.items.iter().any(|item| {
            old.and_then(|old| {
                old.items
                    .iter()
                    .find(|previous| previous.resource == item.resource)
            })
            .is_some_and(|previous| item.on_hand > previous.on_hand)
        })
    });
    if extracted {
        found.insert(DropWorldChange::Extraction);
    }

    let refunded = changes.queues.iter().any(|queue| {
        let old = armed
            .baseline_economy
            .queues
            .iter()
            .find(|item| item.producer == queue.producer);
        old.is_some_and(|old| queue.jobs.len() < old.jobs.len())
    }) && increased;
    if refunded {
        found.insert(DropWorldChange::Refund);
    }
    found
}

fn has_charged_line_transition(previous: &EconomyState, changes: &EconomyChanges) -> bool {
    use turts_mod_api::CostSource;

    changes.queues.iter().any(|queue| {
        let old_queue = previous
            .queues
            .iter()
            .find(|old| old.producer == queue.producer);
        queue.jobs.iter().any(|job| {
            let old_job =
                old_queue.and_then(|old| old.jobs.iter().find(|candidate| candidate.id == job.id));
            job.charged.iter().any(|line| {
                let already_charged = old_job
                    .and_then(|old| {
                        old.charged.iter().find(|old_line| {
                            old_line.source == line.source && old_line.resource == line.resource
                        })
                    })
                    .map_or(0, |old_line| old_line.amount);
                let newly_charged = line.amount.saturating_sub(already_charged);
                if newly_charged == 0 {
                    return false;
                }
                match line.source {
                    CostSource::SlotBalance => {
                        let old = previous
                            .balances
                            .iter()
                            .find(|account| account.resource == line.resource);
                        let new = changes
                            .balances
                            .iter()
                            .find(|account| account.resource == line.resource);
                        old.zip(new).is_some_and(|(old, new)| {
                            old.on_hand.saturating_sub(new.on_hand) >= newly_charged
                        })
                    }
                    CostSource::ProducerInventory => {
                        let old_inventory = previous.inventories.iter().find(|inventory| {
                            inventory.unit.0 == queue.producer.0
                                && inventory.port == turts_net_protocol::InventoryPort::Main
                        });
                        let new_inventory = changes.inventories.iter().find(|inventory| {
                            inventory.unit.0 == queue.producer.0
                                && inventory.port == turts_net_protocol::InventoryPort::Main
                        });
                        let old = old_inventory.and_then(|inventory| {
                            inventory
                                .items
                                .iter()
                                .find(|item| item.resource == line.resource)
                        });
                        let new = new_inventory.and_then(|inventory| {
                            inventory
                                .items
                                .iter()
                                .find(|item| item.resource == line.resource)
                        });
                        old.zip(new).is_some_and(|(old, new)| {
                            old.on_hand.saturating_sub(new.on_hand) >= newly_charged
                        })
                    }
                }
            })
        })
    })
}

fn economy_increased(previous: &EconomyState, changes: &EconomyChanges) -> bool {
    let balance_increase = changes.balances.iter().any(|balance| {
        previous
            .balances
            .iter()
            .find(|old| old.resource == balance.resource)
            .is_some_and(|old| balance.on_hand > old.on_hand)
    });
    let inventory_increase = changes.inventories.iter().any(|inventory| {
        let old = previous
            .inventories
            .iter()
            .find(|item| item.unit == inventory.unit && item.port == inventory.port);
        inventory.items.iter().any(|item| {
            old.and_then(|old| {
                old.items
                    .iter()
                    .find(|previous| previous.resource == item.resource)
            })
            .is_some_and(|previous| item.on_hand > previous.on_hand)
        })
    });
    balance_increase || inventory_increase
}

/// True when a test hook says to ignore this delta. The arrival hook fires once.
fn should_drop(hooks: &mut IngestHooks, deltas: &WorldDelta) -> bool {
    let is_arrival = hooks.drop_arrival_delta
        && hooks.arrival_target.is_some_and(|target| {
            deltas.positions.iter().any(|change| {
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
    let was_outstanding = hooks.outstanding_request;
    hooks.outstanding_request = false;
    let outcome = replica.apply_snapshot(snapshot, now);
    counters.max_buffered_batches = counters
        .max_buffered_batches
        .max(replica.buffered_batches());
    match outcome {
        SnapshotOutcome::Applied { .. } => {
            counters.snapshots_applied += 1;
            if !hooks.initial_drop_world_changes.is_empty()
                && hooks.armed_drop_world_changes.is_none()
            {
                let initial_kinds = std::mem::take(&mut hooks.initial_drop_world_changes);
                arm_drop_world_changes(
                    hooks,
                    initial_kinds,
                    replica.clone(),
                    replica.last_applied_tick.unwrap_or(snapshot.tick),
                );
            } else if hooks.armed_drop_world_changes.is_some() {
                // A reliable coherent answer closes the gap caused by a dropped batch. Preserve
                // unspent predicates and rebase them; stale pre-gap ticks can never match again.
                rebase_armed_world_changes(hooks, replica, snapshot.tick);
            }
            Vec::new()
        }
        SnapshotOutcome::IgnoredStale => {
            if replica.in_resync() {
                if was_outstanding {
                    counters.successor_requests = counters.successor_requests.saturating_add(1);
                }
                request_resync(hooks, counters).into_iter().collect()
            } else {
                Vec::new()
            }
        }
        SnapshotOutcome::ResyncRequested { .. } => {
            if was_outstanding {
                counters.successor_requests = counters.successor_requests.saturating_add(1);
            }
            request_resync(hooks, counters).into_iter().collect()
        }
        SnapshotOutcome::Invalid => {
            if was_outstanding {
                counters.successor_requests = counters.successor_requests.saturating_add(1);
            }
            request_resync(hooks, counters).into_iter().collect()
        }
    }
}

fn rebase_armed_world_changes(hooks: &mut IngestHooks, replica: &Replica, fallback_tick: u32) {
    if let Some(armed) = hooks.armed_drop_world_changes.as_mut() {
        armed.baseline_nodes.clone_from(&replica.nodes);
        armed.baseline_economy.clone_from(&replica.economy);
        armed.baseline_tick = replica.last_applied_tick.unwrap_or(fallback_tick);
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
    if replica
        .resync_span_ticks()
        .is_some_and(|span| span >= IDLE_RESYNC_SPAN_TICKS)
    {
        request_resync(hooks, counters).into_iter().collect()
    } else {
        Vec::new()
    }
}

/// One `ResyncRequest`, unless one is outstanding already.
fn request_resync(hooks: &mut IngestHooks, counters: &mut IngestCounters) -> Option<Outgoing> {
    if hooks.outstanding_request {
        return None;
    }
    hooks.outstanding_request = true;
    counters.resync_requests_sent = counters.resync_requests_sent.saturating_add(1);
    counters.max_outstanding_requests = counters.max_outstanding_requests.max(1);
    Some(Outgoing::ResyncRequest)
}

/// Reads the messages of this frame and feeds the replica. `Welcome` is read first, in the same
/// run as the `Snapshot` (G2b): they arrive in one frame.
#[allow(clippy::too_many_arguments)]
pub fn ingest_messages(
    mut welcomes: Query<(Entity, &mut MessageReceiver<Welcome>)>,
    mut snapshots: Query<(Entity, &mut MessageReceiver<Snapshot>)>,
    mut deltas: Query<&mut MessageReceiver<WorldDelta>>,
    mut acks: Query<(Entity, &mut MessageReceiver<CommandAck>)>,
    mut order_results: Query<(Entity, &mut MessageReceiver<OrderResult>)>,
    mut requests: Query<(Entity, &mut MessageSender<ResyncRequest>)>,
    mut hooks: ResMut<IngestHooks>,
    mut counters: ResMut<IngestCounters>,
    mut state: ResMut<ReplicaState>,
    mut results: ResMut<CommandResults>,
    mut witnesses: ResMut<ReceiptWitnesses>,
    acquired: Option<Res<AcquiredPackages>>,
    options: Res<ClientOpts>,
) {
    state.config.max_buffered_batches = options.0.resync_buffer_batches.max(1);
    if state.content.is_none()
        && state.replica.is_none()
        && let Some(acquired) = acquired
    {
        state.content = Some(acquired.0.content.clone());
    }
    let mut messages = Vec::new();
    for (entity, mut receiver) in &mut welcomes {
        let epoch = witnesses.epoch_for(entity.to_bits());
        for received in receiver.receive_with_tick() {
            witnesses.record_message(
                epoch,
                received.channel_kind == ChannelKind::of::<ReliableChannel>(),
            );
            messages.push(Incoming::Welcome(received.data));
        }
    }
    for (entity, mut receiver) in &mut snapshots {
        let epoch = witnesses.epoch_for(entity.to_bits());
        for received in receiver.receive_with_tick() {
            append_raw_world_dto(
                options.0.raw_world_log.as_deref(),
                "snapshot",
                &received.data,
            );
            witnesses.record_message(
                epoch,
                received.channel_kind == ChannelKind::of::<ReliableChannel>(),
            );
            messages.push(Incoming::Snapshot(received.data));
        }
    }
    for mut receiver in deltas.iter_mut() {
        for delta in receiver.receive() {
            append_raw_world_dto(options.0.raw_world_log.as_deref(), "world_delta", &delta);
            messages.push(Incoming::Deltas(delta));
        }
    }
    // Acks before rejections: the server sends the ack at once.
    for (entity, mut receiver) in &mut acks {
        let epoch = witnesses.epoch_for(entity.to_bits());
        for received in receiver.receive_with_tick() {
            witnesses.record_message(
                epoch,
                received.channel_kind == ChannelKind::of::<ReliableChannel>(),
            );
            witnesses.record_receipt(ReceiptWitness {
                connection_epoch: epoch,
                seq: received.data.seq,
                kind: ReceiptKind::Ack,
                reliable: received.channel_kind == ChannelKind::of::<ReliableChannel>(),
                message_id: received.message_id.map(|id| id.0),
                remote_tick: received.remote_tick.0,
            });
            results.0.push(CommandResult::Acked {
                seq: received.data.seq,
                tick: received.data.tick,
            });
        }
    }
    for (entity, mut receiver) in &mut order_results {
        let epoch = witnesses.epoch_for(entity.to_bits());
        for received in receiver.receive_with_tick() {
            witnesses.record_message(
                epoch,
                received.channel_kind == ChannelKind::of::<ReliableChannel>(),
            );
            let kind = if received.data.accepted.is_empty() {
                ReceiptKind::Rejected
            } else {
                ReceiptKind::Applied
            };
            witnesses.record_receipt(ReceiptWitness {
                connection_epoch: epoch,
                seq: received.data.seq,
                kind,
                reliable: received.channel_kind == ChannelKind::of::<ReliableChannel>(),
                message_id: received.message_id.map(|id| id.0),
                remote_tick: received.remote_tick.0,
            });
            results.0.push(CommandResult::Order {
                seq: received.data.seq,
                tick: received.data.tick,
                accepted: received.data.accepted,
                refused: received.data.refused,
                envelope: received.data.envelope,
                job_id: received.data.job_id,
            });
        }
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
                for (entity, mut sender) in &mut requests {
                    sender.send::<ReliableChannel>(ResyncRequest);
                    let epoch = witnesses.epoch_for(entity.to_bits());
                    witnesses.record_message(epoch, true);
                }
            }
        }
    }
}

/// Append the decoded wire DTO before any replica validation, drop, hold, or owner projection.
/// This opt-in native test hook is deliberately separate from normal debug/report state.
fn append_raw_world_dto<T: serde::Serialize>(path: Option<&std::path::Path>, kind: &str, dto: &T) {
    let Some(path) = path else { return };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|error| panic!("open raw world DTO log {}: {error}", path.display()));
    serde_json::to_writer(&mut file, &serde_json::json!({ "kind": kind, "dto": dto }))
        .unwrap_or_else(|error| panic!("write raw world DTO to {}: {error}", path.display()));
    file.write_all(b"\n").unwrap_or_else(|error| {
        panic!("terminate raw world DTO record {}: {error}", path.display())
    });
}

#[cfg(test)]
mod world_change_tests {
    use super::*;
    use crate::scenario::DropWorldChange;
    use turts_mod_api::{Cost, CostSource};
    use turts_net_protocol::{
        BalanceState, EconomyChanges, InventoryState, JobId, JobStage, JobState, QueueState, UnitId,
    };

    fn balance(resource: &str, on_hand: u64) -> BalanceState {
        BalanceState {
            resource: resource.to_owned(),
            on_hand,
            reserved: 0,
        }
    }

    fn inventory(unit: u32, ore: u64) -> InventoryState {
        InventoryState {
            unit: UnitId(unit),
            items: vec![balance("base:ore", ore)],
            port: turts_net_protocol::InventoryPort::Main,
        }
    }

    fn delta(tick: u32, economy: EconomyChanges) -> WorldDelta {
        WorldDelta {
            tick,
            base_tick: tick - 1,
            spawned: Vec::new(),
            removed: Vec::new(),
            positions: Vec::new(),
            nodes: Vec::new(),
            economy,
            belts: vec![],
            inserters: vec![],
            extractors: vec![],
            weapons: vec![],
            healths: vec![],
            research: vec![],
            unit_orders: Vec::new(),
        }
    }

    fn make_replica(tick: u32, economy: EconomyState) -> Replica {
        let mut replica = Replica::new(20);
        replica.resync = None;
        replica.last_applied_tick = Some(tick);
        replica.economy = economy;
        replica
    }

    fn charge_economy() -> EconomyState {
        EconomyState {
            balances: vec![balance("base:credits", 400)],
            inventories: vec![inventory(2, 5), inventory(4, 5)],
            orders: Vec::new(),
            queues: vec![QueueState {
                producer: UnitId(1),
                jobs: Vec::new(),
                enabled: true,
            }],
        }
    }

    #[test]
    fn store_and_conversion_do_not_match_charge_but_real_queue_debit_does() {
        let baseline = EconomyState {
            balances: vec![balance("base:credits", 400)],
            inventories: vec![inventory(2, 10), inventory(4, 0)],
            orders: Vec::new(),
            queues: vec![QueueState {
                producer: UnitId(1),
                jobs: Vec::new(),
                enabled: true,
            }],
        };
        let store = delta(
            11,
            EconomyChanges {
                balances: Vec::new(),
                inventories: vec![inventory(2, 5), inventory(4, 5)],
                orders: Vec::new(),
                queues: Vec::new(),
            },
        );
        let store_matches = matching_world_changes(
            &ArmedDropWorldChanges {
                requested: BTreeSet::new(),
                baseline_nodes: HashMap::new(),
                baseline_economy: baseline.clone(),
                baseline_tick: 10,
                arm_tick: 10,
            },
            &store,
        );
        assert!(!store_matches.contains(&DropWorldChange::Conversion));
        assert!(!store_matches.contains(&DropWorldChange::Charge));

        let conversion = delta(
            11,
            EconomyChanges {
                balances: vec![balance("base:credits", 410)],
                inventories: vec![inventory(2, 5)],
                orders: Vec::new(),
                queues: Vec::new(),
            },
        );
        let conversion_matches = matching_world_changes(
            &ArmedDropWorldChanges {
                requested: BTreeSet::new(),
                baseline_nodes: HashMap::new(),
                baseline_economy: baseline.clone(),
                baseline_tick: 10,
                arm_tick: 10,
            },
            &conversion,
        );
        assert!(conversion_matches.contains(&DropWorldChange::Conversion));
        assert!(!conversion_matches.contains(&DropWorldChange::Charge));

        let charged_job = JobState {
            id: JobId(7),
            recipe: "base:train_rifleman".to_owned(),
            stage: JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks: 100,
            charged: vec![Cost {
                resource: "base:credits".to_owned(),
                amount: 50,
                source: CostSource::SlotBalance,
            }],
            reserved: Vec::new(),
        };
        let charge = delta(
            11,
            EconomyChanges {
                balances: vec![balance("base:credits", 350)],
                inventories: Vec::new(),
                orders: Vec::new(),
                queues: vec![QueueState {
                    producer: UnitId(1),
                    jobs: vec![charged_job],
                    enabled: true,
                }],
            },
        );
        let charge_matches = matching_world_changes(
            &ArmedDropWorldChanges {
                requested: BTreeSet::new(),
                baseline_nodes: HashMap::new(),
                baseline_economy: baseline,
                baseline_tick: 10,
                arm_tick: 10,
            },
            &charge,
        );
        assert!(charge_matches.contains(&DropWorldChange::Charge));
        assert!(!charge_matches.contains(&DropWorldChange::Conversion));
    }

    #[test]
    fn multiple_kinds_survive_recovery_and_midtrace_rearm_resets_the_baseline() {
        let initial = EconomyState {
            balances: vec![balance("base:credits", 400)],
            inventories: vec![inventory(2, 10), inventory(4, 0)],
            orders: Vec::new(),
            queues: vec![QueueState {
                producer: UnitId(1),
                jobs: Vec::new(),
                enabled: true,
            }],
        };
        let mut replica = make_replica(10, initial.clone());
        let mut hooks = IngestHooks::default();
        arm_drop_world_changes(
            &mut hooks,
            vec![DropWorldChange::Conversion, DropWorldChange::Charge],
            replica.clone(),
            10,
        );

        let store = delta(
            11,
            EconomyChanges {
                balances: Vec::new(),
                inventories: vec![inventory(2, 5), inventory(4, 5)],
                orders: Vec::new(),
                queues: Vec::new(),
            },
        );
        assert!(!should_drop_world_change(&mut hooks, &replica, &store));
        replica.economy.inventories = vec![inventory(2, 5), inventory(4, 5)];
        replica.last_applied_tick = Some(11);

        let conversion = delta(
            12,
            EconomyChanges {
                balances: vec![balance("base:credits", 410)],
                inventories: vec![inventory(2, 0)],
                orders: Vec::new(),
                queues: Vec::new(),
            },
        );
        assert!(should_drop_world_change(&mut hooks, &replica, &conversion));
        assert_eq!(hooks.dropped_world_changes.len(), 1);
        assert_eq!(
            hooks.dropped_world_changes[0].kind,
            DropWorldChange::Conversion
        );
        assert_eq!(hooks.dropped_world_changes[0].arm_tick, 10);
        assert_eq!(
            hooks.armed_drop_world_changes.as_ref().unwrap().requested,
            BTreeSet::from([DropWorldChange::Charge])
        );

        // A coherent response is the only baseline allowed to bridge the intentionally dropped
        // batch. Remaining predicates use this snapshot tick and state.
        replica = make_replica(20, charge_economy());
        rebase_armed_world_changes(&mut hooks, &replica, 20);
        let charged_job = JobState {
            id: JobId(8),
            recipe: "base:train_rifleman".to_owned(),
            stage: JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks: 100,
            charged: vec![Cost {
                resource: "base:credits".to_owned(),
                amount: 50,
                source: CostSource::SlotBalance,
            }],
            reserved: Vec::new(),
        };
        let charge = delta(
            21,
            EconomyChanges {
                balances: vec![balance("base:credits", 350)],
                inventories: Vec::new(),
                orders: Vec::new(),
                queues: vec![QueueState {
                    producer: UnitId(1),
                    jobs: vec![charged_job],
                    enabled: true,
                }],
            },
        );
        assert!(should_drop_world_change(&mut hooks, &replica, &charge));
        assert_eq!(hooks.dropped_world_changes.len(), 2);
        assert_eq!(hooks.dropped_world_changes[1].kind, DropWorldChange::Charge);
        assert_eq!(hooks.dropped_world_changes[1].arm_tick, 10);
        assert!(hooks.armed_drop_world_changes.is_none());

        // A later trace arm clears matching baselines/fired flags but keeps historical witnesses.
        arm_drop_world_changes(
            &mut hooks,
            vec![DropWorldChange::Conversion],
            replica.clone(),
            20,
        );
        let rearmed = delta(
            21,
            EconomyChanges {
                balances: vec![balance("base:credits", 410)],
                inventories: vec![inventory(2, 0)],
                orders: Vec::new(),
                queues: Vec::new(),
            },
        );
        assert!(should_drop_world_change(&mut hooks, &replica, &rearmed));
        assert_eq!(hooks.dropped_world_changes.len(), 3);
        assert_eq!(hooks.dropped_world_changes[2].arm_tick, 20);
        assert_eq!(hooks.dropped_world_changes[2].dropped_tick, 21);
    }
}

#[cfg(test)]
mod browser_config_tests {
    use super::*;
    use crate::options::ClientOptions;

    #[test]
    fn browser_config_requires_exact_valid_fields_and_unique_single_names() {
        let valid = r#"{"drop_world_changes":["extraction","refund"],"hold_resync_snapshots":3,"max_buffered_batches":4}"#;
        let parsed = BrowserIngestConfig::parse_json(valid).unwrap();
        assert_eq!(parsed.drop_world_changes.len(), 2);
        assert_eq!(parsed.hold_resync_snapshots, 3);
        assert_eq!(parsed.max_buffered_batches, 4);
        for invalid in [
            r#"{"drop_world_changes":[],"hold_resync_snapshots":0,"max_buffered_batches":1,"extra":true}"#,
            r#"{"drop_world_changes":[],"hold_resync_snapshots":0,"max_buffered_batches":0}"#,
            r#"{"drop_world_changes":["extraction","extraction"],"hold_resync_snapshots":0,"max_buffered_batches":1}"#,
            r#"{"drop_world_changes":["extraction,refund"],"hold_resync_snapshots":0,"max_buffered_batches":1}"#,
            r#"{"drop_world_changes":[],"hold_resync_snapshots":-1,"max_buffered_batches":1}"#,
            r#"{"drop_world_changes":[],"hold_resync_snapshots":0}"#,
        ] {
            assert!(
                BrowserIngestConfig::parse_json(invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn browser_config_updates_live_replica_and_releases_existing_hold_normally() {
        let mut replica = Replica::new(20);
        replica.last_applied_tick = Some(10);
        replica.resync = Some(Vec::new());
        let mut state = ReplicaState {
            replica: Some(replica),
            ..Default::default()
        };
        let mut hooks = IngestHooks {
            outstanding_request: true,
            highest_delta_tick: Some(11),
            held: vec![(
                Snapshot {
                    tick: 12,
                    units: Vec::new(),
                    nodes: Vec::new(),
                    economy: EconomyState {
                        balances: Vec::new(),
                        inventories: Vec::new(),
                        orders: Vec::new(),
                        queues: Vec::new(),
                    },
                    belts: vec![],
                    inserters: vec![],
                    extractors: vec![],
                    weapons: vec![],
                    healths: vec![],
                    research: vec![],
                    unit_orders: Vec::new(),
                },
                99,
            )],
            ..Default::default()
        };
        let mut options = crate::ClientOpts(ClientOptions::default());
        let mut counters = IngestCounters::default();
        let config = BrowserIngestConfig::parse_json(
            r#"{"drop_world_changes":["charge"],"hold_resync_snapshots":0,"max_buffered_batches":3}"#,
        ).unwrap();
        assert_eq!(
            apply_browser_ingest_config(
                config,
                &mut hooks,
                &mut counters,
                &mut state,
                &mut options
            ),
            Ok(10)
        );
        let replica = state.replica.as_ref().unwrap();
        assert_eq!(replica.last_applied_tick, Some(10));
        assert_eq!(replica.max_buffered_batches(), 3);
        assert_eq!(state.config.max_buffered_batches, 3);
        assert_eq!(options.0.resync_buffer_batches, 3);
        assert_eq!(counters.buffer_overflows, 0);
        assert!(hooks.outstanding_request);
        assert_eq!(hooks.held.len(), 1);
        assert_eq!(hooks.held[0].1, 11);
        assert_eq!(hooks.hold_resync_snapshots, 0);
        assert!(hooks.armed_drop_world_changes.is_some());
    }

    #[test]
    fn shrinking_live_buffer_bound_reports_the_existing_overflow_once() {
        let mut replica = Replica::new(20);
        replica.last_applied_tick = Some(10);
        replica.highest_observed_tick = Some(13);
        replica.resync = Some(vec![
            WorldDelta {
                tick: 11,
                base_tick: 10,
                spawned: vec![],
                removed: vec![],
                positions: vec![],
                nodes: vec![],
                economy: EconomyChanges {
                    balances: vec![],
                    inventories: vec![],
                    orders: vec![],
                    queues: vec![],
                },
                belts: vec![],
                inserters: vec![],
                extractors: vec![],
                weapons: vec![],
                healths: vec![],
                research: vec![],
                unit_orders: Vec::new(),
            },
            WorldDelta {
                tick: 12,
                base_tick: 11,
                spawned: vec![],
                removed: vec![],
                positions: vec![],
                nodes: vec![],
                economy: EconomyChanges {
                    balances: vec![],
                    inventories: vec![],
                    orders: vec![],
                    queues: vec![],
                },
                belts: vec![],
                inserters: vec![],
                extractors: vec![],
                weapons: vec![],
                healths: vec![],
                research: vec![],
                unit_orders: Vec::new(),
            },
            WorldDelta {
                tick: 13,
                base_tick: 12,
                spawned: vec![],
                removed: vec![],
                positions: vec![],
                nodes: vec![],
                economy: EconomyChanges {
                    balances: vec![],
                    inventories: vec![],
                    orders: vec![],
                    queues: vec![],
                },
                belts: vec![],
                inserters: vec![],
                extractors: vec![],
                weapons: vec![],
                healths: vec![],
                research: vec![],
                unit_orders: Vec::new(),
            },
        ]);
        let mut state = ReplicaState {
            replica: Some(replica),
            ..Default::default()
        };
        let mut hooks = IngestHooks {
            outstanding_request: true,
            ..Default::default()
        };
        let mut counters = IngestCounters::default();
        let mut options = crate::ClientOpts(ClientOptions::default());
        let config = BrowserIngestConfig::parse_json(
            r#"{"drop_world_changes":[],"hold_resync_snapshots":0,"max_buffered_batches":2}"#,
        )
        .unwrap();

        apply_browser_ingest_config(config, &mut hooks, &mut counters, &mut state, &mut options)
            .unwrap();

        let replica = state.replica.as_ref().unwrap();
        assert_eq!(replica.max_buffered_batches(), 2);
        assert_eq!(replica.buffered_batches(), 0);
        assert_eq!(replica.buffer_overflows, 1);
        assert_eq!(counters.buffer_overflows, 1);
        assert_eq!(replica.last_applied_tick, Some(10));
        assert!(hooks.outstanding_request);
    }
}

#[cfg(test)]
mod receipt_tests {
    use super::*;

    fn witness(kind: ReceiptKind, message_id: u16) -> ReceiptWitness {
        ReceiptWitness {
            connection_epoch: 7,
            seq: 12,
            kind,
            reliable: true,
            message_id: Some(message_id),
            remote_tick: 10,
        }
    }

    #[test]
    fn request_guard_coalesces_while_one_response_is_outstanding() {
        let mut hooks = IngestHooks::default();
        let mut counters = IngestCounters::default();
        assert_eq!(
            request_resync(&mut hooks, &mut counters),
            Some(Outgoing::ResyncRequest)
        );
        assert_eq!(request_resync(&mut hooks, &mut counters), None);
        assert!(hooks.outstanding_request);
        assert_eq!(counters.resync_requests_sent, 1);
        assert_eq!(counters.max_outstanding_requests, 1);
    }

    #[test]
    fn replayed_sequence_invalidates_receipt_ordering_proof() {
        let mut receipts = ReceiptWitnesses::default();
        receipts.records.extend([
            witness(ReceiptKind::Ack, 10),
            witness(ReceiptKind::Applied, 11),
            witness(ReceiptKind::Rejected, 12),
            witness(ReceiptKind::Ack, 13),
        ]);
        receipts.reliable_counts.insert(7, 4);
        assert_eq!(receipts.ordered_for(12, 7), None);
    }

    #[test]
    fn unique_ack_and_rejection_prove_ordering() {
        let mut receipts = ReceiptWitnesses::default();
        receipts.records.extend([
            witness(ReceiptKind::Ack, 10),
            witness(ReceiptKind::Rejected, 11),
        ]);
        receipts.reliable_counts.insert(7, 2);
        assert_eq!(receipts.ordered_for(12, 7), Some(true));
    }
}
