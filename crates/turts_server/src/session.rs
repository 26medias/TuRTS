//! Sessions and slot claims.
//!
//! One `ClientOf` entity is one connection. A `Hello` turns it into a session (a `Session`
//! component) or gets it a `JoinRejected` and a delayed kick. A session sends `PlayerCommand` and
//! `ResyncRequest`. The server never creates a gameplay entity for a session.

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

use crate::{economylog::EconomyLogSink, export, stats::Stats};

/// How long a refused connection stays open after `JoinRejected`. A kick in the same frame
/// loses the message, so the kick waits by time and not by frames (spike fact, plan 6a).
pub const KICK_DELAY: Duration = Duration::from_millis(300);

const REASON_PROTOCOL: &str = "protocol version mismatch";
const REASON_LOCKFILE: &str = "lockfile hash mismatch";
const REASON_ALREADY_IN_MATCH: &str = "player already in match";
const REASON_NO_OPEN_SLOT: &str = "no open slot";

/// A connection that sent an accepted `Hello`.
#[derive(Component)]
pub struct Session {
    /// Last newer sequence consumed at the fixed-tick admission boundary.
    pub last_seq: Option<u32>,
    /// The slot of the player. It is permanent for the match (D25).
    pub slot: turts_sim::SlotId,
    pub player_id: u64,
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

/// Add the session systems. They run in `PreUpdate`, after lightyear has received messages.
pub fn install(app: &mut App, advertised: Advertised) {
    app.insert_resource(advertised);
    app.init_resource::<StagedPlayerCommands>()
        .init_resource::<PendingOrderCommands>()
        .init_resource::<CommandAdmissionDiagnostics>()
        .init_resource::<ReliableTraffic>();
    app.add_systems(
        FixedPreUpdate,
        admit_player_commands
            .after(SimSet::Advance)
            .before(SimSet::Commands),
    );
    app.add_systems(
        PreUpdate,
        (
            receive_hello,
            receive_player_commands,
            receive_resync,
            send_snapshots,
        )
            .chain()
            .after(MessageSystems::Receive),
    );
    app.add_systems(Update, kick_pending);
}

/// Decide a `Hello`. The order of the checks fixes the reason when several fail: protocol
/// version, lockfile hash, then the slot claim. The claim runs last, so a refused `Hello`
/// takes no slot.
pub fn judge_hello(
    hello: &Hello,
    advertised: &Advertised,
    slots: &mut Slots,
) -> Result<turts_sim::SlotId, &'static str> {
    if hello.protocol_version != NET_PROTOCOL_VERSION {
        return Err(REASON_PROTOCOL);
    }
    if hello.lockfile_hash != advertised.lockfile_hash {
        return Err(REASON_LOCKFILE);
    }
    slots.claim(hello.player_id).map_err(|error| match error {
        ClaimError::AlreadyClaimed => REASON_ALREADY_IN_MATCH,
        ClaimError::NoOpenSlot => REASON_NO_OPEN_SLOT,
    })
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
    ),
    With<ClientOf>,
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
) {
    for (entity, mut receiver, has_session, pending_kick) in clients.iter_mut() {
        let mut decided = has_session || pending_kick;
        for hello in receiver.receive() {
            traffic.record(entity);
            if decided {
                continue;
            }
            decided = true;
            stats.hellos += 1;
            match judge_hello(&hello, &advertised, &mut slots) {
                Ok(slot) => {
                    if let Some(log) = economy_log.as_mut() {
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
        With<ClientOf>,
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
pub fn admit_player_commands(world: &mut World) {
    let batch = std::mem::take(&mut world.resource_mut::<StagedPlayerCommands>().0);
    let tick = world.resource::<SimTick>().0;
    for staged in batch {
        if world.contains_resource::<turts_sim::economy::events::MatchStopped>() {
            continue;
        }
        let newer = if let Some(mut session) = world.get_mut::<Session>(staged.connection) {
            if session.slot != staged.slot
                || session
                    .last_seq
                    .is_some_and(|last| staged.envelope.seq <= last)
            {
                false
            } else {
                session.last_seq = Some(staged.envelope.seq);
                true
            }
        } else {
            false
        };
        if !newer {
            let result = crate::orders::refusal(&staged.envelope, CommandRejectReason::BadSequence);
            record_admission_rejection(
                world,
                staged,
                tick,
                result,
                CommandRejectReason::BadSequence,
            );
            continue;
        }
        if let Some(reason) = production_precheck(world, &staged) {
            let result = crate::orders::refusal(&staged.envelope, reason);
            record_admission_rejection(world, staged, tick, result, reason);
            continue;
        }
        let resolved = {
            let catalog = &world.resource::<crate::orders::ServerCatalog>().0;
            crate::orders::to_sim(staged.slot, &staged.envelope, catalog)
        };
        match resolved {
            Ok(commands) => {
                world.resource_mut::<PendingOrderCommands>().0.insert(
                    (staged.slot.0, staged.envelope.seq),
                    staged.envelope.clone(),
                );
                world.resource_mut::<CommandQueue>().0.extend(commands);
            }
            Err(mut result) => {
                result.tick = tick;
                let reason = result
                    .envelope
                    .or_else(|| result.refused.first().map(|refusal| refusal.reason))
                    .unwrap_or(CommandRejectReason::Internal);
                record_admission_rejection(world, staged, tick, result, reason);
            }
        }
    }
}

/// Preserve the existing information-safe Produce admission order while the open envelope is
/// resolved by package behavior: ownership, capability, then target reference syntax.
fn production_precheck(
    world: &mut World,
    staged: &StagedPlayerCommand,
) -> Option<CommandRejectReason> {
    if staged.envelope.order.units.is_empty() || staged.envelope.order.units.len() > MAX_ORDER_UNITS
    {
        return None;
    }
    let order_id = ContentId::parse(&staged.envelope.order.order).ok()?;
    let is_produce = world
        .resource::<crate::orders::ServerCatalog>()
        .0
        .orders
        .get(&order_id)
        .is_some_and(|definition| definition.behaviour == Behaviour::Produce);
    if !is_produce {
        return None;
    }
    let OrderTarget::Content(reference) = &staged.envelope.order.target else {
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
    if staged.envelope.order.units.len() > 1 {
        // Multi-subject envelopes always reach per-unit validation, including malformed
        // content targets, so ownership and capability priority is resolved per subject.
        return None;
    }
    if representable
        && staged.envelope.order.units.iter().any(|unit| {
            owns_unit(world, unit.0, staged.slot) == Some(true)
                && has_production_capability(world, unit.0) == Some(true)
        })
    {
        // A syntactically representable mixed envelope must receive per-unit domain results.
        return None;
    }

    for unit in &staged.envelope.order.units {
        match owns_unit(world, unit.0, staged.slot) {
            Some(true) => {}
            Some(false) => return Some(CommandRejectReason::NotOwner),
            None => return Some(CommandRejectReason::UnknownUnit),
        }
    }
    if staged
        .envelope
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
) {
    let player_id = world
        .get::<Session>(staged.connection)
        .map_or(0, |session| session.player_id);
    let admission_evidence = if world.contains_resource::<EconomyLogSink>() {
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
    With<ClientOf>,
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
                let message = export::snapshot_for(&world.resource::<CompletedWorld>().0, slot);
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

    #[test]
    fn judge_accepts_a_matching_hello_and_claims_the_first_open_slot() {
        let mut slots = Slots::new(2);
        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots),
            Ok(turts_sim::SlotId(0))
        );
        assert_eq!(
            judge_hello(&good(8), &advertised(), &mut slots),
            Ok(turts_sim::SlotId(1))
        );
    }

    #[test]
    fn judge_checks_protocol_before_hash_and_claims_nothing_on_refusal() {
        let mut slots = Slots::new(2);
        let both_wrong = hello(7, 2, "sha256:other");
        assert_eq!(
            judge_hello(&both_wrong, &advertised(), &mut slots),
            Err("protocol version mismatch")
        );
        let wrong_hash = hello(7, NET_PROTOCOL_VERSION, "sha256:other");
        assert_eq!(
            judge_hello(&wrong_hash, &advertised(), &mut slots),
            Err("lockfile hash mismatch")
        );
        assert_eq!(slots.slot_of(7), None, "a refused Hello takes no slot");
    }

    #[test]
    fn protocol_3_hello_is_refused() {
        let mut slots = Slots::new(2);
        assert_eq!(
            judge_hello(&hello(7, 3, HASH), &advertised(), &mut slots),
            Err("protocol version mismatch")
        );
        assert_eq!(slots.slot_of(7), None, "a refused Hello takes no slot");
    }

    #[test]
    fn judge_refuses_the_same_player_twice_and_keeps_the_first_slot() {
        let mut slots = Slots::new(2);
        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots),
            Ok(turts_sim::SlotId(0))
        );
        assert_eq!(
            judge_hello(&good(7), &advertised(), &mut slots),
            Err("player already in match")
        );
        assert_eq!(slots.slot_of(7), Some(turts_sim::SlotId(0)));
        assert_eq!(slots.owner_of(turts_sim::SlotId(1)), None);
    }

    #[test]
    fn judge_refuses_a_third_player_when_no_slot_is_open() {
        let mut slots = Slots::new(2);
        judge_hello(&good(1), &advertised(), &mut slots).unwrap();
        judge_hello(&good(2), &advertised(), &mut slots).unwrap();
        assert_eq!(
            judge_hello(&good(3), &advertised(), &mut slots),
            Err("no open slot")
        );
    }
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
        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_plugins(turts_sim::SimPlugin { tick_hz: 20 })
            .add_systems(
                FixedPreUpdate,
                admit_player_commands
                    .after(SimSet::Advance)
                    .before(SimSet::Commands),
            );
        app.init_resource::<StagedPlayerCommands>()
            .init_resource::<PendingOrderCommands>()
            .init_resource::<CommandAdmissionDiagnostics>()
            .init_resource::<ReliableTraffic>();
        let package = PackageId::parse("test").unwrap();
        let mut types = UnitTypes::default();
        let type_id = types
            .register_unit_type(
                &package,
                UnitDef {
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
            )
            .unwrap();
        let id = ContentId::new(&package, "producer");
        let mut catalog = ContentCatalog {
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        catalog.units.insert(
            id.clone(),
            ResolvedUnit {
                id,
                name: "Producer".into(),
                footprint: Footprint::Circle { radius: 1.0 },
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: Some(ResolvedProducer {
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
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
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
    fn production_receive_survives_lightyear_last_without_a_fixed_tick() {
        use lightyear::link::recv_payload_from_bytes;
        use lightyear::prelude::server::ServerPlugins;

        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.add_plugins(turts_sim::SimPlugin { tick_hz: 20 });
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
            )
            .unwrap();
        let id = ContentId::new(&package, "producer");
        let mut catalog = ContentCatalog {
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        catalog.units.insert(
            id.clone(),
            ResolvedUnit {
                id,
                name: "Producer".into(),
                footprint: Footprint::Circle { radius: 1.0 },
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: Some(ResolvedProducer {
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
        let producer_type = app
            .world()
            .iter_entities()
            .find_map(|entity| entity.get::<UnitType>().copied())
            .unwrap();
        app.world_mut()
            .spawn((SimUnitId(2), Owner(turts_sim::SlotId(1)), producer_type));
        app.world_mut()
            .spawn((SimUnitId(4), Owner(turts_sim::SlotId(0)), producer_type));
        let static_type = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &PackageId::parse("test").unwrap(),
                UnitDef {
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
        assert_eq!(production_precheck(app.world_mut(), &produce_staged), None);
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
        assert_eq!(production_precheck(app.world_mut(), &later_success), None);
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
                production_precheck(app.world_mut(), &staged),
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
