//! M6 underground utilities: one logical layer of junctions and segments for each slot,
//! burial and exposure, and aggregate supply per connected network.
//!
//! The engine owns the mechanism. The utility types, their ports, depth, reach, cost and the
//! exposure policy are mod data (`turts_mod_api::ResolvedUtility`). `SimSet::Utilities` runs
//! `run_pass` after `Outcome` in each tick: port removal, then exposure, then the networks.
//! The pass of tick `T` writes `next`; `power::run_supply` swaps it into `effective` at the
//! start of tick `T + 1` (`begin_tick`). Every reader uses `effective`.

mod exposure;
mod network;
mod route;
mod state;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod bench;

pub use state::{
    End, GridGeom, NetworkRow, Node, Point, Segment, SegmentState, SharedSnapshot, SlotTopology,
    UtilitySnapshot, UtilityState,
};

pub(crate) use route::apply_route;

use crate::bootstrap::InstalledCatalog;
use crate::economy::views::WorldView;
use crate::world::Tick;
use bevy_ecs::prelude::World;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::ContentId;
use turts_rules::terrain::geometry::{Cell, Surface};

/// One prepared route: the new topology of one slot. It commits with the payment
/// (`Transaction::stage_utility`), so a refusal changes nothing.
#[derive(Debug, Clone)]
pub(crate) struct UtilityWrite {
    slot: u8,
    utility: ContentId,
    /// The id counters of the slot when the write was prepared. A stale write is refused.
    base: state::Counters,
    topology: SlotTopology,
    /// The segments that this route made or split. The next pass checks their exposure.
    made: Vec<u32>,
}

/// The grid that the utility rules use for this world.
pub fn grid_geometry(world: &World) -> Option<GridGeom> {
    if let Some(terrain) = world.get_resource::<crate::terrain::TerrainState>() {
        let grid = terrain.grid;
        return Some(GridGeom {
            cell_width: (f64::from(grid.cell_width) / f64::from(grid.length_scale)) as f32,
            columns: grid.columns,
            rows: grid.rows,
        });
    }
    let size = world.get_resource::<crate::MapBounds>()?.size;
    if !(size.x.is_finite() && size.y.is_finite() && size.x > 0.0 && size.y > 0.0) {
        return None;
    }
    Some(GridGeom {
        cell_width: 1.0,
        columns: size.x.ceil() as u32,
        rows: size.y.ceil() as u32,
    })
}

pub(crate) fn validates(world: &World, write: &UtilityWrite) -> bool {
    world
        .get_resource::<UtilityState>()
        .is_some_and(|state| state.counters(write.slot) == write.base)
}

/// Commit a validated write. It has no fallible step.
pub(crate) fn apply_write(world: &mut World, write: UtilityWrite) {
    let tick = world.get_resource::<Tick>().map_or(0, |tick| tick.0);
    let geom = grid_geometry(world);
    let mut state = world.resource_mut::<UtilityState>();
    let slot = write.slot;
    let (removed, added) = {
        let old = state.topology(slot);
        let segments = |segments: &BTreeMap<u32, Segment>,
                        other: Option<&BTreeMap<u32, Segment>>| {
            segments
                .iter()
                .filter(|(id, _)| other.is_none_or(|other| !other.contains_key(id)))
                .map(|(id, segment)| (*id, segment.a_pos, segment.b_pos))
                .collect::<Vec<_>>()
        };
        let empty = BTreeMap::new();
        let old_segments = old.map_or(&empty, |topology| &topology.segments);
        (
            segments(old_segments, Some(&write.topology.segments)),
            segments(&write.topology.segments, Some(old_segments)),
        )
    };
    *state.topology_mut(slot) = write.topology;
    if let Some(geom) = geom {
        for (id, a, b) in removed {
            state.index_remove(slot, id, &geom.cells_of(a, b));
        }
        for (id, a, b) in added {
            state.index_add(slot, id, &geom.cells_of(a, b));
        }
    }
    state
        .pending
        .extend(write.made.iter().map(|id| (slot, *id)));
    state.mark_dirty(slot, &write.utility);
    state.mark_changed(slot, tick);
}

impl UtilityState {
    /// Swap the `next` rows into `effective`. The owner rows of a slot changed when its rows
    /// differ; the change tick is the current tick.
    fn swap_effective(&mut self, tick: u32) {
        if self.snap.0.networks == self.next {
            return;
        }
        let slots: BTreeSet<u8> = self
            .snap
            .0
            .networks
            .keys()
            .chain(self.next.keys())
            .copied()
            .collect();
        let differing: Vec<u8> = slots
            .into_iter()
            .filter(|slot| self.snap.0.networks.get(slot) != self.next.get(slot))
            .collect();
        let next = self.next.clone();
        let flags = network::unit_flags(&next);
        let snapshot = self.edit();
        for slot in differing {
            snapshot.changed.insert(slot, tick);
        }
        snapshot.networks = next;
        snapshot.unit_short = flags;
    }
}

