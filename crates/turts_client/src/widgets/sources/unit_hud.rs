//! Selected-unit information. Cargo remains unavailable.

use super::SourceContext;
use turts_mod_api::PrimaryField;

fn selected_repair<'a>(context: SourceContext<'a>) -> Option<&'a turts_net_protocol::RepairRecord> {
    let id = context.selection.first()?;
    let unit = context.replica.units.get(id)?;
    if unit.owner_slot != context.owner_slot {
        return None;
    }
    context.replica.repair(*id)
}

pub(in crate::widgets) fn primary_repair(
    field: PrimaryField,
    context: SourceContext<'_>,
) -> serde_json::Value {
    use turts_net_protocol::RepairState;
    let Some(record) = selected_repair(context) else {
        return serde_json::Value::Null;
    };
    match field {
        PrimaryField::RepairState => serde_json::json!(match &record.state {
            RepairState::Approaching => "approaching",
            RepairState::Working => "working",
            RepairState::WaitingForFunds(_) => "waiting_for_funds",
        }),
        PrimaryField::RepairTargetName => context
            .replica
            .units
            .get(&record.target.0)
            .and_then(|unit| turts_mod_api::ContentId::parse(&unit.content_id).ok())
            .and_then(|id| context.content.catalog.units.get(&id))
            .map_or(serde_json::Value::Null, |unit| serde_json::json!(unit.name)),
        PrimaryField::RepairResourceName => match &record.state {
            RepairState::WaitingForFunds(resource) => turts_mod_api::ContentId::parse(resource)
                .ok()
                .and_then(|id| context.content.catalog.resources.get(&id))
                .map_or(serde_json::Value::Null, |resource| {
                    serde_json::json!(resource.name)
                }),
            _ => serde_json::Value::Null,
        },
        _ => serde_json::Value::Null,
    }
}

pub(in crate::widgets) fn predicate_repair_state(
    kind: turts_mod_api::RepairStateKind,
    context: &SourceContext<'_>,
) -> bool {
    use turts_mod_api::RepairStateKind;
    use turts_net_protocol::RepairState;
    selected_repair(*context).is_some_and(|record| {
        matches!(
            (&record.state, kind),
            (RepairState::Approaching, RepairStateKind::Approaching)
                | (RepairState::Working, RepairStateKind::Working)
                | (
                    RepairState::WaitingForFunds(_),
                    RepairStateKind::WaitingForFunds
                )
        )
    })
}

/// The replicated activity of the first selected unit, as the text that a `Text` widget shows. The server sends the
/// activity word and no reason, so the text gives no reason. `Null` with no selection, for a
/// unit of another slot, and for a unit with no order record.
/// The words are fixed here: a mod cannot change or translate them, because a mapping from a
/// stable id to a label needs a schema change (UX note of 2026-11-23, accepted cost).
pub(in crate::widgets) fn primary_activity(context: SourceContext<'_>) -> serde_json::Value {
    use turts_net_protocol::Activity;
    let Some(id) = context.selection.first() else {
        return serde_json::Value::Null;
    };
    if context
        .replica
        .units
        .get(id)
        .is_none_or(|unit| unit.owner_slot != context.owner_slot)
    {
        return serde_json::Value::Null;
    }
    let Some(record) = context.replica.unit_orders.get(id) else {
        return serde_json::Value::Null;
    };
    serde_json::json!(match record.activity {
        Activity::Idle => "Idle",
        Activity::Pending => "Order received",
        Activity::Moving => "Moving",
        Activity::Attacking => "Attacking",
        Activity::Holding => "Holding position",
        Activity::Gathering => "Gathering",
        // The plain word (PM decision of 2026-11-23): a longer text would state a cause, and the
        // server sends none.
        Activity::Blocked => "Blocked",
    })
}

pub(in crate::widgets) fn primary_cargo(_context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Null
}

