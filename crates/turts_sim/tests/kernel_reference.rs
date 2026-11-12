//! The reference harness of the one step function (pre-MP1 slice, task A2a; design section 2 row
//! A2a and section 9 items 8, 9 and 18; plan refinements R9 and R11).
//!
//! `kernel_matches_reference_bit_for_bit` runs the live kernel `step_frame_preserving_resting`
//! and the reference copy of the old kernel (`movement/reference.rs`, the text at the base of A2a)
//! on the same seeded frames, chained for 300 ticks (the outputs of a frame are the inputs of the
//! next one), and compares every f32 of every output with `to_bits`. `0.0` and `-0.0` differ
//! here, because `movement/mod.rs` and `movement/spread.rs` use `atan2` and `total_cmp`.
//!
//! The scenario of seed `s`:
//! - 1 to 5,000 random movers (the table in `movers_for`) in a map that grows with the count;
//! - 0 to 20 statics (units with no speed) with a radius from 0.5 to 8;
//! - a `preserve_resting` set that is never empty;
//! - frames with a unit that has no progress, a unit with a pool, a resting unit that is kept
//!   (an attack-point unit that resumes its slot after a combat pause), a follow unit, a
//!   `Blocked` unit, a unit in a physical escape, and (from the chain) a unit with a plain
//!   progress. The kinds are counted over the run, and the test fails if one kind never
//!   appears;
//! - four units next to the map edge whose target holds a `0.0` or a `-0.0` coordinate, so that
//!   the arrival (`position = target`) writes a zero of each sign;
//! - a counter for each phase of the step (`probe::PhaseCalls`), each above 0 over the run;
//! - five anchor frames that reach branches the random seeds do not (the QA review of A2a): the
//!   late arrival (the step ends exactly on the target after the projection and the clamp), with
//!   and without a pool; two units whose projection meets a separation of 0 (the unit ids decide
//!   the direction); and an early arrival that only the clause `distance <= 0.025` decides, with
//!   and without a pool. Each anchor must raise its own count, and each count must be above 0 over
//!   the run.
//!
//! This file does not name the plain-progress type, which task A2b2 deletes (R11); it compares
//! that field through its `Debug` text, which prints each f32 as the shortest text that reads
//! back to the same bits (`-0.0` is distinct from `0.0`).
//!
//! The exact release command that every later task of the slice (A2b1, A2b2, A4, A3) and the
//! slice regression must run, until A3 has merged:
//!
//!     cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference
//!
//! `cargo test -p turts_sim --locked` runs 0 tests of this file, because the feature is off. A
//! debug build with the feature (the CI form `cargo test --workspace --locked`) runs the first 16
//! seeds with at most 30 movers, and the five anchor frames. The test
//! `reference_helpers_are_unchanged` pins the helper files that the reference copy calls.
#![cfg(feature = "test-probes")]

use std::collections::{BTreeMap, BTreeSet};

use turts_sim::Vec2;
use turts_sim::movement::reference::reference_step_frame_preserving_resting;
use turts_sim::movement::{
    FollowMoveOrigin, MoveProgress, MovementActivity, MovementFrame, MovementFrameOutput,
    MovementFrameUnit, MovementInput, MovementPool, PhysicalEscape, step,
    step_frame_preserving_resting,
};

