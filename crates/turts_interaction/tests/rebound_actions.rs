mod common;
use common::*;
use glam::{Vec2, vec2};
use turts_interaction::{Core, Effect, InputEvent};
use turts_mod_api::{Binding, Button, WheelDirection};
use turts_net_protocol::{OrderTarget, QueueMode, UnitId};

fn key(code: &str) -> Binding {
    Binding::Key {
        code: code.into(),
        mods: vec![],
    }
}
fn pointer(button: Button) -> Binding {
    Binding::Pointer {
        button,
        mods: vec![],
    }
}
fn down(code: &str) -> InputEvent {
    InputEvent::KeyDown {
        code: code.into(),
        mods: vec![],
    }
}
fn at(px: Vec2, over_widget: bool) -> InputEvent {
    InputEvent::PointerMove { px, over_widget }
}
fn select(core: &mut Core, world: &TestWorld, id: u32) {
    select_world_point(core, world, UnitId(id), 0);
}

fn sent(effects: &[Effect]) -> &turts_net_protocol::OrderEnvelope {
    effects
        .iter()
        .find_map(|effect| {
            if let Effect::Send(order) = effect {
                Some(order)
            } else {
                None
            }
        })
        .expect("accepted rebound action must produce a real typed order")
}

#[test]
fn rebound_pointer_authored_move_then_spatial_target_sends_the_real_order() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("key_order_move".into(), vec![pointer(Button::Back)]);
    let mut core = Core::default();
    select(&mut core, &world, 1);
    click(
        &mut core,
        &world,
        Button::Back,
        vec2(640.0, 360.0),
        vec![],
        100,
    );
    let target = core.project_world_to_screen(vec2(20.0, 20.0), &VIEW);
    let effects = click(&mut core, &world, Button::Left, target, vec![], 200);
    assert_eq!(sent(&effects).order, "forge:order_move");
    assert_eq!(sent(&effects).units, vec![UnitId(1)]);
    assert!(matches!(sent(&effects).target, OrderTarget::Point(_)));
}

#[test]
fn rebound_pointer_immediate_stop_sends_the_actual_stop_envelope() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("key_order_stop".into(), vec![pointer(Button::Back)]);
    let mut core = Core::default();
    select(&mut core, &world, 1);
    let effects = click(
        &mut core,
        &world,
        Button::Back,
        vec2(640.0, 360.0),
        vec![],
        100,
    );
    assert_eq!(sent(&effects).order, "forge:order_stop");
    assert_eq!(sent(&effects).target, OrderTarget::None);
    assert_eq!(sent(&effects).queue, QueueMode::Replace);
}

#[test]
fn rebound_key_select_uses_only_the_real_in_world_pointer() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("select".into(), vec![key("KeyP")]);
    let mut core = Core::default();
    let point = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(&[at(point, false), down("KeyP")], &world, &VIEW, 100);
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "the rebound key selects the actual pointed unit"
    );
    core.step(
        &[
            InputEvent::KeyUp {
                code: "KeyP".into(),
            },
            InputEvent::PointerLeft,
            down("KeyP"),
        ],
        &world,
        &VIEW,
        200,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "pointer departure cannot invent another spatial selection"
    );
    let mut guarded = Core::default();
    guarded.step(&[at(point, true), down("KeyP")], &world, &VIEW, 100);
    assert!(
        ids(&guarded).is_empty(),
        "a UI-covered pointer cannot select world units"
    );
}

#[test]
fn rebound_key_box_select_uses_its_actual_press_and_release_endpoints() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("box_select".into(), vec![key("KeyP")]);
    let mut core = Core::default();
    let a = core.project_world_to_screen(world.units[0].position, &VIEW);
    let b = core.project_world_to_screen(world.units[1].position, &VIEW);
    core.step(
        &[
            at(a.min(b) - vec2(3.0, 3.0), false),
            down("KeyP"),
            at(a.max(b) + vec2(3.0, 3.0), false),
            InputEvent::KeyUp {
                code: "KeyP".into(),
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1), UnitId(2)],
        "key-held box must select the same owned mobile units as the pointer gesture"
    );
}

