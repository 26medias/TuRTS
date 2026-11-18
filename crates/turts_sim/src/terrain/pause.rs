//! Identity guard for explicit unloading that temporarily interrupts terrain work.
//!
//! Progress, fractional carry, and claims stay in the ordinary work runtime. The
//! order writer owns queue changes and cancellation at the atomic tick boundary.
use crate::{
    Owner, RejectReason, SlotId,
    orders::{Order, OrderTargetSim, UnitOrders},
    unit_index::unit_entity,
};
use bevy_ecs::{prelude::Resource, world::World};
use std::collections::BTreeMap;
use turts_mod_api::Behaviour;

struct PausedWork {
    work: Order,
    owner: SlotId,
}

#[derive(Resource, Default)]
pub(crate) struct TerrainWorkPauses {
    units: BTreeMap<u32, PausedWork>,
}

fn live_owner(world: &World, unit: u32) -> Option<SlotId> {
    let entity = unit_entity(world, unit)?;
    if world.get::<crate::UnitId>(entity)?.0 != unit
        || world
            .get_resource::<crate::economy::combat::Healths>()
            .and_then(|health| health.0.get(&unit))
            .is_some_and(|health| *health == 0)
    {
        return None;
    }
    Some(world.get::<Owner>(entity)?.0)
}

/// Call before replacing the active work with the validated unload utility.
/// This does not clear motion, alter the queue, or move the work runtime row.
pub(crate) fn suspend(world: &mut World, unit: u32, work: &Order) -> Result<(), RejectReason> {
    if work.behaviour != Behaviour::TerrainWork
        || !matches!(work.target, OrderTargetSim::TerrainWork { .. })
    {
        return Err(RejectReason::TargetInvalid);
    }
    let owner = live_owner(world, unit).ok_or(RejectReason::UnknownUnit)?;
    let orders = unit_entity(world, unit).and_then(|entity| world.get::<UnitOrders>(entity));
    if !orders.is_some_and(|orders| orders.current.as_ref() == Some(work)) {
        return Err(RejectReason::TargetInvalid);
    }
    world.init_resource::<TerrainWorkPauses>();
    let mut pauses = world.resource_mut::<TerrainWorkPauses>();
    match pauses.units.entry(unit) {
        std::collections::btree_map::Entry::Occupied(_) => Err(RejectReason::NotQueueable),
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(PausedWork {
                work: work.clone(),
                owner,
            });
            Ok(())
        }
    }
}

fn saved_orders(world: &World, unit: u32) -> Option<(&PausedWork, &UnitOrders)> {
    let saved = world
        .get_resource::<TerrainWorkPauses>()?
        .units
        .get(&unit)?;
    if live_owner(world, unit)? != saved.owner {
        return None;
    }
    let orders = world.get::<UnitOrders>(unit_entity(world, unit)?)?;
    Some((saved, orders))
}

/// Raw membership lets cleanup distinguish an invalid pause from ordinary work.
pub(crate) fn has_record(world: &World, unit: u32) -> bool {
    world
        .get_resource::<TerrainWorkPauses>()
        .is_some_and(|pauses| pauses.units.contains_key(&unit))
}

/// Retain the ordinary work row while this explicit utility occupies its slot.
pub(crate) fn is_suspended(world: &World, unit: u32) -> bool {
    saved_orders(world, unit).is_some_and(|(saved, orders)| {
        orders
            .current
            .as_ref()
            .is_some_and(|order| order.behaviour == Behaviour::UnloadCargo)
            && orders.queue.front() == Some(&saved.work)
    })
}

/// A failed/completed utility may leave an empty current slot before queue promotion.
pub(crate) fn has_pending_resume(world: &World, unit: u32) -> bool {
    saved_orders(world, unit).is_some_and(|(saved, orders)| {
        orders.current.is_none() && orders.queue.front() == Some(&saved.work)
    })
}

/// Consume the guard only after the exact original work becomes current again.
pub(crate) fn consume_resume(world: &mut World, unit: u32) -> bool {
    let valid = saved_orders(world, unit)
        .is_some_and(|(saved, orders)| orders.current.as_ref() == Some(&saved.work));
    if valid {
        cancel(world, unit);
    }
    valid
}