/// The seeds of a release run. A debug build runs the first 16 (see `seeds`).
const SEEDS: u64 = 64;
const TICKS: usize = 300;
const DT: f32 = 0.05;
const TICK_HZ: u16 = 20;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(0x9e37_79b9_7f4a_7c15 ^ seed.wrapping_mul(0x2545_f491_4f6c_dd1d) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn seeds() -> u64 {
    if cfg!(debug_assertions) { 16 } else { SEEDS }
}

/// The number of random movers of a seed: 1 to 5,000. A debug build caps it at 30 and runs 16
/// seeds: a debug run of 64 seeds with a cap of 300 took 168 s, with a cap of 30 it took 71 s.
fn movers_for(seed: u64) -> usize {
    let n = match seed {
        1..=8 => seed as usize,
        9..=60 => 10 + ((seed * 47) % 290) as usize,
        61 => 800,
        62 => 1500,
        63 => 2500,
        _ => 5000,
    };
    if cfg!(debug_assertions) { n.min(30) } else { n }
}

/// The frame of a seed, and the `preserve_resting` set.
fn start_frame(seed: u64) -> (MovementFrame, BTreeSet<u32>) {
    let mut rng = Rng::new(seed);
    let movers = movers_for(seed);
    let statics = (seed * 7 % 21) as usize;
    let side = (60.0_f32).max(((movers as f32) * 40.0).sqrt() * 1.6);
    let bounds = Vec2::new(side, side * 0.75);
    // One pool for about each 30 movers; the pool point and its slots are inside the map.
    let pool_count = movers / 30 + 1;
    let mut pools = BTreeMap::new();
    for pool_id in 1..=pool_count as u64 {
        let point = Vec2::new(
            rng.range(10.0, bounds.x - 10.0),
            rng.range(10.0, bounds.y - 10.0),
        );
        let slots: Vec<Vec2> = (0..24)
            .map(|index| {
                let angle = index as f32 * std::f32::consts::FRAC_PI_6 * 0.5;
                point + Vec2::new(angle.cos(), angle.sin()) * (1.1 + 0.55 * (index / 12) as f32)
            })
            .collect();
        pools.insert(
            pool_id,
            MovementPool {
                pool_id,
                point,
                spacing: 1.1,
                primary_count: 12,
                slots,
            },
        );
    }
    let mut units: Vec<MovementFrameUnit> = Vec::new();
    let mut preserve = BTreeSet::new();
    let mut id = 0u32;
    let mut next_id = || {
        id += 1;
        id
    };
    let free_position = |rng: &mut Rng| {
        Vec2::new(
            rng.range(2.0, bounds.x - 2.0),
            rng.range(2.0, bounds.y - 2.0),
        )
    };
    // Statics: 0 to 20, radius 0.5 to 8.
    for _ in 0..statics {
        let radius = rng.range(0.5, 8.0);
        units.push(MovementFrameUnit {
            unit_id: next_id(),
            owner_slot: 1,
            unit_type: 9,
            position: Vec2::new(
                rng.range(radius, bounds.x - radius),
                rng.range(radius, bounds.y - radius),
            ),
            radius,
            speed: None,
            target: None,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        });
    }
    let pool_unit = |rng: &mut Rng,
                     unit_id: u32,
                     pool_id: u64,
                     position: Vec2,
                     activity: MovementActivity,
                     moving: bool| {
        let pool = &pools[&pool_id];
        let slot = pool.slots[rng.below(pool.slots.len() as u64) as usize];
        MovementFrameUnit {
            unit_id,
            owner_slot: (pool_id % 2) as u8,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(rng.range(2.0, 5.0)),
            target: moving.then_some(slot),
            progress: Some(MoveProgress {
                pool_id,
                slot,
                activity,
                pass_mode: rng.below(8) == 0,
                stalling_on_group: false,
                stalled_ticks: rng.below(6) as u16,
                best_distance: position.distance(slot) + rng.range(0.0, 2.0),
                claim_count: rng.below(3) as u8,
                follow_origin: None,
            }),
            plain_progress: None,
            physical_escape: None,
        }
    };
    // The random movers.
    for _ in 0..movers {
        let unit_id = next_id();
        let kind = rng.below(100);
        let pool_id = 1 + rng.below(pool_count as u64);
        let position = free_position(&mut rng);
        let unit = match kind {
            // A pooled unit that moves to its slot.
            40..=64 => pool_unit(
                &mut rng,
                unit_id,
                pool_id,
                position,
                MovementActivity::Moving,
                true,
            ),
            // A pooled unit that rests on its slot; some are kept, some follow.
            65..=71 => {
                let slot_of = |p: &MovementPool| p.slots[(unit_id as usize) % p.slots.len()];
                let at = slot_of(&pools[&pool_id]);
                let mut unit = pool_unit(
                    &mut rng,
                    unit_id,
                    pool_id,
                    at,
                    MovementActivity::Resting,
                    false,
                );
                unit.progress.as_mut().unwrap().slot = at;
                if kind.is_multiple_of(3) {
                    unit.progress.as_mut().unwrap().follow_origin = Some(FollowMoveOrigin {
                        target_unit_id: 1 + rng.below(unit_id as u64) as u32,
                        goal: at,
                    });
                }
                if kind.is_multiple_of(2) {
                    preserve.insert(unit_id);
                }
                unit
            }
            // An attack-point unit that resumes its slot after a combat pause: resting
            // progress, a target, and a place in the `preserve_resting` set.
            72..=76 => {
                let mut unit = pool_unit(
                    &mut rng,
                    unit_id,
                    pool_id,
                    position,
                    MovementActivity::Resting,
                    true,
                );
                unit.target = Some(unit.progress.as_ref().unwrap().slot);
                preserve.insert(unit_id);
                unit
            }
            // A blocked pooled unit that looks for a free place.
            77..=81 => pool_unit(
                &mut rng,
                unit_id,
                pool_id,
                position,
                MovementActivity::Blocked,
                true,
            ),
            // A unit in a physical escape, on top of another unit when one exists.
            82..=85 => {
                let mut unit = free_unit(&mut rng, unit_id, position, bounds, false);
                unit.physical_escape = Some(PhysicalEscape {
                    target: (kind.is_multiple_of(2)).then(|| free_position(&mut rng)),
                });
                if let Some(other) = units.last() {
                    unit.position = other.position + Vec2::new(0.3, 0.2);
                }
                unit
            }
            // An idle unit with no target.
            86..=88 => free_unit(&mut rng, unit_id, position, bounds, false),
            // A unit with no progress that has a target (the largest share).
            _ => free_unit(&mut rng, unit_id, position, bounds, true),
        };
        units.push(unit);
    }
    // The units that every seed holds, so that no seed is a world with nothing in it: an
    // attack-point unit in the preserve set, a follow unit, and four edge units whose target has a
    // zero of each sign. The zero units start 0.1 from the edge and reach their target in one step.
    let anchor_pool = 1u64;
    let anchor_slot = pools[&anchor_pool].slots[3];
    let mut anchor = pool_unit(
        &mut rng,
        next_id(),
        anchor_pool,
        anchor_slot,
        MovementActivity::Resting,
        false,
    );
    preserve.insert(anchor.unit_id);
    anchor.progress.as_mut().unwrap().slot = anchor_slot;
    units.push(anchor);
    let mut follow = pool_unit(
        &mut rng,
        next_id(),
        anchor_pool,
        pools[&anchor_pool].slots[5],
        MovementActivity::Resting,
        false,
    );
    follow.progress.as_mut().unwrap().slot = pools[&anchor_pool].slots[5];
    follow.progress.as_mut().unwrap().follow_origin = Some(FollowMoveOrigin {
        target_unit_id: 1,
        goal: pools[&anchor_pool].slots[5],
    });
    units.push(follow);
    for (position, target) in [
        (Vec2::new(0.1, 5.0), Vec2::new(-0.0, 5.0)),
        (Vec2::new(0.1, 8.0), Vec2::new(0.0, 8.0)),
        (Vec2::new(5.0, 0.1), Vec2::new(5.0, -0.0)),
        (Vec2::new(8.0, 0.1), Vec2::new(8.0, 0.0)),
    ] {
        let mut unit = free_unit(&mut rng, next_id(), position, bounds, false);
        unit.speed = Some(4.0);
        unit.radius = 0.5;
        unit.target = Some(target);
        units.push(unit);
    }
    // The units of the random draw do not cover the left edge strip, so the edge units have room.
    units.sort_unstable_by_key(|unit| unit.unit_id);
    assert!(
        !preserve.is_empty(),
        "the preserve_resting set is not empty"
    );
    (
        MovementFrame {
            bounds,
            tick_hz: TICK_HZ,
            units,
            pools,
        },
        preserve,
    )
}

fn free_unit(
    rng: &mut Rng,
    unit_id: u32,
    position: Vec2,
    bounds: Vec2,
    moving: bool,
) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id,
        owner_slot: 0,
        unit_type: 2,
        position,
        radius: [0.4, 0.5, 0.5, 0.8, 1.0][rng.below(5) as usize],
        speed: Some(rng.range(1.0, 6.0)),
        target: moving.then(|| {
            Vec2::new(
                rng.range(2.0, bounds.x - 2.0),
                rng.range(2.0, bounds.y - 2.0),
            )
        }),
        progress: None,
        plain_progress: None,
        physical_escape: None,
    }
}

