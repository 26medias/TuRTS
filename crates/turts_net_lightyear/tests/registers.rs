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

/// Protocol 5 registers five messages after `ResyncRequest`, on both ends.
#[test]
fn protocol_plugin_registers_the_five_messages_of_protocol_5() {
    for app in [server_app(), client_app()] {
        assert!(app.is_message_registered::<LobbyState>());
        assert!(app.is_message_registered::<LobbyAction>());
        assert!(app.is_message_registered::<LobbyRefused>());
        assert!(app.is_message_registered::<MatchResult>());
        assert!(app.is_message_registered::<LeaveMatch>());
    }
}

/// The registration order is one order for both ends: the source of the plugin lists the five
/// messages after `ResyncRequest`, and the channel comment names them.
#[test]
fn the_five_messages_follow_resync_request_in_the_plugin_source() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/protocol_plugin.rs"
    ))
    .unwrap();
    let position = |name: &str| {
        source
            .find(&format!("register_message::<{name}>"))
            .unwrap_or_else(|| panic!("{name} is registered"))
    };
    let order = [
        "ResyncRequest",
        "LobbyState",
        "LobbyAction",
        "LobbyRefused",
        "MatchResult",
        "LeaveMatch",
    ];
    for pair in order.windows(2) {
        assert!(
            position(pair[0]) < position(pair[1]),
            "{} before {}",
            pair[0],
            pair[1]
        );
    }
    let channels =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/channels.rs")).unwrap();
    for name in &order[1..] {
        assert!(channels.contains(name), "the channel comment names {name}");
    }
}

fn net_names(app: &App) -> Vec<&'static str> {
    let registry = app.world().resource::<MessageRegistry>();
    let map = &registry.kind_map;
    (0u16..64)
        .map_while(|id| map.kind(id).and_then(|kind| map.name(kind)))
        .collect()
}

/// The wire index of a message is its place in the registry. Both ends must give the same index
/// to each type, read from the lightyear `MessageRegistry` at run time and not from the plugin
/// source. The five messages of protocol 5 follow `ResyncRequest`.
#[test]
fn the_message_net_ids_are_equal_on_both_ends_and_the_five_follow_resync_request() {
    let client = client_app();
    let server = server_app();
    let c = net_names(&client);
    let s = net_names(&server);
    assert!(s.len() >= 15, "the registry holds the messages: {s:?}");
    assert_eq!(c, s, "the message net ids differ between the ends");
    // The whole list of protocol messages, in net id order: a swap of any two (for example `Hello`
    // and `Welcome`) changes the meaning of a message on the wire, and a client of protocol 4 would
    // read a `Hello` as another message. Lightyear registers its own messages before ours.
    let ours: Vec<&str> = s
        .iter()
        .filter(|name| name.starts_with("turts_net_protocol::"))
        .map(|name| name.rsplit("::").next().unwrap())
        .collect();
    assert_eq!(
        ours,
        [
            "Hello",
            "Welcome",
            "JoinRejected",
            "Snapshot",
            "WorldDelta",
            "PlayerCommand",
            "CommandAck",
            "OrderResult",
            "ResyncRequest",
            "LobbyState",
            "LobbyAction",
            "LobbyRefused",
            "MatchResult",
            "LeaveMatch"
        ]
    );
    let first = s
        .iter()
        .position(|name| name.starts_with("turts_net_protocol::"))
        .unwrap();
    assert!(
        s[first..]
            .iter()
            .all(|name| name.starts_with("turts_net_protocol::")),
        "our messages are contiguous after the ones of lightyear: {s:?}"
    );
}
