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
    for (&unit, site) in &state.economy.construction.sites {
        let Some(funding) = &site.funding else {
            continue;
        };
        let charged = site
            .charged_rows()
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
            (unit, site.job.0),
            Job {
                unit,
                job: site.job.0,
                owner: site.owner_slot,
                recipe: site.recipe.clone(),
                done: site.done,
                total: site.total,
                charged,
                funding: funding.clone(),
                source: site.constructor,
                site: true,
            },
        );
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
    validate_repair_state(state)?;
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
    validate_repair_operation(operation)?;
    let before = jobs(&operation.before)?;
    let after = jobs(&operation.after)?;
    let mut debits = Amounts::new();
    let mut involved = false;
    for (key, previous) in &before {
        let terminal;
        let removed = !after.contains_key(key);
        let next = if let Some(next) = after.get(key) {
            next
        } else {
            let completed=operation.events.iter().any(|event|matches!(event.kind,EconomyEventKind::ProductionCompleted{producer,job,..} if producer==previous.unit && job==previous.job))
                || previous.site && operation.after.economy.construction.completed.contains(&previous.unit);
            let cancelled=operation.events.iter().any(|event|matches!(event.kind,EconomyEventKind::ProductionCancelled{producer,job,..} if producer==previous.unit && job==previous.job));
            let destroyed=operation.events.iter().any(|event|matches!(event.kind,EconomyEventKind::Destroyed{unit,..} if unit==previous.unit));
            if !completed && !cancelled && !destroyed {
                return Err(
                    "paid-work job disappeared without completion/cancel/destruction".into(),
                );
            }
            terminal = Job {
                done: if completed {
                    previous.total
                } else {
                    previous.done
                },
                funding: FundingState {
                    pending_charges: Vec::new(),
                    ..previous.funding.clone()
                },
                ..previous.clone()
            };
            &terminal
        };
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
        if !removed && expected != next.funding {
            return Err(
                "paid-work hold/recovery state differs from advancing or waiting tick".into(),
            );
        }
    }
    let refund_credits = validate_refunds(operation)?;
    involved |= !refund_credits.is_empty();
    if involved {
        let old = accounts(&operation.before);
        let actual = accounts(&operation.after);
        let mut expected = old;
        for (key, amount) in debits {
            let balance = expected.entry(key).or_default();
            *balance = balance
                .checked_sub(amount)
                .ok_or("paid-work debit exceeds account")?;
        }
        for (key, amount) in refund_credits {
            *expected.entry(key).or_default() += amount;
        }
        // Removing an inventory destroys its contents; this is separately
        // represented by Destroyed, rather than an unexplained debit.
        expected.retain(|(destination, _), _| match destination {
            Destination::Slot(_) => true,
            Destination::Inventory(unit) | Destination::Output(unit) => operation
                .after
                .raw_units
                .iter()
                .any(|row| row.unit_id == *unit),
        });
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

fn unit_definition(
    state: &AuditState,
    unit: u32,
) -> Result<&turts_sim::economy::views::CatalogUnitAudit, String> {
    let raw = state
        .raw_units
        .iter()
        .find(|row| row.unit_id == unit)
        .ok_or("repair unit is missing")?;
    let kind = state
        .unit_types
        .iter()
        .find(|kind| kind.type_id == raw.unit_type)
        .ok_or("repair unit type missing")?;
    state
        .catalog
        .as_ref()
        .and_then(|catalog| {
            catalog
                .units
                .iter()
                .find(|row| row.id.to_string() == kind.content_id)
        })
        .ok_or_else(|| "repair unit definition missing".into())
}
fn validate_repair_state(state: &AuditState) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for row in &state.raw_repairs {
        if !seen.insert(row.repairer) {
            return Err("repair audit repeats a source".into());
        }
        let definition = unit_definition(state, row.repairer)?
            .repairer
            .as_ref()
            .ok_or("repair state belongs to a unit without capability")?;
        let mut resources = std::collections::BTreeSet::new();
        for remainder in &row.remainders {
            let cost = definition
                .cost
                .iter()
                .find(|cost| cost.resource == remainder.resource.to_string())
                .ok_or("repair remainder has no authored cost")?;
            if !resources.insert(remainder.resource.clone())
                || cost.per_health == 0
                || remainder.remainder >= cost.per_health
            {
                return Err("repair remainder is duplicated or outside denominator".into());
            }
        }
        if let Some(action) = &row.action {
            if !state
                .raw_units
                .iter()
                .any(|unit| unit.unit_id == action.target)
                || state
                    .economy
                    .construction
                    .sites
                    .contains_key(&action.target)
            {
                return Err("repair action names missing or unfinished target".into());
            }
            if let turts_sim::economy::repair::RepairStatus::WaitingForFunds(resource) =
                &action.state
            {
                if !definition
                    .cost
                    .iter()
                    .any(|cost| cost.resource == resource.to_string())
                {
                    return Err("repair funding resource is not an authored balance cost".into());
                }
            }
        }
    }
    Ok(())
}

