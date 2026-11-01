//! Network spike client (feature spike). Task 6a, items P3 to P7.
//!
//! `spike_client --ws <addr> [--hash <h>] [--moves N] [--run-secs S] [--out <path>]`
//!
//! Connects, sends `Hello`, counts what the server sends, and writes `--out` JSON at exit:
//! `{ welcomed, rejected_reason, disconnected_after_connect, acks, ack_seqs_unique, deltas,
//! deltas_in_order, first_tick, last_tick }`. With `--moves N` it sends N `PlayerCommand` after
//! `Welcome`, one per frame, and exits 1 s after the N-th ack (or at `--run-secs`).

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bevy::app::{App, ScheduleRunnerPlugin};
use bevy::prelude::*;
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use lightyear::transport::channel::ChannelKind;
use lightyear::transport::packet::message::MessageId;
use turts_net_lightyear::{ProtocolPlugin, ReliableChannel, tick_duration};
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::messages::*;

#[derive(Resource)]
struct Cfg {
    ws: SocketAddr,
    hash: String,
    moves: u32,
    run_secs: f64,
    out: Option<PathBuf>,
    run_id: Option<String>,
    player_id: u64,
}

struct CapturedReceipt {
    connection_epoch: u64,
    seq: u32,
    kind: &'static str,
    reliable: bool,
    message_id: Option<u16>,
    remote_tick: u32,
}

#[derive(Resource)]
struct State {
    started: Instant,
    connected_seen: bool,
    welcomed: bool,
    rejected_reason: Option<String>,
    disconnected_after_connect: bool,
    acks: u32,
    ack_seqs: HashSet<u32>,
    receipt_epoch: Option<u64>,
    receipt_witnesses: Vec<CapturedReceipt>,
    reliable_message_count: u32,
    ack_message_ids: Vec<u16>,
    rejection_message_ids: Vec<u16>,
    typed_command_results: Vec<&'static str>,
    all_acked_at: Option<Instant>,
    next_seq: u32,
    deltas: u64,
    deltas_in_order: bool,
    first_tick: Option<u32>,
    last_tick: Option<u32>,
}

fn main() {
    let cfg = parse_args();
    let mut app = App::new();
    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        ))),
    );
    app.add_plugins(ClientPlugins {
        tick_duration: tick_duration(20),
    });
    app.add_plugins(ProtocolPlugin);
    app.insert_resource(cfg);
    app.insert_resource(State {
        started: Instant::now(),
        connected_seen: false,
        welcomed: false,
        rejected_reason: None,
        disconnected_after_connect: false,
        acks: 0,
        ack_seqs: HashSet::new(),
        receipt_epoch: None,
        receipt_witnesses: Vec::new(),
        reliable_message_count: 0,
        ack_message_ids: Vec::new(),
        rejection_message_ids: Vec::new(),
        typed_command_results: Vec::new(),
        all_acked_at: None,
        next_seq: 0,
        deltas: 0,
        deltas_in_order: true,
        first_tick: None,
        last_tick: None,
    });
    app.add_systems(Startup, start);
    app.add_observer(on_connected);
    app.add_observer(on_disconnected);
    app.add_systems(PreUpdate, receive.after(MessageSystems::Receive));
    app.add_systems(Update, (send_moves, exit_when_done));
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
        hash: value("--hash").unwrap_or_else(|| "sha256:spike".to_string()),
        moves: value("--moves").map_or(0, |v| v.parse().expect("--moves: integer")),
        run_secs: value("--run-secs").map_or(10.0, |v| v.parse().expect("--run-secs: number")),
        out: value("--out").map(PathBuf::from),
        run_id: value("--run-id"),
        // Placeholder identity per process (design 5: random per session).
        player_id: std::process::id() as u64,
    }
}

fn start(mut commands: Commands, cfg: Res<Cfg>) -> Result {
    let ws_addr = cfg.ws;
    let entity = commands
        .spawn((
            Client,
            Link::default(),
            NetcodeClient::new(
                Authentication::Manual {
                    server_addr: ws_addr,
                    client_id: cfg.player_id,
                    private_key: [0u8; 32], // M1 placeholder; format open (spec 15)
                    protocol_id: NET_PROTOCOL_VERSION as u64,
                },
                NetcodeConfig {
                    client_timeout_secs: 10,
                    token_expire_secs: -1,
                    ..Default::default()
                },
            )?,
            WebSocketClientIo::from_url(
                ClientConfig::builder().with_no_encryption(),
                format!("ws://{ws_addr}"),
            ),
        ))
        .id();
    commands.trigger(Connect { entity });
    Ok(())
}

fn on_connected(
    trigger: On<Add<Connected>>,
    mut hello: Query<&mut MessageSender<Hello>>,
    mut state: ResMut<State>,
    cfg: Res<Cfg>,
) {
    state.connected_seen = true;
    state.receipt_epoch = Some(trigger.entity.to_bits());
    state.reliable_message_count = 0;
    if let Ok(mut sender) = hello.get_mut(trigger.entity) {
        sender.send::<ReliableChannel>(Hello {
            player_id: cfg.player_id,
            protocol_version: NET_PROTOCOL_VERSION,
            lockfile_hash: cfg.hash.clone(),
        });
        record_reliable_message(&mut state, ChannelKind::of::<ReliableChannel>());
    }
}

/// lightyear inserts `Disconnected` at spawn, before `Connected`. Count it only after `Connected`.
fn on_disconnected(_trigger: On<Add<Disconnected>>, mut state: ResMut<State>) {
    if state.connected_seen {
        state.disconnected_after_connect = true;
    }
}

