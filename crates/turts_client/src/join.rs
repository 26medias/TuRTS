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
    Acquired, VerifiedRootMod, accept_package, build_acquired_with_root, check_advertisement,
    check_lockfile, packages_to_fetch,
};
use crate::fetch::{FetchTask, fetch_bytes, poll};
use crate::ingest::{IngestHooks, ReplicaState};
use crate::rejoin::RejoinController;
use crate::{ClientOpts, PlayerId};

/// Where the client is in the join.
#[derive(Resource, Debug, Clone, PartialEq)]
pub enum Phase {
    FetchAdvertisement,
    FetchLockfile,
    FetchPackages { left: usize },
    VerifyPackage { left: usize },
    Connecting,
    Joining,
    Lobby,
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
    pending_verification: Option<Vec<u8>>,
    archives: Vec<PackageArchive>,
    /// Root ID/version retained only after the advertised, hash-verified lockfile is accepted.
    verified_root_mod: Option<VerifiedRootMod>,
}

impl Acquisition {
    /// Archives accepted so far. Callers that need the complete validated package pool should
    /// wait for `AcquiredPackages`, which is inserted after acquisition finishes.
    pub fn accepted_archives(&self) -> &[PackageArchive] {
        &self.archives
    }

    pub(crate) fn current_package(&self) -> Option<&LockEntry> {
        self.current.as_ref()
    }

    pub(crate) fn package_progress(&self) -> (usize, usize) {
        let total = self.archives.len() + self.queue.len() + usize::from(self.current.is_some());
        (self.archives.len(), total)
    }
}

