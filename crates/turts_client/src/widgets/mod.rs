//! Mod-authored retained UI tree and the allowlisted source/primitive adapters.

pub mod assets;
pub mod cursor;
pub mod guide;
pub mod hit;
pub mod placement;
pub mod primitives;
pub mod snapshot;
pub mod sources;
pub mod tooltip;

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button, observe};
use bevy::window::PrimaryWindow;
use turts_mod_api::{
    Anchor, ButtonAction, ButtonIcon, ContentId, Fill, ItemField, Predicate, Primitive, Size,
    TextValue, Value, Widget,
};

use crate::{
    core_adapter::{CoreInputQueue, FrameCloseSet},
    debug_feed::{AlertsFeed, GroupsFeed, NoticesFeed, SelectionFeed},
    ingest::ReplicaState,
    join::{AcquiredPackages, Phase},
    ui_shared::{UiCaptureRoot, UiPresentationSet},
};

#[derive(Resource, Default)]
struct WidgetTree {
    key: String,
    entities: Vec<Entity>,
}

#[derive(Component, Clone)]
struct WidgetBinding {
    primitive: Primitive,
    at: turts_mod_api::At,
    size: Size,
    item: Option<sources::ItemData>,
    tooltip: Option<Value>,
    row_tooltip: Option<Value>,
    visible: Vec<Predicate>,
    parent_size: Vec2,
    package: String,
    tooltip_style: turts_mod_api::TooltipStyle,
}

#[derive(Component, Clone)]
struct WidgetListRow {
    owner: Entity,
    key: String,
    template: Widget,
    index: usize,
}

/// Retained widget runtime. All visible nodes and sources come from the accepted interface.
pub struct WidgetsPlugin;
impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ScrollAreaPlugin>() {
            app.add_plugins(bevy::ui_widgets::ScrollAreaPlugin);
        }
        app.init_resource::<guide::GuideOpen>()
            .init_resource::<WidgetTree>()
            .init_resource::<cursor::CursorState>()
            .init_resource::<assets::WidgetAssets>()
            .add_systems(
                Update,
                (
                    observe_guide_phase,
                    sync_widget_tree,
                    update_fit_layout,
                    update_widget_button_states,
                )
                    .chain()
                    .in_set(UiPresentationSet)
                    .after(crate::present::update_presented),
            )
            .add_systems(
                Update,
                assets::load_widget_assets
                    .run_if(resource_exists::<Assets<Image>>)
                    .run_if(resource_exists::<Assets<Font>>)
                    .before(sync_widget_tree),
            )
            .add_systems(
                Update,
                consume_shell_actions
                    .run_if(resource_exists::<crate::core_adapter::CoreEffects>)
                    .after(crate::core_adapter::CoreAdapterSet)
                    .after(observe_guide_phase)
                    .before(sync_widget_tree),
            )
            .add_systems(
                Last,
                cursor::update_cursor
                    .before(FrameCloseSet)
                    .before(bevy::window::CursorSystems::Update),
            )
            .add_systems(
                Last,
                tooltip::update_tooltips
                    .before(cursor::update_cursor)
                    .before(snapshot::publish_snapshot),
            )
            .add_systems(Last, snapshot::publish_snapshot.before(FrameCloseSet));
    }
}

fn observe_guide_phase(phase: Res<Phase>, mut guide: ResMut<guide::GuideOpen>) {
    guide::observe_phase(&phase, &mut guide);
}

fn consume_shell_actions(
    effects: Res<crate::core_adapter::CoreEffects>,
    mut guide: ResMut<guide::GuideOpen>,
) {
    if !effects.is_changed() {
        return;
    }
    for effect in &effects.0 {
        if let turts_interaction::Effect::ShellAction { action } = effect
            && action == "toggle_guide"
        {
            guide::toggle_guide(&mut guide);
        }
    }
}