#[test]
fn rebound_key_command_uses_the_actual_pointer_target() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("command".into(), vec![key("KeyP")]);
    let mut core = Core::default();
    select(&mut core, &world, 1);
    let point = core.project_world_to_screen(vec2(20.0, 20.0), &VIEW);
    let effects = core.step(&[at(point, false), down("KeyP")], &world, &VIEW, 100);
    assert_eq!(sent(&effects).order, "forge:order_move");
    let mut blocked = Core::default();
    select(&mut blocked, &world, 1);
    assert!(
        !blocked
            .step(&[at(point, true), down("KeyP")], &world, &VIEW, 100)
            .iter()
            .any(|effect| matches!(effect, Effect::Send(_))),
        "UI capture must block key-bound spatial commands"
    );
}

#[test]
fn rebound_key_zoom_changes_the_real_camera_and_respects_limits() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("zoom_in".into(), vec![key("KeyP")]);
    let mut core = Core::default();
    let before = core.camera_state().height;
    core.step(&[down("KeyP")], &world, &VIEW, 100);
    assert!(
        core.camera_state().height < before,
        "key-bound zoom must change the actual camera"
    );
    for n in 0..100 {
        core.step(
            &[
                InputEvent::KeyUp {
                    code: "KeyP".into(),
                },
                down("KeyP"),
            ],
            &world,
            &VIEW,
            200 + n,
        );
    }
    assert!(core.camera_state().height.is_finite() && core.camera_state().height > 0.0);
}

#[test]
fn rebound_pointer_camera_hold_stops_on_ui_release_and_focus_loss() {
    let mut world = TestWorld::default();
    world.camera_options.edge_scroll = false;
    world.map_size = None;
    world
        .runtime_bindings
        .insert("camera_pan_right".into(), vec![pointer(Button::Back)]);
    let mut core = Core::default();
    core.step(&[], &world, &VIEW, 0);
    core.step(
        &[InputEvent::PointerDown {
            button: Button::Back,
            px: vec2(640.0, 360.0),
            over_widget: false,
        }],
        &world,
        &VIEW,
        1,
    );
    core.step(&[], &world, &VIEW, 1001);
    let moved = core.camera_state().look_at;
    assert!(
        moved.x > 0.0,
        "held pointer binding must pan the actual camera"
    );
    core.step(
        &[InputEvent::PointerUp {
            button: Button::Back,
            px: vec2(640.0, 360.0),
            over_widget: true,
        }],
        &world,
        &VIEW,
        1001,
    );
    core.step(&[], &world, &VIEW, 2001);
    assert_eq!(
        core.camera_state().look_at,
        moved,
        "UI release must clear held pan"
    );
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Back,
                px: vec2(640.0, 360.0),
                over_widget: false,
            },
            InputEvent::FocusLost,
        ],
        &world,
        &VIEW,
        2001,
    );
    core.step(&[], &world, &VIEW, 3001);
    assert_eq!(
        core.camera_state().look_at,
        moved,
        "focus loss must clear held pan"
    );
}

#[test]
fn rebound_pointer_queue_modifier_changes_the_actual_envelope_mode() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("queue_modifier".into(), vec![pointer(Button::Back)]);
    let mut core = Core::default();
    select(&mut core, &world, 1);
    core.step(
        &[
            InputEvent::UiAction {
                name: "forge:order_move".into(),
            },
            InputEvent::PointerDown {
                button: Button::Back,
                px: vec2(640.0, 360.0),
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    let target = core.project_world_to_screen(vec2(20.0, 20.0), &VIEW);
    let effects = click(&mut core, &world, Button::Left, target, vec![], 200);
    assert_eq!(
        sent(&effects).queue,
        QueueMode::Append,
        "held pointer modifier must append the actual order"
    );
}

#[test]
fn swapped_wheel_binding_dispatches_one_real_authored_action_per_event() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("zoom_in".into(), vec![key("KeyP")]);
    world.runtime_bindings.insert(
        "key_order_stop".into(),
        vec![Binding::Wheel(WheelDirection::Up)],
    );
    let mut core = Core::default();
    select(&mut core, &world, 1);
    let effects = core.step(&[InputEvent::Wheel { delta: 8.0 }], &world, &VIEW, 100);
    assert_eq!(sent(&effects).order, "forge:order_stop");
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(effect, Effect::Send(_)))
            .count(),
        1,
        "one actual wheel event must not multiply immediate orders by scroll magnitude"
    );
}

