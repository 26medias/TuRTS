//! Per-unit order validation, queue behaviour, and the legacy plain-move boundary.

use std::collections::VecDeque;

use turts_mod_api::{
    ContentId, Footprint, MapDef, MapSize, PackageId, Placement, UnitDef, UnitRegistry, WorldPos,
};
use turts_sim::{
    AuditTrail, Command, CommandKind, MoveTarget, Position, RejectReason, SlotId, UnitOrders,
    UnitTypes, Vec2, apply_command, audit_state, headless_app, load_map,
    orders::{Order, OrderCommand, OrderTargetSim, QueueMode},
};

fn app_with_units(mobile: usize, static_units: usize, foreign: usize) -> bevy_app::App {
    let mut app = headless_app(20);
    let origin = PackageId("fixture".to_owned());
    for (id, movement) in [("walker", Some(3.0)), ("post", None)] {
        app.world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(
                &origin,
                UnitDef {
                    id: id.into(),
                    name: id.into(),
                    movement: movement.map(|speed| turts_mod_api::MovementDef { speed }),
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
    }
    let placements = (0..mobile + static_units + foreign)
        .map(|index| {
            let row = index / 20;
            let column = index % 20;
            Placement {
                unit: if index < mobile || index >= mobile + static_units {
                    "fixture:walker".into()
                } else {
                    "fixture:post".into()
                },
                owner_slot: u8::from(index >= mobile + static_units),
                pos: WorldPos {
                    x: 2.0 + column as f32 * 3.0,
                    y: 2.0 + row as f32 * 3.0,
                },
                contents: Vec::new(),
                facing: None,
            }
        })
        .collect();
    load_map(
        app.world_mut(),
        &origin,
        &MapDef {
            id: "field".into(),
            size: MapSize { x: 100.0, y: 100.0 },
            placements,
            nodes: Vec::new(),
            slot_economy: Vec::new(),
            belts: Vec::new(),
        },
    )
    .unwrap();
    app
}

fn order_command(
    units: Vec<u32>,
    target: Vec2,
    queue: QueueMode,
    queue_depth: u8,
    group: u32,
) -> Command {
    Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("fixture:move").unwrap(),
            behaviour: turts_mod_api::Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::PointOrUnit,
            policy: None,
            units,
            target: OrderTargetSim::Point(target),
            queue,
            group,
            allowed_units: Vec::new(),
            queue_depth,
        })),
    }
}

fn unit_target_move_command(unit: u32, target: u32, queue: QueueMode) -> Command {
    Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("fixture:move").unwrap(),
            behaviour: turts_mod_api::Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::Unit,
            policy: None,
            units: vec![unit],
            target: OrderTargetSim::Unit(target),
            queue,
            group: 1,
            allowed_units: Vec::new(),
            queue_depth: 16,
        })),
    }
}

fn command_with_target(units: Vec<u32>, target: OrderTargetSim) -> Command {
    let mut command = order_command(units, Vec2::new(50.0, 50.0), QueueMode::Replace, 16, 1);
    let CommandKind::Order(order) = &mut command.kind else {
        unreachable!()
    };
    order.target = target;
    command
}

#[test]
fn per_unit_results_for_a_mixed_envelope() {
    let mut app = app_with_units(290, 5, 5);
    let original: Vec<_> = (1..=300)
        .map(|unit_id| {
            let entity = app
                .world()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|id| id.0 == unit_id)
                })
                .unwrap();
            (
                unit_id,
                entity.get::<Position>().unwrap().0,
                entity.get::<UnitOrders>().cloned(),
            )
        })
        .collect();
    let command = order_command(
        (1..=300).rev().collect(),
        Vec2::new(90.0, 90.0),
        QueueMode::Replace,
        16,
        1,
    );
    let result = apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, (1..=290).collect::<Vec<_>>());
    assert_eq!(result.units.refused.len(), 10);
    assert_eq!(
        result.units.refused[..5],
        (291..=295)
            .map(|unit| (unit, RejectReason::NotApplicable))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.units.refused[5..],
        (296..=300)
            .map(|unit| (unit, RejectReason::NotOwner))
            .collect::<Vec<_>>()
    );
    for unit_id in 1..=290 {
        let entity = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == unit_id)
            })
            .unwrap();
        assert_eq!(
            entity.get::<MoveTarget>().unwrap().0,
            Vec2::new(90.0, 90.0),
            "accepted unit {unit_id} target"
        );
    }
    app.update();
    for unit_id in 1..=290 {
        let original_pos = original[(unit_id - 1) as usize].1;
        let col = (unit_id - 1) % 20;
        let row = (unit_id - 1) / 20;
        let start_x = 2.0 + col as f32 * 3.0;
        let start_y = 2.0 + row as f32 * 3.0;
        assert_eq!(original_pos, Vec2::new(start_x, start_y));
        let dx = 90.0 - start_x;
        let dy = 90.0 - start_y;
        let distance = (dx * dx + dy * dy).sqrt();
        let expected = Vec2::new(
            start_x + dx / distance * 0.15,
            start_y + dy / distance * 0.15,
        );
        let entity = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == unit_id)
            })
            .unwrap();
        let actual = entity.get::<Position>().unwrap().0;
        assert!(
            (actual.x - expected.x).abs() < 0.00001,
            "accepted unit {unit_id} x: {actual:?} != {expected:?}"
        );
        assert!(
            (actual.y - expected.y).abs() < 0.00001,
            "accepted unit {unit_id} y: {actual:?} != {expected:?}"
        );
    }
    for unit_id in 291..=300 {
        let entity = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == unit_id)
            })
            .unwrap();
        assert_eq!(
            entity.get::<Position>().unwrap().0,
            original[(unit_id - 1) as usize].1
        );
        assert!(entity.get::<MoveTarget>().is_none());
        assert_eq!(
            entity.get::<UnitOrders>().cloned(),
            original[(unit_id - 1) as usize].2
        );
    }
}

