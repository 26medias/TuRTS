//! World-space presentation for selection, status, and local command intent.
//!
//! Visual entities are retained by stable `(kind, unit id, ordinal)` identities. Their transforms
//! are written only when geometry or presentation state changes. The meshes are shared across all
//! units; owner colours come from the same palette material handles as the unit bodies.

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

type RingOverlayMaterial = ExtendedMaterial<StandardMaterial, RingOverlayExtension>;

#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
struct OrderMarkerGizmoConfigGroup;

fn order_marker_gizmo_config() -> GizmoConfig {
    GizmoConfig {
        depth_bias: -1.0,
        ..default()
    }
}

/// Keeps footprint rings visible over larger unit silhouettes without changing their ground size.
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
struct RingOverlayExtension {}

impl MaterialExtension for RingOverlayExtension {
    fn alpha_mode() -> Option<AlphaMode> {
        Some(AlphaMode::Blend)
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let depth = descriptor
            .depth_stencil
            .as_mut()
            .expect("3D ring overlay pipeline has depth state");
        configure_ring_overlay_depth(depth);
        Ok(())
    }
}

fn configure_ring_overlay_depth(depth: &mut DepthStencilState) {
    depth.depth_compare = Some(CompareFunction::Always);
    depth.depth_write_enabled = Some(false);
}

fn create_ring_overlay_materials(
    palette: &[Handle<StandardMaterial>],
    materials: &Assets<StandardMaterial>,
    overlays: &mut Assets<RingOverlayMaterial>,
) -> Option<Vec<Handle<RingOverlayMaterial>>> {
    let bases = palette
        .iter()
        .map(|handle| materials.get(handle).cloned())
        .collect::<Option<Vec<_>>>()?;
    Some(
        bases
            .into_iter()
            .map(|slot_material| {
                overlays.add(ExtendedMaterial {
                    base: StandardMaterial {
                        alpha_mode: AlphaMode::Blend,
                        ..slot_material
                    },
                    extension: RingOverlayExtension {},
                })
            })
            .collect(),
    )
}

fn create_neutral_ring_overlay_materials(
    overlays: &mut Assets<RingOverlayMaterial>,
) -> Vec<Handle<RingOverlayMaterial>> {
    [Color::srgb(0.96, 0.96, 0.96), Color::BLACK]
        .into_iter()
        .map(|base_color| {
            overlays.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color,
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                },
                extension: RingOverlayExtension {},
            })
        })
        .collect()
}

fn create_marker_overlay_material(
    colour: Color,
    overlays: &mut Assets<RingOverlayMaterial>,
) -> Handle<RingOverlayMaterial> {
    overlays.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: colour,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        },
        extension: RingOverlayExtension {},
    })
}

fn neutral_ring_overlay_index(slot_colour: Color) -> usize {
    let colour = slot_colour.to_srgba();
    let luminance = 0.2126 * colour.red + 0.7152 * colour.green + 0.0722 * colour.blue;
    usize::from(luminance >= 0.82)
}

#[derive(Clone)]
enum VisualMaterial {
    Standard(Handle<StandardMaterial>),
    RingOverlay(Handle<RingOverlayMaterial>),
}

