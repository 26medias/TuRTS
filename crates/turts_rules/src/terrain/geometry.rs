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
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
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
        let g = i128::try_from(gcd(n.unsigned_abs(), d as u128)).map_err(|_| Error::Arithmetic)?;
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
        let a_scale = b.d / g;
        let b_scale = self.d / g;
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
        let g1 = gcd(self.n.unsigned_abs(), b.d as u128) as i128;
        let g2 = gcd(b.n.unsigned_abs(), self.d as u128) as i128;
        Self::new(
            (self.n / g1)
                .checked_mul(b.n / g2)
                .ok_or(Error::Arithmetic)?,
            (self.d / g2)
                .checked_mul(b.d / g1)
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
        let g = gcd(self.d as u128, b.d as u128) as i128;
        Ok(self
            .n
            .checked_mul(b.d / g)
            .ok_or(Error::Arithmetic)?
            .cmp(&b.n.checked_mul(self.d / g).ok_or(Error::Arithmetic)?))
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
        self.valid()?;
        let axis = |q: Q, count| -> Result<u32, Error> {
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
    fn read(&self, surface: &impl Surface, cell: Cell, budget: &mut Budget) -> Result<i32, Error> {
        self.rect(cell)?;
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
        let center = self.cell_q(p)?;
        let (low, high) = self.bounds(p, p, r)?;
        let r2 = r.square()?;
        let mut cells = Vec::new();
        let (mut minimum, mut maximum) = (i32::MAX, i32::MIN);
        for y in low.y..=high.y {
            for x in low.x..=high.x {
                budget.take()?;
                let cell = Cell { x, y };
                if cell != center && !distance_rect(p, self.rect(cell)?)?.lt(r2)? {
                    continue;
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
    fn interval_support(
        &self,
        surface: &impl Surface,
        a: P,
        delta: P,
        first: u32,
        last: u32,
        radius2: Q,
        profile: Traversal,
        candidates: &[Cell],
        budget: &mut Budget,
    ) -> Result<(), Error> {
        let (lo, hi) = (time(first), time(last));
        let start = self.cell_q(a.at(delta, lo)?)?;
        let end = self.cell_q(a.at(delta, hi)?)?;
        self.transition(surface, start, end, profile, budget)?;
        let (mut minimum, mut maximum) = (i32::MAX, i32::MIN);
        for &cell in candidates {
            budget.take()?;
            let rect = self.rect(cell)?;
            let potential = if radius2.n == 0 {
                // Keep only the owning side when a center travels on a grid edge.
                let constant_outside = (delta.x.n == 0
                    && (a.x.lt(rect.left)? || (rect.right.le(a.x)? && cell.x + 1 != self.columns)))
                    || (delta.y.n == 0
                        && (a.y.lt(rect.bottom)?
                            || (rect.top.le(a.y)? && cell.y + 1 != self.rows)));
                !constant_outside && capsule_touches(a, delta, lo, hi, rect, radius2)?
            } else {
                capsule_touches(a, delta, lo, hi, rect, radius2)?
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
        let mut verified = 0;
        let result = (|| {
            let a = P::point(from)?;
            let b = P::point(to)?;
            let r = Q::float(radius)?;
            self.support_q(surface, a, r, profile.max_step, budget)?;
            self.support_q(surface, b, r, profile.max_step, budget)?;
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
            for pair in events.windows(2) {
                let mut stack = vec![(pair[0], pair[1])];
                while let Some((first, last)) = stack.pop() {
                    budget.take()?;
                    match self.interval_support(
                        surface,
                        a,
                        delta,
                        first,
                        last,
                        r2,
                        profile,
                        &candidates,
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

#[derive(Clone, Copy)]
struct ExactHit {
    t: Q,
    position: [Q; 3],
    cell: Cell,
    face: Face,
    elevation: i32,
}
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
    fn bounded_ray_candidates_match_the_original_full_scan_with_unknowns_and_ties() {
        let terrain = grid();
        for pattern in 0..4 {
            for unknown in [false, true] {
                let surface = |cell: Cell| {
                    if unknown && (cell.x + 2 * cell.y) % 7 == 0 {
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
}
