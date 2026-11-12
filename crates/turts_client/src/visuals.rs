//! World-space presentation for selection, status, and local command intent.
//!
//! Visual entities are retained by stable `(kind, unit id, ordinal)` identities. Their transforms
//! are written only when geometry or presentation state changes. The meshes are shared across all
//! units; owner colours come from the same palette material handles as the unit bodies.

mod blocked;
mod effects;
mod health;
mod hover;
mod markers;
mod materials;
mod order_markers;
mod placement;
mod queued;
mod rings;
mod session;
mod world_tooltip;

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::gizmos::prelude::{AppGizmoBuilder, GizmoConfig, GizmoConfigGroup, Gizmos};
use bevy::material::AlphaMode;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionPipeline, MaterialPlugin};
use bevy::platform::time::Instant;
use bevy::reflect::TypePath;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{
        AsBindGroup, CompareFunction, DepthStencilState, RenderPipelineDescriptor,
    },
    window::PrimaryWindow,
};
use blocked::{draw_blocked_marks, track_blocked};
pub use health::{HealthDisplay, HealthMode};
use health::{collect_health_bars, update_health_fills};
use hover::{draw_hover_ring, resolve_hover};
use markers::{
    marker_is_live, marker_kind, refresh_blocked_marks, reset_marker_session, retained_marker,
};
use materials::{
    OrderMarkerGizmoConfigGroup, RingOverlayMaterial, VisualMaterial,
    create_marker_overlay_material, create_neutral_ring_overlay_materials,
    create_ring_overlay_materials, create_shared_meshes, neutral_ring_overlay_index,
    order_marker_gizmo_config, parse_hex, unlit,
};
use turts_mod_api::ContentId;
use turts_net_protocol::{Activity, OrderTarget, Pos, UnitOrderRecord};

use crate::{
    commands::{OrderMarkerFeed, OrderMarkerIntent, SentOrderFragment},
    core_adapter::CoreRuntime,
    debug_feed::{MarkersShown, RingsShown, SelectionFeed},
    ingest::{CommandResult, CommandResults, ReceiptWitnesses, ReplicaState},
    join::AcquiredPackages,
    present::Presented,
    render::{SlotMaterials, world_to_scene},
};
use order_markers::{draw_order_markers, intake_markers};
use queued::draw_queued_targets;
use rings::draw_selection_rings;
use session::sync_session;
pub(crate) use world_tooltip::WorldTooltip;
use world_tooltip::update_world_tooltip;

#[derive(Resource, Default)]
struct VisualAssets {
    ring: Option<Handle<Mesh>>,
    foreign_ring: Option<Handle<Mesh>>,
    ring_overlay_materials: Vec<Handle<RingOverlayMaterial>>,
    neutral_ring_overlays: Vec<Handle<RingOverlayMaterial>>,
    ring_underlay_indices: Vec<usize>,
    bar: Option<Handle<Mesh>>,
    marker_overlay_material: Option<Handle<RingOverlayMaterial>>,
    blocked_material: Option<Handle<StandardMaterial>>,
    range_material: Option<Handle<StandardMaterial>>,
    bar_background: Option<Handle<StandardMaterial>>,
    bar_health: Option<Handle<StandardMaterial>>,
    range_enabled: bool,
    tooltip_background: Color,
    tooltip_colour: Color,
    tooltip_size: u16,
    tooltip_font: Option<String>,
    health_bar_size: Vec2,
}

#[derive(SystemParam)]
struct VisualInputs<'w> {
    presented: Res<'w, Presented>,
    state: Res<'w, ReplicaState>,
    receipts: Res<'w, ReceiptWitnesses>,
    runtime: Res<'w, CoreRuntime>,
    selected: Res<'w, SelectionFeed>,
    display: Res<'w, HealthDisplay>,
    marker_feed: ResMut<'w, OrderMarkerFeed>,
    results: Res<'w, CommandResults>,
    palette: Res<'w, SlotMaterials>,
    assets: Res<'w, VisualAssets>,
    widget_assets: Option<Res<'w, crate::widgets::assets::WidgetAssets>>,
    visuals: ResMut<'w, VisualState>,
    markers_shown: ResMut<'w, MarkersShown>,
    // One resource for each owner task of later visuals (L0-CORE): `placement::Feed`, `effects::Pool`.
    placement_feed: Res<'w, placement::Feed>,
    effects_pool: ResMut<'w, effects::Pool>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisualId {
    kind: VisualKind,
    unit: u32,
    ordinal: u8,
    generation: u64,
    identity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisualKind {
    Selected,
    RingUnderlay,
    Hover,
    Range,
    HealthBackground,
    Blocked,
    OrderMarker,
}

