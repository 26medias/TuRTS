#[allow(dead_code)]
mod common;

use common::{TestWorld, VIEW, ids, sent};
use glam::vec2;
use turts_interaction::{CameraState, Core, Effect, InputEvent};
use turts_mod_api::Button;
use turts_net_protocol::{OrderTarget, Pos, QueueMode, UnitId};

fn camera() -> CameraState {
    CameraState {
        look_at: vec2(32.0, 32.0),
        height: 40.0,
        pitch: 0.7,
        yaw: 0.0,
        vertical_fov: 0.9,
    }
}

#[test]
fn left_click_moves_look_at_to_the_world_point_under_the_minimap_cursor() {
    let world = TestWorld {
        map_size: Some(vec2(80.0, 40.0)),
        ..Default::default()
    };
    let mut core = Core::new(camera());

    let effects = core.step(
        &[InputEvent::Minimap {
            button: Button::Left,
            local: [0.75, 0.25],
        }],
        &world,
        &VIEW,
        10,
    );

    assert_eq!(core.camera_state().look_at, vec2(60.0, 30.0));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::CameraChanged))
    );
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, Effect::Send(_)))
    );
}

#[test]
fn drag_moves_look_at_with_each_minimap_pointer_position() {
    let world = TestWorld {
        map_size: Some(vec2(80.0, 40.0)),
        ..Default::default()
    };
    let mut core = Core::new(camera());

    for (now_ms, local, expected) in [
        (10, [0.25, 0.75], vec2(20.0, 10.0)),
        (20, [0.75, 0.25], vec2(60.0, 30.0)),
    ] {
        core.step(
            &[InputEvent::Minimap {
                button: Button::Left,
                local,
            }],
            &world,
            &VIEW,
            now_ms,
        );
        assert_eq!(core.camera_state().look_at, expected);
    }
}

#[test]
fn right_click_orders_at_the_minimap_world_point_without_moving_the_camera() {
    let world = TestWorld {
        map_size: Some(vec2(64.0, 48.0)),
        ..Default::default()
    };
    let mut core = Core::new(camera());
    let selection = core.step(
        &[InputEvent::UiAction {
            name: "ui_select:1".into(),
        }],
        &world,
        &VIEW,
        1,
    );
    assert!(
        selection
            .iter()
            .any(|effect| matches!(effect, Effect::SelectionChanged))
    );
    assert_eq!(ids(&core), vec![UnitId(1)]);
    let before = core.camera_state();

    let effects = core.step(
        &[InputEvent::Minimap {
            button: Button::Right,
            local: [0.75, 0.25],
        }],
        &world,
        &VIEW,
        10,
    );

    let orders = sent(&effects);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].order, "forge:order_move");
    assert_eq!(orders[0].units, vec![UnitId(1)]);
    assert_eq!(orders[0].queue, QueueMode::Replace);
    assert_eq!(
        orders[0].target,
        OrderTarget::Point(Pos { x: 48.0, y: 36.0 })
    );
    assert_eq!(core.camera_state(), before);
}

#[test]
fn right_click_cancels_explicit_target_mode_without_sending_an_order() {
    let world = TestWorld::default();
    let mut core = Core::new(camera());
    let selection = core.step(
        &[InputEvent::UiAction {
            name: "ui_select:1".into(),
        }],
        &world,
        &VIEW,
        1,
    );
    assert!(
        selection
            .iter()
            .any(|effect| matches!(effect, Effect::SelectionChanged))
    );

    let target_mode = core.step(
        &[InputEvent::UiAction {
            name: "forge:order_attack".into(),
        }],
        &world,
        &VIEW,
        2,
    );
    assert_eq!(core.pending_order_id(), Some("forge:order_attack"));
    assert!(
        target_mode
            .iter()
            .any(|effect| matches!(effect, Effect::Cursor { context } if context == "target"))
    );

    let effects = core.step(
        &[InputEvent::Minimap {
            button: Button::Right,
            local: [0.25, 0.75],
        }],
        &world,
        &VIEW,
        3,
    );

    assert_eq!(core.pending_order_id(), None);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Cursor { context } if context == "default"))
    );
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, Effect::Send(_)))
    );

    let smart = core.step(
        &[InputEvent::Minimap {
            button: Button::Right,
            local: [0.75, 0.75],
        }],
        &world,
        &VIEW,
        4,
    );
    assert_eq!(core.pending_order_id(), None);
    assert_eq!(
        sent(&smart).len(),
        1,
        "a later ordinary minimap right click must resume smart orders"
    );
}

#[test]
fn unsupported_minimap_buttons_and_nonfinite_coordinates_do_nothing() {
    let world = TestWorld::default();
    let mut core = Core::new(camera());
    let before = core.camera_state();

    let middle = core.step(
        &[InputEvent::Minimap {
            button: Button::Middle,
            local: [0.25, 0.25],
        }],
        &world,
        &VIEW,
        10,
    );
    let nonfinite = core.step(
        &[InputEvent::Minimap {
            button: Button::Left,
            local: [f32::NAN, 0.5],
        }],
        &world,
        &VIEW,
        20,
    );

    assert_eq!(core.camera_state(), before);
    assert!(middle.is_empty());
    assert!(nonfinite.is_empty());
}
