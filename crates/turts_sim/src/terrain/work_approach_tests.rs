use super::*;
use crate::{
    Command, CommandKind, SlotId, apply_command,
    orders::{OrderCommand, QueueMode},
};
use turts_mod_api::{PackageId, SourceDef, TargetKind};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
fn package_app(package: &str) -> bevy_app::App {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(package);
    package_app_at(&path, package)
}
fn package_app_at(path: &std::path::Path, package: &str) -> bevy_app::App {
    let archive = PackageArchive::parse(&pack_dir(path).unwrap()).unwrap();
    let catalog = archive.content().unwrap();
    let (file, mut map) = archive.maps().unwrap().remove(0);
    // Keep the worker cargo empty so these tests isolate movement.
    for placement in &mut map.placements {
        placement.contents.clear();
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
        ContentId::parse(&format!("{package}:field")).unwrap(),
        resolved,
        Some(super::super::PrivateTerrainSeed::new(20261115)),
    )
    .unwrap();
    crate::install_package(app.world_mut(), staged);
    crate::visibility::initialize_context(app.world_mut(), 77, &[0, 1]).unwrap();
    app
}
fn exercise(package: &str, owner: u8, start: bevy_math::Vec2, cell: Cell, tool: &str, target: i32) {
    let mut app = package_app(package);
    let actor = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<Position>()?.0 == start && entity.get::<Owner>()?.0.0 == owner)
                .then(|| entity.get::<UnitId>().unwrap().0)
        })
        .unwrap();
    let before = app
        .world()
        .resource::<TerrainState>()
        .elevation(cell)
        .unwrap();
    let order = OrderCommand {
        order: ContentId::parse(&format!("{package}:order_work")).unwrap(),
        behaviour: Behaviour::TerrainWork,
        target_kind: TargetKind::TerrainDesignation,
        policy: None,
        units: vec![actor],
        target: OrderTargetSim::TerrainWork {
            tool: ContentId::parse(&format!("{package}:{tool}")).unwrap(),
            cells: TerrainRect {
                min: TerrainCell {
                    x: cell.x,
                    y: cell.y,
                },
                max: TerrainCell {
                    x: cell.x + 1,
                    y: cell.y + 1,
                },
            },
            target_elevation: target,
        },
        queue: QueueMode::Replace,
        group: 0,
        allowed_units: vec![],
        queue_depth: 16,
    };
    let accepted = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(owner),
            seq: 1,
            kind: CommandKind::Order(Box::new(order)),
        },
    );
    assert!(
        accepted.outcome.is_ok(),
        "public work order rejected: {accepted:?}"
    );
    let mut moved = false;
    let mut progressed = false;
    let mut first_goal = None;
    for _ in 0..600 {
        app.update();
        let entity = unit_entity(app.world(), actor).unwrap();
        moved |= app.world().get::<Position>(entity).unwrap().0 != start;
        if first_goal.is_none() {
            first_goal = app
                .world()
                .get::<crate::world::MoveTarget>(entity)
                .map(|target| target.0);
        }
        let height = app
            .world()
            .resource::<TerrainState>()
            .elevation(cell)
            .unwrap();
        progressed |= height < before;
        if progressed {
            break;
        }
    }
    let entity = unit_entity(app.world(), actor).unwrap();
    let position = app.world().get::<Position>(entity).unwrap().0;
    let state = app
        .world()
        .resource::<TerrainWorkRuntime>()
        .units
        .get(&actor);
    assert!(
        moved,
        "ordinary work order must approach its cell: package={package},owner={owner},first_goal={first_goal:?},position={position:?},state={state:?}"
    );
    assert!(
        progressed,
        "ordinary work order must make actual terrain progress: package={package},owner={owner},first_goal={first_goal:?},position={position:?},state={state:?}"
    );
    assert!(
        super::super::ground_elevation(app.world(), actor).is_some(),
        "worker must retain complete physical support"
    );
}
#[test]
fn stockyard_west_worker_reaches_and_excavates_the_public_two_unit_cell() {
    exercise(
        "terrain_stockyard",
        0,
        bevy_math::Vec2::new(20.0, 16.0),
        Cell { x: 11, y: 10 },
        "scraper",
        1,
    );
}
#[test]
fn stockyard_east_worker_reaches_and_excavates_without_crossing_the_ridge() {
    exercise(
        "terrain_stockyard",
        1,
        bevy_math::Vec2::new(32.0, 16.0),
        Cell { x: 12, y: 10 },
        "scraper",
        1,
    );
}
#[test]
fn quarry_worker_reaches_and_excavates_at_one_unit_resolution() {
    exercise(
        "quarry",
        0,
        bevy_math::Vec2::new(13.0, 8.0),
        Cell { x: 15, y: 11 },
        "spade",
        3,
    );
}

