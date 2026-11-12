//! Group destination and movement acceptance for MC Task 6.

use turts_mod_api::{
    Footprint, MapDef, MapSize, PackageId, Placement, UnitDef, UnitRegistry, WorldPos,
};
use turts_sim::{
    Command, CommandKind, CommandQueue, CommandResults, MoveTarget, OrderCommand, OrderTargetSim,
    QueueMode, RejectReason, SlotId, Tick, UnitOrders, UnitTypes, Vec2, headless_app, load_map,
    snapshot,
};

const ORIGIN: &str = "fixture_pkg";

fn app_with_units(positions: &[(f32, f32)], owners: &[u8]) -> bevy_app::App {
    app_with_units_on(positions, owners, MapSize { x: 64.0, y: 64.0 }, 0.5)
}

fn app_with_units_on(
    positions: &[(f32, f32)],
    owners: &[u8],
    size: MapSize,
    radius: f32,
) -> bevy_app::App {
    let mut app = headless_app(20);
    let origin = PackageId(ORIGIN.to_owned());
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &origin,
            UnitDef {
                id: "walker".into(),
                name: "Walker".into(),
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
                armour: None,
                acquire: None,
                provides: Vec::new(),
                consumes: Vec::new(),
                income: None,
                spawn_order: None,
                constructor: None,
                placement: None,
                build_area: None,
                repairer: None,
            },
        )
        .unwrap();
    let map = MapDef {
        id: "field".into(),
        size,
        placements: positions
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| Placement {
                unit: format!("{ORIGIN}:walker"),
                owner_slot: owners.get(i).copied().unwrap_or(0),
                pos: WorldPos { x, y },
                contents: Vec::new(),
                facing: None,
                start_order: None,
            })
            .collect(),
        nodes: Vec::new(),
        slot_economy: Vec::new(),
        belts: Vec::new(),
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    app
}

fn send_order(
    app: &mut bevy_app::App,
    slot: u8,
    ids: Vec<u32>,
    target: Vec2,
    group: u32,
    queue: QueueMode,
) {
    let seq = app
        .world()
        .resource::<Tick>()
        .0
        .wrapping_add(1)
        .wrapping_add(app.world().resource::<CommandQueue>().0.len() as u32);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(slot),
            seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                target_kind: turts_mod_api::TargetKind::PointOrUnit,
                policy: None,
                units: ids,
                target: OrderTargetSim::Point(target),
                queue,
                group,
                allowed_units: Vec::new(),
                queue_depth: 16,
            })),
        });
}

#[test]
fn authored_singleton_obeys_shared_oversized_head_budget_in_both_arrival_orders() {
    for prior_active_move in [false, true] {
        for candidate_first in [false, true] {
            let positions: Vec<_> = (0..2049)
                .map(|index| {
                    (
                        2.0 + (index % 63) as f32 * 2.0,
                        2.0 + (index / 63) as f32 * 2.0,
                    )
                })
                .collect();
            let owners = vec![0; 2049];
            let mut app =
                app_with_units_on(&positions, &owners, MapSize { x: 128.0, y: 128.0 }, 0.5);
            if prior_active_move {
                let entity = app
                    .world_mut()
                    .iter_entities()
                    .find(|entity| {
                        entity
                            .get::<turts_sim::UnitId>()
                            .is_some_and(|id| id.0 == 2049)
                    })
                    .unwrap()
                    .id();
                app.world_mut().entity_mut(entity).insert((
                    MoveTarget(Vec2::new(70.0, 66.0)),
                    UnitOrders {
                        current: Some(turts_sim::Order {
                            order: turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
                            behaviour: turts_mod_api::Behaviour::Move,
                            target: OrderTargetSim::Point(Vec2::new(70.0, 66.0)),
                            group: 0,
                        }),
                        ..Default::default()
                    },
                ));
            }
            let group_ids: Vec<_> = (1..=2048).collect();
            if candidate_first {
                send_order(
                    &mut app,
                    0,
                    vec![2049],
                    Vec2::new(120.0, 120.0),
                    0,
                    QueueMode::Replace,
                );
            }
            send_order(
                &mut app,
                0,
                group_ids,
                Vec2::new(80.0, 80.0),
                1,
                QueueMode::Replace,
            );
            if !candidate_first {
                send_order(
                    &mut app,
                    0,
                    vec![2049],
                    Vec2::new(120.0, 120.0),
                    0,
                    QueueMode::Replace,
                );
            }
            send_hold(&mut app, 0, 2049, QueueMode::Append);
            // Arrival order, not numeric command sequence, determines the slot FIFO.
            {
                let mut commands = app.world_mut().resource_mut::<CommandQueue>();
                let candidate_seq = if candidate_first { 99 } else { 1 };
                let group_seq = if candidate_first { 1 } else { 99 };
                for command in &mut commands.0 {
                    if let CommandKind::Order(order) = &command.kind {
                        if order.units == [2049] {
                            command.seq = candidate_seq;
                        } else if order.units.len() == 2048 {
                            command.seq = group_seq;
                        }
                    }
                }
            }
            let initial_candidate_pos = positions[2048];
            app.update();

            let candidate = snapshot(app.world_mut())
                .into_iter()
                .find(|unit| unit.unit_id == 2049)
                .unwrap();
            if candidate_first {
                assert!(candidate.pos.distance(Vec2::from(initial_candidate_pos)) <= 0.150_001);
                assert_ne!(candidate.pos, Vec2::from(initial_candidate_pos));
            } else if prior_active_move {
                assert_eq!(
                    candidate.pos,
                    Vec2::from(initial_candidate_pos) + Vec2::new(0.15, 0.0),
                    "the retained prior Move continues while its replacement waits"
                );
            } else {
                assert_eq!(candidate.pos, Vec2::from(initial_candidate_pos));
            }
            let entity = app
                .world_mut()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|id| id.0 == 2049)
                })
                .unwrap()
                .id();
            assert!(
                app.world()
                    .resource::<turts_sim::orders::PendingMovementOrders>()
                    .pending_member_counts
                    .contains_key(&2049)
                    == !candidate_first,
                "slot arrival order determines which envelope waits"
            );
            if !candidate_first && prior_active_move {
                assert_eq!(
                    app.world().entity(entity).get::<MoveTarget>(),
                    Some(&MoveTarget(Vec2::new(70.0, 66.0))),
                    "a waiting replacement restores the previous execution target"
                );
                assert_eq!(
                    app.world()
                        .entity(entity)
                        .get::<UnitOrders>()
                        .unwrap()
                        .current
                        .as_ref()
                        .map(|order| order.target.clone()),
                    Some(OrderTargetSim::Point(Vec2::new(70.0, 66.0)))
                );
            } else if !candidate_first {
                assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
            } else {
                assert_eq!(
                    app.world().entity(entity).get::<MoveTarget>(),
                    Some(&MoveTarget(Vec2::new(120.0, 120.0)))
                );
            }
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<UnitOrders>()
                    .unwrap()
                    .queue
                    .iter()
                    .map(|order| order.behaviour)
                    .collect::<Vec<_>>(),
                vec![turts_mod_api::Behaviour::Hold],
                "an Append accepted behind the reservation survives promotion"
            );
            assert!(
                app.world()
                    .entity(entity)
                    .get::<turts_sim::movement::MoveProgress>()
                    .is_none()
            );
            assert_eq!(
                app.world()
                    .resource::<turts_sim::orders::MovementDispatchState>()
                    .used,
                if candidate_first { 1 } else { 2048 }
            );
        }
    }
}

#[test]
fn unopposed_authored_singleton_keeps_its_command_only_audit_shape() {
    let mut app = app_with_units(&[(8.0, 8.0)], &[0]);
    app.world_mut()
        .resource_mut::<turts_sim::AuditTrail>()
        .enable();
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(12.0, 8.0),
        0,
        QueueMode::Replace,
    );
    app.update();

    let trail = app.world().resource::<turts_sim::AuditTrail>();
    assert_eq!(trail.commands.len(), 1);
    assert_eq!(
        trail.operations.len(),
        1,
        "one command operation is expected"
    );
    assert_eq!(
        trail.operations[0].phase,
        turts_sim::economy::transaction::AuditPhase::Command,
        "the plain move adds no Orders-phase operation"
    );
    assert!(
        app.world()
            .resource::<turts_sim::orders::MovementDispatchState>()
            .is_default(),
        "a plain-only tick keeps default-omitted audit scheduler state"
    );
    assert!(
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .is_empty()
    );
    let mover = snapshot(app.world_mut()).pop().unwrap();
    assert!(mover.pos.x > 8.0 && mover.pos.x <= 8.150_001);
}

#[test]
fn authored_singleton_reserves_pending_admission_capacity_before_acceptance() {
    let mut app = app_with_units(&[(8.0, 8.0)], &[0]);
    let fake_members: Vec<_> = (10_000..26_384).collect();
    let request = turts_sim::orders::PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1,
        group: 9,
        pool_id: 0,
        point: Vec2::new(20.0, 20.0),
        unit_ids: fake_members,
        order: None,
        member_orders: Default::default(),
        plain: false,
    };
    assert!(
        app.world_mut()
            .resource_mut::<turts_sim::orders::PendingMovementOrders>()
            .queue(request)
    );
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(12.0, 8.0),
        0,
        QueueMode::Replace,
    );
    app.update();

    let result = &app.world().resource::<CommandResults>().0[0];
    assert_eq!(result.outcome, Err(RejectReason::RateLimited));
    assert_eq!(result.units.refused, vec![(1, RejectReason::RateLimited)]);
    assert!(
        app.world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap()
            .get::<MoveTarget>()
            .is_none()
    );
}