/// The next frame: the outputs become the inputs. An idle unit with no progress gets a new
/// target now and then, as the synthetic units of `--synthetic-units` do.
fn next_frame(
    frame: &MovementFrame,
    outputs: &[MovementFrameOutput],
    rng: &mut Rng,
) -> MovementFrame {
    let mut next = frame.clone();
    next.units.sort_unstable_by_key(|unit| unit.unit_id);
    for (unit, output) in next.units.iter_mut().zip(outputs) {
        assert_eq!(unit.unit_id, output.unit_id);
        unit.position = output.position;
        unit.target = output.target;
        unit.progress = output.progress.clone();
        unit.plain_progress = output.plain_progress.clone();
        unit.physical_escape = output.physical_escape.clone();
        if unit.speed.is_some()
            && unit.target.is_none()
            && unit.progress.is_none()
            && unit.plain_progress.is_none()
            && rng.below(12) == 0
        {
            unit.target = Some(Vec2::new(
                rng.range(2.0, frame.bounds.x - 2.0),
                rng.range(2.0, frame.bounds.y - 2.0),
            ));
        }
    }
    next
}

fn bits(point: Vec2) -> [u32; 2] {
    [point.x.to_bits(), point.y.to_bits()]
}

fn bits_of(point: Option<Vec2>) -> Option<[u32; 2]> {
    point.map(bits)
}

