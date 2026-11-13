use super::*;
use crate::orders::apply::{apply_order, unit_has_pending_move, unit_toggleable};
use crate::orders::pending::{
    MovementDispatchState, PendingMovementOrders, PromotionHeaderIndex, SpreadIndexCache,
    dispatch_pending_movement, insert_promoted_request, pending_live_member_count,
    prepare_legacy_move_to_waiters, queue_movement_envelope, queue_unallocated_move_orders,
};
use crate::orders::plain::{
    PlainMoveOrderIdentities, PlainMovePriorState, PlainMoveReservations,
    arbitrate_plain_move_reservations, plain_move_reservation_capacity_available,
    record_plain_move_reservation,
};
use crate::orders::run::run_orders;
use crate::world::MoveTarget;
use crate::{NodeId, SlotId};
use bevy_ecs::prelude::World;
use bevy_math::Vec2;
use std::collections::{BTreeMap, VecDeque};
use turts_mod_api::{Behaviour, ContentId};
use turts_mod_api::{Footprint, GathererDef, PackageId, ProducerDef, UnitDef, UnitRegistry};

#[test]
fn legacy_waiter_preparation_uses_one_indexed_lookup_per_member() {
    const MEMBERS: u32 = 512;
    let mut world = World::new();
    let mut entity_by_unit = std::collections::HashMap::new();
    for unit_id in 1..=MEMBERS {
        let entity = world
            .spawn((
                crate::world::UnitId(unit_id),
                UnitOrders {
                    current: Some(Order {
                        order: ContentId::parse("test:move").unwrap(),
                        behaviour: Behaviour::Move,
                        target: OrderTargetSim::Point(Vec2::new(12.0, 8.0)),
                        group: 0,
                    }),
                    queue: VecDeque::from([Order {
                        order: ContentId::parse("test:old-future").unwrap(),
                        behaviour: Behaviour::Hold,
                        target: OrderTargetSim::None,
                        group: 0,
                    }]),
                    ..Default::default()
                },
            ))
            .id();
        entity_by_unit.insert(unit_id, entity);
    }
    LEGACY_WAITER_ENTITY_LOOKUPS.with(|count| count.set(0));
    prepare_legacy_move_to_waiters(
        &mut world,
        &(1..=MEMBERS).collect::<Vec<_>>(),
        &entity_by_unit,
    );
    assert_eq!(
        LEGACY_WAITER_ENTITY_LOOKUPS.with(std::cell::Cell::get),
        MEMBERS as usize,
        "each accepted member uses the shared entity index once"
    );
    assert!((1..=MEMBERS).all(|unit_id| {
        world
            .entity(entity_by_unit[&unit_id])
            .get::<UnitOrders>()
            .is_some_and(|orders| orders.queue.is_empty())
    }));
}

#[test]
fn unrelated_pending_members_do_not_scan_for_singleton_replacement_preflight() {
    PENDING_QUEUE_MEMBER_SCAN_STEPS.with(|count| count.set(0));
    let mut pending = PendingMovementOrders::default();
    assert!(pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1000,
        group: 91,
        pool_id: 0,
        point: Vec2::new(48.0, 48.0),
        unit_ids: (1000..17_000).collect(),
        order: None,
        member_orders: BTreeMap::new(),
        plain: false,
    }));
    assert_eq!(pending.pending_units.get(&0), Some(&16_000));

    for unit_id in 20_000..21_000 {
        assert!(pending.can_queue_after_replacing(0, 0, 1, &[unit_id]));
        pending.cancel_units(0, &[unit_id]);
    }

    PENDING_QUEUE_MEMBER_SCAN_STEPS.with(|count| {
        assert_eq!(
            count.get(),
            0,
            "unrelated pending members must use the maintained membership index"
        );
    });
    assert_eq!(pending.pending_units.get(&0), Some(&16_000));
    assert_eq!(pending.pending_member_counts.len(), 16_000);
}

fn gatherer_app() -> (bevy_app::App, bevy_ecs::entity::Entity) {
    let mut app = crate::headless_app(20);
    let id = ContentId::parse("test:worker").unwrap();
    let kind = app
        .world_mut()
        .resource_mut::<crate::UnitTypes>()
        .register_unit_type(
            &PackageId("test".into()),
            UnitDef {
                id: "worker".into(),
                name: "Worker".into(),
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: Some(GathererDef {
                    items: vec![],
                    gather_per_second: 1,
                    unload_per_second: 1,
                    interaction_range: 1.0,
                }),
                drop_off: None,
                producer: Some(ProducerDef {
                    recipes: vec![],
                    queue_capacity: 1,
                    spawn_offsets: vec![],
                    when_short: Vec::new(),
                }),
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
        )
        .unwrap();
    let mut catalog = turts_mod_api::ContentCatalog {
        resources: Default::default(),
        units: Default::default(),
        nodes: Default::default(),
        production: Default::default(),
        recipes: Default::default(),
        technologies: Default::default(),
        orders: Default::default(),
        order_queue_depths: Default::default(),
        source_files: Default::default(),
    };
    catalog.units.insert(
        id.clone(),
        turts_mod_api::ResolvedUnit {
            constructor: None,
            placement: None,
            build_area: None,
            id: id.clone(),
            name: "Worker".into(),
            footprint: Footprint::Circle { radius: 0.5 },
            movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
            inventory: None,
            gatherer: Some(turts_mod_api::ResolvedGatherer {
                items: vec![],
                gather_per_second: 1,
                unload_per_second: 1,
                interaction_range: 1.0,
            }),
            drop_off: None,
            producer: Some(turts_mod_api::ResolvedProducer {
                recipes: vec![],
                queue_capacity: 1,
                spawn_offsets: vec![],
            }),
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
    );
    let node_id = ContentId::parse("test:ore").unwrap();
    catalog.nodes.insert(
        node_id.clone(),
        turts_mod_api::ResolvedNode {
            id: node_id,
            name: "Ore".into(),
            item: ContentId::parse("test:iron").unwrap(),
            radius: 1.0,
        },
    );
    app.world_mut()
        .insert_resource(crate::bootstrap::InstalledCatalog(catalog));
    let entity = app
        .world_mut()
        .spawn((
            crate::UnitId(1),
            crate::UnitType(kind),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(8.0, 8.0)),
            crate::world::MoveTarget(Vec2::new(30.0, 30.0)),
            UnitOrders::default(),
        ))
        .id();
    app.world_mut()
        .insert_resource(crate::economy::SurfaceNodes(vec![
            crate::economy::NodeRecord {
                id: NodeId(1),
                content_id: ContentId::parse("test:ore").unwrap(),
                position: crate::Position(Vec2::new(10.0, 8.0)),
                remaining: 1,
            },
        ]));
    (app, entity)
}

#[test]
fn accepted_legacy_gather_clears_stale_movement_state_and_identity() {
    let (mut app, entity) = gatherer_app();
    let worker = ContentId::parse("test:worker").unwrap();
    let item = ContentId::parse("test:iron").unwrap();
    app.world_mut()
        .resource_mut::<crate::bootstrap::InstalledCatalog>()
        .0
        .units
        .get_mut(&worker)
        .unwrap()
        .gatherer
        .as_mut()
        .unwrap()
        .items = vec![item.clone()];
    app.world_mut()
        .insert_resource(crate::economy::EconomyOrders(BTreeMap::from([(
            1,
            crate::economy::GatherOrder {
                node: None,
                deposit: None,
                receiver: None,
                stage: crate::economy::gather::GatherStage::Idle,
                carry: crate::economy::gather::RateCarry::default(),
            },
        )])));
    app.world_mut().insert_resource(crate::EconomyLedger {
        inventories: BTreeMap::from([(
            1,
            crate::economy::ledger::Inventory {
                capacity: 4,
                accepted_items: std::collections::BTreeSet::from([item]),
                items: BTreeMap::new(),
            },
        )]),
        ..Default::default()
    });
    app.world_mut().entity_mut(entity).insert((
        crate::movement::MoveProgress {
            pool_id: 5,
            slot: Vec2::new(30.0, 30.0),
            activity: crate::movement::MovementActivity::Blocked,
            pass_mode: false,
            stalling_on_group: false,
            stalled_ticks: 20,
            best_distance: 1.0,
            claim_count: 3,

            follow_origin: None,
        },
        crate::movement::PlainMoveProgress {
            arrival_yield: false,
            original_point: Vec2::new(30.0, 30.0),
            slot: Vec2::new(30.0, 30.0),
            pool_slots: vec![],
            gather_goal: None,
            activity: crate::movement::MovementActivity::Blocked,
            stalled_ticks: 20,
            best_distance: 1.0,
            claim_count: 0,
            failure_recorded: true,
        },
        MoveTarget(Vec2::new(30.0, 30.0)),
        UnitOrders {
            current: Some(Order {
                order: ContentId::parse("test:move-a").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
                group: 4,
            }),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:hold-c").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 4,
            }]),
            last_failure: Some(OrderFailure {
                order: ContentId::parse("test:old-failure").unwrap(),
                reason: crate::RejectReason::Blocked,
                tick: 7,
            }),
        },
    ));
    app.world_mut()
        .insert_resource(PlainMoveOrderIdentities(BTreeMap::from([(
            1,
            Order {
                order: ContentId::parse("test:old-move").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
                group: 0,
            },
        )])));

    let result = crate::command::apply_command(
        app.world_mut(),
        &crate::command::Command {
            slot: SlotId(0),
            seq: 8,
            kind: crate::command::CommandKind::Gather {
                unit_id: 1,
                node_id: NodeId(1),
            },
        },
    );

    assert!(
        result.outcome.is_ok(),
        "legacy Gather refusal: {:?}",
        result.outcome
    );
    let actor = app.world().entity(entity);
    assert!(actor.get::<MoveTarget>().is_none());
    assert!(actor.get::<crate::movement::MoveProgress>().is_none());
    assert!(actor.get::<crate::movement::PlainMoveProgress>().is_none());
    assert_eq!(actor.get::<UnitOrders>(), Some(&UnitOrders::default()));
    assert!(
        !app.world()
            .resource::<PlainMoveOrderIdentities>()
            .0
            .contains_key(&1)
    );
    assert_eq!(
        app.world()
            .resource::<crate::economy::EconomyOrders>()
            .0
            .get(&1)
            .unwrap()
            .stage,
        crate::economy::gather::GatherStage::MovingToNode
    );
    let view = crate::economy::views::world_view(app.world_mut());
    assert_eq!(
        view.unit_orders
            .iter()
            .find(|order| order.unit_id == 1)
            .unwrap()
            .activity,
        crate::economy::views::UnitOrderActivityView::Gathering
    );
}