#[test]
fn default_wheel_zoom_applies_exactly_one_moderate_step() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let before = core.camera_state().height;
    core.step(&[InputEvent::Wheel { delta: 1.0 }], &world, &VIEW, 100);
    assert!(
        (core.camera_state().height - before / 1.1).abs() < 0.00001,
        "one wheel event must apply the existing zoom factor exactly once"
    );
}

#[test]
fn ctrl_wheel_is_ignored_even_when_ctrl_releases_in_the_same_batch() {
    for rebound in [false, true] {
        let mut world = TestWorld::default();
        if rebound {
            world
                .runtime_bindings
                .insert("zoom_in".into(), vec![key("KeyP")]);
            world.runtime_bindings.insert(
                "key_order_stop".into(),
                vec![Binding::Wheel(WheelDirection::Up)],
            );
        }
        let mut core = Core::default();
        select(&mut core, &world, 1);
        let before = core.camera_state().height;
        let effects = core.step(
            &[
                InputEvent::KeyDown {
                    code: "ControlLeft".into(),
                    mods: vec![turts_mod_api::Mod::Ctrl],
                },
                InputEvent::Wheel { delta: 1.0 },
                InputEvent::KeyUp {
                    code: "ControlLeft".into(),
                },
            ],
            &world,
            &VIEW,
            100,
        );
        assert_eq!(
            core.camera_state().height,
            before,
            "Ctrl+wheel cannot zoom the engine camera"
        );
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::Send(_))),
            "Ctrl+wheel cannot trigger a rebound authored action"
        );
    }
}

#[test]
fn opening_menu_clears_the_previous_edge_pan_in_the_same_frame() {
    let world = TestWorld {
        map_size: None,
        ..Default::default()
    };
    let mut core = Core::default();
    core.step(&[at(vec2(1279.0, 360.0), false)], &world, &VIEW, 0);
    let before = core.camera_state().look_at;
    core.step(&[down("F10")], &world, &VIEW, 1000);
    assert_eq!(
        core.camera_state().look_at,
        before,
        "the menu-opening frame cannot continue the old edge pan before the next UI marker arrives"
    );
}

#[test]
fn opening_menu_cancels_an_old_key_box_gesture_before_its_release() {
    let mut world = TestWorld::default();
    world
        .runtime_bindings
        .insert("box_select".into(), vec![key("KeyP")]);
    let mut core = Core::default();
    select(&mut core, &world, 1);
    let target = core.project_world_to_screen(vec2(20.0, 20.0), &VIEW);
    core.step(
        &[
            InputEvent::UiAction {
                name: "forge:order_move".into(),
            },
            at(target, false),
            down("KeyP"),
        ],
        &world,
        &VIEW,
        100,
    );
    let effects = core.step(
        &[
            down("F10"),
            InputEvent::KeyUp {
                code: "KeyP".into(),
            },
        ],
        &world,
        &VIEW,
        200,
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Send(_))),
        "release of a box gesture started before the menu must not send a target order through the menu"
    );
}

#[test]
fn either_shift_side_applies_the_default_selection_and_queue_modifiers() {
    for code in ["ShiftLeft", "ShiftRight"] {
        let world = TestWorld {
            map_size: None,
            ..Default::default()
        };
        let mut core = Core::default();
        select(&mut core, &world, 1);
        core.step(
            &[InputEvent::KeyDown {
                code: code.into(),
                mods: vec![turts_mod_api::Mod::Shift],
            }],
            &world,
            &VIEW,
            10,
        );
        let second = core.project_world_to_screen(world.units[1].position, &VIEW);
        click(&mut core, &world, Button::Left, second, vec![], 100);
        assert_eq!(
            ids(&core),
            vec![UnitId(1), UnitId(2)],
            "either physical Shift must add the pointed own unit"
        );
        core.step(
            &[InputEvent::UiAction {
                name: "forge:order_move".into(),
            }],
            &world,
            &VIEW,
            200,
        );
        let target = core.project_world_to_screen(vec2(20.0, 20.0), &VIEW);
        let effects = click(&mut core, &world, Button::Left, target, vec![], 300);
        assert_eq!(
            sent(&effects).queue,
            QueueMode::Append,
            "either physical Shift must queue the real order"
        );
    }
}

