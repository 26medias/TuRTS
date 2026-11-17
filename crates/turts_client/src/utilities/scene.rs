//! What the utility view draws, as plain data. Gizmos, the hover panel and the test bridge read
//! it. It is built from the owner view in the replica and nothing else: the client draws no row
//! that it did not receive (spec "Browser mode").

use crate::{content::ClientContent, replica::Replica};
use bevy::prelude::{Color, Vec2, Vec3};
use std::collections::{BTreeMap, HashMap, HashSet};
use turts_mod_api::{ContentId, ResolvedUtility, UtilitySymbol};
use turts_net_protocol::{UtilityEndRecord, UtilitySegmentState, UtilityView};

/// A little above the surface, so that a line on the ground does not z-fight with it.
const LIFT: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortState {
    /// The network of the port is not short.
    Supplied,
    /// The network of the port is short.
    Short,
    /// The port has no segment.
    Unconnected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneSegment {
    pub id: u32,
    pub a: Vec3,
    pub b: Vec3,
    pub colour: Color,
    pub state: UtilitySegmentState,
}

impl SceneSegment {
    pub fn dashed(&self) -> bool {
        self.state == UtilitySegmentState::Disconnected
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneJunction {
    pub id: u32,
    pub at: Vec3,
    pub symbol: UtilitySymbol,
    pub colour: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenePort {
    pub unit: u32,
    /// The port point on the ground.
    pub ground: Vec3,
    pub state: PortState,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    pub segments: Vec<SceneSegment>,
    pub junctions: Vec<SceneJunction>,
    pub ports: Vec<ScenePort>,
}

pub fn parse(colour: &str) -> Color {
    crate::widgets::primitives::parse_colour(colour).unwrap_or(Color::WHITE)
}

/// The world height of a terrain elevation. A map without terrain has the flat height 0.
pub fn elevation_height(replica: &Replica, elevation: i32) -> f32 {
    replica
        .terrain
        .as_ref()
        .map_or(0., |terrain| terrain.layout.elevation_height(elevation))
}

/// The height of the permitted surface at a plan point, for a mark on the ground.
pub fn ground_height(replica: &Replica, point: Vec2) -> f32 {
    crate::terrain::render::marker_height(
        replica.terrain.as_ref(),
        turts_net_protocol::Pos {
            x: point.x,
            y: point.y,
        },
    )
    .unwrap_or(0.)
}

pub fn utility<'a>(content: &'a ClientContent, id: &str) -> Option<&'a ResolvedUtility> {
    content.catalog.utilities.get(&ContentId::parse(id).ok()?)
}

/// The plan point of the port of an own unit, when the unit and its port are known.
pub fn port_point(
    replica: &Replica,
    content: &ClientContent,
    utility_id: &str,
    unit: u32,
) -> Option<Vec2> {
    let definition = utility(content, utility_id)?;
    let unit = replica.units.get(&unit)?;
    let offset = definition
        .ports
        .get(&ContentId::parse(&unit.content_id).ok()?)?;
    let (_, position) = unit.samples.back()?;
    Some(Vec2::new(position.x + offset.x, position.y + offset.y))
}

/// The state of a port: grey with no segment, else the state of its network.
pub fn port_state(view: &UtilityView, unit: u32) -> PortState {
    let has_segment = view.segments.iter().any(|segment| {
        segment.a == UtilityEndRecord::Port(turts_net_protocol::UnitId(unit))
            || segment.b == UtilityEndRecord::Port(turts_net_protocol::UnitId(unit))
    });
    if !has_segment {
        return PortState::Unconnected;
    }
    let short = view
        .networks
        .iter()
        .any(|network| network.short && network.units.iter().any(|id| id.0 == unit));
    if short {
        PortState::Short
    } else {
        PortState::Supplied
    }
}

pub fn build(replica: &Replica, content: &ClientContent) -> Scene {
    let Some(view) = replica.utilities.as_ref() else {
        return Scene::default();
    };
    let nodes: HashMap<u32, (&str, Vec2)> = view
        .nodes
        .iter()
        .map(|node| {
            (
                node.id,
                (node.utility.as_str(), Vec2::new(node.pos.x, node.pos.y)),
            )
        })
        .collect();
    let mut scene = Scene::default();
    let mut ports = BTreeMap::<u32, String>::new();
    for segment in &view.segments {
        let Some(definition) = utility(content, &segment.utility) else {
            continue;
        };
        let height = elevation_height(replica, segment.elevation);
        let mut end = |end: UtilityEndRecord| -> Option<Vec3> {
            let point = match end {
                UtilityEndRecord::Node(id) => nodes.get(&id)?.1,
                UtilityEndRecord::Port(unit) => {
                    ports.insert(unit.0, segment.utility.clone());
                    port_point(replica, content, &segment.utility, unit.0)?
                }
            };
            Some(Vec3::new(point.x, height, point.y))
        };
        let (Some(a), Some(b)) = (end(segment.a), end(segment.b)) else {
            continue;
        };
        let appearance = &definition.appearance;
        scene.segments.push(SceneSegment {
            id: segment.id,
            a,
            b,
            colour: parse(if segment.state == UtilitySegmentState::Conducting {
                &appearance.colour
            } else {
                &appearance.exposed_colour
            }),
            state: segment.state,
        });
    }
    for node in &view.nodes {
        let Some(definition) = utility(content, &node.utility) else {
            continue;
        };
        // A junction sits at the elevation of its segments. With none, it has no body to draw.
        let Some(segment) = view.segments.iter().find(|segment| {
            segment.a == UtilityEndRecord::Node(node.id)
                || segment.b == UtilityEndRecord::Node(node.id)
        }) else {
            continue;
        };
        scene.junctions.push(SceneJunction {
            id: node.id,
            at: Vec3::new(
                node.pos.x,
                elevation_height(replica, segment.elevation),
                node.pos.y,
            ),
            symbol: definition.appearance.junction,
            colour: parse(&definition.appearance.colour),
        });
    }
    // Every own port unit has a marker, with or without a segment (grey).
    let mut listed = HashSet::new();
    for network in &view.networks {
        for unit in &network.units {
            if listed.insert(unit.0) {
                ports
                    .entry(unit.0)
                    .or_insert_with(|| network.utility.clone());
            }
        }
    }
    for (unit, utility_id) in ports {
        let Some(point) = port_point(replica, content, &utility_id, unit) else {
            continue;
        };
        scene.ports.push(ScenePort {
            unit,
            ground: Vec3::new(point.x, ground_height(replica, point) + LIFT, point.y),
            state: port_state(view, unit),
        });
    }
    scene
}

fn number(value: f32) -> serde_json::Value {
    serde_json::Number::from_f64(f64::from(value))
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

fn triple(value: Vec3) -> serde_json::Value {
    serde_json::Value::Array(vec![number(value.x), number(value.y), number(value.z)])
}

fn pair(value: Option<Vec2>) -> serde_json::Value {
    value.map_or(serde_json::Value::Null, |value| {
        serde_json::Value::Array(vec![number(value.x), number(value.y)])
    })
}

fn state_name(state: UtilitySegmentState) -> &'static str {
    match state {
        UtilitySegmentState::Conducting => "conducting",
        UtilitySegmentState::Exposed => "exposed",
        UtilitySegmentState::Disconnected => "disconnected",
    }
}

fn symbol_name(symbol: UtilitySymbol) -> &'static str {
    match symbol {
        UtilitySymbol::Circle => "circle",
        UtilitySymbol::Square => "square",
        UtilitySymbol::Diamond => "diamond",
    }
}

/// The `utilities` entry of `window.turts_scene`, for the browser oracle only. It lists what the
/// view draws, with the screen point of each end (`project` maps a scene point to the window).
/// It holds only own-view rows, so it gives the oracle nothing that the player cannot see.
pub fn bridge(
    replica: &Replica,
    content: &ClientContent,
    output: &turts_interaction::utility::Output,
    project: &dyn Fn(Vec3) -> Option<Vec2>,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "view_on": output.view_on,
        "route_mode": output.route.is_some(),
        "route": output.route.as_ref().map(|route| serde_json::json!({
            "utility": route.utility,
            "depth": route.depth,
            "start": pair(route.start),
            "end": pair(route.end),
            "length": route.length.map(number),
            "reason": route.reason,
        })),
        "preview_screen": null,
        "text": super::panel::panel_text(replica, content, output),
        "segments": [],
        "ports": [],
        "junctions": [],
    });
    if !output.view_on {
        return value;
    }
    if let Some((a, b)) = super::render::preview_line(replica, output) {
        value["preview_screen"] = serde_json::json!({"a": pair(project(a)), "b": pair(project(b))});
    }
    let scene = build(replica, content);
    let utility_of = |id: u32| {
        replica
            .utilities
            .as_ref()
            .and_then(|view| view.segments.iter().find(|segment| segment.id == id))
            .map(|segment| segment.utility.clone())
    };
    value["segments"] = scene
        .segments
        .iter()
        .map(|segment| {
            let colour = segment.colour.to_srgba();
            let colour = [colour.red, colour.green, colour.blue, colour.alpha].map(number);
            serde_json::json!({
                "id": segment.id,
                "utility": utility_of(segment.id),
                "state": state_name(segment.state),
                "dashed": segment.dashed(),
                "colour": colour,
                "a": triple(segment.a),
                "b": triple(segment.b),
                "a_screen": pair(project(segment.a)),
                "b_screen": pair(project(segment.b)),
            })
        })
        .collect();
    value["ports"] = scene
        .ports
        .iter()
        .map(|port| {
            serde_json::json!({
                "unit": port.unit,
                "state": match port.state {
                    PortState::Supplied => "supplied",
                    PortState::Short => "short",
                    PortState::Unconnected => "unconnected",
                },
                "world": triple(port.ground),
                "screen": pair(project(port.ground)),
            })
        })
        .collect();
    value["junctions"] = scene
        .junctions
        .iter()
        .map(|junction| {
            serde_json::json!({
                "id": junction.id,
                "symbol": symbol_name(junction.symbol),
                "world": triple(junction.at),
                "screen": pair(project(junction.at)),
            })
        })
        .collect();
    value
}
