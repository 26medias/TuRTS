#![cfg(all(feature = "client", feature = "server"))]

use bevy_app::App;
use bevy_ecs::prelude::Entity;
use bevy_time::TimePlugin;
use lightyear::prelude::server::ClientOf;
use lightyear::prelude::*;
use std::any::type_name;
use std::time::Duration;
use turts_net_lightyear::{ReliableChannel, StateChannel, staged_v2::StagedV2ProtocolPlugin};
use turts_net_protocol::messages::{CommandAck, Hello, JoinRejected, ResyncRequest, Welcome};
use turts_net_protocol::staged_v2::{
    CommandApplied, CommandRejected, PlayerCommand, Snapshot, WorldDelta,
};

fn client_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    app.add_plugins(lightyear::prelude::client::ClientPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(StagedV2ProtocolPlugin);
    let entity = app.world_mut().spawn((Client, Transport::default())).id();
    app.update();
    (app, entity)
}

fn server_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(StagedV2ProtocolPlugin);
    let entity = app.world_mut().spawn((ClientOf, Transport::default())).id();
    app.update();
    (app, entity)
}

#[test]
fn staged_v2_registry_has_the_approved_ten_message_order() {
    let expected = vec![
        type_name::<Hello>(),
        type_name::<Welcome>(),
        type_name::<JoinRejected>(),
        type_name::<Snapshot>(),
        type_name::<WorldDelta>(),
        type_name::<PlayerCommand>(),
        type_name::<CommandAck>(),
        type_name::<CommandApplied>(),
        type_name::<CommandRejected>(),
        type_name::<ResyncRequest>(),
    ];
    let (client, _) = client_app();
    let (server, _) = server_app();
    for app in [&client, &server] {
        let registry = app.world().resource::<MessageRegistry>();
        let registered = (0..32)
            .filter_map(|id| registry.kind_map.kind(id))
            .map(|kind| {
                registry
                    .kind_map
                    .name(kind)
                    .expect("registered message name")
            })
            .filter(|name| name.starts_with("turts_net_protocol::"))
            .collect::<Vec<_>>();
        assert_eq!(registered, expected);
    }
}

#[test]
fn staged_v2_message_directions_match_the_declared_sender_sides() {
    let (client, client_entity) = client_app();
    let (server, server_entity) = server_app();

    macro_rules! assert_direction {
        ($message:ty, $client_to_server:expr, $server_to_client:expr) => {
            assert!(client.is_message_registered::<$message>());
            assert!(server.is_message_registered::<$message>());
            assert_eq!(
                client
                    .world()
                    .get::<MessageSender<$message>>(client_entity)
                    .is_some(),
                $client_to_server,
                "client sender for {}",
                type_name::<$message>()
            );
            assert_eq!(
                server
                    .world()
                    .get::<MessageSender<$message>>(server_entity)
                    .is_some(),
                $server_to_client,
                "server sender for {}",
                type_name::<$message>()
            );
        };
    }

    assert_direction!(Hello, true, false);
    assert_direction!(Welcome, false, true);
    assert_direction!(JoinRejected, false, true);
    assert_direction!(Snapshot, false, true);
    assert_direction!(WorldDelta, false, true);
    assert_direction!(PlayerCommand, true, false);
    assert_direction!(CommandAck, false, true);
    assert_direction!(CommandApplied, false, true);
    assert_direction!(CommandRejected, false, true);
    assert_direction!(ResyncRequest, true, false);
}

#[test]
fn staged_v2_channels_have_the_required_modes_and_directions() {
    let (client, client_entity) = client_app();
    let (server, server_entity) = server_app();

    let channels = server.world().resource::<ChannelRegistry>();
    let channel_kinds = channels.kind_map();
    let channel_kind = |name| {
        (0..32)
            .filter_map(|id| channel_kinds.kind(id))
            .find(|kind| channel_kinds.name(kind) == Some(name))
            .copied()
            .expect("registered staged channel")
    };
    let reliable_kind = channel_kind(type_name::<ReliableChannel>());
    let state_kind = channel_kind(type_name::<StateChannel>());
    assert!(matches!(
        channels.settings(reliable_kind).unwrap().mode,
        ChannelMode::OrderedReliable(_)
    ));
    assert!(matches!(
        channels.settings(state_kind).unwrap().mode,
        ChannelMode::UnorderedUnreliable
    ));

    let client_transport = client.world().get::<Transport>(client_entity).unwrap();
    assert!(client_transport.has_channel_send::<ReliableChannel>());
    assert!(client_transport.has_channel_receive::<ReliableChannel>());
    assert!(!client_transport.has_channel_send::<StateChannel>());
    assert!(client_transport.has_channel_receive::<StateChannel>());

    let server_transport = server.world().get::<Transport>(server_entity).unwrap();
    assert!(server_transport.has_channel_send::<ReliableChannel>());
    assert!(server_transport.has_channel_receive::<ReliableChannel>());
    assert!(server_transport.has_channel_send::<StateChannel>());
    assert!(!server_transport.has_channel_receive::<StateChannel>());
}
