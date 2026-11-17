//! M6 underground utility geometry and burial rules. The server simulation and the
//! client preview call these functions, so both targets use one interpretation.
//!
//! Plan coordinates are world units. Elevations are integer terrain steps on the fixed
//! datum. A placed segment keeps its elevation; only the surface above it changes.

/// The largest number of candidate cells that `touched_cells` inspects.
pub const MAX_CANDIDATE_CELLS: usize = 65_536;

/// Why a segment has no valid touched-cell set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchError {
    /// A coordinate or the cell width is not finite and positive.
    NotFinite,
    /// An endpoint is outside the grid.
    OutsideGrid,
    /// More cells than the caller's limit, or too many candidates.
    TooManyCells,
}

/// Cells (column, row) whose closed square meets the closed segment `a`-`b`, in
/// ascending (row, column) order. `cell_width` is the side of one cell in world units.
pub fn touched_cells(
    a: (f32, f32),
    b: (f32, f32),
    cell_width: f32,
    columns: u32,
    rows: u32,
    limit: usize,
) -> Result<Vec<(u32, u32)>, TouchError> {
    let values = [a.0, a.1, b.0, b.1, cell_width];
    if values.iter().any(|value| !value.is_finite()) || cell_width <= 0.0 {
        return Err(TouchError::NotFinite);
    }
    let width = f64::from(cell_width);
    let extent_x = width * f64::from(columns);
    let extent_y = width * f64::from(rows);
    let inside = |point: (f32, f32)| {
        let (x, y) = (f64::from(point.0), f64::from(point.1));
        x >= 0.0 && y >= 0.0 && x <= extent_x && y <= extent_y
    };
    if columns == 0 || rows == 0 || !inside(a) || !inside(b) {
        return Err(TouchError::OutsideGrid);
    }
    let (ax, ay) = (f64::from(a.0), f64::from(a.1));
    let (bx, by) = (f64::from(b.0), f64::from(b.1));
    let cell = |value: f64, count: u32| -> (u32, u32) {
        // A coordinate on a boundary belongs to both neighbouring closed squares.
        let scaled = value / width;
        let low = (scaled.ceil() as i64 - 1).max(0) as u32;
        let high = (scaled.floor() as i64).min(i64::from(count) - 1).max(0) as u32;
        (low.min(high), high)
    };
    let (min_x, _) = cell(ax.min(bx), columns);
    let (_, max_x) = cell(ax.max(bx), columns);
    let (min_y, _) = cell(ay.min(by), rows);
    let (_, max_y) = cell(ay.max(by), rows);
    let candidates = (u64::from(max_x - min_x) + 1) * (u64::from(max_y - min_y) + 1);
    if candidates > MAX_CANDIDATE_CELLS as u64 {
        return Err(TouchError::TooManyCells);
    }
    let mut cells = Vec::new();
    for row in min_y..=max_y {
        for column in min_x..=max_x {
            let x0 = f64::from(column) * width;
            let y0 = f64::from(row) * width;
            if segment_meets_box((ax, ay), (bx, by), (x0, y0), (x0 + width, y0 + width)) {
                if cells.len() == limit {
                    return Err(TouchError::TooManyCells);
                }
                cells.push((column, row));
            }
        }
    }
    Ok(cells)
}

/// Liang-Barsky clip of a closed segment against a closed box.
fn segment_meets_box(a: (f64, f64), b: (f64, f64), min: (f64, f64), max: (f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let mut low = 0.0_f64;
    let mut high = 1.0_f64;
    for (p, q) in [
        (-dx, a.0 - min.0),
        (dx, max.0 - a.0),
        (-dy, a.1 - min.1),
        (dy, max.1 - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                low = low.max(t);
            } else {
                high = high.min(t);
            }
            if low > high {
                return false;
            }
        }
    }
    true
}

/// Plan length of a segment.
pub fn length(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// The parameter `t` in 0..=1 and the closest point of segment `a`-`b` to `p`.
pub fn project(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> (f32, (f32, f32)) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let squared = dx * dx + dy * dy;
    if squared == 0.0 {
        return (0.0, a);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / squared).clamp(0.0, 1.0);
    (t, (a.0 + t * dx, a.1 + t * dy))
}

/// Plan distance from `p` to segment `a`-`b`.
pub fn distance_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (_, closest) = project(p, a, b);
    length(p, closest)
}

