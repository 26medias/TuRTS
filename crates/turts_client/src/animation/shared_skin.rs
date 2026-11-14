//! Shared, anchor-relative skin palettes. The ordinary skin path remains the fallback.
use bevy::{
    mesh::{MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
    render::render_resource::VertexFormat,
};

pub const MAX_MATRICES: usize = 65_536;
pub const JOINTS: MeshVertexAttribute =
    MeshVertexAttribute::new("SharedSkinJoints", 32_104_001, VertexFormat::Uint16x4);
pub const WEIGHTS: MeshVertexAttribute =
    MeshVertexAttribute::new("SharedSkinWeights", 32_104_002, VertexFormat::Float32x4);

/// Custom attributes leave Bevy's ordinary per-instance skin extraction disabled.
/// Forward rendering uses locations 6 and 7. The prepass uses 5 and 6.
pub(super) fn prepare_mesh(source: &Mesh, joint_count: usize) -> Option<Mesh> {
    if source.primitive_topology() != PrimitiveTopology::TriangleList
        || source.try_has_morph_targets().ok()?
        || joint_count == 0
        || joint_count > MAX_MATRICES
    {
        return None;
    }
    let Some(VertexAttributeValues::Float32x3(_)) =
        source.try_attribute_option(Mesh::ATTRIBUTE_POSITION).ok()?
    else {
        return None;
    };
    let Some(VertexAttributeValues::Uint16x4(joints)) = source
        .try_attribute_option(Mesh::ATTRIBUTE_JOINT_INDEX)
        .ok()?
    else {
        return None;
    };
    let Some(VertexAttributeValues::Float32x4(weights)) = source
        .try_attribute_option(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        .ok()?
    else {
        return None;
    };
    if joints.len() != source.count_vertices()
        || weights.len() != joints.len()
        || joints
            .iter()
            .flatten()
            .any(|&j| usize::from(j) >= joint_count)
        || weights.iter().flatten().any(|w| !w.is_finite())
    {
        return None;
    }
    let mut mesh = source.clone();
    mesh.try_remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
        .ok()?;
    mesh.try_remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        .ok()?;
    mesh.insert_attribute(JOINTS, VertexAttributeValues::Uint16x4(joints.clone()));
    mesh.insert_attribute(WEIGHTS, weights.clone());
    Some(mesh)
}

pub(super) fn palette(anchor: Mat4, joints: &[Mat4], inverse_bind: &[Mat4]) -> Option<Vec<Mat4>> {
    if joints.is_empty() || joints.len() > MAX_MATRICES || joints.len() != inverse_bind.len() {
        return None;
    }
    let affine = |matrix: Mat4| {
        (matrix.is_finite() && matrix.row(3) == Vec4::W)
            .then(|| bevy::math::Affine3A::from_mat4(matrix))
    };
    let anchor = affine(anchor)?;
    let determinant = anchor.matrix3.determinant();
    if !determinant.is_finite() || determinant == 0. {
        return None;
    }
    let inverse = anchor.inverse();
    if !inverse.is_finite() {
        return None;
    }
    joints
        .iter()
        .zip(inverse_bind)
        .map(|(&joint, &bind)| {
            let matrix = Mat4::from(inverse * affine(joint)? * affine(bind)?);
            matrix.is_finite().then_some(matrix)
        })
        .collect()
}

pub(super) fn palette_tag(previous: u32, offset: usize) -> Option<u32> {
    let offset = u32::try_from(offset).ok()?;
    (offset <= u32::MAX >> 1).then_some((offset << 1) | (previous & 1))
}

use bevy::{
    ecs::system::SystemParam,
    gltf::GltfSkin,
    mesh::{
        MeshTag, MeshVertexBufferLayoutRef,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline},
    reflect::TypePath,
    render::{
        render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError},
        renderer::RenderDevice,
        storage::ShaderBuffer,
    },
    shader::{ShaderDefVal, ShaderRef},
};
use std::collections::HashMap;

#[derive(Resource, Clone)]
pub struct SharedSkinSettings {
    pub enabled: bool,
    pub matrix_limit: usize,
}
impl Default for SharedSkinSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            matrix_limit: MAX_MATRICES,
        }
    }
}

