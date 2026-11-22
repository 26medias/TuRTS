//! Frozen SV4 transport workload. All packet and message acknowledgements use Lightyear.
#![cfg(test)]

use super::*;
use lightyear::link::recv_payload_from_bytes;
use lightyear::prelude::server::{ClientOf, ServerPlugins};
use lightyear::transport::plugin::TransportSystems;
use std::collections::BTreeMap;
use turts_client::{
    content::ClientContent,
    replica::{Replica, ReplicaContext},
};
use turts_mod_api::ContentId;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::visibility_transaction::StateTransactionAssembler;

const STEP_MS: u64 = 5;
const TICK_MS: u64 = 50;
const WARMUP_TICKS: u32 = 100;
const MEASURED_TICKS: u32 = 1200;
const FINAL_TICK: u32 = WARMUP_TICKS + MEASURED_TICKS;

// The app contains both endpoints. Charge its complete registered receive and
// send service to each peer rather than infer a cheaper per-endpoint share.
#[derive(Resource, Default)]
struct BatchTransportClock {
    receive_start: Option<Instant>,
    send_start: Option<Instant>,
    receive_ns: u64,
    send_ns: u64,
}
fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
fn begin_batch_receive(mut clock: ResMut<BatchTransportClock>) {
    clock.receive_start = Some(Instant::now());
}
fn end_batch_receive(mut clock: ResMut<BatchTransportClock>) {
    clock.receive_ns = elapsed_ns(clock.receive_start.take().unwrap());
}
fn begin_batch_send(mut clock: ResMut<BatchTransportClock>) {
    clock.send_start = Some(Instant::now());
}
fn end_batch_send(mut clock: ResMut<BatchTransportClock>) {
    clock.send_ns = elapsed_ns(clock.send_start.take().unwrap());
}
#[derive(serde::Serialize)]
struct ClientBatchSample {
    application_time_ms: u64,
    shared_transport_receive_ns: u64,
    shared_transport_send_ns: u64,
    acknowledgement_queue_ns: u64,
    assembly_validation_activation_ns: u64,
    total_ns: u64,
    activated_transactions: usize,
}
fn batch_quantiles(samples: &[ClientBatchSample]) -> Option<(u64, u64)> {
    let mut sorted = samples
        .iter()
        .map(|sample| sample.total_ns)
        .collect::<Vec<_>>();
    sorted.sort_unstable();
    let maximum = sorted.last().copied()?;
    Some((sorted[(sorted.len() * 99).div_ceil(100) - 1], maximum))
}

#[derive(Resource, Default)]
struct CapturedFrames {
    frames: BTreeMap<Entity, Vec<StateTransactionFrame>>,
    cancels: BTreeMap<Entity, Vec<StateTransactionCancel>>,
}
fn capture_frames(
    mut frames: Query<(Entity, &mut MessageReceiver<StateTransactionFrame>)>,
    mut cancels: Query<(Entity, &mut MessageReceiver<StateTransactionCancel>)>,
    mut captured: ResMut<CapturedFrames>,
) {
    for (entity, mut receiver) in &mut frames {
        captured
            .frames
            .entry(entity)
            .or_default()
            .extend(receiver.receive());
    }
    for (entity, mut receiver) in &mut cancels {
        if captured.frames.contains_key(&entity) {
            captured
                .cancels
                .entry(entity)
                .or_default()
                .extend(receiver.receive());
        }
    }
}
#[derive(Resource, Clone, Copy)]
struct PumpClock {
    epoch: Instant,
    now: Instant,
}
#[derive(Default)]
struct PumpReport {
    transport_acks: Vec<u16>,
    issued: BTreeSet<u16>,
    first_sent: Vec<(u64, u64)>,
    age_ms: u64,
    backlog: usize,
    error: Option<String>,
}
#[derive(Resource, Default)]
struct PumpReports(BTreeMap<Entity, PumpReport>);
#[derive(Resource, Default)]
struct StoppedPeers(BTreeSet<Entity>);
/// The frame limit of the harness pump: the production rule from the ticks that were queued
/// since the last pump, or the fixed limit of 16 that the server had before the M4b repair.
#[derive(Resource, Default)]
struct HarnessPump {
    fixed_old_limit: bool,
    ticks_since_pump: u32,
}
fn pump_in_production_schedule(world: &mut World) {
    let clock = *world.resource::<PumpClock>();
    let budget = {
        let mut pump = world.resource_mut::<HarnessPump>();
        let ticks = std::mem::take(&mut pump.ticks_since_pump);
        if pump.fixed_old_limit {
            FRAMES_PER_PUMP
        } else {
            frame_budget(ticks)
        }
    };
    let entities = world
        .iter_entities()
        .filter(|entity| entity.contains::<KnowledgeSender>())
        .map(|entity| entity.id())
        .collect::<Vec<_>>();
    for entity in entities {
        if world.resource::<StoppedPeers>().0.contains(&entity) {
            continue;
        }
        let mut sender = world.entity_mut(entity).take::<KnowledgeSender>().unwrap();
        let old_ids = sender.outstanding.clone();
        let transport_acks = world
            .get::<Transport>(entity)
            .unwrap()
            .channel_send(lightyear::transport::channel::ChannelKind::of::<
                KnowledgeChannel,
            >())
            .unwrap()
            .message_acks()
            .iter()
            .map(|id| id.0)
            .collect::<Vec<_>>();
        let error = pump_one(world, entity, &mut sender, clock.now, budget).err();
        assert!(
            sender.outstanding.len() <= WINDOW,
            "the pump went above the send window"
        );
        for id in old_ids.difference(&sender.outstanding) {
            assert!(
                transport_acks.contains(id),
                "only actual transport MessageIds release frame credit"
            );
        }
        let first_sent = sender
            .pending
            .iter()
            .chain(sender.patches.iter())
            .filter_map(|pending| {
                pending.started.map(|started| {
                    (
                        pending.encoded.begin().stamp.transaction_id,
                        started.duration_since(clock.epoch).as_millis() as u64,
                    )
                })
            })
            .collect();
        let age_ms = sender.pending.as_ref().map_or(0, |pending| {
            clock
                .now
                .saturating_duration_since(pending.started.unwrap_or(pending.created))
                .as_millis() as u64
        });
        assert!(sender.retained() <= MAX_PENDING_STATE_BYTES as usize);
        let report = PumpReport {
            transport_acks,
            issued: sender.outstanding.clone(),
            first_sent,
            age_ms,
            backlog: sender.patch_bytes,
            error,
        };
        world.entity_mut(entity).insert(sender);
        if report.error.is_some() {
            world.resource_mut::<StoppedPeers>().0.insert(entity);
        }
        world.resource_mut::<PumpReports>().0.insert(entity, report);
    }
}

