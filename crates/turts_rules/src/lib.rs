//! `turts_rules`: the one copy of each pure rule of MP1 (design section 9, "One copy of each
//! rule"). The server simulation, the interaction core and the client call these functions.
//! No other crate holds a second copy of the placement check, the relation, the damage
//! formula, the charge, the refund, the heal, the site health or the "value at tick".
//!
//! The crate has no dependency at all. It takes plain values and gives plain values, so it
//! builds on host and on wasm32 with no rendering crate. It never allocates in a rule.
//! Integer rules use `u128` intermediates and no float. The placement check uses `f32`
//! and compares squared distances only.

#![forbid(unsafe_code)]

mod arith;
pub mod charge;
pub mod damage;
pub mod heal;
pub mod placement;
pub mod progress;
pub mod relation;
pub mod supply;

/// A ratio `numerator / denominator` whose denominator is not 0 (the `Ratio` of the mod
/// schema, with the zero denominator made impossible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    numerator: u64,
    denominator: u64,
}

impl Ratio {
    /// 0/1: no refund, no damage.
    pub const ZERO: Ratio = Ratio {
        numerator: 0,
        denominator: 1,
    };
    /// 1/1: the full amount.
    pub const ONE: Ratio = Ratio {
        numerator: 1,
        denominator: 1,
    };

    /// `None` when `denominator` is 0.
    pub const fn new(numerator: u64, denominator: u64) -> Option<Ratio> {
        if denominator == 0 {
            None
        } else {
            Some(Ratio {
                numerator,
                denominator,
            })
        }
    }

    pub const fn numerator(self) -> u64 {
        self.numerator
    }

    pub const fn denominator(self) -> u64 {
        self.denominator
    }
}
