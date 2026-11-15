//! One checked terrain step. Progress/claims call this only after accumulating required work.
use super::{PreparedCellEdit, TerrainState};
use crate::{
    Owner, Position, RejectReason, UnitType,
    bootstrap::InstalledCatalog,
    economy::{ledger::Endpoint, transaction},
    registry::UnitTypes,
    unit_index::unit_entity,
};
use bevy_ecs::world::World;
use turts_mod_api::{
    ContentId, ObservationPhase, ResolvedTerrainOperation, ResourceKind, TerrainCell,
    TerrainOutputDestination, WorldPos,
};
use turts_rules::terrain::{
    geometry::{Budget, Cell, Point},
    material::{Provenance, StepTerms},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StepWait {
    Unknown,
    OutOfReach,
    Blocked,
    Disabled,
    Material,
    Floor,
    Ceiling,
    Funds,
    CargoFull,
    Invalid,
}
pub(crate) struct StepPlan {
    pub(crate) owner: u8,
    pub(crate) required_work: u64,
    pub(crate) material: ContentId,
    pub(crate) placed: bool,
    pub(crate) before: i32,
    pub(crate) after: i32,
    pub(crate) column: PreparedCellEdit,
    pub(crate) inputs: Vec<(ContentId, u64)>,
    pub(crate) outputs: Vec<(ContentId, u64)>,
    pub(crate) destination: TerrainOutputDestination,
}
fn ratio(ratio: turts_mod_api::Ratio) -> turts_rules::terrain::Ratio {
    turts_rules::terrain::Ratio {
        numerator: ratio.numerator,
        denominator: ratio.denominator,
    }
}
fn outward(value: f64, upper: bool) -> f32 {
    let rounded = value as f32;
    if upper && f64::from(rounded) < value {
        rounded.next_up()
    } else if !upper && f64::from(rounded) > value {
        rounded.next_down()
    } else {
        rounded
    }
}
pub(crate) fn observed(
    world: &World,
    owner: u8,
    state: &TerrainState,
    cell: Cell,
    phase: ObservationPhase,
) -> bool {
    let Some(basis) = crate::visibility::basis(world, phase) else {
        return true;
    };
    let width = f64::from(state.grid.cell_width) / f64::from(state.grid.length_scale);
    let min = WorldPos {
        x: outward(f64::from(cell.x) * width, false),
        y: outward(f64::from(cell.y) * width, false),
    };
    let max = WorldPos {
        x: outward((f64::from(cell.x) + 1.0) * width, true),
        y: outward((f64::from(cell.y) + 1.0) * width, true),
    };
    basis.area_observed(owner, min, max)
}
fn body_blocks(state: &TerrainState, cell: Cell, point: bevy_math::Vec2, radius: f32) -> bool {
    let mut budget = Budget::new(4096);
    state
        .grid
        .disk_support(
            &|_| Some(state.grid.floor),
            Point {
                x: f64::from(point.x),
                y: f64::from(point.y),
            },
            f64::from(radius),
            0,
            &mut budget,
        )
        .map_or(true, |support| support.cells.contains(&cell))
}
fn occupied(world: &World, state: &TerrainState, cell: Cell) -> bool {
    let Some(types) = world.get_resource::<UnitTypes>() else {
        return true;
    };
    for entity in world.iter_entities() {
        let (Some(position), Some(kind)) = (entity.get::<Position>(), entity.get::<UnitType>())
        else {
            continue;
        };
        let Some(definition) = types.get(kind.0) else {
            return true;
        };
        if body_blocks(state, cell, position.0, definition.radius) {
            return true;
        }
    }
    if let (Some(nodes), Some(catalog)) = (
        world.get_resource::<crate::SurfaceNodes>(),
        world.get_resource::<InstalledCatalog>(),
    ) {
        for node in &nodes.0 {
            let Some(definition) = catalog.0.nodes.get(&node.content_id) else {
                return true;
            };
            if body_blocks(state, cell, node.position.0, definition.radius) {
                return true;
            }
        }
    }
    false
}
/// Check observation before looking at hidden material, hardness, work or blockers.
pub(crate) fn prepare_step(
    world: &World,
    actor: u32,
    cell: Cell,
    tool_id: &ContentId,
) -> Result<StepPlan, StepWait> {
    let state = world
        .get_resource::<TerrainState>()
        .ok_or(StepWait::Invalid)?;
    let index = state.index(cell).ok_or(StepWait::Invalid)?;
    let entity = unit_entity(world, actor).ok_or(StepWait::Invalid)?;
    let owner = world.get::<Owner>(entity).ok_or(StepWait::Invalid)?.0.0;
    if !observed(world, owner, state, cell, ObservationPhase::Tactical) {
        return Err(StepWait::Unknown);
    }
    let kind = world.get::<UnitType>(entity).ok_or(StepWait::Invalid)?;
    let entry = world
        .get_resource::<UnitTypes>()
        .and_then(|types| types.get(kind.0))
        .ok_or(StepWait::Invalid)?;
    let actor_id = ContentId::parse(&entry.content_id).map_err(|_| StepWait::Invalid)?;
    let catalog = world
        .get_resource::<InstalledCatalog>()
        .ok_or(StepWait::Invalid)?;
    let definition = catalog.0.units.get(&actor_id).ok_or(StepWait::Invalid)?;
    if !definition.terrain_tools.contains(tool_id) {
        return Err(StepWait::Invalid);
    }
    if world
        .get_resource::<crate::economy::combat::Healths>()
        .and_then(|healths| healths.0.get(&actor))
        .is_some_and(|health| *health == 0)
    {
        return Err(StepWait::Invalid);
    }
    if world
        .get_resource::<crate::economy::research::Disabled>()
        .is_some_and(|disabled| disabled.0.contains(&actor))
    {
        return Err(StepWait::Disabled);
    }
    let tool = catalog
        .0
        .terrain_tools
        .get(tool_id)
        .ok_or(StepWait::Invalid)?;
    let position = world.get::<Position>(entity).ok_or(StepWait::Invalid)?.0;
    let width = f64::from(state.grid.cell_width) / f64::from(state.grid.length_scale);
    let nearest_x =
        f64::from(position.x).clamp(f64::from(cell.x) * width, (f64::from(cell.x) + 1.0) * width);
    let nearest_y =
        f64::from(position.y).clamp(f64::from(cell.y) * width, (f64::from(cell.y) + 1.0) * width);
    let dx = f64::from(position.x) - nearest_x;
    let dy = f64::from(position.y) - nearest_y;
    let reach = f64::from(tool.reach) + f64::from(entry.radius);
    if dx * dx + dy * dy > reach * reach {
        return Err(StepWait::OutOfReach);
    }
    if occupied(world, state, cell) {
        return Err(StepWait::Blocked);
    }
    let original = &state.columns[index];
    let (material_id, placed) = match &tool.operation {
        ResolvedTerrainOperation::Excavate => {
            let exposed = original.exposed().ok_or(StepWait::Floor)?;
            (
                state.material(exposed.material).ok_or(StepWait::Invalid)?,
                exposed.provenance == Provenance::NoYield,
            )
        }
        ResolvedTerrainOperation::Fill { material } => (material, true),
    };
    let material = catalog
        .0
        .terrain_materials
        .get(material_id)
        .ok_or(StepWait::Invalid)?;
    if !tool.allowed_materials.contains(material_id) || material.hardness > tool.max_hardness {
        return Err(StepWait::Material);
    }
    let terms = StepTerms {
        material: turts_rules::terrain::Ratio {
            numerator: 0,
            denominator: 1,
        },
        work: ratio(material.work_per_world_cube),
    };
    let mut column = original.clone();
    let (step, input_rates, output_rates) = match &tool.operation {
        ResolvedTerrainOperation::Excavate => (
            column
                .stage_remove(state.prism, terms, u64::MAX)
                .map_err(|_| StepWait::Invalid)?,
            &[][..],
            if placed {
                &[][..]
            } else {
                material.natural_outputs.as_slice()
            },
        ),
        ResolvedTerrainOperation::Fill { material } => {
            if original.surface() == state.grid.ceiling {
                return Err(StepWait::Ceiling);
            }
            (
                column
                    .stage_fill(
                        state.key(material).ok_or(StepWait::Invalid)?,
                        state.prism,
                        terms,
                        u64::MAX,
                    )
                    .map_err(|_| StepWait::Invalid)?,
                material_inputs(material, catalog)?,
                &[][..],
            )
        }
    };
    let effect = column.commit(step).map_err(|_| StepWait::Invalid)?;
    let quantities = |rates: &[turts_mod_api::ResolvedTerrainResourceRate]| {
        rates
            .iter()
            .map(|rate| {
                state
                    .prism
                    .amount(ratio(rate.per_world_cube), u64::MAX)
                    .map(|amount| (rate.resource.clone(), amount))
                    .map_err(|_| StepWait::Invalid)
            })
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(StepPlan {
        owner,
        required_work: effect.required_work,
        material: material_id.clone(),
        placed,
        before: effect.before,
        after: effect.after,
        column: state
            .prepare_cell(cell, column)
            .map_err(|_| StepWait::Invalid)?,
        inputs: quantities(input_rates)?,
        outputs: quantities(output_rates)?,
        destination: tool.output_destination,
    })
}
fn material_inputs<'a>(
    id: &ContentId,
    catalog: &'a InstalledCatalog,
) -> Result<&'a [turts_mod_api::ResolvedTerrainResourceRate], StepWait> {
    catalog
        .0
        .terrain_materials
        .get(id)
        .map(|material| material.fill_inputs.as_slice())
        .ok_or(StepWait::Invalid)
}

