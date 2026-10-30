use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::world::{EntityRef, World};
use turts_mod_api::{ContentId, ResolvedRecipe};

use crate::{
    Owner, RejectReason, UnitId, UnitType,
    bootstrap::{InstalledCatalog, ProductionDurations},
    economy::{
        events::{CostLine, EconomyEventKind, EventSource},
        ledger::{Account, Endpoint, ReservationLine},
        production::{JobStage, ProductionJob, ProductionQueues},
        research::Disabled,
        transaction::{self, Transaction},
    },
    ids::JobId,
    registry::UnitTypes,
};

/// Return machine subjects in explicit, ascending unit-id order.
pub(crate) fn machine_ids(world: &World) -> BTreeSet<u32> {
    let (Some(catalog), Some(types)) = (
        world.get_resource::<InstalledCatalog>(),
        world.get_resource::<UnitTypes>(),
    ) else {
        return BTreeSet::new();
    };
    world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?.0;
            let unit_type = entity.get::<UnitType>()?.0;
            let content_id = ContentId::parse(&types.get(unit_type)?.content_id).ok()?;
            catalog
                .0
                .units
                .get(&content_id)
                .is_some_and(|unit| unit.machine.is_some())
                .then_some(id)
        })
        .collect()
}

/// Advance each enabled machine by one fixed production tick.
pub(crate) fn run_machines(world: &mut World) {
    for machine in machine_ids(world) {
        if world
            .get_resource::<Disabled>()
            .is_some_and(|disabled| disabled.0.contains(&machine))
        {
            continue;
        }
        let Some((owner_slot, recipe_id, duration_ticks)) = machine_recipe(world, machine) else {
            continue;
        };
        let status = machine_status(world, machine);
        let _ = transaction::transact(world, |tx| {
            tx.mark_m3();
            let mut queues = tx.queues();
            let queue = queues.0.get_mut(&machine).ok_or(RejectReason::UnknownJob)?;
            if queue.is_empty() {
                if status == Some(MachineStatus::InsufficientInput) {
                    return Err(RejectReason::InsufficientItems);
                }
                start_job(tx, machine, owner_slot, &recipe_id, duration_ticks, queue)?;
                tx.stage_queues(queues);
                return Ok(());
            }

            let job = queue.first().cloned().ok_or(RejectReason::UnknownJob)?;
            if job.stage == JobStage::Queued {
                return Err(RejectReason::IntegrityViolation);
            }
            if job.stage == JobStage::OutputBlocked {
                if credit_outputs(tx, machine, &recipe_id)? {
                    queue.remove(0);
                    tx.stage_queues(queues);
                    let recipe = tx
                        .catalog()?
                        .0
                        .recipes
                        .get(&recipe_id)
                        .ok_or(RejectReason::UnknownRecipe)?
                        .clone();
                    tx.stage_event(owner_slot, produced_event(machine, &job, &recipe))?;
                }
                return Ok(());
            }

            let elapsed = job
                .elapsed_ticks
                .checked_add(1)
                .ok_or(RejectReason::ArithmeticOverflow)?
                .min(job.duration_ticks);
            let mut updated = job.clone();
            updated.elapsed_ticks = elapsed;
            if elapsed == job.duration_ticks {
                if credit_outputs(tx, machine, &recipe_id)? {
                    queue.remove(0);
                    tx.stage_queues(queues);
                    let recipe = tx
                        .catalog()?
                        .0
                        .recipes
                        .get(&recipe_id)
                        .ok_or(RejectReason::UnknownRecipe)?
                        .clone();
                    tx.stage_event(owner_slot, produced_event(machine, &job, &recipe))?;
                    return Ok(());
                }
                updated.stage = JobStage::OutputBlocked;
            }
            queue[0] = updated;
            tx.stage_queues(queues);
            Ok(())
        });
    }
}

