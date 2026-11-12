//! Stub of task `UI-HINT`: the hint state and its evaluation, once for each applied delta and on timers (design 7.8).
//!
//! L0-CORE made this file. `Core::step` calls `update` once for each step, after the events of the
//! step ran and before the camera step, in the order `placement`, `hints`, `alerts`. `UI-HINT`
//! fills the body and adds no call line to `step`. Until then `update` adds no effect and changes
//! no state.

use super::{Core, Effect, InputEvent, Viewport, WorldView};

/// The the hint state hook of `Core::step`. Stub: no effect, no state change.
pub fn update(
    _core: &mut Core,
    _events: &[InputEvent],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}
