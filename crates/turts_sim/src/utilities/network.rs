//! Connected networks and their exact totals (spec 11, "Operating model").
//!
//! Components are rebuilt only for the (slot, utility) pairs whose topology changed. The
//! totals are summed every tick, for every network, from the live completed port units.

use super::state::{End, NetworkRow, SlotTopology, UtilityState};
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::ContentId;

/// One connected set of junctions and port units of one slot and utility type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Component {
    pub(crate) nodes: Vec<u32>,
    pub(crate) units: Vec<u32>,
}

/// A live completed port unit of one utility type and its amounts of the Network resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PortUnit {
    pub(crate) slot: u8,
    pub(crate) utility: ContentId,
    pub(crate) provided: u64,
    pub(crate) consumed: u64,
}

/// Unit id to (provided, consumed) of the Network resource.
type Amounts = BTreeMap<u32, (u64, u64)>;

/// Live completed port units by unit id.
pub(crate) type LivePorts = BTreeMap<u32, Vec<PortUnit>>;

pub(crate) fn is_live(live: &LivePorts, unit: u32, slot: u8, utility: &ContentId) -> bool {
    live.get(&unit).is_some_and(|ports| {
        ports
            .iter()
            .any(|p| p.slot == slot && p.utility == *utility)
    })
}

struct Union {
    parent: Vec<usize>,
}

impl Union {
    fn find(&mut self, mut index: usize) -> usize {
        while self.parent[index] != index {
            self.parent[index] = self.parent[self.parent[index]];
            index = self.parent[index];
        }
        index
    }
    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            // The smaller index is the root, so the result does not depend on edge order.
            self.parent[a.max(b)] = a.min(b);
        }
    }
}

/// Union-find over the conducting segments of one utility type.
pub(crate) fn rebuild(topology: &SlotTopology, utility: &ContentId) -> Vec<Component> {
    let mut vertices: BTreeMap<End, usize> = BTreeMap::new();
    let mut edges = Vec::new();
    for segment in topology.segments.values() {
        if segment.utility != *utility || !segment.state.conducts() {
            continue;
        }
        edges.push((segment.a, segment.b));
        for end in [segment.a, segment.b] {
            let next = vertices.len();
            vertices.entry(end).or_insert(next);
        }
    }
    let mut union = Union {
        parent: (0..vertices.len()).collect(),
    };
    for (a, b) in &edges {
        union.join(vertices[a], vertices[b]);
    }
    let mut groups: BTreeMap<usize, Component> = BTreeMap::new();
    for (end, index) in &vertices {
        let root = union.find(*index);
        let group = groups.entry(root).or_default();
        match end {
            End::Node(id) => group.nodes.push(*id),
            End::Port(unit) => group.units.push(*unit),
        }
    }
    let mut components: Vec<Component> = groups.into_values().collect();
    // `vertices` iterates ascending by (variant, id), so each list is ascending already.
    components.sort_by_key(|component| {
        (
            component.units.first().copied().unwrap_or(u32::MAX),
            component.nodes.first().copied().unwrap_or(u32::MAX),
        )
    });
    components
}

fn saturate(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Rebuild the dirty components, then write `next` from the live completed port units.
/// `resource_of` names the Network resource that a utility type carries.
pub(crate) fn run_networks(
    state: &mut UtilityState,
    live: &LivePorts,
    resource_of: &dyn Fn(&ContentId) -> Option<ContentId>,
) {
    for (slot, utility) in std::mem::take(&mut state.dirty) {
        let components = state
            .topology(slot)
            .map(|topology| rebuild(topology, &utility))
            .unwrap_or_default();
        let by_utility = state.components.entry(slot).or_default();
        if components.is_empty() {
            by_utility.remove(&utility);
        } else {
            by_utility.insert(utility, components);
        }
    }
    // The live units of each (slot, utility), with their amounts.
    let mut by_pair: BTreeMap<(u8, &ContentId), Amounts> = BTreeMap::new();
    for (unit, ports) in live {
        for port in ports {
            by_pair
                .entry((port.slot, &port.utility))
                .or_default()
                .insert(*unit, (port.provided, port.consumed));
        }
    }
    let mut next: BTreeMap<u8, Vec<NetworkRow>> = BTreeMap::new();
    let row = |utility: &ContentId,
               units: Vec<u32>,
               nodes: Vec<u32>,
               amounts: &BTreeMap<u32, (u64, u64)>| {
        let resource = resource_of(utility)?;
        let (mut provided, mut consumed) = (0_u128, 0_u128);
        for unit in &units {
            let (p, c) = amounts.get(unit).copied().unwrap_or((0, 0));
            // u128 cannot overflow from u64 amounts; the saturation is a guard, not a policy.
            provided = provided.saturating_add(u128::from(p));
            consumed = consumed.saturating_add(u128::from(c));
        }
        Some(NetworkRow {
            utility: utility.clone(),
            resource,
            units,
            nodes,
            provided: saturate(provided),
            consumed: saturate(consumed),
            short: turts_rules::utilities::network_short(provided, consumed),
        })
    };
    let empty = BTreeMap::new();
    let mut assigned: BTreeSet<(u8, &ContentId, u32)> = BTreeSet::new();
    for (slot, by_utility) in &state.components {
        for (utility, components) in by_utility {
            let amounts = by_pair.get(&(*slot, utility)).unwrap_or(&empty);
            for component in components {
                // A port end whose unit is not live was converted before this step.
                let units: Vec<u32> = component
                    .units
                    .iter()
                    .copied()
                    .filter(|unit| amounts.contains_key(unit))
                    .collect();
                for unit in &units {
                    assigned.insert((*slot, utility, *unit));
                }
                if let Some(row) = row(utility, units, component.nodes.clone(), amounts) {
                    next.entry(*slot).or_default().push(row);
                }
            }
        }
    }
    // Every live port unit with no conducting segment is a network of its own.
    for ((slot, utility), amounts) in &by_pair {
        for unit in amounts.keys() {
            if assigned.contains(&(*slot, *utility, *unit)) {
                continue;
            }
            if let Some(row) = row(utility, vec![*unit], Vec::new(), amounts) {
                next.entry(*slot).or_default().push(row);
            }
        }
    }
    for rows in next.values_mut() {
        rows.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }
    state.next = next;
}

/// The per-unit short flags of a row set, for `network_short` lookups.
pub(crate) fn unit_flags(
    networks: &BTreeMap<u8, Vec<NetworkRow>>,
) -> BTreeMap<u32, Vec<(ContentId, bool)>> {
    let mut flags: BTreeMap<u32, Vec<(ContentId, bool)>> = BTreeMap::new();
    for rows in networks.values() {
        for row in rows {
            for unit in &row.units {
                flags
                    .entry(*unit)
                    .or_default()
                    .push((row.resource.clone(), row.short));
            }
        }
    }
    flags
}
