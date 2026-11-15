//! Queued terrain work. Candidate order uses physical geometry; IDs are lookup handles only.
use super::{
    TerrainState,
    step::{self, StepWait},
};
use crate::{
    Owner, Position, RejectReason, SimClock, UnitId, UnitType,
    bootstrap::InstalledCatalog,
    orders::{Order, OrderTargetSim, UnitOrders},
    unit_index::unit_entity,
};
use bevy_ecs::{prelude::Resource, world::World};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::{Behaviour, ContentId, ObservationPhase, TerrainCell, TerrainRect};
use turts_rules::terrain::geometry::{Cell, Surface};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainWorkWait {
    Working,
    Unknown,
    Approaching,
    Blocked,
    Disabled,
    Material,
    Floor,
    Ceiling,
    MissingInputs,
    CargoFull,
    Contested,
    Budget,
    Invalid,
    Unloading,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorkState {
    pub(crate) owner: crate::SlotId,
    pub(crate) order: Order,
    pub(crate) cell: Option<TerrainCell>,
    pub(crate) done: u64,
    pub(crate) carry: u64,
    pub(crate) banked: bool,
    pub(crate) required: Option<u64>,
    pub(crate) wait: TerrainWorkWait,
    /// Private CAS state. It is never a public surface revision.
    pub(crate) source_revision: Option<u64>,
}
#[derive(Resource, Default)]
pub(crate) struct TerrainWorkRuntime {
    pub(crate) units: BTreeMap<u32, WorkState>,
    pub(crate) claims: BTreeMap<Cell, u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainWorkView {
    pub unit: u32,
    pub owner_slot: u8,
    pub tool: ContentId,
    pub cells: TerrainRect,
    pub target_elevation: i32,
    pub current_cell: Option<TerrainCell>,
    pub completed_work: u64,
    pub required_work: Option<u64>,
    pub wait: TerrainWorkWait,
}
#[derive(Clone)]
struct Candidate {
    unit: u32,
    cell: Cell,
    distance: f64,
    x: f32,
    y: f32,
    content: ContentId,
}
fn zero(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}
fn rank(left: &Candidate, right: &Candidate) -> std::cmp::Ordering {
    left.distance
        .total_cmp(&right.distance)
        .then_with(|| left.x.total_cmp(&right.x))
        .then_with(|| left.y.total_cmp(&right.y))
        .then_with(|| left.content.cmp(&right.content))
}
fn current(world: &World, unit: u32) -> Option<Order> {
    world
        .get::<UnitOrders>(unit_entity(world, unit)?)?
        .current
        .clone()
}
pub(crate) fn validate_designation(
    world: &World,
    unit: u32,
    target: &OrderTargetSim,
) -> Result<(), RejectReason> {
    let OrderTargetSim::TerrainWork {
        tool,
        cells,
        target_elevation,
    } = target
    else {
        return Err(RejectReason::TargetInvalid);
    };
    let state = world
        .get_resource::<TerrainState>()
        .ok_or(RejectReason::NotApplicable)?;
    if cells.min.x >= cells.max.x
        || cells.min.y >= cells.max.y
        || cells.max.x > state.grid.columns
        || cells.max.y > state.grid.rows
        || u64::from(cells.max.x - cells.min.x) * u64::from(cells.max.y - cells.min.y) > 256
        || *target_elevation < state.grid.floor
        || *target_elevation > state.grid.ceiling
    {
        return Err(RejectReason::TargetInvalid);
    }
    let entity = unit_entity(world, unit).ok_or(RejectReason::UnknownUnit)?;
    let kind = world
        .get::<UnitType>(entity)
        .ok_or(RejectReason::MissingCapability)?;
    let id = world
        .get_resource::<crate::UnitTypes>()
        .and_then(|types| types.get(kind.0))
        .and_then(|entry| ContentId::parse(&entry.content_id).ok())
        .ok_or(RejectReason::MissingCapability)?;
    let catalog = world
        .get_resource::<InstalledCatalog>()
        .ok_or(RejectReason::MissingCapability)?;
    if !catalog
        .0
        .units
        .get(&id)
        .is_some_and(|definition| definition.terrain_tools.contains(tool))
        || !catalog.0.terrain_tools.contains_key(tool)
    {
        return Err(RejectReason::MissingCapability);
    }
    Ok(())
}
pub(crate) fn order_changed(world: &mut World, unit: u32) {
    super::pause::cancel(world, unit);
    if let Some(mut runtime) = world.get_resource_mut::<TerrainWorkRuntime>() {
        runtime.units.remove(&unit);
        runtime.claims.retain(|_, owner| *owner != unit);
    }
}
/// Capture authority when an order starts, before a later ownership change.
pub(crate) fn begin_order(world: &mut World, unit: u32, order: &Order) {
    if super::pause::consume_resume(world, unit) {
        return;
    }
    let Some(owner) = unit_entity(world, unit)
        .and_then(|entity| world.get::<Owner>(entity))
        .map(|owner| owner.0)
    else {
        return;
    };
    store(
        world,
        unit,
        WorkState {
            owner,
            order: order.clone(),
            cell: None,
            done: 0,
            carry: 0,
            banked: false,
            required: None,
            wait: TerrainWorkWait::Unknown,
            source_revision: None,
        },
    );
}
fn store(world: &mut World, unit: u32, state: WorkState) {
    world.init_resource::<TerrainWorkRuntime>();
    world
        .resource_mut::<TerrainWorkRuntime>()
        .units
        .insert(unit, state);
}
fn wait(reason: StepWait) -> TerrainWorkWait {
    match reason {
        StepWait::Unknown => TerrainWorkWait::Unknown,
        StepWait::OutOfReach => TerrainWorkWait::Approaching,
        StepWait::Blocked => TerrainWorkWait::Blocked,
        StepWait::Disabled => TerrainWorkWait::Disabled,
        StepWait::Material => TerrainWorkWait::Material,
        StepWait::Floor => TerrainWorkWait::Floor,
        StepWait::Ceiling => TerrainWorkWait::Ceiling,
        StepWait::Invalid => TerrainWorkWait::Invalid,
    }
}
fn account_wait(reason: RejectReason) -> TerrainWorkWait {
    match reason {
        RejectReason::InsufficientBalance | RejectReason::InsufficientItems => {
            TerrainWorkWait::MissingInputs
        }
        RejectReason::RefundDestinationFull | RejectReason::RefundDestinationOverflow => {
            TerrainWorkWait::CargoFull
        }
        _ => TerrainWorkWait::Invalid,
    }
}
fn selected_cell(world: &World, unit: u32, order: &Order) -> Option<Cell> {
    let OrderTargetSim::TerrainWork {
        tool,
        cells,
        target_elevation,
    } = &order.target
    else {
        return None;
    };
    let entity = unit_entity(world, unit)?;
    let owner = world.get::<Owner>(entity)?.0.0;
    let position = world.get::<Position>(entity)?.0;
    let terrain = world.get_resource::<TerrainState>()?;
    let operation = &world
        .get_resource::<InstalledCatalog>()?
        .0
        .terrain_tools
        .get(tool)?
        .operation;
    // A paused designation resumes its current prism even if unloading moved the actor
    // closer to another cell. Physical position must not erase earned partial work.
    if let Some(cell) = world
        .get_resource::<TerrainWorkRuntime>()
        .and_then(|runtime| runtime.units.get(&unit))
        .filter(|state| state.order == *order)
        .and_then(|state| state.cell)
    {
        let cell = Cell {
            x: cell.x,
            y: cell.y,
        };
        if !step::observed(world, owner, terrain, cell, ObservationPhase::Tactical) {
            return Some(cell);
        }
        let height = terrain.elevation(cell)?;
        let finished = match operation {
            turts_mod_api::ResolvedTerrainOperation::Excavate => height <= *target_elevation,
            turts_mod_api::ResolvedTerrainOperation::Fill { .. } => height >= *target_elevation,
        };
        if !finished {
            return Some(cell);
        }
    }
    let mut cells_ranked = Vec::new();
    let width = f64::from(terrain.grid.cell_width) / f64::from(terrain.grid.length_scale);
    for y in cells.min.y..cells.max.y {
        for x in cells.min.x..cells.max.x {
            let cell = Cell { x, y };
            // Unknown cells are never dropped because of their hidden surface or geology.
            if step::observed(world, owner, terrain, cell, ObservationPhase::Tactical) {
                let height = terrain.elevation(cell)?;
                let done = match operation {
                    turts_mod_api::ResolvedTerrainOperation::Excavate => {
                        height <= *target_elevation
                    }
                    turts_mod_api::ResolvedTerrainOperation::Fill { .. } => {
                        height >= *target_elevation
                    }
                };
                if done {
                    continue;
                }
            }
            let dx = f64::from(position.x) - (f64::from(x) + 0.5) * width;
            let dy = f64::from(position.y) - (f64::from(y) + 0.5) * width;
            cells_ranked.push((dx * dx + dy * dy, x, y));
        }
    }
    cells_ranked.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    cells_ranked.first().map(|(_, x, y)| Cell { x: *x, y: *y })
}
fn approach(world: &mut World, unit: u32, cell: Cell) {
    let Some(entity) = unit_entity(world, unit) else {
        return;
    };
    let Some(position) = world.get::<Position>(entity).map(|position| position.0) else {
        return;
    };
    let Some(kind) = world.get::<UnitType>(entity) else {
        return;
    };
    let Some(radius) = world
        .get_resource::<crate::UnitTypes>()
        .and_then(|types| types.get(kind.0))
        .map(|entry| entry.radius)
    else {
        return;
    };
    let Some(terrain) = world.get_resource::<TerrainState>() else {
        return;
    };
    let width = f64::from(terrain.grid.cell_width) / f64::from(terrain.grid.length_scale);
    let cx = (f64::from(cell.x) + 0.5) * width;
    let cy = (f64::from(cell.y) + 0.5) * width;
    let dx = f64::from(position.x) - cx;
    let dy = f64::from(position.y) - cy;
    // A tangent circle does not occupy the work cell. Move beside it, never onto it.
    let point = if dx.abs() >= dy.abs() {
        bevy_math::Vec2::new(
            (cx + if dx <= 0.0 {
                -width * 0.5 - f64::from(radius)
            } else {
                width * 0.5 + f64::from(radius)
            }) as f32,
            f64::from(position.y).clamp(cy - width * 0.5, cy + width * 0.5) as f32,
        )
    } else {
        bevy_math::Vec2::new(
            f64::from(position.x).clamp(cx - width * 0.5, cx + width * 0.5) as f32,
            (cy + if dy <= 0.0 {
                -width * 0.5 - f64::from(radius)
            } else {
                width * 0.5 + f64::from(radius)
            }) as f32,
        )
    };
    world
        .entity_mut(entity)
        .insert(crate::world::MoveTarget(point));
}
/// After Acquire and before Repair/Combat. Existing claimants retain their cell until completion
/// or loss of physical eligibility. New equal physical starts remain contested.
pub(crate) fn run(world: &mut World) {
    if world.get_resource::<TerrainState>().is_none() {
        return;
    }
    world.init_resource::<TerrainWorkRuntime>();
    // A suspended order retains its one progress row and claim, but earns no work.
    let invalid = world
        .resource::<TerrainWorkRuntime>()
        .units
        .iter()
        .filter_map(|(unit, state)| {
            let owner = unit_entity(world, *unit)
                .and_then(|entity| world.get::<Owner>(entity))
                .map(|owner| owner.0);
            let dead = world
                .get_resource::<crate::economy::combat::Healths>()
                .and_then(|health| health.0.get(unit))
                .is_some_and(|health| *health == 0);
            (owner != Some(state.owner)
                || dead
                || (super::pause::has_record(world, *unit)
                    && !super::pause::is_suspended(world, *unit)
                    && !super::pause::has_pending_resume(world, *unit)))
            .then_some((*unit, state.owner))
        })
        .collect::<Vec<_>>();
    for (unit, previous_owner) in invalid {
        if let Some(entity) = unit_entity(world, unit) {
            crate::economy::transaction::capture_post_movement_order_operation(
                world,
                unit,
                |world| {
                    crate::orders::clear_replaced_movement(world, previous_owner, entity, unit);
                    crate::orders::writer::edit_unit_orders(world, entity, |orders| {
                        orders.current = None;
                        orders.queue.clear();
                    });
                },
            );
        } else {
            order_changed(world, unit);
        }
    }
    let orders = world
        .iter_entities()
        .filter_map(|entity| {
            let unit = entity.get::<UnitId>()?.0;
            let order = entity.get::<UnitOrders>()?.current.as_ref()?;
            (order.behaviour == Behaviour::TerrainWork).then(|| (unit, order.clone()))
        })
        .collect::<Vec<_>>();
    let mut retained = orders
        .iter()
        .map(|(unit, _)| *unit)
        .collect::<BTreeSet<_>>();
    retained.extend(
        world
            .resource::<TerrainWorkRuntime>()
            .units
            .keys()
            .filter(|unit| {
                super::pause::is_suspended(world, **unit)
                    || super::pause::has_pending_resume(world, **unit)
            })
            .copied(),
    );
    {
        let mut runtime = world.resource_mut::<TerrainWorkRuntime>();
        runtime.units.retain(|unit, _| retained.contains(unit));
        runtime.claims.retain(|_, unit| retained.contains(unit));
    }
    let mut candidates = Vec::new();
    if orders.is_empty() {
        return;
    }
    let mut occupancy = step::Occupancy::capture(world);
    for (unit, order) in orders {
        let Some(cell) = selected_cell(world, unit, &order) else {
            order_changed(world, unit);
            crate::orders::finish_terrain_order(world, unit);
            occupancy = step::Occupancy::capture(world);
            continue;
        };
        let mut state = world
            .resource::<TerrainWorkRuntime>()
            .units
            .get(&unit)
            .filter(|state| state.order == order)
            .cloned()
            .unwrap_or(WorkState {
                owner: world
                    .get::<Owner>(unit_entity(world, unit).unwrap())
                    .unwrap()
                    .0,
                order: order.clone(),
                cell: None,
                done: 0,
                carry: 0,
                banked: false,
                required: None,
                wait: TerrainWorkWait::Unknown,
                source_revision: None,
            });
        let api_cell = Some(step::api_cell(cell));
        if state.cell != api_cell {
            state.cell = api_cell;
            if !state.banked {
                state.done = 0;
                state.carry = 0;
            }
            state.required = None;
            state.source_revision = None;
            world
                .resource_mut::<TerrainWorkRuntime>()
                .claims
                .retain(|_, owner| *owner != unit);
        }
        let OrderTargetSim::TerrainWork { tool, .. } = &order.target else {
            continue;
        };
        match step::prepare_step(world, unit, cell, tool, &occupancy) {
            Ok(plan) => {
                if state.source_revision != Some(plan.column.expected_revision) {
                    if !state.banked {
                        state.done = 0;
                        state.carry = 0;
                    }
                    state.banked = false;
                    state.source_revision = Some(plan.column.expected_revision);
                }
                state.required = Some(plan.required_work);
                if let Err(reason) = step::accounts_ready(world, unit, &plan) {
                    state.wait = account_wait(reason);
                    store(world, unit, state);
                    continue;
                }
                state.wait = TerrainWorkWait::Working;
                let entity = unit_entity(world, unit).expect("prepared live work actor");
                world
                    .entity_mut(entity)
                    .remove::<crate::world::MoveTarget>();
                let position = world
                    .get::<Position>(entity)
                    .expect("prepared actor position")
                    .0;
                let kind = world.get::<UnitType>(entity).expect("prepared actor type");
                let content = world
                    .resource::<crate::UnitTypes>()
                    .get(kind.0)
                    .and_then(|entry| ContentId::parse(&entry.content_id).ok())
                    .expect("validated actor content");
                let terrain = world.resource::<TerrainState>();
                let width =
                    f64::from(terrain.grid.cell_width) / f64::from(terrain.grid.length_scale);
                let dx = f64::from(position.x) - (f64::from(cell.x) + 0.5) * width;
                let dy = f64::from(position.y) - (f64::from(cell.y) + 0.5) * width;
                candidates.push(Candidate {
                    unit,
                    cell,
                    distance: dx * dx + dy * dy,
                    x: zero(position.x),
                    y: zero(position.y),
                    content,
                });
            }
            Err(reason) => {
                if matches!(
                    reason,
                    StepWait::Unknown | StepWait::OutOfReach | StepWait::Blocked
                ) {
                    approach(world, unit, cell);
                }
                if matches!(reason, StepWait::Unknown) {
                    state.required = None;
                }
                if matches!(
                    reason,
                    StepWait::Unknown | StepWait::OutOfReach | StepWait::Invalid
                ) {
                    world
                        .resource_mut::<TerrainWorkRuntime>()
                        .claims
                        .retain(|_, owner| *owner != unit);
                }
                state.wait = wait(reason);
            }
        }
        store(world, unit, state);
    }
    candidates.sort_by(|left, right| left.cell.cmp(&right.cell).then_with(|| rank(left, right)));
    for index in 0..candidates.len() {
        let candidate = &candidates[index];
        let owner = world
            .resource::<TerrainWorkRuntime>()
            .claims
            .get(&candidate.cell)
            .copied();
        let wins = if let Some(owner) = owner {
            owner == candidate.unit
        } else {
            let prior = index.checked_sub(1).and_then(|index| candidates.get(index));
            let next = candidates.get(index + 1);
            let tied =
                |other: &Candidate| other.cell == candidate.cell && rank(candidate, other).is_eq();
            !prior.is_some_and(|other| other.cell == candidate.cell) && !next.is_some_and(tied)
        };
        if !wins {
            if let Some(state) = world
                .resource_mut::<TerrainWorkRuntime>()
                .units
                .get_mut(&candidate.unit)
            {
                state.wait = TerrainWorkWait::Contested;
            }
            continue;
        }
        world
            .resource_mut::<TerrainWorkRuntime>()
            .claims
            .entry(candidate.cell)
            .or_insert(candidate.unit);
        work_one_actor(world, candidate.unit, candidate.cell, &occupancy);
    }
}
fn work_one_actor(world: &mut World, unit: u32, cell: Cell, occupancy: &step::Occupancy) {
    let Some(order) = current(world, unit) else {
        return;
    };
    let OrderTargetSim::TerrainWork {
        tool,
        target_elevation,
        ..
    } = &order.target
    else {
        return;
    };
    let Some(rate) = world
        .get_resource::<InstalledCatalog>()
        .and_then(|catalog| catalog.0.terrain_tools.get(tool))
        .map(|tool| tool.work_per_second)
    else {
        return;
    };
    let hz = u64::from(world.resource::<SimClock>().tick_hz);
    let Some(mut state) = world
        .resource::<TerrainWorkRuntime>()
        .units
        .get(&unit)
        .cloned()
    else {
        return;
    };
    let earned = u128::from(rate) + u128::from(state.carry);
    let Some(done) = u64::try_from(earned / u128::from(hz))
        .ok()
        .and_then(|increment| state.done.checked_add(increment))
    else {
        state.wait = TerrainWorkWait::Invalid;
        store(world, unit, state);
        return;
    };
    state.done = done;
    state.carry = (earned % u128::from(hz)) as u64;
    // Bound homogeneous-run batches, not prism count. A batch can consume all earned work
    // within the exposed run. Crossing a material boundary rechecks the new exposure. If
    // the service budget ends, exact unpaid integer work and fractional carry remain banked;
    // no hidden complexity changes command admission or erases accrued work.
    for _ in 0..32 {
        let plan = match step::prepare_step(world, unit, cell, tool, occupancy) {
            Ok(plan) => plan,
            Err(reason) => {
                state.wait = wait(reason);
                store(world, unit, state);
                return;
            }
        };
        if let Err(reason) = step::accounts_ready(world, unit, &plan) {
            state.wait = account_wait(reason);
            store(world, unit, state);
            return;
        }
        state.required = Some(plan.required_work);
        if state.done < plan.required_work {
            store(world, unit, state);
            return;
        }
        let remaining =
            u32::try_from((i64::from(plan.before) - i64::from(*target_elevation)).unsigned_abs())
                .unwrap_or(u32::MAX);
        let earned_steps = u32::try_from(state.done / plan.required_work).unwrap_or(u32::MAX);
        let requested = plan.max_steps.min(remaining).min(earned_steps);
        let count = step::affordable_steps(world, unit, &plan, requested);
        if count == 0 {
            state.wait = TerrainWorkWait::MissingInputs;
            store(world, unit, state);
            return;
        }
        let plan = if count == 1 {
            plan
        } else {
            match step::prepare_steps(world, unit, cell, tool, count, occupancy) {
                Ok(plan) => plan,
                Err(reason) => {
                    state.wait = wait(reason);
                    store(world, unit, state);
                    return;
                }
            }
        };
        let before_payment = state.clone();
        state.done -= plan.required_work;
        state.banked = true;
        state.source_revision = Some(plan.column.column.revision());
        state.wait = TerrainWorkWait::Working;
        let reached = match &world
            .resource::<InstalledCatalog>()
            .0
            .terrain_tools
            .get(tool)
            .expect("validated tool")
            .operation
        {
            turts_mod_api::ResolvedTerrainOperation::Excavate => plan.after <= *target_elevation,
            turts_mod_api::ResolvedTerrainOperation::Fill { .. } => plan.after >= *target_elevation,
        };
        if let Err(reason) = step::commit_step(world, unit, plan, state.clone()) {
            state = before_payment;
            state.wait = account_wait(reason);
            store(world, unit, state);
            return;
        }
        if reached {
            world
                .resource_mut::<TerrainWorkRuntime>()
                .claims
                .remove(&cell);
            state.cell = None;
            state.required = None;
            state.source_revision = None;
            store(world, unit, state);
            return;
        }
    }
    state.wait = TerrainWorkWait::Budget;
    store(world, unit, state);
}

/// Owner-private work display. Unknown future cells publish no material-dependent work value.
pub fn rows_for(world: &World, owner: u8) -> Vec<TerrainWorkView> {
    let Some(runtime) = world.get_resource::<TerrainWorkRuntime>() else {
        return Vec::new();
    };
    runtime
        .units
        .iter()
        .filter_map(|(&unit, state)| {
            let entity = unit_entity(world, unit)?;
            if world.get::<Owner>(entity)?.0.0 != owner {
                return None;
            }
            let OrderTargetSim::TerrainWork {
                tool,
                cells,
                target_elevation,
            } = &state.order.target
            else {
                return None;
            };
            Some(TerrainWorkView {
                unit,
                owner_slot: owner,
                tool: tool.clone(),
                cells: *cells,
                target_elevation: *target_elevation,
                current_cell: state.cell,
                completed_work: state
                    .required
                    .map_or(0, |required| state.done.min(required)),
                required_work: state.required,
                wait: if super::pause::is_suspended(world, unit) {
                    TerrainWorkWait::Unloading
                } else {
                    state.wait.clone()
                },
            })
        })
        .collect()
}
