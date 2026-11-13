//! Camera focus and remembered positions for authored alerts.
use super::{Core, Effect, InputEvent, Vec2, Viewport, WorldView};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub latest: Option<Input>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Input {
    pub x: f32,
    pub y: f32,
    pub unit: Option<u32>,
}
impl Core {
    pub fn alert_state(&self) -> &State {
        &self.alerts
    }
}

pub fn on_input(
    core: &mut Core,
    input: &Input,
    world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    effects: &mut Vec<Effect>,
) {
    if !input.x.is_finite() || !input.y.is_finite() {
        return;
    }
    core.alerts.latest = Some(input.clone());
    focus(core, input, world, effects);
}

pub(super) fn focus(
    core: &mut Core,
    input: &Input,
    world: &dyn WorldView,
    effects: &mut Vec<Effect>,
) {
    if core
        .camera_controller
        .centre_world(&mut core.camera, Vec2::new(input.x, input.y), world)
    {
        effects.push(Effect::CameraChanged);
    }
    if let Some(unit) = input
        .unit
        .and_then(|id| world.units().iter().find(|unit| unit.id.0 == id))
        && unit.owner_slot == world.own_slot()
    {
        core.selected = vec![unit.id];
        core.selected_node = None;
        effects.push(Effect::SelectionChanged);
    }
}

pub fn update(
    _core: &mut Core,
    _events: &[InputEvent],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}
