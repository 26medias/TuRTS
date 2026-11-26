//! The movement probe (feature `test-probes`, pre-MP1 task Q1b) on a crowd.
//!
//! The four packages of the frozen M2 stream never use a pool or a plain progress (the counts of
//! `refreeze_classify.py --counts`), so the stream cannot show that the probe works. These cases
//! do: each probe value that the trace emitter reads is made non-trivial here, on the authored
//! `muster` package. The flags `separated` and `was_projected` of the old movement path's hard
//! projection are gone with that path (1962b8d3); the flock has no such pass.
#![cfg(feature = "test-probes")]

use turts_mod_api::{ContentId, OrderDef, PackageId, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::movement::probe;
use turts_sim::{
    Command, CommandKind, CommandQueue, MoveTarget, OrderCommand, OrderTargetSim, QueueMode,
    SlotId, UnitId, Vec2, apply_command, headless_app, install_package, stage_package,
};

struct Muster {
    app: bevy_app::App,
    catalog: turts_mod_api::ContentCatalog,
}

fn muster() -> Muster {
    let package_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive = PackageArchive::parse(&pack_dir(&package_dir).expect("Muster packs"))
        .expect("Muster archive parses");
    let catalog = archive.content().expect("Muster catalog resolves");
    let (map_file, map) = archive
        .maps()
        .expect("Muster maps parse")
        .into_iter()
        .find(|(_, map)| map.id == "field")
        .expect("Muster field map exists");
    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: map_file,
            value: map,
        },
        20,
    )
    .expect("the authored Muster map stages");
    install_package(app.world_mut(), staged);
    Muster { app, catalog }
}

fn group_move(catalog: &turts_mod_api::ContentCatalog, units: Vec<u32>, point: Vec2) -> Command {
    let (order, definition): (ContentId, &OrderDef) = catalog
        .orders
        .iter()
        .find(|(id, _)| id.package.0 == "muster" && id.local == "order_move")
        .map(|(id, definition)| (id.clone(), definition))
        .expect("Muster order_move exists");
    Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order,
            behaviour: definition.behaviour,
            target_kind: definition.target,
            policy: None,
            units,
            target: OrderTargetSim::Point(point),
            queue: QueueMode::Replace,
            group: 5,
            allowed_units: definition.units.clone(),
            queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
        })),
    }
}

fn queue(app: &mut bevy_app::App, command: Command) {
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(command);
}

fn move_to(unit_ids: Vec<u32>, target: Vec2) -> Command {
    Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::MoveTo { unit_ids, target },
    }
}

fn has_move_target(app: &mut bevy_app::App, unit: u32) -> bool {
    app.world_mut()
        .query::<(&UnitId, &MoveTarget)>()
        .iter(app.world())
        .any(|(id, _)| id.0 == unit)
}

/// A group order keeps one pool, and each member reads the pool state while it moves.
#[test]
fn a_routed_group_keeps_pool_progress() {
    let mut fixture = muster();
    let ids: Vec<u32> = (1..=8).collect();
    let command = group_move(&fixture.catalog, ids.clone(), Vec2::new(40.0, 30.0));
    let app = &mut fixture.app;
    let position = |world: &bevy_ecs::world::World, id| {
        let entity = turts_sim::unit_entity(world, id).expect("Muster unit exists");
        world.get::<turts_sim::Position>(entity).unwrap().0
    };
    queue(app, command);

    for _ in 0..2 {
        app.update();
        let world = app.world();
        assert_eq!(probe::pool_count(world), 1, "one pool for one group order");
        for (i, &id) in ids.iter().enumerate() {
            let pose = position(world, id);
            assert_eq!(probe::unit_state(world, id), "pool", "unit {id}");
            assert_eq!(probe::activity(world, id), "Moving", "unit {id}");
            for &other in &ids[i + 1..] {
                assert!(pose.distance(position(world, other)) >= 1.0 - 1e-5);
            }
        }
        assert_eq!(probe::unit_state(world, 100), "none");
    }
}

/// A unit that does not step reads the defaults, and a unit that does not exist reads `none`.
#[test]
fn a_unit_that_does_not_step_reads_the_defaults() {
    let mut fixture = muster();
    let app = &mut fixture.app;
    app.update();
    let world = app.world();
    assert_eq!(probe::unit_state(world, 1), "none");
    assert_eq!(probe::activity(world, 1), "none");
    assert_eq!(probe::pool_count(world), 0);
    assert_eq!(probe::pending_movement_orders(world), 0);
    assert_eq!(probe::unit_state(world, 999_999), "none");
    assert_eq!(probe::activity(world, 999_999), "none");
}

/// The emitter tells the route of a `MoveTo` from the world right after the command: a single
/// unit gets its `MoveTarget` at once (`direct`), more than one unit wait in a pending envelope
/// (`command.rs` takes that route for `unit_ids.len() > 1`). The pending count is in units.
#[test]
fn the_pending_count_and_the_move_target_tell_the_move_to_route() {
    let mut fixture = muster();
    let app = &mut fixture.app;
    let result = apply_command(app.world_mut(), &move_to(vec![1], Vec2::new(60.0, 18.0)));
    assert!(result.outcome.is_ok(), "{result:?}");
    assert!(
        has_move_target(app, 1),
        "direct: the unit has its MoveTarget at once"
    );
    assert_eq!(
        probe::pending_movement_orders(app.world()),
        0,
        "direct: nothing pending"
    );

    let result = apply_command(
        app.world_mut(),
        &move_to(vec![2, 3, 4], Vec2::new(60.0, 30.0)),
    );
    assert!(result.outcome.is_ok(), "{result:?}");
    assert_eq!(
        probe::pending_movement_orders(app.world()),
        3,
        "per_unit: three units wait in the envelope"
    );
    assert!(
        !has_move_target(app, 3),
        "per_unit: no MoveTarget before the Orders phase"
    );
    app.update();
    assert_eq!(
        probe::pending_movement_orders(app.world()),
        0,
        "the Orders phase of the next tick takes the envelope"
    );
}
