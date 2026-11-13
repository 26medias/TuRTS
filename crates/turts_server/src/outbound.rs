//! Protocol 4 snapshots, world deltas, and fixed-tick order results.

use crate::export;
use crate::session::PendingOrderCommands;
use crate::session::{ReliableTraffic, Session};
use crate::stats::Stats;
use bevy::prelude::*;
use lightyear::prelude::*;
use turts_net_lightyear::{ReliableChannel, StateChannel};
use turts_net_protocol::messages::*;
use turts_sim::{CommandResults, Tick as SimTick, committed_tick, completed_world};

pub fn install(app: &mut App) {
    app.add_systems(
        FixedLast,
        (send_command_results, send_deltas, send_final_snapshots).chain(),
    );
}

#[derive(Component)]
struct FinalSnapshotSent;

/// A result ends the tick stream. Send its complete state on the reliable channel so that
/// loss of the last unreliable delta cannot hide the result or leave the destroyed base alive.
pub fn send_final_snapshots(world: &mut World) {
    if !world
        .get_resource::<turts_sim::outcome::MatchOutcomes>()
        .is_some_and(|state| state.end_tick.is_some())
    {
        return;
    }
    let recipients = world
        .iter_entities()
        .filter_map(|entity| {
            (!entity.contains::<FinalSnapshotSent>()
                && entity.contains::<MessageSender<Snapshot>>())
            .then(|| {
                entity
                    .get::<Session>()
                    .map(|session| (entity.id(), session.slot))
            })
            .flatten()
        })
        .collect::<Vec<_>>();
    for (entity, slot) in recipients {
        let snapshot = export::snapshot_for(completed_world(world), slot);
        world
            .get_mut::<MessageSender<Snapshot>>(entity)
            .unwrap()
            .send::<ReliableChannel>(snapshot);
        world.entity_mut(entity).insert(FinalSnapshotSent);
        world.resource_mut::<ReliableTraffic>().record(entity);
    }
}

pub fn send_command_results(
    results: Res<CommandResults>,
    tick: Res<SimTick>,
    mut sessions: Query<(Entity, &Session, &mut MessageSender<OrderResult>)>,
    mut pending: ResMut<PendingOrderCommands>,
    mut traffic: ResMut<ReliableTraffic>,
) {
    let mut sent = std::collections::BTreeSet::new();
    for result in &results.0 {
        let key = (result.slot.0, result.seq);
        if !sent.insert(key) {
            continue;
        }
        let Some(command) = pending.0.remove(&key) else {
            continue;
        };
        let command_results: Vec<_> = results
            .0
            .iter()
            .filter(|candidate| candidate.slot == result.slot && candidate.seq == result.seq)
            .cloned()
            .collect();
        let mut message = crate::orders::to_wire(&command_results, &command);
        message.tick = tick.0;
        for (entity, session, mut sender) in &mut sessions {
            if session.slot != result.slot {
                continue;
            }
            sender.send::<ReliableChannel>(message.clone());
            traffic.record(entity);
        }
    }
}

pub fn send_deltas(world: &mut World) {
    // Only the reliable full snapshot can publish the terminal tick. An empty repeated delta
    // could otherwise advance the replica to that tick before the final snapshot arrives.
    if world
        .get_resource::<turts_sim::outcome::MatchOutcomes>()
        .is_some_and(|state| state.end_tick.is_some())
    {
        return;
    }
    if committed_tick(world).tick == 0 {
        return;
    }
    let recipients: Vec<(Entity, turts_sim::SlotId)> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<Session>()
                .map(|session| (entity.id(), session.slot))
        })
        .collect();
    if recipients.is_empty() {
        return;
    }
    let payloads: Vec<_> = {
        let batch = committed_tick(world);
        let view = completed_world(world);
        debug_assert_eq!(batch.tick, view.tick);
        recipients
            .into_iter()
            .map(|(entity, slot)| {
                let delta = export::delta_for(batch, view, slot);
                let bytes = payload_bytes(&delta);
                (entity, delta, bytes)
            })
            .collect()
    };
    for (entity, delta, bytes) in payloads {
        if let Some(mut sender) = world.get_mut::<MessageSender<WorldDelta>>(entity) {
            sender.send::<StateChannel>(delta);
            world.resource_mut::<Stats>().delta_payload_bytes_total += bytes;
        }
    }
}

