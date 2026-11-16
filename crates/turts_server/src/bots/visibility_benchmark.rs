//! SV7: frozen real simulation workload, with atomic human and public bot consumers.
//! The short driver is diagnostic. Only the fixed 1,000-interval driver is acceptance.
#![cfg(test)]

use super::*;
use lightyear::transport::plugin::TransportSystems;
use lightyear::{
    link::recv_payload_from_bytes,
    prelude::server::{ClientOf, ServerPlugins},
    prelude::*,
};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use turts_client::{
    content::ClientContent,
    replica::{Replica, ReplicaContext},
};
use turts_mod_api::{
    BalanceAmount, ContentCatalog, CostSource, MapDef, MapSize, NodePlacement, Placement,
    ResolvedCost, SlotEconomyInit, SourceDef, VisibilityDef, VisionDef, VisionSharing,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_lightyear::{KNOWLEDGE_FRAME_LIMIT_BYTES, KnowledgeChannel, ReliableChannel};
use turts_net_protocol::{
    StateTransactionAck, StateTransactionCancel, StateTransactionPayload,
    visibility_transaction::{StateTransactionAssembler, StateTransactionFrame},
};
use turts_sim::{CommandQueue, CommittedTick, completed_world};

const WARMUP: usize = 100;
const REQUIRED: usize = 1000;
const HEALTH: u64 = 1_000_000_000_000;
const LONG_WORK_MS: u64 = 2_000_000;
fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("stockyard:{local}")).unwrap()
}
fn place(local: &str, slot: u8, x: f32, y: f32) -> Placement {
    Placement {
        unit: id(local).to_string(),
        owner_slot: slot,
        pos: WorldPos { x, y },
        contents: vec![],
        facing: None,
        start_order: None,
    }
}
pub(super) struct Fixture {
    pub(super) catalog: ContentCatalog,
    pub(super) map: MapDef,
    pub(super) authored: serde_json::Value,
}
pub(super) fn authored_fixture(fog: bool, concentrated: bool) -> Fixture {
    let bytes =
        pack_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard"))
            .unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut catalog = archive.content().unwrap();
    for unit in catalog.units.values_mut() {
        unit.vision = None;
        unit.provides.clear();
        unit.consumes.clear();
        unit.income = None;
        if let Some(hp) = &mut unit.health {
            hp.max = HEALTH;
        }
        unit.footprint = turts_mod_api::Footprint::Circle { radius: 0.3 };
        if let Some(movement) = &mut unit.movement {
            movement.speed = 100.;
        }
        if let Some(weapon) = &mut unit.weapon {
            weapon.range = 10.;
            weapon.cooldown_ms = 100;
        }
        if let Some(inventory) = &mut unit.inventory {
            inventory.capacity = 1_000_000_000;
        }
        if let Some(gatherer) = &mut unit.gatherer {
            gatherer.interaction_range = 16.;
        }
    }
    let mut sight = catalog.units[&id("soldier")].clone();
    sight.id = id("sv7_sight");
    sight.weapon = None;
    sight.acquire = None;
    sight.inventory = None;
    sight.gatherer = None;
    sight.vision = Some(VisionDef { radius: 12. });
    catalog.units.insert(sight.id.clone(), sight);
    let mut stationary = catalog.units[&id("soldier")].clone();
    stationary.id = id("sv7_stationary");
    stationary.movement = None;
    catalog.units.insert(stationary.id.clone(), stationary);
    let movement = catalog.orders.get_mut(&id("order_move")).unwrap();
    movement.units.push(id("sv7_sight").to_string());
    movement.fire = Some(turts_mod_api::FirePolicy::InRange);
    for recipe in catalog.production.values_mut() {
        recipe.duration_ms = LONG_WORK_MS;
        recipe.payment = turts_mod_api::PaymentPolicy::ChargeOverTime;
        recipe.costs = vec![ResolvedCost {
            resource: id("credits"),
            source: CostSource::SlotBalance,
            amount: 50_000,
        }];
    }
    for node in catalog.nodes.values_mut() {
        node.radius = 0.2;
    }
    let visibility = fog.then_some(VisibilityDef {
        cell_size: 2.,
        sharing: VisionSharing::OwnSlot,
        memory_limit: None,
    });
    let mut map = MapDef {
        visibility,
        terrain: None,
        id: "sv7".into(),
        environment: None,
        ground: None,
        size: MapSize { x: 128., y: 128. },
        placements: vec![],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![],
    };
    for slot in 0..2 {
        let lane = if concentrated {
            60. + f32::from(slot) * 16.
        } else {
            32. + f32::from(slot) * 64.
        };
        for n in 0..250 {
            map.placements.push(place(
                if n < 100 { "sv7_sight" } else { "soldier" },
                slot,
                26.75 + (n % 16) as f32 * 0.7,
                lane - 5.25 + (n / 16) as f32 * 0.7,
            ));
        }
        // Static enemies lie on the OTHER slot's ordinary scouting route.
        let foreign_lane = if concentrated {
            76. - f32::from(slot) * 16.
        } else {
            96. - f32::from(slot) * 64.
        };
        for n in 0..100 {
            map.placements.push(place(
                "sv7_stationary",
                slot,
                60.625 + (n % 10) as f32 * 0.75,
                foreign_lane - 3.375 + (n / 10) as f32 * 0.75,
            ));
            map.placements.push(place(
                "worker",
                slot,
                27. + (n % 10) as f32 * 0.75,
                18. + f32::from(slot) * 64. + (n / 10) as f32 * 0.75,
            ));
        }
        for n in 0..50 {
            map.placements.push(place(
                "barracks",
                slot,
                8. + (n % 10) as f32 * 0.75,
                8. + f32::from(slot) * 64. + (n / 10) as f32 * 0.75,
            ));
        }
        map.nodes.push(NodePlacement {
            node: id("trees").to_string(),
            pos: WorldPos {
                x: 40.,
                y: 24. + f32::from(slot) * 64.,
            },
            remaining: 1_000_000_000,
        });
        map.slot_economy.push(SlotEconomyInit {
            slot,
            balances: vec![BalanceAmount {
                balance: id("credits").to_string(),
                amount: 1_000_000_000,
            }],
        });
    }
    // These checks precede timing. No overlapping unit placements are admitted.
    assert_eq!(map.placements.len(), 1000);
    for (n, a) in map.placements.iter().enumerate() {
        for b in &map.placements[n + 1..] {
            assert!(
                (a.pos.x - b.pos.x).hypot(a.pos.y - b.pos.y) >= 0.6,
                "overlapping authored workload"
            );
        }
    }
    let authored = serde_json::json!({"source_archive_sha256":turts_mod_runtime::hash::sha256_hex(&bytes),
        "map":map,"fog":fog,"concentrated":concentrated,"health":HEALTH,"radius":0.3,
        "speed":100,"source_radius":12,"weapon_range":10,"weapon_cooldown_ms":100,
        "inventory_capacity":1_000_000_000u64,"gather_range":16,"cleared_supply_and_income":true,"cot_duration_ms":LONG_WORK_MS,
        "cot_balance_cost":50_000,"public_query_controllers":["sv7-units","sv7-economy"],"public_quota_per_controller":{"queries":32,"candidates":4096,"rows":1024,"proposals":32},"route_x":[32,80],"route_switch_ticks":10,
        "mix_per_slot":{"sight_movers":100,"armed_movers":150,"static_combatants":100,"gatherers":100,"producers":50}});
    Fixture {
        catalog,
        map,
        authored,
    }
}

