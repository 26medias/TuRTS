//! Client presentation for authored minimap widgets.

use std::collections::{HashMap, HashSet};

use bevy::camera::{CameraUpdateSystems, RenderTarget};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::window::{PrimaryWindow, WindowRef};
use turts_net_protocol::NodeId;

use crate::{
    debug_feed::AlertPingsFeed, ingest::ReplicaState, join::AcquiredPackages, present::Presented,
    widgets::primitives::MinimapViewConfig,
};

#[derive(Debug, Clone, Copy, PartialEq)]
struct PingMarker {
    id: u64,
    position: Vec2,
}

#[derive(Component, Default)]
struct MinimapChildren {
    units: HashMap<u32, Entity>,
    nodes: HashMap<NodeId, Entity>,
    pings: HashMap<u64, [Entity; 2]>,
    frame: Vec<Entity>,
    fog: Vec<Entity>,
    fog_basis: Option<(turts_mod_api::KnowledgeBasis, Vec2)>,
    remembered: HashMap<(bool, u32), Entity>,
}

fn ping_markers(pings: &AlertPingsFeed, map_size: Vec2, now: f64) -> Vec<PingMarker> {
    pings
        .0
        .iter()
        .filter_map(|value| {
            let id = value.get("id")?.as_u64()?;
            // The feed retains recent alerts for diagnostics. Their map pictures expire.
            if let Some(at) = value.get("at") {
                let at = at.as_f64()?;
                if !(at..at + 2.0).contains(&now) || !at.is_finite() {
                    return None;
                }
            }
            let x = value.get("x")?.as_f64()? as f32;
            let y = value.get("y")?.as_f64()? as f32;
            let position = Vec2::new(x, y);
            (position.is_finite()
                && map_size.is_finite()
                && map_size.x > 0.0
                && map_size.y > 0.0
                && position.x >= 0.0
                && position.y >= 0.0
                && position.x <= map_size.x
                && position.y <= map_size.y)
                .then_some(PingMarker { id, position })
        })
        .collect()
}

/// Owns only the contents of the authored `MinimapView` root. Every child inherits the root's
/// normal widget visibility and layout target, so hidden or clipped authored widgets stay hidden.
pub struct MinimapPlugin;
impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            sync_minimap
                .after(TransformSystems::Propagate)
                .after(CameraUpdateSystems),
        );
    }
}

