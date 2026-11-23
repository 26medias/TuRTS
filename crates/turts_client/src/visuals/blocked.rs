//! The blocked marks over units that the simulation reports as blocked.

use super::*;
use crate::{present::PresentedUnit, replica::Replica};

/// The time of the loud form of the blocked mark (red bar and cross). After it, a unit that is
/// still blocked keeps the quiet form until its activity changes.
const BLOCKED_ALERT_SECS: f32 = 2.0;
/// The quiet form: three small amber squares in a row (UX note of 2026-11-23, M4b wait mark).
const WAITING_SQUARES: i8 = 3;
const WAITING_SQUARE: f32 = 0.08;
const WAITING_PITCH: f32 = WAITING_SQUARE + 0.14;

/// Start and end the blocked marks from the replicated order records.
pub(super) fn track_blocked(replica: &Replica, visuals: &mut VisualState, now: Instant) {
    let blocked_now = replica
        .unit_orders
        .iter()
        .filter_map(|(&unit_id, order)| (order.activity == Activity::Blocked).then_some(unit_id))
        .collect();
    // The quiet mark stays for a long time, so it is for the units of the local slot only: the
    // same rule as the activity line of the selection panel (UX note of 2026-11-23).
    let local_slot = visuals.session.as_ref().map(|session| session.local_slot);
    visuals.blocked_own = blocked_now_own(replica, local_slot);
    let visuals_state = &mut *visuals;
    refresh_blocked_marks(
        blocked_now,
        &mut visuals_state.blocked_active,
        &mut visuals_state.blocked_since,
        now,
    );
}

/// The blocked units of the local slot.
fn blocked_now_own(replica: &Replica, local_slot: Option<u8>) -> HashSet<u32> {
    replica
        .unit_orders
        .iter()
        .filter(|(_, order)| order.activity == Activity::Blocked)
        .filter(|(unit_id, _)| {
            replica
                .units
                .get(unit_id)
                .is_some_and(|unit| Some(unit.owner_slot) == local_slot)
        })
        .map(|(&unit_id, _)| unit_id)
        .collect()
}

/// Draw the blocked mark of each unit. Inside the first two seconds the mark is the red bar and
/// the cross. After that, the quiet amber squares stay while the replicated activity of a
/// unit of the local slot is `Blocked`. They end with the first record that has another activity, or no record for the
/// unit. The quiet form has no cross, no motion and no sound.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_blocked_marks(
    desired: &mut DesiredVisuals,
    markers_shown: &mut MarkersShown,
    marker_gizmos: &mut Gizmos<OrderMarkerGizmoConfigGroup>,
    presented_by_id: &HashMap<u32, &PresentedUnit>,
    assets: &VisualAssets,
    visuals: &VisualState,
    now: Instant,
    terrain: Option<&crate::terrain::store::PermittedTerrain>,
) {
    let Some(mesh) = assets.bar.clone() else {
        return;
    };
    for (&unit_id, start) in &visuals.blocked_since {
        let alert = now.duration_since(*start).as_secs_f32() < BLOCKED_ALERT_SECS;
        if !alert && !visuals.blocked_own.contains(&unit_id) {
            continue;
        }
        let Some(unit) = presented_by_id.get(&unit_id) else {
            continue;
        };
        let point = world_to_scene(unit.pos)
            + Vec3::Y
                * (1.5 + crate::terrain::render::live_height(terrain, unit.unit_id).unwrap_or(0.));
        let id = |ordinal| VisualId {
            kind: VisualKind::Blocked,
            unit: unit_id,
            ordinal,
            generation: visuals.generation,
            identity: 0,
        };
        if !alert {
            let Some(material) = assets.waiting_material.clone() else {
                continue;
            };
            for square in 0..WAITING_SQUARES {
                let offset = f32::from(square - WAITING_SQUARES / 2) * WAITING_PITCH;
                desired.insert(
                    // Ordinal 0 is the red bar. A kept entity keeps its material, so the
                    // squares have their own ordinals.
                    id(square as u8 + 1),
                    (
                        mesh.clone(),
                        material.clone().into(),
                        Transform {
                            translation: point + Vec3::X * offset,
                            scale: Vec3::splat(WAITING_SQUARE),
                            ..default()
                        },
                    ),
                );
            }
            markers_shown
                .0
                .push(serde_json::json!({"kind":"blocked_waiting","unit":unit_id}));
            continue;
        }
        let Some(material) = assets.blocked_material.clone() else {
            continue;
        };
        desired.insert(
            id(0),
            (
                mesh.clone(),
                material.into(),
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
