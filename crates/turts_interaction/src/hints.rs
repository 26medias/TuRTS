//! Task `UI-HINT`: the hint state and its evaluation, once for each applied delta and on timers
//! (design 7.5, 7.8). L0-CORE made this file and wired every shared edit point once, so `UI-HINT`
//! edits only this file (and its client files) and adds no line elsewhere in this crate:
//!
//! - `State` is the field `Core::hints`.
//! - `Input` is the payload of `InputEvent::Hint` (what the adapter tells the core about the world,
//!   for example an applied delta). Empty until `UI-HINT` knows what it needs.
//! - `Output` is the payload of `Effect::Hint`; the adapter shows it with
//!   `core_adapter/effects/hint.rs`.
//! - `Facts` is what `WorldView::hint_facts` gives (the conditions: jobs on hold, sites, steps).
//! - `on_input` handles `InputEvent::Hint`.
//! - `update` runs once for each step after the events, before the camera step, in the order
//!   `placement`, `hints`, `alerts`.
//!
//! Until `UI-HINT` builds them, every function adds no effect and changes no state.

use super::{Core, Effect, InputEvent, Viewport, WorldView};

/// The hint state of the core. Empty until `UI-HINT`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {}

/// An input of the hint engine. Empty until `UI-HINT`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Input {}

/// An effect of the hint engine (the hint to show). Empty until `UI-HINT`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {}

/// The world data of the hint conditions. Empty until `UI-HINT`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {}

impl Core {
    /// Read-only observation seam for the hint state.
    pub fn hint_state(&self) -> &State {
        &self.hints
    }
}

/// The handler of `InputEvent::Hint`. Stub: no effect, no state change.
pub fn on_input(
    _core: &mut Core,
    _input: &Input,
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}

/// The hint hook of `Core::step`. Stub: no effect, no state change.
pub fn update(
    _core: &mut Core,
    _events: &[InputEvent],
    _world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    _effects: &mut Vec<Effect>,
) {
}
