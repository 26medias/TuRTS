//! UT7: bounded work. An isolated measurement of the utility pass; it is not an M7 claim.
//!
//! The ignored test `ut7_utility_pass_at_the_slot_limits` runs the real pass (`run_pass`) in a
//! real world with `SLOT_COUNT` slots at the limits (512 junctions and 512 segments each) and
//! edits 16 cells under lines in every tick for 2,000 ticks. It then runs a synthetic harness
//! of the same pass functions (`exposure::run_exposure`, `network::run_networks`) over 8
//! slots. It writes p50, p99 and max pass time and the view size to the JSON path in the
//! environment variable `TURTS_UT7_OUT`. Budgets (spec 11, UT7): p99 at most 2 ms; one view at
//! most 64 KiB encoded. A miss is recorded in the JSON, not hidden.
//!
//! Run: `TURTS_UT7_OUT=/path/ut7.json cargo test --release -p turts_sim --lib ut7 -- --ignored
//! --nocapture`.

use super::tests::{Opts, SURFACE, Yard, cid};
use super::*;
use std::time::Instant;
use turts_mod_api::UtilityExposure;
use turts_rules::terrain::{
    Ratio,
    geometry::Cell,
    material::{Prism, StepTerms},
};

const TICKS: usize = 2_000;
const EDITS_PER_TICK: usize = 16;
const SLOT_LIMIT: usize = 512;
const SIZE: u32 = 64;
const BUDGET_P99_NS: u128 = 2_000_000;
const BUDGET_VIEW_BYTES: usize = 64 * 1024;

/// 512 junctions in 16 rows of 32, and 512 segments: 496 along the rows and 16 across. The
/// first segments start at the given port units.
fn limit_topology(ports: &[u32], next_node: u32, next_segment: u32) -> SlotTopology {
    let utility = cid("power_line");
    let mut topology = SlotTopology {
        next_node,
        next_segment,
        ..SlotTopology::default()
    };
    let mut ids = Vec::new();
    for index in 0..SLOT_LIMIT {
        let id = topology.next_node;
        topology.next_node += 1;
        let pos = (
            1.5 + (index % 32) as f32 * 1.8,
            1.5 + (index / 32) as f32 * 3.5,
        );
        topology.nodes.insert(
            id,
            Node {
                utility: utility.clone(),
                pos,
            },
        );
        ids.push(id);
    }
    let mut pairs = Vec::new();
    for row in 0..16 {
        for column in 0..31 {
            pairs.push((row * 32 + column, row * 32 + column + 1));
        }
    }
    for column in 0..16 {
        pairs.push((column, column + 32));
    }
    assert_eq!(pairs.len(), SLOT_LIMIT);
    for (index, (a, b)) in pairs.into_iter().enumerate() {
        let (a_id, b_id) = (ids[a], ids[b]);
        let (a_pos, b_pos) = (topology.nodes[&a_id].pos, topology.nodes[&b_id].pos);
        let a_end = ports
            .get(index)
            .map_or(End::Node(a_id), |unit| End::Port(*unit));
        let id = topology.next_segment;
        topology.next_segment += 1;
        topology.segments.insert(
            id,
            Segment {
                utility: utility.clone(),
                a: a_end,
                b: End::Node(b_id),
                a_pos,
                b_pos,
                elevation: SURFACE - 2,
                depth: 2,
                state: SegmentState::Conducting,
            },
        );
    }
    topology
}

