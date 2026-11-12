//! The relation of a target filter between two slots, for the client (design 6.6, 6.10 row L1).
//!
//! The rule is `turts_rules::relation`; this file maps the mod schema filter to it. The client
//! cannot use the mapping of `turts_sim::rules_adapter` (the client does not depend on the
//! simulation), so the one mapping that the client needs is here. It is an exhaustive `match`:
//! a new `SlotRelation` is a compile error here.

use turts_mod_api::SlotRelation;
use turts_rules::relation::{self, Relation, Teams};

fn relation_of(filter: SlotRelation) -> Relation {
    match filter {
        SlotRelation::Other => Relation::Other,
        SlotRelation::Same => Relation::Same,
        SlotRelation::Any => Relation::Any,
        SlotRelation::Ally => Relation::Ally,
        SlotRelation::Enemy => Relation::Enemy,
    }
}

/// True when `other` is in the relation `filter` to `own`. A slot is its own team until the match
/// config gives the client a team table (`L-MATCH-SIM`, `L-ROOM0`).
pub(crate) fn holds_own_team(filter: SlotRelation, own: u8, other: u8) -> bool {
    relation::matches(relation_of(filter), own, other, Teams::OWN_TEAM)
}

#[cfg(test)]
mod tests {
    use super::holds_own_team;
    use turts_mod_api::SlotRelation::{Ally, Any, Enemy, Other, Same};

    #[test]
    fn the_five_filters_between_two_slots_of_their_own_teams() {
        // (filter, own = other, own != other)
        for (filter, same_slot, two_slots) in [
            (Other, false, true),
            (Same, true, false),
            (Any, true, true),
            (Ally, true, false),
            (Enemy, false, true),
        ] {
            assert_eq!(holds_own_team(filter, 2, 2), same_slot, "{filter:?} same");
            assert_eq!(holds_own_team(filter, 2, 5), two_slots, "{filter:?} other");
        }
    }
}