/// A row of every f32 of a `MoveProgress`, as bits, with the other fields as they are.
fn progress_row(progress: &Option<MoveProgress>) -> String {
    match progress {
        None => "none".to_string(),
        Some(p) => format!(
            "pool {} slot {:?} {:?} pass {} group {} stalled {} best {:#010x} claims {} follow {:?}",
            p.pool_id,
            bits(p.slot),
            p.activity,
            p.pass_mode,
            p.stalling_on_group,
            p.stalled_ticks,
            p.best_distance.to_bits(),
            p.claim_count,
            p.follow_origin.map(|f| (f.target_unit_id, bits(f.goal)))
        ),
    }
}

/// The first difference between two outputs of one unit, as text, or `None` when they are equal
/// bit for bit.
fn difference(new: &MovementFrameOutput, old: &MovementFrameOutput) -> Option<String> {
    let mut fields: Vec<(&str, String, String)> = vec![
        ("unit_id", new.unit_id.to_string(), old.unit_id.to_string()),
        (
            "position",
            format!("{:?}", bits(new.position)),
            format!("{:?}", bits(old.position)),
        ),
        (
            "target",
            format!("{:?}", bits_of(new.target)),
            format!("{:?}", bits_of(old.target)),
        ),
        (
            "progress",
            progress_row(&new.progress),
            progress_row(&old.progress),
        ),
        // `Debug` of an f32 is the shortest text that reads back to the same bits, and it
        // prints `-0.0` apart from `0.0`. The type is not named here (plan R11).
        (
            "plain_progress",
            format!("{:?}", new.plain_progress),
            format!("{:?}", old.plain_progress),
        ),
        (
            "physical_escape",
            format!(
                "{:?}",
                new.physical_escape.as_ref().map(|e| bits_of(e.target))
            ),
            format!(
                "{:?}",
                old.physical_escape.as_ref().map(|e| bits_of(e.target))
            ),
        ),
        (
            "failed_blocked",
            new.failed_blocked.to_string(),
            old.failed_blocked.to_string(),
        ),
        (
            "probe.separated",
            new.probe.separated.to_string(),
            old.probe.separated.to_string(),
        ),
        (
            "probe.was_projected",
            format!("{:?}", new.probe.was_projected),
            format!("{:?}", old.probe.was_projected),
        ),
    ];
    fields
        .drain(..)
        .find(|(_, a, b)| a != b)
        .map(|(name, a, b)| format!("field {name}: live {a} reference {b}"))
}

