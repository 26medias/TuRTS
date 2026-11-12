//! The system slot of construction placement (`SimSet::Construction`, plan task `L0-SIM`).
//! Empty until `L-CON1`.
//!
//! It runs before the commands of the tick, in `FixedPreUpdate`: the placement of held
//! `Construct` orders only. A site that completed, was destroyed or was cancelled in tick t has
//! freed its cap for the placement of tick t+1. Sites do not advance here: a site advances in
//! the one pay pass (`Production`), through `power::rate_factor`, as a production job does.

pub(crate) fn run_construction() {}
