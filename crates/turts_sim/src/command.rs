//! Command queue, validation and results.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};

use crate::economy::{
    EconomyOrders,
    production::{cancel_production, queue_production},
};
use crate::registry::UnitTypes;
use crate::slots::SlotId;
use crate::world::UnitType;
use crate::world::{MapBounds, MoveTarget, Owner, UnitId};

/// The most units one command may list.
pub const MAX_UNITS_PER_COMMAND: usize = 4096;

/// A request from one slot. `seq` is the sender's number for the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub slot: SlotId,
    pub seq: u32,
    pub kind: CommandKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandKind {
    MoveTo {
        unit_ids: Vec<u32>,
        #[serde(with = "vec2_bits")]
        target: Vec2,
    },
    Gather {
        unit_id: u32,
        node_id: crate::ids::NodeId,
    },
    Produce {
        producer_id: u32,
        recipe_id: turts_mod_api::ContentId,
    },
    CancelProduction {
        producer_id: u32,
        job_id: crate::ids::JobId,
    },
    Remove {
        unit_id: u32,
    },
    SetEnabled {
        unit_id: u32,
        enabled: bool,
    },
    Order(Box<crate::orders::OrderCommand>),
}

/// Commands that wait for the next tick.
#[derive(Resource, Default, Debug)]
pub struct CommandQueue(pub VecDeque<Command>);

/// The outcome of one command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResult {
    pub slot: SlotId,
    pub seq: u32,
    pub outcome: Result<Applied, RejectReason>,
    #[serde(default)]
    pub units: crate::orders::UnitOutcomes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    pub job_id: Option<crate::ids::JobId>,
}

/// The results of the commands that the last tick drained. The next drain clears them.
#[derive(Resource, Default, Debug)]
pub struct CommandResults(pub Vec<CommandResult>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    EmptyCommand,
    TooManyUnits,
    DuplicateUnit,
    UnknownUnit,
    NotOwner,
    NotMobile,
    TargetNotFinite,
    TargetOutsideMap,
    MissingCapability,
    UnknownNode,
    IncompatibleItem,
    NodeDepleted,
    UnknownRecipe,
    RecipeNotEnabled,
    QueueFull,
    RateLimited,
    InsufficientBalance,
    InsufficientItems,
    UnknownJob,
    RefundDestinationFull,
    RefundDestinationOverflow,
    ArithmeticOverflow,
    IdExhausted,
    BadSequence,
    UnknownInventory,
    UnknownBalance,
    InvalidAmount,
    NotRemovable,
    NotToggleable,
    PrerequisitesMissing,
    AlreadyResearched,
    ResearchInProgress,
    IntegrityViolation,
    NotApplicable,
    TargetInvalid,
    TargetGone,
    OutOfRange,
    NotQueueable,
    Blocked,
    SiteOutsideMap,
    SiteOutOfArea,
    SiteBlocked,
    SiteNearBuilding,
    SiteNearResource,
    ConstructionQueueFull,
}

impl RejectReason {
    /// The stable name of the reason.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SiteOutsideMap => "site_outside_map",
            Self::SiteOutOfArea => "site_out_of_area",
            Self::SiteBlocked => "site_blocked",
            Self::SiteNearBuilding => "site_near_building",
            Self::SiteNearResource => "site_near_resource",
            Self::ConstructionQueueFull => "construction_queue_full",
            Self::EmptyCommand => "empty_command",
            Self::TooManyUnits => "too_many_units",
            Self::DuplicateUnit => "duplicate_unit",
            Self::UnknownUnit => "unknown_unit",
            Self::NotOwner => "not_owner",
            Self::NotMobile => "not_mobile",
            Self::TargetNotFinite => "target_not_finite",
            Self::TargetOutsideMap => "target_outside_map",
            Self::MissingCapability => "missing_capability",
            Self::UnknownNode => "unknown_node",
            Self::IncompatibleItem => "incompatible_item",
            Self::NodeDepleted => "node_depleted",
            Self::UnknownRecipe => "unknown_recipe",
            Self::RecipeNotEnabled => "recipe_not_enabled",
            Self::QueueFull => "queue_full",
            Self::RateLimited => "rate_limited",
            Self::InsufficientBalance => "insufficient_balance",
            Self::InsufficientItems => "insufficient_items",
            Self::UnknownJob => "unknown_job",
            Self::RefundDestinationFull => "refund_destination_full",
            Self::RefundDestinationOverflow => "refund_destination_overflow",
            Self::ArithmeticOverflow => "arithmetic_overflow",
            Self::IdExhausted => "id_exhausted",
            Self::BadSequence => "bad_sequence",
            Self::UnknownInventory => "unknown_inventory",
            Self::UnknownBalance => "unknown_balance",
            Self::InvalidAmount => "invalid_amount",
            Self::NotRemovable => "not_removable",
            Self::NotToggleable => "not_toggleable",
            Self::PrerequisitesMissing => "prerequisites_missing",
            Self::AlreadyResearched => "already_researched",
            Self::ResearchInProgress => "research_in_progress",
            Self::IntegrityViolation => "integrity_violation",
            Self::NotApplicable => "not_applicable",
            Self::TargetInvalid => "target_invalid",
            Self::TargetGone => "target_gone",
            Self::OutOfRange => "out_of_range",
            Self::NotQueueable => "not_queueable",
            Self::Blocked => "blocked",
        }
    }
}

