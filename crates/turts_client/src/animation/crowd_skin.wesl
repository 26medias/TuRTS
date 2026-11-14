// Skin math from Bevy 0.20.0 (MIT OR Apache-2.0), with a shared anchor-relative palette.
import bevy_pbr::render::mesh_functions;

@group(3) @binding(100) var<storage, read> shared_palettes: array<mat4x4<f32>>;

fn skin_model(indexes: vec4<u32>, weights: vec4<f32>, instance_index: u32) -> mat4x4<f32> {
    let base = mesh_functions::get_tag(instance_index) >> 1u;
    let local = weights.x * shared_palettes[base + indexes.x]
        + weights.y * shared_palettes[base + indexes.y]
        + weights.z * shared_palettes[base + indexes.z]
        + weights.w * shared_palettes[base + indexes.w];
    return mesh_functions::get_world_from_local(instance_index) * local;
}

fn inverse_transpose_3x3m(matrix: mat3x3<f32>) -> mat3x3<f32> {
    let x = cross(matrix[1], matrix[2]);
    let y = cross(matrix[2], matrix[0]);
    let z = cross(matrix[0], matrix[1]);
    let det = dot(matrix[2], z);
    return mat3x3<f32>(x / det, y / det, z / det);
}

fn skin_normals(world_from_local: mat4x4<f32>, normal: vec3<f32>) -> vec3<f32> {
    return normalize(inverse_transpose_3x3m(mat3x3<f32>(world_from_local[0].xyz, world_from_local[1].xyz, world_from_local[2].xyz)) * normal);
}