#[test]
fn refused_legacy_gather_preserves_existing_order_and_queue() {
    let (mut app, entity) = gatherer_app();
    let worker = ContentId::parse("test:worker").unwrap();
    let item = ContentId::parse("test:iron").unwrap();
    app.world_mut()
        .resource_mut::<crate::bootstrap::InstalledCatalog>()
        .0
        .units
        .get_mut(&worker)
        .unwrap()
        .gatherer
        .as_mut()
        .unwrap()
        .items = vec![item.clone()];
    app.world_mut()
        .resource_mut::<crate::economy::SurfaceNodes>()
        .0[0]
        .remaining = 0;
    app.world_mut().insert_resource(crate::EconomyLedger {
        inventories: BTreeMap::from([(
            1,
            crate::economy::ledger::Inventory {
                capacity: 4,
                accepted_items: std::collections::BTreeSet::from([item]),
                items: BTreeMap::new(),
            },
        )]),
        ..Default::default()
    });
    let before = UnitOrders {
        current: Some(Order {
            order: ContentId::parse("test:move-a").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
            group: 4,
        }),
        queue: VecDeque::from([Order {
            order: ContentId::parse("test:hold-c").unwrap(),
            behaviour: Behaviour::Hold,
            target: OrderTargetSim::None,
            group: 4,
        }]),
        last_failure: Some(OrderFailure {
            order: ContentId::parse("test:old-failure").unwrap(),
            reason: crate::RejectReason::Blocked,
            tick: 7,
        }),
    };
    app.world_mut().entity_mut(entity).insert(before.clone());

    let result = crate::command::apply_command(
        app.world_mut(),
        &crate::command::Command {
            slot: SlotId(0),
            seq: 9,
            kind: crate::command::CommandKind::Gather {
                unit_id: 1,
                node_id: NodeId(1),
            },
        },
    );

    assert_eq!(result.outcome, Err(crate::RejectReason::NodeDepleted));
    assert_eq!(
        app.world().entity(entity).get::<UnitOrders>(),
        Some(&before)
    );
    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>(),
        Some(&MoveTarget(Vec2::new(30.0, 30.0)))
    );
}

#[test]
fn one_pending_slot_refuses_a_two_member_queued_group_atomically_before_movement() {
    let (mut app, first) = gatherer_app();
    app.world_mut().insert_resource(crate::world::MapBounds {
        size: Vec2::new(64.0, 64.0),
    });
    let mobile_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    let second = app
        .world_mut()
        .spawn((
            crate::UnitId(2),
            crate::UnitType(mobile_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(9.0, 8.0)),
            MoveTarget(Vec2::new(30.0, 30.0)),
            UnitOrders::default(),
        ))
        .id();
    let move_order = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
        group: 77,
    };
    app.world_mut().entity_mut(first).insert(UnitOrders {
        current: Some(move_order.clone()),
        queue: VecDeque::from([Order {
            order: ContentId::parse("test:hold").unwrap(),
            behaviour: Behaviour::Hold,
            target: OrderTargetSim::None,
            group: 0,
        }]),
        last_failure: None,
    });
    app.world_mut().entity_mut(second).insert(UnitOrders {
        current: Some(Order {
            order: ContentId::parse("test:move-alt").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(31.0, 30.0)),
            group: 77,
        }),
        ..Default::default()
    });
    app.world_mut()
        .entity_mut(second)
        .insert(MoveTarget(Vec2::new(31.0, 30.0)));

    let pending_ids = (1000..17_383).collect::<Vec<_>>();
    let mut full_pending = PendingMovementOrders::default();
    assert!(full_pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1000,
        group: 3,
        pool_id: 0,
        point: Vec2::new(40.0, 40.0),
        unit_ids: pending_ids,
        order: None,
        member_orders: BTreeMap::new(),
        plain: false,
    }));
    assert_eq!(full_pending.pending_units.get(&0), Some(&16_383));
    app.world_mut().insert_resource(full_pending.clone());
    app.world_mut().resource_mut::<crate::AuditTrail>().enable();

    queue_unallocated_move_orders(app.world_mut());

    assert_eq!(
        app.world().resource::<PendingMovementOrders>(),
        &full_pending,
        "failed group admission leaves every pre-existing pending envelope unchanged"
    );
    for entity in [first, second] {
        assert!(
            app.world().entity(entity).get::<MoveTarget>().is_none(),
            "unit {} must not retain an unallocated target after group refusal",
            app.world().entity(entity).get::<crate::UnitId>().unwrap().0
        );
        assert!(
            app.world()
                .entity(entity)
                .get::<crate::movement::MoveProgress>()
                .is_none()
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .as_ref()
                .map(|failure| failure.reason),
            Some(crate::RejectReason::RateLimited)
        );
    }
    assert_eq!(
        app.world()
            .entity(first)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .map(|order| order.behaviour),
        Some(Behaviour::Hold),
        "normal same-phase queued continuation still starts after the atomic refusal"
    );
    assert_eq!(
        app.world()
            .entity(first)
            .get::<UnitOrders>()
            .unwrap()
            .last_failure
            .as_ref()
            .unwrap()
            .order,
        ContentId::parse("test:move").unwrap()
    );
    assert_eq!(
        app.world()
            .entity(second)
            .get::<UnitOrders>()
            .unwrap()
            .last_failure
            .as_ref()
            .unwrap()
            .order,
        ContentId::parse("test:move-alt").unwrap()
    );
    let operations = &app.world().resource::<crate::AuditTrail>().operations;
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].before.pending_movement_orders, full_pending);
    assert_eq!(operations[0].after.pending_movement_orders, full_pending);
    assert_eq!(operations[1].before, operations[0].after);
}

#[test]
fn merged_group_spread_keeps_each_member_order_identity() {
    let (mut app, first) = gatherer_app();
    let mobile_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    app.world_mut().entity_mut(first).remove::<UnitOrders>();
    let second = app
        .world_mut()
        .spawn((
            crate::UnitId(2),
            crate::UnitType(mobile_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(12.0, 8.0)),
            UnitOrders::default(),
        ))
        .id();
    let first_order = Order {
        order: ContentId::parse("test:move-a").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
        group: 9,
    };
    let second_order = Order {
        order: ContentId::parse("test:move-b").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(20.0, 20.0)),
        group: 9,
    };
    for (unit_id, entity, order) in [
        (1, first, first_order.clone()),
        (2, second, second_order.clone()),
    ] {
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(order.clone()),
            ..Default::default()
        });
        assert!(queue_movement_envelope(
            app.world_mut(),
            MovementEnvelopeRequest {
                slot: SlotId(0),
                seq: unit_id,
                group: 9,
                point: match order.target {
                    OrderTargetSim::Point(point) => point,
                    _ => unreachable!(),
                },
                unit_ids: vec![unit_id],
                order: Some(order.clone()),
                member_orders: BTreeMap::from([(unit_id, order)]),
                plain: false,
            },
        ));
    }
    let pending = app.world().resource::<PendingMovementOrders>();
    let envelope = pending.queues.get(&0).unwrap().front().unwrap();
    assert_eq!(envelope.unit_ids, vec![1, 2]);
    assert_eq!(envelope.order_for_unit(1), Some(&first_order));
    assert_eq!(envelope.order_for_unit(2), Some(&second_order));
    assert_eq!(pending.pending_units.get(&0), Some(&2));

    let latest_order = Order {
        order: ContentId::parse("test:move-b-latest").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(25.0, 25.0)),
        group: 9,
    };
    assert!(queue_movement_envelope(
        app.world_mut(),
        MovementEnvelopeRequest {
            slot: SlotId(0),
            seq: 2,
            group: 9,
            point: Vec2::new(25.0, 25.0),
            unit_ids: vec![2],
            order: Some(latest_order.clone()),
            member_orders: BTreeMap::from([(2, latest_order.clone())]),
            plain: false,
        },
    ));
    let pending = app.world().resource::<PendingMovementOrders>();
    let envelope = pending.queues.get(&0).unwrap().front().unwrap();
    assert_eq!(
        envelope.point,
        Vec2::new(40.0, 40.0),
        "K4 retains first-P anchor"
    );
    assert_eq!(envelope.order_for_unit(1), Some(&first_order));
    assert_eq!(envelope.order_for_unit(2), Some(&latest_order));
    assert_eq!(pending.pending_units.get(&0), Some(&2));
    let mut canceled_copy = pending.clone();
    canceled_copy.cancel_units(0, &[1]);
    let surviving = canceled_copy.queues.get(&0).unwrap().front().unwrap();
    assert_eq!(surviving.unit_ids, vec![2]);
    assert!(surviving.order_for_unit(1).is_none());
    assert_eq!(surviving.order_for_unit(2), Some(&latest_order));

    dispatch_pending_movement(app.world_mut());
    assert_eq!(
        app.world()
            .entity(first)
            .get::<UnitOrders>()
            .unwrap()
            .current,
        Some(first_order)
    );
    assert_eq!(
        app.world()
            .entity(second)
            .get::<UnitOrders>()
            .unwrap()
            .current,
        Some(latest_order)
    );
    assert_eq!(
        app.world()
            .resource::<crate::movement::MovementPools>()
            .0
            .len(),
        1,
        "distinct accepted order IDs still use the K4 shared movement envelope"
    );
}

#[test]
fn retained_old_move_is_not_reenqueued_behind_pending_replacement() {
    let (mut app, entity) = gatherer_app();
    let old_order = Order {
        order: ContentId::parse("test:move-old").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
        group: 8,
    };
    let replacement = Order {
        order: ContentId::parse("test:move-new").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
        group: 9,
    };
    app.world_mut().entity_mut(entity).insert(UnitOrders {
        current: Some(old_order),
        ..Default::default()
    });
    let mut pending = PendingMovementOrders::default();
    assert!(pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1,
        group: 9,
        pool_id: 0,
        point: Vec2::new(40.0, 40.0),
        unit_ids: vec![1],
        order: Some(replacement.clone()),
        member_orders: BTreeMap::from([(1, replacement.clone())]),
        plain: false,
    }));
    app.world_mut().insert_resource(pending.clone());

    queue_unallocated_move_orders(app.world_mut());

    assert_eq!(app.world().resource::<PendingMovementOrders>(), &pending);
    assert_eq!(
        app.world()
            .resource::<PendingMovementOrders>()
            .queues
            .get(&0)
            .unwrap()
            .len(),
        1,
        "the retained Move A cannot be appended behind replacement B"
    );
    assert_eq!(
        app.world()
            .resource::<PendingMovementOrders>()
            .queues
            .get(&0)
            .unwrap()[0]
            .order_for_unit(1),
        Some(&replacement)
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .order,
        ContentId::parse("test:move-old").unwrap(),
        "the old current order remains until B dispatches"
    );
}

