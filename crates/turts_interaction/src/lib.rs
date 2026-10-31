//! Renderer-independent input and interaction contracts for a player client.

pub mod camera;
pub mod compose;
pub mod groups;
pub mod input_map;
pub mod minimap;
pub mod projection;
pub mod selection;
pub mod smart;

use glam::Vec2;
use std::collections::{BTreeMap, HashMap};
use turts_mod_api::{
    Behaviour, Capability, InputMapDef, Mod, OrderDef, OrderRulesDef, SmartTarget, TargetKind,
};
use turts_net_protocol::{NodeId, OrderEnvelope, OrderTarget, Pos, QueueMode, SlotId, UnitId};

pub use turts_mod_api::Button;

/// The plain state machine that turns frame input into client effects.
#[derive(Debug)]
pub struct Core {
    camera: CameraState,
    selected: Vec<UnitId>,
    groups: Vec<Vec<UnitId>>,
    keys: Vec<String>,
    physical_mods: Vec<Mod>,
    held_modifier_actions: HashMap<String, Vec<String>>,
    pointer_down: Option<(Button, Vec2, bool, u64)>,
    pending_order: Option<String>,
    last_click: Option<(u64, Vec2, String)>,
    next_group: u32,
    camera_controller: camera::CameraController,
}

impl Core {
    /// Creates the interaction core with the caller's initial camera pose.
    pub fn new(initial_camera: CameraState) -> Self {
        Self {
            camera: initial_camera,
            selected: Vec::new(),
            groups: vec![Vec::new(); 10],
            keys: Vec::new(),
            physical_mods: Vec::new(),
            held_modifier_actions: HashMap::new(),
            pointer_down: None,
            pending_order: None,
            last_click: None,
            next_group: 1,
            camera_controller: camera::CameraController::default(),
        }
    }

    /// Returns the camera pose without allowing presentation to mutate core state.
    pub fn camera_state(&self) -> CameraState {
        self.camera
    }

    /// Read-only observation seam for the active unit selection.
    ///
    /// Task 9 preparation stub: behavior is implemented after semantic red evidence is recorded.
    pub fn selected_units(&self) -> &[UnitId] {
        &self.selected
    }

