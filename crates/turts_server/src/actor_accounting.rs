//! Immutable completed-basis accounting for trusted participant receipts.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use turts_bot::{
    AccountAmount, AccountObservation, ActorResult, BotActor, EmittedCommand, InventoryPort,
    ObservedRequestEffect, ReconcileBatch, RequestKey, SourceEndpoint, SpendKey, Watermark,
    WorkIdentity, WorkKind, WorkObservation, WorkState,
};
use turts_mod_api::{ContentCatalog, ContentId, CostSource, PaymentPolicy, ResourceKind};
use turts_sim::{
    CommittedTick, EconomyEventKind, EventSource,
    economy::views::{ReservationEndpointView, ReservationLineView, WorldView},
    receipts::AdmittedWork,
};

type Amounts = BTreeMap<SpendKey, u64>;

#[derive(Debug, Clone)]
pub(crate) struct TrackedRequest {
    pub command: Arc<EmittedCommand>,
    pub result: Option<ActorResult>,
    pub admitted_work: Option<AdmittedWork>,
    accounted: Option<Watermark>,
    flushed: Amounts,
    started: Amounts,
    last_effect: Option<ObservedRequestEffect>,
}
impl TrackedRequest {
    pub(crate) fn new(command: impl Into<Arc<EmittedCommand>>) -> Self {
        Self {
            command: command.into(),
            result: None,
            admitted_work: None,
            accounted: None,
            flushed: BTreeMap::new(),
            started: BTreeMap::new(),
            last_effect: None,
        }
    }
    pub(crate) fn finalize(&mut self, result: ActorResult, work: Option<AdmittedWork>) {
        self.result = Some(result);
        self.admitted_work = work;
    }
}

