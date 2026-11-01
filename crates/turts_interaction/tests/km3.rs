mod common;
use common::*;
use glam::vec2;
use turts_interaction::{Core, InputEvent};
use turts_mod_api::{Binding, Button, EngineOverride, Mod};
use turts_net_protocol::UnitId;

#[test]
fn shift_click_toggles() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "toggle baseline must contain the clicked unit before Shift+click"
    );
    core.step(
        &[InputEvent::KeyDown {
            code: "ShiftLeft".into(),
            mods: vec![Mod::Shift],
        }],
        &world,
        &VIEW,
        20,
    );
    let px = core.project_world_to_screen(world.units[0].position, &VIEW);
    click(&mut core, &world, Button::Left, px, vec![Mod::Shift], 30);
    assert!(
        ids(&core).is_empty(),
        "Shift+click on the selected unit must remove it from the selection"
    );
}

#[test]
fn shift_drag_adds() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let a = core.project_world_to_screen(world.units[0].position, &VIEW);
    let b = core.project_world_to_screen(world.units[1].position, &VIEW);
    core.step(
        &[
            InputEvent::KeyDown {
                code: "ShiftLeft".into(),
                mods: vec![Mod::Shift],
            },
            InputEvent::PointerDown {
                button: Button::Left,
                px: a - vec2(2.0, 2.0),
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: b + vec2(2.0, 2.0),
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: b + vec2(2.0, 2.0),
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        30,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1), UnitId(2)],
        "Shift+drag must add each boxed own unit and retain the earlier selection"
    );
}

#[test]
fn ctrl_click_selects_type_on_screen() {
    let mut world = TestWorld::default();
    world.units.push(unit(
        6,
        "forge:drone",
        OWN,
        vec2(5000.0, 5000.0),
        &[turts_mod_api::Capability::Movement],
    ));
    let mut core = Core::default();
    let px = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(
        &[InputEvent::KeyDown {
            code: "ControlLeft".into(),
            mods: vec![Mod::Ctrl],
        }],
        &world,
        &VIEW,
        10,
    );
    click(&mut core, &world, Button::Left, px, vec![Mod::Ctrl], 20);
    assert_eq!(
        ids(&core),
        vec![UnitId(1), UnitId(2)],
        "Ctrl+click must select matching own units whose projected centres are on-screen, excluding the off-screen drone"
    );
}

#[test]
fn two_clicks_within_400ms_and_4px_double() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let first = core.project_world_to_screen(world.units[0].position, &VIEW);
    click(&mut core, &world, Button::Left, first, vec![], 100);
    click(
        &mut core,
        &world,
        Button::Left,
        first + vec2(4.0, 0.0),
        vec![],
        500,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1), UnitId(2)],
        "a second click at 400 ms and 4 px must select the visible matching type"
    );
}

#[test]
fn double_click_distance_uses_press_points_not_release_jitter() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let center = core.project_world_to_screen(world.units[0].position, &VIEW);
    for (press, release, now) in [
        (center, center + vec2(3.0, 0.0), 100),
        (center + vec2(3.0, 0.0), center, 300),
    ] {
        core.step(
            &[
                InputEvent::PointerDown {
                    button: Button::Left,
                    px: press,
                    over_widget: false,
                },
                InputEvent::PointerUp {
                    button: Button::Left,
                    px: release,
                    over_widget: false,
                },
            ],
            &world,
            &VIEW,
            now,
        );
    }
    assert_eq!(ids(&core), vec![UnitId(1), UnitId(2)]);
}

#[test]
fn two_clicks_500ms_apart_are_two_singles() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let first = core.project_world_to_screen(world.units[0].position, &VIEW);
    click(&mut core, &world, Button::Left, first, vec![], 100);
    click(&mut core, &world, Button::Left, first, vec![], 600);
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "500 ms separates singles at the same point; the second click must not select the matching type"
    );
}

