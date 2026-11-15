//! Sessions and slot claims.
//!
//! One `ClientOf` entity is one connection. A `Hello` turns it into a session (a `Session`
//! component) or gets it a `JoinRejected` and a delayed kick. A session sends `PlayerCommand` and
//! `ResyncRequest`. The server never creates a gameplay entity for a session.

#[path = "participant_trace.rs"]
pub(crate) mod participant_trace;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::{Duration, Instant};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use turts_mod_api::{Behaviour, ContentId, PackageId};
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::messages::*;
use turts_net_protocol::{CommandRejectReason, MAX_ORDER_UNITS, PlayerCommand};
use turts_sim::{
    ClaimError, CommandQueue, CompletedWorld, Owner, SimSet, Slots, Tick as SimTick,
    UnitId as SimUnitId, UnitType, UnitTypes,
};

use crate::{
    economylog::EconomyLogSink, export, session_diagnostics::SessionDiagnostics, stats::Stats,
};

/// How long a refused connection stays open after `JoinRejected`. A kick in the same frame
/// loses the message, so the kick waits by time and not by frames (spike fact, plan 6a).
pub const KICK_DELAY: Duration = Duration::from_millis(300);

const REASON_PROTOCOL: &str = "protocol version mismatch";
const REASON_LOCKFILE: &str = "lockfile hash mismatch";
const REASON_ALREADY_IN_MATCH: &str = "player already in match";
pub(crate) const REASON_NO_OPEN_SLOT: &str = "no open slot";
pub(crate) const REASON_DUPLICATE_WINDOW: &str = "This game is open in another window";

/// A connection that sent an accepted `Hello`.
#[derive(Component)]
pub struct Session {
    /// Last newer sequence consumed at the fixed-tick admission boundary.
    pub last_seq: Option<u32>,
    /// The slot of the player. It is permanent for the match (D25).
    pub slot: turts_sim::SlotId,
    pub player_id: u64,
}

/// Internal participants have no transport session and cannot be claimed through Hello.
#[derive(Resource, Default)]
pub(crate) struct ReservedParticipantIds(pub(crate) BTreeSet<u64>);

/// Whole tick_hz-scaled admission tokens, retained for the slot across reconnects.
#[derive(Resource, Default)]
pub struct RateLimitBuckets(BTreeMap<u8, ClientBuckets>);

#[derive(Clone, Copy)]
struct ClientBuckets {
    envelope_tokens: u64,
    unit_tokens: u64,
    last_tick: u32,
}

impl RateLimitBuckets {
    pub(crate) fn admit(&mut self, slot: u8, tick: u32, tick_hz: u16, units: usize) -> bool {
        let hz = u64::from(tick_hz.max(1));
        let envelope_capacity = 64 * hz;
        let unit_capacity = 16_384 * hz;
        let buckets = self.0.entry(slot).or_insert(ClientBuckets {
            envelope_tokens: envelope_capacity,
            unit_tokens: unit_capacity,
            last_tick: tick,
        });
        let elapsed = u64::from(tick.saturating_sub(buckets.last_tick));
        buckets.envelope_tokens = buckets
            .envelope_tokens
            .saturating_add(elapsed.saturating_mul(640))
            .min(envelope_capacity);
        buckets.unit_tokens = buckets
            .unit_tokens
            .saturating_add(elapsed.saturating_mul(163_840))
            .min(unit_capacity);
        buckets.last_tick = tick;

        let envelope_cost = hz;
        let unit_cost = (units as u64).saturating_mul(hz);
        if buckets.envelope_tokens < envelope_cost || buckets.unit_tokens < unit_cost {
            return false;
        }
        buckets.envelope_tokens -= envelope_cost;
        buckets.unit_tokens -= unit_cost;
        true
    }
}

/// Unlink the connection when `at` has passed. `JoinRejected` has left by then.
#[derive(Component)]
pub struct PendingKick {
    pub at: Instant,
}

/// What the server advertises over HTTP and what a `Hello` must match.
#[derive(Resource)]
pub struct Advertised {
    pub lockfile_hash: String,
    pub tick_hz: u16,
}

/// The session gets a `Snapshot` of the current tick in this frame: after `Welcome`, and when
/// it sends a `ResyncRequest`.
#[derive(Component)]
pub struct SnapshotDue;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ParticipantTraceComplete;

/// Add the session systems. They run in `PreUpdate`, after lightyear has received messages.
pub fn install(app: &mut App, advertised: Advertised) {
    app.insert_resource(advertised);
    app.init_resource::<crate::permissions::ObservationPolicy>();
    app.init_resource::<StagedPlayerCommands>()
        .init_resource::<PendingOrderCommands>()
        .init_resource::<CommandAdmissionDiagnostics>()
        .init_resource::<ReliableTraffic>()
        .init_resource::<RateLimitBuckets>();
    app.add_observer(retire_disconnected_session);
    app.add_observer(crate::lobby::retire_connection);
    app.add_systems(
        FixedPreUpdate,
        admit_player_commands
            .after(SimSet::Construction)
            .before(SimSet::Commands),
    );
    app.add_systems(
        PreUpdate,
        (
            receive_hello,
            crate::lobby::reject_gameplay,
            crate::lobby::receive_actions,
            crate::lobby::drive,
            receive_player_commands,
            receive_resync,
            send_snapshots,
        )
            .chain()
            .after(MessageSystems::Receive),
    );
    app.add_systems(Update, kick_pending);
    app.add_systems(
        FixedLast,
        participant_trace::completed.in_set(ParticipantTraceComplete),
    );
}

/// A closed transport no longer holds a live application session. Its permanent slot and
/// rate-limit bucket stay intact. Already admitted gameplay orders continue in the sim.
fn retire_disconnected_session(
    trigger: On<Add<Disconnected>>,
    sessions: Query<(), With<Session>>,
    mut staged: ResMut<StagedPlayerCommands>,
    mut commands: Commands,
) {
    if sessions.contains(trigger.entity) {
        staged
            .0
            .retain(|command| command.connection != trigger.entity);
        commands
            .entity(trigger.entity)
            .remove::<(Session, SnapshotDue)>();
    }
}

/// The `MatchStart` of a `Welcome` under `Immediate` (design 6.7.5): the start tick is 1; each
/// slot is its own team; the colour of a slot is its default palette index, which is the slot
/// index (a stage-1 seat never has another colour, design 6.7.6). The lobby start step
/// (`L-MATCH-SRV1`) builds the same record from the room.
pub fn immediate_match_start() -> turts_net_protocol::MatchStart {
    let slots: Vec<u8> = (0..turts_sim::slots::SLOT_COUNT).collect();
    turts_net_protocol::MatchStart {
        start_tick: 1,
        teams: slots.clone(),
        colours: slots,
    }
}

/// Decide a `Hello`. The order of the checks fixes the reason when several fail: protocol
/// version, lockfile hash, then the slot claim. The claim runs last, so a refused `Hello`
/// takes no slot.
pub fn judge_hello(
    hello: &Hello,
    advertised: &Advertised,
    slots: &mut Slots,
    has_live_session: bool,
) -> Result<turts_sim::SlotId, &'static str> {
    if hello.protocol_version != NET_PROTOCOL_VERSION {
        return Err(REASON_PROTOCOL);
    }
    if hello.lockfile_hash != advertised.lockfile_hash {
        return Err(REASON_LOCKFILE);
    }
    match slots.claim(hello.player_id) {
        Ok(slot) => Ok(slot),
        Err(ClaimError::AlreadyClaimed) if !has_live_session => slots
            .slot_of(hello.player_id)
            .ok_or(REASON_ALREADY_IN_MATCH),
        Err(ClaimError::AlreadyClaimed) => Err(REASON_DUPLICATE_WINDOW),
        Err(ClaimError::NoOpenSlot) => Err(REASON_NO_OPEN_SLOT),
    }
}

/// Decide a Hello while tracking accepted identities in the current receive-system invocation.
fn judge_hello_in_batch(
    hello: &Hello,
    advertised: &Advertised,
    slots: &mut Slots,
    has_live_session: bool,
    accepted_in_batch: &mut BTreeSet<u64>,
) -> Result<turts_sim::SlotId, &'static str> {
    let has_live_session = has_live_session || accepted_in_batch.contains(&hello.player_id);
    let result = judge_hello(hello, advertised, slots, has_live_session);
    if result.is_ok() {
        accepted_in_batch.insert(hello.player_id);
    }
    result
}

/// The connections that may send a `Hello`.
type HelloClients<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut MessageReceiver<Hello>,
        Has<Session>,
        Has<PendingKick>,
        Has<crate::lobby::LobbyMember>,
    ),
    (With<ClientOf>, Without<Disconnected>),
>;

/// The two answers to a `Hello`.
#[derive(SystemParam)]
pub struct HelloReplies<'w, 's> {
    welcomes: Query<'w, 's, &'static mut MessageSender<Welcome>>,
    rejections: Query<'w, 's, &'static mut MessageSender<JoinRejected>>,
}

/// Read the `Hello` of each connection. The first `Hello` decides. A connection that already
/// has a session, or waits for its kick, gets no second decision.
#[allow(clippy::too_many_arguments)]
pub fn receive_hello(
    mut commands: Commands,
    mut clients: HelloClients,
    mut replies: HelloReplies,
    mut slots: ResMut<Slots>,
    mut stats: ResMut<Stats>,
    tick: Res<SimTick>,
    advertised: Res<Advertised>,
    mut traffic: ResMut<ReliableTraffic>,
    mut economy_log: Option<ResMut<EconomyLogSink>>,
    match_id: Res<crate::stats::MatchId>,
    live_sessions: Query<&Session, Without<Disconnected>>,
    mut diagnostics: Option<ResMut<SessionDiagnostics>>,
    reserved: Option<Res<ReservedParticipantIds>>,
    mut lobby: Option<ResMut<crate::lobby::HostedLobby>>,
) {
    let mut accepted_in_batch = BTreeSet::new();
    for (entity, mut receiver, has_session, pending_kick, lobby_member) in clients.iter_mut() {
        let mut decided = has_session || pending_kick || lobby_member;
        for hello in receiver.receive() {
            traffic.record(entity);
            if decided {
                continue;
            }
            decided = true;
            stats.hellos += 1;
            if let Some(lobby) = lobby.as_mut().filter(|lobby| lobby.waiting()) {
                let result = if hello.protocol_version != NET_PROTOCOL_VERSION {
                    Err(REASON_PROTOCOL)
                } else if hello.lockfile_hash != advertised.lockfile_hash {
                    Err(REASON_LOCKFILE)
                } else {
                    lobby.join(entity, hello.player_id)
                };
                match result {
                    Ok(()) => {
                        commands.entity(entity).insert(crate::lobby::LobbyMember);
                    }
                    Err(reason) => {
                        if let Ok(mut sender) = replies.rejections.get_mut(entity) {
                            sender.send::<ReliableChannel>(JoinRejected {
                                reason: reason.into(),
                            });
                        }
                        commands.entity(entity).insert(PendingKick {
                            at: Instant::now() + KICK_DELAY,
                        });
                    }
                }
                continue;
            }

            let has_live_session = reserved
                .as_ref()
                .is_some_and(|participants| participants.0.contains(&hello.player_id))
                || live_sessions
                    .iter()
                    .any(|session| session.player_id == hello.player_id);
            let accepted_in_batch_before = accepted_in_batch.contains(&hello.player_id);
            let owners_before = diagnostics.is_some().then(|| slots.owners());
            let already_claimed = slots.slot_of(hello.player_id).is_some();
            let result = judge_hello_in_batch(
                &hello,
                &advertised,
                &mut slots,
                has_live_session,
                &mut accepted_in_batch,
            );
            if let (Some(diagnostics), Some(owners_before)) =
                (diagnostics.as_deref_mut(), owners_before.as_deref())
            {
                let observed_result = result.as_ref().map(|slot| *slot).map_err(|reason| *reason);
                if let Err(error) =
                    diagnostics.hello_decision(crate::session_diagnostics::HelloObservation {
                        entity,
                        player_id: hello.player_id,
                        tick: tick.0,
                        match_id: match_id.0,
                        has_live_session,
                        accepted_in_batch_before,
                        accepted_in_batch_after: accepted_in_batch.contains(&hello.player_id),
                        result: observed_result,
                        owners_before,
                        owners_after: &slots.owners(),
                    })
                {
                    panic!("private session observer failed: {error}");
                }
            }
            match result {
                Ok(slot) => {
                    if let Some(lobby) = lobby.as_mut() {
                        lobby
                            .join(entity, hello.player_id)
                            .expect("durable session rejoin matches the room claim");
                    }
                    if !already_claimed && let Some(log) = economy_log.as_mut() {
                        log.write_claim(tick.0, hello.player_id, slot.0);
                    }
                    eprintln!(
                        "turts_server: player {} took slot {}",
                        hello.player_id, slot.0
                    );
                    commands.entity(entity).insert((
                        Session {
                            last_seq: None,
                            slot,
                            player_id: hello.player_id,
                        },
                        SnapshotDue,
                    ));
                    if let Ok(mut sender) = replies.welcomes.get_mut(entity) {
                        sender.send::<ReliableChannel>(Welcome {
                            slot_id: SlotId(slot.0),
                            tick: tick.0,
                            tick_hz: advertised.tick_hz,
                            match_id: match_id.0,
                            start: lobby
                                .as_ref()
                                .and_then(|lobby| lobby.start.clone())
                                .unwrap_or_else(immediate_match_start),
                        });
                        traffic.record(entity);
                    }
                }
                Err(reason) => {
                    eprintln!("turts_server: player {} refused: {reason}", hello.player_id);
                    if let Ok(mut sender) = replies.rejections.get_mut(entity) {
                        sender.send::<ReliableChannel>(JoinRejected {
                            reason: reason.to_string(),
                        });
                        traffic.record(entity);
                    }
                    commands.entity(entity).insert(PendingKick {
                        at: Instant::now() + KICK_DELAY,
                    });
                }
            }
        }
    }
}

