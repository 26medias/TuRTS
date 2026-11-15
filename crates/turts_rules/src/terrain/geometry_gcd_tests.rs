//! Differential coverage for the exact arithmetic optimization. No dependency or random seed source.
use super::{Q, gcd};

// Retain the old algorithm as an independent division-based oracle.
fn euclidean(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
fn wide(state: &mut u64) -> u128 {
    (u128::from(next(state)) << 64) | u128::from(next(state))
}
fn compare(a: u128, b: u128) {
    let expected = euclidean(a, b);
    assert_eq!(gcd(a, b), expected, "gcd({a},{b})");
    assert_eq!(gcd(b, a), expected, "gcd({b},{a})");
}
#[test]
fn exact_gcd_matches_old_algorithm_at_every_power_boundary() {
    let mut edges = vec![0, 1, u64::MAX as u128, u128::MAX, i128::MAX as u128];
    for bit in 0..128 {
        let power = 1u128 << bit;
        edges.extend([power - 1, power, power + 1]);
    }
    edges.sort_unstable();
    edges.dedup();
    for &a in &edges {
        for &b in &edges {
            compare(a, b);
        }
    }
}
#[test]
fn exact_gcd_matches_old_algorithm_for_seeded_full_width_and_shared_factors() {
    let mut seed = 0xa57b_c31d_f902_486e;
    for _ in 0..32_768 {
        let a = wide(&mut seed);
        let b = wide(&mut seed);
        compare(a, b);
        let shift = (next(&mut seed) % 128) as u32;
        compare((a >> shift) << shift, (b >> shift) << shift);
        compare(a & u128::from(u64::MAX), b & u128::from(u64::MAX));
        let factor = u128::from(next(&mut seed) | 1);
        compare(
            u128::from(next(&mut seed)) * factor,
            u128::from(next(&mut seed)) * factor,
        );
    }
}
fn dyadic_inputs() -> Vec<(u128, u128)> {
    let mut seed = 0x903f_15be_728c_d46a;
    let mut pairs = Vec::new();
    for _ in 0..4096 {
        // Exact binary32 coordinate/radius mantissas, at world-scale exponents.
        // Promote without rounding, as the geometry boundary actually does.
        let value = f32::from_bits(
            ((120 + next(&mut seed) % 14) as u32) << 23 | (next(&mut seed) as u32 & 0x7f_ffff),
        );
        let other = f32::from_bits(
            ((120 + next(&mut seed) % 14) as u32) << 23 | (next(&mut seed) as u32 & 0x7f_ffff),
        );
        let a = Q::float(f64::from(value)).unwrap();
        let b = Q::float(f64::from(other)).unwrap();
        pairs.extend([
            (a.n.unsigned_abs(), a.d as u128),
            (a.n.unsigned_abs(), b.d as u128),
            (a.d as u128, b.d as u128),
            (a.n.unsigned_abs() * 1000, a.d as u128),
            (
                a.n.unsigned_abs() * b.n.unsigned_abs(),
                a.d as u128 * b.d as u128,
            ),
        ]);
    }
    pairs
}
#[test]
fn exact_gcd_matches_old_algorithm_for_float_and_grid_denominators() {
    for (a, b) in dyadic_inputs() {
        compare(a, b);
    }
    for bits in [
        0,
        1,
        u64::MAX,
        1u64 << 63,
        0x3fe1_9999_9999_999a,
        0x43e0_0000_0000_0001,
    ] {
        if let Ok(q) = Q::float(f64::from_bits(bits)) {
            compare(q.n.unsigned_abs(), q.d as u128);
            assert_eq!(euclidean(q.n.unsigned_abs(), q.d as u128), 1);
        }
    }
}

/// Diagnostic only. It does not replace TW7 or enforce a machine-dependent threshold.
/// Run optimized, pinned to one of the recorded CPUs, with no competing workload.
#[test]
#[ignore = "manual optimized before/after exact GCD microbenchmark"]
fn exact_gcd_microbenchmark() {
    use std::{hint::black_box, time::Instant};
    let mut seed = 0x78e1_a0d3_4f6c_b925;
    let random = (0..20_480)
        .map(|_| (wide(&mut seed), wide(&mut seed)))
        .collect::<Vec<_>>();
    for (name, pairs) in [("terrain_dyadic", dyadic_inputs()), ("full_u128", random)] {
        for &(a, b) in &pairs {
            assert_eq!(gcd(a, b), euclidean(a, b));
        }
        let measure = |function: fn(u128, u128) -> u128| {
            let start = Instant::now();
            let mut checksum = 0u128;
            for _ in 0..8 {
                for &(a, b) in &pairs {
                    checksum ^= function(black_box(a), black_box(b));
                }
            }
            black_box(checksum);
            start.elapsed().as_nanos()
        };
        // Warm both implementations, then alternate their sample order.
        black_box(measure(gcd));
        black_box(measure(euclidean));
        let mut old = Vec::new();
        let mut optimized = Vec::new();
        for sample in 0..15 {
            if sample % 2 == 0 {
                old.push(measure(euclidean));
                optimized.push(measure(gcd));
            } else {
                optimized.push(measure(gcd));
                old.push(measure(euclidean));
            }
        }
        println!(
            "gcd_diagnostic {{\"dataset\":\"{name}\",\"pairs\":{},\"repeats\":8,\"old_ns\":{old:?},\"optimized_ns\":{optimized:?}}",
            pairs.len()
        );
    }
}
