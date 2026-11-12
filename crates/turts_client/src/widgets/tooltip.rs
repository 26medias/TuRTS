//! Authored, styled hover tooltips. Tooltip text and style come from the accepted interface.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use turts_mod_api::TooltipStyle;

use super::{
    assets::WidgetAssets,
    placement,
    primitives::{AuthoredWidgetId, WidgetTooltip, parse_colour},
};

#[derive(Component)]
struct TooltipView {
    owner: Entity,
}

#[derive(Component, Default)]
struct TooltipOffset(Vec2);

/// The font of `tooltip_style`, resolved through the widget assets of `package`. Tooltips and
/// button labels use it. When the mod gives no font, this is the Bevy default font.
pub(super) fn style_font(world: &World, style: &TooltipStyle, package: &str) -> Handle<Font> {
    style
        .font
        .as_deref()
        .and_then(|font| {
            world
                .get_resource::<WidgetAssets>()?
                .font(&super::qualify(package, font))
        })
        .unwrap_or_default()
}

pub fn spawn_tooltip_view(
    world: &mut World,
    owner: Entity,
    text: &str,
    style: &TooltipStyle,
    package: &str,
) {
    let font = style_font(world, style, package);
    let owner_id = world
        .get::<AuthoredWidgetId>(owner)
        .map_or_else(|| format!("entity-{}", owner.index()), |id| id.0.clone());
    world.spawn((
        TooltipView { owner },
        AuthoredWidgetId(format!("{owner_id}::tooltip")),
        TooltipOffset::default(),
        crate::ui_shared::UiCaptureRoot,
        Node {
            position_type: PositionType::Absolute,
            left: placement::pixel(0.0),
            top: placement::percent(100.0),
            width: Val::Auto,
            height: Val::Auto,
            padding: placement::tooltip_padding(),
            ..default()
        },
        ZIndex(i32::MAX),
        Text::new(text.to_owned()),
        TextFont {
            font_size: FontSize::Px(style.size as f32),
            font: FontSource::Handle(font),
            ..default()
        },
        TextColor(parse_colour(&style.colour).unwrap_or(Color::WHITE)),
        TextLayout::new(Justify::Left, LineBreak::WordBoundary),
        BackgroundColor(parse_colour(&style.background).unwrap_or(Color::BLACK)),
        Visibility::Hidden,
    ));
}