/// A received envelope retained until a fixed simulation tick is admitted.
#[derive(Debug, Clone)]
pub struct StagedPlayerCommand {
    pub connection: Entity,
    pub slot: turts_sim::SlotId,
    pub envelope: PlayerCommand,
}
#[derive(Resource, Debug, Default)]
pub struct StagedPlayerCommands(pub VecDeque<StagedPlayerCommand>);
#[derive(Resource, Debug, Default)]
pub struct PendingOrderCommands(pub BTreeMap<(u8, u32), PlayerCommand>);
#[derive(Debug, Clone, PartialEq)]
pub struct AdmissionRecord {
    pub envelope: PlayerCommand,
    pub seq: u32,
    pub slot: u8,
    pub player_id: u64,
    pub tick: u32,
    pub reason: CommandRejectReason,
}
#[derive(Resource, Debug, Default)]
pub struct CommandAdmissionDiagnostics(pub Vec<AdmissionRecord>);
#[derive(Resource, Debug, Default)]
pub struct ReliableTraffic {
    pub counts: BTreeMap<u64, u32>,
    pub invalid_epochs: BTreeSet<u64>,
}
impl ReliableTraffic {
    pub fn record(&mut self, entity: Entity) {
        let epoch = entity.to_bits();
        let count = self.counts.entry(epoch).or_default();
        *count = count.saturating_add(1);
        if *count >= 32768 {
            self.invalid_epochs.insert(epoch);
        }
    }
}

/// Read client envelopes before Lightyear clears the frame receivers. Ack means receipt only.
#[allow(clippy::type_complexity)]
pub fn receive_player_commands(
    mut clients: Query<
        (
            Entity,
            &mut MessageReceiver<PlayerCommand>,
            Option<&Session>,
        ),
        (With<ClientOf>, Without<Disconnected>),
    >,
    mut acks: Query<&mut MessageSender<CommandAck>>,
    tick: Res<SimTick>,
    mut staged: ResMut<StagedPlayerCommands>,
    mut traffic: ResMut<ReliableTraffic>,
) {
    for (entity, mut receiver, session) in &mut clients {
        let Some(session) = session else {
            receiver.receive().for_each(drop);
            continue;
        };
        for received in receiver.receive_with_tick() {
            let envelope = received.data;
            traffic.record(entity);
            if let Ok(mut sender) = acks.get_mut(entity) {
                sender.send::<ReliableChannel>(CommandAck {
                    seq: envelope.seq,
                    tick: tick.0,
                });
                traffic.record(entity);
            }
            staged.0.push_back(StagedPlayerCommand {
                connection: entity,
                slot: session.slot,
                envelope,
            });
        }
    }
}

/// Admit the staged batch for the advanced tick before the domain command drain.
/// Consume sequence numbers identically for transported and internal participants.
pub(crate) fn advance_sequence(last: &mut Option<u32>, sequence: u32) -> bool {
    if last.is_some_and(|previous| sequence <= previous) {
        return false;
    }
    *last = Some(sequence);
    true
}

/// Shared ordinary admission after the caller authenticates its participant identity.
/// Rate and sequence tokens are consumed before this function, even for bad envelopes.
pub(crate) fn admit_actor_envelope(
    world: &mut World,
    slot: turts_sim::SlotId,
    envelope: &PlayerCommand,
    admitted: bool,
    newer: bool,
) -> Result<Vec<turts_sim::Command>, (OrderResult, CommandRejectReason)> {
    let reason = if !admitted {
        Some(CommandRejectReason::RateLimited)
    } else if !newer {
        Some(CommandRejectReason::BadSequence)
    } else if world
        .get_resource::<turts_sim::outcome::MatchOutcomes>()
        .is_some_and(|state| state.end_tick.is_some())
    {
        Some(CommandRejectReason::MatchOver)
    } else if turts_sim::outcome::slot_defeated(world, slot.0) {
        Some(CommandRejectReason::SlotDefeated)
    } else {
        production_precheck(world, slot, envelope)
    };
    if let Some(reason) = reason {
        return Err((crate::orders::refusal(envelope, reason), reason));
    }
    let catalog = &world.resource::<crate::orders::ServerCatalog>().0;
    crate::orders::to_sim(slot, envelope, catalog).map_err(|mut result| {
        result.tick = world.resource::<SimTick>().0;
        let reason = result
            .envelope
            .or_else(|| result.refused.first().map(|row| row.reason))
            .unwrap_or(CommandRejectReason::Internal);
        (result, reason)
    })
}

pub fn admit_player_commands(world: &mut World) {
    if participant_trace::replay_enabled(world) {
        world.resource_mut::<StagedPlayerCommands>().0.clear();
        participant_trace::admit_recorded_humans(world);
        return;
    }
    let batch = std::mem::take(&mut world.resource_mut::<StagedPlayerCommands>().0);
    let tick = world.resource::<SimTick>().0;
    for staged in batch {
        if world.contains_resource::<turts_sim::economy::events::MatchStopped>() {
            continue;
        }
        // The rate-limit bucket runs first for every envelope (design 6.8, [net 5]): a refused
        // envelope costs a token, so a flood of bad sequences or bad targets is limited too.
        let tick_hz = world.resource::<Advertised>().tick_hz;
        let admitted = world.resource_mut::<RateLimitBuckets>().admit(
            staged.slot.0,
            tick,
            tick_hz,
            staged.envelope.order.units.len(),
        );

        let newer = if let Some(mut session) = world.get_mut::<Session>(staged.connection) {
            session.slot == staged.slot
                && advance_sequence(&mut session.last_seq, staged.envelope.seq)
        } else {
            false
        };
        let identity = world.get::<Session>(staged.connection).map(|session| {
            turts_audit::ActorLogIdentity::Human {
                player_id: session.player_id,
                slot: session.slot.0,
                connection: staged
                    .connection
                    .to_bits()
                    .checked_add(1)
                    .expect("connection epoch exhausted"),
            }
        });
        let ordinal = identity.as_ref().and_then(|identity| {
            match participant_trace::submission(world, identity, None, &staged.envelope) {
                Ok(ordinal) => ordinal,
                Err(error) => {
                    participant_trace::fail(world, error);
                    None
                }
            }
        });
        match admit_actor_envelope(world, staged.slot, &staged.envelope, admitted, newer) {
            Ok(commands) => {
                if let Some(identity) = identity.clone()
                    && let Err(error) = participant_trace::accept(
                        world,
                        staged.slot.0,
                        staged.envelope.seq,
                        ordinal,
                        identity,
                    )
                {
                    participant_trace::fail(world, error);
                }
                world.resource_mut::<PendingOrderCommands>().0.insert(
                    (staged.slot.0, staged.envelope.seq),
                    staged.envelope.clone(),
                );
                world.resource_mut::<CommandQueue>().0.extend(commands);
            }
            Err((result, reason)) => record_admission_rejection(
                world,
                staged,
                tick,
                result,
                reason,
                identity.zip(ordinal),
            ),
        }
    }
}

/// Preserve the existing information-safe Produce admission order while the open envelope is
/// resolved by package behavior: ownership, capability, then target reference syntax.
fn production_precheck(
    world: &mut World,
    slot: turts_sim::SlotId,
    command: &PlayerCommand,
) -> Option<CommandRejectReason> {
    if command.order.units.is_empty() || command.order.units.len() > MAX_ORDER_UNITS {
        return None;
    }
    let order_id = ContentId::parse(&command.order.order).ok()?;
    let is_produce = world
        .resource::<crate::orders::ServerCatalog>()
        .0
        .orders
        .get(&order_id)
        .is_some_and(|definition| definition.behaviour == Behaviour::Produce);
    if !is_produce {
        return None;
    }
    let OrderTarget::Content(reference) = &command.order.target else {
        return None;
    };

    let representable = ContentId::parse(reference)
        .and_then(|id| {
            PackageId::parse(&id.package.0).map(|package| ContentId {
                package,
                local: id.local,
            })
        })
        .is_ok();
    if command.order.units.len() > 1 {
        // Multi-subject envelopes always reach per-unit validation, including malformed
        // content targets, so ownership and capability priority is resolved per subject.
        return None;
    }
    if representable
        && command.order.units.iter().any(|unit| {
            owns_unit(world, unit.0, slot) == Some(true)
                && has_production_capability(world, unit.0) == Some(true)
        })
    {
        // A syntactically representable mixed envelope must receive per-unit domain results.
        return None;
    }

    for unit in &command.order.units {
        match owns_unit(world, unit.0, slot) {
            Some(true) => {}
            Some(false) => return Some(CommandRejectReason::NotOwner),
            None => return Some(CommandRejectReason::UnknownUnit),
        }
    }
    if command
        .order
        .units
        .iter()
        .any(|unit| has_production_capability(world, unit.0) != Some(true))
    {
        return Some(CommandRejectReason::MissingCapability);
    }
    let parsed = ContentId::parse(reference).and_then(|id| {
        PackageId::parse(&id.package.0).map(|package| ContentId {
            package,
            local: id.local,
        })
    });
    parsed
        .is_err()
        .then_some(CommandRejectReason::UnknownRecipe)
}

fn owns_unit(world: &mut World, unit_id: u32, slot: turts_sim::SlotId) -> Option<bool> {
    let mut query = world.query::<(&SimUnitId, &Owner)>();
    query
        .iter(world)
        .find(|(id, _)| id.0 == unit_id)
        .map(|(_, owner)| owner.0 == slot)
}

fn has_production_capability(world: &World, unit_id: u32) -> Option<bool> {
    let unit_type = world.iter_entities().find_map(|entity| {
        let id = entity.get::<SimUnitId>()?;
        (id.0 == unit_id)
            .then(|| entity.get::<UnitType>().copied())
            .flatten()
    })?;
    let types = world.get_resource::<UnitTypes>()?;
    let entry = types.get(unit_type.0)?;
    let content_id = ContentId::parse(&entry.content_id).ok()?;
    let catalog = world.get_resource::<turts_sim::bootstrap::InstalledCatalog>()?;
    Some(
        catalog
            .0
            .units
            .get(&content_id)
            .is_some_and(|unit| unit.producer.is_some() || unit.research.is_some()),
    )
}

fn postcard_wire_bytes(envelope: &PlayerCommand) -> Option<Vec<u8>> {
    let mut buffer = vec![0; 256];
    loop {
        match postcard::to_slice(envelope, &mut buffer) {
            Ok(encoded) => return Some(encoded.to_vec()),
            Err(postcard::Error::SerializeBufferFull) => {
                buffer.resize(buffer.len().checked_mul(2)?, 0);
            }
            Err(_) => return None,
        }
    }
}

fn record_admission_rejection(
    world: &mut World,
    staged: StagedPlayerCommand,
    tick: u32,
    mut order_result: OrderResult,
    reason: CommandRejectReason,
    trace: Option<(turts_audit::ActorLogIdentity, u64)>,
) {
    let player_id = world
        .get::<Session>(staged.connection)
        .map_or(0, |session| session.player_id);
    let admission_evidence = if trace.is_none() && world.contains_resource::<EconomyLogSink>() {
        let valid_capture = world
            .get_resource::<turts_sim::AuditTrail>()
            .is_some_and(|trail| trail.enabled && trail.valid);
        if !valid_capture {
            if let Some(mut sink) = world.get_resource_mut::<EconomyLogSink>() {
                sink.error = Some("wire admission had no valid economy audit capture".into());
            }
            None
        } else {
            let before = turts_sim::audit_state(world);
            match postcard_wire_bytes(&staged.envelope) {
                Some(raw) => Some((before, raw)),
                None => {
                    if let Some(mut sink) = world.get_resource_mut::<EconomyLogSink>() {
                        sink.error =
                            Some("could not postcard-encode rejected command envelope".into());
                    }
                    None
                }
            }
        }
    } else {
        None
    };
    let ordinal = world
        .resource::<CommandAdmissionDiagnostics>()
        .0
        .len()
        .saturating_add(1) as u64;
    world
        .resource_mut::<CommandAdmissionDiagnostics>()
        .0
        .push(AdmissionRecord {
            envelope: staged.envelope.clone(),
            seq: staged.envelope.seq,
            slot: staged.slot.0,
            player_id,
            tick,
            reason,
        });
    order_result.tick = tick;
    if let Some((identity, ordinal)) = trace
        && let Err(error) = participant_trace::rejection(
            world,
            &identity,
            Some(ordinal),
            &staged.envelope,
            &order_result,
            reason,
        )
    {
        participant_trace::fail(world, error);
    }
    if let Some(mut sender) = world.get_mut::<MessageSender<OrderResult>>(staged.connection) {
        sender.send::<ReliableChannel>(order_result);
        world
            .resource_mut::<ReliableTraffic>()
            .record(staged.connection);
    }
    if let Some((before, raw)) = admission_evidence {
        let after = turts_sim::audit_state(world);
        if let Some(mut sink) = world.get_resource_mut::<EconomyLogSink>() {
            sink.write_admission(
                ordinal,
                staged.connection.to_bits(),
                player_id,
                staged.slot.0,
                staged.envelope.seq,
                tick,
                reason,
                raw,
                before,
                after,
            );
        }
    }
}

