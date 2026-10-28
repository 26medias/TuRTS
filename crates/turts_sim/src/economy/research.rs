//! Per-slot research completion and derived technology effects.

use std::collections::BTreeSet;

use bevy_ecs::{prelude::Resource, world::World};
use turts_mod_api::{
    ContentCatalog, ContentId, CostSource, PaymentPolicy, ResolvedEffect, WeaponStat,
};

use crate::{
    Owner, RejectReason, SlotId, UnitId, UnitType,
    bootstrap::{InstalledCatalog, ProductionDurations},
    economy::{
        events::{CostLine, EconomyEventKind, EventSource},
        ledger::{Endpoint, ReservationLine},
        production::{JobStage, ProductionJob, ProductionQueues},
        transaction,
    },
    ids::JobId,
    registry::UnitTypes,
};

/// Completed technologies indexed by owning slot.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResearchState {
    pub(crate) completed: BTreeSet<(u8, ContentId)>,
}

/// Unit ids whose toggleable capabilities are disabled.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Disabled(pub(crate) BTreeSet<u32>);

/// Compute a modifier from the completed set and catalog. The definition is never mutated.
#[allow(dead_code)] // Used from Task 11 combat.
pub(crate) fn modifier(
    research: &ResearchState,
    catalog: &ContentCatalog,
    slot: u8,
    unit_def: &ContentId,
    stat: WeaponStat,
) -> Result<u64, RejectReason> {
    let mut sum = 0_u128;
    for (_, technology) in research
        .completed
        .iter()
        .filter(|(owner, _)| *owner == slot)
    {
        let Some(definition) = catalog.technologies.get(technology) else {
            continue;
        };
        for effect in &definition.effects {
            let ResolvedEffect::WeaponModifier {
                unit,
                stat: effect_stat,
                add,
            } = effect;
            if unit == unit_def && *effect_stat == stat {
                sum = sum
                    .checked_add(u128::from(*add))
                    .ok_or(RejectReason::ArithmeticOverflow)?;
            }
        }
    }
    u64::try_from(sum).map_err(|_| RejectReason::ArithmeticOverflow)
}

/// Queue research after ownership and capability checks, using production's payment primitives.
pub(crate) fn queue_research(
    world: &mut World,
    slot: SlotId,
    station: u32,
    technology: ContentId,
) -> Result<JobId, RejectReason> {
    let (owner, station_def) = station_owner_and_def(world, station)?;
    if owner != slot.0 {
        return Err(RejectReason::NotOwner);
    }
    let station_def = station_def.ok_or(RejectReason::MissingCapability)?;
    if !station_def.technologies.contains(&technology) {
        return Err(RejectReason::RecipeNotEnabled);
    }
    let (definition, duration_ticks) = {
        let catalog = world
            .get_resource::<InstalledCatalog>()
            .ok_or(RejectReason::UnknownRecipe)?;
        let definition = catalog
            .0
            .technologies
            .get(&technology)
            .cloned()
            .ok_or(RejectReason::UnknownRecipe)?;
        let duration_ticks = world
            .get_resource::<ProductionDurations>()
            .and_then(|durations| durations.0.get(&technology))
            .copied()
            .ok_or(RejectReason::UnknownRecipe)?;
        (definition, duration_ticks)
    };
    let stations = research_stations_for_slot(world, slot.0);
    let costs = definition
        .costs
        .iter()
        .map(|cost| ReservationLine {
            endpoint: match cost.source {
                CostSource::SlotBalance => Endpoint::Slot(slot.0),
                CostSource::ProducerInventory => Endpoint::Inventory(station),
            },
            resource: cost.resource.clone(),
            amount: cost.amount,
        })
        .collect::<Vec<_>>();
    transaction::transact(world, |tx| {
        tx.mark_m3();
        for prerequisite in &definition.prerequisites {
            if !tx.research_complete(slot.0, prerequisite) {
                return Err(RejectReason::PrerequisitesMissing);
            }
        }
        if tx.research_complete(slot.0, &technology) {
            return Err(RejectReason::AlreadyResearched);
        }
        let queues = tx.queues();
        if stations.iter().any(|id| {
            queues
                .0
                .get(id)
                .is_some_and(|queue| queue.iter().any(|job| job.recipe == technology))
        }) {
            return Err(RejectReason::ResearchInProgress);
        }
        let mut queues = queues;
        let queue = queues.0.entry(station).or_default();
        if queue.len() >= usize::from(station_def.queue_capacity) {
            return Err(RejectReason::QueueFull);
        }
        let job_id = tx.allocate_job_id()?;
        let (charged, reservation) = match definition.payment {
            PaymentPolicy::ChargeOnQueue => {
                transaction::charge_costs(tx, &costs)?;
                (costs.clone(), None)
            }
            PaymentPolicy::ReserveThenCharge => {
                let reservation = transaction::reserve_costs(tx, job_id, &costs)?;
                (Vec::new(), Some(reservation))
            }
        };
        queue.push(ProductionJob {
            id: job_id,
            recipe: technology.clone(),
            stage: JobStage::Queued,
            elapsed_ticks: 0,
            duration_ticks,
            charged,
            reservation,
        });
        tx.stage_queues(queues);
        tx.stage_event(
            slot.0,
            EconomyEventKind::ProductionQueued {
                producer: station,
                job: job_id.0,
                recipe: technology,
            },
        )?;
        Ok(job_id)
    })
}

