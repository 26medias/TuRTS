//! Public placement advice regions. Hidden rows cannot affect these bounds.
use crate::placement::Point;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PublicPlacementMaxima {
    pub unit_radius: f32,
    pub static_gap: f32,
    pub node_radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdviceRegion {
    pub min: Point,
    pub max: Point,
}

/// Cover every possible blocker centre that can affect the public placement rule.
/// Own build-area anchors are always permitted. Callers must separately cover any other anchors.
pub fn required_region(
    point: Point,
    radius: f32,
    gap: f32,
    node_clearance: f32,
    maxima: PublicPlacementMaxima,
    map_size: Point,
) -> Option<AdviceRegion> {
    if [
        point.x,
        point.y,
        radius,
        gap,
        node_clearance,
        maxima.unit_radius,
        maxima.static_gap,
        maxima.node_radius,
        map_size.x,
        map_size.y,
    ]
    .iter()
    .any(|value| !value.is_finite())
        || radius <= 0.
        || map_size.x <= 0.
        || map_size.y <= 0.
        || point.x < 0.
        || point.y < 0.
        || point.x > map_size.x
        || point.y > map_size.y
    {
        return None;
    }
    // f64 prevents finite authored radii from overflowing before map clipping.
    let units = f64::from(radius)
        + f64::from(maxima.unit_radius.max(0.))
        + f64::from(gap.max(maxima.static_gap).max(0.));
    let nodes = f64::from(radius)
        + f64::from(maxima.node_radius.max(0.))
        + f64::from(node_clearance.max(0.));
    let reach = units.max(nodes);
    Some(AdviceRegion {
        min: Point {
            x: (f64::from(point.x) - reach).max(0.) as f32,
            y: (f64::from(point.y) - reach).max(0.) as f32,
        },
        max: Point {
            x: (f64::from(point.x) + reach).min(f64::from(map_size.x)) as f32,
            y: (f64::from(point.y) + reach).min(f64::from(map_size.y)) as f32,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_region_includes_clearance_and_static_gap_outside_footprint() {
        let point = Point { x: 20., y: 20. };
        let map = Point { x: 64., y: 64. };
        let maxima = PublicPlacementMaxima {
            unit_radius: 2.,
            static_gap: 7.,
            node_radius: 3.,
        };
        let region = required_region(point, 1., 2., 10., maxima, map).unwrap();
        assert_eq!(region.min, Point { x: 6., y: 6. });
        assert_eq!(region.max, Point { x: 34., y: 34. });
    }
    #[test]
    fn large_finite_values_clip_before_conversion() {
        let map = Point { x: 64., y: 64. };
        let region = required_region(
            Point { x: 32., y: 32. },
            f32::MAX,
            f32::MAX,
            f32::MAX,
            PublicPlacementMaxima {
                unit_radius: f32::MAX,
                static_gap: f32::MAX,
                node_radius: f32::MAX,
            },
            map,
        )
        .unwrap();
        assert_eq!(region.min, Point { x: 0., y: 0. });
        assert_eq!(region.max, map);
    }
}
