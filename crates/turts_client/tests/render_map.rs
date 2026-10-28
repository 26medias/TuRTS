//! Task 10: the pure functions of presentation and input. No window and no renderer.

use std::collections::HashMap;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use turts_client::acquire::Acquired;
use turts_client::content::ClientContent;
use turts_client::input::ground_point;
use turts_client::join::{AcquiredPackages, Phase};
use turts_client::present::{Presented, PresentedUnit as PresentedRecord};
use turts_client::render::{
    PresentedUnit, declared_node_pick_radius, declared_unit_pick_radius, mesh_for, mesh_lift,
    projected_ground_center, setup_scene, sync_units, world_to_scene,
};
use turts_mod_api::Shape;
use turts_net_protocol::Pos;

/// The size of the mesh on each scene axis: `max - min` of the vertex positions.
fn extents(mesh: &Mesh) -> Vec3 {
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|values| values.as_float3())
        .expect("the mesh has float3 positions");
    let (mut low, mut high) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for position in positions {
        low = low.min(Vec3::from(*position));
        high = high.max(Vec3::from(*position));
    }
    high - low
}

#[test]
fn world_to_scene_maps_y_forward_to_scene_z() {
    assert_eq!(
        world_to_scene(Pos { x: 8.0, y: 3.0 }),
        Vec3::new(8.0, 0.0, 3.0)
    );
}

#[test]
fn projected_ground_center_uses_camera_viewport_and_rejects_offscreen_points() {
    let mut camera = Camera::default();
    camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
        physical_size: UVec2::new(1280, 720),
        scale_factor: 2.0,
    });
    camera.computed.clip_from_view = Mat4::IDENTITY;
    let camera_transform = GlobalTransform::IDENTITY;

    let center = projected_ground_center(&camera, &camera_transform, Pos { x: 0.0, y: 0.5 })
        .expect("visible point projects into the camera viewport");
    assert_eq!(center, Vec2::new(320.0, 180.0));
    assert_eq!(
        projected_ground_center(&camera, &camera_transform, Pos { x: 2.0, y: 0.5 },),
        None,
        "a point outside the camera viewport has no clickable center"
    );
    assert_eq!(
        projected_ground_center(&camera, &camera_transform, Pos { x: 0.0, y: -0.1 }),
        None,
        "a ground point beyond the far plane is not clickable"
    );
    assert_eq!(
        projected_ground_center(&camera, &camera_transform, Pos { x: 0.0, y: 1.1 }),
        None,
        "a ground point beyond the near plane is not clickable"
    );
    camera.is_active = false;
    assert_eq!(
        projected_ground_center(&camera, &camera_transform, Pos { x: 0.0, y: 0.5 }),
        None,
        "an inactive camera cannot publish a clickable center"
    );
}

#[test]
fn declared_pick_radii_follow_the_accepted_package_catalog() {
    use std::path::PathBuf;
    use turts_client::content::ClientContent;
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive = PackageArchive::parse(&pack_dir(&base).unwrap()).unwrap();
    let content = ClientContent::from_archives(&[archive]).unwrap();
    let (unit_id, unit) = content.catalog.units.iter().next().unwrap();
    let turts_mod_api::Footprint::Circle { radius } = &unit.footprint;
    assert_eq!(
        declared_unit_pick_radius(&content, &unit_id.to_string()),
        Some(*radius)
    );
    let (node_id, node) = content.catalog.nodes.iter().next().unwrap();
    assert_eq!(
        declared_node_pick_radius(&content, &node_id.to_string()),
        Some(node.radius)
    );
    assert_eq!(declared_unit_pick_radius(&content, "malformed"), None);
    assert_eq!(declared_node_pick_radius(&content, "malformed"), None);
}

#[test]
fn mesh_lift_is_half_height() {
    let capsule = Shape::Capsule {
        radius: 0.3,
        height: 1.8,
    };
    let cuboid = Shape::Cuboid {
        x: 1.0,
        y: 2.0,
        z: 1.4,
    };
    assert!((mesh_lift(&capsule) - 0.9).abs() < 1e-6);
    assert!((mesh_lift(&cuboid) - 0.7).abs() < 1e-6);
}