struct Peer {
    client: Entity,
    server: Entity,
    assembler: StateTransactionAssembler,
    replica: Replica,
    activations: Vec<u32>,
    first_sent_ms: BTreeMap<u64, u64>,
    activation_acks: VecDeque<(u64, StateTransactionAck)>,
    transport_acks: Vec<(u64, u16)>,
    cancellations: Vec<(u64, u64, String)>,
    issued_messages: BTreeSet<u16>,
    max_batch: usize,
    batch_samples: Vec<ClientBatchSample>,
    max_age_ms: u64,
    max_backlog: usize,
    first_measured_backlog: Option<usize>,
    last_measured_backlog: usize,
    restart_error: Option<String>,
}
struct Harness {
    app: App,
    epoch: Instant,
    now_ms: u64,
    peers: Vec<Peer>,
    packets: VecDeque<(u64, Entity, Vec<u8>)>,
    rtt_ms: u64,
    burst_at_ms: Option<u64>,
    retain_received_frames: bool,
    retained_frames: Vec<StateTransactionFrame>,
    step_ms: u64,
}
std::thread_local! {
    /// The units that move in each tick. It sets the size of one tick transaction.
    static MOVING_UNITS: std::cell::Cell<u32> = const { std::cell::Cell::new(500) };
}
fn moving_units() -> u32 {
    MOVING_UNITS.with(std::cell::Cell::get)
}
fn position(unit: u32, tick: u32) -> bevy::math::Vec2 {
    let motion = if unit <= moving_units() {
        (tick % 100) as f32 * 0.01
    } else {
        0.0
    };
    bevy::math::Vec2::new(
        1.0 + ((unit - 1) % 100) as f32 + motion,
        1.0 + ((unit - 1) / 100) as f32 + motion,
    )
}
fn world_view(tick: u32) -> turts_sim::WorldView {
    let mut view = turts_sim::WorldView {
        tick,
        ..Default::default()
    };
    view.units = (1..=1000)
        .map(|unit_id| turts_sim::world::UnitState {
            unit_id,
            content_id: "visibility_probe:scout".into(),
            owner_slot: u8::from(unit_id > 500),
            facing: None,
            pos: position(unit_id, tick),
        })
        .collect();
    view.m3.raw_healths = (1..=1000)
        .map(|unit| turts_sim::economy::views_m3::RawHealthAudit { unit, current: 100 })
        .collect();
    view
}
impl Harness {
    fn new(rtt_ms: u64, count: usize) -> Self {
        Self::with_loss_timeout(rtt_ms, count, true)
    }
    /// `server_loss_timeout` false keeps the lightyear default (10 ms floor) on the server
    /// transports: the state before the M4b repair.
    fn with_loss_timeout(rtt_ms: u64, count: usize, server_loss_timeout: bool) -> Self {
        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(TICK_MS),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.init_resource::<CapturedFrames>()
            .init_resource::<BatchTransportClock>()
            .init_resource::<PumpReports>()
            .init_resource::<HarnessPump>()
            .init_resource::<StoppedPeers>();
        if server_loss_timeout {
            app.add_systems(
                PreUpdate,
                set_server_loss_timeout.before(TransportSystems::Receive),
            );
        }
        app.add_systems(
            PreUpdate,
            begin_batch_receive
                .before(TransportSystems::Receive)
                .before(MessageSystems::Receive),
        );
        app.add_systems(PreUpdate, capture_frames.after(MessageSystems::Receive));
        app.add_systems(
            PreUpdate,
            end_batch_receive
                .after(TransportSystems::Receive)
                .after(MessageSystems::Receive)
                .after(capture_frames),
        );
        app.add_systems(
            PostUpdate,
            begin_batch_send
                .after(pump_in_production_schedule)
                .before(MessageSystems::Send)
                .before(TransportSystems::Send),
        );
        app.add_systems(
            PostUpdate,
            end_batch_send
                .after(MessageSystems::Send)
                .after(TransportSystems::Send),
        );
        app.add_systems(
            PostUpdate,
            pump_in_production_schedule.before(MessageSystems::Send),
        );
        app.finish();
        app.cleanup();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(STEP_MS),
        ));
        app.insert_resource(crate::stats::MatchId(91));
        let map = ContentId::parse("visibility_probe:field").unwrap();
        app.insert_resource(turts_sim::bootstrap::InstalledMapIdentity(map.clone()));
        app.insert_resource(turts_sim::CompletedWorld(world_view(0)));
        app.insert_resource(turts_sim::CommittedTick::default());
        let archive = PackageArchive::parse(
            &pack_dir(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../mods/visibility_probe"),
            )
            .unwrap(),
        )
        .unwrap();
        let content = ClientContent::from_archives(&[archive]).unwrap();
        let peers = (0..count)
            .map(|index| {
                let registry = app.world().resource::<ChannelRegistry>();
                let mut server_transport = Transport::default();
                server_transport.add_channel_send_from_registry::<KnowledgeChannel>(registry);
                server_transport.add_channel_send_from_registry::<ReliableChannel>(registry);
                server_transport.add_channel_receive_from_registry::<ReliableChannel>(registry);
                let mut client_transport = Transport::default();
                client_transport.add_channel_receive_from_registry::<KnowledgeChannel>(registry);
                client_transport.add_channel_send_from_registry::<ReliableChannel>(registry);
                client_transport.add_channel_receive_from_registry::<ReliableChannel>(registry);
                let server = app
                    .world_mut()
                    .spawn((
                        ClientOf,
                        Link::default(),
                        server_transport,
                        RemoteId(PeerId::Local(index as u64 + 17)),
                        Linked,
                        Connected,
                        MessageReceiver::<StateTransactionAck>::default(),
                        MessageReceiver::<StateTransactionCancel>::default(),
                        MessageSender::<StateTransactionCancel>::default(),
                    ))
                    .id();
                let client = app
                    .world_mut()
                    .spawn((
                        Client,
                        Link::default(),
                        client_transport,
                        RemoteId(PeerId::Server),
                        Linked,
                        Connected,
                        MessageReceiver::<StateTransactionFrame>::default(),
                        MessageReceiver::<StateTransactionCancel>::default(),
                        MessageSender::<StateTransactionAck>::default(),
                    ))
                    .id();
                let mut replica = Replica::new(20);
                replica.set_context(ReplicaContext::new(content.clone(), 0));
                assert!(queue_full(app.world_mut(), server, SlotId(0)).unwrap());
                Peer {
                    client,
                    server,
                    assembler: StateTransactionAssembler::new(91, map.clone(), FRAME_LIMIT),
                    replica,
                    activations: vec![],
                    first_sent_ms: BTreeMap::new(),
                    activation_acks: VecDeque::new(),
                    transport_acks: vec![],
                    cancellations: vec![],
                    issued_messages: BTreeSet::new(),
                    max_batch: 0,
                    batch_samples: vec![],
                    max_age_ms: 0,
                    max_backlog: 0,
                    first_measured_backlog: None,
                    last_measured_backlog: 0,
                    restart_error: None,
                }
            })
            .collect();
        Self {
            app,
            epoch: Instant::now(),
            now_ms: 0,
            peers,
            packets: VecDeque::new(),
            rtt_ms,
            burst_at_ms: None,
            retain_received_frames: false,
            retained_frames: vec![],
            step_ms: STEP_MS,
        }
    }
    /// Server tick minus the newest tick whose End frame went to the transport.
    fn send_lag(&self, peer: usize) -> u32 {
        let sender = self
            .app
            .world()
            .get::<KnowledgeSender>(self.peers[peer].server)
            .unwrap();
        let last_end = sender
            .pending
            .iter()
            .chain(sender.patches.iter())
            .filter(|entry| entry.end_sent)
            .map(|entry| entry.encoded.begin().stamp.basis.tick)
            .max()
            .or_else(|| sender.active.as_ref().map(|stamp| stamp.basis.tick))
            .unwrap_or(0);
        sender.tip.state.tick - last_end
    }
    fn set_step_ms(&mut self, step_ms: u64) {
        self.step_ms = step_ms;
        self.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(step_ms),
            ));
    }
    fn enqueue_tick(&mut self, tick: u32) {
        self.app
            .world_mut()
            .resource_mut::<HarnessPump>()
            .ticks_since_pump += 1;
        self.app
            .insert_resource(turts_sim::CompletedWorld(world_view(tick)));
        self.app.insert_resource(turts_sim::CommittedTick {
            tick,
            positions: (1..=moving_units())
                .map(|unit| (unit, position(unit, tick)))
                .collect(),
            ..Default::default()
        });
        for peer in &self.peers {
            if peer.restart_error.is_none() {
                let previous_id = self
                    .app
                    .world()
                    .get::<KnowledgeSender>(peer.server)
                    .unwrap()
                    .next_id;
                assert!(queue_tick(self.app.world_mut(), peer.server, SlotId(0)).unwrap());
                let mut sender = self
                    .app
                    .world_mut()
                    .get_mut::<KnowledgeSender>(peer.server)
                    .unwrap();
                // Public enqueue uses Instant::now(). This test advances a virtual
                // pump clock. Align ONLY newly enqueued timestamps, not deadlines
                // or already-started work, so both sides use the same test clock.
                let enqueued_at = self.epoch + Duration::from_millis(self.now_ms);
                let sender = &mut *sender;
                for pending in sender.pending.iter_mut().chain(sender.patches.iter_mut()) {
                    if pending.encoded.begin().stamp.transaction_id > previous_id {
                        assert!(pending.started.is_none());
                        pending.created = enqueued_at;
                    }
                }
            }
        }
    }
    fn step(&mut self, drop_return_from: Option<usize>) {
        let now = self.epoch + Duration::from_millis(self.now_ms);
        let mut future = VecDeque::new();
        while let Some((due, target, packet)) = self.packets.pop_front() {
            if due <= self.now_ms {
                self.app
                    .world_mut()
                    .get_mut::<Link>(target)
                    .unwrap()
                    .recv
                    .push_raw(recv_payload_from_bytes(packet.into()));
            } else {
                future.push_back((due, target, packet));
            }
        }
        self.packets = future;
        let mut acknowledgement_queue_ns = Vec::with_capacity(self.peers.len());
        for peer in &mut self.peers {
            let queue_start = Instant::now();
            while peer
                .activation_acks
                .front()
                .is_some_and(|(due, _)| *due <= self.now_ms)
            {
                let (_, ack) = peer.activation_acks.pop_front().unwrap();
                self.app
                    .world_mut()
                    .get_mut::<MessageSender<StateTransactionAck>>(peer.client)
                    .unwrap()
                    .send::<ReliableChannel>(ack);
            }
            acknowledgement_queue_ns.push(elapsed_ns(queue_start));
        }
        self.app.insert_resource(PumpClock {
            epoch: self.epoch,
            now,
        });
        self.app.update();
        let transport_clock = self.app.world().resource::<BatchTransportClock>();
        let shared_receive_ns = transport_clock.receive_ns;
        let shared_send_ns = transport_clock.send_ns;
        for (peer, acknowledgement_queue_ns) in self.peers.iter_mut().zip(acknowledgement_queue_ns)
        {
            let report = self
                .app
                .world_mut()
                .resource_mut::<PumpReports>()
                .0
                .remove(&peer.server)
                .unwrap_or_default();
            peer.transport_acks.extend(
                report
                    .transport_acks
                    .into_iter()
                    .map(|id| (self.now_ms, id)),
            );
            peer.issued_messages.extend(report.issued);
            for (id, sent) in report.first_sent {
                peer.first_sent_ms.entry(id).or_insert(sent);
            }
            if let Some(error) = report.error {
                peer.restart_error = Some(error);
            }
            if self.now_ms >= u64::from(WARMUP_TICKS) * TICK_MS {
                peer.max_age_ms = peer.max_age_ms.max(report.age_ms);
                peer.max_backlog = peer.max_backlog.max(report.backlog);
                peer.first_measured_backlog.get_or_insert(report.backlog);
                peer.last_measured_backlog = report.backlog;
            }
            let consume_start = Instant::now();
            let (cancels, frames) = {
                let mut captured = self.app.world_mut().resource_mut::<CapturedFrames>();
                (
                    captured.cancels.remove(&peer.client).unwrap_or_default(),
                    captured.frames.remove(&peer.client).unwrap_or_default(),
                )
            };
            for cancel in cancels {
                peer.cancellations.push((
                    self.now_ms,
                    cancel.stamp.transaction_id,
                    format!("{:?}", cancel.reason),
                ));
                peer.assembler.cancel(&cancel.stamp);
            }
            if self.retain_received_frames {
                self.retained_frames.extend(frames.iter().cloned());
            }
            let mut activations = 0;
            for frame in frames {
                assert!(frame.encoded_bytes().unwrap() <= FRAME_LIMIT);
                let stamp = match &frame {
                    StateTransactionFrame::Begin(x) => &x.stamp,
                    StateTransactionFrame::Chunk(x) => &x.stamp,
                    StateTransactionFrame::End(x) => &x.stamp,
                };
                if peer.assembler.is_obsolete(stamp) {
                    continue;
                }
                match frame {
                    StateTransactionFrame::Begin(begin) => {
                        peer.assembler.begin(begin, self.now_ms).unwrap()
                    }
                    StateTransactionFrame::Chunk(chunk) => peer.assembler.chunk(&chunk).unwrap(),
                    StateTransactionFrame::End(end) => {
                        let staged = peer
                            .assembler
                            .end(&end)
                            .unwrap()
                            .expect("each completed frame yields one whole basis");
                        assert!(
                            peer.replica.activate_transaction(
                                &staged.stamp,
                                &staged.payload,
                                bevy::platform::time::Instant::now()
                            ),
                            "client activation rejected tick{}",
                            staged.stamp.basis.tick
                        );
                        let tick = staged.stamp.basis.tick;
                        assert_eq!(peer.replica.last_applied_tick, Some(tick));
                        assert_eq!(peer.replica.units.len(), 1000);
                        peer.activations.push(tick);
                        activations += 1;
                        let ack = peer.assembler.activate(&staged.stamp).unwrap();
                        let first = peer.first_sent_ms[&staged.stamp.transaction_id];
                        // The extra5ms is the real Link return delivery below.
                        let due = (first + self.rtt_ms)
                            .saturating_sub(self.step_ms)
                            .max(self.now_ms);
                        peer.activation_acks.push_back((due, ack));
                    }
                }
            }
            let consume_ns = elapsed_ns(consume_start);
            peer.max_batch = peer.max_batch.max(activations);
            // Keep empty updates and the final drain. Bootstrap/warm-up is excluded.
            if self.now_ms > u64::from(WARMUP_TICKS) * TICK_MS {
                peer.batch_samples.push(ClientBatchSample {
                    application_time_ms: self.now_ms,
                    shared_transport_receive_ns: shared_receive_ns,
                    shared_transport_send_ns: shared_send_ns,
                    acknowledgement_queue_ns,
                    assembly_validation_activation_ns: consume_ns,
                    total_ns: shared_receive_ns
                        .saturating_add(shared_send_ns)
                        .saturating_add(acknowledgement_queue_ns)
                        .saturating_add(consume_ns),
                    activated_transactions: activations,
                });
            }
        }
        for (index, peer) in self.peers.iter().enumerate() {
            for (from, to, returning) in [
                (peer.server, peer.client, false),
                (peer.client, peer.server, true),
            ] {
                let packets = self
                    .app
                    .world_mut()
                    .get_mut::<Link>(from)
                    .unwrap()
                    .send
                    .drain()
                    .collect::<Vec<_>>();
                if returning && drop_return_from == Some(index) {
                    continue;
                }
                for packet in packets {
                    let mut due = self.now_ms + self.step_ms;
                    if !returning
                        && self
                            .burst_at_ms
                            .is_some_and(|at| self.now_ms >= at && self.now_ms < at + TICK_MS)
                    {
                        due = self.burst_at_ms.unwrap() + TICK_MS + self.step_ms;
                    }
                    self.packets.push_back((due, to, packet.to_vec()));
                }
            }
        }
        self.now_ms += self.step_ms;
    }
    fn evidence(&self, label: &str) {
        let rows = self.peers.iter().enumerate().map(|(index, peer)| serde_json::json!({
            "peer": index, "activation_ticks": peer.activations, "actual_transport_message_acks": peer.transport_acks,
            "actual_issued_message_ids": peer.issued_messages, "cancellations": peer.cancellations, "max_transactions_in_one_client_update": peer.max_batch,
            "max_unacknowledged_age_ms": peer.max_age_ms, "max_encoded_successor_bytes": peer.max_backlog,
            "first_measured_backlog_bytes": peer.first_measured_backlog, "final_backlog_bytes": peer.last_measured_backlog,
            "restart_error": peer.restart_error,
            "client_receive_batches": peer.batch_samples,
            "client_batch_p99_ns": batch_quantiles(&peer.batch_samples).map(|quantiles| quantiles.0),
            "client_batch_max_ns": batch_quantiles(&peer.batch_samples).map(|quantiles| quantiles.1),
        })).collect::<Vec<_>>();
        let report = serde_json::json!({"label": label, "activation_ack_rtt_ms": self.rtt_ms, "application_step_ms": STEP_MS,
            "sender_hz": 20, "snapshot_units": 1000, "position_updates_per_tick": 500, "warmup_ticks": WARMUP_TICKS,
            "measured_ticks": MEASURED_TICKS,
            "release_profile": !cfg!(debug_assertions),
            "client_batch_bounds_enforced": !cfg!(debug_assertions),
            "client_batch_p99_limit_ns": 16_667_000u64,
            "client_batch_max_limit_ns": 33_333_000u64,
            "client_batch_timing": "whole peer consume/assembly/validation/activation/ACK queue plus complete shared registered transport receive/decode and send; shared work charged to every peer; empty warmed updates and final drain included",
            "quantile": "nearest-rank ceil(0.99*n)-1",
            "one_delivery_burst_ms": self.burst_at_ms.map_or(0, |_| TICK_MS), "peers": rows});
        if let Some(directory) = std::env::var_os("TURTS_VISIBILITY_PIPELINE_EVIDENCE") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                directory.join(format!("{label}.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        for (index, peer) in self.peers.iter().enumerate() {
            eprintln!(
                "{label} peer{index}: activations={} transport_acks={} max_batch={} max_age_ms={} max_backlog={} restart={:?}",
                peer.activations.len(),
                peer.transport_acks.len(),
                peer.max_batch,
                peer.max_age_ms,
                peer.max_backlog,
                peer.restart_error
            );
            if let Some((p99, maximum)) = batch_quantiles(&peer.batch_samples) {
                eprintln!(
                    "{label} peer{index}: client_batches={} batch_p99_ns={p99} batch_max_ns={maximum} release_bounds={}",
                    peer.batch_samples.len(),
                    !cfg!(debug_assertions)
                );
                if !cfg!(debug_assertions) {
                    assert!(
                        p99 <= 16_667_000,
                        "client batch p99 {p99} ns exceeds 60 FPS service period"
                    );
                    assert!(
                        maximum <= 33_333_000,
                        "client batch maximum {maximum} ns exceeds 30 FPS service period"
                    );
                }
            }
        }
    }
}

