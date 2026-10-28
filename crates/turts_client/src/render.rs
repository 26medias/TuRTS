//! Rendering.
//!
//! The scene is built from the [`Presented`] record and the ClientContent shapes. It reads the
//! `Presented` resource and nothing else about the world: the mesh of a unit is never the
//! authority for its position (design 4.2, spec 4.1). The window path only: the headless client
//! has no `Assets<Mesh>`, so `build_app` adds [`UnitRenderPlugin`] in the window branch.
//!
//! Coordinates: the map is on the ground plane with `x` right and `y` forward. The scene has `y`
//! up, so a map `(x, y)` is the scene point `(x, 0, y)` (design 4.4).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use turts_mod_api::{ContentId, Footprint, Shape};
use turts_net_protocol::Pos;

use crate::economy_ui::UiPanelRoot;
use crate::ingest::ReplicaState;
use crate::join::{AcquiredPackages, Phase};
use crate::present::{Presented, update_presented};

const CAMERA_TILT_UP: f32 = 0.8;
const CAMERA_TILT_BACK: f32 = 0.6;
const SCENE_MARGIN: f32 = 12.0;
const CAMERA_MAX_FIT_ITERATIONS: usize = 96;
/// The camera stands this high above the ground in the initial scene.
const CAMERA_HEIGHT: f32 = 40.0;
/// The camera stands this far behind the centre of the map in the initial scene.
const CAMERA_BACK: f32 = 30.0;

/// The scene point of a map position, on the ground.
pub fn world_to_scene(pos: Pos) -> Vec3 {
    Vec3::new(pos.x, 0.0, pos.y)
}

/// Project a map-ground point into the active camera's logical viewport coordinates.
///
/// The returned coordinate uses a top-left origin in the camera target's logical pixel space.
/// Points outside the camera frustum or viewport have no selectable screen center.
pub fn projected_ground_center(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    pos: Pos,
) -> Option<Vec2> {
    if !camera.is_active {
        return None;
    }
    let center = camera
        .world_to_viewport(camera_transform, world_to_scene(pos))
        .ok()?;
    let viewport = camera.logical_viewport_rect()?;
    (center.is_finite() && viewport.contains(center)).then_some(center)
}

/// Declared unit pick radius from the accepted package catalog.
pub fn declared_unit_pick_radius(
    content: &crate::content::ClientContent,
    content_id: &str,
) -> Option<f32> {
    let id = ContentId::parse(content_id).ok()?;
    let definition = content.catalog.units.get(&id)?;
    match &definition.footprint {
        Footprint::Circle { radius } if radius.is_finite() && *radius > 0.0 => Some(*radius),
        _ => None,
    }
}

/// Declared node pick radius from the accepted package catalog.
pub fn declared_node_pick_radius(
    content: &crate::content::ClientContent,
    content_id: &str,
) -> Option<f32> {
    let id = ContentId::parse(content_id).ok()?;
    let radius = content.catalog.nodes.get(&id)?.radius;
    (radius.is_finite() && radius > 0.0).then_some(radius)
}

/// The greatest valid ground pick radius declared by the accepted client catalog.
pub fn max_declared_pick_radius(content: &crate::content::ClientContent) -> f32 {
    let unit_radius = content
        .catalog
        .units
        .values()
        .filter_map(|definition| match &definition.footprint {
            Footprint::Circle { radius } if radius.is_finite() && *radius > 0.0 => Some(*radius),
            _ => None,
        })
        .fold(0.0_f32, f32::max);
    content
        .catalog
        .nodes
        .values()
        .map(|definition| definition.radius)
        .filter(|radius| radius.is_finite() && *radius > 0.0)
        .fold(unit_radius, f32::max)
}