#[test]
fn click_over_widget_is_not_a_ground_click() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let ground = core.project_world_to_screen(vec2(40.0, 40.0), &VIEW);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: ground,
                over_widget: true,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: ground,
                over_widget: true,
            },
        ],
        &world,
        &VIEW,
        20,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "a pointer event marked over a widget must not clear selection as a ground click"
    );
}

#[test]
fn runtime_engine_pointer_overrides_are_used_by_core() {
    let mut world = TestWorld::default();
    world.runtime_bindings.insert(
        "select".into(),
        vec![Binding::Pointer {
            button: Button::Right,
            mods: vec![],
        }],
    );
    world.runtime_bindings.insert(
        "command".into(),
        vec![Binding::Pointer {
            button: Button::Middle,
            mods: vec![],
        }],
    );
    let mut core = Core::default();
    let px = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Right,
                px,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Right,
                px,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        10,
    );
    assert_eq!(ids(&core), vec![UnitId(1)]);
}

#[test]
fn interaction_settings_override_double_click_window() {
    let mut world = TestWorld::default();
    world.interaction.double_click_ms = 100;
    let mut core = Core::default();
    let first = core.project_world_to_screen(world.units[0].position, &VIEW);
    click(&mut core, &world, Button::Left, first, vec![], 100);
    click(&mut core, &world, Button::Left, first, vec![], 201);
    assert_eq!(ids(&core), vec![UnitId(1)]);
}

#[test]
fn double_click_uses_press_times_even_when_first_press_is_held() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let point = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(
        &[InputEvent::PointerDown {
            button: Button::Left,
            px: point,
            over_widget: false,
        }],
        &world,
        &VIEW,
        0,
    );
    core.step(
        &[InputEvent::PointerUp {
            button: Button::Left,
            px: point,
            over_widget: false,
        }],
        &world,
        &VIEW,
        1000,
    );
    core.step(
        &[InputEvent::PointerDown {
            button: Button::Left,
            px: point,
            over_widget: false,
        }],
        &world,
        &VIEW,
        1100,
    );
    core.step(
        &[InputEvent::PointerUp {
            button: Button::Left,
            px: point,
            over_widget: false,
        }],
        &world,
        &VIEW,
        1101,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "presses more than 400 ms apart remain singles even if release events are close"
    );
}

#[test]
fn exact_pointer_chord_precedes_default_unmodified_action() {
    let mut world = TestWorld::default();
    world.input.engine_overrides.push(EngineOverride {
        name: "command".into(),
        bindings: vec![Binding::Pointer {
            button: Button::Left,
            mods: vec![Mod::Shift],
        }],
    });
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let second = core.project_world_to_screen(world.units[1].position, &VIEW);
    let effects = core.step(
        &[
            InputEvent::KeyDown {
                code: "ShiftLeft".into(),
                mods: vec![Mod::Shift],
            },
            InputEvent::PointerDown {
                button: Button::Left,
                px: second,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: second,
                over_widget: false,
            },
            InputEvent::KeyUp {
                code: "ShiftLeft".into(),
            },
        ],
        &world,
        &VIEW,
        20,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "an exact Shift+Left command chord takes priority over default Left selection"
    );
    assert!(
        !sent(&effects).is_empty(),
        "the exact pointer command binding must execute"
    );

    click(&mut core, &world, Button::Left, second, vec![], 30);
    assert_eq!(
        ids(&core),
        vec![UnitId(2)],
        "plain Left remains the default selection action"
    );
}

#[test]
fn two_clicks_more_than_4px_apart_are_two_singles() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let first = core.project_world_to_screen(world.units[0].position, &VIEW);
    click(&mut core, &world, Button::Left, first, vec![], 100);
    click(
        &mut core,
        &world,
        Button::Left,
        first + vec2(5.0, 0.0),
        vec![],
        200,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "5 px separates single clicks even within 400 ms"
    );
}
