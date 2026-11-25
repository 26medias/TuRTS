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
                terrain_class: None,
                terrain_cargo: None,
                terrain_foundation: None,
                terrain_tools: Vec::new(),
                vision: None,
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
        terrain: None,
        visibility: None,
        environment: None,
        ground: None,
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
                terrain_class: None,
                terrain_cargo: None,
                terrain_foundation: None,
                terrain_tools: Vec::new(),
                vision: None,
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
    // Rule 2 (2026-11-23): no exact approach slot. The refreshed target is at most the old
    // approach distance from the moved target; the unit stops at contact.
    assert!(
        refreshed_target.distance(moved_follow_target) <= 1.1 + 1.0e-3,
        "the prior follow refresh follows the moved target: {refreshed_target:?}"
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

/// Rule 2 (2026-11-23): a group order targets an area. `count` units of radius 0.5 rest inside
/// the goal area radius sqrt(sum of radius^2 / 0.35), plus one unit diameter.
fn near_bound(count: usize) -> f32 {
    (count as f32 * 0.25 / 0.35).sqrt() + 1.0
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
            .all(|unit| (unit.pos.x - target.x).hypot(unit.pos.y - target.y) <= near_bound(40))
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
            // Rule 2 (2026-11-23): two orders to one point make one crowd of 80 units. A unit
            // that cannot get into the goal area stops at the crowd, so the bound has one more
            // unit diameter than `near_bound`.
            .all(
                |unit| (unit.pos.x - target.x).hypot(unit.pos.y - target.y) <= near_bound(80) + 1.0
            )
    );
}

/// The cross-owner scene of `second_group_same_point` with opaque inputs changed: the owner
/// slots swapped (connection order), or the unit ids of each group reversed. The result is the
/// sorted rest positions of each group, as bit patterns.
fn second_group_rest_points(swap_owners: bool, reverse_ids: bool) -> [Vec<(u32, u32)>; 2] {
    let order = |mut group: Vec<(f32, f32)>| {
        if reverse_ids {
            group.reverse();
        }
        group
    };
    let first = order(
        (0..40)
            .map(|k| (10.0 + 1.2 * (k % 8) as f32, 10.0 + 1.2 * (k / 8) as f32))
            .collect(),
    );
    let parked: Vec<_> = (0..40)
        .map(|k| (5.0 + 1.2 * (k % 8) as f32, 50.0 + 1.2 * (k / 8) as f32))
        .collect();
    let second = order(
        (0..40)
            .map(|k| (10.0 + 1.2 * (k % 8) as f32, 30.0 + 1.2 * (k / 8) as f32))
            .collect(),
    );
    let (first_owner, second_owner) = if swap_owners { (1, 0) } else { (0, 1) };
    let mut positions = first.clone();
    positions.extend(parked);
    let owners: Vec<_> = (0..80)
        .map(|i| if i < 40 { first_owner } else { second_owner })
        .collect();
    let mut app = app_with_units(&positions, &owners);
    let target = Vec2::new(40.0, 30.0);
    send_order(
        &mut app,
        first_owner,
        (1..=40).collect(),
        target,
        7,
        QueueMode::Replace,
    );
    for _ in 0..horizon_ticks(&first, target) {
        app.update();
    }
    set_positions(&mut app, 41, &second);
    send_order(
        &mut app,
        second_owner,
        (41..=80).collect(),
        target,
        7,
        QueueMode::Replace,
    );
    for _ in 0..horizon_ticks(&second, target) {
        app.update();
    }
    assert_settled(&mut app, &(1..=80).collect::<Vec<_>>());
    let units = snapshot(app.world_mut());
    let group = |ids: std::ops::RangeInclusive<u32>| {
        let mut points: Vec<_> = units
            .iter()
            .filter(|unit| ids.contains(&unit.unit_id))
            .map(|unit| (unit.pos.x.to_bits(), unit.pos.y.to_bits()))
            .collect();
        points.sort_unstable();
        points
    };
    [group(1..=40), group(41..=80)]
}

/// The unit id and the owner slot are opaque: neither may decide where a body rests when a
/// group moves to a point that a group of another owner holds.
#[test]
fn second_group_same_point_ignores_unit_ids_and_owner_slots() {
    let baseline = second_group_rest_points(false, false);
    assert!(
        baseline == second_group_rest_points(true, false),
        "swapped owner slots changed the rest points"
    );
    assert!(
        baseline == second_group_rest_points(false, true),
        "reversed unit ids changed the rest points"
    );
}

