//! L0-CORE: the contract of the pre-dispatch hook `placement::intercept` (M1 of the review).
//! The test world is the one of the integration tests (`tests/common/mod.rs`).

use crate::test_common::*;
use crate::{Core, InputEvent};

// Active placement, cancellation, and selection are tested with real inputs in tests/placement.rs.

/// A fresh core leaves normal input available.
#[test]
fn placement_takes_no_event_in_a_fresh_core() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let mut effects = Vec::new();
    let event = InputEvent::PointerLeft;
    assert!(!crate::placement::intercept(
        &mut core,
        0,
        &event,
        &world,
        &VIEW,
        0,
        &mut effects
    ));
    assert!(effects.is_empty());
}
