//! The placement check, design 6.1: the check order, the gap rule, the build area, the node
//! clearance, the visibility predicate, and squared distances only.

use turts_rules::placement::{
    BuildSource, Circle, Obstacle, ObstacleKind, Point, QueueFill, Reason, Site, Verdict, View,
    check,
};

mod common;
use common::Rng;

fn circle(x: f32, y: f32, radius: f32) -> Circle {
    Circle {
        centre: Point { x, y },
        radius,
    }
}

fn site(x: f32, y: f32) -> Site {
    Site {
        footprint: circle(x, y, 1.0),
        gap: 2.0,
        node_clearance: 1.0,
        in_build_area: false,
        queue: None,
    }
}

fn refused(reason: Reason) -> Verdict {
    Verdict::Refused {
        reason,
        blocker: None,
    }
}

fn view<'a>(
    sources: &'a [BuildSource],
    obstacles: &'a [Obstacle],
    nodes: &'a [Circle],
) -> View<'a> {
    View {
        map_width: 64.0,
        map_height: 64.0,
        build_sources: sources,
        obstacles,
        nodes,
    }
}

fn always(_id: u32) -> bool {
    true
}

fn building(id: u32, x: f32, y: f32, radius: f32, gap: f32) -> Obstacle {
    Obstacle {
        id,
        footprint: circle(x, y, radius),
        kind: ObstacleKind::Static { gap },
    }
}

fn mobile(id: u32, x: f32, y: f32, radius: f32) -> Obstacle {
    Obstacle {
        id,
        footprint: circle(x, y, radius),
        kind: ObstacleKind::Mobile,
    }
}

#[test]
fn an_open_site_is_valid() {
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &[], &[]), always),
        Verdict::Valid
    );
}

#[test]
fn the_reason_keys_and_their_order_are_the_design_list() {
    let keys: Vec<&str> = Reason::ALL.iter().map(|r| r.key()).collect();
    assert_eq!(
        keys,
        [
            "site_outside_map",
            "site_out_of_area",
            "site_blocked",
            "site_near_building",
            "site_near_resource",
            "construction_queue_full",
        ]
    );
    assert!(Reason::ALL.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn each_check_alone_gives_its_reason() {
    let src = [BuildSource {
        footprint: circle(10.0, 10.0, 2.0),
        build_area: 6.0,
    }];
    // 1: outside the map.
    assert_eq!(
        check(&site(0.5, 32.0), &view(&[], &[], &[]), always),
        refused(Reason::SiteOutsideMap)
    );
    // 2: farther than build_area from the edge of every source.
    let mut s = site(40.0, 40.0);
    s.in_build_area = true;
    assert_eq!(
        check(&s, &view(&src, &[], &[]), always),
        refused(Reason::SiteOutOfArea)
    );
    // 3: overlap, a mobile unit included.
    let m = [mobile(7, 32.0, 32.0, 0.5)];
    assert_eq!(
        check(&site(32.5, 32.0), &view(&[], &m, &[]), always),
        Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker: Some(7)
        }
    );
    // 4: a building inside the gap, no overlap.
    let b = [building(8, 35.5, 32.0, 1.0, 0.5)];
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &b, &[]), always),
        refused(Reason::SiteNearBuilding)
    );
    // 5: a resource node inside the clearance.
    let n = [circle(34.5, 32.0, 1.0)];
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &[], &n), always),
        refused(Reason::SiteNearResource)
    );
    // 6: the queue is full.
    let mut q = site(32.0, 32.0);
    q.queue = Some(QueueFill {
        held: 5,
        capacity: 5,
    });
    assert_eq!(
        check(&q, &view(&[], &[], &[]), always),
        refused(Reason::ConstructionQueueFull)
    );
    q.queue = Some(QueueFill {
        held: 4,
        capacity: 5,
    });
    assert_eq!(check(&q, &view(&[], &[], &[]), always), Verdict::Valid);
    // A held order that is placed again runs checks 1 to 5 only.
    q.queue = None;
    assert_eq!(check(&q, &view(&[], &[], &[]), always), Verdict::Valid);
}

