//! Process acceptance for MC movement contracts. These cases intentionally use the shipped
//! Muster package and the server's typed fixed-tick command hook.

mod support;

#[path = "../src/orders.rs"]
#[allow(dead_code)]
mod server_orders;

use std::collections::{BTreeMap, BTreeSet};

use support::mc;

const TROOPERS: [u32; 40] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
];

fn id_set(values: &[u32]) -> BTreeSet<u32> {
    values.iter().copied().collect()
}

fn rest_control_rows(samples: u32, drift_per_tick: f32) -> Vec<mc::PositionRow> {
    (1..=samples)
        .map(|tick| mc::PositionRow {
            tick,
            unit_id: 1,
            x: (tick - 1) as f32 * drift_per_tick,
            y: 0.0,
        })
        .collect()
}

#[test]
fn gm40_script_fixture_round_trips_through_typed_protocol_newtypes() {
    let path = mc::repo_root().join("tests/browser/fixtures/gm40-script.json");
    let original: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).expect("GM40 typed script fixture is readable"),
    )
    .expect("GM40 fixture is valid JSON");
    let entries = mc::read_script(&path);
    assert_eq!(
        serde_json::to_value(&entries).expect("typed GM40 entries serialize"),
        original,
        "fixture JSON must match typed UnitId and NodeId newtype serialization"
    );
    assert_eq!(
        entries.len(),
        5,
        "fixture includes GM40, crossing, and queued gather"
    );
    assert_eq!(entries[0].envelope.units.len(), 40);
    assert!(matches!(
        &entries[4].envelope.target,
        turts_net_protocol::OrderTarget::Node(turts_net_protocol::NodeId(1))
    ));
}

#[test]
fn selected_tick_coverage_accepts_complete_configured_end_and_overshoot() {
    let rows = rest_control_rows(43, 0.0);
    assert_eq!(
        mc::assert_actual_tick_coverage(&rows, 42, &id_set(&[1])),
        43,
        "complete harmless tick overshoot is retained"
    );
}

#[test]
#[should_panic(expected = "before configured --run-ticks")]
fn selected_tick_coverage_rejects_a_log_that_ends_before_configuration() {
    let rows = rest_control_rows(41, 0.0);
    mc::assert_actual_tick_coverage(&rows, 42, &id_set(&[1]));
}

#[test]
#[should_panic(expected = "tick log is not contiguous: missing tick 2")]
fn selected_tick_coverage_rejects_a_missing_hazard_tick() {
    let rows = rest_control_rows(3, 0.0)
        .into_iter()
        .filter(|row| row.tick != 2)
        .collect::<Vec<_>>();
    mc::assert_actual_tick_coverage(&rows, 3, &id_set(&[1]));
}

fn terminal_control_rows() -> Vec<mc::PositionRow> {
    (1..=45)
        .map(|tick| mc::PositionRow {
            tick,
            unit_id: 1,
            x: if tick == 45 { 0.002 } else { 0.0 },
            y: 0.0,
        })
        .collect()
}

#[test]
fn rest_window_uses_checked_configured_endpoint_with_valid_overshoot() {
    mc::assert_stationary_and_disjoint_at(&terminal_control_rows(), &id_set(&[1]), 0.5, 40, 44);
}

#[test]
#[should_panic(expected = "moved more than 0.001 over the rest interval")]
fn rest_window_does_not_hide_motion_at_the_configured_endpoint() {
    mc::assert_stationary_and_disjoint_at(&terminal_control_rows(), &id_set(&[1]), 0.5, 40, 45);
}

#[test]
fn rest_window_accepts_forty_complete_stationary_intervals() {
    mc::assert_stationary_and_disjoint(&rest_control_rows(41, 0.0), &id_set(&[1]), 0.5, 40);
}

