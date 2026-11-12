//! The damage formula, design 6.3 and row DMG1 (D:2445). Each cell is an exact number.

use turts_rules::Ratio;
use turts_rules::damage::{applied, base, hit, hit_for, raw};

mod common;
use common::Rng;

fn ratio(num: u64, den: u64) -> Ratio {
    Ratio::new(num, den).expect("denominator above 0")
}

#[test]
fn the_dmg1_cells_are_exact() {
    // (base, num, den, minimum, flat, hit)
    let rows: [(u64, u64, u64, u64, u64, u64); 12] = [
        // A float multiplier gives 28 here (DMG1 plant 1, f64).
        (100, 29, 100, 0, 0, 29),
        // A float multiplier in f32 gives 52 here (DMG1 plant 1, f32).
        (50, 106, 100, 0, 0, 53),
        // `flat` taken before the ratio gives 27 here (DMG1 plant 3).
        (100, 29, 100, 1, 5, 24),
        // The 2 x 2 table whose transpose differs (DMG1).
        (100, 100, 100, 1, 0, 100), // (bullet, light)
        (100, 10, 100, 1, 0, 10),   // (bullet, heavy)
        (100, 25, 100, 1, 0, 25),   // (rocket, light)
        (100, 150, 100, 1, 0, 150), // (rocket, heavy)
        // The minimum holds when the ratio gives 0, and when the flat is above the raw value.
        (1, 1, 100, 1, 0, 1),
        (10, 50, 100, 2, 9, 2),
        // The GD form `max(1, floor(d x percent / 100))` is the case flat 0, minimum 1.
        (7, 15, 100, 1, 0, 1),
        // Floor, not rounding: 5 x 3 / 2 is 7.5.
        (5, 3, 2, 0, 0, 7),
        // A ratio with a huge denominator.
        (u64::MAX, 1, u64::MAX, 0, 0, 1),
    ];
    for (base, num, den, minimum, flat, want) in rows {
        let got = hit(base, ratio(num, den), minimum, flat);
        assert_eq!(
            got, want,
            "hit(base {base}, {num}/{den}, minimum {minimum}, flat {flat})"
        );
    }
}

#[test]
fn the_float_forms_are_wrong_on_the_dmg1_cells() {
    // Instrument check: the two cells are chosen because the float forms miss them. When
    // this test goes red, the cells no longer separate the integer form from a float form.
    assert_eq!((100.0f64 * (29.0f64 / 100.0f64)).floor() as u64, 28);
    assert_eq!((50.0f32 * (106.0f32 / 100.0f32)).floor() as u64, 52);
}

#[test]
fn the_table_axes_do_not_swap() {
    // (bullet, heavy) is 10/100 and (rocket, light) is 25/100: the transpose gives 25 and 10.
    assert_eq!(hit(100, ratio(10, 100), 1, 0), 10);
    assert_eq!(hit(100, ratio(25, 100), 1, 0), 25);
}

#[test]
fn base_is_a_checked_add() {
    assert_eq!(base(3, 4), Some(7));
    assert_eq!(base(u64::MAX, 0), Some(u64::MAX));
    assert_eq!(base(u64::MAX, 1), None);
}

#[test]
fn raw_saturates_and_applied_never_passes_health() {
    assert_eq!(raw(u64::MAX, ratio(2, 1)), u64::MAX);
    assert_eq!(raw(u64::MAX, ratio(u64::MAX, u64::MAX)), u64::MAX);
    assert_eq!(applied(10, 4), 4);
    assert_eq!(applied(4, 10), 4);
    assert_eq!(applied(5, 0), 0);
}

#[test]
fn a_package_with_no_table_keeps_the_base() {
    // 6.3: no damage table gives `hit = base`. That is the cell 1/1 with minimum 0, flat 0.
    for base in [0, 1, 7, u64::MAX] {
        assert_eq!(hit(base, Ratio::ONE, 0, 0), base);
    }
}

/// An independent form of `floor(base x num / den)`: split `base` by `den`, so the product
/// is not the one that the function computes.
fn raw_oracle(base: u64, num: u64, den: u64) -> u128 {
    let (base, num, den) = (u128::from(base), u128::from(num), u128::from(den));
    (base / den) * num + (base % den) * num / den
}

#[test]
fn the_hit_equals_the_design_formula_for_random_inputs() {
    let mut rng = Rng::new(0x00D1_5EA5_ED01);
    for _ in 0..50_000 {
        let base = rng.wide();
        let num = rng.wide();
        let den = rng.wide().max(1);
        let minimum = rng.wide();
        let flat = rng.wide();
        let health = rng.wide();
        let cell = ratio(num, den);
        let raw_wide = raw_oracle(base, num, den);
        // The flat armour comes off the wide value; only the result saturates (review finding 5).
        let after_flat = raw_wide.saturating_sub(u128::from(flat));
        let want = minimum.max(u64::try_from(after_flat).unwrap_or(u64::MAX));
        let got = hit(base, cell, minimum, flat);
        assert_eq!(
            got, want,
            "base {base} {num}/{den} min {minimum} flat {flat}"
        );
        assert!(got >= minimum, "the minimum is a floor");
        let shot = applied(got, health);
        assert_eq!(shot, got.min(health));
        assert!(
            shot <= health && shot <= got,
            "damage is never above health or hit"
        );
    }
}

#[test]
fn flat_armour_is_taken_off_the_wide_raw_value() {
    // Review finding 5: `raw` saturated before `flat` was taken off, so the hit was
    // u64::MAX - 46 where the exact value 18,502,915,224,908,506,354 is above u64::MAX.
    let got = hit(4_625_728_806_227_126_600, ratio(12, 3), 1, 46);
    assert_eq!(got, u64::MAX);
    assert_eq!(applied(got, u64::MAX), u64::MAX);
    // A flat that brings the wide value back into u64: 4 x u64::MAX / 1 - flat is still above.
    assert_eq!(hit(u64::MAX, ratio(2, 1), 0, u64::MAX), u64::MAX);
    // 2 x (2^63 + 5) = 2^64 + 10: minus 11 is 2^64 - 1 = u64::MAX, exactly.
    assert_eq!(hit((1 << 63) + 5, ratio(2, 1), 0, 11), u64::MAX);
    assert_eq!(hit((1 << 63) + 5, ratio(2, 1), 0, 12), u64::MAX - 1);
}

#[test]
fn a_package_with_no_damage_table_keeps_the_base_whatever_the_armour() {
    // Review finding 8 (design 6.3): no table gives `hit = base`.
    for flat in [0, 5, u64::MAX] {
        assert_eq!(hit_for(100, None, flat), 100);
    }
    assert_eq!(hit_for(u64::MAX, None, 7), u64::MAX);
    // With a table, hit_for is the formula.
    assert_eq!(hit_for(100, Some((ratio(29, 100), 1)), 5), 24);
    assert_eq!(hit_for(50, Some((ratio(106, 100), 0)), 0), 53);
    assert_eq!(hit_for(1, Some((ratio(1, 100), 1)), 0), 1);
}
