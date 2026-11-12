//! The `UnitId` index (design section 9, "UnitId index in lane 0").
//!
//! One resource maps a `UnitId` to its entity. The component hooks of `UnitId` (`world.rs`) keep
//! it: one insert for each unit that gets an id and one remove for each unit that loses it, on
//! every spawn and despawn path, tests included. A lookup that used to scan all entities asks
//! the index in one hash step.
//!
//! One id has one holder: a second spawn of a held id panics in the insert hook.
//!
//! The index is for lookup only. It has no iterator, so no system can take a tick order from
//! the hash order, and the tick order does not depend on it.

use bevy_ecs::{
    entity::Entity,
    lifecycle::HookContext,
    prelude::Resource,
    world::{DeferredWorld, World},
};
use std::collections::HashMap;

use crate::world::UnitId;

/// A map from `UnitId` to the entity that holds it. `SimPlugin` inserts it.
#[derive(Resource, Debug, Default)]
pub struct UnitIndex {
    entities: HashMap<u32, Entity>,
}

impl UnitIndex {
    /// The entity of unit `id`, or `None` when no unit has that id.
    pub fn get(&self, id: u32) -> Option<Entity> {
        self.entities.get(&id).copied()
    }

    /// The number of indexed units.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

/// The entity of unit `id`, or `None` when no unit has the id.
pub fn unit_entity(world: &World, id: u32) -> Option<Entity> {
    world.get_resource::<UnitIndex>()?.get(id)
}

impl UnitIndex {
    /// Record `entity` as the holder of `id`. One id has one holder: a second holder would make
    /// the index lose the older one, so it is a fault, not a last-writer-wins.
    fn claim(&mut self, id: u32, entity: Entity) {
        if let Some(held) = self.entities.insert(id, entity)
            && held != entity
        {
            panic!("second holder of UnitId {id}: {entity:?} while {held:?} holds it");
        }
    }
}

/// The `on_insert` hook of `UnitId`: a unit got its id (a spawn, or a replacement of the id).
///
/// A world that `SimPlugin` did not build (a unit test of one transaction) has no index yet:
/// the first unit makes it, in a command that runs when the spawn ends. So the index equals a
/// full scan in every world, and a lookup never needs a scan as a fallback.
pub(crate) fn on_unit_id_insert(mut world: DeferredWorld, context: HookContext) {
    let Some(id) = world.get::<UnitId>(context.entity).map(|id| id.0) else {
        return;
    };
    let entity = context.entity;
    if let Some(mut index) = world.get_resource_mut::<UnitIndex>() {
        index.claim(id, entity);
    } else {
        world.commands().queue(move |world: &mut World| {
            world
                .get_resource_or_insert_with(UnitIndex::default)
                .claim(id, entity);
        });
    }
}

/// The `on_discard` hook of `UnitId`: the unit loses its id (a despawn, a removal of the
/// component, or a replacement of the id; the old value is still readable here). It removes the
/// entry only when the entry is this entity (a test may point an entry at a bare entity).
pub(crate) fn on_unit_id_discard(mut world: DeferredWorld, context: HookContext) {
    let Some(id) = world.get::<UnitId>(context.entity).map(|id| id.0) else {
        return;
    };
    if let Some(mut index) = world.get_resource_mut::<UnitIndex>()
        && index.entities.get(&id) == Some(&context.entity)
    {
        index.entities.remove(&id);
    }
}

/// Test hook: point the index entry of `id` at a new bare entity (no component), whether or not
/// a unit with that id exists. A lookup that scans the world answers with the real unit (or
/// with none); a lookup that reads the index answers with the bare entity. A site that the
/// plan moved to the index must give the second answer.
#[cfg(test)]
pub(crate) fn index_entry_for_test(world: &mut World, id: u32) -> Entity {
    let bare = world.spawn_empty().id();
    world.resource_mut::<UnitIndex>().entities.insert(id, bare);
    bare
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Owner, Position, SlotId, headless_app};
    use bevy_math::Vec2;

    fn unit(id: u32) -> (UnitId, Owner, Position) {
        (UnitId(id), Owner(SlotId(0)), Position(Vec2::ZERO))
    }

    /// The units that a full scan of the world finds, sorted by id.
    fn scan(world: &World) -> Vec<(u32, Entity)> {
        let mut found = world
            .iter_entities()
            .filter_map(|entity| Some((entity.get::<UnitId>()?.0, entity.id())))
            .collect::<Vec<_>>();
        found.sort_unstable();
        found
    }

    /// The entries of the index, sorted by id. Only the tests may iterate the map.
    fn entries(world: &World) -> Vec<(u32, Entity)> {
        let mut found = world
            .resource::<UnitIndex>()
            .entities
            .iter()
            .map(|(id, entity)| (*id, *entity))
            .collect::<Vec<_>>();
        found.sort_unstable();
        found
    }

    #[test]
    fn a_lookup_of_a_removed_id_gives_no_entity() {
        let mut app = headless_app(20);
        let kept = app.world_mut().spawn(unit(1)).id();
        let removed = app.world_mut().spawn(unit(2)).id();
        assert_eq!(unit_entity(app.world(), 1), Some(kept));
        assert_eq!(unit_entity(app.world(), 2), Some(removed));
        assert!(app.world_mut().despawn(removed));
        assert_eq!(
            unit_entity(app.world(), 2),
            None,
            "the index kept a removed unit"
        );
        assert_eq!(unit_entity(app.world(), 1), Some(kept));
        assert_eq!(unit_entity(app.world(), 3), None);
    }

