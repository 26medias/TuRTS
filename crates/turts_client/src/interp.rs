//! Interpolation.
//!
//! Presentation time (design section 5):
//! `t = latest_tick + (now - receipt_time_of_latest) * tick_hz - INTERP_DELAY_TICKS`,
//! clamped to at most `latest_tick`.

use turts_net_protocol::Pos;

/// The presentation delay behind the latest applied tick, in ticks.
pub const INTERP_DELAY_TICKS: f64 = 1.0; // prototype constant: measure at M1

/// The presentation time in ticks. It is never above `latest_tick`.
pub fn presentation_time(latest_tick: u32, since_receipt_secs: f64, tick_hz: u16) -> f64 {
    let latest = f64::from(latest_tick);
    let t = latest + since_receipt_secs * f64::from(tick_hz) - INTERP_DELAY_TICKS;
    t.min(latest)
}

/// The linear interpolation from `a` (at `f = 0`) to `b` (at `f = 1`).
pub fn lerp(a: Pos, b: Pos, f: f32) -> Pos {
    Pos {
        x: a.x + (b.x - a.x) * f,
        y: a.y + (b.y - a.y) * f,
    }
}
