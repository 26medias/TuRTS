//! Per-unit orders and the fixed-tick order phase.

use std::collections::VecDeque;

use bevy_ecs::prelude::{Component, World};
use bevy_math::Vec2;
use serde::{Deserialize, Serialize};
use turts_mod_api::{Behaviour, ContentId};

use crate::{JobId, NodeId, SlotId};

/// The target carried by one simulation order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderTargetSim {
    Invalid,
    None,
    Point(#[serde(with = "crate::command::vec2_bits")] Vec2),
    Unit(u32),
    Node(NodeId),
    Job(JobId),
    Content(ContentId),
}

/// Whether an order replaces or appends to a unit's queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueMode {
    Replace,
    Append,
}

/// One resolved order stored on a unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub order: ContentId,
    pub behaviour: Behaviour,
    pub target: OrderTargetSim,
    pub group: u32,
}

/// Orders currently assigned to one unit.
#[derive(Component, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UnitOrders {
    pub current: Option<Order>,
    pub queue: VecDeque<Order>,
    pub last_failure: Option<OrderFailure>,
}

/// The last accepted order failure, retained for the owner-facing projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderFailure {
    pub order: ContentId,
    pub reason: crate::command::RejectReason,
    pub tick: u32,
}

/// The order request after server-side content resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderCommand {
    pub order: ContentId,
    pub behaviour: Behaviour,
    pub target_kind: turts_mod_api::TargetKind,
    /// The server-captured definition used to resolve this envelope.
    pub policy: Option<OrderPolicyAudit>,
    pub units: Vec<u32>,
    pub target: OrderTargetSim,
    pub queue: QueueMode,
    pub group: u32,
    pub allowed_units: Vec<String>,
    pub queue_depth: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderPolicyAudit {
    pub id: ContentId,
    pub behaviour: Behaviour,
    pub target: turts_mod_api::TargetKind,
    pub units: Vec<String>,
    pub queue_depth: u8,
    pub unit_capabilities: Vec<OrderUnitCapabilitiesAudit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderUnitCapabilitiesAudit {
    pub unit: String,
    pub movement: bool,
    pub weapon: bool,
    pub gatherer: bool,
    pub producer: bool,
    pub research: bool,
    pub removable: bool,
    pub toggleable: bool,
}

/// Per-unit outcome of an order envelope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitOutcomes {
    pub accepted: Vec<u32>,
    pub refused: Vec<(u32, crate::command::RejectReason)>,
}

pub(crate) fn unit_order_capable(world: &World, kind: turts_mod_api::UnitTypeId) -> bool {
    let Some(entry) = world.resource::<crate::registry::UnitTypes>().get(kind) else {
        return false;
    };
    entry.movement.is_some()
        || world
            .get_resource::<crate::bootstrap::InstalledCatalog>()
            .is_some_and(|catalog| {
                catalog
                    .0
                    .units
                    .values()
                    .any(|unit| unit.id.to_string() == entry.content_id && unit.weapon.is_some())
            })
}

fn unit_toggleable(unit: &turts_mod_api::ResolvedUnit) -> bool {
    unit.extractor.is_some()
        || unit.inserter.is_some()
        || unit.machine.is_some()
        || unit.research.is_some()
        || unit.producer.is_some()
        || unit.weapon.is_some()
}

/// Run one fixed-tick order phase.
pub(crate) fn run_orders(world: &mut World) {
    let mut unit_ids: Vec<_> = world
        .iter_entities()
        .filter_map(|entity| entity.get::<crate::world::UnitId>().map(|id| id.0))
        .collect();
    unit_ids.sort_unstable();
    for unit_id in unit_ids {
        let Some(entity) = world
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<crate::world::UnitId>()
                    .is_some_and(|id| id.0 == unit_id)
            })
            .map(|entity| entity.id())
        else {
            continue;
        };
        let current = world
            .entity(entity)
            .get::<UnitOrders>()
            .and_then(|orders| orders.current.clone());
        if current.is_none() {
            let waiting = world
                .entity(entity)
                .get::<UnitOrders>()
                .is_some_and(|orders| !orders.queue.is_empty());
            let plain_move_landed = world
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .is_none();
            if waiting && plain_move_landed {
                let slot = world
                    .entity(entity)
                    .get::<crate::world::Owner>()
                    .map_or(SlotId(0), |owner| owner.0);
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    unit_id,
                    |world| {
                        start_next_queued_order(world, slot, entity, unit_id);
                    },
                );
            }
            continue;
        }
        let current = current.unwrap();
        if current.behaviour == Behaviour::Move
            && let OrderTargetSim::Unit(target_id) = current.target
        {
            run_unit_follow_move(world, entity, unit_id, current, target_id);
            continue;
        }
        let completed_move = current.behaviour == Behaviour::Move
            && world
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .is_none();
        let depleted_gather = current.behaviour == Behaviour::Gather
            && match current.target {
                OrderTargetSim::Node(node) => world
                    .get_resource::<crate::economy::SurfaceNodes>()
                    .is_some_and(|nodes| {
                        nodes
                            .0
                            .iter()
                            .any(|record| record.id == node && record.remaining == 0)
                    }),
                _ => false,
            };
        if !completed_move && !depleted_gather {
            continue;
        }
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            let next = {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                if depleted_gather {
                    orders.last_failure = Some(OrderFailure {
                        order: current.order.clone(),
                        reason: crate::command::RejectReason::NodeDepleted,
                        tick,
                    });
                }
                orders.current = None;
                orders.queue.pop_front()
            };
            if depleted_gather {
                let _ = crate::economy::gather::interrupt_order(world, slot, unit_id);
            }
            if next.is_some() {
                if let Some(next) = next {
                    world
                        .entity_mut(entity)
                        .get_mut::<UnitOrders>()
                        .unwrap()
                        .current = Some(next);
                }
                start_next_queued_order(world, slot, entity, unit_id);
            }
        });
    }
}

