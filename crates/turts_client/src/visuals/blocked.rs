//! The blocked marks over units that the simulation reports as blocked.

use super::materials::world_units_for_pixels;
use super::*;
use crate::{present::PresentedUnit, replica::Replica};

/// The time of the loud form of the blocked mark (red bar and cross). After it, a unit that is
/// still blocked keeps the quiet form until its activity changes.
const BLOCKED_ALERT_SECS: f32 = 2.0;
/// The quiet form: three small amber squares in a row (UX note of 2026-11-23, M4b wait mark).
/// The squares have a size in screen pixels (`waiting_square_px`), as the health bar has: a size in world units was
/// less than 2 pixels at the default camera height, and the anchor of the red bar is inside a
/// large model. The row is above the place of the health bar and does not touch it.
const WAITING_SQUARES: i8 = 3;
const WAITING_CLEAR_PX: f32 = 3.0;
/// The height of the health bar over the unit (`health.rs`).
const HEALTH_BAR_HEIGHT: f32 = 2.2;

/// The side and the gap of the waiting squares in pixels, from the width of the unit on the
/// screen (UX note of 2026-11-23): a row of the largest size over each unit of a close group of
/// small units gave joined lines. The row is 13 pixels for the smallest size and 26 pixels for
/// the largest.
fn waiting_square_px(unit_width_px: f32) -> (f32, f32) {
    let side = if unit_width_px.is_finite() {
        (unit_width_px / 5.0).round().clamp(3.0, 6.0)
    } else {
        6.0
    };
    let gap = if side <= 3.0 {
        2.0
    } else if side < 6.0 {
        3.0
    } else {
        4.0
    };
    (side, gap)
}

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
    camera: &Camera,
    camera_transform: &GlobalTransform,
    content: &crate::content::ClientContent,
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
        let ground = world_to_scene(unit.pos)
            + Vec3::Y * crate::terrain::render::live_height(terrain, unit.unit_id).unwrap_or(0.);
        let point = ground + Vec3::Y * 1.5;
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
            let bar = ground + Vec3::Y * HEALTH_BAR_HEIGHT;
            let per_pixel = world_units_for_pixels(camera, camera_transform, bar, Vec2::ONE);
            // The width of the unit on the screen: two times its declared pick radius.
            let width_px = crate::render::declared_unit_pick_radius(content, &unit.content_id)
                .map_or(f32::NAN, |radius| 2.0 * radius / per_pixel.x);
            let (side, gap) = waiting_square_px(width_px);
            let square = per_pixel * side;
            let rise = assets.health_bar_size.y / 2.0 + WAITING_CLEAR_PX + side / 2.0;
            let row = bar + Vec3::Y * (per_pixel.y * rise);
            let pitch = per_pixel.x * (side + gap);
            for square_index in 0..WAITING_SQUARES {
                let offset = f32::from(square_index - WAITING_SQUARES / 2) * pitch;
                desired.insert(
                    // Ordinal 0 is the red bar. A kept entity keeps its material, so the
                    // squares have their own ordinals.
                    id(square_index as u8 + 1),
                    (
                        mesh.clone(),
                        material.clone().into(),
                        Transform {
                            translation: row + Vec3::X * offset,
                            scale: Vec3::new(square.x, square.y, 0.08),
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

#[cfg(test)]
mod tests {
    use super::waiting_square_px;

    #[test]
    fn waiting_square_follows_the_unit_width_inside_its_limits() {
        // (unit width in pixels, side, gap, row width)
        for (width, side, gap, row) in [
            (4.0, 3.0, 2.0, 13.0),
            (14.0, 3.0, 2.0, 13.0),
            (20.0, 4.0, 3.0, 18.0),
            (25.0, 5.0, 3.0, 21.0),
            (30.0, 6.0, 4.0, 26.0),
            (400.0, 6.0, 4.0, 26.0),
            (f32::NAN, 6.0, 4.0, 26.0),
        ] {
            let (got_side, got_gap) = waiting_square_px(width);
            assert_eq!((got_side, got_gap), (side, gap), "unit width {width}");
            assert_eq!(
                3.0 * got_side + 2.0 * got_gap,
                row,
                "row of unit width {width}"
            );
        }
    }
}
