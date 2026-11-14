//! Join compatible skin parts once per asset. Keep every animation node and skin palette.
use bevy::{
    gltf::{Gltf, GltfMaterial, GltfMesh, GltfNode, GltfSkin},
    mesh::skinning::SkinnedMeshInverseBindposes,
    prelude::*,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(super) struct RigPrimitive {
    pub node: usize,
    pub mesh: Handle<Mesh>,
    pub material: AssetId<GltfMaterial>,
    pub skin: Option<Handle<GltfSkin>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct BatchKey {
    parent: Option<usize>,
    skin: AssetId<GltfSkin>,
    material: AssetId<GltfMaterial>,
}

struct Pending {
    primitive: RigPrimitive,
    key: Option<BatchKey>,
    mesh: Option<Mesh>,
    modified: bool,
}

// Mesh::merge permits missing attributes. Reject them here to preserve all vertex data.
fn merge_compatible(first: &Mesh, second: &Mesh) -> Option<Mesh> {
    if first.primitive_topology() != bevy::mesh::PrimitiveTopology::TriangleList
        || first.primitive_topology() != second.primitive_topology()
        || first.try_has_morph_targets().ok()?
        || second.try_has_morph_targets().ok()?
        || first.try_indices_option().ok()?.is_some() != second.try_indices_option().ok()?.is_some()
    {
        return None;
    }
    let a = first.try_attributes().ok()?.collect::<Vec<_>>();
    let b = second.try_attributes().ok()?.collect::<Vec<_>>();
    if a.len() != b.len()
        || a.iter()
            .zip(b)
            .any(|((attr, values), (other, other_values))| {
                attr.id != other.id
                    || std::mem::discriminant(*values) != std::mem::discriminant(other_values)
            })
    {
        return None;
    }
    let mut merged = first.clone();
    merged.merge(second).ok()?;
    Some(merged)
}

pub(super) fn prepare(
    gltf: &Gltf,
    nodes: &Assets<GltfNode>,
    meshes: &Assets<GltfMesh>,
    skins: &Assets<GltfSkin>,
    bindposes: &Assets<SkinnedMeshInverseBindposes>,
    mesh_assets: &mut Assets<Mesh>,
) -> Option<Vec<RigPrimitive>> {
    let source = gltf.source.as_ref()?;
    let parents: HashMap<_, _> = source
        .nodes()
        .flat_map(|node| {
            node.children()
                .map(move |child| (child.index(), node.index()))
        })
        .collect();
    let animated: HashSet<_> = source
        .animations()
        .flat_map(|animation| {
            animation
                .channels()
                .map(|channel| channel.target().node().index())
        })
        .collect();
    let rigid =
        super::rigid_attachment::Context::new(gltf, nodes, skins, bindposes, &parents, &animated);
    let mut pending: Vec<Pending> = Vec::new();
    for node in source.nodes() {
        let data = nodes.get(&gltf.nodes[node.index()])?;
        let Some(handle) = data.mesh.as_ref() else {
            continue;
        };
        let mesh = meshes.get(handle)?;
        for primitive in &mesh.primitives {
            let material = primitive.material.clone().unwrap_or_default().id();
            // Identity siblings with the same skin have the same mesh-space transform.
            // Animated mesh nodes and morph targets keep their original draw entities.
            let opaque = node
                .mesh()
                .and_then(|mesh| mesh.primitives().nth(primitive.index))
                .is_some_and(|primitive| {
                    matches!(
                        primitive.material().alpha_mode(),
                        bevy::gltf::gltf::material::AlphaMode::Opaque
                            | bevy::gltf::gltf::material::AlphaMode::Mask
                    )
                });
            let mut key = data
                .skin
                .as_ref()
                .filter(|_| {
                    opaque
                        && data.transform == Transform::IDENTITY
                        && !animated.contains(&node.index())
                })
                .map(|skin| BatchKey {
                    parent: parents.get(&node.index()).copied(),
                    skin: skin.id(),
                    material,
                });
            let mut incoming = mesh_assets.get(&primitive.mesh).cloned();
            let mut output_node = node.index();
            let mut output_skin = data.skin.clone();
            let mut modified = false;
            // Bevy selects its skin shader from vertex attributes. glTF selects it per node.
            // Unbound attributes on rigid gear must not select a skin or use another rig's palette.
            if data.skin.is_none()
                && let Some(mesh) = incoming.as_mut()
            {
                modified |= mesh
                    .try_remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                    .ok()
                    .is_some();
                modified |= mesh
                    .try_remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
                    .ok()
                    .is_some();
            }
            if opaque
                && data.skin.is_none()
                && let Some(converted) = incoming
                    .as_ref()
                    .and_then(|mesh| rigid.convert(node.index(), mesh))
            {
                key = Some(BatchKey {
                    parent: converted.parent,
                    skin: converted.skin.id(),
                    material,
                });
                output_node = converted.node;
                output_skin = Some(converted.skin);
                incoming = Some(converted.mesh);
                modified = true;
            }
            let joined = key
                .and_then(|key| {
                    pending
                        .iter_mut()
                        .filter(|part| part.key == Some(key))
                        .find_map(|part| {
                            let merged = merge_compatible(part.mesh.as_ref()?, incoming.as_ref()?)?;
                            part.mesh = Some(merged);
                            part.modified = true;
                            Some(())
                        })
                })
                .is_some();
            if !joined {
                pending.push(Pending {
                    primitive: RigPrimitive {
                        node: output_node,
                        mesh: primitive.mesh.clone(),
                        material,
                        skin: output_skin,
                    },
                    key,
                    mesh: incoming,
                    modified,
                });
            }
        }
    }
    Some(
        pending
            .into_iter()
            .map(|mut part| {
                if part.modified
                    && let Some(mesh) = part.mesh
                {
                    part.primitive.mesh = mesh_assets.add(mesh);
                }
                part.primitive
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        asset::RenderAssetUsages,
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    };

    fn triangle(joint: u16) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.], [1., 0.], [0., 1.]])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[joint, 0, 0, 0]; 3]),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1., 0., 0., 0.]; 3])
        .with_inserted_indices(Indices::U16(vec![0, 1, 2]))
    }

    #[test]
    fn joined_skin_preserves_vertices_uvs_joint_indices_weights_and_triangle_order() {
        let a = triangle(3);
        let b = triangle(17);
        let merged = merge_compatible(&a, &b).unwrap();
        assert_eq!(merged.count_vertices(), 6);
        assert_eq!(
            merged.indices().unwrap().iter().collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5]
        );
        for (attribute, values) in a.attributes() {
            let combined = merged.attribute(attribute.id).unwrap();
            assert_eq!(
                combined.len(),
                values.len() + b.attribute(attribute.id).unwrap().len()
            );
        }
        assert_eq!(
            merged.attribute(Mesh::ATTRIBUTE_JOINT_INDEX),
            Some(&VertexAttributeValues::Uint16x4(
                vec![[3, 0, 0, 0]; 3]
                    .into_iter()
                    .chain(vec![[17, 0, 0, 0]; 3])
                    .collect()
            ))
        );
        assert_eq!(
            merged.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
            Some(&VertexAttributeValues::Float32x4(vec![[1., 0., 0., 0.]; 6]))
        );
        assert_eq!(a.count_vertices(), 3);
        assert_eq!(b.count_vertices(), 3);
    }

    #[test]
    fn incompatible_attributes_and_morph_targets_keep_separate_meshes() {
        let a = triangle(0);
        let mut b = triangle(0);
        b.remove_attribute(Mesh::ATTRIBUTE_UV_0);
        assert!(merge_compatible(&a, &b).is_none());
        let mut b = triangle(0);
        b.set_morph_targets(vec![[Vec3::ZERO; 3].into()]);
        assert!(merge_compatible(&a, &b).is_none());
        let strip = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0., 0., 0.]; 3]);
        assert!(merge_compatible(&strip, &strip).is_none());
    }

    // Use the shipped hierarchy and animation channels. Dummy vertex payloads isolate
    // the grouping policy; the vertex preservation test above checks the mesh operation.
    fn fixture(bytes: &[u8]) -> (Gltf, Assets<GltfNode>, Assets<GltfMesh>, Assets<Mesh>) {
        let source = bevy::gltf::gltf::Gltf::from_slice(bytes).unwrap();
        let mut nodes = Assets::<GltfNode>::default();
        let mut meshes = Assets::<GltfMesh>::default();
        let mut vertex_meshes = Assets::<Mesh>::default();
        let material_assets = Assets::<GltfMaterial>::default();
        let skin_assets = Assets::<GltfSkin>::default();
        let materials: Vec<_> = source
            .materials()
            .map(|_| material_assets.reserve_handle())
            .collect();
        let skins: Vec<_> = source
            .skins()
            .map(|_| skin_assets.reserve_handle())
            .collect();
        let node_handles: Vec<_> = source.nodes().map(|_| nodes.reserve_handle()).collect();
        let mesh_handles: Vec<_> = source
            .meshes()
            .map(|m| {
                let primitives = m
                    .primitives()
                    .map(|p| {
                        bevy::gltf::GltfPrimitive::new(
                            &m,
                            &p,
                            vertex_meshes.add(triangle(0)),
                            p.material().index().map(|i| materials[i].clone()),
                            None,
                            None,
                        )
                    })
                    .collect();
                meshes.add(GltfMesh::new(&m, primitives, None))
            })
            .collect();
        for node in source.nodes() {
            let (translation, rotation, scale) = node.transform().decomposed();
            nodes
                .insert(
                    node_handles[node.index()].id(),
                    GltfNode::new(
                        &node,
                        node.children()
                            .map(|n| node_handles[n.index()].clone())
                            .collect(),
                        node.mesh().map(|m| mesh_handles[m.index()].clone()),
                        Transform {
                            translation: Vec3::from_array(translation),
                            rotation: Quat::from_array(rotation),
                            scale: Vec3::from_array(scale),
                        },
                        node.skin().map(|skin| skins[skin.index()].clone()),
                        None,
                    ),
                )
                .unwrap();
        }
        let gltf = Gltf {
            scenes: vec![],
            named_scenes: Default::default(),
            meshes: mesh_handles,
            named_meshes: Default::default(),
            materials,
            named_materials: Default::default(),
            nodes: node_handles,
            named_nodes: Default::default(),
            skins,
            named_skins: Default::default(),
            default_scene: None,
            animations: vec![],
            named_animations: Default::default(),
            source: Some(source),
        };
        (gltf, nodes, meshes, vertex_meshes)
    }

    type RigidFixture = (
        Gltf,
        Assets<GltfNode>,
        Assets<GltfMesh>,
        Assets<Mesh>,
        Assets<GltfSkin>,
        Assets<SkinnedMeshInverseBindposes>,
    );
    fn rigid_fixture(bytes: &[u8]) -> RigidFixture {
        let (mut gltf, mut nodes, meshes, mut vertex_meshes) = fixture(bytes);
        let source = gltf.source.as_ref().unwrap();
        let mut skins = Assets::default();
        let mut bindposes = Assets::default();
        let handles: Vec<_> = source
            .skins()
            .map(|skin| {
                let matrices: Vec<_> = skin
                    .reader(|_| source.blob.as_deref())
                    .read_inverse_bind_matrices()
                    .map(|values| values.map(|m| Mat4::from_cols_array_2d(&m)).collect())
                    .unwrap_or_else(|| vec![Mat4::IDENTITY; skin.joints().len()]);
                skins.add(GltfSkin::new(
                    &skin,
                    skin.joints()
                        .map(|n| gltf.nodes[n.index()].clone())
                        .collect(),
                    bindposes.add(SkinnedMeshInverseBindposes::from(matrices)),
                    None,
                ))
            })
            .collect();
        for node in source.nodes() {
            if let Some(skin) = node.skin() {
                nodes.get_mut(&gltf.nodes[node.index()]).unwrap().skin =
                    Some(handles[skin.index()].clone());
            }
        }
        gltf.skins = handles;
        for (_, node) in nodes.iter().filter(|(_, n)| n.skin.is_none()) {
            if let Some(mesh) = node.mesh.as_ref() {
                for primitive in &meshes.get(mesh).unwrap().primitives {
                    let mut mesh = vertex_meshes.get_mut(&primitive.mesh).unwrap();
                    mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
                    mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                }
            }
        }
        (gltf, nodes, meshes, vertex_meshes, skins, bindposes)
    }

    #[test]
    fn shipped_rigid_attachments_join_skin_batches_without_losing_vertices() {
        for (bytes, expected) in [
            (
                include_bytes!("../../../../mods/skirmish/assets/models/soldier_anim.glb")
                    .as_slice(),
                3,
            ),
            (
                include_bytes!("../../../../mods/skirmish/assets/models/swordsman_anim.glb")
                    .as_slice(),
                2,
            ),
        ] {
            let (gltf, nodes, meshes, mut vertex_meshes, skins, bindposes) = rigid_fixture(bytes);
            let original: usize = meshes.iter().map(|(_, m)| m.primitives.len()).sum();
            let result = prepare(
                &gltf,
                &nodes,
                &meshes,
                &skins,
                &bindposes,
                &mut vertex_meshes,
            )
            .unwrap();
            assert_eq!(
                result.len(),
                expected,
                "attachment batches: {:?}",
                result
                    .iter()
                    .map(|p| (p.node, p.skin.is_some()))
                    .collect::<Vec<_>>()
            );
            assert!(result.iter().all(|p| p.skin.is_some()));
            assert_eq!(
                result
                    .iter()
                    .map(|p| vertex_meshes.get(&p.mesh).unwrap().count_vertices())
                    .sum::<usize>(),
                original * 3
            );
        }
    }

    #[test]
    fn joined_vertices_follow_the_same_shipped_hierarchy_before_and_after_joint_motion() {
        let bytes = include_bytes!("../../../../mods/skirmish/assets/models/soldier_anim.glb");
        let (gltf, nodes, meshes, mut vertex_meshes, skins, bindposes) = rigid_fixture(bytes);
        let result = prepare(
            &gltf,
            &nodes,
            &meshes,
            &skins,
            &bindposes,
            &mut vertex_meshes,
        )
        .unwrap();
        let parents: HashMap<_, _> = gltf
            .source
            .as_ref()
            .unwrap()
            .nodes()
            .flat_map(|n| n.children().map(move |c| (c.index(), n.index())))
            .collect();
        let rest: Vec<_> = gltf
            .nodes
            .iter()
            .map(|h| nodes.get(h).unwrap().transform.to_matrix())
            .collect();
        for angle in [0., -0.7, 1.2] {
            let mut locals = rest.clone();
            let joint = skins.iter().next().unwrap().1.joints[10].clone();
            let joint = nodes.get(&joint).unwrap().index;
            locals[joint] *= Mat4::from_rotation_z(angle);
            let unit = Mat4::from_scale_rotation_translation(
                Vec3::splat(0.88),
                Quat::from_rotation_y(0.5),
                Vec3::new(20., 0., -2.),
            );
            let worlds: Vec<_> = (0..locals.len())
                .map(|node| {
                    let mut matrix = locals[node];
                    let mut cursor = node;
                    while let Some(&parent) = parents.get(&cursor) {
                        matrix = locals[parent] * matrix;
                        cursor = parent;
                    }
                    unit * matrix
                })
                .collect();
            let positions = |node: usize, mesh: &Mesh, skin: Option<&Handle<GltfSkin>>| {
                let VertexAttributeValues::Float32x3(positions) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                else {
                    panic!()
                };
                if let Some(skin) = skin {
                    let skin = skins.get(skin).unwrap();
                    let inverse = bindposes.get(&skin.inverse_bind_matrices).unwrap();
                    let VertexAttributeValues::Uint16x4(joints) =
                        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).unwrap()
                    else {
                        panic!()
                    };
                    positions
                        .iter()
                        .zip(joints)
                        .map(|(p, j)| {
                            let joint = usize::from(j[0]);
                            let node = nodes.get(&skin.joints[joint]).unwrap().index;
                            (worlds[node] * inverse[joint]).transform_point3(Vec3::from_array(*p))
                        })
                        .collect::<Vec<_>>()
                } else {
                    positions
                        .iter()
                        .map(|p| worlds[node].transform_point3(Vec3::from_array(*p)))
                        .collect()
                }
            };
            let mut expected = Vec::new();
            for (_, node) in nodes.iter() {
                if let Some(mesh) = node.mesh.as_ref() {
                    for primitive in &meshes.get(mesh).unwrap().primitives {
                        let material = primitive.material.clone().unwrap_or_default().id();
                        expected.extend(
                            positions(
                                node.index,
                                vertex_meshes.get(&primitive.mesh).unwrap(),
                                node.skin.as_ref(),
                            )
                            .into_iter()
                            .map(|p| (material, p)),
                        );
                    }
                }
            }
            let actual: Vec<_> = result
                .iter()
                .flat_map(|part| {
                    positions(
                        part.node,
                        vertex_meshes.get(&part.mesh).unwrap(),
                        part.skin.as_ref(),
                    )
                    .into_iter()
                    .map(|p| (part.material, p))
                })
                .collect();
            assert_eq!(actual.len(), expected.len());
            for (material, point) in actual {
                let found = expected
                    .iter()
                    .position(|(m, p)| *m == material && p.abs_diff_eq(point, 2e-4));
                let index = found.unwrap_or_else(|| {
                    panic!("joined vertex {point:?} changed its joint or material at angle {angle}")
                });
                expected.swap_remove(index);
            }
            assert!(expected.is_empty());
        }
    }

    #[test]
    fn animated_attachment_nodes_and_reflected_ancestors_keep_rigid_draws() {
        let bytes = include_bytes!("../../../../mods/skirmish/assets/models/soldier_anim.glb");
        let (mut gltf, nodes, meshes, mut vertex_meshes, skins, bindposes) = rigid_fixture(bytes);
        let node = nodes
            .iter()
            .find(|(_, n)| n.skin.is_none() && n.mesh.is_some())
            .unwrap()
            .1
            .index;
        let blob = gltf.source.as_mut().unwrap().blob.take();
        let mut json =
            serde_json::to_value(gltf.source.as_ref().unwrap().document.as_json()).unwrap();
        json["animations"][0]["channels"][0]["target"]["node"] = node.into();
        let mut source =
            bevy::gltf::gltf::Gltf::from_slice(&serde_json::to_vec(&json).unwrap()).unwrap();
        source.blob = blob;
        gltf.source = Some(source);
        let result = prepare(
            &gltf,
            &nodes,
            &meshes,
            &skins,
            &bindposes,
            &mut vertex_meshes,
        )
        .unwrap();
        assert!(result.iter().any(|p| p.node == node && p.skin.is_none()));
        let (gltf, mut nodes, meshes, mut vertex_meshes, skins, bindposes) = rigid_fixture(bytes);
        let root = gltf
            .source
            .as_ref()
            .unwrap()
            .skins()
            .next()
            .unwrap()
            .joints()
            .next()
            .unwrap()
            .index();
        nodes.get_mut(&gltf.nodes[root]).unwrap().transform.scale.x = -1.;
        let result = prepare(
            &gltf,
            &nodes,
            &meshes,
            &skins,
            &bindposes,
            &mut vertex_meshes,
        )
        .unwrap();
        assert!(result.iter().any(|p| p.skin.is_none()));
    }

    #[test]
    fn malformed_parent_cycles_stop_attachment_preparation() {
        let (gltf, nodes, meshes, vertex_meshes, skins, bindposes) = rigid_fixture(include_bytes!(
            "../../../../mods/skirmish/assets/models/soldier_anim.glb"
        ));
        let node = nodes
            .iter()
            .find(|(_, n)| n.skin.is_none() && n.mesh.is_some())
            .unwrap()
            .1;
        let mesh = &meshes.get(node.mesh.as_ref().unwrap()).unwrap().primitives[0].mesh;
        let parents: HashMap<_, _> = gltf
            .source
            .as_ref()
            .unwrap()
            .nodes()
            .flat_map(|n| n.children().map(move |c| (c.index(), n.index())))
            .collect();
        let root = gltf
            .source
            .as_ref()
            .unwrap()
            .skins()
            .next()
            .unwrap()
            .joints()
            .next()
            .unwrap()
            .index();
        let animated = HashSet::new();
        for cycle in [node.index, root] {
            let mut parents = parents.clone();
            parents.insert(cycle, cycle);
            let context = super::super::rigid_attachment::Context::new(
                &gltf, &nodes, &skins, &bindposes, &parents, &animated,
            );
            assert!(
                context
                    .convert(node.index, vertex_meshes.get(mesh).unwrap())
                    .is_none()
            );
        }
    }

    #[test]
    fn unbound_joint_attributes_do_not_select_a_skin_for_rigid_nodes() {
        let (gltf, nodes, meshes, mut vertex_meshes) = fixture(include_bytes!(
            "../../../../mods/skirmish/assets/models/soldier_anim.glb"
        ));
        let result = prepare(
            &gltf,
            &nodes,
            &meshes,
            &Assets::default(),
            &Assets::default(),
            &mut vertex_meshes,
        )
        .unwrap();
        assert!(result.iter().any(|p| p.skin.is_none()));
        for (_, mesh) in meshes.iter() {
            for primitive in &mesh.primitives {
                let source = vertex_meshes.get(&primitive.mesh).unwrap();
                assert!(source.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some());
                assert!(source.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some());
            }
        }
        for part in result.iter().filter(|p| p.skin.is_none()) {
            let mesh = vertex_meshes.get(&part.mesh).unwrap();
            assert!(mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none());
            assert!(mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_none());
            assert_eq!(mesh.count_vertices(), 3);
        }
        assert!(result.iter().filter(|p| p.skin.is_some()).all(|p| {
            vertex_meshes
                .get(&p.mesh)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                .is_some()
        }));
    }

    #[test]
    fn shipped_body_parts_join_by_skin_and_material_without_merging_attachments() {
        for bytes in [
            include_bytes!("../../../../mods/skirmish/assets/models/soldier_anim.glb").as_slice(),
            include_bytes!("../../../../mods/skirmish/assets/models/swordsman_anim.glb").as_slice(),
            include_bytes!("../../../../mods/skirmish/assets/models/worker_anim.glb").as_slice(),
        ] {
            let (gltf, nodes, meshes, mut vertex_meshes) = fixture(bytes);
            let original: usize = nodes
                .iter()
                .filter_map(|(_, n)| n.mesh.as_ref())
                .map(|m| meshes.get(m).unwrap().primitives.len())
                .sum();
            let attachments: usize = nodes
                .iter()
                .filter(|(_, n)| n.skin.is_none())
                .filter_map(|(_, n)| n.mesh.as_ref())
                .map(|m| meshes.get(m).unwrap().primitives.len())
                .sum();
            let result = prepare(
                &gltf,
                &nodes,
                &meshes,
                &Assets::default(),
                &Assets::default(),
                &mut vertex_meshes,
            )
            .unwrap();
            assert_eq!(result.iter().filter(|p| p.skin.is_some()).count(), 2);
            assert_eq!(
                result.iter().filter(|p| p.skin.is_none()).count(),
                attachments
            );
            assert_eq!(
                result
                    .iter()
                    .map(|p| vertex_meshes.get(&p.mesh).unwrap().count_vertices())
                    .sum::<usize>(),
                original * 3
            );
            assert!(result.len() < original);
        }
    }

    #[test]
    fn transparent_skin_parts_keep_separate_sort_positions() {
        let bytes = include_bytes!("../../../../mods/skirmish/assets/models/soldier_anim.glb");
        let source = bevy::gltf::gltf::Gltf::from_slice(bytes).unwrap();
        let mut json = serde_json::to_value(source.document.as_json()).unwrap();
        for material in json["materials"].as_array_mut().unwrap() {
            material["alphaMode"] = "BLEND".into();
        }
        let bytes = serde_json::to_vec(&json).unwrap();
        let (gltf, nodes, meshes, mut vertex_meshes) = fixture(&bytes);
        let original: usize = meshes.iter().map(|(_, m)| m.primitives.len()).sum();
        assert_eq!(
            prepare(
                &gltf,
                &nodes,
                &meshes,
                &Assets::default(),
                &Assets::default(),
                &mut vertex_meshes
            )
            .unwrap()
            .len(),
            original
        );
    }

    #[test]
    fn animated_or_transformed_mesh_nodes_keep_their_own_draw_entities() {
        let bytes = include_bytes!("../../../../mods/skirmish/assets/models/soldier_anim.glb");
        let (gltf, mut nodes, meshes, mut vertex_meshes) = fixture(bytes);
        let node = nodes
            .iter()
            .find(|(_, n)| n.skin.is_some() && n.mesh.is_some())
            .unwrap()
            .1
            .index;
        let normal = prepare(
            &gltf,
            &nodes,
            &meshes,
            &Assets::default(),
            &Assets::default(),
            &mut vertex_meshes,
        )
        .unwrap();
        nodes
            .get_mut(&gltf.nodes[node])
            .unwrap()
            .transform
            .translation
            .x = 1.;
        let transformed = prepare(
            &gltf,
            &nodes,
            &meshes,
            &Assets::default(),
            &Assets::default(),
            &mut vertex_meshes,
        )
        .unwrap();
        assert!(transformed.len() > normal.len());
        nodes.get_mut(&gltf.nodes[node]).unwrap().transform = Transform::IDENTITY;
        let mut json =
            serde_json::to_value(gltf.source.as_ref().unwrap().document.as_json()).unwrap();
        json["animations"][0]["channels"][0]["target"]["node"] = node.into();
        let source =
            bevy::gltf::gltf::Gltf::from_slice(&serde_json::to_vec(&json).unwrap()).unwrap();
        let gltf = Gltf {
            source: Some(source),
            ..gltf
        };
        let animated = prepare(
            &gltf,
            &nodes,
            &meshes,
            &Assets::default(),
            &Assets::default(),
            &mut vertex_meshes,
        )
        .unwrap();
        assert!(animated.len() > normal.len());
    }
}
