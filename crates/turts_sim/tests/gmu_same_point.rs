//! Row GMU, same point: the native slot check of design 6.3 step 2 (revision 15, ruling O14).
//!
//! On the authored `muster` package, troopers 21 to 40 move to (40, 30) and rest. Then troopers
//! 1 to 20 get a move of another group to the same point. Each slot of the second order must be at
//! least s - 0.00001 from each slot of the first order, s = 2 x r_max + 0.1. The pool of 6.3 is a
//! hex lattice anchored at the point (`movement.rs` `spread_destinations_with_context`), so a
//! build that reuses the slots of the resting group gives a distance of 0.
//!
//! The commands enter the queue at the same ticks as the server `--command-script` hook: an entry
//! for tick T is in the queue when the update that reaches tick T drains it. The process half of
//! the row (overlaps, R_near, deadline) is `crates/turts_server/tests/mc_group.rs`.

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::{ContentId, OrderDef, PackageId, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::movement::{MoveProgress, MovementPools};
use turts_sim::{
    Command, CommandKind, CommandQueue, OrderCommand, OrderTargetSim, Position, QueueMode,
    RejectReason, SlotId, Tick, UnitId, UnitOrders, Vec2, headless_app, install_package,
    stage_package,
};

/// The authored trooper footprint radius (`mods/muster/units/trooper.ron`).
const TROOPER_RADIUS: f32 = 0.5;
/// s of design 6.3 step 1 for a group of troopers.
const SPACING: f32 = 2.0 * TROOPER_RADIUS + 0.1;
const POINT: Vec2 = Vec2::new(40.0, 30.0);
const FIRST_ORDER_TICK: u32 = 1;
const SECOND_ORDER_TICK: u32 = 300;
const END_TICK: u32 = 700;

struct Muster {
    app: bevy_app::App,
    catalog: turts_mod_api::ContentCatalog,
}

fn muster() -> Muster {
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
    Muster { app, catalog }
}

fn move_order(
    catalog: &turts_mod_api::ContentCatalog,
    units: Vec<u32>,
    point: Vec2,
    group: u32,
) -> Command {
    let (order, definition): (ContentId, &OrderDef) = catalog
        .orders
        .iter()
        .find(|(id, _)| id.package.0 == "muster" && id.local == "order_move")
        .map(|(id, definition)| (id.clone(), definition))
        .expect("Muster order_move exists");
    Command {
        slot: SlotId(0),
        seq: group,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order,
            behaviour: definition.behaviour,
            target_kind: definition.target,
            policy: None,
            units,
            target: OrderTargetSim::Point(point),
            queue: QueueMode::Replace,
            group,
            allowed_units: definition.units.clone(),
            queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
        })),
    }
}

fn tick(app: &bevy_app::App) -> u32 {
    app.world().resource::<Tick>().0
}

/// One update. A script entry for the tick that this update reaches enters the queue first.
fn step(app: &mut bevy_app::App, script: &[(u32, Command)]) {
    let next = tick(app) + 1;
    for (at, command) in script {
        if *at == next {
            app.world_mut()
                .resource_mut::<CommandQueue>()
                .0
                .push_back(command.clone());
        }
    }
    app.update();
    assert_eq!(tick(app), next, "one update advances one fixed tick");
}

/// The slot of each listed unit: `MoveProgress.slot` when the unit holds one, else its position
/// (a resting unit stands on its slot, 6.5). The flag tells which source gave the value.
fn slots_of(app: &mut bevy_app::App, ids: &[u32]) -> BTreeMap<u32, (Option<u64>, Vec2)> {
    let positions: BTreeMap<u32, Vec2> = app
        .world_mut()
        .query::<(&UnitId, &Position)>()
        .iter(app.world())
        .filter(|(id, _)| ids.contains(&id.0))
        .map(|(id, position)| (id.0, position.0))
        .collect();
    let progress: BTreeMap<u32, (u64, Vec2)> = app
        .world_mut()
        .query::<(&UnitId, &MoveProgress)>()
        .iter(app.world())
        .filter(|(id, _)| ids.contains(&id.0))
        .map(|(id, progress)| (id.0, (progress.pool_id, progress.slot)))
        .collect();
    positions
        .into_iter()
        .map(|(id, position)| match progress.get(&id) {
            Some((pool, slot)) => (id, (Some(*pool), *slot)),
            None => (id, (None, position)),
        })
        .collect()
}

/// Each listed unit whose last order failed with `Blocked`, with its position.
fn blocked(app: &mut bevy_app::App, ids: &[u32]) -> Vec<(u32, Vec2)> {
    let positions: BTreeMap<u32, Vec2> = app
        .world_mut()
        .query::<(&UnitId, &Position)>()
        .iter(app.world())
        .map(|(id, position)| (id.0, position.0))
        .collect();
    let mut failed: Vec<_> = app
        .world_mut()
        .query::<(&UnitId, &UnitOrders)>()
        .iter(app.world())
        .filter(|(id, orders)| {
            ids.contains(&id.0)
                && orders
                    .last_failure
                    .as_ref()
                    .is_some_and(|failure| failure.reason == RejectReason::Blocked)
        })
        .map(|(id, _)| (id.0, positions[&id.0]))
        .collect();
    failed.sort_by_key(|(id, _)| *id);
    failed
}

