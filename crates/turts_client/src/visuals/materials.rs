//! Shared materials and meshes of the world visuals: the ring overlay material, the order
//! marker gizmo group, and the ring and bar meshes.

use super::*;

pub(super) type RingOverlayMaterial = ExtendedMaterial<StandardMaterial, RingOverlayExtension>;

#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub(super) struct OrderMarkerGizmoConfigGroup;

pub(super) fn order_marker_gizmo_config() -> GizmoConfig {
    GizmoConfig {
        depth_bias: -1.0,
        ..default()
    }
}

/// Keeps footprint rings visible over larger unit silhouettes without changing their ground size.
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub(super) struct RingOverlayExtension {}

impl MaterialExtension for RingOverlayExtension {
    fn alpha_mode() -> Option<AlphaMode> {
        Some(AlphaMode::Blend)
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let depth = descriptor
            .depth_stencil
            .as_mut()
            .expect("3D ring overlay pipeline has depth state");
        configure_ring_overlay_depth(depth);
        Ok(())
    }
}

pub(super) fn configure_ring_overlay_depth(depth: &mut DepthStencilState) {
    depth.depth_compare = Some(CompareFunction::Always);
    depth.depth_write_enabled = Some(false);
}

pub(super) fn create_ring_overlay_materials(
    palette: &[Handle<StandardMaterial>],
    materials: &Assets<StandardMaterial>,
    overlays: &mut Assets<RingOverlayMaterial>,
) -> Option<Vec<Handle<RingOverlayMaterial>>> {
    let bases = palette
        .iter()
        .map(|handle| materials.get(handle).cloned())
        .collect::<Option<Vec<_>>>()?;
    Some(
        bases
            .into_iter()
            .map(|slot_material| {
                overlays.add(ExtendedMaterial {
                    base: StandardMaterial {
                        alpha_mode: AlphaMode::Blend,
                        ..slot_material
                    },
                    extension: RingOverlayExtension {},
                })
            })
            .collect(),
    )
}

pub(super) fn create_neutral_ring_overlay_materials(
    overlays: &mut Assets<RingOverlayMaterial>,
) -> Vec<Handle<RingOverlayMaterial>> {
    [Color::srgb(0.96, 0.96, 0.96), Color::BLACK]
        .into_iter()
        .map(|base_color| {
            overlays.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color,
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                },
                extension: RingOverlayExtension {},
            })
        })
        .collect()
}

pub(super) fn create_marker_overlay_material(
    colour: Color,
    overlays: &mut Assets<RingOverlayMaterial>,
) -> Handle<RingOverlayMaterial> {
    overlays.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: colour,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        },
        extension: RingOverlayExtension {},
    })
}

pub(super) fn neutral_ring_overlay_index(slot_colour: Color) -> usize {
    let colour = slot_colour.to_srgba();
    let luminance = 0.2126 * colour.red + 0.7152 * colour.green + 0.0722 * colour.blue;
    usize::from(luminance >= 0.82)
}

#[derive(Clone)]
pub(super) enum VisualMaterial {
    Standard(Handle<StandardMaterial>),
    RingOverlay(Handle<RingOverlayMaterial>),
}

impl From<Handle<StandardMaterial>> for VisualMaterial {
    fn from(material: Handle<StandardMaterial>) -> Self {
        Self::Standard(material)
    }
}

pub(super) fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    }
}

pub(super) fn parse_hex(value: &str) -> Color {
    let raw = value.strip_prefix('#').unwrap_or(value);
    if raw.len() != 6 {
        return Color::WHITE;
    }
    let Ok(red) = u8::from_str_radix(&raw[0..2], 16) else {
        return Color::WHITE;
    };
    let Ok(green) = u8::from_str_radix(&raw[2..4], 16) else {
        return Color::WHITE;
    };
    let Ok(blue) = u8::from_str_radix(&raw[4..6], 16) else {
        return Color::WHITE;
    };
    Color::srgb_u8(red, green, blue)
}

pub(super) fn world_units_for_pixels(
    camera: &Camera,
    transform: &GlobalTransform,
    point: Vec3,
    size: Vec2,
) -> Vec2 {
    let Some(center) = camera.world_to_viewport(transform, point).ok() else {
        return Vec2::new(0.9, 0.09);
    };
    let Some(x_step) = camera
        .world_to_viewport(transform, point + Vec3::X * 0.5)
        .ok()
    else {
        return Vec2::new(0.9, 0.09);
    };
    let Some(y_step) = camera
        .world_to_viewport(transform, point + Vec3::Y * 0.5)
        .ok()
    else {
        return Vec2::new(0.9, 0.09);
    };
    let px_per_x = center.distance(x_step) / 0.5;
    let px_per_y = center.distance(y_step) / 0.5;
    if px_per_x <= 0.0 || px_per_y <= 0.0 || !px_per_x.is_finite() || !px_per_y.is_finite() {
        return Vec2::new(0.9, 0.09);
    }
    Vec2::new(size.x / px_per_x, size.y / px_per_y)
}

fn annulus_mesh(gaps: bool) -> Mesh {
    let segments = 192_u32;
    let mut positions = Vec::with_capacity((segments as usize + 1) * 2);
    let mut normals = Vec::with_capacity((segments as usize + 1) * 2);
    let mut uvs = Vec::with_capacity((segments as usize + 1) * 2);
    let mut indices = Vec::new();
    for index in 0..=segments {
        let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
        let inner = 0.88;
        let outer = 1.0;
        for radius in [inner, outer] {
            positions.push([radius * angle.cos(), 0.0, radius * angle.sin()]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([index as f32 / segments as f32, radius]);
        }
    }
    for index in 0..segments {
        let angle = (index as f32 + 0.5) / segments as f32 * std::f32::consts::TAU;
        let in_gap = gaps
            && (0..4).any(|gap| {
                let center = gap as f32 * std::f32::consts::FRAC_PI_2;
                let delta = (angle - center + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                delta.abs() < 0.09
            });
        if in_gap {
            continue;
        }
        let first_vertex = index * 2;
        indices.extend_from_slice(&[
            first_vertex,
            first_vertex + 2,
            first_vertex + 1,
            first_vertex + 1,
            first_vertex + 2,
            first_vertex + 3,
        ]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub(super) fn create_shared_meshes(assets: &mut VisualAssets, meshes: &mut Assets<Mesh>) {
    assets.ring = Some(meshes.add(annulus_mesh(false)));
    assets.foreign_ring = Some(meshes.add(annulus_mesh(true)));
    assets.bar = Some(meshes.add(Cuboid::new(1.0, 1.0, 0.12)));
}
