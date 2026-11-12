//! The shortage effect, design 6.2 and row PWR1.

use turts_rules::Ratio;
use turts_rules::supply::{WhenShort, is_short, rate_factor, start_allowed, weapon_enabled};

fn half() -> Ratio {
    Ratio::new(1, 2).expect("denominator above 0")
}

#[test]
fn a_slot_is_short_when_consumed_is_above_provided() {
    assert!(!is_short(100, 100));
    assert!(is_short(100, 101));
    assert!(!is_short(0, 0));
    assert!(is_short(0, 1));
    assert!(!is_short(u64::MAX, 0));
}

#[test]
fn rate_applies_only_while_short() {
    assert_eq!(rate_factor(Some(WhenShort::Rate(half())), true), half());
    assert_eq!(
        rate_factor(Some(WhenShort::Rate(half())), false),
        Ratio::ONE
    );
    // A capability with no `when_short` entry is not affected.
    assert_eq!(rate_factor(None, true), Ratio::ONE);
    // The other effects leave the rate alone.
    assert_eq!(rate_factor(Some(WhenShort::Disable), true), Ratio::ONE);
    assert_eq!(rate_factor(Some(WhenShort::BlockStart), true), Ratio::ONE);
}

#[test]
fn disable_stops_the_weapon_only_while_short() {
    assert!(!weapon_enabled(Some(WhenShort::Disable), true));
    assert!(weapon_enabled(Some(WhenShort::Disable), false));
    assert!(weapon_enabled(None, true));
    assert!(weapon_enabled(Some(WhenShort::Rate(half())), true));
}

#[test]
fn block_start_stops_new_jobs_only_while_short() {
    assert!(!start_allowed(Some(WhenShort::BlockStart), true));
    assert!(start_allowed(Some(WhenShort::BlockStart), false));
    assert!(start_allowed(None, true));
    assert!(start_allowed(Some(WhenShort::Disable), true));
}
