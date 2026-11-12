//! Bound values of the widgets: refresh each frame, text style, predicates, fills, text,
//! numbers, icons, and button actions read from the sources.

use super::*;

pub(super) fn refresh_bound_widgets(world: &mut World, context: &sources::SourceContext<'_>) {
    reconcile_list_rows(world, context);
    let bindings = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<WidgetBinding>()
                .cloned()
                .map(|binding| (entity.id(), binding))
        })
        .collect::<Vec<_>>();
    for (entity, binding) in bindings {
        if let Primitive::Bar {
            value,
            max,
            direction,
            ..
        } = &binding.primitive
        {
            let current = numeric_value(value, context, binding.item.as_ref()).unwrap_or(0.0);
            let maximum = numeric_value(max, context, binding.item.as_ref()).unwrap_or(1.0);
            let ratio = if maximum.is_finite() && maximum > 0.0 {
                (current / maximum).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let child = world
                .get::<Children>(entity)
                .and_then(|children| children.first())
                .copied();
            if let Some(child) = child
                && let Some(mut node) = world.get_mut::<Node>(child)
            {
                *node = primitives::bar_fill_node(*direction, ratio);
            }
        }
        let text = match &binding.primitive {
            Primitive::Text { text, .. } => Some(text_value(text, context, binding.item.as_ref())),
            Primitive::Button {
                label,
                action,
                hotkey_label,
                ..
            } if label.is_some() || *hotkey_label => Some(button_text(
                label.as_ref(),
                action,
                *hotkey_label,
                context,
                binding.item.as_ref(),
            )),
            _ => None,
        };
        if let Some(text) = text {
            let has_text = !text.is_empty();
            if !has_text {
                if world.get::<Text>(entity).is_some() {
                    world.entity_mut(entity).remove::<Text>();
                }
            } else if let Some(mut current) = world.get_mut::<Text>(entity) {
                current.0 = text;
            } else {
                world.entity_mut(entity).insert(Text::new(text));
            }
            if has_text {
                ensure_widget_text_style(world, entity, &binding);
            }
        }
        if let Primitive::Button { action, .. } = &binding.primitive {
            let action = resolve_action(action, binding.item.as_ref());
            if let Some(mut current) = world.get_mut::<primitives::WidgetButtonAction>(entity) {
                if let Some(action) = action {
                    current.0 = action;
                }
            } else if let Some(action) = action {
                world.entity_mut(entity).insert((
                    primitives::WidgetButtonAction(action),
                    observe(activate_widget),
                ));
            }
        }
        refresh_capture_marker(world, entity, &binding.primitive);
        let enabled = binding
            .item
            .as_ref()
            .and_then(|item| item.get(ItemField::Enabled))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        if let Some(mut current) = world.get_mut::<primitives::WidgetEnabled>(entity) {
            current.0 = enabled;
        }
        if enabled {
            world
                .entity_mut(entity)
                .remove::<bevy::ui::InteractionDisabled>();
        } else {
            world
                .entity_mut(entity)
                .insert(bevy::ui::InteractionDisabled);
        }
        if let Some(value) = binding.tooltip.as_ref() {
            let tooltip = display_value(&sources::read_value_for_item(
                value,
                *context,
                binding.item.as_ref(),
            ));
            if let Some(mut current) = world.get_mut::<primitives::WidgetTooltip>(entity) {
                current.0 = tooltip;
            } else {
                world
                    .entity_mut(entity)
                    .insert(primitives::WidgetTooltip(tooltip));
            }
        }
        let visible = binding
            .visible
            .iter()
            .all(|predicate| predicate_matches(predicate, context));
        if let Some(mut current) = world.get_mut::<Visibility>(entity) {
            *current = widget_visibility(visible);
        }
    }
}

