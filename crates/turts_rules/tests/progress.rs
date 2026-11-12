//! The "value at tick" function for rate records (design 6.8; ruling R-K).

use turts_rules::Ratio;
use turts_rules::charge::{charged_at, refund};
use turts_rules::progress::{
    RateRecord, cooldown_left, is_complete, paid_at, rate_step, refund_at, value_at,
};

mod common;
use common::Rng;

fn record(done: u64, total: u64, per_tick: u64, at_tick: u32) -> RateRecord {
    RateRecord {
        done,
        total,
        per_tick,
        at_tick,
        stalled: false,
    }
}

#[test]
fn the_value_at_the_tick_of_the_record_is_the_record_value() {
    let r = record(30, 100, 7, 50);
    assert_eq!(value_at(&r, 50), 30);
    assert_eq!(value_at(&r, 51), 37);
    assert_eq!(value_at(&r, 52), 44);
    // A tick before the record never gives less than the record: the client never goes back.
    assert_eq!(value_at(&r, 49), 30);
    assert_eq!(value_at(&r, 0), 30);
}

#[test]
fn the_value_never_passes_the_end_value() {
    let r = record(30, 100, 7, 50);
    assert_eq!(value_at(&r, 60), 100); // 30 + 70
    assert_eq!(value_at(&r, 61), 100);
    assert_eq!(value_at(&r, u32::MAX), 100);
    assert!(is_complete(&r, 60));
    assert!(!is_complete(&r, 59));
    // A huge rate saturates and still clamps.
    let big = record(1, 1_000, u64::MAX, 0);
    assert_eq!(value_at(&big, u32::MAX), 1_000);
}

#[test]
fn a_stalled_record_keeps_its_value() {
    let mut r = record(30, 100, 7, 50);
    r.stalled = true;
    assert_eq!(value_at(&r, 80), 30);
    assert!(!is_complete(&r, 80));
    // Done at the end stays complete.
    let done = RateRecord {
        done: 100,
        stalled: true,
        ..r
    };
    assert!(is_complete(&done, 80));
}

#[test]
fn a_zero_rate_keeps_the_value() {
    assert_eq!(value_at(&record(5, 100, 0, 10), 1_000), 5);
}

#[test]
fn the_value_is_monotone_and_matches_a_tick_by_tick_run() {
    let mut rng = Rng::new(0x0000_0000_0B06_0001);
    for _ in 0..3_000 {
        let total = rng.range(1, 10_000);
        let done = rng.below(total + 1);
        let per_tick = rng.below(300);
        let at_tick = rng.below(1_000) as u32;
        let r = record(done, total, per_tick, at_tick);
        // The independent oracle: add the rate one tick at a time, clamped at the end.
        let mut run = done;
        let mut last = 0;
        for step in 0..120u32 {
            let tick = at_tick + step;
            let got = value_at(&r, tick);
            assert_eq!(got, run, "tick {tick} of {r:?}");
            assert!(got >= last, "value_at is monotone");
            assert!(got <= total, "value_at never passes the end value");
            last = got;
            run = (run + per_tick).min(total);
        }
    }
}

#[test]
fn paid_and_refund_equal_the_charge_and_refund_functions() {
    let mut rng = Rng::new(0x0000_0000_0B06_0002);
    for _ in 0..3_000 {
        let total = rng.range(1, 1_000_000);
        let done = rng.below(total + 1);
        let per_tick = rng.below(2_000);
        let at_tick = rng.below(500) as u32;
        let cost = rng.wide();
        let den = rng.range(1, 1_000);
        let ratio = Ratio::new(rng.below(den + 1), den).expect("denominator above 0");
        let r = record(done, total, per_tick, at_tick);
        for step in [0u32, 1, 2, 5, 17, 100, 1_000] {
            let tick = at_tick + step;
            // The oracle runs the record forward one tick at a time.
            let mut run = done;
            for _ in 0..step.min(2_000) {
                run = (run + per_tick).min(total);
            }
            let paid = charged_at(cost, run, total);
            // The independent form of the charge: floor(cost x done / total) from u128.
            let share = u128::from(cost) * u128::from(run) / u128::from(total);
            assert_eq!(u128::from(paid), share);
            assert_eq!(paid_at(&r, cost, tick), paid, "Paid at {tick} of {r:?}");
            assert_eq!(refund_at(&r, cost, ratio, tick), refund(paid, ratio));
            let want_refund =
                share * u128::from(ratio.numerator()) / u128::from(ratio.denominator());
            assert_eq!(
                u128::from(refund_at(&r, cost, ratio, tick)),
                want_refund.min(share)
            );
            assert!(paid <= cost && refund_at(&r, cost, ratio, tick) <= paid);
        }
    }
}

#[test]
fn rate_step_is_the_whole_number_q_times_the_rate() {
    let r = |n, d| Ratio::new(n, d).expect("denominator above 0");
    assert_eq!(rate_step(12, Ratio::ONE), Some(12));
    assert_eq!(rate_step(12, r(1, 2)), Some(6));
    assert_eq!(rate_step(12, r(1, 3)), Some(4));
    assert_eq!(rate_step(12, r(2, 3)), Some(8));
    assert_eq!(rate_step(12, r(3, 4)), Some(9));
    assert_eq!(rate_step(5, r(1, 2)), None); // 2.5 is not whole
    assert_eq!(rate_step(u64::MAX, r(u64::MAX, 1)), None); // does not fit in u64
}

#[test]
fn cooldown_counts_down_to_zero() {
    assert_eq!(cooldown_left(100, 90), 10);
    assert_eq!(cooldown_left(100, 100), 0);
    assert_eq!(cooldown_left(100, 130), 0);
}
