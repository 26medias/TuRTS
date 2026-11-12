//! The system slot of the outcome check (`SimSet::Outcome`, plan task `L0-SIM`). Empty until
//! `L-MATCH-SIM`.
//!
//! It runs once for each tick, after every removal and after the death path (the last
//! economy set, combat), and before the `Collect` chain of `FixedPostUpdate` (design 6.7.8).

pub(crate) fn run_outcome_check() {}
