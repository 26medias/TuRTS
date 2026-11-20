//! Tests of the actual-work recovery bound (design `2026-11-19-recovery-actual-work`).

use super::frame::{MovementFrame, MovementFrameOutput, MovementFrameUnit};
use super::recovery::{
    Check, EscapeScan, MIN_SLICE, OWNER_WORK, PoolSearch, RecoveryCounters, RecoveryWorld,
    RowRecovery, SLICE_CANDIDATES, ScheduleReads, ScheduleRow, SliceOutcome, plan,
};
use super::recovery_oracle::take_compared;
use super::recovery_view::{ClaimSet, Contacts, KnownSet, KnownView, body_key};
use super::search::MovementSearch;
use super::{
    MovementVisibility, PhysicalEscape, RecoveryKind, RecoveryScan, RecoverySearch, RecoveryStage,
};
use bevy_math::Vec2;
use std::collections::{BTreeMap, BTreeSet};

const ORIGIN: Vec2 = Vec2::new(10.0, 10.0);

/// Manual wall-time, work and memory diagnostic; run only with the exclusive performance
/// lock. `DENSITY_RADIUS=tiny` (0.001, else 0.4), `DENSITY_SIDE` (71), `DENSITY_KIND`
/// (`spare`: stalled movers, the default; `blocked`: blocked group movers),
/// `DENSITY_FOG` (`none`, the default; `visible`: every other still body is of owner 1 and
/// permitted; `hidden`: the same bodies are hidden, so they are contacts of the rows).
#[test]
#[ignore = "manual release workload"]
fn frame_contact_density_probe() {
    let tiny = std::env::var("DENSITY_RADIUS").as_deref() == Ok("tiny");
    let blocked_kind = std::env::var("DENSITY_KIND").as_deref() == Ok("blocked");
    let fog = std::env::var("DENSITY_FOG").unwrap_or_else(|_| "none".into());
    let side: usize = std::env::var("DENSITY_SIDE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(71);
    let radius = if tiny { 0.001_f32 } else { 0.4_f32 };
    let spacing = radius * 2.1;
    let upper = 2.0 + side as f32 * spacing;
    let slots: Vec<_> = (0..100)
        .map(|i| {
            let index = side * side - 100 + i;
            Vec2::new(
                1.0 + (index % side) as f32 * spacing,
                1.0 + (index / side) as f32 * spacing,
            )
        })
        .collect();
    let base = slots[50];
    let mut units = Vec::with_capacity(side * side);
    let mut permitted = BTreeSet::new();
    for i in 0..side * side {
        let position = Vec2::new(
            1.0 + (i % side) as f32 * spacing,
            1.0 + (i / side) as f32 * spacing,
        );
        let mut unit = if let Some(&slot) = slots.get(i) {
            let mut mover = blocked(i as u32 + 1, position, slot);
            if !blocked_kind {
                let progress = mover.progress.as_mut().unwrap();
                progress.activity = super::MovementActivity::Moving;
                progress.claim_count = 0;
                progress.best_distance = 0.0;
            }
            mover
        } else {
            let owner = u8::from(fog != "none" && i % 2 == 1);
            if owner == 1 && fog == "visible" {
                permitted.insert((0, i as u32 + 1));
            }
            still(i as u32 + 1, owner, position, radius)
        };
        unit.radius = radius;
        units.push(unit);
    }
    let basis = Basis { permitted };
    let bounds = Vec2::splat(upper + 10.0 * spacing + 2.0);
    let frame = MovementFrame {
        bounds,
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(
            7,
            super::MovementPool {
                pool_id: 7,
                point: base,
                spacing,
                primary_count: 100,
                slots,
            },
        )]),
        recovery: BTreeMap::new(),
    };
    let mut times = Vec::new();
    for sample in 0..6 {
        let mut budget = super::sweep::SweepBudget::new(1);
        let start = std::time::Instant::now();
        let outputs = super::frame::step_frame_with_motion_guide_and_visibility(
            &frame,
            0.05,
            &Default::default(),
            &mut budget,
            &mut super::motion::NoMotionGuide,
            (fog != "none").then_some(&basis as &dyn MovementVisibility),
        );
        let millis = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(outputs.len(), side * side);
        let c = &budget.recovery;
        assert!(c.physical_reads >= 100);
        eprintln!(
            "DENSITY tiny={tiny} kind={} fog={fog} bodies={} movers=100 sample={sample} ms={millis:.3} owner={} physical_reads={} physical_work={} view_work={} frame_work={} phase_bytes={} contact_reads={} contact_work={}",
            if blocked_kind { "blocked" } else { "spare" },
            side * side,
            c.owner_work[0],
            c.physical_reads,
            c.physical_work,
            c.view_work,
            c.frame_work,
            c.phase_bytes,
            c.contact_reads,
            c.contact_work
        );
        if sample > 0 {
            times.push(millis);
        }
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "DENSITY_SUMMARY tiny={tiny} kind={} fog={fog} bodies={} median_ms={:.3} max_ms={:.3}",
        if blocked_kind { "blocked" } else { "spare" },
        side * side,
        times[times.len() / 2],
        times[times.len() - 1]
    );
}

fn body(unit_id: u32, owner: u8, position: Vec2, radius: f32) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id,
        owner_slot: owner,
        unit_type: 1,
        position,
        radius,
        speed: Some(1.0),
        target: None,
        progress: None,
        plain_progress: None,
        physical_escape: None,
    }
}

fn still(unit_id: u32, owner: u8, position: Vec2, radius: f32) -> MovementFrameUnit {
    MovementFrameUnit {
        speed: None,
        ..body(unit_id, owner, position, radius)
    }
}

/// A body escape after a failed order. Fast, so a test reaches the end of its escape.
fn escaping(unit_id: u32, owner: u8, position: Vec2, radius: f32) -> MovementFrameUnit {
    MovementFrameUnit {
        physical_escape: Some(PhysicalEscape { target: None }),
        speed: Some(10.0),
        ..body(unit_id, owner, position, radius)
    }
}

/// A stateless search world for one mover (row 0) with its known view.
struct World {
    bounds: Vec2,
    units: Vec<MovementFrameUnit>,
    points: Vec<crate::grid::GridPoint>,
    grid: crate::grid::UniformGrid,
    max_radius: f32,
}

impl World {
    fn new(bounds: Vec2, units: Vec<MovementFrameUnit>) -> Self {
        let points: Vec<_> = units
            .iter()
            .map(|unit| crate::grid::GridPoint {
                id: unit.unit_id,
                position: unit.position,
            })
            .collect();
        let max_radius = units.iter().map(|unit| unit.radius).fold(0.0, f32::max);
        let grid = crate::grid::UniformGrid::new(&points, 2.0 * max_radius);
        Self {
            bounds,
            units,
            points,
            grid,
            max_radius,
        }
    }

    fn view(&self, known: &dyn Fn(u32) -> bool) -> KnownView {
        let members: Vec<_> = self
            .units
            .iter()
            .enumerate()
            .filter(|(row, unit)| *row == 0 || known(unit.unit_id))
            .map(|(row, unit)| (row, unit.position, unit.radius))
            .collect();
        KnownView::build(&members, self.units.len())
    }

    fn slice(
        &self,
        view: &KnownView,
        previous: Option<RecoveryScan>,
        allowance: usize,
        counters: &mut RecoveryCounters,
    ) -> SliceOutcome {
        let mover = &self.units[0];
        let contacts = Contacts::default();
        let known = KnownSet {
            view,
            contacts: &contacts,
            own_key: Some(body_key(mover.position, mover.radius)),
            ignore_soft: false,
        };
        EscapeScan {
            bounds: self.bounds,
            origin: mover.position,
            radius: mover.radius,
            slots: None,
            pool: None,
            bound: None,
            verify: true,
            pool_only: false,
        }
        .run(&known, previous, allowance, 0, counters)
    }

    fn oracle(&self, known: &dyn Fn(u32) -> bool) -> Option<Vec2> {
        MovementSearch {
            bounds: self.bounds,
            units: &self.units,
            points: &self.points,
            grid: &self.grid,
            max_radius: self.max_radius,
        }
        .find_free_singleton_destination_known(
            self.units[0].position,
            self.units[0].radius,
            0,
            known,
        )
    }
}

/// The result of a search sliced with `allowance` over one world, the slices, the peak
/// slice work and whether a slice paused inside a point query.
fn slice_to_end(
    world: &World,
    known: &dyn Fn(u32) -> bool,
    allowance: usize,
) -> (Option<Vec2>, u64, usize, bool, RecoveryCounters) {
    let view = world.view(known);
    let mut counters = RecoveryCounters::default();
    let mut scan = None;
    let mut peak = 0;
    let mut inside = false;
    for _ in 0..1_000_000 {
        let before = counters.owner_actual[0];
        let outcome = world.slice(&view, scan, allowance, &mut counters);
        peak = peak.max(counters.owner_actual[0] - before);
        match outcome {
            SliceOutcome::Done(point) => {
                return (point, counters.slices, peak, inside, counters);
            }
            SliceOutcome::Paused(state) => {
                inside |= state.query.is_some_and(|query| query.started);
                scan = Some(state);
            }
        }
    }
    panic!("the search must end");
}

/// The 40,000 tiny bodies of the handoff counterexample (probe commit `5178aa1c`).
fn counterexample_bodies(owner: u8, first_id: u32) -> Vec<MovementFrameUnit> {
    (0..40_000)
        .map(|n| {
            let offset = Vec2::new((n % 200) as f32 * 0.0022, (n / 200) as f32 * 0.0022);
            still(
                first_id + n,
                owner,
                ORIGIN + Vec2::new(0.88, -0.22) + offset,
                0.001,
            )
        })
        .collect()
}

/// R1. The handoff counterexample with every tiny body known: each slice stays within its
/// allowance, and the sliced search ends with the stateless result. The base used 219,852
/// operations in one 32-unit slice of this scene.
#[test]
fn counterexample_with_known_bodies_stays_within_each_slice() {
    let mut units = vec![body(1, 0, ORIGIN, 0.5)];
    units.extend(counterexample_bodies(0, 2));
    let world = World::new(Vec2::splat(20.0), units);
    let known = |_: u32| true;
    for allowance in [MIN_SLICE, 1_024, OWNER_WORK] {
        let (point, slices, peak, _, counters) = slice_to_end(&world, &known, allowance);
        eprintln!(
            "KNOWN40000 allowance={allowance} slices={slices} peak_slice={peak} bodies={} cells={}",
            counters.bodies, counters.cells
        );
        assert!(peak <= allowance, "{peak} > {allowance}");
        assert_eq!(point, world.oracle(&known), "allowance {allowance}");
    }
}

/// R1. The handoff counterexample with every tiny body hidden: a slice reads only the
/// mover, so its work equals the work of the same scene with no tiny body.
#[test]
fn counterexample_with_hidden_bodies_costs_what_an_empty_scene_costs() {
    let mut units = vec![body(1, 0, ORIGIN, 0.5)];
    let empty = World::new(Vec2::splat(20.0), units.clone());
    units.extend(counterexample_bodies(1, 2));
    let crowded = World::new(Vec2::splat(20.0), units);
    let known = |id: u32| id == 1;
    let a = slice_to_end(&empty, &known, 32);
    let b = slice_to_end(&crowded, &known, 32);
    eprintln!("HIDDEN40000 work={} slices={}", b.4.owner_actual[0], b.1);
    assert_eq!((a.0, a.1, a.2), (b.0, b.1, b.2));
    assert_eq!(a.4, b.4);
}

/// R2. One cell holds more known bodies than a slice can read: a probe pauses inside the
/// cell, resumes in later slices and the search ends with the stateless result.
#[test]
fn a_cell_larger_than_a_slice_resumes_and_finishes() {
    let mut units = vec![body(1, 0, ORIGIN, 0.5)];
    // 2,000 tiny bodies in the cell [10, 11) x [10, 11). They block no ring-1 candidate
    // that has to read them, but every candidate near them reads all of them.
    for n in 0..2_000 {
        units.push(still(
            2 + n,
            0,
            Vec2::new(
                10.62 + (n % 20) as f32 * 0.0022,
                10.55 + (n / 20) as f32 * 0.0022,
            ),
            0.001,
        ));
    }
    let world = World::new(Vec2::splat(20.0), units);
    let known = |_: u32| true;
    let (point, slices, peak, inside, _) = slice_to_end(&world, &known, 256);
    eprintln!("DENSE slices={slices} peak_slice={peak} inside={inside}");
    assert!(inside, "a slice must pause inside a point query");
    assert!(peak <= 256);
    assert!(slices > 8, "the cell needs several slices: {slices}");
    assert_eq!(point, world.oracle(&known));
}

/// R3. A body that decides an examined candidate moves between slices: the proof rejects
/// the stale verdicts. Once the world is stable the search ends with its stateless result.
#[test]
fn relevant_motion_invalidates_and_a_stable_world_then_finishes() {
    let mut units = vec![body(1, 0, ORIGIN, 0.5)];
    for n in 0..60 {
        let angle = n as f32 * 0.37;
        let distance = 1.0 + (n % 7) as f32 * 0.55;
        units.push(still(
            2 + n,
            0,
            ORIGIN + Vec2::new(angle.cos(), angle.sin()) * distance,
            0.5,
        ));
    }
    let worlds: Vec<World> = (0..2)
        .map(|shift| {
            let mut moved = units.clone();
            // A blocker next to the mover changes place in every slice.
            moved[1].position = ORIGIN + Vec2::new(1.1, 0.0) * if shift == 0 { 1.0 } else { -1.0 };
            World::new(Vec2::splat(20.0), moved)
        })
        .collect();
    let known = |_: u32| true;
    let views: Vec<_> = worlds.iter().map(|world| world.view(&known)).collect();
    let mut counters = RecoveryCounters::default();
    let mut scan = None;
    // Eight churning slices, then the stable world. One candidate for each slice, so the
    // search spans the churn.
    super::recovery::SLICE_CANDIDATES.with(|cap| cap.set(1));
    let mut result = None;
    for slice in 0..10_000 {
        let at = if slice < 8 { slice % 2 } else { 0 };
        match worlds[at].slice(
            &views[at],
            scan,
            super::recovery::FRESH_SLICE,
            &mut counters,
        ) {
            SliceOutcome::Done(point) => {
                result = Some((point, slice));
                break;
            }
            SliceOutcome::Paused(state) => scan = Some(state),
        }
    }
    super::recovery::SLICE_CANDIDATES.with(|cap| cap.set(usize::MAX));
    let (point, slice) = result.expect("the search must end once the world is stable");
    assert!(slice >= 8, "the search spans the churn: {slice}");
    assert!(counters.invalidations >= 1, "{point:?} {counters:?}");
    // A relaxed search commits a point free now; a proved one is the nearest.
    let free = point.is_some_and(|point| {
        worlds[0].units[1..]
            .iter()
            .all(|other| point.distance(other.position) >= 1.0 - 1.0e-5)
    });
    assert!(free, "{point:?}");
    if counters.relaxed_commits == 0 {
        assert_eq!(point, worlds[0].oracle(&known));
    }
}

