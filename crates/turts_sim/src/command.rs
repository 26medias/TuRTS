//! Command queue, validation and results.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy_ecs::prelude::*;
use bevy_math::Vec2;

use crate::slots::SlotId;
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
    TargetNotFinite,
    TargetOutsideMap,
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
            Self::TargetNotFinite => "target_not_finite",
            Self::TargetOutsideMap => "target_outside_map",
        }
    }
}

/// FixedPreUpdate: validate every queued command, in order. A command that passes puts a
/// `MoveTarget` on each listed unit. A command that fails changes nothing.
pub(crate) fn drain_commands(
    mut queue: ResMut<CommandQueue>,
    mut results: ResMut<CommandResults>,
    bounds: Option<Res<MapBounds>>,
    units: Query<(Entity, &UnitId, &Owner)>,
    mut commands: Commands,
) {
    results.0.clear();
    if queue.0.is_empty() {
        return;
    }
    let index: HashMap<u32, (Entity, SlotId)> = units
        .iter()
        .map(|(entity, id, owner)| (id.0, (entity, owner.0)))
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
    index: &HashMap<u32, (Entity, SlotId)>,
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
    let found: Vec<(Entity, SlotId)> = unit_ids
        .iter()
        .map(|id| index.get(id).copied().ok_or(RejectReason::UnknownUnit))
        .collect::<Result<_, _>>()?;
    if found.iter().any(|(_, owner)| *owner != slot) {
        return Err(RejectReason::NotOwner);
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
    Ok(found.into_iter().map(|(entity, _)| entity).collect())
}