fn validate_repair_operation(operation: &OperationAudit) -> Result<(), String> {
    use turts_sim::economy::repair::{RepairRemainder, RepairStatus};
    let mut repair_debits = Amounts::new();
    let mut healings = BTreeMap::<u32, u64>::new();
    let mut advanced = false;
    for next in &operation.after.raw_repairs {
        let previous = operation
            .before
            .raw_repairs
            .iter()
            .find(|row| row.repairer == next.repairer);
        let old_remainders = previous
            .map(|row| row.remainders.as_slice())
            .unwrap_or_default();
        let old_action = previous.and_then(|row| row.action.as_ref());
        let new_action = next.action.as_ref();
        let advancement = match (old_action, new_action) {
            (Some(old), Some(new))
                if old.target == new.target
                    && old.automatic == new.automatic
                    && new.elapsed != old.elapsed =>
            {
                Some((old, new))
            }
            _ => None,
        };
        let Some((old, action)) = advancement else {
            if old_remainders != next.remainders.as_slice() {
                return Err("repair Stop, retarget or wait reset fractional cost debt".into());
            }
            continue;
        };
        advanced = true;
        if old.elapsed.checked_add(1) != Some(action.elapsed)
            || action.state != RepairStatus::Working
            || next.hold.is_some()
        {
            return Err("repair clock advances by other than one eligible working tick".into());
        }
        let definition = unit_definition(&operation.before, next.repairer)?
            .repairer
            .as_ref()
            .ok_or("repair advancement has no capability")?;
        let target = unit_definition(&operation.before, action.target)?;
        let max = target
            .health_max
            .ok_or("repair target has no captured health maximum")?;
        let health = operation
            .before
            .raw_healths
            .iter()
            .find(|row| row.unit == action.target)
            .ok_or("repair target has no health")?
            .current;
        if health == 0 || health >= max {
            return Err("repair advances on dead or full target".into());
        }
        let hz = u128::from(
            operation
                .before
                .tick_hz
                .filter(|hz| *hz != 0)
                .ok_or("repair has no tick rate")?,
        );
        let healed = (u128::from(definition.rate_per_second) * u128::from(action.elapsed) / hz)
            - (u128::from(definition.rate_per_second) * u128::from(old.elapsed) / hz);
        let applied = u64::try_from(healed.min(u128::from(max - health)))
            .map_err(|_| "repair heal overflow")?;
        *healings.entry(action.target).or_default() += applied;
        let owner = operation
            .before
            .raw_units
            .iter()
            .find(|unit| unit.unit_id == next.repairer)
            .ok_or("repair source missing")?
            .owner_slot;
        let mut remainders = old_remainders
            .iter()
            .map(|row| (row.resource.clone(), row.remainder))
            .collect::<BTreeMap<_, _>>();
        for cost in &definition.cost {
            let resource =
                ContentId::parse(&cost.resource).map_err(|_| "repair cost reference invalid")?;
            if cost.per_health == 0 {
                return Err("repair cost denominator is zero".into());
            }
            let numerator = u128::from(applied) * u128::from(cost.amount)
                + u128::from(remainders.get(&resource).copied().unwrap_or(0));
            let debit = numerator / u128::from(cost.per_health);
            remainders.insert(
                resource.clone(),
                (numerator % u128::from(cost.per_health)) as u64,
            );
            *repair_debits
                .entry((Destination::Slot(owner), resource))
                .or_default() += debit;
        }
        let expected = remainders
            .into_iter()
            .map(|(resource, remainder)| RepairRemainder {
                resource,
                remainder,
            })
            .collect::<Vec<_>>();
        if next.remainders != expected {
            return Err("repair remainder differs from applied healing cost fraction".into());
        }
    }
    if advanced {
        let mut expected = accounts(&operation.before);
        for (key, amount) in repair_debits {
            let account = expected.entry(key).or_default();
            *account = account
                .checked_sub(amount)
                .ok_or("repair debit exceeds balance")?;
        }
        expected.retain(|_, amount| *amount != 0);
        let mut actual = accounts(&operation.after);
        actual.retain(|_, amount| *amount != 0);
        if expected != actual {
            return Err("repair ledger differs from cost of applied healing".into());
        }
        let mut expected = operation.before.raw_healths.clone();
        for (unit, applied) in healings {
            let health = expected
                .iter_mut()
                .find(|row| row.unit == unit)
                .ok_or("repair target health missing")?;
            health.current = health
                .current
                .checked_add(applied)
                .ok_or("repair health overflow")?;
        }
        if expected != operation.after.raw_healths {
            return Err("repair health differs from capped fractional healing".into());
        }
    }
    Ok(())
}