#[test]
fn either_control_side_applies_the_default_type_modifier() {
    for code in ["ControlLeft", "ControlRight"] {
        let world = TestWorld {
            map_size: None,
            ..Default::default()
        };
        let mut core = Core::default();
        core.step(
            &[InputEvent::KeyDown {
                code: code.into(),
                mods: vec![turts_mod_api::Mod::Ctrl],
            }],
            &world,
            &VIEW,
            10,
        );
        let point = core.project_world_to_screen(world.units[0].position, &VIEW);
        click(&mut core, &world, Button::Left, point, vec![], 100);
        assert_eq!(
            ids(&core),
            vec![UnitId(1), UnitId(2)],
            "either physical Ctrl must select the pointed owned type"
        );
    }
}

#[test]
fn bare_logical_modifiers_survive_other_held_modifiers_in_both_press_orders() {
    for control_first in [false, true] {
        let world = TestWorld {
            map_size: None,
            ..Default::default()
        };
        let mut core = Core::default();
        select(&mut core, &world, 1);
        let (first, first_mod, second) = if control_first {
            ("ControlLeft", turts_mod_api::Mod::Ctrl, "ShiftRight")
        } else {
            ("ShiftRight", turts_mod_api::Mod::Shift, "ControlLeft")
        };
        core.step(
            &[
                InputEvent::KeyDown {
                    code: first.into(),
                    mods: vec![first_mod],
                },
                InputEvent::KeyDown {
                    code: second.into(),
                    mods: vec![turts_mod_api::Mod::Ctrl, turts_mod_api::Mod::Shift],
                },
                InputEvent::UiAction {
                    name: "forge:order_move".into(),
                },
            ],
            &world,
            &VIEW,
            100,
        );
        let target = core.project_world_to_screen(vec2(20.0, 20.0), &VIEW);
        let effects = click(&mut core, &world, Button::Left, target, vec![], 200);
        assert_eq!(
            sent(&effects).queue,
            QueueMode::Append,
            "Shift's logical queue meaning must survive Ctrl in either press order"
        );
    }
}

#[test]
fn player_modifier_override_replaces_both_default_shift_alternatives() {
    for code in ["ShiftLeft", "ShiftRight", "AltLeft"] {
        let mut world = TestWorld {
            map_size: None,
            ..Default::default()
        };
        world
            .runtime_bindings
            .insert("queue_modifier".into(), vec![key("AltLeft")]);
        let mut core = Core::default();
        select(&mut core, &world, 1);
        let modifier = if code.starts_with("Shift") {
            turts_mod_api::Mod::Shift
        } else {
            turts_mod_api::Mod::Alt
        };
        core.step(
            &[
                InputEvent::KeyDown {
                    code: code.into(),
                    mods: vec![modifier],
                },
                InputEvent::UiAction {
                    name: "forge:order_move".into(),
                },
            ],
            &world,
            &VIEW,
            100,
        );
        let target = core.project_world_to_screen(vec2(20.0, 20.0), &VIEW);
        let effects = click(&mut core, &world, Button::Left, target, vec![], 200);
        assert_eq!(
            sent(&effects).queue,
            if code == "AltLeft" {
                QueueMode::Append
            } else {
                QueueMode::Replace
            },
            "the player override replaces the complete default binding vector"
        );
    }
}

