//! Stage the owner utility view (M6) with the state transaction.
//!
//! The view holds only the rows of this client's own slot (protocol 11). The replica stages it
//! next to the unit tables and commits it in the same infallible step; a bad view refuses the
//! whole transaction. The client invents no row: it keeps what the server sent.
//!
//! Not checked here: that each `Port(unit)` names a live own unit. The view and the unit table
//! ride one transaction, but the server rule that keeps them equal is not part of this client;
//! an unmatched port is skipped at draw time instead of refusing the transaction.

use crate::replica::ReplicaContext;
use std::collections::{HashMap, HashSet};
use turts_mod_api::{
    ContentId, MAX_UTILITY_DEPTH, MAX_UTILITY_JUNCTIONS_PER_SLOT, MAX_UTILITY_SEGMENTS_PER_SLOT,
};
use turts_net_protocol::{UtilityEndRecord, UtilityView};

/// The most network rows and port units that one view can hold. A port unit has one row.
const MAX_NETWORK_ROWS: usize = 8_192;
const MAX_NETWORK_UNITS: usize = 65_536;

fn known(context: &ReplicaContext, utility: &str) -> bool {
    ContentId::parse(utility)
        .ok()
        .is_some_and(|id| context.content.catalog.utilities.contains_key(&id))
}

/// Whether the catalog of this match has any utility type.
pub fn catalog_has_utilities(context: &ReplicaContext) -> bool {
    !context.content.catalog.utilities.is_empty()
}

/// A snapshot holds `Some` view exactly when the catalog has a utility type (spec "Privacy and
/// wire"). `None` is the outer failure: the snapshot is refused.
pub fn stage_snapshot(
    context: &ReplicaContext,
    view: Option<&UtilityView>,
) -> Option<Option<UtilityView>> {
    match (catalog_has_utilities(context), view) {
        (false, None) => Some(None),
        (true, Some(view)) => valid(context, view).then(|| Some(view.clone())),
        _ => None,
    }
}

/// A delta with `Some` replaces the whole view. `None` is no change.
pub fn stage_delta(
    context: &ReplicaContext,
    current: Option<UtilityView>,
    delta: Option<&UtilityView>,
) -> Option<Option<UtilityView>> {
    match delta {
        None => Some(current),
        Some(view) => stage_replacement(context, view).map(Some),
    }
}

/// The replacement view of a delta.
pub fn stage_replacement(context: &ReplicaContext, view: &UtilityView) -> Option<UtilityView> {
    (catalog_has_utilities(context) && valid(context, view)).then(|| view.clone())
}

fn finite(x: f32, y: f32) -> bool {
    x.is_finite() && y.is_finite()
}

fn valid(context: &ReplicaContext, view: &UtilityView) -> bool {
    if view.nodes.len() > MAX_UTILITY_JUNCTIONS_PER_SLOT
        || view.segments.len() > MAX_UTILITY_SEGMENTS_PER_SLOT
        || view.networks.len() > MAX_NETWORK_ROWS
        || view
            .networks
            .iter()
            .map(|network| network.units.len() + network.nodes.len())
            .sum::<usize>()
            > MAX_NETWORK_UNITS
    {
        return false;
    }
    let mut nodes = HashMap::with_capacity(view.nodes.len());
    for node in &view.nodes {
        if !known(context, &node.utility)
            || !finite(node.pos.x, node.pos.y)
            || nodes.insert(node.id, node.utility.as_str()).is_some()
        {
            return false;
        }
    }
    let mut segments = HashSet::with_capacity(view.segments.len());
    for segment in &view.segments {
        if !known(context, &segment.utility)
            || !segments.insert(segment.id)
            || segment.a == segment.b
            || segment.depth == 0
            || segment.depth > MAX_UTILITY_DEPTH
        {
            return false;
        }
        for end in [segment.a, segment.b] {
            if let UtilityEndRecord::Node(id) = end
                && nodes.get(&id) != Some(&segment.utility.as_str())
            {
                return false;
            }
        }
    }
    view.networks.iter().all(|network| {
        known(context, &network.utility)
            && network
                .nodes
                .iter()
                .all(|id| nodes.get(id) == Some(&network.utility.as_str()))
    })
}
