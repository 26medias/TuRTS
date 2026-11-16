//! The system slot of construction placement (`SimSet::Construction`, plan task `L0-SIM`).
//! Held requests place before commands. Sites advance in the production phase.
//!
//! It runs before the commands of the tick, in `FixedPreUpdate`: the placement of held
//! `Construct` orders only. A site that completed, was destroyed or was cancelled in tick t has
//! freed its cap for the placement of tick t+1. Sites do not advance here: a site advances in
//! the one pay pass (`Production`), through `power::rate_factor`, as a production job does.

use std::collections::BTreeMap;

use bevy_ecs::{prelude::Resource, world::World};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use turts_mod_api::{ContentId, CostSource, PaymentPolicy};
use turts_rules::placement::{self, BuildSource, Circle, Obstacle, ObstacleKind, Point};

use super::{
    ledger::{Endpoint, ReservationLine},
    transaction,
};
use crate::{
    Owner, Position, RejectReason, SlotId, UnitId, UnitType,
    bootstrap::{InstalledCatalog, ProductionDurations},
    registry::UnitTypes,
    unit_index::unit_entity,
};

/// The site limit from the match definition. No construction is admitted without this resource.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ConstructionLimit(pub u16);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeldConstruction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<crate::receipts::CommandMember>,
    pub recipe: ContentId,
    pub point: Vec2,
}

/// A site owns its job independently of the constructor that placed it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Site {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<crate::receipts::CommandMember>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub funding: Option<super::production::FundingState>,
    pub constructor: u32,
    pub owner_slot: u8,
    pub recipe: ContentId,
    pub job: crate::JobId,
    pub done: u64,
    pub total: u64,
    pub(crate) charged: Vec<ReservationLine>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constructor_when_short: Vec<turts_mod_api::WhenShort>,
}