/// A 36-unit group move on open ground (T-7 proof; FM rows measured headless, 20 Hz):
/// FM1 every unit moves within 20 ticks; FM2 all units rest within 1200 ticks; FM3 the
/// overlap of two moving units is at most 25 % of the radius, and none at rest. With
/// `TURTS_TRACE_OUT=<file>` the test writes each tick as CSV `tick,unit,x,y`.
#[test]
fn thirty_six_units_cross_open_ground() {
    let positions: Vec<_> = (0..36)
        .map(|k| (8.0 + 1.2 * (k % 6) as f32, 26.0 + 1.2 * (k / 6) as f32))
        .collect();
    let owners = vec![0; positions.len()];
    let mut app = app_with_units(&positions, &owners);
    let ids: Vec<u32> = (1..=36).collect();
    let target = Vec2::new(50.0, 30.0);
    send_order(&mut app, 0, ids.clone(), target, 7, QueueMode::Replace);
    let start: Vec<Vec2> = positions.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
    let mut first_motion = vec![None; ids.len()];
    let mut trace = String::from("tick,unit,x,y\n");
    let mut previous = start.clone();
    let mut worst_moving = 0.0_f32;
    let mut rested_at = None;
    for tick in 1..=1200 {
        app.update();
        let now: Vec<Vec2> = snapshot(app.world_mut())
            .iter()
            .map(|unit| unit.pos)
            .collect();
        for (index, at) in now.iter().enumerate() {
            if first_motion[index].is_none() && at.distance(start[index]) > 0.1 {
                first_motion[index] = Some(tick);
            }
            trace.push_str(&format!("{tick},{},{},{}\n", ids[index], at.x, at.y));
            for other in index + 1..now.len() {
                let moving = *at != previous[index] || now[other] != previous[other];
                let overlap = 1.0 - at.distance(now[other]);
                if moving {
                    worst_moving = worst_moving.max(overlap);
                }
            }
        }
        let resting = now == previous
            && ids.iter().all(|id| {
                app.world_mut()
                    .iter_entities()
                    .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|u| u.0 == *id))
                    .is_some_and(|e| e.get::<MoveTarget>().is_none())
            });
        if resting && rested_at.is_none() {
            rested_at = Some(tick);
        }
        previous = now;
        if rested_at.is_some_and(|at| tick >= at + 40) {
            break;
        }
    }
    if let Some(path) = std::env::var_os("TURTS_TRACE_OUT") {
        std::fs::write(path, &trace).unwrap();
    }
    let last_first_motion = first_motion
        .iter()
        .map(|t| t.unwrap_or(u32::MAX))
        .max()
        .unwrap();
    println!(
        "open ground, 36 units: last first motion {last_first_motion} ticks, rest at {rested_at:?} ticks, worst moving overlap {worst_moving} m (radius 0.5)"
    );
    assert!(
        last_first_motion <= 20,
        "FM1: a unit started late: {first_motion:?}"
    );
    assert!(
        rested_at.is_some(),
        "FM2: the group did not rest within 1200 ticks"
    );
    assert!(
        worst_moving <= 0.25 * 0.5,
        "FM3: moving overlap {worst_moving} m"
    );
    assert!(
        rests_without_overlap(&mut app, &ids),
        "FM3: overlap at rest"
    );
}

