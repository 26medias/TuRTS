//! Player slots.

use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};

/// Default player-slot count in a match (design 6).
pub(crate) const SLOT_COUNT: u8 = 2;

/// Index of a player slot. A claimed slot belongs to its player for the whole match (D25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotId(pub u8);

/// Why a claim failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    NoOpenSlot,
    AlreadyClaimed,
}

/// The slot table. A slot goes from open to claimed and never back (D25, spec 6.8).
/// A slot record is not a gameplay entity: claiming creates no entity.
#[derive(Resource, Debug)]
pub struct Slots {
    owners: Vec<Option<u64>>,
}

impl Slots {
    pub fn new(count: u8) -> Self {
        Self {
            owners: vec![None; count as usize],
        }
    }

    /// Number of configured player slots.
    pub fn count(&self) -> u8 {
        self.owners.len() as u8
    }

    /// Whether `slot` names a configured slot, whether or not it has an owner.
    pub fn contains(&self, slot: SlotId) -> bool {
        (slot.0 as usize) < self.owners.len()
    }

    /// Give the first open slot to `player_id`. A player that already holds a slot gets
    /// `AlreadyClaimed`, even when no slot is open.
    pub fn claim(&mut self, player_id: u64) -> Result<SlotId, ClaimError> {
        if self.slot_of(player_id).is_some() {
            return Err(ClaimError::AlreadyClaimed);
        }
        let index = self
            .owners
            .iter()
            .position(Option::is_none)
            .ok_or(ClaimError::NoOpenSlot)?;
        self.owners[index] = Some(player_id);
        Ok(SlotId(index as u8))
    }

    pub fn owner_of(&self, slot: SlotId) -> Option<u64> {
        self.owners.get(slot.0 as usize).copied().flatten()
    }

    pub fn owners(&self) -> Vec<Option<u64>> {
        self.owners.clone()
    }

    pub fn slot_of(&self, player_id: u64) -> Option<SlotId> {
        self.owners
            .iter()
            .position(|owner| *owner == Some(player_id))
            .map(|index| SlotId(index as u8))
    }
}