#[test]
fn production_dispatch_reuses_the_spatial_index_for_many_small_envelopes() {
    let (mut app, first) = gatherer_app();
    app.world_mut().insert_resource(crate::world::MapBounds {
        size: Vec2::new(128.0, 128.0),
    });
    let mobile_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    let post_type = app
        .world_mut()
        .resource_mut::<crate::UnitTypes>()
        .register_unit_type(
            &turts_mod_api::PackageId("test".into()),
            UnitDef {
                id: "post".into(),
                name: "Post".into(),
                movement: None,
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
        )
        .unwrap();
    for index in 0..512_u32 {
        let column = index % 32;
        let row = index / 32;
        app.world_mut().spawn((
            crate::UnitId(1_000 + index),
            crate::UnitType(post_type),
            crate::Owner(SlotId(1)),
            crate::Position(Vec2::new(
                30.0 + column as f32 * 1.5,
                2.0 + row as f32 * 1.5,
            )),
        ));
    }
    for index in 1..128_u32 {
        let column = index % 16;
        let row = index / 16;
        app.world_mut().spawn((
            crate::UnitId(index + 1),
            crate::UnitType(mobile_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(
                10.0 + column as f32 * 1.5,
                35.0 + row as f32 * 1.5,
            )),
            UnitOrders::default(),
        ));
    }
    let order = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(90.0, 90.0)),
        group: 0,
    };
    for unit_id in 1..=128_u32 {
        assert!(queue_movement_envelope(
            app.world_mut(),
            MovementEnvelopeRequest {
                slot: SlotId(0),
                seq: unit_id,
                group: 0,
                point: Vec2::new(90.0, 90.0),
                unit_ids: vec![unit_id],
                order: Some(order.clone()),
                member_orders: BTreeMap::new(),
                plain: false,
            },
        ));
    }

    dispatch_pending_movement(app.world_mut());
    assert_eq!(
        app.world()
            .resource::<PendingMovementOrders>()
            .pending_units
            .get(&0),
        Some(&0)
    );
    let assigned: std::collections::BTreeSet<_> = app
        .world()
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<crate::UnitId>()?.0;
            (1..=128)
                .contains(&id)
                .then(|| {
                    entity
                        .get::<MoveTarget>()
                        .map(|target| (target.0.x.to_bits(), target.0.y.to_bits()))
                })
                .flatten()
        })
        .collect();
    assert_eq!(
        assigned.len(),
        1,
        "separate destination plans cannot reserve the point before arrival"
    );

    dispatch_pending_movement(app.world_mut());
    assert_eq!(
        app.world().resource::<SpreadIndexCache>().context_builds,
        1,
        "the production batch and second dispatch pass reuse one world index"
    );
}

#[test]
fn candidate_capacity_and_winner_indexes_visit_only_relevant_members() {
    let (mut capacity_app, _) = gatherer_app();
    let mut pending = PendingMovementOrders::default();
    assert!(pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1,
        group: 9,
        pool_id: 0,
        point: Vec2::new(50.0, 50.0),
        unit_ids: (10_000..26_000).collect(),
        order: None,
        member_orders: BTreeMap::new(),
        plain: false,
    }));
    capacity_app.world_mut().insert_resource(pending);
    let accepted_order = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(50.0, 50.0)),
        group: 0,
    };
    for unit_id in 27_000..27_064 {
        record_plain_move_reservation(
            capacity_app.world_mut(),
            unit_id,
            SlotId(0),
            unit_id,
            Vec2::new(50.0, 50.0),
            accepted_order.clone(),
            PlainMovePriorState::default(),
        );
    }
    PENDING_CAPACITY_LOOKUPS.with(|count| count.set(0));
    PENDING_QUEUE_MEMBER_SCAN_STEPS.with(|count| count.set(0));
    let admitted = (30_000..31_000)
        .filter(|unit_id| {
            if !plain_move_reservation_capacity_available(capacity_app.world(), SlotId(0), *unit_id)
            {
                return false;
            }
            record_plain_move_reservation(
                capacity_app.world_mut(),
                *unit_id,
                SlotId(0),
                *unit_id,
                Vec2::new(50.0, 50.0),
                accepted_order.clone(),
                PlainMovePriorState::default(),
            );
            true
        })
        .count();
    assert_eq!(admitted, 320, "the indexed counts still enforce 16,384");
    assert_eq!(
        PENDING_CAPACITY_LOOKUPS.with(std::cell::Cell::get),
        1000,
        "each admission checks only its actor membership and per-slot counters"
    );
    assert_eq!(
        PENDING_QUEUE_MEMBER_SCAN_STEPS.with(std::cell::Cell::get),
        0,
        "unrelated 16,000-member requests are not rescanned for each singleton"
    );

    let (mut dispatch_app, first) = gatherer_app();
    dispatch_app
        .world_mut()
        .insert_resource(crate::world::MapBounds {
            size: Vec2::new(64.0, 64.0),
        });
    let mobile_type = dispatch_app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    let target = Vec2::new(50.0, 50.0);
    for unit_id in 1..=128_u32 {
        let entity = if unit_id == 1 {
            first
        } else {
            dispatch_app
                .world_mut()
                .spawn((
                    crate::UnitId(unit_id),
                    crate::UnitType(mobile_type),
                    crate::Owner(SlotId(0)),
                    crate::Position(Vec2::new(
                        4.0 + (unit_id % 12) as f32 * 1.5,
                        4.0 + (unit_id / 12) as f32 * 1.5,
                    )),
                    UnitOrders::default(),
                ))
                .id()
        };
        dispatch_app
            .world_mut()
            .entity_mut(entity)
            .insert(MoveTarget(target));
        record_plain_move_reservation(
            dispatch_app.world_mut(),
            unit_id,
            SlotId(0),
            unit_id,
            target,
            accepted_order.clone(),
            PlainMovePriorState::default(),
        );
    }
    PLAIN_CANDIDATE_INDEX_STEPS.with(|count| count.set(0));
    dispatch_pending_movement(dispatch_app.world_mut());
    dispatch_pending_movement(dispatch_app.world_mut());
    assert!(
        dispatch_app
            .world()
            .resource::<PlainMoveReservations>()
            .entries
            .is_empty()
    );
    assert_eq!(
        dispatch_app
            .world()
            .resource::<PlainMoveReservations>()
            .budget_used,
        128
    );
    assert!(
        dispatch_app
            .world()
            .resource::<MovementDispatchState>()
            .is_default()
    );
    assert_eq!(
        PLAIN_CANDIDATE_INDEX_STEPS.with(std::cell::Cell::get),
        128 * 3,
        "one arbitration index, one dispatch index and one pop per winner bound candidate work linearly"
    );
}

#[test]
fn promotion_header_scan_is_shared_by_losers_with_the_same_predecessor() {
    const OLD: u32 = 16_000;
    const LOSERS: u32 = 64;
    let mut queue = VecDeque::new();
    for index in 0..OLD {
        queue.push_back(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 5,
            seq: index,
            group: 0,
            pool_id: u64::from(index + 1),
            point: Vec2::ZERO,
            unit_ids: vec![100_000 + index],
            order: None,
            member_orders: BTreeMap::new(),
            plain: true,
        });
    }
    PLAIN_PROMOTION_HEADER_STEPS.with(|count| count.set(0));
    let header = PromotionHeaderIndex::new(&queue);
    for index in (0..LOSERS).rev() {
        insert_promoted_request(
            &mut queue,
            PendingMovementOrder {
                slot: SlotId(0),
                accepted_tick: 6,
                seq: index,
                group: 0,
                pool_id: u64::from(OLD + index + 1),
                point: Vec2::new(index as f32, 0.0),
                unit_ids: vec![index + 1],
                order: None,
                member_orders: BTreeMap::new(),
                plain: false,
            },
            6,
            u64::from(OLD),
            &header,
        );
    }
    let promoted: Vec<_> = queue
        .iter()
        .skip(OLD as usize)
        .map(|request| request.unit_ids[0])
        .collect();
    assert_eq!(promoted, (1..=LOSERS).collect::<Vec<_>>());
    assert_eq!(
        PLAIN_PROMOTION_HEADER_STEPS.with(std::cell::Cell::get),
        OLD as usize,
        "one immutable predecessor boundary scans the old header once, regardless of loser count"
    );
}

#[test]
fn promotion_boundaries_are_isolated_per_slot_and_index_distinct_predecessors_once() {
    let mut long = VecDeque::from([pending_header(0, 5, 1), pending_header(0, 5, 2)]);
    let mut short = VecDeque::from([pending_header(1, 5, 1)]);
    let long_header = PromotionHeaderIndex::new(&long);
    let short_header = PromotionHeaderIndex::new(&short);
    insert_promoted_request(&mut long, pending_header(0, 6, 3), 6, 99, &long_header);
    insert_promoted_request(&mut short, pending_header(1, 6, 3), 6, 99, &short_header);
    assert_eq!(long.len(), 3);
    assert_eq!(short.len(), 2);

    let mut unequal_long = VecDeque::from([pending_header(0, 6, 1), pending_header(0, 6, 2)]);
    let mut unequal_short = VecDeque::from([pending_header(1, 6, 1)]);
    let long_index = PromotionHeaderIndex::new(&unequal_long);
    let short_index = PromotionHeaderIndex::new(&unequal_short);
    for offset in (0..3_u64).rev() {
        insert_promoted_request(
            &mut unequal_long,
            pending_header(0, 6, 101 + offset),
            6,
            99,
            &long_index,
        );
        insert_promoted_request(
            &mut unequal_short,
            pending_header(1, 6, 201 + offset),
            6,
            99,
            &short_index,
        );
    }
    assert_eq!(
        unequal_long
            .iter()
            .map(|request| request.pool_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 101, 102, 103],
        "slot 0 keeps its original length-two boundary and FIFO order"
    );
    assert_eq!(
        unequal_short
            .iter()
            .map(|request| request.pool_id)
            .collect::<Vec<_>>(),
        vec![1, 201, 202, 203],
        "slot 1 uses its own length-one boundary and preserves FIFO"
    );

    const OLD: u32 = 16_000;
    const LOSERS: u32 = 64;
    let mut queue = VecDeque::new();
    for index in 0..OLD {
        queue.push_back(pending_header(0, 5, u64::from(index + 1)));
    }
    PLAIN_PROMOTION_HEADER_STEPS.with(|count| count.set(0));
    let header = PromotionHeaderIndex::new(&queue);
    for index in (0..LOSERS).rev() {
        insert_promoted_request(
            &mut queue,
            pending_header(0, 6, u64::from(OLD + index + 1)),
            6,
            u64::from(index),
            &header,
        );
    }
    assert_eq!(queue.len(), OLD as usize + LOSERS as usize);
    assert_eq!(
        queue
            .iter()
            .skip(OLD as usize)
            .map(|request| request.unit_ids[0])
            .collect::<Vec<_>>(),
        (OLD + 1..=OLD + LOSERS).collect::<Vec<_>>(),
        "nonmonotone raw header IDs do not change reverse-promotion FIFO"
    );
    assert_eq!(
        PLAIN_PROMOTION_HEADER_STEPS.with(std::cell::Cell::get),
        OLD as usize,
        "all predecessor queries share one immutable scan of the existing header"
    );
}

#[test]
fn promotion_header_index_matches_linear_reference_for_unordered_ids_and_reverse_fifo() {
    let original = VecDeque::from([
        pending_header(0, 6, 7),
        pending_header(0, 6, 2),
        pending_header(0, 6, 9),
        pending_header(0, 6, 4),
    ]);
    let mut indexed = original.clone();
    let mut reference = original.clone();
    let header = PromotionHeaderIndex::new(&original);
    let predecessors = [10, 8, 6, 6, 1];
    for (reverse_ordinal, predecessor) in predecessors.into_iter().enumerate() {
        let promoted = pending_header(0, 6, 20 + reverse_ordinal as u64);
        insert_promoted_request(&mut indexed, promoted.clone(), 6, predecessor, &header);

        let linear_boundary = reference
            .iter()
            .position(|request| (request.accepted_tick, request.pool_id) > (6, predecessor))
            .unwrap_or(reference.len());
        reference.insert(linear_boundary, promoted);
        assert_eq!(indexed, reference, "after predecessor query {predecessor}");
    }
}