/// V6 (Clara, 2026-11-24): an idle own unit that a passing group pushes away goes back to
/// its place when no own mover is near.
#[test]
fn pushed_idle_unit_goes_back_to_its_place() {
    let home = Vec2::new(30.0, 32.0);
    let mut positions = vec![(home.x, home.y)];
    positions.extend((0..28).map(|k| (8.0 + 1.2 * (k % 4) as f32, 28.4 + 1.2 * (k / 4) as f32)));
    let owners = vec![0; positions.len()];
    let mut app = app_with_units(&positions, &owners);
    let target = Vec2::new(54.0, 32.0);
    send_order(
        &mut app,
        0,
        (2..=29).collect(),
        target,
        7,
        QueueMode::Replace,
    );
    let idle = |app: &mut bevy_app::App| {
        snapshot(app.world_mut())
            .into_iter()
            .find(|unit| unit.unit_id == 1)
            .unwrap()
            .pos
    };
    let mut farthest = 0.0_f32;
    let mut trace = Vec::new();
    for _ in 0..horizon_ticks(&positions[1..], target) + 200 {
        app.update();
        let at = idle(&mut app);
        farthest = farthest.max(at.distance(home));
        trace.push(at);
    }
    // The case proves nothing if the group never pushed the idle unit away.
    assert!(
        farthest > 0.5,
        "the group did not push the idle unit: {farthest}"
    );
    let end = trace[trace.len() - 1];
    assert!(
        end.distance(home) < 1.0e-3,
        "the pushed idle unit did not go back to its place: {end:?} (farthest {farthest})"
    );
    assert_eq!(
        trace[trace.len() - 40..]
            .iter()
            .filter(|at| **at != end)
            .count(),
        0
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
            .all(|unit| (unit.pos.x - 50.0).hypot(unit.pos.y - 36.0) <= near_bound(40))
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
    // Flock (2026-11-23, rule 4: the map edge stays solid): a committed pose keeps a clearance
    // of 1/512 from the map edge, so the unit rests within 0.01 of the clamped point.
    assert!((unit.pos.x - 63.5).abs() < 0.01 && (unit.pos.y - 63.5).abs() < 0.01);
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
    // Rule 4: the map edge stays solid. The step check keeps a clearance from the edge.
    assert!(
        (unit.pos.x - point.x).abs() <= 0.01 && (unit.pos.y - point.y).abs() <= 0.01,
        "the singleton reaches its requested point: {:?}",
        unit.pos
    );
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

/// Paired worlds under fog (from the independent privacy test of 2026-11-18): slot 0 orders two
/// units to a point. A unit of slot 1 that slot 0 may not see stands on the point in one world
/// and far away in the other. The slots and the first step of slot 0 must be equal.
#[test]
fn hidden_unit_of_another_slot_does_not_change_group_dispatch() {
    use turts_mod_api::{ObservationPhase, VisibilityDef, VisibilityGrid, VisionSharing};
    type Rows = Vec<(u32, Vec2)>;
    fn run(enemy: (f32, f32)) -> (Rows, Rows) {
        let mut app = app_with_units(&[(5.0, 5.0), (5.0, 7.0), enemy], &[0, 0, 1]);
        let definition = VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        };
        let sight = turts_sim::visibility::prepare(
            0,
            turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
            &definition,
            VisibilityGrid {
                size: WorldPos { x: 64.0, y: 64.0 },
                cell_size: 2.0,
                columns: 32,
                rows: 32,
            },
            &[0, 1],
        )
        .unwrap();
        app.insert_resource(sight);
        turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            3,
            ObservationPhase::Completed
        ));
        send_order(
            &mut app,
            0,
            vec![1, 2],
            Vec2::new(30.0, 30.0),
            42,
            QueueMode::Replace,
        );
        app.update();
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            3,
            ObservationPhase::Completed
        ));
        let mut targets: Vec<_> = app
            .world_mut()
            .iter_entities()
            .filter_map(|e| {
                let id = e.get::<turts_sim::UnitId>()?.0;
                (id <= 2).then(|| (id, e.get::<MoveTarget>().unwrap().0))
            })
            .collect();
        targets.sort_by_key(|(id, _)| *id);
        let positions: Vec<_> = snapshot(app.world_mut())
            .into_iter()
            .filter(|u| u.unit_id <= 2)
            .map(|u| (u.unit_id, u.pos))
            .collect();
        (targets, positions)
    }
    let near_target = run((30.0, 30.0));
    let remote = run((55.0, 55.0));
    assert_eq!(
        near_target.0, remote.0,
        "a hidden unit of another slot changed the group targets before contact"
    );
    assert_eq!(
        near_target.1, remote.1,
        "a hidden unit of another slot changed the owner movement before contact"
    );
}

/// Paired worlds under fog (from the independent privacy test of 2026-11-18): slot 0 orders two
/// units to a point. A unit of slot 1 that slot 0 may not see stands on the point in one world
/// and far away in the other. The slots and the first step of slot 0 must be equal.
#[test]
fn hidden_unit_of_another_slot_does_not_change_singleton_dispatch() {
    use turts_mod_api::{ObservationPhase, VisibilityDef, VisibilityGrid, VisionSharing};
    type Rows = Vec<(u32, Vec2)>;
    fn run(enemy: (f32, f32)) -> (Rows, Rows) {
        let mut app = app_with_units(&[(5.0, 5.0), enemy], &[0, 1]);
        let definition = VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        };
        let sight = turts_sim::visibility::prepare(
            0,
            turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
            &definition,
            VisibilityGrid {
                size: WorldPos { x: 64.0, y: 64.0 },
                cell_size: 2.0,
                columns: 32,
                rows: 32,
            },
            &[0, 1],
        )
        .unwrap();
        app.insert_resource(sight);
        turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            2,
            ObservationPhase::Completed
        ));
        let mover = app
            .world()
            .iter_entities()
            .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(mover)
            .insert(MoveTarget(Vec2::new(30.0, 30.0)));
        for _ in 0..30 {
            app.update();
        }
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            2,
            ObservationPhase::Completed
        ));
        let mut targets: Vec<_> = app
            .world_mut()
            .iter_entities()
            .filter_map(|e| {
                let id = e.get::<turts_sim::UnitId>()?.0;
                (id == 1).then(|| (id, e.get::<MoveTarget>().unwrap().0))
            })
            .collect();
        targets.sort_by_key(|(id, _)| *id);
        let positions: Vec<_> = snapshot(app.world_mut())
            .into_iter()
            .filter(|u| u.unit_id == 1)
            .map(|u| (u.unit_id, u.pos))
            .collect();
        (targets, positions)
    }
    let near_target = run((30.0, 30.0));
    let remote = run((55.0, 55.0));
    assert_eq!(
        near_target.0, remote.0,
        "a hidden unit of another slot changed the singleton target before contact"
    );
    assert_eq!(
        near_target.1, remote.1,
        "a hidden unit of another slot changed the owner movement before contact"
    );
}

