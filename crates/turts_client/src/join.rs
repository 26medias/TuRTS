//! Join handshake.
//!
//! The phase machine of the client: acquisition (advertisement, lockfile, packages), then the
//! connection, then `Hello`, then `Joined`. The checks are the pure functions of
//! [`crate::acquire`]. `Welcome` is not handled here: it arrives in the same frame as the first
//! `Snapshot`, so [`crate::ingest`] reads it first (G2b). [`advance_phase`] derives `Joined`
//! from what ingest stored.

use std::net::SocketAddr;

use crate::commands::ClientCommand;
use crate::commands::{
    CommandSendStatus, CommandSequence, PendingCommands, owned_mobile_ids, submit,
};
use crate::ingest::ReceiptWitnesses;
use bevy::prelude::*;
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::prelude::client::{ClientConfig, NetcodeClient, WebSocketClientIo};
use lightyear::prelude::*;
use lightyear::transport::channel::ChannelKind;
use turts_mod_api::LockEntry;
use turts_mod_runtime::archive::PackageArchive;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::messages::{Hello, JoinRejected};
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION, Pos, UnitId};

use crate::acquire::{
    Acquired, accept_package, build_acquired, check_advertisement, check_lockfile,
    packages_to_fetch,
};
use crate::fetch::{FetchTask, fetch_bytes, poll};
use crate::ingest::{IngestHooks, ReplicaState};
use crate::{ClientOpts, PlayerId};

/// Where the client is in the join.
#[derive(Resource, Debug, Clone, PartialEq)]
pub enum Phase {
    FetchAdvertisement,
    FetchLockfile,
    FetchPackages { left: usize },
    Connecting,
    Joining,
    Joined { slot: u8 },
    Failed(String),
}

/// What the client keeps after it accepted the server's packages.
#[derive(Resource)]
pub struct AcquiredPackages(pub Acquired);

/// The facts about the connection that a test reads.
#[derive(Resource, Default, Debug)]
pub struct JoinState {
    pub connected_seen: bool,
    pub disconnected_after_connect: bool,
}

impl JoinState {
    pub fn on_connected(&mut self) {
        self.connected_seen = true;
    }

    /// lightyear adds `Disconnected` when it spawns the client entity, before `Connected`.
    /// That one is not a disconnect, so it is ignored.
    pub fn on_disconnected(&mut self) {
        if self.connected_seen {
            self.disconnected_after_connect = true;
        }
    }
}

/// The downloads in progress and what has been accepted.
#[derive(Resource, Default)]
pub struct Acquisition {
    task: Option<FetchTask>,
    advertisement: Option<Advertisement>,
    /// The packages still to download. The next one is last.
    queue: Vec<LockEntry>,
    /// The package that `task` fetches.
    current: Option<LockEntry>,
    archives: Vec<PackageArchive>,
}

impl Acquisition {
    /// Archives accepted so far. Callers that need the complete validated package pool should
    /// wait for `AcquiredPackages`, which is inserted after acquisition finishes.
    pub fn accepted_archives(&self) -> &[PackageArchive] {
        &self.archives
    }
}

/// The phase after `Welcome`: `Joined` once ingest stored the `Welcome`. `None` means no change.
pub fn advance_phase(phase: &Phase, state: &ReplicaState) -> Option<Phase> {
    match (phase, state.welcome) {
        (Phase::Connecting | Phase::Joining, Some(welcome)) => {
            Some(Phase::Joined { slot: welcome.slot })
        }
        _ => None,
    }
}

/// `Update`: set `Joined` when the `Welcome` has arrived.
pub fn update_phase(mut phase: ResMut<Phase>, state: Res<ReplicaState>) {
    if let Some(next) = advance_phase(&phase, &state) {
        *phase = next;
    }
}

/// The socket address in a WebSocket URL: `ws://host:port/path` or `host:port`. netcode needs
/// an address for the connect token. A name is resolved on native; the browser needs an IP.
pub fn ws_socket_addr(ws_url: &str) -> Result<SocketAddr, String> {
    let rest = ws_url
        .strip_prefix("ws://")
        .or_else(|| ws_url.strip_prefix("wss://"))
        .unwrap_or(ws_url);
    let authority = rest.split('/').next().unwrap_or(rest);
    if let Ok(address) = authority.parse() {
        return Ok(address);
    }
    #[cfg(not(target_family = "wasm"))]
    {
        use std::net::ToSocketAddrs;
        if let Some(address) = authority
            .to_socket_addrs()
            .ok()
            .and_then(|mut addresses| addresses.next())
        {
            return Ok(address);
        }
    }
    Err(format!("cannot read a socket address from `{ws_url}`"))
}

/// The URL for the WebSocket client: a bare `host:port` gets the `ws://` scheme.
fn ws_url(ws_url: &str) -> String {
    if ws_url.starts_with("ws://") || ws_url.starts_with("wss://") {
        ws_url.to_string()
    } else {
        format!("ws://{ws_url}")
    }
}

