//! The resting state of a group after an order (pre-MP1 slice, task A1c repair, fault N3).
//!
//! The movement system (`movement/ecs.rs`, `integrate_movement`) gives the step function the set
//! of units that keep their resting progress: the units whose current order is an Attack with a
//! point (`step_frame_preserving_resting`, `movement/frame.rs`: a resting unit loses its progress
//! when its pool is done, unless it is in this set or follows a unit). The rule of the order
//! code (`orders/run.rs`, the attack-move resume rule) reads that state: a unit that is resting at
//! its slot is not sent again.
//!
//! The replay validator computed this transition again and was red when the set was built for
//! another behaviour (`mc_orders.rs:566`, "movement transition changed nonmovement state"). The
//! ledger audit has no movement model, so these two outcome tests hold the rule on the authored
//! `muster` package (4 troopers; a larger group blocks itself, and the rule needs no crowd).
//!
//! The arrival window of a member is a tick end where its current order is still the Attack or
//! the Move with a point and it has no `MoveTarget`: the movement system landed it in this tick,
//! and the order code completes the order in the next tick.
//! - Attack with a point (`rush`): each member in its window holds `MoveProgress` with the
//!   activity `Resting` at its slot (the set keeps it), also the last one, whose landing ends
//!   the pool. If the progress were dropped, the resume rule of `orders/run.rs` would send the
//!   member again to the order point.
//! - Move (`order_move`): the last member in its window holds no progress (it is not in the set),
//!   and at the end the group holds no `MoveProgress`, no `MoveTarget` and no pool.

use turts_mod_api::{Behaviour, ContentId, OrderDef, PackageId, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::movement::{MoveProgress, MovementActivity, MovementPools};
use turts_sim::{
    Command, CommandKind, CommandQueue, MoveTarget, OrderCommand, OrderTargetSim, Position,
    QueueMode, SlotId, Tick, UnitId, UnitOrders, Vec2, headless_app, install_package,
    stage_package,
};

const POINT: Vec2 = Vec2::new(40.0, 30.0);
const END_TICK: u32 = 300;

fn muster() -> (bevy_app::App, turts_mod_api::ContentCatalog) {
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
    (app, catalog)
}

fn order(
    catalog: &turts_mod_api::ContentCatalog,
    local: &str,
    units: Vec<u32>,
    expected: Behaviour,
) -> Command {
    let (id, definition): (ContentId, &OrderDef) = catalog
        .orders
        .iter()
        .find(|(id, _)| id.package.0 == "muster" && id.local == local)
        .map(|(id, definition)| (id.clone(), definition))
        .expect("the Muster order exists");
    assert_eq!(definition.behaviour, expected, "{local} has its behaviour");
    Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: id,
            behaviour: definition.behaviour,
            target_kind: definition.target,
            policy: None,
            units,
            target: OrderTargetSim::Point(POINT),
            queue: QueueMode::Replace,
            group: 1,
            allowed_units: definition.units.clone(),
            queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
        })),
    }
}

/// What the test sees of a member at the end of a tick.
struct Row {
    id: u32,
    position: Vec2,
    orders: UnitOrders,
    progress: Option<MoveProgress>,
    moving: bool,
}

fn rows(app: &mut bevy_app::App, members: &[u32]) -> Vec<Row> {
    let mut rows: Vec<Row> = app
        .world_mut()
        .query::<(
            &UnitId,
            &Position,
            &UnitOrders,
            Option<&MoveProgress>,
            Option<&MoveTarget>,
        )>()
        .iter(app.world())
        .filter(|(id, ..)| members.contains(&id.0))
        .map(|(id, position, orders, progress, target)| Row {
            id: id.0,
            position: position.0,
            orders: orders.clone(),
            progress: progress.cloned(),
            moving: target.is_some(),
        })
        .collect();
    rows.sort_by_key(|row| row.id);
    rows
}

/// What one run observed. The arrival window of a member is a tick end at which the order is
/// still its current Attack or Move with a point and the member has no `MoveTarget`: the
/// movement system has landed the unit in this tick, and the order code completes the order in
/// the next tick. The window is where the set of the movement system acts.
struct Run {
    app: bevy_app::App,
    members: Vec<u32>,
    /// (member, progress) at the tick ends in the arrival window.
    window: Vec<(u32, Option<MoveProgress>, Vec2)>,
}