#[derive(Default, Debug)]
struct Seen {
    frames: usize,
    unit_frames: usize,
    no_progress_moving: usize,
    pooled_moving: usize,
    resting_kept: usize,
    follow: usize,
    blocked: usize,
    escape: usize,
    plain: usize,
    zero_sign_arrivals: usize,
    /// A step that ended exactly on the target after the projection and the clamp
    /// (`frame.rs` "Late arrival"), for a unit with a pool and for a unit with no pool.
    late_arrival_pooled: usize,
    late_arrival_free: usize,
    /// A projection whose position equals the position of another unit exactly (the degenerate
    /// direction of the hard projection, decided by the unit ids).
    same_point: usize,
    /// An early arrival decided by the clause `distance <= 0.025` alone (the distance is above
    /// one step), for a unit with a pool and for a unit with no pool.
    slow_clause_pooled: usize,
    slow_clause_free: usize,
    // The calls of each phase: arrival, steering, projection, stall, fallback.
    phases: [u64; 5],
}

impl Seen {
    fn count(
        &mut self,
        frame: &MovementFrame,
        outputs: &[MovementFrameOutput],
        keep: &BTreeSet<u32>,
    ) {
        self.frames += 1;
        for (unit, output) in frame.units.iter().zip(outputs) {
            self.unit_frames += 1;
            let is_pool = output.progress.is_some();
            if unit.speed.is_some() && unit.target.is_some() && unit.progress.is_none() {
                self.no_progress_moving += 1;
            }
            if is_pool && output.target.is_some() {
                self.pooled_moving += 1;
            }
            if output.progress.as_ref().is_some_and(|p| {
                p.activity == MovementActivity::Resting && keep.contains(&output.unit_id)
            }) {
                self.resting_kept += 1;
            }
            if output
                .progress
                .as_ref()
                .is_some_and(|p| p.follow_origin.is_some())
            {
                self.follow += 1;
            }
            if output
                .progress
                .as_ref()
                .is_some_and(|p| p.activity == MovementActivity::Blocked)
            {
                self.blocked += 1;
            }
            if unit.physical_escape.is_some() {
                self.escape += 1;
            }
            if output.plain_progress.is_some() {
                self.plain += 1;
            }
            let zero = |v: f32| v == 0.0;
            if output.target.is_none()
                && unit.target.is_some_and(|t| zero(t.x) || zero(t.y))
                && output.position == unit.target.unwrap()
            {
                self.zero_sign_arrivals += 1;
            }
            let p = output.probe.phases;
            if let (Some(speed), Some(target)) = (unit.speed, unit.target) {
                let pooled = unit.progress.is_some();
                if p.steering == 1 && output.target.is_none() && !output.failed_blocked {
                    if pooled {
                        self.late_arrival_pooled += 1;
                    } else {
                        self.late_arrival_free += 1;
                    }
                }
                let distance = unit.position.distance(target);
                if p.arrival == 1
                    && p.steering == 0
                    && output.target.is_none()
                    && distance > speed * DT
                    && distance <= 0.025
                {
                    if pooled {
                        self.slow_clause_pooled += 1;
                    } else {
                        self.slow_clause_free += 1;
                    }
                }
                if p.projection == 1 && !pooled {
                    let landing = step(
                        &MovementInput {
                            position: unit.position,
                            target,
                            speed,
                        },
                        DT,
                    )
                    .position;
                    if frame
                        .units
                        .iter()
                        .any(|other| other.unit_id != unit.unit_id && other.position == landing)
                    {
                        self.same_point += 1;
                    }
                }
            }
            for (slot, n) in self.phases.iter_mut().zip([
                p.arrival,
                p.steering,
                p.projection,
                p.stall,
                p.fallback,
            ]) {
                *slot += u64::from(n);
            }
        }
    }
}

