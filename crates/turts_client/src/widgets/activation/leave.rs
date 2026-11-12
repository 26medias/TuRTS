//! Stub activation of task `UI-RESULT` (plan 3.4; design 7.6, row H23). L0-CORE made this file;
//! `UI-RESULT` fills it: the engine action `leave_match` (a `WidgetAction::Input` with that name)
//! sends `LeaveMatch` when the match is over. `activate_widget` calls it for that name. Until then
//! an activation sends nothing.

use super::*;

pub(super) fn activate(
    _queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
}
