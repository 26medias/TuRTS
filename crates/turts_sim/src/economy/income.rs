//! The system slot of free income (`SimSet::Income`, plan task `L0-SIM`). Empty until `L-INC`.
//!
//! It is the first set of `FixedUpdate`: after the commands of the tick and before every
//! per-tick charge. So a charge at the order (`ChargeOnQueue`, the reserve of
//! `ReserveThenCharge`), which runs in the commands, sees the balance before the income of the
//! tick; the pay pass in `Production` sees the balance after it.

pub(crate) fn run_income() {}
