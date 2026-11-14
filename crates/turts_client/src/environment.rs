//! Map-owned presentation. No system in this module writes to replicated or simulation state.
use crate::{
    join::AcquiredPackages,
    models::{GraphicsSettings, Quality},
    render::MainSceneCamera,
};
use bevy::{
    anti_alias::fxaa::Fxaa,
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
};
use turts_mod_api::{Antialiasing, EnvironmentDef};

#[derive(Component)]
pub(crate) struct AppliedEnvironment {
    quality: Quality,
    definition: EnvironmentDef,
}

pub(crate) fn ground_extent(size: (f32, f32), environment: Option<&EnvironmentDef>) -> (f32, f32) {
    let margin = environment.map_or(0.0, |definition| definition.ground_margin);
    (size.0 + 2.0 * margin, size.1 + 2.0 * margin)
}

pub(crate) fn sync_camera(
    mut commands: Commands,
    acquired: Option<Res<AcquiredPackages>>,
    settings: Res<GraphicsSettings>,
    mut cameras: Query<(Entity, &mut Camera, Option<&AppliedEnvironment>), With<MainSceneCamera>>,
) {
    let Some(definition) = acquired
        .as_deref()
        .and_then(|acquired| acquired.0.presentation.environment.as_ref())
    else {
        return;
    };
    for (entity, mut camera, applied) in &mut cameras {
        if applied.is_some_and(|applied| {
            applied.quality == settings.quality && applied.definition == *definition
        }) {
            continue;
        }
        if let Some(colour) = crate::widgets::primitives::parse_colour(&definition.background) {
            camera.clear_color = ClearColorConfig::Custom(colour);
        }
        let detailed = settings.quality != Quality::Low;
        let mut camera = commands.entity(entity);
        camera.insert((
            Msaa::Off,
            Fxaa {
                enabled: detailed && definition.antialiasing == Antialiasing::Fxaa,
                ..default()
            },
            AppliedEnvironment {
                quality: settings.quality,
                definition: definition.clone(),
            },
        ));
        if let Some(fog) = definition.fog.as_ref().filter(|_| detailed) {
            if let Some(colour) = crate::widgets::primitives::parse_colour(&fog.colour) {
                camera.insert(DistanceFog {
                    color: colour.with_alpha(fog.opacity),
                    falloff: FogFalloff::Linear {
                        start: fog.start,
                        end: fog.end,
                    },
                    ..default()
                });
            }
        } else {
            camera.remove::<DistanceFog>();
        }
    }
}

pub(crate) fn boundary_points(size: (f32, f32)) -> [Vec3; 5] {
    [
        Vec3::new(0.0, 0.02, 0.0),
        Vec3::new(size.0, 0.02, 0.0),
        Vec3::new(size.0, 0.02, size.1),
        Vec3::new(0.0, 0.02, size.1),
        Vec3::new(0.0, 0.02, 0.0),
    ]
}

