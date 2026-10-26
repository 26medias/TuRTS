//! The UnitRegistry trait.

use crate::error::ModError;
use crate::ids::{PackageId, UnitTypeId};
use crate::schema::UnitDef;

/// The only path by which a package adds a unit type to the simulation.
/// `turts_sim` implements it. The call checks that the local id has no `:`, that
/// `origin:id` is unique, and that speed and radius are in range. It leaves
/// `ModError::file` as `None`; the loader fills it.
pub trait UnitRegistry {
    fn register_unit_type(
        &mut self,
        origin: &PackageId,
        def: UnitDef,
    ) -> Result<UnitTypeId, ModError>;
}
