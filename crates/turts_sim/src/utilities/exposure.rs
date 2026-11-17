//! Burial and exposure (spec 11, "Burial and exposure").
//!
//! The pass checks the segments that touch a changed terrain cell, plus the segments that a
//! placement or a split made in this tick. It checks no other segment. The counter
//! `UtilityState::checked_segments` records how many it checked.

use super::network::{LivePorts, is_live};
use super::state::{End, GridGeom, SegmentState, UtilityState};
use std::collections::BTreeSet;
use turts_mod_api::{ContentId, UtilityExposure};

/// What the exposure step reads. `surface` is the current surface elevation of a cell.
pub(crate) struct ExposureInput<'a> {
    pub(crate) geom: Option<GridGeom>,
    pub(crate) changed_cells: &'a BTreeSet<(u32, u32)>,
    pub(crate) surface: &'a dyn Fn(u32, u32) -> Option<i32>,
    pub(crate) policy: &'a dyn Fn(&ContentId) -> Option<UtilityExposure>,
    pub(crate) tick: u32,
}

/// Port removal: a port end whose unit is not a live completed own unit becomes a new
/// junction at the old port point, in ascending segment id order. When the slot has no room
/// for the junctions, the segment is deleted instead.
pub(crate) fn convert_ports(
    state: &mut UtilityState,
    live: &LivePorts,
    geom: Option<GridGeom>,
    tick: u32,
) {
    let slots: Vec<u8> = state.snap.0.slots.keys().copied().collect();
    for slot in slots {
        let dead: Vec<u32> = state.topology(slot).map_or_else(Vec::new, |topology| {
            topology
                .segments
                .iter()
                .filter(|(_, segment)| {
                    [segment.a, segment.b].iter().any(|end| {
                        matches!(end, End::Port(unit) if !is_live(live, *unit, slot, &segment.utility))
                    })
                })
                .map(|(id, _)| *id)
                .collect()
        });
        if dead.is_empty() {
            continue;
        }
        for id in dead {
            let Some(segment) = state
                .topology(slot)
                .and_then(|topology| topology.segments.get(&id))
                .cloned()
            else {
                continue;
            };
            let is_dead = |end: End| matches!(end, End::Port(unit) if !is_live(live, unit, slot, &segment.utility));
            let needed = usize::from(is_dead(segment.a)) + usize::from(is_dead(segment.b));
            let room = turts_mod_api::MAX_UTILITY_JUNCTIONS_PER_SLOT.saturating_sub(
                state
                    .topology(slot)
                    .map_or(0, |topology| topology.nodes.len()),
            );
            if needed > room {
                state.delete_segment(slot, id, geom);
            } else {
                let topology = state.topology_mut(slot);
                let mut ends = [(segment.a, segment.a_pos), (segment.b, segment.b_pos)];
                for (end, pos) in &mut ends {
                    if is_dead(*end) {
                        let node = topology.next_node;
                        topology.next_node += 1;
                        topology.nodes.insert(
                            node,
                            super::state::Node {
                                utility: segment.utility.clone(),
                                pos: *pos,
                            },
                        );
                        *end = End::Node(node);
                    }
                }
                if let Some(stored) = topology.segments.get_mut(&id) {
                    stored.a = ends[0].0;
                    stored.b = ends[1].0;
                }
                state.mark_dirty(slot, &segment.utility);
            }
            state.mark_changed(slot, tick);
        }
    }
}

/// Check the segments under the changed cells and the segments made in this tick, then
/// apply the exposure policy of each. Returns nothing; the state records the effects.
pub(crate) fn run_exposure(state: &mut UtilityState, input: &ExposureInput<'_>) {
    let mut candidates: BTreeSet<(u8, u32)> = std::mem::take(&mut state.pending);
    for (slot, index) in &state.index {
        for cell in input.changed_cells {
            if let Some(segments) = index.get(cell) {
                candidates.extend(segments.iter().map(|segment| (*slot, *segment)));
            }
        }
    }
    let Some(geom) = input.geom else {
        return;
    };
    for (slot, id) in candidates {
        let Some(segment) = state
            .topology(slot)
            .and_then(|topology| topology.segments.get(&id))
            .cloned()
        else {
            continue;
        };
        state.checked_segments += 1;
        let exposed = geom
            .cells_of(segment.a_pos, segment.b_pos)
            .iter()
            .any(|&(x, y)| {
                (input.surface)(x, y).is_some_and(|surface| {
                    turts_rules::utilities::exposed_at(surface, segment.elevation)
                })
            });
        let Some(policy) = (input.policy)(&segment.utility) else {
            continue;
        };
        let next = match (exposed, policy) {
            (false, _) => SegmentState::Conducting,
            (true, UtilityExposure::Operate) => SegmentState::Exposed,
            (true, UtilityExposure::Disconnect) => SegmentState::Disconnected,
            (true, UtilityExposure::Remove) => {
                state.delete_segment(slot, id, Some(geom));
                state.mark_changed(slot, input.tick);
                continue;
            }
        };
        if next != segment.state {
            if let Some(stored) = state.topology_mut(slot).segments.get_mut(&id) {
                stored.state = next;
            }
            // `Operate` changes only the label: the components stay as they are.
            if next.conducts() != segment.state.conducts() {
                state.mark_dirty(slot, &segment.utility);
            }
            state.mark_changed(slot, input.tick);
        }
    }
}