/// Place actual tooltip nodes against their target camera viewport after UI layout, and show
/// them only while their actual hovered owner is visible.
pub fn update_tooltips(world: &mut World) {
    let views = world
        .iter_entities()
        .filter_map(|entity| {
            let view = entity.get::<TooltipView>()?;
            Some((entity.id(), view.owner))
        })
        .collect::<Vec<_>>();
    for (view, owner) in views {
        let Some(text) = world
            .get::<WidgetTooltip>(owner)
            .map(|tooltip| tooltip.0.clone())
        else {
            let _ = world.despawn(view);
            continue;
        };
        let visible = world.get::<Hovered>(owner).is_some_and(|hovered| hovered.0)
            && world
                .get::<InheritedVisibility>(owner)
                .is_none_or(|visibility| visibility.get());
        if let Some(mut current) = world.get_mut::<Text>(view) {
            current.0 = text;
        }
        if let Some(mut visibility) = world.get_mut::<Visibility>(view) {
            *visibility = if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        if visible {
            clamp_to_viewport(world, owner, view);
        }
    }
}

pub fn clear_tooltips(world: &mut World) {
    let views = world
        .iter_entities()
        .filter(|entity| entity.contains::<TooltipView>())
        .map(|entity| entity.id())
        .collect::<Vec<_>>();
    for view in views {
        let _ = world.despawn(view);
    }
}

fn clamp_to_viewport(world: &mut World, owner: Entity, view: Entity) {
    let Some((owner_rect, camera_id)) = world.get::<ComputedNode>(owner).and_then(|owner_node| {
        let transform = world.get::<UiGlobalTransform>(owner)?;
        let camera_id = world.get::<ComputedUiTargetCamera>(owner)?.get()?;
        let center = transform.to_scale_angle_translation().2;
        Some((Rect::from_center_size(center, owner_node.size()), camera_id))
    }) else {
        return;
    };
    let Some((tip_size, tip_rect)) = world.get::<ComputedNode>(view).and_then(|node| {
        let transform = world.get::<UiGlobalTransform>(view)?;
        let center = transform.to_scale_angle_translation().2;
        Some((node.size(), Rect::from_center_size(center, node.size())))
    }) else {
        return;
    };
    let Some(camera) = world.get::<Camera>(camera_id) else {
        return;
    };
    let Some(viewport) = camera.physical_viewport_rect() else {
        return;
    };
    let Some(device_scale) = camera.target_scaling_factor() else {
        return;
    };
    let ui_scale = world
        .get_resource::<bevy::ui::UiScale>()
        .map_or(1.0, |scale| scale.0);
    let scale = device_scale * ui_scale;
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let viewport_size = viewport.size().as_vec2();
    let target = clamped_top_left(owner_rect, tip_size, viewport_size);
    let current_offset = world
        .get::<TooltipOffset>(view)
        .map_or(Vec2::ZERO, |offset| offset.0);
    let base_top_left = tip_rect.min - current_offset;
    let offset = target - base_top_left;

    if let Some(mut node) = world.get_mut::<Node>(view) {
        node.left = placement::physical_to_pixel(target.x, scale);
        node.top = placement::physical_to_pixel(target.y, scale);
        node.max_width = placement::physical_to_pixel(viewport_size.x, scale);
    }
    world.entity_mut(view).insert(UiTargetCamera(camera_id));
    if let Some(mut previous) = world.get_mut::<TooltipOffset>(view) {
        previous.0 = offset;
    }
}

fn clamped_top_left(owner: Rect, tooltip_size: Vec2, viewport_size: Vec2) -> Vec2 {
    let max = (viewport_size - tooltip_size).max(Vec2::ZERO);
    let mut x = owner.min.x;
    if x + tooltip_size.x > viewport_size.x {
        x = owner.max.x - tooltip_size.x;
    }
    x = x.clamp(0.0, max.x);

    let mut y = owner.max.y;
    if y + tooltip_size.y > viewport_size.y {
        let above = owner.min.y - tooltip_size.y;
        y = if above >= 0.0 {
            above
        } else {
            viewport_size.y - tooltip_size.y
        };
    }
    y = y.clamp(0.0, max.y);
    Vec2::new(x, y)
}

pub fn tooltip_text(world: &World, entity: Entity) -> Option<String> {
    world
        .get::<WidgetTooltip>(entity)
        .map(|tooltip| tooltip.0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_tooltip_renders_with_interface_style_only_while_hovered() {
        let mut world = World::new();
        let owner = world
            .spawn((
                AuthoredWidgetId("test.owner".into()),
                WidgetTooltip("Ore: 12".into()),
                Hovered(false),
            ))
            .id();
        let style = TooltipStyle {
            background: "#102030".into(),
            colour: "#f0e0d0".into(),
            font: None,
            size: 17,
        };
        spawn_tooltip_view(&mut world, owner, "Ore: 12", &style, "test");
        let view = world
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<AuthoredWidgetId>()
                    .is_some_and(|id| id.0 == "test.owner::tooltip")
            })
            .unwrap()
            .id();
        assert_eq!(world.get::<Text>(view).unwrap().0, "Ore: 12");
        assert_eq!(
            world.get::<TextFont>(view).unwrap().font_size,
            FontSize::Px(17.0)
        );
        assert_eq!(
            world.get::<BackgroundColor>(view).unwrap().0,
            parse_colour("#102030").unwrap()
        );
        assert_eq!(
            world.get::<TextColor>(view).unwrap().0,
            parse_colour("#f0e0d0").unwrap()
        );
        assert_eq!(world.get::<Visibility>(view), Some(&Visibility::Hidden));

        update_tooltips(&mut world);
        assert_eq!(world.get::<Visibility>(view), Some(&Visibility::Hidden));
        world.entity_mut(owner).insert(Hovered(false));
        update_tooltips(&mut world);
        assert_eq!(world.get::<Visibility>(view), Some(&Visibility::Hidden));
        world.entity_mut(owner).insert(Hovered(true));
        world
            .entity_mut(owner)
            .get_mut::<WidgetTooltip>()
            .unwrap()
            .0 = "Ore: 14".into();
        update_tooltips(&mut world);
        assert_eq!(world.get::<Visibility>(view), Some(&Visibility::Visible));
        assert_eq!(world.get::<Text>(view).unwrap().0, "Ore: 14");
    }

    #[test]
    fn tooltip_flips_above_and_clamps_against_the_viewport_edges() {
        let owner = Rect::from_corners(Vec2::new(1240.0, 680.0), Vec2::new(1280.0, 720.0));
        let point = clamped_top_left(owner, Vec2::new(160.0, 48.0), Vec2::new(1280.0, 720.0));
        assert_eq!(point, Vec2::new(1120.0, 632.0));
    }
}
