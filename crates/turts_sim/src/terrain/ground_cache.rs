//! Privileged physical-support memoization. No diagnostics or leases enter a public phase.
use super::TerrainState;
use bevy_ecs::{prelude::Resource, world::World};
use bevy_math::Vec2;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use turts_rules::terrain::geometry::{Grid, Support};
const ENTRIES: usize = 4096;
const CHUNKS: usize = 64;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Key([u32; 5]);
impl Key {
    pub(super) fn new(
        kind: turts_mod_api::UnitTypeId,
        position: Vec2,
        radius: f32,
        max_step: u32,
    ) -> Self {
        Self([
            kind.0,
            position.x.to_bits(),
            position.y.to_bits(),
            radius.to_bits(),
            max_step,
        ])
    }
}
struct Entry {
    key: Key,
    maximum: i32,
    leases: Vec<(usize, u64)>,
}
struct Identity {
    grid: Grid,
    initial: Arc<[i32]>,
}
#[derive(Default)]
struct Cache {
    identity: Option<Identity>,
    entries: BTreeMap<u32, Entry>,
    pruned_tick: Option<u32>,
}
impl Cache {
    fn scope(&mut self, terrain: &TerrainState) {
        if self.identity.as_ref().is_none_or(|identity| {
            identity.grid != terrain.grid || !Arc::ptr_eq(&identity.initial, &terrain.initial)
        }) {
            self.entries.clear();
            self.pruned_tick = None;
            // Exactly one map allocation is retained, never one Arc per unit row.
            self.identity = Some(Identity {
                grid: terrain.grid,
                initial: Arc::clone(&terrain.initial),
            });
        }
    }
    fn get(&self, terrain: &TerrainState, unit: u32, key: Key) -> Option<i32> {
        let entry = self.entries.get(&unit)?;
        (entry.key == key
            && entry.leases.iter().all(|&(chunk, revision)| {
                terrain
                    .chunks
                    .get(chunk)
                    .is_some_and(|value| value.revision == revision)
            }))
        .then_some(entry.maximum)
    }
    fn insert(&mut self, terrain: &TerrainState, unit: u32, key: Key, support: &Support) {
        if self.entries.len() == ENTRIES && !self.entries.contains_key(&unit) {
            return;
        }
        let mut leases: Vec<(usize, u64)> = Vec::new();
        // These are the actual exact disk-support cells, not a bounding rectangle.
        for &cell in &support.cells {
            let Some((chunk, _)) = terrain.chunk_location(cell) else {
                return;
            };
            match leases.binary_search_by_key(&chunk, |entry| entry.0) {
                Ok(_) => {}
                Err(index) => {
                    if leases.len() == CHUNKS || leases.try_reserve_exact(1).is_err() {
                        return;
                    }
                    let Some(value) = terrain.chunks.get(chunk) else {
                        return;
                    };
                    leases.insert(index, (chunk, value.revision));
                }
            }
        }
        self.entries.insert(
            unit,
            Entry {
                key,
                maximum: support.maximum,
                leases,
            },
        );
    }
}
/// Interior mutation serves read-only phase capture. Contention or poison is an exact miss;
/// no thread waits for another capture, and no simulation answer depends on cache service.
#[derive(Resource, Default)]
pub(super) struct GroundSupportCache(Mutex<Cache>);
pub(super) fn lookup(world: &World, terrain: &TerrainState, unit: u32, key: Key) -> Option<i32> {
    let cache = world.get_resource::<GroundSupportCache>()?;
    let mut cache = cache.0.try_lock().ok()?;
    cache.scope(terrain);
    let tick = world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0);
    if cache.pruned_tick != Some(tick) {
        let index = world.get_resource::<crate::UnitIndex>()?;
        cache.entries.retain(|id, _| index.get(*id).is_some());
        cache.pruned_tick = Some(tick);
    }
    cache.get(terrain, unit, key)
}
pub(super) fn remember(
    world: &World,
    terrain: &TerrainState,
    unit: u32,
    key: Key,
    support: &Support,
) {
    let Some(cache) = world.get_resource::<GroundSupportCache>() else {
        return;
    };
    let Ok(mut cache) = cache.0.try_lock() else {
        return;
    };
    cache.scope(terrain);
    cache.insert(terrain, unit, key, support);
}
#[cfg(test)]
#[path = "ground_cache_tests.rs"]
mod tests;