#[test]
fn two_failing_causes_give_the_first_in_design_order() {
    // Outside the map (1) and overlap (3) and a full queue (6): the cause is 1.
    let m = [mobile(1, 0.5, 32.0, 1.0)];
    let mut s = site(0.5, 32.0);
    s.queue = Some(QueueFill {
        held: 5,
        capacity: 5,
    });
    assert_eq!(
        check(&s, &view(&[], &m, &[]), always),
        refused(Reason::SiteOutsideMap)
    );
    // Overlap (3) and near a node (5): the cause is 3.
    let m = [mobile(2, 32.0, 32.0, 1.0)];
    let n = [circle(34.5, 32.0, 1.0)];
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &m, &n), always),
        Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker: Some(2)
        }
    );
    // Near a node (5) and a full queue (6): the cause is 5.
    let mut s = site(32.0, 32.0);
    s.queue = Some(QueueFill {
        held: 5,
        capacity: 5,
    });
    assert_eq!(
        check(&s, &view(&[], &[], &n), always),
        refused(Reason::SiteNearResource)
    );
    // Out of area (2) and near a building (4): the cause is 2.
    let src = [BuildSource {
        footprint: circle(5.0, 5.0, 1.0),
        build_area: 3.0,
    }];
    let b = [building(3, 35.5, 32.0, 1.0, 0.5)];
    let mut s = site(32.0, 32.0);
    s.in_build_area = true;
    assert_eq!(
        check(&s, &view(&src, &b, &[]), always),
        refused(Reason::SiteOutOfArea)
    );
}

#[test]
fn the_map_edge_is_inclusive_and_a_non_finite_site_is_outside() {
    let v = view(&[], &[], &[]);
    assert_eq!(check(&site(1.0, 1.0), &v, always), Verdict::Valid);
    assert_eq!(check(&site(63.0, 63.0), &v, always), Verdict::Valid);
    assert_eq!(
        check(&site(0.75, 32.0), &v, always),
        refused(Reason::SiteOutsideMap)
    );
    assert_eq!(
        check(&site(63.25, 32.0), &v, always),
        refused(Reason::SiteOutsideMap)
    );
    assert_eq!(
        check(&site(f32::NAN, 32.0), &v, always),
        refused(Reason::SiteOutsideMap)
    );
    assert_eq!(
        check(&site(32.0, f32::INFINITY), &v, always),
        refused(Reason::SiteOutsideMap)
    );
}

#[test]
fn the_build_area_is_measured_from_the_footprint_edge_and_is_inclusive() {
    // Source radius 2 at (10, 10), build_area 6: the site centre may be 8 from the source centre.
    let src = [BuildSource {
        footprint: circle(10.0, 10.0, 2.0),
        build_area: 6.0,
    }];
    let mut s = site(18.0, 10.0);
    s.in_build_area = true;
    assert_eq!(check(&s, &view(&src, &[], &[]), always), Verdict::Valid);
    s.footprint.centre.x = 18.25;
    assert_eq!(
        check(&s, &view(&src, &[], &[]), always),
        refused(Reason::SiteOutOfArea)
    );
    // One source in range is enough.
    let two = [
        src[0],
        BuildSource {
            footprint: circle(18.25, 20.0, 1.0),
            build_area: 12.0,
        },
    ];
    assert_eq!(check(&s, &view(&two, &[], &[]), always), Verdict::Valid);
    // With the rule off, sources do not matter. With it on and no source, the site is out.
    s.in_build_area = false;
    assert_eq!(check(&s, &view(&[], &[], &[]), always), Verdict::Valid);
    s.in_build_area = true;
    assert_eq!(
        check(&s, &view(&[], &[], &[]), always),
        refused(Reason::SiteOutOfArea)
    );
}

#[test]
fn the_gap_is_the_larger_of_the_two_and_a_mobile_unit_never_counts_in_it() {
    // New site radius 1, gap 2. Building radius 1 at distance d: the edge distance is d - 2.
    let mut s = site(32.0, 32.0);
    s.gap = 2.0;
    // The other gap is larger (4): edge distance 3 is below 4.
    let b = [building(1, 37.0, 32.0, 1.0, 4.0)];
    assert_eq!(
        check(&s, &view(&[], &b, &[]), always),
        refused(Reason::SiteNearBuilding)
    );
    // The new gap is larger (2) than the other (0.5): edge distance 1.75 is below 2.
    let b = [building(1, 35.75, 32.0, 1.0, 0.5)];
    assert_eq!(
        check(&s, &view(&[], &b, &[]), always),
        refused(Reason::SiteNearBuilding)
    );
    // Exactly at the gap is allowed.
    let b = [building(1, 36.0, 32.0, 1.0, 0.5)];
    assert_eq!(check(&s, &view(&[], &b, &[]), always), Verdict::Valid);
    // A mobile unit inside the gap, with no overlap, does not block.
    let m = [mobile(2, 34.5, 32.0, 0.5)];
    assert_eq!(check(&s, &view(&[], &m, &[]), always), Verdict::Valid);
    // Touching is not overlap.
    let m = [mobile(2, 33.5, 32.0, 0.5)];
    assert_eq!(check(&s, &view(&[], &m, &[]), always), Verdict::Valid);
    let m = [mobile(2, 33.25, 32.0, 0.5)];
    assert_eq!(
        check(&s, &view(&[], &m, &[]), always),
        Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker: Some(2)
        }
    );
}

