//! M6 utility state: the per-slot topology, the `effective` network rows, and the runtime
//! indexes that the pass uses. The topology and the rows are `UtilitySnapshot`; the
//! snapshot is shared with `WorldView` by `Arc`, and it is copied only when it changes.

use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use turts_mod_api::ContentId;

/// A plan point in world units.
pub type Point = (f32, f32);

/// One end of a segment. A port end names the unit; a node end names an own junction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum End {
    Node(u32),
    Port(u32),
}

/// The state of one segment. `Exposed` conducts (policy `Operate`); `Disconnected` does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentState {
    Conducting,
    Exposed,
    Disconnected,
}

impl SegmentState {
    pub(crate) fn conducts(self) -> bool {
        self != Self::Disconnected
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub utility: ContentId,
    pub pos: Point,
}

/// One segment. The elevation never changes after placement (spec 11.2). The touched cells
/// are not stored: `cells_of` computes them from the two end points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub utility: ContentId,
    pub a: End,
    pub b: End,
    pub a_pos: Point,
    pub b_pos: Point,
    pub elevation: i32,
    /// The depth that was requested at placement.
    pub depth: u32,
    pub state: SegmentState,
}

/// The underground layer of one slot. Ids are counted per slot and never reused.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotTopology {
    pub next_node: u32,
    pub next_segment: u32,
    pub nodes: BTreeMap<u32, Node>,
    pub segments: BTreeMap<u32, Segment>,
}

impl Default for SlotTopology {
    fn default() -> Self {
        Self {
            next_node: 1,
            next_segment: 1,
            nodes: BTreeMap::new(),
            segments: BTreeMap::new(),
        }
    }
}

/// One connected network of one slot and utility type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRow {
    pub utility: ContentId,
    /// The Network resource that the utility type carries.
    pub resource: ContentId,
    /// Live completed port units, ascending.
    pub units: Vec<u32>,
    /// Junctions, ascending.
    pub nodes: Vec<u32>,
    pub provided: u64,
    pub consumed: u64,
    pub short: bool,
}

impl NetworkRow {
    pub(crate) fn sort_key(&self) -> (&ContentId, u32, u32) {
        (
            &self.utility,
            self.units.first().copied().unwrap_or(u32::MAX),
            self.nodes.first().copied().unwrap_or(u32::MAX),
        )
    }
}

/// Everything that `WorldView` publishes: topology, the `effective` rows and the ticks at
/// which the owner rows of a slot last changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UtilitySnapshot {
    pub slots: BTreeMap<u8, SlotTopology>,
    pub networks: BTreeMap<u8, Vec<NetworkRow>>,
    /// For each port unit in `networks`: `(Network resource, short)`.
    pub unit_short: BTreeMap<u32, Vec<(ContentId, bool)>>,
    pub changed: BTreeMap<u8, u32>,
}

/// An `Arc` of the snapshot that serializes as the snapshot (the crate has no serde `rc`).
#[derive(Debug, Clone, Default)]
pub struct SharedSnapshot(pub Arc<UtilitySnapshot>);

impl PartialEq for SharedSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || *self.0 == *other.0
    }
}

impl Serialize for SharedSnapshot {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SharedSnapshot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        UtilitySnapshot::deserialize(deserializer).map(|snapshot| Self(Arc::new(snapshot)))
    }
}

/// The grid that the utility rules use: the terrain grid, or one virtual flat grid of
/// 1-unit cells over the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridGeom {
    pub cell_width: f32,
    pub columns: u32,
    pub rows: u32,
}

impl GridGeom {
    /// The cells whose closed square meets the segment. Empty if the rules refuse it.
    pub fn cells_of(self, a: Point, b: Point) -> Vec<(u32, u32)> {
        self.try_cells_of(a, b).unwrap_or_default()
    }

    pub fn try_cells_of(
        self,
        a: Point,
        b: Point,
    ) -> Result<Vec<(u32, u32)>, turts_rules::utilities::TouchError> {
        turts_rules::utilities::touched_cells(
            a,
            b,
            self.cell_width,
            self.columns,
            self.rows,
            turts_mod_api::MAX_UTILITY_SEGMENT_CELLS,
        )
    }
}

