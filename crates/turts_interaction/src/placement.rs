//! Task `UI-PLACE`: the placement mode and the effect `Ghost { content, point, valid, reason }`
//! (design 7.2, 7.5). L0-CORE made this file and wired every shared edit point once, so `UI-PLACE`
//! edits only this file (and its client files) and adds no line elsewhere in this crate:
//!
//! - `State` is the field `Core::placement` (the placement mode).
//! - `Input` is the payload of `InputEvent::Placement` (a client widget enters the mode).
//! - `Output` is the payload of `Effect::Placement`; the adapter shows it with
//!   `core_adapter/effects/placement.rs` (the ghost).
//! - `Facts` is what `WorldView::placement_facts` gives (sites, footprints and the build area).
//! - `intercept` runs at the top of the event loop of `Core::step`, before any other handler. When it
//!   returns `true`, the event is not given to the handlers of the loop: a right click in the
//!   placement mode sends no order, and a left click that places changes no selection (design 7.2,
//!   GHO1 plant (d)). It may take only: `PointerDown` and `PointerUp` of the left and the right
//!   button, and `KeyDown` of the cancel key. It must not take `KeyUp`, `FocusLost`, `PointerLeft`,
//!   `PointerMove`, `UiCapture`, `Wheel`, `Minimap` or `InputEvent::Placement`: a handler that is
//!   skipped does not clear the held keys and buttons or run `on_input`. When it takes a
//!   `PointerUp`, it clears `pointer_down` and the held button itself (what `on_pointer_up` does
//!   first). The camera still sees every event: `camera_controller.step` reads `events`, not the
//!   loop (`lib.rs`, the call after the second `prune`), except the wheel events that `on_wheel`
//!   marks as ignored, so an intercepted `Wheel` would reach the camera even with Ctrl held.
//! - `on_input` handles `InputEvent::Placement`.
//! - `update` runs once for each step after the events, before the camera step, in the order
//!   `placement`, `hints`, `alerts`.
//!
//! Until `UI-PLACE` builds them, `intercept` returns `false` and no code sets
//! `State::taking_input`; `on_input` and `update` add no effect and change no state. The types
//! hold only what `intercept` needs: `UI-PLACE` adds the rest of its fields here.

use super::{Core, Effect, InputEvent, Viewport, WorldView};

/// The placement mode of the core. `taking_input` is `true` while the mode takes the placement
/// clicks and the cancel key (see `intercept`); it never takes the other kinds of input.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub taking_input: bool,
}

/// An input of the placement mode (an activation of `WidgetAction::Place`). Empty until `UI-PLACE`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Input {}

/// An effect of the placement mode (the ghost). Empty until `UI-PLACE`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {}

/// The world data of the placement rule (design 6.1). Empty until `UI-PLACE`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {}

impl Core {
    /// Read-only observation seam for the placement mode.
    pub fn placement_state(&self) -> &State {
        &self.placement
    }
}

/// The pre-dispatch hook of `Core::step`: `true` when the placement mode took this event.
pub fn intercept(
    core: &mut Core,
    _input_index: usize,
    _event: &InputEvent,
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) -> bool {
    core.placement.taking_input
}

/// The handler of `InputEvent::Placement`. Stub: no effect, no state change.
pub fn on_input(
    _core: &mut Core,
    _input: &Input,
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}

/// The placement hook of `Core::step`. Stub: no effect, no state change.
pub fn update(
    _core: &mut Core,
    _events: &[InputEvent],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}
