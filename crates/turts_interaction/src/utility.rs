//! M6 underground utilities: the view toggle and the route mode. Pure state, no renderer.
//!
//! The view shows only the owner rows that the client received (`Facts::view`). Route mode
//! turns two clicks into one `OrderEnvelope` with the target `UtilityRoute`. The client snaps a
//! click to an own port, an own junction or an own segment before it sends (spec "Browser mode").
//! The server never snaps. A reason that the client finds here is advice; the server decides.

use super::{CLICK_DRAG_PIXELS, Core, Effect, InputEvent, Vec2, Viewport, WorldView, projection};
use std::collections::{BTreeMap, HashMap, HashSet};
use turts_mod_api::{
    Behaviour, Button, MAX_UTILITY_JUNCTIONS_PER_SLOT, MAX_UTILITY_SEGMENT_CELLS,
    MAX_UTILITY_SEGMENTS_PER_SLOT, TargetKind, TerrainCell, UTILITY_MIN_SEGMENT_LENGTH,
    UtilityEndpoint, UtilityRouteTarget, WorldPos,
};
use turts_net_protocol::{
    OrderEnvelope, OrderTarget, QueueMode, UnitId, UtilityEndRecord, UtilityView,
};
use turts_rules::utilities as geometry;

/// A click or a hover snaps to an own object within this plan distance, in world units.
pub const SNAP_RADIUS: f32 = 0.6;

/// The engine action names of the utility view. The engine input map lists their default keys.
pub const ACTION_VIEW: &str = "utility_view";
pub const ACTION_DEEPER: &str = "utility_deeper";
pub const ACTION_SHALLOWER: &str = "utility_shallower";

/// Reason keys that the preview and a refused click report.
pub mod reason {
    pub const TOO_SHORT: &str = "too_short";
    pub const TOO_LONG: &str = "too_long";
    pub const TOO_MANY_CELLS: &str = "too_many_cells";
    pub const OUTSIDE_MAP: &str = "outside_map";
    pub const DUPLICATE: &str = "duplicate_segment";
    pub const OUT_OF_REACH: &str = "out_of_reach";
    pub const NOT_OBSERVED: &str = "not_observed";
    pub const BELOW_FLOOR: &str = "below_floor";
    pub const LIMIT: &str = "limit_reached";
}

/// The words for a reason key. The notice and the route preview use them.
pub fn reason_text(key: &str) -> &'static str {
    match key {
        reason::TOO_SHORT => "The line is too short",
        reason::TOO_LONG => "The line is too long",
        reason::TOO_MANY_CELLS => "The line crosses too many cells",
        reason::OUTSIDE_MAP => "The line leaves the map",
        reason::DUPLICATE => "These two points already have a line",
        reason::OUT_OF_REACH => "Out of reach of the selected builders",
        reason::NOT_OBSERVED => "Not observed from permitted terrain",
        reason::BELOW_FLOOR => "The line would go below the terrain floor",
        reason::LIMIT => "Utility limit reached",
        _ => "The line is refused",
    }
}

/// What the client knows about one utility type (from the validated catalog).
#[derive(Debug, Clone, PartialEq)]
pub struct UtilityFact {
    pub name: String,
    pub depth_default: u32,
    pub depth_min: u32,
    pub depth_max: u32,
    pub reach: f32,
    pub max_segment_length: f32,
    /// `(resource display name, amount)` for each new segment.
    pub cost: Vec<(String, u64)>,
    /// Port offset by unit content id, in world axes.
    pub ports: BTreeMap<String, Vec2>,
}

/// The world data of the utility view: the catalog part and the owner rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    /// By package-qualified utility id.
    pub utilities: BTreeMap<String, UtilityFact>,
    /// The owner view as received. Empty when the catalog has no utility type.
    pub view: UtilityView,
}

/// One endpoint of a route as the client resolved it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Endpoint {
    Port { unit: UnitId, point: Vec2 },
    Node { id: u32, point: Vec2 },
    Split { segment: u32, point: Vec2 },
    Free { point: Vec2 },
}

