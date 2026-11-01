#[allow(dead_code)]
mod common;

use common::{TestWorld, VIEW};
use glam::vec2;
use turts_interaction::projection::screen_to_world;
use turts_interaction::{CameraState, Core, InputEvent, Viewport, camera::CameraOptions};
use turts_mod_api::{CameraDef, CameraStart};

fn camera() -> CameraState {
    CameraState {
        look_at: vec2(32.0, 32.0),
        height: 40.0,
        pitch: 0.7,
        yaw: 0.0,
        vertical_fov: 0.9,
    }
}

fn camera_def() -> CameraDef {
    CameraDef {
        pitch: 0.7,
        yaw: 0.0,
        field_of_view: 60.0,
        minimum: 10.0,
        maximum: 80.0,
        start: CameraStart::MapCentre,
        height: 40.0,
        margin: 5.0,
        pan_speed: 12.0,
        edge_band: true,
    }
}

fn world() -> TestWorld {
    TestWorld {
        camera_options: CameraOptions {
            authored: Some(camera_def()),
            scroll_speed: 1.0,
            edge_scroll: true,
        },
        ..TestWorld::default()
    }
}

fn step(core: &mut Core, world: &TestWorld, events: &[InputEvent], now_ms: u64) {
    core.step(events, world, &VIEW, now_ms);
}

fn step_view(
    core: &mut Core,
    world: &TestWorld,
    view: &Viewport,
    events: &[InputEvent],
    now_ms: u64,
) {
    core.step(events, world, view, now_ms);
}

#[test]
fn arrow_pans() {
    let world = world();
    let mut core = Core::new(camera());
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::KeyDown {
            code: "ArrowRight".into(),
            mods: vec![],
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    let after = core.camera_state().look_at;
    assert!(
        after.x > before.x + 0.1,
        "ArrowRight must move the look-at point right: {before:?} -> {after:?}"
    );
}

#[test]
fn edge_band_pans() {
    let world = world();
    let mut core = Core::new(camera());
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 1.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    assert!(
        core.camera_state().look_at.x > before.x + 0.1,
        "the 8 logical px edge band must pan"
    );
}

#[test]
fn edge_band_uses_eight_logical_pixels() {
    let world = world();
    let mut core = Core::new(camera());
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 9.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    assert_eq!(
        core.camera_state().look_at,
        before,
        "9 px from the edge is outside the band"
    );
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 8.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        1_000,
    );
    step(&mut core, &world, &[], 2_000);
    assert!(
        core.camera_state().look_at.x > before.x + 0.1,
        "8 px from the edge is inside the band"
    );
}

#[test]
fn pointer_left_stops_the_edge_band() {
    let world = world();
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 1.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    let stopped_at = core.camera_state().look_at;
    step(&mut core, &world, &[InputEvent::PointerLeft], 1_000);
    step(&mut core, &world, &[], 2_000);
    assert_eq!(
        core.camera_state().look_at,
        stopped_at,
        "PointerLeft must stop edge panning"
    );
}

#[test]
fn zoom_stays_in_limits() {
    let world = world();
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::Wheel { delta: -100_000.0 }],
        1,
    );
    assert_eq!(
        core.camera_state().height,
        80.0,
        "zoom out must stop at the authored maximum"
    );
    step(
        &mut core,
        &world,
        &[InputEvent::Wheel { delta: 100_000.0 }],
        2,
    );
    assert_eq!(
        core.camera_state().height,
        10.0,
        "zoom in must stop at the authored minimum"
    );
}

#[test]
fn bounds_hold() {
    let world = world();
    for (key, origin, expected) in [
        ("ArrowRight", vec2(68.9, 32.0), vec2(69.0, 32.0)),
        ("ArrowLeft", vec2(-4.9, 32.0), vec2(-5.0, 32.0)),
        ("ArrowUp", vec2(32.0, -4.9), vec2(32.0, -5.0)),
        ("ArrowDown", vec2(32.0, 68.9), vec2(32.0, 69.0)),
    ] {
        let mut initial = camera();
        initial.look_at = origin;
        let mut core = Core::new(initial);
        step(
            &mut core,
            &world,
            &[InputEvent::KeyDown {
                code: key.into(),
                mods: vec![],
            }],
            0,
        );
        for frame in 1..=100 {
            step(&mut core, &world, &[], frame * 20);
        }
        assert!(
            core.camera_state().look_at.distance(expected) < 0.001,
            "{key} must reach and stop at the authored map margin: {:?}",
            core.camera_state().look_at
        );
    }
}