#[test]
#[should_panic(expected = "rest interval starts with an actual tick")]
fn rest_window_refuses_thirty_nine_intervals_as_two_seconds() {
    mc::assert_stationary_and_disjoint(&rest_control_rows(40, 0.0), &id_set(&[1]), 0.5, 40);
}

#[test]
#[should_panic(expected = "moved more than 0.001 over the rest interval")]
fn rest_window_refuses_small_per_tick_drift_over_the_whole_interval() {
    mc::assert_stationary_and_disjoint(&rest_control_rows(41, 0.0005), &id_set(&[1]), 0.5, 40);
}

fn script_dir(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("mc-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("MC script directory is created");
    path
}

fn run(
    label: &str,
    entries: Vec<(u32, u8, turts_net_protocol::OrderEnvelope)>,
    ticks: u32,
) -> (
    Vec<mc::PositionRow>,
    Vec<mc::OrderTransition>,
    std::path::PathBuf,
) {
    let selected: BTreeSet<_> = entries
        .iter()
        .flat_map(|(_, _, envelope)| envelope.units.iter().map(|id| id.0))
        .collect();
    let dir = script_dir(label);
    let script = dir.join("commands.json");
    let order_log = dir.join("orders.log");
    mc::write_script(&script, &entries);
    let server = mc::start_muster(
        &dir,
        &script,
        ticks,
        &["--order-log", order_log.to_str().unwrap()],
    );
    let (tick_log, order_path, process_dir) = mc::finish(server, &order_log, ticks);
    let positions = mc::read_positions(&tick_log);
    mc::assert_actual_tick_coverage(&positions, ticks, &selected);
    let transitions = mc::read_transitions(&order_path);
    (positions, transitions, process_dir)
}

#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn gm40_muster_forty_troopers_rest_disjoint_and_near_the_point() {
    let target = (30.0, 30.0);
    let ids = id_set(&TROOPERS);
    let rows = run(
        "gm40",
        vec![(
            1,
            0,
            mc::point_order(TROOPERS.to_vec(), target.0, target.1, 1),
        )],
        420,
    );
    let positions = &rows.0;
    let orders = &rows.1;
    mc::assert_full_tick_coverage(positions, 400, &ids);
    assert_eq!(
        mc::ids_at(positions, 1).intersection(&ids).count(),
        40,
        "the actual Muster map has exactly the selected forty at tick 1"
    );
    assert!(
        mc::blocked_ids(orders).is_disjoint(&ids),
        "GM40 selected troopers must not transition to Blocked: {:?}",
        mc::blocked_ids(orders)
            .intersection(&ids)
            .collect::<Vec<_>>()
    );
    mc::assert_rest_and_geometry_at(positions, &ids, target, 0.5, 4.58, 40, 420);
}

