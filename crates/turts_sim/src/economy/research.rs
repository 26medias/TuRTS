//! Per-slot completed research. Progress behavior is added in Task 10.

use std::collections::BTreeSet;

use bevy_ecs::prelude::Resource;
use turts_mod_api::ContentId;

/// Completed technologies indexed by owning slot.
#[allow(dead_code)] // used from Task 10
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResearchState {
    pub(crate) completed: BTreeSet<(u8, ContentId)>,
}

/// Unit ids whose toggleable capabilities are disabled.
#[allow(dead_code)] // used from Task 12
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Disabled(pub(crate) BTreeSet<u32>);
