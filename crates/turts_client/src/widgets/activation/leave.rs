//! Stub activation of task `UI-RESULT` (plan 3.4; design 7.6, row H23). L0-CORE made this file;
//! `UI-RESULT` fills it: the engine action `leave_match` sends `LeaveMatch` when the match is
//! over. `UI-RESULT` adds the one arm in `activate_widget` that calls it. Until then it sends
//! nothing.

use super::*;

#[allow(dead_code)] // the arm that calls it comes with `UI-RESULT`
pub(super) fn activate(
    _queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
}
