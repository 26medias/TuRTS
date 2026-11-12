//! Stub activation of task `UI-ALERT` (plan 5.1 item 5; design 7.9, row H22). L0-CORE made this
//! file; `UI-ALERT` fills it: the activation of `WidgetAction::FocusAlert` sends the core input
//! `InputEvent::Alert(alerts::Input { x, y, unit })` (the camera target and the optional unit that
//! the click also selects). Until then an activation sends nothing.

use super::*;

pub(super) fn activate(
    _x: f32,
    _y: f32,
    _unit: Option<u32>,
    _queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
}
