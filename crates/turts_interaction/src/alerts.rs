//! Task `UI-ALERT`: the core action `FocusAlert` (design 7.5, 7.9, row H22). L0-CORE made this file
//! and wired every shared edit point once, so `UI-ALERT` edits only this file (and its client
//! files) and adds no line elsewhere in this crate:
//!
//! - `State` is the field `Core::alerts`.
//! - `Input` is the payload of `InputEvent::Alert`: the action `FocusAlert` with its place and its
//!   optional unit (design 7.9: a click moves the camera to the place; "Ore Hauler idle" also
//!   selects the hauler). A headless agent sends the same input (D74).
//! - `on_input` handles `InputEvent::Alert`. The result is the existing effects `CameraChanged`
//!   and `SelectionChanged`: this task has no effect type of its own.
//! - `update` runs once for each step after the events, before the camera step, in the order
//!   `placement`, `hints`, `alerts`.
//!
//! Until `UI-ALERT` builds them, every function adds no effect and changes no state.

use super::{Core, Effect, InputEvent, Viewport, WorldView};

/// The alert state of the core. Empty until `UI-ALERT`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {}

/// The core action `FocusAlert`: the place of the alert and the unit that the click also selects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Input {
    pub x: f32,
    pub y: f32,
    pub unit: Option<u32>,
}

impl Core {
    /// Read-only observation seam for the alert state.
    pub fn alert_state(&self) -> &State {
        &self.alerts
    }
}

/// The handler of `InputEvent::Alert`. Stub: no effect, no state change.
pub fn on_input(
    _core: &mut Core,
    _input: &Input,
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}

/// The alert hook of `Core::step`. Stub: no effect, no state change.
pub fn update(
    _core: &mut Core,
    _events: &[InputEvent],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}
