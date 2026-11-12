//! The system slot of free income (`SimSet::Income`, plan task `L0-SIM`). Empty until `L-INC`.
//!
//! It runs right after the supply computation and before the orders of the tick, so an income
//! tick pays before any charge of the same tick.

pub(crate) fn run_income() {}