/// A snapshot of the state used to detect a stale write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counters {
    pub(crate) next_node: u32,
    pub(crate) next_segment: u32,
}

/// The utility resource. Installed by bootstrap when the catalog has utility types.
#[derive(Resource, Debug, Clone, Default)]
pub struct UtilityState {
    pub(crate) snap: SharedSnapshot,
    /// Rows that the pass of the last tick wrote. `begin_tick` swaps them into `snap`.
    pub(crate) next: BTreeMap<u8, Vec<NetworkRow>>,
    /// For each slot: cell to the segments that touch it.
    pub(crate) index: BTreeMap<u8, BTreeMap<(u32, u32), BTreeSet<u32>>>,
    /// Cached components of each (slot, utility). Rebuilt only for `dirty` pairs.
    pub(crate) components: BTreeMap<u8, BTreeMap<ContentId, Vec<super::network::Component>>>,
    pub(crate) dirty: BTreeSet<(u8, ContentId)>,
    /// Segments that a placement or a split made in this tick.
    pub(crate) pending: BTreeSet<(u8, u32)>,
    /// Operator-only: segments that the exposure pass checked (cumulative).
    pub(crate) checked_segments: u64,
}

impl UtilityState {
    pub(crate) fn topology(&self, slot: u8) -> Option<&SlotTopology> {
        self.snap.0.slots.get(&slot)
    }

    pub(crate) fn counters(&self, slot: u8) -> Counters {
        self.topology(slot).map_or(
            Counters {
                next_node: 1,
                next_segment: 1,
            },
            |topology| Counters {
                next_node: topology.next_node,
                next_segment: topology.next_segment,
            },
        )
    }

    pub(crate) fn edit(&mut self) -> &mut UtilitySnapshot {
        Arc::make_mut(&mut self.snap.0)
    }

    pub(crate) fn topology_mut(&mut self, slot: u8) -> &mut SlotTopology {
        self.edit().slots.entry(slot).or_default()
    }

    pub(crate) fn mark_changed(&mut self, slot: u8, tick: u32) {
        self.edit().changed.insert(slot, tick);
    }

    pub(crate) fn mark_dirty(&mut self, slot: u8, utility: &ContentId) {
        self.dirty.insert((slot, utility.clone()));
    }

    pub(crate) fn index_add(&mut self, slot: u8, segment: u32, cells: &[(u32, u32)]) {
        let index = self.index.entry(slot).or_default();
        for cell in cells {
            index.entry(*cell).or_default().insert(segment);
        }
    }

    pub(crate) fn index_remove(&mut self, slot: u8, segment: u32, cells: &[(u32, u32)]) {
        let Some(index) = self.index.get_mut(&slot) else {
            return;
        };
        for cell in cells {
            if let Some(set) = index.get_mut(cell) {
                set.remove(&segment);
                if set.is_empty() {
                    index.remove(cell);
                }
            }
        }
    }

    /// Delete one segment, its index rows, and the junctions that have no segment left.
    /// The caller marks the change tick. Returns false if the segment is absent.
    pub(crate) fn delete_segment(
        &mut self,
        slot: u8,
        segment: u32,
        geom: Option<GridGeom>,
    ) -> bool {
        let Some(removed) = self
            .topology(slot)
            .and_then(|topology| topology.segments.get(&segment))
            .cloned()
        else {
            return false;
        };
        if let Some(geom) = geom {
            let cells = geom.cells_of(removed.a_pos, removed.b_pos);
            self.index_remove(slot, segment, &cells);
        }
        let topology = self.topology_mut(slot);
        topology.segments.remove(&segment);
        for end in [removed.a, removed.b] {
            if let End::Node(node) = end
                && !topology
                    .segments
                    .values()
                    .any(|other| other.a == End::Node(node) || other.b == End::Node(node))
            {
                topology.nodes.remove(&node);
            }
        }
        self.pending.remove(&(slot, segment));
        self.mark_dirty(slot, &removed.utility);
        true
    }
}