fn row(row: usize, age: u16, has_scan: bool, x: f32) -> ScheduleRow {
    ScheduleRow {
        row,
        owner: 0,
        age,
        has_scan,
        overlapping: true,
        position: Vec2::new(x, 1.0),
        radius: 0.5,
        spare: false,
        retained: false,
    }
}

/// R4. Batches of identical rows larger than the budget never over-allocate, every share
/// is at least `MIN_SLICE`, and the tick turn serves every row.
#[test]
fn equal_key_batches_never_over_allocate_and_all_get_served() {
    for has_scan in [false, true] {
        let rows: Vec<_> = (0..1_000).map(|r| row(r, 4, has_scan, 3.0)).collect();
        let mut served = BTreeSet::new();
        for tick in 0..4 {
            let schedule = plan(rows.clone(), tick, &mut RecoveryCounters::default());
            let shares: Vec<_> = (0..1_000).map(|r| schedule.allowance(r)).collect();
            let total: usize = shares.iter().sum();
            assert!(total <= OWNER_WORK, "{total}");
            assert!(shares.iter().all(|&s| s == 0 || s >= MIN_SLICE));
            served.extend((0..1_000).filter(|&r| shares[r] > 0));
        }
        assert_eq!(served.len(), 1_000, "has_scan {has_scan}");
    }
    // Many rows with distinct keys: the sum stays within the budget.
    let rows: Vec<_> = (0..5_000)
        .map(|r| row(r, (r % 7) as u16, r % 3 == 0, r as f32))
        .collect();
    let schedule = plan(rows, 9, &mut RecoveryCounters::default());
    let total: usize = (0..5_000).map(|r| schedule.allowance(r)).sum();
    assert!(total <= OWNER_WORK, "{total}");
}

struct Basis {
    /// `(viewer, unit)` pairs that are permitted.
    permitted: BTreeSet<(u8, u32)>,
}

impl MovementVisibility for Basis {
    fn unit_permitted(&self, viewer: u8, unit: u32) -> bool {
        self.permitted.contains(&(viewer, unit))
    }
}

fn step(
    frame: &MovementFrame,
    basis: &Basis,
    tick: u32,
) -> (Vec<MovementFrameOutput>, RecoveryCounters) {
    let mut budget = super::sweep::SweepBudget::new(tick);
    let outputs = super::frame::step_frame_with_motion_guide_and_visibility(
        frame,
        0.05,
        &Default::default(),
        &mut budget,
        &mut super::motion::NoMotionGuide,
        Some(basis),
    );
    (outputs, budget.recovery)
}

fn advance(frame: &mut MovementFrame, outputs: &[MovementFrameOutput]) {
    frame.recovery.clear();
    for output in outputs {
        let unit = frame
            .units
            .iter_mut()
            .find(|unit| unit.unit_id == output.unit_id)
            .unwrap();
        unit.position = output.position;
        unit.target = output.target;
        unit.progress = output.progress.clone();
        unit.plain_progress = output.plain_progress.clone();
        unit.physical_escape = output.physical_escape.clone();
        if let Some(job) = output.recovery.as_deref() {
            frame.recovery.insert(output.unit_id, *job);
        }
    }
}

/// The owner-0 escape scene: a packed own crowd with 6 stacked escaping bodies at its
/// centre. Deep searches span several ticks.
fn escape_scene() -> Vec<MovementFrameUnit> {
    let mut units = Vec::new();
    for y in 0..13 {
        for x in 0..13 {
            units.push(still(
                1 + units.len() as u32,
                0,
                Vec2::new(4.0 + x as f32, 4.0 + y as f32),
                0.5,
            ));
        }
    }
    for n in 0..6 {
        units.push(escaping(500 + n, 0, Vec2::splat(10.0), 0.5));
    }
    units
}

/// Hidden owner-1 bodies that touch no owner-0 body: a dense patch of tiny bodies in the
/// gaps of one crowd corner, inside the grid cells of the old queries, and one large body
/// far away that raises the frame maximum radius.
fn hidden_bodies() -> Vec<MovementFrameUnit> {
    let mut units = Vec::new();
    for n in 0..4_000 {
        units.push(still(
            10_000 + n,
            1,
            Vec2::new(
                4.42 + (n % 70) as f32 * 0.0022,
                4.42 + (n / 70) as f32 * 0.0022,
            ),
            0.001,
        ));
    }
    units.push(still(20_000, 1, Vec2::new(28.0, 28.0), 1.5));
    units
}