impl Seen {
    /// Add the counts of another run.
    fn merge(&mut self, other: &Seen) {
        self.frames += other.frames;
        self.unit_frames += other.unit_frames;
        self.no_progress_moving += other.no_progress_moving;
        self.pooled_moving += other.pooled_moving;
        self.resting_kept += other.resting_kept;
        self.follow += other.follow;
        self.blocked += other.blocked;
        self.escape += other.escape;
        self.plain += other.plain;
        self.zero_sign_arrivals += other.zero_sign_arrivals;
        self.late_arrival_pooled += other.late_arrival_pooled;
        self.late_arrival_free += other.late_arrival_free;
        self.same_point += other.same_point;
        self.slow_clause_pooled += other.slow_clause_pooled;
        self.slow_clause_free += other.slow_clause_free;
        for (slot, n) in self.phases.iter_mut().zip(other.phases) {
            *slot += n;
        }
    }
}

/// The ticks of each anchor frame.
const ANCHOR_TICKS: usize = 20;

fn anchor_unit(
    unit_id: u32,
    owner_slot: u8,
    position: Vec2,
    radius: f32,
    speed: Option<f32>,
    target: Option<Vec2>,
) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id,
        owner_slot,
        unit_type: 1,
        position,
        radius,
        speed,
        target,
        progress: None,
        plain_progress: None,
        physical_escape: None,
    }
}

/// Give a unit a pool of one slot at its target.
fn with_pool(unit: &mut MovementFrameUnit, pools: &mut BTreeMap<u64, MovementPool>) {
    let slot = unit.target.expect("a pooled anchor unit has a target");
    unit.progress = Some(MoveProgress {
        pool_id: 1,
        slot,
        activity: MovementActivity::Moving,
        pass_mode: false,
        stalling_on_group: false,
        stalled_ticks: 0,
        best_distance: unit.position.distance(slot),
        claim_count: 0,
        follow_origin: None,
    });
    pools.insert(
        1,
        MovementPool {
            pool_id: 1,
            point: slot,
            spacing: 1.1,
            primary_count: 1,
            slots: vec![slot],
        },
    );
}

fn anchor_frame(
    units: Vec<MovementFrameUnit>,
    pools: BTreeMap<u64, MovementPool>,
) -> MovementFrame {
    MovementFrame {
        bounds: Vec2::new(40.0, 40.0),
        tick_hz: TICK_HZ,
        units,
        pools,
    }
}

/// A mover (radius 0.5, speed 4) at (0.5, 0.9) with the target (0.5, 0.5) in the corner, and a
/// static unit (radius 2) at (0.5, 3.0). The projection pushes the mover past the corner and
/// the clamp puts it exactly on the target: the late arrival of the step (found by the
/// qa-lead review of A2a, finding 1).
fn late_arrival_frame(pooled: bool) -> MovementFrame {
    let target = Vec2::new(0.5, 0.5);
    let mut mover = anchor_unit(1, 0, Vec2::new(0.5, 0.9), 0.5, Some(4.0), Some(target));
    let wall = anchor_unit(2, 1, Vec2::new(0.5, 3.0), 2.0, None, None);
    let mut pools = BTreeMap::new();
    if pooled {
        with_pool(&mut mover, &mut pools);
    }
    anchor_frame(vec![mover, wall], pools)
}

