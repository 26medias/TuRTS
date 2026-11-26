//! Area input for ordinary queued terrain orders (operator and Iris, 2026-11-26): arm a tool, drag over the ground,
//! release. The level of the order is the ground height where the drag starts, like a level tool.
use crate::{Button, Core, Effect, InputEvent, Vec2, Viewport, WorldView, projection};
use turts_mod_api::{
    Behaviour, ContentId, ResolvedTerrainOperation, TargetKind, TerrainCell, TerrainDesignation,
    TerrainRect,
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
    start: Option<TerrainCell>,
    /// Why the last drag was refused; shown until the next press.
    refused: Option<&'static str>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CellPreview {
    pub cell: TerrainCell,
    pub elevation: i32,
    pub current: bool,
    /// The order moves ground in this cell: the cell is dug or filled.
    pub changes: bool,
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
/// A refused drag keeps the tool armed; the reason shows with the hint and goes to the notices.
fn refuse(core: &mut Core, reason: &'static str, effects: &mut Vec<Effect>) {
    core.terrain.refused = Some(reason);
    effects.push(Effect::Notice {
        text_key: reason.into(),
        units: vec![],
    });
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
fn cells_of(rect: TerrainRect) -> impl Iterator<Item = TerrainCell> {
    (rect.min.y..rect.max.y)
        .flat_map(move |y| (rect.min.x..rect.max.x).map(move |x| TerrainCell { x, y }))
}
fn area(rect: TerrainRect) -> u64 {
    u64::from(rect.max.x - rect.min.x) * u64::from(rect.max.y - rect.min.y)
}
/// The operation of the armed tool, from the first selected unit that can use it.
fn operation(
    core: &Core,
    input: &Input,
    world: &dyn WorldView,
) -> Option<ResolvedTerrainOperation> {
    let id = *eligible(core, input, world).first()?;
    let unit = world.units().iter().find(|unit| unit.id == id)?;
    Some(world.terrain_tool(unit, &input.tool)?.operation.clone())
}
/// True when an observed cell at `elevation` lies on the side of `level` that the operation moves.
fn moves(operation: &ResolvedTerrainOperation, elevation: i32, level: i32) -> bool {
    match operation {
        ResolvedTerrainOperation::Excavate => elevation > level,
        ResolvedTerrainOperation::Fill { .. } => elevation < level,
    }
}
/// True when a cell of the area is not observed (it can need work) or lies on the wrong side of the level.
fn has_work(
    rect: TerrainRect,
    level: i32,
    operation: &ResolvedTerrainOperation,
    world: &dyn WorldView,
) -> bool {
    cells_of(rect).any(|cell| {
        !world.terrain_current(cell)
            || world
                .terrain_elevation(cell)
                .is_some_and(|elevation| moves(operation, elevation, level))
    })
}
/// The level of an area order that starts at `start`, or why the area cannot be ordered, in the words that the player
/// reads. The level is the observed ground height of the start cell.
fn refusal(
    start: TerrainCell,
    rect: TerrainRect,
    operation: &ResolvedTerrainOperation,
    world: &dyn WorldView,
) -> Result<i32, &'static str> {
    if area(rect) > 256 {
        return Err("A work rectangle can contain at most 256 cells");
    }
    let level = world
        .terrain_elevation(start)
        .filter(|_| world.terrain_current(start))
        .ok_or("You cannot see this ground yet. Start the drag on ground that you can see.")?;
    if !has_work(rect, level, operation, world) {
        return Err(match operation {
            ResolvedTerrainOperation::Excavate => {
                "Nothing to dig above the height where you started. Start the drag on lower ground."
            }
            ResolvedTerrainOperation::Fill { .. } => {
                "Nothing to fill below the height where you started. Start the drag on higher ground."
            }
        });
    }
    Ok(level)
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
        target_elevation: None,
        cells: vec![],
        rectangle: None,
        can_submit: false,
        reason: None,
    };
    let cell_preview = |cell: TerrainCell| {
        Some(CellPreview {
            cell,
            elevation: world.terrain_elevation(cell)?,
            current: world.terrain_current(cell),
            changes: false,
        })
    };
    match (core.terrain.start, pointer.and_then(|hit| hit.cell)) {
        (Some(start), Some(end)) => {
            if let Some(rect) = rectangle(start, end, world) {
                preview.rectangle = Some(rect);
                if let Some(operation) = operation(core, &preview.input, world) {
                    match refusal(start, rect, &operation, world) {
                        Ok(level) => preview.target_elevation = Some(level),
                        Err(reason) => preview.reason = Some(reason.into()),
                    }
                }
                // Enumeration depends only on the submitted public rectangle.
                if area(rect) <= 256 {
                    preview.cells = cells_of(rect).filter_map(cell_preview).collect();
                    if preview.cells.len() as u64 != area(rect) {
                        preview.reason = Some("Terrain geometry unavailable".into());
                    } else {
                        preview.can_submit = preview.target_elevation.is_some();
                    }
                    if let (Some(level), Some(operation)) = (
                        preview.target_elevation,
                        operation(core, &preview.input, world),
                    ) {
                        for cell in &mut preview.cells {
                            cell.changes = cell.current && moves(&operation, cell.elevation, level);
                        }
                    }
                }
            }
        }
        _ => {
            preview.reason = core.terrain.refused.map(Into::into);
            preview
                .cells
                .extend(pointer.and_then(|hit| hit.cell).and_then(cell_preview));
        }
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
            core.terrain.start = hit(*px, core, world, view).and_then(|hit| hit.cell);
            core.terrain.refused = None;
            true
        }
        InputEvent::PointerUp {
            button,
            px,
            over_widget,
        } if *button == Button::Left => {
            core.pointer_down = None;
            core.held_buttons.retain(|held| held != button);
            core.held_modifier_actions
                .remove(&format!("pointer:{button:?}"));
            let start = core.terrain.start.take();
            if *over_widget || core.ui_capture.is_some() {
                return true;
            }
            let rect = start
                .zip(hit(*px, core, world, view).and_then(|hit| hit.cell))
                .and_then(|(start, end)| rectangle(start, end, world));
            let (Some(cells), Some(input)) = (rect, core.terrain.input.clone()) else {
                refuse(core, "Drag over the ground to choose the area.", effects);
                return true;
            };
            let units = eligible(core, &input, world);
            let Some(operation) = operation(core, &input, world) else {
                return true;
            };
            let Some(start) = start else {
                return true;
            };
            let level = match refusal(start, cells, &operation, world) {
                Ok(level) => level,
                Err(reason) => {
                    refuse(core, reason, effects);
                    return true;
                }
            };
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
                    target_elevation: level,
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
        /// Cells with x in this range stand 4 levels high (a ridge).
        ridge: std::ops::Range<u32>,
        observed_columns: u32,
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
                ridge: 10..14,
                observed_columns: 20,
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
            cell.x < self.observed_columns
        }
        fn terrain_elevation(&self, cell: TerrainCell) -> Option<i32> {
            Some(if self.ridge.contains(&cell.x) { 4 } else { 0 })
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
        Core {
            selected: vec![UnitId(1)],
            ..Default::default()
        }
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
    fn drag(c: &mut Core, w: &World, from: (f32, f32), to: (f32, f32)) -> Vec<Effect> {
        let (a, b) = (px(c, from.0, from.1), px(c, to.0, to.1));
        let mut effects = vec![];
        for event in [
            InputEvent::PointerDown {
                button: Button::Left,
                px: a,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: b,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: b,
                over_widget: false,
            },
        ] {
            assert!(intercept(c, &event, w, &VIEW, &mut effects));
        }
        effects
    }
    fn preview(effects: &[Effect]) -> &Preview {
        effects
            .iter()
            .rev()
            .find_map(|e| match e {
                Effect::TerrainWork(o) => o.preview.as_ref(),
                _ => None,
            })
            .unwrap()
    }
    fn target(order: &OrderEnvelope) -> &TerrainDesignation {
        let OrderTarget::TerrainWork(target) = &order.target else {
            panic!("not terrain work")
        };
        target
    }
    fn notices(effects: &[Effect]) -> Vec<&str> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::Notice { text_key, .. } => Some(text_key.as_str()),
                _ => None,
            })
            .collect()
    }
    fn rect(min: (u32, u32), max: (u32, u32)) -> TerrainRect {
        TerrainRect {
            min: TerrainCell { x: min.0, y: min.1 },
            max: TerrainCell { x: max.0, y: max.1 },
        }
    }
    #[test]
    fn the_first_drag_after_the_tool_orders_the_area_at_the_height_where_it_starts() {
        let w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        // From the foot of the ridge (x 9, level 0) over the ridge.
        let effects = drag(&mut c, &w, (9.5, 3.5), (13.5, 5.5));
        let orders = sent(&effects);
        assert_eq!(orders.len(), 1, "{effects:?}");
        assert_eq!(target(orders[0]).cells, rect((9, 3), (14, 6)));
        assert_eq!(target(orders[0]).target_elevation, 0);
        assert_eq!(orders[0].queue, QueueMode::Replace);
        assert!(c.terrain.input.is_none(), "one order leaves the tool");
    }
    #[test]
    fn a_dig_that_starts_on_the_ridge_has_nothing_above_its_height_and_says_so() {
        let w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        let effects = drag(&mut c, &w, (11.5, 3.5), (8.5, 3.5));
        assert!(sent(&effects).is_empty());
        assert_eq!(
            notices(&effects),
            ["Nothing to dig above the height where you started. Start the drag on lower ground."]
        );
        assert!(c.terrain.input.is_some(), "the tool stays armed");
        let mut e = vec![];
        refresh(&mut c, &w, &VIEW, &mut e);
        assert_eq!(
            preview(&e).reason.as_deref(),
            Some(
                "Nothing to dig above the height where you started. Start the drag on lower ground."
            )
        );
        // The next drag, from the foot of the ridge over it, clears the reason.
        let (a, b) = (px(&c, 9.5, 3.5), px(&c, 10.5, 3.5));
        for event in [
            InputEvent::PointerDown {
                button: Button::Left,
                px: a,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: b,
                over_widget: false,
            },
        ] {
            intercept(&mut c, &event, &w, &VIEW, &mut e);
        }
        e.clear();
        refresh(&mut c, &w, &VIEW, &mut e);
        assert_eq!(preview(&e).reason.as_deref(), None);
    }
    #[test]
    fn a_fill_that_starts_on_the_ridge_goes_up_to_the_ridge_height() {
        let mut w = World::new();
        w.tool.operation = ResolvedTerrainOperation::Fill {
            material: ContentId::parse("forge:loam").unwrap(),
        };
        let mut c = core();
        enter(&mut c, &w);
        // From the ridge top (x 13, level 4) down onto the flat ground (x 14).
        let effects = drag(&mut c, &w, (13.5, 3.5), (14.5, 4.5));
        assert_eq!(target(sent(&effects)[0]).target_elevation, 4);
    }
    #[test]
    fn the_preview_during_a_drag_shows_the_area_and_its_level() {
        let w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        let (a, b) = (px(&c, 9.5, 3.5), px(&c, 10.5, 4.5));
        let mut e = vec![];
        for event in [
            InputEvent::PointerDown {
                button: Button::Left,
                px: a,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: b,
                over_widget: false,
            },
        ] {
            intercept(&mut c, &event, &w, &VIEW, &mut e);
        }
        refresh(&mut c, &w, &VIEW, &mut e);
        let preview = preview(&e);
        assert_eq!(preview.rectangle, Some(rect((9, 3), (11, 5))));
        assert_eq!(preview.cells.len(), 4);
        let changing = preview
            .cells
            .iter()
            .filter(|cell| cell.changes)
            .map(|cell| cell.cell.x)
            .collect::<Vec<_>>();
        assert_eq!(changing, [10, 10], "only the ridge cells are dug");
        assert_eq!(preview.target_elevation, Some(0));
        assert!(preview.can_submit);
    }
    #[test]
    fn a_hover_before_the_drag_shows_the_cell_and_no_level() {
        let w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        let mut e = vec![];
        let p = px(&c, 11.5, 3.5);
        intercept(
            &mut c,
            &InputEvent::PointerMove {
                px: p,
                over_widget: false,
            },
            &w,
            &VIEW,
            &mut e,
        );
        refresh(&mut c, &w, &VIEW, &mut e);
        let preview = preview(&e);
        assert_eq!(preview.cells.len(), 1);
        assert_eq!(preview.target_elevation, None);
        assert_eq!(preview.reason, None);
    }
    #[test]
    fn unobserved_cells_stay_in_the_order_and_an_unseen_start_is_refused() {
        let mut w = World::new();
        w.observed_columns = 11;
        let mut c = core();
        enter(&mut c, &w);
        let effects = drag(&mut c, &w, (9.5, 3.5), (12.5, 3.5));
        assert_eq!(target(sent(&effects)[0]).cells, rect((9, 3), (13, 4)));
        w.observed_columns = 0;
        let mut c = core();
        enter(&mut c, &w);
        let effects = drag(&mut c, &w, (9.5, 3.5), (11.5, 3.5));
        assert!(sent(&effects).is_empty());
        assert_eq!(
            notices(&effects),
            ["You cannot see this ground yet. Start the drag on ground that you can see."]
        );
    }
    #[test]
    fn right_click_leaves_the_tool_and_sends_nothing() {
        let w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        let mut e = vec![];
        let p = px(&c, 10.5, 3.5);
        assert!(intercept(
            &mut c,
            &InputEvent::PointerDown {
                button: Button::Right,
                px: p,
                over_widget: false,
            },
            &w,
            &VIEW,
            &mut e,
        ));
        assert!(sent(&e).is_empty());
        assert!(c.terrain.input.is_none());
    }
    #[test]
    fn oversized_rectangle_and_new_context_send_nothing() {
        let mut w = World::new();
        let mut c = core();
        enter(&mut c, &w);
        let effects = drag(&mut c, &w, (0.5, 0.5), (20.5, 20.5));
        assert!(sent(&effects).is_empty());
        assert_eq!(
            notices(&effects),
            ["A work rectangle can contain at most 256 cells"]
        );
        w.epoch = 2;
        let mut e = vec![];
        refresh(&mut c, &w, &VIEW, &mut e);
        assert!(c.terrain.input.is_none());
        assert!(sent(&e).is_empty());
    }
    #[test]
    fn queued_rectangle_stays_modal_and_context_change_rejects_pointer_up() {
        let mut world = World::new();
        let mut core = core();
        enter(&mut core, &world);
        core.held_modifier_actions
            .insert("test-shift".into(), vec!["queue_modifier".into()]);
        let effects = drag(&mut core, &world, (9.5, 1.5), (10.5, 2.5));
        assert_eq!(sent(&effects)[0].queue, QueueMode::Append);
        assert!(core.terrain.input.is_some());
        let point = px(&core, 10.5, 1.5);
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
