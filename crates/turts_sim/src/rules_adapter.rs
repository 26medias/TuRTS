//! The one place where the mod schema types become the plain value types of `turts_rules`.
//!
//! The rules crate holds the rules (relation, charge, refund, shortage effect) and has no
//! dependency on `turts_mod_api`, by design, so the conversion is here. A rule is not defined twice:
//! these functions only map a schema value to the rule value and call the rule. Each mapping is an
//! exhaustive `match` (no `_` arm), so a new schema variant is a compile error here and not a
//! silent default. The simulation, the audit and the client call these functions; the client has
//! the one mapping that it needs (`turts_client::relation`) because it cannot depend on this crate.

use turts_mod_api::{CancelRefunds, RefundRatio, ShortEffect, SlotRelation};
use turts_rules::{Ratio, charge, relation, supply};

pub use turts_rules::relation::Teams;

/// The relation of the rules crate that a target filter names (design 6.10 row L1).
pub fn relation_of(filter: SlotRelation) -> relation::Relation {
    match filter {
        SlotRelation::Other => relation::Relation::Other,
        SlotRelation::Same => relation::Relation::Same,
        SlotRelation::Any => relation::Relation::Any,
        SlotRelation::Ally => relation::Relation::Ally,
        SlotRelation::Enemy => relation::Relation::Enemy,
    }
}

/// True when `other` is in the relation `filter` to `own`, with the teams of the match.
pub fn holds(filter: SlotRelation, own: u8, other: u8, teams: Teams<'_>) -> bool {
    relation::matches(relation_of(filter), own, other, teams)
}

/// `holds` with every slot its own team (the `Immediate` start policy and the matches that
/// have no team table yet): a slot is allied with itself only.
pub fn holds_own_team(filter: SlotRelation, own: u8, other: u8) -> bool {
    holds(filter, own, other, Teams::OWN_TEAM)
}

/// The rule ratio of a schema ratio; `None` for a denominator of 0 (the packer refuses it).
pub fn ratio(value: RefundRatio) -> Option<Ratio> {
    Ratio::new(u64::from(value.numerator), u64::from(value.denominator))
}

/// The refunds of a job in the form of `turts_rules::charge`. `queued` is not part of it: a held
/// order or a queued job charged nothing under `ChargeOverTime`, so the rule has no `queued`
/// ratio (`charge::CancelCase::HeldOrQueued` refunds 0). The policies of API 0.2.3 still use
/// `queued` in the simulation. `None` when a denominator is 0.
pub fn cancel_refunds(value: &CancelRefunds) -> Option<charge::CancelRefunds> {
    Some(charge::CancelRefunds {
        running: ratio(value.running)?,
        output_blocked: ratio(value.output_blocked)?,
        destroyed: ratio(value.destroyed)?,
    })
}

/// The shortage effect of the rules crate; `None` for a `Rate` with a denominator of 0.
pub fn when_short(effect: ShortEffect) -> Option<supply::WhenShort> {
    Some(match effect {
        ShortEffect::Rate(value) => supply::WhenShort::Rate(ratio(value)?),
        ShortEffect::Disable => supply::WhenShort::Disable,
        ShortEffect::BlockStart => supply::WhenShort::BlockStart,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_filter_maps_to_its_own_relation_and_holds_as_the_rule_says() {
        use relation::Relation;
        let teams_of_slot = [0, 0, 1];
        let teams = Teams::new(&teams_of_slot);
        for (filter, relation) in [
            (SlotRelation::Other, Relation::Other),
            (SlotRelation::Same, Relation::Same),
            (SlotRelation::Any, Relation::Any),
            (SlotRelation::Ally, Relation::Ally),
            (SlotRelation::Enemy, Relation::Enemy),
        ] {
            assert_eq!(relation_of(filter), relation);
            for own in 0..3 {
                for other in 0..3 {
                    assert_eq!(
                        holds(filter, own, other, teams),
                        relation::matches(relation, own, other, teams),
                        "{filter:?} {own} {other}"
                    );
                }
            }
        }
        // Slots 0 and 1 are one team, slot 2 is the other.
        assert!(holds(SlotRelation::Ally, 0, 1, teams));
        assert!(holds(SlotRelation::Enemy, 0, 2, teams));
        assert!(!holds(SlotRelation::Enemy, 0, 1, teams));
        // With no team table, a slot is allied with itself only.
        assert!(holds_own_team(SlotRelation::Ally, 3, 3));
        assert!(!holds_own_team(SlotRelation::Ally, 3, 4));
        assert!(holds_own_team(SlotRelation::Enemy, 3, 4));
        assert!(holds_own_team(SlotRelation::Other, 3, 4));
        assert!(!holds_own_team(SlotRelation::Same, 3, 4));
    }

    #[test]
    fn a_zero_denominator_has_no_rule_value() {
        let bad = RefundRatio {
            numerator: 1,
            denominator: 0,
        };
        assert_eq!(ratio(bad), None);
        assert_eq!(ratio(RefundRatio::ONE), Some(Ratio::ONE));
        assert_eq!(ratio(RefundRatio::ZERO), Some(Ratio::ZERO));
        assert_eq!(when_short(ShortEffect::Rate(bad)), None);
        let refunds = CancelRefunds {
            queued: RefundRatio::ONE,
            running: RefundRatio::ONE,
            output_blocked: bad,
            destroyed: RefundRatio::ZERO,
        };
        assert_eq!(cancel_refunds(&refunds), None);
    }

    #[test]
    fn refunds_and_effects_keep_their_numbers() {
        let half = RefundRatio {
            numerator: 1,
            denominator: 2,
        };
        let refunds = CancelRefunds {
            queued: RefundRatio::ONE,
            running: half,
            output_blocked: RefundRatio::ZERO,
            destroyed: RefundRatio::ONE,
        };
        let rule = cancel_refunds(&refunds).unwrap();
        assert_eq!(rule.running, Ratio::new(1, 2).unwrap());
        assert_eq!(rule.output_blocked, Ratio::ZERO);
        assert_eq!(rule.destroyed, Ratio::ONE);
        assert_eq!(
            when_short(ShortEffect::Rate(half)),
            Some(supply::WhenShort::Rate(Ratio::new(1, 2).unwrap()))
        );
        assert_eq!(
            when_short(ShortEffect::Disable),
            Some(supply::WhenShort::Disable)
        );
        assert_eq!(
            when_short(ShortEffect::BlockStart),
            Some(supply::WhenShort::BlockStart)
        );
    }
}
