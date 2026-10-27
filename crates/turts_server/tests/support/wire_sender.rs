#![allow(dead_code)]
//! Test-only real Lightyear WebSocket sender for diagnostic PlayerCommand envelopes.
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::app::{App, ScheduleRunnerPlugin};
use bevy::prelude::*;
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use lightyear::transport::channel::ChannelKind;
use turts_net_lightyear::{ProtocolPlugin, ReliableChannel, tick_duration};
use turts_net_protocol::{
    CommandApplied, CommandRejected, Hello, JoinRejected, NET_PROTOCOL_VERSION, PlayerCommand,
    Snapshot, Welcome, WorldDelta,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Receipt {
    pub seq: u32,
    pub kind: &'static str,
    pub reliable: bool,
    pub message_id: Option<u16>,
    pub remote_tick: u32,
    pub reason: Option<turts_net_protocol::CommandRejectReason>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AttemptInterval {
    pub index: usize,
    pub seq: u32,
    /// Receipt-vector cursor captured immediately before this command is submitted.
    pub start_cursor: usize,
    /// Absolute receipt cursor of the terminal Applied/Rejected response.
    pub terminal_cursor: Option<usize>,
    /// Absolute receipt cursor of the receipt Ack, even if it follows the terminal.
    pub ack_cursor: Option<usize>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Report {
    pub welcomed: bool,
    pub welcome_slot: Option<u8>,
    pub join_rejected: Option<String>,
    pub receipts: Vec<Receipt>,
    pub attempts: Vec<AttemptInterval>,
    pub reliable_count: u32,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Copy)]
struct PendingAttempt {
    index: usize,
    seq: u32,
    start_cursor: usize,
}

fn terminal_cursor_after(receipts: &[Receipt], seq: u32, start: usize) -> Option<usize> {
    receipts
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, receipt)| receipt.seq == seq && matches!(receipt.kind, "applied" | "rejected"))
        .map(|(cursor, _)| cursor)
}

fn ack_cursor_after(receipts: &[Receipt], seq: u32, start: usize) -> Option<usize> {
    receipts
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, receipt)| receipt.seq == seq && receipt.kind == "ack")
        .map(|(cursor, _)| cursor)
}

fn all_attempts_terminal(attempts: &[AttemptInterval], expected: usize) -> bool {
    attempts.len() == expected
        && attempts
            .iter()
            .all(|attempt| attempt.terminal_cursor.is_some() && attempt.ack_cursor.is_some())
}

#[derive(Resource)]
struct Config {
    ws: SocketAddr,
    player_id: u64,
    hash: String,
    protocol_version: u32,
    commands: Vec<PlayerCommand>,
    report: Arc<Mutex<Report>>,
}

#[derive(Resource)]
struct State {
    started: Instant,
    welcomed: bool,
    next_command: usize,
    pending: Option<PendingAttempt>,
    join_rejected: bool,
    report: Arc<Mutex<Report>>,
}

pub fn send_commands(
    http_url: &str,
    ws: &str,
    player_id: u64,
    commands: Vec<PlayerCommand>,
) -> Report {
    send_commands_with_hello(
        http_url,
        ws,
        player_id,
        NET_PROTOCOL_VERSION,
        None,
        commands,
    )
}

pub fn send_commands_with_protocol(
    http_url: &str,
    ws: &str,
    player_id: u64,
    protocol_version: u32,
    commands: Vec<PlayerCommand>,
) -> Report {
    send_commands_with_hello(http_url, ws, player_id, protocol_version, None, commands)
}

pub fn send_commands_with_hello(
    http_url: &str,
    ws: &str,
    player_id: u64,
    protocol_version: u32,
    lockfile_hash_override: Option<String>,
    commands: Vec<PlayerCommand>,
) -> Report {
    let advertisement_text = ureq::get(&format!("{http_url}/advertisement.json"))
        .call()
        .expect("read advertisement")
        .into_body()
        .read_to_string()
        .expect("read advertisement body");
    let advertisement: turts_net_protocol::Advertisement =
        serde_json::from_str(&advertisement_text).expect("decode advertisement");
    let report = Arc::new(Mutex::new(Report::default()));
    let config = Config {
        ws: ws.parse().expect("WebSocket listen address"),
        player_id,
        hash: lockfile_hash_override.unwrap_or(advertisement.lockfile_hash),
        protocol_version,
        commands,
        report: report.clone(),
    };
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
    app.insert_resource(config);
    app.insert_resource(State {
        started: Instant::now(),
        welcomed: false,
        next_command: 0,
        pending: None,
        join_rejected: false,
        report: report.clone(),
    });
    app.add_systems(Startup, start);
    app.add_observer(on_connected);
    app.add_systems(PreUpdate, receive.after(MessageSystems::Receive));
    app.add_systems(Update, (send_after_welcome, exit_when_done).chain());
    app.run();
    let result = report.lock().expect("wire sender report lock").clone();
    let path = std::env::temp_dir().join(format!("turts-wire-report-{player_id}.json"));
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&result).expect("wire report serializes"),
    )
    .unwrap_or_else(|error| {
        panic!(
            "persist raw wire receipt report to {}: {error}",
            path.display()
        )
    });
    result
}

