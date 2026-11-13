//! Translate authored layers into shared render instances.
use super::particles::{ParticlePool, particle};
use super::*;
use turts_mod_api::{EffectKind, EffectLayer, EffectLayerKind as Kind};

#[derive(Clone)]
pub struct Line {
    pub source: Vec3,
    pub target: Vec3,
    pub colour: Color,
    pub until: f64,
}
#[derive(Resource, Default)]
pub struct Emitters {
    pub lines: VecDeque<Line>,
    pub decals: VecDeque<(f64, Vec3, f32, Color)>,
    serial: u64,
}
fn layers(effect: &turts_mod_api::VisualEffectDef) -> Vec<EffectLayer> {
    if !effect.layers.is_empty() {
        return effect.layers.clone();
    }
    let kind = match effect.kind {
        EffectKind::Flash => Kind::Flash {
            radius: effect.size,
            light: false,
        },
        EffectKind::Spark => Kind::Sparks {
            count: 1,
            speed: 3.,
            spread_deg: 60.,
            gravity: 9.,
        },
        EffectKind::Tracer => Kind::Tracer { width: effect.size },
        EffectKind::Burst => Kind::Burst {
            radius_start: 0.1,
            radius_end: effect.size,
        },
        EffectKind::Smoke => Kind::Smoke {
            rate: 4.,
            rise: 0.5,
            spread: 0.5,
        },
    };
    vec![EffectLayer {
        kind,
        colour: effect.colour.clone(),
        end_colour: None,
        size: effect.size,
        end_size: None,
        ms: effect.ms,
        delay_ms: 0,
        image: None,
        blend: Default::default(),
        offset: [0.; 3],
        looping: false,
    }]
}
#[allow(clippy::too_many_arguments)]
pub fn emit_effects(
    fx: Res<CombatPresentation>,
    acquired: Option<Res<AcquiredPackages>>,
    settings: Res<crate::models::GraphicsSettings>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut pool: ResMut<ParticlePool>,
    mut emitters: ResMut<Emitters>,
) {
    let Some(acquired) = acquired else {
        return;
    };
    pool.limit = ParticlePool::capacity(settings.quality);
    let Some((camera, transform)) = camera.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    let viewport = camera.logical_viewport_rect();
    for emission in &fx.emissions {
        let visible = [emission.source, emission.target].iter().any(|p| {
            camera
                .world_to_viewport(transform, *p)
                .ok()
                .is_some_and(|p| viewport.is_some_and(|r| r.contains(p)))
        });
        if !visible {
            continue;
        }
        let Some(effect) = acquired.0.presentation.effects.get(&emission.effect) else {
            continue;
        };
        let distance = transform.translation().distance_squared(emission.source);
        let quality_scale = match settings.quality {
            crate::models::Quality::Low => 0.25,
            crate::models::Quality::Medium => 0.5,
            crate::models::Quality::High => 1.,
        };
        for layer in layers(effect) {
            emitters.serial = emitters.serial.wrapping_add(1);
            let seed = emitters.serial;
            let base = particle(
                &layer,
                emission.source,
                effect.cost,
                distance,
                emission.critical,
            );
            let mut particles = Vec::new();
            match &layer.kind {
                Kind::Flash { radius, .. } => {
                    let mut p = base;
                    p.size = radius * 2.;
                    p.end_size = radius * 2.5;
                    particles.push(p);
                }
                Kind::Burst {
                    radius_start,
                    radius_end,
                } => {
                    let mut p = base;
                    p.size = radius_start * 2.;
                    p.end_size = radius_end * 2.;
                    particles.push(p);
                }
                Kind::Sparks {
                    count,
                    speed,
                    spread_deg,
                    gravity,
                } => {
                    let count = ((*count as f32 * quality_scale).ceil() as u16).max(1);
                    let direction = (emission.target - emission.source).normalize_or_zero();
                    let axis = if direction.length_squared() > 0.1 {
                        direction
                    } else {
                        Vec3::Y
                    };
                    for i in 0..count {
                        let mut p = base.clone();
                        let (u, v) = noise(seed, i);
                        let angle = u * std::f32::consts::TAU;
                        let cone = v * spread_deg.to_radians() * 0.5;
                        let helper = if axis.dot(Vec3::Y).abs() > 0.9 {
                            Vec3::X
                        } else {
                            Vec3::Y
                        };
                        let right = axis.cross(helper).normalize();
                        let up = right.cross(axis);
                        p.velocity = (axis * cone.cos()
                            + (right * angle.cos() + up * angle.sin()) * cone.sin())
                            * *speed;
                        p.gravity = *gravity;
                        particles.push(p);
                    }
                }
                Kind::Debris {
                    count,
                    speed,
                    image,
                    ..
                } => {
                    for i in 0..((*count as f32 * quality_scale).ceil() as u16).max(1) {
                        let mut p = base.clone();
                        let (u, v) = noise(seed, i);
                        let angle = u * std::f32::consts::TAU;
                        p.velocity = Vec3::new(angle.cos(), 0.5 + v, angle.sin()) * *speed;
                        p.gravity = 9.;
                        p.spin = 3. + v * 5.;
                        p.image = image.clone().or(p.image);
                        particles.push(p);
                    }
                }
                Kind::Smoke { rate, rise, .. } | Kind::Fire { rate, height: rise } => {
                    let spread = if let Kind::Smoke { spread, .. } = layer.kind {
                        spread
                    } else {
                        0.25
                    };
                    let count = (*rate
                        * emission.window.unwrap_or(layer.ms as f32 / 1000.)
                        * quality_scale)
                        .ceil()
                        .clamp(1., 64.) as u16;
                    for i in 0..count {
                        let mut p = base.clone();
                        let (u, v) = noise(seed, i);
                        let angle = u * std::f32::consts::TAU;
                        p.position += Vec3::new(angle.cos(), 0., angle.sin()) * spread * v;
                        p.velocity = Vec3::Y * *rise;
                        p.end_size = layer.end_size.unwrap_or(layer.size * 2.);
                        p.delay += i as f32 / (*rate * quality_scale).max(0.1);
                        particles.push(p);
                    }
                }
                Kind::Projectile {
                    speed,
                    arc_height,
                    image,
                    ..
                } => {
                    let mut p = base;
                    p.life =
                        flight_seconds(emission.source.distance(emission.target), *speed) as f32;
                    p.target = Some(emission.target);
                    p.arc = *arc_height;
                    p.image = image.clone().or(p.image);
                    particles.push(p);
                    // The line remains readable during the sprite pipeline's first compilation and at budget exhaustion.
                    emitters.lines.push_back(Line {
                        source: emission.source,
                        target: emission.target,
                        colour: Color::srgb(0.85, 0.78, 0.55),
                        until: fx.now + f64::from(particles[0].life),
                    });
                }
                Kind::Tracer { .. } | Kind::Trail { .. } => {
                    emitters.lines.push_back(Line {
                        source: emission.source,
                        target: emission.target,
                        colour: Color::from(base.colour),
                        until: fx.now + f64::from(base.life),
                    });
                }
                Kind::Decal { size, ms, .. } => {
                    while emitters.decals.len() >= 128 {
                        emitters.decals.pop_front();
                    }
                    emitters.decals.push_back((
                        fx.now + f64::from(*ms) / 1000.,
                        emission.source,
                        *size,
                        Color::from(base.colour),
                    ));
                }
                Kind::Shake { .. } => {}
            }
            let mut accepted = false;
            for p in particles {
                accepted |= pool.spawn(p);
            }
            if emission.critical
                && !accepted
                && !matches!(
                    layer.kind,
                    Kind::Projectile { .. } | Kind::Tracer { .. } | Kind::Trail { .. }
                )
            {
                // Batched cross marks are the emergency information path, separate from cosmetic particles.
                let position = emission.source;
                emitters.lines.push_back(Line {
                    source: position - Vec3::X * 0.1,
                    target: position + Vec3::X * 0.1,
                    colour: Color::WHITE,
                    until: fx.now + 0.1,
                });
                emitters.lines.push_back(Line {
                    source: position - Vec3::Y * 0.1,
                    target: position + Vec3::Y * 0.1,
                    colour: Color::WHITE,
                    until: fx.now + 0.1,
                });
            }
        }
    }
    emitters.lines.retain(|line| line.until > fx.now);
    while emitters.lines.len() > 4096 {
        emitters.lines.pop_front();
    }
    emitters.decals.retain(|(until, ..)| *until > fx.now);
}
fn noise(seed: u64, index: u16) -> (f32, f32) {
    let a = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(u64::from(index).wrapping_mul(1442695040888963407));
    (
        (a >> 32) as u32 as f32 / u32::MAX as f32,
        a as u32 as f32 / u32::MAX as f32,
    )
}
pub fn draw_lines(emitters: Res<Emitters>, mut gizmos: Gizmos) {
    for line in &emitters.lines {
        gizmos.line(line.source, line.target, line.colour);
    }
}