#[test]
fn hidden_unit_of_another_slot_does_not_change_group_approach() {
    use turts_mod_api::{ObservationPhase, VisibilityDef, VisibilityGrid, VisionSharing};
    type Rows = Vec<(u32, Vec2)>;
    fn run(enemy: (f32, f32)) -> (Rows, Rows) {
        let mut app = app_with_units(&[(5.0, 5.0), (5.0, 7.0), enemy], &[0, 0, 1]);
        let definition = VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        };
        let sight = turts_sim::visibility::prepare(
            0,
            turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
            &definition,
            VisibilityGrid {
                size: WorldPos { x: 64.0, y: 64.0 },
                cell_size: 2.0,
                columns: 32,
                rows: 32,
            },
            &[0, 1],
        )
        .unwrap();
        app.insert_resource(sight);
        turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            3,
            ObservationPhase::Completed
        ));
        send_order(
            &mut app,
            0,
            vec![1, 2],
            Vec2::new(30.0, 30.0),
            42,
            QueueMode::Replace,
        );
        for _ in 0..100 {
            app.update();
        }
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            3,
            ObservationPhase::Completed
        ));
        let mut targets: Vec<_> = app
            .world_mut()
            .iter_entities()
            .filter_map(|e| {
                let id = e.get::<turts_sim::UnitId>()?.0;
                (id <= 2).then(|| (id, e.get::<MoveTarget>().unwrap().0))
            })
            .collect();
        targets.sort_by_key(|(id, _)| *id);
        let positions: Vec<_> = snapshot(app.world_mut())
            .into_iter()
            .filter(|u| u.unit_id <= 2)
            .map(|u| (u.unit_id, u.pos))
            .collect();
        (targets, positions)
    }
    let near_target = run((30.0, 30.0));
    let remote = run((55.0, 55.0));
    assert_eq!(
        near_target.0, remote.0,
        "a hidden unit of another slot changed the group targets before contact"
    );
    assert_eq!(
        near_target.1, remote.1,
        "a hidden unit of another slot changed the owner movement before contact"
    );
}

#[test]
#[ignore = "representative manual timing"]
fn singleton_fog_scale_probe() {
    use turts_mod_api::{VisibilityDef, VisibilityGrid, VisionSharing};
    let mut positions = Vec::new();
    let mut owners = Vec::new();
    for i in 0..500 {
        let x = 2.0 + 2.3 * (i % 25) as f32;
        let y = 2.0 + 2.3 * (i / 25) as f32;
        positions.extend([(x, y), (x + 1.2, y)]);
        owners.extend([0, 1]);
    }
    let mut app = app_with_units(&positions, &owners);
    if std::env::var("FOG_PROBE").is_ok() {
        let sight = turts_sim::visibility::prepare(
            0,
            turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
            &VisibilityDef {
                cell_size: 2.0,
                sharing: VisionSharing::OwnSlot,
                memory_limit: None,
            },
            VisibilityGrid {
                size: WorldPos { x: 64.0, y: 64.0 },
                cell_size: 2.0,
                columns: 32,
                rows: 32,
            },
            &[0, 1],
        )
        .unwrap();
        app.insert_resource(sight);
        turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
    }
    let movers: Vec<_> = app
        .world()
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<turts_sim::UnitId>()?.0;
            (id % 2 == 1).then_some((entity.id(), (id / 2) as usize))
        })
        .collect();
    assert_eq!(movers.len(), 500);
    for (entity, row) in movers {
        let (x, y) = positions[2 * row + 1];
        app.world_mut()
            .entity_mut(entity)
            .insert(MoveTarget(Vec2::new(x, y)));
    }
    let mut samples = Vec::new();
    for _ in 0..20 {
        let now = std::time::Instant::now();
        app.update();
        samples.push(now.elapsed().as_secs_f64() * 1000.0);
    }
    eprintln!(
        "fog={} units=1000 movers=500 ticks=20 mean_ms={:.3} max_ms={:.3}",
        std::env::var("FOG_PROBE").is_ok(),
        samples.iter().sum::<f64>() / samples.len() as f64,
        samples.into_iter().fold(0.0, f64::max)
    );
}

