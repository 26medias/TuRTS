//! The index of a movement pool and its slot clearances.

use bevy_math::Vec2;
use std::collections::BTreeMap;

use super::MovementPool;
#[cfg(test)]
use super::count_pool_slot_candidates;
use super::frame::MovementFrameUnit;
use super::spread::nearest_angle_order;

pub(super) struct MovementPoolIndex {
    pub(super) points: Vec<crate::grid::GridPoint>,
    grid: crate::grid::UniformGrid,
    slot_indices: std::collections::HashMap<(u32, u32), usize>,
    pub(super) clearances: Vec<SlotClearance>,
    pub(super) available_by_clearance: std::collections::BTreeSet<ClearanceRank>,
    pub(super) available_slot_indices: std::collections::HashSet<usize>,
    spacing: f32,
    max_reach: f32,
}

#[derive(Debug, Clone, Copy)]
struct ClearanceHit {
    clearance: f64,
    unit_id: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SlotClearance {
    first: ClearanceHit,
    second: ClearanceHit,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ClearanceRank {
    pub(super) clearance: f64,
    pub(super) slot_index: usize,
}

impl PartialEq for ClearanceRank {
    fn eq(&self, other: &Self) -> bool {
        self.clearance.total_cmp(&other.clearance).is_eq() && self.slot_index == other.slot_index
    }
}

impl Eq for ClearanceRank {}

impl PartialOrd for ClearanceRank {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ClearanceRank {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.clearance
            .total_cmp(&other.clearance)
            .then_with(|| self.slot_index.cmp(&other.slot_index))
    }
}

impl SlotClearance {
    pub(super) fn empty() -> Self {
        let empty = ClearanceHit {
            clearance: f64::INFINITY,
            unit_id: None,
        };
        Self {
            first: empty,
            second: empty,
        }
    }
}

impl MovementPoolIndex {
    pub(super) fn new(pool: &MovementPool, bounds: Vec2) -> Self {
        let points: Vec<_> = pool
            .slots
            .iter()
            .enumerate()
            .map(|(index, &position)| crate::grid::GridPoint {
                id: index as u32,
                position,
            })
            .collect();
        let grid = crate::grid::UniformGrid::new(&points, pool.spacing);
        let slot_indices = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                (
                    (point.position.x.to_bits(), point.position.y.to_bits()),
                    index,
                )
            })
            .collect();
        Self {
            points,
            grid,
            slot_indices,
            clearances: vec![SlotClearance::empty(); pool.slots.len()],
            available_by_clearance: std::collections::BTreeSet::new(),
            available_slot_indices: std::collections::HashSet::new(),
            spacing: pool.spacing.max(0.1),
            max_reach: bounds.length(),
        }
    }

    pub(super) fn refresh_slot_clearance(
        &mut self,
        slot_index: usize,
        units: &[MovementFrameUnit],
        points: &[crate::grid::GridPoint],
        grid: &crate::grid::UniformGrid,
        max_radius: f32,
        assigned: bool,
    ) {
        if !assigned {
            self.available_by_clearance.remove(&ClearanceRank {
                clearance: self.clearances[slot_index].first.clearance,
                slot_index,
            });
        }
        let slot = self.points[slot_index].position;
        let mut nearest = [SlotClearance::empty().first; 2];
        for unit_index in grid.within(points, slot, 2.0 * max_radius) {
            let unit = &units[unit_index];
            let clearance = f64::from(slot.distance(unit.position)) - f64::from(unit.radius);
            let hit = ClearanceHit {
                clearance: if clearance > f64::from(max_radius) {
                    f64::INFINITY
                } else {
                    clearance
                },
                unit_id: (clearance <= f64::from(max_radius)).then_some(unit.unit_id),
            };
            let earlier = |left: ClearanceHit, right: ClearanceHit| {
                left.clearance
                    .total_cmp(&right.clearance)
                    .then_with(|| left.unit_id.cmp(&right.unit_id))
                    .is_lt()
            };
            if earlier(hit, nearest[0]) {
                nearest[1] = nearest[0];
                nearest[0] = hit;
            } else if earlier(hit, nearest[1]) {
                nearest[1] = hit;
            }
        }
        let clearance = SlotClearance {
            first: nearest[0],
            second: nearest[1],
        };
        self.clearances[slot_index] = clearance;
        if !assigned {
            self.available_by_clearance.insert(ClearanceRank {
                clearance: clearance.first.clearance,
                slot_index,
            });
            self.available_slot_indices.insert(slot_index);
        } else {
            self.available_slot_indices.remove(&slot_index);
        }
    }

    fn set_slot_assigned(&mut self, slot_index: usize, assigned: bool) {
        if assigned {
            if self.available_slot_indices.remove(&slot_index) {
                self.available_by_clearance.remove(&ClearanceRank {
                    clearance: self.clearances[slot_index].first.clearance,
                    slot_index,
                });
            }
        } else if self.available_slot_indices.insert(slot_index) {
            self.available_by_clearance.insert(ClearanceRank {
                clearance: self.clearances[slot_index].first.clearance,
                slot_index,
            });
        }
    }

