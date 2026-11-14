//! Join compatible skin parts once per asset. Keep every animation node and skin palette.
use bevy::{
    gltf::{Gltf, GltfMaterial, GltfMesh, GltfNode, GltfSkin},
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
    parts: usize,
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
            let key = data
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
            let incoming = mesh_assets.get(&primitive.mesh);
            let joined = key
                .and_then(|key| {
                    pending
                        .iter_mut()
                        .filter(|part| part.key == Some(key))
                        .find_map(|part| {
                            let merged = merge_compatible(part.mesh.as_ref()?, incoming?)?;
                            part.mesh = Some(merged);
                            part.parts += 1;
                            Some(())
                        })
                })
                .is_some();
            if !joined {
                pending.push(Pending {
                    primitive: RigPrimitive {
                        node: node.index(),
                        mesh: primitive.mesh.clone(),
                        material,
                        skin: data.skin.clone(),
                    },
                    key,
                    mesh: incoming.cloned(),
                    parts: 1,
                });
            }
        }
    }
    Some(
        pending
            .into_iter()
            .map(|mut part| {
                if part.parts > 1
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
            let result = prepare(&gltf, &nodes, &meshes, &mut vertex_meshes).unwrap();
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
            prepare(&gltf, &nodes, &meshes, &mut vertex_meshes)
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
        let normal = prepare(&gltf, &nodes, &meshes, &mut vertex_meshes).unwrap();
        nodes
            .get_mut(&gltf.nodes[node])
            .unwrap()
            .transform
            .translation
            .x = 1.;
        let transformed = prepare(&gltf, &nodes, &meshes, &mut vertex_meshes).unwrap();
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
        let animated = prepare(&gltf, &nodes, &meshes, &mut vertex_meshes).unwrap();
        assert!(animated.len() > normal.len());
    }
}