#[test]
fn frozen_20hz_pipeline_preserves_every_tick_at_50_and_100_ms_activation_ack_rtt() {
    for rtt_ms in [50, 100] {
        let mut harness = Harness::new(rtt_ms, 1);
        harness.burst_at_ms = Some(u64::from(WARMUP_TICKS + 20) * TICK_MS);
        while harness.now_ms <= u64::from(FINAL_TICK) * TICK_MS {
            if harness.now_ms > 0 && harness.now_ms.is_multiple_of(TICK_MS) {
                harness.enqueue_tick((harness.now_ms / TICK_MS) as u32);
            }
            harness.step(None);
        }
        // Drain the final delayed delivery and ACK tail. Do not leave the last movements queued.
        for _ in 0..100 {
            harness.step(None);
        }
        let peer = &harness.peers[0];
        harness.evidence(&format!("pipeline-rtt-{rtt_ms}"));
        assert_eq!(
            peer.activations.len(),
            FINAL_TICK as usize + 1,
            "all movement ticks must activate"
        );
        assert!(
            peer.activations
                .iter()
                .enumerate()
                .all(|(index, tick)| index as u32 == *tick),
            "activation ticks must be exact-once and ordered"
        );
        assert!(peer.max_batch >= 2);
        assert!(
            peer.batch_samples
                .iter()
                .any(|sample| sample.activated_transactions >= 2)
        );
        assert!(peer.restart_error.is_none());
        assert!(peer.max_age_ms <= 1000);
        assert!(peer.max_backlog <= 1024 * 1024);
        assert!(peer.last_measured_backlog <= peer.first_measured_backlog.unwrap());
        assert!(!peer.transport_acks.is_empty());
        assert!(
            peer.transport_acks
                .iter()
                .all(|(_, id)| peer.issued_messages.contains(id))
        );
        let sender = harness
            .app
            .world()
            .get::<KnowledgeSender>(peer.server)
            .unwrap();
        assert!(sender.restarts.is_empty());
        assert!(sender.pending.is_none() && sender.patches.is_empty());
        assert!(sender.outstanding.is_empty());
        assert_eq!(sender.active_stamp().unwrap().basis.tick, FINAL_TICK);
    }
}