#[test]
fn unit_target_move_waits_for_cap_and_samples_live_target_at_activation() {
    let positions: Vec<_> = (0..2050)
        .map(|index| {
            (
                2.0 + (index % 63) as f32 * 2.0,
                2.0 + (index / 63) as f32 * 2.0,
            )
        })
        .collect();
    let mut owners = vec![0; 2050];
    owners[2049] = 1;
    let mut app = app_with_units_on(&positions, &owners, MapSize { x: 128.0, y: 128.0 }, 0.5);
    let before = snapshot(app.world_mut());
    let mover_before = before.iter().find(|unit| unit.unit_id == 2049).unwrap().pos;
    let group_ids: Vec<_> = (1..=2048).collect();
    send_order(
        &mut app,
        0,
        group_ids,
        Vec2::new(80.0, 80.0),
        1,
        QueueMode::Replace,
    );
    send_unit_move(&mut app, 0, 2049, 2050, QueueMode::Replace);
    app.update();
    assert!(
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&2049),
        "the live Unit-target envelope waits behind the full 2048-member head"
    );
    assert_eq!(
        snapshot(app.world_mut())
            .iter()
            .find(|unit| unit.unit_id == 2049)
            .unwrap()
            .pos,
        mover_before,
        "a waiting Unit-target move does not move at the stale admission point"
    );

    let target_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2050)
        })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(target_entity)
        .get_mut::<turts_sim::Position>()
        .unwrap()
        .0 = Vec2::new(10.0, 110.0);
    app.update();
    assert!(
        !app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&2049),
        "the waiting Unit-target envelope activates on its later turn"
    );
    let mover_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2049)
        })
        .unwrap()
        .id();
    let mover = app.world().entity(mover_entity);
    assert_eq!(
        mover
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .map(|order| order.target.clone()),
        Some(OrderTargetSim::Unit(2050)),
        "activation preserves the live Unit target identity"
    );
    let progress = mover
        .get::<turts_sim::movement::MoveProgress>()
        .expect("the Unit-target order receives a spread slot");
    let pool = app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0
        .get(&progress.pool_id)
        .unwrap();
    assert_eq!(pool.point, Vec2::new(10.0, 110.0));
    app.world_mut()
        .entity_mut(target_entity)
        .get_mut::<turts_sim::Position>()
        .unwrap()
        .0 = Vec2::new(10.0, 115.0);
    app.update();
    let mover = app.world().entity(mover_entity);
    assert!(
        mover.get::<turts_sim::movement::MoveProgress>().is_none(),
        "a real follow-target change reopens direct following from the previous slot"
    );
    assert!(
        mover
            .get::<turts_sim::movement::PlainMoveProgress>()
            .is_some_and(|progress| {
                progress.original_point.distance(Vec2::new(10.0, 115.0)) <= 0.5001
            })
            || mover
                .get::<MoveTarget>()
                .is_some_and(|target| target.0.distance(Vec2::new(10.0, 115.0)) <= 1.2),
        "the changed target receives a fresh legal approach or fallback rooted at its new goal: target={:?}, plain={:?}, pos={:?}",
        mover.get::<MoveTarget>(),
        mover.get::<turts_sim::movement::PlainMoveProgress>(),
        mover.get::<turts_sim::Position>()
    );
    assert!(
        !app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&2049),
        "ordinary active follow refresh does not recharge the dispatch cap"
    );
}

#[test]
fn merged_unit_follow_keeps_each_members_radius_inset_origin() {
    let mut app = app_with_units_on(
        &[(8.0, 8.0), (8.0, 56.0), (0.5, 32.0)],
        &[0, 0, 1],
        MapSize { x: 64.0, y: 64.0 },
        0.5,
    );
    let large_type = app
        .world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &PackageId(ORIGIN.to_owned()),
            UnitDef {
                id: "large-walker".into(),
                name: "Large Walker".into(),
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 1.0 },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
                armour: None,
                acquire: None,
                provides: Vec::new(),
                consumes: Vec::new(),
                income: None,
                spawn_order: None,
                constructor: None,
                placement: None,
                build_area: None,
                repairer: None,
            },
        )
        .unwrap();
    let large_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2)
        })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(large_entity)
        .insert(turts_sim::UnitType(large_type));

    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                target_kind: turts_mod_api::TargetKind::PointOrUnit,
                policy: None,
                units: vec![1, 2],
                target: OrderTargetSim::Unit(3),
                queue: QueueMode::Replace,
                group: 77,
                allowed_units: Vec::new(),
                queue_depth: 16,
            })),
        });
    app.update();

    let pool = app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0
        .values()
        .next()
        .expect("the merged Move(Unit) envelope creates one shared pool");
    assert_eq!(pool.primary_count, 2);
    assert_eq!(pool.point, Vec2::new(1.0, 32.0));
    for (unit_id, expected_goal) in [(1, Vec2::new(0.5, 32.0)), (2, Vec2::new(1.0, 32.0))] {
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == unit_id)
            })
            .unwrap()
            .id();
        let progress = app
            .world()
            .entity(entity)
            .get::<turts_sim::movement::MoveProgress>()
            .expect("each member retains its initial spread assignment");
        assert_eq!(
            progress.follow_origin,
            Some(turts_sim::movement::FollowMoveOrigin {
                target_unit_id: 3,
                goal: expected_goal,
            }),
            "member {unit_id} radius {:?} got {:?}",
            app.world()
                .resource::<UnitTypes>()
                .get(
                    app.world()
                        .entity(entity)
                        .get::<turts_sim::UnitType>()
                        .unwrap()
                        .0
                )
                .map(|kind| kind.radius),
            progress.follow_origin
        );
        assert_eq!(
            current_target(&mut app, unit_id),
            Some(OrderTargetSim::Unit(3))
        );
    }

    app.update();
    for unit_id in [1, 2] {
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == unit_id)
            })
            .unwrap()
            .id();
        assert!(
            app.world()
                .entity(entity)
                .get::<turts_sim::movement::MoveProgress>()
                .is_some(),
            "same live Unit target must not discard the actor's assigned slot"
        );
        assert_eq!(
            current_target(&mut app, unit_id),
            Some(OrderTargetSim::Unit(3))
        );
    }
}

#[test]
fn pending_unit_target_failure_preserves_identity_and_starts_append() {
    let mut app = app_with_units(&[(8.0, 8.0), (12.0, 8.0)], &[0, 1]);
    *app.world_mut()
        .resource_mut::<turts_sim::orders::MovementDispatchState>() =
        turts_sim::orders::MovementDispatchState {
            tick: Some(1),
            start_slot: 0,
            used: 2048,
            cursor_advanced: true,
        };
    send_unit_move(&mut app, 0, 1, 2, QueueMode::Replace);
    send_hold(&mut app, 0, 1, QueueMode::Append);
    app.update();
    assert!(
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&1),
        "the Unit-target envelope waits under the shared tick budget: tick={:?}, dispatch={:?}, pending={:?}",
        app.world().resource::<Tick>(),
        app.world()
            .resource::<turts_sim::orders::MovementDispatchState>(),
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
    );
    let target_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2)
        })
        .unwrap()
        .id();
    app.world_mut().entity_mut(target_entity).despawn();
    app.update();
    let mover_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    let mover = app.world().entity(mover_entity);
    let orders = mover.get::<UnitOrders>().unwrap();
    assert_eq!(
        orders.last_failure,
        Some(turts_sim::OrderFailure {
            order: turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
            reason: RejectReason::TargetGone,
            tick: 2,
        }),
        "pending activation reports TargetGone under the accepted Move identity"
    );
    assert!(
        orders
            .current
            .as_ref()
            .is_some_and(|order| order.behaviour == turts_mod_api::Behaviour::Hold)
    );
    assert!(mover.get::<MoveTarget>().is_none());
    assert!(
        !app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&1)
    );
}

#[test]
fn pending_replace_keeps_prior_follow_live_and_gates_append_until_activation() {
    let mut app = app_with_units(&[(8.0, 8.0), (16.0, 8.0)], &[0, 0]);
    let mover_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    app.world_mut().entity_mut(mover_entity).insert((
        MoveTarget(Vec2::new(16.0, 8.0)),
        UnitOrders {
            current: Some(turts_sim::Order {
                order: turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                target: OrderTargetSim::Unit(2),
                group: 0,
            }),
            ..Default::default()
        },
    ));
    *app.world_mut()
        .resource_mut::<turts_sim::orders::MovementDispatchState>() =
        turts_sim::orders::MovementDispatchState {
            tick: Some(1),
            start_slot: 0,
            used: 2048,
            cursor_advanced: true,
        };
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(24.0, 24.0),
        0,
        QueueMode::Replace,
    );
    send_hold(&mut app, 0, 1, QueueMode::Append);
    let target_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2)
        })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(target_entity)
        .get_mut::<turts_sim::Position>()
        .unwrap()
        .0 = Vec2::new(20.0, 8.0);

    app.update();

    let mover = app.world().entity(mover_entity);
    assert!(
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&1),
        "the replacement waits while the prior execution runs"
    );
    assert_eq!(
        mover
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .map(|order| order.target.clone()),
        Some(OrderTargetSim::Unit(2)),
        "the prior Unit-follow order remains current until replacement activation"
    );
    let refreshed_target = mover.get::<MoveTarget>().unwrap().0;
    let moved_follow_target = app
        .world()
        .entity(target_entity)
        .get::<turts_sim::Position>()
        .unwrap()
        .0;
    assert_ne!(refreshed_target, Vec2::new(16.0, 8.0));
    assert!(
        (1.0 - 1.0e-4..=1.1 + 1.0e-3).contains(&refreshed_target.distance(moved_follow_target)),
        "the prior follow refresh selects a legal approach to the moved target: {refreshed_target:?}"
    );
    assert_eq!(
        mover
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .iter()
            .map(|order| order.behaviour)
            .collect::<Vec<_>>(),
        vec![turts_mod_api::Behaviour::Hold],
        "the Append remains behind the pending replacement"
    );

    app.update();

    let mover = app.world().entity(mover_entity);
    assert!(
        !app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&1),
        "the replacement activates on its next available turn"
    );
    assert_eq!(
        mover
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .map(|order| order.target.clone()),
        Some(OrderTargetSim::Point(Vec2::new(24.0, 24.0))),
        "the replacement becomes current before its Append"
    );
    assert_eq!(
        mover
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .iter()
            .map(|order| order.behaviour)
            .collect::<Vec<_>>(),
        vec![turts_mod_api::Behaviour::Hold],
        "the Append is still queued after replacement activation"
    );
}

#[test]
fn pending_replace_allows_prior_completion_without_advancing_append() {
    let mut app = app_with_units(&[(8.0, 8.0)], &[0]);
    let mover_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    app.world_mut().entity_mut(mover_entity).insert((
        MoveTarget(Vec2::new(8.1, 8.0)),
        UnitOrders {
            current: Some(turts_sim::Order {
                order: turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(8.1, 8.0)),
                group: 0,
            }),
            ..Default::default()
        },
    ));
    *app.world_mut()
        .resource_mut::<turts_sim::orders::MovementDispatchState>() =
        turts_sim::orders::MovementDispatchState {
            tick: Some(1),
            start_slot: 0,
            used: 2048,
            cursor_advanced: true,
        };
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(24.0, 24.0),
        0,
        QueueMode::Replace,
    );
    send_hold(&mut app, 0, 1, QueueMode::Append);

    app.update();

    *app.world_mut()
        .resource_mut::<turts_sim::orders::MovementDispatchState>() =
        turts_sim::orders::MovementDispatchState {
            tick: Some(2),
            start_slot: 0,
            used: 2048,
            cursor_advanced: true,
        };
    app.update();

    let mover = app.world().entity(mover_entity);
    assert!(
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&1)
    );
    assert!(
        mover.get::<MoveTarget>().is_none(),
        "the prior Move completes normally"
    );
    let orders = mover.get::<UnitOrders>().unwrap();
    assert!(
        orders.current.is_none(),
        "the completed prior Move becomes idle during Pending: current={:?}, target={:?}, progress={:?}, pending={:?}",
        orders.current,
        mover.get::<MoveTarget>(),
        mover.get::<turts_sim::movement::PlainMoveProgress>(),
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
    );
    assert_eq!(
        orders
            .queue
            .iter()
            .map(|order| order.behaviour)
            .collect::<Vec<_>>(),
        vec![turts_mod_api::Behaviour::Hold],
        "the Append remains behind the waiting replacement"
    );

    app.update();

    let mover = app.world().entity(mover_entity);
    assert!(
        !app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_member_counts
            .contains_key(&1)
    );
    assert_eq!(
        mover
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .map(|order| order.target.clone()),
        Some(OrderTargetSim::Point(Vec2::new(24.0, 24.0)))
    );
    assert_eq!(
        mover
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .iter()
            .map(|order| order.behaviour)
            .collect::<Vec<_>>(),
        vec![turts_mod_api::Behaviour::Hold]
    );
}

