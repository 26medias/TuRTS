//! The retained widget tree: build and clear it from the accepted interface, spawn each
//! widget, reconcile list rows, place nodes, and resolve Fit layout.

use super::*;

/// Resolve Fit axes after the content layout has been measured at least once. Bevy measures
/// text and ordinary images intrinsically; absolute Box/List/Grid children need their measured
/// bounds folded into the container explicitly. This keeps Fit content driven by actual UI
/// layout rather than estimated glyph widths.
pub(super) fn update_fit_layout(world: &mut World) {
    let entities = world
        .iter_entities()
        .filter_map(|entity| entity.get::<WidgetBinding>().map(|_| entity.id()))
        .collect::<Vec<_>>();
    for entity in entities.into_iter().rev() {
        let Some(binding) = world.get::<WidgetBinding>(entity).cloned() else {
            continue;
        };
        let fit_x = matches!(binding.size.w, turts_mod_api::Dimension::Fit);
        let fit_y = matches!(binding.size.h, turts_mod_api::Dimension::Fit);
        if !fit_x && !fit_y {
            continue;
        }

        let mut needed = Vec2::ZERO;
        let mut has_measure = false;
        if let Some(children) = world.get::<Children>(entity) {
            for child in children.iter() {
                let Some(size) = measured_authored_size(world, child) else {
                    continue;
                };
                let Some(child_binding) = world.get::<WidgetBinding>(child) else {
                    continue;
                };
                if let Some(extent) = placement::fit_parent_extent(&child_binding.at, size) {
                    needed = needed.max(extent);
                    has_measure = true;
                }
            }
        }

        // Sliced image nodes intentionally opt out of Bevy's intrinsic image measurement.
        // Their source dimensions remain available from the retained render-only asset.
        if let Some(image_node) = world.get::<ImageNode>(entity)
            && matches!(image_node.image_mode, NodeImageMode::Sliced(_))
            && let Some(image) = world
                .get_resource::<Assets<Image>>()
                .and_then(|images| images.get(&image_node.image))
        {
            // Image content measurement multiplies source pixels by the target scale factor
            // before Taffy computes its physical size. `Node::Px` expresses the same authored
            // virtual pixel count, so preserve the source dimensions here.
            let dimensions = image.size().as_vec2();
            if dimensions.is_finite() {
                needed = needed.max(dimensions);
                has_measure = true;
            }
        }

        let empty_authored_container = !has_measure
            && world
                .get::<Children>(entity)
                .is_none_or(|children| children.is_empty())
            && matches!(
                binding.primitive,
                Primitive::Box { .. } | Primitive::List { .. } | Primitive::Grid { .. }
            );
        if !has_measure && !empty_authored_container {
            continue;
        }
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            if fit_x {
                node.width = placement::pixel(needed.x);
            }
            if fit_y {
                node.height = placement::pixel(needed.y);
            }
        }
        let base_extent = placement::authored_extent(binding.parent_size, binding.size);
        let fitted_extent = Vec2::new(
            if fit_x { needed.x } else { base_extent.x },
            if fit_y { needed.y } else { base_extent.y },
        );
        let children = world
            .get::<Children>(entity)
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for child in children {
            let Some(child_binding) = world.get::<WidgetBinding>(child).cloned() else {
                continue;
            };
            if let Some((node, transform)) =
                placement::authored_node(fitted_extent, &child_binding.at, child_binding.size)
            {
                if let Some(mut current) = world.get_mut::<Node>(child) {
                    apply_node_placement(
                        &mut current,
                        node,
                        matches!(child_binding.size.w, turts_mod_api::Dimension::Fit),
                        matches!(child_binding.size.h, turts_mod_api::Dimension::Fit),
                    );
                }
                if let Some(mut current) = world.get_mut::<UiTransform>(child) {
                    *current = transform;
                }
            }
            if let Some(mut child_binding) = world.get_mut::<WidgetBinding>(child) {
                child_binding.parent_size = fitted_extent;
            }
        }
    }
}