    pub(super) fn has_potential_free_slot(
        &self,
        unit_id: u32,
        position: Vec2,
        radius: f32,
        allowed_slot: Option<(u32, u32)>,
    ) -> bool {
        let threshold = f64::from(radius) - 1.0e-5;
        if self
            .available_by_clearance
            .last()
            .is_some_and(|rank| rank.clearance >= threshold)
        {
            return true;
        }
        if let Some(slot_key) = allowed_slot
            && let Some(&slot_index) = self.slot_indices.get(&slot_key)
        {
            let clearance = self.clearances[slot_index];
            let nearest_without_self = if clearance.first.unit_id == Some(unit_id) {
                clearance.second
            } else {
                clearance.first
            };
            if nearest_without_self.clearance >= threshold {
                return true;
            }
        }
        let local = self.grid.within(&self.points, position, 2.0 * radius);
        #[cfg(test)]
        count_pool_slot_candidates(local.len());
        local.into_iter().any(|slot_index| {
            if !self.available_slot_indices.contains(&slot_index) {
                return false;
            }
            let clearance = self.clearances[slot_index];
            clearance.first.unit_id == Some(unit_id) && clearance.second.clearance >= threshold
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "pool candidate lookup needs exact caller, reservation and collision context"
    )]
    pub(super) fn nearest_free(
        &self,
        unit_id: u32,
        origin: Vec2,
        pool_id: u64,
        allowed_slot: Option<(u64, u32, u32)>,
        assigned_slots: &std::collections::HashSet<(u64, u32, u32)>,
        radius: f32,
        is_free: impl Fn(Vec2) -> bool,
    ) -> Option<Vec2> {
        if !self.has_potential_free_slot(
            unit_id,
            origin,
            radius,
            allowed_slot.map(|(_, x, y)| (x, y)),
        ) {
            return None;
        }
        let mut reach = self.spacing;
        loop {
            let query_reach = reach.min(self.max_reach);
            let mut best: Option<(usize, Vec2)> = None;
            let candidates = self.grid.within(&self.points, origin, query_reach);
            #[cfg(test)]
            count_pool_slot_candidates(candidates.len());
            for index in candidates {
                let point = self.points[index].position;
                let key = (pool_id, point.x.to_bits(), point.y.to_bits());
                if (assigned_slots.contains(&key) && Some(key) != allowed_slot) || !is_free(point) {
                    continue;
                }
                if best.is_none_or(|(best_index, best_point)| {
                    nearest_angle_order(point, best_point, origin)
                        .then_with(|| index.cmp(&best_index))
                        .is_lt()
                }) {
                    best = Some((index, point));
                }
            }
            if let Some((_, point)) = best
                && point.distance(origin) <= query_reach + 1.0e-5
            {
                return Some(point);
            }
            if query_reach >= self.max_reach {
                return None;
            }
            reach = (reach * 2.0).min(self.max_reach);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "incremental clearance refresh receives the shared frame indexes and claim state"
)]
pub(super) fn refresh_nearby_pool_slot_clearances(
    old_position: Vec2,
    new_position: Vec2,
    body_radius: f32,
    slot_points: &[crate::grid::GridPoint],
    slot_refs: &[(u64, usize)],
    slot_grid: &crate::grid::UniformGrid,
    pool_indexes: &mut BTreeMap<u64, MovementPoolIndex>,
    units: &[MovementFrameUnit],
    unit_points: &[crate::grid::GridPoint],
    unit_grid: &crate::grid::UniformGrid,
    assigned_slots: &std::collections::HashSet<(u64, u32, u32)>,
    max_radius: f32,
) {
    let reach = body_radius + max_radius;
    let mut affected = slot_grid
        .within(slot_points, old_position, reach)
        .into_iter()
        .chain(slot_grid.within(slot_points, new_position, reach))
        .collect::<Vec<_>>();
    affected.sort_unstable();
    affected.dedup();
    for global_index in affected {
        let (pool_id, slot_index) = slot_refs[global_index];
        let Some(index) = pool_indexes.get_mut(&pool_id) else {
            continue;
        };
        let point = index.points[slot_index].position;
        let assigned = assigned_slots.contains(&(pool_id, point.x.to_bits(), point.y.to_bits()));
        index.refresh_slot_clearance(
            slot_index,
            units,
            unit_points,
            unit_grid,
            max_radius,
            assigned,
        );
    }
}

pub(super) fn add_pool_slot_claim(
    claim_counts: &mut std::collections::HashMap<(u64, u32, u32), usize>,
    pool_indexes: &mut BTreeMap<u64, MovementPoolIndex>,
    pool_id: u64,
    point: Vec2,
) {
    let key = (pool_id, point.x.to_bits(), point.y.to_bits());
    let count = claim_counts.entry(key).or_default();
    if *count == 0
        && let Some(index) = pool_indexes.get_mut(&pool_id)
        && let Some(&slot_index) = index.slot_indices.get(&(key.1, key.2))
    {
        index.set_slot_assigned(slot_index, true);
    }
    *count += 1;
}

pub(super) fn remove_pool_slot_claim(
    claim_counts: &mut std::collections::HashMap<(u64, u32, u32), usize>,
    pool_indexes: &mut BTreeMap<u64, MovementPoolIndex>,
    pool_id: u64,
    point: Vec2,
) -> bool {
    let key = (pool_id, point.x.to_bits(), point.y.to_bits());
    let Some(count) = claim_counts.get_mut(&key) else {
        return false;
    };
    if *count > 1 {
        *count -= 1;
        return false;
    }
    claim_counts.remove(&key);
    if let Some(index) = pool_indexes.get_mut(&pool_id)
        && let Some(&slot_index) = index.slot_indices.get(&(key.1, key.2))
    {
        index.set_slot_assigned(slot_index, false);
    }
    true
}
