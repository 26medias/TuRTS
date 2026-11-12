//! The `Effect` arms of `drive_core_adapter`: one function for each effect family.
//!
//! Each function records what the arm recorded before the split: `last_effects`, `markers`, the
//! marker feed and the pending commands. `Effect::SelectionChanged`, `GroupsChanged`,
//! `CameraChanged` and the dropped `Send` stay empty arms in the caller.

use super::*;
use turts_net_protocol::OrderEnvelope;

/// `Effect::Send`: the order intent, the marker intent and the visual order.
pub(super) fn apply_send(
    order: OrderEnvelope,
    runtime: &mut CoreRuntime,
    state: &ReplicaState,
    output: &mut AdapterOutputs,
) {
    runtime.last_effects.push(ValueEffect {
            kind: "order_intent".into(),
            value: serde_json::json!({"units":order.units.iter().map(|unit| unit.0).collect::<Vec<_>>() }),
        });
    let marker_kind = state
        .content
        .as_ref()
        .and_then(|content| {
            ContentId::parse(&order.order)
                .ok()
                .and_then(|id| content.catalog.orders.get(&id))
        })
        .and_then(|order_def| match order_def.behaviour {
            turts_mod_api::Behaviour::Move => Some("move"),
            turts_mod_api::Behaviour::Attack => Some("attack"),
            turts_mod_api::Behaviour::Gather => Some("gather"),
            _ => None,
        });
    let intent_id = runtime.next_order_intent_id;
    runtime.next_order_intent_id = intent_id.wrapping_add(1);
    runtime.order_intents_this_frame.push(intent_id);
    if let Some(kind) = marker_kind {
        output
            .order_marker_feed
            .intents
            .push(crate::commands::OrderMarkerIntent {
                id: intent_id,
                kind: kind.to_owned(),
                order: order.clone(),
            });
        runtime.markers.push(serde_json::json!({
            "intent_id": intent_id,
            "kind": kind,
            "order": &order.order,
            "units": order.units.iter().map(|unit| unit.0).collect::<Vec<_>>(),
            "target": &order.target,
        }));
    }
    crate::commands::submit_visual_order(&mut output.pending, order, intent_id);
}

/// `Effect::Marker`: a ground marker.
pub(super) fn apply_marker(point: Vec2, kind: String, runtime: &mut CoreRuntime) {
    let value = serde_json::json!({"x":point.x,"y":point.y,"kind":kind});
    runtime.markers.push(value.clone());
    runtime.last_effects.push(ValueEffect {
        kind: "marker".into(),
        value,
    });
}

/// `Effect::Notice`: a local notice with its units.
pub(super) fn apply_notice(text_key: String, units: Vec<UnitId>, runtime: &mut CoreRuntime) {
    let value = serde_json::json!({"text_key":text_key,"units":units.iter().map(|id|id.0).collect::<Vec<_>>()});
    runtime.last_effects.push(ValueEffect {
        kind: "notice".into(),
        value,
    });
}

/// `Effect::Sound`: a sound by id and category.
pub(super) fn apply_sound(id: String, category: String, runtime: &mut CoreRuntime) {
    let value = serde_json::json!({"id":id,"category":category});
    runtime.last_effects.push(ValueEffect {
        kind: "sound".into(),
        value: value.clone(),
    });
}

/// `Effect::Cursor`: a cursor context.
pub(super) fn apply_cursor(context: String, runtime: &mut CoreRuntime) {
    runtime.last_effects.push(ValueEffect {
        kind: "cursor".into(),
        value: serde_json::json!({"context":context}),
    });
}

/// `Effect::ShellAction`: a shell action by name.
pub(super) fn apply_shell_action(action: String, runtime: &mut CoreRuntime) {
    let value = serde_json::json!({"action":action});
    runtime.last_effects.push(ValueEffect {
        kind: "shell_action".into(),
        value: value.clone(),
    });
}

// One file for each effect family that a later task adds (plan 3.4, L0-CORE): the `Effect` variant
// exists, and `drive_core_adapter` calls `<family>::apply` for it. The owner fills `apply` and the
// family file only.
pub(super) mod hint;
pub(super) mod placement;
