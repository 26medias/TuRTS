//! In-process copies of the plain-map MC group rows (`crates/turts_server/tests/mc_group.rs`).
//!
//! The process rows are `#[ignore]` and need a server binary. These tests run the same authored
//! `muster` package, the same command ticks and the same gates through `headless_app`, so a
//! regression in the shared movement kernel is red in `cargo test -p turts_sim`.
//!
//! The order state follows `crates/turts_server/src/orderlog.rs`: `Blocked` is the Blocked
//! activity; `Done` is the first Idle tick after an active state with no failure in that tick.

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::{ContentId, OrderDef, PackageId, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::economy::views::UnitOrderActivityView;
use turts_sim::{
    Command, CommandKind, CommandQueue, OrderCommand, OrderTargetSim, Position, QueueMode, SlotId,
    Tick, UnitId, Vec2, headless_app, install_package, stage_package,
};

const RADIUS: f32 = 0.5;
const SPEED: f32 = 3.0;
/// Rule 2 (2026-11-23): a group order targets an area. `count` units of radius `RADIUS` rest
/// inside the goal area radius sqrt(sum of radius^2 / 0.35), plus one unit diameter.
fn r_near(count: usize) -> f32 {
    (count as f32 * RADIUS * RADIUS / 0.35).sqrt() + 2.0 * RADIUS
}

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
    point: (f32, f32),
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
            target: OrderTargetSim::Point(Vec2::new(point.0, point.1)),
            queue: QueueMode::Replace,
            group,
            allowed_units: definition.units.clone(),
            queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
        })),
    }
}

/// The recorded run: positions for each tick, and each order state transition.
struct Trace {
    /// `positions[tick]` holds each unit position after that tick. Index 0 is the start.
    positions: Vec<BTreeMap<u32, Vec2>>,
    /// `(tick, unit, state)` with state `Moving`, `Blocked`, `Done` or `Failed`.
    transitions: Vec<(u32, u32, &'static str)>,
    /// `slots[tick]` holds the claimed pool slot of each unit with a group move after that tick.
    slots: Vec<BTreeMap<u32, Vec2>>,
}

fn slots_of(app: &mut bevy_app::App) -> BTreeMap<u32, Vec2> {
    app.world_mut()
        .query::<(&UnitId, &turts_sim::movement::MoveProgress)>()
        .iter(app.world())
        .map(|(id, progress)| (id.0, progress.slot))
        .collect()
}

fn positions_of(app: &mut bevy_app::App) -> BTreeMap<u32, Vec2> {
    app.world_mut()
        .query::<(&UnitId, &Position)>()
        .iter(app.world())
        .map(|(id, position)| (id.0, position.0))
        .collect()
}

fn stop_order(catalog: &turts_mod_api::ContentCatalog, units: Vec<u32>, seq: u32) -> Command {
    let (order, definition): (ContentId, &OrderDef) = catalog
        .orders
        .iter()
        .find(|(id, _)| id.package.0 == "muster" && id.local == "order_stop")
        .map(|(id, definition)| (id.clone(), definition))
        .expect("Muster order_stop exists");
    Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order,
            behaviour: definition.behaviour,
            target_kind: definition.target,
            policy: None,
            units,
            target: OrderTargetSim::None,
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: definition.units.clone(),
            queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
        })),
    }
}

/// A move order: the command tick, the units, the point and the group.
type MoveOrder = (u32, Vec<u32>, (f32, f32), u32);

fn run(orders: &[MoveOrder], ticks: u32) -> Trace {
    run_with_stops(orders, &[], ticks)
}

