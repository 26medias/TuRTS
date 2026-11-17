//! Protocol 11 owner-only underground utility view (M6).
//!
//! A viewer receives only its own slot's junctions, segments, networks and port units.
//! No record carries a surface or cover value, a foreign row, a foreign count or a foreign
//! id. The view rides the existing atomic state transaction next to terrain rows.
use serde::{Deserialize, Serialize};

use crate::{ContentRef, Pos, UnitId};

/// One end of an own segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UtilityEndRecord {
    /// An own junction id (counted per slot).
    Node(u32),
    /// The port of an own completed unit.
    Port(UnitId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UtilitySegmentState {
    Conducting,
    /// Exposed under policy `Operate`: still conducts.
    Exposed,
    /// Exposed under policy `Disconnect`: does not conduct.
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityNodeRecord {
    pub id: u32,
    pub utility: ContentRef,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilitySegmentRecord {
    pub id: u32,
    pub utility: ContentRef,
    pub a: UtilityEndRecord,
    pub b: UtilityEndRecord,
    /// Fixed elevation in terrain elevation steps above the datum.
    pub elevation: i32,
    /// The depth that was requested at placement.
    pub depth: u32,
    pub state: UtilitySegmentState,
}

/// One connected network of one utility type. A port unit with no conducting segment is a
/// network of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityNetworkRecord {
    pub utility: ContentRef,
    /// Own completed port units in this network, ascending.
    pub units: Vec<UnitId>,
    /// Own junctions in this network, ascending.
    pub nodes: Vec<u32>,
    pub provided: u64,
    pub consumed: u64,
    pub short: bool,
}

/// The complete owner view. In a delta, `Some` replaces the whole view.
///
/// On the wire, each row names its utility type by an index into one sorted dictionary of
/// the view (`WireView`), so a long content id costs its bytes once, not once per row.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(into = "WireView", try_from = "WireView")]
pub struct UtilityView {
    pub nodes: Vec<UtilityNodeRecord>,
    pub segments: Vec<UtilitySegmentRecord>,
    pub networks: Vec<UtilityNetworkRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireView {
    utilities: Vec<ContentRef>,
    nodes: Vec<(u32, u16, Pos)>,
    segments: Vec<(
        u32,
        u16,
        UtilityEndRecord,
        UtilityEndRecord,
        i32,
        u32,
        UtilitySegmentState,
    )>,
    networks: Vec<(u16, Vec<UnitId>, Vec<u32>, u64, u64, bool)>,
}

impl From<UtilityView> for WireView {
    fn from(view: UtilityView) -> Self {
        let mut utilities: Vec<ContentRef> = view
            .nodes
            .iter()
            .map(|row| &row.utility)
            .chain(view.segments.iter().map(|row| &row.utility))
            .chain(view.networks.iter().map(|row| &row.utility))
            .cloned()
            .collect();
        utilities.sort();
        utilities.dedup();
        // A view holds the utility types of one package catalog, far below u16::MAX.
        let index = |utility: &ContentRef| {
            u16::try_from(utilities.binary_search(utility).unwrap_or(0)).unwrap_or(u16::MAX)
        };
        let nodes = view
            .nodes
            .iter()
            .map(|row| (row.id, index(&row.utility), row.pos))
            .collect();
        let segments = view
            .segments
            .iter()
            .map(|row| {
                (
                    row.id,
                    index(&row.utility),
                    row.a,
                    row.b,
                    row.elevation,
                    row.depth,
                    row.state,
                )
            })
            .collect();
        let networks = view
            .networks
            .into_iter()
            .map(|row| {
                (
                    index(&row.utility),
                    row.units,
                    row.nodes,
                    row.provided,
                    row.consumed,
                    row.short,
                )
            })
            .collect();
        Self {
            utilities,
            nodes,
            segments,
            networks,
        }
    }
}

impl TryFrom<WireView> for UtilityView {
    type Error = String;

    fn try_from(wire: WireView) -> Result<Self, Self::Error> {
        let name = |index: u16| {
            wire.utilities
                .get(usize::from(index))
                .cloned()
                .ok_or_else(|| format!("utility index {index} is not in the view dictionary"))
        };
        Ok(Self {
            nodes: wire
                .nodes
                .iter()
                .map(|&(id, utility, pos)| {
                    Ok(UtilityNodeRecord {
                        id,
                        utility: name(utility)?,
                        pos,
                    })
                })
                .collect::<Result<_, String>>()?,
            segments: wire
                .segments
                .iter()
                .map(|&(id, utility, a, b, elevation, depth, state)| {
                    Ok(UtilitySegmentRecord {
                        id,
                        utility: name(utility)?,
                        a,
                        b,
                        elevation,
                        depth,
                        state,
                    })
                })
                .collect::<Result<_, String>>()?,
            networks: wire
                .networks
                .iter()
                .map(|(utility, units, nodes, provided, consumed, short)| {
                    Ok(UtilityNetworkRecord {
                        utility: name(*utility)?,
                        units: units.clone(),
                        nodes: nodes.clone(),
                        provided: *provided,
                        consumed: *consumed,
                        short: *short,
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UT7 wire size: one owner view at the slot limits (512 junctions, 512 segments) with
    /// the fixture utility id, and the largest network row count (one row per junction plus
    /// one row per port unit). The M6 budget is 64 KiB for one view.
    #[test]
    fn a_view_at_the_slot_limits_fits_the_wire_budget() {
        let utility = || "utility_yard:power_line".to_string();
        let view = UtilityView {
            nodes: (0..512)
                .map(|id| UtilityNodeRecord {
                    id: 1_000 + id,
                    utility: utility(),
                    pos: Pos {
                        x: 1023.5 - id as f32,
                        y: 511.25,
                    },
                })
                .collect(),
            segments: (0..512)
                .map(|id| UtilitySegmentRecord {
                    id: 1_000 + id,
                    utility: utility(),
                    a: UtilityEndRecord::Node(1_000 + id),
                    b: UtilityEndRecord::Port(UnitId(100_000 + id)),
                    elevation: -64,
                    depth: 4,
                    state: UtilitySegmentState::Disconnected,
                })
                .collect(),
            networks: (0..1024)
                .map(|id| UtilityNetworkRecord {
                    utility: utility(),
                    units: vec![UnitId(100_000 + id)],
                    nodes: vec![1_000 + id],
                    provided: u64::from(id) * 1_000_000,
                    consumed: u64::from(id) * 1_000_001,
                    short: true,
                })
                .collect(),
        };
        let bytes = postcard::to_allocvec(&view).unwrap();
        println!("UT7_WIRE_VIEW_BYTES {}", bytes.len());
        assert!(bytes.len() <= 64 * 1024, "{} bytes", bytes.len());
        assert_eq!(postcard::from_bytes::<UtilityView>(&bytes).unwrap(), view);
        let json = serde_json::to_string(&view).unwrap();
        assert_eq!(serde_json::from_str::<UtilityView>(&json).unwrap(), view);
    }

    #[test]
    fn a_wire_index_outside_the_dictionary_is_refused() {
        let wire = WireView {
            utilities: vec!["a:b".into()],
            nodes: vec![(1, 1, Pos { x: 0.0, y: 0.0 })],
            segments: vec![],
            networks: vec![],
        };
        let bytes = postcard::to_allocvec(&wire).unwrap();
        assert!(postcard::from_bytes::<UtilityView>(&bytes).is_err());
    }
}
