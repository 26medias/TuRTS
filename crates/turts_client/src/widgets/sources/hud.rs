//! Stub sources of task `UI-HUD` (plan 5.3; design 7.3 to 7.5). L0-CORE made this file; `UI-HUD`
//! fills it and reads the store getters of `L0-STORE`.
//!
//! The list sources fill the item fields `Role`, `State`, `Paid`, `Refund` and `Badge` with
//! `ItemData::set` (the key names of these fields are in `item_field_key` in `sources.rs`).
//! Until `UI-HUD` builds a function, it gives what an engine of API 0.2.3 gave: a list with no
//! rows, a null value, a predicate that does not hold.

use super::{SourceContext, SourceData};
use turts_mod_api::{ContentRef, ItemField, SupplySide};

fn no_rows() -> SourceData {
    SourceData {
        scalar: None,
        items: Vec::new(),
    }
}

pub(in crate::widgets) fn source_construction_queue(_context: SourceContext<'_>) -> SourceData {
    no_rows()
}

pub(in crate::widgets) fn source_sites(_context: SourceContext<'_>) -> SourceData {
    no_rows()
}

pub(in crate::widgets) fn source_notices(_context: SourceContext<'_>) -> SourceData {
    no_rows()
}

pub(in crate::widgets) fn source_selection_types(_context: SourceContext<'_>) -> SourceData {
    no_rows()
}

pub(in crate::widgets) fn value_supply(
    _resource: &ContentRef,
    _side: SupplySide,
    _context: SourceContext<'_>,
) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn predicate_supply_short(
    _resource: &ContentRef,
    _context: &SourceContext<'_>,
) -> bool {
    false
}

pub(in crate::widgets) fn predicate_item_has(
    _field: ItemField,
    _context: &SourceContext<'_>,
) -> bool {
    false
}
