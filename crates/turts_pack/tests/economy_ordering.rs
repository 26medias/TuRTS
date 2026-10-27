use bevy_app::FixedUpdate;
use bevy_ecs::prelude::World;
use bevy_ecs::schedule::IntoScheduleConfigs;
use turts_mod_api::{
    Footprint, MapDef, MapSize, PackageId, Placement, UnitDef, UnitRegistry, WorldPos,
};
use turts_sim::{
    Command, CommandKind, CommandQueue, EconomyEventKind, MatchStopped, MoveTarget, RejectReason,
    SimSet, SlotId, Tick, UnitTypes, Vec2, apply_command, audit_state, committed_tick,
    completed_world, headless_app, load_map,
};

fn emit_ordinal_probe_events(world: &mut World) {
    if world.contains_resource::<MatchStopped>() {
        return;
    }
    for job in [1, 2] {
        turts_sim::record_committed_event(
            world,
            0,
            EconomyEventKind::ProductionCompleted {
                producer: 7,
                job,
                output_unit_id: 8,
            },
        )
        .unwrap();
    }
}

const ORIGIN: &str = "fixture_pkg";

fn app_with_units() -> bevy_app::App {
    let mut app = headless_app(20);
    let origin = PackageId(ORIGIN.to_string());
    let def = UnitDef {
        id: "walker".into(),
        name: "Walker".into(),
        movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
        footprint: Footprint::Circle { radius: 0.5 },
    };
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(&origin, def)
        .unwrap();
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &origin,
            UnitDef {
                id: "building".into(),
                name: "Building".into(),
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 0.5 },
            },
        )
        .unwrap();
    let place = |unit: &str, x, y| Placement {
        unit: format!("{ORIGIN}:{unit}"),
        owner_slot: 0,
        pos: WorldPos { x, y },
        contents: Vec::new(),
    };
    load_map(
        app.world_mut(),
        &origin,
        &MapDef {
            id: "field".into(),
            size: MapSize { x: 64.0, y: 64.0 },
            placements: vec![place("walker", 8.0, 8.0), place("building", 20.0, 20.0)],
            nodes: vec![],
            slot_economy: vec![],
        },
    )
    .unwrap();
    app
}

fn command(seq: u32, kind: CommandKind) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind,
    }
}

#[test]
fn same_batch_observes_prior_commit_and_rejection_changes_no_write_set() {
    let mut app = app_with_units();
    let before = audit_state(app.world_mut());
    {
        let mut queue = app.world_mut().resource_mut::<CommandQueue>();
        queue.0.push_back(command(
            1,
            CommandKind::MoveTo {
                unit_ids: vec![1],
                target: Vec2::new(30.0, 30.0),
            },
        ));
        queue.0.push_back(command(
            2,
            CommandKind::MoveTo {
                unit_ids: vec![1],
                target: Vec2::new(40.0, 40.0),
            },
        ));
        queue.0.push_back(command(
            3,
            CommandKind::MoveTo {
                unit_ids: vec![1, 2],
                target: Vec2::new(50.0, 50.0),
            },
        ));
    }
    turts_sim::drain_commands(app.world_mut());
    let results = &app.world().resource::<turts_sim::CommandResults>().0;
    assert_eq!(results[0].outcome, Ok(turts_sim::Applied { job_id: None }));
    assert_eq!(results[1].outcome, Ok(turts_sim::Applied { job_id: None }));
    assert_eq!(results[2].outcome, Err(RejectReason::NotMobile));
    let target_before_rejection = app
        .world_mut()
        .iter_entities()
        .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|id| id.0 == 1))
        .unwrap()
        .get::<MoveTarget>()
        .copied();
    assert_eq!(
        target_before_rejection,
        Some(MoveTarget(Vec2::new(40.0, 40.0)))
    );
    assert_eq!(audit_state(app.world_mut()), before);
    let target_after_rejection = app
        .world_mut()
        .iter_entities()
        .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|id| id.0 == 1))
        .unwrap()
        .get::<MoveTarget>()
        .copied();
    assert_eq!(target_after_rejection, target_before_rejection);
}