/// The part of a full-window camera that is not covered by the measured economy panel.
pub fn free_scene_rect(viewport_size: Vec2, panel_rect: Option<Rect>, margin: f32) -> Option<Rect> {
    if !viewport_size.is_finite()
        || viewport_size.min_element() <= 0.0
        || !margin.is_finite()
        || margin < 0.0
    {
        return None;
    }
    let viewport = Rect::from_corners(Vec2::ZERO, viewport_size);
    let panel = panel_rect?;
    if !panel.min.is_finite() || !panel.max.is_finite() {
        return None;
    }
    let rect = if panel.min.y > viewport_size.y * 0.5 {
        Rect::from_corners(
            Vec2::splat(margin),
            Vec2::new(viewport_size.x - margin, panel.min.y - margin),
        )
    } else {
        Rect::from_corners(
            Vec2::new(panel.max.x + margin, margin),
            Vec2::new(viewport_size.x - margin, viewport_size.y - margin),
        )
    };
    (rect.min.x >= viewport.min.x
        && rect.min.y >= viewport.min.y
        && rect.max.x <= viewport.max.x
        && rect.max.y <= viewport.max.y
        && rect.min.x < rect.max.x
        && rect.min.y < rect.max.y)
        .then_some(rect)
}

/// Fit the complete declared-pick-radius-expanded ground map into a free physical-pixel rect.
pub fn fit_camera_transform(
    map_size: (f32, f32),
    pick_radius: f32,
    viewport_size: Vec2,
    free_rect: Rect,
    fov: f32,
    near: f32,
) -> Option<Transform> {
    let (map_width, map_depth) = map_size;
    if !map_width.is_finite()
        || !map_depth.is_finite()
        || map_width <= 0.0
        || map_depth <= 0.0
        || !pick_radius.is_finite()
        || pick_radius < 0.0
        || !viewport_size.is_finite()
        || viewport_size.min_element() <= 0.0
        || !free_rect.min.is_finite()
        || !free_rect.max.is_finite()
        || free_rect.min.x < 0.0
        || free_rect.min.y < 0.0
        || free_rect.max.x > viewport_size.x
        || free_rect.max.y > viewport_size.y
        || free_rect.min.x >= free_rect.max.x
        || free_rect.min.y >= free_rect.max.y
        || !fov.is_finite()
        || fov <= 0.0
        || fov >= std::f32::consts::PI
        || !near.is_finite()
        || near <= 0.0
    {
        return None;
    }

    let focal = viewport_size.y / (2.0 * (fov * 0.5).tan());
    if !focal.is_finite() || focal <= 0.0 {
        return None;
    }
    let screen_center = viewport_size * 0.5;
    let x_min = -pick_radius;
    let x_max = map_width + pick_radius;
    let z_min = -pick_radius;
    let z_max = map_depth + pick_radius;
    let t_top = (free_rect.min.y - screen_center.y) / focal;
    let t_bottom = (free_rect.max.y - screen_center.y) / focal;
    let bottom_denominator = CAMERA_TILT_UP + CAMERA_TILT_BACK * t_bottom;
    let top_denominator = CAMERA_TILT_UP + CAMERA_TILT_BACK * t_top;
    if bottom_denominator <= 0.0 || top_denominator <= 0.0 {
        return None;
    }

    let mut distance = map_width.max(map_depth).max(1.0);
    for _ in 0..CAMERA_MAX_FIT_ITERATIONS {
        let cz_low = z_max - t_bottom * distance / bottom_denominator;
        let cz_high = z_min - t_top * distance / top_denominator;
        if cz_low <= cz_high {
            let center_z = (cz_low + cz_high) * 0.5;
            let mut cx_low = f32::NEG_INFINITY;
            let mut cx_high = f32::INFINITY;
            let corners = [
                (x_min, z_min),
                (x_max, z_min),
                (x_min, z_max),
                (x_max, z_max),
            ];
            let mut all_in_front = true;
            for (x, z) in corners {
                let depth = distance - CAMERA_TILT_BACK * (z - center_z);
                if !depth.is_finite() || depth <= near {
                    all_in_front = false;
                    break;
                }
                cx_low = cx_low.max(x - (free_rect.max.x - screen_center.x) * depth / focal);
                cx_high = cx_high.min(x - (free_rect.min.x - screen_center.x) * depth / focal);
            }
            if all_in_front && cx_low <= cx_high {
                let center_x = (cx_low + cx_high) * 0.5;
                let inside = corners.into_iter().all(|(x, z)| {
                    let depth = distance - CAMERA_TILT_BACK * (z - center_z);
                    let point = Vec2::new(
                        screen_center.x + focal * (x - center_x) / depth,
                        screen_center.y + focal * CAMERA_TILT_UP * (z - center_z) / depth,
                    );
                    point.is_finite()
                        && point.x >= free_rect.min.x - 0.5
                        && point.x <= free_rect.max.x + 0.5
                        && point.y >= free_rect.min.y - 0.5
                        && point.y <= free_rect.max.y + 0.5
                });
                if inside {
                    let target = Vec3::new(center_x, 0.0, center_z);
                    let eye = target
                        + Vec3::new(0.0, CAMERA_TILT_UP * distance, CAMERA_TILT_BACK * distance);
                    return Some(Transform::from_translation(eye).looking_at(target, Vec3::Y));
                }
            }
        }
        distance *= 1.06;
        if !distance.is_finite() {
            return None;
        }
    }
    None
}