/// Cancel the active item recipe with its recipe-specific stage refund.
pub(crate) fn cancel_production(
    world: &mut World,
    owner_slot: u8,
    machine: u32,
    job_id: JobId,
) -> Result<(), RejectReason> {
    transaction::transact(world, |tx| {
        tx.mark_m3();
        let mut queues = tx.queues();
        let queue = queues.0.get_mut(&machine).ok_or(RejectReason::UnknownJob)?;
        let index = queue
            .iter()
            .position(|job| job.id == job_id)
            .ok_or(RejectReason::UnknownJob)?;
        let job = queue[index].clone();
        let recipe = tx
            .catalog()?
            .0
            .recipes
            .get(&job.recipe)
            .ok_or(RejectReason::UnknownRecipe)?;
        let ratio = match job.stage {
            JobStage::Running => recipe.cancel_refunds.running,
            JobStage::OutputBlocked => recipe.cancel_refunds.output_blocked,
            JobStage::Queued => return Err(RejectReason::IntegrityViolation),
        };
        transaction::refund_charged(
            tx,
            &job.charged,
            u64::from(ratio.numerator),
            u64::from(ratio.denominator),
        )?;
        queue.remove(index);
        tx.stage_queues(queues);
        let refunded = job
            .charged
            .iter()
            .map(|line| {
                Ok(CostLine {
                    source: event_source(line.endpoint),
                    resource: line.resource.clone(),
                    amount: u64::try_from(
                        u128::from(line.amount) * u128::from(ratio.numerator)
                            / u128::from(ratio.denominator),
                    )
                    .map_err(|_| RejectReason::ArithmeticOverflow)?,
                })
            })
            .collect::<Result<Vec<_>, RejectReason>>()?;
        tx.stage_event(
            owner_slot,
            EconomyEventKind::ProductionCancelled {
                producer: machine,
                job: job.id.0,
                released: Vec::new(),
                refunded,
            },
        )?;
        Ok(())
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MachineStatus {
    InsufficientInput,
    Ready,
    Running,
    OutputBlocked,
    Disabled,
}

/// Derive the machine condition from its queue and current input inventory.
fn machine_status(world: &World, machine: u32) -> Option<MachineStatus> {
    if world
        .get_resource::<Disabled>()
        .is_some_and(|disabled| disabled.0.contains(&machine))
    {
        return Some(MachineStatus::Disabled);
    }
    let (_, recipe_id, _) = machine_recipe(world, machine)?;
    let head = world
        .get_resource::<ProductionQueues>()
        .and_then(|queues| queues.0.get(&machine))
        .and_then(|queue| queue.first());
    match head.map(|job| job.stage) {
        Some(JobStage::Running) => Some(MachineStatus::Running),
        Some(JobStage::OutputBlocked) => Some(MachineStatus::OutputBlocked),
        Some(JobStage::Queued) => None,
        None if inputs_available(world, machine, &recipe_id) => Some(MachineStatus::Ready),
        None => Some(MachineStatus::InsufficientInput),
    }
}

fn inputs_available(world: &World, machine: u32, recipe_id: &ContentId) -> bool {
    let Some(recipe) = world
        .get_resource::<InstalledCatalog>()
        .and_then(|catalog| catalog.0.recipes.get(recipe_id))
    else {
        return false;
    };
    let Some(inventory) = world
        .get_resource::<crate::economy::ledger::EconomyLedger>()
        .and_then(|ledger| ledger.inventories.get(&machine))
    else {
        return false;
    };
    let mut totals = BTreeMap::<ContentId, u128>::new();
    for input in &recipe.inputs {
        let amount = totals.entry(input.item.clone()).or_default();
        let Some(total) = amount.checked_add(u128::from(input.amount)) else {
            return false;
        };
        *amount = total;
    }
    totals.into_iter().all(|(item, amount)| {
        inventory.accepted_items.contains(&item)
            && inventory.items.get(&item).is_some_and(|account| {
                account
                    .on_hand
                    .checked_sub(account.reserved)
                    .is_some_and(|available| u128::from(available) >= amount)
            })
    })
}

fn machine_recipe(world: &World, machine: u32) -> Option<(u8, ContentId, u64)> {
    let entity = find_unit(world, machine)?;
    let owner = entity.get::<Owner>()?.0.0;
    let unit_type = entity.get::<UnitType>()?.0;
    let type_entry = world.get_resource::<UnitTypes>()?.get(unit_type)?;
    let unit_id = ContentId::parse(&type_entry.content_id).ok()?;
    let recipe_id = world
        .get_resource::<InstalledCatalog>()?
        .0
        .units
        .get(&unit_id)?
        .machine
        .as_ref()?
        .recipe
        .clone();
    let duration = world
        .get_resource::<ProductionDurations>()?
        .0
        .get(&recipe_id)
        .copied()?;
    Some((owner, recipe_id, duration))
}

fn find_unit(world: &World, unit: u32) -> Option<EntityRef<'_>> {
    world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit))
}

