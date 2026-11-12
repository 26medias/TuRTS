//! The "value at tick" function for rate records (design 6.8; ruling R-K).

use turts_rules::Ratio;
use turts_rules::charge::{CancelCase, CancelRefunds, charged_at, refund};
use turts_rules::progress::{
    RateRecord, cooldown_left, is_complete, paid_at, rate_step, refund_at, steps_at_rate,
    ticks_at_rate, value_at,
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
            let refunds = CancelRefunds {
                running: ratio,
                output_blocked: Ratio::ONE,
                destroyed: Ratio::ZERO,
            };
            let running = refund_at(&r, cost, CancelCase::Running, &refunds, tick);
            assert_eq!(running, refund(paid, ratio));
            let want_refund =
                share * u128::from(ratio.numerator()) / u128::from(ratio.denominator());
            assert_eq!(u128::from(running), want_refund.min(share));
            // A held order or a queued job gives 0 inside the crate, whatever it is paid.
            assert_eq!(
                refund_at(&r, cost, CancelCase::HeldOrQueued, &refunds, tick),
                0
            );
            assert_eq!(
                refund_at(&r, cost, CancelCase::OutputBlocked, &refunds, tick),
                paid
            );
            assert_eq!(
                refund_at(&r, cost, CancelCase::Destroyed, &refunds, tick),
                0
            );
            assert!(paid <= cost && running <= paid);
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

#[test]
fn con3_refund_by_case_from_a_record() {
    // CON3 (D:2449): cost 500 at 37/100 is paid 185; running 3/4 refunds 138; destroyed 1/2
    // refunds 92; a held order refunds 0.
    let r = record(0, 100, 1, 10);
    let refunds = CancelRefunds {
        running: Ratio::new(3, 4).expect("denominator above 0"),
        output_blocked: Ratio::ONE,
        destroyed: Ratio::new(1, 2).expect("denominator above 0"),
    };
    assert_eq!(refund_at(&r, 500, CancelCase::Running, &refunds, 47), 138);
    assert_eq!(refund_at(&r, 500, CancelCase::Destroyed, &refunds, 47), 92);
    assert_eq!(
        refund_at(&r, 500, CancelCase::HeldOrQueued, &refunds, 47),
        0
    );
}

#[test]
fn ticks_at_rate_is_the_build_time_under_low_power() {
    // HUD6 (D:2476): "Build time 12 s (24 s now: low power)". At 20 Hz: 240 ticks, 480 ticks.
    let rate = |n, d| Ratio::new(n, d).expect("denominator above 0");
    assert_eq!(ticks_at_rate(240, rate(1, 2)), Some(480));
    assert_eq!(ticks_at_rate(240, Ratio::ONE), Some(240));
    // A time that is not whole rounds up: 7 x 3 / 2 = 10.5.
    assert_eq!(ticks_at_rate(7, rate(2, 3)), Some(11));
    assert_eq!(ticks_at_rate(0, rate(1, 2)), Some(0));
    assert_eq!(ticks_at_rate(10, rate(3, 2)), Some(7)); // 6.67
    // A zero rate never finishes: no value.
    assert_eq!(ticks_at_rate(240, Ratio::ZERO), None);
    // The u128 intermediate: u64::MAX x 2 / 1 does not fit in u64.
    assert_eq!(ticks_at_rate(u64::MAX, rate(1, 2)), None);
    assert_eq!(ticks_at_rate(u64::MAX, rate(2, 2)), Some(u64::MAX));
    assert_eq!(ticks_at_rate(u64::MAX / 2, rate(1, 2)), Some(u64::MAX - 1));
}

#[test]
fn ticks_at_rate_agrees_with_running_the_job() {
    // The independent form: run a job of `duration` ticks of base time at the rate, tick by
    // tick, with total = duration x Q and step = Q x num / den (Q = 12), and count ticks.
    for (num, den) in [(1u64, 1u64), (1, 2), (1, 3), (2, 3), (3, 4)] {
        let rate = Ratio::new(num, den).expect("denominator above 0");
        for duration in [1u64, 2, 7, 20, 240, 333] {
            let q = 12u64;
            let step = rate_step(q, rate).expect("Q is a multiple of every denominator");
            let total = duration * q;
            let mut done = 0;
            let mut ticks = 0u64;
            while done < total {
                done += step;
                ticks += 1;
            }
            assert_eq!(
                ticks_at_rate(duration, rate),
                Some(ticks),
                "duration {duration} rate {num}/{den}"
            );
        }
    }
}

#[test]
fn steps_at_rate_is_one_at_full_rate_and_a_floor_sum_below_it() {
    // Full rate: 1 in every tick, tick 0 included.
    for tick in [0, 1, 2, 1_000, u32::MAX] {
        assert_eq!(steps_at_rate(tick, Ratio::ONE), 1, "tick {tick}");
    }
    // 1/2 and 1/3: each tick adds 0 or 1, and `n` ticks add `floor(n x num / den)`.
    for (num, den) in [(1, 2), (1, 3), (2, 3), (7, 10)] {
        let rate = Ratio::new(num, den).unwrap();
        let mut sum = 0;
        for tick in 1..=1_000_u32 {
            let step = steps_at_rate(tick, rate);
            assert!(step <= 1, "{num}/{den} tick {tick} gave {step}");
            sum += step;
            assert_eq!(sum, u64::from(tick) * num / den, "{num}/{den} after {tick}");
        }
    }
    // Zero rate: never a step. Above 1/1: more than one step in some ticks.
    assert_eq!(steps_at_rate(5, Ratio::ZERO), 0);
    let double = Ratio::new(3, 2).unwrap();
    assert_eq!((1..=10).map(|t| steps_at_rate(t, double)).sum::<u64>(), 15);
}
