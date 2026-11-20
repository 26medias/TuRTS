//! Tests of the bounded blocked-recovery search (design `2026-11-18-bounded-recovery`).

use super::frame::{MovementFrame, MovementFrameOutput, MovementFrameUnit};
use super::recovery::{
    EscapeScan, FRESH_SLICE, OWNER_WORK, RecoveryCounters, SLICE_CANDIDATES, ScheduleRow,
    SliceOutcome, plan,
};
use super::recovery_view::{ClaimSet, Contacts, KnownSet, KnownView, body_key};
use super::search::MovementSearch;
use super::spread::nearest_angle_order;
use super::{MoveProgress, MovementActivity, MovementPool, RecoveryScan};
use bevy_math::Vec2;
use std::collections::BTreeMap;

fn body(unit_id: u32, position: Vec2, radius: f32) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id,
        owner_slot: 0,
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

    /// The known view of the mover (row 0): itself and the bodies that `known` admits.
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

    fn scan<'a>(
        &'a self,
        slots: Option<(u64, &'a ClaimSet)>,
        known: &'a dyn Fn(u32) -> bool,
        bound: Option<Vec2>,
        verify: bool,
    ) -> Scanner<'a> {
        Scanner {
            world: self,
            view: self.view(known),
            contacts: Contacts::default(),
            slots,
            bound,
            verify,
        }
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

/// One search of the mover of a `World` over its known view.
struct Scanner<'a> {
    world: &'a World,
    view: KnownView,
    contacts: Contacts,
    slots: Option<(u64, &'a ClaimSet)>,
    bound: Option<Vec2>,
    verify: bool,
}

impl Scanner<'_> {
    fn run(
        &self,
        previous: Option<RecoveryScan>,
        allowance: usize,
        owner: u8,
        counters: &mut RecoveryCounters,
    ) -> SliceOutcome {
        let mover = &self.world.units[0];
        let known = KnownSet {
            view: &self.view,
            contacts: &self.contacts,
            own_key: Some(body_key(mover.position, mover.radius)),
        };
        EscapeScan {
            bounds: self.world.bounds,
            origin: mover.position,
            radius: mover.radius,
            slots: self.slots,
            pool: None,
            bound: self.bound,
            verify: self.verify,
        }
        .run(&known, previous, allowance, owner, counters)
    }
}

/// Runs `body` with at most `cap` candidates started in each slice.
fn with_candidate_cap<T>(cap: usize, body: impl FnOnce() -> T) -> T {
    SLICE_CANDIDATES.with(|limit| limit.set(cap));
    let result = body();
    SLICE_CANDIDATES.with(|limit| limit.set(usize::MAX));
    result
}

/// The slice allowance of the one-candidate tests: large enough for any proof.
const WIDE: usize = 4_096;

const ORIGIN: Vec2 = Vec2::new(10.0, 10.0);

/// The six ring-1 candidates of a radius-0.5 mover at `ORIGIN`, in scan order.
fn ring_one() -> Vec<Vec2> {
    let spacing = 1.1;
    let h = 3.0_f32.sqrt() * 0.5;
    [
        (1.0, 0.0),
        (0.0, 1.0),
        (-1.0, 1.0),
        (-1.0, 0.0),
        (0.0, -1.0),
        (1.0, -1.0),
    ]
    .iter()
    .map(|&(i, j): &(f32, f32)| {
        Vec2::new(
            ORIGIN.x + spacing * (i + 0.5 * j),
            ORIGIN.y + spacing * (h * j),
        )
    })
    .collect()
}

/// Mover at `ORIGIN`; A (ring 1, side 0) blocked by body 2 at `a_blocker`; B (side 1)
/// free; the other four ring-1 candidates blocked; a far bystander (id 9) at `far`.
fn stale_world(a_blocker: Vec2, far: Vec2) -> World {
    let ring = ring_one();
    let mut units = vec![body(1, ORIGIN, 0.5), body(2, a_blocker, 0.5)];
    for (offset, point) in ring.iter().enumerate().skip(2) {
        units.push(body(3 + offset as u32, *point, 0.5));
    }
    units.push(body(9, far, 0.5));
    World::new(Vec2::splat(20.0), units)
}

fn run_to_end(
    worlds: &[World],
    allowance: usize,
    verify: bool,
    counters: &mut RecoveryCounters,
) -> Option<Vec2> {
    let known = |_: u32| true;
    let mut scan: Option<RecoveryScan> = None;
    with_candidate_cap(allowance, || {
        for tick in 0.. {
            let world = &worlds[tick.min(worlds.len() - 1)];
            match world
                .scan(None, &known, None, verify)
                .run(scan, WIDE, 0, counters)
            {
                SliceOutcome::Done(point) => return point,
                SliceOutcome::Paused(state) => scan = Some(state),
            }
            assert!(tick < 10_000, "the search must end");
        }
        unreachable!()
    })
}

