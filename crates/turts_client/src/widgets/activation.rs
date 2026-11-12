//! Button activation: the button state colours, and the input or order that an activated
//! widget button sends to the interaction core.

use super::*;

// One file for each task that writes an activation (plan 3.4, L0-CORE). Each task fills its own
// file and adds at most one arm to `activate_widget`.
mod alert;
mod cancel;
mod leave;
mod place;
mod rally;

type ButtonVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static bevy::ui::Pressed>,
        Option<&'static bevy::picking::hover::Hovered>,
        &'static primitives::WidgetEnabled,
        &'static mut BackgroundColor,
    ),
    With<Button>,
>;

type ButtonActionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static primitives::WidgetButtonAction,
        Option<&'static primitives::WidgetEnabled>,
        Has<bevy::ui::InteractionDisabled>,
    ),
    With<Button>,
>;

pub(super) fn update_widget_button_states(mut buttons: ButtonVisualQuery<'_, '_>) {
    for (pressed, hovered, enabled, mut background) in &mut buttons {
        *background = BackgroundColor(if !enabled.0 {
            Color::srgb(0.20, 0.20, 0.20)
        } else if pressed.is_some() {
            Color::srgb(0.10, 0.30, 0.36)
        } else if hovered.is_some_and(|hovered| hovered.0) {
            Color::srgb(0.18, 0.40, 0.44)
        } else {
            Color::srgb(0.13, 0.26, 0.30)
        });
    }
}

pub(super) fn activate_widget(
    trigger: On<Activate>,
    actions: ButtonActionQuery<'_, '_>,
    mut queue: ResMut<CoreInputQueue>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
) {
    let Ok((action, enabled, interaction_disabled)) = actions.get(trigger.entity) else {
        return;
    };
    if interaction_disabled || enabled.is_some_and(|enabled| !enabled.0) {
        return;
    }
    match &action.0 {
        primitives::WidgetAction::Input { name } => {
            queue
                .0
                .push(turts_interaction::InputEvent::UiAction { name: name.clone() });
        }
        primitives::WidgetAction::Order { id } => queue
            .0
            .push(turts_interaction::InputEvent::UiAction { name: id.clone() }),
        primitives::WidgetAction::CenterCamera { x, y } => {
            if x.is_finite() && y.is_finite() {
                queue.0.push(turts_interaction::InputEvent::UiCameraCenter {
                    position: Vec2::new(*x, *y),
                });
            }
        }
        primitives::WidgetAction::Select { unit } => {
            queue.0.push(turts_interaction::InputEvent::UiAction {
                name: format!("ui_select:{unit}"),
            })
        }
        primitives::WidgetAction::Produce {
            producer,
            content_id,
        } => {
            let Some(acquired) = acquired.as_ref() else {
                return;
            };
            let Some(slot) = state.welcome.map(|welcome| welcome.slot) else {
                return;
            };
            let Some(replica) = state.replica.as_ref() else {
                return;
            };
            let Some(unit) = replica
                .units
                .get(producer)
                .filter(|unit| unit.owner_slot == slot)
            else {
                return;
            };
            let Some(unit_id) = turts_mod_api::ContentId::parse(&unit.content_id).ok() else {
                return;
            };
            let Some(definition) = acquired.0.content.catalog.units.get(&unit_id) else {
                return;
            };
            let accepts = definition.producer.as_ref().is_some_and(|producer| {
                producer
                    .recipes
                    .iter()
                    .any(|recipe| recipe.to_string() == *content_id)
            }) || definition.research.as_ref().is_some_and(|station| {
                station
                    .technologies
                    .iter()
                    .any(|technology| technology.to_string() == *content_id)
            });
            if accepts {
                let Some(order) = sources::produce_order(&acquired.0.content, &unit.content_id)
                    .map(|(id, _)| id.to_string())
                else {
                    return;
                };
                queue.0.push(turts_interaction::InputEvent::UiOrder {
                    order,
                    units: vec![turts_net_protocol::UnitId(*producer)],
                    target: turts_interaction::UiOrderTarget::Content(content_id.clone()),
                });
            }
        }
        primitives::WidgetAction::CancelProduction {
            producer,
            job: job_id,
        } => {
            let Some(acquired) = acquired.as_ref() else {
                return;
            };
            let Some(slot) = state.welcome.map(|welcome| welcome.slot) else {
                return;
            };
            let Some(replica) = state.replica.as_ref() else {
                return;
            };
            if let Some(unit) = replica
                .units
                .get(producer)
                .filter(|unit| unit.owner_slot == slot)
                .filter(|_| {
                    replica.economy.queues.iter().any(|queue| {
                        queue.producer.0 == *producer
                            && queue.jobs.iter().any(|job| job.id.0 == *job_id)
                    })
                })
            {
                let Some(order) = acquired
                    .0
                    .content
                    .catalog
                    .orders
                    .iter()
                    .find(|(_, order)| {
                        order.behaviour == turts_mod_api::Behaviour::CancelProduction
                            && order.target == turts_mod_api::TargetKind::Job
                            && (order.units.is_empty()
                                || order
                                    .units
                                    .iter()
                                    .any(|allowed| allowed == &unit.content_id))
                    })
                    .map(|(id, _)| id.to_string())
                else {
                    return;
                };
                queue.0.push(turts_interaction::InputEvent::UiOrder {
                    order,
                    units: vec![turts_net_protocol::UnitId(*producer)],
                    target: turts_interaction::UiOrderTarget::Job(turts_net_protocol::JobId(
                        *job_id,
                    )),
                });
            }
        }
        primitives::WidgetAction::Place {
            constructor,
            content,
        } => place::activate(
            *constructor,
            content,
            &mut queue,
            &state,
            acquired.as_deref(),
        ),
        primitives::WidgetAction::FocusAlert => {
            alert::activate(&mut queue, &state, acquired.as_deref())
        }
    }
}
