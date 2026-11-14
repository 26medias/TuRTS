//! Bake a rigid bone attachment into its existing skin palette without changing its motion.
use bevy::{
    gltf::{
        Gltf, GltfNode, GltfSkin,
        gltf::animation::{Interpolation, Property, util::ReadOutputs},
    },
    mesh::{PrimitiveTopology, VertexAttributeValues, skinning::SkinnedMeshInverseBindposes},
    prelude::*,
};
use std::collections::{HashMap, HashSet};

struct Palette {
    anchor: usize,
    parent: Option<usize>,
    skin: Handle<GltfSkin>,
    joints: Vec<usize>,
    inverse_bindposes: Vec<Mat4>,
}

pub(super) struct Context<'a> {
    gltf: &'a Gltf,
    nodes: &'a Assets<GltfNode>,
    parents: &'a HashMap<usize, usize>,
    animated: &'a HashSet<usize>,
    positive_scale: HashSet<usize>,
    palettes: Vec<Palette>,
}

pub(super) struct Converted {
    pub node: usize,
    pub parent: Option<usize>,
    pub skin: Handle<GltfSkin>,
    pub mesh: Mesh,
}

impl<'a> Context<'a> {
    pub fn new(
        gltf: &'a Gltf,
        nodes: &'a Assets<GltfNode>,
        skins: &Assets<GltfSkin>,
        bindposes: &Assets<SkinnedMeshInverseBindposes>,
        parents: &'a HashMap<usize, usize>,
        animated: &'a HashSet<usize>,
    ) -> Self {
        let mut positive_scale = HashSet::new();
        let mut palettes = Vec::new();
        if let Some(source) = gltf.source.as_ref() {
            for node in source.nodes() {
                if let Some(data) = nodes.get(&gltf.nodes[node.index()]) {
                    if data.transform.scale.is_finite()
                        && data.transform.scale.cmpgt(Vec3::ZERO).all()
                    {
                        positive_scale.insert(node.index());
                    }
                    if let Some(handle) = data.skin.as_ref()
                        && data.mesh.is_some()
                        && data.transform == Transform::IDENTITY
                        && !animated.contains(&node.index())
                        && let Some(skin) = skins.get(handle)
                        && let Some(inverse) = bindposes.get(&skin.inverse_bind_matrices)
                        && let Some(joints) = skin
                            .joints
                            .iter()
                            .map(|joint| nodes.get(joint).map(|n| n.index))
                            .collect::<Option<Vec<_>>>()
                        && !palettes.iter().any(|p: &Palette| {
                            p.skin == *handle && p.parent == parents.get(&node.index()).copied()
                        })
                    {
                        palettes.push(Palette {
                            anchor: node.index(),
                            parent: parents.get(&node.index()).copied(),
                            skin: handle.clone(),
                            joints,
                            inverse_bindposes: inverse.to_vec(),
                        });
                    }
                }
            }
            // A reflected ancestor changes tangent handedness. Keep those draws separate.
            // Positive linear/step scale keys stay positive; cubic curves can overshoot.
            for channel in source
                .animations()
                .flat_map(|animation| animation.channels())
            {
                if channel.target().property() != Property::Scale {
                    continue;
                }
                let positive = matches!(
                    channel.sampler().interpolation(),
                    Interpolation::Linear | Interpolation::Step
                ) && match channel
                    .reader(|buffer| {
                        (buffer.index() == 0)
                            .then_some(source.blob.as_deref())
                            .flatten()
                    })
                    .read_outputs()
                {
                    Some(ReadOutputs::Scales(values)) => values
                        .into_iter()
                        .all(|value| value.into_iter().all(|v| v.is_finite() && v > 0.)),
                    _ => false,
                };
                if !positive {
                    positive_scale.remove(&channel.target().node().index());
                }
            }
        }
        Self {
            gltf,
            nodes,
            parents,
            animated,
            positive_scale,
            palettes,
        }
    }

    pub fn convert(&self, node: usize, mesh: &Mesh) -> Option<Converted> {
        for palette in &self.palettes {
            if let Some(converted) = self.in_palette(node, mesh, palette) {
                return Some(converted);
            }
        }
        None
    }