fn run_with_stops(orders: &[MoveOrder], stops: &[(u32, Vec<u32>)], ticks: u32) -> Trace {
    let mut fixture = muster();
    let mut script: Vec<(u32, Command)> = orders
        .iter()
        .map(|(tick, units, point, group)| {
            (
                *tick,
                move_order(&fixture.catalog, units.clone(), *point, *group),
            )
        })
        .collect();
    script.extend(stops.iter().map(|(tick, units)| {
        (
            *tick,
            stop_order(&fixture.catalog, units.clone(), 1000 + *tick),
        )
    }));
    let app = &mut fixture.app;
    let mut trace = Trace {
        positions: vec![positions_of(app)],
        transitions: Vec::new(),
        slots: vec![slots_of(app)],
    };
    let mut previous = BTreeMap::<u32, &'static str>::new();
    let mut claims = BTreeMap::new();
    let mut slots = BTreeMap::new();
    for _ in 0..ticks {
        let next = app.world().resource::<Tick>().0 + 1;
        for (at, command) in &script {
            if *at == next {
                app.world_mut()
                    .resource_mut::<CommandQueue>()
                    .0
                    .push_back(command.clone());
            }
        }
        app.update();
        assert_eq!(app.world().resource::<Tick>().0, next);
        trace.positions.push(positions_of(app));
        trace.slots.push(slots_of(app));
        if std::env::var_os("TURTS_PLAIN_TRACE").is_some() {
            dump_recovery(app, next, &mut claims, &mut slots);
        }
        let mut current = BTreeMap::new();
        for row in &turts_sim::completed_world(app.world_mut()).unit_orders {
            let state = match row.activity {
                UnitOrderActivityView::Blocked => Some("Blocked"),
                UnitOrderActivityView::Idle => {
                    if row
                        .last_failure
                        .as_ref()
                        .is_some_and(|failure| failure.tick == next)
                    {
                        Some("Failed")
                    } else {
                        previous
                            .get(&row.unit_id)
                            .filter(|state| **state != "Done" && **state != "Failed")
                            .map(|_| "Done")
                    }
                }
                _ => Some("Moving"),
            };
            if let Some(state) = state {
                current.insert(row.unit_id, state);
            }
        }
        for (unit, state) in &current {
            if previous.get(unit) != Some(state) {
                trace.transitions.push((next, *unit, state));
            }
        }
        for (unit, state) in &previous {
            if !current.contains_key(unit) && *state != "Done" && *state != "Failed" {
                trace.transitions.push((next, *unit, "Done"));
            }
        }
        previous = current;
    }
    trace
}

/// Diagnostic only (`TURTS_PLAIN_TRACE=1`): print each recovery event with its neighbours.
fn dump_recovery(
    app: &mut bevy_app::App,
    tick: u32,
    claims: &mut BTreeMap<u32, (u8, bool, turts_sim::movement::MovementActivity)>,
    slots: &mut BTreeMap<u32, Vec2>,
) {
    use turts_sim::movement::MoveProgress;
    let rows: Vec<(u32, Vec2, Option<MoveProgress>, Option<Vec2>)> = app
        .world_mut()
        .query::<(
            &UnitId,
            &Position,
            Option<&MoveProgress>,
            Option<&turts_sim::MoveTarget>,
        )>()
        .iter(app.world())
        .map(|(id, position, progress, target)| {
            (id.0, position.0, progress.cloned(), target.map(|t| t.0))
        })
        .collect();
    let watched = std::env::var("TURTS_PLAIN_UNIT")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    let from = std::env::var("TURTS_PLAIN_FROM")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    for (id, position, progress, target) in &rows {
        if Some(*id) == watched && tick >= from {
            println!(
                "WATCH tick={tick} unit={id} at=({:.4},{:.4}) target={:?} progress={:?}",
                position.x,
                position.y,
                target.map(|t| (t.x, t.y)),
                progress.as_ref().map(|p| (
                    p.slot.x,
                    p.slot.y,
                    p.stalled_ticks,
                    p.claim_count,
                    p.pass_mode,
                    p.best_distance
                ))
            );
            if tick == from {
                for (other, at, other_progress, target) in &rows {
                    if other != id && at.distance(*position) < 2.2 {
                        println!(
                            "    near unit={other} at=({:.3},{:.3}) gap={:.3} target={:?} slot={:?}",
                            at.x,
                            at.y,
                            at.distance(*position) - 2.0 * RADIUS,
                            target.map(|t| (t.x, t.y)),
                            other_progress.as_ref().map(|p| (p.slot.x, p.slot.y)),
                        );
                    }
                }
            }
        }
        let Some(progress) = progress else { continue };
        if slots
            .insert(*id, progress.slot)
            .is_some_and(|before| before != progress.slot)
            && std::env::var_os("TURTS_PLAIN_SLOTS").is_some()
        {
            println!(
                "SLOT tick={tick} unit={id} at=({:.3},{:.3}) slot=({:.3},{:.3}) stalled={}",
                position.x, position.y, progress.slot.x, progress.slot.y, progress.stalled_ticks
            );
        }
        let now = (progress.claim_count, progress.pass_mode, progress.activity);
        let before = claims.insert(*id, now);
        if before.is_some_and(|before| before != now) {
            println!(
                "EVENT tick={tick} unit={id} at=({:.3},{:.3}) slot=({:.3},{:.3}) d={:.3} claims={} pass={} activity={:?} stalled={}",
                position.x,
                position.y,
                progress.slot.x,
                progress.slot.y,
                position.distance(progress.slot),
                progress.claim_count,
                progress.pass_mode,
                progress.activity,
                progress.stalled_ticks
            );
            for (other, at, other_progress, target) in &rows {
                if other != id && at.distance(*position) < 1.6 {
                    println!(
                        "    near unit={other} at=({:.3},{:.3}) gap={:.3} target={:?} slot={:?} activity={:?}",
                        at.x,
                        at.y,
                        at.distance(*position) - 2.0 * RADIUS,
                        target.map(|t| (t.x, t.y)),
                        other_progress.as_ref().map(|p| (p.slot.x, p.slot.y)),
                        other_progress.as_ref().map(|p| p.activity)
                    );
                }
            }
        }
    }
}