#[test]
fn move_interrupt_is_immediate_and_group_static_rejects() {
    let mut app = app_with_units();
    apply_command(
        app.world_mut(),
        &command(
            1,
            CommandKind::MoveTo {
                unit_ids: vec![1],
                target: Vec2::new(30.0, 30.0),
            },
        ),
    );
    let target = app
        .world_mut()
        .iter_entities()
        .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|id| id.0 == 1))
        .unwrap()
        .get::<MoveTarget>()
        .copied();
    assert_eq!(target, Some(MoveTarget(Vec2::new(30.0, 30.0))));
    let rejected = apply_command(
        app.world_mut(),
        &command(
            2,
            CommandKind::MoveTo {
                unit_ids: vec![1, 2],
                target: Vec2::new(40.0, 40.0),
            },
        ),
    );
    assert_eq!(rejected.outcome, Err(RejectReason::NotMobile));
    let target_after_group_rejection = app
        .world_mut()
        .iter_entities()
        .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|id| id.0 == 1))
        .unwrap()
        .get::<MoveTarget>()
        .copied();
    assert_eq!(target_after_group_rejection, target);
    assert_eq!(
        app.world().resource::<turts_sim::CommandResults>().0.len(),
        0
    );
}

#[test]
fn tick_and_event_ordinals_do_not_wrap() {
    let mut app = app_with_units();
    app.add_systems(
        FixedUpdate,
        emit_ordinal_probe_events.after(SimSet::Production),
    );
    app.world_mut().resource_mut::<Tick>().0 = u32::MAX - 1;
    app.update();
    assert_eq!(app.world().resource::<Tick>().0, u32::MAX);
    app.update();
    assert!(app.world().contains_resource::<MatchStopped>());
    assert_eq!(committed_tick(app.world()).tick, u32::MAX);
    assert!(
        committed_tick(app.world())
            .events
            .windows(2)
            .all(|w| w[0].ordinal < w[1].ordinal)
    );
    assert_eq!(
        committed_tick(app.world())
            .events
            .iter()
            .map(|event| event.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    let events = committed_tick(app.world()).events.clone();
    app.world_mut()
        .resource_mut::<turts_sim::NextEventOrdinal>()
        .0 = None;
    assert_eq!(
        turts_sim::record_committed_event(
            app.world_mut(),
            0,
            EconomyEventKind::ProductionCompleted {
                producer: 7,
                job: 3,
                output_unit_id: 9,
            }
        ),
        Err(RejectReason::ArithmeticOverflow),
    );
    assert_eq!(committed_tick(app.world()).events, events);
}

#[test]
fn collected_batch_is_completed_tick_only() {
    let mut app = app_with_units();
    app.add_systems(
        FixedUpdate,
        emit_ordinal_probe_events.after(SimSet::Production),
    );
    let first = committed_tick(app.world()).clone();
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(command(
            1,
            CommandKind::MoveTo {
                unit_ids: vec![1],
                target: Vec2::new(30.0, 30.0),
            },
        ));
    app.update();
    assert_eq!(
        completed_world(app.world()).tick,
        committed_tick(app.world()).tick
    );
    assert_eq!(first.tick + 1, committed_tick(app.world()).tick);
    let snapshot = committed_tick(app.world()).clone();
    app.update();
    assert_eq!(snapshot.tick + 1, committed_tick(app.world()).tick);
    assert_eq!(
        snapshot
            .events
            .iter()
            .map(|event| event.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        completed_world(app.world()).tick,
        committed_tick(app.world()).tick
    );
    assert!(
        !committed_tick(app.world())
            .events
            .iter()
            .any(|e| matches!(e.kind, EconomyEventKind::Extracted { .. }))
    );
}