fn approach_fixture() -> (bevy_app::App, u32, WorkState) {
    let mut app = package_app("terrain_stockyard");
    app.update();
    let unit = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<Position>()?.0 == bevy_math::Vec2::new(20.0, 16.0)
                && entity.get::<Owner>()?.0 == SlotId(0))
            .then(|| entity.get::<UnitId>().unwrap().0)
        })
        .unwrap();
    let order = Order {
        order: ContentId::parse("terrain_stockyard:order_work").unwrap(),
        behaviour: Behaviour::TerrainWork,
        target: OrderTargetSim::TerrainWork {
            tool: ContentId::parse("terrain_stockyard:scraper").unwrap(),
            cells: TerrainRect {
                min: TerrainCell { x: 11, y: 10 },
                max: TerrainCell { x: 13, y: 11 },
            },
            target_elevation: 0,
        },
        group: 0,
    };
    begin_order(app.world_mut(), unit, &order);
    let mut state = app.world().resource::<TerrainWorkRuntime>().units[&unit].clone();
    state.done = 17;
    state.carry = 3;
    state.banked = true;
    (app, unit, state)
}
#[test]
fn approach_retains_pending_endpoint_while_actor_position_changes() {
    let (mut app, unit, mut state) = approach_fixture();
    approach(app.world_mut(), unit, Cell { x: 11, y: 10 }, &mut state);
    let entity = unit_entity(app.world(), unit).unwrap();
    let endpoint = app.world().get::<crate::MoveTarget>(entity).unwrap().0;
    let poses = state.approach.as_ref().unwrap().poses.clone();
    app.world_mut().get_mut::<Position>(entity).unwrap().0 += bevy_math::Vec2::new(0.025, -0.025);
    approach(app.world_mut(), unit, Cell { x: 11, y: 10 }, &mut state);
    assert_eq!(
        app.world().get::<crate::MoveTarget>(entity).unwrap().0,
        endpoint
    );
    assert_eq!(state.approach.as_ref().unwrap().poses, poses);
    assert_eq!((state.done, state.carry, state.banked), (17, 3, true));
}
/// Nora K-1 (a6d54cee): a walk that makes no progress fails its pose after `STUCK_TICKS`
/// approach checks, and the unit takes another pose; one check earlier it keeps the pose.
/// A build without the stuck rule keeps the same endpoint for ever (shown as Approaching).
#[test]
fn approach_takes_another_pose_after_a_walk_with_no_progress() {
    let (mut app, unit, mut state) = approach_fixture();
    let cell = Cell { x: 11, y: 10 };
    let entity = unit_entity(app.world(), unit).unwrap();
    approach(app.world_mut(), unit, cell, &mut state);
    let first = app.world().get::<crate::MoveTarget>(entity).unwrap().0;
    // The unit stands still with its target kept: a pose that it cannot reach.
    for check in 1..STUCK_TICKS {
        approach(app.world_mut(), unit, cell, &mut state);
        assert_eq!(
            app.world()
                .get::<crate::MoveTarget>(entity)
                .map(|target| target.0),
            Some(first),
            "the pose failed early, at check {check}"
        );
    }
    approach(app.world_mut(), unit, cell, &mut state);
    let next = app
        .world()
        .get::<crate::MoveTarget>(entity)
        .map(|target| target.0);
    assert!(
        next.is_some_and(|next| next != first),
        "a walk with no progress kept its pose: {next:?}"
    );
}
/// Nora K-1 (a6d54cee): a walk that the movement ended short of its endpoint (its target is
/// gone) fails that pose at the next check, and the unit takes another pose. A build without
/// the short-walk rule sends the same endpoint again, which the movement ends again.
#[test]
fn approach_takes_another_pose_after_a_walk_that_ended_short() {
    let (mut app, unit, mut state) = approach_fixture();
    let cell = Cell { x: 11, y: 10 };
    let entity = unit_entity(app.world(), unit).unwrap();
    approach(app.world_mut(), unit, cell, &mut state);
    let first = app.world().get::<crate::MoveTarget>(entity).unwrap().0;
    // The movement ends the walk a little way from the endpoint.
    app.world_mut().get_mut::<Position>(entity).unwrap().0 += bevy_math::Vec2::new(0.025, -0.025);
    app.world_mut()
        .entity_mut(entity)
        .remove::<crate::MoveTarget>();
    approach(app.world_mut(), unit, cell, &mut state);
    let next = app
        .world()
        .get::<crate::MoveTarget>(entity)
        .map(|target| target.0);
    assert!(
        next.is_some_and(|next| next != first),
        "a walk that ended short kept its pose: {next:?}"
    );
}
#[test]
fn new_approach_retires_stale_motion_without_erasing_job_and_cancel_drops_candidates() {
    let (mut app, unit, mut state) = approach_fixture();
    let entity = unit_entity(app.world(), unit).unwrap();
    let stale = bevy_math::Vec2::new(3.0, 3.0);
    app.world_mut().entity_mut(entity).insert((
        crate::MoveTarget(stale),
        crate::movement::PlainMoveProgress {
            arrival_yield: false,
            original_point: stale,
            slot: stale,
            pool_slots: vec![],
            gather_goal: None,
            activity: crate::movement::MovementActivity::Blocked,
            stalled_ticks: 100,
            best_distance: 2.0,
            claim_count: 3,
            failure_recorded: false,
        },
    ));
    let order = state.order.clone();
    approach(app.world_mut(), unit, Cell { x: 12, y: 10 }, &mut state);
    assert_ne!(
        app.world().get::<crate::MoveTarget>(entity).unwrap().0,
        stale
    );
    assert!(
        app.world()
            .get::<crate::movement::PlainMoveProgress>(entity)
            .is_none()
    );
    assert_eq!(state.order, order);
    assert_eq!((state.done, state.carry, state.banked), (17, 3, true));
    assert!(state.approach.is_some());
    store(app.world_mut(), unit, state);
    order_changed(app.world_mut(), unit);
    assert!(
        !app.world()
            .resource::<TerrainWorkRuntime>()
            .units
            .contains_key(&unit)
    );
}