#[test]
fn singleton_hidden_target_stops_without_overlap_after_contact() {
    use turts_mod_api::{VisibilityDef, VisibilityGrid, VisionSharing};
    let mut app = app_with_units(&[(5.0, 5.0), (30.0, 30.0)], &[0, 1]);
    let sight = turts_sim::visibility::prepare(
        0,
        turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
        &VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        },
        VisibilityGrid {
            size: WorldPos { x: 64.0, y: 64.0 },
            cell_size: 2.0,
            columns: 32,
            rows: 32,
        },
        &[0, 1],
    )
    .unwrap();
    app.insert_resource(sight);
    turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
    turts_sim::initialize_completed_world(app.world_mut());
    let mover = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == 1)
        })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(mover)
        .insert(MoveTarget(Vec2::new(30.0, 30.0)));
    for _ in 0..400 {
        app.update();
        let rows = snapshot(app.world_mut());
        assert!(
            rows[0].pos.distance(rows[1].pos) >= 1.0 - 1e-5,
            "movement overlapped the hidden body"
        );
    }
    let point = app
        .world()
        .entity(mover)
        .get::<turts_sim::Position>()
        .unwrap()
        .0;
    assert!(
        point.distance(Vec2::new(30.0, 30.0)) <= 1.1,
        "unit did not reach the hidden body: {point:?}"
    );
    assert!(
        app.world().entity(mover).get::<MoveTarget>().is_none(),
        "unit failed to stop"
    );
}

#[test]
fn hidden_remote_body_does_not_change_blocked_recovery() {
    use turts_mod_api::{ObservationPhase, VisibilityDef, VisibilityGrid, VisionSharing};
    use turts_sim::movement::{MoveProgress, MovementActivity, MovementPool, MovementPools};
    fn run(remote: Vec2) -> (Option<Vec2>, Vec2) {
        let mut app = app_with_units(
            &[(10.0, 10.0), (12.0, 10.0), (remote.x, remote.y)],
            &[0, 1, 1],
        );
        let ids: Vec<_> = app
            .world()
            .iter_entities()
            .filter_map(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .map(|id| (id.0, entity.id()))
            })
            .collect();
        let entity = |id| ids.iter().find(|row| row.0 == id).unwrap().1;
        app.world_mut()
            .entity_mut(entity(2))
            .insert(turts_sim::Position(Vec2::new(10.0, 10.0)));
        let goal = Vec2::new(50.0, 50.0);
        app.world_mut().resource_mut::<MovementPools>().0.insert(
            7,
            MovementPool {
                pool_id: 7,
                point: goal,
                spacing: 1.1,
                primary_count: 1,
                slots: vec![goal],
            },
        );
        app.world_mut().entity_mut(entity(1)).insert((
            MoveTarget(goal),
            MoveProgress {
                pool_id: 7,
                slot: goal,
                activity: MovementActivity::Blocked,
                pass_mode: true,
                stalling_on_group: false,
                stalled_ticks: 60,
                best_distance: 1.0,
                claim_count: 3,
                follow_origin: None,
            },
        ));
        let sight = turts_sim::visibility::prepare(
            0,
            turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
            &VisibilityDef {
                cell_size: 2.0,
                sharing: VisionSharing::OwnSlot,
                memory_limit: None,
            },
            VisibilityGrid {
                size: WorldPos { x: 64.0, y: 64.0 },
                cell_size: 2.0,
                columns: 32,
                rows: 32,
            },
            &[0, 1],
        )
        .unwrap();
        app.insert_resource(sight);
        turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            3,
            ObservationPhase::Completed
        ));
        app.update();
        let entity = app.world().entity(entity(1));
        (
            entity.get::<MoveTarget>().map(|target| target.0),
            entity.get::<turts_sim::Position>().unwrap().0,
        )
    }
    assert_eq!(
        run(Vec2::new(55.0, 55.0)),
        run(Vec2::new(50.0, 50.0)),
        "an unseen remote unit changed blocked recovery"
    );
}

