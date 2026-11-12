//! The system slot of repair (`SimSet::Repair`, plan task `L0-SIM`, ruling of the technical
//! director on the set order). Empty until `L-REP`.
//!
//! It runs after the acquisition grids and before combat: the heal of tick t comes before the
//! damage of tick t, so the health that a tick publishes still shows every hit, and the death
//! path stays the last writer of health. Repairers charge after the pay pass of the tick.

pub(crate) fn run_repair() {}
