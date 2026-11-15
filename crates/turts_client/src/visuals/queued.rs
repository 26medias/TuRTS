//! The queued point targets of the selected own units.

use super::*;
use crate::{present::PresentedUnit, replica::Replica};

/// Selected own units show queued point targets in queue order.
pub(super) fn draw_queued_targets(
    gizmos: &mut Gizmos,
    selected: &SelectionFeed,
    replica: &Replica,
    presented_by_id: &HashMap<u32, &PresentedUnit>,
    local_slot: u8,
) {
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
        let mut previous = Vec3::new(
            unit.pos.x,
            0.04 + crate::terrain::render::live_height(replica.terrain.as_ref(), unit.unit_id)
                .unwrap_or(0.),
            unit.pos.y,
        );
        if let Some(turts_net_protocol::RallyTarget::Point(Pos { x, y })) = record.rally {
            let foot = Vec3::new(
                x,
                0.06 + crate::terrain::render::marker_height(
                    replica.terrain.as_ref(),
                    Pos { x, y },
                )
                .unwrap_or(0.),
                y,
            );
            let tip = foot + Vec3::Y * 1.2;
            let colour = Color::srgb(0.95, 0.85, 0.25);
            gizmos.line(previous, foot, colour);
            gizmos.line(foot, tip, colour);
            gizmos.line(tip, tip + Vec3::new(0.55, -0.2, 0.0), colour);
            gizmos.line(
                tip + Vec3::new(0.55, -0.2, 0.0),
                tip - Vec3::Y * 0.4,
                colour,
            );
        }

        for order in record.current.iter().chain(record.queued.iter()) {
            if let OrderTarget::Point(Pos { x, y }) = &order.target {
                let next = Vec3::new(
                    *x,
                    0.04 + crate::terrain::render::marker_height(
                        replica.terrain.as_ref(),
                        Pos { x: *x, y: *y },
                    )
                    .unwrap_or(0.),
                    *y,
                );
                gizmos.line(previous, next, Color::srgb(0.9, 0.9, 0.85));
                previous = next;
            }
        }
    }
}
