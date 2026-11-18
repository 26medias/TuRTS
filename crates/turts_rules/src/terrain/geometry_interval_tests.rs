//! Differential coverage for interval invariant reuse. The reference selects the
//! unchanged capsule_touches path through the complete production sweep scheduler.
use super::*;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug)]
struct Case {
    grid: Grid,
    from: Point,
    to: Point,
    radius: f64,
    profile: Traversal,
    surface: u8,
}
fn height(mode: u8, c: Cell) -> Option<i32> {
    match mode {
        0 => Some(0),
        1 => Some((c.x % 3) as i32),
        2 => Some(((c.x + c.y) % 2) as i32),
        3 if c.x == 2 => None,
        4 if c.x == 2 => Some(5), // Outside this corpus's authored ceiling.
        _ => Some(0),
    }
}
fn compare(case: Case, credit: usize) -> (Sweep, usize) {
    let reference_reads = RefCell::new(Vec::new());
    let optimized_reads = RefCell::new(Vec::new());
    let mut reference_budget = Budget::new(credit);
    let mut optimized_budget = Budget::new(credit);
    let reference = case.grid.sweep_impl::<true>(
        &|cell| {
            reference_reads.borrow_mut().push(cell);
            height(case.surface, cell)
        },
        case.from,
        case.to,
        case.radius,
        case.profile,
        &mut reference_budget,
    );
    let optimized = case.grid.sweep(
        &|cell| {
            optimized_reads.borrow_mut().push(cell);
            height(case.surface, cell)
        },
        case.from,
        case.to,
        case.radius,
        case.profile,
        &mut optimized_budget,
    );
    assert_eq!(optimized, reference, "{case:?}, credit={credit}");
    assert_eq!(
        optimized_budget, reference_budget,
        "{case:?}, credit={credit}"
    );
    assert_eq!(
        optimized_reads, reference_reads,
        "{case:?}, credit={credit}"
    );
    (optimized, credit - optimized_budget.remaining())
}
fn profile(max_step: u32) -> Traversal {
    Traversal {
        max_step,
        max_grade: Ratio {
            numerator: 1,
            denominator: 1,
        },
    }
}
fn base() -> Case {
    Case {
        grid: Grid::checked(6, 6, 1, 1, 1, -2, 4).unwrap(),
        from: Point { x: 1.5, y: 2.5 },
        to: Point { x: 3.5, y: 2.5 },
        radius: 0.25,
        profile: profile(1),
        surface: 0,
    }
}
fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}
#[test]
fn interval_reuse_full_sweeps_match_seeded_f32_and_non_dyadic_grids() {
    let mut seed = 0xb601_9db7_2f95_3a41;
    let mut successes = 0;
    let mut refusals = 0;
    for (scale, width, elevation) in [(1, 1, 1), (3, 5, 2), (1000, 1000, 250)] {
        let grid = Grid::checked(6, 6, scale, width, elevation, -2, 4).unwrap();
        let unit = f64::from(width) / f64::from(scale);
        for i in 0..128 {
            let mut pose = || Point {
                x: f64::from(((0.75 + (next(&mut seed) % 4500) as f64 / 1000.) * unit) as f32),
                y: f64::from(((0.75 + (next(&mut seed) % 4500) as f64 / 1000.) * unit) as f32),
            };
            let from = pose();
            let to = if i % 8 == 0 { from } else { pose() };
            let case = Case {
                grid,
                from,
                to,
                radius: f64::from(((next(&mut seed) % 500) as f64 / 1000. * unit) as f32),
                profile: profile((i % 3) as u32),
                surface: (i % 5) as u8,
            };
            for credit in [0, 1, 2, 3, 7, 16, 31, 64, 128, 512, 4096, 65536] {
                let (result, _) = compare(case, credit);
                if result.result.is_ok() {
                    successes += 1;
                } else {
                    refusals += 1;
                }
            }
        }
    }
    assert!(successes > 100 && refusals > 100);
}
#[test]
fn interval_reuse_preserves_tangencies_unknowns_refinement_and_every_small_budget() {
    let points = [
        (Point { x: 1., y: 1. }, Point { x: 4., y: 1. }, 0.),
        (Point { x: 1.5, y: 1.5 }, Point { x: 4.5, y: 4.5 }, 0.5),
        (Point { x: 0.5, y: 0.5 }, Point { x: 5.5, y: 0.5 }, 0.5),
        (Point { x: 6., y: 2. }, Point { x: 6., y: 4. }, 0.),
        (Point { x: -0., y: 1. }, Point { x: 0., y: 1. }, 0.),
        (Point { x: 1.5, y: 1.5 }, Point { x: 1.625, y: 1.5 }, 0.25),
    ];
    let mut seen_uncertain = false;
    let mut seen_unknown = false;
    let mut seen_elevation = false;
    for (from, to, radius) in points {
        for surface in 0..5 {
            for max_step in 0..=1 {
                let case = Case {
                    from,
                    to,
                    radius,
                    surface,
                    profile: profile(max_step),
                    ..base()
                };
                let (result, used) = compare(case, 65536);
                seen_uncertain |= result.result == Err(Error::Uncertain);
                seen_unknown |= matches!(result.result, Err(Error::Unknown(_)));
                seen_elevation |= result.result == Err(Error::Elevation);
                // Exercise every refusal boundary of short complete queries, and
                // every small budget of longer refinement/scheduling queries.
                for credit in 0..=used.min(128) + 1 {
                    compare(case, credit);
                }
                for credit in [used.saturating_sub(1), used, used + 1] {
                    compare(case, credit);
                }
            }
        }
    }
    assert!(seen_uncertain && seen_unknown && seen_elevation);
}
#[test]
fn interval_reuse_full_sweeps_keep_numeric_and_invalid_refusals() {
    let mut cases = Vec::new();
    for x in [
        f64::from_bits(1),
        2f64.powi(-124),
        2f64.powi(-64),
        2f64.powi(-40),
        f64::MAX,
        f64::INFINITY,
        f64::NAN,
    ] {
        cases.push(Case {
            from: Point { x, y: 1.5 },
            ..base()
        });
        cases.push(Case {
            to: Point { x, y: 1.5 },
            ..base()
        });
        cases.push(Case {
            radius: x,
            ..base()
        });
    }
    cases.push(Case {
        radius: -1.,
        ..base()
    });
    cases.push(Case {
        profile: Traversal {
            max_step: 1,
            max_grade: Ratio {
                numerator: 1,
                denominator: 0,
            },
        },
        ..base()
    });
    cases.push(Case {
        grid: Grid {
            columns: 0,
            ..base().grid
        },
        ..base()
    });
    let mut arithmetic = false;
    for case in cases {
        for credit in [0, 1, 8, 64, 65536] {
            let (result, _) = compare(case, credit);
            arithmetic |= result.result == Err(Error::Arithmetic);
        }
    }
    assert!(arithmetic);
}
fn capsule(a: P, delta: P, first: Q, last: Q, radius2: Q) -> IntervalCapsule {
    IntervalCapsule {
        a,
        delta,
        first,
        last,
        start: a.at(delta, first).unwrap(),
        end: a.at(delta, last).unwrap(),
        radius2,
        length2: None,
        threshold: None,
        // These controls isolate the unchanged checked-Q fallback and its lazy errors.
        filter: None,
    }
}
fn rect(left: f64, right: f64, bottom: f64, top: f64) -> Rect {
    Rect {
        left: Q::float(left).unwrap(),
        right: Q::float(right).unwrap(),
        bottom: Q::float(bottom).unwrap(),
        top: Q::float(top).unwrap(),
    }
}
fn touch_both(c: &mut IntervalCapsule, r: Rect) -> Result<bool, Error> {
    let old = capsule_touches(c.a, c.delta, c.first, c.last, r, c.radius2);
    let new = c.touches(r, None);
    assert_eq!(new, old);
    new
}
#[test]
fn interval_reuse_does_not_eagerly_evaluate_overflowing_length_or_threshold() {
    let zero = P {
        x: Q::ZERO,
        y: Q::ZERO,
    };
    let one = Q::new(1, 1).unwrap();
    let mut c = capsule(
        zero,
        P {
            x: Q::float(2f64.powi(70)).unwrap(),
            y: Q::ZERO,
        },
        Q::ZERO,
        Q::ZERO,
        Q::ZERO,
    );
    assert_eq!(touch_both(&mut c, rect(-1., 1., -1., 1.)), Ok(true));
    assert_eq!(
        c.length2, None,
        "ray hit must not evaluate overflowing length"
    );
    assert_eq!(
        touch_both(&mut c, rect(1., 2., 1., 2.)),
        Err(Error::Arithmetic)
    );
    assert_eq!(c.length2, Some(Err(Error::Arithmetic)));
    assert_eq!(c.threshold, None);

    let delta = P {
        x: Q::float(2f64.powi(60)).unwrap(),
        y: Q::ZERO,
    };
    let radius2 = Q::float(2f64.powi(20)).unwrap();
    let mut c = capsule(zero, delta, Q::ZERO, one, radius2);
    assert_eq!(touch_both(&mut c, rect(0., 1., 1., 2.)), Ok(true));
    assert_eq!(
        c.length2, None,
        "endpoint hit must also preserve lazy refusal"
    );
    assert_eq!(
        touch_both(&mut c, rect(2f64.powi(59), 2f64.powi(59) + 256., 1., 2.)),
        Err(Error::Arithmetic)
    );
    assert!(matches!(c.length2, Some(Ok(_))));
    assert_eq!(c.threshold, Some(Err(Error::Arithmetic)));

    let mut c = capsule(zero, delta, Q::ZERO, one, radius2);
    assert_eq!(
        touch_both(
            &mut c,
            rect(2f64.powi(59), 2f64.powi(59) + 256., 1024., 1025.)
        ),
        Err(Error::Arithmetic)
    );
    assert_eq!(
        c.threshold, None,
        "cross square must fail before overflowing threshold is evaluated"
    );
}
#[test]
fn interval_reuse_keeps_cached_failures_lazy_after_an_earlier_refusal() {
    let zero = P {
        x: Q::ZERO,
        y: Q::ZERO,
    };
    let mut c = capsule(
        zero,
        P {
            x: Q::float(2f64.powi(70)).unwrap(),
            y: Q::ZERO,
        },
        Q::ZERO,
        Q::ZERO,
        Q::ZERO,
    );
    assert_eq!(
        touch_both(&mut c, rect(1., 2., 1., 2.)),
        Err(Error::Arithmetic)
    );
    assert_eq!(touch_both(&mut c, rect(-1., 1., -1., 1.)), Ok(true));
    assert_eq!(
        touch_both(&mut c, rect(1., 2., 1., 2.)),
        Err(Error::Arithmetic)
    );
}