fn frame(units: Vec<MovementFrameUnit>) -> MovementFrame {
    let mut units = units;
    units.sort_by_key(|unit| unit.unit_id);
    MovementFrame {
        bounds: Vec2::splat(30.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::new(),
        recovery: BTreeMap::new(),
    }
}

/// The counters that one owner can observe: its work and the search outcomes.
fn observable(counters: &RecoveryCounters) -> Vec<u64> {
    vec![
        counters.owner_work[0] as u64,
        counters.owner_actual[0] as u64,
        counters.candidates,
        counters.cells,
        counters.bodies,
        counters.slot_checks,
        counters.digests,
        counters.invalidations,
        counters.relaxations,
        counters.relaxed_commits,
        counters.slices,
        counters.completions,
        counters.pauses,
        counters.waits,
        u64::from(counters.max_age),
        counters.jobs,
        counters.pool_work,
        counters.spare_slot_work,
        counters.checks,
    ]
}

fn own_outputs(outputs: &[MovementFrameOutput]) -> Vec<MovementFrameOutput> {
    outputs
        .iter()
        .filter(|output| output.unit_id < 10_000)
        .cloned()
        .collect()
}

/// R5. Hidden bodies that touch no owner-0 body change no owner-0 output, job, slice end
/// or owner counter, in every tick until the escapes end. Only frame work changes.
#[test]
fn hidden_non_contact_bodies_change_no_owner_output_or_counter() {
    let basis = Basis {
        permitted: BTreeSet::new(),
    };
    let mut plain = frame(escape_scene());
    let mut units = escape_scene();
    units.extend(hidden_bodies());
    let mut hidden = frame(units);
    let mut searched = 0;
    let mut frame_work = (0, 0);
    for tick in 0..60 {
        let (a, ca) = step(&plain, &basis, tick);
        let (b, cb) = step(&hidden, &basis, tick);
        assert_eq!(own_outputs(&a), own_outputs(&b), "tick {tick}");
        assert_eq!(observable(&ca), observable(&cb), "tick {tick}");
        searched += usize::from(ca.slices > 0);
        frame_work = (
            frame_work.0.max(ca.frame_work),
            frame_work.1.max(cb.frame_work),
        );
        advance(&mut plain, &a);
        advance(&mut hidden, &b);
    }
    eprintln!("HIDDENFRAME searched_ticks={searched} frame_work={frame_work:?}");
    assert!(searched >= 3, "the escapes span ticks: {searched}");
}

/// R6. A hidden body in contact range is known to the actor: the escape changes. Out of
/// contact range it is ignored while hidden and seen when visible.
#[test]
fn a_contacted_hidden_body_is_known_and_an_uncontacted_one_is_not() {
    let run = |hidden_at: Option<Vec2>, visible: bool| {
        // Speed 1: the first step stays out of reach of the hidden body.
        let mut units = vec![
            MovementFrameUnit {
                speed: Some(1.0),
                ..escaping(1, 0, ORIGIN, 0.5)
            },
            still(2, 0, ORIGIN + Vec2::new(0.05, 0.0), 0.5),
        ];
        if let Some(position) = hidden_at {
            units.push(still(3, 1, position, 0.5));
        }
        let basis = Basis {
            permitted: if visible {
                BTreeSet::from([(0, 3)])
            } else {
                BTreeSet::new()
            },
        };
        let (outputs, _) = step(&frame(units), &basis, 1);
        outputs[0].physical_escape.clone().and_then(|e| e.target)
    };
    let free = run(None, false).expect("an escape point");
    let direction = (free - ORIGIN).normalize();
    // On the escape direction, 1.05 from the mover: in contact range (1.101).
    let touching = ORIGIN + direction * 1.05;
    assert_ne!(run(Some(touching), false), Some(free));
    // A contacted hidden body counts as a known body: the escape is the one that the same
    // body gives when it is visible.
    assert_eq!(run(Some(touching), false), run(Some(touching), true));
    // 1.15 from the mover: out of contact range; ignored while hidden, seen when visible.
    let near = ORIGIN + direction * 1.15;
    assert_eq!(run(Some(near), false), Some(free));
    assert_ne!(run(Some(near), true), Some(free));
}

/// R7. Reversed unit ids, reversed input rows and swapped owner slots give the same
/// physical outcome and the same per-owner work.
#[test]
fn id_row_and_owner_reversal_keep_outcomes_and_work() {
    let scene = |reverse_ids: bool, swap_owner: bool| {
        let mut units = escape_scene();
        units.extend(hidden_bodies());
        for unit in &mut units {
            if reverse_ids {
                unit.unit_id = 30_000 - unit.unit_id;
            }
            if swap_owner {
                unit.owner_slot = 1 - unit.owner_slot;
            }
        }
        let mut frame = frame(units);
        if reverse_ids {
            frame.units.reverse();
        }
        frame
    };
    let run = |reverse_ids: bool, swap_owner: bool| {
        let basis = Basis {
            permitted: BTreeSet::new(),
        };
        let mut frame = scene(reverse_ids, swap_owner);
        let mut work = Vec::new();
        for tick in 0..60 {
            let (outputs, counters) = step(&frame, &basis, tick);
            let owner = usize::from(swap_owner);
            work.push((
                counters.owner_work[owner],
                counters.slices,
                counters.completions,
            ));
            advance(&mut frame, &outputs);
        }
        let mut positions: Vec<_> = frame
            .units
            .iter()
            .map(|unit| (unit.position.x.to_bits(), unit.position.y.to_bits()))
            .collect();
        positions.sort_unstable();
        (positions, work)
    };
    let reference = run(false, false);
    assert_eq!(run(true, false), reference, "reversed ids and rows");
    assert_eq!(run(false, true), reference, "swapped owners");
    assert_eq!(run(true, true), reference, "both");
}

/// A blocked group mover of pool 7 whose slot is under a crowd body.
fn blocked(unit_id: u32, position: Vec2, goal: Vec2) -> MovementFrameUnit {
    MovementFrameUnit {
        target: Some(goal),
        progress: Some(super::MoveProgress {
            pool_id: 7,
            slot: goal,
            activity: super::MovementActivity::Blocked,
            pass_mode: true,
            stalling_on_group: false,
            stalled_ticks: 60,
            best_distance: 1.0,
            claim_count: 3,
            follow_origin: None,
        }),
        ..body(unit_id, 0, position, 0.5)
    }
}

/// R8. A hundred blocked group movers of one owner inside a packed crowd, each with a
/// search that needs several slices: every mover gets service and resolves, and no tick
/// exceeds the owner allowance.
#[test]
fn a_hundred_blocked_bodies_all_get_service() {
    let side = 20;
    let mut units = Vec::new();
    for y in 0..side {
        for x in 0..side {
            units.push(still(
                1 + units.len() as u32,
                0,
                Vec2::new(6.5 + x as f32, 6.5 + y as f32),
                0.5,
            ));
        }
    }
    let goal = Vec2::new(6.5, 6.5);
    for n in 0..100 {
        units.push(blocked(
            1_000 + n,
            Vec2::new(12.0 + (n % 10) as f32 * 0.83, 12.0 + (n / 10) as f32 * 0.83),
            goal,
        ));
    }
    let mut frame = frame(units);
    frame.bounds = Vec2::splat(side as f32 + 13.0);
    frame.pools.insert(
        7,
        super::MovementPool {
            pool_id: 7,
            point: goal,
            spacing: 1.1,
            primary_count: 100,
            slots: vec![goal],
        },
    );
    let mut peak = 0;
    let mut ticks = 0;
    let unresolved = |frame: &MovementFrame| {
        frame
            .units
            .iter()
            .filter(|unit| {
                unit.progress
                    .as_ref()
                    .is_some_and(|progress| progress.activity == super::MovementActivity::Blocked)
                    && unit.target.is_none_or(|target| target == goal)
            })
            .count()
    };
    for tick in 0..600 {
        let mut budget = super::sweep::SweepBudget::new(tick);
        let outputs =
            super::step_frame_with_sweep_budget(&frame, 0.05, &Default::default(), &mut budget);
        peak = peak.max(budget.recovery.owner_work[0]);
        advance(&mut frame, &outputs);
        ticks = tick + 1;
        if unresolved(&frame) == 0 {
            break;
        }
    }
    eprintln!(
        "HUNDRED ticks={ticks} peak_work={peak} unresolved={}",
        unresolved(&frame)
    );
    assert!(peak <= OWNER_WORK);
    assert_eq!(unresolved(&frame), 0, "every mover resolves");
}

/// A slice allowance large enough for every proof of these scenes.
const WIDE: usize = 4_096;

fn view_of(units: &[MovementFrameUnit]) -> KnownView {
    let members: Vec<_> = units
        .iter()
        .enumerate()
        .map(|(row, unit)| (row, unit.position, unit.radius))
        .collect();
    KnownView::build(&members, units.len())
}

/// One slice of the search of the mover `units[0]` over `view`, with the claims of pool 7.
fn pool_slice(
    units: &[MovementFrameUnit],
    view: &KnownView,
    claims: &ClaimSet,
    pool: Option<PoolSearch>,
    previous: Option<RecoveryScan>,
    allowance: usize,
    counters: &mut RecoveryCounters,
) -> SliceOutcome {
    let contacts = Contacts::default();
    let known = KnownSet {
        view,
        contacts: &contacts,
        own_key: Some(body_key(units[0].position, units[0].radius)),
        ignore_soft: false,
    };
    EscapeScan {
        bounds: Vec2::splat(20.0),
        origin: units[0].position,
        radius: units[0].radius,
        slots: Some((7, claims)),
        pool,
        bound: None,
        verify: true,
        pool_only: false,
    }
    .run(&known, previous, allowance, 0, counters)
}

fn with_candidate_cap<T>(cap: usize, body: impl FnOnce() -> T) -> T {
    SLICE_CANDIDATES.with(|limit| limit.set(cap));
    let result = body();
    SLICE_CANDIDATES.with(|limit| limit.set(usize::MAX));
    result
}

/// R9. The check of a relaxed point pauses inside a dense cell. Then a claim of the point,
/// or a body on the point, appears. The resumed check must not commit the point.
#[test]
fn a_paused_check_sees_a_new_claim_or_body_on_its_point() {
    let point = ORIGIN + Vec2::new(2.5, 0.0);
    let mut units = vec![still(1, 0, ORIGIN, 0.5)];
    // Tiny bodies 0.6 from the point: they do not block it, but its query must read them.
    for n in 0..2_000u32 {
        units.push(still(
            2 + n,
            0,
            point + Vec2::new(0.6 + (n % 40) as f32 * 0.0025, (n / 40) as f32 * 0.0025),
            0.001,
        ));
    }
    let view = view_of(&units);
    let mut with_body = units.clone();
    with_body.push(still(9_999, 0, point, 0.5));
    let body_view = view_of(&with_body);
    let contacts = Contacts::default();
    let cell = KnownSet {
        view: &view,
        contacts: &contacts,
        own_key: None,
        ignore_soft: false,
    }
    .cell_bits();
    assert_eq!(
        cell,
        KnownSet {
            view: &body_view,
            contacts: &contacts,
            own_key: None,
            ignore_soft: false,
        }
        .cell_bits()
    );
    let start = RecoveryScan {
        origin: ORIGIN,
        radius: 0.5,
        ring: 1,
        side: 0,
        step: 0,
        best: None,
        examined: 0.0,
        digest: [0; 3],
        invalidations: 2,
        relaxed: true,
        cell,
        stage: RecoveryStage::Verify { point, slot: false },
        pool: None,
        query: None,
        spanned: true,
    };
    let empty = ClaimSet::default();
    let mut claimed = ClaimSet::default();
    claimed.insert((7, point.x.to_bits(), point.y.to_bits()));
    for (name, claims, later) in [("claim", &claimed, &view), ("body", &empty, &body_view)] {
        let mut counters = RecoveryCounters::default();
        let first = pool_slice(
            &units,
            &view,
            &empty,
            None,
            Some(start),
            MIN_SLICE,
            &mut counters,
        );
        let SliceOutcome::Paused(mut scan) = first else {
            panic!("{name}: the check must pause");
        };
        assert!(scan.query.is_some_and(|query| query.started), "{name}");
        let mut result = None;
        for _ in 0..10_000 {
            match pool_slice(
                &units,
                later,
                claims,
                None,
                Some(scan),
                MIN_SLICE,
                &mut counters,
            ) {
                SliceOutcome::Done(point) => {
                    result = Some(point);
                    break;
                }
                SliceOutcome::Paused(next) => scan = next,
            }
        }
        let result = result.unwrap_or_else(|| panic!("{name}: the search must end"));
        assert_ne!(result, Some(point), "{name}: the point is not free now");
        if let Some(found) = result
            && name == "body"
        {
            assert!(found.distance(point) >= 1.0 - 1.0e-5, "{name}: {found:?}");
        }
    }
}

/// R10. A far body joins and leaves the view in turn, so the view cell size changes in
/// every slice. The search counts each change, runs relaxed and ends.
#[test]
fn a_view_cell_size_that_changes_every_slice_does_not_stop_the_search() {
    let mut units = vec![still(1, 0, ORIGIN, 0.5)];
    for n in 0..60 {
        let angle = n as f32 * 0.37;
        let distance = 1.0 + (n % 7) as f32 * 0.55;
        units.push(still(
            2 + n,
            0,
            ORIGIN + Vec2::new(angle.cos(), angle.sin()) * distance,
            0.5,
        ));
    }
    let small = view_of(&units);
    let mut with_big = units.clone();
    with_big.push(still(999, 1, Vec2::new(18.0, 18.0), 1.5));
    let big = view_of(&with_big);
    let claims = ClaimSet::default();
    let finish = |toggle: bool| {
        let mut counters = RecoveryCounters::default();
        let mut scan = None;
        for slice in 0..2_000 {
            let view = if toggle && slice % 2 == 1 {
                &big
            } else {
                &small
            };
            match pool_slice(&units, view, &claims, None, scan, MIN_SLICE, &mut counters) {
                SliceOutcome::Done(point) => return (Some((slice, point)), counters),
                SliceOutcome::Paused(next) => scan = Some(next),
            }
        }
        (None, counters)
    };
    let (stable, _) = finish(false);
    let (toggled, counters) = finish(true);
    eprintln!(
        "CELLTOGGLE stable={stable:?} toggled={toggled:?} invalidations={} relaxations={}",
        counters.invalidations, counters.relaxations
    );
    let (stable_slices, stable_point) = stable.expect("the stable search ends");
    assert!(stable_slices > 1, "the search spans slices");
    let (_, point) = toggled.expect("the toggled search ends");
    assert!(counters.invalidations > 0);
    // The relaxed point is free in both views.
    let point = point.expect("a free point");
    for unit in &with_big[1..] {
        assert!(point.distance(unit.position) >= 0.5 + unit.radius - 1.0e-5);
    }
    assert!(stable_point.is_some());
    // A cheap proof (a wide slice, one candidate in each slice) never relaxes by cost, so
    // only the counted cell changes make the scan relaxed.
    let (cheap, counters) = with_candidate_cap(1, || {
        let mut counters = RecoveryCounters::default();
        let mut scan = None;
        for slice in 0..2_000 {
            let view = if slice % 2 == 1 { &big } else { &small };
            match pool_slice(&units, view, &claims, None, scan, 1_024, &mut counters) {
                SliceOutcome::Done(point) => return (Some(point), counters),
                SliceOutcome::Paused(next) => scan = Some(next),
            }
        }
        (None, counters)
    });
    eprintln!(
        "CELLTOGGLE cheap={cheap:?} invalidations={} relaxations={}",
        counters.invalidations, counters.relaxations
    );
    assert_eq!(counters.relaxations, 0);
    assert!(
        cheap.is_some(),
        "the toggled search with a cheap proof ends"
    );
}

/// The slots of a dense pool round `ORIGIN`: 289 slots 0.15 apart.
fn dense_pool() -> super::MovementPool {
    let mut slots = Vec::new();
    for y in -8..=8 {
        for x in -8..=8 {
            slots.push(ORIGIN + Vec2::new(x as f32 * 0.15, y as f32 * 0.15));
        }
    }
    super::MovementPool {
        pool_id: 7,
        point: ORIGIN,
        spacing: 0.15,
        primary_count: slots.len() as u32,
        slots,
    }
}

/// R11. A pool precheck with more local slots than one slice can read is metered: each
/// slice stays within its allowance and the search ends with the unsliced result.
#[test]
fn the_pool_precheck_stays_inside_each_slice() {
    let units = vec![still(1, 0, ORIGIN, 0.5)];
    let view = view_of(&units);
    let index = super::pool_index::MovementPoolIndex::new(&dense_pool(), Vec2::splat(20.0));
    let claims = ClaimSet::default();
    let pool = || {
        Some(PoolSearch {
            index: &index,
            unit_id: 1,
            allowed: None,
            skip: None,
        })
    };
    let mut wide = RecoveryCounters::default();
    let SliceOutcome::Done(expected) =
        pool_slice(&units, &view, &claims, pool(), None, WIDE, &mut wide)
    else {
        panic!("one wide slice ends");
    };
    assert!(wide.pool_work > MIN_SLICE as u64, "{}", wide.pool_work);
    let mut counters = RecoveryCounters::default();
    let mut scan = None;
    let mut slices = 0;
    let result = loop {
        let before = counters.owner_actual[0];
        let outcome = pool_slice(
            &units,
            &view,
            &claims,
            pool(),
            scan,
            MIN_SLICE,
            &mut counters,
        );
        assert!(counters.owner_actual[0] - before <= MIN_SLICE);
        slices += 1;
        assert!(slices < 1_000, "the search must end");
        match outcome {
            SliceOutcome::Done(point) => break point,
            SliceOutcome::Paused(next) => scan = Some(next),
        }
    };
    assert!(slices > 2, "the precheck spans slices: {slices}");
    assert_eq!(result, expected);
    assert_eq!(counters.relaxations, 0);
}

/// R12. A body that moves earlier in the tick is at its new place for a later search of
/// the same tick: the view follows each move.
#[test]
fn a_move_earlier_in_the_tick_is_in_the_view() {
    let mut units = vec![
        still(1, 0, ORIGIN, 0.5),
        still(2, 0, Vec2::new(18.0, 18.0), 0.5),
    ];
    let world = RecoveryWorld::new(None, None, 0.0);
    let scan = EscapeScan {
        bounds: Vec2::splat(20.0),
        origin: ORIGIN,
        radius: 0.5,
        slots: None,
        pool: None,
        bound: None,
        verify: true,
        pool_only: false,
    };
    let mut counters = RecoveryCounters::default();
    let mut search = |units: &[MovementFrameUnit]| {
        let mut jobs = BTreeMap::new();
        RowRecovery {
            row: 0,
            owner: 0,
            allowance: WIDE,
            check_only: false,
            previous: None,
            consumed: false,
            next_jobs: &mut jobs,
            counters: &mut counters,
            world: &world,
        }
        .search(RecoveryKind::Physical, &scan, units)
    };
    let first = search(&units).expect("a free point");
    world.relocate(1, units[1].position, first, 0.5);
    units[1].position = first;
    let second = search(&units).expect("a free point");
    assert_ne!(second, first);
    assert!(second.distance(first) >= 1.0 - 1.0e-5);
}

/// R13. The mover's own body does not block a pool slot under it.
#[test]
fn the_mover_does_not_block_a_pool_slot_under_itself() {
    let units = vec![still(1, 0, ORIGIN, 0.5)];
    let view = view_of(&units);
    let near = ORIGIN + Vec2::new(0.3, 0.0);
    let pool = super::MovementPool {
        pool_id: 7,
        point: ORIGIN,
        spacing: 1.0,
        primary_count: 2,
        slots: vec![near, ORIGIN + Vec2::new(3.0, 0.0)],
    };
    let mut index = super::pool_index::MovementPoolIndex::new(&pool, Vec2::splat(20.0));
    let points: Vec<_> = units
        .iter()
        .map(|unit| crate::grid::GridPoint {
            id: unit.unit_id,
            position: unit.position,
        })
        .collect();
    let grid = crate::grid::UniformGrid::new(&points, 1.0);
    let witness = vec![Vec::new(); units.len()];
    for slot in 0..pool.slots.len() {
        index.refresh_slot_clearance(slot, &units, &points, &grid, 0.5, false, &witness);
    }
    let claims = ClaimSet::default();
    let search = PoolSearch {
        index: &index,
        unit_id: 1,
        allowed: None,
        skip: None,
    };
    let mut counters = RecoveryCounters::default();
    let outcome = pool_slice(
        &units,
        &view,
        &claims,
        Some(search),
        None,
        WIDE,
        &mut counters,
    );
    assert!(matches!(outcome, SliceOutcome::Done(Some(point)) if point == near));
}

/// R14. A claim on the best point so far that appears while a proving scan pauses changes
/// its proof: the scan restarts and does not commit the claimed point.
#[test]
fn a_new_claim_in_the_examined_region_invalidates_the_proof() {
    let units = vec![still(1, 0, ORIGIN, 0.5)];
    let view = view_of(&units);
    let empty = ClaimSet::default();
    let mut counters = RecoveryCounters::default();
    let SliceOutcome::Done(Some(winner)) =
        pool_slice(&units, &view, &empty, None, None, WIDE, &mut counters)
    else {
        panic!("an empty world has a free point");
    };
    let mut claimed = ClaimSet::default();
    claimed.insert((7, winner.x.to_bits(), winner.y.to_bits()));
    let result = with_candidate_cap(1, || {
        let mut counters = RecoveryCounters::default();
        let mut scan = None;
        // One candidate in each slice, until the winner is the best point so far.
        for _ in 0..100 {
            match pool_slice(&units, &view, &empty, None, scan, WIDE, &mut counters) {
                SliceOutcome::Paused(next) if next.best == Some(winner) => {
                    scan = Some(next);
                    break;
                }
                SliceOutcome::Paused(next) => scan = Some(next),
                SliceOutcome::Done(_) => panic!("the capped scan pauses"),
            }
        }
        assert!(scan.is_some_and(|scan| scan.best == Some(winner) && !scan.relaxed));
        for _ in 0..1_000 {
            match pool_slice(&units, &view, &claimed, None, scan, WIDE, &mut counters) {
                SliceOutcome::Done(point) => return (point, counters.invalidations),
                SliceOutcome::Paused(next) => scan = Some(next),
            }
        }
        panic!("the search must end");
    });
    assert_ne!(result.0, Some(winner));
    assert!(result.1 > 0);
}

/// The stress scene of `tests/recovery_probe.rs`: a packed 16 x 16 map and 100 blocked
/// group movers outside it.
fn probe_stress_frame() -> MovementFrame {
    let goal = Vec2::splat(0.5);
    let mut units = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            units.push(body(
                units.len() as u32 + 1,
                0,
                Vec2::new(x as f32 + 0.5, y as f32 + 0.5),
                0.6,
            ));
        }
    }
    for n in 0..100 {
        let mut unit = blocked(
            units.len() as u32 + 1,
            Vec2::new((n % 32) as f32 + 0.5, 20.0 + (n / 32) as f32),
            goal,
        );
        unit.radius = 0.6;
        units.push(unit);
    }
    MovementFrame {
        bounds: Vec2::splat(16.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(
            7,
            super::MovementPool {
                pool_id: 7,
                point: goal,
                spacing: 1.1,
                primary_count: 100,
                slots: vec![goal],
            },
        )]),
        recovery: BTreeMap::new(),
    }
}

/// Report (no time claim): the work of 40 ticks of the stress scene, by class (K: owner
/// work; P: neighbourhood reads; V: view builds).
#[test]
fn stress_scene_work_report() {
    let mut frame = probe_stress_frame();
    let mut peak = RecoveryCounters::default();
    let mut total = RecoveryCounters::default();
    for tick in 1..=40 {
        let mut budget = super::sweep::SweepBudget::new(tick);
        let outputs =
            super::step_frame_with_sweep_budget(&frame, 0.05, &Default::default(), &mut budget);
        let counters = &budget.recovery;
        assert!(counters.owner_work[0] <= OWNER_WORK);
        assert_eq!(
            counters.frame_work,
            counters.physical_work + counters.view_work
        );
        peak.owner_work[0] = peak.owner_work[0].max(counters.owner_work[0]);
        peak.physical_reads = peak.physical_reads.max(counters.physical_reads);
        peak.physical_work = peak.physical_work.max(counters.physical_work);
        peak.view_work = peak.view_work.max(counters.view_work);
        total.owner_work[0] += counters.owner_work[0];
        total.physical_reads += counters.physical_reads;
        total.physical_work += counters.physical_work;
        total.view_work += counters.view_work;
        total.checks += counters.checks;
        advance(&mut frame, &outputs);
    }
    eprintln!(
        "STRESSWORK ticks=40 k_peak={} k_total={} physical_reads_peak={} physical_reads_total={} \
         physical_work_peak={} physical_work_total={} view_work_peak={} view_work_total={} \
         frame_total={} checks_total={}",
        peak.owner_work[0],
        total.owner_work[0],
        peak.physical_reads,
        total.physical_reads,
        peak.physical_work,
        total.physical_work,
        peak.view_work,
        total.view_work,
        total.physical_work + total.view_work,
        total.checks
    );
    // 100 blocked group rows: two fixed reads for each row and tick, and the reads of the
    // slot refresh (this pool has one slot, far from the rows).
    assert!(peak.physical_reads <= 200);
}