#[test]
fn point_move_outside_map_clamps_on_the_order_path() {
    let mut app = app_with_units(1, 0, 0);
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
    let initial_pos = app.world().entity(actor).get::<Position>().unwrap().0;
    let result = apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(101.0, 40.0), QueueMode::Replace, 16, 0),
    );
    assert_eq!(result.units.accepted, vec![1]);
    assert!(result.units.refused.is_empty());
    assert_eq!(
        app.world().entity(actor).get::<MoveTarget>().unwrap().0,
        Vec2::new(100.0, 40.0)
    );
    assert_eq!(
        app.world().entity(actor).get::<Position>().unwrap().0,
        initial_pos
    );
    let orders = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert!(orders.queue.is_empty());
    app.update();
    let moved = app.world().entity(actor).get::<Position>().unwrap().0;
    assert!(moved.x > initial_pos.x && moved.x <= 100.0);
    assert!(moved.y > initial_pos.y && moved.y <= 40.0);
    assert_eq!(
        app.world().entity(actor).get::<MoveTarget>().unwrap().0,
        Vec2::new(100.0, 40.0)
    );
}

#[test]
fn per_unit_owner_and_capability_precede_a_bad_target() {
    let mut app = app_with_units(1, 1, 1);
    let result = apply_command(
        app.world_mut(),
        &command_with_target(vec![3, 999, 1, 2], OrderTargetSim::Invalid),
    );
    assert_eq!(
        result.units.refused,
        vec![
            (1, RejectReason::TargetInvalid),
            (2, RejectReason::NotApplicable),
            (3, RejectReason::NotOwner),
            (999, RejectReason::UnknownUnit),
        ]
    );
    let result = apply_command(
        app.world_mut(),
        &command_with_target(vec![1], OrderTargetSim::Point(Vec2::new(f32::NAN, 50.0))),
    );
    assert_eq!(
        result.units.refused,
        vec![(1, RejectReason::TargetNotFinite)]
    );
}

#[test]
fn shift_appends_and_plain_replaces() {
    let mut app = app_with_units(1, 0, 0);
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(20.0, 20.0), QueueMode::Replace, 2, 1),
    );
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(30.0, 30.0), QueueMode::Append, 2, 1),
    );
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<turts_sim::UnitId>().is_some())
        .unwrap()
        .id();
    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert_eq!(
        orders.current.as_ref().unwrap().target,
        OrderTargetSim::Point(Vec2::new(20.0, 20.0))
    );
    assert_eq!(orders.queue.len(), 1);
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(40.0, 40.0), QueueMode::Replace, 2, 1),
    );
    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert_eq!(
        orders.current.as_ref().unwrap().target,
        OrderTargetSim::Point(Vec2::new(40.0, 40.0))
    );
    assert!(orders.queue.is_empty());
}

#[test]
fn plain_group_zero_replace_clears_an_appended_waypoint() {
    let mut app = app_with_units(1, 0, 0);
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(20.0, 20.0), QueueMode::Replace, 2, 0),
    );
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(30.0, 30.0), QueueMode::Append, 2, 0),
    );
    let result = apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(40.0, 40.0), QueueMode::Replace, 2, 0),
    );
    assert_eq!(result.units.accepted, vec![1]);
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<turts_sim::UnitId>().is_some())
        .unwrap()
        .id();
    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert!(orders.queue.is_empty());
    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>().unwrap().0,
        Vec2::new(40.0, 40.0)
    );
    app.update();
    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>().unwrap().0,
        Vec2::new(40.0, 40.0)
    );
}

