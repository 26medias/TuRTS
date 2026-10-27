use std::collections::BTreeMap;
use turts_mod_api::{ContentCatalog, ContentId, CostSource, PaymentPolicy};
use turts_sim::{
    AuditState, Command, CommandKind, CommandResult, EconomyEvent, EconomyEventKind, JobStage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomaticPhase {
    Gather,
    ProductionAdvance,
    OutputAttempt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomaticSubject {
    Gatherer(u32),
    HeadJob { producer: u32, job: u64 },
}
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedOperation {
    Command(Command),
    InventoryTransfer {
        slot: u8,
        from: u32,
        to: u32,
        item: ContentId,
        amount: u64,
    },
    Automatic {
        tick: u32,
        phase: AutomaticPhase,
        subject: AutomaticSubject,
    },
    TickBoundary {
        tick: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrozenJob {
    recipe: ContentId,
    stage: JobStage,
    elapsed: u64,
    duration: u64,
    charged: Vec<turts_sim::CostView>,
    reserved: Vec<turts_sim::CostView>,
}
impl FrozenJob {
    fn view(&self, id: u64) -> turts_sim::JobView {
        turts_sim::JobView {
            id,
            recipe: self.recipe.clone(),
            stage: self.stage,
            elapsed_ticks: self.elapsed,
            duration_ticks: self.duration,
            charged: self.charged.clone(),
            reserved: self.reserved.clone(),
        }
    }
}

/// A reference model whose expectations are made from the input command and frozen catalog.
/// It deliberately ignores diagnostic operation counters when checking economic truth.
pub struct AccountingOracle {
    initial: AuditState,
    catalog: ContentCatalog,
    last: AuditState,
    tick_start_state: AuditState,
    pub extraction: u128,
    pub delivery: u128,
    pub conversion: u128,
    pub gross_charge: u128,
    pub refunds: u128,
    item_charges: BTreeMap<ContentId, u128>,
    item_refunds: BTreeMap<ContentId, u128>,
    conversion_sinks: BTreeMap<ContentId, u128>,
    balance_charges: BTreeMap<(u8, ContentId), u128>,
    balance_refunds: BTreeMap<(u8, ContentId), u128>,
    balance_conversions: BTreeMap<(u8, ContentId), u128>,
    pub outputs: u64,
    pub cancellations: u64,
    pub operations: u64,
    movement_checked_this_tick: bool,
    expected_commands: Vec<Command>,
    frozen_jobs: BTreeMap<(u32, u64), FrozenJob>,
}
impl AccountingOracle {
    pub fn new(initial: &AuditState, catalog: &ContentCatalog) -> Self {
        Self {
            initial: initial.clone(),
            catalog: catalog.clone(),
            last: initial.clone(),
            tick_start_state: initial.clone(),
            extraction: 0,
            delivery: 0,
            conversion: 0,
            gross_charge: 0,
            refunds: 0,
            item_charges: BTreeMap::new(),
            item_refunds: BTreeMap::new(),
            conversion_sinks: BTreeMap::new(),
            balance_charges: BTreeMap::new(),
            balance_refunds: BTreeMap::new(),
            balance_conversions: BTreeMap::new(),
            outputs: 0,
            cancellations: 0,
            operations: 0,
            movement_checked_this_tick: false,
            expected_commands: Vec::new(),
            frozen_jobs: BTreeMap::new(),
        }
    }

    pub fn observe(
        &mut self,
        expected: &ExpectedOperation,
        before: &AuditState,
        after: &AuditState,
        events: &[EconomyEvent],
        result: Option<&CommandResult>,
    ) -> Result<(), String> {
        if let ExpectedOperation::TickBoundary { tick } = expected {
            self.tick_boundary(*tick, before, after, events)?;
            self.operations += 1;
            self.last = after.clone();
            return Ok(());
        }
        if *before != self.last {
            if matches!(expected, ExpectedOperation::Automatic { .. }) {
                if self.movement_checked_this_tick {
                    return Err(
                        "second movement discontinuity occurred within one modeled tick".into(),
                    );
                }
                if self.last.economy.tick.checked_add(1) != Some(before.economy.tick) {
                    return Err(
                        "automatic before snapshot skipped or repeated a tick boundary".into(),
                    );
                }
                self.movement_transition(&self.last, before)?;
                self.movement_checked_this_tick = true;
                self.last = before.clone();
            } else {
                return Err("operation boundary is discontinuous from prior modeled state".into());
            }
        }
        if before.tick_hz != after.tick_hz
            || before.catalog != after.catalog
            || before.bounds != after.bounds
            || before.compiled_durations != after.compiled_durations
            || before.unit_types != after.unit_types
            || before.resources != after.resources
        {
            return Err("immutable clock, bounds, catalog, durations, type registry, or resource presence changed".into());
        }
        for inventory in &before.raw_inventories {
            if after
                .raw_inventories
                .iter()
                .find(|x| x.unit == inventory.unit)
                != Some(inventory)
            {
                return Err(
                    "an operation changed installed inventory capacity or accepted-item filters"
                        .into(),
                );
            }
        }
        check_reservations(after)?;
        match expected {
            ExpectedOperation::Command(command) => {
                self.command(command, before, after, events, result)?
            }
            ExpectedOperation::InventoryTransfer {
                slot,
                from,
                to,
                item,
                amount,
            } => {
                if !events.is_empty() {
                    return Err("inventory transfer emitted economy events".into());
                }
                let from_owner = before
                    .units
                    .iter()
                    .find(|u| u.unit_id == *from)
                    .map(|u| u.owner_slot);
                let to_owner = before
                    .units
                    .iter()
                    .find(|u| u.unit_id == *to)
                    .map(|u| u.owner_slot);
                let source_inventory = before.raw_inventories.iter().find(|i| i.unit == *from);
                let destination_inventory = before.raw_inventories.iter().find(|i| i.unit == *to);
                let source = inventory_item(before, *from, item);
                let destination = inventory_item(before, *to, item);
                let destination_used = before
                    .economy
                    .inventories
                    .iter()
                    .find(|i| i.unit == *to)
                    .map(|i| i.items.iter().map(|x| u128::from(x.on_hand)).sum::<u128>())
                    .unwrap_or(0);
                let rejected = *amount == 0
                    || from_owner != Some(*slot)
                    || to_owner != Some(*slot)
                    || from == to
                    || source_inventory.is_none_or(|i| !i.accepted_items.contains(item))
                    || destination_inventory.is_none_or(|i| !i.accepted_items.contains(item))
                    || source.0.saturating_sub(source.1) < *amount
                    || destination_used + u128::from(*amount)
                        > u128::from(destination_inventory.map_or(0, |i| i.capacity))
                    || destination.0.checked_add(*amount).is_none();
                if rejected {
                    if before != after {
                        return Err("rejected transfer changed state".into());
                    }
                } else {
                    if inventory_item(after, *from, item).0 != source.0 - *amount
                        || inventory_item(after, *from, item).1 != source.1
                        || inventory_item(after, *to, item).0 != destination.0 + *amount
                        || inventory_item(after, *to, item).1 != destination.1
                    {
                        return Err("transfer state delta differs from submitted endpoints, filter and amount".into());
                    }
                    let mut expected = before.clone();
                    change_inventory_amount(&mut expected, *from, item, -i128::from(*amount))?;
                    change_inventory_amount(&mut expected, *to, item, i128::from(*amount))?;
                    if expected != *after {
                        return Err("transfer changed state outside the two submitted filtered inventory endpoints".into());
                    }
                }
            }
            ExpectedOperation::Automatic {
                tick,
                phase,
                subject,
            } => {
                if *tick != before.economy.tick + 1 && *tick != before.economy.tick {
                    return Err("automatic operation has wrong tick".into());
                }
                match (phase, subject) {
                    (AutomaticPhase::Gather, AutomaticSubject::Gatherer(unit)) => {
                        self.gather(*unit, before, after, events)?
                    }
                    (
                        AutomaticPhase::ProductionAdvance,
                        AutomaticSubject::HeadJob { producer, job },
                    ) => self.production_advance(*producer, *job, before, after, events)?,
                    (
                        AutomaticPhase::OutputAttempt,
                        AutomaticSubject::HeadJob { producer, job },
                    ) => self.output_attempt(*producer, *job, before, after, events)?,
                    _ => return Err("automatic phase and subject are incompatible".into()),
                }
            }
            ExpectedOperation::TickBoundary { .. } => {
                unreachable!("handled before operation continuity")
            }
        }
        self.append_events(before, after, events)?;
        self.check_conservation(after)?;
        self.operations += 1;
        self.last = after.clone();
        Ok(())
    }

    fn record_charge(&mut self, source: CostSource, slot: u8, item: &ContentId, amount: u64) {
        match source {
            CostSource::SlotBalance => {
                *self
                    .balance_charges
                    .entry((slot, item.clone()))
                    .or_default() += u128::from(amount)
            }
            CostSource::ProducerInventory => {
                *self.item_charges.entry(item.clone()).or_default() += u128::from(amount)
            }
        }
    }
    fn record_refund(&mut self, source: CostSource, slot: u8, item: &ContentId, amount: u64) {
        match source {
            CostSource::SlotBalance => {
                *self
                    .balance_refunds
                    .entry((slot, item.clone()))
                    .or_default() += u128::from(amount)
            }
            CostSource::ProducerInventory => {
                *self.item_refunds.entry(item.clone()).or_default() += u128::from(amount)
            }
        }
    }
    fn check_conservation(&self, state: &AuditState) -> Result<(), String> {
        let mut items = BTreeMap::<ContentId, u128>::new();
        for inv in &state.economy.inventories {
            for line in &inv.items {
                *items.entry(line.resource.clone()).or_default() += u128::from(line.on_hand);
            }
        }
        for node in &state.nodes {
            if let Some(def) = state
                .catalog
                .as_ref()
                .and_then(|c| c.nodes.iter().find(|d| d.id == node.content_id))
            {
                *items.entry(def.item.clone()).or_default() += u128::from(node.remaining);
            }
        }
        let mut initial = BTreeMap::<ContentId, u128>::new();
        for inv in &self.initial.economy.inventories {
            for line in &inv.items {
                *initial.entry(line.resource.clone()).or_default() += u128::from(line.on_hand);
            }
        }
        for node in &self.initial.nodes {
            if let Some(def) = self
                .initial
                .catalog
                .as_ref()
                .and_then(|c| c.nodes.iter().find(|d| d.id == node.content_id))
            {
                *initial.entry(def.item.clone()).or_default() += u128::from(node.remaining);
            }
        }
        let mut ids = initial
            .keys()
            .chain(items.keys())
            .chain(self.item_charges.keys())
            .chain(self.item_refunds.keys())
            .chain(self.conversion_sinks.keys())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        for id in std::mem::take(&mut ids) {
            let expected = initial.get(&id).copied().unwrap_or(0)
                + self.item_refunds.get(&id).copied().unwrap_or(0);
            let accounted = items.get(&id).copied().unwrap_or(0)
                + self.item_charges.get(&id).copied().unwrap_or(0)
                + self.conversion_sinks.get(&id).copied().unwrap_or(0);
            if expected != accounted {
                return Err(format!(
                    "physical item equation failed for {id}: initial + refund != stock + charge + conversion sink"
                ));
            }
        }
        let mut keys = BTreeMap::<(u8, ContentId), ()>::new();
        for x in &self.initial.economy.balances {
            keys.insert((x.slot, x.resource.clone()), ());
        }
        for x in &state.economy.balances {
            keys.insert((x.slot, x.resource.clone()), ());
        }
        for k in self.balance_charges.keys() {
            keys.insert(k.clone(), ());
        }
        for k in self.balance_refunds.keys() {
            keys.insert(k.clone(), ());
        }
        for k in self.balance_conversions.keys() {
            keys.insert(k.clone(), ());
        }
        for ((slot, id), ()) in keys {
            let old = self
                .initial
                .economy
                .balances
                .iter()
                .find(|x| x.slot == slot && x.resource == id)
                .map_or(0, |x| x.on_hand);
            let current = state
                .economy
                .balances
                .iter()
                .find(|x| x.slot == slot && x.resource == id)
                .map_or(0, |x| x.on_hand);
            let lhs = u128::from(old)
                + self
                    .balance_refunds
                    .get(&(slot, id.clone()))
                    .copied()
                    .unwrap_or(0)
                + self
                    .balance_conversions
                    .get(&(slot, id.clone()))
                    .copied()
                    .unwrap_or(0);
            let rhs = u128::from(current)
                + self
                    .balance_charges
                    .get(&(slot, id.clone()))
                    .copied()
                    .unwrap_or(0);
            if lhs != rhs {
                return Err(format!("balance equation failed for slot {slot} {id}"));
            }
        }
        Ok(())
    }

    fn tick_boundary(
        &mut self,
        tick: u32,
        before: &AuditState,
        after: &AuditState,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        if before != &self.tick_start_state {
            return Err(
                "TickAudit before snapshot differs from independently modeled tick-start state"
                    .into(),
            );
        }
        if self.tick_start_state.economy.tick.checked_add(1) != Some(tick)
            || after.economy.tick != tick
        {
            return Err("tick transition is not exactly one completed tick".into());
        }
        let mut expected_events = self.last.pending_events.clone();
        expected_events.sort_by_key(|e| e.ordinal);
        if events != expected_events {
            return Err(
                "published events differ from independently validated pending operations".into(),
            );
        }
        if !events.windows(2).all(|w| w[0].ordinal < w[1].ordinal) {
            return Err("published events are not in strict ordinal order".into());
        }
        let mut modeled = self.last.clone();
        if !self.movement_checked_this_tick {
            let tick_start = modeled.clone();
            modeled = self.predicted_movement(&tick_start, tick)?;
        }
        modeled.economy.tick = tick;
        modeled.pending_events.clear();
        modeled.next_event_ordinal = Some(0);
        if modeled != *after {
            return Err("completed TickAudit state differs from operations, one predicted movement transition, and event drain".into());
        }
        self.movement_checked_this_tick = false;
        self.tick_start_state = after.clone();
        Ok(())
    }

    pub fn modeled_state(&self) -> &AuditState {
        &self.last
    }

    fn command(
        &mut self,
        c: &Command,
        b: &AuditState,
        a: &AuditState,
        events: &[EconomyEvent],
        result: Option<&CommandResult>,
    ) -> Result<(), String> {
        let r = result.ok_or("submitted command has no result")?;
        if r.slot != c.slot || r.seq != c.seq {
            return Err("command result identity differs from submitted input".into());
        }
        let should_succeed = self.command_should_succeed(c, b)?;
        if r.outcome.is_ok() != should_succeed {
            return Err(
                "command acceptance differs from independent prior-state and catalog validation"
                    .into(),
            );
        }
        if !should_succeed {
            if b != a || !events.is_empty() {
                return Err("rejected command changed state or emitted events".into());
            }
            self.expected_commands.push(c.clone());
            self.tick_start_state = a.clone();
            return Ok(());
        }
        match &c.kind {
            CommandKind::Produce {
                producer_id,
                recipe_id,
            } => {
                let job_id = r
                    .outcome
                    .as_ref()
                    .unwrap()
                    .job_id
                    .ok_or("accepted Produce omitted JobId")?
                    .0;
                let expected_id = b.allocators.next_values().2;
                if job_id != expected_id || expected_id.checked_add(1).is_none() {
                    return Err("JobId does not equal the checked prior allocator value".into());
                }
                let recipe = self
                    .catalog
                    .production
                    .get(recipe_id)
                    .ok_or("requested recipe missing from immutable catalog")?
                    .clone();
                let producer = self.unit_catalog(b, *producer_id)?;
                let prod = producer
                    .producer
                    .as_ref()
                    .ok_or("requested producer lacks catalog capability")?;
                let duration = u64::try_from(
                    (u128::from(recipe.duration_ms)
                        * u128::from(b.tick_hz.ok_or("clock missing")?))
                    .div_ceil(1000),
                )
                .map_err(|_| "recipe duration exceeds u64")?;
                let (charged, reserved) = match recipe.payment {
                    PaymentPolicy::ChargeOnQueue => (
                        recipe
                            .costs
                            .iter()
                            .map(|x| turts_sim::CostView {
                                source: x.source,
                                resource: x.resource.clone(),
                                amount: x.amount,
                            })
                            .collect(),
                        Vec::new(),
                    ),
                    PaymentPolicy::ReserveThenCharge => (
                        Vec::new(),
                        recipe
                            .costs
                            .iter()
                            .map(|x| turts_sim::CostView {
                                source: x.source,
                                resource: x.resource.clone(),
                                amount: x.amount,
                            })
                            .collect(),
                    ),
                };
                let frozen = FrozenJob {
                    recipe: recipe_id.clone(),
                    stage: JobStage::Queued,
                    elapsed: 0,
                    duration,
                    charged,
                    reserved,
                };
                let job = frozen.view(job_id);
                let target_queue = b
                    .economy
                    .queues
                    .iter()
                    .find(|q| q.producer == *producer_id)
                    .ok_or("producer queue is absent")?;
                let mut expected_state = b.clone();
                let q = expected_state
                    .economy
                    .queues
                    .iter_mut()
                    .find(|q| q.producer == *producer_id)
                    .ok_or("producer queue absent")?;
                q.jobs.push(job.clone());
                let next = b.allocators.next_values();
                expected_state
                    .allocators
                    .set_next_values(next.0, next.1, expected_id + 1);
                if recipe.payment == PaymentPolicy::ReserveThenCharge {
                    expected_state
                        .ledger_claims
                        .push(turts_sim::ReservationClaimView::Active {
                            map_job_id: job_id,
                            token_job_id: job_id,
                            lines: recipe
                                .costs
                                .iter()
                                .map(|cost| turts_sim::ReservationLineView {
                                    endpoint: match cost.source {
                                        CostSource::SlotBalance => {
                                            turts_sim::ReservationEndpointView::Slot(c.slot.0)
                                        }
                                        CostSource::ProducerInventory => {
                                            turts_sim::ReservationEndpointView::Inventory(
                                                *producer_id,
                                            )
                                        }
                                    },
                                    resource: cost.resource.clone(),
                                    amount: cost.amount,
                                })
                                .collect(),
                        });
                }
                for cost in &recipe.costs {
                    match recipe.payment {
                        PaymentPolicy::ChargeOnQueue => apply_cost(
                            &mut expected_state,
                            cost.source,
                            c.slot.0,
                            *producer_id,
                            &cost.resource,
                            -i128::from(cost.amount),
                            0,
                        )?,
                        PaymentPolicy::ReserveThenCharge => apply_cost(
                            &mut expected_state,
                            cost.source,
                            c.slot.0,
                            *producer_id,
                            &cost.resource,
                            0,
                            i128::from(cost.amount),
                        )?,
                    }
                }
                expected_state
                    .ledger_claims
                    .sort_by_key(|claim| match claim {
                        turts_sim::ReservationClaimView::Active { map_job_id, .. } => *map_job_id,
                        turts_sim::ReservationClaimView::Closed { map_job_id } => *map_job_id,
                    });
                append_modeled_events(&mut expected_state, b, events)?;
                if expected_state != *a {
                    return Err("accepted Produce differs from exact requested recipe, ledger and unrelated-state model".into());
                }
                if queue_job(a, *producer_id, job_id) != Some(&job) {
                    return Err("accepted job recipe, costs, policy, duration, stage or id differs from driver request".into());
                }
                if target_queue.jobs.len() >= usize::from(prod.1) {
                    return Err("accepted Produce exceeded catalog queue capacity".into());
                }
                if a.allocators.next_values()
                    != (
                        b.allocators.next_values().0,
                        b.allocators.next_values().1,
                        expected_id + 1,
                    )
                {
                    return Err("accepted Produce allocator delta is not exactly one JobId".into());
                }
                if a.nodes != b.nodes
                    || a.units != b.units
                    || a.raw_units != b.raw_units
                    || a.raw_orders != b.raw_orders
                    || a.raw_inventories != b.raw_inventories
                    || a.unit_types != b.unit_types
                    || a.claims != b.claims
                {
                    return Err(
                        "Produce changed unrelated entity, order, node or resource metadata".into(),
                    );
                }
                let mut expected_claims = b.ledger_claims.clone();
                if recipe.payment == PaymentPolicy::ReserveThenCharge {
                    expected_claims.push(turts_sim::ReservationClaimView::Active {
                        map_job_id: job_id,
                        token_job_id: job_id,
                        lines: recipe
                            .costs
                            .iter()
                            .map(|cost| turts_sim::ReservationLineView {
                                endpoint: match cost.source {
                                    CostSource::SlotBalance => {
                                        turts_sim::ReservationEndpointView::Slot(c.slot.0)
                                    }
                                    CostSource::ProducerInventory => {
                                        turts_sim::ReservationEndpointView::Inventory(*producer_id)
                                    }
                                },
                                resource: cost.resource.clone(),
                                amount: cost.amount,
                            })
                            .collect(),
                    });
                }
                expected_claims.sort_by_key(|claim| match claim {
                    turts_sim::ReservationClaimView::Active { map_job_id, .. } => *map_job_id,
                    turts_sim::ReservationClaimView::Closed { map_job_id } => *map_job_id,
                });
                let mut got_claims = a.ledger_claims.clone();
                got_claims.sort_by_key(|claim| match claim {
                    turts_sim::ReservationClaimView::Active { map_job_id, .. } => *map_job_id,
                    turts_sim::ReservationClaimView::Closed { map_job_id } => *map_job_id,
                });
                if expected_claims != got_claims {
                    return Err(
                        "reservation claim registry differs from requested recipe costs".into(),
                    );
                }
                for cost in &recipe.costs {
                    let amount_before =
                        endpoint_amount(b, cost.source, c.slot.0, *producer_id, &cost.resource);
                    let amount_after =
                        endpoint_amount(a, cost.source, c.slot.0, *producer_id, &cost.resource);
                    let reserved_before =
                        endpoint_reserved(b, cost.source, c.slot.0, *producer_id, &cost.resource);
                    let reserved_after =
                        endpoint_reserved(a, cost.source, c.slot.0, *producer_id, &cost.resource);
                    match recipe.payment {
                        PaymentPolicy::ChargeOnQueue => {
                            if amount_before.checked_sub(cost.amount) != Some(amount_after)
                                || reserved_before != reserved_after
                            {
                                return Err(
                                    "charge-on-queue delta differs from requested recipe".into()
                                );
                            }
                            self.record_charge(cost.source, c.slot.0, &cost.resource, cost.amount);
                            self.gross_charge += u128::from(cost.amount);
                        }
                        PaymentPolicy::ReserveThenCharge => {
                            if amount_before != amount_after
                                || reserved_before.checked_add(cost.amount) != Some(reserved_after)
                            {
                                return Err(
                                    "reserve-then-charge reservation differs from requested recipe"
                                        .into(),
                                );
                            }
                        }
                    }
                }
                if events.len() != 1
                    || events[0].kind
                        != (EconomyEventKind::ProductionQueued {
                            producer: *producer_id,
                            job: job_id,
                            recipe: recipe_id.clone(),
                        })
                {
                    return Err("ProductionQueued event does not match submitted request".into());
                }
                self.frozen_jobs.insert((*producer_id, job_id), frozen);
            }
            CommandKind::CancelProduction {
                producer_id,
                job_id,
            } => {
                let frozen = self
                    .frozen_jobs
                    .get(&(*producer_id, job_id.0))
                    .cloned()
                    .ok_or("cancel target is not present in independent prior job model")?;
                let before_job = queue_job(b, *producer_id, job_id.0)
                    .ok_or("frozen cancel target absent from prior snapshot")?;
                if before_job != &frozen.view(job_id.0) {
                    return Err(
                        "prior cancel target was poisoned relative to frozen recipe and costs"
                            .into(),
                    );
                }
                let recipe = self
                    .catalog
                    .production
                    .get(&frozen.recipe)
                    .ok_or("frozen cancel recipe absent")?
                    .clone();
                let (ratio, refund_lines, release_lines) = match frozen.stage {
                    JobStage::Queued => {
                        let refund = if recipe.payment == PaymentPolicy::ChargeOnQueue {
                            frozen.charged.clone()
                        } else {
                            Vec::new()
                        };
                        (
                            recipe.cancel_refunds.queued,
                            refund,
                            frozen.reserved.clone(),
                        )
                    }
                    JobStage::Running => (
                        recipe.cancel_refunds.running,
                        frozen.charged.clone(),
                        Vec::new(),
                    ),
                    JobStage::OutputBlocked => (
                        recipe.cancel_refunds.output_blocked,
                        frozen.charged.clone(),
                        Vec::new(),
                    ),
                };
                let mut expected_refunds = Vec::new();
                for line in &refund_lines {
                    let amount = u64::try_from(
                        u128::from(line.amount) * u128::from(ratio.numerator)
                            / u128::from(ratio.denominator),
                    )
                    .map_err(|_| "refund amount overflow")?;
                    expected_refunds.push(turts_sim::CostLine {
                        source: event_source(line.source, c.slot.0, *producer_id),
                        resource: line.resource.clone(),
                        amount,
                    });
                }
                let expected_released = release_lines
                    .iter()
                    .map(|line| turts_sim::CostLine {
                        source: event_source(line.source, c.slot.0, *producer_id),
                        resource: line.resource.clone(),
                        amount: line.amount,
                    })
                    .collect::<Vec<_>>();
                let expected_event = EconomyEventKind::ProductionCancelled {
                    producer: *producer_id,
                    job: job_id.0,
                    released: expected_released,
                    refunded: expected_refunds.clone(),
                };
                if events.len() != 1 || events[0].kind != expected_event {
                    return Err(
                        "cancel event differs from requested frozen job and catalog refund ratio"
                            .into(),
                    );
                }
                let mut expected_queues = b.economy.queues.clone();
                let q = expected_queues
                    .iter_mut()
                    .find(|q| q.producer == *producer_id)
                    .ok_or("cancel producer queue missing")?;
                let index = q
                    .jobs
                    .iter()
                    .position(|x| x.id == job_id.0)
                    .ok_or("requested target absent")?;
                q.jobs.remove(index);
                if a.economy.queues != expected_queues {
                    return Err("cancel changed another job or queue entry".into());
                }
                if a.nodes != b.nodes
                    || a.units != b.units
                    || a.raw_units != b.raw_units
                    || a.raw_orders != b.raw_orders
                    || a.raw_inventories != b.raw_inventories
                    || a.unit_types != b.unit_types
                    || a.claims != b.claims
                    || a.allocators != b.allocators
                {
                    return Err(
                        "cancel changed unrelated entity, node, order, or allocator state".into(),
                    );
                }
                let refunds_by_line = refund_lines
                    .iter()
                    .zip(expected_refunds.iter())
                    .map(|(line, event)| (line, event.amount))
                    .collect::<Vec<_>>();
                for line in &refund_lines {
                    let amount = refunds_by_line
                        .iter()
                        .find(|(x, _)| x.resource == line.resource)
                        .map_or(0, |(_, amount)| *amount);
                    let old =
                        endpoint_amount(b, line.source, c.slot.0, *producer_id, &line.resource);
                    let new =
                        endpoint_amount(a, line.source, c.slot.0, *producer_id, &line.resource);
                    if old.checked_add(amount) != Some(new) {
                        return Err(
                            "refund destination delta differs from independent charged lines"
                                .into(),
                        );
                    }
                    if amount > 0 {
                        self.record_refund(line.source, c.slot.0, &line.resource, amount);
                        self.refunds += u128::from(amount);
                    }
                }
                for line in &release_lines {
                    if endpoint_reserved(b, line.source, c.slot.0, *producer_id, &line.resource)
                        .checked_sub(line.amount)
                        != Some(endpoint_reserved(
                            a,
                            line.source,
                            c.slot.0,
                            *producer_id,
                            &line.resource,
                        ))
                    {
                        return Err(
                            "cancellation did not release exact original reservation".into()
                        );
                    }
                }
                let mut claims = b.ledger_claims.clone();
                if frozen.stage == JobStage::Queued
                    && recipe.payment == PaymentPolicy::ReserveThenCharge
                {
                    for claim in &mut claims {
                        if matches!(claim,turts_sim::ReservationClaimView::Active{map_job_id,..}if *map_job_id==job_id.0)
                        {
                            *claim = turts_sim::ReservationClaimView::Closed {
                                map_job_id: job_id.0,
                            };
                        }
                    }
                }
                if a.ledger_claims != claims {
                    return Err("cancel claim registry changed beyond the requested job".into());
                }
                let mut expected_state = b.clone();
                expected_state.economy.queues = expected_queues;
                expected_state.ledger_claims = claims;
                for (line, event_line) in refund_lines.iter().zip(expected_refunds.iter()) {
                    if event_line.amount > 0 {
                        apply_cost(
                            &mut expected_state,
                            line.source,
                            c.slot.0,
                            *producer_id,
                            &line.resource,
                            i128::from(event_line.amount),
                            0,
                        )?;
                    }
                }
                for line in &release_lines {
                    apply_cost(
                        &mut expected_state,
                        line.source,
                        c.slot.0,
                        *producer_id,
                        &line.resource,
                        0,
                        -i128::from(line.amount),
                    )?;
                }
                append_modeled_events(&mut expected_state, b, events)?;
                if expected_state != *a {
                    return Err("Cancel changed state outside the requested job, catalog-derived refund, reservation release and claim close".into());
                }
                self.frozen_jobs.remove(&(*producer_id, job_id.0));
                self.cancellations += 1;
            }
            CommandKind::MoveTo { unit_ids, target } => {
                self.validate_move_command(b, a, unit_ids, *target, events)?
            }
            CommandKind::Gather { unit_id, node_id } => {
                self.validate_gather_command(b, a, *unit_id, *node_id, events)?
            }
        }
        self.expected_commands.push(c.clone());
        self.tick_start_state = a.clone();
        Ok(())
    }

    fn command_should_succeed(&self, c: &Command, b: &AuditState) -> Result<bool, String> {
        match &c.kind {
            CommandKind::MoveTo { unit_ids, target } => {
                if unit_ids.is_empty()
                    || unit_ids.len() > 256
                    || unit_ids
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        != unit_ids.len()
                    || !target.is_finite()
                    || b.bounds.as_ref().is_none_or(|m| {
                        target.x < 0.0 || target.y < 0.0 || target.x > m.x || target.y > m.y
                    })
                {
                    return Ok(false);
                }
                for id in unit_ids {
                    let Some(unit) = b.raw_units.iter().find(|u| u.unit_id == *id) else {
                        return Ok(false);
                    };
                    if unit.owner_slot != c.slot.0 {
                        return Ok(false);
                    }
                    let Some(def) = self.unit_catalog(b, *id).ok() else {
                        return Ok(false);
                    };
                    if def.movement_speed.is_none() {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            CommandKind::Gather { unit_id, node_id } => {
                let Some(unit) = b.raw_units.iter().find(|u| u.unit_id == *unit_id) else {
                    return Ok(false);
                };
                if unit.owner_slot != c.slot.0 {
                    return Ok(false);
                }
                let Some(node) = b.nodes.iter().find(|n| n.id == *node_id && n.remaining > 0)
                else {
                    return Ok(false);
                };
                let Some(unit_def) = self.unit_catalog(b, *unit_id).ok() else {
                    return Ok(false);
                };
                let Some(g) = unit_def.gatherer.as_ref() else {
                    return Ok(false);
                };
                let item = b
                    .catalog
                    .as_ref()
                    .and_then(|c| c.nodes.iter().find(|n| n.id == node.content_id))
                    .map(|n| &n.item);
                Ok(item.is_some_and(|i| g.items.contains(i))
                    && b.raw_inventories
                        .iter()
                        .find(|i| i.unit == *unit_id)
                        .is_some_and(|inv| item.is_some_and(|i| inv.accepted_items.contains(i))))
            }
            CommandKind::Produce {
                producer_id,
                recipe_id,
            } => {
                let Some(unit) = b
                    .units
                    .iter()
                    .find(|u| u.unit_id == *producer_id && u.owner_slot == c.slot.0)
                else {
                    return Ok(false);
                };
                let id = ContentId::parse(&unit.content_id)
                    .map_err(|_| "producer content id invalid")?;
                let Some(prod) = b
                    .catalog
                    .as_ref()
                    .and_then(|cat| cat.units.iter().find(|u| u.id == id))
                    .and_then(|u| u.producer.as_ref())
                else {
                    return Ok(false);
                };
                let Some(recipe) = self.catalog.production.get(recipe_id) else {
                    return Ok(false);
                };
                let queue = b.economy.queues.iter().find(|q| q.producer == *producer_id);
                if !prod.0.contains(recipe_id)
                    || queue.is_none_or(|q| q.jobs.len() >= usize::from(prod.1))
                    || b.allocators.next_values().2.checked_add(1).is_none()
                {
                    return Ok(false);
                }
                for cost in &recipe.costs {
                    let amount =
                        endpoint_amount(b, cost.source, c.slot.0, *producer_id, &cost.resource);
                    let reserved =
                        endpoint_reserved(b, cost.source, c.slot.0, *producer_id, &cost.resource);
                    if amount.saturating_sub(reserved) < cost.amount {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            CommandKind::CancelProduction {
                producer_id,
                job_id,
            } => {
                if !b
                    .units
                    .iter()
                    .any(|u| u.unit_id == *producer_id && u.owner_slot == c.slot.0)
                    || !self.frozen_jobs.contains_key(&(*producer_id, job_id.0))
                {
                    return Ok(false);
                }
                let job = self.frozen_jobs.get(&(*producer_id, job_id.0)).unwrap();
                let Some(recipe) = self.catalog.production.get(&job.recipe) else {
                    return Ok(false);
                };
                let ratio = match job.stage {
                    JobStage::Queued => recipe.cancel_refunds.queued,
                    JobStage::Running => recipe.cancel_refunds.running,
                    JobStage::OutputBlocked => recipe.cancel_refunds.output_blocked,
                };
                let lines = if job.stage == JobStage::Queued
                    && recipe.payment == PaymentPolicy::ReserveThenCharge
                {
                    &[][..]
                } else {
                    &job.charged[..]
                };
                for line in lines {
                    let refund = u64::try_from(
                        u128::from(line.amount) * u128::from(ratio.numerator)
                            / u128::from(ratio.denominator),
                    )
                    .map_err(|_| "refund overflow")?;
                    let old =
                        endpoint_amount(b, line.source, c.slot.0, *producer_id, &line.resource);
                    if old.checked_add(refund).is_none() {
                        return Ok(false);
                    }
                    if line.source == CostSource::ProducerInventory {
                        let inv = b
                            .economy
                            .inventories
                            .iter()
                            .find(|i| i.unit == *producer_id);
                        let used = inv
                            .map(|i| i.items.iter().map(|x| u128::from(x.on_hand)).sum::<u128>())
                            .unwrap_or(0);
                        let cap = b
                            .raw_inventories
                            .iter()
                            .find(|i| i.unit == *producer_id)
                            .map_or(0, |i| i.capacity);
                        if used + u128::from(refund) > u128::from(cap) {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
        }
    }

    fn validate_move_command(
        &self,
        b: &AuditState,
        a: &AuditState,
        ids: &[u32],
        target: bevy_math::Vec2,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        if !events.is_empty() {
            return Err("MoveTo emitted economy events".into());
        }
        let target = turts_mod_api::WorldPos {
            x: target.x,
            y: target.y,
        };
        let mut expected = b.clone();
        for id in ids {
            let old = expected
                .raw_units
                .iter_mut()
                .find(|u| u.unit_id == *id)
                .ok_or("MoveTo unit absent")?;
            old.move_target = Some(target.clone());
            let view = expected
                .units
                .iter()
                .find(|u| u.unit_id == *id)
                .ok_or("MoveTo unit view absent")?;
            let _ = view;
            if let Some(order) = expected.raw_orders.iter_mut().find(|o| o.unit == *id) {
                order.node = None;
                order.deposit = None;
                order.receiver = None;
                order.stage = turts_sim::GatherStage::Idle;
                order.rate_carry = 0;
            }
            if let Some(order) = expected.economy.orders.iter_mut().find(|o| o.unit == *id) {
                order.node = None;
                order.receiver = None;
                order.stage = turts_sim::GatherStage::Idle;
            }
        }
        if expected != *a {
            return Err(
                "MoveTo changed state outside submitted targets and gather interruption".into(),
            );
        }
        Ok(())
    }
    fn validate_gather_command(
        &self,
        b: &AuditState,
        a: &AuditState,
        unit: u32,
        node: turts_sim::NodeId,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        if !events.is_empty() {
            return Err("Gather command emitted events".into());
        }
        let mut expected = b.clone();
        let order = expected
            .raw_orders
            .iter_mut()
            .find(|o| o.unit == unit)
            .ok_or("gather order absent")?;
        order.node = Some(node);
        order.deposit = Some(node);
        order.receiver = None;
        order.rate_carry = 0;
        let node_before = b.nodes.iter().find(|n| n.id == node).ok_or("node absent")?;
        let item = b
            .catalog
            .as_ref()
            .and_then(|c| c.nodes.iter().find(|n| n.id == node_before.content_id))
            .map(|n| &n.item)
            .ok_or("node catalog item absent")?;
        let inv = b
            .economy
            .inventories
            .iter()
            .find(|i| i.unit == unit)
            .ok_or("gather inventory absent")?;
        let used = inv
            .items
            .iter()
            .map(|x| u128::from(x.on_hand))
            .sum::<u128>();
        let other = inv
            .items
            .iter()
            .any(|x| x.on_hand > x.reserved && &x.resource != item);
        let must_deliver = used >= u128::from(inv.capacity) || other;
        order.stage = if must_deliver {
            turts_sim::GatherStage::MovingToReceiver
        } else {
            turts_sim::GatherStage::MovingToNode
        };
        if let Some(raw) = expected.raw_units.iter_mut().find(|u| u.unit_id == unit) {
            raw.move_target = None;
        }
        if expected.nodes != a.nodes
            || expected.economy.balances != a.economy.balances
            || expected.economy.inventories != a.economy.inventories
            || expected.economy.queues != a.economy.queues
            || expected.allocators != a.allocators
        {
            return Err("Gather command changed economic quantities".into());
        }
        if let Some(view) = expected.economy.orders.iter_mut().find(|o| o.unit == unit) {
            view.node = Some(node);
            view.receiver = None;
            view.stage = order.stage;
        }
        if expected != *a {
            return Err(
                "Gather command changed state outside the independently predicted order assignment"
                    .into(),
            );
        }
        Ok(())
    }

    fn gather(
        &mut self,
        unit: u32,
        b: &AuditState,
        a: &AuditState,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        self.gather_inner(unit, b, a, events)?;
        if b.raw_inventories != a.raw_inventories
            || b.unit_types != a.unit_types
            || b.resources != a.resources
            || b.catalog != a.catalog
            || b.bounds != a.bounds
            || b.tick_hz != a.tick_hz
            || b.allocators != a.allocators
            || b.claims != a.claims
            || b.ledger_claims != a.ledger_claims
            || b.economy.queues != a.economy.queues
        {
            return Err("gather operation changed immutable filters, capabilities, claims, allocators, clock, or jobs".into());
        }
        for order in &b.raw_orders {
            if order.unit != unit
                && a.raw_orders.iter().find(|x| x.unit == order.unit) != Some(order)
            {
                return Err("gather operation changed an unrelated raw order".into());
            }
        }
        for unit_state in &b.raw_units {
            if unit_state.unit_id != unit
                && a.raw_units.iter().find(|x| x.unit_id == unit_state.unit_id) != Some(unit_state)
            {
                return Err(
                    "gather operation changed another unit's raw target or position".into(),
                );
            }
        }
        for unit_state in &b.units {
            if unit_state.unit_id != unit
                && a.units.iter().find(|x| x.unit_id == unit_state.unit_id) != Some(unit_state)
            {
                return Err("gather operation changed another unit projection".into());
            }
        }
        Ok(())
    }
    fn gather_inner(
        &mut self,
        unit: u32,
        b: &AuditState,
        a: &AuditState,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        let order = b
            .raw_orders
            .iter()
            .find(|o| o.unit == unit)
            .ok_or("expected gatherer is not modeled")?;
        if order.stage == turts_sim::GatherStage::Idle {
            if b != a || !events.is_empty() {
                return Err("idle gatherer operation changed state".into());
            }
            return Ok(());
        }
        if order.stage == turts_sim::GatherStage::Unloading {
            return self.unload(unit, b, a, events);
        }
        let node_id = order.deposit.ok_or("gatherer has no frozen deposit")?;
        let node = b
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or("frozen deposit absent")?;
        let def = self
            .catalog
            .nodes
            .get(&node.content_id)
            .ok_or("node definition absent")?;
        let unit_content = b
            .units
            .iter()
            .find(|u| u.unit_id == unit)
            .ok_or("gatherer unit missing")?
            .content_id
            .clone();
        let unit_id = ContentId::parse(&unit_content).map_err(|_| "unit content id invalid")?;
        let unit_def = b
            .catalog
            .as_ref()
            .and_then(|c| c.units.iter().find(|u| u.id == unit_id))
            .ok_or("unit catalog entry absent")?;
        let gather = unit_def
            .gatherer
            .as_ref()
            .ok_or("expected gatherer lacks gather definition")?;
        let hz = u128::from(b.tick_hz.ok_or("clock missing")?);
        let numerator = u128::from(gather.gather_per_second) + u128::from(order.rate_carry);
        let budget = if order.stage == turts_sim::GatherStage::Gathering {
            u64::try_from(numerator / hz).map_err(|_| "gather budget overflow")?
        } else {
            0
        };
        let inventory = b
            .economy
            .inventories
            .iter()
            .find(|i| i.unit == unit)
            .ok_or("gather inventory absent")?;
        let used: u128 = inventory.items.iter().map(|i| u128::from(i.on_hand)).sum();
        let headroom =
            u64::try_from(u128::from(inventory.capacity).saturating_sub(used)).unwrap_or(u64::MAX);
        let expected = budget.min(node.remaining).min(headroom);
        let extracted: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let EconomyEventKind::Extracted {
                    node,
                    cargo,
                    item,
                    amount,
                } = &e.kind
                {
                    Some((*node, *cargo, item, *amount))
                } else {
                    None
                }
            })
            .collect();
        if extracted.len() > 1 {
            return Err("one gatherer operation extracted more than once".into());
        }
        if expected == 0 {
            if !extracted.is_empty() {
                return Err("gather operation extracted without modeled budget or headroom".into());
            }
        } else {
            if extracted.len() != 1 {
                return Err("modeled gather budget was not extracted".into());
            }
            let (n, c, item, amount) = extracted[0];
            if n != node.id.0 || c != unit || item != &def.item || amount != expected {
                return Err("extraction differs from prior node, catalog item, rate carry or inventory headroom".into());
            }
            let node_after = a
                .nodes
                .iter()
                .find(|x| x.id == node_id)
                .ok_or("node disappeared")?;
            if node_after.remaining != node.remaining - expected
                || inventory_item(a, unit, &def.item).0
                    != inventory_item(b, unit, &def.item).0 + expected
            {
                return Err(
                    "node/cargo deltas disagree with independently derived extraction".into(),
                );
            }
            self.extraction += u128::from(expected);
        }
        if expected == 0 {
            self.assert_gather_no_resource_delta(unit, b, a)?;
        }
        let hz_u16 = b.tick_hz.ok_or("clock missing")?;
        if order.stage == turts_sim::GatherStage::Gathering {
            let carry = if headroom == 0 || node.remaining == 0 {
                order.rate_carry
            } else {
                u16::try_from(numerator % u128::from(hz_u16))
                    .map_err(|_| "rate remainder overflow")?
            };
            if a.raw_orders
                .iter()
                .find(|o| o.unit == unit)
                .map(|o| o.rate_carry)
                != Some(carry)
            {
                return Err("gather carry differs from independent rate remainder".into());
            }
        }
        for e in events {
            if !matches!(
                e.kind,
                EconomyEventKind::Extracted { .. }
                    | EconomyEventKind::Stored { .. }
                    | EconomyEventKind::Converted { .. }
            ) {
                return Err("gather phase emitted unrelated event".into());
            }
        }
        Ok(())
    }

    fn assert_gather_no_resource_delta(
        &self,
        _unit: u32,
        b: &AuditState,
        a: &AuditState,
    ) -> Result<(), String> {
        if b.nodes != a.nodes
            || b.economy.balances != a.economy.balances
            || b.economy.queues != a.economy.queues
            || b.economy.inventories != a.economy.inventories
        {
            return Err("gather phase changed unrelated stock, nodes, balances or jobs".into());
        }
        Ok(())
    }

    fn unload(
        &mut self,
        unit: u32,
        b: &AuditState,
        a: &AuditState,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        let order = b
            .raw_orders
            .iter()
            .find(|o| o.unit == unit)
            .ok_or("unloading order missing")?;
        let owner = b
            .units
            .iter()
            .find(|u| u.unit_id == unit)
            .map(|u| u.owner_slot)
            .ok_or("unloading unit missing")?;
        let Some(item) = b
            .economy
            .inventories
            .iter()
            .find(|i| i.unit == unit)
            .and_then(|i| i.items.iter().find(|x| x.on_hand > x.reserved))
            .map(|x| x.resource.clone())
        else {
            if !events.is_empty() {
                return Err("empty unload emitted a transfer event".into());
            }
            self.assert_gather_no_resource_delta(unit, b, a)?;
            return Ok(());
        };
        let receiver = order.receiver.ok_or("unloading receiver is not frozen")?;
        if receiver == unit {
            return Err("self-Store receiver was selected".into());
        }
        let receiver_unit = b
            .units
            .iter()
            .find(|u| u.unit_id == receiver)
            .ok_or("receiver unit missing")?;
        if receiver_unit.owner_slot != owner {
            return Err("foreign receiver selected".into());
        }
        let receiver_id =
            ContentId::parse(&receiver_unit.content_id).map_err(|_| "receiver id invalid")?;
        let receiver_def = b
            .catalog
            .as_ref()
            .and_then(|c| c.units.iter().find(|u| u.id == receiver_id))
            .ok_or("receiver catalog entry missing")?;
        let effect = receiver_def
            .drop_off
            .iter()
            .find(|r| r.item == item)
            .ok_or("receiver has no matching drop-off rule")?;
        let hz = u128::from(b.tick_hz.ok_or("clock missing")?);
        let gatherer = self.unit_catalog(b, unit)?;
        let g = gatherer
            .gatherer
            .as_ref()
            .ok_or("gatherer definition missing")?;
        let numerator = u128::from(g.unload_per_second) + u128::from(order.rate_carry);
        let budget = u64::try_from(numerator / hz).map_err(|_| "unload budget overflow")?;
        let cargo = inventory_item(b, unit, &item);
        let available_cargo = cargo.0 - cargo.1;
        let available_destination = match &effect.effect {
            turts_sim::economy::views::CatalogDropOffEffectAudit::Store => {
                let inv = b
                    .economy
                    .inventories
                    .iter()
                    .find(|i| i.unit == receiver)
                    .ok_or("store inventory missing")?;
                let used = inv
                    .items
                    .iter()
                    .map(|x| u128::from(x.on_hand))
                    .sum::<u128>();
                u64::try_from(u128::from(inv.capacity).saturating_sub(used)).unwrap_or(u64::MAX)
            }
            turts_sim::economy::views::CatalogDropOffEffectAudit::Convert { balance, .. } => {
                u64::MAX
                    - b.economy
                        .balances
                        .iter()
                        .find(|x| x.slot == owner && &x.resource == balance)
                        .ok_or("conversion balance missing")?
                        .on_hand
            }
        };
        let amount = budget.min(available_cargo).min(available_destination);
        let expected_carry = u16::try_from(numerator % hz).map_err(|_| "unload carry overflow")?;
        if a.raw_orders
            .iter()
            .find(|o| o.unit == unit)
            .map(|o| o.rate_carry)
            != Some(expected_carry)
        {
            return Err("unload carry differs from independent rate remainder".into());
        }
        if amount == 0 {
            if !events.is_empty() {
                return Err("zero-budget or full-destination unload emitted an event".into());
            }
            self.assert_gather_no_resource_delta(unit, b, a)?;
            return Ok(());
        }
        if events.len() != 1 {
            return Err("unload must emit one event for positive modeled amount".into());
        }
        if inventory_item(a, unit, &item).0 != cargo.0 - amount {
            return Err("unload cargo debit differs from rate, cargo and endpoint capacity".into());
        }
        match &effect.effect {
            turts_sim::economy::views::CatalogDropOffEffectAudit::Store => {
                let ev = &events[0];
                if ev.kind
                    != (EconomyEventKind::Stored {
                        from: unit,
                        to: receiver,
                        item: item.clone(),
                        amount,
                    })
                {
                    return Err("Store event differs from independent endpoints and amount".into());
                }
                if inventory_item(a, receiver, &item).0
                    != inventory_item(b, receiver, &item).0 + amount
                {
                    return Err("Store receiver credit is not conserved".into());
                }
            }
            turts_sim::economy::views::CatalogDropOffEffectAudit::Convert { balance, per_item } => {
                let credited = amount
                    .checked_mul(*per_item)
                    .ok_or("conversion multiplication overflow")?;
                let ev = &events[0];
                if ev.kind
                    != (EconomyEventKind::Converted {
                        from: unit,
                        to: receiver,
                        item: item.clone(),
                        amount,
                        balance: balance.clone(),
                        credited,
                    })
                {
                    return Err("conversion event differs from catalog ratio or endpoints".into());
                }
                let old = b
                    .economy
                    .balances
                    .iter()
                    .find(|x| x.slot == owner && &x.resource == balance)
                    .ok_or("conversion balance absent")?
                    .on_hand;
                let new = a
                    .economy
                    .balances
                    .iter()
                    .find(|x| x.slot == owner && &x.resource == balance)
                    .ok_or("conversion balance absent")?
                    .on_hand;
                if old.checked_add(credited) != Some(new) {
                    return Err("converted balance does not equal catalog multiplier".into());
                }
                self.conversion += u128::from(credited);
                *self.conversion_sinks.entry(item.clone()).or_default() += u128::from(amount);
                *self
                    .balance_conversions
                    .entry((owner, balance.clone()))
                    .or_default() += u128::from(credited);
            }
        }
        self.delivery += u128::from(amount);
        Ok(())
    }
    fn unit_catalog<'a>(
        &self,
        s: &'a AuditState,
        unit: u32,
    ) -> Result<&'a turts_sim::economy::views::CatalogUnitAudit, String> {
        let cid = s
            .units
            .iter()
            .find(|u| u.unit_id == unit)
            .ok_or("unit missing")?
            .content_id
            .clone();
        let id = ContentId::parse(&cid).map_err(|_| "unit id invalid")?;
        s.catalog
            .as_ref()
            .and_then(|c| c.units.iter().find(|u| u.id == id))
            .ok_or_else(|| "unit catalog missing".into())
    }

    fn production_advance(
        &mut self,
        producer: u32,
        job_id: u64,
        b: &AuditState,
        a: &AuditState,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        let frozen = self
            .frozen_jobs
            .get(&(producer, job_id))
            .cloned()
            .ok_or("production job absent from independent frozen model")?;
        let before = queue_job(b, producer, job_id).ok_or("modeled production head missing")?;
        if before != &frozen.view(job_id) {
            return Err(
                "production before-job differs from frozen requested recipe, costs, and duration"
                    .into(),
            );
        }
        let mut next = frozen.clone();
        let mut expected = b.clone();
        match frozen.stage {
            JobStage::Queued => {
                let recipe = self
                    .catalog
                    .production
                    .get(&frozen.recipe)
                    .ok_or("start recipe missing")?;
                let owner = b
                    .economy
                    .queues
                    .iter()
                    .find(|q| q.producer == producer)
                    .map(|q| q.owner_slot)
                    .ok_or("producer queue owner missing")?;
                let mut charged = frozen.charged.clone();
                if recipe.payment == PaymentPolicy::ReserveThenCharge {
                    charged = frozen.reserved.clone();
                }
                let expected_lines = charged
                    .iter()
                    .map(|line| turts_sim::CostLine {
                        source: event_source(line.source, owner, producer),
                        resource: line.resource.clone(),
                        amount: line.amount,
                    })
                    .collect::<Vec<_>>();
                if events.len() != 1
                    || events[0].kind
                        != (EconomyEventKind::ProductionStarted {
                            producer,
                            job: job_id,
                            charged: expected_lines,
                        })
                {
                    return Err("start charge event differs from frozen accepted recipe".into());
                }
                if recipe.payment == PaymentPolicy::ReserveThenCharge {
                    for line in &charged {
                        apply_cost(
                            &mut expected,
                            line.source,
                            owner,
                            producer,
                            &line.resource,
                            -(line.amount as i128),
                            -(line.amount as i128),
                        )?;
                        self.record_charge(line.source, owner, &line.resource, line.amount);
                        self.gross_charge += u128::from(line.amount);
                    }
                    for claim in &mut expected.ledger_claims {
                        if matches!(claim,turts_sim::ReservationClaimView::Active{map_job_id,..} if *map_job_id==job_id)
                        {
                            *claim = turts_sim::ReservationClaimView::Closed { map_job_id: job_id };
                        }
                    }
                } else {
                    if charged != frozen.charged {
                        return Err("charge-on-queue frozen charged lines changed".into());
                    }
                }
                next.stage = JobStage::Running;
                next.elapsed = 0;
                next.charged = charged;
                next.reserved.clear();
            }
            JobStage::Running => {
                if !events.is_empty() {
                    return Err("running production advance emitted an event".into());
                }
                next.elapsed = next
                    .elapsed
                    .checked_add(1)
                    .ok_or("elapsed tick overflow")?
                    .min(next.duration);
                if next.elapsed >= next.duration {
                    next.stage = JobStage::OutputBlocked;
                }
            }
            JobStage::OutputBlocked => {
                return Err("blocked output is not a production-advance operation".into());
            }
        }
        let queue = expected
            .economy
            .queues
            .iter_mut()
            .find(|q| q.producer == producer)
            .ok_or("queue disappeared")?;
        let slot = queue
            .jobs
            .iter_mut()
            .find(|j| j.id == job_id)
            .ok_or("frozen job disappeared")?;
        *slot = next.view(job_id);
        append_modeled_events(&mut expected, b, events)?;
        if expected != *a {
            return Err("production advance differs from exact frozen job, ledger, claims, and unrelated-state model".into());
        }
        self.frozen_jobs.insert((producer, job_id), next);
        Ok(())
    }

    fn output_attempt(
        &mut self,
        producer: u32,
        job_id: u64,
        b: &AuditState,
        a: &AuditState,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        let frozen = self
            .frozen_jobs
            .get(&(producer, job_id))
            .cloned()
            .ok_or("output job absent from frozen model")?;
        if frozen.stage != JobStage::OutputBlocked
            || queue_job(b, producer, job_id) != Some(&frozen.view(job_id))
        {
            return Err("output attempt expected the independently frozen blocked job".into());
        }
        let complete = events
            .iter()
            .filter_map(|e| {
                if let EconomyEventKind::ProductionCompleted {
                    producer: p,
                    job: j,
                    output_unit_id,
                } = &e.kind
                {
                    (*p == producer && *j == job_id).then_some(*output_unit_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if complete.len() > 1 || events.len() != complete.len() {
            return Err("output attempt event count is not zero or one matching completion".into());
        }
        if complete.is_empty() {
            if b != a {
                return Err(
                    "blocked output attempt changed state despite no valid spawn location".into(),
                );
            }
            return Ok(());
        }
        let id = complete[0];
        let recipe = self
            .catalog
            .production
            .get(&frozen.recipe)
            .ok_or("output recipe absent")?;
        let output_def = b
            .catalog
            .as_ref()
            .and_then(|c| c.units.iter().find(|u| u.id == recipe.output_unit))
            .ok_or("output unit definition absent from frozen catalog")?;
        let producer_pos = b
            .raw_units
            .iter()
            .find(|u| u.unit_id == producer)
            .ok_or("producer raw state missing")?
            .pos
            .clone();
        let producer_def = self.unit_catalog(b, producer)?;
        let offsets = &producer_def
            .producer
            .as_ref()
            .ok_or("producer spawn offsets absent")?
            .2;
        let radius = output_def.radius;
        let bounds = b.bounds.as_ref().ok_or("map bounds missing")?;
        let candidate = offsets
            .iter()
            .map(|offset| turts_mod_api::WorldPos {
                x: producer_pos.x + offset.x,
                y: producer_pos.y + offset.y,
            })
            .find(|p| {
                p.x - radius >= 0.0
                    && p.y - radius >= 0.0
                    && p.x + radius <= bounds.x
                    && p.y + radius <= bounds.y
                    && b.units.iter().all(|u| {
                        let Some(d) = b.catalog.as_ref().and_then(|c| {
                            c.units.iter().find(|d| d.id.to_string() == u.content_id)
                        }) else {
                            return false;
                        };
                        let dx = p.x - u.pos.x;
                        let dy = p.y - u.pos.y;
                        dx * dx + dy * dy >= (radius + d.radius) * (radius + d.radius)
                    })
                    && b.nodes.iter().all(|n| {
                        let nr = b
                            .catalog
                            .as_ref()
                            .and_then(|c| c.nodes.iter().find(|d| d.id == n.content_id))
                            .map_or(0.0, |d| d.radius);
                        let dx = p.x - n.pos.x;
                        let dy = p.y - n.pos.y;
                        dx * dx + dy * dy >= (radius + nr) * (radius + nr)
                    })
            })
            .ok_or("completion event occurred without a catalog-valid spawn candidate")?;
        let owner = b
            .units
            .iter()
            .find(|u| u.unit_id == producer)
            .ok_or("producer absent")?
            .owner_slot;
        let type_id = b
            .unit_types
            .iter()
            .find(|u| u.content_id == recipe.output_unit.to_string())
            .map(|u| u.type_id)
            .ok_or("output unit type absent from frozen type table")?;
        if id != b.allocators.next_values().0 {
            return Err("spawned UnitId differs from prior checked allocator value".into());
        }
        let mut expected = b.clone();
        let q = expected
            .economy
            .queues
            .iter_mut()
            .find(|q| q.producer == producer)
            .ok_or("producer queue absent")?;
        if q.jobs.first().is_none_or(|j| j.id != job_id) {
            return Err("completed output is not the FIFO head".into());
        }
        q.jobs.remove(0);
        let next = b.allocators.next_values();
        expected.allocators.set_next_values(
            next.0.checked_add(1).ok_or("UnitId exhausted")?,
            next.1,
            next.2,
        );
        expected.units.push(turts_sim::UnitView {
            unit_id: id,
            content_id: recipe.output_unit.to_string(),
            owner_slot: owner,
            pos: candidate.clone(),
        });
        expected.units.sort_by_key(|u| u.unit_id);
        expected.economy.units.push(turts_sim::UnitState {
            unit_id: id,
            content_id: recipe.output_unit.to_string(),
            owner_slot: owner,
            pos: bevy_math::Vec2::new(candidate.x, candidate.y),
        });
        expected.economy.units.sort_by_key(|u| u.unit_id);
        expected
            .raw_units
            .push(turts_sim::economy::views::RawUnitAudit {
                unit_id: id,
                unit_type: type_id,
                owner_slot: owner,
                pos: candidate.clone(),
                move_target: None,
            });
        expected.raw_units.sort_by_key(|u| u.unit_id);
        if let Some((capacity, accepts)) = &output_def.inventory {
            expected
                .raw_inventories
                .push(turts_sim::economy::views::RawInventoryAudit {
                    unit: id,
                    capacity: *capacity,
                    accepted_items: accepts.clone(),
                });
            expected.raw_inventories.sort_by_key(|x| x.unit);
            expected
                .economy
                .economy
                .inventories
                .push(turts_sim::InventoryView {
                    unit: id,
                    owner_slot: owner,
                    capacity: *capacity,
                    items: Vec::new(),
                });
            expected.economy.economy.inventories.sort_by_key(|x| x.unit);
        }
        if output_def.gatherer.is_some() {
            let idle = turts_sim::GatherStage::Idle;
            expected
                .raw_orders
                .push(turts_sim::economy::views::RawOrderAudit {
                    unit: id,
                    node: None,
                    deposit: None,
                    receiver: None,
                    stage: idle,
                    rate_carry: 0,
                });
            expected.raw_orders.sort_by_key(|x| x.unit);
            expected.economy.economy.orders.push(turts_sim::OrderView {
                unit: id,
                owner_slot: owner,
                node: None,
                receiver: None,
                stage: idle,
            });
            expected.economy.economy.orders.sort_by_key(|x| x.unit);
        }
        if let Some((_, capacity, _)) = &output_def.producer {
            let _ = capacity;
            expected.economy.economy.queues.push(turts_sim::QueueView {
                producer: id,
                owner_slot: owner,
                jobs: Vec::new(),
            });
            expected.economy.economy.queues.sort_by_key(|x| x.producer);
        }
        append_modeled_events(&mut expected, b, events)?;
        if events[0].kind
            != (EconomyEventKind::ProductionCompleted {
                producer,
                job: job_id,
                output_unit_id: id,
            })
            || expected != *a
        {
            return Err("completion differs from exact frozen recipe, catalog spawn candidate, allocator, and whole-state model".into());
        }
        self.frozen_jobs.remove(&(producer, job_id));
        self.outputs += 1;
        Ok(())
    }

    fn movement_transition(&self, b: &AuditState, a: &AuditState) -> Result<(), String> {
        let expected_tick = b.economy.tick.checked_add(1).ok_or("tick exhausted")?;
        if a.economy.tick != expected_tick {
            return Err("movement transition did not cross exactly one tick".into());
        }
        let modeled = self.predicted_movement(b, expected_tick)?;
        if modeled != *a {
            return Err("movement boundary differs from independently predicted fixed-step transition or changes unrelated state".into());
        }
        Ok(())
    }
    fn predicted_movement(&self, b: &AuditState, tick: u32) -> Result<AuditState, String> {
        let mut modeled = b.clone();
        modeled.economy.tick = tick;
        let hz = f32::from(b.tick_hz.ok_or("movement clock missing")?);
        for unit in &mut modeled.raw_units {
            if let Some(target) = unit.move_target.clone() {
                let def = self.unit_catalog(b, unit.unit_id)?;
                let speed = def.movement_speed.ok_or("moving unit lacks speed")?;
                let dx = target.x - unit.pos.x;
                let dy = target.y - unit.pos.y;
                let distance = (dx * dx + dy * dy).sqrt();
                let step = speed / hz;
                if distance <= step {
                    unit.pos = target;
                    unit.move_target = None
                } else {
                    unit.pos = turts_mod_api::WorldPos {
                        x: unit.pos.x + dx * (step / distance),
                        y: unit.pos.y + dy * (step / distance),
                    }
                }
            }
        }
        for unit in &mut modeled.units {
            let raw = modeled
                .raw_units
                .iter()
                .find(|x| x.unit_id == unit.unit_id)
                .ok_or("unit view missing")?;
            unit.pos = raw.pos.clone();
        }
        for unit in &mut modeled.economy.units {
            let raw = modeled
                .raw_units
                .iter()
                .find(|x| x.unit_id == unit.unit_id)
                .ok_or("economy unit view missing")?;
            unit.pos = bevy_math::Vec2::new(raw.pos.x, raw.pos.y);
        }
        Ok(modeled)
    }

    fn check_movement(&self, b: &AuditState, a: &AuditState) -> Result<(), String> {
        let hz = f32::from(b.tick_hz.ok_or("movement clock missing")?);
        for old in &b.raw_units {
            let Some(target) = &old.move_target else {
                continue;
            };
            let Some(new) = a.raw_units.iter().find(|u| u.unit_id == old.unit_id) else {
                continue;
            };
            if new.move_target.as_ref() != Some(target) {
                let dx = target.x - old.pos.x;
                let dy = target.y - old.pos.y;
                let d = (dx * dx + dy * dy).sqrt();
                let unit_content = b
                    .units
                    .iter()
                    .find(|x| x.unit_id == old.unit_id)
                    .ok_or("moving unit missing")?
                    .content_id
                    .clone();
                let id = ContentId::parse(&unit_content).map_err(|_| "unit content id invalid")?;
                let speed = b
                    .catalog
                    .as_ref()
                    .and_then(|c| c.units.iter().find(|u| u.id == id))
                    .and_then(|u| u.movement_speed)
                    .ok_or("unit movement speed missing")?;
                if new.move_target.is_some()
                    || d > speed / f32::from(b.tick_hz.unwrap_or(1))
                    || ((new.pos.x - target.x).abs() > 1e-4 || (new.pos.y - target.y).abs() > 1e-4)
                {
                    return Err("movement removed or replaced a target before exact arrival".into());
                }
                continue;
            }
            let unit_content = b
                .units
                .iter()
                .find(|x| x.unit_id == old.unit_id)
                .ok_or("moving unit missing")?
                .content_id
                .clone();
            let unit_id = ContentId::parse(&unit_content).map_err(|_| "unit content id invalid")?;
            let type_def = b
                .catalog
                .as_ref()
                .and_then(|c| c.units.iter().find(|u| u.id == unit_id))
                .ok_or("moving unit type missing")?;
            let speed = type_def
                .movement_speed
                .ok_or("unit with target lacks catalog movement speed")?;
            let dx = target.x - old.pos.x;
            let dy = target.y - old.pos.y;
            let d = (dx * dx + dy * dy).sqrt();
            let step = speed / hz;
            let (x, y) = if d <= step {
                (target.x, target.y)
            } else {
                (old.pos.x + dx * (step / d), old.pos.y + dy * (step / d))
            };
            if (new.pos.x - x).abs() > 1e-4 || (new.pos.y - y).abs() > 1e-4 {
                return Err(format!(
                    "unit {} movement diverged from the independent fixed-step rule",
                    old.unit_id
                ));
            }
        }
        for unit in &a.raw_units {
            if let Some(bounds) = &a.bounds
                && (unit.pos.x < 0.0
                    || unit.pos.y < 0.0
                    || unit.pos.x > bounds.x
                    || unit.pos.y > bounds.y)
            {
                return Err("unit position is outside frozen map bounds".into());
            }
        }
        Ok(())
    }

    pub fn assert_positive_progress(&self) -> Result<(), String> {
        if self.operations == 0 {
            return Err("accounting trace contains no operations".into());
        }
        if self.extraction
            + self.delivery
            + self.conversion
            + self.gross_charge
            + self.refunds
            + u128::from(self.outputs)
            + u128::from(self.cancellations)
            == 0
        {
            return Err("no positive extraction, delivery, payment, cancellation or output within the trace".into());
        }
        Ok(())
    }

    fn append_events(
        &self,
        b: &AuditState,
        a: &AuditState,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        let mut pending = b.pending_events.clone();
        let mut next = b.next_event_ordinal.ok_or("event cursor absent")?;
        for event in events {
            if event.tick != b.economy.tick || event.ordinal != next {
                return Err("operation event tick/ordinal differs from modeled cursor".into());
            }
            pending.push(event.clone());
            next = next.checked_add(1).ok_or("event ordinal exhausted")?;
        }
        if a.pending_events != pending || a.next_event_ordinal != Some(next) {
            return Err(
                "operation event buffer/cursor differs from independently appended events".into(),
            );
        }
        Ok(())
    }
    pub fn assert_command_records(
        &self,
        records: &[turts_sim::CommandAudit],
    ) -> Result<(), String> {
        if records.len() != self.expected_commands.len() {
            return Err("command audit count differs from submitted command ledger".into());
        }
        let mut unmatched = self.expected_commands.clone();
        for record in records {
            let Some(i) = unmatched.iter().position(|c| {
                c.slot == record.input.slot
                    && c.seq == record.input.seq
                    && c.kind == record.input.kind
            }) else {
                return Err(
                    "command audit is missing, duplicated, or differs from submitted input".into(),
                );
            };
            unmatched.remove(i);
        }
        if !unmatched.is_empty() {
            return Err("submitted command lacks exactly one audit record".into());
        }
        Ok(())
    }
}

fn change_inventory_amount(
    state: &mut AuditState,
    unit: u32,
    item: &ContentId,
    delta: i128,
) -> Result<(), String> {
    let inv = state
        .economy
        .inventories
        .iter_mut()
        .find(|x| x.unit == unit)
        .ok_or("transfer inventory absent")?;
    let line = inv
        .items
        .iter_mut()
        .find(|x| &x.resource == item)
        .ok_or("transfer inventory item row absent")?;
    line.on_hand = u64::try_from(i128::from(line.on_hand) + delta)
        .map_err(|_| "transfer expected item overflow")?;
    Ok(())
}

fn append_modeled_events(
    expected: &mut AuditState,
    before: &AuditState,
    events: &[EconomyEvent],
) -> Result<(), String> {
    let mut ordinal = before.next_event_ordinal.ok_or("event cursor missing")?;
    for event in events {
        if event.tick != before.economy.tick || event.ordinal != ordinal {
            return Err("event identity differs from modeled cursor".into());
        }
        expected.pending_events.push(event.clone());
        ordinal = ordinal.checked_add(1).ok_or("event ordinal overflow")?;
    }
    expected.next_event_ordinal = Some(ordinal);
    Ok(())
}

fn apply_cost(
    state: &mut AuditState,
    source: CostSource,
    slot: u8,
    producer: u32,
    item: &ContentId,
    on_delta: i128,
    reserved_delta: i128,
) -> Result<(), String> {
    let (on, res) = match source {
        CostSource::SlotBalance => {
            let x = state
                .economy
                .balances
                .iter_mut()
                .find(|x| x.slot == slot && &x.resource == item)
                .ok_or("balance endpoint absent")?;
            (&mut x.on_hand, &mut x.reserved)
        }
        CostSource::ProducerInventory => {
            let x = state
                .economy
                .inventories
                .iter_mut()
                .find(|x| x.unit == producer)
                .ok_or("inventory endpoint absent")?;
            let y = x
                .items
                .iter_mut()
                .find(|y| &y.resource == item)
                .ok_or("inventory item endpoint absent")?;
            (&mut y.on_hand, &mut y.reserved)
        }
    };
    *on = u64::try_from(i128::from(*on) + on_delta).map_err(|_| "expected on-hand overflow")?;
    *res = u64::try_from(i128::from(*res) + reserved_delta)
        .map_err(|_| "expected reserved overflow")?;
    Ok(())
}

fn check_reservations(s: &AuditState) -> Result<(), String> {
    let mut slots = BTreeMap::<(u8, ContentId), u128>::new();
    let mut inventories = BTreeMap::<(u32, ContentId), u128>::new();
    for claim in &s.ledger_claims {
        if let turts_sim::ReservationClaimView::Active { lines, .. } = claim {
            for line in lines {
                match line.endpoint {
                    turts_sim::ReservationEndpointView::Slot(slot) => {
                        *slots.entry((slot, line.resource.clone())).or_default() +=
                            u128::from(line.amount)
                    }
                    turts_sim::ReservationEndpointView::Inventory(unit) => {
                        *inventories
                            .entry((unit, line.resource.clone()))
                            .or_default() += u128::from(line.amount)
                    }
                }
            }
        }
    }
    for balance in &s.economy.balances {
        let total = slots
            .get(&(balance.slot, balance.resource.clone()))
            .copied()
            .unwrap_or(0);
        if total != u128::from(balance.reserved) {
            return Err(format!(
                "reservation claim sum for slot {} {} differs from reserved balance",
                balance.slot, balance.resource
            ));
        }
        if balance.reserved > balance.on_hand {
            return Err("reserved balance exceeds on-hand stock".into());
        }
    }
    for inv in &s.economy.inventories {
        for item in &inv.items {
            let total = inventories
                .get(&(inv.unit, item.resource.clone()))
                .copied()
                .unwrap_or(0);
            if total != u128::from(item.reserved) {
                return Err(format!(
                    "reservation claim sum for inventory {} {} differs from reserved amount",
                    inv.unit, item.resource
                ));
            }
            if item.reserved > item.on_hand {
                return Err("reserved inventory items exceed on-hand stock".into());
            }
        }
    }
    for ((unit, item), amount) in inventories {
        let stored = inventory_item(s, unit, &item).1;
        if amount != u128::from(stored) {
            return Err("active inventory reservation refers to a missing endpoint item".into());
        }
    }
    Ok(())
}
fn inventory_item(s: &AuditState, unit: u32, item: &ContentId) -> (u64, u64) {
    s.economy
        .inventories
        .iter()
        .find(|i| i.unit == unit)
        .and_then(|i| i.items.iter().find(|x| &x.resource == item))
        .map_or((0, 0), |x| (x.on_hand, x.reserved))
}
fn queue_job(s: &AuditState, producer: u32, job: u64) -> Option<&turts_sim::JobView> {
    s.economy
        .queues
        .iter()
        .find(|q| q.producer == producer)?
        .jobs
        .iter()
        .find(|j| j.id == job)
}

fn endpoint_amount(
    s: &AuditState,
    source: CostSource,
    slot: u8,
    producer: u32,
    item: &ContentId,
) -> u64 {
    match source {
        CostSource::SlotBalance => s
            .economy
            .balances
            .iter()
            .find(|b| b.slot == slot && &b.resource == item)
            .map_or(0, |b| b.on_hand),
        CostSource::ProducerInventory => inventory_item(s, producer, item).0,
    }
}
fn endpoint_reserved(
    s: &AuditState,
    source: CostSource,
    slot: u8,
    producer: u32,
    item: &ContentId,
) -> u64 {
    match source {
        CostSource::SlotBalance => s
            .economy
            .balances
            .iter()
            .find(|b| b.slot == slot && &b.resource == item)
            .map_or(0, |b| b.reserved),
        CostSource::ProducerInventory => inventory_item(s, producer, item).1,
    }
}

fn event_source(source: CostSource, slot: u8, producer: u32) -> turts_sim::EventSource {
    match source {
        CostSource::SlotBalance => turts_sim::EventSource::Slot(slot),
        CostSource::ProducerInventory => turts_sim::EventSource::Inventory(producer),
    }
}
