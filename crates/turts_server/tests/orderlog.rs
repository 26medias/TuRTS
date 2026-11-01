//! Exact text contract for the public order log writer.

#[path = "../src/orderlog.rs"]
#[allow(dead_code)]
mod orderlog;

#[test]
fn moving_then_blocked_has_exact_lines() {
    let mut output = Vec::new();
    orderlog::write_transition(&mut output, 12, 7, "Moving", None).unwrap();
    orderlog::write_transition(&mut output, 40, 7, "Blocked", Some("blocked")).unwrap();

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "12,7,Moving,-\n40,7,Blocked,blocked\n"
    );
}

fn unit_order(
    unit_id: u32,
    activity: turts_sim::economy::views::UnitOrderActivityView,
) -> turts_sim::economy::views::UnitOrderView {
    turts_sim::economy::views::UnitOrderView {
        unit_id,
        owner_slot: 0,
        activity,
        current: None,
        queued: Vec::new(),
        last_failure: None,
    }
}

#[test]
fn installed_logger_sorts_new_and_done_states_by_unit_id() {
    use bevy::prelude::App;
    use turts_sim::economy::views::{UnitOrderActivityView as Activity, WorldView};

    let path = std::env::temp_dir().join(format!(
        "turts_orderlog_sort_{}_{}.csv",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_file(&path);
    let mut app = App::new();
    app.insert_resource(turts_sim::Tick(1));
    app.insert_resource(turts_sim::CompletedWorld(WorldView {
        unit_orders: vec![
            unit_order(1, Activity::Moving),
            unit_order(2, Activity::Moving),
        ],
        ..WorldView::default()
    }));
    orderlog::install(&mut app, Some(&path)).unwrap();
    orderlog::record_changes(app.world_mut());

    app.world_mut().resource_mut::<turts_sim::Tick>().0 = 2;
    app.world_mut()
        .resource_mut::<turts_sim::CompletedWorld>()
        .0
        .unit_orders = vec![unit_order(2, Activity::Blocked)];
    orderlog::record_changes(app.world_mut());

    let lines: Vec<_> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        lines,
        [
            "1,1,Moving,-",
            "1,2,Moving,-",
            "2,1,Done,-",
            "2,2,Blocked,blocked"
        ]
    );
    drop(app);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn installed_logger_does_not_repeat_a_historical_failure_after_completion() {
    use bevy::prelude::App;
    use turts_sim::economy::views::{
        UnitOrderActivityView as Activity, UnitOrderFailureView, WorldView,
    };

    let path = std::env::temp_dir().join(format!(
        "turts_orderlog_failure_{}_{}.csv",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_file(&path);
    let mut app = App::new();
    app.insert_resource(turts_sim::Tick(1));
    let mut failed = unit_order(1, Activity::Idle);
    failed.last_failure = Some(UnitOrderFailureView {
        order: "move".to_owned(),
        reason: turts_sim::RejectReason::TargetGone,
        tick: 1,
    });
    app.insert_resource(turts_sim::CompletedWorld(WorldView {
        unit_orders: vec![failed],
        ..WorldView::default()
    }));
    orderlog::install(&mut app, Some(&path)).unwrap();
    orderlog::record_changes(app.world_mut());

    app.world_mut().resource_mut::<turts_sim::Tick>().0 = 2;
    let mut moving = unit_order(1, Activity::Moving);
    moving.last_failure = Some(UnitOrderFailureView {
        order: "move".to_owned(),
        reason: turts_sim::RejectReason::TargetGone,
        tick: 1,
    });
    app.world_mut()
        .resource_mut::<turts_sim::CompletedWorld>()
        .0
        .unit_orders = vec![moving];
    orderlog::record_changes(app.world_mut());

    app.world_mut().resource_mut::<turts_sim::Tick>().0 = 3;
    let mut completed = unit_order(1, Activity::Idle);
    completed.last_failure = Some(UnitOrderFailureView {
        order: "move".to_owned(),
        reason: turts_sim::RejectReason::TargetGone,
        tick: 1,
    });
    app.world_mut()
        .resource_mut::<turts_sim::CompletedWorld>()
        .0
        .unit_orders = vec![completed];
    orderlog::record_changes(app.world_mut());

    app.world_mut().resource_mut::<turts_sim::Tick>().0 = 4;
    let mut still_completed = unit_order(1, Activity::Idle);
    still_completed.last_failure = Some(UnitOrderFailureView {
        order: "move".to_owned(),
        reason: turts_sim::RejectReason::TargetGone,
        tick: 1,
    });
    app.world_mut()
        .resource_mut::<turts_sim::CompletedWorld>()
        .0
        .unit_orders = vec![still_completed];
    orderlog::record_changes(app.world_mut());

    let lines: Vec<_> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        lines,
        ["1,1,Failed,target_gone", "2,1,Moving,-", "3,1,Done,-",]
    );
    drop(app);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn installed_logger_uses_blocked_state_reason_not_historical_failure() {
    use bevy::prelude::App;
    use turts_sim::economy::views::{
        UnitOrderActivityView as Activity, UnitOrderFailureView, WorldView,
    };

    let path = std::env::temp_dir().join(format!(
        "turts_orderlog_blocked_{}_{}.csv",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_file(&path);
    let mut app = App::new();
    app.insert_resource(turts_sim::Tick(1));
    app.insert_resource(turts_sim::CompletedWorld(WorldView {
        unit_orders: vec![unit_order(2, Activity::Moving)],
        ..WorldView::default()
    }));
    orderlog::install(&mut app, Some(&path)).unwrap();
    orderlog::record_changes(app.world_mut());

    app.world_mut().resource_mut::<turts_sim::Tick>().0 = 2;
    let mut blocked = unit_order(2, Activity::Blocked);
    blocked.last_failure = Some(UnitOrderFailureView {
        order: "attack".to_owned(),
        reason: turts_sim::RejectReason::TargetGone,
        tick: 1,
    });
    app.world_mut()
        .resource_mut::<turts_sim::CompletedWorld>()
        .0
        .unit_orders = vec![blocked.clone()];
    orderlog::record_changes(app.world_mut());

    app.world_mut().resource_mut::<turts_sim::Tick>().0 = 3;
    app.world_mut()
        .resource_mut::<turts_sim::CompletedWorld>()
        .0
        .unit_orders = vec![blocked];
    orderlog::record_changes(app.world_mut());

    let lines: Vec<_> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(lines, ["1,2,Moving,-", "2,2,Blocked,blocked"]);
    drop(app);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn installed_logger_tracks_real_failed_then_appended_move_to_completion() {
    use turts_mod_api::{
        ContentId, Footprint, MapDef, MapSize, PackageId, Placement, UnitDef, UnitRegistry,
        WorldPos,
    };
    use turts_sim::orders::{OrderCommand, OrderTargetSim, QueueMode, UnitOrders};
    use turts_sim::{Command, CommandKind, Position, RejectReason, SlotId, UnitTypes, Vec2};

    let dir = std::env::temp_dir().join(format!(
        "turts_orderlog_runtime_{}_{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("orders.log");

    let mut app = turts_sim::headless_app(20);
    let package = PackageId("fixture".to_owned());
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &package,
            UnitDef {
                id: "walker".into(),
                name: "walker".into(),
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 0.5 },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        )
        .unwrap();
    turts_sim::load_map(
        app.world_mut(),
        &package,
        &MapDef {
            id: "field".into(),
            size: MapSize { x: 20.0, y: 20.0 },
            placements: vec![
                Placement {
                    unit: "fixture:walker".into(),
                    owner_slot: 0,
                    pos: WorldPos { x: 2.0, y: 2.0 },
                    contents: Vec::new(),
                    facing: None,
                },
                Placement {
                    unit: "fixture:walker".into(),
                    owner_slot: 1,
                    pos: WorldPos { x: 5.0, y: 2.0 },
                    contents: Vec::new(),
                    facing: None,
                },
            ],
            nodes: Vec::new(),
            slot_economy: Vec::new(),
            belts: Vec::new(),
        },
    )
    .unwrap();
    let actor = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    let target = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2)
        })
        .unwrap()
        .id();
    let make_move = |target, queue| Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("fixture:move").unwrap(),
            behaviour: turts_mod_api::Behaviour::Move,
            target_kind: if matches!(target, OrderTargetSim::Unit(_)) {
                turts_mod_api::TargetKind::Unit
            } else {
                turts_mod_api::TargetKind::PointOrUnit
            },
            policy: None,
            units: vec![1],
            target,
            queue,
            group: 1,
            allowed_units: Vec::new(),
            queue_depth: 16,
        })),
    };

    orderlog::install(&mut app, Some(&path)).unwrap();
    let first = turts_sim::apply_command(
        app.world_mut(),
        &make_move(OrderTargetSim::Unit(2), QueueMode::Replace),
    );
    assert_eq!(first.units.accepted, [1]);
    app.world_mut().despawn(target);
    app.update();
    let failed = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert_eq!(
        failed.last_failure.as_ref().unwrap().reason,
        RejectReason::TargetGone
    );
    orderlog::record_changes(app.world_mut());

    let appended = turts_sim::apply_command(
        app.world_mut(),
        &make_move(
            OrderTargetSim::Point(Vec2::new(3.0, 2.0)),
            QueueMode::Append,
        ),
    );
    assert_eq!(
        app.world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .last_failure
            .as_ref()
            .unwrap()
            .reason,
        RejectReason::TargetGone
    );
    assert_eq!(
        app.world().entity(actor).get::<Position>().unwrap().0,
        Vec2::new(2.0, 2.0)
    );
    assert_eq!(appended.units.accepted, [1]);

    for _ in 0..20 {
        app.update();
        orderlog::record_changes(app.world_mut());
        if app
            .world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .is_none()
        {
            break;
        }
    }
    assert!(
        app.world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .is_none()
    );
    assert!(app.world().resource::<turts_sim::Tick>().0 >= 3);

    let lines: Vec<_> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        lines.first().map(String::as_str),
        Some("1,1,Failed,target_gone")
    );
    assert_eq!(lines.get(1).map(String::as_str), Some("2,1,Moving,-"));
    assert!(
        lines.last().is_some_and(|line| line.ends_with(",1,Done,-")),
        "successful appended move must finish with Done: {lines:?}"
    );
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}