#[test]
fn unloading_keeps_retained_approach_with_exact_banked_work_and_carry() {
    let (mut app, unit, mut state) = approach_fixture();
    approach(app.world_mut(), unit, Cell { x: 11, y: 10 }, &mut state);
    let entity = unit_entity(app.world(), unit).unwrap();
    let order = state.order.clone();
    app.world_mut()
        .get_mut::<UnitOrders>(entity)
        .unwrap()
        .current = Some(order.clone());
    store(app.world_mut(), unit, state.clone());
    super::super::pause::suspend(app.world_mut(), unit, &order).unwrap();
    {
        let mut orders = app.world_mut().get_mut::<UnitOrders>(entity).unwrap();
        orders.current = Some(Order {
            order: ContentId::parse("terrain_stockyard:order_unload").unwrap(),
            behaviour: Behaviour::UnloadCargo,
            target: OrderTargetSim::Unit(99),
            group: 0,
        });
        orders.queue.push_front(order.clone());
    }
    run(app.world_mut());
    assert_eq!(
        app.world().resource::<TerrainWorkRuntime>().units[&unit],
        state
    );
    {
        let mut orders = app.world_mut().get_mut::<UnitOrders>(entity).unwrap();
        orders.current = orders.queue.pop_front();
    }
    begin_order(app.world_mut(), unit, &order);
    assert_eq!(
        app.world().resource::<TerrainWorkRuntime>().units[&unit],
        state,
        "resume must keep the frozen approach, refusal history, banked work and fractional carry"
    );
}