fn cells_under_lines(topology: &SlotTopology, geom: GridGeom) -> Vec<(u32, u32)> {
    let mut cells = BTreeSet::new();
    for segment in topology.segments.values() {
        cells.extend(geom.cells_of(segment.a_pos, segment.b_pos));
    }
    cells.into_iter().collect()
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
struct Timing {
    ticks: usize,
    p50_ns: u128,
    p99_ns: u128,
    max_ns: u128,
    mean_ns: u128,
    checked_segments: u64,
}

fn summarize(mut samples: Vec<u128>, checked_segments: u64) -> Timing {
    samples.sort_unstable();
    let at = |per_mille: usize| samples[(samples.len() * per_mille / 1000).min(samples.len() - 1)];
    Timing {
        ticks: samples.len(),
        p50_ns: at(500),
        p99_ns: at(990),
        max_ns: *samples.last().unwrap(),
        mean_ns: samples.iter().sum::<u128>() / samples.len() as u128,
        checked_segments,
    }
}

/// Dig the cell by 3 steps, or fill 3 steps. It uses the prepared edit and the
/// `apply_prevalidated` commit of ordinary terrain work.
fn edit_cell(world: &mut bevy_ecs::world::World, cell: (u32, u32), dig: bool) {
    let mut terrain = world.resource_mut::<crate::terrain::TerrainState>();
    let at = Cell {
        x: cell.0,
        y: cell.1,
    };
    let index = terrain.index(at).unwrap();
    let mut column = terrain.columns[index].clone();
    let prism: Prism = terrain.prism;
    let terms = StepTerms {
        material: Ratio {
            numerator: 0,
            denominator: 1,
        },
        work: Ratio {
            numerator: 1000,
            denominator: 1,
        },
    };
    let step = if dig {
        column.stage_remove_many(prism, terms, u64::MAX, 3).unwrap()
    } else {
        let key = terrain.key(&cid("stone")).unwrap();
        column
            .stage_fill_many(key, prism, terms, u64::MAX, 3)
            .unwrap()
    };
    column.commit(step).unwrap();
    let edit = terrain.prepare_cell(at, column).unwrap();
    terrain.apply_prevalidated(edit);
}

/// Dig during the first half of each period of ticks, fill during the second half.
fn digging(tick: usize, cell_count: usize) -> bool {
    (tick / (cell_count / EDITS_PER_TICK).max(1)).is_multiple_of(2)
}

fn real_world_timing() -> (Timing, usize) {
    let mut opts = Opts::new(UtilityExposure::Disconnect);
    opts.size = SIZE;
    let mut yard = Yard::build(opts);
    yard.ticks(2);
    let geom = grid_geometry(yard.app.world()).unwrap();
    let ports = vec![yard.generator(), yard.workshop()];
    for slot in 0..crate::slots::SLOT_COUNT {
        let world = yard.app.world_mut();
        let base = world.resource::<UtilityState>().counters(slot);
        let topology = limit_topology(
            if slot == 0 { &ports } else { &[] },
            base.next_node,
            base.next_segment,
        );
        apply_write(
            world,
            UtilityWrite {
                slot,
                utility: cid("power_line"),
                base,
                topology,
                made: Vec::new(),
            },
        );
    }
    let cells = cells_under_lines(yard.state().topology(0).unwrap(), geom);
    assert!(cells.len() >= EDITS_PER_TICK * 2);
    let view_bytes = {
        let view = yard.view();
        serde_json::to_vec(&owner_view(&view, 0)).unwrap().len()
    };
    // The first pass settles the seeded topology; it is not measured.
    run_pass(yard.app.world_mut());
    begin_tick(yard.app.world_mut());
    let before = privileged_checked_segments(yard.app.world());
    let mut samples = Vec::with_capacity(TICKS);
    for tick in 0..TICKS {
        let world = yard.app.world_mut();
        world.resource_mut::<crate::Tick>().0 += 1;
        crate::terrain::begin_tick(world);
        for edit in 0..EDITS_PER_TICK {
            let cell = cells[(tick * EDITS_PER_TICK + edit) % cells.len()];
            edit_cell(world, cell, digging(tick, cells.len()));
        }
        begin_tick(world);
        let started = Instant::now();
        run_pass(world);
        samples.push(started.elapsed().as_nanos());
    }
    let checked = privileged_checked_segments(yard.app.world()) - before;
    (summarize(samples, checked), view_bytes)
}

/// The same pass functions over 8 slots, with no world: a synthetic terrain grid and a live
/// port set stand in for the world reads.
fn synthetic_timing(slots: u8) -> (Timing, usize) {
    let geom = GridGeom {
        cell_width: 1.0,
        columns: SIZE,
        rows: SIZE,
    };
    let utility = cid("power_line");
    let mut state = UtilityState::default();
    let mut live = network::LivePorts::new();
    let mut cells = Vec::new();
    for slot in 0..slots {
        let ports: Vec<u32> = (0..32).map(|i| 1000 * (u32::from(slot) + 1) + i).collect();
        for unit in &ports {
            live.insert(
                *unit,
                vec![network::PortUnit {
                    slot,
                    utility: utility.clone(),
                    provided: u64::from(unit % 2) * 10,
                    consumed: u64::from(unit % 3) * 6,
                }],
            );
        }
        let topology = limit_topology(&ports, 1, 1);
        if cells.is_empty() {
            cells = cells_under_lines(&topology, geom);
        }
        for (id, segment) in &topology.segments {
            state.index_add(slot, *id, &geom.cells_of(segment.a_pos, segment.b_pos));
        }
        *state.topology_mut(slot) = topology;
        state.mark_dirty(slot, &utility);
    }
    let mut surface = vec![SURFACE; (SIZE * SIZE) as usize];
    let policy = |_: &ContentId| Some(UtilityExposure::Disconnect);
    let resource = |_: &ContentId| Some(cid("power"));
    // Settle the seeded state; it is not measured.
    let none = BTreeSet::new();
    exposure::run_exposure(
        &mut state,
        &exposure::ExposureInput {
            geom: Some(geom),
            changed_cells: &none,
            surface: &|_, _| Some(SURFACE),
            policy: &policy,
            tick: 0,
        },
    );
    network::run_networks(&mut state, &live, &resource);
    let before = state.checked_segments;
    let mut samples = Vec::with_capacity(TICKS);
    for tick in 0..TICKS {
        let mut changed = BTreeSet::new();
        for edit in 0..EDITS_PER_TICK {
            let cell = cells[(tick * EDITS_PER_TICK + edit) % cells.len()];
            let level = if digging(tick, cells.len()) {
                SURFACE - 3
            } else {
                SURFACE
            };
            surface[(cell.1 * SIZE + cell.0) as usize] = level;
            changed.insert(cell);
        }
        let started = Instant::now();
        exposure::run_exposure(
            &mut state,
            &exposure::ExposureInput {
                geom: Some(geom),
                changed_cells: &changed,
                surface: &|x, y| Some(surface[(y * SIZE + x) as usize]),
                policy: &policy,
                tick: tick as u32,
            },
        );
        network::run_networks(&mut state, &live, &resource);
        state.swap_effective(tick as u32);
        samples.push(started.elapsed().as_nanos());
    }
    let view = WorldView {
        utilities: Some(state.snap.clone()),
        ..WorldView::default()
    };
    let bytes = serde_json::to_vec(&owner_view(&view, 0)).unwrap().len();
    (summarize(samples, state.checked_segments - before), bytes)
}

#[test]
#[ignore = "UT7 measurement: run in release under the exclusive lock"]
fn ut7_utility_pass_at_the_slot_limits() {
    let (real, real_view) = real_world_timing();
    let (synthetic, synthetic_view) = synthetic_timing(8);
    let report = serde_json::json!({
        "ticks": TICKS,
        "edited_cells_per_tick": EDITS_PER_TICK,
        "limits": { "junctions_per_slot": SLOT_LIMIT, "segments_per_slot": SLOT_LIMIT },
        "real_pass_slot_count_slots": {
            "slots": crate::slots::SLOT_COUNT,
            "pass_time": real,
            "view_json_bytes_one_slot": real_view,
        },
        "synthetic_8_slots": {
            "slots": 8,
            "pass_time": synthetic,
            "view_json_bytes_one_slot": synthetic_view,
        },
        "budget": {
            "pass_p99_ns": BUDGET_P99_NS,
            "view_bytes": BUDGET_VIEW_BYTES,
            "real_p99_within_budget": real.p99_ns <= BUDGET_P99_NS,
            "synthetic_p99_within_budget": synthetic.p99_ns <= BUDGET_P99_NS,
            "view_within_budget": real_view <= BUDGET_VIEW_BYTES,
            "note": "view bytes are the JSON size of the sim-side owner rows, not the postcard wire size",
        },
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
    });
    let text = serde_json::to_string_pretty(&report).unwrap();
    println!("{text}");
    if let Ok(path) = std::env::var("TURTS_UT7_OUT") {
        std::fs::write(path, text).unwrap();
    }
    // The measurement ran real work: every tick checked the segments under its edited cells.
    assert!(real.checked_segments >= TICKS as u64);
    assert!(synthetic.checked_segments >= TICKS as u64);
}
