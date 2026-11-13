//! Shared particle materials and bounded instance storage.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    material::AlphaMode,
    mesh::{MeshTag, MeshVertexBufferLayoutRef},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    reflect::TypePath,
    render::{
        render_resource::{
            AsBindGroup, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError,
            TextureDimension, TextureFormat,
        },
        storage::ShaderBuffer,
    },
    shader::ShaderRef,
};
use turts_mod_api::{EffectBlend, EffectCost, EffectLayer};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ParticleMaterial {
    #[texture(0)]
    #[sampler(1)]
    image: Handle<Image>,
    #[storage(2, read_only)]
    colours: Handle<ShaderBuffer>,
    additive: bool,
}
impl Material for ParticleMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://turts_client/fx/particles.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.additive {
            AlphaMode::Add
        } else {
            AlphaMode::Blend
        }
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .vertex
            .shader_defs
            .push("VERTEX_OUTPUT_INSTANCE_INDEX".into());
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment
                .shader_defs
                .push("VERTEX_OUTPUT_INSTANCE_INDEX".into());
        }
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
#[derive(Clone)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub gravity: f32,
    pub age: f32,
    pub delay: f32,
    pub life: f32,
    pub size: f32,
    pub end_size: f32,
    pub colour: LinearRgba,
    pub end_colour: LinearRgba,
    pub cost: EffectCost,
    pub distance: f32,
    pub critical: bool,
    pub image: Option<String>,
    pub additive: bool,
    pub target: Option<Vec3>,
    pub start: Vec3,
    pub arc: f32,
    pub ground: bool,
    pub spin: f32,
}
#[derive(Default)]
struct Slot {
    entity: Option<Entity>,
    particle: Option<Particle>,
}
#[derive(Resource, Default)]
pub struct ParticlePool {
    slots: Vec<Slot>,
    pub limit: usize,
    active_count: usize,
    pub dropped: u64,
    pub peak: usize,
    mesh: Option<Handle<Mesh>>,
    colours: Option<Handle<ShaderBuffer>>,
    dot: Option<Handle<Image>>,
    images: HashMap<String, Handle<Image>>,
    materials: HashMap<(Option<String>, bool), Handle<ParticleMaterial>>,
}
impl ParticlePool {
    pub fn capacity(quality: crate::models::Quality) -> usize {
        match quality {
            crate::models::Quality::Low => 512,
            crate::models::Quality::Medium => 2048,
            crate::models::Quality::High => 8192,
        }
    }
    pub fn active(&self) -> usize {
        self.active_count
    }
    pub fn spawn(&mut self, particle: Particle) -> bool {
        let free = self.slots.iter().position(|slot| slot.particle.is_none());
        let index = free.or_else(|| {
            if self.slots.len() < self.limit {
                self.slots.push(Slot::default());
                Some(self.slots.len() - 1)
            } else {
                self.slots
                    .iter()
                    .enumerate()
                    .filter_map(|(i, slot)| {
                        let old = slot.particle.as_ref()?;
                        (!old.critical && (particle.critical || old.cost == EffectCost::Large))
                            .then_some((i, old.cost == EffectCost::Large, old.distance))
                    })
                    .max_by(|a, b| a.1.cmp(&b.1).then(a.2.total_cmp(&b.2)))
                    .map(|(i, _, _)| i)
            }
        });
        if let Some(index) = index {
            if self.slots[index].particle.is_none() {
                self.active_count += 1;
            }
            self.slots[index].particle = Some(particle);
            self.peak = self.peak.max(self.active());
            true
        } else {
            self.dropped += 1;
            false
        }
    }
}
fn dot_image() -> Image {
    let mut bytes = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64 {
        for x in 0..64 {
            let d = Vec2::new((x as f32 + 0.5) / 32. - 1., (y as f32 + 0.5) / 32. - 1.).length();
            bytes.extend([255, 255, 255, ((1. - d).clamp(0., 1.).powi(2) * 255.) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
pub fn register(app: &mut App) {
    bevy::shader::load_shader_library!(app, "particles.wesl");
    app.add_plugins(MaterialPlugin::<ParticleMaterial>::default())
        .init_resource::<ParticlePool>();
}
#[allow(clippy::too_many_arguments)]
pub fn update_particles(
    time: Res<Time>,
    settings: Res<crate::models::GraphicsSettings>,
    mut commands: Commands,
    mut pool: ResMut<ParticlePool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
    mut shown: Query<(
        &mut Transform,
        &mut Visibility,
        &mut MeshMaterial3d<ParticleMaterial>,
    )>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
) {
    pool.limit = ParticlePool::capacity(settings.quality);
    let limit = pool.limit;
    let retained = limit.min(pool.slots.len());
    for slot in pool.slots.drain(retained..) {
        if let Some(entity) = slot.entity {
            commands.entity(entity).despawn();
        }
    }
    let rotation = camera
        .iter()
        .next()
        .map_or(Quat::IDENTITY, |c| c.rotation());
    if pool.mesh.is_none() {
        pool.mesh = Some(meshes.add(Rectangle::new(1., 1.)));
    }
    if pool.dot.is_none() {
        pool.dot = Some(images.add(dot_image()));
    }
    if pool.colours.is_none() {
        pool.colours = Some(buffers.add(ShaderBuffer::from(vec![[0_f32; 4]; 8192])));
    }
    let mut colours = vec![[0_f32; 4]; 8192];
    let delta = time.delta_secs();
    for index in 0..pool.slots.len() {
        let mut slot = std::mem::take(&mut pool.slots[index]);
        if let Some(p) = slot.particle.as_mut() {
            p.age += delta;
            if p.age >= p.delay + p.life {
                slot.particle = None;
            }
        }
        if let Some(p) = slot.particle.as_mut() {
            let t = ((p.age - p.delay) / p.life.max(0.001)).clamp(0., 1.);
            if p.age >= p.delay {
                if let Some(target) = p.target {
                    p.position = p.start.lerp(target, t) + Vec3::Y * (4. * p.arc * t * (1. - t));
                } else {
                    p.velocity.y -= p.gravity * delta;
                    p.position += p.velocity * delta;
                }
            }
            let key = (p.image.clone(), p.additive);
            let material = if let Some(material) = pool.materials.get(&key) {
                material.clone()
            } else {
                let material = materials.add(ParticleMaterial {
                    image: key
                        .0
                        .as_ref()
                        .and_then(|id| pool.images.get(id))
                        .unwrap_or(pool.dot.as_ref().unwrap())
                        .clone(),
                    colours: pool.colours.as_ref().unwrap().clone(),
                    additive: key.1,
                });
                pool.materials.insert(key, material.clone());
                material
            };
            let colour = p.colour.to_vec4().lerp(p.end_colour.to_vec4(), t);
            colours[index] = colour.to_array();
            let transform = Transform {
                translation: p.position,
                rotation: if p.ground {
                    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                } else {
                    rotation * Quat::from_rotation_z(p.spin * p.age)
                },
                scale: Vec3::splat(p.size + (p.end_size - p.size) * t),
            };
            let visibility = if p.age < p.delay {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            if let Some(entity) = slot.entity {
                if let Ok((mut tr, mut vis, mut mat)) = shown.get_mut(entity) {
                    *tr = transform;
                    *vis = visibility;
                    mat.0 = material;
                }
            } else {
                slot.entity = Some(
                    commands
                        .spawn((
                            Mesh3d(pool.mesh.as_ref().unwrap().clone()),
                            MeshMaterial3d(material),
                            MeshTag::new(index as u32),
                            transform,
                            visibility,
                            NotShadowCaster,
                            NotShadowReceiver,
                        ))
                        .id(),
                );
            }
        } else if let Some(entity) = slot.entity
            && let Ok((_, mut visibility, _)) = shown.get_mut(entity)
        {
            *visibility = Visibility::Hidden;
        }
        pool.slots[index] = slot;
    }
    pool.active_count = pool.slots.iter().filter(|s| s.particle.is_some()).count();
    if let Some(mut buffer) = pool.colours.as_ref().and_then(|h| buffers.get_mut(h)) {
        buffer.clear();
        buffer.extend_from_slice(&colours);
    }
}

pub fn colour(text: &str) -> LinearRgba {
    Color::srgba_u8(
        u8::from_str_radix(text.get(1..3).unwrap_or("ff"), 16).unwrap_or(255),
        u8::from_str_radix(text.get(3..5).unwrap_or("ff"), 16).unwrap_or(255),
        u8::from_str_radix(text.get(5..7).unwrap_or("ff"), 16).unwrap_or(255),
        u8::from_str_radix(text.get(7..9).unwrap_or("ff"), 16).unwrap_or(255),
    )
    .to_linear()
}
pub fn particle(
    layer: &EffectLayer,
    position: Vec3,
    cost: EffectCost,
    distance: f32,
    critical: bool,
) -> Particle {
    let colour = colour(&layer.colour);
    let mut end = layer.end_colour.as_deref().map_or(colour, self::colour);
    end.alpha = 0.;
    Particle {
        position: position + Vec3::from_array(layer.offset),
        start: position,
        velocity: Vec3::ZERO,
        gravity: 0.,
        age: 0.,
        delay: layer.delay_ms as f32 / 1000.,
        life: layer.ms as f32 / 1000.,
        size: layer.size,
        end_size: layer.end_size.unwrap_or(layer.size),
        colour,
        end_colour: end,
        cost,
        distance,
        critical,
        image: layer.image.clone(),
        additive: layer.blend == EffectBlend::Add,
        target: None,
        arc: 0.,
        ground: false,
        spin: 0.,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(cost: EffectCost, distance: f32, critical: bool) -> Particle {
        let layer: EffectLayer = ron::from_str("(kind:Flash())").unwrap();
        particle(&layer, Vec3::ZERO, cost, distance, critical)
    }
    #[test]
    fn fx7_low_pool_never_grows_and_critical_hits_replace_large_cosmetics() {
        let mut pool = ParticlePool {
            limit: 512,
            ..Default::default()
        };
        for i in 0..512 {
            assert!(pool.spawn(sample(EffectCost::Large, i as f32, false)));
        }
        assert!(pool.spawn(sample(EffectCost::Small, 0., true)));
        assert_eq!(pool.active(), 512);
        assert_eq!(pool.slots.len(), 512);
        assert!(
            pool.slots
                .iter()
                .any(|s| s.particle.as_ref().is_some_and(|p| p.critical))
        );
        for _ in 0..1000 {
            pool.spawn(sample(EffectCost::Small, 0., true));
        }
        assert_eq!(pool.slots.len(), 512);
        assert_eq!(pool.active(), 512);
    }
}
