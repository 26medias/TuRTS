//! Nonselected protocol 2 Lightyear registration for integration preparation.

use core::time::Duration;

use bevy_app::{App, Plugin};
use lightyear::prelude::*;
use turts_net_protocol::messages::{CommandAck, Hello, JoinRejected, ResyncRequest, Welcome};
use turts_net_protocol::staged_v2::{
    CommandApplied, CommandRejected, PlayerCommand, Snapshot, WorldDelta,
};

use crate::channels::{ReliableChannel, StateChannel};

/// Registers the staged protocol 2 messages without selecting protocol 2 in the live plugin.
///
/// Use this plugin in place of [`crate::ProtocolPlugin`] for staged registry checks. The
/// existing handshake messages retain their current types and directions. Send all messages
/// except `WorldDelta` on `ReliableChannel`; send `WorldDelta` on `StateChannel`.
pub struct StagedV2ProtocolPlugin;

impl Plugin for StagedV2ProtocolPlugin {
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
