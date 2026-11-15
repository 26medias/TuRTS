//! Independent terrain conservation. This module never calls simulation terrain/payment code.
//! The Initial snapshot is the trusted starting geology; private seed provenance is a
//! separate host/replay input. Sight authorization is tested by the knowledge provider.
//! Existing movement validation retains exact equality and does not certify an unmatched
//! terrain detour as flat movement.
use crate::EconomyLogRecord;
use std::collections::{BTreeMap, BTreeSet};
use turts_mod_api::{
    Behaviour, ContentId, ResourceKind, TerrainOperation, TerrainOutputDestination,
};
use turts_sim::economy::{
    events::{CostLine, EventSource},
    views::{CatalogUnitAudit, ItemView},
};
use turts_sim::{
    AuditState, EconomyEventKind, OperationAudit, OrderTargetSim,
    terrain::{TerrainAudit, TerrainColumnAudit, TerrainIntervalAudit, TerrainWorkAudit},
};

type Accounts = BTreeMap<(u8, u32, ContentId), u64>;
fn key(source: EventSource, id: ContentId) -> (u8, u32, ContentId) {
    match source {
        EventSource::Slot(slot) => (0, u32::from(slot), id),
        EventSource::Inventory(unit) => (1, unit, id),
        EventSource::Output(unit) => (2, unit, id),
    }
}
fn costs(rows: &[CostLine]) -> Result<Accounts, String> {
    let mut result = Accounts::new();
    for row in rows {
        if row.amount == 0 {
            return Err("terrain event contains zero cost row".into());
        }
        let sum = result
            .entry(key(row.source, row.resource.clone()))
            .or_default();
        *sum = sum
            .checked_add(row.amount)
            .ok_or("terrain cost sum overflow")?;
    }
    Ok(result)
}
fn definition(state: &AuditState, unit: u32) -> Result<&CatalogUnitAudit, String> {
    let actor = state
        .raw_units
        .iter()
        .find(|row| row.unit_id == unit)
        .ok_or("terrain actor missing")?;
    let content = &state
        .unit_types
        .iter()
        .find(|row| row.type_id == actor.unit_type)
        .ok_or("terrain actor type missing")?
        .content_id;
    state
        .catalog
        .as_ref()
        .ok_or("terrain catalog missing")?
        .units
        .iter()
        .find(|row| row.id.to_string() == *content)
        .ok_or_else(|| "terrain actor definition missing".into())
}
fn terrain(state: &AuditState) -> Result<&TerrainAudit, String> {
    state
        .raw_terrain
        .as_ref()
        .ok_or_else(|| "terrain audit is missing".into())
}
fn point(column: &TerrainColumnAudit) -> (u32, u32) {
    (column.cell.x, column.cell.y)
}
fn work_order(state: &AuditState, unit: u32) -> Option<&turts_sim::Order> {
    state
        .unit_orders
        .iter()
        .find(|row| row.unit_id == unit)?
        .orders
        .current
        .as_ref()
}
fn one_prism(terrain: &TerrainAudit, ratio: turts_mod_api::Ratio) -> Result<u64, String> {
    let volume = u128::from(terrain.cell_width)
        .checked_mul(u128::from(terrain.cell_width))
        .and_then(|v| v.checked_mul(u128::from(terrain.elevation_step)))
        .ok_or("terrain prism volume overflow")?;
    let numerator = volume
        .checked_mul(u128::from(ratio.numerator))
        .ok_or("terrain ratio numerator overflow")?;
    let scale = u128::from(terrain.length_scale);
    let denominator = scale
        .checked_mul(scale)
        .and_then(|v| v.checked_mul(scale))
        .and_then(|v| v.checked_mul(u128::from(ratio.denominator)))
        .filter(|v| *v > 0)
        .ok_or("terrain ratio denominator invalid")?;
    if numerator % denominator != 0 {
        return Err("terrain ratio is not an exact per-prism integer".into());
    }
    u64::try_from(numerator / denominator).map_err(|_| "terrain ratio exceeds account range".into())
}
fn validate_state(state: &AuditState) -> Result<(), String> {
    let Some(terrain) = &state.raw_terrain else {
        return Ok(());
    };
    let catalog = state.catalog.as_ref().ok_or("terrain catalog missing")?;

    if terrain.columns == 0
        || terrain.rows == 0
        || terrain.length_scale == 0
        || terrain.cell_width == 0
        || terrain.elevation_step == 0
        || terrain.floor > terrain.ceiling
        || terrain.columns_state.len() != terrain.columns as usize * terrain.rows as usize
    {
        return Err("terrain geometry/column count invalid".into());
    }
    for (index, column) in terrain.columns_state.iter().enumerate() {
        if point(column)
            != (
                index as u32 % terrain.columns,
                index as u32 / terrain.columns,
            )
            || !(terrain.floor..=terrain.ceiling).contains(&column.initial_surface)
            || !(terrain.floor..=column.initial_surface).contains(&column.natural_top)
        {
            return Err("terrain column identity or natural cursor invalid".into());
        }
        let mut top = terrain.floor;
        for interval in &column.natural {
            if interval.bottom != top
                || interval.top <= top
                || !catalog
                    .terrain_materials
                    .iter()
                    .any(|material| material.id == interval.material.to_string())
            {
                return Err("terrain natural coverage/material invalid".into());
            }
            top = interval.top;
        }
        if top != column.natural_top {
            return Err("terrain remaining natural coverage differs from depletion cursor".into());
        }
        top = column.natural_top;
        for interval in &column.placed {
            if interval.bottom != top
                || interval.top <= top
                || !catalog
                    .terrain_materials
                    .iter()
                    .any(|material| material.id == interval.material.to_string())
            {
                return Err("terrain placed provenance has gap or unknown material".into());
            }
            top = interval.top;
        }
        if top != column.surface || top > terrain.ceiling {
            return Err("terrain surface differs from retained material runs".into());
        }
    }
    for inventory in &state.economy.inventories {
        let definition = definition(state, inventory.unit)?;
        let Some((capacity, accepts)) = &definition.inventory else {
            return Err("terrain-map inventory has no authored capability".into());
        };
        if inventory.capacity != *capacity
            || inventory
                .items
                .iter()
                .any(|item| !accepts.contains(&item.resource) || item.reserved > item.on_hand)
            || inventory
                .items
                .iter()
                .map(|item| u128::from(item.on_hand))
                .sum::<u128>()
                > u128::from(*capacity)
        {
            return Err(
                "terrain-map inventory differs from finite authored capacity/acceptance".into(),
            );
        }
    }
    let mut cargo_units = BTreeSet::new();
    for (unit, carry) in &terrain.cargo_carries {
        if !cargo_units.insert(*unit)
            || *carry
                >= u64::from(
                    state
                        .tick_hz
                        .filter(|hz| *hz > 0)
                        .ok_or("cargo clock has no tick rate")?,
                )
            || definition(state, *unit)?.terrain_cargo.is_none()
        {
            return Err("cargo fractional clock has invalid actor/remainder".into());
        }
    }
    let mut units = BTreeSet::new();
    for row in &terrain.work {
        if !units.insert(row.unit)
            || row.carry >= u64::from(state.tick_hz.ok_or("terrain work has no tick rate")?)
            || row.required == Some(0)
        {
            return Err("terrain work duplicate/rate remainder invalid".into());
        }
        let actor = state
            .raw_units
            .iter()
            .find(|actor| actor.unit_id == row.unit)
            .ok_or("terrain work has absent actor")?;
        if actor.owner_slot != row.owner.0 {
            return Err("terrain work retained another owner's authority".into());
        }
        if row.order.behaviour != Behaviour::TerrainWork
            || !matches!(row.order.target, OrderTargetSim::TerrainWork { .. })
        {
            return Err("terrain runtime has non-work order".into());
        }
        if target_refusal(state, row.unit, &row.order.target, false)?.is_some() {
            return Err("terrain runtime designation is not valid for its actor/map".into());
        }
        let paused = terrain.paused.iter().find(|pause| pause.unit == row.unit);
        let current = work_order(state, row.unit);
        if current != Some(&row.order)
            && !paused.is_some_and(|pause| {
                pause.work == row.order
                    && pause.owner == row.owner
                    && current.is_some_and(|current| current.behaviour == Behaviour::UnloadCargo)
                    && state
                        .unit_orders
                        .iter()
                        .find(|orders| orders.unit_id == row.unit)
                        .is_some_and(|orders| orders.orders.queue.front() == Some(&row.order))
            })
        {
            return Err(
                "terrain progress has neither its exact live order nor explicit unload suspension"
                    .into(),
            );
        }
    }
    for orders in &state.unit_orders {
        if let Some(order) = orders
            .orders
            .current
            .as_ref()
            .filter(|order| order.behaviour == Behaviour::TerrainWork)
            && !terrain.work.iter().any(|row| row.unit == orders.unit_id)
            && !order_complete(state, order)?
        {
            return Err("live terrain order lost its partial work state".into());
        }
    }
    let mut claims = BTreeSet::new();
    for claim in &terrain.claims {
        if !claims.insert((claim.cell.x, claim.cell.y))
            || !terrain
                .work
                .iter()
                .any(|row| row.unit == claim.unit && row.cell == Some(claim.cell))
        {
            return Err("terrain claim has no unique matching work cell".into());
        }
    }
    let mut paused_units = BTreeSet::new();
    for pause in &terrain.paused {
        if !paused_units.insert(pause.unit)
            || !state
                .unit_orders
                .iter()
                .find(|row| row.unit_id == pause.unit)
                .is_some_and(|row| {
                    row.orders
                        .current
                        .as_ref()
                        .is_some_and(|order| order.behaviour == Behaviour::UnloadCargo)
                        && row.orders.queue.front() == Some(&pause.work)
                })
        {
            return Err("terrain pause guard has no exact active utility/resume queue".into());
        }
        if !terrain.work.iter().any(|row| {
            row.unit == pause.unit && row.owner == pause.owner && row.order == pause.work
        }) {
            return Err("terrain pause has no matching work state".into());
        }
    }
    Ok(())
}
fn same_columns(before: &TerrainAudit, after: &TerrainAudit) -> bool {
    before.columns == after.columns
        && before.rows == after.rows
        && before.floor == after.floor
        && before.ceiling == after.ceiling
        && before.cell_width == after.cell_width
        && before.elevation_step == after.elevation_step
        && before.length_scale == after.length_scale
        && before.columns_state == after.columns_state
}
#[derive(Default)]
struct Credits {
    paid: BTreeMap<u32, u32>,
    resumed: BTreeMap<u32, u32>,
}
impl Credits {
    fn earned(&mut self, state: &AuditState, row: &TerrainWorkAudit) -> Result<(u64, u64), String> {
        let tick = state.economy.tick;
        if self.paid.insert(row.unit, tick) == Some(tick) {
            return Ok((0, row.carry));
        }
        let OrderTargetSim::TerrainWork { tool, .. } = &row.order.target else {
            return Err("terrain runtime target invalid".into());
        };
        let rate = state
            .catalog
            .as_ref()
            .ok_or("terrain catalog missing")?
            .terrain_tools
            .iter()
            .find(|definition| definition.id == tool.to_string())
            .ok_or("terrain tool missing")?
            .work_per_second;
        let hz = u128::from(
            state
                .tick_hz
                .filter(|hz| *hz > 0)
                .ok_or("terrain tick rate invalid")?,
        );
        let earned = u128::from(rate) + u128::from(row.carry);
        Ok((
            u64::try_from(earned / hz).map_err(|_| "terrain earned work overflow")?,
            (earned % hz) as u64,
        ))
    }
    fn finish_tick(&self, state: &AuditState) -> Result<(), String> {
        if let Some(raw) = &state.raw_terrain {
            for row in &raw.work {
                if matches!(
                    row.wait,
                    turts_sim::terrain::TerrainWorkWait::Working
                        | turts_sim::terrain::TerrainWorkWait::Budget
                ) && work_order(state, row.unit) == Some(&row.order)
                    && !raw.paused.iter().any(|pause| pause.unit == row.unit)
                    && self.paid.get(&row.unit) != Some(&state.economy.tick)
                    && self.resumed.get(&row.unit) != Some(&state.economy.tick)
                {
                    return Err("ready terrain work omitted its exact tick credit".into());
                }
            }
        }
        Ok(())
    }
    fn passive(
        &mut self,
        before: &AuditState,
        after: &AuditState,
        reset_cargo: &BTreeSet<u32>,
    ) -> Result<(), String> {
        let (Some(old), Some(new)) = (&before.raw_terrain, &after.raw_terrain) else {
            if before.raw_terrain.is_some() != after.raw_terrain.is_some() {
                return Err("terrain appeared/disappeared without package installation".into());
            }
            return Ok(());
        };
        if !same_columns(old, new) {
            return Err(
                "terrain column/depletion/revision changed without a committed step".into(),
            );
        }
        for (unit, carry) in &new.cargo_carries {
            if !old.cargo_carries.contains(&(*unit, *carry)) {
                return Err("cargo clock changed outside a timed utility operation".into());
            }
        }
        for (unit, _) in &old.cargo_carries {
            if !new.cargo_carries.iter().any(|(actor, _)| actor == unit)
                && !reset_cargo.contains(unit)
                && after.raw_units.iter().any(|actor| actor.unit_id == *unit)
                && !after
                    .raw_healths
                    .iter()
                    .any(|health| health.unit == *unit && health.current == 0)
                && work_order(after, *unit)
                    .is_some_and(|order| order.behaviour == Behaviour::UnloadCargo)
            {
                return Err(
                    "cargo fractional carry disappeared without cancellation/completion".into(),
                );
            }
        }
        for next in &new.work {
            if reset_cargo.contains(&next.unit) {
                if next.done != 0
                    || next.carry != 0
                    || next.banked
                    || next.cell.is_some()
                    || next.source_revision.is_some()
                    || next.required.is_some()
                    || next.wait != turts_sim::terrain::TerrainWorkWait::Unknown
                {
                    return Err("replaced terrain order retained fabricated old progress".into());
                }
                continue;
            }
            let Some(prior) = old.work.iter().find(|row| {
                row.unit == next.unit && row.owner == next.owner && row.order == next.order
            }) else {
                if next.done != 0 || next.carry != 0 || next.banked {
                    return Err("new terrain order starts with fabricated work".into());
                }
                continue;
            };
            let suspended = old.paused.iter().any(|pause| pause.unit == next.unit)
                || new.paused.iter().any(|pause| pause.unit == next.unit);
            if next.done == prior.done && next.carry == prior.carry {
                continue;
            }
            if suspended {
                return Err("suspended terrain order changed progress/carry".into());
            }
            if !prior.banked && next.banked {
                return Err("terrain created banked work without a committed step".into());
            }
            if next.done == 0
                && next.carry == 0
                && (!prior.banked
                    && (prior.cell != next.cell
                        || next
                            .cell
                            .and_then(|cell| {
                                new.columns_state.iter().find(|column| column.cell == cell)
                            })
                            .is_some_and(|column| {
                                next.source_revision == Some(column.revision)
                                    && prior.source_revision != Some(column.revision)
                            })))
            {
                continue;
            }
            if !matches!(
                next.wait,
                turts_sim::terrain::TerrainWorkWait::Working
                    | turts_sim::terrain::TerrainWorkWait::Budget
            ) {
                return Err("waiting terrain order accrued work".into());
            }
            let (increment, carry) = self.earned(after, prior)?;
            if next.done
                != prior
                    .done
                    .checked_add(increment)
                    .ok_or("terrain work bank overflow")?
                || next.carry != carry
            {
                return Err(
                    "terrain passive work differs from exact one-tick integer accrual".into(),
                );
            }
            if after.disabled.contains(&next.unit) {
                return Err("disabled terrain order accrued work".into());
            }
        }
        Ok(())
    }
}
fn change_account(state: &mut AuditState, row: &CostLine, credit: bool) -> Result<(), String> {
    let adjust = |value: u64, reserved: u64| -> Result<u64, String> {
        if credit {
            value
                .checked_add(row.amount)
                .ok_or_else(|| "terrain credit overflow".into())
        } else {
            value
                .checked_sub(row.amount)
                .filter(|value| *value >= reserved)
                .ok_or_else(|| "terrain debit consumes absent/reserved resources".into())
        }
    };
    match row.source {
        EventSource::Slot(slot) => {
            let account = state
                .economy
                .balances
                .iter_mut()
                .find(|balance| balance.slot == slot && balance.resource == row.resource)
                .ok_or("terrain balance endpoint missing")?;
            account.on_hand = adjust(account.on_hand, account.reserved)?;
        }
        EventSource::Inventory(unit) => {
            let inventory = state
                .economy
                .inventories
                .iter_mut()
                .find(|inventory| inventory.unit == unit)
                .ok_or("terrain inventory endpoint missing")?;
            if !inventory
                .items
                .iter()
                .any(|item| item.resource == row.resource)
            {
                if !credit {
                    return Err("terrain input inventory item missing".into());
                }
                inventory.items.push(ItemView {
                    resource: row.resource.clone(),
                    on_hand: 0,
                    reserved: 0,
                });
                inventory.items.sort_by(|a, b| a.resource.cmp(&b.resource));
            }
            let account = inventory
                .items
                .iter_mut()
                .find(|item| item.resource == row.resource)
                .unwrap();
            account.on_hand = adjust(account.on_hand, account.reserved)?;
            if inventory
                .items
                .iter()
                .try_fold(0u64, |sum, item| sum.checked_add(item.on_hand))
                .ok_or("terrain inventory total overflow")?
                > inventory.capacity
            {
                return Err("terrain output exceeds physical capacity".into());
            }
        }
        EventSource::Output(_) => {
            return Err("terrain cannot debit or yield to output inventory".into());
        }
    }
    Ok(())
}
fn step(operation: &OperationAudit, credits: &mut Credits) -> Result<bool, String> {
    let Some(event) = operation
        .events
        .iter()
        .find(|event| matches!(event.kind, EconomyEventKind::TerrainStepCommitted { .. }))
    else {
        return Ok(false);
    };
    if operation.events.len() != 1 {
        return Err("terrain step contains unrelated events".into());
    }
    let EconomyEventKind::TerrainStepCommitted {
        unit,
        cell,
        before,
        after,
        material,
        placed,
        inputs,
        outputs,
    } = &event.kind
    else {
        unreachable!()
    };
    let raw = terrain(&operation.before)?;
    let catalog = operation
        .before
        .catalog
        .as_ref()
        .ok_or("terrain catalog missing")?;
    let actor = operation
        .before
        .raw_units
        .iter()
        .find(|actor| actor.unit_id == *unit)
        .ok_or("terrain step actor absent")?;
    if event.owner_slot != actor.owner_slot
        || event.tick != operation.before.economy.tick
        || Some(event.ordinal) != operation.before.next_event_ordinal
    {
        return Err("terrain event owner/tick/ordinal differs from authoritative operation".into());
    }
    let runtime = raw
        .work
        .iter()
        .find(|row| row.unit == *unit)
        .ok_or("terrain step has no work state")?;
    if runtime.owner.0 != actor.owner_slot
        || work_order(&operation.before, *unit) != Some(&runtime.order)
        || operation.before.disabled.contains(unit)
        || operation
            .before
            .raw_healths
            .iter()
            .any(|health| health.unit == *unit && health.current == 0)
    {
        return Err("terrain step uses inactive actor/order authority".into());
    }
    let OrderTargetSim::TerrainWork {
        tool,
        cells,
        target_elevation,
    } = &runtime.order.target
    else {
        return Err("terrain step lacks designation".into());
    };
    if cell.x < cells.min.x
        || cell.x >= cells.max.x
        || cell.y < cells.min.y
        || cell.y >= cells.max.y
        || runtime.cell != Some(*cell)
        || !raw
            .claims
            .iter()
            .any(|claim| claim.unit == *unit && claim.cell == *cell)
    {
        return Err("terrain step is outside its designation/claim".into());
    }
    let tool = catalog
        .terrain_tools
        .iter()
        .find(|definition| definition.id == tool.to_string())
        .ok_or("terrain step tool missing")?;
    if !definition(&operation.before, *unit)?
        .terrain_tools
        .iter()
        .any(|id| id.to_string() == tool.id)
    {
        return Err("terrain step tool not installed on actor".into());
    }
    let index = raw
        .columns_state
        .iter()
        .position(|column| column.cell == *cell)
        .ok_or("terrain step cell absent")?;
    let actor_definition = definition(&operation.before, *unit)?;
    let width = f64::from(raw.cell_width) / f64::from(raw.length_scale);
    let distance_to_cell = |position: turts_mod_api::WorldPos| {
        let dx = f64::from(position.x)
            - f64::from(position.x).clamp(f64::from(cell.x) * width, f64::from(cell.x + 1) * width);
        let dy = f64::from(position.y)
            - f64::from(position.y).clamp(f64::from(cell.y) * width, f64::from(cell.y + 1) * width);
        dx * dx + dy * dy
    };
    let reach = f64::from(tool.reach) + f64::from(actor_definition.radius);
    if distance_to_cell(actor.pos) > reach * reach {
        return Err("terrain step is outside authored tool reach".into());
    }
    for body in &operation.before.raw_units {
        let radius = f64::from(definition(&operation.before, body.unit_id)?.radius);
        if distance_to_cell(body.pos) < radius * radius {
            return Err("terrain step edits an occupied body/foundation footprint".into());
        }
    }
    for node in &operation.before.nodes {
        let radius = f64::from(
            catalog
                .nodes
                .iter()
                .find(|definition| definition.id == node.content_id)
                .ok_or("terrain blocker node definition missing")?
                .radius,
        );
        if distance_to_cell(node.pos) < radius * radius {
            return Err("terrain step edits an occupied node footprint".into());
        }
    }
    let column = &raw.columns_state[index];
    if *before != column.surface || before == after {
        return Err("terrain step start/size differs from current surface".into());
    }
    let count = u64::from((i64::from(*before) - i64::from(*after)).unsigned_abs() as u32);
    let (expected_material, expected_placed) = match &tool.operation {
        TerrainOperation::Excavate => {
            if after >= before || after < target_elevation {
                return Err("excavation passes its requested target".into());
            }
            if let Some(run) = column.placed.last() {
                if count > (i64::from(run.top) - i64::from(run.bottom)) as u64 {
                    return Err("batch crosses exposed placed run".into());
                }
                (run.material.clone(), true)
            } else {
                let run = column
                    .natural
                    .iter()
                    .find(|run| run.bottom < column.natural_top && run.top >= column.natural_top)
                    .ok_or("excavation crosses bedrock")?;
                if count > (i64::from(column.natural_top) - i64::from(run.bottom)) as u64 {
                    return Err("batch inspects/crosses buried natural run".into());
                }
                (run.material.clone(), false)
            }
        }
        TerrainOperation::Fill { material } => {
            if after <= before || after > target_elevation {
                return Err("fill passes its requested target".into());
            }
            (
                ContentId::parse(material).map_err(|_| "fill material reference invalid")?,
                true,
            )
        }
    };
    if &expected_material != material || expected_placed != *placed {
        return Err("terrain material/provenance differs from exposed prism".into());
    }
    let material = catalog
        .terrain_materials
        .iter()
        .find(|definition| definition.id == expected_material.to_string())
        .ok_or("terrain exposed material missing")?;
    if material.hardness > tool.max_hardness || !tool.allowed_materials.contains(&material.id) {
        return Err("terrain tool cannot work exposed material".into());
    }
    let work = one_prism(raw, material.work_per_world_cube)?
        .checked_mul(count)
        .filter(|work| *work > 0)
        .ok_or("terrain work total invalid")?;
    let mut expected_inputs = Vec::new();
    let mut expected_outputs = Vec::new();
    let filling = matches!(tool.operation, TerrainOperation::Fill { .. });
    let rates = if filling {
        &material.fill_inputs
    } else {
        &material.natural_outputs
    };
    if filling || !*placed {
        for rate in rates {
            let resource = ContentId::parse(&rate.resource)
                .map_err(|_| "terrain resource reference invalid")?;
            let kind = &catalog
                .resource_kinds
                .iter()
                .find(|(id, _)| id == &resource)
                .ok_or("terrain resource missing")?
                .1;
            let source = if filling {
                match kind {
                    ResourceKind::Item => EventSource::Inventory(*unit),
                    ResourceKind::Balance => EventSource::Slot(actor.owner_slot),
                    _ => return Err("terrain supply input invalid".into()),
                }
            } else {
                match (tool.output_destination, kind) {
                    (TerrainOutputDestination::Inventory, ResourceKind::Item) => {
                        EventSource::Inventory(*unit)
                    }
                    (TerrainOutputDestination::SlotBalances, ResourceKind::Balance) => {
                        EventSource::Slot(actor.owner_slot)
                    }
                    _ => return Err("terrain yield endpoint kind mismatch".into()),
                }
            };
            let amount = one_prism(raw, rate.per_world_cube)?
                .checked_mul(count)
                .ok_or("terrain resource total overflow")?;
            if amount > 0 {
                let row = CostLine {
                    source,
                    resource,
                    amount,
                };
                if filling {
                    expected_inputs.push(row);
                } else {
                    expected_outputs.push(row);
                }
            }
        }
    }
    if costs(inputs)? != costs(&expected_inputs)? || costs(outputs)? != costs(&expected_outputs)? {
        return Err("terrain step resource payload differs from independent per-prism law".into());
    }
    let mut expected = operation.before.clone();
    for row in &expected_inputs {
        change_account(&mut expected, row, false)?;
    }
    for row in &expected_outputs {
        change_account(&mut expected, row, true)?;
    }
    let next = expected.raw_terrain.as_mut().unwrap();
    let column = &mut next.columns_state[index];
    if filling {
        if let Some(last) = column
            .placed
            .last_mut()
            .filter(|last| last.material == expected_material)
        {
            last.top = *after;
        } else {
            column.placed.push(TerrainIntervalAudit {
                bottom: *before,
                top: *after,
                material: expected_material,
            });
        }
    } else if *placed {
        let last = column.placed.last_mut().unwrap();
        last.top = *after;
        if last.top == last.bottom {
            column.placed.pop();
        }
    } else {
        column.natural_top = *after;
        // Remaining natural mass is the intersection of each original run with
        // the not-yet-depleted datum interval. Removed natural mass never returns.
        column.natural = column
            .natural
            .iter()
            .filter_map(|run| {
                let top = run.top.min(*after);
                (top > run.bottom).then(|| TerrainIntervalAudit {
                    bottom: run.bottom,
                    top,
                    material: run.material.clone(),
                })
            })
            .collect();
    }
    column.surface = *after;
    column.revision = column
        .revision
        .checked_add(count)
        .ok_or("terrain revision overflow")?;
    let (increment, carry) = credits.earned(&operation.before, runtime)?;
    let row = next.work.iter_mut().find(|row| row.unit == *unit).unwrap();
    row.done = row
        .done
        .checked_add(increment)
        .and_then(|done| done.checked_sub(work))
        .ok_or("terrain step spends work that was never earned")?;
    row.carry = carry;
    row.required = Some(work);
    row.banked = true;
    row.source_revision = Some(column.revision);
    row.wait = turts_sim::terrain::TerrainWorkWait::Working;
    expected.pending_events.push(event.clone());
    expected.next_event_ordinal = event.ordinal.checked_add(1);
    if expected != operation.after {
        return Err(format!(
            "terrain step {} differs from exact independent transition: {:?}",
            operation.operation_id,
            crate::replay::audit_differences(
                &serde_json::to_value(&expected).map_err(|error| error.to_string())?,
                &serde_json::to_value(&operation.after).map_err(|error| error.to_string())?,
                ""
            )
        ));
    }

    Ok(true)
}
/// Run before generic operation validation. This provides the proof used by the
/// generic validator when it reconciles this separately owned state component.
pub(crate) fn validate_records(records: &[EconomyLogRecord]) -> Result<(), String> {
    let Some(EconomyLogRecord::Initial(initial)) = records.first() else {
        return Ok(());
    };
    if initial.raw_terrain.is_none() {
        // Keep legacy streams linear. No separately verified terrain component may
        // appear later and be reconciled into a trusted flat starting state.
        let supplied = records.iter().any(|record| match record {
            EconomyLogRecord::Initial(state) => state.raw_terrain.is_some(),
            EconomyLogRecord::Operation(operation) => {
                operation.before.raw_terrain.is_some()
                    || operation.after.raw_terrain.is_some()
                    || operation.events.iter().any(|event| {
                        matches!(event.kind, EconomyEventKind::TerrainStepCommitted { .. })
                    })
            }
            EconomyLogRecord::CommandBoundary(command) => {
                command.before.raw_terrain.is_some() || command.after.raw_terrain.is_some()
            }
            EconomyLogRecord::Tick {
                before,
                state,
                events,
                ..
            } => {
                before.raw_terrain.is_some()
                    || state.raw_terrain.is_some()
                    || events.iter().any(|event| {
                        matches!(event.kind, EconomyEventKind::TerrainStepCommitted { .. })
                    })
            }
            EconomyLogRecord::Admission { before, after, .. } => {
                before.raw_terrain.is_some() || after.raw_terrain.is_some()
            }
            _ => false,
        });
        return if supplied {
            Err("terrain appeared in a flat initial-state stream".into())
        } else {
            Ok(())
        };
    }
    validate_state(initial)?;
    let reset_spans = records
        .iter()
        .filter_map(|record| {
            let EconomyLogRecord::CommandBoundary(command) = record else {
                return None;
            };
            let turts_sim::CommandKind::Order(order) = &command.input.kind else {
                return None;
            };
            if order.behaviour == Behaviour::Stop
                || (order.queue == turts_sim::QueueMode::Replace
                    && matches!(
                        order.behaviour,
                        Behaviour::Move
                            | Behaviour::Hold
                            | Behaviour::Attack
                            | Behaviour::Gather
                            | Behaviour::Repair
                            | Behaviour::TerrainWork
                            | Behaviour::UnloadCargo
                    ))
            {
                Some((
                    command.operation_id,
                    (
                        command.operation_end_id.unwrap_or(command.operation_id),
                        command
                            .result
                            .units
                            .accepted
                            .iter()
                            .copied()
                            .collect::<BTreeSet<_>>(),
                    ),
                ))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();
    let empty = BTreeSet::new();
    let mut previous = initial;
    let mut credits = Credits::default();
    for record in records.iter().skip(1) {
        match record {
            EconomyLogRecord::Operation(operation) => {
                validate_state(&operation.before)?;
                credits.passive(previous, &operation.before, &BTreeSet::new())?;
                let stepped = step(operation, &mut credits)?;
                let unloaded = !stepped && cargo(operation)?;
                if unloaded
                    && let Some(unit) = operation.unit_id
                    && work_order(&operation.after, unit)
                        .is_some_and(|order| order.behaviour == Behaviour::TerrainWork)
                    && work_order(&operation.before, unit)
                        .is_some_and(|order| order.behaviour == Behaviour::UnloadCargo)
                {
                    credits.resumed.insert(unit, operation.after.economy.tick);
                }
                if !stepped && !unloaded {
                    if operation.before.raw_terrain.is_some()
                        && operation.events.is_empty()
                        && operation.before.raw_queues == operation.after.raw_queues
                        && operation.before.economy.construction
                            == operation.after.economy.construction
                        && operation.before.raw_repairs == operation.after.raw_repairs
                        && (operation.before.economy.balances != operation.after.economy.balances
                            || operation.before.economy.inventories
                                != operation.after.economy.inventories)
                    {
                        return Err("failed/passive terrain operation changed accounts without committed work or funded progress".into());
                    }
                    let resets = reset_spans
                        .range(..=operation.operation_id)
                        .next_back()
                        .filter(|(_, (end, _))| *end >= operation.operation_id)
                        .map_or(&empty, |(_, (_, units))| units);
                    credits.passive(&operation.before, &operation.after, resets)?;
                }
                validate_state(&operation.after)?;
                previous = &operation.after;
            }
            EconomyLogRecord::Tick { state, .. } => {
                validate_state(state)?;
                credits.passive(previous, state, &BTreeSet::new())?;
                credits.finish_tick(state)?;
                previous = state;
            }
            _ => {}
        }
    }
    Ok(())
}
/// Only call after validate_records has established this component's transitions.
pub(crate) fn reconcile_verified(expected: &mut AuditState, current: &AuditState) {
    expected.raw_terrain = current.raw_terrain.clone();
}
pub(crate) fn has_capability(state: &AuditState, unit: u32, cargo: bool) -> bool {
    definition(state, unit).is_ok_and(|unit| {
        if cargo {
            unit.terrain_cargo.is_some()
        } else {
            !unit.terrain_tools.is_empty()
        }
    })
}

pub(crate) fn order_complete(state: &AuditState, order: &turts_sim::Order) -> Result<bool, String> {
    let OrderTargetSim::TerrainWork {
        tool,
        cells,
        target_elevation,
    } = &order.target
    else {
        return Ok(false);
    };
    let raw = terrain(state)?;
    let catalog = state.catalog.as_ref().ok_or("terrain catalog missing")?;
    let tool = catalog
        .terrain_tools
        .iter()
        .find(|row| row.id == tool.to_string())
        .ok_or("terrain tool missing")?;
    Ok((cells.min.y..cells.max.y).all(|y| {
        (cells.min.x..cells.max.x).all(|x| {
            raw.columns_state
                .iter()
                .find(|row| row.cell.x == x && row.cell.y == y)
                .is_some_and(|column| match tool.operation {
                    TerrainOperation::Excavate => column.surface <= *target_elevation,
                    TerrainOperation::Fill { .. } => column.surface >= *target_elevation,
                })
        })
    }))
}
pub(crate) fn target_refusal(
    state: &AuditState,
    unit: u32,
    target: &OrderTargetSim,
    cargo: bool,
) -> Result<Option<turts_sim::RejectReason>, String> {
    use turts_sim::RejectReason as R;
    if cargo {
        let OrderTargetSim::Unit(depot) = target else {
            return Ok(Some(R::TargetInvalid));
        };
        if *depot == unit {
            return Ok(Some(R::TargetInvalid));
        }
        let Some(source) = state.raw_units.iter().find(|row| row.unit_id == unit) else {
            return Ok(Some(R::UnknownUnit));
        };
        let Some(receiver) = state.raw_units.iter().find(|row| row.unit_id == *depot) else {
            return Ok(Some(R::TargetGone));
        };
        if source.owner_slot != receiver.owner_slot {
            return Ok(Some(R::NotOwner));
        }
        if state
            .raw_healths
            .iter()
            .any(|row| (row.unit == unit || row.unit == *depot) && row.current == 0)
        {
            return Ok(Some(R::TargetGone));
        }
        if definition(state, *depot)?.drop_off.is_empty() {
            return Ok(Some(R::TargetInvalid));
        }
        return Ok(None);
    }
    let OrderTargetSim::TerrainWork {
        tool,
        cells,
        target_elevation,
    } = target
    else {
        return Ok(Some(R::TargetInvalid));
    };
    let Some(raw) = state.raw_terrain.as_ref() else {
        return Ok(Some(R::NotApplicable));
    };
    if cells.min.x >= cells.max.x
        || cells.min.y >= cells.max.y
        || cells.max.x > raw.columns
        || cells.max.y > raw.rows
        || u64::from(cells.max.x - cells.min.x) * u64::from(cells.max.y - cells.min.y) > 256
        || *target_elevation < raw.floor
        || *target_elevation > raw.ceiling
    {
        return Ok(Some(R::TargetInvalid));
    }
    if !definition(state, unit)?.terrain_tools.contains(tool)
        || !state
            .catalog
            .as_ref()
            .ok_or("terrain catalog missing")?
            .terrain_tools
            .iter()
            .any(|row| row.id == tool.to_string())
    {
        return Ok(Some(R::MissingCapability));
    }
    Ok(None)
}

fn set_cargo_carry(state: &mut AuditState, unit: u32, carry: Option<u64>) {
    let raw = state.raw_terrain.as_mut().unwrap();
    raw.cargo_carries.retain(|(actor, _)| *actor != unit);
    if let Some(carry) = carry {
        raw.cargo_carries.push((unit, carry));
        raw.cargo_carries.sort_by_key(|(actor, _)| *actor);
    }
}
fn cargo(operation: &OperationAudit) -> Result<bool, String> {
    if operation.phase != turts_sim::economy::transaction::AuditPhase::PostMovementOrders {
        return Ok(false);
    }
    let Some(unit) = operation.unit_id else {
        return Ok(false);
    };
    let Some(order) = work_order(&operation.before, unit)
        .filter(|order| order.behaviour == Behaviour::UnloadCargo)
    else {
        return Ok(false);
    };
    let OrderTargetSim::Unit(depot) = order.target else {
        return Err("cargo utility lacks its depot".into());
    };
    let mut expected = operation.before.clone();
    if target_refusal(&operation.before, unit, &order.target, true)?.is_some()
        || operation.before.disabled.contains(&unit)
    {
        if expected != operation.after || !operation.events.is_empty() {
            return Err("inactive cargo utility changed state".into());
        }
        return Ok(true);
    }
    let actor = operation
        .before
        .raw_units
        .iter()
        .find(|row| row.unit_id == unit)
        .unwrap();
    let receiver = operation
        .before
        .raw_units
        .iter()
        .find(|row| row.unit_id == depot)
        .unwrap();
    let actor_definition = definition(&operation.before, unit)?;
    let receiver_definition = definition(&operation.before, depot)?;
    let cargo = actor_definition
        .terrain_cargo
        .as_ref()
        .ok_or("cargo utility has no authored rate")?;
    let bodies = actor_definition.radius + receiver_definition.radius;
    let from = bevy::math::Vec2::new(actor.pos.x, actor.pos.y);
    let to = bevy::math::Vec2::new(receiver.pos.x, receiver.pos.y);
    let dx = f64::from(actor.pos.x) - f64::from(receiver.pos.x);
    let dy = f64::from(actor.pos.y) - f64::from(receiver.pos.y);
    let reach = f64::from(bodies) + f64::from(cargo.interaction_range);
    if dx * dx + dy * dy > reach * reach {
        let target = to + (from - to).normalize_or_zero() * bodies;
        crate::replay::set_audit_move_target(
            &mut expected,
            unit,
            Some(turts_mod_api::WorldPos {
                x: target.x,
                y: target.y,
            }),
        )?;
        if expected != operation.after || !operation.events.is_empty() {
            return Err("cargo approach differs from public depot surface goal".into());
        }
        return Ok(true);
    }
    crate::replay::set_audit_move_target(&mut expected, unit, None)?;
    let stock = operation
        .before
        .economy
        .inventories
        .iter()
        .find(|row| row.unit == unit)
        .map(|inventory| {
            inventory
                .items
                .iter()
                .filter_map(|row| {
                    row.on_hand
                        .checked_sub(row.reserved)
                        .filter(|amount| *amount > 0)
                        .map(|amount| (row.resource.clone(), amount))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if stock.is_empty() {
        let mut orders = operation
            .before
            .unit_orders
            .iter()
            .find(|row| row.unit_id == unit)
            .map(|row| row.orders.clone())
            .unwrap_or_default();
        orders.current = None;
        let resume = terrain(&operation.before)?
            .paused
            .iter()
            .find(|pause| pause.unit == unit && orders.queue.front() == Some(&pause.work));
        set_cargo_carry(&mut expected, unit, None);
        let raw = expected.raw_terrain.as_mut().unwrap();
        raw.paused.retain(|pause| pause.unit != unit);
        if resume.is_none() {
            raw.work.retain(|row| row.unit != unit);
            raw.claims.retain(|row| row.unit != unit);
        }
        expected
            .pending_movement_orders
            .cancel_units(actor.owner_slot, &[unit]);
        expected.movement_progress.retain(|row| row.unit_id != unit);
        expected
            .plain_movement_progress
            .retain(|row| row.unit_id != unit);
        crate::replay::start_next_audited_order_if_not_pending(&mut expected, unit, &mut orders)?;
        if resume.is_none()
            && let Some(order) = orders
                .current
                .as_ref()
                .filter(|order| order.behaviour == Behaviour::TerrainWork)
        {
            expected
                .raw_terrain
                .as_mut()
                .unwrap()
                .work
                .push(TerrainWorkAudit {
                    owner: turts_sim::SlotId(actor.owner_slot),
                    unit,
                    order: order.clone(),
                    cell: None,
                    done: 0,
                    carry: 0,
                    banked: false,
                    required: None,
                    wait: turts_sim::terrain::TerrainWorkWait::Unknown,
                    source_revision: None,
                });
            expected
                .raw_terrain
                .as_mut()
                .unwrap()
                .work
                .sort_by_key(|row| row.unit);
        }
        crate::replay::store_audited_unit_orders(&mut expected, unit, orders);
        if expected != operation.after || !operation.events.is_empty() {
            return Err("cargo completion does not preserve exact paused work and queue".into());
        }
        return Ok(true);
    }
    let carry = terrain(&operation.before)?
        .cargo_carries
        .iter()
        .find(|(actor, _)| *actor == unit)
        .map_or(0, |(_, carry)| *carry);
    let hz = u128::from(
        operation
            .before
            .tick_hz
            .filter(|hz| *hz > 0)
            .ok_or("cargo tick rate missing")?,
    );
    let earned = u128::from(cargo.unload_per_second) + u128::from(carry);
    let mut budget = u64::try_from(earned / hz).map_err(|_| "cargo rate budget overflow")?;
    let next_carry = (earned % hz) as u64;
    if budget == 0 {
        set_cargo_carry(&mut expected, unit, Some(next_carry));
    }
    let mut events = Vec::new();
    for (item, available) in stock {
        if budget == 0 {
            break;
        }
        if !receiver_definition.drop_off.iter().any(|row| {
            row.item == item
                && matches!(
                    row.effect,
                    turts_sim::economy::views::CatalogDropOffEffectAudit::Store
                )
        }) {
            continue;
        }
        let Some((_, accepts)) = &receiver_definition.inventory else {
            continue;
        };
        if !accepts.contains(&item) {
            continue;
        }
        let Some(inventory) = expected
            .economy
            .inventories
            .iter()
            .find(|row| row.unit == depot)
        else {
            continue;
        };
        let used = inventory
            .items
            .iter()
            .try_fold(0u64, |sum, row| sum.checked_add(row.on_hand))
            .ok_or("cargo depot inventory overflows")?;
        let headroom = inventory
            .capacity
            .checked_sub(used)
            .ok_or("cargo depot exceeds capacity")?;
        let amount = budget.min(available).min(headroom);
        if amount == 0 {
            continue;
        }
        change_account(
            &mut expected,
            &CostLine {
                source: EventSource::Inventory(unit),
                resource: item.clone(),
                amount,
            },
            false,
        )?;
        change_account(
            &mut expected,
            &CostLine {
                source: EventSource::Inventory(depot),
                resource: item.clone(),
                amount,
            },
            true,
        )?;
        let event = turts_sim::EconomyEvent {
            tick: operation.before.economy.tick,
            ordinal: operation
                .before
                .next_event_ordinal
                .ok_or("cargo ordinal missing")?
                .checked_add(events.len() as u64)
                .ok_or("cargo ordinal overflow")?,
            owner_slot: actor.owner_slot,
            kind: EconomyEventKind::Stored {
                from: unit,
                to: depot,
                item,
                amount,
            },
        };
        events.push(event);
        budget -= amount;
    }
    if !events.is_empty() {
        set_cargo_carry(&mut expected, unit, Some(next_carry));
        for event in &events {
            expected.pending_events.push(event.clone());
            expected.next_event_ordinal = event.ordinal.checked_add(1);
        }
    }
    if events != operation.events || expected != operation.after {
        return Err("cargo store/carry differs from exact authored timed transfer".into());
    }
    Ok(true)
}
