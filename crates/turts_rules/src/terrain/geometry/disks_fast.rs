//! Exact dyadic pair predicates without repeated rational normalization.
//!
//! All inputs share a power-of-two denominator with at most 31 bits. Products
//! reach degree four, so their common denominator fits positive i128 (2^124).
//! Every numerator operation is checked in the common scale. A successful
//! calculation therefore also fits the original reduced-rational operations:
//! reduction can only remove powers of two from these common numerators and
//! denominators. Subtractions also retain the original checked-negation
//! intermediate: a fitting lhs-rhs alone does not prove that negating rhs fits.
//! A common-scale rhs of i128::MIN conservatively falls back even when reduction
//! would make its original rational numerator negatable. On any conversion or
//! intermediate overflow, run that original
//! implementation instead. No wider arithmetic domain is accepted here.
use super::Point;

#[derive(Clone, Copy)]
struct Dyadic {
    numerator: i128,
    bits: u32,
}
fn dyadic(value: f64) -> Option<Dyadic> {
    if !value.is_finite() {
        return None;
    }
    let raw = value.to_bits();
    let biased = ((raw >> 52) & 0x7ff) as i32;
    let mut mantissa = raw & ((1u64 << 52) - 1);
    let exponent = if biased == 0 {
        -1074
    } else {
        mantissa |= 1u64 << 52;
        biased - 1023 - 52
    };
    if mantissa == 0 {
        return Some(Dyadic {
            numerator: 0,
            bits: 0,
        });
    }
    let zeros = mantissa.trailing_zeros();
    let power = exponent + zeros as i32;
    let mut numerator = i128::from(mantissa >> zeros);
    let bits = if power >= 0 {
        if power > 126 {
            return None;
        }
        numerator = numerator.checked_mul(1i128 << power)?;
        0
    } else {
        let bits = (-power) as u32;
        if bits > 31 {
            return None;
        }
        bits
    };
    if raw >> 63 != 0 {
        numerator = numerator.checked_neg()?;
    }
    Some(Dyadic { numerator, bits })
}
/// Mirror Q::sub's checked negation before addition. Rejecting a common-scale
/// MIN rhs can be conservative after reduction; the original fallback decides.
fn subtract(lhs: i128, rhs: i128) -> Option<i128> {
    lhs.checked_add(rhs.checked_neg()?)
}
struct Relative {
    x: i128,
    y: i128,
    vx: i128,
    vy: i128,
    radius2: i128,
}
fn relative(a: Point, to: Point, ar: f64, b: Point, bt: Point, br: f64) -> Option<Relative> {
    if ar <= 0. || br <= 0. {
        return None;
    }
    let values = [a.x, a.y, to.x, to.y, b.x, b.y, bt.x, bt.y, ar, br];
    let mut parts = [Dyadic {
        numerator: 0,
        bits: 0,
    }; 10];
    let mut bits = 0;
    for (part, value) in parts.iter_mut().zip(values) {
        *part = dyadic(value)?;
        bits = bits.max(part.bits);
    }
    let mut n = [0i128; 10];
    for (out, part) in n.iter_mut().zip(parts) {
        *out = part.numerator.checked_mul(1i128 << (bits - part.bits))?;
    }
    let x = subtract(n[0], n[4])?;
    let y = subtract(n[1], n[5])?;
    let vx = subtract(subtract(n[2], n[0])?, subtract(n[6], n[4])?)?;
    let vy = subtract(subtract(n[3], n[1])?, subtract(n[7], n[5])?)?;
    let radius = n[8].checked_add(n[9])?;
    Some(Relative {
        x,
        y,
        vx,
        vy,
        radius2: radius.checked_mul(radius)?,
    })
}
fn dot(x: i128, y: i128, vx: i128, vy: i128) -> Option<i128> {
    x.checked_mul(vx)?.checked_add(y.checked_mul(vy)?)
}
pub(super) fn clear(a: Point, to: Point, ar: f64, b: Point, bt: Point, br: f64) -> Option<bool> {
    let Relative {
        x,
        y,
        vx,
        vy,
        radius2,
    } = relative(a, to, ar, b, bt, br)?;
    if dot(x, y, x, y)? < radius2 {
        return Some(false);
    }
    let velocity2 = dot(vx, vy, vx, vy)?;
    let projection = dot(x, y, vx, vy)?;
    if velocity2 == 0 || projection >= 0 {
        return Some(true);
    }
    if projection <= velocity2.checked_neg()? {
        let (ex, ey) = (x.checked_add(vx)?, y.checked_add(vy)?);
        return Some(radius2 <= dot(ex, ey, ex, ey)?);
    }
    let cross = subtract(x.checked_mul(vy)?, y.checked_mul(vx)?)?;
    Some(radius2.checked_mul(velocity2)? <= cross.checked_mul(cross)?)
}
pub(super) fn non_deepening(
    a: Point,
    to: Point,
    ar: f64,
    b: Point,
    bt: Point,
    br: f64,
) -> Option<bool> {
    let Relative {
        x,
        y,
        vx,
        vy,
        radius2,
    } = relative(a, to, ar, b, bt, br)?;
    Some(dot(x, y, x, y)? < radius2 && dot(x, y, vx, vy)? >= 0)
}