fn run_unit_follow_move(
    world: &mut World,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    current: Order,
    target_id: u32,
) {
    let target = world.iter_entities().find(|candidate| {
        candidate
            .get::<crate::world::UnitId>()
            .is_some_and(|id| id.0 == target_id)
    });
    let target_position =
        target.and_then(|target| target.get::<crate::world::Position>().map(|pos| pos.0));
    let current_position = world
        .entity(entity)
        .get::<crate::world::Position>()
        .map(|pos| pos.0);
    let Some(target_position) = target_position else {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        let tick = world.resource::<crate::world::Tick>().0;
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                orders.last_failure = Some(OrderFailure {
                    order: current.order.clone(),
                    reason: crate::command::RejectReason::TargetGone,
                    tick,
                });
                orders.current = None;
            }
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            start_next_queued_order(world, slot, entity, unit_id);
        });
        return;
    };
    let Some(current_position) = current_position else {
        return;
    };
    if current_position == target_position {
        let slot = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map_or(SlotId(0), |owner| owner.0);
        crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
            {
                let mut entity_mut = world.entity_mut(entity);
                let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
                orders.current = None;
            }
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            start_next_queued_order(world, slot, entity, unit_id);
        });
    } else {
        let move_target = world
            .entity(entity)
            .get::<crate::world::MoveTarget>()
            .map(|target| target.0);
        if move_target != Some(target_position) {
            crate::economy::transaction::capture_unit_order_operation(world, unit_id, |world| {
                world
                    .entity_mut(entity)
                    .insert(crate::world::MoveTarget(target_position));
            });
        }
    }
}

fn start_next_queued_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit: u32,
) {
    loop {
        let next = {
            let mut entity_mut = world.entity_mut(entity);
            let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
            let Some(next) = orders.current.take().or_else(|| orders.queue.pop_front()) else {
                return;
            };
            orders.current = Some(next.clone());
            next
        };
        if let Err(reason) = start_order(world, slot, entity, unit, &next) {
            let tick = world.resource::<crate::world::Tick>().0;
            let mut entity_mut = world.entity_mut(entity);
            let mut orders = entity_mut.get_mut::<UnitOrders>().unwrap();
            orders.current = None;
            orders.last_failure = Some(OrderFailure {
                order: next.order,
                reason,
                tick,
            });
            continue;
        }
        break;
    }
}

fn start_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit: u32,
    order: &Order,
) -> Result<(), crate::command::RejectReason> {
    match (order.behaviour, &order.target) {
        (Behaviour::Move, OrderTargetSim::Point(point)) => {
            let target = clamp_target(world, *point);
            world
                .entity_mut(entity)
                .insert(crate::world::MoveTarget(target));
            Ok(())
        }
        (Behaviour::Move, OrderTargetSim::Unit(target_id)) => {
            let target = world
                .iter_entities()
                .find(|candidate| {
                    candidate
                        .get::<crate::world::UnitId>()
                        .is_some_and(|id| id.0 == *target_id)
                })
                .and_then(|candidate| candidate.get::<crate::world::Position>().map(|pos| pos.0))
                .ok_or(crate::command::RejectReason::TargetGone)?;
            let target = clamp_target(world, target);
            world
                .entity_mut(entity)
                .insert(crate::world::MoveTarget(target));
            Ok(())
        }
        (Behaviour::Hold, _) => {
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            Ok(())
        }
        (Behaviour::Gather, OrderTargetSim::Node(node)) => {
            let state = world
                .get_resource::<crate::economy::SurfaceNodes>()
                .and_then(|nodes| nodes.0.iter().find(|record| record.id == *node));
            if state.is_none() {
                return Err(crate::command::RejectReason::TargetGone);
            }
            if state.is_some_and(|record| record.remaining == 0) {
                return Err(crate::command::RejectReason::NodeDepleted);
            }
            crate::economy::gather::assign_gather(world, slot, unit, *node)
        }
        _ => Err(crate::command::RejectReason::NotApplicable),
    }
}