#[derive(Resource, Default, serde::Serialize)]
pub struct SharedSkinStats {
    pub enabled: bool,
    pub palettes: usize,
    pub matrices: usize,
    pub shared_draws: usize,
    pub ordinary_draws: usize,
}

pub(crate) type CrowdMaterial = ExtendedMaterial<StandardMaterial, CrowdExtension>;
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct CrowdExtension {
    #[storage(100, read_only)]
    pub matrices: Handle<ShaderBuffer>,
}
impl MaterialExtension for CrowdExtension {
    fn vertex_shader() -> ShaderRef {
        "embedded://turts_client/animation/crowd_vertex.wesl".into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        "embedded://turts_client/animation/crowd_prepass.wesl".into()
    }
    fn deferred_vertex_shader() -> ShaderRef {
        Self::prepass_vertex_shader()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://turts_client/fx/flash.wesl".into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        Self::fragment_shader()
    }
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let prepass = descriptor.vertex.shader_defs.iter().any(|definition| {
            matches!(definition, ShaderDefVal::Bool(name, true) if name == "PREPASS_PIPELINE")
        });
        let index_location = if prepass { 5 } else { 6 };
        let joints = layout.0.get_layout(&[
            JOINTS.at_shader_location(index_location),
            WEIGHTS.at_shader_location(index_location + 1),
        ])?;
        // SpecializedMeshPipeline guarantees one interleaved vertex buffer.
        descriptor.vertex.buffers[0]
            .attributes
            .extend(joints.attributes);
        // Reuse Bevy's vertex IO and decompression. This shader owns skinning and its buffer.
        descriptor.vertex.shader_defs.push("SKINNED".into());
        descriptor
            .vertex
            .shader_defs
            .push("VERTEX_OUTPUT_INSTANCE_INDEX".into());
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment
                .shader_defs
                .push("VERTEX_OUTPUT_INSTANCE_INDEX".into());
        }
        Ok(())
    }
}

#[derive(Resource, Default)]
pub(crate) struct SharedSkinAtlas {
    buffer: Handle<ShaderBuffer>,
    limit: usize,
    meshes: HashMap<(AssetId<Mesh>, usize), Option<Handle<Mesh>>>,
    materials: HashMap<AssetId<crate::fx::flash::ModelMaterial>, Handle<CrowdMaterial>>,
}

#[derive(SystemParam)]
pub(crate) struct SharedSkinAssets<'w> {
    settings: Res<'w, SharedSkinSettings>,
    atlas: ResMut<'w, SharedSkinAtlas>,
    materials: ResMut<'w, Assets<CrowdMaterial>>,
}

#[derive(Component)]
struct SharedSkin {
    player: Entity,
    anchor: Entity,
    anchor_node: usize,
    palette: AssetId<GltfSkin>,
    original_mesh: Handle<Mesh>,
    original_material: Handle<crate::fx::flash::ModelMaterial>,
    shared_mesh: Handle<Mesh>,
    shared_material: Handle<CrowdMaterial>,
    original_skin: SkinnedMesh,
    active: bool,
}