fn pending_header(slot: u8, accepted_tick: u32, pool_id: u64) -> PendingMovementOrder {
    PendingMovementOrder {
        slot: SlotId(slot),
        accepted_tick,
        seq: pool_id as u32,
        group: 0,
        pool_id,
        point: Vec2::ZERO,
        unit_ids: vec![pool_id as u32],
        order: None,
        member_orders: BTreeMap::new(),
        plain: true,
    }
}

#[test]
fn missing_unit_target_does_not_consume_virtual_spread_capacity() {
    let (mut app, _first) = gatherer_app();
    let mut pending = PendingMovementOrders::default();
    let missing_target = Order {
        order: ContentId::parse("test:follow-missing").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Unit(999),
        group: 71,
    };
    let point_order = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(50.0, 50.0)),
        group: 71,
    };
    let member_orders = BTreeMap::from([
        (10, missing_target),
        (11, point_order.clone()),
        (12, point_order),
    ]);
    assert!(pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1,
        group: 71,
        pool_id: 0,
        point: Vec2::new(50.0, 50.0),
        unit_ids: vec![10, 11, 12],
        order: None,
        member_orders,
        plain: false,
    }));
    app.world_mut().insert_resource(pending);
    app.world_mut().insert_resource(MovementDispatchState {
        tick: Some(0),
        start_slot: 0,
        used: 2045,
        cursor_advanced: true,
    });
    for unit_id in [10, 11, 12] {
        app.world_mut().spawn(crate::UnitId(unit_id));
    }
    let candidate = Order {
        order: ContentId::parse("test:move-candidate").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(52.0, 50.0)),
        group: 0,
    };
    record_plain_move_reservation(
        app.world_mut(),
        1,
        SlotId(0),
        2,
        Vec2::new(52.0, 50.0),
        candidate,
        PlainMovePriorState::default(),
    );

    arbitrate_plain_move_reservations(app.world_mut());

    assert!(
        app.world()
            .resource::<PlainMoveReservations>()
            .winners
            .contains(&1),
        "the two valid pending members leave one cap slot for the later singleton"
    );
    assert_eq!(
        pending_live_member_count(
            app.world().resource::<PendingMovementOrders>().queues[&0]
                .front()
                .unwrap(),
            &app.world()
                .iter_entities()
                .filter_map(|entity| entity.get::<crate::UnitId>().map(|id| id.0))
                .collect(),
        ),
        2
    );
}

#[test]
fn dead_first_unit_target_member_does_not_anchor_surviving_envelope() {
    let (mut app, first) = gatherer_app();
    app.world_mut().entity_mut(first).insert(crate::UnitId(2));
    app.world_mut()
        .entity_mut(first)
        .insert(crate::Position(Vec2::new(12.0, 8.0)));
    let mobile_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    for (unit_id, position) in [(3, Vec2::new(24.0, 24.0)), (4, Vec2::new(48.0, 48.0))] {
        app.world_mut().spawn((
            crate::UnitId(unit_id),
            crate::UnitType(mobile_type),
            crate::Owner(SlotId(0)),
            crate::Position(position),
            UnitOrders::default(),
        ));
    }
    let dead_member_order = Order {
        order: ContentId::parse("test:follow-three").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Unit(3),
        group: 81,
    };
    let survivor_order = Order {
        order: ContentId::parse("test:follow-four").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Unit(4),
        group: 81,
    };
    let mut pending = PendingMovementOrders::default();
    assert!(pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1,
        group: 81,
        pool_id: 1,
        point: Vec2::new(24.0, 24.0),
        unit_ids: vec![1, 2],
        order: Some(dead_member_order.clone()),
        member_orders: BTreeMap::from([(1, dead_member_order), (2, survivor_order.clone())]),
        plain: false,
    }));
    app.world_mut().insert_resource(pending);

    dispatch_pending_movement(app.world_mut());

    let target = app.world().entity(first).get::<MoveTarget>().unwrap().0;
    assert!(
        target.distance(Vec2::new(48.0, 48.0)) < 2.0,
        "the surviving member's live Unit target, not the removed first member's target, anchors the pool"
    );
    assert!(
        target.distance(Vec2::new(24.0, 24.0)) > 20.0,
        "the dead first member's stale target must not redirect the survivor"
    );
    assert!(
        app.world()
            .resource::<PendingMovementOrders>()
            .pending_member_counts
            .is_empty()
    );
    assert_eq!(
        app.world()
            .entity(first)
            .get::<UnitOrders>()
            .unwrap()
            .current,
        Some(survivor_order)
    );
}

#[test]
fn target_gone_queued_hold_refreshes_reservation_before_survivor_spread() {
    let (mut app, first) = gatherer_app();
    let mobile_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    app.world_mut().entity_mut(first).insert((
        crate::Position(Vec2::new(8.0, 8.0)),
        MoveTarget(Vec2::new(40.0, 40.0)),
        UnitOrders {
            current: Some(Order {
                order: ContentId::parse("test:follow-gone").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Unit(99),
                group: 5,
            }),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:hold").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 0,
            }]),
            last_failure: None,
        },
    ));
    let survivor = app
        .world_mut()
        .spawn((
            crate::UnitId(2),
            crate::UnitType(mobile_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(12.0, 8.0)),
            UnitOrders::default(),
        ))
        .id();
    let failed_order = Order {
        order: ContentId::parse("test:follow-gone").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Unit(99),
        group: 5,
    };
    let survivor_order = Order {
        order: ContentId::parse("test:move-point").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(8.0, 8.0)),
        group: 5,
    };
    let mut pending = PendingMovementOrders::default();
    assert!(pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1,
        group: 5,
        pool_id: 1,
        point: Vec2::new(8.0, 8.0),
        unit_ids: vec![1, 2],
        order: Some(failed_order.clone()),
        member_orders: BTreeMap::from([(1, failed_order), (2, survivor_order)]),
        plain: false,
    }));
    app.world_mut().insert_resource(pending);

    dispatch_pending_movement(app.world_mut());

    assert_eq!(
        app.world().entity(first).get::<MoveTarget>(),
        None,
        "TargetGone starts the queued Hold before the survivor is spread"
    );
    let allocated = app
        .world()
        .entity(survivor)
        .get::<crate::movement::MoveProgress>()
        .expect("the surviving group member receives a movement pool")
        .slot;
    assert!(
        allocated.distance(Vec2::new(8.0, 8.0)) >= 1.1 - 1.0e-4,
        "the survivor's destination respects the failed member's new resting position"
    );
}

#[test]
fn queued_self_follow_waits_for_the_shared_cap_before_advancing_hold() {
    let (mut app, entity) = gatherer_app();
    let unit_id = 1;
    let move_id = ContentId::parse("test:move").unwrap();
    let hold_id = ContentId::parse("test:hold").unwrap();
    let missing_follow = Order {
        order: move_id.clone(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Unit(999),
        group: 0,
    };
    let queued_self_follow = Order {
        order: move_id.clone(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Unit(unit_id),
        group: 0,
    };
    let queued_hold = Order {
        order: hold_id,
        behaviour: Behaviour::Hold,
        target: OrderTargetSim::None,
        group: 0,
    };
    app.world_mut().entity_mut(entity).insert((
        UnitOrders {
            current: Some(missing_follow.clone()),
            queue: VecDeque::from([queued_self_follow.clone(), queued_hold.clone()]),
            last_failure: None,
        },
        MoveTarget(Vec2::new(16.0, 8.0)),
    ));
    let tick = app.world().resource::<crate::world::Tick>().0;
    let request = PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: tick,
        seq: 1,
        group: 1,
        pool_id: 1,
        point: Vec2::new(16.0, 8.0),
        unit_ids: vec![unit_id],
        order: Some(missing_follow.clone()),
        member_orders: BTreeMap::from([(unit_id, missing_follow)]),
        plain: false,
    };
    assert!(
        app.world_mut()
            .resource_mut::<PendingMovementOrders>()
            .queue(request)
    );
    *app.world_mut().resource_mut::<MovementDispatchState>() = MovementDispatchState {
        tick: Some(tick),
        start_slot: 0,
        used: 2048,
        cursor_advanced: true,
    };

    run_orders(app.world_mut());

    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert_eq!(orders.current.as_ref(), Some(&queued_self_follow));
    assert_eq!(orders.queue.front(), Some(&queued_hold));
    assert!(unit_has_pending_move(app.world(), unit_id));
    assert!(
        app.world()
            .resource::<PendingMovementOrders>()
            .queues
            .get(&0)
            .unwrap()
            .iter()
            .any(
                |request| request.order_for_unit(unit_id).is_some_and(|order| {
                    order.behaviour == Behaviour::Move
                        && order.target == OrderTargetSim::Unit(unit_id)
                })
            ),
        "the self-follow waits as an initial movement envelope"
    );
}

#[test]
fn second_dispatch_refreshes_reservations_after_rate_limited_hold_continuation() {
    let (mut app, held) = gatherer_app();
    app.world_mut().insert_resource(crate::world::MapBounds {
        size: Vec2::new(64.0, 64.0),
    });
    let mobile_type = app.world().entity(held).get::<crate::UnitType>().unwrap().0;
    app.world_mut().entity_mut(held).insert((
        crate::Position(Vec2::new(20.0, 20.0)),
        MoveTarget(Vec2::new(40.0, 40.0)),
        UnitOrders {
            current: Some(Order {
                order: ContentId::parse("test:move-refused").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                group: 77,
            }),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:hold-next").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 0,
            }]),
            last_failure: None,
        },
    ));
    let old_target = Vec2::new(40.0, 40.0);
    let hold_position = Vec2::new(20.0, 20.0);
    let second = app
        .world_mut()
        .spawn((
            crate::UnitId(2),
            crate::UnitType(mobile_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(8.0, 8.0)),
        ))
        .id();
    let third = app
        .world_mut()
        .spawn((
            crate::UnitId(3),
            crate::UnitType(mobile_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(10.0, 8.0)),
        ))
        .id();

    // Build the actual production index before the first dispatch pass.
    dispatch_pending_movement(app.world_mut());
    let stale_fill = (1000..17_382).collect::<Vec<_>>();
    let mut pending = PendingMovementOrders::default();
    assert!(pending.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1000,
        group: 99,
        pool_id: 0,
        point: Vec2::new(50.0, 50.0),
        unit_ids: stale_fill,
        order: None,
        member_orders: BTreeMap::new(),
        plain: false,
    }));
    for (unit_id, point, group) in [(2, old_target, 1), (3, hold_position, 2)] {
        assert!(pending.queue(PendingMovementOrder {
            slot: SlotId(0),
            accepted_tick: 0,
            seq: unit_id,
            group,
            pool_id: 0,
            point,
            unit_ids: vec![unit_id],
            order: None,
            member_orders: BTreeMap::new(),
            plain: false,
        }));
    }
    assert_eq!(pending.pending_units.get(&0), Some(&16_384));
    app.world_mut().insert_resource(pending);

    // The refusal replaces the old target reservation with the current
    // body reservation because the queued Hold starts in this same phase.
    queue_unallocated_move_orders(app.world_mut());
    assert_eq!(
        app.world()
            .entity(held)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .map(|order| order.behaviour),
        Some(Behaviour::Hold)
    );
    assert!(app.world().entity(held).get::<MoveTarget>().is_none());

    // The second production dispatch reuses the refreshed index: the old
    // destination is available, while the new Hold position remains taken.
    dispatch_pending_movement(app.world_mut());
    let old_target_assignee = app
        .world()
        .entity(second)
        .get::<MoveTarget>()
        .expect("the stale refused target must no longer reserve this point");
    assert_eq!(old_target_assignee.0, old_target);
    let hold_target_assignee = app
        .world()
        .entity(third)
        .get::<MoveTarget>()
        .expect("the other envelope must still be dispatched");
    assert!(
        hold_target_assignee.0.distance(hold_position) >= 1.1 - 1.0e-5,
        "the new Hold position remains reserved, got {:?}",
        hold_target_assignee.0
    );
    assert_eq!(app.world().resource::<SpreadIndexCache>().context_builds, 1);
}

