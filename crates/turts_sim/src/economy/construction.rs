//! The system slot of construction (`SimSet::Construction`, plan task `L0-SIM`). Empty until
//! `L-CON1`.
//!
//! It runs after the inserters and before production: the placement of held orders and the
//! advance of sites sit beside the advance of production jobs. A site advances through
//! `power::rate_factor`, as a production job does.

pub(crate) fn run_construction() {}
