//! The damage formula (design 6.3). All arithmetic is integer with a `u128` intermediate.
//!
//! ```text
//! base    = weapon damage + modifier   (checked add)
//! raw     = floor(base x num / den)    (the cell of damage type and armour class)
//! hit     = max(minimum, raw saturating_sub flat)
//! applied = min(hit, health)
//! ```

use crate::Ratio;
use crate::arith::{mul_div_u64, saturate_u64};

/// `weapon_damage + modifier`, or `None` on overflow.
pub fn base(weapon_damage: u64, modifier: u64) -> Option<u64> {
    weapon_damage.checked_add(modifier)
}

/// `floor(base x cell)`, saturated to `u64::MAX`.
pub fn raw(base: u64, cell: Ratio) -> u64 {
    saturate_u64(raw_wide(base, cell))
}

/// `floor(base x cell)` with no saturation: the value fits in `u128`.
fn raw_wide(base: u64, cell: Ratio) -> u128 {
    mul_div_u64(base, cell.numerator(), cell.denominator())
}

/// `max(minimum, raw saturating_sub flat_armour)`. `flat_armour` is the `flat` of the unit armour
/// (the name differs because the no-content scan of `turts_checks` reads the identifier `flat`
/// as the id of a map of the `base` package).
pub fn hit(base: u64, cell: Ratio, minimum: u64, flat_armour: u64) -> u64 {
    // The flat armour comes off the wide value, and only then the result saturates: a raw
    // value above `u64::MAX` must not lose the armour before the subtraction.
    minimum.max(saturate_u64(
        raw_wide(base, cell).saturating_sub(u128::from(flat_armour)),
    ))
}

/// The hit when the package may have no damage table (design 6.3: "A package with no damage
/// table keeps the damage of today (`hit = base`)"). `table` is the cell and the `minimum` of the
/// damage table, or `None` for a package with no table.
pub fn hit_for(base: u64, table: Option<(Ratio, u64)>, flat_armour: u64) -> u64 {
    match table {
        None => base,
        Some((cell, minimum)) => hit(base, cell, minimum, flat_armour),
    }
}

/// `min(hit, health)`.
pub fn applied(hit: u64, health: u64) -> u64 {
    hit.min(health)
}