fn send_unit_move(app: &mut bevy_app::App, slot: u8, id: u32, target_id: u32, queue: QueueMode) {
    send_unit_move_in_group(app, slot, id, target_id, queue, 0);
}

fn send_unit_move_in_group(
    app: &mut bevy_app::App,
    slot: u8,
    id: u32,
    target_id: u32,
    queue: QueueMode,
    group: u32,
) {
    let seq = app
        .world()
        .resource::<Tick>()
        .0
        .wrapping_add(101)
        .wrapping_add(app.world().resource::<CommandQueue>().0.len() as u32);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(slot),
            seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
                behaviour: turts_mod_api::Behaviour::Move,
                target_kind: turts_mod_api::TargetKind::PointOrUnit,
                policy: None,
                units: vec![id],
                target: OrderTargetSim::Unit(target_id),
                queue,
                group,
                allowed_units: Vec::new(),
                queue_depth: 16,
            })),
        });
}

fn send_stop(app: &mut bevy_app::App, slot: u8, id: u32) {
    let seq = app.world().resource::<Tick>().0.wrapping_add(50);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(slot),
            seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: turts_mod_api::ContentId::parse("fixture_pkg:stop").unwrap(),
                behaviour: turts_mod_api::Behaviour::Stop,
                target_kind: turts_mod_api::TargetKind::None,
                policy: None,
                units: vec![id],
                target: OrderTargetSim::None,
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: Vec::new(),
                queue_depth: 16,
            })),
        });
}

fn send_hold(app: &mut bevy_app::App, slot: u8, id: u32, queue: QueueMode) {
    let seq = app
        .world()
        .resource::<Tick>()
        .0
        .wrapping_add(51)
        .wrapping_add(app.world().resource::<CommandQueue>().0.len() as u32);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(slot),
            seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: turts_mod_api::ContentId::parse("fixture_pkg:hold").unwrap(),
                behaviour: turts_mod_api::Behaviour::Hold,
                target_kind: turts_mod_api::TargetKind::None,
                policy: None,
                units: vec![id],
                target: OrderTargetSim::None,
                queue,
                group: 0,
                allowed_units: Vec::new(),
                queue_depth: 16,
            })),
        });
}

fn current_target(app: &mut bevy_app::App, id: u32) -> Option<OrderTargetSim> {
    let entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|unit| unit.0 == id)
        })
        .map(|entity| entity.id())?;
    app.world()
        .entity(entity)
        .get::<UnitOrders>()?
        .current
        .as_ref()
        .map(|order| order.target.clone())
}

fn count_current_target(
    app: &mut bevy_app::App,
    ids: std::ops::RangeInclusive<u32>,
    target: Vec2,
) -> usize {
    ids.filter(|id| current_target(app, *id) == Some(OrderTargetSim::Point(target)))
        .count()
}

fn rests_without_overlap(app: &mut bevy_app::App, ids: &[u32]) -> bool {
    let units = snapshot(app.world_mut());
    let selected: Vec<_> = units
        .iter()
        .filter(|unit| ids.contains(&unit.unit_id))
        .collect();
    for i in 0..selected.len() {
        for j in i + 1..selected.len() {
            let dx = selected[i].pos.x - selected[j].pos.x;
            let dy = selected[i].pos.y - selected[j].pos.y;
            if dx * dx + dy * dy < 1.0 - 1.0e-5 {
                return false;
            }
        }
    }
    true
}

fn overlap_pairs(app: &mut bevy_app::App, ids: &[u32]) -> Vec<(u32, u32, f32)> {
    let units = snapshot(app.world_mut());
    let selected: Vec<_> = units
        .iter()
        .filter(|unit| ids.contains(&unit.unit_id))
        .collect();
    let mut pairs = Vec::new();
    for i in 0..selected.len() {
        for j in i + 1..selected.len() {
            let distance = selected[i].pos.distance(selected[j].pos);
            if distance < 1.0 - 1.0e-5 {
                pairs.push((selected[i].unit_id, selected[j].unit_id, distance));
            }
        }
    }
    pairs
}

fn horizon_ticks(positions: &[(f32, f32)], target: Vec2) -> usize {
    let farthest = positions
        .iter()
        .map(|(x, y)| Vec2::new(*x, *y).distance(target))
        .fold(0.0_f32, f32::max);
    ((2.0 * farthest / 3.0 * 20.0).ceil() as usize) + 5 * 20
}

fn set_positions(app: &mut bevy_app::App, first_id: u32, positions: &[(f32, f32)]) {
    for (offset, &(x, y)) in positions.iter().enumerate() {
        let id = first_id + offset as u32;
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == id)
            })
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(entity)
            .insert(turts_sim::Position(Vec2::new(x, y)));
    }
}

fn assert_settled(app: &mut bevy_app::App, ids: &[u32]) {
    assert_eq!(
        snapshot(app.world_mut())
            .iter()
            .filter(|unit| ids.contains(&unit.unit_id))
            .count(),
        ids.len(),
        "every commanded unit still exists"
    );
    assert!(
        rests_without_overlap(app, ids),
        "footprint discs overlap at rest: {:?}",
        overlap_pairs(app, ids)
    );
    for id in ids {
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == *id)
            })
            .unwrap();
        assert!(
            entity.get::<MoveTarget>().is_none(),
            "unit {id} still has a movement target"
        );
        assert!(
            !entity
                .get::<UnitOrders>()
                .and_then(|orders| orders.last_failure.as_ref())
                .is_some_and(|failure| failure.reason == RejectReason::Blocked),
            "unit {id} failed with Blocked: pos={:?}, progress={:?}, order={:?}",
            entity.get::<turts_sim::Position>(),
            entity.get::<turts_sim::movement::MoveProgress>(),
            entity
                .get::<UnitOrders>()
                .and_then(|orders| orders.current.as_ref())
        );
    }
    let before: Vec<_> = snapshot(app.world_mut())
        .into_iter()
        .filter(|unit| ids.contains(&unit.unit_id))
        .map(|unit| unit.pos)
        .collect();
    for _ in 0..5 {
        app.update();
    }
    let after: Vec<_> = snapshot(app.world_mut())
        .into_iter()
        .filter(|unit| ids.contains(&unit.unit_id))
        .map(|unit| unit.pos)
        .collect();
    assert_eq!(before, after, "resting positions are stable");
}

fn run_through_escape_to_stable_rest(app: &mut bevy_app::App, ids: &[u32]) {
    let map_diameter = 64.0_f32 * 2.0_f32.sqrt();
    let horizon = (2.0 * map_diameter / 3.0 * 20.0).ceil() as usize + 5 * 20;
    let mut previous = Vec::new();
    let mut stable_ticks = 0;
    for _ in 0..horizon {
        app.update();
        let all_rest = ids.iter().all(|id| {
            app.world_mut()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|unit| unit.0 == *id)
                })
                .is_some_and(|entity| entity.get::<MoveTarget>().is_none())
        });
        if all_rest {
            let positions: Vec<_> = snapshot(app.world_mut())
                .into_iter()
                .filter(|unit| ids.contains(&unit.unit_id))
                .map(|unit| unit.pos)
                .collect();
            if positions == previous {
                stable_ticks += 1;
            } else {
                stable_ticks = 0;
            }
            previous = positions;
            if stable_ticks >= 40 {
                break;
            }
        } else {
            stable_ticks = 0;
            previous.clear();
        }
    }
    assert!(
        stable_ticks >= 40,
        "blocked units did not finish escaping and remain stable within the map-derived horizon: {:?}",
        ids.iter()
            .filter_map(|id| {
                let entity = app.world_mut().iter_entities().find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|unit| unit.0 == *id)
                })?;
                entity.get::<MoveTarget>().map(|target| {
                    (
                        *id,
                        entity.get::<turts_sim::Position>().copied(),
                        target.0,
                        entity.get::<turts_sim::movement::MoveProgress>().cloned(),
                    )
                })
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        snapshot(app.world_mut())
            .iter()
            .filter(|unit| ids.contains(&unit.unit_id))
            .count(),
        ids.len()
    );
    assert!(
        rests_without_overlap(app, ids),
        "rested/escaped units overlap: {:?}",
        overlap_pairs(app, ids)
    );
}

#[test]
fn forty_units_rest_with_no_overlap_and_no_blocked() {
    let positions: Vec<_> = (0..40)
        .map(|k| (10.0 + 1.2 * (k % 8) as f32, 10.0 + 1.2 * (k / 8) as f32))
        .collect();
    let mut app = app_with_units(&positions, &[]);
    let target = Vec2::new(40.0, 30.0);
    send_order(
        &mut app,
        0,
        (1..=40).collect(),
        target,
        1,
        QueueMode::Replace,
    );
    for _ in 0..horizon_ticks(&positions, target) {
        app.update();
    }
    assert_settled(&mut app, &(1..=40).collect::<Vec<_>>());
    assert!(
        snapshot(app.world_mut())
            .iter()
            .all(|unit| (unit.pos.x - target.x).hypot(unit.pos.y - target.y) <= 4.58)
    );
}

#[test]
fn one_unit_alone_takes_the_point() {
    let mut app = app_with_units(&[(8.0, 8.0)], &[]);
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(32.0, 32.0),
        1,
        QueueMode::Replace,
    );
    for _ in 0..300 {
        app.update();
    }
    assert_eq!(snapshot(app.world_mut())[0].pos, Vec2::new(32.0, 32.0));
    assert!(
        app.world_mut()
            .query::<&MoveTarget>()
            .iter(app.world())
            .next()
            .is_none()
    );
}