pub(super) fn sync_widget_tree(world: &mut World) {
    let Some(phase) = world.get_resource::<Phase>() else {
        return;
    };
    if matches!(phase, Phase::Failed(_)) {
        clear_widget_tree(world);
        return;
    }
    if !matches!(phase, Phase::Joined { .. }) {
        return;
    }
    let Some(acquired) = world.get_resource::<AcquiredPackages>() else {
        return;
    };
    if !world
        .get_resource::<assets::WidgetAssets>()
        .is_some_and(|assets| {
            assets.matches_accepted_packages(&acquired.0.lockfile_hash, &acquired.0.presentation)
        })
    {
        return;
    }
    let Some(interface) = acquired.0.presentation.interface.clone() else {
        return;
    };
    let interface_package = acquired.0.presentation.interface_package.clone();
    let acquired_content = acquired.0.content.clone();
    let Some(state) = world.get_resource::<ReplicaState>() else {
        return;
    };
    let Some(replica) = state.replica.clone() else {
        return;
    };
    let content = state.content.as_ref().unwrap_or(&acquired_content).clone();
    let Some(owner_slot) = state.welcome.map(|welcome| welcome.slot) else {
        return;
    };
    let selection = world.get_resource::<SelectionFeed>().map_or_else(
        || {
            world
                .get_resource::<crate::ui_shared::Selection>()
                .and_then(|selection| selection.0)
                .into_iter()
                .collect::<Vec<_>>()
        },
        |selection| selection.0.clone(),
    );
    let window_size = world
        .iter_entities()
        .find(|entity| entity.contains::<PrimaryWindow>())
        .and_then(|entity| entity.get::<Window>())
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(window_size) = window_size else {
        return;
    };
    let ui_scale = world
        .get_resource::<bevy::ui::UiScale>()
        .map_or(1.0, |scale| scale.0);
    let Some(authored_size) = placement::authored_size(window_size, ui_scale) else {
        return;
    };
    let key = serde_json::to_string(&(
        &interface_package,
        &interface,
        &selection,
        window_size,
        ui_scale,
        world.resource::<assets::WidgetAssets>().revision,
    ))
    .unwrap_or_default();
    let guide_open = world.resource::<guide::GuideOpen>().is_open;
    let groups = world
        .get_resource::<GroupsFeed>()
        .map_or_else(Default::default, |feed| feed.0.clone());
    let alerts = world
        .get_resource::<AlertsFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let notices = world
        .get_resource::<NoticesFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let bindings = world
        .get_resource::<crate::core_adapter::CorePreferences>()
        .map_or_else(Default::default, |prefs| prefs.runtime_bindings.clone());
    let binding_labels = world
        .get_resource::<crate::client_shell::BindingLabels>()
        .cloned()
        .unwrap_or_default();
    let source_context = sources::SourceContext {
        binding_labels: Some(&binding_labels),
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot,
        interface: Some(&interface),
        interface_package: interface_package.as_deref(),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &content.input_map,
        guide: &interface.guide,
        guide_open,
        last_notice: notices
            .last()
            .and_then(|notice| notice.get("text").or_else(|| notice.get("message")))
            .and_then(serde_json::Value::as_str),
    };
    if world.resource::<WidgetTree>().key == key {
        refresh_bound_widgets(world, &source_context);
        return;
    }
    tooltip::clear_tooltips(world);
    let old = std::mem::take(&mut world.resource_mut::<WidgetTree>().entities);
    for entity in old {
        let _ = world.despawn(entity);
    }
    let mut roots = Vec::new();
    for widget in &interface.root {
        roots.push(spawn_widget(
            world,
            None,
            widget,
            authored_size,
            &source_context,
            interface_package.as_deref().unwrap_or_default(),
            &interface.tooltip_style,
            None,
        ));
    }
    for (page_index, page) in interface.guide.iter().enumerate() {
        for widget in &page.widgets {
            let mut widget = widget.clone();
            widget.id = format!("guide.{page_index}.{}", widget.id);
            widget.visible.push(Predicate::GuideOpen);
            roots.push(spawn_widget(
                world,
                None,
                &widget,
                authored_size,
                &source_context,
                interface_package.as_deref().unwrap_or_default(),
                &interface.tooltip_style,
                None,
            ));
        }
    }
    refresh_bound_widgets(world, &source_context);
    world.resource_mut::<WidgetTree>().entities = roots;
    world.resource_mut::<WidgetTree>().key = key;
}