#[test]
fn space_centres() {
    let world = world();
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::UiAction {
            name: "ui_select:1".into(),
        }],
        0,
    );
    step(
        &mut core,
        &world,
        &[InputEvent::KeyDown {
            code: "Space".into(),
            mods: vec![],
        }],
        1,
    );
    let expected = world.units[0].position;
    assert!(
        core.camera_state().look_at.distance(expected) < 0.01,
        "Space must centre on selected unit at {expected:?}"
    );
}

#[test]
fn scroll_speed_2x_gives_2x_distance() {
    let mut normal_world = world();
    let mut fast_world = world();
    normal_world.camera_options.scroll_speed = 1.0;
    fast_world.camera_options.scroll_speed = 2.0;
    // Keep both traces away from bounds so the assertion measures speed, not clamping.
    normal_world.map_size = Some(vec2(4096.0, 4096.0));
    fast_world.map_size = normal_world.map_size;
    let mut normal = Core::new(camera());
    let mut fast = Core::new(camera());
    let right = [InputEvent::KeyDown {
        code: "ArrowRight".into(),
        mods: vec![],
    }];
    step(&mut normal, &normal_world, &right, 0);
    step(&mut fast, &fast_world, &right, 0);
    step(&mut normal, &normal_world, &[], 1_000);
    step(&mut fast, &fast_world, &[], 1_000);
    let normal_distance = normal.camera_state().look_at.distance(camera().look_at);
    let fast_distance = fast.camera_state().look_at.distance(camera().look_at);
    assert!(normal_distance > 0.0, "baseline camera input must pan");
    assert!(
        (fast_distance - 2.0 * normal_distance).abs() < 0.01,
        "2x speed must double the one-second pan distance: {normal_distance} -> {fast_distance}"
    );
}

#[test]
fn middle_drag_pans() {
    let world = world();
    let mut core = Core::new(camera());
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::PointerDown {
            button: turts_interaction::Button::Middle,
            px: vec2(640.0, 360.0),
            over_widget: false,
        }],
        0,
    );
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(700.0, 360.0),
            over_widget: false,
        }],
        16,
    );
    step(
        &mut core,
        &world,
        &[InputEvent::PointerUp {
            button: turts_interaction::Button::Middle,
            px: vec2(700.0, 360.0),
            over_widget: false,
        }],
        16,
    );
    assert!(
        core.camera_state().look_at.distance(before) > 0.1,
        "middle drag must move the look-at point"
    );
}

#[test]
fn middle_drag_keeps_the_ground_anchor_at_default_speed() {
    let world = world();
    let mut core = Core::new(camera());
    let start_px = vec2(640.0, 360.0);
    let end_px = vec2(700.0, 360.0);
    let anchor = screen_to_world(start_px, &VIEW, camera()).expect("start ray hits the ground");
    step(
        &mut core,
        &world,
        &[InputEvent::PointerDown {
            button: turts_interaction::Button::Middle,
            px: start_px,
            over_widget: false,
        }],
        0,
    );
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: end_px,
            over_widget: false,
        }],
        16,
    );
    let under_pointer =
        screen_to_world(end_px, &VIEW, core.camera_state()).expect("end ray hits the ground");
    assert!(
        under_pointer.distance(anchor) < 0.01,
        "1x drag keeps the starting ground point under the pointer: {anchor:?} -> {under_pointer:?}"
    );
}

#[test]
fn middle_drag_stops_after_pointer_left_or_focus_loss() {
    let world = world();
    for stop in [InputEvent::PointerLeft, InputEvent::FocusLost] {
        let mut core = Core::new(camera());
        step(
            &mut core,
            &world,
            &[InputEvent::PointerDown {
                button: turts_interaction::Button::Middle,
                px: vec2(640.0, 360.0),
                over_widget: false,
            }],
            0,
        );
        step(
            &mut core,
            &world,
            &[InputEvent::PointerMove {
                px: vec2(700.0, 360.0),
                over_widget: false,
            }],
            16,
        );
        let stopped_at = core.camera_state().look_at;
        step(&mut core, &world, std::slice::from_ref(&stop), 17);
        step(
            &mut core,
            &world,
            &[InputEvent::PointerMove {
                px: vec2(760.0, 360.0),
                over_widget: false,
            }],
            32,
        );
        assert_eq!(
            core.camera_state().look_at,
            stopped_at,
            "{stop:?} must release middle-drag camera control"
        );
    }
}

