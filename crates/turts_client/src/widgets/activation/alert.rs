//! Send an authored alert activation through the shared interaction core.
use super::*;
pub(super) fn activate(
    x: f32,
    y: f32,
    unit: Option<u32>,
    queue: &mut CoreInputQueue,
    _state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
    if x.is_finite() && y.is_finite() {
        queue.0.push(turts_interaction::InputEvent::Alert(
            turts_interaction::alerts::Input { x, y, unit },
        ));
    }
}
