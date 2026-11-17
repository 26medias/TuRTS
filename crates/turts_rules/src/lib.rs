//! `turts_rules`: the one copy of each pure rule of MP1 (design section 9, "One copy of each
//! rule"). The server simulation, the interaction core and the client call these functions.
//! No other crate holds a second copy of placement, relations, damage, charging,
//! refunds, healing, site health, terrain geometry or material conservation.
//!
//! The crate has no dependency at all. It takes plain values and gives plain values, so it
//! builds on host and on wasm32 with no rendering crate. Scalar rules do not allocate.
//! Terrain columns and queries use bounded owned storage. Material accounting uses
//! checked `u128` intermediates. Terrain predicates preserve continuous actor poses
//! through checked rational arithmetic. Legacy placement uses squared `f32` distances.

#![forbid(unsafe_code)]

mod arith;
pub mod charge;
pub mod damage;
pub mod heal;
pub mod placement;
pub mod progress;
pub mod relation;
pub mod supply;
pub mod utilities;
pub mod visibility;
pub mod visibility_advice;

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

/// Exact terrain geometry and material accounting.
pub mod terrain;
