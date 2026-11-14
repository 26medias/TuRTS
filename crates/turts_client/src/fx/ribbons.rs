//! Shared meshes draw projectile trails and emergency information sprites.
use super::{emitters::Emitters, *};
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    material::AlphaMode,
    mesh::{Indices, PrimitiveTopology},
};
struct RibbonMesh {
    mesh: Handle<Mesh>,
    entity: Entity,
    visible: bool,
}
#[derive(Resource, Default)]
pub struct Ribbons {
    batches: HashMap<(Option<String>, bool), RibbonMesh>,
    dot: Option<Handle<Image>>,
    source: String,
}
#[allow(clippy::too_many_arguments)]
pub fn draw_ribbons(
    mut commands: Commands,
    fx: Res<CombatPresentation>,
    emitters: Res<Emitters>,
    pool: Res<super::particles::ParticlePool>,
    acquired: Option<Res<AcquiredPackages>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut state: ResMut<Ribbons>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let source = acquired.as_ref().map_or("", |a| a.0.lockfile_hash.as_str());
    if state.source != source {
        for batch in state.batches.values() {
            commands.entity(batch.entity).despawn();
        }
        state.batches.clear();
        state.source = source.to_owned();
    }
    let active: HashSet<_> = emitters
        .lines
        .iter()
        .filter(|line| fx.now >= line.starts.max(line.visible_at) && fx.now < line.until)
        .map(|line| (line.image.clone(), line.additive))
        .collect();
    for (key, batch) in &mut state.batches {
        if !active.contains(key) && batch.visible {
            commands.entity(batch.entity).insert(Visibility::Hidden);
            batch.visible = false;
        }
    }
    for key in active {
        let right = camera.rotation() * Vec3::X;
        let up = camera.rotation() * Vec3::Y;
        let mut positions = Vec::with_capacity(emitters.lines.len() * 4);
        let mut colours = Vec::with_capacity(emitters.lines.len() * 4);
        let mut uvs = Vec::with_capacity(emitters.lines.len() * 4);
        let mut indices = Vec::with_capacity(emitters.lines.len() * 6);
        for line in emitters
            .lines
            .iter()
            .filter(|line| line.image == key.0 && line.additive == key.1)
        {
            if fx.now < line.starts.max(line.visible_at) || fx.now >= line.until {
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
                        actor.position + Vec3::Y * actor.height * 0.5 + line.target_offset
                    });
                let point = |phase: f32| {
                    line.source.lerp(target, phase)
                        + Vec3::Y * (4. * line.arc * phase * (1. - phase))
                };
                (point(tail), point(t), 1.)
            } else {
                (line.source, line.target, 1. - t)
            };
            let start_colour = line.colour.to_linear().to_vec4();
            let colour = line
                .end_colour
                .map_or(start_colour * Vec4::new(1., 1., 1., alpha), |end| {
                    start_colour.lerp(end.to_linear().to_vec4(), t)
                });
            let width = line.width + (line.end_width.unwrap_or(line.width) - line.width) * t;
            let direction = end - start;
            let (a, b, c, d) = if direction.length_squared() < 0.00001 {
                let x = right * width * 0.5;
                let y = up * width * 0.5;
                (end - x - y, end + x - y, end + x + y, end - x + y)
            } else {
                let side = direction
                    .cross(camera.translation() - (start + end) * 0.5)
                    .normalize_or(right)
                    * width
                    * 0.5;
                (start - side, start + side, end + side, end - side)
            };
            let base = positions.len() as u32;
            positions.extend([a.to_array(), b.to_array(), c.to_array(), d.to_array()]);
            colours.extend([colour.to_array(); 4]);
            uvs.extend([[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        if !state.batches.contains_key(&key) {
            let mesh = meshes.add(Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            ));
            let dot = state
                .dot
                .get_or_insert_with(|| images.add(super::particles::dot_image()))
                .clone();
            let image = key
                .0
                .as_ref()
                .and_then(|id| pool.images.get(id))
                .cloned()
                .unwrap_or(dot);
            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(image),
                unlit: true,
                alpha_mode: if key.1 {
                    AlphaMode::Add
                } else {
                    AlphaMode::Blend
                },
                cull_mode: None,
                ..default()
            });
            let entity = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    NotShadowCaster,
                    NotShadowReceiver,
                    bevy::camera::visibility::NoFrustumCulling,
                ))
                .id();
            state.batches.insert(
                key.clone(),
                RibbonMesh {
                    mesh,
                    entity,
                    visible: true,
                },
            );
        }
        let batch = state.batches.get_mut(&key).unwrap();
        if !batch.visible {
            commands.entity(batch.entity).insert(Visibility::Visible);
            batch.visible = true;
        }
        if let Some(mut mesh) = meshes.get_mut(&batch.mesh) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            let count = mesh.count_vertices();
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; count]);
            mesh.insert_indices(Indices::U32(indices));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_volley_hides_the_shared_mesh_then_reuses_it() {
        let mut app = App::new();
        app.init_resource::<CombatPresentation>()
            .init_resource::<Emitters>()
            .init_resource::<Ribbons>()
            .init_resource::<super::super::particles::ParticlePool>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(Update, draw_ribbons);
        app.world_mut()
            .spawn((Camera3d::default(), GlobalTransform::IDENTITY));
        app.world_mut()
            .resource_mut::<Emitters>()
            .lines
            .push_back(super::super::emitters::Line {
                source: Vec3::ZERO,
                target: Vec3::X,
                colour: Color::WHITE,
                starts: 0.,
                until: 1.,
                width: 0.1,
                projectile: false,
                arc: 0.,
                trail_seconds: 0.,
                target_unit: None,
                ..default()
            });
        app.update();
        let entity = app
            .world()
            .resource::<Ribbons>()
            .batches
            .values()
            .next()
            .unwrap()
            .entity;
        let mesh = app
            .world()
            .resource::<Ribbons>()
            .batches
            .values()
            .next()
            .unwrap()
            .mesh
            .clone();
        assert_eq!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&mesh)
                .unwrap()
                .count_vertices(),
            4
        );
        app.world_mut().resource_mut::<CombatPresentation>().now = 2.;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&mesh)
                .unwrap()
                .count_vertices(),
            4
        );
        app.world_mut().resource_mut::<Emitters>().lines[0].until = 3.;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Visible)
        );
        assert_eq!(
            app.world()
                .resource::<Ribbons>()
                .batches
                .values()
                .next()
                .unwrap()
                .entity,
            entity
        );
    }
    #[test]
    fn line_batches_apply_authored_colour_width_image_and_blend() {
        let mut app = App::new();
        app.init_resource::<CombatPresentation>()
            .init_resource::<Emitters>()
            .init_resource::<Ribbons>()
            .init_resource::<super::super::particles::ParticlePool>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(Update, draw_ribbons);
        app.world_mut().spawn((
            Camera3d::default(),
            GlobalTransform::from_translation(Vec3::Z * 5.),
        ));
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.world_mut()
            .resource_mut::<super::super::particles::ParticlePool>()
            .images
            .insert("test:stripe".into(), image.clone());
        let line = super::super::emitters::Line {
            source: Vec3::ZERO,
            target: Vec3::X,
            colour: Color::linear_rgba(1., 0., 0., 1.),
            end_colour: Some(Color::linear_rgba(0., 0., 1., 0.5)),
            starts: 0.,
            until: 2.,
            width: 1.,
            end_width: Some(3.),
            image: Some("test:stripe".into()),
            ..default()
        };
        app.world_mut().resource_mut::<Emitters>().lines.extend([
            line.clone(),
            super::super::emitters::Line {
                additive: true,
                ..line
            },
        ]);
        app.world_mut().resource_mut::<CombatPresentation>().now = 1.;
        app.update();
        let state = app.world().resource::<Ribbons>();
        assert_eq!(state.batches.len(), 2);
        for (key, batch) in &state.batches {
            let mesh = app
                .world()
                .resource::<Assets<Mesh>>()
                .get(&batch.mesh)
                .unwrap();
            let bevy::mesh::VertexAttributeValues::Float32x3(points) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!("positions");
            };
            assert!(
                (Vec3::from_array(points[0]).distance(Vec3::from_array(points[1])) - 2.).abs()
                    < 0.00001
            );
            let bevy::mesh::VertexAttributeValues::Float32x4(colours) =
                mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap()
            else {
                panic!("colours");
            };
            assert_eq!(colours[0], [0.5, 0., 0.5, 0.75]);
            let handle = app
                .world()
                .get::<MeshMaterial3d<StandardMaterial>>(batch.entity)
                .unwrap();
            let material = app
                .world()
                .resource::<Assets<StandardMaterial>>()
                .get(&handle.0)
                .unwrap();
            assert_eq!(
                material.alpha_mode,
                if key.1 {
                    AlphaMode::Add
                } else {
                    AlphaMode::Blend
                }
            );
            assert_eq!(material.base_color_texture.as_ref(), Some(&image));
        }
    }
}