#[test]
fn cell_memo_full_sweeps_match_at_and_beyond_fixed_capacity() {
    for width in [16, 17, 20] {
        let case = Case {
            grid: Grid::checked(24, 6, 1, 1, 1, -2, 4).unwrap(),
            from: Point { x: 0.25, y: 2.5 },
            to: Point {
                x: f64::from(width) - 0.25,
                y: 2.5,
            },
            radius: 0.,
            ..base()
        };
        let (low, high) = case
            .grid
            .bounds(
                P::point(case.from).unwrap(),
                P::point(case.to).unwrap(),
                Q::ZERO,
            )
            .unwrap();
        assert_eq!((high.x - low.x + 1) * (high.y - low.y + 1), width);
        for surface in 0..5 {
            for max_step in 0..=1 {
                let case = Case {
                    surface,
                    profile: profile(max_step),
                    ..case
                };
                let (_, used) = compare(case, 65536);
                for credit in 0..=128 {
                    compare(case, credit);
                }
                for credit in [256, 512, 1024, 4096, used.saturating_sub(1), used, used + 1] {
                    compare(case, credit);
                }
            }
        }
    }
    assert!(std::mem::size_of::<SweepArithmetic>() <= 7 * 1024);
    eprintln!(
        "fixed cell memo: {} bytes",
        std::mem::size_of::<SweepArithmetic>()
    );
}
fn ray_both(
    a: P,
    delta: P,
    r: Rect,
    first: Q,
    last: Q,
    memo: &mut CellArithmetic,
) -> Result<bool, Error> {
    let old = ray_interval(
        &[a.x, a.y],
        &[delta.x, delta.y],
        &[r.left, r.bottom],
        &[r.right, r.top],
        first,
        Some(last),
    )
    .map(|v| v.is_some());
    let new = cell_ray_interval(a, delta, r, first, last, Some(memo));
    assert_eq!(new, old);
    new
}
#[test]
fn cell_memo_keeps_axis_short_circuit_and_p_before_q_failure_order() {
    let tiny = Q::float(2f64.powi(-126)).unwrap();
    let a = P {
        x: Q::ZERO,
        y: tiny,
    };
    let delta = P {
        x: Q::new(1, 1).unwrap(),
        y: tiny,
    };
    let r = rect(0.5, 0.75, 1., 2.);
    let mut memo = CellArithmetic::EMPTY;
    // The first interval is outside the x slab; y's q would overflow.
    assert_eq!(
        ray_both(a, delta, r, Q::ZERO, Q::new(1, 4).unwrap(), &mut memo),
        Ok(false)
    );
    assert_eq!(memo.valid, 0b0011);
    assert_eq!(
        ray_both(a, delta, r, Q::ZERO, Q::new(1, 1).unwrap(), &mut memo),
        Err(Error::Arithmetic)
    );
    assert_eq!(
        memo.valid, 0b0111,
        "y p succeeds before y q fails; failed terms are not cached"
    );
    assert_eq!(
        ray_both(a, delta, r, Q::ZERO, Q::new(1, 4).unwrap(), &mut memo),
        Ok(false)
    );
}
fn touch_memo(c: &mut IntervalCapsule, r: Rect, memo: &mut CellArithmetic) -> Result<bool, Error> {
    let old = capsule_touches(c.a, c.delta, c.first, c.last, r, c.radius2);
    let new = c.touches(r, Some(memo));
    assert_eq!(new, old);
    new
}
#[test]
fn cell_memo_projects_lazily_then_reuses_across_intervals_without_crossing_early_hits() {
    let a = P {
        x: Q::ZERO,
        y: Q::ZERO,
    };
    let delta = P {
        x: Q::new(4, 1).unwrap(),
        y: Q::ZERO,
    };
    let r = rect(1., 2., 1., 2.);
    let radius2 = Q::new(25, 16).unwrap();
    let mut memo = CellArithmetic::EMPTY;
    let mut before = capsule(a, delta, Q::ZERO, Q::ZERO, radius2);
    assert_eq!(touch_memo(&mut before, r, &mut memo), Ok(false));
    assert_eq!(
        memo.valid & 0x0f0,
        0x0f0,
        "all corner projections were outside the interval"
    );
    assert_eq!(
        memo.valid & 0xf00,
        0,
        "out-of-range projections must not compute cross squares"
    );
    let mut entire = capsule(a, delta, Q::ZERO, Q::new(1, 1).unwrap(), radius2);
    assert_eq!(touch_memo(&mut entire, r, &mut memo), Ok(true));
    assert_eq!(
        memo.valid & 0xf00,
        0x100,
        "only the first corner was needed"
    );
    let valid = memo.valid;
    let mut endpoint = capsule(
        a,
        delta,
        Q::new(1, 4).unwrap(),
        Q::new(3, 8).unwrap(),
        radius2,
    );
    assert_eq!(touch_memo(&mut endpoint, r, &mut memo), Ok(true));
    assert_eq!(memo.valid, valid);
    assert_eq!(
        endpoint.length2, None,
        "cached later terms cannot move before an endpoint hit"
    );
}
#[test]
fn cell_memo_cached_projection_does_not_suppress_later_cross_square_overflow() {
    let a = P {
        x: Q::ZERO,
        y: Q::ZERO,
    };
    let delta = P {
        x: Q::float(2f64.powi(60)).unwrap(),
        y: Q::ZERO,
    };
    let radius2 = Q::float(2f64.powi(20)).unwrap();
    let r = rect(2f64.powi(59), 2f64.powi(59) + 256., 1024., 1025.);
    let mut memo = CellArithmetic::EMPTY;
    let mut before = capsule(a, delta, Q::ZERO, Q::ZERO, radius2);
    assert_eq!(touch_memo(&mut before, r, &mut memo), Ok(false));
    assert_eq!(memo.valid & 0x0f0, 0x0f0);
    assert_eq!(memo.valid & 0xf00, 0);
    let mut entire = capsule(a, delta, Q::ZERO, Q::new(1, 1).unwrap(), radius2);
    assert_eq!(
        touch_memo(&mut entire, r, &mut memo),
        Err(Error::Arithmetic)
    );
    assert_eq!(memo.valid & 0xf00, 0);
    assert_eq!(
        entire.threshold, None,
        "cross failure must still precede threshold overflow"
    );
}