pub(super) struct PreparedSharedDraw {
    pub mesh: Handle<Mesh>,
    pub material: Handle<CrowdMaterial>,
    component: SharedSkin,
}
impl PreparedSharedDraw {
    pub fn install(self, commands: &mut Commands, entity: Entity) {
        commands.entity(entity).insert(self.component);
    }
}
impl SharedSkinAssets<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare(
        &mut self,
        player: Entity,
        anchor: Entity,
        anchor_node: usize,
        skin_handle: &Handle<GltfSkin>,
        skin: SkinnedMesh,
        original_mesh: &Handle<Mesh>,
        original_material: &Handle<crate::fx::flash::ModelMaterial>,
        meshes: &mut Assets<Mesh>,
        materials: &Assets<crate::fx::flash::ModelMaterial>,
    ) -> Option<PreparedSharedDraw> {
        if !self.settings.enabled || skin.joints.len() > self.atlas.limit || self.atlas.limit == 0 {
            return None;
        }
        let key = (original_mesh.id(), skin.joints.len());
        let mesh = self
            .atlas
            .meshes
            .entry(key)
            .or_insert_with(|| {
                meshes
                    .get(original_mesh)
                    .and_then(|mesh| prepare_mesh(mesh, skin.joints.len()))
                    .map(|mesh| meshes.add(mesh))
            })
            .as_ref()?
            .clone();
        let material = if let Some(material) = self.atlas.materials.get(&original_material.id()) {
            material.clone()
        } else {
            let original = materials.get(original_material)?;
            let material = self.materials.add(CrowdMaterial {
                base: original.base.clone(),
                extension: CrowdExtension {
                    matrices: self.atlas.buffer.clone(),
                },
            });
            self.atlas
                .materials
                .insert(original_material.id(), material.clone());
            material
        };
        Some(PreparedSharedDraw {
            mesh: mesh.clone(),
            material: material.clone(),
            component: SharedSkin {
                player,
                anchor,
                anchor_node,
                palette: skin_handle.id(),
                original_mesh: original_mesh.clone(),
                original_material: original_material.clone(),
                shared_mesh: mesh,
                shared_material: material,
                original_skin: skin,
                active: true,
            },
        })
    }
}

pub(crate) fn register(app: &mut App) {
    bevy::shader::load_shader_library!(app, "crowd_skin.wesl");
    bevy::shader::load_shader_library!(app, "crowd_vertex.wesl");
    bevy::shader::load_shader_library!(app, "crowd_prepass.wesl");
    app.add_plugins(MaterialPlugin::<CrowdMaterial>::default())
        .init_resource::<SharedSkinSettings>()
        .init_resource::<SharedSkinAtlas>()
        .init_resource::<SharedSkinStats>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            upload.after(bevy::transform::TransformSystems::Propagate),
        );
}

fn setup(
    settings: Res<SharedSkinSettings>,
    device: Option<Res<RenderDevice>>,
    mut atlas: ResMut<SharedSkinAtlas>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    atlas.limit = device.as_ref().map_or(0, |device| {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage == 0 {
            return 0;
        }
        (limits.max_storage_buffer_binding_size as usize / size_of::<Mat4>())
            .min(MAX_MATRICES)
            .min(settings.matrix_limit)
    });
    atlas.buffer = buffers.add(ShaderBuffer::from(vec![Mat4::IDENTITY]));
}

