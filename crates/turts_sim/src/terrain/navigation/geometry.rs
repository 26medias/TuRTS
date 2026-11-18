//! Shared exact predicates. Planning observes no live strata or private revision.
use super::super::TerrainState;
use crate::movement::MovementFrameUnit;
use bevy_ecs::world::World;
use bevy_math::Vec2;
use std::{cell::RefCell, collections::BTreeMap};
use turts_mod_api::ContentId;
use turts_rules::terrain::{
    Ratio,
    geometry::{Budget, Cell, Error, Grid, Point, Surface, Traversal},
};
const GEOMETRY_BUDGET: usize = 16_384;
const MAX_QUERY_LEASES: usize = 4096;
pub(super) trait TerrainRead: Surface {
    fn epoch(&self) -> u64 {
        0
    }
    fn lease_matches(&self, key: usize, revision: u64) -> bool {
        self.valid_lease(&BTreeMap::from([(key, revision)]))
    }
    fn grid(&self) -> Grid;
    fn rectangle(&self, minimum: Cell, maximum: Cell, lease: &mut BTreeMap<usize, u64>) -> bool;
    fn valid_lease(&self, lease: &BTreeMap<usize, u64>) -> bool;
}
impl TerrainRead for TerrainState {
    fn grid(&self) -> Grid {
        self.grid
    }
    fn rectangle(&self, minimum: Cell, maximum: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
        self.rectangle_certificate(minimum, maximum, lease)
    }
    fn valid_lease(&self, lease: &BTreeMap<usize, u64>) -> bool {
        self.lease_valid(lease)
    }
}
/// A planner cannot access live columns or private revision counters through this type.
pub(super) struct Planning<'a> {
    pub(super) phase: &'a super::super::TerrainPhase,
    pub(super) viewer: u8,
}
impl Surface for Planning<'_> {
    fn elevation(&self, cell: Cell) -> Option<i32> {
        self.phase.planning_elevation(self.viewer, cell)
    }
}
impl TerrainRead for Planning<'_> {
    fn epoch(&self) -> u64 {
        self.phase.planning_epoch(self.viewer)
    }
    fn lease_matches(&self, key: usize, revision: u64) -> bool {
        self.phase.knowledge_revision(self.viewer, key) == revision
    }
    fn grid(&self) -> Grid {
        self.phase.grid()
    }
    fn rectangle(&self, minimum: Cell, maximum: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
        self.phase
            .planning_rectangle(self.viewer, minimum, maximum, lease)
    }
    fn valid_lease(&self, lease: &BTreeMap<usize, u64>) -> bool {
        self.phase.planning_lease_valid(self.viewer, lease)
    }
}
pub(super) fn point(point: Vec2) -> Point {
    Point {
        x: f64::from(point.x),
        y: f64::from(point.y),
    }
}
pub(super) fn bits(value: f32) -> u32 {
    if value == 0.0 { 0 } else { value.to_bits() }
}
pub(super) fn profile(world: &World, unit: &MovementFrameUnit) -> Option<Traversal> {
    let entry = world
        .get_resource::<crate::UnitTypes>()?
        .get(turts_mod_api::UnitTypeId(unit.unit_type))?;
    let id = ContentId::parse(&entry.content_id).ok()?;
    let catalog = &world
        .get_resource::<crate::bootstrap::InstalledCatalog>()?
        .0;
    let class = catalog
        .units
        .get(&id)?
        .terrain_class
        .as_ref()
        .and_then(|id| catalog.terrain_movement_classes.get(id));
    Some(class.map_or(
        Traversal {
            max_step: 0,
            max_grade: Ratio {
                numerator: 0,
                denominator: 1,
            },
        },
        |class| Traversal {
            max_step: class.max_step,
            max_grade: Ratio {
                numerator: u64::from(class.max_grade.numerator),
                denominator: u64::from(class.max_grade.denominator),
            },
        },
    ))
}
/// Certify the entire segment. A flat chunk rectangle is an exact sufficient certificate;
/// otherwise use the common analytic swept-disk predicate. End support is always checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Certificate {
    Clear,
    Blocked,
    Pending,
}
struct Tracked<'a, T> {
    terrain: &'a T,
    lease: RefCell<&'a mut BTreeMap<usize, u64>>,
}
impl<T: TerrainRead> Surface for Tracked<'_, T> {
    fn elevation(&self, cell: Cell) -> Option<i32> {
        let mut lease = self.lease.borrow_mut();
        if lease.len() > MAX_QUERY_LEASES {
            return None;
        }
        self.terrain.rectangle(cell, cell, &mut lease);
        if lease.len() > MAX_QUERY_LEASES {
            return None;
        }
        self.terrain.elevation(cell)
    }
}
pub(super) fn certify(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    lease: &mut BTreeMap<usize, u64>,
) -> Certificate {
    if !radius.is_finite() || radius <= 0.0 || !from.is_finite() || !to.is_finite() {
        return Certificate::Blocked;
    }
    let tracked = Tracked {
        terrain,
        lease: RefCell::new(lease),
    };
    let mut budget = Budget::new(GEOMETRY_BUDGET);
    let classify = |error| match error {
        Error::Budget | Error::Uncertain | Error::Unknown(_) => Certificate::Pending,
        _ => Certificate::Blocked,
    };
    let first = match terrain.grid().disk_support(
        &tracked,
        point(from),
        f64::from(radius),
        profile.max_step,
        &mut budget,
    ) {
        Ok(value) => value,
        Err(error) => return classify(error),
    };
    let last = match terrain.grid().disk_support(
        &tracked,
        point(to),
        f64::from(radius),
        profile.max_step,
        &mut budget,
    ) {
        Ok(value) => value,
        Err(error) => return classify(error),
    };
    let mut minimum = Cell {
        x: u32::MAX,
        y: u32::MAX,
    };
    let mut maximum = Cell { x: 0, y: 0 };
    for cell in first.cells.iter().chain(&last.cells) {
        minimum.x = minimum.x.min(cell.x);
        minimum.y = minimum.y.min(cell.y);
        maximum.x = maximum.x.max(cell.x);
        maximum.y = maximum.y.max(cell.y);
    }
    let area = u64::from(maximum.x - minimum.x + 1) * u64::from(maximum.y - minimum.y + 1);
    if area <= MAX_QUERY_LEASES as u64
        && terrain.rectangle(minimum, maximum, &mut tracked.lease.borrow_mut())
    {
        return Certificate::Clear;
    }
    let result = terrain.grid().sweep(
        &tracked,
        point(from),
        point(to),
        f64::from(radius),
        profile,
        &mut budget,
    );
    match result.result {
        Ok(()) if result.numerator == result.denominator => Certificate::Clear,
        Ok(()) => Certificate::Pending,
        Err(error) => classify(error),
    }
}
pub(super) fn clear(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    lease: &mut BTreeMap<usize, u64>,
) -> bool {
    certify(terrain, from, to, radius, profile, lease) == Certificate::Clear
}
pub(super) fn physical_prefix(
    terrain: &TerrainState,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
) -> Vec2 {
    let mut low = 0u32;
    let mut high = 1u32 << 20;
    let candidate = |part: u32| {
        let fraction = f64::from(part) / f64::from(1u32 << 20);
        Vec2::new(
            (f64::from(from.x) + (f64::from(to.x) - f64::from(from.x)) * fraction) as f32,
            (f64::from(from.y) + (f64::from(to.y) - f64::from(from.y)) * fraction) as f32,
        )
    };
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if clear(
            terrain,
            from,
            candidate(middle),
            radius,
            profile,
            &mut BTreeMap::new(),
        ) {
            low = middle;
        } else {
            high = middle;
        }
    }
    let point = candidate(low);
    if clear(terrain, from, point, radius, profile, &mut BTreeMap::new()) {
        point
    } else {
        from
    }
}

