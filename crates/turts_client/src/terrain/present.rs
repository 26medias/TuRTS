//! Public work previews and owner-only progress. No geological query is made here.
use crate::{
    core_adapter::{CoreInputQueue, CoreRuntime},
    ingest::ReplicaState,
    join::AcquiredPackages,
    ui_shared::UiCaptureRoot,
};
use bevy::{
    prelude::*,
    ui_widgets::{Activate, Button, observe},
};
use turts_mod_api::{Behaviour, ContentId, ResolvedTerrainOperation, TerrainProvenance};
use turts_net_protocol::{OrderTarget, TerrainWorkWait};

#[derive(Resource, Default)]
pub(crate) struct WorkPanel {
    root: Option<Entity>,
    key: String,
}
#[derive(Component)]
struct Resume(turts_interaction::terrain::UnloadResume);
fn resume(trigger: On<Activate>, actions: Query<&Resume>, mut queue: ResMut<CoreInputQueue>) {
    if let Ok(action) = actions.get(trigger.entity) {
        queue.0.push(turts_interaction::InputEvent::UnloadResume(
            action.0.clone(),
        ));
    }
}

/// Unknown geometry remains a public initial/remembered shape, with a gray ghost.
/// The green/blue target face is only a designation, not a promise of feasibility.
pub(crate) fn preview(
    runtime: Option<Res<CoreRuntime>>,
    state: Res<ReplicaState>,
    mut lines: Gizmos,
) {
    let Some(terrain) = state.replica.as_ref().and_then(|r| r.terrain.as_ref()) else {
        return;
    };
    let Some(preview) = runtime.as_ref().and_then(|r| r.terrain_preview.as_ref()) else {
        return;
    };
    let width = terrain.layout.cell_width();
    for cell in &preview.cells {
        let height = terrain
            .layout
            .elevation_height(preview.target_elevation.unwrap_or(cell.elevation))
            + 0.025;
        let color = if !cell.current {
            Color::srgba(0.65, 0.65, 0.65, 0.95)
        } else if preview
            .target_elevation
            .is_some_and(|level| level > cell.elevation)
        {
            Color::srgb(0.2, 0.65, 1.)
        } else {
            Color::srgb(0.2, 1., 0.45)
        };
        let (x, z) = (cell.cell.x as f32 * width, cell.cell.y as f32 * width);
        let corners = [
            Vec3::new(x, height, z),
            Vec3::new(x + width, height, z),
            Vec3::new(x + width, height, z + width),
            Vec3::new(x, height, z + width),
        ];
        for i in 0..4 {
            lines.line(corners[i], corners[(i + 1) % 4], color);
        }
        let surface = terrain.layout.elevation_height(cell.elevation) + 0.025;
        for point in corners {
            lines.line(Vec3::new(point.x, surface, point.z), point, color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn panel(
    mut commands: Commands,
    runtime: Option<Res<CoreRuntime>>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut panel: ResMut<WorkPanel>,
) {
    let mut text = String::new();
    let mut actions = Vec::<(String, turts_interaction::terrain::UnloadResume)>::new();
    if let (Some(runtime), Some(replica), Some(acquired)) =
        (runtime.as_ref(), state.replica.as_ref(), acquired.as_ref())
    {
        if let Some(terrain) = replica.terrain.as_ref() {
            let content = &acquired.0.content;
            if let Some(preview) = runtime.terrain_preview.as_ref() {
                let tool = ContentId::parse(&preview.input.tool)
                    .ok()
                    .and_then(|id| content.catalog.terrain_tools.get(&id));
                text.push_str(tool.map_or("Terrain work", |tool| tool.name.as_str()));
                if let Some(level) = preview.target_elevation {
                    let unknown = preview.cells.iter().filter(|cell| !cell.current).count();
                    text.push_str(&format!("\nTarget level {level} · {} cells · {unknown} Unknown\nDrag a rectangle. Shift queues. Right click cancels.",preview.cells.len()));
                    if let Some(tool) = tool {
                        match &tool.operation {
                            ResolvedTerrainOperation::Excavate => {
                                let placed = preview
                                    .cells
                                    .iter()
                                    .filter(|cell| {
                                        cell.current
                                            && cell.elevation > level
                                            && terrain.surfaces.get(&cell.cell).is_some_and(|s| {
                                                s.provenance == Some(TerrainProvenance::Placed)
                                            })
                                    })
                                    .count();
                                if placed > 0 {
                                    text.push_str(&format!(
                                        "\n{placed} observed placed surfaces: NoYield"
                                    ));
                                }
                                if preview.cells.iter().any(|cell| {
                                    cell.current
                                        && cell.elevation > level
                                        && terrain.surfaces.get(&cell.cell).is_some_and(|s| {
                                            s.provenance == Some(TerrainProvenance::Natural)
                                        })
                                }) {
                                    text.push_str("\nNatural output applies only to each observed exposed prism.");
                                }
                            }
                            ResolvedTerrainOperation::Fill { material } => {
                                if let Some(material) =
                                    content.catalog.terrain_materials.get(material)
                                {
                                    text.push_str(&format!(
                                        "\nFill: {} · placed material has NoYield",
                                        material.name
                                    ));
                                }
                            }
                        }
                    }
                    if unknown > 0 {
                        text.push_str(
                            "\nUnknown cells remain in the order. Cost and work are Unknown.",
                        );
                    }
                } else {
                    text.push_str("\nClick an observed cell to choose its reference level.");
                }
                if let Some(reason) = &preview.reason {
                    text.push_str(&format!("\n{reason}"));
                }
            }
            if let Some(unit) = runtime
                .core()
                .and_then(|core| core.selected_units().first())
            {
                if let Some(work) = terrain.work.get(&unit.0) {
                    if !text.is_empty() {
                        text.push_str("\n\n");
                    }
                    text.push_str(&format!(
                        "Work target level {}",
                        work.designation.target_elevation
                    ));
                    match work.required_work {
                        Some(required) => text
                            .push_str(&format!("\nProgress: {} / {required}", work.completed_work)),
                        None => text.push_str("\nProgress: Unknown"),
                    };
                    if let Some(wait) = work.wait {
                        text.push_str(&format!("\n{}", wait_text(wait)));
                    }
                    // The server keeps the exact suspended work record while unloading.
                    // Only the active original TerrainWork may offer another unload action.
                    let current=replica.unit_orders.get(&unit.0).and_then(|r|r.current.as_ref()).filter(|o|matches!(&o.target,OrderTarget::TerrainWork(d) if d==&work.designation));
                    if work.wait == Some(TerrainWorkWait::FullCargo) {
                        if let (Some(order), Some(actor)) = (current, replica.units.get(&unit.0)) {
                            if let Ok(actor_id) = ContentId::parse(&actor.content_id) {
                                if content
                                    .catalog
                                    .units
                                    .get(&actor_id)
                                    .is_some_and(|d| d.terrain_cargo.is_some())
                                {
                                    let mut depots=replica.units.iter().filter_map(|(id,depot)|{
                                        let definition=ContentId::parse(&depot.content_id).ok().and_then(|id|content.catalog.units.get(&id))?;
                                        (depot.owner_slot==actor.owner_slot && definition.inventory.is_some() && definition.drop_off.as_ref().is_some_and(|dropoff|dropoff.rules.iter().any(|rule|matches!(rule.effect,turts_mod_api::ResolvedDropOffEffect::Store)))).then_some((*id,depot,definition))
                                    }).collect::<Vec<_>>();
                                    depots.sort_by(|a, b| {
                                        a.1.samples
                                            .back()
                                            .map(|(_, p)| (p.x, p.y))
                                            .partial_cmp(
                                                &b.1.samples.back().map(|(_, p)| (p.x, p.y)),
                                            )
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                            .then_with(|| a.2.name.cmp(&b.2.name))
                                    });
                                    for unload in content.catalog.orders.values().filter(|d| {
                                        d.behaviour == Behaviour::UnloadCargo
                                            && d.target == turts_mod_api::TargetKind::Unit
                                            && (d.units.is_empty()
                                                || d.units.contains(&actor.content_id))
                                    }) {
                                        for (depot, unit_def, definition) in &depots {
                                            let Some((_, pos)) = unit_def.samples.back() else {
                                                continue;
                                            };
                                            actions.push((
                                                format!(
                                                    "Unload and resume: {} at ({:0.1}, {:0.1})",
                                                    definition.name, pos.x, pos.y
                                                ),
                                                turts_interaction::terrain::UnloadResume {
                                                    identity: (
                                                        terrain.basis.match_id,
                                                        terrain.basis.map.clone(),
                                                    ),
                                                    unit: *unit,
                                                    depot: turts_net_protocol::UnitId(*depot),
                                                    unload_order: unload.id.to_string(),
                                                    work_order: order.order.clone(),
                                                    designation: work.designation.clone(),
                                                },
                                            ));
                                        }
                                    }
                                    if actions.is_empty() {
                                        text.push_str("\nNo owned cargo depot and eligible unload order are available.");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let key = format!("{text}\n{actions:?}");
    if panel.key == key {
        return;
    }
    panel.key = key;
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    if text.is_empty() {
        return;
    }
    panel.root = Some(
        commands
            .spawn((
                UiCaptureRoot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(12.),
                    top: Val::Px(72.),
                    max_width: Val::Px(400.),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.)),
                    row_gap: Val::Px(5.),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.04, 0.055, 0.92)),
                GlobalZIndex(40),
            ))
            .with_children(|root| {
                root.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(14.),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                for (label, action) in actions {
                    root.spawn((
                        Button,
                        Resume(action),
                        observe(resume),
                        Node {
                            padding: UiRect::all(Val::Px(6.)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.3, 0.35)),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: bevy::text::FontSize::Px(14.),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                }
            })
            .id(),
    );
}
fn wait_text(wait: TerrainWorkWait) -> &'static str {
    match wait {
        TerrainWorkWait::Approaching => "Approaching work cell",
        TerrainWorkWait::Unknown => "Unknown: observe the whole work cell",
        TerrainWorkWait::Contested => "Waiting: another worker has this cell",
        TerrainWorkWait::Occupied => "Waiting: an entity occupies this cell",
        TerrainWorkWait::UnsupportedMaterial => "This tool does not support the exposed material",
        TerrainWorkWait::TooHard => "The exposed material is too hard",
        TerrainWorkWait::Bedrock => "Bedrock reached",
        TerrainWorkWait::Ceiling => "Terrain ceiling reached",
        TerrainWorkWait::FullCargo => "Cargo full: unload to resume this work",
        TerrainWorkWait::InsufficientInput => "Waiting for fill resources",
        TerrainWorkWait::Blocked => "Work cell is unreachable",
        TerrainWorkWait::Paused => "Work paused",
        TerrainWorkWait::Unloading => "Unloading; the same work and partial progress will resume",
    }
}
