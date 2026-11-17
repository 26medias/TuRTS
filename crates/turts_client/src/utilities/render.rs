//! The utility view lines: segments at their fixed elevation, junction symbols, port squares
//! and the route preview. Gizmo lines only; nothing here spawns an entity.

use super::scene::{self, PortState, Scene};
use crate::{core_adapter::CoreRuntime, ingest::ReplicaState, join::AcquiredPackages};
use bevy::prelude::*;
use turts_interaction::utility::{Output, RoutePreview, SnapKind};
use turts_mod_api::UtilitySymbol;
use turts_net_protocol::UtilitySegmentState;

const SUPPLIED: Color = Color::srgb(0.25, 0.9, 0.35);
const SHORT: Color = Color::srgb(0.95, 0.25, 0.2);
const UNCONNECTED: Color = Color::srgb(0.6, 0.6, 0.6);
const REFUSED: Color = Color::srgb(1.0, 0.3, 0.25);
/// Half the side of a port square, in world units.
const PORT_HALF: f32 = 0.5;
const JUNCTION_RADIUS: f32 = 0.3;
const DASH: f32 = 0.6;
const GAP: f32 = 0.4;

pub(crate) fn draw(
    runtime: Option<Res<CoreRuntime>>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut gizmos: Gizmos,
) {
    let Some(runtime) = runtime else {
        return;
    };
    let output = runtime.utility_output();
    if !output.view_on {
        return;
    }
    let (Some(replica), Some(content)) = (
        state.replica.as_ref(),
        state
            .content
            .as_ref()
            .or_else(|| acquired.as_ref().map(|acquired| &acquired.0.content)),
    ) else {
        return;
    };
    draw_scene(&mut gizmos, &scene::build(replica, content));
    if let Some(preview) = output.route.as_ref() {
        draw_preview(&mut gizmos, replica, content, preview);
    }
}

fn draw_scene(gizmos: &mut Gizmos, scene: &Scene) {
    for segment in &scene.segments {
        if segment.dashed() {
            dashed(gizmos, segment.a, segment.b, segment.colour);
        } else {
            gizmos.line(segment.a, segment.b, segment.colour);
        }
        // A second line a little above the first keeps a thin line readable on a faded ground.
        let lift = Vec3::Y * 0.04;
        if segment.state != UtilitySegmentState::Disconnected {
            gizmos.line(segment.a + lift, segment.b + lift, segment.colour);
        }
    }
    for junction in &scene.junctions {
        symbol(gizmos, junction.at, junction.symbol, junction.colour);
    }
    for port in &scene.ports {
        let colour = match port.state {
            PortState::Supplied => SUPPLIED,
            PortState::Short => SHORT,
            PortState::Unconnected => UNCONNECTED,
        };
        square(gizmos, port.ground, PORT_HALF, colour);
        square(gizmos, port.ground, PORT_HALF * 0.6, colour);
    }
    // A port is joined to its segment end by a vertical line.
    for segment in &scene.segments {
        for end in [segment.a, segment.b] {
            if let Some(port) = scene.ports.iter().find(|port| {
                (port.ground.x - end.x).abs() < 0.001 && (port.ground.z - end.z).abs() < 0.001
            }) {
                gizmos.line(port.ground, end, segment.colour);
            }
        }
    }
}

fn dashed(gizmos: &mut Gizmos, a: Vec3, b: Vec3, colour: Color) {
    let length = a.distance(b);
    if length <= f32::EPSILON {
        return;
    }
    let direction = (b - a) / length;
    let mut at = 0.0;
    while at < length {
        let end = (at + DASH).min(length);
        gizmos.line(a + direction * at, a + direction * end, colour);
        at += DASH + GAP;
    }
}

fn square(gizmos: &mut Gizmos, centre: Vec3, half: f32, colour: Color) {
    let corner = |x: f32, z: f32| centre + Vec3::new(x * half, 0.0, z * half);
    let corners = [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ];
    for index in 0..4 {
        gizmos.line(corners[index], corners[(index + 1) % 4], colour);
    }
}