#[test]
fn a_nonreturning_viewer_restarts_and_disconnects_without_stalling_the_healthy_viewer() {
    let mut harness = Harness::new(100, 2);
    while harness.now_ms <= 16_000 {
        if harness.now_ms > 0 && harness.now_ms.is_multiple_of(TICK_MS) {
            harness.enqueue_tick((harness.now_ms / TICK_MS) as u32);
        }
        harness.step(Some(0));
    }
    for _ in 0..100 {
        harness.step(Some(0));
    }
    harness.evidence("slow-and-healthy");
    assert!(
        harness.peers[0]
            .restart_error
            .as_ref()
            .is_some_and(|reason| reason.contains("three times"))
    );
    assert!(harness.peers[1].restart_error.is_none());
    assert_eq!(harness.peers[1].activations.len(), 321);
    assert!(
        harness.peers[1]
            .activations
            .iter()
            .enumerate()
            .all(|(index, tick)| index as u32 == *tick)
    );
    let healthy = harness
        .app
        .world()
        .get::<KnowledgeSender>(harness.peers[1].server)
        .unwrap();
    assert!(healthy.restarts.is_empty());
    assert!(healthy.outstanding.is_empty());
    let slow = harness
        .app
        .world()
        .get::<KnowledgeSender>(harness.peers[0].server)
        .unwrap();
    assert_eq!(slow.outstanding.len(), WINDOW);
    assert!(slow.retained() <= MAX_PENDING_STATE_BYTES as usize);
}

