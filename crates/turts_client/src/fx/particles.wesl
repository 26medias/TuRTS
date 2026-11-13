import bevy_pbr::render::{forward_io::VertexOutput, mesh_functions};
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var dot_image: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var dot_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<storage, read> colours: array<vec4<f32>>;
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let tint = colours[mesh_functions::get_tag(in.instance_index)];
    let colour = textureSample(dot_image, dot_sampler, in.uv) * tint;
    if colour.a < 0.005 { discard; }
    return colour;
}