#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn gma_crossing_and_second_group_while_first_moves_leave_no_rest_overlap() {
    let first: Vec<_> = (1..=20).collect();
    let second: Vec<_> = (21..=40).collect();
    // First group travels north. The second order begins at tick 20 and travels east; phases
    // follow the actual initial geometry and 3 units/s troop speed, so their paths intersect.
    // Ticks 1 to 19 also walk the first group past the idle second group (a part of case I). This
    // part stays on purpose: two groups that pass at a right angle at full speed overlap each
    // pair for at most 1.8 units of relative path at 3 x sqrt(2) units/s, 0.42 s, so the red
    // build "avoidance off" can break the 0.5 s rule only where one group is slow or idle (P3 of
    // Task 22 went red at tick 12). The witness below checks the crossing with both groups moving.
    let entries = vec![
        (1, 0, mc::point_order(first.clone(), 29.0, 35.0, 1)),
        (20, 0, mc::point_order(second.clone(), 50.0, 22.0, 2)),
    ];
    let (positions, orders, _) = run("gma-crossing", entries, 600);
    let left = id_set(&first);
    let right = id_set(&second);
    let all = left.union(&right).copied().collect::<BTreeSet<_>>();
    mc::assert_full_tick_coverage(&positions, 400, &all);
    // Guard: the idle phase is the only part where "avoidance off" can go red (see above). A
    // fixture change that removes it must fail here, not hide the red path.
    assert!(
        mc::moving_units_between(&positions, &right, 1, 19).is_empty(),
        "GMA crossing guard: the second group must stay idle in ticks 1 to 19"
    );
    let crossing_ticks = mc::by_tick(&positions);
    assert!(
        (2..=19).any(|tick| moving_unit_near_group(
            crossing_ticks.get(&(tick - 1)).unwrap(),
            crossing_ticks.get(&tick).unwrap(),
            &left,
            &right,
            1.05
        )),
        "GMA crossing guard: a moving first-group unit must come within 1.05 of the idle second group in ticks 2 to 19"
    );
    assert_moving_transitions(&orders, &left, "GMA crossing first group");
    assert_moving_transitions(&orders, &right, "GMA crossing second group");
    let left_motion = mc::moving_units_between(&positions, &left, 1, 600);
    let right_motion = mc::moving_units_between(&positions, &right, 1, 600);
    assert!(
        !left_motion.is_empty(),
        "GMA crossing first group changed actual positions"
    );
    assert!(
        !right_motion.is_empty(),
        "GMA crossing second group changed actual positions"
    );
    assert_concurrent_crossing(&positions, &left, &right, 1.05, 600);
    assert_penetration_not_sustained(&positions, &left, &right, 0.5, 0.2, 10);
    mc::assert_stationary_and_disjoint_at(&positions, &all, 0.5, 40, 600);
    // F/I can expose reachable targets that finish Blocked. Record this known MC limit; do not
    // turn its count into a zero-blocked gate for GMA.
    let blocked = mc::blocked_ids(&orders);
    eprintln!(
        "GMA crossing blocked transition ids (recorded MC limitation): {:?}",
        blocked.intersection(&all).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn gma_second_group_passes_through_a_resting_group_and_both_rest_disjoint() {
    // The north rows (21..=40) settle first on open ground north of the start, so no idle
    // trooper stands between them and their slots (idle units do not move for others, 6.4). The
    // south rows (1..=20) then walk north through them.
    let first: Vec<_> = (21..=40).collect();
    let second: Vec<_> = (1..=20).collect();
    const SECOND_COMMAND_TICK: u32 = 240;
    const END_TICK: u32 = 700;
    let entries = vec![
        (1, 0, mc::point_order(first.clone(), 29.0, 35.0, 1)),
        (
            SECOND_COMMAND_TICK,
            0,
            mc::point_order(second.clone(), 29.0, 44.0, 2),
        ),
    ];
    let (positions, orders, _) = run("gma-resting-group", entries, END_TICK);
    let left = id_set(&first);
    let right = id_set(&second);
    let all = left.union(&right).copied().collect::<BTreeSet<_>>();
    mc::assert_full_tick_coverage(&positions, 400, &all);
    mc::assert_stationary_and_disjoint_at(&positions, &left, 0.5, 40, SECOND_COMMAND_TICK - 1);
    assert_moving_transitions(&orders, &right, "GMA I second group");
    assert!(
        !mc::moving_units_between(&positions, &right, SECOND_COMMAND_TICK, END_TICK).is_empty(),
        "GMA I second group has actual post-command position changes"
    );
    let ticks = mc::by_tick(&positions);
    let mut saw_second_cross_first_resting_formation = false;
    for tick in SECOND_COMMAND_TICK + 1..=SECOND_COMMAND_TICK + 200 {
        let at = ticks.get(&tick).expect("GMA tick is complete");
        let prior = ticks.get(&(tick - 1)).expect("GMA prior tick is complete");
        let first_resting_now = left
            .iter()
            .all(|id| mc::distance(*prior.get(id).unwrap(), *at.get(id).unwrap()) <= 0.001)
            && left.iter().all(|id| {
                mc::distance(
                    *ticks
                        .get(&(SECOND_COMMAND_TICK - 1))
                        .unwrap()
                        .get(id)
                        .unwrap(),
                    *at.get(id).unwrap(),
                ) <= 0.001
            });
        saw_second_cross_first_resting_formation |=
            first_resting_now && moving_unit_near_group(prior, at, &right, &left, 1.05);
    }
    assert!(
        saw_second_cross_first_resting_formation,
        "GMA I must observe a moving second-group unit within the actual 1.05 trooper separation reach of the first group's resting formation"
    );
    mc::assert_stationary_and_disjoint_at(&positions, &all, 0.5, 40, END_TICK);
    eprintln!(
        "GMA I blocked transition ids (recorded MC limitation): {:?}",
        mc::blocked_ids(&orders)
            .intersection(&all)
            .collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn gmu_clamps_finite_outside_target_to_the_inset_point() {
    // Clamp: one group of all forty to a finite point north of the map. The path north is clear
    // of every placement. The clamped point is the north edge inset by the 0.5 radius.
    let clamp_target = (29.0, 47.5);
    let (clamp_positions, clamp_orders, _) = run(
        "gmu-clamp",
        vec![(1, 0, mc::point_order(TROOPERS.to_vec(), 29.0, 55.0, 1))],
        600,
    );
    assert_group_terminal_by_deadline(
        &clamp_positions,
        &clamp_orders,
        &TROOPERS,
        1,
        clamp_target,
        3.0,
    );
    let clamp_ticks = mc::by_tick(&clamp_positions);
    let clamp_rest = clamp_ticks.get(&600).unwrap();
    mc::assert_no_overlap(clamp_rest, &id_set(&TROOPERS), 0.5, 600);
    assert_on_pool_lattice(clamp_rest, &TROOPERS, clamp_target, 1.1);
    assert!(
        TROOPERS
            .iter()
            .any(|id| clamp_rest.get(id) == Some(&clamp_target)),
        "GMU finite outside target is clamped to the authored inset point before group spreading"
    );
    for id in TROOPERS {
        let position = clamp_rest.get(&id).unwrap();
        assert!(
            (0.5..=63.5).contains(&position.0) && (0.5..=47.5).contains(&position.1),
            "GMU unit {id} ended outside the actual Muster map's trooper-inset bounds: {position:?}"
        );
    }
    eprintln!(
        "GMU clamp blocked transition ids (recorded, not gated): {:?}",
        mc::blocked_ids(&clamp_orders)
    );
}

/// Row GMU, same point (design revision 15, ruling O14). Troopers 21 to 40 move to (40, 30) and
/// rest with 0 `Blocked`. Then troopers 1 to 20 get a move of another group to the same point.
/// Gated: at rest no two footprint discs overlap; each unit of both orders, `Done` or `Blocked`,
/// stands within R_near = 4.58 of the point; each unit rests or is `Blocked` within
/// 2 x (distance / speed) + 5 s. Recorded, not gated: the `Blocked` count of the second order and
/// the rest position of each such unit (M4b owns this limit). The slot check of 6.3 step 2 is
/// the native test `crates/turts_sim/tests/gmu_same_point.rs`.
#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn gmu_second_order_to_the_same_point_rests_disjoint_within_r_near() {
    let first: Vec<_> = (21..=40).collect();
    let second: Vec<_> = (1..=20).collect();
    let target = (40.0, 30.0);
    const SECOND_ORDER_TICK: u32 = 300;
    const END_TICK: u32 = 700;
    let entries = vec![
        (1, 0, mc::point_order(first.clone(), target.0, target.1, 2)),
        (
            SECOND_ORDER_TICK,
            0,
            mc::point_order(second.clone(), target.0, target.1, 3),
        ),
    ];
    let (positions, orders, _) = run("gmu-same-point", entries, END_TICK);
    let left = id_set(&first);
    let right = id_set(&second);
    let all = left.union(&right).copied().collect::<BTreeSet<_>>();
    let blocked = mc::blocked_ids(&orders);
    assert!(
        blocked.is_disjoint(&left),
        "GMU same point: the first order (troopers 21 to 40) must rest with 0 Blocked: {:?}",
        blocked.intersection(&left).collect::<Vec<_>>()
    );
    assert_group_done_by_deadline(&positions, &orders, &first, 1, target, 3.0);
    assert_group_terminal_by_deadline(&positions, &orders, &second, SECOND_ORDER_TICK, target, 3.0);
    mc::assert_stationary_and_disjoint_at(&positions, &all, 0.5, 40, END_TICK);
    let ticks = mc::by_tick(&positions);
    let rest = ticks
        .get(&END_TICK)
        .expect("the configured end tick is present");
    for id in &all {
        let distance = mc::distance(*rest.get(id).unwrap(), target);
        assert!(
            distance <= 4.58,
            "GMU same point: unit {id} rests {distance} from {target:?}, beyond R_near = 4.58"
        );
    }
    record_blocked_units("open-ground", &orders, &right, rest, target, END_TICK);
}

/// Row GMU, same point at the clamped corner: recorded only; it gates rest and overlaps only.
/// Both orders go to (70, 55), clamped to (63.5, 47.5): troopers 1 to 20 at tick 1, troopers 21
/// to 40 at tick 300. The second order starts before the first order rests (RUN122: first-order
/// `Done` at ticks 288 to 412). Causes seen in RUN122, from the tick log at each `Blocked` tick:
/// first-order unit 18 touches the idle rows 21 to 40 on its route; second-order unit 22 touches
/// the idle slot-1 dummies at (54, 34) and (54, 35.2) on its route (idle units do not move for
/// others, 6.4); second-order units 40 and 39 touch resting first-order units (a slot behind a
/// resting group, 6.5). The authored weapons of the troopers also destroy slot-1 units in range
/// during the run (the post at (61, 44) after tick 355). `docs/measurements/mc.md` records each.
#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn gmu_second_order_to_the_clamped_corner_is_recorded_and_rests_disjoint() {
    let first: Vec<_> = (1..=20).collect();
    let second: Vec<_> = (21..=40).collect();
    let clamped = (63.5, 47.5);
    const END_TICK: u32 = 1300;
    let entries = vec![
        (1, 0, mc::point_order(first.clone(), 70.0, 55.0, 1)),
        (300, 0, mc::point_order(second.clone(), 70.0, 55.0, 2)),
    ];
    let (positions, orders, _) = run("gmu-corner", entries, END_TICK);
    let left = id_set(&first);
    let right = id_set(&second);
    let all = left.union(&right).copied().collect::<BTreeSet<_>>();
    let ticks = mc::by_tick(&positions);
    let rest = ticks
        .get(&END_TICK)
        .expect("the configured end tick is present");
    mc::assert_stationary_and_disjoint_at(&positions, &all, 0.5, 40, END_TICK);
    record_blocked_units(
        "corner-first-order",
        &orders,
        &left,
        rest,
        clamped,
        END_TICK,
    );
    record_blocked_units("corner", &orders, &right, rest, clamped, END_TICK);
}

/// Print the recorded MC limit of row GMU in a form that `docs/measurements/mc.md` copies: one
/// count line, then one line for each `Blocked` unit with its rest position.
fn record_blocked_units(
    case: &str,
    transitions: &[mc::OrderTransition],
    ids: &BTreeSet<u32>,
    rest: &BTreeMap<u32, (f32, f32)>,
    point: (f32, f32),
    end_tick: u32,
) {
    let blocked: Vec<_> = transitions
        .iter()
        .filter(|row| row.state == "Blocked" && ids.contains(&row.unit_id))
        .collect();
    println!(
        "GMU record case={case} point=({}, {}) units={} blocked_count={} end_tick={end_tick}",
        point.0,
        point.1,
        ids.len(),
        blocked
            .iter()
            .map(|row| row.unit_id)
            .collect::<BTreeSet<_>>()
            .len()
    );
    for row in blocked {
        let position = *rest.get(&row.unit_id).unwrap();
        println!(
            "GMU record case={case} blocked unit_id={} blocked_tick={} rest_x={} rest_y={} distance_to_point={:.3}",
            row.unit_id,
            row.tick,
            position.0,
            position.1,
            mc::distance(position, point)
        );
    }
}

#[test]
#[ignore = "MC Task22 process acceptance; run under the assigned compiler lease"]
fn gmu_post_target_ends_on_a_ring_round_the_post() {
    let post_entries = vec![(1, 0, mc::point_order(TROOPERS.to_vec(), 21.0, 20.0, 3))];
    let (post_positions, post_orders, _) = run("gmu-post-ring", post_entries, 500);
    assert!(
        mc::blocked_ids(&post_orders).is_disjoint(&id_set(&TROOPERS)),
        "GMU post-ring orders must not fail"
    );
    assert_group_done_by_deadline(
        &post_positions,
        &post_orders,
        &TROOPERS,
        1,
        (21.0, 20.0),
        3.0,
    );
    let post_ticks = mc::by_tick(&post_positions);
    let end_tick = 500;
    for id in TROOPERS {
        let point = *post_ticks.get(&end_tick).unwrap().get(&id).unwrap();
        let distance = mc::distance(point, (21.0, 20.0));
        assert!(
            distance >= 2.5 - 0.001,
            "trooper {id} overlaps the actual radius-2 post"
        );
        assert!(
            distance.is_finite(),
            "trooper {id} has a non-finite post-ring position"
        );
    }
}

/// Every slot of a pool is a point of the hex lattice round the pool origin (design 6.3 step 3),
/// so units at rest on primary or spare slots lie on the lattice of the clamped point.
fn assert_on_pool_lattice(
    rest: &BTreeMap<u32, (f32, f32)>,
    ids: &[u32],
    origin: (f32, f32),
    spacing: f32,
) {
    let pitch = spacing * 3.0_f32.sqrt() * 0.5;
    for id in ids {
        let (x, y) = *rest.get(id).unwrap();
        let j = ((y - origin.1) / pitch).round();
        let i = (x - origin.0) / spacing - 0.5 * j;
        assert!(
            (j * pitch - (y - origin.1)).abs() <= 0.001 && (i - i.round()).abs() * spacing <= 0.001,
            "GMU unit {id} at ({x}, {y}) is not on the slot lattice of the clamped point {origin:?}"
        );
    }
}

fn assert_group_terminal_by_deadline(
    positions: &[mc::PositionRow],
    transitions: &[mc::OrderTransition],
    ids: &[u32],
    command_tick: u32,
    target: (f32, f32),
    speed: f32,
) {
    assert_group_state_by_deadline(
        positions,
        transitions,
        ids,
        command_tick,
        target,
        speed,
        &["Done", "Blocked"],
    );
}

fn assert_group_done_by_deadline(
    positions: &[mc::PositionRow],
    transitions: &[mc::OrderTransition],
    ids: &[u32],
    command_tick: u32,
    target: (f32, f32),
    speed: f32,
) {
    assert_group_state_by_deadline(
        positions,
        transitions,
        ids,
        command_tick,
        target,
        speed,
        &["Done"],
    );
}

fn assert_group_state_by_deadline(
    positions: &[mc::PositionRow],
    transitions: &[mc::OrderTransition],
    ids: &[u32],
    command_tick: u32,
    target: (f32, f32),
    speed: f32,
    states: &[&str],
) {
    let initial = mc::by_tick(positions);
    let start = initial.get(&1).expect("the actual initial tick is present");
    for id in ids {
        let position = *start
            .get(id)
            .unwrap_or_else(|| panic!("initial tick omits unit {id}"));
        let travel_seconds = mc::distance(position, target) / speed;
        let deadline = command_tick + (2.0 * travel_seconds * 20.0).ceil() as u32 + 100;
        let done_tick = transitions
            .iter()
            .filter(|row| {
                row.unit_id == *id
                    && states.contains(&row.state.as_str())
                    && row.tick >= command_tick
            })
            .map(|row| row.tick)
            .min()
            .unwrap_or_else(|| {
                panic!("GMU unit {id} has no actual terminal {states:?} transition")
            });
        assert!(
            done_tick <= deadline,
            "GMU unit {id} reached {states:?} at tick {done_tick}, after its design deadline {deadline}"
        );
    }
}

#[test]
#[ignore = "MC Task22 typed-seam acceptance; run under the assigned compiler lease"]
fn gmu_refuses_nonfinite_point_at_the_real_typed_order_validation_seam() {
    use std::path::PathBuf;
    use turts_mod_api::SourceDef;
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
    use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId};
    use turts_sim::{SlotId, apply_command, headless_app, install_package, stage_package};

    let package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive = PackageArchive::parse(&pack_dir(&package_dir).expect("Muster package packs"))
        .expect("Muster archive parses");
    let catalog = archive.content().expect("Muster catalog resolves");
    let (map_file, map) = archive
        .maps()
        .expect("Muster map resolves")
        .into_iter()
        .next()
        .expect("Muster exports the field map");
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
    .expect("the actual Muster map stages");
    install_package(app.world_mut(), staged);

    // serde_json cannot represent NaN/Inf in the command-script JSON. Exercise the real typed
    // session resolver and simulation validator directly, preserving the protocol reason.
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let command = PlayerCommand {
            seq: 1,
            order: OrderEnvelope {
                order: "muster:order_move".into(),
                units: vec![UnitId(1)],
                target: OrderTarget::Point(Pos { x: bad, y: 20.0 }),
                queue: QueueMode::Replace,
                group: 3,
            },
        };
        let resolved = server_orders::to_sim(SlotId(0), &command, &catalog)
            .expect("the actual typed order resolver reaches simulation target validation");
        let result = apply_command(app.world_mut(), &resolved[0]);
        assert!(
            result
                .units
                .refused
                .iter()
                .any(|(unit, reason)| *unit == 1
                    && *reason == turts_sim::RejectReason::TargetNotFinite),
            "nonfinite point must be refused as TargetNotFinite before clamp: {bad:?}"
        );
        let wire = server_orders::to_wire(&[result], &command);
        assert!(
            wire.refused.iter().any(|refusal| refusal.unit == UnitId(1)
                && refusal.reason == turts_net_protocol::CommandRejectReason::TargetNotFinite),
            "nonfinite refusal remains target_not_finite on the actual protocol result: {bad:?}"
        );
    }
}

