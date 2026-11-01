//! Camera owner configuration, authored startup pose, and scene-light adaptation.

use crate::{core_adapter::CoreAdapterSet, ingest::ReplicaState, join::AcquiredPackages};
use bevy::prelude::{
    App, Color, IntoScheduleConfigs, Plugin, Res, ResMut, Resource, Update, Vec2, Vec3,
};
use bevy::{
    light::GlobalAmbientLight,
    prelude::{DirectionalLight, Transform},
};
use turts_interaction::{CameraState, camera::CameraOptions};
use turts_mod_api::{CameraDef, LightDef};
use turts_mod_runtime::archive::PackageArchive;

use crate::replica::Replica;
#[derive(Resource, Debug, Clone, Default)]
pub struct CameraConfiguration {
    /// Initial pose latched from authored settings and the first accepted world snapshot.
    pub initial_pose: Option<CameraState>,
    /// Authored camera controls and player settings consumed by the interaction core.
    pub options: CameraOptions,
}

pub struct ClientCameraPlugin;
impl Plugin for ClientCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraConfiguration>()
            .add_systems(Update, configure_camera.before(CoreAdapterSet));
    }
}

/// Copy accepted camera controls and prepare the initial pose before the core is constructed.
fn configure_camera(
    acquired: Option<Res<AcquiredPackages>>,
    replica_state: Res<ReplicaState>,
    mut configuration: ResMut<CameraConfiguration>,
) {
    let Some(acquired) = acquired else {
        return;
    };
    let content = &acquired.0.content;
    if configuration.options.authored.is_none() {
        configuration.options.authored = content.camera.clone();
    }
    if configuration.initial_pose.is_some()
        || !replica_state
            .replica
            .as_ref()
            .is_some_and(|replica| replica.snapshot_tick.is_some())
    {
        return;
    }
    let Some(replica) = replica_state.replica.as_ref() else {
        return;
    };
    let Some(welcome) = replica_state.welcome else {
        return;
    };
    let map_size = Vec2::new(acquired.0.map_size.0, acquired.0.map_size.1);
    let owned_centre = owned_centre(replica, welcome.slot, map_size);
    let definition = configuration
        .options
        .authored
        .as_ref()
        .or(content.camera.as_ref());
    configuration.initial_pose = Some(definition.map_or_else(
        || camera_state_from_content(None, map_size, owned_centre),
        |definition| camera_state_from_definition(definition, map_size, owned_centre),
    ));
}

/// Resolve the first authored light in the same archive order used for client camera settings.
pub(crate) fn resolve_light(archives: &[PackageArchive]) -> Option<LightDef> {
    archives.iter().find_map(|archive| {
        archive
            .interfaces()
            .ok()?
            .into_iter()
            .find_map(|(_, interface)| interface.light)
    })
}

/// Translate the authored camera start and optical controls to the renderer-independent pose.
pub(crate) fn camera_state_from_definition(
    definition: &CameraDef,
    map_size: Vec2,
    owned_centre: Option<Vec2>,
) -> CameraState {
    camera_state_from_content(Some(definition), map_size, owned_centre)
}

fn camera_state_from_content(
    definition: Option<&CameraDef>,
    map_size: Vec2,
    owned_centre: Option<Vec2>,
) -> CameraState {
    turts_interaction::camera::initial_state(definition, map_size, owned_centre)
}