    fn in_palette(&self, node: usize, mesh: &Mesh, palette: &Palette) -> Option<Converted> {
        let mut cursor = node;
        let mut relative = Mat4::IDENTITY;
        let mut remaining = self.gltf.nodes.len();
        let joint = loop {
            if remaining == 0 {
                return None;
            }
            remaining -= 1;
            if let Some(joint) = palette.joints.iter().position(|&joint| joint == cursor) {
                break joint;
            }
            if self.animated.contains(&cursor) || !self.positive_scale.contains(&cursor) {
                return None;
            }
            relative = self
                .nodes
                .get(&self.gltf.nodes[cursor])?
                .transform
                .to_matrix()
                * relative;
            cursor = *self.parents.get(&cursor)?;
        };
        let mut ancestor = Some(cursor);
        let mut shared_parent = palette.parent.is_none();
        let mut remaining = self.gltf.nodes.len();
        while let Some(node) = ancestor {
            if remaining == 0 {
                return None;
            }
            remaining -= 1;
            if !self.positive_scale.contains(&node) {
                return None;
            }
            shared_parent |= palette.parent == Some(node);
            ancestor = self.parents.get(&node).copied();
        }
        if !shared_parent {
            return None;
        }
        let inverse_bind = *palette.inverse_bindposes.get(joint)?;
        let determinant = Mat3::from_mat4(inverse_bind).determinant();
        if !inverse_bind.is_finite()
            || inverse_bind.row(3) != Vec4::W
            || !determinant.is_finite()
            || determinant == 0.
        {
            return None;
        }
        let bind = Mat4::from(bevy::math::Affine3A::from_mat4(inverse_bind).inverse());
        // Bevy's skin shader uses joint_world * inverse_bind. These cancel the baked inverse.
        let baked = bake(mesh, bind * relative, u16::try_from(joint).ok()?)?;
        Some(Converted {
            node: palette.anchor,
            parent: palette.parent,
            skin: palette.skin.clone(),
            mesh: baked,
        })
    }
}

fn bake(mesh: &Mesh, matrix: Mat4, joint: u16) -> Option<Mesh> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList
        || mesh.try_has_morph_targets().ok()?
        || mesh
            .try_attribute_option(Mesh::ATTRIBUTE_JOINT_INDEX)
            .ok()?
            .is_some()
        || mesh
            .try_attribute_option(Mesh::ATTRIBUTE_JOINT_WEIGHT)
            .ok()?
            .is_some()
        || !matrix.is_finite()
        || matrix.row(3) != Vec4::W
    {
        return None;
    }
    let linear = Mat3::from_mat4(matrix);
    let determinant = linear.determinant();
    if !determinant.is_finite() || determinant <= 0. {
        return None;
    }
    let normal = linear.inverse().transpose();
    if !normal.is_finite() {
        return None;
    }
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.try_attribute_option(Mesh::ATTRIBUTE_POSITION).ok()?
    else {
        return None;
    };
    let positions: Option<Vec<_>> = positions
        .iter()
        .map(|p| {
            let p = matrix.transform_point3(Vec3::from_array(*p));
            p.is_finite().then(|| p.to_array())
        })
        .collect();
    let positions = positions?;
    let count = positions.len();
    let mut result = mesh.clone();
    result.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    if let Some(values) = mesh.try_attribute_option(Mesh::ATTRIBUTE_NORMAL).ok()? {
        let VertexAttributeValues::Float32x3(values) = values else {
            return None;
        };
        let normals: Option<Vec<_>> = values
            .iter()
            .map(|v| normalized_finite(normal * Vec3::from_array(*v)).map(|v| v.to_array()))
            .collect();
        result.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals?);
    }
    if let Some(values) = mesh.try_attribute_option(Mesh::ATTRIBUTE_TANGENT).ok()? {
        let VertexAttributeValues::Float32x4(values) = values else {
            return None;
        };
        let tangents: Option<Vec<_>> = values
            .iter()
            .map(|v| {
                let t = normalized_finite(linear * Vec3::new(v[0], v[1], v[2]))?.extend(v[3]);
                t.is_finite().then(|| t.to_array())
            })
            .collect();
        result.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents?);
    }
    result.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        VertexAttributeValues::Uint16x4(vec![[joint, 0, 0, 0]; count]),
    );
    result.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1., 0., 0., 0.]; count]);
    Some(result)
}

