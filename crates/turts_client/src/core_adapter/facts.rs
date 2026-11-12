//! The world facts that the owner tasks give to the interaction core (`WorldView::placement_facts`
//! and `hint_facts`). One file for each owner, called once from `frame_world` (L0-CORE).

pub(super) mod hint;
pub(super) mod placement;
