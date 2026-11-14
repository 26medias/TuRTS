//! Optional projectile and debris models share glTF assets and quantized fading materials.
use super::*;
use bevy::{
    gltf::Gltf,
    material::AlphaMode,
    world_serialization::{WorldAssetRoot, WorldInstance, WorldInstanceSpawner},
};
#[derive(Component)]
pub(crate) struct OriginalMaterial(Handle<StandardMaterial>);
struct Picture {
    asset: String,
    entity: Entity,
}
#[derive(Resource, Default)]
pub struct ModelPictures {
    roots: HashMap<Entity, Picture>,
    pub ready: usize,
    materials: HashMap<(AssetId<StandardMaterial>, [u32; 4], bool), Handle<StandardMaterial>>,
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_model_particles(
    mut commands: Commands,
    pool: Res<particles::ParticlePool>,
    sources: Res<crate::models::ModelState>,
    gltfs: Res<Assets<Gltf>>,
    spawner: Option<Res<WorldInstanceSpawner>>,
    instances: Query<&WorldInstance>,
    pbr: Query<(&MeshMaterial3d<StandardMaterial>, Option<&OriginalMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pictures: ResMut<ModelPictures>,
) {
    let Some(spawner) = spawner else {
        return;
    };
    pictures.ready = 0;
    let mut live = HashSet::new();
    for slot in &pool.slots {
        let (Some(root), Some(particle)) = (slot.entity, slot.particle.as_ref()) else {
            continue;
        };
        let Some(asset) = particle.model.as_ref() else {
            continue;
        };
        live.insert(root);
        if pictures
            .roots
            .get(&root)
            .is_some_and(|picture| picture.asset != *asset)
            && let Some(old) = pictures.roots.remove(&root)
        {
            commands.entity(old.entity).despawn();
        }
        if let std::collections::hash_map::Entry::Vacant(entry) = pictures.roots.entry(root) {
            let Some(scene) = sources
                .effect_model(asset)
                .and_then(|handle| gltfs.get(&handle))
                .and_then(|gltf| gltf.default_scene.as_ref().or(gltf.scenes.first()))
                .cloned()
            else {
                continue;
            };
            let entity = commands
                .spawn((
                    WorldAssetRoot(scene),
                    Transform::IDENTITY,
                    Visibility::Inherited,
                    ChildOf(root),
                ))
                .id();
            entry.insert(Picture {
                asset: asset.clone(),
                entity,
            });
        }
        let picture = &pictures.roots[&root];
        let Ok(instance) = instances.get(picture.entity) else {
            continue;
        };
        if !spawner.instance_is_ready(**instance) {
            continue;
        }
        pictures.ready += 1;
        commands.entity(root).remove::<Mesh3d>();
        let phase = (((particle.age - particle.delay) / particle.life.max(0.001)).clamp(0., 1.)
            * 15.)
            .round()
            / 15.;
        let tint = particle
            .colour
            .to_vec4()
            .lerp(particle.end_colour.to_vec4(), phase);
        let key_colour = tint.to_array().map(f32::to_bits);
        for entity in spawner.iter_instance_entities(**instance) {
            let Ok((current, original)) = pbr.get(entity) else {
                continue;
            };
            let original =
                original.map_or_else(|| current.0.clone(), |original| original.0.clone());
            let key = (original.id(), key_colour, particle.additive);
            let material = if let Some(material) = pictures.materials.get(&key) {
                material.clone()
            } else {
                let Some(mut material) = materials.get(&original).cloned() else {
                    continue;
                };
                let colour = material.base_color.to_linear().to_vec4() * tint;
                material.base_color = Color::linear_rgba(colour.x, colour.y, colour.z, colour.w);
                material.alpha_mode = if particle.additive {
                    AlphaMode::Add
                } else {
                    AlphaMode::Blend
                };
                let handle = materials.add(material);
                pictures.materials.insert(key, handle.clone());
                handle
            };
            if current.0 != material {
                commands.entity(entity).insert((
                    MeshMaterial3d(material),
                    OriginalMaterial(original),
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ));
            }
        }
    }
    pictures.roots.retain(|root, picture| {
        if live.contains(root) {
            true
        } else {
            commands.entity(picture.entity).despawn();
            if let Some(mesh) = pool.mesh.as_ref() {
                commands.entity(*root).try_insert(Mesh3d(mesh.clone()));
            }
            false
        }
    });
}
