//! L0-CORE: the stubs of `Core::step` hooks (`placement`, `hints`, `alerts`). Each hook is empty
//! until its owner task builds it: it adds no effect and changes no state of the core. The owner
//! task replaces its test here with its own (the contract of the stub ends with the stub).

mod common;
use common::*;
use glam::vec2;
use turts_interaction::{Button, Core, Effect, InputEvent, Viewport, WorldView};
use turts_net_protocol::UnitId;

type Hook = fn(&mut Core, &[InputEvent], &dyn WorldView, &Viewport, u64, &mut Vec<Effect>);

fn assert_hook_is_a_stub(hook: Hook, name: &str) {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let events = [
        InputEvent::PointerMove {
            px: vec2(300.0, 200.0),
            over_widget: false,
        },
        InputEvent::PointerDown {
            button: Button::Left,
            px: vec2(300.0, 200.0),
            over_widget: false,
        },
    ];
    let before = format!("{core:?}");
    let mut effects = Vec::new();
    hook(&mut core, &events, &world, &VIEW, 40, &mut effects);
    assert!(
        effects.is_empty(),
        "{name}::update must add no effect: {effects:?}"
    );
    assert_eq!(
        format!("{core:?}"),
        before,
        "{name}::update must not change the state of the core"
    );
}

#[test]
fn placement_hook_is_empty_until_ui_place_builds_it() {
    assert_hook_is_a_stub(turts_interaction::placement::update, "placement");
}

#[test]
fn hints_hook_is_empty_until_ui_hint_builds_it() {
    assert_hook_is_a_stub(turts_interaction::hints::update, "hints");
}

#[test]
fn alerts_hook_is_empty_until_ui_alert_builds_it() {
    assert_hook_is_a_stub(turts_interaction::alerts::update, "alerts");
}
