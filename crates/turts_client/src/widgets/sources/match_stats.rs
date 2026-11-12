//! Stub value of task `UI2-RESULT` (plan 5.3; design 6.7.9, row H7). L0-CORE made this file;
//! `UI2-RESULT` fills it. Until then: a null value.

use super::SourceContext;
use turts_mod_api::StatField;

pub(in crate::widgets) fn value_match_stats(
    _field: &StatField,
    _context: SourceContext<'_>,
) -> serde_json::Value {
    serde_json::Value::Null
}