impl From<Handle<StandardMaterial>> for VisualMaterial {
    fn from(material: Handle<StandardMaterial>) -> Self {
        Self::Standard(material)
    }
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

#[derive(Component)]
pub(crate) struct WorldTooltip(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthMode {
    Selected,
    DamagedOrSelected,
    Always,
}

/// Player-facing health-bar visibility. Task16 settings can write this resource directly.
#[derive(Resource, Debug, Clone, Copy)]
pub struct HealthDisplay(pub HealthMode);

impl Default for HealthDisplay {
    fn default() -> Self {
        Self(HealthMode::DamagedOrSelected)
    }
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

fn retained_marker(
    intent: OrderMarkerIntent,
    replica: &crate::replica::Replica,
    now: Instant,
) -> Option<RetainedMarker> {
    let position = match &intent.order.target {
        OrderTarget::Point(Pos { x, y }) => Vec2::new(*x, *y),
        OrderTarget::Unit(unit) => replica
            .units
            .get(&unit.0)?
            .samples
            .back()
            .map(|(_, pos)| Vec2::new(pos.x, pos.y))?,
        OrderTarget::Node(node) => {
            let pos = replica.nodes.get(node)?.pos;
            Vec2::new(pos.x, pos.y)
        }
        OrderTarget::None | OrderTarget::Job(_) | OrderTarget::Content(_) => return None,
    };
    Some(RetainedMarker {
        id: intent.id,
        position,
        kind: intent.kind,
        order: intent.order,
        fragments: Vec::new(),
        results: HashMap::new(),
        seen_results: HashSet::new(),
        started: now,
    })
}

fn marker_is_live(
    marker: &RetainedMarker,
    replica: &crate::replica::Replica,
    replica_tick: u32,
    now: Instant,
) -> bool {
    marker_is_live_for_orders(marker, &replica.unit_orders, replica_tick, now)
}

fn marker_is_live_for_orders(
    marker: &RetainedMarker,
    unit_orders: &HashMap<u32, UnitOrderRecord>,
    replica_tick: u32,
    now: Instant,
) -> bool {
    if now.duration_since(marker.started).as_secs_f32() >= 60.0 {
        return false;
    }
    if marker.fragments.is_empty()
        || marker
            .fragments
            .iter()
            .any(|fragment| !marker.results.contains_key(&fragment.seq))
    {
        return true;
    }

    let accepted: Vec<_> = marker
        .fragments
        .iter()
        .flat_map(|fragment| {
            marker.results[&fragment.seq]
                .accepted
                .iter()
                .map(move |unit| (*unit, marker.results[&fragment.seq].tick))
        })
        .collect();
    if accepted.is_empty() {
        return false;
    }
    !accepted.into_iter().all(|(unit, result_tick)| {
        replica_tick >= result_tick
            && unit_orders
                .get(&unit)
                .is_some_and(|record| matches!(record.activity, Activity::Idle | Activity::Blocked))
    })
}

fn refresh_blocked_marks(
    blocked_now: HashSet<u32>,
    blocked_active: &mut HashSet<u32>,
    blocked_since: &mut HashMap<u32, Instant>,
    now: Instant,
) {
    for unit in &blocked_now {
        if !blocked_active.contains(unit) {
            blocked_since.insert(*unit, now);
        }
    }
    blocked_since.retain(|unit, start| {
        blocked_now.contains(unit) || now.duration_since(*start).as_secs_f32() < 2.0
    });
    *blocked_active = blocked_now;
}

fn health_bar_visible(mode: HealthMode, selected: bool, current: u64, max: u64) -> bool {
    match mode {
        HealthMode::Selected => selected,
        HealthMode::DamagedOrSelected => selected || current < max,
        HealthMode::Always => true,
    }
}

fn health_fraction(current: u64, max: u64) -> Option<f32> {
    (max > 0).then(|| (current as f32 / max as f32).clamp(0.0, 1.0))
}

fn update_health_fill_transform(transform: &mut Mut<'_, Transform>, fraction: f32) -> bool {
    // Read through `Deref` first; the first mutable field access below marks the Bevy component
    // changed, so it must only happen after the geometry comparison succeeds.
    if (transform.scale.x - fraction).abs() <= f32::EPSILON {
        return false;
    }
    transform.scale.x = fraction;
    transform.translation.x = -(1.0 - fraction) * 0.45;
    true
}

fn reset_marker_session(
    visuals: &mut VisualState,
    marker_feed: &mut OrderMarkerFeed,
    session: Option<VisualSession>,
) -> bool {
    if visuals.session != session {
        visuals.markers.clear();
        visuals.hover_unit = None;
        visuals.hover_started = None;
        visuals.blocked_since.clear();
        visuals.blocked_active.clear();
        if session.is_none() {
            marker_feed.intents.clear();
            marker_feed.sent.clear();
        } else {
            // CoreAdapter clears intents at the beginning of every frame, before
            // any early return. Anything left here therefore belongs to the new
            // session's current input frame, including a click before its ACK.
            let current_ids: HashSet<_> =
                marker_feed.intents.iter().map(|intent| intent.id).collect();
            marker_feed
                .sent
                .retain(|fragment| current_ids.contains(&fragment.intent_id));
        }
        visuals.generation = visuals.generation.wrapping_add(1);
        visuals.session = session;
        visuals.active = false;
        true
    } else {
        false
    }
}

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
    } = inputs;
    markers_shown.0.clear();
    let visual_session = state
        .welcome
        .map(|welcome| VisualSession {
            match_id: welcome.match_id,
            connection_epoch: receipts.next_epoch,
            local_slot: welcome.slot,
        })
        .filter(|_| state.replica.is_some() && state.content.is_some());
    let session_changed = reset_marker_session(&mut visuals, &mut marker_feed, visual_session);
    if session_changed {
        for (entity, _, _) in fills.iter_mut() {
            commands.entity(entity).despawn();
        }
    }
    if visual_session.is_none() {
        marker_feed.intents.clear();
        marker_feed.sent.clear();
    }
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

    let hovered = runtime
        .pointer_world_hover_allowed()
        .then(|| window.iter().next().and_then(Window::cursor_position))
        .flatten()
        .and_then(|cursor| {
            let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
            let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Dir3::Y))?;
            let point = ray.get_point(distance);
            presented
                .units
                .iter()
                .filter_map(|unit| {
                    let def = ContentId::parse(&unit.content_id)
                        .ok()
                        .and_then(|id| content.catalog.units.get(&id))?;
                    let turts_mod_api::Footprint::Circle { radius } = &def.footprint;
                    let dx = point.x - unit.pos.x;
                    let dz = point.z - unit.pos.y;
                    (dx * dx + dz * dz <= radius * radius).then_some(unit.unit_id)
                })
                .min()
        });
    if visuals.hover_unit != hovered {
        visuals.hover_unit = hovered;
        visuals.hover_started = hovered.map(|_| now);
    }

    for intent in marker_feed.intents.drain(..) {
        if let Some(marker) = retained_marker(intent, replica, now) {
            visuals.markers.push_back(marker);
        }
    }
    for fragment in marker_feed.sent.drain(..) {
        if let Some(marker) = visuals
            .markers
            .iter_mut()
            .find(|marker| marker.id == fragment.intent_id)
        {
            marker.fragments.push(fragment);
        }
    }
    for marker in &mut visuals.markers {
        for fragment in &marker.fragments {
            if marker.seen_results.contains(&fragment.seq) {
                continue;
            }
            if let Some(result) = results.0.iter().find_map(|result| match result {
                CommandResult::Order {
                    seq,
                    tick,
                    accepted,
                    ..
                } if *seq == fragment.seq => Some(MarkerOrderResult {
                    tick: *tick,
                    accepted: accepted.iter().map(|unit| unit.0).collect(),
                }),
                _ => None,
            }) {
                marker.seen_results.insert(fragment.seq);
                marker.results.insert(fragment.seq, result);
            }
        }
    }
    let replica_tick = replica.last_applied_tick.unwrap_or(0);
    visuals
        .markers
        .retain(|marker| marker_is_live(marker, replica, replica_tick, now));

    let blocked_now = replica
        .unit_orders
        .iter()
        .filter_map(|(&unit_id, order)| (order.activity == Activity::Blocked).then_some(unit_id))
        .collect();
    let visuals_state = &mut *visuals;
    refresh_blocked_marks(
        blocked_now,
        &mut visuals_state.blocked_active,
        &mut visuals_state.blocked_since,
        now,
    );

    let mut desired = HashMap::<VisualId, (Handle<Mesh>, VisualMaterial, Transform)>::new();
    let mut ring_metadata = HashMap::new();
    for unit_id in &selected.0 {
        let Some(shown_unit) = presented_by_id.get(unit_id) else {
            continue;
        };
        let Some(unit) = replica.units.get(unit_id) else {
            continue;
        };
        let Some(def) = ContentId::parse(&shown_unit.content_id)
            .ok()
            .and_then(|id| content.catalog.units.get(&id))
        else {
            continue;
        };
        let turts_mod_api::Footprint::Circle { radius } = &def.footprint;
        let slot = usize::from(unit.owner_slot).min(palette.0.len().saturating_sub(1));
        let Some(body_material) = palette.0.get(slot).cloned() else {
            continue;
        };
        let Some(material) = assets.ring_overlay_materials.get(slot).cloned() else {
            continue;
        };
        let own = unit.owner_slot == local_slot;
        let mesh = if own {
            assets.ring.clone()
        } else {
            assets.foreign_ring.clone()
        };
        let Some(mesh) = mesh else { continue };
        let position = world_to_scene(shown_unit.pos) + Vec3::Y * 0.025;
        let id = VisualId {
            kind: VisualKind::Selected,
            unit: *unit_id,
            ordinal: 0,
            generation: visuals.generation,
            identity: 0,
        };
        if !own
            && let (Some(underlay), Some(neutral_index)) =
                (assets.ring.clone(), assets.ring_underlay_indices.get(slot))
            && let Some(neutral) = assets.neutral_ring_overlays.get(*neutral_index).cloned()
        {
            desired.insert(
                VisualId {
                    kind: VisualKind::RingUnderlay,
                    unit: *unit_id,
                    ordinal: 0,
                    generation: visuals.generation,
                    identity: 0,
                },
                (
                    underlay,
                    VisualMaterial::RingOverlay(neutral),
                    Transform {
                        translation: world_to_scene(shown_unit.pos) + Vec3::Y * 0.021,
                        scale: Vec3::splat(*radius),
                        ..default()
                    },
                ),
            );
        }
        desired.insert(
            id,
            (
                mesh,
                VisualMaterial::RingOverlay(material),
                Transform {
                    translation: position,
                    scale: Vec3::splat(*radius),
                    ..default()
                },
            ),
        );
        ring_metadata.insert(
            *unit_id,
            RingMetadata {
                unit: *unit_id,
                slot: unit.owner_slot,
                own,
                radius: *radius,
            },
        );

        if assets.range_enabled
            && let Some(weapon) = def.weapon.as_ref()
        {
            let range_id = VisualId {
                kind: VisualKind::Range,
                unit: *unit_id,
                ordinal: 0,
                generation: visuals.generation,
                identity: 0,
            };
            let range_position = world_to_scene(shown_unit.pos) + Vec3::Y * 0.018;
            if let Some(range_mesh) = assets.ring.clone() {
                desired.insert(
                    range_id,
                    (
                        range_mesh,
                        assets
                            .range_material
                            .clone()
                            .unwrap_or(body_material)
                            .into(),
                        Transform {
                            translation: range_position,
                            scale: Vec3::splat(weapon.range),
                            ..default()
                        },
                    ),
                );
            }
        }
    }

    if let Some(unit_id) = hovered
        && let (Some(shown_unit), Some(unit)) =
            (presented_by_id.get(&unit_id), replica.units.get(&unit_id))
        && let Some(material) = assets
            .ring_overlay_materials
            .get(usize::from(unit.owner_slot))
            .cloned()
    {
        let mesh = if unit.owner_slot == local_slot {
            assets.ring.clone()
        } else {
            assets.foreign_ring.clone()
        };
        if let Some(mesh) = mesh {
            let radius = ContentId::parse(&shown_unit.content_id)
                .ok()
                .and_then(|id| content.catalog.units.get(&id))
                .map(|unit| {
                    let turts_mod_api::Footprint::Circle { radius } = &unit.footprint;
                    *radius
                })
                .unwrap_or(0.5);
            let slot = usize::from(unit.owner_slot);
            if unit.owner_slot != local_slot
                && let (Some(underlay), Some(neutral_index)) =
                    (assets.ring.clone(), assets.ring_underlay_indices.get(slot))
                && let Some(neutral) = assets.neutral_ring_overlays.get(*neutral_index).cloned()
            {
                desired.insert(
                    VisualId {
                        kind: VisualKind::RingUnderlay,
                        unit: unit_id,
                        ordinal: 1,
                        generation: visuals.generation,
                        identity: 0,
                    },
                    (
                        underlay,
                        VisualMaterial::RingOverlay(neutral),
                        Transform {
                            translation: world_to_scene(shown_unit.pos) + Vec3::Y * 0.028,
                            scale: Vec3::splat(radius * 0.96),
                            ..default()
                        },
                    ),
                );
            }
            desired.insert(
                VisualId {
                    kind: VisualKind::Hover,
                    unit: unit_id,
                    ordinal: 0,
                    generation: visuals.generation,
                    identity: 0,
                },
                (
                    mesh,
                    VisualMaterial::RingOverlay(material),
                    Transform {
                        translation: world_to_scene(shown_unit.pos) + Vec3::Y * 0.032,
                        scale: Vec3::splat(radius * 0.96),
                        ..default()
                    },
                ),
            );
        }
    }

    let mut health_fractions = HashMap::new();
    sync_health_bars(
        HealthBarSync {
            healths: &replica.healths,
            presented: &presented_by_id,
            catalog: &content.catalog,
            selected: &selected_ids,
            mode: display.0,
            assets: &assets,
            camera,
            camera_transform,
            generation: visuals.generation,
        },
        &mut desired,
        &mut health_fractions,
    );

    for marker in &visuals.markers {
        let material = assets.marker_overlay_material.clone();
        if let Some(material) = material {
            let kind = marker_kind(&marker.kind);
            let position = Vec3::new(marker.position.x, 0.035, marker.position.y);
            let id = VisualId {
                kind: VisualKind::OrderMarker,
                unit: 0,
                ordinal: 0,
                generation: visuals.generation,
                identity: marker.id,
            };
            match kind {
                "move" => {
                    if let Some(mesh) = assets.ring.clone() {
                        desired.insert(
                            id,
                            (
                                mesh,
                                VisualMaterial::RingOverlay(material.clone()),
                                Transform {
                                    translation: position,
                                    scale: Vec3::splat(0.38),
                                    ..default()
                                },
                            ),
                        );
                    }
                }
                "attack" => {
                    marker_gizmos.line(
                        position + Vec3::new(-0.3, 0.04, -0.3),
                        position + Vec3::new(0.3, 0.04, 0.3),
                        Color::srgb(0.96, 0.88, 0.2),
                    );
                    marker_gizmos.line(
                        position + Vec3::new(-0.3, 0.04, 0.3),
                        position + Vec3::new(0.3, 0.04, -0.3),
                        Color::srgb(0.96, 0.88, 0.2),
                    );
                }
                "gather" => {
                    if let Some(mesh) = assets.bar.clone() {
                        desired.insert(
                            id,
                            (
                                mesh,
                                VisualMaterial::RingOverlay(material.clone()),
                                Transform {
                                    translation: position,
                                    scale: Vec3::splat(0.16),
                                    ..default()
                                },
                            ),
                        );
                    }
                }
                _ => {}
            }
            markers_shown.0.push(serde_json::json!({
                "kind": kind,
                "intent_id": marker.id,
                "order": &marker.order.order,
                "units": marker.order.units.iter().map(|unit| unit.0).collect::<Vec<_>>(),
                "x": marker.position.x,
                "y": marker.position.y
            }));
        }
    }

    for (&unit_id, start) in &visuals.blocked_since {
        if now.duration_since(*start).as_secs_f32() >= 2.0 {
            continue;
        }
        if let Some(unit) = presented_by_id.get(&unit_id)
            && let Some(material) = assets.blocked_material.clone()
            && let Some(mesh) = assets.bar.clone()
        {
            let id = VisualId {
                kind: VisualKind::Blocked,
                unit: unit_id,
                ordinal: 0,
                generation: visuals.generation,
                identity: 0,
            };
            let point = world_to_scene(unit.pos) + Vec3::Y * 1.5;
            desired.insert(
                id,
                (
                    mesh,
                    material.clone().into(),
                    Transform {
                        translation: point,
                        scale: Vec3::new(0.7, 0.08, 0.08),
                        ..default()
                    },
                ),
            );
            marker_gizmos.line(
                point + Vec3::new(-0.3, 0.07, -0.3),
                point + Vec3::new(0.3, 0.07, 0.3),
                Color::srgb(0.9, 0.12, 0.08),
            );
            marker_gizmos.line(
                point + Vec3::new(-0.3, 0.07, 0.3),
                point + Vec3::new(0.3, 0.07, -0.3),
                Color::srgb(0.9, 0.12, 0.08),
            );
            markers_shown
                .0
                .push(serde_json::json!({"kind":"blocked","unit":unit_id}));
        }
    }

    // Selected own units show queued point targets in queue order.
    for unit_id in &selected.0 {
        let Some(record) = replica.unit_orders.get(unit_id) else {
            continue;
        };
        let Some(unit) = presented_by_id.get(unit_id) else {
            continue;
        };
        if replica
            .units
            .get(unit_id)
            .is_none_or(|unit| unit.owner_slot != local_slot)
        {
            continue;
        }
        let mut previous = Vec3::new(unit.pos.x, 0.04, unit.pos.y);
        for order in record.current.iter().chain(record.queued.iter()) {
            if let OrderTarget::Point(Pos { x, y }) = &order.target {
                let next = Vec3::new(*x, 0.04, *y);
                gizmos.line(previous, next, Color::srgb(0.9, 0.9, 0.85));
                previous = next;
            }
        }
    }

    for (_, fill, mut transform) in &mut fills {
        if let Some(fraction) = health_fractions.get(&fill.0) {
            update_health_fill_transform(&mut transform, *fraction);
        }
    }
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

