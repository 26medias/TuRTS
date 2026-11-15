use super::*;
use turts_mod_api::{ResolvedTerrainDef, UnitTypeId};
use turts_rules::terrain::{
    geometry::Cell,
    material::{Column, NaturalInterval},
};
fn terrain() -> TerrainState {
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    super::super::prepare(
        &archive.content().unwrap(),
        &ResolvedTerrainDef {
            length_scale: 1000,
            cell_width: 1000,
            elevation_step: 250,
            columns: 48,
            rows: 48,
            chunk_width: 4,
            initial_elevation: 0,
            initial_heights: vec![],
            floor: 0,
            ceiling: 8,
            generation: vec![],
        },
        super::super::PrivateTerrainSeed::new(1),
    )
    .unwrap()
}
fn exact(terrain: &TerrainState, position: Vec2, radius: f32, step: u32) -> Support {
    super::super::exact_support(terrain, position, radius, step).unwrap()
}
// Unit-test truth edits exercise cache invalidation only. Public terrain acceptance uses
// ordinary authored work commands; these edits are not gameplay or conservation evidence.
fn change(terrain: &mut TerrainState, cell: Cell, height: i32) {
    let column = Column::new(
        0,
        8,
        height,
        if height == 0 {
            vec![]
        } else {
            vec![NaturalInterval {
                bottom: 0,
                top: height,
                material: 0,
            }]
        },
    )
    .unwrap();
    let edit = terrain.prepare_cell(cell, column).unwrap();
    terrain.apply_prevalidated(edit);
}
#[test]
fn stationary_moving_kind_radius_and_profile_keys_match_exact_support() {
    let terrain = terrain();
    let mut cache = Cache::default();
    cache.scope(&terrain);
    for (index, (position, radius, step, kind)) in [
        (Vec2::new(5.5, 5.5), 0.25, 0, 0),
        (Vec2::new(5.5, 5.5), 0.25, 0, 0),
        (Vec2::new(6.25, 5.5), 0.25, 0, 0),
        (Vec2::new(6.25, 5.5), 1.25, 0, 0),
        (Vec2::new(6.25, 5.5), 1.25, 1, 0),
        (Vec2::new(6.25, 5.5), 1.25, 1, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let key = Key::new(UnitTypeId(kind), position, radius, step);
        let support = exact(&terrain, position, radius, step);
        if index == 1 {
            assert_eq!(cache.get(&terrain, 7, key), Some(support.maximum));
        } else {
            assert_eq!(cache.get(&terrain, 7, key), None);
        }
        cache.insert(&terrain, 7, key, &support);
        assert_eq!(cache.get(&terrain, 7, key), Some(support.maximum));
    }
    assert_ne!(
        Key::new(UnitTypeId(0), Vec2::new(0.0, 1.0), 0.25, 0).0,
        Key::new(UnitTypeId(0), Vec2::new(-0.0, 1.0), 0.25, 0).0
    );
}
#[test]
fn relevant_actual_edits_invalidate_but_distant_edits_keep_exact_cached_result() {
    let mut terrain = terrain();
    let mut cache = Cache::default();
    cache.scope(&terrain);
    let position = Vec2::new(5.5, 5.5);
    let key = Key::new(UnitTypeId(0), position, 0.25, 0);
    cache.insert(&terrain, 7, key, &exact(&terrain, position, 0.25, 0));
    change(&mut terrain, Cell { x: 40, y: 40 }, 1);
    assert_eq!(
        cache.get(&terrain, 7, key),
        Some(exact(&terrain, position, 0.25, 0).maximum)
    );
    change(&mut terrain, Cell { x: 5, y: 5 }, 1);
    assert_eq!(cache.get(&terrain, 7, key), None);
    let support = exact(&terrain, position, 0.25, 0);
    assert_eq!(support.maximum, 1);
    cache.insert(&terrain, 7, key, &support);
    assert_eq!(cache.get(&terrain, 7, key), Some(1));
}
#[test]
fn one_map_identity_resets_equal_grids_and_does_not_pin_old_allocations() {
    let mut terrain = terrain();
    let mut cache = Cache::default();
    cache.scope(&terrain);
    let position = Vec2::new(5.5, 5.5);
    let key = Key::new(UnitTypeId(0), position, 0.25, 0);
    cache.insert(&terrain, 7, key, &exact(&terrain, position, 0.25, 0));
    let old = Arc::clone(&terrain.initial);
    terrain.initial = Arc::from(terrain.initial.to_vec());
    cache.scope(&terrain);
    assert!(cache.entries.is_empty());
    assert_eq!(
        Arc::strong_count(&old),
        1,
        "old map must not remain pinned by a cache entry"
    );
    cache.insert(&terrain, 7, key, &exact(&terrain, position, 0.25, 0));
    terrain.grid.elevation_step = 500;
    cache.scope(&terrain);
    assert!(cache.entries.is_empty());
}
#[test]
fn entry_and_local_lease_pressure_skip_storage_without_changing_geometry() {
    let terrain = terrain();
    let mut cache = Cache::default();
    cache.scope(&terrain);
    let position = Vec2::new(5.5, 5.5);
    let key = Key::new(UnitTypeId(0), position, 0.25, 0);
    let support = exact(&terrain, position, 0.25, 0);
    for unit in 0..=ENTRIES as u32 {
        cache.insert(&terrain, unit, key, &support);
    }
    assert_eq!(cache.entries.len(), ENTRIES);
    assert_eq!(cache.get(&terrain, ENTRIES as u32, key), None);
    assert_eq!(exact(&terrain, position, 0.25, 0).maximum, support.maximum);
    let large = exact(&terrain, Vec2::splat(24.0), 21.0, 0);
    let chunks = large
        .cells
        .iter()
        .map(|&cell| terrain.chunk_location(cell).unwrap().0)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        chunks.len() > CHUNKS,
        "fixture must actually exceed the local lease cap"
    );
    let mut empty = Cache::default();
    empty.scope(&terrain);
    empty.insert(&terrain, 9, key, &large);
    assert!(empty.entries.is_empty());
    assert_eq!(large.maximum, 0);
}
#[test]
fn missing_contended_or_poisoned_optional_cache_is_an_exact_miss() {
    let mut world = World::new();
    world.insert_resource(terrain());
    let key = Key::new(UnitTypeId(0), Vec2::splat(5.5), 0.25, 0);
    assert_eq!(
        lookup(&world, world.resource::<TerrainState>(), 1, key),
        None
    );
    world.insert_resource(GroundSupportCache::default());
    let resource = world.resource::<GroundSupportCache>();
    let held = resource.0.lock().unwrap();
    assert_eq!(
        lookup(&world, world.resource::<TerrainState>(), 1, key),
        None
    );
    drop(held);
    let _ = std::panic::catch_unwind(|| {
        let _held = resource.0.lock().unwrap();
        panic!("unit-test cache poison");
    });
    assert_eq!(
        lookup(&world, world.resource::<TerrainState>(), 1, key),
        None
    );
    assert_eq!(
        exact(world.resource::<TerrainState>(), Vec2::splat(5.5), 0.25, 0).maximum,
        0
    );
}
#[test]
fn unit_index_prunes_dead_rows_once_per_tick() {
    let mut world = World::new();
    world.insert_resource(terrain());
    world.insert_resource(GroundSupportCache::default());
    world.insert_resource(crate::UnitIndex::default());
    world.insert_resource(crate::Tick(0));
    let entity = world.spawn(crate::UnitId(7)).id();
    let position = Vec2::splat(5.5);
    let key = Key::new(UnitTypeId(0), position, 0.25, 0);
    let terrain = world.resource::<TerrainState>();
    let support = exact(terrain, position, 0.25, 0);
    remember(&world, terrain, 7, key, &support);
    assert_eq!(lookup(&world, terrain, 7, key), Some(0));
    world.despawn(entity);
    world.resource_mut::<crate::Tick>().0 = 1;
    assert_eq!(
        lookup(&world, world.resource::<TerrainState>(), 7, key),
        None
    );
    assert!(
        world
            .resource::<GroundSupportCache>()
            .0
            .lock()
            .unwrap()
            .entries
            .is_empty()
    );
}

