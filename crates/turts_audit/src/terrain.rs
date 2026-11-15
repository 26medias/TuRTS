//! Independent terrain conservation. This module never calls simulation terrain/payment code.
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
    if terrain.length_scale == 0
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
        if top != column.initial_surface {
            return Err("terrain initial natural coverage differs from initial height".into());
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
    for pause in &terrain.paused {
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
    paid: BTreeSet<(u32, u32)>,
}
impl Credits {
    fn earned(&mut self, state: &AuditState, row: &TerrainWorkAudit) -> Result<(u64, u64), String> {
        let tick = state.economy.tick;
        if !self.paid.insert((tick, row.unit)) {
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
    fn passive(&mut self, before: &AuditState, after: &AuditState) -> Result<(), String> {
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
        for next in &new.work {
            let Some(prior) = old.work.iter().find(|row| {
                row.unit == next.unit && row.owner == next.owner && row.order == next.order
            }) else {
                if next.done != 0 || next.carry != 0 {
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
            if next.done == 0
                && next.carry == 0
                && (!prior.banked
                    && (prior.cell != next.cell || prior.source_revision != next.source_revision))
            {
                continue;
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
        return Err(
            "terrain step differs from exact independent column/account/work/event transition"
                .into(),
        );
    }
    Ok(true)
}
/// Run before generic operation validation. This provides the proof used by the
/// generic validator when it reconciles this separately owned state component.
pub(crate) fn validate_records(records: &[EconomyLogRecord]) -> Result<(), String> {
    let Some(EconomyLogRecord::Initial(initial)) = records.first() else {
        return Ok(());
    };
    validate_state(initial)?;
    let mut previous = initial;
    let mut credits = Credits::default();
    for record in records.iter().skip(1) {
        match record {
            EconomyLogRecord::Operation(operation) => {
                validate_state(&operation.before)?;
                credits.passive(previous, &operation.before)?;
                if !step(operation, &mut credits)? {
                    credits.passive(&operation.before, &operation.after)?;
                }
                validate_state(&operation.after)?;
                previous = &operation.after;
            }
            EconomyLogRecord::Tick { state, .. } => {
                validate_state(state)?;
                credits.passive(previous, state)?;
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
