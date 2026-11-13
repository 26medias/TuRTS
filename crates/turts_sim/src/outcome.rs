//! Server-owned defeat and victory from the completed tick's surviving key units.
//!
//! It runs once for each tick, after every removal and after the death path (the last
//! economy set, combat), and before the `Collect` chain of `FixedPostUpdate` (design 6.7.8).

use bevy_ecs::{prelude::*, world::World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use turts_mod_api::ContentId;

use crate::{MatchConfig, Tick, economy::views::WorldView};

/// Declarative victory data. A team survives while it retains a key unit in each group.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRules {
    pub key_groups: Vec<Vec<ContentId>>,
    pub participants: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Victory,
    Defeat,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeReason {
    KeyUnitsLost,
    OpponentsDefeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotOutcome {
    pub outcome: Outcome,
    pub reason: OutcomeReason,
    pub tick: u32,
}

/// The reliable result state. It is written only after the tick's removals.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchOutcomes {
    pub slots: Vec<Option<SlotOutcome>>,
    pub end_tick: Option<u32>,
}

impl Default for MatchOutcomes {
    fn default() -> Self {
        Self {
            slots: vec![None; usize::from(crate::slots::SLOT_COUNT)],
            end_tick: None,
        }
    }
}

fn team(config: &MatchConfig, slot: u8) -> u8 {
    config.teams.get(usize::from(slot)).copied().unwrap_or(slot)
}

fn alive_teams(view: &WorldView, rules: &MatchRules, config: &MatchConfig) -> BTreeSet<u8> {
    rules
        .participants
        .iter()
        .map(|slot| team(config, *slot))
        .filter(|team_id| {
            rules.key_groups.iter().all(|group| {
                view.units.iter().any(|unit| {
                    rules.participants.contains(&unit.owner_slot)
                        && team(config, unit.owner_slot) == *team_id
                        && group.iter().any(|id| id.to_string() == unit.content_id)
                        && !view.construction.sites.contains_key(&unit.unit_id)
                        && view
                            .m3
                            .raw_healths
                            .iter()
                            .find(|health| health.unit == unit.unit_id)
                            .is_none_or(|health| health.current > 0)
                })
            })
        })
        .collect()
}

pub fn match_running(outcomes: Option<Res<MatchOutcomes>>) -> bool {
    outcomes.is_none_or(|outcomes| outcomes.end_tick.is_none())
}

pub fn slot_defeated(world: &World, slot: u8) -> bool {
    world
        .get_resource::<MatchOutcomes>()
        .and_then(|state| state.slots.get(usize::from(slot)))
        .copied()
        .flatten()
        .is_some_and(|result| result.outcome == Outcome::Defeat)
}

pub(crate) fn run_outcome_check(world: &mut World) {
    let Some(rules) = world.get_resource::<MatchRules>().cloned() else {
        return;
    };
    if world
        .get_resource::<MatchOutcomes>()
        .is_some_and(|state| state.end_tick.is_some())
    {
        return;
    }
    let config = world.resource::<MatchConfig>().clone();
    let before = &world.resource::<crate::economy::events::CompletedWorld>().0;
    let alive_before = alive_teams(before, &rules, &config);
    let current = crate::economy::views::world_view(world);
    let alive = alive_teams(&current, &rules, &config);
    let tick = world.resource::<Tick>().0;
    let mut outcomes = world
        .get_resource::<MatchOutcomes>()
        .cloned()
        .unwrap_or_else(|| MatchOutcomes {
            slots: vec![None; usize::from(crate::slots::SLOT_COUNT)],
            end_tick: None,
        });
    for slot in &rules.participants {
        let team = team(&config, *slot);
        let result = if alive.is_empty() && alive_before.contains(&team) {
            Some((Outcome::Draw, OutcomeReason::KeyUnitsLost))
        } else if !alive.contains(&team) {
            Some((Outcome::Defeat, OutcomeReason::KeyUnitsLost))
        } else if alive.len() == 1 {
            Some((Outcome::Victory, OutcomeReason::OpponentsDefeated))
        } else {
            None
        };
        if let Some((outcome, reason)) = result {
            let entry = &mut outcomes.slots[usize::from(*slot)];
            if entry.is_none() {
                *entry = Some(SlotOutcome {
                    outcome,
                    reason,
                    tick,
                });
            }
        }
    }
    if alive.len() <= 1 {
        outcomes.end_tick = Some(tick);
    }
    world.insert_resource(outcomes);
}
