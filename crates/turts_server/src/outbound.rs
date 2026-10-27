//! Protocol 2 snapshots, world deltas, and fixed-tick command outcomes.

use crate::session::{ReliableTraffic, Session};
use crate::stats::Stats;
use bevy::prelude::*;
use lightyear::prelude::*;
use turts_net_lightyear::{ReliableChannel, StateChannel};
use turts_net_protocol::CommandRejectReason;
use turts_net_protocol::messages::*;
use turts_sim::{ChangedThisTick, CommandResults, Tick as SimTick};

pub fn install(app: &mut App) {
    app.add_systems(FixedLast, (send_command_results, send_deltas));
}

pub fn send_command_results(
    results: Res<CommandResults>,
    tick: Res<SimTick>,
    mut sessions: Query<(
        Entity,
        &Session,
        &mut MessageSender<CommandApplied>,
        &mut MessageSender<CommandRejected>,
    )>,
    mut traffic: ResMut<ReliableTraffic>,
) {
    for result in &results.0 {
        for (entity, session, mut applied, mut rejected) in &mut sessions {
            if session.slot != result.slot {
                continue;
            }
            match result.outcome {
                Ok(done) => {
                    applied.send::<ReliableChannel>(CommandApplied {
                        seq: result.seq,
                        tick: tick.0,
                        job_id: done.job_id.map(|id| turts_net_protocol::JobId(id.0)),
                    });
                    traffic.record(entity);
                }
                Err(reason) => {
                    rejected.send::<ReliableChannel>(CommandRejected {
                        seq: result.seq,
                        tick: tick.0,
                        reason: wire_reason(reason),
                    });
                    traffic.record(entity);
                }
            }
        }
    }
}

fn wire_reason(reason: turts_sim::RejectReason) -> CommandRejectReason {
    use turts_sim::RejectReason as R;
    match reason {
        R::EmptyCommand => CommandRejectReason::EmptyCommand,
        R::TooManyUnits => CommandRejectReason::TooManyUnits,
        R::DuplicateUnit => CommandRejectReason::DuplicateUnit,
        R::UnknownUnit => CommandRejectReason::UnknownUnit,
        R::NotOwner => CommandRejectReason::NotOwner,
        R::NotMobile => CommandRejectReason::NotMobile,
        R::TargetNotFinite => CommandRejectReason::TargetNotFinite,
        R::TargetOutsideMap => CommandRejectReason::TargetOutsideMap,
        R::MissingCapability => CommandRejectReason::MissingCapability,
        R::UnknownNode => CommandRejectReason::UnknownNode,
        R::IncompatibleItem => CommandRejectReason::IncompatibleItem,
        R::NodeDepleted => CommandRejectReason::NodeDepleted,
        R::UnknownRecipe => CommandRejectReason::UnknownRecipe,
        R::RecipeNotEnabled => CommandRejectReason::RecipeNotEnabled,
        R::QueueFull => CommandRejectReason::QueueFull,
        R::InsufficientBalance => CommandRejectReason::InsufficientBalance,
        R::InsufficientItems | R::UnknownInventory | R::InvalidAmount => {
            CommandRejectReason::InsufficientItems
        }
        R::UnknownJob => CommandRejectReason::UnknownJob,
        R::RefundDestinationFull => CommandRejectReason::RefundDestinationFull,
        R::RefundDestinationOverflow => CommandRejectReason::RefundDestinationOverflow,
        R::ArithmeticOverflow => CommandRejectReason::ArithmeticOverflow,
        R::IdExhausted => CommandRejectReason::IdExhausted,
        R::BadSequence => CommandRejectReason::BadSequence,
        R::UnknownBalance => CommandRejectReason::InsufficientBalance,
    }
}

pub fn send_deltas(
    tick: Res<SimTick>,
    changed: Res<ChangedThisTick>,
    mut sessions: Query<&mut MessageSender<WorldDelta>, With<Session>>,
    mut stats: ResMut<Stats>,
) {
    if sessions.is_empty() {
        return;
    }
    let delta = WorldDelta {
        tick: tick.0,
        base_tick: tick.0.saturating_sub(1),
        spawned: Vec::new(),
        removed: Vec::new(),
        positions: changed
            .0
            .iter()
            .map(|(id, pos)| UnitDelta {
                unit_id: UnitId(*id),
                pos: Pos { x: pos.x, y: pos.y },
            })
            .collect(),
        nodes: Vec::new(),
        economy: EconomyChanges {
            balances: vec![],
            inventories: vec![],
            orders: vec![],
            queues: vec![],
        },
    };
    let payload = payload_bytes(&delta);
    for mut sender in &mut sessions {
        sender.send::<StateChannel>(delta.clone());
        stats.delta_payload_bytes_total += payload;
    }
}
fn payload_bytes<T: serde::Serialize>(message: &T) -> u64 {
    postcard::experimental::serialized_size(message).expect("a wire message serializes to postcard")
        as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_internal_reasons_never_leak_as_strings() {
        assert_eq!(
            wire_reason(turts_sim::RejectReason::UnknownInventory),
            CommandRejectReason::InsufficientItems
        );
        assert_eq!(
            wire_reason(turts_sim::RejectReason::UnknownBalance),
            CommandRejectReason::InsufficientBalance
        );
    }
}