#[derive(Component)]
struct HealthFill(u32);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
struct RingMetadata {
    unit: u32,
    slot: u8,
    own: bool,
    radius: f32,
}

#[derive(Debug, Clone)]
struct RetainedMarker {
    id: u64,
    position: Vec2,
    kind: String,
    order: turts_net_protocol::OrderEnvelope,
    fragments: Vec<SentOrderFragment>,
    results: HashMap<u32, MarkerOrderResult>,
    seen_results: HashSet<u32>,
    started: Instant,
}

#[derive(Debug, Clone)]
struct MarkerOrderResult {
    tick: u32,
    accepted: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VisualSession {
    match_id: u64,
    connection_epoch: u64,
    local_slot: u8,
}

/// The visuals that one frame wants, by stable identity: mesh, material and transform.
type DesiredVisuals = HashMap<VisualId, (Handle<Mesh>, VisualMaterial, Transform)>;

#[derive(Resource, Default)]
struct VisualState {
    hover_unit: Option<u32>,
    hover_started: Option<Instant>,
    session: Option<VisualSession>,
    generation: u64,
    active: bool,
    blocked_since: HashMap<u32, Instant>,
    blocked_active: HashSet<u32>,
    markers: VecDeque<RetainedMarker>,
}

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::render::RenderPlugin>() {
            return;
        }
        app.insert_gizmo_config(OrderMarkerGizmoConfigGroup, order_marker_gizmo_config());
        app.add_plugins(MaterialPlugin::<RingOverlayMaterial>::default());
        placement::register(app);
        effects::register(app);
        app.init_resource::<VisualAssets>()
            .init_resource::<HealthDisplay>()
            .init_resource::<VisualState>()
            .add_systems(
                Update,
                (prepare_assets, draw_world_visuals)
                    .chain()
                    .after(crate::render::sync_units)
                    .after(crate::present::update_presented)
                    .after(crate::core_adapter::CoreAdapterSet),
            )
            .add_systems(
                Last,
                collect_visible_rings.before(crate::core_adapter::FrameCloseSet),
            );
    }
}

