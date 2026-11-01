//! Task12 guide state test seam.

use bevy::prelude::Resource;

use crate::join::Phase;

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GuideOpen {
    pub is_open: bool,
    _first_join_seen: bool,
}

/// Open the guide once, on the first joined phase.
pub fn observe_phase(phase: &Phase, guide: &mut GuideOpen) {
    if !guide._first_join_seen && matches!(phase, Phase::Joined { .. }) {
        guide._first_join_seen = true;
        guide.is_open = true;
    }
}

/// Apply the `toggle_guide` shell action or guide close-button action.
///
pub fn toggle_guide(guide: &mut GuideOpen) {
    guide.is_open = !guide.is_open;
}

pub fn close_guide(guide: &mut GuideOpen) {
    guide.is_open = false;
}