pub(crate) fn commit_step(
    world: &mut World,
    actor: u32,
    plan: StepPlan,
) -> Result<(), RejectReason> {
    transaction::transact(world, |tx| {
        for (resource, amount) in &plan.inputs {
            if *amount == 0 {
                continue;
            }
            let kind = tx
                .world()
                .get_resource::<InstalledCatalog>()
                .and_then(|catalog| catalog.0.resources.get(resource))
                .map(|resource| resource.kind)
                .ok_or(RejectReason::IntegrityViolation)?;
            let endpoint = match kind {
                ResourceKind::Item => Endpoint::Inventory(actor),
                ResourceKind::Balance => Endpoint::Slot(plan.owner),
                ResourceKind::Supply { .. } => return Err(RejectReason::IncompatibleItem),
            };
            tx.validate_endpoint(endpoint, resource)?;
            let mut account = tx.account(endpoint, resource);
            let available = account
                .on_hand
                .checked_sub(account.reserved)
                .ok_or(RejectReason::IntegrityViolation)?;
            if available < *amount {
                return Err(if kind == ResourceKind::Balance {
                    RejectReason::InsufficientBalance
                } else {
                    RejectReason::InsufficientItems
                });
            }
            account.on_hand -= *amount;
            tx.stage_account(endpoint, resource.clone(), account);
        }
        for (resource, amount) in &plan.outputs {
            if *amount == 0 {
                continue;
            }
            let endpoint = match plan.destination {
                TerrainOutputDestination::Inventory => Endpoint::Inventory(actor),
                TerrainOutputDestination::SlotBalances => Endpoint::Slot(plan.owner),
            };
            tx.validate_endpoint(endpoint, resource)?;
            let mut account = tx.account(endpoint, resource);
            account.on_hand = account
                .on_hand
                .checked_add(*amount)
                .ok_or(RejectReason::RefundDestinationOverflow)?;
            tx.stage_account(endpoint, resource.clone(), account);
            if let Endpoint::Inventory(_) = endpoint {
                let capacity = tx.inventory(endpoint)?.capacity;
                if tx.inventory_used(endpoint)? > u128::from(capacity) {
                    return Err(RejectReason::RefundDestinationFull);
                }
            }
        }
        tx.stage_terrain_cell(plan.column)?;
        Ok(())
    })
}

pub(crate) fn api_cell(cell: Cell) -> TerrainCell {
    TerrainCell {
        x: cell.x,
        y: cell.y,
    }
}
