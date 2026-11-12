//! Stub value of task `UI2-CONN` (plan 5.3; design 7.9, row H7). L0-CORE made this file;
//! `UI2-CONN` fills it. Until then: a null value.

use super::SourceContext;

pub(in crate::widgets) fn value_connection(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}