#[test]
fn gmu_second_order_slots_keep_s_from_the_slots_of_the_resting_first_order() {
    let mut fixture = muster();
    let first: Vec<u32> = (21..=40).collect();
    let second: Vec<u32> = (1..=20).collect();
    let script = [
        (
            FIRST_ORDER_TICK,
            move_order(&fixture.catalog, first.clone(), POINT, 2),
        ),
        (
            SECOND_ORDER_TICK,
            move_order(&fixture.catalog, second.clone(), POINT, 3),
        ),
    ];
    let app = &mut fixture.app;
    while tick(app) < SECOND_ORDER_TICK - 1 {
        step(app, &script);
    }

    // The first order rests with 0 `Blocked` (row GMU, gated) before the second order spreads.
    assert_eq!(
        blocked(app, &first),
        Vec::new(),
        "GMU native: the first order (troopers 21 to 40) has Blocked units (unit id, position)"
    );
    let first_slots = slots_of(app, &first);
    assert_eq!(
        first_slots.len(),
        first.len(),
        "every first-order trooper exists before the second order"
    );
    let first_pools: BTreeSet<u64> = first_slots.values().filter_map(|(pool, _)| *pool).collect();

    // The update that reaches the second-order tick drains and spreads the second order.
    step(app, &script);
    assert_eq!(tick(app), SECOND_ORDER_TICK);
    let second_slots = slots_of(app, &second);
    let second_pools: BTreeSet<u64> = second_slots
        .values()
        .map(|(pool, _)| {
            (*pool).expect("each second-order trooper holds a slot in the spread tick")
        })
        .collect();
    assert_eq!(
        second_pools.len(),
        1,
        "the second order spreads into one pool: {second_pools:?}"
    );
    assert!(
        first_pools.is_disjoint(&second_pools),
        "the second order has a pool of its own: first {first_pools:?}, second {second_pools:?}"
    );
    let pool_id = *second_pools.iter().next().unwrap();
    let (pool_point, pool_spacing, primary_count, pool_slots) = {
        let pool = app
            .world()
            .resource::<MovementPools>()
            .0
            .get(&pool_id)
            .expect("the spread pool of the second order is live");
        (
            pool.point,
            pool.spacing,
            pool.primary_count,
            pool.slots.clone(),
        )
    };
    assert_eq!(
        pool_point, POINT,
        "the second pool is anchored at the point"
    );
    assert!(
        (pool_spacing - SPACING).abs() <= 1.0e-6,
        "the second pool uses s = {SPACING}, not {pool_spacing}"
    );
    for (unit, (_, slot)) in &second_slots {
        assert!(
            pool_slots.contains(slot),
            "second-order trooper {unit} holds slot {slot:?}, which is not in its pool"
        );
    }

    // 6.3 step 2: every slot of the second pool, primary or spare, keeps s from every slot of
    // the first order.
    let mut nearest = f32::INFINITY;
    for slot in &pool_slots {
        for (unit, (_, first_slot)) in &first_slots {
            let distance = slot.distance(*first_slot);
            nearest = nearest.min(distance);
            assert!(
                distance >= SPACING - 0.00001,
                "GMU slot check (6.3 step 2): second-order slot ({}, {}) is {distance} from the slot ({}, {}) of first-order trooper {unit}; s - 0.00001 = {}",
                slot.x,
                slot.y,
                first_slot.x,
                first_slot.y,
                SPACING - 0.00001
            );
        }
    }
    println!(
        "GMU native slot check: second pool {pool_id} has {} slots ({primary_count} primary); first order {} slots ({} from MoveProgress); nearest distance {nearest}",
        pool_slots.len(),
        first_slots.len(),
        first_slots
            .values()
            .filter(|(pool, _)| pool.is_some())
            .count()
    );

    while tick(app) < END_TICK {
        step(app, &script);
    }
    assert_eq!(
        blocked(app, &first),
        Vec::new(),
        "GMU native: the first order keeps 0 Blocked units after the second order"
    );
    // Recorded, not gated (row GMU, M4b owns this limit).
    let second_blocked = blocked(app, &second);
    println!(
        "GMU native record: second order Blocked count {} at tick {END_TICK}",
        second_blocked.len()
    );
    for (unit, position) in second_blocked {
        println!(
            "GMU native record: Blocked unit {unit} rests at ({}, {}), {} from the point",
            position.x,
            position.y,
            position.distance(POINT)
        );
    }
}