#[test]
fn mesh_for_builds_both_shapes() {
    let capsule = Shape::Capsule {
        radius: 0.3,
        height: 1.8,
    };
    let cuboid = Shape::Cuboid {
        x: 1.0,
        y: 2.0,
        z: 1.4,
    };
    assert!(mesh_for(&capsule).count_vertices() > 0);
    assert!(mesh_for(&cuboid).count_vertices() > 0);
}

#[test]
fn mesh_for_sizes_follow_the_scene_axes() {
    // Scene y is up: the capsule is `height` tall and `2 * radius` wide.
    let capsule = extents(&mesh_for(&Shape::Capsule {
        radius: 0.3,
        height: 1.8,
    }));
    assert!((capsule.y - 1.8).abs() < 1e-4, "{capsule}");
    assert!((capsule.x - 0.6).abs() < 1e-4, "{capsule}");
    // The cuboid `z` is its height, so it becomes scene y. The cuboid `y` is its depth on the
    // ground, so it becomes scene z.
    let cuboid = extents(&mesh_for(&Shape::Cuboid {
        x: 1.0,
        y: 2.0,
        z: 1.4,
    }));
    assert!((cuboid.x - 1.0).abs() < 1e-4, "{cuboid}");
    assert!((cuboid.y - 1.4).abs() < 1e-4, "{cuboid}");
    assert!((cuboid.z - 2.0).abs() < 1e-4, "{cuboid}");
}

#[test]
fn ground_point_reads_scene_z_as_map_y() {
    // From (10, 40, 20) towards (14, 0, 29): the ray meets the ground at scene (14, 0, 29).
    let ray = Ray3d::new(
        Vec3::new(10.0, 40.0, 20.0),
        Dir3::new(Vec3::new(4.0, -40.0, 9.0)).unwrap(),
    );
    let point = ground_point(ray).expect("a downward ray meets the ground");
    assert!((point.x - 14.0).abs() < 1e-4, "{point:?}");
    assert!((point.y - 29.0).abs() < 1e-4, "{point:?}");
}

#[test]
fn ground_point_is_none_for_a_ray_that_looks_up() {
    let ray = Ray3d::new(Vec3::new(0.0, 5.0, 0.0), Dir3::Y);
    assert_eq!(ground_point(ray), None);
}

fn record(unit_id: u32, x: f32, y: f32) -> PresentedRecord {
    PresentedRecord {
        unit_id,
        content_id: "pkg:man".to_string(),
        pos: Pos { x, y },
        lo_tick: 1,
        hi_tick: 2,
        fraction: 0.0,
    }
}

/// An app with only `sync_units`, the asset stores it needs, and one visual.
fn sync_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_resource::<Presented>()
        .init_resource::<turts_client::ingest::ReplicaState>()
        .insert_resource(AcquiredPackages(Acquired {
            lockfile_hash: String::new(),
            visuals: HashMap::from([(
                "pkg:man".to_string(),
                Shape::Capsule {
                    radius: 0.3,
                    height: 1.8,
                },
            )]),
            map_size: (64.0, 64.0),
            unit_names: HashMap::new(),
            content: ClientContent {
                catalog: ClientContent::default().catalog,
                shapes: HashMap::from([(
                    "pkg:man".to_string(),
                    Shape::Capsule {
                        radius: 0.3,
                        height: 1.8,
                    },
                )]),
            },
        }))
        .add_systems(Update, sync_units);
    app
}

/// The shown units as `(unit_id, translation)`, ascending.
fn shown(app: &mut App) -> Vec<(u32, Vec3)> {
    let mut query = app.world_mut().query::<(&PresentedUnit, &Transform)>();
    let mut shown: Vec<(u32, Vec3)> = query
        .iter(app.world())
        .map(|(unit, transform)| (unit.0, transform.translation))
        .collect();
    shown.sort_by_key(|(id, _)| *id);
    shown
}

#[test]
fn sync_units_spawns_moves_and_despawns() {
    let mut app = sync_app();
    app.world_mut().resource_mut::<Presented>().units =
        vec![record(1, 8.0, 3.0), record(2, 5.0, 6.0)];
    app.update();
    // The capsule is 1.8 tall, so it stands 0.9 above the ground.
    assert_eq!(
        shown(&mut app),
        vec![(1, Vec3::new(8.0, 0.9, 3.0)), (2, Vec3::new(5.0, 0.9, 6.0))]
    );

    // Unit 1 moves; unit 2 is gone from the record; unit 3 is new. No duplicate for unit 1.
    app.world_mut().resource_mut::<Presented>().units =
        vec![record(1, 9.5, 4.0), record(3, 1.0, 2.0)];
    app.update();
    assert_eq!(
        shown(&mut app),
        vec![(1, Vec3::new(9.5, 0.9, 4.0)), (3, Vec3::new(1.0, 0.9, 2.0))]
    );
}

