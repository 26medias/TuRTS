//! Automatic acquisition after movement. Only units with an authored acquire capability use it.
use crate::orders::{AcquiredTarget, UnitOrders};
use crate::world::MoveTarget;
use crate::{Owner, Position, UnitId, UnitType, bootstrap::InstalledCatalog, registry::UnitTypes};
use bevy_ecs::{prelude::Entity, world::World};
use bevy_math::Vec2;
use std::collections::BTreeMap;
use turts_mod_api::{AcquireDef, ContentId, SlotRelation, WeaponRangeMode};

#[derive(Clone, Copy)]
struct Actor {
    id: u32,
    entity: Entity,
    owner: u8,
    position: Vec2,
    radius: f32,
    alive: bool,
    capability: Option<(AcquireDef, f32, WeaponRangeMode, SlotRelation)>,
}

/// Build one spatial index from this tick's positions. Explicit player orders always take priority.
pub(crate) fn run_acquire(world: &mut World) {
    let (Some(catalog), Some(types), Some(healths)) = (
        world.get_resource::<InstalledCatalog>(),
        world.get_resource::<UnitTypes>(),
        world.get_resource::<super::combat::Healths>(),
    ) else {
        return;
    };
    if !catalog
        .0
        .units
        .values()
        .any(|unit| unit.acquire.is_some() && unit.movement.is_some())
    {
        return;
    }
    // Resolve immutable capability data once per type, rather than parsing an ID per actor.
    let type_info = types
        .entries()
        .iter()
        .map(|kind| {
            let definition = ContentId::parse(&kind.content_id)
                .ok()
                .and_then(|id| catalog.0.units.get(&id))?;
            let capability = definition
                .acquire
                .zip(definition.weapon.as_ref())
                .filter(|_| definition.movement.is_some())
                .map(|(acquire, weapon)| {
                    (
                        acquire,
                        weapon.range,
                        weapon.range_mode,
                        weapon.targets.slots,
                    )
                });
            Some((kind.radius, capability))
        })
        .collect::<Vec<_>>();
    let actors = world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?.0;
            let (radius, capability) = type_info
                .get(entity.get::<UnitType>()?.0.0 as usize)
                .copied()
                .flatten()?;
            Some(Actor {
                id,
                entity: entity.id(),
                owner: entity.get::<Owner>()?.0.0,
                position: entity.get::<Position>()?.0,
                radius,
                alive: healths.0.get(&id).is_some_and(|health| *health > 0),
                capability,
            })
        })
        .collect::<Vec<_>>();
    if !actors.iter().any(|actor| actor.capability.is_some()) {
        return;
    }
    let points = actors
        .iter()
        .map(|actor| crate::grid::GridPoint {
            id: actor.id,
            position: actor.position,
        })
        .collect::<Vec<_>>();
    let max_radius = actors
        .iter()
        .map(|actor| actor.radius)
        .fold(0_f32, f32::max);
    let cell_size = actors
        .iter()
        .filter_map(|actor| actor.capability.map(|(acquire, _, _, _)| acquire.range))
        .fold(1_f32, f32::max);
    let grid = crate::grid::UniformGrid::new(&points, cell_size);
    let by_id = actors
        .iter()
        .enumerate()
        .map(|(index, actor)| (actor.id, index))
        .collect::<BTreeMap<_, _>>();
    let mut sources = actors
        .iter()
        .filter(|actor| actor.capability.is_some())
        .copied()
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| {
        left.position
            .x
            .total_cmp(&right.position.x)
            .then_with(|| left.position.y.total_cmp(&right.position.y))
    });
    for actor in sources {
        let Some((acquire, range, mode, relation)) = actor.capability else {
            continue;
        };
        let (has_current, has_queue, acquired) =
            world
                .get::<UnitOrders>(actor.entity)
                .map_or((false, false, None), |orders| {
                    (
                        orders.current.is_some(),
                        !orders.queue.is_empty(),
                        orders.acquired,
                    )
                });
        let pending = world
            .get_resource::<crate::orders::PendingMovementOrders>()
            .is_some_and(|pending| pending.pending_member_counts.contains_key(&actor.id));
        if pending {
            continue;
        }
        let explicit = has_current
            || has_queue
            || world
                .get_resource::<super::EconomyOrders>()
                .and_then(|orders| orders.0.get(&actor.id))
                .is_some_and(|order| {
                    order.node.is_some() || order.deposit.is_some() || order.receiver.is_some()
                });
        if explicit {
            if acquired.is_some() {
                write(world, actor, None, None, false);
            }
            continue;
        }
        if !super::power::weapon_enabled(world, actor.id)
            || super::repair::active_action(world, actor.id)
        {
            if acquired.is_some() {
                write(world, actor, None, None, true);
            }
            continue;
        }
        if acquired.is_none() && world.get::<MoveTarget>(actor.entity).is_some() {
            continue;
        }
        if let Some(state) = acquired.filter(|state| state.returning) {
            if world.get::<MoveTarget>(actor.entity).is_none()
                || actor.position.distance_squared(state.home) < 0.0001
            {
                write(world, actor, None, None, true);
            } else {
                write(world, actor, Some(state), Some(state.home), true);
            }
            continue;
        }
        let home = acquired.map_or(actor.position, |state| state.home);
        let reach = |target: &Actor| mode.centre_reach(range, actor.radius, target.radius);
        let valid = |target: &Actor| {
            target.alive
                && target.id != actor.id
                && crate::rules_adapter::holds_own_team(relation, actor.owner, target.owner)
                && target.position.distance_squared(home) <= (acquire.leash + reach(target)).powi(2)
        };
        let in_range = |target: &Actor| {
            super::combat::within_weapon_range(actor.position, target.position, reach(target))
        };
        let current = acquired
            .and_then(|state| by_id.get(&state.target))
            .map(|index| &actors[*index])
            .filter(|target| valid(target));
        let nearest = |weapon_only: bool| {
            grid.within(
                &points,
                actor.position,
                mode.centre_reach(acquire.range.max(range), actor.radius, max_radius),
            )
            .into_iter()
            .map(|index| &actors[index])
            .filter(|target| {
                valid(target)
                    && if weapon_only {
                        in_range(target)
                    } else {
                        super::combat::within_weapon_range(
                            actor.position,
                            target.position,
                            mode.centre_reach(acquire.range, actor.radius, target.radius),
                        )
                    }
            })
            .min_by(|left, right| {
                left.position
                    .distance_squared(actor.position)
                    .total_cmp(&right.position.distance_squared(actor.position))
                    .then_with(|| left.position.x.total_cmp(&right.position.x))
                    .then_with(|| left.position.y.total_cmp(&right.position.y))
            })
        };
        // Keep a target already in reach. A blocked chase yields to another enemy in reach.
        let target = current
            .filter(|target| in_range(target))
            .or_else(|| nearest(true))
            .or(current)
            .or_else(|| nearest(false));
        let outside_leash =
            actor.position.distance_squared(home) > acquire.leash.powi(2) + f32::EPSILON;
        let Some(target) = target.filter(|_| !outside_leash) else {
            if acquired.is_some() {
                if actor.position.distance_squared(home) < 0.0001 {
                    write(world, actor, None, None, true);
                } else {
                    write(
                        world,
                        actor,
                        Some(AcquiredTarget {
                            home,
                            target: 0,
                            returning: true,
                        }),
                        Some(home),
                        true,
                    );
                }
            }
            continue;
        };
        let state = AcquiredTarget {
            home,
            target: target.id,
            returning: false,
        };
        let goal = if in_range(target) {
            None
        } else {
            let goal =
                super::combat::weapon_stand_off(target.position, actor.position, reach(target));
            let goal = home + (goal - home).clamp_length_max(acquire.leash);
            let goal = world
                .get_resource::<crate::MapBounds>()
                .map_or(goal, |bounds| {
                    Vec2::new(
                        goal.x.clamp(
                            actor.radius,
                            (bounds.size.x - actor.radius).max(actor.radius),
                        ),
                        goal.y.clamp(
                            actor.radius,
                            (bounds.size.y - actor.radius).max(actor.radius),
                        ),
                    )
                });
            (goal.distance_squared(actor.position) > f32::EPSILON).then_some(goal)
        };
        write(world, actor, Some(state), goal, true);
    }
}