fn ensure_widget_text_style(world: &mut World, entity: Entity, binding: &WidgetBinding) {
    match &binding.primitive {
        Primitive::Text {
            font,
            size,
            colour,
            align,
            wrap,
            ..
        } => {
            if world.get::<TextFont>(entity).is_none() {
                let font = font
                    .as_deref()
                    .and_then(|font| {
                        world
                            .get_resource::<assets::WidgetAssets>()
                            .and_then(|assets| assets.font(&qualify(&binding.package, font)))
                    })
                    .unwrap_or_default();
                world.entity_mut(entity).insert(TextFont {
                    font_size: FontSize::Px(*size as f32),
                    font: FontSource::Handle(font),
                    ..default()
                });
            }
            if world.get::<TextColor>(entity).is_none() {
                world.entity_mut(entity).insert(TextColor(
                    primitives::parse_colour(colour).unwrap_or(Color::WHITE),
                ));
            }
            if world.get::<TextLayout>(entity).is_none() {
                world.entity_mut(entity).insert(TextLayout::new(
                    primitives::text_alignment(*align),
                    if *wrap {
                        LineBreak::WordBoundary
                    } else {
                        LineBreak::NoWrap
                    },
                ));
            }
        }
        Primitive::Button { .. } => {
            // `Text` requires `TextFont` (bevy_ui-0.20.0/src/widget/text.rs:116-119), so a label
            // that the refresh adds has the default font. Compare first; set only on a change.
            let font = FontSource::Handle(tooltip::style_font(
                world,
                &binding.tooltip_style,
                &binding.package,
            ));
            let size = FontSize::Px(14.0);
            let current = world
                .get::<TextFont>(entity)
                .map(|text_font| (text_font.font.clone(), text_font.font_size));
            if current.as_ref() != Some(&(font.clone(), size)) {
                world.entity_mut(entity).insert(TextFont {
                    font_size: size,
                    font,
                    ..default()
                });
            }
            if world.get::<TextColor>(entity).is_none() {
                world.entity_mut(entity).insert(TextColor(Color::WHITE));
            }
        }
        _ => {}
    }
}

pub(super) fn predicate_matches(
    predicate: &Predicate,
    context: &sources::SourceContext<'_>,
) -> bool {
    match predicate {
        Predicate::SelectionEmpty => context.selection.is_empty(),
        Predicate::SelectionNotEmpty => !context.selection.is_empty(),
        Predicate::SelectionSingle => context.selection.len() == 1,
        Predicate::SelectionMany => context.selection.len() > 1,
        Predicate::PrimaryOwned => context
            .selection
            .first()
            .and_then(|id| context.replica.units.get(id))
            .is_some_and(|unit| unit.owner_slot == context.owner_slot),
        Predicate::PrimaryForeign => context
            .selection
            .first()
            .and_then(|id| context.replica.units.get(id))
            .is_some_and(|unit| unit.owner_slot != context.owner_slot),
        Predicate::PrimaryHas(capability) => context
            .selection
            .first()
            .and_then(|id| context.replica.units.get(id))
            .and_then(|unit| {
                turts_mod_api::ContentId::parse(&unit.content_id)
                    .ok()
                    .and_then(|id| context.content.catalog.units.get(&id))
            })
            .is_some_and(|unit| match capability {
                turts_mod_api::Capability::Movement => unit.movement.is_some(),
                turts_mod_api::Capability::Weapon => unit.weapon.is_some(),
                turts_mod_api::Capability::Gatherer => unit.gatherer.is_some(),
                turts_mod_api::Capability::Producer => unit.producer.is_some(),
                turts_mod_api::Capability::Research => unit.research.is_some(),
                // API 0.2.4 (row O6): the resolved unit has no constructor or repairer yet.
                turts_mod_api::Capability::Constructor | turts_mod_api::Capability::Repairer => {
                    false
                }
            }),
        Predicate::GuideOpen => context.guide_open,
        Predicate::Not(inner) => !predicate_matches(inner, context),
        // API 0.2.4 (row H10): the predicates of `UI-HUD`, `UI-PLACE`, `UI-RESULT`, `UI-HINT`
        // and `UI2-HUD`. Until then none holds (a widget behind one stays hidden).
        Predicate::ItemHas(_)
        | Predicate::PrimaryHasValue(_)
        | Predicate::SupplyShort(_)
        | Predicate::MatchOver
        | Predicate::OutcomeIs(_)
        | Predicate::OutcomeReasonIs(_)
        | Predicate::Placing
        | Predicate::HintShown => false,
    }
}

