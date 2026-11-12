//! Payment over time and refunds, design 6.4 and rows PAY1, CON3 (D:2449).

use turts_rules::Ratio;
use turts_rules::charge::{
    CancelCase, CancelRefunds, charge_difference, charged_at, refund, refund_for,
};

mod common;
use common::Rng;

fn ratio(num: u64, den: u64) -> Ratio {
    Ratio::new(num, den).expect("denominator above 0")
}

#[test]
fn charged_at_is_the_floor_of_the_share() {
    assert_eq!(charged_at(500, 37, 100), 185);
    assert_eq!(charged_at(500, 0, 100), 0);
    assert_eq!(charged_at(500, 100, 100), 500);
    assert_eq!(charged_at(7, 1, 3), 2); // floor(7 / 3)
    assert_eq!(charged_at(u64::MAX, 1, 2), u64::MAX / 2);
    assert_eq!(charged_at(u64::MAX, u64::MAX - 1, u64::MAX), u64::MAX - 1);
    assert_eq!(charged_at(u64::MAX, u64::MAX, u64::MAX), u64::MAX);
}

#[test]
fn done_above_total_counts_as_total_and_a_zero_total_is_complete() {
    assert_eq!(charged_at(500, 101, 100), 500);
    assert_eq!(charged_at(500, u64::MAX, 100), 500);
    assert_eq!(charged_at(500, 0, 0), 500);
}

#[test]
fn the_difference_is_the_step_between_two_totals() {
    assert_eq!(charge_difference(500, 0, 37, 100), 185);
    assert_eq!(charge_difference(500, 37, 38, 100), 190 - 185);
    assert_eq!(charge_difference(500, 38, 38, 100), 0);
    // A done that goes back charges nothing: a balance never gets a negative charge.
    assert_eq!(charge_difference(500, 38, 37, 100), 0);
}

#[test]
fn con3_cancel_at_37_of_100_on_a_cost_of_500() {
    // CON3 (D:2449): the charge is 185. Ratio 3/4 refunds 138. A destroyed site refunds 0
    // when `destroyed` is absent (0/1) and 92 when it is 1/2.
    let refunds = CancelRefunds {
        running: ratio(3, 4),
        output_blocked: ratio(1, 1),
        destroyed: Ratio::ZERO,
    };
    let charged = charged_at(500, 37, 100);
    assert_eq!(charged, 185);
    assert_eq!(refund_for(CancelCase::Running, charged, &refunds), 138);
    assert_eq!(refund_for(CancelCase::Destroyed, charged, &refunds), 0);
    let half = CancelRefunds {
        destroyed: ratio(1, 2),
        ..refunds
    };
    assert_eq!(refund_for(CancelCase::Destroyed, charged, &half), 92);
    // A held order or a queued job charged 0, and the case refunds 0 whatever the ratio is.
    assert_eq!(refund_for(CancelCase::HeldOrQueued, charged, &refunds), 0);
    assert_eq!(refund_for(CancelCase::HeldOrQueued, 500, &half), 0);
    // A complete job that waits for a free exit refunds by `output_blocked`.
    assert_eq!(refund_for(CancelCase::OutputBlocked, 500, &refunds), 500);
}

#[test]
fn a_refund_never_passes_what_was_charged() {
    // The packer refuses a ratio above 1. The function must not mint credits if one arrives.
    assert_eq!(refund(100, ratio(3, 2)), 100);
    assert_eq!(refund(u64::MAX, ratio(u64::MAX, 1)), u64::MAX);
}

#[test]
fn the_refund_is_the_floor_for_random_ratios_up_to_one() {
    let mut rng = Rng::new(0x0000_0000_0EF0_0D01);
    for _ in 0..50_000 {
        let charged = rng.wide();
        let den = rng.wide().max(1);
        let num = if rng.chance(8) {
            den
        } else {
            rng.below(den + 1)
        };
        let want = (u128::from(charged) * u128::from(num) / u128::from(den)) as u64;
        assert_eq!(
            refund(charged, ratio(num, den)),
            want,
            "{charged} x {num}/{den}"
        );
        assert!(want <= charged);
    }
}

/// The five rates of PAY1 as (numerator, denominator). Q is their least common multiple: 12.
const RATES: [(u64, u64); 5] = [(1, 1), (1, 2), (1, 3), (2, 3), (3, 4)];
const Q: u64 = 12;

#[test]
fn the_charged_sum_equals_the_cost_for_costs_up_to_u64_max_and_five_rates() {
    let mut rng = Rng::new(0x0000_0000_00C4_A29E);
    let mut checked = 0u32;
    for case in 0..4_000u32 {
        let cost = match case % 5 {
            0 => 0,
            1 => u64::MAX,
            2 => rng.range(1, 1_000),
            _ => rng.wide(),
        };
        let (num, den) = RATES[(case % 5) as usize];
        let step = Q * num / den;
        assert!(
            (Q * num).is_multiple_of(den),
            "Q is a whole multiple of every rate"
        );
        let duration = rng.range(1, 300);
        let total = duration * Q;
        let mut done = 0u64;
        let mut charged_sum = 0u64;
        let mut ticks = 0u64;
        while done < total {
            ticks += 1;
            // A random stall: the job cannot pay, charges nothing and does not advance.
            if rng.chance(4) {
                continue;
            }
            let next = (done + step).min(total);
            let part = charge_difference(cost, done, next, total);
            assert!(charged_at(cost, next, total) >= charged_at(cost, done, total));
            charged_sum = charged_sum.checked_add(part).expect("the sum fits in u64");
            assert!(
                charged_sum <= cost,
                "charged {charged_sum} above cost {cost}"
            );
            done = next;
        }
        assert_eq!(
            charged_sum, cost,
            "case {case}: cost {cost} rate {num}/{den}"
        );
        assert!(
            ticks >= duration * den / num,
            "the job took at least its nominal time"
        );
        checked += 1;
    }
    assert_eq!(checked, 4_000);
}