/// Apply one server-resolved order envelope, validating each unit independently.
pub(crate) fn apply_order(
    world: &mut World,
    slot: SlotId,
    command: &OrderCommand,
) -> (
    Result<crate::command::Applied, crate::command::RejectReason>,
    UnitOutcomes,
) {
    use crate::command::RejectReason as R;

    if command.units.is_empty() {
        return (Err(R::EmptyCommand), UnitOutcomes::default());
    }
    if command.units.len() > crate::command::MAX_UNITS_PER_COMMAND {
        return (Err(R::TooManyUnits), UnitOutcomes::default());
    }
    let mut seen = std::collections::HashSet::with_capacity(command.units.len());
    if command.units.iter().any(|unit| !seen.insert(*unit)) {
        return (Err(R::DuplicateUnit), UnitOutcomes::default());
    }

    let mut unit_ids = command.units.clone();
    unit_ids.sort_unstable();
    let mut outcomes = UnitOutcomes::default();
    let mut first_job_id = None;
    let immediate = matches!(
        command.behaviour,
        Behaviour::Produce
            | Behaviour::CancelProduction
            | Behaviour::Remove
            | Behaviour::SetEnabled(_)
    );
    for unit_id in &unit_ids {
        let Some(entity) = world
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<crate::world::UnitId>()
                    .is_some_and(|id| id.0 == *unit_id)
            })
            .map(|entity| entity.id())
        else {
            outcomes.refused.push((*unit_id, R::UnknownUnit));
            continue;
        };
        let owner = world
            .entity(entity)
            .get::<crate::world::Owner>()
            .map(|owner| owner.0);
        if owner != Some(slot) {
            outcomes.refused.push((*unit_id, R::NotOwner));
            continue;
        }
        let Some(unit_type) = world
            .entity(entity)
            .get::<crate::world::UnitType>()
            .map(|kind| kind.0)
        else {
            outcomes.refused.push((*unit_id, R::NotApplicable));
            continue;
        };
        let unit_id_text = world
            .resource::<crate::registry::UnitTypes>()
            .get(unit_type)
            .map(|entry| entry.content_id.clone());
        let registry_movement = world
            .resource::<crate::registry::UnitTypes>()
            .get(unit_type)
            .is_some_and(|entry| entry.movement.is_some());
        let definition = world
            .get_resource::<crate::bootstrap::InstalledCatalog>()
            .and_then(|catalog| {
                let content_id = unit_id_text.as_deref()?;
                catalog
                    .0
                    .units
                    .values()
                    .find(|unit| unit.id.to_string() == content_id)
            });
        let capable = match command.behaviour {
            Behaviour::Move => registry_movement,
            Behaviour::Stop | Behaviour::Hold => {
                definition.is_some_and(|unit| unit.weapon.is_some()) || registry_movement
            }
            Behaviour::Attack => match command.target {
                OrderTargetSim::Point(_) => registry_movement,
                OrderTargetSim::Unit(_) => definition.is_some_and(|unit| unit.weapon.is_some()),
                _ => false,
            },
            Behaviour::Gather => definition.is_some_and(|unit| unit.gatherer.is_some()),
            Behaviour::Produce => {
                definition.is_some_and(|unit| unit.producer.is_some() || unit.research.is_some())
            }
            Behaviour::CancelProduction => definition.is_some_and(|unit| {
                unit.producer.is_some() || unit.research.is_some() || unit.machine.is_some()
            }),
            Behaviour::Remove => definition
                .and_then(|unit| unit.removal.as_ref())
                .is_some_and(|removal| removal.by_command),
            Behaviour::SetEnabled(_) => definition.is_some_and(unit_toggleable),
        };
        if !capable {
            let reason = match command.behaviour {
                Behaviour::Gather | Behaviour::Produce | Behaviour::CancelProduction => {
                    R::MissingCapability
                }
                Behaviour::Remove => R::NotRemovable,
                Behaviour::SetEnabled(_) => R::NotToggleable,
                _ => R::NotApplicable,
            };
            outcomes.refused.push((*unit_id, reason));
            continue;
        }
        if !command.allowed_units.is_empty()
            && !unit_id_text
                .as_ref()
                .is_some_and(|id| command.allowed_units.iter().any(|allowed| allowed == id))
        {
            outcomes.refused.push((*unit_id, R::NotApplicable));
            continue;
        }
        if let Err(reason) = validate_target(world, command) {
            outcomes.refused.push((*unit_id, reason));
            continue;
        }
        if command.queue == QueueMode::Append && !immediate {
            let orders = world.entity(entity).get::<UnitOrders>();
            if orders.is_some_and(|orders| {
                orders.current.as_ref().is_some_and(|current| {
                    matches!(current.behaviour, Behaviour::Hold | Behaviour::Gather)
                }) || orders
                    .queue
                    .iter()
                    .any(|queued| matches!(queued.behaviour, Behaviour::Hold | Behaviour::Gather))
            }) {
                outcomes.refused.push((*unit_id, R::NotQueueable));
                continue;
            }
            let legacy_gather_active = world
                .get_resource::<crate::economy::EconomyOrders>()
                .and_then(|orders| orders.0.get(unit_id))
                .is_some_and(|order| order.node.is_some());
            if legacy_gather_active {
                outcomes.refused.push((*unit_id, R::NotQueueable));
                continue;
            }
            if orders.is_some_and(|orders| orders.queue.len() >= command.queue_depth as usize) {
                outcomes.refused.push((*unit_id, R::QueueFull));
                continue;
            }
        }

        let order = Order {
            order: command.order.clone(),
            behaviour: command.behaviour,
            target: command.target.clone(),
            group: command.group,
        };
        let plain_move = command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && command.group == 0
            && command.units.len() == 1
            && matches!(&command.target, OrderTargetSim::Point(_))
            && plain_move_has_no_prior_order_state(world, command);
        let plain_gather = command.behaviour == Behaviour::Gather
            && command.queue == QueueMode::Replace
            && command.group == 0;
        if plain_move {
            let OrderTargetSim::Point(target) = &command.target else {
                outcomes.refused.push((*unit_id, R::TargetInvalid));
                continue;
            };
            let target = clamp_target(world, *target);
            crate::command::apply_legacy_move_to(world, slot, &[*unit_id], target)
                .expect("order validation precedes legacy move application");
            outcomes.accepted.push(*unit_id);
            continue;
        }
        if command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && command.group == 0
            && command.units.len() == 1
            && matches!(&command.target, OrderTargetSim::Point(_))
        {
            let OrderTargetSim::Point(target) = &command.target else {
                outcomes.refused.push((*unit_id, R::TargetInvalid));
                continue;
            };
            crate::economy::transaction::capture_unit_order_operation(world, *unit_id, |world| {
                crate::command::apply_legacy_move_to(
                    world,
                    slot,
                    &[*unit_id],
                    clamp_target(world, *target),
                )
                .expect("order validation precedes legacy move application");
                if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                    *orders = UnitOrders::default();
                }
                clear_gather(world, *unit_id);
            });
            outcomes.accepted.push(*unit_id);
            continue;
        }
        let result = if plain_gather {
            apply_unit_order(world, slot, entity, *unit_id, order, command)
        } else {
            crate::economy::transaction::capture_unit_order_operation(world, *unit_id, |world| {
                apply_unit_order(world, slot, entity, *unit_id, order, command)
            })
        };
        match result {
            Ok(job_id) => {
                if first_job_id.is_none() {
                    first_job_id = job_id;
                }
                outcomes.accepted.push(*unit_id)
            }
            Err(reason) => outcomes.refused.push((*unit_id, reason)),
        }
    }

    let outcome = if outcomes.accepted.is_empty() {
        outcomes
            .refused
            .first()
            .map_or(Err(R::NotApplicable), |(_, reason)| Err(*reason))
    } else {
        Ok(crate::command::Applied {
            job_id: first_job_id,
        })
    };
    (outcome, outcomes)
}

