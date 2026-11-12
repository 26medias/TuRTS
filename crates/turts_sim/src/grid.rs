//! Deterministic uniform-grid neighbour lookup for flat-ground simulation.

use std::collections::HashMap;

use bevy_math::Vec2;

/// Relative margin on the query box of `within`, times max(1, reach).
const EDGE_MARGIN: f32 = 1e-4;

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
        // Scan the cells of the bounding box of the query disc. The margin is
        // larger than the f32 error of the distance filter below, so every
        // point that the filter accepts is in a scanned cell (ruling 3, G1).
        let half = reach.abs() + EDGE_MARGIN * reach.abs().max(1.0);
        let (low_x, low_y) = cell(position - Vec2::splat(half), self.cell_size);
        let (high_x, high_y) = cell(position + Vec2::splat(half), self.cell_size);
        let min_x = i64::from(low_x);
        let max_x = i64::from(high_x);
        let min_y = i64::from(low_y);
        let max_y = i64::from(high_y);
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

    /// Seeded xorshift stream. It is local to the property test, so the test
    /// repeats with the same queries on each run.
    struct TestRng(u64);

    impl TestRng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        /// A value in [0, 1).
        fn unit(&mut self) -> f32 {
            (self.next() >> 40) as f32 / (1_u32 << 24) as f32
        }

        /// A whole number in [-half, half).
        fn whole(&mut self, half: u64) -> f32 {
            (self.next() % (2 * half)) as f32 - half as f32
        }
    }

    /// The reference: the distance filter of `within` applied to every point.
    fn brute_force_ids(points: &[GridPoint], position: Vec2, reach: f32) -> Vec<u32> {
        let mut ids: Vec<u32> = points
            .iter()
            .filter(|point| point.position.distance_squared(position) <= reach * reach)
            .map(|point| point.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    fn within_ids(
        grid: &UniformGrid,
        points: &[GridPoint],
        position: Vec2,
        reach: f32,
    ) -> Vec<u32> {
        grid.within(points, position, reach)
            .into_iter()
            .map(|index| points[index].id)
            .collect()
    }

    #[derive(Default)]
    struct PropertyCount {
        queries: usize,
        misses: usize,
        extras: usize,
        cell_size_dependent: usize,
    }

    /// Runs each query against the brute-force scan at 3 cell sizes.
    /// Returns true when the query missed one or more points.
    fn check_queries(
        points: &[GridPoint],
        cell_size: f32,
        queries: &[(Vec2, f32)],
        count: &mut PropertyCount,
    ) -> bool {
        let grids = [
            UniformGrid::new(points, cell_size),
            UniformGrid::new(points, cell_size * 2.0 + 0.1),
            UniformGrid::new(points, cell_size * 3.0),
        ];
        let mut missed = false;
        for &(position, reach) in queries {
            count.queries += 1;
            let expected = brute_force_ids(points, position, reach);
            let results: Vec<Vec<u32>> = grids
                .iter()
                .map(|grid| within_ids(grid, points, position, reach))
                .collect();
            for found in &results {
                let lost = expected.iter().filter(|id| !found.contains(id)).count();
                count.misses += lost;
                count.extras += found.iter().filter(|id| !expected.contains(id)).count();
                missed |= lost > 0;
            }
            if results.iter().any(|found| *found != results[0]) {
                count.cell_size_dependent += 1;
            }
        }
        missed
    }

    #[test]
    fn within_matches_brute_force_scan() {
        let mut count = PropertyCount::default();

        // The gate case (design section 9 item 3): a whole-number reach/cell
        // and negative coordinates. The point is inside reach in f32.
        let gate_points = [
            GridPoint {
                id: 0,
                position: Vec2::new(-7.200_000_3, 0.0),
            },
            GridPoint {
                id: 1,
                position: Vec2::new(-7.200_000_3, -3.6),
            },
        ];
        let gate_queries = [(Vec2::new(1.8, 0.0), 9.0), (Vec2::new(1.8, -3.6), 9.0)];
        let gate_missed = check_queries(&gate_points, 0.9, &gate_queries, &mut count);

        // 24,000 seeded queries: 400 rounds of 60 queries.
        let mut rng = TestRng(0x9E37_79B9_7F4A_7C15);
        let cell_sizes = [
            0.1_f32, 0.3, 0.7, 0.9, 1.0, 1.1, 2.0, 2.1, 3.0, 8.0, 8.1, 16.0,
        ];
        for round in 0..400 {
            let span = [64.0_f32, 256.0, 1024.0, 4096.0][round % 4];
            let cell_size = cell_sizes[round % cell_sizes.len()];
            let half_cells = (span / cell_size / 2.0) as u64;
            let mut positions = Vec::new();
            // Random points, with negative coordinates.
            for _ in 0..300 {
                positions.push(Vec2::new(
                    (rng.unit() - 0.5) * span,
                    (rng.unit() - 0.5) * span,
                ));
            }
            // Points on cell edges.
            for _ in 0..100 {
                positions.push(Vec2::new(
                    rng.whole(half_cells) * cell_size,
                    rng.whole(half_cells) * cell_size,
                ));
            }
            let mut queries = Vec::new();
            for _ in 0..60 {
                let position = if rng.next().is_multiple_of(2) {
                    Vec2::new(
                        rng.whole(half_cells) * cell_size,
                        rng.whole(half_cells) * cell_size,
                    )
                } else {
                    Vec2::new((rng.unit() - 0.5) * span, (rng.unit() - 0.5) * span)
                };
                let whole = (1 + rng.next() % 4) as f32;
                let reach = match rng.next() % 4 {
                    // A whole-number reach/cell.
                    0 => whole * cell_size,
                    1 => whole * cell_size + 0.23,
                    2 => 0.5 + rng.unit() * 9.0,
                    _ => {
                        if rng.next().is_multiple_of(8) {
                            0.0
                        } else {
                            whole * cell_size * 0.5
                        }
                    }
                };
                // Points at exactly `reach` on the axes and the diagonals.
                let diagonal = reach * std::f32::consts::FRAC_1_SQRT_2;
                for offset in [
                    Vec2::new(reach, 0.0),
                    Vec2::new(-reach, 0.0),
                    Vec2::new(0.0, reach),
                    Vec2::new(0.0, -reach),
                    Vec2::new(diagonal, diagonal),
                    Vec2::new(-diagonal, -diagonal),
                ] {
                    positions.push(position + offset);
                }
                queries.push((position, reach));
            }
            let points: Vec<GridPoint> = positions
                .iter()
                .enumerate()
                .map(|(id, position)| GridPoint {
                    id: id as u32,
                    position: *position,
                })
                .collect();
            check_queries(&points, cell_size, &queries, &mut count);
        }

        assert_eq!(count.queries, 24_002);
        assert_eq!(count.extras, 0, "within returned points outside reach");
        assert!(
            count.misses == 0,
            "within missed {} points inside reach ({} queries at 3 cell sizes; gate case missed: \
             {gate_missed}; cell-size-dependent results: {})",
            count.misses,
            count.queries,
            count.cell_size_dependent,
        );
        assert!(
            count.cell_size_dependent == 0,
            "within result depends on the cell size: {} queries",
            count.cell_size_dependent,
        );
    }
}
