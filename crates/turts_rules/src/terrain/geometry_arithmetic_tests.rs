//! Differential oracle uses the pre-optimization signed divisions and Euclidean GCD.
//! Include unreduced scheduling fractions: not every internal Q is canonical.
use super::{Error, Q, ResultQ, exact_quotient};
use std::cmp::Ordering;
fn gcd(mut a: u128, mut b: u128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    i128::try_from(a).unwrap()
}
fn normalize(mut n: i128, mut d: i128) -> ResultQ {
    if d == 0 {
        return Err(Error::Arithmetic);
    }
    if d < 0 {
        n = n.checked_neg().ok_or(Error::Arithmetic)?;
        d = d.checked_neg().ok_or(Error::Arithmetic)?;
    }
    if n == 0 {
        return Ok(Q { n: 0, d: 1 });
    }
    let g = gcd(n.unsigned_abs(), d as u128);
    Ok(Q { n: n / g, d: d / g })
}
fn add(a: Q, b: Q) -> ResultQ {
    let g = gcd(a.d as u128, b.d as u128);
    let (am, bm) = (b.d / g, a.d / g);
    normalize(
        a.n.checked_mul(am)
            .and_then(|n| b.n.checked_mul(bm).and_then(|m| n.checked_add(m)))
            .ok_or(Error::Arithmetic)?,
        a.d.checked_mul(am).ok_or(Error::Arithmetic)?,
    )
}
fn mul(a: Q, b: Q) -> ResultQ {
    let g1 = gcd(a.n.unsigned_abs(), b.d as u128);
    let g2 = gcd(b.n.unsigned_abs(), a.d as u128);
    normalize(
        (a.n / g1).checked_mul(b.n / g2).ok_or(Error::Arithmetic)?,
        (a.d / g2).checked_mul(b.d / g1).ok_or(Error::Arithmetic)?,
    )
}
fn cmp(a: Q, b: Q) -> Result<Ordering, Error> {
    let g = gcd(a.d as u128, b.d as u128);
    Ok(a.n
        .checked_mul(b.d / g)
        .ok_or(Error::Arithmetic)?
        .cmp(&b.n.checked_mul(a.d / g).ok_or(Error::Arithmetic)?))
}
fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}
fn wide(seed: &mut u64) -> i128 {
    ((u128::from(next(seed)) << 64) | u128::from(next(seed))) as i128
}
fn check(a: Q, b: Q) -> (bool, bool) {
    let (sum, product) = (add(a, b), mul(a, b));
    assert_eq!(a.add(b), sum, "add {a:?} {b:?}");
    assert_eq!(a.mul(b), product, "mul {a:?} {b:?}");
    assert_eq!(a.cmp(b), cmp(a, b), "cmp {a:?} {b:?}");
    (sum.is_ok(), product.is_ok())
}
#[test]
fn exact_quotients_match_signed_division_at_extremes_and_seeded_multiples() {
    let mut seed = 0xeae2_379c_b50d_714f;
    for shift in 0..127 {
        let divisor = 1i128 << shift;
        let mask = divisor - 1;
        for value in [i128::MIN, i128::MAX, -1, 0, 1, wide(&mut seed)] {
            let multiple = value & !mask;
            assert_eq!(exact_quotient(multiple, divisor), multiple / divisor);
        }
    }
    for _ in 0..32_768 {
        let divisor = ((wide(&mut seed) & i128::MAX) | 1).max(1);
        let quotient = wide(&mut seed) / divisor;
        let multiple = quotient.checked_mul(divisor).unwrap();
        assert_eq!(exact_quotient(multiple, divisor), quotient);
    }
}
#[test]
fn arithmetic_matches_division_oracle_for_raw_schedule_and_full_width_fractions() {
    let mut seed = 0xb07a_09f1_235c_e4ad;
    let mut successes = [0usize; 2];
    let mut failures = [0usize; 2];
    for _ in 0..32_768 {
        let coordinate = |seed: &mut u64| {
            let bits = (((next(seed) % 15 + 119) as u32) << 23)
                | (next(seed) as u32 & 0x7f_ffff)
                | ((next(seed) as u32 & 1) << 31);
            Q::float(f64::from(f32::from_bits(bits))).unwrap()
        };
        let a = coordinate(&mut seed);
        let b = coordinate(&mut seed);
        let time = Q {
            n: i128::from(next(&mut seed) % ((1 << 20) + 1)),
            d: 1 << 20,
        };
        let grid = Q {
            n: i128::from(next(&mut seed) % 4096),
            d: 1000,
        };
        let raw = Q {
            n: wide(&mut seed),
            d: (wide(&mut seed) & i128::MAX).max(1),
        };
        for pair in [(a, b), (a, time), (time, grid), (raw, a), (raw, grid)] {
            let (sum, product) = check(pair.0, pair.1);
            for (index, ok) in [sum, product].into_iter().enumerate() {
                if ok {
                    successes[index] += 1;
                } else {
                    failures[index] += 1;
                }
            }
        }
    }
    assert!(successes.into_iter().all(|n| n > 90_000));
    assert!(failures.into_iter().all(|n| n > 10_000));
    for n in [i128::MIN, i128::MIN + 1, -1, 0, 1, i128::MAX] {
        for d in [1, 2, 3, 1 << 63, 1 << 126, i128::MAX] {
            for other in [Q { n: 0, d: 1 }, Q { n: 1, d: 3 }, Q { n: -1, d: 1 << 100 }] {
                check(Q { n, d }, other);
            }
        }
    }
}

