//! Stub predicate of task `UI-PLACE` (plan 5.3; design 7.2). L0-CORE made this file; `UI-PLACE`
//! fills it from the placement mode of the interaction core. Until then: the predicate does not
//! hold.

use super::SourceContext;

pub(in crate::widgets) fn predicate_placing(_context: &SourceContext<'_>) -> bool {
    false
}