#[cfg(test)]
mod tests {
    use super::super::{
        Budget, Error, swept_disks_clear, swept_disks_clear_reference,
        swept_disks_non_deepening_overlap, swept_disks_non_deepening_reference,
    };
    use super::*;
    #[test]
    fn dyadic_pairs_match_rational_results_errors_and_visit_charges() {
        let mut seed = 0x6b37_810a_39f2_918du64;
        let mut random = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut hits = 0;
        for i in 0..12000 {
            let a = Point {
                x: f64::from((random() % 12800) as f32 / 100.),
                y: f64::from((random() % 12800) as f32 / 100.),
            };
            let shift = |v: u64| (v % 1200) as f32 / 100. - 6.;
            let b = Point {
                x: f64::from(a.x as f32 + shift(random())),
                y: f64::from(a.y as f32 + shift(random())),
            };
            let to = Point {
                x: f64::from(a.x as f32 + shift(random())),
                y: f64::from(a.y as f32 + shift(random())),
            };
            let bt = Point {
                x: f64::from(b.x as f32 + shift(random())),
                y: f64::from(b.y as f32 + shift(random())),
            };
            let radii = [0.2, 0.3, 0.5, 1., 0.05];
            let ar = f64::from(radii[i % 5] as f32);
            let br = f64::from(radii[(i / 5) % 5] as f32);
            hits += usize::from(clear(a, to, ar, b, bt, br).is_some());
            for credit in [0, 1, 2] {
                let mut old = Budget::new(credit);
                let mut new = old;
                assert_eq!(
                    swept_disks_clear(a, to, ar, b, bt, br, &mut new),
                    swept_disks_clear_reference(a, to, ar, b, bt, br, &mut old),
                    "case {i}"
                );
                assert_eq!(new, old);
                let mut old = Budget::new(credit);
                let mut new = old;
                assert_eq!(
                    swept_disks_non_deepening_overlap(a, to, ar, b, bt, br, &mut new),
                    swept_disks_non_deepening_reference(a, to, ar, b, bt, br, &mut old),
                    "recovery {i}"
                );
                assert_eq!(new, old);
            }
        }
        assert!(hits > 10000, "fast admission was not exercised: {hits}");
    }
    #[test]
    fn checked_subtraction_keeps_rational_negation_boundary() {
        use super::super::Q;
        for (lhs, rhs) in [
            (i128::MIN, i128::MIN),
            (i128::MIN + 1, i128::MIN),
            (0, i128::MIN),
            (i128::MIN, 0),
            (i128::MIN, -1),
            (i128::MAX, -1),
            (-(1i128 << 126), 1i128 << 126),
            (i128::MIN, -(1i128 << 126)),
        ] {
            let reference = Q::new(lhs, 1)
                .unwrap()
                .sub(Q::new(rhs, 1).unwrap())
                .ok()
                .map(|q| q.n);
            assert_eq!(subtract(lhs, rhs), reference, "lhs={lhs}, rhs={rhs}");
        }
    }
    #[test]
    fn shared_extreme_motion_preserves_the_original_negation_refusal() {
        let huge = 2f64.powi(126);
        for vertical in [false, true] {
            let p = |v| {
                if vertical {
                    Point { x: 0., y: v }
                } else {
                    Point { x: v, y: 0. }
                }
            };
            for credit in [0, 1, 2] {
                let mut old_clear = Budget::new(credit);
                let mut new_clear = old_clear;
                let mut old_recovery = Budget::new(credit);
                let mut new_recovery = old_recovery;
                let reference = (
                    swept_disks_clear_reference(
                        p(huge),
                        p(-huge),
                        1.,
                        p(huge),
                        p(-huge),
                        1.,
                        &mut old_clear,
                    ),
                    swept_disks_non_deepening_reference(
                        p(huge),
                        p(-huge),
                        1.,
                        p(huge),
                        p(-huge),
                        1.,
                        &mut old_recovery,
                    ),
                );
                let actual = (
                    swept_disks_clear(p(huge), p(-huge), 1., p(huge), p(-huge), 1., &mut new_clear),
                    swept_disks_non_deepening_overlap(
                        p(huge),
                        p(-huge),
                        1.,
                        p(huge),
                        p(-huge),
                        1.,
                        &mut new_recovery,
                    ),
                );
                assert_eq!((new_clear, new_recovery), (old_clear, old_recovery));
                if credit > 0 {
                    assert_eq!(reference, (Err(Error::Arithmetic), Err(Error::Arithmetic)));
                }
                assert_eq!(
                    actual, reference,
                    "shared extreme displacement, vertical={vertical}, credit={credit}"
                );
            }
        }
    }
    #[test]
    fn common_scale_minimum_fallback_keeps_a_legal_reduced_result() {
        let start = Point { x: 0., y: 0. };
        let end = Point {
            x: -2f64.powi(126),
            y: 0.,
        };
        for credit in [0, 1, 2] {
            let mut old = Budget::new(credit);
            let mut new = old;
            assert_eq!(
                swept_disks_clear(start, end, 0.5, start, end, 0.5, &mut new),
                swept_disks_clear_reference(start, end, 0.5, start, end, 0.5, &mut old)
            );
            assert_eq!(new, old);
            let mut old = Budget::new(credit);
            let mut new = old;
            assert_eq!(
                swept_disks_non_deepening_overlap(start, end, 0.5, start, end, 0.5, &mut new),
                swept_disks_non_deepening_reference(start, end, 0.5, start, end, 0.5, &mut old)
            );
            assert_eq!(new, old);
        }
    }
    #[test]
    fn dyadic_pairs_keep_tangency_and_fall_back_for_extreme_domains() {
        let p = |x, y| Point { x, y };
        assert_eq!(
            clear(p(-2., 1.), p(2., 1.), 0.5, p(0., 0.), p(0., 0.), 0.5),
            Some(true)
        );
        for value in [
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::INFINITY,
            f64::NAN,
            -f64::MAX,
            2f64.powi(-32),
            2f64.powi(100),
        ] {
            let a = p(value, 0.);
            let b = p(0., 1.);
            for credit in [0, 1] {
                let mut old = Budget::new(credit);
                let mut new = old;
                assert_eq!(
                    swept_disks_clear(a, b, 0.5, b, b, 0.5, &mut new),
                    swept_disks_clear_reference(a, b, 0.5, b, b, 0.5, &mut old)
                );
                assert_eq!(new, old);
            }
        }
        for radius in [-1., 0., f64::NAN, f64::INFINITY] {
            let mut old = Budget::new(1);
            let mut new = old;
            assert_eq!(
                swept_disks_clear(
                    p(0., 0.),
                    p(1., 0.),
                    radius,
                    p(2., 0.),
                    p(2., 0.),
                    0.5,
                    &mut new
                ),
                swept_disks_clear_reference(
                    p(0., 0.),
                    p(1., 0.),
                    radius,
                    p(2., 0.),
                    p(2., 0.),
                    0.5,
                    &mut old
                )
            );
            assert_eq!(new, old);
        }
    }
}