/// The first step of tick `T + 1`: `next` of tick `T` becomes `effective`.
/// `power::run_supply` calls this outside its own early return.
pub(crate) fn begin_tick(world: &mut World) {
    let tick = world.get_resource::<Tick>().map_or(0, |tick| tick.0);
    if let Some(mut state) = world.get_resource_mut::<UtilityState>() {
        state.swap_effective(tick);
    }
}

/// The live completed port units of every slot, with their amounts of the Network resource.
fn capture_live_ports(
    world: &World,
    catalog: &turts_mod_api::ContentCatalog,
) -> network::LivePorts {
    let mut live = network::LivePorts::new();
    let Some(types) = world.get_resource::<crate::registry::UnitTypes>() else {
        return live;
    };
    // Unit type -> (utility, Network resource, provided, consumed) of its ports.
    let mut port_types: BTreeMap<String, Vec<(&ContentId, u64, u64)>> = BTreeMap::new();
    for utility in catalog.utilities.values() {
        for unit in utility.ports.keys() {
            let amount = |rows: &[turts_mod_api::SupplyAmount]| {
                rows.iter()
                    .filter(|row| {
                        ContentId::parse(&row.resource).ok().as_ref() == Some(&utility.carries)
                    })
                    .map(|row| row.amount)
                    .fold(0_u64, u64::saturating_add)
            };
            let (provided, consumed) = catalog
                .units
                .get(unit)
                .map_or((0, 0), |def| (amount(&def.provides), amount(&def.consumes)));
            port_types
                .entry(unit.to_string())
                .or_default()
                .push((&utility.id, provided, consumed));
        }
    }
    let sites = world.get_resource::<crate::economy::construction::ConstructionState>();
    let healths = world.get_resource::<crate::economy::combat::Healths>();
    for entity in world.iter_entities() {
        let (Some(id), Some(kind), Some(owner)) = (
            entity.get::<crate::UnitId>(),
            entity.get::<crate::UnitType>(),
            entity.get::<crate::Owner>(),
        ) else {
            continue;
        };
        let Some(ports) = types
            .get(kind.0)
            .and_then(|entry| port_types.get(&entry.content_id))
        else {
            continue;
        };
        if sites.is_some_and(|sites| sites.sites.contains_key(&id.0))
            || healths.is_some_and(|healths| healths.0.get(&id.0) == Some(&0))
        {
            continue;
        }
        live.insert(
            id.0,
            ports
                .iter()
                .map(|(utility, provided, consumed)| network::PortUnit {
                    slot: owner.0.0,
                    utility: (*utility).clone(),
                    provided: *provided,
                    consumed: *consumed,
                })
                .collect(),
        );
    }
    live
}

/// `SimSet::Utilities`: port removal, then the exposure check, then the networks.
pub(crate) fn run_pass(world: &mut World) {
    if !world.contains_resource::<UtilityState>() || !world.contains_resource::<InstalledCatalog>()
    {
        return;
    }
    world.resource_scope(
        |world, mut state: bevy_ecs::change_detection::Mut<UtilityState>| {
            let tick = world.resource::<Tick>().0;
            let catalog = &world.resource::<InstalledCatalog>().0;
            let geom = grid_geometry(world);
            let live = capture_live_ports(world, catalog);
            let state = &mut *state;
            exposure::convert_ports(state, &live, geom, tick);
            let terrain = world.get_resource::<crate::terrain::TerrainState>();
            let changed: BTreeSet<(u32, u32)> = terrain.map_or_else(BTreeSet::new, |terrain| {
                terrain
                    .changed_cells()
                    .iter()
                    .map(|cell| (cell.x, cell.y))
                    .collect()
            });
            // A map with no terrain has surface 0 everywhere (spec 11.2).
            let surface = |x: u32, y: u32| match terrain {
                Some(terrain) => terrain.elevation(Cell { x, y }),
                None => Some(0),
            };
            let policy =
                |utility: &ContentId| catalog.utilities.get(utility).map(|def| def.exposure);
            exposure::run_exposure(
                state,
                &exposure::ExposureInput {
                    geom,
                    changed_cells: &changed,
                    surface: &surface,
                    policy: &policy,
                    tick,
                },
            );
            network::run_networks(state, &live, &|utility| {
                catalog
                    .utilities
                    .get(utility)
                    .map(|def| def.carries.clone())
            });
        },
    );
}

