//! Relations and teams, design 6.6, with two slots in each team.

use turts_rules::relation::{Relation, Teams, matches, relation};

const ALL: [Relation; 5] = [
    Relation::Same,
    Relation::Ally,
    Relation::Enemy,
    Relation::Other,
    Relation::Any,
];

/// Slots 0 and 1 are team 0; slots 2 and 3 are team 1.
const TEAM_TABLE: [u8; 4] = [0, 0, 1, 1];

#[test]
fn the_relation_table_with_two_slots_in_each_team() {
    let teams = Teams::new(&TEAM_TABLE);
    for a in 0..4u8 {
        for b in 0..4u8 {
            let same_team = TEAM_TABLE[a as usize] == TEAM_TABLE[b as usize];
            let want = |r: Relation| match r {
                Relation::Same => a == b,
                Relation::Ally => same_team,
                Relation::Enemy => !same_team,
                Relation::Other => a != b,
                Relation::Any => true,
            };
            let got = relation(a, b, teams);
            for r in ALL {
                assert_eq!(got.has(r), want(r), "slots {a} and {b}, relation {r:?}");
                assert_eq!(matches(r, a, b, teams), want(r));
            }
        }
    }
}

#[test]
fn an_ally_of_another_slot_is_other_and_never_enemy() {
    // TD trap B8: `Other` is not "enemy". Slot 1 is an ally of slot 0.
    let teams = Teams::new(&TEAM_TABLE);
    assert!(matches(Relation::Other, 0, 1, teams));
    assert!(matches(Relation::Ally, 0, 1, teams));
    assert!(!matches(Relation::Enemy, 0, 1, teams));
    assert!(!matches(Relation::Same, 0, 1, teams));
}

#[test]
fn the_own_slot_is_same_and_ally_and_any_and_not_other_or_enemy() {
    let teams = Teams::new(&TEAM_TABLE);
    let got = relation(2, 2, teams);
    assert!(got.has(Relation::Same) && got.has(Relation::Ally) && got.has(Relation::Any));
    assert!(!got.has(Relation::Other) && !got.has(Relation::Enemy));
}

#[test]
fn in_mp1_each_slot_is_its_own_team() {
    // Design 6.6: Ally equals Same, and Enemy equals Other. An empty table and a table that
    // gives each slot its own team must agree.
    let own_table = [0u8, 1, 2, 3];
    for teams in [Teams::OWN_TEAM, Teams::new(&own_table)] {
        for a in 0..4u8 {
            for b in 0..4u8 {
                let got = relation(a, b, teams);
                assert_eq!(got.has(Relation::Ally), got.has(Relation::Same));
                assert_eq!(got.has(Relation::Enemy), got.has(Relation::Other));
                assert_eq!(got.has(Relation::Same), a == b);
            }
        }
    }
}

#[test]
fn a_slot_outside_the_table_is_its_own_team() {
    let teams = Teams::new(&TEAM_TABLE);
    assert!(matches(Relation::Enemy, 9, 0, teams));
    assert!(matches(Relation::Ally, 9, 9, teams));
    assert!(matches(Relation::Enemy, 9, 10, teams));
}

#[test]
fn a_missing_slot_is_never_an_ally_by_its_number() {
    // Review finding 1: the slot number was used as a team id for a slot with no entry, and
    // it can equal a real team id. Slots 0 and 1 are team 2; slot 2 has no entry.
    let table = [2u8, 2];
    let teams = Teams::new(&table);
    assert!(matches(Relation::Ally, 0, 1, teams));
    for (a, b) in [(0u8, 2u8), (2, 0), (1, 2), (2, 1)] {
        assert!(!matches(Relation::Ally, a, b, teams), "slots {a} and {b}");
        assert!(matches(Relation::Enemy, a, b, teams), "slots {a} and {b}");
        assert!(matches(Relation::Other, a, b, teams));
    }
    // A missing slot is allied with itself only.
    assert!(matches(Relation::Same, 2, 2, teams) && matches(Relation::Ally, 2, 2, teams));
    assert!(matches(Relation::Enemy, 2, 3, teams));
    // Two missing slots are not allies, even when one number is a team id of the table.
    assert!(!matches(Relation::Ally, 5, 2, teams));
}