/// T1. The first slice sees A blocked. Before the second slice, A's blocker leaves. A
/// continued cursor alone picks B; the verified job proves the region changed, restarts,
/// and picks A, the nearest point of the new world.
#[test]
fn stale_nearer_site_is_rejected_by_the_region_proof() {
    let ring = ring_one();
    let (a, b) = (ring[0], ring[1]);
    let before = stale_world(a, Vec2::new(2.0, 2.0));
    let after = stale_world(Vec2::new(3.0, 3.0), Vec2::new(2.0, 2.0));
    assert_eq!(
        after.oracle(&|_| true),
        Some(a),
        "A is the nearest free point now"
    );

    let mut counters = RecoveryCounters::default();
    let verified = run_to_end(&[before, after], 1, true, &mut counters);
    assert_eq!(verified, Some(a));
    assert_eq!(counters.invalidations, 1);

    // The same two slices with the proof off: the obsolete result survives. This is the
    // failing case of the handoff; it shows the test above can go red.
    let before = stale_world(a, Vec2::new(2.0, 2.0));
    let after = stale_world(Vec2::new(3.0, 3.0), Vec2::new(2.0, 2.0));
    let mut counters = RecoveryCounters::default();
    let unverified = run_to_end(&[before, after], 1, false, &mut counters);
    assert_eq!(
        unverified,
        Some(b),
        "without the proof the stale result wins"
    );
    assert_ne!(unverified, verified);
}

/// T2. A body outside the examined region moves between slices: no restart.
#[test]
fn unrelated_movement_does_not_restart_the_scan() {
    let ring = ring_one();
    let before = stale_world(ring[0], Vec2::new(2.0, 2.0));
    let after = stale_world(ring[0], Vec2::new(2.5, 3.0));
    let mut counters = RecoveryCounters::default();
    let result = run_to_end(&[before, after], 1, true, &mut counters);
    assert_eq!(result, Some(ring[1]));
    assert_eq!(counters.invalidations, 0);
    assert!(counters.slices >= 2, "the search spans slices");
}

/// T1b. A hidden body that is not admitted by `known` moves into the region: no restart,
/// no change of the public charge.
#[test]
fn hidden_bodies_change_neither_the_proof_nor_the_public_charge() {
    let ring = ring_one();
    let hidden = |id: u32| id != 9;
    let charge = |far: Vec2| {
        let worlds = [
            stale_world(ring[0], Vec2::new(2.0, 2.0)),
            stale_world(ring[0], far),
        ];
        let mut counters = RecoveryCounters::default();
        let mut scan = None;
        let mut result = None;
        with_candidate_cap(1, || {
            for world in worlds.iter().cycle().take(64) {
                match world
                    .scan(None, &hidden, None, true)
                    .run(scan, WIDE, 0, &mut counters)
                {
                    SliceOutcome::Done(point) => {
                        result = point;
                        break;
                    }
                    SliceOutcome::Paused(state) => scan = Some(state),
                }
            }
        });
        (
            result,
            counters.owner_work[0],
            counters.invalidations,
            counters.slices,
        )
    };
    // Body 9 is hidden. It moves next to the origin (inside the examined region).
    let away = charge(Vec2::new(2.0, 2.0));
    let near = charge(Vec2::new(10.3, 10.2));
    assert_eq!(away.0, near.0);
    assert_eq!(away.1, near.1, "public charge");
    assert_eq!(away.2, near.2, "invalidations");
    assert_eq!(away.3, near.3, "slices");
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn float(&mut self, high: f32) -> f32 {
        (self.next() % 1_000_000) as f32 / 1_000_000.0 * high
    }
}