/// Resolve Fit axes after the content layout has been measured at least once. Bevy measures
/// text and ordinary images intrinsically; absolute Box/List/Grid children need their measured
/// bounds folded into the container explicitly. This keeps Fit content driven by actual UI
/// layout rather than estimated glyph widths.
fn update_fit_layout(world: &mut World) {
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

        if !has_measure {
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

fn sync_widget_tree(world: &mut World) {
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
    let source_context = sources::SourceContext {
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
fn spawn_widget(
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
            if widget
                .visible
                .iter()
                .all(|predicate| predicate_matches(predicate, source_context))
            {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
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
                    world.entity_mut(id).insert((
                        Text::new(text),
                        TextFont {
                            font_size: FontSize::Px(14.0),
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
            background, frame, ..
        } => {
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

fn refresh_capture_marker(world: &mut World, entity: Entity, primitive: &Primitive) {
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

fn refresh_bound_widgets(world: &mut World, context: &sources::SourceContext<'_>) {
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
            *current = if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
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
            if world.get::<TextFont>(entity).is_none() {
                world.entity_mut(entity).insert(TextFont {
                    font_size: FontSize::Px(14.0),
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

fn reconcile_list_rows(world: &mut World, context: &sources::SourceContext<'_>) {
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

fn grid_item_included(
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

fn predicate_matches(predicate: &Predicate, context: &sources::SourceContext<'_>) -> bool {
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
            }),
        Predicate::GuideOpen => context.guide_open,
        Predicate::Not(inner) => !predicate_matches(inner, context),
    }
}

fn primitive_fill(primitive: &Primitive) -> Option<Fill> {
    match primitive {
        Primitive::Box { fill, .. } => fill.clone(),
        Primitive::MinimapView { background, .. } => background.clone(),
        _ => None,
    }
}

fn primitive_border(primitive: &Primitive) -> Option<&str> {
    match primitive {
        Primitive::Box { border, .. } => border.as_deref(),
        Primitive::MinimapView { frame, .. } => Some(frame),
        _ => None,
    }
}

fn text_value(
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

fn button_text(
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

fn numeric_value(
    value: &Value,
    context: &sources::SourceContext<'_>,
    item: Option<&sources::ItemData>,
) -> Option<f32> {
    sources::read_value_for_item(value, *context, item)
        .as_f64()
        .map(|number| number as f32)
        .filter(|number| number.is_finite())
}

fn display_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn value_uses_item(value: &Value) -> bool {
    match value {
        Value::Item(_) => true,
        Value::Format(_, arguments) => arguments.iter().any(value_uses_item),
        _ => false,
    }
}

fn resolve_icon(
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

fn resolve_action(
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

fn qualify(package: &str, id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("{package}:{id}")
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

fn update_widget_button_states(mut buttons: ButtonVisualQuery<'_, '_>) {
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

fn activate_widget(
    trigger: On<Activate>,
    actions: ButtonActionQuery<'_, '_>,
    mut queue: ResMut<CoreInputQueue>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut pending: ResMut<crate::commands::PendingCommands>,
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
                crate::commands::submit(
                    &mut pending,
                    crate::commands::ClientCommand::Produce {
                        producer_id: turts_net_protocol::UnitId(*producer),
                        recipe_id: content_id.clone(),
                    },
                );
            }
        }
        primitives::WidgetAction::CancelProduction {
            producer,
            job: job_id,
        } => {
            let Some(slot) = state.welcome.map(|welcome| welcome.slot) else {
                return;
            };
            let Some(replica) = state.replica.as_ref() else {
                return;
            };
            if replica
                .units
                .get(producer)
                .is_some_and(|unit| unit.owner_slot == slot)
                && replica.economy.queues.iter().any(|queue| {
                    queue.producer.0 == *producer
                        && queue.jobs.iter().any(|job| job.id.0 == *job_id)
                })
            {
                crate::commands::submit(
                    &mut pending,
                    crate::commands::ClientCommand::CancelProduction {
                        producer_id: turts_net_protocol::UnitId(*producer),
                        job_id: turts_net_protocol::JobId(*job_id),
                    },
                );
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn test_tooltip_style() -> turts_mod_api::TooltipStyle {
        turts_mod_api::TooltipStyle {
            background: "#000000".into(),
            colour: "#ffffff".into(),
            font: None,
            size: 12,
        }
    }

    #[test]
    fn icon_only_hotkey_button_still_renders_effective_binding_and_tracks_hover() {
        let mut world = World::new();
        let replica = crate::replica::Replica::new(20);
        let content = crate::content::ClientContent::default();
        let selection = Vec::new();
        let groups = Default::default();
        let alerts = Vec::new();
        let bindings = Default::default();
        let input_map = turts_mod_api::InputMapDef {
            actions: vec![turts_mod_api::ActionDef {
                name: "centre_selection".into(),
                default: vec![turts_mod_api::Binding::Key {
                    code: "KeyC".into(),
                    mods: Vec::new(),
                }],
            }],
            engine_overrides: Vec::new(),
        };
        let context = sources::SourceContext {
            replica: &replica,
            content: &content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &input_map,
            guide: &[],
            guide_open: false,
            last_notice: None,
        };
        let widget = Widget {
            id: "card/centre".into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Px(40.0),
                h: turts_mod_api::Dimension::Px(40.0),
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Button {
                icon: None,
                label: None,
                action: ButtonAction::Input("centre_selection".into()),
                hotkey_label: true,
            },
        };
        let entity = spawn_widget(
            &mut world,
            None,
            &widget,
            Vec2::new(100.0, 100.0),
            &context,
            "test",
            &test_tooltip_style(),
            None,
        );
        assert_eq!(world.get::<Text>(entity).unwrap().0, " [C]");
        assert_eq!(
            world.get::<bevy::picking::hover::Hovered>(entity),
            Some(&bevy::picking::hover::Hovered(false))
        );
    }

    #[test]
    fn dynamic_list_values_refresh_without_replacing_button_entities() {
        let mut world = World::new();
        let mut replica = crate::replica::Replica::new(20);
        replica.units.insert(
            1,
            crate::replica::UnitReplica {
                content_id: "test:scout".into(),
                owner_slot: 0,
                facing: None,
                samples: std::collections::VecDeque::from([(
                    1,
                    turts_net_protocol::Pos { x: 1.0, y: 1.0 },
                )]),
            },
        );
        let mut content = crate::content::ClientContent::default();
        let id = turts_mod_api::ContentId::parse("test:scout").unwrap();
        content.catalog.units.insert(
            id.clone(),
            turts_mod_api::ResolvedUnit {
                id,
                name: "Scout".into(),
                footprint: turts_mod_api::Footprint::Circle { radius: 1.0 },
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        );
        let initial_content = content.clone();
        let selection = [1];
        let groups = Default::default();
        let alerts = Vec::new();
        let bindings = Default::default();
        let input_map = turts_mod_api::InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        };
        let context = sources::SourceContext {
            replica: &replica,
            content: &initial_content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &input_map,
            guide: &[],
            guide_open: false,
            last_notice: None,
        };
        assert_eq!(
            text_value(
                &TextValue::Format(
                    "value {0} then {1}".into(),
                    vec![Value::Literal("{1}".into()), Value::Literal("done".into())]
                ),
                &context,
                None
            ),
            "value {1} then done",
            "renderer formatting must not rescan inserted placeholder text",
        );
        let row = Widget {
            id: "row".into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Px(80.0),
                h: turts_mod_api::Dimension::Px(20.0),
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Box {
                fill: None,
                border: Some("#abcdef".into()),
                children: vec![Widget {
                    id: "row.button".into(),
                    at: None,
                    size: Size {
                        w: turts_mod_api::Dimension::Px(80.0),
                        h: turts_mod_api::Dimension::Px(20.0),
                    },
                    layer: 0,
                    visible: Vec::new(),
                    tooltip: None,
                    widget: Primitive::Button {
                        icon: None,
                        label: Some(TextValue::Item(ItemField::Name)),
                        action: ButtonAction::Item(turts_mod_api::ActionField::Action),
                        hotkey_label: false,
                    },
                }],
            },
        };
        let list = Widget {
            id: "units".into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Px(100.0),
                h: turts_mod_api::Dimension::Px(100.0),
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::List {
                source: turts_mod_api::Source::SelectionUnits,
                direction: turts_mod_api::Direction::Vertical,
                gap: 2,
                max_items: 4,
                item: Box::new(row),
            },
        };
        let root = spawn_widget(
            &mut world,
            None,
            &list,
            Vec2::new(100.0, 100.0),
            &context,
            "test",
            &test_tooltip_style(),
            None,
        );
        refresh_bound_widgets(&mut world, &context);
        let row_entity = world
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<primitives::AuthoredWidgetId>()
                    .is_some_and(|id| id.0.starts_with("units/row["))
            })
            .unwrap()
            .id();
        let row_id = world.get::<Children>(row_entity).unwrap()[0];
        assert_eq!(
            world.get::<Node>(row_entity).unwrap().border,
            UiRect::all(placement::pixel(1.0)),
            "the authored Box row keeps its border after initial placement"
        );
        world.get_mut::<Node>(row_entity).unwrap().overflow = Overflow::scroll();
        assert_eq!(world.get::<Text>(row_id).unwrap().0, "Scout");
        assert_eq!(
            world
                .get::<primitives::WidgetButtonAction>(row_id)
                .unwrap()
                .0,
            primitives::WidgetAction::Select { unit: 1 }
        );

        content
            .catalog
            .units
            .get_mut(&turts_mod_api::ContentId::parse("test:scout").unwrap())
            .unwrap()
            .name = "Scout II".into();
        let updated_context = sources::SourceContext {
            content: &content,
            ..context
        };
        refresh_bound_widgets(&mut world, &updated_context);
        let updated_row = world
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<primitives::AuthoredWidgetId>()
                    .is_some_and(|id| id.0.starts_with("units/row["))
            })
            .unwrap()
            .id();
        assert_eq!(
            updated_row, row_entity,
            "the real row entity survives changing source values"
        );
        let updated_button = world.get::<Children>(updated_row).unwrap()[0];
        assert_eq!(updated_button, row_id);
        assert_eq!(world.get::<Text>(updated_button).unwrap().0, "Scout II");
        assert_eq!(
            world.get::<Node>(updated_row).unwrap().border,
            UiRect::all(placement::pixel(1.0)),
            "reconciliation updates geometry without replacing the Box border"
        );
        assert_eq!(
            world.get::<Node>(updated_row).unwrap().overflow,
            Overflow::scroll(),
            "reconciliation preserves the row's existing overflow style"
        );
        assert!(
            world
                .get::<Children>(root)
                .is_some_and(|children| children.contains(&updated_row))
        );
    }

    #[test]
    fn text_that_appears_after_an_empty_binding_keeps_authored_typography() {
        let mut world = World::new();
        world.insert_resource(assets::WidgetAssets::default());
        let replica = crate::replica::Replica::new(20);
        let content = crate::content::ClientContent::default();
        let selection = Vec::new();
        let groups = BTreeMap::new();
        let alerts = Vec::new();
        let bindings = BTreeMap::new();
        let input_map = turts_mod_api::InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        };
        let context = sources::SourceContext {
            replica: &replica,
            content: &content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &input_map,
            guide: &[],
            guide_open: false,
            last_notice: None,
        };
        let text = Widget {
            id: "notice".into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Fit,
                h: turts_mod_api::Dimension::Fit,
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Text {
                text: TextValue::Bind(Value::LastNotice),
                font: None,
                size: 21,
                colour: "#abcdef".into(),
                align: turts_mod_api::TextAlign::Centre,
                wrap: true,
            },
        };
        let entity = spawn_widget(
            &mut world,
            None,
            &text,
            Vec2::new(300.0, 120.0),
            &context,
            "test",
            &test_tooltip_style(),
            None,
        );
        refresh_bound_widgets(&mut world, &context);
        assert!(world.get::<Text>(entity).is_none());

        let updated_context = sources::SourceContext {
            last_notice: Some("New alert"),
            ..context
        };
        refresh_bound_widgets(&mut world, &updated_context);
        assert_eq!(world.get::<Text>(entity).unwrap().0, "New alert");
        assert_eq!(
            world.get::<TextFont>(entity).unwrap().font_size,
            FontSize::Px(21.0)
        );
        assert_eq!(
            world.get::<TextColor>(entity).unwrap().0,
            primitives::parse_colour("#abcdef").unwrap()
        );
        assert!(world.get::<TextLayout>(entity).is_some());
    }

    #[test]
    fn grid_item_tooltips_inherit_context_and_template_tooltips_override_it() {
        let mut replica = crate::replica::Replica::new(20);
        replica.units.insert(
            1,
            crate::replica::UnitReplica {
                content_id: "test:worker".into(),
                owner_slot: 0,
                facing: None,
                samples: Default::default(),
            },
        );
        let worker_id = turts_mod_api::ContentId::parse("test:worker").unwrap();
        let mut content = crate::content::ClientContent::default();
        content.catalog.units.insert(
            worker_id.clone(),
            turts_mod_api::ResolvedUnit {
                id: worker_id.clone(),
                name: "Worker".into(),
                footprint: turts_mod_api::Footprint::Circle { radius: 1.0 },
                movement: Some(turts_mod_api::MovementDef { speed: 2.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        );
        let move_id = turts_mod_api::ContentId::parse("test:move").unwrap();
        let attack_id = turts_mod_api::ContentId::parse("test:attack").unwrap();
        let order = |id: &str,
                     name: &str,
                     behaviour: turts_mod_api::Behaviour,
                     target: turts_mod_api::TargetKind,
                     hotkey: &str,
                     description: &str,
                     column: u8| turts_mod_api::OrderDef {
            id: id.into(),
            name: name.into(),
            behaviour,
            target,
            fire: None,
            engage: None,
            icon: String::new(),
            hotkey: hotkey.into(),
            cursor: None,
            card: Some(turts_mod_api::CardCell { row: 0, column }),
            units: vec![worker_id.to_string()],
            description: description.into(),
        };
        content.catalog.orders.insert(
            move_id.clone(),
            order(
                "test:move",
                "Move",
                turts_mod_api::Behaviour::Move,
                turts_mod_api::TargetKind::Point,
                "key_move",
                "Move description",
                0,
            ),
        );
        content.catalog.orders.insert(
            attack_id.clone(),
            order(
                "test:attack",
                "Attack",
                turts_mod_api::Behaviour::Attack,
                turts_mod_api::TargetKind::Unit,
                "key_attack",
                "Attack description",
                1,
            ),
        );
        content.input_map.actions = vec![
            turts_mod_api::ActionDef {
                name: "key_move".into(),
                default: vec![turts_mod_api::Binding::Key {
                    code: "KeyM".into(),
                    mods: Vec::new(),
                }],
            },
            turts_mod_api::ActionDef {
                name: "key_attack".into(),
                default: vec![turts_mod_api::Binding::Key {
                    code: "KeyA".into(),
                    mods: Vec::new(),
                }],
            },
        ];
        content.order_rules.smart = vec![
            turts_mod_api::SmartOrder {
                target: turts_mod_api::SmartTarget::Ground,
                needs: turts_mod_api::Capability::Movement,
                order: "test:move".into(),
            },
            turts_mod_api::SmartOrder {
                target: turts_mod_api::SmartTarget::ForeignUnit,
                needs: turts_mod_api::Capability::Weapon,
                order: "test:attack".into(),
            },
        ];
        let selection = [1];
        let groups = Default::default();
        let alerts = Vec::new();
        let bindings = Default::default();
        let context = sources::SourceContext {
            replica: &replica,
            content: &content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &content.input_map,
            guide: &[],
            guide_open: false,
            last_notice: None,
        };
        let inherited = Value::Format(
            "{0} | {1} | {2} | {3}".into(),
            vec![
                Value::Item(ItemField::Name),
                Value::Item(ItemField::Title),
                Value::Item(ItemField::Body),
                Value::Item(ItemField::Reason),
            ],
        );
        let make_grid = |id: &str, template_tooltip: Option<Value>| Widget {
            id: id.into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Px(130.0),
                h: turts_mod_api::Dimension::Px(36.0),
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: Some(inherited.clone()),
            widget: Primitive::Grid {
                source: turts_mod_api::Source::CommandCard,
                columns: 2,
                rows: 1,
                cell: (64, 32),
                gap: 2,
                item: Box::new(Widget {
                    id: "order".into(),
                    at: None,
                    size: Size {
                        w: turts_mod_api::Dimension::Fill,
                        h: turts_mod_api::Dimension::Fill,
                    },
                    layer: 0,
                    visible: Vec::new(),
                    tooltip: template_tooltip,
                    widget: Primitive::Button {
                        icon: None,
                        label: Some(TextValue::Item(ItemField::Name)),
                        action: ButtonAction::Item(turts_mod_api::ActionField::Action),
                        hotkey_label: false,
                    },
                }),
            },
        };

        let mut world = World::new();
        let grid = spawn_widget(
            &mut world,
            None,
            &make_grid("card", None),
            Vec2::new(200.0, 100.0),
            &context,
            "test",
            &test_tooltip_style(),
            None,
        );
        refresh_bound_widgets(&mut world, &context);
        assert!(world.get::<primitives::WidgetTooltip>(grid).is_none());
        let mut rows = world
            .iter_entities()
            .filter_map(|entity| {
                let action = entity.get::<primitives::WidgetButtonAction>()?;
                Some((action.0.clone(), entity.id()))
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|(action, _)| format!("{action:?}"));
        assert_eq!(rows.len(), 2);
        let move_button = rows
            .iter()
            .find(|(action, _)| {
                action
                    == &primitives::WidgetAction::Order {
                        id: "test:move".into(),
                    }
            })
            .unwrap()
            .1;
        let attack_button = rows
            .iter()
            .find(|(action, _)| {
                action
                    == &primitives::WidgetAction::Order {
                        id: "test:attack".into(),
                    }
            })
            .unwrap()
            .1;
        assert_eq!(
            world
                .get::<primitives::WidgetTooltip>(move_button)
                .unwrap()
                .0,
            "Move | M | Move description | "
        );
        assert_eq!(
            world
                .get::<primitives::WidgetTooltip>(attack_button)
                .unwrap()
                .0,
            "Attack | A | Attack description | Requires a selected unit with this capability"
        );
        assert_eq!(
            world.get::<primitives::WidgetEnabled>(attack_button),
            Some(&primitives::WidgetEnabled(false))
        );

        let override_grid = spawn_widget(
            &mut world,
            None,
            &make_grid("override_card", Some(Value::Item(ItemField::Name))),
            Vec2::new(200.0, 100.0),
            &context,
            "test",
            &test_tooltip_style(),
            None,
        );
        refresh_bound_widgets(&mut world, &context);
        let override_button = world
            .iter_entities()
            .find(|entity| {
                entity.contains::<Button>()
                    && entity
                        .get::<primitives::AuthoredWidgetId>()
                        .is_some_and(|id| {
                            id.0.starts_with("override_card/") && id.0.contains("test:move")
                        })
            })
            .unwrap()
            .id();
        assert_eq!(
            world
                .get::<primitives::WidgetTooltip>(override_button)
                .unwrap()
                .0,
            "Move"
        );
        assert!(
            world
                .get::<primitives::WidgetTooltip>(override_grid)
                .is_none()
        );
    }

    #[test]
    fn disabled_authored_button_cannot_dispatch_an_action() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WidgetsPlugin))
            .init_resource::<CoreInputQueue>()
            .init_resource::<crate::ingest::ReplicaState>()
            .init_resource::<crate::commands::PendingCommands>();
        let replica = crate::replica::Replica::new(20);
        let content = crate::content::ClientContent::default();
        let selection = Vec::new();
        let groups = Default::default();
        let alerts = Vec::new();
        let bindings = Default::default();
        let input_map = turts_mod_api::InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        };
        let context = sources::SourceContext {
            replica: &replica,
            content: &content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &input_map,
            guide: &[],
            guide_open: false,
            last_notice: None,
        };
        let item = sources::ItemData {
            fields: vec![(ItemField::Enabled, serde_json::json!(false))],
        };
        let button = Widget {
            id: "test.disabled".into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Px(80.0),
                h: turts_mod_api::Dimension::Px(24.0),
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Button {
                icon: None,
                label: Some(TextValue::Literal("Disabled".into())),
                action: ButtonAction::Input("toggle_guide".into()),
                hotkey_label: false,
            },
        };
        let entity = spawn_widget(
            app.world_mut(),
            None,
            &button,
            Vec2::new(100.0, 100.0),
            &context,
            "test",
            &test_tooltip_style(),
            Some(&item),
        );
        refresh_bound_widgets(app.world_mut(), &context);
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(entity)
                .is_some()
        );
        app.world_mut().trigger(Activate { entity });
        assert!(app.world().resource::<CoreInputQueue>().0.is_empty());
        assert!(!app.world().resource::<guide::GuideOpen>().is_open);
    }

    #[test]
    fn fit_text_and_image_use_real_asset_components_with_bevy_auto_sizing() {
        let mut world = World::new();
        let mut image_store = Assets::<Image>::default();
        let image_handle = image_store.add(Image::default());
        let mut font_store = Assets::<Font>::default();
        let font_handle = font_store.add(Font::from_bytes(
            include_bytes!("../../../../mods/muster/assets/fonts/Roboto-Regular.ttf").to_vec(),
        ));
        let mut widget_assets = assets::WidgetAssets::default();
        widget_assets
            .images
            .insert("test:icon".into(), image_handle.clone());
        widget_assets
            .fonts
            .insert("test:font".into(), font_handle.clone());
        widget_assets
            .nine_slices
            .insert("test:icon".into(), (2, 3, 4, 5));
        world.insert_resource(widget_assets);
        world.insert_resource(image_store);

        let replica = crate::replica::Replica::new(20);
        let content = crate::content::ClientContent::default();
        let selection = Vec::new();
        let groups = Default::default();
        let alerts = Vec::new();
        let bindings = Default::default();
        let input_map = turts_mod_api::InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        };
        let context = sources::SourceContext {
            replica: &replica,
            content: &content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &input_map,
            guide: &[],
            guide_open: false,
            last_notice: None,
        };
        let fit = Size {
            w: turts_mod_api::Dimension::Fit,
            h: turts_mod_api::Dimension::Fit,
        };
        let text = Widget {
            id: "fit.text".into(),
            at: None,
            size: fit,
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Text {
                text: TextValue::Literal("Measured by the installed font".into()),
                font: Some("font".into()),
                size: 19,
                colour: "#ffffff".into(),
                align: turts_mod_api::TextAlign::Left,
                wrap: false,
            },
        };
        let image = Widget {
            id: "fit.image".into(),
            at: None,
            size: fit,
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Image {
                image: ButtonIcon::Asset("icon".into()),
                nine_slice: None,
                tint: None,
            },
        };
        let style = test_tooltip_style();
        let text_entity = spawn_widget(
            &mut world,
            None,
            &text,
            Vec2::new(500.0, 300.0),
            &context,
            "test",
            &style,
            None,
        );
        let image_entity = spawn_widget(
            &mut world,
            None,
            &image,
            Vec2::new(500.0, 300.0),
            &context,
            "test",
            &style,
            None,
        );
        for entity in [text_entity, image_entity] {
            let node = world.get::<Node>(entity).unwrap();
            assert_eq!(node.width, Val::Auto);
            assert_eq!(node.height, Val::Auto);
        }
        assert_eq!(
            world.get::<TextFont>(text_entity).unwrap().font,
            FontSource::Handle(font_handle)
        );
        assert_eq!(
            world.get::<ImageNode>(image_entity).unwrap().image,
            image_handle
        );

        let sliced = Widget {
            id: "fit.sliced-image".into(),
            at: None,
            size: fit,
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Image {
                image: ButtonIcon::Asset("icon".into()),
                nine_slice: Some((1, 1, 1, 1)),
                tint: None,
            },
        };
        let sliced_entity = spawn_widget(
            &mut world,
            None,
            &sliced,
            Vec2::new(500.0, 300.0),
            &context,
            "test",
            &style,
            None,
        );
        assert!(matches!(
            world.get::<ImageNode>(sliced_entity).unwrap().image_mode,
            NodeImageMode::Sliced(_)
        ));
        update_fit_layout(&mut world);
        assert_eq!(
            world.get::<Node>(sliced_entity).unwrap().width,
            placement::pixel(1.0),
            "Fit for Sliced images uses actual retained image dimensions"
        );

        let bar = Widget {
            id: "fit.bar".into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Px(120.0),
                h: turts_mod_api::Dimension::Px(12.0),
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Bar {
                value: Value::Literal("3".into()),
                max: Value::Literal("4".into()),
                fill: Fill::Image {
                    id: "icon".into(),
                    nine_slice: None,
                },
                background: Fill::Image {
                    id: "icon".into(),
                    nine_slice: None,
                },
                direction: turts_mod_api::Direction::Horizontal,
            },
        };
        let bar_entity = spawn_widget(
            &mut world,
            None,
            &bar,
            Vec2::new(500.0, 300.0),
            &context,
            "test",
            &style,
            None,
        );
        let background = world.get::<ImageNode>(bar_entity).unwrap();
        assert_eq!(background.image, image_handle);
        assert!(matches!(background.image_mode, NodeImageMode::Sliced(_)));
        let fill = world
            .get::<Children>(bar_entity)
            .unwrap()
            .iter()
            .find_map(|child| world.get::<ImageNode>(child))
            .expect("Bar image fill keeps its own rendered image");
        assert_eq!(fill.image, image_handle);
        assert!(matches!(fill.image_mode, NodeImageMode::Sliced(_)));

        let measured_child = Widget {
            id: "fit.measured-child".into(),
            at: Some(turts_mod_api::At {
                anchor: Anchor::BottomRight,
                offset: (-4.0, -3.0),
                pivot: Some(Anchor::BottomRight),
            }),
            size: fit,
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Text {
                text: TextValue::Literal("measured child".into()),
                font: Some("font".into()),
                size: 19,
                colour: "#ffffff".into(),
                align: turts_mod_api::TextAlign::Left,
                wrap: false,
            },
        };
        let containers = [
            Primitive::Box {
                fill: None,
                border: None,
                children: vec![measured_child.clone()],
            },
            Primitive::List {
                source: turts_mod_api::Source::Balances,
                direction: turts_mod_api::Direction::Vertical,
                gap: 0,
                max_items: 1,
                item: Box::new(measured_child.clone()),
            },
            Primitive::Grid {
                source: turts_mod_api::Source::Balances,
                columns: 1,
                rows: 1,
                cell: (1, 1),
                gap: 0,
                item: Box::new(measured_child.clone()),
            },
        ];
        for (index, primitive) in containers.into_iter().enumerate() {
            let container = Widget {
                id: format!("fit.container.{index}"),
                at: None,
                size: fit,
                layer: 0,
                visible: Vec::new(),
                tooltip: None,
                widget: primitive.clone(),
            };
            let parent = spawn_widget(
                &mut world,
                None,
                &container,
                Vec2::new(500.0, 300.0),
                &context,
                "test",
                &style,
                None,
            );
            if !matches!(primitive, Primitive::Box { .. }) {
                let child = spawn_widget(
                    &mut world,
                    Some(parent),
                    &measured_child,
                    Vec2::new(500.0, 300.0),
                    &context,
                    "test",
                    &style,
                    None,
                );
                world.entity_mut(child).insert(ComputedNode {
                    size: Vec2::new(80.0, 40.0),
                    inverse_scale_factor: 0.5,
                    ..default()
                });
            } else {
                let child = world.get::<Children>(parent).unwrap()[0];
                world.entity_mut(child).insert(ComputedNode {
                    size: Vec2::new(80.0, 40.0),
                    inverse_scale_factor: 0.5,
                    ..default()
                });
            }
            let child = world.get::<Children>(parent).unwrap()[0];
            if !matches!(primitive, Primitive::Box { .. })
                && let Some(mut node) = world.get_mut::<Node>(child)
            {
                node.border = UiRect::all(placement::pixel(3.0));
                node.overflow = Overflow::scroll();
            }
            update_fit_layout(&mut world);
            let node = world.get::<Node>(parent).unwrap();
            assert_eq!(
                node.width,
                placement::pixel(44.0),
                "Fit {index} width uses the measured child and bottom-right inset"
            );
            assert_eq!(node.height, placement::pixel(23.0));
            let child = world.get::<Children>(parent).unwrap()[0];
            assert_eq!(
                world.get::<Node>(child).unwrap().left,
                placement::pixel(40.0),
                "the anchored child is repositioned relative to the resolved Fit width"
            );
            assert_eq!(
                world.get::<Node>(child).unwrap().top,
                placement::pixel(20.0)
            );
            if !matches!(primitive, Primitive::Box { .. }) {
                let node = world.get::<Node>(child).unwrap();
                assert_eq!(node.border, UiRect::all(placement::pixel(3.0)));
                assert_eq!(node.overflow, Overflow::scroll());
            }
            world.entity_mut(child).insert(ComputedNode {
                size: Vec2::new(120.0, 60.0),
                inverse_scale_factor: 0.5,
                ..default()
            });
            update_fit_layout(&mut world);
            assert_eq!(
                world.get::<Node>(parent).unwrap().width,
                placement::pixel(64.0),
                "Fit containers follow later measured content changes"
            );
            assert_eq!(
                world.get::<Node>(child).unwrap().left,
                placement::pixel(60.0),
                "the anchored child follows the updated natural container size"
            );
        }
    }

    #[test]
    fn normal_hovered_pressed_and_disabled_buttons_receive_distinct_states() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, update_widget_button_states);
        let normal = app
            .world_mut()
            .spawn((
                Button,
                primitives::WidgetEnabled(true),
                BackgroundColor(Color::NONE),
            ))
            .id();
        let hovered = app
            .world_mut()
            .spawn((
                Button,
                primitives::WidgetEnabled(true),
                bevy::picking::hover::Hovered(true),
                BackgroundColor(Color::NONE),
            ))
            .id();
        let pressed = app
            .world_mut()
            .spawn((
                Button,
                primitives::WidgetEnabled(true),
                bevy::ui::Pressed,
                BackgroundColor(Color::NONE),
            ))
            .id();
        let disabled = app
            .world_mut()
            .spawn((
                Button,
                primitives::WidgetEnabled(false),
                BackgroundColor(Color::NONE),
            ))
            .id();
        app.update();
        let colour = |entity| app.world().get::<BackgroundColor>(entity).unwrap().0;
        assert_eq!(colour(normal), Color::srgb(0.13, 0.26, 0.30));
        assert_eq!(colour(hovered), Color::srgb(0.18, 0.40, 0.44));
        assert_eq!(colour(pressed), Color::srgb(0.10, 0.30, 0.36));
        assert_eq!(colour(disabled), Color::srgb(0.20, 0.20, 0.20));
    }

    #[test]
    fn bar_refresh_tracks_bound_progress_without_replacing_fill_entity() {
        let mut world = World::new();
        let replica = crate::replica::Replica::new(20);
        let content = crate::content::ClientContent::default();
        let selection = Vec::new();
        let groups = Default::default();
        let alerts = Vec::new();
        let bindings = Default::default();
        let input_map = turts_mod_api::InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        };
        let context = sources::SourceContext {
            replica: &replica,
            content: &content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &input_map,
            guide: &[],
            guide_open: false,
            last_notice: None,
        };
        let item = sources::ItemData {
            fields: vec![
                (ItemField::Progress, serde_json::json!(20.0)),
                (ItemField::Count, serde_json::json!(100.0)),
            ],
        };
        let bar = Widget {
            id: "progress".into(),
            at: None,
            size: Size {
                w: turts_mod_api::Dimension::Px(100.0),
                h: turts_mod_api::Dimension::Px(8.0),
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: Primitive::Bar {
                value: Value::Item(ItemField::Progress),
                max: Value::Item(ItemField::Count),
                fill: Fill::Colour("#00ff00".into()),
                background: Fill::Colour("#000000".into()),
                direction: turts_mod_api::Direction::Horizontal,
            },
        };
        let root = spawn_widget(
            &mut world,
            None,
            &bar,
            Vec2::new(100.0, 8.0),
            &context,
            "test",
            &test_tooltip_style(),
            Some(&item),
        );
        let fill = world
            .get::<Children>(root)
            .unwrap()
            .first()
            .copied()
            .unwrap();
        let action_identity = fill;
        refresh_bound_widgets(&mut world, &context);
        assert_eq!(
            world.get::<Node>(fill).unwrap().width,
            placement::percent(20.0)
        );
        let updated = sources::ItemData {
            fields: vec![
                (ItemField::Progress, serde_json::json!(80.0)),
                (ItemField::Count, serde_json::json!(100.0)),
            ],
        };
        world.get_mut::<WidgetBinding>(root).unwrap().item = Some(updated);
        refresh_bound_widgets(&mut world, &context);
        assert_eq!(
            world.get::<Children>(root).unwrap().first().copied(),
            Some(action_identity)
        );
        assert_eq!(
            world.get::<Node>(fill).unwrap().width,
            placement::percent(80.0)
        );
    }

    #[test]
    fn command_card_grid_requires_authored_cells_but_generic_grid_can_flow() {
        let command_card = turts_mod_api::Source::CommandCard;
        let located = sources::ItemData {
            fields: vec![(
                ItemField::CardCell,
                serde_json::json!({ "row": 1, "column": 2 }),
            )],
        };
        let unplaced = sources::ItemData::default();
        assert!(grid_item_included(&command_card, true, &located));
        assert!(
            !grid_item_included(&command_card, true, &unplaced),
            "an unplaced action does not claim a reserved card cell"
        );
        assert!(
            grid_item_included(&command_card, false, &unplaced),
            "the same source can still drive an authored list"
        );
        assert!(
            grid_item_included(&turts_mod_api::Source::Balances, true, &unplaced),
            "generic grids retain sequential flow"
        );
    }
}
