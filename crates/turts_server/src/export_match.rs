//! The public match state of one delta (design 6.7.13): the connection of each slot and the
//! outcome of each slot, sent on change. The sim has no match state yet (`L-MATCH-SIM` adds it),
//! so the writer returns `None`: no change.
//!
//! This file reads only `CommittedTick` fields (the scan `order_writers.rs`). `MatchState`
//! names slots, never units.

use turts_net_protocol::messages as wire;
use turts_sim::CommittedTick;

pub(super) fn match_state(batch: &CommittedTick) -> Option<wire::MatchState> {
    batch.match_outcomes.as_ref().map(state)
}

pub(super) fn state(outcomes: &turts_sim::outcome::MatchOutcomes) -> wire::MatchState {
    use turts_sim::outcome::{Outcome, OutcomeReason};
    wire::MatchState {
        connections: Vec::new(),
        outcomes: outcomes
            .slots
            .iter()
            .map(|result| {
                result.map(|result| wire::SlotOutcome {
                    outcome: match result.outcome {
                        Outcome::Victory => wire::Outcome::Victory,
                        Outcome::Defeat => wire::Outcome::Defeat,
                        Outcome::Draw => wire::Outcome::Draw,
                    },
                    reason: match result.reason {
                        OutcomeReason::KeyUnitsLost => wire::OutcomeReason::KeyUnitsLost,
                        OutcomeReason::OpponentsDefeated => wire::OutcomeReason::OpponentsDefeated,
                    },
                    tick: result.tick,
                })
            })
            .collect(),
    }
}