/// T3. In a stable world, any slicing gives the stateless result: singleton (no slots)
/// and group escape (slots and a pool bound).
#[test]
fn sliced_search_equals_the_stateless_search_in_a_stable_world() {
    let mut rng = Lcg(7);
    let mut compared = 0;
    let mut multi_slice = 0;
    for case in 0..400 {
        let side = 6.0 + rng.float(10.0);
        let bounds = Vec2::splat(side);
        let count = 1 + rng.next() as usize % 60;
        let mut units = vec![body(1, Vec2::new(rng.float(side), rng.float(side)), 0.5)];
        for id in 0..count {
            let radius = [0.3, 0.5, 0.8][rng.next() as usize % 3];
            units.push(body(
                2 + id as u32,
                Vec2::new(rng.float(side), rng.float(side)),
                radius,
            ));
        }
        let world = World::new(bounds, units);
        let hidden_every = 2 + (case % 3) as u32;
        let known = move |id: u32| !id.is_multiple_of(hidden_every);
        let allowance = super::recovery::MIN_SLICE + rng.next() as usize % 48;
        // Odd cases also cap the candidates of a slice, so they pause between candidates
        // as well as inside a probe.
        let cap = if case % 2 == 1 {
            1 + rng.next() as usize % 4
        } else {
            usize::MAX
        };
        SLICE_CANDIDATES.with(|limit| limit.set(cap));

        // Singleton: oracle vs slices.
        let oracle = world.oracle(&known);
        let mut counters = RecoveryCounters::default();
        let mut scan = None;
        let sliced = loop {
            match world
                .scan(None, &known, None, true)
                .run(scan, allowance, 0, &mut counters)
            {
                SliceOutcome::Done(point) => break point,
                SliceOutcome::Paused(state) => scan = Some(state),
            }
        };
        assert_eq!(sliced, oracle, "case {case}: singleton");
        multi_slice += usize::from(counters.slices > 1);
        assert_eq!(counters.invalidations, 0);

        // Group escape: assigned slots on some lattice points and a pool bound.
        let spacing = 1.1;
        let origin = world.units[0].position;
        let mut assigned = ClaimSet::default();
        for step in 0..(rng.next() % 6) {
            let point = Vec2::new(origin.x + spacing * (1.0 + step as f32), origin.y);
            assigned.insert((7, point.x.to_bits(), point.y.to_bits()));
        }
        let bound = rng
            .next()
            .is_multiple_of(2)
            .then(|| Vec2::new(rng.float(side), rng.float(side)));
        let full = MovementSearch {
            bounds,
            units: &world.units,
            points: &world.points,
            grid: &world.grid,
            max_radius: world.max_radius,
        }
        .find_free_escape_destination(origin, 0.5, 0, 7, &assigned, &known);
        let mut scan = None;
        let sliced = loop {
            match world.scan(Some((7, &assigned)), &known, bound, true).run(
                scan,
                allowance,
                0,
                &mut counters,
            ) {
                SliceOutcome::Done(point) => break point,
                SliceOutcome::Paused(state) => scan = Some(state),
            }
        };
        let choose = |outside: Option<Vec2>| match (bound, outside) {
            (Some(pool), Some(point)) => {
                Some(if nearest_angle_order(pool, point, origin).is_le() {
                    pool
                } else {
                    point
                })
            }
            (Some(point), None) | (None, Some(point)) => Some(point),
            (None, None) => None,
        };
        // The base returns no outside point when the scan reaches the ring limit with an
        // unproved best; the bounded scan returns that best (every in-bounds point was
        // examined). Compare only where the base proved its answer.
        if full.is_some() || sliced.is_none() {
            assert_eq!(choose(sliced), choose(full), "case {case}: group");
        }
        SLICE_CANDIDATES.with(|limit| limit.set(usize::MAX));
        compared += 1;
    }
    assert_eq!(compared, 400);
    assert!(
        multi_slice > 100,
        "most cases must span slices: {multi_slice}"
    );
}

fn blocked(unit_id: u32, position: Vec2, radius: f32, pool: u64, goal: Vec2) -> MovementFrameUnit {
    MovementFrameUnit {
        target: Some(goal),
        progress: Some(MoveProgress {
            pool_id: pool,
            slot: goal,
            activity: MovementActivity::Blocked,
            pass_mode: true,
            stalling_on_group: false,
            stalled_ticks: 60,
            best_distance: 1.0,
            claim_count: 3,
            follow_origin: None,
        }),
        ..body(unit_id, position, radius)
    }
}

fn advance(frame: &mut MovementFrame, outputs: &[MovementFrameOutput]) {
    frame.recovery.clear();
    for (unit, output) in frame.units.iter_mut().zip(outputs) {
        assert_eq!(unit.unit_id, output.unit_id);
        unit.position = output.position;
        unit.target = output.target;
        unit.progress = output.progress.clone();
        unit.plain_progress = output.plain_progress.clone();
        unit.physical_escape = output.physical_escape.clone();
        if let Some(job) = output.recovery.as_deref() {
            frame.recovery.insert(output.unit_id, *job);
        }
    }
    let live: std::collections::BTreeSet<_> = frame
        .units
        .iter()
        .filter_map(|unit| unit.progress.as_ref().map(|progress| progress.pool_id))
        .collect();
    frame.pools.retain(|pool, _| live.contains(pool));
}

struct Run {
    ticks: usize,
    peak_work: usize,
    peak_actual: usize,
    peak_phase_bytes: usize,
    peak_retained_bytes: usize,
    candidates: u64,
    invalidations: u64,
    relaxed_commits: u64,
    max_age: u16,
    outputs: Vec<MovementFrameOutput>,
}

/// Steps until no row is blocked and overlapping, or `limit` ticks.
fn run_frame(
    frame: &mut MovementFrame,
    goal: Vec2,
    limit: usize,
    churn: impl Fn(&mut MovementFrame, usize),
) -> Run {
    let mut run = Run {
        ticks: 0,
        peak_work: 0,
        peak_actual: 0,
        peak_phase_bytes: 0,
        peak_retained_bytes: 0,
        candidates: 0,
        invalidations: 0,
        relaxed_commits: 0,
        max_age: 0,
        outputs: vec![],
    };
    for tick in 0..limit {
        let mut budget = super::sweep::SweepBudget::new(tick as u32);
        let outputs =
            super::step_frame_with_sweep_budget(frame, 0.05, &Default::default(), &mut budget);
        let recovery = &budget.recovery;
        run.peak_work = run.peak_work.max(recovery.peak_owner_work());
        run.peak_actual = run.peak_actual.max(recovery.peak_owner_actual());
        run.peak_phase_bytes = run.peak_phase_bytes.max(recovery.phase_bytes);
        run.peak_retained_bytes = run.peak_retained_bytes.max(recovery.retained_bytes);
        run.candidates += recovery.candidates;
        run.invalidations += recovery.invalidations;
        run.relaxed_commits += recovery.relaxed_commits;
        run.max_age = run.max_age.max(recovery.max_age);
        advance(frame, &outputs);
        run.outputs = outputs;
        run.ticks = tick + 1;
        churn(frame, tick);
        let unresolved = frame.units.iter().any(|unit| {
            unit.progress
                .as_ref()
                .is_some_and(|progress| progress.activity == MovementActivity::Blocked)
                && unit.target.is_none_or(|target| target == goal)
        });
        if !unresolved {
            break;
        }
    }
    run
}