/// Certified traffic entrance timing from this viewer's permitted terrain only.
/// The two ±2r probe offsets retain their exact widened-f32 products. A probe
/// constrains by disk-support spread, not by path grade. This helper retains no
/// truth lease or inferred contact height; only the supplied immutable phase is read.
pub(crate) fn permitted_entrance_contact(
    phase: &super::super::TerrainPhase,
    viewer: u8,
    from: Vec2,
    to: Vec2,
    axis: Vec2,
    radius: f32,
    profile: Traversal,
) -> turts_rules::terrain::geometry::DualSupportContact {
    entrance_contact(&Planning { phase, viewer }, from, to, axis, radius, profile)
}
fn entrance_contact(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    axis: Vec2,
    radius: f32,
    profile: Traversal,
) -> turts_rules::terrain::geometry::DualSupportContact {
    entrance_contact_with_probe(terrain, from, to, axis, radius, radius, profile)
}
#[allow(
    clippy::too_many_arguments,
    reason = "A public body and canonical resource probe use separate physical radii"
)]
pub(crate) fn permitted_public_contact(
    phase: &super::super::TerrainPhase,
    viewer: u8,
    from: Vec2,
    to: Vec2,
    axis: Vec2,
    radius: f32,
    probe_radius: f32,
    profile: Traversal,
) -> turts_rules::terrain::geometry::DualSupportContact {
    entrance_contact_with_probe(
        &Planning { phase, viewer },
        from,
        to,
        axis,
        radius,
        probe_radius,
        profile,
    )
}
fn entrance_contact_with_probe(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    axis: Vec2,
    radius: f32,
    probe_radius: f32,
    profile: Traversal,
) -> turts_rules::terrain::geometry::DualSupportContact {
    use turts_rules::terrain::geometry::DualSupportContact;
    let x = f64::from(axis.x);
    let y = f64::from(axis.y);
    if !probe_radius.is_finite()
        || probe_radius <= 0.
        || !axis.is_finite()
        || x * x + y * y == 0.0
        || certify(terrain, from, to, radius, profile, &mut BTreeMap::new()) != Certificate::Clear
    {
        return DualSupportContact::Unresolved;
    }
    let reach = 2.0 * f64::from(probe_radius);
    let offset = Point {
        x: -y * reach,
        y: x * reach,
    };
    // A single homogeneous permitted rectangle containing BOTH complete probe
    // sweeps also contains the centered body sweep. It proves that neither
    // flank can have a positive-area height spread at any instant.
    if flat_probe_rectangle(
        terrain,
        point(from),
        point(to),
        offset,
        f64::from(radius),
        &mut BTreeMap::new(),
    ) {
        return DualSupportContact::Outside;
    }
    terrain.grid().first_dual_support_contact(
        terrain,
        point(from),
        point(to),
        [
            offset,
            Point {
                x: -offset.x,
                y: -offset.y,
            },
        ],
        f64::from(radius),
        profile.max_step,
        &mut Budget::new(GEOMETRY_BUDGET),
    )
}