#[test]
fn rectangle_memo_keeps_first_use_errors_fallback_and_stack_bound() {
    let grid = base().grid;
    let mut memo = SweepArithmetic {
        cells: [CellArithmetic::EMPTY; MEMO_CELLS],
        rectangles: [None; 4],
    };
    for ordinal in 0..8 {
        let bad = Cell {
            x: grid.columns,
            y: 0,
        };
        assert_eq!(
            memo.rectangle(&grid, ordinal, bad).err(),
            Some(Error::OutOfBounds)
        );
        if ordinal < 4 {
            assert!(memo.rectangles[ordinal].is_none());
        }
        let cell = Cell {
            x: (ordinal % grid.columns as usize) as u32,
            y: 1,
        };
        let expected = grid.rect(cell).unwrap();
        for _ in 0..8 {
            let actual = memo.rectangle(&grid, ordinal, cell).unwrap();
            assert_eq!(
                [actual.left, actual.right, actual.bottom, actual.top],
                [expected.left, expected.right, expected.bottom, expected.top]
            );
        }
    }
    assert!(std::mem::size_of::<SweepArithmetic>() <= 7 * 1024);
    eprintln!(
        "RECTANGLE_MEMO_STACK bytes={}",
        std::mem::size_of::<SweepArithmetic>()
    );
}

