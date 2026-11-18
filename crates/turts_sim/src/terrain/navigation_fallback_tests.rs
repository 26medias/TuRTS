use super::*;
use crate::{
    Command, CommandKind, SlotId, apply_command,
    orders::{OrderCommand, OrderTargetSim, QueueMode, UnitOrders},
};
use bevy_math::Vec2;
use turts_mod_api::{
    Behaviour, ContentId, PackageId, SourceDef, TargetKind, TerrainCell, TerrainRect,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_rules::terrain::geometry::{Budget, Cell, Point, Surface};
fn exercise(package: &str, rows: u32, expect_crossing: bool) {
    let stockyard = package == "terrain_stockyard";
    let id = |local: &str| ContentId::parse(&format!("{package}:{local}")).unwrap();
    let ridge_x = if stockyard { 11 } else { 15 };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(package);
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let catalog = archive.content().unwrap();
    let (file, mut map) = archive.maps().unwrap().remove(0);
    if !stockyard {
        // This test needs a scout on the far side without changing the shipped map.
        let mut scout = map.placements[0].clone();
        scout.pos = turts_mod_api::WorldPos { x: 17.5, y: 19.5 };
        scout.contents.clear();
        map.placements.push(scout);
    }
    let resolved = turts_mod_runtime::terrain::validate_terrain_map(
        &PackageId(package.into()),
        &file,
        &map,
        &catalog,
    )
    .unwrap();
    let mut app = crate::headless_app(20);
    let staged = crate::stage_package_with_terrain(
        app.world(),
        catalog,
        SourceDef { file, value: map },
        20,
        id("field"),
        resolved,
        Some(super::super::PrivateTerrainSeed::new(20261115)),
    )
    .unwrap();
    crate::install_package(app.world_mut(), staged);
    crate::visibility::initialize_context(app.world_mut(), 77, &[0, 1]).unwrap();
    let at = |point: Vec2| {
        app.world()
            .iter_entities()
            .find_map(|entity| {
                (entity.get::<crate::Position>()?.0 == point)
                    .then(|| entity.get::<crate::UnitId>().unwrap().0)
            })
            .unwrap()
    };
    let worker = at(if stockyard {
        Vec2::new(20.0, 16.0)
    } else {
        Vec2::new(13.0, 8.0)
    });
    let crawler = at(if stockyard {
        Vec2::new(18.0, 20.0)
    } else {
        Vec2::new(11.0, 10.0)
    });
    let depot = at(if stockyard {
        Vec2::new(8.0, 8.0)
    } else {
        Vec2::new(4.0, 4.0)
    });
    let scout = (!stockyard).then(|| at(Vec2::new(17.5, 19.5)));
    let mut seq = 0;
    let mut command = |world: &mut World, unit, behaviour, target, queue| {
        seq += 1;
        let (name, target_kind) = match &target {
            OrderTargetSim::Point(_) => ("order_move", TargetKind::Point),
            OrderTargetSim::Unit(_) => ("order_unload", TargetKind::Unit),
            OrderTargetSim::TerrainWork { .. } => ("order_work", TargetKind::TerrainDesignation),
            _ => unreachable!(),
        };
        let result = apply_command(
            world,
            &Command {
                slot: SlotId(0),
                seq,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: id(name),
                    behaviour,
                    target_kind,
                    policy: None,
                    units: vec![unit],
                    target,
                    queue,
                    group: 0,
                    allowed_units: vec![],
                    queue_depth: 16,
                })),
            },
        );
        assert!(
            result.outcome.is_ok(),
            "ordinary fixture command failed: {result:?}"
        );
    };
    let goal = if stockyard {
        Vec2::new(31.000_002, 18.999_989)
    } else {
        Vec2::new(23.000_002, if rows == 2 { 7.999_989_5 } else { 7.5 })
    };
    command(
        app.world_mut(),
        crawler,
        Behaviour::Move,
        OrderTargetSim::Point(goal),
        QueueMode::Replace,
    );
    for _ in 0..40 {
        app.update();
    }
    let current = |world: &World, unit| {
        world
            .get::<crate::Position>(crate::unit_entity(world, unit).unwrap())
            .unwrap()
            .0
    };
    assert!(
        current(app.world(), crawler).x < if stockyard { 21.2 } else { 14.2 },
        "closed ridge must not be crossed"
    );
    if !stockyard {
        // OC requires every fine terrain cell overlapping a coarse fog cell
        // to be current. The old pre-OC probe had no right-side sight of the
        // far ground beside this cut. Move the existing scout ordinarily.
        let scout = scout.expect("existing own right-side scout");
        let cut_current = |world: &World| {
            (7..7 + rows).all(|y| {
                (ridge_x..ridge_x + 2).all(|x| {
                    super::super::step::observed(
                        world,
                        0,
                        world.resource::<TerrainState>(),
                        Cell { x, y },
                        turts_mod_api::ObservationPhase::Tactical,
                    )
                })
            })
        };
        eprintln!(
            "Quarry cut current before ordinary scout move: {}",
            cut_current(app.world())
        );
        command(
            app.world_mut(),
            scout,
            Behaviour::Move,
            OrderTargetSim::Point(Vec2::new(17.5, 13.5)),
            QueueMode::Replace,
        );
        for _ in 0..80 {
            app.update();
            if current(app.world(), scout) == Vec2::new(17.5, 13.5) && cut_current(app.world()) {
                break;
            }
        }
        eprintln!(
            "Quarry cut current after ordinary scout move: {}, scout={:?}",
            cut_current(app.world()),
            current(app.world(), scout)
        );
        assert!(
            cut_current(app.world()),
            "ordinary scout must earn current cut coverage"
        );
        assert!(
            current(app.world(), crawler).x < 14.2,
            "new sight alone cannot cross the still-closed ridge"
        );
    }
    command(
        app.world_mut(),
        worker,
        Behaviour::TerrainWork,
        OrderTargetSim::TerrainWork {
            tool: id(if stockyard { "scraper" } else { "spade" }),
            cells: TerrainRect {
                min: TerrainCell { x: ridge_x, y: 7 },
                max: TerrainCell {
                    x: ridge_x + 2,
                    y: 7 + rows,
                },
            },
            target_elevation: 0,
        },
        QueueMode::Replace,
    );
    let open = |world: &World| {
        (7..7 + rows).all(|y| {
            (ridge_x..ridge_x + 2)
                .all(|x| world.resource::<TerrainState>().elevation(Cell { x, y }) == Some(0))
        })
    };
    let mut unloads = 0;
    for _ in 0..2400 {
        app.update();
        if open(app.world()) {
            break;
        }
        let orders = app
            .world()
            .get::<UnitOrders>(crate::unit_entity(app.world(), worker).unwrap())
            .unwrap();
        let working = orders
            .current
            .as_ref()
            .is_some_and(|order| order.behaviour == Behaviour::TerrainWork);
        let full = app
            .world()
            .resource::<super::super::work::TerrainWorkRuntime>()
            .units
            .get(&worker)
            .is_some_and(|state| state.wait == super::super::TerrainWorkWait::CargoFull);
        let loaded = app
            .world()
            .resource::<crate::EconomyLedger>()
            .inventories
            .get(&worker)
            .is_some_and(|inventory| inventory.items.values().any(|account| account.on_hand > 0));
        if working && full && loaded {
            command(
                app.world_mut(),
                worker,
                Behaviour::UnloadCargo,
                OrderTargetSim::Unit(depot),
                QueueMode::Append,
            );
            unloads += 1;
        }
    }
    assert!(
        open(app.world()),
        "ordinary excavation/unload loop must open the authored cells: worker={:?} crawler={:?} orders={:?} target={:?} work={:?} unloads={unloads} nav={:?}",
        current(app.world(), worker),
        current(app.world(), crawler),
        app.world()
            .get::<UnitOrders>(crate::unit_entity(app.world(), worker).unwrap()),
        app.world()
            .get::<crate::MoveTarget>(crate::unit_entity(app.world(), worker).unwrap()),
        app.world()
            .resource::<super::super::work::TerrainWorkRuntime>()
            .units
            .get(&worker),
        super::super::navigation::privileged_navigation_audit(app.world())
    );
    if !stockyard {
        assert!(unloads > 0);
    }
    let settled_worker = current(app.world(), worker);
    // Clear the feasible passage by an ordinary worker move. The impossible
    // one-cell case retains its settled worker and must never evict that body.
    if expect_crossing {
        command(
            app.world_mut(),
            worker,
            Behaviour::Move,
            OrderTargetSim::Point(if stockyard {
                Vec2::new(18.0, 12.0)
            } else {
                Vec2::new(12.0, 5.0)
            }),
            QueueMode::Replace,
        );
    }
    let mut reached = false;
    for _ in 0..2400 {
        app.update();
        let position = current(app.world(), crawler);
        if position == goal {
            reached = true;
            break;
        }
    }
    let position = current(app.world(), crawler);
    if expect_crossing && !reached {
        let entity = crate::unit_entity(app.world(), crawler).unwrap();
        let route = debug_field_status(app.world());
        eprintln!(
            "movement diagnostic package={package}: worker={:?} order={:?} target={:?} progress={:?} plain={:?} route={route:?}",
            current(app.world(), worker),
            app.world().get::<UnitOrders>(entity),
            app.world().get::<crate::world::MoveTarget>(entity),
            app.world().get::<crate::movement::MoveProgress>(entity),
            app.world()
                .get::<crate::movement::PlainMoveProgress>(entity)
        );
    }
    if expect_crossing {
        assert!(
            reached,
            "crawler must resume its ORIGINAL move through legal continuous opening: pos={position:?},goal={goal:?},unloads={unloads}"
        );
    } else {
        assert!(
            !reached && position.x < 14.2,
            "the same full footprint must still refuse the one-cell opening: {position:?}"
        );
        assert_eq!(
            current(app.world(), worker),
            settled_worker,
            "a settled body cannot be evicted to force progress"
        );
        let truth = app.world().resource::<TerrainState>();
        assert!(
            truth
                .grid
                .disk_support(
                    truth,
                    Point { x: 15.5, y: 7.5 },
                    0.85,
                    0,
                    &mut Budget::new(16_384)
                )
                .is_err(),
            "one-cell opening must be geometrically too narrow even without the worker"
        );
    }
}
#[test]
fn public_quarry_work_opens_continuous_route_and_retained_move_reaches_unsnapped_goal() {
    exercise("quarry", 2, true);
}
#[test]
fn public_quarry_one_cell_opening_still_refuses_the_wide_footprint() {
    exercise("quarry", 1, false);
}

