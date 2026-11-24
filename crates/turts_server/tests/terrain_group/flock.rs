//! T-7 flock path: headless ridge traces (FM1, FM2, FM3) and the reversal check. `tick`
//! keeps only the terrain and speed checks; `Fixture::step` adds the body rules.
use super::{
    fixture::{Fixture, VARIANTS},
    physical::SurfaceState,
};
use std::collections::BTreeMap;
use turts_net_protocol::{OrderTarget, Pos, QueueMode};
use turts_rules::terrain::{
    Ratio,
    geometry::{Budget, Point, Traversal},
};
use turts_sim::Vec2;

fn flock_fixture(scene: &str) -> Fixture {
    Fixture::new("quarry", scene, VARIANTS[0])
}
fn point(p: Vec2) -> Point {
    Point {
        x: f64::from(p.x),
        y: f64::from(p.y),
    }
}
/// One real fixed tick on the flock path, with the checks that stay true: every committed
/// step is clear of the true terrain for the whole body, and no unit moves faster than its speed.
fn tick(f: &mut Fixture) {
    for command in f
        .commands
        .iter()
        .filter(|c| c.tick == f.tick)
        .cloned()
        .collect::<Vec<_>>()
    {
        f.send(command.slot, command.envelope);
    }
    let before: Vec<Vec2> = f.actors.iter().map(|a| f.position(&a.label)).collect();
    f.app.update();
    f.tick += 1;
    let lookup = |cell| f.surface.elevation(cell);
    for (actor, from) in f.actors.iter().zip(&before) {
        let to = f.position(&actor.label);
        if *from == to {
            continue;
        }
        let distance = from.distance(to);
        assert!(
            distance <= actor.profile.speed / 20.0 * 1.001,
            "tick{} {} teleported {distance}",
            f.tick,
            actor.label
        );
        let sweep = f.surface.grid.sweep(
            &lookup,
            point(*from),
            point(to),
            f64::from(actor.profile.radius),
            Traversal {
                max_step: actor.profile.max_step,
                max_grade: Ratio {
                    numerator: actor.profile.max_grade[0],
                    denominator: actor.profile.max_grade[1],
                },
            },
            &mut Budget::new(1_000_000),
        );
        assert!(
            sweep.result.is_ok() && sweep.numerator == sweep.denominator,
            "tick{} {} crossed terrain {from:?}->{to:?}: {sweep:?}",
            f.tick,
            actor.label
        );
    }
    if turts_sim::terrain::privileged_changed_chunk_count(f.app.world()) > 0 {
        f.surface = SurfaceState::read(f.app.world());
    }
}
struct Watch {
    start: BTreeMap<String, Vec2>,
    last: BTreeMap<String, Vec2>,
    first_motion: BTreeMap<String, u32>,
    still_ticks: BTreeMap<String, u32>,
    worst_still: (u32, String),
    worst_moving_overlap: (f32, String),
    sent: u32,
}
impl Watch {
    fn new(f: &Fixture, members: &[String]) -> Self {
        let start: BTreeMap<_, _> = members.iter().map(|l| (l.clone(), f.position(l))).collect();
        Self {
            last: start.clone(),
            start,
            first_motion: BTreeMap::new(),
            still_ticks: BTreeMap::new(),
            worst_still: (0, String::new()),
            worst_moving_overlap: (0.0, String::new()),
            sent: f.tick,
        }
    }
    fn observe(&mut self, f: &Fixture) {
        let all = f.labels(|_| true);
        for (label, start) in &self.start {
            let here = f.position(label);
            if !self.first_motion.contains_key(label) && start.distance(here) > 0.05 {
                self.first_motion.insert(label.clone(), f.tick - self.sent);
            }
            let still = self.still_ticks.entry(label.clone()).or_default();
            if f.complete(label) || self.last[label] != here {
                *still = 0;
            } else {
                *still += 1;
                if *still > self.worst_still.0 {
                    self.worst_still = (*still, label.clone());
                }
            }
            self.last.insert(label.clone(), here);
        }
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                let (ra, rb) = (f.actor(a).profile.radius, f.actor(b).profile.radius);
                let depth = ra + rb - f.position(a).distance(f.position(b));
                let share = depth / ra.min(rb);
                if share > self.worst_moving_overlap.0 && (!f.complete(a) || !f.complete(b)) {
                    self.worst_moving_overlap = (share, format!("{a}/{b} tick{}", f.tick));
                }
            }
        }
    }
}
/// The largest overlap depth between two bodies, in metres (0 when all stand apart).
fn rest_overlap(f: &Fixture) -> (f32, String) {
    let all = f.labels(|_| true);
    let mut worst = (0.0f32, String::new());
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            let depth = f.actor(a).profile.radius + f.actor(b).profile.radius
                - f.position(a).distance(f.position(b));
            if depth > worst.0 {
                worst = (depth, format!("{a}/{b}"));
            }
        }
    }
    worst
}
fn row(f: &Fixture, phase: &str, members: &[String]) {
    let audit = turts_sim::terrain::privileged_flock_audit(f.app.world());
    let mut states: BTreeMap<&str, u32> = BTreeMap::new();
    for member in &audit.members {
        *states.entry(member.state).or_default() += 1;
    }
    let complete = members.iter().filter(|l| f.complete(l)).count();
    let west = members.iter().filter(|l| f.position(l).x < 30.0).count();
    let east = members.iter().filter(|l| f.position(l).x > 34.0).count();
    eprintln!(
        "ROW phase={phase} tick={} complete={complete}/{} west={west} east={east} moving={} states={states:?} field_work={:?} rest_overlap={:.3}",
        f.tick,
        members.len(),
        audit.moving,
        audit.owners,
        rest_overlap(f).0
    );
}