fn pending_replacement_survives_work_transition(mode: u8) {
    let (mut app, unit, initial) = approach_fixture();
    let entity = unit_entity(app.world(), unit).unwrap();
    let peer = app
        .world()
        .iter_entities()
        .find_map(|other| {
            (other.get::<Position>()?.0 == bevy_math::Vec2::new(23.0, 25.0)
                && other.get::<Owner>()?.0 == SlotId(0))
            .then(|| other.get::<UnitId>().unwrap().0)
        })
        .unwrap();
    let work = OrderCommand {
        order: initial.order.order.clone(),
        behaviour: Behaviour::TerrainWork,
        target_kind: TargetKind::TerrainDesignation,
        policy: None,
        units: vec![unit],
        target: OrderTargetSim::TerrainWork {
            tool: ContentId::parse("terrain_stockyard:scraper").unwrap(),
            cells: TerrainRect {
                min: TerrainCell { x: 11, y: 10 },
                max: TerrainCell {
                    x: if mode == 3 { 12 } else { 13 },
                    y: 11,
                },
            },
            target_elevation: 1,
        },
        queue: QueueMode::Replace,
        group: 0,
        allowed_units: vec![],
        queue_depth: 16,
    };
    assert_eq!(
        apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::Order(Box::new(work)),
            }
        )
        .units
        .accepted,
        vec![unit]
    );
    run(app.world_mut());
    assert!(
        app.world().resource::<TerrainWorkRuntime>().units[&unit]
            .approach
            .is_some()
    );
    match mode {
        0 | 3 => {
            let mut cut = false;
            for _ in 0..600 {
                app.update();
                cut = app
                    .world()
                    .resource::<TerrainState>()
                    .elevation(Cell { x: 11, y: 10 })
                    == Some(1);
                if cut {
                    break;
                }
            }
            assert!(
                cut,
                "ordinary prior work completes its first cell before replacement"
            );
            assert_eq!(
                app.world()
                    .resource::<TerrainState>()
                    .elevation(Cell { x: 12, y: 10 }),
                Some(2)
            );
        }
        1 => {
            // A retained candidate has been refused; its next selection must
            // yield to the already accepted replacement before changing motion.
            let mut runtime = app.world_mut().resource_mut::<TerrainWorkRuntime>();
            let saved = runtime
                .units
                .get_mut(&unit)
                .unwrap()
                .approach
                .as_mut()
                .unwrap();
            saved.current = None;
            saved.select = true;
        }
        2 => {
            // Legal prepared work pose: entering Working retires approach motion.
            app.world_mut().get_mut::<Position>(entity).unwrap().0 =
                bevy_math::Vec2::new(21.65, 21.0);
        }
        _ => unreachable!(),
    }
    let tick = app.world().resource::<crate::Tick>().0;
    *app.world_mut()
        .resource_mut::<crate::orders::MovementDispatchState>() =
        crate::orders::MovementDispatchState {
            tick: Some(tick),
            start_slot: 0,
            used: 2048,
            cursor_advanced: true,
        };
    let destination = bevy_math::Vec2::new(12.0, 26.0);
    let moved = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("terrain_stockyard:order_move").unwrap(),
                behaviour: Behaviour::Move,
                target_kind: TargetKind::Point,
                policy: None,
                units: vec![unit, peer],
                target: OrderTargetSim::Point(destination),
                queue: QueueMode::Replace,
                group: 77,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        },
    );
    assert_eq!(
        moved.units.accepted.len(),
        2,
        "actual replacement is accepted: {moved:?}"
    );
    crate::orders::run_orders(app.world_mut());
    let pending = app
        .world()
        .resource::<crate::orders::PendingMovementOrders>()
        .clone();
    assert!(pending.pending_member_counts.contains_key(&unit));
    assert_eq!(
        app.world()
            .get::<UnitOrders>(entity)
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .behaviour,
        Behaviour::TerrainWork
    );
    run(app.world_mut());
    assert_eq!(
        *app.world()
            .resource::<crate::orders::PendingMovementOrders>(),
        pending,
        "old work transition {mode} must not erase the accepted replacement envelope"
    );
    app.update();
    assert!(
        !app.world()
            .resource::<crate::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&unit)
    );
    let orders = app.world().get::<UnitOrders>(entity).unwrap();
    assert_eq!(
        orders.current.as_ref().map(|order| order.behaviour),
        Some(Behaviour::Move)
    );
    assert_eq!(
        orders.current.as_ref().map(|order| &order.target),
        Some(&OrderTargetSim::Point(destination))
    );
    assert!(app.world().get::<crate::MoveTarget>(entity).is_some());
}
#[test]
fn public_pending_replacement_survives_work_cell_advance() {
    pending_replacement_survives_work_transition(0);
}
#[test]
fn public_pending_replacement_survives_work_retarget() {
    pending_replacement_survives_work_transition(1);
}
#[test]
fn public_pending_replacement_survives_entering_working() {
    pending_replacement_survives_work_transition(2);
}