#[test]
fn superseded_sent_suffix_keeps_credit_and_cannot_reactivate_after_replacement() {
    let mut harness = Harness::new(100, 1);
    for _ in 0..60 {
        harness.step(None);
    }
    let server = harness.peers[0].server;
    let client = harness.peers[0].client;
    assert!(
        harness
            .app
            .world()
            .get::<KnowledgeSender>(server)
            .unwrap()
            .pending
            .is_none()
    );
    harness.enqueue_tick(1);
    harness.enqueue_tick(2);
    harness.step(None);
    let (old_stamps, old_credits) = {
        let sender = harness.app.world().get::<KnowledgeSender>(server).unwrap();
        let stamps = sender
            .pending
            .iter()
            .chain(sender.patches.iter())
            .map(|pending| {
                assert!(
                    pending.end_sent,
                    "both successors must really be submitted before cancellation"
                );
                pending.encoded.begin().stamp.clone()
            })
            .collect::<Vec<_>>();
        (stamps, sender.outstanding.clone())
    };
    assert_eq!(old_stamps.len(), 2);
    assert!(!old_credits.is_empty());
    let watermark = old_stamps.last().unwrap().transaction_id;
    assert!(queue_full(harness.app.world_mut(), server, SlotId(0)).unwrap());
    {
        let mut sender = harness
            .app
            .world_mut()
            .get_mut::<KnowledgeSender>(server)
            .unwrap();
        assert_eq!(
            sender.outstanding, old_credits,
            "logical cancellation must not release submitted reliable copies"
        );
        for old in &old_stamps {
            assert!(
                !sender.acknowledge(old),
                "old activation ACK cannot advance replacement"
            );
        }
    }
    harness.retain_received_frames = true;
    for _ in 0..100 {
        harness.step(None);
    }
    harness.retain_received_frames = false;
    let active = harness
        .app
        .world()
        .get::<KnowledgeSender>(server)
        .unwrap()
        .active_stamp()
        .unwrap()
        .clone();
    assert!(active.transaction_id > watermark);
    assert_eq!(active.basis.tick, 2);
    let old_frames = harness
        .retained_frames
        .iter()
        .filter(|frame| {
            let stamp = match frame {
                StateTransactionFrame::Begin(x) => &x.stamp,
                StateTransactionFrame::Chunk(x) => &x.stamp,
                StateTransactionFrame::End(x) => &x.stamp,
            };
            stamp.transaction_id <= watermark
        })
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        !old_frames.is_empty(),
        "capture actual retired frames from the real receiver"
    );
    let previous_activations = harness.peers[0].activations.clone();
    let mut late_message_ids = Vec::new();
    // Deliver the old application stamps again through NEW reliable message identities.
    // This proves application retirement instead of merely relying on packet deduplication.
    for frame in old_frames {
        let id = send_frame(harness.app.world_mut(), server, &frame).unwrap();
        assert!(
            harness
                .app
                .world_mut()
                .get_mut::<KnowledgeSender>(server)
                .unwrap()
                .outstanding
                .insert(id)
        );
        late_message_ids.push(id);
    }
    for stamp in old_stamps {
        harness
            .app
            .world_mut()
            .get_mut::<MessageSender<StateTransactionAck>>(client)
            .unwrap()
            .send::<ReliableChannel>(StateTransactionAck { stamp });
    }
    for _ in 0..100 {
        harness.step(None);
    }
    harness.evidence("retired-suffix-after-replacement");
    assert_eq!(harness.peers[0].activations, previous_activations);
    assert_eq!(harness.peers[0].replica.last_applied_tick, Some(2));
    let sender = harness.app.world().get::<KnowledgeSender>(server).unwrap();
    assert_eq!(sender.active_stamp(), Some(&active));
    assert!(sender.outstanding.is_empty());
    assert!(late_message_ids.iter().all(|id| {
        harness.peers[0]
            .transport_acks
            .iter()
            .any(|(_, ack)| ack == id)
    }));
}