fn sync_minimap(world: &mut World) {
    let roots = world
        .query::<(Entity, &MinimapViewConfig, &ComputedNode)>()
        .iter(world)
        .map(|(entity, config, node)| (entity, *config, node.size() * node.inverse_scale_factor))
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return;
    }

    let Some(acquired) = world.get_resource::<AcquiredPackages>() else {
        return;
    };
    let map_size = Vec2::new(acquired.0.map_size.0, acquired.0.map_size.1);
    if !map_size.is_finite() || map_size.x <= 0.0 || map_size.y <= 0.0 {
        return;
    }
    let slot_colours = acquired
        .0
        .presentation
        .interface
        .as_ref()
        .map(|interface| interface.slot_colours.clone())
        .unwrap_or_default();
    let match_start = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.match_start.as_ref());
    let slot_colours = (0..crate::palette::SLOT_RGB.len())
        .map(|index| {
            let colour_index = crate::palette::colour_index(index as u8, match_start);
            slot_colours
                .get(colour_index)
                .and_then(|hex| crate::widgets::primitives::parse_colour(hex))
                .unwrap_or_else(|| {
                    let [r, g, b] = crate::palette::SLOT_RGB
                        [colour_index.min(crate::palette::SLOT_RGB.len() - 1)];
                    Color::srgb_u8(r, g, b)
                })
        })
        .collect::<Vec<_>>();
    let node_colour = Color::srgb_u8(242, 204, 96);

    let units = world
        .get_resource::<Presented>()
        .zip(world.get_resource::<ReplicaState>())
        .and_then(|(presented, state)| state.replica.as_ref().map(|replica| (presented, replica)))
        .map(|(presented, replica)| {
            presented
                .units
                .iter()
                .filter_map(|shown| {
                    let owner_slot = replica.units.get(&shown.unit_id)?.owner_slot;
                    Some((shown.unit_id, shown.pos, owner_slot))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let nodes = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.replica.as_ref())
        .map(|replica| {
            replica
                .nodes
                .values()
                .map(|node| (node.id, node.pos))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let knowledge = world
        .get_resource::<ReplicaState>()
        .and_then(|state| state.replica.as_ref())
        .and_then(|replica| replica.knowledge.clone());
    let now = world
        .get_resource::<Time>()
        .map_or(0.0, Time::elapsed_secs_f64);
    let pings = world
        .get_resource::<AlertPingsFeed>()
        .map_or_else(Vec::new, |feed| ping_markers(feed, map_size, now));
    let frame_corners = camera_ground_corners(world);

    for (root, config, size) in roots {
        if !size.is_finite() || size.min_element() <= 0.0 {
            continue;
        }
        if world.get::<MinimapChildren>(root).is_none() {
            world.entity_mut(root).insert(MinimapChildren::default());
        }
        let mut children = world
            .entity_mut(root)
            .take::<MinimapChildren>()
            .unwrap_or_default();

        let fog_basis = knowledge
            .as_ref()
            .map(|knowledge| (knowledge.basis.clone(), size));
        if children.fog_basis != fog_basis {
            sync_fog(world, root, &mut children.fog, knowledge.as_deref(), size);
            children.fog_basis = fog_basis;
        }
        let remembered = knowledge
            .as_ref()
            .map(|knowledge| {
                knowledge
                    .remembered_units
                    .iter()
                    .map(|row| ((false, row.unit), row.position))
                    .chain(
                        knowledge
                            .remembered_nodes
                            .iter()
                            .map(|row| ((true, row.node), row.position)),
                    )
                    .filter_map(|(id, pos)| {
                        map_local(Vec2::new(pos.x, pos.y), map_size, size)
                            .map(|point| (id, point, Color::srgb(0.48, 0.50, 0.56)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        sync_square_markers(world, root, &mut children.remembered, remembered, 3.0);

        let desired_units = units
            .iter()
            .filter_map(|(id, pos, slot)| {
                map_local(Vec2::new(pos.x, pos.y), map_size, size).map(|local| {
                    (
                        *id,
                        local,
                        slot_colours[usize::from(*slot).min(slot_colours.len() - 1)],
                    )
                })
            })
            .collect::<Vec<_>>();
        sync_square_markers(
            world,
            root,
            &mut children.units,
            desired_units,
            config.unit_size,
        );

        let desired_nodes = nodes
            .iter()
            .filter_map(|(id, pos)| {
                map_local(Vec2::new(pos.x, pos.y), map_size, size)
                    .map(|local| (*id, local, node_colour))
            })
            .collect::<Vec<_>>();
        sync_square_markers(world, root, &mut children.nodes, desired_nodes, 4.0);

        if let Some(ping_colour) = config.ping_colour {
            sync_ping_markers(
                world,
                root,
                &mut children.pings,
                &pings,
                map_size,
                size,
                ping_colour,
            );
        } else {
            sync_ping_markers(
                world,
                root,
                &mut children.pings,
                &[],
                map_size,
                size,
                Color::NONE,
            );
        }

        let frame_points = frame_corners.as_ref().map_or_else(Vec::new, |corners| {
            camera_frame_points(corners, map_size, size)
        });
        sync_frame_points(
            world,
            root,
            &mut children.frame,
            &frame_points,
            config.frame_colour.unwrap_or(Color::WHITE),
        );
        world.entity_mut(root).insert(children);
    }
}

fn camera_ground_corners(world: &mut World) -> Option<[Vec2; 4]> {
    let primary = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next()?;
    let mut query =
        world.query_filtered::<(&Camera, &GlobalTransform, &RenderTarget), With<Camera3d>>();
    let (camera, transform, _) = query.iter(world).find(|(camera, _, target)| {
        camera.is_active
            && (matches!(target, RenderTarget::Window(WindowRef::Primary))
                || matches!(target, RenderTarget::Window(WindowRef::Entity(entity)) if *entity == primary))
    })?;
    let viewport = camera.logical_viewport_rect()?;
    let positions = [
        viewport.min,
        Vec2::new(viewport.max.x, viewport.min.y),
        viewport.max,
        Vec2::new(viewport.min.x, viewport.max.y),
    ];
    let mut corners = [Vec2::ZERO; 4];
    for (index, position) in positions.into_iter().enumerate() {
        let ray = camera.viewport_to_world(transform, position).ok()?;
        let direction = ray.direction.as_vec3();
        if !direction.is_finite() || direction.y.abs() <= f32::EPSILON {
            return None;
        }
        let distance = -ray.origin.y / direction.y;
        if !distance.is_finite() || distance < 0.0 {
            return None;
        }
        let ground = ray.origin + direction * distance;
        if !ground.is_finite() {
            return None;
        }
        corners[index] = Vec2::new(ground.x, ground.z);
    }
    Some(corners)
}

fn map_local(position: Vec2, map_size: Vec2, widget_size: Vec2) -> Option<Vec2> {
    if !position.is_finite()
        || !map_size.is_finite()
        || map_size.min_element() <= 0.0
        || !widget_size.is_finite()
        || widget_size.min_element() <= 0.0
        || position.x < 0.0
        || position.y < 0.0
        || position.x > map_size.x
        || position.y > map_size.y
    {
        return None;
    }
    let max = (widget_size - Vec2::splat(1.0)).max(Vec2::ZERO);
    let local = turts_interaction::minimap::map_to_minimap(position, map_size) * widget_size;
    Some(local.clamp(Vec2::ZERO, max))
}

fn camera_frame_points(corners: &[Vec2; 4], map_size: Vec2, widget_size: Vec2) -> Vec<Vec2> {
    let mut unique = HashSet::new();
    let mut points = Vec::new();
    for index in 0..4 {
        let start = corners[index];
        let end = corners[(index + 1) % 4];
        let Some((start, end)) = clip_to_map(start, end, map_size) else {
            continue;
        };
        let start_local = map_local(start, map_size, widget_size).unwrap_or(Vec2::ZERO);
        let end_local = map_local(end, map_size, widget_size).unwrap_or(Vec2::ZERO);
        let steps = ((end_local - start_local).length() / 2.0).ceil().max(1.0) as usize;
        for step in 0..=steps {
            let point = start_local
                .lerp(end_local, step as f32 / steps as f32)
                .round();
            let key = (point.x as i32, point.y as i32);
            if unique.insert(key) {
                points.push(point);
            }
        }
    }
    points
}

fn clip_to_map(start: Vec2, end: Vec2, size: Vec2) -> Option<(Vec2, Vec2)> {
    let delta = end - start;
    let mut near = 0.0_f32;
    let mut far = 1.0_f32;
    for (p, q) in [
        (-delta.x, start.x),
        (delta.x, size.x - start.x),
        (-delta.y, start.y),
        (delta.y, size.y - start.y),
    ] {
        if p.abs() <= f32::EPSILON {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let ratio = q / p;
        if p < 0.0 {
            near = near.max(ratio);
        } else {
            far = far.min(ratio);
        }
        if near > far {
            return None;
        }
    }
    Some((start + delta * near, start + delta * far))
}

fn sync_square_markers<K: Copy + Eq + std::hash::Hash>(
    world: &mut World,
    root: Entity,
    existing: &mut HashMap<K, Entity>,
    desired: Vec<(K, Vec2, Color)>,
    marker_size: f32,
) {
    let marker_size = if marker_size.is_finite() && marker_size > 0.0 {
        marker_size
    } else {
        2.0
    };
    let keep = desired.iter().map(|(id, _, _)| *id).collect::<HashSet<_>>();
    let removed = existing
        .keys()
        .filter(|id| !keep.contains(id))
        .copied()
        .collect::<Vec<_>>();
    for id in removed {
        if let Some(entity) = existing.remove(&id) {
            let _ = world.despawn(entity);
        }
    }
    for (key, point, color) in desired {
        let entity = if let Some(entity) = existing.get(&key).copied() {
            entity
        } else {
            let entity = spawn_marker(world, root);
            existing.insert(key, entity);
            entity
        };
        let half = marker_size / 2.0;
        update_marker(
            world,
            entity,
            marker_node(point.x - half, point.y - half, marker_size, marker_size),
            color,
            2,
        );
    }
}

fn sync_ping_markers(
    world: &mut World,
    root: Entity,
    existing: &mut HashMap<u64, [Entity; 2]>,
    desired: &[PingMarker],
    map_size: Vec2,
    widget_size: Vec2,
    color: Color,
) {
    let keep = desired.iter().map(|ping| ping.id).collect::<HashSet<_>>();
    let removed = existing
        .keys()
        .filter(|id| !keep.contains(id))
        .copied()
        .collect::<Vec<_>>();
    for id in removed {
        if let Some(entities) = existing.remove(&id) {
            for entity in entities {
                let _ = world.despawn(entity);
            }
        }
    }
    for ping in desired {
        let Some(point) = map_local(ping.position, map_size, widget_size) else {
            continue;
        };
        let entities = if let Some(entities) = existing.get(&ping.id).copied() {
            entities
        } else {
            let pair = [spawn_marker(world, root), spawn_marker(world, root)];
            existing.insert(ping.id, pair);
            pair
        };
        update_marker(
            world,
            entities[0],
            marker_node(point.x - 5.0, point.y - 1.0, 10.0, 2.0),
            color,
            4,
        );
        update_marker(
            world,
            entities[1],
            marker_node(point.x - 1.0, point.y - 5.0, 2.0, 10.0),
            color,
            4,
        );
    }
}

fn sync_frame_points(
    world: &mut World,
    root: Entity,
    existing: &mut Vec<Entity>,
    desired: &[Vec2],
    color: Color,
) {
    while existing.len() > desired.len() {
        if let Some(entity) = existing.pop() {
            let _ = world.despawn(entity);
        }
    }
    while existing.len() < desired.len() {
        existing.push(spawn_marker(world, root));
    }
    for (entity, point) in existing.iter().copied().zip(desired) {
        update_marker(
            world,
            entity,
            marker_node(point.x - 1.0, point.y - 1.0, 2.0, 2.0),
            color,
            3,
        );
    }
}

// Avoid forcing a full UI layout when a stationary marker has no visible change.
fn update_marker(world: &mut World, entity: Entity, node: Node, color: Color, z: i32) {
    if let Some(mut current) = world.get_mut::<Node>(entity) {
        current.set_if_neq(node);
    }
    if let Some(mut current) = world.get_mut::<BackgroundColor>(entity) {
        current.set_if_neq(BackgroundColor(color));
    }
    if let Some(mut current) = world.get_mut::<ZIndex>(entity) {
        current.set_if_neq(ZIndex(z));
    }
}

fn spawn_marker(world: &mut World, root: Entity) -> Entity {
    let entity = world
        .spawn((
            marker_node(0.0, 0.0, 1.0, 1.0),
            BackgroundColor(Color::NONE),
            ZIndex(2),
        ))
        .id();
    world.entity_mut(root).add_child(entity);
    entity
}

fn marker_node(left: f32, top: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(left),
        top: Val::Px(top),
        width: Val::Px(width),
        height: Val::Px(height),
        ..default()
    }
}

/// Rasterize accepted knowledge into bounded minimap rows, then merge equal adjacent cells.
fn sync_fog(
    world: &mut World,
    root: Entity,
    entities: &mut Vec<Entity>,
    knowledge: Option<&turts_mod_api::KnowledgeSnapshot>,
    size: Vec2,
) {
    let mut desired = Vec::new();
    if let Some(knowledge) = knowledge {
        use crate::visibility::{FogCell, position_state};
        let width = (size.x.ceil() as u32).clamp(1, 512);
        let height = (size.y.ceil() as u32).clamp(1, 512);
        let pixel = Vec2::new(size.x / width as f32, size.y / height as f32);
        for y in 0..height {
            let state = |x: u32| {
                position_state(knowledge, {
                    let at = turts_interaction::minimap::minimap_to_map(
                        Vec2::new(
                            (x as f32 + 0.5) / width as f32,
                            (y as f32 + 0.5) / height as f32,
                        ),
                        Vec2::new(knowledge.grid.size.x, knowledge.grid.size.y),
                    );
                    turts_mod_api::WorldPos { x: at.x, y: at.y }
                })
                .unwrap_or(FogCell::Unexplored)
            };
            let mut x = 0;
            while x < width {
                let first = x;
                let fog = state(x);
                x += 1;
                while x < width && state(x) == fog {
                    x += 1;
                }
                let colour = match fog {
                    FogCell::Observed => continue,
                    FogCell::Remembered => Color::srgba(0.01, 0.015, 0.025, 0.65),
                    FogCell::Unexplored => Color::srgb(0.008, 0.01, 0.015),
                };
                desired.push((
                    marker_node(
                        first as f32 * pixel.x,
                        y as f32 * pixel.y,
                        (x - first) as f32 * pixel.x,
                        pixel.y,
                    ),
                    colour,
                ));
            }
        }
    }
    while entities.len() > desired.len() {
        if let Some(entity) = entities.pop() {
            let _ = world.despawn(entity);
        }
    }
    for (index, (node, colour)) in desired.into_iter().enumerate() {
        if entities.len() <= index {
            entities.push(spawn_marker(world, root));
        }
        update_marker(world, entities[index], node, colour, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn subpixel_minimap_dimensions_keep_marker_coordinates_finite_and_clipped() {
        let size = Vec2::splat(0.5);

        assert_eq!(
            map_local(Vec2::ZERO, Vec2::new(64.0, 48.0), size),
            Some(Vec2::ZERO)
        );
        assert_eq!(
            map_local(Vec2::new(64.0, 48.0), Vec2::new(64.0, 48.0), size),
            Some(Vec2::ZERO)
        );
        assert_eq!(map_local(Vec2::ZERO, Vec2::ZERO, size), None);
    }

    #[test]
    fn ping_shows_for_an_alert_ping() {
        let feed = AlertPingsFeed(vec![serde_json::json!({
            "id": 7,
            "alert_id": "muster_unit_damaged",
            "tick": 19,
            "unit": 41,
            "x": 12.0,
            "y": 9.0,
            "text": "Unit damaged",
        })]);

        let markers = ping_markers(&feed, Vec2::new(64.0, 48.0), 0.0);

        assert_eq!(
            markers.len(),
            1,
            "the real alert-ping feed entry must make one minimap marker"
        );
        assert_eq!(markers[0].id, 7);
        assert_eq!(markers[0].position, Vec2::new(12.0, 9.0));
    }

    #[test]
    fn timed_attack_ping_disappears_after_two_seconds() {
        let feed = AlertPingsFeed(vec![serde_json::json!({"id":1,"x":10.0,"y":20.0,"at":5.0})]);
        let map = Vec2::new(64.0, 48.0);
        assert!(ping_markers(&feed, map, 4.99).is_empty());
        assert_eq!(ping_markers(&feed, map, 5.0).len(), 1);
        assert_eq!(ping_markers(&feed, map, 6.99).len(), 1);
        assert!(ping_markers(&feed, map, 7.0).is_empty());
        assert!(ping_markers(&feed, map, 100.0).is_empty());
    }

    #[test]
    fn stationary_marker_does_not_request_another_layout() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let entity = spawn_marker(&mut world, root);
        update_marker(
            &mut world,
            entity,
            marker_node(20.0, 30.0, 4.0, 4.0),
            Color::WHITE,
            2,
        );
        world.clear_trackers();
        update_marker(
            &mut world,
            entity,
            marker_node(20.0, 30.0, 4.0, 4.0),
            Color::WHITE,
            2,
        );
        assert!(!world.entity(entity).get_ref::<Node>().unwrap().is_changed());
        assert!(
            !world
                .entity(entity)
                .get_ref::<BackgroundColor>()
                .unwrap()
                .is_changed()
        );
        update_marker(
            &mut world,
            entity,
            marker_node(21.0, 30.0, 4.0, 4.0),
            Color::WHITE,
            2,
        );
        assert!(world.entity(entity).get_ref::<Node>().unwrap().is_changed());
    }

    #[test]
    fn malformed_or_out_of_map_alert_pings_do_not_make_minimap_markers() {
        let feed = AlertPingsFeed(vec![
            Value::Null,
            serde_json::json!({ "id": 8, "x": 65.0, "y": 9.0 }),
            serde_json::json!({ "id": 9, "x": 12.0, "y": f32::NAN }),
        ]);

        assert!(ping_markers(&feed, Vec2::new(64.0, 48.0), 0.0).is_empty());
    }

    #[test]
    fn alert_ping_feed_is_consumed_as_two_visible_minimap_cross_primitives() {
        let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
        let archive = turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&package).expect("pack Muster"),
        )
        .expect("parse Muster");
        let mut acquired = crate::acquire::build_acquired(&[archive], String::new())
            .expect("acquire authored minimap presentation");
        acquired.map_size = (64.0, 48.0);

        let mut world = World::new();
        world.insert_resource(AcquiredPackages(acquired));
        world.insert_resource(ReplicaState::default());
        world.insert_resource(Presented::default());
        world.insert_resource(AlertPingsFeed(vec![serde_json::json!({
            "id": 7,
            "alert_id": "muster_unit_damaged",
            "tick": 19,
            "unit": 41,
            "x": 12.0,
            "y": 9.0,
        })]));
        let root = world
            .spawn((
                ComputedNode {
                    size: Vec2::splat(200.0),
                    inverse_scale_factor: 1.0,
                    ..default()
                },
                MinimapViewConfig {
                    unit_size: 4.0,
                    frame_colour: Some(Color::WHITE),
                    ping_colour: Some(Color::srgb_u8(255, 204, 0)),
                },
            ))
            .id();

        sync_minimap(&mut world);

        let children = world
            .get::<MinimapChildren>(root)
            .expect("minimap draw state");
        let [horizontal, vertical] = children.pings.get(&7).expect("rendered alert ping");
        let horizontal_node = world.get::<Node>(*horizontal).expect("horizontal ping arm");
        let vertical_node = world.get::<Node>(*vertical).expect("vertical ping arm");
        assert_eq!(horizontal_node.left, Val::Px(32.5));
        assert_eq!(horizontal_node.top, Val::Px(36.5));
        assert_eq!(vertical_node.left, Val::Px(36.5));
        assert_eq!(vertical_node.top, Val::Px(32.5));
        assert_eq!(
            world.get::<BackgroundColor>(*horizontal).unwrap().0,
            Color::srgb_u8(255, 204, 0)
        );
        assert_eq!(
            world.get::<BackgroundColor>(*vertical).unwrap().0,
            Color::srgb_u8(255, 204, 0)
        );
    }
}
