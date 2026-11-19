//! Tests of the actual-work recovery bound (design `2026-11-19-recovery-actual-work`).

use super::frame::{MovementFrame, MovementFrameOutput, MovementFrameUnit};
use super::recovery::{
    EscapeScan, MIN_SLICE, OWNER_WORK, RecoveryCounters, ScheduleRow, SliceOutcome, plan,
};
use super::recovery_view::{Contacts, KnownSet, KnownView, body_key};
use super::search::MovementSearch;
use super::{MovementVisibility, PhysicalEscape, RecoveryScan};
use bevy_math::Vec2;
use std::collections::{BTreeMap, BTreeSet};

const ORIGIN: Vec2 = Vec2::new(10.0, 10.0);

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
        };
        EscapeScan {
            bounds: self.bounds,
            origin: mover.position,
            radius: mover.radius,
            slots: None,
            pool: None,
            bound: None,
            verify: true,
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
