//! The world facts that the owner tasks give to the interaction core (`WorldView::placement_facts`
//! and `hint_facts`). One file for each owner, called once from `OwnerFacts::build` (L0-CORE).
//!
//! The facts are kept in `CoreRuntime` from frame to frame. `build` receives the facts of the last
//! frame and updates them in place, so an owner keeps its cache (for example the replica tick of
//! its last rebuild) inside its own `Facts` type and rebuilds once for each applied delta, not for
//! each frame (plan, `UI-PLACE` and `UI-HINT`). No owner edits this file or `frame_world`.

use crate::{content::ClientContent, replica::Replica};

pub(super) mod hint;
pub(super) mod placement;

/// The facts of every owner task, in one value that `CoreRuntime` keeps.
#[derive(Debug, Default)]
pub(super) struct OwnerFacts {
    pub(super) placement: turts_interaction::placement::Facts,
    pub(super) hints: turts_interaction::hints::Facts,
    /// The number of frames that built these facts. Tests only: it shows that the same value
    /// reaches `build` in each frame.
    #[cfg(test)]
    pub(super) builds: u32,
}

impl OwnerFacts {
    /// Updates the facts of each owner in place. Called once for each frame by `frame_world`.
    pub(super) fn build(&mut self, replica: &Replica, content: &ClientContent) {
        placement::build(&mut self.placement, replica, content);
        hint::build(&mut self.hints, replica, content);
        #[cfg(test)]
        {
            self.builds += 1;
        }
    }
}
