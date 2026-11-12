//! The one "value at tick" function for the rate records of design 6.8: a site, a job or a
//! weapon record carries a value at a tick and a rate. The HUD, the hints and the sim all call
//! these functions. The client evaluates a record at the tick of its last applied delta,
//! never later, so this is not prediction.

use crate::Ratio;
use crate::arith::saturate_u64;
use crate::charge::{self, CancelCase, CancelRefunds};

/// A rate record: `done` of `total` at `at_tick`, `per_tick` more in each following tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateRecord {
    pub done: u64,
    pub total: u64,
    pub per_tick: u64,
    pub at_tick: u32,
    /// The job state is `WaitingForFunds`: `done` does not advance.
    pub stalled: bool,
}

/// The value of `record` at `tick`: `done + per_tick x (tick - at_tick)`, never above
/// `total`. A tick before `at_tick` gives the value of the record. A stalled record keeps
/// its value.
pub fn value_at(record: &RateRecord, tick: u32) -> u64 {
    let ticks = tick.saturating_sub(record.at_tick);
    let gained = if record.stalled {
        0
    } else {
        u128::from(record.per_tick) * u128::from(ticks)
    };
    saturate_u64(u128::from(record.done).saturating_add(gained)).min(record.total)
}

/// True when `value_at(record, tick)` has reached `total`.
pub fn is_complete(record: &RateRecord, tick: u32) -> bool {
    value_at(record, tick) >= record.total
}

/// The HUD field `Paid` of one cost entry at `tick`: the charge function at the value.
pub fn paid_at(record: &RateRecord, cost: u64, tick: u32) -> u64 {
    charge::charged_at(cost, value_at(record, tick), record.total)
}

/// The HUD field `Refund` of one cost entry at `tick`: the refund of what is paid, for the
/// cancel `case` (a held order or a queued job gives 0). The one way to compute a refund from a
/// record.
pub fn refund_at(
    record: &RateRecord,
    cost: u64,
    case: CancelCase,
    refunds: &CancelRefunds,
    tick: u32,
) -> u64 {
    charge::refund_for(case, paid_at(record, cost, tick), refunds)
}

/// The ticks that `duration_ticks` of base time take at `rate`: `ceil(duration x den / num)`.
pub fn ticks_at_rate(duration_ticks: u64, rate: Ratio) -> Option<u64> {
    let numerator = u128::from(rate.numerator());
    if numerator == 0 {
        return None;
    }
    let product = u128::from(duration_ticks) * u128::from(rate.denominator());
    u64::try_from(product.div_ceil(numerator)).ok()
}

/// The whole number that each tick adds at `rate`: `q x numerator / denominator`. `None`
/// when it is not a whole number (`q` is not a multiple of the denominator).
pub fn rate_step(q: u64, rate: Ratio) -> Option<u64> {
    let product = u128::from(q) * u128::from(rate.numerator());
    let denominator = u128::from(rate.denominator());
    if !product.is_multiple_of(denominator) {
        return None;
    }
    u64::try_from(product / denominator).ok()
}

/// The whole steps that a counter takes in tick `tick` when it advances at `rate`:
/// `floor(tick x num / den) - floor((tick - 1) x num / den)`. At 1/1 it is 1 in every tick; at
/// 1/2 it is 0 or 1 and the sum over `n` ticks is `floor(n / 2)`; at a rate above 1/1 it can be
/// above 1. The value depends on the tick only, so it keeps no state and two jobs at one rate
/// move in step. Tick 0 is before the first step: a call at tick 0 counts as tick 1 (a test
/// world that calls a system by hand before any tick).
pub fn steps_at_rate(tick: u32, rate: Ratio) -> u64 {
    let tick = u128::from(tick.max(1));
    let numerator = u128::from(rate.numerator());
    let denominator = u128::from(rate.denominator());
    let now = tick * numerator / denominator;
    let before = (tick - 1) * numerator / denominator;
    saturate_u64(now - before)
}

/// Ticks left until `ready_tick` (a weapon record): 0 when ready.
pub fn cooldown_left(ready_tick: u32, tick: u32) -> u32 {
    ready_tick.saturating_sub(tick)
}
