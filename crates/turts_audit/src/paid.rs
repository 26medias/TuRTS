//! Independent paid-work accounting. This module uses captured definitions and
//! integer arithmetic. It does not call simulation payment or repair helpers.
use std::collections::BTreeMap;
use turts_mod_api::{ContentId, CostSource, PaymentPolicy};
use turts_sim::economy::production::FundingState;
use turts_sim::economy::{
    events::{CostLine, EventSource},
    views::{CatalogRecipeAudit, ReservationEndpointView},
};
use turts_sim::{AuditState, OperationAudit};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Destination {
    Slot(u8),
    Inventory(u32),
    Output(u32),
}
type AccountKey = (Destination, ContentId);
type Amounts = BTreeMap<AccountKey, u128>;

fn event_key(row: &CostLine) -> AccountKey {
    let endpoint = match row.source {
        EventSource::Slot(id) => Destination::Slot(id),
        EventSource::Inventory(id) => Destination::Inventory(id),
        EventSource::Output(id) => Destination::Output(id),
    };
    (endpoint, row.resource.clone())
}
fn sum(rows: &[CostLine]) -> Result<Amounts, String> {
    let mut sums = Amounts::new();
    for row in rows {
        let total = sums.entry(event_key(row)).or_default();
        *total = total
            .checked_add(u128::from(row.amount))
            .ok_or("paid-work aggregate overflow")?;
    }
    Ok(sums)
}
fn recipe<'a>(state: &'a AuditState, id: &ContentId) -> Result<&'a CatalogRecipeAudit, String> {
    state
        .catalog
        .as_ref()
        .and_then(|catalog| catalog.recipes.iter().find(|row| &row.id == id))
        .ok_or_else(|| format!("paid-work recipe {id} absent from catalog"))
}
#[derive(Clone)]
struct Job {
    unit: u32,
    job: u64,
    owner: u8,
    recipe: ContentId,
    done: u64,
    total: u64,
    charged: Vec<CostLine>,
    funding: FundingState,
    source: u32,
    site: bool,
}
fn jobs(state: &AuditState) -> Result<BTreeMap<(u32, u64), Job>, String> {
    let mut output = BTreeMap::new();
    for queue in &state.raw_queues {
        let owner = state
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == queue.producer)
            .ok_or("paid producer is absent")?
            .owner_slot;
        for job in &queue.jobs {
            let Some(funding) = &job.funding else {
                continue;
            };
            let charged = job
                .charged
                .iter()
                .map(|row| CostLine {
                    source: match row.endpoint {
                        ReservationEndpointView::Slot(id) => EventSource::Slot(id),
                        ReservationEndpointView::Inventory(id) => EventSource::Inventory(id),
                        ReservationEndpointView::Output(id) => EventSource::Output(id),
                    },
                    resource: row.resource.clone(),
                    amount: row.amount,
                })
                .collect();
            output.insert(
                (queue.producer, job.id),
                Job {
                    unit: queue.producer,
                    job: job.id,
                    owner,
                    recipe: job.recipe.clone(),
                    done: job.elapsed_ticks,
                    total: job.duration_ticks,
                    charged,
                    funding: funding.clone(),
                    source: queue.producer,
                    site: false,
                },
            );
        }
    }
    Ok(output)
}
fn cumulative(state: &AuditState, job: &Job, done: u64) -> Result<Vec<CostLine>, String> {
    if job.total == 0 || done > job.total {
        return Err("paid-work progress is outside its total".into());
    }
    let definition = recipe(state, &job.recipe)?;
    if definition.payment != PaymentPolicy::ChargeOverTime {
        return Err("paid-work funding on another payment policy".into());
    }
    Ok(definition
        .costs
        .iter()
        .filter_map(|cost| {
            let amount =
                (u128::from(cost.amount) * u128::from(done) / u128::from(job.total)) as u64;
            (amount != 0).then(|| CostLine {
                source: match cost.source {
                    CostSource::SlotBalance => EventSource::Slot(job.owner),
                    CostSource::ProducerInventory => EventSource::Inventory(job.source),
                },
                resource: cost.resource.clone(),
                amount,
            })
        })
        .collect())
}
fn accounts(state: &AuditState) -> Amounts {
    let mut output = Amounts::new();
    for row in &state.economy.balances {
        output.insert(
            (Destination::Slot(row.slot), row.resource.clone()),
            u128::from(row.on_hand),
        );
    }
    for inventory in &state.economy.inventories {
        for row in &inventory.items {
            output.insert(
                (Destination::Inventory(inventory.unit), row.resource.clone()),
                u128::from(row.on_hand),
            );
        }
    }
    for inventory in &state.raw_output_inventories {
        for (resource, account) in &inventory.items {
            output.insert(
                (Destination::Output(inventory.unit), resource.clone()),
                u128::from(account.on_hand),
            );
        }
    }
    output
}
fn delta(old: &Amounts, new: &Amounts) -> Result<Amounts, String> {
    let mut changes = Amounts::new();
    for (key, amount) in new {
        let old = old.get(key).copied().unwrap_or(0);
        let change = amount
            .checked_sub(old)
            .ok_or("paid charged amount went backwards")?;
        if change != 0 {
            changes.insert(key.clone(), change);
        }
    }
    for (key, amount) in old {
        if *amount != 0 && !new.contains_key(key) {
            return Err("paid charged row disappeared".into());
        }
    }
    Ok(changes)
}

