//! Published view of actual, post-layout authored widgets.

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};

use super::primitives::{AuthoredWidgetId, WidgetButtonAction, WidgetEnabled};
use crate::visuals::WorldTooltip;
use crate::{debug_feed::UiSnapshotFeed, ui_shared::UiCaptureRoot};

/// Build the bridge rows from computed Bevy UI geometry, never from the authored declaration.
pub fn snapshot(world: &World) -> Value {
    let mut rows = Vec::new();
    for entity in world.iter_entities() {
        if entity.contains::<crate::client_shell::ClientNotice>() {
            let (Some(node), Some(transform)) = (
                entity.get::<ComputedNode>(),
                entity.get::<UiGlobalTransform>(),
            ) else {
                rows.push(json!({"kind":"client_notice","id":"client_notice","rect":null,"visible":false,"text":entity.get::<Text>().map(|text| text.0.as_str())}));
                continue;
            };
            let center = transform.to_scale_angle_translation().2;
            let size = node.size();
            let rect = [
                center.x - size.x / 2.0,
                center.y - size.y / 2.0,
                size.x,
                size.y,
            ];
            rows.push(json!({
                "kind":"client_notice",
                "id":"client_notice",
                "rect":rect,
                "visible":layout_visible(world, entity.id(), node, transform),
                "text":entity.get::<Text>().map(|text| text.0.as_str()),
            }));
            continue;
        }
        let Some(id) = entity.get::<AuthoredWidgetId>() else {
            continue;
        };
        let (Some(node), Some(transform)) = (
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
        ) else {
            rows.push(json!({"kind":"widget","id":id.0,"rect":null,"visible":false,"enabled":enabled(&entity)}));
            continue;
        };
        // turts_ui keeps the existing camera-viewport physical-pixel contract.
        let center = transform.to_scale_angle_translation().2;
        let size = node.size();
        let rect = [
            center.x - size.x / 2.0,
            center.y - size.y / 2.0,
            size.x,
            size.y,
        ];
        let action = entity.get::<WidgetButtonAction>().map(|action| &action.0);
        let text = entity.get::<Text>().map(|text| text.0.as_str());
        let button = entity.contains::<Button>();
        rows.push(json!({
            "kind":if button { "button" } else { "widget" },
            "id":id.0,
            "rect":rect,
            "visible":layout_visible(world, entity.id(), node, transform),
            "enabled":enabled(&entity),
            "action":null,
            "widget_action":action,
            "text":text,
            "button_pressed":button && entity.contains::<bevy::ui::Pressed>(),
            "capture_root":entity.contains::<UiCaptureRoot>(),
        }));
    }
    for entity in world.iter_entities() {
        let Some(tooltip) = entity.get::<WorldTooltip>() else {
            continue;
        };
        let text = entity.get::<Text>().map(|text| text.0.as_str());
        let (rect, visible) = match (
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
        ) {
            (Some(node), Some(transform)) => {
                // Use the actual viewport-pixel node geometry and visibility of the world label.
                let center = transform.to_scale_angle_translation().2;
                let size = node.size();
                let rect = [
                    center.x - size.x / 2.0,
                    center.y - size.y / 2.0,
                    size.x,
                    size.y,
                ];
                (
                    json!(rect),
                    layout_visible(world, entity.id(), node, transform),
                )
            }
            _ => (Value::Null, false),
        };
        rows.push(json!({
            "kind":"world_tooltip",
            "id":format!("world.tooltip.{}", tooltip.0),
            "unit_id":tooltip.0,
            "rect":rect,
            "visible":visible,
            "text":text,
        }));
    }
    rows.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    Value::Array(rows)
}

/// Scene-bridge widget rectangles in window-logical/CSS coordinates, separate from turts_ui.
pub fn scene_widgets(world: &World) -> Value {
    let mut widgets = Vec::new();
    for entity in world.iter_entities() {
        let Some(id) = entity.get::<AuthoredWidgetId>() else {
            continue;
        };
        let (Some(node), Some(transform), Some(camera_id)) = (
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
            entity
                .get::<ComputedUiTargetCamera>()
                .and_then(ComputedUiTargetCamera::get),
        ) else {
            continue;
        };
        if !layout_visible(world, entity.id(), node, transform) {
            continue;
        }
        let Some(camera) = world.get::<Camera>(camera_id) else {
            continue;
        };
        let (Some(viewport), Some(scale)) = (
            camera.physical_viewport_rect(),
            camera.target_scaling_factor(),
        ) else {
            continue;
        };
        if !scale.is_finite() || scale <= 0.0 {
            continue;
        }
        let center = transform.to_scale_angle_translation().2;
        let size = node.size();
        let origin = viewport.min.as_vec2();
        let rect = [
            (origin.x + center.x - size.x / 2.0) / scale,
            (origin.y + center.y - size.y / 2.0) / scale,
            size.x / scale,
            size.y / scale,
        ];
        widgets.push(json!({"id":id.0,"rect":rect}));
    }
    widgets.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    Value::Array(widgets)
}

pub fn publish_snapshot(world: &mut World) {
    let value = snapshot(world);
    if let Some(mut feed) = world.get_resource_mut::<UiSnapshotFeed>() {
        feed.0 = value;
    }
}

fn enabled(entity: &EntityRef<'_>) -> bool {
    entity
        .get::<WidgetEnabled>()
        .is_none_or(|enabled| enabled.0)
        && !entity.contains::<bevy::ui::InteractionDisabled>()
}

