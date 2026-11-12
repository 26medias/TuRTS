//! The rings of the selected units: ring, neutral underlay and weapon range.

use super::*;
use crate::{content::ClientContent, present::PresentedUnit, replica::Replica};

/// Write the ring visuals of every selected unit into `desired` and `ring_metadata`.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_selection_rings(
    desired: &mut DesiredVisuals,
    ring_metadata: &mut HashMap<u32, RingMetadata>,
    selected: &SelectionFeed,
    presented_by_id: &HashMap<u32, &PresentedUnit>,
    replica: &Replica,
    content: &ClientContent,
    palette: &SlotMaterials,
    assets: &VisualAssets,
    local_slot: u8,
    visuals: &VisualState,
) {
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
}