fn receive(
    mut state: ResMut<State>,
    mut welcome: Query<&mut MessageReceiver<Welcome>>,
    mut rejected: Query<&mut MessageReceiver<JoinRejected>>,
    mut acks: Query<&mut MessageReceiver<CommandAck>>,
    mut order_results: Query<&mut MessageReceiver<OrderResult>>,
    mut deltas: Query<&mut MessageReceiver<WorldDelta>>,
) {
    let epoch = state.receipt_epoch.unwrap_or_default();
    for mut receiver in welcome.iter_mut() {
        for received in receiver.receive_with_tick() {
            record_reliable_message(&mut state, received.channel_kind);
            state.welcomed = true;
        }
    }
    for mut receiver in rejected.iter_mut() {
        for received in receiver.receive_with_tick() {
            record_reliable_message(&mut state, received.channel_kind);
            state.rejected_reason = Some(received.data.reason);
        }
    }
    for mut receiver in acks.iter_mut() {
        for received in receiver.receive_with_tick() {
            state.acks += 1;
            state.ack_seqs.insert(received.data.seq);
            capture_receipt(
                &mut state,
                epoch,
                received.data.seq,
                "ack",
                received.channel_kind,
                received.message_id,
                received.remote_tick.0,
            );
            if let Some(id) = received.message_id {
                state.ack_message_ids.push(id.0);
            }
            state.typed_command_results.push("ack");
        }
    }
    for mut receiver in order_results.iter_mut() {
        for received in receiver.receive_with_tick() {
            capture_receipt(
                &mut state,
                epoch,
                received.data.seq,
                if received.data.accepted.is_empty() {
                    "rejected"
                } else {
                    "applied"
                },
                received.channel_kind,
                received.message_id,
                received.remote_tick.0,
            );
            if let Some(id) = received.message_id {
                state.rejection_message_ids.push(id.0);
            }
            state
                .typed_command_results
                .push(if received.data.accepted.is_empty() {
                    "rejected"
                } else {
                    "applied"
                });
        }
    }
    for mut receiver in deltas.iter_mut() {
        for delta in receiver.receive() {
            state.deltas += 1;
            if let Some(last) = state.last_tick
                && delta.tick <= last
            {
                state.deltas_in_order = false;
            }
            state.first_tick.get_or_insert(delta.tick);
            state.last_tick = Some(delta.tick);
        }
    }
}

fn capture_receipt(
    state: &mut State,
    epoch: u64,
    seq: u32,
    kind: &'static str,
    channel_kind: ChannelKind,
    message_id: Option<MessageId>,
    remote_tick: u32,
) {
    let reliable = channel_kind == ChannelKind::of::<ReliableChannel>();
    record_reliable_message(state, channel_kind);
    state.receipt_witnesses.push(CapturedReceipt {
        connection_epoch: epoch,
        seq,
        kind,
        reliable,
        message_id: message_id.map(|id| id.0),
        remote_tick,
    });
}

fn record_reliable_message(state: &mut State, channel_kind: ChannelKind) {
    if channel_kind == ChannelKind::of::<ReliableChannel>() {
        state.reliable_message_count = state.reliable_message_count.saturating_add(1);
    }
}

fn send_moves(
    mut state: ResMut<State>,
    mut sender: Query<&mut MessageSender<PlayerCommand>>,
    cfg: Res<Cfg>,
) {
    if !state.welcomed || state.next_seq >= cfg.moves {
        return;
    }
    if let Ok(mut sender) = sender.single_mut() {
        sender.send::<ReliableChannel>(PlayerCommand {
            seq: state.next_seq,
            order: OrderEnvelope {
                order: "spike:move".into(),
                units: vec![UnitId(0)],
                target: OrderTarget::Point(Pos { x: 10.0, y: 10.0 }),
                queue: QueueMode::Replace,
                group: 0,
            },
        });
        record_reliable_message(&mut state, ChannelKind::of::<ReliableChannel>());
        state.next_seq += 1;
    }
}

fn exit_when_done(mut state: ResMut<State>, cfg: Res<Cfg>, mut exit: MessageWriter<AppExit>) {
    if cfg.moves > 0 && state.acks >= cfg.moves && state.all_acked_at.is_none() {
        state.all_acked_at = Some(Instant::now());
    }
    let acked_long_enough = state
        .all_acked_at
        .is_some_and(|at| at.elapsed() >= Duration::from_secs(1));
    if acked_long_enough || state.started.elapsed().as_secs_f64() >= cfg.run_secs {
        // `App::run` hands the world to the runner, so write the result before the exit.
        write_out(&cfg, &state);
        exit.write(AppExit::Success);
    }
}

fn write_out(cfg: &Cfg, state: &State) {
    let out = serde_json::json!({
        "run_id": cfg.run_id,
        "welcomed": state.welcomed,
        "rejected_reason": state.rejected_reason,
        "disconnected_after_connect": state.disconnected_after_connect,
        "acks": state.acks,
        "ack_seqs_unique": state.ack_seqs.len(),
        "connection_epoch": state.receipt_epoch,
        "reliable_message_count": state.reliable_message_count,
        "receipt_witnesses": state.receipt_witnesses.iter().map(|w| serde_json::json!({
            "connection_epoch": w.connection_epoch,
            "seq": w.seq,
            "kind": w.kind,
            "reliable": w.reliable,
            "message_id": w.message_id,
            "remote_tick": w.remote_tick,
        })).collect::<Vec<_>>(),
        "ack_message_ids": state.ack_message_ids,
        "rejection_message_ids": state.rejection_message_ids,
        "typed_command_results": state.typed_command_results,
        "deltas": state.deltas,
        "deltas_in_order": state.deltas_in_order,
        "first_tick": state.first_tick,
        "last_tick": state.last_tick,
    });
    println!("spike_client result {out}");
    if let Some(path) = &cfg.out {
        std::fs::write(path, out.to_string()).expect("write --out file");
    }
}
