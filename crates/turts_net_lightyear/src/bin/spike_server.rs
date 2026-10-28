//! Network spike server (feature spike). Task 6a, items P3 to P7.
//!
//! `spike_server --ws 127.0.0.1:<port> [--units N] [--stats <path>]`
//!
//! - `Hello` with `lockfile_hash == "sha256:spike"` gets `Welcome`. Any other hash gets
//!   `JoinRejected { reason: "lockfile hash mismatch" }` and an `Unlink` 300 ms later.
//! - Each `PlayerCommand` gets one `CommandAck` at once.
//! - Each fixed tick sends `WorldDelta` with `--units` synthetic units to every welcomed client.
//! - Each second, `--stats` gets one JSON line (see `write_stats`).
//!
//! Prints `spike_server listening <addr>` when the WebSocket port is bound.

use std::fs::OpenOptions;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bevy::app::{App, ScheduleRunnerPlugin};
use bevy::prelude::*;
use lightyear::netcode::NetcodeServer;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use turts_net_lightyear::{ProtocolPlugin, ReliableChannel, StateChannel, tick_duration};
use turts_net_protocol::EconomyChanges;
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::messages::*;

const ACCEPTED_HASH: &str = "sha256:spike";
const KICK_DELAY: Duration = Duration::from_millis(300);
const UNIT_STEP: f32 = 0.15;

#[derive(Resource)]
struct Cfg {
    ws: SocketAddr,
    units: u32,
    stats: Option<PathBuf>,
    reverse_command_results: bool,
}

/// The simulation tick. The first tick is 1; `WorldDelta.base_tick` is the tick before.
#[derive(Resource, Default)]
struct SimTick(u32);

#[derive(Resource, Default)]
struct DelayedCommandAcks(Vec<(Entity, u32)>);

#[derive(Component)]
struct Welcomed;

/// Unlink the client at `at` (after `JoinRejected` had time to leave).
#[derive(Component)]
struct PendingKick {
    at: Instant,
}

/// Tick timing, payload and memory counters for `--stats`.
#[derive(Resource)]
struct Stats {
    frame_start: Instant,
    ticked: bool,
    window_tick_us: Vec<u64>,
    window_rss_peak: u64,
    delta_payload_bytes_total: u64,
    last_write: Instant,
}

fn main() {
    let cfg = parse_args();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1))));
    app.add_plugins(ServerPlugins {
        tick_duration: tick_duration(20),
    });
    app.add_plugins(ProtocolPlugin);
    app.insert_resource(cfg);
    app.init_resource::<SimTick>();
    app.init_resource::<DelayedCommandAcks>();
    app.insert_resource(Stats {
        frame_start: Instant::now(),
        ticked: false,
        window_tick_us: Vec::new(),
        window_rss_peak: 0,
        delta_payload_bytes_total: 0,
        last_write: Instant::now(),
    });
    app.add_systems(Startup, start);
    app.add_observer(on_linked);
    app.add_systems(First, frame_start);
    app.add_systems(
        PreUpdate,
        (send_delayed_command_acks, receive_hello, receive_moves)
            .chain()
            .after(MessageSystems::Receive),
    );
    app.add_systems(Update, kick);
    app.add_systems(FixedPostUpdate, send_deltas);
    app.add_systems(Last, frame_end);
    app.run();
}

fn parse_args() -> Cfg {
    let args: Vec<String> = std::env::args().collect();
    let value = |key: &str| {
        args.iter().position(|a| a == key).map(|i| {
            args.get(i + 1)
                .unwrap_or_else(|| panic!("{key} needs a value"))
                .clone()
        })
    };
    Cfg {
        ws: value("--ws")
            .expect("--ws <addr> is required")
            .parse()
            .expect("--ws: socket address"),
        units: value("--units").map_or(1, |v| v.parse().expect("--units: integer")),
        stats: value("--stats").map(PathBuf::from),
        reverse_command_results: args.iter().any(|arg| arg == "--reverse-command-results"),
    }
}

fn start(mut commands: Commands, cfg: Res<Cfg>) {
    let ws_addr = cfg.ws;
    let entity = commands
        .spawn((
            LocalAddr(ws_addr),
            Server,
            NetcodeServer::new(NetcodeConfig {
                protocol_id: NET_PROTOCOL_VERSION as u64,
                private_key: [0u8; 32], // M1 placeholder; format open (spec 15)
                server_addr_check: false,
                // No client_timeout_secs: with Manual tokens the server uses the token's value.
                ..Default::default()
            }),
            WebSocketEndpoint {
                config: ServerConfig::builder()
                    .with_bind_address(ws_addr)
                    .with_no_encryption(),
            },
        ))
        .id();
    commands.trigger(Start { entity });
}

