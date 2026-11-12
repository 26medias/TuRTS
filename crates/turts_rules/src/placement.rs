//! The placement check (design 6.1, "Refusal causes" and "One pure function, with
//! visibility"). The checks run in the order of `Reason::ALL`, and the first failure is the
//! cause. Every comparison uses squared distances.

/// A point on the map, `f32` (the crate has no vector type: the lockfile has two `glam`
/// versions, design section 9).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A circular footprint (`Footprint::Circle`, `schema.rs:28-30`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub centre: Point,
    pub radius: f32,
}

/// The queue state of the constructor, for check 6. `held` is the number of held orders;
/// `capacity` is `queue_capacity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFill {
    pub held: u32,
    pub capacity: u32,
}

/// The site to check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Site {
    pub footprint: Circle,
    /// `placement.gap` of the new building.
    pub gap: f32,
    /// `placement.node_clearance`.
    pub node_clearance: f32,
    /// `placement.in_build_area`.
    pub in_build_area: bool,
    /// `Some` for a new order. `None` for the placement of a held order, which runs checks 1
    /// to 5 again (design 6.1, last paragraph of the refusal causes).
    pub queue: Option<QueueFill>,
}

/// A completed own unit that declares `build_area`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildSource {
    pub footprint: Circle,
    pub build_area: f32,
}

/// What an obstacle is for the gap rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObstacleKind {
    /// A static footprint (a building or a site) with its own `placement.gap`.
    Static { gap: f32 },
    /// A mobile unit: it counts by overlap only, never in the gap.
    Mobile,
}

/// A footprint on the map that can block a site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    pub id: u32,
    pub footprint: Circle,
    pub kind: ObstacleKind,
}

/// The world as the caller sees it. The server fills it from the authoritative world and the
/// interaction core fills it from the replica.
#[derive(Debug, Clone, Copy)]
pub struct View<'a> {
    pub map_width: f32,
    pub map_height: f32,
    pub build_sources: &'a [BuildSource],
    pub obstacles: &'a [Obstacle],
    /// The footprints of the resource nodes.
    pub nodes: &'a [Circle],
}

/// The refusal causes, in check order (design 6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    SiteOutsideMap,
    SiteOutOfArea,
    SiteBlocked,
    SiteNearBuilding,
    SiteNearResource,
    ConstructionQueueFull,
}

impl Reason {
    /// Every cause, in check order.
    pub const ALL: [Reason; 6] = [
        Reason::SiteOutsideMap,
        Reason::SiteOutOfArea,
        Reason::SiteBlocked,
        Reason::SiteNearBuilding,
        Reason::SiteNearResource,
        Reason::ConstructionQueueFull,
    ];

    /// The reason key of design 6.1 (the key of the mod `reasons` map).
    pub const fn key(self) -> &'static str {
        match self {
            Reason::SiteOutsideMap => "site_outside_map",
            Reason::SiteOutOfArea => "site_out_of_area",
            Reason::SiteBlocked => "site_blocked",
            Reason::SiteNearBuilding => "site_near_building",
            Reason::SiteNearResource => "site_near_resource",
            Reason::ConstructionQueueFull => "construction_queue_full",
        }
    }
}

/// The verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    /// `blocker` is the id of a visible unit that blocks the site (`SiteBlocked` only). It is
    /// `None` when the blocker is hidden: a refusal never names a hidden blocker.
    Refused {
        reason: Reason,
        blocker: Option<u32>,
    },
}

/// Check `site` against `view`. `visible(id)` is the viewer's visibility predicate (always
/// true in MP1).
pub fn check(site: &Site, view: &View<'_>, visible: impl Fn(u32) -> bool) -> Verdict {
    let footprint = &site.footprint;
    // 1. The footprint is fully inside the map. A non-finite value is outside.
    if !inside_map(footprint, view.map_width, view.map_height) {
        return refuse(Reason::SiteOutsideMap);
    }
    // 2. The centre is within `build_area` of the footprint edge of a completed own unit that
    //    declares it. With no such unit, the site is out of the area.
    if site.in_build_area
        && !view.build_sources.iter().any(|source| {
            within(
                footprint.centre,
                source.footprint.centre,
                source.footprint.radius + source.build_area,
            )
        })
    {
        return refuse(Reason::SiteOutOfArea);
    }
    // 3. No overlap with any footprint, a mobile unit included. Touching is not overlap. The
    //    verdict names the lowest id among the blockers that the viewer can see, and no id
    //    when every blocker is hidden.
    let mut overlaps = false;
    let mut blocker: Option<u32> = None;
    for obstacle in view.obstacles {
        if overlap(footprint, &obstacle.footprint) {
            overlaps = true;
            if visible(obstacle.id) {
                blocker = Some(blocker.map_or(obstacle.id, |best| best.min(obstacle.id)));
            }
        }
    }
    if overlaps {
        return Verdict::Refused {
            reason: Reason::SiteBlocked,
            blocker,
        };
    }
    // 4. Between two static footprints the edge distance is at least the larger gap. A mobile
    //    unit counts by overlap only. A building that the viewer cannot see blocks the site
    //    like a hidden overlap does: the cause is `site_blocked` with no id, so that the
    //    refusal never shows that a hidden building is there. A visible building comes first.
    let mut hidden_in_gap = false;
    for obstacle in view.obstacles {
        if let ObstacleKind::Static { gap } = obstacle.kind
            && edge_below(footprint, &obstacle.footprint, site.gap.max(gap))
        {
            if visible(obstacle.id) {
                return refuse(Reason::SiteNearBuilding);
            }
            hidden_in_gap = true;
        }
    }
    if hidden_in_gap {
        return refuse(Reason::SiteBlocked);
    }
    // 5. The edge distance to a resource node is at least `node_clearance`.
    for node in view.nodes {
        if edge_below(footprint, node, site.node_clearance) {
            return refuse(Reason::SiteNearResource);
        }
    }
    // 6. The constructor queue is not full. A held order that is placed again skips this.
    if let Some(queue) = site.queue
        && queue.held >= queue.capacity
    {
        return refuse(Reason::ConstructionQueueFull);
    }
    Verdict::Valid
}

fn refuse(reason: Reason) -> Verdict {
    Verdict::Refused {
        reason,
        blocker: None,
    }
}

/// The square of a distance.
fn distance_squared(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// The distance from `a` to `b` is at most `limit` (compared as squares; `limit` is not
/// negative).
fn within(a: Point, b: Point, limit: f32) -> bool {
    distance_squared(a, b) <= limit * limit
}

/// The two circles overlap: the centre distance is below the sum of the radii.
fn overlap(a: &Circle, b: &Circle) -> bool {
    let reach = a.radius + b.radius;
    distance_squared(a.centre, b.centre) < reach * reach
}

/// The edge distance between two circles is below `limit`. For a `limit` below 0 nothing is
/// below it, and the circles may overlap by that much.
fn edge_below(a: &Circle, b: &Circle, limit: f32) -> bool {
    let reach = a.radius + b.radius + limit;
    if reach <= 0.0 {
        return false;
    }
    distance_squared(a.centre, b.centre) < reach * reach
}

fn inside_map(circle: &Circle, width: f32, height: f32) -> bool {
    let Circle {
        centre: Point { x, y },
        radius,
    } = *circle;
    x.is_finite()
        && y.is_finite()
        && radius.is_finite()
        && x - radius >= 0.0
        && y - radius >= 0.0
        && x + radius <= width
        && y + radius <= height
}