#[test]
fn player_scroll_speed_scales_middle_drag_distance() {
    let mut normal_world = world();
    let mut fast_world = world();
    normal_world.camera_options.scroll_speed = 1.0;
    fast_world.camera_options.scroll_speed = 2.0;
    normal_world.map_size = Some(vec2(4_096.0, 4_096.0));
    fast_world.map_size = normal_world.map_size;

    let drag = |core: &mut Core, world: &TestWorld| {
        step(
            core,
            world,
            &[InputEvent::PointerDown {
                button: turts_interaction::Button::Middle,
                px: vec2(640.0, 360.0),
                over_widget: false,
            }],
            0,
        );
        step(
            core,
            world,
            &[InputEvent::PointerMove {
                px: vec2(700.0, 360.0),
                over_widget: false,
            }],
            16,
        );
    };
    let mut normal = Core::new(camera());
    let mut fast = Core::new(camera());
    drag(&mut normal, &normal_world);
    drag(&mut fast, &fast_world);
    let normal_distance = normal.camera_state().look_at.distance(camera().look_at);
    let fast_distance = fast.camera_state().look_at.distance(camera().look_at);
    assert!(normal_distance > 0.0);
    assert!(
        (fast_distance - 2.0 * normal_distance).abs() < 0.01,
        "2x player speed must double middle-drag distance: {normal_distance} -> {fast_distance}"
    );
}

#[test]
fn middle_drag_distance_is_stable_when_input_is_split_across_frames() {
    let mut world = world();
    world.camera_options.scroll_speed = 2.0;
    world.map_size = Some(vec2(4_096.0, 4_096.0));
    let down = [InputEvent::PointerDown {
        button: turts_interaction::Button::Middle,
        px: vec2(640.0, 360.0),
        over_widget: false,
    }];
    let move_to = |x, y| {
        [InputEvent::PointerMove {
            px: vec2(x, y),
            over_widget: false,
        }]
    };
    let mut single_frame = Core::new(camera());
    let mut split_frames = Core::new(camera());
    step(&mut single_frame, &world, &down, 0);
    step(&mut split_frames, &world, &down, 0);
    step(&mut single_frame, &world, &move_to(700.0, 360.0), 32);
    step(&mut split_frames, &world, &move_to(670.0, 360.0), 16);
    step(&mut split_frames, &world, &move_to(700.0, 360.0), 32);
    assert!(
        single_frame
            .camera_state()
            .look_at
            .abs_diff_eq(split_frames.camera_state().look_at, 0.01),
        "equal total pointer movement must produce equal camera movement across frames"
    );
}

#[test]
fn authored_edge_setting_disables_edge_pan() {
    let mut world = world();
    world.camera_options.authored.as_mut().unwrap().edge_band = false;
    let mut core = Core::new(camera());
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 1.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    assert_eq!(
        core.camera_state().look_at,
        before,
        "the authored edge-band switch must disable edge pan"
    );
}

#[test]
fn camera_projection_ray_round_trips_a_ground_point_after_pan_and_zoom() {
    let world = world();
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::KeyDown {
            code: "ArrowRight".into(),
            mods: vec![],
        }],
        0,
    );
    step(&mut core, &world, &[], 500);
    step(&mut core, &world, &[InputEvent::Wheel { delta: 3.0 }], 501);
    assert_ne!(
        core.camera_state(),
        camera(),
        "the ground-ray round trip must use the camera state after its pan and zoom inputs"
    );
    let ground_point = vec2(41.0, 27.0);
    let screen = core.project_world_to_screen(ground_point, &VIEW);
    let recovered = screen_to_world(screen, &VIEW, core.camera_state())
        .expect("the projected screen point must have a valid ground ray");
    assert!(
        recovered.distance(ground_point) < 0.05,
        "ground ray recovered {recovered:?} from {ground_point:?}"
    );
}

#[test]
fn player_edge_setting_disables_edge_pan_independently_of_the_mod() {
    let mut world = world();
    world.camera_options.edge_scroll = false;
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 1.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        0,
    );
    step(&mut core, &world, &[], 1000);
    assert_eq!(core.camera_state().look_at, camera().look_at);
}