/// Prepare a complete account/receipt basis before advancing any tracked state.
/// A failed bound or consistency check leaves every request unchanged.
pub(crate) fn build_batch(
    view: &WorldView,
    committed: &CommittedTick,
    catalog: &ContentCatalog,
    actor: &BotActor,
    requests: &mut BTreeMap<RequestKey, TrackedRequest>,
    watermark: Watermark,
    row_limit: usize,
) -> Result<ReconcileBatch, String> {
    validate_completed_input(view, committed, watermark)?;
    let mut budget = RowBudget(row_limit);
    #[cfg(test)]
    let account_started = crate::bot_profile::start();
    let account_result = account_rows(view, catalog, actor, &mut budget);
    #[cfg(test)]
    crate::bot_profile::finish("publication.accounts", account_started);
    let accounts = account_result?;
    #[cfg(test)]
    let clone_started = crate::bot_profile::start();
    let mut staged = requests.clone();
    #[cfg(test)]
    crate::bot_profile::finish("publication.clone_requests", clone_started);
    let mut included = Vec::new();
    let mut unapplied = Vec::new();
    #[cfg(test)]
    let capture_started = crate::bot_profile::start();
    for (key, tracked) in &mut staged {
        budget.take(1)?;
        let Some(effect) =
            capture_request(view, committed, catalog, actor, key, tracked, watermark)?
        else {
            unapplied.push(key.clone());
            continue;
        };
        budget.take(effect.paid.len())?;
        budget.take(effect.reserved.len())?;
        if let Some(work) = &effect.work {
            budget.take(1)?;
            budget.take(work.paid.len())?;
            budget.take(work.reserved.len())?;
        }
        included.push(effect);
    }
    #[cfg(test)]
    crate::bot_profile::finish("publication.capture", capture_started);
    #[cfg(test)]
    let commit_started = crate::bot_profile::start();
    *requests = staged;
    #[cfg(test)]
    crate::bot_profile::finish("publication.commit_drop", commit_started);
    Ok(ReconcileBatch {
        watermark,
        accounts,
        included,
        unapplied,
        complete: true,
    })
}
/// Preserve exact current receipt facts even when the separate account-row
/// publication is over budget. This stores one cumulative effect per retained
/// request, not historical ticks or complete account/world snapshots.
pub(crate) fn capture_receipts(
    view: &WorldView,
    committed: &CommittedTick,
    catalog: &ContentCatalog,
    actor: &BotActor,
    requests: &mut BTreeMap<RequestKey, TrackedRequest>,
    watermark: Watermark,
) -> Result<(), String> {
    validate_completed_input(view, committed, watermark)?;
    let mut staged = requests.clone();
    for (key, tracked) in &mut staged {
        capture_request(view, committed, catalog, actor, key, tracked, watermark)?;
    }
    *requests = staged;
    Ok(())
}
fn validate_completed_input(
    view: &WorldView,
    committed: &CommittedTick,
    watermark: Watermark,
) -> Result<(), String> {
    if view.tick != committed.tick || watermark.completed_tick != view.tick {
        return Err("accounting view, events and watermark are not one completed tick".into());
    }
    if committed
        .events
        .iter()
        .any(|event| event.tick != committed.tick)
        || !committed
            .events
            .windows(2)
            .all(|pair| pair[0].ordinal < pair[1].ordinal)
    {
        return Err("accounting events do not belong to one ordered completed tick".into());
    }
    Ok(())
}
fn capture_request(
    view: &WorldView,
    committed: &CommittedTick,
    catalog: &ContentCatalog,
    actor: &BotActor,
    key: &RequestKey,
    tracked: &mut TrackedRequest,
    watermark: Watermark,
) -> Result<Option<ObservedRequestEffect>, String> {
    if key.actor != *actor
        || tracked.command.actor != *actor
        || key.sequence != tracked.command.sequence
        || !tracked.command.units.contains(&key.unit)
    {
        return Err("tracked request differs from its trusted actor command".into());
    }
    if tracked.accounted.is_some_and(|old| watermark < old) {
        return Err("receipt accounting watermark moved backwards".into());
    }
    let Some(result) = tracked
        .result
        .clone()
        .filter(|result| result.effect_watermark <= watermark)
    else {
        if tracked.last_effect.is_some() {
            return Err("represented request lost its admission result".into());
        }
        return Ok(None);
    };
    if result.request != *key {
        return Err("receipt result names a different request".into());
    }
    let effect = if !result.accepted {
        if tracked.admitted_work.is_some() || result.work.is_some() {
            return Err("refused request unexpectedly owns admitted work".into());
        }
        ObservedRequestEffect::refusal(key.clone(), result.effect_watermark)
    } else {
        prepare_effect(view, committed, catalog, actor, key, tracked, &result)?
    };
    if tracked.accounted == Some(watermark) && tracked.last_effect.as_ref() != Some(&effect) {
        return Err("one accounting watermark describes different receipt effects".into());
    }
    validate_advance(tracked.last_effect.as_ref(), &effect)?;
    tracked.accounted = Some(watermark);
    tracked.last_effect = Some(effect.clone());
    Ok(Some(effect))
}
pub(crate) const ROW_LIMIT_EXCEEDED: &str = "completed accounting row limit exceeded";
struct RowBudget(usize);
impl RowBudget {
    fn take(&mut self, rows: usize) -> Result<(), String> {
        self.0 = self.0.checked_sub(rows).ok_or(ROW_LIMIT_EXCEEDED)?;
        Ok(())
    }
}
fn account_rows(
    view: &WorldView,
    catalog: &ContentCatalog,
    actor: &BotActor,
    budget: &mut RowBudget,
) -> Result<Vec<AccountObservation>, String> {
    let mut rows = BTreeMap::<SpendKey, (u64, u64)>::new();
    // Installed Balance resources are valid zero accounts even when absent from
    // a sparse ledger view. No foreign slot is inspected or exposed.
    for (id, resource) in &catalog.resources {
        if resource.kind == ResourceKind::Balance {
            insert_account(
                &mut rows,
                SpendKey {
                    endpoint: SourceEndpoint::SlotBalance(actor.slot),
                    resource: id.clone(),
                },
                (0, 0),
                budget,
            )?;
        }
    }
    let mut seen = BTreeSet::new();
    for balance in view.balances.iter().filter(|row| row.slot == actor.slot) {
        let key = SpendKey {
            endpoint: SourceEndpoint::SlotBalance(actor.slot),
            resource: balance.resource.clone(),
        };
        if !seen.insert(key.clone()) {
            return Err("duplicate own balance account".into());
        }
        set_account(&mut rows, key, (balance.on_hand, balance.reserved), budget)?;
    }
    // Resolve only inventories that actually exist in this completed view. Most
    // combat units have no inventory; indexing and parsing every owned unit made
    // account publication scale with the whole army instead of its accounts.
    let mut accepts_by_content = BTreeMap::<&str, &[ContentId]>::new();
    for inventory in view
        .inventories
        .iter()
        .filter(|row| row.owner_slot == actor.slot)
    {
        let Some(unit) =
            completed_unit(view, inventory.unit).filter(|unit| unit.owner_slot == actor.slot)
        else {
            // A removed original payment source is absent, never a zero account.
            continue;
        };
        let accepts = if let Some(accepts) = accepts_by_content.get(unit.content_id.as_str()) {
            *accepts
        } else {
            // Borrow qualified identity parts. Do not allocate a ContentId for
            // every entity, or copy immutable accepted-item lists.
            let (package, local) = unit
                .content_id
                .split_once(':')
                .ok_or("own inventory unit has an invalid content identifier")?;
            let definition = catalog
                .units
                .iter()
                .find_map(|(id, definition)| {
                    (id.package.0 == package && id.local == local).then_some(definition)
                })
                .ok_or("own inventory unit definition is absent from the installed catalog")?;
            let accepts = definition
                .inventory
                .as_ref()
                .map_or(&[][..], |inventory| inventory.accepts.as_slice());
            accepts_by_content.insert(unit.content_id.as_str(), accepts);
            accepts
        };
        for resource in accepts {
            insert_account(
                &mut rows,
                SpendKey {
                    endpoint: SourceEndpoint::Inventory {
                        unit: inventory.unit,
                        port: InventoryPort::Main,
                    },
                    resource: resource.clone(),
                },
                (0, 0),
                budget,
            )?;
        }
        for item in &inventory.items {
            let key = SpendKey {
                endpoint: SourceEndpoint::Inventory {
                    unit: inventory.unit,
                    port: InventoryPort::Main,
                },
                resource: item.resource.clone(),
            };
            if !seen.insert(key.clone()) {
                return Err("duplicate own inventory account".into());
            }
            set_account(&mut rows, key, (item.on_hand, item.reserved), budget)?;
        }
    }
    for inventory in &view.m3.raw_output_inventories {
        if !completed_unit(view, inventory.unit).is_some_and(|unit| unit.owner_slot == actor.slot) {
            continue;
        }
        for resource in &inventory.accepted_items {
            insert_account(
                &mut rows,
                SpendKey {
                    endpoint: SourceEndpoint::Inventory {
                        unit: inventory.unit,
                        port: InventoryPort::Output,
                    },
                    resource: resource.clone(),
                },
                (0, 0),
                budget,
            )?;
        }
        for (resource, amount) in &inventory.items {
            let key = SpendKey {
                endpoint: SourceEndpoint::Inventory {
                    unit: inventory.unit,
                    port: InventoryPort::Output,
                },
                resource: resource.clone(),
            };
            if !seen.insert(key.clone()) {
                return Err("duplicate own output account".into());
            }
            set_account(&mut rows, key, (amount.on_hand, amount.reserved), budget)?;
        }
    }
    Ok(rows
        .into_iter()
        .map(|(account, (on_hand, reserved))| AccountObservation {
            account,
            on_hand,
            reserved,
        })
        .collect())
}
fn completed_unit(view: &WorldView, id: u32) -> Option<&turts_sim::world::UnitState> {
    // Authoritative completed rows are sorted. The fallback preserves the helper's
    // behavior for valid synthetic views without that presentation ordering.
    view.units
        .binary_search_by_key(&id, |unit| unit.unit_id)
        .ok()
        .map(|index| &view.units[index])
        .or_else(|| view.units.iter().find(|unit| unit.unit_id == id))
}
fn insert_account(
    rows: &mut BTreeMap<SpendKey, (u64, u64)>,
    key: SpendKey,
    amount: (u64, u64),
    budget: &mut RowBudget,
) -> Result<(), String> {
    if let std::collections::btree_map::Entry::Vacant(e) = rows.entry(key) {
        budget.take(1)?;
        e.insert(amount);
    }
    Ok(())
}
fn set_account(
    rows: &mut BTreeMap<SpendKey, (u64, u64)>,
    key: SpendKey,
    amount: (u64, u64),
    budget: &mut RowBudget,
) -> Result<(), String> {
    if amount.1 > amount.0 {
        return Err("reserved account amount exceeds its balance".into());
    }
    if !rows.contains_key(&key) {
        budget.take(1)?;
    }
    rows.insert(key, amount);
    Ok(())
}
fn endpoint(endpoint: &ReservationEndpointView) -> SourceEndpoint {
    match endpoint {
        ReservationEndpointView::Slot(slot) => SourceEndpoint::SlotBalance(*slot),
        ReservationEndpointView::Inventory(unit) => SourceEndpoint::Inventory {
            unit: *unit,
            port: InventoryPort::Main,
        },
        ReservationEndpointView::Output(unit) => SourceEndpoint::Inventory {
            unit: *unit,
            port: InventoryPort::Output,
        },
    }
}
fn reservation_amounts(rows: &[ReservationLineView]) -> Result<Amounts, String> {
    collect_amounts(rows.iter().map(|line| {
        (
            SpendKey {
                endpoint: endpoint(&line.endpoint),
                resource: line.resource.clone(),
            },
            line.amount,
        )
    }))
}
fn event_amounts(rows: &[turts_sim::CostLine]) -> Result<Amounts, String> {
    collect_amounts(rows.iter().map(|line| {
        (
            SpendKey {
                endpoint: match line.source {
                    EventSource::Slot(slot) => SourceEndpoint::SlotBalance(slot),
                    EventSource::Inventory(unit) => SourceEndpoint::Inventory {
                        unit,
                        port: InventoryPort::Main,
                    },
                    EventSource::Output(unit) => SourceEndpoint::Inventory {
                        unit,
                        port: InventoryPort::Output,
                    },
                },
                resource: line.resource.clone(),
            },
            line.amount,
        )
    }))
}
fn collect_amounts(rows: impl Iterator<Item = (SpendKey, u64)>) -> Result<Amounts, String> {
    let mut amounts = BTreeMap::new();
    for (key, amount) in rows {
        if amount == 0 {
            continue;
        }
        if amounts.insert(key, amount).is_some() {
            return Err("duplicate payment endpoint/resource row".into());
        }
    }
    Ok(amounts)
}
fn add_amounts(to: &mut Amounts, rows: &Amounts) -> Result<(), String> {
    for (key, amount) in rows {
        let old = to.entry(key.clone()).or_default();
        *old = old
            .checked_add(*amount)
            .ok_or("receipt paid amount overflow")?;
    }
    Ok(())
}
fn rows(amounts: Amounts) -> Vec<AccountAmount> {
    amounts
        .into_iter()
        .map(|(account, amount)| AccountAmount { account, amount })
        .collect()
}
fn snapshot_amounts(rows: &[AccountAmount]) -> Result<Amounts, String> {
    collect_amounts(rows.iter().map(|row| (row.account.clone(), row.amount)))
}
fn validate_advance(
    previous: Option<&ObservedRequestEffect>,
    next: &ObservedRequestEffect,
) -> Result<(), String> {
    if let Some(previous) = previous {
        if previous.effect_watermark != next.effect_watermark
            || previous.refused != next.refused
            || previous.terminal && previous != next
        {
            return Err("stable admission receipt changed after publication".into());
        }
        let paid = snapshot_amounts(&next.paid)?;
        for (key, old) in snapshot_amounts(&previous.paid)? {
            if paid.get(&key).copied().unwrap_or(0) < old {
                return Err("cumulative paid amount moved backwards".into());
            }
        }
        if let (Some(old), Some(work)) = (&previous.work, &next.work)
            && (old.identity != work.identity || old.total != work.total || work.done < old.done)
        {
            return Err("work identity or progress moved backwards".into());
        }
    }
    Ok(())
}