// ---- Section 7 of the design: the work outside the owner allowance (R15 to R24). ----

/// One row turn of the actor `units[0]` (owner 0) with `allowance`. `job` is the job of
/// the last turn and becomes the job that this turn keeps.
fn row_turn<T>(
    world: &RecoveryWorld,
    job: &mut Option<RecoverySearch>,
    allowance: usize,
    counters: &mut RecoveryCounters,
    run: impl FnOnce(&mut RowRecovery) -> T,
) -> T {
    let mut jobs = BTreeMap::new();
    let result = run(&mut RowRecovery {
        row: 0,
        owner: 0,
        allowance,
        check_only: false,
        previous: *job,
        consumed: false,
        next_jobs: &mut jobs,
        counters,
        world,
    });
    *job = jobs.remove(&0);
    result
}

/// The actor at `ORIGIN` and 2,000 tiny own bodies 0.6 from `point`: they do not block
/// the point, but a query of the point must read all of them (one view cell).
fn dense_round(point: Vec2) -> Vec<MovementFrameUnit> {
    let mut units = vec![still(1, 0, ORIGIN, 0.5)];
    for n in 0..2_000u32 {
        units.push(still(
            2 + n,
            0,
            point + Vec2::new(0.6 + (n % 40) as f32 * 0.0025, (n / 40) as f32 * 0.0025),
            0.001,
        ));
    }
    units
}

fn one_slot_pool(slot: Vec2) -> super::pool_index::MovementPoolIndex {
    super::pool_index::MovementPoolIndex::new(
        &super::MovementPool {
            pool_id: 7,
            point: slot,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![slot],
        },
        Vec2::splat(20.0),
    )
}

/// The check scan of the actor at `ORIGIN` for a group row (`slot`) or another row.
fn check_scan<'a>(
    slot: bool,
    claims: &'a ClaimSet,
    index: &'a super::pool_index::MovementPoolIndex,
    allowed: Option<(u64, u32, u32)>,
) -> EscapeScan<'a> {
    EscapeScan {
        bounds: Vec2::splat(20.0),
        origin: ORIGIN,
        radius: 0.5,
        slots: slot.then_some((7, claims)),
        pool: slot.then_some(PoolSearch {
            index,
            unit_id: 1,
            allowed,
            skip: None,
        }),
        bound: None,
        verify: true,
        pool_only: false,
    }
}

const KINDS: [(&str, RecoveryKind, bool); 3] = [
    ("group", RecoveryKind::Group(7), true),
    ("physical", RecoveryKind::Physical, false),
    ("plain", RecoveryKind::Plain, false),
];

/// Runs the check of `point` in slices of `MIN_SLICE` until it ends. Returns the result
/// and the count of slices. Each slice stays within its allowance.
fn sliced_check(
    units: &[MovementFrameUnit],
    scan: &EscapeScan,
    kind: RecoveryKind,
    point: Vec2,
    slot: bool,
) -> (Check, usize) {
    let world = RecoveryWorld::new(None, None, 0.0);
    let mut counters = RecoveryCounters::default();
    let mut job = None;
    for slices in 1..10_000 {
        let before = counters.owner_work[0];
        let result = row_turn(&world, &mut job, MIN_SLICE, &mut counters, |row| {
            row.check(kind, scan, point, slot, false, units)
        });
        assert!(counters.owner_work[0] - before <= MIN_SLICE);
        if result != Check::Pending {
            assert!(job.is_none(), "a check that ended keeps no job");
            return (result, slices);
        }
        let stored = job.and_then(|job| job.scan).expect("a paused check");
        assert!(stored.is_check());
        assert_eq!(job.map(|job| job.age), Some(slices as u16));
    }
    panic!("the check must end");
}

/// R15. A retained target whose query box holds more known bodies than one slice can
/// read: the check pauses with a cursor, resumes and ends with the unsliced result, for
/// each kind of blocked row. No slice is above its allowance.
#[test]
fn a_retained_target_in_a_cell_larger_than_a_slice_resumes_and_ends() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    let clear = dense_round(target);
    // A blocker that is the last key of the cell in canonical order.
    let mut blocked = clear.clone();
    blocked.push(still(9_000, 0, target + Vec2::new(0.9, 0.0), 0.5));
    let claims = ClaimSet::default();
    let index = one_slot_pool(target);
    for (name, kind, slot) in KINDS {
        let scan = check_scan(slot, &claims, &index, None);
        for (units, expected) in [(&clear, Check::Clear), (&blocked, Check::Blocked)] {
            let world = RecoveryWorld::new(None, None, 0.0);
            let whole = row_turn(
                &world,
                &mut None,
                OWNER_WORK,
                &mut RecoveryCounters::default(),
                |row| row.check(kind, &scan, target, slot, false, units),
            );
            assert_eq!(whole, expected, "{name}: one wide slice");
            let (result, slices) = sliced_check(units, &scan, kind, target, slot);
            assert_eq!(result, expected, "{name}");
            assert!(slices > 8, "{name}: the check spans slices: {slices}");
        }
    }
}

/// The owner-0 group row of pool 7 at `ORIGIN`, blocked on a body under it, with the
/// retained escape target `target`.
fn retained_group_scene(target: Vec2) -> MovementFrame {
    let mut mover = blocked(1, ORIGIN, target);
    mover.speed = Some(10.0);
    let mut units = vec![mover, still(2, 0, ORIGIN + Vec2::new(-0.6, 0.0), 0.5)];
    // 36,000 tiny own bodies in one view cell, 0.6 from the target: more than the
    // allowance of one owner.
    for n in 0..36_000u32 {
        units.push(still(
            10 + n,
            0,
            target + Vec2::new(0.6 + (n % 200) as f32 * 0.0005, (n / 200) as f32 * 0.0005),
            0.001,
        ));
    }
    let mut frame = frame(units);
    frame.pools.insert(
        7,
        super::MovementPool {
            pool_id: 7,
            point: target,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![target],
        },
    );
    frame
}

/// R15, in the frame. The check of a retained target that costs more than the owner
/// allowance: the row holds its position and its target, keeps a check job, and moves in
/// a later tick. No tick is above the owner allowance.
#[test]
fn a_frame_row_holds_its_position_while_its_retained_check_is_paused() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    let mut frame = retained_group_scene(target);
    let mut held = 0;
    for tick in 0..6 {
        let mut budget = super::sweep::SweepBudget::new(tick);
        let outputs =
            super::step_frame_with_sweep_budget(&frame, 0.05, &Default::default(), &mut budget);
        assert!(budget.recovery.owner_work[0] <= OWNER_WORK);
        let mover = outputs.iter().find(|output| output.unit_id == 1).unwrap();
        if let Some(job) = mover.recovery.as_deref() {
            assert!(job.scan.is_some_and(|scan| scan.is_check()), "{job:?}");
            assert_eq!(mover.position, ORIGIN, "a paused check holds the row");
            assert_eq!(
                mover.target,
                Some(target),
                "a paused check keeps the target"
            );
            held += 1;
        } else {
            assert!(held >= 1, "the check costs more than one owner allowance");
            // The check ended and found the target clear: the row keeps it and starts
            // no search. (The dense cell makes the sweep guard refuse the step itself.)
            assert_eq!(mover.target, Some(target));
            assert_eq!((budget.recovery.checks, budget.recovery.slices), (1, 0));
            return;
        }
        advance(&mut frame, &outputs);
    }
    panic!("the retained check must end");
}

/// R16. The check of a retained target pauses inside a dense cell. Then a body, or a
/// claim of another mover, appears on the target. The resumed check must not report the
/// target as clear.
#[test]
fn a_paused_retained_check_sees_a_new_body_or_claim_on_its_target() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    let units = dense_round(target);
    // The new body is the first key of its cell: a cursor that just continues is past it.
    let mut with_body = units.clone();
    with_body.insert(1, still(9_000, 0, target + Vec2::new(0.55, 0.0), 0.5));
    let empty = ClaimSet::default();
    let mut claimed = ClaimSet::default();
    claimed.insert((7, target.x.to_bits(), target.y.to_bits()));
    let index = one_slot_pool(target);
    for (name, kind, slot) in KINDS {
        for change in ["body", "claim"] {
            if change == "claim" && !slot {
                continue;
            }
            let scan = check_scan(slot, &empty, &index, None);
            let world = RecoveryWorld::new(None, None, 0.0);
            let mut counters = RecoveryCounters::default();
            let mut job = None;
            let first = row_turn(&world, &mut job, MIN_SLICE, &mut counters, |row| {
                row.check(kind, &scan, target, slot, false, &units)
            });
            assert_eq!(first, Check::Pending, "{name}");
            assert!(
                job.and_then(|job| job.scan)
                    .and_then(|scan| scan.query)
                    .is_some_and(|query| query.started),
                "{name}: the check pauses inside its query"
            );
            let later_units = if change == "body" { &with_body } else { &units };
            let later_scan = check_scan(
                slot,
                if change == "claim" { &claimed } else { &empty },
                &index,
                None,
            );
            let later_world = RecoveryWorld::new(None, None, 0.0);
            let mut result = Check::Pending;
            for _ in 0..10_000 {
                result = row_turn(&later_world, &mut job, MIN_SLICE, &mut counters, |row| {
                    row.check(kind, &later_scan, target, slot, false, later_units)
                });
                if result != Check::Pending {
                    break;
                }
            }
            assert_eq!(result, Check::Blocked, "{name}, new {change}");
        }
    }
}

/// A guide with no claim exchange: the frame solver of the terrain navigation.
struct NoExchange;

impl super::motion::MotionGuide for NoExchange {
    fn kind(&self, _unit_id: u32) -> super::motion::MotionKind {
        super::motion::MotionKind::Destination
    }

    fn next_position(
        &mut self,
        _request: super::motion::MotionRequest,
        _budget: &mut super::sweep::SweepBudget,
    ) -> Option<Vec2> {
        None
    }
}

/// One tick of `frame` with or without the claim exchange and a visibility basis.
fn step_mode(
    frame: &MovementFrame,
    basis: Option<&Basis>,
    exchange: bool,
    tick: u32,
) -> (Vec<MovementFrameOutput>, RecoveryCounters) {
    let mut budget = super::sweep::SweepBudget::new(tick);
    let mut no_exchange = NoExchange;
    let mut with_exchange = super::motion::NoMotionGuide;
    let guide: &mut dyn super::motion::MotionGuide = if exchange {
        &mut with_exchange
    } else {
        &mut no_exchange
    };
    let outputs = super::frame::step_frame_with_motion_guide_and_visibility(
        frame,
        0.05,
        &Default::default(),
        &mut budget,
        guide,
        basis.map(|basis| basis as &dyn MovementVisibility),
    );
    (outputs, budget.recovery)
}

const OWN_SLOT: Vec2 = Vec2::new(14.0, 10.0);
/// The spare slots of the stalled mover at `ORIGIN`, nearest first.
const SPARES: [Vec2; 3] = [
    Vec2::new(10.0, 11.5),
    Vec2::new(10.0, 8.2),
    Vec2::new(7.5, 10.0),
];

/// A group mover of pool 7 whose stall clock becomes due in the next tick. It makes no
/// progress (its best distance is 0), so it runs the spare-slot search.
fn stalled(unit_id: u32, position: Vec2, slot: Vec2) -> MovementFrameUnit {
    stalled_in(7, unit_id, position, slot)
}

fn stalled_in(pool_id: u64, unit_id: u32, position: Vec2, slot: Vec2) -> MovementFrameUnit {
    MovementFrameUnit {
        target: Some(slot),
        progress: Some(super::MoveProgress {
            pool_id,
            slot,
            activity: super::MovementActivity::Moving,
            pass_mode: true,
            stalling_on_group: false,
            stalled_ticks: 19,
            best_distance: 0.0,
            claim_count: 0,
            follow_origin: None,
        }),
        ..body(unit_id, 0, position, 0.5)
    }
}

fn spare_frame(extra: Vec<MovementFrameUnit>) -> MovementFrame {
    let mut units = vec![stalled(1, ORIGIN, OWN_SLOT)];
    units.extend(extra);
    let mut frame = frame(units);
    let mut slots = vec![OWN_SLOT];
    slots.extend(SPARES);
    frame.pools.insert(
        7,
        super::MovementPool {
            pool_id: 7,
            point: OWN_SLOT,
            spacing: 1.1,
            primary_count: 1,
            slots,
        },
    );
    frame
}

/// The slot of the stalled mover (unit 1) after one tick.
fn spare_of(frame: &MovementFrame, basis: Option<&Basis>, exchange: bool) -> Option<Vec2> {
    let (outputs, _) = step_mode(frame, basis, exchange, 1);
    let mover = outputs.iter().find(|output| output.unit_id == 1).unwrap();
    assert!(mover.recovery.is_none(), "the search ends in one slice");
    let progress = mover.progress.as_ref().unwrap();
    (progress.activity == super::MovementActivity::Moving).then_some(progress.slot)
}

/// One case of R17: the name, the bodies, the permitted units under fog (`None`: no
/// basis), and the slot with and without the claim exchange.
type SpareCase<'a> = (
    &'a str,
    Vec<MovementFrameUnit>,
    Option<&'a BTreeSet<(u8, u32)>>,
    Vec2,
    Vec2,
);

