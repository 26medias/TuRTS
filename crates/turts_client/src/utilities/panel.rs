//! The text of the utility view: the route preview and the hover inspection. One text panel,
//! built from the owner view, the catalog and the permitted terrain. It never reads hidden state.

use crate::{
    content::ClientContent, core_adapter::CoreRuntime, ingest::ReplicaState,
    join::AcquiredPackages, replica::Replica,
};
use bevy::prelude::*;
use turts_interaction::utility::{Hover, HoverTarget, Output, RoutePreview, reason_text};
use turts_net_protocol::{
    UtilityEndRecord, UtilityNetworkRecord, UtilitySegmentRecord, UtilitySegmentState, UtilityView,
};

#[derive(Resource, Default)]
pub struct UtilityPanel {
    root: Option<Entity>,
    text: Option<Entity>,
    key: String,
}

/// The text of the route preview.
pub fn route_text(route: &RoutePreview) -> String {
    let mut text = format!(
        "{}: route\nDepth {} ({} to {}). [ and ] change the depth.",
        route.name, route.depth, route.depth_min, route.depth_max
    );
    match route.length {
        Some(length) => {
            let cost = if route.cost.is_empty() {
                "free".to_owned()
            } else {
                route
                    .cost
                    .iter()
                    .map(|(name, amount)| format!("{amount} {name}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            text.push_str(&format!("\nLength {length:.1}. Cost {cost}."));
        }
        None if route.start.is_none() => text.push_str("\nClick to set the start."),
        None => {}
    }
    if let Some(reason) = route.reason {
        text.push_str(&format!("\n{}", reason_text(reason)));
    } else if route.start.is_some() {
        text.push_str("\nClick to place the line.");
    }
    text.push_str("\nRight click or Escape ends route mode.");
    text
}

fn network_of_segment<'a>(
    view: &'a UtilityView,
    segment: &UtilitySegmentRecord,
) -> Option<&'a UtilityNetworkRecord> {
    // A line that does not conduct has no network of its own.
    if segment.state == UtilitySegmentState::Disconnected {
        return None;
    }
    view.networks.iter().find(|network| {
        network.utility == segment.utility
            && [segment.a, segment.b].iter().any(|end| match end {
                UtilityEndRecord::Node(id) => network.nodes.contains(id),
                UtilityEndRecord::Port(unit) => network.units.contains(unit),
            })
    })
}

fn state_label(
    content: &ClientContent,
    utility: &str,
    state: UtilitySegmentState,
    network: Option<&UtilityNetworkRecord>,
) -> String {
    let Some(definition) = super::scene::utility(content, utility) else {
        return String::new();
    };
    let labels = &definition.appearance.labels;
    match (state, network) {
        (UtilitySegmentState::Exposed | UtilitySegmentState::Disconnected, _) => {
            labels.exposed.clone()
        }
        (_, Some(network)) if network.short => labels.short.clone(),
        (_, Some(_)) => labels.powered.clone(),
        (_, None) => labels.unconnected.clone(),
    }
}

fn network_text(network: Option<&UtilityNetworkRecord>) -> String {
    match network {
        Some(network) => format!(
            "Network: provided {}, consumed {}",
            network.provided, network.consumed
        ),
        None => "Network: not connected".to_owned(),
    }
}

/// The cover line: the surface of the hovered cell minus the elevation of the line, from the
/// permitted terrain only. A cell with no surface row of the client gives "cover unknown".
fn cover_text(replica: &Replica, hover: &Hover, elevation: i32) -> String {
    let surface = match replica.terrain.as_ref() {
        // A map without terrain has the surface 0.
        None => Some((0, true)),
        Some(terrain) => hover
            .cell
            .and_then(|cell| terrain.surfaces.get(&cell))
            .map(|row| (row.elevation, row.current)),
    };
    match surface {
        Some((surface, current)) => format!(
            "Cover {}{}",
            turts_rules::utilities::cover(surface, elevation),
            if current { "" } else { " (last seen)" }
        ),
        None => "Cover unknown".to_owned(),
    }
}

/// The text of the hover inspection.
pub fn hover_text(
    replica: &Replica,
    content: &ClientContent,
    view: &UtilityView,
    hover: &Hover,
) -> String {
    match hover.target {
        HoverTarget::Segment(id) => {
            let Some(segment) = view.segments.iter().find(|segment| segment.id == id) else {
                return String::new();
            };
            let network = network_of_segment(view, segment);
            let name = super::scene::utility(content, &segment.utility)
                .map_or("Utility", |utility| utility.name.as_str());
            format!(
                "{name}\n{}\n{}\nDepth {} · elevation {}\n{}",
                state_label(content, &segment.utility, segment.state, network),
                network_text(network),
                segment.depth,
                segment.elevation,
                cover_text(replica, hover, segment.elevation),
            )
        }
        HoverTarget::Node(id) => {
            let Some(node) = view.nodes.iter().find(|node| node.id == id) else {
                return String::new();
            };
            let network = view
                .networks
                .iter()
                .find(|network| network.utility == node.utility && network.nodes.contains(&id));
            let name = super::scene::utility(content, &node.utility)
                .map_or("Utility", |utility| utility.name.as_str());
            let state = view
                .segments
                .iter()
                .filter(|segment| {
                    segment.a == UtilityEndRecord::Node(id)
                        || segment.b == UtilityEndRecord::Node(id)
                })
                .map(|segment| segment.state)
                .max_by_key(|state| match state {
                    UtilitySegmentState::Conducting => 0,
                    UtilitySegmentState::Exposed => 1,
                    UtilitySegmentState::Disconnected => 2,
                })
                .unwrap_or(UtilitySegmentState::Conducting);
            format!(
                "{name} junction\n{}\n{}",
                state_label(content, &node.utility, state, network),
                network_text(network),
            )
        }
        HoverTarget::Port(unit) => {
            let Some(network) = view
                .networks
                .iter()
                .find(|network| network.units.contains(&unit))
            else {
                return String::new();
            };
            let name = super::scene::utility(content, &network.utility)
                .map_or("Utility", |utility| utility.name.as_str());
            let label = match super::scene::port_state(view, unit.0) {
                super::scene::PortState::Unconnected => state_label(
                    content,
                    &network.utility,
                    UtilitySegmentState::Conducting,
                    None,
                ),
                _ => state_label(
                    content,
                    &network.utility,
                    UtilitySegmentState::Conducting,
                    Some(network),
                ),
            };
            format!("{name} port\n{label}\n{}", network_text(Some(network)))
        }
    }
}

/// The whole panel text for the output of the core. Empty when there is nothing to say.
pub fn panel_text(replica: &Replica, content: &ClientContent, output: &Output) -> String {
    if !output.view_on {
        return String::new();
    }
    let mut parts = Vec::new();
    if let Some(route) = output.route.as_ref() {
        parts.push(route_text(route));
    }
    if let (Some(hover), Some(view)) = (output.hover.as_ref(), replica.utilities.as_ref()) {
        let text = hover_text(replica, content, view, hover);
        if !text.is_empty() {
            parts.push(text);
        }
    }
    if parts.is_empty() {
        parts.push("Utility view. U turns it off.".to_owned());
    }
    parts.join("\n\n")
}

pub(crate) fn sync(
    mut commands: Commands,
    runtime: Option<Res<CoreRuntime>>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut panel: ResMut<UtilityPanel>,
    mut texts: Query<&mut Text>,
) {
    let text = match (
        runtime.as_ref(),
        state.replica.as_ref(),
        state
            .content
            .as_ref()
            .or_else(|| acquired.as_ref().map(|acquired| &acquired.0.content)),
    ) {
        (Some(runtime), Some(replica), Some(content)) => {
            panel_text(replica, content, runtime.utility_output())
        }
        _ => String::new(),
    };
    if panel.key == text && (text.is_empty() == panel.root.is_none()) {
        return;
    }
    panel.key.clone_from(&text);
    if text.is_empty() {
        if let Some(root) = panel.root.take() {
            commands.entity(root).despawn();
        }
        panel.text = None;
        return;
    }
    if let Some(entity) = panel.text
        && let Ok(mut current) = texts.get_mut(entity)
    {
        **current = text;
        return;
    }
    let mut label = None;
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.),
                top: Val::Px(220.),
                max_width: Val::Px(360.),
                padding: UiRect::all(Val::Px(10.)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.04, 0.055, 0.88)),
            GlobalZIndex(39),
        ))
        .with_children(|root| {
            label = Some(
                root.spawn((
                    crate::widgets::primitives::AuthoredWidgetId("utility_view_status".into()),
                    Text::new(text),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(14.),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ))
                .id(),
            );
        })
        .id();
    panel.root = Some(root);
    panel.text = label;
}
