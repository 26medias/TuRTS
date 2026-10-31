//! Minimap interaction extension point. Task 14 supplies the behavior.

use crate::{Button, Core, Effect, Viewport, WorldView};

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_input(
    _core: &mut Core,
    _button: Button,
    _local: [f32; 2],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
    // Task 14 fills this stable dispatch point.
}
