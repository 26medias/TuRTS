//! Package-owned glTF bytes, shared render assets, client quality, LOD and facing.
use crate::{
    ingest::ReplicaState,
    join::AcquiredPackages,
    present::Presented,
    render::{PresentedNode, PresentedUnit, SlotMaterials},
};
use bevy::{
    asset::{
        AssetApp, AssetId,
        io::{
            AssetSourceBuilder,
            memory::{Dir, MemoryAssetReader},
        },
    },
    gltf::{Gltf, GltfLoaderSettings, GltfMaterial, GltfMesh, GltfNode, GltfSkin},
    prelude::*,
};
use std::{collections::HashMap, path::Path};

#[derive(Resource, Clone, Default)]
pub struct PackageModelSource(Dir);
/// Register before AssetPlugin. No package model is ever resolved through an HTTP reader.
pub fn register_model_source(app: &mut App) {
    #[cfg(target_family = "wasm")]
    if let Some(delay) = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| {
            q.trim_start_matches('?').split('&').find_map(|v| {
                v.strip_prefix("test_model_delay_ms=")
                    .and_then(|v| v.parse::<u64>().ok())
            })
        })
    {
        app.insert_resource(ModelState {
            ready_delay_ms: delay.min(10000),
            ..Default::default()
        });
    }
    let dir = Dir::default();
    let reader = dir.clone();
    app.register_asset_source(
        "package",
        AssetSourceBuilder::new(move || {
            Box::new(MemoryAssetReader {
                root: reader.clone(),
            })
        }),
    );
    app.insert_resource(PackageModelSource(dir));
}
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Low,
    Medium,
    #[default]
    High,
}
impl Quality {
    pub fn next(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Low,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }
    fn texture_limit(self) -> u32 {
        match self {
            Self::Low => 512,
            Self::Medium => 1024,
            Self::High => 4096,
        }
    }
    fn lod_bias(self) -> f32 {
        match self {
            Self::Low => 0.5,
            Self::Medium => 0.75,
            Self::High => 1.0,
        }
    }
}
#[derive(Resource, Default, Clone, Copy)]
pub struct GraphicsSettings {
    pub quality: Quality,
}
#[derive(Clone)]
struct Primitive {
    mesh: Handle<Mesh>,
    material: Handle<GltfMaterial>,
    transform: Transform,
}
#[derive(Component)]
pub struct ModelInstance {
    pub ready: bool,
    ticket: u64,
    retired_children: Vec<Entity>,
    pub asset: String,
    pub level: usize,
    pub quality: Quality,
    children: Vec<Entity>,
}
#[derive(Resource, Default)]
pub struct ModelState {
    next_gpu_ticket: u64,
    handles: HashMap<String, Handle<Gltf>>,
    templates: HashMap<String, Vec<Primitive>>,
    materials: HashMap<
        (AssetId<GltfMaterial>, Option<u8>, Quality),
        Handle<crate::fx::flash::ModelMaterial>,
    >,
    images: HashMap<(AssetId<Image>, Quality), Handle<Image>>,
    facing: HashMap<u32, (Vec2, f32)>,
    /// Test driver may delay readiness. It never changes authoritative state.
    pub ready_delay_ms: u64,
    first_ready: HashMap<String, f64>,
}
impl ModelState {
    pub(crate) fn effect_model(&self, id: &str) -> Option<Handle<Gltf>> {
        self.handles.get(id).cloned()
    }

