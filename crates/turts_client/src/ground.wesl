// Forward/deferred PBR path from Bevy 0.20.0 (MIT OR Apache-2.0), with a bounded ground layer.
import bevy_pbr::{
    render::{
        pbr_types,
        pbr_functions::alpha_discard,
        pbr_fragment::pbr_input_from_standard_material,
    },
    decal::clustered::apply_decals,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
};

@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;

@if(MESHLET_MESH_MATERIAL_PASS)
import bevy_pbr::meshlet::visibility_buffer_resolve::resolve_vertex_output;

@if(MATERIAL_OIT_ENABLED)
import bevy_core_pipeline::oit::draw::oit_draw;

@if(FORWARD_DECAL)
import bevy_pbr::decal::forward::get_forward_decal_info;

struct Patch {
    centre_rotation: vec4<f32>,
    half_size_feather_opacity: vec4<f32>,
}
struct Layer {
    colour: vec4<f32>,
    settings: vec4<f32>,
    patches: array<Patch, 16>,
}
@group(3) @binding(100) var<uniform> layer: Layer;
@group(3) @binding(101) var layer_texture: texture_2d<f32>;
@group(3) @binding(102) var layer_sampler: sampler;

fn layer_mask(world: vec2<f32>) -> f32 {
    var mask = 0.0;
    for (var i = 0u; i < min(u32(layer.settings.x),16u); i += 1u) {
        let zone = layer.patches[i];
        let delta = world - zone.centre_rotation.xy;
        let c = zone.centre_rotation.z;
        let s = zone.centre_rotation.w;
        let local = vec2<f32>(c*delta.x+s*delta.y, -s*delta.x+c*delta.y);
        let edge = zone.half_size_feather_opacity.xy - abs(local);
        let weight = smoothstep(0.0, max(zone.half_size_feather_opacity.z, 0.00001), min(edge.x,edge.y));
        mask = max(mask, weight * zone.half_size_feather_opacity.w);
    }
    return mask;
}

@fragment
fn fragment(
@if(MESHLET_MESH_MATERIAL_PASS)
    @builtin(position) frag_coord: vec4<f32>,
@if(!MESHLET_MESH_MATERIAL_PASS)
    vertex_output: VertexOutput,
@if(!MESHLET_MESH_MATERIAL_PASS)
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
@if(MESHLET_MESH_MATERIAL_PASS)
    let vertex_output = resolve_vertex_output(frag_coord);
@if(MESHLET_MESH_MATERIAL_PASS)
    let is_front = true;

    var in = vertex_output;

    // If we're in the crossfade section of a visibility range, conditionally
    // discard the fragment according to the visibility pattern.
@if(VISIBILITY_RANGE_DITHER)
    visibility_range_dither(in.position, in.visibility_range_dither);

@if(FORWARD_DECAL)
    let forward_decal_info = get_forward_decal_info(in);
@if(FORWARD_DECAL)
    in.world_position = forward_decal_info.world_position;
@if(FORWARD_DECAL)
    in.uv = forward_decal_info.uv;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let mask = layer_mask(in.world_position.xz);
    // Always sample outside the varying mask branch so texture derivatives stay valid.
    let colour = layer.colour * textureSample(layer_texture, layer_sampler, in.world_position.xz / layer.settings.y);
    pbr_input.material.base_color = mix(pbr_input.material.base_color, colour, mask);
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, layer.settings.z, mask);
    pbr_input.N = normalize(mix(pbr_input.N, pbr_input.world_normal, mask));

    // alpha discard
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color
    );

    // clustered decals
    apply_decals(&pbr_input);

    // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
@if(PREPASS_PIPELINE)
    let out = deferred_output(in, pbr_input);
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
@if(!PREPASS_PIPELINE)
    var out: FragmentOutput;
@if(!PREPASS_PIPELINE)
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
@if(!PREPASS_PIPELINE)
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

@if(MATERIAL_OIT_ENABLED) {
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, vec4(out.color.rgb * out.color.a, out.color.a));
        discard;
    }
    // Both `Premultiplied` and `Add` colors are premultiplied in `premultiply_alpha()`
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED
        || alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
}

@if(FORWARD_DECAL)
        out.color.a = min(forward_decal_info.alpha, out.color.a);

        return out;
}