#[test]
fn hidden_remote_target_does_not_change_blocked_recovery() {
    use turts_mod_api::{ObservationPhase, VisibilityDef, VisibilityGrid, VisionSharing};
    use turts_sim::movement::{MoveProgress, MovementActivity, MovementPool, MovementPools};
    fn run(enemy_moving: bool) -> (Option<Vec2>, Vec2) {
        let remote = Vec2::new(55.0, 55.0);
        let mut app = app_with_units(
            &[(10.0, 10.0), (12.0, 10.0), (remote.x, remote.y)],
            &[0, 1, 1],
        );
        let ids: Vec<_> = app
            .world()
            .iter_entities()
            .filter_map(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .map(|id| (id.0, entity.id()))
            })
            .collect();
        let entity = |id| ids.iter().find(|row| row.0 == id).unwrap().1;
        app.world_mut()
            .entity_mut(entity(2))
            .insert(turts_sim::Position(Vec2::new(10.0, 10.0)));
        let goal = Vec2::new(50.0, 50.0);
        if enemy_moving {
            app.world_mut()
                .entity_mut(entity(3))
                .insert(MoveTarget(goal));
        }
        app.world_mut().resource_mut::<MovementPools>().0.insert(
            7,
            MovementPool {
                pool_id: 7,
                point: goal,
                spacing: 1.1,
                primary_count: 1,
                slots: vec![goal],
            },
        );
        app.world_mut().entity_mut(entity(1)).insert((
            MoveTarget(goal),
            MoveProgress {
                pool_id: 7,
                slot: goal,
                activity: MovementActivity::Blocked,
                pass_mode: true,
                stalling_on_group: false,
                stalled_ticks: 60,
                best_distance: 1.0,
                claim_count: 3,
                follow_origin: None,
            },
        ));
        let sight = turts_sim::visibility::prepare(
            0,
            turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
            &VisibilityDef {
                cell_size: 2.0,
                sharing: VisionSharing::OwnSlot,
                memory_limit: None,
            },
            VisibilityGrid {
                size: WorldPos { x: 64.0, y: 64.0 },
                cell_size: 2.0,
                columns: 32,
                rows: 32,
            },
            &[0, 1],
        )
        .unwrap();
        app.insert_resource(sight);
        turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            3,
            ObservationPhase::Completed
        ));
        app.update();
        let entity = app.world().entity(entity(1));
        (
            entity.get::<MoveTarget>().map(|target| target.0),
            entity.get::<turts_sim::Position>().unwrap().0,
        )
    }
    assert_eq!(
        run(false),
        run(true),
        "an unseen remote unit changed blocked recovery"
    );
}

#[test]
fn hidden_body_does_not_change_plain_blocked_recovery() {
    use turts_mod_api::{ObservationPhase, VisibilityDef, VisibilityGrid, VisionSharing};
    use turts_sim::movement::{MovementActivity, PlainMoveProgress};
    fn run(remote: Vec2) -> (Option<Vec2>, Vec2) {
        let mut app = app_with_units(
            &[(10.0, 10.0), (12.0, 10.0), (remote.x, remote.y)],
            &[0, 1, 1],
        );
        let ids: Vec<_> = app
            .world()
            .iter_entities()
            .filter_map(|e| e.get::<turts_sim::UnitId>().map(|id| (id.0, e.id())))
            .collect();
        let entity = |id| ids.iter().find(|row| row.0 == id).unwrap().1;
        app.world_mut()
            .entity_mut(entity(2))
            .insert(turts_sim::Position(Vec2::new(10.0, 10.0)));
        app.world_mut()
            .entity_mut(entity(1))
            .insert(PlainMoveProgress {
                arrival_yield: false,
                original_point: Vec2::splat(50.0),
                slot: Vec2::splat(50.0),
                pool_slots: vec![],
                gather_goal: None,
                activity: MovementActivity::Blocked,
                stalled_ticks: 60,
                best_distance: 1.0,
                claim_count: 3,
                failure_recorded: false,
            });
        let sight = turts_sim::visibility::prepare(
            0,
            turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
            &VisibilityDef {
                cell_size: 2.0,
                sharing: VisionSharing::OwnSlot,
                memory_limit: None,
            },
            VisibilityGrid {
                size: WorldPos { x: 64.0, y: 64.0 },
                cell_size: 2.0,
                columns: 32,
                rows: 32,
            },
            &[0, 1],
        )
        .unwrap();
        app.insert_resource(sight);
        turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            3,
            ObservationPhase::Completed
        ));
        app.update();
        let mover = app.world().entity(entity(1));
        (
            mover.get::<MoveTarget>().map(|t| t.0),
            mover.get::<turts_sim::Position>().unwrap().0,
        )
    }
    assert_eq!(
        run(Vec2::new(55.0, 55.0)),
        run(Vec2::new(8.9, 8.9)),
        "hidden body changed singleton blocked recovery"
    );
}