#[test]
fn pristine_idle_ticks_keep_legacy_audit_state_omitting_movement_scheduler_fields() {
    let mut app = crate::headless_app(20);
    for _ in 0..10 {
        app.update();
    }
    assert!(app.world().resource::<crate::world::Tick>().0 >= 8);
    assert!(app.world().resource::<MovementDispatchState>().is_default());
    assert!(app.world().resource::<PendingMovementOrders>().is_empty());
    let audit = crate::audit_state(app.world_mut());
    let serialized = serde_json::to_value(audit).unwrap();
    assert!(serialized.get("pending_movement_orders").is_none());
    assert!(serialized.get("movement_dispatch").is_none());
}

#[test]
fn first_cohort_turn_uses_global_tick_then_idle_ticks_keep_rotation() {
    let (mut app, first) = gatherer_app();
    for _ in 0..10 {
        app.update();
    }
    assert!(app.world().resource::<MovementDispatchState>().is_default());
    let first_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    for id in 2..=8_u32 {
        app.world_mut().spawn((
            crate::UnitId(id),
            crate::UnitType(first_type),
            crate::Owner(SlotId((id - 1) as u8)),
            crate::Position(Vec2::new(4.0 + id as f32, 4.0)),
            UnitOrders::default(),
        ));
    }
    let tick = app.world().resource::<crate::world::Tick>().0;
    let order = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
        group: 0,
    };
    for slot in 0..8_u8 {
        assert!(queue_movement_envelope(
            app.world_mut(),
            MovementEnvelopeRequest {
                slot: SlotId(slot),
                seq: u32::from(slot),
                group: 0,
                point: Vec2::new(40.0, 40.0),
                unit_ids: vec![u32::from(slot) + 1],
                order: Some(order.clone()),
                member_orders: BTreeMap::new(),
                plain: false,
            },
        ));
    }
    dispatch_pending_movement(app.world_mut());
    let first_turn = ((tick.saturating_sub(1)) % 8) as u8;
    assert_eq!(
        app.world().resource::<MovementDispatchState>().start_slot,
        first_turn
    );
    assert_eq!(
        app.world().resource::<PendingMovementOrders>().next_slot,
        (first_turn + 1) % 8
    );

    app.update();
    app.update();
    let next_tick = app.world().resource::<crate::world::Tick>().0;
    let expected_turn = ((tick.saturating_sub(1) + 2) % 8) as u8;
    assert_eq!(
        app.world().resource::<MovementDispatchState>().start_slot,
        expected_turn,
        "the stored first turn advances across two idle ticks"
    );
    assert_eq!(
        app.world().resource::<MovementDispatchState>().tick,
        Some(next_tick)
    );
}

#[test]
fn remove_capability_requires_by_command_before_filter_and_target_checks() {
    let (mut app, entity) = gatherer_app();
    let worker_id = ContentId::parse("test:worker").unwrap();
    let catalog = &mut app
        .world_mut()
        .resource_mut::<crate::bootstrap::InstalledCatalog>()
        .0;
    catalog.units.get_mut(&worker_id).unwrap().removal = Some(turts_mod_api::RemovalDef {
        by_command: false,
        contents: turts_mod_api::ContentsPolicy::Destroy,
    });

    let command = OrderCommand {
        order: ContentId::parse("test:remove").unwrap(),
        behaviour: Behaviour::Remove,
        target_kind: turts_mod_api::TargetKind::None,
        policy: None,
        units: vec![app.world().entity(entity).get::<crate::UnitId>().unwrap().0],
        target: OrderTargetSim::Point(Vec2::new(8.0, 8.0)),
        queue: QueueMode::Replace,
        group: 0,
        allowed_units: vec!["other:unit".into()],
        queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
    };
    let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 1, &command);
    assert_eq!(
        outcomes.refused,
        vec![(1, crate::command::RejectReason::NotRemovable)],
        "a disabled removal capability wins over custom filter and target failures"
    );
}

#[test]
fn append_gather_waits_behind_a_legacy_plain_move() {
    let (mut app, entity) = gatherer_app();
    let command = crate::Command {
        slot: SlotId(0),
        seq: 1,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:gather").unwrap(),
            behaviour: Behaviour::Gather,
            target_kind: turts_mod_api::TargetKind::Node,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Node(NodeId(1)),
            queue: QueueMode::Append,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![1]);
    let entity_ref = app.world().entity(entity);
    assert_eq!(
        entity_ref.get::<crate::world::MoveTarget>().unwrap().0,
        Vec2::new(30.0, 30.0)
    );
    let orders = entity_ref.get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert_eq!(orders.queue.len(), 1);
    assert_eq!(orders.queue[0].behaviour, Behaviour::Gather);
}

#[test]
fn deferred_replace_refusal_preserves_current_and_queued_orders() {
    let (mut app, entity) = gatherer_app();
    let current = Order {
        order: ContentId::parse("test:gather").unwrap(),
        behaviour: Behaviour::Gather,
        target: OrderTargetSim::Node(NodeId(1)),
        group: 4,
    };
    let queued = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(22.0, 22.0)),
        group: 5,
    };
    app.world_mut().entity_mut(entity).insert(UnitOrders {
        current: Some(current.clone()),
        queue: VecDeque::from([queued.clone()]),
        ..Default::default()
    });
    app.world_mut()
        .resource_mut::<PendingMovementOrders>()
        .pending_units
        .insert(0, 16_384);
    let command = crate::Command {
        slot: SlotId(0),
        seq: 1,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::Point,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            queue: QueueMode::Replace,
            group: 1,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, Vec::<u32>::new());
    assert_eq!(
        result.units.refused,
        vec![(1, crate::RejectReason::RateLimited)]
    );
    let after = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert_eq!(after.current, Some(current));
    assert_eq!(after.queue, VecDeque::from([queued]));
    assert!(
        app.world()
            .resource::<PendingMovementOrders>()
            .queues
            .is_empty()
    );
}

#[test]
fn pending_move_activity_overrides_prior_blocked_failure_without_erasing_it() {
    let (mut app, entity) = gatherer_app();
    let move_order = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
        group: 7,
    };
    app.world_mut().entity_mut(entity).insert(UnitOrders {
        current: Some(move_order.clone()),
        last_failure: Some(crate::OrderFailure {
            order: move_order.order.clone(),
            reason: crate::RejectReason::Blocked,
            tick: 0,
        }),
        ..Default::default()
    });
    assert!(queue_movement_envelope(
        app.world_mut(),
        MovementEnvelopeRequest {
            slot: SlotId(0),
            seq: 2,
            group: 7,
            point: Vec2::new(40.0, 40.0),
            unit_ids: vec![1],
            order: Some(move_order),
            member_orders: BTreeMap::new(),
            plain: false,
        },
    ));
    let view = crate::economy::views::world_view(app.world_mut());
    assert_eq!(
        view.unit_orders[0].activity,
        crate::economy::views::UnitOrderActivityView::Pending
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .last_failure
            .as_ref()
            .unwrap()
            .reason,
        crate::RejectReason::Blocked,
        "the prior failure remains stored while the replacement is pending"
    );
}

#[test]
fn appended_move_replaces_blocked_export_activity_but_keeps_failure_history() {
    let (mut app, entity) = gatherer_app();
    app.world_mut().entity_mut(entity).remove::<MoveTarget>();
    app.world_mut().entity_mut(entity).insert(UnitOrders {
        last_failure: Some(crate::OrderFailure {
            order: ContentId::parse("test:move").unwrap(),
            reason: crate::RejectReason::Blocked,
            tick: 0,
        }),
        ..Default::default()
    });
    let command = crate::Command {
        slot: SlotId(0),
        seq: 3,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::Point,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            queue: QueueMode::Append,
            group: 8,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![1]);
    let view = crate::economy::views::world_view(app.world_mut());
    assert_eq!(
        view.unit_orders[0].activity,
        crate::economy::views::UnitOrderActivityView::Moving
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .last_failure
            .as_ref()
            .unwrap()
            .reason,
        crate::RejectReason::Blocked
    );
}

#[test]
fn fresh_blocked_progress_precedes_active_move_in_export() {
    let (mut app, entity) = gatherer_app();
    let order = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
        group: 7,
    };
    app.world_mut().entity_mut(entity).insert((
        MoveTarget(Vec2::new(40.0, 40.0)),
        crate::movement::MoveProgress {
            pool_id: 1,
            slot: Vec2::new(40.0, 40.0),
            activity: crate::movement::MovementActivity::Blocked,
            pass_mode: false,
            stalling_on_group: false,
            stalled_ticks: 20,
            best_distance: 1.0,
            claim_count: 3,

            follow_origin: None,
        },
    ));
    app.world_mut().entity_mut(entity).insert(UnitOrders {
        current: Some(order),
        ..Default::default()
    });
    let view = crate::economy::views::world_view(app.world_mut());
    assert_eq!(
        view.unit_orders[0].activity,
        crate::economy::views::UnitOrderActivityView::Blocked
    );
}

#[test]
fn hold_replace_cancels_a_stale_pending_move_before_dispatch() {
    let (mut app, entity) = gatherer_app();
    let stale = Vec2::new(40.0, 40.0);
    assert!(queue_movement_envelope(
        app.world_mut(),
        MovementEnvelopeRequest {
            slot: SlotId(0),
            seq: 1,
            group: 9,
            point: stale,
            unit_ids: vec![1],
            order: Some(Order {
                order: ContentId::parse("test:move").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(stale),
                group: 9,
            }),
            member_orders: BTreeMap::new(),
            plain: false,
        },
    ));
    let command = crate::Command {
        slot: SlotId(0),
        seq: 2,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:hold").unwrap(),
            behaviour: Behaviour::Hold,
            target_kind: turts_mod_api::TargetKind::None,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::None,
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![1]);
    assert_eq!(
        app.world()
            .resource::<PendingMovementOrders>()
            .pending_units
            .get(&0),
        Some(&0),
        "Hold replacement removes the old request from pending membership"
    );
    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert_eq!(orders.current.as_ref().unwrap().behaviour, Behaviour::Hold);
    assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
    dispatch_pending_movement(app.world_mut());
    assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
}

