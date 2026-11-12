//! The public match state of one delta (design 6.7.13): the connection of each slot and the
//! outcome of each slot, sent on change. The sim has no match state yet (`L-MATCH-SIM` adds it),
//! so the writer returns `None`: no change.
//!
//! This file reads only `CommittedTick` fields (the scan `order_writers.rs`). `MatchState`
//! names slots, never units.

use turts_net_protocol::messages as wire;
use turts_sim::CommittedTick;

pub(super) fn match_state(_batch: &CommittedTick) -> Option<wire::MatchState> {
    None
}