/// Run `END_TICK` ticks with the command at tick 1.
fn run(command: Command, behaviour: Behaviour, members: &[u32]) -> Run {
    let (mut app, _) = muster();
    let mut window = Vec::new();
    while app.world().resource::<Tick>().0 < END_TICK {
        if app.world().resource::<Tick>().0 == 0 {
            app.world_mut()
                .resource_mut::<CommandQueue>()
                .0
                .push_back(command.clone());
        }
        app.update();
        for row in rows(&mut app, members) {
            let in_window = row.orders.current.as_ref().is_some_and(|order| {
                order.behaviour == behaviour && matches!(order.target, OrderTargetSim::Point(_))
            }) && !row.moving;
            if in_window {
                window.push((row.id, row.progress, row.position));
            }
        }
    }
    Run {
        app,
        members: members.to_vec(),
        window,
    }
}

#[test]
fn a_member_that_lands_after_a_point_attack_keeps_its_resting_progress() {
    let (_, catalog) = muster();
    let members: Vec<u32> = (21..=24).collect();
    let mut run = run(
        order(&catalog, "rush", members.clone(), Behaviour::Attack),
        Behaviour::Attack,
        &members,
    );
    // Every member lands, so every member has one arrival window at least.
    let landed: std::collections::BTreeSet<u32> = run.window.iter().map(|(id, ..)| *id).collect();
    assert_eq!(
        landed.len(),
        members.len(),
        "each member has an arrival window: {landed:?}"
    );
    for (id, progress, position) in &run.window {
        let progress = progress.as_ref().unwrap_or_else(|| {
            panic!("unit {id} landed after an attack-move with no MoveProgress")
        });
        assert_eq!(
            progress.activity,
            MovementActivity::Resting,
            "unit {id} rests"
        );
        assert_eq!(*position, progress.slot, "unit {id} rests at its slot");
    }
    // The attack-move completes at the slot: no member is sent again to the order point, so
    // the members stand on distinct slots (no two bodies overlap) and hold no target.
    let end = rows(&mut run.app, &run.members);
    for row in &end {
        assert!(!row.moving, "unit {} rests: it has no MoveTarget", row.id);
        assert!(
            row.orders.current.is_none(),
            "unit {} completed its attack-move",
            row.id
        );
        assert!(
            row.orders.last_failure.is_none(),
            "unit {} has no failure",
            row.id
        );
    }
    for (index, left) in end.iter().enumerate() {
        for right in &end[index + 1..] {
            assert!(
                left.position.distance(right.position) >= 0.99,
                "units {} and {} overlap at rest",
                left.id,
                right.id
            );
        }
    }
}

#[test]
fn a_member_that_lands_after_a_move_holds_no_progress_and_the_pool_goes() {
    let (_, catalog) = muster();
    let members: Vec<u32> = (21..=24).collect();
    let mut run = run(
        order(&catalog, "order_move", members.clone(), Behaviour::Move),
        Behaviour::Move,
        &members,
    );
    // A Move is not in the set of the movement system: the last member that lands ends the pool,
    // and then no resting progress stays. So at least one arrival window shows no progress.
    assert!(
        run.window.iter().any(|(_, progress, _)| progress.is_none()),
        "no member landed with its resting progress dropped: {:?}",
        run.window.len()
    );
    let held: Vec<u32> = rows(&mut run.app, &run.members)
        .iter()
        .filter(|row| row.progress.is_some() || row.moving)
        .map(|row| row.id)
        .collect();
    assert_eq!(
        held,
        Vec::<u32>::new(),
        "a unit at rest holds no MoveProgress and no MoveTarget"
    );
    assert!(
        run.app.world().resource::<MovementPools>().0.is_empty(),
        "the pool of a finished Move group is gone"
    );
    for row in rows(&mut run.app, &run.members) {
        assert!(
            row.orders.current.is_none(),
            "unit {} completed its move",
            row.id
        );
        assert!(
            row.position.distance(POINT) < 14.0,
            "unit {} reached the point: {:?}",
            row.id,
            row.position
        );
    }
}
