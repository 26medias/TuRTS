//! The hovered unit and its ring.

use super::*;
use crate::{content::ClientContent, present::PresentedUnit, replica::Replica};

// Both hover paths use only the same live contact permission as the rendered body.
fn body_hit(
    ray: Ray3d,
    position: Vec2,
    shape: &turts_mod_api::Shape,
    yaw: f32,
    support: Option<f32>,
) -> Option<f32> {
    turts_interaction::picking::shape_hit(
        ray.origin - Vec3::Y * support?,
        *ray.direction,
        position,
        shape,
        yaw,
    )
}
fn footprint_hit(ray: Ray3d, position: Vec2, radius: f32, support: Option<f32>) -> Option<f32> {
    let distance = ray.intersect_plane(Vec3::Y * support?, InfinitePlane3d::new(Dir3::Y))?;
    let point = ray.get_point(distance);
    let delta = Vec2::new(point.x, point.z) - position;
    (delta.length_squared() <= radius * radius).then_some(distance)
}
fn physical_hit_order(a: &(u32, f32, Vec2), b: &(u32, f32, Vec2)) -> std::cmp::Ordering {
    a.1.total_cmp(&b.1)
        .then_with(|| a.2.x.total_cmp(&b.2.x))
        .then_with(|| a.2.y.total_cmp(&b.2.y))
}

/// The unit under the pointer. It updates the hover state of `visuals`.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_hover(
    runtime: &CoreRuntime,
    window: &Query<&Window, With<PrimaryWindow>>,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    presented: &Presented,
    content: &ClientContent,
    replica: &Replica,
    model_state: Option<&crate::models::ModelState>,
    visuals: &mut VisualState,
    now: Instant,
) -> Option<u32> {
    let hovered = runtime
        .pointer_world_hover_allowed()
        .then(|| window.iter().next().and_then(Window::cursor_position))
        .flatten()
        .and_then(|cursor| {
            let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
            let body = presented
                .units
                .iter()
                .filter_map(|unit| {
                    let shape = content
                        .model_pick_shapes
                        .get(&unit.content_id)
                        .or_else(|| content.shapes.get(&unit.content_id))?;
                    let yaw = replica
                        .units
                        .get(&unit.unit_id)
                        .and_then(|unit| unit.facing)
                        .map_or(0.0, |facing| {
                            crate::render::facing_rotation(facing)
                                .to_euler(EulerRot::YXZ)
                                .0
                        });
                    let position = Vec2::new(unit.pos.x, unit.pos.y);
                    body_hit(
                        ray,
                        position,
                        shape,
                        model_state.and_then(|m| m.yaw(unit.unit_id)).unwrap_or(yaw),
                        crate::terrain::render::live_height(replica.terrain.as_ref(), unit.unit_id),
                    )
                    .map(|distance| (unit.unit_id, distance, position))
                })
                .min_by(physical_hit_order);
            if let Some((id, _, _)) = body {
                return Some(id);
            }
            presented
                .units
                .iter()
                .filter_map(|unit| {
                    let def = ContentId::parse(&unit.content_id)
                        .ok()
                        .and_then(|id| content.catalog.units.get(&id))?;
                    let turts_mod_api::Footprint::Circle { radius } = &def.footprint;
                    let position = Vec2::new(unit.pos.x, unit.pos.y);
                    footprint_hit(
                        ray,
                        position,
                        *radius,
                        crate::terrain::render::live_height(replica.terrain.as_ref(), unit.unit_id),
                    )
                    .map(|distance| (unit.unit_id, distance, position))
                })
                .min_by(physical_hit_order)
                .map(|(id, _, _)| id)
        });
    if visuals.hover_unit != hovered {
        visuals.hover_unit = hovered;
        visuals.hover_started = hovered.map(|_| now);
    }
    hovered
}

/// The ring of the hovered unit, with the neutral underlay for a foreign unit.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_hover_ring(
    desired: &mut DesiredVisuals,
    hovered: Option<u32>,
    presented_by_id: &HashMap<u32, &PresentedUnit>,
    replica: &Replica,
    content: &ClientContent,
    assets: &VisualAssets,
    local_slot: u8,
    visuals: &VisualState,
) {
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
                            translation: world_to_scene(shown_unit.pos)
                                + Vec3::Y
                                    * crate::terrain::render::live_height(
                                        replica.terrain.as_ref(),
                                        shown_unit.unit_id,
                                    )
                                    .unwrap_or(0.)
                                + Vec3::Y * 0.028,
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
                        translation: world_to_scene(shown_unit.pos)
                            + Vec3::Y
                                * crate::terrain::render::live_height(
                                    replica.terrain.as_ref(),
                                    shown_unit.unit_id,
                                )
                                .unwrap_or(0.)
                            + Vec3::Y * 0.032,
                        scale: Vec3::splat(radius * 0.96),
                        ..default()
                    },
                ),
            );
        }
    }
}

#[cfg(test)]
#[path = "hover_ground_tests.rs"]
mod hover_ground_tests;