#[test]
fn public_stockyard_corner_approach_keeps_its_vertex_until_next_connector_is_clear() {
    exercise("terrain_stockyard", 2, true);
}

// Frozen ef619 browser cut: keep authored cargo, sight/terrain and every body.
// The worker must return past the settled crawler after ordinary unloads.
fn exercise_browser_quarry_worker_cut() {
    let package = "quarry";
    let rows = 2;
    let stockyard = package == "terrain_stockyard";
    let id = |local: &str| ContentId::parse(&format!("{package}:{local}")).unwrap();
    let ridge_x = if stockyard { 11 } else { 15 };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(package);
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let catalog = archive.content().unwrap();
    let (file, map) = archive.maps().unwrap().remove(0);
    let resolved = turts_mod_runtime::terrain::validate_terrain_map(
        &PackageId(package.into()),
        &file,
        &map,
        &catalog,
    )
    .unwrap();
    let mut app = crate::headless_app(20);
    let staged = crate::stage_package_with_terrain(
        app.world(),
        catalog,
        SourceDef { file, value: map },
        20,
        id("field"),
        resolved,
        Some(super::super::PrivateTerrainSeed::new(20261115)),
    )
    .unwrap();
    crate::install_package(app.world_mut(), staged);
    crate::visibility::initialize_context(app.world_mut(), 77, &[0, 1]).unwrap();
    let at = |point: Vec2| {
        app.world()
            .iter_entities()
            .find_map(|entity| {
                (entity.get::<crate::Position>()?.0 == point)
                    .then(|| entity.get::<crate::UnitId>().unwrap().0)
            })
            .unwrap()
    };
    let worker = at(if stockyard {
        Vec2::new(20.0, 16.0)
    } else {
        Vec2::new(13.0, 8.0)
    });
    let crawler = at(if stockyard {
        Vec2::new(18.0, 20.0)
    } else {
        Vec2::new(11.0, 10.0)
    });
    let depot = at(if stockyard {
        Vec2::new(8.0, 8.0)
    } else {
        Vec2::new(4.0, 4.0)
    });
    let mut seq = 0;
    let mut command = |world: &mut World, unit, behaviour, target, queue| {
        seq += 1;
        let (name, target_kind) = match &target {
            OrderTargetSim::Point(_) => ("order_move", TargetKind::Point),
            OrderTargetSim::Unit(_) => ("order_unload", TargetKind::Unit),
            OrderTargetSim::TerrainWork { .. } => ("order_work", TargetKind::TerrainDesignation),
            _ => unreachable!(),
        };
        let result = apply_command(
            world,
            &Command {
                slot: SlotId(0),
                seq,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: id(name),
                    behaviour,
                    target_kind,
                    policy: None,
                    units: vec![unit],
                    target,
                    queue,
                    group: 0,
                    allowed_units: vec![],
                    queue_depth: 16,
                })),
            },
        );
        assert!(
            result.outcome.is_ok(),
            "ordinary fixture command failed: {result:?}"
        );
    };
    command(
        app.world_mut(),
        worker,
        Behaviour::TerrainWork,
        OrderTargetSim::TerrainWork {
            tool: id(if stockyard { "scraper" } else { "spade" }),
            cells: TerrainRect {
                min: TerrainCell { x: ridge_x, y: 10 },
                max: TerrainCell {
                    x: ridge_x + 2,
                    y: 10 + rows,
                },
            },
            target_elevation: 0,
        },
        QueueMode::Replace,
    );
    let open = |world: &World| {
        (10..10 + rows).all(|y| {
            (ridge_x..ridge_x + 2)
                .all(|x| world.resource::<TerrainState>().elevation(Cell { x, y }) == Some(0))
        })
    };
    let mut unloads = 0;
    for _ in 0..1200 {
        app.update();
        if open(app.world()) {
            break;
        }
        let orders = app
            .world()
            .get::<UnitOrders>(crate::unit_entity(app.world(), worker).unwrap())
            .unwrap();
        let working = orders
            .current
            .as_ref()
            .is_some_and(|order| order.behaviour == Behaviour::TerrainWork);
        let full = app
            .world()
            .resource::<super::super::work::TerrainWorkRuntime>()
            .units
            .get(&worker)
            .is_some_and(|state| state.wait == super::super::TerrainWorkWait::CargoFull);
        let loaded = app
            .world()
            .resource::<crate::EconomyLedger>()
            .inventories
            .get(&worker)
            .is_some_and(|inventory| inventory.items.values().any(|account| account.on_hand > 0));
        if working && full && loaded {
            command(
                app.world_mut(),
                worker,
                Behaviour::UnloadCargo,
                OrderTargetSim::Unit(depot),
                QueueMode::Append,
            );
            unloads += 1;
        }
    }
    if !open(app.world()) {
        eprintln!(
            "worker navigation: {:?}",
            super::super::navigation::privileged_navigation_audit(app.world())
                .members
                .into_iter()
                .find(|m| m.unit == worker)
        );
    }
    let entity = crate::unit_entity(app.world(), worker).unwrap();
    assert!(
        open(app.world()),
        "ordinary browser cut must complete: position={:?}, target={:?}, plain={:?}, escape={:?}, work={:?}, unloads={unloads}",
        app.world().get::<crate::Position>(entity),
        app.world().get::<crate::MoveTarget>(entity),
        app.world()
            .get::<crate::movement::PlainMoveProgress>(entity),
        app.world().get::<crate::movement::PhysicalEscape>(entity),
        app.world()
            .resource::<super::super::work::TerrainWorkRuntime>()
            .units
            .get(&worker)
    );
    assert_eq!(
        app.world()
            .get::<crate::Position>(crate::unit_entity(app.world(), crawler).unwrap())
            .unwrap()
            .0,
        bevy_math::Vec2::new(11., 10.),
        "the settled crawler keeps its position"
    );
    assert!(
        unloads >= 3,
        "the regression must exercise the observed repeated unload continuation"
    );
    if !stockyard {
        assert!(unloads > 0);
    }
}

#[test]
fn public_quarry_browser_cut_resumes_after_three_unloads_without_crawler_eviction() {
    exercise_browser_quarry_worker_cut();
}