#[test]
fn the_node_clearance_is_an_edge_distance() {
    let mut s = site(32.0, 32.0);
    s.node_clearance = 2.0;
    // Node radius 1: the edge distance is d - 2. At d = 4 it is 2: allowed.
    assert_eq!(
        check(&s, &view(&[], &[], &[circle(36.0, 32.0, 1.0)]), always),
        Verdict::Valid
    );
    assert_eq!(
        check(&s, &view(&[], &[], &[circle(35.75, 32.0, 1.0)]), always),
        refused(Reason::SiteNearResource)
    );
    // The default clearance 0 refuses only an overlap with the node.
    s.node_clearance = 0.0;
    assert_eq!(
        check(&s, &view(&[], &[], &[circle(34.0, 32.0, 1.0)]), always),
        Verdict::Valid
    );
    assert_eq!(
        check(&s, &view(&[], &[], &[circle(33.75, 32.0, 1.0)]), always),
        refused(Reason::SiteNearResource)
    );
}

#[test]
fn a_hidden_static_inside_the_gap_is_site_blocked_with_no_id() {
    // Review finding 6, design 6.1: a blocker that the viewer cannot see gives `site_blocked`
    // with no unit id. A hidden building inside the gap blocks the site as much as an
    // overlap does, and `site_near_building` would show that a building is there.
    let hidden = |id: u32| id != 8;
    let b = [building(8, 35.5, 32.0, 1.0, 0.5)];
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &b, &[]), hidden),
        Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker: None
        }
    );
    // The same building in view is `site_near_building`.
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &b, &[]), always),
        refused(Reason::SiteNearBuilding)
    );
    // A visible building inside the gap comes first: the viewer sees a building that is too close.
    let two = [
        building(8, 35.5, 32.0, 1.0, 0.5),
        building(9, 28.5, 32.0, 1.0, 0.5),
    ];
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &two, &[]), hidden),
        refused(Reason::SiteNearBuilding)
    );
}

#[test]
fn a_hidden_blocker_is_refused_and_never_named() {
    let hidden = |id: u32| id != 5;
    let m = [mobile(5, 32.0, 32.0, 1.0)];
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &m, &[]), hidden),
        Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker: None
        }
    );
    // A visible blocker is named, and the lowest id of the visible ones is the one named.
    let two = [
        mobile(9, 32.0, 32.0, 1.0),
        mobile(6, 32.5, 32.0, 1.0),
        mobile(5, 32.0, 32.5, 1.0),
    ];
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &two, &[]), hidden),
        Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker: Some(6)
        }
    );
    // The verdict is the same with the predicate always true, apart from the name.
    assert_eq!(
        check(&site(32.0, 32.0), &view(&[], &m, &[]), always),
        Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker: Some(5)
        }
    );
}

/// Source text of `placement.rs` with the line comments removed.
fn code_tokens(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for line in source.lines() {
        let code = line.split("//").next().unwrap_or("");
        let mut word = String::new();
        for c in code.chars().chain(std::iter::once(' ')) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
            } else if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
        }
    }
    tokens
}

const BANNED: [&str; 9] = [
    "sqrt", "mul_add", "sin", "cos", "atan", "atan2", "hypot", "powf", "powi",
];

fn banned_in(source: &str) -> Vec<String> {
    code_tokens(source)
        .into_iter()
        .filter(|t| BANNED.contains(&t.as_str()))
        .collect()
}

#[test]
fn placement_arithmetic_uses_squared_distances_only() {
    // Design section 9: no square root, no trigonometry and no fused multiply-add, so that
    // the f32 result is the same on wasm32 and native.
    let source = include_str!("../src/placement.rs");
    assert_eq!(banned_in(source), Vec::<String>::new());
}

#[test]
fn the_source_scan_can_go_red() {
    // Instrument check: the scan finds each banned word in code and ignores a comment.
    assert_eq!(banned_in("let d = (dx * dx + dy * dy).sqrt();"), ["sqrt"]);
    assert_eq!(
        banned_in("let a = x.mul_add(y, z); let b = f32::atan2(y, x);"),
        ["mul_add", "atan2"]
    );
    assert_eq!(
        banned_in("let b = angle.sin() + angle.cos();"),
        ["sin", "cos"]
    );
    assert!(banned_in("// uses no sqrt\nlet single = using + cost;").is_empty());
}

