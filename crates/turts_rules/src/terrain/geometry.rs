//! Shared plateau terrain geometry. Actor coordinates remain continuous world values.
//!
//! Predicates use reduced, checked i128 rationals, including the exact binary value
//! of input floats. Arithmetic exhaustion fails closed; it never rounds a pose.
//! Sweeps certify complete intervals. Analytic contacts schedule those intervals;
//! an approximate root is never itself evidence that movement is clear.
//!
//! Numeric limits are explicit: a binary input and every reduced intermediate
//! must fit signed i128 numerator/positive i128 denominator. Unrepresentable
//! values return Arithmetic. Sweep scheduling resolves 2^-20 of a segment;
//! unresolved final contact intervals return Uncertain, never false clearance.
//! No actor coordinate or radius is snapped to that scheduling resolution.
use super::Ratio;
use std::cmp::Ordering;

mod disks_fast;
mod interval_filter;
use interval_filter::IntervalFilter;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell {
    pub x: u32,
    pub y: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    OutOfBounds,
    Unknown(Cell),
    Elevation,
    Arithmetic,
    Budget,
    Step,
    Grade,
    NotAdjacent,
    Uncertain,
}
/// Every cell visit and scheduling/refinement operation consumes caller-owned work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    remaining: usize,
}
impl Budget {
    pub fn new(remaining: usize) -> Self {
        Self { remaining }
    }
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    fn take(&mut self) -> Result<(), Error> {
        self.remaining = self.remaining.checked_sub(1).ok_or(Error::Budget)?;
        Ok(())
    }
}
/// None denotes unavailable knowledge, never a zero-height default.
pub trait Surface {
    fn elevation(&self, cell: Cell) -> Option<i32>;
}
impl<F: Fn(Cell) -> Option<i32>> Surface for F {
    fn elevation(&self, cell: Cell) -> Option<i32> {
        self(cell)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub columns: u32,
    pub rows: u32,
    pub length_scale: u32,
    pub cell_width: u32,
    pub elevation_step: u32,
    pub floor: i32,
    pub ceiling: i32,
}
impl Grid {
    /// A terrain cell is observed only when all positive-area overlapping fog
    /// cells are observed. Shared edges do not disclose neighboring terrain.
    /// Both grids retain their authored precision; no float snapping is used.
    pub fn cell_observed(
        self,
        cell: Cell,
        fog: crate::visibility::Grid,
        budget: &mut Budget,
        mut observed: impl FnMut(u32) -> bool,
    ) -> Result<bool, Error> {
        self.valid()?;
        if cell.x >= self.columns || cell.y >= self.rows {
            return Err(Error::OutOfBounds);
        }
        if crate::visibility::Grid::checked(fog.width, fog.height, fog.cell_size) != Some(fog) {
            return Err(Error::Invalid);
        }
        // The existing map contract stores world extents as f32. Authoring
        // validates this same conversion. Only its outer boundary can clip the
        // exact terrain cell; internal cell boundaries retain rational precision.
        if (f64::from(self.columns) * f64::from(self.cell_width) / f64::from(self.length_scale))
            as f32
            != fog.width
            || (f64::from(self.rows) * f64::from(self.cell_width) / f64::from(self.length_scale))
                as f32
                != fog.height
        {
            return Err(Error::Invalid);
        }
        let size = Q::float(f64::from(fog.cell_size))?;
        let range = |index: u32, extent: f32, count: u32| -> Result<(u32, u32), Error> {
            let min = self.axis(index)?;
            let mut max = self.axis(index.checked_add(1).ok_or(Error::Arithmetic)?)?;
            let map_end = Q::float(f64::from(extent))?;
            if max.cmp(map_end)? == Ordering::Greater {
                max = map_end;
            }
            let first = min.div(size)?;
            let end = max.div(size)?;
            let first = u32::try_from(first.n / first.d).map_err(|_| Error::Arithmetic)?;
            let last = end.n / end.d + i128::from(end.n % end.d != 0) - 1;
            let last = u32::try_from(last).map_err(|_| Error::Arithmetic)?;
            if first > last || last >= count {
                return Err(Error::OutOfBounds);
            }
            Ok((first, last))
        };
        let (first_x, last_x) = range(cell.x, fog.width, fog.columns)?;
        let (first_y, last_y) = range(cell.y, fog.height, fog.rows)?;
        for y in first_y..=last_y {
            for x in first_x..=last_x {
                budget.take()?;
                let index = y
                    .checked_mul(fog.columns)
                    .and_then(|row| row.checked_add(x))
                    .ok_or(Error::Arithmetic)?;
                if !observed(index) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Traversal {
    pub max_step: u32,
    pub max_grade: Ratio,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Support {
    pub cells: Vec<Cell>,
    pub minimum: i32,
    pub maximum: i32,
    /// Highest supporting plateau in world units; not permission for other cells.
    pub anchor: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub cell: Cell,
    pub elevation: i32,
    pub height: f64,
}
/// Exact fraction of the original segment verified by a sweep. Floating-point
/// interpolation of this fraction is not an additional clearance certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sweep {
    pub numerator: u32,
    pub denominator: u32,
    pub result: Result<(), Error>,
}
impl Sweep {
    pub fn fraction(&self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }
}
/// First simultaneous positive-area support constraint for two translated probe disks.
/// Fractions refer to the original continuous segment. Overlapping transition bounds
/// do not establish an ordering. A point transition at zero can certify the infimum
/// of positive contact immediately after an exact starting tangency; it is not Inside.
/// No unresolved query is evidence of contact or clearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DualSupportContact {
    Inside,
    Outside,
    Transition {
        lower: u32,
        upper: u32,
        denominator: u32,
    },
    Unresolved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Face {
    Top,
    XMin,
    XMax,
    YMin,
    YMax,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Point3,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub cell: Cell,
    pub face: Face,
    pub parameter: f64,
    pub position: Point3,
    pub elevation: i32,
    pub top_height: f64,
}

type ResultQ = Result<Q, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Q {
    n: i128,
    d: i128,
}
#[inline]
fn gcd(mut a: u128, mut b: u128) -> u128 {
    if a == 0 {
        return b;
    }
    if b == 0 || a == b {
        return a;
    }
    // Float coordinates yield dyadic denominators. Their GCD needs only the
    // common power of two, including when the numerator uses all 128 bits.
    if a.is_power_of_two() || b.is_power_of_two() {
        return 1u128 << a.trailing_zeros().min(b.trailing_zeros());
    }
    let shift = a.trailing_zeros().min(b.trailing_zeros());
    a >>= a.trailing_zeros();
    b >>= b.trailing_zeros();
    loop {
        // Once the odd residues fit, use single-word arithmetic on native and
        // WASM alike. No 128-bit division is needed in either branch.
        if (a | b) <= u128::from(u64::MAX) {
            let mut a = a as u64;
            let mut b = b as u64;
            loop {
                if a > b {
                    core::mem::swap(&mut a, &mut b);
                }
                b -= a;
                if b == 0 {
                    return u128::from(a) << shift;
                }
                b >>= b.trailing_zeros();
            }
        }
        if a > b {
            core::mem::swap(&mut a, &mut b);
        }
        b -= a;
        if b == 0 {
            return a << shift;
        }
        b >>= b.trailing_zeros();
    }
}

#[cfg(test)]
#[path = "geometry_gcd_tests.rs"]
mod gcd_tests;
// Every caller divides by a positive GCD, so the quotient is exact. A
// signed arithmetic shift equals division in this domain, even for i128::MIN.
// Keep general division for non-dyadic grids and intermediate fractions.
#[inline]
fn exact_quotient(n: i128, divisor: i128) -> i128 {
    debug_assert!(divisor > 0);
    if (divisor as u128).is_power_of_two() {
        n >> divisor.trailing_zeros()
    } else {
        n / divisor
    }
}

#[cfg(test)]
#[path = "geometry_arithmetic_tests.rs"]
mod arithmetic_tests;
impl Q {
    const ZERO: Self = Self { n: 0, d: 1 };
    fn new(mut n: i128, mut d: i128) -> ResultQ {
        if d == 0 {
            return Err(Error::Arithmetic);
        }
        if d < 0 {
            n = n.checked_neg().ok_or(Error::Arithmetic)?;
            d = d.checked_neg().ok_or(Error::Arithmetic)?;
        }
        if n == 0 {
            return Ok(Self::ZERO);
        }
        if d == 1 {
            return Ok(Self { n, d });
        }
        let g = i128::try_from(gcd(n.unsigned_abs(), d as u128)).map_err(|_| Error::Arithmetic)?;
        if g == 1 {
            return Ok(Self { n, d });
        }
        if (g as u128).is_power_of_two() {
            // GCD divides both operands exactly. Arithmetic shift therefore
            // equals signed division, including for negative numerators.
            let shift = g.trailing_zeros();
            return Ok(Self {
                n: n >> shift,
                d: d >> shift,
            });
        }
        Ok(Self { n: n / g, d: d / g })
    }
    fn float(value: f64) -> ResultQ {
        if !value.is_finite() {
            return Err(Error::Invalid);
        }
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as i32;
        let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
        if mantissa == 0 {
            return Ok(Self::ZERO);
        }
        let trailing = mantissa.trailing_zeros();
        let mut n = i128::from(mantissa >> trailing);
        let power = if exponent == 0 {
            -1074
        } else {
            exponent - 1075
        } + trailing as i32;
        if bits >> 63 != 0 {
            n = -n;
        }
        if power >= 0 {
            let multiplier = 1i128
                .checked_shl(power as u32)
                .filter(|v| *v > 0)
                .ok_or(Error::Arithmetic)?;
            Self::new(n.checked_mul(multiplier).ok_or(Error::Arithmetic)?, 1)
        } else {
            let d = 1i128
                .checked_shl((-power) as u32)
                .filter(|v| *v > 0)
                .ok_or(Error::Arithmetic)?;
            Self::new(n, d)
        }
    }
    fn add(self, b: Self) -> ResultQ {
        let g = gcd(self.d as u128, b.d as u128) as i128;
        let a_scale = exact_quotient(b.d, g);
        let b_scale = exact_quotient(self.d, g);
        Self::new(
            self.n
                .checked_mul(a_scale)
                .and_then(|a| b.n.checked_mul(b_scale).and_then(|b| a.checked_add(b)))
                .ok_or(Error::Arithmetic)?,
            self.d.checked_mul(a_scale).ok_or(Error::Arithmetic)?,
        )
    }
    fn neg(self) -> ResultQ {
        Self::new(self.n.checked_neg().ok_or(Error::Arithmetic)?, self.d)
    }
    fn sub(self, b: Self) -> ResultQ {
        self.add(b.neg()?)
    }
    fn mul(self, b: Self) -> ResultQ {
        // Both cross-GCDs and the result denominator are exactly one. Keep
        // the original checked numerator product, including overflow refusal.
        if self.d == 1 && b.d == 1 {
            return self
                .n
                .checked_mul(b.n)
                .map(|n| Self { n, d: 1 })
                .ok_or(Error::Arithmetic);
        }
        let g1 = gcd(self.n.unsigned_abs(), b.d as u128) as i128;
        let g2 = gcd(b.n.unsigned_abs(), self.d as u128) as i128;
        Self::new(
            exact_quotient(self.n, g1)
                .checked_mul(exact_quotient(b.n, g2))
                .ok_or(Error::Arithmetic)?,
            exact_quotient(self.d, g2)
                .checked_mul(exact_quotient(b.d, g1))
                .ok_or(Error::Arithmetic)?,
        )
    }
    fn div(self, b: Self) -> ResultQ {
        self.mul(Self::new(b.d, b.n)?)
    }
    fn square(self) -> ResultQ {
        self.mul(self)
    }
    fn cmp(self, b: Self) -> Result<Ordering, Error> {
        // Equal positive denominators make both original checked scales one.
        // This does not require either input fraction to be reduced.
        if self.d == b.d && self.d > 0 {
            return Ok(self.n.cmp(&b.n));
        }
        let g = gcd(self.d as u128, b.d as u128) as i128;
        Ok(self
            .n
            .checked_mul(exact_quotient(b.d, g))
            .ok_or(Error::Arithmetic)?
            .cmp(
                &b.n.checked_mul(exact_quotient(self.d, g))
                    .ok_or(Error::Arithmetic)?,
            ))
    }
    fn le(self, b: Self) -> Result<bool, Error> {
        Ok(self.cmp(b)? != Ordering::Greater)
    }
    fn lt(self, b: Self) -> Result<bool, Error> {
        Ok(self.cmp(b)? == Ordering::Less)
    }
    fn min(self, b: Self) -> ResultQ {
        Ok(if self.le(b)? { self } else { b })
    }
    fn max(self, b: Self) -> ResultQ {
        Ok(if self.le(b)? { b } else { self })
    }
    fn value(self) -> f64 {
        self.n as f64 / self.d as f64
    }
}
#[derive(Clone, Copy)]
struct P {
    x: Q,
    y: Q,
}
impl P {
    fn point(p: Point) -> Result<Self, Error> {
        Ok(Self {
            x: Q::float(p.x)?,
            y: Q::float(p.y)?,
        })
    }
    fn sub(self, b: Self) -> Result<Self, Error> {
        Ok(Self {
            x: self.x.sub(b.x)?,
            y: self.y.sub(b.y)?,
        })
    }
    fn at(self, delta: Self, t: Q) -> Result<Self, Error> {
        Ok(Self {
            x: self.x.add(delta.x.mul(t)?)?,
            y: self.y.add(delta.y.mul(t)?)?,
        })
    }
    fn dot(self, b: Self) -> ResultQ {
        self.x.mul(b.x)?.add(self.y.mul(b.y)?)
    }
}
struct RelativeDisks {
    from: P,
    delta: P,
    radius2: Q,
}
fn relative_disks(
    a_from: Point,
    a_to: Point,
    a_radius: f64,
    b_from: Point,
    b_to: Point,
    b_radius: f64,
    budget: &mut Budget,
) -> Result<RelativeDisks, Error> {
    budget.take()?;
    if !a_radius.is_finite() || !b_radius.is_finite() || a_radius <= 0.0 || b_radius <= 0.0 {
        return Err(Error::Invalid);
    }
    let a_from = P::point(a_from)?;
    let a_to = P::point(a_to)?;
    let b_from = P::point(b_from)?;
    let b_to = P::point(b_to)?;
    Ok(RelativeDisks {
        from: a_from.sub(b_from)?,
        delta: a_to.sub(a_from)?.sub(b_to.sub(b_from)?)?,
        radius2: Q::float(a_radius)?.add(Q::float(b_radius)?)?.square()?,
    })
}
/// Certify separation of two linearly swept disks throughout the closed tick.
/// Exact tangency is clear. Any strict penetration, including initial overlap,
/// is false even when both endpoints are separated. One pair visit is charged.
/// Arithmetic/input/budget errors are not clearance and must fail closed at callers.
pub fn swept_disks_clear(
    a_from: Point,
    a_to: Point,
    a_radius: f64,
    b_from: Point,
    b_to: Point,
    b_radius: f64,
    budget: &mut Budget,
) -> Result<bool, Error> {
    if budget.remaining() != 0
        && let Some(clear) = disks_fast::clear(a_from, a_to, a_radius, b_from, b_to, b_radius)
    {
        budget.take()?;
        return Ok(clear);
    }
    swept_disks_clear_reference(a_from, a_to, a_radius, b_from, b_to, b_radius, budget)
}
fn swept_disks_clear_reference(
    a_from: Point,
    a_to: Point,
    a_radius: f64,
    b_from: Point,
    b_to: Point,
    b_radius: f64,
    budget: &mut Budget,
) -> Result<bool, Error> {
    let relative = relative_disks(a_from, a_to, a_radius, b_from, b_to, b_radius, budget)?;
    let distance2 = relative.from.dot(relative.from)?;
    if distance2.lt(relative.radius2)? {
        return Ok(false);
    }
    let velocity2 = relative.delta.dot(relative.delta)?;
    let projection = relative.from.dot(relative.delta)?;
    if velocity2.n == 0 || projection.n >= 0 {
        return Ok(true);
    }
    if projection.le(velocity2.neg()?)? {
        let end = relative.from.at(relative.delta, Q::new(1, 1)?)?;
        return relative.radius2.le(end.dot(end)?);
    }
    // At the interior minimum, distance² = cross(relative_start,velocity)²/|v|².
    // Cross multiplication is exact and avoids constructing/squaring the generally
    // non-dyadic minimizing time. No endpoint sampling or collision epsilon is used.
    let cross = relative
        .from
        .x
        .mul(relative.delta.y)?
        .sub(relative.from.y.mul(relative.delta.x)?)?;
    relative.radius2.mul(velocity2)?.le(cross.square()?)
}
/// The existing invalid overlap does not deepen anywhere in this closed tick.
/// For squared distance, the derivative is 2*(r·v+|v|²*t); r·v>=0 is necessary
/// and sufficient for nondecreasing separation on[0,1]. Constant overlap qualifies
/// as non-deepening, not as progress. Initially legal pairs must use swept_disks_clear.
pub fn swept_disks_non_deepening_overlap(
    a_from: Point,
    a_to: Point,
    a_radius: f64,
    b_from: Point,
    b_to: Point,
    b_radius: f64,
    budget: &mut Budget,
) -> Result<bool, Error> {
    if budget.remaining() != 0
        && let Some(clear) =
            disks_fast::non_deepening(a_from, a_to, a_radius, b_from, b_to, b_radius)
    {
        budget.take()?;
        return Ok(clear);
    }
    swept_disks_non_deepening_reference(a_from, a_to, a_radius, b_from, b_to, b_radius, budget)
}
fn swept_disks_non_deepening_reference(
    a_from: Point,
    a_to: Point,
    a_radius: f64,
    b_from: Point,
    b_to: Point,
    b_radius: f64,
    budget: &mut Budget,
) -> Result<bool, Error> {
    let relative = relative_disks(a_from, a_to, a_radius, b_from, b_to, b_radius, budget)?;
    Ok(relative.from.dot(relative.from)?.lt(relative.radius2)?
        && relative.from.dot(relative.delta)?.n >= 0)
}

#[derive(Clone, Copy)]
struct Rect {
    left: Q,
    right: Q,
    bottom: Q,
    top: Q,
}
impl Grid {
    pub fn checked(
        columns: u32,
        rows: u32,
        length_scale: u32,
        cell_width: u32,
        elevation_step: u32,
        floor: i32,
        ceiling: i32,
    ) -> Result<Self, Error> {
        if columns == 0
            || rows == 0
            || length_scale == 0
            || cell_width == 0
            || elevation_step == 0
            || floor >= ceiling
            || u64::from(columns) * u64::from(rows) > 1_048_576
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            columns,
            rows,
            length_scale,
            cell_width,
            elevation_step,
            floor,
            ceiling,
        })
    }
    fn valid(&self) -> Result<(), Error> {
        Self::checked(
            self.columns,
            self.rows,
            self.length_scale,
            self.cell_width,
            self.elevation_step,
            self.floor,
            self.ceiling,
        )
        .map(|_| ())
    }
    fn axis(&self, index: u32) -> ResultQ {
        Q::new(
            i128::from(index) * i128::from(self.cell_width),
            i128::from(self.length_scale),
        )
    }
    fn height(&self, elevation: i32) -> ResultQ {
        Q::new(
            i128::from(elevation) * i128::from(self.elevation_step),
            i128::from(self.length_scale),
        )
    }
    fn rect(&self, cell: Cell) -> Result<Rect, Error> {
        if cell.x >= self.columns || cell.y >= self.rows {
            return Err(Error::OutOfBounds);
        }
        Ok(Rect {
            left: self.axis(cell.x)?,
            right: self.axis(cell.x + 1)?,
            bottom: self.axis(cell.y)?,
            top: self.axis(cell.y + 1)?,
        })
    }
    fn cell_q(&self, p: P) -> Result<Cell, Error> {
        self.cell_impl::<true>(p)
    }
    fn cell_impl<const UNIT: bool>(&self, p: P) -> Result<Cell, Error> {
        if UNIT && self.cell_width != self.length_scale {
            return self.cell_impl::<false>(p);
        }
        self.valid()?;
        let axis = |q: Q, count| -> Result<u32, Error> {
            if UNIT && q.n >= 0 && q.d > 0 && i128::from(count).checked_mul(q.d).is_some() {
                // Exact floor on a unit-ratio grid, including unreduced Q.
                // The checked product preserves the old bound comparison's
                // Arithmetic refusal, including before later invalid inputs.
                let index = q.n / q.d;
                let count_q = i128::from(count);
                if index > count_q || (index == count_q && q.n % q.d != 0) {
                    return Err(Error::OutOfBounds);
                }
                return Ok((index as u32).min(count - 1));
            }
            if !Q::ZERO.le(q)? || !q.le(self.axis(count)?)? {
                return Err(Error::OutOfBounds);
            }
            let index = q.mul(Q::new(
                i128::from(self.length_scale),
                i128::from(self.cell_width),
            )?)?;
            let index = u32::try_from(index.n / index.d).map_err(|_| Error::Arithmetic)?;
            Ok(index.min(count - 1))
        };
        Ok(Cell {
            x: axis(p.x, self.columns)?,
            y: axis(p.y, self.rows)?,
        })
    }
    pub fn point_to_cell(&self, point: Point) -> Result<Cell, Error> {
        self.cell_q(P::point(point)?)
    }
    fn validate_read_cell(&self, cell: Cell) -> Result<(), Error> {
        if cell.x >= self.columns || cell.y >= self.rows {
            return Err(Error::OutOfBounds);
        }
        // rect's u32 index*width products fit i128. Its only remaining
        // possible failure is the zero denominator, before any charged read.
        if self.length_scale == 0 {
            return Err(Error::Arithmetic);
        }
        Ok(())
    }
    fn read(&self, surface: &impl Surface, cell: Cell, budget: &mut Budget) -> Result<i32, Error> {
        self.validate_read_cell(cell)?;
        budget.take()?;
        let elevation = surface.elevation(cell).ok_or(Error::Unknown(cell))?;
        if elevation < self.floor || elevation > self.ceiling {
            return Err(Error::Elevation);
        }
        Ok(elevation)
    }
    pub fn sample(
        &self,
        surface: &impl Surface,
        point: Point,
        budget: &mut Budget,
    ) -> Result<Sample, Error> {
        let cell = self.point_to_cell(point)?;
        let elevation = self.read(surface, cell, budget)?;
        Ok(Sample {
            cell,
            elevation,
            height: self.height(elevation)?.value(),
        })
    }
    fn bounds(&self, from: P, to: P, r: Q) -> Result<(Cell, Cell), Error> {
        if r.n < 0 {
            return Err(Error::Invalid);
        }
        let low = P {
            x: from.x.min(to.x)?.sub(r)?,
            y: from.y.min(to.y)?.sub(r)?,
        };
        let high = P {
            x: from.x.max(to.x)?.add(r)?,
            y: from.y.max(to.y)?.add(r)?,
        };
        Ok((self.cell_q(low)?, self.cell_q(high)?))
    }
    fn support_q(
        &self,
        surface: &impl Surface,
        p: P,
        r: Q,
        max_step: u32,
        budget: &mut Budget,
    ) -> Result<Support, Error> {
        self.support_impl::<true>(surface, p, r, max_step, budget)
    }
    fn support_impl<const FILTER: bool>(
        &self,
        surface: &impl Surface,
        p: P,
        r: Q,
        max_step: u32,
        budget: &mut Budget,
    ) -> Result<Support, Error> {
        let center = self.cell_q(p)?;
        let (low, high) = self.bounds(p, p, r)?;
        let r2 = r.square()?;
        let filter = (FILTER && r.n > 0)
            .then(|| IntervalFilter::prepare(p, p, r2))
            .flatten();
        let mut cells = Vec::new();
        let (mut minimum, mut maximum) = (i32::MAX, i32::MIN);
        for y in low.y..=high.y {
            for x in low.x..=high.x {
                budget.take()?;
                let cell = Cell { x, y };
                if cell != center {
                    let rect = self.rect(cell)?;
                    let touches = match filter.and_then(|filter| filter.touches(rect)) {
                        Some(touches) => touches,
                        None => distance_rect(p, rect)?.lt(r2)?,
                    };
                    if !touches {
                        continue;
                    }
                }
                let elevation = self.read(surface, cell, budget)?;
                minimum = minimum.min(elevation);
                maximum = maximum.max(elevation);
                cells.push(cell);
            }
        }
        if i64::from(maximum) - i64::from(minimum) > i64::from(max_step) {
            return Err(Error::Step);
        }
        Ok(Support {
            cells,
            minimum,
            maximum,
            anchor: self.height(maximum)?.value(),
        })
    }
    /// Exact positive-area disk/cell intersection; tangencies are excluded.
    /// The owning center cell is included even for a zero-radius body.
    pub fn disk_support(
        &self,
        surface: &impl Surface,
        point: Point,
        radius: f64,
        max_step: u32,
        budget: &mut Budget,
    ) -> Result<Support, Error> {
        self.support_q(
            surface,
            P::point(point)?,
            Q::float(radius)?,
            max_step,
            budget,
        )
    }
    pub fn flat_foundation(
        &self,
        surface: &impl Surface,
        point: Point,
        radius: f64,
        budget: &mut Budget,
    ) -> Result<Support, Error> {
        self.disk_support(surface, point, radius, 0, budget)
    }
    fn cardinal(
        &self,
        surface: &impl Surface,
        a: Cell,
        b: Cell,
        profile: Traversal,
        budget: &mut Budget,
    ) -> Result<(), Error> {
        if a.x.abs_diff(b.x) + a.y.abs_diff(b.y) != 1 {
            return Err(Error::NotAdjacent);
        }
        if profile.max_grade.denominator == 0 {
            return Err(Error::Invalid);
        }
        let difference = u128::from(
            self.read(surface, a, budget)?
                .abs_diff(self.read(surface, b, budget)?),
        );
        if difference > u128::from(profile.max_step) {
            return Err(Error::Step);
        }
        let rise = difference
            .checked_mul(u128::from(self.elevation_step))
            .and_then(|n| n.checked_mul(u128::from(profile.max_grade.denominator)))
            .ok_or(Error::Arithmetic)?;
        let run = u128::from(self.cell_width)
            .checked_mul(u128::from(profile.max_grade.numerator))
            .ok_or(Error::Arithmetic)?;
        if rise > run {
            return Err(Error::Grade);
        }
        Ok(())
    }
    /// Cardinal rules also apply to all four edges of a diagonal. Call sweep for
    /// body clearance; this cell-level check alone never authorizes a movement.
    pub fn transition(
        &self,
        surface: &impl Surface,
        a: Cell,
        b: Cell,
        profile: Traversal,
        budget: &mut Budget,
    ) -> Result<(), Error> {
        self.valid()?;
        if profile.max_grade.denominator == 0 {
            return Err(Error::Invalid);
        }
        self.rect(a)?;
        self.rect(b)?;
        let (dx, dy) = (a.x.abs_diff(b.x), a.y.abs_diff(b.y));
        if dx == 0 && dy == 0 {
            self.read(surface, a, budget)?;
            return Ok(());
        }
        if dx > 1 || dy > 1 {
            return Err(Error::NotAdjacent);
        }
        if dx == 0 || dy == 0 {
            return self.cardinal(surface, a, b, profile, budget);
        }
        let x = Cell { x: b.x, y: a.y };
        let y = Cell { x: a.x, y: b.y };
        for (from, to) in [(a, x), (a, y), (x, b), (y, b)] {
            self.cardinal(surface, from, to, profile, budget)?;
        }
        Ok(())
    }
}
fn distance_rect(p: P, r: Rect) -> ResultQ {
    let dx = if p.x.lt(r.left)? {
        r.left.sub(p.x)?
    } else if r.right.lt(p.x)? {
        p.x.sub(r.right)?
    } else {
        Q::ZERO
    };
    let dy = if p.y.lt(r.bottom)? {
        r.bottom.sub(p.y)?
    } else if r.top.lt(p.y)? {
        p.y.sub(r.top)?
    } else {
        Q::ZERO
    };
    dx.square()?.add(dy.square()?)
}
fn ray_interval(
    a: &[Q],
    delta: &[Q],
    low: &[Q],
    high: &[Q],
    mut first: Q,
    mut last: Option<Q>,
) -> Result<Option<(Q, Option<Q>)>, Error> {
    for i in 0..a.len() {
        if delta[i].n == 0 {
            if a[i].lt(low[i])? || high[i].lt(a[i])? {
                return Ok(None);
            }
        } else {
            let p = low[i].sub(a[i])?.div(delta[i])?;
            let q = high[i].sub(a[i])?.div(delta[i])?;
            first = first.max(p.min(q)?)?;
            let end = p.max(q)?;
            last = Some(if let Some(last) = last {
                last.min(end)?
            } else {
                end
            });
            if let Some(last) = last
                && last.lt(first)?
            {
                return Ok(None);
            }
        }
    }
    Ok(Some((first, last)))
}
/// Exact squared-distance test for a complete segment interval against a cell.
fn capsule_touches(a: P, delta: P, first: Q, last: Q, rect: Rect, r2: Q) -> Result<bool, Error> {
    if ray_interval(
        &[a.x, a.y],
        &[delta.x, delta.y],
        &[rect.left, rect.bottom],
        &[rect.right, rect.top],
        first,
        Some(last),
    )?
    .is_some()
    {
        return Ok(true);
    }
    if distance_rect(a.at(delta, first)?, rect)?.lt(r2)?
        || distance_rect(a.at(delta, last)?, rect)?.lt(r2)?
    {
        return Ok(true);
    }
    let length2 = delta.dot(delta)?;
    if length2.n == 0 {
        return Ok(false);
    }
    for x in [rect.left, rect.right] {
        for y in [rect.bottom, rect.top] {
            let offset = P { x, y }.sub(a)?;
            let t = offset.dot(delta)?.div(length2)?;
            if first.le(t)? && t.le(last)? {
                let cross = offset.x.mul(delta.y)?.sub(offset.y.mul(delta.x)?)?;
                if cross.square()?.lt(r2.mul(length2)?)? {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

// One sweep has immutable a/delta/grid. Candidate ordinals therefore identify
// exact operands for these terms across event intervals and their refinements.
// Only successes are retained, at the original first-use operation. Later cells
// use the unchanged arithmetic; this is not an admission or geometry limit.
const MEMO_CELLS: usize = 16;
#[derive(Clone, Copy)]
struct CellArithmetic {
    // Slab p/q for x/y, four corner projection times, four corner cross squares.
    values: [Q; 12],
    valid: u16,
}
impl CellArithmetic {
    const EMPTY: Self = Self {
        values: [Q::ZERO; 12],
        valid: 0,
    };
}
struct SweepArithmetic {
    cells: [CellArithmetic; MEMO_CELLS],
    // Same immutable grid and candidate ordinal throughout this sweep. Cache
    // only successful original first uses; never move a checked operation earlier.
    rectangles: [Option<Rect>; 4],
}
impl SweepArithmetic {
    fn rectangle(&mut self, grid: &Grid, ordinal: usize, cell: Cell) -> Result<Rect, Error> {
        let Some(entry) = self.rectangles.get_mut(ordinal) else {
            return grid.rect(cell);
        };
        if let Some(rect) = *entry {
            return Ok(rect);
        }
        let rect = grid.rect(cell)?;
        *entry = Some(rect);
        Ok(rect)
    }
}
const _: () = assert!(std::mem::size_of::<SweepArithmetic>() <= 7 * 1024);
const _: () = assert!(
    std::mem::size_of::<SweepArithmetic>() + std::mem::size_of::<IntervalCapsule>() <= 8 * 1024
);
fn cell_term(
    memo: &mut Option<&mut CellArithmetic>,
    index: usize,
    compute: impl FnOnce() -> ResultQ,
) -> ResultQ {
    let Some(memo) = memo.as_deref_mut() else {
        return compute();
    };
    let mask = 1 << index;
    if memo.valid & mask != 0 {
        return Ok(memo.values[index]);
    }
    let value = compute()?;
    memo.values[index] = value;
    memo.valid |= mask;
    Ok(value)
}
fn cell_ray_interval(
    a: P,
    delta: P,
    rect: Rect,
    mut first: Q,
    mut last: Q,
    mut memo: Option<&mut CellArithmetic>,
) -> Result<bool, Error> {
    if memo.is_none() {
        return Ok(ray_interval(
            &[a.x, a.y],
            &[delta.x, delta.y],
            &[rect.left, rect.bottom],
            &[rect.right, rect.top],
            first,
            Some(last),
        )?
        .is_some());
    }
    for (i, (a, delta, low, high)) in [
        (a.x, delta.x, rect.left, rect.right),
        (a.y, delta.y, rect.bottom, rect.top),
    ]
    .into_iter()
    .enumerate()
    {
        if delta.n == 0 {
            if a.lt(low)? || high.lt(a)? {
                return Ok(false);
            }
        } else {
            let p = cell_term(&mut memo, 2 * i, || low.sub(a)?.div(delta))?;
            let q = cell_term(&mut memo, 2 * i + 1, || high.sub(a)?.div(delta))?;
            first = first.max(p.min(q)?)?;
            let end = p.max(q)?;
            last = last.min(end)?;
            if last.lt(first)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Invariants for one interval whose exact endpoints were already evaluated by
/// interval_support. Keep fallible corner-only terms lazy: an earlier ray or
/// endpoint hit must not acquire a new arithmetic refusal.
struct IntervalCapsule {
    a: P,
    delta: P,
    first: Q,
    last: Q,
    start: P,
    end: P,
    radius2: Q,
    length2: Option<ResultQ>,
    threshold: Option<ResultQ>,
    filter: Option<IntervalFilter>,
}
impl IntervalCapsule {
    fn touches(
        &mut self,
        rect: Rect,
        mut memo: Option<&mut CellArithmetic>,
    ) -> Result<bool, Error> {
        if let Some(result) = self.filter.and_then(|filter| filter.touches(rect)) {
            return Ok(result);
        }
        let Self {
            a,
            delta,
            first,
            last,
            start,
            end,
            radius2,
            ..
        } = *self;
        if cell_ray_interval(a, delta, rect, first, last, memo.as_deref_mut())? {
            return Ok(true);
        }
        if distance_rect(start, rect)?.lt(radius2)? || distance_rect(end, rect)?.lt(radius2)? {
            return Ok(true);
        }
        let length2 = (*self.length2.get_or_insert_with(|| delta.dot(delta)))?;
        if length2.n == 0 {
            return Ok(false);
        }
        for (ix, x) in [rect.left, rect.right].into_iter().enumerate() {
            for (iy, y) in [rect.bottom, rect.top].into_iter().enumerate() {
                let corner = 2 * ix + iy;
                let mut offset = None;
                let t = cell_term(&mut memo, 4 + corner, || {
                    let value = P { x, y }.sub(a)?;
                    offset = Some(value);
                    value.dot(delta)?.div(length2)
                })?;
                if first.le(t)? && t.le(last)? {
                    // A cached projection already proved this offset representable.
                    // Recompute it only if cross² is first needed in a later interval.
                    let cross2 = cell_term(&mut memo, 8 + corner, || {
                        let offset = match offset {
                            Some(value) => value,
                            None => P { x, y }.sub(a)?,
                        };
                        let cross = offset.x.mul(delta.y)?.sub(offset.y.mul(delta.x)?)?;
                        cross.square()
                    })?;
                    // Preserve cross-square failure before threshold multiplication.
                    let threshold = (*self.threshold.get_or_insert_with(|| radius2.mul(length2)))?;
                    if cross2.lt(threshold)? {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
#[path = "geometry_interval_tests.rs"]
mod interval_tests;

// Dyadic event scheduling does not quantize actor poses. Every scheduled interval
// is certified as a continuous segment using exact predicates. An unresolved
// final interval reports Uncertain rather than treating its endpoints as proof.
const TIME_PARTS: u32 = 1 << 20;
fn time(t: u32) -> Q {
    Q {
        n: i128::from(t),
        d: i128::from(TIME_PARTS),
    }
}
fn schedule(events: &mut Vec<u32>, t: f64, budget: &mut Budget) -> Result<(), Error> {
    if t > 0.0 && t < 1.0 && t.is_finite() {
        budget.take()?;
        let scaled = t * f64::from(TIME_PARTS);
        events.push(scaled.floor() as u32);
        events.push(scaled.ceil() as u32);
    }
    Ok(())
}
impl Grid {
    #[allow(
        clippy::too_many_arguments,
        reason = "Exact sweep coordinates, interval, profile and budget are distinct geometric inputs"
    )]
    fn interval_support<const REFERENCE: bool>(
        &self,
        surface: &impl Surface,
        a: P,
        delta: P,
        first: u32,
        last: u32,
        radius2: Q,
        profile: Traversal,
        candidates: &[Cell],
        arithmetic: &mut SweepArithmetic,
        budget: &mut Budget,
    ) -> Result<(), Error> {
        let (lo, hi) = (time(first), time(last));
        let start_point = a.at(delta, lo)?;
        let start = self.cell_q(start_point)?;
        let end_point = a.at(delta, hi)?;
        let end = self.cell_q(end_point)?;
        self.transition(surface, start, end, profile, budget)?;
        let mut capsule = IntervalCapsule {
            a,
            delta,
            first: lo,
            last: hi,
            start: start_point,
            end: end_point,
            radius2,
            length2: None,
            threshold: None,
            filter: IntervalFilter::prepare(start_point, end_point, radius2),
        };
        let (mut minimum, mut maximum) = (i32::MAX, i32::MIN);
        for (ordinal, &cell) in candidates.iter().enumerate() {
            budget.take()?;
            let rect = if REFERENCE {
                self.rect(cell)?
            } else {
                arithmetic.rectangle(self, ordinal, cell)?
            };
            let potential = if radius2.n == 0 {
                // Keep only the owning side when a center travels on a grid edge.
                let constant_outside = (delta.x.n == 0
                    && (a.x.lt(rect.left)? || (rect.right.le(a.x)? && cell.x + 1 != self.columns)))
                    || (delta.y.n == 0
                        && (a.y.lt(rect.bottom)?
                            || (rect.top.le(a.y)? && cell.y + 1 != self.rows)));
                !constant_outside
                    && if REFERENCE {
                        capsule_touches(a, delta, lo, hi, rect, radius2)?
                    } else {
                        capsule.touches(rect, arithmetic.cells.get_mut(ordinal))?
                    }
            } else if REFERENCE {
                capsule_touches(a, delta, lo, hi, rect, radius2)?
            } else {
                capsule.touches(rect, arithmetic.cells.get_mut(ordinal))?
            };
            if !potential {
                continue;
            }
            let elevation = self.read(surface, cell, budget)?;
            minimum = minimum.min(elevation);
            maximum = maximum.max(elevation);
        }
        if i64::from(maximum) - i64::from(minimum) > i64::from(profile.max_step) {
            return Err(Error::Step);
        }
        Ok(())
    }
    /// Verify an entire swept disk, with bounded analytic-event scheduling and
    /// exact continuous interval certificates. Any unfinished interval stops at
    /// the last certified fraction; it is never accepted from samples alone.
    pub fn sweep(
        &self,
        surface: &impl Surface,
        from: Point,
        to: Point,
        radius: f64,
        profile: Traversal,
        budget: &mut Budget,
    ) -> Sweep {
        self.sweep_impl::<false>(surface, from, to, radius, profile, budget)
    }
    // REFERENCE selects the unchanged capsule helper only in differential tests.
    fn sweep_impl<const REFERENCE: bool>(
        &self,
        surface: &impl Surface,
        from: Point,
        to: Point,
        radius: f64,
        profile: Traversal,
        budget: &mut Budget,
    ) -> Sweep {
        let mut verified = 0;
        let result = (|| {
            let a = P::point(from)?;
            let b = P::point(to)?;
            let r = Q::float(radius)?;
            if REFERENCE {
                self.support_impl::<false>(surface, a, r, profile.max_step, budget)?;
                self.support_impl::<false>(surface, b, r, profile.max_step, budget)?;
            } else {
                self.support_q(surface, a, r, profile.max_step, budget)?;
                self.support_q(surface, b, r, profile.max_step, budget)?;
            }
            if profile.max_grade.denominator == 0 {
                return Err(Error::Invalid);
            }
            let (low, high) = self.bounds(a, b, r)?;
            let delta = b.sub(a)?;
            let mut events = vec![0, TIME_PARTS];
            let mut candidates = Vec::new();
            let dx = to.x - from.x;
            let dy = to.y - from.y;
            let length2 = dx * dx + dy * dy;
            for y in low.y..=high.y {
                for x in low.x..=high.x {
                    budget.take()?;
                    let cell = Cell { x, y };
                    candidates.push(cell);
                    let rect = self.rect(cell)?;
                    let (left, right, bottom, top) = (
                        rect.left.value(),
                        rect.right.value(),
                        rect.bottom.value(),
                        rect.top.value(),
                    );
                    for edge in [left - radius, left, right, right + radius] {
                        if dx != 0.0 {
                            schedule(&mut events, (edge - from.x) / dx, budget)?;
                        }
                    }
                    for edge in [bottom - radius, bottom, top, top + radius] {
                        if dy != 0.0 {
                            schedule(&mut events, (edge - from.y) / dy, budget)?;
                        }
                    }
                    if length2 > 0.0 && length2.is_finite() {
                        for cx in [left, right] {
                            for cy in [bottom, top] {
                                let ox = from.x - cx;
                                let oy = from.y - cy;
                                let linear = ox * dx + oy * dy;
                                let constant = ox * ox + oy * oy - radius * radius;
                                let discriminant = linear * linear - length2 * constant;
                                if discriminant >= 0.0 {
                                    let root = discriminant.sqrt();
                                    schedule(&mut events, (-linear - root) / length2, budget)?;
                                    schedule(&mut events, (-linear + root) / length2, budget)?;
                                }
                            }
                        }
                    }
                }
            }
            events.sort_unstable();
            events.dedup();
            let r2 = r.square()?;
            let mut arithmetic = SweepArithmetic {
                cells: [CellArithmetic::EMPTY; MEMO_CELLS],
                rectangles: [None; 4],
            };
            for pair in events.windows(2) {
                let mut stack = vec![(pair[0], pair[1])];
                while let Some((first, last)) = stack.pop() {
                    budget.take()?;
                    match self.interval_support::<REFERENCE>(
                        surface,
                        a,
                        delta,
                        first,
                        last,
                        r2,
                        profile,
                        &candidates,
                        &mut arithmetic,
                        budget,
                    ) {
                        Ok(()) => verified = last,
                        Err(Error::Step | Error::NotAdjacent | Error::Grade)
                            if last - first > 1 =>
                        {
                            let middle = first + (last - first) / 2;
                            stack.push((middle, last));
                            stack.push((first, middle));
                        }
                        Err(Error::Step | Error::NotAdjacent | Error::Grade) => {
                            return Err(Error::Uncertain);
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            Ok(())
        })();
        Sweep {
            numerator: verified,
            denominator: TIME_PARTS,
            result,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProbeInterval {
    Clear,
    Constrained,
    Unresolved,
}
struct SupportProbe {
    from: P,
    delta: P,
    radius2: Q,
    cells: Vec<(Rect, i32)>,
}
impl SupportProbe {
    fn classify(
        &self,
        first: u32,
        last: u32,
        max_step: u32,
        budget: &mut Budget,
    ) -> Result<ProbeInterval, Error> {
        let (lo, hi) = (time(first), time(last));
        let a = self.from.at(self.delta, lo)?;
        let b = self.from.at(self.delta, hi)?;
        let (mut potential_min, mut potential_max) = (i32::MAX, i32::MIN);
        let (mut persistent_min, mut persistent_max) = (i32::MAX, i32::MIN);
        for &(rect, height) in &self.cells {
            budget.take()?;
            if !capsule_touches(self.from, self.delta, lo, hi, rect, self.radius2)? {
                continue;
            }
            potential_min = potential_min.min(height);
            potential_max = potential_max.max(height);
            // A cell's positive-area disk contact times form an interval: the distance
            // from a moving point to a convex rectangle is convex. Strict contact at
            // both ends therefore proves strict contact throughout this interval.
            if distance_rect(a, rect)?.lt(self.radius2)?
                && distance_rect(b, rect)?.lt(self.radius2)?
            {
                persistent_min = persistent_min.min(height);
                persistent_max = persistent_max.max(height);
            }
        }
        if i64::from(potential_max) - i64::from(potential_min) <= i64::from(max_step) {
            return Ok(ProbeInterval::Clear);
        }
        if i64::from(persistent_max) - i64::from(persistent_min) > i64::from(max_step) {
            return Ok(ProbeInterval::Constrained);
        }
        Ok(ProbeInterval::Unresolved)
    }
}
impl Grid {
    fn support_probe(
        &self,
        surface: &impl Surface,
        from: P,
        delta: P,
        offset: P,
        radius: Q,
        budget: &mut Budget,
    ) -> Result<SupportProbe, Error> {
        let from = P {
            x: from.x.add(offset.x)?,
            y: from.y.add(offset.y)?,
        };
        let to = from.at(delta, Q::new(1, 1)?)?;
        let (min, max) = self.bounds(from, to, radius)?;
        let radius2 = radius.square()?;
        let mut cells = Vec::new();
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                budget.take()?;
                let cell = Cell { x, y };
                let rect = self.rect(cell)?;
                if capsule_touches(from, delta, time(0), time(TIME_PARTS), rect, radius2)? {
                    cells.push((rect, self.read(surface, cell, budget)?));
                }
            }
        }
        Ok(SupportProbe {
            from,
            delta,
            radius2,
            cells,
        })
    }
    /// Right-limit support uses only current positive-area cells and cells entered
    /// immediately from exact tangency. A strictly negative distance derivative
    /// proves an open contact interval, without mistaking an outward bracket for equality.
    #[allow(
        clippy::too_many_arguments,
        reason = "Exact pose, velocity, support and caller budget remain separate inputs"
    )]
    fn right_limit_constrained(
        &self,
        surface: &impl Surface,
        point: P,
        delta: P,
        radius: Q,
        max_step: u32,
        budget: &mut Budget,
    ) -> Result<bool, Error> {
        let (min, max) = self.bounds(point, point, radius)?;
        // A probe that leaves the public map immediately has no complete support
        // certificate. Known in-map heights cannot substitute for missing support.
        for (coordinate, velocity, count) in [
            (point.x, delta.x, self.columns),
            (point.y, delta.y, self.rows),
        ] {
            if (coordinate.sub(radius)?.n == 0 && velocity.n < 0)
                || (coordinate.add(radius)?.cmp(self.axis(count)?)? == Ordering::Equal
                    && velocity.n > 0)
            {
                return Err(Error::OutOfBounds);
            }
        }
        let radius2 = radius.square()?;
        let (mut minimum, mut maximum) = (i32::MAX, i32::MIN);
        // The ordinary lower half-open bound omits the cell just below an exact
        // tangency. Include its neighbour, then apply the exact contact predicate.
        for y in min.y.saturating_sub(1)..=max.y {
            for x in min.x.saturating_sub(1)..=max.x {
                budget.take()?;
                let cell = Cell { x, y };
                let rect = self.rect(cell)?;
                let contact = match distance_rect(point, rect)?.cmp(radius2)? {
                    Ordering::Less => true,
                    Ordering::Greater => false,
                    Ordering::Equal => {
                        let component = |value: Q, low: Q, high: Q| -> ResultQ {
                            if value.lt(low)? {
                                value.sub(low)
                            } else if high.lt(value)? {
                                value.sub(high)
                            } else {
                                Ok(Q::ZERO)
                            }
                        };
                        let separation = P {
                            x: component(point.x, rect.left, rect.right)?,
                            y: component(point.y, rect.bottom, rect.top)?,
                        };
                        // Squared distance to a convex rectangle is differentiable.
                        // Its derivative is 2*separation.dot(delta); only its exact sign
                        // matters. Zero/positive derivatives do not prove entry.
                        separation.dot(delta)?.n < 0
                    }
                };
                if contact {
                    let height = self.read(surface, cell, budget)?;
                    minimum = minimum.min(height);
                    maximum = maximum.max(height);
                }
            }
        }
        Ok(i64::from(maximum) - i64::from(minimum) > i64::from(max_step))
    }
    /// Inspect two fixed-offset disk probes along one continuous segment. A probe
    /// constrains the actor only when its simultaneous positive-area support heights
    /// exceed max_step. This does not test path grade or authorize centered movement.
    ///
    /// Offsets and poses retain their exact supplied binary values. The caller must
    /// certify the centered actor segment separately. Unknown/OOB probes, arithmetic
    /// exhaustion, work exhaustion, and unresolved 2^-20 intervals return Unresolved.
    #[allow(
        clippy::too_many_arguments,
        reason = "Two probe offsets, body support and caller-owned budget are separate geometric inputs"
    )]
    pub fn first_dual_support_contact(
        &self,
        surface: &impl Surface,
        from: Point,
        to: Point,
        offsets: [Point; 2],
        radius: f64,
        max_step: u32,
        budget: &mut Budget,
    ) -> DualSupportContact {
        let query = (|| -> Result<DualSupportContact, Error> {
            self.valid()?;
            let from = P::point(from)?;
            let to = P::point(to)?;
            let delta = to.sub(from)?;
            let radius = Q::float(radius)?;
            if radius.n <= 0 {
                return Err(Error::Invalid);
            }
            let offsets = [P::point(offsets[0])?, P::point(offsets[1])?];
            let constrained = |offset: P, budget: &mut Budget| -> Result<bool, Error> {
                let point = P {
                    x: from.x.add(offset.x)?,
                    y: from.y.add(offset.y)?,
                };
                match self.support_q(surface, point, radius, max_step, budget) {
                    Ok(_) => Ok(false),
                    Err(Error::Step) => Ok(true),
                    Err(error) => Err(error),
                }
            };
            // Current contact cannot depend on cells visited only later in the segment.
            let initial = [
                constrained(offsets[0], budget)?,
                constrained(offsets[1], budget)?,
            ];
            if initial[0] && initial[1] {
                return Ok(DualSupportContact::Inside);
            }
            let right_limit = |offset: P, budget: &mut Budget| {
                self.right_limit_constrained(
                    surface,
                    P {
                        x: from.x.add(offset.x)?,
                        y: from.y.add(offset.y)?,
                    },
                    delta,
                    radius,
                    max_step,
                    budget,
                )
            };
            if right_limit(offsets[0], budget)? && right_limit(offsets[1], budget)? {
                // There are finitely many witnessed support cells. Their strict
                // contacts share some positive time interval after zero, proving
                // an exact entry infimum of zero rather than merely overlapping bounds.
                return Ok(DualSupportContact::Transition {
                    lower: 0,
                    upper: 0,
                    denominator: 1,
                });
            }
            let probes = [
                self.support_probe(surface, from, delta, offsets[0], radius, budget)?,
                self.support_probe(surface, from, delta, offsets[1], radius, budget)?,
            ];
            let mut stack = vec![(0, TIME_PARTS)];
            while let Some((first, last)) = stack.pop() {
                budget.take()?;
                let left = probes[0].classify(first, last, max_step, budget)?;
                let right = probes[1].classify(first, last, max_step, budget)?;
                if left == ProbeInterval::Clear || right == ProbeInterval::Clear {
                    continue;
                }
                if last - first <= 1 {
                    // Earlier intervals were continuously certified outside. An actual
                    // simultaneous witness is required before publishing an entry bound.
                    let at = |part, budget: &mut Budget| -> Result<bool, Error> {
                        Ok(probes[0].classify(part, part, max_step, budget)?
                            == ProbeInterval::Constrained
                            && probes[1].classify(part, part, max_step, budget)?
                                == ProbeInterval::Constrained)
                    };
                    return if at(first, budget)? || at(last, budget)? {
                        Ok(DualSupportContact::Transition {
                            lower: first,
                            upper: last,
                            denominator: TIME_PARTS,
                        })
                    } else {
                        Err(Error::Uncertain)
                    };
                }
                // Chronological subdivision proves the intersection of both contact
                // sets, including exits and reentries. Endpoint samples alone never
                // skip an interval, and no monotonic blocked-pose assumption is made.
                let middle = first + (last - first) / 2;
                stack.push((middle, last));
                stack.push((first, middle));
            }
            Ok(DualSupportContact::Outside)
        })();
        query.unwrap_or(DualSupportContact::Unresolved)
    }
}

#[derive(Clone, Copy)]
struct ExactHit {
    t: Q,
    position: [Q; 3],
    cell: Cell,
    face: Face,
    elevation: i32,
}
#[allow(
    clippy::too_many_arguments,
    reason = "Ray, face bounds and surface ownership remain explicit for exact hit certification"
)]
fn face_hit(
    origin: [Q; 3],
    direction: [Q; 3],
    axis: usize,
    plane: Q,
    low: [Q; 3],
    high: [Q; 3],
    cell: Cell,
    face: Face,
    elevation: i32,
) -> Result<Option<ExactHit>, Error> {
    let t = if direction[axis].n == 0 {
        if origin[axis].cmp(plane)? != Ordering::Equal {
            return Ok(None);
        }
        // A coplanar ray can enter a finite face after its origin. The
        // first slab intersection, rather than t=0, is its actual nearest hit.
        let Some((entry, _)) = ray_interval(&origin, &direction, &low, &high, Q::ZERO, None)?
        else {
            return Ok(None);
        };
        entry
    } else {
        plane.sub(origin[axis])?.div(direction[axis])?
    };
    if t.n < 0 {
        return Ok(None);
    }
    let mut position = [Q::ZERO; 3];
    for i in 0..3 {
        position[i] = origin[i].add(direction[i].mul(t)?)?;
        if position[i].lt(low[i])? || high[i].lt(position[i])? {
            return Ok(None);
        }
    }
    Ok(Some(ExactHit {
        t,
        position,
        cell,
        face,
        elevation,
    }))
}
fn nearer(candidate: ExactHit, best: Option<ExactHit>) -> Result<bool, Error> {
    let Some(best) = best else {
        return Ok(true);
    };
    Ok(match candidate.t.cmp(best.t)? {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => (candidate.cell, candidate.face) < (best.cell, best.face),
    })
}
impl Grid {
    /// Nearest plateau top or exposed wall, in world coordinates with Z up.
    /// Unknown columns whose bounding prism could hide an earlier hit fail closed.
    /// Walls are owned by the higher column and select its integer top elevation.
    pub fn ray_hit(
        &self,
        surface: &impl Surface,
        ray: Ray,
        budget: &mut Budget,
    ) -> Result<Option<RayHit>, Error> {
        self.ray_hit_impl::<true>(surface, ray, budget)
    }
    fn ray_hit_impl<const PRUNE: bool>(
        &self,
        surface: &impl Surface,
        ray: Ray,
        budget: &mut Budget,
    ) -> Result<Option<RayHit>, Error> {
        self.valid()?;
        let origin = [
            Q::float(ray.origin.x)?,
            Q::float(ray.origin.y)?,
            Q::float(ray.origin.z)?,
        ];
        let direction = [
            Q::float(ray.direction.x)?,
            Q::float(ray.direction.y)?,
            Q::float(ray.direction.z)?,
        ];
        if direction.iter().all(|v| v.n == 0) {
            return Err(Error::Invalid);
        }
        let (floor, ceiling) = (self.height(self.floor)?, self.height(self.ceiling)?);
        let (first_x, last_x, first_y, last_y) = if PRUNE {
            // Every legal surface lies inside these PUBLIC height bounds. Clip
            // the ray once, then inspect only the XY extent of that segment.
            // This does not query a hidden cell to choose its candidate set.
            let Some((enter, leave)) = ray_interval(
                &origin,
                &direction,
                &[Q::ZERO, Q::ZERO, floor],
                &[self.axis(self.columns)?, self.axis(self.rows)?, ceiling],
                Q::ZERO,
                None,
            )?
            else {
                return Ok(None);
            };
            let leave = leave.ok_or(Error::Arithmetic)?;
            let range = |axis: usize, count: u32| -> Result<(u32, u32), Error> {
                let a = origin[axis].add(direction[axis].mul(enter)?)?;
                let b = origin[axis].add(direction[axis].mul(leave)?)?;
                let low = a.min(b)?.div(self.axis(1)?)?;
                let high = a.max(b)?.div(self.axis(1)?)?;
                let mut first = u32::try_from(low.n / low.d).map_err(|_| Error::Arithmetic)?;
                // Closed face contacts include the cell on either side of an
                // exact grid boundary. Preserve the existing wall/top ties.
                if low.n % low.d == 0 && first > 0 {
                    first -= 1;
                }
                let last = u32::try_from(high.n / high.d).map_err(|_| Error::Arithmetic)?;
                Ok((first.min(count - 1), last.min(count - 1)))
            };
            let (first_x, last_x) = range(0, self.columns)?;
            let (first_y, last_y) = range(1, self.rows)?;
            (first_x, last_x, first_y, last_y)
        } else {
            (0, self.columns - 1, 0, self.rows - 1)
        };
        let mut best: Option<ExactHit> = None;
        let mut unknown: Option<(Q, Cell)> = None;
        for y in first_y..=last_y {
            for x in first_x..=last_x {
                budget.take()?;
                let cell = Cell { x, y };
                let r = self.rect(cell)?;
                // Broad phase also limits which unavailable cells affect the result.
                let Some((entry, _)) = ray_interval(
                    &origin,
                    &direction,
                    &[r.left, r.bottom, floor],
                    &[r.right, r.top, ceiling],
                    Q::ZERO,
                    None,
                )?
                else {
                    continue;
                };
                if let Some(best) = best
                    && best.t.lt(entry)?
                {
                    continue;
                }
                let Some(elevation) = surface.elevation(cell) else {
                    let replace = if let Some((old, old_cell)) = unknown {
                        entry.lt(old)? || (entry.cmp(old)? == Ordering::Equal && cell < old_cell)
                    } else {
                        true
                    };
                    if replace {
                        unknown = Some((entry, cell));
                    }
                    continue;
                };
                if elevation < self.floor || elevation > self.ceiling {
                    return Err(Error::Elevation);
                }
                let top = self.height(elevation)?;
                if let Some(hit) = face_hit(
                    origin,
                    direction,
                    2,
                    top,
                    [r.left, r.bottom, top],
                    [r.right, r.top, top],
                    cell,
                    Face::Top,
                    elevation,
                )? && self.cell_q(P {
                    x: hit.position[0],
                    y: hit.position[1],
                })? == cell
                    && nearer(hit, best)?
                {
                    best = Some(hit);
                }
                for (face, axis, plane, neighbor) in [
                    (
                        Face::XMin,
                        0,
                        r.left,
                        x.checked_sub(1).map(|x| Cell { x, y }),
                    ),
                    (
                        Face::XMax,
                        0,
                        r.right,
                        (x + 1 < self.columns).then_some(Cell { x: x + 1, y }),
                    ),
                    (
                        Face::YMin,
                        1,
                        r.bottom,
                        y.checked_sub(1).map(|y| Cell { x, y }),
                    ),
                    (
                        Face::YMax,
                        1,
                        r.top,
                        (y + 1 < self.rows).then_some(Cell { x, y: y + 1 }),
                    ),
                ] {
                    budget.take()?;
                    let below = if let Some(neighbor) = neighbor {
                        let Some(value) = surface.elevation(neighbor) else {
                            continue;
                        };
                        if value < self.floor || value > self.ceiling {
                            return Err(Error::Elevation);
                        }
                        value
                    } else {
                        self.floor
                    };
                    if below >= elevation {
                        continue;
                    }
                    let base = self.height(below)?;
                    let mut low = [r.left, r.bottom, base];
                    let mut high = [r.right, r.top, top];
                    low[axis] = plane;
                    high[axis] = plane;
                    if let Some(hit) = face_hit(
                        origin, direction, axis, plane, low, high, cell, face, elevation,
                    )? && nearer(hit, best)?
                    {
                        best = Some(hit);
                    }
                }
            }
        }
        if let Some((entry, cell)) = unknown {
            if let Some(hit) = best {
                if entry.le(hit.t)? {
                    return Err(Error::Unknown(cell));
                }
            } else {
                return Err(Error::Unknown(cell));
            }
        }
        best.map(|hit| {
            Ok(RayHit {
                cell: hit.cell,
                face: hit.face,
                parameter: hit.t.value(),
                position: Point3 {
                    x: hit.position[0].value(),
                    y: hit.position[1].value(),
                    z: hit.position[2].value(),
                },
                elevation: hit.elevation,
                top_height: self.height(hit.elevation)?.value(),
            })
        })
        .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unit_cells_match_checked_reference_for_unreduced_extremes_and_invalid_inputs() {
        for scale in [1, 1000, u32::MAX] {
            let grid = Grid::checked(12, 1, scale, scale, 1, -8, 8).unwrap();
            for n in [i128::MIN, -1, 0, 1, 2, 12, 13, i128::MAX / 12, i128::MAX] {
                for d in [0, 1, 2, 3, 1i128 << 124, i128::MAX / 2, i128::MAX] {
                    let p = P {
                        x: Q { n, d },
                        y: Q::ZERO,
                    };
                    let old = grid.cell_impl::<false>(p);
                    let new = grid.cell_q(p);
                    assert_eq!(new, old, "n={n},d={d},scale={scale}");
                }
            }
            for x in [
                0.,
                f64::from(0.3f32),
                1.,
                12f64.next_down(),
                12.,
                12f64.next_up(),
                -0.1,
            ] {
                let p = P::point(Point { x, y: 0. }).unwrap();
                assert_eq!(grid.cell_q(p), grid.cell_impl::<false>(p));
            }
            for width in [0, scale.saturating_sub(1), scale.saturating_add(1)] {
                let other = Grid {
                    cell_width: width,
                    ..grid
                };
                let p = P::point(Point { x: 0.3, y: 0. }).unwrap();
                assert_eq!(other.cell_q(p), other.cell_impl::<false>(p));
            }
        }
    }
    #[test]
    fn read_validation_matches_rectangle_error_order_on_raw_grids() {
        for columns in [0, 1, u32::MAX] {
            for rows in [0, 1, u32::MAX] {
                for length_scale in [0, 1, 1000, u32::MAX] {
                    for cell_width in [0, 1, 1000, u32::MAX] {
                        let g = Grid {
                            columns,
                            rows,
                            length_scale,
                            cell_width,
                            elevation_step: 0,
                            floor: 3,
                            ceiling: -3,
                        };
                        for x in [0, 1, u32::MAX - 1, u32::MAX] {
                            for y in [0, u32::MAX - 1, u32::MAX] {
                                let cell = Cell { x, y };
                                assert_eq!(
                                    g.validate_read_cell(cell),
                                    g.rect(cell).map(|_| ()),
                                    "grid={g:?},cell={cell:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
        use std::cell::Cell as Counter;
        for scale in [0, 1] {
            for pos in [Cell { x: 0, y: 0 }, Cell { x: 1, y: 0 }] {
                for credits in [0, 1] {
                    let g = Grid {
                        length_scale: scale,
                        ..grid()
                    };
                    let calls = Counter::new(0);
                    let surface = |_| {
                        calls.set(calls.get() + 1);
                        None
                    };
                    let mut b = Budget::new(credits);
                    let actual = g.read(&surface, pos, &mut b);
                    let actual_calls = calls.replace(0);
                    let mut old = Budget::new(credits);
                    let expected = (|| {
                        g.rect(pos)?;
                        old.take()?;
                        surface(pos).ok_or(Error::Unknown(pos))
                    })();
                    assert_eq!(
                        (actual, b.remaining(), actual_calls),
                        (expected, old.remaining(), calls.get())
                    );
                }
            }
        }
    }
    #[test]
    #[ignore = "Independent Python Fraction probe"]
    fn emit_unit_cell_fraction_cases() {
        let g = Grid::checked(12, 1, 1000, 1000, 250, -8, 8).unwrap();
        for n in [0, 1, 12, 13, i128::MAX / 12, i128::MAX] {
            for d in [1, 2, 3, 1i128 << 124, i128::MAX / 2, i128::MAX] {
                let result = g.cell_q(P {
                    x: Q { n, d },
                    y: Q::ZERO,
                });
                println!("UNIT_CELL {n} {d} {result:?}");
            }
        }
    }
    #[test]
    #[ignore = "Scoped exact ownership cost"]
    fn unit_cell_scoped_cost() {
        use std::{hint::black_box, time::Instant};
        let mut sums = Vec::new();
        for unit_grid in [true, false] {
            for fast in [false, true] {
                let g = Grid::checked(
                    32,
                    32,
                    if unit_grid { 1000 } else { 3 },
                    if unit_grid { 1000 } else { 2 },
                    1,
                    -8,
                    8,
                )
                .unwrap();
                let p = P::point(Point {
                    x: f64::from(12.314f32),
                    y: f64::from(12.719f32),
                })
                .unwrap();
                let start = Instant::now();
                let mut sum = 0u64;
                for _ in 0..200000 {
                    let cell = if fast {
                        g.cell_q(black_box(p))
                    } else {
                        g.cell_impl::<false>(black_box(p))
                    }
                    .unwrap();
                    sum += u64::from(cell.x + cell.y);
                }
                eprintln!(
                    "unit={unit_grid} fast={fast} elapsed={:?} checksum={sum}",
                    start.elapsed()
                );
                sums.push(sum);
            }
        }
        assert_eq!(sums[0], sums[1]);
        assert_eq!(sums[2], sums[3]);
    }
    #[test]
    #[ignore = "Scoped cost only; not an engine timing"]
    fn stationary_support_scoped_cost() {
        use std::{hint::black_box, time::Instant};
        let grid = Grid::checked(32, 32, 1000, 1000, 250, -8, 8).unwrap();
        let point = P::point(Point {
            x: 12.314,
            y: 12.719,
        })
        .unwrap();
        let radius = Q::float(1.1).unwrap();
        let mut checksums = Vec::new();
        for filtered in [false, true] {
            let start = Instant::now();
            let mut checksum = 0;
            for _ in 0..20000 {
                let mut budget = Budget::new(256);
                let result = if filtered {
                    grid.support_impl::<true>(
                        &|_| Some(0),
                        black_box(point),
                        radius,
                        0,
                        &mut budget,
                    )
                } else {
                    grid.support_impl::<false>(
                        &|_| Some(0),
                        black_box(point),
                        radius,
                        0,
                        &mut budget,
                    )
                }
                .unwrap();
                checksum += result.cells.len() + budget.remaining();
                black_box(result);
            }
            eprintln!(
                "support filtered={filtered} elapsed={:?} checksum={checksum}",
                start.elapsed()
            );
            checksums.push(checksum);
        }
        assert_eq!(checksums[0], checksums[1]);
    }
    #[test]
    fn stationary_filtered_support_matches_q_reads_and_budget() {
        use std::cell::RefCell;
        for (scale, width) in [(1, 1), (3, 2), (1000, 1000)] {
            let grid = Grid::checked(12, 12, scale, width, 1, -8, 8).unwrap();
            let w = f64::from(width) / f64::from(scale);
            for i in 0..96 {
                let point = P::point(Point {
                    x: (2.0 + f64::from(i % 7) * 0.37) * w,
                    y: (2.0 + f64::from(i % 9) * 0.29) * w,
                })
                .unwrap();
                let radius = Q::float([0.0, 0.3, 0.5, 1.0, 1.1][i as usize % 5] * w).unwrap();
                for limit in [0, 1, 4, 16, 256] {
                    for unknown in [false, true] {
                        let run = |filtered| {
                            let reads = RefCell::new(Vec::new());
                            let surface = |cell: Cell| {
                                reads.borrow_mut().push(cell);
                                if unknown && (cell.x + cell.y).is_multiple_of(7) {
                                    None
                                } else {
                                    Some(((cell.x + cell.y) % 3) as i32)
                                }
                            };
                            let mut budget = Budget::new(limit);
                            let result = if filtered {
                                grid.support_impl::<true>(&surface, point, radius, 2, &mut budget)
                            } else {
                                grid.support_impl::<false>(&surface, point, radius, 2, &mut budget)
                            };
                            (result, reads.into_inner(), budget.remaining())
                        };
                        assert_eq!(run(true), run(false), "scale={scale},i={i},limit={limit}");
                    }
                }
            }
        }
    }
    #[test]
    fn bounded_ray_candidates_match_the_original_full_scan_with_unknowns_and_ties() {
        let terrain = grid();
        for pattern in 0..4 {
            for unknown in [false, true] {
                let surface = |cell: Cell| {
                    if unknown && (cell.x + 2 * cell.y).is_multiple_of(7) {
                        return None;
                    }
                    Some(match pattern {
                        0 => 0,
                        1 => (cell.x % 4) as i32,
                        2 => ((cell.x + cell.y) % 5) as i32 - 2,
                        _ => {
                            if cell.x == 3 && cell.y < 5 {
                                8
                            } else {
                                -4
                            }
                        }
                    })
                };
                for (x, y, z) in [
                    (-1.0, 2.0, 8.0),
                    (0.0, 0.0, 0.0),
                    (3.0, 3.0, 8.0),
                    (3.0, 3.0, -2.0),
                    (8.0, 8.0, 2.0),
                    (9.0, 4.0, 1.0),
                    (2.5, 4.5, 0.0),
                ] {
                    for dx in [-1.0, 0.0, 1.0] {
                        for dy in [-1.0, 0.0, 1.0] {
                            for dz in [-1.0, 0.0, 1.0] {
                                let ray = Ray {
                                    origin: Point3 { x, y, z },
                                    direction: Point3 {
                                        x: dx,
                                        y: dy,
                                        z: dz,
                                    },
                                };
                                let actual = terrain.ray_hit(&surface, ray, &mut budget());
                                let expected =
                                    terrain.ray_hit_impl::<false>(&surface, ray, &mut budget());
                                assert_eq!(
                                    actual, expected,
                                    "pattern={pattern},unknown={unknown},ray={ray:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn camera_ray_uses_public_height_bounds_without_visiting_the_entire_map() {
        let terrain = Grid::checked(128, 128, 1000, 1000, 250, -8, 8).unwrap();
        let ray = Ray {
            origin: Point3 {
                x: 32.0,
                y: 16.0,
                z: 66.0,
            },
            direction: Point3 {
                x: 0.25,
                y: 0.5,
                z: -1.0,
            },
        };
        let mut small = Budget::new(100);
        let hit = terrain
            .ray_hit(&|_| Some(0), ray, &mut small)
            .unwrap()
            .unwrap();
        assert_eq!(hit.elevation, 0);
        let mut full = budget();
        assert_eq!(
            Some(hit),
            terrain
                .ray_hit_impl::<false>(&|_| Some(0), ray, &mut full)
                .unwrap()
        );
        assert!(1_000_000 - full.remaining() >= 128 * 128);
    }

    #[test]
    fn complete_cell_observation_matches_independent_positive_area_integer_oracle() {
        for width in 1..=4 {
            let terrain = Grid::checked(4, 4, 2, width, 1, -1, 2).unwrap();
            for fog_quarters in [1, 2, 4, 6] {
                let extent = (width * 2) as f32;
                let fog =
                    crate::visibility::Grid::checked(extent, extent, fog_quarters as f32 / 4.0)
                        .unwrap();
                for y in 0..4 {
                    for x in 0..4 {
                        let mut expected = Vec::new();
                        for fy in 0..fog.rows {
                            for fx in 0..fog.columns {
                                let overlaps = |a: u32, f: u32| {
                                    let a0 = a * width * 2;
                                    let a1 = (a + 1) * width * 2;
                                    let f0 = f * fog_quarters;
                                    let f1 = ((f + 1) * fog_quarters).min(width * 8);
                                    a0 < f1 && f0 < a1
                                };
                                if overlaps(x, fx) && overlaps(y, fy) {
                                    expected.push(fy * fog.columns + fx);
                                }
                            }
                        }
                        let mut actual = Vec::new();
                        assert_eq!(
                            terrain.cell_observed(Cell { x, y }, fog, &mut budget(), |id| {
                                actual.push(id);
                                true
                            }),
                            Ok(true)
                        );
                        assert_eq!(actual, expected);
                        for hidden in expected {
                            assert_eq!(
                                terrain.cell_observed(Cell { x, y }, fog, &mut budget(), |id| id
                                    != hidden),
                                Ok(false)
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn observation_handles_rounded_map_edges_and_exhaustion_without_disclosure() {
        let terrain = Grid::checked(7, 7, 10, 1, 1, -1, 2).unwrap();
        let fog = crate::visibility::Grid::checked(0.7, 0.7, 0.1).unwrap();
        assert_eq!(
            terrain.cell_observed(Cell { x: 6, y: 6 }, fog, &mut budget(), |_| true),
            Ok(true)
        );
        assert_eq!(
            terrain.cell_observed(Cell { x: 6, y: 6 }, fog, &mut Budget::new(0), |_| panic!(
                "budget must fail before visiting"
            )),
            Err(Error::Budget)
        );
        assert_eq!(
            terrain.cell_observed(Cell { x: 7, y: 0 }, fog, &mut budget(), |_| true),
            Err(Error::OutOfBounds)
        );
        let other = crate::visibility::Grid::checked(1.0, 1.0, 0.1).unwrap();
        assert_eq!(
            terrain.cell_observed(Cell { x: 0, y: 0 }, other, &mut budget(), |_| true),
            Err(Error::Invalid)
        );
    }

    fn grid() -> Grid {
        Grid::checked(8, 8, 1000, 1000, 250, -8, 8).unwrap()
    }
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    fn budget() -> Budget {
        Budget::new(1_000_000)
    }
    fn profile(step: u32) -> Traversal {
        Traversal {
            max_step: step,
            max_grade: Ratio {
                numerator: 1,
                denominator: 1,
            },
        }
    }
    #[test]
    fn half_open_cells_signed_zero_and_maximum_edge_share_one_conversion() {
        let g = grid();
        assert_eq!(g.point_to_cell(p(-0.0, 0.0)), Ok(Cell { x: 0, y: 0 }));
        assert_eq!(g.point_to_cell(p(1.0, 1.0)), Ok(Cell { x: 1, y: 1 }));
        assert_eq!(g.point_to_cell(p(8.0, 8.0)), Ok(Cell { x: 7, y: 7 }));
        assert_eq!(
            g.point_to_cell(p(f64::from_bits(1.0f64.to_bits() - 1), 0.0)),
            Ok(Cell { x: 0, y: 0 })
        );
        assert_eq!(g.point_to_cell(p(-0.01, 0.0)), Err(Error::OutOfBounds));
        assert!(g.point_to_cell(p(f64::NAN, 0.0)).is_err());
        assert!(g.point_to_cell(p(f64::INFINITY, 0.0)).is_err());
        assert_eq!(
            g.disk_support(&|_| Some(0), p(8.0, 4.0), 0.0, 0, &mut budget())
                .unwrap()
                .cells,
            [Cell { x: 7, y: 4 }]
        );
        assert_eq!(
            g.disk_support(&|_| Some(0), p(8.0, 4.0), 0.1, 0, &mut budget()),
            Err(Error::OutOfBounds)
        );
    }
    #[test]
    fn disks_exclude_tangencies_include_center_and_preserve_fractional_actor_poses() {
        let g = grid();
        let flat = |_| Some(0);
        assert_eq!(
            g.disk_support(&flat, p(1.5, 1.5), 0.5, 0, &mut budget())
                .unwrap()
                .cells,
            [Cell { x: 1, y: 1 }]
        );
        let body = g
            .disk_support(&flat, p(1.5, 1.5), f64::from(0.55f32), 0, &mut budget())
            .unwrap();
        assert_eq!(
            body.cells.len(),
            5,
            "0.55f32 must not snap down to a half-unit radius"
        );
        assert!(!body.cells.contains(&Cell { x: 0, y: 0 }));
        assert_eq!(
            g.disk_support(&flat, p(2.0, 2.0), 0.0, 0, &mut budget())
                .unwrap()
                .cells,
            [Cell { x: 2, y: 2 }]
        );
        let corner = g
            .disk_support(&flat, p(2.0, 2.0), 0.25, 0, &mut budget())
            .unwrap();
        assert_eq!(
            corner.cells,
            [
                Cell { x: 1, y: 1 },
                Cell { x: 2, y: 1 },
                Cell { x: 1, y: 2 },
                Cell { x: 2, y: 2 }
            ]
        );
        assert_eq!(
            g.disk_support(&flat, p(1.0, 1.0), -1.0, 0, &mut budget()),
            Err(Error::Invalid)
        );
    }
    #[test]
    fn support_anchors_at_highest_plateau_and_static_foundations_are_flat() {
        let g = grid();
        let heights = |c: Cell| Some(if c.x == 2 { 2 } else { 0 });
        assert_eq!(
            g.disk_support(&heights, p(2.0, 2.5), 0.4, 1, &mut budget()),
            Err(Error::Step)
        );
        let support = g
            .disk_support(&heights, p(2.0, 2.5), 0.4, 2, &mut budget())
            .unwrap();
        assert_eq!(
            (support.minimum, support.maximum, support.anchor),
            (0, 2, 0.5)
        );
        assert_eq!(
            g.flat_foundation(&heights, p(2.0, 2.5), 0.4, &mut budget()),
            Err(Error::Step)
        );
        assert!(
            g.disk_support(&|_| None, p(2.0, 2.0), 0.0, 0, &mut budget())
                .is_err()
        );
        assert!(
            g.disk_support(&heights, p(2.0, 2.0), 0.0, 0, &mut Budget::new(0))
                .is_err()
        );
    }
    #[test]
    fn grade_uses_exact_ratio_and_diagonals_check_both_intermediates() {
        let g = grid();
        let heights = |c: Cell| Some(if c.x == 2 { 2 } else { 0 });
        let a = Cell { x: 1, y: 1 };
        let b = Cell { x: 2, y: 1 };
        let exact = Traversal {
            max_step: 2,
            max_grade: Ratio {
                numerator: 1,
                denominator: 2,
            },
        };
        assert_eq!(g.transition(&heights, a, b, exact, &mut budget()), Ok(()));
        let low = Traversal {
            max_grade: Ratio {
                numerator: 1,
                denominator: 3,
            },
            ..exact
        };
        assert_eq!(
            g.transition(&heights, a, b, low, &mut budget()),
            Err(Error::Grade)
        );
        let corner = |c: Cell| Some(if c == (Cell { x: 2, y: 1 }) { 4 } else { 0 });
        assert_eq!(
            g.transition(&corner, a, Cell { x: 2, y: 2 }, profile(1), &mut budget()),
            Err(Error::Step)
        );
        assert_eq!(
            g.transition(
                &heights,
                a,
                b,
                Traversal {
                    max_grade: Ratio {
                        numerator: 1,
                        denominator: 0
                    },
                    ..exact
                },
                &mut budget()
            ),
            Err(Error::Invalid)
        );
    }
    #[test]
    fn continuous_sweep_cannot_skip_a_one_cell_cliff_or_narrow_body_gap() {
        let g = grid();
        let cliff = |c: Cell| Some(if c.x == 3 { 6 } else { 0 });
        let result = g.sweep(
            &cliff,
            p(1.5, 2.5),
            p(6.5, 2.5),
            0.25,
            profile(1),
            &mut budget(),
        );
        assert!(result.result.is_err());
        assert!(result.fraction() < 0.5);
        let flat = g.sweep(
            &|_| Some(0),
            p(1.5, 2.5),
            p(6.5, 2.5),
            0.25,
            profile(1),
            &mut budget(),
        );
        assert_eq!(flat.result, Ok(()));
        assert_eq!(flat.fraction(), 1.0);
        let channel = |c: Cell| Some(if c.y == 3 { 0 } else { 4 });
        assert_eq!(
            g.sweep(
                &channel,
                p(1.5, 3.5),
                p(6.5, 3.5),
                0.5,
                profile(0),
                &mut budget()
            )
            .result,
            Ok(())
        );
        assert!(
            g.sweep(
                &channel,
                p(1.5, 3.5),
                p(6.5, 3.5),
                f64::from(0.55f32),
                profile(0),
                &mut budget()
            )
            .result
            .is_err()
        );
    }
    #[test]
    fn swept_budget_and_unknown_surface_never_authorize_the_remaining_segment() {
        let g = grid();
        for work in [0, 1, 8, 30] {
            let result = g.sweep(
                &|_| Some(0),
                p(1.5, 2.5),
                p(6.5, 2.5),
                0.25,
                profile(0),
                &mut Budget::new(work),
            );
            assert!(result.result.is_err());
            assert!(result.numerator < result.denominator);
        }
        let unknown = |c: Cell| if c.x == 4 { None } else { Some(0) };
        let result = g.sweep(
            &unknown,
            p(1.5, 2.5),
            p(6.5, 2.5),
            0.25,
            profile(0),
            &mut budget(),
        );
        assert!(result.result.is_err());
        assert!(result.fraction() < 1.0);
    }
    #[test]
    fn ray_picking_selects_nearest_top_and_wall_owners_top_elevation() {
        let g = grid();
        let ridge = |c: Cell| Some(if c.x == 3 { 4 } else { 0 });
        let hit = g
            .ray_hit(
                &ridge,
                Ray {
                    origin: Point3 {
                        x: 3.5,
                        y: 2.5,
                        z: 5.0,
                    },
                    direction: Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: -1.0,
                    },
                },
                &mut budget(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            (hit.cell, hit.face, hit.elevation, hit.top_height),
            (Cell { x: 3, y: 2 }, Face::Top, 4, 1.0)
        );
        let wall = g
            .ray_hit(
                &ridge,
                Ray {
                    origin: Point3 {
                        x: 1.5,
                        y: 2.5,
                        z: 0.5,
                    },
                    direction: Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                },
                &mut budget(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            (wall.cell, wall.face, wall.elevation, wall.top_height),
            (Cell { x: 3, y: 2 }, Face::XMin, 4, 1.0)
        );
        assert_eq!(
            wall.position.z, 0.5,
            "wall selects top level without changing the actual hit location"
        );
        let edge = g
            .ray_hit(
                &|_| Some(0),
                Ray {
                    origin: Point3 {
                        x: 8.0,
                        y: 8.0,
                        z: 1.0,
                    },
                    direction: Point3 {
                        x: -0.0,
                        y: 0.0,
                        z: -1.0,
                    },
                },
                &mut budget(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(edge.cell, Cell { x: 7, y: 7 });
    }
    #[test]
    fn unknown_ray_columns_and_budget_fail_closed() {
        let g = grid();
        let ray = Ray {
            origin: Point3 {
                x: 2.5,
                y: 2.5,
                z: 2.0,
            },
            direction: Point3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        };
        assert!(matches!(
            g.ray_hit(&|_| None, ray, &mut budget()),
            Err(Error::Unknown(_))
        ));
        assert_eq!(
            g.ray_hit(&|_| Some(0), ray, &mut Budget::new(0)),
            Err(Error::Budget)
        );
        assert!(
            g.ray_hit(
                &|_| Some(0),
                Ray {
                    direction: Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0
                    },
                    ..ray
                },
                &mut budget()
            )
            .is_err()
        );
    }
    #[test]
    fn seeded_dyadic_disk_queries_match_an_independent_integer_cell_oracle() {
        let g = grid();
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..512 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = 16 + (state % 96) as i64;
            let y = 16 + ((state >> 8) % 96) as i64;
            let radius = ((state >> 16) % 8) as i64;
            // Integer coordinates are sixteenths of a world unit in this oracle.
            let point = p(x as f64 / 16.0, y as f64 / 16.0);
            let owning = Cell {
                x: (x / 16) as u32,
                y: (y / 16) as u32,
            };
            let mut expected = Vec::new();
            for cy in 0..8 {
                for cx in 0..8 {
                    let dx = (cx * 16 - x).max(0).max(x - (cx + 1) * 16);
                    let dy = (cy * 16 - y).max(0).max(y - (cy + 1) * 16);
                    let cell = Cell {
                        x: cx as u32,
                        y: cy as u32,
                    };
                    if cell == owning || dx * dx + dy * dy < radius * radius {
                        expected.push(cell);
                    }
                }
            }
            assert_eq!(
                g.disk_support(&|_| Some(0), point, radius as f64 / 16.0, 0, &mut budget())
                    .unwrap()
                    .cells,
                expected
            );
        }
    }
    #[test]
    fn exhaustive_small_ridge_sweeps_match_a_flat_route_oracle() {
        let g = Grid::checked(3, 2, 4, 4, 1, -1, 4).unwrap();
        for mask in 0..64u32 {
            let heights = |cell: Cell| Some(((mask >> (cell.y * 3 + cell.x)) & 1) as i32);
            for row in 0..2 {
                let expected =
                    (0..3).all(|x| heights(Cell { x, y: row }) == heights(Cell { x: 0, y: row }));
                let sweep = g.sweep(
                    &heights,
                    p(0.5, f64::from(row) + 0.5),
                    p(2.5, f64::from(row) + 0.5),
                    0.25,
                    profile(0),
                    &mut budget(),
                );
                assert_eq!(sweep.result.is_ok(), expected, "mask {mask}, row {row}");
                if expected {
                    assert_eq!(sweep.numerator, sweep.denominator);
                }
            }
        }
        let stairs = |cell: Cell| Some(cell.x as i32);
        assert_eq!(
            g.sweep(
                &stairs,
                p(0.5, 0.5),
                p(2.5, 0.5),
                0.25,
                profile(1),
                &mut budget()
            )
            .result,
            Ok(()),
            "separated support events must allow a valid staircase, rather than comparing the union of the whole path"
        );
        assert!(matches!(
            g.transition(
                &|_| None,
                Cell { x: 0, y: 0 },
                Cell { x: 0, y: 0 },
                profile(0),
                &mut budget()
            ),
            Err(Error::Unknown(_))
        ));
    }

    #[test]
    fn vertical_ray_oracle_uses_cell_ownership_and_exact_plateau_levels() {
        let g = grid();
        let heights = |cell: Cell| Some(((cell.x * 3 + cell.y * 5) % 7) as i32 - 3);
        for y in 0..8 {
            for x in 0..8 {
                let ray = Ray {
                    origin: Point3 {
                        x: f64::from(x) + 0.25,
                        y: f64::from(y) + 0.75,
                        z: 4.0,
                    },
                    direction: Point3 {
                        x: 0.0,
                        y: -0.0,
                        z: -2.0,
                    },
                };
                let hit = g.ray_hit(&heights, ray, &mut budget()).unwrap().unwrap();
                let expected = heights(Cell { x, y }).unwrap();
                assert_eq!(
                    (hit.cell, hit.face, hit.elevation),
                    (Cell { x, y }, Face::Top, expected)
                );
                assert_eq!(hit.position.z, f64::from(expected) * 0.25);
                assert_eq!(hit.parameter, (4.0 - f64::from(expected) * 0.25) / 2.0);
            }
        }
    }

    #[test]
    fn coplanar_ray_enters_a_finite_top_face_at_its_first_in_bounds_point() {
        let g = grid();
        let hit = g
            .ray_hit(
                &|_| Some(0),
                Ray {
                    origin: Point3 {
                        x: -1.0,
                        y: 2.5,
                        z: 0.0,
                    },
                    direction: Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                },
                &mut budget(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            (hit.cell, hit.face, hit.parameter),
            (Cell { x: 0, y: 2 }, Face::Top, 1.0)
        );
        assert_eq!(
            hit.position,
            Point3 {
                x: 0.0,
                y: 2.5,
                z: 0.0
            }
        );
    }

    #[test]
    fn widened_f32_flat_sweeps_preserve_continuous_poses_without_arithmetic_stalls() {
        let g = grid();
        let mut seed = 0x6a09_e667_f3bc_c909u64;
        for _ in 0..64 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let x = 1.0f32 + (seed as u32 % 5000) as f32 / 1000.0;
            let y = 1.0f32 + ((seed >> 32) as u32 % 5000) as f32 / 1000.0;
            let from = p(f64::from(x), f64::from(y));
            let to = p(f64::from(x + 0.125), f64::from(y - 0.0625));
            let result = g.sweep(
                &|_| Some(0),
                from,
                to,
                f64::from(0.55f32),
                profile(0),
                &mut budget(),
            );
            assert_eq!(result.result, Ok(()), "from {from:?} to {to:?}: {result:?}");
            assert_eq!(result.numerator, result.denominator);
        }
    }

    #[test]
    fn rational_numeric_limits_fail_closed_without_rounding_or_panicking() {
        assert_eq!(
            Q::float(f64::from(0.55f32)).unwrap().value(),
            f64::from(0.55f32)
        );
        assert!(Q::float(f64::MAX).is_err());
        assert!(Q::float(f64::from_bits(1)).is_err());
        assert_eq!(Q::float(-0.0), Ok(Q::ZERO));
        assert!(
            grid()
                .disk_support(&|_| Some(0), p(2.0, 2.0), f64::MAX, 0, &mut budget())
                .is_err()
        );
    }
    fn contact_grid() -> Grid {
        Grid::checked(16, 8, 1, 1, 1, -8, 8).unwrap()
    }
    #[test]
    fn dual_support_contact_ranks_actual_entry_not_fast_actor_eta() {
        let g = contact_grid();
        let surface = |cell: Cell| {
            Some(if (6..10).contains(&cell.x) && !(2..5).contains(&cell.y) {
                4
            } else {
                0
            })
        };
        let radius = f64::from(1.1_f32);
        let offsets = [p(0.0, 2.0 * radius), p(0.0, -2.0 * radius)];
        let query = |from: f32, speed: f32, direction: f32| {
            g.first_dual_support_contact(
                &surface,
                p(f64::from(from), 3.5),
                p(f64::from(from + direction * speed * 0.05_f32), 3.5),
                offsets,
                radius,
                0,
                &mut budget(),
            )
        };
        let left = query(4.895, 0.5, 1.0);
        let right = query(11.18, 2.0, -1.0);
        let DualSupportContact::Transition {
            lower: ll,
            upper: lu,
            denominator: ld,
        } = left
        else {
            panic!("left {left:?}");
        };
        let DualSupportContact::Transition {
            lower: rl,
            upper: ru,
            denominator: rd,
        } = right
        else {
            panic!("right {right:?}");
        };
        assert!(
            u64::from(lu) * u64::from(rd) < u64::from(rl) * u64::from(ld),
            "{left:?} versus {right:?}"
        );
        assert!(f64::from(ll) / f64::from(ld) > 0.19 && f64::from(lu) / f64::from(ld) < 0.21);
        assert!(f64::from(rl) / f64::from(rd) > 0.79 && f64::from(ru) / f64::from(rd) < 0.81);
    }
    #[test]
    fn dual_support_contact_intersects_non_monotone_flanks_instead_of_first_hit_maximum() {
        let g = contact_grid();
        let surface = |cell: Cell| {
            Some(
                if (cell.y == 5 && (2..4).contains(&cell.x))
                    || (cell.y == 2 && (7..9).contains(&cell.x))
                {
                    4
                } else {
                    0
                },
            )
        };
        let result = g.first_dual_support_contact(
            &surface,
            p(1.0, 4.0),
            p(11.0, 4.0),
            [p(0.0, 1.0), p(0.0, -1.0)],
            0.5,
            0,
            &mut budget(),
        );
        assert_eq!(result, DualSupportContact::Outside);
    }
    #[test]
    fn dual_support_contact_tangency_and_signed_zero_do_not_forge_inside() {
        let g = contact_grid();
        let surface = |cell: Cell| {
            Some(
                if (6..10).contains(&cell.x) && (cell.y == 2 || cell.y == 5) {
                    4
                } else {
                    0
                },
            )
        };
        let offsets = [p(-0.0, 1.0), p(0.0, -1.0)];
        let tangent = g.first_dual_support_contact(
            &surface,
            p(5.5, 4.0),
            p(5.5, 4.0),
            offsets,
            0.5,
            0,
            &mut budget(),
        );
        assert_eq!(tangent, DualSupportContact::Outside);
        let inside = g.first_dual_support_contact(
            &surface,
            p(6.0, 4.0),
            p(6.0, 4.0),
            offsets,
            0.5,
            0,
            &mut budget(),
        );
        assert_eq!(inside, DualSupportContact::Inside);
    }
    #[test]
    fn dual_support_contact_unknown_and_exhaustion_never_establish_inside() {
        let g = contact_grid();
        let offsets = [p(0.0, 1.0), p(0.0, -1.0)];
        let unknown = |_: Cell| None;
        assert_eq!(
            g.first_dual_support_contact(
                &unknown,
                p(4.0, 4.0),
                p(5.0, 4.0),
                offsets,
                0.5,
                0,
                &mut budget()
            ),
            DualSupportContact::Unresolved
        );
        assert_eq!(
            g.first_dual_support_contact(
                &|_: Cell| Some(0),
                p(4.0, 4.0),
                p(5.0, 4.0),
                offsets,
                0.5,
                0,
                &mut Budget::new(0)
            ),
            DualSupportContact::Unresolved
        );
    }
    #[test]
    fn dual_support_contact_bounds_match_axis_aligned_positive_area_oracle() {
        let g = contact_grid();
        let offsets = [p(0.0, 1.0), p(0.0, -1.0)];
        for upper in 0u32..16 {
            for lower in 0u32..16 {
                let surface = |cell: Cell| {
                    Some(
                        if (2..6).contains(&cell.x)
                            && ((cell.y == 5 && (upper >> (cell.x - 2)) & 1 != 0)
                                || (cell.y == 2 && (lower >> (cell.x - 2)) & 1 != 0))
                        {
                            4
                        } else {
                            0
                        },
                    )
                };
                let mut first: Option<f64> = None;
                for a in 0..4 {
                    for b in 0..4 {
                        if upper >> a & 1 == 0 || lower >> b & 1 == 0 {
                            continue;
                        }
                        let start = (f64::from(a + 2) - 0.5).max(f64::from(b + 2) - 0.5);
                        let end = (f64::from(a + 3) + 0.5).min(f64::from(b + 3) + 0.5);
                        if start < end {
                            first = Some(first.map_or(start, |prior| prior.min(start)));
                        }
                    }
                }
                let result = g.first_dual_support_contact(
                    &surface,
                    p(1.0, 4.0),
                    p(7.0, 4.0),
                    offsets,
                    0.5,
                    0,
                    &mut budget(),
                );
                match result {
                    DualSupportContact::Transition {
                        lower: lo,
                        upper: hi,
                        denominator,
                    } => {
                        let first = first
                            .unwrap_or_else(|| panic!("false contact upper={upper},lower={lower}"));
                        let expected = (first - 1.0) / 6.0;
                        assert!(
                            f64::from(lo) / f64::from(denominator) <= expected
                                && expected <= f64::from(hi) / f64::from(denominator),
                            "upper={upper},lower={lower}: {result:?}, expected {expected}"
                        );
                    }
                    DualSupportContact::Outside => {
                        assert!(first.is_none(), "missed upper={upper},lower={lower}")
                    }
                    DualSupportContact::Inside => {
                        panic!("starts clear upper={upper},lower={lower}")
                    }
                    // A sub-resolution contact swap without an overlapping witness is
                    // deliberately unresolved; it never becomes a false traffic winner.
                    DualSupportContact::Unresolved => {}
                }
            }
        }
    }
    #[test]
    fn dual_support_contact_proves_exact_zero_entry_at_both_binary_mouth_tangencies() {
        let g = Grid::checked(30, 30, 1, 1, 1, 0, 8).unwrap();
        let surface = |cell: Cell| {
            Some(if cell.x == 10 && !(9..12).contains(&cell.y) {
                4
            } else {
                0
            })
        };
        let r = 1.125;
        let offsets = [p(0.0, 2.0 * r), p(0.0, -2.0 * r)];
        for (x, direction) in [(8.875, 1.0), (12.125, -1.0)] {
            let result = g.first_dual_support_contact(
                &surface,
                p(x, 10.5),
                p(x + direction * 0.125, 10.5),
                offsets,
                r,
                0,
                &mut budget(),
            );
            assert_eq!(
                result,
                DualSupportContact::Transition {
                    lower: 0,
                    upper: 0,
                    denominator: 1
                }
            );
            for to in [p(x, 10.5), p(x - direction * 0.125, 10.5), p(x, 10.625)] {
                assert_eq!(
                    g.first_dual_support_contact(
                        &surface,
                        p(x, 10.5),
                        to,
                        offsets,
                        r,
                        0,
                        &mut budget()
                    ),
                    DualSupportContact::Outside
                );
            }
        }
    }
    #[test]
    fn dual_support_contact_zero_proof_requires_both_known_constraining_flanks() {
        let g = Grid::checked(30, 30, 1, 1, 1, 0, 8).unwrap();
        let offsets = [p(0.0, 2.25), p(0.0, -2.25)];
        let one = |cell: Cell| Some(if cell.x == 10 && cell.y >= 12 { 4 } else { 0 });
        assert_eq!(
            g.first_dual_support_contact(
                &one,
                p(8.875, 10.5),
                p(9.0, 10.5),
                offsets,
                1.125,
                0,
                &mut budget()
            ),
            DualSupportContact::Outside
        );
        let unknown = |cell: Cell| {
            if cell.x == 10 && cell.y >= 12 {
                None
            } else {
                Some(if cell.x == 10 && cell.y < 9 { 4 } else { 0 })
            }
        };
        assert_eq!(
            g.first_dual_support_contact(
                &unknown,
                p(8.875, 10.5),
                p(9.0, 10.5),
                offsets,
                1.125,
                0,
                &mut budget()
            ),
            DualSupportContact::Unresolved
        );
        let surface = |cell: Cell| {
            Some(if cell.x == 10 && !(9..12).contains(&cell.y) {
                4
            } else {
                0
            })
        };
        assert_eq!(
            g.first_dual_support_contact(
                &surface,
                p(8.875, 10.5),
                p(9.0, 10.5),
                offsets,
                1.125,
                4,
                &mut budget()
            ),
            DualSupportContact::Outside
        );
    }
    #[test]
    fn dual_support_contact_positive_delay_does_not_become_zero_from_an_overlapping_bracket() {
        let g = Grid::checked(30, 30, 1, 1, 1, 0, 8).unwrap();
        let surface = |cell: Cell| {
            Some(if cell.x == 10 && !(9..12).contains(&cell.y) {
                4
            } else {
                0
            })
        };
        let from = 8.875_f64.next_down();
        let result = g.first_dual_support_contact(
            &surface,
            p(from, 10.5),
            p(9.0, 10.5),
            [p(0.0, 2.25), p(0.0, -2.25)],
            1.125,
            0,
            &mut budget(),
        );
        assert_ne!(
            result,
            DualSupportContact::Transition {
                lower: 0,
                upper: 0,
                denominator: 1
            }
        );
        if let DualSupportContact::Transition { lower, upper, .. } = result {
            assert!(upper > lower);
        }
    }
    #[test]
    fn dual_support_contact_zero_entry_uses_exact_corner_derivative_not_tangent_motion() {
        let g = Grid::checked(30, 30, 1, 1, 1, 0, 8).unwrap();
        let surface = |cell: Cell| Some(if cell == (Cell { x: 10, y: 9 }) { 4 } else { 0 });
        let offsets = [p(0.0, 0.0), p(-0.0, 0.0)];
        let from = p(9.25, 8.0);
        assert_eq!(
            g.first_dual_support_contact(
                &surface,
                from,
                p(9.375, 8.125),
                offsets,
                1.25,
                0,
                &mut budget()
            ),
            DualSupportContact::Transition {
                lower: 0,
                upper: 0,
                denominator: 1
            }
        );
        assert_eq!(
            g.first_dual_support_contact(
                &surface,
                from,
                p(9.375, 7.90625),
                offsets,
                1.25,
                0,
                &mut budget()
            ),
            DualSupportContact::Outside
        );
    }
    #[test]
    fn swept_disks_clear_rejects_real_gm6_chord_with_separated_endpoints() {
        let fixed = p(f64::from(29.020_203_f32), 24.5);
        let large = f64::from(1.1_f32);
        let small = f64::from(0.4_f32);
        let from = p(f64::from(29.714_956_f32), f64::from(25.829_517_f32));
        let to = p(f64::from(29.846_33_f32), f64::from(25.752_123_f32));
        assert_eq!(
            swept_disks_clear(from, from, small, fixed, fixed, large, &mut budget()),
            Ok(true)
        );
        assert_eq!(
            swept_disks_clear(to, to, small, fixed, fixed, large, &mut budget()),
            Ok(true)
        );
        assert_eq!(
            swept_disks_clear(from, to, small, fixed, fixed, large, &mut budget()),
            Ok(false)
        );
        assert_eq!(
            swept_disks_clear(fixed, fixed, large, from, to, small, &mut budget()),
            Ok(false)
        );
        assert_eq!(
            swept_disks_clear(to, from, small, fixed, fixed, large, &mut budget()),
            Ok(false)
        );
    }
    #[test]
    fn swept_disks_clear_handles_crossing_and_common_moving_frames() {
        assert_eq!(
            swept_disks_clear(
                p(-2.0, 0.0),
                p(2.0, 0.0),
                0.25,
                p(0.0, -2.0),
                p(0.0, 2.0),
                0.75,
                &mut budget()
            ),
            Ok(false)
        );
        assert_eq!(
            swept_disks_clear(
                p(-2.0, 1.0),
                p(5.0, -1.0),
                0.75,
                p(0.0, 0.0),
                p(3.0, -2.0),
                0.25,
                &mut budget()
            ),
            Ok(true)
        );
    }
    #[test]
    fn swept_disks_clear_preserves_exact_tangency_and_rejects_next_radius() {
        let fixed = p(0.0, 0.0);
        let from = p(-2.0, 1.0);
        let to = p(2.0, 1.0);
        assert_eq!(
            swept_disks_clear(from, to, 0.75, fixed, fixed, 0.25, &mut budget()),
            Ok(true)
        );
        assert_eq!(
            swept_disks_clear(
                from,
                to,
                f64::from(0.75_f32.next_up()),
                fixed,
                fixed,
                0.25,
                &mut budget()
            ),
            Ok(false)
        );
        assert_eq!(
            swept_disks_clear(
                p(-2.0, -0.0),
                p(-1.0, 0.0),
                0.25,
                fixed,
                fixed,
                0.75,
                &mut budget()
            ),
            Ok(true)
        );
        assert_eq!(
            swept_disks_clear(
                p(-2.0, 0.0),
                p(-1.0, 0.0),
                f64::from(0.25_f32.next_up()),
                fixed,
                fixed,
                0.75,
                &mut budget()
            ),
            Ok(false)
        );
    }
    #[test]
    fn swept_disks_clear_separates_initial_overlap_from_monotone_recovery() {
        let fixed = p(0.0, 0.0);
        let initial = p(0.5, 0.0);
        assert_eq!(
            swept_disks_clear(
                initial,
                p(2.0, 0.0),
                0.25,
                fixed,
                fixed,
                0.75,
                &mut budget()
            ),
            Ok(false)
        );
        assert_eq!(
            swept_disks_non_deepening_overlap(
                initial,
                p(2.0, 0.0),
                0.25,
                fixed,
                fixed,
                0.75,
                &mut budget()
            ),
            Ok(true)
        );
        assert_eq!(
            swept_disks_non_deepening_overlap(
                initial,
                p(-0.5, 0.0),
                0.25,
                fixed,
                fixed,
                0.75,
                &mut budget()
            ),
            Ok(false)
        );
        assert_eq!(
            swept_disks_non_deepening_overlap(
                initial,
                initial,
                0.25,
                fixed,
                fixed,
                0.75,
                &mut budget()
            ),
            Ok(true)
        );
        assert_eq!(
            swept_disks_non_deepening_overlap(
                p(2.0, 0.0),
                p(3.0, 0.0),
                0.25,
                fixed,
                fixed,
                0.75,
                &mut budget()
            ),
            Ok(false)
        );
    }
    #[test]
    fn swept_disks_clear_errors_are_not_clearance() {
        let a = p(0.0, 0.0);
        let b = p(4.0, 0.0);
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                swept_disks_clear(a, a, invalid, b, b, 0.5, &mut budget()),
                Err(Error::Invalid)
            );
            assert_eq!(
                swept_disks_clear(a, a, 0.5, b, b, invalid, &mut budget()),
                Err(Error::Invalid)
            );
        }
        assert_eq!(
            swept_disks_clear(a, a, 0.5, b, b, 0.5, &mut Budget::new(0)),
            Err(Error::Budget)
        );
        assert_eq!(
            swept_disks_clear(p(f64::NAN, 0.0), a, 0.5, b, b, 0.5, &mut budget()),
            Err(Error::Invalid)
        );
        assert_eq!(
            swept_disks_clear(p(2.0_f64.powi(200), 0.0), a, 0.5, b, b, 0.5, &mut budget()),
            Err(Error::Arithmetic)
        );
    }
    #[test]
    fn swept_disks_clear_matches_independent_integer_quadratic_oracle() {
        let mut seed = 0x715c_b49a_8e31_6d02_u64;
        for _ in 0..4096 {
            let mut next = || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed % 33) as i128 - 16
            };
            let a = [next(), next()];
            let end_a = [next(), next()];
            let b = [next(), next()];
            let end_b = [next(), next()];
            let ra = next().unsigned_abs() as i128 % 5 + 1;
            let rb = next().unsigned_abs() as i128 % 5 + 1;
            let r = [a[0] - b[0], a[1] - b[1]];
            let v = [
                end_a[0] - a[0] - end_b[0] + b[0],
                end_a[1] - a[1] - end_b[1] + b[1],
            ];
            let c = r[0] * r[0] + r[1] * r[1] - (ra + rb) * (ra + rb);
            let linear = 2 * (r[0] * v[0] + r[1] * v[1]);
            let quadratic = v[0] * v[0] + v[1] * v[1];
            let expected = if c < 0 || quadratic + linear + c < 0 {
                false
            } else if quadratic == 0 || linear >= 0 || linear <= -2 * quadratic {
                true
            } else {
                linear * linear <= 4 * quadratic * c
            };
            let point = |q: [i128; 2]| p(q[0] as f64 / 4.0, q[1] as f64 / 4.0);
            let result = swept_disks_clear(
                point(a),
                point(end_a),
                ra as f64 / 4.0,
                point(b),
                point(end_b),
                rb as f64 / 4.0,
                &mut budget(),
            );
            assert_eq!(
                result,
                Ok(expected),
                "a={a:?}->{end_a:?},b={b:?}->{end_b:?},r={ra}/{rb}"
            );
            assert_eq!(
                swept_disks_clear(
                    point(b),
                    point(end_b),
                    rb as f64 / 4.0,
                    point(a),
                    point(end_a),
                    ra as f64 / 4.0,
                    &mut budget()
                ),
                Ok(expected)
            );
            assert_eq!(
                swept_disks_non_deepening_overlap(
                    point(a),
                    point(end_a),
                    ra as f64 / 4.0,
                    point(b),
                    point(end_b),
                    rb as f64 / 4.0,
                    &mut budget()
                ),
                Ok(c < 0 && linear >= 0)
            );
        }
    }
}