fn assert_penetration_not_sustained(
    rows: &[mc::PositionRow],
    left: &BTreeSet<u32>,
    right: &BTreeSet<u32>,
    radius: f32,
    penetration_fraction: f32,
    duration_ticks: u32,
) {
    let ticks = mc::by_tick(rows);
    let threshold = 2.0 * radius - penetration_fraction * radius;
    let mut consecutive: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for (tick, positions) in ticks {
        let mut currently_deep = BTreeSet::new();
        for a in left {
            for b in right {
                let separation =
                    mc::distance(*positions.get(a).unwrap(), *positions.get(b).unwrap());
                if separation < threshold {
                    let count = consecutive.entry((*a, *b)).or_default();
                    *count += 1;
                    currently_deep.insert((*a, *b));
                    assert!(
                        *count <= duration_ticks,
                        "groups {a}/{b} penetrate by more than 20% radius for >0.5 s at tick {tick}"
                    );
                }
            }
        }
        consecutive.retain(|pair, _| currently_deep.contains(pair));
    }
}

fn assert_moving_transitions(
    transitions: &[mc::OrderTransition],
    ids: &BTreeSet<u32>,
    label: &str,
) {
    let moving: BTreeSet<_> = transitions
        .iter()
        .filter(|transition| transition.state == "Moving")
        .map(|transition| transition.unit_id)
        .collect();
    assert!(
        ids.is_subset(&moving),
        "{label} lacks an actual Moving transition for units {:?}",
        ids.difference(&moving).collect::<Vec<_>>()
    );
}

