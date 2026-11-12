//! The hovered unit and its ring.

use super::*;
use crate::{content::ClientContent, present::PresentedUnit, replica::Replica};

/// The unit under the pointer. It updates the hover state of `visuals`.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_hover(
    runtime: &CoreRuntime,
    window: &Query<&Window, With<PrimaryWindow>>,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    presented: &Presented,
    content: &ClientContent,
    visuals: &mut VisualState,
    now: Instant,
) -> Option<u32> {
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
}