/// The M4b overload case: a tick costs more than its period, so one application frame of
/// 250 ms (`max_delta`) runs 5 fixed ticks and an acknowledgement is seen 2 frames after the
/// send. Returns the largest send lag after the first slow frames and the lag at the end.
fn slow_frame_send_lag(fixed_old_limit: bool, server_loss_timeout: bool) -> (u32, u32, usize) {
    const SLOW_FRAME_MS: u64 = 250;
    const TICKS_PER_SLOW_FRAME: u32 = 5;
    const SLOW_FRAMES: u32 = 60;
    // 200 moving units give a tick transaction of 4 frames: 20 frames for each slow frame.
    MOVING_UNITS.with(|units| units.set(200));
    let mut harness = Harness::with_loss_timeout(50, 1, server_loss_timeout);
    harness
        .app
        .world_mut()
        .resource_mut::<HarnessPump>()
        .fixed_old_limit = fixed_old_limit;
    let mut tick = 0;
    // Normal load first: 1 tick for each 50 ms, frames of 5 ms. The baseline completes.
    while harness.now_ms < 40 * TICK_MS {
        if harness.now_ms > 0 && harness.now_ms.is_multiple_of(TICK_MS) {
            tick += 1;
            harness.enqueue_tick(tick);
        }
        harness.step(None);
    }
    assert_eq!(harness.send_lag(0), 0, "normal load must leave no send lag");
    let frames_before = harness.peers[0].issued_messages.len();
    harness.set_step_ms(SLOW_FRAME_MS);
    let (mut peak, mut last) = (0, 0);
    for frame in 0..SLOW_FRAMES {
        for _ in 0..TICKS_PER_SLOW_FRAME {
            tick += 1;
            harness.enqueue_tick(tick);
        }
        harness.step(None);
        last = harness.send_lag(0);
        if frame >= 4 {
            peak = peak.max(last);
        }
    }
    let frames_for_each_tick = (harness.peers[0].issued_messages.len() - frames_before)
        .div_ceil((SLOW_FRAMES * TICKS_PER_SLOW_FRAME) as usize);
    eprintln!(
        "slow frames: old_limit={fixed_old_limit} loss_timeout={server_loss_timeout} peak_lag={peak} last_lag={last} frames_for_each_tick~{frames_for_each_tick} restart={:?}",
        harness.peers[0].restart_error
    );
    (peak, last, frames_for_each_tick)
}

