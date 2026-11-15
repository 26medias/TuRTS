//! Optional host receipts for ordinary grouped command admission.
//!
//! These records identify work. They never authorize or apply a gameplay change.
//! Hosts install tracking only when they need exact per-member work identities.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::prelude::Resource;

use crate::{JobId, SlotId};

/// A command member, independent of network connections and bot implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandMember {
    pub slot: u8,
    pub sequence: u32,
    pub unit: u32,
}

/// Exact job identities returned by successful ordinary order application.
/// The host must bound watched requests and drain them after the completed tick.
#[derive(Resource, Default)]
pub struct CommandWorkReceipts {
    watched: BTreeSet<(u8, u32)>,
    jobs: BTreeMap<CommandMember, JobId>,
}

impl CommandWorkReceipts {
    /// Register an admitted envelope. This grants no command permission.
    pub fn watch(&mut self, slot: SlotId, sequence: u32) {
        self.watched.insert((slot.0, sequence));
    }

    pub(crate) fn record(&mut self, slot: SlotId, sequence: u32, unit: u32, job: JobId) {
        if self.watched.contains(&(slot.0, sequence)) {
            self.jobs.insert(
                CommandMember {
                    slot: slot.0,
                    sequence,
                    unit,
                },
                job,
            );
        }
    }

    /// Consume only this envelope's job hints. Per-member acceptance/refusal still
    /// comes from the ordinary CommandResult, including envelope-level failures.
    pub fn take(&mut self, slot: SlotId, sequence: u32) -> BTreeMap<u32, JobId> {
        self.watched.remove(&(slot.0, sequence));
        let keys = self
            .jobs
            .keys()
            .filter(|key| key.slot == slot.0 && key.sequence == sequence)
            .copied()
            .collect::<Vec<_>>();
        keys.into_iter()
            .filter_map(|key| self.jobs.remove(&key).map(|job| (key.unit, job)))
            .collect()
    }
}