#[test]
fn gather_replace_cancels_a_stale_pending_move_before_dispatch() {
    let (mut app, entity) = gatherer_app();
    app.world_mut()
        .resource_mut::<crate::economy::SurfaceNodes>()
        .0[0]
        .remaining = 0;
    let stale = Vec2::new(40.0, 40.0);
    assert!(queue_movement_envelope(
        app.world_mut(),
        MovementEnvelopeRequest {
            slot: SlotId(0),
            seq: 1,
            group: 9,
            point: stale,
            unit_ids: vec![1],
            order: Some(Order {
                order: ContentId::parse("test:move").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(stale),
                group: 9,
            }),
            member_orders: BTreeMap::new(),
            plain: false,
        },
    ));
    let command = crate::Command {
        slot: SlotId(0),
        seq: 2,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:gather").unwrap(),
            behaviour: Behaviour::Gather,
            target_kind: turts_mod_api::TargetKind::Node,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Node(NodeId(1)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![1]);
    assert_eq!(
        app.world()
            .resource::<PendingMovementOrders>()
            .pending_units
            .get(&0),
        Some(&0),
        "Gather replacement removes the old request from pending membership"
    );
    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert_eq!(
        orders.current.as_ref().unwrap().behaviour,
        Behaviour::Gather
    );
    assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
    dispatch_pending_movement(app.world_mut());
    assert!(app.world().entity(entity).get::<MoveTarget>().is_none());
}

#[test]
fn append_hold_refusal_preserves_active_gather_state() {
    let (mut app, entity) = gatherer_app();
    let active_gather = Order {
        order: ContentId::parse("test:gather").unwrap(),
        behaviour: Behaviour::Gather,
        target: OrderTargetSim::Node(NodeId(1)),
        group: 0,
    };
    app.world_mut().entity_mut(entity).insert(UnitOrders {
        current: Some(active_gather),
        ..Default::default()
    });
    app.world_mut()
        .insert_resource(crate::economy::EconomyOrders(
            [(
                1,
                crate::economy::GatherOrder {
                    node: Some(NodeId(1)),
                    deposit: Some(NodeId(1)),
                    receiver: None,
                    stage: crate::economy::gather::GatherStage::Gathering,
                    carry: crate::economy::gather::RateCarry::default(),
                },
            )]
            .into(),
        ));
    let before = app
        .world()
        .resource::<crate::economy::EconomyOrders>()
        .clone();
    let command = crate::Command {
        slot: SlotId(0),
        seq: 2,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:hold").unwrap(),
            behaviour: Behaviour::Hold,
            target_kind: turts_mod_api::TargetKind::None,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::None,
            queue: QueueMode::Append,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(
        result.units.refused,
        vec![(1, crate::command::RejectReason::NotQueueable)]
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .behaviour,
        Behaviour::Gather
    );
    assert_eq!(
        *app.world().resource::<crate::economy::EconomyOrders>(),
        before
    );
}

#[test]
fn rejected_incompatible_gather_preserves_current_order_and_queue() {
    let (mut app, entity) = gatherer_app();
    let before_order = Order {
        order: ContentId::parse("test:hold").unwrap(),
        behaviour: Behaviour::Hold,
        target: OrderTargetSim::None,
        group: 0,
    };
    let queued = Order {
        order: ContentId::parse("test:move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
        group: 0,
    };
    app.world_mut().entity_mut(entity).insert(UnitOrders {
        current: Some(before_order),
        queue: VecDeque::from([queued]),
        last_failure: None,
    });
    let before = app
        .world()
        .entity(entity)
        .get::<UnitOrders>()
        .unwrap()
        .clone();
    app.world_mut().resource_mut::<crate::AuditTrail>().enable();
    let command = crate::Command {
        slot: SlotId(0),
        seq: 1,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:gather").unwrap(),
            behaviour: Behaviour::Gather,
            target_kind: turts_mod_api::TargetKind::Node,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Node(NodeId(1)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(
        result.units.refused,
        vec![(1, crate::RejectReason::IncompatibleItem)]
    );
    assert_eq!(
        app.world().entity(entity).get::<UnitOrders>().unwrap(),
        &before
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<crate::world::MoveTarget>()
            .unwrap()
            .0,
        Vec2::new(30.0, 30.0)
    );
    let trail = app.world().resource::<crate::AuditTrail>();
    assert_eq!(trail.commands.len(), 1);
    assert_eq!(trail.operations.len(), 1);
    assert_eq!(trail.operations[0].before, trail.operations[0].after);
}

#[test]
fn late_refused_replace_keeps_provisional_plain_move_and_append_suffix() {
    let (mut app, entity) = gatherer_app();
    let candidate = OrderCommand {
        order: ContentId::parse("test:move-candidate").unwrap(),
        behaviour: Behaviour::Move,
        target_kind: turts_mod_api::TargetKind::Point,
        policy: None,
        units: vec![1],
        target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
        queue: QueueMode::Replace,
        group: 0,
        allowed_units: vec![],
        queue_depth: 16,
    };
    let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 1, &candidate);
    assert_eq!(outcomes.accepted, vec![1]);
    let suffix = OrderCommand {
        order: ContentId::parse("test:move-suffix").unwrap(),
        behaviour: Behaviour::Move,
        target_kind: turts_mod_api::TargetKind::Point,
        policy: None,
        units: vec![1],
        target: OrderTargetSim::Point(Vec2::new(45.0, 40.0)),
        queue: QueueMode::Append,
        group: 0,
        allowed_units: vec![],
        queue_depth: 16,
    };
    let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 2, &suffix);
    assert_eq!(outcomes.accepted, vec![1]);
    let before_orders = app
        .world()
        .entity(entity)
        .get::<UnitOrders>()
        .unwrap()
        .clone();
    let before_target = app.world().entity(entity).get::<MoveTarget>().unwrap().0;
    assert_eq!(before_target, Vec2::new(40.0, 40.0));
    assert!(before_orders.current.is_none());
    assert_eq!(before_orders.queue.len(), 1);
    assert_eq!(
        before_orders.queue[0].order, suffix.order,
        "the accepted Append suffix remains queued behind the legacy-style candidate"
    );
    assert!(
        app.world()
            .resource::<PlainMoveReservations>()
            .entries
            .contains_key(&1)
    );
    let before_identity = app.world().resource::<PlainMoveOrderIdentities>().0[&1].clone();

    app.world_mut().resource_mut::<crate::AuditTrail>().enable();
    let rejected = OrderCommand {
        order: ContentId::parse("test:gather-incompatible").unwrap(),
        behaviour: Behaviour::Gather,
        target_kind: turts_mod_api::TargetKind::Node,
        policy: None,
        units: vec![1],
        target: OrderTargetSim::Node(NodeId(1)),
        queue: QueueMode::Replace,
        group: 0,
        allowed_units: vec![],
        queue_depth: 16,
    };
    let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), 3, &rejected);
    assert_eq!(
        outcomes.refused,
        vec![(1, crate::command::RejectReason::IncompatibleItem)]
    );
    assert_eq!(
        app.world().entity(entity).get::<UnitOrders>().unwrap(),
        &before_orders
    );
    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>().unwrap().0,
        before_target
    );
    assert!(
        app.world()
            .resource::<PlainMoveReservations>()
            .entries
            .contains_key(&1)
    );
    assert_eq!(
        app.world().resource::<PlainMoveOrderIdentities>().0[&1],
        before_identity
    );
    let trail = app.world().resource::<crate::AuditTrail>();
    assert_eq!(trail.operations.len(), 1);
    assert_eq!(trail.operations[0].before, trail.operations[0].after);
}

#[test]
fn accepted_legacy_move_to_cancels_a_modern_plain_candidate() {
    let (mut app, entity) = gatherer_app();
    let candidate = crate::Command {
        slot: SlotId(0),
        seq: 1,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:modern-move").unwrap(),
            behaviour: Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::PointOrUnit,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    assert_eq!(
        crate::apply_command(app.world_mut(), &candidate)
            .units
            .accepted,
        vec![1]
    );
    assert!(
        app.world()
            .resource::<PlainMoveReservations>()
            .entries
            .contains_key(&1)
    );
    let active = Order {
        order: ContentId::parse("test:active-before-legacy").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(32.0, 32.0)),
        group: 5,
    };
    app.world_mut().entity_mut(entity).insert((
        UnitOrders {
            current: Some(active.clone()),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:future-before-legacy").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::Point(Vec2::new(8.0, 8.0)),
                group: 0,
            }]),
            last_failure: None,
        },
        crate::movement::MoveProgress {
            pool_id: 3,
            slot: Vec2::new(32.0, 32.0),
            activity: crate::movement::MovementActivity::Moving,
            pass_mode: false,
            stalling_on_group: false,
            stalled_ticks: 0,
            best_distance: 1.0,
            claim_count: 0,
            follow_origin: None,
        },
        crate::movement::PhysicalEscape {
            target: Some(Vec2::new(9.0, 8.0)),
        },
    ));

    let rejected = crate::Command {
        slot: SlotId(1),
        seq: 2,
        kind: crate::CommandKind::MoveTo {
            unit_ids: vec![1],
            target: Vec2::new(44.0, 44.0),
        },
    };
    assert_eq!(
        crate::apply_command(app.world_mut(), &rejected).outcome,
        Err(crate::command::RejectReason::NotOwner)
    );
    assert!(
        app.world()
            .resource::<PlainMoveReservations>()
            .entries
            .contains_key(&1)
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .current,
        Some(active)
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .len(),
        1
    );
    assert!(
        app.world()
            .entity(entity)
            .get::<crate::movement::MoveProgress>()
            .is_some()
    );

    let accepted = crate::Command {
        slot: SlotId(0),
        seq: 3,
        kind: crate::CommandKind::MoveTo {
            unit_ids: vec![1],
            target: Vec2::new(44.0, 44.0),
        },
    };
    assert_eq!(
        crate::apply_command(app.world_mut(), &accepted).outcome,
        Ok(crate::Applied { job_id: None })
    );
    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>().unwrap().0,
        Vec2::new(44.0, 44.0)
    );
    assert_eq!(
        app.world().entity(entity).get::<UnitOrders>().unwrap(),
        &UnitOrders::default()
    );
    assert!(
        app.world()
            .entity(entity)
            .get::<crate::movement::MoveProgress>()
            .is_none()
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<crate::movement::PhysicalEscape>()
            .and_then(|escape| escape.target),
        Some(Vec2::new(9.0, 8.0)),
        "legacy MoveTo replaces voluntary movement without erasing physical escape"
    );
    assert!(
        !app.world()
            .resource::<PlainMoveReservations>()
            .entries
            .contains_key(&1)
    );
    app.update();
    assert_eq!(
        app.world().entity(entity).get::<MoveTarget>().unwrap().0,
        Vec2::new(44.0, 44.0),
        "the replaced cohort progress cannot restore its older target"
    );
    assert!(
        !app.world()
            .resource::<PlainMoveOrderIdentities>()
            .0
            .contains_key(&1)
    );
}