/// Fixed placement elevation: the lowest touched surface minus the requested depth.
/// `None` when there is no touched surface or the result overflows.
pub fn placement_elevation(surfaces: impl IntoIterator<Item = i32>, depth: u32) -> Option<i32> {
    let lowest = surfaces.into_iter().min()?;
    lowest.checked_sub(i32::try_from(depth).ok()?)
}

/// Cover in elevation steps: the surface minus the fixed line elevation.
pub fn cover(surface: i32, elevation: i32) -> i64 {
    i64::from(surface) - i64::from(elevation)
}

/// A line is exposed at a cell when its cover there is 0 or less.
pub fn exposed_at(surface: i32, elevation: i32) -> bool {
    cover(surface, elevation) <= 0
}

/// A network is short when its consumers ask for more than its providers give.
/// Equality is sufficient supply, the same rule as slot supply.
pub fn network_short(provided: u128, consumed: u128) -> bool {
    consumed > provided
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_segment_inside_one_row_touches_each_crossed_cell() {
        let cells = touched_cells((0.5, 0.5), (3.5, 0.5), 1.0, 8, 8, 256).unwrap();
        assert_eq!(cells, vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
    }

    #[test]
    fn boundary_segments_touch_both_neighbouring_squares() {
        let cells = touched_cells((1.0, 0.5), (1.0, 1.5), 1.0, 4, 4, 256).unwrap();
        assert_eq!(cells, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
    }

    #[test]
    fn diagonal_through_a_corner_touches_the_corner_cells() {
        let cells = touched_cells((0.5, 0.5), (1.5, 1.5), 1.0, 4, 4, 256).unwrap();
        assert_eq!(cells, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
    }

    #[test]
    fn limits_and_bounds_are_refused() {
        assert_eq!(
            touched_cells((0.5, 0.5), (7.5, 0.5), 1.0, 8, 8, 3),
            Err(TouchError::TooManyCells)
        );
        assert_eq!(
            touched_cells((0.5, 0.5), (8.5, 0.5), 1.0, 8, 8, 256),
            Err(TouchError::OutsideGrid)
        );
        assert_eq!(
            touched_cells((f32::NAN, 0.5), (1.0, 0.5), 1.0, 8, 8, 256),
            Err(TouchError::NotFinite)
        );
        assert_eq!(
            touched_cells((0.5, 0.5), (1.0, 0.5), 0.0, 8, 8, 256),
            Err(TouchError::NotFinite)
        );
    }

    #[test]
    fn map_edge_endpoint_stays_in_the_last_cell() {
        let cells = touched_cells((7.5, 8.0), (8.0, 8.0), 1.0, 8, 8, 256).unwrap();
        assert_eq!(cells, vec![(7, 7)]);
    }

    #[test]
    fn elevation_uses_the_lowest_surface_and_cover_tracks_only_the_surface() {
        let elevation = placement_elevation([4, 2, 3], 2).unwrap();
        assert_eq!(elevation, 0);
        assert_eq!(cover(5, elevation), 5);
        assert!(!exposed_at(1, elevation));
        assert!(exposed_at(0, elevation));
        assert!(exposed_at(-1, elevation));
        assert_eq!(placement_elevation(std::iter::empty(), 2), None);
        assert_eq!(placement_elevation([i32::MIN], 1), None);
    }

    #[test]
    fn projection_and_short_rule() {
        let (t, point) = project((1.0, 1.0), (0.0, 0.0), (2.0, 0.0));
        assert_eq!(t, 0.5);
        assert_eq!(point, (1.0, 0.0));
        assert_eq!(distance_to_segment((1.0, 1.0), (0.0, 0.0), (2.0, 0.0)), 1.0);
        assert!(!network_short(10, 10));
        assert!(network_short(10, 11));
        assert!(!network_short(0, 0));
    }
}
