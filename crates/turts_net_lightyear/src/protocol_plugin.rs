//! Message and channel registration.
//!
//! lightyear gives ids in registration order. The client and the server must register in
//! the same order, so both ends use this one plugin.

use core::time::Duration;

use bevy_app::{App, Plugin};
use lightyear::prelude::*;
use turts_net_protocol::messages::*;

use crate::channels::{ReliableChannel, StateChannel};

/// Registers the two channels and the nine messages, in a fixed order.
/// Add it after `ClientPlugins` or `ServerPlugins` and before any `Client` or `Server` entity.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Channels first.
        app.add_channel::<ReliableChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..Default::default()
        })
        .add_direction(NetworkDirection::Bidirectional);
        // Unordered, not sequenced: lightyear 0.31 sequenced channels keep only the newest
        // unread message, so a burst of deltas loses all but one inside the library. The
        // client's `base_tick` chain and resync guard against real loss (design 5).
        app.add_channel::<StateChannel>(ChannelSettings {
            mode: ChannelMode::UnorderedUnreliable,
            send_frequency: Duration::default(),
            ..Default::default()
        })
        .add_direction(NetworkDirection::ServerToClient);
        // Messages next. Do not reorder: the order assigns the wire ids.
        app.register_message::<Hello>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<Welcome>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<JoinRejected>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<Snapshot>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<UnitDeltas>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<MoveTo>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<CommandAck>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<CommandRejected>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<ResyncRequest>()
            .add_direction(NetworkDirection::ClientToServer);
    }
}
