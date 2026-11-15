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
    pub fresh: bool,
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
    pub model: Option<String>,
    pub follow: Option<u32>,
    pub additive: bool,
    pub target: Option<Vec3>,
    pub start: Vec3,
    pub arc: f32,
    pub ground: bool,
    pub spin: f32,
}
#[derive(Default)]
pub(super) struct Slot {
    pub(super) entity: Option<Entity>,
    pub(super) particle: Option<Particle>,
}
#[derive(Resource, Default)]
pub struct ParticlePool {
    pub(super) slots: Vec<Slot>,
    pub limit: usize,
    active_count: usize,
    free: Vec<usize>,
    pub dropped: u64,
    pub dropped_by_cost: [u64; 3],
    pub peak: usize,
    pub(super) mesh: Option<Handle<Mesh>>,
    pub(super) colours: Option<Handle<ShaderBuffer>>,
    pub(super) dot: Option<Handle<Image>>,
    pub(super) images: HashMap<String, Handle<Image>>,
    image_source: Option<String>,
    materials: HashMap<(Option<String>, bool), Handle<ParticleMaterial>>,
}
impl ParticlePool {
    pub(super) fn material(
        &mut self,
        image: Option<String>,
        additive: bool,
        materials: &mut Assets<ParticleMaterial>,
    ) -> Handle<ParticleMaterial> {
        let key = (image, additive);
        if let Some(handle) = self.materials.get(&key) {
            return handle.clone();
        }
        let material = materials.add(ParticleMaterial {
            image: key
                .0
                .as_ref()
                .and_then(|id| self.images.get(id))
                .unwrap_or(self.dot.as_ref().unwrap())
                .clone(),
            colours: self.colours.as_ref().unwrap().clone(),
            additive,
        });
        self.materials.insert(key, material.clone());
        material
    }
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
    fn cost_rank(cost: EffectCost) -> usize {
        match cost {
            EffectCost::Small => 0,
            EffectCost::Medium => 1,
            EffectCost::Large => 2,
        }
    }
    fn may_replace(old: &Particle, new: &Particle) -> bool {
        !old.critical
            && (new.critical
                || Self::cost_rank(old.cost) > Self::cost_rank(new.cost)
                || (old.cost == new.cost && old.distance > new.distance))
    }
    /// Reserve a whole particle composition before changing any live slot.
    pub fn spawn_batch(&mut self, particles: Vec<Particle>) -> bool {
        let Some(first) = particles.first() else {
            return true;
        };
        let free = self.limit.saturating_sub(self.active_count);
        let needed = particles.len().saturating_sub(free);
        let replaceable = if needed == 0 {
            0
        } else {
            self.slots
                .iter()
                .filter(|slot| {
                    slot.particle
                        .as_ref()
                        .is_some_and(|old| Self::may_replace(old, first))
                })
                .take(needed)
                .count()
        };
        if free + replaceable < particles.len() {
            self.dropped += 1;
            self.dropped_by_cost[Self::cost_rank(first.cost)] += 1;
            return false;
        }
        for particle in particles {
            let accepted = self.spawn(particle);
            debug_assert!(accepted);
        }
        true
    }
    pub fn spawn(&mut self, particle: Particle) -> bool {
        let free = self.free.pop().filter(|index| {
            *index < self.slots.len()
                && *index < self.limit
                && self.slots[*index].particle.is_none()
        });
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
                        Self::may_replace(old, &particle).then_some((
                            i,
                            Self::cost_rank(old.cost),
                            old.distance,
                        ))
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
pub(super) fn dot_image() -> Image {
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
    state: Option<Res<ReplicaState>>,
    time: Res<Time>,
    acquired: Option<Res<AcquiredPackages>>,
    decals: Res<super::emitters::Emitters>,
    combat: Res<CombatPresentation>,
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
        pool.colours = Some(buffers.add(ShaderBuffer::from(vec![[0_f32; 4]; 8320])));
    }
    if let Some(acquired) = acquired
        && pool.image_source.as_ref() != Some(&acquired.0.lockfile_hash)
    {
        pool.images.clear();
        pool.materials.clear();
        for (id, image) in &acquired.0.presentation.images {
            if let Ok(decoded) = image::load_from_memory(&image.bytes) {
                let rgba = decoded.to_rgba8();
                let (width, height) = rgba.dimensions();
                if width <= 512 && height <= 512 {
                    let handle = images.add(Image::new(
                        Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        rgba.into_raw(),
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::default(),
                    ));
                    pool.images.insert(id.clone(), handle);
                }
            }
        }
        pool.image_source = Some(acquired.0.lockfile_hash.clone());
    }
    let mut colours = vec![[0_f32; 4]; 8320];
    let delta = time.delta_secs();
    for (index, tint) in colours.iter_mut().enumerate().take(pool.slots.len()) {
        let mut slot = std::mem::take(&mut pool.slots[index]);
        if let Some(p) = slot.particle.as_mut() {
            if p.fresh {
                p.fresh = false;
            } else {
                p.age += delta;
            }
            if p.age >= p.delay + p.life {
                slot.particle = None;
            }
        }
        if slot.particle.as_ref().is_some_and(|particle| {
            !super::permitted_position(state.as_deref(), particle.position)
                || particle.follow.is_some_and(|id| {
                    state
                        .as_ref()
                        .and_then(|state| state.replica.as_ref())
                        .is_some_and(|replica| {
                            replica.knowledge.is_some() && !replica.units.contains_key(&id)
                        })
                })
        }) {
            slot.particle = None;
        }
        if let Some(p) = slot.particle.as_mut() {
            let t = ((p.age - p.delay) / p.life.max(0.001)).clamp(0., 1.);
            if p.age >= p.delay {
                if let Some(id) = p.follow
                    && let Some(actor) = combat.actors.get(&id)
                {
                    p.target = Some(actor.position + Vec3::Y * actor.height * 0.5);
                }
                if let Some(target) = p.target {
                    p.position = p.start.lerp(target, t) + Vec3::Y * (4. * p.arc * t * (1. - t));
                } else {
                    p.velocity.y -= p.gravity * delta;
                    p.position += p.velocity * delta;
                }
            }
            let material = pool.material(p.image.clone(), p.additive, &mut materials);
            let colour = p.colour.to_vec4().lerp(p.end_colour.to_vec4(), t);
            *tint = colour.to_array();
            let transform = Transform {
                translation: p.position,
                rotation: if p.model.is_some() {
                    if let Some(target) = p.target {
                        Quat::from_rotation_arc(Vec3::Z, (target - p.start).normalize_or(Vec3::Z))
                    } else {
                        Quat::from_euler(
                            EulerRot::XYZ,
                            p.spin * p.age * 0.7,
                            p.spin * p.age * 0.4,
                            p.spin * p.age,
                        )
                    }
                } else if p.ground {
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
                    if *vis != visibility {
                        *vis = visibility;
                    }
                    if mat.0 != material {
                        mat.0 = material;
                    }
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
            && *visibility != Visibility::Hidden
        {
            *visibility = Visibility::Hidden;
        }
        pool.slots[index] = slot;
    }
    for decal in &decals.decals {
        colours[8192 + (decal.id % 128) as usize] = decal.tint(combat.now).to_array();
    }
    pool.free = pool
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| slot.particle.is_none().then_some(index))
        .collect();
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
    if layer.end_colour.is_none() {
        end.alpha = 0.;
    }
    Particle {
        position: position + Vec3::from_array(layer.offset),
        start: position + Vec3::from_array(layer.offset),
        velocity: Vec3::ZERO,
        gravity: 0.,
        age: 0.,
        fresh: true,
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
        model: None,
        follow: None,
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

#[cfg(test)]
mod batch_tests {
    use super::*;
    fn sample(cost: EffectCost, distance: f32, critical: bool) -> Particle {
        let layer: EffectLayer = ron::from_str("(kind:Flash())").unwrap();
        particle(&layer, Vec3::ZERO, cost, distance, critical)
    }
    #[test]
    fn fx7_an_effect_reserves_all_its_particles_or_none() {
        let mut pool = ParticlePool {
            limit: 3,
            ..Default::default()
        };
        pool.spawn(sample(EffectCost::Small, 0., true));
        pool.spawn(sample(EffectCost::Small, 0., true));
        assert!(!pool.spawn_batch(vec![sample(EffectCost::Small, 1., true); 2]));
        assert_eq!(pool.active(), 2);
        assert_eq!(pool.slots.len(), 2);
        assert_eq!(pool.dropped_by_cost, [1, 0, 0]);
    }
    #[test]
    fn fx7_near_small_effect_replaces_the_farthest_large_effect() {
        let mut pool = ParticlePool {
            limit: 3,
            ..Default::default()
        };
        for distance in [10., 30., 20.] {
            pool.spawn(sample(EffectCost::Large, distance, false));
        }
        assert!(pool.spawn_batch(vec![sample(EffectCost::Small, 1., false)]));
        let distances: Vec<_> = pool
            .slots
            .iter()
            .filter_map(|slot| slot.particle.as_ref().map(|p| p.distance))
            .collect();
        assert!(distances.contains(&10.) && distances.contains(&20.) && distances.contains(&1.));
        assert!(!distances.contains(&30.));
    }
}