impl Trace {
    fn blocked(&self, ids: &[u32]) -> BTreeSet<u32> {
        self.transitions
            .iter()
            .filter(|(_, unit, state)| *state == "Blocked" && ids.contains(unit))
            .map(|(_, unit, _)| *unit)
            .collect()
    }

    fn assert_stationary_and_disjoint(&self, ids: &[u32], rest_ticks: u32, end: u32) {
        let first = (end - rest_ticks) as usize;
        for tick in first..=end as usize {
            for id in ids {
                let moved = self.positions[first][id].distance(self.positions[tick][id]);
                assert!(
                    moved <= 0.001,
                    "unit {id} moved {moved} over the rest interval at tick {tick}"
                );
            }
            self.assert_no_overlap(ids, tick as u32);
        }
    }

    fn assert_no_overlap(&self, ids: &[u32], tick: u32) {
        let at = &self.positions[tick as usize];
        for (index, a) in ids.iter().enumerate() {
            for b in &ids[index + 1..] {
                let distance = at[a].distance(at[b]);
                assert!(
                    distance >= 2.0 * RADIUS - 0.001,
                    "units {a} and {b} overlap at tick {tick}: distance {distance}"
                );
            }
        }
    }

    /// Rule 4 (2026-11-23): own units that move may overlap for a short time, at most 25 % of
    /// the smaller radius. Two units that both keep their position in a tick are at rest and
    /// must not overlap.
    fn assert_never_overlapping(&self, ids: &[u32]) {
        for tick in 1..self.positions.len() {
            let at = &self.positions[tick];
            let before = &self.positions[tick - 1];
            for (index, a) in ids.iter().enumerate() {
                for b in &ids[index + 1..] {
                    let distance = at[a].distance(at[b]);
                    let at_rest = at[a] == before[a] && at[b] == before[b];
                    let allowed = if at_rest { 1.0e-4 } else { 0.25 * RADIUS };
                    assert!(
                        distance >= 2.0 * RADIUS - allowed,
                        "units {a} and {b} overlap at tick {tick} (at rest: {at_rest}): distance {distance}"
                    );
                }
            }
        }
    }

