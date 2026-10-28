//! Extractor binding state. The phase is added in Task 6.

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;

use crate::{economy::gather::RateCarry, ids::NodeId};

/// Current extractor condition.
#[allow(dead_code)] // used from Task 6
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ExtractorStatus {
    #[default]
    Extracting,
    Full,
    Depleted,
}

/// A drill's bound node, rate remainder, and current condition.
#[allow(dead_code)] // used from Task 6
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExtractorState {
    pub(crate) node: Option<NodeId>,
    pub(crate) carry: RateCarry,
    pub(crate) status: ExtractorStatus,
}

/// Extractor states indexed by unit id.
#[allow(dead_code)] // used from Task 6
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Extractors(pub(crate) BTreeMap<u32, ExtractorState>);
