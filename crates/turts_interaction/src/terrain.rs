//! Reference-height then rectangle input for ordinary queued terrain orders.
use crate::{Button, Core, Effect, InputEvent, Vec2, Viewport, WorldView, projection};
use turts_mod_api::{
    Behaviour, ContentId, TargetKind, TerrainCell, TerrainDesignation, TerrainRect,
};
use turts_net_protocol::{OrderEnvelope, OrderTarget, QueueMode, UnitId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub order: String,
    pub tool: String,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    input: Option<Input>,
    identity: Option<(u64, ContentId)>,
    reference: Option<i32>,
    start: Option<TerrainCell>,
    pressed: Option<(Button, Vec2)>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CellPreview {
    pub cell: TerrainCell,
    pub elevation: i32,
    pub current: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub input: Input,
    pub target_elevation: Option<i32>,
    pub cells: Vec<CellPreview>,
    pub rectangle: Option<TerrainRect>,
    pub can_submit: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Output {
    pub preview: Option<Preview>,
}

fn eligible(core: &Core, input: &Input, world: &dyn WorldView) -> Vec<UnitId> {
    let Some(order) = world.orders().iter().find(|order| {
        order.id == input.order
            && order.behaviour == Behaviour::TerrainWork
            && order.target == TargetKind::TerrainDesignation
    }) else {
        return vec![];
    };
    core.selected
        .iter()
        .copied()
        .filter(|id| {
            world.units().iter().any(|unit| {
                unit.id == *id
                    && unit.owner_slot == world.own_slot()
                    && (order.units.is_empty() || order.units.contains(&unit.content_id))
                    && world.terrain_tool(unit, &input.tool).is_some()
            })
        })
        .collect()
}
pub(crate) fn cancel(core: &mut Core, effects: &mut Vec<Effect>) {
    if core.terrain.input.is_some() {
        core.terrain = State::default();
        core.pending_order = None;
        core.pointer_down = None;
        effects.push(Effect::TerrainWork(Output::default()));
        effects.push(Effect::Cursor {
            context: "default".into(),
        });
    }
}
pub(crate) fn on_input(
    core: &mut Core,
    input: &Input,
    world: &dyn WorldView,
    effects: &mut Vec<Effect>,
) {
    if core.ui_capture.is_some() {
        return;
    }
    cancel(core, effects);
    let Some(identity) = world.terrain_identity() else {
        effects.push(Effect::Notice {
            text_key: "Waiting for permitted terrain state".into(),
            units: vec![],
        });
        return;
    };
    if eligible(core, input, world).is_empty() {
        effects.push(Effect::Notice {
            text_key: "No selected worker can use this tool".into(),
            units: vec![],
        });
        return;
    }
    core.placement = Default::default();
    core.pointer_down = None;
    core.pending_order = Some(input.order.clone());
    core.terrain = State {
        input: Some(input.clone()),
        identity: Some(identity),
        ..Default::default()
    };
    effects.push(Effect::Cursor {
        context: "target".into(),
    });
}
fn rectangle(a: TerrainCell, b: TerrainCell, world: &dyn WorldView) -> Option<TerrainRect> {
    let grid = world.terrain_grid()?;
    let min = TerrainCell {
        x: a.x.min(b.x),
        y: a.y.min(b.y),
    };
    let max = TerrainCell {
        x: a.x.max(b.x).checked_add(1)?,
        y: a.y.max(b.y).checked_add(1)?,
    };
    (max.x <= grid.columns && max.y <= grid.rows).then_some(TerrainRect { min, max })
}
fn hit(px: Vec2, core: &Core, world: &dyn WorldView, view: &Viewport) -> Option<crate::SurfaceHit> {
    projection::surface_hit(px, view, core.camera, world)
        .filter(|hit| hit.cell.is_some() && hit.elevation.is_some())
}
pub(crate) fn refresh(
    core: &mut Core,
    world: &dyn WorldView,
    view: &Viewport,
    effects: &mut Vec<Effect>,
) {
    let Some(input) = core.terrain.input.clone() else {
        return;
    };
    if core.terrain.identity != world.terrain_identity() || eligible(core, &input, world).is_empty()
    {
        cancel(core, effects);
        return;
    }
    let pointer = core
        .pointer
        .filter(|(_, over_widget)| !over_widget)
        .and_then(|(px, _)| hit(px, core, world, view));
    let mut preview = Preview {
        input,
        target_elevation: core.terrain.reference,
        cells: vec![],
        rectangle: None,
        can_submit: false,
        reason: None,
    };
    if let Some(reference) = core.terrain.reference {
        if let (Some(start), Some(end)) = (core.terrain.start, pointer.and_then(|hit| hit.cell)) {
            if let Some(rect) = rectangle(start, end, world) {
                let count = u64::from(rect.max.x - rect.min.x) * u64::from(rect.max.y - rect.min.y);
                preview.rectangle = Some(rect);
                if count <= 256 {
                    // Enumeration depends only on the submitted public rectangle.
                    for y in rect.min.y..rect.max.y {
                        for x in rect.min.x..rect.max.x {
                            let cell = TerrainCell { x, y };
                            let Some(elevation) = world.terrain_elevation(cell) else {
                                preview.reason = Some("Terrain geometry unavailable".into());
                                continue;
                            };
                            preview.cells.push(CellPreview {
                                cell,
                                elevation,
                                current: world.terrain_current(cell),
                            });
                        }
                    }
                    preview.can_submit = preview.cells.len() as u64 == count;
                } else {
                    preview.reason = Some("A work rectangle can contain at most 256 cells".into());
                }
            }
        }
        preview.target_elevation = Some(reference);
    } else if let Some(hit) = pointer {
        if !hit.current {
            preview.reason =
                Some("Observe the whole reference cell before choosing its level".into());
        }
        preview.cells.push(CellPreview {
            cell: hit.cell.unwrap(),
            elevation: hit.elevation.unwrap(),
            current: hit.current,
        });
    }
    effects.push(Effect::TerrainWork(Output {
        preview: Some(preview),
    }));
}
pub(crate) fn intercept(
    core: &mut Core,
    event: &InputEvent,
    world: &dyn WorldView,
    view: &Viewport,
    effects: &mut Vec<Effect>,
) -> bool {
    let Some(input) = core.terrain.input.as_ref() else {
        return false;
    };
    if core.terrain.identity != world.terrain_identity() || eligible(core, input, world).is_empty()
    {
        cancel(core, effects);
        return false;
    }
    match event {
        InputEvent::KeyDown { code, .. }
            if core.action_for_key(code, world).as_deref() == Some("cancel") =>
        {
            cancel(core, effects);
            true
        }
        InputEvent::UiAction { name } if name == "cancel" => {
            cancel(core, effects);
            true
        }
        InputEvent::FocusLost | InputEvent::UiCapture { .. } => {
            cancel(core, effects);
            false
        }
        InputEvent::PointerMove { px, over_widget } => {
            core.pointer = Some((*px, *over_widget));
            true
        }
        InputEvent::PointerDown {
            button,
            px,
            over_widget: false,
        } if matches!(button, Button::Left | Button::Right) && core.ui_capture.is_none() => {
            core.pointer = Some((*px, false));
            core.pointer_down = None;
            if *button == Button::Right {
                cancel(core, effects);
                return true;
            }
            core.terrain.pressed = Some((*button, *px));
            if core.terrain.reference.is_some() {
                core.terrain.start = hit(*px, core, world, view).and_then(|hit| hit.cell);
            }
            true
        }
        InputEvent::PointerUp {
            button,
            px,
            over_widget,
        } if *button == Button::Left => {
            let pressed = core.terrain.pressed.take();
            core.pointer_down = None;
            core.held_buttons.retain(|held| held != button);
            core.held_modifier_actions
                .remove(&format!("pointer:{button:?}"));
            if *over_widget || core.ui_capture.is_some() {
                core.terrain.start = None;
                return true;
            }
            if core.terrain.reference.is_none() {
                if pressed.is_some_and(|(_, start)| start.distance(*px) <= crate::CLICK_DRAG_PIXELS)
                {
                    if let Some(hit) = hit(*px, core, world, view).filter(|hit| hit.current) {
                        core.terrain.reference = hit.elevation;
                    } else {
                        effects.push(Effect::Notice {
                            text_key: "Choose an observed reference cell".into(),
                            units: vec![],
                        });
                    }
                }
            } else if let (Some(start), Some(end)) = (
                core.terrain.start.take(),
                hit(*px, core, world, view).and_then(|hit| hit.cell),
            ) {
                if let Some(cells) = rectangle(start, end, world).filter(|cells| {
                    u64::from(cells.max.x - cells.min.x) * u64::from(cells.max.y - cells.min.y)
                        <= 256
                }) {
                    let input = core.terrain.input.clone().unwrap();
                    let units = eligible(core, &input, world);
                    if !units.is_empty() {
                        let group = if units.len() > 1 {
                            core.alloc_group()
                        } else {
                            0
                        };
                        let append = core.has_modifier_action("queue_modifier");
                        effects.push(Effect::Send(OrderEnvelope {
                            order: input.order,
                            units,
                            target: OrderTarget::TerrainWork(TerrainDesignation {
                                tool: input.tool,
                                cells,
                                target_elevation: core.terrain.reference.unwrap(),
                            }),
                            queue: if append {
                                QueueMode::Append
                            } else {
                                QueueMode::Replace
                            },
                            group,
                        }));
                        if !append {
                            cancel(core, effects);
                        }
                    }
                } else {
                    effects.push(Effect::Notice {
                        text_key: "A work rectangle can contain at most 256 cells".into(),
                        units: vec![],
                    });
                }
            }
            true
        }
        _ => false,
    }
}

/// An explicit owner action. It carries only the previously visible designation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnloadResume {
    pub identity: (u64, ContentId),
    pub unit: UnitId,
    pub depot: UnitId,
    pub unload_order: String,
    pub work_order: String,
    pub designation: TerrainDesignation,
}
pub(crate) fn unload_resume(
    core: &mut Core,
    input: &UnloadResume,
    world: &dyn WorldView,
    effects: &mut Vec<Effect>,
) {
    if core.ui_capture.is_some() || world.terrain_identity() != Some(input.identity.clone()) {
        return;
    }
    let Some(actor) = world
        .units()
        .iter()
        .find(|unit| unit.id == input.unit && unit.owner_slot == world.own_slot())
    else {
        return;
    };
    let Some(depot) = world
        .units()
        .iter()
        .find(|unit| unit.id == input.depot && unit.owner_slot == world.own_slot())
    else {
        return;
    };
    let Some((work_order, designation)) = world.terrain_active_order(input.unit) else {
        return;
    };
    if work_order != input.work_order
        || designation != &input.designation
        || !world.can_unload(actor, depot)
        || world.terrain_tool(actor, &designation.tool).is_none()
    {
        return;
    }
    let Some(unload) = world.orders().iter().find(|order| {
        order.id == input.unload_order
            && order.behaviour == Behaviour::UnloadCargo
            && order.target == TargetKind::Unit
            && (order.units.is_empty() || order.units.contains(&actor.content_id))
    }) else {
        return;
    };
    let Some(_work) = world.orders().iter().find(|order| {
        order.id == input.work_order
            && order.behaviour == Behaviour::TerrainWork
            && order.target == TargetKind::TerrainDesignation
            && (order.units.is_empty() || order.units.contains(&actor.content_id))
    }) else {
        return;
    };
    cancel(core, effects);
    // API17: an appended unload utility pauses the exact active work and carry.
    effects.push(Effect::Send(OrderEnvelope {
        order: unload.id.clone(),
        units: vec![input.unit],
        target: OrderTarget::Unit(input.depot),
        queue: QueueMode::Append,
        group: 0,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NodeView, UnitView, test_common::TestWorld};
    use turts_mod_api::{
        InputMapDef, OrderDef, OrderRulesDef, ResolvedTerrainOperation, ResolvedTerrainTool,
        TerrainOutputDestination,
    };
    use turts_net_protocol::SlotId;
    const VIEW: Viewport = Viewport {
        size: Vec2::new(1280., 720.),
        scale_factor: 1.,
    };
    struct World {
        base: TestWorld,
        tool: ResolvedTerrainTool,
        epoch: u64,
        designation: TerrainDesignation,
        can_unload: bool,
        height: f32,
    }
    impl World {
        fn new() -> Self {
            let mut base = TestWorld::default();
            base.orders.push(crate::test_common::order(
                "forge:work",
                Behaviour::TerrainWork,
                TargetKind::TerrainDesignation,
            ));
            base.orders.push(crate::test_common::order(
                "forge:unload",
                Behaviour::UnloadCargo,
                TargetKind::Unit,
            ));
            Self {
                base,
                tool: ResolvedTerrainTool {
                    id: ContentId::parse("forge:spade").unwrap(),
                    name: "Spade".into(),
                    operation: ResolvedTerrainOperation::Excavate,
                    work_per_second: 1,
                    reach: 1.,
                    allowed_materials: vec![],
                    max_hardness: 1,
                    output_destination: TerrainOutputDestination::Inventory,
                },
                epoch: 1,
                designation: TerrainDesignation {
                    tool: "forge:spade".into(),
                    cells: TerrainRect {
                        min: TerrainCell { x: 1, y: 1 },
                        max: TerrainCell { x: 3, y: 3 },
                    },
                    target_elevation: 0,
                },
                can_unload: true,
                height: 0.,
            }
        }
    }
    impl WorldView for World {
        fn units(&self) -> &[UnitView] {
            &self.base.units
        }
        fn nodes(&self) -> &[NodeView] {
            &self.base.nodes
        }
        fn own_slot(&self) -> SlotId {
            SlotId(0)
        }
        fn orders(&self) -> &[OrderDef] {
            &self.base.orders
        }
        fn order_rules(&self) -> &OrderRulesDef {
            &self.base.rules
        }
        fn input_map(&self) -> &InputMapDef {
            &self.base.input
        }
        fn selection_priority(&self) -> &[String] {
            &self.base.selection_priority
        }
        fn can_gather(&self, _: &UnitView, _: &NodeView) -> bool {
            false
        }
        fn can_produce(&self, _: &UnitView, _: &str) -> bool {
            false
        }
        fn has_job(&self, _: &UnitView, _: turts_net_protocol::JobId) -> bool {
            false
        }
        fn can_weapon_target(&self, _: &UnitView, _: &UnitView) -> bool {
            false
        }
        fn terrain_enabled(&self) -> bool {
            true
        }
        fn terrain_identity(&self) -> Option<(u64, ContentId)> {
            Some((self.epoch, ContentId::parse("forge:map").unwrap()))
        }
        fn terrain_grid(&self) -> Option<turts_rules::terrain::geometry::Grid> {
            turts_rules::terrain::geometry::Grid::checked(32, 32, 1, 1, 1, -8, 8).ok()
        }
        fn terrain_tool(&self, unit: &UnitView, tool: &str) -> Option<&ResolvedTerrainTool> {
            (unit.id == UnitId(1) && tool == "forge:spade").then_some(&self.tool)
        }
        fn terrain_current(&self, cell: TerrainCell) -> bool {
            cell.x < 2
        }
        fn terrain_elevation(&self, _: TerrainCell) -> Option<i32> {
            Some(0)
        }
        fn terrain_active_order(&self, unit: UnitId) -> Option<(&str, &TerrainDesignation)> {
            (unit == UnitId(1)).then_some(("forge:work", &self.designation))
        }
        fn can_unload(&self, _: &UnitView, target: &UnitView) -> bool {
            self.can_unload && target.id == UnitId(2)
        }
        fn camera_ground_height(&self, _: Vec2) -> f32 {
            self.height
        }
        fn unit_ground_height(&self, _: &UnitView) -> Option<f32> {
            Some(self.height)
        }
        fn surface_hit(
            &self,
            origin: glam::Vec3,
            direction: glam::Vec3,
        ) -> Option<crate::SurfaceHit> {
            let t = (self.height - origin.y) / direction.y;
            let p = origin + direction * t;
            if t < 0. || p.x < 0. || p.z < 0. || p.x >= 32. || p.z >= 32. {
                return None;
            }
            let cell = TerrainCell {
                x: p.x.floor() as u32,
                y: p.z.floor() as u32,
            };
            Some(crate::SurfaceHit {
                position: Vec2::new(p.x, p.z),
                height: self.height,
                elevation: Some(0),
                cell: Some(cell),
                parameter: t,
                current: self.terrain_current(cell),
            })
        }
    }
    fn core() -> Core {
        let mut c = Core::default();
        c.selected = vec![UnitId(1)];
        c
    }
    fn enter(c: &mut Core, w: &World) {
        on_input(
            c,
            &Input {
                order: "forge:work".into(),
                tool: "forge:spade".into(),
            },
            w,
            &mut vec![],
        );
        assert!(c.terrain.input.is_some());
    }
    fn px(c: &Core, x: f32, y: f32) -> Vec2 {
        c.project_world_to_screen(Vec2::new(x, y), &VIEW)
    }
    fn click(c: &mut Core, w: &World, p: Vec2) -> Vec<Effect> {
        let mut effects = vec![];
        for event in [
            InputEvent::PointerDown {
                button: Button::Left,
                px: p,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: p,
                over_widget: false,
            },
        ] {
            assert!(intercept(c, &event, w, &VIEW, &mut effects));
        }
        effects
    }
    fn sent(effects: &[Effect]) -> Vec<&OrderEnvelope> {
        effects
            .iter()
            .filter_map(|e| {
                if let Effect::Send(order) = e {
                    Some(order)
                } else {
                    None
                }
            })
            .collect()
    }
    #[test]
    fn reference_needs_current_cell_and_never_sends_an_order() {
        let w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        let p = px(&c, 3.5, 1.5);
        assert!(sent(&click(&mut c, &w, p)).is_empty());
        assert_eq!(c.terrain.reference, None);
        let p = px(&c, 1.5, 1.5);
        assert!(sent(&click(&mut c, &w, p)).is_empty());
        assert_eq!(c.terrain.reference, Some(0));
    }
    #[test]
    fn rectangle_preserves_unknown_cells_and_sends_the_exact_public_bounds() {
        let w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        c.terrain.reference = Some(0);
        let mut e = vec![];
        let a = px(&c, 1.5, 1.5);
        let b = px(&c, 3.5, 2.5);
        intercept(
            &mut c,
            &InputEvent::PointerDown {
                button: Button::Left,
                px: a,
                over_widget: false,
            },
            &w,
            &VIEW,
            &mut e,
        );
        intercept(
            &mut c,
            &InputEvent::PointerMove {
                px: b,
                over_widget: false,
            },
            &w,
            &VIEW,
            &mut e,
        );
        refresh(&mut c, &w, &VIEW, &mut e);
        let preview = e
            .iter()
            .find_map(|e| {
                if let Effect::TerrainWork(o) = e {
                    o.preview.as_ref()
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(preview.cells.len(), 6);
        assert_eq!(preview.cells.iter().filter(|cell| !cell.current).count(), 4);
        intercept(
            &mut c,
            &InputEvent::PointerUp {
                button: Button::Left,
                px: b,
                over_widget: false,
            },
            &w,
            &VIEW,
            &mut e,
        );
        let orders = sent(&e);
        assert_eq!(orders.len(), 1);
        let OrderTarget::TerrainWork(target) = &orders[0].target else {
            panic!()
        };
        assert_eq!(
            target.cells,
            TerrainRect {
                min: TerrainCell { x: 1, y: 1 },
                max: TerrainCell { x: 4, y: 3 }
            }
        );
        assert_eq!(orders[0].queue, QueueMode::Replace);
        assert!(c.terrain.input.is_none());
    }
    #[test]
    fn oversized_rectangle_and_new_context_send_nothing() {
        let mut w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        c.terrain.reference = Some(0);
        let a = px(&c, 0.5, 0.5);
        let b = px(&c, 20.5, 20.5);
        let mut e = vec![];
        intercept(
            &mut c,
            &InputEvent::PointerDown {
                button: Button::Left,
                px: a,
                over_widget: false,
            },
            &w,
            &VIEW,
            &mut e,
        );
        intercept(
            &mut c,
            &InputEvent::PointerUp {
                button: Button::Left,
                px: b,
                over_widget: false,
            },
            &w,
            &VIEW,
            &mut e,
        );
        assert!(sent(&e).is_empty());
        w.epoch = 2;
        refresh(&mut c, &w, &VIEW, &mut e);
        assert!(c.terrain.input.is_none());
        assert!(sent(&e).is_empty());
    }
    #[test]
    fn unload_resume_is_one_appended_utility_not_recreated_work() {
        let w = World::new();
        let mut c = core();
        let input = UnloadResume {
            identity: w.terrain_identity().unwrap(),
            unit: UnitId(1),
            depot: UnitId(2),
            unload_order: "forge:unload".into(),
            work_order: "forge:work".into(),
            designation: w.designation.clone(),
        };
        let mut e = vec![];
        unload_resume(&mut c, &input, &w, &mut e);
        let orders = sent(&e);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].order, "forge:unload");
        assert_eq!(orders[0].queue, QueueMode::Append);
        assert_eq!(orders[0].target, OrderTarget::Unit(UnitId(2)));
        let mut stale = input.clone();
        stale.identity.0 += 1;
        e.clear();
        unload_resume(&mut c, &stale, &w, &mut e);
        assert!(sent(&e).is_empty());
        let mut changed = input;
        changed.designation.target_elevation = 1;
        unload_resume(&mut c, &changed, &w, &mut e);
        assert!(sent(&e).is_empty());
    }
    #[test]
    fn queued_rectangle_stays_modal_and_context_change_rejects_pointer_up() {
        let mut world = World::new();
        let mut core = core();
        enter(&mut core, &world);
        core.terrain.reference = Some(0);
        core.held_modifier_actions
            .insert("test-shift".into(), vec!["queue_modifier".into()]);
        let point = px(&core, 1.5, 1.5);
        let effects = click(&mut core, &world, point);
        assert_eq!(sent(&effects)[0].queue, QueueMode::Append);
        assert!(core.terrain.input.is_some());
        let mut effects = vec![];
        intercept(
            &mut core,
            &InputEvent::PointerDown {
                button: Button::Left,
                px: point,
                over_widget: false,
            },
            &world,
            &VIEW,
            &mut effects,
        );
        world.epoch += 1;
        intercept(
            &mut core,
            &InputEvent::PointerUp {
                button: Button::Left,
                px: point,
                over_widget: false,
            },
            &world,
            &VIEW,
            &mut effects,
        );
        assert!(sent(&effects).is_empty());
        assert!(core.terrain.input.is_none());
    }
    #[test]
    fn raised_surface_projection_and_zoom_anchor_agree_at_both_limits() {
        let mut world = World::new();
        world.height = 5.;
        for delta in [-100_000., 100_000.] {
            let mut core = Core::new(crate::CameraState {
                look_at: Vec2::new(16., 16.),
                height: 20.,
                ..Core::default().camera
            });
            let pointer = Vec2::new(650., 350.);
            let hit = projection::surface_hit(pointer, &VIEW, core.camera, &world).unwrap();
            assert_eq!(hit.height, 5.);
            core.step(
                &[
                    InputEvent::PointerMove {
                        px: pointer,
                        over_widget: false,
                    },
                    InputEvent::Wheel { delta },
                ],
                &world,
                &VIEW,
                0,
            );
            let after = projection::world_to_screen_on(
                hit.position,
                hit.height,
                &VIEW,
                core.camera,
                &world,
            )
            .unwrap();
            assert!(after.distance(pointer) < 0.1, "{delta}: {after:?}");
        }
    }
}
