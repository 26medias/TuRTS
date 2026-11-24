//! Independent full-circle collision arithmetic, plus the shared exact terrain predicate.
//! Flock rules of 2026-11-23: a step never crosses terrain, a building or a body of another
//! owner; two own bodies overlap at most 25 % of the smaller radius; at rest no overlap.
use bevy::math::Vec2;
use turts_rules::terrain::{
    Ratio,
    geometry::{Budget, Cell, Grid, Point, Traversal},
};

#[derive(Clone)]
pub struct Body {
    pub label: String,
    pub position: Vec2,
    pub radius: f32,
    pub speed: f32,
    pub max_step: u32,
    pub max_grade: [u64; 2],
    pub owner: u8,
    /// The unit has no order and no move target.
    pub idle: bool,
}
/// Rule 4: the largest overlap of two own bodies, as a share of the smaller radius.
pub const OWN_OVERLAP_SHARE: f64 = 0.25;
/// Tolerance of "no overlap at rest", in metres.
pub const REST_TOLERANCE: f64 = 1.0e-4;
/// True when the unit has no current order, no queued order, no failure and no move target.
pub fn idle(world: &bevy::prelude::World, unit: u32) -> bool {
    let entity = turts_sim::unit_entity(world, unit).unwrap();
    let orders = world.get::<turts_sim::UnitOrders>(entity).unwrap();
    orders.current.is_none()
        && orders.queue.is_empty()
        && orders.last_failure.is_none()
        && world.get::<turts_sim::MoveTarget>(entity).is_none()
}
pub struct SurfaceState {
    pub grid: Grid,
    pub heights: Vec<i32>,
}
impl SurfaceState {
    pub fn read(world: &bevy::prelude::World) -> Self {
        let audit =
            turts_sim::terrain::privileged_audit(world).expect("opted-in public terrain fixture");
        let grid = turts_sim::terrain::grid(world).unwrap();
        let heights = audit
            .columns_state
            .iter()
            .map(|column| column.surface)
            .collect();
        Self { grid, heights }
    }
    pub fn elevation(&self, cell: Cell) -> Option<i32> {
        if cell.x >= self.grid.columns || cell.y >= self.grid.rows {
            return None;
        }
        self.heights
            .get((cell.y * self.grid.columns + cell.x) as usize)
            .copied()
    }
    pub fn rectangle_is(&self, min: [u32; 2], max: [u32; 2], height: i32) -> bool {
        (min[1]..max[1])
            .all(|y| (min[0]..max[0]).all(|x| self.elevation(Cell { x, y }) == Some(height)))
    }
}
fn point(value: Vec2) -> Point {
    Point {
        x: f64::from(value.x),
        y: f64::from(value.y),
    }
}
pub fn verify_support(scene: &str, tick: u32, bodies: &[Body], surface: &SurfaceState) {
    let lookup = |cell| surface.elevation(cell);
    for body in bodies {
        let support = surface.grid.disk_support(
            &lookup,
            point(body.position),
            f64::from(body.radius),
            body.max_step,
            &mut Budget::new(1_000_000),
        );
        assert!(
            support.is_ok(),
            "{scene} tick{tick} {} has unsupported complete body at {:?}: {support:?}",
            body.label,
            body.position
        );
    }
}
// The frozen world's f32 coordinates and radii are exactly representable at this scale.
// Assert this precondition; rounding a coordinate could conceal a one-ULP overlap.
fn exact(value: f32) -> i128 {
    assert!(value.is_finite());
    let scaled = f64::from(value) * 33_554_432.0;
    assert_eq!(
        scaled.fract(),
        0.0,
        "extend exact oracle for this coordinate: {value:?}"
    );
    scaled as i128
}
fn square(value: i128) -> i128 {
    value
        .checked_mul(value)
        .expect("frozen bounded-world exact square")
}
fn norm(x: i128, y: i128) -> i128 {
    square(x).checked_add(square(y)).unwrap()
}
/// Overlap depth of two bodies in metres. Negative when they stand apart.
pub fn overlap_depth(a: &Body, b: &Body) -> f64 {
    f64::from(a.radius) + f64::from(b.radius)
        - (a.position.as_dvec2() - b.position.as_dvec2()).length()
}
fn no_body_crossing(a0: &Body, a1: &Body, b0: &Body, b1: &Body) -> bool {
    let x = exact(a0.position.x) - exact(b0.position.x);
    let y = exact(a0.position.y) - exact(b0.position.y);
    let vx = (exact(a1.position.x) - exact(a0.position.x))
        - (exact(b1.position.x) - exact(b0.position.x));
    let vy = (exact(a1.position.y) - exact(a0.position.y))
        - (exact(b1.position.y) - exact(b0.position.y));
    let r = exact(a0.radius) + exact(b0.radius);
    let initial = norm(x, y);
    let final_distance = norm(x + vx, y + vy);
    let required = square(r);
    if initial < required || final_distance < required {
        return false;
    }
    let velocity = norm(vx, vy);
    let dot = x
        .checked_mul(vx)
        .unwrap()
        .checked_add(y.checked_mul(vy).unwrap())
        .unwrap();
    if velocity == 0 || dot >= 0 || -dot >= velocity {
        return true;
    }
    // Minimum of |relative_start + t*relative_velocity|², without float division.
    (initial - required).checked_mul(velocity).unwrap() >= square(dot)
}
pub fn verify(scene: &str, tick: u32, before: &[Body], after: &[Body], surface: &SurfaceState) {
    assert_eq!(before.len(), after.len());
    verify_support(scene, tick, after, surface);
    let lookup = |cell| surface.elevation(cell);
    for (index, (from, to)) in before.iter().zip(after).enumerate() {
        assert_eq!(from.label, to.label);
        // f32 position rounding can add at most a few coordinate ULPs to a legal step.
        let ulp = |x: f32| f64::from(x.abs().next_up()) - f64::from(x.abs());
        let rounding = 4.0
            * (ulp(from.position.x)
                + ulp(from.position.y)
                + ulp(to.position.x)
                + ulp(to.position.y));
        let distance = (to.position.as_dvec2() - from.position.as_dvec2()).length();
        assert!(
            distance <= f64::from(from.speed) / 20.0 + rounding,
            "{scene} tick{tick}: {} teleported {distance}",
            from.label
        );
        if from.position != to.position {
            let sweep = surface.grid.sweep(
                &lookup,
                point(from.position),
                point(to.position),
                f64::from(from.radius),
                Traversal {
                    max_step: from.max_step,
                    max_grade: Ratio {
                        numerator: from.max_grade[0],
                        denominator: from.max_grade[1],
                    },
                },
                &mut Budget::new(1_000_000),
            );
            assert!(
                sweep.result.is_ok() && sweep.numerator == sweep.denominator,
                "{scene} tick{tick}: {} committed uncertified {:?}->{:?}: {sweep:?}",
                from.label,
                from.position,
                to.position
            );
        }
        for other in 0..index {
            let (other_from, other_to) = (&before[other], &after[other]);
            let context = || {
                format!(
                    "{} {:?}->{:?} / {} {:?}->{:?}",
                    from.label,
                    from.position,
                    to.position,
                    other_from.label,
                    other_from.position,
                    other_to.position
                )
            };
            // GM4: no committed step crosses a body of another owner or a building.
            if from.owner != other_from.owner || from.speed == 0.0 || other_from.speed == 0.0 {
                assert!(
                    no_body_crossing(from, to, other_from, other_to),
                    "{scene} tick{tick}: swept complete circles overlap: {}",
                    context()
                );
                continue;
            }
            // Rule 4 (FM3): two own bodies overlap at most 25 % of the smaller radius.
            let depth = overlap_depth(to, other_to);
            assert!(
                depth <= OWN_OVERLAP_SHARE * f64::from(to.radius.min(other_to.radius)),
                "{scene} tick{tick}: own bodies overlap {depth} m, above the rule-4 cap: {}",
                context()
            );
            // Rule 4: at rest no overlap. Both units have no order and did not move.
            if to.idle
                && other_to.idle
                && from.position == to.position
                && other_from.position == other_to.position
            {
                assert!(
                    depth <= REST_TOLERANCE,
                    "{scene} tick{tick}: bodies at rest overlap {depth} m: {}",
                    context()
                );
            }
        }
    }
}
#[test]
fn circle_oracle_rejects_crossing_even_with_clear_endpoints_and_keeps_exact_tangency() {
    let body = |label: &str, x, y| Body {
        label: label.into(),
        position: Vec2::new(x, y),
        radius: 1.0,
        speed: 100.0,
        max_step: 0,
        max_grade: [0, 1],
        owner: 0,
        idle: false,
    };
    assert!(!no_body_crossing(
        &body("a", 2.0, 4.0),
        &body("a", 6.0, 4.0),
        &body("b", 6.0, 4.0),
        &body("b", 2.0, 4.0)
    ));
    assert!(no_body_crossing(
        &body("a", 2.0, 4.0),
        &body("a", 6.0, 4.0),
        &body("b", 6.0, 6.0),
        &body("b", 2.0, 6.0)
    ));
    let below = f32::from_bits(6.0f32.to_bits() - 1);
    assert!(!no_body_crossing(
        &body("a", 2.0, 4.0),
        &body("a", 6.0, 4.0),
        &body("b", 6.0, below),
        &body("b", 2.0, below)
    ));
}

