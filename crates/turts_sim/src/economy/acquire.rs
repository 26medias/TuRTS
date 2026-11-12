//! The system slot of the acquisition grids (`SimSet::Acquire`, plan task `L0-SIM`). Empty
//! until `L-TGT`.
//!
//! It runs right before combat, so the grids of a tick are built from the positions after
//! movement and are ready for the weapons of the same tick.

pub(crate) fn run_acquire() {}
