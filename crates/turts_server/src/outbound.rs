//! Protocol 2 snapshots, world deltas, and fixed-tick command outcomes.

use crate::export;
use crate::session::{ReliableTraffic, Session};
use crate::stats::Stats;
use bevy::prelude::*;
use lightyear::prelude::*;
use turts_net_lightyear::{ReliableChannel, StateChannel};
use turts_net_protocol::CommandRejectReason;
use turts_net_protocol::messages::*;
use turts_sim::{CommandResults, Tick as SimTick, committed_tick};

pub fn install(app: &mut App) {
    app.add_systems(FixedLast, (send_command_results, send_deltas).chain());
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
        R::NotRemovable => CommandRejectReason::NotRemovable,
        R::NotToggleable => CommandRejectReason::NotToggleable,
        R::PrerequisitesMissing => CommandRejectReason::PrerequisitesMissing,
        R::AlreadyResearched => CommandRejectReason::AlreadyResearched,
        R::ResearchInProgress => CommandRejectReason::ResearchInProgress,
        R::IntegrityViolation => CommandRejectReason::Internal,
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
        recipients
            .into_iter()
            .map(|(entity, slot)| {
                let delta = export::delta_for(batch, slot);
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

    #[test]
    fn wire_reason_covers_every_sim_reason() {
        use turts_sim::RejectReason as R;

        let cases = [
            (R::EmptyCommand, CommandRejectReason::EmptyCommand),
            (R::TooManyUnits, CommandRejectReason::TooManyUnits),
            (R::DuplicateUnit, CommandRejectReason::DuplicateUnit),
            (R::UnknownUnit, CommandRejectReason::UnknownUnit),
            (R::NotOwner, CommandRejectReason::NotOwner),
            (R::NotMobile, CommandRejectReason::NotMobile),
            (R::TargetNotFinite, CommandRejectReason::TargetNotFinite),
            (R::TargetOutsideMap, CommandRejectReason::TargetOutsideMap),
            (R::MissingCapability, CommandRejectReason::MissingCapability),
            (R::UnknownNode, CommandRejectReason::UnknownNode),
            (R::IncompatibleItem, CommandRejectReason::IncompatibleItem),
            (R::NodeDepleted, CommandRejectReason::NodeDepleted),
            (R::UnknownRecipe, CommandRejectReason::UnknownRecipe),
            (R::RecipeNotEnabled, CommandRejectReason::RecipeNotEnabled),
            (R::QueueFull, CommandRejectReason::QueueFull),
            (
                R::InsufficientBalance,
                CommandRejectReason::InsufficientBalance,
            ),
            (R::InsufficientItems, CommandRejectReason::InsufficientItems),
            (R::UnknownJob, CommandRejectReason::UnknownJob),
            (
                R::RefundDestinationFull,
                CommandRejectReason::RefundDestinationFull,
            ),
            (
                R::RefundDestinationOverflow,
                CommandRejectReason::RefundDestinationOverflow,
            ),
            (
                R::ArithmeticOverflow,
                CommandRejectReason::ArithmeticOverflow,
            ),
            (R::IdExhausted, CommandRejectReason::IdExhausted),
            (R::BadSequence, CommandRejectReason::BadSequence),
            (R::UnknownInventory, CommandRejectReason::InsufficientItems),
            (R::UnknownBalance, CommandRejectReason::InsufficientBalance),
            (R::InvalidAmount, CommandRejectReason::InsufficientItems),
            (R::NotRemovable, CommandRejectReason::NotRemovable),
            (R::NotToggleable, CommandRejectReason::NotToggleable),
            (
                R::PrerequisitesMissing,
                CommandRejectReason::PrerequisitesMissing,
            ),
            (R::AlreadyResearched, CommandRejectReason::AlreadyResearched),
            (
                R::ResearchInProgress,
                CommandRejectReason::ResearchInProgress,
            ),
            (R::IntegrityViolation, CommandRejectReason::Internal),
        ];

        for (reason, expected) in cases {
            assert_eq!(wire_reason(reason), expected, "{}", reason.as_str());
        }
    }

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
