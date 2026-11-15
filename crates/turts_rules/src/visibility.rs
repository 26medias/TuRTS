//! Square sight-grid geometry. No allocation or engine types.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub width: f32,
    pub height: f32,
    pub cell_size: f32,
    pub columns: u32,
    pub rows: u32,
}
impl Grid {
    pub fn checked(width: f32, height: f32, cell_size: f32) -> Option<Self> {
        if [width, height, cell_size]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            return None;
        }
        let columns = (f64::from(width) / f64::from(cell_size)).ceil();
        let rows = (f64::from(height) / f64::from(cell_size)).ceil();
        if columns > f64::from(u32::MAX) || rows > f64::from(u32::MAX) {
            return None;
        }
        let (columns, rows) = (columns as u32, rows as u32);
        columns.checked_mul(rows)?;
        Some(Self {
            width,
            height,
            cell_size,
            columns,
            rows,
        })
    }
    pub fn cell(self, point: Point) -> Option<u32> {
        if self.columns == 0 || self.rows == 0 {
            return None;
        }
        self.columns.checked_mul(self.rows)?;
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.0
            || point.y < 0.0
            || point.x > self.width
            || point.y > self.height
        {
            return None;
        }
        let column =
            ((f64::from(point.x) / f64::from(self.cell_size)).floor() as u32).min(self.columns - 1);
        let row =
            ((f64::from(point.y) / f64::from(self.cell_size)).floor() as u32).min(self.rows - 1);
        Some(row * self.columns + column)
    }
    pub fn centre(self, cell: u32) -> Option<Point> {
        if self.columns == 0 || self.rows == 0 {
            return None;
        }
        if cell >= self.columns.checked_mul(self.rows)? {
            return None;
        }
        let column = cell % self.columns;
        let row = cell / self.columns;
        let axis = |index: u32, extent: f32| {
            let lo = f64::from(index) * f64::from(self.cell_size);
            let hi = (f64::from(index + 1) * f64::from(self.cell_size)).min(f64::from(extent));
            ((lo + hi) * 0.5) as f32
        };
        Some(Point {
            x: axis(column, self.width),
            y: axis(row, self.height),
        })
    }
    /// Visit clipped circle cells once each, including the source's own cell.
    pub fn circle(self, origin: Point, radius: f32, mut visit: impl FnMut(u32)) -> bool {
        let Some(own) = self.cell(origin) else {
            return false;
        };
        if !radius.is_finite() || radius <= 0.0 {
            return false;
        }
        let range = |coordinate: f32, count: u32| {
            let size = f64::from(self.cell_size);
            let low = ((f64::from(coordinate) - f64::from(radius)) / size)
                .floor()
                .max(0.0);
            let high = ((f64::from(coordinate) + f64::from(radius)) / size)
                .floor()
                .max(0.0);
            ((low as u32).min(count - 1), (high as u32).min(count - 1))
        };
        let (left, right) = range(origin.x, self.columns);
        let (bottom, top) = range(origin.y, self.rows);
        let radius_squared = f64::from(radius) * f64::from(radius);
        // Keep the exact clipped f32 centres used by centre(). The loop already
        // knows row/column, so avoid dividing a cell index back into both axes.
        let axis = |index: u32, extent: f32| {
            let lo = f64::from(index) * f64::from(self.cell_size);
            let hi = (f64::from(index + 1) * f64::from(self.cell_size)).min(f64::from(extent));
            ((lo + hi) * 0.5) as f32
        };
        for row in bottom..=top {
            let dy = f64::from(axis(row, self.height)) - f64::from(origin.y);
            let dy_squared = dy * dy;
            let offset = row * self.columns;
            for column in left..=right {
                let cell = offset + column;
                let dx = f64::from(axis(column, self.width)) - f64::from(origin.x);
                if cell == own || dx * dx + dy_squared <= radius_squared {
                    visit(cell);
                }
            }
        }
        true
    }
    /// The inclusive rectangle includes any cell that its boundary touches.
    pub fn rectangle(self, min: Point, max: Point, mut visit: impl FnMut(u32) -> bool) -> bool {
        if min.x > max.x || min.y > max.y {
            return false;
        }
        let (Some(first), Some(last)) = (self.cell(min), self.cell(max)) else {
            return false;
        };
        for row in first / self.columns..=last / self.columns {
            for column in first % self.columns..=last % self.columns {
                if !visit(row * self.columns + column) {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_cells_and_maximum_boundary_share_one_rule() {
        let grid = Grid::checked(5.0, 7.0, 2.0).unwrap();
        assert_eq!((grid.columns, grid.rows), (3, 4));
        assert_eq!(grid.cell(Point { x: 5.0, y: 7.0 }), Some(11));
        assert_eq!(grid.centre(11), Some(Point { x: 4.5, y: 6.5 }));
        assert_eq!(
            grid.cell(Point { x: -0.0, y: 0.0 }),
            grid.cell(Point { x: 0.0, y: 0.0 })
        );
        assert!(grid.cell(Point { x: 5.01, y: 7.0 }).is_none());
    }
    #[test]
    fn source_cell_and_inclusive_circle_do_not_depend_on_ids() {
        let grid = Grid::checked(4.0, 4.0, 2.0).unwrap();
        let mut cells = Vec::new();
        assert!(grid.circle(Point { x: 0.0, y: 0.0 }, 0.01, |cell| cells.push(cell)));
        assert_eq!(cells, [0]);
        cells.clear();
        grid.circle(Point { x: 1.0, y: 1.0 }, 2.0, |cell| cells.push(cell));
        assert_eq!(cells, [0, 1, 2]);
        cells.clear();
        grid.circle(Point { x: 1.0, y: 1.0 }, f32::MAX, |cell| cells.push(cell));
        assert_eq!(cells, [0, 1, 2, 3]);
    }
    #[test]
    fn full_rectangle_coverage_requires_its_boundary_cells() {
        let grid = Grid::checked(4.0, 4.0, 2.0).unwrap();
        assert!(!grid.rectangle(
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 2.0 },
            |cell| cell == 0
        ));
        assert!(grid.rectangle(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.9, y: 1.9 },
            |cell| cell == 0
        ));
        assert!(Grid::checked(f32::MAX, f32::MAX, f32::from_bits(1)).is_none());
    }
}

#[cfg(test)]
mod circle_oracle_tests {
    use super::*;

    #[test]
    fn circle_matches_all_public_cell_centres_at_edges_and_partial_cells() {
        for (width, height, size) in [
            (5.3, 7.7, 2.0),
            (3.25, 1.75, 0.5),
            (64.0, 64.0, 2.0),
            (1.0, 1.0, 2.0),
        ] {
            let grid = Grid::checked(width, height, size).unwrap();
            for x in [0.0, width / 3.0, width / 2.0, width] {
                for y in [0.0, height / 3.0, height / 2.0, height] {
                    let origin = Point { x, y };
                    let own = grid.cell(origin).unwrap();
                    for radius in [f32::from_bits(1), 0.25, 0.5, 1.0, 2.0, 8.0, f32::MAX] {
                        let expected: Vec<_> = (0..grid.columns * grid.rows)
                            .filter(|cell| {
                                let centre = grid.centre(*cell).unwrap();
                                let dx = f64::from(centre.x) - f64::from(x);
                                let dy = f64::from(centre.y) - f64::from(y);
                                *cell == own
                                    || dx * dx + dy * dy <= f64::from(radius) * f64::from(radius)
                            })
                            .collect();
                        let mut actual = Vec::new();
                        assert!(grid.circle(origin, radius, |cell| actual.push(cell)));
                        assert_eq!(
                            actual, expected,
                            "grid {width}x{height}/{size}, origin {x},{y}, radius {radius}"
                        );
                    }
                }
            }
        }
    }
}