fn start_job(
    tx: &mut Transaction<'_>,
    machine: u32,
    owner_slot: u8,
    recipe_id: &ContentId,
    duration_ticks: u64,
    queue: &mut Vec<ProductionJob>,
) -> Result<(), RejectReason> {
    let recipe = tx
        .catalog()?
        .0
        .recipes
        .get(recipe_id)
        .ok_or(RejectReason::UnknownRecipe)?;
    let mut totals = BTreeMap::<ContentId, u128>::new();
    for input in &recipe.inputs {
        let amount = totals.entry(input.item.clone()).or_default();
        *amount = amount
            .checked_add(u128::from(input.amount))
            .ok_or(RejectReason::ArithmeticOverflow)?;
    }
    let charged = totals
        .into_iter()
        .map(|(resource, amount)| {
            Ok(ReservationLine {
                endpoint: Endpoint::Inventory(machine),
                resource,
                amount: u64::try_from(amount).map_err(|_| RejectReason::ArithmeticOverflow)?,
            })
        })
        .collect::<Result<Vec<_>, RejectReason>>()?;
    transaction::charge_costs(tx, &charged)?;
    let id = tx.allocate_job_id()?;
    queue.push(ProductionJob {
        id,
        recipe: recipe_id.clone(),
        stage: JobStage::Running,
        elapsed_ticks: 0,
        duration_ticks,
        charged: charged.clone(),
        reservation: None,
    });
    tx.stage_event(
        owner_slot,
        EconomyEventKind::ProductionStarted {
            producer: machine,
            job: id.0,
            charged: charged
                .iter()
                .map(|line| CostLine {
                    source: EventSource::Inventory(machine),
                    resource: line.resource.clone(),
                    amount: line.amount,
                })
                .collect(),
        },
    )?;
    Ok(())
}

fn credit_outputs(
    tx: &mut Transaction<'_>,
    machine: u32,
    recipe_id: &ContentId,
) -> Result<bool, RejectReason> {
    let recipe = tx
        .catalog()?
        .0
        .recipes
        .get(recipe_id)
        .ok_or(RejectReason::UnknownRecipe)?;
    let endpoint = Endpoint::Output(machine);
    let capacity = tx.inventory(endpoint)?.capacity;
    let mut items_by_content = BTreeMap::<ContentId, u128>::new();
    let mut credit_total = 0_u128;
    for output in &recipe.outputs {
        tx.validate_endpoint(endpoint, &output.item)?;
        let amount = items_by_content.entry(output.item.clone()).or_default();
        *amount = amount
            .checked_add(u128::from(output.amount))
            .ok_or(RejectReason::ArithmeticOverflow)?;
        credit_total = credit_total
            .checked_add(u128::from(output.amount))
            .ok_or(RejectReason::ArithmeticOverflow)?;
    }
    let used = tx.inventory_used(endpoint)?;
    if used
        .checked_add(credit_total)
        .ok_or(RejectReason::ArithmeticOverflow)?
        > u128::from(capacity)
    {
        return Ok(false);
    }

    let mut updates = Vec::with_capacity(items_by_content.len());
    for (item, amount) in items_by_content {
        let amount = u64::try_from(amount).map_err(|_| RejectReason::ArithmeticOverflow)?;
        let account = tx.account(endpoint, &item);
        updates.push((
            item,
            Account {
                on_hand: account
                    .on_hand
                    .checked_add(amount)
                    .ok_or(RejectReason::ArithmeticOverflow)?,
                ..account
            },
        ));
    }
    for (item, account) in updates {
        tx.stage_account(endpoint, item, account);
    }
    Ok(true)
}

fn produced_event(machine: u32, job: &ProductionJob, recipe: &ResolvedRecipe) -> EconomyEventKind {
    EconomyEventKind::ItemsProduced {
        machine,
        job: job.id.0,
        outputs: recipe
            .outputs
            .iter()
            .map(|output| CostLine {
                source: EventSource::Output(machine),
                resource: output.item.clone(),
                amount: output.amount,
            })
            .collect(),
    }
}

