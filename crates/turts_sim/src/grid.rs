//! Deterministic uniform-grid neighbour lookup for flat-ground simulation.

use std::collections::HashMap;

use bevy_math::Vec2;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GridPoint {
    pub id: u32,
    pub position: Vec2,
}

/// Neighbour index with a cell width chosen by the caller. Results are always
/// sorted by unit id, so hash bucket order cannot affect simulation decisions.
pub(crate) struct UniformGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
    #[cfg(test)]
    visited_cells: std::sync::atomic::AtomicUsize,
}

impl UniformGrid {
    pub(crate) fn new(points: &[GridPoint], cell_size: f32) -> Self {
        let cell_size = cell_size.max(f32::EPSILON);
        let mut grid = Self {
            cell_size,
            cells: HashMap::with_capacity(points.len()),
            #[cfg(test)]
            visited_cells: std::sync::atomic::AtomicUsize::new(0),
        };
        for (index, point) in points.iter().enumerate() {
            grid.insert(index, point.position);
        }
        grid
    }

    pub(crate) fn insert(&mut self, index: usize, position: Vec2) {
        self.cells
            .entry(cell(position, self.cell_size))
            .or_default()
            .push(index);
    }

    pub(crate) fn move_point(&mut self, index: usize, from: Vec2, to: Vec2) {
        let old_cell = cell(from, self.cell_size);
        let new_cell = cell(to, self.cell_size);
        if old_cell == new_cell {
            return;
        }
        if let Some(indices) = self.cells.get_mut(&old_cell) {
            if let Some(at) = indices.iter().position(|candidate| *candidate == index) {
                indices.swap_remove(at);
            }
            if indices.is_empty() {
                self.cells.remove(&old_cell);
            }
        }
        self.insert(index, to);
    }

    pub(crate) fn within(&self, points: &[GridPoint], position: Vec2, reach: f32) -> Vec<usize> {
        let (cx, cy) = cell(position, self.cell_size);
        let rings = (reach / self.cell_size).ceil().max(0.0) as i64;
        let min_x = i64::from(cx) - rings;
        let max_x = i64::from(cx) + rings;
        let min_y = i64::from(cy) - rings;
        let max_y = i64::from(cy) + rings;
        let width = (max_x - min_x + 1) as u128;
        let height = (max_y - min_y + 1) as u128;
        let sparse_scan = width.saturating_mul(height) > self.cells.len() as u128;
        let mut found = Vec::new();
        #[cfg(test)]
        self.visited_cells
            .store(0, std::sync::atomic::Ordering::Relaxed);
        if sparse_scan {
            for (&(x, y), indices) in &self.cells {
                #[cfg(test)]
                self.visited_cells
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i64::from(x) >= min_x
                    && i64::from(x) <= max_x
                    && i64::from(y) >= min_y
                    && i64::from(y) <= max_y
                {
                    found.extend(indices.iter().copied().filter(|index| {
                        points[*index].position.distance_squared(position) <= reach * reach
                    }));
                }
            }
        } else {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    #[cfg(test)]
                    self.visited_cells
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(indices) = i32::try_from(x)
                        .ok()
                        .zip(i32::try_from(y).ok())
                        .and_then(|key| self.cells.get(&key))
                    else {
                        continue;
                    };
                    found.extend(indices.iter().copied().filter(|index| {
                        points[*index].position.distance_squared(position) <= reach * reach
                    }));
                }
            }
        }
        found.sort_unstable_by_key(|index| (points[*index].id, *index));
        found
    }

    #[cfg(test)]
    fn visited_cells(&self) -> usize {
        self.visited_cells
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

fn cell(position: Vec2, size: f32) -> (i32, i32) {
    (
        (position.x / size).floor() as i32,
        (position.y / size).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::{GridPoint, UniformGrid};
    use bevy_math::Vec2;

    #[test]
    fn grid_returns_neighbours_in_unit_id_order() {
        let points = [
            GridPoint {
                id: 9,
                position: Vec2::new(0.4, 0.0),
            },
            GridPoint {
                id: 2,
                position: Vec2::new(-0.4, 0.0),
            },
            GridPoint {
                id: 1,
                position: Vec2::new(2.0, 0.0),
            },
        ];
        let grid = UniformGrid::new(&points, 1.0);
        let ids: Vec<_> = grid
            .within(&points, Vec2::ZERO, 0.5)
            .into_iter()
            .map(|i| points[i].id)
            .collect();
        assert_eq!(ids, vec![2, 9]);
    }

    #[test]
    fn huge_sparse_queries_visit_occupied_cells_instead_of_empty_rectangle() {
        let points: Vec<_> = (0..2_500_u32)
            .map(|id| GridPoint {
                id,
                position: Vec2::new((id % 50) as f32 * 2.0, (id / 50) as f32 * 2.0),
            })
            .collect();
        let grid = UniformGrid::new(&points, 1.0);
        let found = grid.within(&points, Vec2::ZERO, 1_000_000.0);
        assert_eq!(found.len(), points.len());
        assert_eq!(grid.visited_cells(), points.len());
    }
}