    #[test]
    fn the_index_equals_a_full_scan_after_every_spawn_and_despawn_of_a_long_scenario() {
        let mut app = headless_app(20);
        // A fixed linear congruential stream: the scenario repeats, and no random source of the
        // process is used.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move |bound: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) % bound
        };
        let mut live: Vec<Entity> = Vec::new();
        let mut next_id = 1_u32;
        let mut checked = 0_u32;
        for step in 0..600 {
            let world = app.world_mut();
            match next(5) {
                // Spawn, with a fresh id.
                0..=1 => {
                    live.push(world.spawn(unit(next_id)).id());
                    next_id += 1;
                }
                // Despawn a live entity.
                2 if !live.is_empty() => {
                    let at = usize::try_from(next(live.len() as u64)).unwrap();
                    assert!(world.despawn(live.swap_remove(at)));
                }
                // Give a live entity another id (the old id must leave the index).
                3 if !live.is_empty() => {
                    let at = usize::try_from(next(live.len() as u64)).unwrap();
                    world.entity_mut(live[at]).insert(UnitId(next_id));
                    next_id += 1;
                }
                // Take the id off a live entity; the entity stays and is no longer a unit.
                _ if !live.is_empty() => {
                    let at = usize::try_from(next(live.len() as u64)).unwrap();
                    world.entity_mut(live.swap_remove(at)).remove::<UnitId>();
                }
                _ => {}
            }
            assert_eq!(
                entries(app.world()),
                scan(app.world()),
                "the index differs from a full scan at step {step}"
            );
            checked += 1;
        }
        assert_eq!(checked, 600);
        assert!(!app.world().resource::<UnitIndex>().is_empty());
    }

    #[test]
    fn a_world_that_simplugin_did_not_build_gets_its_index_from_its_first_unit() {
        let mut world = World::new();
        assert_eq!(unit_entity(&world, 1), None);
        let first = world.spawn(unit(1)).id();
        assert_eq!(unit_entity(&world, 1), Some(first));
        let second = world.spawn(unit(2)).id();
        assert_eq!(entries(&world), scan(&world));
        assert!(world.despawn(first));
        assert_eq!(unit_entity(&world, 1), None);
        assert_eq!(unit_entity(&world, 2), Some(second));
    }

    #[test]
    #[should_panic(expected = "second holder of UnitId 4")]
    fn a_second_holder_of_one_id_panics() {
        // One id has one holder (design section 9). A second holder would make the index lose
        // the older one, so the insert hook stops the world at the second spawn.
        let mut app = headless_app(20);
        app.world_mut().spawn(unit(4));
        app.world_mut().spawn(unit(4));
    }

    #[test]
    #[should_panic(expected = "second holder of UnitId 4")]
    fn a_second_holder_panics_in_a_world_that_simplugin_did_not_build() {
        // The first unit makes the index in a queued command; the second spawn checks it.
        let mut world = World::new();
        world.spawn(unit(4));
        world.spawn(unit(4));
    }

    #[test]
    fn the_same_holder_may_insert_its_own_id_again() {
        let mut app = headless_app(20);
        let holder = app.world_mut().spawn(unit(4)).id();
        app.world_mut().entity_mut(holder).insert(UnitId(4));
        assert_eq!(unit_entity(app.world(), 4), Some(holder));
        assert!(app.world_mut().despawn(holder));
        assert_eq!(unit_entity(app.world(), 4), None);
        // The id is free again after the holder is gone.
        let next = app.world_mut().spawn(unit(4)).id();
        assert_eq!(unit_entity(app.world(), 4), Some(next));
    }

    #[test]
    fn the_unit_id_component_is_immutable() {
        // A `get_mut::<UnitId>` would change the id with no hook, and the index would keep the
        // old id (review of L0-SIM, M2). An immutable component can only be replaced by an
        // insert, which runs both hooks.
        use bevy_ecs::component::{Component, Immutable};
        assert_eq!(
            std::any::TypeId::of::<<UnitId as Component>::Mutability>(),
            std::any::TypeId::of::<Immutable>()
        );
    }

    #[test]
    fn a_lookup_answers_from_the_map_and_does_not_scan() {
        let mut app = headless_app(20);
        let real = app.world_mut().spawn(unit(9)).id();
        let bare = index_entry_for_test(app.world_mut(), 9);
        assert_ne!(real, bare);
        assert_eq!(unit_entity(app.world(), 9), Some(bare));
    }

    /// Record the cost of a scan and of a map lookup (plan task `L0-SIM`; the figures go into
    /// `docs/measurements/mp1.md`). `cargo test -p turts_sim --release --locked --lib
    /// unit_index::tests::lookup_cost -- --ignored --nocapture`.
    #[test]
    #[ignore = "records timings; run by hand"]
    fn lookup_cost() {
        for units in [1_000_u32, 5_000] {
            let mut app = headless_app(20);
            for id in 1..=units {
                app.world_mut().spawn(unit(id));
            }
            let probes = 200_u32;
            let wanted = units; // the last unit: the worst case of a scan
            let start = std::time::Instant::now();
            let mut hits = 0_u32;
            for _ in 0..probes {
                hits += u32::from(
                    app.world()
                        .iter_entities()
                        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == wanted))
                        .is_some(),
                );
            }
            let scan_ns = start.elapsed().as_nanos() / u128::from(probes);
            let start = std::time::Instant::now();
            for _ in 0..probes * 1_000 {
                hits += u32::from(unit_entity(app.world(), wanted).is_some());
            }
            let map_ns = start.elapsed().as_nanos() / u128::from(probes * 1_000);
            println!("units={units} scan_ns={scan_ns} map_ns={map_ns} hits={hits}");
        }
    }
}