/// Apply one admitted command synchronously. Automatic simulation phases do not run here.
pub fn apply_command(world: &mut World, command: &Command) -> CommandResult {
    if let CommandKind::MoveTo { unit_ids, target } = &command.kind
        && unit_ids.len() > 1
    {
        return crate::economy::transaction::capture_command(world, command, |world| {
            let (outcome, units) =
                apply_move_to_per_unit(world, command.slot, command.seq, unit_ids, *target);
            CommandResult {
                slot: command.slot,
                seq: command.seq,
                outcome,
                units,
            }
        });
    }
    if let CommandKind::Order(order) = &command.kind {
        let plain_move = order.behaviour == turts_mod_api::Behaviour::Move
            && order.queue == crate::orders::QueueMode::Replace
            && order.group == 0
            && order.units.len() == 1
            && matches!(&order.target, crate::orders::OrderTargetSim::Point(_))
            && (crate::orders::plain_move_has_no_prior_order_state(world, order)
                || crate::orders::plain_move_reservation_exists(world, order.units[0]));
        let plain_gather = order.behaviour == turts_mod_api::Behaviour::Gather
            && order.queue == crate::orders::QueueMode::Replace
            && order.group == 0;
        if plain_move || plain_gather {
            return crate::economy::transaction::capture_command(world, command, |world| {
                let (outcome, units) =
                    crate::orders::apply_order(world, command.slot, command.seq, order);
                CommandResult {
                    slot: command.slot,
                    seq: command.seq,
                    outcome,
                    units,
                }
            });
        }
        return crate::economy::transaction::capture_order_command(world, command, |world| {
            let (outcome, units) =
                crate::orders::apply_order(world, command.slot, command.seq, order);
            CommandResult {
                slot: command.slot,
                seq: command.seq,
                outcome,
                units,
            }
        });
    }
    crate::economy::transaction::capture_command(world, command, |world| {
        apply_command_inner(world, command)
    })
}

fn apply_command_inner(world: &mut World, command: &Command) -> CommandResult {
    let outcome = match &command.kind {
        CommandKind::MoveTo { unit_ids, target } => {
            apply_legacy_command_move_to(world, command.slot, unit_ids, *target)
                .map(|()| Applied { job_id: None })
        }
        CommandKind::Gather { unit_id, node_id } => {
            crate::economy::gather::assign_gather(world, command.slot, *unit_id, *node_id).map(
                |()| {
                    // The legacy Gather command is a Replace with no authored
                    // Order identity. Keep this reset out of shared Gather
                    // activation so queued catalog orders retain their order.
                    let entity = world
                        .iter_entities()
                        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == *unit_id))
                        .map(|entity| entity.id());
                    if let Some(entity) = entity {
                        crate::orders::writer::set_unit_orders(
                            world,
                            entity,
                            crate::UnitOrders::default(),
                        );
                    }
                    Applied { job_id: None }
                },
            )
        }
        CommandKind::Produce {
            producer_id,
            recipe_id,
        } => queue_production(world, command.slot, *producer_id, recipe_id.clone()).map(|job_id| {
            Applied {
                job_id: Some(job_id),
            }
        }),
        CommandKind::CancelProduction {
            producer_id,
            job_id,
        } => cancel_production(world, command.slot, *producer_id, *job_id)
            .map(|()| Applied { job_id: None }),
        CommandKind::Remove { unit_id } => {
            let result = crate::command_m3::remove(world, command.slot, *unit_id);
            if result.is_ok()
                && let Some(mut identities) =
                    world.get_resource_mut::<crate::orders::PlainMoveOrderIdentities>()
            {
                identities.0.remove(unit_id);
            }
            result.map(|()| Applied { job_id: None })
        }
        CommandKind::SetEnabled { unit_id, enabled } => {
            crate::command_m3::set_enabled(world, command.slot, *unit_id, *enabled)
                .map(|()| Applied { job_id: None })
        }
        CommandKind::Order(_) => Err(RejectReason::NotApplicable),
    };
    CommandResult {
        slot: command.slot,
        seq: command.seq,
        outcome,
        units: crate::orders::UnitOutcomes::default(),
    }
}