/// The handoff stress scene: a packed 16 x 16 map and 100 overlapping blocked movers
/// outside it. No free point exists, so every search scans to its ring limit.
fn stress_frame(movers: usize) -> MovementFrame {
    let goal = Vec2::splat(0.5);
    let mut units = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            units.push(body(
                units.len() as u32 + 1,
                Vec2::new(x as f32 + 0.5, y as f32 + 0.5),
                0.6,
            ));
        }
    }
    for i in 0..movers {
        let position = Vec2::new((i % 32) as f32 + 0.5, 20.0 + (i / 32) as f32);
        units.push(blocked(units.len() as u32 + 1, position, 0.6, 7, goal));
    }
    MovementFrame {
        bounds: Vec2::splat(16.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(
            7,
            MovementPool {
                pool_id: 7,
                point: goal,
                spacing: 1.1,
                primary_count: movers as u32,
                slots: vec![goal],
            },
        )]),
        recovery: BTreeMap::new(),
    }
}

/// T4. Per-owner public charge stays within the bound in every tick; phase and retained
/// bytes stay small.
#[test]
fn stress_scene_charge_and_memory_stay_bounded() {
    let mut frame = stress_frame(100);
    // The base stateless search: candidates in one tick, for the report.
    let base: usize = {
        let world = World::new(frame.bounds, frame.units.clone());
        let mut count = 0usize;
        for row in 256..world.units.len() {
            let spacing = 2.0 * 0.6 + 0.1;
            let limit = super::spread::finite_hex_ring_limit(
                world.units[row].position,
                0.6,
                frame.bounds,
                spacing,
            );
            count += (1..=limit as usize).map(|ring| 6 * ring).sum::<usize>();
        }
        count
    };
    eprintln!(
        "SIZES recovery_search={} recovery_scan={} schedule_row={} frame_unit={} frame_output={}",
        std::mem::size_of::<super::RecoverySearch>(),
        std::mem::size_of::<RecoveryScan>(),
        std::mem::size_of::<ScheduleRow>(),
        std::mem::size_of::<MovementFrameUnit>(),
        std::mem::size_of::<MovementFrameOutput>(),
    );
    let run = run_frame(&mut frame, Vec2::splat(0.5), 120, |_, _| {});
    eprintln!(
        "STRESS base_candidates_one_tick={base} ticks={} peak_work={} peak_actual={} phase_bytes={} retained_bytes={} candidates={} max_age={}",
        run.ticks,
        run.peak_work,
        run.peak_actual,
        run.peak_phase_bytes,
        run.peak_retained_bytes,
        run.candidates,
        run.max_age
    );
    // Metered work never exceeds the owner allowance. The actual work adds one pool
    // precheck for each served pool search (2 + 3 local slots; this pool has 1 slot), and
    // no served row has less than `MIN_SLICE`.
    assert!(run.peak_work <= OWNER_WORK, "{}", run.peak_work);
    let served = OWNER_WORK / super::recovery::MIN_SLICE;
    assert!(
        run.peak_actual <= OWNER_WORK + served * 5,
        "{}",
        run.peak_actual
    );
    // Phase memory: the schedule, one owner view (an entry and a cell for each body at
    // most) and the claim set.
    let units = frame.units.len();
    assert!(
        run.peak_phase_bytes <= units * 256,
        "{} > {}",
        run.peak_phase_bytes,
        units * 256
    );
    assert!(
        run.peak_retained_bytes
            <= 100 * (std::mem::size_of::<super::RecoverySearch>() + std::mem::size_of::<u32>())
    );
}

/// A packed stationary crowd of `side x side` bodies with `movers` blocked bodies stacked
/// at its centre. The nearest free points are outside the crowd.
fn crowd_frame(side: usize, movers: usize) -> MovementFrame {
    let bounds = Vec2::splat(side as f32 + 12.0);
    let mut units = Vec::new();
    let start = 6.0;
    for y in 0..side {
        for x in 0..side {
            let mut stationary = body(
                units.len() as u32 + 1,
                Vec2::new(start + x as f32 + 0.5, start + y as f32 + 0.5),
                0.5,
            );
            stationary.speed = None;
            units.push(stationary);
        }
    }
    let centre = Vec2::splat(start + side as f32 * 0.5 + 0.25);
    let goal = crowd_goal();
    for i in 0..movers {
        let offset = Vec2::new((i % 4) as f32 * 0.01, (i / 4) as f32 * 0.01);
        units.push(blocked(
            units.len() as u32 + 1,
            centre + offset,
            0.5,
            7,
            goal,
        ));
    }
    MovementFrame {
        bounds,
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(
            7,
            MovementPool {
                pool_id: 7,
                point: goal,
                spacing: 1.1,
                primary_count: movers as u32,
                slots: vec![goal],
            },
        )]),
        recovery: BTreeMap::new(),
    }
}