/// Cancel a research job. Disabled stations still accept cancellation.
pub(crate) fn cancel_research(
    world: &mut World,
    slot: SlotId,
    station: u32,
    job_id: JobId,
) -> Result<(), RejectReason> {
    let (owner, _) = station_owner_and_def(world, station)?;
    if owner != slot.0 {
        return Err(RejectReason::NotOwner);
    }
    transaction::transact(world, |tx| {
        tx.mark_m3();
        let mut queues = tx.queues();
        let queue = queues.0.get_mut(&station).ok_or(RejectReason::UnknownJob)?;
        let index = queue
            .iter()
            .position(|job| job.id == job_id)
            .ok_or(RejectReason::UnknownJob)?;
        let job = queue[index].clone();
        let technology = tx
            .catalog()?
            .0
            .technologies
            .get(&job.recipe)
            .ok_or(RejectReason::UnknownRecipe)?
            .clone();
        let (ratio, released, refund) = match job.stage {
            JobStage::Queued => (
                technology.cancel_refunds.queued,
                job.reservation.clone(),
                if technology.payment == PaymentPolicy::ChargeOnQueue {
                    job.charged.clone()
                } else {
                    Vec::new()
                },
            ),
            JobStage::Running => (technology.cancel_refunds.running, None, job.charged.clone()),
            JobStage::OutputBlocked => (
                technology.cancel_refunds.output_blocked,
                None,
                job.charged.clone(),
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
                producer: station,
                job: job_id.0,
                released: released.map_or_else(Vec::new, |reservation| {
                    reservation.lines.iter().map(event_cost_line).collect()
                }),
                refunded,
            },
        )?;
        Ok(())
    })
}

/// Advance research stations in ascending unit-id order, one transaction per subject and tick.
pub(crate) fn run_research(world: &mut World) {
    let stations = research_stations(world);
    for (station, slot) in stations {
        if world
            .get_resource::<Disabled>()
            .is_some_and(|disabled| disabled.0.contains(&station))
        {
            continue;
        }
        let Some(job) = world
            .get_resource::<ProductionQueues>()
            .and_then(|queues| queues.0.get(&station))
            .and_then(|queue| queue.first())
            .cloned()
        else {
            continue;
        };
        let _ = transaction::transact(world, |tx| {
            tx.mark_m3();
            let mut queues = tx.queues();
            let queue = queues.0.get_mut(&station).ok_or(RejectReason::UnknownJob)?;
            let head = queue.first_mut().ok_or(RejectReason::UnknownJob)?;
            if head.id != job.id {
                return Err(RejectReason::UnknownJob);
            }
            if head.stage == JobStage::Queued {
                let charged = if let Some(reservation) = head.reservation.take() {
                    transaction::consume_reservation(tx, &reservation)?;
                    reservation.lines
                } else {
                    head.charged.clone()
                };
                head.charged = charged.clone();
                head.stage = JobStage::Running;
                tx.stage_queues(queues);
                tx.stage_event(
                    slot,
                    EconomyEventKind::ProductionStarted {
                        producer: station,
                        job: job.id.0,
                        charged: charged.iter().map(event_cost_line).collect(),
                    },
                )?;
                return Ok(());
            }
            let elapsed = head
                .elapsed_ticks
                .checked_add(1)
                .ok_or(RejectReason::ArithmeticOverflow)?
                .min(head.duration_ticks);
            if elapsed == head.duration_ticks {
                let technology = head.recipe.clone();
                queue.remove(0);
                tx.stage_queues(queues);
                tx.stage_research_complete(slot, technology.clone());
                tx.stage_event(
                    slot,
                    EconomyEventKind::ResearchCompleted {
                        station,
                        job: job.id.0,
                        slot,
                        technology,
                    },
                )?;
            } else {
                head.elapsed_ticks = elapsed;
                tx.stage_queues(queues);
            }
            Ok(())
        });
    }
}

