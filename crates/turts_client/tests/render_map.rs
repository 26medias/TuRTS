//! Task 10: the pure functions of presentation and input. No window and no renderer.

use std::collections::HashMap;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use turts_client::acquire::Acquired;
use turts_client::input::ground_point;
use turts_client::join::AcquiredPackages;
use turts_client::present::{Presented, PresentedUnit as PresentedRecord};
use turts_client::render::{PresentedUnit, mesh_for, mesh_lift, sync_units, world_to_scene};
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
