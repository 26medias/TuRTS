//! SV7: frozen real simulation workload, with atomic human and public bot consumers.
//! The short driver is diagnostic. Only the fixed 1,000-interval driver is acceptance.
use super::*;
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
struct Fixture {
    catalog: ContentCatalog,
    map: MapDef,
    authored: serde_json::Value,
}
fn authored_fixture(fog: bool, concentrated: bool) -> Fixture {
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
        "cot_balance_cost":50_000,"route_x":[32,80],"route_switch_ticks":10,
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
    server_ns: u64,
    client_ns: u64,
}
fn begin_cpu(mut cpu: ResMut<Cpu>) {
    cpu.start = Some(Instant::now());
}
fn end_server(mut cpu: ResMut<Cpu>) {
    if let Some(t) = cpu.start.take() {
        cpu.server_ns += t.elapsed().as_nanos() as u64;
    }
}
fn end_client(mut cpu: ResMut<Cpu>) {
    if let Some(t) = cpu.start.take() {
        cpu.client_ns += t.elapsed().as_nanos() as u64;
    }
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
struct Harness {
    app: App,
    peers: Vec<Peer>,
    catalog: Arc<ContentCatalog>,
    authored: serde_json::Value,
    seq: [u32; 2],
    fog: bool,
    concentrated: bool,
    frame_ms: u64,
}
impl Harness {
    fn new(fog: bool, concentrated: bool) -> Self {
        let fixture = authored_fixture(fog, concentrated);
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
        let staged = turts_sim::stage_package(
            app.world(),
            fixture.catalog.clone(),
            SourceDef {
                file: "stockyard/maps/sv7.ron".into(),
                value: fixture.map,
            },
            20,
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        let map = id("sv7");
        app.insert_resource(turts_sim::InstalledMapIdentity(map.clone()));
        turts_sim::visibility::initialize_context(app.world_mut(), 91, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        app.insert_resource(ServerCatalog(fixture.catalog.clone()));
        crate::outbound::knowledge::install(&mut app);
        app.init_resource::<Cpu>();
        app.init_resource::<Received>();
        app.add_systems(
            PreUpdate,
            capture_frames
                .after(MessageSystems::Receive)
                .before(end_client),
        );
        // Conservative: the completed visibility refresh runs inside Collect. Include
        // collection and immutable completed-world construction rather than subtract it.
        app.add_systems(
            FixedPreUpdate,
            begin_cpu
                .in_set(SimSet::Advance)
                .before(turts_sim::visibility::refresh_start),
        );
        app.add_systems(
            FixedPreUpdate,
            end_server
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
            end_server
                .in_set(SimSet::Acquire)
                .after(turts_sim::visibility::refresh_tactical)
                .after(begin_cpu),
        );
        app.add_systems(FixedPostUpdate, begin_cpu.before(SimSet::Collect));
        app.add_systems(
            FixedPostUpdate,
            end_server.after(SimSet::Collect).after(begin_cpu),
        );
        app.add_systems(PreUpdate, begin_cpu.before(MessageSystems::Receive));
        app.add_systems(
            PreUpdate,
            end_client.after(MessageSystems::Receive).after(begin_cpu),
        );
        app.add_systems(
            PostUpdate,
            begin_cpu.before(crate::outbound::knowledge::pump),
        );
        app.add_systems(
            PostUpdate,
            end_server.after(MessageSystems::Send).after(begin_cpu),
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
        };
        harness.flush_to(0);
        harness
    }
    fn command(
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
    fn consume(&mut self) {
        let started = Instant::now();
        for peer in &mut self.peers {
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
                match frame {
                    StateTransactionFrame::Begin(begin) => {
                        peer.assembler.begin(begin, self.frame_ms).unwrap();
                    }
                    StateTransactionFrame::Chunk(chunk) => peer.assembler.chunk(&chunk).unwrap(),
                    StateTransactionFrame::End(end) => {
                        let staged = peer.assembler.end(&end).unwrap().unwrap();
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
                        assert!(
                            peer.replica.activate_transaction(
                                &staged.stamp,
                                &staged.payload,
                                bevy::platform::time::Instant::now()
                            ),
                            "native replica refused tick{}",
                            staged.stamp.basis.tick
                        );
                        peer.latest = staged.stamp.basis.tick;
                        peer.replica.visual_events.clear();
                        let ack = peer.assembler.activate(&staged.stamp).unwrap();
                        self.app
                            .world_mut()
                            .get_mut::<MessageSender<StateTransactionAck>>(peer.client)
                            .unwrap()
                            .send::<ReliableChannel>(ack);
                    }
                }
                peer.max_retained = peer.max_retained.max(peer.assembler.retained_bytes());
            }
            peer.max_retained = peer.max_retained.max(peer.assembler.retained_bytes());
            peer.max_server_retained = peer.max_server_retained.max(
                self.app
                    .world()
                    .get::<crate::outbound::knowledge::KnowledgeSender>(peer.server)
                    .unwrap()
                    .benchmark_retained(),
            );
        }
        self.app.world_mut().resource_mut::<Cpu>().client_ns += started.elapsed().as_nanos() as u64;
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
        self.app.world_mut().resource_mut::<Cpu>().server_ns += started.elapsed().as_nanos() as u64;
    }
    fn update(&mut self, ms: u64) {
        self.frame_ms += ms;
        self.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(ms),
            ));
        self.app.update();
        self.consume();
        self.transfer();
    }
    fn flush_to(&mut self, tick: u32) {
        for _ in 0..2048 {
            self.update(0);
            if self.peers.iter().all(|p| {
                p.latest == tick
                    && p.assembler.active().is_some()
                    && self
                        .app
                        .world()
                        .get::<crate::outbound::knowledge::KnowledgeSender>(p.server)
                        .unwrap()
                        .active_stamp()
                        == p.assembler.active()
            }) {
                return;
            }
        }
        panic!("atomic publication did not drain to tick{tick}");
    }
}

#[derive(serde::Serialize)]
struct Sample {
    server_ns: u64,
    client_ns: u64,
    sum_ns: u64,
    fired: u64,
    permitted_shots: [u64; 2],
    extracted: u64,
    moved: usize,
    hides: [usize; 2],
    reveals: [usize; 2],
    retained_bytes: [usize; 2],
    wire_bytes: [u64; 2],
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
            let mut queries = lease.queries("sv7-public", operator_quota());
            queries.pin_basis(u64::from(view.tick)).unwrap();
            let mut requested = vec![
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
                Query::Works,
            ];
            // Public account queries accept at most64 keys. The complete private
            // reconciliation above remains one atomic batch, never key-chunked.
            requested.extend(keys.chunks(64).map(|keys| Query::Accounts(keys.to_vec())));
            for query in requested {
                let mut cursor = None;
                loop {
                    let page = queries
                        .query(query.clone(), cursor, 64)
                        .unwrap_or_else(|error| panic!("public SV7 query {query:?}: {error:?}"));
                    match page.state {
                        PageState::More(next) => cursor = Some(next),
                        PageState::Complete => break,
                        PageState::UnresolvedTie(_) => panic!("frozen query physical tie"),
                    }
                }
            }
            queries.release_basis(u64::from(view.tick)).unwrap();
            std::hint::black_box(coordinator);
        }
        self.app.world_mut().resource_mut::<Cpu>().server_ns += started.elapsed().as_nanos() as u64;
    }
    fn interval(&mut self, index: usize, resync: bool) -> Sample {
        let _ = std::mem::take(&mut *self.app.world_mut().resource_mut::<Cpu>());
        for peer in &mut self.peers {
            peer.hides = 0;
            peer.permitted_shots = 0;
            peer.reveals = 0;
            peer.wire_bytes = 0;
        }
        let mut fired = 0;
        let mut extracted = 0;
        let mut moved = BTreeSet::new();
        for _ in 0..20 {
            let next = completed_world(self.app.world()).tick + 1;
            self.orders(next);
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
            self.app.world_mut().resource_mut::<Cpu>().server_ns +=
                start.elapsed().as_nanos() as u64;
            // Drain actual registered transport and native activation. No work is
            // deferred outside the sample or silently coalesced into a later tick.
            self.flush_to(tick);
            if tick.is_multiple_of(20) {
                self.bots();
            }
        }
        if resync && (index + 1).is_multiple_of(100) {
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
            self.app.world_mut().resource_mut::<Cpu>().server_ns +=
                start.elapsed().as_nanos() as u64;
            let tick = completed_world(self.app.world()).tick;
            // Same-tick resync must actually activate the new transaction as well.
            let old = self
                .peers
                .iter()
                .map(|p| p.assembler.active().unwrap().transaction_id)
                .collect::<Vec<_>>();
            for _ in 0..2048 {
                self.update(0);
                if self.peers.iter().zip(&old).all(|(p, id)| {
                    p.latest == tick
                        && p.assembler.active().unwrap().transaction_id > *id
                        && self
                            .app
                            .world()
                            .get::<crate::outbound::knowledge::KnowledgeSender>(p.server)
                            .unwrap()
                            .active_stamp()
                            == p.assembler.active()
                }) {
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
                self.peers.iter().all(|p| self
                    .app
                    .world()
                    .get::<crate::outbound::knowledge::KnowledgeSender>(p.server)
                    .unwrap()
                    .active_stamp()
                    == p.assembler.active()),
                "resync activation ACK tail must be charged in this interval"
            );
        }
        let cpu = self.app.world().resource::<Cpu>();
        let sample = Sample {
            server_ns: cpu.server_ns,
            client_ns: cpu.client_ns,
            sum_ns: cpu.server_ns + cpu.client_ns,
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
        sample
    }
}
fn quantiles(mut values: Vec<u64>) -> serde_json::Value {
    values.sort_unstable();
    let at = |percent: usize| values[(values.len() * percent).div_ceil(100) - 1];
    serde_json::json!({"p50_ns":at(50),"p95_ns":at(95),"p99_ns":at(99),"max_ns":values.last().unwrap()})
}
fn run(intervals: usize, acceptance: bool) {
    // Record immutable authoring before any timed work. Root captures CPU affinity,
    // compiler, machine, and build flags beside this artifact.
    let mut all = vec![];
    let modes = if acceptance {
        vec![(true, false), (false, false), (false, true)]
    } else {
        vec![(true, false)]
    };
    for (fog, concentrated) in modes {
        let mut harness = Harness::new(fog, concentrated);
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
        let pass = sum["p99_ns"].as_u64().unwrap() <= 5_000_000;
        all.push(serde_json::json!({"fog":fog,"concentrated":concentrated,"authored":harness.authored,
            "server":quantiles(samples.iter().map(|s|s.server_ns).collect()),"client":quantiles(samples.iter().map(|s|s.client_ns).collect()),
            "sum":sum,"threshold_pass":pass,"samples":samples}));
    }
    let artifact = serde_json::json!({"acceptance":acceptance,"diagnostic_only":!acceptance,
        "warmup_intervals":WARMUP,"measured_intervals":intervals,"ticks_per_interval":20,
        "server_includes_completed_world_collection":true,"network_wait_excluded":true,"rows":all});
    if let Some(path) = std::env::var_os("TURTS_SV7_OUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    }
    println!("SV7_RESULT {artifact}");
    if acceptance {
        assert_eq!(intervals, REQUIRED);
        assert!(
            artifact["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["threshold_pass"] == true),
            "SV7 p99 exceeded frozen 5ms budget"
        );
    }
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