fn station_owner_and_def(
    world: &World,
    station: u32,
) -> Result<(u8, Option<turts_mod_api::ResolvedResearchStation>), RejectReason> {
    let (owner, unit_type) = world
        .iter_entities()
        .find_map(|entity| {
            let id = entity.get::<UnitId>()?;
            (id.0 == station).then(|| {
                (
                    entity.get::<Owner>().copied(),
                    entity.get::<UnitType>().copied(),
                )
            })
        })
        .ok_or(RejectReason::UnknownUnit)?;
    let owner = owner.ok_or(RejectReason::UnknownUnit)?;
    let unit_type = unit_type.ok_or(RejectReason::MissingCapability)?;
    let type_entry = world
        .get_resource::<UnitTypes>()
        .and_then(|types| types.get(unit_type.0))
        .ok_or(RejectReason::MissingCapability)?;
    let id =
        ContentId::parse(&type_entry.content_id).map_err(|_| RejectReason::MissingCapability)?;
    let definition = world
        .get_resource::<InstalledCatalog>()
        .and_then(|catalog| catalog.0.units.get(&id))
        .ok_or(RejectReason::MissingCapability)?;
    Ok((owner.0.0, definition.research.clone()))
}

pub(crate) fn is_research_station(world: &World, station: u32) -> bool {
    station_owner_and_def(world, station).is_ok_and(|(_, definition)| definition.is_some())
}

pub(crate) fn research_station_ids(world: &World) -> BTreeSet<u32> {
    research_stations(world)
        .into_iter()
        .map(|(station, _)| station)
        .collect()
}

fn research_stations_for_slot(world: &World, slot: u8) -> Vec<u32> {
    research_stations(world)
        .into_iter()
        .filter_map(|(station, owner)| (owner == slot).then_some(station))
        .collect()
}