/// A mover with a speed below 0.5 and a target between one step and 0.025 away: only the clause
/// `distance <= 0.025` of the early arrival decides (review finding 4).
fn slow_clause_frame(pooled: bool) -> MovementFrame {
    let mut mover = anchor_unit(
        1,
        0,
        Vec2::new(10.0, 10.0),
        0.5,
        Some(0.4),
        Some(Vec2::new(10.024, 10.0)),
    );
    let mut pools = BTreeMap::new();
    if pooled {
        with_pool(&mut mover, &mut pools);
    }
    anchor_frame(vec![mover], pools)
}

/// Two movers whose step lands exactly on a static unit, so that the projection meets a
/// separation of 0 and takes its direction from the unit ids: mover 1 below static 2 (+X) and
/// static 3 below mover 4 (-X). The speed is 100, so a step is 5 and the neighbour is not
/// within the separation limit at the start (review finding 2).
fn same_point_frame() -> MovementFrame {
    let lane = |id: u32, y: f32| {
        let position = Vec2::new(10.0, y);
        let target = Vec2::new(20.0, y);
        let mover = anchor_unit(id, 0, position, 0.5, Some(100.0), Some(target));
        let landing = step(
            &MovementInput {
                position,
                target,
                speed: 100.0,
            },
            DT,
        )
        .position;
        (mover, landing)
    };
    let (mover_a, landing_a) = lane(1, 10.0);
    let (mover_b, landing_b) = lane(4, 30.0);
    anchor_frame(
        vec![
            mover_a,
            anchor_unit(2, 1, landing_a, 0.5, None, None),
            anchor_unit(3, 1, landing_b, 0.5, None, None),
            mover_b,
        ],
        BTreeMap::new(),
    )
}

/// The anchor frames, each with the count that it must raise.
#[allow(clippy::type_complexity)]
fn anchors() -> Vec<(&'static str, MovementFrame, fn(&Seen) -> usize)> {
    vec![
        ("late arrival, no pool", late_arrival_frame(false), |s| {
            s.late_arrival_free
        }),
        ("late arrival, pool", late_arrival_frame(true), |s| {
            s.late_arrival_pooled
        }),
        ("same point", same_point_frame(), |s| s.same_point),
        ("slow clause, no pool", slow_clause_frame(false), |s| {
            s.slow_clause_free
        }),
        ("slow clause, pool", slow_clause_frame(true), |s| {
            s.slow_clause_pooled
        }),
    ]
}

/// Run one frame chain through both kernels and compare. Returns the unit frames compared.
fn run_chain(
    label: &str,
    mut frame: MovementFrame,
    keep: &BTreeSet<u32>,
    ticks: usize,
    rng: &mut Rng,
    seen: &mut Seen,
) -> usize {
    let mut compared = 0;
    for tick in 0..ticks {
        let live = step_frame_preserving_resting(&frame, DT, keep);
        let reference = reference_step_frame_preserving_resting(&frame, DT, keep);
        assert_eq!(
            live.len(),
            reference.len(),
            "{label} tick {tick}: the output count differs"
        );
        for (new, old) in live.iter().zip(&reference) {
            compared += 1;
            if let Some(text) = difference(new, old) {
                panic!("{label} tick {tick} unit {}: {text}", new.unit_id);
            }
        }
        seen.count(&frame, &live, keep);
        frame = next_frame(&frame, &live, rng);
    }
    compared
}

