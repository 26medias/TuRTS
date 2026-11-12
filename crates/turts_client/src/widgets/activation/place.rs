//! Stub activation of task `UI-PLACE` (plan 3.4; design 7.2). L0-CORE made this file; `UI-PLACE`
//! fills it: the activation of `WidgetAction::Place` enters the placement mode of the interaction
//! core. Until then an activation sends nothing.

use super::*;

pub(super) fn activate(
    _constructor: u32,
    _content: &str,
    _queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
}