#[test]
fn ring_to_point() {
    let positions: Vec<_> = (0..40)
        .map(|i| {
            let angle = std::f32::consts::TAU * i as f32 / 40.0;
            (32.0 + angle.cos() * 7.0, 32.0 + angle.sin() * 7.0)
        })
        .collect();
    let mut app = app_with_units(&positions, &[]);
    send_order(
        &mut app,
        0,
        (1..=40).collect(),
        Vec2::new(32.0, 32.0),
        2,
        QueueMode::Replace,
    );
    for _ in 0..400 {
        app.update();
    }
    assert_settled(&mut app, &(1..=40).collect::<Vec<_>>());
    assert!(
        snapshot(app.world_mut()).iter().all(|unit| {
            let dx = unit.pos.x - 32.0;
            let dy = unit.pos.y - 32.0;
            dx * dx + dy * dy <= 4.58_f32.powi(2)
        }),
        "units are outside the arrival radius"
    );
}

#[test]
fn second_group_same_point() {
    let mut positions: Vec<_> = (0..40)
        .map(|k| (10.0 + 1.2 * (k % 8) as f32, 10.0 + 1.2 * (k / 8) as f32))
        .collect();
    positions.extend((0..40).map(|k| (5.0 + 1.2 * (k % 8) as f32, 50.0 + 1.2 * (k / 8) as f32)));
    let owners: Vec<_> = (0..80).map(|i| if i < 40 { 0 } else { 1 }).collect();
    let mut app = app_with_units(&positions, &owners);
    let target = Vec2::new(40.0, 30.0);
    send_order(
        &mut app,
        0,
        (1..=40).collect(),
        target,
        7,
        QueueMode::Replace,
    );
    for _ in 0..horizon_ticks(&positions[..40], target) {
        app.update();
    }
    assert_settled(&mut app, &(1..=40).collect::<Vec<_>>());
    let second_positions: Vec<_> = (0..40)
        .map(|k| (10.0 + 1.2 * (k % 8) as f32, 30.0 + 1.2 * (k / 8) as f32))
        .collect();
    set_positions(&mut app, 41, &second_positions);
    send_order(
        &mut app,
        1,
        (41..=80).collect(),
        target,
        7,
        QueueMode::Replace,
    );
    for _ in 0..horizon_ticks(&second_positions, target) {
        app.update();
    }
    assert_eq!(snapshot(app.world_mut()).len(), 80);
    assert_settled(&mut app, &(1..=80).collect::<Vec<_>>());
    assert!(
        snapshot(app.world_mut())
            .iter()
            .all(|unit| (unit.pos.x - target.x).hypot(unit.pos.y - target.y) <= 6.2)
    );
}

#[test]
fn new_order_while_moving() {
    let positions: Vec<_> = (0..40)
        .map(|i| (10.0 + 1.2 * (i % 8) as f32, 10.0 + 1.2 * (i / 8) as f32))
        .collect();
    let mut app = app_with_units(&positions, &[]);
    send_order(
        &mut app,
        0,
        (1..=40).collect(),
        Vec2::new(40.0, 30.0),
        9,
        QueueMode::Replace,
    );
    for _ in 0..20 {
        app.update();
    }
    send_order(
        &mut app,
        0,
        (1..=40).collect(),
        Vec2::new(50.0, 36.0),
        10,
        QueueMode::Replace,
    );
    for _ in 0..400 {
        app.update();
    }
    assert_settled(&mut app, &(1..=40).collect::<Vec<_>>());
    assert!(
        snapshot(app.world_mut())
            .iter()
            .all(|unit| (unit.pos.x - 50.0).hypot(unit.pos.y - 36.0) <= 4.58)
    );
}

#[test]
fn queued_point_move_uses_a_new_full_pool_after_partial_old_pool_arrival() {
    let mut app = app_with_units(&[(10.0, 10.0), (11.2, 10.0)], &[]);
    send_order(
        &mut app,
        0,
        vec![1, 2],
        Vec2::new(30.0, 30.0),
        1,
        QueueMode::Replace,
    );
    app.update();

    let first = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    let second = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2)
        })
        .unwrap()
        .id();
    let old_pool = app
        .world()
        .entity(first)
        .get::<turts_sim::movement::MoveProgress>()
        .unwrap()
        .pool_id;
    assert_eq!(
        app.world()
            .entity(second)
            .get::<turts_sim::movement::MoveProgress>()
            .unwrap()
            .pool_id,
        old_pool
    );

    let first_destination = app.world().entity(first).get::<MoveTarget>().unwrap().0;
    app.world_mut()
        .entity_mut(first)
        .get_mut::<turts_sim::Position>()
        .unwrap()
        .0 = first_destination;
    app.world_mut().entity_mut(first).remove::<MoveTarget>();
    app.world_mut()
        .entity_mut(first)
        .get_mut::<turts_sim::movement::MoveProgress>()
        .unwrap()
        .activity = turts_sim::movement::MovementActivity::Resting;

    let next = Vec2::new(55.0, 55.0);
    let after_next = Vec2::new(50.0, 60.0);
    send_order(&mut app, 0, vec![1, 2], next, 2, QueueMode::Append);
    send_order(&mut app, 0, vec![1, 2], after_next, 3, QueueMode::Append);
    app.update();

    let first_orders = app.world().entity(first).get::<UnitOrders>().unwrap();
    assert_eq!(
        first_orders.current.as_ref().unwrap().target,
        OrderTargetSim::Point(next)
    );
    assert_eq!(first_orders.queue.len(), 1);
    assert_eq!(
        first_orders.queue[0].target,
        OrderTargetSim::Point(after_next)
    );
    let first_progress = app
        .world()
        .entity(first)
        .get::<turts_sim::movement::MoveProgress>()
        .unwrap();
    assert_ne!(
        first_progress.pool_id, old_pool,
        "new move uses a fresh pool"
    );
    assert_eq!(
        first_progress.activity,
        turts_sim::movement::MovementActivity::Moving
    );
    assert_eq!(
        app.world()
            .entity(second)
            .get::<turts_sim::movement::MoveProgress>()
            .unwrap()
            .pool_id,
        old_pool,
        "the other member keeps moving in its prior pool"
    );
    let new_pool = &app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0[&first_progress.pool_id];
    assert_eq!(new_pool.primary_count, 1);
    assert_eq!(new_pool.slots.len(), 7);
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert!(pending.queues.is_empty());
    assert_eq!(pending.pending_units.get(&0), Some(&0));
}

#[test]
fn cases_f_and_i_rest_with_no_overlap() {
    // F: the second 40-unit group is ordered while the first is moving to P.
    let mut f_positions: Vec<_> = (0..40)
        .map(|i| (10.0 + 1.2 * (i % 8) as f32, 10.0 + 1.2 * (i / 8) as f32))
        .collect();
    f_positions.extend((0..40).map(|i| (10.0 + 1.2 * (i % 8) as f32, 30.0 + 1.2 * (i / 8) as f32)));
    let f_owners: Vec<_> = (0..80).map(|i| if i < 40 { 0 } else { 1 }).collect();
    let mut f = app_with_units(&f_positions, &f_owners);
    let p = Vec2::new(40.0, 30.0);
    send_order(&mut f, 0, (1..=40).collect(), p, 11, QueueMode::Replace);
    f.update();
    for _ in 0..39 {
        f.update();
    }
    send_order(&mut f, 1, (41..=80).collect(), p, 12, QueueMode::Replace);
    run_through_escape_to_stable_rest(&mut f, &(1..=80).collect::<Vec<_>>());
    assert_eq!(snapshot(f.world_mut()).len(), 80);

    // I: a second group walks through a settled group and escapes every overlap.
    let mut i_positions: Vec<_> = (0..40)
        .map(|i| (10.0 + 1.2 * (i % 8) as f32, 10.0 + 1.2 * (i / 8) as f32))
        .collect();
    i_positions.extend((0..40).map(|i| (10.0 + 1.2 * (i % 8) as f32, 32.0 + 1.2 * (i / 8) as f32)));
    let i_owners: Vec<_> = (0..80).map(|n| if n < 40 { 0 } else { 1 }).collect();
    let mut i_app = app_with_units(&i_positions, &i_owners);
    send_order(
        &mut i_app,
        0,
        (1..=40).collect(),
        Vec2::new(25.0, 25.0),
        13,
        QueueMode::Replace,
    );
    for _ in 0..400 {
        i_app.update();
    }
    send_order(
        &mut i_app,
        1,
        (41..=80).collect(),
        Vec2::new(40.0, 18.0),
        14,
        QueueMode::Replace,
    );
    run_through_escape_to_stable_rest(&mut i_app, &(1..=80).collect::<Vec<_>>());
    assert_eq!(snapshot(i_app.world_mut()).len(), 80);
}

#[test]
fn two_twenty_groups_cross_without_long_overlap() {
    let mut positions: Vec<_> = (0..20)
        .map(|k| (10.0 + 1.2 * (k % 5) as f32, 20.0 + 1.2 * (k / 5) as f32))
        .collect();
    positions.extend((0..20).map(|k| (24.0 + 1.2 * (k % 5) as f32, 6.0 + 1.2 * (k / 5) as f32)));
    let owners: Vec<_> = (0..40).map(|id| if id < 20 { 0 } else { 1 }).collect();
    let mut app = app_with_units(&positions, &owners);
    send_order(
        &mut app,
        0,
        (1..=20).collect(),
        Vec2::new(40.0, 22.0),
        31,
        QueueMode::Replace,
    );
    send_order(
        &mut app,
        1,
        (21..=40).collect(),
        Vec2::new(26.0, 36.0),
        32,
        QueueMode::Replace,
    );
    let mut pair_runs = std::collections::BTreeMap::<(u32, u32), usize>::new();
    let mut bad_pairs = std::collections::BTreeSet::new();
    for _ in 0..600 {
        app.update();
        let units = snapshot(app.world_mut());
        for a in units.iter().filter(|unit| unit.unit_id <= 20) {
            for b in units.iter().filter(|unit| unit.unit_id > 20) {
                let pair = (a.unit_id, b.unit_id);
                let overlaps = (a.pos.x - b.pos.x).hypot(a.pos.y - b.pos.y) < 0.9;
                if overlaps {
                    let run = pair_runs.entry(pair).or_default();
                    *run += 1;
                    if *run > 10 {
                        bad_pairs.insert(pair);
                    }
                } else {
                    pair_runs.insert(pair, 0);
                }
            }
        }
    }
    assert!(
        bad_pairs.is_empty(),
        "pairs exceeded 20% radius overlap for more than 10 consecutive ticks: {bad_pairs:?}"
    );
    assert!(rests_without_overlap(
        &mut app,
        &(1..=40).collect::<Vec<_>>()
    ));
}

#[test]
fn outside_target_is_clamped() {
    let mut app = app_with_units(&[(8.0, 8.0)], &[]);
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(100.0, 100.0),
        1,
        QueueMode::Replace,
    );
    app.update();
    assert!(
        app.world().resource::<CommandResults>().0[0]
            .outcome
            .is_ok()
    );
    for _ in 0..600 {
        app.update();
    }
    let unit = &snapshot(app.world_mut())[0];
    assert!((unit.pos.x - 63.5).abs() < 1.0e-4 && (unit.pos.y - 63.5).abs() < 1.0e-4);
}

