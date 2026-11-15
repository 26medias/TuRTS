//! Frozen SV4 transport workload. All packet and message acknowledgements use Lightyear.
use super::*;
use lightyear::link::recv_payload_from_bytes;
use lightyear::prelude::server::{ClientOf, ServerPlugins};
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
fn pump_in_production_schedule(world: &mut World) {
    let clock = *world.resource::<PumpClock>();
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
        let error = pump_one(world, entity, &mut sender, clock.now).err();
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
    issued_messages: BTreeSet<u16>,
    max_batch: usize,
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
}
fn position(unit: u32, tick: u32) -> bevy::math::Vec2 {
    let motion = if unit <= 500 {
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
        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(TICK_MS),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.init_resource::<CapturedFrames>()
            .init_resource::<PumpReports>()
            .init_resource::<StoppedPeers>();
        app.add_systems(PreUpdate, capture_frames.after(MessageSystems::Receive));
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
                    issued_messages: BTreeSet::new(),
                    max_batch: 0,
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
        }
    }
    fn enqueue_tick(&mut self, tick: u32) {
        self.app
            .insert_resource(turts_sim::CompletedWorld(world_view(tick)));
        self.app.insert_resource(turts_sim::CommittedTick {
            tick,
            positions: (1..=500).map(|unit| (unit, position(unit, tick))).collect(),
            ..Default::default()
        });
        for peer in &self.peers {
            if peer.restart_error.is_none() {
                assert!(queue_tick(self.app.world_mut(), peer.server, SlotId(0)).unwrap());
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
        for peer in &mut self.peers {
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
        }
        self.app.insert_resource(PumpClock {
            epoch: self.epoch,
            now,
        });
        self.app.update();
        for peer in &mut self.peers {
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
            let (cancels, frames) = {
                let mut captured = self.app.world_mut().resource_mut::<CapturedFrames>();
                (
                    captured.cancels.remove(&peer.client).unwrap_or_default(),
                    captured.frames.remove(&peer.client).unwrap_or_default(),
                )
            };
            for cancel in cancels {
                peer.assembler.cancel(&cancel.stamp);
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
                            .saturating_sub(STEP_MS)
                            .max(self.now_ms);
                        peer.activation_acks.push_back((due, ack));
                    }
                }
            }
            peer.max_batch = peer.max_batch.max(activations);
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
                    let mut due = self.now_ms + STEP_MS;
                    if !returning
                        && self
                            .burst_at_ms
                            .is_some_and(|at| self.now_ms >= at && self.now_ms < at + TICK_MS)
                    {
                        due = self.burst_at_ms.unwrap() + TICK_MS;
                    }
                    self.packets.push_back((due, to, packet.to_vec()));
                }
            }
        }
        self.now_ms += STEP_MS;
    }
    fn evidence(&self, label: &str) {
        let rows = self.peers.iter().enumerate().map(|(index, peer)| serde_json::json!({
            "peer": index, "activation_ticks": peer.activations, "actual_transport_message_acks": peer.transport_acks,
            "actual_issued_message_ids": peer.issued_messages, "max_transactions_in_one_client_update": peer.max_batch,
            "max_unacknowledged_age_ms": peer.max_age_ms, "max_encoded_successor_bytes": peer.max_backlog,
            "first_measured_backlog_bytes": peer.first_measured_backlog, "final_backlog_bytes": peer.last_measured_backlog,
            "restart_error": peer.restart_error,
        })).collect::<Vec<_>>();
        let report = serde_json::json!({"label": label, "activation_ack_rtt_ms": self.rtt_ms, "application_step_ms": STEP_MS,
            "sender_hz": 20, "snapshot_units": 1000, "position_updates_per_tick": 500, "warmup_ticks": WARMUP_TICKS,
            "measured_ticks": MEASURED_TICKS, "one_delivery_burst_ms": TICK_MS, "peers": rows});
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
        }
    }
}

#[test]
fn frozen_20hz_pipeline_preserves_every_tick_at_50_and_100_ms_activation_ack_rtt() {
    for rtt_ms in [50, 100] {
        let mut harness = Harness::new(rtt_ms, 1);
        harness.burst_at_ms = Some(u64::from(WARMUP_TICKS + 20) * TICK_MS);
        while harness.now_ms <= u64::from(FINAL_TICK) * TICK_MS {
            if harness.now_ms > 0 && harness.now_ms % TICK_MS == 0 {
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
        if harness.now_ms > 0 && harness.now_ms % TICK_MS == 0 {
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