#[test]
fn move_follows_a_unit_target_as_it_moves() {
    let mut app = app_with_units(2, 0, 0);
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
    let initial_target = app.world().entity(target).get::<Position>().unwrap().0;
    let result = apply_command(
        app.world_mut(),
        &unit_target_move_command(1, 2, QueueMode::Replace),
    );
    assert_eq!(result.units.accepted, vec![1]);
    assert_eq!(
        app.world().entity(actor).get::<MoveTarget>().unwrap().0,
        initial_target
    );
    app.world_mut()
        .entity_mut(target)
        .get_mut::<Position>()
        .unwrap()
        .0 = Vec2::new(40.0, 40.0);
    app.update();
    let actor_pos = app.world().entity(actor).get::<Position>().unwrap().0;
    assert!(
        actor_pos.x > 2.0 && actor_pos.y > 2.0,
        "actor advances toward the target's new position"
    );
    assert_eq!(
        app.world().entity(actor).get::<MoveTarget>().unwrap().0,
        Vec2::new(40.0, 40.0)
    );
}

#[test]
fn move_target_gone_records_failure_and_idles() {
    let mut app = app_with_units(2, 0, 0);
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
    assert_eq!(
        apply_command(
            app.world_mut(),
            &unit_target_move_command(1, 2, QueueMode::Replace)
        )
        .units
        .accepted,
        vec![1]
    );
    app.world_mut().despawn(target);
    app.update();
    let orders = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert_eq!(
        orders.last_failure.as_ref().unwrap().reason,
        RejectReason::TargetGone
    );
}

#[test]
fn completed_unit_follow_starts_the_next_queued_order_in_orders_phase() {
    let mut app = app_with_units(2, 0, 0);
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
    let target_pos = app.world().entity(target).get::<Position>().unwrap().0;
    assert_eq!(
        apply_command(
            app.world_mut(),
            &unit_target_move_command(1, 2, QueueMode::Replace)
        )
        .units
        .accepted,
        vec![1]
    );
    let appended = order_command(vec![1], Vec2::new(70.0, 70.0), QueueMode::Append, 16, 2);
    assert_eq!(
        apply_command(app.world_mut(), &appended).units.accepted,
        vec![1]
    );
    app.world_mut()
        .entity_mut(actor)
        .get_mut::<Position>()
        .unwrap()
        .0 = target_pos;
    app.world_mut().entity_mut(actor).remove::<MoveTarget>();
    app.update();
    let orders = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert_eq!(
        orders.current.as_ref().unwrap().target,
        OrderTargetSim::Point(Vec2::new(70.0, 70.0))
    );
    assert!(orders.queue.is_empty());
    assert_eq!(
        app.world().entity(actor).get::<MoveTarget>().unwrap().0,
        Vec2::new(70.0, 70.0)
    );
}

#[test]
fn stop_clears_current_and_queue() {
    let mut app = app_with_units(1, 0, 0);
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(20.0, 20.0), QueueMode::Replace, 2, 1),
    );
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(30.0, 30.0), QueueMode::Append, 2, 1),
    );
    let stop = Command {
        slot: SlotId(0),
        seq: 2,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("fixture:stop").unwrap(),
            behaviour: turts_mod_api::Behaviour::Stop,
            target_kind: turts_mod_api::TargetKind::None,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::None,
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: Vec::new(),
            queue_depth: 16,
        })),
    };
    apply_command(app.world_mut(), &stop);
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<turts_sim::UnitId>().is_some())
        .unwrap()
        .id();
    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert!(orders.queue.is_empty());
    assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
}

#[test]
fn queue_beyond_depth_is_queue_full() {
    let mut app = app_with_units(1, 0, 0);
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(20.0, 20.0), QueueMode::Replace, 1, 1),
    );
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(30.0, 30.0), QueueMode::Append, 1, 1),
    );
    let result = apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(40.0, 40.0), QueueMode::Append, 1, 1),
    );
    assert_eq!(result.units.refused, vec![(1, RejectReason::QueueFull)]);
}

#[test]
fn append_behind_hold_or_gather_is_not_queueable() {
    let mut app = app_with_units(1, 0, 0);
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<turts_sim::UnitId>().is_some())
        .unwrap()
        .id();
    for behaviour in [
        turts_mod_api::Behaviour::Hold,
        turts_mod_api::Behaviour::Gather,
    ] {
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(Order {
                order: ContentId::parse("fixture:current").unwrap(),
                behaviour,
                target: if behaviour == turts_mod_api::Behaviour::Gather {
                    OrderTargetSim::Node(turts_sim::NodeId(1))
                } else {
                    OrderTargetSim::None
                },
                group: 1,
            }),
            queue: VecDeque::new(),
            last_failure: None,
        });
        let result = apply_command(
            app.world_mut(),
            &order_command(vec![1], Vec2::new(40.0, 40.0), QueueMode::Append, 16, 1),
        );
        assert_eq!(result.units.refused, vec![(1, RejectReason::NotQueueable)]);
    }
}