impl Endpoint {
    pub fn point(&self) -> Vec2 {
        match *self {
            Self::Port { point, .. }
            | Self::Node { point, .. }
            | Self::Split { point, .. }
            | Self::Free { point } => point,
        }
    }

    pub fn wire(&self) -> UtilityEndpoint {
        let world = |point: Vec2| WorldPos {
            x: point.x,
            y: point.y,
        };
        match *self {
            Self::Port { unit, .. } => UtilityEndpoint::Port(unit.0),
            Self::Node { id, .. } => UtilityEndpoint::Node(id),
            Self::Split { segment, point } => UtilityEndpoint::Split {
                segment,
                point: world(point),
            },
            Self::Free { point } => UtilityEndpoint::Free(world(point)),
        }
    }

    pub fn kind(&self) -> SnapKind {
        match self {
            Self::Port { .. } => SnapKind::Port,
            Self::Node { .. } => SnapKind::Node,
            Self::Split { .. } => SnapKind::Split,
            Self::Free { .. } => SnapKind::Free,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    Port,
    Node,
    Split,
    Free,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteMode {
    pub order: String,
    pub utility: String,
    pub depth: u32,
    /// The point of the first click. It is resolved again when the route is sent, so that the
    /// end of the last route (a free point that became a junction) snaps to that junction.
    pub start: Option<Endpoint>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    view_on: bool,
    route: Option<RouteMode>,
    pressed: Option<(Button, Vec2)>,
    shown: Output,
}

impl State {
    pub fn view_on(&self) -> bool {
        self.view_on
    }

    pub fn route(&self) -> Option<&RouteMode> {
        self.route.as_ref()
    }
}

/// A typed choice from a mod-authored `UtilityRoutes` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub order: String,
    pub utility: String,
}

/// What the client can say about the segment under the pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutePreview {
    pub utility: String,
    pub name: String,
    pub depth: u32,
    pub depth_min: u32,
    pub depth_max: u32,
    pub start: Option<Vec2>,
    /// The snapped point under the pointer.
    pub end: Option<Vec2>,
    pub end_kind: Option<SnapKind>,
    pub length: Option<f32>,
    pub cost: Vec<(String, u64)>,
    /// A reason key of `reason`, when the client knows one.
    pub reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverTarget {
    Segment(u32),
    Node(u32),
    Port(UnitId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hover {
    pub target: HoverTarget,
    /// The terrain cell under the pointer, for the cover line of the panel.
    pub cell: Option<TerrainCell>,
}

/// The effect of the utility view. The core sends it only when it changes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {
    pub view_on: bool,
    pub route: Option<RoutePreview>,
    pub hover: Option<Hover>,
}

impl Core {
    pub fn utility_state(&self) -> &State {
        &self.utility
    }
}

pub(crate) fn is_action(action: &str) -> bool {
    matches!(action, ACTION_VIEW | ACTION_DEEPER | ACTION_SHALLOWER)
}

/// Turns the view on or off, or changes the route depth. The caller recorded the dispatch.
pub(crate) fn on_action(
    core: &mut Core,
    action: &str,
    world: &dyn WorldView,
    effects: &mut Vec<Effect>,
) {
    match action {
        ACTION_VIEW if !world.utility_facts().utilities.is_empty() => {
            if core.utility.view_on {
                core.utility.view_on = false;
                end_route(core, effects);
            } else {
                core.utility.view_on = true;
            }
        }
        ACTION_DEEPER | ACTION_SHALLOWER => {
            let Some(route) = core.utility.route.as_mut() else {
                return;
            };
            let Some(fact) = world.utility_facts().utilities.get(&route.utility) else {
                return;
            };
            route.depth = if action == ACTION_DEEPER {
                route.depth.saturating_add(1).min(fact.depth_max)
            } else {
                route.depth.saturating_sub(1).max(fact.depth_min)
            };
        }
        _ => {}
    }
}

/// Own selected units that the order lists. They are the issuing units of a route.
fn eligible(core: &Core, order: &str, world: &dyn WorldView) -> Vec<UnitId> {
    let Some(definition) = world.orders().iter().find(|definition| {
        definition.id == order
            && definition.behaviour == Behaviour::UtilityRoute
            && definition.target == TargetKind::UtilityRoute
    }) else {
        return Vec::new();
    };
    core.selected
        .iter()
        .copied()
        .filter(|id| {
            world.units().iter().any(|unit| {
                unit.id == *id
                    && unit.owner_slot == world.own_slot()
                    && (definition.units.is_empty() || definition.units.contains(&unit.content_id))
            })
        })
        .collect()
}

pub(crate) fn end_route(core: &mut Core, effects: &mut Vec<Effect>) {
    let Some(route) = core.utility.route.take() else {
        return;
    };
    core.utility.pressed = None;
    core.pointer_down = None;
    if core.pending_order.as_deref() == Some(route.order.as_str()) {
        core.pending_order = None;
    }
    effects.push(Effect::Cursor {
        context: "default".into(),
    });
}

pub(crate) fn on_input(
    core: &mut Core,
    input: &Input,
    world: &dyn WorldView,
    effects: &mut Vec<Effect>,
) {
    if core.ui_capture.is_some() {
        return;
    }
    let Some(fact) = world.utility_facts().utilities.get(&input.utility) else {
        return;
    };
    if eligible(core, &input.order, world).is_empty() {
        effects.push(Effect::Notice {
            text_key: "No selected builder can route this utility".into(),
            units: vec![],
        });
        return;
    }
    super::terrain::cancel(core, effects);
    core.placement = Default::default();
    core.pointer_down = None;
    core.pending_order = Some(input.order.clone());
    core.utility.pressed = None;
    core.utility.view_on = true;
    core.utility.route = Some(RouteMode {
        order: input.order.clone(),
        utility: input.utility.clone(),
        depth: fact.depth_default,
        start: None,
    });
    effects.push(Effect::Cursor {
        context: "target".into(),
    });
}

/// The ground point under a pointer, from the same ray as every other pointer order.
fn ground(
    core: &Core,
    px: Vec2,
    world: &dyn WorldView,
    view: &Viewport,
) -> Option<crate::SurfaceHit> {
    projection::surface_hit(px, view, core.camera, world)
}

pub(crate) fn intercept(
    core: &mut Core,
    event: &InputEvent,
    world: &dyn WorldView,
    view: &Viewport,
    effects: &mut Vec<Effect>,
) -> bool {
    let Some(route) = core.utility.route.as_ref() else {
        return false;
    };
    if !world.utility_facts().utilities.contains_key(&route.utility)
        || eligible(core, &route.order, world).is_empty()
    {
        end_route(core, effects);
        return false;
    }
    match event {
        InputEvent::KeyDown { code, .. }
            if core.action_for_key(code, world).as_deref() == Some("cancel") =>
        {
            end_route(core, effects);
            true
        }
        InputEvent::UiAction { name } if name == "cancel" => {
            end_route(core, effects);
            true
        }
        InputEvent::FocusLost | InputEvent::UiCapture { .. } => {
            end_route(core, effects);
            false
        }
        InputEvent::PointerMove { px, over_widget } => {
            core.pointer = Some((*px, *over_widget));
            true
        }
        InputEvent::PointerDown {
            button,
            px,
            over_widget: false,
        } if matches!(button, Button::Left | Button::Right) && core.ui_capture.is_none() => {
            core.pointer = Some((*px, false));
            core.pointer_down = None;
            if *button == Button::Right {
                end_route(core, effects);
            } else {
                core.utility.pressed = Some((*button, *px));
            }
            true
        }
        InputEvent::PointerUp {
            button: Button::Left,
            px,
            over_widget,
        } => {
            let pressed = core.utility.pressed.take();
            core.pointer_down = None;
            core.held_buttons.retain(|held| *held != Button::Left);
            core.held_modifier_actions.remove("pointer:Left");
            if *over_widget || core.ui_capture.is_some() {
                return true;
            }
            if pressed.is_some_and(|(_, start)| start.distance(*px) <= CLICK_DRAG_PIXELS)
                && let Some(hit) = ground(core, *px, world, view)
            {
                click(core, hit, world, effects);
            }
            true
        }
        _ => false,
    }
}

/// One left click: the first sets the start; the second sends the route.
fn click(
    core: &mut Core,
    hit: crate::SurfaceHit,
    world: &dyn WorldView,
    effects: &mut Vec<Effect>,
) {
    let Some(route) = core.utility.route.clone() else {
        return;
    };
    let end = resolve(hit.position, &route.utility, world);
    let Some(start) = route.start else {
        if let Some(active) = core.utility.route.as_mut() {
            active.start = Some(end);
        }
        return;
    };
    // The last end may have become a junction since: snap the start again.
    let start = resolve(start.point(), &route.utility, world);
    let units = eligible(core, &route.order, world);
    let verdict = judge(&route, start, end, &units, world);
    if let Some(found) = verdict.reason
        && found.blocking
    {
        effects.push(Effect::Notice {
            text_key: reason_text(found.key).into(),
            units: units.clone(),
        });
        return;
    }
    let group = if units.len() > 1 {
        core.alloc_group()
    } else {
        0
    };
    for chunk in units.chunks(turts_net_protocol::MAX_ORDER_UNITS) {
        effects.push(Effect::Send(OrderEnvelope {
            order: route.order.clone(),
            units: chunk.to_vec(),
            target: OrderTarget::UtilityRoute(UtilityRouteTarget {
                utility: route.utility.clone(),
                depth: route.depth,
                from: start.wire(),
                to: end.wire(),
            }),
            queue: QueueMode::Replace,
            group,
        }));
    }
    if let Some(active) = core.utility.route.as_mut() {
        active.start = Some(end);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Found {
    key: &'static str,
    /// The client is sure: it never sends the route.
    blocking: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Verdict {
    length: f32,
    reason: Option<Found>,
}

fn blocked(key: &'static str) -> Option<Found> {
    Some(Found {
        key,
        blocking: true,
    })
}

fn advice(key: &'static str) -> Option<Found> {
    Some(Found {
        key,
        blocking: false,
    })
}

/// The plan grid that the server uses: the terrain cells, or 1-unit cells without terrain.
fn plan_grid(world: &dyn WorldView) -> Option<(f32, u32, u32)> {
    if let Some(grid) = world.terrain_grid() {
        return Some((
            grid.cell_width as f32 / grid.length_scale as f32,
            grid.columns,
            grid.rows,
        ));
    }
    let size = world.map_size()?;
    Some((1.0, size.x.ceil() as u32, size.y.ceil() as u32))
}

/// The client checks of one segment, in the order of the server (spec "Placement validation").
fn judge(
    route: &RouteMode,
    start: Endpoint,
    end: Endpoint,
    units: &[UnitId],
    world: &dyn WorldView,
) -> Verdict {
    let facts = world.utility_facts();
    let (a, b) = (start.point(), end.point());
    let length = geometry::length((a.x, a.y), (b.x, b.y));
    let verdict = |reason| Verdict { length, reason };
    let Some(fact) = facts.utilities.get(&route.utility) else {
        return verdict(None);
    };
    if route.depth < fact.depth_min || route.depth > fact.depth_max {
        return verdict(None);
    }
    if length <= UTILITY_MIN_SEGMENT_LENGTH || start == end {
        return verdict(blocked(reason::TOO_SHORT));
    }
    if length > fact.max_segment_length {
        return verdict(blocked(reason::TOO_LONG));
    }
    if duplicate(start, end, &route.utility, facts) {
        return verdict(blocked(reason::DUPLICATE));
    }
    let within = |unit: &crate::UnitView| {
        units.contains(&unit.id)
            && unit.position.distance(a) <= fact.reach
            && unit.position.distance(b) <= fact.reach
    };
    if !world.units().iter().any(within) {
        return verdict(advice(reason::OUT_OF_REACH));
    }
    if let Some((cell_width, columns, rows)) = plan_grid(world) {
        match geometry::touched_cells(
            (a.x, a.y),
            (b.x, b.y),
            cell_width,
            columns,
            rows,
            MAX_UTILITY_SEGMENT_CELLS,
        ) {
            Err(geometry::TouchError::TooManyCells) => {
                return verdict(blocked(reason::TOO_MANY_CELLS));
            }
            Err(geometry::TouchError::OutsideGrid) => {
                return verdict(blocked(reason::OUTSIDE_MAP));
            }
            Err(geometry::TouchError::NotFinite) => return verdict(None),
            Ok(cells) if world.terrain_enabled() => {
                let mut lowest: Option<i32> = None;
                for (x, y) in cells {
                    let cell = TerrainCell { x, y };
                    if !world.terrain_current(cell) {
                        return verdict(advice(reason::NOT_OBSERVED));
                    }
                    let Some(surface) = world.terrain_elevation(cell) else {
                        return verdict(advice(reason::NOT_OBSERVED));
                    };
                    lowest = Some(lowest.map_or(surface, |low| low.min(surface)));
                }
                if let (Some(grid), Some(lowest)) = (world.terrain_grid(), lowest)
                    && geometry::placement_elevation([lowest], route.depth)
                        .is_none_or(|elevation| elevation < grid.floor)
                {
                    return verdict(advice(reason::BELOW_FLOOR));
                }
            }
            Ok(_) => {}
        }
    }
    let new_nodes = [start, end]
        .iter()
        .filter(|end| matches!(end, Endpoint::Free { .. } | Endpoint::Split { .. }))
        .count();
    let splits = [start, end]
        .iter()
        .filter(|end| matches!(end, Endpoint::Split { .. }))
        .count();
    if facts.view.segments.len() + 1 + splits > MAX_UTILITY_SEGMENTS_PER_SLOT
        || facts.view.nodes.len() + new_nodes > MAX_UTILITY_JUNCTIONS_PER_SLOT
    {
        return verdict(advice(reason::LIMIT));
    }
    verdict(None)
}

/// Whether a segment of this utility already joins the same two explicit endpoints.
fn duplicate(start: Endpoint, end: Endpoint, utility: &str, facts: &Facts) -> bool {
    let end_record = |end: Endpoint| match end {
        Endpoint::Port { unit, .. } => Some(UtilityEndRecord::Port(unit)),
        Endpoint::Node { id, .. } => Some(UtilityEndRecord::Node(id)),
        _ => None,
    };
    let (Some(a), Some(b)) = (end_record(start), end_record(end)) else {
        return false;
    };
    facts.view.segments.iter().any(|segment| {
        segment.utility == utility
            && ((segment.a == a && segment.b == b) || (segment.a == b && segment.b == a))
    })
}

/// The own ports of one utility type: `(unit, port point)` for each own unit that has a port.
/// `completed_only` keeps the units that the owner view lists in a network (completed, live).
pub fn own_ports(
    utility: &str,
    completed_only: bool,
    world: &dyn WorldView,
) -> Vec<(UnitId, Vec2)> {
    let facts = world.utility_facts();
    let Some(fact) = facts.utilities.get(utility) else {
        return Vec::new();
    };
    let listed: HashSet<u32> = if completed_only {
        facts
            .view
            .networks
            .iter()
            .filter(|network| network.utility == utility)
            .flat_map(|network| network.units.iter().map(|unit| unit.0))
            .collect()
    } else {
        HashSet::new()
    };
    let mut ports: Vec<_> = world
        .units()
        .iter()
        .filter(|unit| unit.owner_slot == world.own_slot())
        .filter(|unit| !completed_only || listed.contains(&unit.id.0))
        .filter_map(|unit| {
            fact.ports
                .get(&unit.content_id)
                .map(|offset| (unit.id, unit.position + *offset))
        })
        .collect();
    ports.sort_by_key(|(unit, _)| unit.0);
    ports
}

/// The plan points of the own junctions and segment ends of one utility type.
/// A segment id with the plan points of its two ends.
type SegmentPlan = (u32, (f32, f32), (f32, f32));

fn segment_geometry(utility: &str, world: &dyn WorldView) -> Vec<SegmentPlan> {
    let facts = world.utility_facts();
    let nodes: HashMap<u32, Vec2> = facts
        .view
        .nodes
        .iter()
        .filter(|node| node.utility == utility)
        .map(|node| (node.id, Vec2::new(node.pos.x, node.pos.y)))
        .collect();
    let ports: HashMap<u32, Vec2> = own_ports(utility, false, world)
        .into_iter()
        .map(|(unit, point)| (unit.0, point))
        .collect();
    let point = |end: UtilityEndRecord| match end {
        UtilityEndRecord::Node(id) => nodes.get(&id).copied(),
        UtilityEndRecord::Port(unit) => ports.get(&unit.0).copied(),
    };
    facts
        .view
        .segments
        .iter()
        .filter(|segment| segment.utility == utility)
        .filter_map(|segment| {
            let (a, b) = (point(segment.a)?, point(segment.b)?);
            Some((segment.id, (a.x, a.y), (b.x, b.y)))
        })
        .collect()
}

/// What lies within `SNAP_RADIUS` of a ground point: an own port, then an own junction, then an
/// own segment.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Near {
    Port(UnitId, Vec2),
    Node(u32, Vec2),
    Segment {
        id: u32,
        closest: Vec2,
        interior: bool,
    },
}

fn nearest(point: Vec2, utility: &str, world: &dyn WorldView) -> Option<Near> {
    let facts = world.utility_facts();
    let within = |other: Vec2| point.distance(other) <= SNAP_RADIUS;
    if let Some((unit, at)) = own_ports(utility, true, world)
        .into_iter()
        .filter(|(_, at)| within(*at))
        .min_by(|a, b| {
            point
                .distance(a.1)
                .total_cmp(&point.distance(b.1))
                .then(a.0.0.cmp(&b.0.0))
        })
    {
        return Some(Near::Port(unit, at));
    }
    if let Some(node) = facts
        .view
        .nodes
        .iter()
        .filter(|node| node.utility == utility)
        .filter(|node| within(Vec2::new(node.pos.x, node.pos.y)))
        .min_by(|a, b| {
            let distance = |node: &turts_net_protocol::UtilityNodeRecord| {
                point.distance(Vec2::new(node.pos.x, node.pos.y))
            };
            distance(a).total_cmp(&distance(b)).then(a.id.cmp(&b.id))
        })
    {
        return Some(Near::Node(node.id, Vec2::new(node.pos.x, node.pos.y)));
    }
    let p = (point.x, point.y);
    let (id, a, b, _) = segment_geometry(utility, world)
        .into_iter()
        .map(|(id, a, b)| (id, a, b, geometry::distance_to_segment(p, a, b)))
        .filter(|(_, _, _, distance)| *distance <= SNAP_RADIUS)
        .min_by(|x, y| x.3.total_cmp(&y.3).then(x.0.cmp(&y.0)))?;
    let (_, closest) = geometry::project(p, a, b);
    let interior = geometry::length(closest, a) > UTILITY_MIN_SEGMENT_LENGTH
        && geometry::length(closest, b) > UTILITY_MIN_SEGMENT_LENGTH;
    Some(Near::Segment {
        id,
        closest: Vec2::new(closest.0, closest.1),
        interior,
    })
}

/// The route endpoint for a ground point: an own port, an own junction, a split of an own
/// segment (only more than 0.5 from both ends), else a free point.
pub fn resolve(point: Vec2, utility: &str, world: &dyn WorldView) -> Endpoint {
    match nearest(point, utility, world) {
        Some(Near::Port(unit, at)) => Endpoint::Port { unit, point: at },
        Some(Near::Node(id, at)) => Endpoint::Node { id, point: at },
        Some(Near::Segment {
            id,
            closest,
            interior: true,
        }) => Endpoint::Split {
            segment: id,
            point: closest,
        },
        _ => Endpoint::Free { point },
    }
}

fn hover_of(
    core: &Core,
    world: &dyn WorldView,
    view: &Viewport,
) -> Option<(crate::SurfaceHit, Option<Hover>)> {
    let (px, false) = core.pointer? else {
        return None;
    };
    let hit = ground(core, px, world, view)?;
    let facts = world.utility_facts();
    let target = facts.utilities.keys().find_map(|utility| {
        nearest(hit.position, utility, world).map(|near| match near {
            Near::Port(unit, _) => HoverTarget::Port(unit),
            Near::Node(id, _) => HoverTarget::Node(id),
            Near::Segment { id, .. } => HoverTarget::Segment(id),
        })
    });
    Some((
        hit,
        target.map(|target| Hover {
            target,
            cell: hit.cell,
        }),
    ))
}

/// The hook of `Core::step` after the events: the ended modes, then the effect of the view.
pub(crate) fn update(
    core: &mut Core,
    events: &[InputEvent],
    world: &dyn WorldView,
    view: &Viewport,
    effects: &mut Vec<Effect>,
) {
    let facts = world.utility_facts();
    if core.utility.route.as_ref().is_some_and(|route| {
        !facts.utilities.contains_key(&route.utility)
            || events
                .iter()
                .any(|event| matches!(event, InputEvent::FocusLost))
            // Another targeting mode (an ordinary order, terrain work, placement) ended this one.
            || core.pending_order.as_deref() != Some(route.order.as_str())
    }) {
        end_route(core, effects);
    }
    if core.utility.view_on && facts.utilities.is_empty() {
        core.utility.view_on = false;
    }
    let mut output = Output {
        view_on: core.utility.view_on,
        ..Output::default()
    };
    if core.utility.view_on && core.ui_capture.is_none() {
        let hovered = hover_of(core, world, view);
        if let Some((hit, hover)) = hovered {
            output.hover = hover;
            if let Some(route) = core.utility.route.as_ref() {
                output.route = Some(preview(core, route, hit, world));
            }
        }
        if output.route.is_none()
            && let Some(route) = core.utility.route.as_ref()
        {
            output.route = Some(idle_preview(route, world));
        }
    }
    if output != core.utility.shown {
        core.utility.shown = output.clone();
        effects.push(Effect::Utility(output));
    }
}

fn idle_preview(route: &RouteMode, world: &dyn WorldView) -> RoutePreview {
    let facts = world.utility_facts();
    let fact = facts.utilities.get(&route.utility);
    RoutePreview {
        utility: route.utility.clone(),
        name: fact.map(|fact| fact.name.clone()).unwrap_or_default(),
        depth: route.depth,
        depth_min: fact.map_or(route.depth, |fact| fact.depth_min),
        depth_max: fact.map_or(route.depth, |fact| fact.depth_max),
        start: route.start.map(|start| start.point()),
        end: None,
        end_kind: None,
        length: None,
        cost: fact.map(|fact| fact.cost.clone()).unwrap_or_default(),
        reason: None,
    }
}

fn preview(
    core: &Core,
    route: &RouteMode,
    hit: crate::SurfaceHit,
    world: &dyn WorldView,
) -> RoutePreview {
    let mut shown = idle_preview(route, world);
    let end = resolve(hit.position, &route.utility, world);
    shown.end = Some(end.point());
    shown.end_kind = Some(end.kind());
    if let Some(start) = route.start {
        let start = resolve(start.point(), &route.utility, world);
        shown.start = Some(start.point());
        let units = eligible(core, &route.order, world);
        let verdict = judge(route, start, end, &units, world);
        shown.length = Some(verdict.length);
        shown.reason = verdict.reason.map(|found| found.key);
    }
    shown
}
