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

/// The rate of a capability that has several `when_short` entries.
///
/// The smallest active rate wins. No active constraint gives the full rate.
/// The `Option` return shape is retained for existing callers; the defined composition
/// always produces `Some`.
pub fn combined_rate(entries: &[(WhenShort, bool)]) -> Option<Ratio> {
    Some(
        entries
            .iter()
            .fold(Ratio::ONE, |rate, (effect, short)| match effect {
                WhenShort::Rate(other) if *short => minimum_rate(rate, *other),
                _ => rate,
            }),
    )
}

/// Select the smaller fraction without rounding or overflowing a `u64` product.
pub fn minimum_rate(left: Ratio, right: Ratio) -> Ratio {
    if u128::from(left.numerator()) * u128::from(right.denominator())
        <= u128::from(right.numerator()) * u128::from(left.denominator())
    {
        left
    } else {
        right
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