/// The transport inserts `Linked` on the server entity when it has bound the port.
fn on_linked(trigger: On<Add<Linked>>, servers: Query<(), With<Server>>, cfg: Res<Cfg>) {
    if servers.get(trigger.entity).is_ok() {
        println!("spike_server listening {}", cfg.ws);
    }
}

fn receive_hello(
    mut commands: Commands,
    mut hellos: Query<(Entity, &mut MessageReceiver<Hello>), With<ClientOf>>,
    mut welcome: Query<&mut MessageSender<Welcome>>,
    mut rejected: Query<&mut MessageSender<JoinRejected>>,
    tick: Res<SimTick>,
) {
    for (entity, mut receiver) in hellos.iter_mut() {
        for hello in receiver.receive() {
            if hello.lockfile_hash == ACCEPTED_HASH {
                if let Ok(mut sender) = welcome.get_mut(entity) {
                    sender.send::<ReliableChannel>(Welcome {
                        slot_id: SlotId(0),
                        tick: tick.0,
                        tick_hz: 20,
                    });
                }
                commands.entity(entity).insert(Welcomed);
                println!("spike_server welcomed player {}", hello.player_id);
            } else {
                if let Ok(mut sender) = rejected.get_mut(entity) {
                    sender.send::<ReliableChannel>(JoinRejected {
                        reason: "lockfile hash mismatch".to_string(),
                    });
                }
                commands.entity(entity).insert(PendingKick {
                    at: Instant::now() + KICK_DELAY,
                });
                println!("spike_server rejected player {}", hello.player_id);
            }
        }
    }
}

fn receive_moves(
    mut moves: Query<(Entity, &mut MessageReceiver<PlayerCommand>), With<ClientOf>>,
    mut acks: Query<&mut MessageSender<CommandAck>>,
    mut rejections: Query<&mut MessageSender<CommandRejected>>,
    mut delayed_acks: ResMut<DelayedCommandAcks>,
    cfg: Res<Cfg>,
    tick: Res<SimTick>,
) {
    for (entity, mut receiver) in moves.iter_mut() {
        for envelope in receiver.receive() {
            let command = envelope;
            if cfg.reverse_command_results {
                if let Ok(mut sender) = rejections.get_mut(entity) {
                    sender.send::<ReliableChannel>(CommandRejected {
                        seq: command.seq,
                        tick: tick.0,
                        reason: turts_net_protocol::CommandRejectReason::BadSequence,
                    });
                }
                delayed_acks.0.push((entity, command.seq));
                continue;
            }
            if let Ok(mut sender) = acks.get_mut(entity) {
                sender.send::<ReliableChannel>(CommandAck {
                    seq: command.seq,
                    tick: tick.0,
                });
            }
        }
    }
}

fn send_delayed_command_acks(
    mut delayed: ResMut<DelayedCommandAcks>,
    mut acks: Query<&mut MessageSender<CommandAck>>,
    tick: Res<SimTick>,
) {
    for (entity, seq) in std::mem::take(&mut delayed.0) {
        if let Ok(mut sender) = acks.get_mut(entity) {
            sender.send::<ReliableChannel>(CommandAck { seq, tick: tick.0 });
        }
    }
}

