use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::asset::{AssetPlugin, RenderAssetUsages};
use bevy::camera::{Camera, Camera3d, ImageRenderTarget, RenderTarget};
use bevy::image::{Image, ImagePlugin};
use bevy::math::vec2;
use bevy::mesh::MeshPlugin;
use bevy::prelude::{GlobalTransform, Vec3};
use bevy::render::RenderPlugin;
use bevy::render::render_resource::TextureFormat;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::transform::TransformPlugin;
use bevy::window::WindowPlugin;
use turts_client::core_adapter::camera_components;
use turts_interaction::{CameraState, Core, Viewport};

#[test]
fn core_projection_matches_bevy_1280x720() {
    check_projection(vec2(1280.0, 720.0), 1.0, 1280, 720);
}

#[test]
fn core_projection_matches_bevy_1920x1080() {
    check_projection(vec2(1920.0, 1080.0), 1.0, 1920, 1080);
}

#[test]
fn core_projection_matches_bevy_1280x720_scale2() {
    check_projection(vec2(1280.0, 720.0), 2.0, 2560, 1440);
}

fn check_projection(logical_size: bevy::math::Vec2, scale_factor: f32, width: u32, height: u32) {
    let camera_state = CameraState {
        look_at: vec2(28.85, 20.20),
        height: 38.0,
        pitch: 0.7,
        yaw: 0.0,
        vertical_fov: 0.9,
    };
    let core = Core::new(camera_state);

    let mut app = no_window_render_app();
    let image = Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None);
    let handle = app
        .world_mut()
        .resource_mut::<bevy::asset::Assets<Image>>()
        .add(Image {
            asset_usage: RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            ..image
        });
    let (transform, projection) = camera_components(core.camera_state());
    let camera_entity = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            RenderTarget::Image(ImageRenderTarget {
                handle,
                scale_factor,
            }),
            transform,
            projection,
        ))
        .id();
    app.finish();
    app.cleanup();
    app.update();

    let world = app.world();
    let camera = world
        .get::<Camera>(camera_entity)
        .expect("PROJ1 adapter camera must exist");
    let camera_transform = world
        .get::<GlobalTransform>(camera_entity)
        .expect("TransformPlugin must update the camera transform");
    let viewport = Viewport {
        size: logical_size,
        scale_factor,
    };
    for row in -2..=2 {
        for column in -2..=1 {
            let point = camera_state.look_at + vec2(column as f32 * 2.0, row as f32 * 2.0);
            let expected = core.project_world_to_screen(point, &viewport);
            let actual = camera
                .world_to_viewport(camera_transform, Vec3::new(point.x, 0.0, point.y))
                .expect("PROJ1 sample must be in front of the camera");
            let error = expected.distance(actual);
            assert!(
                error <= 1.0,
                "PROJ1: scale={scale_factor}, point={point:?}, expected={expected:?}, actual={actual:?}, error={error:.2}px"
            );
        }
    }
}

fn no_window_render_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        ImagePlugin::default(),
        WindowPlugin {
            primary_window: None,
            ..Default::default()
        },
        TransformPlugin,
        MeshPlugin,
        RenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                backends: None,
                ..Default::default()
            })),
            ..Default::default()
        },
    ));
    app
}
