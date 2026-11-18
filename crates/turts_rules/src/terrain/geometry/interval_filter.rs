//! Sufficient floating interval certificates only; ambiguity uses checked Q.
use super::{P, Q, Rect};

#[derive(Clone, Copy, Debug)]
struct Bound {
    low: f64,
    high: f64,
}
impl Bound {
    fn rational(q: Q) -> Option<Self> {
        if q.d <= 0 {
            return None;
        }
        if q.n == 0 {
            return Some(Self {
                low: 0.0,
                high: 0.0,
            });
        }
        let n = q.n as f64;
        let d = q.d as f64;
        let (nl, nh, dl, dh) = (n.next_down(), n.next_up(), d.next_down(), d.next_up());
        if dl <= 0.0 {
            return None;
        }
        let quotients = [nl / dl, nl / dh, nh / dl, nh / dh];
        if quotients
            .iter()
            .any(|v| !v.is_finite() || *v == 0.0 || v.is_subnormal())
        {
            return None;
        }
        let low = quotients
            .into_iter()
            .fold(f64::INFINITY, f64::min)
            .next_down();
        let high = quotients
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max)
            .next_up();
        (low.is_finite() && high.is_finite()).then_some(Self { low, high })
    }
}
fn subtract(a: f64, b: f64, up: bool) -> Option<f64> {
    let result = a - b;
    if !result.is_finite() || result.is_subnormal() || (result == 0.0 && a != b) {
        return None;
    }
    let rounded = if up {
        result.next_up()
    } else {
        result.next_down()
    };
    rounded.is_finite().then_some(rounded)
}
fn square(value: f64, up: bool) -> Option<f64> {
    if value == 0.0 {
        return Some(0.0);
    }
    let result = value * value;
    if !result.is_finite() || result == 0.0 || result.is_subnormal() {
        return None;
    }
    let rounded = if up {
        result.next_up()
    } else {
        result.next_down()
    };
    rounded.is_finite().then_some(rounded.max(0.0))
}
fn add(a: f64, b: f64, up: bool) -> Option<f64> {
    if a == 0.0 && b == 0.0 {
        return Some(0.0);
    }
    let result = a + b;
    if !result.is_finite() || result == 0.0 || result.is_subnormal() {
        return None;
    }
    let rounded = if up {
        result.next_up()
    } else {
        result.next_down()
    };
    rounded.is_finite().then_some(rounded.max(0.0))
}
#[derive(Clone, Copy)]
pub(super) struct IntervalFilter {
    start: [Bound; 2],
    end: [Bound; 2],
    radius2: Bound,
}
impl IntervalFilter {
    pub(super) fn prepare(start: P, end: P, radius2: Q) -> Option<Self> {
        if radius2.n < 0 {
            return None;
        }
        Some(Self {
            start: [Bound::rational(start.x)?, Bound::rational(start.y)?],
            end: [Bound::rational(end.x)?, Bound::rational(end.y)?],
            radius2: Bound::rational(radius2)?,
        })
    }
    pub(super) fn touches(self, rect: Rect) -> Option<bool> {
        let low = [Bound::rational(rect.left)?, Bound::rational(rect.bottom)?];
        let high = [Bound::rational(rect.right)?, Bound::rational(rect.top)?];
        if (0..2).any(|axis| low[axis].high > high[axis].low) {
            return None;
        }
        let mut gap = [0.0; 2];
        for axis in 0..2 {
            let minimum = self.start[axis].low.min(self.end[axis].low);
            let maximum = self.start[axis].high.max(self.end[axis].high);
            gap[axis] = if maximum < low[axis].low {
                subtract(low[axis].low, maximum, false)?.max(0.0)
            } else if high[axis].high < minimum {
                subtract(minimum, high[axis].high, false)?.max(0.0)
            } else {
                0.0
            };
        }
        let lower = add(square(gap[0], false)?, square(gap[1], false)?, false)?;
        if (self.radius2.low > 0.0 && lower >= self.radius2.high)
            || (self.radius2.high == 0.0 && lower > 0.0)
        {
            return Some(false);
        }
        for point in [self.start, self.end] {
            if (0..2)
                .all(|axis| point[axis].low >= low[axis].high && point[axis].high <= high[axis].low)
            {
                return Some(true);
            }
            if self.radius2.low <= 0.0 {
                continue;
            }
            let mut gap = [0.0; 2];
            for axis in 0..2 {
                gap[axis] = subtract(low[axis].high, point[axis].low, true)?
                    .max(subtract(point[axis].high, high[axis].low, true)?)
                    .max(0.0);
            }
            let upper = add(square(gap[0], true)?, square(gap[1], true)?, true)?;
            if upper < self.radius2.low {
                return Some(true);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        Cell, CellArithmetic, Grid, IntervalCapsule, MEMO_CELLS, SweepArithmetic, TIME_PARTS,
        capsule_touches, time,
    };
    use super::*;
    fn q(v: f64) -> Q {
        Q::float(v).unwrap()
    }
    fn p(x: f64, y: f64) -> P {
        P { x: q(x), y: q(y) }
    }
    fn rect(x: f64, y: f64) -> Rect {
        Rect {
            left: q(x),
            right: q(x + 1.0),
            bottom: q(y),
            top: q(y + 1.0),
        }
    }
    fn full(a: P, b: P, r2: Q, rect: Rect) -> super::super::ResultQ {
        // Encode bool for a uniform printable Result.
        let delta = if a.x == b.x && a.y == b.y {
            p(0.0, 0.0)
        } else {
            b.sub(a)?
        };
        capsule_touches(a, delta, Q::ZERO, q(1.0), rect, r2).map(|v| Q {
            n: i128::from(v),
            d: 1,
        })
    }
    #[test]
    fn interval_filter_certifies_only_definite_results_and_keeps_tangent_neighbors() {
        let cell = rect(1.0, 1.0);
        let a = p(1.5, 0.5);
        let b = p(1.75, 0.5);
        assert!(
            IntervalFilter::prepare(a, b, q(0.25))
                .unwrap()
                .touches(cell)
                .is_none()
        );
        for radius in [
            f64::from(f32::from_bits(0.5f32.to_bits() - 1)),
            f64::from(f32::from_bits(0.5f32.to_bits() + 1)),
        ] {
            let r = q(radius);
            let r2 = r.square().unwrap();
            assert_eq!(
                IntervalFilter::prepare(a, b, r2).unwrap().touches(cell),
                Some(radius > 0.5)
            );
        }
        assert_eq!(
            IntervalFilter::prepare(p(1.5, 1.5), p(1.75, 1.5), Q::ZERO)
                .unwrap()
                .touches(cell),
            Some(true)
        );
        assert_eq!(
            IntervalFilter::prepare(p(5.0, 5.0), p(6.0, 6.0), Q::ZERO)
                .unwrap()
                .touches(cell),
            Some(false)
        );
        assert!(IntervalFilter::prepare(a, b, Q { n: -1, d: 1 }).is_none());
        assert!(Bound::rational(Q { n: 1, d: 0 }).is_none());
        assert!(Bound::rational(Q { n: 1, d: -1 }).is_none());
        assert!(square(f64::from_bits(1), false).is_none());
        assert!(square(f64::MAX, true).is_none());
        let mut bad = cell;
        bad.left = q(3.0);
        assert!(
            IntervalFilter::prepare(a, b, q(1.0))
                .unwrap()
                .touches(bad)
                .is_none()
        );
    }
    #[test]
    fn interval_filter_seeded_decisions_match_full_q_and_reverse() {
        let mut seed = 0x189a_ae47_995b_673du64;
        let mut counts = [0usize; 3];
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for i in 0..16384 {
            let a = p(
                (next() % 2048) as f64 / 256.0,
                (next() % 2048) as f64 / 256.0,
            );
            let b = if i % 8 == 0 {
                a
            } else {
                p(
                    (next() % 2048) as f64 / 256.0,
                    (next() % 2048) as f64 / 256.0,
                )
            };
            let r = q(f64::from((next() % 1024) as f32 / 1024.0));
            let r2 = r.square().unwrap();
            let cell = rect((next() % 8) as f64, (next() % 8) as f64);
            let result = IntervalFilter::prepare(a, b, r2).unwrap().touches(cell);
            assert_eq!(
                result,
                IntervalFilter::prepare(b, a, r2).unwrap().touches(cell)
            );
            match result {
                Some(value) => {
                    assert_eq!(
                        full(a, b, r2, cell),
                        Ok(Q {
                            n: i128::from(value),
                            d: 1
                        })
                    );
                    counts[usize::from(value)] += 1;
                }
                None => counts[2] += 1,
            }
        }
        assert!(counts.into_iter().all(|n| n > 100));
        eprintln!(
            "FILTER_STACK preparation={} capsule={} arithmetic={} combined={}",
            std::mem::size_of::<IntervalFilter>(),
            std::mem::size_of::<IntervalCapsule>(),
            std::mem::size_of::<SweepArithmetic>(),
            std::mem::size_of::<IntervalCapsule>() + std::mem::size_of::<SweepArithmetic>()
        );
        eprintln!(
            "FILTER_DECISIONS false={} true={} fallback={}",
            counts[0], counts[1], counts[2]
        );
    }
    #[test]
    #[ignore]
    fn interval_filter_emit_fraction_oracle_cases() {
        for numerator in [
            i128::MIN,
            i128::MIN + 1,
            -(1 << 100),
            -3,
            -1,
            0,
            1,
            3,
            1 << 100,
            i128::MAX,
        ] {
            for denominator in [1, 2, 3, 1 << 20, 1 << 126, i128::MAX] {
                let term = Q {
                    n: numerator,
                    d: denominator,
                };
                let bound = Bound::rational(term).unwrap();
                println!(
                    "FILTER_BOUND\t{}/{}\t{}\t{}",
                    term.n,
                    term.d,
                    bound.low.to_bits(),
                    bound.high.to_bits()
                );
            }
        }
        let mut count = 0;
        for exponent in [0, 25, 63, 100, 126] {
            for denominator in [1, 3, 1 << 20, i128::MAX] {
                for sign in [-1, 1] {
                    for radius2 in [Q::ZERO, q(0.25), q(4.0)] {
                        let a = P {
                            x: Q {
                                n: sign * (1i128 << exponent),
                                d: denominator,
                            },
                            y: Q {
                                n: sign * (1i128 << exponent),
                                d: denominator,
                            },
                        };
                        for b in [a, P { x: a.x, y: Q::ZERO }] {
                            for cell in [rect(0.0, 0.0), rect(-2.0, -2.0)] {
                                let result = IntervalFilter::prepare(a, b, radius2)
                                    .unwrap()
                                    .touches(cell);
                                let old = full(a, b, radius2, cell);
                                let old = match old {
                                    Ok(v) => {
                                        if v.n == 0 {
                                            "false"
                                        } else {
                                            "true"
                                        }
                                    }
                                    Err(super::super::Error::Arithmetic) => "error",
                                    Err(error) => {
                                        panic!("unexpected invalid oracle input: {error:?}")
                                    }
                                };
                                let result = match result {
                                    Some(true) => "true",
                                    Some(false) => "false",
                                    None => "none",
                                };
                                let terms = [
                                    a.x,
                                    a.y,
                                    b.x,
                                    b.y,
                                    radius2,
                                    cell.left,
                                    cell.right,
                                    cell.bottom,
                                    cell.top,
                                ];
                                print!("FILTER_ORACLE\t{count}\t{result}\t{old}");
                                for term in terms {
                                    print!("\t{}/{}", term.n, term.d);
                                }
                                println!();
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(count, 480);
    }
    #[test]
    #[ignore]
    fn interval_filter_scoped_cost() {
        use std::{hint::black_box, time::Instant};
        let grid = Grid::checked(8, 8, 1, 1, 1, -2, 4).unwrap();
        let a = p(1.5, 1.5);
        let delta = p(5.0, 2.0);
        let r = q(f64::from(1.1f32));
        let r2 = r.square().unwrap();
        for filtered in [false, true, false, true, false, true] {
            let start = Instant::now();
            let mut checksum = 0u64;
            for _ in 0..100 {
                let mut arithmetic = SweepArithmetic {
                    cells: [CellArithmetic::EMPTY; MEMO_CELLS],
                    rectangles: [None; 4],
                };
                for interval in 0..100 {
                    let first = time(interval * TIME_PARTS / 100);
                    let last = time((interval + 1) * TIME_PARTS / 100);
                    let start_point = a.at(delta, first).unwrap();
                    let end_point = a.at(delta, last).unwrap();
                    let mut capsule = IntervalCapsule {
                        a,
                        delta,
                        first,
                        last,
                        start: start_point,
                        end: end_point,
                        radius2: r2,
                        length2: None,
                        threshold: None,
                        filter: filtered
                            .then(|| IntervalFilter::prepare(start_point, end_point, r2))
                            .flatten(),
                    };
                    for (ordinal, cell) in (0..=4)
                        .flat_map(|y| (0..=7).map(move |x| Cell { x, y }))
                        .enumerate()
                    {
                        let rect = grid.rect(cell).unwrap();
                        checksum += u64::from(
                            black_box(
                                capsule.touches(black_box(rect), arithmetic.cells.get_mut(ordinal)),
                            )
                            .unwrap(),
                        );
                    }
                }
            }
            black_box(checksum);
            eprintln!(
                "FILTER_SCOPED filtered={filtered} ns={} checksum={checksum}",
                start.elapsed().as_nanos()
            );
        }
    }
}
