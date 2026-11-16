//! The actual scheduled feedback system must preserve terrain grounding.
use super::*;
use crate::fx::{
    CombatPresentation,
    camera_feedback::shake_camera,
    emitters::{Emitters, ShakePulse},
    flash::{EffectSettings, Shake},
};

fn fixture(ground: f32) -> (App, Entity, Transform) {
    let camera = CameraState {
        look_at: Vec2::new(16.0, 12.0),
        height: 32.0,
        pitch: 0.85,
        yaw: 0.0,
        vertical_fov: 60f32.to_radians(),
    };
    let base = camera_components_at_height(camera, ground).0;
    let mut app = App::new();
    app.insert_resource(CoreRuntime {
        core: Some(Core::new(camera)),
        base_camera_transform: Some(base),
        ..Default::default()
    })
    .init_resource::<CombatPresentation>()
    .init_resource::<Emitters>()
    .insert_resource(EffectSettings {
        flashes: true,
        shake: Shake::Full,
    })
    .add_systems(Update, shake_camera);
    let entity = app
        .world_mut()
        .spawn((crate::render::MainSceneCamera, base))
        .id();
    (app, entity, base)
}
#[test]
fn terrain_camera_feedback_keeps_nonzero_base_with_no_shake() {
    let (mut app, camera, base) = fixture(1.0);
    app.update();
    assert_eq!(
        *app.world().get::<Transform>(camera).unwrap(),
        base,
        "zero-shake feedback moved render camera away from the input camera's terrain support"
    );
    assert_eq!(
        app.world()
            .resource::<CoreRuntime>()
            .base_camera_transform(),
        Some(base)
    );
}
#[test]
fn terrain_camera_feedback_shakes_without_accumulating_then_restores_base() {
    for ground in [0.0, 1.0, -2.0] {
        let (mut app, camera, base) = fixture(ground);
        app.world_mut().resource_mut::<CombatPresentation>().now = 0.25;
        app.world_mut()
            .resource_mut::<Emitters>()
            .shakes
            .push_back(ShakePulse {
                starts: 0.0,
                until: 1.0,
                strength: 0.4,
            });
        app.update();
        let shaken = *app.world().get::<Transform>(camera).unwrap();
        assert_ne!(
            shaken.translation, base.translation,
            "active shake was lost"
        );
        assert_eq!(shaken.rotation, base.rotation);
        assert_eq!(shaken.scale, base.scale);
        assert!(
            (shaken.translation - base.translation).length() <= 0.5,
            "shake displaced more than its bounded local amplitude"
        );
        for _ in 0..4 {
            app.update();
            assert_eq!(
                *app.world().get::<Transform>(camera).unwrap(),
                shaken,
                "feedback accumulated onto a previous shaken pose"
            );
        }
        app.world_mut().resource_mut::<CombatPresentation>().now = 2.0;
        app.update();
        assert_eq!(
            *app.world().get::<Transform>(camera).unwrap(),
            base,
            "expired shake lost terrain support"
        );
        app.world_mut().resource_mut::<Emitters>().shakes.clear();
        app.update();
        assert_eq!(*app.world().get::<Transform>(camera).unwrap(), base);
    }
}
#[test]
fn terrain_camera_feedback_tracks_new_base_ground_and_preserves_flat_pose() {
    let (mut app, camera, flat) = fixture(0.0);
    app.update();
    assert_eq!(*app.world().get::<Transform>(camera).unwrap(), flat);
    let state = app
        .world()
        .resource::<CoreRuntime>()
        .core()
        .unwrap()
        .camera_state();
    let elevated = camera_components_at_height(state, 1.0).0;
    app.world_mut()
        .resource_mut::<CoreRuntime>()
        .base_camera_transform = Some(elevated);
    app.update();
    assert_eq!(
        *app.world().get::<Transform>(camera).unwrap(),
        elevated,
        "new ground offset was not applied"
    );
    app.world_mut()
        .resource_mut::<CoreRuntime>()
        .base_camera_transform = Some(flat);
    app.update();
    assert_eq!(
        *app.world().get::<Transform>(camera).unwrap(),
        flat,
        "returning to flat terrain retained an old offset"
    );
}
