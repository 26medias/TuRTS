//! Stub activation of task `UI-HUD` (plan 3.4; design 7.4, open point 16). L0-CORE made this file;
//! `UI-HUD` fills it: the right-click cancel of a queue row and of a queue entry. `UI-HUD` adds the
//! one arm in `activate_widget` that calls it. Until then it sends nothing.

use super::*;

#[allow(dead_code)] // the arm that calls it comes with `UI-HUD`
pub(super) fn activate(
    _queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
}