fn start(mut commands: Commands, config: Res<Config>) -> Result {
    let entity = commands
        .spawn((
            Client,
            Link::default(),
            NetcodeClient::new(
                Authentication::Manual {
                    server_addr: config.ws,
                    client_id: config.player_id,
                    private_key: [0; 32],
                    protocol_id: NET_PROTOCOL_VERSION as u64,
                },
                NetcodeConfig {
                    client_timeout_secs: 10,
                    token_expire_secs: -1,
                    ..default()
                },
            )?,
            WebSocketClientIo::from_url(
                ClientConfig::builder().with_no_encryption(),
                format!("ws://{}", config.ws),
            ),
        ))
        .id();
    commands.trigger(Connect { entity });
    Ok(())
}

fn on_connected(
    trigger: On<Add<Connected>>,
    mut hello: Query<&mut MessageSender<Hello>>,
    config: Res<Config>,
    state: Res<State>,
) {
    if let Ok(mut sender) = hello.get_mut(trigger.entity) {
        sender.send::<ReliableChannel>(Hello {
            player_id: config.player_id,
            protocol_version: config.protocol_version,
            lockfile_hash: config.hash.clone(),
        });
        let report_arc = state.report.clone();
        let mut report = report_arc.lock().expect("wire sender report lock");
        report.reliable_count = report.reliable_count.saturating_add(1);
    }
}

#[allow(clippy::too_many_arguments)] // Bevy receiver resources remain explicit per message type.
fn receive(
    mut state: ResMut<State>,
    mut welcome: Query<&mut MessageReceiver<Welcome>>,
    mut rejected: Query<&mut MessageReceiver<JoinRejected>>,
    mut acks: Query<&mut MessageReceiver<turts_net_protocol::CommandAck>>,
    mut applied: Query<&mut MessageReceiver<CommandApplied>>,
    mut rejections: Query<&mut MessageReceiver<CommandRejected>>,
    mut snapshots: Query<&mut MessageReceiver<Snapshot>>,
    mut deltas: Query<&mut MessageReceiver<WorldDelta>>,
) {
    let report_arc = state.report.clone();
    let mut report = report_arc.lock().expect("wire sender report lock");
    for mut receiver in welcome.iter_mut() {
        for message in receiver.receive_with_tick() {
            report.reliable_count = report.reliable_count.saturating_add(u32::from(
                message.channel_kind == ChannelKind::of::<ReliableChannel>(),
            ));
            state.welcomed = true;
            report.welcomed = true;
            report.welcome_slot = Some(message.data.slot_id.0);
        }
    }
    for mut receiver in rejected.iter_mut() {
        for message in receiver.receive_with_tick() {
            report.reliable_count = report.reliable_count.saturating_add(u32::from(
                message.channel_kind == ChannelKind::of::<ReliableChannel>(),
            ));
            report.join_rejected = Some(message.data.reason);
            state.join_rejected = true;
        }
    }
    for mut receiver in acks.iter_mut() {
        for message in receiver.receive_with_tick() {
            let reliable = message.channel_kind == ChannelKind::of::<ReliableChannel>();
            report.reliable_count = report.reliable_count.saturating_add(u32::from(reliable));
            report.receipts.push(Receipt {
                seq: message.data.seq,
                kind: "ack",
                reliable,
                message_id: message.message_id.map(|id| id.0),
                remote_tick: message.remote_tick.0,
                reason: None,
            });
        }
    }
    for mut receiver in applied.iter_mut() {
        for message in receiver.receive_with_tick() {
            let reliable = message.channel_kind == ChannelKind::of::<ReliableChannel>();
            report.reliable_count = report.reliable_count.saturating_add(u32::from(reliable));
            report.receipts.push(Receipt {
                seq: message.data.seq,
                kind: "applied",
                reliable,
                message_id: message.message_id.map(|id| id.0),
                remote_tick: message.remote_tick.0,
                reason: None,
            });
        }
    }
    for mut receiver in rejections.iter_mut() {
        for message in receiver.receive_with_tick() {
            let reliable = message.channel_kind == ChannelKind::of::<ReliableChannel>();
            report.reliable_count = report.reliable_count.saturating_add(u32::from(reliable));
            report.receipts.push(Receipt {
                seq: message.data.seq,
                kind: "rejected",
                reliable,
                message_id: message.message_id.map(|id| id.0),
                remote_tick: message.remote_tick.0,
                reason: Some(message.data.reason),
            });
        }
    }
    for mut receiver in snapshots.iter_mut() {
        for message in receiver.receive_with_tick() {
            report.reliable_count = report.reliable_count.saturating_add(u32::from(
                message.channel_kind == ChannelKind::of::<ReliableChannel>(),
            ));
        }
    }
    for mut receiver in deltas.iter_mut() {
        for message in receiver.receive_with_tick() {
            report.reliable_count = report.reliable_count.saturating_add(u32::from(
                message.channel_kind == ChannelKind::of::<ReliableChannel>(),
            ));
        }
    }
}

