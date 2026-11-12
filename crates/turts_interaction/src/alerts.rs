//! Stub of task `UI-ALERT`: the core action `FocusAlert` (design 7.5, row H22).
//!
//! L0-CORE made this file. `Core::step` calls `update` once for each step, after the events of the
//! step ran and before the camera step, in the order `placement`, `hints`, `alerts`. `UI-ALERT`
//! fills the body and adds no call line to `step`. Until then `update` adds no effect and changes
//! no state.

use super::{Core, Effect, InputEvent, Viewport, WorldView};

/// The the alert focus hook of `Core::step`. Stub: no effect, no state change.
pub fn update(
    _core: &mut Core,
    _events: &[InputEvent],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}
