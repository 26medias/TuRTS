//! The `UtilityRoute` command (spec 11, "Placement validation").
//!
//! `apply_route` runs on admission, from one early branch in `orders::apply::apply_order`.
//! A route never enters a unit order queue. The checks run in the spec order and each gives
//! one refusal reason. The payment and the topology change commit in one audited operation.

use super::state::{End, GridGeom, Point, Segment, SegmentState, SlotTopology, UtilityState};
use super::{UtilityWrite, grid_geometry};
use crate::SlotId;
use crate::command::{Applied, RejectReason};
use crate::economy::ledger::{Endpoint, ReservationLine};
use crate::orders::{OrderCommand, OrderTargetSim, UnitOutcomes};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use std::collections::BTreeMap;
use turts_mod_api::{
    ContentId, MAX_UTILITY_JUNCTIONS_PER_SLOT, MAX_UTILITY_SEGMENTS_PER_SLOT, ObservationPhase,
    ResolvedUtility, TargetKind, UTILITY_MIN_SEGMENT_LENGTH, UTILITY_SPLIT_TOLERANCE,
    UtilityEndpoint,
};
use turts_rules::terrain::geometry::{Budget, Cell, Grid, Surface};
use turts_rules::utilities as rules;

type RouteResult = (Result<Applied, RejectReason>, UnitOutcomes);

/// An endpoint after step 3: what it names and where it is.
#[derive(Debug, Clone, Copy)]
enum Resolved {
    Existing(End, Point),
    Split { segment: u32, point: Point },
    Free(Point),
}

impl Resolved {
    fn pos(self) -> Point {
        match self {
            Self::Existing(_, pos) | Self::Free(pos) => pos,
            Self::Split { point, .. } => point,
        }
    }
    fn new_nodes(self) -> usize {
        usize::from(!matches!(self, Self::Existing(..)))
    }
}

fn refuse_all(
    mut outcomes: UnitOutcomes,
    units: &[(u32, bevy_math::Vec2)],
    reason: RejectReason,
) -> RouteResult {
    for (unit, _) in units {
        outcomes.refused.push((*unit, reason));
    }
    outcomes.accepted.clear();
    outcomes.refused.sort_unstable_by_key(|(unit, _)| *unit);
    let first = outcomes
        .refused
        .first()
        .map_or(RejectReason::NotApplicable, |(_, reason)| *reason);
    (Err(first), outcomes)
}

/// Step 1: the envelope. Own, live, completed issuing units of a listed type.
fn check_unit(
    world: &World,
    slot: SlotId,
    command: &OrderCommand,
    unit: u32,
    entities: &BTreeMap<u32, Entity>,
) -> Result<bevy_math::Vec2, RejectReason> {
    let entity = *entities.get(&unit).ok_or(RejectReason::UnknownUnit)?;
    if world.get::<crate::Owner>(entity).map(|owner| owner.0) != Some(slot) {
        return Err(RejectReason::NotOwner);
    }
    if world
        .resource::<crate::economy::construction::ConstructionState>()
        .sites
        .contains_key(&unit)
        || world
            .get_resource::<crate::economy::combat::Healths>()
            .is_some_and(|healths| healths.0.get(&unit) == Some(&0))
    {
        return Err(RejectReason::NotApplicable);
    }
    let kind = world
        .get::<crate::UnitType>(entity)
        .ok_or(RejectReason::NotApplicable)?;
    let content = &world
        .resource::<crate::registry::UnitTypes>()
        .get(kind.0)
        .ok_or(RejectReason::NotApplicable)?
        .content_id;
    // The order's `units` list is required: an empty list names no builder.
    if !command
        .allowed_units
        .iter()
        .any(|allowed| allowed == content)
    {
        return Err(RejectReason::NotApplicable);
    }
    world
        .get::<crate::Position>(entity)
        .map(|position| position.0)
        .filter(|position| position.is_finite())
        .ok_or(RejectReason::IntegrityViolation)
}