pub(super) fn primitive_fill(primitive: &Primitive) -> Option<Fill> {
    match primitive {
        Primitive::Box { fill, .. } => fill.clone(),
        Primitive::MinimapView { background, .. } => background.clone(),
        _ => None,
    }
}

pub(super) fn primitive_border(primitive: &Primitive) -> Option<&str> {
    match primitive {
        Primitive::Box { border, .. } => border.as_deref(),
        Primitive::MinimapView { frame, .. } => Some(frame),
        _ => None,
    }
}

pub(super) fn text_value(
    value: &TextValue,
    context: &sources::SourceContext<'_>,
    item: Option<&sources::ItemData>,
) -> String {
    match value {
        TextValue::Literal(text) => text.clone(),
        TextValue::Bind(value) => {
            display_value(&sources::read_value_for_item(value, *context, item))
        }
        TextValue::Item(field) => item
            .and_then(|item| item.get(*field))
            .map(display_value)
            .unwrap_or_default(),
        TextValue::Format(pattern, values) => {
            let value = Value::Format(pattern.clone(), values.clone());
            display_value(&sources::read_value_for_item(&value, *context, item))
        }
    }
}

pub(super) fn button_text(
    label: Option<&TextValue>,
    action: &ButtonAction,
    show_hotkey: bool,
    context: &sources::SourceContext<'_>,
    item: Option<&sources::ItemData>,
) -> String {
    let mut text = label.map_or_else(String::new, |label| text_value(label, context, item));
    if !show_hotkey {
        return text;
    }
    let action_name = match resolve_action(action, item) {
        Some(primitives::WidgetAction::Input { name }) => Some(name),
        Some(primitives::WidgetAction::Order { id }) => ContentId::parse(&id).ok().and_then(|id| {
            context
                .content
                .catalog
                .orders
                .get(&id)
                .map(|order| order.hotkey.clone())
        }),
        _ => None,
    };
    if let Some(label) = action_name
        .map(|name| sources::current_hotkey(*context, &name))
        .filter(|label| !label.is_empty())
    {
        text.push_str(" [");
        text.push_str(&label);
        text.push(']');
    }
    text
}

pub(super) fn numeric_value(
    value: &Value,
    context: &sources::SourceContext<'_>,
    item: Option<&sources::ItemData>,
) -> Option<f32> {
    sources::read_value_for_item(value, *context, item)
        .as_f64()
        .map(|number| number as f32)
        .filter(|number| number.is_finite())
}

pub(super) fn display_value(value: &serde_json::Value) -> String {
    sources::display_text_value(value)
}

pub(super) fn value_uses_item(value: &Value) -> bool {
    match value {
        Value::Item(_) => true,
        Value::Format(_, arguments) => arguments.iter().any(value_uses_item),
        _ => false,
    }
}

pub(super) fn resolve_icon(
    icon: &ButtonIcon,
    context: &sources::SourceContext<'_>,
    item: Option<&sources::ItemData>,
    interface_package: &str,
) -> Option<String> {
    match icon {
        ButtonIcon::Asset(id) => Some(qualify(interface_package, id)),
        ButtonIcon::Item(field) => item
            .and_then(|item| {
                item.get(match field {
                    turts_mod_api::IconField::Icon => ItemField::Icon,
                })
            })
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        ButtonIcon::Bind(value) => sources::read_value_for_item(value, *context, item)
            .as_str()
            .map(str::to_owned),
    }
}

pub(super) fn resolve_action(
    action: &ButtonAction,
    item: Option<&sources::ItemData>,
) -> Option<primitives::WidgetAction> {
    match action {
        ButtonAction::Input(name) => Some(primitives::WidgetAction::Input { name: name.clone() }),
        ButtonAction::Order(id) => Some(primitives::WidgetAction::Order { id: id.clone() }),
        ButtonAction::Item(field) => {
            let field = match field {
                turts_mod_api::ActionField::Action => ItemField::Action,
                turts_mod_api::ActionField::CancelAction => ItemField::CancelAction,
            };
            item?
                .get(field)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        }
    }
}

pub(super) fn qualify(package: &str, id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("{package}:{id}")
    }
}
