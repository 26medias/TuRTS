//! A bounded map material layer. All masks are presentation in world X/Z coordinates.
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use turts_mod_api::GroundOverlayDef;

pub type GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>;

#[derive(Clone, Copy, Default, Debug, ShaderType)]
pub struct Patch {
    centre_rotation: Vec4,
    half_size_feather_opacity: Vec4,
}
#[derive(Clone, Debug, ShaderType)]
pub struct Layer {
    colour: Vec4,
    settings: Vec4,
    patches: [Patch; 16],
}
impl Layer {
    pub(crate) fn from_definition(definition: &GroundOverlayDef) -> Self {
        let colour = crate::widgets::primitives::parse_colour(&definition.colour)
            .unwrap_or(Color::WHITE)
            .to_linear();
        let mut layer = Self {
            colour: colour.to_vec4(),
            settings: Vec4::new(
                definition.patches.len().min(16) as f32,
                definition.tile_size,
                definition.roughness,
                0.0,
            ),
            patches: [Patch::default(); 16],
        };
        for (patch, source) in layer.patches.iter_mut().zip(&definition.patches) {
            let (sin, cos) = source.rotation_deg.to_radians().sin_cos();
            *patch = Patch {
                centre_rotation: Vec4::new(source.centre.x, source.centre.y, cos, sin),
                half_size_feather_opacity: Vec4::new(
                    source.size.x * 0.5,
                    source.size.y * 0.5,
                    source.feather,
                    source.opacity,
                ),
            };
        }
        layer
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct GroundExtension {
    #[uniform(100)]
    pub layer: Layer,
    #[texture(101)]
    #[sampler(102)]
    pub texture: Option<Handle<Image>>,
}
impl GroundExtension {
    pub(crate) fn observe(&self, images: &Assets<Image>) -> serde_json::Value {
        serde_json::json!({"patches":self.layer.settings.x as u32,"colour":self.layer.colour.to_array(),"tile_size":self.layer.settings.y,"roughness":self.layer.settings.z,"texture_size":self.texture.as_ref().and_then(|h| images.get(h)).map(|i|[i.width(),i.height()])})
    }
}
impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
        "embedded://turts_client/ground.wesl".into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        Self::fragment_shader()
    }
}
pub(crate) fn register(app: &mut App) {
    bevy::shader::load_shader_library!(app, "ground.wesl");
    app.add_plugins(MaterialPlugin::<GroundMaterial>::default());
}

/// Keep authored pixels separate from each quality-limited GPU image.
#[derive(Component)]
pub struct OverlaySource(pub Option<Handle<Image>>);

pub fn sync_quality(
    settings: Res<crate::models::GraphicsSettings>,
    ground: Query<(
        &crate::models::GroundSurface,
        &OverlaySource,
        &MeshMaterial3d<GroundMaterial>,
    )>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<crate::models::ModelState>,
) {
    if !settings.is_changed() {
        return;
    }
    for (source, overlay, handle) in &ground {
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        material.base.base_color_texture = source
            .base
            .as_ref()
            .map(|h| state.texture(h, settings.quality, &mut images));
        material.base.normal_map_texture = source
            .normal
            .as_ref()
            .map(|h| state.texture(h, settings.quality, &mut images));
        material.extension.texture = overlay
            .0
            .as_ref()
            .map(|h| state.texture(h, settings.quality, &mut images));
    }
}