fn refund_rows(
    state: &AuditState,
    unit: u32,
    job_filter: Option<u64>,
    destroyed: bool,
) -> Result<Vec<CostLine>, String> {
    let mut result = Vec::new();
    let catalog = state.catalog.as_ref().ok_or("refund has no catalog")?;
    for queue in state
        .raw_queues
        .iter()
        .filter(|queue| queue.producer == unit)
    {
        for job in queue
            .jobs
            .iter()
            .filter(|job| job_filter.is_none_or(|id| id == job.id))
        {
            let (refunds, payment) = if let Some(recipe) = catalog
                .recipes
                .iter()
                .find(|recipe| recipe.id == job.recipe)
            {
                (recipe.refunds, recipe.payment)
            } else if let Some(technology) = catalog
                .technologies
                .iter()
                .find(|tech| tech.id == job.recipe)
            {
                (technology.refunds, technology.payment)
            } else {
                return Err("refund job definition missing".into());
            };
            let ratio = if destroyed {
                refunds.destroyed
            } else {
                match job.stage {
                    turts_sim::economy::production::JobStage::Queued => refunds.queued,
                    turts_sim::economy::production::JobStage::Running => refunds.running,
                    turts_sim::economy::production::JobStage::OutputBlocked => {
                        refunds.output_blocked
                    }
                }
            };
            if job.stage == turts_sim::economy::production::JobStage::Queued
                && payment != PaymentPolicy::ChargeOnQueue
            {
                continue;
            }
            if ratio.denominator == 0 {
                return Err("refund ratio denominator is zero".into());
            }
            for charged in &job.charged {
                result.push(CostLine {
                    source: match charged.endpoint {
                        ReservationEndpointView::Slot(id) => EventSource::Slot(id),
                        ReservationEndpointView::Inventory(id) => EventSource::Inventory(id),
                        ReservationEndpointView::Output(id) => EventSource::Output(id),
                    },
                    resource: charged.resource.clone(),
                    amount: (u128::from(charged.amount) * u128::from(ratio.numerator)
                        / u128::from(ratio.denominator)) as u64,
                });
            }
        }
    }
    if let Some(site) = state.economy.construction.sites.get(&unit)
        && job_filter.is_none_or(|id| id == site.job.0)
    {
        let definition = recipe(state, &site.recipe)?;
        let ratio = if destroyed {
            definition.refunds.destroyed
        } else {
            definition.refunds.running
        };
        if ratio.denominator == 0 {
            return Err("site refund ratio denominator is zero".into());
        }
        for charged in site.charged_rows() {
            result.push(CostLine {
                source: match charged.endpoint {
                    ReservationEndpointView::Slot(id) => EventSource::Slot(id),
                    ReservationEndpointView::Inventory(id) => EventSource::Inventory(id),
                    ReservationEndpointView::Output(id) => EventSource::Output(id),
                },
                resource: charged.resource,
                amount: (u128::from(charged.amount) * u128::from(ratio.numerator)
                    / u128::from(ratio.denominator)) as u64,
            });
        }
    }
    Ok(result)
}
fn validate_refunds(operation: &OperationAudit) -> Result<Amounts, String> {
    use turts_sim::EconomyEventKind;
    let mut credits = Amounts::new();
    for event in &operation.events {
        match &event.kind {
            EconomyEventKind::ProductionCancelled {
                producer,
                job,
                refunded,
                ..
            } => {
                let expected = refund_rows(&operation.before, *producer, Some(*job), false)?;
                if sum(&expected)? != sum(refunded)? {
                    return Err("manual refund differs from actual paid stage ratio".into());
                }
                for (key, amount) in sum(&expected)? {
                    *credits.entry(key).or_default() += amount;
                }
            }
            EconomyEventKind::DestructionRefunded {
                unit,
                desired,
                credited,
                discarded,
            } => {
                let expected = refund_rows(&operation.before, *unit, None, true)?;
                // Preserve per-cost desired multiplicity. Sorting only changes
                // serialization order, never floors an already aggregated cost.
                let rows = |rows: &[CostLine]| {
                    let mut rows = rows
                        .iter()
                        .map(|row| (event_key(row), row.amount))
                        .collect::<Vec<_>>();
                    rows.sort();
                    rows
                };
                if rows(&expected) != rows(desired) {
                    return Err(
                        "destruction desired refunds differ from per-cost actual-paid floors"
                            .into(),
                    );
                }
                let desired = sum(&expected)?;
                let old = accounts(&operation.before);
                let mut expected_credits = Amounts::new();
                let mut destination_batches = BTreeMap::<Destination, Amounts>::new();
                for (key, amount) in &desired {
                    destination_batches
                        .entry(key.0.clone())
                        .or_default()
                        .insert(key.clone(), *amount);
                }
                for (destination, batch) in destination_batches {
                    match destination {
                        Destination::Slot(slot) => {
                            for (key, amount) in batch {
                                if operation
                                    .before
                                    .economy
                                    .balances
                                    .iter()
                                    .any(|row| row.slot == slot && row.resource == key.1)
                                {
                                    expected_credits.insert(
                                        key.clone(),
                                        amount.min(
                                            u128::from(u64::MAX)
                                                - old.get(&key).copied().unwrap_or(0),
                                        ),
                                    );
                                }
                            }
                        }
                        Destination::Inventory(unit) => {
                            if let Some(inventory) = operation
                                .before
                                .economy
                                .inventories
                                .iter()
                                .find(|row| row.unit == unit)
                                && operation
                                    .after
                                    .raw_units
                                    .iter()
                                    .any(|row| row.unit_id == unit)
                                && operation
                                    .after
                                    .economy
                                    .inventories
                                    .iter()
                                    .any(|row| row.unit == unit)
                            {
                                let contents = inventory
                                    .items
                                    .iter()
                                    .map(|row| u128::from(row.on_hand))
                                    .sum::<u128>();
                                if contents + batch.values().sum::<u128>()
                                    <= u128::from(inventory.capacity)
                                {
                                    expected_credits.extend(batch);
                                }
                            }
                        }
                        Destination::Output(_) => {
                            return Err("refund uses unsupported output destination".into());
                        }
                    }
                }
                let mut expected_discard = Amounts::new();
                for (key, amount) in desired {
                    expected_discard.insert(
                        key.clone(),
                        amount - expected_credits.get(&key).copied().unwrap_or(0),
                    );
                }
                let positive = |mut rows: Amounts| {
                    rows.retain(|_, amount| *amount != 0);
                    rows
                };
                if positive(expected_credits.clone()) != positive(sum(credited)?)
                    || positive(expected_discard) != positive(sum(discarded)?)
                {
                    return Err("destruction refund credited/discarded split differs from destination headroom".into());
                }
                for (key, amount) in expected_credits {
                    *credits.entry(key).or_default() += amount;
                }
            }
            _ => {}
        }
    }
    Ok(credits)
}