#[test]
fn player_rebound_camera_action_drives_pan() {
    let mut world = world();
    world.runtime_bindings.insert(
        "camera_pan_right".into(),
        vec![turts_mod_api::Binding::Key {
            code: "KeyN".into(),
            mods: vec![],
        }],
    );
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::KeyDown {
            code: "KeyN".into(),
            mods: vec![],
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    assert!(core.camera_state().look_at.x > camera().look_at.x + 0.1);
}

#[test]
fn opposite_camera_actions_cancel_each_other() {
    let world = world();
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[
            InputEvent::KeyDown {
                code: "ArrowRight".into(),
                mods: vec![],
            },
            InputEvent::KeyDown {
                code: "ArrowLeft".into(),
                mods: vec![],
            },
        ],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    assert_eq!(core.camera_state().look_at, camera().look_at);
}

#[test]
fn stale_pointer_outside_resized_viewport_does_not_edge_pan() {
    let world = world();
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 1.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        0,
    );
    let resized = Viewport {
        size: vec2(1_000.0, 600.0),
        scale_factor: 1.0,
    };
    step_view(&mut core, &world, &resized, &[], 1_000);
    assert_eq!(core.camera_state().look_at, camera().look_at);
}

#[test]
fn key_up_and_focus_lost_each_stop_a_held_arrow() {
    let mut world = world();
    world.map_size = Some(vec2(4096.0, 4096.0));
    for release in [
        InputEvent::KeyUp {
            code: "ArrowRight".into(),
        },
        InputEvent::FocusLost,
    ] {
        let mut core = Core::new(camera());
        step(
            &mut core,
            &world,
            &[InputEvent::KeyDown {
                code: "ArrowRight".into(),
                mods: vec![],
            }],
            0,
        );
        step(&mut core, &world, &[], 1000);
        let moved = core.camera_state().look_at;
        assert!(
            moved.x > camera().look_at.x,
            "the stop test must start with an actually moving camera"
        );
        step(&mut core, &world, &[release], 1000);
        step(&mut core, &world, &[], 2000);
        assert_eq!(core.camera_state().look_at, moved);
    }
}

#[test]
fn focus_lost_stops_edge_scroll() {
    let mut world = world();
    world.map_size = Some(vec2(4_096.0, 4_096.0));
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 1.0, VIEW.size.y * 0.5),
            over_widget: false,
        }],
        0,
    );
    step(&mut core, &world, &[], 1000);
    let moved = core.camera_state().look_at;
    assert!(
        moved.x > camera().look_at.x,
        "edge scroll must move before focus is lost"
    );
    step(&mut core, &world, &[InputEvent::FocusLost], 1000);
    step(&mut core, &world, &[], 2000);
    assert_eq!(
        core.camera_state().look_at,
        moved,
        "focus loss must stop edge panning"
    );
}

#[test]
fn camera_height_scales_pan_distance_without_hitting_bounds() {
    let mut world = world();
    world.map_size = Some(vec2(4096.0, 4096.0));
    let mut low_pose = camera();
    low_pose.height = 20.0;
    let mut high = Core::new(camera());
    let mut low = Core::new(low_pose);
    for core in [&mut low, &mut high] {
        step(
            core,
            &world,
            &[InputEvent::KeyDown {
                code: "ArrowRight".into(),
                mods: vec![],
            }],
            0,
        );
        step(core, &world, &[], 1000);
    }
    let short_distance = low.camera_state().look_at.distance(camera().look_at);
    let tall_distance = high.camera_state().look_at.distance(camera().look_at);
    assert!(short_distance > 0.0);
    assert!(
        (tall_distance - 2.0 * short_distance).abs() < 0.01,
        "twice the height must scale the same one-second pan: {short_distance} -> {tall_distance}"
    );
}

#[test]
fn double_recall_centres_on_the_assigned_group() {
    let world = world();
    let mut core = Core::new(camera());
    step(
        &mut core,
        &world,
        &[InputEvent::RestoreGroups {
            groups: vec![(1, vec![world.units[0].id, world.units[1].id])],
        }],
        0,
    );
    for time in [100, 110, 200] {
        let event = if time == 110 {
            InputEvent::KeyUp {
                code: "Digit1".into(),
            }
        } else {
            InputEvent::KeyDown {
                code: "Digit1".into(),
                mods: vec![],
            }
        };
        step(&mut core, &world, &[event], time);
        if time == 100 {
            assert_eq!(
                core.camera_state().look_at,
                camera().look_at,
                "one recall must not centre the camera"
            );
        }
    }
    let expected = (world.units[0].position + world.units[1].position) / 2.0;
    assert_eq!(
        core.selected_units(),
        &[world.units[0].id, world.units[1].id]
    );
    assert!(
        core.camera_state().look_at.distance(expected) < 0.01,
        "double recall must centre the camera on the whole group at {expected:?}"
    );
}

