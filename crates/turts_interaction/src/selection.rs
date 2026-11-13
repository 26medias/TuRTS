//! Selection: click, double click, box selection, priority, and pruning.

use super::*;

impl Core {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn complete_selection_gesture(
        &mut self,
        start: Vec2,
        px: Vec2,
        dragged: bool,
        down_ms: u64,
        world: &dyn WorldView,
        view: &Viewport,
        effects: &mut Vec<Effect>,
    ) {
        if let Some(order_id) = self.pending_order.take() {
            if let Some(order) = world.orders().iter().find(|order| order.id == order_id) {
                if let Some(target) = self.explicit_target(order, px, view, world) {
                    self.compose_explicit(order, target, world, effects);
                }
                effects.push(Effect::Cursor {
                    context: "default".into(),
                });
            }
        } else if dragged {
            self.box_select(start, px, view, world);
            effects.push(Effect::SelectionChanged);
        } else {
            self.select_click(px, start, view, world, down_ms, effects);
        }
    }
    pub(super) fn prune(&mut self, world: &dyn WorldView) {
        let ids: std::collections::HashSet<_> = world.units().iter().map(|unit| unit.id).collect();
        self.selected.retain(|id| ids.contains(id));
        if !self.selected.is_empty()
            || self
                .selected_node
                .is_some_and(|id| !world.nodes().iter().any(|node| node.id == id))
        {
            self.selected_node = None;
        }
        for group in &mut self.groups {
            group.retain(|id| ids.contains(id));
        }
    }
    fn select_click(
        &mut self,
        px: Vec2,
        press_px: Vec2,
        view: &Viewport,
        world: &dyn WorldView,
        now: u64,
        effects: &mut Vec<Effect>,
    ) {
        let hit = super::smart::nearest_unit(px, view, self.camera, world);
        self.selected_node = None;
        let Some(unit) = hit else {
            if !self.has_modifier_action("add_modifier") {
                self.selected.clear();
                self.last_click = None;
                self.selected_node =
                    super::smart::nearest_node(px, view, self.camera, world).map(|node| node.id);
                effects.push(Effect::SelectionChanged);
            }
            return;
        };
        let own_hit = unit.owner_slot == world.own_slot();
        let content = unit.content_id.clone();
        let settings = world.interaction_settings();
        let double = own_hit
            && !self.has_modifier_action("add_modifier")
            && !self.has_modifier_action("type_modifier")
            && self.last_click.as_ref().is_some_and(|(time, point, id)| {
                now.saturating_sub(*time) <= settings.double_click_ms
                    && point.distance(press_px) <= settings.double_click_pixels
                    && id == &content
            });
        self.last_click = own_hit.then(|| (now, press_px, content.clone()));
        if double || (own_hit && self.has_modifier_action("type_modifier")) {
            self.selected = world
                .units()
                .iter()
                .filter(|u| {
                    u.owner_slot == world.own_slot()
                        && u.content_id == content
                        && projection::world_to_screen(u.position, view, self.camera).is_some_and(
                            |p| {
                                p.x >= 0.0 && p.y >= 0.0 && p.x <= view.size.x && p.y <= view.size.y
                            },
                        )
                })
                .map(|u| u.id)
                .collect();
            self.selected.sort_by_key(|id| {
                world
                    .units()
                    .iter()
                    .find(|unit| unit.id == *id)
                    .map_or(usize::MAX, |unit| world.selection_rank(&unit.content_id))
            });
            self.apply_priority(world);
        } else if self.has_modifier_action("add_modifier") {
            if let Some(index) = self.selected.iter().position(|id| *id == unit.id) {
                self.selected.remove(index);
            } else {
                self.selected.push(unit.id);
            }
        } else {
            self.selected.clear();
            self.selected.push(unit.id);
        }
        effects.push(Effect::SelectionChanged);
    }
    fn box_select(&mut self, a: Vec2, b: Vec2, view: &Viewport, world: &dyn WorldView) {
        self.selected_node = None;
        let lo = a.min(b);
        let hi = a.max(b);
        let candidates: Vec<_> = world
            .units()
            .iter()
            .filter(|u| u.owner_slot == world.own_slot())
            .filter_map(|u| {
                let px = projection::world_to_screen(u.position, view, self.camera)?;
                (px.x >= lo.x && px.x <= hi.x && px.y >= lo.y && px.y <= hi.y).then_some(u)
            })
            .collect();
        let class = candidates
            .iter()
            .filter_map(|unit| {
                let rank = world.selection_rank(&unit.content_id);
                (rank != usize::MAX).then_some((
                    rank,
                    unit.id.0,
                    unit.capabilities.contains(&Capability::Movement),
                ))
            })
            .min_by_key(|(rank, id, _)| (*rank, *id))
            .map(|(_, _, mobile)| mobile)
            .unwrap_or_else(|| {
                candidates
                    .iter()
                    .any(|unit| unit.capabilities.contains(&Capability::Movement))
            });
        let mut picked: Vec<_> = candidates
            .into_iter()
            .filter(|u| u.capabilities.contains(&Capability::Movement) == class)
            .collect();
        picked.sort_by_key(|u| (world.selection_rank(&u.content_id), u.id.0));
        let ids: Vec<_> = picked.into_iter().map(|u| u.id).collect();
        if self.has_modifier_action("add_modifier") {
            for id in ids {
                if !self.selected.contains(&id) {
                    self.selected.push(id);
                }
            }
        } else {
            self.selected = ids;
        }
    }
    fn apply_priority(&mut self, world: &dyn WorldView) {
        let mobile = self
            .selected
            .iter()
            .filter_map(|id| world.units().iter().find(|u| u.id == *id))
            .any(|u| u.capabilities.contains(&Capability::Movement));
        self.selected.retain(|id| {
            world
                .units()
                .iter()
                .find(|u| u.id == *id)
                .is_some_and(|u| u.capabilities.contains(&Capability::Movement) == mobile)
        });
        self.selected.sort_by_key(|id| {
            world
                .units()
                .iter()
                .find(|unit| unit.id == *id)
                .map_or((usize::MAX, id.0), |unit| {
                    (world.selection_rank(&unit.content_id), id.0)
                })
        });
    }
}