/// The connections that may send a `ResyncRequest`.
type ResyncClients<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut MessageReceiver<ResyncRequest>,
        Has<Session>,
    ),
    (With<ClientOf>, Without<Disconnected>),
>;

/// Answer each `ResyncRequest` of a session with a `Snapshot` of the current tick. Several
/// requests in one frame get one `Snapshot`.
pub fn receive_resync(
    mut commands: Commands,
    mut clients: ResyncClients,
    mut stats: ResMut<Stats>,
    mut traffic: ResMut<ReliableTraffic>,
) {
    for (entity, mut receiver, has_session) in clients.iter_mut() {
        for _request in receiver.receive() {
            traffic.record(entity);
            if has_session {
                stats.resync_requests += 1;
                commands.entity(entity).insert(SnapshotDue);
            }
        }
    }
}

/// Send one owner-specific `Snapshot` from the latest completed world to each session that waits
/// for one. Welcome and Snapshot leave in this order on the ordered reliable channel.
pub fn send_snapshots(world: &mut World, mut due: Local<QueryState<Entity, With<SnapshotDue>>>) {
    let entities: Vec<Entity> = due.iter(world).collect();
    if entities.is_empty() {
        return;
    }
    let payloads: Vec<_> = entities
        .into_iter()
        .filter_map(|entity| {
            let slot = world.get::<Session>(entity).map(|session| session.slot);
            let payload = slot.map(|slot| {
                let policy = crate::permissions::completed(world);
                let (message, history) = export::snapshot_for_with_permissions(
                    &world.resource::<CompletedWorld>().0,
                    slot,
                    policy.0.as_ref(),
                );
                world.entity_mut(entity).insert(history);
                (entity, message)
            });
            world.entity_mut(entity).remove::<SnapshotDue>();
            payload
        })
        .collect();
    for (entity, message) in payloads {
        if let Some(mut sender) = world.get_mut::<MessageSender<Snapshot>>(entity) {
            sender.send::<ReliableChannel>(message);
            world.resource_mut::<ReliableTraffic>().record(entity);
        }
    }
}