fn kick(mut commands: Commands, pending: Query<(Entity, &PendingKick)>) {
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

fn send_deltas(
    mut tick: ResMut<SimTick>,
    mut senders: Query<&mut MessageSender<WorldDelta>, With<Welcomed>>,
    mut stats: ResMut<Stats>,
    cfg: Res<Cfg>,
) {
    tick.0 += 1;
    stats.ticked = true;
    let changes: Vec<UnitDelta> = (0..cfg.units)
        .map(|i| UnitDelta {
            unit_id: UnitId(i),
            pos: Pos {
                x: tick.0 as f32 * UNIT_STEP,
                y: i as f32,
            },
        })
        .collect();
    let deltas = WorldDelta {
        tick: tick.0,
        base_tick: tick.0 - 1,
        spawned: vec![],
        removed: vec![],
        positions: changes,
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
    };
    let payload = postcard_len(&deltas) as u64;
    for mut sender in senders.iter_mut() {
        sender.send::<StateChannel>(deltas.clone());
        stats.delta_payload_bytes_total += payload;
    }
}

fn postcard_len(deltas: &WorldDelta) -> usize {
    postcard::experimental::serialized_size(deltas)
        .expect("a protocol 3 WorldDelta serializes to postcard")
}

#[cfg(test)]
mod payload_tests {
    use super::*;
    use turts_net_protocol::{
        BalanceState, EconomyChanges, InventoryState, JobId, NodeId, NodeState, OrderState,
        QueueState,
    };

    #[test]
    fn protocol_two_delta_size_matches_encoded_nonempty_fixture() {
        let delta = WorldDelta {
            tick: 300,
            base_tick: 299,
            spawned: vec![UnitSnapshot {
                unit_id: UnitId(9),
                content_id: "base:worker".into(),
                owner_slot: turts_net_protocol::SlotId(1),
                pos: Pos { x: 1.0, y: -2.0 },

                facing: None,

                belts: vec![],
                inserters: vec![],
                extractors: vec![],
                weapons: vec![],
                healths: vec![],
                research: vec![],
            }],
            removed: vec![UnitId(4)],
            positions: vec![UnitDelta {
                unit_id: UnitId(8),
                pos: Pos { x: 3.0, y: 4.0 },
            }],
            nodes: vec![NodeState {
                id: NodeId(2),
                content_id: "base:ore".into(),
                pos: Pos { x: 1.0, y: -2.0 },
                remaining: 900,
            }],
            economy: EconomyChanges {
                balances: vec![BalanceState {
                    resource: "base:ore".into(),
                    on_hand: 11,
                    reserved: 2,
                }],
                inventories: vec![InventoryState {
                    unit: UnitId(8),
                    items: vec![],

                    port: InventoryPort::Main,
                }],
                orders: vec![OrderState {
                    unit: UnitId(8),
                    node: Some(NodeId(2)),
                    receiver: None,
                    stage: turts_net_protocol::GatherStage::Gathering,
                }],
                queues: vec![QueueState {
                    producer: UnitId(8),
                    jobs: vec![turts_net_protocol::JobState {
                        id: JobId(7),
                        recipe: "base:hire".into(),
                        stage: turts_net_protocol::JobStage::Running,
                        elapsed_ticks: 1,
                        duration_ticks: 8,
                        charged: vec![],
                        reserved: vec![],
                    }],

                    enabled: true,
                }],
            },
        };
        let mut buffer = [0; 4096];
        let encoded = postcard::to_slice(&delta, &mut buffer).unwrap();
        assert_eq!(postcard_len(&delta), encoded.len());
    }
}

fn frame_start(mut stats: ResMut<Stats>) {
    stats.frame_start = Instant::now();
}

/// A frame that ran a fixed tick counts as one tick sample, from `First` to `Last`.
fn frame_end(mut stats: ResMut<Stats>, tick: Res<SimTick>, cfg: Res<Cfg>) {
    if stats.ticked {
        let us = stats.frame_start.elapsed().as_micros() as u64;
        stats.window_tick_us.push(us);
        stats.ticked = false;
        let rss = resident_bytes();
        stats.window_rss_peak = stats.window_rss_peak.max(rss);
    }
    if stats.last_write.elapsed() >= Duration::from_secs(1) {
        stats.last_write = Instant::now();
        if let Some(path) = &cfg.stats {
            write_stats(path, &mut stats, tick.0);
        }
        stats.window_tick_us.clear();
        stats.window_rss_peak = 0;
    }
}

/// One JSON line per second. `tick_us` holds the tick samples of the window; `tick_p50_us`,
/// `tick_p99_us` and `tick_max_us` summarize them. `resident_peak_bytes` is the largest RSS
/// sampled in the window. `ticks` and `delta_payload_bytes_total` count from the start.
fn write_stats(path: &PathBuf, stats: &mut Stats, ticks: u32) {
    let mut sorted = stats.window_tick_us.clone();
    sorted.sort_unstable();
    let pick = |q: f64| -> u64 {
        if sorted.is_empty() {
            return 0;
        }
        let rank = ((sorted.len() as f64) * q).ceil() as usize;
        sorted[rank.saturating_sub(1)]
    };
    let rss = resident_bytes();
    let line = serde_json::json!({
        "unix_ms": SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64),
        "ticks": ticks,
        "window_ticks": sorted.len(),
        "tick_p50_us": pick(0.50),
        "tick_p99_us": pick(0.99),
        "tick_max_us": sorted.last().copied().unwrap_or(0),
        "tick_us": stats.window_tick_us,
        "delta_payload_bytes_total": stats.delta_payload_bytes_total,
        "resident_bytes": rss,
        "resident_peak_bytes": stats.window_rss_peak.max(rss),
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open --stats file");
    file.write_all(format!("{line}\n").as_bytes())
        .expect("write --stats line");
}

/// Resident set size: `/proc/self/statm` resident pages times 4096.
fn resident_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
        .unwrap_or(0)
        * 4096
}