/// The run that later tasks (A2b1, A2b2, A4, A3 and the slice regression) must run, in release:
///
///     cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference
///
/// `cargo test -p turts_sim --locked` runs 0 tests of this file (the feature is off). The CI
/// form `cargo test --workspace --locked` turns the feature on and runs the debug form: 16 seeds,
/// at most 30 movers, and the five anchor frames. The seeds with 800 to 5,000 movers run only in
/// release.
#[test]
fn kernel_matches_reference_bit_for_bit() {
    let mut seen = Seen::default();
    let mut compared = 0usize;
    for seed in 1..=seeds() {
        let (frame, keep) = start_frame(seed);
        let mut rng = Rng::new(seed ^ 0xa5a5_5a5a);
        compared += run_chain(
            &format!("seed {seed}"),
            frame,
            &keep,
            TICKS,
            &mut rng,
            &mut seen,
        );
    }
    // The anchor frames reach the branches that the random seeds do not. Each raises its own
    // count in its own run, so that a random seed cannot stand in for it.
    let anchors = anchors();
    let anchor_count = anchors.len();
    for (name, frame, raised) in anchors {
        let mut own = Seen::default();
        let mut rng = Rng::new(0x5eed);
        compared += run_chain(
            &format!("anchor {name}"),
            frame,
            &BTreeSet::new(),
            ANCHOR_TICKS,
            &mut rng,
            &mut own,
        );
        assert!(
            raised(&own) > 0,
            "the anchor frame {name:?} did not reach its branch: {own:?}"
        );
        seen.merge(&own);
    }
    println!("\nkernel_reference compared {compared} unit frames: {seen:?}");
    // Each kind of unit and each phase must be reached, else the comparison proves nothing.
    assert_eq!(
        seen.frames,
        seeds() as usize * TICKS + anchor_count * ANCHOR_TICKS
    );
    for (name, n) in [
        ("no-progress moving", seen.no_progress_moving),
        ("pooled moving", seen.pooled_moving),
        ("resting and kept", seen.resting_kept),
        ("follow", seen.follow),
        ("blocked", seen.blocked),
        ("physical escape", seen.escape),
        ("plain progress", seen.plain),
        ("zero-sign arrival", seen.zero_sign_arrivals),
        ("late arrival, pool", seen.late_arrival_pooled),
        ("late arrival, no pool", seen.late_arrival_free),
        ("same point", seen.same_point),
        ("slow clause, pool", seen.slow_clause_pooled),
        ("slow clause, no pool", seen.slow_clause_free),
    ] {
        assert!(n > 0, "no {name} unit frame in the run: {seen:?}");
    }
    for (name, n) in ["arrival", "steering", "projection", "stall", "fallback"]
        .iter()
        .zip(seen.phases)
    {
        assert!(n > 0, "phase {name} never ran in the run: {seen:?}");
    }
}

/// The reference copy calls these helper files of the live crate. A change of one of them would
/// make the harness compare new code with new code. Each pin is the sha256 of the file at the
/// base of A2a. A task that must edit a helper first puts a copy of the old text into the
/// reference (`movement/reference.rs` or a sibling file), then updates the pin here. I chose the
/// pin and not a copy of the helpers: the helpers are 1,300 lines of private types (`pool_index`,
/// `search`, `spread`, `plain`) and `grid.rs`, and a pin costs one line and still stops the
/// silent case. `angle` and the types in `movement/mod.rs` are not pinned (NOT covered).
const REFERENCE_HELPERS: [(&str, &str); 5] = [
    (
        "src/movement/plain.rs",
        "024dccea7b92bd4de8820dcbba5a02a8760e63459f7ed0cc18913690456cfd36",
    ),
    (
        "src/movement/pool_index.rs",
        "e6f5e9b5368cd6d861d03bd7554eeabfa5d88fbd55957128af721230fa7e6fb4",
    ),
    (
        "src/movement/search.rs",
        "b5d11b68e4550b592aecc79cf2941dca981f2df33ec4965e455f74234de57e3b",
    ),
    (
        "src/movement/spread.rs",
        "f6123d1c4090607ba9c8b3f57982be993e869cc5dd51afe115eb81062127cdb6",
    ),
    (
        "src/grid.rs",
        "700abffed1f53610ac2c28bd72c28bf69a3d519b3e1d41ffcf3dc97539ae0291",
    ),
];

#[test]
fn reference_helpers_are_unchanged() {
    for (file, pinned) in REFERENCE_HELPERS {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {file}: {e}"));
        let digest = turts_mod_runtime::hash::sha256_hex(&bytes);
        let actual = digest.strip_prefix("sha256:").unwrap_or(&digest);
        assert_eq!(
            actual, pinned,
            "{file} changed, and the reference copy calls it. Copy the old text into \
             movement/reference.rs first, then update the pin in tests/kernel_reference.rs"
        );
    }
}