/// R17. The metered spare-slot search gives the slot of `nearest_free` with the base
/// predicate, for each class of body on the nearest spare slot, with and without the
/// claim exchange. The frame compares each search with `nearest_free` (`recovery_oracle`).
#[test]
fn the_spare_search_gives_the_slot_of_nearest_free() {
    let on_first = SPARES[0];
    let foreign = |unit_id: u32, position: Vec2| still(unit_id, 1, position, 0.5);
    let nothing = BTreeSet::new();
    let sees_50 = BTreeSet::from([(0u8, 50u32)]);
    // (name, bodies, permitted units under fog, slot with the exchange, slot without).
    let cases: Vec<SpareCase> = vec![
        ("no body", vec![], None, SPARES[0], SPARES[0]),
        (
            "own body",
            vec![still(50, 0, on_first, 0.5)],
            None,
            SPARES[1],
            SPARES[1],
        ),
        // A body of another owner at rest that no own body touches: soft in exchange mode.
        (
            "foreign at rest, no witness",
            vec![foreign(50, on_first)],
            None,
            SPARES[0],
            SPARES[1],
        ),
        // The same body with an own body in contact range of it: it blocks.
        (
            "foreign at rest, own witness",
            vec![
                foreign(50, on_first),
                still(51, 0, on_first + Vec2::new(0.0, 1.05), 0.5),
            ],
            None,
            SPARES[1],
            SPARES[1],
        ),
        // A body of another owner that moves: soft in exchange mode.
        (
            "foreign mover",
            vec![MovementFrameUnit {
                target: Some(on_first + Vec2::new(0.0, 6.0)),
                speed: Some(0.2),
                ..foreign(50, on_first)
            }],
            None,
            SPARES[0],
            SPARES[1],
        ),
        (
            "fog: hidden, no contact",
            vec![foreign(50, on_first)],
            Some(&nothing),
            SPARES[0],
            SPARES[0],
        ),
        // 1.05 from the mover: in contact range (1.101), and 0.45 from the first slot.
        (
            "fog: hidden, in contact",
            vec![foreign(50, ORIGIN + Vec2::new(0.0, 1.05))],
            Some(&nothing),
            SPARES[1],
            SPARES[1],
        ),
        (
            "fog: visible at rest, no witness",
            vec![foreign(50, on_first)],
            Some(&sees_50),
            SPARES[0],
            SPARES[1],
        ),
        (
            "fog: visible at rest, own witness",
            vec![
                foreign(50, on_first),
                still(51, 0, on_first + Vec2::new(0.0, 1.05), 0.5),
            ],
            Some(&sees_50),
            SPARES[1],
            SPARES[1],
        ),
    ];
    take_compared();
    for (name, bodies, permitted, with_exchange, without) in cases {
        let frame = spare_frame(bodies);
        let basis = permitted.map(|permitted| Basis {
            permitted: permitted.clone(),
        });
        for (exchange, expected) in [(true, with_exchange), (false, without)] {
            assert_eq!(
                spare_of(&frame, basis.as_ref(), exchange),
                Some(expected),
                "{name}, exchange {exchange}"
            );
        }
    }
    assert_eq!(
        take_compared()[2],
        18,
        "each search is compared with nearest_free"
    );
}

/// A pool of 289 slots 0.15 apart round `ORIGIN` and one slot far away. A known body
/// blocks every near slot.
fn blocked_dense_pool() -> (super::MovementPool, Vec<MovementFrameUnit>, Vec2) {
    let far = ORIGIN + Vec2::new(6.0, 0.0);
    let mut pool = dense_pool();
    pool.slots.push(far);
    let units = vec![
        still(1, 0, ORIGIN + Vec2::new(-4.0, 0.0), 0.5),
        still(2, 0, ORIGIN, 2.5),
    ];
    (pool, units, far)
}

/// R18. A spare-slot search that reads more slots than one slice pays is resumable: each
/// slice stays within its allowance, the job keeps its kind and ages, and the result is
/// the slot of the unsliced search and of `nearest_free`.
#[test]
fn a_spare_search_larger_than_a_slice_resumes() {
    let (pool, units, far) = blocked_dense_pool();
    let mut index = super::pool_index::MovementPoolIndex::new(&pool, Vec2::splat(20.0));
    let points: Vec<_> = units
        .iter()
        .map(|unit| crate::grid::GridPoint {
            id: unit.unit_id,
            position: unit.position,
        })
        .collect();
    let grid = crate::grid::UniformGrid::new(&points, 5.0);
    let witness = vec![Vec::new(); units.len()];
    for slot in 0..pool.slots.len() {
        index.refresh_slot_clearance(slot, &units, &points, &grid, 2.5, false, &witness);
    }
    let claims = ClaimSet::default();
    let origin = units[0].position;
    let scan = EscapeScan {
        bounds: Vec2::splat(20.0),
        origin,
        radius: 0.5,
        slots: Some((7, &claims)),
        pool: Some(PoolSearch {
            index: &index,
            unit_id: 1,
            allowed: None,
            skip: None,
        }),
        bound: None,
        verify: true,
        pool_only: true,
    };
    let kind = RecoveryKind::Spare(7);
    let oracle = index.nearest_free(
        1,
        origin,
        7,
        None,
        &claims,
        0.5,
        |slot| slot.distance(units[1].position) >= 0.5 + units[1].radius,
        &std::cell::Cell::new(0),
    );
    assert_eq!(oracle, Some(far));
    let world = RecoveryWorld::new(None, None, 0.0);
    let mut counters = RecoveryCounters::default();
    let mut job = None;
    let mut slices = 0;
    let result = loop {
        let before = counters.owner_work[0];
        let point = row_turn(&world, &mut job, MIN_SLICE, &mut counters, |row| {
            row.search(kind, &scan, &units)
        });
        assert!(counters.owner_work[0] - before <= MIN_SLICE);
        slices += 1;
        assert!(slices < 10_000, "the search must end");
        match job {
            None => break point,
            Some(job) => {
                assert_eq!(point, None, "an unfinished search gives no slot");
                assert_eq!((job.kind, job.age), (kind, slices as u16));
            }
        }
    };
    assert!(slices > 8, "the search spans slices: {slices}");
    assert_eq!(result, oracle);
    assert_eq!(counters.spare_slot_work, counters.owner_work[0] as u64);
    // The slot that the mover leaves is never the answer, with or without a claim on it.
    let leaving = EscapeScan {
        pool: Some(PoolSearch {
            index: &index,
            unit_id: 1,
            allowed: None,
            skip: Some((7, far.x.to_bits(), far.y.to_bits())),
        }),
        ..scan
    };
    let point = row_turn(&world, &mut None, OWNER_WORK, &mut counters, |row| {
        row.search(kind, &leaving, &units)
    });
    assert_eq!(point, None);
}

fn schedule_row(row: usize, spare: bool, age: u16, overlapping: bool, x: f32) -> ScheduleRow {
    ScheduleRow {
        row,
        owner: 0,
        age,
        has_scan: false,
        overlapping,
        position: Vec2::new(x, 1.0),
        radius: 0.5,
        spare,
        retained: false,
    }
}

/// R19. More spare rows than one tick serves: blocked rows get the fresh slices first,
/// each spare row is served within `ceil(rows / served)` ticks, and no tick gives more
/// than the owner allowance.
#[test]
fn spare_rows_take_turns_after_blocked_rows() {
    // 40 blocked rows that sort last by every other key, and 600 older spare rows.
    let blocked_rows: Vec<_> = (0..40)
        .map(|row| schedule_row(row, false, 0, false, 1_000.0 + row as f32))
        .collect();
    let mut waiting: Vec<_> = (0..600)
        .map(|row| schedule_row(40 + row, true, 5, true, row as f32))
        .collect();
    let per_tick = (OWNER_WORK - OWNER_WORK / 2) / 2 / MIN_SLICE;
    let mut ticks = 0;
    while !waiting.is_empty() {
        let mut rows = blocked_rows.clone();
        rows.extend(waiting.iter().copied());
        let schedule = plan(rows, ticks, &mut RecoveryCounters::default());
        let total: usize = (0..640).map(|row| schedule.allowance(row)).sum();
        assert!(total <= OWNER_WORK, "{total}");
        // Blocked rows first: they take every fresh slice, and no spare row gets more
        // than the floor.
        let served_blocked = (0..40)
            .filter(|&row| schedule.allowance(row) >= super::recovery::FRESH_SLICE)
            .count();
        assert_eq!(served_blocked, 16);
        assert!(
            waiting.iter().all(
                |row| schedule.allowance(row.row) <= MIN_SLICE && !schedule.check_only(row.row)
            )
        );
        let before = waiting.len();
        // A served spare search ends; a row that waits is one tick older.
        waiting.retain(|row| schedule.allowance(row.row) == 0);
        for row in &mut waiting {
            row.age += 1;
        }
        assert_eq!(before - waiting.len(), per_tick.min(before), "tick {ticks}");
        ticks += 1;
    }
    assert_eq!(ticks as usize, 600_usize.div_ceil(per_tick));
}

/// R19, in the frame. 200 stalled movers with no free spare slot at first: more than one
/// tick serves. A mover that waits keeps its order, its stall state and a spare job;
/// every mover gets its answer in a few ticks (no slot, or the slot that an earlier
/// failure set free); no tick is above the owner allowance.
#[test]
fn stalled_movers_wait_for_their_spare_search_and_all_get_an_answer() {
    let mut units = Vec::new();
    let mut slots = Vec::new();
    for n in 0..200u32 {
        let at = Vec2::new(5.0 + (n % 20) as f32 * 3.0, 5.0 + (n / 20) as f32 * 3.0);
        let slot = at + Vec2::new(1.5, 1.5);
        units.push(stalled_in(8, 1 + n, at, slot));
        slots.push(slot);
    }
    let mut frame = frame(units);
    frame.bounds = Vec2::splat(80.0);
    frame.pools.insert(
        8,
        super::MovementPool {
            pool_id: 8,
            point: Vec2::new(30.0, 20.0),
            spacing: 1.1,
            primary_count: 200,
            slots,
        },
    );
    let mut answered = BTreeSet::new();
    let mut waited = 0;
    for tick in 0..12 {
        let (outputs, counters) = step_mode(&frame, None, false, tick);
        assert!(counters.owner_work[0] <= OWNER_WORK);
        assert_eq!(counters.spare_slot_work, counters.owner_work[0] as u64);
        for output in &outputs {
            let claimed = output
                .progress
                .as_ref()
                .is_some_and(|progress| progress.claim_count & 3 == 1);
            if answered.contains(&output.unit_id) {
                continue;
            }
            if output.failed_blocked || claimed {
                answered.insert(output.unit_id);
                assert!(output.recovery.is_none());
            } else {
                // No answer yet: the mover keeps its order, its stall state and its job.
                let progress = output.progress.as_ref().unwrap();
                waited += 1;
                assert_eq!(progress.activity, super::MovementActivity::Moving);
                assert!(progress.stalled_ticks >= 20, "tick {tick}");
                assert!(output.target.is_some());
                let job = output.recovery.as_deref().expect("a waiting spare job");
                assert_eq!(job.kind, RecoveryKind::Spare(8));
            }
        }
        advance(&mut frame, &outputs);
        if answered.len() == 200 {
            assert!(tick >= 1 && waited > 0, "200 rows need more than one tick");
            return;
        }
    }
    panic!("every stalled mover must get an answer: {}", answered.len());
}

fn plain_blocked(unit_id: u32, position: Vec2, target: Vec2) -> MovementFrameUnit {
    MovementFrameUnit {
        target: Some(target),
        plain_progress: Some(super::PlainMoveProgress {
            arrival_yield: false,
            original_point: target,
            slot: target,
            pool_slots: Vec::new(),
            gather_goal: None,
            activity: super::MovementActivity::Blocked,
            stalled_ticks: 0,
            best_distance: 0.0,
            claim_count: 3,
            failure_recorded: true,
        }),
        ..body(unit_id, 0, position, 0.5)
    }
}

/// The places of the three stalled movers of the mixed scene.
fn spare_place(n: u32) -> Vec2 {
    Vec2::new(10.0 + 8.0 * n as f32, 40.0)
}

/// An owner-0 scene with each class of recovery row: the physical escapes of
/// `escape_scene`, `group` blocked group movers of pool 7 inside a packed crowd, three
/// blocked plain rows with a retained target under a body, and three stalled movers of
/// pool 8 (one with an own body on its nearest spare slot).
fn mixed_scene(group: u32) -> MovementFrame {
    let mut units = escape_scene();
    let goal = Vec2::new(30.0, 6.0);
    for n in 0..49u32 {
        units.push(still(
            600 + n,
            0,
            Vec2::new(30.0 + (n % 7) as f32, 6.0 + (n / 7) as f32),
            0.5,
        ));
    }
    for n in 0..group {
        units.push(blocked(
            1_000 + n,
            Vec2::new(33.0 + (n % 10) as f32 * 0.07, 9.0 + (n / 10) as f32 * 0.07),
            goal,
        ));
    }
    units.push(still(810, 0, Vec2::new(47.0, 45.0), 0.5));
    for n in 0..3u32 {
        units.push(plain_blocked(
            800 + n,
            Vec2::new(44.0 + n as f32 * 0.1, 45.0),
            Vec2::new(47.0, 45.0),
        ));
    }
    let mut slots = Vec::new();
    for n in 0..3u32 {
        let at = spare_place(n);
        units.push(stalled_in(8, 900 + n, at, at + Vec2::new(4.0, 0.0)));
        slots.push(at + Vec2::new(4.0, 0.0));
    }
    for n in 0..3u32 {
        slots.push(spare_place(n) + Vec2::new(0.0, 1.5));
        slots.push(spare_place(n) + Vec2::new(0.0, -1.8));
    }
    units.push(still(950, 0, spare_place(0) + Vec2::new(0.0, 1.5), 0.5));
    let mut frame = frame(units);
    frame.bounds = Vec2::splat(60.0);
    frame.pools.insert(
        7,
        super::MovementPool {
            pool_id: 7,
            point: goal,
            spacing: 1.1,
            primary_count: group,
            slots: vec![goal],
        },
    );
    frame.pools.insert(
        8,
        super::MovementPool {
            pool_id: 8,
            point: spare_place(1),
            spacing: 1.1,
            primary_count: 3,
            slots,
        },
    );
    frame
}

/// Hidden owner-1 bodies for `mixed_scene`: the bodies of `hidden_bodies`, and patches of
/// tiny bodies inside the query boxes of the group crowd, the plain target and a spare
/// slot. None comes in contact range of an owner-0 mover.
fn mixed_hidden() -> Vec<MovementFrameUnit> {
    let mut units = hidden_bodies();
    let patches = [
        Vec2::new(30.42, 6.42),
        Vec2::new(47.0, 45.62),
        spare_place(1) + Vec2::new(0.62, 1.5),
        spare_place(2) + Vec2::new(-0.3, -2.45),
    ];
    for (patch, corner) in patches.into_iter().enumerate() {
        for n in 0..1_000u32 {
            units.push(still(
                30_000 + patch as u32 * 1_000 + n,
                1,
                corner + Vec2::new((n % 40) as f32 * 0.0022, (n / 40) as f32 * 0.0022),
                0.001,
            ));
        }
    }
    units
}

fn with_hidden(mut frame: MovementFrame, hidden: Vec<MovementFrameUnit>) -> MovementFrame {
    frame.units.extend(hidden);
    frame.units.sort_by_key(|unit| unit.unit_id);
    frame
}

/// True when a hidden body (owner 1) is in contact range of an owner-0 body that can move.
fn hidden_contact(frame: &MovementFrame) -> bool {
    let movers: Vec<_> = frame
        .units
        .iter()
        .filter(|unit| unit.owner_slot == 0 && unit.speed.is_some())
        .collect();
    frame
        .units
        .iter()
        .filter(|unit| unit.owner_slot == 1)
        .any(|hidden| {
            movers.iter().any(|mover| {
                mover.position.distance(hidden.position) <= mover.radius + hidden.radius + 0.11
            })
        })
}

