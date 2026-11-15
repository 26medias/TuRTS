//! The validation and application of an order command.

use crate::SlotId;
use crate::orders::behaviours::queue::start_order;
#[cfg(test)]
use crate::orders::count_pending_capacity_lookup;
use crate::orders::pending::PendingMovementOrders;
use crate::orders::plain::{
    PlainMoveOrderIdentities, PlainMoveReservations, capture_plain_move_prior,
    forget_plain_move_identity, plain_move_has_no_prior_order_state,
    plain_move_reservation_capacity_available, plain_move_reservation_exists,
    record_plain_move_reservation, remember_plain_move_identity, restore_plain_move_prior,
};
use crate::orders::writer::{edit_unit_orders, set_unit_orders};
use crate::orders::{
    Order, OrderCommand, OrderTargetSim, PendingMovementOrder, QueueMode, UnitOrders, UnitOutcomes,
};
use crate::world::MoveTarget;
use bevy_ecs::prelude::World;
use bevy_math::Vec2;
use std::collections::BTreeMap;
use turts_mod_api::Behaviour;

pub(crate) fn unit_order_capable(world: &World, kind: turts_mod_api::UnitTypeId) -> bool {
    let Some(entry) = world.resource::<crate::registry::UnitTypes>().get(kind) else {
        return false;
    };
    entry.movement.is_some()
        || world
            .get_resource::<crate::bootstrap::InstalledCatalog>()
            .is_some_and(|catalog| {
                catalog.0.units.values().any(|unit| {
                    unit.id.to_string() == entry.content_id
                        && (unit.weapon.is_some()
                            || unit.producer.is_some()
                            || unit.constructor.is_some()
                            || unit.repairer.is_some())
                })
            })
}

pub(super) fn unit_toggleable(unit: &turts_mod_api::ResolvedUnit) -> bool {
    unit.extractor.is_some()
        || unit.inserter.is_some()
        || unit.machine.is_some()
        || unit.research.is_some()
        || unit.producer.is_some()
        || unit.constructor.is_some()
        || unit.weapon.is_some()
        || unit.repairer.is_some()
}

