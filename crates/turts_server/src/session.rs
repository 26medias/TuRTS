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
use turts_mod_api::{ContentId, PackageId};
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::messages::*;
use turts_net_protocol::{CommandRejectReason, PlayerCommand, PlayerCommandKind};
use turts_sim::{
    ClaimError, Command as SimCommand, CommandKind, CommandQueue, CompletedWorld, Owner, SimSet,
    Slots, Tick as SimTick, UnitId as SimUnitId, UnitType, UnitTypes, Vec2,
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
            record_admission_rejection(world, staged, tick, CommandRejectReason::BadSequence);
            continue;
        }
        let Some(kind) = admit_kind(world, &staged, tick) else {
            continue;
        };
        world
            .resource_mut::<CommandQueue>()
            .0
            .push_back(SimCommand {
                slot: staged.slot,
                seq: staged.envelope.seq,
                kind,
            });
    }
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

fn admit_kind(world: &mut World, staged: &StagedPlayerCommand, tick: u32) -> Option<CommandKind> {
    let kind = match &staged.envelope.kind {
        PlayerCommandKind::MoveTo { unit_ids, target } => CommandKind::MoveTo {
            unit_ids: unit_ids.iter().map(|id| id.0).collect(),
            target: Vec2::new(target.x, target.y),
        },
        PlayerCommandKind::Gather { unit_id, node_id } => CommandKind::Gather {
            unit_id: unit_id.0,
            node_id: turts_sim::ids::NodeId(node_id.0),
        },
        PlayerCommandKind::Produce {
            producer_id,
            recipe_id,
        } => {
            // Check ownership, then production or research capability, before parsing client text.
            // The simulation still validates enabled recipes and technology queue semantics.
            match owns_unit(world, producer_id.0, staged.slot) {
                Some(false) => {
                    record_admission_rejection(
                        world,
                        staged.clone(),
                        tick,
                        CommandRejectReason::NotOwner,
                    );
                    return None;
                }
                None => {
                    record_admission_rejection(
                        world,
                        staged.clone(),
                        tick,
                        CommandRejectReason::UnknownUnit,
                    );
                    return None;
                }
                Some(true) => {}
            }
            if has_production_capability(world, producer_id.0) != Some(true) {
                record_admission_rejection(
                    world,
                    staged.clone(),
                    tick,
                    CommandRejectReason::MissingCapability,
                );
                return None;
            }
            let parsed = ContentId::parse(recipe_id).and_then(|id| {
                PackageId::parse(&id.package.0).map(|package| ContentId {
                    package,
                    local: id.local,
                })
            });
            let Ok(recipe_id) = parsed else {
                record_admission_rejection(
                    world,
                    staged.clone(),
                    tick,
                    CommandRejectReason::UnknownRecipe,
                );
                return None;
            };
            CommandKind::Produce {
                producer_id: producer_id.0,
                recipe_id,
            }
        }
        PlayerCommandKind::CancelProduction {
            producer_id,
            job_id,
        } => CommandKind::CancelProduction {
            producer_id: producer_id.0,
            job_id: turts_sim::ids::JobId(job_id.0),
        },
        PlayerCommandKind::Remove { unit_id } => CommandKind::Remove { unit_id: unit_id.0 },
        PlayerCommandKind::SetEnabled { unit_id, enabled } => CommandKind::SetEnabled {
            unit_id: unit_id.0,
            enabled: *enabled,
        },
    };
    Some(kind)
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
    if let Some(mut sender) = world.get_mut::<MessageSender<CommandRejected>>(staged.connection) {
        sender.send::<ReliableChannel>(CommandRejected {
            seq: staged.envelope.seq,
            tick,
            reason,
        });
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
    use turts_sim::{CommandResults, Tick, UnitType};

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
        (app, connection)
    }
    fn staged(connection: Entity, seq: u32, kind: PlayerCommandKind) -> StagedPlayerCommand {
        StagedPlayerCommand {
            connection,
            slot: turts_sim::SlotId(0),
            envelope: PlayerCommand { seq, kind },
        }
    }
    fn move_kind() -> PlayerCommandKind {
        PlayerCommandKind::MoveTo {
            unit_ids: vec![UnitId(1)],
            target: Pos { x: 4.0, y: 5.0 },
        }
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
                PlayerCommandKind::Produce {
                    producer_id: UnitId(2),
                    recipe_id: "test:technology".into(),
                },
            ),
            3,
        );
        assert_eq!(
            accepted,
            Some(CommandKind::Produce {
                producer_id: 2,
                recipe_id: ContentId::new(&package, "technology"),
            })
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
            PlayerCommand {
                seq: 1,
                kind: move_kind(),
            },
            PlayerCommand {
                seq: 2,
                kind: PlayerCommandKind::Produce {
                    producer_id: UnitId(1),
                    recipe_id: "BAD:??".into(),
                },
            },
            PlayerCommand {
                seq: 2,
                kind: move_kind(),
            },
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
            Err(turts_sim::RejectReason::NotMobile)
        ));
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
            PlayerCommandKind::Produce {
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
        assert_eq!(records[0].envelope.kind, envelope.envelope.kind);
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
                PlayerCommandKind::Produce {
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
                PlayerCommandKind::Produce {
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
        let remove = staged(
            connection,
            1,
            PlayerCommandKind::Remove { unit_id: UnitId(1) },
        );
        assert_eq!(
            admit_kind(app.world_mut(), &remove, 0),
            Some(CommandKind::Remove { unit_id: 1 })
        );
        let toggle = staged(
            connection,
            2,
            PlayerCommandKind::SetEnabled {
                unit_id: UnitId(1),
                enabled: false,
            },
        );
        assert_eq!(
            admit_kind(app.world_mut(), &toggle, 0),
            Some(CommandKind::SetEnabled {
                unit_id: 1,
                enabled: false
            })
        );
    }
}
