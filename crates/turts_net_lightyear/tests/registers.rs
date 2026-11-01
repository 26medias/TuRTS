#![cfg(all(feature = "client", feature = "server"))]

use bevy_app::App;
use bevy_time::TimePlugin;
use lightyear::prelude::*;
use turts_net_lightyear::{ProtocolPlugin, tick_duration};
use turts_net_protocol::messages::*;

fn client_app() -> App {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    app.add_plugins(lightyear::prelude::client::ClientPlugins {
        tick_duration: tick_duration(20),
    });
    app.add_plugins(ProtocolPlugin);
    app
}

fn server_app() -> App {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: tick_duration(20),
    });
    app.add_plugins(ProtocolPlugin);
    app
}

#[test]
fn protocol_plugin_builds_on_client_and_server_apps() {
    let mut app = client_app();
    app.finish();
    app.update();
    let mut app = server_app();
    app.finish();
    app.update();
}

#[test]
fn protocol_plugin_registers_nine_messages() {
    let app = server_app();
    assert!(app.is_message_registered::<Hello>());
    assert!(app.is_message_registered::<Welcome>());
    assert!(app.is_message_registered::<JoinRejected>());
    assert!(app.is_message_registered::<Snapshot>());
    assert!(app.is_message_registered::<WorldDelta>());
    assert!(app.is_message_registered::<PlayerCommand>());
    assert!(app.is_message_registered::<CommandAck>());
    assert!(app.is_message_registered::<OrderResult>());
    assert!(app.is_message_registered::<ResyncRequest>());
    let app = client_app();
    assert!(app.is_message_registered::<Hello>());
    assert!(app.is_message_registered::<Welcome>());
    assert!(app.is_message_registered::<JoinRejected>());
    assert!(app.is_message_registered::<Snapshot>());
    assert!(app.is_message_registered::<WorldDelta>());
    assert!(app.is_message_registered::<PlayerCommand>());
    assert!(app.is_message_registered::<CommandAck>());
    assert!(app.is_message_registered::<OrderResult>());
    assert!(app.is_message_registered::<ResyncRequest>());
}
