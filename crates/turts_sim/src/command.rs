//! Command queue, validation and results.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy_ecs::prelude::*;
use bevy_math::Vec2;

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
    MoveTo { unit_ids: Vec<u32>, target: Vec2 },
}

/// Commands that wait for the next tick.
#[derive(Resource, Default, Debug)]
pub struct CommandQueue(pub VecDeque<Command>);

/// The outcome of one command.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandResult {
    pub slot: SlotId,
    pub seq: u32,
    pub outcome: Result<(), RejectReason>,
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

/// FixedPreUpdate: validate every queued command, in order. A command that passes puts a
/// `MoveTarget` on each listed unit. A command that fails changes nothing.
pub(crate) fn drain_commands(
    mut queue: ResMut<CommandQueue>,
    mut results: ResMut<CommandResults>,
    bounds: Option<Res<MapBounds>>,
    units: Query<(Entity, &UnitId, &Owner, &UnitType)>,
    types: Res<UnitTypes>,
    mut commands: Commands,
) {
    results.0.clear();
    if queue.0.is_empty() {
        return;
    }
    let index: HashMap<u32, (Entity, SlotId, bool)> = units
        .iter()
        .map(|(entity, id, owner, unit_type)| {
            (
                id.0,
                (
                    entity,
                    owner.0,
                    types
                        .get(unit_type.0)
                        .is_some_and(|entry| entry.movement.is_some()),
                ),
            )
        })
        .collect();
    while let Some(command) = queue.0.pop_front() {
        let outcome = match &command.kind {
            CommandKind::MoveTo { unit_ids, target } => {
                validate_move_to(command.slot, unit_ids, *target, &index, bounds.as_deref()).map(
                    |entities| {
                        for entity in entities {
                            commands.entity(entity).insert(MoveTarget(*target));
                        }
                    },
                )
            }
        };
        results.0.push(CommandResult {
            slot: command.slot,
            seq: command.seq,
            outcome,
        });
    }
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
