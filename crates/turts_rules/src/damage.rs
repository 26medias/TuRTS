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

/// Optional armour curve. Ratios retain their exact numerator and denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    None,
    Ratio { k: Ratio },
    Percent { per_point: Ratio, cap: Ratio },
}

impl Curve {
    /// Exact damage multiplier. Invalid curves and checked-arithmetic overflow return `None`.
    pub fn factor(self, points: u64) -> Option<(u128, u128)> {
        match self {
            Self::None => Some((1, 1)),
            Self::Ratio { k } => {
                if k.numerator() == 0 {
                    return None;
                }
                let numerator = u128::from(k.numerator());
                let denominator = u128::from(k.denominator())
                    .checked_mul(u128::from(points))?
                    .checked_add(numerator)?;
                Some((numerator, denominator))
            }
            Self::Percent { per_point, cap } => {
                if cap.numerator() >= cap.denominator() {
                    return None;
                }
                let reduction =
                    u128::from(per_point.numerator()).checked_mul(u128::from(points))?;
                let denominator = u128::from(per_point.denominator());
                let cap_numerator = u128::from(cap.numerator());
                let cap_denominator = u128::from(cap.denominator());
                if reduction.checked_mul(cap_denominator)?
                    >= cap_numerator.checked_mul(denominator)?
                {
                    Some((cap_denominator - cap_numerator, cap_denominator))
                } else {
                    Some((denominator.checked_sub(reduction)?, denominator))
                }
            }
        }
    }
}

/// Checked inputs for the common damage pipeline. The caller sums matching bonus rows and
/// armour modifiers with checked arithmetic before constructing this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub base: u64,
    pub bonus: u64,
    pub cell: Ratio,
    pub curve: Curve,
    pub points: u64,
    pub flat: u64,
    pub minimum: u64,
}

impl Hit {
    /// Apply bonus, table and curve with one floor, then flat armour and the minimum.
    /// `None` rejects an overflowing sum, overflowing denominator, or invalid curve.
    pub fn resolve(self) -> Option<u64> {
        let base = self.base.checked_add(self.bonus)?;
        let (curve_num, curve_den) = self.curve.factor(self.points)?;
        let numerator = u128::from(self.cell.numerator()).checked_mul(curve_num)?;
        let denominator = u128::from(self.cell.denominator()).checked_mul(curve_den)?;
        // The triple product can need 192 bits. Use the existing exact 256-bit helper.
        // If the quotient exceeds u128, subtracting a u64 cannot bring it below u64::MAX.
        let wide = crate::arith::mul_div_wide(u128::from(base), numerator, denominator)
            .unwrap_or(u128::MAX);
        Some(
            self.minimum
                .max(saturate_u64(wide.saturating_sub(u128::from(self.flat)))),
        )
    }
}