pub(crate) fn apply_legacy_move_to(
    world: &mut World,
    slot: SlotId,
    unit_ids: &[u32],
    target: Vec2,
) -> Result<(), RejectReason> {
    apply_move_to(world, slot, unit_ids, target)
}

fn apply_move_to_per_unit(
    world: &mut World,
    slot: SlotId,
    seq: u32,
    unit_ids: &[u32],
    target: Vec2,
) -> (Result<Applied, RejectReason>, crate::orders::UnitOutcomes) {
    use crate::orders::UnitOutcomes;
    if unit_ids.is_empty() {
        return (Err(RejectReason::EmptyCommand), UnitOutcomes::default());
    }
    if unit_ids.len() > MAX_UNITS_PER_COMMAND {
        return (Err(RejectReason::TooManyUnits), UnitOutcomes::default());
    }
    let mut seen = HashSet::with_capacity(unit_ids.len());
    if unit_ids.iter().any(|id| !seen.insert(*id)) {
        return (Err(RejectReason::DuplicateUnit), UnitOutcomes::default());
    }
    let bounds = world.get_resource::<MapBounds>().copied();
    let types = world.resource::<UnitTypes>();
    let index: HashMap<u32, (Entity, SlotId, bool, f32)> = world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?;
            let owner = entity.get::<Owner>()?;
            let unit_type = entity.get::<UnitType>()?;
            Some((
                id.0,
                (
                    entity.id(),
                    owner.0,
                    types
                        .get(unit_type.0)
                        .is_some_and(|entry| entry.movement.is_some()),
                    types.get(unit_type.0).map_or(0.5, |entry| entry.radius),
                ),
            ))
        })
        .collect();
    if !target.is_finite() {
        return (Err(RejectReason::TargetNotFinite), UnitOutcomes::default());
    }
    let radius = unit_ids
        .iter()
        .filter_map(|id| index.get(id).map(|(_, _, _, radius)| *radius))
        .fold(0.0_f32, f32::max);
    let target = bounds.map_or(target, |bounds| {
        Vec2::new(
            target.x.clamp(radius, (bounds.size.x - radius).max(radius)),
            target.y.clamp(radius, (bounds.size.y - radius).max(radius)),
        )
    });
    let mut outcomes = UnitOutcomes::default();
    for unit_id in unit_ids {
        let result = match index.get(unit_id) {
            None => Err(RejectReason::UnknownUnit),
            Some((_, owner, _, _)) if *owner != slot => Err(RejectReason::NotOwner),
            Some((_, _, false, _)) => Err(RejectReason::NotApplicable),
            Some(_) => Ok(()),
        };
        match result {
            Ok(()) => outcomes.accepted.push(*unit_id),
            Err(reason) => outcomes.refused.push((*unit_id, reason)),
        }
    }
    if !outcomes.accepted.is_empty()
        && !crate::orders::queue_legacy_move_to_envelope(
            world,
            crate::orders::MovementEnvelopeRequest {
                slot,
                seq,
                group: 0,
                point: target,
                unit_ids: outcomes.accepted.clone(),
                order: None,
                member_orders: Default::default(),
                plain: true,
            },
        )
    {
        outcomes.refused.extend(
            outcomes
                .accepted
                .drain(..)
                .map(|unit| (unit, RejectReason::RateLimited)),
        );
    } else if !outcomes.accepted.is_empty() {
        let entity_by_unit = index
            .iter()
            .map(|(&unit_id, (entity, _, _, _))| (unit_id, *entity))
            .collect::<HashMap<_, _>>();
        crate::orders::prepare_legacy_move_to_waiters(world, &outcomes.accepted, &entity_by_unit);
    }
    let outcome = if outcomes.accepted.is_empty() {
        outcomes
            .refused
            .first()
            .map_or(Err(RejectReason::NotApplicable), |(_, reason)| Err(*reason))
    } else {
        Ok(Applied { job_id: None })
    };
    (outcome, outcomes)
}

pub(crate) fn apply_legacy_unit(
    world: &mut World,
    slot: SlotId,
    kind: CommandKind,
) -> Result<Applied, RejectReason> {
    apply_command_inner(world, &Command { slot, seq: 0, kind }).outcome
}

/// Freeze the admitted batch, then apply commands in its original order.
pub fn drain_commands(world: &mut World) {
    if world.contains_resource::<crate::economy::events::MatchStopped>() {
        world.resource_mut::<CommandQueue>().0.clear();
        world.resource_mut::<CommandResults>().0.clear();
        return;
    }
    let batch = std::mem::take(&mut world.resource_mut::<CommandQueue>().0);
    world.resource_mut::<CommandResults>().0.clear();
    for command in batch {
        let result = apply_command(world, &command);
        world.resource_mut::<CommandResults>().0.push(result);
    }
}

