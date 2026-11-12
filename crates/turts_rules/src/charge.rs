//! Payment over time and refunds (design 6.4). The total charged at `done` is
//! `floor(cost x done / total)`; each tick charges the difference to the previous total.

use crate::Ratio;
use crate::arith::{mul_div_u64, saturate_u64};

/// The total charged for one cost entry when `done` of `total` is reached. `done` above
/// `total` counts as `total`. A `total` of 0 is a job that is complete: the full cost.
pub fn charged_at(cost: u64, done: u64, total: u64) -> u64 {
    if total == 0 {
        return cost;
    }
    // `done <= total`, so the result is at most `cost` and fits in `u64`.
    saturate_u64(mul_div_u64(cost, done.min(total), total))
}

/// What one tick charges: `charged_at(done) - charged_at(previous_done)`. It is 0 when
/// `done` is not above `previous_done`.
pub fn charge_difference(cost: u64, previous_done: u64, done: u64, total: u64) -> u64 {
    charged_at(cost, done, total).saturating_sub(charged_at(cost, previous_done, total))
}

/// `floor(charged x ratio)`, never above `charged`.
pub fn refund(charged: u64, ratio: Ratio) -> u64 {
    saturate_u64(mul_div_u64(charged, ratio.numerator(), ratio.denominator())).min(charged)
}

/// The cases of the refund table of design 6.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelCase {
    /// A held order or a queued job: it charged 0.
    HeldOrQueued,
    /// A running job or a site.
    Running,
    /// A complete job that waits for a free exit.
    OutputBlocked,
    /// The producer or the site is destroyed.
    Destroyed,
}

/// The `cancel_refunds` of a job (`destroyed` is 0/1 when the package has none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelRefunds {
    pub running: Ratio,
    pub output_blocked: Ratio,
    pub destroyed: Ratio,
}

/// The refund of one cost entry for `case`.
pub fn refund_for(case: CancelCase, charged: u64, refunds: &CancelRefunds) -> u64 {
    match case {
        CancelCase::HeldOrQueued => 0,
        CancelCase::Running => refund(charged, refunds.running),
        CancelCase::OutputBlocked => refund(charged, refunds.output_blocked),
        CancelCase::Destroyed => refund(charged, refunds.destroyed),
    }
}