/// R20. Hidden-world pairs with each class of recovery row (physical, group, plain,
/// spare): hidden bodies near and far, never in contact, change no owner output, job or
/// owner counter in 60 ticks, with and without the claim exchange. They change only the
/// physical work and the view work.
#[test]
fn hidden_bodies_out_of_contact_change_no_row_of_any_class() {
    let basis = Basis {
        permitted: BTreeSet::new(),
    };
    for exchange in [true, false] {
        let mut plain = mixed_scene(12);
        let mut hidden = with_hidden(mixed_scene(12), mixed_hidden());
        let mut seen = [0u64; 4];
        let mut unpaid = (0, 0);
        for tick in 0..60 {
            assert!(
                !hidden_contact(&hidden),
                "tick {tick}: the scene keeps apart"
            );
            let (a, ca) = step_mode(&plain, Some(&basis), exchange, tick);
            let (b, cb) = step_mode(&hidden, Some(&basis), exchange, tick);
            assert_eq!(own_outputs(&a), own_outputs(&b), "tick {tick}");
            assert_eq!(observable(&ca), observable(&cb), "tick {tick}");
            assert_eq!(ca.physical_reads, cb.physical_reads, "tick {tick}");
            seen[0] += ca.checks;
            seen[1] += ca.spare_slot_work;
            seen[2] += ca.slices;
            seen[3] += ca.pool_work;
            unpaid = (
                unpaid.0 + ca.physical_work + ca.view_work,
                unpaid.1 + cb.physical_work + cb.view_work,
            );
            advance(&mut plain, &a);
            advance(&mut hidden, &b);
        }
        eprintln!("HIDDENMIXED exchange={exchange} seen={seen:?} unpaid={unpaid:?}");
        assert!(seen.iter().all(|&count| count > 0), "{seen:?}");
        assert!(unpaid.1 > unpaid.0, "hidden bodies cost frame work only");
    }
}

/// R21. A hidden body in contact range of the actor at tick start gives the result of the
/// same body when visible, for the check of a retained target and for the spare-slot
/// search. A hidden body out of contact range gives the result of no body.
#[test]
fn a_contacted_hidden_body_counts_for_the_retained_check_and_the_spare_search() {
    let target = ORIGIN + Vec2::new(1.3, 0.0);
    // Blocks the target (0.354 from it); 1.079 from the actor: in contact range (1.101).
    let touching = ORIGIN + Vec2::new(1.05, 0.25);
    // Blocks the target (0.6 from it); 1.9 from the actor: out of contact range.
    let apart = target + Vec2::new(0.6, 0.0);
    let retained = |hidden_at: Option<Vec2>, visible: bool| {
        let mut units = vec![
            MovementFrameUnit {
                physical_escape: Some(PhysicalEscape {
                    target: Some(target),
                }),
                ..escaping(1, 0, ORIGIN, 0.5)
            },
            still(2, 0, ORIGIN + Vec2::new(-0.05, 0.0), 0.5),
        ];
        units.extend(hidden_at.map(|at| still(3, 1, at, 0.5)));
        let basis = Basis {
            permitted: BTreeSet::from_iter(visible.then_some((0, 3))),
        };
        let (outputs, counters) = step(&frame(units), &basis, 1);
        assert_eq!(counters.checks, 1);
        // A search slice runs only when the check of the retained target fails. (The
        // escape target of the output is no witness: the sweep guard restores the old
        // target when it clips the step.)
        (outputs[0].clone(), counters.slices)
    };
    let free = retained(None, false);
    assert_eq!(free.1, 0, "a clear retained target needs no search");
    assert_eq!(
        free.0.physical_escape,
        Some(PhysicalEscape {
            target: Some(target)
        })
    );
    assert_eq!(
        retained(Some(touching), false).1,
        1,
        "contact: the check fails"
    );
    assert_eq!(
        retained(Some(touching), false),
        retained(Some(touching), true)
    );
    assert_eq!(retained(Some(apart), false), free);
    assert_eq!(
        retained(Some(apart), true).1,
        1,
        "a visible body fails the check"
    );

    // The spare-slot search. 1.05 from the mover and 0.45 from the first spare slot.
    let touching = ORIGIN + Vec2::new(0.0, 1.05);
    for exchange in [true, false] {
        let spare = |hidden_at: Option<Vec2>, visible: bool| {
            let frame = spare_frame(
                hidden_at
                    .map(|at| still(50, 1, at, 0.5))
                    .into_iter()
                    .collect(),
            );
            let basis = Basis {
                permitted: BTreeSet::from_iter(visible.then_some((0, 50))),
            };
            let (outputs, _) = step_mode(&frame, Some(&basis), exchange, 1);
            outputs
                .into_iter()
                .find(|output| output.unit_id == 1)
                .unwrap()
        };
        let slot = |output: &MovementFrameOutput| output.progress.as_ref().unwrap().slot;
        assert_eq!(slot(&spare(None, false)), SPARES[0]);
        assert_eq!(slot(&spare(Some(touching), false)), SPARES[1]);
        assert_eq!(spare(Some(touching), false), spare(Some(touching), true));
        // On the first spare slot, 1.5 from the mover: out of contact range.
        assert_eq!(spare(Some(SPARES[0]), false), spare(None, false));
    }
}

/// The count of rows of each class at tick start: physical escape, blocked group, blocked
/// plain. The same rule as the schedule of the frame.
fn blocked_classes(frame: &MovementFrame) -> [u64; 3] {
    let mut classes = [0; 3];
    for unit in frame.units.iter().filter(|unit| unit.speed.is_some()) {
        let group = unit.progress.as_ref().is_some_and(|progress| {
            progress.activity == super::MovementActivity::Blocked
                && frame.pools.contains_key(&progress.pool_id)
        });
        let plain = unit
            .plain_progress
            .as_ref()
            .is_some_and(|progress| progress.activity == super::MovementActivity::Blocked);
        if unit.physical_escape.is_some() {
            classes[0] += 1;
        } else if group {
            classes[1] += 1;
        } else if plain {
            classes[2] += 1;
        }
    }
    classes
}

/// R22. A scene with blocked rows of each class and spare rows, with more work than one
/// owner allowance: the metered work of the owner is never above `OWNER_WORK`, and the
/// neighbourhood reads of each row are the fixed count of its class (physical escape 2
/// or 3, blocked group 2, blocked plain 1, spare 1).
#[test]
fn owner_work_and_neighbourhood_reads_stay_within_their_bounds() {
    let mut frame = mixed_scene(100);
    let mut peak = 0;
    let mut classes_seen = [0u64; 4];
    for tick in 0..80 {
        let [physical, group, plain] = blocked_classes(&frame);
        // A spare row: a group mover whose stall clock can be due, or that has a spare job.
        let spare = frame
            .units
            .iter()
            .filter(|unit| {
                unit.physical_escape.is_none()
                    && unit.plain_progress.is_none()
                    && unit.target.is_some()
                    && unit.progress.as_ref().is_some_and(|progress| {
                        progress.activity != super::MovementActivity::Blocked
                            && progress.claim_count & 3 < 3
                            && (progress.stalled_ticks >= 19
                                || frame.recovery.get(&unit.unit_id).is_some_and(|job| {
                                    job.kind == RecoveryKind::Spare(progress.pool_id)
                                }))
                    })
            })
            .count() as u64;
        super::pool_index::SLOT_CLEARANCE_READS.with(|count| count.set(0));
        let (outputs, counters) = step_mode(&frame, None, true, tick);
        // The slot refresh reads of all movers: an upper bound of those of recovery rows.
        let refresh = super::pool_index::SLOT_CLEARANCE_READS.with(|count| count.get());
        peak = peak.max(counters.owner_work[0]);
        assert!(counters.owner_work[0] <= OWNER_WORK, "tick {tick}");
        let least = 2 * physical + 2 * group + plain + spare;
        assert!(
            (least..=least + physical + refresh).contains(&counters.physical_reads),
            "tick {tick}: {} reads for {physical} physical, {group} group, {plain} plain, \
             {spare} spare rows",
            counters.physical_reads
        );
        assert_eq!(
            counters.frame_work,
            counters.physical_work + counters.view_work
        );
        for (seen, count) in classes_seen.iter_mut().zip([physical, group, plain, spare]) {
            *seen += count;
        }
        advance(&mut frame, &outputs);
    }
    eprintln!("MIXEDBOUND peak_owner_work={peak} class_row_ticks={classes_seen:?}");
    assert!(
        classes_seen.iter().all(|&count| count > 0),
        "{classes_seen:?}"
    );
    assert!(
        peak > OWNER_WORK / 2,
        "the scene uses the allowance: {peak}"
    );
}

/// R22, one row. The check and the search of one row turn share one allowance: a check
/// that ends does not give the search a new allowance.
#[test]
fn a_check_and_a_search_of_one_row_share_one_allowance() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    // A blocker on the target that is the first key of its cell, then a dense search.
    let mut units = vec![still(1, 0, ORIGIN, 0.5)];
    units.push(still(2, 0, target + Vec2::new(-0.2, 0.0), 0.5));
    for n in 0..2_000u32 {
        units.push(still(
            10 + n,
            0,
            ORIGIN
                + Vec2::new(
                    0.62 + (n % 20) as f32 * 0.0022,
                    0.55 + (n / 20) as f32 * 0.0022,
                ),
            0.001,
        ));
    }
    let claims = ClaimSet::default();
    let index = one_slot_pool(target);
    let scan = check_scan(false, &claims, &index, None);
    let world = RecoveryWorld::new(None, None, 0.0);
    let mut counters = RecoveryCounters::default();
    let mut job = None;
    let allowance = 256;
    let (checked, point) = row_turn(&world, &mut job, allowance, &mut counters, |row| {
        let checked = row.check(RecoveryKind::Physical, &scan, target, false, false, &units);
        (checked, row.search(RecoveryKind::Physical, &scan, &units))
    });
    assert_eq!(checked, Check::Blocked);
    assert_eq!(point, None);
    assert!(job.is_some_and(|job| job.scan.is_some_and(|scan| !scan.is_check())));
    assert_eq!((counters.checks, counters.slices), (1, 1));
    assert!(
        counters.owner_work[0] <= allowance,
        "{} > {allowance}",
        counters.owner_work[0]
    );
    assert!(
        counters.owner_work[0] > allowance - 32,
        "the search uses the rest"
    );
}

fn unit_mut(frame: &mut MovementFrame, unit_id: u32) -> &mut MovementFrameUnit {
    frame
        .units
        .iter_mut()
        .find(|unit| unit.unit_id == unit_id)
        .unwrap()
}

/// R23. A stop order, a new order and a death drop the job of a row: the paused check of
/// a retained target, and a waiting spare job. Only the unchanged order continues it.
#[test]
fn a_stop_order_a_new_order_and_a_death_drop_the_job() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    let mut frame = retained_group_scene(target);
    frame.pools.insert(
        9,
        super::MovementPool {
            pool_id: 9,
            point: target,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![target],
        },
    );
    let outputs = super::step_frame_at_tick(&frame, 0.05, 0);
    advance(&mut frame, &outputs);
    let paused = frame.recovery[&1];
    assert_eq!((paused.kind, paused.age), (RecoveryKind::Group(7), 1));
    assert!(
        paused
            .scan
            .is_some_and(|scan| scan.is_check() && scan.query.is_some())
    );
    let run = |change: &dyn Fn(&mut MovementFrame)| {
        let mut frame = frame.clone();
        change(&mut frame);
        let mut budget = super::sweep::SweepBudget::new(1);
        let outputs =
            super::step_frame_with_sweep_budget(&frame, 0.05, &Default::default(), &mut budget);
        let job = outputs
            .iter()
            .find(|output| output.unit_id == 1)
            .and_then(|output| output.recovery.as_deref().copied());
        (job, budget.recovery, outputs.len())
    };
    // The unchanged order resumes the cursor: the rest of the check is small.
    let (job, counters, _) = run(&|_| {});
    assert_eq!(job, None);
    assert!(counters.checks == 1 && counters.owner_work[0] < OWNER_WORK / 2);
    let resumed = counters.owner_work[0];
    // A stop order: no order, no blocked row, no job.
    let (job, counters, _) = run(&|frame| {
        let unit = unit_mut(frame, 1);
        unit.target = None;
        unit.progress = None;
    });
    assert_eq!(job, None);
    assert_eq!((counters.checks, counters.owner_work[0]), (0, 0));
    // A new order of another pool with the same target: the check starts again.
    let (job, counters, _) = run(&|frame| {
        unit_mut(frame, 1).progress.as_mut().unwrap().pool_id = 9;
    });
    let job = job.expect("the new check does not end in one tick");
    assert_eq!((job.kind, job.age), (RecoveryKind::Group(9), 1));
    assert!(
        counters.owner_work[0] > resumed,
        "no cursor of the old order"
    );
    // A death: the row is gone, its job has no reader.
    let (job, counters, rows) = run(&|frame| frame.units.retain(|unit| unit.unit_id != 1));
    assert_eq!(job, None);
    assert_eq!((counters.checks, rows), (0, frame.units.len() - 1));

    // A waiting spare job of pool 8.
    let waiting = RecoverySearch {
        kind: RecoveryKind::Spare(8),
        age: 4,
        scan: None,
    };
    let spare = |change: &dyn Fn(&mut MovementFrame)| {
        let mut frame = mixed_scene(1);
        frame.recovery.insert(900, waiting);
        // Only unit 901 has a free allowance in this probe: unit 900 must wait or end.
        change(&mut frame);
        let (outputs, counters) = step_mode(&frame, None, false, 1);
        let job = outputs
            .iter()
            .find(|output| output.unit_id == 900)
            .and_then(|output| output.recovery.as_deref().copied());
        (job, counters)
    };
    // The unchanged order ends its search in this tick (the job is served).
    let (job, counters) = spare(&|_| {});
    assert_eq!(job, None);
    let searched = counters.spare_slot_work;
    assert!(searched > 0);
    // A stop order.
    let (job, counters) = spare(&|frame| {
        let unit = unit_mut(frame, 900);
        unit.target = None;
        unit.progress = None;
    });
    assert_eq!(job, None);
    assert!(
        counters.spare_slot_work < searched,
        "no search for the stopped mover"
    );
    // A new order with a fresh stall clock: no spare search, no job.
    let (job, counters) = spare(&|frame| {
        let progress = unit_mut(frame, 900).progress.as_mut().unwrap();
        progress.stalled_ticks = 0;
        progress.best_distance = 100.0;
    });
    assert_eq!(job, None);
    assert!(counters.spare_slot_work < searched);
    // A death.
    let (job, _) = spare(&|frame| frame.units.retain(|unit| unit.unit_id != 900));
    assert_eq!(job, None);
}