/// The pool slot of the crowd movers: under a crowd body, so it is never a free escape.
fn crowd_goal() -> Vec2 {
    Vec2::splat(6.5)
}

fn movers_escaped(frame: &MovementFrame, first_mover: usize) -> usize {
    frame.units[first_mover..]
        .iter()
        .filter(|unit| {
            unit.progress.as_ref().is_none()
                || unit.target.is_some_and(|target| target != crowd_goal())
        })
        .count()
}

/// T5. A deep stable crowd: every mover gets an escape point within a bounded number of
/// ticks, and the charge stays within the bound.
#[test]
fn deep_stable_crowd_resolves_every_mover() {
    for (side, movers) in [(24, 8), (40, 4)] {
        let mut frame = crowd_frame(side, movers);
        let first = side * side;
        let run = run_frame(&mut frame, crowd_goal(), 400, |_, _| {});
        eprintln!(
            "CROWD side={side} movers={movers} ticks={} peak_work={} peak_actual={} candidates={} invalidations={} relaxed_commits={} max_age={}",
            run.ticks,
            run.peak_work,
            run.peak_actual,
            run.candidates,
            run.invalidations,
            run.relaxed_commits,
            run.max_age
        );
        assert_eq!(movers_escaped(&frame, first), movers, "side {side}");
        assert!(run.ticks < 400);
        assert!(run.peak_work <= OWNER_WORK);
    }
}

/// T6. The crowd keeps moving by a tiny step each tick (all bodies stay where they block).
/// The proof fails, the jobs relax, and every mover still commits a free point.
#[test]
fn churning_crowd_relaxes_and_still_resolves() {
    let side = 30;
    let movers = 24;
    let mut frame = crowd_frame(side, movers);
    let first = side * side;
    let run = run_frame(&mut frame, crowd_goal(), 400, |frame, tick| {
        let delta = if tick % 2 == 0 { 1.0e-3 } else { -1.0e-3 };
        for unit in &mut frame.units[..side * side] {
            unit.position.x += delta;
        }
    });
    eprintln!(
        "CHURN ticks={} peak_work={} invalidations={} relaxed_commits={} max_age={}",
        run.ticks, run.peak_work, run.invalidations, run.relaxed_commits, run.max_age
    );
    assert_eq!(movers_escaped(&frame, first), movers);
    assert!(run.invalidations > 0, "the churn must break the proof");
    for unit in &frame.units[first..] {
        if let Some(target) = unit.target {
            let others = frame
                .units
                .iter()
                .filter(|other| other.unit_id != unit.unit_id);
            for other in others.take(first) {
                assert!(
                    target.distance(other.position) >= unit.radius + other.radius - 1.0e-3,
                    "a committed escape must be free when committed: {target:?}"
                );
            }
        }
    }
}

/// T6b. Every slice sees the blockers a little moved. The proof fails two times, the job
/// relaxes, and it commits a point that is free in the last world.
#[test]
fn a_job_under_permanent_churn_relaxes_and_commits_a_free_point() {
    let mut units = vec![body(1, ORIGIN, 0.5)];
    for y in 0..9 {
        for x in 0..9 {
            units.push(body(
                2 + y * 9 + x,
                Vec2::new(6.0 + x as f32, 6.0 + y as f32),
                0.5,
            ));
        }
    }
    let worlds: Vec<World> = (0..2)
        .map(|shift| {
            let mut moved = units.clone();
            for unit in &mut moved[1..] {
                unit.position.x += shift as f32 * 1.0e-3;
            }
            World::new(Vec2::splat(20.0), moved)
        })
        .collect();
    let known = |_: u32| true;
    // 400: the proof is cheap and fails by churn. `MIN_SLICE`: the proof costs more than
    // half the slice, so the job relaxes by cost.
    for allowance in [400, super::recovery::MIN_SLICE] {
        let mut counters = RecoveryCounters::default();
        let mut scan = None;
        let mut result = None;
        for slice in 0..10_000 {
            let world = &worlds[slice % 2];
            match world
                .scan(None, &known, None, true)
                .run(scan, allowance, 0, &mut counters)
            {
                SliceOutcome::Done(point) => {
                    result = point.map(|point| (point, slice % 2));
                    break;
                }
                SliceOutcome::Paused(state) => scan = Some(state),
            }
        }
        let (point, last) =
            result.unwrap_or_else(|| panic!("a relaxed job must finish: {counters:?}"));
        if allowance == 400 {
            assert!(counters.invalidations >= 2, "{counters:?}");
        } else {
            assert!(counters.relaxations >= 1, "{counters:?}");
        }
        assert_eq!(counters.relaxed_commits, 1);
        let world = &worlds[last];
        assert!(
            world.units[1..]
                .iter()
                .all(|other| point.distance(other.position) >= 1.0 - 1.0e-5),
            "the relaxed point must be free in the world of its commit"
        );
    }
}

