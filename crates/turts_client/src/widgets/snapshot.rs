//! Published view of actual, post-layout authored widgets.

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};

use super::primitives::{AuthoredWidgetId, WidgetButtonAction, WidgetEnabled};
use crate::visuals::WorldTooltip;
use crate::{debug_feed::UiSnapshotFeed, ui_shared::UiCaptureRoot};

const MAX_SETTINGS_MENU_ROOTS: usize = 4;
const MAX_SETTINGS_MENU_DESCENDANTS: usize = 512;
const MAX_SETTINGS_MENU_TEXT_NODES: usize = 256;
const MAX_FOCUS_OVERLAY_ROOTS: usize = 4;
const MAX_FOCUS_OVERLAY_DESCENDANTS: usize = 512;
const MAX_FOCUS_OVERLAY_TEXT_NODES: usize = 256;

/// Build the bridge rows from computed Bevy UI geometry, never from the authored declaration.
pub fn snapshot(world: &World) -> Value {
    let mut rows = Vec::new();
    let mut focus_overlay_root = None;
    let mut focus_overlay_count = 0;
    for entity in world.iter_entities() {
        if entity.contains::<crate::client_shell::FocusOverlay>() {
            focus_overlay_count += 1;
            if focus_overlay_count == 1 {
                focus_overlay_root = Some(entity.id());
            }
        }
    }
    if focus_overlay_count > MAX_FOCUS_OVERLAY_ROOTS {
        rows.push(json!({
            "kind": "focus_overlay_overflow",
            "count_at_least": MAX_FOCUS_OVERLAY_ROOTS + 1,
        }));
    } else if focus_overlay_count > 1 {
        rows.push(json!({
            "kind": "focus_overlay",
            "id": "focus_overlay",
            "root_count": focus_overlay_count,
            "rect": null,
            "visible": false,
            "capture_root": false,
            "focus_policy_block": false,
            "ambiguous_roots": true,
            "text_nodes": [],
            "descendants_visited": 0,
            "descendants_truncated": false,
            "text_truncated": false,
        }));
    } else if focus_overlay_count == 0 {
        rows.push(json!({
            "kind": "focus_overlay",
            "id": "focus_overlay",
            "root_count": 0,
            "rect": null,
            "visible": false,
            "capture_root": false,
            "focus_policy_block": false,
            "text_nodes": [],
            "descendants_visited": 0,
            "descendants_truncated": false,
            "text_truncated": false,
        }));
    } else if let Some(root) = focus_overlay_root {
        let entity = world.entity(root);
        let (rect, visible) = match (
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
        ) {
            (Some(node), Some(transform)) => {
                let rect = computed_rect(node, transform);
                let visible = rect.is_some() && layout_visible(world, root, node, transform);
                (rect.map_or(Value::Null, |rect| json!(rect)), visible)
            }
            _ => (Value::Null, false),
        };
        let (text_nodes, descendants_visited, descendants_truncated, text_truncated) =
            focus_overlay_text_nodes(world, root);
        rows.push(json!({
            "kind": "focus_overlay",
            "id": "focus_overlay",
            "root_count": 1,
            "rect": rect,
            "visible": visible,
            "capture_root": entity.contains::<UiCaptureRoot>(),
            "focus_policy_block": entity.get::<bevy::ui::FocusPolicy>() == Some(&bevy::ui::FocusPolicy::Block),
            "text_nodes": text_nodes,
            "descendants_visited": descendants_visited,
            "descendants_truncated": descendants_truncated,
            "text_truncated": text_truncated,
        }));
    }
    let mut settings_menu_roots = 0;
    for entity in world.iter_entities() {
        if entity.contains::<crate::client_shell::SettingsMenuOverlay>() {
            settings_menu_roots += 1;
            if settings_menu_roots > MAX_SETTINGS_MENU_ROOTS {
                if settings_menu_roots == MAX_SETTINGS_MENU_ROOTS + 1 {
                    rows.push(json!({
                        "kind": "settings_menu_overflow",
                        "count_at_least": settings_menu_roots,
                    }));
                }
                continue;
            }
            let (rect, visible) = match (
                entity.get::<ComputedNode>(),
                entity.get::<UiGlobalTransform>(),
            ) {
                (Some(node), Some(transform)) => {
                    let rect = computed_rect(node, transform);
                    let visible =
                        rect.is_some() && layout_visible(world, entity.id(), node, transform);
                    (rect.map_or(Value::Null, |rect| json!(rect)), visible)
                }
                _ => (Value::Null, false),
            };
            let (text_nodes, descendants_visited, descendants_truncated, text_truncated) =
                settings_menu_text_nodes(world, entity.id());
            rows.push(json!({
                "kind": "settings_menu",
                "id": "settings_menu",
                "rect": rect,
                "visible": visible,
                "capture_root": entity.contains::<UiCaptureRoot>(),
                "focus_policy_block": entity.get::<bevy::ui::FocusPolicy>() == Some(&bevy::ui::FocusPolicy::Block),
                "text_nodes": text_nodes,
                "descendants_visited": descendants_visited,
                "descendants_truncated": descendants_truncated,
                "text_truncated": text_truncated,
            }));
            continue;
        }
        if entity.contains::<crate::client_shell::PlayScreenSoundSwitch>() {
            // The sound switch of the play screen: its words and its physical rectangle, so a
            // browser check can see that a click lands on it.
            let text = world.get::<Children>(entity.id()).and_then(|children| {
                children
                    .iter()
                    .find_map(|child| world.get::<Text>(child).map(|text| text.0.as_str()))
            });
            let (rect, visible) = match (
                entity.get::<ComputedNode>(),
                entity.get::<UiGlobalTransform>(),
            ) {
                (Some(node), Some(transform)) => {
                    let rect = computed_rect(node, transform);
                    let visible =
                        rect.is_some() && layout_visible(world, entity.id(), node, transform);
                    (rect.map_or(Value::Null, |rect| json!(rect)), visible)
                }
                _ => (Value::Null, false),
            };
            rows.push(json!({
                "kind":"sound_switch",
                "id":"sound_switch",
                "rect":rect,
                "visible":visible,
                "text":text,
                "capture_root":entity.contains::<UiCaptureRoot>(),
            }));
            continue;
        }
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
        let action = if entity.contains::<crate::client_screens::ReloadButton>() {
            Some(json!({"kind":"reload"}))
        } else {
            entity
                .get::<WidgetButtonAction>()
                .map(|action| json!(&action.0))
        };
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

fn settings_menu_text_nodes(world: &World, root: Entity) -> (Vec<Value>, usize, bool, bool) {
    let mut pending = world
        .get::<Children>(root)
        .map(|children| children.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    pending.reverse();
    let mut text_nodes = Vec::new();
    let mut descendants_visited = 0;
    let mut descendants_truncated = false;
    let mut text_truncated = false;

    while let Some(entity) = pending.pop() {
        if descendants_visited >= MAX_SETTINGS_MENU_DESCENDANTS {
            descendants_truncated = true;
            break;
        }
        descendants_visited += 1;
        if let Some(text) = world.get::<Text>(entity) {
            if text_nodes.len() >= MAX_SETTINGS_MENU_TEXT_NODES {
                text_truncated = true;
            } else {
                let (rect, visible) = match (
                    world.get::<ComputedNode>(entity),
                    world.get::<UiGlobalTransform>(entity),
                ) {
                    (Some(node), Some(transform)) => {
                        let rect = computed_rect(node, transform);
                        let visible =
                            rect.is_some() && layout_visible(world, entity, node, transform);
                        (rect.map_or(Value::Null, |rect| json!(rect)), visible)
                    }
                    _ => (Value::Null, false),
                };
                let button_geometry = world
                    .get::<ChildOf>(entity)
                    .map(ChildOf::parent)
                    .filter(|parent| world.get::<Button>(*parent).is_some())
                    .and_then(|parent| {
                        let node = world.get::<ComputedNode>(parent)?;
                        let transform = world.get::<UiGlobalTransform>(parent)?;
                        Some((
                            computed_rect(node, transform)?,
                            layout_visible(world, parent, node, transform),
                        ))
                    });
                text_nodes.push(json!({
                    "text": text.0.as_str(),
                    "rect": rect,
                    "visible": visible,
                    "button_rect": button_geometry.map(|(rect, _)| rect),
                    "button_visible": button_geometry.is_some_and(|(_, visible)| visible),
                }));
            }
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter().rev());
        }
    }
    (
        text_nodes,
        descendants_visited,
        descendants_truncated,
        text_truncated,
    )
}

fn focus_overlay_text_nodes(world: &World, root: Entity) -> (Vec<Value>, usize, bool, bool) {
    let mut pending = world
        .get::<Children>(root)
        .map(|children| children.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    pending.reverse();
    let mut text_nodes = Vec::new();
    let mut descendants_visited = 0;
    let mut descendants_truncated = false;
    let mut text_truncated = false;

    while let Some(entity) = pending.pop() {
        if descendants_visited >= MAX_FOCUS_OVERLAY_DESCENDANTS {
            descendants_truncated = true;
            break;
        }
        descendants_visited += 1;
        if let Some(text) = world.get::<Text>(entity) {
            if text_nodes.len() >= MAX_FOCUS_OVERLAY_TEXT_NODES {
                text_truncated = true;
            } else {
                let (rect, visible) = match (
                    world.get::<ComputedNode>(entity),
                    world.get::<UiGlobalTransform>(entity),
                ) {
                    (Some(node), Some(transform)) => {
                        let rect = computed_rect(node, transform);
                        let visible =
                            rect.is_some() && layout_visible(world, entity, node, transform);
                        (rect.map_or(Value::Null, |rect| json!(rect)), visible)
                    }
                    _ => (Value::Null, false),
                };
                text_nodes.push(json!({
                    "text": text.0.as_str(),
                    "rect": rect,
                    "visible": visible,
                }));
            }
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter().rev());
        }
    }
    (
        text_nodes,
        descendants_visited,
        descendants_truncated,
        text_truncated,
    )
}

fn computed_rect(node: &ComputedNode, transform: &UiGlobalTransform) -> Option<[f32; 4]> {
    let size = node.size();
    let center = transform.to_scale_angle_translation().2;
    if !size.is_finite() || !center.is_finite() || size.min_element() <= 0.0 {
        return None;
    }
    Some([
        center.x - size.x / 2.0,
        center.y - size.y / 2.0,
        size.x,
        size.y,
    ])
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
        let notice = nonbutton
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"] == "notice"))
            .expect("authored notice is present beside the focus-overlay observation");
        assert_eq!(notice["kind"], "widget");
        assert_eq!(notice["text"], "queue_full");
        assert_eq!(
            notice["visible"], false,
            "no camera means no rendered proof"
        );
        assert_eq!(notice["button_pressed"], false);

        let action = WidgetAction::Input {
            name: "toggle_guide".into(),
        };
        world.entity_mut(text).insert((
            Button,
            bevy::ui::Pressed,
            WidgetButtonAction(action.clone()),
        ));
        let button = snapshot(&world);
        let button = button
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"] == "notice"))
            .expect("pressed authored widget is present beside the focus-overlay observation");
        assert_eq!(button["kind"], "button");
        assert_eq!(button["button_pressed"], true);
        assert_eq!(button["widget_action"]["kind"], "input");
        assert_eq!(button["widget_action"]["name"], "toggle_guide");
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
        let notices: Vec<_> = rows
            .iter()
            .filter(|row| row["kind"] == "client_notice")
            .collect();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0]["id"], "client_notice");
        assert_eq!(notices[0]["text"], "This game defines no orders");
        assert_eq!(
            notices[0]["visible"], false,
            "layout without a real active camera is not claimed as visible"
        );
    }

    #[test]
    fn focus_overlay_snapshot_emits_actual_absence_and_zero_layout_without_claiming_visibility() {
        let mut world = World::new();
        let rows = snapshot(&world);
        let absent = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["kind"] == "focus_overlay")
            .expect("the existing snapshot feed reports the actual focus overlay count");
        assert_eq!(absent["root_count"], 0);
        assert_eq!(absent["rect"], Value::Null);
        assert_eq!(absent["visible"], false);
        assert_eq!(absent["text_nodes"], json!([]));

        let root = world.spawn(crate::client_shell::FocusOverlay).id();
        let text = world.spawn(Text::new("Click to play")).id();
        world.entity_mut(root).add_child(text);
        let rows = snapshot(&world);
        let overlay = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["kind"] == "focus_overlay")
            .unwrap();
        assert_eq!(overlay["root_count"], 1);
        assert_eq!(overlay["rect"], Value::Null);
        assert_eq!(overlay["visible"], false);
        assert_eq!(overlay["text_nodes"][0]["text"], "Click to play");
        assert_eq!(overlay["text_nodes"][0]["rect"], Value::Null);
        assert_eq!(overlay["text_nodes"][0]["visible"], false);
    }

    #[test]
    fn focus_overlay_snapshot_caps_roots_and_emits_an_explicit_overflow_row() {
        let mut world = World::new();
        for _ in 0..=MAX_FOCUS_OVERLAY_ROOTS {
            world.spawn(crate::client_shell::FocusOverlay);
        }
        let rows = snapshot(&world);
        assert!(rows.as_array().unwrap().iter().any(|row| {
            row["kind"] == "focus_overlay_overflow"
                && row["count_at_least"] == MAX_FOCUS_OVERLAY_ROOTS + 1
        }));
    }

    #[test]
    fn focus_overlay_snapshot_bounds_actual_descendant_and_text_traversal() {
        let mut world = World::new();
        let root = world.spawn(crate::client_shell::FocusOverlay).id();
        for index in 0..=MAX_FOCUS_OVERLAY_DESCENDANTS {
            let child = world
                .spawn(Text::new(format!("overlay label {index}")))
                .id();
            world.entity_mut(root).add_child(child);
        }
        let rows = snapshot(&world);
        let overlay = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["kind"] == "focus_overlay")
            .unwrap();
        assert_eq!(overlay["root_count"], 1);
        assert_eq!(
            overlay["text_nodes"].as_array().unwrap().len(),
            MAX_FOCUS_OVERLAY_TEXT_NODES
        );
        assert_eq!(
            overlay["descendants_visited"],
            MAX_FOCUS_OVERLAY_DESCENDANTS
        );
        assert_eq!(overlay["descendants_truncated"], true);
        assert_eq!(overlay["text_truncated"], true);
    }

    #[test]
    fn settings_menu_snapshot_caps_roots_and_emits_an_explicit_overflow_row() {
        let mut world = World::new();
        for _ in 0..=MAX_SETTINGS_MENU_ROOTS {
            world.spawn(crate::client_shell::SettingsMenuOverlay);
        }

        let rows = snapshot(&world);
        let rows = rows.as_array().expect("snapshot rows are an array");
        assert_eq!(
            rows.iter()
                .filter(|row| row["kind"] == "settings_menu")
                .count(),
            MAX_SETTINGS_MENU_ROOTS
        );
        assert!(rows.iter().any(|row| {
            row["kind"] == "settings_menu_overflow"
                && row["count_at_least"] == MAX_SETTINGS_MENU_ROOTS + 1
        }));
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