#[test]
fn queued_outside_point_is_clamped_to_the_radius_inset() {
    let mut app = app_with_units(&[(8.0, 8.0)], &[]);
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(20.0, 20.0),
        3,
        QueueMode::Replace,
    );
    app.update();
    let entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    let first_target = app.world().entity(entity).get::<MoveTarget>().unwrap().0;
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(100.0, 100.0),
        4,
        QueueMode::Append,
    );
    app.update();
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .len(),
        1
    );

    app.world_mut()
        .entity_mut(entity)
        .get_mut::<turts_sim::Position>()
        .unwrap()
        .0 = first_target;
    app.world_mut().entity_mut(entity).remove::<MoveTarget>();
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<turts_sim::movement::MoveProgress>()
        .unwrap()
        .activity = turts_sim::movement::MovementActivity::Resting;
    app.update();

    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>().unwrap().0,
        Vec2::new(63.5, 63.5),
        "queued target is clamped using the unit radius before movement resumes"
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
        OrderTargetSim::Point(Vec2::new(100.0, 100.0)),
        "the authored order retains its original point while MoveTarget uses the radius inset"
    );
}

#[test]
fn nan_target_is_refused() {
    let mut app = app_with_units(&[(8.0, 8.0)], &[]);
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(f32::NAN, 9.0),
        1,
        QueueMode::Replace,
    );
    app.update();
    assert_eq!(
        app.world().resource::<CommandResults>().0[0].units.refused,
        vec![(1, RejectReason::TargetNotFinite)]
    );
}

#[test]
fn post_target_gives_a_ring() {
    let mut app = headless_app(20);
    let origin = PackageId(ORIGIN.to_owned());
    for def in [
        UnitDef {
            id: "walker".into(),
            name: "Walker".into(),
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
            armour: None,
            acquire: None,
            provides: Vec::new(),
            consumes: Vec::new(),
            income: None,
            spawn_order: None,
            constructor: None,
            placement: None,
            build_area: None,
            repairer: None,
        },
        UnitDef {
            id: "post".into(),
            name: "Post".into(),
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            footprint: Footprint::Circle { radius: 2.0 },
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
            armour: None,
            acquire: None,
            provides: Vec::new(),
            consumes: Vec::new(),
            income: None,
            spawn_order: None,
            constructor: None,
            placement: None,
            build_area: None,
            repairer: None,
        },
    ] {
        app.world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(&origin, def)
            .unwrap();
    }
    let map = MapDef {
        id: "obstacle".into(),
        size: MapSize { x: 64.0, y: 64.0 },
        placements: vec![
            Placement {
                unit: format!("{ORIGIN}:walker"),
                owner_slot: 0,
                pos: WorldPos { x: 20.0, y: 32.0 },
                contents: Vec::new(),
                facing: None,
                start_order: None,
            },
            Placement {
                unit: format!("{ORIGIN}:walker"),
                owner_slot: 0,
                pos: WorldPos { x: 20.0, y: 33.2 },
                contents: Vec::new(),
                facing: None,
                start_order: None,
            },
            Placement {
                unit: format!("{ORIGIN}:post"),
                owner_slot: 0,
                pos: WorldPos { x: 32.0, y: 32.0 },
                contents: Vec::new(),
                facing: None,
                start_order: None,
            },
        ],
        nodes: Vec::new(),
        slot_economy: Vec::new(),
        belts: Vec::new(),
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(44.0, 32.0),
        0,
        QueueMode::Replace,
    );
    send_order(
        &mut app,
        0,
        vec![2],
        Vec2::new(32.0, 32.0),
        15,
        QueueMode::Replace,
    );
    for _ in 0..500 {
        app.update();
    }
    assert_settled(&mut app, &[1, 2]);
    let units = snapshot(app.world_mut());
    assert_eq!(
        units.iter().find(|unit| unit.unit_id == 1).unwrap().pos,
        Vec2::new(44.0, 32.0),
        "group-zero mover passes the post on its way to a free point"
    );
    assert!(
        (units.iter().find(|unit| unit.unit_id == 2).unwrap().pos - Vec2::new(32.0, 32.0)).length()
            >= 2.5,
        "nonzero singleton takes a free ring point around the post"
    );
    assert_eq!(
        units.iter().find(|unit| unit.unit_id == 3).unwrap().pos,
        Vec2::new(32.0, 32.0),
        "the stationary post remains fixed"
    );
}

#[test]
fn group_zero_singleton_finds_a_free_point_beyond_a_large_post() {
    let mut app = headless_app(20);
    let origin = PackageId(ORIGIN.to_owned());
    for def in [
        UnitDef {
            id: "walker".into(),
            name: "Walker".into(),
            movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            footprint: Footprint::Circle { radius: 0.01 },
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
            armour: None,
            acquire: None,
            provides: Vec::new(),
            consumes: Vec::new(),
            income: None,
            spawn_order: None,
            constructor: None,
            placement: None,
            build_area: None,
            repairer: None,
        },
        UnitDef {
            id: "large_post".into(),
            name: "Large Post".into(),
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            footprint: Footprint::Circle { radius: 50.0 },
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
            armour: None,
            acquire: None,
            provides: Vec::new(),
            consumes: Vec::new(),
            income: None,
            spawn_order: None,
            constructor: None,
            placement: None,
            build_area: None,
            repairer: None,
        },
    ] {
        app.world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(&origin, def)
            .unwrap();
    }
    let map = MapDef {
        id: "large_obstacle".into(),
        size: MapSize {
            x: 2048.0,
            y: 2048.0,
        },
        placements: vec![
            Placement {
                unit: format!("{ORIGIN}:walker"),
                owner_slot: 0,
                pos: WorldPos {
                    x: 700.0,
                    y: 1024.0,
                },
                contents: Vec::new(),
                facing: None,
                start_order: None,
            },
            Placement {
                unit: format!("{ORIGIN}:large_post"),
                owner_slot: 0,
                pos: WorldPos {
                    x: 1024.0,
                    y: 1024.0,
                },
                contents: Vec::new(),
                facing: None,
                start_order: None,
            },
        ],
        nodes: Vec::new(),
        slot_economy: Vec::new(),
        belts: Vec::new(),
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    let center = Vec2::new(1024.0, 1024.0);
    send_order(&mut app, 0, vec![1], center, 0, QueueMode::Replace);

    for _ in 0..3000 {
        app.update();
    }
    let units = snapshot(app.world_mut());
    let mover = units.iter().find(|unit| unit.unit_id == 1).unwrap();
    let post = units.iter().find(|unit| unit.unit_id == 2).unwrap();
    assert!(
        mover.pos.distance(center) >= 50.01 - 1.0e-3,
        "singleton settled outside the post footprint: {:?}",
        mover.pos
    );
    assert_eq!(
        app.world()
            .iter_entities()
            .find(|entity| entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1))
            .unwrap()
            .get::<MoveTarget>(),
        None,
        "singleton reached a free destination and is resting"
    );
    assert_eq!(post.pos, center, "post does not move");
}

#[test]
fn plain_singleton_blocked_by_a_full_width_post_reports_failure_by_gmu_deadline() {
    let mut app = app_with_units_on(&[(0.5, 2.5)], &[], MapSize { x: 7.0, y: 5.0 }, 0.5);
    let post_type = app
        .world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &PackageId(ORIGIN.to_owned()),
            UnitDef {
                id: "wide-post".into(),
                name: "Wide post".into(),
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 2.0 },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
                armour: None,
                acquire: None,
                provides: Vec::new(),
                consumes: Vec::new(),
                income: None,
                spawn_order: None,
                constructor: None,
                placement: None,
                build_area: None,
                repairer: None,
            },
        )
        .unwrap();
    app.world_mut().spawn((
        turts_sim::UnitId(2),
        turts_sim::UnitType(post_type),
        turts_sim::Owner(SlotId(0)),
        turts_sim::Position(Vec2::new(3.5, 2.5)),
    ));

    send_order(
        &mut app,
        0,
        vec![1],
        Vec2::new(6.5, 2.5),
        0,
        QueueMode::Replace,
    );
    for _ in 0..200 {
        app.update();
    }
    let mover_id = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    let mover = app.world().entity(mover_id);
    let position = mover.get::<turts_sim::Position>().unwrap().0;
    let target = mover.get::<MoveTarget>().map(|target| target.0);
    let orders = mover.get::<UnitOrders>();
    assert!(
        target.is_none()
            || orders
                .and_then(|orders| orders.last_failure.as_ref())
                .is_some_and(|failure| failure.reason == RejectReason::Blocked),
        "a plain singleton must stop with no target or a Blocked failure by the 10s GMU bound; position={:?}, target={:?}, orders={:?}",
        position,
        target,
        orders
    );
    assert_eq!(
        mover
            .get::<turts_sim::movement::PlainMoveProgress>()
            .map(|progress| progress.activity),
        Some(turts_sim::movement::MovementActivity::Blocked),
        "the plain move must retain an observable Blocked activity after its target is removed"
    );
    let progress = mover
        .get::<turts_sim::movement::PlainMoveProgress>()
        .unwrap();
    assert!(
        progress.pool_slots.len() <= 6,
        "N=1 pool has at most six spares"
    );
    assert!(
        progress.claim_count <= 3,
        "plain fallback uses at most three claims"
    );
    assert!(
        progress.failure_recorded,
        "Orders acknowledges Blocked once"
    );
    assert_eq!(
        orders
            .and_then(|orders| orders.last_failure.as_ref())
            .map(|failure| (failure.order.clone(), failure.reason)),
        Some((
            turts_mod_api::ContentId::parse("fixture_pkg:move").unwrap(),
            RejectReason::Blocked,
        )),
        "an authored plain move retains its accepted order identity"
    );
    assert_eq!(
        turts_sim::world_view(app.world_mut())
            .unit_orders
            .iter()
            .find(|row| row.unit_id == 1)
            .map(|row| row.activity),
        Some(turts_sim::economy::views::UnitOrderActivityView::Blocked),
        "the owner-facing activity reports Blocked after acknowledgement"
    );
}

fn full_width_post_app() -> bevy_app::App {
    let mut app = app_with_units_on(&[(0.5, 2.5)], &[], MapSize { x: 7.0, y: 5.0 }, 0.5);
    let post_type = app
        .world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &PackageId(ORIGIN.to_owned()),
            UnitDef {
                id: "wide-post".into(),
                name: "Wide post".into(),
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 2.0 },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
                armour: None,
                acquire: None,
                provides: Vec::new(),
                consumes: Vec::new(),
                income: None,
                spawn_order: None,
                constructor: None,
                placement: None,
                build_area: None,
                repairer: None,
            },
        )
        .unwrap();
    app.world_mut().spawn((
        turts_sim::UnitId(2),
        turts_sim::UnitType(post_type),
        turts_sim::Owner(SlotId(0)),
        turts_sim::Position(Vec2::new(3.5, 2.5)),
    ));
    app
}