/// R24. The mixed scene with hidden bodies, with reversed unit ids, reversed input rows
/// and swapped owner slots: the same physical outcome and the same owner work in each
/// tick. And the audit replay kernel, from jobs that went through the log encoding,
/// gives the outputs and the jobs of the live frame.
#[test]
fn mirrored_ids_rows_and_owners_and_the_replay_kernel_agree() {
    let scene = |reverse_ids: bool, swap_owner: bool| {
        // One hidden body in eight: each patch and the large far body are there.
        let hidden = mixed_hidden().into_iter().step_by(8).collect();
        let mut frame = with_hidden(mixed_scene(40), hidden);
        for unit in &mut frame.units {
            if reverse_ids {
                unit.unit_id = 90_000 - unit.unit_id;
            }
            if swap_owner {
                unit.owner_slot = 1 - unit.owner_slot;
            }
        }
        frame.units.sort_by_key(|unit| unit.unit_id);
        if reverse_ids {
            frame.units.reverse();
        }
        frame
    };
    let basis = Basis {
        permitted: BTreeSet::new(),
    };
    let run = |reverse_ids: bool, swap_owner: bool, replay: bool| {
        let mut frame = scene(reverse_ids, swap_owner);
        let owner = usize::from(swap_owner);
        let mut work = Vec::new();
        let mut jobs = 0;
        for tick in 0..40 {
            let (outputs, counters) = step(&frame, &basis, tick);
            work.push((
                counters.owner_work[owner],
                counters.slices,
                counters.completions,
                counters.checks,
                counters.spare_slot_work,
                counters.physical_reads,
            ));
            if replay {
                // The auditor reads the jobs from the log and runs the public kernel.
                let logged = serde_json::to_string(&frame.recovery).unwrap();
                let mut audited = frame.clone();
                audited.recovery = serde_json::from_str(&logged).unwrap();
                assert_eq!(audited.recovery, frame.recovery, "tick {tick}");
                jobs += audited.recovery.len();
                let replayed = super::step_frame_preserving_resting_at_tick_with_visibility(
                    &audited,
                    0.05,
                    &Default::default(),
                    tick,
                    &basis,
                );
                assert_eq!(replayed, outputs, "tick {tick}");
            }
            advance(&mut frame, &outputs);
        }
        if replay {
            assert!(jobs > 0, "the replay reads recovery jobs");
        }
        let mut positions: Vec<_> = frame
            .units
            .iter()
            .map(|unit| (unit.position.x.to_bits(), unit.position.y.to_bits()))
            .collect();
        positions.sort_unstable();
        (positions, work)
    };
    let reference = run(false, false, true);
    assert!(reference.1.iter().any(|tick| tick.3 > 0 && tick.4 > 0));
    assert_eq!(run(true, false, false), reference, "reversed ids and rows");
    assert_eq!(run(false, true, false), reference, "swapped owners");
    assert_eq!(run(true, true, false), reference, "both");
}

/// R15, the schedule. Rows with a retained target that get no search slice get the floor
/// for their check: no row on the way to its target waits for a slice, the floor starts
/// no search, and the owner total stays within `OWNER_WORK`.
#[test]
fn rows_with_a_retained_target_get_a_check_floor() {
    let rows: Vec<_> = (0..300)
        .map(|row| ScheduleRow {
            retained: row % 3 != 0,
            ..schedule_row(row, false, 0, true, row as f32)
        })
        .collect();
    let schedule = plan(rows.clone(), 3, &mut RecoveryCounters::default());
    let total: usize = (0..300).map(|row| schedule.allowance(row)).sum();
    assert!(total <= OWNER_WORK, "{total}");
    let floor = (OWNER_WORK - OWNER_WORK / 2) / 2 / MIN_SLICE;
    let mut on_floor = 0;
    for row in &rows {
        let allowance = schedule.allowance(row.row);
        if row.row < 16 {
            // The first rows in service order get the fresh slices.
            assert!(allowance >= super::recovery::FRESH_SLICE);
            assert!(!schedule.check_only(row.row));
        } else if row.retained && on_floor < floor {
            on_floor += 1;
            assert_eq!(allowance, MIN_SLICE, "row {}", row.row);
            assert!(schedule.check_only(row.row));
        } else {
            assert_eq!(allowance, 0, "row {}", row.row);
        }
    }
    assert_eq!(on_floor, floor);
}

/// R15, the schedule in the frame. Sixty blocked plain rows walk to clear retained
/// targets: more rows than fresh slices. Each row moves in each tick.
#[test]
fn rows_on_the_way_to_a_retained_target_do_not_stop() {
    let mut units = Vec::new();
    for n in 0..60u32 {
        let at = Vec2::new(5.0 + (n % 10) as f32 * 5.0, 5.0 + (n / 10) as f32 * 5.0);
        units.push(plain_blocked(1 + n, at, at + Vec2::new(2.0, 0.0)));
    }
    let mut frame = frame(units);
    frame.bounds = Vec2::splat(60.0);
    let start = frame.clone();
    for tick in 0..10 {
        let (outputs, counters) = step_mode(&frame, None, true, tick);
        assert!(counters.owner_work[0] <= OWNER_WORK);
        assert_eq!(counters.jobs, 0, "tick {tick}: no row waits");
        advance(&mut frame, &outputs);
    }
    for (unit, before) in frame.units.iter().zip(&start.units) {
        let moved = unit.position.x - before.position.x;
        assert!(
            (moved - 0.5).abs() < 1.0e-3,
            "unit {} moved {moved}",
            unit.unit_id
        );
    }
}

/// B1. A retained group target that the metered check reports as clear has no known body
/// at rest on it: the base test `target_is_blocked_by_stationary` is implied. Random
/// bodies round the target, at rest and moving, at the tolerance edges too.
#[test]
fn a_clear_retained_group_target_has_no_resting_body_on_it() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    let claims = ClaimSet::default();
    let index = one_slot_pool(target);
    let scan = check_scan(true, &claims, &index, None);
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 40) as f32 / (1u64 << 24) as f32
    };
    let (mut clear, mut resting_blocks, mut edge) = (0, 0, 0);
    for case in 0..3_000u32 {
        let mut units = vec![still(1, 0, ORIGIN, 0.5)];
        for n in 0..3u32 {
            let radius = [0.2, 0.5, 0.8][(case + n) as usize % 3];
            let angle = random() * std::f32::consts::TAU;
            // Every third case puts one body inside the tolerance band of the two tests.
            let distance = if case % 3 == 0 && n == 0 {
                0.5 + radius - [2.0e-5, 5.0e-6, 0.0, -1.0e-6][case as usize / 3 % 4]
            } else {
                0.5 + radius + (random() - 0.5) * 0.6
            };
            let mut other = still(
                2 + n,
                0,
                target + Vec2::from_angle(angle) * distance,
                radius,
            );
            if random() < 0.5 {
                // A mover is not at rest.
                other.speed = Some(1.0);
                other.target = Some(Vec2::splat(1.0));
            }
            units.push(other);
        }
        let points: Vec<_> = units
            .iter()
            .map(|unit| crate::grid::GridPoint {
                id: unit.unit_id,
                position: unit.position,
            })
            .collect();
        let grid = crate::grid::UniformGrid::new(&points, 1.6);
        let oracle = super::recovery_oracle::Oracle {
            units: &units,
            points: &points,
            grid: &grid,
            max_radius: 0.8,
            index: 0,
            known: &|_| true,
        };
        let world = RecoveryWorld::new(None, None, 0.0);
        let checked = row_turn(
            &world,
            &mut None,
            WIDE,
            &mut RecoveryCounters::default(),
            |row| row.check(RecoveryKind::Group(7), &scan, target, true, false, &units),
        );
        // The metered check is the base `is_free` (tolerance 0).
        assert_eq!(
            checked == Check::Clear,
            !oracle.occupied(target, 0.0),
            "case {case}"
        );
        if checked == Check::Clear {
            clear += 1;
            assert!(!oracle.blocked_by_stationary(target), "case {case}");
        }
        resting_blocks += usize::from(oracle.blocked_by_stationary(target));
        edge += usize::from(oracle.occupied(target, 0.0) && !oracle.occupied(target, 1.0e-5));
    }
    assert!(
        clear > 100 && resting_blocks > 100 && edge > 50,
        "{clear} {resting_blocks} {edge}"
    );
}

// ---- Repairs after the review of section 7 (counting and instruments). ----

/// One blocked group row (radius 0.3) on the way to a retained slot, inside a pool of hex
/// slots 0.7 apart, and one own body of radius 1.5 far away (the largest radius of the
/// frame sets the reach of the slot refresh). `walkers` ordinary movers are far away.
fn slot_refresh_scene(walkers: u32) -> (MovementFrame, Vec<Vec2>) {
    let target = ORIGIN + Vec2::new(2.8, 0.0);
    let mut mover = blocked(1, ORIGIN, target);
    mover.radius = 0.3;
    let mut units = vec![
        mover,
        still(2, 0, ORIGIN + Vec2::new(-0.3, 0.0), 0.3),
        still(3, 0, Vec2::new(50.0, 50.0), 1.5),
    ];
    for n in 0..walkers {
        let at = Vec2::new(5.0 + (n % 30) as f32 * 1.5, 30.0 + (n / 30) as f32 * 1.5);
        let mut walker = body(100 + n, 0, at, 0.3);
        walker.target = Some(at + Vec2::new(0.0, 5.0));
        units.push(walker);
    }
    let mut slots = vec![target];
    for j in -8i32..=8 {
        for i in -8i32..=8 {
            let slot =
                ORIGIN + Vec2::new(0.7 * (i as f32 + 0.5 * (j & 1) as f32), 0.606 * j as f32);
            if slot.distance(target) > 0.3 {
                slots.push(slot);
            }
        }
    }
    let mut frame = frame(units);
    frame.bounds = Vec2::splat(60.0);
    frame.pools.insert(
        7,
        super::MovementPool {
            pool_id: 7,
            point: target,
            spacing: 0.7,
            primary_count: 1,
            slots: slots.clone(),
        },
    );
    (frame, slots)
}

/// Review finding 1. The slot clearance refresh after the turn of a recovery row does one
/// read of the base grid for each pool slot within `radius + max radius` of its old or
/// new position. `physical_reads` holds them: it equals the two fixed reads of the row
/// plus a count of these slots that the test makes from the scene. The reads of ordinary
/// movers are not recovery reads, and a hidden body out of contact changes no count.
#[test]
fn the_slot_refresh_reads_of_a_recovery_row_are_counted() {
    let basis = Basis {
        permitted: BTreeSet::new(),
    };
    for fog in [false, true] {
        let (mut alone, slots) = slot_refresh_scene(0);
        let (mut crowded, _) = slot_refresh_scene(300);
        // A hidden body 1.2 from the row: inside the reach of the reads, out of contact.
        let mut dark = alone.clone();
        dark.units
            .push(still(10_000, 1, ORIGIN + Vec2::new(0.0, 1.2), 0.3));
        let mut reads = Vec::new();
        for tick in 0..3 {
            let old = alone.units[0].position;
            super::pool_index::SLOT_CLEARANCE_READS.with(|count| count.set(0));
            let (outputs, counters) = step_mode(&alone, fog.then_some(&basis), false, tick);
            let probe = super::pool_index::SLOT_CLEARANCE_READS.with(|count| count.get());
            let new = outputs[0].position;
            assert_ne!(new, old, "the row moves");
            // The filter of the slot grid: squared distance within the squared reach.
            let reach = 0.3_f32 + 1.5;
            let near = slots
                .iter()
                .filter(|slot| {
                    slot.distance_squared(old) <= reach * reach
                        || slot.distance_squared(new) <= reach * reach
                })
                .count() as u64;
            assert!(near > 10, "the scene has slots near the row: {near}");
            // Fixed reads: the schedule read and the exit read of a blocked group row.
            assert_eq!(counters.physical_reads, 2 + near, "fog {fog} tick {tick}");
            assert_eq!(probe, near, "only the recovery row moves");
            assert!(counters.physical_work >= counters.physical_reads);
            reads.push(counters.physical_reads);
            // Ordinary movers refresh slots too; those are not recovery reads.
            let (crowded_outputs, crowded_counters) =
                step_mode(&crowded, fog.then_some(&basis), false, tick);
            assert_eq!(crowded_counters.physical_reads, counters.physical_reads);
            // A hidden body out of contact changes no read count and no owner counter.
            let (dark_outputs, dark_counters) = step_mode(&dark, Some(&basis), false, tick);
            let (lit_outputs, lit_counters) = step_mode(&alone, Some(&basis), false, tick);
            assert_eq!(dark_counters.physical_reads, lit_counters.physical_reads);
            assert_eq!(observable(&dark_counters), observable(&lit_counters));
            assert_eq!(own_outputs(&dark_outputs), own_outputs(&lit_outputs));
            advance(&mut alone, &outputs);
            advance(&mut crowded, &crowded_outputs);
            advance(&mut dark, &dark_outputs);
        }
        eprintln!("SLOTREADS fog={fog} physical_reads={reads:?}");
    }
}

/// Review finding 2. Each move of a member of a built view costs two units of view work
/// (one remove and one insert): `view_work` is the build (one check for each row and one
/// insert for each member) plus twice the relocations that the view counts itself.
#[test]
fn view_relocations_are_counted_as_view_work() {
    let (mut frame, _) = slot_refresh_scene(300);
    let rows = frame.units.len() as u64;
    for tick in 0..3 {
        super::recovery_view::RELOCATIONS.with(|count| count.set(0));
        let (outputs, counters) = step_mode(&frame, None, false, tick);
        let relocations = super::recovery_view::RELOCATIONS.with(|count| count.get());
        assert!(relocations >= 250, "the walkers move: {relocations}");
        // One shared view: every row is a member.
        assert_eq!(
            counters.view_work,
            2 * rows + 2 * relocations,
            "tick {tick}"
        );
        assert_eq!(
            counters.frame_work,
            counters.physical_work + counters.view_work
        );
        advance(&mut frame, &outputs);
    }
}

/// Review finding 3 (B1, "with a stored scan"). A group row has a stored search scan and
/// a retained target that is blocked behind a cell larger than half the allowance. The
/// check runs first in each tick, cannot be paid and is not stored. It never counts as
/// clear (the test oracle of the frame checks each result), and it uses half the slice
/// at most, so the stored search continues and ends.
#[test]
fn an_unpaid_check_does_not_stop_or_replace_the_stored_search() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    let mut frame = retained_group_scene(target);
    // The blocker is the last key of the dense cell.
    frame
        .units
        .push(still(90_000, 0, target + Vec2::new(0.9, 0.0), 0.5));
    frame.units.sort_by_key(|unit| unit.unit_id);
    let mut with_stored_search = 0;
    let mut ended = None;
    for tick in 0..14 {
        let had_search = frame
            .recovery
            .get(&1)
            .is_some_and(|job| job.scan.is_some_and(|scan| !scan.is_check()));
        let (outputs, counters) = with_candidate_cap(1, || {
            let mut budget = super::sweep::SweepBudget::new(tick);
            let outputs =
                super::step_frame_with_sweep_budget(&frame, 0.05, &Default::default(), &mut budget);
            (outputs, budget.recovery)
        });
        assert!(counters.owner_work[0] <= OWNER_WORK);
        let mover = outputs.iter().find(|output| output.unit_id == 1).unwrap();
        if had_search {
            with_stored_search += 1;
            // The unpaid check takes half the slice at most: the search runs in the tick.
            assert_eq!((counters.checks, counters.slices), (1, 1), "tick {tick}");
            if mover.recovery.is_none() {
                // The search ended with an escape point that is not the blocked target.
                assert!(mover.target.is_some_and(|point| point != target));
                ended = Some(tick);
                break;
            }
            assert_eq!(mover.target, Some(target), "tick {tick}: no choice yet");
        }
        advance(&mut frame, &outputs);
    }
    assert!(with_stored_search >= 2, "{with_stored_search}");
    assert!(ended.is_some(), "the stored search must end");
}