/// The phase after `Welcome`: `Joined` once ingest stored the `Welcome`. `None` means no change.
pub fn advance_phase(phase: &Phase, state: &ReplicaState) -> Option<Phase> {
    match (phase, state.welcome) {
        (Phase::Connecting | Phase::Joining | Phase::Lobby, Some(welcome)) => {
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

/// The address stored in the netcode token. The WebSocket transport dials the original URL.
/// Native clients resolve hostnames. Browser clients leave DNS to the browser and use a
/// placeholder token address for hostnames. The server already disables token address checks
/// because a proxy's public address can differ from its bind address.
pub fn ws_socket_addr(ws_url: &str) -> Result<SocketAddr, String> {
    let url = parse_websocket_url(ws_url)?;
    #[cfg(target_family = "wasm")]
    return Ok(browser_token_address(&url));
    #[cfg(not(target_family = "wasm"))]
    {
        use std::net::ToSocketAddrs;
        let host = url.host_str().expect("validated WebSocket host");
        // URL host_str retains brackets around IPv6 literals. Host converts them directly.
        match url.host().expect("validated WebSocket host") {
            url::Host::Ipv4(ip) => return Ok((ip, url.port_or_known_default().unwrap()).into()),
            url::Host::Ipv6(ip) => return Ok((ip, url.port_or_known_default().unwrap()).into()),
            url::Host::Domain(_) => {}
        }
        if let Some(address) = (host, url.port_or_known_default().unwrap())
            .to_socket_addrs()
            .ok()
            .and_then(|mut addresses| addresses.next())
        {
            return Ok(address);
        }
        Err(format!("cannot read a socket address from `{ws_url}`"))
    }
}

fn parse_websocket_url(input: &str) -> Result<url::Url, String> {
    let error = || format!("cannot read a WebSocket address from `{input}`");
    let url = url::Url::parse(&ws_url(input)).map_err(|_| error())?;
    if !matches!(url.scheme(), "ws" | "wss")
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port_or_known_default().is_none_or(|port| port == 0)
    {
        return Err(error());
    }
    Ok(url)
}

#[cfg(any(target_family = "wasm", test))]
fn browser_token_address(url: &url::Url) -> SocketAddr {
    let port = url
        .port_or_known_default()
        .expect("validated WebSocket port");
    match url.host().expect("validated WebSocket host") {
        url::Host::Ipv4(ip) => (ip, port).into(),
        url::Host::Ipv6(ip) => (ip, port).into(),
        url::Host::Domain(_) => ([127, 0, 0, 1], port).into(),
    }
}

#[cfg(test)]
mod websocket_address_tests {
    use super::*;

    #[test]
    fn browser_accepts_hostnames_without_resolving_dns() {
        for (input, expected) in [
            ("wss://game.example.invalid/ws?room=two", "127.0.0.1:443"),
            ("ws://localhost:7001/", "127.0.0.1:7001"),
            ("ws://game.example.invalid", "127.0.0.1:80"),
            ("wss://game.example.invalid:8443/ws", "127.0.0.1:8443"),
            ("127.0.0.1:7001", "127.0.0.1:7001"),
            ("wss://[::1]/ws", "[::1]:443"),
        ] {
            let url = parse_websocket_url(input).unwrap();
            assert_eq!(
                browser_token_address(&url),
                expected.parse().unwrap(),
                "{input}"
            );
            assert_eq!(
                url.as_str(),
                url::Url::parse(&ws_url(input)).unwrap().as_str()
            );
        }
    }

    #[test]
    fn invalid_websocket_urls_are_rejected_before_connecting() {
        for input in [
            "",
            "ws://not an address",
            "https://host/ws",
            "ftp://host",
            "ws://host:0",
            "wss://host:99999",
            "ws://user:password@host",
            "ws://host/#fragment",
        ] {
            assert!(parse_websocket_url(input).is_err(), "{input}");
        }
    }

    #[test]
    fn native_numeric_addresses_keep_ports_and_default_ports() {
        for (input, expected) in [
            ("wss://127.0.0.1/ws", "127.0.0.1:443"),
            ("ws://[::1]/ws", "[::1]:80"),
        ] {
            assert_eq!(ws_socket_addr(input).unwrap(), expected.parse().unwrap());
        }
    }
}

/// The URL for the WebSocket client: a bare `host:port` gets the `ws://` scheme.
fn ws_url(ws_url: &str) -> String {
    if ws_url.contains("://") {
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
                    let root_id = mod_id;
                    let root = lockfile
                        .packages
                        .iter()
                        .find(|entry| entry.id == root_id)
                        .map(|entry| VerifiedRootMod {
                            id: entry.id.clone(),
                            version: entry.version.clone(),
                        })
                        .expect("check_lockfile verified the advertised root entry");
                    acquisition.verified_root_mod = Some(root);
                    let mut queue: Vec<LockEntry> =
                        packages_to_fetch(&lockfile).into_iter().cloned().collect();
                    queue.reverse();
                    acquisition.queue = queue;
                    *phase = next_package(&mut acquisition, &mut commands);
                }
                Err(error) => *phase = Phase::Failed(error),
            }
        }
        Phase::FetchPackages { left } => {
            let Some(task) = acquisition.task.as_mut() else {
                return;
            };
            let Some(result) = poll(task) else { return };
            acquisition.task = None;
            match result {
                Ok(bytes) => {
                    acquisition.pending_verification = Some(bytes);
                    *phase = Phase::VerifyPackage { left };
                }
                Err(error) => *phase = Phase::Failed(error),
            }
        }
        Phase::VerifyPackage { .. } => {
            let entry = acquisition.current.take().expect("a package is in flight");
            let bytes = acquisition
                .pending_verification
                .take()
                .expect("downloaded bytes wait for package verification");
            match accept_package(&bytes, &entry) {
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
        return match build_acquired_with_root(
            &acquisition.archives,
            hash,
            acquisition.verified_root_mod.clone(),
        ) {
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

/// `Update`: spawn the first client in `Connecting`, or a bounded stored-identity retry after the
/// rejected transport has disconnected and the one-second cadence has elapsed.
pub fn start_connection(
    mut commands: Commands,
    mut phase: ResMut<Phase>,
    time: Res<Time<Real>>,
    frame_count: Option<Res<bevy::diagnostic::FrameCount>>,
    opts: Res<ClientOpts>,
    mut rejoin: ResMut<RejoinController>,
) {
    let initial = *phase == Phase::Connecting && rejoin.attempts_started() == 0;
    let retry = *phase == Phase::Joining && rejoin.retry_ready();
    if !initial && !retry {
        return;
    }
    match spawn_client(&mut commands, &opts.0.ws_url) {
        Ok(entity) => {
            let real_now_secs = time.elapsed_secs_f64();
            rejoin.observe_deadline_check(
                frame_count.as_deref().map(|frame| frame.0),
                "start_connection",
                real_now_secs,
            );
            if !rejoin.start_attempt_at(entity.to_bits(), real_now_secs) {
                commands.entity(entity).despawn();
            }
        }
        Err(error) => {
            rejoin.terminate_at(error.clone(), time.elapsed_secs_f64());
            *phase = Phase::Failed(error);
        }
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

fn spawn_client(commands: &mut Commands, ws: &str) -> Result<Entity, String> {
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
    Ok(entity)
}

/// On `Connected`: send `Hello`. `--hello-hash` and `--protocol-version` replace the real values.
#[allow(clippy::too_many_arguments)]
pub fn on_connected(
    trigger: On<Add<Connected>>,
    mut hello: Query<&mut MessageSender<Hello>>,
    mut join: ResMut<JoinState>,
    mut phase: ResMut<Phase>,
    mut rejoin: ResMut<RejoinController>,
    time: Res<Time<Real>>,
    frame_count: Option<Res<bevy::diagnostic::FrameCount>>,
    player: Res<PlayerId>,
    opts: Res<ClientOpts>,
    acquired: Res<AcquiredPackages>,
    mut sequence: ResMut<CommandSequence>,
    mut send_status: ResMut<CommandSendStatus>,
    mut witnesses: ResMut<ReceiptWitnesses>,
) {
    let Ok(mut sender) = hello.get_mut(trigger.entity) else {
        return;
    };
    let real_now_secs = time.elapsed_secs_f64();
    let frame_index = frame_count.as_deref().map(|frame| frame.0);
    if matches!(*phase, Phase::Failed(_) | Phase::Joined { .. }) || {
        rejoin.observe_deadline_check(frame_index, "connected", real_now_secs);
        !rejoin.on_connected_at(trigger.entity.to_bits(), real_now_secs)
    } {
        return;
    }
    join.on_connected();
    sequence.next = Some(1);
    send_status.reconnect_required = false;
    let epoch = witnesses.start_epoch(trigger.entity.to_bits());
    witnesses.reliable_counts.insert(epoch, 0);
    sender.send::<ReliableChannel>(Hello {
        player_id: player.0,
        protocol_version: opts.0.protocol_version.unwrap_or(NET_PROTOCOL_VERSION),
        lockfile_hash: opts
            .0
            .hello_hash
            .clone()
            .unwrap_or_else(|| acquired.0.lockfile_hash.clone()),
    });
    rejoin.observe_deadline_check(frame_index, "hello_sent", real_now_secs);
    rejoin.on_hello_sent_at(trigger.entity.to_bits(), real_now_secs);
    witnesses.record_message(epoch, true);
    if *phase == Phase::Connecting {
        *phase = Phase::Joining;
    }
}

pub fn on_disconnected(
    trigger: On<Add<Disconnected>>,
    mut join: ResMut<JoinState>,
    mut rejoin: ResMut<RejoinController>,
    phase: Res<Phase>,
    mut send_status: ResMut<CommandSendStatus>,
) {
    if rejoin.on_disconnected(trigger.entity.to_bits()) {
        join.on_disconnected();
        if matches!(*phase, Phase::Joined { .. } | Phase::Lobby) {
            send_status.reconnect_required = true;
        }
    }
}

/// `PreUpdate`: `JoinRejected` ends the join with the server's reason.
pub fn receive_rejection(
    mut rejected: Query<(Entity, &mut MessageReceiver<JoinRejected>)>,
    mut phase: ResMut<Phase>,
    mut rejoin: ResMut<RejoinController>,
    mut witnesses: ResMut<ReceiptWitnesses>,
    time: Res<Time<Real>>,
    frame_count: Option<Res<bevy::diagnostic::FrameCount>>,
) {
    let real_now_secs = time.elapsed_secs_f64();
    let frame_index = frame_count.as_deref().map(|frame| frame.0);
    for (entity, mut receiver) in &mut rejected {
        let epoch = witnesses.epoch_for(entity.to_bits());
        for received in receiver.receive_with_tick() {
            witnesses.record_message(
                epoch,
                received.channel_kind == ChannelKind::of::<ReliableChannel>(),
            );
            rejoin.observe_deadline_check(frame_index, "join_rejection", real_now_secs);
            apply_join_rejection(
                &mut phase,
                &mut rejoin,
                entity.to_bits(),
                received.data.reason,
                real_now_secs,
            );
        }
    }
}

fn apply_join_rejection(
    phase: &mut Phase,
    rejoin: &mut RejoinController,
    attempt: u64,
    reason: String,
    real_now_secs: f64,
) {
    if matches!(*phase, Phase::Failed(_) | Phase::Joined { .. }) {
        return;
    }
    let _ = rejoin.on_rejection_at(attempt, reason, real_now_secs);
}

/// `Update`: apply Welcome before the retry deadline, or advance the designed 15-second window.
pub fn advance_rejoin(
    time: Res<Time<Real>>,
    frame_count: Option<Res<bevy::diagnostic::FrameCount>>,
    state: Res<ReplicaState>,
    mut phase: ResMut<Phase>,
    mut rejoin: ResMut<RejoinController>,
) {
    let joined = state.welcome.is_some() && matches!(*phase, Phase::Joined { .. });
    if joined || matches!(*phase, Phase::Lobby) {
        rejoin.advance_to(time.elapsed_secs_f64(), true);
        return;
    }
    if let Some(text) = rejoin.terminal_text().map(str::to_owned) {
        *phase = Phase::Failed(text);
        return;
    }
    let real_now_secs = time.elapsed_secs_f64();
    rejoin.observe_deadline_check(
        frame_count.as_deref().map(|frame| frame.0),
        "advance_rejoin",
        real_now_secs,
    );
    if let Some(text) = rejoin.advance_to(real_now_secs, false) {
        *phase = Phase::Failed(text);
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

#[cfg(test)]
mod rejoin_control_tests {
    use super::{Phase, apply_join_rejection};
    use crate::rejoin::{DUPLICATE_PLAYER_REASON, RejoinController};

    #[test]
    fn marked_stored_identity_keeps_duplicate_window_rejection_transient() {
        let mut phase = Phase::Joining;
        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt(1));
        assert!(rejoin.on_connected(1));
        assert!(rejoin.on_hello_sent(1));
        apply_join_rejection(
            &mut phase,
            &mut rejoin,
            1,
            DUPLICATE_PLAYER_REASON.to_owned(),
            0.0,
        );
        assert_eq!(
            phase,
            Phase::Joining,
            "the same stored browser identity must remain in its bounded rejoin phase after the live-session refusal"
        );
        assert!(rejoin.on_disconnected(1));
        rejoin.advance(1.0, false);
        assert!(
            rejoin.retry_ready(),
            "the marked duplicate refusal releases one new transport attempt at the next cadence"
        );
    }

    #[test]
    fn stored_transport_without_a_hello_still_shows_and_expires_the_real_deadline() {
        use std::time::Duration;

        use crate::ingest::ReplicaState;
        use bevy::{app::App, prelude::*, time::TimeUpdateStrategy};

        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt(71));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .insert_resource(Phase::Joining)
            .insert_resource(ReplicaState::default())
            .insert_resource(rejoin)
            .add_systems(Update, super::advance_rejoin);
        // Bevy's first real-time update establishes its clock and has a zero delta.
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            14_999,
        )));
        app.update();
        assert_eq!(*app.world().resource::<Phase>(), Phase::Joining);
        assert_eq!(
            app.world()
                .resource::<RejoinController>()
                .visible_text()
                .as_deref(),
            Some("Reconnecting to your game\n1 s"),
            "the first actual transport attempt shows countdown before Connected or Hello"
        );
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(1)));
        app.update();
        assert_eq!(
            *app.world().resource::<Phase>(),
            Phase::Failed(crate::rejoin::REJOIN_FINAL_TEXT.to_owned()),
            "the scheduled client phase publishes the exact design text at 15 seconds"
        );
    }

    #[test]
    fn pre_update_connected_after_the_real_deadline_cannot_send_hello() {
        use std::time::Duration;

        use crate::ingest::ReplicaState;
        use bevy::{app::App, prelude::*, time::TimeUpdateStrategy};

        #[derive(Resource, Default)]
        struct Probe {
            attempted: bool,
            connected: bool,
            hello_sent: bool,
        }

        fn late_connected(
            time: Res<Time<Real>>,
            mut rejoin: ResMut<RejoinController>,
            mut probe: ResMut<Probe>,
        ) {
            let now = time.elapsed_secs_f64();
            if !probe.attempted && now >= crate::rejoin::REJOIN_WINDOW_SECS {
                probe.attempted = true;
                probe.connected = rejoin.on_connected_at(72, now);
                if probe.connected {
                    probe.hello_sent = rejoin.on_hello_sent_at(72, now);
                }
            }
        }

        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt_at(72, 0.0));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .insert_resource(Phase::Joining)
            .insert_resource(ReplicaState::default())
            .insert_resource(rejoin)
            .init_resource::<Probe>()
            .add_systems(PreUpdate, late_connected)
            .add_systems(Update, super::advance_rejoin);
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(15)));
        app.update();

        let probe = app.world().resource::<Probe>();
        assert!(
            probe.attempted,
            "the simulated Lightyear event ran in PreUpdate"
        );
        assert!(
            !probe.connected,
            "the deadline guard rejects late Connected"
        );
        assert!(
            !probe.hello_sent,
            "a late Connected event cannot send Hello"
        );
        assert_eq!(
            *app.world().resource::<Phase>(),
            Phase::Failed(crate::rejoin::REJOIN_FINAL_TEXT.to_owned())
        );
    }
}

#[cfg(test)]
mod independent_hostname_review {
    use super::*;

    #[test]
    fn accepted_endpoint_and_transport_endpoint_must_match() {
        for input in ["WS://127.0.0.1:7001/path?room=two", "WSS://127.0.0.1/ws"] {
            let validated = parse_websocket_url(input).unwrap();
            let transport = url::Url::parse(&ws_url(input)).unwrap();
            assert_eq!(
                transport, validated,
                "validated endpoint differs from transport for {input}"
            );
        }
    }
}
