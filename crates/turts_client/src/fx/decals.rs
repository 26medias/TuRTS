//! At most 128 ground pictures. They have no collision or simulation component.
use super::*;
use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::MeshTag,
};
#[derive(Resource, Default)]
pub struct DecalInstances {
    entities: HashMap<u64, Entity>,
}
#[allow(clippy::too_many_arguments)]
pub fn draw_decals(
    mut commands: Commands,
    emitters: Res<emitters::Emitters>,
    mut state: ResMut<DecalInstances>,
    mut pool: ResMut<particles::ParticlePool>,
    mut materials: ResMut<Assets<particles::ParticleMaterial>>,
) {
    let Some(mesh) = pool.mesh.clone() else {
        return;
    };
    let live: HashSet<_> = emitters.decals.iter().map(|decal| decal.id).collect();
    state.entities.retain(|id, entity| {
        if live.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    for decal in &emitters.decals {
        if state.entities.contains_key(&decal.id) {
            continue;
        }
        let material = pool.material(decal.image.clone(), false, &mut materials);
        let entity = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                MeshTag::new(8192 + (decal.id % 128) as u32),
                Transform {
                    translation: Vec3::new(decal.position.x, 0.025, decal.position.z),
                    rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    scale: Vec3::splat(decal.size),
                },
                NotShadowCaster,
                NotShadowReceiver,
            ))
            .id();
        state.entities.insert(decal.id, entity);
    }
}
