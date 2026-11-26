//! Button activation: the button state colours, and the input or order that an activated
//! widget button sends to the interaction core.

use super::*;

// One file for each task that writes an activation (plan 3.4, L0-CORE): `place` (`UI-PLACE`),
// `alert` (`UI-ALERT`), `cancel` (`UI-HUD`, the arm of `CancelProduction` moved as it was) and
// `leave` (`UI-RESULT`). Each task fills its own file; the arm that calls it exists.
mod alert;
mod cancel;
mod leave;
mod place;

type ButtonVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static bevy::ui::Pressed>,
        Option<&'static bevy::picking::hover::Hovered>,
        &'static primitives::WidgetEnabled,
        Option<&'static primitives::WidgetButtonColour>,
        Option<&'static primitives::WidgetButtonAction>,
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

/// The background of the button of the armed terrain tool.
pub(super) const ARMED_BUTTON: Color = Color::srgb(0.62, 0.45, 0.08);

pub(super) fn update_widget_button_states(
    runtime: Option<Res<crate::core_adapter::CoreRuntime>>,
    mut buttons: ButtonVisualQuery<'_, '_>,
) {
    let armed = runtime.as_deref().and_then(|r| r.terrain_preview.as_ref());
    for (pressed, hovered, enabled, colour, action, mut background) in &mut buttons {
        let is_armed = armed.is_some_and(|preview| {
            matches!(action.map(|action| &action.0), Some(primitives::WidgetAction::TerrainWork { tool, .. }) if *tool == preview.input.tool)
        });
        *background = BackgroundColor(if let Some(colour) = colour {
            colour.0.with_alpha(if enabled.0 { 1.0 } else { 0.35 })
        } else if !enabled.0 {
            Color::srgb(0.20, 0.20, 0.20)
        } else if is_armed {
            ARMED_BUTTON
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
    mut volumes: Option<ResMut<crate::volume_settings::VolumeSettings>>,
) {
    let Ok((action, enabled, interaction_disabled)) = actions.get(trigger.entity) else {
        return;
    };
    if interaction_disabled || enabled.is_some_and(|enabled| !enabled.0) {
        return;
    }
    match &action.0 {
        // The engine action `leave_match` (API 0.2.4, row H23) is `UI-RESULT`'s; every other input
        // goes to the core as before.
        primitives::WidgetAction::Input { name } if name == "leave_match" => {
            leave::activate(&mut queue, &state, acquired.as_deref())
        }
        // The engine action `toggle_sound` (API 0.2.4, row H25) flips the sound switch. It is a
        // client preference, so it never goes to the interaction core.
        primitives::WidgetAction::Input { name } if name == "toggle_sound" => {
            if let Some(volumes) = volumes.as_mut() {
                volumes.sound_on = !volumes.sound_on;
            }
        }
        primitives::WidgetAction::Input { name } => {
            queue
                .0
                .push(turts_interaction::InputEvent::UiAction { name: name.clone() });
        }
        primitives::WidgetAction::Order { id } => queue
            .0
            .push(turts_interaction::InputEvent::UiAction { name: id.clone() }),
        primitives::WidgetAction::TerrainWork { order, tool } => queue.0.push(
            turts_interaction::InputEvent::TerrainWork(turts_interaction::terrain::Input {
                order: order.clone(),
                tool: tool.clone(),
            }),
        ),
        primitives::WidgetAction::UtilityRoute { order, utility } => queue.0.push(
            turts_interaction::InputEvent::UtilityRoute(turts_interaction::utility::Input {
                order: order.clone(),
                utility: utility.clone(),
            }),
        ),
        primitives::WidgetAction::UnloadResume {
            match_id,
            map,
            unit,
            depot,
            unload_order,
            work_order,
            designation,
        } => {
            if let (Ok(match_id), Ok(map)) =
                (match_id.parse(), turts_mod_api::ContentId::parse(map))
            {
                queue.0.push(turts_interaction::InputEvent::UnloadResume(
                    turts_interaction::terrain::UnloadResume {
                        identity: (match_id, map),
                        unit: turts_net_protocol::UnitId(*unit),
                        depot: turts_net_protocol::UnitId(*depot),
                        unload_order: unload_order.clone(),
                        work_order: work_order.clone(),
                        designation: designation.clone(),
                    },
                ));
            }
        }
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
        } => cancel::activate(producer, job_id, &mut queue, &state, acquired.as_deref()),
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
        primitives::WidgetAction::FocusAlert { x, y, unit } => {
            alert::activate(*x, *y, *unit, &mut queue, &state, acquired.as_deref())
        }
    }
}

#[cfg(test)]
mod armed_tests {
    use super::*;
    use crate::core_adapter::CoreRuntime;

    #[test]
    fn the_button_of_the_armed_terrain_tool_shows_as_active() {
        let mut app = App::new();
        let mut runtime = CoreRuntime::default();
        runtime.terrain_preview = Some(turts_interaction::terrain::Preview {
            input: turts_interaction::terrain::Input {
                order: "quarry:order_work".into(),
                tool: "quarry:spade".into(),
            },
            target_elevation: None,
            cells: vec![],
            rectangle: None,
            can_submit: false,
            reason: None,
        });
        app.insert_resource(runtime);
        app.add_systems(Update, update_widget_button_states);
        let button = |tool: &str| {
            (
                Button,
                primitives::WidgetEnabled(true),
                primitives::WidgetButtonAction(primitives::WidgetAction::TerrainWork {
                    order: "quarry:order_work".into(),
                    tool: tool.into(),
                }),
                BackgroundColor::default(),
            )
        };
        let armed = app.world_mut().spawn(button("quarry:spade")).id();
        let other = app.world_mut().spawn(button("quarry:backfill")).id();
        app.update();
        let colour = |app: &App, entity| app.world().get::<BackgroundColor>(entity).unwrap().0;
        assert_eq!(colour(&app, armed), ARMED_BUTTON);
        assert_ne!(colour(&app, other), ARMED_BUTTON);
        app.world_mut()
            .resource_mut::<CoreRuntime>()
            .terrain_preview = None;
        app.update();
        assert_ne!(colour(&app, armed), ARMED_BUTTON);
    }
}
