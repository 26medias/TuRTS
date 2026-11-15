//! Shared terrain geometry and material laws. No engine or renderer dependencies.

pub mod geometry;
pub mod generation;
pub mod material;

/// An authored nonnegative coefficient. Callers validate a nonzero denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    pub numerator: u64,
    pub denominator: u64,
}