fn validate_target(
    world: &World,
    command: &OrderCommand,
) -> Result<(), crate::command::RejectReason> {
    use crate::command::RejectReason as R;
    let target_kind_accepts = match (&command.target_kind, &command.target) {
        (turts_mod_api::TargetKind::Point, OrderTargetSim::Point(_))
        | (turts_mod_api::TargetKind::Unit, OrderTargetSim::Unit(_))
        | (turts_mod_api::TargetKind::PointOrUnit, OrderTargetSim::Point(_))
        | (turts_mod_api::TargetKind::PointOrUnit, OrderTargetSim::Unit(_))
        | (turts_mod_api::TargetKind::Node, OrderTargetSim::Node(_))
        | (turts_mod_api::TargetKind::Job, OrderTargetSim::Job(_))
        | (turts_mod_api::TargetKind::Content, OrderTargetSim::Content(_))
        | (turts_mod_api::TargetKind::None, OrderTargetSim::None) => true,
        (turts_mod_api::TargetKind::Content, OrderTargetSim::Invalid)
            if command.behaviour == Behaviour::Produce =>
        {
            true
        }
        _ => false,
    };
    if !target_kind_accepts {
        return Err(R::TargetInvalid);
    }
    match (&command.behaviour, &command.target) {
        (Behaviour::Move, OrderTargetSim::Point(point)) if point.is_finite() => Ok(()),
        (Behaviour::Move, OrderTargetSim::Point(_)) => Err(R::TargetNotFinite),
        (Behaviour::Move, OrderTargetSim::Unit(unit)) => {
            if world.iter_entities().any(|entity| {
                entity
                    .get::<crate::world::UnitId>()
                    .is_some_and(|id| id.0 == *unit)
            }) {
                Ok(())
            } else {
                Err(R::TargetGone)
            }
        }
        (Behaviour::Move, _) => Err(R::TargetInvalid),
        (Behaviour::Stop | Behaviour::Hold, OrderTargetSim::None) => Ok(()),
        (Behaviour::Stop | Behaviour::Hold, _) => Err(R::TargetInvalid),
        (Behaviour::Gather, OrderTargetSim::Node(node)) => {
            if world
                .get_resource::<crate::economy::SurfaceNodes>()
                .is_some_and(|nodes| nodes.0.iter().any(|record| record.id == *node))
            {
                Ok(())
            } else {
                Err(R::UnknownNode)
            }
        }
        (Behaviour::Produce, OrderTargetSim::Content(_))
        | (Behaviour::CancelProduction, OrderTargetSim::Job(_))
        | (Behaviour::Remove, OrderTargetSim::None)
        | (Behaviour::SetEnabled(_), OrderTargetSim::None) => Ok(()),
        (Behaviour::Produce, OrderTargetSim::Invalid) => Err(R::UnknownRecipe),
        (Behaviour::Attack, OrderTargetSim::Unit(unit)) => {
            if world.iter_entities().any(|entity| {
                entity
                    .get::<crate::world::UnitId>()
                    .is_some_and(|id| id.0 == *unit)
            }) {
                Ok(())
            } else {
                Err(R::TargetGone)
            }
        }
        (Behaviour::Attack, OrderTargetSim::Point(point)) if point.is_finite() => Ok(()),
        (Behaviour::Attack, OrderTargetSim::Point(_)) => Err(R::TargetNotFinite),
        _ => Err(R::TargetInvalid),
    }
}

pub(crate) fn plain_move_has_no_prior_order_state(world: &World, command: &OrderCommand) -> bool {
    command.units.len() == 1 && unit_orders_empty(world, command.units[0])
}

fn unit_orders_empty(world: &World, unit_id: u32) -> bool {
    let entity = world.iter_entities().find(|entity| {
        entity
            .get::<crate::world::UnitId>()
            .is_some_and(|id| id.0 == unit_id)
    });
    let stored_empty = entity
        .and_then(|entity| entity.get::<UnitOrders>())
        .is_none_or(|orders| {
            orders.current.is_none() && orders.queue.is_empty() && orders.last_failure.is_none()
        });
    let gather_empty = world
        .get_resource::<crate::economy::EconomyOrders>()
        .and_then(|orders| orders.0.get(&unit_id))
        .is_none_or(|order| order.node.is_none());
    stored_empty && gather_empty
}

fn clamp_target(world: &World, target: Vec2) -> Vec2 {
    world
        .get_resource::<crate::world::MapBounds>()
        .map_or(target, |bounds| {
            Vec2::new(
                target.x.clamp(0.0, bounds.size.x),
                target.y.clamp(0.0, bounds.size.y),
            )
        })
}