fn research_stations(world: &World) -> Vec<(u32, u8)> {
    let (Some(catalog), Some(types)) = (
        world.get_resource::<InstalledCatalog>(),
        world.get_resource::<UnitTypes>(),
    ) else {
        return Vec::new();
    };
    let mut stations = world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?.0;
            let owner = entity.get::<Owner>()?.0.0;
            let unit_type = entity.get::<UnitType>()?.0;
            let content_id = ContentId::parse(&types.get(unit_type)?.content_id).ok()?;
            catalog
                .0
                .units
                .get(&content_id)
                .is_some_and(|unit| unit.research.is_some())
                .then_some((id, owner))
        })
        .collect::<Vec<_>>();
    stations.sort_by_key(|(id, _)| *id);
    stations
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use turts_mod_api::{
        CancelRefunds, ContentCatalog, ContentId, CostSource, Footprint, MapDef, MapSize,
        PaymentPolicy, Placement, RefundRatio, ResolvedCost, ResolvedEffect, ResolvedInventory,
        ResolvedResearchStation, ResolvedTechnology, ResolvedUnit, ResourceDef, ResourceKind,
        SourceDef, WeaponStat, WorldPos,
    };

    use crate::{
        Command, CommandKind, CommittedTick, EconomyEventKind, RejectReason, SlotId, apply_command,
        bootstrap::{install_package, stage_package},
        economy::{ledger::EconomyLedger, production::ProductionQueues},
        headless_app,
        ids::IdAllocators,
    };

    use super::{ResearchState, modifier};

    fn cid(value: &str) -> ContentId {
        ContentId::parse(value).unwrap()
    }

    fn app_with_research(tick_hz: u16, shells_per_lab: u64) -> bevy_app::App {
        let lab = cid("research_fixture:lab");
        let second_lab = cid("research_fixture:second_lab");
        let shell = cid("research_fixture:shell");
        let first_technology = cid("research_fixture:first");
        let second_technology = cid("research_fixture:second");
        let turret = cid("research_fixture:turret");
        let station = |id: ContentId, name: &str| ResolvedUnit {
            id,
            name: name.into(),
            footprint: Footprint::Circle { radius: 0.8 },
            movement: None,
            inventory: Some(ResolvedInventory {
                capacity: 10,
                accepts: vec![shell.clone()],
            }),
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: Some(ResolvedResearchStation {
                technologies: vec![first_technology.clone(), second_technology.clone()],
                queue_capacity: 1,
            }),
            health: None,
            weapon: None,
            removal: None,
        };
        let catalog = ContentCatalog {
            resources: BTreeMap::from([(
                shell.clone(),
                ResourceDef {
                    id: "shell".into(),
                    name: "Shell".into(),
                    unit_name: "shell".into(),
                    kind: ResourceKind::Item,
                },
            )]),
            units: BTreeMap::from([
                (lab.clone(), station(lab.clone(), "Lab")),
                (
                    second_lab.clone(),
                    station(second_lab.clone(), "Second Lab"),
                ),
                (
                    turret.clone(),
                    ResolvedUnit {
                        id: turret.clone(),
                        name: "Turret".into(),
                        footprint: Footprint::Circle { radius: 0.8 },
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
                    },
                ),
            ]),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::from([
                (
                    first_technology.clone(),
                    ResolvedTechnology {
                        id: first_technology.clone(),
                        name: "First".into(),
                        prerequisites: vec![],
                        duration_ms: 10_000,
                        costs: vec![ResolvedCost {
                            resource: shell.clone(),
                            amount: 5,
                            source: CostSource::ProducerInventory,
                        }],
                        payment: PaymentPolicy::ReserveThenCharge,
                        cancel_refunds: CancelRefunds {
                            queued: RefundRatio {
                                numerator: 1,
                                denominator: 1,
                            },
                            running: RefundRatio {
                                numerator: 1,
                                denominator: 2,
                            },
                            output_blocked: RefundRatio {
                                numerator: 1,
                                denominator: 2,
                            },
                        },
                        effects: vec![ResolvedEffect::WeaponModifier {
                            unit: turret.clone(),
                            stat: WeaponStat::Damage,
                            add: 10,
                        }],
                    },
                ),
                (
                    second_technology.clone(),
                    ResolvedTechnology {
                        id: second_technology.clone(),
                        name: "Second".into(),
                        prerequisites: vec![first_technology.clone()],
                        duration_ms: 10_000,
                        costs: vec![ResolvedCost {
                            resource: shell.clone(),
                            amount: 5,
                            source: CostSource::ProducerInventory,
                        }],
                        payment: PaymentPolicy::ReserveThenCharge,
                        cancel_refunds: CancelRefunds {
                            queued: RefundRatio {
                                numerator: 1,
                                denominator: 1,
                            },
                            running: RefundRatio {
                                numerator: 1,
                                denominator: 2,
                            },
                            output_blocked: RefundRatio {
                                numerator: 1,
                                denominator: 2,
                            },
                        },
                        effects: vec![ResolvedEffect::WeaponModifier {
                            unit: turret.clone(),
                            stat: WeaponStat::Damage,
                            add: u64::MAX,
                        }],
                    },
                ),
            ]),
            source_files: BTreeMap::new(),
        };
        let map = MapDef {
            id: "test_map".into(),
            size: MapSize { x: 32.0, y: 32.0 },
            placements: vec![
                placement("research_fixture:lab", 6.0),
                placement("research_fixture:second_lab", 16.0),
            ],
            nodes: vec![],
            slot_economy: vec![],
            belts: vec![],
        };
        let mut app = headless_app(tick_hz);
        let staged = stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "maps/test_map.ron".into(),
                value: map,
            },
            tick_hz,
        )
        .unwrap();
        install_package(app.world_mut(), staged);
        for id in [1, 2] {
            app.world_mut()
                .resource_mut::<EconomyLedger>()
                .inventories
                .get_mut(&id)
                .unwrap()
                .items
                .insert(
                    shell.clone(),
                    crate::economy::ledger::Account {
                        on_hand: shells_per_lab,
                        reserved: 0,
                    },
                );
        }
        assert_eq!(app.world().resource::<ProductionQueues>().0.len(), 2);
        app
    }

    fn placement(unit: &str, x: f32) -> Placement {
        Placement {
            unit: unit.into(),
            owner_slot: 0,
            pos: WorldPos { x, y: 8.0 },
            contents: vec![],
            facing: None,
        }
    }

    fn produce(
        app: &mut bevy_app::App,
        station: u32,
        technology: &str,
        seq: u32,
    ) -> Result<u64, RejectReason> {
        produce_for_slot(app, 0, station, technology, seq)
    }

    fn produce_for_slot(
        app: &mut bevy_app::App,
        slot: u8,
        station: u32,
        technology: &str,
        seq: u32,
    ) -> Result<u64, RejectReason> {
        apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(slot),
                seq,
                kind: CommandKind::Produce {
                    producer_id: station,
                    recipe_id: cid(technology),
                },
            },
        )
        .outcome
        .map(|applied| applied.job_id.unwrap().0)
    }

    fn cancel(app: &mut bevy_app::App, station: u32, job: u64, seq: u32) {
        let result = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq,
                kind: CommandKind::CancelProduction {
                    producer_id: station,
                    job_id: crate::ids::JobId(job),
                },
            },
        );
        assert!(
            result.outcome.is_ok(),
            "cancellation result: {:?}",
            result.outcome
        );
    }

    #[test]
    fn research_completes_after_ceil_duration() {
        for (hz, duration) in [(10, 100), (20, 200), (30, 300)] {
            let mut app = app_with_research(hz, 5);
            let job = produce(&mut app, 1, "research_fixture:first", 1).unwrap();
            app.update(); // queued -> running, no progress on the start tick
            assert_eq!(
                app.world()
                    .resource::<crate::bootstrap::ProductionDurations>()
                    .0[&cid("research_fixture:first")],
                duration,
                "boot duration at {hz} Hz"
            );
            for _ in 0..duration - 1 {
                app.update();
            }
            assert!(
                !app.world()
                    .resource::<ResearchState>()
                    .completed
                    .contains(&(0, cid("research_fixture:first")))
            );
            assert!(
                app.world().resource::<ProductionQueues>().0[&1]
                    .iter()
                    .any(|j| j.id.0 == job)
            );
            app.world_mut()
                .resource_mut::<crate::economy::transaction::AuditTrail>()
                .enable();
            app.world_mut()
                .resource_mut::<crate::economy::transaction::AuditTrail>()
                .operations
                .clear();
            app.update();
            assert!(
                app.world()
                    .resource::<ResearchState>()
                    .completed
                    .contains(&(0, cid("research_fixture:first")))
            );
            assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
            let completed_event = |kind: &EconomyEventKind| {
                matches!(
                    kind,
                    EconomyEventKind::ResearchCompleted {
                        station: 1,
                        job: completed_job,
                        slot: 0,
                        technology,
                    } if *completed_job == job && technology == &cid("research_fixture:first")
                )
            };
            assert!(
                app.world()
                    .resource::<CommittedTick>()
                    .events
                    .iter()
                    .any(|event| completed_event(&event.kind))
            );
            let completion_operations = &app
                .world()
                .resource::<crate::economy::transaction::AuditTrail>()
                .operations;
            assert_eq!(
                completion_operations.len(),
                1,
                "completion and final advance share one captured transaction"
            );
            assert!(
                completion_operations[0]
                    .events
                    .iter()
                    .any(|event| completed_event(&event.kind))
            );
            let before_queue = completion_operations[0]
                .before
                .raw_queues
                .iter()
                .find(|queue| queue.producer == 1)
                .unwrap();
            assert_eq!(before_queue.jobs[0].elapsed_ticks, duration - 1);
            let after_queue = completion_operations[0]
                .after
                .raw_queues
                .iter()
                .find(|queue| queue.producer == 1)
                .unwrap();
            assert!(after_queue.jobs.is_empty());
        }
    }

    #[test]
    fn produce_without_shells_rejects_insufficient_items_and_leaves_claims() {
        let mut app = app_with_research(20, 4);
        let before_ledger = app.world().resource::<EconomyLedger>().clone();
        let before_ids = *app.world().resource::<IdAllocators>();
        let before_events = app
            .world()
            .resource::<crate::economy::events::PendingEconomyEvents>()
            .clone();
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        assert_eq!(
            produce(&mut app, 1, "research_fixture:first", 1),
            Err(RejectReason::InsufficientItems)
        );
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
        assert_eq!(app.world().resource::<EconomyLedger>(), &before_ledger);
        assert_eq!(app.world().resource::<IdAllocators>(), &before_ids);
        assert_eq!(
            app.world()
                .resource::<crate::economy::events::PendingEconomyEvents>(),
            &before_events
        );
        let operations = &app
            .world()
            .resource::<crate::economy::transaction::AuditTrail>()
            .operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].before, operations[0].after);
    }

    #[test]
    fn already_researched_rejects() {
        let mut app = app_with_research(20, 5);
        app.world_mut()
            .resource_mut::<ResearchState>()
            .completed
            .insert((0, cid("research_fixture:first")));
        app.world_mut()
            .resource_mut::<ProductionQueues>()
            .0
            .get_mut(&1)
            .unwrap()
            .push(crate::economy::production::ProductionJob {
                id: crate::ids::JobId(99),
                recipe: cid("research_fixture:first"),
                stage: crate::economy::production::JobStage::Queued,
                elapsed_ticks: 0,
                duration_ticks: 200,
                charged: vec![],
                reservation: None,
            });
        assert_eq!(
            produce(&mut app, 2, "research_fixture:first", 1),
            Err(RejectReason::AlreadyResearched)
        );
        // Research state is per-slot: the same completed technology does not block another slot.
        let second_lab_entity = app
            .world()
            .iter_entities()
            .find(|e| e.get::<crate::UnitId>().is_some_and(|id| id.0 == 2))
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(second_lab_entity)
            .insert(crate::Owner(SlotId(1)));
        app.world_mut()
            .resource_mut::<crate::economy::ledger::EconomyLedger>()
            .inventories
            .get_mut(&2)
            .unwrap()
            .items
            .insert(
                cid("research_fixture:shell"),
                crate::economy::ledger::Account {
                    on_hand: 5,
                    reserved: 0,
                },
            );
        assert!(produce_for_slot(&mut app, 1, 2, "research_fixture:first", 3).is_ok());
    }

    #[test]
    fn second_request_same_slot_rejects_in_progress_across_stations() {
        let mut app = app_with_research(20, 5);
        assert!(produce(&mut app, 1, "research_fixture:first", 1).is_ok());
        assert_eq!(
            produce(&mut app, 2, "research_fixture:first", 2),
            Err(RejectReason::ResearchInProgress)
        );
        assert!(app.world().resource::<ProductionQueues>().0[&2].is_empty());
    }

    #[test]
    fn research_rejection_checks_follow_contract_order() {
        let mut app = app_with_research(20, 5);
        let lab = cid("research_fixture:lab");
        app.world_mut()
            .resource_mut::<crate::bootstrap::InstalledCatalog>()
            .0
            .units
            .get_mut(&lab)
            .unwrap()
            .research
            .as_mut()
            .unwrap()
            .technologies = vec![cid("research_fixture:first")];
        // The second technology is both unlisted and missing its prerequisite.
        assert_eq!(
            produce(&mut app, 1, "research_fixture:second", 1),
            Err(RejectReason::RecipeNotEnabled)
        );
    }

    #[test]
    fn locked_prerequisite_rejects() {
        let mut app = app_with_research(20, 5);
        assert_eq!(
            produce(&mut app, 1, "research_fixture:second", 1),
            Err(RejectReason::PrerequisitesMissing)
        );
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
    }

    #[test]
    fn cancel_running_refunds_half() {
        let mut app = app_with_research(20, 5);
        let job = produce(&mut app, 1, "research_fixture:first", 1).unwrap();
        app.update();
        app.update();
        cancel(&mut app, 1, job, 2);
        let shell = cid("research_fixture:shell");
        let account = &app.world().resource::<EconomyLedger>().inventories[&1].items[&shell];
        assert_eq!(
            account.on_hand, 2,
            "five charged; half refund floors to two"
        );
        assert_eq!(account.reserved, 0);
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
    }

    #[test]
    fn cancel_queued_refunds_all_and_releases_claim() {
        let mut app = app_with_research(20, 5);
        let job = produce(&mut app, 1, "research_fixture:first", 1).unwrap();
        let shell = cid("research_fixture:shell");
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&1].items[&shell].reserved,
            5
        );
        cancel(&mut app, 1, job, 2);
        let account = &app.world().resource::<EconomyLedger>().inventories[&1].items[&shell];
        assert_eq!((account.on_hand, account.reserved), (5, 0));
        assert_eq!(
            app.world()
                .resource::<EconomyLedger>()
                .reservation_claims
                .get(&crate::ids::JobId(job)),
            Some(&crate::economy::ledger::ReservationClaim::Closed)
        );
    }

    #[test]
    fn cancel_charge_on_queue_refunds_declared_queued_ratio() {
        let mut app = app_with_research(20, 5);
        app.world_mut()
            .resource_mut::<crate::bootstrap::InstalledCatalog>()
            .0
            .technologies
            .get_mut(&cid("research_fixture:first"))
            .unwrap()
            .payment = turts_mod_api::PaymentPolicy::ChargeOnQueue;
        let job = produce(&mut app, 1, "research_fixture:first", 1).unwrap();
        let shell = cid("research_fixture:shell");
        let account = &app.world().resource::<EconomyLedger>().inventories[&1].items[&shell];
        assert_eq!((account.on_hand, account.reserved), (0, 0));
        cancel(&mut app, 1, job, 2);
        let account = &app.world().resource::<EconomyLedger>().inventories[&1].items[&shell];
        assert_eq!((account.on_hand, account.reserved), (5, 0));
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
    }

    #[test]
    fn modifier_is_computed_from_completed_set() {
        let app = app_with_research(20, 5);
        let catalog = &app
            .world()
            .resource::<crate::bootstrap::InstalledCatalog>()
            .0;
        let mut research = ResearchState::default();
        let turret = cid("research_fixture:turret");
        assert_eq!(
            modifier(&research, catalog, 0, &turret, WeaponStat::Damage),
            Ok(0)
        );
        research
            .completed
            .insert((0, cid("research_fixture:first")));
        assert_eq!(
            modifier(&research, catalog, 0, &turret, WeaponStat::Damage),
            Ok(10)
        );
        assert_eq!(
            modifier(&research, catalog, 1, &turret, WeaponStat::Damage),
            Ok(0)
        );
    }

    #[test]
    fn modifier_reports_checked_overflow() {
        let app = app_with_research(20, 5);
        let catalog = &app
            .world()
            .resource::<crate::bootstrap::InstalledCatalog>()
            .0;
        let mut research = ResearchState::default();
        research
            .completed
            .insert((0, cid("research_fixture:first")));
        research
            .completed
            .insert((0, cid("research_fixture:second")));
        assert_eq!(
            modifier(
                &research,
                catalog,
                0,
                &cid("research_fixture:turret"),
                WeaponStat::Damage
            ),
            Err(RejectReason::ArithmeticOverflow)
        );
    }

    #[test]
    fn research_only_unit_accepts_produce_command() {
        // The server admits this capability in session.rs; this checks the shared sim command path.
        let mut app = app_with_research(20, 5);
        assert!(produce(&mut app, 1, "research_fixture:first", 1).is_ok());
    }
}