pub(in crate::widgets) fn primary_armour(context: SourceContext<'_>) -> serde_json::Value {
    let Some(unit) = context
        .selection
        .first()
        .and_then(|id| context.replica.units.get(id))
    else {
        return serde_json::Value::Null;
    };
    // Research is owner-only. Never infer an enemy's upgraded armour from its base definition.
    if unit.owner_slot != context.owner_slot {
        return serde_json::Value::Null;
    }
    let Ok(id) = turts_mod_api::ContentId::parse(&unit.content_id) else {
        return serde_json::Value::Null;
    };
    let order = context
        .replica
        .research
        .iter()
        .find(|row| row.slot.0 == context.owner_slot)
        .map(|row| {
            row.armour_order
                .iter()
                .filter_map(|text| turts_mod_api::ContentId::parse(text).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let Ok(Some(armour)) = turts_mod_api::armour::armour_for(&context.content.catalog, &id, &order)
    else {
        return serde_json::Value::Null;
    };
    let mut parts = Vec::new();
    let curve = context
        .content
        .catalog
        .combat
        .as_ref()
        .map_or(turts_mod_api::ArmourCurve::None, |table| table.curve);
    let ratio = |r: turts_mod_api::Ratio| {
        turts_rules::Ratio::new(u64::from(r.numerator), u64::from(r.denominator))
    };
    let curve = match curve {
        turts_mod_api::ArmourCurve::None => None,
        turts_mod_api::ArmourCurve::Ratio { k } => {
            ratio(k).map(|k| turts_rules::damage::Curve::Ratio { k })
        }
        turts_mod_api::ArmourCurve::Percent { per_point, cap } => ratio(per_point)
            .zip(ratio(cap))
            .map(|(per_point, cap)| turts_rules::damage::Curve::Percent { per_point, cap }),
    };
    if let Some(curve) = curve {
        let Some((num, den)) = curve.factor(armour.points) else {
            return serde_json::Value::Null;
        };
        parts.push(format!("{} ({} %)", armour.points, (den - num) * 100 / den));
        if armour.flat != 0 {
            parts.push(format!("flat {}", armour.flat));
        }
    } else {
        parts.push(armour.flat.to_string());
    }
    for (damage, value) in &armour.flat_by_type {
        if *value != armour.flat {
            parts.push(format!(
                "{} {}",
                damage.rsplit(':').next().unwrap_or(damage),
                value
            ));
        }
    }
    serde_json::json!(parts.join("; "))
}

pub(in crate::widgets) fn primary_weapon_details(context: SourceContext<'_>) -> serde_json::Value {
    let Some(id) = context.selection.first() else {
        return serde_json::Value::Null;
    };
    let Some(unit) = context
        .replica
        .units
        .get(id)
        .filter(|unit| unit.owner_slot == context.owner_slot)
    else {
        return serde_json::Value::Null;
    };
    let Some(weapon) = turts_mod_api::ContentId::parse(&unit.content_id)
        .ok()
        .and_then(|id| context.content.catalog.units.get(&id))
        .and_then(|unit| unit.weapon.as_ref())
    else {
        return serde_json::Value::Null;
    };
    let Some(damage) = context
        .replica
        .weapons
        .iter()
        .find(|row| row.unit.0 == *id)
        .and_then(|row| row.damage.as_ref().ok())
    else {
        return serde_json::Value::Null;
    };
    let kind = weapon
        .damage_type
        .as_deref()
        .map(|kind| kind.rsplit(':').next().unwrap_or(kind))
        .unwrap_or("untyped");
    let mut text = format!("Damage: {damage} {kind}");
    for bonus in &weapon.bonus {
        text.push_str(&format!(
            "; +{} against {}",
            bonus.add,
            bonus.against.rsplit(':').next().unwrap_or(&bonus.against)
        ));
    }
    serde_json::json!(text)
}

pub(in crate::widgets) fn predicate_primary_has_value(
    field: PrimaryField,
    context: &SourceContext<'_>,
) -> bool {
    !super::primary_value(field, *context).is_null()
}