fn arrival_yield_fog_fixture(enemy: Vec2, visible: bool, actor: Vec2) -> (Option<Vec2>, Vec2) {
    use turts_mod_api::{ObservationPhase, VisibilityDef, VisibilityGrid, VisionSharing};
    use turts_sim::movement::{MovementActivity, PlainMoveProgress};
    let mut app = app_with_units(&[(actor.x, actor.y), (enemy.x, enemy.y)], &[0, 1]);
    let mover = app
        .world()
        .iter_entities()
        .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|id| id.0 == 1))
        .unwrap()
        .id();
    app.world_mut().entity_mut(mover).insert((
        MoveTarget(Vec2::new(47.0, 50.0)),
        PlainMoveProgress {
            arrival_yield: true,
            original_point: Vec2::splat(50.0),
            slot: Vec2::new(47.0, 50.0),
            pool_slots: vec![],
            gather_goal: None,
            activity: MovementActivity::Moving,
            stalled_ticks: 0,
            best_distance: 1.0,
            claim_count: 1,
            failure_recorded: false,
        },
    ));
    let sight = turts_sim::visibility::prepare(
        0,
        turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
        &VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        },
        VisibilityGrid {
            size: WorldPos { x: 64.0, y: 64.0 },
            cell_size: 2.0,
            columns: 32,
            rows: 32,
        },
        &[0, 1],
    )
    .unwrap();
    if visible {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
        let mut catalog = turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&root).unwrap(),
        )
        .unwrap()
        .content()
        .unwrap();
        let mut definition = catalog.units.values().next().unwrap().clone();
        definition.id = turts_mod_api::ContentId::parse("fixture_pkg:walker").unwrap();
        definition.vision = Some(turts_mod_api::VisionDef { radius: 64.0 });
        definition.health = None;
        catalog.units.insert(definition.id.clone(), definition);
        app.insert_resource(turts_sim::bootstrap::InstalledCatalog(catalog));
    }
    app.insert_resource(sight);
    turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
    turts_sim::initialize_completed_world(app.world_mut());
    assert_eq!(
        turts_sim::visibility::unit_permitted(app.world(), 0, 2, ObservationPhase::Completed),
        visible
    );
    app.update();
    let mover = app.world().entity(mover);
    (
        mover.get::<MoveTarget>().map(|target| target.0),
        mover.get::<turts_sim::Position>().unwrap().0,
    )
}

#[test]
fn arrival_yield_ignores_hidden_remote_holder() {
    assert_eq!(
        arrival_yield_fog_fixture(Vec2::splat(55.0), false, Vec2::new(48.0, 50.0)),
        arrival_yield_fog_fixture(Vec2::splat(50.0), false, Vec2::new(48.0, 50.0))
    );
}

// Hidden-order arrival privacy (handoff 2026-11-18). A mover may use its own goal and the
// physical state of a body that it can know. It never uses the goal of another owner.

/// A fogged two-owner world. The fixture units have no sight, so a unit of the other owner
/// is known only by physical contact.
fn fog_app(positions: &[(f32, f32)], owners: &[u8]) -> bevy_app::App {
    use turts_mod_api::{VisibilityDef, VisibilityGrid, VisionSharing};
    let mut app = app_with_units(positions, owners);
    let sight = turts_sim::visibility::prepare(
        0,
        turts_mod_api::ContentId::parse("fixture_pkg:field").unwrap(),
        &VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        },
        VisibilityGrid {
            size: WorldPos { x: 64.0, y: 64.0 },
            cell_size: 2.0,
            columns: 32,
            rows: 32,
        },
        &[0, 1],
    )
    .unwrap();
    app.insert_resource(sight);
    turts_sim::visibility::initialize_context(app.world_mut(), 0, &[0, 1]).unwrap();
    turts_sim::initialize_completed_world(app.world_mut());
    app
}

/// Position, target, and the temporary-stop flag with its point.
type OwnState = (Vec2, Option<Vec2>, Option<(bool, Vec2)>);

