//! Control groups: assign, recall, and the group id of each sent order.

use super::*;

impl Core {
    pub(super) fn assign_group(&mut self, group: u8, additive: bool, world: &dyn WorldView) {
        let members: Vec<_> = self
            .selected
            .iter()
            .copied()
            .filter(|id| {
                world
                    .units()
                    .iter()
                    .any(|u| u.id == *id && u.owner_slot == world.own_slot())
            })
            .collect();
        if group as usize >= self.groups.len() {
            return;
        }
        if additive {
            for id in members {
                if !self.groups[group as usize].contains(&id) {
                    self.groups[group as usize].push(id);
                }
            }
        } else {
            self.groups[group as usize] = members;
        }
        self.groups[group as usize].sort_by_key(|id| id.0);
    }
    pub(super) fn recall_group(&mut self, group: u8, world: &dyn WorldView) {
        self.selected = self
            .groups
            .get(group as usize)
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| {
                world
                    .units()
                    .iter()
                    .any(|u| u.id == *id && u.owner_slot == world.own_slot())
            })
            .collect();
        self.selected.sort_by_key(|id| id.0);
    }
    pub(super) fn alloc_group(&mut self) -> u32 {
        let id = self.next_group.max(1);
        self.next_group = id.wrapping_add(1).max(1);
        id
    }
}

pub(super) fn engine_group_action(action: &str, prefix: &str) -> Option<u8> {
    match action.strip_prefix(prefix)?.as_bytes() {
        [digit @ b'1'..=b'9'] => Some(*digit - b'0'),
        _ => None,
    }
}