pub(crate) fn validate_state(state: &AuditState) -> Result<(), String> {
    for job in jobs(state)?.values() {
        if sum(&job.charged)? != sum(&cumulative(state, job, job.done)?)? {
            return Err("paid-work charged totals differ from cumulative catalog floors".into());
        }
        let paid = sum(&job.charged)?;
        for (key, amount) in sum(&job.funding.pending_charges)? {
            if amount > paid.get(&key).copied().unwrap_or(0) {
                return Err("paid-work pending buffer exceeds actual charged amount".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_operation(operation: &OperationAudit) -> Result<(), String> {
    use turts_sim::EconomyEventKind;
    let before = jobs(&operation.before)?;
    let after = jobs(&operation.after)?;
    let mut debits = Amounts::new();
    let mut involved = false;
    for (key, previous) in &before {
        let Some(next) = after.get(key) else { continue };
        if previous.done == next.done
            && previous.funding == next.funding
            && previous.charged == next.charged
        {
            continue;
        }
        involved = true;
        if previous.total != next.total
            || previous.recipe != next.recipe
            || previous.source != next.source
            || previous.site != next.site
        {
            return Err("paid-work job identity changed".into());
        }
        let differences = delta(
            &sum(&cumulative(&operation.before, previous, previous.done)?)?,
            &sum(&cumulative(&operation.before, previous, next.done)?)?,
        )?;
        for (key, amount) in &differences {
            *debits.entry(key.clone()).or_default() += *amount;
        }
        let mut pending = sum(&previous.funding.pending_charges)?;
        for (key, amount) in &differences {
            *pending.entry(key.clone()).or_default() += *amount;
        }
        let flush = operation
            .events
            .iter()
            .filter_map(|event| {
                if let EconomyEventKind::ProgressCharged { unit, job, charged } = &event.kind {
                    (*unit == previous.unit && *job == previous.job).then_some(charged)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if flush.len() > 1 {
            return Err("paid-work flush is duplicated".into());
        }
        if let Some(flushed) = flush.first() {
            if sum(flushed)? != pending {
                return Err("paid-work flush differs from buffered intermediate debits".into());
            }
            pending.clear();
        }
        if sum(&next.funding.pending_charges)? != pending {
            return Err("paid-work pending buffer does not explain its actual debit".into());
        }
        let mut expected = previous.funding.clone();
        expected.pending_charges = next.funding.pending_charges.clone();
        if next.done > previous.done {
            expected.waiting_for_funds = None;
            if expected.held_since_tick.is_some() {
                expected.recovery_ticks = expected
                    .recovery_ticks
                    .checked_add(1)
                    .ok_or("paid recovery overflow")?;
                if expected.recovery_ticks
                    >= u32::from(
                        operation
                            .before
                            .tick_hz
                            .ok_or("paid work has no tick rate")?,
                    )
                {
                    expected.recovery_ticks = 0;
                    expected.held_since_tick = None;
                }
            }
        } else if let Some(resource) = &next.funding.waiting_for_funds {
            expected.waiting_for_funds = Some(resource.clone());
            expected
                .held_since_tick
                .get_or_insert(operation.before.economy.tick);
            expected.recovery_ticks = 0;
        }
        if expected != next.funding {
            return Err(
                "paid-work hold/recovery state differs from advancing or waiting tick".into(),
            );
        }
    }
    // Destruction/refund and completion operations are handled separately. The
    // intermediate debit path must have no unrelated ledger changes.
    if involved
        && before.keys().eq(after.keys())
        && operation
            .events
            .iter()
            .all(|event| matches!(event.kind, EconomyEventKind::ProgressCharged { .. }))
    {
        let old = accounts(&operation.before);
        let actual = accounts(&operation.after);
        let mut expected = old;
        for (key, amount) in debits {
            let balance = expected.entry(key).or_default();
            *balance = balance
                .checked_sub(amount)
                .ok_or("paid-work debit exceeds account")?;
        }
        expected.retain(|_, value| *value != 0);
        let actual = actual
            .into_iter()
            .filter(|(_, value)| *value != 0)
            .collect::<Amounts>();
        if expected != actual {
            return Err("paid-work ledger differs from cumulative progress debit".into());
        }
    }
    Ok(())
}
