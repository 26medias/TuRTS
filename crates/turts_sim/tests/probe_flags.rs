//! The movement probe (feature `test-probes`, pre-MP1 task Q1b) on a crowd.
//!
//! The four packages of the frozen M2 stream never set `separated` or `was_projected` and
//! never use a pool or a plain progress (the counts of `refreeze_classify.py --counts`), so
//! the stream cannot show that the probe works. These cases do: each probe value that the
//! trace emitter reads is made non-trivial here, on the authored `muster` package.
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

/// Trooper 1 stands at (25, 18) with trooper 2 at (26.1, 18): a gap of 1.1, which is more than
/// the separation limit 1.05, so the unit is not `separated` on its first step. The step of 0.15
/// puts it at 0.95 from trooper 2, inside the footprint limit 1.0001, so the hard projection
/// moves it back: `was_projected`. A moved and projected unit gets a plain progress.
#[test]
fn a_unit_stepped_into_a_neighbour_reads_projected_then_plain_and_separated() {
    let mut fixture = muster();
    let app = &mut fixture.app;
    queue(app, move_to(vec![1], Vec2::new(60.0, 18.0)));
    app.update();
    let world = app.world();
    assert_eq!(
        probe::was_projected(world, 1),
        Some(true),
        "tick 1: projected"
    );
    assert!(!probe::separated(world, 1), "tick 1: not separated");
    assert_eq!(
        probe::unit_state(world, 1),
        "plain",
        "tick 1: plain progress"
    );
    assert_eq!(
        probe::activity(world, 1),
        "none",
        "a plain progress is no pool"
    );
    app.update();
    let world = app.world();
    assert!(
        probe::separated(world, 1),
        "tick 2: separated (plain branch)"
    );
    assert_eq!(
        probe::was_projected(world, 1),
        Some(false),
        "tick 2: not projected"
    );
    assert_eq!(
        probe::unit_state(world, 1),
        "plain",
        "tick 2: plain progress"
    );
}

/// A group order gives a pool of one group. Each pooled unit reads `pool`, its pool activity,
/// and a null `was_projected` (the pool branch has no such local). `separated` comes from the
/// pool branch.
#[test]
fn a_pooled_group_reads_pool_activity_null_flag_separated_and_pool_count() {
    let mut fixture = muster();
    let ids: Vec<u32> = (1..=8).collect();
    let command = group_move(&fixture.catalog, ids.clone(), Vec2::new(40.0, 30.0));
    let app = &mut fixture.app;
    queue(app, command);
    app.update();
    let world = app.world();
    assert_eq!(probe::pool_count(world), 1, "one pool for one group order");
    for &id in &ids {
        assert_eq!(probe::unit_state(world, id), "pool", "unit {id}");
        assert_eq!(probe::activity(world, id), "Moving", "unit {id}");
        assert_eq!(probe::was_projected(world, id), None, "unit {id}");
    }
    assert!(
        ids.iter().any(|&id| probe::separated(world, id)),
        "a unit of the packed row is separated in the pool branch"
    );
    assert_eq!(
        probe::unit_state(world, 100),
        "none",
        "a unit outside the order has no movement state"
    );
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
    assert!(!probe::separated(world, 1));
    assert_eq!(probe::was_projected(world, 1), Some(false));
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