/// Unlink each refused connection when its delay has passed. `Disconnect` on a `ClientOf` does
/// nothing (spike fact); `Unlink` closes it.
pub fn kick_pending(mut commands: Commands, pending: Query<(Entity, &PendingKick)>) {
    let now = Instant::now();
    for (entity, kick) in pending.iter() {
        if now >= kick.at {
            commands.entity(entity).remove::<PendingKick>();
            commands.trigger(Unlink {
                entity,
                reason: UnlinkReason::UserRequested(None),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "sha256:advertised";

    fn advertised() -> Advertised {
        Advertised {
            lockfile_hash: HASH.to_string(),
            tick_hz: 20,
        }
    }

    fn hello(player_id: u64, protocol_version: u32, lockfile_hash: &str) -> Hello {
        Hello {
            player_id,
            protocol_version,
            lockfile_hash: lockfile_hash.to_string(),
        }
    }

    fn good(player_id: u64) -> Hello {
        hello(player_id, NET_PROTOCOL_VERSION, HASH)
    }

    fn spawn_hello_peers(app: &mut App, id: u64) -> (Entity, Entity) {
        let registry = app.world().resource::<ChannelRegistry>();
        let mut sender_transport = Transport::default();
        sender_transport.add_channel_send_from_registry::<ReliableChannel>(registry);
        sender_transport.add_channel_receive_from_registry::<ReliableChannel>(registry);
        let sender = app
            .world_mut()
            .spawn((
                Client,
                Link::default(),
                sender_transport,
                MessageSender::<Hello>::default(),
                MessageSender::<LobbyAction>::default(),
                MessageSender::<PlayerCommand>::default(),
                MessageReceiver::<LobbyState>::default(),
                MessageReceiver::<LobbyRefused>::default(),
                MessageReceiver::<OrderResult>::default(),
                MessageReceiver::<CommandAck>::default(),
                RemoteId(PeerId::Server),
                Linked,
                Connected,
            ))
            .id();
        let receiver = app
            .world_mut()
            .spawn((
                ClientOf,
                Link::default(),
                Transport::default(),
                RemoteId(PeerId::Local(id)),
                Linked,
                Connected,
                MessageReceiver::<Hello>::default(),
                MessageSender::<Welcome>::default(),
                MessageSender::<JoinRejected>::default(),
                MessageSender::<LobbyState>::default(),
                MessageSender::<LobbyRefused>::default(),
                MessageReceiver::<LobbyAction>::default(),
                MessageReceiver::<PlayerCommand>::default(),
                MessageSender::<CommandAck>::default(),
                MessageSender::<OrderResult>::default(),
            ))
            .id();
        (sender, receiver)
    }

    fn hello_test_app() -> App {
        use lightyear::prelude::server::ServerPlugins;

        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.add_plugins(turts_sim::SimPlugin { tick_hz: 20 });
        app.insert_resource(turts_sim::MatchConfig::immediate(20));
        app.init_resource::<crate::stats::Stats>();
        app.insert_resource(crate::stats::MatchId(0));
        install(&mut app, advertised());
        app.finish();
        app.cleanup();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));
        app.update();
        app
    }

    fn deliver_hello(app: &mut App, source: Entity, client: Entity, hello: Hello) {
        use lightyear::link::recv_payload_from_bytes;

        app.world_mut()
            .get_mut::<MessageSender<Hello>>(source)
            .unwrap()
            .send::<ReliableChannel>(hello);
        app.update();
        let packets = {
            let mut entity = app.world_mut().entity_mut(source);
            let mut link = entity.get_mut::<Link>().unwrap();
            let mut packets = Vec::new();
            while let Some(packet) = link.send.pop() {
                packets.push(packet);
            }
            packets
        };
        assert!(
            !packets.is_empty(),
            "the Hello uses the real Link transport"
        );
        for packet in packets {
            app.world_mut()
                .entity_mut(client)
                .get_mut::<Link>()
                .unwrap()
                .recv
                .push_raw(recv_payload_from_bytes(packet));
        }
        app.update();
    }

    #[derive(Resource, Default)]
    struct LobbyRepliesCaptured {
        states: Vec<LobbyState>,
        orders: Vec<OrderResult>,
        refusals: Vec<LobbyRefused>,
    }
    fn capture_lobby_replies(
        mut states: Query<&mut MessageReceiver<LobbyState>>,
        mut orders: Query<&mut MessageReceiver<OrderResult>>,
        mut refusals: Query<&mut MessageReceiver<LobbyRefused>>,
        mut captured: ResMut<LobbyRepliesCaptured>,
    ) {
        for mut reader in &mut states {
            captured.states.extend(reader.receive());
        }
        for mut reader in &mut orders {
            captured.orders.extend(reader.receive());
        }
        for mut reader in &mut refusals {
            captured.refusals.extend(reader.receive());
        }
    }

    fn lobby_app() -> App {
        let mut app = hello_test_app();
        app.init_resource::<LobbyRepliesCaptured>().add_systems(
            PreUpdate,
            capture_lobby_replies
                .after(MessageSystems::Receive)
                .before(receive_hello),
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
        let archive = turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&path).unwrap(),
        )
        .unwrap();
        turts_pack::load_package(app.world_mut(), &archive).unwrap();
        app.insert_resource(crate::orders::ServerCatalog(archive.content().unwrap()));
        app.world_mut().remove_resource::<turts_sim::MatchConfig>();
        app.insert_resource(crate::lobby::HostedLobby::new(
            turts_mod_api::StartPolicy::AllSeatedReady { countdown_secs: 3 },
            2,
        ));
        turts_sim::initialize_completed_world(app.world_mut());
        app
    }

    fn relay_test_packets(app: &mut App, source: Entity, client: Entity) {
        app.update();
        let packets = {
            let mut link = app.world_mut().get_mut::<Link>(source).unwrap();
            let mut packets = Vec::new();
            while let Some(packet) = link.send.pop() {
                packets.push(packet);
            }
            packets
        };
        assert!(!packets.is_empty());
        for packet in packets {
            app.world_mut()
                .get_mut::<Link>(client)
                .unwrap()
                .recv
                .push_raw(lightyear::link::recv_payload_from_bytes(packet));
        }
        app.update();
    }

    fn lobby_action(app: &mut App, source: Entity, client: Entity, action: LobbyActionKind) {
        let revision = app
            .world()
            .resource::<crate::lobby::HostedLobby>()
            .room
            .seats()
            .revision();
        app.world_mut()
            .get_mut::<MessageSender<LobbyAction>>(source)
            .unwrap()
            .send::<ReliableChannel>(LobbyAction { revision, action });
        relay_test_packets(app, source, client);
    }

    fn advance_lobby(app: &mut App, seconds: f64) {
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f64(seconds),
        ));
        app.update();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));
    }

    #[test]
    fn independent_review_first_real_hello_receives_typed_lobby_state() {
        let mut app = lobby_app();
        let (a, ca) = spawn_hello_peers(&mut app, 1);
        deliver_hello(&mut app, a, ca, good(7));
        relay_test_packets(&mut app, ca, a);
        let states = &app.world().resource::<LobbyRepliesCaptured>().states;
        assert!(
            !states.is_empty(),
            "first Hello must receive LobbyState through the typed reliable transport"
        );
        assert_eq!(states.last().unwrap().you, Some(0));
        assert_eq!(states.last().unwrap().seats.len(), 2);
    }

    #[test]
    fn independent_review_prestart_order_in_final_ready_packet_is_refused() {
        let mut app = lobby_app();
        app.insert_resource(crate::lobby::HostedLobby::new(
            turts_mod_api::StartPolicy::AllSeatedReady { countdown_secs: 0 },
            2,
        ));
        let (a, ca) = spawn_hello_peers(&mut app, 1);
        deliver_hello(&mut app, a, ca, good(7));
        let (b, cb) = spawn_hello_peers(&mut app, 2);
        deliver_hello(&mut app, b, cb, good(8));
        lobby_action(&mut app, a, ca, LobbyActionKind::SetReady(true));
        let revision = app
            .world()
            .resource::<crate::lobby::HostedLobby>()
            .room
            .seats()
            .revision();
        app.world_mut()
            .get_mut::<MessageSender<PlayerCommand>>(b)
            .unwrap()
            .send::<ReliableChannel>(PlayerCommand {
                seq: 1,
                order: OrderEnvelope {
                    order: "skirmish:move".into(),
                    units: vec![UnitId(3)],
                    target: OrderTarget::Point(Pos { x: 18.0, y: 30.0 }),
                    queue: QueueMode::Replace,
                    group: 0,
                },
            });
        app.world_mut()
            .get_mut::<MessageSender<LobbyAction>>(b)
            .unwrap()
            .send::<ReliableChannel>(LobbyAction {
                revision,
                action: LobbyActionKind::SetReady(true),
            });
        relay_test_packets(&mut app, b, cb);
        assert!(app.world().contains_resource::<turts_sim::MatchConfig>());
        assert!(app.world().resource::<StagedPlayerCommands>().0.is_empty());
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        relay_test_packets(&mut app, cb, b);
        assert!(
            app.world()
                .resource::<LobbyRepliesCaptured>()
                .orders
                .iter()
                .any(|reply| reply.seq == 1
                    && reply.envelope == Some(CommandRejectReason::MatchNotStarted))
        );
    }

    #[test]
    fn all_seated_zero_starts_when_the_second_human_joins() {
        let mut app = lobby_app();
        app.insert_resource(crate::lobby::HostedLobby::new(
            turts_mod_api::StartPolicy::AllSeated { countdown_secs: 0 },
            8,
        ));
        let (a, ca) = spawn_hello_peers(&mut app, 1);
        deliver_hello(&mut app, a, ca, good(7));
        assert!(!app.world().contains_resource::<turts_sim::MatchConfig>());
        let (b, cb) = spawn_hello_peers(&mut app, 2);
        deliver_hello(&mut app, b, cb, good(8));
        assert!(app.world().contains_resource::<turts_sim::MatchConfig>());
        assert_eq!(
            app.world().resource::<Slots>().owners(),
            vec![Some(7), Some(8)]
        );
        assert!(app.world().get::<Session>(ca).is_some());
        assert!(app.world().get::<Session>(cb).is_some());
    }

    #[test]
    fn lobby_waits_cancels_and_claims_once_with_authoritative_colours() {
        let mut app = lobby_app();
        let before = turts_sim::audit_state(app.world_mut());
        let (a, ca) = spawn_hello_peers(&mut app, 1);
        deliver_hello(&mut app, a, ca, good(7));
        assert!(app.world().get::<Session>(ca).is_none());
        assert!(app.world().get::<crate::lobby::LobbyMember>(ca).is_some());
        advance_lobby(&mut app, 20.0);
        assert_eq!(
            turts_sim::audit_state(app.world_mut()),
            before,
            "a waiting lobby changed gameplay state"
        );
        lobby_action(&mut app, a, ca, LobbyActionKind::SetColour(1));
        lobby_action(&mut app, a, ca, LobbyActionKind::SetReady(true));
        let (b, cb) = spawn_hello_peers(&mut app, 2);
        deliver_hello(&mut app, b, cb, good(8));
        lobby_action(&mut app, b, cb, LobbyActionKind::SetReady(true));
        assert_eq!(
            app.world()
                .resource::<crate::lobby::HostedLobby>()
                .room
                .state(),
            crate::room::RoomState::Countdown
        );
        advance_lobby(&mut app, 1.0);
        lobby_action(&mut app, b, cb, LobbyActionKind::SetReady(false));
        advance_lobby(&mut app, 4.0);
        assert_eq!(
            turts_sim::audit_state(app.world_mut()),
            before,
            "a cancelled countdown started gameplay"
        );
        lobby_action(&mut app, b, cb, LobbyActionKind::SetReady(true));
        advance_lobby(&mut app, 1.0);
        app.world_mut().entity_mut(cb).insert(Unlinked {
            reason: UnlinkReason::ByPeer("left lobby".into()),
        });
        app.update();
        advance_lobby(&mut app, 4.0);
        assert_eq!(app.world().resource::<Slots>().owners(), vec![None, None]);
        let (c, cc) = spawn_hello_peers(&mut app, 3);
        deliver_hello(&mut app, c, cc, good(9));
        lobby_action(&mut app, c, cc, LobbyActionKind::SetReady(true));
        advance_lobby(&mut app, 2.0);
        assert_eq!(app.world().resource::<SimTick>().0, 0);
        advance_lobby(&mut app, 1.1);
        assert!(app.world().contains_resource::<turts_sim::MatchConfig>());
        assert_eq!(
            app.world().resource::<Slots>().owners(),
            vec![Some(7), Some(9)]
        );
        assert_eq!(app.world().get::<Session>(ca).unwrap().slot.0, 0);
        assert_eq!(app.world().get::<Session>(cc).unwrap().slot.0, 1);
        assert_eq!(
            app.world()
                .resource::<crate::lobby::HostedLobby>()
                .start
                .as_ref()
                .unwrap()
                .colours,
            vec![1, 0]
        );
    }

    #[test]
    fn lobby_rejects_stale_actions_and_prestart_gameplay() {
        let mut app = lobby_app();
        let (a, ca) = spawn_hello_peers(&mut app, 1);
        deliver_hello(&mut app, a, ca, good(7));
        let stale = app
            .world()
            .resource::<crate::lobby::HostedLobby>()
            .room
            .seats()
            .revision();
        let (b, cb) = spawn_hello_peers(&mut app, 2);
        deliver_hello(&mut app, b, cb, good(8));
        app.world_mut()
            .get_mut::<MessageSender<LobbyAction>>(a)
            .unwrap()
            .send::<ReliableChannel>(LobbyAction {
                revision: stale,
                action: LobbyActionKind::SetReady(true),
            });
        relay_test_packets(&mut app, a, ca);
        assert!(
            !app.world()
                .resource::<crate::lobby::HostedLobby>()
                .room
                .seats()
                .seat(0)
                .unwrap()
                .ready
        );
        lobby_action(&mut app, a, ca, LobbyActionKind::SetColour(1));
        assert_eq!(
            app.world()
                .resource::<crate::lobby::HostedLobby>()
                .room
                .seats()
                .seat(0)
                .unwrap()
                .colour,
            0
        );
        lobby_action(&mut app, a, ca, LobbyActionKind::SetColour(250));
        assert_eq!(
            app.world()
                .resource::<crate::lobby::HostedLobby>()
                .room
                .seats()
                .seat(0)
                .unwrap()
                .colour,
            0
        );
        app.world_mut()
            .get_mut::<MessageSender<PlayerCommand>>(a)
            .unwrap()
            .send::<ReliableChannel>(PlayerCommand {
                seq: 1,
                order: OrderEnvelope {
                    order: "skirmish:move".into(),
                    units: vec![UnitId(3)],
                    target: OrderTarget::Point(Pos { x: 18.0, y: 30.0 }),
                    queue: QueueMode::Replace,
                    group: 0,
                },
            });
        relay_test_packets(&mut app, a, ca);
        assert!(app.world().resource::<StagedPlayerCommands>().0.is_empty());
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        assert_eq!(app.world().resource::<SimTick>().0, 0);
        relay_test_packets(&mut app, ca, a);
        let replies = app.world().resource::<LobbyRepliesCaptured>();
        assert!(
            replies.orders.iter().any(|reply| reply.seq == 1
                && reply.envelope == Some(CommandRejectReason::MatchNotStarted))
        );
        assert!(
            replies
                .refusals
                .iter()
                .any(|reply| reply.reason == LobbyRefusal::LobbyChanged)
        );
        assert!(
            replies
                .refusals
                .iter()
                .any(|reply| reply.reason == LobbyRefusal::ColourTaken)
        );
        assert!(
            replies
                .refusals
                .iter()
                .any(|reply| reply.reason == LobbyRefusal::ColourUnknown)
        );
        let (_, duplicate) = spawn_hello_peers(&mut app, 3);
        // The pure room also rejects the same live identity before permanent claims exist.
        assert!(
            app.world_mut()
                .resource_mut::<crate::lobby::HostedLobby>()
                .join(duplicate, 7)
                .is_err()
        );
    }

    #[test]
    fn judge_accepts_a_matching_hello_and_claims_the_first_open_slot() {
        let mut slots = Slots::new(2);
        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots, false),
            Ok(turts_sim::SlotId(0))
        );
        assert_eq!(
            judge_hello(&good(8), &advertised(), &mut slots, false),
            Ok(turts_sim::SlotId(1))
        );
    }

    #[test]
    fn judge_checks_protocol_before_hash_and_claims_nothing_on_refusal() {
        let mut slots = Slots::new(2);
        let both_wrong = hello(7, 2, "sha256:other");
        assert_eq!(
            judge_hello(&both_wrong, &advertised(), &mut slots, false),
            Err("protocol version mismatch")
        );
        let wrong_hash = hello(7, NET_PROTOCOL_VERSION, "sha256:other");
        assert_eq!(
            judge_hello(&wrong_hash, &advertised(), &mut slots, false),
            Err("lockfile hash mismatch")
        );
        assert_eq!(slots.slot_of(7), None, "a refused Hello takes no slot");
    }

    #[test]
    fn protocol_3_hello_is_refused() {
        let mut slots = Slots::new(2);
        assert_eq!(
            judge_hello(&hello(7, 3, HASH), &advertised(), &mut slots, false),
            Err("protocol version mismatch")
        );
        assert_eq!(slots.slot_of(7), None, "a refused Hello takes no slot");
    }

    /// Older clients receive a clear refusal before they can claim a slot.
    #[test]
    fn older_protocols_are_refused_before_a_slot_is_claimed() {
        assert_eq!(turts_net_protocol::NET_PROTOCOL_VERSION, 8);
        let mut slots = Slots::new(2);
        for version in [4, 5, 6, 7] {
            assert_eq!(
                judge_hello(&hello(7, version, HASH), &advertised(), &mut slots, false),
                Err("protocol version mismatch")
            );
            assert_eq!(slots.slot_of(7), None, "a refused Hello takes no slot");
        }
    }

    /// Under `Immediate` the `Welcome` carries the `MatchStart` (design 6.7.5): start tick 1, one
    /// team and one default colour for each slot.
    #[test]
    fn the_immediate_welcome_carries_the_match_start() {
        let start = immediate_match_start();
        assert_eq!(start.start_tick, 1);
        assert_eq!(start.teams, vec![0, 1]);
        assert_eq!(start.colours, vec![0, 1]);
        assert_eq!(start.teams.len(), usize::from(turts_sim::slots::SLOT_COUNT));
    }

    #[test]
    fn judge_refuses_the_same_player_twice_and_keeps_the_first_slot() {
        let mut slots = Slots::new(2);
        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots, false),
            Ok(turts_sim::SlotId(0))
        );
        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots, true),
            Err("This game is open in another window")
        );
        assert_eq!(slots.slot_of(7), Some(turts_sim::SlotId(0)));
        assert_eq!(slots.owner_of(turts_sim::SlotId(1)), None);
    }

    #[test]
    fn judge_refuses_a_third_player_when_no_slot_is_open() {
        let mut slots = Slots::new(2);
        judge_hello(&good(1), &advertised(), &mut slots, false).unwrap();
        judge_hello(&good(2), &advertised(), &mut slots, false).unwrap();
        assert_eq!(
            judge_hello(&good(3), &advertised(), &mut slots, false),
            Err("no open slot")
        );
    }

    #[test]
    fn reload_reclaims_a_held_slot_with_no_live_session() {
        let mut slots = Slots::new(2);
        let slot = slots.claim(7).unwrap();

        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots, false),
            Ok(slot),
        );
    }

    #[test]
    fn closed_transport_retires_session_and_rejoins_without_timeout() {
        let mut app = hello_test_app();
        let (source_a, client_a) = spawn_hello_peers(&mut app, 1);
        deliver_hello(&mut app, source_a, client_a, good(7));
        let (source_other, client_other) = spawn_hello_peers(&mut app, 2);
        deliver_hello(&mut app, source_other, client_other, good(8));
        let owners = app.world().resource::<Slots>().owners();
        for _ in 0..64 {
            assert!(
                app.world_mut()
                    .resource_mut::<RateLimitBuckets>()
                    .admit(0, 0, 20, 1)
            );
        }
        assert!(
            !app.world_mut()
                .resource_mut::<RateLimitBuckets>()
                .admit(0, 0, 20, 1)
        );
        let pending = |connection, unit| StagedPlayerCommand {
            connection,
            slot: turts_sim::SlotId(if connection == client_a { 0 } else { 1 }),
            envelope: PlayerCommand {
                seq: 1,
                order: OrderEnvelope {
                    order: "skirmish:move".into(),
                    units: vec![UnitId(unit)],
                    target: OrderTarget::Point(Pos { x: 18.0, y: 30.0 }),
                    queue: QueueMode::Replace,
                    group: 0,
                },
            },
        };
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend([pending(client_a, 1), pending(client_other, 2)]);
        app.world_mut().entity_mut(client_a).insert(Unlinked {
            reason: UnlinkReason::ByPeer("page reload".into()),
        });
        app.update();
        assert!(app.world().get::<Disconnected>(client_a).is_some());
        assert!(
            app.world().get::<Session>(client_a).is_none(),
            "closed transport still reserves the application session"
        );
        assert!(app.world().get::<SnapshotDue>(client_a).is_none());
        let staged = &app.world().resource::<StagedPlayerCommands>().0;
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].connection, client_other);
        assert!(app.world().get::<Session>(client_other).is_some());
        let (source_b, client_b) = spawn_hello_peers(&mut app, 3);
        deliver_hello(&mut app, source_b, client_b, good(7));
        assert_eq!(
            app.world().get::<Session>(client_b).unwrap().slot,
            turts_sim::SlotId(0)
        );
        assert_eq!(app.world().resource::<Slots>().owners(), owners);
        assert!(
            !app.world_mut()
                .resource_mut::<RateLimitBuckets>()
                .admit(0, 0, 20, 1),
            "rejoin must not refill the player's admission bucket"
        );
        let (source_duplicate, client_duplicate) = spawn_hello_peers(&mut app, 4);
        deliver_hello(&mut app, source_duplicate, client_duplicate, good(7));
        assert!(app.world().get::<Session>(client_duplicate).is_none());
        assert!(app.world().get::<PendingKick>(client_duplicate).is_some());
        assert!(app.world().get::<Session>(client_b).is_some());
    }

    #[test]
    fn reload_while_the_old_session_lives_is_refused() {
        let mut slots = Slots::new(2);
        slots.claim(7).unwrap();

        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots, true),
            Err("This game is open in another window"),
        );
    }

    #[test]
    fn two_hellos_for_one_player_in_the_same_receive_batch_reject_the_second() {
        let mut slots = Slots::new(2);
        let mut accepted_in_batch = BTreeSet::new();
        assert_eq!(
            judge_hello_in_batch(
                &good(7),
                &advertised(),
                &mut slots,
                false,
                &mut accepted_in_batch,
            ),
            Ok(turts_sim::SlotId(0)),
        );
        assert_eq!(
            judge_hello_in_batch(
                &good(7),
                &advertised(),
                &mut slots,
                false,
                &mut accepted_in_batch,
            ),
            Err("This game is open in another window"),
        );
        assert_eq!(slots.slot_of(7), Some(turts_sim::SlotId(0)));
        assert_eq!(slots.owner_of(turts_sim::SlotId(1)), None);
    }

    #[test]
    fn receive_hello_rejects_two_live_connections_in_one_frame() {
        use bevy::time::TimeUpdateStrategy;
        use lightyear::link::recv_payload_from_bytes;
        use lightyear::prelude::server::ServerPlugins;

        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.add_plugins(turts_sim::SimPlugin { tick_hz: 20 });
        app.insert_resource(turts_sim::MatchConfig::immediate(20));
        app.init_resource::<crate::stats::Stats>();
        app.insert_resource(crate::stats::MatchId(0));
        install(&mut app, advertised());
        let (source_a, client_a) = spawn_hello_peers(&mut app, 1);
        let (source_b, client_b) = spawn_hello_peers(&mut app, 2);
        app.finish();
        app.cleanup();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();

        for source in [source_a, source_b] {
            app.world_mut()
                .get_mut::<MessageSender<Hello>>(source)
                .unwrap()
                .send::<ReliableChannel>(good(7));
        }
        app.update();
        for (source, client) in [(source_a, client_a), (source_b, client_b)] {
            let packets = {
                let mut entity = app.world_mut().entity_mut(source);
                let mut link = entity.get_mut::<Link>().unwrap();
                let mut packets = Vec::new();
                while let Some(packet) = link.send.pop() {
                    packets.push(packet);
                }
                packets
            };
            assert!(!packets.is_empty(), "Hello was serialized for {source:?}");
            for packet in packets {
                app.world_mut()
                    .entity_mut(client)
                    .get_mut::<Link>()
                    .unwrap()
                    .recv
                    .push_raw(recv_payload_from_bytes(packet));
            }
        }
        app.update();

        let accepted = [client_a, client_b]
            .into_iter()
            .filter(|client| app.world().get::<Session>(*client).is_some())
            .count();
        let refused = [client_a, client_b]
            .into_iter()
            .filter(|client| app.world().get::<PendingKick>(*client).is_some())
            .count();
        assert_eq!(accepted, 1, "exactly one live session keeps the slot");
        assert_eq!(refused, 1, "the other connection is rejected");
        assert_eq!(app.world().resource::<crate::stats::Stats>().hellos, 2);
        assert_eq!(
            app.world().resource::<Slots>().slot_of(7),
            Some(turts_sim::SlotId(0))
        );
    }

    #[test]
    fn rejoin_reuses_slot_without_writing_a_second_economy_claim() {
        let mut app = hello_test_app();
        let path = std::env::temp_dir().join(format!(
            "turts_hello_rejoin_{}_{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&path);
        app.world_mut()
            .resource_mut::<turts_sim::AuditTrail>()
            .enable();
        // A start tick that is not zero, so a Claim with a fixed or default tick is red.
        app.world_mut().resource_mut::<turts_sim::Tick>().0 = 3;
        let initial = turts_sim::audit_state(app.world_mut());
        let trail = app.world().resource::<turts_sim::AuditTrail>();
        app.insert_resource(
            crate::economylog::EconomyLogSink::create(&path, trail, initial).unwrap(),
        );

        // The test clock does not move (`TimeUpdateStrategy::ManualDuration(ZERO)` in
        // `hello_test_app`), so the tick of the Claim is known with no race: it is the sim tick
        // before the first Hello, and the test checks that the tick did not move.
        let hello_tick = app.world().resource::<turts_sim::Tick>().0;
        let (source_a, client_a) = spawn_hello_peers(&mut app, 1);
        deliver_hello(&mut app, source_a, client_a, good(7));
        assert_eq!(
            app.world().get::<Session>(client_a).unwrap().slot,
            turts_sim::SlotId(0)
        );
        app.world_mut().despawn(client_a);

        let (source_b, client_b) = spawn_hello_peers(&mut app, 2);
        deliver_hello(&mut app, source_b, client_b, good(7));
        assert_eq!(
            app.world().get::<Session>(client_b).unwrap().slot,
            turts_sim::SlotId(0)
        );

        let records = turts_audit::read_jsonl_file(&path).unwrap();
        let claims = records
            .iter()
            .filter(|record| matches!(record, turts_audit::EconomyLogRecord::Claim { .. }))
            .count();
        assert_eq!(claims, 1, "a rejoin must not emit a second Claim record");
        assert_eq!(
            app.world().resource::<turts_sim::Tick>().0,
            hello_tick,
            "the sim tick did not move during the test"
        );
        let claim = records.iter().find_map(|record| match record {
            turts_audit::EconomyLogRecord::Claim {
                tick,
                player_id,
                slot,
            } => Some((*tick, *player_id, *slot)),
            _ => None,
        });
        assert_eq!(
            claim,
            Some((hello_tick, 7, 0)),
            "the one Claim record names the tick of the Hello, player 7 and the slot that its \
             session holds"
        );
        turts_audit::validate_records(&records).unwrap();
        drop(app);
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
/// The admission systems of the server run in `FixedPreUpdate` between the tick advance and
/// the commands. The sets `Supply` and `Construction` sit in that gap (ruling of the set
/// order), so the admission systems must be ordered against them: an exclusive system
/// that is not ordered against another system that touches a resource is an ambiguity, and
/// the ambiguity check of the schedule makes it an error.
pub(crate) fn assert_fixed_pre_update_is_totally_ordered(mut app: App) {
    use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
    // Supply and Construction now have real exclusive systems. They detect missing
    // admission edges directly. A synthetic probe in either set would instead add
    // an unrelated conflict between two systems within the same phase.
    app.edit_schedule(FixedPreUpdate, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Error,
            ..Default::default()
        });
    });
    app.world_mut().run_schedule(FixedPreUpdate);
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use bevy::time::{TimePlugin, TimeUpdateStrategy};
    use std::collections::BTreeMap;
    use std::time::Duration;
    use turts_mod_api::content::{ResolvedProducer, ResolvedResearchStation, ResolvedUnit};
    use turts_mod_api::{
        ContentCatalog, ContentId, Footprint, PackageId, ProducerDef, ResearchStationDef, UnitDef,
        UnitRegistry,
    };
    use turts_sim::{
        CommandKind, CommandResults, Owner, Tick, UnitId as SimUnitId, UnitType, UnitTypes,
    };

    fn app() -> (App, Entity) {
        app_at_hz(20)
    }

    fn app_at_hz(tick_hz: u16) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(Advertised {
            lockfile_hash: String::new(),
            tick_hz,
        });
        app.add_plugins(TimePlugin)
            .add_plugins(turts_sim::SimPlugin { tick_hz })
            .insert_resource(turts_sim::MatchConfig::immediate(tick_hz))
            .add_systems(
                FixedPreUpdate,
                admit_player_commands
                    .after(SimSet::Construction)
                    .before(SimSet::Commands),
            );
        app.init_resource::<StagedPlayerCommands>()
            .init_resource::<PendingOrderCommands>()
            .init_resource::<CommandAdmissionDiagnostics>()
            .init_resource::<ReliableTraffic>()
            .init_resource::<RateLimitBuckets>();
        let package = PackageId::parse("test").unwrap();
        let mut types = UnitTypes::default();
        let type_id = types
            .register_unit_type(
                &package,
                UnitDef {
                    vision: None,
                    id: "producer".into(),
                    name: "Producer".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: Some(ProducerDef {
                        recipes: vec![],
                        queue_capacity: 1,
                        spawn_offsets: vec![],
                        when_short: Vec::new(),
                    }),
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                    armour: None,
                    acquire: None,
                    provides: Vec::new(),
                    consumes: Vec::new(),
                    income: None,
                    spawn_order: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    repairer: None,
                },
            )
            .unwrap();
        let id = ContentId::new(&package, "producer");
        let mut catalog = ContentCatalog {
            combat: None,
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
            bots: Default::default(),
        };
        catalog.units.insert(
            id.clone(),
            ResolvedUnit {
                vision: None,
                repairer: None,

                consumes: Vec::new(),
                income: None,
                provides: Vec::new(),

                acquire: None,
                armour: None,
                constructor: None,
                placement: None,
                build_area: None,
                id,
                name: "Producer".into(),
                footprint: Footprint::Circle { radius: 1.0 },
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: Some(ResolvedProducer {
                    when_short: Vec::new(),

                    recipes: vec![],
                    queue_capacity: 1,
                    spawn_offsets: vec![],
                }),
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        );
        app.world_mut().insert_resource(types);
        app.world_mut()
            .insert_resource(turts_sim::bootstrap::InstalledCatalog(catalog));
        app.world_mut()
            .insert_resource(crate::orders::ServerCatalog(test_order_catalog()));
        let connection = app
            .world_mut()
            .spawn(Session {
                slot: turts_sim::SlotId(0),
                player_id: 1,
                last_seq: None,
            })
            .id();
        app.world_mut()
            .spawn((SimUnitId(1), Owner(turts_sim::SlotId(0)), UnitType(type_id)));
        (app, connection)
    }
    #[derive(Clone)]
    enum FixtureOrder {
        MoveTo {
            unit_ids: Vec<UnitId>,
            target: Pos,
        },
        Produce {
            producer_id: UnitId,
            recipe_id: String,
        },
        CancelProduction {
            producer_id: UnitId,
            job_id: u64,
        },
        Remove {
            unit_id: UnitId,
        },
        SetEnabled {
            unit_id: UnitId,
            enabled: bool,
        },
    }

    fn test_order_catalog() -> ContentCatalog {
        use turts_mod_api::{Behaviour, ContentId, OrderDef, TargetKind};
        let mut catalog = ContentCatalog {
            combat: None,
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
            bots: Default::default(),
        };
        for (name, behaviour, target) in [
            ("move", Behaviour::Move, TargetKind::Point),
            ("gather", Behaviour::Gather, TargetKind::Node),
            ("produce", Behaviour::Produce, TargetKind::Content),
            ("cancel", Behaviour::CancelProduction, TargetKind::Job),
            ("remove", Behaviour::Remove, TargetKind::None),
            ("enabled", Behaviour::SetEnabled(false), TargetKind::None),
            ("enabled-on", Behaviour::SetEnabled(true), TargetKind::None),
        ] {
            let id = ContentId::parse(&format!("test:{name}")).unwrap();
            catalog.orders.insert(
                id.clone(),
                OrderDef {
                    id: id.local.clone(),
                    name: name.into(),
                    behaviour,
                    target,
                    fire: None,
                    engage: None,
                    icon: String::new(),
                    hotkey: String::new(),
                    cursor: None,
                    card: None,
                    units: Vec::new(),
                    description: String::new(),
                },
            );
        }
        catalog
    }

    fn staged(connection: Entity, seq: u32, kind: FixtureOrder) -> StagedPlayerCommand {
        let (order, units, target) = match kind {
            FixtureOrder::MoveTo { unit_ids, target } => {
                ("test:move", unit_ids, OrderTarget::Point(target))
            }
            FixtureOrder::Produce {
                producer_id,
                recipe_id,
            } => (
                "test:produce",
                vec![producer_id],
                OrderTarget::Content(recipe_id),
            ),
            FixtureOrder::CancelProduction {
                producer_id,
                job_id,
            } => (
                "test:cancel",
                vec![producer_id],
                OrderTarget::Job(turts_net_protocol::economy::JobId(job_id)),
            ),
            FixtureOrder::Remove { unit_id } => ("test:remove", vec![unit_id], OrderTarget::None),
            FixtureOrder::SetEnabled { unit_id, enabled } => {
                let order = if enabled {
                    "test:enabled-on"
                } else {
                    "test:enabled"
                };
                (order, vec![unit_id], OrderTarget::None)
            }
        };
        StagedPlayerCommand {
            connection,
            slot: turts_sim::SlotId(0),
            envelope: PlayerCommand {
                seq,
                order: OrderEnvelope {
                    order: order.into(),
                    units,
                    target,
                    queue: QueueMode::Replace,
                    group: 0,
                },
            },
        }
    }
    fn move_kind() -> FixtureOrder {
        FixtureOrder::MoveTo {
            unit_ids: vec![UnitId(1)],
            target: Pos { x: 4.0, y: 5.0 },
        }
    }

    fn move_for(connection: Entity, seq: u32, unit_ids: Vec<UnitId>) -> StagedPlayerCommand {
        staged(
            connection,
            seq,
            FixtureOrder::MoveTo {
                unit_ids,
                target: Pos { x: 4.0, y: 5.0 },
            },
        )
    }

    #[test]
    fn bucket_refuses_the_65th_envelope_in_one_burst() {
        let (mut app, connection) = app();
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend((1..=65).map(|seq| move_for(connection, seq, vec![UnitId(1)])));

        admit_player_commands(app.world_mut());

        let records = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(records.len(), 1, "only the 65th envelope is refused");
        assert_eq!(records[0].seq, 65);
        assert_eq!(records[0].reason, CommandRejectReason::RateLimited);
        assert_eq!(records[0].envelope.order.units, vec![UnitId(1)]);
    }

    /// The bucket runs first for every envelope (design 6.8, [net 5]): an envelope that
    /// `BadSequence` refuses still costs a token. 1 envelope is accepted and 63 repeat its
    /// sequence, so the 64 tokens are spent and a valid 65th envelope is `RateLimited`. Before
    /// protocol 5 the 63 refusals cost nothing and the 65th envelope was accepted.
    #[test]
    fn a_refused_envelope_costs_a_bucket_token() {
        let (mut app, connection) = app();
        let mut staged_commands: Vec<_> = (0..64)
            .map(|_| move_for(connection, 1, vec![UnitId(1)]))
            .collect();
        staged_commands.push(move_for(connection, 2, vec![UnitId(1)]));
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend(staged_commands);

        admit_player_commands(app.world_mut());

        let records = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(records.len(), 64, "63 bad sequences and the 65th envelope");
        assert!(
            records[..63]
                .iter()
                .all(|record| record.reason == CommandRejectReason::BadSequence)
        );
        assert_eq!(records[63].seq, 2);
        assert_eq!(records[63].reason, CommandRejectReason::RateLimited);
        assert_eq!(
            app.world().get::<Session>(connection).unwrap().last_seq,
            Some(2),
            "the sequence of a rate-limited envelope still counts, as before"
        );
    }

    #[test]
    fn bucket_refills_after_one_second_of_ticks() {
        let (mut app, connection) = app();
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend((1..=65).map(|seq| move_for(connection, seq, vec![UnitId(1)])));
        admit_player_commands(app.world_mut());

        app.world_mut().resource_mut::<SimTick>().0 = 20;
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .push_back(move_for(connection, 66, vec![UnitId(1)]));
        admit_player_commands(app.world_mut());

        let records = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(records.len(), 1, "refill admits the next envelope");
        assert_eq!(records[0].seq, 65);
        assert_eq!(records[0].reason, CommandRejectReason::RateLimited);
        assert_eq!(
            app.world().resource::<PendingOrderCommands>().0.len(),
            65,
            "64 pre-refill envelopes and sequence 66 are admitted",
        );
    }

    #[test]
    fn envelope_bucket_refills_32_tokens_per_tick_at_twenty_hz() {
        let (mut app, connection) = app();
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend((1..=64).map(|seq| move_for(connection, seq, vec![UnitId(1)])));
        admit_player_commands(app.world_mut());

        app.world_mut().resource_mut::<SimTick>().0 = 1;
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend((65..=97).map(|seq| move_for(connection, seq, vec![UnitId(1)])));
        admit_player_commands(app.world_mut());

        assert_eq!(app.world().resource::<PendingOrderCommands>().0.len(), 96);
        let refused = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].seq, 97);
        assert_eq!(refused[0].reason, CommandRejectReason::RateLimited);
    }

    #[test]
    fn unit_bucket_refuses_above_16384() {
        let (mut app, connection) = app();
        let mut first = 1_u32;
        for (seq, size) in [(1, 4096), (2, 4096), (3, 4096), (4, 4096), (5, 1)] {
            let units = (first..first + size).map(UnitId).collect();
            app.world_mut()
                .resource_mut::<StagedPlayerCommands>()
                .0
                .push_back(move_for(connection, seq, units));
            first += size;
        }

        admit_player_commands(app.world_mut());

        let records = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(records.len(), 1, "the 16,385th unit order is refused");
        assert_eq!(records[0].seq, 5);
        assert_eq!(records[0].reason, CommandRejectReason::RateLimited);
        assert_eq!(records[0].envelope.order.units, vec![UnitId(16_385)]);
    }

    #[test]
    fn unit_bucket_refills_8192_tokens_per_tick_at_twenty_hz() {
        let (mut app, connection) = app();
        for (seq, start) in [(1, 1_u32), (2, 4097), (3, 8193), (4, 12_289)] {
            app.world_mut()
                .resource_mut::<StagedPlayerCommands>()
                .0
                .push_back(move_for(
                    connection,
                    seq,
                    (start..start + 4096).map(UnitId).collect(),
                ));
        }
        admit_player_commands(app.world_mut());

        app.world_mut().resource_mut::<SimTick>().0 = 1;
        for (seq, start) in [(5, 16_385_u32), (6, 20_481), (7, 24_577)] {
            app.world_mut()
                .resource_mut::<StagedPlayerCommands>()
                .0
                .push_back(move_for(
                    connection,
                    seq,
                    (start..start + 4096).map(UnitId).collect(),
                ));
        }
        admit_player_commands(app.world_mut());

        let admitted_units: usize = app
            .world()
            .resource::<PendingOrderCommands>()
            .0
            .values()
            .map(|command| command.order.units.len())
            .sum();
        assert_eq!(admitted_units, 24_576);
        let refused = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].seq, 7);
        assert_eq!(refused[0].reason, CommandRejectReason::RateLimited);
    }

    #[test]
    fn envelope_bucket_keeps_fractional_refill_tokens_at_thirty_hz() {
        let (mut app, connection) = app_at_hz(30);
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend((1..=64).map(|seq| move_for(connection, seq, vec![UnitId(1)])));
        admit_player_commands(app.world_mut());
        let mut next_seq = 65;
        // 640 scaled tokens per tick / 30 per envelope: 21, 21, then 22.
        for (tick, expected_total_admitted, expected_total_refused) in
            [(1, 85, 2), (2, 106, 4), (3, 128, 5)]
        {
            app.world_mut().resource_mut::<SimTick>().0 = tick;
            for _ in 0..23 {
                app.world_mut()
                    .resource_mut::<StagedPlayerCommands>()
                    .0
                    .push_back(move_for(connection, next_seq, vec![UnitId(1)]));
                next_seq += 1;
            }
            admit_player_commands(app.world_mut());
            assert_eq!(
                app.world().resource::<PendingOrderCommands>().0.len(),
                expected_total_admitted,
                "whole scaled tokens must retain the fractional refill at tick {tick}",
            );
            let refused = &app.world().resource::<CommandAdmissionDiagnostics>().0;
            assert_eq!(refused.len(), expected_total_refused, "tick {tick}");
            assert!(
                refused
                    .iter()
                    .all(|record| record.reason == CommandRejectReason::RateLimited)
            );
        }
    }

    #[test]
    fn unit_bucket_keeps_fractional_refill_tokens_at_thirty_hz() {
        let (mut app, connection) = app_at_hz(30);
        let mut next_seq = 1;
        for _ in 0..4 {
            app.world_mut()
                .resource_mut::<StagedPlayerCommands>()
                .0
                .push_back(move_for(
                    connection,
                    next_seq,
                    (1..=4096).map(UnitId).collect(),
                ));
            next_seq += 1;
        }
        admit_player_commands(app.world_mut());
        // 163840 scaled tokens per tick / 30: 5461, 5461, then 5462 units.
        for (tick, tail, expected_total_units) in
            [(1, 1365_u32, 21845), (2, 1365, 27306), (3, 1366, 32768)]
        {
            app.world_mut().resource_mut::<SimTick>().0 = tick;
            for count in [4096, tail, 1] {
                app.world_mut()
                    .resource_mut::<StagedPlayerCommands>()
                    .0
                    .push_back(move_for(
                        connection,
                        next_seq,
                        (1..=count).map(UnitId).collect(),
                    ));
                next_seq += 1;
            }
            admit_player_commands(app.world_mut());
            let admitted_units: usize = app
                .world()
                .resource::<PendingOrderCommands>()
                .0
                .values()
                .map(|command| command.order.units.len())
                .sum();
            assert_eq!(admitted_units, expected_total_units, "tick {tick}");
            let refused = &app.world().resource::<CommandAdmissionDiagnostics>().0;
            assert_eq!(
                refused.len(),
                tick as usize,
                "exactly one excess unit per tick"
            );
            assert!(
                refused
                    .iter()
                    .all(|record| record.reason == CommandRejectReason::RateLimited)
            );
        }
    }

    #[test]
    fn unit_bucket_refusal_does_not_spend_envelope_token() {
        let mut buckets = RateLimitBuckets::default();
        let slot = 0;
        for _ in 0..4 {
            assert!(buckets.admit(slot, 0, 20, 4096));
        }
        assert!(!buckets.admit(slot, 0, 20, 1));
        let bucket = buckets.0.get(&slot).unwrap();
        assert_eq!(bucket.envelope_tokens, 60 * 20);
        assert_eq!(bucket.unit_tokens, 0);
    }

    #[test]
    fn bucket_capacity_is_shared_across_connections_rejoining_the_same_slot() {
        let (mut app, first_connection) = app();
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend((1..=64).map(|seq| move_for(first_connection, seq, vec![UnitId(1)])));
        admit_player_commands(app.world_mut());

        let rejoined_connection = app
            .world_mut()
            .spawn(Session {
                last_seq: None,
                slot: turts_sim::SlotId(0),
                player_id: 1,
            })
            .id();
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .push_back(move_for(rejoined_connection, 1, vec![UnitId(1)]));
        admit_player_commands(app.world_mut());

        let records = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].reason, CommandRejectReason::RateLimited);
    }

    fn admit_kind(
        world: &mut World,
        staged: &StagedPlayerCommand,
        _tick: u32,
    ) -> Option<CommandKind> {
        crate::orders::to_sim(
            staged.slot,
            &staged.envelope,
            &world.resource::<crate::orders::ServerCatalog>().0,
        )
        .ok()?
        .into_iter()
        .next()
        .map(|command| command.kind)
    }

    #[test]
    fn server_admits_produce_for_research_unit() {
        let (mut app, connection) = app();
        let package = PackageId::parse("test").unwrap();
        let research_type = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &package,
                UnitDef {
                    vision: None,
                    id: "lab".into(),
                    name: "Lab".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: Some(ResearchStationDef {
                        technologies: vec!["test:technology".into()],
                        queue_capacity: 1,
                    }),
                    health: None,
                    weapon: None,
                    removal: None,
                    armour: None,
                    acquire: None,
                    provides: Vec::new(),
                    consumes: Vec::new(),
                    income: None,
                    spawn_order: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    repairer: None,
                },
            )
            .unwrap();
        let id = ContentId::new(&package, "lab");
        app.world_mut()
            .resource_mut::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .units
            .insert(
                id.clone(),
                ResolvedUnit {
                    vision: None,
                    repairer: None,

                    consumes: Vec::new(),
                    income: None,
                    provides: Vec::new(),

                    acquire: None,
                    armour: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    id,
                    name: "Lab".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: Some(ResolvedResearchStation {
                        technologies: vec![ContentId::new(&package, "technology")],
                        queue_capacity: 1,
                    }),
                    health: None,
                    weapon: None,
                    removal: None,
                },
            );
        app.world_mut().spawn((
            SimUnitId(2),
            Owner(turts_sim::SlotId(0)),
            UnitType(research_type),
        ));

        let accepted = admit_kind(
            app.world_mut(),
            &staged(
                connection,
                1,
                FixtureOrder::Produce {
                    producer_id: UnitId(2),
                    recipe_id: "test:technology".into(),
                },
            ),
            3,
        );
        assert_eq!(
            accepted,
            Some(CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: ContentId::new(&package, "produce"),
                behaviour: Behaviour::Produce,
                target_kind: turts_mod_api::TargetKind::Content,
                policy: Some(turts_sim::OrderPolicyAudit {
                    id: ContentId::new(&package, "produce"),
                    behaviour: Behaviour::Produce,
                    target: turts_mod_api::TargetKind::Content,
                    fire: None,
                    engage: None,
                    units: vec![],
                    queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
                    unit_capabilities: vec![],
                }),
                units: vec![2],
                target: turts_sim::OrderTargetSim::Content(ContentId::new(&package, "technology")),
                queue: turts_sim::QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
            })))
        );
        assert!(
            app.world()
                .resource::<CommandAdmissionDiagnostics>()
                .0
                .is_empty()
        );
    }

    #[test]
    fn the_production_admission_system_is_ordered_against_every_set_before_the_commands() {
        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_plugins(turts_sim::SimPlugin { tick_hz: 20 })
            .insert_resource(turts_sim::MatchConfig::immediate(20));
        install(
            &mut app,
            Advertised {
                lockfile_hash: String::new(),
                tick_hz: 20,
            },
        );
        super::assert_fixed_pre_update_is_totally_ordered(app);
    }

    #[test]
    fn production_receive_survives_lightyear_last_without_a_fixed_tick() {
        use lightyear::link::recv_payload_from_bytes;
        use lightyear::prelude::server::ServerPlugins;

        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.add_plugins(turts_sim::SimPlugin { tick_hz: 20 });
        app.insert_resource(turts_sim::MatchConfig::immediate(20));
        app.init_resource::<crate::stats::Stats>();
        app.insert_resource(crate::stats::MatchId(0));
        app.insert_resource(crate::orders::ServerCatalog(test_order_catalog()));
        install(
            &mut app,
            Advertised {
                lockfile_hash: "sha256:advertised".into(),
                tick_hz: 20,
            },
        );

        let package = PackageId::parse("test").unwrap();
        let mut types = UnitTypes::default();
        let type_id = types
            .register_unit_type(
                &package,
                UnitDef {
                    vision: None,
                    id: "producer".into(),
                    name: "Producer".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: Some(ProducerDef {
                        recipes: vec![],
                        queue_capacity: 1,
                        spawn_offsets: vec![],
                        when_short: Vec::new(),
                    }),
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                    armour: None,
                    acquire: None,
                    provides: Vec::new(),
                    consumes: Vec::new(),
                    income: None,
                    spawn_order: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    repairer: None,
                },
            )
            .unwrap();
        let id = ContentId::new(&package, "producer");
        let mut catalog = ContentCatalog {
            combat: None,
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
            bots: Default::default(),
        };
        catalog.units.insert(
            id.clone(),
            ResolvedUnit {
                vision: None,
                repairer: None,

                consumes: Vec::new(),
                income: None,
                provides: Vec::new(),

                acquire: None,
                armour: None,
                constructor: None,
                placement: None,
                build_area: None,
                id,
                name: "Producer".into(),
                footprint: Footprint::Circle { radius: 1.0 },
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: Some(ResolvedProducer {
                    when_short: Vec::new(),

                    recipes: vec![],
                    queue_capacity: 1,
                    spawn_offsets: vec![],
                }),
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        );
        app.world_mut().insert_resource(types);
        app.world_mut()
            .insert_resource(turts_sim::bootstrap::InstalledCatalog(catalog));
        let connection = app
            .world_mut()
            .spawn(Session {
                slot: turts_sim::SlotId(0),
                player_id: 1,
                last_seq: None,
            })
            .id();
        app.world_mut()
            .spawn((SimUnitId(1), Owner(turts_sim::SlotId(0)), UnitType(type_id)));

        app.finish();
        app.cleanup();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update(); // initialize Bevy time with a real zero-fixed frame

        let registry = app.world().resource::<ChannelRegistry>();
        let mut source_transport = Transport::default();
        source_transport.add_channel_send_from_registry::<ReliableChannel>(registry);
        source_transport.add_channel_receive_from_registry::<ReliableChannel>(registry);
        let source = app
            .world_mut()
            .spawn((
                Client,
                Link::default(),
                source_transport,
                MessageSender::<PlayerCommand>::default(),
                RemoteId(PeerId::Server),
                Linked,
                Connected,
            ))
            .id();
        app.world_mut().entity_mut(connection).insert((
            ClientOf,
            Link::default(),
            Transport::default(),
            RemoteId(PeerId::Local(1)),
            Linked,
            Connected,
        ));
        let no_session = app
            .world_mut()
            .spawn((
                ClientOf,
                Link::default(),
                Transport::default(),
                RemoteId(PeerId::Local(2)),
                Linked,
                Connected,
            ))
            .id();

        let commands = [
            staged(connection, 1, move_kind()).envelope,
            staged(
                connection,
                2,
                FixtureOrder::Produce {
                    producer_id: UnitId(1),
                    recipe_id: "BAD:??".into(),
                },
            )
            .envelope,
            staged(connection, 2, move_kind()).envelope,
        ];
        {
            let mut sender = app
                .world_mut()
                .get_mut::<MessageSender<PlayerCommand>>(source)
                .unwrap();
            for command in commands.clone() {
                sender.send::<ReliableChannel>(command);
            }
        }
        app.update(); // serialize through real MessageSystems::Send
        let packets = {
            let mut source_entity = app.world_mut().entity_mut(source);
            let mut link = source_entity.get_mut::<Link>().unwrap();
            let mut packets = Vec::new();
            while let Some(packet) = link.send.pop() {
                packets.push(packet);
            }
            packets
        };
        assert!(
            !packets.is_empty(),
            "the public Link send path produced no packet"
        );
        for packet in packets {
            app.world_mut()
                .entity_mut(connection)
                .get_mut::<Link>()
                .unwrap()
                .recv
                .push_raw(recv_payload_from_bytes(packet.clone()));
            app.world_mut()
                .entity_mut(no_session)
                .get_mut::<Link>()
                .unwrap()
                .recv
                .push_raw(recv_payload_from_bytes(packet));
        }
        app.update(); // production PreUpdate receive, real Lightyear Last cleanup, zero fixed ticks

        let staged = &app.world().resource::<StagedPlayerCommands>().0;
        assert_eq!(
            staged
                .iter()
                .map(|item| item.envelope.seq)
                .collect::<Vec<_>>(),
            [1, 2, 2]
        );
        assert!(staged.iter().all(|item| item.slot == turts_sim::SlotId(0)));
        assert!(
            !app.world()
                .get::<MessageReceiver<PlayerCommand>>(connection)
                .unwrap()
                .has_messages(),
            "Last cleared the actual receiver"
        );
        assert_eq!(
            app.world().get::<Session>(connection).unwrap().last_seq,
            None
        );
        assert_eq!(app.world().resource::<Tick>().0, 0);
        assert!(
            !app.world()
                .get::<MessageReceiver<PlayerCommand>>(no_session)
                .unwrap()
                .has_messages()
        );
        assert!(
            !app.world()
                .resource::<StagedPlayerCommands>()
                .0
                .iter()
                .any(|item| item.connection == no_session)
        );
        assert!(
            !app.world()
                .resource::<ReliableTraffic>()
                .counts
                .contains_key(&no_session.to_bits())
        );
        assert!(
            app.world()
                .resource::<CommandAdmissionDiagnostics>()
                .0
                .is_empty()
        );
        assert!(app.world().resource::<CommandResults>().0.is_empty());
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        assert_eq!(
            app.world()
                .resource::<ReliableTraffic>()
                .counts
                .get(&connection.to_bits()),
            Some(&6)
        );
        let ack_payloads = {
            let mut destination_entity = app.world_mut().entity_mut(connection);
            let mut link = destination_entity.get_mut::<Link>().unwrap();
            let mut count = 0;
            while link.send.pop().is_some() {
                count += 1;
            }
            count
        };
        assert!(
            ack_payloads > 0,
            "production receipt Acks were not serialized to Link.send"
        );

        app.update(); // second zero-fixed frame: no duplicate receipt or admission
        assert_eq!(app.world().resource::<StagedPlayerCommands>().0.len(), 3);
        assert_eq!(
            app.world()
                .resource::<ReliableTraffic>()
                .counts
                .get(&connection.to_bits()),
            Some(&6)
        );
        let before_audit = {
            let audit = app.world().resource::<turts_sim::AuditTrail>();
            (
                audit.commands.len(),
                audit.operations.len(),
                audit.ticks.len(),
            )
        };
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        assert_eq!(app.world().resource::<Tick>().0, 1);
        assert!(app.world().resource::<StagedPlayerCommands>().0.is_empty());
        assert_eq!(
            app.world().get::<Session>(connection).unwrap().last_seq,
            Some(2)
        );
        let diagnostics = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            (
                diagnostics[0].seq,
                diagnostics[0].tick,
                diagnostics[0].reason
            ),
            (2, 1, CommandRejectReason::UnknownRecipe)
        );
        assert_eq!(
            (
                diagnostics[1].seq,
                diagnostics[1].tick,
                diagnostics[1].reason
            ),
            (2, 1, CommandRejectReason::BadSequence)
        );
        let results = &app.world().resource::<CommandResults>().0;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].seq, 1);
        assert!(matches!(
            results[0].outcome,
            Err(turts_sim::RejectReason::NotApplicable)
        ));
        assert_eq!(
            results[0].units.refused,
            vec![(1, turts_sim::RejectReason::NotApplicable)]
        );
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        let audit = app.world().resource::<turts_sim::AuditTrail>();
        assert_eq!(
            (
                audit.commands.len(),
                audit.operations.len(),
                audit.ticks.len()
            ),
            before_audit
        );
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        assert_eq!(
            app.world()
                .resource::<CommandAdmissionDiagnostics>()
                .0
                .len(),
            2
        );
        assert_eq!(app.world().resource::<CommandResults>().0.len(), 1);
    }

    #[test]
    fn repeated_sequence_rejects_without_second_domain_command_or_audit() {
        let (mut app, connection) = app();
        app.world_mut().resource_mut::<Tick>().0 = 6;
        let before_audit = {
            let audit = app.world().resource::<turts_sim::AuditTrail>();
            (
                audit.commands.len(),
                audit.operations.len(),
                audit.ticks.len(),
            )
        };
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .extend([
                staged(connection, 9, move_kind()),
                staged(connection, 9, move_kind()),
            ]);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        assert_eq!(app.world().resource::<Tick>().0, 7);
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        assert_eq!(app.world().resource::<CommandResults>().0.len(), 1);
        assert_eq!(
            app.world().resource::<CommandAdmissionDiagnostics>().0[0].reason,
            CommandRejectReason::BadSequence
        );
        assert_eq!(
            app.world().resource::<CommandAdmissionDiagnostics>().0[0].tick,
            7
        );
        assert_eq!(
            app.world().get::<Session>(connection).unwrap().last_seq,
            Some(9)
        );
        let audit = app.world().resource::<turts_sim::AuditTrail>();
        assert_eq!(
            (
                audit.commands.len(),
                audit.operations.len(),
                audit.ticks.len()
            ),
            before_audit,
            "the rejected replay added no simulation audit"
        );
    }

    #[test]
    fn unknown_reference_is_rejected_without_foreign_details() {
        let (mut app, connection) = app();
        let envelope = staged(
            connection,
            3,
            FixtureOrder::Produce {
                producer_id: UnitId(1),
                recipe_id: "FOREIGN:bad:recipe".into(),
            },
        );
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .push_back(envelope.clone());
        app.world_mut().resource_mut::<Tick>().0 = 10;
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        let records = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(records[0].reason, CommandRejectReason::UnknownRecipe);
        assert_eq!(records[0].seq, envelope.envelope.seq);
        assert_eq!(records[0].reason.as_str(), "unknown_recipe");
        assert!(!records[0].reason.as_str().contains("FOREIGN"));
        assert_eq!(records[0].envelope.order, envelope.envelope.order);
        assert_eq!(records[0].tick, 11);
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        assert_eq!(
            app.world().get::<Session>(connection).unwrap().last_seq,
            Some(3)
        );
    }

    #[test]
    fn ownership_precedes_malformed_recipe_details() {
        let (mut app, connection) = app();
        let producer_type = app
            .world()
            .iter_entities()
            .find_map(|entity| entity.get::<UnitType>().copied())
            .unwrap();
        app.world_mut()
            .spawn((SimUnitId(2), Owner(turts_sim::SlotId(1)), producer_type));
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .push_back(staged(
                connection,
                4,
                FixtureOrder::Produce {
                    producer_id: UnitId(2),
                    recipe_id: "BAD:??".into(),
                },
            ));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        assert_eq!(
            app.world().resource::<CommandAdmissionDiagnostics>().0[0].reason,
            CommandRejectReason::NotOwner
        );
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        assert!(app.world().resource::<CommandResults>().0.is_empty());
    }

    #[test]
    fn representable_mixed_produce_envelope_reaches_per_unit_validation() {
        let (mut app, connection) = app();
        let initial = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<SimUnitId>().is_some_and(|unit| unit.0 == 1))
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(initial)
            .insert(turts_sim::Position(Vec2::new(4.0, 4.0)));
        let producer_type = app
            .world()
            .iter_entities()
            .find_map(|entity| entity.get::<UnitType>().copied())
            .unwrap();
        app.world_mut().spawn((
            SimUnitId(2),
            Owner(turts_sim::SlotId(1)),
            producer_type,
            turts_sim::Position(Vec2::new(8.0, 4.0)),
        ));
        app.world_mut().spawn((
            SimUnitId(4),
            Owner(turts_sim::SlotId(0)),
            producer_type,
            turts_sim::Position(Vec2::new(12.0, 4.0)),
        ));
        let static_type = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &PackageId::parse("test").unwrap(),
                UnitDef {
                    vision: None,
                    id: "static".into(),
                    name: "Static".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                    armour: None,
                    acquire: None,
                    provides: Vec::new(),
                    consumes: Vec::new(),
                    income: None,
                    spawn_order: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    repairer: None,
                },
            )
            .unwrap();
        let static_id = ContentId::parse("test:static").unwrap();
        app.world_mut()
            .resource_mut::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .units
            .insert(
                static_id.clone(),
                ResolvedUnit {
                    vision: None,
                    repairer: None,

                    consumes: Vec::new(),
                    income: None,
                    provides: Vec::new(),

                    acquire: None,
                    armour: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    id: static_id,
                    name: "Static".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            );
        app.world_mut().spawn((
            SimUnitId(3),
            Owner(turts_sim::SlotId(0)),
            UnitType(static_type),
        ));
        let recipe = ContentId::parse("test:existing_recipe").unwrap();
        let producer = ContentId::parse("test:producer").unwrap();
        {
            let mut installed = app
                .world_mut()
                .resource_mut::<turts_sim::bootstrap::InstalledCatalog>();
            let unit = installed.0.units.get_mut(&producer).unwrap();
            unit.producer.as_mut().unwrap().recipes = vec![recipe.clone()];
            installed.0.production.insert(
                recipe.clone(),
                turts_mod_api::ResolvedProduction {
                    id: recipe.clone(),
                    name: "Existing recipe".into(),
                    output_unit: producer,
                    duration_ms: 1000,
                    costs: vec![],
                    payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
                    cancel_refunds: turts_mod_api::CancelRefunds {
                        queued: turts_mod_api::RefundRatio {
                            numerator: 0,
                            denominator: 1,
                        },
                        running: turts_mod_api::RefundRatio {
                            numerator: 0,
                            denominator: 1,
                        },
                        output_blocked: turts_mod_api::RefundRatio {
                            numerator: 0,
                            denominator: 1,
                        },
                        destroyed: turts_mod_api::RefundRatio::ZERO,
                    },
                },
            );
        }
        app.world_mut()
            .insert_resource(turts_sim::bootstrap::ProductionDurations(BTreeMap::from([
                (recipe.clone(), 20),
            ])));
        let mut produce_staged = staged(
            connection,
            8,
            FixtureOrder::Produce {
                producer_id: UnitId(1),
                recipe_id: recipe.to_string(),
            },
        );
        produce_staged.envelope.order.units = vec![UnitId(4), UnitId(1), UnitId(2), UnitId(3)];
        assert_eq!(
            production_precheck(
                app.world_mut(),
                produce_staged.slot,
                &produce_staged.envelope
            ),
            None
        );
        let command = crate::orders::to_sim(
            produce_staged.slot,
            &produce_staged.envelope,
            &app.world().resource::<crate::orders::ServerCatalog>().0,
        )
        .unwrap()
        .remove(0);
        let result = turts_sim::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1, 4]);
        assert_eq!(
            result.units.refused,
            vec![
                (2, turts_sim::RejectReason::NotOwner),
                (3, turts_sim::RejectReason::MissingCapability),
            ]
        );
        assert_eq!(
            result.outcome,
            Ok(turts_sim::Applied {
                job_id: Some(turts_sim::JobId(1))
            })
        );
        let queues = turts_sim::economy::views::world_view(app.world_mut())
            .economy
            .queues;
        assert_eq!(
            queues
                .iter()
                .find(|queue| queue.producer == 1)
                .unwrap()
                .jobs[0]
                .id,
            1
        );
        assert!(queues.iter().all(|queue| queue.producer != 2));
        assert_eq!(
            queues
                .iter()
                .find(|queue| queue.producer == 4)
                .unwrap()
                .jobs[0]
                .id,
            2,
            "unit-id order determines the next job id, independent of envelope order"
        );

        let cancel = crate::orders::to_sim(
            turts_sim::SlotId(0),
            &staged(
                connection,
                9,
                FixtureOrder::CancelProduction {
                    producer_id: UnitId(1),
                    job_id: 1,
                },
            )
            .envelope,
            &app.world().resource::<crate::orders::ServerCatalog>().0,
        )
        .unwrap()
        .remove(0);
        let canceled = turts_sim::apply_command(app.world_mut(), &cancel);
        assert_eq!(canceled.units.accepted, vec![1]);
        let queues = turts_sim::economy::views::world_view(app.world_mut())
            .economy
            .queues;
        assert!(
            queues
                .iter()
                .find(|queue| queue.producer == 1)
                .unwrap()
                .jobs
                .is_empty()
        );
        assert_eq!(
            queues
                .iter()
                .find(|queue| queue.producer == 4)
                .unwrap()
                .jobs[0]
                .id,
            2
        );

        let cancel_second = crate::orders::to_sim(
            turts_sim::SlotId(0),
            &staged(
                connection,
                10,
                FixtureOrder::CancelProduction {
                    producer_id: UnitId(4),
                    job_id: 2,
                },
            )
            .envelope,
            &app.world().resource::<crate::orders::ServerCatalog>().0,
        )
        .unwrap()
        .remove(0);
        assert_eq!(
            turts_sim::apply_command(app.world_mut(), &cancel_second)
                .units
                .accepted,
            vec![4]
        );

        let producer_one = app
            .world()
            .iter_entities()
            .find_map(|entity| {
                entity
                    .get::<SimUnitId>()
                    .is_some_and(|id| id.0 == 1)
                    .then_some(entity.id())
            })
            .unwrap();
        app.world_mut()
            .entity_mut(producer_one)
            .insert(Owner(turts_sim::SlotId(1)));
        let mut later_success = staged(
            connection,
            11,
            FixtureOrder::Produce {
                producer_id: UnitId(1),
                recipe_id: recipe.to_string(),
            },
        );
        later_success.envelope.order.units = vec![UnitId(4), UnitId(1)];
        assert_eq!(
            production_precheck(app.world_mut(), later_success.slot, &later_success.envelope),
            None
        );
        let later_success_command = crate::orders::to_sim(
            later_success.slot,
            &later_success.envelope,
            &app.world().resource::<crate::orders::ServerCatalog>().0,
        )
        .unwrap()
        .remove(0);
        let later_success_result =
            turts_sim::apply_command(app.world_mut(), &later_success_command);
        assert_eq!(
            later_success_result.units.accepted,
            vec![4],
            "{later_success_result:?}"
        );
        assert_eq!(
            later_success_result.units.refused,
            vec![(1, turts_sim::RejectReason::NotOwner)]
        );
        assert_eq!(
            later_success_result.outcome,
            Ok(turts_sim::Applied {
                job_id: Some(turts_sim::JobId(3))
            }),
            "the first successful unit in unit-id order supplies the envelope job id"
        );
    }

    #[test]
    fn multi_produce_all_invalid_subjects_keep_per_unit_priority() {
        let (mut app, connection) = app();
        let producer_type = app
            .world()
            .iter_entities()
            .find_map(|entity| entity.get::<UnitType>().copied())
            .unwrap();
        app.world_mut()
            .spawn((SimUnitId(2), Owner(turts_sim::SlotId(1)), producer_type));
        let static_type = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &PackageId::parse("test").unwrap(),
                UnitDef {
                    vision: None,
                    id: "static".into(),
                    name: "Static".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                    armour: None,
                    acquire: None,
                    provides: Vec::new(),
                    consumes: Vec::new(),
                    income: None,
                    spawn_order: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    repairer: None,
                },
            )
            .unwrap();
        let static_id = ContentId::parse("test:static").unwrap();
        app.world_mut()
            .resource_mut::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .units
            .insert(
                static_id.clone(),
                ResolvedUnit {
                    vision: None,
                    repairer: None,

                    consumes: Vec::new(),
                    income: None,
                    provides: Vec::new(),

                    acquire: None,
                    armour: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    id: static_id,
                    name: "Static".into(),
                    footprint: Footprint::Circle { radius: 1.0 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                },
            );
        app.world_mut().spawn((
            SimUnitId(3),
            Owner(turts_sim::SlotId(0)),
            UnitType(static_type),
        ));
        let recipe = ContentId::parse("test:existing_recipe").unwrap();
        let producer = ContentId::parse("test:producer").unwrap();
        {
            let mut installed = app
                .world_mut()
                .resource_mut::<turts_sim::bootstrap::InstalledCatalog>();
            installed
                .0
                .units
                .get_mut(&producer)
                .unwrap()
                .producer
                .as_mut()
                .unwrap()
                .recipes = vec![recipe.clone()];
            installed.0.production.insert(
                recipe.clone(),
                turts_mod_api::ResolvedProduction {
                    id: recipe.clone(),
                    name: "Existing recipe".into(),
                    output_unit: producer,
                    duration_ms: 1000,
                    costs: vec![],
                    payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
                    cancel_refunds: turts_mod_api::CancelRefunds {
                        queued: turts_mod_api::RefundRatio {
                            numerator: 0,
                            denominator: 1,
                        },
                        running: turts_mod_api::RefundRatio {
                            numerator: 0,
                            denominator: 1,
                        },
                        output_blocked: turts_mod_api::RefundRatio {
                            numerator: 0,
                            denominator: 1,
                        },
                        destroyed: turts_mod_api::RefundRatio::ZERO,
                    },
                },
            );
        }
        app.world_mut()
            .insert_resource(turts_sim::bootstrap::ProductionDurations(BTreeMap::from([
                (recipe, 20),
            ])));
        app.world_mut()
            .resource_mut::<turts_sim::AuditTrail>()
            .enable();
        for (seq, reference) in [(9, "test:existing_recipe"), (10, "BAD:??")] {
            let mut staged = staged(
                connection,
                seq,
                FixtureOrder::Produce {
                    producer_id: UnitId(2),
                    recipe_id: reference.into(),
                },
            );
            staged.envelope.order.units = vec![UnitId(2), UnitId(3)];
            assert_eq!(
                production_precheck(app.world_mut(), staged.slot, &staged.envelope),
                None,
                "multi-subject Produce must reach per-unit checks even when no subject is eligible"
            );
            let command = crate::orders::to_sim(
                staged.slot,
                &staged.envelope,
                &app.world().resource::<crate::orders::ServerCatalog>().0,
            )
            .unwrap()
            .remove(0);
            let result = turts_sim::apply_command(app.world_mut(), &command);
            assert!(result.units.accepted.is_empty());
            assert_eq!(
                result.units.refused,
                vec![
                    (2, turts_sim::RejectReason::NotOwner),
                    (3, turts_sim::RejectReason::MissingCapability),
                ]
            );
            let trail = app.world().resource::<turts_sim::AuditTrail>();
            let audit = trail.commands.last().unwrap();
            assert_eq!(audit.result.units.refused, result.units.refused);
            assert_eq!(
                audit.before, audit.after,
                "all-refused envelope must preserve its command boundary"
            );
            assert_eq!(
                trail.operations.last().unwrap().phase,
                turts_sim::economy::transaction::AuditPhase::Command
            );
        }
    }

    #[test]
    fn scheduled_admission_uses_advanced_tick_and_rejects_before_domain_audit() {
        let (mut app, connection) = app();
        app.world_mut().resource_mut::<Tick>().0 = 40;
        let before = {
            let audit = app.world().resource::<turts_sim::AuditTrail>();
            (
                audit.commands.len(),
                audit.operations.len(),
                audit.ticks.len(),
            )
        };
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .push_back(staged(
                connection,
                1,
                FixtureOrder::Produce {
                    producer_id: UnitId(1),
                    recipe_id: "BAD:??".into(),
                },
            ));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        assert_eq!(app.world().resource::<StagedPlayerCommands>().0.len(), 1);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();

        assert_eq!(app.world().resource::<Tick>().0, 41);
        let records = &app.world().resource::<CommandAdmissionDiagnostics>().0;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tick, 41);
        assert_eq!(records[0].reason, CommandRejectReason::UnknownRecipe);
        assert!(app.world().resource::<CommandQueue>().0.is_empty());
        assert_eq!(app.world().resource::<CommandResults>().0.len(), 0);
        let audit = app.world().resource::<turts_sim::AuditTrail>();
        assert_eq!(
            (
                audit.commands.len(),
                audit.operations.len(),
                audit.ticks.len()
            ),
            before
        );
    }

    #[test]
    fn staged_envelopes_survive_a_frame_without_fixed_admission() {
        let (mut app, connection) = app();
        app.world_mut()
            .resource_mut::<StagedPlayerCommands>()
            .0
            .push_back(staged(connection, 1, move_kind()));
        // A render frame with zero fixed ticks does not touch the server-owned staging FIFO.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        assert_eq!(app.world().resource::<StagedPlayerCommands>().0.len(), 1);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        assert_eq!(app.world().resource::<StagedPlayerCommands>().0.len(), 0);
        assert_eq!(
            app.world().get::<Session>(connection).unwrap().last_seq,
            Some(1)
        );
        assert_eq!(app.world().resource::<CommandResults>().0.len(), 1);
    }

    #[test]
    fn server_admit_kind_covers_remove_and_set_enabled() {
        let (mut app, connection) = app();
        let remove = staged(connection, 1, FixtureOrder::Remove { unit_id: UnitId(1) });
        assert_eq!(
            admit_kind(app.world_mut(), &remove, 0),
            Some(CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: ContentId::parse("test:remove").unwrap(),
                behaviour: Behaviour::Remove,
                target_kind: turts_mod_api::TargetKind::None,
                policy: Some(turts_sim::OrderPolicyAudit {
                    id: ContentId::parse("test:remove").unwrap(),
                    behaviour: Behaviour::Remove,
                    target: turts_mod_api::TargetKind::None,
                    fire: None,
                    engage: None,
                    units: vec![],
                    queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
                    unit_capabilities: vec![],
                }),
                units: vec![1],
                target: turts_sim::OrderTargetSim::None,
                queue: turts_sim::QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
            })))
        );
        let toggle = staged(
            connection,
            2,
            FixtureOrder::SetEnabled {
                unit_id: UnitId(1),
                enabled: false,
            },
        );
        assert_eq!(
            admit_kind(app.world_mut(), &toggle, 0),
            Some(CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: ContentId::parse("test:enabled").unwrap(),
                behaviour: Behaviour::SetEnabled(false),
                target_kind: turts_mod_api::TargetKind::None,
                policy: Some(turts_sim::OrderPolicyAudit {
                    id: ContentId::parse("test:enabled").unwrap(),
                    behaviour: Behaviour::SetEnabled(false),
                    target: turts_mod_api::TargetKind::None,
                    fire: None,
                    engage: None,
                    units: vec![],
                    queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
                    unit_capabilities: vec![],
                }),
                units: vec![1],
                target: turts_sim::OrderTargetSim::None,
                queue: turts_sim::QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
            })))
        );
    }
}