/// T7. The schedule gives the same allowance to the same physical row under any unit id
/// order. It serves overlapping jobs first, then the oldest job.
#[test]
fn schedule_is_id_free_and_oldest_first() {
    // An old job with no overlap does not search in this tick, so a young overlapping
    // job gets the continuation budget. The same rule applies to fresh rows.
    let row = |row, age, has_scan, overlapping, x| ScheduleRow {
        row,
        owner: 0,
        age,
        has_scan,
        overlapping,
        position: Vec2::new(x, 1.0),
        radius: 0.5,
    };
    let mut counters = RecoveryCounters::default();
    let tiers = plan(
        vec![
            row(0, 50, true, false, 1.0),
            row(1, 1, true, true, 2.0),
            row(2, 30, false, false, 3.0),
        ]
        .into_iter()
        .chain((3..19).map(|id| row(id, 0, false, true, 4.0 + id as f32)))
        .collect(),
        0,
        &mut counters,
    );
    assert_eq!(tiers.allowance(0), 0, "old scan job with no overlap");
    assert!(tiers.allowance(1) > 0, "young overlapping scan job");
    assert_eq!(tiers.allowance(2), 0, "old fresh row with no overlap");
    assert!((3..19).all(|id| tiers.allowance(id) == FRESH_SLICE));

    let rows: Vec<ScheduleRow> = (0..12)
        .map(|row| ScheduleRow {
            row,
            owner: (row % 2) as u8,
            age: (row % 5) as u16,
            has_scan: row % 3 != 0,
            overlapping: row % 4 != 1,
            position: Vec2::new((row * 7 % 11) as f32, (row * 5 % 7) as f32),
            radius: 0.5,
        })
        .collect();
    let mut counters = RecoveryCounters::default();
    let reference = plan(rows.clone(), 0, &mut counters);
    // Permute the rows: a different id order gives a different row order.
    let order: Vec<usize> = (0..12).rev().collect();
    let permuted: Vec<ScheduleRow> = order
        .iter()
        .enumerate()
        .map(|(new_row, &old)| ScheduleRow {
            row: new_row,
            ..rows[old]
        })
        .collect();
    let other = plan(permuted, 0, &mut counters);
    for (new_row, &old) in order.iter().enumerate() {
        assert_eq!(reference.allowance(old), other.allowance(new_row));
    }
    for owner in 0..2u8 {
        let continuing: Vec<_> = rows
            .iter()
            .filter(|row| row.owner == owner && row.has_scan)
            .collect();
        let first = continuing
            .iter()
            .map(|row| (row.overlapping, row.age))
            .max()
            .unwrap();
        for row in continuing {
            let served = reference.allowance(row.row) > 0;
            assert_eq!(
                served,
                (row.overlapping, row.age) == first,
                "row {}",
                row.row
            );
        }
        let total: usize = rows
            .iter()
            .filter(|row| row.owner == owner)
            .map(|row| reference.allowance(row.row))
            .sum();
        assert!(total <= OWNER_WORK, "owner {owner}: {total}");
    }
}

/// T7b. Stacked identical rows share one budget: the total stays within the owner budget.
#[test]
fn a_stacked_batch_shares_one_budget() {
    let rows: Vec<ScheduleRow> = (0..500)
        .map(|row| ScheduleRow {
            row,
            owner: 0,
            age: 3,
            has_scan: true,
            overlapping: true,
            position: Vec2::splat(5.0),
            radius: 0.5,
        })
        .collect();
    let schedule = plan(rows, 0, &mut RecoveryCounters::default());
    let total: usize = (0..500).map(|row| schedule.allowance(row)).sum();
    assert!(total <= OWNER_WORK);
    assert!((0..500).all(|row| schedule.allowance(row) == schedule.allowance(0)));
}

/// T8. The same frame steps to the same outputs and jobs.
#[test]
fn recovery_is_deterministic() {
    let run = |_: ()| {
        let mut frame = crowd_frame(24, 6);
        run_frame(&mut frame, crowd_goal(), 30, |_, _| {});
        frame
    };
    assert_eq!(run(()), run(()));
}

/// T9. A small scene that ends in one slice keeps the base result (no new latency).
#[test]
fn small_scene_ends_in_the_first_tick() {
    let mut frame = crowd_frame(4, 1);
    let first = 16;
    let run = run_frame(&mut frame, crowd_goal(), 3, |_, _| {});
    assert_eq!(run.ticks, 1);
    assert_eq!(movers_escaped(&frame, first), 1);
    let _ = run.outputs;
}

