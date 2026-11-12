//! Command composition: explicit targets and the order envelopes of the selection.

use super::smart::{nearest_node, nearest_unit};
use super::*;

impl Core {
    pub(super) fn explicit_target(
        &self,
        order: &OrderDef,
        px: Vec2,
        view: &Viewport,
        world: &dyn WorldView,
    ) -> Option<OrderTarget> {
        let point = projection::screen_to_world(px, view, self.camera)?;
        match order.target {
            TargetKind::None => Some(OrderTarget::None),
            TargetKind::Point => Some(OrderTarget::Point(Pos {
                x: point.x,
                y: point.y,
            })),
            TargetKind::Unit => {
                nearest_unit(px, view, self.camera, world).map(|u| OrderTarget::Unit(u.id))
            }
            TargetKind::PointOrUnit if order.behaviour == Behaviour::Move => {
                Some(OrderTarget::Point(Pos {
                    x: point.x,
                    y: point.y,
                }))
            }
            TargetKind::PointOrUnit => nearest_unit(px, view, self.camera, world).map_or(
                Some(OrderTarget::Point(Pos {
                    x: point.x,
                    y: point.y,
                })),
                |u| Some(OrderTarget::Unit(u.id)),
            ),
            TargetKind::Node => {
                nearest_node(px, view, self.camera, world).map(|n| OrderTarget::Node(n.id))
            }
            _ => None,
        }
    }
    pub(super) fn compose_explicit(
        &mut self,
        order: &OrderDef,
        target: OrderTarget,
        world: &dyn WorldView,
        effects: &mut Vec<Effect>,
    ) {
        let units: Vec<_> = self
            .selected
            .iter()
            .copied()
            .filter(|id| {
                world.units().iter().any(|u| {
                    u.id == *id
                        && u.owner_slot == world.own_slot()
                        && (order.units.is_empty()
                            || order.units.iter().any(|allowed| allowed == &u.content_id))
                })
            })
            .collect();
        let group = if units.len() > 1 {
            self.alloc_group()
        } else {
            0
        };
        let queue = if self.has_modifier_action("queue_modifier") {
            QueueMode::Append
        } else {
            QueueMode::Replace
        };
        send_explicit_units(order, &units, target, queue, group, effects);
    }
}

pub(super) fn intended_units(
    order: &OrderDef,
    requested: &[UnitId],
    world: &dyn WorldView,
) -> Option<Vec<UnitId>> {
    let mut seen = std::collections::HashSet::new();
    let mut valid = Vec::new();
    for id in requested {
        if !seen.insert(*id) {
            continue;
        }
        let Some(unit) = world.units().iter().find(|unit| unit.id == *id) else {
            continue;
        };
        if unit.owner_slot != world.own_slot()
            || (!order.units.is_empty()
                && !order
                    .units
                    .iter()
                    .any(|content| content == &unit.content_id))
        {
            continue;
        }
        valid.push(*id);
    }
    Some(valid)
}

pub(super) fn send_explicit_units(
    order: &OrderDef,
    units: &[UnitId],
    target: OrderTarget,
    queue: QueueMode,
    group: u32,
    effects: &mut Vec<Effect>,
) {
    for chunk in units.chunks(turts_net_protocol::MAX_ORDER_UNITS) {
        if !chunk.is_empty() {
            effects.push(Effect::Send(OrderEnvelope {
                order: order.id.clone(),
                units: chunk.to_vec(),
                target: target.clone(),
                queue,
                group,
            }));
        }
    }
}
