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
    fx: Res<CombatPresentation>,
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
        if fx.now < decal.starts || state.entities.contains_key(&decal.id) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_decal_is_not_drawn_until_its_start() {
        let mut app = App::new();
        app.init_resource::<emitters::Emitters>()
            .init_resource::<DecalInstances>()
            .init_resource::<particles::ParticlePool>()
            .init_resource::<Assets<particles::ParticleMaterial>>()
            .init_resource::<CombatPresentation>()
            .add_systems(Update, draw_decals);
        {
            let mut pool = app.world_mut().resource_mut::<particles::ParticlePool>();
            pool.mesh = Some(Handle::default());
            pool.dot = Some(Handle::default());
            pool.colours = Some(Handle::default());
        }
        app.world_mut()
            .resource_mut::<emitters::Emitters>()
            .decals
            .push_back(emitters::Decal {
                id: 1,
                starts: 0.5,
                until: 1.5,
                position: Vec3::ZERO,
                size: 1.,
                colour: Color::WHITE,
                image: None,
            });
        app.update();
        assert!(app.world().resource::<DecalInstances>().entities.is_empty());
        app.world_mut().resource_mut::<CombatPresentation>().now = 0.5;
        app.update();
        assert_eq!(app.world().resource::<DecalInstances>().entities.len(), 1);
    }
}