/// One slow frame of ticks may wait for the next pump; more is a backlog.
const SLOW_FRAME_LAG_BOUND: u32 = 5;

#[test]
fn slow_frames_leave_no_send_backlog_with_the_scaled_limit_and_the_loss_timeout_floor() {
    let (peak, last, frames) = slow_frame_send_lag(false, true);
    assert!(
        frames >= 4,
        "the workload must need more than 16 frames in a slow frame"
    );
    assert!(peak <= SLOW_FRAME_LAG_BOUND, "peak send lag {peak} ticks");
    assert!(last <= SLOW_FRAME_LAG_BOUND, "final send lag {last} ticks");
}

/// The red controls of the test above: each part of the repair is necessary.
#[test]
fn slow_frames_build_a_send_backlog_without_each_part_of_the_repair() {
    for (fixed_old_limit, server_loss_timeout, label) in [
        (true, false, "old limit, default loss timeout (56a008b4)"),
        (false, false, "scaled limit, default loss timeout"),
        (true, true, "old limit, loss timeout floor"),
    ] {
        let (peak, _, _) = slow_frame_send_lag(fixed_old_limit, server_loss_timeout);
        assert!(
            peak > SLOW_FRAME_LAG_BOUND,
            "{label}: expected a backlog, peak send lag {peak} ticks"
        );
    }
}

