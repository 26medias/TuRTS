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
            },
            Placement {
                unit: format!("{ORIGIN}:walker"),
                owner_slot: 0,
                pos: WorldPos { x: 20.0, y: 33.2 },
                contents: Vec::new(),
                facing: None,
            },
            Placement {
                unit: format!("{ORIGIN}:post"),
                owner_slot: 0,
                pos: WorldPos { x: 32.0, y: 32.0 },
                contents: Vec::new(),
                facing: None,
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
    send_order(&mut app, 0, vec![1, 2, 3], point, 1, QueueMode::Replace);
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
        let orders = entity.get::<UnitOrders>().unwrap();
        assert_eq!(orders.current, None, "unit {id} has no active failed move");
        assert_eq!(
            orders.last_failure.as_ref().map(|failure| failure.reason),
            Some(RejectReason::Blocked),
            "unit {id} receives the blocked failure"
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