/// Apply one server-resolved order envelope, validating each unit independently.
pub(crate) fn apply_order(
    world: &mut World,
    slot: SlotId,
    seq: u32,
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
    let entity_by_unit: BTreeMap<_, _> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<crate::world::UnitId>()
                .map(|id| (id.0, entity.id()))
        })
        .collect();
    if command.behaviour == Behaviour::CancelConstruction {
        return cancel_sites(world, slot, command, &unit_ids, &entity_by_unit);
    }
    let financial = matches!(command.behaviour, Behaviour::Produce | Behaviour::Construct);
    let mut anchors = BTreeMap::new();
    let mut tied = std::collections::BTreeSet::new();
    if financial {
        for &unit in &unit_ids {
            if let Ok(entity) = validate_member(world, slot, command, &unit, &entity_by_unit)
                && let Some(position) = world.get::<crate::Position>(entity)
                && position.0.is_finite()
            {
                anchors.insert(unit, position.0);
            }
        }
        let coordinate = |value: f32| if value == 0.0 { 0.0 } else { value };
        unit_ids.sort_by(
            |left, right| match (anchors.get(left), anchors.get(right)) {
                (Some(left), Some(right)) => coordinate(left.x)
                    .total_cmp(&coordinate(right.x))
                    .then_with(|| coordinate(left.y).total_cmp(&coordinate(right.y))),
                (None, None) => left.cmp(right),
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
            },
        );
        for pair in unit_ids.windows(2) {
            if let (Some(left), Some(right)) = (anchors.get(&pair[0]), anchors.get(&pair[1]))
                && left == right
            {
                tied.extend(pair.iter().copied());
            }
        }
    }
    let mut outcomes = UnitOutcomes::default();
    let mut first_job_id = None;
    let mut spread_units = Vec::new();
    let mut spread_order = None;
    let immediate = matches!(
        command.behaviour,
        Behaviour::Stop
            | Behaviour::Produce
            | Behaviour::Construct
            | Behaviour::SetRally
            | Behaviour::CancelProduction
            | Behaviour::Remove
            | Behaviour::SetEnabled(_)
    );
    for unit_id in &unit_ids {
        let entity = match validate_member(world, slot, command, unit_id, &entity_by_unit) {
            Ok(entity) => entity,
            Err(reason) => {
                outcomes.refused.push((*unit_id, reason));
                continue;
            }
        };
        if financial {
            if !anchors.contains_key(unit_id) {
                outcomes.refused.push((*unit_id, R::IntegrityViolation));
                continue;
            }
            if tied.contains(unit_id) {
                outcomes.refused.push((*unit_id, R::Blocked));
                continue;
            }
        }
        if command.behaviour == Behaviour::Repair {
            let OrderTargetSim::Unit(target) = command.target else {
                outcomes.refused.push((*unit_id, R::TargetInvalid));
                continue;
            };
            if let Err(reason) = crate::economy::repair::validate_order(world, *unit_id, target) {
                outcomes.refused.push((*unit_id, reason));
                continue;
            }
        }
        if command.queue == QueueMode::Append && !immediate {
            let orders = world.entity(entity).get::<UnitOrders>();
            let pending_move = unit_has_pending_move(world, *unit_id);
            let effective_behaviour = if pending_move {
                Some(Behaviour::Move)
            } else {
                orders.and_then(|orders| orders.current.as_ref().map(|current| current.behaviour))
            };
            if effective_behaviour
                .is_some_and(|behaviour| matches!(behaviour, Behaviour::Hold | Behaviour::Gather))
                || orders.is_some_and(|orders| {
                    orders.queue.iter().any(|queued| {
                        matches!(queued.behaviour, Behaviour::Hold | Behaviour::Gather)
                    })
                })
            {
                outcomes.refused.push((*unit_id, R::NotQueueable));
                continue;
            }
            let legacy_gather_active = effective_behaviour != Some(Behaviour::Move)
                && world
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

        let deferred_move = (command.behaviour == Behaviour::Move
            || (command.behaviour == Behaviour::Attack
                && matches!(command.target, OrderTargetSim::Point(_))))
            && command.queue == QueueMode::Replace
            && (((command.group != 0 || command.units.len() > 1)
                && matches!(
                    &command.target,
                    OrderTargetSim::Point(_) | OrderTargetSim::Unit(_)
                ))
                || matches!(&command.target, OrderTargetSim::Unit(_)));
        let singleton_replace_move = command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && command.group == 0
            && command.units.len() == 1
            && matches!(&command.target, OrderTargetSim::Point(_));
        if singleton_replace_move
            && !plain_move_reservation_capacity_available(world, slot, *unit_id)
        {
            outcomes.refused.push((*unit_id, R::RateLimited));
            continue;
        }
        let inherited_prior = if singleton_replace_move {
            let reservation = world
                .get_resource_mut::<PlainMoveReservations>()
                .and_then(|mut reservations| reservations.remove(unit_id));
            reservation.map(|reservation| {
                restore_plain_move_prior(world, entity, *unit_id, &reservation.prior, false);
                reservation.prior
            })
        } else {
            None
        };
        let reservation_prior = singleton_replace_move.then(|| {
            inherited_prior
                .clone()
                .unwrap_or_else(|| capture_plain_move_prior(world, entity, *unit_id))
        });
        let order = Order {
            order: command.order.clone(),
            behaviour: command.behaviour,
            target: command.target.clone(),
            group: command.group,
        };
        if deferred_move {
            spread_units.push(*unit_id);
            spread_order = Some(order);
            outcomes.accepted.push(*unit_id);
            continue;
        }

        let plain_move = command.behaviour == Behaviour::Move
            && command.queue == QueueMode::Replace
            && command.group == 0
            && command.units.len() == 1
            && matches!(&command.target, OrderTargetSim::Point(_))
            && (plain_move_has_no_prior_order_state(world, command)
                || plain_move_reservation_exists(world, *unit_id));
        let plain_gather = command.behaviour == Behaviour::Gather
            && command.queue == QueueMode::Replace
            && command.group == 0
            && !unit_has_pending_move(world, *unit_id)
            && world.entity(entity).get::<MoveTarget>().is_none()
            && world
                .entity(entity)
                .get::<crate::movement::MoveProgress>()
                .is_none()
            && world
                .entity(entity)
                .get::<crate::movement::PlainMoveProgress>()
                .is_none();
        if plain_move {
            let OrderTargetSim::Point(target) = &command.target else {
                outcomes.refused.push((*unit_id, R::TargetInvalid));
                continue;
            };
            let target = clamp_target(world, *target, unit_radius(world, entity));
            crate::command::apply_legacy_move_to(world, slot, &[*unit_id], target)
                .expect("order validation precedes legacy move application");
            remember_plain_move_identity(world, *unit_id, order.clone());
            record_plain_move_reservation(
                world,
                *unit_id,
                slot,
                seq,
                target,
                order.clone(),
                reservation_prior.clone().unwrap_or_default(),
            );
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
            let target = clamp_target(world, *target, unit_radius(world, entity));
            crate::economy::transaction::capture_unit_order_operation(world, *unit_id, |world| {
                world
                    .resource_mut::<PendingMovementOrders>()
                    .cancel_units(slot.0, &[*unit_id]);
                let mut movement = world.entity_mut(entity);
                movement.remove::<MoveTarget>();
                movement.remove::<crate::movement::MoveProgress>();
                movement.remove::<crate::movement::PlainMoveProgress>();
                crate::command::apply_legacy_move_to(world, slot, &[*unit_id], target)
                    .expect("order validation precedes legacy move application");
                edit_unit_orders(world, entity, |orders| *orders = UnitOrders::default());
                clear_gather(world, *unit_id);
                remember_plain_move_identity(world, *unit_id, order.clone());
            });
            record_plain_move_reservation(
                world,
                *unit_id,
                slot,
                seq,
                target,
                order.clone(),
                reservation_prior.clone().unwrap_or_default(),
            );
            outcomes.accepted.push(*unit_id);
            continue;
        }
        let result = if plain_gather {
            apply_unit_order(world, slot, seq, entity, *unit_id, order, command)
        } else {
            crate::economy::transaction::capture_unit_order_operation(world, *unit_id, |world| {
                apply_unit_order(world, slot, seq, entity, *unit_id, order, command)
            })
        };
        match result {
            Ok(job_id) => {
                if let Some(job) = job_id {
                    crate::receipts::capture_admitted_work(world, slot, seq, *unit_id, job);
                }
                if first_job_id.is_none() {
                    first_job_id = job_id;
                }
                if command.queue == QueueMode::Replace
                    && command.behaviour != Behaviour::Move
                    && let Some(mut reservations) =
                        world.get_resource_mut::<PlainMoveReservations>()
                {
                    reservations.remove(unit_id);
                }
                outcomes.accepted.push(*unit_id)
            }
            Err(reason) => outcomes.refused.push((*unit_id, reason)),
        }
    }

    if !spread_units.is_empty() {
        let spread_unit_set: std::collections::HashSet<_> = spread_units.iter().copied().collect();
        let max_radius = spread_units
            .iter()
            .filter_map(|unit_id| entity_by_unit.get(unit_id))
            .map(|entity| unit_radius(world, *entity))
            .fold(0.0_f32, f32::max);
        let point = match command.target {
            OrderTargetSim::Point(point) => point,
            OrderTargetSim::Unit(target_id) => entity_by_unit
                .get(&target_id)
                .and_then(|entity| {
                    world
                        .entity(*entity)
                        .get::<crate::world::Position>()
                        .map(|p| p.0)
                })
                .expect("validated unit target remains present during command application"),
            _ => unreachable!("deferred movement has a point or unit target"),
        };
        let point = clamp_target(world, point, max_radius);
        let tick = world.resource::<crate::world::Tick>().0;
        let can_queue = world
            .resource::<PendingMovementOrders>()
            .can_queue_after_replacing(slot.0, command.group, tick, &spread_units)
            && movement_envelope_capacity_available_after_replacing(world, slot, &spread_units);
        if !can_queue {
            outcomes
                .accepted
                .retain(|unit| !spread_unit_set.contains(unit));
            outcomes.refused.extend(
                spread_units
                    .iter()
                    .copied()
                    .map(|unit| (unit, R::RateLimited)),
            );
        } else {
            let request = PendingMovementOrder {
                slot,
                accepted_tick: tick,
                seq,
                group: command.group,
                pool_id: 0,
                point,
                unit_ids: spread_units.clone(),
                order: spread_order,
                member_orders: BTreeMap::new(),
                plain: false,
            };
            let mut request = Some(request);
            spread_units.sort_unstable();
            for unit_id in &spread_units {
                let Some(entity) = entity_by_unit.get(unit_id).copied() else {
                    continue;
                };
                crate::economy::transaction::capture_unit_order_operation(
                    world,
                    *unit_id,
                    |world| {
                        if let Some(request) = request.take() {
                            let mut pending = world.resource_mut::<PendingMovementOrders>();
                            pending.cancel_units(slot.0, &spread_units);
                            assert!(
                                pending.queue(request),
                                "capacity was preflighted before captured envelope admission"
                            );
                        }
                        edit_unit_orders(world, entity, |orders| orders.queue.clear());
                        let canceled_plain = world
                            .get_resource_mut::<PlainMoveReservations>()
                            .and_then(|mut reservations| reservations.remove(unit_id));
                        if let Some(reservation) = canceled_plain {
                            restore_plain_move_prior(
                                world,
                                entity,
                                *unit_id,
                                &reservation.prior,
                                false,
                            );
                        }
                    },
                );
            }
        }
    }

    // Processing priority is physical. Public per-unit records keep their canonical ID order.
    outcomes.accepted.sort_unstable();
    outcomes.refused.sort_unstable_by_key(|(unit, _)| *unit);
    for &unit in &outcomes.accepted {
        note_accepted_player_order(world, unit, command.behaviour, command.queue);
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

fn unit_definition(world: &World, unit: u32) -> Option<&turts_mod_api::ResolvedUnit> {
    let entity = crate::unit_index::unit_entity(world, unit)?;
    let kind = world.get::<crate::UnitType>(entity)?;
    let name = &world
        .get_resource::<crate::UnitTypes>()?
        .get(kind.0)?
        .content_id;
    world
        .get_resource::<crate::bootstrap::InstalledCatalog>()?
        .0
        .units
        .get(&turts_mod_api::ContentId::parse(name).ok()?)
}

/// Only accepted player commands may change the persistent automatic-repair latch.
pub(crate) fn note_accepted_player_order(
    world: &mut World,
    unit: u32,
    behaviour: Behaviour,
    queue: QueueMode,
) {
    if !unit_definition(world, unit).is_some_and(|definition| definition.repairer.is_some()) {
        return;
    }
    let Some(entity) = crate::unit_index::unit_entity(world, unit) else {
        return;
    };
    crate::economy::transaction::capture_unit_order_operation(world, unit, |world| {
        crate::orders::writer::set_repair_stopped(world, entity, behaviour == Behaviour::Stop);
        let replaces_action = matches!(
            behaviour,
            Behaviour::Move
                | Behaviour::Stop
                | Behaviour::Hold
                | Behaviour::Attack
                | Behaviour::Gather
                | Behaviour::Repair
        );
        if !replaces_action {
            return;
        }
        let manual_action = world
            .get_resource::<crate::economy::repair::RepairRuntime>()
            .and_then(|runtime| runtime.units.get(&unit))
            .and_then(|state| state.action.as_ref())
            .is_some_and(|action| !action.automatic);
        let current_repair = world
            .get::<UnitOrders>(entity)
            .and_then(|orders| orders.current.as_ref())
            .is_some_and(|order| order.behaviour == Behaviour::Repair);
        if queue == QueueMode::Append
            && behaviour != Behaviour::Stop
            && manual_action
            && current_repair
        {
            return;
        }
        let no_current = world
            .get::<UnitOrders>(entity)
            .is_none_or(|orders| orders.current.is_none());
        if queue == QueueMode::Append
            && no_current
            && !unit_has_pending_move(world, unit)
            && crate::economy::repair::active_action(world, unit)
        {
            // Only an actual repair action owns implicit approach motion. Plain
            // player moves also have no current order and must keep their motion.
            // A queued explicit order must not wait for an implicit repair approach to finish.
            crate::economy::repair::clear_motion(world, unit);
        }
        crate::economy::repair::order_changed(world, unit);
    });
}

fn cancel_sites(
    world: &mut World,
    slot: SlotId,
    command: &OrderCommand,
    unit_ids: &[u32],
    entities: &BTreeMap<u32, bevy_ecs::entity::Entity>,
) -> (
    Result<crate::command::Applied, crate::RejectReason>,
    UnitOutcomes,
) {
    use crate::RejectReason as R;
    let mut outcomes = UnitOutcomes::default();
    let mut sites = Vec::new();
    for &unit in unit_ids {
        let checked = (|| {
            let entity = *entities.get(&unit).ok_or(R::UnknownUnit)?;
            if world.get::<crate::Owner>(entity).map(|owner| owner.0) != Some(slot) {
                return Err(R::NotOwner);
            }
            if !world
                .resource::<crate::economy::construction::ConstructionState>()
                .sites
                .contains_key(&unit)
            {
                return Err(R::NotApplicable);
            }
            let definition = unit_definition(world, unit).ok_or(R::NotApplicable)?;
            if !command.allowed_units.is_empty()
                && !command.allowed_units.contains(&definition.id.to_string())
            {
                return Err(R::NotApplicable);
            }
            validate_target(world, command)?;
            let position = world
                .get::<crate::Position>(entity)
                .ok_or(R::IntegrityViolation)?
                .0;
            Ok(position)
        })();
        match checked {
            Ok(position) => sites.push((unit, position)),
            Err(reason) => outcomes.refused.push((unit, reason)),
        }
    }
    let coordinate = |value: f32| if value == 0.0 { 0.0 } else { value };
    sites.sort_by(|left, right| {
        coordinate(left.1.x)
            .total_cmp(&coordinate(right.1.x))
            .then_with(|| coordinate(left.1.y).total_cmp(&coordinate(right.1.y)))
    });
    for (index, &(unit, position)) in sites.iter().enumerate() {
        let tied = index
            .checked_sub(1)
            .is_some_and(|previous| sites[previous].1 == position)
            || sites.get(index + 1).is_some_and(|next| next.1 == position);
        if tied {
            outcomes.refused.push((unit, R::Blocked));
            continue;
        }
        match crate::economy::construction::cancel_site(world, slot, unit) {
            Ok(()) => outcomes.accepted.push(unit),
            Err(reason) => outcomes.refused.push((unit, reason)),
        }
    }
    outcomes.accepted.sort_unstable();
    outcomes.refused.sort_unstable_by_key(|(unit, _)| *unit);
    let result = if outcomes.accepted.is_empty() {
        Err(outcomes
            .refused
            .first()
            .map_or(R::NotApplicable, |(_, reason)| *reason))
    } else {
        Ok(crate::command::Applied { job_id: None })
    };
    (result, outcomes)
}

fn validate_member(
    world: &World,
    slot: SlotId,
    command: &OrderCommand,
    unit_id: &u32,
    entity_by_unit: &BTreeMap<u32, bevy_ecs::entity::Entity>,
) -> Result<bevy_ecs::entity::Entity, crate::command::RejectReason> {
    use crate::command::RejectReason as R;
    let Some(entity) = entity_by_unit.get(unit_id).copied() else {
        return Err(R::UnknownUnit);
    };
    let owner = world
        .entity(entity)
        .get::<crate::world::Owner>()
        .map(|owner| owner.0);
    if owner != Some(slot) {
        return Err(R::NotOwner);
    }
    if world
        .resource::<crate::economy::construction::ConstructionState>()
        .sites
        .contains_key(unit_id)
    {
        return Err(R::NotApplicable);
    }
    let Some(unit_type) = world
        .entity(entity)
        .get::<crate::world::UnitType>()
        .map(|kind| kind.0)
    else {
        return Err(R::NotApplicable);
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
            definition.is_some_and(|unit| {
                unit.weapon.is_some() || unit.constructor.is_some() || unit.repairer.is_some()
            }) || registry_movement
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
        // API 0.2.4: no unit is capable of a new behaviour until its lane builds it (the
        // packer refuses these orders with `UNBUILT_FEATURES` until then).
        Behaviour::Construct => definition.is_some_and(|unit| unit.constructor.is_some()),
        Behaviour::SetRally => definition.is_some_and(|unit| unit.producer.is_some()),
        Behaviour::Repair => definition.is_some_and(|unit| unit.repairer.is_some()),
        Behaviour::CancelConstruction | Behaviour::Surrender => false,
    };
    if !capable {
        let reason = match command.behaviour {
            Behaviour::Gather
            | Behaviour::Produce
            | Behaviour::CancelProduction
            | Behaviour::Construct
            | Behaviour::Repair => R::MissingCapability,
            Behaviour::Remove => R::NotRemovable,
            Behaviour::SetEnabled(_) => R::NotToggleable,
            _ => R::NotApplicable,
        };
        return Err(reason);
    }
    if !command.allowed_units.is_empty()
        && !unit_id_text
            .as_ref()
            .is_some_and(|id| command.allowed_units.iter().any(|allowed| allowed == id))
    {
        return Err(R::NotApplicable);
    }
    crate::visibility::validate_target(world, slot.0, &command.target)?;
    validate_target(world, command)?;
    Ok(entity)
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
        | (turts_mod_api::TargetKind::ContentAtPoint, OrderTargetSim::ContentAtPoint { .. })
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
        (Behaviour::SetRally, OrderTargetSim::Point(point)) => {
            if !point.is_finite() {
                return Err(R::TargetNotFinite);
            }
            let bounds = world.resource::<crate::MapBounds>().size;
            if point.x < 0.0 || point.y < 0.0 || point.x > bounds.x || point.y > bounds.y {
                return Err(R::TargetInvalid);
            }
            Ok(())
        }
        (Behaviour::SetRally, _) => Err(R::TargetInvalid),
        (Behaviour::CancelConstruction, OrderTargetSim::None) => Ok(()),
        (Behaviour::CancelConstruction, _) => Err(R::TargetInvalid),
        (Behaviour::Repair, OrderTargetSim::Unit(unit)) => {
            crate::unit_index::unit_entity(world, *unit)
                .map(|_| ())
                .ok_or(R::TargetGone)
        }
        (Behaviour::Repair, _) => Err(R::TargetInvalid),
        (Behaviour::Construct, OrderTargetSim::ContentAtPoint { point, .. })
            if point.is_finite() =>
        {
            Ok(())
        }
        (Behaviour::Construct, OrderTargetSim::ContentAtPoint { .. }) => Err(R::TargetNotFinite),
        (Behaviour::Move, OrderTargetSim::Point(point)) if point.is_finite() => Ok(()),
        (Behaviour::Move, OrderTargetSim::Point(_)) => Err(R::TargetNotFinite),
        (Behaviour::Move, OrderTargetSim::Unit(unit)) => {
            if crate::unit_index::unit_entity(world, *unit).is_some() {
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
            if crate::unit_index::unit_entity(world, *unit).is_some() {
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

pub(super) fn movement_envelope_capacity_available_after_replacing(
    world: &World,
    slot: SlotId,
    unit_ids: &[u32],
) -> bool {
    let replace_ids: std::collections::HashSet<_> = unit_ids.iter().copied().collect();
    let pending = world.resource::<PendingMovementOrders>();
    let current_pending = pending
        .pending_units
        .get(&slot.0)
        .copied()
        .unwrap_or_default();
    let removed_pending = replace_ids
        .iter()
        .filter_map(|unit_id| {
            #[cfg(test)]
            count_pending_capacity_lookup();
            pending.pending_member_counts.get(unit_id)
        })
        .copied()
        .sum::<usize>();
    let pending_after = current_pending
        .saturating_sub(removed_pending)
        .saturating_add(replace_ids.len());
    let reserved_after = world
        .get_resource::<PlainMoveReservations>()
        .map_or(0, |reservations| {
            reservations
                .slot_counts
                .get(&slot.0)
                .copied()
                .unwrap_or_default()
                .saturating_sub(
                    replace_ids
                        .iter()
                        .filter(|unit_id| {
                            reservations
                                .entries
                                .get(unit_id)
                                .is_some_and(|reservation| reservation.slot == slot)
                        })
                        .count(),
                )
        });
    pending_after.saturating_add(reserved_after) <= 16_384
}

pub(super) fn unit_has_pending_move(world: &World, unit_id: u32) -> bool {
    world
        .resource::<PendingMovementOrders>()
        .pending_member_counts
        .contains_key(&unit_id)
        || plain_move_reservation_exists(world, unit_id)
}

pub(super) fn unit_orders_empty(world: &World, unit_id: u32) -> bool {
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

pub(super) fn unit_radius(world: &World, entity: bevy_ecs::entity::Entity) -> f32 {
    world
        .entity(entity)
        .get::<crate::world::UnitType>()
        .and_then(|kind| world.resource::<crate::registry::UnitTypes>().get(kind.0))
        .map_or(0.5, |unit| unit.radius)
}

pub(super) fn clamp_target(world: &World, target: Vec2, radius: f32) -> Vec2 {
    world
        .get_resource::<crate::world::MapBounds>()
        .map_or(target, |bounds| {
            Vec2::new(
                target.x.clamp(radius, (bounds.size.x - radius).max(radius)),
                target.y.clamp(radius, (bounds.size.y - radius).max(radius)),
            )
        })
}

fn apply_unit_order(
    world: &mut World,
    slot: SlotId,
    seq: u32,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
    order: Order,
    command: &OrderCommand,
) -> Result<Option<crate::JobId>, crate::command::RejectReason> {
    use crate::command::{CommandKind, RejectReason as R};
    match order.behaviour {
        Behaviour::Move => {
            let pending_replace =
                command.queue == QueueMode::Append && unit_has_pending_move(world, unit_id);
            let target = match &order.target {
                OrderTargetSim::Point(target) => {
                    clamp_target(world, *target, unit_radius(world, entity))
                }
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
                    .map(|target| clamp_target(world, target, unit_radius(world, entity)))
                    .ok_or(R::TargetGone)?,
                _ => return Err(R::TargetInvalid),
            };
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            let starts_now = if command.queue == QueueMode::Append {
                let existing_move = world
                    .entity(entity)
                    .get::<crate::world::MoveTarget>()
                    .is_some();
                let pending_move = unit_has_pending_move(world, unit_id);
                let appended = edit_unit_orders(world, entity, |orders| {
                    if orders.current.is_none() && (existing_move || pending_move) {
                        orders.queue.push_back(order.clone());
                        false
                    } else if orders.current.is_none() {
                        orders.current = Some(order.clone());
                        true
                    } else {
                        orders.queue.push_back(order.clone());
                        false
                    }
                });
                appended.unwrap_or_else(|| {
                    set_unit_orders(
                        world,
                        entity,
                        UnitOrders {
                            current: Some(order.clone()),
                            ..Default::default()
                        },
                    );
                    true
                })
            } else {
                set_unit_orders(
                    world,
                    entity,
                    UnitOrders {
                        current: Some(order),
                        ..Default::default()
                    },
                );
                true
            };
            if starts_now {
                let mut entity_mut = world.entity_mut(entity);
                entity_mut.remove::<crate::movement::MoveProgress>();
                entity_mut.remove::<crate::movement::PlainMoveProgress>();
                entity_mut.insert(crate::world::MoveTarget(target));
                forget_plain_move_identity(world, unit_id);
            }
            if !pending_replace {
                clear_gather(world, unit_id);
            }
            Ok(None)
        }
        Behaviour::Attack => match order.target {
            OrderTargetSim::Unit(_) | OrderTargetSim::Point(_) => {
                let pending_replace =
                    command.queue == QueueMode::Append && unit_has_pending_move(world, unit_id);
                let active_order = order.clone();
                if command.queue == QueueMode::Replace {
                    clear_replaced_movement(world, slot, entity, unit_id);
                }
                let mut orders = world
                    .entity(entity)
                    .get::<UnitOrders>()
                    .cloned()
                    .unwrap_or_default();
                let starts_now = if command.queue == QueueMode::Append {
                    if orders.current.is_some()
                        || world.entity(entity).get::<MoveTarget>().is_some()
                        || unit_has_pending_move(world, unit_id)
                    {
                        orders.queue.push_back(order.clone());
                        false
                    } else {
                        orders.current = Some(order);
                        true
                    }
                } else {
                    orders.current = Some(order);
                    orders.queue.clear();
                    true
                };
                world.entity_mut(entity).insert(orders);
                if starts_now {
                    if let Some(mut weapons) =
                        world.get_resource_mut::<crate::economy::combat::Weapons>()
                        && let Some(state) = weapons.0.get_mut(&unit_id)
                    {
                        state.last_target = None;
                        state.ordered_target_fired = None;
                    }
                    start_order(world, slot, entity, unit_id, &active_order)?;
                }
                if !pending_replace {
                    clear_gather(world, unit_id);
                }
                Ok(None)
            }
            _ => Err(R::TargetInvalid),
        },
        Behaviour::Stop => {
            if unit_definition(world, unit_id).is_some_and(|unit| unit.constructor.is_some()) {
                crate::economy::construction::clear_held(world, slot, unit_id)?;
            }
            clear_replaced_movement(world, slot, entity, unit_id);
            set_unit_orders(world, entity, UnitOrders::default());
            clear_gather(world, unit_id);
            Ok(None)
        }
        Behaviour::Hold => {
            if command.queue == QueueMode::Append {
                if edit_unit_orders(world, entity, |orders| orders.queue.push_back(order)).is_none()
                {
                    return Err(R::NotQueueable);
                }
            } else {
                clear_replaced_movement(world, slot, entity, unit_id);
                world
                    .entity_mut(entity)
                    .remove::<crate::world::MoveTarget>();
                set_unit_orders(
                    world,
                    entity,
                    UnitOrders {
                        current: Some(order),
                        ..Default::default()
                    },
                );
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
                    .is_some()
                    || unit_has_pending_move(world, unit_id);
                if no_current && !moving_from_legacy_order {
                    if depleted {
                        world
                            .entity_mut(entity)
                            .remove::<crate::world::MoveTarget>();
                        clear_gather(world, unit_id);
                    } else {
                        crate::economy::gather::assign_gather(world, slot, unit_id, node)?;
                    }
                    set_unit_orders(
                        world,
                        entity,
                        UnitOrders {
                            current: Some(order),
                            ..Default::default()
                        },
                    );
                } else {
                    edit_unit_orders(world, entity, |orders| orders.queue.push_back(order));
                }
            } else {
                if depleted {
                    clear_replaced_movement(world, slot, entity, unit_id);
                    world
                        .entity_mut(entity)
                        .remove::<crate::world::MoveTarget>();
                    clear_gather(world, unit_id);
                } else {
                    crate::economy::gather::assign_gather(world, slot, unit_id, node)?;
                    clear_replaced_movement(world, slot, entity, unit_id);
                }
                set_unit_orders(
                    world,
                    entity,
                    UnitOrders {
                        current: Some(order),
                        ..Default::default()
                    },
                );
            }
            Ok(None)
        }
        Behaviour::Produce => {
            let OrderTargetSim::Content(recipe_id) = order.target else {
                return Err(R::TargetInvalid);
            };
            let job_id = crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::Produce {
                    producer_id: unit_id,
                    recipe_id,
                },
            )
            .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            Ok(job_id)
        }
        Behaviour::CancelProduction => {
            let OrderTargetSim::Job(job_id) = order.target else {
                return Err(R::TargetInvalid);
            };
            let job_id = crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::CancelProduction {
                    producer_id: unit_id,
                    job_id,
                },
            )
            .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            Ok(job_id)
        }
        Behaviour::Remove => {
            let job_id =
                crate::command::apply_legacy_unit(world, slot, CommandKind::Remove { unit_id })
                    .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                world
                    .resource_mut::<PendingMovementOrders>()
                    .cancel_units(slot.0, &[unit_id]);
            }
            Ok(job_id)
        }
        Behaviour::SetEnabled(enabled) => {
            let job_id = crate::command::apply_legacy_unit(
                world,
                slot,
                CommandKind::SetEnabled { unit_id, enabled },
            )
            .map(|applied| applied.job_id)?;
            if command.queue == QueueMode::Replace {
                clear_replaced_movement(world, slot, entity, unit_id);
            }
            Ok(job_id)
        }
        Behaviour::Construct => {
            let OrderTargetSim::ContentAtPoint { content, point } = order.target else {
                return Err(R::TargetInvalid);
            };
            let origin = world
                .get_resource::<crate::receipts::CommandWorkReceipts>()
                .and_then(|receipts| receipts.member(slot, seq, unit_id));
            crate::economy::construction::construct_with_origin(
                world, slot, unit_id, content, point, origin,
            )
        }
        Behaviour::SetRally => {
            let OrderTargetSim::Point(point) = order.target else {
                return Err(R::TargetInvalid);
            };
            let mut orders = world.get::<UnitOrders>(entity).cloned().unwrap_or_default();
            orders.rally = Some(point);
            set_unit_orders(world, entity, orders);
            Ok(None)
        }
        Behaviour::Repair => {
            let OrderTargetSim::Unit(target) = order.target else {
                return Err(R::TargetInvalid);
            };
            crate::economy::repair::validate_order(world, unit_id, target)?;
            let active = world
                .get::<UnitOrders>(entity)
                .is_some_and(|orders| orders.current.is_some())
                || world.get::<MoveTarget>(entity).is_some()
                || unit_has_pending_move(world, unit_id);
            if command.queue == QueueMode::Append && active {
                if edit_unit_orders(world, entity, |orders| {
                    orders.queue.push_back(order.clone())
                })
                .is_none()
                {
                    set_unit_orders(
                        world,
                        entity,
                        UnitOrders {
                            queue: [order].into(),
                            ..Default::default()
                        },
                    );
                }
            } else {
                clear_replaced_movement(world, slot, entity, unit_id);
                clear_gather(world, unit_id);
                set_unit_orders(
                    world,
                    entity,
                    UnitOrders {
                        current: Some(order),
                        ..Default::default()
                    },
                );
            }
            Ok(None)
        }
        Behaviour::CancelConstruction | Behaviour::Surrender => Err(R::NotApplicable),
    }
}

pub(crate) fn clear_replaced_movement(
    world: &mut World,
    slot: SlotId,
    entity: bevy_ecs::entity::Entity,
    unit_id: u32,
) {
    edit_unit_orders(world, entity, |orders| orders.acquired = None);
    world
        .resource_mut::<PendingMovementOrders>()
        .cancel_units(slot.0, &[unit_id]);
    let mut movement = world.entity_mut(entity);
    movement.remove::<MoveTarget>();
    movement.remove::<crate::movement::MoveProgress>();
    movement.remove::<crate::movement::PlainMoveProgress>();
    if let Some(mut identities) = world.get_resource_mut::<PlainMoveOrderIdentities>() {
        identities.0.remove(&unit_id);
    }
}

pub(super) fn clear_gather(world: &mut World, unit_id: u32) {
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
mod unit_index_tests {
    use super::validate_target;
    use crate::command::RejectReason;
    use crate::orders::{OrderCommand, OrderTargetSim, QueueMode};
    use crate::unit_index::index_entry_for_test;
    use crate::{UnitId, headless_app};
    use turts_mod_api::{Behaviour, ContentId, TargetKind};

    fn command(behaviour: Behaviour, target: u32) -> OrderCommand {
        OrderCommand {
            order: ContentId::parse("fixture:order").unwrap(),
            behaviour,
            target_kind: TargetKind::Unit,
            policy: None,
            units: Vec::new(),
            target: OrderTargetSim::Unit(target),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: Vec::new(),
            queue_depth: 1,
        }
    }

    #[test]
    fn validate_target_reads_the_unit_index_and_does_not_scan() {
        let mut app = headless_app(20);
        app.world_mut().spawn(UnitId(8));
        for behaviour in [Behaviour::Move, Behaviour::Attack] {
            assert_eq!(validate_target(app.world(), &command(behaviour, 8)), Ok(()));
            assert_eq!(
                validate_target(app.world(), &command(behaviour, 99)),
                Err(RejectReason::TargetGone)
            );
        }
        // An entry for an id that no unit has: the index says the unit exists.
        index_entry_for_test(app.world_mut(), 99);
        for behaviour in [Behaviour::Move, Behaviour::Attack] {
            assert_eq!(
                validate_target(app.world(), &command(behaviour, 99)),
                Ok(()),
                "{behaviour:?} scanned the world"
            );
        }
    }
}