#[derive(Resource, Default)]
struct Cpu {
    start: Option<Instant>,
    simulation_start: Option<Instant>,
    simulation_ns: u64,
    server_ns: u64,
    client_ns: u64,
    peer_client_ns: [u64; 2],
    client_batches_ns: [Vec<u64>; 2],
    stages_ns: [u64; 10],
    profiling: bool,
}
fn begin_simulation(mut cpu: ResMut<Cpu>) {
    assert!(cpu.simulation_start.replace(Instant::now()).is_none());
}
fn end_simulation(mut cpu: ResMut<Cpu>) {
    cpu.simulation_ns += cpu.simulation_start.take().unwrap().elapsed().as_nanos() as u64;
}
impl Cpu {
    fn full_server_service_ns(&self) -> u64 {
        // Refresh stages0..2 are already inside the direct simulation timer.
        // Queue, pump/send, bot queries, local link and shared receive execute
        // outside that timer in this harness. Each is charged exactly once.
        self.simulation_ns + self.stages_ns[3..=7].iter().sum::<u64>()
    }
}
fn begin_cpu(mut cpu: ResMut<Cpu>) {
    cpu.start = Some(Instant::now());
}
fn finish_server(cpu: &mut Cpu, stage: usize) {
    if let Some(t) = cpu.start.take() {
        let elapsed = t.elapsed().as_nanos() as u64;
        cpu.server_ns += elapsed;
        cpu.stages_ns[stage] += elapsed;
    }
}
fn end_start(mut cpu: ResMut<Cpu>) {
    finish_server(&mut cpu, 0);
}
fn end_tactical(mut cpu: ResMut<Cpu>) {
    finish_server(&mut cpu, 1);
}
fn end_completed(mut cpu: ResMut<Cpu>) {
    finish_server(&mut cpu, 2);
}
fn end_send(mut cpu: ResMut<Cpu>) {
    finish_server(&mut cpu, 4);
}
fn end_client(mut cpu: ResMut<Cpu>) {
    if let Some(t) = cpu.start.take() {
        let elapsed = t.elapsed().as_nanos() as u64;
        cpu.client_ns += elapsed;
        cpu.stages_ns[7] += elapsed;
    }
}
fn charge_server(world: &mut World, stage: usize, start: Instant) {
    let elapsed = start.elapsed().as_nanos() as u64;
    let mut cpu = world.resource_mut::<Cpu>();
    cpu.server_ns += elapsed;
    cpu.stages_ns[stage] += elapsed;
}
#[derive(Resource, Default)]
struct Received {
    frames: BTreeMap<Entity, Vec<StateTransactionFrame>>,
    cancels: BTreeMap<Entity, Vec<StateTransactionCancel>>,
}
fn capture_frames(
    mut received: ResMut<Received>,
    mut frames: bevy::prelude::Query<(Entity, &mut MessageReceiver<StateTransactionFrame>)>,
    mut cancels: bevy::prelude::Query<
        (Entity, &mut MessageReceiver<StateTransactionCancel>),
        With<Client>,
    >,
) {
    for (entity, mut receiver) in &mut frames {
        received
            .frames
            .entry(entity)
            .or_default()
            .extend(receiver.receive());
    }
    for (entity, mut receiver) in &mut cancels {
        received
            .cancels
            .entry(entity)
            .or_default()
            .extend(receiver.receive());
    }
}
struct Peer {
    slot: u8,
    server: Entity,
    client: Entity,
    assembler: StateTransactionAssembler,
    replica: Replica,
    latest: u32,
    hides: usize,
    reveals: usize,
    max_retained: usize,
    max_server_retained: usize,
    permitted_shots: u64,
    wire_bytes: u64,
}
pub(super) struct Harness {
    pub(super) app: App,
    peers: Vec<Peer>,
    pub(super) catalog: Arc<ContentCatalog>,
    pub(super) authored: serde_json::Value,
    seq: [u32; 2],
    fog: bool,
    concentrated: bool,
    frame_ms: u64,
    ack_clock_ms: u64,
    ack_rtt_ms: u64,
    delayed_acks: Vec<(u64, Entity, StateTransactionAck)>,
}
impl Harness {
    fn new(fog: bool, concentrated: bool) -> Self {
        Self::with_fixture(authored_fixture(fog, concentrated), fog, concentrated, None)
    }
    pub(super) fn with_fixture(
        fixture: Fixture,
        fog: bool,
        concentrated: bool,
        terrain: Option<(
            turts_mod_api::ResolvedTerrainDef,
            turts_sim::terrain::PrivateTerrainSeed,
        )>,
    ) -> Self {
        let map = ContentId::parse(&format!("stockyard:{}", fixture.map.id)).unwrap();
        let client_terrain = fixture.map.terrain.as_ref().map(|definition| {
            Arc::new(turts_client::terrain::store::Layout::new(map.clone(), definition).unwrap())
        });
        let hash =
            turts_mod_runtime::hash::sha256_hex(&serde_json::to_vec(&fixture.authored).unwrap());
        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.add_plugins(turts_sim::SimPlugin { tick_hz: 20 });
        app.insert_resource(turts_sim::MatchConfig::immediate(20));
        app.init_resource::<crate::stats::Stats>();
        app.insert_resource(crate::stats::MatchId(91));
        crate::session::install(
            &mut app,
            crate::session::Advertised {
                lockfile_hash: hash.clone(),
                tick_hz: 20,
            },
        );
        let source = SourceDef {
            file: format!("stockyard/maps/{}.ron", fixture.map.id),
            value: fixture.map,
        };
        let staged = if let Some((definition, seed)) = terrain {
            turts_sim::stage_package_with_terrain(
                app.world(),
                fixture.catalog.clone(),
                source,
                20,
                map.clone(),
                Some(definition),
                Some(seed),
            )
        } else {
            turts_sim::stage_package(app.world(), fixture.catalog.clone(), source, 20)
        }
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        app.insert_resource(turts_sim::InstalledMapIdentity(map.clone()));
        turts_sim::visibility::initialize_context(app.world_mut(), 91, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        app.insert_resource(ServerCatalog(fixture.catalog.clone()));
        crate::outbound::knowledge::install(&mut app);
        app.init_resource::<Cpu>();
        app.init_resource::<Received>();
        app.add_systems(FixedPreUpdate, begin_simulation.before(SimSet::Advance));
        app.add_systems(FixedPostUpdate, end_simulation.after(SimSet::Collect));
        app.add_systems(
            PreUpdate,
            capture_frames
                .after(MessageSystems::Receive)
                .before(end_client),
        );
        // Boundary sets bracket the actual refresh without timing adjacent collectors
        // or full-world publication. All publication/encoding/client work stays charged.
        app.add_systems(
            FixedPreUpdate,
            begin_cpu
                .in_set(SimSet::Advance)
                .after(turts_sim::plugin::VisibilityBoundary::StartBefore)
                .before(turts_sim::visibility::refresh_start),
        );
        app.add_systems(
            FixedPreUpdate,
            end_start
                .in_set(SimSet::Advance)
                .after(turts_sim::visibility::refresh_start)
                .after(begin_cpu),
        );
        app.add_systems(
            FixedUpdate,
            begin_cpu
                .in_set(SimSet::Acquire)
                .before(turts_sim::visibility::refresh_tactical),
        );
        app.add_systems(
            FixedUpdate,
            end_tactical
                .in_set(SimSet::Acquire)
                .after(turts_sim::visibility::refresh_tactical)
                .before(turts_sim::plugin::VisibilityBoundary::TacticalAfter)
                .after(begin_cpu),
        );
        app.add_systems(
            FixedPostUpdate,
            begin_cpu
                .in_set(SimSet::Collect)
                .after(turts_sim::plugin::VisibilityBoundary::CompletedBefore)
                .before(turts_sim::visibility::refresh_completed),
        );
        app.add_systems(
            FixedPostUpdate,
            end_completed
                .in_set(SimSet::Collect)
                .after(turts_sim::visibility::refresh_completed)
                .before(turts_sim::plugin::VisibilityBoundary::CompletedAfter)
                .after(begin_cpu),
        );
        app.add_systems(
            PreUpdate,
            begin_cpu
                .before(TransportSystems::Receive)
                .before(MessageSystems::Receive),
        );
        app.add_systems(
            PreUpdate,
            end_client
                .after(TransportSystems::Receive)
                .after(MessageSystems::Receive)
                .after(begin_cpu),
        );
        app.add_systems(
            PostUpdate,
            begin_cpu.before(crate::outbound::knowledge::pump),
        );
        app.add_systems(
            PostUpdate,
            end_send
                .after(MessageSystems::Send)
                .after(TransportSystems::Send)
                .after(begin_cpu),
        );
        app.finish();
        app.cleanup();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));
        app.update();
        let peers = (0..2)
            .map(|slot| {
                let registry = app.world().resource::<ChannelRegistry>();
                let mut outgoing = Transport::default();
                outgoing.add_channel_send_from_registry::<KnowledgeChannel>(registry);
                outgoing.add_channel_send_from_registry::<ReliableChannel>(registry);
                outgoing.add_channel_receive_from_registry::<ReliableChannel>(registry);
                let mut incoming = Transport::default();
                incoming.add_channel_receive_from_registry::<KnowledgeChannel>(registry);
                incoming.add_channel_send_from_registry::<ReliableChannel>(registry);
                incoming.add_channel_receive_from_registry::<ReliableChannel>(registry);
                let server = app
                    .world_mut()
                    .spawn((
                        ClientOf,
                        Link::default(),
                        outgoing,
                        RemoteId(PeerId::Local(101 + u64::from(slot))),
                        Linked,
                        Connected,
                        crate::session::Session {
                            last_seq: None,
                            slot: turts_sim::SlotId(slot),
                            player_id: 101 + u64::from(slot),
                        },
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
                        incoming,
                        RemoteId(PeerId::Server),
                        Linked,
                        Connected,
                        MessageReceiver::<StateTransactionFrame>::default(),
                        MessageReceiver::<StateTransactionCancel>::default(),
                        MessageSender::<StateTransactionAck>::default(),
                    ))
                    .id();
                let mut replica = Replica::new(20);
                replica.set_context(ReplicaContext::new(
                    ClientContent {
                        catalog: fixture.catalog.clone(),
                        terrain: client_terrain.clone(),
                        ..Default::default()
                    },
                    slot,
                ));
                assert!(
                    crate::outbound::knowledge::queue_oversized(
                        app.world_mut(),
                        server,
                        turts_sim::SlotId(slot)
                    )
                    .unwrap()
                );
                Peer {
                    slot,
                    server,
                    client,
                    assembler: StateTransactionAssembler::new(
                        91,
                        map.clone(),
                        KNOWLEDGE_FRAME_LIMIT_BYTES,
                    ),
                    replica,
                    latest: 0,
                    hides: 0,
                    reveals: 0,
                    max_retained: 0,
                    max_server_retained: 0,
                    permitted_shots: 0,
                    wire_bytes: 0,
                }
            })
            .collect();
        let mut harness = Self {
            app,
            peers,
            catalog: Arc::new(fixture.catalog),
            authored: serde_json::json!({"sha256":hash,"inputs":fixture.authored}),
            seq: [0, 0],
            fog,
            concentrated,
            frame_ms: 0,
            ack_clock_ms: 0,
            ack_rtt_ms: 0,
            delayed_acks: Vec::new(),
        };
        harness.flush_to(0);
        harness
    }
    pub(super) fn set_ack_rtt(&mut self, rtt_ms: u64) {
        assert!(matches!(rtt_ms, 50 | 100));
        self.ack_rtt_ms = rtt_ms;
    }
    fn send_due_acks(&mut self) {
        let mut index = 0;
        while index < self.delayed_acks.len() {
            if self.delayed_acks[index].0 <= self.ack_clock_ms {
                let (_, client, ack) = self.delayed_acks.remove(index);
                self.app
                    .world_mut()
                    .get_mut::<MessageSender<StateTransactionAck>>(client)
                    .unwrap()
                    .send::<ReliableChannel>(ack);
            } else {
                index += 1;
            }
        }
    }
    fn advance_ack_tail(&mut self) {
        if let Some(next) = self.delayed_acks.iter().map(|(due, _, _)| *due).min() {
            self.ack_clock_ms = self.ack_clock_ms.max(next);
        }
    }
    pub(super) fn set_profiling(&mut self, enabled: bool) {
        self.app.world_mut().resource_mut::<Cpu>().profiling = enabled;
    }
    pub(super) fn command(
        &mut self,
        slot: u8,
        local: &str,
        units: Vec<u32>,
        target: turts_net_protocol::OrderTarget,
    ) {
        self.seq[usize::from(slot)] += 1;
        let seq = self.seq[usize::from(slot)];
        let wire = turts_net_protocol::PlayerCommand {
            seq,
            order: turts_net_protocol::OrderEnvelope {
                order: id(local).to_string(),
                units: units.into_iter().map(turts_net_protocol::UnitId).collect(),
                target,
                queue: turts_net_protocol::QueueMode::Replace,
                group: seq,
            },
        };
        let resolved =
            crate::orders::to_sim(turts_sim::SlotId(slot), &wire, &self.catalog).unwrap();
        self.app
            .world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .extend(resolved);
    }
    fn orders(&mut self, tick: u32) {
        for slot in 0..2 {
            let units = completed_world(self.app.world())
                .units
                .iter()
                .filter(|u| u.owner_slot == slot)
                .cloned()
                .collect::<Vec<_>>();
            if (tick - 1).is_multiple_of(10) {
                let movers = units
                    .iter()
                    .filter(|u| {
                        u.content_id == id("soldier").to_string()
                            || u.content_id == id("sv7_sight").to_string()
                    })
                    .map(|u| u.unit_id)
                    .collect();
                let lane = if self.concentrated {
                    60. + f32::from(slot) * 16.
                } else {
                    32. + f32::from(slot) * 64.
                };
                self.command(
                    slot,
                    "order_move",
                    movers,
                    turts_net_protocol::OrderTarget::Point(turts_net_protocol::Pos {
                        x: if ((tick - 1) / 10).is_multiple_of(2) {
                            80.
                        } else {
                            32.
                        },
                        y: lane,
                    }),
                );
            }
            if (tick - 1).is_multiple_of(20) {
                let node = completed_world(self.app.world())
                    .nodes
                    .iter()
                    .min_by(|a, b| {
                        (a.pos.y - (24. + f32::from(slot) * 64.))
                            .abs()
                            .total_cmp(&(b.pos.y - (24. + f32::from(slot) * 64.)).abs())
                    })
                    .unwrap()
                    .id
                    .0;
                self.command(
                    slot,
                    "order_gather",
                    units
                        .iter()
                        .filter(|u| u.content_id == id("worker").to_string())
                        .map(|u| u.unit_id)
                        .collect(),
                    turts_net_protocol::OrderTarget::Node(turts_net_protocol::NodeId(node)),
                );
            }
            if tick == 1 {
                self.command(
                    slot,
                    "order_produce",
                    units
                        .iter()
                        .filter(|u| u.content_id == id("barracks").to_string())
                        .map(|u| u.unit_id)
                        .collect(),
                    turts_net_protocol::OrderTarget::Content(id("train_soldier").to_string()),
                );
            }
        }
    }
    fn consume(&mut self, shared_transport_ns: u64) {
        let started = Instant::now();
        let profiling = self.app.world().resource::<Cpu>().profiling;
        let mut assembly_ns = 0;
        let mut activation_ns = 0;
        let mut peer_batches = [0; 2];
        for peer in &mut self.peers {
            let peer_started = Instant::now();
            let cancels = self
                .app
                .world_mut()
                .resource_mut::<Received>()
                .cancels
                .remove(&peer.client)
                .unwrap_or_default();
            for cancel in cancels {
                peer.assembler.cancel(&cancel.stamp);
            }
            let frames = self
                .app
                .world_mut()
                .resource_mut::<Received>()
                .frames
                .remove(&peer.client)
                .unwrap_or_default();
            for frame in frames {
                let stamp = match &frame {
                    StateTransactionFrame::Begin(f) => &f.stamp,
                    StateTransactionFrame::Chunk(f) => &f.stamp,
                    StateTransactionFrame::End(f) => &f.stamp,
                };
                if peer.assembler.is_obsolete(stamp) {
                    continue;
                }
                let mut assembly_start = profiling.then(Instant::now);
                match frame {
                    StateTransactionFrame::Begin(begin) => {
                        peer.assembler.begin(begin, self.frame_ms).unwrap();
                    }
                    StateTransactionFrame::Chunk(chunk) => peer.assembler.chunk(&chunk).unwrap(),
                    StateTransactionFrame::End(end) => {
                        let staged = peer.assembler.end(&end).unwrap().unwrap();
                        if let Some(start) = assembly_start.take() {
                            assembly_ns += start.elapsed().as_nanos() as u64;
                        }
                        if let StateTransactionPayload::Patch { state, .. } = &staged.payload {
                            peer.permitted_shots += state
                                .records
                                .effects
                                .iter()
                                .filter(|e| matches!(e, turts_net_protocol::Effect::Fired { .. }))
                                .count() as u64;
                        }
                        if let StateTransactionPayload::Patch {
                            knowledge: Some(knowledge),
                            ..
                        } = &staged.payload
                        {
                            peer.hides += knowledge.hidden_units.len();
                            peer.reveals += knowledge.revealed_units.len();
                        }
                        let activation_start = profiling.then(Instant::now);
                        assert!(
                            peer.replica.activate_transaction(
                                &staged.stamp,
                                &staged.payload,
                                bevy::platform::time::Instant::now()
                            ),
                            "native replica refused tick{}",
                            staged.stamp.basis.tick
                        );
                        if let Some(start) = activation_start {
                            activation_ns += start.elapsed().as_nanos() as u64;
                        }
                        peer.latest = staged.stamp.basis.tick;
                        peer.replica.visual_events.clear();
                        let ack = peer.assembler.activate(&staged.stamp).unwrap();
                        if self.ack_rtt_ms == 0 {
                            self.app
                                .world_mut()
                                .get_mut::<MessageSender<StateTransactionAck>>(peer.client)
                                .unwrap()
                                .send::<ReliableChannel>(ack);
                        } else {
                            self.delayed_acks.push((
                                self.ack_clock_ms + self.ack_rtt_ms,
                                peer.client,
                                ack,
                            ));
                        }
                    }
                }
                if let Some(start) = assembly_start {
                    assembly_ns += start.elapsed().as_nanos() as u64;
                }
                peer.max_retained = peer.max_retained.max(peer.assembler.retained_bytes());
            }
            peer.max_retained = peer.max_retained.max(peer.assembler.retained_bytes());
            // Server memory accounting is a harness probe, not client service.
            peer_batches[usize::from(peer.slot)] =
                peer_started.elapsed().as_nanos() as u64 + shared_transport_ns;
            peer.max_server_retained = peer.max_server_retained.max(
                self.app
                    .world()
                    .get::<crate::outbound::knowledge::KnowledgeSender>(peer.server)
                    .unwrap()
                    .benchmark_retained(),
            );
        }
        let mut cpu = self.app.world_mut().resource_mut::<Cpu>();
        cpu.client_ns += started.elapsed().as_nanos() as u64;
        for (slot, batch) in peer_batches.into_iter().enumerate() {
            cpu.peer_client_ns[slot] += batch;
            cpu.client_batches_ns[slot].push(batch);
        }
        cpu.stages_ns[8] += assembly_ns;
        cpu.stages_ns[9] += activation_ns;
    }
    fn transfer(&mut self) {
        let started = Instant::now();
        for peer in &mut self.peers {
            for (from, to) in [(peer.server, peer.client), (peer.client, peer.server)] {
                let packets = {
                    let mut link = self.app.world_mut().get_mut::<Link>(from).unwrap();
                    let mut packets = Vec::new();
                    while let Some(packet) = link.send.pop() {
                        packets.push(packet);
                    }
                    packets
                };
                for packet in packets {
                    peer.wire_bytes += packet.len() as u64;
                    self.app
                        .world_mut()
                        .get_mut::<Link>(to)
                        .unwrap()
                        .recv
                        .push_raw(recv_payload_from_bytes(packet));
                }
            }
        }
        charge_server(self.app.world_mut(), 6, started);
    }
    fn trace_update(&self, step_ms: u64, phase: &str) {
        let Some(directory) =
            std::env::var_os("TURTS_TRANSFER_TRACE_DIR").map(std::path::PathBuf::from)
        else {
            return;
        };
        use std::io::Write;
        std::fs::create_dir_all(&directory).unwrap();
        let cpu = self.app.world().resource::<Cpu>();
        let peers = self.peers.iter().map(|peer| serde_json::json!({
            "slot":peer.slot,"latest":peer.latest,"assembler_retained":peer.assembler.retained_bytes(),
            "sender":self.app.world().get::<crate::outbound::knowledge::KnowledgeSender>(peer.server)
                .map(|sender|sender.benchmark_transfer_trace()),
        })).collect::<Vec<_>>();
        let row = serde_json::json!({"phase":phase,"actual_tick":completed_world(self.app.world()).tick,
            "frame_ms":self.frame_ms,"step_ms":step_ms,"full_service_ns":cpu.full_server_service_ns(),
            "simulation_ns":cpu.simulation_ns,"stages_ns":cpu.stages_ns,"peers":peers});
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("updates.jsonl"))
            .unwrap();
        serde_json::to_writer(&mut file, &row).unwrap();
        writeln!(file).unwrap();
    }
    fn update(&mut self, ms: u64) {
        self.frame_ms += ms;
        self.ack_clock_ms += ms;
        if self.ack_rtt_ms != 0 {
            let started = Instant::now();
            self.send_due_acks();
            charge_server(self.app.world_mut(), 6, started);
        }
        self.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(ms),
            ));
        let before = self.app.world().resource::<Cpu>().stages_ns;
        self.trace_update(ms, "before_app");
        self.app.update();
        self.trace_update(ms, "after_app");
        let after = self.app.world().resource::<Cpu>().stages_ns;
        // The app contains both endpoints. Charge all shared receive and send
        // work to each client rather than undercount its decoder or ACK sender.
        self.consume(after[7] - before[7] + after[4] - before[4]);
        self.transfer();
        self.trace_update(ms, "after_receive");
        if std::env::var_os("TURTS_TRANSFER_TRACE_DIR").is_some()
            && let Ok(limit) = std::env::var("TURTS_TRANSFER_TRACE_TICKS")
                .unwrap_or_else(|_| "2".into())
                .parse::<u32>()
            && ms > 0
            && completed_world(self.app.world()).tick >= limit
        {
            panic!("diagnostic transfer trace stopped after {limit} ticks");
        }
    }
    fn flush_to(&mut self, tick: u32) {
        let trace = std::env::var_os("TURTS_TRANSFER_TRACE_DIR").map(std::path::PathBuf::from);
        if let Some(directory) = &trace {
            std::fs::create_dir_all(directory).unwrap();
            for peer in &self.peers {
                let sender = self
                    .app
                    .world()
                    .get::<crate::outbound::knowledge::KnowledgeSender>(peer.server)
                    .unwrap();
                let path = directory.join(format!("first-slot{}-tick{tick}.postcard", peer.slot));
                if !path.exists() {
                    let payload = sender.benchmark_snapshot_payload();
                    let summary = sender.benchmark_transfer_trace();
                    let stamp =
                        serde_json::from_value::<turts_net_protocol::StateTransactionStamp>(
                            summary["pending"]["begin"]["stamp"].clone(),
                        )
                        .unwrap();
                    let mut probe = peer.replica.clone();
                    let accepted = probe.activate_transaction(
                        &stamp,
                        &payload,
                        bevy::platform::time::Instant::now(),
                    );
                    std::fs::write(
                        directory.join(format!("first-slot{}-tick{tick}-probe.json", peer.slot)),
                        serde_json::to_vec(
                            &serde_json::json!({"accepted":accepted,"summary":summary}),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    std::fs::write(
                        path,
                        postcard::to_allocvec(&sender.benchmark_snapshot_payload()).unwrap(),
                    )
                    .unwrap();
                    std::fs::write(
                        directory.join(format!("first-slot{}-tick{tick}.json", peer.slot)),
                        serde_json::to_vec(&sender.benchmark_snapshot_payload()).unwrap(),
                    )
                    .unwrap();
                }
            }
        }
        for pump in 0..2048 {
            if self.ack_rtt_ms != 0 {
                self.advance_ack_tail();
            }
            self.update(0);
            if let Some(directory) = &trace {
                let rows = self.peers.iter().map(|peer| serde_json::json!({
                    "slot":peer.slot,"latest":peer.latest,"assembler_retained":peer.assembler.retained_bytes(),
                    "sender":self.app.world().get::<crate::outbound::knowledge::KnowledgeSender>(peer.server)
                        .map(|sender|sender.benchmark_transfer_trace()),
                })).collect::<Vec<_>>();
                std::fs::write(directory.join(format!("pump-{pump:03}.json")),
                    serde_json::to_vec(&serde_json::json!({"pump":pump,"target_tick":tick,
                        "actual_tick":completed_world(self.app.world()).tick,"frame_ms":self.frame_ms,"peers":rows})).unwrap()).unwrap();
            }
            if self.drained_to(tick) {
                return;
            }
            if trace.is_some() && pump == 31 {
                panic!("diagnostic transfer trace stopped after32 pumps");
            }
        }
        panic!("atomic publication did not drain to tick{tick}");
    }
    /// Publish each completed tick while application ACKs remain delayed. The actual
    /// ACK tail drains at the final tick, without granting another service period.
    fn flush_delivered_to(&mut self, tick: u32) {
        for _ in 0..2048 {
            self.update(0);
            if self.peers.iter().all(|peer| {
                peer.latest == tick
                    && [peer.server, peer.client].iter().all(|entity| {
                        let link = self.app.world().get::<Link>(*entity).unwrap();
                        link.recv.len() == 0 && link.send.len() == 0
                    })
            }) {
                return;
            }
        }
        panic!("delayed ACK publication failed to deliver tick{tick}");
    }
    fn drained_to(&self, tick: u32) -> bool {
        self.peers.iter().all(|peer| {
            let sender = self
                .app
                .world()
                .get::<crate::outbound::knowledge::KnowledgeSender>(peer.server)
                .unwrap();
            peer.latest == tick
                && peer.assembler.active().is_some()
                && sender.active_stamp() == peer.assembler.active()
                && sender.benchmark_drained()
                && [peer.server, peer.client].iter().all(|entity| {
                    let link = self.app.world().get::<Link>(*entity).unwrap();
                    link.recv.len() == 0 && link.send.len() == 0
                })
        })
    }
}

#[derive(serde::Serialize)]
pub(super) struct Sample {
    pub(super) server_ns: u64,
    pub(super) client_ns: u64,
    pub(super) sum_ns: u64,
    pub(super) stages_ns: [u64; 10],
    pub(super) full_server_tick_ns: Vec<u64>,
    pub(super) client_tick_ns: [Vec<u64>; 2],
    pub(super) client_batches_ns: [Vec<u64>; 2],
    pub(super) fired: u64,
    pub(super) permitted_shots: [u64; 2],
    pub(super) extracted: u64,
    pub(super) moved: usize,
    pub(super) hides: [usize; 2],
    pub(super) reveals: [usize; 2],
    pub(super) retained_bytes: [usize; 2],
    pub(super) wire_bytes: [u64; 2],
}
impl Harness {
    fn bots(&mut self) {
        use turts_bot::{BotQueries, PageState, Query, UnitQuery};
        let started = Instant::now();
        let policy = crate::permissions::completed(self.app.world());
        let view = completed_world(self.app.world());
        for slot in 0..2 {
            let basis = projection::project(
                view,
                slot,
                u64::from(view.tick),
                self.catalog.clone(),
                vec![0, 1],
                WorldPos { x: 128., y: 128. },
                policy.0.as_ref(),
            )
            .basis;
            let keys = basis
                .accounts
                .iter()
                .map(|row| row.account.clone())
                .collect::<Vec<_>>();
            let mut coordinator = turts_bot::BotCoordinator::new(
                turts_bot::BotActor {
                    slot,
                    profile: id("opponent"),
                },
                turts_bot::CoordinatorLimits::default(),
            );
            coordinator
                .reconcile(
                    basis.id,
                    &turts_bot::ReconcileBatch::initial(
                        turts_bot::Watermark {
                            completed_tick: view.tick,
                            revision: u64::from(view.tick),
                        },
                        basis.accounts.clone(),
                    ),
                )
                .unwrap();
            let mut lease = observation::CandidateLease::new(basis, u64::from(view.tick));
            let requested = vec![
                Query::Units(UnitQuery::own(vec![])),
                Query::RememberedUnits(UnitQuery {
                    relation: turts_mod_api::SlotRelation::Enemy,
                    content: vec![],
                    center: None,
                    radius: None,
                }),
                Query::Units(UnitQuery {
                    relation: turts_mod_api::SlotRelation::Enemy,
                    content: vec![],
                    center: None,
                    radius: None,
                }),
                Query::Nodes(turts_bot::NodeQuery {
                    resources: vec![],
                    center: None,
                    radius: None,
                }),
                Query::RememberedNodes(turts_bot::NodeQuery {
                    resources: vec![],
                    center: None,
                    radius: None,
                }),
            ];
            // Public account queries accept at most64 keys. The complete private
            // reconciliation above remains one atomic batch, never key-chunked.
            let mut financial = vec![Query::Works];
            financial.extend(keys.chunks(64).map(|keys| Query::Accounts(keys.to_vec())));
            for (controller, requested) in [("sv7-units", requested), ("sv7-economy", financial)] {
                let mut queries = lease.queries(controller, operator_quota());
                queries.pin_basis(u64::from(view.tick)).unwrap();
                for query in requested {
                    let mut cursor = None;
                    loop {
                        let page =
                            queries
                                .query(query.clone(), cursor, 64)
                                .unwrap_or_else(|error| {
                                    panic!("public SV7 query {query:?}: {error:?}")
                                });
                        match page.state {
                            PageState::More(next) => cursor = Some(next),
                            PageState::Complete => break,
                            PageState::UnresolvedTie(_) => panic!("frozen query physical tie"),
                        }
                    }
                }
                queries.release_basis(u64::from(view.tick)).unwrap();
            }
            std::hint::black_box(coordinator);
        }
        charge_server(self.app.world_mut(), 5, started);
    }
    fn interval(&mut self, index: usize, resync: bool) -> Sample {
        self.interval_impl(index, resync, None::<fn(&mut Self, u32, bool)>)
    }
    /// Optional workload orders/counters run on each real authoritative tick.
    pub(super) fn interval_with(
        &mut self,
        index: usize,
        resync: bool,
        hook: impl FnMut(&mut Self, u32, bool),
    ) -> Sample {
        self.interval_impl(index, resync, Some(hook))
    }
    fn interval_impl(
        &mut self,
        index: usize,
        resync: bool,
        mut hook: Option<impl FnMut(&mut Self, u32, bool)>,
    ) -> Sample {
        let profiling = self.app.world().resource::<Cpu>().profiling;
        crate::bot_profile::activate(profiling && resync);
        *self.app.world_mut().resource_mut::<Cpu>() = Cpu {
            profiling,
            ..Default::default()
        };
        for peer in &mut self.peers {
            peer.hides = 0;
            peer.permitted_shots = 0;
            peer.reveals = 0;
            peer.wire_bytes = 0;
        }
        let mut fired = 0;
        let mut extracted = 0;
        let mut moved = BTreeSet::new();
        let mut full_server_tick_ns = Vec::with_capacity(20);
        let mut client_tick_ns: [Vec<u64>; 2] = std::array::from_fn(|_| Vec::with_capacity(20));
        for step in 0..20 {
            let cpu = self.app.world().resource::<Cpu>();
            let service_before = cpu.full_server_service_ns();
            let clients_before = cpu.peer_client_ns;
            let next = completed_world(self.app.world()).tick + 1;
            self.orders(next);
            if let Some(hook) = hook.as_mut() {
                let started = Instant::now();
                hook(self, next, false);
                charge_server(self.app.world_mut(), 5, started);
            }
            let before = completed_world(self.app.world())
                .units
                .iter()
                .map(|u| (u.unit_id, u.pos))
                .collect::<BTreeMap<_, _>>();
            self.update(50);
            let tick = completed_world(self.app.world()).tick;
            assert_eq!(tick, next, "one authoritative tick per workload step");
            let view = completed_world(self.app.world());
            assert_eq!(view.units.len(), 1000, "no workload death or early output");
            for unit in &view.units {
                if before.get(&unit.unit_id).is_some_and(|p| *p != unit.pos) {
                    moved.insert(unit.unit_id);
                }
            }
            let jobs = view
                .economy
                .queues
                .iter()
                .map(|q| q.jobs.len())
                .sum::<usize>();
            assert_eq!(jobs, 100, "all 100 real COT jobs remain active");
            for event in &self.app.world().resource::<CommittedTick>().events {
                match event.kind {
                    turts_sim::EconomyEventKind::Fired { .. } => fired += 1,
                    turts_sim::EconomyEventKind::Extracted { amount, .. } => extracted += amount,
                    _ => {}
                }
            }
            if let Some(hook) = hook.as_mut() {
                hook(self, tick, true);
            }
            let start = Instant::now();
            for peer in &self.peers {
                assert!(
                    crate::outbound::knowledge::queue_tick(
                        self.app.world_mut(),
                        peer.server,
                        turts_sim::SlotId(peer.slot)
                    )
                    .unwrap()
                );
            }
            charge_server(self.app.world_mut(), 3, start);
            // Drain actual registered transport and native activation. No work is
            // deferred outside the sample or silently coalesced into a later tick.
            if self.ack_rtt_ms == 0 || step == 19 {
                self.flush_to(tick);
            } else {
                self.flush_delivered_to(tick);
            }
            if tick.is_multiple_of(20) {
                self.bots();
            }
            if step == 19 && resync && (index + 1).is_multiple_of(100) {
                let start = Instant::now();
                for peer in &self.peers {
                    assert!(
                        crate::outbound::knowledge::queue_oversized(
                            self.app.world_mut(),
                            peer.server,
                            turts_sim::SlotId(peer.slot)
                        )
                        .unwrap()
                    );
                }
                charge_server(self.app.world_mut(), 3, start);
                let tick = completed_world(self.app.world()).tick;
                // Same-tick resync must actually activate the new transaction as well.
                let old = self
                    .peers
                    .iter()
                    .map(|p| p.assembler.active().unwrap().transaction_id)
                    .collect::<Vec<_>>();
                for _ in 0..2048 {
                    if self.ack_rtt_ms != 0 {
                        self.advance_ack_tail();
                    }
                    self.update(0);
                    if self.drained_to(tick)
                        && self.peers.iter().zip(&old).all(|(p, id)| {
                            p.latest == tick
                                && p.assembler.active().unwrap().transaction_id > *id
                                && self
                                    .app
                                    .world()
                                    .get::<crate::outbound::knowledge::KnowledgeSender>(p.server)
                                    .unwrap()
                                    .active_stamp()
                                    == p.assembler.active()
                        })
                    {
                        break;
                    }
                }
                assert!(
                    self.peers.iter().zip(old).all(|(p, id)| p
                        .assembler
                        .active()
                        .unwrap()
                        .transaction_id
                        > id),
                    "full resync did not finish"
                );
                assert!(
                    self.drained_to(tick),
                    "resync activation ACK tail must be charged in this interval"
                );
            }
            let cpu = self.app.world().resource::<Cpu>();
            full_server_tick_ns.push(cpu.full_server_service_ns() - service_before);
            for slot in 0..2 {
                client_tick_ns[slot].push(cpu.peer_client_ns[slot] - clients_before[slot]);
            }
        }
        let cpu = self.app.world().resource::<Cpu>();
        let sample = Sample {
            server_ns: cpu.server_ns,
            client_ns: cpu.client_ns,
            sum_ns: cpu.server_ns + cpu.client_ns,
            stages_ns: cpu.stages_ns,
            full_server_tick_ns,
            client_tick_ns,
            client_batches_ns: cpu.client_batches_ns.clone(),
            fired,
            permitted_shots: std::array::from_fn(|i| self.peers[i].permitted_shots),
            extracted,
            moved: moved.len(),
            hides: std::array::from_fn(|i| self.peers[i].hides),
            reveals: std::array::from_fn(|i| self.peers[i].reveals),
            retained_bytes: std::array::from_fn(|i| {
                self.peers[i].max_server_retained + self.peers[i].max_retained
            }),
            wire_bytes: std::array::from_fn(|i| self.peers[i].wire_bytes),
        };
        assert_eq!(sample.moved, 500, "all designated units must actually move");
        assert!(
            sample.fired > 0 && sample.permitted_shots.iter().all(|s| *s > 0),
            "permitted combat must fire for each viewer each interval"
        );
        assert!(sample.extracted > 0, "gatherers must harvest each interval");
        if self.fog {
            assert!(
                sample.hides.iter().all(|x| *x >= 20) && sample.reveals.iter().all(|x| *x >= 20),
                "required hide/reveal churn: {:?}/{:?}",
                sample.hides,
                sample.reveals
            );
        }
        crate::bot_profile::activate(false);
        sample
    }
}
pub(super) fn quantiles(mut values: Vec<u64>) -> serde_json::Value {
    values.sort_unstable();
    let at = |percent: usize| values[(values.len() * percent).div_ceil(100) - 1];
    serde_json::json!({"p50_ns":at(50),"p95_ns":at(95),"p99_ns":at(99),"max_ns":values.last().unwrap()})
}
fn rank(values: &[u64], permille: usize) -> u64 {
    assert!(!values.is_empty() && (1..=1000).contains(&permille));
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    ordered[(ordered.len() * permille).div_ceil(1000) - 1]
}
pub(super) fn deadline_summary(
    server: &[u64],
    clients: [&[u64]; 2],
    batches: [&[u64]; 2],
) -> serde_json::Value {
    let mut debt = 0u64;
    let mut maximum_debt = 0;
    for service in server {
        debt = debt.saturating_add(*service).saturating_sub(50_000_000);
        maximum_debt = maximum_debt.max(debt);
    }
    let server_pass = rank(server, 990) <= 40_000_000
        && rank(server, 999) <= 50_000_000
        && rank(server, 1000) <= 100_000_000
        && maximum_debt <= 50_000_000
        && debt == 0;
    let client_rows: Vec<_> = (0..2)
        .map(|slot| {
            let pass = rank(clients[slot], 990) <= 16_667_000
                && rank(clients[slot], 1000) <= 33_333_000
                && rank(batches[slot], 990) <= 16_667_000
                && rank(batches[slot], 1000) <= 33_333_000;
            serde_json::json!({"slot":slot,"pass":pass,
                "tick":quantiles(clients[slot].to_vec()),"batch":quantiles(batches[slot].to_vec()),
                "tick_samples":clients[slot].len(),"batch_samples":batches[slot].len()})
        })
        .collect();
    serde_json::json!({"pass":server_pass && client_rows.iter().all(|r|r["pass"]==true),
        "server_pass":server_pass,"server_tick":quantiles(server.to_vec()),
        "server_p99_9_ns":rank(server,999),"server_tick_samples":server.len(),
        "maximum_debt_ns":maximum_debt,"final_debt_ns":debt,"clients":client_rows})
}
pub(super) fn peak_resident_bytes() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))?
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)
}
fn run(intervals: usize, acceptance: bool) {
    #[cfg(debug_assertions)]
    if acceptance {
        panic!("SV7 performance acceptance requires a release build");
    }
    // Record immutable authoring before any timed work. Root captures CPU affinity,
    // compiler, machine, and build flags beside this artifact.
    let mut all = vec![];
    let modes = if acceptance {
        vec![(true, false), (false, false), (false, true)]
    } else {
        vec![(true, false)]
    };
    for (fog, concentrated) in modes {
        crate::bot_profile::reset();
        let mut harness = Harness::new(fog, concentrated);
        harness.app.world_mut().resource_mut::<Cpu>().profiling = !acceptance;
        println!("SV7_AUTHORED {}", harness.authored);
        for index in 0..WARMUP {
            harness.interval(index, false);
            if index % 10 == 0 {
                println!("SV7 warmup {index}/{WARMUP} fog={fog} concentrated={concentrated}");
            }
        }
        let mut samples = Vec::with_capacity(intervals);
        for index in 0..intervals {
            samples.push(harness.interval(index, true));
            if index % 25 == 0 {
                println!("SV7 sample {index}/{intervals} fog={fog} concentrated={concentrated}");
            }
        }
        let sum = quantiles(samples.iter().map(|s| s.sum_ns).collect());
        let v1_pass = sum["p99_ns"].as_u64().unwrap() <= 5_000_000;
        let server: Vec<_> = samples
            .iter()
            .flat_map(|s| s.full_server_tick_ns.iter().copied())
            .collect();
        let client: [Vec<_>; 2] = std::array::from_fn(|slot| {
            samples
                .iter()
                .flat_map(|s| s.client_tick_ns[slot].iter().copied())
                .collect()
        });
        let batches: [Vec<_>; 2] = std::array::from_fn(|slot| {
            samples
                .iter()
                .flat_map(|s| s.client_batches_ns[slot].iter().copied())
                .collect()
        });
        let deadline = deadline_summary(
            &server,
            [&client[0], &client[1]],
            [&batches[0], &batches[1]],
        );
        let pass = deadline["pass"] == true;
        assert_eq!(server.len(), intervals * 20);
        all.push(serde_json::json!({"fog":fog,"concentrated":concentrated,"authored":harness.authored,
            "server":quantiles(samples.iter().map(|s|s.server_ns).collect()),"client":quantiles(samples.iter().map(|s|s.client_ns).collect()),
            "sum":sum,"v1_threshold_pass":v1_pass,"threshold_pass":pass,"deadline":deadline,
            "absolute_process_peak_resident_bytes":peak_resident_bytes(),
            "queue_subphases":crate::bot_profile::summary(),"samples":samples}));
    }
    let artifact = serde_json::json!({"acceptance":acceptance,"diagnostic_only":!acceptance,
        "warmup_intervals":WARMUP,"measured_intervals":intervals,"ticks_per_interval":20,
        "performance_contract_version":2,"profile":if cfg!(debug_assertions){"debug"}else{"release"},
        "full_server_includes_completed_world_collection":true,"subsystem_server_includes_completed_world_collection":false,
        "timer_boundaries":{"simulation":"FixedPreUpdate before SimSet::Advance through FixedPostUpdate after SimSet::Collect",
            "server_service":"direct simulation plus outside-fixed queue, pump/send, bot queries, local link and shared receive; each once",
            "client_batch":"whole per-peer consume plus conservatively shared transport receive and send",
            "deferred_work":"actual chronological step, drained before next simulation step, no extra deadline credits"},
        "nested_profile_clocks":!acceptance,
        "stage_names":["start_refresh","tactical_refresh","completed_refresh","queue_encode","pump_and_transport_send","bot_projection_queries_accounts","local_link_transfer","transport_receive","assembly_decode_nested","replica_activation_nested"],"network_wait_excluded":true,"rows":all});
    if let Some(path) = std::env::var_os("TURTS_SV7_OUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    }
    println!(
        "SV7_RESULT {}",
        serde_json::json!({"acceptance":acceptance,"performance_contract_version":2,
        "rows":artifact["rows"].as_array().unwrap().iter().map(|r|serde_json::json!({"fog":r["fog"],"concentrated":r["concentrated"],"deadline":r["deadline"],"v1_threshold_pass":r["v1_threshold_pass"]})).collect::<Vec<_>>()})
    );
    if acceptance {
        assert_eq!(intervals, REQUIRED);
        assert!(
            artifact["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["threshold_pass"] == true),
            "SV7 failed frozen deadline-based v2 budget; original v1 result remains separate"
        );
    }
}
#[test]
fn service_deadlines_keep_debt_across_intervals_and_bound_whole_batches() {
    let clients = vec![1_000_000; 40];
    let batches = vec![1_000_000; 80];
    let mut server = vec![10_000_000; 40];
    assert_eq!(rank(&[1, 2, 3, 4], 999), 4);
    assert_eq!(rank(&[1, 2, 3, 4], 500), 2);
    assert_eq!(
        deadline_summary(&server, [&clients, &clients], [&batches, &batches])["pass"],
        true
    );
    // Two exceptional services straddle the20-tick boundary. No reset or
    // artificial drain tick can erase their accumulated100ms debt.
    server[19] = 100_000_000;
    server[20] = 100_000_000;
    let failed = deadline_summary(&server, [&clients, &clients], [&batches, &batches]);
    assert_eq!(failed["maximum_debt_ns"], 100_000_000);
    assert_eq!(failed["final_debt_ns"], 0);
    assert_eq!(failed["pass"], false);
    server.fill(10_000_000);
    let mut oversized_batch = batches.clone();
    oversized_batch[0] = 40_000_000;
    assert_eq!(
        deadline_summary(&server, [&clients, &clients], [&oversized_batch, &batches])["pass"],
        false
    );
    let pending = deadline_summary(&[100_000_000], [&[1], &[1]], [&[1], &[1]]);
    assert_eq!(pending["final_debt_ns"], 50_000_000);
    assert_eq!(pending["pass"], false);
}
#[test]
#[ignore = "release, four selected CPUs, full frozen SV7 acceptance"]
fn sv7_frozen_1000_warmed_intervals() {
    run(REQUIRED, true);
}
#[test]
#[ignore = "diagnostic only; same workload and 100-interval warmup; not acceptance"]
fn diagnostic_sv7_10_intervals() {
    run(10, false);
}
#[test]
#[ignore = "bounded workload preflight before timing"]
fn sv7_workload_preflight() {
    let mut harness = Harness::new(true, false);
    for index in 0..3 {
        let s = harness.interval(index, false);
        println!("SV7_PREFLIGHT {}", serde_json::to_string(&s).unwrap());
    }
}