/// Exact sufficient certificate only. Outward float bounds may include extra
/// cells and reject this shortcut; uncertainty always uses the existing kernel.
fn flat_probe_rectangle(
    terrain: &impl TerrainRead,
    from: Point,
    to: Point,
    offset: Point,
    radius: f64,
    lease: &mut BTreeMap<usize, u64>,
) -> bool {
    flat_rectangle(
        terrain,
        from,
        to,
        Point {
            x: -offset.x.abs(),
            y: -offset.y.abs(),
        },
        Point {
            x: offset.x.abs(),
            y: offset.y.abs(),
        },
        radius,
        lease,
    )
}
fn flat_rectangle(
    terrain: &impl TerrainRead,
    from: Point,
    to: Point,
    low_offset: Point,
    high_offset: Point,
    radius: f64,
    lease: &mut BTreeMap<usize, u64>,
) -> bool {
    let grid = terrain.grid();
    if ![
        from.x,
        from.y,
        to.x,
        to.y,
        low_offset.x,
        low_offset.y,
        high_offset.x,
        high_offset.y,
        radius,
    ]
    .into_iter()
    .all(f64::is_finite)
        || radius <= 0.0
        || grid.length_scale == 0
        || grid.cell_width == 0
    {
        return false;
    }
    let low = |a: f64, b: f64, o: f64| ((a.min(b) + o).next_down() - radius).next_down();
    let high = |a: f64, b: f64, o: f64| ((a.max(b) + o).next_up() + radius).next_up();
    let min = Point {
        x: low(from.x, to.x, low_offset.x),
        y: low(from.y, to.y, low_offset.y),
    };
    let max = Point {
        x: high(from.x, to.x, high_offset.x),
        y: high(from.y, to.y, high_offset.y),
    };
    let scale = f64::from(grid.length_scale);
    let width = f64::from(grid.cell_width);
    let extent = |n: u32| ((f64::from(n) * width).next_down() / scale).next_down();
    if min.x < 0.0 || min.y < 0.0 || max.x > extent(grid.columns) || max.y > extent(grid.rows) {
        return false;
    }
    let lower = |v: f64| ((v * scale).next_down() / width).next_down().floor();
    let upper = |v: f64| ((v * scale).next_up() / width).next_up().floor();
    let (x0, y0, x1, y1) = (lower(min.x), lower(min.y), upper(max.x), upper(max.y));
    if x0 < 0.0 || y0 < 0.0 || x1 >= f64::from(grid.columns) || y1 >= f64::from(grid.rows) {
        return false;
    }
    let minimum = Cell {
        x: x0 as u32,
        y: y0 as u32,
    };
    let maximum = Cell {
        x: x1 as u32,
        y: y1 as u32,
    };
    let area = u64::from(maximum.x - minimum.x + 1) * u64::from(maximum.y - minimum.y + 1);
    area <= MAX_QUERY_LEASES as u64 && terrain.rectangle(minimum, maximum, lease)
}