#[test]
fn an_opposite_side_modifier_chord_dispatches_the_exact_authored_action() {
    for (left, right, modifier) in [
        ("ControlLeft", "ControlRight", turts_mod_api::Mod::Ctrl),
        ("ShiftLeft", "ShiftRight", turts_mod_api::Mod::Shift),
        ("AltLeft", "AltRight", turts_mod_api::Mod::Alt),
    ] {
        let mut world = TestWorld {
            map_size: None,
            ..Default::default()
        };
        world.runtime_bindings.insert(
            "key_order_stop".into(),
            vec![Binding::Key {
                code: left.into(),
                mods: vec![modifier],
            }],
        );
        let mut core = Core::default();
        select(&mut core, &world, 1);
        let effects = core.step(
            &[
                InputEvent::KeyDown {
                    code: right.into(),
                    mods: vec![modifier],
                },
                InputEvent::KeyDown {
                    code: left.into(),
                    mods: vec![modifier],
                },
            ],
            &world,
            &VIEW,
            100,
        );
        assert_eq!(
            sent(&effects).order,
            "forge:order_stop",
            "the opposite physical modifier must remain in the accepted chord"
        );
    }
}

fn authored_prefix_order(action: &str) {
    let mut world = TestWorld {
        map_size: None,
        ..Default::default()
    };
    world
        .orders
        .iter_mut()
        .find(|order| order.id == "forge:order_stop")
        .unwrap()
        .hotkey = action.into();
    world.input.actions.push(turts_mod_api::ActionDef {
        name: action.into(),
        default: vec![key("KeyP")],
    });
    let mut core = Core::default();
    select(&mut core, &world, 1);
    let effects = core.step(&[down("KeyP")], &world, &VIEW, 100);
    assert_eq!(
        sent(&effects).order,
        "forge:order_stop",
        "a valid authored action must not be swallowed by an unreserved engine-name prefix"
    );
}
#[test]
fn authored_camera_prefix_is_not_an_engine_pan_action() {
    authored_prefix_order("camera_pan_patrol");
}
#[test]
fn authored_assign_prefix_outside_the_nine_groups_is_not_an_engine_action() {
    authored_prefix_order("group_assign_10");
}
#[test]
fn authored_add_prefix_outside_the_nine_groups_is_not_an_engine_action() {
    authored_prefix_order("group_add_0");
}
#[test]
fn authored_recall_prefix_outside_the_nine_groups_is_not_an_engine_action() {
    authored_prefix_order("group_recall_10");
}

#[test]
fn authored_leading_zero_group_suffix_is_not_an_engine_action() {
    for action in ["group_assign_01", "group_add_09", "group_recall_01"] {
        authored_prefix_order(action);
    }
}

#[test]
fn authored_wheel_pan_is_one_actual_frame_pulse() {
    let mut world = TestWorld {
        map_size: None,
        ..Default::default()
    };
    world.camera_options.edge_scroll = false;
    world
        .input
        .engine_overrides
        .push(turts_mod_api::EngineOverride {
            name: "camera_pan_right".into(),
            bindings: vec![Binding::Wheel(WheelDirection::Up)],
        });
    let mut core = Core::default();
    core.step(&[], &world, &VIEW, 0);
    core.step(&[InputEvent::Wheel { delta: 1.0 }], &world, &VIEW, 100);
    let moved = core.camera_state().look_at;
    assert!(
        moved.x > 0.0,
        "accepted wheel-bound pan must move the actual camera for its input frame"
    );
    core.step(&[], &world, &VIEW, 200);
    assert_eq!(
        core.camera_state().look_at,
        moved,
        "a wheel pulse must not latch held pan into later frames"
    );
}

/// The engine action `toggle_sound` (API 0.2.4, row H25) can be bound to a key by a mod
/// (`engine_overrides`): the key sends the shell action. It has no default key. An unbound
/// `toggle_sound` does nothing.
#[test]
fn a_key_bound_to_toggle_sound_sends_the_shell_action() {
    let mut world = TestWorld::default();
    let mut core = Core::default();
    let none = core.step(&[down("F9")], &world, &VIEW, 50);
    assert!(
        !none
            .iter()
            .any(|effect| matches!(effect, Effect::ShellAction { .. })),
        "F9 is not bound: nothing happens"
    );
    world
        .runtime_bindings
        .insert("toggle_sound".into(), vec![key("F9")]);
    let mut core = Core::default();
    let effects = core.step(&[down("F9")], &world, &VIEW, 100);
    assert!(
        effects.iter().any(
            |effect| matches!(effect, Effect::ShellAction { action } if action == "toggle_sound")
        ),
        "the bound key sends the shell action: {effects:?}"
    );
}
