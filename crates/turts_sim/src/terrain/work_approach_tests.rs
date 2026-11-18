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
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
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
    let poses = state.approach.as_ref().unwrap().poses;
    app.world_mut().get_mut::<Position>(entity).unwrap().0 += bevy_math::Vec2::new(0.025, -0.025);
    approach(app.world_mut(), unit, Cell { x: 11, y: 10 }, &mut state);
    assert_eq!(
        app.world().get::<crate::MoveTarget>(entity).unwrap().0,
        endpoint
    );
    assert_eq!(state.approach.as_ref().unwrap().poses, poses);
    assert_eq!((state.done, state.carry, state.banked), (17, 3, true));
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
