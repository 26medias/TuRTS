//! Snapshot and delta sending.
//!
//! These systems run in `FixedLast`, after the simulation has collected the changes of the
//! tick in `FixedPostUpdate`. `collect_changes` is private to `turts_sim`, so a system cannot
//! name it for `.after(..)`. `FixedLast` runs after the whole `FixedPostUpdate` schedule.

use bevy::prelude::*;
use lightyear::prelude::*;
use turts_net_lightyear::{ReliableChannel, StateChannel};
use turts_net_protocol::messages::*;
use turts_sim::{ChangedThisTick, CommandResults, Tick as SimTick};

use crate::session::Session;
use crate::stats::Stats;

/// Add the outbound systems to the fixed loop.
pub fn install(app: &mut App) {
    app.add_systems(FixedLast, (send_command_results, send_deltas));
}

/// Tell a sender that its command failed validation. The tick that drained the commands made
/// these results, and the next drain clears them, so each result goes out once.
pub fn send_command_results(
    results: Res<CommandResults>,
    mut sessions: Query<(&Session, &mut MessageSender<CommandRejected>)>,
) {
    for result in &results.0 {
        let Err(reason) = result.outcome else {
            continue;
        };
        for (session, mut sender) in sessions.iter_mut() {
            if session.slot == result.slot {
                eprintln!(
                    "turts_server: command {} of player {} rejected: {}",
                    result.seq,
                    session.player_id,
                    reason.as_str()
                );
                sender.send::<ReliableChannel>(CommandRejected {
                    seq: result.seq,
                    reason: reason.as_str().to_string(),
                });
            }
        }
    }
}

/// Send `UnitDeltas` for this tick to every session, every tick, also when nothing changed.
/// `base_tick` is the tick before, so the client can detect a gap.
pub fn send_deltas(
    tick: Res<SimTick>,
    changed: Res<ChangedThisTick>,
    mut sessions: Query<&mut MessageSender<UnitDeltas>, With<Session>>,
    mut stats: ResMut<Stats>,
) {
    if sessions.is_empty() {
        return;
    }
    let deltas = UnitDeltas {
        tick: tick.0,
        base_tick: tick.0.saturating_sub(1),
        changes: changed
            .0
            .iter()
            .map(|(unit_id, pos)| UnitDelta {
                unit_id: UnitId(*unit_id),
                pos: Pos { x: pos.x, y: pos.y },
            })
            .collect(),
    };
    let payload = payload_bytes(&deltas);
    for mut sender in sessions.iter_mut() {
        sender.send::<StateChannel>(deltas.clone());
        stats.delta_payload_bytes_total += payload;
    }
}

/// The postcard size of a message. lightyear's message codec is postcard, so this is the
/// payload before the lightyear and netcode headers.
fn payload_bytes<T: serde::Serialize>(message: &T) -> u64 {
    postcard::experimental::serialized_size(message).expect("a wire message serializes to postcard")
        as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_bytes_matches_the_postcard_encoding() {
        let deltas = UnitDeltas {
            tick: 300,
            base_tick: 299,
            changes: (0..200)
                .map(|id| UnitDelta {
                    unit_id: UnitId(id * 100),
                    pos: Pos {
                        x: id as f32,
                        y: 2.5,
                    },
                })
                .collect(),
        };
        let encoded = postcard::to_vec::<_, 16384>(&deltas).expect("the message fits");
        assert_eq!(payload_bytes(&deltas), encoded.len() as u64);
        assert!(encoded.len() > 1000, "the sample is not trivial");
    }
}