#[cfg(test)]
#[path = "contact_rectangle_tests.rs"]
mod contact_rectangle_tests;

/// Positive canonical rectangular-throat witness from CURRENT permitted cells.
/// Every cell read is charged before its height can affect the result.
pub(super) fn public_throat(
    terrain: &impl TerrainRead,
    from: Vec2,
    goal: Vec2,
    radius: f32,
    profile: Traversal,
    lease: &mut BTreeMap<usize, u64>,
    charge: &mut (impl FnMut() -> bool + ?Sized),
) -> Option<super::super::traffic::public_passage::Throat> {
    use super::super::traffic::public_passage::Throat;
    if !from.is_finite() || !goal.is_finite() || !radius.is_finite() || radius <= 0. {
        return None;
    }
    let grid = terrain.grid();
    let cell = f64::from(grid.cell_width) / f64::from(grid.length_scale);
    if !cell.is_finite() || cell <= 0. {
        return None;
    }
    let x_axis = (goal.x - from.x).abs() >= (goal.y - from.y).abs();
    let along = |p: Vec2| if x_axis { p.x } else { p.y };
    let across = |p: Vec2| if x_axis { p.y } else { p.x };
    let start = (f64::from(along(from)) / cell).floor() as i64;
    let normal = (f64::from(across(goal)) / cell).floor() as i64;
    let sign = if along(goal) >= along(from) { 1 } else { -1 };
    let reach = (8. * f64::from(radius) / cell).ceil() as i64;
    let probe = (2. * f64::from(radius) / cell).ceil() as i64;
    if !(1..=128).contains(&reach) || !(1..=128).contains(&probe) {
        return None;
    }
    let tracked = Tracked {
        terrain,
        lease: RefCell::new(lease),
    };
    let mut read = |a: i64, n: i64| -> Option<i32> {
        if a < 0 || n < 0 || !charge() {
            return None;
        }
        let c = if x_axis {
            Cell {
                x: a.try_into().ok()?,
                y: n.try_into().ok()?,
            }
        } else {
            Cell {
                x: n.try_into().ok()?,
                y: a.try_into().ok()?,
            }
        };
        tracked.elevation(c)
    };
    for step in 0..=reach {
        let a = start + sign * step;
        let level = read(a, normal)?;
        let low = read(a, normal - probe)?;
        let high = read(a, normal + probe)?;
        let blocked = |h: i32| h.abs_diff(level) > profile.max_step;
        if !blocked(low) || !blocked(high) {
            continue;
        }
        let mut lo = normal;
        let mut hi = normal + 1;
        while lo > normal - probe && read(a, lo - 1)? == level {
            lo -= 1;
        }
        while hi <= normal + probe && read(a, hi)? == level {
            hi += 1;
        }
        if !blocked(read(a, lo - 1)?) || !blocked(read(a, hi)?) {
            continue;
        }
        let width = (hi - lo) as f64 * cell;
        if width < 2. * f64::from(radius) || width >= 4. * f64::from(radius) {
            continue;
        }
        let mut left = a;
        let mut right = a + 1;
        for direction in [-1, 1] {
            let mut edge = if direction < 0 { left - 1 } else { right };
            let mut finished = false;
            for _ in 0..=reach {
                let floor = read(edge, normal)?;
                let l = read(edge, lo - 1)?;
                let h = read(edge, hi)?;
                if floor == level && !blocked(l) && !blocked(h) {
                    finished = true;
                    break;
                }
                if floor != level || !blocked(l) || !blocked(h) {
                    break;
                }
                if direction < 0 {
                    left = edge;
                } else {
                    right = edge + 1;
                }
                edge += direction;
            }
            if !finished {
                return None;
            }
        }
        for a in left..right {
            for n in lo..hi {
                if read(a, n)? != level {
                    return None;
                }
            }
            if !blocked(read(a, lo - 1)?) || !blocked(read(a, hi)?) {
                return None;
            }
        }
        for a in [left - 1, right] {
            for n in lo - 1..=hi {
                if read(a, n)? != level {
                    return None;
                }
            }
        }
        let world = |a: i64, n: i64| -> Option<Vec2> {
            let d = if x_axis {
                [a as f64 * cell, n as f64 * cell]
            } else {
                [n as f64 * cell, a as f64 * cell]
            };
            let v = Vec2::new(d[0] as f32, d[1] as f32);
            (v.is_finite() && f64::from(v.x) == d[0] && f64::from(v.y) == d[1]).then_some(v)
        };
        return Some(Throat {
            min: world(left, lo)?,
            max: world(right, hi)?,
            axis: if x_axis { Vec2::X } else { Vec2::Y },
            level,
        });
    }
    None
}