impl Site {
    /// Read-only original payment endpoints for independent accounting validation.
    pub fn charged_rows(&self) -> Vec<super::views::ReservationLineView> {
        self.charged
            .iter()
            .cloned()
            .map(super::views::reservation_line_view)
            .collect()
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConstructionState {
    pub sites: BTreeMap<u32, Site>,
    pub held: BTreeMap<u32, Vec<HeldConstruction>>,
    pub completed: Vec<u32>,
}

impl ConstructionState {
    pub fn is_empty(&self) -> bool {
        self.sites.is_empty() && self.held.is_empty() && self.completed.is_empty()
    }
}

fn definition(world: &World, unit: u32) -> Option<&turts_mod_api::ResolvedUnit> {
    let entity = unit_entity(world, unit)?;
    let kind = world.get::<UnitType>(entity)?;
    let id = &world.get_resource::<UnitTypes>()?.get(kind.0)?.content_id;
    world
        .get_resource::<InstalledCatalog>()?
        .0
        .units
        .get(&ContentId::parse(id).ok()?)
}

fn circle(point: Vec2, radius: f32) -> Circle {
    Circle {
        centre: Point {
            x: point.x,
            y: point.y,
        },
        radius,
    }
}

pub fn check_placement(
    world: &World,
    slot: SlotId,
    output: &turts_mod_api::ResolvedUnit,
    point: Vec2,
    queue: Option<placement::QueueFill>,
) -> Result<(), RejectReason> {
    let config = output.placement.ok_or(RejectReason::MissingCapability)?;
    if let Some(terrain) = world.get_resource::<crate::terrain::TerrainState>() {
        let turts_mod_api::Footprint::Circle { radius } = output.footprint;
        let mut budget = turts_rules::terrain::geometry::Budget::new(2_097_168);
        terrain
            .grid
            .flat_foundation(
                terrain,
                turts_rules::terrain::geometry::Point {
                    x: f64::from(point.x),
                    y: f64::from(point.y),
                },
                f64::from(radius),
                &mut budget,
            )
            .map_err(|_| RejectReason::SiteBlocked)?;
    }

    let bounds = world
        .get_resource::<crate::MapBounds>()
        .ok_or(RejectReason::IntegrityViolation)?;
    let types = world.resource::<UnitTypes>();
    let state = world.resource::<ConstructionState>();
    let mut sources = Vec::new();
    let mut obstacles = Vec::new();
    for entity in world.iter_entities() {
        let (Some(id), Some(position), Some(kind)) = (
            entity.get::<UnitId>(),
            entity.get::<Position>(),
            entity.get::<UnitType>(),
        ) else {
            continue;
        };
        let Some(entry) = types.get(kind.0) else {
            continue;
        };
        let footprint = circle(position.0, entry.radius);
        let def = definition(world, id.0);
        let is_site = state.sites.contains_key(&id.0);
        if !is_site
            && entity.get::<Owner>() == Some(&Owner(slot))
            && let Some(area) = def.and_then(|unit| unit.build_area)
        {
            sources.push(BuildSource {
                footprint,
                build_area: area,
            });
        }
        obstacles.push(Obstacle {
            id: id.0,
            footprint,
            kind: if entry.movement.is_some() && !is_site {
                ObstacleKind::Mobile
            } else {
                ObstacleKind::Static {
                    gap: def
                        .and_then(|unit| unit.placement)
                        .map_or(0.0, |placement| placement.gap),
                }
            },
        });
    }
    let catalog = &world.resource::<InstalledCatalog>().0;
    let nodes = world
        .resource::<super::SurfaceNodes>()
        .0
        .iter()
        .filter(|node| node.remaining > 0)
        .filter_map(|node| {
            Some(circle(
                node.position.0,
                catalog.nodes.get(&node.content_id)?.radius,
            ))
        })
        .collect::<Vec<_>>();
    let turts_mod_api::Footprint::Circle { radius } = output.footprint;
    let verdict = placement::check(
        &placement::Site {
            footprint: circle(point, radius),
            gap: config.gap,
            node_clearance: config.node_clearance,
            in_build_area: config.in_build_area,
            queue,
        },
        &placement::View {
            map_width: bounds.size.x,
            map_height: bounds.size.y,
            build_sources: &sources,
            obstacles: &obstacles,
            nodes: &nodes,
        },
        |_| true,
    );
    match verdict {
        placement::Verdict::Valid => Ok(()),
        placement::Verdict::Refused { reason, .. } => Err(match reason {
            placement::Reason::SiteOutsideMap => RejectReason::SiteOutsideMap,
            placement::Reason::SiteOutOfArea => RejectReason::SiteOutOfArea,
            placement::Reason::SiteBlocked => RejectReason::SiteBlocked,
            placement::Reason::SiteNearBuilding => RejectReason::SiteNearBuilding,
            placement::Reason::SiteNearResource => RejectReason::SiteNearResource,
            placement::Reason::ConstructionQueueFull => RejectReason::ConstructionQueueFull,
        }),
    }
}

/// Admit a declared construction request. A refusal changes no balances, ids, or sites.
pub fn construct(
    world: &mut World,
    slot: SlotId,
    constructor: u32,
    recipe: ContentId,
    point: Vec2,
) -> Result<Option<crate::JobId>, RejectReason> {
    construct_with_origin(world, slot, constructor, recipe, point, None)
}

/// The origin identifies ordinary accepted work. It changes no validation or payment.
pub(crate) fn construct_with_origin(
    world: &mut World,
    slot: SlotId,
    constructor: u32,
    recipe: ContentId,
    point: Vec2,
    origin: Option<crate::receipts::CommandMember>,
) -> Result<Option<crate::JobId>, RejectReason> {
    let entity = unit_entity(world, constructor).ok_or(RejectReason::UnknownUnit)?;
    if world.get::<Owner>(entity) != Some(&Owner(slot)) {
        return Err(RejectReason::NotOwner);
    }
    if world
        .resource::<ConstructionState>()
        .sites
        .contains_key(&constructor)
    {
        return Err(RejectReason::NotApplicable);
    }
    let capability = definition(world, constructor)
        .and_then(|unit| unit.constructor.as_ref())
        .ok_or(RejectReason::MissingCapability)?;
    if !capability.buildings.contains(&recipe) {
        return Err(RejectReason::RecipeNotEnabled);
    }
    let concurrent = usize::from(capability.concurrent_sites);
    let queue_capacity = capability.queue_capacity;
    let catalog = &world.resource::<InstalledCatalog>().0;
    let production = catalog
        .production
        .get(&recipe)
        .ok_or(RejectReason::UnknownRecipe)?;
    let output = catalog
        .units
        .get(&production.output_unit)
        .ok_or(RejectReason::UnknownRecipe)?;
    if output.extractor.is_some() || output.inserter.is_some() || output.conveyor.is_some() {
        return Err(RejectReason::NotApplicable);
    }
    let state = world.resource::<ConstructionState>();
    check_placement(
        world,
        slot,
        output,
        point,
        Some(placement::QueueFill {
            held: state
                .held
                .get(&constructor)
                .map_or(0, |queue| queue.len() as u32),
            capacity: u32::from(queue_capacity),
        }),
    )?;
    let limit = usize::from(
        world
            .get_resource::<ConstructionLimit>()
            .ok_or(RejectReason::NotApplicable)?
            .0,
    );
    let has_room = state
        .sites
        .values()
        .filter(|site| site.owner_slot == slot.0)
        .count()
        < limit
        && state
            .sites
            .values()
            .filter(|site| site.constructor == constructor)
            .count()
            < concurrent;
    if has_room && state.held.get(&constructor).is_none_or(Vec::is_empty) {
        place(world, slot, constructor, recipe, point, false, origin).map(Some)
    } else {
        transaction::transact(world, |tx| {
            let mut state = tx.construction();
            state
                .held
                .entry(constructor)
                .or_default()
                .push(HeldConstruction {
                    recipe,
                    point,
                    origin,
                });
            tx.stage_construction(state);
            Ok(None)
        })
    }
}

fn place(
    world: &mut World,
    slot: SlotId,
    constructor: u32,
    recipe_id: ContentId,
    point: Vec2,
    held: bool,
    origin: Option<crate::receipts::CommandMember>,
) -> Result<crate::JobId, RejectReason> {
    let catalog = &world.resource::<InstalledCatalog>().0;
    let recipe = catalog
        .production
        .get(&recipe_id)
        .ok_or(RejectReason::UnknownRecipe)?
        .clone();
    let output = catalog
        .units
        .get(&recipe.output_unit)
        .ok_or(RejectReason::UnknownRecipe)?
        .clone();
    check_placement(world, slot, &output, point, None)?;
    let unit_type = world
        .resource::<UnitTypes>()
        .find(&output.id.to_string())
        .ok_or(RejectReason::MissingCapability)?;
    let total = *world
        .resource::<ProductionDurations>()
        .0
        .get(&recipe_id)
        .ok_or(RejectReason::UnknownRecipe)?;
    let total = total
        .checked_mul(super::power::rate_q(world))
        .ok_or(RejectReason::ArithmeticOverflow)?;
    let constructor_when_short = if world.contains_resource::<super::power::OperatingResources>() {
        definition(world, constructor)
            .and_then(|unit| unit.constructor.as_ref())
            .map(|constructor| constructor.when_short.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let lines = recipe
        .costs
        .iter()
        .map(|cost| ReservationLine {
            endpoint: match cost.source {
                CostSource::SlotBalance => Endpoint::Slot(slot.0),
                CostSource::ProducerInventory => Endpoint::Inventory(constructor),
            },
            resource: cost.resource.clone(),
            amount: cost.amount,
        })
        .collect::<Vec<_>>();
    let mut capability_rows = crate::boot_m3::PreparedCapabilityRows {
        inventory: None,
        output_inventory: None,
        extractor: None,
        inserter: None,
        weapon: None,
        health: None,
        has_queue: false,
    };
    if let Some(health) = output.health {
        let ratio = output
            .placement
            .ok_or(RejectReason::MissingCapability)?
            .site_health;
        let ratio =
            turts_rules::Ratio::new(u64::from(ratio.numerator), u64::from(ratio.denominator))
                .ok_or(RejectReason::ArithmeticOverflow)?;
        capability_rows.health = Some(turts_rules::heal::site_health_target(
            health.max, ratio, 0, total,
        ));
    }
    let job = transaction::transact(world, |tx| {
        let job = tx.allocate_job_id()?;
        match recipe.payment {
            PaymentPolicy::ChargeOnQueue => transaction::charge_costs(tx, &lines)?,
            PaymentPolicy::ReserveThenCharge => {
                let reservation = transaction::reserve_costs(tx, job, &lines)?;
                transaction::consume_reservation(tx, &reservation)?;
            }
            PaymentPolicy::ChargeOverTime => {}
        }
        let id = tx.allocate_unit_id()?;
        let mut state = tx.construction();
        if held {
            state
                .held
                .get_mut(&constructor)
                .ok_or(RejectReason::IntegrityViolation)?
                .remove(0);
        }
        state.sites.insert(
            id,
            Site {
                origin,
                constructor,
                owner_slot: slot.0,
                recipe: recipe_id.clone(),
                job,
                done: 0,
                total,
                charged: if recipe.payment == PaymentPolicy::ChargeOverTime {
                    Vec::new()
                } else {
                    lines
                },
                funding: (recipe.payment == PaymentPolicy::ChargeOverTime)
                    .then(super::production::FundingState::default),
                constructor_when_short,
            },
        );
        tx.stage_construction(state);
        tx.stage_spawn(transaction::SpawnWrite {
            id,
            unit_type,
            owner_slot: slot.0,
            position: point,
            state: crate::world::UnitState {
                unit_id: id,
                content_id: output.id.to_string(),
                owner_slot: slot.0,
                facing: None,
                pos: point,
            },
            inventory: None,
            gatherer: false,
            producer: false,
            capability_rows,
        });
        Ok(job)
    })?;
    if let Some(origin) = origin {
        crate::receipts::capture_origin_work(world, origin, job);
    }
    Ok(job)
}

/// Place held requests before commands. A blocked or unpaid head keeps its place in the queue.
/// Cancel one explicitly addressed unfinished site. A refused refund leaves it unchanged.
pub fn cancel_site(world: &mut World, slot: SlotId, id: u32) -> Result<(), RejectReason> {
    let entity = unit_entity(world, id).ok_or(RejectReason::UnknownUnit)?;
    if world.get::<Owner>(entity) != Some(&Owner(slot)) {
        return Err(RejectReason::NotOwner);
    }
    let site = world
        .resource::<ConstructionState>()
        .sites
        .get(&id)
        .cloned()
        .ok_or(RejectReason::NotApplicable)?;
    let ratio = world
        .resource::<InstalledCatalog>()
        .0
        .production
        .get(&site.recipe)
        .ok_or(RejectReason::UnknownRecipe)?
        .cancel_refunds
        .running;
    transaction::transact(world, |tx| {
        if !site.charged.is_empty() {
            transaction::refund_charged(
                tx,
                &site.charged,
                u64::from(ratio.numerator),
                u64::from(ratio.denominator),
            )?;
        }
        let mut funding = site.funding.clone();
        super::production::flush_funding(tx, slot.0, id, site.job.0, &mut funding)?;
        tx.stage_site_terminal(id, &site, crate::receipts::TerminalWorkState::Cancelled);
        let mut construction = tx.construction();
        construction
            .sites
            .get_mut(&id)
            .ok_or(RejectReason::IntegrityViolation)?
            .funding = funding;
        tx.stage_construction(construction);
        let refunded = site
            .charged
            .iter()
            .map(|line| {
                let amount = u64::try_from(
                    u128::from(line.amount) * u128::from(ratio.numerator)
                        / u128::from(ratio.denominator),
                )
                .map_err(|_| RejectReason::ArithmeticOverflow)?;
                Ok(super::events::CostLine {
                    source: match line.endpoint {
                        Endpoint::Slot(slot) => super::events::EventSource::Slot(slot),
                        Endpoint::Inventory(unit) => super::events::EventSource::Inventory(unit),
                        Endpoint::Output(unit) => super::events::EventSource::Output(unit),
                    },
                    resource: line.resource.clone(),
                    amount,
                })
            })
            .collect::<Result<Vec<_>, RejectReason>>()?;
        tx.stage_event(
            slot.0,
            super::events::EconomyEventKind::ProductionCancelled {
                producer: id,
                job: site.job.0,
                released: Vec::new(),
                refunded,
            },
        )?;
        transaction::remove_unit(tx, id, super::events::RemoveCause::CancelledConstruction)
    })
}

/// Stop clears unplaced requests only. Existing sites and a hybrid producer queue survive.
pub fn clear_held(world: &mut World, slot: SlotId, constructor: u32) -> Result<(), RejectReason> {
    let entity = unit_entity(world, constructor).ok_or(RejectReason::UnknownUnit)?;
    if world.get::<Owner>(entity) != Some(&Owner(slot)) {
        return Err(RejectReason::NotOwner);
    }
    if definition(world, constructor)
        .and_then(|unit| unit.constructor.as_ref())
        .is_none()
    {
        return Err(RejectReason::MissingCapability);
    }
    transaction::transact(world, |tx| {
        let mut state = tx.construction();
        state.held.remove(&constructor);
        tx.stage_construction(state);
        Ok(())
    })
}

pub(crate) fn run_construction(world: &mut World) {
    let constructors = world
        .resource::<ConstructionState>()
        .held
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for constructor in constructors {
        let Some(entity) = unit_entity(world, constructor) else {
            world
                .resource_mut::<ConstructionState>()
                .held
                .remove(&constructor);
            continue;
        };
        let Some(owner) = world.get::<Owner>(entity).copied() else {
            continue;
        };
        while let Some(capability) =
            definition(world, constructor).and_then(|unit| unit.constructor.as_ref())
        {
            let state = world.resource::<ConstructionState>();
            let Some(order) = state
                .held
                .get(&constructor)
                .and_then(|queue| queue.first())
                .cloned()
            else {
                break;
            };
            let Some(limit) = world.get_resource::<ConstructionLimit>() else {
                break;
            };
            if state
                .sites
                .values()
                .filter(|site| site.owner_slot == owner.0.0)
                .count()
                >= usize::from(limit.0)
                || state
                    .sites
                    .values()
                    .filter(|site| site.constructor == constructor)
                    .count()
                    >= usize::from(capability.concurrent_sites)
            {
                break;
            }
            if place(
                world,
                owner.0,
                constructor,
                order.recipe,
                order.point,
                true,
                order.origin,
            )
            .is_err()
            {
                break;
            }
        }
    }
}

/// Add construction health by the shared difference rule. Damage is never erased at completion.
pub(crate) fn advance_sites(world: &mut World) {
    let ids = world
        .resource::<ConstructionState>()
        .sites
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for id in ids {
        advance_site(world, id);
    }
}

pub(crate) fn advance_site(world: &mut World, id: u32) {
    let Some(site) = world
        .resource::<ConstructionState>()
        .sites
        .get(&id)
        .cloned()
    else {
        return;
    };
    let Some(output) = definition(world, id).cloned() else {
        return;
    };
    let Some(entity) = unit_entity(world, id) else {
        return;
    };
    let Some(point) = world.get::<Position>(entity).map(|position| position.0) else {
        return;
    };
    if super::production::freeze_facts_in_world(world, id).frozen()
        || super::production::freeze_facts_in_world(world, site.constructor).frozen()
    {
        return;
    }
    let rate = super::power::rate_for_rules(world, site.owner_slot, &site.constructor_when_short);
    let Some(step) = super::production::job_step_per_tick(
        super::production::JobStage::Running,
        false,
        rate,
        super::power::rate_q(world),
    ) else {
        return;
    };
    let Some(done) = site
        .total
        .checked_sub(site.done)
        .and_then(|remaining| site.done.checked_add(step.min(remaining)))
    else {
        return;
    };
    if done == site.done {
        return;
    }
    let costs = if site.funding.is_some() {
        let Some(recipe) = world
            .get_resource::<InstalledCatalog>()
            .and_then(|catalog| catalog.0.production.get(&site.recipe))
        else {
            return;
        };
        super::production::payment_lines(&recipe.costs, site.owner_slot, site.constructor)
    } else {
        Vec::new()
    };
    let tick = world.resource::<crate::world::Tick>().0;
    let hz = world.resource::<crate::SimClock>().tick_hz;
    let completed = done == site.total;
    let activation = if completed && world.contains_resource::<super::power::OperatingResources>() {
        match super::operating::prepare_activation(world, &output, site.owner_slot) {
            Ok(grant) => Some(grant),
            Err(reason) => {
                super::operating::hold(world, id, reason);
                return;
            }
        }
    } else {
        None
    };
    let rows = if completed {
        let Ok(binding) = crate::boot_m3::resolve_spawn_capability_binding(
            world,
            &output,
            point,
            site.owner_slot,
            None,
        ) else {
            return;
        };
        let mut rows = crate::boot_m3::install_capability_rows(&output, binding);
        rows.health = None;
        Some(rows)
    } else {
        None
    };
    let result = transaction::transact(world, |tx| {
        let mut state = tx.construction();
        let current = state
            .sites
            .get_mut(&id)
            .ok_or(RejectReason::IntegrityViolation)?;
        if current.funding.is_some() {
            match super::production::charge_progress(tx, &costs, site.done, done, site.total)? {
                super::production::PaymentAttempt::Waiting(resource) => {
                    let entering = current
                        .funding
                        .as_mut()
                        .ok_or(RejectReason::IntegrityViolation)?
                        .wait(resource, tick);
                    if entering {
                        super::production::flush_funding(
                            tx,
                            site.owner_slot,
                            id,
                            site.job.0,
                            &mut current.funding,
                        )?;
                    }
                    tx.stage_construction(state);
                    return Ok(());
                }
                super::production::PaymentAttempt::Funded(payment) => {
                    let funding = current
                        .funding
                        .as_mut()
                        .ok_or(RejectReason::IntegrityViolation)?;
                    funding.add_charges(&payment.debits)?;
                    funding.advance(hz);
                    current.charged = payment.charged;
                    if completed {
                        super::production::flush_funding(
                            tx,
                            site.owner_slot,
                            id,
                            site.job.0,
                            &mut current.funding,
                        )?;
                    }
                }
            }
        }
        if let (Some(health), Some(placement)) = (output.health, output.placement) {
            let ratio = turts_rules::Ratio::new(
                u64::from(placement.site_health.numerator),
                u64::from(placement.site_health.denominator),
            )
            .ok_or(RejectReason::IntegrityViolation)?;
            let gain = turts_rules::heal::site_health_difference(
                health.max, ratio, site.done, done, site.total,
            );
            let current = tx.health(id)?;
            tx.stage_health(
                id,
                current
                    .checked_add(gain)
                    .ok_or(RejectReason::ArithmeticOverflow)?
                    .min(health.max),
            );
        }
        if let Some(rows) = rows {
            let mut terminal = state
                .sites
                .get(&id)
                .cloned()
                .ok_or(RejectReason::IntegrityViolation)?;
            terminal.done = done;
            tx.stage_site_terminal(id, &terminal, crate::receipts::TerminalWorkState::Completed);
            state.sites.remove(&id);
            state.completed.push(id);
            tx.stage_completed_building(id, rows)?;
            if let Some((grant, clock)) = activation {
                tx.stage_operating_activation(id, grant, clock)?;
            }
        } else {
            state
                .sites
                .get_mut(&id)
                .ok_or(RejectReason::IntegrityViolation)?
                .done = done;
        }
        tx.stage_construction(state);
        Ok(())
    });
    match result {
        Err(reason) => super::operating::hold(world, id, reason),
        Ok(()) => super::operating::clear_hold(world, id),
    }
}

fn site_hold(
    view: &super::views::WorldView,
    unit: u32,
) -> Option<&super::operating::OperatingHold> {
    view.operating
        .as_ref()?
        .holds
        .iter()
        .find(|hold| hold.unit == unit)
}
fn site_step(view: &super::views::WorldView, unit: u32, site: &Site) -> u64 {
    if site_hold(view, unit).is_some()
        || site
            .funding
            .as_ref()
            .is_some_and(|state| state.waiting_for_funds.is_some())
        || super::production::freeze_facts_in_view(view, unit).frozen()
        || super::production::freeze_facts_in_view(view, site.constructor).frozen()
    {
        return 0;
    }
    let q = view
        .operating
        .as_ref()
        .map_or(1, |operating| operating.rate_q);
    let rate =
        super::power::rate_for_rules_in_view(view, site.owner_slot, &site.constructor_when_short);
    super::production::job_step_per_tick(super::production::JobStage::Running, false, rate, q)
        .unwrap_or(0)
}

pub fn progress_rows(view: &super::views::WorldView) -> Vec<super::events::RateRecordRow> {
    progress_rows_matching(view, None).collect()
}
/// Filter the owner before allocating progress rows or cloning payment endpoints.
pub fn progress_rows_for_slot(
    view: &super::views::WorldView,
    slot: SlotId,
) -> impl Iterator<Item = super::events::RateRecordRow> + '_ {
    progress_rows_matching(view, Some(slot))
}
fn progress_rows_matching(
    view: &super::views::WorldView,
    slot: Option<SlotId>,
) -> impl Iterator<Item = super::events::RateRecordRow> + '_ {
    view.construction
        .sites
        .iter()
        .filter(move |(_, site)| slot.is_none_or(|slot| site.owner_slot == slot.0))
        .map(move |(id, site)| super::events::RateRecordRow {
            production: Some(site.recipe.clone()),
            unit: *id,
            owner_slot: site.owner_slot,
            job: None,
            done: site.done,
            total: site.total,
            per_tick: site_step(view, *id, site),
            at_tick: view.tick,
            state: if let Some(resource) = site
                .funding
                .as_ref()
                .and_then(|state| state.waiting_for_funds.clone())
            {
                super::events::RecordStateRow::WaitingForFunds(resource)
            } else if site_hold(view, *id).is_some() {
                super::events::RecordStateRow::Held
            } else if super::power::rate_for_rules_in_view(
                view,
                site.owner_slot,
                &site.constructor_when_short,
            ) != turts_rules::Ratio::ONE
            {
                super::events::RecordStateRow::Slowed
            } else {
                super::events::RecordStateRow::Running
            },
            charged: site
                .charged
                .iter()
                .map(|line| super::views::CostView {
                    source: match line.endpoint {
                        Endpoint::Slot(_) => CostSource::SlotBalance,
                        _ => CostSource::ProducerInventory,
                    },
                    resource: line.resource.clone(),
                    amount: line.amount,
                })
                .collect(),
            held: site_hold(view, *id).is_some()
                || site
                    .funding
                    .as_ref()
                    .is_some_and(|state| state.held_since_tick.is_some()),
            held_since_tick: site_hold(view, *id).map_or_else(
                || {
                    site.funding
                        .as_ref()
                        .and_then(|state| state.held_since_tick)
                        .unwrap_or(0)
                },
                |hold| hold.since_tick,
            ),
        })
}

pub fn queue_rows(view: &super::views::WorldView) -> Vec<super::events::ConstructionQueueRow> {
    queue_rows_matching(view, None).collect()
}
/// Filter the owner before cloning any held recipe list.
pub fn queue_rows_for_slot(
    view: &super::views::WorldView,
    slot: SlotId,
) -> impl Iterator<Item = super::events::ConstructionQueueRow> + '_ {
    queue_rows_matching(view, Some(slot))
}
fn queue_rows_matching(
    view: &super::views::WorldView,
    slot: Option<SlotId>,
) -> impl Iterator<Item = super::events::ConstructionQueueRow> + '_ {
    view.construction
        .held
        .iter()
        .filter_map(move |(constructor, queue)| {
            let owner_slot = view
                .units
                .iter()
                .find(|unit| unit.unit_id == *constructor)?
                .owner_slot;
            if slot.is_some_and(|slot| slot.0 != owner_slot) {
                return None;
            }
            Some(super::events::ConstructionQueueRow {
                constructor: *constructor,
                owner_slot,
                orders: queue
                    .iter()
                    .map(|order| (order.recipe.clone(), order.point))
                    .collect(),
            })
        })
}