/// Scoped CPU evidence only; no engine/capacity assertion or timing gate.
#[test]
#[ignore]
fn rectangle_memo_scoped_cost() {
    use std::{hint::black_box, time::Instant};
    let grid = Grid::checked(128, 128, 1000, 1000, 250, -16, 16).unwrap();
    let cells = [
        Cell { x: 31, y: 28 },
        Cell { x: 32, y: 28 },
        Cell { x: 33, y: 28 },
        Cell { x: 34, y: 28 },
    ];
    for cached in [false, true, false, true, false, true] {
        let start = Instant::now();
        let mut total = 0.0;
        let mut computations = 0usize;
        for _ in 0..2000 {
            let mut memo = SweepArithmetic {
                cells: [CellArithmetic::EMPTY; MEMO_CELLS],
                rectangles: [None; 4],
            };
            for _ in 0..100 {
                for (ordinal, cell) in cells.into_iter().enumerate() {
                    let rect = if cached {
                        computations += usize::from(memo.rectangles[ordinal].is_none());
                        memo.rectangle(black_box(&grid), ordinal, black_box(cell))
                            .unwrap()
                    } else {
                        computations += 1;
                        black_box(&grid).rect(black_box(cell)).unwrap()
                    };
                    total += black_box(
                        rect.left.value()
                            + rect.right.value()
                            + rect.bottom.value()
                            + rect.top.value(),
                    );
                }
            }
        }
        black_box(total);
        assert_eq!(computations, if cached { 8000 } else { 800000 });
        eprintln!(
            "RECTANGLE_SCOPED cached={cached} rect_computations={computations} ns={} checksum={total}",
            start.elapsed().as_nanos()
        );
    }
}
