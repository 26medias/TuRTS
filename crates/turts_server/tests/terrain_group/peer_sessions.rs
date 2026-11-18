//! Complete GM6's transport boundary with real Lightyear Links, accepted Hello
//! sessions, command admission, owner results and atomic client replicas.
use crate::{boot, config, export, orders, permissions};
#[allow(dead_code)]
#[path = "diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "physical.rs"]
mod physical;
#[path = "privacy.rs"]
mod privacy;

use bevy::prelude::*;
use lightyear::{link::recv_payload_from_bytes, prelude::*};
use std::{collections::BTreeMap, time::Duration};
use turts_client::replica::{Replica, ReplicaContext};
use turts_mod_api::ContentId;
use turts_net_lightyear::{KnowledgeChannel, ReliableChannel};
use turts_net_protocol::{
    Hello, OrderResult, PlayerCommand, StateTransactionAck, StateTransactionBegin,
    StateTransactionCancel, StateTransactionChunk, StateTransactionEnd, Welcome,
    visibility_transaction::{StateTransactionAssembler, StateTransactionFrame as Frame},
};

#[derive(Resource, Default)]
struct Received {
    frames: BTreeMap<Entity, Vec<Frame>>,
    results: BTreeMap<Entity, Vec<OrderResult>>,
}
fn receive(
    mut frames: Query<(Entity, &mut MessageReceiver<Frame>)>,
    mut results: Query<(Entity, &mut MessageReceiver<OrderResult>)>,
    mut out: ResMut<Received>,
) {
    for (id, mut receiver) in &mut frames {
        out.frames.entry(id).or_default().extend(receiver.receive());
    }
    for (id, mut receiver) in &mut results {
        out.results
            .entry(id)
            .or_default()
            .extend(receiver.receive());
    }
}
struct Peer {
    client: Entity,
    server: Entity,
    assembler: StateTransactionAssembler,
    replica: Replica,
    results: Vec<OrderResult>,
}
struct ConnectedWorld {
    fixture: privacy::PrivacyWorld,
    peers: BTreeMap<u8, Peer>,
    packets: usize,
}
impl ConnectedWorld {
    fn new(
        package: &str,
        protected: u8,
        remote_x: u32,
        reverse_creation: bool,
        reverse_hello: bool,
    ) -> Self {
        let mut app = crate::session::tests::hello_test_app();
        crate::outbound::install(&mut app);
        app.insert_resource(crate::stats::MatchId(0x474d0006));
        app.init_resource::<Received>()
            .add_systems(PreUpdate, receive.after(MessageSystems::Receive));
        let mut f = privacy::PrivacyWorld::with_app(
            package,
            protected,
            fixture::Variant::default(),
            remote_x,
            app,
        );
        f.wire_outbox = Some(Vec::new());
        f.app
            .insert_resource(crate::orders::ServerCatalog(f.catalog.clone()));
        f.app
            .world_mut()
            .resource_mut::<crate::session::Advertised>()
            .lockfile_hash = f.lockfile_hash.clone();
        let public = f.dir.join(package);
        let archive = turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&public).unwrap(),
        )
        .unwrap();
        let content = turts_client::content::ClientContent::from_archives(&[archive]).unwrap();
        let peer_ids = if reverse_creation {
            [202, 101]
        } else {
            [101, 202]
        };
        let pairs = peer_ids.map(|id| crate::session::tests::spawn_hello_peers(&mut f.app, id));
        for (client, server) in pairs {
            f.app.world_mut().entity_mut(client).insert((
                MessageReceiver::<Welcome>::default(),
                MessageReceiver::<Frame>::default(),
                MessageSender::<StateTransactionAck>::default(),
                MessageSender::<StateTransactionCancel>::default(),
            ));
            f.app.world_mut().entity_mut(server).insert((
                MessageSender::<StateTransactionBegin>::default(),
                MessageSender::<StateTransactionChunk>::default(),
                MessageSender::<StateTransactionEnd>::default(),
                MessageSender::<StateTransactionCancel>::default(),
                MessageReceiver::<StateTransactionAck>::default(),
                MessageReceiver::<StateTransactionCancel>::default(),
            ));
            let registry = f.app.world().resource::<ChannelRegistry>();
            let mut server_transport = Transport::default();
            server_transport.add_channel_send_from_registry::<ReliableChannel>(registry);
            server_transport.add_channel_receive_from_registry::<ReliableChannel>(registry);
            server_transport.add_channel_send_from_registry::<KnowledgeChannel>(registry);
            let mut client_transport = Transport::default();
            client_transport.add_channel_send_from_registry::<ReliableChannel>(registry);
            client_transport.add_channel_receive_from_registry::<ReliableChannel>(registry);
            client_transport.add_channel_receive_from_registry::<KnowledgeChannel>(registry);
            f.app
                .world_mut()
                .entity_mut(server)
                .insert(server_transport);
            f.app
                .world_mut()
                .entity_mut(client)
                .insert(client_transport);
        }
        for i in if reverse_hello { [1, 0] } else { [0, 1] } {
            crate::session::tests::deliver_hello(
                &mut f.app,
                pairs[i].0,
                pairs[i].1,
                Hello {
                    player_id: peer_ids[i],
                    protocol_version: turts_net_protocol::NET_PROTOCOL_VERSION,
                    lockfile_hash: f.lockfile_hash.clone(),
                },
            );
        }
        let mut peers = BTreeMap::new();
        for (client, server) in pairs {
            // Derive the logical viewer from the accepted session, never peer order.
            let slot = f
                .app
                .world()
                .get::<crate::session::Session>(server)
                .unwrap()
                .slot
                .0;
            let mut replica = Replica::new(20);
            replica.set_context(ReplicaContext::new(content.clone(), slot));
            assert!(
                peers
                    .insert(
                        slot,
                        Peer {
                            client,
                            server,
                            replica,
                            results: Vec::new(),
                            assembler: StateTransactionAssembler::new(
                                0x474d0006,
                                ContentId::parse(&format!("{package}:field")).unwrap(),
                                turts_net_lightyear::KNOWLEDGE_FRAME_LIMIT_BYTES
                            ),
                        }
                    )
                    .is_none()
            );
        }
        assert_eq!(peers.keys().copied().collect::<Vec<_>>(), [0, 1]);
        let mut result = Self {
            fixture: f,
            peers,
            packets: 0,
        };
        result.flush();
        result
    }
    fn pump(&mut self) {
        for peer in self.peers.values() {
            for (from, to) in [(peer.client, peer.server), (peer.server, peer.client)] {
                let packets = self
                    .fixture
                    .app
                    .world_mut()
                    .get_mut::<Link>(from)
                    .unwrap()
                    .send
                    .drain()
                    .collect::<Vec<_>>();
                self.packets += packets.len();
                for packet in packets {
                    self.fixture
                        .app
                        .world_mut()
                        .get_mut::<Link>(to)
                        .unwrap()
                        .recv
                        .push_raw(recv_payload_from_bytes(packet));
                }
            }
        }
        self.fixture
            .app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::ZERO,
            ));
        self.fixture.app.update();
        assert_eq!(
            self.fixture.app.world().resource::<turts_sim::Tick>().0,
            self.fixture.tick
        );
        let mut received =
            std::mem::take(&mut *self.fixture.app.world_mut().resource_mut::<Received>());
        for (&slot, peer) in &mut self.peers {
            for frame in received.frames.remove(&peer.client).unwrap_or_default() {
                match frame {
                    Frame::Begin(begin) => peer
                        .assembler
                        .begin(begin, u64::from(self.fixture.tick) * 50)
                        .unwrap(),
                    Frame::Chunk(chunk) => peer.assembler.chunk(&chunk).unwrap(),
                    Frame::End(end) => {
                        if let Some(staged) = peer.assembler.end(&end).unwrap() {
                            assert!(peer.replica.activate_transaction(
                                &staged.stamp,
                                &staged.payload,
                                bevy::platform::time::Instant::now()
                            ));
                            assert_eq!(peer.replica.knowledge.as_ref().unwrap().viewer, slot);
                            let ack = peer.assembler.activate(&staged.stamp).unwrap();
                            self.fixture
                                .app
                                .world_mut()
                                .get_mut::<MessageSender<StateTransactionAck>>(peer.client)
                                .unwrap()
                                .send::<ReliableChannel>(ack);
                        }
                    }
                }
            }
            peer.results
                .extend(received.results.remove(&peer.client).unwrap_or_default());
        }
        assert!(received.frames.is_empty() && received.results.is_empty());
    }
    fn flush(&mut self) {
        for _ in 0..120 {
            self.pump();
            if self
                .peers
                .values()
                .all(|p| p.replica.last_applied_tick == Some(self.fixture.tick))
            {
                // Deliver acknowledgements through the same real transport before continuing.
                for _ in 0..4 {
                    self.pump();
                }
                return;
            }
        }
        panic!(
            "wire replicas did not reach completed tick{}",
            self.fixture.tick
        );
    }
    fn step(&mut self) {
        self.fixture.submit_scheduled(false);
        for (slot, command) in std::mem::take(self.fixture.wire_outbox.as_mut().unwrap()) {
            let client = self.peers[&slot].client;
            self.fixture
                .app
                .world_mut()
                .get_mut::<MessageSender<PlayerCommand>>(client)
                .unwrap()
                .send::<ReliableChannel>(command);
        }
        // Serialize and receive commands with no simulation time elapsed.
        for _ in 0..4 {
            self.pump();
        }
        let before = self.fixture.bodies();
        self.fixture
            .app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(50),
            ));
        self.fixture.app.update();
        self.fixture.check_completed_step(&before);
        self.flush();
    }
    fn owner_wire(&self, slot: u8) -> serde_json::Value {
        let p = &self.peers[&slot];
        let policy = permissions::completed(self.fixture.app.world());
        let (expected, _) = export::snapshot_for_with_permissions(
            turts_sim::completed_world(self.fixture.app.world()),
            turts_sim::SlotId(slot),
            policy.0.as_ref(),
        );
        assert_eq!(
            p.replica.units.len(),
            expected.units.len(),
            "wire membership must be the complete permitted projection"
        );
        for unit in &expected.units {
            let actual = &p.replica.units[&unit.unit_id.0];
            assert_eq!(actual.owner_slot, unit.owner_slot.0);
            assert_eq!(actual.content_id, unit.content_id);
            assert_eq!(
                actual.samples.back().unwrap().1,
                unit.pos,
                "wire positions must match the completed permitted state"
            );
        }
        assert_eq!(p.replica.economy, expected.economy);
        let expected_orders = expected
            .unit_orders
            .into_iter()
            .map(|row| (row.unit.0, row))
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(
            p.replica.unit_orders, expected_orders,
            "complete owner order wire"
        );
        for result in &p.results {
            assert!(result.envelope.is_none() && result.refused.is_empty());
            assert!(!result.accepted.is_empty());
            for id in &result.accepted {
                let entity = turts_sim::unit_entity(self.fixture.app.world(), id.0).unwrap();
                assert_eq!(
                    self.fixture
                        .app
                        .world()
                        .get::<turts_sim::Owner>(entity)
                        .unwrap()
                        .0
                        .0,
                    slot,
                    "foreign order result leaked over owner wire"
                );
            }
        }
        let units = p.replica.units.iter().map(|(&id, u)| {
            let (tick, pos) = u.samples.back().unwrap();
            (id, serde_json::json!({"content":u.content_id,"owner":u.owner_slot,"tick":tick,"position_bits":[pos.x.to_bits(),pos.y.to_bits()]}))
        }).collect::<BTreeMap<_,_>>();
        for id in p.replica.unit_orders.keys() {
            assert_eq!(p.replica.units[id].owner_slot, slot);
        }
        let terrain = p.replica.terrain.as_ref().map(|t| {
            serde_json::json!({
                "basis":t.basis,"materials":t.materials,
                "surfaces":t.surfaces.values().collect::<Vec<_>>(),
                "anchors":t.anchors.values().collect::<Vec<_>>(),
                "work":t.work.values().collect::<Vec<_>>()
            })
        });
        serde_json::json!({"units":units,"orders":p.replica.unit_orders,"economy":p.replica.economy,"knowledge":p.replica.knowledge,"terrain":terrain,"results":p.results})
    }
}

