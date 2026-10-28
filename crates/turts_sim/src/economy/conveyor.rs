//! Compact conveyor state. Transport behavior is added in Task 7.

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;
use turts_mod_api::ContentId;

use super::gather::RateCarry;

/// Dense item index. Zero marks an empty belt slot.
#[allow(dead_code)] // used from Task 7
pub(crate) type ItemIx = u16;

/// One ordered conveyor line and its compact item slots.
#[allow(dead_code)] // used from Task 7
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BeltLine {
    pub(crate) def: ContentId,
    pub(crate) owner: u8,
    pub(crate) pieces: Vec<u32>,
    pub(crate) slots: Vec<ItemIx>,
    pub(crate) slots_per_piece: u8,
    pub(crate) slots_per_second: u64,
    pub(crate) carry: RateCarry,
}

/// Conveyor lines, indexed by their first piece and by each contained piece.
#[allow(dead_code)] // used from Task 7
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Conveyors {
    pub(crate) lines: BTreeMap<u32, BeltLine>,
    pub(crate) piece_line: BTreeMap<u32, (u32, u16)>,
}