fn layout_visible(
    world: &World,
    entity: Entity,
    node: &ComputedNode,
    transform: &UiGlobalTransform,
) -> bool {
    if node.size().min_element() <= 0.0
        || !world
            .get::<InheritedVisibility>(entity)
            .is_some_and(|visibility| visibility.get())
    {
        return false;
    }
    let Some(camera_id) = world
        .get::<ComputedUiTargetCamera>(entity)
        .and_then(ComputedUiTargetCamera::get)
    else {
        return false;
    };
    let Some(camera) = world.get::<Camera>(camera_id) else {
        return false;
    };
    let Some(viewport) = camera
        .physical_viewport_rect()
        .map(|rect| rect.size().as_vec2())
    else {
        return false;
    };
    if !camera.is_active {
        return false;
    }
    let centre = transform.to_scale_angle_translation().2;
    let half_size = node.size() / 2.0;
    let rect = Rect::from_center_size(centre, node.size());
    let viewport_local = Rect::from_corners(Vec2::ZERO, viewport);
    if rect.intersect(viewport_local).is_empty() {
        return false;
    }
    match world.get::<CalculatedClip>(entity) {
        Some(CalculatedClip::FullyClipped) => false,
        Some(CalculatedClip::Rects(clips)) => clips.iter().all(|clip| {
            let corners = [
                Vec2::new(centre.x - half_size.x, centre.y - half_size.y),
                Vec2::new(centre.x + half_size.x, centre.y - half_size.y),
                Vec2::new(centre.x - half_size.x, centre.y + half_size.y),
                Vec2::new(centre.x + half_size.x, centre.y + half_size.y),
            ];
            let mut local = corners
                .into_iter()
                .map(|point| clip.world_to_clip_local.transform_point2(point));
            let first = local.next().unwrap_or(Vec2::ZERO);
            let bounds = local.fold(Rect::from_corners(first, first), |bounds, point| {
                Rect::from_corners(bounds.min.min(point), bounds.max.max(point))
            });
            !bounds.intersect(clip.rect).is_empty()
        }),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::primitives::{WidgetAction, WidgetButtonAction};

    #[test]
    fn layout_visibility_uses_viewport_local_bounds() {
        // A viewport with a non-zero physical origin still has UI-local top-left (0, 0).
        // Its origin is added later when the separate CSS/window rect is published.
        let local_viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(640.0, 360.0));
        let top_left_widget = Rect::from_corners(Vec2::ZERO, Vec2::new(80.0, 40.0));
        let outside_widget = Rect::from_corners(Vec2::new(650.0, 10.0), Vec2::new(670.0, 30.0));
        assert!(!top_left_widget.intersect(local_viewport).is_empty());
        assert!(outside_widget.intersect(local_viewport).is_empty());
    }

    #[test]
    fn snapshot_keeps_nonbutton_text_and_reports_only_a_real_pressed_button() {
        let mut world = World::new();
        let text = world
            .spawn((
                AuthoredWidgetId("notice".into()),
                Text::new("queue_full"),
                ComputedNode {
                    size: Vec2::new(120.0, 20.0),
                    ..default()
                },
                UiGlobalTransform::from_translation(Vec2::new(60.0, 10.0)),
                InheritedVisibility::VISIBLE,
                WidgetEnabled(true),
            ))
            .id();
        let nonbutton = snapshot(&world);
        assert_eq!(nonbutton[0]["kind"], "widget");
        assert_eq!(nonbutton[0]["text"], "queue_full");
        assert_eq!(
            nonbutton[0]["visible"], false,
            "no camera means no rendered proof"
        );
        assert_eq!(nonbutton[0]["button_pressed"], false);

        let action = WidgetAction::Input {
            name: "toggle_guide".into(),
        };
        world.entity_mut(text).insert((
            Button,
            bevy::ui::Pressed,
            WidgetButtonAction(action.clone()),
        ));
        let button = snapshot(&world);
        assert_eq!(button[0]["kind"], "button");
        assert_eq!(button[0]["button_pressed"], true);
        assert_eq!(button[0]["widget_action"]["kind"], "input");
        assert_eq!(button[0]["widget_action"]["name"], "toggle_guide");
    }

    #[test]
    fn engine_notice_publishes_its_real_text_without_an_authored_widget_id() {
        let mut world = World::new();
        world.spawn((
            crate::client_shell::ClientNotice,
            Text::new("This game defines no orders"),
            ComputedNode {
                size: Vec2::new(240.0, 28.0),
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(120.0, 14.0)),
            InheritedVisibility::VISIBLE,
        ));
        let rows = snapshot(&world);
        let rows = rows.as_array().expect("snapshot rows are an array");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["kind"], "client_notice");
        assert_eq!(rows[0]["id"], "client_notice");
        assert_eq!(rows[0]["text"], "This game defines no orders");
        assert_eq!(
            rows[0]["visible"], false,
            "layout without a real active camera is not claimed as visible"
        );
    }

    #[test]
    fn snapshot_observes_world_tooltip_text_from_its_real_marker_and_layout() {
        let mut world = World::new();
        world.spawn((
            WorldTooltip(42),
            Text::new("Shell Turret · Slot 2"),
            ComputedNode {
                size: Vec2::new(140.0, 24.0),
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(90.0, 42.0)),
            InheritedVisibility::VISIBLE,
        ));

        let rows = snapshot(&world);
        let tooltip = rows
            .as_array()
            .expect("snapshot must return a JSON array")
            .iter()
            .find(|row| row["kind"] == "world_tooltip")
            .expect("turts_ui must include actual world tooltip entities");
        assert_eq!(tooltip["id"], "world.tooltip.42");
        assert_eq!(tooltip["unit_id"], 42);
        assert_eq!(tooltip["text"], "Shell Turret · Slot 2");
        assert_eq!(
            tooltip["rect"],
            serde_json::json!([20.0, 30.0, 140.0, 24.0])
        );
        assert_eq!(
            tooltip["visible"], false,
            "no camera means no rendered proof"
        );
    }
}