fn payload_bytes<T: serde::Serialize>(message: &T) -> u64 {
    postcard::experimental::serialized_size(message).expect("a wire message serializes to postcard")
        as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightyear::link::recv_payload_from_bytes;
    use lightyear::prelude::server::{ClientOf, ServerPlugins};
    use lightyear::transport::channel::ChannelKind;
    use std::time::Duration;

    #[test]
    fn terminal_tick_reaches_the_wire_as_one_reliable_full_snapshot() {
        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.add_plugins(turts_sim::SimPlugin { tick_hz: 20 });
        app.init_resource::<ReliableTraffic>();
        app.init_resource::<Stats>();
        app.finish();
        app.cleanup();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));
        app.update();
        let registry = app.world().resource::<ChannelRegistry>();
        let mut transport = Transport::default();
        transport.add_channel_send_from_registry::<ReliableChannel>(registry);
        transport.add_channel_send_from_registry::<StateChannel>(registry);
        let source = app
            .world_mut()
            .spawn((
                ClientOf,
                Link::default(),
                transport,
                RemoteId(PeerId::Local(42)),
                Linked,
                Connected,
                Session {
                    slot: turts_sim::SlotId(0),
                    player_id: 42,
                    last_seq: None,
                },
                MessageSender::<Snapshot>::default(),
                MessageSender::<WorldDelta>::default(),
            ))
            .id();
        let destination = app
            .world_mut()
            .spawn((
                Client,
                Link::default(),
                Transport::default(),
                RemoteId(PeerId::Server),
                Linked,
                Connected,
                MessageReceiver::<Snapshot>::default(),
                MessageReceiver::<WorldDelta>::default(),
            ))
            .id();
        let outcomes = turts_sim::outcome::MatchOutcomes {
            end_tick: Some(1),
            ..Default::default()
        };
        app.insert_resource(outcomes);
        send_deltas(app.world_mut());
        send_final_snapshots(app.world_mut());
        assert!(app.world().get::<FinalSnapshotSent>(source).is_some());
        app.update();
        let mut packets = Vec::new();
        while let Some(packet) = app.world_mut().get_mut::<Link>(source).unwrap().send.pop() {
            packets.push(packet);
        }
        assert!(
            !packets.is_empty(),
            "the final snapshot produced no wire packet"
        );
        for packet in packets {
            app.world_mut()
                .get_mut::<Link>(destination)
                .unwrap()
                .recv
                .push_raw(recv_payload_from_bytes(packet));
        }
        // Observe before Last clears the receivers.
        #[derive(Resource, Default)]
        struct Received(Vec<(Snapshot, ChannelKind)>, usize);
        app.init_resource::<Received>();
        app.add_systems(
            PreUpdate,
            (|mut snapshots: Query<&mut MessageReceiver<Snapshot>>,
              mut deltas: Query<&mut MessageReceiver<WorldDelta>>,
              mut received: ResMut<Received>| {
                for mut receiver in &mut snapshots {
                    for message in receiver.receive_with_tick() {
                        received.0.push((message.data, message.channel_kind));
                    }
                }
                for mut receiver in &mut deltas {
                    received.1 += receiver.receive().count();
                }
            })
            .after(MessageSystems::Receive),
        );
        app.update();
        let received = app.world().resource::<Received>();
        assert_eq!(received.0.len(), 1);
        assert_eq!(received.0[0].1, ChannelKind::of::<ReliableChannel>());
        assert_eq!(
            received.1, 0,
            "terminal unreliable delta can hide the final snapshot"
        );
        send_final_snapshots(app.world_mut());
        app.update();
        assert_eq!(app.world().resource::<Received>().0.len(), 1);
    }
}
