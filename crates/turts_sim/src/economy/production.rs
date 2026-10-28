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

/// Cancel any queued production job atomically with its stage-specific refund.
pub fn cancel_production(
    world: &mut World,
    slot: SlotId,
    producer: u32,
    job: JobId,
) -> Result<(), RejectReason> {
    let owner = world
        .iter_entities()
        .find_map(|e| {
            (e.get::<UnitId>().is_some_and(|id| id.0 == producer))
                .then(|| e.get::<Owner>().map(|o| o.0))?
        })
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
        let current = queue[index].clone();
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

/// Advance the head of each producer queue once, in ascending producer-id order.
pub(crate) fn run_production(world: &mut World) {
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
    for producer in producers {
        let head_stage = world
            .get_resource::<ProductionQueues>()
            .and_then(|queues| queues.0.get(&producer))
            .and_then(|queue| queue.first())
            .map(|job| job.stage);
        if head_stage.is_none() {
            continue;
        }
        let Some(owner_slot) = world.iter_entities().find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|id| id.0 == producer))
                .then(|| entity.get::<Owner>().map(|owner| owner.0.0))?
        }) else {
            continue;
        };
        if head_stage != Some(JobStage::OutputBlocked) {
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
        let head = world
            .get_resource::<ProductionQueues>()
            .and_then(|q| q.0.get(&producer))
            .and_then(|q| q.first())
            .cloned();
        if let Some(job) = head.filter(|job| job.stage == JobStage::OutputBlocked) {
            let _ =
                transaction::capture_operation(world, |world| try_output(world, producer, job.id));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputAttempt {
    Spawned(u32),
    Blocked,
}

fn try_output(
    world: &mut World,
    producer: u32,
    job_id: JobId,
) -> Result<OutputAttempt, RejectReason> {
    let (owner, pos, producer_type) = world
        .iter_entities()
        .find_map(|e| {
            (e.get::<UnitId>().is_some_and(|id| id.0 == producer))
                .then(|| {
                    Some((
                        e.get::<Owner>()?.0,
                        e.get::<crate::Position>()?.0,
                        e.get::<UnitType>()?.0,
                    ))
                })
                .flatten()
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
        .filter(|j| j.id == job_id && j.stage == JobStage::OutputBlocked)
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
    let candidate = offsets
        .iter()
        .map(|off| bevy_math::Vec2::new(pos.x + off.x, pos.y + off.y))
        .find(|point| {
            point.x - radius >= 0.0
                && point.y - radius >= 0.0
                && point.x + radius <= bounds.size.x
                && point.y + radius <= bounds.size.y
                && circles
                    .iter()
                    .all(|(_, p, r)| point.distance(*p) >= radius + *r)
                && nodes.0.iter().filter(|n| n.remaining > 0).all(|n| {
                    point.distance(n.position.0)
                        >= radius + catalog.nodes.get(&n.content_id).map_or(0.0, |d| d.radius)
                })
        });
    let Some(point) = candidate else {
        return Ok(OutputAttempt::Blocked);
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
    transaction::transact(world, |tx| {
        let mut queues = tx.queues();
        let queue = queues
            .0
            .get_mut(&producer)
            .ok_or(RejectReason::UnknownJob)?;
        if queue
            .first()
            .is_none_or(|j| j.id != job_id || j.stage != JobStage::OutputBlocked)
        {
            return Err(RejectReason::UnknownJob);
        }
        let id = tx.allocate_unit_id()?;
        queue.remove(0);
        tx.stage_queues(queues);
        tx.stage_spawn(transaction::SpawnWrite {
            id,
            unit_type: output_type,
            owner_slot: owner.0,
            position: point,
            state: crate::world::UnitState {
                unit_id: id,
                content_id: recipe.output_unit.to_string(),
                owner_slot: owner.0,
                pos: point,
            },
            inventory,
            gatherer: output.gatherer.is_some(),
            producer: output.producer.is_some(),
            capability_rows,
        });
        tx.stage_event(
            owner.0,
            EconomyEventKind::ProductionCompleted {
                producer,
                job: job_id.0,
                output_unit_id: id,
            },
        )?;
        Ok(OutputAttempt::Spawned(id))
    })
}

fn event_cost_line(line: &ReservationLine) -> CostLine {
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