fn prepare_assets(
    mut assets: ResMut<VisualAssets>,
    acquired: Option<Res<AcquiredPackages>>,
    palette: Res<SlotMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ring_materials: ResMut<Assets<RingOverlayMaterial>>,
) {
    if assets.ring.is_none() {
        let Some(acquired) = acquired else { return };
        if palette.0.is_empty() {
            return;
        }
        let Some(ring_overlay_materials) =
            create_ring_overlay_materials(&palette.0, &materials, &mut ring_materials)
        else {
            return;
        };
        let neutral_ring_overlays = create_neutral_ring_overlay_materials(&mut ring_materials);
        let ring_underlay_indices = palette
            .0
            .iter()
            .map(|handle| {
                materials
                    .get(handle)
                    .map(|material| neutral_ring_overlay_index(material.base_color))
            })
            .collect::<Option<Vec<_>>>();
        let Some(ring_underlay_indices) = ring_underlay_indices else {
            return;
        };
        let world = acquired
            .0
            .presentation
            .interface
            .as_ref()
            .map(|interface| &interface.world);
        let range_color =
            world.map_or(Color::srgb(0.95, 0.8, 0.3), |style| parse_hex(&style.range));
        let bar_background = world.map_or(Color::srgb(0.06, 0.06, 0.06), |style| {
            parse_hex(&style.health_bar.background)
        });
        let bar_health = world.map_or(Color::srgb(0.2, 0.82, 0.24), |style| {
            parse_hex(&style.health_bar.health)
        });
        assets.range_enabled = world.is_none_or(|style| style.show_range);
        assets.tooltip_background = world.map_or(Color::srgb(0.02, 0.04, 0.06), |style| {
            parse_hex(&style.tooltip.background)
        });
        assets.tooltip_colour =
            world.map_or(Color::WHITE, |style| parse_hex(&style.tooltip.colour));
        assets.tooltip_size = world.map_or(14, |style| style.tooltip.size);
        assets.tooltip_font = world
            .and_then(|style| style.tooltip.font.clone())
            .map(|font| {
                if font.contains(':') {
                    font
                } else {
                    format!(
                        "{}:{font}",
                        acquired
                            .0
                            .presentation
                            .interface_package
                            .as_deref()
                            .unwrap_or_default()
                    )
                }
            });
        assets.health_bar_size = world.map_or(Vec2::new(48.0, 6.0), |style| {
            Vec2::new(
                f32::from(style.health_bar.width),
                f32::from(style.health_bar.height),
            )
        });
        create_shared_meshes(&mut assets, &mut meshes);
        assets.ring_overlay_materials = ring_overlay_materials;
        assets.neutral_ring_overlays = neutral_ring_overlays;
        assets.ring_underlay_indices = ring_underlay_indices;
        assets.marker_overlay_material = Some(create_marker_overlay_material(
            range_color,
            &mut ring_materials,
        ));
        assets.blocked_material = Some(materials.add(unlit(Color::srgb(0.9, 0.12, 0.08))));
        assets.range_material = Some(materials.add(unlit(range_color)));
        assets.bar_background = Some(materials.add(unlit(bar_background)));
        assets.bar_health = Some(materials.add(unlit(bar_health)));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_world_visuals(
    mut commands: Commands,
    inputs: VisualInputs,
    window: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut gizmos: Gizmos,
    mut marker_gizmos: Gizmos<OrderMarkerGizmoConfigGroup>,
    mut shown: Query<
        (Entity, &VisualId, &mut Transform, Option<&mut RingMetadata>),
        Without<HealthFill>,
    >,
    mut fills: Query<(Entity, &HealthFill, &mut Transform), Without<VisualId>>,
    mut labels: Query<(Entity, &WorldTooltip, &mut Node, &mut Text, &mut TextFont)>,
) {
    let VisualInputs {
        presented,
        state,
        receipts,
        runtime,
        selected,
        display,
        mut marker_feed,
        results,
        palette,
        assets,
        widget_assets,
        mut visuals,
        mut markers_shown,
        placement_feed,
        mut effects_pool,
    } = inputs;
    markers_shown.0.clear();
    sync_session(
        &state,
        &receipts,
        &mut visuals,
        &mut marker_feed,
        &mut commands,
        &mut fills,
    );
    visuals.active = false;
    let (Some(replica), Some(content), Some(welcome), Some(camera_pair)) = (
        state.replica.as_ref(),
        state.content.as_ref(),
        state.welcome,
        cameras.iter().next(),
    ) else {
        for (entity, _, _, _) in shown.iter_mut() {
            commands.entity(entity).despawn();
        }
        for (entity, _, _) in fills.iter_mut() {
            commands.entity(entity).despawn();
        }
        for (entity, _, _, _, _) in labels.iter_mut() {
            commands.entity(entity).despawn();
        }
        return;
    };
    visuals.active = true;
    let (camera, camera_transform) = camera_pair;
    let local_slot = welcome.slot;
    let selected_ids: HashSet<u32> = selected.0.iter().copied().collect();
    let presented_by_id: HashMap<u32, _> = presented
        .units
        .iter()
        .map(|unit| (unit.unit_id, unit))
        .collect();
    let now = Instant::now();

    let hovered = resolve_hover(
        &runtime,
        &window,
        camera,
        camera_transform,
        &presented,
        content,
        &mut visuals,
        now,
    );

    intake_markers(&mut marker_feed, &mut visuals, replica, &results, now);
    track_blocked(replica, &mut visuals, now);

    let mut desired = HashMap::<VisualId, (Handle<Mesh>, VisualMaterial, Transform)>::new();
    let mut ring_metadata = HashMap::new();
    draw_selection_rings(
        &mut desired,
        &mut ring_metadata,
        &selected,
        &presented_by_id,
        replica,
        content,
        &palette,
        &assets,
        local_slot,
        &visuals,
    );

    draw_hover_ring(
        &mut desired,
        hovered,
        &presented_by_id,
        replica,
        content,
        &assets,
        local_slot,
        &visuals,
    );

    let mut health_fractions = HashMap::new();
    collect_health_bars(
        replica,
        &presented_by_id,
        content,
        &selected_ids,
        &display,
        &assets,
        camera,
        camera_transform,
        &visuals,
        &mut desired,
        &mut health_fractions,
    );

    draw_order_markers(
        &mut desired,
        &mut markers_shown,
        &mut marker_gizmos,
        &assets,
        &visuals,
    );

    draw_blocked_marks(
        &mut desired,
        &mut markers_shown,
        &mut marker_gizmos,
        &presented_by_id,
        &assets,
        &visuals,
        now,
    );

    // Selected own units show queued point targets in queue order.
    draw_queued_targets(
        &mut gizmos,
        &selected,
        replica,
        &presented_by_id,
        local_slot,
    );

    // The concerns of the later tasks (L0-CORE): each draws into `desired`, after the last present
    // concern and before `update_health_fills`.
    placement::draw(&mut desired, &placement_feed);
    effects::draw(&mut desired, &mut effects_pool);

    update_health_fills(&mut fills, &health_fractions);
    reconcile_visual_entities(
        &mut commands,
        &mut shown,
        desired,
        &ring_metadata,
        &health_fractions,
        assets.bar.clone(),
        assets.bar_health.clone(),
    );
    update_world_tooltip(
        &mut commands,
        hovered,
        visuals.hover_started,
        now,
        camera,
        camera_transform,
        &presented_by_id,
        replica,
        &content.catalog,
        &assets,
        widget_assets.as_deref(),
        &mut labels,
    );
}

fn collect_visible_rings(
    mut shown: ResMut<RingsShown>,
    visuals: Res<VisualState>,
    rings: Query<(
        &RingMetadata,
        &Mesh3d,
        &ViewVisibility,
        &InheritedVisibility,
    )>,
) {
    shown.0.clear();
    if !visuals.active {
        return;
    }
    shown
        .0
        .extend(rings.iter().filter_map(|(ring, _mesh, view, inherited)| {
            (view.get() && inherited.get()).then_some(serde_json::json!({
                "unit": ring.unit,
                "slot": ring.slot,
                "own": ring.own,
                "radius": ring.radius,
            }))
        }));
}

struct HealthBarSync<'a> {
    healths: &'a [turts_net_protocol::HealthRecord],
    presented: &'a HashMap<u32, &'a crate::present::PresentedUnit>,
    catalog: &'a turts_mod_api::ContentCatalog,
    selected: &'a HashSet<u32>,
    mode: HealthMode,
    assets: &'a VisualAssets,
    camera: &'a Camera,
    camera_transform: &'a GlobalTransform,
    generation: u64,
}

fn reconcile_visual_entities(
    commands: &mut Commands,
    shown: &mut Query<
        (Entity, &VisualId, &mut Transform, Option<&mut RingMetadata>),
        Without<HealthFill>,
    >,
    mut desired: HashMap<VisualId, (Handle<Mesh>, VisualMaterial, Transform)>,
    ring_metadata: &HashMap<u32, RingMetadata>,
    health_fractions: &HashMap<u32, f32>,
    bar_mesh: Option<Handle<Mesh>>,
    health_material: Option<Handle<StandardMaterial>>,
) {
    let mut kept = HashSet::new();
    for (entity, id, mut transform, metadata) in shown.iter_mut() {
        if let Some((_, _, target)) = desired.remove(id) {
            kept.insert(*id);
            if *transform != target {
                *transform = target;
            }
            if id.kind == VisualKind::Selected
                && let (Some(mut metadata), Some(next)) = (metadata, ring_metadata.get(&id.unit))
                && *metadata != *next
            {
                *metadata = *next;
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
    for (id, (mesh, material, transform)) in desired {
        if kept.contains(&id) {
            continue;
        }
        let parent = if id.kind == VisualKind::Selected {
            let Some(metadata) = ring_metadata.get(&id.unit).copied() else {
                continue;
            };
            spawn_visual(commands, id, Some(metadata), mesh, material, transform)
        } else {
            spawn_visual(commands, id, None, mesh, material, transform)
        };
        if id.kind == VisualKind::HealthBackground
            && let (Some(bar_mesh), Some(health_material)) = (&bar_mesh, &health_material)
        {
            let fraction = health_fractions.get(&id.unit).copied().unwrap_or(1.0);
            commands.spawn((
                HealthFill(id.unit),
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(health_material.clone()),
                Transform {
                    translation: Vec3::new(-(1.0 - fraction) * 0.45, 0.0, 0.08),
                    scale: Vec3::new(fraction, 1.0, 1.0),
                    ..default()
                },
                ChildOf(parent),
            ));
        }
    }
}

fn spawn_visual(
    commands: &mut Commands,
    id: VisualId,
    ring_metadata: Option<RingMetadata>,
    mesh: Handle<Mesh>,
    material: VisualMaterial,
    transform: Transform,
) -> Entity {
    match material {
        VisualMaterial::Standard(material) => {
            let mut entity =
                commands.spawn((id, Mesh3d(mesh), MeshMaterial3d(material), transform));
            if let Some(metadata) = ring_metadata {
                entity.insert(metadata);
            }
            entity.id()
        }
        VisualMaterial::RingOverlay(material) => {
            let mut entity =
                commands.spawn((id, Mesh3d(mesh), MeshMaterial3d(material), transform));
            if let Some(metadata) = ring_metadata {
                entity.insert(metadata);
            }
            entity.id()
        }
    }
}

#[cfg(test)]
mod scene_tests;
#[cfg(test)]
mod tests;
