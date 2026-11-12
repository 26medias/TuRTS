//! Stub activation of task `UI-ALERT` (plan 5.1 item 5; design 7.5, row H22). L0-CORE made this
//! file; `UI-ALERT` fills it: the activation of `WidgetAction::FocusAlert` sends the core action
//! `FocusAlert`. Until then an activation sends nothing.

use super::*;

pub(super) fn activate(
    _queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
}