fn write(
    world: &mut World,
    actor: Actor,
    state: Option<AcquiredTarget>,
    goal: Option<Vec2>,
    owns_movement: bool,
) {
    let current = world
        .get::<UnitOrders>(actor.entity)
        .and_then(|orders| orders.acquired);
    let old_goal = world.get::<MoveTarget>(actor.entity).map(|goal| goal.0);
    if current == state && (!owns_movement || old_goal == goal) {
        return;
    }
    super::transaction::capture_post_movement_order_operation(world, actor.id, |world| {
        let mut orders = world
            .get::<UnitOrders>(actor.entity)
            .cloned()
            .unwrap_or_default();
        orders.acquired = state;
        crate::orders::writer::set_unit_orders(world, actor.entity, orders);
        if owns_movement {
            crate::orders::forget_plain_move_identity(world, actor.id);
            let mut entity = world.entity_mut(actor.entity);
            if let Some(goal) = goal {
                if old_goal != Some(goal) {
                    entity.remove::<crate::movement::MoveProgress>();
                    entity.remove::<crate::movement::PlainMoveProgress>();
                }
                entity.insert(MoveTarget(goal));
            } else {
                entity.remove::<MoveTarget>();
                entity.remove::<crate::movement::MoveProgress>();
                entity.remove::<crate::movement::PlainMoveProgress>();
            }
        }
    });
}
