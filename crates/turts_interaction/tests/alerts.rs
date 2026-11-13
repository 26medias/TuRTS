#[allow(dead_code)]
mod common;
use common::{TestWorld, VIEW};
use glam::vec2;
use turts_interaction::{CameraState, Core, Effect, InputEvent, alerts::Input};
use turts_net_protocol::UnitId;

#[test]
fn announced_warning_does_not_move_camera_and_jump_reaches_it() {
    let world = TestWorld::default();
    let mut core = Core::new(CameraState {
        look_at: vec2(0., 0.),
        ..Default::default()
    });
    let alert = Input {
        x: 20.,
        y: 15.,
        unit: None,
    };
    core.step(&[InputEvent::AlertAnnounced(alert)], &world, &VIEW, 0);
    assert_eq!(core.camera_state().look_at, vec2(0., 0.));
    let effects = core.step(
        &[InputEvent::UiAction {
            name: "jump_to_alert".into(),
        }],
        &world,
        &VIEW,
        1,
    );
    assert_eq!(core.camera_state().look_at, vec2(20., 15.));
    assert!(effects.iter().any(|e| matches!(e, Effect::CameraChanged)));
}
#[test]
fn alert_click_selects_only_a_live_owned_unit() {
    let world = TestWorld::default();
    let mut core = Core::new(CameraState::default());
    core.step(
        &[InputEvent::Alert(Input {
            x: 6.,
            y: 0.,
            unit: Some(2),
        })],
        &world,
        &VIEW,
        0,
    );
    assert_eq!(core.selected_units(), &[UnitId(2)]);
    core.step(
        &[InputEvent::Alert(Input {
            x: 12.,
            y: 0.,
            unit: Some(4),
        })],
        &world,
        &VIEW,
        1,
    );
    assert_eq!(core.selected_units(), &[UnitId(2)]);
}
#[test]
fn invalid_warning_cannot_poison_jump_target() {
    let world = TestWorld::default();
    let mut core = Core::new(CameraState::default());
    core.step(
        &[InputEvent::AlertAnnounced(Input {
            x: f32::NAN,
            y: 0.,
            unit: None,
        })],
        &world,
        &VIEW,
        0,
    );
    assert!(core.alert_state().latest.is_none());
}