fn sync_health_bars(
    input: HealthBarSync<'_>,
    desired: &mut HashMap<VisualId, (Handle<Mesh>, VisualMaterial, Transform)>,
    fractions: &mut HashMap<u32, f32>,
) {
    let HealthBarSync {
        healths,
        presented,
        catalog,
        selected,
        mode,
        assets,
        camera,
        camera_transform,
        generation,
    } = input;
    let (Some(mesh), Some(background), Some(fill)) =
        (&assets.bar, &assets.bar_background, &assets.bar_health)
    else {
        return;
    };
    for health in healths {
        let Some(unit) = presented.get(&health.unit.0) else {
            continue;
        };
        let Some(def) = ContentId::parse(&unit.content_id)
            .ok()
            .and_then(|id| catalog.units.get(&id))
        else {
            continue;
        };
        let Some(max) = def.health.map(|health| health.max).filter(|max| *max > 0) else {
            continue;
        };
        let is_selected = selected.contains(&health.unit.0);
        if !health_bar_visible(mode, is_selected, health.current, max) {
            continue;
        }
        let position = world_to_scene(unit.pos) + Vec3::Y * 2.2;
        let size =
            world_units_for_pixels(camera, camera_transform, position, assets.health_bar_size);
        let id = VisualId {
            kind: VisualKind::HealthBackground,
            unit: health.unit.0,
            ordinal: 0,
            generation,
            identity: 0,
        };
        desired.insert(
            id,
            (
                mesh.clone(),
                background.clone().into(),
                Transform {
                    translation: position,
                    scale: Vec3::new(size.x, size.y, 0.1),
                    ..default()
                },
            ),
        );
        let Some(fraction) = health_fraction(health.current, max) else {
            continue;
        };
        let _ = fill;
        fractions.insert(health.unit.0, fraction);
    }
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

#[allow(clippy::too_many_arguments)]
fn update_world_tooltip(
    commands: &mut Commands,
    hovered: Option<u32>,
    started: Option<Instant>,
    now: Instant,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    presented: &HashMap<u32, &crate::present::PresentedUnit>,
    replica: &crate::replica::Replica,
    catalog: &turts_mod_api::ContentCatalog,
    assets: &VisualAssets,
    widget_assets: Option<&crate::widgets::assets::WidgetAssets>,
    labels: &mut Query<(Entity, &WorldTooltip, &mut Node, &mut Text, &mut TextFont)>,
) {
    let text = hovered
        .filter(|_| started.is_some_and(|time| now.duration_since(time).as_secs_f32() >= 0.5))
        .and_then(|id| {
            let shown = presented.get(&id)?;
            let unit = replica.units.get(&id)?;
            let name = ContentId::parse(&shown.content_id)
                .ok()
                .and_then(|content| catalog.units.get(&content))
                .map(|def| def.name.clone())?;
            let point = camera
                .world_to_viewport(camera_transform, world_to_scene(shown.pos) + Vec3::Y * 2.0)
                .ok()?;
            Some((id, tooltip_text(&name, unit.owner_slot), point))
        });
    let font = assets
        .tooltip_font
        .as_deref()
        .and_then(|id| widget_assets.and_then(|assets| assets.font(id)))
        .unwrap_or_default();
    let mut matching_tooltip_exists = false;
    for (entity, tooltip, mut node, mut label, mut font_style) in labels.iter_mut() {
        if let Some((id, text, point)) = &text
            && tooltip.0 == *id
        {
            matching_tooltip_exists = true;
            *label = Text::new(text.clone());
            node.left = px(point.x + 12.0);
            node.top = px(point.y - 12.0);
            if font_style.font != FontSource::Handle(font.clone()) {
                font_style.font = FontSource::Handle(font.clone());
            }
            let size = f32::from(assets.tooltip_size);
            if !matches!(font_style.font_size, FontSize::Px(current) if current == size) {
                font_style.font_size = FontSize::Px(size);
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let Some((id, text, point)) = text
        && !matching_tooltip_exists
    {
        commands.spawn((
            WorldTooltip(id),
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(f32::from(assets.tooltip_size)),
                font: FontSource::Handle(font),
                ..default()
            },
            TextColor(assets.tooltip_colour),
            BackgroundColor(assets.tooltip_background.with_alpha(0.92)),
            Node {
                position_type: PositionType::Absolute,
                left: px(point.x + 12.0),
                top: px(point.y - 12.0),
                padding: UiRect::axes(px(6), px(3)),
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
        ));
    }
}

fn tooltip_text(name: &str, owner_slot: u8) -> String {
    format!("{name} · Slot {}", owner_slot + 1)
}

fn marker_kind(kind: &str) -> &str {
    match kind {
        "attack" => "attack",
        "gather" => "gather",
        _ => "move",
    }
}

fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    }
}

fn parse_hex(value: &str) -> Color {
    let raw = value.strip_prefix('#').unwrap_or(value);
    if raw.len() != 6 {
        return Color::WHITE;
    }
    let Ok(red) = u8::from_str_radix(&raw[0..2], 16) else {
        return Color::WHITE;
    };
    let Ok(green) = u8::from_str_radix(&raw[2..4], 16) else {
        return Color::WHITE;
    };
    let Ok(blue) = u8::from_str_radix(&raw[4..6], 16) else {
        return Color::WHITE;
    };
    Color::srgb_u8(red, green, blue)
}

fn world_units_for_pixels(
    camera: &Camera,
    transform: &GlobalTransform,
    point: Vec3,
    size: Vec2,
) -> Vec2 {
    let Some(center) = camera.world_to_viewport(transform, point).ok() else {
        return Vec2::new(0.9, 0.09);
    };
    let Some(x_step) = camera
        .world_to_viewport(transform, point + Vec3::X * 0.5)
        .ok()
    else {
        return Vec2::new(0.9, 0.09);
    };
    let Some(y_step) = camera
        .world_to_viewport(transform, point + Vec3::Y * 0.5)
        .ok()
    else {
        return Vec2::new(0.9, 0.09);
    };
    let px_per_x = center.distance(x_step) / 0.5;
    let px_per_y = center.distance(y_step) / 0.5;
    if px_per_x <= 0.0 || px_per_y <= 0.0 || !px_per_x.is_finite() || !px_per_y.is_finite() {
        return Vec2::new(0.9, 0.09);
    }
    Vec2::new(size.x / px_per_x, size.y / px_per_y)
}

fn annulus_mesh(gaps: bool) -> Mesh {
    let segments = 192_u32;
    let mut positions = Vec::with_capacity((segments as usize + 1) * 2);
    let mut normals = Vec::with_capacity((segments as usize + 1) * 2);
    let mut uvs = Vec::with_capacity((segments as usize + 1) * 2);
    let mut indices = Vec::new();
    for index in 0..=segments {
        let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
        let inner = 0.88;
        let outer = 1.0;
        for radius in [inner, outer] {
            positions.push([radius * angle.cos(), 0.0, radius * angle.sin()]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([index as f32 / segments as f32, radius]);
        }
    }
    for index in 0..segments {
        let angle = (index as f32 + 0.5) / segments as f32 * std::f32::consts::TAU;
        let in_gap = gaps
            && (0..4).any(|gap| {
                let center = gap as f32 * std::f32::consts::FRAC_PI_2;
                let delta = (angle - center + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                delta.abs() < 0.09
            });
        if in_gap {
            continue;
        }
        let first_vertex = index * 2;
        indices.extend_from_slice(&[
            first_vertex,
            first_vertex + 2,
            first_vertex + 1,
            first_vertex + 1,
            first_vertex + 2,
            first_vertex + 3,
        ]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn create_shared_meshes(assets: &mut VisualAssets, meshes: &mut Assets<Mesh>) {
    assets.ring = Some(meshes.add(annulus_mesh(false)));
    assets.foreign_ring = Some(meshes.add(annulus_mesh(true)));
    assets.bar = Some(meshes.add(Cuboid::new(1.0, 1.0, 0.12)));
}

#[cfg(test)]
mod tests;
