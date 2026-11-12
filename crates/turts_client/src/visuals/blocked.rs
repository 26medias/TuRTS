//! The blocked marks over units that the simulation reports as blocked.

use super::*;
use crate::{present::PresentedUnit, replica::Replica};

/// Start and end the two-second blocked marks from the replicated order records.
pub(super) fn track_blocked(replica: &Replica, visuals: &mut VisualState, now: Instant) {
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
}

/// Draw the blocked mark of each unit that is inside its two seconds.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_blocked_marks(
    desired: &mut DesiredVisuals,
    markers_shown: &mut MarkersShown,
    marker_gizmos: &mut Gizmos<OrderMarkerGizmoConfigGroup>,
    presented_by_id: &HashMap<u32, &PresentedUnit>,
    assets: &VisualAssets,
    visuals: &VisualState,
    now: Instant,
) {
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
}