/// `Update`: fetch and check the advertisement, the lockfile and the packages, in this order.
/// A failed check ends in `Phase::Failed` and nothing more is fetched.
pub fn drive_acquisition(
    mut commands: Commands,
    mut phase: ResMut<Phase>,
    mut acquisition: ResMut<Acquisition>,
    opts: Res<ClientOpts>,
) {
    match phase.clone() {
        Phase::FetchAdvertisement => {
            let url = format!(
                "{}/advertisement.json",
                opts.0.server_url.trim_end_matches('/')
            );
            let task = acquisition.task.get_or_insert_with(|| fetch_bytes(&url));
            let Some(result) = poll(task) else { return };
            acquisition.task = None;
            match result.and_then(|bytes| check_advertisement(&bytes)) {
                Ok(advertisement) => {
                    acquisition.task = Some(fetch_bytes(&advertisement.lockfile_url));
                    acquisition.advertisement = Some(advertisement);
                    *phase = Phase::FetchLockfile;
                }
                Err(error) => *phase = Phase::Failed(error),
            }
        }
        Phase::FetchLockfile => {
            let Some(task) = acquisition.task.as_mut() else {
                return;
            };
            let Some(result) = poll(task) else { return };
            acquisition.task = None;
            let hash = advertised_lockfile_hash(&acquisition);
            let mod_id = acquisition
                .advertisement
                .as_ref()
                .expect("the advertisement was accepted before the lockfile")
                .mod_id
                .clone();
            let lockfile = result.and_then(|bytes| check_lockfile(&bytes, &hash, &mod_id));
            match lockfile {
                Ok(lockfile) => {
                    let mut queue: Vec<LockEntry> =
                        packages_to_fetch(&lockfile).into_iter().cloned().collect();
                    queue.reverse();
                    acquisition.queue = queue;
                    *phase = next_package(&mut acquisition, &mut commands);
                }
                Err(error) => *phase = Phase::Failed(error),
            }
        }
        Phase::FetchPackages { .. } => {
            let Some(task) = acquisition.task.as_mut() else {
                return;
            };
            let Some(result) = poll(task) else { return };
            acquisition.task = None;
            let entry = acquisition.current.take().expect("a package is in flight");
            match result.and_then(|bytes| accept_package(&bytes, &entry)) {
                Ok(archive) => {
                    acquisition.archives.push(archive);
                    *phase = next_package(&mut acquisition, &mut commands);
                }
                Err(error) => *phase = Phase::Failed(error),
            }
        }
        _ => {}
    }
}

fn advertised_lockfile_hash(acquisition: &Acquisition) -> String {
    acquisition
        .advertisement
        .as_ref()
        .map(|advertisement| advertisement.lockfile_hash.clone())
        .expect("the advertisement was accepted before the lockfile")
}

/// Starts the next download. When none is left, builds what the client keeps and moves on to
/// `Connecting`.
fn next_package(acquisition: &mut Acquisition, commands: &mut Commands) -> Phase {
    let Some(entry) = acquisition.queue.pop() else {
        let hash = advertised_lockfile_hash(acquisition);
        return match build_acquired(&acquisition.archives, hash) {
            Ok(acquired) => {
                commands.insert_resource(AcquiredPackages(acquired));
                Phase::Connecting
            }
            Err(error) => Phase::Failed(error),
        };
    };
    let Some(url) = entry.urls.first() else {
        return Phase::Failed(format!("package `{}` has no url", entry.id));
    };
    acquisition.task = Some(fetch_bytes(url));
    acquisition.current = Some(entry);
    Phase::FetchPackages {
        left: acquisition.queue.len() + 1,
    }
}

/// `Update`: in `Phase::Connecting`, spawn the client entity once and connect (the Task 6a
/// shape). The 10 s timeout is in the connect token (design 4.3).
pub fn start_connection(
    mut commands: Commands,
    mut phase: ResMut<Phase>,
    opts: Res<ClientOpts>,
    mut started: Local<bool>,
) {
    if *started || *phase != Phase::Connecting {
        return;
    }
    *started = true;
    if let Err(error) = spawn_client(&mut commands, &opts.0.ws_url) {
        *phase = Phase::Failed(error);
    }
}

// On wasm32 the aeronet `ClientConfig` is a unit struct. `::default()` keeps the spike shape.
#[cfg(target_family = "wasm")]
#[allow(clippy::default_constructed_unit_structs)]
fn websocket_config() -> ClientConfig {
    ClientConfig::default()
}

#[cfg(not(target_family = "wasm"))]
fn websocket_config() -> ClientConfig {
    ClientConfig::builder().with_no_encryption().disable_nagle()
}