#[test]
fn public_pending_replacement_survives_completed_work() {
    pending_replacement_survives_work_transition(3);
}

/// Busy own unit (Clara, 2026-11-24): own units do not push a unit with a work order, on its
/// walk to its cell and while it works there. Own movers cross its line; each tick of the
/// walk, its step lies along its own advice (a push adds a side part), and while it works,
/// it does not move.
#[test]
fn own_movers_do_not_push_a_unit_with_a_work_order() {
    let mut app = package_app("quarry");
    let start = bevy_math::Vec2::new(13.0, 8.0);
    let digger = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<Position>()?.0 == start && entity.get::<Owner>()?.0.0 == 0)
                .then(|| entity.get::<UnitId>().unwrap().0)
        })
        .unwrap();
    let entity = unit_entity(app.world(), digger).unwrap();
    let walker = app.world().get::<crate::world::UnitType>(entity).unwrap().0;
    let order = |units: Vec<u32>, target: OrderTargetSim, work: bool, group: u32| OrderCommand {
        order: ContentId::parse(if work {
            "quarry:order_work"
        } else {
            "quarry:order_move"
        })
        .unwrap(),
        behaviour: if work {
            Behaviour::TerrainWork
        } else {
            Behaviour::Move
        },
        target_kind: if work {
            TargetKind::TerrainDesignation
        } else {
            TargetKind::Point
        },
        policy: None,
        units,
        target,
        queue: QueueMode::Replace,
        group,
        allowed_units: vec![],
        queue_depth: 16,
    };
    let mut seq = 0;
    let mut send = |app: &mut bevy_app::App, command: OrderCommand| {
        seq += 1;
        let result = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq,
                kind: CommandKind::Order(Box::new(command)),
            },
        );
        assert!(result.outcome.is_ok(), "order rejected: {result:?}");
    };
    let work = OrderTargetSim::TerrainWork {
        tool: ContentId::parse("quarry:spade").unwrap(),
        cells: TerrainRect {
            min: TerrainCell { x: 15, y: 11 },
            max: TerrainCell { x: 16, y: 12 },
        },
        target_elevation: 0,
    };
    send(&mut app, order(vec![digger], work, true, 1));
    let position = |app: &bevy_app::App, unit: u32| {
        app.world()
            .get::<Position>(unit_entity(app.world(), unit).unwrap())
            .unwrap()
            .0
    };
    let walking = |app: &bevy_app::App| {
        app.world()
            .get::<crate::world::MoveTarget>(unit_entity(app.world(), digger).unwrap())
            .is_some()
    };
    let busy = |app: &bevy_app::App| {
        app.world()
            .get::<crate::UnitOrders>(unit_entity(app.world(), digger).unwrap())
            .and_then(|orders| orders.current.as_ref())
            .is_some_and(|order| order.behaviour == Behaviour::TerrainWork)
    };
    // Movers cross the line of the walk, and later the cell where the digger works.
    let crossing = |app: &mut bevy_app::App, from: bevy_math::Vec2| {
        let positions: Vec<_> = (0..6)
            .map(|k| from + bevy_math::Vec2::new(0.0, 0.8 * k as f32 - 2.0))
            .collect();
        crate::spawn_synthetic_units(app.world_mut(), walker, &positions).unwrap()
    };
    let walk_movers = crossing(&mut app, bevy_math::Vec2::new(12.0, 9.5));
    send(
        &mut app,
        order(
            walk_movers.clone(),
            OrderTargetSim::Point(bevy_math::Vec2::new(19.0, 9.5)),
            false,
            2,
        ),
    );
    let (mut walk_contacts, mut work_contacts) = (0, 0);
    let mut work_movers = Vec::new();
    for _ in 0..600 {
        let before = position(&app, digger);
        let was_walking = walking(&app);
        let was_busy = busy(&app);
        app.update();
        if !(was_busy && busy(&app)) {
            continue;
        }
        let after = position(&app, digger);
        let contact = |app: &bevy_app::App, movers: &[u32]| {
            movers
                .iter()
                .any(|&unit| position(app, unit).distance(after) < 2.0 * 0.35 + 0.05)
        };
        if was_walking && walking(&app) {
            walk_contacts += usize::from(contact(&app, &walk_movers));
            let advice = crate::terrain::privileged_flock_audit(app.world())
                .members
                .iter()
                .find(|member| member.unit == digger)
                .map(|member| {
                    bevy_math::Vec2::new(
                        f32::from_bits(member.advice[0]),
                        f32::from_bits(member.advice[1]),
                    )
                })
                .unwrap_or_default();
            let step = after - before;
            if let Some(along) = advice.try_normalize() {
                assert!(
                    along.perp_dot(step).abs() <= 1.0e-4 && along.dot(step) >= -1.0e-6,
                    "the walking digger was pushed: step {step:?}, advice {advice:?}"
                );
            }
        }
        if !was_walking && !walking(&app) {
            if work_movers.is_empty() {
                // It works at its cell: movers now walk through its place.
                work_movers = crossing(&mut app, after - bevy_math::Vec2::new(1.5, 0.0));
                send(
                    &mut app,
                    order(
                        work_movers.clone(),
                        OrderTargetSim::Point(after + bevy_math::Vec2::new(5.0, 0.0)),
                        false,
                        3,
                    ),
                );
            } else {
                work_contacts += usize::from(contact(&app, &work_movers));
                assert_eq!(after, before, "the working digger was pushed");
            }
        }
    }
    assert!(!busy(&app), "the dig did not end in 600 ticks");
    // The case proves nothing if no mover came to the digger.
    assert!(walk_contacts > 0, "no mover touched the walking digger");
    assert!(work_contacts > 0, "no mover touched the working digger");
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A tight node at the map edge (Nora N-1, f84bc1d1; Marco's battle400 replay on 431bddac).
/// The public quarry package (1 m cells, a 0.5 m node lattice) with a walker of radius
/// 0.50005 m: the nodes 0.5 m from the north edge are a hair narrower than the body, as the
/// f32 map edge made them in battle400. The field marks them passable (it allows 1e-4) and
/// the exact step onto them is refused. A walker that stands 0.002 m from such a node is
/// sent to its own node first; before f84bc1d1 it turned round it with no approach. A walker sent along that edge must arrive; before
/// f84bc1d1 it turned round such a node with no approach.
#[test]
fn a_walker_along_a_row_of_tight_nodes_at_the_map_edge_arrives() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry");
    let dir = std::env::temp_dir().join(format!("turts-tight-nodes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&source, &dir);
    let edit = |file: &str, from: &str, to: &str| {
        let path = dir.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(from), "{file} has {from}");
        std::fs::write(&path, text.replacen(from, to, 1)).unwrap();
    };
    edit("units/walker.ron", "radius:0.35", "radius:0.50005");
    // The wider walker at (15.5, 13.5) would touch the ridge step; it stands off it.
    edit(
        "maps/field.ron",
        "pos:(x:15.5,y:13.5)",
        "pos:(x:12.5,y:13.5)",
    );
    let mut app = package_app_at(&dir, "quarry");
    let walker = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<Position>()?.0 == bevy_math::Vec2::new(13.0, 8.0)
                && entity.get::<Owner>()?.0.0 == 0)
                .then(|| entity.get::<UnitId>().unwrap().0)
        })
        .unwrap();
    let start = bevy_math::Vec2::new(12.5016, 23.4988);
    let entity = unit_entity(app.world(), walker).unwrap();
    app.world_mut().entity_mut(entity).insert(Position(start));
    let target = bevy_math::Vec2::new(2.0, 20.0);
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("quarry:order_move").unwrap(),
                behaviour: Behaviour::Move,
                target_kind: TargetKind::PointOrUnit,
                policy: None,
                units: vec![walker],
                target: OrderTargetSim::Point(target),
                queue: QueueMode::Replace,
                group: 1,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        },
    );
    assert!(result.outcome.is_ok(), "move rejected: {result:?}");
    for _ in 0..300 {
        app.update();
    }
    let _ = std::fs::remove_dir_all(&dir);
    let at = app
        .world()
        .get::<Position>(unit_entity(app.world(), walker).unwrap())
        .unwrap()
        .0;
    assert!(
        at.distance(target) < 0.1,
        "the walker did not arrive along the edge: it stands at {at:?}"
    );
}