/// One complete search: (result, public charge, actual charge, visibility checks,
/// invalidations, slices).
fn trace(
    worlds: &[World],
    known: &dyn Fn(u32) -> bool,
    allowance: usize,
) -> (Option<Vec2>, usize, usize, u64, u64, u64) {
    let mut counters = RecoveryCounters::default();
    let mut scan = None;
    for slice in 0..100_000 {
        let world = &worlds[slice % worlds.len()];
        match world
            .scan(None, known, None, true)
            .run(scan, allowance, 0, &mut counters)
        {
            SliceOutcome::Done(point) => {
                return (
                    point,
                    counters.owner_work[0],
                    counters.owner_actual[0],
                    counters.visibility_checks,
                    counters.invalidations,
                    counters.slices,
                );
            }
            SliceOutcome::Paused(state) => scan = Some(state),
        }
    }
    panic!("the search must end");
}

/// A packed 9 x 9 crowd round `ORIGIN` with free points outside it; body ids start at 2.
fn crowd_units() -> Vec<MovementFrameUnit> {
    let mut units = vec![body(1, ORIGIN, 0.5)];
    for y in 0..9 {
        for x in 0..9 {
            units.push(body(
                2 + y * 9 + x,
                Vec2::new(6.0 + x as f32, 6.0 + y as f32),
                0.5,
            ));
        }
    }
    units
}

/// T1c. Hidden bodies next to tested candidates, and a large hidden body that changes the
/// frame maximum radius, change neither the result nor the public charge, the proofs or
/// the slices. Catches a public charge or a digest membership that reads hidden facts.
#[test]
fn hidden_bodies_and_a_large_hidden_radius_leave_the_public_trace_unchanged() {
    const HIDDEN: u32 = 1_000;
    let known = |id: u32| id < HIDDEN;
    let bounds = Vec2::splat(20.0);
    let plain = World::new(bounds, crowd_units());
    let mut near = crowd_units();
    // Hidden bodies on candidate points of rings 1 to 3 (inside the crowd).
    for (offset, point) in ring_one().iter().enumerate() {
        near.push(body(
            HIDDEN + offset as u32,
            *point + Vec2::new(0.05, 0.0),
            0.5,
        ));
        near.push(body(
            HIDDEN + 10 + offset as u32,
            ORIGIN + (*point - ORIGIN) * 2.0,
            0.5,
        ));
    }
    let near = World::new(bounds, near);
    let mut large = crowd_units();
    large.push(body(HIDDEN + 50, Vec2::new(17.0, 3.0), 2.5));
    let large = World::new(bounds, large);
    assert!(large.max_radius > plain.max_radius);
    for allowance in [60, 400] {
        let reference = trace(std::slice::from_ref(&plain), &known, allowance);
        assert!(reference.5 > 1, "the search must span slices");
        for (name, world) in [("near", &near), ("large", &large)] {
            let other = trace(std::slice::from_ref(world), &known, allowance);
            assert_eq!(other.0, reference.0, "{name}: result");
            assert_eq!(other.1, reference.1, "{name}: public charge");
            assert_eq!(other.4, reference.4, "{name}: invalidations");
            assert_eq!(other.5, reference.5, "{name}: slices");
        }
        // Alternate the plain and the hidden worlds between slices: no restart.
        let mixed = trace(
            &[World::new(bounds, crowd_units()), near_copy(&near)],
            &known,
            allowance,
        );
        assert_eq!(mixed.4, 0, "hidden motion must not break the proof");
        assert_eq!(mixed.0, reference.0);
    }
}

fn near_copy(world: &World) -> World {
    World::new(world.bounds, world.units.clone())
}

/// T3b. The grid insertion order (the row order of the bodies) changes neither the result
/// nor any charge or visit count. Catches a free check that stops at the first blocker.
#[test]
fn grid_insertion_order_changes_no_result_or_charge() {
    let known = |id: u32| !id.is_multiple_of(7);
    // A large far body makes the grid cells 5 units wide, so each cell holds many bodies
    // and the visit order inside a cell differs between the three orders.
    let mut units = crowd_units();
    units.push(body(500, Vec2::new(18.0, 2.0), 2.5));
    let forward = World::new(Vec2::splat(20.0), units.clone());
    let mut reversed_units = vec![units[0].clone()];
    reversed_units.extend(units[1..].iter().rev().cloned());
    let reversed = World::new(Vec2::splat(20.0), reversed_units);
    let mut rng = Lcg(11);
    let mut shuffled_units = units[1..].to_vec();
    for index in (1..shuffled_units.len()).rev() {
        let other = rng.next() as usize % (index + 1);
        shuffled_units.swap(index, other);
    }
    shuffled_units.insert(0, units[0].clone());
    let shuffled = World::new(Vec2::splat(20.0), shuffled_units);
    for allowance in [super::recovery::MIN_SLICE, 100, usize::MAX] {
        let reference = trace(std::slice::from_ref(&forward), &known, allowance);
        for world in [&reversed, &shuffled] {
            assert_eq!(
                trace(std::slice::from_ref(world), &known, allowance),
                reference
            );
        }
    }
}

