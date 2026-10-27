//! Command queue, validation and results.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy_ecs::prelude::*;
use bevy_math::Vec2;

use crate::economy::{EconomyOrders, production::queue_production};
use crate::registry::UnitTypes;
use crate::slots::SlotId;
use crate::world::UnitType;
use crate::world::{MapBounds, MoveTarget, Owner, UnitId};

/// The most units one command may list.
const MAX_UNITS_PER_COMMAND: usize = 256;

/// A request from one slot. `seq` is the sender's number for the request.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub slot: SlotId,
    pub seq: u32,
    pub kind: CommandKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandKind {
    MoveTo {
        unit_ids: Vec<u32>,
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
}

/// Commands that wait for the next tick.
#[derive(Resource, Default, Debug)]
pub struct CommandQueue(pub VecDeque<Command>);

/// The outcome of one command.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandResult {
    pub slot: SlotId,
    pub seq: u32,
    pub outcome: Result<Applied, RejectReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    pub job_id: Option<crate::ids::JobId>,
}

/// The results of the commands that the last tick drained. The next drain clears them.
#[derive(Resource, Default, Debug)]
pub struct CommandResults(pub Vec<CommandResult>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        }
    }
}

/// Apply one admitted command synchronously. Automatic simulation phases do not run here.
pub fn apply_command(world: &mut World, command: &Command) -> CommandResult {
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
        CommandKind::CancelProduction { .. } => Err(RejectReason::UnknownJob),
    };
    CommandResult {
        slot: command.slot,
        seq: command.seq,
        outcome,
    }
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
