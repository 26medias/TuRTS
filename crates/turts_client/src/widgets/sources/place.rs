//! Stub predicate of task `UI-PLACE` (plan 5.3; design 7.2). L0-CORE made this file; `UI-PLACE`
//! fills it from the placement mode of the interaction core. Until then: the predicate does not
//! hold.

use super::SourceContext;

/// What the sources read from the placement mode of the core (`CoreFeed::placement`). Empty until
/// `UI-PLACE`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {}

/// The placement part of the feed, from the core (`core.placement_state()`).
pub(in crate::widgets) fn feed(_core: Option<&turts_interaction::Core>) -> Feed {
    Feed::default()
}

pub(in crate::widgets) fn predicate_placing(_context: &SourceContext<'_>) -> bool {
    false
}
