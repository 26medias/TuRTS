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
    let tick_hz = state
        .replica
        .as_ref()
        .map_or(1, |replica| replica.tick_hz)
        .max(1);
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
                format!(
                    "◇ {}",
                    last_seen_age(knowledge.basis.tick, row.last_seen_tick, tick_hz)
                ),
            );
        }
        for row in &knowledge.remembered_nodes {
            add(
                (true, row.node),
                row.position,
                format!(
                    "◇ Remembered: {} remaining · {}",
                    row.remaining,
                    last_seen_age(knowledge.basis.tick, row.last_seen_tick, tick_hz)
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

fn last_seen_age(current_tick: u32, last_seen_tick: u32, tick_hz: u16) -> String {
    let seconds = current_tick.saturating_sub(last_seen_tick) / u32::from(tick_hz.max(1));
    if seconds == 0 {
        "Last seen just now".into()
    } else {
        format!("Last seen {seconds} s ago")
    }
}

/// Corpses and delayed death bodies carry no live identity after their event.
#[allow(clippy::type_complexity)] // Bevy query filters describe the existing body variants.
fn clear_hidden_bodies(
    mut commands: Commands,
    state: Res<ReplicaState>,
    fx: Option<Res<crate::fx::CombatPresentation>>,
    bodies: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&crate::fx::AuthorizedDeathPose>,
        ),
        Or<(
            With<crate::animation::VisualBody>,
            With<crate::fx::PendingDeath>,
        )>,
    >,
) {
    for (entity, transform, pose) in &bodies {
        if let Some(pose) = pose {
            let valid = fx.as_ref().is_some_and(|fx| {
                state.replica.is_some()
                    && pose.epoch == fx.epoch
                    && pose.until.is_finite()
                    && pose.until > fx.now
            });
            if !valid {
                commands.entity(entity).despawn();
            }
            continue;
        }
        if !crate::fx::permitted_position(Some(&state), transform.translation()) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod death_pose_tests {
    use super::*;
    use crate::{
        animation::{BodySerial, VisualBody},
        fx::{AuthorizedDeathPose, CombatPresentation, PendingDeath},
    };

    #[test]
    fn witnessed_pending_death_survives_fog_then_expires_without_live_identity() {
        let mut app = App::new();
        let mut replica = crate::replica::Replica::new(20);
        replica.knowledge = Some(std::sync::Arc::new(
            crate::fx::timeline_tests::hidden_knowledge(),
        ));
        app.insert_resource(ReplicaState {
            replica: Some(replica),
            ..default()
        })
        .init_resource::<CombatPresentation>()
        .init_resource::<BodySerial>()
        .add_systems(Update, crate::fx::release_deaths)
        .add_systems(PostUpdate, clear_hidden_bodies);
        let witnessed = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                PendingDeath {
                    at: 0.3,
                    remove_model: false,
                },
                AuthorizedDeathPose {
                    epoch: 0,
                    until: 4.8,
                },
            ))
            .id();
        let unwitnessed = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                PendingDeath {
                    at: 0.3,
                    remove_model: false,
                },
            ))
            .id();
        app.world_mut().resource_mut::<CombatPresentation>().now = 0.1;
        app.update();
        assert!(
            app.world().get_entity(unwitnessed).is_err(),
            "ordinary hidden bodies must disappear"
        );
        assert!(app.world().get::<PendingDeath>(witnessed).is_some());
        assert!(
            app.world()
                .get::<crate::render::PresentedUnit>(witnessed)
                .is_none()
        );
        let replica = app
            .world()
            .resource::<ReplicaState>()
            .replica
            .as_ref()
            .unwrap();
        assert!(replica.units.is_empty());
        assert!(replica.healths.is_empty());
        app.world_mut().resource_mut::<CombatPresentation>().now = 0.3;
        app.update();
        assert!(app.world().get::<PendingDeath>(witnessed).is_none());
        assert!(app.world().get::<VisualBody>(witnessed).is_some());
        app.world_mut().resource_mut::<CombatPresentation>().now = 4.8;
        app.update();
        assert!(app.world().get_entity(witnessed).is_err());
    }

    #[test]
    fn a_new_effect_epoch_retires_a_witnessed_body_even_on_observed_ground() {
        let mut app = App::new();
        app.insert_resource(ReplicaState {
            replica: Some(crate::replica::Replica::new(20)),
            ..default()
        })
        .init_resource::<CombatPresentation>()
        .add_systems(Update, clear_hidden_bodies);
        let body = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                PendingDeath {
                    at: 0.3,
                    remove_model: true,
                },
                AuthorizedDeathPose {
                    epoch: 0,
                    until: 4.8,
                },
            ))
            .id();
        app.world_mut().resource_mut::<CombatPresentation>().epoch = 1;
        app.update();
        assert!(app.world().get_entity(body).is_err());
    }
}