/// The complete movement state of one unit that its owner can see.
fn own_state(app: &bevy_app::App, unit: u32) -> OwnState {
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == unit)
        })
        .unwrap();
    (
        entity.get::<turts_sim::Position>().unwrap().0,
        entity.get::<MoveTarget>().map(|target| target.0),
        entity
            .get::<turts_sim::movement::PlainMoveProgress>()
            .map(|progress| (progress.arrival_yield, progress.slot)),
    )
}

fn unit_position(app: &bevy_app::App, unit: u32) -> Vec2 {
    own_state(app, unit).0
}

fn all_orders_complete(app: &bevy_app::App) -> bool {
    app.world()
        .iter_entities()
        .filter(|entity| entity.contains::<turts_sim::UnitId>())
        .all(|entity| entity.get::<MoveTarget>().is_none())
}

/// Two units of two owners get one ordinary point order each, to `GOAL`. Unit 1 belongs to
/// slot 0 when `reverse` is false. Returns the final positions of `first` and `second`.
fn fog_contest(first: (f32, f32), second: (f32, f32), reverse: bool) -> (Vec2, Vec2, bool) {
    let goal = Vec2::splat(50.0);
    let (positions, owners, first_id, second_id) = if reverse {
        ([second, first], [1, 0], 2, 1)
    } else {
        ([first, second], [0, 1], 1, 2)
    };
    let mut app = fog_app(&positions, &owners);
    send_order(&mut app, 0, vec![first_id], goal, 1, QueueMode::Replace);
    send_order(&mut app, 1, vec![second_id], goal, 2, QueueMode::Replace);
    for tick in 0..160 {
        app.update();
        let gap = unit_position(&app, 1).distance(unit_position(&app, 2));
        assert!(
            gap >= 1.0 - 1e-5,
            "overlap {gap} at tick {tick}, first={first:?} second={second:?} reverse={reverse}"
        );
    }
    (
        unit_position(&app, first_id),
        unit_position(&app, second_id),
        all_orders_complete(&app),
    )
}

#[test]
fn ordinary_enemy_order_does_not_change_arrival_before_contact() {
    // The enemy starts 1.0 clear of the mover. Each body moves 0.15 in one tick. When both go
    // to (50,50), the gap is 0.4 after tick 2 and 0.1 after tick 3. A gap of 0.1 is contact
    // range (0.101), so tick 3 is the tick of first contact. Ticks 1 and 2 are before contact.
    fn run(enemy_target: Option<Vec2>, ticks: usize) -> Vec<OwnState> {
        let mut app = fog_app(&[(49.0, 50.0), (51.0, 50.0)], &[0, 1]);
        assert!(!turts_sim::visibility::unit_permitted(
            app.world(),
            0,
            2,
            turts_mod_api::ObservationPhase::Completed
        ));
        send_order(
            &mut app,
            0,
            vec![1],
            Vec2::splat(50.0),
            1,
            QueueMode::Replace,
        );
        if let Some(target) = enemy_target {
            send_order(&mut app, 1, vec![2], target, 2, QueueMode::Replace);
        }
        (0..ticks)
            .map(|_| {
                app.update();
                own_state(&app, 1)
            })
            .collect()
    }
    // An enemy order away from our unit never gives contact: compare 40 ticks for those.
    for (target, ticks) in [
        (Vec2::splat(50.0), 2),
        (Vec2::new(55.0, 50.0), 40),
        (Vec2::new(45.0, 50.0), 2),
        (Vec2::new(51.0, 55.0), 40),
    ] {
        assert_eq!(
            run(None, ticks),
            run(Some(target), ticks),
            "a hidden enemy order to {target:?} changed our unit before contact"
        );
    }
}
#[test]
fn fog_cross_owner_physical_tie_has_no_winner() {
    let goal = Vec2::splat(50.0);
    for (first, second) in [
        ((49.0, 50.0), (50.0, 51.0)),
        ((49.0, 50.0), (51.0, 50.0)),
        ((48.0, 50.0), (52.0, 50.0)),
        ((48.63, 50.0), (50.0, 51.37)),
    ] {
        let mut previous = None;
        for reverse in [false, true] {
            let (a, b, complete) = fog_contest(first, second, reverse);
            assert!(
                complete,
                "tie never completed: {first:?} {second:?} reverse={reverse}"
            );
            assert!(
                (a.distance(goal) - b.distance(goal)).abs() < 1e-4 && a.distance(goal) > 0.4,
                "a tie had a winner: {first:?} {second:?} reverse={reverse} a={a:?} b={b:?}"
            );
            if let Some(previous) = previous {
                assert_eq!(previous, (a, b), "unit ids or slots changed a tie result");
            }
            previous = Some((a, b));
        }
    }
}
