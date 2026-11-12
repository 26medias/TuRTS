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
        let mut previous = Vec3::new(unit.pos.x, 0.04, unit.pos.y);
        for order in record.current.iter().chain(record.queued.iter()) {
            if let OrderTarget::Point(Pos { x, y }) = &order.target {
                let next = Vec3::new(*x, 0.04, *y);
                gizmos.line(previous, next, Color::srgb(0.9, 0.9, 0.85));
                previous = next;
            }
        }
    }
}