fn apply_unit_order(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    order: Order,
    command: &OrderCommand,
) -> Result<Option<crate::JobId>, crate::command::RejectReason> {
    use crate::command::{CommandKind, RejectReason as R};
    match order.behaviour {
        Behaviour::Move => {
            let target = match &order.target {
                OrderTargetSim::Point(target) => clamp_target(world, *target),
                OrderTargetSim::Unit(target_id) => world
                    .iter_entities()
                    .find(|candidate| {
                        candidate
                            .get::<crate::world::UnitId>()
                            .is_some_and(|id| id.0 == *target_id)
                    })
                    .and_then(|candidate| {
                        candidate.get::<crate::world::Position>().map(|pos| pos.0)
                    })
                    .ok_or(R::TargetGone)?,
                _ => return Err(R::TargetInvalid),
            };
            let starts_now = if command.queue == QueueMode::Append {
                let existing_move = world
                    .entity(entity)
                    .get::<crate::world::MoveTarget>()
                    .is_some();
                let mut entity_mut = world.entity_mut(entity);
                if let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() {
                    if orders.current.is_none() && existing_move {
                        orders.queue.push_back(order.clone());
                        false
                    } else if orders.current.is_none() {
                        orders.current = Some(order.clone());
                        true
                    } else {
                        orders.queue.push_back(order.clone());
                        false
                    }
                } else {
                    world.entity_mut(entity).insert(UnitOrders {
                        current: Some(order.clone()),
                        ..Default::default()
                    });
                    true
                }
            } else {
                world.entity_mut(entity).insert(UnitOrders {
                    current: Some(order),
                    ..Default::default()
                });
                true
            };
            if starts_now {
                world
                    .entity_mut(entity)
                    .insert(crate::world::MoveTarget(target));
            }
            clear_gather(world, unit_id);
            Ok(None)
        }
        Behaviour::Stop => {
            world.entity_mut(entity).insert(UnitOrders::default());
            world
                .entity_mut(entity)
                .remove::<crate::world::MoveTarget>();
            clear_gather(world, unit_id);
            Ok(None)
        }
        Behaviour::Hold => {
            if command.queue == QueueMode::Append {
                let mut entity_mut = world.entity_mut(entity);
                let Some(mut orders) = entity_mut.get_mut::<UnitOrders>() else {
                    return Err(R::NotQueueable);
                };
                orders.queue.push_back(order);
            } else {
                world
                    .entity_mut(entity)
                    .remove::<crate::world::MoveTarget>();
                world.entity_mut(entity).insert(UnitOrders {
                    current: Some(order),
                    ..Default::default()
                });
            }
            clear_gather(world, unit_id);
            Ok(None)
        }
        Behaviour::Gather => {
            let OrderTargetSim::Node(node) = order.target else {
                return Err(R::TargetInvalid);
            };
            let depleted = world
                .get_resource::<crate::economy::SurfaceNodes>()
                .and_then(|nodes| nodes.0.iter().find(|record| record.id == node))
                .is_some_and(|record| record.remaining == 0);
            if command.queue == QueueMode::Append {
                let no_current = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .is_none_or(|orders| orders.current.is_none());
                let moving_from_legacy_order = world
                    .entity(entity)
                    .get::<crate::world::MoveTarget>()
                    .is_some();
                if no_current && !moving_from_legacy_order {
                    if depleted {
                        world
                            .entity_mut(entity)
                            .remove::<crate::world::MoveTarget>();
                        clear_gather(world, unit_id);
                    } else {
                        crate::economy::gather::assign_gather(world, slot, unit_id, node)?;
                    }
                    world.entity_mut(entity).insert(UnitOrders {
                        current: Some(order),
                        ..Default::default()
                    });
                } else if let Some(mut orders) = world.entity_mut(entity).get_mut::<UnitOrders>() {
                    orders.queue.push_back(order);
                }
            } else {
                if depleted {
                    world
                        .entity_mut(entity)
                        .remove::<crate::world::MoveTarget>();
                    clear_gather(world, unit_id);
                } else {
                    crate::economy::gather::assign_gather(world, slot, unit_id, node)?;
                }
                world.entity_mut(entity).insert(UnitOrders {
                    current: Some(order),
                    ..Default::default()
                });
            }
            Ok(None)
        }
        Behaviour::Produce => {
            let OrderTargetSim::Content(recipe_id) = order.target else {
                return Err(R::TargetInvalid);
            };
            crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::Produce {
                    producer_id: unit_id,
                    recipe_id,
                },
            )
            .map(|applied| applied.job_id)
        }
        Behaviour::CancelProduction => {
            let OrderTargetSim::Job(job_id) = order.target else {
                return Err(R::TargetInvalid);
            };
            crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::CancelProduction {
                    producer_id: unit_id,
                    job_id,
                },
            )
            .map(|applied| applied.job_id)
        }
        Behaviour::Remove => {
            crate::command::apply_legacy_unit(world, slot, CommandKind::Remove { unit_id })
                .map(|applied| applied.job_id)
        }
        Behaviour::SetEnabled(enabled) => crate::command::apply_legacy_unit(
            world,
            slot,
            CommandKind::SetEnabled { unit_id, enabled },
        )
        .map(|applied| applied.job_id),
        Behaviour::Attack => Err(R::NotApplicable),
    }
}