    /// Read-only observation seam for a control group.
    ///
    /// Task 9 preparation stub: behavior is implemented after semantic red evidence is recorded.
    pub fn group_members(&self, group: u8) -> &[UnitId] {
        self.groups
            .get(group as usize)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Projects a ground point into screen space for callers and deterministic fixtures.
    ///
    /// Task 9 preparation stub: behavior is implemented after semantic red evidence is recorded.
    pub fn project_world_to_screen(&self, point: Vec2, view: &Viewport) -> Vec2 {
        projection::world_to_screen(point, view, self.camera).unwrap_or(Vec2::ZERO)
    }

    /// Processes one frame of input. Interaction behavior is added by later tasks.
    pub fn step(
        &mut self,
        events: &[InputEvent],
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.prune(world);
        for event in events {
            match event {
                InputEvent::KeyDown { code, mods } => {
                    if !self.keys.contains(code) {
                        self.keys.push(code.clone());
                    }
                    self.physical_mods = mods.clone();
                    let modifier_actions = self.modifier_actions_for_key(code, world);
                    if !modifier_actions.is_empty() {
                        self.held_modifier_actions
                            .insert(code.clone(), modifier_actions);
                    }
                    let action = self.action_for_key(code, world);
                    if let Some(action) = action {
                        if let Some(group) = action
                            .strip_prefix("group_assign_")
                            .and_then(|value| value.parse::<u8>().ok())
                        {
                            self.assign_group(group, false, world);
                            effects.push(Effect::GroupsChanged);
                        } else if let Some(group) = action
                            .strip_prefix("group_add_")
                            .and_then(|value| value.parse::<u8>().ok())
                        {
                            self.assign_group(group, true, world);
                            effects.push(Effect::GroupsChanged);
                        } else if let Some(group) = action
                            .strip_prefix("group_recall_")
                            .and_then(|value| value.parse::<u8>().ok())
                        {
                            self.recall_group(group, world);
                            effects.push(Effect::SelectionChanged);
                        } else if let Some(order) =
                            world.orders().iter().find(|o| o.hotkey == action)
                        {
                            self.pending_order = Some(order.id.clone());
                            if order.target == TargetKind::None {
                                self.compose_explicit(
                                    order,
                                    OrderTarget::None,
                                    world,
                                    &mut effects,
                                );
                                self.pending_order = None;
                            }
                        } else if action == "menu" {
                            effects.push(Effect::ShellAction {
                                action: "menu".into(),
                            });
                        } else if action == "cancel" {
                            if self.pending_order.take().is_some() {
                                effects.push(Effect::Cursor {
                                    context: "default".into(),
                                });
                            } else {
                                effects.push(Effect::ShellAction {
                                    action: "menu".into(),
                                });
                            }
                        }
                    }
                }
                InputEvent::KeyUp { code } => {
                    self.keys.retain(|key| key != code);
                    self.held_modifier_actions.remove(code);
                    self.physical_mods = physical_modifiers(&self.keys);
                }
                InputEvent::FocusLost => {
                    self.keys.clear();
                    self.physical_mods.clear();
                    self.held_modifier_actions.clear();
                    self.pointer_down = None;
                }
                InputEvent::UiAction { name } => {
                    if let Some(unit_text) = name.strip_prefix("ui_select:") {
                        if let Ok(id) = unit_text.parse::<u32>()
                            && world.units().iter().any(|unit| {
                                unit.id == UnitId(id) && unit.owner_slot == world.own_slot()
                            })
                        {
                            self.selected.clear();
                            self.selected.push(UnitId(id));
                            effects.push(Effect::SelectionChanged);
                        }
                    } else if let Some(order) =
                        world.orders().iter().find(|order| order.id == *name)
                    {
                        if order.target == TargetKind::None {
                            self.compose_explicit(order, OrderTarget::None, world, &mut effects);
                        } else {
                            self.pending_order = Some(order.id.clone());
                            effects.push(Effect::Cursor {
                                context: order.cursor.clone().unwrap_or_else(|| "target".into()),
                            });
                        }
                    } else {
                        effects.push(Effect::ShellAction {
                            action: name.clone(),
                        });
                    }
                }
                InputEvent::RestoreGroups { groups } => {
                    self.groups = vec![Vec::new(); 10];
                    for (group, members) in groups {
                        if !(1..=9).contains(group) {
                            continue;
                        }
                        let mut seen = std::collections::HashSet::new();
                        self.groups[*group as usize] = members
                            .iter()
                            .copied()
                            .filter(|id| {
                                seen.insert(*id)
                                    && world.units().iter().any(|unit| {
                                        unit.id == *id && unit.owner_slot == world.own_slot()
                                    })
                            })
                            .collect();
                    }
                    effects.push(Effect::GroupsChanged);
                }
                InputEvent::PointerDown {
                    button,
                    px,
                    over_widget,
                } => {
                    if !over_widget {
                        self.pointer_down = Some((*button, *px, false, now_ms));
                    }
                }
                InputEvent::PointerMove { px, over_widget: _ } => {
                    if let Some((button, start, dragged, down_ms)) = self.pointer_down {
                        self.pointer_down =
                            Some((button, start, dragged || start.distance(*px) > 4.0, down_ms));
                    }
                }
                InputEvent::PointerUp {
                    button,
                    px,
                    over_widget,
                } => {
                    let Some((down_button, start, dragged, down_ms)) = self.pointer_down.take()
                    else {
                        continue;
                    };
                    if *over_widget || down_button != *button {
                        continue;
                    }
                    let dragged = dragged || start.distance(*px) > 4.0;
                    let pointer_action = self.action_for_pointer(down_button, world);
                    if matches!(pointer_action.as_deref(), Some("select" | "box_select")) {
                        if let Some(order_id) = self.pending_order.take() {
                            if let Some(order) =
                                world.orders().iter().find(|order| order.id == order_id)
                            {
                                if let Some(target) = self.explicit_target(order, *px, view, world)
                                {
                                    self.compose_explicit(order, target, world, &mut effects);
                                }
                                effects.push(Effect::Cursor {
                                    context: "default".into(),
                                });
                            }
                        } else if dragged {
                            self.box_select(start, *px, view, world);
                            effects.push(Effect::SelectionChanged);
                        } else {
                            self.select_click(*px, start, view, world, down_ms, &mut effects);
                        }
                    } else if pointer_action.as_deref() == Some("command") {
                        if self.pending_order.take().is_some() {
                            effects.push(Effect::Cursor {
                                context: "default".into(),
                            });
                        } else {
                            self.smart_command(*px, view, world, &mut effects);
                        }
                    } else if down_button == Button::Right && self.pending_order.take().is_some() {
                        effects.push(Effect::Cursor {
                            context: "default".into(),
                        });
                    }
                }
                InputEvent::PointerLeft => {
                    self.pointer_down = None;
                }
                InputEvent::Wheel { .. } => {}
                InputEvent::Minimap { button, local } => {
                    minimap::handle_input(self, *button, *local, world, view, now_ms, &mut effects);
                }
            }
        }
        self.prune(world);
        self.camera_controller.step(
            &mut self.camera,
            events,
            world,
            view,
            now_ms,
            &self.selected,
            &mut effects,
        );
        effects
    }
}

fn modifiers_equal(a: &[Mod], b: &[Mod]) -> bool {
    a.len() == b.len() && a.iter().all(|modifier| b.contains(modifier))
}
fn logical_modifier(action: &str) -> Option<Mod> {
    match action {
        "add_modifier" | "queue_modifier" => Some(Mod::Shift),
        "type_modifier" => Some(Mod::Ctrl),
        _ => None,
    }
}
fn physical_modifiers(keys: &[String]) -> Vec<Mod> {
    let mut modifiers = Vec::new();
    if keys.iter().any(|key| key.starts_with("Shift")) {
        modifiers.push(Mod::Shift);
    }
    if keys.iter().any(|key| key.starts_with("Control")) {
        modifiers.push(Mod::Ctrl);
    }
    if keys.iter().any(|key| key.starts_with("Alt")) {
        modifiers.push(Mod::Alt);
    }
    modifiers
}
fn modifiers_before_key(code: &str, physical: &[Mod]) -> Vec<Mod> {
    let introduced = if code.starts_with("Shift") {
        Some(Mod::Shift)
    } else if code.starts_with("Control") {
        Some(Mod::Ctrl)
    } else if code.starts_with("Alt") {
        Some(Mod::Alt)
    } else {
        None
    };
    physical
        .iter()
        .copied()
        .filter(|modifier| Some(*modifier) != introduced)
        .collect()
}

impl Core {
    fn modifier_actions_for_key(&self, code: &str, world: &dyn WorldView) -> Vec<String> {
        let physical = modifiers_before_key(code, &self.physical_mods);
        input_map::effective_actions_with_runtime(world.input_map(), world.runtime_binding_overrides()).into_iter().filter_map(|(name,bindings)| {
            (logical_modifier(&name).is_some() && bindings.iter().any(|binding|matches!(binding,turts_mod_api::Binding::Key{code:key,mods} if key==code&&modifiers_equal(mods,&physical)))).then_some(name)
        }).collect()
    }
    fn has_modifier_action(&self, action: &str) -> bool {
        self.held_modifier_actions
            .values()
            .any(|actions| actions.iter().any(|held| held == action))
    }
    fn action_for_key(&self, code: &str, world: &dyn WorldView) -> Option<String> {
        let actions = input_map::effective_actions_with_runtime(
            world.input_map(),
            world.runtime_binding_overrides(),
        );
        let exact = actions.iter().find_map(|(name, bindings)| bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Key { code: key, mods } if key == code && modifiers_equal(mods, &self.physical_mods))).then_some(name.clone()));
        exact.or_else(|| {
            (self.physical_mods.len() == 1 && self.physical_mods.contains(&Mod::Shift)).then(|| actions.into_iter().find_map(|(name, bindings)| {
                (world.orders().iter().any(|order| order.hotkey == name) && bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Key { code: key, mods } if key == code && mods.is_empty()))).then_some(name)
            })).flatten()
        })
    }
    fn action_for_pointer(&self, button: Button, world: &dyn WorldView) -> Option<String> {
        let actions = input_map::effective_actions_with_runtime(
            world.input_map(),
            world.runtime_binding_overrides(),
        );
        let exact = actions.iter().find_map(|(name, bindings)| {
            bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Pointer { button: bound, mods } if *bound == button && modifiers_equal(mods, &self.physical_mods))).then_some(name.clone())
        });
        exact.or_else(|| actions.into_iter().find_map(|(name, bindings)| {
            bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Pointer { button: bound, mods } if *bound == button && mods.is_empty() && !self.physical_mods.is_empty())).then_some(name)
        }))
    }
    fn prune(&mut self, world: &dyn WorldView) {
        let ids: std::collections::HashSet<_> = world.units().iter().map(|unit| unit.id).collect();
        self.selected.retain(|id| ids.contains(id));
        for group in &mut self.groups {
            group.retain(|id| ids.contains(id));
        }
    }
    fn assign_group(&mut self, group: u8, additive: bool, world: &dyn WorldView) {
        let members: Vec<_> = self
            .selected
            .iter()
            .copied()
            .filter(|id| {
                world
                    .units()
                    .iter()
                    .any(|u| u.id == *id && u.owner_slot == world.own_slot())
            })
            .collect();
        if group as usize >= self.groups.len() {
            return;
        }
        if additive {
            for id in members {
                if !self.groups[group as usize].contains(&id) {
                    self.groups[group as usize].push(id);
                }
            }
        } else {
            self.groups[group as usize] = members;
        }
        self.groups[group as usize].sort_by_key(|id| id.0);
    }
    fn recall_group(&mut self, group: u8, world: &dyn WorldView) {
        self.selected = self
            .groups
            .get(group as usize)
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| {
                world
                    .units()
                    .iter()
                    .any(|u| u.id == *id && u.owner_slot == world.own_slot())
            })
            .collect();
        self.selected.sort_by_key(|id| id.0);
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
        let ground = projection::screen_to_world(px, view, self.camera);
        let hit = ground.and_then(|point| {
            world
                .units()
                .iter()
                .filter_map(|unit| {
                    let distance = unit.position.distance(point);
                    (distance <= unit.footprint_radius.max(0.0)).then_some((unit, distance))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(unit, _)| unit)
        });
        let Some(unit) = hit else {
            if !self.has_modifier_action("add_modifier") {
                self.selected.clear();
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
    fn explicit_target(
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
    fn compose_explicit(
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
        for chunk in units.chunks(turts_net_protocol::MAX_ORDER_UNITS) {
            let eligible = chunk.to_vec();
            if eligible.is_empty() {
                continue;
            }
            effects.push(Effect::Send(OrderEnvelope {
                order: order.id.clone(),
                units: eligible,
                target: target.clone(),
                queue,
                group,
            }));
        }
    }
    fn smart_command(
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
    fn alloc_group(&mut self) -> u32 {
        let id = self.next_group.max(1);
        self.next_group = id.wrapping_add(1).max(1);
        id
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

fn nearest_unit<'a>(
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
fn nearest_node<'a>(
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

impl Default for Core {
    fn default() -> Self {
        Self::new(CameraState::default())
    }
}

/// Input observed by the client adapter during one frame.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    KeyDown {
        code: String,
        mods: Vec<Mod>,
    },
    KeyUp {
        code: String,
    },
    PointerMove {
        px: Vec2,
        over_widget: bool,
    },
    PointerDown {
        button: Button,
        px: Vec2,
        over_widget: bool,
    },
    PointerUp {
        button: Button,
        px: Vec2,
        over_widget: bool,
    },
    PointerLeft,
    Wheel {
        delta: f32,
    },
    FocusLost,
    UiAction {
        name: String,
    },
    /// Restore persisted control groups after validating membership against the joined world.
    RestoreGroups {
        groups: Vec<(u8, Vec<UnitId>)>,
    },
    /// Pointer input on the minimap, with widget-local coordinates in `0..1`.
    Minimap {
        button: Button,
        local: [f32; 2],
    },
}

/// A unit record that is safe for the renderer-independent interaction core.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitView {
    pub id: UnitId,
    pub content_id: String,
    pub owner_slot: SlotId,
    pub position: Vec2,
    pub footprint_radius: f32,
    pub capabilities: Vec<Capability>,
    /// Dense catalog index cached by the client adapter to avoid per-frame id parsing.
    pub content_index: u32,
}

/// A resource-node record visible to the interaction core.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub id: NodeId,
    pub content_id: String,
    pub position: Vec2,
    pub radius: f32,
    /// Dense catalog index cached by the client adapter.
    pub content_index: u32,
}

/// Read-only view of the replicated world and package input/order definitions.
pub trait WorldView {
    fn units(&self) -> &[UnitView];
    fn nodes(&self) -> &[NodeView];
    fn own_slot(&self) -> SlotId;
    fn orders(&self) -> &[OrderDef];
    fn order_rules(&self) -> &OrderRulesDef;
    fn input_map(&self) -> &InputMapDef;
    fn runtime_binding_overrides(&self) -> &BTreeMap<String, Vec<turts_mod_api::Binding>> {
        static EMPTY: std::sync::LazyLock<BTreeMap<String, Vec<turts_mod_api::Binding>>> =
            std::sync::LazyLock::new(BTreeMap::new);
        &EMPTY
    }
    fn interaction_settings(&self) -> InteractionSettings {
        InteractionSettings::default()
    }
    fn camera_options(&self) -> camera::CameraOptions {
        camera::CameraOptions::default()
    }
    fn map_size(&self) -> Option<Vec2> {
        None
    }
    /// Authored content priority used to choose the preferred class in mixed boxes.
    fn selection_priority(&self) -> &[String];
    fn selection_rank(&self, content_id: &str) -> usize {
        self.selection_priority()
            .iter()
            .position(|priority| priority == content_id)
            .unwrap_or(usize::MAX)
    }
    /// Returns whether this unit can gather the item represented by the node.
    fn can_gather(&self, unit: &UnitView, node: &NodeView) -> bool;
    /// Returns whether this unit's weapon can target this foreign unit.
    fn can_weapon_target(&self, unit: &UnitView, target: &UnitView) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractionSettings {
    pub double_click_ms: u64,
    pub double_click_pixels: f32,
}

impl Default for InteractionSettings {
    fn default() -> Self {
        Self {
            double_click_ms: 400,
            double_click_pixels: 4.0,
        }
    }
}

/// Logical viewport dimensions and device scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub size: Vec2,
    pub scale_factor: f32,
}

/// The renderer-independent camera pose held by the interaction core.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    pub look_at: Vec2,
    pub height: f32,
    /// Angle in radians from the vertical axis.
    pub pitch: f32,
    pub yaw: f32,
    pub vertical_fov: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            look_at: Vec2::ZERO,
            height: 40.0,
            pitch: 0.643_501_1,
            yaw: 0.0,
            vertical_fov: 0.9,
        }
    }
}

/// An output for the client adapter to apply or present.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Send(OrderEnvelope),
    SelectionChanged,
    GroupsChanged,
    CameraChanged,
    Marker {
        point: Vec2,
        kind: String,
    },
    Sound {
        id: String,
        category: String,
    },
    Notice {
        text_key: String,
        units: Vec<UnitId>,
    },
    Cursor {
        context: String,
    },
    ShellAction {
        action: String,
    },
}