/// T7c. Rows that are admitted by state but overlap nothing at tick start never take a
/// fresh slice before a row that overlaps: the searcher escapes in the first tick.
#[test]
fn idle_admitted_rows_do_not_delay_an_overlapping_searcher() {
    let mut units = Vec::new();
    // 40 idle body-escape rows with smaller x than the searcher, far from every body.
    for i in 0..40 {
        let mut idle = body(
            i + 1,
            Vec2::new(1.0 + (i % 10) as f32 * 2.0, 2.0 + (i / 10) as f32 * 2.0),
            0.5,
        );
        idle.physical_escape = Some(super::PhysicalEscape { target: None });
        units.push(idle);
    }
    let mut searcher = body(100, Vec2::new(30.0, 30.0), 0.5);
    searcher.physical_escape = Some(super::PhysicalEscape { target: None });
    units.push(searcher);
    let mut blocker = body(101, Vec2::new(30.3, 30.0), 0.5);
    blocker.speed = None;
    units.push(blocker);
    let frame = MovementFrame {
        bounds: Vec2::splat(40.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::new(),
        recovery: BTreeMap::new(),
    };
    let mut budget = super::sweep::SweepBudget::new(1);
    let outputs =
        super::step_frame_with_sweep_budget(&frame, 0.05, &Default::default(), &mut budget);
    assert_eq!(budget.recovery.waits, 0, "{:?}", budget.recovery);
    let moved = outputs.iter().find(|output| output.unit_id == 100).unwrap();
    assert_ne!(
        moved.position,
        Vec2::new(30.0, 30.0),
        "the searcher must move in tick 1"
    );
}

#[test]
fn recovery_physical_outcomes_do_not_depend_on_unit_ids() {
    fn run(swapped: bool, a: Vec2, b: Vec2) -> [Vec2; 2] {
        let mut left = body(if swapped { 2 } else { 1 }, a, 0.5);
        let mut right = body(if swapped { 1 } else { 2 }, b, 0.5);
        left.physical_escape = Some(super::PhysicalEscape { target: None });
        right.physical_escape = Some(super::PhysicalEscape { target: None });
        let mut obstacle = body(3, Vec2::new(10.0, 10.0), 0.5);
        obstacle.speed = None;
        let mut units = vec![left, right, obstacle];
        units.sort_by_key(|unit| unit.unit_id);
        let mut frame = MovementFrame {
            bounds: Vec2::splat(20.0),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
            recovery: BTreeMap::new(),
        };
        let mut outputs = Vec::new();
        for tick in 0..30 {
            let mut budget = super::sweep::SweepBudget::new(tick);
            outputs =
                super::step_frame_with_sweep_budget(&frame, 0.05, &Default::default(), &mut budget);
            advance(&mut frame, &outputs);
        }
        let pos = |id| {
            outputs
                .iter()
                .find(|out| out.unit_id == id)
                .unwrap()
                .position
        };
        [
            pos(if swapped { 2 } else { 1 }),
            pos(if swapped { 1 } else { 2 }),
        ]
    }
    for i in 0..17 {
        for j in 0..17 {
            let a = Vec2::new(9.3 + i as f32 * 0.1, 9.9);
            let b = Vec2::new(9.3 + j as f32 * 0.1, 10.1);
            if a == b {
                continue;
            }
            assert_eq!(run(false, a, b), run(true, a, b), "a={a:?} b={b:?}");
        }
    }
}

#[test]
fn stacked_blocked_crowd_keeps_unordered_targets_and_positions_under_id_swap() {
    let mut a = crowd_frame(2, 2);
    let n = a.units.len();
    a.units[n - 1].position = a.units[n - 2].position;
    let mut b = a.clone();
    b.units[n - 2].unit_id = n as u32;
    b.units[n - 1].unit_id = n as u32 - 1;
    b.units.sort_by_key(|u| u.unit_id);
    for tick in 0..200 {
        let mut x = super::sweep::SweepBudget::new(tick);
        let mut y = super::sweep::SweepBudget::new(tick);
        let ax = super::step_frame_with_sweep_budget(&a, 0.05, &Default::default(), &mut x);
        let by = super::step_frame_with_sweep_budget(&b, 0.05, &Default::default(), &mut y);
        let get = |out: &[MovementFrameOutput], id: u32| {
            out.iter().find(|o| o.unit_id == id).unwrap().clone()
        };
        for (i, j) in [(n as u32 - 1, n as u32), (n as u32, n as u32 - 1)] {
            let left = get(&ax, i);
            let right = get(&by, j);
            assert_eq!(
                left.position, right.position,
                "tick={tick} ids={i} {j} targets={:?} {:?}",
                left.target, right.target
            );
        }
        let target_set = |rows: &[MovementFrameOutput]| {
            let mut targets = [rows[n - 2].target, rows[n - 1].target];
            targets.sort_by(|a, b| match (a, b) {
                (Some(a), Some(b)) => a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)),
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
            targets
        };
        assert_eq!(target_set(&ax), target_set(&by), "tick={tick}");
        advance(&mut a, &ax);
        advance(&mut b, &by);
    }
}
