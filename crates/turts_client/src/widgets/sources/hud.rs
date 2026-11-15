//! Construction progress and owner supply read committed replica records.
//! Other pending HUD sources stay empty.

use super::{ItemData, SourceContext, SourceData, WidgetAction};
use turts_mod_api::{ContentId, ContentRef, ItemField, SupplySide};

fn no_rows() -> SourceData {
    SourceData {
        scalar: None,
        items: Vec::new(),
    }
}

pub(in crate::widgets) fn source_construction_queue(context: SourceContext<'_>) -> SourceData {
    let Some(id) = context.selection.first().copied().filter(|id| {
        context
            .replica
            .units
            .get(id)
            .is_some_and(|unit| unit.owner_slot == context.owner_slot)
    }) else {
        return no_rows();
    };
    let items = context
        .replica
        .construction_queue(id)
        .iter()
        .filter_map(|order| {
            let recipe = ContentId::parse(&order.content)
                .ok()
                .and_then(|id| context.content.catalog.production.get(&id))?;
            let mut row = ItemData::default();
            row.set(ItemField::Name, &recipe.name);
            row.set(
                ItemField::State,
                context
                    .interface
                    .and_then(|interface| interface.states.get("held"))
                    .map_or("Held", String::as_str),
            );
            set_payment_fields(
                &mut row,
                &[],
                Some(turts_mod_api::RefundRatio::ZERO),
                context,
            );
            Some(row)
        })
        .collect();
    SourceData {
        scalar: None,
        items,
    }
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
            set_work_state(&mut row, record, context);
            let refund = context
                .replica
                .site_production(id)
                .and_then(|id| ContentId::parse(id).ok())
                .and_then(|id| context.content.catalog.production.get(&id))
                .map(|recipe| recipe.cancel_refunds.running);
            set_payment_fields(&mut row, &record.charged, refund, context);
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
    resource: &ContentRef,
    side: SupplySide,
    context: SourceContext<'_>,
) -> serde_json::Value {
    serde_json::json!(context.replica.supply(resource, side))
}

pub(in crate::widgets) fn predicate_supply_short(
    resource: &ContentRef,
    context: &SourceContext<'_>,
) -> bool {
    context.replica.supply_short(resource)
}

pub(in crate::widgets) fn predicate_item_has(
    _field: ItemField,
    _context: &SourceContext<'_>,
) -> bool {
    false
}

/// The display hold is separate from the actual advancing rate.
pub(in crate::widgets) fn set_work_state(
    row: &mut ItemData,
    record: &turts_net_protocol::RateRecord,
    context: SourceContext<'_>,
) {
    use turts_net_protocol::RecordState;
    let (key, fallback, resource) = match &record.state {
        RecordState::WaitingForFunds(resource) => {
            ("waiting_for_funds", "Waiting for funds", Some(resource))
        }
        RecordState::Slowed => ("slowed", "Slowed", None),
        RecordState::OutputBlocked => ("output_blocked", "Exit blocked", None),
        RecordState::Held => ("held", "Held", None),
        RecordState::Running if record.held => ("held", "Held", None),
        RecordState::Running => return,
    };
    let label = context
        .interface
        .and_then(|interface| interface.states.get(key))
        .map_or(fallback, String::as_str);
    let label = match resource {
        Some(resource) => format!(
            "{label}: {}",
            super::resource_name(resource, context.content)
        ),
        None => label.to_owned(),
    };
    row.set(ItemField::State, label);
}

/// Amounts come from committed charged rows. The display never predicts a payment.
pub(in crate::widgets) fn set_payment_fields(
    row: &mut ItemData,
    charged: &[turts_mod_api::Cost],
    refund: Option<turts_mod_api::RefundRatio>,
    context: SourceContext<'_>,
) {
    fn label(
        charged: &[turts_mod_api::Cost],
        ratio: turts_mod_api::RefundRatio,
        context: SourceContext<'_>,
    ) -> String {
        let costs = charged
            .iter()
            .filter_map(|cost| {
                Some(turts_mod_api::ResolvedCost {
                    resource: ContentId::parse(&cost.resource).ok()?,
                    amount: ((u128::from(cost.amount) * u128::from(ratio.numerator))
                        / u128::from(ratio.denominator)) as u64,
                    source: cost.source,
                })
            })
            .collect::<Vec<_>>();
        if costs.is_empty() {
            "0".into()
        } else {
            super::cost_label(&costs, context.content)
        }
    }
    row.set(
        ItemField::Paid,
        label(charged, turts_mod_api::RefundRatio::ONE, context),
    );
    if let Some(ratio) = refund {
        row.set(ItemField::Refund, label(charged, ratio, context));
    }
}
