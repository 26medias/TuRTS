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
}

impl RejectReason {
    /// The stable name of the reason.
    pub fn as_str(self) -> &'static str {
        match self {
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
            let (outcome, units) = apply_move_to_per_unit(world, command.slot, unit_ids, *target);
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
            && crate::orders::plain_move_has_no_prior_order_state(world, order);
        let plain_gather = order.behaviour == turts_mod_api::Behaviour::Gather
            && order.queue == crate::orders::QueueMode::Replace
            && order.group == 0;
        if plain_move || plain_gather {
            return crate::economy::transaction::capture_command(world, command, |world| {
                let (outcome, units) = crate::orders::apply_order(world, command.slot, order);
                CommandResult {
                    slot: command.slot,
                    seq: command.seq,
                    outcome,
                    units,
                }
            });
        }
        return crate::economy::transaction::capture_order_command(world, command, |world| {
            let (outcome, units) = crate::orders::apply_order(world, command.slot, order);
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
            apply_move_to(world, command.slot, unit_ids, *target).map(|()| Applied { job_id: None })
        }
        CommandKind::Gather { unit_id, node_id } => {
            crate::economy::gather::assign_gather(world, command.slot, *unit_id, *node_id)
                .map(|()| Applied { job_id: None })
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
        CommandKind::Remove { unit_id } => crate::command_m3::remove(world, command.slot, *unit_id)
            .map(|()| Applied { job_id: None }),
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
                    types
                        .get(unit_type.0)
                        .is_some_and(|entry| entry.movement.is_some()),
                ),
            ))
        })
        .collect();
    let inside = bounds.is_some_and(|b| {
        (0.0..=b.size.x).contains(&target.x) && (0.0..=b.size.y).contains(&target.y)
    });
    let mut outcomes = UnitOutcomes::default();
    for unit_id in unit_ids {
        let result = match index.get(unit_id) {
            None => Err(RejectReason::UnknownUnit),
            Some((_, owner, _)) if *owner != slot => Err(RejectReason::NotOwner),
            Some((_, _, false)) => Err(RejectReason::NotApplicable),
            Some(_) if !target.is_finite() => Err(RejectReason::TargetNotFinite),
            Some(_) if !inside => Err(RejectReason::TargetOutsideMap),
            Some((entity, _, _)) => {
                world.entity_mut(*entity).insert(MoveTarget(target));
                Ok(())
            }
        };
        match result {
            Ok(()) => {
                outcomes.accepted.push(*unit_id);
                if let Some(mut orders) = world.get_resource_mut::<EconomyOrders>()
                    && let Some(order) = orders.0.get_mut(unit_id)
                {
                    order.node = None;
                    order.deposit = None;
                    order.receiver = None;
                    order.stage = crate::economy::gather::GatherStage::Idle;
                    order.carry = crate::economy::gather::RateCarry::default();
                }
            }
            Err(reason) => outcomes.refused.push((*unit_id, reason)),
        }
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
    for entity in entities {
        let mut entity = world.entity_mut(entity);
        entity.insert(MoveTarget(target));
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

/// Check order: list empty; more than 256; duplicate id; every id exists; every unit owned by
/// the sender; target finite; target inside the map. Returns the entities to move.
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
    let inside = bounds.is_some_and(|b| {
        (0.0..=b.size.x).contains(&target.x) && (0.0..=b.size.y).contains(&target.y)
    });
    if !inside {
        return Err(RejectReason::TargetOutsideMap);
    }
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