#[test]
fn append_rejects_when_hold_or_gather_is_queued() {
    let mut app = app_with_units(1, 0, 0);
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<turts_sim::UnitId>().is_some())
        .unwrap()
        .id();
    for behaviour in [
        turts_mod_api::Behaviour::Hold,
        turts_mod_api::Behaviour::Gather,
    ] {
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(Order {
                order: ContentId::parse("fixture:current").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                group: 1,
            }),
            queue: VecDeque::from([Order {
                order: ContentId::parse("fixture:barrier").unwrap(),
                behaviour,
                target: if behaviour == turts_mod_api::Behaviour::Gather {
                    OrderTargetSim::Node(turts_sim::NodeId(1))
                } else {
                    OrderTargetSim::None
                },
                group: 1,
            }]),
            last_failure: None,
        });
        let result = apply_command(
            app.world_mut(),
            &order_command(vec![1], Vec2::new(50.0, 50.0), QueueMode::Append, 16, 1),
        );
        assert_eq!(result.units.refused, vec![(1, RejectReason::NotQueueable)]);
    }
}

#[test]
fn queued_order_starts_in_the_tick_after_landing() {
    let mut app = app_with_units(1, 0, 0);
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(2.0, 2.0), QueueMode::Replace, 2, 0),
    );
    apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(30.0, 30.0), QueueMode::Append, 2, 1),
    );
    app.update();
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<turts_sim::UnitId>().is_some())
        .unwrap()
        .id();
    assert_eq!(
        app.world().entity(entity).get::<Position>().unwrap().0,
        Vec2::new(2.0, 2.0)
    );
    assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
    assert!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .is_none()
    );
    app.update();
    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>().unwrap().0,
        Vec2::new(30.0, 30.0)
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .target,
        OrderTargetSim::Point(Vec2::new(30.0, 30.0))
    );
}

#[test]
fn plain_move_writes_no_order_operation() {
    let mut app = app_with_units(1, 0, 0);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let command = order_command(vec![1], Vec2::new(20.0, 20.0), QueueMode::Replace, 16, 0);
    apply_command(app.world_mut(), &command);
    let trail = app.world().resource::<AuditTrail>();
    assert_eq!(trail.operations.len(), 1);
    assert_eq!(
        trail.operations[0].phase,
        turts_sim::economy::transaction::AuditPhase::Command
    );
    let state = audit_state(app.world_mut());
    assert!(state.unit_orders.is_empty());
}

#[test]
fn mixed_order_command_audit_covers_every_per_unit_operation() {
    let mut app = app_with_units(2, 1, 0);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let result = apply_command(
        app.world_mut(),
        &order_command(
            vec![3, 2, 1],
            Vec2::new(60.0, 60.0),
            QueueMode::Replace,
            16,
            1,
        ),
    );
    assert_eq!(result.units.accepted, vec![1, 2]);
    assert_eq!(result.units.refused, vec![(3, RejectReason::NotApplicable)]);
    let trail = app.world().resource::<AuditTrail>();
    assert_eq!(trail.commands.len(), 1);
    assert_eq!(trail.operations.len(), 2);
    assert!(trail.operations.iter().all(|operation| operation.phase == turts_sim::economy::transaction::AuditPhase::Orders));
    assert_eq!(
        trail
            .operations
            .iter()
            .map(|operation| operation.unit_id)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2)]
    );
    assert_eq!(trail.commands[0].before, trail.operations[0].before);
    assert_eq!(trail.commands[0].after, trail.operations[1].after);
    assert_eq!(
        trail.commands[0].operation_end_id,
        Some(trail.operations[1].operation_id)
    );
}

#[test]
fn all_refused_order_has_a_no_change_command_boundary() {
    let mut app = app_with_units(0, 1, 0);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let before = audit_state(app.world_mut());
    let result = apply_command(
        app.world_mut(),
        &order_command(vec![1], Vec2::new(60.0, 60.0), QueueMode::Replace, 16, 1),
    );
    assert_eq!(result.units.refused, vec![(1, RejectReason::NotApplicable)]);
    let trail = app.world().resource::<AuditTrail>();
    assert_eq!(trail.commands.len(), 1);
    assert_eq!(trail.operations.len(), 1);
    assert_eq!(
        trail.operations[0].phase,
        turts_sim::economy::transaction::AuditPhase::Command
    );
    assert_eq!(trail.commands[0].before, before);
    assert_eq!(trail.commands[0].after, before);
    assert_eq!(trail.operations[0].before, before);
    assert_eq!(trail.operations[0].after, before);
}