fn panel_screen_rect(node: &ComputedNode, transform: &UiGlobalTransform) -> Option<Rect> {
    let size = node.size();
    let center = transform.to_scale_angle_translation().2;
    (size.is_finite() && size.min_element() > 0.0 && center.is_finite())
        .then(|| Rect::from_center_half_size(center, size * 0.5))
}

fn fit_scene_camera(
    acquired: Option<Res<AcquiredPackages>>,
    panels: Query<(&ComputedNode, &UiGlobalTransform), With<UiPanelRoot>>,
    mut cameras: Query<(&Camera, &Projection, &mut Transform), With<Camera3d>>,
) {
    let Some(acquired) = acquired else { return };
    let Some((panel_node, panel_transform)) = panels.iter().next() else {
        return;
    };
    let Some(panel_rect) = panel_screen_rect(panel_node, panel_transform) else {
        return;
    };
    for (camera, projection, mut transform) in &mut cameras {
        let Some(viewport) = camera.physical_viewport_rect() else {
            continue;
        };
        let viewport_size = viewport.size().as_vec2();
        let panel_rect = Rect::from_corners(
            panel_rect.min - viewport.min.as_vec2(),
            panel_rect.max - viewport.min.as_vec2(),
        );
        let margin = SCENE_MARGIN / panel_node.inverse_scale_factor.max(f32::MIN_POSITIVE);
        let Some(free_rect) = free_scene_rect(viewport_size, Some(panel_rect), margin) else {
            continue;
        };
        let Projection::Perspective(perspective) = projection else {
            continue;
        };
        if let Some(fitted) = fit_camera_transform(
            acquired.0.map_size,
            max_declared_pick_radius(&acquired.0.content),
            viewport_size,
            free_rect,
            perspective.fov,
            perspective.near,
        ) {
            *transform = fitted;
        }
    }
}

/// The mesh of a visual shape. The mesh is centred on its own origin.
///
/// A capsule `height` is the full height, so the cylinder between the caps is
/// `height - 2 * radius`. A cuboid `z` is its height in the map, so it is the scene `y`; a
/// cuboid `y` lies on the ground plane, so it is the scene `z`.
pub fn mesh_for(shape: &Shape) -> Mesh {
    match *shape {
        Shape::Capsule { radius, height } => Capsule3d::new(radius, height - 2.0 * radius).into(),
        Shape::Cuboid { x, y, z } => Cuboid::new(x, z, y).into(),
    }
}

/// How far the mesh origin is above the ground, so that the mesh stands on the ground: half the
/// height of the shape.
pub fn mesh_lift(shape: &Shape) -> f32 {
    match *shape {
        Shape::Capsule { height, .. } => height / 2.0,
        Shape::Cuboid { z, .. } => z / 2.0,
    }
}

/// Marks the entity that shows one unit.
#[derive(Component)]
pub struct PresentedUnit(pub u32);

/// Public surface resource node mesh, keyed by its stable protocol id.
#[derive(Component)]
pub struct PresentedNode(pub u32);

/// The mesh and lift of each shown shape, by content id, and the one material of all units.
#[derive(Default)]
pub struct UnitAssets {
    meshes: HashMap<String, (Handle<Mesh>, f32)>,
    material: Option<Handle<StandardMaterial>>,
}

