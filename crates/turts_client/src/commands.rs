//! Client-side player command FIFO and one checked sequence per fresh connection.

use crate::ingest::{ReceiptWitnesses, ReplicaState};
use crate::join::Phase;
use bevy::prelude::*;
use lightyear::prelude::*;
use std::collections::VecDeque;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::UnitId;
use turts_net_protocol::{PlayerCommand, PlayerCommandKind};

#[derive(Debug, Default, Resource)]
pub struct PendingCommands(pub VecDeque<PlayerCommandKind>);

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
    pending.0.push_back(kind);
}

/// Owned mobile ids, ascending, for convenience adapters that expand an implicit all-units move
/// before it enters the shared FIFO. Explicit command bodies must remain literal.
pub fn owned_mobile_ids(state: &ReplicaState) -> Vec<UnitId> {
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

/// Send queued commands in FIFO order only after a fresh session has joined.
pub fn send_commands(
    mut pending: ResMut<PendingCommands>,
    mut sequence: ResMut<CommandSequence>,
    mut status: ResMut<CommandSendStatus>,
    phase: Res<Phase>,
    mut senders: Query<(Entity, &mut MessageSender<PlayerCommand>), With<Connected>>,
    mut witnesses: ResMut<ReceiptWitnesses>,
) {
    if !matches!(*phase, Phase::Joined { .. }) || senders.is_empty() || status.reconnect_required {
        return;
    }
    for (entity, mut sender) in &mut senders {
        let epoch = witnesses.epoch_for(entity.to_bits());
        while let Some(kind) = pending.0.pop_front() {
            match sequence.allocate() {
                Ok(seq) => {
                    sender.send::<ReliableChannel>(PlayerCommand { seq, kind });
                    witnesses.record_message(epoch, true);
                }
                Err(_) => {
                    status.reconnect_required = true;
                    pending.0.clear();
                    break;
                }
            }
        }
    }
}
