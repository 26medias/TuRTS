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

#[cfg(test)]
mod item_table_tests {
    use super::{ItemIx, ItemTable};
    use turts_mod_api::ContentId;

    #[test]
    fn item_table_reserves_zero_and_maps_sorted_content_ids() {
        let alpha = ContentId::parse("probe:alpha").unwrap();
        let zeta = ContentId::parse("probe:zeta").unwrap();
        let table = ItemTable(vec![alpha.clone(), zeta.clone()]);

        assert_eq!(table.content(0), None);
        assert_eq!(table.content(1), Some(&alpha));
        assert_eq!(table.content(2), Some(&zeta));
        assert_eq!(table.content(3), None);
        assert_eq!(table.item_ix(&alpha), Some(1));
        assert_eq!(table.item_ix(&zeta), Some(2));
    }

    #[test]
    fn item_table_reverse_mapping_checks_u16_boundary() {
        assert_eq!(
            ItemTable::ix_for_position(usize::from(ItemIx::MAX) - 1),
            Some(ItemIx::MAX)
        );
        assert_eq!(ItemTable::ix_for_position(usize::from(ItemIx::MAX)), None);
    }
}

/// Sorted item ids used to translate compact belt slots to content ids.
/// Item index zero is reserved for an empty slot, so the first item is at vector index zero.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ItemTable(pub(crate) Vec<ContentId>);

impl ItemTable {
    #[allow(dead_code)] // used from Task 3
    pub(crate) fn content(&self, item: ItemIx) -> Option<&ContentId> {
        (item != 0)
            .then(|| self.0.get(usize::from(item) - 1))
            .flatten()
    }

    #[allow(dead_code)] // used from Task 4
    pub(crate) fn item_ix(&self, item: &ContentId) -> Option<ItemIx> {
        let position = self.0.binary_search(item).ok()?;
        Self::ix_for_position(position)
    }

    fn ix_for_position(position: usize) -> Option<ItemIx> {
        ItemIx::try_from(position.checked_add(1)?).ok()
    }
}