fn assert_concurrent_crossing(
    rows: &[mc::PositionRow],
    left: &BTreeSet<u32>,
    right: &BTreeSet<u32>,
    reach: f32,
    end_tick: u32,
) {
    let ticks = mc::by_tick(rows);
    for tick in 2..=end_tick {
        let prior = ticks
            .get(&(tick - 1))
            .expect("crossing prior tick is complete");
        let current = ticks.get(&tick).expect("crossing tick is complete");
        if concurrent_moving_pair_near(prior, current, left, right, reach) {
            return;
        }
    }
    panic!("GMA crossing has no tick with both groups moving and an actual pair within {reach}");
}

fn moving_unit_near_group(
    prior: &BTreeMap<u32, (f32, f32)>,
    current: &BTreeMap<u32, (f32, f32)>,
    moving: &BTreeSet<u32>,
    group: &BTreeSet<u32>,
    reach: f32,
) -> bool {
    moving.iter().any(|id| {
        mc::distance(*prior.get(id).unwrap(), *current.get(id).unwrap()) > 0.001
            && group.iter().any(|other| {
                mc::distance(*current.get(id).unwrap(), *current.get(other).unwrap()) <= reach
            })
    })
}

fn concurrent_moving_pair_near(
    prior: &BTreeMap<u32, (f32, f32)>,
    current: &BTreeMap<u32, (f32, f32)>,
    left: &BTreeSet<u32>,
    right: &BTreeSet<u32>,
    reach: f32,
) -> bool {
    left.iter().any(|a| {
        mc::distance(*prior.get(a).unwrap(), *current.get(a).unwrap()) > 0.001
            && right.iter().any(|b| {
                mc::distance(*prior.get(b).unwrap(), *current.get(b).unwrap()) > 0.001
                    && mc::distance(*current.get(a).unwrap(), *current.get(b).unwrap()) <= reach
            })
    })
}