/// FM1 and FM2 trace on the ridge layout (GM2-narrow): outbound move of 36, ordinary dig,
/// one return order. Env: RIDGE_DIG_TICK (default 400), RIDGE_RETURN "x,y" (default 22,24),
/// RIDGE_RETURN_ALL=1 puts the digger in the return order.
#[test]
#[ignore = "T-7 flock trace; prints the FM1 and FM2 figures"]
fn flock_ridge_outbound_dig_and_return_trace() {
    let env = |name: &str| std::env::var(name).ok();
    let dig_tick: u32 = env("RIDGE_DIG_TICK")
        .and_then(|v| v.parse().ok())
        .unwrap_or(400);
    let back: Vec<f32> = env("RIDGE_RETURN")
        .unwrap_or_else(|| "22,24".into())
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    let mut f = flock_fixture("GM2-narrow");
    f.commands[1].tick = dig_tick;
    let all = f.labels(|_| true);
    let large = f.labels(|a| a.kind == "large");
    let small = f.labels(|a| a.kind == "small");
    // Outbound.
    let mut watch = Watch::new(&f, &all);
    let mut opened = None;
    let mut small_done = None;
    let mut large_resume = None;
    while f.tick < dig_tick + 2400 {
        tick(&mut f);
        watch.observe(&f);
        if f.tick <= 40 || f.tick.is_multiple_of(40) {
            row(&f, "out", &all);
        }
        for member in &turts_sim::terrain::privileged_flock_audit(f.app.world()).members {
            if member.state.starts_with("settled") {
                let label = &f
                    .actors
                    .iter()
                    .find(|a| a.unit == member.unit)
                    .unwrap()
                    .label;
                eprintln!(
                    "SETTLE tick={} {label} {} at {:?}",
                    f.tick,
                    member.state,
                    f.position(label)
                );
            }
        }
        if small_done.is_none()
            && small
                .iter()
                .filter(|l| *l != "small.0.0")
                .all(|l| f.complete(l))
        {
            small_done = Some(f.tick);
            eprintln!("EVENT small units complete at tick {}", f.tick);
        }
        if opened.is_none() && f.surface.rectangle_is([30, 20], [34, 28], 0) {
            opened = Some(f.tick);
            eprintln!("EVENT passage open at tick {}", f.tick);
        }
        if let Some(open) = opened {
            if large_resume.is_none() && large.iter().any(|l| f.position(l).x > 30.5) {
                large_resume = Some(f.tick);
                eprintln!(
                    "EVENT first large unit in the passage at tick {} (+{})",
                    f.tick,
                    f.tick - open
                );
            }
            if large.iter().all(|l| f.complete(l)) {
                eprintln!(
                    "EVENT all large complete at tick {} (+{})",
                    f.tick,
                    f.tick - open
                );
                break;
            }
        }
    }
    let audit = turts_sim::terrain::privileged_flock_audit(f.app.world());
    for l in &all {
        if !f.complete(l) {
            let member = audit.members.iter().find(|m| m.unit == f.actor(l).unit);
            eprintln!(
                "OUTBOUND-INCOMPLETE {l} pos={:?} member={:?}",
                f.position(l),
                member.map(|m| (
                    m.state,
                    f32::from_bits(m.advice[0]),
                    f32::from_bits(m.advice[1]),
                    m.neighbours.len(),
                    m.field_revision,
                    m.field_ready_cells
                ))
            );
        }
    }
    eprintln!("FM1 outbound first_motion_ticks={:?}", watch.first_motion);
    eprintln!(
        "FM1 outbound first={:?} last={:?} worst_still={:?} worst_moving_overlap={:?}",
        watch.first_motion.values().min(),
        watch.first_motion.values().max(),
        watch.worst_still,
        watch.worst_moving_overlap
    );
    for _ in 0..100 {
        tick(&mut f);
    }
    row(&f, "settled", &all);
    eprintln!("REST after outbound: overlap {:?}", rest_overlap(&f));
    // Return.
    let members: Vec<String> = if env("RIDGE_RETURN_ALL").is_some() {
        all.clone()
    } else {
        all.iter().filter(|l| *l != "small.0.0").cloned().collect()
    };
    let target = OrderTarget::Point(Pos {
        x: back[0],
        y: back[1],
    });
    let e = f.envelope("order_move", &members, target, QueueMode::Replace, 7);
    f.send(0, e);
    let mut watch = Watch::new(&f, &members);
    eprintln!(
        "EVENT return order at tick {} members={} target={back:?}",
        f.tick,
        members.len()
    );
    let mut done = None;
    while f.tick < watch.sent + 4000 {
        tick(&mut f);
        watch.observe(&f);
        let age = f.tick - watch.sent;
        if let Some(label) = env("FLOCK_DEBUG_UNIT")
            && age.is_multiple_of(5)
            && age <= 600
        {
            let audit = turts_sim::terrain::privileged_flock_audit(f.app.world());
            let member = audit
                .members
                .iter()
                .find(|m| m.unit == f.actor(&label).unit);
            eprintln!(
                "DEBUG +{age} {label} pos={:?} complete={} gaps={:?} {:?}",
                f.position(&label),
                f.complete(&label),
                {
                    let mut gaps: Vec<(i32, String, bool)> = all
                        .iter()
                        .filter(|other| **other != label)
                        .map(|other| {
                            let gap = f.position(&label).distance(f.position(other))
                                - f.actor(&label).profile.radius
                                - f.actor(other).profile.radius;
                            ((gap * 1000.0) as i32, other.clone(), f.complete(other))
                        })
                        .collect();
                    gaps.sort();
                    gaps.truncate(3);
                    gaps
                },
                member.map(|m| (
                    m.state,
                    f32::from_bits(m.advice[0]),
                    f32::from_bits(m.advice[1]),
                    m.neighbours.len()
                ))
            );
        }
        if age <= 40 || age.is_multiple_of(40) {
            row(&f, "back", &members);
        }
        if age == 1200 {
            eprintln!(
                "FM2 at 60 s: complete {}/{} rest_overlap={:?}",
                members.iter().filter(|l| f.complete(l)).count(),
                members.len(),
                rest_overlap(&f)
            );
        }
        if done.is_none() && members.iter().all(|l| f.complete(l)) {
            done = Some(age);
            eprintln!("EVENT all return members complete at +{age} ticks");
            break;
        }
    }
    for _ in 0..100 {
        tick(&mut f);
    }
    eprintln!(
        "FM1 return first={:?} last={:?}",
        watch.first_motion.values().min(),
        watch.first_motion.values().max()
    );
    eprintln!(
        "FM2 return done={done:?} worst_still={:?} worst_moving_overlap={:?} rest_overlap={:?}",
        watch.worst_still,
        watch.worst_moving_overlap,
        rest_overlap(&f)
    );
    for l in &all {
        eprintln!(
            "FINAL {l} pos={:?} complete={}",
            f.position(l),
            f.complete(l)
        );
    }
}