fn normalized_finite(vector: Vec3) -> Option<Vec3> {
    if !vector.is_finite() {
        return None;
    }
    let scale = vector.abs().max_element();
    Some(if scale > 0. {
        (vector / scale).normalize()
    } else {
        Vec3::ZERO
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::RenderAssetUsages, mesh::Indices};

    fn triangle() -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[1., 0., 0., 1.]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.], [1., 0.], [0., 1.]])
        .with_inserted_indices(Indices::U16(vec![0, 1, 2]))
    }

    #[test]
    fn baked_attachment_matches_rigid_positions_normals_and_tangents_across_joint_poses() {
        let original = triangle();
        let inverse_bind = Mat4::from(
            bevy::math::Affine3A::from_scale_rotation_translation(
                Vec3::new(0.8, 1.2, 0.7),
                Quat::from_rotation_x(0.4),
                Vec3::new(0.2, 0.8, -0.1),
            )
            .inverse(),
        );
        let relative = Mat4::from_scale_rotation_translation(
            Vec3::new(0.6, 0.9, 1.1),
            Quat::from_rotation_y(0.7),
            Vec3::new(-0.2, 0.3, 0.5),
        );
        let converted = bake(
            &original,
            Mat4::from(bevy::math::Affine3A::from_mat4(inverse_bind).inverse()) * relative,
            7,
        )
        .unwrap();
        let VertexAttributeValues::Float32x3(positions) =
            converted.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        let VertexAttributeValues::Float32x3(normals) =
            converted.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!()
        };
        let VertexAttributeValues::Float32x4(tangents) =
            converted.attribute(Mesh::ATTRIBUTE_TANGENT).unwrap()
        else {
            panic!()
        };
        for angle in [-1.2, 0., 0.8] {
            let joint = Mat4::from_scale_rotation_translation(
                Vec3::new(1.3, 0.7, 1.1),
                Quat::from_rotation_z(angle),
                Vec3::new(3., 2., -4.),
            );
            let rigid = joint * relative;
            let skin = joint * inverse_bind;
            for (i, p) in [Vec3::ZERO, Vec3::X, Vec3::Y].into_iter().enumerate() {
                assert!(
                    skin.transform_point3(Vec3::from_array(positions[i]))
                        .abs_diff_eq(rigid.transform_point3(p), 1e-5)
                );
                let actual = (Mat3::from_mat4(skin).inverse().transpose()
                    * Vec3::from_array(normals[i]))
                .normalize();
                let expected = (Mat3::from_mat4(rigid).inverse().transpose() * Vec3::Z).normalize();
                assert!(actual.abs_diff_eq(expected, 1e-5));
                let actual = (Mat3::from_mat4(skin)
                    * Vec3::new(tangents[i][0], tangents[i][1], tangents[i][2]))
                .normalize();
                assert!(actual.abs_diff_eq((Mat3::from_mat4(rigid) * Vec3::X).normalize(), 1e-5));
                assert_eq!(tangents[i][3], 1.);
            }
        }
        assert_eq!(
            converted.attribute(Mesh::ATTRIBUTE_UV_0),
            original.attribute(Mesh::ATTRIBUTE_UV_0)
        );
        assert_eq!(
            converted.indices().unwrap().iter().collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(
            converted.attribute(Mesh::ATTRIBUTE_JOINT_INDEX),
            Some(&VertexAttributeValues::Uint16x4(vec![[7, 0, 0, 0]; 3]))
        );
        assert_eq!(
            converted.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
            Some(&VertexAttributeValues::Float32x4(vec![[1., 0., 0., 0.]; 3]))
        );
    }

    #[test]
    fn unsupported_or_singular_payloads_keep_the_original_rigid_draw() {
        let mesh = triangle();
        for transform in [
            Mat4::from_scale(Vec3::new(-1., 1., 1.)),
            Mat4::from_scale(Vec3::new(0., 1., 1.)),
            Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::Z, Vec4::new(0., 0., 0., 0.5)),
        ] {
            assert!(bake(&mesh, transform, 0).is_none());
        }
        let mut invalid_normal = triangle();
        invalid_normal.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[f32::NAN, 0., 1.]; 3]);
        assert!(bake(&invalid_normal, Mat4::IDENTITY, 0).is_none());
        assert!(
            normalized_finite(Vec3::splat(f32::MAX))
                .unwrap()
                .is_normalized()
        );
        let mut morph = triangle();
        morph.set_morph_targets(vec![[Vec3::ZERO; 3].into()]);
        assert!(bake(&morph, Mat4::IDENTITY, 0).is_none());
        let mut weighted = triangle();
        weighted.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1., 0., 0., 0.]; 3]);
        assert!(bake(&weighted, Mat4::IDENTITY, 0).is_none());
    }
}
