use crate::{
    Owner, RejectReason, SlotId, UnitId, UnitType,
    bootstrap::{InstalledCatalog, ProductionDurations},
    economy::{
        events::{CostLine, EconomyEventKind, EventSource},
        ledger::{Endpoint, Reservation, ReservationLine},
        transaction,
    },
    ids::JobId,
    registry::UnitTypes,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductionJob {
    pub(crate) id: JobId,
    pub(crate) recipe: ContentId,
    pub(crate) stage: JobStage,
    pub(crate) elapsed_ticks: u64,
    pub(crate) duration_ticks: u64,
    pub(crate) charged: Vec<ReservationLine>,
    pub(crate) reservation: Option<Reservation>,
}
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
    let (owner, unit_type) = world
        .iter_entities()
        .find_map(|entity| {
            let id = entity.get::<UnitId>()?;
            (id.0 == producer).then(|| {
                (
                    entity.get::<Owner>().copied(),
                    entity.get::<UnitType>().copied(),
                )
            })
        })
        .ok_or(RejectReason::UnknownUnit)?;
    let owner = owner.ok_or(RejectReason::UnknownUnit)?;
    if owner.0 != slot {
        return Err(RejectReason::NotOwner);
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
        };
        queue.push(ProductionJob {
            id: job_id,
            recipe: recipe_id_for_job.clone(),
            stage: JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks,
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

/// Advance the head of each producer queue once, in ascending producer-id order.
pub(crate) fn run_production(world: &mut World) {
    let producers = world
        .get_resource::<ProductionQueues>()
        .map(|queues| queues.0.keys().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    for producer in producers {
        let Some(owner_slot) = world.iter_entities().find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|id| id.0 == producer))
                .then(|| entity.get::<Owner>().map(|owner| owner.0.0))?
        }) else {
            continue;
        };
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
                        let elapsed = job
                            .elapsed_ticks
                            .checked_add(1)
                            .ok_or(RejectReason::ArithmeticOverflow)?;
                        job.elapsed_ticks = elapsed.min(job.duration_ticks);
                        if job.elapsed_ticks >= job.duration_ticks {
                            // Task 7 supplies output allocation; until then a completed head blocks FIFO.
                            job.stage = JobStage::OutputBlocked;
                        }
                        changed = true;
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
}

fn event_cost_line(line: &ReservationLine) -> CostLine {
    CostLine {
        source: match line.endpoint {
            Endpoint::Slot(slot) => EventSource::Slot(slot),
            Endpoint::Inventory(unit) => EventSource::Inventory(unit),
        },
        resource: line.resource.clone(),
        amount: line.amount,
    }
}