/// The oracle: the causes that fail, in design order, from distances with a square root, in f64.
fn oracle(site: &Site, v: &View<'_>, visible: impl Fn(u32) -> bool) -> Verdict {
    let d = |a: &Circle, b: &Circle| {
        let dx = f64::from(a.centre.x) - f64::from(b.centre.x);
        let dy = f64::from(a.centre.y) - f64::from(b.centre.y);
        (dx * dx + dy * dy).sqrt()
    };
    let f = f64::from;
    let c = &site.footprint;
    let (x, y, r) = (f(c.centre.x), f(c.centre.y), f(c.radius));
    if x - r < 0.0 || y - r < 0.0 || x + r > f(v.map_width) || y + r > f(v.map_height) {
        return refused(Reason::SiteOutsideMap);
    }
    if site.in_build_area
        && !v
            .build_sources
            .iter()
            .any(|s| d(c, &s.footprint) - f(s.footprint.radius) <= f(s.build_area))
    {
        return refused(Reason::SiteOutOfArea);
    }
    let overlapping: Vec<u32> = v
        .obstacles
        .iter()
        .filter(|o| d(c, &o.footprint) < r + f(o.footprint.radius))
        .map(|o| o.id)
        .collect();
    if !overlapping.is_empty() {
        let blocker = overlapping.into_iter().filter(|id| visible(*id)).min();
        return Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker,
        };
    }
    let mut hidden_in_gap = false;
    for o in v.obstacles {
        if let ObstacleKind::Static { gap } = o.kind {
            let edge = d(c, &o.footprint) - r - f(o.footprint.radius);
            if edge < f(gap).max(f(site.gap)) {
                if visible(o.id) {
                    return refused(Reason::SiteNearBuilding);
                }
                hidden_in_gap = true;
            }
        }
    }
    if hidden_in_gap {
        return refused(Reason::SiteBlocked);
    }
    for n in v.nodes {
        if d(c, n) - r - f(n.radius) < f(site.node_clearance) {
            return refused(Reason::SiteNearResource);
        }
    }
    if let Some(q) = site.queue
        && q.held >= q.capacity
    {
        return refused(Reason::ConstructionQueueFull);
    }
    Verdict::Valid
}

#[test]
fn the_first_failing_check_in_design_order_is_the_reported_reason() {
    let mut rng = Rng::new(0x0000_0000_91AC_0001);
    let mut seen = [0u32; 7];
    for _ in 0..40_000 {
        // Quarter-grid values are exact in f32, so the f64 oracle with a square root agrees
        // with squared distances at the touching boundary.
        let s = Site {
            footprint: circle(
                rng.quarter(0.0, 40.0),
                rng.quarter(0.0, 40.0),
                rng.quarter(0.25, 3.0),
            ),
            gap: rng.quarter(0.0, 3.0),
            node_clearance: rng.quarter(0.0, 3.0),
            in_build_area: rng.chance(2),
            queue: rng.chance(2).then(|| QueueFill {
                held: rng.below(7) as u32,
                capacity: 5,
            }),
        };
        let sources: Vec<BuildSource> = (0..rng.below(3))
            .map(|_| BuildSource {
                footprint: circle(
                    rng.quarter(0.0, 40.0),
                    rng.quarter(0.0, 40.0),
                    rng.quarter(0.5, 3.0),
                ),
                build_area: rng.quarter(2.0, 14.0),
            })
            .collect();
        let obstacles: Vec<Obstacle> = (0..rng.below(6))
            .map(|i| {
                let c = circle(
                    rng.quarter(0.0, 40.0),
                    rng.quarter(0.0, 40.0),
                    rng.quarter(0.25, 3.0),
                );
                let kind = if rng.chance(2) {
                    ObstacleKind::Mobile
                } else {
                    ObstacleKind::Static {
                        gap: rng.quarter(0.0, 3.0),
                    }
                };
                Obstacle {
                    id: 10 + i as u32 * 7,
                    footprint: c,
                    kind,
                }
            })
            .collect();
        let nodes: Vec<Circle> = (0..rng.below(3))
            .map(|_| {
                circle(
                    rng.quarter(0.0, 40.0),
                    rng.quarter(0.0, 40.0),
                    rng.quarter(0.25, 2.0),
                )
            })
            .collect();
        let v = View {
            map_width: 36.0,
            map_height: 36.0,
            build_sources: &sources,
            obstacles: &obstacles,
            nodes: &nodes,
        };
        let visible = |id: u32| !id.is_multiple_of(3);
        let got = check(&s, &v, visible);
        assert_eq!(got, oracle(&s, &v, visible), "site {s:?} view {v:?}");
        match got {
            Verdict::Valid => seen[6] += 1,
            Verdict::Refused { reason, .. } => seen[reason as usize] += 1,
        }
    }
    // The case set reaches every cause and the valid verdict, so no branch goes untested.
    assert!(
        seen.iter().all(|n| *n > 100),
        "coverage of the causes: {seen:?}"
    );
}