fn event_source(endpoint: Endpoint) -> EventSource {
    match endpoint {
        Endpoint::Slot(slot) => EventSource::Slot(slot),
        Endpoint::Inventory(unit) => EventSource::Inventory(unit),
        Endpoint::Output(unit) => EventSource::Output(unit),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{MachineStatus, machine_status};

    use turts_mod_api::{
        ContentCatalog, ContentId, Footprint, MapDef, MapSize, Placement, RefundRatio,
        ResolvedInventory, ResolvedItemStack, ResolvedMachine, ResolvedRecipe, ResolvedUnit,
        ResourceDef, ResourceKind, RunRefunds, SourceDef, WorldPos,
    };

    use crate::{
        Command, CommandKind, CommandQueue, CommandResults, CommittedTick, EconomyEventKind,
        SlotId, Tick,
        bootstrap::{install_package, stage_package},
        economy::{
            ledger::{Account, EconomyLedger},
            production::{JobStage, ProductionQueues},
            research::Disabled,
            transaction::AuditTrail,
        },
        headless_app,
    };

    fn cid(text: &str) -> ContentId {
        ContentId::parse(text).unwrap()
    }

    fn app_with_machine(input_ore: u64, output_parts: u64) -> bevy_app::App {
        app_with_machine_config(
            input_ore,
            output_parts,
            2,
            RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            false,
        )
    }

    fn app_with_machine_input_lines(
        input_ore: u64,
        output_parts: u64,
        duplicate_lines: bool,
    ) -> bevy_app::App {
        app_with_machine_config(
            input_ore,
            output_parts,
            2,
            RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            duplicate_lines,
        )
    }

    fn app_with_machine_config(
        input_ore: u64,
        output_parts: u64,
        input_cost: u64,
        running_refund: RefundRatio,
        duplicate_lines: bool,
    ) -> bevy_app::App {
        let package = "machine_fixture";
        let machine_id = cid("machine_fixture:press");
        let recipe_id = cid("machine_fixture:make_part");
        let ore = cid("machine_fixture:ore");
        let part = cid("machine_fixture:part");
        let definition = ResolvedUnit {
            id: machine_id.clone(),
            name: "Press".into(),
            footprint: Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: Some(ResolvedInventory {
                capacity: 6,
                accepts: vec![ore.clone()],
            }),
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: Some(ResolvedInventory {
                capacity: 4,
                accepts: vec![part.clone()],
            }),
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: Some(ResolvedMachine {
                recipe: recipe_id.clone(),
            }),
            research: None,
            health: None,
            weapon: None,
            removal: None,
        };
        let recipe = ResolvedRecipe {
            id: recipe_id.clone(),
            name: "Make Part".into(),
            inputs: if duplicate_lines {
                let first = input_cost / 2;
                vec![
                    ResolvedItemStack {
                        item: ore.clone(),
                        amount: first,
                    },
                    ResolvedItemStack {
                        item: ore.clone(),
                        amount: input_cost - first,
                    },
                ]
            } else {
                vec![ResolvedItemStack {
                    item: ore.clone(),
                    amount: input_cost,
                }]
            },
            outputs: vec![ResolvedItemStack {
                item: part.clone(),
                amount: 1,
            }],
            duration_ms: 2_000,
            cancel_refunds: RunRefunds {
                running: running_refund,
                output_blocked: running_refund,
            },
        };
        let resources = [(&ore, "Ore"), (&part, "Part")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id.clone(),
                    ResourceDef {
                        id: id.local.clone(),
                        name: name.into(),
                        unit_name: "item".into(),
                        kind: ResourceKind::Item,
                    },
                )
            })
            .collect();
        let catalog = ContentCatalog {
            resources,
            units: BTreeMap::from([(machine_id, definition)]),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::from([(recipe_id, recipe)]),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        let map = MapDef {
            id: "test_map".into(),
            size: MapSize { x: 16.0, y: 16.0 },
            placements: vec![Placement {
                unit: format!("{package}:press"),
                owner_slot: 0,
                pos: WorldPos { x: 8.0, y: 8.0 },
                contents: Vec::new(),
                facing: None,
            }],
            nodes: Vec::new(),
            slot_economy: Vec::new(),
            belts: Vec::new(),
        };
        let mut app = headless_app(20);
        let staged = stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "maps/test_map.ron".into(),
                value: map,
            },
            20,
        )
        .unwrap();
        install_package(app.world_mut(), staged);
        {
            let mut ledger = app.world_mut().resource_mut::<EconomyLedger>();
            ledger.inventories.get_mut(&1).unwrap().items.insert(
                ore,
                Account {
                    on_hand: input_ore,
                    reserved: 0,
                },
            );
            ledger.output_inventories.get_mut(&1).unwrap().items.insert(
                part,
                Account {
                    on_hand: output_parts,
                    reserved: 0,
                },
            );
        }
        app
    }

    fn cancel(app: &mut bevy_app::App, job_id: u64) {
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::CancelProduction {
                    producer_id: 1,
                    job_id: crate::ids::JobId(job_id),
                },
            });
        app.update();
    }

    fn item_amount(ledger: &EconomyLedger, output: bool, unit: u32) -> u64 {
        let inventory = if output {
            ledger.output_inventories.get(&unit)
        } else {
            ledger.inventories.get(&unit)
        }
        .unwrap();
        inventory
            .items
            .values()
            .map(|account| account.on_hand)
            .sum()
    }

    #[test]
    fn cycle_is_duration_plus_one() {
        let mut app = app_with_machine(2, 0);

        app.update();
        let queue = app.world().resource::<ProductionQueues>();
        assert_eq!(queue.0[&1].len(), 1, "machine starts automatically");
        assert_eq!(queue.0[&1][0].elapsed_ticks, 0);

        for _ in 0..39 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<ProductionQueues>().0[&1][0].elapsed_ticks,
            39
        );

        app.update();
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
        assert!(
            app.world()
                .resource::<CommittedTick>()
                .events
                .iter()
                .any(|event| {
                    matches!(
                        event.kind,
                        EconomyEventKind::ItemsProduced { machine: 1, .. }
                    )
                })
        );
    }

    #[test]
    fn insufficient_input_status_no_write() {
        let mut app = app_with_machine(1, 0);
        let allocators = *app.world().resource::<crate::ids::IdAllocators>();
        app.world_mut().resource_mut::<AuditTrail>().enable();
        app.update();
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
        assert_eq!(
            app.world().resource::<crate::ids::IdAllocators>(),
            &allocators
        );
        assert_eq!(
            item_amount(app.world().resource::<EconomyLedger>(), false, 1),
            1
        );
        assert_eq!(
            machine_status(app.world(), 1),
            Some(MachineStatus::InsufficientInput)
        );
        let operations = &app.world().resource::<AuditTrail>().operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].before, operations[0].after);
    }

    #[test]
    fn repeated_input_lines_are_canonicalized_for_charge_and_refund() {
        let mut app = app_with_machine_input_lines(2, 0, true);
        app.update();
        let job = &app.world().resource::<ProductionQueues>().0[&1][0];
        assert_eq!(job.charged.len(), 1);
        assert_eq!(job.charged[0].amount, 2);
        cancel(&mut app, 1);
        assert!(
            app.world()
                .resource::<CommandResults>()
                .0
                .last()
                .unwrap()
                .outcome
                .is_ok()
        );
        let restarted = &app.world().resource::<ProductionQueues>().0[&1][0];
        assert_eq!(restarted.id.0, 2);
        assert_eq!(restarted.charged.len(), 1);
        assert_eq!(restarted.charged[0].amount, 2);
    }

    #[test]
    fn output_blocked_holds_progress_no_recharge() {
        let mut app = app_with_machine(2, 4);
        app.world_mut().resource_mut::<AuditTrail>().enable();
        for _ in 0..41 {
            app.update();
        }
        assert_eq!(app.world().resource::<AuditTrail>().operations.len(), 41);
        let job = &app.world().resource::<ProductionQueues>().0[&1][0];
        assert_eq!(job.stage, JobStage::OutputBlocked);
        assert_eq!(job.elapsed_ticks, 40);
        assert_eq!(
            item_amount(app.world().resource::<EconomyLedger>(), false, 1),
            0
        );
        for expected_operations in 42..=44 {
            app.update();
            assert_eq!(
                app.world().resource::<AuditTrail>().operations.len(),
                expected_operations
            );
            let job = &app.world().resource::<ProductionQueues>().0[&1][0];
            assert_eq!(job.stage, JobStage::OutputBlocked);
            assert_eq!(job.elapsed_ticks, 40);
            assert_eq!(
                item_amount(app.world().resource::<EconomyLedger>(), false, 1),
                0
            );
        }
    }

    #[test]
    fn one_shell_when_room_returns() {
        let mut app = app_with_machine(2, 4);
        for _ in 0..41 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .output_inventories
            .get_mut(&1)
            .unwrap()
            .items
            .values_mut()
            .next()
            .unwrap()
            .on_hand -= 1;
        app.update();
        assert_eq!(
            item_amount(app.world().resource::<EconomyLedger>(), true, 1),
            4
        );
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
        let produced = app
            .world()
            .resource::<CommittedTick>()
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    EconomyEventKind::ItemsProduced { machine: 1, .. }
                )
            })
            .count();
        assert_eq!(produced, 1);
    }

    #[test]
    fn cancel_running_refunds_floor_charged_times_ratio() {
        let mut app = app_with_machine_config(
            5,
            0,
            5,
            RefundRatio {
                numerator: 1,
                denominator: 2,
            },
            false,
        );
        app.update();
        app.world_mut().resource_mut::<AuditTrail>().enable();
        cancel(&mut app, 1);
        let result = app.world().resource::<CommandResults>().0.last().unwrap();
        assert!(result.outcome.is_ok());
        assert_eq!(
            item_amount(app.world().resource::<EconomyLedger>(), false, 1),
            2
        );
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
        let event = app
            .world()
            .resource::<CommittedTick>()
            .events
            .iter()
            .find(|event| {
                matches!(
                    event.kind,
                    EconomyEventKind::ProductionCancelled { job: 1, .. }
                )
            })
            .unwrap();
        assert!(matches!(
            &event.kind,
            EconomyEventKind::ProductionCancelled { refunded, .. }
                if refunded.iter().map(|line| line.amount).sum::<u64>() == 2
        ));
    }

    #[test]
    fn cancel_output_blocked_refunds_and_credits_no_output() {
        let mut app = app_with_machine_config(
            5,
            4,
            5,
            RefundRatio {
                numerator: 1,
                denominator: 2,
            },
            false,
        );
        for _ in 0..41 {
            app.update();
        }
        cancel(&mut app, 1);
        assert!(
            app.world()
                .resource::<CommandResults>()
                .0
                .last()
                .unwrap()
                .outcome
                .is_ok()
        );
        assert_eq!(
            item_amount(app.world().resource::<EconomyLedger>(), true, 1),
            4
        );
        assert_eq!(
            item_amount(app.world().resource::<EconomyLedger>(), false, 1),
            2
        );
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
        assert!(
            !app.world()
                .resource::<CommittedTick>()
                .events
                .iter()
                .any(|event| {
                    matches!(
                        event.kind,
                        EconomyEventKind::ItemsProduced { machine: 1, .. }
                    )
                })
        );
        assert!(
            app.world()
                .resource::<CommittedTick>()
                .events
                .iter()
                .any(|event| {
                    matches!(
                        event.kind,
                        EconomyEventKind::ProductionCancelled { job: 1, .. }
                    )
                })
        );
    }

    #[test]
    fn cancel_rejects_refund_destination_full_and_changes_nothing() {
        let mut app = app_with_machine(2, 0);
        app.update();
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&1)
            .unwrap()
            .items
            .values_mut()
            .next()
            .unwrap()
            .on_hand = 6;
        app.world_mut().resource_mut::<AuditTrail>().enable();
        cancel(&mut app, 1);
        let audit = app.world().resource::<AuditTrail>();
        let command = audit.commands.last().unwrap();
        assert_eq!(
            command.result.outcome,
            Err(crate::RejectReason::RefundDestinationFull)
        );
        assert_eq!(command.before, command.after);
    }

    #[test]
    fn enabled_machine_restarts_same_tick_after_cancel() {
        let mut app = app_with_machine(2, 0);
        app.update();
        let tick = app.world().resource::<Tick>().0;
        cancel(&mut app, 1);
        assert_eq!(app.world().resource::<Tick>().0, tick + 1);
        let queue = &app.world().resource::<ProductionQueues>().0[&1];
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].id.0, 2);
        assert_eq!(queue[0].elapsed_ticks, 0);
    }

    #[test]
    fn disabled_machine_frozen_but_cancel_works() {
        let mut app = app_with_machine(2, 0);
        app.update();
        app.world_mut().resource_mut::<Disabled>().0.insert(1);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<ProductionQueues>().0[&1][0].elapsed_ticks,
            0
        );
        cancel(&mut app, 1);
        assert!(
            app.world()
                .resource::<CommandResults>()
                .0
                .last()
                .unwrap()
                .outcome
                .is_ok()
        );
        assert!(app.world().resource::<ProductionQueues>().0[&1].is_empty());
        assert_eq!(
            item_amount(app.world().resource::<EconomyLedger>(), false, 1),
            2
        );
    }
}