    fn assert_state_by_deadline(
        &self,
        ids: &[u32],
        command_tick: u32,
        target: (f32, f32),
        states: &[&str],
    ) {
        let target = Vec2::new(target.0, target.1);
        let mut late = Vec::new();
        for id in ids {
            let travel_seconds = self.positions[1][id].distance(target) / SPEED;
            let deadline = command_tick + (2.0 * travel_seconds * 20.0).ceil() as u32 + 100;
            let done = self
                .transitions
                .iter()
                .filter(|(tick, unit, state)| {
                    unit == id && states.contains(state) && *tick >= command_tick
                })
                .map(|(tick, _, _)| *tick)
                .min()
                .unwrap_or_else(|| panic!("unit {id} has no terminal {states:?} transition"));
            if std::env::var_os("TURTS_PLAIN_TRACE").is_some() && done + 20 > deadline {
                println!("DEADLINE unit={id} done={done} deadline={deadline}");
            }
            late.push((*id, done, deadline));
        }
        late.retain(|(_, done, deadline)| done > deadline);
        assert!(
            late.is_empty(),
            "units reached {states:?} after the deadline (unit, tick, deadline): {late:?}"
        );
    }
}

fn troopers() -> Vec<u32> {
    (1..=40).collect()
}

#[test]
fn gm40_forty_troopers_rest_disjoint_and_near_the_point() {
    let ids = troopers();
    let trace = run(&[(1, ids.clone(), (30.0, 30.0), 1)], 420);
    assert_eq!(trace.blocked(&ids), BTreeSet::new(), "GM40 Blocked units");
    trace.assert_stationary_and_disjoint(&ids, 40, 420);
    for tick in 380..=420 {
        for id in &ids {
            let distance = trace.positions[tick][id].distance(Vec2::new(30.0, 30.0));
            assert!(
                distance <= r_near(ids.len()),
                "unit {id} is {distance} from the point at tick {tick}"
            );
        }
    }
    trace.assert_never_overlapping(&ids);
}

#[test]
fn gma_crossing_groups_rest_disjoint() {
    let first: Vec<u32> = (1..=20).collect();
    let second: Vec<u32> = (21..=40).collect();
    let trace = run(
        &[
            (1, first.clone(), (29.0, 35.0), 1),
            (20, second.clone(), (50.0, 22.0), 2),
        ],
        600,
    );
    trace.assert_stationary_and_disjoint(&troopers(), 40, 600);
    trace.assert_never_overlapping(&troopers());
}

#[test]
fn gma_second_group_passes_a_resting_group_and_both_rest_disjoint() {
    let first: Vec<u32> = (21..=40).collect();
    let second: Vec<u32> = (1..=20).collect();
    let trace = run(
        &[
            (1, first.clone(), (29.0, 35.0), 1),
            (240, second.clone(), (29.0, 44.0), 2),
        ],
        700,
    );
    trace.assert_stationary_and_disjoint(&first, 40, 239);
    trace.assert_stationary_and_disjoint(&troopers(), 40, 700);
    trace.assert_never_overlapping(&troopers());
}

#[test]
fn gmu_second_order_to_the_same_point_rests_disjoint_within_r_near() {
    let first: Vec<u32> = (21..=40).collect();
    let second: Vec<u32> = (1..=20).collect();
    let target = (40.0, 30.0);
    let trace = run(
        &[
            (1, first.clone(), target, 2),
            (300, second.clone(), target, 3),
        ],
        700,
    );
    assert_eq!(
        trace.blocked(&first),
        BTreeSet::new(),
        "first order Blocked"
    );
    trace.assert_state_by_deadline(&first, 1, target, &["Done"]);
    trace.assert_state_by_deadline(&second, 300, target, &["Done", "Blocked"]);
    trace.assert_stationary_and_disjoint(&troopers(), 40, 700);
    for id in troopers() {
        let distance = trace.positions[700][&id].distance(Vec2::new(target.0, target.1));
        // The two orders have one point: the crowd of the 40 units is one goal area.
        assert!(
            distance <= r_near(troopers().len()),
            "unit {id} rests {distance} from the point"
        );
    }
    trace.assert_never_overlapping(&troopers());
}

#[test]
fn gmu_second_order_to_the_clamped_corner_rests_disjoint() {
    let first: Vec<u32> = (1..=20).collect();
    let second: Vec<u32> = (21..=40).collect();
    let trace = run(
        &[
            (1, first.clone(), (70.0, 55.0), 1),
            (300, second.clone(), (70.0, 55.0), 2),
        ],
        1300,
    );
    trace.assert_stationary_and_disjoint(&troopers(), 40, 1300);
}

