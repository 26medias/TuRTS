//! Task 8a-2: ingest, resync hooks, join state, debug JSON and frame log format.
//!
//! The ingest tests run in process with a `Replica` and `IngestHooks`: no network. Each test
//! asserts on the `Vec<Outgoing>` that the ingest function returns, not on the counters.

use std::{collections::HashMap, path::PathBuf, sync::OnceLock};

use bevy::platform::time::Instant;
use bevy::prelude::*;
use turts_client::acquire::Acquired;
use turts_client::content::ClientContent;
use turts_client::debug::debug_json;
use turts_client::framelog::{frame_line, summary_line};
use turts_client::ingest::{
    Incoming, IngestCounters, IngestHooks, Outgoing, ReplicaState, ingest_batch, ingest_one,
};
use turts_client::join::{AcquiredPackages, JoinState, Phase, advance_phase, ws_socket_addr};
use turts_client::options::parse_cli;
use turts_client::present::{Presented, PresentedUnit};
use turts_mod_api::Shape;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    Pos, SlotId, Snapshot, UnitDelta, UnitId, UnitSnapshot, Welcome, WorldDelta,
};

/// The parts that the ingest system owns, with one clock.
fn base_content() -> ClientContent {
    static CONTENT: OnceLock<ClientContent> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
            let archive =
                PackageArchive::parse(&pack_dir(&base).expect("pack base")).expect("parse base");
            ClientContent::from_archives(&[archive]).expect("resolve base")
        })
        .clone()
}

struct Rig {
    hooks: IngestHooks,
    counters: IngestCounters,
    state: ReplicaState,
    now: Instant,
}

impl Rig {
    fn new() -> Self {
        let state = ReplicaState {
            content: Some(base_content()),
            ..ReplicaState::default()
        };
        Self {
            hooks: IngestHooks::default(),
            counters: IngestCounters::default(),
            state,
            now: Instant::now(),
        }
    }

    fn feed(&mut self, message: Incoming) -> Vec<Outgoing> {
        ingest_one(
            &mut self.hooks,
            &mut self.counters,
            &mut self.state,
            message,
            self.now,
        )
    }

    fn feed_batch(&mut self, messages: Vec<Incoming>) -> Vec<Outgoing> {
        ingest_batch(
            &mut self.hooks,
            &mut self.counters,
            &mut self.state,
            messages,
            self.now,
        )
    }
}

fn welcome(tick: u32) -> Incoming {
    Incoming::Welcome(Welcome {
        slot_id: SlotId(0),
        tick,
        tick_hz: 20,
    })
}