fn pair(gap: f32, owners: [u8; 2], idle: bool) -> Vec<Body> {
    let body = |label: &str, x, owner| Body {
        label: label.into(),
        position: Vec2::new(x, 4.0),
        radius: 1.0,
        speed: 100.0,
        max_step: 0,
        max_grade: [0, 1],
        owner,
        idle,
    };
    vec![body("a", 4.0, owners[0]), body("b", 6.0 + gap, owners[1])]
}
fn flat() -> SurfaceState {
    SurfaceState {
        grid: Grid {
            columns: 16,
            rows: 16,
            length_scale: 4,
            cell_width: 4,
            elevation_step: 1,
            floor: -8,
            ceiling: 8,
        },
        heights: vec![0; 256],
    }
}
#[test]
fn own_moving_bodies_can_overlap_up_to_the_rule_4_cap() {
    let bodies = pair(-0.25, [0, 0], false);
    verify("oracle", 1, &bodies, &bodies, &flat());
}
#[test]
#[should_panic(expected = "above the rule-4 cap")]
fn own_bodies_above_the_rule_4_cap_are_refused() {
    let bodies = pair(-0.26, [0, 0], false);
    verify("oracle", 1, &bodies, &bodies, &flat());
}
#[test]
#[should_panic(expected = "bodies at rest overlap")]
fn own_bodies_at_rest_cannot_overlap() {
    let bodies = pair(-0.001, [0, 0], true);
    verify("oracle", 1, &bodies, &bodies, &flat());
}
#[test]
#[should_panic(expected = "swept complete circles overlap")]
fn bodies_of_different_owners_cannot_overlap() {
    let bodies = pair(-0.001, [0, 1], false);
    verify("oracle", 1, &bodies, &bodies, &flat());
}