    pub(crate) fn texture(
        &mut self,
        source: &Handle<Image>,
        quality: Quality,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        limited_texture(source, quality, images, &mut self.images)
    }
    pub fn yaw(&self, id: u32) -> Option<f32> {
        self.facing.get(&id).map(|(_, yaw)| *yaw)
    }
    pub fn yaws(&self) -> HashMap<u32, f32> {
        self.facing
            .iter()
            .map(|(id, (_, yaw))| (*id, *yaw))
            .collect()
    }
}
#[derive(bevy::ecs::system::SystemParam)]
pub struct ModelSources<'w> {
    acquired: Option<Res<'w, AcquiredPackages>>,
    source: Option<Res<'w, PackageModelSource>>,
    server: Res<'w, AssetServer>,
    gltfs: Res<'w, Assets<Gltf>>,
    meshes: Res<'w, Assets<GltfMesh>>,
    nodes: Res<'w, Assets<GltfNode>>,
    skins: Res<'w, Assets<GltfSkin>>,
    bindposes: Res<'w, Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
    clips: Res<'w, Assets<AnimationClip>>,
    materials: Res<'w, Assets<GltfMaterial>>,
    palette: Res<'w, SlotMaterials>,
    replica: Res<'w, ReplicaState>,
    presented: Res<'w, Presented>,
    time: Res<'w, Time>,
    settings: Res<'w, GraphicsSettings>,
}
fn flatten(
    handle: &Handle<GltfNode>,
    parent: Transform,
    nodes: &Assets<GltfNode>,
    meshes: &Assets<GltfMesh>,
    out: &mut Vec<Primitive>,
) -> Option<()> {
    let node = nodes.get(handle)?;
    let transform = parent.mul_transform(node.transform);
    if let Some(mesh) = &node.mesh {
        for p in &meshes.get(mesh)?.primitives {
            out.push(Primitive {
                mesh: p.mesh.clone(),
                material: p.material.clone().unwrap_or_default(),
                transform,
            });
        }
    }
    for child in &node.children {
        flatten(child, transform, nodes, meshes, out)?;
    }
    Some(())
}
/// Select by projected model height. The previous level adds hysteresis around each threshold.
pub fn select_lod(
    model: &turts_mod_api::ModelDef,
    screen_height: f32,
    quality: Quality,
    previous: usize,
) -> usize {
    let height = screen_height * quality.lod_bias();
    let mut level = 0;
    for (i, lod) in model.lods.iter().enumerate() {
        let hysteresis = if previous > i { 1.08 } else { 0.92 };
        if height < lod.below_screen_height * hysteresis {
            level = i + 1;
        }
    }
    level
}
/// Rate-limited visual yaw (+Z forward), independent of server movement and combat.
pub fn turn_toward(yaw: f32, direction: Vec2, dt: f32) -> f32 {
    if direction.length_squared() < 1e-8 {
        return yaw;
    }
    let target = direction.x.atan2(direction.y);
    let delta = (target - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    yaw + delta.clamp(
        -std::f32::consts::TAU / 0.4 * dt,
        std::f32::consts::TAU / 0.4 * dt,
    )
}
fn limited_texture(
    handle: &Handle<Image>,
    quality: Quality,
    images: &mut Assets<Image>,
    cache: &mut HashMap<(AssetId<Image>, Quality), Handle<Image>>,
) -> Handle<Image> {
    if let Some(result) = cache.get(&(handle.id(), quality)) {
        return result.clone();
    }
    let Some(source) = images.get(handle) else {
        return handle.clone();
    };
    let limit = quality.texture_limit();
    if source.width() <= limit && source.height() <= limit {
        let mut result = source.clone();
        result.asset_usage = bevy::asset::RenderAssetUsages::default();
        let result = images.add(result);
        cache.insert((handle.id(), quality), result.clone());
        return result;
    }
    let Ok(dynamic) = source.clone().try_into_dynamic() else {
        return handle.clone();
    };
    let scaled = dynamic.resize(limit, limit, image::imageops::FilterType::Triangle);
    let mut result = Image::from_dynamic(
        scaled,
        source.texture_descriptor.format.is_srgb(),
        bevy::asset::RenderAssetUsages::default(),
    );
    result.sampler = source.sampler.clone();
    let result = images.add(result);
    cache.insert((handle.id(), quality), result.clone());
    result
}

/// Models replace the fallback atomically after their dependent meshes and textures are ready.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_models(
    mut commands: Commands,
    sources: ModelSources,
    mut state: ResMut<ModelState>,
    bridge: Res<ModelGpuBridge>,
    mut library: ResMut<crate::animation::AnimationLibrary>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    material_assets: Res<Assets<StandardMaterial>>,
    mut model_materials: ResMut<Assets<crate::fx::flash::ModelMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut shown: Query<
        (
            Entity,
            Option<&PresentedUnit>,
            Option<&PresentedNode>,
            &mut Transform,
            Option<&mut ModelInstance>,
            Option<&mut crate::animation::UnitAnimation>,
        ),
        Or<(With<PresentedUnit>, With<PresentedNode>)>,
    >,
) {
    let Some(acquired) = sources.acquired.as_deref() else {
        return;
    };
    let Some(source) = sources.source.as_deref() else {
        return;
    };
    let presentation = &acquired.0.presentation;
    if presentation.models.is_empty() {
        return;
    }
    for (id, bytes) in &presentation.models {
        if !state.handles.contains_key(id) {
            let path = format!("{}.glb", id.replace(':', "/"));
            source.0.insert_asset(Path::new(&path), bytes.clone());
            let handle = sources
                .server
                .load_builder()
                .with_settings(|settings: &mut GltfLoaderSettings| {
                    settings.include_source = true;
                    settings.load_animations = true;
                    settings.load_cameras = false;
                    settings.load_lights = false;
                    settings.load_materials = bevy::asset::RenderAssetUsages::MAIN_WORLD;
                })
                .load(format!("package://{path}"));
            state.handles.insert(id.clone(), handle);
        }
    }
    let live_tickets: std::collections::HashSet<_> = shown
        .iter()
        .filter_map(|(_, _, _, _, instance, _)| instance.map(|i| i.ticket))
        .collect();
    let live_roots: std::collections::HashSet<_> =
        shown.iter().map(|(entity, _, _, _, _, _)| entity).collect();
    if let Ok(mut bridge) = bridge.0.lock() {
        bridge.pending.retain(|ticket, request| {
            live_roots.contains(&request.root) && live_tickets.contains(ticket)
        });
        bridge
            .ready
            .retain(|ticket, root| live_roots.contains(root) && live_tickets.contains(ticket));
    }
    let now = sources.time.elapsed_secs_f64();
    let handles: Vec<_> = state
        .handles
        .iter()
        .map(|(id, h)| (id.clone(), h.clone()))
        .collect();
    for (id, handle) in handles {
        if state.templates.contains_key(&id) || !sources.server.is_loaded_with_dependencies(&handle)
        {
            continue;
        }
        let delay = state.ready_delay_ms as f64 / 1000.;
        let ready = *state.first_ready.entry(id.clone()).or_insert(now);
        if now - ready < delay {
            continue;
        }
        let Some(gltf) = sources.gltfs.get(&handle) else {
            continue;
        };
        let Some(document) = gltf.source.as_ref() else {
            continue;
        };
        let mut primitives = Vec::new();
        let mut complete = true;
        let scenes: Vec<_> = document
            .default_scene()
            .into_iter()
            .chain(if document.default_scene().is_none() {
                document.scenes().collect()
            } else {
                Vec::new()
            })
            .collect();
        for scene in scenes {
            for node in scene.nodes() {
                if flatten(
                    &gltf.nodes[node.index()],
                    Transform::IDENTITY,
                    &sources.nodes,
                    &sources.meshes,
                    &mut primitives,
                )
                .is_none()
                {
                    complete = false;
                }
            }
        }
        if complete && !primitives.is_empty() {
            state.templates.insert(id, primitives);
        }
    }
    let quality = sources.settings.quality;
    let mut shown_units = HashMap::with_capacity(sources.presented.units.len());
    for unit in &sources.presented.units {
        shown_units.entry(unit.unit_id).or_insert(unit);
    }
    let mut weapon_targets = HashMap::new();
    let mut shown_nodes = HashMap::new();
    if let Some(replica) = sources.replica.replica.as_ref() {
        for weapon in &replica.weapons {
            weapon_targets
                .entry(weapon.unit.0)
                .or_insert(weapon.last_target);
        }
        for node in replica.nodes.values() {
            shown_nodes.entry(node.id.0).or_insert(node);
        }
    }
    for (entity, unit, node, mut transform, mut instance, mut animation) in &mut shown {
        if let Some(instance) = instance.as_deref_mut()
            && !instance.ready
            && bridge
                .0
                .lock()
                .is_ok_and(|mut b| b.ready.remove(&instance.ticket).is_some())
        {
            instance.ready = true;
            commands
                .entity(entity)
                .remove::<Mesh3d>()
                .remove::<MeshMaterial3d<StandardMaterial>>();
            for child in instance.retired_children.drain(..) {
                commands.entity(child).despawn();
            }
        }
        let (id, content, pos, owner) = if let Some(unit) = unit {
            let Some(replica) = sources
                .replica
                .replica
                .as_ref()
                .and_then(|r| r.units.get(&unit.0))
            else {
                continue;
            };
            let Some(presented) = shown_units.get(&unit.0) else {
                continue;
            };
            (
                unit.0,
                replica.content_id.as_str(),
                Vec2::new(presented.pos.x, presented.pos.y),
                Some(replica.owner_slot),
            )
        } else if let Some(node) = node {
            let Some(node) = shown_nodes.get(&node.0) else {
                continue;
            };
            (
                node.id.0,
                node.content_id.as_str(),
                Vec2::new(node.pos.x, node.pos.y),
                None,
            )
        } else {
            continue;
        };
        let Some(model) = presentation.model_visuals.get(content) else {
            continue;
        };
        let Some((min, max)) = presentation.model_bounds.get(&model.asset) else {
            continue;
        };
        let height = (max[1] - min[1]) * model.scale;
        let screen_height = cameras
            .iter()
            .next()
            .and_then(|(camera, global)| {
                let a = camera
                    .world_to_viewport(global, Vec3::new(pos.x, 0., pos.y))
                    .ok()?;
                let b = camera
                    .world_to_viewport(global, Vec3::new(pos.x, height, pos.y))
                    .ok()?;
                Some((a.y - b.y).abs() / camera.logical_viewport_size()?.y)
            })
            .unwrap_or(1.0);
        if let Some(animation) = animation.as_deref_mut() {
            animation.quality = quality;
            animation.screen_height = screen_height;
            animation.visible = cameras.iter().next().is_none_or(|(camera, global)| {
                camera
                    .world_to_viewport(global, Vec3::new(pos.x, height * 0.5, pos.y))
                    .ok()
                    .zip(camera.logical_viewport_size())
                    .is_some_and(|(point, size)| {
                        point.x >= -height
                            && point.y >= -height
                            && point.x <= size.x + height
                            && point.y <= size.y + height
                    })
            });
        }
        if let Some(replica) = sources.replica.replica.as_ref()
            && let Some(unit) = unit
            && let Some(progress) = replica.site_progress(unit.0)
        {
            let fraction =
                replica.site_value(unit.0).unwrap_or(0) as f32 / progress.total.max(1) as f32;
            transform.translation.y -= -crate::animation::construction_offset(height, fraction);
        }
        let level = select_lod(
            model,
            screen_height,
            quality,
            instance.as_deref().map_or(0, |i| i.level),
        );
        let wanted = if level == 0 {
            &model.asset
        } else {
            &model.lods[level - 1].asset
        };
        // Keep the shown level until the requested level is ready.
        let asset = if state.templates.contains_key(wanted) {
            wanted
        } else if instance.is_some() {
            &instance.as_deref().unwrap().asset
        } else {
            continue;
        };
        let Some(primitives) = state.templates.get(asset).cloned() else {
            continue;
        };
        if instance
            .as_deref()
            .is_none_or(|i| i.asset != *asset || i.quality != quality)
        {
            let handle = state.handles[asset].clone();
            let Some(gltf) = sources.gltfs.get(&handle) else {
                continue;
            };
            let team = model
                .team_material
                .as_ref()
                .and_then(|name| gltf.named_materials.get(name.as_str()))
                .map(|h| h.id());
            let animated = model.animations.is_some() && !gltf.animations.is_empty();
            let mut rig_materials = HashMap::new();
            let mut children = Vec::new();
            let mut gpu_children = Vec::new();
            for primitive in primitives {
                let slot = if team == Some(primitive.material.id()) {
                    owner
                } else {
                    None
                };
                let key = (primitive.material.id(), slot, quality);
                let material = if let Some(handle) = state.materials.get(&key) {
                    handle.clone()
                } else {
                    let raw = sources
                        .materials
                        .get(&primitive.material)
                        .cloned()
                        .unwrap_or_default();
                    let mut material = bevy::pbr::gltf::standard_material_from_gltf_material(&raw);
                    if let Some(slot) = slot
                        .and_then(|slot| sources.palette.0.get(slot as usize))
                        .and_then(|h| material_assets.get(h))
                    {
                        material.base_color = slot.base_color;
                    }
                    for texture in [
                        &mut material.base_color_texture,
                        &mut material.normal_map_texture,
                        &mut material.metallic_roughness_texture,
                        &mut material.occlusion_texture,
                        &mut material.emissive_texture,
                    ] {
                        if let Some(h) = texture.as_mut() {
                            *h = limited_texture(h, quality, &mut images, &mut state.images);
                        }
                    }
                    let handle = model_materials.add(crate::fx::flash::ModelMaterial {
                        base: material,
                        extension: crate::fx::flash::FlashExtension::default(),
                    });
                    state.materials.insert(key, handle.clone());
                    handle
                };
                rig_materials.insert(primitive.material.id(), material.clone());
                if animated {
                    continue;
                }
                let mut local = primitive.transform;
                local.translation *= model.scale;
                local.scale *= model.scale;
                local.translation.y -= acquired
                    .0
                    .content
                    .shapes
                    .get(content)
                    .map_or(0.0, crate::render::mesh_lift);
                let child = commands
                    .spawn((
                        Mesh3d(primitive.mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        bevy::mesh::MeshTag::new(0),
                        local,
                        ChildOf(entity),
                    ))
                    .id();
                if let Some(unit) = unit {
                    commands
                        .entity(child)
                        .insert(crate::fx::flash::ModelFlash(unit.0));
                }
                children.push(child);
                gpu_children.push((child, primitive.mesh.id(), material.id()));
            }
            if !animated
                && let Some(name) = model.muzzle_node.as_deref()
                && let Some(mut local) = named_node_transform(gltf, name)
            {
                local.translation *= model.scale;
                local.scale *= model.scale;
                local.translation.y -= acquired
                    .0
                    .content
                    .shapes
                    .get(content)
                    .map_or(0., crate::render::mesh_lift);
                let anchor = commands
                    .spawn((local, ChildOf(entity), crate::fx::UnitMuzzle(id)))
                    .id();
                children.push(anchor);
            }
            if animated {
                if let Some((rig, gpu)) = crate::animation::spawn_rig(
                    &mut commands,
                    entity,
                    id,
                    unit.map(|unit| unit.0),
                    pos,
                    acquired
                        .0
                        .content
                        .shapes
                        .get(content)
                        .map_or(0., crate::render::mesh_lift),
                    model.scale,
                    gltf,
                    handle.id(),
                    &sources.nodes,
                    &sources.meshes,
                    &mut mesh_assets,
                    &sources.skins,
                    &sources.bindposes,
                    &sources.clips,
                    &mut graphs,
                    &mut library,
                    &rig_materials,
                    model.animations.as_ref().unwrap(),
                    model.muzzle_node.as_deref(),
                ) {
                    children.push(rig);
                    gpu_children = gpu;
                } else {
                    continue;
                }
            }
            let mut retired_children = Vec::new();
            if let Some(instance) = instance.as_deref() {
                retired_children.extend(instance.children.iter().copied());
                retired_children.extend(instance.retired_children.iter().copied());
            }
            state.next_gpu_ticket += 1;
            let ticket = state.next_gpu_ticket;
            if let Ok(mut bridge) = bridge.0.lock() {
                bridge.pending.insert(
                    ticket,
                    GpuRequest {
                        root: entity,
                        children: gpu_children,
                    },
                );
            }
            commands.entity(entity).insert(ModelInstance {
                asset: asset.clone(),
                level,
                quality,
                children,
                retired_children,
                ticket,
                ready: false,
            });
        }
        if unit.is_some() && owner.is_some() {
            let mobile = turts_mod_api::ContentId::parse(content)
                .ok()
                .and_then(|id| acquired.0.content.catalog.units.get(&id))
                .is_some_and(|u| u.movement.is_some());
            if mobile {
                let initial = transform.rotation.to_euler(EulerRot::YXZ).0;
                let (previous, yaw) = state.facing.entry(id).or_insert((pos, initial));
                let mut direction = pos - *previous;
                if let Some(target) = weapon_targets
                    .get(&id)
                    .copied()
                    .flatten()
                    .and_then(|target| shown_units.get(&target.0))
                {
                    direction = Vec2::new(target.pos.x, target.pos.y) - pos;
                }
                *yaw = turn_toward(*yaw, direction, sources.time.delta_secs());
                *previous = pos;
                let rotation = Quat::from_rotation_y(*yaw);
                if transform.rotation != rotation {
                    transform.rotation = rotation;
                }
            }
        }
    }
}

/// Only packages with models opt into new lighting. Older shape-only scenes stay unchanged.
pub fn quality_lighting(
    acquired: Option<Res<AcquiredPackages>>,
    settings: Res<GraphicsSettings>,
    mut lights: Query<(&mut DirectionalLight, &mut bevy::light::CascadeShadowConfig)>,
    mut configured: Local<bool>,
    mut shadow_map: ResMut<bevy::light::DirectionalLightShadowMap>,
) {
    if acquired.is_none_or(|a| a.0.presentation.models.is_empty()) {
        return;
    }
    shadow_map.size = if settings.quality == Quality::High {
        2048
    } else {
        1024
    };
    for (mut light, mut cascade) in &mut lights {
        light.shadow_maps_enabled = settings.quality != Quality::Low;
        if !*configured {
            *cascade = bevy::light::CascadeShadowConfigBuilder {
                num_cascades: 1,
                maximum_distance: 180.0,
                ..Default::default()
            }
            .build();
        }
    }
    *configured = true;
}

#[derive(Component)]
pub struct GroundSurface {
    pub base: Option<Handle<Image>>,
    pub normal: Option<Handle<Image>>,
}
pub fn sync_ground_quality(
    settings: Res<GraphicsSettings>,
    ground: Query<(&GroundSurface, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ModelState>,
) {
    if !settings.is_changed() {
        return;
    }
    for (source, handle) in &ground {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.base_color_texture = source
                .base
                .as_ref()
                .map(|h| limited_texture(h, settings.quality, &mut images, &mut state.images));
            material.normal_map_texture = source
                .normal
                .as_ref()
                .map(|h| limited_texture(h, settings.quality, &mut images, &mut state.images));
        }
    }
}

fn named_node_transform(gltf: &Gltf, name: &str) -> Option<Transform> {
    let source = gltf.source.as_ref()?;
    let node = source.nodes().find(|node| node.name() == Some(name))?;
    let mut parents = HashMap::new();
    let mut matrices = HashMap::new();
    for node in source.nodes() {
        matrices.insert(
            node.index(),
            Mat4::from_cols_array_2d(&node.transform().matrix()),
        );
        for child in node.children() {
            parents.insert(child.index(), node.index());
        }
    }
    let mut cursor = node.index();
    let mut matrix = matrices[&cursor];
    let mut depth = 0;
    while let Some(parent) = parents.get(&cursor) {
        depth += 1;
        if depth > 128 {
            return None;
        }
        matrix = matrices[parent] * matrix;
        cursor = *parent;
    }
    Some(Transform::from_matrix(matrix))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn md5_east_and_attack_facing_reach_target_within_half_second() {
        for target in [Vec2::X, Vec2::NEG_X, Vec2::NEG_Y] {
            let mut yaw = 0.;
            for _ in 0..30 {
                yaw = turn_toward(yaw, target, 1. / 60.);
            }
            let forward = Vec2::new(yaw.sin(), yaw.cos());
            assert!(
                forward.distance(target) < 0.0001,
                "MD5: direction {target:?}, yaw {yaw}"
            );
        }
    }
    #[test]
    fn md7_lod_responds_to_screen_height_and_has_hysteresis() {
        let model = turts_mod_api::ModelDef {
            asset: "test:full".into(),
            team_material: None,
            turret_node: None,
            muzzle_node: None,
            scale: 1.,
            animations: None,
            lods: vec![turts_mod_api::ModelLod {
                asset: "test:low".into(),
                below_screen_height: 0.08,
            }],
        };
        assert_eq!(select_lod(&model, 0.1, Quality::High, 1), 0, "MD7 zoom in");
        assert_eq!(
            select_lod(&model, 0.05, Quality::High, 0),
            1,
            "MD7 zoom out"
        );
        assert_eq!(select_lod(&model, 0.08, Quality::High, 0), 0);
        assert_eq!(select_lod(&model, 0.08, Quality::High, 1), 1);
    }
}

/// A render-world acknowledgement keeps the shape until the model can draw.
#[derive(Resource, Clone, Default)]
pub struct ModelGpuBridge(std::sync::Arc<std::sync::Mutex<ModelGpuRequests>>);
#[derive(Default)]
struct ModelGpuRequests {
    pending: HashMap<u64, GpuRequest>,
    ready: HashMap<u64, Entity>,
}
struct GpuRequest {
    root: Entity,
    children: Vec<(
        Entity,
        AssetId<Mesh>,
        AssetId<crate::fx::flash::ModelMaterial>,
    )>,
}

pub fn register_gpu_readiness(app: &mut App) {
    let bridge = ModelGpuBridge::default();
    app.insert_resource(bridge.clone());
    if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp) {
        render.insert_resource(bridge).add_systems(
            bevy::render::Render,
            acknowledge_models.in_set(bevy::render::RenderSystems::Cleanup),
        );
    }
}
fn acknowledge_models(
    bridge: Res<ModelGpuBridge>,
    meshes: Res<bevy::render::render_asset::RenderAssets<bevy::render::mesh::RenderMesh>>,
    materials: Res<
        bevy::render::erased_render_asset::ErasedRenderAssets<bevy::pbr::PreparedMaterial>,
    >,
    instances: Res<bevy::pbr::RenderMeshInstances>,
    pipelines: Res<bevy::render::render_resource::PipelineCache>,
) {
    if pipelines.waiting_pipelines().next().is_some() {
        return;
    }
    let Ok(mut bridge) = bridge.0.lock() else {
        return;
    };
    let ready: Vec<_> = bridge
        .pending
        .iter()
        .filter(|(_, request)| {
            request.children.iter().all(|(entity, mesh, material)| {
                meshes.get(*mesh).is_some()
                    && materials.get(*material).is_some()
                    && instances.mesh_asset_id(bevy::render::sync_world::MainEntity::from(*entity))
                        == Some(*mesh)
            })
        })
        .map(|(ticket, _)| *ticket)
        .collect();
    for ticket in ready {
        if let Some(request) = bridge.pending.remove(&ticket) {
            bridge.ready.insert(ticket, request.root);
        }
    }
}