/// The caller separately discards work progress, carry, and claims on cancellation.
pub(crate) fn cancel(world: &mut World, unit: u32) {
    if let Some(mut pauses) = world.get_resource_mut::<TerrainWorkPauses>() {
        pauses.units.remove(&unit);
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TerrainPauseAudit {
    pub unit: u32,
    pub owner: SlotId,
    pub work: Order,
}
pub(super) fn audit(world: &World) -> Vec<TerrainPauseAudit> {
    world
        .get_resource::<TerrainWorkPauses>()
        .into_iter()
        .flat_map(|pauses| pauses.units.iter())
        .map(|(unit, saved)| TerrainPauseAudit {
            unit: *unit,
            owner: saved.owner,
            work: saved.work.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::work::{TerrainWorkRuntime, TerrainWorkWait, WorkState};
    use turts_mod_api::{ContentId, TerrainCell, TerrainRect};
    use turts_rules::terrain::geometry::Cell;

    fn work() -> Order {
        Order {
            order: ContentId::parse("quarry:dig_order").unwrap(),
            behaviour: Behaviour::TerrainWork,
            target: OrderTargetSim::TerrainWork {
                tool: ContentId::parse("quarry:bucket").unwrap(),
                cells: TerrainRect {
                    min: TerrainCell { x: 2, y: 3 },
                    max: TerrainCell { x: 4, y: 5 },
                },
                target_elevation: -2,
            },
            group: 91,
        }
    }

    fn unload() -> Order {
        Order {
            order: ContentId::parse("quarry:unload_order").unwrap(),
            behaviour: Behaviour::UnloadCargo,
            target: OrderTargetSim::Unit(42),
            group: 7,
        }
    }

    fn scene() -> (World, bevy_ecs::entity::Entity, WorkState, Order) {
        let mut world = World::new();
        let pending = Order {
            order: ContentId::parse("quarry:move_order").unwrap(),
            behaviour: Behaviour::Move,
            target: OrderTargetSim::Point(bevy_math::Vec2::new(8.25, 9.5)),
            group: 13,
        };
        let entity = world
            .spawn((
                crate::UnitId(17),
                Owner(SlotId(1)),
                UnitOrders {
                    current: Some(work()),
                    queue: [pending.clone()].into(),
                    ..Default::default()
                },
            ))
            .id();
        world.flush();
        let progress = WorkState {
            owner: SlotId(1),
            order: work(),
            cell: Some(TerrainCell { x: 2, y: 3 }),
            done: 17,
            carry: 3,
            banked: true,
            required: Some(23),
            wait: TerrainWorkWait::CargoFull,
            source_revision: Some(9),
            approach: None,
        };
        world.insert_resource(TerrainWorkRuntime {
            units: [(17, progress.clone())].into(),
            claims: [(Cell { x: 2, y: 3 }, 17)].into(),
        });
        (world, entity, progress, pending)
    }

    fn install_utility(world: &mut World, entity: bevy_ecs::entity::Entity) {
        let mut orders = world.get_mut::<UnitOrders>(entity).unwrap();
        let original = orders.current.replace(unload()).unwrap();
        orders.queue.push_front(original);
    }

    #[test]
    fn unload_round_trip_preserves_exact_work_progress_carry_claim_and_pending_queue() {
        let (mut world, entity, progress, pending) = scene();
        suspend(&mut world, 17, &work()).unwrap();
        install_utility(&mut world, entity);
        assert!(is_suspended(&world, 17));
        let visible = super::super::work::rows_for(&world, 1);
        assert_eq!(visible[0].wait, TerrainWorkWait::Unloading);
        assert_eq!(visible[0].completed_work, progress.done);
        assert_eq!(visible[0].current_cell, progress.cell);
        assert_eq!(visible[0].required_work, progress.required);
        assert!(!consume_resume(&mut world, 17));
        assert_eq!(world.resource::<TerrainWorkRuntime>().units[&17], progress);
        {
            let mut orders = world.get_mut::<UnitOrders>(entity).unwrap();
            orders.current = None;
        }
        assert!(has_pending_resume(&world, 17));
        assert!(!is_suspended(&world, 17));
        {
            let mut orders = world.get_mut::<UnitOrders>(entity).unwrap();
            orders.current = orders.queue.pop_front();
        }
        assert!(consume_resume(&mut world, 17));
        assert_eq!(
            super::super::work::rows_for(&world, 1)[0].wait,
            progress.wait
        );
        assert!(!consume_resume(&mut world, 17));
        assert_eq!(world.resource::<TerrainWorkRuntime>().units[&17], progress);
        assert_eq!(
            world.resource::<TerrainWorkRuntime>().claims[&Cell { x: 2, y: 3 }],
            17
        );
        assert_eq!(
            world.get::<UnitOrders>(entity).unwrap().current,
            Some(work())
        );
        assert_eq!(
            world.get::<UnitOrders>(entity).unwrap().queue,
            std::collections::VecDeque::from([pending])
        );
    }

    #[test]
    fn changed_identity_or_queue_front_cannot_resume_another_work_order() {
        let (mut world, entity, _, pending) = scene();
        let mut other = work();
        other.group += 1;
        assert_eq!(
            suspend(&mut world, 17, &other),
            Err(RejectReason::TargetInvalid)
        );
        suspend(&mut world, 17, &work()).unwrap();
        assert_eq!(
            suspend(&mut world, 17, &work()),
            Err(RejectReason::NotQueueable)
        );
        install_utility(&mut world, entity);
        world
            .get_mut::<UnitOrders>(entity)
            .unwrap()
            .queue
            .push_front(pending);
        assert!(!is_suspended(&world, 17));
        world.get_mut::<UnitOrders>(entity).unwrap().current = None;
        assert!(!has_pending_resume(&world, 17));
        world.get_mut::<UnitOrders>(entity).unwrap().current = Some(other);
        assert!(!consume_resume(&mut world, 17));
    }

    #[test]
    fn ownership_loss_death_and_destruction_invalidate_the_guard() {
        for invalidation in 0..3 {
            let (mut world, entity, _, _) = scene();
            suspend(&mut world, 17, &work()).unwrap();
            install_utility(&mut world, entity);
            match invalidation {
                0 => {
                    world.entity_mut(entity).insert(Owner(SlotId(2)));
                }
                1 => {
                    world.insert_resource(crate::economy::combat::Healths([(17, 0)].into()));
                }
                _ => {
                    world.despawn(entity);
                }
            }
            assert!(has_record(&world, 17));
            assert!(!is_suspended(&world, 17));
            assert!(!has_pending_resume(&world, 17));
            assert!(!consume_resume(&mut world, 17));
        }
    }

    #[test]
    fn cancel_removes_only_guard_and_does_not_own_work_progress() {
        let (mut world, entity, progress, _) = scene();
        suspend(&mut world, 17, &work()).unwrap();
        install_utility(&mut world, entity);
        cancel(&mut world, 17);
        assert!(!has_record(&world, 17));
        assert!(!is_suspended(&world, 17));
        assert_eq!(world.resource::<TerrainWorkRuntime>().units[&17], progress);
        assert_eq!(world.resource::<TerrainWorkRuntime>().claims.len(), 1);
    }
}