#[test]
fn sync_units_skips_a_unit_without_a_visual() {
    let mut app = sync_app();
    let mut other = record(1, 8.0, 3.0);
    other.content_id = "pkg:unknown".to_string();
    app.world_mut().resource_mut::<Presented>().units = vec![other];
    app.update();
    assert_eq!(shown(&mut app), Vec::new());
}

/// An app with only `setup_scene`, the asset stores it needs, and the given phase.
fn setup_app(phase: Phase) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(phase)
        .insert_resource(AcquiredPackages(Acquired {
            lockfile_hash: String::new(),
            visuals: HashMap::new(),
            map_size: (64.0, 64.0),
            unit_names: HashMap::new(),
            content: ClientContent::default(),
        }))
        .add_systems(Update, setup_scene);
    app
}

/// The number of 3D cameras that are on, as the browser hook counts them.
fn active_cameras_3d(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<&Camera, With<Camera3d>>();
    query
        .iter(app.world())
        .filter(|camera| camera.is_active)
        .count()
}

#[test]
fn setup_scene_spawns_exactly_one_active_camera_3d() {
    // M1 verify: a scene with no `Camera3d` has meshes that nothing can see. `window.turts_scene`
    // lists entities, so the camera needs its own check. A `setup_scene` that spawns no camera
    // goes red here, and so does one that spawns a second camera.
    let mut app = setup_app(Phase::Joined { slot: 0 });
    app.update();
    assert_eq!(active_cameras_3d(&mut app), 1);
    // The system runs once. Later frames add no camera.
    app.update();
    app.update();
    assert_eq!(active_cameras_3d(&mut app), 1);
}

#[test]
fn setup_scene_waits_for_the_join() {
    let mut app = setup_app(Phase::Joining);
    app.update();
    assert_eq!(active_cameras_3d(&mut app), 0);
    *app.world_mut().resource_mut::<Phase>() = Phase::Joined { slot: 0 };
    app.update();
    assert_eq!(active_cameras_3d(&mut app), 1);
}

#[test]
fn sync_nodes_uses_downloaded_shapes_and_stable_ids() {
    use turts_client::{
        ingest::ReplicaState,
        render::{PresentedNode, sync_nodes},
        replica::Replica,
    };
    use turts_net_protocol::{NodeId, NodeState};

    let shape = Shape::Cuboid {
        x: 1.0,
        y: 2.0,
        z: 1.4,
    };
    let mut content = ClientContent::default();
    content.shapes.insert("pkg:ore".into(), shape);
    let mut state = ReplicaState::default();
    let mut replica = Replica::new(20);
    replica.nodes.insert(
        NodeId(9),
        NodeState {
            id: NodeId(9),
            content_id: "pkg:ore".into(),
            pos: Pos { x: 12.0, y: 7.0 },
            remaining: 0,
        },
    );
    state.replica = Some(replica);

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(state)
        .insert_resource(AcquiredPackages(Acquired {
            lockfile_hash: String::new(),
            visuals: HashMap::new(),
            map_size: (64.0, 64.0),
            unit_names: HashMap::new(),
            content,
        }))
        .add_systems(Update, sync_nodes);
    app.update();

    let mut query = app
        .world_mut()
        .query::<(Entity, &PresentedNode, &Transform)>();
    let found: Vec<_> = query
        .iter(app.world())
        .map(|(entity, node, transform)| (entity, node.0, transform.translation))
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, 9);
    assert_eq!(found[0].2, Vec3::new(12.0, 0.7, 7.0));

    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .nodes
        .get_mut(&NodeId(9))
        .unwrap()
        .pos = Pos { x: 15.0, y: 8.0 };
    app.update();
    let moved: Vec<_> = query
        .iter(app.world())
        .map(|(entity, node, transform)| (entity, node.0, transform.translation))
        .collect();
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].0, found[0].0);
    assert_eq!(moved[0].2, Vec3::new(15.0, 0.7, 8.0));

    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .nodes
        .clear();
    app.update();
    assert_eq!(query.iter(app.world()).count(), 0);
}
