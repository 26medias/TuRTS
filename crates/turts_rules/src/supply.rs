//! The shortage effect (design 6.2). A slot is short of a supply resource when the consumed
//! amount is above the provided amount. Each capability states its own effect in `when_short`.

use crate::Ratio;

/// One `when_short` effect (6.10 row U6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhenShort {
    /// Progress runs at the ratio (producer and constructor).
    Rate(Ratio),
    /// The weapon neither fires nor acquires.
    Disable,
    /// No new job starts. The packer refuses it as reserved in MP1.
    BlockStart,
}

/// True when `consumed` is above `provided`.
pub fn is_short(provided: u64, consumed: u64) -> bool {
    consumed > provided
}

/// The rate of progress: the ratio of a `Rate` effect while short, else 1/1.
pub fn rate_factor(when_short: Option<WhenShort>, short: bool) -> Ratio {
    match when_short {
        Some(WhenShort::Rate(ratio)) if short => ratio,
        _ => Ratio::ONE,
    }
}

/// False for a weapon with a `Disable` effect while short.
pub fn weapon_enabled(when_short: Option<WhenShort>, short: bool) -> bool {
    !(short && when_short == Some(WhenShort::Disable))
}

/// False for a producer with a `BlockStart` effect while short.
pub fn start_allowed(when_short: Option<WhenShort>, short: bool) -> bool {
    !(short && when_short == Some(WhenShort::BlockStart))
}