/// Unit 1 is at `(x, y)` in a snapshot.
fn snapshot_at(tick: u32, x: f32, y: f32) -> Incoming {
    Incoming::Snapshot(Snapshot {
        tick,
        nodes: vec![],
        economy: turts_net_protocol::EconomyState {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        units: vec![UnitSnapshot {
            unit_id: UnitId(1),
            content_id: "base:command_center".to_string(),
            owner_slot: SlotId(0),
            pos: Pos { x, y },

            facing: None,
        }],

        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    })
}

/// Unit 1 is at x = tick, so a position shows the tick it came from.
fn snapshot(tick: u32) -> Incoming {
    snapshot_at(tick, tick as f32, 0.0)
}

fn delta_at(tick: u32, base_tick: u32, x: f32, y: f32) -> Incoming {
    Incoming::Deltas(WorldDelta {
        tick,
        base_tick,
        spawned: vec![],
        removed: vec![],
        nodes: vec![],
        economy: turts_net_protocol::EconomyChanges {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        positions: vec![UnitDelta {
            unit_id: UnitId(1),
            pos: Pos { x, y },
        }],

        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    })
}

fn delta(tick: u32, base_tick: u32) -> Incoming {
    delta_at(tick, base_tick, tick as f32, 0.0)
}

/// A delta with no change: the unit is unchanged at `tick`.
fn empty_delta(tick: u32, base_tick: u32) -> Incoming {
    Incoming::Deltas(WorldDelta {
        tick,
        base_tick,
        spawned: vec![],
        removed: vec![],
        nodes: vec![],
        economy: turts_net_protocol::EconomyChanges {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        positions: Vec::new(),

        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    })
}

/// Feeds the chained deltas `from..=to` and checks that none of them returns an `Outgoing`.
fn feed_quiet_chain(rig: &mut Rig, from: u32, to: u32) {
    for tick in from..=to {
        let out = rig.feed(delta(tick, tick - 1));
        assert!(out.is_empty(), "delta {tick} returned {out:?}");
    }
}

#[test]
fn drop_offsets_resolve_against_welcome_tick() {
    let mut rig = Rig::new();
    rig.hooks.drop_offsets = vec![10, 90];
    assert!(rig.feed(welcome(100)).is_empty());
    assert_eq!(rig.hooks.drop_ticks, vec![110, 190]);
    assert!(rig.feed(snapshot(100)).is_empty());
    feed_quiet_chain(&mut rig, 101, 109);
    // Delta 110 is dropped: nothing is returned yet.
    assert!(rig.feed(delta(110, 109)).is_empty());
    assert_eq!(rig.counters.deltas_dropped, vec![110]);
    // Delta 111 names base 110, which the replica never applied: one request.
    assert_eq!(
        rig.feed(delta(111, 110)),
        vec![Outgoing::ResyncRequest],
        "delta 111 must start the resync"
    );
    // The request is outstanding: later deltas are buffered and send nothing.
    assert!(rig.feed(delta(112, 111)).is_empty());
    assert!(rig.feed(delta(113, 112)).is_empty());
}

#[test]
fn arrival_delta_is_dropped_once_and_recorded() {
    let mut rig = Rig::new();
    rig.hooks.drop_arrival_delta = true;
    rig.hooks.arrival_target = Some(Pos { x: 20.0, y: 8.0 });
    rig.hooks.arrival_unit = Some(1);
    assert!(rig.feed(welcome(100)).is_empty());
    assert!(rig.feed(snapshot_at(100, 18.0, 8.0)).is_empty());
    assert!(rig.feed(delta_at(101, 100, 19.0, 8.0)).is_empty());
    // The first delta that puts unit 1 at the target is the arrival delta: dropped.
    assert!(rig.feed(delta_at(102, 101, 20.0, 8.0)).is_empty());
    assert_eq!(rig.counters.deltas_dropped, vec![102]);
    assert_eq!(
        rig.state.replica.as_ref().unwrap().last_applied_tick,
        Some(101)
    );
    // The next delta has no change, because the unit stopped. Its base is the dropped tick.
    assert_eq!(
        rig.feed(empty_delta(103, 102)),
        vec![Outgoing::ResyncRequest]
    );
    // The answer applies at once (hold is 0).
    assert!(rig.feed(snapshot_at(103, 20.0, 8.0)).is_empty());
    // A later delta with the same position is applied: the hook fires once.
    assert!(rig.feed(delta_at(104, 103, 20.0, 8.0)).is_empty());
    assert_eq!(rig.counters.deltas_dropped, vec![102]);
    let replica = rig.state.replica.as_ref().unwrap();
    assert_eq!(replica.last_applied_tick, Some(104));
    assert!(!replica.in_resync());
}

#[test]
fn held_snapshot_releases_after_n_delta_ticks() {
    let mut rig = Rig::new();
    rig.hooks.hold_resync_snapshots = 3;
    let mut all = Vec::new();
    all.extend(rig.feed(welcome(100)));
    all.extend(rig.feed(snapshot(100)));
    for tick in 101..=110 {
        all.extend(rig.feed(delta(tick, tick - 1)));
    }
    assert!(all.is_empty());
    // Delta 111 is lost. Delta 112 names base 111: one request.
    let out = rig.feed(delta(112, 111));
    assert_eq!(out, vec![Outgoing::ResyncRequest]);
    all.extend(out);
    // The answer snapshot (tick 112) arrives and is held until the delta of tick 115.
    all.extend(rig.feed(snapshot(112)));
    assert_eq!(rig.hooks.held.len(), 1);
    all.extend(rig.feed(delta(113, 112)));
    all.extend(rig.feed(delta(114, 113)));
    {
        let replica = rig.state.replica.as_ref().unwrap();
        assert!(replica.in_resync(), "held: still in resync at tick 114");
        assert_eq!(replica.last_applied_tick, Some(110));
        assert_eq!(replica.snapshot_tick, Some(100));
    }
    // Delta 115 is buffered, then the held snapshot is released and the replay applies 113..=115.
    all.extend(rig.feed(delta(115, 114)));
    let replica = rig.state.replica.as_ref().unwrap();
    assert!(!replica.in_resync());
    assert_eq!(replica.snapshot_tick, Some(112));
    assert_eq!(replica.last_applied_tick, Some(115));
    assert!(rig.hooks.held.is_empty());
    assert!(!rig.hooks.outstanding_request);
    // One request in the whole run.
    assert_eq!(all, vec![Outgoing::ResyncRequest]);
}

#[test]
fn welcome_snapshot_is_never_held() {
    let mut rig = Rig::new();
    rig.hooks.hold_resync_snapshots = 3;
    assert!(rig.feed(welcome(100)).is_empty());
    assert!(rig.feed(snapshot(100)).is_empty());
    assert!(rig.hooks.held.is_empty());
    let replica = rig.state.replica.as_ref().unwrap();
    assert_eq!(replica.snapshot_tick, Some(100));
    assert!(!replica.in_resync());
}

#[test]
fn disconnect_before_connect_is_ignored() {
    // lightyear adds `Disconnected` when it spawns the client entity, before `Connected`.
    let mut join = JoinState::default();
    join.on_disconnected();
    join.on_connected();
    assert!(join.connected_seen);
    assert!(!join.disconnected_after_connect);
    // A `Disconnected` after `Connected` counts.
    join.on_disconnected();
    assert!(join.disconnected_after_connect);
}

#[test]
fn welcome_and_snapshot_in_one_frame_apply_both() {
    // The order of the batch does not matter: ingest reads Welcome first.
    let batches = [
        ("welcome first", vec![welcome(100), snapshot(101)]),
        ("snapshot first", vec![snapshot(101), welcome(100)]),
    ];
    for (name, messages) in batches {
        let mut rig = Rig::new();
        let out = rig.feed_batch(messages);
        assert!(out.is_empty(), "{name} returned {out:?}");
        let replica = rig
            .state
            .replica
            .as_ref()
            .expect("Welcome creates the replica");
        assert_eq!(replica.snapshot_tick, Some(101), "{name}");
        assert_eq!(rig.counters.snapshots_applied, 1, "{name}");
        assert_eq!(
            advance_phase(&Phase::Joining, &rig.state),
            Some(Phase::Joined { slot: 0 }),
            "{name}"
        );
    }
}

#[test]
fn valid_snapshot_covers_invalid_initial_snapshot_without_successor() {
    let mut rig = Rig::new();
    assert!(rig.feed(welcome(100)).is_empty());
    let mut invalid = match snapshot(100) {
        Incoming::Snapshot(snapshot) => snapshot,
        _ => unreachable!(),
    };
    invalid.units[0].content_id = "unknown:unit".to_string();
    assert_eq!(
        rig.feed(Incoming::Snapshot(invalid)),
        vec![Outgoing::ResyncRequest]
    );
    assert_eq!(rig.counters.resync_requests_sent, 1);
    assert!(rig.hooks.outstanding_request);

    assert!(rig.feed(snapshot(101)).is_empty());
    assert!(!rig.hooks.outstanding_request);
    assert_eq!(rig.counters.resync_requests_sent, 1);
    assert_eq!(rig.counters.successor_requests, 0);
    assert_eq!(
        rig.state.replica.as_ref().unwrap().last_applied_tick,
        Some(101)
    );
    assert!(!rig.state.replica.as_ref().unwrap().in_resync());
}

#[test]
fn idle_resync_sends_one_request_after_10_ticks() {
    let mut rig = Rig::new();
    assert!(rig.feed(welcome(100)).is_empty());
    // No Snapshot arrives. The deltas are buffered. A span of 8 ticks sends nothing.
    feed_quiet_chain(&mut rig, 101, 109);
    // Delta 111 makes the span 10 ticks (101 to 111): exactly one request.
    assert_eq!(
        rig.feed(delta(111, 110)),
        vec![Outgoing::ResyncRequest],
        "a span of 10 ticks must send the request"
    );
    for tick in 112..=130 {
        let out = rig.feed(delta(tick, tick - 1));
        assert!(out.is_empty(), "delta {tick} returned {out:?}");
    }
}

#[test]
fn overflow_discards_trigger_coalesces_request_and_converges_on_successor() {
    let mut rig = Rig::new();
    rig.state.config.max_buffered_batches = 3;
    rig.hooks.hold_resync_snapshots = 4;
    assert!(rig.feed(welcome(100)).is_empty());
    assert!(rig.feed(snapshot(100)).is_empty());

    assert_eq!(rig.feed(delta(102, 101)), vec![Outgoing::ResyncRequest]);
    assert!(rig.feed(snapshot(102)).is_empty());
    assert_eq!(rig.hooks.held.len(), 1);
    assert!(rig.feed(delta(103, 102)).is_empty());
    assert!(rig.feed(delta(104, 103)).is_empty());
    assert!(rig.feed(delta(105, 104)).is_empty());
    assert_eq!(rig.counters.buffer_overflows, 1);
    assert_eq!(rig.state.replica.as_ref().unwrap().buffered_batches(), 0);
    assert!(rig.hooks.outstanding_request);

    // A later heartbeat starts a fresh tail. Consuming the delayed, incomplete response allows
    // exactly one successor request; the request marker remains set while that response waits.
    assert_eq!(rig.feed(delta(106, 105)), vec![Outgoing::ResyncRequest]);
    assert_eq!(
        rig.state.replica.as_ref().unwrap().last_applied_tick,
        Some(100)
    );
    assert_eq!(rig.counters.snapshots_applied, 1);
    assert_eq!(rig.counters.successor_requests, 1);
    assert!(rig.hooks.outstanding_request);
    assert_eq!(rig.counters.resync_requests_sent, 2);
    rig.hooks.hold_resync_snapshots = 2;
    assert!(rig.feed(snapshot(106)).is_empty());
    assert!(rig.hooks.outstanding_request);
    assert!(rig.feed(delta(107, 106)).is_empty());
    assert!(rig.feed(delta(108, 107)).is_empty());

    let replica = rig.state.replica.as_ref().unwrap();
    assert_eq!(replica.last_applied_tick, Some(108));
    assert!(!replica.in_resync());
    assert!(rig.hooks.held.is_empty());
    assert!(!rig.hooks.outstanding_request);
    assert_eq!(rig.counters.max_buffered_batches, 3);
    assert_eq!(rig.counters.max_outstanding_requests, 1);
    assert_eq!(rig.counters.buffer_overflows, 1);
    assert_eq!(rig.counters.successor_requests, 1);
    assert_eq!(rig.counters.snapshots_applied, 2);
    assert_eq!(rig.counters.resync_requests_sent, 2);
}

#[test]
fn held_stale_response_does_not_rollback_and_requests_one_successor() {
    let mut rig = Rig::new();
    assert!(rig.feed(welcome(100)).is_empty());
    assert!(rig.feed(snapshot(100)).is_empty());
    assert!(rig.feed(delta(101, 100)).is_empty());
    assert_eq!(rig.feed(delta(103, 102)), vec![Outgoing::ResyncRequest]);

    rig.hooks.hold_resync_snapshots = 5;
    assert!(rig.feed(snapshot(100)).is_empty());
    assert_eq!(rig.hooks.held.len(), 1);
    assert!(rig.feed(delta(104, 103)).is_empty());
    assert_eq!(rig.feed(delta(105, 104)), vec![Outgoing::ResyncRequest]);

    let replica = rig.state.replica.as_ref().unwrap();
    assert_eq!(replica.last_applied_tick, Some(101));
    assert_eq!(replica.sample(1, 101.0).unwrap().pos.x, 101.0);
    assert!(rig.hooks.outstanding_request);
    assert_eq!(rig.counters.snapshots_applied, 1);
    assert_eq!(rig.counters.successor_requests, 1);
    assert_eq!(rig.counters.max_outstanding_requests, 1);
    assert_eq!(rig.counters.resync_requests_sent, 2);
}

#[test]
fn messages_before_welcome_are_ignored() {
    let mut rig = Rig::new();
    assert!(rig.feed(snapshot(50)).is_empty());
    assert!(rig.feed(delta(51, 50)).is_empty());
    assert!(rig.state.replica.is_none());
    assert_eq!(rig.counters.snapshots_applied, 0);
}

#[test]
fn phase_advances_to_joined_only_after_welcome() {
    let mut rig = Rig::new();
    assert_eq!(advance_phase(&Phase::Joining, &rig.state), None);
    rig.feed(welcome(7));
    assert_eq!(
        advance_phase(&Phase::Joining, &rig.state),
        Some(Phase::Joined { slot: 0 })
    );
    // A failed join stays failed; a joined client does not advance again.
    let failed = Phase::Failed("lockfile hash mismatch".to_string());
    assert_eq!(advance_phase(&failed, &rig.state), None);
    assert_eq!(advance_phase(&Phase::Joined { slot: 0 }, &rig.state), None);
}

#[test]
fn ws_address_accepts_a_url_or_host_and_port() {
    let expected = "127.0.0.1:7001".parse().unwrap();
    assert_eq!(ws_socket_addr("ws://127.0.0.1:7001"), Ok(expected));
    assert_eq!(ws_socket_addr("127.0.0.1:7001"), Ok(expected));
    assert_eq!(ws_socket_addr("ws://127.0.0.1:7001/path"), Ok(expected));
    let error = ws_socket_addr("ws://not an address").unwrap_err();
    assert!(error.contains("not an address"), "{error}");
}

#[test]
fn protocol_version_hook_is_read_from_the_command_line() {
    let base = ["--server", "http://h:7000", "--ws", "ws://h:7001"];
    let args = |extra: &[&str]| -> Vec<String> {
        base.iter()
            .chain(extra)
            .map(|arg| arg.to_string())
            .collect()
    };
    assert_eq!(parse_cli(&args(&[])).unwrap().protocol_version, None);
    let options = parse_cli(&args(&["--protocol-version", "7"])).unwrap();
    assert_eq!(options.protocol_version, Some(7));
    let error = parse_cli(&args(&["--protocol-version", "x"])).unwrap_err();
    assert!(error.contains("--protocol-version"), "{error}");
}

/// A world with one joined client that presents one unit.
fn world_with_one_unit() -> World {
    let mut world = World::new();
    let mut rig = Rig::new();
    rig.feed(welcome(100));
    rig.feed(Incoming::Snapshot(Snapshot {
        tick: 100,
        nodes: vec![],
        economy: turts_net_protocol::EconomyState {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
        units: vec![UnitSnapshot {
            unit_id: UnitId(1),
            content_id: "acme:scout".to_string(),
            owner_slot: SlotId(0),
            pos: Pos { x: 4.0, y: 4.0 },

            facing: None,
        }],

        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    }));
    world.insert_resource(rig.state);
    world.insert_resource(Phase::Joined { slot: 0 });
    world.insert_resource(AcquiredPackages(Acquired {
        lockfile_hash: "sha256:x".to_string(),
        visuals: HashMap::from([(
            "acme:scout".to_string(),
            Shape::Cuboid {
                x: 1.0,
                y: 0.8,
                z: 1.4,
            },
        )]),
        map_size: (32.0, 32.0),
        unit_names: HashMap::new(),
        content: ClientContent::default(),
    }));
    world.insert_resource(Presented {
        frame_ns: 5,
        t: 99.5,
        latest_tick: 100,
        units: vec![PresentedUnit {
            unit_id: 1,
            content_id: "acme:scout".to_string(),
            pos: Pos { x: 4.0, y: 4.0 },
            lo_tick: 99,
            hi_tick: 100,
            fraction: 0.5,
        }],
    });
    world
}

#[test]
fn debug_json_reports_phase_units_shape_and_last_frame() {
    let world = world_with_one_unit();
    let json: serde_json::Value = serde_json::from_str(&debug_json(&world)).expect("valid JSON");
    assert_eq!(json["phase"], "Joined { slot: 0 }");
    let units = json["units"].as_array().expect("units array");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["unit_id"], 1);
    assert_eq!(units[0]["content_id"], "acme:scout");
    assert_eq!(units[0]["pos"], serde_json::json!([4.0, 4.0]));
    // The shape is the downloaded visual: the variant name is the key.
    assert!(
        units[0]["shape"]["Cuboid"].is_object(),
        "{}",
        units[0]["shape"]
    );
    assert_eq!(json["last_frame"]["lo_tick"], 99);
    assert_eq!(json["last_frame"]["hi_tick"], 100);
    assert_eq!(json["last_frame"]["latest_tick"], 100);
    assert_eq!(json["last_frame"]["fraction"], 0.5);
    assert_eq!(json["last_frame"]["t"], 99.5);
}

#[test]
fn debug_json_without_units_has_null_last_frame() {
    let mut world = World::new();
    world.insert_resource(Phase::FetchAdvertisement);
    world.insert_resource(ReplicaState::default());
    world.insert_resource(Presented::default());
    let json: serde_json::Value = serde_json::from_str(&debug_json(&world)).expect("valid JSON");
    assert_eq!(json["phase"], "FetchAdvertisement");
    assert_eq!(json["units"], serde_json::json!([]));
    assert!(json["last_frame"].is_null());
}

#[test]
fn frame_log_lines_match_the_format() {
    let unit = PresentedUnit {
        unit_id: 1,
        content_id: "base:command_center".to_string(),
        pos: Pos { x: 12.5, y: 8.0 },
        lo_tick: 140,
        hi_tick: 141,
        fraction: 0.25,
    };
    let presented = Presented {
        frame_ns: 1_700_000_000_123_456_789,
        t: 140.25,
        latest_tick: 141,
        units: vec![unit.clone()],
    };
    assert_eq!(
        frame_line(&presented, &unit),
        "1700000000123456789,1,140.25,140,141,0.25,12.5,8,141"
    );
    assert_eq!(
        summary_line(Some(100), Some(101), 2, &[110, 150]),
        "# welcome_tick=100 first_present_tick=101 resync_requests_sent=2 deltas_dropped=110;150"
    );
    // Nothing presented, nothing dropped: the fields say so, and a parser fails on `none`.
    assert_eq!(
        summary_line(None, None, 0, &[]),
        "# welcome_tick=none first_present_tick=none resync_requests_sent=0 deltas_dropped="
    );
}