#[test]
fn blocked_plain_move_escape_survives_queued_and_pre_ack_holds() {
    use turts_sim::economy::views::UnitOrderActivityView as Activity;
    for replace_before_ack in [false, true] {
        let mut app = full_width_post_app();
        app.world_mut()
            .resource_mut::<turts_sim::AuditTrail>()
            .enable();
        send_order(
            &mut app,
            0,
            vec![1],
            Vec2::new(6.5, 2.5),
            0,
            QueueMode::Replace,
        );
        if !replace_before_ack {
            send_hold(&mut app, 0, 1, QueueMode::Append);
        }

        let mut blocked_output_seen = false;
        for _ in 0..200 {
            app.update();
            let entity = app
                .world_mut()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|id| id.0 == 1)
                })
                .unwrap()
                .id();
            let mover = app.world().entity(entity);
            if mover.get::<turts_sim::movement::PhysicalEscape>().is_some() {
                blocked_output_seen = true;
                if replace_before_ack {
                    let progress = mover
                        .get::<turts_sim::movement::PlainMoveProgress>()
                        .expect(
                            "the movement result is still present before Orders acknowledges it",
                        );
                    assert_eq!(
                        progress.activity,
                        turts_sim::movement::MovementActivity::Blocked
                    );
                    assert!(!progress.failure_recorded);
                    send_hold(&mut app, 0, 1, QueueMode::Replace);
                }
                break;
            }
        }
        assert!(
            blocked_output_seen,
            "Blocked movement output creates escape before Order cleanup"
        );

        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap()
            .id();
        let before_cleanup_escape = app
            .world()
            .entity(entity)
            .get::<turts_sim::movement::PhysicalEscape>()
            .is_some();
        assert!(before_cleanup_escape);
        app.update();
        let after_cleanup = app.world().entity(entity);
        assert_eq!(
            after_cleanup
                .get::<UnitOrders>()
                .unwrap()
                .current
                .as_ref()
                .map(|order| order.behaviour),
            Some(turts_mod_api::Behaviour::Hold),
            "normal queue/replacement continuation remains immediate"
        );
        assert!(
            after_cleanup
                .get::<turts_sim::movement::PhysicalEscape>()
                .is_some(),
            "Hold cleanup cannot erase the actor's physical escape obligation"
        );

        let mut previous = app
            .world()
            .entity(entity)
            .get::<turts_sim::Position>()
            .unwrap()
            .0;
        let mut cleared = false;
        for _ in 0..300 {
            app.update();
            let mover = app.world().entity(entity);
            let position = mover.get::<turts_sim::Position>().unwrap().0;
            assert!(
                position.distance(previous) <= 0.150_001,
                "escape takes at most one speed-times-delta step"
            );
            previous = position;
            if mover.get::<turts_sim::movement::PhysicalEscape>().is_none() {
                cleared = true;
                break;
            }
        }
        assert!(cleared, "physical escape ends once penetration clears");
        let post = Vec2::new(3.5, 2.5);
        assert!(previous.distance(post) >= 2.5 - 1.0e-5);
        let view = turts_sim::world_view(app.world_mut());
        assert_eq!(
            view.unit_orders
                .iter()
                .find(|row| row.unit_id == 1)
                .unwrap()
                .activity,
            Activity::Holding,
            "new/queued Hold activity appears after the physical obligation clears"
        );
        let settled = previous;
        for _ in 0..40 {
            app.update();
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<turts_sim::Position>()
                    .unwrap()
                    .0,
                settled,
                "the cleared actor rests stably"
            );
        }
    }
}

#[test]
fn unit_follow_move_completes_at_its_reached_legal_fallback() {
    let mut app = app_with_units_on(
        &[(2.0, 8.0), (8.0, 8.0)],
        &[0, 1],
        MapSize { x: 16.0, y: 16.0 },
        0.5,
    );
    send_unit_move(&mut app, 0, 1, 2, QueueMode::Replace);
    send_hold(&mut app, 0, 1, QueueMode::Append);

    let mut completed_tick = None;
    let mut observed_rest = None;
    for _ in 0..300 {
        let mover_entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap()
            .id();
        let prior_mover = app.world().entity(mover_entity);
        let plain_resting = prior_mover
            .get::<turts_sim::movement::PlainMoveProgress>()
            .is_some_and(|progress| {
                progress.activity == turts_sim::movement::MovementActivity::Resting
                    && progress.slot == prior_mover.get::<turts_sim::Position>().unwrap().0
            });
        let cohort_resting = prior_mover
            .get::<turts_sim::movement::MoveProgress>()
            .is_some_and(|progress| {
                progress.activity == turts_sim::movement::MovementActivity::Resting
                    && progress.slot == prior_mover.get::<turts_sim::Position>().unwrap().0
                    && app
                        .world()
                        .resource::<turts_sim::movement::MovementPools>()
                        .0
                        .get(&progress.pool_id)
                        .is_some_and(|pool| pool.point == Vec2::new(8.0, 8.0))
            });
        if plain_resting || cohort_resting {
            observed_rest = Some(prior_mover.get::<turts_sim::Position>().unwrap().0);
        }
        app.update();
        let units = snapshot(app.world_mut());
        let mover = units.iter().find(|unit| unit.unit_id == 1).unwrap();
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap()
            .id();
        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        if orders
            .current
            .as_ref()
            .is_some_and(|order| order.behaviour == turts_mod_api::Behaviour::Hold)
        {
            completed_tick = Some(app.world().resource::<Tick>().0);
            assert!(mover.pos.distance(Vec2::new(8.0, 8.0)) > 0.5);
            let rested_at = observed_rest.expect("follow move reaches and rests at a legal slot");
            assert_eq!(
                mover.pos, rested_at,
                "completion preserves the reached slot"
            );
            assert!(
                mover.pos.distance(Vec2::new(8.0, 8.0)) >= 1.0 - 1.0e-5,
                "the completed fallback remains outside the sum of unit radii"
            );
            assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
            assert!(
                app.world()
                    .entity(entity)
                    .get::<turts_sim::movement::MoveProgress>()
                    .is_none()
            );
            assert!(
                app.world()
                    .entity(entity)
                    .get::<turts_sim::movement::PlainMoveProgress>()
                    .is_none()
            );
            break;
        }
    }
    let completed_tick = completed_tick.expect("a rested legal fallback completes Move(Unit)");
    assert!(
        completed_tick <= 100,
        "completion is prompt, tick={completed_tick}"
    );
    let entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    let settled = app
        .world()
        .entity(entity)
        .get::<turts_sim::Position>()
        .unwrap()
        .0;
    for _ in 0..40 {
        app.update();
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<turts_sim::Position>()
                .unwrap()
                .0,
            settled
        );
    }
}

#[test]
fn mixed_radius_unit_cohort_consumes_resting_follow_origin_in_orders() {
    let mut app = app_with_units_on(
        &[(2.0, 8.0), (2.0, 4.0), (8.0, 8.0), (14.0, 8.0)],
        &[0, 0, 1, 1],
        MapSize { x: 16.0, y: 16.0 },
        0.5,
    );
    let origin = PackageId(ORIGIN.to_owned());
    let large_type = app
        .world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &origin,
            UnitDef {
                id: "large-walker".into(),
                name: "Large walker".into(),
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 0.8 },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
                armour: None,
                acquire: None,
                provides: Vec::new(),
                consumes: Vec::new(),
                income: None,
                spawn_order: None,
                constructor: None,
                placement: None,
                build_area: None,
                repairer: None,
            },
        )
        .unwrap();
    for id in [2, 3] {
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == id)
            })
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(entity)
            .insert(turts_sim::UnitType(large_type));
    }

    send_unit_move_in_group(&mut app, 0, 1, 3, QueueMode::Replace, 71);
    send_unit_move_in_group(&mut app, 0, 2, 4, QueueMode::Replace, 71);
    send_hold(&mut app, 0, 1, QueueMode::Append);
    app.update();

    let pool_ids: Vec<_> = [1, 2]
        .into_iter()
        .map(|id| {
            let entity = app
                .world_mut()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|unit| unit.0 == id)
                })
                .unwrap();
            entity
                .get::<turts_sim::movement::MoveProgress>()
                .expect("both accepted Unit references use the shared cohort")
                .pool_id
        })
        .collect();
    assert_eq!(pool_ids[0], pool_ids[1]);
    for (id, target_id) in [(1, 3), (2, 4)] {
        let actor = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == id)
            })
            .unwrap();
        assert_eq!(
            actor
                .get::<UnitOrders>()
                .unwrap()
                .current
                .as_ref()
                .map(|order| &order.target),
            Some(&OrderTargetSim::Unit(target_id)),
            "each merged member keeps its own live Unit reference"
        );
    }

    let mut resting_slots = [None, None];
    let mut finished = [false, false];
    for _ in 0..300 {
        app.update();
        for (index, id) in [1, 2].into_iter().enumerate() {
            let entity = app
                .world_mut()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|unit| unit.0 == id)
                })
                .unwrap()
                .id();
            let actor = app.world().entity(entity);
            if actor
                .get::<turts_sim::movement::MoveProgress>()
                .is_some_and(|progress| {
                    progress.activity == turts_sim::movement::MovementActivity::Resting
                        && progress.slot == actor.get::<turts_sim::Position>().unwrap().0
                        && progress.follow_origin.is_some()
                })
            {
                resting_slots[index] = Some(actor.get::<turts_sim::Position>().unwrap().0);
            }
            let orders = actor.get::<UnitOrders>().unwrap();
            let completed = if id == 1 {
                orders
                    .current
                    .as_ref()
                    .is_some_and(|order| order.behaviour == turts_mod_api::Behaviour::Hold)
            } else {
                orders.current.is_none()
            };
            if completed && resting_slots[index].is_some() {
                assert_eq!(
                    actor.get::<turts_sim::Position>().unwrap().0,
                    resting_slots[index].unwrap(),
                    "follow completion consumes the reached slot without reopening movement"
                );
                assert!(actor.get::<MoveTarget>().is_none());
                assert!(actor.get::<turts_sim::movement::MoveProgress>().is_none());
                finished[index] = true;
            }
        }
        if finished == [true, true] {
            break;
        }
    }
    assert!(
        resting_slots[0].is_some(),
        "queued actor reached a legal slot"
    );
    assert!(
        resting_slots[1].is_some(),
        "unqueued actor reached a legal slot"
    );
    assert_eq!(
        finished,
        [true, true],
        "both follow orders complete in Orders"
    );

    let settled: Vec<_> = [1, 2]
        .into_iter()
        .map(|id| {
            let entity = app
                .world_mut()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|unit| unit.0 == id)
                })
                .unwrap();
            entity.get::<turts_sim::Position>().unwrap().0
        })
        .collect();
    for _ in 0..40 {
        app.update();
        for (index, id) in [1, 2].into_iter().enumerate() {
            let entity = app
                .world_mut()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|unit| unit.0 == id)
                })
                .unwrap();
            assert_eq!(
                entity.get::<turts_sim::Position>().unwrap().0,
                settled[index]
            );
            assert!(entity.get::<turts_sim::movement::MoveProgress>().is_none());
        }
    }
}

