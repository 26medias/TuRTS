//! Inactive placement and hint inputs leave the core alone.
//! Alerts have real input behavior and separate semantic tests. Their idle update is passive.

mod common;
use common::*;
use glam::vec2;
use turts_interaction::{Button, Core, Effect, InputEvent, Viewport, WorldView, hints, placement};
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
fn inactive_placement_adds_no_effect_and_changes_no_state() {
    assert_hook_is_a_stub(turts_interaction::placement::update, "placement");
}

#[test]
fn hints_hook_is_empty_until_ui_hint_builds_it() {
    assert_hook_is_a_stub(turts_interaction::hints::update, "hints");
}

#[test]
fn idle_alert_updates_do_not_change_the_core() {
    assert_hook_is_a_stub(turts_interaction::alerts::update, "alerts");
}

/// Inactive placement and hint inputs leave the ordinary input path unchanged.
#[test]
fn inactive_inputs_through_step_add_no_effect_and_change_no_state() {
    let world = TestWorld::default();
    let inputs = [
        InputEvent::Placement(placement::Input::default()),
        InputEvent::Hint(hints::Input::default()),
    ];
    for input in inputs {
        // The same core, once with the owner input and once with no event: the two must agree.
        let mut with_input = Core::default();
        let mut with_nothing = Core::default();
        select_world_point(&mut with_input, &world, UnitId(1), 10);
        select_world_point(&mut with_nothing, &world, UnitId(1), 10);
        let effects = with_input.step(std::slice::from_ref(&input), &world, &VIEW, 50);
        let baseline = with_nothing.step(&[], &world, &VIEW, 50);
        assert_eq!(effects, baseline, "{input:?} must add no effect");
        assert_eq!(
            format!("{with_input:?}"),
            format!("{with_nothing:?}"),
            "{input:?} must not change the state of the core"
        );
    }
}

/// Default placement and hint inputs have no active interaction to change.
#[test]
fn inactive_place_and_hint_handlers_leave_the_core_unchanged() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let before = format!("{core:?}");
    let mut effects = Vec::new();
    placement::on_input(
        &mut core,
        &placement::Input::default(),
        &world,
        &VIEW,
        1,
        &mut effects,
    );
    hints::on_input(
        &mut core,
        &hints::Input::default(),
        &world,
        &VIEW,
        1,
        &mut effects,
    );
    assert!(
        effects.is_empty(),
        "the owner handlers must add no effect: {effects:?}"
    );
    assert_eq!(format!("{core:?}"), before);
}

/// A world gives the owner facts, empty until their owners build them.
#[test]
fn world_gives_empty_owner_facts_by_default() {
    let world = TestWorld::default();
    assert_eq!(world.placement_facts(), &placement::Facts::default());
    assert_eq!(world.hint_facts(), &hints::Facts::default());
}
