//! Stub activation of task `UI2-HUD` (plan 3.4; design 7.5, row H11). L0-CORE made this file;
//! `UI2-HUD` fills it: the rally order of a producer. `UI2-HUD` adds the one arm in
//! `activate_widget` that calls it. Until then it sends nothing.

use super::*;

#[allow(dead_code)] // the arm that calls it comes with `UI2-HUD`
pub(super) fn activate(
    _queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
}
