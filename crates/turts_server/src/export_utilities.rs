//! Protocol 11 owner-only underground utility rows (M6).
//!
//! A viewer gets only the rows of its own slot. A snapshot holds `None` only when the
//! catalog has no utility type. A delta holds `Some` only on a tick where the viewer's own
//! rows changed, so a foreign placement changes neither the bytes nor the size of a delta.

use turts_net_protocol::{self as proto, messages as wire};
use turts_sim::{
    SlotId, WorldView,
    utilities::{
        OwnerEnd, OwnerSegmentState, UtilityOwnerView, owner_view, owner_view_changed_tick,
    },
};

/// The whole owner view, or `None` when the catalog has no utility type.
pub(super) fn snapshot(view: &WorldView, slot: SlotId) -> Option<proto::UtilityView> {
    owner_view_changed_tick(view, slot.0)?;
    Some(convert(owner_view(view, slot.0)))
}

/// The whole owner view when it changed at `tick`, else `None` (no change).
pub(super) fn delta(view: &WorldView, slot: SlotId, tick: u32) -> Option<proto::UtilityView> {
    (owner_view_changed_tick(view, slot.0)? == tick).then(|| convert(owner_view(view, slot.0)))
}

/// A baseline diff: the new view when the two permitted baselines differ.
pub(super) fn changed(
    old: &Option<proto::UtilityView>,
    new: &Option<proto::UtilityView>,
) -> Option<proto::UtilityView> {
    (old != new).then(|| new.clone()).flatten()
}

fn end(end: OwnerEnd) -> proto::UtilityEndRecord {
    match end {
        OwnerEnd::Node(id) => proto::UtilityEndRecord::Node(id),
        OwnerEnd::Port(unit) => proto::UtilityEndRecord::Port(wire::UnitId(unit)),
    }
}

fn convert(view: UtilityOwnerView) -> proto::UtilityView {
    proto::UtilityView {
        nodes: view
            .nodes
            .into_iter()
            .map(|node| proto::UtilityNodeRecord {
                id: node.id,
                utility: node.utility.to_string(),
                pos: wire::Pos {
                    x: node.pos.0,
                    y: node.pos.1,
                },
            })
            .collect(),
        segments: view
            .segments
            .into_iter()
            .map(|segment| proto::UtilitySegmentRecord {
                id: segment.id,
                utility: segment.utility.to_string(),
                a: end(segment.a),
                b: end(segment.b),
                elevation: segment.elevation,
                depth: segment.depth,
                state: match segment.state {
                    OwnerSegmentState::Conducting => proto::UtilitySegmentState::Conducting,
                    OwnerSegmentState::Exposed => proto::UtilitySegmentState::Exposed,
                    OwnerSegmentState::Disconnected => proto::UtilitySegmentState::Disconnected,
                },
            })
            .collect(),
        networks: view
            .networks
            .into_iter()
            .map(|network| proto::UtilityNetworkRecord {
                utility: network.utility.to_string(),
                units: network.units.into_iter().map(wire::UnitId).collect(),
                nodes: network.nodes,
                provided: network.provided,
                consumed: network.consumed,
                short: network.short,
            })
            .collect(),
    }
}
