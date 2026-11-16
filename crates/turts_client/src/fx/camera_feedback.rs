//! Bounded optional lights and local camera shake.
use super::*;
#[derive(Resource, Default)]
pub struct FlashLights {
    entities: HashMap<u64, Entity>,
}
pub fn update_lights(
    mut commands: Commands,
    fx: Res<CombatPresentation>,
    emitters: Res<emitters::Emitters>,
    mut shown: ResMut<FlashLights>,
) {
    let live: HashSet<_> = emitters.lights.iter().map(|light| light.id).collect();
    shown.entities.retain(|id, entity| {
        if live.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    for light in &emitters.lights {
        if light.starts > fx.now || shown.entities.contains_key(&light.id) {
            continue;
        }
        let entity = commands
            .spawn((
                PointLight {
                    color: light.colour,
                    intensity: 300.,
                    range: light.radius * 4.,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(light.position),
            ))
            .id();
        shown.entities.insert(light.id, entity);
    }
}
pub fn shake_camera(
    fx: Res<CombatPresentation>,
    emitters: Res<emitters::Emitters>,
    settings: Res<flash::EffectSettings>,
    core: Option<Res<crate::core_adapter::CoreRuntime>>,
    mut cameras: Query<&mut Transform, With<crate::render::MainSceneCamera>>,
) {
    let Some(mut desired) = core.as_ref().and_then(|core| core.base_camera_transform()) else {
        return;
    };
    let amount = emitters
        .shakes
        .iter()
        .filter(|shake| shake.starts <= fx.now)
        .map(|shake| {
            let fade = ((shake.until - fx.now) / (shake.until - shake.starts).max(0.001))
                .clamp(0., 1.) as f32;
            shake.strength * fade
        })
        .sum::<f32>()
        .min(0.5)
        * settings.shake.gain();
    if amount > 0. {
        let phase = fx.now as f32;
        desired.translation += (desired.rotation * Vec3::X) * (phase * 83.).sin() * amount
            + (desired.rotation * Vec3::Y) * (phase * 107.).sin() * amount * 0.5;
    }
    for mut transform in &mut cameras {
        if *transform != desired {
            *transform = desired;
        }
    }
}