#[test]
fn a_frame_with_no_tick_or_one_tick_keeps_the_limit_of_16_frames() {
    // The first baseline has more than 16 frames and no tick ran before the first pump.
    let mut harness = Harness::new(50, 1);
    harness.step(None);
    assert_eq!(harness.peers[0].issued_messages.len(), FRAMES_PER_PUMP);
    harness.enqueue_tick(1);
    harness.step(None);
    assert_eq!(harness.peers[0].issued_messages.len(), 2 * FRAMES_PER_PUMP);
}

#[test]
fn many_ticks_in_one_frame_stop_at_the_send_window_and_keep_the_order() {
    // 1000 moving units: 5 ticks need more than 64 frames.
    MOVING_UNITS.with(|units| units.set(1000));
    let mut harness = Harness::new(50, 1);
    harness.retain_received_frames = true;
    let mut tick = 0;
    while harness.now_ms < 40 * TICK_MS {
        if harness.now_ms > 0 && harness.now_ms.is_multiple_of(TICK_MS) {
            tick += 1;
            harness.enqueue_tick(tick);
        }
        harness.step(None);
    }
    let before = harness.peers[0].issued_messages.len();
    for _ in 0..5 {
        tick += 1;
        harness.enqueue_tick(tick);
    }
    harness.step(None);
    let server = harness.peers[0].server;
    let outstanding = |harness: &Harness| {
        harness
            .app
            .world()
            .get::<KnowledgeSender>(server)
            .unwrap()
            .outstanding
            .len()
    };
    assert_eq!(outstanding(&harness), WINDOW, "the window is the limit");
    assert_eq!(harness.peers[0].issued_messages.len() - before, WINDOW);
    assert!(
        harness.send_lag(0) > 0,
        "the rest waits for the acknowledgements"
    );
    for _ in 0..200 {
        harness.step(None);
    }
    assert_eq!(harness.send_lag(0), 0);
    assert_eq!(*harness.peers[0].activations.last().unwrap(), tick);
    assert!(
        harness.peers[0]
            .activations
            .windows(2)
            .all(|pair| pair[0] < pair[1]),
        "activation ticks must stay ordered"
    );
}