/// Whether the Network `resource` is short for the network of `unit`, read from `effective`.
/// A unit with no row in `effective` (it completed after the last pass) is a network of its
/// own: it is short when its own `consumes` is above its own `provides`.
pub(crate) fn network_short(world: &World, unit: u32, resource: &ContentId) -> bool {
    let Some(state) = world.get_resource::<UtilityState>() else {
        return false;
    };
    if let Some((_, short)) = state
        .snap
        .0
        .unit_short
        .get(&unit)
        .and_then(|rows| rows.iter().find(|(row, _)| row == resource))
    {
        return *short;
    }
    let Some(definition) = crate::economy::power::definition(world, unit) else {
        return false;
    };
    let amount = |rows: &[turts_mod_api::SupplyAmount]| {
        rows.iter()
            .filter(|row| ContentId::parse(&row.resource).ok().as_ref() == Some(resource))
            .map(|row| u128::from(row.amount))
            .sum::<u128>()
    };
    turts_rules::utilities::network_short(
        amount(&definition.provides),
        amount(&definition.consumes),
    )
}

/// The snapshot that `world_view` publishes. `None` when the catalog has no utility type.
pub(crate) fn shared_snapshot(world: &World) -> Option<SharedSnapshot> {
    world
        .get_resource::<UtilityState>()
        .map(|state| state.snap.clone())
}

/// Operator-only: the number of segments that the exposure pass has checked. It is not part
/// of any player view.
pub fn privileged_checked_segments(world: &World) -> u64 {
    world
        .get_resource::<UtilityState>()
        .map_or(0, |state| state.checked_segments)
}

/// Operator-only copy of the complete utility state (topology, `effective` rows, `next`).
pub fn privileged_state(world: &World) -> Option<(UtilitySnapshot, BTreeMap<u8, Vec<NetworkRow>>)> {
    world
        .get_resource::<UtilityState>()
        .map(|state| ((*state.snap.0).clone(), state.next.clone()))
}

// ----- owner view (plain sim-side rows for the server) -----

/// One end of an own segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnerEnd {
    Node(u32),
    Port(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnerSegmentState {
    Conducting,
    Exposed,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerNode {
    pub id: u32,
    pub utility: ContentId,
    pub pos: (f32, f32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerSegment {
    pub id: u32,
    pub utility: ContentId,
    pub a: OwnerEnd,
    pub b: OwnerEnd,
    /// Fixed elevation in terrain elevation steps.
    pub elevation: i32,
    /// The depth that was requested at placement.
    pub depth: u32,
    pub state: OwnerSegmentState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerNetwork {
    pub utility: ContentId,
    pub units: Vec<u32>,
    pub nodes: Vec<u32>,
    pub provided: u64,
    pub consumed: u64,
    pub short: bool,
}

/// The utility rows of one slot. All lists are ascending by id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UtilityOwnerView {
    pub nodes: Vec<OwnerNode>,
    pub segments: Vec<OwnerSegment>,
    pub networks: Vec<OwnerNetwork>,
}

/// The rows of `slot` only: no foreign row, count or id. The network rows come from
/// `effective`; the topology rows are current, so a network row can lag by one tick.
pub fn owner_view(view: &WorldView, slot: u8) -> UtilityOwnerView {
    let Some(snapshot) = &view.utilities else {
        return UtilityOwnerView::default();
    };
    let snapshot = &*snapshot.0;
    let mut owner = UtilityOwnerView::default();
    if let Some(topology) = snapshot.slots.get(&slot) {
        owner.nodes = topology
            .nodes
            .iter()
            .map(|(id, node)| OwnerNode {
                id: *id,
                utility: node.utility.clone(),
                pos: node.pos,
            })
            .collect();
        let end = |end: End| match end {
            End::Node(id) => OwnerEnd::Node(id),
            End::Port(unit) => OwnerEnd::Port(unit),
        };
        owner.segments = topology
            .segments
            .iter()
            .map(|(id, segment)| OwnerSegment {
                id: *id,
                utility: segment.utility.clone(),
                a: end(segment.a),
                b: end(segment.b),
                elevation: segment.elevation,
                depth: segment.depth,
                state: match segment.state {
                    SegmentState::Conducting => OwnerSegmentState::Conducting,
                    SegmentState::Exposed => OwnerSegmentState::Exposed,
                    SegmentState::Disconnected => OwnerSegmentState::Disconnected,
                },
            })
            .collect();
    }
    if let Some(rows) = snapshot.networks.get(&slot) {
        owner.networks = rows
            .iter()
            .map(|row| OwnerNetwork {
                utility: row.utility.clone(),
                units: row.units.clone(),
                nodes: row.nodes.clone(),
                provided: row.provided,
                consumed: row.consumed,
                short: row.short,
            })
            .collect();
    }
    owner
}

/// The tick at which the owner rows of `slot` last changed: a placement, a split, an
/// exposure state change, a port conversion or removal, or a changed `effective` network row.
/// `None` when the catalog has no utility type.
pub fn owner_view_changed_tick(view: &WorldView, slot: u8) -> Option<u32> {
    let snapshot = view.utilities.as_ref()?;
    Some(snapshot.0.changed.get(&slot).copied().unwrap_or(0))
}
