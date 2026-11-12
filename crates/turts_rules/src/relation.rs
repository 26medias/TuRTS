//! Relations and teams (design 6.6). One function gives the relation; the target filters, the
//! acquisition grids, retaliation, repair targets, the build area and the victory check use it.

/// The five relations of design 6.6, as the mod `SlotRelation` will have them (6.10 row L1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The own slot.
    Same,
    /// A slot of the own team, the own slot included.
    Ally,
    /// A slot of another team.
    Enemy,
    /// Any other slot. It does not mean enemy.
    Other,
    /// Every slot.
    Any,
}

/// The set of relations that hold between two slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Relations(u8);

impl Relations {
    pub const fn has(self, relation: Relation) -> bool {
        self.0 & (1 << relation as u8) != 0
    }
}

/// The team of each slot, indexed by slot number. A slot with no entry is its own team
/// (the `Immediate` start policy, design 6.6).
#[derive(Debug, Clone, Copy)]
pub struct Teams<'a>(&'a [u8]);

impl<'a> Teams<'a> {
    /// Every slot is its own team.
    pub const OWN_TEAM: Teams<'static> = Teams(&[]);

    pub const fn new(team_of_slot: &'a [u8]) -> Self {
        Teams(team_of_slot)
    }

    /// The team of `slot`. A slot with no entry is its own team: its team is its number.
    pub fn team(self, slot: u8) -> u8 {
        self.0.get(usize::from(slot)).copied().unwrap_or(slot)
    }
}

/// The relations that `b` has to `a` (`a` is the own slot).
pub fn relation(a: u8, b: u8, teams: Teams<'_>) -> Relations {
    let same_team = teams.team(a) == teams.team(b);
    let held = [
        (Relation::Same, a == b),
        (Relation::Ally, same_team),
        (Relation::Enemy, !same_team),
        (Relation::Other, a != b),
        (Relation::Any, true),
    ];
    let mut bits = 0u8;
    for (relation, holds) in held {
        if holds {
            bits |= 1 << relation as u8;
        }
    }
    Relations(bits)
}

/// True when `filter` holds between `a` and `b`.
pub fn matches(filter: Relation, a: u8, b: u8, teams: Teams<'_>) -> bool {
    relation(a, b, teams).has(filter)
}
