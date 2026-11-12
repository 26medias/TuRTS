//! Stub values and predicates of task `UI-RESULT` (plan 5.3; design 7.6). L0-CORE made this file;
//! `UI-RESULT` fills it through the store getters of `L0-STORE`. Until then: a null value, a
//! predicate that does not hold.

use super::SourceContext;
use turts_mod_api::{OutcomeKind, OutcomeReasonKind};

pub(in crate::widgets) fn value_outcome(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn value_outcome_reason(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn value_match_clock(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn predicate_match_over(_context: &SourceContext<'_>) -> bool {
    false
}

pub(in crate::widgets) fn predicate_outcome_is(
    _kind: OutcomeKind,
    _context: &SourceContext<'_>,
) -> bool {
    false
}

pub(in crate::widgets) fn predicate_outcome_reason_is(
    _kind: OutcomeReasonKind,
    _context: &SourceContext<'_>,
) -> bool {
    false
}