/// `Update`: on the first frame in `Phase::Joined`, spawn the camera, the light and the ground.
pub fn setup_scene(
    mut commands: Commands,
    phase: Res<Phase>,
    acquired: Option<Res<AcquiredPackages>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut done: Local<bool>,
) {
    if *done || !matches!(*phase, Phase::Joined { .. }) {
        return;
    }
    let Some(acquired) = acquired else { return };
    *done = true;
    let (width, depth) = acquired.0.map_size;
    let centre = Vec3::new(width / 2.0, 0.0, depth / 2.0);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(centre + Vec3::new(0.0, CAMERA_HEIGHT, CAMERA_BACK))
            .looking_at(centre, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(-20.0, 60.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(width, depth))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.4, 0.25))),
        Transform::from_translation(centre),
    ));
}

/// `Update`, after [`update_presented`]: one entity per presented unit. A new unit gets an
/// entity with the mesh of its visual. A shown unit moves to its presented position. A unit that
/// is gone from the record loses its entity. The entities never write back to the replica.
#[allow(clippy::too_many_arguments)]
pub fn sync_units(
    mut commands: Commands,
    presented: Res<Presented>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shown: Query<(Entity, &PresentedUnit, &mut Transform)>,
    mut assets: Local<UnitAssets>,
) {
    let Some(acquired) = acquired else { return };
    let mut entities: HashMap<u32, Entity> = HashMap::new();
    for (entity, unit, _) in &shown {
        entities.insert(unit.0, entity);
    }
    let mut kept: HashSet<u32> = HashSet::new();
    for unit in &presented.units {
        kept.insert(unit.unit_id);
        let position = world_to_scene(unit.pos);
        if let Some(&entity) = entities.get(&unit.unit_id) {
            if let Ok((_, _, mut transform)) = shown.get_mut(entity) {
                let lift = assets
                    .meshes
                    .get(&unit.content_id)
                    .map_or(0.0, |&(_, lift)| lift);
                transform.translation = position + Vec3::Y * lift;
                if let Some(facing) = state
                    .replica
                    .as_ref()
                    .and_then(|replica| replica.units.get(&unit.unit_id))
                    .and_then(|unit| unit.facing)
                {
                    transform.rotation = facing_rotation(facing);
                }
            }
            continue;
        }
        let Some(shape) = acquired.0.content.shapes.get(&unit.content_id) else {
            continue;
        };
        let (mesh, lift) = assets
            .meshes
            .entry(unit.content_id.clone())
            .or_insert_with(|| (meshes.add(mesh_for(shape)), mesh_lift(shape)))
            .clone();
        let material = assets
            .material
            .get_or_insert_with(|| materials.add(Color::srgb(0.8, 0.3, 0.2)))
            .clone();
        let rotation = state
            .replica
            .as_ref()
            .and_then(|replica| replica.units.get(&unit.unit_id))
            .and_then(|unit| unit.facing)
            .map(facing_rotation)
            .unwrap_or(Quat::IDENTITY);
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform {
                translation: position + Vec3::Y * lift,
                rotation,
                ..default()
            },
            PresentedUnit(unit.unit_id),
        ));
    }
    for (unit_id, entity) in entities {
        if !kept.contains(&unit_id) {
            commands.entity(entity).despawn();
        }
    }
}

/// Convert authored map facing to a visual yaw. Missing facing keeps the shape's default pose.
pub fn facing_rotation(facing: turts_net_protocol::Facing) -> Quat {
    let yaw = match facing {
        turts_net_protocol::Facing::PosY => 0.0,
        turts_net_protocol::Facing::PosX => std::f32::consts::FRAC_PI_2,
        turts_net_protocol::Facing::NegY => std::f32::consts::PI,
        turts_net_protocol::Facing::NegX => -std::f32::consts::FRAC_PI_2,
    };
    Quat::from_rotation_y(yaw)
}

/// The window path of presentation: the scene and the unit entities.
pub struct UnitRenderPlugin;

impl Plugin for UnitRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (setup_scene, sync_units, sync_nodes, sync_world_names)
                .chain()
                .after(update_presented),
        );
        app.add_systems(
            PostUpdate,
            fit_scene_camera
                .after(bevy::ui::UiSystems::Layout)
                .before(bevy::transform::TransformSystems::Propagate),
        );
    }
}

#[derive(Default)]
pub struct NodeAssets {
    meshes: HashMap<String, (Handle<Mesh>, f32)>,
    material: Option<Handle<StandardMaterial>>,
}