/// The pool, the bodies and the far slot of `blocked_dense_pool`, with slot clearances.
fn blocked_dense_index() -> (
    super::pool_index::MovementPoolIndex,
    Vec<MovementFrameUnit>,
    Vec2,
) {
    let (pool, units, far) = blocked_dense_pool();
    let mut index = super::pool_index::MovementPoolIndex::new(&pool, Vec2::splat(20.0));
    let points: Vec<_> = units
        .iter()
        .map(|unit| crate::grid::GridPoint {
            id: unit.unit_id,
            position: unit.position,
        })
        .collect();
    let grid = crate::grid::UniformGrid::new(&points, 5.0);
    let witness = vec![Vec::new(); units.len()];
    for slot in 0..pool.slots.len() {
        index.refresh_slot_clearance(slot, &units, &points, &grid, 2.5, false, &witness);
    }
    (index, units, far)
}

fn spare_scan<'a>(
    origin: Vec2,
    claims: &'a ClaimSet,
    index: &'a super::pool_index::MovementPoolIndex,
) -> EscapeScan<'a> {
    EscapeScan {
        bounds: Vec2::splat(20.0),
        origin,
        radius: 0.5,
        slots: Some((7, claims)),
        pool: Some(PoolSearch {
            index,
            unit_id: 1,
            allowed: None,
            skip: None,
        }),
        bound: None,
        verify: true,
        pool_only: true,
    }
}

/// Review finding 5 (`spare_origin`). A stalled mover creeps while its spare search
/// spans many slices. The search keeps its origin while the mover is within its radius of
/// it, so it ends with the slot of the unsliced search. A mover that goes farther starts
/// a new search at its place.
#[test]
fn a_spare_search_keeps_its_origin_while_the_mover_creeps() {
    let (index, units, far) = blocked_dense_index();
    let claims = ClaimSet::default();
    let start = units[0].position;
    let kind = RecoveryKind::Spare(7);
    let world = RecoveryWorld::new(None, None, 0.0);
    let mut counters = RecoveryCounters::default();
    let mut job = None;
    let mut slices = 0u32;
    let mut paused = None;
    let result = loop {
        // The mover creeps 0.002 in each tick: 0.4 at most in this test.
        let position = start + Vec2::new(0.002 * slices as f32, 0.0);
        let point = row_turn(&world, &mut job, MIN_SLICE, &mut counters, |row| {
            let origin = row.spare_origin(kind, position, 0.5);
            if slices > 0 {
                assert_eq!(origin, start, "slice {slices}: the search keeps its origin");
            }
            row.search(kind, &spare_scan(origin, &claims, &index), &units)
        });
        slices += 1;
        assert!(slices < 200, "the search of a creeping mover must end");
        if job.is_none() {
            break point;
        }
        paused = job;
    };
    assert!(slices > 8, "the search spans slices: {slices}");
    assert_eq!(result, Some(far));
    // The mover is 0.6 from the origin of its stored search: more than its radius.
    let away = start + Vec2::new(0.6, 0.0);
    let origin = row_turn(&world, &mut paused, MIN_SLICE, &mut counters, |row| {
        row.spare_origin(kind, away, 0.5)
    });
    assert_eq!(origin, away);
}

/// Review finding 5 (the stale check). A row waits with the stored check of a retained
/// target. Then it has another target and no allowance. The job that it keeps holds no
/// cursor of the old target: the row is a fresh row for the schedule. With the same
/// target, the cursor stays.
#[test]
fn a_waiting_row_drops_the_check_of_a_point_that_it_left() {
    let target = ORIGIN + Vec2::new(2.5, 0.0);
    let units = dense_round(target);
    let claims = ClaimSet::default();
    let index = one_slot_pool(target);
    let scan = check_scan(false, &claims, &index, None);
    let kind = RecoveryKind::Physical;
    // The row is a schedule row: a row with no schedule read keeps no job at all.
    let world = RecoveryWorld::new(None, None, 0.0);
    let mut reads = ScheduleReads::default();
    reads.begin(0);
    world.set_reads(reads);
    for (next_target, keeps_cursor) in [(target, true), (ORIGIN + Vec2::new(0.0, 2.5), false)] {
        let mut counters = RecoveryCounters::default();
        let mut job = None;
        let first = row_turn(&world, &mut job, MIN_SLICE, &mut counters, |row| {
            row.check(kind, &scan, target, false, false, &units)
        });
        assert_eq!(first, Check::Pending);
        assert!(job.is_some_and(|job| job.scan.is_some_and(|scan| scan.is_check())));
        let waited = row_turn(&world, &mut job, 0, &mut counters, |row| {
            row.check(kind, &scan, next_target, false, false, &units)
        });
        assert_eq!(waited, Check::Pending);
        let job = job.expect("a row that waits keeps its job");
        assert_eq!(job.age, 2);
        assert_eq!(job.scan.is_some(), keeps_cursor);
    }
}

/// Review finding 7. A paused `Spare` job goes through the log encoding in the middle of
/// its search. The restored job continues as the original: the same slices, the same
/// work and the same slot.
#[test]
fn a_paused_spare_job_continues_the_same_after_a_json_round_trip() {
    let (index, units, far) = blocked_dense_index();
    let claims = ClaimSet::default();
    let origin = units[0].position;
    let kind = RecoveryKind::Spare(7);
    let scan = spare_scan(origin, &claims, &index);
    let world = RecoveryWorld::new(None, None, 0.0);
    // Runs the job to its end; `restore_at` sends it through JSON after that slice.
    let run = |restore_at: Option<u32>| {
        let mut counters = RecoveryCounters::default();
        let mut job: Option<RecoverySearch> = None;
        let mut trace = Vec::new();
        for slice in 1..10_000u32 {
            let point = row_turn(&world, &mut job, MIN_SLICE, &mut counters, |row| {
                row.search(kind, &scan, &units)
            });
            trace.push((counters.owner_work[0], counters.candidates, job));
            if job.is_none() {
                return (point, trace, counters);
            }
            if restore_at == Some(slice) {
                let stored = job.unwrap();
                let cursor = stored
                    .scan
                    .and_then(|scan| scan.pool)
                    .expect("a pool cursor");
                assert!(
                    stored.kind == kind && cursor.checked && !cursor.done && cursor.at > 0,
                    "the job is paused inside its slot search: {stored:?}"
                );
                let logged = serde_json::to_string(&stored).unwrap();
                let restored: RecoverySearch = serde_json::from_str(&logged).unwrap();
                assert_eq!(restored, stored);
                job = Some(restored);
            }
        }
        panic!("the search must end");
    };
    let reference = run(None);
    assert_eq!(reference.0, Some(far));
    assert!(reference.1.len() > 12, "{}", reference.1.len());
    for slice in [3, 6, 9] {
        assert_eq!(run(Some(slice)), reference, "restored after slice {slice}");
    }
}

// Design `2026-11-20-bounded-contact`: the schedule reads of a recovery row.

/// A crowd of `side * side` still tiny bodies (radius 0.001, 0.0021 apart, no overlap) at
/// `origin`, in owner 0 unless `foreign` gives another owner. The first `rows` bodies are
/// escaping rows of owner 0 (schedule rows of class physical).
fn tiny_crowd(
    side: usize,
    rows: usize,
    origin: Vec2,
    foreign: impl Fn(usize) -> u8,
) -> Vec<MovementFrameUnit> {
    (0..side * side)
        .map(|n| {
            let position = origin + Vec2::new((n % side) as f32, (n / side) as f32) * 0.0021;
            if n < rows {
                MovementFrameUnit {
                    speed: Some(1.0),
                    ..escaping(n as u32 + 1, 0, position, 0.001)
                }
            } else {
                still(n as u32 + 1, foreign(n), position, 0.001)
            }
        })
        .collect()
}

fn tiny_frame(units: Vec<MovementFrameUnit>, bounds: f32) -> MovementFrame {
    let mut frame = frame(units);
    frame.bounds = Vec2::splat(bounds);
    frame
}

fn step_open(frame: &MovementFrame) -> RecoveryCounters {
    let mut budget = super::sweep::SweepBudget::new(1);
    let _ = super::step_frame_with_sweep_budget(frame, 0.05, &Default::default(), &mut budget);
    budget.recovery
}

/// G1 with no fog: a recovery row reads no contact range. 4,096 tiny bodies lie within the
/// contact gap of each row; each schedule, exit and new-position read scans at most 16
/// cells and the bodies in them (red on `5f18ef1c`: one read scanned about 8,000 cells and
/// bodies). The test oracle compares each schedule read with a scan of every row.
#[test]
fn tiny_footprints_cost_no_contact_scan_without_fog() {
    let _ = super::recovery_oracle::take_schedule_compared();
    let counters = step_open(&tiny_frame(
        tiny_crowd(64, 100, Vec2::splat(1.0), |_| 0),
        4.0,
    ));
    assert_eq!(super::recovery_oracle::take_schedule_compared(), 100);
    assert_eq!(counters.contact_reads, 0);
    assert_eq!(counters.contact_work, 0);
    assert!(counters.physical_reads >= 200, "{counters:?}");
    // One cell holds at most one body (0.0021 apart, cells 0.002 wide).
    assert!(
        counters.physical_work <= 32 * counters.physical_reads,
        "reads {} work {}",
        counters.physical_reads,
        counters.physical_work
    );
}

/// G2 under fog: the contact read reads the hidden bodies of the owner only. A crowd of
/// 4,000 own and permitted foreign tiny bodies costs it nothing, and a hidden crowd away
/// from the rows costs it no body test. Hidden bodies in contact range are found (the
/// test oracle checks every contact set).
#[test]
fn own_and_visible_density_costs_no_contact_work_under_fog() {
    let side = 63;
    let origin = Vec2::splat(1.0);
    // Owner 1 for every third body; all of them are permitted.
    let crowd = tiny_crowd(side, 100, origin, |n| u8::from(n % 3 == 0));
    let permitted: BTreeSet<(u8, u32)> = crowd
        .iter()
        .filter(|unit| unit.owner_slot == 1)
        .map(|unit| (0, unit.unit_id))
        .collect();
    let basis = Basis { permitted };
    // Hidden bodies of owner 1: three in contact range of the first rows, and a dense
    // patch far from every row.
    let near: Vec<_> = (0..3)
        .map(|n| {
            still(
                50_000 + n,
                1,
                origin + Vec2::new(-0.05, 0.01 * n as f32),
                0.001,
            )
        })
        .collect();
    let far: Vec<_> = (0..90)
        .map(|n| {
            still(
                60_000 + n,
                1,
                Vec2::new(
                    3.0 + (n % 10) as f32 * 0.0021,
                    3.0 + (n / 10) as f32 * 0.0021,
                ),
                0.001,
            )
        })
        .collect();
    let rows_only: Vec<_> = crowd.iter().take(100).cloned().collect();
    let run = |units: Vec<MovementFrameUnit>| {
        let _ = super::recovery_oracle::take_schedule_compared();
        let (_, counters) = step(&tiny_frame(units, 4.0), &basis, 1);
        assert_eq!(super::recovery_oracle::take_schedule_compared(), 100);
        counters
    };
    let crowded = run([crowd.clone(), near.clone(), far.clone()].concat());
    let sparse = run([rows_only.clone(), near.clone(), far.clone()].concat());
    assert_eq!(crowded.contact_reads, 100);
    assert_eq!(
        (crowded.contact_reads, crowded.contact_work),
        (sparse.contact_reads, sparse.contact_work),
        "own and permitted bodies change no contact work"
    );
    // A far hidden patch adds no body test; it can change the cell count of a read (the
    // sparse scan of `grid`), at most 16 cells for each read.
    let without_far = run([rows_only.clone(), near.clone()].concat());
    assert!(crowded.contact_work <= without_far.contact_work + 100 * 16);
    // At most 16 cells and the 3 near hidden bodies for each read.
    assert!(
        crowded.contact_work <= 100 * (16 + 3),
        "{}",
        crowded.contact_work
    );
    // The near hidden bodies are contacts of the first rows: the hidden index is used.
    assert!(crowded.contact_work > 100, "{}", crowded.contact_work);
}

/// G1 at large finite coordinates: each read still scans at most 16 cells (red on
/// `5f18ef1c` for the f32 cell boxes of the audit probe).
#[test]
fn reads_at_large_coordinates_scan_at_most_16_cells() {
    for origin in [1.0e5_f32, 3.0e6] {
        let units: Vec<_> = (0..64)
            .map(|n| {
                let position =
                    Vec2::splat(origin) + Vec2::new((n % 8) as f32, (n / 8) as f32) * 0.5;
                if n < 8 {
                    escaping(n + 1, 0, position, 0.2)
                } else {
                    still(n + 1, 1, position, 0.2)
                }
            })
            .collect();
        let frame = tiny_frame(units, origin * 2.0);
        let counters = step_open(&frame);
        // Cells 0.4 wide, bodies 0.5 apart: at most one body in a cell.
        assert!(
            counters.physical_work <= 32 * counters.physical_reads,
            "origin {origin}: {counters:?}"
        );
        let (_, fog) = step(
            &frame,
            &Basis {
                permitted: BTreeSet::new(),
            },
            1,
        );
        assert_eq!(fog.contact_reads, 8);
        // Contact cells are 0.8 wide or more: at most 16 cells and 4 bodies in each.
        assert!(
            fog.contact_work <= 8 * (16 + 64),
            "origin {origin}: {fog:?}"
        );
    }
}

/// G3: an overlap read stops at the first overlap. 400 escaping bodies stacked on one spot
/// read a few bodies each, not the whole stack (red on `5f18ef1c`: each schedule read
/// tested every body of the stack).
#[test]
fn a_stack_of_escaping_bodies_reads_a_few_bodies_for_each_overlap_read() {
    let units: Vec<_> = (0..400)
        .map(|n| escaping(n + 1, 0, Vec2::splat(10.0), 0.5))
        .collect();
    let counters = step_open(&frame(units));
    assert!(counters.physical_reads >= 800, "{counters:?}");
    assert!(
        counters.physical_work <= 40 * counters.physical_reads,
        "reads {} work {}",
        counters.physical_reads,
        counters.physical_work
    );
}