#[test]
fn refused_legacy_multi_move_to_preserves_gather_and_prior_orders() {
    let (mut app, first) = gatherer_app();
    let first_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    let second = app
        .world_mut()
        .spawn((
            crate::UnitId(2),
            crate::UnitType(first_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(10.0, 8.0)),
            UnitOrders::default(),
        ))
        .id();
    let current = Order {
        order: ContentId::parse("test:old-current").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(20.0, 20.0)),
        group: 1,
    };
    let queued = Order {
        order: ContentId::parse("test:old-queued").unwrap(),
        behaviour: Behaviour::Hold,
        target: OrderTargetSim::Point(Vec2::new(10.0, 8.0)),
        group: 0,
    };
    let gather = crate::economy::GatherOrder {
        node: Some(crate::NodeId(7)),
        deposit: Some(crate::NodeId(7)),
        receiver: None,
        stage: crate::economy::gather::GatherStage::Gathering,
        carry: crate::economy::gather::RateCarry::default(),
    };
    for entity in [first, second] {
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(current.clone()),
            queue: VecDeque::from([queued.clone()]),
            last_failure: None,
        });
    }
    app.world_mut()
        .resource_mut::<crate::economy::EconomyOrders>()
        .0
        .insert(1, gather.clone());
    app.world_mut()
        .resource_mut::<crate::economy::EconomyOrders>()
        .0
        .insert(2, gather.clone());
    let before_targets = [
        app.world().entity(first).get::<MoveTarget>().copied(),
        app.world().entity(second).get::<MoveTarget>().copied(),
    ];
    let mut full = PendingMovementOrders::default();
    assert!(full.queue(PendingMovementOrder {
        slot: SlotId(0),
        accepted_tick: 0,
        seq: 1,
        group: 0,
        pool_id: 0,
        point: Vec2::new(48.0, 48.0),
        unit_ids: (10_000..26_384).collect(),
        order: None,
        member_orders: BTreeMap::new(),
        plain: true,
    }));
    app.world_mut().insert_resource(full);

    let refused = crate::Command {
        slot: SlotId(0),
        seq: 2,
        kind: crate::CommandKind::MoveTo {
            unit_ids: vec![1, 2],
            target: Vec2::new(40.0, 40.0),
        },
    };
    let result = crate::apply_command(app.world_mut(), &refused);
    assert!(result.units.accepted.is_empty());
    assert_eq!(result.units.refused.len(), 2);
    for (index, (unit_id, entity)) in [(1, first), (2, second)].into_iter().enumerate() {
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current,
            Some(current.clone())
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .queue,
            VecDeque::from([queued.clone()])
        );
        assert_eq!(
            app.world().resource::<crate::economy::EconomyOrders>().0[&unit_id],
            gather
        );
        assert_eq!(
            app.world().entity(entity).get::<MoveTarget>().copied(),
            before_targets[index]
        );
    }
    assert_eq!(
        app.world()
            .resource::<PendingMovementOrders>()
            .pending_units[&0],
        16_384
    );
}

#[test]
fn authored_plain_blocked_identity_is_acknowledged_while_legacy_multi_waits() {
    let (mut app, first) = gatherer_app();
    let first_type = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    let second = app
        .world_mut()
        .spawn((
            crate::UnitId(2),
            crate::UnitType(first_type),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(10.0, 8.0)),
            UnitOrders::default(),
        ))
        .id();
    let authored = Order {
        order: ContentId::parse("test:authored-old-move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(20.0, 20.0)),
        group: 0,
    };
    app.world_mut().entity_mut(first).insert((
        UnitOrders::default(),
        crate::movement::PlainMoveProgress {
            arrival_yield: false,
            original_point: Vec2::new(20.0, 20.0),
            slot: Vec2::new(8.0, 8.0),
            pool_slots: Vec::new(),
            gather_goal: None,
            activity: crate::movement::MovementActivity::Blocked,
            stalled_ticks: 20,
            best_distance: Vec2::new(20.0, 20.0).distance(Vec2::new(8.0, 8.0)),
            claim_count: 3,
            failure_recorded: false,
        },
    ));
    app.world_mut().init_resource::<PlainMoveOrderIdentities>();
    app.world_mut()
        .resource_mut::<PlainMoveOrderIdentities>()
        .0
        .insert(1, authored.clone());
    let tick = app.world().resource::<crate::world::Tick>().0;
    *app.world_mut().resource_mut::<MovementDispatchState>() = MovementDispatchState {
        tick: Some(tick),
        start_slot: 0,
        used: 2047,
        cursor_advanced: true,
    };

    let replacement = crate::Command {
        slot: SlotId(0),
        seq: 2,
        kind: crate::CommandKind::MoveTo {
            unit_ids: vec![1, 2],
            target: Vec2::new(40.0, 40.0),
        },
    };
    let admitted = crate::apply_command(app.world_mut(), &replacement);
    assert_eq!(admitted.units.accepted, vec![1, 2]);
    assert_eq!(
        app.world().resource::<PlainMoveOrderIdentities>().0.get(&1),
        Some(&authored)
    );
    run_orders(app.world_mut());

    let orders = app.world().entity(first).get::<UnitOrders>().unwrap();
    assert_eq!(orders.current, None);
    let failure = orders
        .last_failure
        .as_ref()
        .expect("prior A is acknowledged");
    assert_eq!(failure.order, authored.order);
    assert_eq!(failure.reason, crate::command::RejectReason::Blocked);
    assert_eq!(failure.tick, tick);
    assert!(
        app.world()
            .entity(first)
            .get::<crate::movement::PlainMoveProgress>()
            .is_some_and(|progress| progress.failure_recorded)
    );
    assert!(unit_has_pending_move(app.world(), 1));
    assert!(unit_has_pending_move(app.world(), 2));
    assert!(
        !app.world()
            .resource::<PlainMoveOrderIdentities>()
            .0
            .contains_key(&1)
    );
    assert!(
        app.world()
            .entity(second)
            .get::<UnitOrders>()
            .unwrap()
            .last_failure
            .is_none()
    );
}

#[test]
fn admitted_legacy_multi_move_retains_current_until_dispatch_and_keeps_new_append() {
    let (mut app, first) = gatherer_app();
    app.world_mut().insert_resource(crate::world::MapBounds {
        size: Vec2::new(64.0, 64.0),
    });
    let kind = app
        .world()
        .entity(first)
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    let second = app
        .world_mut()
        .spawn((
            crate::UnitId(2),
            crate::UnitType(kind),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(10.0, 8.0)),
            UnitOrders::default(),
        ))
        .id();
    let head = app
        .world_mut()
        .spawn((
            crate::UnitId(3),
            crate::UnitType(kind),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(16.0, 8.0)),
            UnitOrders::default(),
        ))
        .id();
    let current = Order {
        order: ContentId::parse("test:prior-move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(30.0, 30.0)),
        group: 4,
    };
    let gather = crate::economy::GatherOrder {
        node: Some(crate::NodeId(7)),
        deposit: Some(crate::NodeId(7)),
        receiver: None,
        stage: crate::economy::gather::GatherStage::Gathering,
        carry: crate::economy::gather::RateCarry::default(),
    };
    for (unit_id, entity) in [(1, first), (2, second)] {
        app.world_mut().entity_mut(entity).insert((
            UnitOrders {
                current: Some(current.clone()),
                queue: VecDeque::from([Order {
                    order: ContentId::parse("test:old-future").unwrap(),
                    behaviour: Behaviour::Hold,
                    target: OrderTargetSim::None,
                    group: 0,
                }]),
                last_failure: None,
            },
            MoveTarget(Vec2::new(30.0, 30.0)),
        ));
        app.world_mut()
            .resource_mut::<crate::economy::EconomyOrders>()
            .0
            .insert(unit_id, gather.clone());
    }
    let tick = app.world().resource::<crate::world::Tick>().0;
    let head_order = Order {
        order: ContentId::parse("test:head-move").unwrap(),
        behaviour: Behaviour::Move,
        target: OrderTargetSim::Point(Vec2::new(50.0, 50.0)),
        group: 7,
    };
    assert!(
        app.world_mut()
            .resource_mut::<PendingMovementOrders>()
            .queue(PendingMovementOrder {
                slot: SlotId(0),
                accepted_tick: tick,
                seq: 1,
                group: 7,
                pool_id: 0,
                point: Vec2::new(50.0, 50.0),
                unit_ids: vec![3],
                order: Some(head_order.clone()),
                member_orders: BTreeMap::from([(3, head_order)]),
                plain: false,
            })
    );
    *app.world_mut().resource_mut::<MovementDispatchState>() = MovementDispatchState {
        tick: Some(tick),
        start_slot: 0,
        used: 2047,
        cursor_advanced: true,
    };

    let replace = crate::Command {
        slot: SlotId(0),
        seq: 2,
        kind: crate::CommandKind::MoveTo {
            unit_ids: vec![1, 2],
            target: Vec2::new(40.0, 40.0),
        },
    };
    let result = crate::apply_command(app.world_mut(), &replace);
    assert_eq!(result.units.accepted, vec![1, 2]);
    for (unit_id, entity) in [(1, first), (2, second)] {
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current,
            Some(current.clone())
        );
        assert!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .queue
                .is_empty()
        );
        assert_eq!(
            app.world().resource::<crate::economy::EconomyOrders>().0[&unit_id].stage,
            crate::economy::gather::GatherStage::Gathering
        );
    }
    let append = crate::Command {
        slot: SlotId(0),
        seq: 3,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:append-hold").unwrap(),
            behaviour: Behaviour::Hold,
            target_kind: turts_mod_api::TargetKind::None,
            policy: None,
            units: vec![1, 2],
            target: OrderTargetSim::None,
            queue: QueueMode::Append,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    assert_eq!(
        crate::apply_command(app.world_mut(), &append)
            .units
            .accepted,
        vec![1, 2]
    );

    dispatch_pending_movement(app.world_mut());
    assert_eq!(
        app.world()
            .resource::<PendingMovementOrders>()
            .pending_units[&0],
        2
    );
    for entity in [first, second] {
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current,
            Some(current.clone())
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .queue
                .len(),
            1
        );
    }
    app.world_mut().resource_mut::<crate::world::Tick>().0 = tick + 1;
    dispatch_pending_movement(app.world_mut());
    for (unit_id, entity) in [(1, first), (2, second)] {
        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert_eq!(orders.current, None);
        assert_eq!(orders.queue.len(), 1, "only post-Replace Append survives");
        assert!(app.world().entity(entity).get::<MoveTarget>().is_some());
        assert!(
            !app.world()
                .resource::<PendingMovementOrders>()
                .pending_member_counts
                .contains_key(&unit_id)
        );
        assert_eq!(
            app.world().resource::<crate::economy::EconomyOrders>().0[&unit_id].stage,
            crate::economy::gather::GatherStage::Idle
        );
    }
    assert!(app.world().entity(head).get::<MoveTarget>().is_some());
}