fn send_after_welcome(
    mut state: ResMut<State>,
    config: Res<Config>,
    mut sender: Query<&mut MessageSender<PlayerCommand>>,
) {
    if !state.welcomed {
        return;
    }
    let report_arc = state.report.clone();
    if let Some(pending) = state.pending {
        let completed = {
            let report = report_arc.lock().expect("wire sender report lock");
            (
                terminal_cursor_after(&report.receipts, pending.seq, pending.start_cursor),
                ack_cursor_after(&report.receipts, pending.seq, pending.start_cursor),
            )
        };
        if let (Some(terminal_cursor), Some(ack_cursor)) = completed {
            let mut report = report_arc.lock().expect("wire sender report lock");
            if let Some(attempt) = report.attempts.get_mut(pending.index) {
                attempt.terminal_cursor = Some(terminal_cursor);
                attempt.ack_cursor = Some(ack_cursor);
            }
            state.pending = None;
            state.next_command = pending.index + 1;
        }
    }
    if state.pending.is_some() || state.next_command >= config.commands.len() {
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    let index = state.next_command;
    let command = config.commands[index].clone();
    let mut report = report_arc.lock().expect("wire sender report lock");
    let start_cursor = report.receipts.len();
    sender.send::<ReliableChannel>(command.clone());
    report.reliable_count = report.reliable_count.saturating_add(1);
    report.attempts.push(AttemptInterval {
        index,
        seq: command.seq,
        start_cursor,
        terminal_cursor: None,
        ack_cursor: None,
    });
    state.pending = Some(PendingAttempt {
        index,
        seq: command.seq,
        start_cursor,
    });
}

fn exit_when_done(state: Res<State>, config: Res<Config>, mut exit: MessageWriter<AppExit>) {
    let done = state.join_rejected
        || (state.welcomed
            && state.pending.is_none()
            && state.next_command == config.commands.len()
            && all_attempts_terminal(
                &state
                    .report
                    .lock()
                    .expect("wire sender report lock")
                    .attempts,
                config.commands.len(),
            ));
    let timed_out = state.started.elapsed() >= Duration::from_secs(8);
    if done || timed_out {
        if timed_out {
            state
                .report
                .lock()
                .expect("wire sender report lock")
                .timed_out = true;
        }
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptInterval, Receipt, ack_cursor_after, all_attempts_terminal, terminal_cursor_after,
    };

    fn receipt(seq: u32, kind: &'static str) -> Receipt {
        Receipt {
            seq,
            kind,
            reliable: true,
            message_id: Some(1),
            remote_tick: 7,
            reason: None,
        }
    }

    #[test]
    fn acknowledgements_and_partial_submissions_are_not_terminal() {
        let receipts = [receipt(1, "ack"), receipt(1, "applied")];
        assert_eq!(terminal_cursor_after(&receipts[..1], 1, 0), None);
        assert_eq!(ack_cursor_after(&receipts[..1], 1, 0), Some(0));
        let mut attempts = [AttemptInterval {
            index: 0,
            seq: 1,
            start_cursor: 0,
            terminal_cursor: Some(1),
            ack_cursor: None,
        }];
        assert!(!all_attempts_terminal(&attempts, 1));
        attempts[0].ack_cursor = Some(0);
        assert!(all_attempts_terminal(&attempts, 1));
    }

    #[test]
    fn repeated_sequence_requires_a_fresh_terminal_after_its_send_cursor() {
        let receipts = vec![receipt(1, "ack"), receipt(1, "rejected")];
        assert_eq!(terminal_cursor_after(&receipts, 1, 0), Some(1));
        assert_eq!(terminal_cursor_after(&receipts, 1, 2), None);
        let mut after_repeat = receipts;
        after_repeat.push(receipt(1, "ack"));
        after_repeat.push(receipt(1, "rejected"));
        assert_eq!(terminal_cursor_after(&after_repeat, 1, 2), Some(3));
        assert_eq!(ack_cursor_after(&after_repeat, 1, 2), Some(2));
    }
}