/// Publish existing held lists and one empty replacement when a surviving source clears.
/// Removal is represented by the removed-unit record, never by another queue record.
pub fn queue_changes(
    previous: &super::views::WorldView,
    current: &super::views::WorldView,
) -> Vec<super::events::ConstructionQueueRow> {
    let mut rows = queue_rows(current)
        .into_iter()
        .filter(|row| !row.orders.is_empty())
        .map(|row| (row.constructor, row))
        .collect::<BTreeMap<_, _>>();
    for old in queue_rows(previous)
        .into_iter()
        .filter(|row| !row.orders.is_empty())
    {
        if !rows.contains_key(&old.constructor)
            && current
                .units
                .iter()
                .any(|unit| unit.unit_id == old.constructor)
        {
            rows.insert(
                old.constructor,
                super::events::ConstructionQueueRow {
                    constructor: old.constructor,
                    owner_slot: old.owner_slot,
                    orders: Vec::new(),
                },
            );
        }
    }
    rows.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate};
    use turts_mod_api::*;

    fn cid(local: &str) -> ContentId {
        ContentId::parse(&format!("build:{local}")).unwrap()
    }

    fn fixture(policy: PaymentPolicy) -> App {
        let mut app = crate::headless_app(20);
        let tower = ResolvedUnit {
            terrain_class: None,
            terrain_cargo: None,
            terrain_foundation: None,
            terrain_tools: Vec::new(),
            vision: None,
            repairer: None,
            consumes: Vec::new(),
            income: None,
            provides: Vec::new(),
            acquire: None,
            armour: None,
            id: cid("tower"),
            name: "Tower".into(),
            footprint: Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: Some(ResolvedProducer {
                when_short: Vec::new(),

                recipes: vec![cid("train")],
                queue_capacity: 2,
                spawn_offsets: vec![WorldPos { x: 3.0, y: 0.0 }],
            }),
            constructor: None,
            placement: Some(SitePlacementDef {
                gap: 1.0,
                node_clearance: 1.0,
                in_build_area: true,
                site_health: Ratio {
                    numerator: 1,
                    denominator: 2,
                },
            }),
            build_area: Some(8.0),
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: Some(HealthDef { max: 100 }),
            weapon: None,
            removal: None,
        };
        let mut base = tower.clone();
        base.id = cid("base");
        base.name = "Base".into();
        base.producer = None;
        base.constructor = Some(ResolvedConstructor {
            buildings: vec![cid("build_tower")],
            concurrent_sites: 1,
            queue_capacity: 2,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
        base.build_area = Some(15.0);
        let mut soldier = tower.clone();
        soldier.id = cid("soldier");
        soldier.movement = Some(MovementDef { speed: 3.0 });
        soldier.producer = None;
        soldier.placement = None;
        soldier.build_area = None;
        let refunds = CancelRefunds {
            queued: RefundRatio::one(),
            running: RefundRatio::one(),
            output_blocked: RefundRatio::one(),
            destroyed: RefundRatio {
                numerator: 0,
                denominator: 1,
            },
        };
        let recipe = |id: &str, output: &str, amount| ResolvedProduction {
            id: cid(id),
            name: id.into(),
            output_unit: cid(output),
            duration_ms: 100,
            costs: vec![ResolvedCost {
                resource: cid("credits"),
                amount,
                source: CostSource::SlotBalance,
            }],
            payment: policy,
            cancel_refunds: refunds,
        };
        let catalog = ContentCatalog {
            terrain_materials: Default::default(),
            terrain_movement_classes: Default::default(),
            terrain_tools: Default::default(),
            bots: Default::default(),

            combat: None,
            resources: BTreeMap::from([(
                cid("credits"),
                ResourceDef {
                    id: "credits".into(),
                    name: "Credits".into(),
                    unit_name: "credit".into(),
                    kind: ResourceKind::Balance,
                },
            )]),
            units: BTreeMap::from([
                (base.id.clone(), base),
                (tower.id.clone(), tower),
                (soldier.id.clone(), soldier),
            ]),
            production: BTreeMap::from([
                (cid("build_tower"), recipe("build_tower", "tower", 100)),
                (cid("train"), recipe("train", "soldier", 20)),
            ]),
            nodes: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        let map = MapDef {
            terrain: None,
            visibility: None,
            environment: None,
            ground: None,
            id: "field".into(),
            size: MapSize { x: 64.0, y: 64.0 },
            placements: vec![Placement {
                unit: "build:base".into(),
                owner_slot: 0,
                pos: WorldPos { x: 10.0, y: 10.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            }],
            nodes: vec![],
            slot_economy: vec![SlotEconomyInit {
                slot: 0,
                balances: vec![BalanceAmount {
                    balance: "build:credits".into(),
                    amount: 500,
                }],
            }],
            belts: vec![],
        };
        let staged = crate::stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "maps/field.ron".into(),
                value: map,
            },
            20,
        )
        .unwrap();
        crate::install_package(app.world_mut(), staged);
        app.insert_resource(ConstructionLimit(2));
        app
    }

    fn tick(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut().run_schedule(FixedPostUpdate);
    }

    fn balance(app: &App) -> u64 {
        crate::balance_view(app.world(), SlotId(0), &cid("credits"))
            .unwrap()
            .on_hand
    }

    #[test]
    fn a_site_charges_once_then_completes_with_the_same_identity_and_can_train() {
        for policy in [
            PaymentPolicy::ChargeOnQueue,
            PaymentPolicy::ReserveThenCharge,
        ] {
            let mut app = fixture(policy);
            let job = construct(
                app.world_mut(),
                SlotId(0),
                1,
                cid("build_tower"),
                Vec2::new(16.0, 10.0),
            )
            .unwrap()
            .unwrap();
            assert_ne!(job.0, 0);
            let id = *app
                .world()
                .resource::<ConstructionState>()
                .sites
                .keys()
                .next()
                .unwrap();
            assert_eq!(balance(&app), 400);
            assert_eq!(
                app.world().resource::<super::super::combat::Healths>().0[&id],
                50
            );
            assert_eq!(
                super::super::production::queue_production(
                    app.world_mut(),
                    SlotId(0),
                    id,
                    cid("train")
                ),
                Err(RejectReason::NotApplicable)
            );
            tick(&mut app);
            assert_eq!(
                app.world().resource::<ConstructionState>().sites[&id].done,
                1
            );
            // Construction adds health by progress, so existing damage remains at completion.
            app.world_mut()
                .resource_mut::<super::super::combat::Healths>()
                .0
                .insert(id, 55);
            tick(&mut app);
            assert!(app.world().resource::<ConstructionState>().sites.is_empty());
            assert_eq!(
                app.world().resource::<super::super::combat::Healths>().0[&id],
                80
            );
            assert_eq!(crate::committed_tick(app.world()).sites_completed, vec![id]);
            assert_eq!(balance(&app), 400);
            super::super::production::queue_production(
                app.world_mut(),
                SlotId(0),
                id,
                cid("train"),
            )
            .unwrap();
            for _ in 0..4 {
                tick(&mut app);
            }
            assert_eq!(balance(&app), 380);
            assert!(
                crate::completed_world(app.world())
                    .units
                    .iter()
                    .any(|unit| unit.content_id == "build:soldier")
            );
        }
    }

    #[test]
    fn placement_and_ownership_refusals_leave_payment_ids_and_sites_unchanged() {
        let mut app = fixture(PaymentPolicy::ChargeOnQueue);
        for (slot, point, reason) in [
            (SlotId(1), Vec2::new(16.0, 10.0), RejectReason::NotOwner),
            (
                SlotId(0),
                Vec2::new(-1.0, 10.0),
                RejectReason::SiteOutsideMap,
            ),
            (
                SlotId(0),
                Vec2::new(55.0, 55.0),
                RejectReason::SiteOutOfArea,
            ),
            (SlotId(0), Vec2::new(10.0, 10.0), RejectReason::SiteBlocked),
            (
                SlotId(0),
                Vec2::new(12.5, 10.0),
                RejectReason::SiteNearBuilding,
            ),
        ] {
            let before = crate::audit_state(app.world_mut());
            assert_eq!(
                construct(app.world_mut(), slot, 1, cid("build_tower"), point),
                Err(reason)
            );
            assert_eq!(
                crate::audit_state(app.world_mut()),
                before,
                "{reason:?} changed the world"
            );
        }
        app.world_mut()
            .resource_mut::<super::super::ledger::EconomyLedger>()
            .balances
            .get_mut(&(0, cid("credits")))
            .unwrap()
            .on_hand = 5;
        let before = crate::audit_state(app.world_mut());
        assert_eq!(
            construct(
                app.world_mut(),
                SlotId(0),
                1,
                cid("build_tower"),
                Vec2::new(16.0, 10.0)
            ),
            Err(RejectReason::InsufficientBalance)
        );
        assert_eq!(crate::audit_state(app.world_mut()), before);
    }

    #[test]
    fn held_requests_spend_only_when_a_site_cap_is_free_on_the_next_tick() {
        let mut app = fixture(PaymentPolicy::ChargeOnQueue);
        construct(
            app.world_mut(),
            SlotId(0),
            1,
            cid("build_tower"),
            Vec2::new(16.0, 10.0),
        )
        .unwrap();
        assert_eq!(
            construct(
                app.world_mut(),
                SlotId(0),
                1,
                cid("build_tower"),
                Vec2::new(20.0, 10.0)
            ),
            Ok(None)
        );
        assert_eq!(balance(&app), 400);
        tick(&mut app);
        tick(&mut app);
        assert_eq!(balance(&app), 400);
        assert_eq!(
            app.world().resource::<ConstructionState>().held[&1].len(),
            1
        );
        tick(&mut app);
        assert_eq!(balance(&app), 300);
        assert!(app.world().resource::<ConstructionState>().held[&1].is_empty());
        assert_eq!(app.world().resource::<ConstructionState>().sites.len(), 1);
    }

    #[test]
    fn a_destroyed_site_never_completes_or_refunds() {
        let mut app = fixture(PaymentPolicy::ReserveThenCharge);
        construct(
            app.world_mut(),
            SlotId(0),
            1,
            cid("build_tower"),
            Vec2::new(16.0, 10.0),
        )
        .unwrap();
        tick(&mut app);
        let id = *app
            .world()
            .resource::<ConstructionState>()
            .sites
            .keys()
            .next()
            .unwrap();
        transaction::transact(app.world_mut(), |tx| {
            transaction::remove_unit(tx, id, super::super::events::RemoveCause::Killed { by: 1 })
        })
        .unwrap();
        for _ in 0..4 {
            tick(&mut app);
        }
        assert_eq!(balance(&app), 400);
        assert!(app.world().resource::<ConstructionState>().sites.is_empty());
        assert!(
            !crate::completed_world(app.world())
                .units
                .iter()
                .any(|unit| unit.unit_id == id)
        );
        assert!(
            crate::committed_tick(app.world())
                .sites_completed
                .is_empty()
        );
    }

    #[test]
    fn construction_orders_replay_to_the_same_state_through_the_command_queue() {
        use crate::orders::{OrderCommand, OrderTargetSim, QueueMode};
        let run = || {
            let mut app = fixture(PaymentPolicy::ChargeOnQueue);
            for (seq, slot, point) in [
                (1, SlotId(1), Vec2::new(16.0, 10.0)),
                (2, SlotId(0), Vec2::new(16.0, 10.0)),
                (3, SlotId(0), Vec2::new(20.0, 10.0)),
            ] {
                app.world_mut()
                    .resource_mut::<crate::CommandQueue>()
                    .0
                    .push_back(crate::Command {
                        slot,
                        seq,
                        kind: crate::CommandKind::Order(Box::new(OrderCommand {
                            order: cid("construct"),
                            behaviour: Behaviour::Construct,
                            target_kind: TargetKind::ContentAtPoint,
                            policy: None,
                            units: vec![1],
                            target: OrderTargetSim::ContentAtPoint {
                                content: cid("build_tower"),
                                point,
                            },
                            queue: QueueMode::Replace,
                            group: 0,
                            allowed_units: vec!["build:base".into()],
                            queue_depth: 16,
                        })),
                    });
                tick(&mut app);
                let results = &app.world().resource::<crate::CommandResults>().0;
                if seq == 1 {
                    assert_eq!(results[0].outcome, Err(RejectReason::NotOwner));
                } else {
                    assert!(results[0].outcome.is_ok(), "{results:?}");
                }
            }
            for _ in 0..5 {
                tick(&mut app);
            }
            assert_eq!(balance(&app), 300);
            assert_eq!(crate::completed_world(app.world()).units.len(), 3);
            crate::audit_state(app.world_mut())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn unsupported_construction_outputs_are_refused_before_payment_or_allocation() {
        let mut app = fixture(PaymentPolicy::ChargeOnQueue);
        app.world_mut()
            .resource_mut::<InstalledCatalog>()
            .0
            .units
            .get_mut(&cid("tower"))
            .unwrap()
            .extractor = Some(ExtractorDef {
            per_second: 1,
            interaction_range: 3.0,
        });
        let before = crate::audit_state(app.world_mut());
        assert_eq!(
            construct(
                app.world_mut(),
                SlotId(0),
                1,
                cid("build_tower"),
                Vec2::new(16.0, 10.0)
            ),
            Err(RejectReason::NotApplicable)
        );
        assert_eq!(crate::audit_state(app.world_mut()), before);
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn owner_scoped_construction_rows_match_global_rows_without_foreign_outputs() {
        let recipe = ContentId::parse("fixture:building").unwrap();
        let mut view = super::super::views::WorldView::default();
        for owner in 0..2 {
            let id = u32::from(owner) + 1;
            view.units.push(crate::world::UnitState {
                unit_id: id,
                content_id: "fixture:builder".into(),
                owner_slot: owner,
                facing: None,
                pos: Vec2::new(id as f32, 1.),
            });
            view.construction.held.insert(
                id,
                vec![HeldConstruction {
                    origin: None,
                    recipe: recipe.clone(),
                    point: Vec2::new(8., 9.),
                }],
            );
            view.construction.sites.insert(
                10 + id,
                Site {
                    origin: None,
                    funding: None,
                    constructor: id,
                    owner_slot: owner,
                    recipe: recipe.clone(),
                    job: crate::JobId(u64::from(id)),
                    done: 2,
                    total: 20,
                    charged: vec![ReservationLine {
                        endpoint: Endpoint::Slot(owner),
                        resource: ContentId::parse("fixture:credits").unwrap(),
                        amount: 7,
                    }],
                    constructor_when_short: vec![],
                },
            );
        }
        // A removed constructor's held list produces no row, as in the original writer.
        view.construction.held.insert(
            99,
            vec![HeldConstruction {
                origin: None,
                recipe,
                point: Vec2::ZERO,
            }],
        );
        for slot in [SlotId(0), SlotId(1)] {
            assert_eq!(
                progress_rows_for_slot(&view, slot).collect::<Vec<_>>(),
                progress_rows(&view)
                    .into_iter()
                    .filter(|r| r.owner_slot == slot.0)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                queue_rows_for_slot(&view, slot).collect::<Vec<_>>(),
                queue_rows(&view)
                    .into_iter()
                    .filter(|r| r.owner_slot == slot.0)
                    .collect::<Vec<_>>()
            );
        }
    }
}
