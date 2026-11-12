//! Smart commands: the authored smart-order rules at a screen point or a world point.

use super::*;

impl Core {
    pub(super) fn smart_command(
        &mut self,
        px: Vec2,
        view: &Viewport,
        world: &dyn WorldView,
        effects: &mut Vec<Effect>,
    ) {
        let Some(point) = projection::screen_to_world(px, view, self.camera) else {
            return;
        };
        self.smart_command_at_world_point(point, world, effects);
    }

    /// Applies the same authored smart-order rules used by pointer commands to a world-space point.
    pub fn smart_command_at_world_point(
        &mut self,
        point: Vec2,
        world: &dyn WorldView,
        effects: &mut Vec<Effect>,
    ) {
        let target_unit = nearest_unit_at(point, world);
        let target_node = nearest_node_at(point, world);
        let mut grouped: Vec<(String, OrderTarget, Vec<UnitId>)> = Vec::new();
        let mut omitted = Vec::new();
        for id in &self.selected {
            let Some(unit) = world
                .units()
                .iter()
                .find(|u| u.id == *id && u.owner_slot == world.own_slot())
            else {
                continue;
            };
            let selected_target = if let Some(target) = target_unit {
                let kind = if target.owner_slot == world.own_slot() {
                    SmartTarget::OwnUnit
                } else {
                    SmartTarget::ForeignUnit
                };
                (kind, OrderTarget::Unit(target.id))
            } else if let Some(node) = target_node {
                (SmartTarget::Node, OrderTarget::Node(node.id))
            } else {
                (
                    SmartTarget::Ground,
                    OrderTarget::Point(Pos {
                        x: point.x,
                        y: point.y,
                    }),
                )
            };
            let weapon_eligible =
                target_unit.is_some_and(|target| world.can_weapon_target(unit, target));
            let gather_eligible = target_node.is_some_and(|node| world.can_gather(unit, node));
            let rule = world.order_rules().smart.iter().find(|rule| {
                let target_match = match (selected_target.0, rule.target) {
                    (SmartTarget::ForeignUnit, SmartTarget::ForeignUnitWeaponTarget) => {
                        weapon_eligible
                    }
                    (SmartTarget::Node, SmartTarget::ResourceNodeAccepted) => gather_eligible,
                    (candidate, authored) => candidate == authored,
                };
                target_match
                    && unit.capabilities.contains(&rule.needs)
                    && world
                        .orders()
                        .iter()
                        .find(|order| order.id == rule.order)
                        .is_some_and(|order| {
                            order.units.is_empty()
                                || order
                                    .units
                                    .iter()
                                    .any(|allowed| allowed == &unit.content_id)
                        })
                    && world
                        .orders()
                        .iter()
                        .find(|order| order.id == rule.order)
                        .is_some_and(|order| smart_target_compatible(order, &selected_target.1))
            });
            if let Some(rule) = rule {
                let definition = world.orders().iter().find(|order| order.id == rule.order);
                let target = match (definition, selected_target.1) {
                    (Some(order), OrderTarget::Node(node))
                        if order.behaviour == Behaviour::Gather
                            && order.target == TargetKind::Node =>
                    {
                        OrderTarget::Node(node)
                    }
                    (Some(order), OrderTarget::Node(node))
                        if order.behaviour == Behaviour::Move
                            && matches!(
                                order.target,
                                TargetKind::Point | TargetKind::PointOrUnit
                            ) =>
                    {
                        world
                            .nodes()
                            .iter()
                            .find(|n| n.id == node)
                            .map(|n| {
                                OrderTarget::Point(Pos {
                                    x: n.position.x,
                                    y: n.position.y,
                                })
                            })
                            .unwrap_or(OrderTarget::None)
                    }
                    (Some(order), OrderTarget::Unit(target_id))
                        if order.behaviour == Behaviour::Move
                            && order.target == TargetKind::Point =>
                    {
                        world
                            .units()
                            .iter()
                            .find(|u| u.id == target_id)
                            .map(|u| {
                                OrderTarget::Point(Pos {
                                    x: u.position.x,
                                    y: u.position.y,
                                })
                            })
                            .unwrap_or(OrderTarget::None)
                    }
                    (_, value) => value,
                };
                if let Some((_, _, units)) = grouped.iter_mut().find(|(id, existing_target, _)| {
                    id == &rule.order && existing_target == &target
                }) {
                    units.push(*id);
                } else {
                    grouped.push((rule.order.clone(), target, vec![*id]));
                }
            } else {
                omitted.push(*id);
            }
        }
        let needs_group = grouped.len() > 1
            || grouped.iter().any(|(_, _, units)| {
                units.len() > 1 || units.len() > turts_net_protocol::MAX_ORDER_UNITS
            });
        let group = if needs_group { self.alloc_group() } else { 0 };
        let queue = if self.has_modifier_action("queue_modifier") {
            QueueMode::Append
        } else {
            QueueMode::Replace
        };
        for (order, target, units) in grouped {
            let Some(def) = world.orders().iter().find(|o| o.id == order) else {
                continue;
            };
            for chunk in units.chunks(turts_net_protocol::MAX_ORDER_UNITS) {
                effects.push(Effect::Send(OrderEnvelope {
                    order: order.clone(),
                    units: chunk.to_vec(),
                    target: target.clone(),
                    queue,
                    group,
                }));
            }
            let _ = def;
        }
        if !omitted.is_empty() {
            effects.push(Effect::Notice {
                text_key: "no_applicable_order".into(),
                units: omitted,
            });
        }
    }
}

fn smart_target_compatible(order: &OrderDef, target: &OrderTarget) -> bool {
    match target {
        OrderTarget::None => order.target == TargetKind::None,
        OrderTarget::Point(_) => {
            matches!(order.target, TargetKind::Point | TargetKind::PointOrUnit)
        }
        OrderTarget::Unit(_) => {
            matches!(order.target, TargetKind::Unit | TargetKind::PointOrUnit)
                || (order.behaviour == Behaviour::Move && order.target == TargetKind::Point)
        }
        OrderTarget::Node(_) => {
            (order.behaviour == Behaviour::Gather && order.target == TargetKind::Node)
                || (order.behaviour == Behaviour::Move
                    && matches!(order.target, TargetKind::Point | TargetKind::PointOrUnit))
        }
        _ => false,
    }
}

pub(super) fn nearest_unit<'a>(
    px: Vec2,
    view: &Viewport,
    camera: CameraState,
    world: &'a dyn WorldView,
) -> Option<&'a UnitView> {
    let point = projection::screen_to_world(px, view, camera)?;
    nearest_unit_at(point, world)
}

fn nearest_unit_at(point: Vec2, world: &dyn WorldView) -> Option<&UnitView> {
    world
        .units()
        .iter()
        .filter_map(|unit| {
            let distance = unit.position.distance(point);
            (distance <= unit.footprint_radius.max(0.0)).then_some((unit, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(unit, _)| unit)
}
pub(super) fn nearest_node<'a>(
    px: Vec2,
    view: &Viewport,
    camera: CameraState,
    world: &'a dyn WorldView,
) -> Option<&'a NodeView> {
    let point = projection::screen_to_world(px, view, camera)?;
    nearest_node_at(point, world)
}

fn nearest_node_at(point: Vec2, world: &dyn WorldView) -> Option<&NodeView> {
    world
        .nodes()
        .iter()
        .filter_map(|node| {
            let distance = node.position.distance(point);
            (distance <= node.radius.max(0.0)).then_some((node, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(node, _)| node)
}
