//! L0-CORE: the contract of the pre-dispatch hook `placement::intercept` (M1 of the review).
//! The test world is the one of the integration tests (`tests/common/mod.rs`).

use crate::test_common::*;
use crate::{Core, InputEvent};
use turts_mod_api::Button;
use turts_net_protocol::UnitId;

/// With the hook taking the input, a right click sends no order and a left click changes no
/// selection (design 7.2, GHO1 plant (d)). The same events with the real stub (`false`) send the
/// order and change the selection, so the test can tell the two apart.
#[test]
fn intercept_that_takes_the_input_sends_no_order_and_changes_no_selection() {
    let world = TestWorld::default();
    for taking_input in [false, true] {
        let mut core = Core::default();
        select_world_point(&mut core, &world, UnitId(1), 10);
        assert_eq!(
            ids(&core),
            vec![UnitId(1)],
            "the setup click must select unit 1 while the hook takes nothing"
        );
        core.placement.taking_input = taking_input;
        let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
        let order_effects = click(&mut core, &world, Button::Right, foreign, vec![], 20);
        let second = core.project_world_to_screen(world.units[1].position, &VIEW);
        click(&mut core, &world, Button::Left, second, vec![], 30);
        let sent_orders = sent(&order_effects).len();
        if taking_input {
            assert_eq!(
                sent_orders, 0,
                "a right click in the placement mode must send no order"
            );
            assert_eq!(
                ids(&core),
                vec![UnitId(1)],
                "a left click in the placement mode must not change the selection"
            );
        } else {
            assert!(
                sent_orders > 0,
                "control: the right click sends an order: {order_effects:?}"
            );
            assert_eq!(
                ids(&core),
                vec![UnitId(2)],
                "control: the left click selects the clicked unit"
            );
        }
    }
}

/// The real stub: nothing sets `taking_input`, so the hook never takes an event.
#[test]
fn intercept_stub_takes_no_event_in_a_fresh_core() {
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
