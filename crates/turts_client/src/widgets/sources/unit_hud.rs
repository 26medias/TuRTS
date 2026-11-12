//! Stub primary fields and predicate of task `UI2-HUD` (plan 5.3; design 7.5, rows H8 and H10).
//! L0-CORE made this file; `UI2-HUD` fills it. It also fills the item fields `BuildTimeNow`,
//! `SupplyChange` and `CausesShortage` in its list sources (the key names are in
//! `item_field_key` in `sources.rs`). Until then: a null value, a predicate that does not hold.

use super::SourceContext;
use turts_mod_api::PrimaryField;

pub(in crate::widgets) fn primary_activity(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn primary_cargo(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn primary_armour(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn predicate_primary_has_value(
    _field: PrimaryField,
    _context: &SourceContext<'_>,
) -> bool {
    false
}