fn apply_move_to(
    world: &mut World,
    slot: SlotId,
    unit_ids: &[u32],
    target: Vec2,
) -> Result<(), RejectReason> {
    apply_move_to_with_legacy_replace(world, slot, unit_ids, target, false)
}

fn apply_legacy_command_move_to(
    world: &mut World,
    slot: SlotId,
    unit_ids: &[u32],
    target: Vec2,
) -> Result<(), RejectReason> {
    apply_move_to_with_legacy_replace(world, slot, unit_ids, target, true)
}

fn apply_move_to_with_legacy_replace(
    world: &mut World,
    slot: SlotId,
    unit_ids: &[u32],
    target: Vec2,
    legacy_replace: bool,
) -> Result<(), RejectReason> {
    let bounds = world.get_resource::<MapBounds>().copied();
    let types = world.resource::<UnitTypes>();
    let index: HashMap<u32, (Entity, SlotId, bool)> = world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?;
            let owner = entity.get::<Owner>()?;
            let unit_type = entity.get::<UnitType>()?;
            Some((
                id.0,
                (
                    entity.id(),
                    owner.0,
                    types.get(unit_type.0).is_some_and(|t| t.movement.is_some()),
                ),
            ))
        })
        .collect();
    let entities = validate_move_to(slot, unit_ids, target, &index, bounds.as_ref())?;
    if !target.is_finite() {
        return Err(RejectReason::TargetNotFinite);
    }
    let moves: Vec<_> = entities
        .into_iter()
        .map(|entity| {
            let radius = world
                .entity(entity)
                .get::<UnitType>()
                .and_then(|kind| types.get(kind.0))
                .map_or(0.5, |entry| entry.radius);
            let clamped = bounds.map_or(target, |bounds| {
                Vec2::new(
                    target.x.clamp(radius, (bounds.size.x - radius).max(radius)),
                    target.y.clamp(radius, (bounds.size.y - radius).max(radius)),
                )
            });
            (entity, clamped)
        })
        .collect();
    for (entity, target) in moves {
        if legacy_replace {
            let unit_id = world.entity(entity).get::<UnitId>().map(|id| id.0).unwrap();
            crate::orders::reset_legacy_move_to_unit(world, slot, entity, unit_id);
        }
        let mut movement = world.entity_mut(entity);
        movement.remove::<crate::movement::PlainMoveProgress>();
        movement.insert(MoveTarget(target));
    }
    for id in unit_ids {
        crate::orders::forget_plain_move_identity(world, *id);
    }
    if let Some(mut orders) = world.get_resource_mut::<EconomyOrders>() {
        for id in unit_ids {
            if let Some(order) = orders.0.get_mut(id) {
                order.node = None;
                order.deposit = None;
                order.receiver = None;
                order.stage = crate::economy::gather::GatherStage::Idle;
                order.carry = crate::economy::gather::RateCarry::default();
            }
        }
    }
    Ok(())
}

/// Check order: list empty; more than 4096; duplicate id; every id exists; every unit owned by
/// the sender; target finite. Finite target points are clamped by footprint radius.
fn validate_move_to(
    slot: SlotId,
    unit_ids: &[u32],
    target: Vec2,
    index: &HashMap<u32, (Entity, SlotId, bool)>,
    bounds: Option<&MapBounds>,
) -> Result<Vec<Entity>, RejectReason> {
    if unit_ids.is_empty() {
        return Err(RejectReason::EmptyCommand);
    }
    if unit_ids.len() > MAX_UNITS_PER_COMMAND {
        return Err(RejectReason::TooManyUnits);
    }
    let mut seen = HashSet::with_capacity(unit_ids.len());
    if !unit_ids.iter().all(|id| seen.insert(*id)) {
        return Err(RejectReason::DuplicateUnit);
    }
    let found: Vec<(Entity, SlotId, bool)> = unit_ids
        .iter()
        .map(|id| index.get(id).copied().ok_or(RejectReason::UnknownUnit))
        .collect::<Result<_, _>>()?;
    if found.iter().any(|(_, owner, _)| *owner != slot) {
        return Err(RejectReason::NotOwner);
    }
    if found.iter().any(|(_, _, mobile)| !mobile) {
        return Err(RejectReason::NotMobile);
    }
    if !target.is_finite() {
        return Err(RejectReason::TargetNotFinite);
    }
    let _ = bounds;
    Ok(found.into_iter().map(|(entity, _, _)| entity).collect())
}

pub(crate) mod vec2_bits {
    use bevy_math::Vec2;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(value: &Vec2, serializer: S) -> Result<S::Ok, S::Error> {
        [value.x.to_bits(), value.y.to_bits()].serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec2, D::Error> {
        let [x, y] = <[u32; 2]>::deserialize(deserializer)?;
        Ok(Vec2::new(f32::from_bits(x), f32::from_bits(y)))
    }
}
