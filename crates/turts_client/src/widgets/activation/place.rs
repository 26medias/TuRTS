//! A build recipe starts targeting. The interaction core validates it again.

use super::*;

pub(super) fn activate(
    constructor: u32,
    content: &str,
    queue: &mut CoreInputQueue,
    state: &ReplicaState,
    _acquired: Option<&AcquiredPackages>,
) {
    let (Some(replica), Some(welcome)) = (state.replica.as_ref(), state.welcome) else {
        return;
    };
    if replica.is_site(constructor)
        || !replica
            .units
            .get(&constructor)
            .is_some_and(|unit| unit.owner_slot == welcome.slot)
    {
        return;
    }
    queue.0.push(turts_interaction::InputEvent::Placement(
        turts_interaction::placement::Input {
            constructor,
            content: content.into(),
        },
    ));
}