#[test]
fn unit_self_follow_completion_clears_allocated_progress_without_queue() {
    let mut app = app_with_units_on(&[(8.0, 8.0)], &[0], MapSize { x: 16.0, y: 16.0 }, 0.5);
    send_unit_move(&mut app, 0, 1, 1, QueueMode::Replace);
    let mut completed = false;
    for _ in 0..4 {
        app.update();
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == 1)
            })
            .unwrap()
            .id();
        let actor = app.world().entity(entity);
        let orders = actor.get::<UnitOrders>().unwrap();
        if orders.current.is_none() {
            assert!(actor.get::<MoveTarget>().is_none());
            assert!(actor.get::<turts_sim::movement::MoveProgress>().is_none());
            completed = true;
            break;
        }
    }
    assert!(
        completed,
        "a self-target resolves as an already reached legal goal"
    );
}

#[test]
fn moved_follow_target_reopens_motion_instead_of_consuming_old_fallback() {
    let mut app = app_with_units_on(
        &[(2.0, 8.0), (8.0, 8.0)],
        &[0, 1],
        MapSize { x: 16.0, y: 16.0 },
        0.5,
    );
    send_unit_move(&mut app, 0, 1, 2, QueueMode::Replace);
    send_hold(&mut app, 0, 1, QueueMode::Append);

    let mut reached_old_fallback = false;
    for _ in 0..300 {
        app.update();
        let mover_entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap()
            .id();
        let mover = app.world().entity(mover_entity);
        let plain_resting = mover
            .get::<turts_sim::movement::PlainMoveProgress>()
            .is_some_and(|progress| {
                progress.activity == turts_sim::movement::MovementActivity::Resting
            });
        let cohort_resting = mover
            .get::<turts_sim::movement::MoveProgress>()
            .is_some_and(|progress| {
                progress.activity == turts_sim::movement::MovementActivity::Resting
                    && progress.slot == mover.get::<turts_sim::Position>().unwrap().0
                    && app
                        .world()
                        .resource::<turts_sim::movement::MovementPools>()
                        .0
                        .get(&progress.pool_id)
                        .is_some_and(|pool| pool.point == Vec2::new(8.0, 8.0))
            });
        if plain_resting || cohort_resting {
            reached_old_fallback = true;
            break;
        }
    }
    assert!(
        reached_old_fallback,
        "the old target produces a real Resting fallback before the follow phase"
    );
    let target_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 2)
        })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(target_entity)
        .get_mut::<turts_sim::Position>()
        .unwrap()
        .0 = Vec2::new(8.0, 11.0);
    app.update();
    let mover_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    let mover = app.world().entity(mover_entity);
    let orders = mover.get::<UnitOrders>().unwrap();
    assert_eq!(
        orders
            .current
            .as_ref()
            .map(|order| (&order.behaviour, &order.target)),
        Some((&turts_mod_api::Behaviour::Move, &OrderTargetSim::Unit(2))),
        "moving the live target does not consume the previous fallback or start Hold"
    );
    assert_eq!(
        orders.queue.len(),
        1,
        "the queued Hold remains behind the active follow order"
    );
    let retargeted = mover.get::<MoveTarget>().map(|target| target.0).unwrap();
    assert!(
        retargeted.distance(Vec2::new(8.0, 11.0)) <= 1.2,
        "follow motion adopts an approach point for the new target"
    );
    assert!(
        mover
            .get::<turts_sim::movement::PlainMoveProgress>()
            .is_none_or(|progress| progress.original_point == Vec2::new(8.0, 11.0))
    );
}

#[test]
fn idless_legacy_move_reports_blocked_without_inventing_an_order_id() {
    let mut app = app_with_units_on(&[(0.5, 2.5)], &[], MapSize { x: 7.0, y: 5.0 }, 0.5);
    let post_type = app
        .world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &PackageId(ORIGIN.to_owned()),
            UnitDef {
                id: "wide-post".into(),
                name: "Wide post".into(),
                movement: None,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 2.0 },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
                armour: None,
                acquire: None,
                provides: Vec::new(),
                consumes: Vec::new(),
                income: None,
                spawn_order: None,
                constructor: None,
                placement: None,
                build_area: None,
                repairer: None,
            },
        )
        .unwrap();
    app.world_mut().spawn((
        turts_sim::UnitId(2),
        turts_sim::UnitType(post_type),
        turts_sim::Owner(SlotId(0)),
        turts_sim::Position(Vec2::new(3.5, 2.5)),
    ));
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::MoveTo {
                unit_ids: vec![1],
                target: Vec2::new(6.5, 2.5),
            },
        });
    for _ in 0..200 {
        app.update();
    }
    let (progress, orders) = {
        let mover = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap();
        (
            mover
                .get::<turts_sim::movement::PlainMoveProgress>()
                .cloned()
                .expect("idless movement retains its Blocked activity"),
            mover.get::<UnitOrders>().cloned(),
        )
    };
    assert_eq!(
        progress.activity,
        turts_sim::movement::MovementActivity::Blocked
    );
    assert!(progress.failure_recorded);
    let mover_entity = app
        .world_mut()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    assert!(
        app.world()
            .entity(mover_entity)
            .get::<turts_sim::movement::PhysicalEscape>()
            .is_none(),
        "the idless terminal Blocked result remains after physical clearance"
    );
    assert!(orders.is_none_or(|orders| orders.last_failure.is_none()));
    assert_eq!(
        turts_sim::world_view(app.world_mut())
            .unit_orders
            .iter()
            .find(|row| row.unit_id == 1)
            .map(|row| row.activity),
        Some(turts_sim::economy::views::UnitOrderActivityView::Blocked),
        "idless legacy movement is still visible as Blocked"
    );
    let settled = app
        .world()
        .entity(mover_entity)
        .get::<turts_sim::Position>()
        .unwrap()
        .0;
    for _ in 0..40 {
        app.update();
        assert_eq!(
            app.world()
                .entity(mover_entity)
                .get::<turts_sim::Position>()
                .unwrap()
                .0,
            settled,
            "terminal idless Blocked remains stable without a fabricated order ID"
        );
    }
}

#[test]
fn finite_map_singleton_reaches_point_when_spares_do_not_fit() {
    let point = Vec2::new(1.0, 1.0);
    let mut app = app_with_units_on(&[(0.5, 0.5)], &[], MapSize { x: 1.5, y: 1.5 }, 0.5);
    send_order(&mut app, 0, vec![1], point, 1, QueueMode::Replace);
    app.update();
    assert_eq!(
        app.world().resource::<CommandResults>().0[0].units.accepted,
        vec![1],
        "the valid point order is accepted"
    );
    let pool = app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0
        .values()
        .next()
        .expect("the singleton primary pool is allocated");
    assert_eq!(pool.primary_count, 1);
    assert_eq!(pool.slots.len(), 1, "the exact-N pool has zero spares");

    for _ in 0..200 {
        app.update();
    }

    let unit = snapshot(app.world_mut())
        .into_iter()
        .find(|unit| unit.unit_id == 1)
        .unwrap();
    assert_eq!(unit.pos, point, "the singleton reaches its requested point");
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert!(pending.queues.is_empty());
    assert!(pending.pending_units.values().all(|count| *count == 0));
    assert!(pending.pending_member_counts.is_empty());
    assert_eq!(
        app.world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == 1)
            })
            .unwrap()
            .get::<MoveTarget>(),
        None,
        "the unit has no pending movement target after arrival"
    );
}

#[test]
fn an_exactly_primary_sized_pool_dispatches_without_any_spares() {
    let point = Vec2::new(1.5, 1.5);
    let mut app = app_with_units_on(
        &[(0.5, 0.5), (1.5, 0.5)],
        &[],
        MapSize { x: 2.0, y: 2.0 },
        0.5,
    );
    send_order(&mut app, 0, vec![1, 2], point, 1, QueueMode::Replace);
    app.update();

    let pool = app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0
        .values()
        .next()
        .expect("exactly the required primary pool is legal");
    assert_eq!(pool.primary_count, 2);
    assert_eq!(pool.slots.len(), 2, "this pool has no spares");
    let mut targets: Vec<_> = [1, 2]
        .into_iter()
        .map(|id| {
            app.world()
                .iter_entities()
                .find(|entity| {
                    entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|unit| unit.0 == id)
                })
                .unwrap()
                .get::<MoveTarget>()
                .unwrap()
                .0
        })
        .collect();
    targets.sort_by(|left, right| left.x.total_cmp(&right.x).then(left.y.total_cmp(&right.y)));
    let mut slots = pool.slots.clone();
    slots.sort_by(|left, right| left.x.total_cmp(&right.x).then(left.y.total_cmp(&right.y)));
    assert_eq!(targets, slots, "each member receives one primary slot");
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert!(pending.queues.is_empty());
    assert!(pending.pending_units.values().all(|count| *count == 0));
}

#[test]
fn insufficient_primary_slots_block_the_whole_envelope_without_movement() {
    let positions = [(0.5, 0.5), (1.5, 0.5), (0.5, 1.5)];
    let point = Vec2::new(1.5, 1.5);
    let mut app = app_with_units_on(&positions, &[], MapSize { x: 2.0, y: 2.0 }, 0.5);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::MoveTo {
                unit_ids: vec![1, 2, 3],
                target: point,
            },
        });
    app.update();
    assert_eq!(
        app.world().resource::<CommandResults>().0[0].units.accepted,
        vec![1, 2, 3],
        "the valid point order is accepted before its spread allocation"
    );

    let units = snapshot(app.world_mut());
    for (unit, &(x, y)) in units.iter().zip(&positions) {
        assert_eq!(unit.pos, Vec2::new(x, y), "no member moves on pool failure");
    }
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert!(pending.queues.is_empty());
    assert!(pending.pending_units.values().all(|count| *count == 0));
    assert!(
        pending.pending_member_counts.is_empty(),
        "an impossible envelope is failed instead of left pending"
    );
    for id in [1, 2, 3] {
        let entity = app
            .world_mut()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|unit| unit.0 == id)
            })
            .unwrap();
        assert_eq!(entity.get::<MoveTarget>(), None, "unit {id} has no target");
        assert_eq!(
            entity.get::<turts_sim::movement::MoveProgress>(),
            None,
            "unit {id} has no allocated movement state"
        );
        let plain = entity
            .get::<turts_sim::movement::PlainMoveProgress>()
            .expect("legacy idless failure remains visible as blocked");
        assert_eq!(
            plain.activity,
            turts_sim::movement::MovementActivity::Blocked
        );
        let orders = entity.get::<UnitOrders>().unwrap();
        assert_eq!(orders.current, None, "unit {id} has no active failed move");
        assert_eq!(
            orders.last_failure, None,
            "idless legacy input invents no Order ID"
        );
    }
}

