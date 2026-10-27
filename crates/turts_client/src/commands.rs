//! Client-side player command FIFO and one checked sequence per fresh connection.

use crate::ingest::{CommandResults, ReceiptWitnesses};
use crate::join::Phase;
use crate::scenario::ScenarioState;
use bevy::prelude::*;
use lightyear::prelude::*;
use std::collections::VecDeque;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::{PlayerCommand, PlayerCommandKind};

#[derive(Debug, Clone, PartialEq)]
pub enum PendingCommand {
    Submit {
        kind: PlayerCommandKind,
        trace_step: Option<usize>,
    },
    Repeat {
        envelope: PlayerCommand,
        trace_step: usize,
    },
}

#[derive(Debug, Default, Resource)]
pub struct PendingCommands(pub VecDeque<PendingCommand>);

#[derive(Debug, Resource)]
pub struct CommandSequence {
    pub next: Option<u32>,
}
impl Default for CommandSequence {
    fn default() -> Self {
        Self { next: Some(1) }
    }
}
impl CommandSequence {
    pub fn allocate(&mut self) -> Result<u32, SequenceExhausted> {
        let current = self.next.ok_or(SequenceExhausted)?;
        self.next = current.checked_add(1);
        Ok(current)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceExhausted;
#[derive(Debug, Default, Resource)]
pub struct CommandSendStatus {
    pub reconnect_required: bool,
}
pub fn submit(pending: &mut PendingCommands, kind: PlayerCommandKind) {
    pending.0.push_back(PendingCommand::Submit {
        kind,
        trace_step: None,
    });
}

pub(crate) fn submit_trace(
    pending: &mut PendingCommands,
    kind: PlayerCommandKind,
    trace_step: usize,
) {
    pending.0.push_back(PendingCommand::Submit {
        kind,
        trace_step: Some(trace_step),
    });
}

pub(crate) fn repeat_trace(
    pending: &mut PendingCommands,
    envelope: PlayerCommand,
    trace_step: usize,
) {
    pending.0.push_back(PendingCommand::Repeat {
        envelope,
        trace_step,
    });
}

/// Owned mobile ids, ascending, for convenience adapters that expand an implicit all-units move
/// before it enters the shared FIFO. Explicit command bodies must remain literal.
pub fn owned_mobile_ids(state: &crate::ingest::ReplicaState) -> Vec<turts_net_protocol::UnitId> {
    let (Some(welcome), Some(replica), Some(content)) = (
        state.welcome,
        state.replica.as_ref(),
        state.content.as_ref(),
    ) else {
        return Vec::new();
    };
    let mut ids: Vec<_> = replica
        .units
        .iter()
        .filter_map(|(&id, unit)| {
            if unit.owner_slot != welcome.slot {
                return None;
            }
            let content_id = turts_mod_api::ContentId::parse(&unit.content_id).ok()?;
            let definition = content.catalog.units.get(&content_id)?;
            definition.movement.as_ref().map(|_| UnitId(id))
        })
        .collect();
    ids.sort_by_key(|id| id.0);
    ids
}

/// Send the single shared FIFO only after a fresh session has joined. Ordinary adapters must
/// provide their intended literal bodies before enqueueing; trace steps use the same path.
#[allow(clippy::too_many_arguments)] // Bevy system resources stay explicit at this boundary.
pub(crate) fn send_commands_traced(
    mut pending: ResMut<PendingCommands>,
    mut sequence: ResMut<CommandSequence>,
    mut status: ResMut<CommandSendStatus>,
    phase: Res<Phase>,
    mut senders: Query<(Entity, &mut MessageSender<PlayerCommand>), With<Connected>>,
    mut witnesses: ResMut<ReceiptWitnesses>,
    mut scenario: ResMut<ScenarioState>,
    results: Res<CommandResults>,
) {
    if !matches!(*phase, Phase::Joined { .. }) || senders.is_empty() || status.reconnect_required {
        return;
    }
    for (entity, mut sender) in &mut senders {
        let epoch = witnesses.epoch_for(entity.to_bits());
        while let Some(command) = pending.0.pop_front() {
            let (envelope, trace_step) = match command {
                PendingCommand::Submit { kind, trace_step } => {
                    let seq = match sequence.allocate() {
                        Ok(seq) => seq,
                        Err(_) => {
                            status.reconnect_required = true;
                            pending.0.clear();
                            scenario.error = Some("command sequence exhausted".into());
                            break;
                        }
                    };
                    (PlayerCommand { seq, kind }, trace_step)
                }
                PendingCommand::Repeat {
                    envelope,
                    trace_step,
                } => (envelope, Some(trace_step)),
            };
            sender.send::<ReliableChannel>(envelope.clone());
            witnesses.record_message(epoch, true);
            if let Some(step) = trace_step
                && let Err(error) = scenario.record_sent(step, envelope, results.0.len())
            {
                scenario.error = Some(error);
            }
        }
    }
}
