//! Message and channel registration.
//!
//! lightyear assigns ids in registration order. Both ends use this plugin.

use crate::channels::{ReliableChannel, StateChannel};
use bevy_app::{App, Plugin};
use core::time::Duration;
use lightyear::prelude::*;
use turts_net_protocol::messages::*;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_channel::<ReliableChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..Default::default()
        })
        .add_direction(NetworkDirection::Bidirectional);
        app.add_channel::<StateChannel>(ChannelSettings {
            mode: ChannelMode::UnorderedUnreliable,
            send_frequency: Duration::default(),
            ..Default::default()
        })
        .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<Hello>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<Welcome>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<JoinRejected>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<Snapshot>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<WorldDelta>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<PlayerCommand>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<CommandAck>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<CommandApplied>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<CommandRejected>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<ResyncRequest>()
            .add_direction(NetworkDirection::ClientToServer);
    }
}
