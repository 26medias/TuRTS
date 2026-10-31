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
}

impl UniformGrid {
    pub(crate) fn new(points: &[GridPoint], cell_size: f32) -> Self {
        let cell_size = cell_size.max(f32::EPSILON);
        let mut grid = Self {
            cell_size,
            cells: HashMap::with_capacity(points.len()),
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
        let rings = (reach / self.cell_size).ceil() as i32;
        let mut found = Vec::new();
        for y in cy - rings..=cy + rings {
            for x in cx - rings..=cx + rings {
                if let Some(indices) = self.cells.get(&(x, y)) {
                    found.extend(indices.iter().copied().filter(|index| {
                        points[*index].position.distance_squared(position) <= reach * reach
                    }));
                }
            }
        }
        found.sort_unstable_by_key(|index| points[*index].id);
        found
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
}