/// Step 3 for one endpoint. Every failure is the same `TargetInvalid`: a foreign id and an
/// absent id give the same answer.
fn resolve(
    world: &World,
    state: &UtilityState,
    slot: SlotId,
    def: &ResolvedUtility,
    endpoint: &UtilityEndpoint,
    bounds: bevy_math::Vec2,
) -> Option<Resolved> {
    let inside = |point: Point| {
        point.0.is_finite()
            && point.1.is_finite()
            && point.0 >= 0.0
            && point.1 >= 0.0
            && point.0 <= bounds.x
            && point.1 <= bounds.y
    };
    let topology = state.topology(slot.0);
    let resolved = match endpoint {
        UtilityEndpoint::Port(unit) => {
            let entity = crate::unit_index::unit_entity(world, *unit)?;
            if world.get::<crate::Owner>(entity)?.0 != slot {
                return None;
            }
            if world
                .resource::<crate::economy::construction::ConstructionState>()
                .sites
                .contains_key(unit)
                || world
                    .get_resource::<crate::economy::combat::Healths>()
                    .is_some_and(|healths| healths.0.get(unit) == Some(&0))
            {
                return None;
            }
            let kind = world.get::<crate::UnitType>(entity)?.0;
            let content = ContentId::parse(
                &world
                    .resource::<crate::registry::UnitTypes>()
                    .get(kind)?
                    .content_id,
            )
            .ok()?;
            let offset = def.ports.get(&content)?;
            let position = world.get::<crate::Position>(entity)?.0;
            Resolved::Existing(
                End::Port(*unit),
                (position.x + offset.x, position.y + offset.y),
            )
        }
        UtilityEndpoint::Node(id) => {
            let node = topology?.nodes.get(id)?;
            if node.utility != def.id {
                return None;
            }
            Resolved::Existing(End::Node(*id), node.pos)
        }
        UtilityEndpoint::Split { segment, point } => {
            let stored = topology?.segments.get(segment)?;
            let point = (point.x, point.y);
            if stored.utility != def.id
                || !inside(point)
                || rules::distance_to_segment(point, stored.a_pos, stored.b_pos)
                    > UTILITY_SPLIT_TOLERANCE
                || rules::length(point, stored.a_pos) <= UTILITY_MIN_SEGMENT_LENGTH
                || rules::length(point, stored.b_pos) <= UTILITY_MIN_SEGMENT_LENGTH
            {
                return None;
            }
            Resolved::Split {
                segment: *segment,
                point,
            }
        }
        UtilityEndpoint::Free(point) => Resolved::Free((point.x, point.y)),
    };
    inside(resolved.pos()).then_some(resolved)
}

/// Whether the slot currently observes the cell, in the Completed basis of the previous
/// tick. A map with no terrain uses a virtual 1-unit grid when its size is whole.
fn observed(world: &World, slot: SlotId, cell: (u32, u32)) -> bool {
    let cell = Cell {
        x: cell.0,
        y: cell.1,
    };
    if let Some(terrain) = world.get_resource::<crate::terrain::TerrainState>() {
        return crate::terrain::step::observed(
            world,
            slot.0,
            terrain,
            cell,
            ObservationPhase::Completed,
        );
    }
    let Some(bounds) = world.get_resource::<crate::MapBounds>() else {
        return true;
    };
    let (width, height) = (bounds.size.x, bounds.size.y);
    if width.fract() != 0.0 || height.fract() != 0.0 {
        return true;
    }
    let Ok(grid) = Grid::checked(width as u32, height as u32, 1, 1, 1, 0, 1) else {
        return true;
    };
    let Some(basis) = crate::visibility::basis(world, ObservationPhase::Completed) else {
        return true;
    };
    let limit = usize::try_from(u64::from(grid.columns) * u64::from(grid.rows))
        .ok()
        .and_then(|cells| cells.checked_mul(2))
        .and_then(|cells| cells.checked_add(16));
    let Some(limit) = limit else {
        return false;
    };
    basis
        .terrain_cell_observed(slot.0, &grid, cell, &mut Budget::new(limit))
        .unwrap_or(false)
}

/// Create the node or the split that an endpoint asks for; return the segment end.
fn materialize(
    resolved: Resolved,
    utility: &ContentId,
    topology: &mut SlotTopology,
    made: &mut Vec<u32>,
) -> End {
    match resolved {
        Resolved::Existing(end, _) => end,
        Resolved::Free(pos) => End::Node(add_node(topology, utility, pos)),
        Resolved::Split { segment, point } => {
            let old = topology
                .segments
                .remove(&segment)
                .expect("a validated split names a live segment");
            let node = add_node(topology, utility, point);
            let halves = [
                (old.a, End::Node(node), old.a_pos, point),
                (End::Node(node), old.b, point, old.b_pos),
            ];
            for (a, b, a_pos, b_pos) in halves {
                let id = topology.next_segment;
                topology.next_segment += 1;
                topology.segments.insert(
                    id,
                    Segment {
                        utility: utility.clone(),
                        a,
                        b,
                        a_pos,
                        b_pos,
                        elevation: old.elevation,
                        depth: old.depth,
                        state: old.state,
                    },
                );
                made.push(id);
            }
            End::Node(node)
        }
    }
}