#[test]
fn public_ground_capture_matches_uncached_geometry_across_motion_and_type_changes() {
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let catalog = archive.content().unwrap();
    let types = crate::UnitTypes::from_catalog(&catalog, &crate::UnitTypes::default()).unwrap();
    let worker = types.find("base:harvester").unwrap();
    let building = types.find("base:command_center").unwrap();
    let mut world = World::new();
    world.insert_resource(terrain());
    world.insert_resource(types);
    world.insert_resource(crate::bootstrap::InstalledCatalog(catalog));
    world.insert_resource(crate::UnitIndex::default());
    world.insert_resource(crate::Tick(0));
    world.insert_resource(GroundSupportCache::default());
    let entity = world
        .spawn((
            crate::UnitId(7),
            crate::UnitType(worker),
            crate::Position(Vec2::splat(8.0)),
        ))
        .id();
    for (kind, position) in [
        (worker, Vec2::splat(8.0)),
        (worker, Vec2::splat(8.0)),
        (worker, Vec2::new(9.25, 8.0)),
        (building, Vec2::splat(16.0)),
        (worker, Vec2::new(-1.0, 8.0)),
    ] {
        world
            .entity_mut(entity)
            .insert((crate::UnitType(kind), crate::Position(position)));
        let expected = super::super::support_elevation(
            world.resource::<TerrainState>(),
            &world.resource::<crate::bootstrap::InstalledCatalog>().0,
            world.resource::<crate::UnitTypes>(),
            kind,
            position,
        )
        .ok();
        assert_eq!(super::super::ground_elevation(&world, 7), expected);
        assert_eq!(super::super::ground_elevation(&world, 7), expected);
    }
}
