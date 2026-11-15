//! Remembered subjects are screen markers. They never enter live picking or animation tables.
use crate::{ingest::ReplicaState, render::MainSceneCamera};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
struct RememberedMarker;

pub struct VisibilityPresentationPlugin;
impl Plugin for VisibilityPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            clear_hidden_bodies.after(bevy::transform::TransformSystems::Propagate),
        );
        app.add_systems(
            PostUpdate,
            sync_remembered.after(bevy::camera::CameraUpdateSystems),
        );
    }
}

fn sync_remembered(
    mut commands: Commands,
    state: Res<ReplicaState>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainSceneCamera>>,
    mut markers: Local<HashMap<(bool, u32), Entity>>,
    mut labels: Query<(&mut Node, &mut Text), With<RememberedMarker>>,
) {
    let knowledge = state
        .replica
        .as_ref()
        .and_then(|replica| replica.knowledge.as_ref());
    let camera = cameras.iter().find(|(camera, _)| camera.is_active);
    let mut desired = HashMap::new();
    if let (Some(knowledge), Some((camera, transform))) = (knowledge, camera) {
        let mut add = |key, position: turts_mod_api::WorldPos, text: String| {
            let pos = turts_net_protocol::Pos {
                x: position.x,
                y: position.y,
            };
            if let Some(point) = crate::render::projected_ground_center(camera, transform, pos) {
                desired.insert(key, (point, text));
            }
        };
        for row in &knowledge.remembered_units {
            add(
                (false, row.unit),
                row.position,
                format!("◇ Last seen {}", row.last_seen_tick),
            );
        }
        for row in &knowledge.remembered_nodes {
            add(
                (true, row.node),
                row.position,
                format!(
                    "◇ Remembered: {} remaining · seen {}",
                    row.remaining, row.last_seen_tick
                ),
            );
        }
    }
    markers.retain(|key, entity| {
        if desired.contains_key(key) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    for (key, (point, text)) in desired {
        if let Some(entity) = markers.get(&key)
            && let Ok((mut node, mut label)) = labels.get_mut(*entity)
        {
            node.left = px(point.x + 5.);
            node.top = px(point.y - 10.);
            if label.0 != text {
                *label = Text::new(text);
            }
            continue;
        }
        let entity = commands
            .spawn((
                RememberedMarker,
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(11.),
                    ..default()
                },
                TextColor(Color::srgb(0.66, 0.68, 0.72)),
                BackgroundColor(Color::srgba(0.03, 0.04, 0.06, 0.8)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(point.x + 5.),
                    top: px(point.y - 10.),
                    ..default()
                },
                bevy::picking::Pickable::IGNORE,
            ))
            .id();
        markers.insert(key, entity);
    }
}

/// Corpses and delayed death bodies carry no live identity after their event.
fn clear_hidden_bodies(
    mut commands: Commands,
    state: Res<ReplicaState>,
    bodies: Query<
        (Entity, &GlobalTransform),
        Or<(
            With<crate::animation::VisualBody>,
            With<crate::fx::PendingDeath>,
        )>,
    >,
) {
    for (entity, transform) in &bodies {
        if !crate::fx::permitted_position(Some(&state), transform.translation()) {
            commands.entity(entity).despawn();
        }
    }
}