fn add_node(topology: &mut SlotTopology, utility: &ContentId, pos: Point) -> u32 {
    let id = topology.next_node;
    topology.next_node += 1;
    topology.nodes.insert(
        id,
        super::state::Node {
            utility: utility.clone(),
            pos,
        },
    );
    id
}

pub(crate) fn apply_route(
    world: &mut World,
    slot: SlotId,
    command: &OrderCommand,
    unit_ids: &[u32],
    entities: &BTreeMap<u32, Entity>,
) -> RouteResult {
    let mut outcomes = UnitOutcomes::default();
    // Step 1: envelope.
    let mut valid: Vec<(u32, bevy_math::Vec2)> = Vec::new();
    for &unit in unit_ids {
        match check_unit(world, slot, command, unit, entities) {
            Ok(position) => valid.push((unit, position)),
            Err(reason) => outcomes.refused.push((unit, reason)),
        }
    }
    if valid.is_empty() {
        return refuse_all(outcomes, &valid, RejectReason::NotApplicable);
    }
    let OrderTargetSim::UtilityRoute(route) = &command.target else {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    };
    if command.target_kind != TargetKind::UtilityRoute {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    }
    // Step 2: the utility type and the depth.
    let Some(catalog) = world.get_resource::<crate::bootstrap::InstalledCatalog>() else {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    };
    let Some(def) = ContentId::parse(&route.utility)
        .ok()
        .and_then(|id| catalog.0.utilities.get(&id))
        .cloned()
    else {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    };
    if route.depth < def.depth.min || route.depth > def.depth.max {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    }
    let (Some(state), Some(bounds), Some(geom)) = (
        world.get_resource::<UtilityState>(),
        world.get_resource::<crate::MapBounds>().map(|b| b.size),
        grid_geometry(world),
    ) else {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    };
    // Step 3: endpoints.
    let (Some(from), Some(to)) = (
        resolve(world, state, slot, &def, &route.from, bounds),
        resolve(world, state, slot, &def, &route.to, bounds),
    ) else {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    };
    // Step 4: the two endpoints differ, the length, no duplicate pair.
    let same = match (from, to) {
        (Resolved::Existing(a, _), Resolved::Existing(b, _)) => a == b,
        (Resolved::Split { segment: a, .. }, Resolved::Split { segment: b, .. }) => a == b,
        _ => false,
    };
    if same {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    }
    let length = rules::length(from.pos(), to.pos());
    if length <= UTILITY_MIN_SEGMENT_LENGTH {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    }
    if length > def.max_segment_length {
        return refuse_all(outcomes, &valid, RejectReason::OutOfRange);
    }
    if let (Resolved::Existing(a, _), Resolved::Existing(b, _)) = (from, to)
        && state.topology(slot.0).is_some_and(|topology| {
            topology.segments.values().any(|segment| {
                segment.utility == def.id
                    && ((segment.a == a && segment.b == b) || (segment.a == b && segment.b == a))
            })
        })
    {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    }
    // A split joined to an end of the split segment would duplicate one of its halves.
    let split_duplicates = |split: Resolved, other: Resolved| match (split, other) {
        (Resolved::Split { segment, .. }, Resolved::Existing(end, _)) => state
            .topology(slot.0)
            .and_then(|topology| topology.segments.get(&segment))
            .is_some_and(|segment| segment.a == end || segment.b == end),
        _ => false,
    };
    if split_duplicates(from, to) || split_duplicates(to, from) {
        return refuse_all(outcomes, &valid, RejectReason::TargetInvalid);
    }
    // Step 5: reach. A unit out of reach of either endpoint is refused on its own.
    let within = |position: bevy_math::Vec2, point: Point| {
        rules::length((position.x, position.y), point) <= def.reach
    };
    let (in_reach, out_of_reach): (Vec<_>, Vec<_>) = valid
        .iter()
        .copied()
        .partition(|(_, position)| within(*position, from.pos()) && within(*position, to.pos()));
    for (unit, _) in &out_of_reach {
        outcomes.refused.push((*unit, RejectReason::OutOfRange));
    }
    if in_reach.is_empty() {
        return refuse_all(outcomes, &[], RejectReason::OutOfRange);
    }
    // Step 6: cells, observation, fixed elevation.
    let cells = match geom.try_cells_of(from.pos(), to.pos()) {
        Ok(cells) => cells,
        Err(rules::TouchError::TooManyCells) => {
            return refuse_all(outcomes, &in_reach, RejectReason::UtilityLimitReached);
        }
        Err(_) => return refuse_all(outcomes, &in_reach, RejectReason::TargetInvalid),
    };
    if !halves_fit(state, slot, geom, from, to) {
        return refuse_all(outcomes, &in_reach, RejectReason::UtilityLimitReached);
    }
    if cells.iter().any(|cell| !observed(world, slot, *cell)) {
        return refuse_all(outcomes, &in_reach, RejectReason::UtilityNotObserved);
    }
    let elevation = match world.get_resource::<crate::terrain::TerrainState>() {
        Some(terrain) => {
            let surfaces = cells
                .iter()
                .filter_map(|&(x, y)| terrain.elevation(Cell { x, y }));
            match rules::placement_elevation(surfaces, route.depth) {
                Some(elevation) if elevation >= terrain.grid.floor => elevation,
                _ => return refuse_all(outcomes, &in_reach, RejectReason::OutOfRange),
            }
        }
        None => match rules::placement_elevation([0], route.depth) {
            Some(elevation) => elevation,
            None => return refuse_all(outcomes, &in_reach, RejectReason::OutOfRange),
        },
    };
    // Step 7: slot limits, with the new junctions and the splits counted.
    let splits = usize::from(matches!(from, Resolved::Split { .. }))
        + usize::from(matches!(to, Resolved::Split { .. }));
    let (junctions, segments) = state.topology(slot.0).map_or((0, 0), |topology| {
        (topology.nodes.len(), topology.segments.len())
    });
    if junctions + from.new_nodes() + to.new_nodes() > MAX_UTILITY_JUNCTIONS_PER_SLOT
        || segments + 1 + splits > MAX_UTILITY_SEGMENTS_PER_SLOT
    {
        return refuse_all(outcomes, &in_reach, RejectReason::UtilityLimitReached);
    }
    // Build the new topology of the slot. Nothing is written until the transaction commits.
    let base = state.counters(slot.0);
    let mut topology = state.topology(slot.0).cloned().unwrap_or_default();
    let mut made = Vec::new();
    let a = materialize(from, &def.id, &mut topology, &mut made);
    let b = materialize(to, &def.id, &mut topology, &mut made);
    let id = topology.next_segment;
    topology.next_segment += 1;
    topology.segments.insert(
        id,
        Segment {
            utility: def.id.clone(),
            a,
            b,
            a_pos: from.pos(),
            b_pos: to.pos(),
            elevation,
            depth: route.depth,
            state: SegmentState::Conducting,
        },
    );
    made.push(id);
    let write = UtilityWrite {
        slot: slot.0,
        utility: def.id.clone(),
        base,
        topology,
        made,
    };
    // Step 8: the payment and the write are one audited operation.
    let lines: Vec<ReservationLine> = def
        .segment_cost
        .iter()
        .filter(|(_, amount)| *amount != 0)
        .map(|(resource, amount)| ReservationLine {
            endpoint: Endpoint::Slot(slot.0),
            resource: resource.clone(),
            amount: *amount,
        })
        .collect();
    let committed = crate::economy::transaction::transact(world, |tx| {
        if !lines.is_empty() {
            crate::economy::transaction::charge_costs(tx, &lines)?;
        }
        tx.stage_utility(write)
    });
    if let Err(reason) = committed {
        return refuse_all(outcomes, &in_reach, reason);
    }
    outcomes.accepted = in_reach.iter().map(|(unit, _)| *unit).collect();
    outcomes.accepted.sort_unstable();
    outcomes.refused.sort_unstable_by_key(|(unit, _)| *unit);
    for unit in outcomes.accepted.clone() {
        crate::orders::note_accepted_player_order(world, unit, command.behaviour, command.queue);
    }
    (Ok(Applied { job_id: None }), outcomes)
}

/// A split half must also fit the cell limit (the split point can lie 0.25 off the line).
fn halves_fit(
    state: &UtilityState,
    slot: SlotId,
    geom: GridGeom,
    from: Resolved,
    to: Resolved,
) -> bool {
    [from, to].iter().all(|end| {
        let Resolved::Split { segment, point } = end else {
            return true;
        };
        state
            .topology(slot.0)
            .and_then(|topology| topology.segments.get(segment))
            .is_some_and(|old| {
                geom.try_cells_of(old.a_pos, *point).is_ok()
                    && geom.try_cells_of(*point, old.b_pos).is_ok()
            })
    })
}