#[test]
fn equal_denominator_and_integer_fast_paths_match_old_errors_at_all_edges() {
    let mut seed = 0x189a_9e41_053a_896d;
    for d in [1, 2, 3, 1000, 1 << 20, 1 << 63, 1 << 126, i128::MAX] {
        for a in [i128::MIN, i128::MIN + 1, -1, 0, 1, i128::MAX] {
            for b in [i128::MIN, -2, -1, 0, 1, 2, i128::MAX] {
                let (x, y) = (Q { n: a, d }, Q { n: b, d });
                assert_eq!(x.cmp(y), cmp(x, y));
                let (x, y) = (Q { n: a, d: 1 }, Q { n: b, d: 1 });
                assert_eq!(x.mul(y), mul(x, y));
            }
        }
    }
    for _ in 0..32768 {
        let d = (wide(&mut seed) & i128::MAX).max(1);
        let (x, y) = (
            Q {
                n: wide(&mut seed),
                d,
            },
            Q {
                n: wide(&mut seed),
                d,
            },
        );
        assert_eq!(x.cmp(y), cmp(x, y));
        let (x, y) = (Q { n: x.n, d: 1 }, Q { n: y.n, d: 1 });
        assert_eq!(x.mul(y), mul(x, y));
    }
}

// Use the exact immediately preceding production implementation for scoped
// timing, not the slower independent Euclidean-GCD correctness oracle above.
#[inline(never)]
fn previous_cmp(a: Q, b: Q) -> Result<Ordering, Error> {
    let g = super::gcd(a.d as u128, b.d as u128) as i128;
    Ok(a.n
        .checked_mul(exact_quotient(b.d, g))
        .ok_or(Error::Arithmetic)?
        .cmp(
            &b.n.checked_mul(exact_quotient(a.d, g))
                .ok_or(Error::Arithmetic)?,
        ))
}
#[inline(never)]
fn previous_mul(a: Q, b: Q) -> ResultQ {
    let g1 = super::gcd(a.n.unsigned_abs(), b.d as u128) as i128;
    let g2 = super::gcd(b.n.unsigned_abs(), a.d as u128) as i128;
    Q::new(
        exact_quotient(a.n, g1)
            .checked_mul(exact_quotient(b.n, g2))
            .ok_or(Error::Arithmetic)?,
        exact_quotient(a.d, g2)
            .checked_mul(exact_quotient(b.d, g1))
            .ok_or(Error::Arithmetic)?,
    )
}
#[inline(never)]
fn current_cmp(a: Q, b: Q) -> Result<Ordering, Error> {
    a.cmp(b)
}
#[inline(never)]
fn current_mul(a: Q, b: Q) -> ResultQ {
    a.mul(b)
}

#[test]
#[ignore]
fn exact_arithmetic_scoped_cost() {
    use std::{hint::black_box, time::Instant};
    for mode in ["equal", "unequal", "mixed"] {
        let inputs: Vec<_> = (1..=256)
            .map(|i| {
                let d = if i % 2 == 0 { 1 << 20 } else { 1000 };
                let e = if mode == "equal" || (mode == "mixed" && i % 2 == 0) {
                    d
                } else {
                    d + 1
                };
                (
                    Q { n: i * 71, d },
                    Q {
                        n: i * 67 - 1000,
                        d: e,
                    },
                )
            })
            .collect();
        for current in [false, true, false, true, false, true] {
            let start = Instant::now();
            let mut checksum = 0i64;
            for _ in 0..2000 {
                for &(a, b) in &inputs {
                    let r = if current {
                        current_cmp(black_box(a), black_box(b))
                    } else {
                        previous_cmp(black_box(a), black_box(b))
                    };
                    checksum += r.unwrap() as i64;
                }
            }
            black_box(checksum);
            eprintln!(
                "ARITH_SCOPED op=cmp mode={mode} current={current} ns={} checksum={checksum}",
                start.elapsed().as_nanos()
            );
        }
    }
    for mode in ["integer", "fraction", "mixed"] {
        let inputs: Vec<_> = (1..=256)
            .map(|i| {
                let d = if mode == "integer" || (mode == "mixed" && i % 2 == 0) {
                    1
                } else {
                    1000
                };
                (
                    Q { n: i * 71, d },
                    Q {
                        n: i * 67 - 1000,
                        d,
                    },
                )
            })
            .collect();
        for current in [false, true, false, true, false, true] {
            let start = Instant::now();
            let mut checksum = 0i128;
            for _ in 0..2000 {
                for &(a, b) in &inputs {
                    let r = if current {
                        current_mul(black_box(a), black_box(b))
                    } else {
                        previous_mul(black_box(a), black_box(b))
                    };
                    checksum += r.unwrap().n;
                }
            }
            black_box(checksum);
            eprintln!(
                "ARITH_SCOPED op=mul mode={mode} current={current} ns={} checksum={checksum}",
                start.elapsed().as_nanos()
            );
        }
    }
}
