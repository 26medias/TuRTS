//! One shared mesh draws projectile trails and emergency information sprites.
use super::{emitters::Emitters, *};
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    material::AlphaMode,
    mesh::{Indices, PrimitiveTopology},
};
#[derive(Resource, Default)]
pub struct Ribbons {
    mesh: Option<Handle<Mesh>>,
    entity: Option<Entity>,
}
#[allow(clippy::too_many_arguments)]
pub fn draw_ribbons(
    mut commands: Commands,
    fx: Res<CombatPresentation>,
    emitters: Res<Emitters>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut state: ResMut<Ribbons>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let right = camera.rotation() * Vec3::X;
    let up = camera.rotation() * Vec3::Y;
    let mut positions = Vec::with_capacity(emitters.lines.len() * 4);
    let mut colours = Vec::with_capacity(emitters.lines.len() * 4);
    let mut uvs = Vec::with_capacity(emitters.lines.len() * 4);
    let mut indices = Vec::with_capacity(emitters.lines.len() * 6);
    for line in &emitters.lines {
        if fx.now < line.starts || fx.now >= line.until {
            continue;
        }
        let duration = (line.until - line.starts).max(0.001);
        let t = ((fx.now - line.starts) / duration).clamp(0., 1.) as f32;
        let (start, end, alpha) = if line.projectile {
            let tail = (t - line.trail_seconds / duration as f32).max(0.);
            let target = line
                .target_unit
                .and_then(|id| fx.actors.get(&id))
                .map_or(line.target, |actor| {
                    actor.position + Vec3::Y * actor.height * 0.5
                });
            let point = |phase: f32| {
                line.source.lerp(target, phase) + Vec3::Y * (4. * line.arc * phase * (1. - phase))
            };
            (point(tail), point(t), 1.)
        } else {
            (line.source, line.target, 1. - t)
        };
        let colour = line.colour.to_linear().to_vec4() * Vec4::new(1., 1., 1., alpha);
        let direction = end - start;
        let (a, b, c, d) = if direction.length_squared() < 0.00001 {
            let x = right * line.width * 0.5;
            let y = up * line.width * 0.5;
            (end - x - y, end + x - y, end + x + y, end - x + y)
        } else {
            let side = direction
                .cross(camera.translation() - (start + end) * 0.5)
                .normalize_or(right)
                * line.width
                * 0.5;
            (start - side, start + side, end + side, end - side)
        };
        let base = positions.len() as u32;
        positions.extend([a.to_array(), b.to_array(), c.to_array(), d.to_array()]);
        colours.extend([colour.to_array(); 4]);
        uvs.extend([[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    if state.mesh.is_none() {
        let mesh = meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        ));
        let dot = images.add(super::particles::dot_image());
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(dot),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        });
        state.entity = Some(
            commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    NotShadowCaster,
                    NotShadowReceiver,
                    bevy::camera::visibility::NoFrustumCulling,
                ))
                .id(),
        );
        state.mesh = Some(mesh);
    }
    if let Some(mut mesh) = state.mesh.as_ref().and_then(|mesh| meshes.get_mut(mesh)) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        let count = mesh.count_vertices();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; count]);
        mesh.insert_indices(Indices::U32(indices));
    }
}
