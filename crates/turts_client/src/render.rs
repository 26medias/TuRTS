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
use turts_mod_api::Shape;
use turts_net_protocol::Pos;

use crate::join::{AcquiredPackages, Phase};
use crate::present::{Presented, update_presented};

/// The camera stands this high above the ground.
const CAMERA_HEIGHT: f32 = 40.0;
/// The camera stands this far behind the centre of the map.
const CAMERA_BACK: f32 = 30.0;

/// The scene point of a map position, on the ground.
pub fn world_to_scene(pos: Pos) -> Vec3 {
    Vec3::new(pos.x, 0.0, pos.y)
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
pub fn sync_units(
    mut commands: Commands,
    presented: Res<Presented>,
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
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position + Vec3::Y * lift),
            PresentedUnit(unit.unit_id),
        ));
    }
    for (unit_id, entity) in entities {
        if !kept.contains(&unit_id) {
            commands.entity(entity).despawn();
        }
    }
}

/// The window path of presentation: the scene and the unit entities.
pub struct UnitRenderPlugin;

impl Plugin for UnitRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (setup_scene, sync_units).chain().after(update_presented),
        );
    }
}
