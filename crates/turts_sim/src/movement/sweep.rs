//! The exact swept-disk predicate for one pair of bodies.
use bevy_math::Vec2;
use turts_rules::terrain::geometry::{
    Budget, Point, swept_disks_clear, swept_disks_non_deepening_overlap,
};

fn point(value: Vec2) -> Point {
    Point {
        x: f64::from(value.x),
        y: f64::from(value.y),
    }
}
pub(crate) fn pair_clear(
    a: Vec2,
    to: Vec2,
    radius: f32,
    b: Vec2,
    b_to: Vec2,
    b_radius: f32,
) -> bool {
    if !a.is_finite()
        || !to.is_finite()
        || !b.is_finite()
        || !b_to.is_finite()
        || !radius.is_finite()
        || !b_radius.is_finite()
        || radius <= 0.0
        || b_radius <= 0.0
    {
        return false;
    }
    // Outward-rounded relative bounds only reject pairs that cannot touch.
    // Every remaining pair receives the exact checked-rational certificate.
    let r = (f64::from(radius) + f64::from(b_radius)).next_up();
    let outside = |a0: f32, a1: f32, b0: f32, b1: f32| {
        let first = f64::from(a0) - f64::from(b0);
        let last = f64::from(a1) - f64::from(b1);
        first.min(last).next_down() >= r || first.max(last).next_up() <= -r
    };
    if outside(a.x, to.x, b.x, b_to.x) || outside(a.y, to.y, b.y, b_to.y) {
        return true;
    }
    let mut budget = Budget::new(2);
    swept_disks_clear(
        point(a),
        point(to),
        f64::from(radius),
        point(b),
        point(b_to),
        f64::from(b_radius),
        &mut budget,
    )
    .and_then(|clear| {
        if clear {
            Ok(true)
        } else {
            // Existing overlap recovery is not a legal-start sweep. It may only
            // increase separation; no new overlap or deeper penetration is allowed.
            swept_disks_non_deepening_overlap(
                point(a),
                point(to),
                f64::from(radius),
                point(b),
                point(b_to),
                f64::from(b_radius),
                &mut budget,
            )
        }
    })
    .unwrap_or(false)
}
