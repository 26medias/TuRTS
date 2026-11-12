//! Stub value and predicate of task `UI-HINT` (plan 5.3; design 7.8). L0-CORE made this file;
//! `UI-HINT` fills it. Until then: a null value, a predicate that does not hold.

use super::SourceContext;

/// What the sources read from the hint state of the core (`CoreFeed::hint`). Empty until
/// `UI-HINT`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {}

/// The hint part of the feed, from the core (`core.hint_state()`).
pub(in crate::widgets) fn feed(_core: Option<&turts_interaction::Core>) -> Feed {
    Feed::default()
}

pub(in crate::widgets) fn value_hint(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn predicate_hint_shown(_context: &SourceContext<'_>) -> bool {
    false
}
