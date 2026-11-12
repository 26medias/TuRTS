//! Repair (design 6.5). Heal for each tick is `floor(rate x t / tick_hz)` minus the same value
//! at `t - 1`; the charge is by applied health. The health target of a site (design 6.10 row K2)
//! is here too: it is a health difference rule.

use crate::Ratio;
use crate::arith::{mul_div_u64, mul_div_wide, saturate_u64};
use core::num::{NonZeroU32, NonZeroU64};

/// `floor(rate_per_second x t / tick_hz)`: the health healed after `t` ticks of one repair
/// order, saturated to `u64::MAX`.
pub fn healed_at(rate_per_second: u64, t: u64, tick_hz: NonZeroU32) -> u64 {
    saturate_u64(mul_div_u64(rate_per_second, t, u64::from(tick_hz.get())))
}

/// The heal of tick `t`: `healed_at(t) - healed_at(t - 1)`. It is 0 for `t` = 0.
pub fn heal_difference(rate_per_second: u64, t: u64, tick_hz: NonZeroU32) -> u64 {
    let Some(before) = t.checked_sub(1) else {
        return 0;
    };
    // The difference of the two floors is taken in u128, and only then saturated: each floor
    // alone can be above `u64::MAX`.
    let hz = u64::from(tick_hz.get());
    saturate_u64(mul_div_u64(rate_per_second, t, hz) - mul_div_u64(rate_per_second, before, hz))
}

/// `min(difference, max - health)`: the applied heal.
pub fn applied_heal(difference: u64, max: u64, health: u64) -> u64 {
    difference.min(max.saturating_sub(health))
}

/// The total charged after `applied_total` applied health points: `floor(H x amount /
/// per_health)`, saturated to `u64::MAX`.
pub fn repair_charged(applied_total: u64, amount: u64, per_health: NonZeroU64) -> u64 {
    saturate_u64(mul_div_u64(applied_total, amount, per_health.get()))
}

/// What one tick charges: the difference of `repair_charged` after and before the applied heal
/// of the tick.
pub fn repair_charge_difference(
    applied_before: u64,
    applied_now: u64,
    amount: u64,
    per_health: NonZeroU64,
) -> u64 {
    // Both totals in u128, then the difference, then the saturation: a total above
    // `u64::MAX` must not make the charge of one tick 0 (a free repair).
    let per = per_health.get();
    saturate_u64(
        mul_div_u64(applied_now, amount, per).saturating_sub(mul_div_u64(
            applied_before,
            amount,
            per,
        )),
    )
}

/// The health target of a site at `done` of `total` (design 6.10 row K2): with `site_health`
/// = n/d the target is `floor(max x (n x total + (d - n) x done) / (d x total))`. It is
/// `floor(max x n / d)` at progress 0 and `max` at completion.
pub fn site_health_target(max: u64, site_health: Ratio, done: u64, total: u64) -> u64 {
    if total == 0 {
        return max;
    }
    let done = u128::from(done.min(total));
    let total = u128::from(total);
    let denominator = u128::from(site_health.denominator());
    // A ratio above 1 counts as 1, so `d - n` cannot go below 0.
    let numerator = u128::from(site_health.numerator()).min(denominator);
    // `done <= total`, so `part <= denominator x total`, and that product is below 2^128.
    let part = numerator * total + (denominator - numerator) * done;
    mul_div_wide(u128::from(max), part, denominator * total)
        .map_or(max, |target| saturate_u64(target).min(max))
}

/// The health that one tick of progress adds to a site: the difference of the target (the
/// difference rule of 6.4). It is 0 when `done` is not above `previous_done`.
pub fn site_health_difference(
    max: u64,
    site_health: Ratio,
    previous_done: u64,
    done: u64,
    total: u64,
) -> u64 {
    site_health_target(max, site_health, done, total).saturating_sub(site_health_target(
        max,
        site_health,
        previous_done,
        total,
    ))
}
