//! Stub of task `UI-PLACE`: the placement mode and the effect `Ghost { content, point, valid, reason }` (design 7.5, `Core` items).
//!
//! L0-CORE made this file. `Core::step` calls `update` once for each step, after the events of the
//! step ran and before the camera step, in the order `placement`, `hints`, `alerts`. `UI-PLACE`
//! fills the body and adds no call line to `step`. Until then `update` adds no effect and changes
//! no state.

use super::{Core, Effect, InputEvent, Viewport, WorldView};

/// The placement mode hook of `Core::step`. Stub: no effect, no state change.
pub fn update(
    _core: &mut Core,
    _events: &[InputEvent],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}