#[test]
fn real_peer_creation_and_hello_reversal_preserve_paired_hidden_group_worlds() {
    let mut compared_dirs = Vec::new();
    for package in ["quarry", "terrain_stockyard"] {
        for protected in [0, 1] {
            let mut canonical = None;
            for (creation, hello) in [(false, false), (true, false), (false, true), (true, true)] {
                let mut a = ConnectedWorld::new(package, protected, 52, creation, hello);
                let mut b = ConnectedWorld::new(package, protected, 54, creation, hello);
                assert_eq!(a.fixture.archive_hash, b.fixture.archive_hash);
                let mut rows = Vec::new();
                let mut frontier_ticks = 0;
                let mut other_leases = None;
                while a.fixture.tick < 560 {
                    a.step();
                    b.step();
                    let tick = a.fixture.tick;
                    assert_eq!(
                        a.fixture.permitted(),
                        b.fixture.permitted(),
                        "hidden full projection tick{tick}"
                    );
                    assert_eq!(
                        a.fixture.protected_navigation(),
                        b.fixture.protected_navigation(),
                        "hidden navigation tick{tick}"
                    );
                    assert_eq!(
                        a.fixture.position("protected_probe"),
                        b.fixture.position("protected_probe")
                    );
                    for slot in [0, 1] {
                        a.owner_wire(slot);
                        b.owner_wire(slot);
                    }
                    let wire = a.owner_wire(protected);
                    assert_eq!(
                        wire,
                        b.owner_wire(protected),
                        "hidden owner wire tick{tick}"
                    );
                    if tick == 200 {
                        assert!(a.fixture.surface.rectangle_is([30, 23], [31, 26], 4));
                        assert!(b.fixture.surface.rectangle_is([30, 23], [31, 26], 4));
                        assert!(a.fixture.surface.rectangle_is([52, 36], [53, 37], 4));
                        assert!(b.fixture.surface.rectangle_is([52, 36], [53, 37], 0));
                        assert!(b.fixture.surface.rectangle_is([54, 36], [55, 37], 4));
                        assert!(a.fixture.surface.rectangle_is([54, 36], [55, 37], 0));
                        assert_ne!(
                            a.owner_wire(1 - protected),
                            b.owner_wire(1 - protected),
                            "editor must observe the meaningful hidden difference"
                        );
                        other_leases = Some((
                            a.fixture.member("other_probe").unwrap(),
                            b.fixture.member("other_probe").unwrap(),
                        ));
                    }
                    if (203..=300).contains(&tick) {
                        let (left, right) = other_leases.as_ref().unwrap();
                        diagnostics::same_retained_revisions(
                            left,
                            &a.fixture.member("other_probe").unwrap(),
                        );
                        diagnostics::same_retained_revisions(
                            right,
                            &b.fixture.member("other_probe").unwrap(),
                        );
                    }
                    let pos = a.fixture.position("protected_probe");
                    if tick > 202
                        && tick < 360
                        && (28.7..30.).contains(&pos.x)
                        && (22.0..27.0).contains(&pos.y)
                    {
                        frontier_ticks += 1;
                    }
                    rows.push(serde_json::json!({"tick":tick,"wire":wire,"navigation":a.fixture.protected_navigation()}));
                }
                assert!(frontier_ticks >= 20);
                for world in [&a, &b] {
                    assert!(world.packets > 0);
                    assert!(world.fixture.surface.rectangle_is([30, 23], [31, 26], 0));
                    for peer in world.peers.values() {
                        assert!(!peer.results.is_empty());
                    }
                }
                if let Some(expected) = &canonical {
                    assert_eq!(expected, &rows, "creation{creation}/Hello{hello}");
                } else {
                    canonical = Some(rows);
                }
                for world in [&a, &b] {
                    compared_dirs.push(world.fixture.dir.clone());
                    std::fs::write(world.fixture.dir.join("real-peer-result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","protected":protected,"reverse_creation":creation,"reverse_hello":hello,"ticks":560,"frontier_ticks":frontier_ticks,"packets":world.packets,"real_sessions":2,"peer_connection_reversal":"pending complete matrix"})).unwrap()).unwrap();
                }
            }
        }
    }
    for dir in compared_dirs {
        let path = dir.join("real-peer-result.json");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        result["peer_connection_reversal"] = serde_json::json!("CHECKED");
        std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }
    println!("ALL_GM6_PEER_REVERSALS_CHECKED");
}
