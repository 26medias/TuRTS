//! Repair heal and charge (design 6.5) and the site-health target (row K2, D:1536).

use core::num::{NonZeroU32, NonZeroU64};
use turts_rules::Ratio;
use turts_rules::heal::{
    applied_heal, heal_difference, healed_at, repair_charge_difference, repair_charged,
    site_health_difference, site_health_target,
};

mod common;
use common::Rng;

fn hz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("above 0")
}

fn per(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("above 0")
}

fn ratio(num: u64, den: u64) -> Ratio {
    Ratio::new(num, den).expect("denominator above 0")
}

#[test]
fn heal_follows_the_floor_of_the_rate() {
    // 30 health a second at 20 Hz: 1.5 a tick.
    let got: Vec<u64> = (1..=4).map(|t| heal_difference(30, t, hz(20))).collect();
    assert_eq!(got, [1, 2, 1, 2]);
    assert_eq!(healed_at(30, 4, hz(20)), 6);
    assert_eq!(heal_difference(30, 0, hz(20)), 0);
    assert_eq!(healed_at(u64::MAX, u64::MAX, hz(1)), u64::MAX);
}

#[test]
fn the_heal_differences_add_up_to_the_total() {
    let mut rng = Rng::new(0x0000_0000_4EA1_0001);
    for _ in 0..2_000 {
        let rate = rng.wide() >> 8;
        let tick_hz = hz(rng.range(1, 60) as u32);
        let ticks = rng.range(1, 200);
        let sum: u64 = (1..=ticks).map(|t| heal_difference(rate, t, tick_hz)).sum();
        assert_eq!(
            sum,
            healed_at(rate, ticks, tick_hz),
            "rate {rate} ticks {ticks}"
        );
        // The independent form: the floor of the whole rate, from u128.
        let want = u128::from(rate) * u128::from(ticks) / u128::from(tick_hz.get());
        assert_eq!(
            u128::from(sum),
            want,
            "rate {rate} ticks {ticks} hz {tick_hz}"
        );
    }
}

#[test]
fn the_applied_heal_is_capped_by_missing_health() {
    assert_eq!(applied_heal(5, 100, 98), 2);
    assert_eq!(applied_heal(5, 100, 50), 5);
    assert_eq!(applied_heal(5, 100, 100), 0);
    assert_eq!(applied_heal(5, 100, 120), 0);
}

#[test]
fn the_repair_charge_is_by_applied_health() {
    // amount 3 for each 4 health.
    assert_eq!(repair_charged(0, 3, per(4)), 0);
    assert_eq!(repair_charged(10, 3, per(4)), 7);
    assert_eq!(repair_charge_difference(10, 11, 3, per(4)), 8 - 7);
    // Free repair (amount 0) charges nothing.
    assert_eq!(repair_charged(1_000, 0, per(4)), 0);
    assert_eq!(repair_charged(u64::MAX, u64::MAX, per(1)), u64::MAX);
    let mut rng = Rng::new(0x0000_0000_4EA1_0002);
    for _ in 0..2_000 {
        let amount = rng.wide() >> 20;
        let per_health = per(rng.range(1, 1_000));
        let mut applied = 0u64;
        let mut sum = 0u64;
        for _ in 0..100 {
            let more = rng.below(9);
            sum += repair_charge_difference(applied, applied + more, amount, per_health);
            applied += more;
        }
        assert_eq!(sum, repair_charged(applied, amount, per_health));
    }
}

#[test]
fn the_site_health_target_at_progress_zero_and_at_completion() {
    // Row K2: floor(max x n / d) at progress 0, and max at completion.
    let tenth = ratio(1, 10);
    assert_eq!(site_health_target(1_000, tenth, 0, 100), 100);
    assert_eq!(site_health_target(1_000, tenth, 100, 100), 1_000);
    assert_eq!(site_health_target(1_000, tenth, 50, 100), 550);
    assert_eq!(site_health_target(999, tenth, 0, 100), 99); // floor(99.9)
    // The default 1/1: the site has full health from the start.
    assert_eq!(site_health_target(1_000, Ratio::ONE, 0, 100), 1_000);
    assert_eq!(site_health_target(1_000, Ratio::ZERO, 0, 100), 0);
    assert_eq!(site_health_target(1_000, Ratio::ZERO, 100, 100), 1_000);
    // A total of 0 is a complete job.
    assert_eq!(site_health_target(1_000, tenth, 0, 0), 1_000);
}

#[test]
fn the_site_health_target_does_not_overflow_u128() {
    // Values from Python integers: floor(max x (n x total + (d - n) x done) / (d x total)).
    let max = u64::MAX;
    let total = u64::MAX;
    let d = u64::MAX;
    assert_eq!(
        site_health_target(max, ratio(1, d), u64::MAX / 2, total),
        9_223_372_036_854_775_807
    );
    assert_eq!(
        site_health_target(max, ratio(d - 1, d), 0, total),
        18_446_744_073_709_551_614
    );
    assert_eq!(
        site_health_target(max, ratio(1, 3), 12_345_678_901_234_567, total),
        6_157_145_143_837_340_249
    );
    assert_eq!(
        site_health_target(max, ratio(7, 8), u64::MAX - 1, total),
        max - 1
    );
}

#[test]
fn the_site_health_differences_add_up_to_the_target() {
    let mut rng = Rng::new(0x0000_0000_5174_0001);
    for _ in 0..2_000 {
        let max = rng.wide();
        let den = rng.range(1, 1_000);
        let num = rng.below(den + 1);
        let total = rng.range(1, 5_000);
        let rate = ratio(num, den);
        let mut done = 0u64;
        let mut health = site_health_target(max, rate, 0, total);
        assert_eq!(
            health,
            (u128::from(max) * u128::from(num) / u128::from(den)) as u64
        );
        while done < total {
            let next = (done + rng.range(1, 40)).min(total);
            let step = site_health_difference(max, rate, done, next, total);
            assert!(
                site_health_target(max, rate, next, total)
                    >= site_health_target(max, rate, done, total)
            );
            health += step;
            done = next;
        }
        assert_eq!(
            health, max,
            "an undamaged site completes at full health (CON1)"
        );
    }
}