fn clear_gather(world: &mut World, unit_id: u32) {
    if let Some(mut orders) = world.get_resource_mut::<crate::economy::EconomyOrders>()
        && let Some(order) = orders.0.get_mut(&unit_id)
    {
        order.node = None;
        order.deposit = None;
        order.receiver = None;
        order.stage = crate::economy::gather::GatherStage::Idle;
        order.carry = crate::economy::gather::RateCarry::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{Footprint, GathererDef, PackageId, ProducerDef, UnitDef, UnitRegistry};

    fn gatherer_app() -> (bevy_app::App, bevy_ecs::entity::Entity) {
        let mut app = crate::headless_app(20);
        let id = ContentId::parse("test:worker").unwrap();
        let kind = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &PackageId("test".into()),
                UnitDef {
                    id: "worker".into(),
                    name: "Worker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: Some(GathererDef {
                        items: vec![],
                        gather_per_second: 1,
                        unload_per_second: 1,
                        interaction_range: 1.0,
                    }),
                    drop_off: None,
                    producer: Some(ProducerDef {
                        recipes: vec![],
                        queue_capacity: 1,
                        spawn_offsets: vec![],
                    }),
                    footprint: Footprint::Circle { radius: 0.5 },
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
            )
            .unwrap();
        let mut catalog = turts_mod_api::ContentCatalog {
            resources: Default::default(),
            units: Default::default(),
            nodes: Default::default(),
            production: Default::default(),
            recipes: Default::default(),
            technologies: Default::default(),
            orders: Default::default(),
            order_queue_depths: Default::default(),
            source_files: Default::default(),
        };
        catalog.units.insert(
            id.clone(),
            turts_mod_api::ResolvedUnit {
                id: id.clone(),
                name: "Worker".into(),
                footprint: Footprint::Circle { radius: 0.5 },
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: Some(turts_mod_api::ResolvedGatherer {
                    items: vec![],
                    gather_per_second: 1,
                    unload_per_second: 1,
                    interaction_range: 1.0,
                }),
                drop_off: None,
                producer: Some(turts_mod_api::ResolvedProducer {
                    recipes: vec![],
                    queue_capacity: 1,
                    spawn_offsets: vec![],
                }),
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
        );
        let node_id = ContentId::parse("test:ore").unwrap();
        catalog.nodes.insert(
            node_id.clone(),
            turts_mod_api::ResolvedNode {
                id: node_id,
                name: "Ore".into(),
                item: ContentId::parse("test:iron").unwrap(),
                radius: 1.0,
            },
        );
        app.world_mut()
            .insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        let entity = app
            .world_mut()
            .spawn((
                crate::UnitId(1),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(8.0, 8.0)),
                crate::world::MoveTarget(Vec2::new(30.0, 30.0)),
                UnitOrders::default(),
            ))
            .id();
        app.world_mut()
            .insert_resource(crate::economy::SurfaceNodes(vec![
                crate::economy::NodeRecord {
                    id: NodeId(1),
                    content_id: ContentId::parse("test:ore").unwrap(),
                    position: crate::Position(Vec2::new(10.0, 8.0)),
                    remaining: 1,
                },
            ]));
        (app, entity)
    }

    #[test]
    fn remove_capability_requires_by_command_before_filter_and_target_checks() {
        let (mut app, entity) = gatherer_app();
        let worker_id = ContentId::parse("test:worker").unwrap();
        let catalog = &mut app
            .world_mut()
            .resource_mut::<crate::bootstrap::InstalledCatalog>()
            .0;
        catalog.units.get_mut(&worker_id).unwrap().removal = Some(turts_mod_api::RemovalDef {
            by_command: false,
            contents: turts_mod_api::ContentsPolicy::Destroy,
        });

        let command = OrderCommand {
            order: ContentId::parse("test:remove").unwrap(),
            behaviour: Behaviour::Remove,
            target_kind: turts_mod_api::TargetKind::None,
            policy: None,
            units: vec![app.world().entity(entity).get::<crate::UnitId>().unwrap().0],
            target: OrderTargetSim::Point(Vec2::new(8.0, 8.0)),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec!["other:unit".into()],
            queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
        };
        let (_, outcomes) = apply_order(app.world_mut(), SlotId(0), &command);
        assert_eq!(
            outcomes.refused,
            vec![(1, crate::command::RejectReason::NotRemovable)],
            "a disabled removal capability wins over custom filter and target failures"
        );
    }

    #[test]
    fn append_gather_waits_behind_a_legacy_plain_move() {
        let (mut app, entity) = gatherer_app();
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:gather").unwrap(),
                behaviour: Behaviour::Gather,
                target_kind: turts_mod_api::TargetKind::Node,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Node(NodeId(1)),
                queue: QueueMode::Append,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1]);
        let entity_ref = app.world().entity(entity);
        assert_eq!(
            entity_ref.get::<crate::world::MoveTarget>().unwrap().0,
            Vec2::new(30.0, 30.0)
        );
        let orders = entity_ref.get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert_eq!(orders.queue.len(), 1);
        assert_eq!(orders.queue[0].behaviour, Behaviour::Gather);
    }

    #[test]
    fn rejected_incompatible_gather_preserves_current_order_and_queue() {
        let (mut app, entity) = gatherer_app();
        let before_order = Order {
            order: ContentId::parse("test:hold").unwrap(),
            behaviour: Behaviour::Hold,
            target: OrderTargetSim::None,
            group: 0,
        };
        let queued = Order {
            order: ContentId::parse("test:move").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
            group: 0,
        };
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(before_order),
            queue: VecDeque::from([queued]),
            last_failure: None,
        });
        let before = app
            .world()
            .entity(entity)
            .get::<UnitOrders>()
            .unwrap()
            .clone();
        app.world_mut().resource_mut::<crate::AuditTrail>().enable();
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:gather").unwrap(),
                behaviour: Behaviour::Gather,
                target_kind: turts_mod_api::TargetKind::Node,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Node(NodeId(1)),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(
            result.units.refused,
            vec![(1, crate::RejectReason::IncompatibleItem)]
        );
        assert_eq!(
            app.world().entity(entity).get::<UnitOrders>().unwrap(),
            &before
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .unwrap()
                .0,
            Vec2::new(30.0, 30.0)
        );
        let trail = app.world().resource::<crate::AuditTrail>();
        assert_eq!(trail.commands.len(), 1);
        assert_eq!(trail.operations.len(), 1);
        assert_eq!(trail.operations[0].before, trail.operations[0].after);
    }

    #[test]
    fn gather_on_an_existing_empty_node_is_accepted_then_fails_as_depleted() {
        let (mut app, entity) = gatherer_app();
        app.world_mut()
            .resource_mut::<crate::economy::SurfaceNodes>()
            .0[0]
            .remaining = 0;
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:gather").unwrap(),
                behaviour: Behaviour::Gather,
                target_kind: turts_mod_api::TargetKind::Node,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Node(NodeId(1)),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1]);
        assert!(
            app.world()
                .entity(entity)
                .get::<UnitOrders>()
                .unwrap()
                .current
                .is_some()
        );
        app.world_mut().resource_mut::<crate::Tick>().0 = 8;
        run_orders(app.world_mut());
        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert_eq!(
            orders.last_failure.as_ref().unwrap().reason,
            crate::RejectReason::NodeDepleted
        );
        assert_eq!(orders.last_failure.as_ref().unwrap().tick, 8);
    }

    #[test]
    fn immediate_cancel_ignores_unit_order_queue_and_preserves_it_on_failure() {
        let (mut app, entity) = gatherer_app();
        let existing = UnitOrders {
            current: Some(Order {
                order: ContentId::parse("test:hold").unwrap(),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 0,
            }),
            queue: VecDeque::from([Order {
                order: ContentId::parse("test:move").unwrap(),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(Vec2::new(40.0, 40.0)),
                group: 0,
            }]),
            last_failure: None,
        };
        app.world_mut().entity_mut(entity).insert(existing.clone());
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:cancel").unwrap(),
                behaviour: Behaviour::CancelProduction,
                target_kind: turts_mod_api::TargetKind::Job,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Job(crate::JobId(999)),
                queue: QueueMode::Append,
                group: 0,
                allowed_units: vec![],
                queue_depth: 1,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(
            result.units.refused,
            vec![(1, crate::RejectReason::UnknownJob)]
        );
        assert_eq!(
            app.world().entity(entity).get::<UnitOrders>().unwrap(),
            &existing
        );
    }

    #[test]
    fn set_enabled_keeps_all_legacy_toggleable_capability_families() {
        let (app, _) = gatherer_app();
        let id = ContentId::parse("test:worker").unwrap();
        let base = app
            .world()
            .resource::<crate::bootstrap::InstalledCatalog>()
            .0
            .units[&id]
            .clone();
        let mut variants = Vec::new();
        let mut unit = base.clone();
        unit.extractor = Some(turts_mod_api::ExtractorDef {
            per_second: 1,
            interaction_range: 1.0,
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.inserter = Some(turts_mod_api::InserterDef {
            reach: 1.0,
            swing_ms: 1,
            return_ms: 1,
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.machine = Some(turts_mod_api::ResolvedMachine {
            recipe: ContentId::parse("test:recipe").unwrap(),
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.research = Some(turts_mod_api::ResolvedResearchStation {
            technologies: vec![],
            queue_capacity: 1,
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.producer = Some(turts_mod_api::ResolvedProducer {
            recipes: vec![],
            queue_capacity: 1,
            spawn_offsets: vec![],
        });
        variants.push(unit);
        let mut unit = base.clone();
        unit.weapon = Some(turts_mod_api::ResolvedWeapon {
            range: 1.0,
            cooldown_ms: 1,
            damage: 1,
            ammo: None,
            targets: turts_mod_api::TargetFilter {
                slots: turts_mod_api::SlotRelation::Any,
            },
        });
        variants.push(unit);
        assert!(variants.iter().all(unit_toggleable));
        let mut conveyor_only = base;
        conveyor_only.producer = None;
        conveyor_only.gatherer = None;
        conveyor_only.conveyor = Some(turts_mod_api::ResolvedConveyor {
            slots_per_piece: 1,
            slots_per_second: 1,
            accepts: vec![],
        });
        assert!(!unit_toggleable(&conveyor_only));
    }

    #[test]
    fn immediate_produce_returns_the_first_job_in_unit_id_order() {
        let (mut app, _) = gatherer_app();
        let producer = ContentId::parse("test:worker").unwrap();
        let recipe = ContentId::parse("test:recipe").unwrap();
        let mut catalog = app
            .world()
            .resource::<crate::bootstrap::InstalledCatalog>()
            .0
            .clone();
        catalog
            .units
            .get_mut(&producer)
            .unwrap()
            .producer
            .as_mut()
            .unwrap()
            .recipes = vec![recipe.clone()];
        catalog.production.insert(
            recipe.clone(),
            turts_mod_api::ResolvedProduction {
                id: recipe.clone(),
                name: "Recipe".into(),
                output_unit: producer.clone(),
                duration_ms: 1000,
                costs: vec![],
                payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
                cancel_refunds: turts_mod_api::CancelRefunds {
                    queued: turts_mod_api::RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    running: turts_mod_api::RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                    output_blocked: turts_mod_api::RefundRatio {
                        numerator: 0,
                        denominator: 1,
                    },
                },
            },
        );
        app.world_mut()
            .insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        app.world_mut()
            .insert_resource(crate::bootstrap::ProductionDurations(
                std::collections::BTreeMap::from([(recipe.clone(), 20)]),
            ));
        let kind = app
            .world()
            .entity(
                app.world()
                    .iter_entities()
                    .find(|entity| entity.get::<crate::UnitId>().is_some_and(|id| id.0 == 1))
                    .unwrap()
                    .id(),
            )
            .get::<crate::UnitType>()
            .unwrap()
            .0;
        app.world_mut().spawn((
            crate::UnitId(2),
            crate::UnitType(kind),
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(12.0, 8.0)),
            UnitOrders::default(),
        ));
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:produce").unwrap(),
                behaviour: Behaviour::Produce,
                target_kind: turts_mod_api::TargetKind::Content,
                policy: None,
                units: vec![2, 1],
                target: OrderTargetSim::Content(recipe),
                queue: QueueMode::Append,
                group: 1,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = crate::apply_command(app.world_mut(), &command);
        assert_eq!(result.units.accepted, vec![1, 2]);
        assert_eq!(
            result.outcome,
            Ok(crate::Applied {
                job_id: Some(crate::JobId(1))
            })
        );
        let queues = app
            .world()
            .resource::<crate::economy::production::ProductionQueues>();
        assert_eq!(queues.0.get(&1).unwrap()[0].id, crate::JobId(1));
        assert_eq!(queues.0.get(&2).unwrap()[0].id, crate::JobId(2));
        let owner_view = crate::economy::views::world_view(app.world_mut());
        assert_eq!(
            owner_view
                .economy
                .queues
                .iter()
                .find(|queue| queue.producer == 1)
                .unwrap()
                .jobs[0]
                .id,
            1
        );
        let cancel = crate::Command {
            slot: SlotId(0),
            seq: 2,
            kind: crate::CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("test:cancel").unwrap(),
                behaviour: Behaviour::CancelProduction,
                target_kind: turts_mod_api::TargetKind::Job,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Job(result.outcome.unwrap().job_id.unwrap()),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let cancelled = crate::apply_command(app.world_mut(), &cancel);
        assert_eq!(cancelled.units.accepted, vec![1]);
        let owner_view = crate::economy::views::world_view(app.world_mut());
        assert!(
            owner_view
                .economy
                .queues
                .iter()
                .all(|queue| queue.producer != 1 || queue.jobs.is_empty())
        );
    }

    #[test]
    fn depleted_gather_writes_last_failure() {
        let mut app = crate::headless_app(20);
        let kind = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &PackageId("test".into()),
                UnitDef {
                    id: "worker".into(),
                    name: "Worker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    footprint: Footprint::Circle { radius: 0.5 },
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
            )
            .unwrap();
        let order_id = ContentId::parse("test:gather").unwrap();
        let entity = app
            .world_mut()
            .spawn((
                crate::UnitId(1),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(8.0, 8.0)),
                UnitOrders {
                    current: Some(Order {
                        order: order_id.clone(),
                        behaviour: Behaviour::Gather,
                        target: OrderTargetSim::Node(NodeId(1)),
                        group: 0,
                    }),
                    queue: VecDeque::new(),
                    last_failure: None,
                },
            ))
            .id();
        app.world_mut()
            .insert_resource(crate::economy::SurfaceNodes(vec![
                crate::economy::NodeRecord {
                    id: NodeId(1),
                    content_id: ContentId::parse("test:ore").unwrap(),
                    position: crate::Position(Vec2::new(10.0, 8.0)),
                    remaining: 0,
                },
            ]));
        app.world_mut()
            .resource_mut::<crate::EconomyOrders>()
            .0
            .insert(
                1,
                crate::economy::GatherOrder {
                    node: Some(NodeId(1)),
                    deposit: None,
                    receiver: None,
                    stage: crate::GatherStage::Gathering,
                    carry: crate::RateCarry::default(),
                },
            );
        app.world_mut().resource_mut::<crate::Tick>().0 = 4;
        app.world_mut().resource_mut::<crate::AuditTrail>().enable();

        run_orders(app.world_mut());

        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert_eq!(
            orders.last_failure,
            Some(OrderFailure {
                order: order_id,
                reason: crate::RejectReason::NodeDepleted,
                tick: 4,
            })
        );
        assert_eq!(
            app.world().resource::<crate::AuditTrail>().operations[0].phase,
            crate::economy::transaction::AuditPhase::Orders
        );
    }

    #[test]
    fn queued_gather_that_depletes_before_activation_fails_and_idles() {
        let mut app = crate::headless_app(20);
        let kind = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &PackageId("test".into()),
                UnitDef {
                    id: "worker".into(),
                    name: "Worker".into(),
                    movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                    inventory: None,
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    footprint: Footprint::Circle { radius: 0.5 },
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
            )
            .unwrap();
        let order_id = ContentId::parse("test:gather").unwrap();
        let entity = app
            .world_mut()
            .spawn((
                crate::UnitId(1),
                crate::UnitType(kind),
                crate::Owner(SlotId(0)),
                crate::Position(Vec2::new(8.0, 8.0)),
                UnitOrders {
                    current: None,
                    queue: VecDeque::from([Order {
                        order: order_id.clone(),
                        behaviour: Behaviour::Gather,
                        target: OrderTargetSim::Node(NodeId(1)),
                        group: 0,
                    }]),
                    last_failure: None,
                },
            ))
            .id();
        app.world_mut()
            .insert_resource(crate::economy::SurfaceNodes(vec![
                crate::economy::NodeRecord {
                    id: NodeId(1),
                    content_id: ContentId::parse("test:ore").unwrap(),
                    position: crate::Position(Vec2::new(10.0, 8.0)),
                    remaining: 0,
                },
            ]));
        app.world_mut().resource_mut::<crate::Tick>().0 = 7;
        app.world_mut().resource_mut::<crate::AuditTrail>().enable();

        run_orders(app.world_mut());

        let orders = app.world().entity(entity).get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert!(orders.queue.is_empty());
        assert_eq!(
            orders.last_failure,
            Some(OrderFailure {
                order: order_id,
                reason: crate::RejectReason::NodeDepleted,
                tick: 7,
            })
        );
        assert_eq!(
            app.world().resource::<crate::AuditTrail>().operations.len(),
            1
        );
        assert_eq!(
            app.world().resource::<crate::AuditTrail>().operations[0].phase,
            crate::economy::transaction::AuditPhase::Orders
        );
    }
}