#[test]
fn edge_pan_ignores_a_mod_widget() {
    let world = world();
    let mut core = Core::new(camera());
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(VIEW.size.x - 1.0, VIEW.size.y * 0.5),
            over_widget: true,
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    assert_eq!(
        core.camera_state().look_at,
        before,
        "edge scroll must not start over a mod widget"
    );
}

#[test]
fn middle_drag_started_over_a_mod_widget_does_not_pan() {
    let world = world();
    let mut core = Core::new(camera());
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::PointerDown {
            button: turts_interaction::Button::Middle,
            px: vec2(640.0, 360.0),
            over_widget: true,
        }],
        0,
    );
    step(
        &mut core,
        &world,
        &[InputEvent::PointerMove {
            px: vec2(760.0, 360.0),
            over_widget: false,
        }],
        16,
    );
    step(
        &mut core,
        &world,
        &[InputEvent::PointerUp {
            button: turts_interaction::Button::Middle,
            px: vec2(760.0, 360.0),
            over_widget: false,
        }],
        16,
    );
    assert_eq!(
        core.camera_state().look_at,
        before,
        "a drag that starts on a mod widget must not pan the scene"
    );
}

#[test]
fn edge_band_uses_logical_pixels_after_viewport_resize() {
    let world = world();
    let mut core = Core::new(camera());
    let resized = Viewport {
        size: vec2(1_920.0, 1_080.0),
        scale_factor: 2.0,
    };
    let before = core.camera_state().look_at;
    step_view(
        &mut core,
        &world,
        &resized,
        &[InputEvent::PointerMove {
            px: vec2(1_911.0, 540.0),
            over_widget: false,
        }],
        0,
    );
    step_view(&mut core, &world, &resized, &[], 1_000);
    assert_eq!(
        core.camera_state().look_at,
        before,
        "9 logical px from a resized viewport edge is outside the band"
    );
    step_view(
        &mut core,
        &world,
        &resized,
        &[InputEvent::PointerMove {
            px: vec2(1_912.0, 540.0),
            over_widget: false,
        }],
        1_000,
    );
    step_view(&mut core, &world, &resized, &[], 2_000);
    assert!(
        core.camera_state().look_at.x > before.x + 0.1,
        "the 8 logical px band must remain 8 logical px at scale 2"
    );
}

#[test]
fn yaw_rotates_arrow_pan_into_camera_space() {
    let world = world();
    let mut initial = camera();
    initial.yaw = std::f32::consts::FRAC_PI_2;
    let mut core = Core::new(initial);
    let before = core.camera_state().look_at;
    step(
        &mut core,
        &world,
        &[InputEvent::KeyDown {
            code: "ArrowRight".into(),
            mods: vec![],
        }],
        0,
    );
    step(&mut core, &world, &[], 1_000);
    let delta = core.camera_state().look_at - before;
    assert!(
        delta.x.abs() < 0.1 && delta.y.abs() > 0.1,
        "ArrowRight must follow the camera's yaw-rotated right axis: {delta:?}"
    );
}

#[test]
fn elapsed_time_scales_pan_distance() {
    let mut world = world();
    world.map_size = Some(vec2(4_096.0, 4_096.0));
    let mut quarter_second = Core::new(camera());
    let mut one_second = Core::new(camera());
    let key_down = [InputEvent::KeyDown {
        code: "ArrowRight".into(),
        mods: vec![],
    }];
    step(&mut quarter_second, &world, &key_down, 0);
    step(&mut one_second, &world, &key_down, 0);
    for time in (16..=256).step_by(16) {
        step(&mut quarter_second, &world, &[], time);
    }
    for time in (16..=1_024).step_by(16) {
        step(&mut one_second, &world, &[], time);
    }
    let quarter = quarter_second
        .camera_state()
        .look_at
        .distance(camera().look_at);
    let full = one_second.camera_state().look_at.distance(camera().look_at);
    assert!(quarter > 0.0);
    assert!(
        (full - 4.0 * quarter).abs() < 0.05,
        "four times as much elapsed time must pan four times as far: {quarter} -> {full}"
    );
}