pub(crate) fn draw_boundary(acquired: Option<Res<AcquiredPackages>>, mut gizmos: Gizmos) {
    let Some(acquired) = acquired else { return };
    let Some(colour) = acquired
        .0
        .presentation
        .environment
        .as_ref()
        .and_then(|definition| definition.boundary_colour.as_ref())
        .and_then(|hex| crate::widgets::primitives::parse_colour(hex))
    else {
        return;
    };
    gizmos.linestrip(boundary_points(acquired.0.map_size), colour);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acquire::{VerifiedRootMod, build_acquired, build_acquired_with_root};
    use turts_mod_api::DistanceFogDef;
    use turts_mod_runtime::{
        archive::PackageArchive,
        pack::{pack_dir, pack_files},
    };

    fn definition() -> EnvironmentDef {
        EnvironmentDef {
            background: "#B8D4DF".into(),
            ground_margin: 64.0,
            boundary_colour: Some("#536A46".into()),
            fog: Some(DistanceFogDef {
                colour: "#B8D4DF".into(),
                start: 80.0,
                end: 220.0,
                opacity: 0.2,
            }),
            antialiasing: Antialiasing::Fxaa,
        }
    }
    fn package(name: &str) -> PackageArchive {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../mods")
            .join(name);
        PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap()
    }
    fn app(environment: Option<EnvironmentDef>) -> (App, Entity) {
        let mut acquired = build_acquired(&[package("skirmish")], String::new()).unwrap();
        acquired.presentation.environment = environment;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(AcquiredPackages(acquired))
            .init_resource::<GraphicsSettings>()
            .add_systems(Update, sync_camera);
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), MainSceneCamera, Msaa::Sample4))
            .id();
        (app, camera)
    }

    #[test]
    fn quality_changes_toggle_fog_and_fxaa_without_changing_the_map() {
        let (mut app, camera) = app(Some(definition()));
        app.update();
        assert_eq!(*app.world().get::<Msaa>(camera).unwrap(), Msaa::Off);
        assert!(app.world().get::<Fxaa>(camera).unwrap().enabled);
        let fog = app.world().get::<DistanceFog>(camera).unwrap();
        assert!(matches!(
            fog.falloff,
            FogFalloff::Linear {
                start: 80.0,
                end: 220.0
            }
        ));
        assert_eq!(fog.color.alpha(), 0.2);
        assert_eq!(
            app.world().resource::<AcquiredPackages>().0.map_size,
            (64.0, 48.0)
        );
        app.world_mut().resource_mut::<GraphicsSettings>().quality = Quality::Low;
        app.update();
        assert!(app.world().get::<DistanceFog>(camera).is_none());
        assert!(!app.world().get::<Fxaa>(camera).unwrap().enabled);
        app.world_mut().resource_mut::<GraphicsSettings>().quality = Quality::High;
        app.update();
        assert!(app.world().get::<DistanceFog>(camera).is_some());
        assert!(app.world().get::<Fxaa>(camera).unwrap().enabled);
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .environment
            .as_mut()
            .unwrap()
            .antialiasing = Antialiasing::Off;
        app.update();
        assert!(!app.world().get::<Fxaa>(camera).unwrap().enabled);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Fxaa>>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn absent_environment_preserves_the_existing_camera_settings() {
        let (mut app, camera) = app(None);
        let before = Color::srgb(0.1, 0.2, 0.3);
        app.world_mut()
            .get_mut::<Camera>(camera)
            .unwrap()
            .clear_color = ClearColorConfig::Custom(before);
        app.update();
        assert_eq!(*app.world().get::<Msaa>(camera).unwrap(), Msaa::Sample4);
        assert!(
            matches!(app.world().get::<Camera>(camera).unwrap().clear_color, ClearColorConfig::Custom(colour) if colour == before)
        );
        assert!(app.world().get::<Fxaa>(camera).is_none());
        assert!(app.world().get::<DistanceFog>(camera).is_none());
    }

    #[test]
    fn decorative_extent_and_boundary_keep_separate_dimensions() {
        assert_eq!(ground_extent((64.0, 48.0), None), (64.0, 48.0));
        assert_eq!(
            ground_extent((64.0, 48.0), Some(&definition())),
            (192.0, 176.0)
        );
        let points = boundary_points((64.0, 48.0));
        assert_eq!(points[0], points[4]);
        assert_eq!(points[0], Vec3::new(0.0, 0.02, 0.0));
        assert_eq!(points[2], Vec3::new(64.0, 0.02, 48.0));
    }

    fn ground_vertices(margin: f32, tile_size: f32) -> Vec<([f32; 3], [f32; 2])> {
        let mut acquired = build_acquired(&[package("skirmish")], String::new()).unwrap();
        let mut env = definition();
        env.ground_margin = margin;
        acquired.presentation.environment = Some(env);
        acquired.presentation.ground.as_mut().unwrap().tile_size = tile_size;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(AcquiredPackages(acquired))
            .insert_resource(crate::join::Phase::Joined { slot: 0 })
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<GraphicsSettings>()
            .init_resource::<crate::models::ModelState>()
            .add_systems(Update, crate::render::setup_scene);
        app.update();
        let (handle, transform) = app
            .world_mut()
            .query_filtered::<(&Mesh3d, &Transform), With<crate::models::GroundSurface>>()
            .single(app.world())
            .unwrap();
        let mesh = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&handle.0)
            .unwrap();
        let bevy::mesh::VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!("positions")
        };
        let bevy::mesh::VertexAttributeValues::Float32x2(uvs) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!("uvs")
        };
        positions
            .iter()
            .zip(uvs)
            .map(|(position, uv)| {
                (
                    transform.transform_point(Vec3::from(*position)).to_array(),
                    *uv,
                )
            })
            .collect()
    }

    #[test]
    fn independent_fractional_margin_preserves_world_texture_phase() {
        let sample = |vertices: &[([f32; 3], [f32; 2])], axis: usize, world: f32| {
            let lo = vertices
                .iter()
                .min_by(|a, b| a.0[axis].total_cmp(&b.0[axis]))
                .unwrap();
            let hi = vertices
                .iter()
                .max_by(|a, b| a.0[axis].total_cmp(&b.0[axis]))
                .unwrap();
            let uv_axis = if axis == 0 { 0 } else { 1 };
            lo.1[uv_axis]
                + (hi.1[uv_axis] - lo.1[uv_axis]) * (world - lo.0[axis]) / (hi.0[axis] - lo.0[axis])
        };
        let original = ground_vertices(0.0, 8.0);
        let extended = ground_vertices(3.25, 8.0);
        for axis in [0, 2] {
            for world in [0.0, 7.5, 24.0, 48.0] {
                assert!(
                    (sample(&original, axis, world) - sample(&extended, axis, world)).abs() < 1e-5,
                    "phase changed at axis {axis}, world {world}"
                );
            }
        }
    }

    #[test]
    fn verified_root_map_wins_over_an_extra_package_map() {
        let extra = std::collections::BTreeMap::from([
            ("manifest.ron".into(), br##"(id:"atlas",version:"1.0.0",kind:Game,engine_api:"^0.2.10",dependencies:{},exports:(units:[],maps:["preview.ron"],visuals:[],resources:[],nodes:[],production:[]),context:Shared,capabilities:[],license:"MIT")"##.to_vec()),
            ("preview.ron".into(), br##"(id:"preview",size:(x:8.0,y:9.0),placements:[],nodes:[],slot_economy:[],environment:Some((background:"#FFFFFF",ground_margin:500.0)))"##.to_vec())
        ]);
        let extra = PackageArchive::parse(&pack_files(&extra).unwrap()).unwrap();
        let root = package("skirmish");
        let expected = root.maps().unwrap()[0].1.clone();
        // An extra archive must not override the verified game map.
        let acquired = build_acquired_with_root(
            &[extra, root.clone()],
            String::new(),
            Some(VerifiedRootMod {
                id: root.manifest.id.clone(),
                version: root.manifest.version.clone(),
            }),
        )
        .unwrap();
        assert_eq!(acquired.map_size, (64.0, 48.0));
        assert_eq!(acquired.presentation.ground, expected.ground);
        assert_eq!(acquired.presentation.environment, expected.environment);
    }
}

/// Actual camera components and ground vertices for browser acceptance. No input path reads this.
#[derive(Resource, Default)]
pub(crate) struct EnvironmentObservation(pub serde_json::Value);

#[allow(clippy::type_complexity)]
pub(crate) fn capture(
    cameras: Query<
        (&Camera, Option<&Msaa>, Option<&Fxaa>, Option<&DistanceFog>),
        With<MainSceneCamera>,
    >,
    ground: Query<(&Mesh3d, &Transform), With<crate::models::GroundSurface>>,
    meshes: Res<Assets<Mesh>>,
    acquired: Option<Res<AcquiredPackages>>,
    mut observation: ResMut<EnvironmentObservation>,
    time: Res<Time<Real>>,
    mut next: Local<f64>,
) {
    let now = time.elapsed_secs_f64();
    if now < *next {
        return;
    }
    *next = now + 0.25;
    let camera = cameras.iter().next().map(|(camera, msaa, fxaa, fog)| {
        let background = match camera.clear_color {
            ClearColorConfig::Custom(colour) => { let c=colour.to_srgba(); Some([c.red,c.green,c.blue,c.alpha]) },
            _ => None,
        };
        let fog = fog.map(|fog| { let c=fog.color.to_srgba(); let distances=match fog.falloff { FogFalloff::Linear {start,end} => Some([start,end]), _=>None }; serde_json::json!({"colour":[c.red,c.green,c.blue,c.alpha],"distances":distances}) });
        serde_json::json!({"background":background,"samples":msaa.map(Msaa::samples),"fxaa":fxaa.map(|value|value.enabled),"fog":fog})
    });
    let ground = ground
        .iter()
        .filter_map(|(mesh, transform)| {
            let mesh = meshes.get(&mesh.0)?;
            let bevy::mesh::VertexAttributeValues::Float32x3(positions) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
            else {
                return None;
            };
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for position in positions {
                let point = transform.transform_point(Vec3::from(*position));
                min = min.min(point);
                max = max.max(point);
            }
            Some(serde_json::json!({"min":min.to_array(),"max":max.to_array()}))
        })
        .collect::<Vec<_>>();
    observation.0 = serde_json::json!({"camera":camera,"ground":ground,"map_size":acquired.as_deref().map(|value|value.0.map_size)});
}