fn symbol(gizmos: &mut Gizmos, at: Vec3, symbol: UtilitySymbol, colour: Color) {
    match symbol {
        UtilitySymbol::Square => square(gizmos, at, JUNCTION_RADIUS, colour),
        UtilitySymbol::Diamond => {
            let corners = [
                at + Vec3::new(JUNCTION_RADIUS * 1.3, 0.0, 0.0),
                at + Vec3::new(0.0, 0.0, JUNCTION_RADIUS * 1.3),
                at - Vec3::new(JUNCTION_RADIUS * 1.3, 0.0, 0.0),
                at - Vec3::new(0.0, 0.0, JUNCTION_RADIUS * 1.3),
            ];
            for index in 0..4 {
                gizmos.line(corners[index], corners[(index + 1) % 4], colour);
            }
        }
        UtilitySymbol::Circle => {
            const SIDES: usize = 12;
            let point = |index: usize| {
                let angle = index as f32 / SIDES as f32 * std::f32::consts::TAU;
                at + Vec3::new(angle.cos(), 0.0, angle.sin()) * JUNCTION_RADIUS
            };
            for index in 0..SIDES {
                gizmos.line(point(index), point(index + 1), colour);
            }
        }
    }
}

/// The height of the route preview: as the server sets it, the lowest touched surface minus
/// the depth. A map without terrain draws it at 0.
pub fn preview_height(replica: &crate::replica::Replica, a: Vec2, b: Vec2, depth: u32) -> f32 {
    let Some(terrain) = replica.terrain.as_ref() else {
        return 0.0;
    };
    let grid = terrain.layout.grid;
    let cells = turts_rules::utilities::touched_cells(
        (a.x, a.y),
        (b.x, b.y),
        terrain.layout.cell_width(),
        grid.columns,
        grid.rows,
        turts_mod_api::MAX_UTILITY_SEGMENT_CELLS,
    );
    let lowest = cells.ok().and_then(|cells| {
        cells
            .into_iter()
            .filter_map(|(x, y)| terrain.elevation(turts_mod_api::TerrainCell { x, y }))
            .min()
    });
    match lowest.and_then(|lowest| turts_rules::utilities::placement_elevation([lowest], depth)) {
        Some(elevation) => terrain.layout.elevation_height(elevation),
        None => scene::ground_height(replica, a) - depth as f32,
    }
}

fn draw_preview(
    gizmos: &mut Gizmos,
    replica: &crate::replica::Replica,
    content: &crate::content::ClientContent,
    preview: &RoutePreview,
) {
    let colour = if preview.reason.is_some() {
        REFUSED
    } else {
        scene::utility(content, &preview.utility).map_or(Color::WHITE, |utility| {
            scene::parse(&utility.appearance.colour)
        })
    };
    for (point, kind) in [
        (preview.start, Some(SnapKind::Free)),
        (preview.end, preview.end_kind),
    ] {
        let Some(point) = point else {
            continue;
        };
        let ground = Vec3::new(
            point.x,
            scene::ground_height(replica, point) + 0.06,
            point.y,
        );
        match kind {
            Some(SnapKind::Port) => square(gizmos, ground, PORT_HALF * 1.4, colour),
            Some(SnapKind::Node) => symbol(gizmos, ground, UtilitySymbol::Circle, colour),
            Some(SnapKind::Split) => symbol(gizmos, ground, UtilitySymbol::Diamond, colour),
            _ => symbol(gizmos, ground, UtilitySymbol::Square, colour),
        }
    }
    if let (Some(start), Some(end)) = (preview.start, preview.end) {
        let height = preview_height(replica, start, end, preview.depth);
        dashed(
            gizmos,
            Vec3::new(start.x, height, start.y),
            Vec3::new(end.x, height, end.y),
            colour,
        );
    }
}

/// The preview line as plan points and height, for the test bridge.
pub fn preview_line(replica: &crate::replica::Replica, output: &Output) -> Option<(Vec3, Vec3)> {
    let preview = output.route.as_ref()?;
    let (start, end) = (preview.start?, preview.end?);
    let height = preview_height(replica, start, end, preview.depth);
    Some((
        Vec3::new(start.x, height, start.y),
        Vec3::new(end.x, height, end.y),
    ))
}