/// The unit id never decides a result. The four variants reverse the allocation and the
/// member order, so the unit ids differ. Every physical frame must be equal, row by label,
/// with exact f32 bits. A neighbour sum in unit-id order makes this test red.
#[test]
fn flock_reversal_gives_equal_frames() {
    const TICKS: usize = 600;
    let traces = std::thread::scope(|scope| {
        let workers = VARIANTS.map(|variant| {
            scope.spawn(move || {
                let mut f = Fixture::new("quarry", "GM2-narrow", variant);
                let mut frames = Vec::with_capacity(TICKS);
                for _ in 0..TICKS {
                    tick(&mut f);
                    frames.push(f.frame());
                }
                frames
            })
        });
        workers.map(|worker| match worker.join() {
            Ok(frames) => frames,
            Err(panic) => std::panic::resume_unwind(panic),
        })
    });
    for (variant, trace) in VARIANTS.iter().zip(&traces).skip(1) {
        assert_eq!(trace.len(), traces[0].len());
        for (index, (reference, frame)) in traces[0].iter().zip(trace).enumerate() {
            if let Some((expected, actual)) = reference.iter().zip(frame).find(|(a, b)| a != b) {
                panic!(
                    "frames differ under {variant:?} at tick {} for {}: {expected:?} / {actual:?}",
                    index + 1,
                    expected.label
                );
            }
        }
    }
}
