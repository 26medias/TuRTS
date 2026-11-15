use crate::{
    Owner, RejectReason, SlotId, UnitId, UnitType,
    bootstrap::{InstalledCatalog, ProductionDurations},
    economy::{
        events::{CostLine, EconomyEventKind, EventSource},
        ledger::{Endpoint, Reservation, ReservationLine},
        power, transaction,
    },
    ids::JobId,
    registry::UnitTypes,
    unit_index::unit_entity,
};
use bevy_ecs::{prelude::Resource, world::World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use turts_mod_api::{ContentId, CostSource, PaymentPolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStage {
    Queued,
    Running,
    OutputBlocked,
}
/// Charge-over-time state. Other payment policies retain `None` and the legacy audit shape.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundingState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting_for_funds: Option<ContentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_since_tick: Option<u32>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub recovery_ticks: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_charges: Vec<CostLine>,
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

impl FundingState {
    pub(crate) fn wait(&mut self, resource: ContentId, tick: u32) -> bool {
        let entering = self.waiting_for_funds.as_ref() != Some(&resource);
        self.waiting_for_funds = Some(resource);
        self.held_since_tick.get_or_insert(tick);
        self.recovery_ticks = 0;
        entering
    }

    pub(crate) fn advance(&mut self, tick_hz: u16) {
        self.waiting_for_funds = None;
        if self.held_since_tick.is_some() {
            let threshold = u32::from(tick_hz);
            if self.recovery_ticks < threshold {
                self.recovery_ticks += 1;
            }
            if self.recovery_ticks >= threshold {
                self.held_since_tick = None;
                self.recovery_ticks = 0;
            }
        }
    }

    pub(crate) fn add_charges(&mut self, rows: &[ReservationLine]) -> Result<(), RejectReason> {
        for row in rows {
            let added = event_cost_line(row);
            if let Some(pending) = self.pending_charges.iter_mut().find(|pending| {
                pending.source == added.source && pending.resource == added.resource
            }) {
                pending.amount = pending
                    .amount
                    .checked_add(added.amount)
                    .ok_or(RejectReason::ArithmeticOverflow)?;
            } else {
                self.pending_charges.push(added);
            }
        }
        Ok(())
    }
}

/// Flush exact buffered debits at the operation's existing accounting boundary.
pub(crate) fn flush_funding(
    tx: &mut transaction::Transaction<'_>,
    owner_slot: u8,
    unit: u32,
    job: u64,
    funding: &mut Option<FundingState>,
) -> Result<(), RejectReason> {
    if let Some(funding) = funding
        && !funding.pending_charges.is_empty()
    {
        tx.stage_event(
            owner_slot,
            EconomyEventKind::ProgressCharged {
                unit,
                job,
                charged: std::mem::take(&mut funding.pending_charges),
            },
        )?;
    }
    Ok(())
}

pub(crate) fn payment_lines(
    costs: &[turts_mod_api::ResolvedCost],
    owner: u8,
    inventory: u32,
) -> Vec<ReservationLine> {
    costs
        .iter()
        .map(|cost| ReservationLine {
            endpoint: match cost.source {
                CostSource::SlotBalance => Endpoint::Slot(owner),
                CostSource::ProducerInventory => Endpoint::Inventory(inventory),
            },
            resource: cost.resource.clone(),
            amount: cost.amount,
        })
        .collect()
}

pub(crate) enum PaymentAttempt {
    Funded(super::payment::ProgressCharge),
    Waiting(ContentId),
}

/// Check every authored source, including zero-difference entries, before staging any debit.
pub(crate) fn charge_progress(
    tx: &mut transaction::Transaction<'_>,
    costs: &[ReservationLine],
    previous: u64,
    next: u64,
    total: u64,
) -> Result<PaymentAttempt, RejectReason> {
    let proposal = super::payment::progress_charges(costs, previous, next, total)?;
    for cost in costs {
        tx.validate_endpoint_legacy(cost.endpoint, &cost.resource)?;
        let due = proposal
            .debits
            .iter()
            .find(|row| row.endpoint == cost.endpoint && row.resource == cost.resource)
            .map_or(0, |row| row.amount);
        let account = tx.account(cost.endpoint, &cost.resource);
        let available = account
            .on_hand
            .checked_sub(account.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if available < due {
            return Ok(PaymentAttempt::Waiting(cost.resource.clone()));
        }
    }
    if !proposal.debits.is_empty() {
        transaction::charge_costs(tx, &proposal.debits)?;
    }
    Ok(PaymentAttempt::Funded(proposal))
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductionJob {
    pub(crate) funding: Option<FundingState>,
    pub(crate) id: JobId,
    pub(crate) recipe: ContentId,
    pub(crate) stage: JobStage,
    /// The progress of the job, in units of 1/Q tick (design 6.4 `done`). At Q = 1 these are
    /// ticks. Read it through `job_elapsed_ticks` when a tick count is the wanted unit.
    pub(crate) elapsed_ticks: u64,
    pub(crate) duration_ticks: u64,
    /// The progress that completes the job: `duration_ticks x Q`, with the Q of the catalog at
    /// the time the job was queued (design 6.4 `total`). A job that does not read the rate (a
    /// machine job, a research job) has `total = duration_ticks`.
    pub(crate) total: u64,
    pub(crate) charged: Vec<ReservationLine>,
    pub(crate) reservation: Option<Reservation>,
    /// The job could not pay in full at some tick and has not paid in full on every tick for
    /// `HOLD_CLEAR_MS` since (design 6.4, erratum E21, ruling R-Y). The shape only: `L-PAY`
    /// writes and reads it. It stays false until then.
    #[allow(dead_code)]
    pub(crate) held: bool,
    /// The first tick of the hold; 0 while `held` is false (`L-PAY`).
    #[allow(dead_code)]
    pub(crate) held_since_tick: u32,
}

/// The time of unbroken full payment after which a hold clears: 1,000 ms (20 ticks at 20 Hz).
/// An engine constant, not a mod field (erratum E21, ruling R-Y). The shape only: `L-PAY` uses
/// it.
#[allow(dead_code)]
pub(crate) const HOLD_CLEAR_MS: u32 = 1_000;
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProductionQueues(pub(crate) BTreeMap<u32, Vec<ProductionJob>>);

/// Atomically queue one recipe on an owned producer, applying its declared payment policy.
pub fn queue_production(
    world: &mut World,
    slot: SlotId,
    producer: u32,
    recipe_id: ContentId,
) -> Result<JobId, RejectReason> {
    // Resolve ownership before recipe existence or capability so foreign catalog details stay private.
    let (owner, unit_type) = unit_entity(world, producer)
        .map(|entity| {
            (
                world.get::<Owner>(entity).copied(),
                world.get::<UnitType>(entity).copied(),
            )
        })
        .ok_or(RejectReason::UnknownUnit)?;
    let owner = owner.ok_or(RejectReason::UnknownUnit)?;
    if owner.0 != slot {
        return Err(RejectReason::NotOwner);
    }
    if world
        .get_resource::<super::construction::ConstructionState>()
        .is_some_and(|state| state.sites.contains_key(&producer))
    {
        return Err(RejectReason::NotApplicable);
    }
    let unit_type = unit_type.ok_or(RejectReason::MissingCapability)?;
    let type_entry = world
        .get_resource::<UnitTypes>()
        .and_then(|types| types.get(unit_type.0))
        .ok_or(RejectReason::MissingCapability)?;
    let unit_id =
        ContentId::parse(&type_entry.content_id).map_err(|_| RejectReason::MissingCapability)?;
    let catalog = &world
        .get_resource::<InstalledCatalog>()
        .ok_or(RejectReason::UnknownRecipe)?
        .0;
    if catalog
        .units
        .get(&unit_id)
        .and_then(|unit| unit.research.as_ref())
        .is_some()
    {
        return super::research::queue_research(world, slot, producer, recipe_id);
    }
    let producer_def = catalog
        .units
        .get(&unit_id)
        .and_then(|unit| unit.producer.as_ref())
        .ok_or(RejectReason::MissingCapability)?;
    let recipe = catalog
        .production
        .get(&recipe_id)
        .ok_or(RejectReason::UnknownRecipe)?;
    if !producer_def.recipes.contains(&recipe_id) {
        return Err(RejectReason::RecipeNotEnabled);
    }
    let duration_ticks = world
        .get_resource::<ProductionDurations>()
        .and_then(|durations| durations.0.get(&recipe_id))
        .copied()
        .ok_or(RejectReason::UnknownRecipe)?;
    // The total is fixed when the job is queued (design 6.4: `total = duration_ticks x Q`).
    let total = duration_ticks
        .checked_mul(power::rate_q(world))
        .ok_or(RejectReason::ArithmeticOverflow)?;
    let costs = recipe.costs.clone();
    let policy = recipe.payment;
    let queue_capacity = producer_def.queue_capacity as usize;
    let lines = costs
        .into_iter()
        .map(|cost| ReservationLine {
            endpoint: match cost.source {
                CostSource::SlotBalance => Endpoint::Slot(slot.0),
                CostSource::ProducerInventory => Endpoint::Inventory(producer),
            },
            resource: cost.resource,
            amount: cost.amount,
        })
        .collect::<Vec<_>>();
    let recipe_id_for_job = recipe_id.clone();
    let owner_slot = slot.0;

    transaction::transact(world, |tx| {
        let mut queues = tx.queues();
        let queue = queues.0.entry(producer).or_default();
        if queue.len() >= queue_capacity {
            return Err(RejectReason::QueueFull);
        }
        let job_id = tx.allocate_job_id()?;
        let (charged, reservation) = match policy {
            PaymentPolicy::ChargeOnQueue => {
                transaction::charge_costs(tx, &lines)?;
                (lines.clone(), None)
            }
            PaymentPolicy::ReserveThenCharge => {
                let reservation = transaction::reserve_costs(tx, job_id, &lines)?;
                (Vec::new(), Some(reservation))
            }
            PaymentPolicy::ChargeOverTime => (Vec::new(), None),
        };
        queue.push(ProductionJob {
            funding: (policy == PaymentPolicy::ChargeOverTime).then(FundingState::default),
            id: job_id,
            recipe: recipe_id_for_job.clone(),
            stage: JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks,
            total,
            held: false,
            held_since_tick: 0,
            charged,
            reservation,
        });
        tx.stage_queues(queues);
        tx.stage_event(
            owner_slot,
            EconomyEventKind::ProductionQueued {
                producer,
                job: job_id.0,
                recipe: recipe_id_for_job.clone(),
            },
        )?;
        Ok(job_id)
    })
}

/// Cancel any queued production job atomically with its stage-specific refund.
pub fn cancel_production(
    world: &mut World,
    slot: SlotId,
    producer: u32,
    job: JobId,
) -> Result<(), RejectReason> {
    let owner = unit_entity(world, producer)
        .and_then(|entity| world.get::<Owner>(entity))
        .map(|owner| owner.0)
        .ok_or(RejectReason::UnknownUnit)?;
    if owner != slot {
        return Err(RejectReason::NotOwner);
    }
    if super::research::is_research_station(world, producer) {
        return super::research::cancel_research(world, slot, producer, job);
    }
    if super::machine::machine_ids(world).contains(&producer) {
        return super::machine::cancel_production(world, slot.0, producer, job);
    }
    transaction::transact(world, |tx| {
        let mut queues = tx.queues();
        let queue = queues
            .0
            .get_mut(&producer)
            .ok_or(RejectReason::UnknownJob)?;
        let index = queue
            .iter()
            .position(|entry| entry.id == job)
            .ok_or(RejectReason::UnknownJob)?;
        let mut current = queue[index].clone();
        let catalog = &tx.catalog()?.0;
        let recipe = catalog
            .production
            .get(&current.recipe)
            .ok_or(RejectReason::UnknownRecipe)?;
        let (ratio, released, refund) = match current.stage {
            JobStage::Queued => (
                recipe.cancel_refunds.queued,
                current.reservation.clone(),
                if recipe.payment == PaymentPolicy::ChargeOnQueue {
                    current.charged.clone()
                } else {
                    Vec::new()
                },
            ),
            JobStage::Running => (recipe.cancel_refunds.running, None, current.charged.clone()),
            JobStage::OutputBlocked => (
                recipe.cancel_refunds.output_blocked,
                None,
                current.charged.clone(),
            ),
        };
        if let Some(reservation) = &released {
            transaction::release_reservation(tx, reservation)?;
        }
        if !refund.is_empty() {
            transaction::refund_charged(
                tx,
                &refund,
                u64::from(ratio.numerator),
                u64::from(ratio.denominator),
            )?;
        }
        flush_funding(tx, slot.0, producer, job.0, &mut current.funding)?;
        tx.stage_job_terminal(
            producer,
            &current,
            crate::receipts::TerminalWorkState::Cancelled,
        );
        queue.remove(index);
        tx.stage_queues(queues);
        let refunded = refund
            .iter()
            .map(|line| {
                let mut event = event_cost_line(line);
                event.amount = u64::try_from(
                    u128::from(line.amount) * u128::from(ratio.numerator)
                        / u128::from(ratio.denominator),
                )
                .map_err(|_| RejectReason::ArithmeticOverflow)?;
                Ok(event)
            })
            .collect::<Result<Vec<_>, RejectReason>>()?;
        tx.stage_event(
            slot.0,
            EconomyEventKind::ProductionCancelled {
                producer,
                job: job.0,
                released: released
                    .map_or_else(Vec::new, |r| r.lines.iter().map(event_cost_line).collect()),
                refunded,
            },
        )?;
        Ok(())
    })
}

/// Advance each queue head once. Operating work competes in physical source order.
pub(crate) fn run_production(world: &mut World) {
    let operating = world.contains_resource::<power::OperatingResources>();
    let site_ids = if operating {
        world
            .resource::<super::construction::ConstructionState>()
            .sites
            .keys()
            .copied()
            .collect::<Vec<_>>()
    } else {
        super::construction::advance_sites(world);
        Vec::new()
    };
    let research_ids = super::research::research_station_ids(world);
    super::research::run_research(world);
    let machine_ids = super::machine::machine_ids(world);
    super::machine::run_machines(world);
    let producers = world
        .get_resource::<ProductionQueues>()
        .map(|queues| {
            queues
                .0
                .keys()
                .filter(|producer| {
                    !machine_ids.contains(producer) && !research_ids.contains(producer)
                })
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if operating {
        let mut work = site_ids
            .into_iter()
            .map(|id| (id, true))
            .chain(producers.into_iter().map(|id| (id, false)))
            .filter_map(|(id, site)| {
                unit_entity(world, id)
                    .and_then(|entity| world.get::<crate::Position>(entity))
                    .map(|position| (position.0, id, site))
            })
            .collect::<Vec<_>>();
        let coordinate = |value: f32| if value == 0.0 { 0.0 } else { value };
        work.sort_by(|a, b| {
            coordinate(a.0.x)
                .total_cmp(&coordinate(b.0.x))
                .then_with(|| coordinate(a.0.y).total_cmp(&coordinate(b.0.y)))
        });
        for (index, (position, id, site)) in work.iter().copied().enumerate() {
            // Exact source ties have equal priority. Refuse both instead of selecting an id.
            if index > 0 && work[index - 1].0 == position
                || index + 1 < work.len() && work[index + 1].0 == position
            {
                super::operating::hold(world, id, RejectReason::IntegrityViolation);
                continue;
            }
            if site {
                super::construction::advance_site(world, id);
            } else {
                advance_producer(world, id);
            }
        }
    } else {
        for producer in producers {
            advance_producer(world, producer);
        }
    }
}

fn advance_producer(world: &mut World, producer: u32) {
    let head_stage = world
        .get_resource::<ProductionQueues>()
        .and_then(|queues| queues.0.get(&producer))
        .and_then(|queue| queue.first())
        .map(|job| job.stage);
    if head_stage.is_none() {
        return;
    }
    // Preserve M2 worlds without M3 state, but freeze every frozen legacy producer (a disabled
    // one today). The one predicate of the freeze: `FreezeFacts::frozen`.
    let frozen = freeze_facts_in_world(world, producer).frozen();
    if frozen {
        return;
    }
    let Some(owner_slot) = unit_entity(world, producer)
        .and_then(|entity| world.get::<Owner>(entity))
        .map(|owner| owner.0.0)
    else {
        return;
    };
    if world
        .resource::<ProductionQueues>()
        .0
        .get(&producer)
        .and_then(|queue| queue.first())
        .is_some_and(|job| job.funding.is_some())
    {
        // Include the post-spawn rally order in the same authoritative audit boundary.
        transaction::capture_operation(world, |world| {
            advance_paid_producer(world, producer, owner_slot)
        });
        return;
    }
    if world.contains_resource::<power::OperatingResources>() {
        let ready = world
            .resource::<ProductionQueues>()
            .0
            .get(&producer)
            .and_then(|queue| queue.first())
            .filter(|job| {
                job.stage == JobStage::OutputBlocked
                    || job.stage == JobStage::Running
                        && job.elapsed_ticks.saturating_add(job_step(
                            job,
                            false,
                            power::rate_factor(world, producer),
                        )) >= job.total
            })
            .map(|job| job.recipe.clone());
        if let Some(recipe) = ready
            && let Err(reason) = prepare_output_activation(world, producer, &recipe)
        {
            super::operating::hold(world, producer, reason);
            return;
        }
        super::operating::clear_hold(world, producer);
    }
    if head_stage != Some(JobStage::OutputBlocked) {
        let rate = power::rate_factor(world, producer);
        let _ = transaction::transact(world, |tx| {
            let mut queues = tx.queues();
            let mut changed = false;
            let started = {
                let Some(job) = queues
                    .0
                    .get_mut(&producer)
                    .and_then(|queue| queue.first_mut())
                else {
                    return Ok(());
                };
                match job.stage {
                    JobStage::Queued => {
                        let charged = if let Some(reservation) = job.reservation.take() {
                            transaction::consume_reservation(tx, &reservation)?;
                            reservation.lines
                        } else {
                            job.charged.clone()
                        };
                        job.charged = charged.clone();
                        job.stage = JobStage::Running;
                        changed = true;
                        Some((job.id.0, charged))
                    }
                    JobStage::Running => {
                        let steps = job_step(job, frozen, rate);
                        let elapsed = job
                            .elapsed_ticks
                            .checked_add(
                                steps.min(
                                    job.total
                                        .checked_sub(job.elapsed_ticks)
                                        .ok_or(RejectReason::IntegrityViolation)?,
                                ),
                            )
                            .ok_or(RejectReason::ArithmeticOverflow)?;
                        job.elapsed_ticks = elapsed.min(job.total);
                        if job.elapsed_ticks >= job.total {
                            // Task 7 supplies output allocation; until then a completed head blocks FIFO.
                            job.stage = JobStage::OutputBlocked;
                        }
                        changed = steps > 0;
                        None
                    }
                    JobStage::OutputBlocked => None,
                }
            };
            if changed {
                tx.stage_queues(queues);
            }
            if let Some((job, charged)) = started {
                tx.stage_event(
                    owner_slot,
                    EconomyEventKind::ProductionStarted {
                        producer,
                        job,
                        charged: charged.iter().map(event_cost_line).collect(),
                    },
                )?;
            }
            Ok(())
        });
    }
    let head = world
        .get_resource::<ProductionQueues>()
        .and_then(|q| q.0.get(&producer))
        .and_then(|q| q.first())
        .cloned();
    if let Some(job) = head.filter(|job| job.stage == JobStage::OutputBlocked) {
        let _ = transaction::capture_operation(world, |world| try_output(world, producer, job.id));
    }
}

/// Advance a paid job in one transaction, including its final output when an exit exists.
fn advance_paid_producer(world: &mut World, producer: u32, owner_slot: u8) {
    let Some(head) = world
        .resource::<ProductionQueues>()
        .0
        .get(&producer)
        .and_then(|queue| queue.first())
        .cloned()
    else {
        return;
    };
    if head.stage == JobStage::OutputBlocked {
        let result =
            transaction::capture_operation(world, |world| try_output(world, producer, head.id));
        match result {
            Err(reason) => super::operating::hold(world, producer, reason),
            Ok(_) => super::operating::clear_hold(world, producer),
        }
        return;
    }
    if head.stage == JobStage::Queued {
        let result = transaction::transact(world, |tx| {
            let mut queues = tx.queues();
            let job = queues
                .0
                .get_mut(&producer)
                .and_then(|queue| queue.first_mut())
                .ok_or(RejectReason::UnknownJob)?;
            job.stage = JobStage::Running;
            tx.stage_queues(queues);
            tx.stage_event(
                owner_slot,
                EconomyEventKind::ProductionStarted {
                    producer,
                    job: head.id.0,
                    charged: Vec::new(),
                },
            )?;
            Ok(())
        });
        match result {
            Err(reason) => super::operating::hold(world, producer, reason),
            Ok(()) => super::operating::clear_hold(world, producer),
        }
        return;
    }
    let step = job_step(&head, false, power::rate_factor(world, producer));
    if step == 0 {
        return;
    }
    let Some(next) = head
        .total
        .checked_sub(head.elapsed_ticks)
        .and_then(|left| head.elapsed_ticks.checked_add(step.min(left)))
    else {
        super::operating::hold(world, producer, RejectReason::ArithmeticOverflow);
        return;
    };
    let Some(recipe) = world
        .get_resource::<InstalledCatalog>()
        .and_then(|catalog| catalog.0.production.get(&head.recipe))
        .cloned()
    else {
        return;
    };
    let costs = payment_lines(&recipe.costs, owner_slot, producer);
    let output = if next == head.total {
        // Admission must succeed even when geometry will delay the already-paid output.
        if let Err(reason) = prepare_output_activation(world, producer, &head.recipe) {
            super::operating::hold(world, producer, reason);
            return;
        }
        match prepare_output(world, producer, head.id) {
            Ok(output) => output,
            Err(reason) => {
                super::operating::hold(world, producer, reason);
                return;
            }
        }
    } else {
        None
    };
    let tick = world.resource::<crate::world::Tick>().0;
    let hz = world.resource::<crate::SimClock>().tick_hz;
    let follow = output
        .as_ref()
        .map(|row| (row.owner, row.rally, row.mobile));
    let result = transaction::transact(world, |tx| {
        let mut queues = tx.queues();
        let job = queues
            .0
            .get_mut(&producer)
            .and_then(|queue| queue.first_mut())
            .ok_or(RejectReason::UnknownJob)?;
        match charge_progress(tx, &costs, job.elapsed_ticks, next, job.total)? {
            PaymentAttempt::Waiting(resource) => {
                let entering = job
                    .funding
                    .as_mut()
                    .ok_or(RejectReason::IntegrityViolation)?
                    .wait(resource, tick);
                if entering {
                    flush_funding(tx, owner_slot, producer, job.id.0, &mut job.funding)?;
                }
                job.held = true;
                job.held_since_tick = job
                    .funding
                    .as_ref()
                    .and_then(|state| state.held_since_tick)
                    .unwrap_or(tick);
                tx.stage_queues(queues);
                return Ok(None);
            }
            PaymentAttempt::Funded(payment) => {
                let funding = job
                    .funding
                    .as_mut()
                    .ok_or(RejectReason::IntegrityViolation)?;
                funding.add_charges(&payment.debits)?;
                funding.advance(hz);
                job.held = funding.held_since_tick.is_some();
                job.held_since_tick = funding.held_since_tick.unwrap_or(0);
                job.charged = payment.charged;
                job.elapsed_ticks = next;
                if next == job.total {
                    job.stage = JobStage::OutputBlocked;
                    flush_funding(tx, owner_slot, producer, job.id.0, &mut job.funding)?;
                }
            }
        }
        tx.stage_queues(queues);
        output
            .map(|output| stage_output(tx, producer, head.id, output))
            .transpose()
    });
    match result {
        Ok(spawned) => {
            super::operating::clear_hold(world, producer);
            if let (Some(id), Some((owner, rally, mobile))) = (spawned, follow) {
                let _ = follow_output(world, id, owner, rally, mobile);
            }
        }
        Err(reason) => super::operating::hold(world, producer, reason),
    }
}

/// THE ONE FUNCTION of the rate that a queue job advances in one tick (MP1 review, repair 2): the
/// sim advance steps (`run_production`, `run_machines`, `run_research`) and the wire (`JobProgress`
/// through `view_job_per_tick`) both call it, so the wire rate is the rate that the sim applies.
/// A job advances only while its stage is `Running` and its producer is not frozen (a disabled
/// producer, machine or research station keeps the stage `Running` and does not advance). A job
/// that waits (`Queued`: behind another job, or the tick in which it starts) or whose output is
/// blocked (`OutputBlocked`) advances by 0. `None` when `rate` does not divide `q`.
pub fn job_step_per_tick(
    stage: JobStage,
    frozen: bool,
    rate: turts_rules::Ratio,
    q: u64,
) -> Option<u64> {
    if frozen || stage != JobStage::Running {
        return Some(0);
    }
    turts_rules::progress::rate_step(q, rate)
}

/// The facts that freeze a producer, a machine or a research station: the ONE place of the freeze
/// rule. A frozen producer keeps the stage `Running` and does not advance. The sim skips it with
/// `freeze_facts_in_world(..).frozen()` and the wire freezes it with `freeze_facts_in_view(..)
/// .frozen()`: the same predicate. A new cause (`L-PWR`: no power; `L-PAY`: a held job) is a new
/// field here. `frozen` destructures the struct and both builders list every field, so a cause that
/// one builder or the predicate leaves out does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FreezeFacts {
    /// The producer is disabled (`SetEnabled`).
    pub disabled: bool,
}

impl FreezeFacts {
    /// True when the producer does not advance its jobs.
    pub fn frozen(self) -> bool {
        let Self { disabled } = self;
        disabled
    }
}

/// The freeze facts of `unit` in the world (the sim advance steps read this).
pub(crate) fn freeze_facts_in_world(world: &World, unit: u32) -> FreezeFacts {
    FreezeFacts {
        disabled: world
            .get_resource::<super::research::Disabled>()
            .is_some_and(|disabled| disabled.0.contains(&unit)),
    }
}

/// The freeze facts of `unit` in a view (the wire reads this).
pub fn freeze_facts_in_view(view: &super::views::WorldView, unit: u32) -> FreezeFacts {
    FreezeFacts {
        disabled: view.m3.disabled.binary_search(&unit).is_ok(),
    }
}

/// The progress per tick of a queue job as the view holds it (the wire reads this): the same
/// function as the advance step, with the freeze facts of the view.
///
/// `L-PWR` MUST CHANGE THIS FUNCTION. It passes Q = 1 and the full rate, which is right only while
/// no capability has a `when_short` rate (`power::rate_factor` is the stub `1/1`). Rule (repair
/// follow-up, `RateRecord` doc): the wire `done`, `total` and `per_tick` count raw units of 1/Q
/// tick, so the view must carry the raw progress, the Q and the rate of each producer, and this
/// function must call `job_step_per_tick(stage, frozen, rate, q)` with them and write
/// `per_tick` in raw units (Q = 2, 100 ticks, rate 1/2: `total` 200, `per_tick` 1). A machine and
/// a research station read no rate: they call the function with the full rate.
pub fn view_job_per_tick(view: &super::views::WorldView, producer: u32, stage: JobStage) -> u64 {
    let frozen = freeze_facts_in_view(view, producer).frozen();
    let (rate, q) = view
        .operating
        .as_ref()
        .map_or((power::full_rate(), 1), |operating| {
            let pair = operating
                .rates
                .iter()
                .find(|row| row.unit == producer)
                .map_or((1, 1), |row| row.producer);
            (
                turts_rules::Ratio::new(pair.0, pair.1).expect("validated operating rate"),
                operating.rate_q,
            )
        });
    let held = view
        .operating
        .as_ref()
        .is_some_and(|operating| operating.holds.iter().any(|hold| hold.unit == producer));
    let waiting = view
        .operating
        .as_ref()
        .and_then(|operating| {
            operating.jobs.iter().find(|job| {
                job.unit == producer
                    && job
                        .funding
                        .as_ref()
                        .is_some_and(|state| state.waiting_for_funds.is_some())
            })
        })
        .is_some();
    job_step_per_tick(stage, frozen || held || waiting, rate, q).unwrap_or(0)
}

/// Exact absolute queue counters. The legacy public view remains in physical ticks.
pub fn queue_progress_rows(
    view: &super::views::WorldView,
    queues: &[super::views::QueueView],
    tick: u32,
) -> Vec<super::events::RateRecordRow> {
    queues
        .iter()
        .flat_map(|queue| {
            queue.jobs.iter().map(move |job| {
                let operating = view.operating.as_ref();
                let raw = operating.and_then(|state| {
                    state
                        .jobs
                        .iter()
                        .find(|row| row.unit == queue.producer && row.job == job.id)
                });
                let hold = operating
                    .and_then(|state| state.holds.iter().find(|hold| hold.unit == queue.producer))
                    .filter(|_| job.stage != JobStage::Queued);
                let per_tick = view_job_per_tick(view, queue.producer, job.stage);
                let q = operating.map_or(1, |state| state.rate_q);
                let held = raw.is_some_and(|row| row.held) || hold.is_some();
                super::events::RateRecordRow {
                    production: None,
                    unit: queue.producer,
                    owner_slot: queue.owner_slot,
                    job: Some(job.id),
                    done: raw.map_or(job.elapsed_ticks, |row| row.done),
                    total: raw.map_or(job.duration_ticks, |row| row.total),
                    per_tick,
                    at_tick: tick,
                    state: if let Some(resource) = raw
                        .and_then(|job| job.funding.as_ref())
                        .and_then(|state| state.waiting_for_funds.clone())
                    {
                        super::events::RecordStateRow::WaitingForFunds(resource)
                    } else if hold.is_some() {
                        super::events::RecordStateRow::Held
                    } else {
                        match job.stage {
                            JobStage::OutputBlocked => super::events::RecordStateRow::OutputBlocked,
                            JobStage::Running if per_tick > 0 && per_tick < q => {
                                super::events::RecordStateRow::Slowed
                            }
                            JobStage::Queued | JobStage::Running => {
                                super::events::RecordStateRow::Running
                            }
                        }
                    },
                    charged: if raw.is_some_and(|row| row.funding.is_some()) {
                        job.charged.clone()
                    } else {
                        Vec::new()
                    },
                    held,
                    held_since_tick: hold.map_or_else(
                        || raw.map_or(0, |row| row.held_since_tick),
                        |hold| hold.since_tick,
                    ),
                }
            })
        })
        .collect()
}

/// Send a new absolute point only when prediction or the work state changes.
pub(crate) fn progress_changes(
    before: &super::views::WorldView,
    after: &super::views::WorldView,
) -> Vec<super::events::RateRecordRow> {
    if after.operating.is_none() {
        return Vec::new();
    }
    let old = queue_progress_rows(before, &before.economy.queues, before.tick)
        .into_iter()
        .map(|row| ((row.unit, row.job), row))
        .collect::<BTreeMap<_, _>>();
    queue_progress_rows(after, &after.economy.queues, after.tick)
        .into_iter()
        .filter(|row| {
            let Some(previous) = old.get(&(row.unit, row.job)) else {
                return true;
            };
            let projected = previous
                .done
                .saturating_add(
                    previous
                        .per_tick
                        .saturating_mul(u64::from(row.at_tick.saturating_sub(previous.at_tick))),
                )
                .min(previous.total);
            projected != row.done
                || previous.total != row.total
                || previous.per_tick != row.per_tick
                || previous.state != row.state
                || previous.held != row.held
                || previous.held_since_tick != row.held_since_tick
            // Cumulative charges follow exact progress; do not force one wire point per tick.
        })
        .collect()
}

/// The progress that `job` makes in one tick at `rate`: `Q x num / den` (design 6.4, "Progress in
/// integers"). The Q of the job is `total / duration_ticks`, the Q of its queue time. Design 6.4
/// makes Q the least common multiple of every `Rate` denominator of the catalog, and the packer
/// refuses a catalog that breaks it. So a rate whose denominator does not divide the Q of a job
/// is not an input error: it is a broken invariant of the catalog install (`L-PWR` sets Q there),
/// and the engine stops with a message at once. A silent stall would hide it.
fn job_step(job: &ProductionJob, frozen: bool, rate: turts_rules::Ratio) -> u64 {
    let q = job.total.checked_div(job.duration_ticks).unwrap_or(1);
    job_step_per_tick(job.stage, frozen, rate, q).unwrap_or_else(|| {
        panic!(
            "the rate {}/{} does not divide Q = {q} of job {} (design 6.4: Q is the least common \
             multiple of every Rate denominator of the catalog)",
            rate.numerator(),
            rate.denominator(),
            job.id.0
        )
    })
}

/// The progress of `job` in ticks, for the views: `elapsed_ticks` counts units of 1/Q tick, and
/// `duration_ticks` counts ticks. At Q = 1 (`total == duration_ticks`) this is `elapsed_ticks`
/// itself, so no number of a Q = 1 catalog changes.
pub(crate) fn job_elapsed_ticks(job: &ProductionJob) -> u64 {
    turts_rules::progress::base_ticks_done(job.elapsed_ticks, job.duration_ticks, job.total)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputAttempt {
    Spawned(u32),
    Blocked,
}

/// The owner's rally point, including the engine default for a producer.
pub(crate) fn rally_target(world: &World, producer: u32) -> Option<bevy_math::Vec2> {
    let entity = unit_entity(world, producer)?;
    let kind = world.get::<UnitType>(entity)?.0;
    let entry = world.get_resource::<UnitTypes>()?.get(kind)?;
    let id = ContentId::parse(&entry.content_id).ok()?;
    world
        .get_resource::<InstalledCatalog>()?
        .0
        .units
        .get(&id)?
        .producer
        .as_ref()?;
    if let Some(point) = world
        .get::<crate::orders::UnitOrders>(entity)
        .and_then(|orders| orders.rally)
    {
        return Some(point);
    }
    let pos = world.get::<crate::Position>(entity)?.0;
    let centre = world.get_resource::<crate::MapBounds>()?.size * 0.5;
    let direction = (centre - pos).try_normalize().unwrap_or(bevy_math::Vec2::Y);
    Some(pos + direction * (entry.radius + 4.0))
}

fn prepare_output_activation(
    world: &World,
    producer: u32,
    recipe: &ContentId,
) -> Result<(power::Activation, Option<super::income::IncomeActivation>), RejectReason> {
    let owner = unit_entity(world, producer)
        .and_then(|entity| world.get::<Owner>(entity))
        .ok_or(RejectReason::UnknownUnit)?
        .0
        .0;
    let catalog = &world
        .get_resource::<InstalledCatalog>()
        .ok_or(RejectReason::UnknownRecipe)?
        .0;
    let output = catalog
        .production
        .get(recipe)
        .and_then(|recipe| catalog.units.get(&recipe.output_unit))
        .ok_or(RejectReason::UnknownRecipe)?;
    super::operating::prepare_activation(world, output, owner)
}

fn prepare_output(
    world: &World,
    producer: u32,
    job_id: JobId,
) -> Result<Option<PreparedOutput>, RejectReason> {
    let (owner, pos, producer_type) = unit_entity(world, producer)
        .and_then(|entity| {
            Some((
                world.get::<Owner>(entity)?.0,
                world.get::<crate::Position>(entity)?.0,
                world.get::<UnitType>(entity)?.0,
            ))
        })
        .ok_or(RejectReason::UnknownUnit)?;
    let catalog = world
        .get_resource::<InstalledCatalog>()
        .ok_or(RejectReason::UnknownRecipe)?
        .0
        .clone();
    let type_entry = world
        .get_resource::<UnitTypes>()
        .and_then(|types| types.get(producer_type))
        .ok_or(RejectReason::MissingCapability)?;
    let producer_id =
        ContentId::parse(&type_entry.content_id).map_err(|_| RejectReason::MissingCapability)?;
    let offsets = catalog
        .units
        .get(&producer_id)
        .and_then(|u| u.producer.as_ref())
        .ok_or(RejectReason::MissingCapability)?
        .spawn_offsets
        .clone();
    let queue = world
        .get_resource::<ProductionQueues>()
        .and_then(|q| q.0.get(&producer))
        .ok_or(RejectReason::UnknownJob)?;
    let job = queue
        .first()
        .filter(|j| j.id == job_id)
        .ok_or(RejectReason::UnknownJob)?;
    let recipe = catalog
        .production
        .get(&job.recipe)
        .ok_or(RejectReason::UnknownRecipe)?;
    let output = catalog
        .units
        .get(&recipe.output_unit)
        .ok_or(RejectReason::MissingCapability)?;
    let output_type = world
        .get_resource::<UnitTypes>()
        .and_then(|types| types.find(&recipe.output_unit.to_string()))
        .ok_or(RejectReason::MissingCapability)?;
    let radius = world
        .get_resource::<UnitTypes>()
        .and_then(|types| types.get(output_type))
        .ok_or(RejectReason::MissingCapability)?
        .radius;
    let bounds = world
        .get_resource::<crate::MapBounds>()
        .copied()
        .ok_or(RejectReason::ArithmeticOverflow)?;
    let rally = rally_target(world, producer).ok_or(RejectReason::MissingCapability)?;
    let producer_radius = type_entry.radius;
    let mut circles = Vec::new();
    for e in world.iter_entities() {
        let (Some(id), Some(position), Some(kind)) = (
            e.get::<UnitId>(),
            e.get::<crate::Position>(),
            e.get::<UnitType>(),
        ) else {
            continue;
        };
        let r = world
            .resource::<UnitTypes>()
            .get(kind.0)
            .map_or(0.0, |t| t.radius);
        circles.push((id.0, position.0, r));
    }
    let nodes = world
        .get_resource::<crate::economy::SurfaceNodes>()
        .cloned()
        .unwrap_or_default();
    // Build the existing spatial index once per attempt, never scan every unit per point.
    let points: Vec<_> = circles
        .iter()
        .map(|(id, position, _)| crate::grid::GridPoint {
            id: *id,
            position: *position,
        })
        .collect();
    let max_radius = circles.iter().map(|(_, _, r)| *r).fold(radius, f32::max);
    let grid = crate::grid::UniformGrid::new(&points, (max_radius + radius).max(1.0));
    let free = |point: bevy_math::Vec2| {
        point.x - radius >= 0.0
            && point.y - radius >= 0.0
            && point.x + radius <= bounds.size.x
            && point.y + radius <= bounds.size.y
            && grid
                .within(&points, point, radius + max_radius)
                .into_iter()
                .all(|i| point.distance(circles[i].1) >= radius + circles[i].2)
            && nodes.0.iter().filter(|n| n.remaining > 0).all(|n| {
                point.distance(n.position.0)
                    >= radius + catalog.nodes.get(&n.content_id).map_or(0.0, |d| d.radius)
            })
    };
    let mut candidate = offsets
        .iter()
        .map(|off| pos + bevy_math::Vec2::new(off.x, off.y))
        .find(|p| free(*p));
    let direction = (rally - pos).try_normalize().unwrap_or(bevy_math::Vec2::Y);
    let mut ring = 1;
    while candidate.is_none() {
        let distance = producer_radius + radius + ring as f32 * (2.0 * radius);
        if distance > producer_radius + 12.0 {
            break;
        }
        let count = ((std::f32::consts::TAU * distance / (2.0 * radius)).floor() as usize).max(8);
        for i in 0..count {
            // 0, left 1, right 1, left 2, right 2, ...
            let step = if i % 2 == 1 {
                i.div_ceil(2) as f32
            } else {
                -(i as f32 / 2.0)
            };
            let angle = step * std::f32::consts::TAU / count as f32;
            let (sin, cos) = libm::sincosf(angle);
            let point = pos
                + bevy_math::Vec2::new(
                    direction.x * cos - direction.y * sin,
                    direction.x * sin + direction.y * cos,
                ) * distance;
            if free(point) {
                candidate = Some(point);
                break;
            }
        }
        ring += 1;
    }
    let Some(point) = candidate else {
        return Ok(None);
    };
    let inventory = output
        .inventory
        .as_ref()
        .map(|i| crate::economy::ledger::Inventory {
            capacity: i.capacity,
            accepted_items: i.accepts.iter().cloned().collect(),
            items: BTreeMap::new(),
        });
    let binding =
        crate::boot_m3::resolve_spawn_capability_binding(world, output, point, owner.0, None)?;
    let capability_rows = crate::boot_m3::install_capability_rows(output, binding);
    let activation = if world.contains_resource::<power::OperatingResources>() {
        Some(super::operating::prepare_activation(
            world, output, owner.0,
        )?)
    } else {
        None
    };
    Ok(Some(PreparedOutput {
        owner,
        rally,
        mobile: output.movement.is_some(),
        output_type,
        content: recipe.output_unit.clone(),
        point,
        inventory,
        gatherer: output.gatherer.is_some(),
        producer: output.producer.is_some(),
        capability_rows,
        activation,
    }))
}

struct PreparedOutput {
    owner: SlotId,
    rally: bevy_math::Vec2,
    mobile: bool,
    output_type: turts_mod_api::UnitTypeId,
    content: ContentId,
    point: bevy_math::Vec2,
    inventory: Option<super::ledger::Inventory>,
    gatherer: bool,
    producer: bool,
    capability_rows: crate::boot_m3::PreparedCapabilityRows,
    activation: Option<(power::Activation, Option<super::income::IncomeActivation>)>,
}

fn stage_output(
    tx: &mut transaction::Transaction<'_>,
    producer: u32,
    job_id: JobId,
    output: PreparedOutput,
) -> Result<u32, RejectReason> {
    let mut queues = tx.queues();
    let queue = queues
        .0
        .get_mut(&producer)
        .ok_or(RejectReason::UnknownJob)?;
    if queue
        .first()
        .is_none_or(|job| job.id != job_id || job.stage != JobStage::OutputBlocked)
    {
        return Err(RejectReason::UnknownJob);
    }
    let id = tx.allocate_unit_id()?;
    tx.stage_job_terminal(
        producer,
        &queue[0],
        crate::receipts::TerminalWorkState::Completed,
    );
    queue.remove(0);
    tx.stage_queues(queues);
    tx.stage_spawn(transaction::SpawnWrite {
        id,
        unit_type: output.output_type,
        owner_slot: output.owner.0,
        position: output.point,
        state: crate::world::UnitState {
            unit_id: id,
            content_id: output.content.to_string(),
            owner_slot: output.owner.0,
            facing: None,
            pos: output.point,
        },
        inventory: output.inventory,
        gatherer: output.gatherer,
        producer: output.producer,
        capability_rows: output.capability_rows,
    });
    if let Some((activation, clock)) = output.activation {
        tx.stage_operating_activation(id, activation, clock)?;
    }
    tx.stage_event(
        output.owner.0,
        EconomyEventKind::ProductionCompleted {
            producer,
            job: job_id.0,
            output_unit_id: id,
        },
    )?;
    Ok(id)
}

fn follow_output(
    world: &mut World,
    unit: u32,
    owner: SlotId,
    rally: bevy_math::Vec2,
    mobile: bool,
) -> Result<(), RejectReason> {
    if mobile {
        crate::command::apply_legacy_unit(
            world,
            owner,
            crate::CommandKind::MoveTo {
                unit_ids: vec![unit],
                target: rally,
            },
        )?;
    }
    Ok(())
}

fn try_output(
    world: &mut World,
    producer: u32,
    job_id: JobId,
) -> Result<OutputAttempt, RejectReason> {
    let Some(output) = prepare_output(world, producer, job_id)? else {
        return Ok(OutputAttempt::Blocked);
    };
    let (owner, rally, mobile) = (output.owner, output.rally, output.mobile);
    let id = transaction::transact(world, |tx| stage_output(tx, producer, job_id, output))?;
    follow_output(world, id, owner, rally, mobile)?;
    Ok(OutputAttempt::Spawned(id))
}

pub(crate) fn event_cost_line(line: &ReservationLine) -> CostLine {
    CostLine {
        source: match line.endpoint {
            Endpoint::Slot(slot) => EventSource::Slot(slot),
            Endpoint::Inventory(unit) => EventSource::Inventory(unit),
            Endpoint::Output(unit) => EventSource::Output(unit),
        },
        resource: line.resource.clone(),
        amount: line.amount,
    }
}

#[cfg(test)]
mod m3_disabled_legacy_producer_tests {
    use super::*;
    use crate::{
        Applied, Command, CommandKind, apply_command,
        bootstrap::InstalledCatalog,
        economy::{events::NextEventOrdinal, research::Disabled},
        headless_app,
        world::Position,
    };
    use bevy_math::Vec2;
    use std::collections::BTreeMap;
    use turts_mod_api::{
        CancelRefunds, ContentCatalog, Footprint, MovementDef, PackageId, ProducerDef, RefundRatio,
        ResolvedProducer, ResolvedProduction, ResolvedUnit, UnitDef, UnitRegistry, WorldPos,
    };

    /// A world with one producer (unit 7, slot 0) whose queue holds one running job.
    fn producer_app(elapsed: u64, duration: u64) -> bevy_app::App {
        producer_app_q(elapsed, duration, 1)
    }

    /// The same, with a job that was queued when Q was `q`: its total is `duration x q`.
    fn producer_app_q(elapsed: u64, duration: u64, q: u64) -> bevy_app::App {
        let mut app = headless_app(20);
        let package = PackageId::parse("test").unwrap();
        let producer_content = ContentId::new(&package, "producer");
        let product_content = ContentId::new(&package, "product");
        let recipe_id = ContentId::new(&package, "make_product");
        let offsets = vec![WorldPos { x: 0.0, y: 0.0 }];
        let producer_def = ProducerDef {
            recipes: vec!["test:make_product".into()],
            queue_capacity: 2,
            spawn_offsets: offsets.clone(),
            when_short: Vec::new(),
        };
        let mut types = UnitTypes::default();
        let producer_type = types
            .register_unit_type(
                &package,
                UnitDef {
                    id: "producer".into(),
                    name: "Producer".into(),
                    footprint: Footprint::Circle { radius: 0.5 },
                    movement: None,
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: Some(producer_def.clone()),
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                    armour: None,
                    acquire: None,
                    provides: Vec::new(),
                    consumes: Vec::new(),
                    income: None,
                    spawn_order: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    repairer: None,
                },
            )
            .unwrap();
        let _product_type = types
            .register_unit_type(
                &package,
                UnitDef {
                    id: "product".into(),
                    name: "Product".into(),
                    footprint: Footprint::Circle { radius: 0.25 },
                    movement: Some(MovementDef { speed: 1.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: None,
                    conveyor: None,
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: None,
                    armour: None,
                    acquire: None,
                    provides: Vec::new(),
                    consumes: Vec::new(),
                    income: None,
                    spawn_order: None,
                    constructor: None,
                    placement: None,
                    build_area: None,
                    repairer: None,
                },
            )
            .unwrap();
        let resolved_unit =
            |id: turts_mod_api::ContentId, name: &str, radius, producer, movement| ResolvedUnit {
                repairer: None,
                consumes: Vec::new(),
                income: None,
                provides: Vec::new(),
                acquire: None,
                armour: None,
                constructor: None,
                placement: None,
                build_area: None,
                id,
                name: name.into(),
                footprint: Footprint::Circle { radius },
                movement,
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer,
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            };
        let mut catalog = ContentCatalog {
            bots: Default::default(),

            combat: None,
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        catalog.units.insert(
            producer_content.clone(),
            resolved_unit(
                producer_content,
                "Producer",
                0.5,
                Some(ResolvedProducer {
                    when_short: Vec::new(),

                    recipes: vec![recipe_id.clone()],
                    queue_capacity: 2,
                    spawn_offsets: offsets,
                }),
                None,
            ),
        );
        catalog.units.insert(
            product_content.clone(),
            resolved_unit(
                product_content.clone(),
                "Product",
                0.25,
                None,
                Some(MovementDef { speed: 1.0 }),
            ),
        );
        catalog.production.insert(
            recipe_id.clone(),
            ResolvedProduction {
                id: recipe_id.clone(),
                name: "Product".into(),
                output_unit: product_content,
                duration_ms: 500,
                costs: vec![],
                payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
                cancel_refunds: CancelRefunds {
                    queued: RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    running: RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    output_blocked: RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    destroyed: turts_mod_api::RefundRatio::ZERO,
                },
            },
        );
        app.world_mut().insert_resource(types);
        app.world_mut().insert_resource(InstalledCatalog(catalog));
        app.world_mut().resource_mut::<NextEventOrdinal>().0 = Some(0);
        app.world_mut().spawn((
            UnitId(7),
            Owner(SlotId(0)),
            UnitType(producer_type),
            Position(Vec2::ZERO),
        ));
        app.world_mut().resource_mut::<ProductionQueues>().0.insert(
            7,
            vec![ProductionJob {
                funding: None,
                id: JobId(1),
                recipe: recipe_id,
                stage: JobStage::Running,
                elapsed_ticks: elapsed,
                duration_ticks: duration,
                total: duration * q,
                held: false,
                held_since_tick: 0,
                charged: vec![],
                reservation: None,
            }],
        );
        app
    }

    fn exit_app() -> bevy_app::App {
        let mut app = producer_app(1, 1);
        app.insert_resource(crate::MapBounds {
            size: Vec2::splat(64.0),
        });
        let producer = unit_entity(app.world(), 7).unwrap();
        app.world_mut()
            .entity_mut(producer)
            .insert(Position(Vec2::splat(20.0)));
        app.world_mut()
            .resource_mut::<crate::IdAllocators>()
            .set_next_values(100, 1, 2);
        app.world_mut()
            .resource_mut::<ProductionQueues>()
            .0
            .get_mut(&7)
            .unwrap()[0]
            .stage = JobStage::OutputBlocked;
        app
    }

    fn attempt(app: &mut bevy_app::App) -> OutputAttempt {
        try_output(app.world_mut(), 7, JobId(1)).unwrap()
    }

    fn blocker(app: &mut bevy_app::App, id: u32, point: Vec2) -> bevy_ecs::entity::Entity {
        let kind = app
            .world()
            .resource::<UnitTypes>()
            .find("test:product")
            .unwrap();
        app.world_mut()
            .spawn((
                UnitId(id),
                UnitType(kind),
                Owner(SlotId(0)),
                Position(point),
            ))
            .id()
    }

    #[test]
    fn ex1_full_authored_exits_use_first_ring_point() {
        let mut app = exit_app();
        let centre = Vec2::splat(20.0);
        let offsets = vec![
            turts_mod_api::WorldPos { x: 1.5, y: 0.0 },
            turts_mod_api::WorldPos { x: -1.5, y: 0.0 },
            turts_mod_api::WorldPos { x: 0.0, y: 1.5 },
            turts_mod_api::WorldPos { x: 0.0, y: -1.5 },
        ];
        app.world_mut()
            .resource_mut::<InstalledCatalog>()
            .0
            .units
            .get_mut(&ContentId::parse("test:producer").unwrap())
            .unwrap()
            .producer
            .as_mut()
            .unwrap()
            .spawn_offsets = offsets.clone();
        for (i, p) in offsets.iter().enumerate() {
            blocker(&mut app, 10 + i as u32, centre + Vec2::new(p.x, p.y));
        }
        assert_eq!(attempt(&mut app), OutputAttempt::Spawned(100));
        let entity = unit_entity(app.world(), 100).unwrap();
        let expected = centre + Vec2::ONE.normalize() * 1.25;
        assert!(
            app.world()
                .get::<Position>(entity)
                .unwrap()
                .0
                .distance(expected)
                < 0.0001,
            "EX1: first ring point"
        );
    }

    #[test]
    fn ex3_enclosed_output_waits_then_uses_freed_ground() {
        let mut app = exit_app();
        let package = PackageId::parse("wall").unwrap();
        let wall = UnitDef {
            id: "wall".into(),
            name: "Producer".into(),
            footprint: Footprint::Circle { radius: 20.0 },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
            armour: None,
            acquire: None,
            provides: Vec::new(),
            consumes: Vec::new(),
            income: None,
            spawn_order: None,
            constructor: None,
            placement: None,
            build_area: None,
            repairer: None,
        };
        let kind = app
            .world_mut()
            .resource_mut::<UnitTypes>()
            .register_unit_type(&package, wall)
            .unwrap();
        let blocked = app
            .world_mut()
            .spawn((
                UnitId(50),
                UnitType(kind),
                Owner(SlotId(0)),
                Position(Vec2::splat(20.0)),
            ))
            .id();
        assert_eq!(
            attempt(&mut app),
            OutputAttempt::Blocked,
            "EX3: enclosure must block"
        );
        assert_eq!(
            app.world().resource::<ProductionQueues>().0[&7][0].stage,
            JobStage::OutputBlocked
        );
        app.world_mut().despawn(blocked);
        assert_eq!(
            attempt(&mut app),
            OutputAttempt::Spawned(100),
            "EX3: resume next attempt"
        );
    }

    #[test]
    fn rl2_rally_checks_owner_and_map_bounds() {
        use crate::orders::{OrderCommand, OrderTargetSim, QueueMode};
        let mut app = exit_app();
        for (slot, point, expected) in [
            (0, Vec2::new(30.0, 25.0), None),
            (1, Vec2::new(30.0, 25.0), Some(RejectReason::NotOwner)),
            (0, Vec2::new(100.0, 25.0), Some(RejectReason::TargetInvalid)),
            (
                0,
                Vec2::new(f32::NAN, 25.0),
                Some(RejectReason::TargetNotFinite),
            ),
        ] {
            let command = Command {
                slot: SlotId(slot),
                seq: 1,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: ContentId::parse("test:rally").unwrap(),
                    behaviour: turts_mod_api::Behaviour::SetRally,
                    target_kind: turts_mod_api::TargetKind::Point,
                    policy: None,
                    units: vec![7],
                    target: OrderTargetSim::Point(point),
                    queue: QueueMode::Replace,
                    group: 0,
                    allowed_units: vec![],
                    queue_depth: 16,
                })),
            };
            assert_eq!(
                apply_command(app.world_mut(), &command).outcome.err(),
                expected,
                "RL2: slot {slot}, point {point}"
            );
        }
        assert_eq!(rally_target(app.world(), 7), Some(Vec2::new(30.0, 25.0)));
    }

    fn train_thirty() -> Vec<Vec<(u32, Vec2)>> {
        let mut app = exit_app();
        let template = app.world().resource::<ProductionQueues>().0[&7][0].clone();
        let mut traces = Vec::new();
        for n in 0..30 {
            if n > 0 {
                app.world_mut()
                    .resource_mut::<ProductionQueues>()
                    .0
                    .get_mut(&7)
                    .unwrap()
                    .push(template.clone());
            }
            assert!(
                matches!(attempt(&mut app), OutputAttempt::Spawned(_)),
                "EX2: production stopped at {n}"
            );
            for _ in 0..150 {
                app.update();
                let states = crate::world::snapshot(app.world_mut());
                for (i, a) in states.iter().enumerate() {
                    for b in &states[i + 1..] {
                        let min = if a.unit_id == 7 || b.unit_id == 7 {
                            0.75
                        } else {
                            0.5
                        };
                        assert!(
                            a.pos.distance(b.pos) >= min - 0.0001,
                            "EX2: overlap {} {}",
                            a.unit_id,
                            b.unit_id
                        );
                    }
                }
                traces.push(states.iter().map(|s| (s.unit_id, s.pos)).collect());
            }
            if n == 0 {
                let entity = unit_entity(app.world(), 100).unwrap();
                assert!(
                    app.world()
                        .get::<Position>(entity)
                        .unwrap()
                        .0
                        .distance(rally_target(app.world(), 7).unwrap())
                        < 1.5,
                    "RL1: spawned unit did not walk to default rally"
                );
            }
        }
        assert_eq!(
            crate::world::snapshot(app.world_mut()).len(),
            31,
            "EX2: all thirty exist"
        );
        traces
    }

    #[test]
    fn ex2_rl1_thirty_outputs_move_without_overlap() {
        train_thirty();
    }

    #[test]
    fn ex4_rally_positions_are_deterministic_each_tick() {
        assert_eq!(train_thirty(), train_thirty(), "EX4: positions differ");
    }

    #[test]
    fn legacy_producer_freezes_resumes_and_can_cancel_while_disabled() {
        let mut app = producer_app(3, 10);

        let set_enabled = |world: &mut bevy_ecs::world::World, enabled| {
            apply_command(
                world,
                &Command {
                    slot: SlotId(0),
                    seq: 1,
                    kind: CommandKind::SetEnabled {
                        unit_id: 7,
                        enabled,
                    },
                },
            )
        };
        assert_eq!(
            set_enabled(app.world_mut(), false).outcome,
            Ok(Applied { job_id: None })
        );
        assert!(app.world().resource::<Disabled>().0.contains(&7));
        run_production(app.world_mut());
        assert_eq!(
            app.world().resource::<ProductionQueues>().0[&7][0].elapsed_ticks,
            3
        );

        assert_eq!(
            set_enabled(app.world_mut(), true).outcome,
            Ok(Applied { job_id: None })
        );
        run_production(app.world_mut());
        assert_eq!(
            app.world().resource::<ProductionQueues>().0[&7][0].elapsed_ticks,
            4
        );

        assert_eq!(
            set_enabled(app.world_mut(), false).outcome,
            Ok(Applied { job_id: None })
        );
        let cancelled = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: 2,
                kind: CommandKind::CancelProduction {
                    producer_id: 7,
                    job_id: JobId(1),
                },
            },
        );
        assert_eq!(cancelled.outcome, Ok(Applied { job_id: None }));
        assert!(app.world().resource::<Disabled>().0.contains(&7));
        assert!(app.world().resource::<ProductionQueues>().0[&7].is_empty());
    }

    fn elapsed_of_job(app: &bevy_app::App) -> u64 {
        app.world().resource::<ProductionQueues>().0[&7][0].elapsed_ticks
    }

    #[test]
    fn a_job_advances_at_the_rate_of_the_power_stub() {
        use crate::economy::power::PowerProbe;
        use turts_rules::Ratio;
        // Full rate: one unit of progress in each tick.
        let mut full = producer_app(0, 100);
        full.world_mut().insert_resource(PowerProbe {
            weapon_enabled: true,
            rate: Ratio::ONE,
        });
        for _ in 0..10 {
            full.update();
        }
        assert_eq!(elapsed_of_job(&full), 10);
        // Half rate, with Q = 2 (the denominator of the one rate of the catalog): each tick adds
        // 1 unit of 1/2 tick at 1/2 and 2 at full rate, so 10 ticks give 10 against 20.
        for (rate, expected) in [(Ratio::new(1, 2).unwrap(), 10), (Ratio::ONE, 20)] {
            let mut app = producer_app_q(0, 100, 2);
            app.world_mut().insert_resource(PowerProbe {
                weapon_enabled: true,
                rate,
            });
            for _ in 0..10 {
                app.update();
            }
            assert_eq!(
                elapsed_of_job(&app),
                expected,
                "the job ignored the rate factor at {}/{}",
                rate.numerator(),
                rate.denominator()
            );
        }
        // No probe: the stub itself gives full rate, as the code did before the stub.
        let mut stub = producer_app(0, 100);
        for _ in 0..10 {
            stub.update();
        }
        assert_eq!(elapsed_of_job(&stub), 10);
    }

    /// The ticks that the head job of producer 7 takes to reach `OutputBlocked`, when the first
    /// tick of the run is `first_tick` and `rate_at(n)` is the rate of the n-th tick of the run
    /// (n from 1). `None` when it takes more than 500 ticks.
    fn ticks_to_finish(
        app: &mut bevy_app::App,
        first_tick: u32,
        rate_at: impl Fn(u32) -> turts_rules::Ratio,
    ) -> Option<u32> {
        use crate::economy::power::PowerProbe;
        app.world_mut().resource_mut::<crate::world::Tick>().0 = first_tick - 1;
        for n in 1..=500 {
            app.world_mut().insert_resource(PowerProbe {
                weapon_enabled: true,
                rate: rate_at(n),
            });
            app.update();
            let queues = app.world().resource::<ProductionQueues>();
            if queues.0[&7][0].stage == JobStage::OutputBlocked {
                return Some(n);
            }
        }
        None
    }

    #[test]
    fn a_job_at_a_low_rate_takes_duration_over_rate_ticks_at_every_start_phase() {
        use turts_rules::Ratio;
        // Design 6.4: `total = duration x Q`, each tick adds `Q x num / den`. Rate 1/2 with
        // duration 3 takes 6 ticks; rate 1/3 with duration 10 takes 30. The start phase (the
        // tick number of the first tick of the job) must not change the count.
        let half = Ratio::new(1, 2).unwrap();
        for phase in 1..=2 {
            let mut app = producer_app_q(0, 3, 2);
            assert_eq!(
                ticks_to_finish(&mut app, phase, |_| half),
                Some(6),
                "rate 1/2, duration 3, first tick {phase}"
            );
        }
        let third = Ratio::new(1, 3).unwrap();
        for phase in 1..=3 {
            let mut app = producer_app_q(0, 10, 3);
            assert_eq!(
                ticks_to_finish(&mut app, phase, |_| third),
                Some(30),
                "rate 1/3, duration 10, first tick {phase}"
            );
        }
    }

    #[test]
    fn a_job_that_changes_rate_keeps_the_progress_it_made() {
        use turts_rules::Ratio;
        // Duration 10 and Q = 2: total 20. Five ticks at 1/1 add 2 each (10), then 1/2 adds 1
        // each: 10 more ticks. The whole job takes 15 ticks.
        let half = Ratio::new(1, 2).unwrap();
        for phase in 1..=2 {
            let mut app = producer_app_q(0, 10, 2);
            let ticks =
                ticks_to_finish(&mut app, phase, |n| if n <= 5 { Ratio::ONE } else { half });
            assert_eq!(ticks, Some(15), "first tick {phase}");
            assert_eq!(
                elapsed_of_job(&app),
                20,
                "the progress is in units of 1/Q tick"
            );
        }
    }

    #[test]
    #[should_panic(expected = "does not divide")]
    fn a_rate_whose_denominator_does_not_divide_q_is_loud_and_not_a_silent_stall() {
        use crate::economy::power::PowerProbe;
        // Design 6.4: Q is the least common multiple of every `Rate` denominator, so each tick
        // adds a whole number. A job queued at Q = 1 that meets a rate of 1/2 breaks that
        // invariant of the catalog install: the engine stops with a message at once. (Before this
        // repair the transaction failed, its error was dropped, and the job never advanced.)
        let mut app = producer_app(0, 3);
        app.world_mut().insert_resource(PowerProbe {
            weapon_enabled: true,
            rate: turts_rules::Ratio::new(1, 2).unwrap(),
        });
        app.update();
    }

    #[test]
    fn a_job_queued_at_q_2_at_rate_1_2_takes_two_times_its_duration() {
        use crate::economy::power::{PowerProbe, RateQ};
        use turts_rules::Ratio;
        // The job gets its total when it is queued, from the Q of that moment (design 6.4).
        let mut app = producer_app(0, 3);
        app.world_mut().insert_resource(RateQ(2));
        app.world_mut().resource_mut::<ProductionQueues>().0.clear();
        let recipe = ContentId::parse("test:make_product").unwrap();
        app.world_mut()
            .insert_resource(crate::bootstrap::ProductionDurations(BTreeMap::from([(
                recipe.clone(),
                3,
            )])));
        queue_production(app.world_mut(), SlotId(0), 7, recipe).unwrap();
        let job = &app.world().resource::<ProductionQueues>().0[&7][0];
        assert_eq!(
            job.total,
            job.duration_ticks * 2,
            "the queued total is duration x Q"
        );
        let duration = job.duration_ticks;
        // The job starts in the first tick (Queued to Running), then runs at 1/2.
        let half = Ratio::new(1, 2).unwrap();
        let mut ticks = 0;
        loop {
            ticks += 1;
            assert!(ticks < 10_000, "the job never finished");
            app.world_mut().insert_resource(PowerProbe {
                weapon_enabled: true,
                rate: half,
            });
            app.update();
            if app.world().resource::<ProductionQueues>().0[&7][0].stage == JobStage::OutputBlocked
            {
                break;
            }
        }
        assert_eq!(
            ticks,
            1 + 2 * duration,
            "one start tick, then 2 x duration ticks at 1/2"
        );
    }

    #[test]
    fn the_views_show_ticks_and_not_units_of_one_over_q() {
        use crate::economy::power::PowerProbe;
        use crate::economy::views::world_view;
        use turts_rules::Ratio;
        // A job queued at Q = 2, 4 ticks long: total 8. Four ticks at 1/2 add 1 unit each: the
        // progress is 4 of 8 units, which is 2 of 4 ticks. The views show the pair in ticks.
        let mut app = producer_app_q(0, 4, 2);
        app.world_mut().insert_resource(PowerProbe {
            weapon_enabled: true,
            rate: Ratio::new(1, 2).unwrap(),
        });
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            elapsed_of_job(&app),
            4,
            "the raw progress is in units of 1/Q tick"
        );
        let view = world_view(app.world_mut());
        let job = &view
            .economy
            .queues
            .iter()
            .find(|queue| queue.producer == 7)
            .unwrap()
            .jobs[0];
        assert_eq!((job.elapsed_ticks, job.duration_ticks), (2, 4));
        // The audit view of the queue shows the same unit.
        let jobs = app.world().resource::<ProductionQueues>().0[&7].clone();
        let raw = crate::economy::views::raw_queue_view((7, jobs));
        let raw_job = &raw.jobs[0];
        assert_eq!((raw_job.elapsed_ticks, raw_job.duration_ticks), (2, 4));
    }

    #[test]
    fn the_producer_sites_read_the_unit_index_and_do_not_scan() {
        use crate::unit_index::index_entry_for_test;
        let recipe = ContentId::parse("test:make_product").unwrap();
        // The advance of a job reads the owner of the producer.
        let mut app = producer_app(3, 10);
        run_production(app.world_mut());
        assert_eq!(elapsed_of_job(&app), 4);
        let mut app = producer_app(3, 10);
        index_entry_for_test(app.world_mut(), 7);
        run_production(app.world_mut());
        assert_eq!(
            elapsed_of_job(&app),
            3,
            "run_production scanned for the owner"
        );
        // The queue and cancel commands read the owner and the type.
        assert_eq!(
            queue_production(app.world_mut(), SlotId(0), 7, recipe),
            Err(RejectReason::UnknownUnit),
            "queue_production scanned"
        );
        assert_eq!(
            cancel_production(app.world_mut(), SlotId(0), 7, JobId(1)),
            Err(RejectReason::UnknownUnit),
            "cancel_production scanned"
        );
        // The output step reads the owner, the position and the type. With the real unit the
        // call goes past the lookup (it stops later, for a reason of the fixture); with an
        // entry that has an owner and no position, the lookup itself fails.
        let mut control = producer_app(10, 10);
        assert_ne!(
            try_output(control.world_mut(), 7, JobId(1)),
            Err(RejectReason::UnknownUnit)
        );
        let mut indexed = producer_app(10, 10);
        let entry = index_entry_for_test(indexed.world_mut(), 7);
        indexed
            .world_mut()
            .entity_mut(entry)
            .insert(Owner(SlotId(0)));
        assert_eq!(
            try_output(indexed.world_mut(), 7, JobId(1)),
            Err(RejectReason::UnknownUnit),
            "try_output scanned for the producer"
        );
    }

    #[test]
    fn a_job_starts_with_no_hold_and_the_hold_clear_time_is_one_second() {
        // The shape of erratum E21: lane 0 gives the fields and the constant; `L-PAY` writes them.
        let app = producer_app(0, 100);
        let job = &app.world().resource::<ProductionQueues>().0[&7][0];
        assert!(!job.held);
        assert_eq!(job.held_since_tick, 0);
        assert_eq!(HOLD_CLEAR_MS, 1_000);
        assert_eq!(
            crate::boot_m3::try_ms_to_ticks(u64::from(HOLD_CLEAR_MS), 20),
            Some(20)
        );
    }
}

#[cfg(test)]
mod rate_tests {
    use super::*;
    use crate::economy::power::full_rate;

    /// The one function of the rate (repair 2 of L0-WIRE): every stalled condition gives 0, and
    /// only a `Running` job of a producer that is not frozen advances. The wire reads the same
    /// function (`view_job_per_tick`), so this table is also the table of the wire rate.
    #[test]
    fn only_a_running_job_of_a_producer_that_is_not_frozen_advances() {
        let step = |stage, frozen| job_step_per_tick(stage, frozen, full_rate(), 1);
        assert_eq!(step(JobStage::Running, false), Some(1));
        assert_eq!(
            step(JobStage::Running, true),
            Some(0),
            "a disabled producer"
        );
        assert_eq!(
            step(JobStage::Queued, false),
            Some(0),
            "behind another job, or the start tick"
        );
        assert_eq!(step(JobStage::Queued, true), Some(0));
        assert_eq!(
            step(JobStage::OutputBlocked, false),
            Some(0),
            "the output is blocked"
        );
        assert_eq!(step(JobStage::OutputBlocked, true), Some(0));
        // A rate that Q does not divide has no step (the advance step stops with a message).
        let half = turts_rules::Ratio::new(1, 2).expect("a ratio");
        assert_eq!(job_step_per_tick(JobStage::Running, false, half, 1), None);
        assert_eq!(
            job_step_per_tick(JobStage::Running, false, half, 2),
            Some(1)
        );
    }

    /// The view-level rate reads the disabled set of the view, as the advance step reads the
    /// `Disabled` resource.
    #[test]
    fn the_view_rate_reads_the_disabled_set() {
        let mut view = crate::economy::views::WorldView::default();
        assert_eq!(view_job_per_tick(&view, 7, JobStage::Running), 1);
        view.m3.disabled = vec![7];
        assert_eq!(view_job_per_tick(&view, 7, JobStage::Running), 0);
        assert_eq!(view_job_per_tick(&view, 8, JobStage::Running), 1);
        assert_eq!(view_job_per_tick(&view, 8, JobStage::OutputBlocked), 0);
    }
}