type PaletteKey = (Entity, AssetId<GltfSkin>, usize);
#[allow(clippy::too_many_arguments)]
fn upload(
    mut commands: Commands,
    settings: Res<SharedSkinSettings>,
    atlas: Res<SharedSkinAtlas>,
    poses: Res<super::SharedPoses>,
    transforms: Query<&GlobalTransform>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut draws: Query<(Entity, &mut SharedSkin, &mut MeshTag)>,
    ordinary: Query<(), With<SkinnedMesh>>,
    cameras: Query<&Camera, With<bevy::core_pipeline::prepass::MotionVectorPrepass>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    bridge: Res<crate::models::ModelGpuBridge>,
    mut stats: ResMut<SharedSkinStats>,
) {
    let enabled =
        settings.enabled && atlas.limit > 0 && !cameras.iter().any(|camera| camera.is_active);
    let mut offsets: HashMap<PaletteKey, Option<usize>> = HashMap::new();
    let mut matrices = Vec::new();
    let mut replacements = HashMap::new();
    *stats = SharedSkinStats {
        enabled,
        ordinary_draws: ordinary.iter().count(),
        ..default()
    };
    for (entity, mut draw, mut tag) in &mut draws {
        let key = poses
            .masters
            .get(&draw.player)
            .copied()
            .map(|master| (master, draw.palette, draw.anchor_node));
        let offset = if enabled {
            key.and_then(|key| {
                *offsets.entry(key).or_insert_with(|| {
                    let inverse_bind = bindposes.get(&draw.original_skin.inverse_bindposes)?;
                    let anchor = transforms.get(draw.anchor).ok()?.to_matrix();
                    let joints = draw
                        .original_skin
                        .joints
                        .iter()
                        .map(|joint| transforms.get(*joint).ok().map(GlobalTransform::to_matrix))
                        .collect::<Option<Vec<_>>>()?;
                    let palette = palette(anchor, &joints, inverse_bind)?;
                    if palette.len() > atlas.limit.saturating_sub(matrices.len()) {
                        return None;
                    }
                    let offset = matrices.len();
                    matrices.extend(palette);
                    Some(offset)
                })
            })
        } else {
            None
        };
        if let Some(value) = offset.and_then(|offset| palette_tag(tag.value, offset)) {
            if !draw.active {
                commands
                    .entity(entity)
                    .remove::<(SkinnedMesh, MeshMaterial3d<crate::fx::flash::ModelMaterial>)>()
                    .insert((
                        Mesh3d(draw.shared_mesh.clone()),
                        MeshMaterial3d(draw.shared_material.clone()),
                    ));
                replacements.insert(
                    entity,
                    (draw.shared_mesh.id(), draw.shared_material.id().untyped()),
                );
                draw.active = true;
                stats.ordinary_draws = stats.ordinary_draws.saturating_sub(1);
            }
            if tag.value != value {
                tag.value = value;
            }
            stats.shared_draws += 1;
        } else {
            if draw.active {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<CrowdMaterial>>()
                    .insert((
                        Mesh3d(draw.original_mesh.clone()),
                        MeshMaterial3d(draw.original_material.clone()),
                        draw.original_skin.clone(),
                    ));
                replacements.insert(
                    entity,
                    (
                        draw.original_mesh.id(),
                        draw.original_material.id().untyped(),
                    ),
                );
                draw.active = false;
                stats.ordinary_draws += 1;
            }
            if tag.value & !1 != 0 {
                tag.value &= 1;
            }
        }
    }
    stats.palettes = offsets.values().filter(|offset| offset.is_some()).count();
    stats.matrices = matrices.len();
    let changed = !matrices.is_empty()
        && buffers
            .get(&atlas.buffer)
            .and_then(ShaderBuffer::cast_slice::<Mat4>)
            != Some(matrices.as_slice());
    if changed && let Some(mut buffer) = buffers.get_mut(&atlas.buffer) {
        let capacity = buffer
            .buffer_size()
            .max((matrices.len() * size_of::<Mat4>()).next_power_of_two() as u64)
            .min((atlas.limit * size_of::<Mat4>()).max(size_of::<Mat4>()) as u64);
        buffer.clear();
        buffer.extend_from_slice(&matrices);
        buffer.resize_buffer(capacity);
    }
    bridge.replace_children(&replacements);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::RenderAssetUsages, mesh::Indices};

    fn source_mesh() -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0., 0., 0.]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 1.]; 3])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[0, 1, 0, 1]; 3]),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0.2, 0.8, 0., 0.]; 3])
        .with_inserted_indices(Indices::U16(vec![0, 1, 2]))
    }

    #[test]
    fn shared_vertex_layout_preserves_source_data_and_triangle_order() {
        let original = source_mesh();
        let shared = prepare_mesh(&original, 2).unwrap();
        for (attribute, values) in original.attributes() {
            let output = if attribute.id == Mesh::ATTRIBUTE_JOINT_INDEX.id {
                JOINTS.id
            } else if attribute.id == Mesh::ATTRIBUTE_JOINT_WEIGHT.id {
                WEIGHTS.id
            } else {
                attribute.id
            };
            assert_eq!(shared.attribute(output), Some(values));
        }
        assert!(original.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some());
        assert!(shared.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none());
        assert!(shared.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_none());
        assert_eq!(shared.indices(), original.indices());
    }

    #[test]
    fn missing_joints_invalid_weights_and_morphs_use_the_ordinary_path() {
        assert!(prepare_mesh(&source_mesh(), 1).is_none());
        let mut mesh = source_mesh();
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[f32::NAN; 4]; 3]);
        assert!(prepare_mesh(&mesh, 2).is_none());
        let mut mesh = source_mesh();
        mesh.set_morph_targets(vec![[Vec3::ZERO; 3].into()]);
        assert!(prepare_mesh(&mesh, 2).is_none());
    }

    #[test]
    fn instance_palette_matches_joint_world_positions_normals_and_tangents() {
        for scale in [Vec3::ONE, Vec3::new(0.7, 1.3, 0.9)] {
            for angle in [-1., 0., 1.4] {
                let root = Mat4::from_scale_rotation_translation(
                    scale,
                    Quat::from_rotation_y(angle),
                    Vec3::new(54., -0.2, 23.),
                );
                let anchor_local = Mat4::from_translation(Vec3::new(0.1, 0.3, 0.2));
                let joint_local = Mat4::from_scale_rotation_translation(
                    Vec3::new(0.8, 1.1, 0.9),
                    Quat::from_rotation_z(0.6),
                    Vec3::new(0.3, 1.2, 0.4),
                );
                let bind = Mat4::from_translation(Vec3::new(-0.1, -0.8, 0.2));
                let anchor = root * anchor_local;
                let joint = root * joint_local;
                let master_root = Mat4::from_scale_rotation_translation(
                    Vec3::splat(1.2),
                    Quat::from_rotation_y(0.3),
                    Vec3::new(7., 0., -9.),
                );
                let local = palette(
                    master_root * anchor_local,
                    &[master_root * joint_local],
                    &[bind],
                )
                .unwrap()[0];
                let original = joint * bind;
                let shared = anchor * local;
                for point in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(0.3, 0.7, -0.2)] {
                    assert!(
                        shared
                            .transform_point3(point)
                            .abs_diff_eq(original.transform_point3(point), 2e-5)
                    );
                }
                let normal =
                    |m: Mat4| (Mat3::from_mat4(m).inverse().transpose() * Vec3::Y).normalize();
                assert!(normal(shared).abs_diff_eq(normal(original), 1e-5));
                let tangent = |m: Mat4| (Mat3::from_mat4(m) * Vec3::X).normalize();
                assert!(tangent(shared).abs_diff_eq(tangent(original), 1e-5));
            }
        }
    }

    #[test]
    fn invalid_palettes_fall_back_and_palette_tags_preserve_the_flash_bit() {
        assert!(palette(Mat4::ZERO, &[Mat4::IDENTITY], &[Mat4::IDENTITY]).is_none());
        assert!(palette(Mat4::IDENTITY, &[], &[]).is_none());
        assert!(palette(Mat4::IDENTITY, &[Mat4::NAN], &[Mat4::IDENTITY]).is_none());
        assert!(palette(Mat4::IDENTITY, &[Mat4::IDENTITY], &[]).is_none());
        for flash in [0, 1] {
            let tag = palette_tag((123 << 1) | flash, 44).unwrap();
            assert_eq!(tag & 1, flash);
            assert_eq!(tag >> 1, 44);
        }
        assert!(palette_tag(1, usize::MAX).is_none());
    }

    #[test]
    fn weighted_palette_matches_the_existing_skin_and_exports_gpu_cases() {
        let mut atlas = vec![Mat4::IDENTITY; 2];
        let mut cases = Vec::new();
        let anchor_local = Mat4::from_translation(Vec3::new(0.1, 0.2, -0.1));
        let master = Mat4::from_scale_rotation_translation(
            Vec3::splat(0.9),
            Quat::from_rotation_y(0.4),
            Vec3::new(7., 0.2, -9.),
        );
        for scale in [
            Vec3::ONE,
            Vec3::new(0.7, 1.3, 0.9),
            Vec3::new(-0.8, 1.2, 1.1),
        ] {
            for angle in [-0.8, 0., 1.4] {
                let root = Mat4::from_scale_rotation_translation(
                    scale,
                    Quat::from_rotation_y(angle),
                    Vec3::new(54., -0.2, 23.),
                );
                let anchor = root * anchor_local;
                let local_joints = [
                    Mat4::from_rotation_translation(
                        Quat::from_rotation_z(angle),
                        Vec3::new(0.3, 1.2, 0.4),
                    ),
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.8, 1.1, 0.9),
                        Quat::from_rotation_x(0.6),
                        Vec3::new(0.2, 0.7, -0.1),
                    ),
                    Mat4::from_translation(Vec3::new(-0.2, 0.5, 0.3)),
                ];
                let bind = [
                    Mat4::from_translation(Vec3::new(0., -0.8, 0.)),
                    Mat4::from_rotation_y(0.2),
                    Mat4::IDENTITY,
                ];
                let source_joints: Vec<_> =
                    local_joints.iter().map(|joint| master * *joint).collect();
                let pose = palette(master * anchor_local, &source_joints, &bind).unwrap();
                let offset = atlas.len();
                atlas.extend_from_slice(&pose);
                for weights in [[1., 0., 0., 0.], [0.2, 0.3, 0.5, 0.]] {
                    let indices = [0_usize, 1, 2, 0];
                    let old: Mat4 = (0..4)
                        .map(|i| (root * local_joints[indices[i]] * bind[indices[i]]) * weights[i])
                        .fold(Mat4::ZERO, |a, b| a + b);
                    let shared: Mat4 = anchor
                        * (0..4)
                            .map(|i| pose[indices[i]] * weights[i])
                            .fold(Mat4::ZERO, |a, b| a + b);
                    let position = Vec4::new(0.3, 0.7, -0.2, 1.);
                    let normal =
                        |m: Mat4| (Mat3::from_mat4(m).inverse().transpose() * Vec3::Y).normalize();
                    let tangent = |m: Mat4| (Mat3::from_mat4(m) * Vec3::X).normalize();
                    assert!((shared * position).abs_diff_eq(old * position, 3e-5));
                    assert!(normal(shared).abs_diff_eq(normal(old), 2e-5));
                    assert!(tangent(shared).abs_diff_eq(tangent(old), 2e-5));
                    let tangent_w = if cases.len() % 2 == 0 { 1. } else { -1. };
                    let handedness = if Mat3::from_mat4(anchor).determinant() < 0. {
                        -1.
                    } else {
                        1.
                    };
                    cases.push(serde_json::json!({
                        "anchor":anchor.to_cols_array(), "indices":indices, "weights":weights,
                        "position":position.to_array(), "normal":[0.,1.,0.,0.], "tangent":[1.,0.,0.,tangent_w],
                        "tag":palette_tag((cases.len() % 2) as u32, offset).unwrap(),
                        "expected_position":(old * position).to_array(),
                        "expected_normal":normal(old).extend(0.).to_array(),
                        "expected_tangent":tangent(old).extend(tangent_w * handedness).to_array(),
                    }));
                }
            }
        }
        assert_eq!(cases.len(), 18);
        if let Ok(path) = std::env::var("TURTS_SHARED_SKIN_GPU_FIXTURE") {
            let data = serde_json::json!({"matrices":atlas.iter().map(Mat4::to_cols_array).collect::<Vec<_>>(), "cases":cases});
            std::fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
        }
    }

    fn runtime_fixture() -> (App, Vec<Entity>, Handle<ShaderBuffer>) {
        let mut app = App::new();
        app.init_resource::<SharedSkinSettings>()
            .init_resource::<SharedSkinStats>()
            .init_resource::<super::super::SharedPoses>()
            .init_resource::<crate::models::ModelGpuBridge>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<crate::fx::flash::ModelMaterial>>()
            .init_resource::<Assets<CrowdMaterial>>()
            .init_resource::<Assets<ShaderBuffer>>()
            .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
            .add_systems(Update, upload);
        let buffer = app
            .world_mut()
            .resource_mut::<Assets<ShaderBuffer>>()
            .add(ShaderBuffer::from(vec![Mat4::IDENTITY]));
        app.insert_resource(SharedSkinAtlas {
            buffer: buffer.clone(),
            limit: 4,
            ..default()
        });
        let original_mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(source_mesh());
        let shared_mesh = {
            let mesh = prepare_mesh(
                app.world()
                    .resource::<Assets<Mesh>>()
                    .get(&original_mesh)
                    .unwrap(),
                2,
            )
            .unwrap();
            app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh)
        };
        let original_material = app
            .world_mut()
            .resource_mut::<Assets<crate::fx::flash::ModelMaterial>>()
            .add(crate::fx::flash::ModelMaterial {
                base: default(),
                extension: default(),
            });
        let shared_material =
            app.world_mut()
                .resource_mut::<Assets<CrowdMaterial>>()
                .add(CrowdMaterial {
                    base: default(),
                    extension: CrowdExtension {
                        matrices: buffer.clone(),
                    },
                });
        let binds = app
            .world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY; 2]));
        let mut players = Vec::new();
        let mut draws = Vec::new();
        for i in 0..2 {
            let player = app.world_mut().spawn_empty().id();
            players.push(player);
            let x = 3. + i as f32 * 20.;
            let anchor = app
                .world_mut()
                .spawn(GlobalTransform::from(Transform::from_xyz(x, 0., 4.)))
                .id();
            let joints: Vec<_> = [0.3, 0.8]
                .into_iter()
                .map(|y| {
                    app.world_mut()
                        .spawn(GlobalTransform::from(
                            Transform::from_xyz(x, y, 4.).with_rotation(Quat::from_rotation_z(0.2)),
                        ))
                        .id()
                })
                .collect();
            draws.push(
                app.world_mut()
                    .spawn((
                        Mesh3d(shared_mesh.clone()),
                        MeshMaterial3d(shared_material.clone()),
                        MeshTag::new((123 << 1) | 1),
                        SharedSkin {
                            player,
                            anchor,
                            anchor_node: 0,
                            palette: Handle::<GltfSkin>::default().id(),
                            original_mesh: original_mesh.clone(),
                            shared_mesh: shared_mesh.clone(),
                            original_material: original_material.clone(),
                            shared_material: shared_material.clone(),
                            original_skin: SkinnedMesh {
                                inverse_bindposes: binds.clone(),
                                joints,
                            },
                            active: true,
                        },
                    ))
                    .id(),
            );
        }
        let mut poses = app.world_mut().resource_mut::<super::super::SharedPoses>();
        poses.masters.insert(players[0], players[0]);
        poses.masters.insert(players[1], players[0]);
        (app, draws, buffer)
    }

    #[test]
    fn two_instances_share_one_bounded_palette_and_budget_fallback_restores_typed_draws() {
        let (mut app, draws, buffer) = runtime_fixture();
        app.update();
        let stats = app.world().resource::<SharedSkinStats>();
        assert_eq!(
            (
                stats.palettes,
                stats.matrices,
                stats.shared_draws,
                stats.ordinary_draws
            ),
            (1, 2, 2, 0)
        );
        assert_eq!(
            app.world()
                .resource::<Assets<ShaderBuffer>>()
                .get(&buffer)
                .unwrap()
                .cast_slice::<Mat4>()
                .unwrap()
                .len(),
            2
        );
        app.world_mut().resource_mut::<SharedSkinAtlas>().limit = 1;
        app.update();
        for &entity in &draws {
            let world = app.world();
            assert!(world.get::<SkinnedMesh>(entity).is_some());
            assert!(world.get::<MeshMaterial3d<CrowdMaterial>>(entity).is_none());
            assert!(
                world
                    .get::<MeshMaterial3d<crate::fx::flash::ModelMaterial>>(entity)
                    .is_some()
            );
            assert_eq!(world.get::<MeshTag>(entity).unwrap().value, 1);
        }
        assert_eq!(app.world().resource::<SharedSkinStats>().matrices, 0);
        app.world_mut().resource_mut::<SharedSkinAtlas>().limit = 4;
        app.update();
        for entity in draws {
            assert!(app.world().get::<SkinnedMesh>(entity).is_none());
            assert!(
                app.world()
                    .get::<MeshMaterial3d<CrowdMaterial>>(entity)
                    .is_some()
            );
        }
    }

    #[test]
    fn motion_vector_rendering_uses_the_ordinary_skin_path() {
        let (mut app, draws, _) = runtime_fixture();
        let camera = app
            .world_mut()
            .spawn((
                Camera::default(),
                bevy::core_pipeline::prepass::MotionVectorPrepass,
            ))
            .id();
        app.update();
        assert!(!app.world().resource::<SharedSkinStats>().enabled);
        assert!(
            draws
                .iter()
                .all(|&entity| app.world().get::<SkinnedMesh>(entity).is_some())
        );
        app.world_mut().despawn(camera);
        app.update();
        assert_eq!(app.world().resource::<SharedSkinStats>().shared_draws, 2);
    }

    #[test]
    fn independent_next_frame_pose_upload_and_missing_bindpose_recovery() {
        let (mut app, draws, buffer) = runtime_fixture();
        app.update();
        let before = app
            .world()
            .resource::<Assets<ShaderBuffer>>()
            .get(&buffer)
            .unwrap()
            .cast_slice::<Mat4>()
            .unwrap()
            .to_vec();
        let skins: Vec<_> = draws
            .iter()
            .map(|entity| {
                app.world()
                    .get::<SharedSkin>(*entity)
                    .unwrap()
                    .original_skin
                    .clone()
            })
            .collect();
        for skin in &skins {
            let mut joint = app
                .world_mut()
                .get_mut::<GlobalTransform>(skin.joints[0])
                .unwrap();
            let mut transform = joint.compute_transform();
            transform.translation.y += 0.5;
            *joint = GlobalTransform::from(transform);
        }
        app.update();
        let after = app
            .world()
            .resource::<Assets<ShaderBuffer>>()
            .get(&buffer)
            .unwrap()
            .cast_slice::<Mat4>()
            .unwrap()
            .to_vec();
        assert_ne!(
            before, after,
            "a changed joint pose must reach the palette in the next update"
        );
        assert!((after[0].w_axis.y - 0.8).abs() < 1e-5);
        assert_eq!(app.world().resource::<SharedSkinStats>().palettes, 1);
        let handle = skins[0].inverse_bindposes.clone();
        let saved = app
            .world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .remove(handle.id())
            .unwrap();
        app.update();
        for &draw in &draws {
            assert!(
                app.world().get::<SkinnedMesh>(draw).is_some(),
                "missing bindposes must restore the ordinary draw"
            );
            assert!(
                app.world()
                    .get::<MeshMaterial3d<CrowdMaterial>>(draw)
                    .is_none()
            );
            assert_eq!(app.world().get::<MeshTag>(draw).unwrap().value, 1);
        }
        app.world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .insert(handle.id(), saved)
            .unwrap();
        app.update();
        assert_eq!(app.world().resource::<SharedSkinStats>().shared_draws, 2);
        for draw in draws {
            assert!(app.world().get::<SkinnedMesh>(draw).is_none());
            assert!(
                app.world()
                    .get::<MeshMaterial3d<CrowdMaterial>>(draw)
                    .is_some()
            );
            assert_eq!(app.world().get::<MeshTag>(draw).unwrap().value & 1, 1);
        }
    }
}
