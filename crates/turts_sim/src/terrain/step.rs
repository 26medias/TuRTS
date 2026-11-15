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
    TerrainOutputDestination,
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
    Invalid,
}
pub(crate) struct StepPlan {
    pub(crate) owner: u8,
    pub(crate) required_work: u64,
    pub(crate) max_steps: u32,
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
        numerator: u64::from(ratio.numerator),
        denominator: u64::from(ratio.denominator),
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
    let grid = basis.grid();
    let cells = u64::from(grid.columns) * u64::from(grid.rows);
    let Some(limit) = usize::try_from(cells)
        .ok()
        .and_then(|cells| cells.checked_mul(2))
        .and_then(|cells| cells.checked_add(16))
    else {
        return false;
    };
    let mut budget = Budget::new(limit);
    basis
        .terrain_cell_observed(owner, &state.grid, cell, &mut budget)
        .unwrap_or(false)
}
/// Exact body coverage captured after movement. Work edits do not move bodies.
/// Refresh this value whenever queue promotion can create or remove a body.
pub(crate) struct Occupancy {
    cells: std::collections::BTreeSet<Cell>,
    blocked_all: bool,
}
impl Occupancy {
    fn add_body(&mut self, state: &TerrainState, point: bevy_math::Vec2, radius: f32) {
        let support = state.grid.disk_support(
            &|_| Some(state.grid.floor),
            Point {
                x: f64::from(point.x),
                y: f64::from(point.y),
            },
            f64::from(radius),
            0,
            &mut Budget::new(4096),
        );
        match support {
            Ok(support) => self.cells.extend(support.cells),
            Err(_) => self.blocked_all = true,
        }
    }
    pub(crate) fn capture(world: &World) -> Self {
        let mut result = Self {
            cells: Default::default(),
            blocked_all: false,
        };
        let (Some(state), Some(types)) = (
            world.get_resource::<TerrainState>(),
            world.get_resource::<UnitTypes>(),
        ) else {
            result.blocked_all = true;
            return result;
        };
        for entity in world.iter_entities() {
            let (Some(position), Some(kind)) = (entity.get::<Position>(), entity.get::<UnitType>())
            else {
                continue;
            };
            let Some(definition) = types.get(kind.0) else {
                result.blocked_all = true;
                return result;
            };
            result.add_body(state, position.0, definition.radius);
        }
        if let (Some(nodes), Some(catalog)) = (
            world.get_resource::<crate::SurfaceNodes>(),
            world.get_resource::<InstalledCatalog>(),
        ) {
            for node in &nodes.0 {
                let Some(definition) = catalog.0.nodes.get(&node.content_id) else {
                    result.blocked_all = true;
                    return result;
                };
                result.add_body(state, node.position.0, definition.radius);
            }
        }
        result
    }
    pub(crate) fn blocks(&self, cell: Cell) -> bool {
        self.blocked_all || self.cells.contains(&cell)
    }
}
/// Check observation before looking at hidden material, hardness, work or blockers.
pub(crate) fn prepare_step(
    world: &World,
    actor: u32,
    cell: Cell,
    tool_id: &ContentId,
    occupancy: &Occupancy,
) -> Result<StepPlan, StepWait> {
    prepare_steps(world, actor, cell, tool_id, 1, occupancy)
}
pub(crate) fn prepare_steps(
    world: &World,
    actor: u32,
    cell: Cell,
    tool_id: &ContentId,
    steps: u32,
    occupancy: &Occupancy,
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
    if occupancy.blocks(cell) {
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
                .stage_remove_many(state.prism, terms, u64::MAX, steps)
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
                    .stage_fill_many(
                        state.key(material).ok_or(StepWait::Invalid)?,
                        state.prism,
                        terms,
                        u64::MAX,
                        steps,
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
                    .and_then(|amount| {
                        amount
                            .checked_mul(u64::from(steps))
                            .ok_or(turts_rules::terrain::material::Error::ArithmeticOverflow)
                    })
                    .map(|amount| (rate.resource.clone(), amount))
                    .map_err(|_| StepWait::Invalid)
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let max_steps = match tool.operation {
        ResolvedTerrainOperation::Excavate => original.exposed_run_steps(),
        ResolvedTerrainOperation::Fill { .. } => {
            u32::try_from(i64::from(state.grid.ceiling) - i64::from(original.surface()))
                .map_err(|_| StepWait::Invalid)?
        }
    };
    Ok(StepPlan {
        owner,
        max_steps,
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

fn stage_accounts(
    tx: &mut transaction::Transaction<'_>,
    actor: u32,
    plan: &StepPlan,
) -> Result<(), RejectReason> {
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
    Ok(())
}
pub(crate) fn affordable_steps(
    world: &World,
    actor: u32,
    single: &StepPlan,
    requested: u32,
) -> u32 {
    use crate::economy::ledger::EconomyLedger;
    let Some(ledger) = world.get_resource::<EconomyLedger>() else {
        return 0;
    };
    let Some(catalog) = world.get_resource::<InstalledCatalog>() else {
        return 0;
    };
    let mut count = u64::from(requested);
    for (resource, amount) in &single.inputs {
        if *amount == 0 {
            continue;
        }
        let account = match catalog
            .0
            .resources
            .get(resource)
            .map(|resource| resource.kind)
        {
            Some(ResourceKind::Balance) => ledger.balances.get(&(single.owner, resource.clone())),
            Some(ResourceKind::Item) => ledger
                .inventories
                .get(&actor)
                .and_then(|inventory| inventory.items.get(resource)),
            _ => return 0,
        };
        let available = account
            .and_then(|account| account.on_hand.checked_sub(account.reserved))
            .unwrap_or(0);
        count = count.min(available / amount);
    }
    let mut inventory_total = 0u128;
    for (resource, amount) in &single.outputs {
        if *amount == 0 {
            continue;
        }
        let account = match single.destination {
            TerrainOutputDestination::SlotBalances => {
                ledger.balances.get(&(single.owner, resource.clone()))
            }
            TerrainOutputDestination::Inventory => ledger
                .inventories
                .get(&actor)
                .and_then(|inventory| inventory.items.get(resource)),
        };
        count = count.min((u64::MAX - account.map_or(0, |account| account.on_hand)) / amount);
        if single.destination == TerrainOutputDestination::Inventory {
            let Some(total) = inventory_total.checked_add(u128::from(*amount)) else {
                return 0;
            };
            inventory_total = total;
        }
    }
    if inventory_total > 0 {
        let Some(inventory) = ledger.inventories.get(&actor) else {
            return 0;
        };
        let Some(used) = inventory.items.values().try_fold(0u128, |sum, account| {
            sum.checked_add(u128::from(account.on_hand))
        }) else {
            return 0;
        };
        let remaining = u128::from(inventory.capacity).saturating_sub(used);
        count = count.min(u64::try_from(remaining / inventory_total).unwrap_or(u64::MAX));
    }
    u32::try_from(count).unwrap_or(0)
}
pub(crate) fn accounts_ready(
    world: &mut World,
    actor: u32,
    plan: &StepPlan,
) -> Result<(), RejectReason> {
    transaction::validate_only(world, |tx| stage_accounts(tx, actor, plan))
}
pub(crate) fn commit_step(
    world: &mut World,
    actor: u32,
    plan: StepPlan,
    state: super::work::WorkState,
) -> Result<(), RejectReason> {
    transaction::transact(world, |tx| {
        stage_accounts(tx, actor, &plan)?;
        let inputs = plan
            .inputs
            .iter()
            .filter(|(_, amount)| *amount > 0)
            .map(|(resource, amount)| {
                let kind = tx
                    .world()
                    .resource::<InstalledCatalog>()
                    .0
                    .resources
                    .get(resource)
                    .ok_or(RejectReason::IntegrityViolation)?
                    .kind;
                let source = match kind {
                    ResourceKind::Item => crate::EventSource::Inventory(actor),
                    ResourceKind::Balance => crate::EventSource::Slot(plan.owner),
                    ResourceKind::Supply { .. } => return Err(RejectReason::IncompatibleItem),
                };
                Ok(crate::CostLine {
                    source,
                    resource: resource.clone(),
                    amount: *amount,
                })
            })
            .collect::<Result<Vec<_>, RejectReason>>()?;
        let outputs = plan
            .outputs
            .iter()
            .filter(|(_, amount)| *amount > 0)
            .map(|(resource, amount)| crate::CostLine {
                source: match plan.destination {
                    TerrainOutputDestination::Inventory => crate::EventSource::Inventory(actor),
                    TerrainOutputDestination::SlotBalances => crate::EventSource::Slot(plan.owner),
                },
                resource: resource.clone(),
                amount: *amount,
            })
            .collect();
        tx.stage_event(
            plan.owner,
            crate::EconomyEventKind::TerrainStepCommitted {
                unit: actor,
                cell: api_cell(plan.column.cell),
                before: plan.before,
                after: plan.after,
                material: plan.material.clone(),
                placed: plan.placed,
                inputs,
                outputs,
            },
        )?;
        tx.stage_terrain_work(actor, state);
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