fn prepare_effect(
    view: &WorldView,
    committed: &CommittedTick,
    catalog: &ContentCatalog,
    actor: &BotActor,
    key: &RequestKey,
    tracked: &mut TrackedRequest,
    result: &ActorResult,
) -> Result<ObservedRequestEffect, String> {
    if let Some(effect) = tracked
        .last_effect
        .as_ref()
        .filter(|effect| effect.terminal)
    {
        return Ok(effect.clone());
    }
    let member = turts_sim::receipts::CommandMember {
        slot: actor.slot,
        sequence: key.sequence,
        unit: key.unit,
    };
    let origin_sites = view
        .construction
        .sites
        .iter()
        .filter(|(_, site)| site.origin == Some(member))
        .collect::<Vec<_>>();
    if origin_sites.len() > 1 {
        return Err("one admitted constructor member owns multiple sites".into());
    }
    let placed = origin_sites.first().copied();
    let held = view
        .construction
        .held
        .values()
        .flatten()
        .filter(|held| held.origin == Some(member))
        .count();
    if held > 1 || held > 0 && placed.is_some() {
        return Err("ambiguous held construction origin".into());
    }
    let hint = tracked.admitted_work.as_ref();
    let identity = hint
        .map(|work| WorkIdentity {
            unit: work.unit,
            id: work.job.0,
            kind: if work.construction {
                WorkKind::Construction
            } else if catalog.technologies.contains_key(&work.recipe) {
                WorkKind::Research
            } else {
                WorkKind::Production
            },
        })
        .or_else(|| {
            placed.map(|(id, site)| WorkIdentity {
                unit: *id,
                id: site.job.0,
                kind: WorkKind::Construction,
            })
        });
    let Some(identity) = identity else {
        if result.work.is_some() {
            return Err("accepted work result lost its admission metadata".into());
        }
        return Ok(if held == 1 {
            ObservedRequestEffect::held_placement(key.clone(), result.effect_watermark)
        } else {
            // An ordinary nonproductive order, or a held request cleared by a
            // later Stop/removal, has no monetary effect in this complete basis.
            ObservedRequestEffect {
                request: key.clone(),
                effect_watermark: result.effect_watermark,
                refused: false,
                pending_placement: false,
                paid: vec![],
                reserved: vec![],
                work: None,
                terminal: true,
            }
        });
    };
    if result.work.as_ref().is_some_and(|old| old != &identity) {
        return Err("actual admitted work differs from its original actor result".into());
    }
    let recipe = hint
        .map(|hint| &hint.recipe)
        .or_else(|| placed.map(|(_, site)| &site.recipe))
        .ok_or("admitted work lacks content")?
        .clone();
    let (payment, costs) = if let Some(recipe) = catalog.production.get(&recipe) {
        (recipe.payment, &recipe.costs)
    } else if let Some(technology) = catalog.technologies.get(&recipe) {
        (technology.payment, &technology.costs)
    } else {
        return Err("admitted work content is absent from the locked catalog".into());
    };
    if tracked
        .accounted
        .is_none_or(|old| old.completed_tick < committed.tick)
    {
        for event in committed
            .events
            .iter()
            .filter(|event| event.owner_slot == actor.slot)
        {
            match &event.kind {
                EconomyEventKind::ProgressCharged { unit, job, charged }
                    if *unit == identity.unit && *job == identity.id =>
                {
                    add_amounts(&mut tracked.flushed, &event_amounts(charged)?)?;
                }
                EconomyEventKind::ProductionStarted {
                    producer,
                    job,
                    charged,
                } if *producer == identity.unit && *job == identity.id => {
                    let amounts = event_amounts(charged)?;
                    if !tracked.started.is_empty() && tracked.started != amounts {
                        return Err("production start changed its exact debit rows".into());
                    }
                    tracked.started = amounts;
                }
                _ => {}
            }
        }
    }
    let site = placed.map(|(_, site)| site).or_else(|| {
        view.construction
            .sites
            .get(&identity.unit)
            .filter(|site| site.job.0 == identity.id)
    });
    let queue = view
        .queues
        .iter()
        .find(|queue| queue.producer == identity.unit && queue.owner_slot == actor.slot);
    let job = queue.and_then(|queue| queue.jobs.iter().find(|job| job.id == identity.id));
    let mut work = if identity.kind == WorkKind::Construction {
        if let Some(site) = site {
            if site.owner_slot != actor.slot
                || site.constructor != key.unit
                || site.recipe != recipe
            {
                return Err("site ownership or constructor differs from its receipt".into());
            }
            let paid = reservation_amounts(&site.charged_rows())?;
            check_funding_buffer(payment, &paid, site.funding.as_ref(), tracked)?;
            WorkObservation {
                identity: identity.clone(),
                actor_unit: key.unit,
                content: Some(recipe.clone()),
                payment: Some(payment),
                done: site.done,
                total: site.total,
                paid: rows(paid),
                reserved: vec![],
                state: funding_state(site.funding.as_ref()).unwrap_or(WorkState::Running),
            }
        } else {
            return terminal_work(
                view, committed, catalog, actor, key, tracked, result, &identity, &recipe, payment,
            );
        }
    } else if let Some(job) = job {
        if job.recipe != recipe {
            return Err("queue job content differs from admission".into());
        }
        let paid = collect_amounts(job.charged.iter().map(|cost| {
            (
                cost_key(cost.source, &cost.resource, actor.slot, key.unit),
                cost.amount,
            )
        }))?;
        let reserved = collect_amounts(job.reserved.iter().map(|cost| {
            (
                cost_key(cost.source, &cost.resource, actor.slot, key.unit),
                cost.amount,
            )
        }))?;
        let exact = view.operating.as_ref().and_then(|op| {
            op.jobs
                .iter()
                .find(|row| row.unit == identity.unit && row.job == identity.id)
        });
        if payment == PaymentPolicy::ChargeOverTime && exact.is_none() {
            return Err("paid-over-time job lacks exact scaled work metadata".into());
        }
        check_funding_buffer(
            payment,
            &paid,
            exact.and_then(|row| row.funding.as_ref()),
            tracked,
        )?;
        WorkObservation {
            identity: identity.clone(),
            actor_unit: key.unit,
            content: Some(recipe.clone()),
            payment: Some(payment),
            done: exact.map_or(job.elapsed_ticks, |row| row.done),
            total: exact.map_or(job.duration_ticks, |row| row.total),
            paid: rows(paid),
            reserved: rows(reserved),
            state: funding_state(exact.and_then(|row| row.funding.as_ref())).unwrap_or(
                match job.stage {
                    turts_sim::JobStage::Queued => WorkState::Queued,
                    turts_sim::JobStage::Running => WorkState::Running,
                    turts_sim::JobStage::OutputBlocked => WorkState::OutputBlocked,
                },
            ),
        }
    } else {
        return terminal_work(
            view, committed, catalog, actor, key, tracked, result, &identity, &recipe, payment,
        );
    };
    let declared = collect_amounts(costs.iter().map(|cost| {
        (
            cost_key(cost.source, &cost.resource, actor.slot, key.unit),
            cost.amount,
        )
    }))?;
    validate_admission_metadata(tracked, &work)?;
    validate_paid_work(&work, &declared)?;
    // Exact own enabled/held state is from this completed basis, not the live ECS.
    if !work.state.terminal() && view.m3.disabled.contains(&identity.unit) {
        work.state = WorkState::Paused;
    }
    Ok(ObservedRequestEffect::observed_work(
        key.clone(),
        result.effect_watermark,
        work,
    ))
}
fn funding_state(
    funding: Option<&turts_sim::economy::production::FundingState>,
) -> Option<WorkState> {
    funding.and_then(|funding| {
        funding
            .waiting_for_funds
            .clone()
            .map(WorkState::WaitingForFunds)
    })
}
fn cost_key(source: CostSource, resource: &ContentId, slot: u8, inventory: u32) -> SpendKey {
    SpendKey {
        endpoint: match source {
            CostSource::SlotBalance => SourceEndpoint::SlotBalance(slot),
            CostSource::ProducerInventory => SourceEndpoint::Inventory {
                unit: inventory,
                port: InventoryPort::Main,
            },
        },
        resource: resource.clone(),
    }
}
fn check_funding_buffer(
    payment: PaymentPolicy,
    paid: &Amounts,
    funding: Option<&turts_sim::economy::production::FundingState>,
    tracked: &TrackedRequest,
) -> Result<(), String> {
    if payment != PaymentPolicy::ChargeOverTime {
        return Ok(());
    }
    let funding = funding.ok_or("paid-over-time work lacks its funding buffer")?;
    let mut accounted = tracked.flushed.clone();
    add_amounts(&mut accounted, &event_amounts(&funding.pending_charges)?)?;
    if &accounted != paid {
        return Err("live cumulative paid rows differ from flushed plus pending debits".into());
    }
    Ok(())
}
fn validate_admission_metadata(
    tracked: &TrackedRequest,
    work: &WorkObservation,
) -> Result<(), String> {
    if let Some(admitted) = &tracked.admitted_work {
        if admitted.payment != work.payment
            || admitted.total != work.total
            || admitted.done > work.done
            || admitted.done > admitted.total
        {
            return Err(
                "completed work differs from its exact admission policy or progress".into(),
            );
        }
        let paid = snapshot_amounts(&work.paid)?;
        for (key, amount) in reservation_amounts(&admitted.paid)? {
            if paid.get(&key).copied().unwrap_or(0) < amount {
                return Err("completed work lost an actual admission debit".into());
            }
        }
    }
    Ok(())
}
fn validate_paid_work(work: &WorkObservation, declared: &Amounts) -> Result<(), String> {
    if work.total == 0 || work.done > work.total {
        return Err("invalid exact work progress".into());
    }
    let paid = snapshot_amounts(&work.paid)?;
    let reserved = snapshot_amounts(&work.reserved)?;
    if paid
        .keys()
        .chain(reserved.keys())
        .any(|key| !declared.contains_key(key))
    {
        return Err("paid work contains an unauthored endpoint or resource".into());
    }
    for (key, full) in declared {
        let paid = paid.get(key).copied().unwrap_or(0);
        let reserved = reserved.get(key).copied().unwrap_or(0);
        if paid
            .checked_add(reserved)
            .is_none_or(|amount| amount > *full)
        {
            return Err("paid and reserved work exceeds its authored cost".into());
        }
        if work.payment == Some(PaymentPolicy::ChargeOverTime)
            && (reserved != 0
                || paid != turts_rules::charge::charged_at(*full, work.done, work.total))
        {
            return Err("paid-over-time work differs from its exact progress charge".into());
        }
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn terminal_work(
    _view: &WorldView,
    committed: &CommittedTick,
    catalog: &ContentCatalog,
    actor: &BotActor,
    key: &RequestKey,
    tracked: &TrackedRequest,
    result: &ActorResult,
    identity: &WorkIdentity,
    recipe: &ContentId,
    payment: PaymentPolicy,
) -> Result<ObservedRequestEffect, String> {
    let matches = committed
        .terminal_work
        .iter()
        .filter(|row| row.unit == identity.unit && row.job == identity.id)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("terminal work needs one immutable exact completion/removal receipt".into());
    }
    let terminal = matches[0];
    if terminal.recipe != *recipe
        || terminal.payment != Some(payment)
        || terminal.construction != (identity.kind == WorkKind::Construction)
    {
        return Err("terminal work metadata differs from its actual admission".into());
    }
    let paid = reservation_amounts(&terminal.paid)?;
    let reserved = reservation_amounts(&terminal.reserved)?;
    if !reserved.is_empty() {
        return Err("terminal work retained an active reservation".into());
    }
    if payment == PaymentPolicy::ChargeOverTime && tracked.flushed != paid {
        return Err("terminal cumulative paid differs from exact flushed progress debits".into());
    }
    if payment != PaymentPolicy::ChargeOverTime {
        let mut known = tracked.started.clone();
        if let Some(hint) = &tracked.admitted_work {
            for (key, amount) in reservation_amounts(&hint.paid)? {
                if known.get(&key).is_some_and(|old| *old != amount) {
                    return Err("admission and start payments disagree".into());
                }
                known.insert(key, amount);
            }
        }
        if known != paid {
            return Err("terminal paid rows lack their exact admission/start witness".into());
        }
    }
    let costs = catalog
        .production
        .get(recipe)
        .map(|row| &row.costs)
        .or_else(|| catalog.technologies.get(recipe).map(|row| &row.costs))
        .ok_or("terminal work content is absent from the locked catalog")?;
    let declared = collect_amounts(costs.iter().map(|cost| {
        (
            cost_key(cost.source, &cost.resource, actor.slot, key.unit),
            cost.amount,
        )
    }))?;
    let completed = terminal.state == turts_sim::receipts::TerminalWorkState::Completed;
    if completed && terminal.done != terminal.total {
        return Err("completed work does not have full exact progress".into());
    }
    let work = WorkObservation {
        identity: identity.clone(),
        actor_unit: key.unit,
        content: Some(recipe.clone()),
        payment: Some(payment),
        done: terminal.done,
        total: terminal.total,
        paid: rows(paid),
        reserved: vec![],
        state: if completed {
            WorkState::Finished
        } else {
            WorkState::Failed
        },
    };
    validate_admission_metadata(tracked, &work)?;
    validate_paid_work(&work, &declared)?;
    Ok(ObservedRequestEffect::observed_work(
        key.clone(),
        result.effect_watermark,
        work,
    ))
}

#[cfg(test)]
#[path = "actor_accounting_tests.rs"]
mod tests;