/// Two exchange rules must not undo each other. Before the repair the pocket rule (3a) moved a
/// mover to the slot beside its own and the deeper-slot rule (3b) moved it back, in every tick,
/// for more than twenty ticks: the mover stepped left and right and did not arrive, and its
/// peer lost its slot each time. One reversal can be correct (a new resting body changes the
/// facts). A unit that goes A, B, A, B, A in five successive ticks is in a rule cycle.
#[test]
fn gmu_post_ring_movers_do_not_swap_the_same_two_slots_tick_after_tick() {
    let ids = troopers();
    let trace = run(&[(1, ids.clone(), (21.0, 20.0), 3)], 260);
    let mut cycles = Vec::new();
    for id in &ids {
        let slot = |tick: usize| trace.slots[tick].get(id).copied();
        for tick in 4..trace.slots.len() {
            let (Some(a), Some(b)) = (slot(tick), slot(tick - 1)) else {
                continue;
            };
            if a != b
                && slot(tick - 2) == Some(a)
                && slot(tick - 3) == Some(b)
                && slot(tick - 4) == Some(a)
            {
                cycles.push((*id, tick));
                break;
            }
        }
    }
    assert!(
        cycles.is_empty(),
        "units swap the same two slots tick after tick (unit, tick): {cycles:?}"
    );
}

/// Stop and Replace while the group exchanges claims and follows routes (GM40 exchanges run
/// from about tick 55). The exchange keeps its state in the order progress only, so a Stop
/// ends it at once and a new order starts clean.
#[test]
fn stop_and_replace_during_exchanges_end_clean() {
    let ids = troopers();
    let stopped: Vec<u32> = (9..=16).collect();
    let replaced: Vec<u32> = (25..=40).collect();
    let kept: Vec<u32> = ids
        .iter()
        .copied()
        .filter(|id| !stopped.contains(id) && !replaced.contains(id))
        .collect();
    let trace = run_with_stops(
        &[
            (1, ids.clone(), (30.0, 30.0), 1),
            (80, replaced.clone(), (45.0, 22.0), 2),
        ],
        &[(70, stopped.clone())],
        600,
    );
    // Rule 1 (2026-11-23): a moving unit pushes an idle friendly unit aside, so the group can
    // push a stopped unit. A stopped unit gets no movement of its own after the Stop.
    for id in &stopped {
        let moves_again: Vec<u32> = trace
            .transitions
            .iter()
            .filter(|(tick, unit, state)| unit == id && *tick > 70 && *state == "Moving")
            .map(|(tick, _, _)| *tick)
            .collect();
        assert!(
            moves_again.is_empty(),
            "stopped unit {id} has a movement of its own at ticks {moves_again:?}"
        );
    }
    // The Stop must arrive during real motion, or the case proves nothing. The Stop applies
    // in tick 70, so the last motion is from tick 68 to tick 69.
    assert!(
        stopped
            .iter()
            .any(|id| trace.positions[68][id] != trace.positions[69][id]),
        "the stopped units were still moving when the Stop arrived"
    );
    assert_eq!(trace.blocked(&replaced), BTreeSet::new(), "replaced group");
    trace.assert_stationary_and_disjoint(&ids, 40, 600);
    trace.assert_never_overlapping(&ids);
    for id in &replaced {
        let distance = trace.positions[600][id].distance(Vec2::new(45.0, 22.0));
        assert!(
            distance <= r_near(ids.len()),
            "replaced unit {id} rests {distance} away"
        );
    }
    for id in &kept {
        let distance = trace.positions[600][id].distance(Vec2::new(30.0, 30.0));
        assert!(
            distance <= r_near(ids.len()) + 1.0,
            "kept unit {id} rests {distance} away"
        );
    }
}

/// The same commands give the same positions bit for bit.
#[test]
fn repeated_gm40_runs_are_bit_identical() {
    let ids = troopers();
    let first = run(&[(1, ids.clone(), (30.0, 30.0), 1)], 300);
    let second = run(&[(1, ids, (30.0, 30.0), 1)], 300);
    assert!(first.positions == second.positions);
}
