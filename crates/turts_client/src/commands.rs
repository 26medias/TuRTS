//! Client-side staging for player commands and their shared sequence numbers.

use std::collections::VecDeque;

use bevy::prelude::Resource;
use turts_net_protocol::staged_v2::PlayerCommandKind;

/// Commands waiting to be sent, in the order they were submitted.
#[derive(Debug, Default, Resource)]
pub struct PendingCommands(pub VecDeque<PlayerCommandKind>);

/// The next sequence number for one fresh connection.
#[derive(Debug, Resource)]
pub struct CommandSequence {
    /// `None` means every usable sequence number has already been allocated.
    pub next: Option<u32>,
}

impl Default for CommandSequence {
    fn default() -> Self {
        Self { next: Some(1) }
    }
}

impl CommandSequence {
    /// Allocate one sequence number. `u32::MAX` is usable exactly once.
    pub fn allocate(&mut self) -> Result<u32, SequenceExhausted> {
        let current = self.next.ok_or(SequenceExhausted)?;
        self.next = current.checked_add(1);
        Ok(current)
    }
}

/// The connection has used its final sequence number and must be replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceExhausted;

/// Append a player command to the pending FIFO.
pub fn submit(pending: &mut PendingCommands, kind: PlayerCommandKind) {
    pending.0.push_back(kind);
}
