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
pub(super) const GEOMETRY_BUDGET: usize = 16_384;
const MAX_QUERY_LEASES: usize = 4096;
pub(super) trait TerrainRead: Surface {
    fn grid(&self) -> Grid;
    fn rectangle(&self, minimum: Cell, maximum: Cell, lease: &mut BTreeMap<usize, u64>) -> bool;
}
impl TerrainRead for TerrainState {
    fn grid(&self) -> Grid {
        self.grid
    }
    fn rectangle(&self, minimum: Cell, maximum: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
        self.rectangle_certificate(minimum, maximum, lease)
    }
}
pub(super) fn point(point: Vec2) -> Point {
    Point {
        x: f64::from(point.x),
        y: f64::from(point.y),
    }
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
    certify_with_refusal(terrain, from, to, radius, profile, lease).0
}
/// As `certify`. The flag is true when a `Pending` result is a refusal of the
/// geometry itself (work budget, arithmetic uncertainty, or a partial sweep).
/// Such a refusal is a pure function of the inputs and the permitted terrain.
/// A `Pending` from unknown terrain is not flagged: knowledge can arrive.
/// The last value is the geometry work that the query used.
pub(super) fn certify_with_refusal(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    lease: &mut BTreeMap<usize, u64>,
) -> (Certificate, bool, usize) {
    let mut budget = Budget::new(GEOMETRY_BUDGET);
    let (result, refusal) =
        certify_budgeted(terrain, from, to, radius, profile, lease, &mut budget);
    // The work is counted in geometry operations, never in time.
    (result, refusal, GEOMETRY_BUDGET - budget.remaining())
}
fn certify_budgeted(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    radius: f32,
    profile: Traversal,
    lease: &mut BTreeMap<usize, u64>,
    budget: &mut Budget,
) -> (Certificate, bool) {
    if !radius.is_finite() || radius <= 0.0 || !from.is_finite() || !to.is_finite() {
        return (Certificate::Blocked, false);
    }
    let tracked = Tracked {
        terrain,
        lease: RefCell::new(lease),
    };
    let classify = |error| match error {
        Error::Budget | Error::Uncertain => (Certificate::Pending, true),
        Error::Unknown(_) => (Certificate::Pending, false),
        _ => (Certificate::Blocked, false),
    };
    let first = match terrain.grid().disk_support(
        &tracked,
        point(from),
        f64::from(radius),
        profile.max_step,
        budget,
    ) {
        Ok(value) => value,
        Err(error) => return classify(error),
    };
    let last = match terrain.grid().disk_support(
        &tracked,
        point(to),
        f64::from(radius),
        profile.max_step,
        budget,
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
        return (Certificate::Clear, false);
    }
    let result = terrain.grid().sweep(
        &tracked,
        point(from),
        point(to),
        f64::from(radius),
        profile,
        budget,
    );
    match result.result {
        Ok(()) if result.numerator == result.denominator => (Certificate::Clear, false),
        Ok(()) => (Certificate::Pending, true),
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
    terrain: &impl TerrainRead,
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
