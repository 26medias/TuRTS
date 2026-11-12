//! Integer helpers. `u128` intermediates; no float.

/// `floor(a x b / d)` for `d` above 0. The product of two `u64` values always fits in `u128`.
pub(crate) fn mul_div_u64(a: u64, b: u64, d: u64) -> u128 {
    u128::from(a) * u128::from(b) / u128::from(d)
}

/// The value as `u64`, or `u64::MAX` when it does not fit.
pub(crate) fn saturate_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// `floor(a x b / d)` with a 256-bit product. `None` when `d` is 0 or the result does not fit
/// in `u128`.
pub(crate) fn mul_div_wide(a: u128, b: u128, d: u128) -> Option<u128> {
    if d == 0 {
        return None;
    }
    if let Some(product) = a.checked_mul(b) {
        return Some(product / d);
    }
    let (high, low) = widening_mul(a, b);
    if high >= d {
        return None;
    }
    // Long division of the 256-bit product (high, low) by `d`. `high < d`, so the quotient
    // fits in 128 bits. The remainder can reach 129 bits, so the carry bit is kept.
    let mut remainder = high;
    let mut quotient = 0u128;
    for bit in (0..128).rev() {
        let carry = remainder >> 127;
        remainder = (remainder << 1) | ((low >> bit) & 1);
        if carry == 1 || remainder >= d {
            remainder = remainder.wrapping_sub(d);
            quotient |= 1 << bit;
        }
    }
    Some(quotient)
}

/// The 256-bit product of two `u128` values as (high, low).
fn widening_mul(a: u128, b: u128) -> (u128, u128) {
    const MASK: u128 = u64::MAX as u128;
    let (a_low, a_high) = (a & MASK, a >> 64);
    let (b_low, b_high) = (b & MASK, b >> 64);
    let low_low = a_low * b_low;
    let low_high = a_low * b_high;
    let high_low = a_high * b_low;
    let high_high = a_high * b_high;
    let middle = (low_low >> 64) + (low_high & MASK) + (high_low & MASK);
    let low = (low_low & MASK) | (middle << 64);
    let high = high_high + (low_high >> 64) + (high_low >> 64) + (middle >> 64);
    (high, low)
}

#[cfg(test)]
mod tests {
    use super::{mul_div_u64, mul_div_wide, saturate_u64};

    #[test]
    fn u64_products_use_a_u128_intermediate() {
        assert_eq!(mul_div_u64(100, 29, 100), 29);
        assert_eq!(mul_div_u64(7, 3, 2), 10);
        assert_eq!(
            mul_div_u64(u64::MAX, u64::MAX, u64::MAX),
            u128::from(u64::MAX)
        );
        assert_eq!(
            mul_div_u64(u64::MAX, u64::MAX, 1),
            u128::from(u64::MAX) * u128::from(u64::MAX)
        );
    }

    #[test]
    fn saturate_clamps_at_u64_max() {
        assert_eq!(saturate_u64(5), 5);
        assert_eq!(saturate_u64(u128::from(u64::MAX)), u64::MAX);
        assert_eq!(saturate_u64(u128::from(u64::MAX) + 1), u64::MAX);
        assert_eq!(saturate_u64(u128::MAX), u64::MAX);
    }

    #[test]
    fn the_wide_product_matches_python_integers() {
        // (a, b, d, floor(a x b / d)); the product of each row is above 2^128.
        let rows: [(u128, u128, u128, Option<u128>); 8] = [
            (
                844_920_406_192_130_754_552_550_012_702_180,
                280_141_434_230_972_539_788_080_131_891,
                1_317_398_732_306_098_197_051_068_451_493_981,
                Some(179_670_139_796_887_777_311_968_268_690),
            ),
            (
                33_944_010_507_038_304_785_436_040_057_048,
                2_182_818_849_270_895_973_030_973_675_300,
                2_386_653_476_831_974_312_349_824_488_623,
                Some(31_044_986_913_208_645_026_819_285_839_855),
            ),
            (
                153_748_825_050_930_457_256_821_554_983_736,
                110_656_427_976_595_717_482_209_936_591,
                21_618_075_082_601_348_995_140_870_264_753,
                Some(786_994_018_696_286_970_174_535_811_438),
            ),
            (
                76_418_786_213_113_420_652_391_932_146_242,
                226_634_283_464_470_205_548_567_472_708_823_608,
                25_952_864_616_315_674_708_730_785_684_451,
                Some(667_329_680_660_591_148_549_731_333_467_593_542),
            ),
            (
                57_800_727_260_191_689_818_755_789_320_707,
                1_079_009_105_524_396_900_337_822_888_131,
                32_284_797_048_452_877_030_715_156_662_759_765,
                Some(1_931_791_949_197_579_957_182_132_441),
            ),
            (u128::MAX, u128::MAX, u128::MAX, Some(u128::MAX)),
            (u128::MAX, 3, 2, None),
            (1 << 100, 1 << 100, 1 << 90, Some(1 << 110)),
        ];
        for (a, b, d, want) in rows {
            assert_eq!(mul_div_wide(a, b, d), want, "{a} x {b} / {d}");
        }
    }

    #[test]
    fn the_wide_product_agrees_with_u128_when_it_fits_and_refuses_a_zero_divisor() {
        assert_eq!(mul_div_wide(7, 3, 2), Some(10));
        assert_eq!(mul_div_wide(0, u128::MAX, 5), Some(0));
        assert_eq!(mul_div_wide(1, 1, 0), None);
        assert_eq!(mul_div_wide(u128::MAX, u128::MAX, 0), None);
        let mut state = 0x00DD_BA11_u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            u128::from(state >> 11)
        };
        for _ in 0..20_000 {
            // 53-bit values: the product fits in u128, so the direct form is the oracle.
            let (a, b, d) = (next(), next(), next() | 1);
            assert_eq!(mul_div_wide(a, b, d), Some(a * b / d));
        }
    }
}