/// Build the actual Bevy components for the authored scene light.
pub(crate) fn light_components(
    light: Option<&LightDef>,
) -> (DirectionalLight, Transform, GlobalAmbientLight) {
    let Some(light) = light else {
        return (
            DirectionalLight::default(),
            Transform::from_xyz(-20.0, 60.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
            GlobalAmbientLight::default(),
        );
    };
    let colour =
        Color::Srgba(bevy::color::Srgba::hex(&light.colour).unwrap_or(bevy::color::Srgba::WHITE));
    let direction = Vec3::new(light.direction.0, light.direction.1, light.direction.2);
    let direction = if direction.is_finite() && direction.length_squared() > f32::EPSILON {
        direction.normalize()
    } else {
        Vec3::new(1.0, -1.0, 1.0).normalize()
    };
    let up = if direction.dot(Vec3::Y).abs() > 0.999 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let mut directional = DirectionalLight::default();
    directional.color = colour;
    directional.illuminance = light.illuminance;
    (
        directional,
        Transform::default().looking_to(direction, up),
        GlobalAmbientLight {
            color: colour,
            brightness: light.ambient,
            ..Default::default()
        },
    )
}

fn owned_centre(replica: &Replica, own_slot: u8, map_size: Vec2) -> Option<Vec2> {
    let mut own_units = replica
        .units
        .values()
        .filter(|unit| unit.owner_slot == own_slot)
        .filter_map(|unit| unit.samples.back().map(|(_, pos)| Vec2::new(pos.x, pos.y)))
        .filter(|position| position.is_finite());
    let first = own_units.next()?;
    let (sum, count) = own_units.fold((first, 1_u32), |(sum, count), position| {
        (sum + position, count + 1)
    });
    let centre = sum / count as f32;
    Some(centre.clamp(Vec2::ZERO, map_size))
}

#[cfg(test)]
mod camera_light_tests {
    use crate::{
        acquire::build_acquired, core_adapter::CoreAdapterSet, ingest::ReplicaState,
        join::AcquiredPackages,
    };
    use bevy::prelude::{
        App, Color, DirectionalLight, IntoScheduleConfigs, Res, ResMut, Resource, Transform,
        Update, Vec2, Vec3, World,
    };
    use std::path::PathBuf;
    use turts_mod_api::{CameraDef, CameraStart, LightDef};
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

    use super::{
        CameraConfiguration, CameraState, ClientCameraPlugin, camera_state_from_definition,
        light_components, resolve_light,
    };

    fn authored_camera(start: CameraStart) -> CameraDef {
        CameraDef {
            pitch: 0.62,
            yaw: -0.3,
            field_of_view: 48.0,
            minimum: 12.0,
            maximum: 88.0,
            start,
            height: 42.0,
            margin: 6.0,
            pan_speed: 14.0,
            edge_band: true,
        }
    }

    #[test]
    fn authored_camera_start_and_optics_become_the_initial_core_pose() {
        let map_size = Vec2::new(64.0, 48.0);
        let owned_centre = Some(Vec2::new(12.0, 18.0));
        let cases = [
            (CameraStart::OwnedCentre, Vec2::new(12.0, 18.0)),
            (CameraStart::MapCentre, Vec2::new(32.0, 24.0)),
            (CameraStart::Point(7.0, 9.0), Vec2::new(7.0, 9.0)),
        ];
        for (start, expected_look_at) in cases {
            let actual =
                camera_state_from_definition(&authored_camera(start), map_size, owned_centre);
            assert_eq!(actual.look_at, expected_look_at, "authored start {start:?}");
            assert_eq!(actual.height, 42.0);
            assert_eq!(actual.pitch, 0.62);
            assert_eq!(actual.yaw, -0.3);
            assert!((actual.vertical_fov - 48.0_f32.to_radians()).abs() < 1e-6);
        }
    }

    #[derive(Resource, Default)]
    struct CorePoseObservation(Option<CameraState>);

    fn observe_core_pose(
        configuration: Res<CameraConfiguration>,
        mut observation: ResMut<CorePoseObservation>,
    ) {
        observation.0 = configuration.initial_pose;
    }

    #[test]
    fn authored_camera_options_are_ready_while_pose_waits_for_first_snapshot() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let archive_path = root.join("muster");
        let archive = PackageArchive::parse(&pack_dir(&archive_path).expect("pack camera fixture"))
            .expect("parse camera fixture");
        let mut acquired = build_acquired(&[archive], String::new()).expect("build camera fixture");
        acquired.map_size = (64.0, 48.0);
        acquired.content.camera = Some(authored_camera(CameraStart::Point(7.0, 9.0)));

        let mut app = App::new();
        app.insert_resource(AcquiredPackages(acquired));
        app.init_resource::<ReplicaState>()
            .init_resource::<CorePoseObservation>()
            .add_plugins(ClientCameraPlugin)
            .add_systems(Update, observe_core_pose.in_set(CoreAdapterSet));
        app.update();

        let configuration = app.world().resource::<CameraConfiguration>();
        assert_eq!(
            configuration.options.authored,
            Some(authored_camera(CameraStart::Point(7.0, 9.0)))
        );
        assert_eq!(
            app.world().resource::<CorePoseObservation>().0,
            None,
            "the first authored pose must wait for the accepted initial snapshot"
        );
    }

    #[test]
    fn accepted_authored_light_reaches_the_spawned_bevy_components() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let archives = ["base", "muster"]
            .into_iter()
            .map(|package| {
                let path = root.join(package);
                PackageArchive::parse(&pack_dir(&path).expect("pack light fixture"))
                    .expect("parse light fixture")
            })
            .collect::<Vec<_>>();

        let authored =
            resolve_light(&archives).expect("muster's accepted interface defines LightDef");
        assert_eq!(authored.direction, (-0.5, -1.0, -0.4));
        assert_eq!(authored.colour, "#ffffff");
        assert_eq!(authored.illuminance, 10_000.0);
        assert_eq!(authored.ambient, 0.3);

        let (directional, transform, ambient) = light_components(Some(&authored));
        let mut world = World::new();
        world.insert_resource(ambient);
        let entity = world.spawn((directional, transform)).id();
        let actual_light = world
            .get::<bevy::prelude::DirectionalLight>(entity)
            .expect("the scene light is a real Bevy component");
        let actual_transform = world
            .get::<Transform>(entity)
            .expect("the scene light direction is an actual entity transform");
        let expected_direction = Vec3::new(-0.5, -1.0, -0.4).normalize();
        let actual_direction = actual_transform.rotation * Vec3::NEG_Z;
        assert!(actual_direction.abs_diff_eq(expected_direction, 1e-4));
        assert_eq!(actual_light.illuminance, 10_000.0);
        assert_eq!(actual_light.color, Color::Srgba(bevy::color::Srgba::WHITE));
        assert_eq!(
            world
                .resource::<bevy::light::GlobalAmbientLight>()
                .brightness,
            0.3
        );
    }

    #[test]
    fn authored_light_color_is_converted_for_directional_and_ambient_components() {
        let light = LightDef {
            direction: (0.0, -1.0, 1.0),
            colour: "#336699".into(),
            illuminance: 4_200.0,
            ambient: 0.45,
        };
        let (directional, _, ambient) = light_components(Some(&light));
        let expected =
            Color::Srgba(bevy::color::Srgba::hex("#336699").expect("valid fixture hex colour"));
        assert_eq!(directional.color, expected);
        assert_eq!(directional.illuminance, 4_200.0);
        assert_eq!(ambient.color, expected);
        assert_eq!(ambient.brightness, 0.45);
    }

    #[test]
    fn missing_authored_light_keeps_the_existing_scene_light() {
        let (directional, transform, ambient) = light_components(None);
        let mut world = World::new();
        world.insert_resource(ambient);
        let entity = world.spawn((directional, transform)).id();
        let actual_light = world
            .get::<DirectionalLight>(entity)
            .expect("the legacy directional light remains a Bevy component");
        let actual_transform = world
            .get::<Transform>(entity)
            .expect("the legacy light direction remains on the entity transform");

        assert_eq!(actual_transform.translation, Vec3::new(-20.0, 60.0, -20.0));
        assert_eq!(actual_light.color, DirectionalLight::default().color);
        assert_eq!(
            actual_light.illuminance,
            DirectionalLight::default().illuminance
        );
        assert_eq!(
            world
                .resource::<bevy::light::GlobalAmbientLight>()
                .brightness,
            bevy::light::GlobalAmbientLight::default().brightness
        );
    }
}