fn clear_widget_tree(world: &mut World) {
    let old = std::mem::take(&mut world.resource_mut::<WidgetTree>().entities);
    for entity in old {
        let _ = world.despawn(entity);
    }
    world.resource_mut::<WidgetTree>().key.clear();
    tooltip::clear_tooltips(world);
    cursor::clear_cursor(world);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_widget(
    world: &mut World,
    parent: Option<Entity>,
    widget: &Widget,
    parent_size: Vec2,
    source_context: &sources::SourceContext<'_>,
    interface_package: &str,
    tooltip_style: &turts_mod_api::TooltipStyle,
    item: Option<&sources::ItemData>,
) -> Entity {
    let at = widget.at.unwrap_or(turts_mod_api::At {
        anchor: Anchor::TopLeft,
        offset: (0.0, 0.0),
        pivot: None,
    });
    let (layout_node, transform) = placement::authored_node(parent_size, &at, widget.size)
        .unwrap_or((
            Node {
                width: placement::pixel(0.0),
                height: placement::pixel(0.0),
                ..default()
            },
            UiTransform::IDENTITY,
        ));
    let declared_tooltip = widget.tooltip.clone().or_else(|| match &widget.widget {
        Primitive::Tooltip { value } => Some(value.clone()),
        _ => None,
    });
    let row_tooltip = matches!(
        &widget.widget,
        Primitive::List { .. } | Primitive::Grid { .. }
    )
    .then(|| declared_tooltip.as_ref())
    .flatten()
    .filter(|value| value_uses_item(value))
    .cloned();
    let bound_tooltip = if row_tooltip.is_some() {
        None
    } else {
        declared_tooltip.clone()
    };
    let id = world
        .spawn((
            layout_node,
            transform,
            ZIndex(widget.layer as i32),
            primitives::AuthoredWidgetId(widget.id.clone()),
            primitives::WidgetEnabled(true),
            WidgetBinding {
                primitive: widget.widget.clone(),
                at,
                size: widget.size,
                item: item.cloned(),
                tooltip: bound_tooltip.clone(),
                row_tooltip: row_tooltip.clone(),
                visible: widget.visible.clone(),
                parent_size,
                package: interface_package.to_owned(),
                tooltip_style: tooltip_style.clone(),
            },
            BackgroundColor(Color::NONE),
            widget_visibility(
                widget
                    .visible
                    .iter()
                    .all(|predicate| predicate_matches(predicate, source_context)),
            ),
        ))
        .id();
    configure_scroll_area(world, id, &widget.widget);
    let fill = primitive_fill(&widget.widget);
    if let Some(color) = primitives::fill_colour(fill.as_ref()) {
        world.entity_mut(id).insert(BackgroundColor(color));
    }
    if let Some(Fill::Image {
        id: image,
        nine_slice,
    }) = fill.as_ref()
    {
        let image_id = qualify(interface_package, image);
        if let Some(handle) = world
            .get_resource::<assets::WidgetAssets>()
            .and_then(|assets| assets.image(&image_id))
        {
            let margins = nine_slice.or_else(|| {
                world
                    .get_resource::<assets::WidgetAssets>()
                    .and_then(|assets| assets.nine_slice(&image_id))
            });
            entity_insert_image(world, id, handle, None, margins);
        }
    }
    if let Some(border) = primitive_border(&widget.widget).and_then(primitives::parse_colour) {
        world.entity_mut(id).insert(BorderColor::all(border));
        if let Some(mut node) = world.get_mut::<Node>(id) {
            node.border = UiRect::all(placement::pixel(1.0));
        }
    }
    if let Some(parent) = parent {
        world.entity_mut(parent).add_child(id);
    }

    if let Some(value) = bound_tooltip.as_ref().filter(|_| widget.tooltip.is_some()) {
        let tooltip = sources::read_value_for_item(value, *source_context, item);
        world.entity_mut(id).insert((
            primitives::WidgetTooltip(display_value(&tooltip)),
            bevy::picking::hover::Hovered(false),
        ));
    }

    match &widget.widget {
        Primitive::Box { children, .. } => {
            let child_size = placement::authored_extent(parent_size, widget.size);
            for child in children {
                spawn_widget(
                    world,
                    Some(id),
                    child,
                    child_size,
                    source_context,
                    interface_package,
                    tooltip_style,
                    item,
                );
            }
        }
        Primitive::Image {
            image,
            nine_slice,
            tint,
        } => {
            let image_id = resolve_icon(image, source_context, item, interface_package);
            if let Some(image_id) = image_id
                && let Some(handle) = world
                    .get_resource::<assets::WidgetAssets>()
                    .and_then(|assets| assets.image(&image_id))
            {
                let margins = nine_slice.or_else(|| {
                    world
                        .get_resource::<assets::WidgetAssets>()
                        .and_then(|assets| assets.nine_slice(&image_id))
                });
                entity_insert_image(world, id, handle, tint.as_deref(), margins);
            }
        }
        Primitive::Text {
            text,
            font,
            size: font_size,
            colour,
            align,
            wrap,
        } => {
            let text = text_value(text, source_context, item);
            let font_handle = font.as_deref().and_then(|font| {
                world
                    .get_resource::<assets::WidgetAssets>()
                    .and_then(|assets| assets.font(&qualify(interface_package, font)))
            });
            let mut text_bundle = (
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(*font_size as f32),
                    font: FontSource::Handle(font_handle.unwrap_or_default()),
                    ..default()
                },
                TextColor(primitives::parse_colour(colour).unwrap_or(Color::WHITE)),
                TextLayout::new(
                    primitives::text_alignment(*align),
                    if *wrap {
                        LineBreak::WordBoundary
                    } else {
                        LineBreak::NoWrap
                    },
                ),
            );
            world
                .entity_mut(id)
                .insert(std::mem::take(&mut text_bundle));
        }
        Primitive::Button {
            icon,
            label,
            action: action_def,
            hotkey_label,
        } => {
            world
                .entity_mut(id)
                .insert((Button, bevy::picking::hover::Hovered(false)));
            let action = resolve_action(action_def, item);
            if let Some(action) = action {
                world.entity_mut(id).insert((
                    primitives::WidgetButtonAction(action),
                    observe(activate_widget),
                ));
            }
            if let Some(icon) = icon
                && let Some(image_id) = resolve_icon(icon, source_context, item, interface_package)
                && let Some(handle) = world
                    .get_resource::<assets::WidgetAssets>()
                    .and_then(|assets| assets.image(&image_id))
            {
                let margins = world
                    .get_resource::<assets::WidgetAssets>()
                    .and_then(|assets| assets.nine_slice(&image_id));
                entity_insert_image(world, id, handle, None, margins);
            }
            if label.is_some() || *hotkey_label {
                let text = button_text(
                    label.as_ref(),
                    action_def,
                    *hotkey_label,
                    source_context,
                    item,
                );
                if !text.is_empty() {
                    let font = tooltip::style_font(world, tooltip_style, interface_package);
                    world.entity_mut(id).insert((
                        Text::new(text),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            font: FontSource::Handle(font),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                }
            }
        }
        Primitive::Bar {
            value,
            max,
            fill,
            background,
            direction,
        } => {
            if let Some(color) = primitives::fill_colour(Some(background)) {
                world.entity_mut(id).insert(BackgroundColor(color));
            }
            if let Fill::Image {
                id: image,
                nine_slice,
            } = background
            {
                let image_id = qualify(interface_package, image);
                if let Some((handle, margins)) = world
                    .get_resource::<assets::WidgetAssets>()
                    .and_then(|assets| {
                        let handle = assets.image(&image_id)?;
                        let margins = nine_slice.or_else(|| assets.nine_slice(&image_id));
                        Some((handle, margins))
                    })
                {
                    entity_insert_image(world, id, handle, None, margins);
                }
            }
            let current = numeric_value(value, source_context, item).unwrap_or(0.0);
            let maximum = numeric_value(max, source_context, item).unwrap_or(1.0);
            let ratio = if maximum.is_finite() && maximum > 0.0 {
                (current / maximum).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let image_fill = match fill {
                Fill::Image {
                    id: image,
                    nine_slice,
                } => {
                    let image_id = qualify(interface_package, image);
                    world
                        .get_resource::<assets::WidgetAssets>()
                        .and_then(|assets| {
                            let handle = assets.image(&image_id)?;
                            let margins = nine_slice.or_else(|| assets.nine_slice(&image_id));
                            Some((handle, margins))
                        })
                }
                Fill::Colour(_) => None,
            };
            let child = world
                .spawn((
                    primitives::bar_fill_node(*direction, ratio),
                    BackgroundColor(primitives::fill_colour(Some(fill)).unwrap_or(Color::WHITE)),
                ))
                .id();
            if let Some((handle, margins)) = image_fill {
                entity_insert_image(world, child, handle, None, margins);
            }
            world.entity_mut(id).add_child(child);
        }
        Primitive::List { .. } | Primitive::Grid { .. } => {}
        Primitive::Tooltip { value } if row_tooltip.is_none() => {
            let tooltip = sources::read_value_for_item(value, *source_context, item);
            world.entity_mut(id).insert((
                primitives::WidgetTooltip(display_value(&tooltip)),
                bevy::picking::hover::Hovered(false),
            ));
        }
        Primitive::Tooltip { .. } => {}
        Primitive::Cursor { .. } => {
            world.entity_mut(id).insert(Visibility::Hidden);
        }
        Primitive::MinimapView {
            background,
            frame,
            unit_size,
            ping,
        } => {
            if let Some(mut node) = world.get_mut::<Node>(id) {
                node.overflow = Overflow::clip();
            }
            world.entity_mut(id).insert(primitives::MinimapViewConfig {
                unit_size: if unit_size.is_finite() && *unit_size > 0.0 {
                    *unit_size
                } else {
                    2.0
                },
                frame_colour: primitives::parse_colour(frame),
                ping_colour: ping.as_deref().and_then(primitives::parse_colour),
            });
            if let Some(color) = primitives::fill_colour(background.as_ref()) {
                world.entity_mut(id).insert(BackgroundColor(color));
            }
            if let Some(color) = primitives::parse_colour(frame) {
                world.entity_mut(id).insert(BorderColor::all(color));
                if let Some(mut node) = world.get_mut::<Node>(id) {
                    node.border = UiRect::all(placement::pixel(1.0));
                }
            }
        }
    }
    refresh_capture_marker(world, id, &widget.widget);
    if let Some(tooltip) = world
        .get::<primitives::WidgetTooltip>(id)
        .map(|tooltip| tooltip.0.clone())
    {
        tooltip::spawn_tooltip_view(world, id, &tooltip, tooltip_style, interface_package);
    }
    id
}

fn configure_scroll_area(world: &mut World, entity: Entity, primitive: &Primitive) {
    let overflow = match primitive {
        Primitive::List { direction, .. } => match direction {
            turts_mod_api::Direction::Horizontal
            | turts_mod_api::Direction::Left
            | turts_mod_api::Direction::Right => Overflow {
                x: OverflowAxis::Scroll,
                y: OverflowAxis::Clip,
            },
            turts_mod_api::Direction::Vertical
            | turts_mod_api::Direction::Up
            | turts_mod_api::Direction::Down => Overflow {
                x: OverflowAxis::Clip,
                y: OverflowAxis::Scroll,
            },
        },
        Primitive::Grid { .. } => Overflow::scroll(),
        _ => return,
    };
    if let Some(mut node) = world.get_mut::<Node>(entity) {
        node.overflow = overflow;
    }
    world
        .entity_mut(entity)
        .insert(bevy::ui_widgets::ScrollArea);
}

pub(super) fn refresh_capture_marker(world: &mut World, entity: Entity, primitive: &Primitive) {
    let captures_world = world.get::<Button>(entity).is_some()
        || world.get::<bevy::ui_widgets::ScrollArea>(entity).is_some()
        || world.get::<ImageNode>(entity).is_some()
        || world
            .get::<Text>(entity)
            .is_some_and(|text| !text.0.is_empty())
        || world
            .get::<BackgroundColor>(entity)
            .is_some_and(|color| color.0.to_srgba().alpha > 0.0)
        || world.get::<BorderColor>(entity).is_some()
        || matches!(primitive, Primitive::MinimapView { .. });
    if captures_world {
        world.entity_mut(entity).insert(UiCaptureRoot);
    } else {
        world.entity_mut(entity).remove::<UiCaptureRoot>();
    }
}

pub(super) fn reconcile_list_rows(world: &mut World, context: &sources::SourceContext<'_>) {
    let lists = world
        .iter_entities()
        .filter_map(|entity| {
            let binding = entity.get::<WidgetBinding>()?.clone();
            let id = entity.get::<primitives::AuthoredWidgetId>()?.0.clone();
            let (source, direction, gap, limit, template, grid) = match &binding.primitive {
                Primitive::List {
                    source,
                    direction,
                    gap,
                    max_items,
                    item,
                } => (
                    source,
                    *direction,
                    *gap,
                    usize::from(*max_items),
                    (**item).clone(),
                    None,
                ),
                Primitive::Grid {
                    source,
                    columns,
                    rows,
                    cell,
                    gap,
                    item,
                } => (
                    source,
                    turts_mod_api::Direction::Horizontal,
                    *gap,
                    usize::from(*columns).saturating_mul(usize::from(*rows)),
                    (**item).clone(),
                    Some((*columns, *cell)),
                ),
                _ => return None,
            };
            Some((
                entity.id(),
                id,
                binding.parent_size,
                binding.package,
                binding.row_tooltip,
                source.clone(),
                direction,
                gap,
                limit,
                template,
                grid,
            ))
        })
        .collect::<Vec<_>>();

    for (
        owner,
        owner_id,
        parent_size,
        package,
        row_tooltip,
        source,
        direction,
        gap,
        limit,
        template,
        grid,
    ) in lists
    {
        let data = sources::read_source(&source, *context);
        let items = data
            .items
            .into_iter()
            .filter(|item| grid_item_included(&source, grid.is_some(), item))
            .take(limit)
            .collect::<Vec<_>>();
        let existing = world
            .iter_entities()
            .filter_map(|entity| {
                let row = entity.get::<WidgetListRow>()?;
                (row.owner == owner).then_some((row.key.clone(), entity.id()))
            })
            .collect::<std::collections::HashMap<_, _>>();
        let desired_keys = items
            .iter()
            .enumerate()
            .map(|(index, item)| row_key(item, index))
            .collect::<Vec<_>>();
        for (key, entity) in &existing {
            if !desired_keys.contains(key) {
                let _ = world.despawn(*entity);
            }
        }
        let mut flow_offset = 0.0;
        for (index, item) in items.into_iter().enumerate() {
            let key = row_key(&item, index);
            let mut row_template = template.clone();
            if row_template.tooltip.is_none()
                && !matches!(&row_template.widget, Primitive::Tooltip { .. })
            {
                row_template.tooltip = row_tooltip.clone();
            }
            row_template.id = format!("{owner_id}/{}[{key}]", template.id);
            let cell_size = grid.map(|(_, cell)| Vec2::new(cell.0 as f32, cell.1 as f32));
            if let Some(cell_size) = cell_size {
                row_template.size = Size {
                    w: turts_mod_api::Dimension::Px(cell_size.x),
                    h: turts_mod_api::Dimension::Px(cell_size.y),
                };
            }
            let at = if let Some((columns, cell)) = grid {
                let columns = usize::from(columns.max(1));
                let authored_cell = item.get(ItemField::CardCell).and_then(|cell| {
                    let row = cell.get("row")?.as_u64()?;
                    let column = cell.get("column")?.as_u64()?;
                    Some((usize::try_from(column).ok()?, usize::try_from(row).ok()?))
                });
                let (x, y) = if matches!(source, turts_mod_api::Source::CommandCard) {
                    let Some(cell) = authored_cell else { continue };
                    cell
                } else {
                    authored_cell.unwrap_or((index % columns, index / columns))
                };
                turts_mod_api::At {
                    anchor: Anchor::TopLeft,
                    offset: (
                        x as f32 * (cell.0 as f32 + gap as f32),
                        y as f32 * (cell.1 as f32 + gap as f32),
                    ),
                    pivot: None,
                }
            } else {
                list_at(flow_offset, direction)
            };
            row_template.at = Some(at);
            let row_entity = if let Some(entity) = existing.get(&key).copied() {
                entity
            } else {
                let style = world
                    .get::<WidgetBinding>(owner)
                    .map(|binding| binding.tooltip_style.clone())
                    .unwrap_or(turts_mod_api::TooltipStyle {
                        background: "#000000".into(),
                        colour: "#ffffff".into(),
                        font: None,
                        size: 12,
                    });
                let entity = spawn_widget(
                    world,
                    Some(owner),
                    &row_template,
                    parent_size,
                    context,
                    &package,
                    &style,
                    Some(&item),
                );
                world.entity_mut(entity).insert(WidgetListRow {
                    owner,
                    key: key.clone(),
                    template: row_template.clone(),
                    index,
                });
                entity
            };
            update_bound_item_subtree(world, row_entity, &item);
            if let Some(mut row) = world.get_mut::<WidgetListRow>(row_entity) {
                row.index = index;
                row.template = row_template.clone();
            }
            if let Some(mut binding) = world.get_mut::<WidgetBinding>(row_entity) {
                binding.at = row_template.at.unwrap();
                binding.size = row_template.size;
                binding.parent_size = parent_size;
            }
            if let Some((node, transform)) =
                placement::authored_node(parent_size, &row_template.at.unwrap(), row_template.size)
            {
                if let Some(mut current) = world.get_mut::<Node>(row_entity) {
                    apply_node_placement(
                        &mut current,
                        node,
                        matches!(row_template.size.w, turts_mod_api::Dimension::Fit),
                        matches!(row_template.size.h, turts_mod_api::Dimension::Fit),
                    );
                }
                if let Some(mut current) = world.get_mut::<UiTransform>(row_entity) {
                    *current = transform;
                }
            }
            if grid.is_none() {
                let extent = measured_authored_size(world, row_entity)
                    .unwrap_or_else(|| placement::authored_extent(parent_size, row_template.size));
                flow_offset += match direction {
                    turts_mod_api::Direction::Horizontal
                    | turts_mod_api::Direction::Left
                    | turts_mod_api::Direction::Right => extent.x,
                    turts_mod_api::Direction::Vertical
                    | turts_mod_api::Direction::Up
                    | turts_mod_api::Direction::Down => extent.y,
                } + gap as f32;
            }
        }
    }
}

/// Update only authored geometry while keeping primitive-owned style such as borders,
/// overflow, padding, and image layout intact.
fn apply_node_placement(
    current: &mut Node,
    placement: Node,
    preserve_fit_width: bool,
    preserve_fit_height: bool,
) {
    current.position_type = placement.position_type;
    current.left = placement.left;
    current.right = placement.right;
    current.top = placement.top;
    current.bottom = placement.bottom;
    if !preserve_fit_width {
        current.width = placement.width;
    }
    if !preserve_fit_height {
        current.height = placement.height;
    }
}

pub(super) fn grid_item_included(
    source: &turts_mod_api::Source,
    is_grid: bool,
    item: &sources::ItemData,
) -> bool {
    if !is_grid || !matches!(source, turts_mod_api::Source::CommandCard) {
        return true;
    }
    item.get(ItemField::CardCell).is_some_and(|cell| {
        cell.get("row")
            .and_then(serde_json::Value::as_u64)
            .is_some()
            && cell
                .get("column")
                .and_then(serde_json::Value::as_u64)
                .is_some()
    })
}

fn measured_authored_size(world: &World, entity: Entity) -> Option<Vec2> {
    let node = world.get::<ComputedNode>(entity)?;
    let inverse_scale = node.inverse_scale_factor;
    if !inverse_scale.is_finite() || inverse_scale <= 0.0 || !node.size().is_finite() {
        return None;
    }
    Some(node.size() * inverse_scale)
}

fn row_key(item: &sources::ItemData, index: usize) -> String {
    for field in [ItemField::Action, ItemField::CancelAction] {
        if let Some(value) = item.get(field) {
            return format!("{field:?}:{value}");
        }
    }
    // Read-only rows may share a display name; identity follows stable source order.
    format!("index:{index}")
}

fn update_bound_item_subtree(world: &mut World, root: Entity, item: &sources::ItemData) {
    let mut entities = vec![root];
    let mut index = 0;
    while index < entities.len() {
        let entity = entities[index];
        // Nested list rows own their item context; the ancestor row must not overwrite it.
        if entity != root && world.get::<WidgetListRow>(entity).is_some() {
            index += 1;
            continue;
        }
        if let Some(children) = world.get::<Children>(entity) {
            entities.extend(children.iter());
        }
        if let Some(mut binding) = world.get_mut::<WidgetBinding>(entity) {
            binding.item = Some(item.clone());
        }
        index += 1;
    }
}

fn list_at(offset: f32, direction: turts_mod_api::Direction) -> turts_mod_api::At {
    match direction {
        turts_mod_api::Direction::Horizontal | turts_mod_api::Direction::Right => {
            turts_mod_api::At {
                anchor: Anchor::TopLeft,
                offset: (offset, 0.0),
                pivot: None,
            }
        }
        turts_mod_api::Direction::Left => turts_mod_api::At {
            anchor: Anchor::TopRight,
            offset: (-offset, 0.0),
            pivot: Some(Anchor::TopRight),
        },
        turts_mod_api::Direction::Vertical | turts_mod_api::Direction::Down => turts_mod_api::At {
            anchor: Anchor::TopLeft,
            offset: (0.0, offset),
            pivot: None,
        },
        turts_mod_api::Direction::Up => turts_mod_api::At {
            anchor: Anchor::BottomLeft,
            offset: (0.0, -offset),
            pivot: Some(Anchor::BottomLeft),
        },
    }
}

fn entity_insert_image(
    world: &mut World,
    entity: Entity,
    handle: Handle<Image>,
    tint: Option<&str>,
    margins: Option<(u16, u16, u16, u16)>,
) {
    let mut node = ImageNode::new(handle);
    if let Some(tint) = tint.and_then(primitives::parse_colour) {
        node.color = tint;
    }
    if let Some((left, top, right, bottom)) = margins {
        node.image_mode = NodeImageMode::Sliced(bevy::sprite::TextureSlicer {
            border: bevy::sprite::BorderRect::from([
                left as f32,
                right as f32,
                top as f32,
                bottom as f32,
            ]),
            ..default()
        });
    }
    world.entity_mut(entity).insert(node);
}