#[test]
fn crossing_witness_rejects_stationary_neighbours_and_unrelated_movers() {
    let prior = BTreeMap::from([
        (1, (0.0, 0.0)),
        (2, (50.0, 50.0)),
        (3, (0.5, 0.0)),
        (4, (70.0, 70.0)),
    ]);
    let current = BTreeMap::from([
        (1, (0.0, 0.0)),
        (2, (51.0, 50.0)),
        (3, (0.5, 0.0)),
        (4, (71.0, 70.0)),
    ]);
    assert!(!concurrent_moving_pair_near(
        &prior,
        &current,
        &id_set(&[1, 2]),
        &id_set(&[3, 4]),
        1.05
    ));
    assert!(!moving_unit_near_group(
        &prior,
        &current,
        &id_set(&[3, 4]),
        &id_set(&[1]),
        1.05
    ));
}

#[test]
fn crossing_witness_accepts_the_actual_moving_pair_in_reach() {
    let prior = BTreeMap::from([(1, (0.0, 0.0)), (2, (1.0, 0.0))]);
    let current = BTreeMap::from([(1, (0.1, 0.0)), (2, (0.9, 0.0))]);
    assert!(concurrent_moving_pair_near(
        &prior,
        &current,
        &id_set(&[1]),
        &id_set(&[2]),
        1.05
    ));
    assert!(moving_unit_near_group(
        &prior,
        &current,
        &id_set(&[2]),
        &id_set(&[1]),
        1.05
    ));
}