fn spawn_client(commands: &mut Commands, ws: &str) -> Result<(), String> {
    let server_addr = ws_socket_addr(ws)?;
    let netcode = NetcodeClient::new(
        Authentication::Manual {
            server_addr,
            // Each connection gets its own netcode id; the player id travels in Hello.
            // Reusing the player id here made the transport refuse a duplicate player
            // before the server could answer with JoinRejected (design section 5).
            client_id: crate::options::random_player_id(),
            private_key: [0u8; 32], // M1 placeholder; format open (spec 15)
            protocol_id: NET_PROTOCOL_VERSION as u64,
        },
        NetcodeConfig {
            client_timeout_secs: 10,
            token_expire_secs: -1,
            ..Default::default()
        },
    )
    .map_err(|error| format!("cannot start the connection: {error:?}"))?;
    let entity = commands
        .spawn((
            Client,
            Link::default(),
            netcode,
            WebSocketClientIo::from_url(websocket_config(), ws_url(ws)),
        ))
        .id();
    commands.trigger(Connect { entity });
    Ok(())
}

/// On `Connected`: send `Hello`. `--hello-hash` and `--protocol-version` replace the real values.
#[allow(clippy::too_many_arguments)]
pub fn on_connected(
    trigger: On<Add<Connected>>,
    mut hello: Query<&mut MessageSender<Hello>>,
    mut join: ResMut<JoinState>,
    mut phase: ResMut<Phase>,
    player: Res<PlayerId>,
    opts: Res<ClientOpts>,
    acquired: Res<AcquiredPackages>,
    mut sequence: ResMut<CommandSequence>,
    mut send_status: ResMut<CommandSendStatus>,
    mut witnesses: ResMut<ReceiptWitnesses>,
) {
    join.on_connected();
    sequence.next = Some(1);
    send_status.reconnect_required = false;
    let epoch = witnesses.start_epoch(trigger.entity.to_bits());
    witnesses.reliable_counts.insert(epoch, 0);
    if let Ok(mut sender) = hello.get_mut(trigger.entity) {
        sender.send::<ReliableChannel>(Hello {
            player_id: player.0,
            protocol_version: opts.0.protocol_version.unwrap_or(NET_PROTOCOL_VERSION),
            lockfile_hash: opts
                .0
                .hello_hash
                .clone()
                .unwrap_or_else(|| acquired.0.lockfile_hash.clone()),
        });
        witnesses.record_message(epoch, true);
    }
    if *phase == Phase::Connecting {
        *phase = Phase::Joining;
    }
}

pub fn on_disconnected(_trigger: On<Add<Disconnected>>, mut join: ResMut<JoinState>) {
    join.on_disconnected();
}

/// `PreUpdate`: `JoinRejected` ends the join with the server's reason.
pub fn receive_rejection(
    mut rejected: Query<(Entity, &mut MessageReceiver<JoinRejected>)>,
    mut phase: ResMut<Phase>,
    mut witnesses: ResMut<ReceiptWitnesses>,
) {
    for (entity, mut receiver) in &mut rejected {
        let epoch = witnesses.epoch_for(entity.to_bits());
        for received in receiver.receive_with_tick() {
            witnesses.record_message(
                epoch,
                received.channel_kind == ChannelKind::of::<ReliableChannel>(),
            );
            if !matches!(*phase, Phase::Failed(_)) {
                *phase = Phase::Failed(received.data.reason);
            }
        }
    }
}

/// `PostUpdate`: `--move X,Y` sends one `MoveTo` (seq 1) after `auto_move_after_ticks` delta
/// ticks since `Welcome`, once the `Snapshot` has told the client which units it owns.
pub fn auto_move(
    opts: Res<ClientOpts>,
    state: Res<ReplicaState>,
    hooks: Res<IngestHooks>,
    mut pending: ResMut<PendingCommands>,
    mut sent: Local<bool>,
    status: Res<CommandSendStatus>,
) {
    if status.reconnect_required {
        return;
    }
    let Some((x, y)) = opts.0.auto_move else {
        return;
    };
    let (Some(welcome), Some(replica), Some(high)) = (
        state.welcome,
        state.replica.as_ref(),
        hooks.highest_delta_tick,
    ) else {
        return;
    };
    if *sent
        || replica.snapshot_tick.is_none()
        || high < welcome.tick + opts.0.auto_move_after_ticks
        || opts
            .0
            .move_release_file
            .as_ref()
            .is_some_and(|path| !move_release_ready(path))
    {
        return;
    }
    let mut unit_ids: Vec<UnitId> = match opts.0.move_unit {
        Some(id) => vec![UnitId(id)],
        None => owned_mobile_ids(&state),
    };
    if unit_ids.is_empty() {
        return;
    }
    unit_ids.sort_by_key(|id| id.0);
    submit(
        &mut pending,
        ClientCommand::MoveTo {
            unit_ids,
            target: Pos { x, y },
        },
    );
    *sent = true;
}

#[cfg(not(target_family = "wasm"))]
fn move_release_ready(path: &std::path::Path) -> bool {
    path.is_file()
}

#[cfg(target_family = "wasm")]
fn move_release_ready(_path: &std::path::Path) -> bool {
    true
}
