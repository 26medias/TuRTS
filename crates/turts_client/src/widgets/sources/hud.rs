//! Construction progress reads committed replica records. Other pending HUD sources stay empty.

use super::{ItemData, SourceContext, SourceData, WidgetAction};
use turts_mod_api::{ContentId, ContentRef, ItemField, SupplySide};

fn no_rows() -> SourceData {
    SourceData {
        scalar: None,
        items: Vec::new(),
    }
}

pub(in crate::widgets) fn source_construction_queue(_context: SourceContext<'_>) -> SourceData {
    no_rows()
}

pub(in crate::widgets) fn source_sites(context: SourceContext<'_>) -> SourceData {
    let items = context
        .replica
        .sites()
        .filter_map(|id| {
            let unit = context.replica.units.get(&id)?;
            if unit.owner_slot != context.owner_slot {
                return None;
            }
            let record = context.replica.site_progress(id)?;
            let done = context.replica.site_value(id)?.min(record.total);
            let name = ContentId::parse(&unit.content_id)
                .ok()
                .and_then(|id| context.content.catalog.units.get(&id))
                .map_or(unit.content_id.as_str(), |unit| unit.name.as_str());
            let mut row = ItemData::default();
            row.set(ItemField::Name, name);
            row.set(ItemField::Action, WidgetAction::Select { unit: id });
            row.set(ItemField::Enabled, true);
            row.set(
                ItemField::Progress,
                done as f64 / record.total.max(1) as f64,
            );
            row.set(
                ItemField::RemainingSeconds,
                if record.per_tick == 0 {
                    None
                } else {
                    Some(
                        (record.total - done) as f64
                            / record.per_tick as f64
                            / f64::from(context.replica.tick_hz.max(1)),
                    )
                },
            );
            Some(row)
        })
        .collect();
    SourceData {
        scalar: None,
        items,
    }
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