#[test]
fn gather_on_an_existing_empty_node_is_accepted_then_fails_as_depleted() {
    let (mut app, entity) = gatherer_app();
    app.world_mut()
        .resource_mut::<crate::economy::SurfaceNodes>()
        .0[0]
        .remaining = 0;
    let command = crate::Command {
        slot: SlotId(0),
        seq: 1,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:gather").unwrap(),
            behaviour: Behaviour::Gather,
            target_kind: turts_mod_api::TargetKind::Node,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Node(NodeId(1)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![1]);
    assert!(
        app.world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .is_some()
    );
    app.world_mut().resource_mut::<crate::Tick>().0 = 8;
    run_orders(app.world_mut());
    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert_eq!(
        orders.last_failure.as_ref().unwrap().reason,
        crate::RejectReason::NodeDepleted
    );
    assert_eq!(orders.last_failure.as_ref().unwrap().tick, 8);
}

#[test]
fn immediate_cancel_ignores_unit_order_queue_and_preserves_it_on_failure() {
    let (mut app, entity) = gatherer_app();
    let existing = UnitOrders {
        current: Some(Order {
            order: ContentId::parse("test:hold").unwrap(),
            behaviour: Behaviour::Hold,
            target: OrderTargetSim::None,
            group: 0,
        }),
        queue: VecDeque::from([Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 0,
        }]),
        last_failure: None,
    };
    app.world_mut().entity_mut(entity).insert(existing.clone());
    let command = crate::Command {
        slot: SlotId(0),
        seq: 1,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:cancel").unwrap(),
            behaviour: Behaviour::CancelProduction,
            target_kind: turts_mod_api::TargetKind::Job,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Job(crate::JobId(999)),
            queue: QueueMode::Append,
            group: 0,
            allowed_units: vec![],
            queue_depth: 1,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(
        result.units.refused,
        vec![(1, crate::RejectReason::UnknownJob)]
    );
    assert_eq!(
        app.world().entity(entity).get::<UnitOrders>().unwrap(),
        &existing
    );
}

#[test]
fn set_enabled_keeps_all_legacy_toggleable_capability_families() {
    let (app, _) = gatherer_app();
    let id = ContentId::parse("test:worker").unwrap();
    let base = app
        .world()
        .resource::<crate::bootstrap::InstalledCatalog>()
        .0
        .units[&id]
        .clone();
    let mut variants = Vec::new();
    let mut unit = base.clone();
    unit.extractor = Some(turts_mod_api::ExtractorDef {
        per_second: 1,
        interaction_range: 1.0,
    });
    variants.push(unit);
    let mut unit = base.clone();
    unit.inserter = Some(turts_mod_api::InserterDef {
        reach: 1.0,
        swing_ms: 1,
        return_ms: 1,
    });
    variants.push(unit);
    let mut unit = base.clone();
    unit.machine = Some(turts_mod_api::ResolvedMachine {
        recipe: ContentId::parse("test:recipe").unwrap(),
    });
    variants.push(unit);
    let mut unit = base.clone();
    unit.research = Some(turts_mod_api::ResolvedResearchStation {
        technologies: vec![],
        queue_capacity: 1,
    });
    variants.push(unit);
    let mut unit = base.clone();
    unit.producer = Some(turts_mod_api::ResolvedProducer {
        recipes: vec![],
        queue_capacity: 1,
        spawn_offsets: vec![],
    });
    variants.push(unit);
    let mut unit = base.clone();
    unit.weapon = Some(turts_mod_api::ResolvedWeapon {
        range: 1.0,
        cooldown_ms: 1,
        damage: 1,
        ammo: None,
        targets: turts_mod_api::TargetFilter {
            slots: turts_mod_api::SlotRelation::Any,
        },
    });
    variants.push(unit);
    assert!(variants.iter().all(unit_toggleable));
    let mut conveyor_only = base;
    conveyor_only.producer = None;
    conveyor_only.gatherer = None;
    conveyor_only.conveyor = Some(turts_mod_api::ResolvedConveyor {
        slots_per_piece: 1,
        slots_per_second: 1,
        accepts: vec![],
    });
    assert!(!unit_toggleable(&conveyor_only));
}

#[test]
fn immediate_produce_returns_the_first_job_in_unit_id_order() {
    let (mut app, _) = gatherer_app();
    let producer = ContentId::parse("test:worker").unwrap();
    let recipe = ContentId::parse("test:recipe").unwrap();
    let mut catalog = app
        .world()
        .resource::<crate::bootstrap::InstalledCatalog>()
        .0
        .clone();
    catalog
        .units
        .get_mut(&producer)
        .unwrap()
        .producer
        .as_mut()
        .unwrap()
        .recipes = vec![recipe.clone()];
    catalog.production.insert(
        recipe.clone(),
        turts_mod_api::ResolvedProduction {
            id: recipe.clone(),
            name: "Recipe".into(),
            output_unit: producer.clone(),
            duration_ms: 1000,
            costs: vec![],
            payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
            cancel_refunds: turts_mod_api::CancelRefunds {
                queued: turts_mod_api::RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                running: turts_mod_api::RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                output_blocked: turts_mod_api::RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                destroyed: turts_mod_api::RefundRatio::ZERO,
            },
        },
    );
    app.world_mut()
        .insert_resource(crate::bootstrap::InstalledCatalog(catalog));
    app.world_mut()
        .insert_resource(crate::bootstrap::ProductionDurations(
            std::collections::BTreeMap::from([(recipe.clone(), 20)]),
        ));
    let kind = app
        .world()
        .entity(
            app.world()
                .iter_entities()
                .find(|entity| entity.get::<crate::UnitId>().is_some_and(|id| id.0 == 1))
                .unwrap()
                .id(),
        )
        .get::<crate::UnitType>()
        .unwrap()
        .0;
    app.world_mut().spawn((
        crate::UnitId(2),
        crate::UnitType(kind),
        crate::Owner(SlotId(0)),
        crate::Position(Vec2::new(12.0, 8.0)),
        UnitOrders::default(),
    ));
    let command = crate::Command {
        slot: SlotId(0),
        seq: 1,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:produce").unwrap(),
            behaviour: Behaviour::Produce,
            target_kind: turts_mod_api::TargetKind::Content,
            policy: None,
            units: vec![2, 1],
            target: OrderTargetSim::Content(recipe),
            queue: QueueMode::Append,
            group: 1,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let result = crate::apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![1, 2]);
    assert_eq!(
        result.outcome,
        Ok(crate::Applied {
            job_id: Some(crate::JobId(1))
        })
    );
    let queues = app
        .world()
        .resource::<crate::economy::production::ProductionQueues>();
    assert_eq!(queues.0.get(&1).unwrap()[0].id, crate::JobId(1));
    assert_eq!(queues.0.get(&2).unwrap()[0].id, crate::JobId(2));
    let owner_view = crate::economy::views::world_view(app.world_mut());
    assert_eq!(
        owner_view
            .economy
            .queues
            .iter()
            .find(|queue| queue.producer == 1)
            .unwrap()
            .jobs[0]
            .id,
        1
    );
    let cancel = crate::Command {
        slot: SlotId(0),
        seq: 2,
        kind: crate::CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("test:cancel").unwrap(),
            behaviour: Behaviour::CancelProduction,
            target_kind: turts_mod_api::TargetKind::Job,
            policy: None,
            units: vec![1],
            target: OrderTargetSim::Job(result.outcome.unwrap().job_id.unwrap()),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    let cancelled = crate::apply_command(app.world_mut(), &cancel);
    assert_eq!(cancelled.units.accepted, vec![1]);
    let owner_view = crate::economy::views::world_view(app.world_mut());
    assert!(
        owner_view
            .economy
            .queues
            .iter()
            .all(|queue| queue.producer != 1 || queue.jobs.is_empty())
    );
}

#[test]
fn depleted_gather_writes_last_failure() {
    let mut app = crate::headless_app(20);
    let kind = app
        .world_mut()
        .resource_mut::<crate::UnitTypes>()
        .register_unit_type(
            &PackageId("test".into()),
            UnitDef {
                id: "worker".into(),
                name: "Worker".into(),
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
        )
        .unwrap();
    let order_id = ContentId::parse("test:gather").unwrap();
    let entity = app
        .world_mut()
        .spawn((
            crate::UnitId(1),
            crate::UnitType(kind),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(8.0, 8.0)),
            UnitOrders {
                current: Some(Order {
                    order: order_id.clone(),
                    behaviour: Behaviour::Gather,
                    target: OrderTargetSim::Node(NodeId(1)),
                    group: 0,
                }),
                queue: VecDeque::new(),
                last_failure: None,
            },
        ))
        .id();
    app.world_mut()
        .insert_resource(crate::economy::SurfaceNodes(vec![
            crate::economy::NodeRecord {
                id: NodeId(1),
                content_id: ContentId::parse("test:ore").unwrap(),
                position: crate::Position(Vec2::new(10.0, 8.0)),
                remaining: 0,
            },
        ]));
    app.world_mut()
        .resource_mut::<crate::EconomyOrders>()
        .0
        .insert(
            1,
            crate::economy::GatherOrder {
                node: Some(NodeId(1)),
                deposit: None,
                receiver: None,
                stage: crate::GatherStage::Gathering,
                carry: crate::RateCarry::default(),
            },
        );
    app.world_mut().resource_mut::<crate::Tick>().0 = 4;
    app.world_mut().resource_mut::<crate::AuditTrail>().enable();

    run_orders(app.world_mut());

    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert_eq!(
        orders.last_failure,
        Some(OrderFailure {
            order: order_id,
            reason: crate::RejectReason::NodeDepleted,
            tick: 4,
        })
    );
    assert_eq!(
        app.world().resource::<crate::AuditTrail>().operations[0].phase,
        crate::economy::transaction::AuditPhase::Orders
    );
}

#[test]
fn queued_gather_that_depletes_before_activation_fails_and_idles() {
    let mut app = crate::headless_app(20);
    let kind = app
        .world_mut()
        .resource_mut::<crate::UnitTypes>()
        .register_unit_type(
            &PackageId("test".into()),
            UnitDef {
                id: "worker".into(),
                name: "Worker".into(),
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
        )
        .unwrap();
    let order_id = ContentId::parse("test:gather").unwrap();
    let entity = app
        .world_mut()
        .spawn((
            crate::UnitId(1),
            crate::UnitType(kind),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(8.0, 8.0)),
            UnitOrders {
                current: None,
                queue: VecDeque::from([Order {
                    order: order_id.clone(),
                    behaviour: Behaviour::Gather,
                    target: OrderTargetSim::Node(NodeId(1)),
                    group: 0,
                }]),
                last_failure: None,
            },
        ))
        .id();
    app.world_mut()
        .insert_resource(crate::economy::SurfaceNodes(vec![
            crate::economy::NodeRecord {
                id: NodeId(1),
                content_id: ContentId::parse("test:ore").unwrap(),
                position: crate::Position(Vec2::new(10.0, 8.0)),
                remaining: 0,
            },
        ]));
    app.world_mut().resource_mut::<crate::Tick>().0 = 7;
    app.world_mut().resource_mut::<crate::AuditTrail>().enable();

    run_orders(app.world_mut());

    let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
    assert!(orders.current.is_none());
    assert!(orders.queue.is_empty());
    assert_eq!(
        orders.last_failure,
        Some(OrderFailure {
            order: order_id,
            reason: crate::RejectReason::NodeDepleted,
            tick: 7,
        })
    );
    assert_eq!(
        app.world().resource::<crate::AuditTrail>().operations.len(),
        1
    );
    assert_eq!(
        app.world().resource::<crate::AuditTrail>().operations[0].phase,
        crate::economy::transaction::AuditPhase::Orders
    );
}
