//! Lightyear server listener.
//!
//! The setup follows the networking spike (`spike_server`): one `Server` entity with a
//! `NetcodeServer` and a WebSocket endpoint, started with a `Start` trigger.

use std::net::SocketAddr;

use bevy::prelude::*;
use lightyear::netcode::NetcodeServer;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use turts_net_protocol::NET_PROTOCOL_VERSION;

/// The addresses that the server announces in its `LISTENING` line.
#[derive(Resource)]
pub struct Listening {
    pub http: SocketAddr,
    pub ws: SocketAddr,
}

/// Start the lightyear server in the first frame, and print `LISTENING` when the WebSocket
/// port is bound.
pub fn install(app: &mut App, http: SocketAddr, ws: SocketAddr) {
    app.insert_resource(Listening { http, ws });
    app.add_systems(Startup, start);
    app.add_observer(on_linked);
}

fn start(mut commands: Commands, listening: Res<Listening>) {
    spawn_lightyear_server(&mut commands, listening.ws);
}

/// Spawn the `Server` entity and trigger `Start`.
///
/// `server_addr_check` is off: the client's token names the address that the client dials,
/// and the bind address can differ. The server sets no timeout: with `Manual` tokens the
/// timeout comes from the token, and the client sets 10 s there (design 4.3).
pub fn spawn_lightyear_server(commands: &mut Commands, ws_addr: SocketAddr) {
    let entity = commands
        .spawn((
            LocalAddr(ws_addr),
            Server,
            NetcodeServer::new(NetcodeConfig {
                protocol_id: NET_PROTOCOL_VERSION as u64,
                private_key: [0u8; 32], // M1 placeholder; format open (spec 15)
                server_addr_check: false,
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

/// The transport inserts `Linked` on the server entity when it has bound the port. `Started`
/// is added earlier, before the bind is complete, so it is not the readiness signal.
fn on_linked(
    trigger: On<Add<Linked>>,
    servers: Query<(), With<Server>>,
    listening: Res<Listening>,
) {
    if servers.get(trigger.entity).is_ok() {
        println!("LISTENING http={} ws={}", listening.http, listening.ws);
    }
}