#[test]
fn a_pool_with_fewer_spares_than_requested_still_spreads_its_primary_slots() {
    let positions = [(0.6, 0.6), (2.4, 0.6), (0.6, 2.4), (2.4, 2.4)];
    let point = Vec2::new(1.5, 1.5);
    let mut app = app_with_units_on(&positions, &[], MapSize { x: 3.0, y: 3.0 }, 0.5);
    send_order(&mut app, 0, vec![1, 2, 3, 4], point, 1, QueueMode::Replace);
    app.update();

    let pool = app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0
        .values()
        .next()
        .expect("the primary pool is allocated");
    assert_eq!(pool.primary_count, 4);
    assert!(
        (4..10).contains(&pool.slots.len()),
        "the complete primary pool is retained with only a partial spare set: {:?}",
        pool.slots
    );
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert!(pending.queues.is_empty());
    assert!(pending.pending_units.values().all(|count| *count == 0));
}

#[test]
fn two_slots_of_3000_spread_in_ticks_t_and_t_plus_1() {
    let positions: Vec<_> = (0..6000)
        .map(|i| (0.5 + (i % 100) as f32 * 0.15, 0.5 + (i / 100) as f32 * 0.15))
        .collect();
    let owners: Vec<_> = (0..6000).map(|i| if i < 3000 { 0 } else { 1 }).collect();
    let mut app = app_with_units_on(&positions, &owners, MapSize { x: 100.0, y: 100.0 }, 0.05);
    for _ in 0..7 {
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<turts_sim::orders::MovementDispatchState>(),
        &turts_sim::orders::MovementDispatchState::default(),
        "legacy idle ticks leave first-cohort scheduler state pristine"
    );
    // Tick 8 starts at slot 7, so this initial slot-B command advances the cursor through wrap.
    *app.world_mut()
        .resource_mut::<turts_sim::orders::MovementDispatchState>() =
        turts_sim::orders::MovementDispatchState {
            tick: Some(0),
            start_slot: 7,
            used: 0,
            cursor_advanced: false,
        };
    let previous = Vec2::new(20.0, 20.0);
    let target = Vec2::new(45.0, 45.0);
    send_order(
        &mut app,
        1,
        (3001..=6000).collect(),
        previous,
        19,
        QueueMode::Replace,
    );
    app.update();
    assert_eq!(
        count_current_target(&mut app, 3001..=6000, previous),
        3000,
        "slot B has its previous accepted order before waiting"
    );
    send_order(
        &mut app,
        0,
        (1..=3000).collect(),
        target,
        20,
        QueueMode::Replace,
    );
    send_order(
        &mut app,
        1,
        (3001..=6000).collect(),
        target,
        21,
        QueueMode::Replace,
    );
    app.update();
    assert_eq!(
        count_current_target(&mut app, 1..=3000, target),
        3000,
        "the first slot head envelope spreads in full above the shared cap"
    );
    assert_eq!(
        count_current_target(&mut app, 3001..=6000, previous),
        3000,
        "the waiting slot preserves its prior order for the whole pending envelope"
    );
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert_eq!(pending.pending_units.get(&1), Some(&3000));
    assert_eq!(
        pending
            .queues
            .get(&1)
            .unwrap()
            .front()
            .unwrap()
            .unit_ids
            .len(),
        3000,
        "the second envelope remains whole while waiting"
    );
    app.update();
    assert_eq!(
        count_current_target(&mut app, 3001..=6000, target),
        3000,
        "round robin starts with the next slot on tick t+1"
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_units
            .get(&1),
        Some(&0)
    );
}

#[test]
fn same_tick_group_parts_merge_before_the_2048_cap() {
    let positions: Vec<_> = (0..3000)
        .map(|i| (0.5 + (i % 100) as f32 * 0.15, 0.5 + (i / 100) as f32 * 0.15))
        .collect();
    let mut app = app_with_units_on(&positions, &[], MapSize { x: 64.0, y: 64.0 }, 0.05);
    let target = Vec2::new(32.0, 32.0);
    send_order(
        &mut app,
        0,
        (1..=1500).collect(),
        target,
        77,
        QueueMode::Replace,
    );
    send_order(
        &mut app,
        0,
        (1501..=3000).collect(),
        target,
        77,
        QueueMode::Replace,
    );
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&MoveTarget>()
            .iter(app.world())
            .count(),
        3000,
        "both same-key parts form one 3000-unit head envelope above the cap"
    );
    let pools = &app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0;
    assert_eq!(
        pools.len(),
        1,
        "same slot/group/tick parts share one merged pool"
    );
    assert_eq!(pools.values().next().unwrap().primary_count, 3000);
}

#[test]
fn per_slot_pending_limit_refuses_units_above_16384() {
    let count = 20_480;
    let positions: Vec<_> = (0..count)
        .map(|i| (0.1 + (i % 200) as f32 * 0.02, 0.1 + (i / 200) as f32 * 0.02))
        .collect();
    let mut app = app_with_units_on(&positions, &[], MapSize { x: 128.0, y: 128.0 }, 0.005);
    for batch in 0..5 {
        let start = batch * 4096 + 1;
        send_order(
            &mut app,
            0,
            (start..start + 4096).collect(),
            Vec2::new(64.0, 64.0),
            batch + 1,
            QueueMode::Replace,
        );
    }
    app.update();
    let results = &app.world().resource::<CommandResults>().0;
    assert_eq!(results.len(), 5);
    assert_eq!(
        results[4].units.refused.len(),
        4096,
        "the fifth envelope exceeds the 16384-unit per-slot pending limit atomically"
    );
    assert_eq!(
        results[4].units.refused.len(),
        4096,
        "the pending queue must reject the entire excess envelope"
    );
    assert!(
        results[4]
            .units
            .refused
            .iter()
            .all(|(_, reason)| *reason == RejectReason::RateLimited)
    );
}

#[test]
fn stop_and_replacement_remove_stale_pending_move_membership() {
    let count = 3002;
    let positions: Vec<_> = (0..count)
        .map(|i| (0.5 + (i % 50) as f32 * 0.15, 0.5 + (i / 50) as f32 * 0.15))
        .collect();
    let mut app = app_with_units_on(&positions, &[], MapSize { x: 64.0, y: 64.0 }, 0.05);
    let previous = Vec2::new(20.0, 20.0);
    send_order(
        &mut app,
        0,
        vec![3001, 3002],
        previous,
        40,
        QueueMode::Replace,
    );
    app.update();
    let old_target = Vec2::new(40.0, 30.0);
    send_order(
        &mut app,
        0,
        (1..=3000).collect(),
        old_target,
        41,
        QueueMode::Replace,
    );
    send_order(
        &mut app,
        0,
        vec![3001, 3002],
        old_target,
        42,
        QueueMode::Replace,
    );
    app.update();
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert_eq!(
        pending.queues.get(&0).unwrap().front().unwrap().unit_ids,
        vec![3001, 3002]
    );
    send_stop(&mut app, 0, 3001);
    let replacement = Vec2::new(50.0, 40.0);
    send_order(&mut app, 0, vec![3002], replacement, 43, QueueMode::Replace);
    app.update();
    let pending = app
        .world()
        .resource::<turts_sim::orders::PendingMovementOrders>();
    assert!(pending.queues.get(&0).is_none_or(|queue| {
        queue
            .iter()
            .all(|request| !request.unit_ids.contains(&3001))
    }));
    assert!(
        app.world_mut()
            .iter_entities()
            .find(|entity| entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 3001))
            .unwrap()
            .get::<MoveTarget>()
            .is_none()
    );
    assert_eq!(
        current_target(&mut app, 3001),
        None,
        "Stop removes the old order"
    );
    assert_eq!(
        current_target(&mut app, 3002),
        Some(OrderTargetSim::Point(replacement)),
        "replacement wins over the old pending move"
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::orders::PendingMovementOrders>()
            .pending_units
            .get(&0),
        Some(&0)
    );
}

#[test]
fn group_0_never_merges() {
    let mut app = app_with_units(&[(8.0, 8.0), (9.2, 8.0), (8.0, 9.2), (9.2, 9.2)], &[]);
    send_order(
        &mut app,
        0,
        vec![1, 2],
        Vec2::new(32.0, 32.0),
        0,
        QueueMode::Replace,
    );
    send_order(
        &mut app,
        0,
        vec![3, 4],
        Vec2::new(32.0, 32.0),
        0,
        QueueMode::Replace,
    );
    app.update();
    let pool_ids: Vec<_> = app
        .world_mut()
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<turts_sim::UnitId>()?.0;
            (id <= 4)
                .then(|| {
                    entity
                        .get::<turts_sim::movement::MoveProgress>()
                        .map(|progress| (id, progress.pool_id))
                })
                .flatten()
        })
        .collect();
    assert_eq!(pool_ids.len(), 4);
    assert_eq!(
        pool_ids[0].1, pool_ids[1].1,
        "members from the first group-zero envelope share its pool"
    );
    assert_eq!(
        pool_ids[2].1, pool_ids[3].1,
        "members from the second group-zero envelope share its pool"
    );
    assert_ne!(
        pool_ids[0].1, pool_ids[2].1,
        "separate group-zero envelopes own separate pools"
    );
    let pools = &app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0;
    assert_eq!(pools.len(), 2);
    assert!(pools.values().all(|pool| pool.primary_count == 2));
    for _ in 0..500 {
        app.update();
    }
    assert!(rests_without_overlap(&mut app, &[1, 2, 3, 4]));
}

#[test]
fn repeated_group_inputs_produce_bit_identical_positions() {
    let make = || {
        let positions: Vec<_> = (0..40)
            .map(|i| (10.0 + 1.2 * (i % 8) as f32, 10.0 + 1.2 * (i / 8) as f32))
            .collect();
        let mut app = app_with_units(&positions, &[]);
        send_order(
            &mut app,
            0,
            (1..=40).collect(),
            Vec2::new(40.0, 30.0),
            88,
            QueueMode::Replace,
        );
        for _ in 0..400 {
            app.update();
        }
        snapshot(app.world_mut())
            .into_iter()
            .map(|unit| unit.pos)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        make(),
        make(),
        "same command stream produces bit-exact positions"
    );
}
