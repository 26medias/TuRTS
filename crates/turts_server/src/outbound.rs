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
    app.add_systems(FixedLast, (send_command_results, send_deltas).chain());
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