/// Marco, quarry_ridge (2026-11-26): the twelve crawlers (radius 1.1, the wide class with no
/// step) dig the whole designation on the ridge with the Auger. Few poses that touch a ridge
/// cell fit a crawler on one height, so a crawler must also work from farther away in its
/// reach, and a walk that cannot end must give way to another pose. A build with only the
/// touching poses digs two cells and then stands still (Marco's run, tick 382 to 2321).
#[test]
fn crawlers_dig_the_ridge_from_the_edge_of_their_reach() {
    let mut app = package_app("quarry_ridge");
    let crawlers: Vec<u32> = app
        .world()
        .iter_entities()
        .filter_map(|entity| {
            let kind = entity.get::<crate::world::UnitType>()?.0;
            let content = &app
                .world()
                .resource::<crate::UnitTypes>()
                .get(kind)?
                .content_id;
            (content == "quarry_ridge:crawler" && entity.get::<Owner>()?.0.0 == 0)
                .then(|| entity.get::<UnitId>().unwrap().0)
        })
        .collect();
    assert_eq!(crawlers.len(), 12, "the map places twelve own crawlers");
    let cells = TerrainRect {
        min: TerrainCell { x: 29, y: 20 },
        max: TerrainCell { x: 34, y: 28 },
    };
    let heights = |app: &bevy_app::App| -> Vec<i32> {
        let terrain = app.world().resource::<TerrainState>();
        (cells.min.y..cells.max.y)
            .flat_map(|y| (cells.min.x..cells.max.x).map(move |x| Cell { x, y }))
            .map(|cell| terrain.elevation(cell).unwrap())
            .collect()
    };
    let order = OrderCommand {
        order: ContentId::parse("quarry_ridge:order_work").unwrap(),
        behaviour: Behaviour::TerrainWork,
        target_kind: TargetKind::TerrainDesignation,
        policy: None,
        units: crawlers,
        target: OrderTargetSim::TerrainWork {
            tool: ContentId::parse("quarry_ridge:auger").unwrap(),
            cells,
            target_elevation: 0,
        },
        queue: QueueMode::Replace,
        group: 1,
        allowed_units: vec![],
        queue_depth: 16,
    };
    let accepted = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(order)),
        },
    );
    assert!(
        accepted.outcome.is_ok(),
        "work order rejected: {accepted:?}"
    );
    // The fixed build ends the work in about 600 ticks; 60 s leaves a margin.
    for _ in 0..1200 {
        app.update();
        if heights(&app).iter().all(|height| *height <= 0) {
            return;
        }
    }
    let waits: Vec<_> = app
        .world()
        .resource::<TerrainWorkRuntime>()
        .units
        .values()
        .map(|state| (state.cell, state.wait.clone()))
        .collect();
    let left = heights(&app).iter().filter(|height| **height > 0).count();
    panic!("{left} cells are not dug after 60 s: {waits:?}");
}