/// Public nodes are rendered from the coherent replica and downloaded shape catalog.
pub fn sync_nodes(
    mut commands: Commands,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shown: Query<(Entity, &PresentedNode, &mut Transform)>,
    mut assets: Local<NodeAssets>,
) {
    let Some(replica) = state.replica.as_ref() else {
        return;
    };
    let Some(acquired) = acquired else {
        return;
    };
    let mut entities = HashMap::new();
    for (entity, node, _) in &shown {
        entities.insert(node.0, entity);
    }
    let mut kept = HashSet::new();
    for node in replica.nodes.values() {
        kept.insert(node.id.0);
        let pos = world_to_scene(node.pos);
        if let Some(&entity) = entities.get(&node.id.0) {
            if let Ok((_, _, mut transform)) = shown.get_mut(entity) {
                let lift = assets
                    .meshes
                    .get(&node.content_id)
                    .map_or(0.0, |(_, lift)| *lift);
                transform.translation = pos + Vec3::Y * lift;
            }
            continue;
        }
        let Some(shape) = acquired.0.content.shapes.get(&node.content_id) else {
            continue;
        };
        let (mesh, lift) = assets
            .meshes
            .entry(node.content_id.clone())
            .or_insert_with(|| (meshes.add(mesh_for(shape)), mesh_lift(shape)))
            .clone();
        let material = assets
            .material
            .get_or_insert_with(|| materials.add(Color::srgb(0.72, 0.62, 0.31)))
            .clone();
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(pos + Vec3::Y * lift),
            PresentedNode(node.id.0),
        ));
    }
    for (id, entity) in entities {
        if !kept.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
}

/// Read-only names for principal machines, mobile units and resource nodes.
/// Tiny transport pieces are named when selected to avoid overlapping belt labels.
#[derive(Component)]
struct WorldName(String);

#[allow(clippy::too_many_arguments)]
fn sync_world_names(
    mut commands: Commands,
    presented: Res<Presented>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    selection: Res<crate::economy_ui::Selection>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(Entity, &WorldName, &mut Node, &mut Text)>,
) {
    let (Some(acquired), Some(replica), Some((camera, transform))) =
        (acquired, state.replica.as_ref(), cameras.iter().next())
    else {
        return;
    };
    let mut desired = HashMap::new();
    for unit in &presented.units {
        let Some(def) = ContentId::parse(&unit.content_id)
            .ok()
            .and_then(|id| acquired.0.content.catalog.units.get(&id))
        else {
            continue;
        };
        if def.research.is_none()
            && def.movement.is_none()
            && def.machine.is_none()
            && def.weapon.is_none()
            && def.extractor.is_none()
            && def.producer.is_none()
            && selection.0 != Some(unit.unit_id)
        {
            continue;
        }
        let lift = acquired
            .0
            .content
            .shapes
            .get(&unit.content_id)
            .map_or(1.0, mesh_lift);
        if let Ok(point) =
            camera.world_to_viewport(transform, world_to_scene(unit.pos) + Vec3::Y * (lift + 1.8))
        {
            desired.insert(format!("unit:{}", unit.unit_id), (def.name.clone(), point));
        }
    }
    for node in replica.nodes.values() {
        let Some(def) = ContentId::parse(&node.content_id)
            .ok()
            .and_then(|id| acquired.0.content.catalog.nodes.get(&id))
        else {
            continue;
        };
        if let Ok(point) =
            camera.world_to_viewport(transform, world_to_scene(node.pos) + Vec3::Y * 1.5)
        {
            desired.insert(format!("node:{}", node.id.0), (def.name.clone(), point));
        }
    }
    for (entity, name, mut node, mut text) in &mut labels {
        if let Some((label, point)) = desired.remove(&name.0) {
            node.left = px(point.x - 45.0);
            node.top = px(point.y - 14.0);
            text.0 = label;
        } else {
            commands.entity(entity).despawn();
        }
    }
    for (key, (label, point)) in desired {
        commands.spawn((
            WorldName(key),
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::WHITE),
            BackgroundColor(Color::srgba(0.03, 0.05, 0.07, 0.9)),
            Node {
                position_type: PositionType::Absolute,
                left: px(point.x - 45.0),
                top: px(point.y - 14.0),
                padding: UiRect::axes(px(4), px(2)),
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
        ));
    }
}
